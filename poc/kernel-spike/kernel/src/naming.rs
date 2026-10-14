//! PoC K-2: persistent references (G.4) over OCCT operation history.
//!
//! * Every result face carries a set of *labels* derived from authoritative inputs (feature id, profile curve id,
//!   tool-box side), never from kernel enumeration order.
//! * Labels are propagated through the operation history (modified / generated / unchanged).
//! * A [`RefDesc`] stores a label (a face label, or the pair of face labels around an edge) plus a geometric
//!   signature, and is resolved against a rebuilt body to `exact | expanded(n) | signature | lost | ambiguous`.
//! * [`Timeline`] replays features from scratch (the recompute of G.3) and reports per-reference resolutions.

use crate::{HistoryEntry, Relation, Solid, SubKind};
use std::collections::{BTreeSet, HashMap};

pub type Labels = BTreeSet<String>;

const SIG_TOL: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    pub kind: SubKind,
    pub centroid: [f64; 3],
    pub normal: Option<[f64; 3]>,
    /// Area for a face, length for an edge.
    pub size: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefTarget {
    Face(String),
    /// Edge between faces carrying these labels (sorted pair).
    Edge(String, String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefDesc {
    pub target: RefTarget,
    pub signature: Signature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Exact,
    Expanded(usize),
    Signature,
    Lost,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub resolution: Resolution,
    /// 1-based face or edge indices in the body.
    pub indices: Vec<u32>,
}

/// A body together with the labels of its faces.
pub struct Labeled {
    pub solid: Solid,
    pub face_labels: Vec<Labels>,
}

fn primary(l: &Labels) -> String {
    l.iter().next().cloned().unwrap_or_default()
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt()
}

impl Labeled {
    fn labels_of(&self, face: u32) -> &Labels {
        &self.face_labels[face as usize - 1]
    }

    /// Sorted pair of primary labels of the two faces around `edge`.
    pub fn edge_pair(&self, edge: u32) -> Option<(String, String)> {
        let faces = self.solid.edge_faces(edge).ok()?;
        let (a, b) = match faces.as_slice() {
            [a, b] => (primary(self.labels_of(*a)), primary(self.labels_of(*b))),
            [a] => (primary(self.labels_of(*a)), primary(self.labels_of(*a))),
            _ => return None,
        };
        Some(if a <= b { (a, b) } else { (b, a) })
    }

    pub fn face_ref(&self, face: u32) -> Option<RefDesc> {
        let f = self.solid.face(face).ok()?;
        Some(RefDesc {
            target: RefTarget::Face(primary(self.labels_of(face))),
            signature: Signature { kind: SubKind::Face, centroid: f.center, normal: f.normal, size: f.area },
        })
    }

    pub fn edge_ref(&self, edge: u32) -> Option<RefDesc> {
        let e = self.solid.edge(edge).ok()?;
        let (a, b) = self.edge_pair(edge)?;
        Some(RefDesc {
            target: RefTarget::Edge(a, b),
            signature: Signature { kind: SubKind::Edge, centroid: e.center, normal: None, size: e.length },
        })
    }

    /// Descriptor of the face whose centroid is at `p` (selection by picking).
    pub fn face_ref_at(&self, p: [f64; 3]) -> Option<RefDesc> {
        self.face_ref(self.solid.find_face_at(p, 1e-6)?)
    }

    pub fn edge_ref_at(&self, p: [f64; 3]) -> Option<RefDesc> {
        self.edge_ref(self.solid.find_edge_at(p, 1e-6)?)
    }

    /// Resolve per G.4 step 4: exact label → expanded to all descendants → signature → lost.
    pub fn resolve(&self, r: &RefDesc) -> Resolved {
        let by_label: Vec<u32> = match &r.target {
            RefTarget::Face(l) => {
                (1..=self.solid.face_count()).filter(|&f| self.labels_of(f).contains(l)).collect()
            }
            RefTarget::Edge(a, b) => (1..=self.solid.edge_count())
                .filter(|&e| {
                    let Ok(fs) = self.solid.edge_faces(e) else { return false };
                    let (f1, f2) = match fs.as_slice() {
                        [x, y] => (*x, *y),
                        [x] => (*x, *x),
                        _ => return false,
                    };
                    let (l1, l2) = (self.labels_of(f1), self.labels_of(f2));
                    (l1.contains(a) && l2.contains(b)) || (l1.contains(b) && l2.contains(a))
                })
                .collect(),
        };
        match by_label.len() {
            0 => {}
            1 => return Resolved { resolution: Resolution::Exact, indices: by_label },
            n => return Resolved { resolution: Resolution::Expanded(n), indices: by_label },
        }
        let sig = &r.signature;
        let hits: Vec<u32> = match sig.kind {
            SubKind::Face => (1..=self.solid.face_count())
                .filter(|&f| {
                    self.solid.face(f).is_ok_and(|i| {
                        dist(i.center, sig.centroid) <= SIG_TOL
                            && (i.area - sig.size).abs() <= SIG_TOL * sig.size.max(1.0)
                            && match (i.normal, sig.normal) {
                                (Some(a), Some(b)) => dist(a, b) <= 1e-9,
                                (None, None) => true,
                                _ => false,
                            }
                    })
                })
                .collect(),
            SubKind::Edge => (1..=self.solid.edge_count())
                .filter(|&e| {
                    self.solid.edge(e).is_ok_and(|i| {
                        dist(i.center, sig.centroid) <= SIG_TOL
                            && (i.length - sig.size).abs() <= SIG_TOL * sig.size.max(1.0)
                    })
                })
                .collect(),
        };
        match hits.len() {
            0 => Resolved { resolution: Resolution::Lost, indices: vec![] },
            1 => Resolved { resolution: Resolution::Signature, indices: hits },
            _ => Resolved { resolution: Resolution::Ambiguous, indices: hits },
        }
    }
}

/// Carry labels from `prev` into `next` through `hist`. `tool_role` names a face of operand 1 (a boolean tool).
fn propagate(
    prev: &Labeled,
    next: &Solid,
    hist: &[HistoryEntry],
    fid: &str,
    tool_role: Option<&dyn Fn(u32) -> String>,
) -> Vec<Labels> {
    let mut out = vec![Labels::new(); next.face_count() as usize];
    for h in hist {
        let Some((SubKind::Face, d)) = h.dst else { continue };
        let slot = &mut out[d as usize - 1];
        match (h.arg, h.kind, h.relation) {
            (0, SubKind::Face, Relation::Modified | Relation::Unchanged) => {
                slot.extend(prev.face_labels[h.src as usize - 1].iter().cloned());
            }
            (0, SubKind::Face, Relation::Generated) => {
                for l in &prev.face_labels[h.src as usize - 1] {
                    slot.insert(format!("{fid}:gen:{l}"));
                }
            }
            (0, SubKind::Edge, Relation::Generated) => {
                if let Some((a, b)) = prev.edge_pair(h.src) {
                    slot.insert(format!("{fid}:gen:{a}|{b}"));
                }
            }
            (1, SubKind::Face, Relation::Modified | Relation::Unchanged | Relation::Generated) => {
                if let Some(role) = tool_role {
                    slot.insert(format!("{fid}:tool:{}", role(h.src)));
                }
            }
            _ => {}
        }
    }
    for (i, s) in out.iter_mut().enumerate() {
        if s.is_empty() {
            s.insert(format!("{fid}:anon:{}", i + 1));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------------------
// Timeline

#[derive(Debug, Clone, PartialEq)]
pub struct ProfilePoint {
    pub curve: String,
    /// Start point of the segment that begins here; the polygon is closed implicitly.
    pub xy: [f64; 2],
}

/// How a feature that needs one piece treats a reference that expanded to several.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiecePolicy {
    /// Several pieces → `REF.AMBIGUOUS`, never a silent pick.
    Single,
    /// Coplanar pieces are accepted as one support plane.
    AllPieces,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeatureKind {
    Extrude { profile: Vec<ProfilePoint>, height: f64 },
    /// Subtract an axis-aligned box (groove, hole, corner cut).
    CutBox { min: [f64; 3], size: [f64; 3] },
    Fillet { edges: Vec<RefDesc>, radius: f64 },
    Shell { faces: Vec<RefDesc>, thickness: f64 },
    /// Sketch on a face: only its support plane is modelled.
    Sketch { face: RefDesc, policy: PiecePolicy },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub id: String,
    pub suppressed: bool,
    pub kind: FeatureKind,
}

impl Feature {
    pub fn new(id: &str, kind: FeatureKind) -> Self {
        Feature { id: id.to_string(), suppressed: false, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Error,
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureReport {
    pub id: String,
    pub health: Health,
    pub resolutions: Vec<Resolution>,
    /// Diagnostic codes (H.5): `REF.LOST`, `REF.AMBIGUOUS`, `SKETCH.REF.LOST`, `KERNEL.OP.FAILED`.
    pub diagnostics: Vec<String>,
    pub plane: Option<Plane>,
}

pub struct Rebuild {
    pub body: Option<Labeled>,
    pub reports: Vec<FeatureReport>,
}

impl Rebuild {
    pub fn report(&self, id: &str) -> &FeatureReport {
        self.reports.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("no report for {id}"))
    }
}

#[derive(Default)]
pub struct Timeline {
    pub features: Vec<Feature>,
    /// Last good support plane per sketch feature ("cached frame", F27).
    plane_cache: HashMap<String, Plane>,
}

fn tool_face_role(tool: &Solid, min: [f64; 3], size: [f64; 3]) -> impl Fn(u32) -> String + '_ {
    move |f| {
        let c = tool.face(f).map(|i| i.center).unwrap_or([0.0; 3]);
        for (axis, name) in ["x", "y", "z"].iter().enumerate() {
            if (c[axis] - min[axis]).abs() < 1e-6 {
                return format!("{name}-");
            }
            if (c[axis] - (min[axis] + size[axis])).abs() < 1e-6 {
                return format!("{name}+");
            }
        }
        format!("f{f}")
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild(&mut self) -> Rebuild {
        let mut body: Option<Labeled> = None;
        let mut reports = Vec::new();
        for i in 0..self.features.len() {
            let f = self.features[i].clone();
            let mut rep = FeatureReport {
                id: f.id.clone(),
                health: Health::Healthy,
                resolutions: vec![],
                diagnostics: vec![],
                plane: None,
            };
            if f.suppressed {
                rep.health = Health::Suppressed;
                reports.push(rep);
                continue;
            }
            match self.apply(&f, body.take(), &mut rep) {
                Ok(next) => body = next,
                Err((prev, code)) => {
                    // Failed feature: keep the previous body ("using cached geometry"), flag the feature.
                    rep.health = Health::Error;
                    rep.diagnostics.push(code);
                    body = prev;
                }
            }
            reports.push(rep);
        }
        Rebuild { body, reports }
    }

    #[allow(clippy::type_complexity)]
    fn apply(
        &mut self,
        f: &Feature,
        prev: Option<Labeled>,
        rep: &mut FeatureReport,
    ) -> Result<Option<Labeled>, (Option<Labeled>, String)> {
        let fid = f.id.as_str();
        let fail = |prev: Option<Labeled>, code: &str| Err((prev, code.to_string()));
        match &f.kind {
            FeatureKind::Extrude { profile, height } => {
                let pts: Vec<[f64; 2]> = profile.iter().map(|p| p.xy).collect();
                let Ok((solid, hist)) = Solid::extrude(&pts, *height) else {
                    return fail(prev, "KERNEL.OP.FAILED");
                };
                let mut labels = vec![Labels::new(); solid.face_count() as usize];
                let n = pts.len();
                for h in &hist {
                    let Some((SubKind::Face, d)) = h.dst else { continue };
                    let slot = &mut labels[d as usize - 1];
                    match (h.kind, h.relation) {
                        (SubKind::Edge, Relation::Generated) => {
                            // Side face of segment k: its centroid projects onto the segment midpoint.
                            let c = solid.face(d).map(|i| i.center).unwrap_or([0.0; 3]);
                            let best = (0..n)
                                .min_by(|&a, &b| {
                                    let m = |k: usize| {
                                        let (p, q) = (pts[k], pts[(k + 1) % n]);
                                        ((p[0] + q[0]) / 2.0 - c[0]).powi(2) + ((p[1] + q[1]) / 2.0 - c[1]).powi(2)
                                    };
                                    m(a).total_cmp(&m(b))
                                })
                                .unwrap();
                            slot.insert(format!("{fid}:side:{}", profile[best].curve));
                        }
                        (SubKind::Face, Relation::Unchanged | Relation::Modified) => {
                            slot.insert(format!("{fid}:cap:start"));
                        }
                        (SubKind::Face, Relation::SweepEnd) => {
                            slot.insert(format!("{fid}:cap:end"));
                        }
                        _ => {}
                    }
                }
                Ok(Some(Labeled { solid, face_labels: labels }))
            }
            FeatureKind::CutBox { min, size } => {
                let Some(prev) = prev else { return fail(None, "KERNEL.OP.FAILED") };
                let tool = match Solid::make_box(size[0], size[1], size[2])
                    .and_then(|t| t.translated(min[0], min[1], min[2]))
                {
                    Ok(t) => t,
                    Err(_) => return fail(Some(prev), "KERNEL.OP.FAILED"),
                };
                let Ok((solid, hist)) = prev.solid.cut(&tool) else { return fail(Some(prev), "KERNEL.OP.FAILED") };
                let role = tool_face_role(&tool, *min, *size);
                let labels = propagate(&prev, &solid, &hist, fid, Some(&role));
                Ok(Some(Labeled { solid, face_labels: labels }))
            }
            FeatureKind::Fillet { edges, radius } => {
                let Some(prev) = prev else { return fail(None, "KERNEL.OP.FAILED") };
                let mut idx: Vec<u32> = Vec::new();
                let mut lost = false;
                for r in edges {
                    let res = prev.resolve(r);
                    rep.resolutions.push(res.resolution);
                    match res.resolution {
                        Resolution::Lost => {
                            lost = true;
                            rep.diagnostics.push("REF.LOST".into());
                        }
                        Resolution::Ambiguous => {
                            lost = true;
                            rep.diagnostics.push("REF.AMBIGUOUS".into());
                        }
                        _ => idx.extend(res.indices),
                    }
                }
                if lost {
                    rep.health = Health::Error;
                    return Ok(Some(prev));
                }
                idx.sort();
                idx.dedup();
                let Ok((solid, hist)) = prev.solid.fillet(&idx, *radius) else {
                    return fail(Some(prev), "KERNEL.OP.FAILED");
                };
                let labels = propagate(&prev, &solid, &hist, fid, None);
                Ok(Some(Labeled { solid, face_labels: labels }))
            }
            FeatureKind::Shell { faces, thickness } => {
                let Some(prev) = prev else { return fail(None, "KERNEL.OP.FAILED") };
                let mut idx: Vec<u32> = Vec::new();
                let mut lost = false;
                for r in faces {
                    let res = prev.resolve(r);
                    rep.resolutions.push(res.resolution);
                    match res.resolution {
                        Resolution::Lost => {
                            lost = true;
                            rep.diagnostics.push("REF.LOST".into());
                        }
                        Resolution::Ambiguous => {
                            lost = true;
                            rep.diagnostics.push("REF.AMBIGUOUS".into());
                        }
                        _ => idx.extend(res.indices),
                    }
                }
                if lost {
                    rep.health = Health::Error;
                    return Ok(Some(prev));
                }
                idx.sort();
                idx.dedup();
                let Ok((solid, hist)) = prev.solid.shell(&idx, *thickness) else {
                    return fail(Some(prev), "KERNEL.OP.FAILED");
                };
                let labels = propagate(&prev, &solid, &hist, fid, None);
                Ok(Some(Labeled { solid, face_labels: labels }))
            }
            FeatureKind::Sketch { face, policy } => {
                let Some(prev) = prev else { return fail(None, "KERNEL.OP.FAILED") };
                let res = prev.resolve(face);
                rep.resolutions.push(res.resolution);
                let plane_of = |i: u32| {
                    prev.solid
                        .face(i)
                        .ok()
                        .and_then(|fi| fi.normal.map(|n| Plane { origin: fi.center, normal: n }))
                };
                let mut plane = None;
                match res.resolution {
                    Resolution::Lost => {
                        rep.health = Health::Warning;
                        rep.diagnostics.push("SKETCH.REF.LOST".into());
                    }
                    Resolution::Ambiguous => {
                        rep.health = Health::Warning;
                        rep.diagnostics.push("REF.AMBIGUOUS".into());
                    }
                    Resolution::Expanded(_) if *policy == PiecePolicy::Single => {
                        // Several pieces and the feature needs one: report, never pick silently.
                        rep.resolutions.pop();
                        rep.resolutions.push(Resolution::Ambiguous);
                        rep.health = Health::Warning;
                        rep.diagnostics.push("REF.AMBIGUOUS".into());
                    }
                    _ => {
                        let planes: Vec<Plane> = res.indices.iter().filter_map(|&i| plane_of(i)).collect();
                        // All pieces must share one support plane.
                        let coplanar = planes.windows(2).all(|w| {
                            let d: f64 = (0..3).map(|k| (w[1].origin[k] - w[0].origin[k]) * w[0].normal[k]).sum();
                            dist(w[0].normal, w[1].normal) < 1e-9 && d.abs() < 1e-6
                        });
                        if planes.len() == res.indices.len() && !planes.is_empty() && coplanar {
                            plane = Some(planes[0]);
                        } else {
                            rep.health = Health::Warning;
                            rep.diagnostics.push("REF.AMBIGUOUS".into());
                        }
                    }
                }
                match plane {
                    Some(p) => {
                        self.plane_cache.insert(f.id.clone(), p);
                        rep.plane = Some(p);
                    }
                    None => rep.plane = self.plane_cache.get(&f.id).copied(),
                }
                Ok(Some(prev))
            }
        }
    }
}
