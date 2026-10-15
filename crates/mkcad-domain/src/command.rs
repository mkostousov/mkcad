//! Command trait, enumeration, dispatch bus, and transaction stack.
//!
//! The CommandBus is the entry point for user actions. Commands are dispatched,
//! executed, and results are collected with optional undo/redo support.

use serde::{Deserialize, Serialize};

/// Document-level settings (display units, etc.).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocumentSettings {
    /// Display unit for length (mm, in, etc.).
    pub length_unit: String,
    /// Display unit for angle (deg, rad).
    pub angle_unit: String,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        DocumentSettings {
            length_unit: "mm".to_string(),
            angle_unit: "deg".to_string(),
        }
    }
}

/// Enumeration of supported commands (skeleton for M1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Emit a diagnostic for testing/debugging.
    #[serde(rename = "emit_diagnostic")]
    EmitDiagnostic {
        /// Machine-readable diagnostic code (e.g., "SKETCH.CONSTRAINT.REDUNDANT").
        code: String,
        /// Severity level: "info", "warning", or "error".
        severity: String,
    },

    /// No-op command for testing the bus.
    #[serde(rename = "noop")]
    NoOp,

    /// Set the document's display unit for lengths.
    #[serde(rename = "set_display_unit")]
    SetDisplayUnit {
        /// New display unit (e.g., "mm", "in").
        unit: String,
        /// Unit type: "length" or "angle".
        unit_type: String,
    },
}

impl Command {
    /// Returns a label suitable for undo history.
    pub fn label(&self) -> &'static str {
        match self {
            Command::EmitDiagnostic { .. } => "Emit Diagnostic",
            Command::NoOp => "No-op",
            Command::SetDisplayUnit { .. } => "Set Display Unit",
        }
    }
}

/// Outcome of a command execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandOutcome {
    /// Command succeeded with an optional message.
    #[serde(rename = "success")]
    Success {
        /// Descriptive message about the successful outcome.
        message: String,
    },

    /// Command was rejected with a diagnostic.
    #[serde(rename = "rejected")]
    Rejected {
        /// Error code identifying the rejection reason.
        code: String,
        /// Human-readable reason for rejection.
        reason: String,
    },
}

impl CommandOutcome {
    /// Returns true if the outcome is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, CommandOutcome::Success { .. })
    }

    /// Converts a Result to CommandOutcome.
    pub fn from_result<E: ToString>(r: Result<String, E>) -> Self {
        match r {
            Ok(msg) => CommandOutcome::Success { message: msg },
            Err(e) => CommandOutcome::Rejected { code: "ERROR".to_string(), reason: e.to_string() },
        }
    }
}

/// A single undo/redo entry.
#[derive(Clone, Debug)]
pub struct Transaction {
    /// Label shown in undo history (e.g., "Extrude1").
    pub label: String,
    /// The command that created this transaction.
    pub command: Command,
    /// Reversible state (stub for M1).
    pub reverse_data: Vec<u8>,
}

/// Undo/redo transaction stack (skeleton).
#[derive(Debug)]
pub struct TransactionStack {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    max_depth: usize,
}

impl TransactionStack {
    /// Creates a new transaction stack.
    pub fn new() -> Self {
        TransactionStack {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 100,
        }
    }

