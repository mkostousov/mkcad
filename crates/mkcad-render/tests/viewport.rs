//! M3 stage 3a: persistent ViewportRenderer (colour pass, picking, highlight, resize, load).

use glam::Vec3;
use mkcad_render::camera::ProjectionMode;
use mkcad_render::viewport::{frame_bounds, MeshData, ViewportRenderer};
use mkcad_render::{Camera, GpuContext};
use std::time::Instant;

fn gpu() -> Option<GpuContext> {
    match pollster::block_on(GpuContext::new_headless()) {
        Ok(c) => Some(c),
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            None
        }
    }
}

const GREY: [f32; 4] = [0.7, 0.72, 0.75, 1.0];
const BLUE: [f32; 4] = [0.3, 0.45, 0.85, 1.0];

/// Two boxes side by side: id 1 at x ∈ [-30,-10], id 2 at x ∈ [10,30]; both 20×20×20 mm.
fn two_boxes(ctx: &GpuContext, w: u32, h: u32) -> ViewportRenderer {
    let mut vp = ViewportRenderer::new(ctx.device(), w, h);
    vp.add_mesh(ctx.device(), 1, &MeshData::cuboid(Vec3::new(-30.0, -10.0, 0.0), Vec3::new(-10.0, 10.0, 20.0)), GREY);
    vp.add_mesh(ctx.device(), 2, &MeshData::cuboid(Vec3::new(10.0, -10.0, 0.0), Vec3::new(30.0, 10.0, 20.0)), BLUE);
    vp
}

fn pixel(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn cuboid_mesh_is_closed_and_outward() {
    let m = MeshData::cuboid(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(m.positions.len(), 24);
    assert_eq!(m.triangle_count(), 12);
    // Every triangle's winding agrees with its stored normal (counter-clockwise from outside).
    for t in m.indices.chunks(3) {
        let p = |i: u32| Vec3::from(m.positions[i as usize]);
        let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0])).normalize();
        assert!((n - Vec3::from(m.normals[t[0] as usize])).length() < 1e-6, "winding vs normal");
    }
    assert_eq!(MeshData::plane_grid(Vec3::ZERO, 10.0, 4).triangle_count(), 32);
}

#[test]
fn top_view_picks_each_box_and_background() {
    let Some(ctx) = gpu() else { return };
    let (w, h) = (200, 100);
    let vp = two_boxes(&ctx, w, h);
    // Top view, ortho half-height 50 mm → 1 px = 1 mm; screen x = 100 + world x, y = 50 - world y.
    let cam = Camera::orthographic_view("top", Vec3::ZERO, 200.0);
    assert_eq!(cam.projection, ProjectionMode::Orthographic(100.0));
    let cam = Camera { projection: ProjectionMode::Orthographic(50.0), ..cam };
    assert_eq!(vp.pick(ctx.device(), ctx.queue(), &cam, 80, 50), Some(1));
    assert_eq!(vp.pick(ctx.device(), ctx.queue(), &cam, 120, 50), Some(2));
    assert_eq!(vp.pick(ctx.device(), ctx.queue(), &cam, 100, 50), None);
    assert_eq!(vp.pick(ctx.device(), ctx.queue(), &cam, 5, 5), None);
    assert_eq!(vp.pick(ctx.device(), ctx.queue(), &cam, w, 0), None, "out of range");
}

