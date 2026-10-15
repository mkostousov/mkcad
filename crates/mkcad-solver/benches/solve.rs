//! K-1 performance exit criterion: ≤ 50 ms per interactive solve for 500 entities / 600 constraints.
//!
//! "Ladder" sketch = one connected cluster (worst case for decomposition): 150 bays of a ladder
//! (151 rungs, 300 rails = 451 lines) + 49 free circles = 500 curves; 2H per bay + 1V per rung + 1 width per
//! bay + 1 height + 1 fix = 603 constraints; 351 points → 604 ladder unknowns (fully constrained) + 147 free circle DOF.

use criterion::{Criterion, criterion_group, criterion_main};
use sketch_solver::*;
use std::hint::black_box;

pub struct Ladder {
    pub sketch: Sketch,
    pub widths: Vec<ConstraintId>,
    pub top: Vec<PointId>,
}

pub fn ladder(bays: usize, circles: usize, dimensioned: bool) -> Ladder {
    let mut sk = Sketch::default();
    let mut next = 1u64;
    let mut id = || {
        next += 1;
        next
    };
    let point = |sk: &mut Sketch, id: u64, x: f64, y: f64| {
        let p = PointId(id);
        sk.points.insert(p, Point { id: p, x, y, prov: Provenance::default() });
        p
    };
    let mut bottom = Vec::new();
    let mut top = Vec::new();
    // Slightly perturbed start so the solver has real work to do.
    for i in 0..=bays {
        let x = 10.0 * i as f64;
        let w = 0.3 * ((i * 7919) % 13) as f64 / 13.0;
        bottom.push(point(&mut sk, id(), x + w, 0.2 * w));
        top.push(point(&mut sk, id(), x - w, 8.0 + w));
    }
    let mut line = |sk: &mut Sketch, a: PointId, b: PointId| {
        let c = CurveId(id());
        sk.curves.insert(
            c,
            Curve { id: c, geom: CurveGeom::Line { a, b }, construction: false, prov: Provenance::default() },
        );
        c
    };
    let mut cons: Vec<ConstraintKind> = Vec::new();
    let mut widths_at = Vec::new();
    for i in 0..=bays {
        let r = line(&mut sk, bottom[i], top[i]);
        cons.push(ConstraintKind::Vertical { line: r });
        if i < bays {
            let b = line(&mut sk, bottom[i], bottom[i + 1]);
            let t = line(&mut sk, top[i], top[i + 1]);
            cons.push(ConstraintKind::Horizontal { line: b });
            cons.push(ConstraintKind::Horizontal { line: t });
            if dimensioned {
                widths_at.push(cons.len());
                cons.push(ConstraintKind::DistanceH { a: bottom[i], b: bottom[i + 1], value: 10.0, sign: 1 });
            }
        }
    }
    cons.push(ConstraintKind::DistanceV { a: bottom[0], b: top[0], value: 8.0, sign: 1 });
    cons.push(ConstraintKind::Fix { entity: EntityRef::Point(bottom[0]) });
    for k in 0..circles {
        let c = point(&mut sk, id(), 5.0 + 10.0 * k as f64, -20.0);
        let cid = CurveId(id());
        sk.curves.insert(
            cid,
            Curve {
                id: cid,
                geom: CurveGeom::Circle { center: c, r: 3.0 },
                construction: false,
                prov: Provenance::default(),
            },
        );
    }
    let mut widths = Vec::new();
    for (i, kind) in cons.into_iter().enumerate() {
        let cid = ConstraintId(id());
        sk.constraints.insert(cid, Constraint { id: cid, kind, driving: true, prov: Provenance::default() });
        if widths_at.contains(&i) {
            widths.push(cid);
        }
    }
    Ladder { sketch: sk, widths, top }
}

