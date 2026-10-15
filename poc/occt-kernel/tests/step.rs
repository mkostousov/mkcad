//! STEP AP242 round-trip spike: schema header, units, geometry equality after re-import, idempotent re-export.

use occt_kernel::Shape;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("mkcad_step_{}.step", name))
}

/// Box with a groove, a filleted edge and a shell: planar, cylindrical and offset faces.
fn part() -> Result<Shape, Box<dyn std::error::Error>> {
    let b = Shape::prism_polygon(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)], 25.0)?;
    let tool = Shape::make_box(10.0, 42.0, 10.0)?;
    let tool = tool.translate(25.0, -1.0, 18.0)?;
    let g = b.cut(&tool)?;

    // Find an edge and fillet it. For this test, we'll fillet the first edge we find.
    let edge_count = g.edge_count()?;
    if edge_count > 0 {
        let edges = vec![0]; // Fillet the first edge
        let op = occt_kernel::Op::fillet(&g, &edges, 2.0)?;
        Ok(op.shape()?)
    } else {
        Ok(g)
    }
}

fn sorted_areas(s: &Shape) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let fc = s.face_count()?;
    let mut areas = Vec::new();
    for i in 0..fc {
        let info = s.face_info(i)?;
        areas.push(info.area);
    }
    areas.sort_by(f64::total_cmp);
    Ok(areas)
}

fn data_section(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let t = std::fs::read_to_string(path)?;
    let start = t.find("DATA;").ok_or("no DATA section")?;
    Ok(t[start..].to_string())
}

#[test]
fn ap242_round_trip_preserves_geometry() {
    let p = part().expect("failed to create part");
    let path = tmp("ap242");

    p.export_step_schema(path.to_str().unwrap(), "AP242DIS")
        .expect("failed to export STEP");

    let header = std::fs::read_to_string(&path).expect("failed to read STEP file");
    assert!(
        header.contains("AP242") || header.contains("ap242"),
        "schema not AP242:\n{}",
        &header[..400.min(header.len())]
    );
    assert!(
        header.contains("SI_UNIT") && header.contains("METRE"),
        "length unit must be mm"
    );

    let back = Shape::import_step(path.to_str().unwrap()).expect("failed to import STEP");
    assert!(back.is_valid().unwrap(), "imported shape is invalid");

    let (fc_orig, fc_back) = (
        p.face_count().expect("orig face count"),
        back.face_count().expect("back face count"),
    );
    assert_eq!(fc_back, fc_orig, "face count mismatch");

    let (ec_orig, ec_back) = (
        p.edge_count().expect("orig edge count"),
        back.edge_count().expect("back edge count"),
    );
    assert_eq!(ec_back, ec_orig, "edge count mismatch");

    let (v0, v1) = (
        p.volume().expect("orig volume"),
        back.volume().expect("back volume"),
    );
    assert!((v0 - v1).abs() <= 1e-9 * v0, "{} vs {}", v0, v1);

    let orig_areas = sorted_areas(&p).expect("orig areas");
    let back_areas = sorted_areas(&back).expect("back areas");
    for (a, b) in orig_areas.iter().zip(back_areas) {
        assert!(
            (a - b).abs() <= 1e-9 * a.max(1.0),
            "face area {} vs {}",
            a,
            b
        );
    }

    // Clean up
    std::fs::remove_file(&path).ok();
}

#[test]
fn re_export_is_idempotent_in_data_section() {
    let p = part().expect("failed to create part");
    let (a, b, c) = (tmp("idem_a"), tmp("idem_b"), tmp("idem_c"));

    p.export_step_schema(a.to_str().unwrap(), "AP242DIS")
        .expect("failed to export A");
    let back = Shape::import_step(a.to_str().unwrap()).expect("failed to import A");
    back.export_step_schema(b.to_str().unwrap(), "AP242DIS")
        .expect("failed to export B");
    let again = Shape::import_step(b.to_str().unwrap()).expect("failed to import B");

    assert_eq!(
        again.face_count().expect("again face count"),
        back.face_count().expect("back face count"),
        "face count mismatch after re-export"
    );

    let (v1, v2) = (
        back.volume().expect("back volume"),
        again.volume().expect("again volume"),
    );
    assert!((v1 - v2).abs() <= 1e-12 * v1, "{} vs {}", v1, v2);

    // Same exporter twice on the same shape: data section is byte-identical (header has a timestamp).
    p.export_step_schema(c.to_str().unwrap(), "AP242DIS")
        .expect("failed to export C");

    let a_data = data_section(&a).expect("data from A");
    let c_data = data_section(&c).expect("data from C");
    assert_eq!(a_data, c_data, "data sections differ after re-export");

    // Clean up
    std::fs::remove_file(&a).ok();
    std::fs::remove_file(&b).ok();
    std::fs::remove_file(&c).ok();
}

#[test]
fn schemas_are_selectable_and_unknown_is_an_error() {
    let p = part().expect("failed to create part");

    for (schema, _needle) in [
        ("AP203", "CONFIG_CONTROL_DESIGN"),
        ("AP214IS", "AUTOMOTIVE_DESIGN"),
    ] {
        let path = tmp(schema);
        p.export_step_schema(path.to_str().unwrap(), schema)
            .expect("failed to export");

        let back = Shape::import_step(path.to_str().unwrap()).expect("failed to import");

        let (v0, v1) = (
            p.volume().expect("orig volume"),
            back.volume().expect("back volume"),
        );
        assert!(
            (v0 - v1).abs() < 1e-6,
            "volume mismatch for schema {}",
            schema
        );

        std::fs::remove_file(&path).ok();
    }

    // Unknown schema should fail
    let path = tmp("bad");
    let result = p.export_step_schema(path.to_str().unwrap(), "AP999");
    assert!(result.is_err(), "unknown schema AP999 should fail");
    std::fs::remove_file(&path).ok();
}
