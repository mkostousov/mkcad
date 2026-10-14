use occt_kernel::{Kind, Op, Shape};
use std::f64::consts::PI;

#[test]
fn test_cut_box_by_through_cylinder() {
    // Create a 10x10x10 box at origin
    let box_shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

    // Create a cylinder at (5, 5, -1) with r=2, h=12 (goes through the box)
    let cyl = Shape::cylinder_at(5.0, 5.0, -1.0, 2.0, 12.0).expect("failed to create cylinder");

    // Perform cut with history
    let op = Op::cut(&box_shape, &cyl).expect("failed to cut");
    let result = op.shape().expect("failed to get result shape");

    // Check face count: should be 7 (6 box faces + hole wall)
    let face_count = result.count(Kind::Face).expect("failed to count faces");
    assert_eq!(
        face_count, 7,
        "should have 7 faces (6 from box + 1 hole wall)"
    );

    // Check volume: box is 1000, cylinder volume through box is PI*r^2*h = PI*4*10 = 40*PI
    let result_volume = result.volume().expect("failed to get volume");
    let expected = 10.0 * 10.0 * 10.0 - PI * 2.0 * 2.0 * 10.0;
    let rel_error = (result_volume - expected).abs() / expected;
    assert!(
        rel_error < 1e-9,
        "volume error: {} vs {}, rel: {}",
        result_volume,
        expected,
        rel_error
    );

    // Check history: all 6 box faces should map to exactly one result face each
    for face_idx in 0..6 {
        let modified = op
            .modified(0, Kind::Face, face_idx)
            .expect("failed to get modified");
        assert!(
            modified.len() == 1,
            "face {} should map to exactly 1 result face, got {}",
            face_idx,
            modified.len()
        );
    }

    // History identifies the top face: the box's +Z face (index 5) maps to a +Z face of area 100 - 4π.
    let top = op.modified(0, Kind::Face, 5).expect("modified");
    let top_info = result.face_info(top[0]).expect("face info");
    assert!(
        (top_info.area - (100.0 - 4.0 * PI)).abs() / (100.0 - 4.0 * PI) < 1e-9,
        "top area {}",
        top_info.area
    );
    assert!((top_info.normal[2] - 1.0).abs() < 1e-12 && (top_info.centroid[2] - 10.0).abs() < 1e-9);

    // Check that one of the faces has area 100 - 4*PI (top face with hole)
    let expected_top_area = 100.0 - PI * 2.0 * 2.0;
    let mut found_top_face = false;
    for face_idx in 0..face_count {
        if let Ok(info) = result.face_info(face_idx) {
            let area_rel_error = (info.area - expected_top_area).abs() / expected_top_area;
            if area_rel_error < 1e-8 {
                found_top_face = true;
                // Check that this face normal points up (+Z)
                assert!(
                    info.normal[2] > 0.9,
                    "top face normal should point +Z, got {:?}",
                    info.normal
                );
                break;
            }
        }
    }
    assert!(
        found_top_face,
        "top face with area {} not found",
        expected_top_area
    );
}

#[test]
fn test_cut_box_split_face() {
    // Create a 10x10x10 box
    let box1 = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box1");

    // Create a slot box: 10x2x10 positioned at (0, 4, 0)
    // This will split the +Y face of box1 into 2 pieces
    let slot = Shape::make_box(10.0, 2.0, 10.0).expect("failed to create slot");
    let slot_translated = slot
        .translate(0.0, 4.0, 0.0)
        .expect("failed to translate slot");

    // Cut box1 by the slot
    let op = Op::cut(&box1, &slot_translated).expect("failed to cut");
    let result = op.shape().expect("failed to get result shape");

    // The slot spans the full X and Z extent, so it cuts the box into two halves. Face index 1 is the +X
    // face (TopExp::MapShapes order of BRepPrimAPI_MakeBox: -X, +X, -Y, +Y, -Z, +Z); it splits into 2 pieces.
    assert!(result.face_info(1).is_ok());
    let fx = box1.face_info(1).expect("face info");
    assert!(
        (fx.normal[0] - 1.0).abs() < 1e-12,
        "face 1 should be +X, normal {:?}",
        fx.normal
    );
    let modified = op
        .modified(0, Kind::Face, 1)
        .expect("failed to get modified faces");
    assert_eq!(
        modified.len(),
        2,
        "face 1 should be split into 2 pieces, got {}",
        modified.len()
    );

    assert!(
        result.is_valid().expect("failed to check validity"),
        "result should be valid"
    );
}

