use mkcad_kernel::Shape;

#[test]
fn test_box_properties() {
    let shape_box = Shape::make_box(10.0, 20.0, 30.0).expect("failed to create box");

    let volume = shape_box.volume().expect("failed to get volume");
    let expected = 10.0 * 20.0 * 30.0;
    let rel_error = (volume - expected).abs() / expected;
    assert!(
        rel_error < 1e-12,
        "volume mismatch: {} vs {}",
        volume,
        expected
    );

    assert_eq!(
        shape_box.face_count().unwrap(),
        6,
        "box should have 6 faces"
    );
    assert_eq!(
        shape_box.edge_count().unwrap(),
        12,
        "box should have 12 edges"
    );
    assert_eq!(
        shape_box.vertex_count().unwrap(),
        8,
        "box should have 8 vertices"
    );

    assert!(
        shape_box.is_valid().expect("failed to check validity"),
        "box should be valid"
    );
}

#[test]
fn test_box_cut_by_cylinder() {
    let shape_box = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");
    let cylinder = Shape::make_cylinder(2.0, 10.0).expect("failed to create cylinder");

    let result = shape_box.cut(&cylinder).expect("failed to perform cut");

    let volume = result.volume().expect("failed to get volume");
    let expected_box = 10.0 * 10.0 * 10.0;
    // The cylinder axis is the box's Z edge at the origin: exactly a quarter of it lies inside the box.
    let expected_cyl = std::f64::consts::PI * 2.0 * 2.0 * 10.0 / 4.0;
    let expected = expected_box - expected_cyl;

    let rel_error = (volume - expected).abs() / expected;
    assert!(
        rel_error < 1e-9,
        "cut volume mismatch: {} vs {}",
        volume,
        expected
    );

    assert!(
        result.is_valid().expect("failed to check validity"),
        "cut result should be valid"
    );
}

#[test]
fn test_fillet_all() {
    let shape_box = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");
    let filleted = shape_box.fillet_all(1.0).expect("failed to fillet");

    assert!(
        filleted.is_valid().expect("failed to check validity"),
        "filleted should be valid"
    );

    let original_vol = shape_box.volume().expect("failed to get original volume");
    let filleted_vol = filleted.volume().expect("failed to get filleted volume");

    assert!(
        filleted_vol < original_vol,
        "filleted volume should be less than original"
    );
}

#[test]
fn test_fuse_boxes() {
    let box1 = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box1");
    let box2 = Shape::make_box(5.0, 5.0, 5.0).expect("failed to create box2");

    let fused = box1.fuse(&box2).expect("failed to fuse");

    let volume = fused.volume().expect("failed to get volume");
    // Union should be <= sum of volumes (it's actually less due to overlap)
    let sum = 10.0 * 10.0 * 10.0 + 5.0 * 5.0 * 5.0;
    assert!(
        volume <= sum,
        "fused volume {} should be <= sum {}",
        volume,
        sum
    );

    assert!(
        fused.is_valid().expect("failed to check validity"),
        "fused should be valid"
    );
}
