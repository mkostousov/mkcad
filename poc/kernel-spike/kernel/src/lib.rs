//! Safe wrapper over the OCCT shim plus a linear operation history.
//!
//! The kernel is stateless; [`Model`] keeps the sequence of [`OpRecord`]s, each with the sub-shape
//! mapping OCCT reported (modified / generated / deleted / unchanged). That mapping is the raw
//! material for topological naming (K-2).

use occt_sys::ffi;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelError(pub String);

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "kernel error: {}", self.0)
    }
}

impl std::error::Error for KernelError {}

impl From<cxx::Exception> for KernelError {
    fn from(e: cxx::Exception) -> Self {
        KernelError(e.what().to_string())
    }
}

pub type Result<T> = std::result::Result<T, KernelError>;

/// An immutable B-rep shape (single owner of the underlying `TopoDS_Shape`).
pub struct Solid(cxx::UniquePtr<ffi::Shape>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Modified,
    Generated,
    Deleted,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubKind {
    Edge,
    Face,
}

/// One input→output relation. `arg` is 0 for the primary operand, 1 for a boolean tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryEntry {
    pub relation: Relation,
    pub arg: u8,
    pub kind: SubKind,
    /// 1-based index in the operand.
    pub src: u32,
    /// Kind and 1-based index in the result; `None` when deleted. An edge may generate a face.
    pub dst: Option<(SubKind, u32)>,
}

