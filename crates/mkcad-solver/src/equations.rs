//! Residual equations with analytic Jacobian rows (F11 §1).
//!
//! Every constraint compiles into one or more [`Prim`] equations over *slots* — indices into a flat value
//! vector holding all point coordinates and radii (fixed or not). The solver later maps free slots to
//! Jacobian columns. Dimensionless rows (parallel, perpendicular, angle, direction) are multiplied by the
//! sketch length scale so one residual tolerance in mm applies to all rows.

use crate::model::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Slot pair (x, y) of a point.
pub type P = [usize; 2];

/// Flat variable layout: every point gets two slots, every circle/arc one radius slot, in ID order.
#[derive(Clone, Debug, Default)]
pub struct Slots {
    pub pt: BTreeMap<PointId, usize>,
    pub rad: BTreeMap<CurveId, usize>,
    pub vals: Vec<f64>,
}

impl Slots {
    pub fn new(sk: &Sketch) -> Slots {
        let mut s = Slots::default();
        for p in sk.points.values() {
            s.pt.insert(p.id, s.vals.len());
            s.vals.push(p.x);
            s.vals.push(p.y);
        }
        for c in sk.curves.values() {
            if let Some(r) = c.radius() {
                s.rad.insert(c.id, s.vals.len());
                s.vals.push(r);
            }
        }
        s
    }

    pub fn p(&self, id: PointId) -> P {
        let b = self.pt[&id];
        [b, b + 1]
    }

    pub fn r(&self, id: CurveId) -> usize {
        self.rad[&id]
    }

    /// Writes the values back into a sketch with the same entities.
    pub fn store(&self, sk: &mut Sketch) {
        for p in sk.points.values_mut() {
            let b = self.pt[&p.id];
            p.x = self.vals[b];
            p.y = self.vals[b + 1];
        }
        for c in sk.curves.values_mut() {
            if let Some(&s) = self.rad.get(&c.id) {
                c.set_radius(self.vals[s]);
            }
        }
    }
}

/// Characteristic length of a sketch (bounding-box diagonal, at least 1 mm) used to scale
/// dimensionless rows. Deterministic: depends only on the coordinates.
pub fn length_scale(sk: &Sketch) -> f64 {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in sk.points.values() {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    for c in sk.curves.values() {
        if let Some(r) = c.radius() {
            x1 = x1.max(x0 + 2.0 * r.abs());
        }
    }
    if x0 > x1 {
        return 1.0;
    }
    libm::hypot(x1 - x0, y1 - y0).max(1.0)
}

/// Up to 8 (slot, ∂r/∂slot) entries; duplicate slots are merged.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub val: f64,
    pub n: usize,
    pub g: [(usize, f64); 8],
}

impl Row {
    fn new(val: f64) -> Row {
        Row { val, n: 0, g: [(0, 0.0); 8] }
    }

    fn add(&mut self, slot: usize, d: f64) {
        for e in &mut self.g[..self.n] {
            if e.0 == slot {
                e.1 += d;
                return;
            }
        }
        self.g[self.n] = (slot, d);
        self.n += 1;
    }

    fn add_p(&mut self, p: P, d: [f64; 2]) {
        self.add(p[0], d[0]);
        self.add(p[1], d[1]);
    }

    pub fn grad(&self) -> &[(usize, f64)] {
        &self.g[..self.n]
    }
}

