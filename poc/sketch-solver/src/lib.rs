//! PoC K-1 — own 2D sketch constraint solver (see `docs/spec/F11-solver-design.md`).
//!
//! Core logic is pure computation: no file system, threads or OS calls, so the crate builds for
//! `wasm32-unknown-unknown`. Transcendental functions come from `libm` for cross-platform determinism.

pub mod diag;
pub mod equations;
pub mod model;

pub use diag::{Code, Diagnostic, Severity, Suggestion};
pub use model::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_json_round_trip() {
        let c = Constraint {
            id: ConstraintId(7),
            kind: ConstraintKind::DistanceH { a: PointId(1), b: PointId(2), value: 40.0, sign: 1 },
            driving: true,
            prov: Provenance::default(),
        };
        let s = serde_json::to_string(&c).unwrap();
        assert!(s.contains("\"type\":\"distance_h\""), "{s}");
        let back: Constraint = serde_json::from_str(&s).unwrap();
        assert_eq!(back, c);
        let d = Diagnostic::new(Code::ConstraintRedundant).with_constraints(vec![ConstraintId(3), ConstraintId(1)]);
        assert_eq!(serde_json::to_value(&d).unwrap()["code"], "SKETCH.CONSTRAINT.REDUNDANT");
        assert_eq!(d.constraints, vec![ConstraintId(1), ConstraintId(3)]);
    }
}
