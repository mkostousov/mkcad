//! K-4: rebuild one document N times and print a canonical report; compare reports across processes/OSes.
//!
//! `det run [--n 100] [--threads 1] [--out FILE]`  rebuild N times, check all runs agree, write the report
//! `det compare A B`                               strict lines must be byte-equal, numeric lines rel 1e-12
//!
//! Report lines are `key value`. `S` lines are strict (solved coordinates, labels, counts: bit-identical required),
//! `N` lines are kernel numerics (volume, areas, centroids: relative 1e-12 required, bits printed for the record).

use kernel::naming::*;
use sketch_solver::model::{ConstraintKind, PointId};
use sketch_solver::profiles::detect_profiles;
use sketch_solver::{AnyId, SketchDoc};
use std::fmt::Write as _;

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn num(out: &mut String, key: &str, x: f64) {
    writeln!(out, "N {key} {} {x:e}", bits(x)).unwrap();
}

fn strict(out: &mut String, key: &str, v: impl std::fmt::Display) {
    writeln!(out, "S {key} {v}").unwrap();
}

/// Build the sketch exactly as a user session would: commands, dimension edits, drags.
fn build_sketch(out: &mut String) -> Vec<ProfilePoint> {
    let mut d = SketchDoc::new();
    d.add_rectangle((0.0, 0.0), (60.0, 40.0)).expect("rect");
    let pts: Vec<PointId> = d.sketch.points.keys().copied().collect();
    let w = d
        .add_dimension_measured(ConstraintKind::DistanceH { a: pts[0], b: pts[1], value: 0.0, sign: 1 }, true)
        .expect("width");
    let wid = w.op.created.iter().find_map(|a| if let AnyId::Constraint(c) = a { Some(*c) } else { None }).unwrap();
    d.add_dimension_measured(ConstraintKind::DistanceV { a: pts[1], b: pts[2], value: 0.0, sign: 1 }, true)
        .expect("height");
    d.add_constraint(ConstraintKind::Fix { entity: sketch_solver::EntityRef::Point(pts[0]) }, true).expect("fix");
    // SK-16-style toggling, ends at 60.
    for i in 0..10 {
        d.set_dimension(wid, if i % 2 == 0 { 40.0 } else { 60.0 }).expect("toggle");
    }
    d.set_dimension(wid, 60.0).expect("final width");
    // An under-constrained rectangle that is dragged around: result must be reproducible.
    d.add_rectangle((100.0, 0.0), (130.0, 25.0)).expect("rect2");
    let p2: Vec<PointId> = d.sketch.points.keys().copied().skip(4).collect();
    for i in 0..10 {
        let t = i as f64;
        let _ = d.drag(p2[2], 130.0 + t * 1.7, 25.0 - t * 0.9);
    }
    for (id, p) in &d.sketch.points {
        strict(out, &format!("sketch.point.{}.x", id.0), bits(p.x));
        strict(out, &format!("sketch.point.{}.y", id.0), bits(p.y));
    }
    let set = detect_profiles(&d.sketch);
    strict(out, "profiles.count", set.profiles.len());
    for (i, p) in set.profiles.iter().enumerate() {
        strict(out, &format!("profile.{i}.area"), bits(p.area));
        strict(out, &format!("profile.{i}.pieces"), p.outer.pieces.len());
    }
    // Extrude the profile that contains the fixed corner (the 60 x 40 rectangle).
    let prof = set
        .profiles
        .iter()
        .find(|p| p.outer.pieces.iter().any(|pc| pc.start == [0.0, 0.0]))
        .expect("profile at origin");
    prof.outer
        .pieces
        .iter()
        .map(|pc| ProfilePoint { curve: format!("c:{}", pc.curve.0), xy: pc.start })
        .collect()
}

fn describe(out: &mut String, body: &Labeled) {
    let s = &body.solid;
    strict(out, "faces", s.face_count());
    strict(out, "edges", s.edge_count());
    strict(out, "valid", s.is_valid());
    num(out, "volume", s.volume().unwrap());
    for f in 1..=s.face_count() {
        let info = s.face(f).unwrap();
        let labels: Vec<&str> = body.face_labels[f as usize - 1].iter().map(String::as_str).collect();
        strict(out, &format!("face.{f}.labels"), labels.join("|"));
        num(out, &format!("face.{f}.area"), info.area);
        for (k, c) in ["x", "y", "z"].iter().enumerate() {
            num(out, &format!("face.{f}.c{c}"), info.center[k]);
        }
    }
}