/// Equation primitives. Notation: `d = B − A`, `|d|` its length, `cross(u, v) = ux·vy − uy·vx`.
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// `Σ cᵢ·v[sᵢ] − k`
    Lin { terms: Vec<(usize, f64)>, k: f64 },
    /// `|B − A| − Σ cᵢ·v[rᵢ] − k`
    Dist { a: P, b: P, r: Vec<(usize, f64)>, k: f64 },
    /// Signed distance of `p` from line AB (positive on the left of A→B) `− c·v[r] − k`.
    LineDist { p: P, a: P, b: P, r: Option<(usize, f64)>, k: f64 },
    /// `s·cross(d1, d2)/(|d1||d2|)`
    Cross { l1: [P; 2], l2: [P; 2], s: f64 },
    /// `s·dot(d1, d2)/(|d1||d2|)`
    Dot { l1: [P; 2], l2: [P; 2], s: f64 },
    /// `|d1| − |d2|`
    LenDiff { l1: [P; 2], l2: [P; 2] },
    /// `s·wrap(atan2(cross(d1,d2), dot(d1,d2)) − θ)`, θ in radians.
    Angle { l1: [P; 2], l2: [P; 2], theta: f64, s: f64 },
    /// Signed distance of the midpoint of PQ from the axis AB.
    SymMid { p: P, q: P, a: P, b: P },
    /// `dot(Q − P, B − A)/|B − A|`
    SymPerp { p: P, q: P, a: P, b: P },
    /// `s·cross(B − A, u)/|B − A|` for a constant unit vector `u` (keeps a direction).
    CrossConst { a: P, b: P, u: [f64; 2], s: f64 },
}

fn get(v: &[f64], p: P) -> [f64; 2] {
    [v[p[0]], v[p[1]]]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn norm(a: [f64; 2]) -> f64 {
    libm::hypot(a[0], a[1])
}

fn neg(a: [f64; 2]) -> [f64; 2] {
    [-a[0], -a[1]]
}

fn scale(a: [f64; 2], k: f64) -> [f64; 2] {
    [a[0] * k, a[1] * k]
}

/// Wraps an angle to (−π, π].
pub fn wrap(mut a: f64) -> f64 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a <= -PI {
        a += 2.0 * PI;
    }
    a
}

/// Gradients of `f(d1, d2)` pushed onto both lines' endpoints (`d = B − A`).
fn add_lines(row: &mut Row, l1: [P; 2], g1: [f64; 2], l2: [P; 2], g2: [f64; 2]) {
    row.add_p(l1[1], g1);
    row.add_p(l1[0], neg(g1));
    row.add_p(l2[1], g2);
    row.add_p(l2[0], neg(g2));
}

/// Value and gradient of the signed distance of `p` from line AB, w.r.t. p, A, B.
fn line_dist(v: &[f64], p: P, a: P, b: P) -> (f64, [f64; 2], [f64; 2], [f64; 2]) {
    let (pv, av, bv) = (get(v, p), get(v, a), get(v, b));
    let d = sub(bv, av);
    let w = sub(pv, av);
    let l = norm(d);
    if l == 0.0 {
        return (norm(w), [0.0; 2], [0.0; 2], [0.0; 2]);
    }
    let c = cross(d, w);
    let f = c / l;
    let dp = [-d[1] / l, d[0] / l];
    let l3 = l * l * l;
    let dd = [w[1] / l - c * d[0] / l3, -w[0] / l - c * d[1] / l3];
    let da = [-dd[0] - dp[0], -dd[1] - dp[1]];
    (f, dp, da, dd)
}