    /// Pushes a transaction onto the undo stack, clearing redo.
    pub fn push(&mut self, tx: Transaction) {
        self.undo_stack.push(tx);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    /// Undoes the last transaction (stub; returns label).
    pub fn undo(&mut self) -> Option<String> {
        self.undo_stack.pop().map(|tx| {
            self.redo_stack.push(tx.clone());
            tx.label
        })
    }

    /// Redoes the last undone transaction (stub; returns label).
    pub fn redo(&mut self) -> Option<String> {
        self.redo_stack.pop().map(|tx| {
            self.undo_stack.push(tx.clone());
            tx.label
        })
    }

    /// Returns the count of undo-able transactions.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the count of redo-able transactions.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for TransactionStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Command dispatch bus.
///
/// Executes commands, returns outcomes, and optionally records transactions for undo/redo.
pub struct CommandBus {
    /// Undo/redo transaction stack.
    tx_stack: TransactionStack,
    /// Whether to record transactions.
    record_transactions: bool,
    /// Document settings (display units, etc.).
    pub settings: DocumentSettings,
}

impl CommandBus {
    /// Creates a new command bus.
    pub fn new() -> Self {
        CommandBus {
            tx_stack: TransactionStack::new(),
            record_transactions: true,
            settings: DocumentSettings::default(),
        }
    }

    /// Dispatches a command and returns the outcome.
    ///
    /// For M1, this is a stub that handles test commands (EmitDiagnostic, NoOp, SetDisplayUnit).
    pub fn dispatch(&mut self, cmd: Command) -> CommandOutcome {
        let outcome = self.execute(&cmd);

        if self.record_transactions && outcome.is_success() {
            let tx = Transaction {
                label: cmd.label().to_string(),
                command: cmd,
                reverse_data: Vec::new(),
            };
            self.tx_stack.push(tx);
        }

        outcome
    }

    fn execute(&mut self, cmd: &Command) -> CommandOutcome {
        match cmd {
            Command::EmitDiagnostic { code, severity } => {
                CommandOutcome::Success {
                    message: format!("Emitted {} diagnostic: {}", severity, code),
                }
            }
            Command::NoOp => {
                CommandOutcome::Success { message: "No-op completed".to_string() }
            }
            Command::SetDisplayUnit { unit, unit_type } => {
                match unit_type.as_str() {
                    "length" => {
                        self.settings.length_unit = unit.clone();
                        CommandOutcome::Success {
                            message: format!("Set display unit to {}", unit),
                        }
                    }
                    "angle" => {
                        self.settings.angle_unit = unit.clone();
                        CommandOutcome::Success {
                            message: format!("Set angle unit to {}", unit),
                        }
                    }
                    _ => CommandOutcome::Rejected {
                        code: "INVALID_UNIT_TYPE".to_string(),
                        reason: format!("Unknown unit type: {}", unit_type),
                    },
                }
            }
        }
    }

    /// Undoes the last transaction.
    pub fn undo(&mut self) -> Option<String> {
        self.tx_stack.undo()
    }

    /// Redoes the last undone transaction.
    pub fn redo(&mut self) -> Option<String> {
        self.tx_stack.redo()
    }

    /// Returns the number of undo-able transactions.
    pub fn undo_count(&self) -> usize {
        self.tx_stack.undo_count()
    }

    /// Returns the number of redo-able transactions.
    pub fn redo_count(&self) -> usize {
        self.tx_stack.redo_count()
    }

    /// Enables or disables transaction recording.
    pub fn set_record_transactions(&mut self, record: bool) {
        self.record_transactions = record;
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_bus_dispatch() {
        let mut bus = CommandBus::new();
        let cmd = Command::EmitDiagnostic {
            code: "TEST.CODE".to_string(),
            severity: "warning".to_string(),
        };
        let outcome = bus.dispatch(cmd);
        assert!(outcome.is_success());
        assert_eq!(bus.undo_count(), 1);
    }

    #[test]
    fn transaction_stack_undo_redo() {
        let mut stack = TransactionStack::new();
        let tx = Transaction {
            label: "Test".to_string(),
            command: Command::NoOp,
            reverse_data: Vec::new(),
        };
        stack.push(tx);
        assert_eq!(stack.undo_count(), 1);

        let label = stack.undo();
        assert_eq!(label, Some("Test".to_string()));
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 1);

        let label = stack.redo();
        assert_eq!(label, Some("Test".to_string()));
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn command_outcome_success() {
        let outcome = CommandOutcome::Success { message: "OK".to_string() };
        assert!(outcome.is_success());
    }

    #[test]
    fn command_outcome_rejected() {
        let outcome = CommandOutcome::Rejected {
            code: "ERR".to_string(),
            reason: "Invalid input".to_string(),
        };
        assert!(!outcome.is_success());
    }

    #[test]
    fn command_outcome_from_result() {
        let ok: Result<String, String> = Ok("Success".to_string());
        let outcome = CommandOutcome::from_result(ok);
        assert!(outcome.is_success());

        let err: Result<String, String> = Err("Failed".to_string());
        let outcome = CommandOutcome::from_result(err);
        assert!(!outcome.is_success());
    }
}
