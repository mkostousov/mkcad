//! Sketch command layer: every command validates, solves, and either commits exactly one [`SketchOp`]
//! (one undo step, H.3.18) or is rejected with diagnostics and leaves the document unchanged (F.3, S-11).

use crate::diag::{Code, Diagnostic, Suggestion};
use crate::equations::{measure, resolve_orientation};
use crate::model::*;
use crate::solver::{EditContext, Goal, SolveOptions, SolveResult, Status, solve};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Any document ID (single never-reused counter for all kinds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnyId {
    Point(PointId),
    Curve(CurveId),
    Constraint(ConstraintId),
}

/// One sketch command (H.3.18); same granularity as undo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SketchOp {
    pub id: OpId,
    pub seq: u64,
    pub kind: String,
    pub created: Vec<AnyId>,
    pub modified: Vec<AnyId>,
    pub deleted: Vec<AnyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<GroupId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct Applied {
    pub op: SketchOp,
    pub solve: SolveResult,
}

#[derive(Clone, Debug)]
pub struct Rejection {
    pub diagnostics: Vec<Diagnostic>,
    pub solve: Option<Box<SolveResult>>,
}

impl Rejection {
    fn code(code: Code) -> Rejection {
        Rejection { diagnostics: vec![Diagnostic::new(code)], solve: None }
    }

    pub fn has(&self, code: Code) -> bool {
        self.diagnostics.iter().any(|d| d.code == code)
    }
}

pub type OpResult = Result<Applied, Rejection>;

/// End point of a new line: an existing point (shared, F.2.2) or new coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointRef {
    Existing(PointId),
    New(f64, f64),
}

#[derive(Clone, Debug)]
struct UndoEntry {
    before: Sketch,
    after: Sketch,
    op: SketchOp,
}

#[derive(Clone, Debug, Default)]
pub struct SketchDoc {
    pub sketch: Sketch,
    next_id: u64,
    seq: u64,
    pub log: Vec<SketchOp>,
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
    pub opts: SolveOptions,
}

/// Transaction being built by a command.
struct Tx {
    sketch: Sketch,
    op: SketchOp,
}

