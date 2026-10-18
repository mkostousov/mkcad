use mkcad_render::{Camera, GpuContext, OffscreenTarget, render_grid, render_test_scene, save_png};

#[tokio::test]
async fn adapter_is_found() {
    match GpuContext::new_headless().await {
        Ok(ctx) => {
            let info = ctx.adapter_info();
            println!("Adapter name: {}", info.name);
            println!("Adapter backend: {}", info.backend);
            println!("Adapter device type: {}", info.device_type);

            // Check if it's CPU/software when MKCAD_GPU_REQUIRED=1
            let gpu_required = std::env::var("MKCAD_GPU_REQUIRED")
                .map(|v| v == "1")
                .unwrap_or(false);

            if gpu_required {
                assert!(
                    !info.device_type.contains("Cpu") && !info.device_type.contains("Software"),
                    "MKCAD_GPU_REQUIRED=1 but got CPU/software adapter: {}",
                    info.device_type
                );
            }
        }
        Err(e) => {
            let gpu_required = std::env::var("MKCAD_GPU_REQUIRED")
                .map(|v| v == "1")
                .unwrap_or(false);

            if gpu_required {
                panic!("MKCAD_GPU_REQUIRED=1 but no adapter found: {}", e);
            } else {
                eprintln!("No GPU adapter found (test skipped): {}", e);
            }
        }
    }
}

#[tokio::test]
async fn clear_colour_readback() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 64u32;
    let height = 64u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Render a scene (which clears to a specific color)
    render_test_scene(&ctx, &target).expect("Failed to render test scene");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Clear color from render_test_scene: [0.1, 0.2, 0.3, 1.0] in linear float
    // When written to an sRGB texture, wgpu converts from linear to sRGB
    // sRGB(0.1) ≈ 0.33, sRGB(0.2) ≈ 0.49, sRGB(0.3) ≈ 0.58
    // So in RGBA8: [84, 125, 148, 255] (approximately)
    // Let's measure the actual values
    let expected_r = 89u8;  // sRGB encoding of 0.1 linear
    let expected_g = 124u8; // sRGB encoding of 0.2 linear
    let expected_b = 149u8; // sRGB encoding of 0.3 linear
    let expected_a = 255u8;

    // Sample some pixels that should be the clear color (avoid the triangle and grid lines)
    // Grid is at intervals of 0.2 in normalized coords (-1 to 1)
    // Grid lines are at (-1, -0.8, -0.6, ..., 0.8, 1.0)
    // In a 64x64 image: pixels at 0, 12.8, 25.6, etc.
    // Sample at pixel (5, 5) which should be between grid lines
    let sample_x = 5usize;
    let sample_y = 5usize;
    let sample_idx = (sample_y * width as usize + sample_x) * 4;

    let sample_r = pixels[sample_idx];
    let sample_g = pixels[sample_idx + 1];
    let sample_b = pixels[sample_idx + 2];
    let sample_a = pixels[sample_idx + 3];

    println!(
        "Clear color at ({}, {}): R={}, G={}, B={}, A={}",
        sample_x, sample_y, sample_r, sample_g, sample_b, sample_a
    );
    println!(
        "Expected clear color: R={}, G={}, B={}, A={}",
        expected_r, expected_g, expected_b, expected_a
    );

    // Allow for some tolerance due to sRGB conversion
    assert!(
        (sample_r as i32 - expected_r as i32).abs() <= 2,
        "Red channel mismatch: got {}, expected {}",
        sample_r,
        expected_r
    );
    assert!(
        (sample_g as i32 - expected_g as i32).abs() <= 2,
        "Green channel mismatch: got {}, expected {}",
        sample_g,
        expected_g
    );
    assert!(
        (sample_b as i32 - expected_b as i32).abs() <= 2,
        "Blue channel mismatch: got {}, expected {}",
        sample_b,
        expected_b
    );
    assert_eq!(sample_a, expected_a, "Alpha channel mismatch");
}

