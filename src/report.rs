//! Human-readable terminal rendering.

use crate::finding::Report;
use crate::rules::{Level, RULES};

/// Render a scan report grouped by severity, most severe first, with fixes + sources.
pub fn render(report: &Report, root: &str) -> String {
    if report.findings.is_empty() {
        return format!("mcp-herald: no 2026-07-28 migration issues found in {root}. [OK]\n");
    }
    let mut out = String::new();
    out.push_str(&format!(
        "mcp-herald: {} finding(s) in {} — {} error, {} warning, {} info\n\n",
        report.findings.len(),
        root,
        report.count(Level::Error),
        report.count(Level::Warning),
        report.count(Level::Info),
    ));
    for level in [Level::Error, Level::Warning, Level::Info] {
        let group: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.level == level)
            .collect();
        if group.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "  {} ({})\n",
            level.label().to_uppercase(),
            group.len()
        ));
        for f in group {
            out.push_str(&format!("    {}:{} — {}\n", f.file, f.line, f.message));
            out.push_str(&format!("      fix: {}\n", f.fix));
            out.push_str(&format!("      {} · {}\n", f.sep, f.url));
            if !f.snippet.is_empty() {
                out.push_str(&format!("      | {}\n", f.snippet));
            }
        }
        out.push('\n');
    }
    out
}

/// Render the full ruleset (for `mcp-herald rules`).
pub fn render_rules() -> String {
    let mut out = String::from("mcp-herald rules:\n\n");
    for r in RULES {
        out.push_str(&format!(
            "  [{}] {} ({})\n      {}\n      {}\n\n",
            r.level.label(),
            r.id,
            r.sep,
            r.message,
            r.url
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Report;

    #[test]
    fn clean_report_message() {
        let out = render(&Report::default(), "./src");
        assert!(out.contains("no 2026-07-28 migration issues"));
    }

    #[test]
    fn rules_listing_lists_all_rules() {
        let out = render_rules();
        for r in RULES {
            assert!(out.contains(r.id), "missing {}", r.id);
        }
    }
}