#[test]
fn test_fillet_one_edge() {
    // Create a 10x10x10 box
    let box_shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

    // A box has 12 edges, fillet edge 0 with r=1
    let op = Op::fillet(&box_shape, &[0], 1.0).expect("failed to fillet");
    let result = op.shape().expect("failed to get result shape");

    assert!(
        result.is_valid().expect("failed to check validity"),
        "filleted result should be valid"
    );

    // The box's 6 faces should be modified (2 adjacent to the filleted edge are modified)
    // and 1 new fillet face should be generated
    let face_count = result.count(Kind::Face).expect("failed to count faces");
    assert!(
        face_count > 6,
        "filleted box should have more than 6 faces, got {}",
        face_count
    );

    // History: the filleted edge generates exactly one new (fillet) face, and that face is not planar-normal
    // to any box axis.
    let generated = op
        .generated(0, Kind::Edge, 0)
        .expect("failed to get generated")
        .into_iter()
        .filter(|(k, _)| *k == Kind::Face)
        .map(|(_, i)| i)
        .collect::<Vec<_>>();
    assert_eq!(
        generated.len(),
        1,
        "filleted edge should generate exactly one face, got {generated:?}"
    );
    let fillet_area = result.face_info(generated[0]).expect("face info").area;
    let expected_fillet_area = std::f64::consts::PI / 2.0 * 1.0 * 10.0;
    assert!(
        (fillet_area - expected_fillet_area).abs() / expected_fillet_area < 1e-9,
        "fillet area {fillet_area}"
    );

    // Volume should be less than original
    let original_vol = box_shape.volume().expect("failed to get original volume");
    let filleted_vol = result.volume().expect("failed to get filleted volume");
    assert!(
        filleted_vol < original_vol,
        "filleted volume {} should be < original {}",
        filleted_vol,
        original_vol
    );
}

#[test]
fn test_prism_polygon_square() {
    // Define a square polygon: (0,0), (5,0), (5,5), (0,5)
    let points = vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)];

    // Create prism with height 10
    let op = Op::prism_polygon(&points, 10.0).expect("failed to create prism");
    let result = op.shape().expect("failed to get result shape");

    // Check face count: bottom, top, 4 sides = 6 faces
    let face_count = result.count(Kind::Face).expect("failed to count faces");
    assert_eq!(
        face_count, 6,
        "prism of square should have 6 faces, got {}",
        face_count
    );

    // Check volume: 5x5x10 = 250
    let volume = result.volume().expect("failed to get volume");
    let expected = 5.0 * 5.0 * 10.0;
    let rel_error = (volume - expected).abs() / expected;
    assert!(
        rel_error < 1e-9,
        "volume error: {} vs {}, rel: {}",
        volume,
        expected,
        rel_error
    );

    // Check history: each profile edge should generate exactly one side face
    // The profile has 4 edges
    for edge_idx in 0..4 {
        let generated = op
            .generated(0, Kind::Edge, edge_idx)
            .expect("failed to get generated");
        assert_eq!(
            generated.len(),
            1,
            "edge {} should generate 1 face, got {}",
            edge_idx,
            generated.len()
        );
        assert_eq!(
            generated[0].0,
            Kind::Face,
            "generated shape should be a face"
        );
    }
}

#[test]
fn test_translate_preserves_volume() {
    // Create a box
    let box_shape = Shape::make_box(10.0, 20.0, 30.0).expect("failed to create box");
    let original_volume = box_shape.volume().expect("failed to get original volume");

    // Translate it
    let translated = box_shape
        .translate(5.0, 10.0, 15.0)
        .expect("failed to translate");
    let translated_volume = translated
        .volume()
        .expect("failed to get translated volume");

    // Volumes should be identical
    let rel_error = (translated_volume - original_volume).abs() / original_volume;
    assert!(
        rel_error < 1e-12,
        "translation should preserve volume: {} vs {}, rel: {}",
        translated_volume,
        original_volume,
        rel_error
    );

    assert!(
        translated.is_valid().expect("failed to check validity"),
        "translated shape should be valid"
    );
}

#[test]
fn test_cylinder_at_position() {
    // Create a cylinder at (10, 20, 30) with r=3, h=5
    let cyl = Shape::cylinder_at(10.0, 20.0, 30.0, 3.0, 5.0).expect("failed to create cylinder");

    // Check volume: PI*r^2*h = PI*9*5 = 45*PI
    let volume = cyl.volume().expect("failed to get volume");
    let expected = PI * 3.0 * 3.0 * 5.0;
    let rel_error = (volume - expected).abs() / expected;
    assert!(
        rel_error < 1e-9,
        "volume error: {} vs {}, rel: {}",
        volume,
        expected,
        rel_error
    );

    // Should have 3 faces: bottom, top, lateral
    let face_count = cyl.count(Kind::Face).expect("failed to count faces");
    assert_eq!(
        face_count, 3,
        "cylinder should have 3 faces, got {}",
        face_count
    );

    assert!(
        cyl.is_valid().expect("failed to check validity"),
        "cylinder should be valid"
    );
}

#[test]
fn test_fuse_with_history() {
    let box1 = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box1");
    let box2 = Shape::make_box(5.0, 5.0, 5.0).expect("failed to create box2");

    let op = Op::fuse(&box1, &box2).expect("failed to fuse");
    let result = op.shape().expect("failed to get result shape");

    // All faces of box1 should be modified or unchanged
    let box1_faces = box1.count(Kind::Face).expect("failed to count box1 faces");
    for i in 0..box1_faces {
        let modified = op
            .modified(0, Kind::Face, i)
            .expect("failed to get modified");
        // Modified should have at least one entry (either modified into or stayed same)
        assert!(
            !modified.is_empty(),
            "face {} should have modified entry",
            i
        );
    }

    // Volume should be less than sum of two boxes
    let sum_volume = 10.0 * 10.0 * 10.0 + 5.0 * 5.0 * 5.0;
    let result_volume = result.volume().expect("failed to get volume");
    assert!(
        result_volume < sum_volume,
        "fused volume should be < sum: {} < {}",
        result_volume,
        sum_volume
    );

    assert!(
        result.is_valid().expect("failed to check validity"),
        "result should be valid"
    );
}