#[tokio::test]
async fn triangle_pixels() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Render test scene
    render_test_scene(&ctx, &target).expect("Failed to render test scene");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // The triangle is at normalized coords: [-0.5, -0.5], [0.5, -0.5], [0.0, 0.5]
    // In screen space (assuming NDC to screen): center is (width/2, height/2)
    // Expected triangle color: [1.0, 0.0, 0.0, 1.0] = [255, 0, 0, 255]

    let center_x = (width / 2) as usize;
    let center_y = (height / 2) as usize;

    // Sample a pixel in the middle of the triangle (should be red)
    let tri_idx = (center_y * width as usize + center_x) * 4;
    if tri_idx < pixels.len() {
        let tri_r = pixels[tri_idx];
        let tri_g = pixels[tri_idx + 1];
        let tri_b = pixels[tri_idx + 2];
        let tri_a = pixels[tri_idx + 3];

        println!(
            "Triangle pixel at center: R={}, G={}, B={}, A={}",
            tri_r, tri_g, tri_b, tri_a
        );

        // Should be red
        assert!(
            tri_r > 200,
            "Triangle should be red, got R={}",
            tri_r
        );
        assert!(tri_g < 50, "Triangle should be red, got G={}", tri_g);
        assert!(tri_b < 50, "Triangle should be red, got B={}", tri_b);
    }

    // Sample a corner pixel (should be clear color)
    let corner_idx = 0usize;
    let corner_r = pixels[corner_idx * 4];
    let corner_g = pixels[corner_idx * 4 + 1];
    let corner_b = pixels[corner_idx * 4 + 2];
    let corner_a = pixels[corner_idx * 4 + 3];

    println!(
        "Clear color at corner: R={}, G={}, B={}, A={}",
        corner_r, corner_g, corner_b, corner_a
    );

    // Should not be red
    let is_red = corner_r > 200 && corner_g < 50 && corner_b < 50;
    assert!(!is_red, "Corner should not be red (triangle color)");
}

#[tokio::test]
async fn msaa_edge() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 128u32;
    let height = 128u32;

    // Test with MSAA (sample_count = 4)
    let target_msaa = match OffscreenTarget::new(&ctx, width, height, 4) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("MSAA not supported (test skipped): {}", e);
            return;
        }
    };

    render_test_scene(&ctx, &target_msaa).expect("Failed to render test scene with MSAA");
    let pixels_msaa = target_msaa.read_pixels(&ctx).expect("Failed to read MSAA pixels");

    // Test without MSAA
    let target_no_msaa = OffscreenTarget::new(&ctx, width, height, 1)
        .expect("Failed to create target without MSAA");

    render_test_scene(&ctx, &target_no_msaa).expect("Failed to render test scene without MSAA");
    let pixels_no_msaa = target_no_msaa
        .read_pixels(&ctx)
        .expect("Failed to read non-MSAA pixels");

    // Anti-aliased edge pixels have an R value strictly between the clear colour (89) and the
    // triangle (255). Without MSAA the hard edge produces none; with 4x MSAA it must.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    let intermediate = |px: &[u8]| {
        px.chunks_exact(4)
            .filter(|p| p[0] > 100 && p[0] < 245 && p[1] < 130)
            .count()
    };
    let with = intermediate(&pixels_msaa);
    let without = intermediate(&pixels_no_msaa);
    println!("intermediate edge pixels: msaa={with} no_msaa={without}");
    assert!(with > 0, "4x MSAA produced no anti-aliased edge pixels");
    assert_eq!(without, 0, "non-MSAA edge should be hard");
}

#[tokio::test]
async fn png_written() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 128u32;
    let height = 128u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    render_test_scene(&ctx, &target).expect("Failed to render test scene");
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Use CARGO_TARGET_TMPDIR for test output
    let test_output_dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));

    std::fs::create_dir_all(&test_output_dir).expect("Failed to create test output directory");

    let png_path = test_output_dir.join("test_triangle.png");
    save_png(png_path.to_str().unwrap(), width, height, &pixels)
        .expect("Failed to save PNG");

    println!("PNG written to: {}", png_path.display());

    // Verify file exists and has content
    let metadata = std::fs::metadata(&png_path).expect("PNG file not found");
    assert!(
        metadata.len() > 0,
        "PNG file is empty"
    );

    println!("PNG file size: {} bytes", metadata.len());
}

#[tokio::test]
async fn determinism() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 64u32;
    let height = 64u32;

    // First render
    let target1 = OffscreenTarget::new(&ctx, width, height, 1)
        .expect("Failed to create first target");
    render_test_scene(&ctx, &target1).expect("First render failed");
    let pixels1 = target1.read_pixels(&ctx).expect("Failed to read first pixels");

    // Second render with same parameters
    let target2 = OffscreenTarget::new(&ctx, width, height, 1)
        .expect("Failed to create second target");
    render_test_scene(&ctx, &target2).expect("Second render failed");
    let pixels2 = target2.read_pixels(&ctx).expect("Failed to read second pixels");

    // Results should be byte-identical
    assert_eq!(
        pixels1, pixels2,
        "Determinism test failed: pixel data differs between runs"
    );

    println!("Determinism test passed: {} bytes rendered identically", pixels1.len());
}