impl SketchDoc {
    pub fn new() -> SketchDoc {
        SketchDoc { next_id: 1, ..Default::default() }
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    fn alloc(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn begin(&mut self, kind: &str) -> Tx {
        let id = OpId(self.alloc());
        self.seq += 1;
        Tx {
            sketch: self.sketch.clone(),
            op: SketchOp {
                id,
                seq: self.seq,
                kind: kind.to_string(),
                created: Vec::new(),
                modified: Vec::new(),
                deleted: Vec::new(),
                group_id: None,
                diagnostics: Vec::new(),
            },
        }
    }

    fn prov(tx: &Tx) -> Provenance {
        Provenance { created_by: Some(tx.op.id), group_id: tx.op.group_id, modified_by: Vec::new() }
    }

    fn new_point(&mut self, tx: &mut Tx, x: f64, y: f64) -> PointId {
        let id = PointId(self.alloc());
        tx.sketch.points.insert(id, Point { id, x, y, prov: Self::prov(tx) });
        tx.op.created.push(AnyId::Point(id));
        id
    }

    fn new_curve(&mut self, tx: &mut Tx, geom: CurveGeom) -> CurveId {
        let id = CurveId(self.alloc());
        tx.sketch.curves.insert(id, Curve { id, geom, construction: false, prov: Self::prov(tx) });
        tx.op.created.push(AnyId::Curve(id));
        id
    }

    fn new_constraint(&mut self, tx: &mut Tx, mut kind: ConstraintKind, driving: bool) -> ConstraintId {
        resolve_orientation(&mut kind, &tx.sketch);
        let id = ConstraintId(self.alloc());
        tx.sketch.constraints.insert(id, Constraint { id, kind, driving, prov: Self::prov(tx) });
        tx.op.created.push(AnyId::Constraint(id));
        id
    }

    fn point_ref(&mut self, tx: &mut Tx, p: PointRef) -> Result<PointId, Rejection> {
        match p {
            PointRef::Existing(id) if tx.sketch.points.contains_key(&id) => Ok(id),
            PointRef::Existing(_) => Err(Rejection::code(Code::InputInvalid)),
            PointRef::New(x, y) => Ok(self.new_point(tx, x, y)),
        }
    }

    /// Solves the transaction's sketch and commits on success. On failure the document is unchanged
    /// (IDs allocated by the attempt are simply never used — they are not reused, F.1 stable IDs).
    fn commit(&mut self, mut tx: Tx, edit: &EditContext, fail_code: Code, added: Option<ConstraintId>) -> OpResult {
        let res = solve(&tx.sketch, edit, &self.opts);
        let reject = |diags: Vec<Diagnostic>, res: SolveResult| {
            Err(Rejection { diagnostics: diags, solve: Some(Box::new(res)) })
        };
        match res.status {
            Status::Conflict => {
                let mut diags = vec![Diagnostic::new(fail_code).with_constraints(res.conflicts.clone())];
                diags.extend(res.diagnostics.iter().filter(|d| d.code == Code::DimInvalidValue).cloned());
                return reject(diags, res);
            }
            Status::NonConvergent => return reject(vec![Diagnostic::new(Code::SolveNonconvergent)], res),
            Status::Redundant => {
                let mut d = Diagnostic::new(Code::ConstraintRedundant);
                let set = res.redundant.iter().find(|(id, _)| Some(*id) == added).or(res.redundant.first());
                if let Some((_, set)) = set {
                    d = d.with_constraints(set.clone());
                }
                let is_dim = added.is_some_and(|id| tx.sketch.constraints[&id].kind.is_dimension());
                d.suggestion = Some(if is_dim { Suggestion::MakeDriven } else { Suggestion::DeleteOne });
                return reject(vec![d], res);
            }
            _ => {}
        }
        if res.has(Code::DimInvalidValue) {
            let diags = res.diagnostics.iter().filter(|d| d.code == Code::DimInvalidValue).cloned().collect();
            return reject(diags, res);
        }
        // Record what the solve moved.
        let before = &self.sketch;
        let mut modified: BTreeSet<AnyId> = tx.op.modified.iter().copied().collect();
        for (id, p) in &res.sketch.points {
            if before.points.get(id).is_some_and(|q| q.x != p.x || q.y != p.y) {
                modified.insert(AnyId::Point(*id));
            }
        }
        for (id, c) in &res.sketch.curves {
            if before.curves.get(id).is_some_and(|q| q.radius() != c.radius()) {
                modified.insert(AnyId::Curve(*id));
            }
        }
        for (id, c) in &res.sketch.constraints {
            if before.constraints.get(id).is_some_and(|q| q.kind != c.kind || q.driving != c.driving) {
                modified.insert(AnyId::Constraint(*id));
            }
        }
        for d in &tx.op.deleted {
            modified.remove(d);
        }
        tx.op.modified = modified.into_iter().collect();
        tx.op.diagnostics.extend(res.diagnostics.iter().filter(|d| d.code == Code::GeometryDegenerate).cloned());
        let mut after = res.sketch.clone();
        let op_id = tx.op.id;
        for m in &tx.op.modified {
            match m {
                AnyId::Point(id) => after.points.get_mut(id).unwrap().prov.touch(op_id),
                AnyId::Curve(id) => after.curves.get_mut(id).unwrap().prov.touch(op_id),
                AnyId::Constraint(id) => after.constraints.get_mut(id).unwrap().prov.touch(op_id),
            }
        }
        let entry = UndoEntry { before: self.sketch.clone(), after: after.clone(), op: tx.op.clone() };
        self.sketch = after;
        self.undo.push(entry);
        self.redo.clear();
        self.log.push(tx.op.clone());
        let mut res = res;
        res.sketch = self.sketch.clone();
        Ok(Applied { op: tx.op, solve: res })
    }

    // ----------------------------------------------------------------------------------------------------
    // Geometry commands

    pub fn add_point(&mut self, x: f64, y: f64) -> OpResult {
        let mut tx = self.begin("point");
        self.new_point(&mut tx, x, y);
        self.commit(tx, &EditContext::default(), Code::SolveConflict, None)
    }

    pub fn add_line(&mut self, a: PointRef, b: PointRef) -> OpResult {
        let mut tx = self.begin("line");
        let pa = self.point_ref(&mut tx, a)?;
        let pb = self.point_ref(&mut tx, b)?;
        if pa == pb {
            return Err(Rejection::code(Code::InputInvalid));
        }
        self.new_curve(&mut tx, CurveGeom::Line { a: pa, b: pb });
        self.commit(tx, &EditContext::default(), Code::SolveConflict, None)
    }

    pub fn add_circle(&mut self, center: PointRef, r: f64) -> OpResult {
        if !(r.is_finite() && r > 0.0) {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        let mut tx = self.begin("circle");
        let c = self.point_ref(&mut tx, center)?;
        self.new_curve(&mut tx, CurveGeom::Circle { center: c, r });
        self.commit(tx, &EditContext::default(), Code::SolveConflict, None)
    }

    /// Arc counter-clockwise from `start` to `end`; the radius is taken from `start`.
    pub fn add_arc(&mut self, center: PointRef, start: PointRef, end: PointRef) -> OpResult {
        let mut tx = self.begin("arc");
        let c = self.point_ref(&mut tx, center)?;
        let s = self.point_ref(&mut tx, start)?;
        let e = self.point_ref(&mut tx, end)?;
        let (pc, ps) = (tx.sketch.xy(c), tx.sketch.xy(s));
        let r = libm::hypot(ps.0 - pc.0, ps.1 - pc.1);
        if r <= 0.0 || s == e {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        self.new_curve(&mut tx, CurveGeom::Arc { center: c, start: s, end: e, r });
        self.commit(tx, &EditContext::default(), Code::SolveConflict, None)
    }

    /// 2-point rectangle (F36): 4 lines with shared corners + 2 Horizontal + 2 Vertical, one group.
    /// Created IDs, in order: 4 corners (p1, (x2,y1), p2, (x1,y2)), lines bottom/right/top/left, H, V, H, V.
    pub fn add_rectangle(&mut self, p1: (f64, f64), p2: (f64, f64)) -> OpResult {
        if p1.0 == p2.0 || p1.1 == p2.1 {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        let mut tx = self.begin("rectangle");
        tx.op.group_id = Some(GroupId(self.alloc()));
        let c = [
            self.new_point(&mut tx, p1.0, p1.1),
            self.new_point(&mut tx, p2.0, p1.1),
            self.new_point(&mut tx, p2.0, p2.1),
            self.new_point(&mut tx, p1.0, p2.1),
        ];
        let l: Vec<CurveId> =
            (0..4).map(|i| self.new_curve(&mut tx, CurveGeom::Line { a: c[i], b: c[(i + 1) % 4] })).collect();
        self.new_constraint(&mut tx, ConstraintKind::Horizontal { line: l[0] }, true);
        self.new_constraint(&mut tx, ConstraintKind::Vertical { line: l[1] }, true);
        self.new_constraint(&mut tx, ConstraintKind::Horizontal { line: l[2] }, true);
        self.new_constraint(&mut tx, ConstraintKind::Vertical { line: l[3] }, true);
        self.commit(tx, &EditContext::default(), Code::ConstraintConflict, None)
    }

    // ----------------------------------------------------------------------------------------------------
    // Constraints and dimensions

    fn valid_value(kind: &ConstraintKind) -> bool {
        match (kind, kind.value()) {
            (_, None) => true,
            (ConstraintKind::Angle { .. }, Some(v)) => v.is_finite() && v > 0.0 && v < 180.0,
            (_, Some(v)) => v.is_finite() && v > 0.0,
        }
    }

    /// Argument-order-insensitive identity of a geometric constraint (dimensions are never duplicates;
    /// a second dimension on the same entities is redundant instead).
    fn same_constraint(a: &ConstraintKind, b: &ConstraintKind) -> bool {
        use ConstraintKind::*;
        if a.is_dimension() || b.is_dimension() || std::mem::discriminant(a) != std::mem::discriminant(b) {
            return false;
        }
        let sym = matches!(
            a,
            Coincident { .. }
                | Parallel { .. }
                | Perpendicular { .. }
                | Collinear { .. }
                | EqualLength { .. }
                | EqualRadius { .. }
                | Concentric { .. }
                | TangentCurves { .. }
        );
        let (ea, mut eb) = (a.entities(), b.entities());
        if ea == eb {
            return true;
        }
        if sym {
            eb.reverse();
            return ea == eb;
        }
        false
    }

    /// Adds a constraint or dimension. Rejections: `SKETCH.INPUT.INVALID`, `SKETCH.DIM.INVALID_VALUE`,
    /// `SKETCH.CONSTRAINT.DUPLICATE`, `SKETCH.CONSTRAINT.REDUNDANT` (+ make_driven), `SKETCH.CONSTRAINT.CONFLICT`.
    pub fn add_constraint(&mut self, kind: ConstraintKind, driving: bool) -> OpResult {
        if !self.sketch.check_refs(&kind) {
            return Err(Rejection::code(Code::InputInvalid));
        }
        if !Self::valid_value(&kind) {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        if let Some(dup) = self.sketch.constraints.values().find(|c| Self::same_constraint(&c.kind, &kind)) {
            let d = Diagnostic::new(Code::ConstraintDuplicate).with_constraints(vec![dup.id]);
            return Err(Rejection { diagnostics: vec![d], solve: None });
        }
        let mut tx = self.begin(if kind.is_dimension() { "dimension" } else { "constraint" });
        let edit = if driving { EditContext::for_constraint(&kind, &tx.sketch) } else { EditContext::default() };
        let id = self.new_constraint(&mut tx, kind, driving);
        self.commit(tx, &edit, Code::ConstraintConflict, Some(id))
    }

    /// Places a dimension at its measured value (F51: placing records the current value; typing a new
    /// value is a separate, undoable [`SketchDoc::set_dimension`]).
    pub fn add_dimension_measured(&mut self, mut kind: ConstraintKind, driving: bool) -> OpResult {
        if !self.sketch.check_refs(&kind) {
            return Err(Rejection::code(Code::InputInvalid));
        }
        let v = measure(&kind, &self.sketch).ok_or(Rejection::code(Code::InputInvalid))?;
        kind.set_value(v);
        self.add_constraint(kind, driving)
    }

    /// Edits a driving dimension. Invalid values and conflicts leave the document (and undo stack)
    /// unchanged: `SKETCH.DIM.INVALID_VALUE`, `SKETCH.SOLVE.CONFLICT`.
    pub fn set_dimension(&mut self, id: ConstraintId, value: f64) -> OpResult {
        let Some(c) = self.sketch.constraints.get(&id) else {
            return Err(Rejection::code(Code::InputInvalid));
        };
        let mut kind = c.kind.clone();
        if !kind.is_dimension() || !c.driving {
            return Err(Rejection::code(Code::InputInvalid));
        }
        kind.set_value(value);
        if !Self::valid_value(&kind) {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        let mut tx = self.begin("edit_dimension");
        let edit = EditContext::for_constraint(&kind, &tx.sketch);
        tx.sketch.constraints.get_mut(&id).unwrap().kind = kind;
        tx.op.modified.push(AnyId::Constraint(id));
        self.commit(tx, &edit, Code::SolveConflict, Some(id))
    }

    /// Switches a dimension between driving and driven (F38).
    pub fn set_driving(&mut self, id: ConstraintId, driving: bool) -> OpResult {
        let Some(c) = self.sketch.constraints.get(&id) else {
            return Err(Rejection::code(Code::InputInvalid));
        };
        if !c.kind.is_dimension() {
            return Err(Rejection::code(Code::InputInvalid));
        }
        let mut tx = self.begin("edit_dimension");
        let c = tx.sketch.constraints.get_mut(&id).unwrap();
        c.driving = driving;
        if driving {
            // Becoming driving keeps the geometry: start from the measured value.
            let v = measure(&c.kind, &self.sketch).expect("dimension");
            c.kind.set_value(v);
        }
        tx.op.modified.push(AnyId::Constraint(id));
        self.commit(tx, &EditContext::default(), Code::ConstraintConflict, Some(id))
    }

    // ----------------------------------------------------------------------------------------------------
    // Edits

    /// Deletes entities and constraints. Deleting a curve removes the constraints that reference it and
    /// its points not used elsewhere (F95); attached curves stay.
    pub fn delete(&mut self, ids: &[AnyId]) -> OpResult {
        let mut tx = self.begin("delete");
        let mut del_curves: BTreeSet<CurveId> = BTreeSet::new();
        let mut del_points: BTreeSet<PointId> = BTreeSet::new();
        let mut del_cons: BTreeSet<ConstraintId> = BTreeSet::new();
        for id in ids {
            match *id {
                AnyId::Curve(c) if tx.sketch.curves.contains_key(&c) => {
                    del_curves.insert(c);
                }
                AnyId::Point(p) if tx.sketch.points.contains_key(&p) => {
                    del_points.insert(p);
                    del_curves.extend(tx.sketch.curves_of_point(p));
                }
                AnyId::Constraint(c) if tx.sketch.constraints.contains_key(&c) => {
                    del_cons.insert(c);
                }
                _ => return Err(Rejection::code(Code::InputInvalid)),
            }
        }
        for c in &del_curves {
            for p in tx.sketch.curve(*c).points() {
                if tx.sketch.curves_of_point(p).iter().all(|o| del_curves.contains(o)) {
                    del_points.insert(p);
                }
            }
        }
        for c in tx.sketch.constraints.values() {
            if c.kind.entities().iter().any(|e| match e {
                EntityRef::Point(p) => del_points.contains(p),
                EntityRef::Curve(k) => del_curves.contains(k),
            }) {
                del_cons.insert(c.id);
            }
        }
        for c in &del_cons {
            tx.sketch.constraints.remove(c);
            tx.op.deleted.push(AnyId::Constraint(*c));
        }
        for c in &del_curves {
            tx.sketch.curves.remove(c);
            tx.op.deleted.push(AnyId::Curve(*c));
        }
        for p in &del_points {
            tx.sketch.points.remove(p);
            tx.op.deleted.push(AnyId::Point(*p));
        }
        self.commit(tx, &EditContext::default(), Code::SolveConflict, None)
    }

    /// Drags a point towards (x, y) (F231). A point with no DOF left does not move and the command
    /// returns `SKETCH.MOVE.CONSTRAINED` instead of a silent success (S-16).
    pub fn drag(&mut self, p: PointId, x: f64, y: f64) -> OpResult {
        if !self.sketch.points.contains_key(&p) {
            return Err(Rejection::code(Code::InputInvalid));
        }
        let tx = self.begin("drag");
        let edit = EditContext::for_drag(p, x, y);
        let probe = solve(&tx.sketch, &edit, &self.opts);
        let moved = probe.sketch.xy(p) != self.sketch.xy(p);
        if probe.is_ok() && !moved && (x, y) != self.sketch.xy(p) {
            return Err(Rejection {
                diagnostics: vec![Diagnostic::new(Code::MoveConstrained)],
                solve: Some(Box::new(probe)),
            });
        }
        self.commit(tx, &edit, Code::SolveConflict, None)
    }

    /// Rotates curves (with their points) about a centre. H/V constraints on rotated lines are swapped for
    /// quarter turns and otherwise removed — never silently: `SKETCH.CONSTRAINT.REMOVED` lists them (S-08).
    pub fn rotate(&mut self, curves: &[CurveId], center: (f64, f64), degrees: f64) -> OpResult {
        if curves.iter().any(|c| !self.sketch.curves.contains_key(c)) || !degrees.is_finite() {
            return Err(Rejection::code(Code::InputInvalid));
        }
        let mut tx = self.begin("rotate");
        let set: BTreeSet<CurveId> = curves.iter().copied().collect();
        let pts: BTreeSet<PointId> = curves.iter().flat_map(|c| tx.sketch.curve(*c).points()).collect();
        let t = degrees.to_radians();
        let (s, c) = (libm::sin(t), libm::cos(t));
        for p in &pts {
            let q = tx.sketch.points.get_mut(p).unwrap();
            let (dx, dy) = (q.x - center.0, q.y - center.1);
            q.x = center.0 + c * dx - s * dy;
            q.y = center.1 + s * dx + c * dy;
        }
        let quarter = (degrees / 90.0).round();
        let is_quarter = (degrees / 90.0 - quarter).abs() < 1e-12;
        let odd = is_quarter && (quarter as i64).rem_euclid(2) == 1;
        let mut removed = Vec::new();
        let ids: Vec<ConstraintId> = tx.sketch.constraints.keys().copied().collect();
        for id in ids {
            let kind = tx.sketch.constraints[&id].kind.clone();
            let line = match kind {
                ConstraintKind::Horizontal { line } | ConstraintKind::Vertical { line } => line,
                _ => continue,
            };
            if !set.contains(&line) || (is_quarter && !odd) {
                continue;
            }
            if odd {
                let k = &mut tx.sketch.constraints.get_mut(&id).unwrap().kind;
                *k = match kind {
                    ConstraintKind::Horizontal { line } => ConstraintKind::Vertical { line },
                    _ => ConstraintKind::Horizontal { line },
                };
                tx.op.modified.push(AnyId::Constraint(id));
            } else {
                tx.sketch.constraints.remove(&id);
                tx.op.deleted.push(AnyId::Constraint(id));
                removed.push(id);
            }
        }
        if !removed.is_empty() {
            tx.op.diagnostics.push(Diagnostic::new(Code::ConstraintRemoved).with_constraints(removed));
        }
        let goals = pts.iter().map(|&p| Goal::KeepPoint { p }).collect();
        self.commit(tx, &EditContext { goals, moving: Vec::new() }, Code::SolveConflict, None)
    }

    /// Uniform scale of curves about a centre. Dimensions whose entities all scale keep existing and get
    /// scaled values (never silently deleted, S-08); radii scale too.
    pub fn scale(&mut self, curves: &[CurveId], center: (f64, f64), factor: f64) -> OpResult {
        if curves.iter().any(|c| !self.sketch.curves.contains_key(c)) {
            return Err(Rejection::code(Code::InputInvalid));
        }
        if !(factor.is_finite() && factor > 0.0) {
            return Err(Rejection::code(Code::DimInvalidValue));
        }
        let mut tx = self.begin("scale");
        let set: BTreeSet<CurveId> = curves.iter().copied().collect();
        let pts: BTreeSet<PointId> = curves.iter().flat_map(|c| tx.sketch.curve(*c).points()).collect();
        for p in &pts {
            let q = tx.sketch.points.get_mut(p).unwrap();
            q.x = center.0 + factor * (q.x - center.0);
            q.y = center.1 + factor * (q.y - center.1);
        }
        for c in &set {
            let cv = tx.sketch.curves.get_mut(c).unwrap();
            if let Some(r) = cv.radius() {
                cv.set_radius(r * factor);
            }
        }
        let ids: Vec<ConstraintId> = tx.sketch.constraints.keys().copied().collect();
        for id in ids {
            let con = &tx.sketch.constraints[&id];
            let inside = con.kind.entities().iter().all(|e| match e {
                EntityRef::Point(p) => pts.contains(p),
                EntityRef::Curve(c) => set.contains(c),
            });
            if inside && con.kind.is_dimension() && !matches!(con.kind, ConstraintKind::Angle { .. }) {
                let k = &mut tx.sketch.constraints.get_mut(&id).unwrap().kind;
                k.set_value(k.value().unwrap() * factor);
                tx.op.modified.push(AnyId::Constraint(id));
            }
        }
        let goals = pts.iter().map(|&p| Goal::KeepPoint { p }).collect();
        self.commit(tx, &EditContext { goals, moving: Vec::new() }, Code::SolveConflict, None)
    }

    // ----------------------------------------------------------------------------------------------------
    // Undo / redo (snapshot based: redo is bit-identical by construction, SK-13)

    pub fn undo(&mut self) -> Option<SketchOp> {
        let e = self.undo.pop()?;
        self.sketch = e.before.clone();
        self.log.pop();
        let op = e.op.clone();
        self.redo.push(e);
        Some(op)
    }

    pub fn redo(&mut self) -> Option<SketchOp> {
        let e = self.redo.pop()?;
        self.sketch = e.after.clone();
        self.log.push(e.op.clone());
        let op = e.op.clone();
        self.undo.push(e);
        Some(op)
    }

    /// Re-solves the current document without changing it (status, DOF, per-entity DOF).
    pub fn analyze(&self) -> SolveResult {
        solve(&self.sketch, &EditContext::default(), &self.opts)
    }
}
