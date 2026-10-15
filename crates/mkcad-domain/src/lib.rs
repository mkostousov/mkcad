//! Document and identifier types for mkcad.
//!
//! This crate defines the core domain model: documents, stable IDs, object references,
//! command dispatch, and diagnostic collection. It is wasm32-compatible and has no UI dependencies.

#![warn(missing_docs)]

pub mod command;
pub mod diagnostics;

pub use command::{Command, CommandBus, CommandOutcome};
pub use diagnostics::{DiagnosticsSink, Severity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_sink_basic() {
        let mut sink = DiagnosticsSink::new();
        assert_eq!(sink.all().len(), 0);

        // Emit a diagnostic (via helper for testing)
        let code = mkcad_solver::Code::ConstraintDuplicate;
        sink.emit(code.as_str(), "warning");

        assert_eq!(sink.all().len(), 1);
        assert_eq!(sink.all()[0].code, "SKETCH.CONSTRAINT.DUPLICATE");
    }

    #[test]
    fn command_outcome_serialization() {
        let outcome = CommandOutcome::Success { message: "Test".to_string() };
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("success"));
    }
}