impl From<&ffi::HistEntry> for HistoryEntry {
    fn from(h: &ffi::HistEntry) -> Self {
        HistoryEntry {
            relation: match h.kind {
                0 => Relation::Modified,
                1 => Relation::Generated,
                2 => Relation::Deleted,
                _ => Relation::Unchanged,
            },
            arg: h.arg,
            kind: if h.dim == 1 { SubKind::Edge } else { SubKind::Face },
            src: h.src,
            dst: (h.dst != 0).then(|| (if h.dst_dim == 1 { SubKind::Edge } else { SubKind::Face }, h.dst)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceInfo {
    pub area: f64,
    pub center: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInfo {
    pub length: f64,
    pub center: [f64; 3],
}

fn split(r: ffi::OpResult) -> (Solid, Vec<HistoryEntry>) {
    (Solid(r.shape), r.history.iter().map(HistoryEntry::from).collect())
}

impl Solid {
    pub fn make_box(dx: f64, dy: f64, dz: f64) -> Result<Solid> {
        Ok(Solid(ffi::make_box(dx, dy, dz)?))
    }

    /// Extrudes the polygon `points` (XY plane, implicitly closed) by `height` along +Z.
    pub fn extrude(points: &[[f64; 2]], height: f64) -> Result<(Solid, Vec<HistoryEntry>)> {
        let flat: Vec<f64> = points.iter().flatten().copied().collect();
        Ok(split(ffi::extrude_polygon(&flat, height)?))
    }

    pub fn translated(&self, x: f64, y: f64, z: f64) -> Result<Solid> {
        Ok(Solid(ffi::translate(&self.0, x, y, z)?))
    }

    /// Fillets the 1-based `edges` with a constant `radius`.
    pub fn fillet(&self, edges: &[u32], radius: f64) -> Result<(Solid, Vec<HistoryEntry>)> {
        Ok(split(ffi::fillet_edges(&self.0, edges, radius)?))
    }

    pub fn cut(&self, tool: &Solid) -> Result<(Solid, Vec<HistoryEntry>)> {
        Ok(split(ffi::boolean_cut(&self.0, &tool.0)?))
    }

    pub fn fuse(&self, tool: &Solid) -> Result<(Solid, Vec<HistoryEntry>)> {
        Ok(split(ffi::boolean_fuse(&self.0, &tool.0)?))
    }

    pub fn volume(&self) -> Result<f64> {
        Ok(ffi::volume(&self.0)?)
    }

    pub fn is_valid(&self) -> bool {
        ffi::is_valid(&self.0)
    }

    pub fn face_count(&self) -> u32 {
        ffi::count_faces(&self.0)
    }

    pub fn edge_count(&self) -> u32 {
        ffi::count_edges(&self.0)
    }

    pub fn face(&self, index: u32) -> Result<FaceInfo> {
        let p = ffi::face_props(&self.0, index)?;
        Ok(FaceInfo { area: p.area, center: [p.cx, p.cy, p.cz] })
    }

    pub fn edge(&self, index: u32) -> Result<EdgeInfo> {
        let p = ffi::edge_props(&self.0, index)?;
        Ok(EdgeInfo { length: p.length, center: [p.cx, p.cy, p.cz] })
    }

    /// First 1-based edge whose centroid is within `tol` of `c`.
    pub fn find_edge_at(&self, c: [f64; 3], tol: f64) -> Option<u32> {
        (1..=self.edge_count()).find(|&i| {
            self.edge(i).is_ok_and(|e| e.center.iter().zip(c).all(|(a, b)| (a - b).abs() <= tol))
        })
    }

    /// First 1-based face whose centroid is within `tol` of `c`.
    pub fn find_face_at(&self, c: [f64; 3], tol: f64) -> Option<u32> {
        (1..=self.face_count()).find(|&i| {
            self.face(i).is_ok_and(|f| f.center.iter().zip(c).all(|(a, b)| (a - b).abs() <= tol))
        })
    }

    pub fn export_step(&self, path: &Path) -> Result<()> {
        let p = path.to_str().ok_or_else(|| KernelError("non-UTF-8 path".into()))?;
        Ok(ffi::export_step(&self.0, p)?)
    }

    pub fn import_step(path: &Path) -> Result<Solid> {
        let p = path.to_str().ok_or_else(|| KernelError("non-UTF-8 path".into()))?;
        Ok(Solid(ffi::import_step(p)?))
    }
}

/// One recorded modelling step.
#[derive(Debug, Clone, PartialEq)]
pub struct OpRecord {
    pub name: String,
    pub params: Vec<(String, f64)>,
    pub history: Vec<HistoryEntry>,
    pub volume: f64,
    pub faces: u32,
    pub edges: u32,
}

/// A linear feature tree: the current solid plus the records of every step that produced it.
#[derive(Default)]
pub struct Model {
    current: Option<Solid>,
    steps: Vec<OpRecord>,
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solid(&self) -> Option<&Solid> {
        self.current.as_ref()
    }

    pub fn steps(&self) -> &[OpRecord] {
        &self.steps
    }

    fn commit(
        &mut self,
        name: &str,
        params: &[(&str, f64)],
        solid: Solid,
        history: Vec<HistoryEntry>,
    ) -> Result<&OpRecord> {
        let record = OpRecord {
            name: name.to_string(),
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            history,
            volume: solid.volume()?,
            faces: solid.face_count(),
            edges: solid.edge_count(),
        };
        self.current = Some(solid);
        self.steps.push(record);
        Ok(self.steps.last().unwrap())
    }

    fn require(&self) -> Result<&Solid> {
        self.current.as_ref().ok_or_else(|| KernelError("model has no solid yet".into()))
    }

    pub fn add_box(&mut self, dx: f64, dy: f64, dz: f64) -> Result<&OpRecord> {
        let s = Solid::make_box(dx, dy, dz)?;
        self.commit("box", &[("dx", dx), ("dy", dy), ("dz", dz)], s, Vec::new())
    }

    pub fn extrude(&mut self, points: &[[f64; 2]], height: f64) -> Result<&OpRecord> {
        let (s, h) = Solid::extrude(points, height)?;
        self.commit("extrude", &[("height", height)], s, h)
    }

    pub fn fillet(&mut self, edges: &[u32], radius: f64) -> Result<&OpRecord> {
        let (s, h) = self.require()?.fillet(edges, radius)?;
        self.commit("fillet", &[("radius", radius)], s, h)
    }

    pub fn cut(&mut self, tool: &Solid) -> Result<&OpRecord> {
        let (s, h) = self.require()?.cut(tool)?;
        self.commit("cut", &[], s, h)
    }

    pub fn fuse(&mut self, tool: &Solid) -> Result<&OpRecord> {
        let (s, h) = self.require()?.fuse(tool)?;
        self.commit("fuse", &[], s, h)
    }
}
