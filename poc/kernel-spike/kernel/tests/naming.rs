//! K-2: replay of the reference cases REF-01 … REF-08 (M.1) on the OCCT history.

use kernel::naming::*;
use std::f64::consts::FRAC_PI_4;

fn profile() -> Vec<ProfilePoint> {
    [("c:0", [0.0, 0.0]), ("c:1", [20.0, 0.0]), ("c:2", [20.0, 20.0]), ("c:3", [0.0, 20.0])]
        .into_iter()
        .map(|(c, xy)| ProfilePoint { curve: c.into(), xy })
        .collect()
}

fn extrude() -> Feature {
    Feature::new("f1", FeatureKind::Extrude { profile: profile(), height: 20.0 })
}

/// Groove across the whole top: x 8..12, depth 5. Splits the top face and the top edges.
fn groove() -> Feature {
    Feature::new("g", FeatureKind::CutBox { min: [8.0, -1.0, 15.0], size: [4.0, 22.0, 10.0] })
}

const TOP_EDGE_Y0: [f64; 3] = [10.0, 0.0, 20.0];
const TOP_FACE: [f64; 3] = [10.0, 10.0, 20.0];

fn vol(rb: &Rebuild) -> f64 {
    rb.body.as_ref().unwrap().solid.volume().unwrap()
}

fn valid(rb: &Rebuild) -> bool {
    rb.body.as_ref().unwrap().solid.is_valid()
}

fn fillet_loss(r: f64, len: f64) -> f64 {
    (1.0 - FRAC_PI_4) * r * r * len
}

fn edge_ref(tl: &mut Timeline, p: [f64; 3]) -> RefDesc {
    tl.rebuild().body.unwrap().edge_ref_at(p).expect("edge at point")
}

fn face_ref(tl: &mut Timeline, p: [f64; 3]) -> RefDesc {
    tl.rebuild().body.unwrap().face_ref_at(p).expect("face at point")
}

#[test]
fn ref_01_unrelated_upstream_insert_is_exact() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, TOP_EDGE_Y0);
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 1.0 }));
    let before = tl.rebuild();
    assert_eq!(before.report("fil").resolutions, [Resolution::Exact]);

    // A through hole in the middle, inserted between extrude and fillet.
    tl.features.insert(1, Feature::new("hole", FeatureKind::CutBox { min: [8.0, 8.0, -1.0], size: [4.0, 4.0, 22.0] }));
    let after = tl.rebuild();
    let r = after.report("fil");
    assert_eq!((r.health, r.resolutions.as_slice()), (Health::Healthy, &[Resolution::Exact][..]));
    assert!((vol(&after) - (8000.0 - fillet_loss(1.0, 20.0) - 320.0)).abs() < 1e-6);
    assert!(valid(&after));
}

#[test]
fn ref_02_edge_split_expands_to_both_pieces() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, TOP_EDGE_Y0);
    tl.features.push(groove());
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 1.0 }));
    let rb = tl.rebuild();
    let r = rb.report("fil");
    assert_eq!(r.resolutions, [Resolution::Expanded(2)]);
    assert_eq!(r.health, Health::Healthy);
    // Both 8 mm pieces are filleted.
    assert!((vol(&rb) - (8000.0 - 4.0 * 20.0 * 5.0 - fillet_loss(1.0, 16.0))).abs() < 1e-6, "vol {}", vol(&rb));
    assert!(valid(&rb));
}

#[test]
fn ref_03_edge_removed_is_lost_then_recovers() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, [20.0, 20.0, 10.0]);
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 2.0 }));
    // Cut away the whole corner: the vertical edge no longer exists.
    tl.features.insert(
        1,
        Feature::new("corner", FeatureKind::CutBox { min: [15.0, 15.0, -1.0], size: [10.0, 10.0, 22.0] }),
    );
    let lost = tl.rebuild();
    let r = lost.report("fil");
    assert_eq!(r.resolutions, [Resolution::Lost]);
    assert_eq!(r.health, Health::Error);
    assert_eq!(r.diagnostics, ["REF.LOST"]);
    // Cached geometry: the body is the one before the fillet.
    assert!((vol(&lost) - (8000.0 - 5.0 * 5.0 * 20.0)).abs() < 1e-6);

    // Cause removed: automatic recovery with the original geometry (F44).
    tl.features[1].suppressed = true;
    let ok = tl.rebuild();
    let r = ok.report("fil");
    assert_eq!((r.health, r.resolutions.as_slice()), (Health::Healthy, &[Resolution::Exact][..]));
    assert!((vol(&ok) - (8000.0 - fillet_loss(2.0, 20.0))).abs() < 1e-6);
}

#[test]
fn ref_04_sketch_on_face_follows_resize() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let f = face_ref(&mut tl, TOP_FACE);
    tl.features.push(Feature::new("sk", FeatureKind::Sketch { face: f, policy: PiecePolicy::Single }));
    let a = tl.rebuild();
    assert_eq!(a.report("sk").plane.unwrap().origin[2], 20.0);

    if let FeatureKind::Extrude { height, .. } = &mut tl.features[0].kind {
        *height = 30.0;
    }
    let b = tl.rebuild();
    let r = b.report("sk");
    assert_eq!((r.health, r.resolutions.as_slice()), (Health::Healthy, &[Resolution::Exact][..]));
    let p = r.plane.unwrap();
    assert!((p.origin[2] - 30.0).abs() < 1e-9 && (p.normal[2] - 1.0).abs() < 1e-9);
}

