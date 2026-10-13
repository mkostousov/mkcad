//! Solver core (F11 §2): cluster decomposition → least-norm Gauss–Newton with minimal-change preferences →
//! LM fallback → rank analysis (DOF, redundancy, conflicts) → branch and degeneracy guards.
//!
//! Minimal change (F61–F67) is expressed as *soft goals* appended after the hard equations: a goal that is
//! linearly dependent on the hard equations (or on higher-priority goals) is dropped by the in-order pivoting
//! of [`crate::linalg::factor`]; independent goals are met exactly. Remaining freedom is resolved by the
//! least-norm step, i.e. the smallest weighted coordinate change.

use crate::diag::{Code, Diagnostic, Suggestion};
use crate::equations::*;
use crate::linalg::{FactorOpts, SRow, factor};
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Soft goal of an edit (never reported as conflicting; dropped when it cannot be met).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "goal", rename_all = "snake_case")]
pub enum Goal {
    /// Point stays where it is.
    KeepPoint {
        p: PointId,
    },
    /// `to − from` keeps its value (rigid translation of the pair).
    KeepOffset {
        from: PointId,
        to: PointId,
    },
    KeepLength {
        line: CurveId,
    },
    KeepDirection {
        line: CurveId,
    },
    KeepMidpoint {
        line: CurveId,
    },
    KeepRadius {
        curve: CurveId,
    },
    /// Drag target.
    Target {
        p: PointId,
        x: f64,
        y: f64,
    },
}

/// What the edit touches (F11 §4 `EditContext`): goals by priority, and entities that are cheap to move.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditContext {
    #[serde(default)]
    pub goals: Vec<Goal>,
    #[serde(default)]
    pub moving: Vec<EntityRef>,
}

#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub max_iter: usize,
    /// Residual tolerance ‖r‖∞ in mm (dimensionless rows are scaled by the sketch size).
    pub tol: f64,
    pub rank_tol: f64,
    pub per_entity_dof: bool,
    /// Largest cluster (in constraints) for the deletion-filter conflict search; larger clusters report
    /// the fundamental circuit of the violated rows instead.
    pub conflict_search_limit: usize,
    /// Retry by continuation when a solve flips an orientation (F107).
    pub branch_guard: bool,
}

