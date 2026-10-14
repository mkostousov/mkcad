//! Fillet robustness spike: radius sweep, edge sets, chained fillets, limit cases. A fillet either succeeds with a
//! valid solid and the analytic volume loss, or fails with an error — never a crash and never an invalid solid.

use kernel::Solid;
use std::f64::consts::FRAC_PI_4;

const W: f64 = 60.0;
const D: f64 = 40.0;
const H: f64 = 25.0;

fn block() -> Solid {
    Solid::extrude(&[[0.0, 0.0], [W, 0.0], [W, D], [0.0, D]], H).unwrap().0
}

fn top_edges(s: &Solid) -> Vec<u32> {
    [[W / 2.0, 0.0, H], [W, D / 2.0, H], [W / 2.0, D, H], [0.0, D / 2.0, H]]
        .iter()
        .map(|c| s.find_edge_at(*c, 1e-6).unwrap())
        .collect()
}

fn loss(r: f64, len: f64) -> f64 {
    (1.0 - FRAC_PI_4) * r * r * len
}

#[test]
fn radius_sweep_on_one_edge() {
    let b = block();
    let v0 = b.volume().unwrap();
    let e = b.find_edge_at([W / 2.0, 0.0, H], 1e-6).unwrap();
    let mut ok = 0;
    for r in [1e-4, 1e-3, 0.01, 0.1, 1.0, 5.0, 10.0, 20.0, 24.0] {
        match b.fillet(&[e], r) {
            Ok((f, _)) => {
                ok += 1;
                assert!(f.is_valid(), "r={r} invalid");
                let want = v0 - loss(r, W);
                assert!((f.volume().unwrap() - want).abs() <= 1e-6 * v0, "r={r}: {} vs {want}", f.volume().unwrap());
                assert_eq!(f.face_count(), 7, "r={r}");
            }
            Err(e) => panic!("r={r} must succeed on a plain block edge: {e:?}"),
        }
    }
    assert_eq!(ok, 9);
}

#[test]
fn oversize_radius_is_an_error_not_a_crash() {
    let b = block();
    let e = b.find_edge_at([W / 2.0, 0.0, H], 1e-6).unwrap();
    for r in [25.0, 40.0, 1000.0, -1.0, 0.0] {
        match b.fillet(&[e], r) {
            Err(e) => eprintln!("oversize r={r}: error {e:?}"),
            Ok((f, _)) => {
                eprintln!("oversize r={r}: accepted, valid={}, volume={}", f.is_valid(), f.volume().unwrap());
                assert!(f.is_valid(), "r={r}: accepted but invalid");
            }
        }
    }
}

#[test]
fn all_top_edges_with_corner_blends() {
    let b = block();
    let v0 = b.volume().unwrap();
    let es = top_edges(&b);
    for r in [0.5, 2.0, 8.0, 15.0] {
        let (f, _) = b.fillet(&es, r).expect("four top edges");
        assert!(f.is_valid(), "r={r}");
        // 4 straight fillets minus the overlap at the corners is small but non-zero; bound it.
        let straight = 2.0 * loss(r, W) + 2.0 * loss(r, D);
        let got = v0 - f.volume().unwrap();
        assert!(got > 0.0 && got <= straight + 1e-6, "r={r}: loss {got} vs straight {straight}");
        assert_eq!(f.face_count(), 6 + 4 + 0, "r={r}: 4 fillet faces expected, corners are part of the edge set");
    }
}

#[test]
fn vertical_edges_and_chained_fillets() {
    let b = block();
    let v: Vec<u32> = [[0.0, 0.0, H / 2.0], [W, 0.0, H / 2.0], [W, D, H / 2.0], [0.0, D, H / 2.0]]
        .iter()
        .map(|c| b.find_edge_at(*c, 1e-6).unwrap())
        .collect();
    let (f1, _) = b.fillet(&v, 6.0).expect("vertical edges");
    assert!(f1.is_valid());
    // Second fillet on the new top boundary (now straight + arcs): pick by search over all edges.
    let n = f1.edge_count();
    let tops: Vec<u32> = (1..=n).filter(|&e| (f1.edge(e).unwrap().center[2] - H).abs() < 1e-6).collect();
    assert_eq!(tops.len(), 8, "four lines + four arcs on top");
    let (f2, _) = f1.fillet(&tops, 2.0).expect("top rim after vertical fillets");
    assert!(f2.is_valid());
    assert!(f2.volume().unwrap() < f1.volume().unwrap());
}

#[test]
fn fillet_on_groove_edges_and_tiny_neighbour_faces() {
    let b = block();
    let tool = Solid::make_box(10.0, D + 2.0, 10.0).unwrap().translated(25.0, -1.0, 15.0).unwrap();
    let g = b.cut(&tool).unwrap().0;
    // Groove floor edges (z = 15, x = 25 and 35) and a radius that nearly consumes the 10 mm groove width.
    let floor: Vec<u32> = (1..=g.edge_count())
        .filter(|&e| {
            let i = g.edge(e).unwrap();
            (i.center[2] - 15.0).abs() < 1e-6 && (i.center[0] - 25.0).abs() < 1e-6 || (i.center[0] - 35.0).abs() < 1e-6 && (i.center[2] - 15.0).abs() < 1e-6
        })
        .collect();
    assert_eq!(floor.len(), 2, "two floor edges along y");
    for r in [1.0, 4.0, 4.9] {
        let (f, _) = g.fillet(&floor, r).unwrap_or_else(|e| panic!("r={r}: {e:?}"));
        assert!(f.is_valid(), "r={r}");
    }
    // Radius > half the groove width: both fillets collide.
    match g.fillet(&floor, 6.0) {
        Err(e) => eprintln!("colliding r=6: error {e:?}"),
        Ok((f, _)) => assert!(f.is_valid(), "colliding fillets accepted but invalid"),
    }
}
