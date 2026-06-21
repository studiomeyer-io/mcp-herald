//! Shared helpers for the integration tests. Compiles the ruleset once per call and
//! exposes small predicates so individual tests stay focused on the assertion.
//!
//! `#![allow(dead_code)]`: each test binary `mod common;`-includes this whole module, so a
//! helper that one file doesn't use would otherwise warn (and fail `clippy -D warnings`).
#![allow(dead_code)]

use mcp_herald::{compile, scan_content, Finding};

/// Scan a single in-memory file with the full ruleset and return every finding.
/// Pure path — no filesystem touched.
pub fn scan(file: &str, content: &str) -> Vec<Finding> {
    let rules = compile();
    let mut out = Vec::new();
    scan_content(file, content, &rules, &mut out);
    out
}

/// Convenience: scan with a default `.ts` file name.
pub fn scan_ts(content: &str) -> Vec<Finding> {
    scan("server.ts", content)
}

/// True if any finding has the given rule id.
pub fn fired(findings: &[Finding], rule: &str) -> bool {
    findings.iter().any(|f| f.rule == rule)
}

/// How many findings carry the given rule id.
pub fn count_rule(findings: &[Finding], rule: &str) -> usize {
    findings.iter().filter(|f| f.rule == rule).count()
}
