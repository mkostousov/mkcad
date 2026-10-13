//! Smoke tests for the OCCT bridge. Run with the OCCT `bin` dir on PATH (dynamic DLLs).

use kernel::{HistoryEntry, Model, Relation, Solid, SubKind};
use std::path::Path;

const TOL: f64 = 1e-6;

fn square(side: f64) -> [[f64; 2]; 4] {
    [[0.0, 0.0], [side, 0.0], [side, side], [0.0, side]]
}

/// Result faces that `src` (a face or edge of operand `arg`) maps to, by relation.
fn faces_of(h: &[HistoryEntry], arg: u8, kind: SubKind, src: u32, relation: Relation) -> Vec<u32> {
    h.iter()
        .filter(|e| e.arg == arg && e.kind == kind && e.src == src && e.relation == relation)
        .filter_map(|e| e.dst.filter(|(k, _)| *k == SubKind::Face).map(|(_, i)| i))
        .collect()
}

#[test]
fn box_has_8000_mm3() {
    let b = Solid::make_box(20.0, 20.0, 20.0).unwrap();
    assert!((b.volume().unwrap() - 8000.0).abs() < TOL);
    assert_eq!((b.face_count(), b.edge_count()), (6, 12));
    assert!(b.is_valid());
}

#[test]
fn extruded_square_has_8000_mm3() {
    let (s, history) = Solid::extrude(&square(20.0), 20.0).unwrap();
    assert!((s.volume().unwrap() - 8000.0).abs() < TOL);
    assert_eq!(s.face_count(), 6);
    assert!(s.is_valid());
    // Each of the 4 profile edges generates exactly one side face, all distinct.
    let mut sides: Vec<u32> =
        (1..=4).flat_map(|e| faces_of(&history, 0, SubKind::Edge, e, Relation::Generated)).collect();
    assert_eq!(sides.len(), 4);
    sides.sort();
    sides.dedup();
    assert_eq!(sides.len(), 4);
    // Every side face is 20 x 20.
    for f in sides {
        assert!((s.face(f).unwrap().area - 400.0).abs() < TOL);
    }
}

#[test]
fn fillet_one_edge_removes_expected_volume() {
    let b = Solid::make_box(20.0, 20.0, 20.0).unwrap();
    // Vertical edge at x=20,y=20: centroid (20, 20, 10).
    let e = b.find_edge_at([20.0, 20.0, 10.0], 1e-6).expect("edge");
    let (f, history) = b.fillet(&[e], 2.0).unwrap();
    let expected = 8000.0 - (1.0 - std::f64::consts::FRAC_PI_4) * 2.0 * 2.0 * 20.0;
    assert!((f.volume().unwrap() - expected).abs() < 1e-6, "got {}", f.volume().unwrap());
    assert_eq!(f.face_count(), 7);
    assert!(f.is_valid());

    // The filleted edge is deleted and generates the cylindrical face (area = pi/2 * r * L).
    assert!(history.iter().any(|h| h.kind == SubKind::Edge && h.src == e && h.relation == Relation::Deleted));
    let generated = faces_of(&history, 0, SubKind::Edge, e, Relation::Generated);
    assert_eq!(generated.len(), 1);
    let area = f.face(generated[0]).unwrap().area;
    assert!((area - std::f64::consts::FRAC_PI_2 * 2.0 * 20.0).abs() < 1e-6, "fillet face area {area}");

    // The two faces adjacent to the edge are modified; the other four box faces are tracked too.
    let tracked = (1..=6)
        .filter(|&i| history.iter().any(|h| h.kind == SubKind::Face && h.src == i && h.dst.is_some()))
        .count();
    assert_eq!(tracked, 6);
    let modified = (1..=6).filter(|&i| !faces_of(&history, 0, SubKind::Face, i, Relation::Modified).is_empty()).count();
    assert_eq!(modified, 4, "4 faces change; the 2 not touching the edge are unchanged");
}

#[test]
fn boolean_cut_through_hole() {
    let b = Solid::make_box(20.0, 20.0, 20.0).unwrap();
    let tool = Solid::make_box(10.0, 10.0, 30.0).unwrap().translated(5.0, 5.0, -5.0).unwrap();
    let (c, history) = b.cut(&tool).unwrap();
    assert!((c.volume().unwrap() - 6000.0).abs() < 1e-6);
    assert_eq!(c.face_count(), 10);
    assert!(c.is_valid());

    // Top face (z=20) of the box is modified into a face with area 400 - 100.
    let top = b.find_face_at([10.0, 10.0, 20.0], 1e-6).unwrap();
    let dst = faces_of(&history, 0, SubKind::Face, top, Relation::Modified);
    assert_eq!(dst.len(), 1);
    assert!((c.face(dst[0]).unwrap().area - 300.0).abs() < 1e-6);

    // All six box faces survive (the bottom and top are modified, the sides unchanged).
    for i in 1..=6 {
        assert!(history.iter().any(|h| h.arg == 0 && h.kind == SubKind::Face && h.src == i && h.dst.is_some()));
    }
    // The tool's four side faces become the hole walls (modified into result faces); its caps are removed.
    let walls = (1..=6).filter(|&i| !faces_of(&history, 1, SubKind::Face, i, Relation::Modified).is_empty()).count();
    assert_eq!(walls, 4);
}

#[test]
fn step_round_trip_keeps_volume() {
    let b = Solid::make_box(20.0, 20.0, 20.0).unwrap();
    let path = std::env::temp_dir().join("mkcad_k2_box.step");
    b.export_step(&path).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 100);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("ISO-10303-21") && text.contains("MANIFOLD_SOLID_BREP"));
    let back = Solid::import_step(&path).unwrap();
    assert!((back.volume().unwrap() - 8000.0).abs() < 1e-6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn model_records_history() {
    let mut m = Model::new();
    assert!(m.fillet(&[1], 1.0).is_err(), "no solid yet");
    let r = m.add_box(20.0, 20.0, 20.0).unwrap();
    assert!((r.volume - 8000.0).abs() < TOL);
    let e = m.solid().unwrap().find_edge_at([20.0, 20.0, 10.0], 1e-6).unwrap();
    m.fillet(&[e], 2.0).unwrap();
    let tool = Solid::make_box(4.0, 4.0, 30.0).unwrap().translated(8.0, 8.0, -5.0).unwrap();
    m.cut(&tool).unwrap();

    let names: Vec<_> = m.steps().iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["box", "fillet", "cut"]);
    assert!(m.steps()[1].volume < m.steps()[0].volume);
    assert!(m.steps()[2].volume < m.steps()[1].volume);
    assert!(!m.steps()[1].history.is_empty() && !m.steps()[2].history.is_empty());
    assert!(m.solid().unwrap().is_valid());
}

#[test]
fn invalid_input_is_an_error_not_a_crash() {
    assert!(Solid::make_box(0.0, 1.0, 1.0).is_err());
    assert!(Solid::extrude(&[[0.0, 0.0], [1.0, 0.0]], 1.0).is_err());
    let b = Solid::make_box(10.0, 10.0, 10.0).unwrap();
    assert!(b.fillet(&[999], 1.0).is_err());
    assert!(b.fillet(&[1], 50.0).is_err(), "radius larger than the box");
    assert!(Solid::import_step(Path::new("Z:/definitely/missing.step")).is_err());
}