impl Default for SolveOptions {
    fn default() -> Self {
        SolveOptions {
            max_iter: 50,
            tol: 1e-10,
            rank_tol: 1e-10,
            per_entity_dof: true,
            conflict_search_limit: 64,
            branch_guard: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    FullyConstrained,
    UnderConstrained,
    Redundant,
    Conflict,
    NonConvergent,
}

#[derive(Clone, Debug, Serialize)]
pub struct SolveResult {
    pub status: Status,
    /// Solved sketch (coordinates, radii, driven dimension values). On failure: the best attempt.
    pub sketch: Sketch,
    pub dof: usize,
    pub entity_dof: BTreeMap<EntityRef, usize>,
    /// Redundant constraints (dropped rows whose residual is zero), each with its minimal dependent set.
    pub redundant: Vec<(ConstraintId, Vec<ConstraintId>)>,
    /// Smallest conflicting set found.
    pub conflicts: Vec<ConstraintId>,
    pub diagnostics: Vec<Diagnostic>,
    pub iterations: usize,
    pub residual: f64,
}

impl SolveResult {
    pub fn has(&self, code: Code) -> bool {
        self.diagnostics.iter().any(|d| d.code == code)
    }

    pub fn is_ok(&self) -> bool {
        matches!(self.status, Status::FullyConstrained | Status::UnderConstrained | Status::Redundant)
            && !self.has(Code::DimInvalidValue)
    }
}

// ---------------------------------------------------------------------------------------------------------
// Problem assembly

struct Problem {
    slots: Slots,
    eqs: Vec<Eq>,
    n_hard: usize,
    fixed: Vec<bool>,
    winv: Vec<f64>,
}

fn fixed_slots(sk: &Sketch, slots: &Slots, active: &dyn Fn(&Constraint) -> bool) -> Vec<bool> {
    let mut fixed = vec![false; slots.vals.len()];
    for c in sk.constraints.values().filter(|c| active(c)) {
        if let ConstraintKind::Fix { entity } = c.kind {
            for p in sk.entity_points(entity) {
                let s = slots.p(p);
                fixed[s[0]] = true;
                fixed[s[1]] = true;
            }
            if let EntityRef::Curve(cid) = entity
                && let Some(&r) = slots.rad.get(&cid)
            {
                fixed[r] = true;
            }
        }
    }
    fixed
}

fn goal_eqs(g: &Goal, sk: &Sketch, slots: &Slots, scale: f64, prio: usize, out: &mut Vec<Eq>) {
    let owner = Owner::Soft(prio);
    let mut push = |prim| out.push(Eq { prim, owner });
    let lin = |terms: Vec<(usize, f64)>, k: f64| Prim::Lin { terms, k };
    let line = |id: CurveId| sk.curve(id).line_points();
    match *g {
        Goal::KeepPoint { p } | Goal::Target { p, .. } => {
            let (x, y) = match *g {
                Goal::Target { x, y, .. } => (x, y),
                _ => sk.xy(p),
            };
            let s = slots.p(p);
            push(lin(vec![(s[0], 1.0)], x));
            push(lin(vec![(s[1], 1.0)], y));
        }
        Goal::KeepOffset { from, to } => {
            let (a, b) = (sk.xy(from), sk.xy(to));
            let (sa, sb) = (slots.p(from), slots.p(to));
            push(lin(vec![(sb[0], 1.0), (sa[0], -1.0)], b.0 - a.0));
            push(lin(vec![(sb[1], 1.0), (sa[1], -1.0)], b.1 - a.1));
        }
        Goal::KeepLength { line: l } => {
            if let Some((a, b)) = line(l) {
                let (pa, pb) = (sk.xy(a), sk.xy(b));
                let len = libm::hypot(pb.0 - pa.0, pb.1 - pa.1);
                push(Prim::Dist { a: slots.p(a), b: slots.p(b), r: vec![], k: len });
            }
        }
        Goal::KeepDirection { line: l } => {
            if let Some((a, b)) = line(l) {
                let (pa, pb) = (sk.xy(a), sk.xy(b));
                let len = libm::hypot(pb.0 - pa.0, pb.1 - pa.1);
                if len > 0.0 {
                    let u = [(pb.0 - pa.0) / len, (pb.1 - pa.1) / len];
                    push(Prim::CrossConst { a: slots.p(a), b: slots.p(b), u, s: scale });
                }
            }
        }
        Goal::KeepMidpoint { line: l } => {
            if let Some((a, b)) = line(l) {
                let (pa, pb) = (sk.xy(a), sk.xy(b));
                let (sa, sb) = (slots.p(a), slots.p(b));
                for i in 0..2 {
                    let m = 0.5 * ([pa.0, pa.1][i] + [pb.0, pb.1][i]);
                    push(lin(vec![(sa[i], 0.5), (sb[i], 0.5)], m));
                }
            }
        }
        Goal::KeepRadius { curve } => {
            if let (Some(&s), Some(r)) = (slots.rad.get(&curve), sk.curve(curve).radius()) {
                push(lin(vec![(s, 1.0)], r));
            }
        }
    }
}

fn build(sk: &Sketch, active: &dyn Fn(&Constraint) -> bool, edit: &EditContext, scale: f64) -> Problem {
    let slots = Slots::new(sk);
    let mut eqs = Vec::new();
    for c in sk.curves.values() {
        compile_internal(c, &slots, &mut eqs);
    }
    for c in sk.constraints.values().filter(|c| active(c) && (c.driving || !c.kind.is_dimension())) {
        compile(c, sk, &slots, scale, &mut eqs);
    }
    let n_hard = eqs.len();
    for (prio, g) in edit.goals.iter().enumerate() {
        goal_eqs(g, sk, &slots, scale, prio, &mut eqs);
    }
    let fixed = fixed_slots(sk, &slots, active);
    let mut winv = vec![1.0; slots.vals.len()];
    for e in &edit.moving {
        for p in sk.entity_points(*e) {
            for s in slots.p(p) {
                winv[s] = 100.0;
            }
        }
        if let EntityRef::Curve(c) = e
            && let Some(&s) = slots.rad.get(c)
        {
            winv[s] = 100.0;
        }
    }
    Problem { slots, eqs, n_hard, fixed, winv }
}

/// Connected component of free variables and the equations that touch them.
struct Cluster {
    /// Slots, ascending.
    cols: Vec<usize>,
    /// Equation indices, hard first then soft, each group in original order.
    eqs: Vec<usize>,
    n_hard: usize,
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Splits into clusters (ordered by smallest slot). Also returns hard equations without free variables.
fn decompose(p: &Problem) -> (Vec<Cluster>, Vec<usize>) {
    let n = p.slots.vals.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut touched = vec![false; n];
    let free_slots = |e: &Eq| -> Vec<usize> {
        e.prim.eval(&p.slots.vals).grad().iter().map(|g| g.0).filter(|&s| !p.fixed[s]).collect()
    };
    let eq_slots: Vec<Vec<usize>> = p.eqs.iter().map(free_slots).collect();
    for s in &eq_slots {
        for &x in s {
            touched[x] = true;
        }
        for w in s.windows(2) {
            let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut by_root: BTreeMap<usize, Cluster> = BTreeMap::new();
    for s in 0..n {
        if touched[s] {
            let r = find(&mut parent, s);
            by_root.entry(r).or_insert_with(|| Cluster { cols: Vec::new(), eqs: Vec::new(), n_hard: 0 }).cols.push(s);
        }
    }
    let mut constant = Vec::new();
    for (i, s) in eq_slots.iter().enumerate() {
        match s.first() {
            Some(&x) => {
                let r = find(&mut parent, x);
                let cl = by_root.get_mut(&r).expect("cluster");
                cl.eqs.push(i);
                if i < p.n_hard {
                    cl.n_hard += 1;
                }
            }
            None if i < p.n_hard => constant.push(i),
            None => {}
        }
    }
    (by_root.into_values().collect(), constant)
}

// ---------------------------------------------------------------------------------------------------------
// Numerical iteration on one cluster

struct Local<'a> {
    p: &'a Problem,
    eqs: &'a [usize],
    col: Vec<usize>,
    winv: Vec<f64>,
    cols: &'a [usize],
}

impl<'a> Local<'a> {
    fn new(p: &'a Problem, cl: &'a Cluster, n_eqs: usize) -> Local<'a> {
        let mut col = vec![usize::MAX; p.slots.vals.len()];
        for (i, &s) in cl.cols.iter().enumerate() {
            col[s] = i;
        }
        let winv = cl.cols.iter().map(|&s| p.winv[s]).collect();
        Local { p, eqs: &cl.eqs[..n_eqs], col, winv, cols: &cl.cols }
    }

    fn eval(&self, vals: &[f64]) -> (Vec<SRow>, Vec<f64>) {
        let mut rows = Vec::with_capacity(self.eqs.len());
        let mut r = Vec::with_capacity(self.eqs.len());
        for &e in self.eqs {
            let row = self.p.eqs[e].prim.eval(vals);
            r.push(row.val);
            rows.push(
                row.grad()
                    .iter()
                    .filter(|g| self.col[g.0] != usize::MAX && g.1 != 0.0)
                    .map(|g| (self.col[g.0], g.1))
                    .collect(),
            );
        }
        (rows, r)
    }

    fn resid(&self, vals: &[f64]) -> Vec<f64> {
        self.eqs.iter().map(|&e| self.p.eqs[e].prim.eval(vals).val).collect()
    }

    fn apply(&self, vals: &mut [f64], base: &[f64], dx: &[f64], alpha: f64) {
        for (i, &s) in self.cols.iter().enumerate() {
            vals[s] = base[s] + alpha * dx[i];
        }
    }
}

fn max_abs<'a>(it: impl IntoIterator<Item = &'a f64>) -> f64 {
    it.into_iter().fold(0.0f64, |m, v| m.max(v.abs()))
}

/// Least-norm Gauss–Newton with backtracking on the kept rows. Returns (converged, iterations).
fn gauss_newton(loc: &Local, vals: &mut [f64], opts: &SolveOptions) -> (bool, usize) {
    let fo = FactorOpts { lambda: 0.0, rel_tol: opts.rank_tol, circuits: false };
    for it in 0..opts.max_iter {
        let (rows, r) = loc.eval(vals);
        let f = factor(&rows, &loc.winv, fo);
        let kept_max = max_abs(f.kept_rows.iter().map(|&k| &r[k]));
        if kept_max <= opts.tol {
            polish(loc, vals, opts);
            return (true, it);
        }
        let dx = f.step(&rows, &r);
        let phi0: f64 = f.kept_rows.iter().map(|&k| r[k] * r[k]).sum();
        let base = vals.to_vec();
        let mut alpha = 1.0;
        let mut accepted = false;
        for _ in 0..30 {
            loc.apply(vals, &base, &dx, alpha);
            let rt = loc.resid(vals);
            let phi: f64 = f.kept_rows.iter().map(|&k| rt[k] * rt[k]).sum();
            if phi < phi0 {
                accepted = true;
                break;
            }
            alpha *= 0.5;
        }
        if !accepted {
            vals.copy_from_slice(&base);
            return (false, it);
        }
    }
    let (rows, r) = loc.eval(vals);
    let f = factor(&rows, &loc.winv, fo);
    (max_abs(f.kept_rows.iter().map(|&k| &r[k])) <= opts.tol, opts.max_iter)
}

/// A few extra full steps after convergence so repeated solves land on the same floating-point fixed point.
fn polish(loc: &Local, vals: &mut [f64], opts: &SolveOptions) {
    let fo = FactorOpts { lambda: 0.0, rel_tol: opts.rank_tol, circuits: false };
    for _ in 0..opts.max_iter {
        let (rows, r) = loc.eval(vals);
        let f = factor(&rows, &loc.winv, fo);
        let before = max_abs(f.kept_rows.iter().map(|&k| &r[k]));
        let dx = f.step(&rows, &r);
        if dx.iter().all(|&d| d == 0.0) {
            return;
        }
        let base = vals.to_vec();
        loc.apply(vals, &base, &dx, 1.0);
        let rt = loc.resid(vals);
        let after = max_abs(f.kept_rows.iter().map(|&k| &rt[k]));
        if after > before {
            vals.copy_from_slice(&base);
            return;
        }
        if after == before {
            return;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Converged,
    Conflict,
    NonConvergent,
}

/// Levenberg–Marquardt on hard rows: minimises ‖r‖² when Gauss–Newton fails. A stationary point with a
/// non-zero residual means the equations are inconsistent (conflict).
fn levenberg_marquardt(loc: &Local, vals: &mut [f64], opts: &SolveOptions) -> (Outcome, usize) {
    let (rows, r) = loc.eval(vals);
    let mut phi: f64 = r.iter().map(|v| v * v).sum();
    let diag_max = rows.iter().map(|row| row.iter().map(|e| e.1 * e.1).sum::<f64>()).fold(0.0f64, f64::max);
    let mut lambda = 1e-3 * diag_max.max(1e-12);
    let mut stalled = 0;
    for it in 0..opts.max_iter * 4 {
        let (rows, r) = loc.eval(vals);
        if max_abs(&r) <= opts.tol {
            return (Outcome::Converged, it);
        }
        let mut g = vec![0.0; loc.cols.len()];
        for (row, rv) in rows.iter().zip(&r) {
            for &(c, v) in row {
                g[c] += v * rv;
            }
        }
        // Stationary when r is (numerically) orthogonal to range(J): relative test, so slow linear
        // convergence towards a singular root (collinear triangle, SK-06) is not mistaken for a conflict.
        let jn: f64 = rows.iter().flat_map(|row| row.iter().map(|e| e.1 * e.1)).sum::<f64>().sqrt();
        let gn: f64 = g.iter().map(|v| v * v).sum::<f64>().sqrt();
        if gn <= 1e-10 * jn * phi.sqrt() {
            return (Outcome::Conflict, it);
        }
        let f = factor(&rows, &loc.winv, FactorOpts { lambda, rel_tol: 0.0, circuits: false });
        let dx = f.step(&rows, &r);
        let base = vals.to_vec();
        loc.apply(vals, &base, &dx, 1.0);
        let rt = loc.resid(vals);
        let phit: f64 = rt.iter().map(|v| v * v).sum();
        if phit < phi {
            stalled = if phi - phit <= 1e-15 * phi { stalled + 1 } else { 0 };
            phi = phit;
            lambda = (lambda / 3.0).max(1e-15);
            if stalled >= 5 {
                return (Outcome::Conflict, it);
            }
        } else {
            vals.copy_from_slice(&base);
            lambda *= 4.0;
            if lambda > 1e15 * diag_max.max(1.0) {
                return (Outcome::Conflict, it);
            }
        }
    }
    (Outcome::NonConvergent, opts.max_iter * 4)
}

fn run_cluster(p: &Problem, cl: &Cluster, vals: &mut [f64], opts: &SolveOptions) -> (Outcome, usize) {
    let start = vals.to_vec();
    let hard_ok = |vals: &[f64]| cl.eqs[..cl.n_hard].iter().all(|&e| p.eqs[e].prim.eval(vals).val.abs() <= opts.tol);
    let mut iters = 0;
    // 1. Hard + soft goals.
    if cl.eqs.len() > cl.n_hard {
        let (conv, it) = gauss_newton(&Local::new(p, cl, cl.eqs.len()), vals, opts);
        iters += it;
        if conv && hard_ok(vals) {
            return (Outcome::Converged, iters);
        }
        vals.copy_from_slice(&start);
    }
    // 2. Hard equations only (pure minimal change).
    let loc = Local::new(p, cl, cl.n_hard);
    let (conv, it) = gauss_newton(&loc, vals, opts);
    iters += it;
    if conv && hard_ok(vals) {
        return (Outcome::Converged, iters);
    }
    // 2b. Kept rows converged but a dropped (nearly dependent) row is still violated: the root is singular
    //     (e.g. collinear triangle, SK-06). Keep near-dependent rows and continue Newton.
    if conv {
        let tight = SolveOptions { rank_tol: 1e-15, ..opts.clone() };
        let (conv, it) = gauss_newton(&loc, vals, &tight);
        iters += it;
        if conv && hard_ok(vals) {
            return (Outcome::Converged, iters);
        }
    }
    // 3. Least squares.
    let (out, it) = levenberg_marquardt(&loc, vals, opts);
    iters += it;
    if out == Outcome::Converged {
        let (_, it) = gauss_newton(&loc, vals, opts);
        iters += it;
        if hard_ok(vals) {
            return (Outcome::Converged, iters);
        }
        return (Outcome::Conflict, iters);
    }
    (out, iters)
}

struct Run {
    p: Problem,
    vals: Vec<f64>,
    clusters: Vec<Cluster>,
    constant: Vec<usize>,
    outcomes: Vec<Outcome>,
    iterations: usize,
    /// Curves that the solve shrank to zero size (lines to a point, circles to radius 0).
    collapsed: Vec<EntityRef>,
}

/// Line length or radius of a curve in `vals`.
fn curve_size(c: &Curve, slots: &Slots, vals: &[f64]) -> f64 {
    match c.geom {
        CurveGeom::Line { a, b } => {
            let (sa, sb) = (slots.p(a), slots.p(b));
            libm::hypot(vals[sb[0]] - vals[sa[0]], vals[sb[1]] - vals[sa[1]])
        }
        _ => vals[slots.r(c.id)],
    }
}

impl Run {
    fn feasible(&self, tol: f64) -> bool {
        self.outcomes.iter().all(|o| *o == Outcome::Converged)
            && self.constant.iter().all(|&e| self.p.eqs[e].prim.eval(&self.vals).val.abs() <= tol)
    }
}

fn run(sk: &Sketch, active: &dyn Fn(&Constraint) -> bool, edit: &EditContext, opts: &SolveOptions, scale: f64) -> Run {
    let r = run_from(sk, active, edit, opts, scale, &[]);
    if r.collapsed.is_empty() {
        return r;
    }
    // Newton cannot see a rotation from an exactly perpendicular start (e.g. Vertical on a horizontal line):
    // the min-norm step shrinks the line instead. Retry once with the collapsed lines nudged by 1e-3 rad and
    // their size kept as a soft goal; keep the retry only if nothing collapses.
    let mut edit2 = edit.clone();
    for e in &r.collapsed {
        if let EntityRef::Curve(c) = *e {
            edit2.goals.push(match sk.curve(c).geom {
                CurveGeom::Line { .. } => Goal::KeepLength { line: c },
                _ => Goal::KeepRadius { curve: c },
            });
        }
    }
    let r2 = run_from(sk, active, &edit2, opts, scale, &r.collapsed);
    if r2.collapsed.is_empty() { r2 } else { r }
}

fn run_from(
    sk: &Sketch,
    active: &dyn Fn(&Constraint) -> bool,
    edit: &EditContext,
    opts: &SolveOptions,
    scale: f64,
    nudge: &[EntityRef],
) -> Run {
    let p = build(sk, active, edit, scale);
    let (clusters, constant) = decompose(&p);
    let mut vals = p.slots.vals.clone();
    for e in nudge {
        if let EntityRef::Curve(c) = *e
            && let Some((a, b)) = sk.curve(c).line_points()
        {
            let (sa, sb) = (p.slots.p(a), p.slots.p(b));
            let (d0, d1) = (vals[sb[0]] - vals[sa[0]], vals[sb[1]] - vals[sa[1]]);
            let (end, sign) = if p.fixed[sb[0]] { (sa, -1.0) } else { (sb, 1.0) };
            vals[end[0]] -= sign * 1e-3 * d1;
            vals[end[1]] += sign * 1e-3 * d0;
        }
    }
    let mut outcomes = Vec::with_capacity(clusters.len());
    let mut iterations = 0;
    for cl in &clusters {
        let (o, it) = run_cluster(&p, cl, &mut vals, opts);
        outcomes.push(o);
        iterations = iterations.max(it);
    }
    // A solve that shrinks a curve to zero size has not really satisfied direction/tangency constraints
    // (they are undefined there): treat it as infeasible so the conflict search can name the cause.
    let eps = 1e-9 * scale;
    let mut collapsed = Vec::new();
    for c in sk.curves.values() {
        if curve_size(c, &p.slots, &vals) <= eps && curve_size(c, &p.slots, &p.slots.vals) > eps {
            collapsed.push(EntityRef::Curve(c.id));
            let slots: Vec<usize> = c.points().iter().flat_map(|&q| p.slots.p(q)).collect();
            for (ci, cl) in clusters.iter().enumerate() {
                if slots.iter().any(|s| cl.cols.binary_search(s).is_ok()) {
                    outcomes[ci] = Outcome::Conflict;
                }
            }
        }
    }
    let mut p = p;
    p.slots.vals.copy_from_slice(&vals);
    Run { p, vals, clusters, constant, outcomes, iterations, collapsed }
}

// ---------------------------------------------------------------------------------------------------------
// Analysis

fn owner_id(e: &Eq) -> Option<ConstraintId> {
    match e.owner {
        Owner::Constraint(id) => Some(id),
        _ => None,
    }
}

/// Fix constraints acting on the entities of the given constraints.
fn related_fixes(sk: &Sketch, ids: &[ConstraintId]) -> Vec<ConstraintId> {
    let mut pts = BTreeSet::new();
    let mut curves = BTreeSet::new();
    for id in ids {
        for e in sk.constraints[id].kind.entities() {
            if let EntityRef::Curve(c) = e {
                curves.insert(c);
            }
            pts.extend(sk.entity_points(e));
        }
    }
    sk.constraints
        .values()
        .filter(|c| match c.kind {
            ConstraintKind::Fix { entity: EntityRef::Point(p) } => pts.contains(&p),
            ConstraintKind::Fix { entity: EntityRef::Curve(cid) } => {
                curves.contains(&cid) || sk.curve(cid).points().iter().any(|p| pts.contains(p))
            }
            _ => false,
        })
        .map(|c| c.id)
        .collect()
}

/// Deletion filter: smallest infeasible subset of `candidates` (irreducible, F11 §2.4).
fn deletion_filter(sk: &Sketch, candidates: &[ConstraintId], opts: &SolveOptions, scale: f64) -> Vec<ConstraintId> {
    let mut set: BTreeSet<ConstraintId> = candidates.iter().copied().collect();
    let quick = SolveOptions { per_entity_dof: false, branch_guard: false, ..opts.clone() };
    for &c in candidates {
        set.remove(&c);
        let r = run(sk, &|k: &Constraint| set.contains(&k.id), &EditContext::default(), &quick, scale);
        if r.feasible(opts.tol) {
            set.insert(c);
        }
    }
    set.into_iter().collect()
}

/// Orientation of every corner (two lines sharing an endpoint): sign of the cross product of the two
/// directions taken from the shared point, or 0 when (nearly) collinear.
fn corner_signs(sk: &Sketch) -> Vec<i8> {
    let mut at: BTreeMap<PointId, Vec<(PointId, PointId)>> = BTreeMap::new();
    for c in sk.curves.values() {
        if let Some((a, b)) = c.line_points() {
            at.entry(a).or_default().push((a, b));
            at.entry(b).or_default().push((b, a));
        }
    }
    let mut out = Vec::new();
    for lines in at.values() {
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                let (o, p1) = (sk.xy(lines[i].0), sk.xy(lines[i].1));
                let p2 = sk.xy(lines[j].1);
                let d1 = (p1.0 - o.0, p1.1 - o.1);
                let d2 = (p2.0 - o.0, p2.1 - o.1);
                let n = libm::hypot(d1.0, d1.1) * libm::hypot(d2.0, d2.1);
                let s = if n == 0.0 { 0.0 } else { (d1.0 * d2.1 - d1.1 * d2.0) / n };
                out.push(if s > 1e-6 {
                    1
                } else if s < -1e-6 {
                    -1
                } else {
                    0
                });
            }
        }
    }
    out
}

fn flipped(before: &[i8], after: &[i8]) -> bool {
    before.iter().zip(after).any(|(a, b)| *a != 0 && *b == -*a)
}

/// Degeneracy guard (F11 §2.6): zero-length lines / non-positive radii are invalid; folded corners warn.
fn degeneracy(sk: &Sketch, scale: f64) -> Vec<Diagnostic> {
    let eps = 1e-9 * scale;
    let mut invalid = Vec::new();
    let mut folded = Vec::new();
    for c in sk.curves.values() {
        match c.geom {
            CurveGeom::Line { a, b } => {
                let (pa, pb) = (sk.xy(a), sk.xy(b));
                if libm::hypot(pb.0 - pa.0, pb.1 - pa.1) <= eps {
                    invalid.push(EntityRef::Curve(c.id));
                }
            }
            CurveGeom::Circle { r, .. } | CurveGeom::Arc { r, .. } => {
                if r <= eps {
                    invalid.push(EntityRef::Curve(c.id));
                }
            }
        }
    }
    let lines: Vec<(CurveId, PointId, PointId)> =
        sk.curves.values().filter_map(|c| c.line_points().map(|(a, b)| (c.id, a, b))).collect();
    for (i, &(c1, a1, b1)) in lines.iter().enumerate() {
        for &(c2, a2, b2) in &lines[i + 1..] {
            // Shared end point o and the two far ends p, q.
            let (o, p, q) = if a1 == a2 {
                (a1, b1, b2)
            } else if a1 == b2 {
                (a1, b1, a2)
            } else if b1 == a2 {
                (b1, a1, b2)
            } else if b1 == b2 {
                (b1, a1, a2)
            } else {
                continue;
            };
            let (po, pp, pq) = (sk.xy(o), sk.xy(p), sk.xy(q));
            let d1 = (pp.0 - po.0, pp.1 - po.1);
            let d2 = (pq.0 - po.0, pq.1 - po.1);
            let n = libm::hypot(d1.0, d1.1) * libm::hypot(d2.0, d2.1);
            // Folded: both lines leave the shared point in the same direction (zero angle between them).
            if n > 0.0 && ((d1.0 * d2.1 - d1.1 * d2.0) / n).abs() <= 1e-6 && d1.0 * d2.0 + d1.1 * d2.1 > 0.0 {
                folded.push(EntityRef::Curve(c1));
                folded.push(EntityRef::Curve(c2));
            }
        }
    }
    let mut out = Vec::new();
    if !invalid.is_empty() {
        out.push(Diagnostic::new(Code::DimInvalidValue).with_entities(invalid));
    }
    if !folded.is_empty() {
        out.push(Diagnostic::new(Code::GeometryDegenerate).with_entities(folded));
    }
    out
}

/// Driving dimensions moved from their measured start value towards the target in `steps` substeps.
fn continuation(sk: &Sketch, edit: &EditContext, opts: &SolveOptions, scale: f64, steps: usize) -> Option<Sketch> {
    let start: BTreeMap<ConstraintId, f64> =
        sk.constraints.values().filter(|c| c.driving).filter_map(|c| measure(&c.kind, sk).map(|v| (c.id, v))).collect();
    let mut cur = sk.clone();
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let mut stage = cur.clone();
        for (id, v0) in &start {
            let c = stage.constraints.get_mut(id).expect("constraint");
            let v1 = sk.constraints[id].kind.value().expect("dimension");
            c.kind.set_value(if i == steps { v1 } else { v0 + t * (v1 - v0) });
        }
        let r = run(&stage, &|_| true, edit, opts, scale);
        if !r.feasible(opts.tol) {
            return None;
        }
        r.p.slots.store(&mut stage);
        stage.constraints = sk.constraints.clone();
        cur = stage;
    }
    Some(cur)
}

/// Solves the sketch starting from its stored coordinates (N.2a-5).
pub fn solve(sk: &Sketch, edit: &EditContext, opts: &SolveOptions) -> SolveResult {
    let scale = length_scale(sk);
    let mut r = run(sk, &|_| true, edit, opts, scale);
    let mut solved = sk.clone();
    r.p.slots.store(&mut solved);
    if opts.branch_guard && r.feasible(opts.tol) {
        let before = corner_signs(sk);
        if flipped(&before, &corner_signs(&solved)) {
            for steps in [2, 4, 8, 16] {
                if let Some(s) = continuation(sk, edit, opts, scale, steps)
                    && !flipped(&before, &corner_signs(&s))
                {
                    // Re-run from the continuation result so the analysis below sees its state.
                    r = run(&s, &|_| true, &EditContext::default(), opts, scale);
                    solved = s;
                    r.p.slots.store(&mut solved);
                    break;
                }
            }
        }
    }
    analyze(sk, solved, r, opts, scale)
}

fn analyze(orig: &Sketch, mut solved: Sketch, r: Run, opts: &SolveOptions, scale: f64) -> SolveResult {
    let p = &r.p;
    let vals = &r.vals;
    let mut redundant: Vec<(ConstraintId, Vec<ConstraintId>)> = Vec::new();
    let mut conflict_sets: Vec<Vec<ConstraintId>> = Vec::new();
    let mut nonconvergent = false;
    let mut dof = 0;
    let mut entity_dof: BTreeMap<EntityRef, usize> = BTreeMap::new();
    let entities: Vec<(EntityRef, Vec<usize>)> = orig
        .points
        .keys()
        .map(|&id| (EntityRef::Point(id), p.slots.p(id).to_vec()))
        .chain(orig.curves.values().map(|c| {
            let mut s: Vec<usize> = c.points().iter().flat_map(|&q| p.slots.p(q)).collect();
            s.extend(p.slots.rad.get(&c.id));
            (EntityRef::Curve(c.id), s)
        }))
        .collect();
    let mut cluster_of = vec![usize::MAX; vals.len()];
    for (ci, cl) in r.clusters.iter().enumerate() {
        for &s in &cl.cols {
            cluster_of[s] = ci;
        }
    }
    // Free slots no equation touches are plain DOF.
    for s in 0..vals.len() {
        if !p.fixed[s] && cluster_of[s] == usize::MAX {
            dof += 1;
        }
    }
    if opts.per_entity_dof {
        for (e, slots) in &entities {
            let n = slots.iter().filter(|&&s| !p.fixed[s] && cluster_of[s] == usize::MAX).count();
            entity_dof.insert(*e, n);
        }
    }
    for (ci, cl) in r.clusters.iter().enumerate() {
        let hard = Cluster { cols: cl.cols.clone(), eqs: cl.eqs[..cl.n_hard].to_vec(), n_hard: cl.n_hard };
        let mut loc = Local::new(p, &hard, hard.n_hard);
        loc.winv = vec![1.0; hard.cols.len()];
        let (rows, res) = loc.eval(vals);
        let f = factor(&rows, &loc.winv, FactorOpts { lambda: 0.0, rel_tol: opts.rank_tol, circuits: true });
        dof += hard.cols.len() - f.rank();
        if r.outcomes[ci] == Outcome::NonConvergent {
            nonconvergent = true;
        }
        for (row, members) in &f.circuits {
            let id = owner_id(&p.eqs[hard.eqs[*row]]);
            let mut set: Vec<ConstraintId> = members.iter().filter_map(|&m| owner_id(&p.eqs[hard.eqs[m]])).collect();
            if let Some(id) = id {
                set.push(id);
            }
            set.sort();
            set.dedup();
            if res[*row].abs() <= opts.tol {
                if let Some(id) = id
                    && !redundant.iter().any(|(r, _)| *r == id)
                {
                    redundant.push((id, set));
                }
            } else {
                conflict_sets.push(set);
            }
        }
        if r.outcomes[ci] == Outcome::Conflict && conflict_sets.is_empty() {
            // Nonlinear inconsistency without a dropped row: blame the rows that stay violated.
            let mut set: Vec<ConstraintId> = hard
                .eqs
                .iter()
                .zip(&res)
                .filter(|(_, v)| v.abs() > opts.tol)
                .filter_map(|(&e, _)| owner_id(&p.eqs[e]))
                .collect();
            set.sort();
            set.dedup();
            conflict_sets.push(set);
        }
        if opts.per_entity_dof {
            for (e, slots) in &entities {
                let local: Vec<usize> =
                    slots.iter().filter(|&&s| cluster_of[s] == ci && !p.fixed[s]).map(|&s| loc.col[s]).collect();
                if !local.is_empty() {
                    *entity_dof.entry(*e).or_default() += f.free_dims(&local);
                }
            }
        }
    }
    for &e in &r.constant {
        let mut set: Vec<ConstraintId> = owner_id(&p.eqs[e]).into_iter().collect();
        let fixes = related_fixes(orig, &set);
        let id = set.first().copied();
        set.extend(fixes);
        set.sort();
        set.dedup();
        if p.eqs[e].prim.eval(vals).val.abs() <= opts.tol {
            if let Some(id) = id
                && !redundant.iter().any(|(r, _)| *r == id)
            {
                redundant.push((id, set));
            }
        } else {
            conflict_sets.push(set);
        }
    }

    // Smallest conflicting set: deletion filter over the involved clusters when small enough.
    let mut conflicts: Vec<ConstraintId> = Vec::new();
    if !conflict_sets.is_empty() {
        let mut seeds: BTreeSet<ConstraintId> = conflict_sets.iter().flatten().copied().collect();
        let mut candidates: BTreeSet<ConstraintId> = BTreeSet::new();
        for (ci, cl) in r.clusters.iter().enumerate() {
            let ids: BTreeSet<ConstraintId> = cl.eqs[..cl.n_hard].iter().filter_map(|&e| owner_id(&p.eqs[e])).collect();
            if r.outcomes[ci] == Outcome::Conflict || ids.iter().any(|id| seeds.contains(id)) {
                candidates.extend(ids);
            }
        }
        candidates.extend(seeds.iter().copied());
        let fixes = related_fixes(orig, &candidates.iter().copied().collect::<Vec<_>>());
        candidates.extend(fixes);
        let candidates_all: Vec<ConstraintId> = candidates.iter().copied().collect();
        if candidates.len() <= opts.conflict_search_limit {
            let cand: Vec<ConstraintId> = candidates.into_iter().collect();
            let iis = deletion_filter(orig, &cand, opts, scale);
            if !iis.is_empty() {
                seeds = iis.into_iter().collect();
            }
        }
        conflicts = seeds.into_iter().collect();
        if conflicts.is_empty() {
            conflicts = candidates_all;
        }
    }

    r.p.slots.store(&mut solved);
    for c in solved.constraints.clone().values() {
        if !c.driving && c.kind.is_dimension() {
            let v = measure(&c.kind, &solved).expect("dimension");
            solved.constraints.get_mut(&c.id).unwrap().kind.set_value(v);
        }
    }
    let feasible = conflicts.is_empty() && !nonconvergent;
    let mut diagnostics = Vec::new();
    let degenerate = if feasible { degeneracy(&solved, scale) } else { Vec::new() };
    let has_degenerate = degenerate.iter().any(|d| d.code == Code::GeometryDegenerate);
    if has_degenerate || !r.collapsed.is_empty() {
        // Rank loss at a degenerate configuration is explained by the degeneracy, not by redundancy.
        redundant.clear();
    }
    if !conflicts.is_empty() {
        diagnostics.push(Diagnostic::new(Code::SolveConflict).with_constraints(conflicts.clone()));
        if !r.collapsed.is_empty() {
            diagnostics.push(Diagnostic::new(Code::DimInvalidValue).with_entities(r.collapsed.clone()));
        }
    } else if nonconvergent {
        diagnostics.push(Diagnostic::new(Code::SolveNonconvergent));
    }
    for (_, set) in &redundant {
        diagnostics.push(
            Diagnostic::new(Code::ConstraintRedundant)
                .with_constraints(set.clone())
                .with_suggestion(Suggestion::MakeDriven),
        );
    }
    diagnostics.extend(degenerate);
    let status = if !conflicts.is_empty() {
        Status::Conflict
    } else if nonconvergent {
        Status::NonConvergent
    } else if !redundant.is_empty() {
        Status::Redundant
    } else if dof == 0 {
        Status::FullyConstrained
    } else {
        Status::UnderConstrained
    };
    let residual = (0..p.n_hard).map(|e| p.eqs[e].prim.eval(vals).val.abs()).fold(0.0, f64::max);
    SolveResult {
        status,
        sketch: solved,
        dof,
        entity_dof,
        redundant,
        conflicts,
        diagnostics,
        iterations: r.iterations,
        residual,
    }
}

// ---------------------------------------------------------------------------------------------------------
// Movement policy (F.3 table, F61–F65): which entity stays, which moves and how.

impl EditContext {
    /// Context for applying constraint `kind` to existing geometry: the first argument stays, the second
    /// moves as described in F.3 (translate / rotate about midpoint keeping length / keep start point).
    pub fn for_constraint(kind: &ConstraintKind, sk: &Sketch) -> EditContext {
        use ConstraintKind::*;
        let mut g: Vec<Goal> = Vec::new();
        let mut moving: Vec<EntityRef> = Vec::new();
        let anchor_pts = |pts: &[PointId], g: &mut Vec<Goal>| {
            for &p in pts {
                g.push(Goal::KeepPoint { p });
            }
        };
        let anchor_curve = |c: CurveId, g: &mut Vec<Goal>| {
            for p in sk.curve(c).points() {
                g.push(Goal::KeepPoint { p });
            }
            if sk.curve(c).radius().is_some() {
                g.push(Goal::KeepRadius { curve: c });
            }
        };
        // Rigid translation of a curve.
        let translate = |c: CurveId, g: &mut Vec<Goal>| {
            let cv = sk.curve(c);
            let pts = cv.points();
            for &q in &pts[1..] {
                g.push(Goal::KeepOffset { from: pts[0], to: q });
            }
            if cv.radius().is_some() {
                g.push(Goal::KeepRadius { curve: c });
            }
        };
        // Rotation keeping length: about a point shared with anchored geometry, else about the midpoint.
        let rotate = |l: CurveId, anchored: &BTreeSet<PointId>, g: &mut Vec<Goal>| {
            let (a, b) = sk.curve(l).line_points().expect("line");
            if !anchored.contains(&a) && !anchored.contains(&b) {
                g.push(Goal::KeepMidpoint { line: l });
            }
            g.push(Goal::KeepLength { line: l });
        };
        let curve_pts = |c: CurveId| -> BTreeSet<PointId> { sk.curve(c).points().into_iter().collect() };
        // Point `b` moves with its owning curves translating; point `a` stays.
        let point_pair = |a: PointId, b: PointId, g: &mut Vec<Goal>, moving: &mut Vec<EntityRef>| {
            anchor_pts(&[a], g);
            let owners = sk.curves_of_point(b);
            let same: Vec<CurveId> = owners.iter().copied().filter(|c| sk.curve(*c).points().contains(&a)).collect();
            if same.is_empty() {
                for c in &owners {
                    translate(*c, g);
                    moving.push(EntityRef::Curve(*c));
                }
            } else {
                for c in same {
                    if sk.curve(c).line_points().is_some() {
                        g.push(Goal::KeepDirection { line: c });
                    }
                }
            }
            moving.push(EntityRef::Point(b));
        };
        match *kind {
            Coincident { a, b } | Distance { a, b, .. } | DistanceH { a, b, .. } | DistanceV { a, b, .. } => {
                point_pair(a, b, &mut g, &mut moving)
            }
            PointOnLine { p, line } | Midpoint { p, line } | DistancePointLine { p, line, .. } => {
                anchor_pts(&[p], &mut g);
                translate(line, &mut g);
                moving.push(EntityRef::Curve(line));
            }
            PointOnCircle { p, curve } => {
                anchor_pts(&[p], &mut g);
                translate(curve, &mut g);
                moving.push(EntityRef::Curve(curve));
            }
            Horizontal { line } | Vertical { line } => {
                rotate(line, &BTreeSet::new(), &mut g);
                moving.push(EntityRef::Curve(line));
            }
            Parallel { l1, l2 } | Perpendicular { l1, l2 } | Angle { l1, l2, .. } => {
                anchor_curve(l1, &mut g);
                rotate(l2, &curve_pts(l1), &mut g);
                moving.push(EntityRef::Curve(l2));
            }
            Collinear { l1, l2 } => {
                anchor_curve(l1, &mut g);
                // An end already on l1's extension stays: l2 rotates about it (F109).
                let (a, b) = sk.curve(l2).line_points().expect("line");
                let on_l1 = |p: PointId| signed_dist(sk, p, l1).abs() <= 1e-9 * length_scale(sk);
                for p in [a, b] {
                    if on_l1(p) {
                        g.push(Goal::KeepPoint { p });
                        break;
                    }
                }
                g.push(Goal::KeepLength { line: l2 });
                moving.push(EntityRef::Curve(l2));
            }
            EqualLength { l1, l2 } => {
                anchor_curve(l1, &mut g);
                let (a, _) = sk.curve(l2).line_points().expect("line");
                anchor_pts(&[a], &mut g);
                g.push(Goal::KeepDirection { line: l2 });
                moving.push(EntityRef::Curve(l2));
            }
            EqualRadius { c1, c2 } => {
                anchor_curve(c1, &mut g);
                anchor_pts(&sk.curve(c2).points(), &mut g);
                moving.push(EntityRef::Curve(c2));
            }
            Concentric { c1, c2 } | TangentCurves { c1, c2, .. } => {
                anchor_curve(c1, &mut g);
                translate(c2, &mut g);
                moving.push(EntityRef::Curve(c2));
            }
            Tangent { line, curve, .. } => {
                anchor_curve(curve, &mut g);
                translate(line, &mut g);
                moving.push(EntityRef::Curve(line));
            }
            Symmetric { p1, p2, axis } => {
                anchor_curve(axis, &mut g);
                anchor_pts(&[p1], &mut g);
                moving.push(EntityRef::Point(p2));
            }
            SymmetricLines { l1, l2, axis } => {
                anchor_curve(axis, &mut g);
                anchor_curve(l1, &mut g);
                moving.push(EntityRef::Curve(l2));
            }
            Radius { curve, .. } | Diameter { curve, .. } => {
                anchor_pts(&[sk.curve(curve).center().expect("round")], &mut g);
                moving.push(EntityRef::Curve(curve));
            }
            Fix { .. } => {}
        }
        EditContext { goals: g, moving }
    }

