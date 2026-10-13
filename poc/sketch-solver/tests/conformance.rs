//! Data-driven conformance tests (artifact M, SK-* rows and the F60–F67 constraint matrix).
//!
//! Each `tests/fixtures/*.json` file holds one or more cases; a case is a list of command steps run through
//! [`SketchDoc`] with expectations on codes (never strings), coordinates, DOF and status. Entities are named
//! in the fixture and resolved to the IDs the document allocates. Every case runs twice and must produce a
//! bit-identical final document (determinism, SK-16 / TX-DET).

use serde_json::{Map, Value, json};
use sketch_solver::*;
use std::collections::BTreeMap;
use std::path::Path;

struct Ctx {
    doc: SketchDoc,
    names: BTreeMap<String, AnyId>,
    snaps: BTreeMap<String, String>,
    groups: BTreeMap<String, Vec<String>>,
}

type Outcome<'a> = Result<&'a Applied, &'a Rejection>;

fn snap(sk: &Sketch) -> String {
    serde_json::to_string(sk).unwrap()
}

impl Ctx {
    fn id(&self, name: &str) -> Result<AnyId, String> {
        self.names.get(name).copied().ok_or_else(|| format!("unknown name '{name}'"))
    }

    fn point(&self, name: &str) -> Result<PointId, String> {
        match self.id(name)? {
            AnyId::Point(p) => Ok(p),
            other => Err(format!("'{name}' is not a point: {other:?}")),
        }
    }

    fn curve(&self, name: &str) -> Result<CurveId, String> {
        match self.id(name)? {
            AnyId::Curve(c) => Ok(c),
            other => Err(format!("'{name}' is not a curve: {other:?}")),
        }
    }

    fn constraint(&self, name: &str) -> Result<ConstraintId, String> {
        match self.id(name)? {
            AnyId::Constraint(c) => Ok(c),
            other => Err(format!("'{name}' is not a constraint: {other:?}")),
        }
    }

    fn point_ref(&self, v: &Value) -> Result<PointRef, String> {
        match v {
            Value::String(s) => Ok(PointRef::Existing(self.point(s)?)),
            Value::Array(a) => Ok(PointRef::New(a[0].as_f64().unwrap(), a[1].as_f64().unwrap())),
            _ => Err(format!("bad point ref {v}")),
        }
    }

    /// Replaces entity names in a constraint object by IDs (`entity` of `fix` becomes {"point"|"curve": id}).
    fn constraint_kind(&self, v: &Value) -> Result<ConstraintKind, String> {
        let mut obj: Map<String, Value> = v.as_object().ok_or("constraint must be an object")?.clone();
        for (k, val) in obj.iter_mut() {
            if k == "type" {
                continue;
            }
            if let Value::String(name) = val {
                *val = match self.id(name)? {
                    AnyId::Point(p) if k == "entity" => json!({ "point": p.0 }),
                    AnyId::Curve(c) if k == "entity" => json!({ "curve": c.0 }),
                    AnyId::Point(p) => json!(p.0),
                    AnyId::Curve(c) => json!(c.0),
                    AnyId::Constraint(c) => json!(c.0),
                };
            }
        }
        serde_json::from_value(Value::Object(obj)).map_err(|e| format!("constraint {v}: {e}"))
    }

    fn curves(&self, v: &Value) -> Result<Vec<CurveId>, String> {
        v.as_array().ok_or("curves must be an array")?.iter().map(|n| self.curve(n.as_str().unwrap())).collect()
    }

    fn entity_points(&self, name: &str) -> Result<Vec<PointId>, String> {
        Ok(match self.id(name)? {
            AnyId::Point(p) => vec![p],
            AnyId::Curve(c) => self.doc.sketch.curve(c).points(),
            AnyId::Constraint(_) => return Err(format!("'{name}' is not geometry")),
        })
    }

    fn name_created(&mut self, base: &str, created: &[AnyId], suffixes: &[&str]) {
        if base.is_empty() {
            return;
        }
        for (id, s) in created.iter().zip(suffixes) {
            let n = if s.is_empty() { base.to_string() } else { format!("{base}.{s}") };
            self.names.insert(n, *id);
        }
    }
}

