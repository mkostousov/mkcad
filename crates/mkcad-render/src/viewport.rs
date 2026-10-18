//! Persistent interactive viewport renderer (M3 stage 3a).
//!
//! Unlike the headless spike functions in `lib.rs`, a [`ViewportRenderer`] keeps its pipelines and
//! GPU meshes alive between frames and renders with a device owned by someone else (the app's
//! eframe/egui-wgpu device, or a [`crate::GpuContext`] in tests). Output: an MSAA-resolved sRGB colour
//! texture that the UI shows, plus an on-demand ID pass for picking (VW-06).

use crate::Camera;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Colour format of the resolved viewport image (sampled by the UI).
pub const VIEWPORT_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const ID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;
/// MSAA sample count of the colour pass.
pub const SAMPLE_COUNT: u32 = 4;

const BACKGROUND: wgpu::Color = wgpu::Color { r: 0.16, g: 0.17, b: 0.19, a: 1.0 };
const HIGHLIGHT: [f32; 4] = [1.0, 0.62, 0.12, 1.0];

const SHADED_SHADER: &str = r"
struct Cam { view_proj: mat4x4<f32>, light_dir: vec4<f32> };
struct Obj { color: vec4<f32>, id: vec4<u32> };
@group(0) @binding(0) var<uniform> cam: Cam;
@group(1) @binding(0) var<uniform> obj: Obj;

struct VsIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32> };
struct VsOut { @builtin(position) clip: vec4<f32>, @location(0) normal: vec3<f32> };

@vertex fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = cam.view_proj * vec4<f32>(v.pos, 1.0);
    o.normal = v.normal;
    return o;
}

@fragment fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    // Headlight: light comes from the camera, both faces lit (open shells stay readable).
    let d = abs(dot(normalize(i.normal), normalize(cam.light_dir.xyz)));
    return vec4<f32>(obj.color.rgb * (0.3 + 0.7 * d), obj.color.a);
}

@fragment fn fs_id(i: VsOut) -> @location(0) u32 {
    return obj.id.x + 1u;
}
";

const LINE_SHADER: &str = r"
struct Cam { view_proj: mat4x4<f32>, light_dir: vec4<f32> };
@group(0) @binding(0) var<uniform> cam: Cam;

struct VsIn { @location(0) pos: vec3<f32>, @location(1) color: vec4<f32> };
struct VsOut { @builtin(position) clip: vec4<f32>, @location(0) color: vec4<f32> };

@vertex fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = cam.view_proj * vec4<f32>(v.pos, 1.0);
    o.color = v.color;
    return o;
}

@fragment fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    return i.color;
}
";

