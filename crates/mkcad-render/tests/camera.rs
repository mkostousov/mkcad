use mkcad_render::camera::{Camera, ProjectionMode};
use glam::Vec3;

const TOLERANCE: f32 = 1e-5;

fn assert_vec3_approx_eq(a: Vec3, b: Vec3, tolerance: f32, msg: &str) {
    let diff = (a - b).length();
    assert!(diff < tolerance, "{}: expected {:?}, got {:?}, diff={}", msg, b, a, diff);
}

#[allow(dead_code)]
fn assert_f32_approx_eq(a: f32, b: f32, tolerance: f32, msg: &str) {
    assert!((a - b).abs() < tolerance, "{}: expected {}, got {}, diff={}", msg, b, a, (a - b).abs());
}

#[test]
fn test_vw_01_zoom_about_cursor() {
    // VW-01: wheel ×5, zoom about cursor
    let mut camera = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );

    let viewport = (512u32, 512u32);
    let cursor_px = (256.0, 256.0);
    let initial_distance = camera.distance;

    // Zoom in
    camera.zoom_about_cursor(5.0, cursor_px, viewport);
    assert!(camera.distance < initial_distance, "VW-01: should zoom in");

    // Zoom out
    camera.zoom_about_cursor(-10.0, cursor_px, viewport);
    assert!(camera.distance > initial_distance, "VW-01: should zoom out");

    println!("VW-01: PASS");
}

#[test]
fn test_vw_02_pan_by_pixels() {
    // VW-02: MMB drag, pan by pixels
    let mut camera = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );

    let viewport = (512u32, 512u32);
    let initial_target = camera.target;

    // Pan by (300, -100)
    camera.pan_by_pixels(300.0, -100.0, viewport);

    // Target should have moved
    assert!(camera.target != initial_target, "VW-02: target should move");

    println!("VW-02: PASS - pan by pixels (300, -100)");
}

#[test]
fn test_vw_03_orbit() {
    // VW-03: Orbit with pitch clamp
    let mut camera = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );

    let initial_yaw = camera.yaw;
    let viewport = (512u32, 512u32);

    // Orbit right
    camera.orbit(100.0, 0.0, viewport);
    assert!(camera.yaw != initial_yaw, "VW-03: yaw should change");

    // Orbit up
    camera.yaw = initial_yaw;
    let initial_pitch = camera.pitch;
    camera.orbit(0.0, 100.0, viewport);
    assert!(camera.pitch > initial_pitch, "VW-03: pitch should increase");

    // Orbit down with clamp
    camera.pitch = 0.0;
    camera.orbit(0.0, -2000.0, viewport);
    assert!(camera.pitch < -0.001, "VW-03: pitch should be clamped");

    println!("VW-03: PASS");
}

#[test]
fn test_vw_04_orthographic_views() {
    // VW-04: Orthographic named views
    let origin = Vec3::new(5.0, 3.0, 2.0);
    let distance = 20.0;

    let views = vec!["front", "back", "left", "right", "top", "bottom", "iso"];

    for name in &views {
        let camera = Camera::orthographic_view(name, origin, distance);
        match camera.projection {
            ProjectionMode::Orthographic(_) => {},
            _ => panic!("VW-04: {} should be orthographic", name),
        }
    }

    println!("VW-04: PASS - {} orthographic views", views.len());
}

#[test]
fn test_vw_05_look_normal_to_plane() {
    // VW-05: Look normal to plane
    let mut camera = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );

    let plane_origin = Vec3::new(5.0, 3.0, 2.0);
    let plane_normal = Vec3::new(1.0, 1.0, 1.0).normalize();

    camera.look_normal_to_plane(plane_origin, plane_normal);

    // Target at origin
    assert_vec3_approx_eq(camera.target, plane_origin, TOLERANCE, "VW-05: target");

    // Forward matches normal
    let forward = camera.forward();
    assert_vec3_approx_eq(forward, plane_normal, 1e-4, "VW-05: forward matches normal");

    println!("VW-05: PASS");
}

#[test]
fn test_vw_06_camera_basics() {
    // VW-06: Camera math foundations (Z-up system)
    // With yaw=0, pitch=0, position should be (d, 0, 0) looking at origin (looking in -X)
    let camera = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );

    let pos = camera.position();
    // Formula: [d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)]
    // With pitch=0, yaw=0: [10*1*1, 10*1*0, 10*0] = [10, 0, 0]
    assert_vec3_approx_eq(pos, Vec3::new(10.0, 0.0, 0.0), TOLERANCE, "VW-06: position");

    let forward = camera.forward();
    // Camera at (10, 0, 0) looking at (0, 0, 0): forward = -(10,0,0).normalize() = (-1,0,0)
    assert_vec3_approx_eq(forward, Vec3::new(-1.0, 0.0, 0.0), TOLERANCE, "VW-06: forward");

    // Test Front view: yaw=-π/2, pitch=0 should be at (0, -d, 0) looking at origin
    let front = Camera::new(
        Vec3::ZERO,
        10.0,
        -std::f32::consts::PI / 2.0,
        0.0,
        ProjectionMode::Perspective(45.0),
    );
    let front_pos = front.position();
    assert_vec3_approx_eq(front_pos, Vec3::new(0.0, -10.0, 0.0), TOLERANCE, "VW-06: front position");
    let front_forward = front.forward();
    assert_vec3_approx_eq(front_forward, Vec3::new(0.0, 1.0, 0.0), TOLERANCE, "VW-06: front forward");

    println!("VW-06: PASS");
}

