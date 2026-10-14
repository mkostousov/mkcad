//! PoC K-3 — profile detection as a planar arrangement (F.8).
//!
//! Every non-construction curve is cut at its intersections with the others; the pieces form a planar graph whose
//! bounded faces are the profiles. Regions come from curve *intersections*, not from connectivity (F73–F76):
//! a crossing line splits a rectangle, overshooting lines bound an inner square, open chains give no profile, nested
//! loops become holes, exact duplicates collapse to one region.
//!
//! One linear tolerance [`TOL_LIN`] is used for closure, coincidence and degeneracy. Points closer than it are
//! one vertex (this only affects the arrangement; the sketch's points are never moved).
//!
//! Pure computation, deterministic ordering, `libm` for transcendental functions.

use crate::diag::Code;
use crate::model::{CurveGeom, CurveId, Sketch};
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

/// Linear tolerance (mm): chain gaps up to this close a profile, larger ones do not (F76).
pub const TOL_LIN: f64 = 1e-5;

type P = [f64; 2];

#[derive(Debug, Clone, PartialEq)]
pub enum PieceGeom {
    Line,
    /// `ccw` is the traversal sense of this piece within the loop.
    Arc { center: P, radius: f64, ccw: bool },
    /// A whole circle (start == end).
    Circle { center: P, radius: f64, ccw: bool },
}

/// One boundary piece in traversal order.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub curve: CurveId,
    pub start: P,
    pub end: P,
    pub geom: PieceGeom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub pieces: Vec<Piece>,
    /// Unsigned area enclosed by the loop.
    pub area: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub outer: Loop,
    pub holes: Vec<Loop>,
    /// `outer.area − Σ holes.area`.
    pub area: f64,
}

