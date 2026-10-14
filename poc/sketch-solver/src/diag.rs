//! Diagnostic codes (H.5) — machine codes, never display strings. Tests assert codes only.

use crate::model::{ConstraintId, EntityRef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Code {
    #[serde(rename = "SKETCH.CONSTRAINT.DUPLICATE")]
    ConstraintDuplicate,
    #[serde(rename = "SKETCH.CONSTRAINT.REDUNDANT")]
    ConstraintRedundant,
    #[serde(rename = "SKETCH.CONSTRAINT.CONFLICT")]
    ConstraintConflict,
    #[serde(rename = "SKETCH.SOLVE.CONFLICT")]
    SolveConflict,
    #[serde(rename = "SKETCH.SOLVE.NONCONVERGENT")]
    SolveNonconvergent,
    #[serde(rename = "SKETCH.DIM.INVALID_VALUE")]
    DimInvalidValue,
    #[serde(rename = "SKETCH.MOVE.CONSTRAINED")]
    MoveConstrained,
    #[serde(rename = "SKETCH.CONSTRAINT.REMOVED")]
    ConstraintRemoved,
    /// Proposed addition to H.5 (SK-06): solved, but the geometry is degenerate (folded corner).
    #[serde(rename = "SKETCH.GEOMETRY.DEGENERATE")]
    GeometryDegenerate,
    /// Proposed addition to H.5 (F.8): exact duplicate curves; tolerated, one region.
    #[serde(rename = "SKETCH.GEOMETRY.DUPLICATE")]
    GeometryDuplicate,
    /// Request references a missing entity or an entity of the wrong kind (API misuse).
    #[serde(rename = "SKETCH.INPUT.INVALID")]
    InputInvalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::ConstraintDuplicate => "SKETCH.CONSTRAINT.DUPLICATE",
            Code::ConstraintRedundant => "SKETCH.CONSTRAINT.REDUNDANT",
            Code::ConstraintConflict => "SKETCH.CONSTRAINT.CONFLICT",
            Code::SolveConflict => "SKETCH.SOLVE.CONFLICT",
            Code::SolveNonconvergent => "SKETCH.SOLVE.NONCONVERGENT",
            Code::DimInvalidValue => "SKETCH.DIM.INVALID_VALUE",
            Code::MoveConstrained => "SKETCH.MOVE.CONSTRAINED",
            Code::ConstraintRemoved => "SKETCH.CONSTRAINT.REMOVED",
            Code::GeometryDegenerate => "SKETCH.GEOMETRY.DEGENERATE",
            Code::GeometryDuplicate => "SKETCH.GEOMETRY.DUPLICATE",
            Code::InputInvalid => "SKETCH.INPUT.INVALID",
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            Code::MoveConstrained => Severity::Info,
            Code::ConstraintRemoved | Code::GeometryDegenerate | Code::GeometryDuplicate => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// Remedy the UI can offer (F38: "make driven").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Suggestion {
    MakeDriven,
    DeleteOne,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: Code,
    /// Constraints taking part (e.g. the minimal redundant or conflicting set), ascending by ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<Suggestion>,
}

impl Diagnostic {
    pub fn new(code: Code) -> Self {
        Diagnostic { code, constraints: Vec::new(), entities: Vec::new(), suggestion: None }
    }

    pub fn with_constraints(mut self, mut ids: Vec<ConstraintId>) -> Self {
        ids.sort();
        ids.dedup();
        self.constraints = ids;
        self
    }

    pub fn with_entities(mut self, mut ids: Vec<EntityRef>) -> Self {
        ids.sort();
        ids.dedup();
        self.entities = ids;
        self
    }

    pub fn with_suggestion(mut self, s: Suggestion) -> Self {
        self.suggestion = Some(s);
        self
    }

    pub fn severity(&self) -> Severity {
        self.code.severity()
    }
}
