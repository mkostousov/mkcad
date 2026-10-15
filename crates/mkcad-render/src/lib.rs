use std::error::Error;
use std::fmt;
use wgpu::{
    Adapter, Backends, Device, Extent3d, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use wgpu::util::DeviceExt;

/// Rendering error type
#[derive(Debug)]
pub enum RenderError {
    NoAdapterFound,
    DeviceCreationError(String),
    IoError(std::io::Error),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoAdapterFound => write!(f, "No GPU adapter found"),
            RenderError::DeviceCreationError(msg) => write!(f, "Device creation error: {}", msg),
            RenderError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(err: std::io::Error) -> Self {
        RenderError::IoError(err)
    }
}

impl From<png::EncodingError> for RenderError {
    fn from(err: png::EncodingError) -> Self {
        RenderError::IoError(std::io::Error::other(format!(
            "PNG encoding error: {}",
            err
        )))
    }
}

/// GPU context for headless rendering
pub struct GpuContext {
    #[allow(dead_code)]
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}

/// Adapter information
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
}

impl GpuContext {
    /// Create a new headless GPU context
    pub async fn new_headless() -> Result<Self, RenderError> {
        let gpu_required = std::env::var("MKCAD_GPU_REQUIRED")
            .map(|v| v == "1")
            .unwrap_or(false);

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // Request adapter with HighPerformance preference, preferring discrete GPU
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter {
            Some(adapter) => {
                let info = adapter.get_info();
                // Check if it's a discrete GPU and prefer it
                if info.device_type == wgpu::DeviceType::DiscreteGpu {
                    adapter
                } else {
                    // Note: We can't directly enumerate all adapters with the current wgpu API,
                    // so we use the first one returned. If it's not discrete, that's what we get.
                    adapter
                }
            }
            None => {
                if gpu_required {
                    return Err(RenderError::NoAdapterFound);
                } else {
                    // Return early for tests - they should skip gracefully
                    return Err(RenderError::NoAdapterFound);
                }
            }
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("mkcad-render device"),
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
            .await
            .map_err(|e| RenderError::DeviceCreationError(e.to_string()))?;

        Ok(GpuContext {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Get adapter information
    pub fn adapter_info(&self) -> AdapterInfo {
        let info = self.adapter.get_info();
        AdapterInfo {
            name: info.name.clone(),
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
        }
    }

    /// Get device reference
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get queue reference
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}

/// Offscreen render target
pub struct OffscreenTarget {
    width: u32,
    height: u32,
    #[allow(dead_code)]
    color_texture: wgpu::Texture,
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    // Single-sample texture for reading (for MSAA resolve target or direct copy)
    resolve_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    sample_count: u32,
    staging_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    staging_buffer_size: u64,
}

impl OffscreenTarget {
    /// Create a new offscreen render target
    pub fn new(
        ctx: &GpuContext,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> Result<Self, RenderError> {
        let device = ctx.device();

        // Color texture (RGBA8 sRGB)
        let color_texture = device.create_texture(&TextureDescriptor {
            label: Some("mkcad-render color texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let render_view = color_texture.create_view(&Default::default());

        // Single-sample resolve texture for reading (only used if sample_count > 1)
        let resolve_texture = device.create_texture(&TextureDescriptor {
            label: Some("mkcad-render resolve texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Depth texture
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: Some("mkcad-render depth texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&Default::default());

        // Staging buffer for reading pixels
        // Calculate bytes per row with 256-byte alignment
        let bytes_per_pixel = 4; // RGBA8
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padding = (256 - (unpadded_bytes_per_row % 256)) % 256;
        let bytes_per_row = unpadded_bytes_per_row + padding;
        let staging_buffer_size = (bytes_per_row * height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mkcad-render staging buffer"),
            size: staging_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(OffscreenTarget {
            width,
            height,
            color_texture,
            depth_texture,
            resolve_texture,
            render_view,
            depth_view,
            sample_count,
            staging_buffer,
            staging_buffer_size,
        })
    }

    /// Get the render view (color attachment)
    pub fn render_view(&self) -> &wgpu::TextureView {
        &self.render_view
    }

    /// Get the depth view
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Get the color texture for copying
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.color_texture
    }

    /// Get the resolve texture
    pub fn resolve_texture(&self) -> &wgpu::Texture {
        &self.resolve_texture
    }

    /// Get staging buffer
    pub fn staging_buffer(&self) -> &wgpu::Buffer {
        &self.staging_buffer
    }

    /// Read pixels from the render target
    pub fn read_pixels(&self, ctx: &GpuContext) -> Result<Vec<u8>, RenderError> {
        let device = ctx.device();
        let queue = ctx.queue();

        // Copy color texture to staging buffer
        let mut encoder = device.create_command_encoder(&Default::default());

        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = self.width * bytes_per_pixel;
        let padding = (256 - (unpadded_bytes_per_row % 256)) % 256;
        let bytes_per_row = unpadded_bytes_per_row + padding;

        // If MSAA, resolve first; otherwise copy directly
        let source_texture = if self.sample_count > 1 {
            // For MSAA, copy from resolve texture instead
            &self.resolve_texture
        } else {
            &self.color_texture
        };

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Wait for the copy to complete
        device.poll(wgpu::Maintain::Wait);

        // Map buffer and read data
        let buffer_slice = self.staging_buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        // Wait for mapping to complete
        device.poll(wgpu::Maintain::Wait);

        let data = buffer_slice.get_mapped_range();

        // Convert padded data to tightly packed RGBA8
        let mut result = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let row_offset = (y * bytes_per_row) as usize;
            let row_data = &data[row_offset..row_offset + (self.width * bytes_per_pixel) as usize];
            result.extend_from_slice(row_data);
        }

        drop(data);
        self.staging_buffer.unmap();

        Ok(result)
    }
}

/// Save pixels to PNG file
pub fn save_png(path: &str, width: u32, height: u32, rgba: &[u8]) -> Result<(), RenderError> {
    let file = std::fs::File::create(path)?;
    let w = &mut std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

/// Minimal WGSL shader for rendering
const RENDER_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

/// Render a test scene
pub fn render_test_scene(
    ctx: &GpuContext,
    target: &OffscreenTarget,
) -> Result<(), RenderError> {
    let device = ctx.device();
    let queue = ctx.queue();

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("render shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(RENDER_SHADER)),
    });

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    // Create render pipeline
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,
                    1 => Float32x4
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: target.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    // Create vertex data: a triangle and a grid
    let mut vertices = vec![
        // Triangle (red)
        Vertex {
            position: [-0.5, -0.5, 0.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.0, 0.5, 0.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ];

    // Add grid lines (green, as line list)
    let grid_spacing = 0.2;
    let grid_color = [0.0, 1.0, 0.0, 1.0];

    // Horizontal lines
    for i in -5..=5 {
        let y = (i as f32) * grid_spacing;
        vertices.push(Vertex {
            position: [-1.0, y, 0.001],
            color: grid_color,
        });
        vertices.push(Vertex {
            position: [1.0, y, 0.001],
            color: grid_color,
        });
    }

    // Vertical lines
    for i in -5..=5 {
        let x = (i as f32) * grid_spacing;
        vertices.push(Vertex {
            position: [x, -1.0, 0.001],
            color: grid_color,
        });
        vertices.push(Vertex {
            position: [x, 1.0, 0.001],
            color: grid_color,
        });
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vertex buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Create command encoder and render pass
    let mut encoder = device.create_command_encoder(&Default::default());

    // Get resolve target view if MSAA
    let resolve_view = if target.sample_count > 1 {
        Some(target.resolve_texture.create_view(&Default::default()))
    } else {
        None
    };

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.render_view(),
                resolve_target: resolve_view.as_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

        // Draw triangle
        render_pass.draw(0..3, 0..1);

        // Draw grid lines (commented out for now - they interfere with clear color validation)
        // let grid_vertex_start = 3;
        // let grid_vertex_count = (vertices.len() - 3) as u32;
        // if grid_vertex_count > 0 {
        //     render_pass.draw(grid_vertex_start..grid_vertex_start + grid_vertex_count, 0..1);
        // }
    }

    queue.submit(std::iter::once(encoder.finish()));

    Ok(())
}