impl Prim {
    /// Residual value and Jacobian row at `v`.
    pub fn eval(&self, v: &[f64]) -> Row {
        match self {
            Prim::Lin { terms, k } => {
                let mut row = Row::new(-k);
                for &(s, c) in terms {
                    row.val += c * v[s];
                    row.add(s, c);
                }
                row
            }
            Prim::Dist { a, b, r, k } => {
                let d = sub(get(v, *b), get(v, *a));
                let n = norm(d);
                let mut row = Row::new(n - k);
                let u = if n == 0.0 { [0.0; 2] } else { scale(d, 1.0 / n) };
                row.add_p(*b, u);
                row.add_p(*a, neg(u));
                for &(s, c) in r {
                    row.val -= c * v[s];
                    row.add(s, -c);
                }
                row
            }
            Prim::LineDist { p, a, b, r, k } => {
                let (f, dp, da, db) = line_dist(v, *p, *a, *b);
                let mut row = Row::new(f - k);
                row.add_p(*p, dp);
                row.add_p(*a, da);
                row.add_p(*b, db);
                if let Some((s, c)) = *r {
                    row.val -= c * v[s];
                    row.add(s, -c);
                }
                row
            }
            Prim::Cross { l1, l2, s } | Prim::Dot { l1, l2, s } => {
                let d1 = sub(get(v, l1[1]), get(v, l1[0]));
                let d2 = sub(get(v, l2[1]), get(v, l2[0]));
                let (n1, n2) = (norm(d1), norm(d2));
                if n1 == 0.0 || n2 == 0.0 {
                    return Row::new(0.0);
                }
                let is_cross = matches!(self, Prim::Cross { .. });
                let (raw, dr1, dr2) =
                    if is_cross { (cross(d1, d2), [d2[1], -d2[0]], [-d1[1], d1[0]]) } else { (dot(d1, d2), d2, d1) };
                let nn = n1 * n2;
                let f = raw / nn;
                let g1 = [s * (dr1[0] / nn - f * d1[0] / (n1 * n1)), s * (dr1[1] / nn - f * d1[1] / (n1 * n1))];
                let g2 = [s * (dr2[0] / nn - f * d2[0] / (n2 * n2)), s * (dr2[1] / nn - f * d2[1] / (n2 * n2))];
                let mut row = Row::new(s * f);
                add_lines(&mut row, *l1, g1, *l2, g2);
                row
            }
            Prim::LenDiff { l1, l2 } => {
                let d1 = sub(get(v, l1[1]), get(v, l1[0]));
                let d2 = sub(get(v, l2[1]), get(v, l2[0]));
                let (n1, n2) = (norm(d1), norm(d2));
                let u1 = if n1 == 0.0 { [0.0; 2] } else { scale(d1, 1.0 / n1) };
                let u2 = if n2 == 0.0 { [0.0; 2] } else { scale(d2, -1.0 / n2) };
                let mut row = Row::new(n1 - n2);
                add_lines(&mut row, *l1, u1, *l2, u2);
                row
            }
            Prim::Angle { l1, l2, theta, s } => {
                let d1 = sub(get(v, l1[1]), get(v, l1[0]));
                let d2 = sub(get(v, l2[1]), get(v, l2[0]));
                let c = cross(d1, d2);
                let d = dot(d1, d2);
                let n = c * c + d * d;
                let mut row = Row::new(s * wrap(libm::atan2(c, d) - theta));
                if n == 0.0 {
                    return row;
                }
                let g1 = [s * (d * d2[1] - c * d2[0]) / n, s * (-d * d2[0] - c * d2[1]) / n];
                let g2 = [s * (-d * d1[1] - c * d1[0]) / n, s * (d * d1[0] - c * d1[1]) / n];
                add_lines(&mut row, *l1, g1, *l2, g2);
                row
            }
            Prim::SymMid { p, q, a, b } => {
                let (pv, qv) = (get(v, *p), get(v, *q));
                let m = [0.5 * (pv[0] + qv[0]), 0.5 * (pv[1] + qv[1])];
                let (av, bv) = (get(v, *a), get(v, *b));
                let d = sub(bv, av);
                let w = sub(m, av);
                let l = norm(d);
                if l == 0.0 {
                    return Row::new(0.0);
                }
                let c = cross(d, w);
                let f = c / l;
                let dm = [-d[1] / l, d[0] / l];
                let l3 = l * l * l;
                let dd = [w[1] / l - c * d[0] / l3, -w[0] / l - c * d[1] / l3];
                let mut row = Row::new(f);
                row.add_p(*p, scale(dm, 0.5));
                row.add_p(*q, scale(dm, 0.5));
                row.add_p(*b, dd);
                row.add_p(*a, [-dd[0] - dm[0], -dd[1] - dm[1]]);
                row
            }
            Prim::SymPerp { p, q, a, b } => {
                let w = sub(get(v, *q), get(v, *p));
                let d = sub(get(v, *b), get(v, *a));
                let l = norm(d);
                if l == 0.0 {
                    return Row::new(0.0);
                }
                let f = dot(w, d) / l;
                let gd = [w[0] / l - f * d[0] / (l * l), w[1] / l - f * d[1] / (l * l)];
                let mut row = Row::new(f);
                row.add_p(*q, scale(d, 1.0 / l));
                row.add_p(*p, scale(d, -1.0 / l));
                row.add_p(*b, gd);
                row.add_p(*a, neg(gd));
                row
            }
            Prim::CrossConst { a, b, u, s } => {
                let d = sub(get(v, *b), get(v, *a));
                let l = norm(d);
                if l == 0.0 {
                    return Row::new(0.0);
                }
                let c = cross(d, *u);
                let l3 = l * l * l;
                let g = [s * (u[1] / l - c * d[0] / l3), s * (-u[0] / l - c * d[1] / l3)];
                let mut row = Row::new(s * c / l);
                row.add_p(*b, g);
                row.add_p(*a, neg(g));
                row
            }
        }
    }
}