#[tokio::test]
async fn grid_visual_check() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Create Top orthographic view
    let camera = Camera::orthographic_view("top", glam::Vec3::ZERO, 20.0);

    // Render grid
    render_grid(&ctx, &target, &camera, "top").expect("Failed to render grid");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Save to CARGO_TARGET_TMPDIR for inspection
    let test_output_dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&test_output_dir).expect("Failed to create test output directory");
    let png_path = test_output_dir.join("grid_visual_top.png");
    save_png(png_path.to_str().unwrap(), width, height, &pixels)
        .expect("Failed to save PNG");
    println!("Grid PNG written to: {}", png_path.display());

    // Top ortho, 256 px for 20 world units: world origin at the image centre, X right, Y up.
    let px = |x: usize, y: usize| {
        let i = (y * width as usize + x) * 4;
        (pixels[i] as i32, pixels[i + 1] as i32, pixels[i + 2] as i32)
    };
    let (w, h) = (width as usize, height as usize);

    // X axis: red horizontal line through the centre row, on both sides of the origin.
    for x in [30, 60, 200, 230] {
        let (r, g, b) = px(x, h / 2);
        assert!(r > 150 && g < 80 && b < 80, "X axis at x={x} should be red: {r},{g},{b}");
    }
    // Y axis: green vertical line through the centre column, above and below the origin.
    // rows chosen between horizontal grid lines (lines every 12.8 px around row 128)
    for y in [45, 58, 211, 224] {
        let (r, g, b) = px(w / 2, y);
        assert!(g > 150 && r < 80 && b < 80, "Y axis at y={y} should be green: {r},{g},{b}");
    }
    // Background corner pixel is dark (displays below 40/255 in every channel).
    let (r, g, b) = px(2, 2);
    assert!(r < 40 && g < 40 && b < 40, "background should be dark: {r},{g},{b}");

    // Along a row that crosses no axis, grid lines are neutral grey, brighter than the
    // background, with two distinct brightness levels (minor < major).
    let row = 45usize;
    let mut levels: Vec<i32> = Vec::new();
    for x in (0..w).filter(|x| x.abs_diff(w / 2) > 2) {
        let (r, g, b) = px(x, row);
        if (r + g + b) / 3 > 40 {
            assert!((r - g).abs() < 10 && (g - b).abs() < 10, "grid line not neutral at x={x}: {r},{g},{b}");
            levels.push((r + g + b) / 3);
        }
    }
    assert!(!levels.is_empty(), "no grid lines found on row {row}");
    let (lo, hi) = (*levels.iter().min().unwrap(), *levels.iter().max().unwrap());
    assert!(hi - lo >= 20, "expected distinct minor/major brightness, got {lo}..{hi}");
}

#[tokio::test]
async fn grid_golden_top() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Create Top orthographic view
    let camera = Camera::orthographic_view("top", glam::Vec3::ZERO, 20.0);

    // Render grid
    render_grid(&ctx, &target, &camera, "top").expect("Failed to render grid");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Get golden image path (in repo, not in target dir)
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("top.png");

    let update_golden = std::env::var("MKCAD_UPDATE_GOLDEN")
        .map(|v| v == "1")
        .unwrap_or(false);

    if update_golden {
        // Write golden image
        save_png(golden_path.to_str().unwrap(), width, height, &pixels)
            .expect("Failed to save golden PNG");
        println!("Golden image written to: {}", golden_path.display());
    } else {
        // Compare with golden image
        if !golden_path.exists() {
            panic!("Golden image not found: {}. Run with MKCAD_UPDATE_GOLDEN=1 to create it.", golden_path.display());
        }

        // Read golden image
        let golden_file = std::fs::File::open(&golden_path).expect("Failed to open golden image");
        let reader = std::io::BufReader::new(golden_file);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().expect("Failed to read golden PNG");
        let mut golden_pixels = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut golden_pixels).expect("Failed to decode golden PNG");

        // Compare pixel data with tolerance
        const TOLERANCE: i32 = 2;
        for (i, (actual, expected)) in pixels.iter().zip(golden_pixels.iter()).enumerate() {
            let diff = (*actual as i32 - *expected as i32).abs();
            assert!(
                diff <= TOLERANCE,
                "Pixel {} mismatch: actual={}, expected={}, diff={}",
                i, actual, expected, diff
            );
        }

        println!("Golden image comparison passed with tolerance={}", TOLERANCE);
    }
}