/// Triangle mesh in world coordinates (mm), one normal per vertex.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Axis-aligned box with flat-shaded faces (24 vertices, 12 triangles).
    #[must_use]
    pub fn cuboid(min: Vec3, max: Vec3) -> Self {
        let faces: [(Vec3, [Vec3; 4]); 6] = [
            (Vec3::X, [Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, max.y, min.z), Vec3::new(max.x, max.y, max.z), Vec3::new(max.x, min.y, max.z)]),
            (Vec3::NEG_X, [Vec3::new(min.x, max.y, min.z), Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, min.y, max.z), Vec3::new(min.x, max.y, max.z)]),
            (Vec3::Y, [Vec3::new(max.x, max.y, min.z), Vec3::new(min.x, max.y, min.z), Vec3::new(min.x, max.y, max.z), Vec3::new(max.x, max.y, max.z)]),
            (Vec3::NEG_Y, [Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, min.y, max.z), Vec3::new(min.x, min.y, max.z)]),
            (Vec3::Z, [Vec3::new(min.x, min.y, max.z), Vec3::new(max.x, min.y, max.z), Vec3::new(max.x, max.y, max.z), Vec3::new(min.x, max.y, max.z)]),
            (Vec3::NEG_Z, [Vec3::new(min.x, max.y, min.z), Vec3::new(max.x, max.y, min.z), Vec3::new(max.x, min.y, min.z), Vec3::new(min.x, min.y, min.z)]),
        ];
        let mut m = Self::default();
        for (n, quad) in faces {
            let base = u32::try_from(m.positions.len()).unwrap_or(u32::MAX);
            for p in quad {
                m.positions.push(p.to_array());
                m.normals.push(n.to_array());
            }
            m.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        m
    }

    /// Flat-shaded grid of `n`×`n` cells on a square in the XY plane (2·n² triangles); for load tests.
    #[must_use]
    pub fn plane_grid(origin: Vec3, size: f32, n: u32) -> Self {
        let mut m = Self::default();
        let step = size / n as f32;
        for j in 0..=n {
            for i in 0..=n {
                m.positions.push((origin + Vec3::new(i as f32 * step, j as f32 * step, 0.0)).to_array());
                m.normals.push([0.0, 0.0, 1.0]);
            }
        }
        let row = n + 1;
        for j in 0..n {
            for i in 0..n {
                let a = j * row + i;
                m.indices.extend_from_slice(&[a, a + 1, a + row + 1, a, a + row + 1, a + row]);
            }
        }
        m
    }

    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MeshVertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    pos: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CamUniform {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ObjUniform {
    color: [f32; 4],
    id: [u32; 4],
}

struct GpuMesh {
    id: u32,
    color: [f32; 4],
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct Targets {
    width: u32,
    height: u32,
    msaa_view: wgpu::TextureView,
    #[allow(dead_code)]
    msaa: wgpu::Texture,
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    id_view: wgpu::TextureView,
    id: wgpu::Texture,
    id_depth_view: wgpu::TextureView,
}

impl Targets {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let tex = |label: &str, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let msaa = tex("viewport msaa", VIEWPORT_COLOR_FORMAT, SAMPLE_COUNT, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let color = tex(
            "viewport color",
            VIEWPORT_COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = tex("viewport depth", DEPTH_FORMAT, SAMPLE_COUNT, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let id = tex("viewport id", ID_FORMAT, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
        let id_depth = tex("viewport id depth", DEPTH_FORMAT, 1, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            width,
            height,
            msaa_view: view(&msaa),
            msaa,
            color_view: view(&color),
            color,
            depth_view: view(&depth),
            id_view: view(&id),
            id,
            id_depth_view: view(&id_depth),
        }
    }
}

/// Interactive 3D viewport: grid + axes, shaded meshes, selection highlight, ID picking.
pub struct ViewportRenderer {
    targets: Targets,
    cam_buffer: wgpu::Buffer,
    cam_bind_group: wgpu::BindGroup,
    obj_layout: wgpu::BindGroupLayout,
    shaded: wgpu::RenderPipeline,
    id_pipeline: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    meshes: Vec<GpuMesh>,
    highlight: Option<u32>,
}

impl ViewportRenderer {
    /// Create a renderer with an initial size in physical pixels (clamped to at least 1×1).
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let uniform_entry = |vis| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: vis,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let cam_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport cam layout"),
            entries: &[uniform_entry(wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let obj_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport obj layout"),
            entries: &[uniform_entry(wgpu::ShaderStages::FRAGMENT)],
        });
        let cam_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport cam"),
            size: std::mem::size_of::<CamUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cam_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport cam bg"),
            layout: &cam_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: cam_buffer.as_entire_binding() }],
        });

        let shaded_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport shaded"),
            source: wgpu::ShaderSource::Wgsl(SHADED_SHADER.into()),
        });
        let line_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport lines"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });
        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport mesh pl"),
            bind_group_layouts: &[&cam_layout, &obj_layout],
            push_constant_ranges: &[],
        });
        let line_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport line pl"),
            bind_group_layouts: &[&cam_layout],
            push_constant_ranges: &[],
        });

        let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let mesh_vb = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &mesh_attrs,
        };
        let line_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let line_vb = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &line_attrs,
        };
        let depth = |write: bool, samples: u32| {
            (
                Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            )
        };

        let (ds, ms) = depth(true, SAMPLE_COUNT);
        let shaded = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport shaded"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState { module: &shaded_module, entry_point: "vs_main", buffers: &[mesh_vb.clone()] },
            fragment: Some(wgpu::FragmentState {
                module: &shaded_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: ds,
            multisample: ms,
            multiview: None,
        });

        let (ds, ms) = depth(true, 1);
        let id_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport id"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState { module: &shaded_module, entry_point: "vs_main", buffers: &[mesh_vb] },
            fragment: Some(wgpu::FragmentState {
                module: &shaded_module,
                entry_point: "fs_id",
                targets: &[Some(wgpu::ColorTargetState {
                    format: ID_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: ds,
            multisample: ms,
            multiview: None,
        });

        // Grid lines do not write depth, so coplanar faces (sketch planes, z = 0 bottoms) win.
        let (ds, ms) = depth(false, SAMPLE_COUNT);
        let lines = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport lines"),
            layout: Some(&line_layout),
            vertex: wgpu::VertexState { module: &line_module, entry_point: "vs_main", buffers: &[line_vb] },
            fragment: Some(wgpu::FragmentState {
                module: &line_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..Default::default() },
            depth_stencil: ds,
            multisample: ms,
            multiview: None,
        });

        let grid = grid_lines(500.0, 10.0, 5);
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport grid"),
            contents: bytemuck::cast_slice(&grid),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            targets: Targets::new(device, width.max(1), height.max(1)),
            cam_buffer,
            cam_bind_group,
            obj_layout,
            shaded,
            id_pipeline,
            lines,
            grid_buffer,
            grid_vertex_count: u32::try_from(grid.len()).unwrap_or(0),
            meshes: Vec::new(),
            highlight: None,
        }
    }

    /// Current size in physical pixels.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.targets.width, self.targets.height)
    }

    /// Resize the render targets; returns true if the colour texture was recreated (the UI must
    /// re-register it).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) -> bool {
        let (w, h) = (width.max(1), height.max(1));
        if (w, h) == self.size() {
            return false;
        }
        self.targets = Targets::new(device, w, h);
        true
    }

    /// View of the resolved colour image, for registration with the UI.
    #[must_use]
    pub const fn color_view(&self) -> &wgpu::TextureView {
        &self.targets.color_view
    }

    /// Upload a mesh under an entity `id` (the value returned by [`Self::pick`]).
    pub fn add_mesh(&mut self, device: &wgpu::Device, id: u32, mesh: &MeshData, color: [f32; 4]) {
        let vertices: Vec<MeshVertex> = mesh
            .positions
            .iter()
            .zip(&mesh.normals)
            .map(|(p, n)| MeshVertex { pos: *p, normal: *n })
            .collect();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport mesh vb"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport mesh ib"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport mesh obj"),
            contents: bytemuck::bytes_of(&ObjUniform { color, id: [id, 0, 0, 0] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport mesh bg"),
            layout: &self.obj_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        self.meshes.push(GpuMesh {
            id,
            color,
            vertex_buffer,
            index_buffer,
            index_count: u32::try_from(mesh.indices.len()).unwrap_or(0),
            uniform,
            bind_group,
        });
    }

    /// Remove all meshes.
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
        self.highlight = None;
    }

    /// Total triangles uploaded.
    #[must_use]
    pub fn triangle_count(&self) -> u64 {
        self.meshes.iter().map(|m| u64::from(m.index_count / 3)).sum()
    }

    /// Highlight one entity (selection colour) or none.
    pub fn set_highlight(&mut self, queue: &wgpu::Queue, id: Option<u32>) {
        self.highlight = id;
        for m in &self.meshes {
            let color = if Some(m.id) == id { HIGHLIGHT } else { m.color };
            queue.write_buffer(&m.uniform, 0, bytemuck::bytes_of(&ObjUniform { color, id: [m.id, 0, 0, 0] }));
        }
    }

    #[must_use]
    pub const fn highlight(&self) -> Option<u32> {
        self.highlight
    }

    fn write_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let (w, h) = self.size();
        let vp: Mat4 = camera.view_projection_matrix(w as f32 / h as f32);
        let light = -camera.forward();
        let u = CamUniform { view_proj: vp.to_cols_array_2d(), light_dir: [light.x, light.y, light.z, 0.0] };
        queue.write_buffer(&self.cam_buffer, 0, bytemuck::bytes_of(&u));
    }

    /// Render the scene into the colour texture and submit.
    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, camera: &Camera) {
        self.write_camera(queue, camera);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("viewport") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport colour"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.msaa_view,
                    resolve_target: Some(&self.targets.color_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BACKGROUND), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.cam_bind_group, &[]);
            pass.set_pipeline(&self.shaded);
            for m in &self.meshes {
                pass.set_bind_group(1, &m.bind_group, &[]);
                pass.set_vertex_buffer(0, m.vertex_buffer.slice(..));
                pass.set_index_buffer(m.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..m.index_count, 0, 0..1);
            }
            pass.set_pipeline(&self.lines);
            pass.set_vertex_buffer(0, self.grid_buffer.slice(..));
            pass.draw(0..self.grid_vertex_count, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Entity id under pixel (x, y) (physical pixels, origin top-left), or None for background.
    /// Renders the ID pass and reads back one pixel (blocking).
    pub fn pick(&self, device: &wgpu::Device, queue: &wgpu::Queue, camera: &Camera, x: u32, y: u32) -> Option<u32> {
        let (w, h) = self.size();
        if x >= w || y >= h {
            return None;
        }
        self.write_camera(queue, camera);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport pick staging"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("viewport pick") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport id"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.id_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.id_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Only the clicked pixel matters.
            pass.set_scissor_rect(x, y, 1, 1);
            pass.set_bind_group(0, &self.cam_bind_group, &[]);
            pass.set_pipeline(&self.id_pipeline);
            for m in &self.meshes {
                pass.set_bind_group(1, &m.bind_group, &[]);
                pass.set_vertex_buffer(0, m.vertex_buffer.slice(..));
                pass.set_index_buffer(m.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..m.index_count, 0, 0..1);
            }
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.targets.id,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(1) },
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        queue.submit(Some(encoder.finish()));
        let value = read_mapped(device, &staging, |bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))?;
        value.checked_sub(1)
    }

    /// Read the resolved colour image as tightly packed RGBA8 (blocking; tests and screenshots).
    pub fn read_color(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Vec<u8>> {
        let (w, h) = self.size();
        let row = w * 4;
        let padded = row.div_ceil(256) * 256;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport colour staging"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("viewport read") });
        encoder.copy_texture_to_buffer(
            self.targets.color.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        queue.submit(Some(encoder.finish()));
        read_mapped(device, &staging, |bytes| {
            let mut out = Vec::with_capacity((row * h) as usize);
            for r in 0..h as usize {
                let start = r * padded as usize;
                out.extend_from_slice(&bytes[start..start + row as usize]);
            }
            out
        })
    }
}

fn read_mapped<T>(device: &wgpu::Device, buffer: &wgpu::Buffer, f: impl FnOnce(&[u8]) -> T) -> Option<T> {
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    let value = f(&slice.get_mapped_range());
    buffer.unmap();
    Some(value)
}

/// XY-plane grid (Z up) of half-extent `size`/2 with minor lines every `step` mm and a major line
/// every `major_every` steps; X axis red, Y axis green, short Z axis blue.
fn grid_lines(size: f32, step: f32, major_every: i32) -> Vec<LineVertex> {
    let half = size / 2.0;
    let n = (half / step).round() as i32;
    let minor = [0.42, 0.44, 0.48, 0.35];
    let major = [0.55, 0.57, 0.62, 0.7];
    let mut v = Vec::new();
    let mut line = |a: [f32; 3], b: [f32; 3], color: [f32; 4]| {
        v.push(LineVertex { pos: a, color });
        v.push(LineVertex { pos: b, color });
    };
    for i in -n..=n {
        if i == 0 {
            continue;
        }
        let c = if i % major_every == 0 { major } else { minor };
        let t = i as f32 * step;
        line([t, -half, 0.0], [t, half, 0.0], c);
        line([-half, t, 0.0], [half, t, 0.0], c);
    }
    line([-half, 0.0, 0.0], [half, 0.0, 0.0], [0.9, 0.25, 0.25, 1.0]);
    line([0.0, -half, 0.0], [0.0, half, 0.0], [0.3, 0.8, 0.3, 1.0]);
    line([0.0, 0.0, 0.0], [0.0, 0.0, half / 5.0], [0.3, 0.5, 1.0, 1.0]);
    v
}

/// Camera framing all of `bounds` (min, max) from the iso direction, perspective 45°.
#[must_use]
pub fn frame_bounds(min: Vec3, max: Vec3) -> Camera {
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(1.0);
    let distance = radius / (22.5_f32.to_radians()).sin() * 1.1;
    let mut cam = Camera::orthographic_view("iso", center, distance);
    cam.projection = crate::camera::ProjectionMode::Perspective(45.0);
    cam
}
