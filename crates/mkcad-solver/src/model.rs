//! Sketch data model (spec G, H.3.18): stable IDs, points, curves, constraints/dimensions, provenance.
//!
//! All collections are `BTreeMap`s keyed by stable ID so every iteration order is deterministic (F11 §3).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

id_type!(
    /// Point entity ID (document-unique, never reused).
    PointId
);
id_type!(
    /// Curve entity ID (line, circle, arc).
    CurveId
);
id_type!(
    /// Constraint or dimension ID.
    ConstraintId
);
id_type!(
    /// Sketch operation ID (H.3.18).
    OpId
);
id_type!(
    /// Composite-tool group ID (rectangle, slot, ...).
    GroupId
);

/// Reference to a geometric entity, used by `Fix` and diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityRef {
    Point(PointId),
    Curve(CurveId),
}

/// Who created / modified an entity (H.3.18).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub created_by: Option<OpId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<GroupId>,
    /// Most recent modifying operations, bounded to [`Provenance::MAX_MODIFIED`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_by: Vec<OpId>,
}

impl Provenance {
    pub const MAX_MODIFIED: usize = 8;

    pub fn touch(&mut self, op: OpId) {
        if self.modified_by.last() != Some(&op) {
            self.modified_by.push(op);
            if self.modified_by.len() > Self::MAX_MODIFIED {
                self.modified_by.remove(0);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub id: PointId,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub prov: Provenance,
}

/// Curve parameterization (F11 §1): lines are point pairs, circles centre + radius,
/// arcs centre + start + end + radius, counter-clockwise from start to end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CurveGeom {
    Line { a: PointId, b: PointId },
    Circle { center: PointId, r: f64 },
    Arc { center: PointId, start: PointId, end: PointId, r: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub id: CurveId,
    pub geom: CurveGeom,
    #[serde(default)]
    pub construction: bool,
    #[serde(default)]
    pub prov: Provenance,
}

impl Curve {
    /// Points owned by the curve in a fixed order.
    pub fn points(&self) -> Vec<PointId> {
        match self.geom {
            CurveGeom::Line { a, b } => vec![a, b],
            CurveGeom::Circle { center, .. } => vec![center],
            CurveGeom::Arc { center, start, end, .. } => vec![center, start, end],
        }
    }

    pub fn radius(&self) -> Option<f64> {
        match self.geom {
            CurveGeom::Circle { r, .. } | CurveGeom::Arc { r, .. } => Some(r),
            CurveGeom::Line { .. } => None,
        }
    }

    pub fn set_radius(&mut self, value: f64) {
        if let CurveGeom::Circle { r, .. } | CurveGeom::Arc { r, .. } = &mut self.geom {
            *r = value;
        }
    }

    pub fn center(&self) -> Option<PointId> {
        match self.geom {
            CurveGeom::Circle { center, .. } | CurveGeom::Arc { center, .. } => Some(center),
            CurveGeom::Line { .. } => None,
        }
    }

    pub fn line_points(&self) -> Option<(PointId, PointId)> {
        match self.geom {
            CurveGeom::Line { a, b } => Some((a, b)),
            _ => None,
        }
    }
}

/// Orientation sign stored on orientation-sensitive constraints (branch stability, F11 §2.5).
/// `0` means "take it from the current geometry when the constraint is added".
pub type Sign = i8;

/// Constraint and dimension kinds (F.3, F11 §5.2). Dimensions carry a `value`
/// (mm, or degrees for `Angle`) and are driving or driven via [`Constraint::driving`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintKind {
    Coincident {
        a: PointId,
        b: PointId,
    },
    PointOnLine {
        p: PointId,
        line: CurveId,
    },
    PointOnCircle {
        p: PointId,
        curve: CurveId,
    },
    Midpoint {
        p: PointId,
        line: CurveId,
    },
    Horizontal {
        line: CurveId,
    },
    Vertical {
        line: CurveId,
    },
    Parallel {
        l1: CurveId,
        l2: CurveId,
    },
    Perpendicular {
        l1: CurveId,
        l2: CurveId,
    },
    Collinear {
        l1: CurveId,
        l2: CurveId,
    },
    EqualLength {
        l1: CurveId,
        l2: CurveId,
    },
    EqualRadius {
        c1: CurveId,
        c2: CurveId,
    },
    Concentric {
        c1: CurveId,
        c2: CurveId,
    },
    /// Line tangent to a circle/arc; `side` = side of the line the centre lies on.
    Tangent {
        line: CurveId,
        curve: CurveId,
        #[serde(default)]
        side: Sign,
    },
    /// Circle/arc tangent to circle/arc; `side` +1 external, -1 internal.
    TangentCurves {
        c1: CurveId,
        c2: CurveId,
        #[serde(default)]
        side: Sign,
    },
    /// `p2` is the mirror image of `p1` about the axis line.
    Symmetric {
        p1: PointId,
        p2: PointId,
        axis: CurveId,
    },
    /// True mirror of both endpoints (N.2a-2): `l2.a = mirror(l1.a)`, `l2.b = mirror(l1.b)`.
    SymmetricLines {
        l1: CurveId,
        l2: CurveId,
        axis: CurveId,
    },
    /// Removes the entity's variables from the unknown vector (F66 semantics, recorded as a constraint).
    Fix {
        entity: EntityRef,
    },
    Distance {
        a: PointId,
        b: PointId,
        value: f64,
    },
    DistanceH {
        a: PointId,
        b: PointId,
        value: f64,
        #[serde(default)]
        sign: Sign,
    },
    DistanceV {
        a: PointId,
        b: PointId,
        value: f64,
        #[serde(default)]
        sign: Sign,
    },
    DistancePointLine {
        p: PointId,
        line: CurveId,
        value: f64,
        #[serde(default)]
        side: Sign,
    },
    Radius {
        curve: CurveId,
        value: f64,
    },
    Diameter {
        curve: CurveId,
        value: f64,
    },
    /// Directed angle from `l1` to `l2`, `degrees` in (0, 180); `sign` is the rotation sense.
    Angle {
        l1: CurveId,
        l2: CurveId,
        degrees: f64,
        #[serde(default)]
        sign: Sign,
    },
}

impl ConstraintKind {
    /// Short stable name used in diagnostics and logs.
    pub fn name(&self) -> &'static str {
        use ConstraintKind::*;
        match self {
            Coincident { .. } => "coincident",
            PointOnLine { .. } => "point_on_line",
            PointOnCircle { .. } => "point_on_circle",
            Midpoint { .. } => "midpoint",
            Horizontal { .. } => "horizontal",
            Vertical { .. } => "vertical",
            Parallel { .. } => "parallel",
            Perpendicular { .. } => "perpendicular",
            Collinear { .. } => "collinear",
            EqualLength { .. } => "equal_length",
            EqualRadius { .. } => "equal_radius",
            Concentric { .. } => "concentric",
            Tangent { .. } => "tangent",
            TangentCurves { .. } => "tangent_curves",
            Symmetric { .. } => "symmetric",
            SymmetricLines { .. } => "symmetric_lines",
            Fix { .. } => "fix",
            Distance { .. } => "distance",
            DistanceH { .. } => "distance_h",
            DistanceV { .. } => "distance_v",
            DistancePointLine { .. } => "distance_point_line",
            Radius { .. } => "radius",
            Diameter { .. } => "diameter",
            Angle { .. } => "angle",
        }
    }

    /// Dimension value, if this is a dimension.
    pub fn value(&self) -> Option<f64> {
        use ConstraintKind::*;
        match *self {
            Distance { value, .. }
            | DistanceH { value, .. }
            | DistanceV { value, .. }
            | DistancePointLine { value, .. }
            | Radius { value, .. }
            | Diameter { value, .. } => Some(value),
            Angle { degrees, .. } => Some(degrees),
            _ => None,
        }
    }

    pub fn is_dimension(&self) -> bool {
        self.value().is_some()
    }

    pub fn set_value(&mut self, v: f64) {
        use ConstraintKind::*;
        match self {
            Distance { value, .. }
            | DistanceH { value, .. }
            | DistanceV { value, .. }
            | DistancePointLine { value, .. }
            | Radius { value, .. }
            | Diameter { value, .. } => *value = v,
            Angle { degrees, .. } => *degrees = v,
            _ => {}
        }
    }

    /// Entities referenced directly, in argument order ("first selected" first, F62).
    pub fn entities(&self) -> Vec<EntityRef> {
        use ConstraintKind::*;
        use EntityRef::{Curve as C, Point as P};
        match *self {
            Coincident { a, b } | Distance { a, b, .. } | DistanceH { a, b, .. } | DistanceV { a, b, .. } => {
                vec![P(a), P(b)]
            }
            PointOnLine { p, line } | Midpoint { p, line } | DistancePointLine { p, line, .. } => vec![P(p), C(line)],
            PointOnCircle { p, curve } => vec![P(p), C(curve)],
            Horizontal { line } | Vertical { line } => vec![C(line)],
            Parallel { l1, l2 }
            | Perpendicular { l1, l2 }
            | Collinear { l1, l2 }
            | EqualLength { l1, l2 }
            | Angle { l1, l2, .. } => vec![C(l1), C(l2)],
            EqualRadius { c1, c2 } | Concentric { c1, c2 } | TangentCurves { c1, c2, .. } => vec![C(c1), C(c2)],
            Tangent { line, curve, .. } => vec![C(line), C(curve)],
            Symmetric { p1, p2, axis } => vec![P(p1), P(p2), C(axis)],
            SymmetricLines { l1, l2, axis } => vec![C(l1), C(l2), C(axis)],
            Fix { entity } => vec![entity],
            Radius { curve, .. } | Diameter { curve, .. } => vec![C(curve)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub id: ConstraintId,
    pub kind: ConstraintKind,
    /// Driven (reference) dimensions contribute no equation; their value follows the geometry.
    #[serde(default = "yes")]
    pub driving: bool,
    #[serde(default)]
    pub prov: Provenance,
}

fn yes() -> bool {
    true
}

/// A sketch snapshot: the authoritative solved coordinates plus constraints (N.2a-5).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sketch {
    pub points: BTreeMap<PointId, Point>,
    pub curves: BTreeMap<CurveId, Curve>,
    pub constraints: BTreeMap<ConstraintId, Constraint>,
}

impl Sketch {
    pub fn point(&self, id: PointId) -> &Point {
        &self.points[&id]
    }

    pub fn curve(&self, id: CurveId) -> &Curve {
        &self.curves[&id]
    }

    pub fn xy(&self, id: PointId) -> (f64, f64) {
        let p = &self.points[&id];
        (p.x, p.y)
    }

    /// Points of an entity (a point itself, or the points a curve owns).
    pub fn entity_points(&self, e: EntityRef) -> Vec<PointId> {
        match e {
            EntityRef::Point(p) => vec![p],
            EntityRef::Curve(c) => self.curves[&c].points(),
        }
    }

    /// Curves that own the point, ascending by ID.
    pub fn curves_of_point(&self, p: PointId) -> Vec<CurveId> {
        self.curves.values().filter(|c| c.points().contains(&p)).map(|c| c.id).collect()
    }

    /// Checks that every reference resolves to an entity of the right kind.
    pub fn check_refs(&self, kind: &ConstraintKind) -> bool {
        use ConstraintKind::*;
        let is_line = |c: &CurveId| self.curves.get(c).is_some_and(|c| c.line_points().is_some());
        let is_round = |c: &CurveId| self.curves.get(c).is_some_and(|c| c.radius().is_some());
        let refs_ok = kind.entities().iter().all(|e| match e {
            EntityRef::Point(p) => self.points.contains_key(p),
            EntityRef::Curve(c) => self.curves.contains_key(c),
        });
        refs_ok
            && match kind {
                PointOnLine { line, .. } | Midpoint { line, .. } | DistancePointLine { line, .. } => is_line(line),
                Horizontal { line } | Vertical { line } => is_line(line),
                Parallel { l1, l2 }
                | Perpendicular { l1, l2 }
                | Collinear { l1, l2 }
                | EqualLength { l1, l2 }
                | Angle { l1, l2, .. } => is_line(l1) && is_line(l2) && l1 != l2,
                PointOnCircle { curve, .. } | Radius { curve, .. } | Diameter { curve, .. } => is_round(curve),
                EqualRadius { c1, c2 } | Concentric { c1, c2 } | TangentCurves { c1, c2, .. } => {
                    is_round(c1) && is_round(c2) && c1 != c2
                }
                Tangent { line, curve, .. } => is_line(line) && is_round(curve),
                Symmetric { p1, p2, axis } => is_line(axis) && p1 != p2,
                SymmetricLines { l1, l2, axis } => is_line(l1) && is_line(l2) && is_line(axis) && l1 != l2,
                Coincident { a, b } | Distance { a, b, .. } | DistanceH { a, b, .. } | DistanceV { a, b, .. } => a != b,
                Fix { .. } => true,
            }
    }
}
