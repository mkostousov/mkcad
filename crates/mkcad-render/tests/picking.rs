use mkcad_render::{Camera, GpuContext, PickingTarget, Mesh, MeshRef, render_ids};
use glam::Vec3;

/// Helper to create a centered square in XY plane at given Z
fn create_square_xy(z: f32, id: u32) -> Mesh {
    let vertices = vec![
        [-1.0, -1.0, z],
        [1.0, -1.0, z],
        [1.0, 1.0, z],
        [-1.0, 1.0, z],
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    Mesh { vertices, indices, id }
}

#[tokio::test]
async fn picking_single_mesh_top_view() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match PickingTarget::new(&ctx, width, height) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create picking target: {}", e),
    };

    // Create a single square at z=0 with id=42, in top-down view
    let square = create_square_xy(0.0, 42);
    let mesh = MeshRef::new(&ctx, &square).expect("Failed to create mesh");

    // Top view: camera looking down at origin
    let camera = Camera::orthographic_view("top", Vec3::ZERO, 5.0);

    let meshes = vec![&mesh];
    render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render");

    // Pick at center
    let center_id = target.pick(&ctx, width / 2, height / 2)
        .expect("Failed to pick center");

    match center_id {
        Some(id) => {
            assert_eq!(id, 42, "Center should pick square with id=42, got {}", id);
            println!("Single mesh test PASSED: center={}", id);
        }
        None => {
            panic!("Failed to pick at center, got None. The mesh may not be rendering.");
        }
    }
}

#[tokio::test]
async fn picking_depth_test() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match PickingTarget::new(&ctx, width, height) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create picking target: {}", e),
    };

    // Two squares at different depths: farther at z=-2, closer at z=2
    // In top view (looking down from +Z), larger z is closer to camera
    let square_far = create_square_xy(-2.0, 10);
    let square_near = create_square_xy(2.0, 20);

    let mesh_far = MeshRef::new(&ctx, &square_far).expect("Failed to create far mesh");
    let mesh_near = MeshRef::new(&ctx, &square_near).expect("Failed to create near mesh");

    // Top view: camera at (0, 0, 10) looking down
    let camera = Camera::orthographic_view("top", Vec3::ZERO, 5.0);

    // Render far first, then near. Due to depth testing, near should win.
    let meshes = vec![&mesh_far, &mesh_near];
    render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render");

    // Pick at center - should get id=20 (the nearer square)
    match target.pick(&ctx, width / 2, height / 2) {
        Ok(Some(id)) => {
            assert_eq!(id, 20, "Should pick nearer square (id=20), got id={}", id);
            println!("Depth test PASSED: got nearer square id=20");
        }
        Ok(None) => {
            panic!("Picked None instead of id=20");
        }
        Err(e) => {
            panic!("Pick error: {:?}", e);
        }
    }

    // Draw order must not matter: near first, far second, perspective and ortho alike.
    let meshes = vec![&mesh_near, &mesh_far];
    render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render");
    assert_eq!(
        target.pick(&ctx, width / 2, height / 2).expect("pick"),
        Some(20),
        "near square must win regardless of draw order"
    );
    let persp = Camera::new(
        Vec3::ZERO,
        10.0,
        0.0,
        std::f32::consts::FRAC_PI_2 - 0.001,
        mkcad_render::camera::ProjectionMode::Perspective(45.0),
    );
    let meshes = vec![&mesh_far, &mesh_near];
    render_ids(&ctx, &target, &persp, &meshes).expect("Failed to render");
    assert_eq!(
        target.pick(&ctx, width / 2, height / 2).expect("pick"),
        Some(20),
        "near square must win under perspective"
    );
}

#[tokio::test]
async fn picking_out_of_bounds() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match PickingTarget::new(&ctx, width, height) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create picking target: {}", e),
    };

    // Out of bounds should return None, not panic
    let result = target.pick(&ctx, width + 100, height + 100);
    assert!(result.is_ok(), "Out of bounds should not panic");
    assert!(result.unwrap().is_none(), "Out of bounds should return None");

    println!("Out of bounds test PASSED");
}