#[test]
fn colour_pass_shows_boxes_and_highlight() {
    let Some(ctx) = gpu() else { return };
    let (w, h) = (200, 100);
    let mut vp = two_boxes(&ctx, w, h);
    let cam = Camera { projection: ProjectionMode::Orthographic(50.0), ..Camera::orthographic_view("top", Vec3::ZERO, 200.0) };

    vp.render(ctx.device(), ctx.queue(), &cam);
    let img = vp.read_color(ctx.device(), ctx.queue()).expect("readback");
    assert_eq!(img.len(), (w * h * 4) as usize);
    let bg = pixel(&img, w, 3, 3);
    let left = pixel(&img, w, 80, 45);
    let right = pixel(&img, w, 120, 45);
    assert_ne!(left, bg, "box 1 visible");
    assert!(right[2] > right[0] + 40, "box 2 is blue: {right:?}");

    vp.set_highlight(ctx.queue(), Some(1));
    vp.render(ctx.device(), ctx.queue(), &cam);
    let img = vp.read_color(ctx.device(), ctx.queue()).expect("readback");
    let hl = pixel(&img, w, 80, 45);
    assert!(hl[0] > hl[2] + 60, "box 1 highlighted orange: {hl:?}");
    assert_eq!(pixel(&img, w, 120, 45), right, "box 2 unchanged");
    assert_eq!(vp.highlight(), Some(1));

    if std::env::var("MKCAD_SAVE_PNG").is_ok() {
        let mut cam = frame_bounds(Vec3::new(-30.0, -10.0, 0.0), Vec3::new(30.0, 10.0, 20.0));
        cam.projection = ProjectionMode::Perspective(45.0);
        vp.render(ctx.device(), ctx.queue(), &cam);
        let img = vp.read_color(ctx.device(), ctx.queue()).expect("readback");
        mkcad_render::save_png("viewport_iso.png", w, h, &img).expect("png");
    }
}

#[test]
fn resize_recreates_targets_once() {
    let Some(ctx) = gpu() else { return };
    let mut vp = two_boxes(&ctx, 64, 64);
    assert!(!vp.resize(ctx.device(), 64, 64));
    assert!(vp.resize(ctx.device(), 300, 120));
    assert_eq!(vp.size(), (300, 120));
    assert!(vp.resize(ctx.device(), 0, 0), "zero is clamped to 1×1");
    assert_eq!(vp.size(), (1, 1));
}

#[test]
fn ortho_zoom_changes_view_height() {
    let mut cam = Camera::orthographic_view("top", Vec3::ZERO, 200.0);
    cam.zoom_about_cursor(1.0, (50.0, 50.0), (100, 100));
    match cam.projection {
        ProjectionMode::Orthographic(h) => assert!((h - 100.0 / 1.1).abs() < 1e-3, "h = {h}"),
        ProjectionMode::Perspective(_) => panic!("projection changed"),
    }
}

/// VW performance: 1M triangles in the persistent renderer at 1920×1080 with 4× MSAA.
/// Reports median frame time (submit + GPU completion); asserts only that it is under 1/30 s,
/// the 60 fps target is reported, not asserted (CI GPUs vary).
#[test]
fn million_triangles_frame_time() {
    let Some(ctx) = gpu() else { return };
    let mut vp = ViewportRenderer::new(ctx.device(), 1920, 1080);
    // 4 tiles × 2·354² ≈ 1.0M triangles.
    for (k, (x, y)) in [(-100.0, -100.0), (0.0, -100.0), (-100.0, 0.0), (0.0, 0.0)].into_iter().enumerate() {
        let mesh = MeshData::plane_grid(Vec3::new(x, y, 0.0), 100.0, 354);
        vp.add_mesh(ctx.device(), u32::try_from(k).unwrap(), &mesh, GREY);
    }
    assert!(vp.triangle_count() >= 1_000_000, "{}", vp.triangle_count());
    let mut cam = frame_bounds(Vec3::new(-100.0, -100.0, 0.0), Vec3::new(100.0, 100.0, 1.0));
    let mut times = Vec::new();
    for i in 0..30 {
        cam.orbit(3.0, 0.0, (1920, 1080));
        let t = Instant::now();
        vp.render(ctx.device(), ctx.queue(), &cam);
        ctx.device().poll(wgpu::Maintain::Wait);
        if i >= 5 {
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    eprintln!("VIEWPORT_1M_TRIS_MEDIAN_MS={median:.2} max={:.2}", times[times.len() - 1]);
    assert!(median < 33.3, "median {median:.2} ms");
}
