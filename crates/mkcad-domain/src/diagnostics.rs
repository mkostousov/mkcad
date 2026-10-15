//! Diagnostics collection sink for the application.
//!
//! Collects diagnostics from all sources (solver, kernel, features) with severity,
//! code, and optional parameters. Timestamp-free for determinism (H.5, H-data-contracts).

use serde::{Deserialize, Serialize};

pub use mkcad_solver::Severity;

/// A collected diagnostic entry with machine code, severity, and display info.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    /// Machine-readable code (e.g., "SKETCH.CONSTRAINT.REDUNDANT").
    pub code: String,
    /// Severity level: info, warning, error.
    pub severity: String,
    /// Optional subject IDs (sketch, feature, entity references).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Optional parameters for display (e.g., suggestion: "make_driven").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A sink for collecting diagnostics throughout the application lifetime.
///
/// Diagnostics are appended as they occur; the sink never forgets them (session history).
/// For determinism, no timestamps are recorded here (per H.5, H-data-contracts).
#[derive(Debug)]
pub struct DiagnosticsSink {
    entries: Vec<DiagnosticEntry>,
    max_entries: usize,
}

impl DiagnosticsSink {
    /// Creates a new empty diagnostics sink.
    pub fn new() -> Self {
        DiagnosticsSink { entries: Vec::new(), max_entries: 1000 }
    }

    /// Emits a diagnostic (code, severity, optional subject and params).
    pub fn emit(&mut self, code: &str, severity: &str) {
        let entry = DiagnosticEntry {
            code: code.to_string(),
            severity: severity.to_string(),
            subject: None,
            params: None,
        };
        self.push_entry(entry);
    }

    /// Emits a diagnostic with a subject.
    pub fn emit_with_subject(&mut self, code: &str, severity: &str, subject: &str) {
        let entry = DiagnosticEntry {
            code: code.to_string(),
            severity: severity.to_string(),
            subject: Some(subject.to_string()),
            params: None,
        };
        self.push_entry(entry);
    }

    /// Emits a diagnostic with code, severity, subject, and params.
    pub fn emit_full(
        &mut self,
        code: &str,
        severity: &str,
        subject: Option<&str>,
        params: Option<serde_json::Value>,
    ) {
        let entry = DiagnosticEntry {
            code: code.to_string(),
            severity: severity.to_string(),
            subject: subject.map(|s| s.to_string()),
            params,
        };
        self.push_entry(entry);
    }

    /// Internal helper to push an entry, respecting max_entries.
    fn push_entry(&mut self, entry: DiagnosticEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            // Drop oldest entry when at capacity
            self.entries.remove(0);
        }
    }

    /// Returns all collected diagnostic entries.
    pub fn all(&self) -> &[DiagnosticEntry] {
        &self.entries
    }

    /// Returns errors only.
    pub fn errors(&self) -> Vec<&DiagnosticEntry> {
        self.entries.iter().filter(|e| e.severity == "error").collect()
    }

    /// Returns warnings only.
    pub fn warnings(&self) -> Vec<&DiagnosticEntry> {
        self.entries.iter().filter(|e| e.severity == "warning").collect()
    }

    /// Returns info diagnostics only.
    pub fn infos(&self) -> Vec<&DiagnosticEntry> {
        self.entries.iter().filter(|e| e.severity == "info").collect()
    }

    /// Clears all diagnostics.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the count of all diagnostics.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the count of error diagnostics.
    pub fn error_count(&self) -> usize {
        self.errors().len()
    }

    /// Returns the count of warning diagnostics.
    pub fn warning_count(&self) -> usize {
        self.warnings().len()
    }

    /// Returns the count of info diagnostics.
    pub fn info_count(&self) -> usize {
        self.infos().len()
    }
}

impl Default for DiagnosticsSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_sink_emit() {
        let mut sink = DiagnosticsSink::new();
        sink.emit("TEST.CODE", "error");
        sink.emit("TEST.WARNING", "warning");
        sink.emit("TEST.INFO", "info");

        assert_eq!(sink.count(), 3);
        assert_eq!(sink.error_count(), 1);
        assert_eq!(sink.warning_count(), 1);
        assert_eq!(sink.info_count(), 1);
    }

    #[test]
    fn diagnostics_sink_filters() {
        let mut sink = DiagnosticsSink::new();
        sink.emit("E1", "error");
        sink.emit("W1", "warning");
        sink.emit("E2", "error");

        assert_eq!(sink.errors().len(), 2);
        assert_eq!(sink.warnings().len(), 1);
        assert_eq!(sink.infos().len(), 0);
    }

    #[test]
    fn diagnostics_sink_with_subject() {
        let mut sink = DiagnosticsSink::new();
        sink.emit_with_subject("REF.LOST", "error", "f-1");

        let entry = &sink.all()[0];
        assert_eq!(entry.code, "REF.LOST");
        assert_eq!(entry.subject, Some("f-1".to_string()));
    }

    #[test]
    fn diagnostics_sink_max_entries() {
        let mut sink = DiagnosticsSink::new();
        sink.max_entries = 5;

        for i in 0..10 {
            sink.emit(&format!("CODE.{}", i), "info");
        }

        assert_eq!(sink.count(), 5);
        // First 5 entries (0-4) should be dropped, last 5 (5-9) remain
        assert_eq!(sink.all()[0].code, "CODE.5");
        assert_eq!(sink.all()[4].code, "CODE.9");
    }

    #[test]
    fn diagnostics_sink_clear() {
        let mut sink = DiagnosticsSink::new();
        sink.emit("CODE1", "error");
        sink.emit("CODE2", "warning");
        assert_eq!(sink.count(), 2);

        sink.clear();
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn diagnostics_sink_emit_full() {
        let mut sink = DiagnosticsSink::new();
        let params = Some(serde_json::json!({"suggestion": "make_driven"}));
        sink.emit_full("SKETCH.CONSTRAINT.REDUNDANT", "error", Some("s-1"), params);

        let entry = &sink.all()[0];
        assert_eq!(entry.code, "SKETCH.CONSTRAINT.REDUNDANT");
        assert_eq!(entry.subject, Some("s-1".to_string()));
        assert!(entry.params.is_some());
    }
}