#[tokio::test]
async fn picking_empty_mesh_list() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 128u32;
    let height = 128u32;

    let target = match PickingTarget::new(&ctx, width, height) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create picking target: {}", e),
    };

    let camera = Camera::default();

    // Render with empty mesh list
    let meshes: Vec<&MeshRef> = vec![];
    render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render empty mesh list");

    // Pick should return None
    let result = target.pick(&ctx, width / 2, height / 2).expect("Failed to pick");
    assert!(result.is_none(), "Empty scene should pick background");

    println!("Empty mesh list test PASSED");
}

#[tokio::test]
async fn picking_vw_06_performance() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let gpu_required = std::env::var("MKCAD_GPU_REQUIRED")
        .map(|v| v == "1")
        .unwrap_or(false);

    let adapter_info = ctx.adapter_info();
    println!("Adapter: {} ({}, {})",
        adapter_info.name,
        adapter_info.backend,
        adapter_info.device_type
    );

    let width = 512u32;
    let height = 512u32;

    let target = match PickingTarget::new(&ctx, width, height) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create picking target: {}", e),
    };

    // Create a 708x708 heightfield (~1M triangles)
    let grid_size = 708;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for z in 0..=grid_size {
        for x in 0..=grid_size {
            let xf = (x as f32 - grid_size as f32 / 2.0) / 200.0;
            let zf = (z as f32 - grid_size as f32 / 2.0) / 200.0;
            let yf = (xf * xf + zf * zf).sqrt() * 0.05;
            vertices.push([xf, yf, zf]);
        }
    }

    for z in 0..grid_size {
        for x in 0..grid_size {
            let i0 = z * (grid_size + 1) + x;
            let i1 = i0 + 1;
            let i2 = i0 + (grid_size + 1);
            let i3 = i2 + 1;

            indices.push(i0 as u32);
            indices.push(i2 as u32);
            indices.push(i1 as u32);

            indices.push(i1 as u32);
            indices.push(i2 as u32);
            indices.push(i3 as u32);
        }
    }

    let triangle_count = indices.len() / 3;
    println!("Created heightfield mesh with {} triangles", triangle_count);

    let large_mesh = Mesh {
        vertices,
        indices,
        id: 1000,
    };

    let mesh_large = MeshRef::new(&ctx, &large_mesh).expect("Failed to create large mesh");
    let camera = Camera::default();

    // Warm-up runs
    for _ in 0..2 {
        let meshes = vec![&mesh_large];
        render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render warmup");
        let _ = target.pick(&ctx, width / 2, height / 2);
    }

    // Performance test: 20 runs
    let mut times = Vec::new();
    for _run in 0..20 {
        let start = std::time::Instant::now();
        let meshes = vec![&mesh_large];
        render_ids(&ctx, &target, &camera, &meshes).expect("Failed to render");
        let picked = target.pick(&ctx, width / 2, height / 2).expect("Failed to pick");
        assert_eq!(picked, Some(1000), "centre pixel must hit the 1M-triangle mesh");
        times.push(start.elapsed().as_secs_f32() * 1000.0);
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    let max = times.last().copied().unwrap_or(0.0);
    let min = times.first().copied().unwrap_or(0.0);

    println!("\n=== VW-06 Performance Test Results ===");
    println!("Triangles: {}", triangle_count);
    println!("Min:    {:.2} ms", min);
    println!("Median: {:.2} ms", median);
    println!("Max:    {:.2} ms", max);
    println!("Adapter: {}", adapter_info.name);

    if gpu_required {
        assert!(median < 16.0,
            "VW-06 FAILED: Median {:.2} ms >= 16 ms (MKCAD_GPU_REQUIRED=1)",
            median
        );
        println!("VW-06 PASSED: Median {:.2} ms < 16 ms", median);
    } else {
        if median < 16.0 {
            println!("VW-06 PASSED: Median {:.2} ms < 16 ms (not required)", median);
        } else {
            println!("VW-06 Note: Median {:.2} ms >= 16 ms (not required without MKCAD_GPU_REQUIRED=1)", median);
        }
    }
}
