//! STEP AP242 round-trip spike: schema header, units, geometry equality after re-import, idempotent re-export.

use kernel::Solid;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("mkcad_step_{name}.step"))
}

/// Box with a groove, a filleted edge and a shell: planar, cylindrical and offset faces.
fn part() -> Solid {
    let (b, _) = Solid::extrude(&[[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]], 25.0).unwrap();
    let tool = Solid::make_box(10.0, 42.0, 10.0).unwrap().translated(25.0, -1.0, 18.0).unwrap();
    let (g, _) = b.cut(&tool).unwrap();
    let e = g.find_edge_at([12.5, 0.0, 25.0], 1e-6).unwrap();
    let (f, _) = g.fillet(&[e], 2.0).unwrap();
    let top = f.find_face_at([47.5, 20.0, 25.0], 1e-6).unwrap();
    f.shell(&[top], 1.5).unwrap().0
}

fn sorted_areas(s: &Solid) -> Vec<f64> {
    let mut a: Vec<f64> = (1..=s.face_count()).map(|i| s.face(i).unwrap().area).collect();
    a.sort_by(f64::total_cmp);
    a
}

fn data_section(path: &PathBuf) -> String {
    let t = std::fs::read_to_string(path).unwrap();
    t[t.find("DATA;").unwrap()..].to_string()
}

#[test]
fn ap242_round_trip_preserves_geometry() {
    let p = part();
    let path = tmp("ap242");
    p.export_step_schema(&path, "AP242DIS").unwrap();
    let header = std::fs::read_to_string(&path).unwrap();
    assert!(header.contains("AP242") || header.contains("ap242"), "schema not AP242:\n{}", &header[..400.min(header.len())]);
    assert!(header.contains("SI_UNIT(.MILLI.,.METRE.)"), "length unit must be mm");

    let back = Solid::import_step(&path).unwrap();
    assert!(back.is_valid());
    assert_eq!(back.face_count(), p.face_count());
    assert_eq!(back.edge_count(), p.edge_count());
    let (v0, v1) = (p.volume().unwrap(), back.volume().unwrap());
    assert!((v0 - v1).abs() <= 1e-9 * v0, "{v0} vs {v1}");
    for (a, b) in sorted_areas(&p).iter().zip(sorted_areas(&back)) {
        assert!((a - b).abs() <= 1e-9 * a.max(1.0), "face area {a} vs {b}");
    }
}

#[test]
fn re_export_is_idempotent_in_data_section() {
    let p = part();
    let (a, b) = (tmp("idem_a"), tmp("idem_b"));
    p.export_step_schema(&a, "AP242DIS").unwrap();
    let back = Solid::import_step(&a).unwrap();
    back.export_step_schema(&b, "AP242DIS").unwrap();
    let again = Solid::import_step(&b).unwrap();
    assert_eq!(again.face_count(), back.face_count());
    let (v1, v2) = (back.volume().unwrap(), again.volume().unwrap());
    assert!((v1 - v2).abs() <= 1e-12 * v1, "{v1} vs {v2}");
    // Same exporter twice on the same shape: data section is byte-identical (header has a timestamp).
    let c = tmp("idem_c");
    p.export_step_schema(&c, "AP242DIS").unwrap();
    assert_eq!(data_section(&a), data_section(&c));
}

#[test]
fn schemas_are_selectable_and_unknown_is_an_error() {
    let p = part();
    for (schema, needle) in [("AP203", "CONFIG_CONTROL_DESIGN"), ("AP214IS", "AUTOMOTIVE_DESIGN")] {
        let path = tmp(schema);
        p.export_step_schema(&path, schema).unwrap();
        let t = std::fs::read_to_string(&path).unwrap();
        assert!(t.contains(needle), "{schema}: no {needle}");
        assert!((Solid::import_step(&path).unwrap().volume().unwrap() - p.volume().unwrap()).abs() < 1e-6);
    }
    assert!(p.export_step_schema(&tmp("bad"), "AP999").is_err());
}
