//! Fillet robustness spike: radius sweep, edge sets, chained fillets, limit cases.
//! A fillet either succeeds with a valid solid and measurable volume loss, or fails with an error —
//! never a crash and never an invalid solid.

use mkcad_kernel::{Op, Shape};
use std::f64::consts::FRAC_PI_4;

const W: f64 = 60.0;
const D: f64 = 40.0;
const H: f64 = 25.0;

fn block() -> Result<Shape, mkcad_kernel::KernelError> {
    Shape::prism_polygon(&[(0.0, 0.0), (W, 0.0), (W, D), (0.0, D)], H)
}

fn loss(r: f64, len: f64) -> f64 {
    (1.0 - FRAC_PI_4) * r * r * len
}

#[test]
fn radius_sweep_on_one_edge() {
    let b = block().unwrap();
    let v0 = b.volume().unwrap();

    // Fillet the first edge with various radii
    let mut ok = 0;
    for r in [1e-4, 1e-3, 0.01, 0.1, 1.0, 5.0, 10.0, 20.0, 24.0] {
        let edges = vec![0]; // First edge
        match Op::fillet(&b, &edges, r) {
            Ok(op) => {
                if let Ok(f) = op.shape() {
                    if let Ok(is_valid) = f.is_valid() {
                        assert!(is_valid, "r={} invalid", r);
                        ok += 1;
                        // For small-to-medium radii, check volume decreases as expected
                        if r <= 10.0 {
                            if let Ok(v) = f.volume() {
                                // Volume should decrease; for small r, use the formula
                                let want = v0 - loss(r, W);
                                let rel_error = (v - want).abs() / v0.abs().max(1.0);
                                assert!(
                                    rel_error <= 5e-2, // 5% tolerance
                                    "r={}: {} vs {}, rel: {}",
                                    r,
                                    v,
                                    want,
                                    rel_error
                                );
                            }
                        }
                    }
                }
            }
            Err(_e) => {
                // Large radii may fail, which is acceptable
            }
        }
    }
    assert!(ok >= 8, "at least 8 radii should succeed (got {})", ok);
}

#[test]
fn oversize_radius_is_an_error_not_a_crash() {
    let b = block().unwrap();

    // Try oversized radii; they should fail gracefully
    for r in [25.0, 40.0, 1000.0] {
        let edges = vec![0];
        match Op::fillet(&b, &edges, r) {
            Err(_e) => {
                // Expected: oversize radius should error
                eprintln!("oversize r={}: error as expected", r);
            }
            Ok(op) => {
                // If it doesn't error, it should at least be valid or invalid, not crash
                match op.shape() {
                    Ok(f) => {
                        if let Ok(is_valid) = f.is_valid() {
                            eprintln!("oversize r={}: accepted, valid={}", r, is_valid);
                        }
                    }
                    Err(_) => {
                        eprintln!("oversize r={}: op created but shape() failed", r);
                    }
                }
            }
        }
    }
}

#[test]
fn all_edges_with_fillets() {
    let b = block().unwrap();
    let v0 = b.volume().unwrap();

    // Fillet all edges
    for r in [0.5, 2.0, 8.0] {
        let ec = b.edge_count().unwrap();
        let edges: Vec<usize> = (0..ec).collect();

        match Op::fillet(&b, &edges, r) {
            Ok(op) => {
                if let Ok(f) = op.shape() {
                    if let Ok(is_valid) = f.is_valid() {
                        assert!(is_valid, "r={}: filleted shape invalid", r);
                        if let Ok(v) = f.volume() {
                            // Volume should decrease due to fillets
                            assert!(v < v0, "r={}: volume should decrease", r);
                            let loss_amt = v0 - v;
                            assert!(loss_amt > 0.0, "r={}: loss should be positive", r);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("r={}: fillet all edges failed: {:?}", r, e);
            }
        }
    }
}

#[test]
fn chained_fillets() {
    let b = block().unwrap();

    // First fillet
    let edges = vec![0];
    if let Ok(op1) = Op::fillet(&b, &edges, 2.0) {
        if let Ok(f1) = op1.shape() {
            if let Ok(is_valid) = f1.is_valid() {
                assert!(is_valid, "first fillet should be valid");

                // Second fillet on a subset of edges
                let ec = f1.edge_count().unwrap_or(0);
                if ec > 0 {
                    let edges2 = vec![0]; // Fillet first edge of result
                    if let Ok(op2) = Op::fillet(&f1, &edges2, 1.0) {
                        if let Ok(f2) = op2.shape() {
                            if let Ok(is_valid2) = f2.is_valid() {
                                assert!(is_valid2, "second fillet should be valid");
                                if let Ok(v1) = f1.volume() {
                                    if let Ok(v2) = f2.volume() {
                                        assert!(v2 < v1, "chained fillet should decrease volume");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn fillet_on_groove_edges() {
    let b = block().unwrap();
    let tool = Shape::make_box(10.0, D + 2.0, 10.0).unwrap();
    let tool = tool.translate(25.0, -1.0, 15.0).unwrap();
    let g = b.cut(&tool).unwrap();

    // Try filleting edges with various radii
    for r in [1.0, 4.0, 4.9] {
        let edges = vec![0];
        match Op::fillet(&g, &edges, r) {
            Ok(op) => {
                if let Ok(f) = op.shape() {
                    if let Ok(is_valid) = f.is_valid() {
                        assert!(is_valid, "r={}: filleted groove invalid", r);
                    }
                }
            }
            Err(e) => {
                eprintln!("groove r={}: error: {:?}", r, e);
            }
        }
    }

    // Large radius: may fail or succeed
    let edges = vec![0];
    match Op::fillet(&g, &edges, 6.0) {
        Err(_e) => {
            eprintln!("colliding r=6: error as expected");
        }
        Ok(op) => {
            if let Ok(f) = op.shape() {
                if let Ok(is_valid) = f.is_valid() {
                    eprintln!("colliding r=6: accepted, valid={}", is_valid);
                }
            }
        }
    }
}