impl Profile {
    /// Number of loops (outer + holes).
    pub fn loop_count(&self) -> usize {
        1 + self.holes.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileDiagnostic {
    pub code: Code,
    pub curves: Vec<CurveId>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProfileSet {
    pub profiles: Vec<Profile>,
    pub diagnostics: Vec<ProfileDiagnostic>,
}

// ---------------------------------------------------------------------------------------------------------
// Primitives

#[derive(Debug, Clone)]
enum Kind {
    Line { p: P, q: P },
    /// Counter-clockwise arc from angle `a0` over `sweep`; a circle has `sweep == TAU`.
    Arc { c: P, r: f64, a0: f64, sweep: f64 },
}

#[derive(Debug, Clone)]
struct Prim {
    id: CurveId,
    kind: Kind,
}

impl Prim {
    fn bbox(&self) -> (P, P) {
        match self.kind {
            Kind::Line { p, q } => ([p[0].min(q[0]), p[1].min(q[1])], [p[0].max(q[0]), p[1].max(q[1])]),
            Kind::Arc { c, r, .. } => ([c[0] - r, c[1] - r], [c[0] + r, c[1] + r]),
        }
    }

    fn ends(&self) -> Vec<P> {
        match self.kind {
            Kind::Line { p, q } => vec![p, q],
            Kind::Arc { c, r, a0, sweep } => {
                if sweep >= TAU {
                    vec![]
                } else {
                    vec![at(c, r, a0), at(c, r, a0 + sweep)]
                }
            }
        }
    }
}

fn at(c: P, r: f64, a: f64) -> P {
    [c[0] + r * libm::cos(a), c[1] + r * libm::sin(a)]
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn len(a: P) -> f64 {
    a[0].hypot(a[1])
}

fn angle(v: P) -> f64 {
    libm::atan2(v[1], v[0])
}

/// Angle of `p` around `c`, measured counter-clockwise from `a0`, in [0, TAU).
fn ang_from(c: P, a0: f64, p: P) -> f64 {
    let mut a = angle(sub(p, c)) - a0;
    a -= TAU * (a / TAU).floor();
    if a >= TAU { 0.0 } else { a }
}

/// Distance from `pt` to the primitive and the closest point on it.
fn closest(prim: &Prim, pt: P) -> (f64, P) {
    match prim.kind {
        Kind::Line { p, q } => {
            let d = sub(q, p);
            let t = (dot(sub(pt, p), d) / dot(d, d)).clamp(0.0, 1.0);
            let c = [p[0] + t * d[0], p[1] + t * d[1]];
            (len(sub(pt, c)), c)
        }
        Kind::Arc { c, r, a0, sweep } => {
            let v = sub(pt, c);
            if len(v) > 0.0 {
                let a = ang_from(c, a0, pt);
                if sweep >= TAU || a <= sweep {
                    return ((len(v) - r).abs(), at(c, r, a0 + a));
                }
            }
            let (s, e) = (at(c, r, a0), at(c, r, a0 + sweep));
            let (ds, de) = (len(sub(pt, s)), len(sub(pt, e)));
            if ds <= de { (ds, s) } else { (de, e) }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Intersections of the carriers (filtered against both primitives afterwards)

fn carrier_points(a: &Prim, b: &Prim, out: &mut Vec<P>) {
    match (&a.kind, &b.kind) {
        (Kind::Line { p: p1, q: q1 }, Kind::Line { p: p2, q: q2 }) => {
            let (r, s) = (sub(*q1, *p1), sub(*q2, *p2));
            let den = cross(r, s);
            if den.abs() <= 1e-14 * len(r) * len(s) {
                return; // parallel / collinear: handled by endpoint proximity
            }
            let t = cross(sub(*p2, *p1), s) / den;
            out.push([p1[0] + t * r[0], p1[1] + t * r[1]]);
        }
        (Kind::Line { p, q }, Kind::Arc { c, r, .. }) | (Kind::Arc { c, r, .. }, Kind::Line { p, q }) => {
            let d = sub(*q, *p);
            let l = len(d);
            let u = [d[0] / l, d[1] / l];
            let along = dot(sub(*c, *p), u);
            let foot = [p[0] + along * u[0], p[1] + along * u[1]];
            let dist = len(sub(*c, foot));
            let h2 = r * r - dist * dist;
            if h2 < 0.0 {
                if *r - dist >= -TOL_LIN {
                    out.push(foot);
                }
                return;
            }
            let h = h2.sqrt();
            out.push([foot[0] - h * u[0], foot[1] - h * u[1]]);
            out.push([foot[0] + h * u[0], foot[1] + h * u[1]]);
        }
        (Kind::Arc { c: c1, r: r1, .. }, Kind::Arc { c: c2, r: r2, .. }) => {
            let v = sub(*c2, *c1);
            let d = len(v);
            if d <= TOL_LIN {
                return; // concentric: no transversal intersection
            }
            if d > r1 + r2 + TOL_LIN || d < (r1 - r2).abs() - TOL_LIN {
                return;
            }
            let a = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
            let h = (r1 * r1 - a * a).max(0.0).sqrt();
            let u = [v[0] / d, v[1] / d];
            let m = [c1[0] + a * u[0], c1[1] + a * u[1]];
            out.push([m[0] - h * u[1], m[1] + h * u[0]]);
            out.push([m[0] + h * u[1], m[1] - h * u[0]]);
        }
    }
}

fn overlap(a: &(P, P), b: &(P, P), tol: f64) -> bool {
    a.0[0] <= b.1[0] + tol && b.0[0] <= a.1[0] + tol && a.0[1] <= b.1[1] + tol && b.0[1] <= a.1[1] + tol
}

// ---------------------------------------------------------------------------------------------------------
// Graph

#[derive(Debug, Clone)]
enum EGeom {
    Line,
    /// Forward direction (u→v) is counter-clockwise from angle `a0` over `sweep`.
    Arc { c: P, r: f64, a0: f64, sweep: f64 },
}

#[derive(Debug, Clone)]
struct Edge {
    u: usize,
    v: usize,
    curve: CurveId,
    geom: EGeom,
}

struct Graph {
    verts: Vec<P>,
    edges: Vec<Edge>,
}

impl Graph {
    /// Tail and head vertex of half-edge `h` (even = forward u→v, odd = backward).
    fn half(&self, h: usize) -> (usize, usize) {
        let e = &self.edges[h / 2];
        if h.is_multiple_of(2) { (e.u, e.v) } else { (e.v, e.u) }
    }

    /// Direction angle and signed curvature (left turn positive) at the tail of half-edge `h`.
    fn tail_dir(&self, h: usize) -> (f64, f64) {
        let e = &self.edges[h / 2];
        let fwd = h.is_multiple_of(2);
        match &e.geom {
            EGeom::Line => {
                let (a, b) = (self.verts[e.u], self.verts[e.v]);
                let d = if fwd { sub(b, a) } else { sub(a, b) };
                (angle(d), 0.0)
            }
            EGeom::Arc { a0, sweep, r, .. } => {
                if fwd {
                    (a0 + PI / 2.0, 1.0 / r)
                } else {
                    (a0 + sweep - PI / 2.0, -1.0 / r)
                }
            }
        }
    }

    /// Signed area (Green's theorem) of a cycle of half-edges, relative to its first vertex.
    fn cycle_area(&self, cycle: &[usize]) -> f64 {
        let o = self.verts[self.half(cycle[0]).0];
        let mut a = 0.0;
        for &h in cycle {
            let e = &self.edges[h / 2];
            let (s, t) = self.half(h);
            match &e.geom {
                EGeom::Line => a += cross(sub(self.verts[s], o), sub(self.verts[t], o)),
                EGeom::Arc { c, r, a0, sweep } => {
                    let (a_start, delta) = if h.is_multiple_of(2) { (*a0, *sweep) } else { (a0 + sweep, -*sweep) };
                    let a_end = a_start + delta;
                    let cc = sub(*c, o);
                    a += r * r * delta + r * cc[0] * (libm::sin(a_end) - libm::sin(a_start))
                        - r * cc[1] * (libm::cos(a_end) - libm::cos(a_start));
                }
            }
        }
        0.5 * a
    }

    /// Faces as half-edge cycles (the face on the left of each half-edge).
    fn cycles(&self) -> Vec<Vec<usize>> {
        let nv = self.verts.len();
        let mut out_he: Vec<Vec<(f64, f64, usize)>> = vec![Vec::new(); nv];
        for h in 0..self.edges.len() * 2 {
            let (t, _) = self.half(h);
            let (mut th, k) = self.tail_dir(h);
            th -= TAU * (th / TAU).floor();
            if th > TAU - 1e-9 {
                th = 0.0;
            }
            out_he[t].push((th, k, h));
        }
        // Counter-clockwise order; equal directions (tangent curves) ordered by curvature.
        let mut pos = vec![0usize; self.edges.len() * 2];
        for list in out_he.iter_mut() {
            list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
            let mut i = 0;
            while i < list.len() {
                let mut j = i + 1;
                while j < list.len() && list[j].0 - list[i].0 < 1e-9 {
                    j += 1;
                }
                list[i..j].sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)));
                i = j;
            }
            for (k, e) in list.iter().enumerate() {
                pos[e.2] = k;
            }
        }
        let mut seen = vec![false; self.edges.len() * 2];
        let mut cycles = Vec::new();
        for start in 0..self.edges.len() * 2 {
            if seen[start] {
                continue;
            }
            let mut cyc = Vec::new();
            let mut h = start;
            while !seen[h] {
                seen[h] = true;
                cyc.push(h);
                let (_, head) = self.half(h);
                let k = pos[h ^ 1];
                let list = &out_he[head];
                h = list[(k + list.len() - 1) % list.len()].2;
            }
            cycles.push(cyc);
        }
        cycles
    }

    /// Connected-component id per vertex.
    fn components(&self) -> Vec<usize> {
        let mut parent: Vec<usize> = (0..self.verts.len()).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        for e in &self.edges {
            let (a, b) = (find(&mut parent, e.u), find(&mut parent, e.v));
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        }
        (0..self.verts.len()).map(|v| find(&mut parent, v)).collect()
    }
}

/// Winding number of the closed half-edge cycle around `p`.
fn winding(g: &Graph, cycle: &[usize], p: P) -> i32 {
    let mut total = 0.0;
    for &h in cycle {
        let e = &g.edges[h / 2];
        let (s, t) = g.half(h);
        let (a, b) = (g.verts[s], g.verts[t]);
        total += libm::atan2(cross(sub(a, p), sub(b, p)), dot(sub(a, p), sub(b, p)));
        if let EGeom::Arc { c, r, .. } = &e.geom {
            // The region between the forward (ccw) arc and its chord is the disc ∩ right side of the chord.
            let (fa, fb) = (g.verts[e.u], g.verts[e.v]);
            let inside = len(sub(p, *c)) < *r;
            let right = e.u == e.v || cross(sub(fb, fa), sub(p, fa)) < 0.0;
            if inside && right {
                total += if h.is_multiple_of(2) { TAU } else { -TAU };
            }
        }
    }
    (total / TAU).round() as i32
}

// ---------------------------------------------------------------------------------------------------------

fn degenerate(diags: &mut Vec<ProfileDiagnostic>, id: CurveId) {
    diags.push(ProfileDiagnostic { code: Code::GeometryDegenerate, curves: vec![id] });
}

/// Detects the profiles of the solved sketch.
pub fn detect_profiles(sketch: &Sketch) -> ProfileSet {
    let mut diagnostics = Vec::new();
    let xy = |id| {
        let p = &sketch.points[&id];
        [p.x, p.y]
    };

    // 1. Primitives (construction curves never take part; degenerate ones are reported and skipped).
    let mut prims: Vec<Prim> = Vec::new();
    for (id, c) in &sketch.curves {
        if c.construction {
            continue;
        }
        match c.geom {
            CurveGeom::Line { a, b } => {
                let (p, q) = (xy(a), xy(b));
                if len(sub(q, p)) <= TOL_LIN {
                    degenerate(&mut diagnostics, *id);
                    continue;
                }
                prims.push(Prim { id: *id, kind: Kind::Line { p, q } });
            }
            CurveGeom::Circle { center, r } => {
                if r <= TOL_LIN {
                    degenerate(&mut diagnostics, *id);
                    continue;
                }
                prims.push(Prim { id: *id, kind: Kind::Arc { c: xy(center), r, a0: 0.0, sweep: TAU } });
            }
            CurveGeom::Arc { center, start, end, r } => {
                if r <= TOL_LIN {
                    degenerate(&mut diagnostics, *id);
                    continue;
                }
                let c = xy(center);
                let a0 = angle(sub(xy(start), c));
                let mut sweep = ang_from(c, a0, xy(end));
                if sweep <= 1e-12 {
                    sweep = TAU;
                }
                prims.push(Prim { id: *id, kind: Kind::Arc { c, r, a0, sweep } });
            }
        }
    }

    // Exact duplicates are tolerated (one region) and reported once per extra copy.
    let mut dup = vec![false; prims.len()];
    for i in 0..prims.len() {
        if dup[i] {
            continue;
        }
        for j in i + 1..prims.len() {
            if !dup[j] && same_prim(&prims[i], &prims[j]) {
                dup[j] = true;
                diagnostics.push(ProfileDiagnostic {
                    code: Code::GeometryDuplicate,
                    curves: vec![prims[i].id, prims[j].id],
                });
            }
        }
    }
    let prims: Vec<Prim> = prims.into_iter().zip(&dup).filter(|(_, d)| !**d).map(|(p, _)| p).collect();

    // 2. Cut points on every primitive.
    let boxes: Vec<(P, P)> = prims.iter().map(Prim::bbox).collect();
    let mut cuts: Vec<Vec<P>> = prims.iter().map(|p| p.ends()).collect();
    for i in 0..prims.len() {
        for j in i + 1..prims.len() {
            if !overlap(&boxes[i], &boxes[j], TOL_LIN) {
                continue;
            }
            let mut pts = Vec::new();
            carrier_points(&prims[i], &prims[j], &mut pts);
            for p in pts {
                if closest(&prims[i], p).0 <= TOL_LIN && closest(&prims[j], p).0 <= TOL_LIN {
                    cuts[i].push(p);
                    cuts[j].push(p);
                }
            }
            // Endpoints touching the other curve (T junctions, overlaps, near misses within the tolerance).
            for (a, b) in [(i, j), (j, i)] {
                for e in prims[a].ends() {
                    let (d, cp) = closest(&prims[b], e);
                    if d <= TOL_LIN {
                        cuts[b].push(cp);
                    }
                }
            }
        }
    }

    // 3. Vertices: cluster all cut points within the tolerance (sorted; the first point represents a cluster).
    let mut all: Vec<(P, usize, usize)> = Vec::new(); // (point, prim, index in cuts)
    for (i, c) in cuts.iter().enumerate() {
        for (k, p) in c.iter().enumerate() {
            all.push((*p, i, k));
        }
    }
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by(|&a, &b| {
        all[a].0[0].total_cmp(&all[b].0[0]).then(all[a].0[1].total_cmp(&all[b].0[1])).then(a.cmp(&b))
    });
    let tol = TOL_LIN * (1.0 + 1e-9);
    let mut parent: Vec<usize> = (0..all.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for (oi, &a) in order.iter().enumerate() {
        for &b in &order[oi + 1..] {
            if all[b].0[0] - all[a].0[0] > tol {
                break;
            }
            if len(sub(all[a].0, all[b].0)) <= tol {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut vert_of_root: BTreeMap<usize, usize> = BTreeMap::new();
    let mut verts: Vec<P> = Vec::new();
    let mut vid = vec![0usize; all.len()];
    for &a in &order {
        let r = find(&mut parent, a);
        let v = *vert_of_root.entry(r).or_insert_with(|| {
            verts.push(all[a].0);
            verts.len() - 1
        });
        vid[a] = v;
    }
    let mut vert_by_prim: Vec<Vec<usize>> = cuts.iter().map(|c| vec![0; c.len()]).collect();
    for (n, &(_, i, k)) in all.iter().enumerate() {
        vert_by_prim[i][k] = vid[n];
    }

    // 4. Edges between consecutive cut points of each primitive.
    let mut edges: Vec<Edge> = Vec::new();
    for (i, prim) in prims.iter().enumerate() {
        match prim.kind {
            Kind::Line { p, q } => {
                let d = sub(q, p);
                let mut pts: Vec<(f64, usize)> = cuts[i]
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (dot(sub(*c, p), d) / dot(d, d), vert_by_prim[i][k]))
                    .collect();
                pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                pts.dedup_by_key(|x| x.1);
                for w in pts.windows(2) {
                    if w[0].1 != w[1].1 {
                        edges.push(Edge { u: w[0].1, v: w[1].1, curve: prim.id, geom: EGeom::Line });
                    }
                }
            }
            Kind::Arc { c, r, a0, sweep } => {
                let circle = sweep >= TAU;
                let tang = 4.0 * TOL_LIN / r;
                let mut pts: Vec<(f64, usize)> = cuts[i]
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let mut a = ang_from(c, a0, *p);
                        if !circle && a > sweep && a < TAU - tang {
                            a = sweep; // within tolerance of the end
                        }
                        if !circle && a >= TAU - tang {
                            a = 0.0;
                        }
                        (a.min(if circle { TAU } else { sweep }), vert_by_prim[i][k])
                    })
                    .collect();
                if circle && pts.is_empty() {
                    verts.push(at(c, r, a0));
                    pts.push((0.0, verts.len() - 1));
                }
                pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                pts.dedup_by_key(|x| x.1);
                if circle && pts.len() > 1 && pts[0].1 == pts[pts.len() - 1].1 {
                    pts.pop();
                }
                if circle {
                    if pts.len() == 1 {
                        edges.push(Edge {
                            u: pts[0].1,
                            v: pts[0].1,
                            curve: prim.id,
                            geom: EGeom::Arc { c, r, a0: a0 + pts[0].0, sweep: TAU },
                        });
                    } else {
                        for k in 0..pts.len() {
                            let (a, va) = pts[k];
                            let (b, vb) = pts[(k + 1) % pts.len()];
                            let sw = if k + 1 < pts.len() { b - a } else { b + TAU - a };
                            edges.push(Edge {
                                u: va,
                                v: vb,
                                curve: prim.id,
                                geom: EGeom::Arc { c, r, a0: a0 + a, sweep: sw },
                            });
                        }
                    }
                } else {
                    for w in pts.windows(2) {
                        if w[0].1 != w[1].1 {
                            edges.push(Edge {
                                u: w[0].1,
                                v: w[1].1,
                                curve: prim.id,
                                geom: EGeom::Arc { c, r, a0: a0 + w[0].0, sweep: w[1].0 - w[0].0 },
                            });
                        }
                    }
                }
            }
        }
    }
    let edges = dedupe_edges(edges);

    // 5. Faces; edges with the same face on both sides (dangling or bridging) are removed until stable.
    let mut g = Graph { verts, edges };
    let cycles = loop {
        let cycles = g.cycles();
        let mut face_of = vec![0usize; g.edges.len() * 2];
        for (ci, c) in cycles.iter().enumerate() {
            for &h in c {
                face_of[h] = ci;
            }
        }
        let keep: Vec<Edge> = g
            .edges
            .iter()
            .enumerate()
            .filter(|(e, _)| face_of[2 * e] != face_of[2 * e + 1])
            .map(|(_, e)| e.clone())
            .collect();
        if keep.len() == g.edges.len() {
            break cycles;
        }
        g.edges = keep;
    };

    // 6. Bounded faces are counter-clockwise cycles; a clockwise cycle is the outer boundary of a component and
    // becomes a hole of the smallest bounded face (of another component) that contains it.
    let comp = g.components();
    let comp_of = |c: &[usize]| comp[g.half(c[0]).0];
    let areas: Vec<f64> = cycles.iter().map(|c| g.cycle_area(c)).collect();
    let bounded: Vec<usize> = (0..cycles.len()).filter(|&i| areas[i] > 0.0).collect();
    let outers: Vec<usize> = (0..cycles.len()).filter(|&i| areas[i] < 0.0).collect();
    let mut holes_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &o in &outers {
        let probe = g.verts[g.half(cycles[o][0]).0];
        let mut best: Option<usize> = None;
        for &f in &bounded {
            if comp_of(&cycles[f]) == comp_of(&cycles[o]) {
                continue;
            }
            if winding(&g, &cycles[f], probe) == 1 && best.is_none_or(|b| areas[f] < areas[b]) {
                best = Some(f);
            }
        }
        if let Some(f) = best {
            holes_of.entry(f).or_default().push(o);
        }
    }

    let make_loop = |c: &[usize]| -> Loop {
        let mut pieces: Vec<Piece> = c
            .iter()
            .map(|&h| {
                let e = &g.edges[h / 2];
                let (s, t) = g.half(h);
                let ccw = h.is_multiple_of(2);
                let geom = match &e.geom {
                    EGeom::Line => PieceGeom::Line,
                    EGeom::Arc { c, r, sweep, .. } if s == t && *sweep >= TAU => {
                        PieceGeom::Circle { center: *c, radius: *r, ccw }
                    }
                    EGeom::Arc { c, r, .. } => PieceGeom::Arc { center: *c, radius: *r, ccw },
                };
                Piece { curve: e.curve, start: g.verts[s], end: g.verts[t], geom }
            })
            .collect();
        // Deterministic start: the piece with the smallest (curve id, start point).
        let k = (0..pieces.len())
            .min_by(|&a, &b| {
                let (pa, pb) = (&pieces[a], &pieces[b]);
                pa.curve
                    .cmp(&pb.curve)
                    .then(pa.start[0].total_cmp(&pb.start[0]))
                    .then(pa.start[1].total_cmp(&pb.start[1]))
            })
            .unwrap_or(0);
        pieces.rotate_left(k);
        Loop { pieces, area: g.cycle_area(c).abs() }
    };

    let mut profiles: Vec<Profile> = bounded
        .iter()
        .map(|&f| {
            let outer = make_loop(&cycles[f]);
            let holes: Vec<Loop> = holes_of
                .get(&f)
                .map(|hs| hs.iter().map(|&o| make_loop(&cycles[o])).collect())
                .unwrap_or_default();
            let area = outer.area - holes.iter().map(|h| h.area).sum::<f64>();
            Profile { outer, holes, area }
        })
        .collect();
    profiles.sort_by(|a, b| {
        let key = |p: &Profile| {
            let first = &p.outer.pieces[0];
            (first.curve, first.start[0], first.start[1])
        };
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then(ka.2.total_cmp(&kb.2)).then(a.area.total_cmp(&b.area))
    });

    ProfileSet { profiles, diagnostics }
}

fn same_prim(a: &Prim, b: &Prim) -> bool {
    let near = |p: P, q: P| len(sub(p, q)) <= TOL_LIN;
    match (&a.kind, &b.kind) {
        (Kind::Line { p: p1, q: q1 }, Kind::Line { p: p2, q: q2 }) => {
            (near(*p1, *p2) && near(*q1, *q2)) || (near(*p1, *q2) && near(*q1, *p2))
        }
        (Kind::Arc { c: c1, r: r1, a0: a1, sweep: s1 }, Kind::Arc { c: c2, r: r2, a0: a2, sweep: s2 }) => {
            near(*c1, *c2)
                && (r1 - r2).abs() <= TOL_LIN
                && ((*s1 >= TAU && *s2 >= TAU)
                    || (*s1 < TAU
                        && *s2 < TAU
                        && near(at(*c1, *r1, *a1), at(*c2, *r2, *a2))
                        && near(at(*c1, *r1, a1 + s1), at(*c2, *r2, a2 + s2))))
        }
        _ => false,
    }
}

/// Identical sub-edges (overlapping curves cut at each other's endpoints) collapse to one.
fn dedupe_edges(edges: Vec<Edge>) -> Vec<Edge> {
    let mid = |e: &Edge| match &e.geom {
        EGeom::Arc { c, r, a0, sweep } => at(*c, *r, a0 + sweep / 2.0),
        EGeom::Line => [0.0, 0.0],
    };
    let mut by_pair: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    let mut out: Vec<Edge> = Vec::new();
    for e in edges {
        let key = (e.u.min(e.v), e.u.max(e.v));
        let same = by_pair.get(&key).is_some_and(|idx| {
            idx.iter().any(|&k| {
                let o: &Edge = &out[k];
                match (&o.geom, &e.geom) {
                    (EGeom::Line, EGeom::Line) => true,
                    // Two arcs between the same vertices on one circle are the same arc only if the midpoints agree.
                    (EGeom::Arc { c: c1, r: r1, .. }, EGeom::Arc { c: c2, r: r2, .. }) => {
                        len(sub(*c1, *c2)) <= TOL_LIN
                            && (r1 - r2).abs() <= TOL_LIN
                            && len(sub(mid(o), mid(&e))) <= 10.0 * TOL_LIN
                    }
                    _ => false,
                }
            })
        });
        if !same {
            by_pair.entry(key).or_default().push(out.len());
            out.push(e);
        }
    }
    out
}