fn f(v: &Value, key: &str) -> f64 {
    v[key].as_f64().unwrap_or_else(|| panic!("missing number '{key}' in {v}"))
}

fn line_len(sk: &Sketch, c: CurveId) -> f64 {
    let (a, b) = sk.curve(c).line_points().unwrap();
    let (pa, pb) = (sk.xy(a), sk.xy(b));
    ((pb.0 - pa.0).powi(2) + (pb.1 - pa.1).powi(2)).sqrt()
}

fn diags<'a>(out: &Outcome<'a>) -> Vec<&'a Diagnostic> {
    match out {
        Ok(a) => a.op.diagnostics.iter().chain(a.solve.diagnostics.iter()).collect(),
        Err(r) => r.diagnostics.iter().collect(),
    }
}

fn code_str(c: Code) -> &'static str {
    c.as_str()
}

fn check(ctx: &Ctx, e: &Value, out: Outcome, before: &Sketch, undo_before: usize) -> Result<(), String> {
    let tol = e.get("tol").and_then(Value::as_f64).unwrap_or(1e-9);
    let sk = &ctx.doc.sketch;
    let ds = diags(&out);
    let codes: Vec<&str> = ds.iter().map(|d| code_str(d.code)).collect();
    match e.get("error").and_then(Value::as_str) {
        Some(code) => {
            if out.is_ok() {
                return Err(format!("expected error {code}, got success"));
            }
            if !codes.contains(&code) {
                return Err(format!("expected error {code}, got {codes:?}"));
            }
        }
        None => {
            if let Err(r) = &out {
                return Err(format!("unexpected rejection {:?}", r.diagnostics));
            }
        }
    }
    if let Some(list) = e.get("codes").and_then(Value::as_array) {
        for c in list {
            let c = c.as_str().unwrap();
            if !codes.contains(&c) {
                return Err(format!("missing code {c}; got {codes:?}"));
            }
        }
    }
    if let Some(list) = e.get("no_codes").and_then(Value::as_array) {
        for c in list {
            let c = c.as_str().unwrap();
            if codes.contains(&c) {
                return Err(format!("unexpected code {c}"));
            }
        }
    }
    for (key, code_prefix) in [("conflicts", "CONFLICT"), ("redundant_set", "REDUNDANT")] {
        if let Some(list) = e.get(key).and_then(Value::as_array) {
            // "$new" stands for the rejected constraint itself: its ID was allocated but never entered the doc.
            let n_new = list.iter().filter(|n| n.as_str() == Some("$new")).count();
            let mut want: Vec<ConstraintId> = list
                .iter()
                .filter(|n| n.as_str() != Some("$new"))
                .map(|n| ctx.constraint(n.as_str().unwrap()))
                .collect::<Result<_, _>>()?;
            want.sort();
            let got = ds.iter().find(|d| code_str(d.code).ends_with(code_prefix)).map(|d| d.constraints.clone());
            let ok = got.as_ref().is_some_and(|g| {
                let extra: Vec<&ConstraintId> = g.iter().filter(|c| !want.contains(c)).collect();
                want.iter().all(|w| g.contains(w))
                    && extra.len() == n_new
                    && extra.iter().all(|c| !ctx.doc.sketch.constraints.contains_key(c))
            });
            if !ok {
                return Err(format!("{key}: want {want:?} + {n_new} new, got {got:?}"));
            }
        }
    }
    if let Some(s) = e.get("suggestion").and_then(Value::as_str) {
        let got: Vec<String> =
            ds.iter().filter_map(|d| d.suggestion).map(|s| serde_json::to_value(s).unwrap().to_string()).collect();
        if !got.iter().any(|g| g.trim_matches('"') == s) {
            return Err(format!("suggestion {s} missing: {got:?}"));
        }
    }
    if e.get("unchanged").and_then(Value::as_bool) == Some(true) {
        if snap(sk) != snap(before) {
            return Err("document changed".into());
        }
        if ctx.doc.undo_depth() != undo_before {
            return Err(format!("undo depth changed {undo_before} → {}", ctx.doc.undo_depth()));
        }
    }
    if let Some(n) = e.get("undo_depth").and_then(Value::as_u64)
        && ctx.doc.undo_depth() as u64 != n
    {
        return Err(format!("undo depth {} != {n}", ctx.doc.undo_depth()));
    }
    if e.get("status").is_some() || e.get("dof").is_some() || e.get("entity_dof").is_some() {
        let a = ctx.doc.analyze();
        if let Some(s) = e.get("status").and_then(Value::as_str) {
            let got = serde_json::to_value(a.status).unwrap();
            if got != s {
                return Err(format!("status {got} != {s} ({:?})", a.diagnostics));
            }
        }
        if let Some(d) = e.get("dof").and_then(Value::as_u64)
            && a.dof as u64 != d
        {
            return Err(format!("dof {} != {d}", a.dof));
        }
        if let Some(m) = e.get("entity_dof").and_then(Value::as_object) {
            for (name, want) in m {
                let er = match ctx.id(name)? {
                    AnyId::Point(p) => EntityRef::Point(p),
                    AnyId::Curve(c) => EntityRef::Curve(c),
                    _ => return Err(format!("{name} is not geometry")),
                };
                let got = a.entity_dof.get(&er).copied().unwrap_or(usize::MAX);
                if got as u64 != want.as_u64().unwrap() {
                    return Err(format!("entity_dof[{name}] {got} != {want}"));
                }
            }
        }
    }
    if let Some(m) = e.get("points").and_then(Value::as_object) {
        for (name, xy) in m {
            let (x, y) = sk.xy(ctx.point(name)?);
            let (wx, wy) = (xy[0].as_f64().unwrap(), xy[1].as_f64().unwrap());
            if (x - wx).abs() > tol || (y - wy).abs() > tol {
                return Err(format!("point {name} = ({x}, {y}), want ({wx}, {wy}) ±{tol}"));
            }
        }
    }
    if let Some(m) = e.get("radius").and_then(Value::as_object) {
        for (name, r) in m {
            let got = sk.curve(ctx.curve(name)?).radius().unwrap();
            if (got - r.as_f64().unwrap()).abs() > tol {
                return Err(format!("radius {name} = {got}, want {r}"));
            }
        }
    }
    if let Some(m) = e.get("length").and_then(Value::as_object) {
        for (name, l) in m {
            let got = line_len(sk, ctx.curve(name)?);
            if (got - l.as_f64().unwrap()).abs() > tol {
                return Err(format!("length {name} = {got}, want {l}"));
            }
        }
    }
    if let Some(list) = e.get("same_length").and_then(Value::as_array) {
        for n in list {
            let c = ctx.curve(n.as_str().unwrap())?;
            let (l0, l1) = (line_len(before, c), line_len(sk, c));
            if (l0 - l1).abs() > tol {
                return Err(format!("length of {n} changed {l0} → {l1}"));
            }
        }
    }
    if let Some(list) = e.get("unmoved").and_then(Value::as_array) {
        for n in list {
            for p in ctx.entity_points(n.as_str().unwrap())? {
                let (a, b) = (before.xy(p), sk.xy(p));
                if (a.0 - b.0).abs() > tol || (a.1 - b.1).abs() > tol {
                    return Err(format!("{n} moved: {a:?} → {b:?}"));
                }
            }
        }
    }
    if let Some(list) = e.get("y_positive").and_then(Value::as_array) {
        for n in list {
            let (_, y) = sk.xy(ctx.point(n.as_str().unwrap())?);
            if y <= 0.0 {
                return Err(format!("{n}.y = {y} not > 0"));
            }
        }
    }
    if let Some(m) = e.get("dim_value").and_then(Value::as_object) {
        for (name, v) in m {
            let c = &sk.constraints[&ctx.constraint(name)?];
            let got = c.kind.value().unwrap();
            if (got - v.as_f64().unwrap()).abs() > tol {
                return Err(format!("dimension {name} = {got}, want {v}"));
            }
        }
    }
    if let Some(m) = e.get("driving").and_then(Value::as_object) {
        for (name, v) in m {
            let c = &sk.constraints[&ctx.constraint(name)?];
            if c.driving != v.as_bool().unwrap() {
                return Err(format!("{name}.driving = {}", c.driving));
            }
        }
    }
    if let Some(m) = e.get("kind").and_then(Value::as_object) {
        for (name, v) in m {
            let got = sk.constraints.get(&ctx.constraint(name)?).map(|c| c.kind.name());
            if got != v.as_str() {
                return Err(format!("{name} kind {got:?} != {v}"));
            }
        }
    }
    if let Some(list) = e.get("removed").and_then(Value::as_array) {
        for n in list {
            if sk.constraints.contains_key(&ctx.constraint(n.as_str().unwrap())?) {
                return Err(format!("{n} still present"));
            }
        }
    }
    if let Some(a) = e.get("area") {
        let pts: Vec<(f64, f64)> = a["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| Ok(sk.xy(ctx.point(n.as_str().unwrap())?)))
            .collect::<Result<_, String>>()?;
        let mut s = 0.0;
        for i in 0..pts.len() {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            s += p.0 * q.1 - q.0 * p.1;
        }
        let area = (s / 2.0).abs();
        if (area - f(a, "value")).abs() > tol {
            return Err(format!("area {area} != {}", a["value"]));
        }
    }
    if let Some(l) = e.get("snapshot_eq").and_then(Value::as_str)
        && ctx.snaps.get(l) != Some(&snap(sk))
    {
        return Err(format!("document differs from snapshot '{l}'"));
    }
    if let Some(n) = e.get("modified_count").and_then(Value::as_u64)
        && let Ok(a) = &out
        && a.op.modified.len() as u64 != n
    {
        return Err(format!("modified {:?}", a.op.modified));
    }
    Ok(())
}

fn step(ctx: &mut Ctx, s: &Value) -> Result<(), String> {
    let op = s["op"].as_str().ok_or("step without op")?;
    let name = s.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let before = ctx.doc.sketch.clone();
    let undo_before = ctx.doc.undo_depth();
    let result: OpResult = match op {
        "repeat" => {
            for _ in 0..s["times"].as_u64().unwrap() {
                for inner in s["steps"].as_array().unwrap() {
                    step(ctx, inner)?;
                }
            }
            return Ok(());
        }
        "snapshot" => {
            let label = s["label"].as_str().unwrap().to_string();
            ctx.snaps.insert(label.clone(), snap(&ctx.doc.sketch));
            return Ok(());
        }
        "collect" => {
            // Geometry only: provenance legitimately records every edit.
            let sk = &ctx.doc.sketch;
            let geo: Vec<(u64, u64)> = sk
                .points
                .values()
                .flat_map(|p| [p.x, p.y])
                .chain(sk.curves.values().filter_map(|c| c.radius()))
                .map(|v| match s.get("tol").and_then(Value::as_f64) {
                    // Positions equal within tol (spec tolerance) instead of bit-equal.
                    Some(t) => ((v / t).round() as i64 as u64, 0),
                    None => (v.to_bits(), 0),
                })
                .collect();
            let s2 = format!("{geo:?}");
            ctx.groups.entry(s["group"].as_str().unwrap().to_string()).or_default().push(s2);
            return Ok(());
        }
        "distinct" => {
            let g = &ctx.groups[s["group"].as_str().unwrap()];
            let mut uniq = g.clone();
            uniq.sort();
            uniq.dedup();
            let want = s["count"].as_u64().unwrap() as usize;
            return if uniq.len() == want {
                Ok(())
            } else {
                Err(format!("group has {} distinct states, want {want}", uniq.len()))
            };
        }
        "check" => {
            let a = ctx.doc.analyze();
            let applied = Applied {
                op: SketchOp {
                    id: OpId(0),
                    seq: 0,
                    kind: "check".into(),
                    created: vec![],
                    modified: vec![],
                    deleted: vec![],
                    group_id: None,
                    diagnostics: vec![],
                },
                solve: a,
            };
            return check(ctx, &s["expect"], Ok(&applied), &before, undo_before);
        }
        "undo" | "redo" => {
            let r = if op == "undo" { ctx.doc.undo() } else { ctx.doc.redo() };
            if r.is_none() {
                return Err(format!("nothing to {op}"));
            }
            if let Some(e) = s.get("expect") {
                let a = Applied { op: r.unwrap(), solve: ctx.doc.analyze() };
                return check(ctx, e, Ok(&a), &before, undo_before);
            }
            return Ok(());
        }
        "point" => {
            let at = &s["at"];
            ctx.doc.add_point(at[0].as_f64().unwrap(), at[1].as_f64().unwrap())
        }
        "line" => {
            let (a, b) = (ctx.point_ref(&s["a"])?, ctx.point_ref(&s["b"])?);
            ctx.doc.add_line(a, b)
        }
        "circle" => {
            let c = ctx.point_ref(&s["center"])?;
            ctx.doc.add_circle(c, f(s, "r"))
        }
        "arc" => {
            let (c, a, b) = (ctx.point_ref(&s["center"])?, ctx.point_ref(&s["start"])?, ctx.point_ref(&s["end"])?);
            ctx.doc.add_arc(c, a, b)
        }
        "rectangle" => {
            let (p1, p2) = (&s["p1"], &s["p2"]);
            ctx.doc.add_rectangle(
                (p1[0].as_f64().unwrap(), p1[1].as_f64().unwrap()),
                (p2[0].as_f64().unwrap(), p2[1].as_f64().unwrap()),
            )
        }
        "constrain" => {
            let kind = ctx.constraint_kind(&s["c"])?;
            let driving = s.get("driving").and_then(Value::as_bool).unwrap_or(true);
            if s.get("measured").and_then(Value::as_bool) == Some(true) {
                ctx.doc.add_dimension_measured(kind, driving)
            } else {
                ctx.doc.add_constraint(kind, driving)
            }
        }
        "set" => {
            let id = ctx.constraint(s["dim"].as_str().unwrap())?;
            ctx.doc.set_dimension(id, f(s, "value"))
        }
        "driving" => {
            let id = ctx.constraint(s["dim"].as_str().unwrap())?;
            ctx.doc.set_driving(id, s["value"].as_bool().unwrap())
        }
        "drag" => {
            let p = ctx.point(s["p"].as_str().unwrap())?;
            ctx.doc.drag(p, s["to"][0].as_f64().unwrap(), s["to"][1].as_f64().unwrap())
        }
        "delete" => {
            let ids: Vec<AnyId> =
                s["ids"].as_array().unwrap().iter().map(|n| ctx.id(n.as_str().unwrap())).collect::<Result<_, _>>()?;
            ctx.doc.delete(&ids)
        }
        "rotate" => {
            let cs = ctx.curves(&s["curves"])?;
            ctx.doc.rotate(&cs, (s["center"][0].as_f64().unwrap(), s["center"][1].as_f64().unwrap()), f(s, "deg"))
        }
        "scale" => {
            let cs = ctx.curves(&s["curves"])?;
            ctx.doc.scale(&cs, (s["center"][0].as_f64().unwrap(), s["center"][1].as_f64().unwrap()), f(s, "factor"))
        }
        other => return Err(format!("unknown op '{other}'")),
    };
    if let Ok(a) = &result {
        let created = a.op.created.clone();
        match op {
            "point" => ctx.name_created(&name, &created, &[""]),
            "line" => {
                // Created: new end points (if any) then the line.
                let line = *created.last().unwrap();
                ctx.names.insert(name.clone(), line);
                if let AnyId::Curve(c) = line {
                    let (pa, pb) = ctx.doc.sketch.curve(c).line_points().unwrap();
                    ctx.names.insert(format!("{name}.a"), AnyId::Point(pa));
                    ctx.names.insert(format!("{name}.b"), AnyId::Point(pb));
                }
            }
            "circle" | "arc" => {
                let curve = *created.last().unwrap();
                ctx.names.insert(name.clone(), curve);
                if let AnyId::Curve(c) = curve {
                    let pts = ctx.doc.sketch.curve(c).points();
                    for (p, sfx) in pts.iter().zip(["c", "s", "e"]) {
                        ctx.names.insert(format!("{name}.{sfx}"), AnyId::Point(*p));
                    }
                }
            }
            "rectangle" => ctx.name_created(
                &name,
                &created,
                &["p0", "p1", "p2", "p3", "bottom", "right", "top", "left", "h1", "v1", "h2", "v2"],
            ),
            "constrain" => ctx.name_created(&name, &created, &[""]),
            _ => {}
        }
    }
    let expect = s.get("expect").cloned().unwrap_or(json!({}));
    check(ctx, &expect, result.as_ref(), &before, undo_before)
}

fn run_case(case: &Value) -> Result<String, String> {
    let mut ctx =
        Ctx { doc: SketchDoc::new(), names: BTreeMap::new(), snaps: BTreeMap::new(), groups: BTreeMap::new() };
    for (i, s) in case["steps"].as_array().ok_or("case without steps")?.iter().enumerate() {
        step(&mut ctx, s).map_err(|e| format!("step {i} ({}): {e}", s["op"]))?;
    }
    Ok(snap(&ctx.doc.sketch))
}

fn run_fixture(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let fx: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let cases: Vec<Value> = match fx.get("cases") {
        Some(c) => c.as_array().unwrap().clone(),
        None => vec![fx.clone()],
    };
    let mut failures = Vec::new();
    for case in &cases {
        let label = case.get("name").or(fx.get("id")).and_then(Value::as_str).unwrap_or("?").to_string();
        match (run_case(case), run_case(case)) {
            (Ok(a), Ok(b)) if a != b => failures.push(format!("{label}: not deterministic across runs")),
            (Ok(_), Ok(_)) => {}
            (Err(e), _) | (_, Err(e)) => failures.push(format!("{label}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{} ({}):\n  {}", fx["id"], file, failures.join("\n  "));
}

macro_rules! fixtures {
    ($($name:ident => $file:literal),* $(,)?) => {
        $( #[test] fn $name() { run_fixture($file); } )*
    };
}

fixtures! {
    sk01_rectangle => "sk01_rectangle.json",
    sk02_dimension_toggle => "sk02_dimension_toggle.json",
    sk03_redundant_dimension => "sk03_redundant_dimension.json",
    sk04_invalid_values => "sk04_invalid_values.json",
    sk05_conflict_keeps_geometry => "sk05_conflict.json",
    sk06_degenerate_triangle => "sk06_degenerate.json",
    sk07_branch_stability => "sk07_branch.json",
    sk08_constraint_matrix => "sk08_constraint_matrix.json",
    sk09_duplicates => "sk09_duplicates.json",
    sk13_undo_redo => "sk13_undo_redo.json",
    sk16_under_constrained_reproducible => "sk16_reproducible.json",
    sk17_fix_conflict => "sk17_fix_conflict.json",
    sk18_symmetry_true_mirror => "sk18_symmetry.json",
    sk19_rotate_scale => "sk19_rotate_scale.json",
    s07_angle_dimension => "s07_angle.json",
    s16_drag => "s16_drag.json",
    dof_samples => "dof_samples.json",
}

/// Cross-platform determinism (TX-DET / K-1 exit "identical on Windows + Linux"): FNV-1a digest over the
/// final document of every fixture case, compared with `tests/golden/digest.txt`. Regenerate with
/// `UPDATE_GOLDEN=1 cargo test --test conformance cross_platform_digest` after an intended numeric change.
#[test]
fn cross_platform_digest() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    let mut h: u64 = 0xcbf29ce484222325;
    let mut lines = Vec::new();
    for f in files {
        let fx: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let cases = fx.get("cases").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![fx.clone()]);
        for case in &cases {
            let state = run_case(case).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            let mut ch: u64 = 0xcbf29ce484222325;
            for b in state.bytes() {
                ch = (ch ^ b as u64).wrapping_mul(0x100000001b3);
                h = (h ^ b as u64).wrapping_mul(0x100000001b3);
            }
            let name = case.get("name").or(fx.get("id")).and_then(Value::as_str).unwrap_or("?");
            lines.push(format!("{ch:016x} {} :: {name}", f.file_name().unwrap().to_string_lossy()));
        }
    }
    let text = format!("total {h:016x}\n{}\n", lines.join("\n"));
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/digest.txt");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, &text).unwrap();
    }
    let want = std::fs::read_to_string(&golden)
        .expect("golden digest missing; run with UPDATE_GOLDEN=1")
        .replace("\r\n", "\n");
    assert_eq!(text, want, "solved documents differ from the golden digest (platform or numeric change)");
}
