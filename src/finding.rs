//! A single migration finding.

use serde::Serialize;

use crate::rules::Level;

/// One match of a rule against a source file.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Severity.
    pub level: Level,
    /// Rule id that fired, e.g. `spec.error_code`.
    pub rule: &'static str,
    /// The SEP/RFC behind the rule.
    pub sep: &'static str,
    /// Path to the file (relative to the scan root when possible).
    pub file: String,
    /// 1-based line number. File-level rules report their first matching line.
    pub line: usize,
    /// What changed in the spec.
    pub message: &'static str,
    /// What to do about it.
    pub fix: &'static str,
    /// Source to verify against.
    pub url: &'static str,
    /// The matched source line, trimmed and length-capped.
    pub snippet: String,
}

/// Result of a scan.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    /// All findings, sorted by file then line.
    pub findings: Vec<Finding>,
}

impl Report {
    /// Number of findings at exactly the given level.
    pub fn count(&self, level: Level) -> usize {
        self.findings.iter().filter(|f| f.level == level).count()
    }

    /// Whether any finding is at or above the given level.
    pub fn has_at_least(&self, level: Level) -> bool {
        self.findings.iter().any(|f| f.level >= level)
    }
}