#[test]
fn ref_05_face_deleted_keeps_cached_frame() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let f = face_ref(&mut tl, TOP_FACE);
    tl.features.push(Feature::new("sk", FeatureKind::Sketch { face: f, policy: PiecePolicy::Single }));
    tl.rebuild();
    // A slab cut removes the top face (the new top is at z = 15).
    tl.features
        .insert(1, Feature::new("slab", FeatureKind::CutBox { min: [-1.0, -1.0, 15.0], size: [22.0, 22.0, 10.0] }));
    let rb = tl.rebuild();
    let r = rb.report("sk");
    assert_eq!(r.resolutions, [Resolution::Lost]);
    assert_eq!(r.health, Health::Warning);
    assert_eq!(r.diagnostics, ["SKETCH.REF.LOST"]);
    assert_eq!(r.plane.unwrap().origin[2], 20.0, "cached frame from the last good rebuild");
}

#[test]
fn ref_06_split_support_is_ambiguous_or_expanded_never_silent() {
    let cases: [(PiecePolicy, Resolution, Health, &[&str]); 2] = [
        (PiecePolicy::Single, Resolution::Ambiguous, Health::Warning, &["REF.AMBIGUOUS"]),
        (PiecePolicy::AllPieces, Resolution::Expanded(2), Health::Healthy, &[]),
    ];
    for (policy, expect, health, diag) in cases {
        let mut tl = Timeline::new();
        tl.features.push(extrude());
        let f = face_ref(&mut tl, TOP_FACE);
        tl.features.push(Feature::new("sk", FeatureKind::Sketch { face: f, policy }));
        tl.rebuild(); // plane cached before the split
        tl.features.insert(1, groove());
        let rb = tl.rebuild();
        let r = rb.report("sk");
        assert_eq!(r.resolutions, [expect], "{policy:?}");
        assert_eq!(r.health, health);
        assert_eq!(r.diagnostics, diag);
        assert!((r.plane.unwrap().origin[2] - 20.0).abs() < 1e-9);
    }
}

#[test]
fn ref_07_shell_face_split_upstream_removes_both_pieces() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let f = face_ref(&mut tl, TOP_FACE);
    tl.features.push(groove());
    tl.features.push(Feature::new("sh", FeatureKind::Shell { faces: vec![f], thickness: 1.0 }));
    let rb = tl.rebuild();
    let r = rb.report("sh");
    assert_eq!(r.resolutions, [Resolution::Expanded(2)]);
    assert_eq!(r.health, Health::Healthy, "{:?}", r.diagnostics);
    let body = rb.body.as_ref().unwrap();
    assert!(body.solid.is_valid());
    assert!(body.solid.volume().unwrap() < 7600.0);
    // Both removed pieces are gone as faces (8 x 20 = 160 mm2 each); only the 1 mm thick rims remain in the
    // plane of the opening: 160 - 6 x 18 = 52 mm2 each.
    let at_top: Vec<f64> = (1..=body.solid.face_count())
        .filter_map(|f| body.solid.face(f).ok())
        .filter(|i| (i.center[2] - 20.0).abs() < 1e-6)
        .map(|i| i.area)
        .collect();
    assert_eq!(at_top.len(), 2);
    assert!(at_top.iter().all(|a| (a - 52.0).abs() < 1e-6), "{at_top:?}");
}

#[test]
fn ref_08_split_edge_remerged_set_shrinks_back() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, TOP_EDGE_Y0);
    tl.features.push(groove());
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 1.0 }));
    assert_eq!(tl.rebuild().report("fil").resolutions, [Resolution::Expanded(2)]);
    tl.features[1].suppressed = true;
    let rb = tl.rebuild();
    assert_eq!(rb.report("fil").resolutions, [Resolution::Exact]);
    assert!((vol(&rb) - (8000.0 - fillet_loss(1.0, 20.0))).abs() < 1e-6);
}

#[test]
fn signature_fallback_when_labels_change_but_geometry_does_not() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, TOP_EDGE_Y0);
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 1.0 }));
    // The profile is redrawn with new curve ids: every side label changes, the geometry is identical.
    if let FeatureKind::Extrude { profile, .. } = &mut tl.features[0].kind {
        for (i, p) in profile.iter_mut().enumerate() {
            p.curve = format!("new:{i}");
        }
    }
    let rb = tl.rebuild();
    let r = rb.report("fil");
    assert_eq!(r.resolutions, [Resolution::Signature]);
    assert!((vol(&rb) - (8000.0 - fillet_loss(1.0, 20.0))).abs() < 1e-6);
}

#[test]
fn rebuild_is_repeatable() {
    let mut tl = Timeline::new();
    tl.features.push(extrude());
    let e = edge_ref(&mut tl, TOP_EDGE_Y0);
    tl.features.push(groove());
    tl.features.push(Feature::new("fil", FeatureKind::Fillet { edges: vec![e], radius: 1.0 }));
    let a = tl.rebuild();
    let b = tl.rebuild();
    assert_eq!(a.reports, b.reports);
    assert_eq!(vol(&a), vol(&b));
    assert_eq!(a.body.as_ref().unwrap().face_labels, b.body.as_ref().unwrap().face_labels);
}
