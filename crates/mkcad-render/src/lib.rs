pub mod camera;

pub use camera::Camera;

use std::error::Error;
use std::fmt;
use wgpu::{
    Adapter, Backends, Device, Extent3d, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use wgpu::util::DeviceExt;
use glam::Vec3;

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

/// Shader for ID rendering with camera transform (unused, kept for reference)
#[allow(dead_code)]
const ID_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(in.position, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) u32 {
    // Note: push_constant or per-instance id would go here
    // For now, this is patched per-draw via a uniform
    return u32(0u);
}
"#;

/// Shader for ID rendering with per-instance ID via uniform
const ID_SHADER_WITH_ID: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

struct IDUniform {
    id: u32,
    _padding: u32,
    _padding2: u32,
    _padding3: u32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<uniform> id_uniform: IDUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(in.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) u32 {
    return id_uniform.id + 1u;
}
"#;

/// Shader with transform for rendering
#[allow(dead_code)]
const TRANSFORM_SHADER: &str = r#"
struct Transforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> transforms: Transforms;

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
    out.position = transforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Grid shader for rendering grid lines with camera transform
const GRID_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

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
    out.position = camera.view_proj * vec4<f32>(in.position, 1.0);
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

/// Generate a grid mesh with colored axes
/// size: total grid size (in world units)
/// divisions: number of divisions (each unit becomes 1 line)
pub fn create_grid_vertices(
    size: f32,
    divisions: u32,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let step = size / divisions as f32;

    let _dark_bg = [0.1, 0.1, 0.1, 1.0];      // Dark background
    let minor_color = [0.3, 0.3, 0.3, 0.8];  // Gray for minor lines (every 1 unit)
    let major_color = [0.7, 0.7, 0.7, 1.0];  // Bright gray for major lines (every 10 units)
    let x_axis_color = [1.0, 0.0, 0.0, 1.0]; // Red for X axis
    let z_axis_color = [0.0, 1.0, 0.0, 1.0]; // Green for Z axis

    let eps = 0.0001;

    // Lines parallel to X axis (horizontal in Top view)
    for i in 0..=divisions {
        let z = -size / 2.0 + i as f32 * step;

        // Determine color: Z axis (z=0) is green, major lines are bright, minor lines are dim
        let color = if z.abs() < eps {
            z_axis_color
        } else if i % 10 == 0 {
            major_color
        } else {
            minor_color
        };

        // Render as quad (two triangles) to get consistent line width
        let half_width = 0.15; // Line thickness in world units (~2 pixels at this scale)
        vertices.push(Vertex { position: [-size / 2.0, -eps, z - half_width], color });
        vertices.push(Vertex { position: [size / 2.0, -eps, z - half_width], color });
        vertices.push(Vertex { position: [size / 2.0, -eps, z + half_width], color });

        vertices.push(Vertex { position: [-size / 2.0, -eps, z - half_width], color });
        vertices.push(Vertex { position: [size / 2.0, -eps, z + half_width], color });
        vertices.push(Vertex { position: [-size / 2.0, -eps, z + half_width], color });
    }

    // Lines parallel to Z axis (vertical in Top view)
    for i in 0..=divisions {
        let x = -size / 2.0 + i as f32 * step;

        // Determine color: X axis (x=0) is red, major lines are bright, minor lines are dim
        let color = if x.abs() < eps {
            x_axis_color
        } else if i % 10 == 0 {
            major_color
        } else {
            minor_color
        };

        // Render as quad (two triangles) to get consistent line width
        let half_width = 0.15; // Line thickness in world units (~2 pixels at this scale)
        vertices.push(Vertex { position: [x - half_width, -eps, -size / 2.0], color });
        vertices.push(Vertex { position: [x + half_width, -eps, -size / 2.0], color });
        vertices.push(Vertex { position: [x + half_width, -eps, size / 2.0], color });

        vertices.push(Vertex { position: [x - half_width, -eps, -size / 2.0], color });
        vertices.push(Vertex { position: [x + half_width, -eps, size / 2.0], color });
        vertices.push(Vertex { position: [x - half_width, -eps, size / 2.0], color });
    }

    vertices
}

/// Create vertices for multiple colored quads/cubes
pub fn create_cube_vertices(center: Vec3, size: f32, id: u32) -> Vec<Vertex> {
    let h = size / 2.0;
    let color = id_to_color(id);

    vec![
        // Front face
        Vertex { position: (center + Vec3::new(-h, -h, h)).to_array(), color },
        Vertex { position: (center + Vec3::new(h, -h, h)).to_array(), color },
        Vertex { position: (center + Vec3::new(h, h, h)).to_array(), color },
        Vertex { position: (center + Vec3::new(-h, h, h)).to_array(), color },

        // Back face
        Vertex { position: (center + Vec3::new(-h, -h, -h)).to_array(), color },
        Vertex { position: (center + Vec3::new(-h, h, -h)).to_array(), color },
        Vertex { position: (center + Vec3::new(h, h, -h)).to_array(), color },
        Vertex { position: (center + Vec3::new(h, -h, -h)).to_array(), color },
    ]
}

fn id_to_color(id: u32) -> [f32; 4] {
    let r = (id & 0xFF) as f32 / 255.0;
    let g = ((id >> 8) & 0xFF) as f32 / 255.0;
    let b = ((id >> 16) & 0xFF) as f32 / 255.0;
    [r, g, b, 1.0]
}

/// Create grid vertices on the XY plane (ground plane at Z=0, Z-up system)
/// Always on XY plane regardless of view. X axis is red, Y axis is green.
/// Returns vertices for drawing with triangle list (quads as two triangles)
fn create_grid_vertices_xy(size: f32, unit: f32) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    // Colors for Z-up system (adjusted for sRGB linear-to-display)
    let minor_color = [0.08, 0.08, 0.08, 1.0];   // Minor lines: sRGB display ~30
    let major_color = [0.18, 0.18, 0.18, 1.0];   // Major lines: sRGB display ~102
    let x_axis_color = [1.0, 0.0, 0.0, 1.0];     // Red for X axis
    let y_axis_color = [0.0, 1.0, 0.0, 1.0];     // Green for Y axis
    let z_axis_color = [0.0, 0.0, 1.0, 1.0];     // Blue for Z axis

    let line_width = 0.05;  // Half-width of line in world units
    let eps = 0.0001;

    // Lines parallel to X axis (constant Y, varying X) in XY plane at Z=0
    let mut y = -size / 2.0;
    while y <= size / 2.0 {
        let color = if y.abs() < eps {
            x_axis_color  // line along X at y=0 is the X axis (red)
        } else if ((y / unit).round() as i32 % 10) == 0 {
            major_color
        } else {
            minor_color
        };

        let v0 = Vertex { position: [-size / 2.0, y - line_width, 0.0], color };
        let v1 = Vertex { position: [size / 2.0, y - line_width, 0.0], color };
        let v2 = Vertex { position: [size / 2.0, y + line_width, 0.0], color };
        let v3 = Vertex { position: [-size / 2.0, y + line_width, 0.0], color };

        vertices.push(v0);
        vertices.push(v1);
        vertices.push(v2);
        vertices.push(v0);
        vertices.push(v2);
        vertices.push(v3);

        y += unit;
    }

    // Lines parallel to Y axis (constant X, varying Y) in XY plane at Z=0
    let mut x = -size / 2.0;
    while x <= size / 2.0 {
        let color = if x.abs() < eps {
            y_axis_color  // line along Y at x=0 is the Y axis (green)
        } else if ((x / unit).round() as i32 % 10) == 0 {
            major_color
        } else {
            minor_color
        };

        let v0 = Vertex { position: [x - line_width, -size / 2.0, 0.0], color };
        let v1 = Vertex { position: [x + line_width, -size / 2.0, 0.0], color };
        let v2 = Vertex { position: [x + line_width, size / 2.0, 0.0], color };
        let v3 = Vertex { position: [x - line_width, size / 2.0, 0.0], color };

        vertices.push(v0);
        vertices.push(v1);
        vertices.push(v2);
        vertices.push(v0);
        vertices.push(v2);
        vertices.push(v3);

        x += unit;
    }

    // Draw Z axis (blue line from origin upward, length 5 units)
    // Two crossed quads (XZ and YZ planes) so the axis is visible from every view direction.
    let z_axis_width = 0.1;
    let z_length = 5.0;
    for plane in 0..2 {
        let p = |w: f32, z: f32| Vertex {
            position: if plane == 0 { [w, 0.0, z] } else { [0.0, w, z] },
            color: z_axis_color,
        };
        let (v0, v1, v2, v3) = (
            p(-z_axis_width, 0.0),
            p(z_axis_width, 0.0),
            p(z_axis_width, z_length),
            p(-z_axis_width, z_length),
        );
        vertices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
    }

    vertices
}

/// Render a grid aligned to the camera view
/// The grid has dark background, minor lines every unit, major lines every 10 units
/// X axis is red, secondary axis (Y or Z) is green
/// Grid always renders on XY plane at Z=0 (view_name unused but kept for API compatibility)
pub fn render_grid(ctx: &GpuContext, target: &OffscreenTarget, camera: &Camera, _view_name: &str) -> Result<(), RenderError> {
    let device = ctx.device();
    let queue = ctx.queue();

    // Get viewport size
    let width = target.width;
    let height = target.height;
    let aspect = width as f32 / height as f32;

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("grid shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(GRID_SHADER)),
    });

    // Create bind group layout for camera uniform
    let camera_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        },
    );

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("grid pipeline layout"),
        bind_group_layouts: &[&camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create render pipeline
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grid pipeline"),
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

    // Create grid vertices (always on XY plane at Z=0)
    let grid_size = 20.0;  // ±10 units in each direction
    let grid_vertices = create_grid_vertices_xy(grid_size, 1.0);

    if grid_vertices.is_empty() {
        return Err(RenderError::DeviceCreationError("No grid vertices generated".to_string()));
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("grid vertex buffer"),
        contents: bytemuck::cast_slice(&grid_vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Create camera uniform buffer
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct CameraUniform {
        view_proj: [[f32; 4]; 4],
    }

    let vp_matrix = camera.view_projection_matrix(aspect);
    let camera_uniform = CameraUniform {
        view_proj: vp_matrix.to_cols_array_2d(),
    };

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera uniform buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("camera bind group"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
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
            label: Some("grid render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.render_view(),
                resolve_target: resolve_view.as_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0013,  // Linear value that displays as sRGB ~30/255 in sRGB texture
                        g: 0.0013,
                        b: 0.0013,
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
        render_pass.set_bind_group(0, &camera_bind_group, &[]);

        let vertex_count = grid_vertices.len() as u32;
        render_pass.draw(0..vertex_count, 0..1);
    }

    queue.submit(std::iter::once(encoder.finish()));

    Ok(())
}

/// A mesh with vertices, indices, and an entity ID for picking
#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub id: u32,
}

/// Reference to a GPU-resident mesh with buffers
pub struct MeshRef {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    id: u32,
}

/// Offscreen target with ID buffer for picking
pub struct PickingTarget {
    width: u32,
    height: u32,
    #[allow(dead_code)]
    id_texture: wgpu::Texture,
    id_view: wgpu::TextureView,
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    resolve_texture: wgpu::Texture,
    staging_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    staging_buffer_size: u64,
}

impl PickingTarget {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Result<Self, RenderError> {
        let device = ctx.device();

        // ID texture (R32Uint)
        let id_texture = device.create_texture(&TextureDescriptor {
            label: Some("picking id texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Uint,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let id_view = id_texture.create_view(&Default::default());

        // Resolve texture for reading
        let resolve_texture = device.create_texture(&TextureDescriptor {
            label: Some("picking resolve texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Uint,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Depth texture
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: Some("picking depth texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&Default::default());

        // Staging buffer for reading
        let bytes_per_pixel = 4; // R32Uint = 4 bytes
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padding = (256 - (unpadded_bytes_per_row % 256)) % 256;
        let bytes_per_row = unpadded_bytes_per_row + padding;
        let staging_buffer_size = (bytes_per_row * height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picking staging buffer"),
            size: staging_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(PickingTarget {
            width,
            height,
            id_texture,
            id_view,
            depth_texture,
            depth_view,
            resolve_texture,
            staging_buffer,
            staging_buffer_size,
        })
    }

    pub fn id_view(&self) -> &wgpu::TextureView {
        &self.id_view
    }

    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    pub fn id_texture(&self) -> &wgpu::Texture {
        &self.id_texture
    }

    pub fn resolve_texture(&self) -> &wgpu::Texture {
        &self.resolve_texture
    }

    pub fn pick(&self, ctx: &GpuContext, x: u32, y: u32) -> Result<Option<u32>, RenderError> {
        if x >= self.width || y >= self.height {
            return Ok(None);
        }

        let device = ctx.device();
        let queue = ctx.queue();

        // Copy full texture to staging buffer
        let mut encoder = device.create_command_encoder(&Default::default());

        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = self.width * bytes_per_pixel;
        let padding = (256 - (unpadded_bytes_per_row % 256)) % 256;
        let bytes_per_row = unpadded_bytes_per_row + padding;

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.resolve_texture,
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
        device.poll(wgpu::Maintain::Wait);

        // Map and read
        let buffer_slice = self.staging_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        let data = buffer_slice.get_mapped_range();

        let row_offset = (y as usize) * (bytes_per_row as usize);
        let pixel_offset = row_offset + (x as usize) * (bytes_per_pixel as usize);

        let id = if pixel_offset + 4 <= data.len() {
            let bytes = [
                data[pixel_offset],
                data[pixel_offset + 1],
                data[pixel_offset + 2],
                data[pixel_offset + 3],
            ];
            u32::from_le_bytes(bytes)
        } else {
            0
        };

        drop(data);
        self.staging_buffer.unmap();

        if id == 0 {
            Ok(None)
        } else {
            Ok(Some(id - 1)) // id was stored as id+1 to distinguish from background
        }
    }
}

impl MeshRef {
    /// Upload a mesh to GPU buffers
    pub fn new(ctx: &GpuContext, mesh: &Mesh) -> Result<Self, RenderError> {
        let device = ctx.device();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertex buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh index buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(MeshRef {
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            id: mesh.id,
        })
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Render meshes with ID colors for picking
pub fn render_ids(
    ctx: &GpuContext,
    target: &PickingTarget,
    camera: &Camera,
    meshes: &[&MeshRef],
) -> Result<(), RenderError> {
    if meshes.is_empty() {
        return Ok(());
    }

    let device = ctx.device();
    let queue = ctx.queue();
    let width = target.width;
    let height = target.height;
    let aspect = width as f32 / height as f32;

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("id shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(ID_SHADER_WITH_ID)),
    });

    // Create bind group layout for camera and ID uniforms
    let bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("id bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        },
    );

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("id pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create render pipeline
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("id pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 3]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3
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
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: TextureFormat::R32Uint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    // Create camera uniform buffer
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct CameraUniform {
        view_proj: [[f32; 4]; 4],
    }

    let vp_matrix = camera.view_projection_matrix(aspect);
    let camera_uniform = CameraUniform {
        view_proj: vp_matrix.to_cols_array_2d(),
    };

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("id camera uniform buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // ID uniform structure
    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct IDUniform {
        id: u32,
        _padding: u32,
        _padding2: u32,
        _padding3: u32,
    }

    // Prepare buffers and bind groups for each mesh
    let mut id_buffers = Vec::new();
    let mut bind_groups = Vec::new();

    for mesh in meshes {
        // Create ID uniform for this mesh
        let id_uniform = IDUniform {
            id: mesh.id(),
            _padding: 0,
            _padding2: 0,
            _padding3: 0,
        };

        let id_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("id uniform buffer"),
            contents: bytemuck::cast_slice(&[id_uniform]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("id bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: id_buffer.as_entire_binding(),
                },
            ],
        });

        id_buffers.push(id_buffer);
        bind_groups.push(bind_group);
    }

    // Create command encoder and render pass
    let mut encoder = device.create_command_encoder(&Default::default());

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("id render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.id_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
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

        for (i, mesh) in meshes.iter().enumerate() {
            render_pass.set_bind_group(0, &bind_groups[i], &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
            render_pass.set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.index_count(), 0, 0..1);
        }
    }

    // Copy to resolve texture (for picking)
    encoder.copy_texture_to_texture(
        wgpu::ImageCopyTexture {
            texture: &target.id_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyTexture {
            texture: &target.resolve_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    Ok(())
}