/// Nonlinear stress case: open polyline of `n` segments (aligned length + angle to the previous segment),
/// first point fixed, first segment horizontal; plus free circles up to 500 curves. Editing one angle in the
/// middle swings the whole tail (long lever arm, every equation nonlinear).
pub fn chain(n: usize, circles: usize) -> (Sketch, Vec<ConstraintId>) {
    let mut sk = Sketch::default();
    let mut next = 1u64;
    let mut id = || {
        next += 1;
        next
    };
    let mut pts = Vec::new();
    let (mut x, mut y, mut heading) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..=n {
        let p = PointId(id());
        sk.points.insert(p, Point { id: p, x, y, prov: Provenance::default() });
        pts.push(p);
        heading += if i % 2 == 0 { 0.35 } else { -0.3 };
        x += 5.0 * libm::cos(heading);
        y += 5.0 * libm::sin(heading);
    }
    let mut lines = Vec::new();
    for i in 0..n {
        let c = CurveId(id());
        sk.curves.insert(
            c,
            Curve {
                id: c,
                geom: CurveGeom::Line { a: pts[i], b: pts[i + 1] },
                construction: false,
                prov: Provenance::default(),
            },
        );
        lines.push(c);
    }
    for k in 0..circles {
        let p = PointId(id());
        sk.points.insert(p, Point { id: p, x: 7.0 * k as f64, y: -50.0, prov: Provenance::default() });
        let c = CurveId(id());
        sk.curves.insert(
            c,
            Curve {
                id: c,
                geom: CurveGeom::Circle { center: p, r: 2.0 },
                construction: false,
                prov: Provenance::default(),
            },
        );
    }
    let mut cons =
        vec![ConstraintKind::Fix { entity: EntityRef::Point(pts[0]) }, ConstraintKind::Horizontal { line: lines[0] }];
    let mut angles = Vec::new();
    for i in 0..n {
        cons.push(ConstraintKind::Distance { a: pts[i], b: pts[i + 1], value: 5.0 });
        if i > 0 {
            let deg = if i % 2 == 0 { 20.0 } else { 17.0 };
            cons.push(ConstraintKind::Angle {
                l1: lines[i - 1],
                l2: lines[i],
                degrees: deg,
                sign: if i % 2 == 0 { 1 } else { -1 },
            });
        }
    }
    for kind in cons {
        let cid = ConstraintId(id());
        if matches!(kind, ConstraintKind::Angle { .. }) {
            angles.push(cid);
        }
        sk.constraints.insert(cid, Constraint { id: cid, kind, driving: true, prov: Provenance::default() });
    }
    (sk, angles)
}

fn benches(c: &mut Criterion) {
    let base = ladder(150, 49, true);
    let opts = SolveOptions::default();
    let solved = solve(&base.sketch, &EditContext::default(), &opts);
    // The ladder is fully constrained; only the 49 free circles keep their 3 DOF each.
    assert_eq!(solved.status, Status::UnderConstrained, "{:?}", solved.diagnostics);
    assert_eq!(solved.dof, 49 * 3);
    assert_eq!(solved.sketch.curves.len(), 500);
    eprintln!(
        "ladder: {} curves, {} points, {} constraints, dof {}",
        solved.sketch.curves.len(),
        solved.sketch.points.len(),
        solved.sketch.constraints.len(),
        solved.dof
    );

    // Edit the middle width dimension 10 → 12 mm on the solved sketch (typical interactive edit).
    let mut edited = solved.sketch.clone();
    let w = base.widths[75];
    edited.constraints.get_mut(&w).unwrap().kind.set_value(12.0);
    let ctx = EditContext::for_constraint(&edited.constraints[&w].kind.clone(), &solved.sketch);
    let mut g = c.benchmark_group("ladder_500_curves_603_constraints");
    g.sample_size(20);
    g.bench_function("edit_dimension_full_analysis", |b| b.iter(|| black_box(solve(black_box(&edited), &ctx, &opts))));
    let lean = SolveOptions { per_entity_dof: false, ..opts.clone() };
    g.bench_function("edit_dimension_no_entity_dof", |b| b.iter(|| black_box(solve(black_box(&edited), &ctx, &lean))));
    g.bench_function("initial_solve_from_perturbed", |b| {
        b.iter(|| black_box(solve(black_box(&base.sketch), &EditContext::default(), &lean)))
    });

    // Under-constrained ladder (no widths): drag a top point — 150 DOF remain.
    let free = solve(&ladder(150, 49, false).sketch, &EditContext::default(), &lean).sketch;
    let p = base.top[75];
    let (x, y) = free.xy(p);
    let drag = EditContext::for_drag(p, x + 3.0, y + 1.0);
    g.bench_function("drag_under_constrained", |b| b.iter(|| black_box(solve(black_box(&free), &drag, &opts))));
    g.finish();

    let (chain_sk, angles) = chain(300, 200);
    let chain_solved = solve(&chain_sk, &EditContext::default(), &opts);
    assert_eq!(chain_solved.dof, 200 * 3, "{:?}", chain_solved.diagnostics);
    eprintln!(
        "chain: {} curves, {} constraints, dof {}, iterations {}",
        chain_solved.sketch.curves.len(),
        chain_solved.sketch.constraints.len(),
        chain_solved.dof,
        chain_solved.iterations
    );
    let mut bent = chain_solved.sketch.clone();
    let a = angles[150];
    bent.constraints.get_mut(&a).unwrap().kind.set_value(35.0);
    let ctx = EditContext::for_constraint(&bent.constraints[&a].kind.clone(), &chain_solved.sketch);
    let check = solve(&bent, &ctx, &opts);
    assert!(check.is_ok(), "{:?}", check.diagnostics);
    eprintln!("chain edit: iterations {}, residual {:e}", check.iterations, check.residual);
    let mut g = c.benchmark_group("chain_300_lines_200_circles_601_constraints");
    g.sample_size(20);
    g.bench_function("edit_angle_full_analysis", |b| b.iter(|| black_box(solve(black_box(&bent), &ctx, &opts))));
    g.bench_function("edit_angle_no_entity_dof", |b| b.iter(|| black_box(solve(black_box(&bent), &ctx, &lean))));
    g.finish();
}

criterion_group!(solver, benches);
criterion_main!(solver);