fn run_once() -> String {
    let mut out = String::new();
    let profile = build_sketch(&mut out);
    let mut tl = Timeline::new();
    tl.features.push(Feature::new("base", FeatureKind::Extrude { profile, height: 25.0 }));
    tl.features.push(Feature::new(
        "groove",
        FeatureKind::CutBox { min: [25.0, -1.0, 18.0], size: [10.0, 42.0, 10.0] },
    ));
    let rb = tl.rebuild();
    let body = rb.body.as_ref().expect("body after groove");
    let top_edge = body.edge_ref_at([12.5, 0.0, 25.0]).expect("top edge");
    let top_face = body.face_ref_at([12.5, 20.0, 25.0]).expect("top face");
    tl.features.push(Feature::new("fillet", FeatureKind::Fillet { edges: vec![top_edge], radius: 2.0 }));
    tl.features.push(Feature::new("shell", FeatureKind::Shell { faces: vec![top_face], thickness: 1.5 }));
    let rb = tl.rebuild();
    for r in &rb.reports {
        strict(&mut out, &format!("feature.{}.health", r.id), format!("{:?}", r.health));
        strict(&mut out, &format!("feature.{}.res", r.id), format!("{:?}", r.resolutions));
        strict(&mut out, &format!("feature.{}.diag", r.id), r.diagnostics.join(","));
    }
    describe(&mut out, rb.body.as_ref().expect("final body"));
    out
}

fn digest(s: &str, only: char) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for l in s.lines().filter(|l| l.starts_with(only)) {
        for b in l.bytes().chain(std::iter::once(b'\n')) {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    h
}

fn run(n: usize, threads: usize, out: Option<&str>) -> bool {
    let first = run_once();
    let mut ok = true;
    let mut mism = 0usize;
    let per = n.div_ceil(threads);
    let results: Vec<Vec<String>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads).map(|_| sc.spawn(move || (0..per).map(|_| run_once()).collect::<Vec<_>>())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for r in results.iter().flatten() {
        if *r != first {
            mism += 1;
        }
    }
    if mism > 0 {
        ok = false;
    }
    println!(
        "runs {} threads {threads} mismatches {mism} strict {:016x} numeric {:016x}",
        results.iter().map(Vec::len).sum::<usize>() + 1,
        digest(&first, 'S'),
        digest(&first, 'N')
    );
    if let Some(p) = out {
        std::fs::write(p, &first).unwrap();
    }
    ok
}

fn compare(a: &str, b: &str) -> bool {
    let (ta, tb) = (std::fs::read_to_string(a).unwrap(), std::fs::read_to_string(b).unwrap());
    let (la, lb): (Vec<_>, Vec<_>) = (ta.lines().collect(), tb.lines().collect());
    if la.len() != lb.len() {
        println!("line count differs: {} vs {}", la.len(), lb.len());
        return false;
    }
    let (mut strict_bad, mut num_bad, mut num_bits_diff, mut worst) = (0, 0, 0, 0.0f64);
    for (x, y) in la.iter().zip(&lb) {
        let (px, py): (Vec<_>, Vec<_>) = (x.split(' ').collect(), y.split(' ').collect());
        if px[0] != py[0] || px[1] != py[1] {
            strict_bad += 1;
            println!("key mismatch: {x} / {y}");
        } else if px[0] == "S" {
            if x != y {
                strict_bad += 1;
                println!("strict differs:\n  {x}\n  {y}");
            }
        } else {
            let (vx, vy): (f64, f64) = (px[3].parse().unwrap(), py[3].parse().unwrap());
            let rel = (vx - vy).abs() / vx.abs().max(vy.abs()).max(1e-300);
            if px[2] != py[2] {
                num_bits_diff += 1;
            }
            worst = worst.max(rel);
            if rel > 1e-12 {
                num_bad += 1;
                println!("numeric differs (rel {rel:e}): {x} / {y}");
            }
        }
    }
    println!(
        "lines {} strict mismatches {strict_bad} numeric beyond 1e-12 {num_bad} numeric not bit-identical {num_bits_diff} worst rel {worst:e}",
        la.len()
    );
    strict_bad == 0 && num_bad == 0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |name: &str, default: &str| -> String {
        args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned().unwrap_or(default.to_string())
    };
    let ok = match args.first().map(String::as_str) {
        Some("run") => {
            let out = opt("--out", "");
            run(opt("--n", "100").parse().unwrap(), opt("--threads", "1").parse().unwrap(), (!out.is_empty()).then_some(out.as_str()))
        }
        Some("compare") => compare(&args[1], &args[2]),
        _ => {
            eprintln!("usage: det run [--n N] [--threads T] [--out FILE] | det compare A B");
            false
        }
    };
    std::process::exit(if ok { 0 } else { 1 });
}