    /// Context for dragging a point to (x, y) (F231): the target is the only goal.
    pub fn for_drag(p: PointId, x: f64, y: f64) -> EditContext {
        EditContext { goals: vec![Goal::Target { p, x, y }], moving: vec![EntityRef::Point(p)] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(sk: &mut Sketch, id: u64, x: f64, y: f64) -> PointId {
        let id = PointId(id);
        sk.points.insert(id, Point { id, x, y, prov: Provenance::default() });
        id
    }

    fn line(sk: &mut Sketch, id: u64, a: PointId, b: PointId) -> CurveId {
        let id = CurveId(id);
        sk.curves
            .insert(id, Curve { id, geom: CurveGeom::Line { a, b }, construction: false, prov: Provenance::default() });
        id
    }

    fn con(sk: &mut Sketch, id: u64, kind: ConstraintKind) -> ConstraintId {
        let id = ConstraintId(id);
        sk.constraints.insert(id, Constraint { id, kind, driving: true, prov: Provenance::default() });
        id
    }

    /// Rectangle from the fixed origin: 4 points, 4 lines, 2H + 2V, W and H dimensions.
    fn rectangle(w: f64, h: f64) -> Sketch {
        use ConstraintKind::*;
        let mut sk = Sketch::default();
        let o = pt(&mut sk, 1, 0.0, 0.0);
        let p1 = pt(&mut sk, 2, 0.0, 0.0);
        let p2 = pt(&mut sk, 3, 31.0, 0.5);
        let p3 = pt(&mut sk, 4, 30.0, 17.0);
        let p4 = pt(&mut sk, 5, -0.4, 18.0);
        let b = line(&mut sk, 10, p1, p2);
        let r = line(&mut sk, 11, p2, p3);
        let t = line(&mut sk, 12, p3, p4);
        let l = line(&mut sk, 13, p4, p1);
        con(&mut sk, 20, Fix { entity: EntityRef::Point(o) });
        con(&mut sk, 21, Coincident { a: o, b: p1 });
        con(&mut sk, 22, Horizontal { line: b });
        con(&mut sk, 23, Vertical { line: r });
        con(&mut sk, 24, Horizontal { line: t });
        con(&mut sk, 25, Vertical { line: l });
        con(&mut sk, 26, Distance { a: p1, b: p2, value: w });
        con(&mut sk, 27, Distance { a: p2, b: p3, value: h });
        sk
    }

    #[test]
    fn rectangle_is_fully_constrained() {
        let r = solve(&rectangle(40.0, 20.0), &EditContext::default(), &SolveOptions::default());
        assert_eq!(r.status, Status::FullyConstrained, "{:?}", r.diagnostics);
        assert_eq!(r.dof, 0);
        assert_eq!(r.sketch.xy(PointId(4)), (40.0, 20.0));
        assert!(r.entity_dof.values().all(|&d| d == 0), "{:?}", r.entity_dof);
    }

    #[test]
    fn redundant_dimension_names_minimal_set() {
        let mut sk = rectangle(40.0, 20.0);
        sk = solve(&sk, &EditContext::default(), &SolveOptions::default()).sketch;
        con(&mut sk, 28, ConstraintKind::Distance { a: PointId(4), b: PointId(5), value: 40.0 });
        let r = solve(&sk, &EditContext::default(), &SolveOptions::default());
        assert_eq!(r.status, Status::Redundant);
        assert_eq!(r.redundant.len(), 1);
        assert_eq!(r.redundant[0].0, ConstraintId(28));
        // Top width = bottom width through H/V of the four sides.
        let set: Vec<u64> = r.redundant[0].1.iter().map(|c| c.0).collect();
        assert!(set.contains(&26) && set.contains(&28), "{set:?}");
    }

    #[test]
    fn horizontal_on_fixed_oblique_line_conflicts() {
        let mut sk = Sketch::default();
        let a = pt(&mut sk, 1, 0.0, 0.0);
        let b = pt(&mut sk, 2, 10.0, 3.0);
        let l = line(&mut sk, 3, a, b);
        con(&mut sk, 4, ConstraintKind::Fix { entity: EntityRef::Curve(l) });
        con(&mut sk, 5, ConstraintKind::Horizontal { line: l });
        let r = solve(&sk, &EditContext::default(), &SolveOptions::default());
        assert_eq!(r.status, Status::Conflict);
        assert_eq!(r.conflicts, vec![ConstraintId(4), ConstraintId(5)]);
    }

    #[test]
    fn horizontal_rotates_about_midpoint_keeping_length() {
        let mut sk = Sketch::default();
        let a = pt(&mut sk, 1, 0.0, 0.0);
        let b = pt(&mut sk, 2, 8.0, 6.0);
        let l = line(&mut sk, 3, a, b);
        let k = ConstraintKind::Horizontal { line: l };
        con(&mut sk, 4, k.clone());
        let r = solve(&sk, &EditContext::for_constraint(&k, &sk), &SolveOptions::default());
        let (pa, pb) = (r.sketch.xy(a), r.sketch.xy(b));
        assert!((pa.1 - 3.0).abs() < 1e-9 && (pb.1 - 3.0).abs() < 1e-9, "{pa:?} {pb:?}");
        assert!((pb.0 - pa.0 - 10.0).abs() < 1e-9);
        assert!((pa.0 + pb.0 - 8.0).abs() < 1e-9);
        assert_eq!(r.dof, 3);
    }
}