#[tokio::test]
async fn grid_golden_front() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Create Front orthographic view
    let camera = Camera::orthographic_view("front", glam::Vec3::ZERO, 20.0);

    // Render grid
    render_grid(&ctx, &target, &camera, "front").expect("Failed to render grid");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Get golden image path (in repo, not in target dir)
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("front.png");

    let update_golden = std::env::var("MKCAD_UPDATE_GOLDEN")
        .map(|v| v == "1")
        .unwrap_or(false);

    if update_golden {
        // Write golden image
        save_png(golden_path.to_str().unwrap(), width, height, &pixels)
            .expect("Failed to save golden PNG");
        println!("Golden image written to: {}", golden_path.display());
    } else {
        // Compare with golden image
        if !golden_path.exists() {
            panic!("Golden image not found: {}. Run with MKCAD_UPDATE_GOLDEN=1 to create it.", golden_path.display());
        }

        // Read golden image
        let golden_file = std::fs::File::open(&golden_path).expect("Failed to open golden image");
        let reader = std::io::BufReader::new(golden_file);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().expect("Failed to read golden PNG");
        let mut golden_pixels = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut golden_pixels).expect("Failed to decode golden PNG");

        // Compare pixel data with tolerance
        const TOLERANCE: i32 = 2;
        for (i, (actual, expected)) in pixels.iter().zip(golden_pixels.iter()).enumerate() {
            let diff = (*actual as i32 - *expected as i32).abs();
            assert!(
                diff <= TOLERANCE,
                "Pixel {} mismatch: actual={}, expected={}, diff={}",
                i, actual, expected, diff
            );
        }

        println!("Golden image comparison passed with tolerance={}", TOLERANCE);
    }
}

#[tokio::test]
async fn grid_golden_iso() {
    let ctx = match GpuContext::new_headless().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("No GPU adapter found (test skipped)");
            return;
        }
    };

    let width = 256u32;
    let height = 256u32;

    let target = match OffscreenTarget::new(&ctx, width, height, 1) {
        Ok(t) => t,
        Err(e) => panic!("Failed to create offscreen target: {}", e),
    };

    // Create Iso orthographic view
    let camera = Camera::orthographic_view("iso", glam::Vec3::ZERO, 20.0);

    // Render grid
    render_grid(&ctx, &target, &camera, "iso").expect("Failed to render grid");

    // Read pixels
    let pixels = target.read_pixels(&ctx).expect("Failed to read pixels");

    // Get golden image path (in repo, not in target dir)
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("iso.png");

    let update_golden = std::env::var("MKCAD_UPDATE_GOLDEN")
        .map(|v| v == "1")
        .unwrap_or(false);

    if update_golden {
        // Write golden image
        save_png(golden_path.to_str().unwrap(), width, height, &pixels)
            .expect("Failed to save golden PNG");
        println!("Golden image written to: {}", golden_path.display());
    } else {
        // Compare with golden image
        if !golden_path.exists() {
            panic!("Golden image not found: {}. Run with MKCAD_UPDATE_GOLDEN=1 to create it.", golden_path.display());
        }

        // Read golden image
        let golden_file = std::fs::File::open(&golden_path).expect("Failed to open golden image");
        let reader = std::io::BufReader::new(golden_file);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().expect("Failed to read golden PNG");
        let mut golden_pixels = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut golden_pixels).expect("Failed to decode golden PNG");

        // Compare pixel data with tolerance
        const TOLERANCE: i32 = 2;
        for (i, (actual, expected)) in pixels.iter().zip(golden_pixels.iter()).enumerate() {
            let diff = (*actual as i32 - *expected as i32).abs();
            assert!(
                diff <= TOLERANCE,
                "Pixel {} mismatch: actual={}, expected={}, diff={}",
                i, actual, expected, diff
            );
        }

        println!("Golden image comparison passed with tolerance={}", TOLERANCE);
    }
}