/// Who an equation belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Owner {
    Constraint(ConstraintId),
    /// Built-in arc equations (start/end on the arc's circle).
    Internal(CurveId),
    /// Edit-context preference (minimal-change policy), by priority; never reported as a conflict.
    Soft(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Eq {
    pub prim: Prim,
    pub owner: Owner,
}

fn sign_of(x: f64) -> Sign {
    if x < 0.0 { -1 } else { 1 }
}

fn line(sk: &Sketch, id: CurveId) -> (PointId, PointId) {
    sk.curve(id).line_points().expect("line expected")
}

fn dir(sk: &Sketch, id: CurveId) -> (f64, f64) {
    let (a, b) = line(sk, id);
    let (pa, pb) = (sk.xy(a), sk.xy(b));
    (pb.0 - pa.0, pb.1 - pa.1)
}

/// Fills orientation signs that are still `0` from the current geometry (branch selection on add, F107).
pub fn resolve_orientation(kind: &mut ConstraintKind, sk: &Sketch) {
    use ConstraintKind::*;
    match kind {
        DistanceH { a, b, sign, .. } if *sign == 0 => *sign = sign_of(sk.xy(*b).0 - sk.xy(*a).0),
        DistanceV { a, b, sign, .. } if *sign == 0 => *sign = sign_of(sk.xy(*b).1 - sk.xy(*a).1),
        DistancePointLine { p, line: l, side, .. } if *side == 0 => {
            *side = sign_of(signed_dist(sk, *p, *l));
        }
        Tangent { line: l, curve, side } if *side == 0 => {
            let c = sk.curve(*curve).center().expect("round curve");
            *side = sign_of(signed_dist(sk, c, *l));
        }
        TangentCurves { c1, c2, side } if *side == 0 => {
            let (k1, k2) = (sk.curve(*c1), sk.curve(*c2));
            let (p1, p2) = (sk.xy(k1.center().unwrap()), sk.xy(k2.center().unwrap()));
            let d = libm::hypot(p2.0 - p1.0, p2.1 - p1.1);
            let (r1, r2) = (k1.radius().unwrap(), k2.radius().unwrap());
            *side = if (d - (r1 + r2)).abs() <= (d - (r1 - r2).abs()).abs() { 1 } else { -1 };
        }
        Angle { l1, l2, sign, .. } if *sign == 0 => {
            let (d1, d2) = (dir(sk, *l1), dir(sk, *l2));
            *sign = sign_of(d1.0 * d2.1 - d1.1 * d2.0);
        }
        _ => {}
    }
}

/// Signed distance of a point from a line in the current geometry.
pub fn signed_dist(sk: &Sketch, p: PointId, l: CurveId) -> f64 {
    let (a, b) = line(sk, l);
    let (pa, pb, pp) = (sk.xy(a), sk.xy(b), sk.xy(p));
    let d = (pb.0 - pa.0, pb.1 - pa.1);
    let len = libm::hypot(d.0, d.1);
    if len == 0.0 { 0.0 } else { (d.0 * (pp.1 - pa.1) - d.1 * (pp.0 - pa.0)) / len }
}

/// Compiles one driving constraint into equations. `Fix` yields none (handled by removing variables);
/// driven dimensions must not be passed here.
pub fn compile(c: &Constraint, sk: &Sketch, slots: &Slots, scale: f64, out: &mut Vec<Eq>) {
    use ConstraintKind::*;
    let mut kind = c.kind.clone();
    resolve_orientation(&mut kind, sk);
    let owner = Owner::Constraint(c.id);
    let mut push = |prim: Prim| out.push(Eq { prim, owner });
    let lp = |id: CurveId| {
        let (a, b) = line(sk, id);
        [slots.p(a), slots.p(b)]
    };
    let center = |id: CurveId| slots.p(sk.curve(id).center().expect("round curve"));
    let lin = |terms: Vec<(usize, f64)>, k: f64| Prim::Lin { terms, k };
    match kind {
        Coincident { a, b } => {
            let (a, b) = (slots.p(a), slots.p(b));
            push(lin(vec![(b[0], 1.0), (a[0], -1.0)], 0.0));
            push(lin(vec![(b[1], 1.0), (a[1], -1.0)], 0.0));
        }
        PointOnLine { p, line } => {
            let [a, b] = lp(line);
            push(Prim::LineDist { p: slots.p(p), a, b, r: None, k: 0.0 });
        }
        PointOnCircle { p, curve } => {
            push(Prim::Dist { a: center(curve), b: slots.p(p), r: vec![(slots.r(curve), 1.0)], k: 0.0 });
        }
        Midpoint { p, line } => {
            let ([a, b], p) = (lp(line), slots.p(p));
            for i in 0..2 {
                push(lin(vec![(a[i], 0.5), (b[i], 0.5), (p[i], -1.0)], 0.0));
            }
        }
        Horizontal { line } => {
            let [a, b] = lp(line);
            push(lin(vec![(b[1], 1.0), (a[1], -1.0)], 0.0));
        }
        Vertical { line } => {
            let [a, b] = lp(line);
            push(lin(vec![(b[0], 1.0), (a[0], -1.0)], 0.0));
        }
        Parallel { l1, l2 } => push(Prim::Cross { l1: lp(l1), l2: lp(l2), s: scale }),
        Perpendicular { l1, l2 } => push(Prim::Dot { l1: lp(l1), l2: lp(l2), s: scale }),
        Collinear { l1, l2 } => {
            let ([a, b], [p, q]) = (lp(l1), lp(l2));
            push(Prim::LineDist { p, a, b, r: None, k: 0.0 });
            push(Prim::LineDist { p: q, a, b, r: None, k: 0.0 });
        }
        EqualLength { l1, l2 } => push(Prim::LenDiff { l1: lp(l1), l2: lp(l2) }),
        EqualRadius { c1, c2 } => push(lin(vec![(slots.r(c1), 1.0), (slots.r(c2), -1.0)], 0.0)),
        Concentric { c1, c2 } => {
            let (a, b) = (center(c1), center(c2));
            push(lin(vec![(b[0], 1.0), (a[0], -1.0)], 0.0));
            push(lin(vec![(b[1], 1.0), (a[1], -1.0)], 0.0));
        }
        Tangent { line, curve, side } => {
            let [a, b] = lp(line);
            push(Prim::LineDist { p: center(curve), a, b, r: Some((slots.r(curve), side as f64)), k: 0.0 });
        }
        TangentCurves { c1, c2, side } => {
            let (r1, r2) = (slots.r(c1), slots.r(c2));
            let r = if side >= 0 {
                vec![(r1, 1.0), (r2, 1.0)]
            } else {
                let s = if slots.vals[r1] >= slots.vals[r2] { 1.0 } else { -1.0 };
                vec![(r1, s), (r2, -s)]
            };
            push(Prim::Dist { a: center(c1), b: center(c2), r, k: 0.0 });
        }
        Symmetric { p1, p2, axis } => {
            let ([a, b], p, q) = (lp(axis), slots.p(p1), slots.p(p2));
            push(Prim::SymMid { p, q, a, b });
            push(Prim::SymPerp { p, q, a, b });
        }
        SymmetricLines { l1, l2, axis } => {
            let ([a, b], m, n) = (lp(axis), lp(l1), lp(l2));
            for i in 0..2 {
                push(Prim::SymMid { p: m[i], q: n[i], a, b });
                push(Prim::SymPerp { p: m[i], q: n[i], a, b });
            }
        }
        Fix { .. } => {}
        Distance { a, b, value } => push(Prim::Dist { a: slots.p(a), b: slots.p(b), r: vec![], k: value }),
        DistanceH { a, b, value, sign } => {
            let (a, b) = (slots.p(a), slots.p(b));
            push(lin(vec![(b[0], 1.0), (a[0], -1.0)], sign as f64 * value));
        }
        DistanceV { a, b, value, sign } => {
            let (a, b) = (slots.p(a), slots.p(b));
            push(lin(vec![(b[1], 1.0), (a[1], -1.0)], sign as f64 * value));
        }
        DistancePointLine { p, line, value, side } => {
            let [a, b] = lp(line);
            push(Prim::LineDist { p: slots.p(p), a, b, r: None, k: side as f64 * value });
        }
        Radius { curve, value } => push(lin(vec![(slots.r(curve), 1.0)], value)),
        Diameter { curve, value } => push(lin(vec![(slots.r(curve), 2.0)], value)),
        Angle { l1, l2, degrees, sign } => {
            let theta = sign as f64 * degrees * (PI / 180.0);
            push(Prim::Angle { l1: lp(l1), l2: lp(l2), theta, s: scale });
        }
    }
}

/// Built-in equations of a curve (arc endpoints on its circle).
pub fn compile_internal(c: &Curve, slots: &Slots, out: &mut Vec<Eq>) {
    if let CurveGeom::Arc { center, start, end, .. } = c.geom {
        let (cs, r) = (slots.p(center), slots.r(c.id));
        for p in [start, end] {
            out.push(Eq {
                prim: Prim::Dist { a: cs, b: slots.p(p), r: vec![(r, 1.0)], k: 0.0 },
                owner: Owner::Internal(c.id),
            });
        }
    }
}

/// Measured value of a dimension in the current geometry (driven dimensions, F38).
pub fn measure(kind: &ConstraintKind, sk: &Sketch) -> Option<f64> {
    use ConstraintKind::*;
    let dist = |a: PointId, b: PointId| {
        let (pa, pb) = (sk.xy(a), sk.xy(b));
        libm::hypot(pb.0 - pa.0, pb.1 - pa.1)
    };
    Some(match *kind {
        Distance { a, b, .. } => dist(a, b),
        DistanceH { a, b, .. } => (sk.xy(b).0 - sk.xy(a).0).abs(),
        DistanceV { a, b, .. } => (sk.xy(b).1 - sk.xy(a).1).abs(),
        DistancePointLine { p, line, .. } => signed_dist(sk, p, line).abs(),
        Radius { curve, .. } => sk.curve(curve).radius()?,
        Diameter { curve, .. } => 2.0 * sk.curve(curve).radius()?,
        Angle { l1, l2, .. } => {
            let (d1, d2) = (dir(sk, l1), dir(sk, l2));
            let a = libm::atan2(d1.0 * d2.1 - d1.1 * d2.0, d1.0 * d2.0 + d1.1 * d2.1);
            a.abs() * (180.0 / PI)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random generator for test configurations (no external RNG crate).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 20.0 - 10.0
        }
    }

    fn check_fd(prim: &Prim, v: &[f64]) {
        let row = prim.eval(v);
        let h = 1e-6;
        for s in 0..v.len() {
            let mut vp = v.to_vec();
            let mut vm = v.to_vec();
            vp[s] += h;
            vm[s] -= h;
            let fd = (prim.eval(&vp).val - prim.eval(&vm).val) / (2.0 * h);
            let an: f64 = row.grad().iter().filter(|e| e.0 == s).map(|e| e.1).sum();
            assert!((fd - an).abs() <= 1e-6 * (1.0 + fd.abs()), "{prim:?} slot {s}: fd {fd} analytic {an}");
        }
    }

    #[test]
    fn primitives_match_finite_differences() {
        let mut rng = Lcg(42);
        for _ in 0..200 {
            let v: Vec<f64> = (0..12).map(|_| rng.next()).collect();
            let (p0, p1, p2, p3, p4) = ([0, 1], [2, 3], [4, 5], [6, 7], [8, 9]);
            let prims = [
                Prim::Lin { terms: vec![(0, 1.0), (3, -2.0), (10, 0.5)], k: 1.5 },
                Prim::Dist { a: p0, b: p1, r: vec![(10, 1.0), (11, -1.0)], k: 2.0 },
                Prim::LineDist { p: p2, a: p0, b: p1, r: Some((10, -1.0)), k: 0.3 },
                Prim::Cross { l1: [p0, p1], l2: [p2, p3], s: 7.0 },
                Prim::Dot { l1: [p0, p1], l2: [p2, p3], s: 7.0 },
                Prim::Cross { l1: [p0, p1], l2: [p1, p2], s: 1.0 },
                Prim::LenDiff { l1: [p0, p1], l2: [p2, p3] },
                Prim::Angle { l1: [p0, p1], l2: [p2, p3], theta: 0.7, s: 3.0 },
                Prim::SymMid { p: p2, q: p3, a: p0, b: p4 },
                Prim::SymPerp { p: p2, q: p3, a: p0, b: p4 },
                Prim::CrossConst { a: p0, b: p1, u: [0.6, 0.8], s: 2.0 },
            ];
            for prim in &prims {
                // Stay away from the ±π cut of the angle residual where it is discontinuous.
                if let Prim::Angle { .. } = prim {
                    if prim.eval(&v).val.abs() > 0.9 * 3.0 * PI {
                        continue;
                    }
                }
                check_fd(prim, &v);
            }
        }
    }

    fn sample_sketch() -> Sketch {
        let mut sk = Sketch::default();
        let pts = [
            (0.0, 0.0),
            (10.0, 1.0),
            (2.0, 5.0),
            (9.0, 7.0),
            (4.0, -3.0),
            (6.0, 3.0),
            (-2.0, 4.0),
            (1.0, 9.0),
            (3.0, 1.0),
        ];
        for (i, (x, y)) in pts.iter().enumerate() {
            let id = PointId(i as u64 + 1);
            sk.points.insert(id, Point { id, x: *x, y: *y, prov: Provenance::default() });
        }
        let mut curve = |id: u64, geom: CurveGeom| {
            sk.curves
                .insert(CurveId(id), Curve { id: CurveId(id), geom, construction: false, prov: Provenance::default() });
        };
        curve(20, CurveGeom::Line { a: PointId(1), b: PointId(2) });
        curve(21, CurveGeom::Line { a: PointId(3), b: PointId(4) });
        curve(22, CurveGeom::Line { a: PointId(5), b: PointId(9) });
        curve(23, CurveGeom::Circle { center: PointId(6), r: 2.5 });
        curve(24, CurveGeom::Arc { center: PointId(7), start: PointId(8), end: PointId(3), r: 4.0 });
        sk
    }

    #[test]
    fn every_constraint_compiles_with_correct_jacobian() {
        use ConstraintKind::*;
        let sk = sample_sketch();
        let (p, l, c) = (PointId, CurveId, CurveId);
        let kinds = vec![
            Coincident { a: p(1), b: p(3) },
            PointOnLine { p: p(5), line: l(20) },
            PointOnCircle { p: p(4), curve: c(23) },
            Midpoint { p: p(6), line: l(21) },
            Horizontal { line: l(20) },
            Vertical { line: l(21) },
            Parallel { l1: l(20), l2: l(21) },
            Perpendicular { l1: l(20), l2: l(22) },
            Collinear { l1: l(20), l2: l(22) },
            EqualLength { l1: l(20), l2: l(21) },
            EqualRadius { c1: c(23), c2: c(24) },
            Concentric { c1: c(23), c2: c(24) },
            Tangent { line: l(21), curve: c(23), side: 0 },
            TangentCurves { c1: c(23), c2: c(24), side: 0 },
            Symmetric { p1: p(3), p2: p(4), axis: l(22) },
            SymmetricLines { l1: l(20), l2: l(21), axis: l(22) },
            Distance { a: p(1), b: p(2), value: 12.0 },
            DistanceH { a: p(1), b: p(2), value: 12.0, sign: 0 },
            DistanceV { a: p(1), b: p(3), value: 4.0, sign: 0 },
            DistancePointLine { p: p(6), line: l(20), value: 3.0, side: 0 },
            Radius { curve: c(23), value: 3.0 },
            Diameter { curve: c(24), value: 3.0 },
            Angle { l1: l(20), l2: l(21), degrees: 30.0, sign: 0 },
        ];
        let slots = Slots::new(&sk);
        let mut all = Vec::new();
        for (i, kind) in kinds.into_iter().enumerate() {
            assert!(sk.check_refs(&kind), "{kind:?}");
            let con = Constraint { id: ConstraintId(100 + i as u64), kind, driving: true, prov: Provenance::default() };
            let before = all.len();
            compile(&con, &sk, &slots, 10.0, &mut all);
            assert!(all.len() > before, "{:?} produced no equations", con.kind);
        }
        compile_internal(sk.curve(CurveId(24)), &slots, &mut all);
        for eq in &all {
            check_fd(&eq.prim, &slots.vals);
        }
    }

    #[test]
    fn orientation_signs_follow_geometry() {
        let sk = sample_sketch();
        let mut k = ConstraintKind::DistanceH { a: PointId(2), b: PointId(1), value: 5.0, sign: 0 };
        resolve_orientation(&mut k, &sk);
        assert_eq!(k, ConstraintKind::DistanceH { a: PointId(2), b: PointId(1), value: 5.0, sign: -1 });
        let mut t = ConstraintKind::Tangent { line: CurveId(20), curve: CurveId(23), side: 0 };
        resolve_orientation(&mut t, &sk);
        assert_eq!(t, ConstraintKind::Tangent { line: CurveId(20), curve: CurveId(23), side: 1 });
    }

    #[test]
    fn measured_values() {
        let sk = sample_sketch();
        let m = |k| measure(&k, &sk).unwrap();
        assert!(
            (m(ConstraintKind::Distance { a: PointId(1), b: PointId(2), value: 0.0 }) - 101f64.sqrt()).abs() < 1e-12
        );
        assert_eq!(m(ConstraintKind::Diameter { curve: CurveId(23), value: 0.0 }), 5.0);
        let a = m(ConstraintKind::Angle { l1: CurveId(20), l2: CurveId(21), degrees: 0.0, sign: 0 });
        let expect = (libm::atan2(2.0, 7.0) - libm::atan2(1.0, 10.0)) * 180.0 / PI;
        assert!((a - expect).abs() < 1e-12, "{a} {expect}");
    }
}
