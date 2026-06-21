//! Source scanner: walk a tree (respecting `.gitignore`), read text files, apply rules.

use std::path::Path;

use ignore::WalkBuilder;

use crate::finding::{Finding, Report};
use crate::rules::{CompiledKind, CompiledRule};

/// Skip files larger than this (generated bundles, lockfiles, vendored blobs).
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Cap a snippet so a minified line can't blow up the report.
const SNIPPET_MAX: usize = 200;

/// Source extensions worth scanning. Kept broad — MCP servers are written in many languages.
const SOURCE_EXTS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "java", "kt", "kts", "rb", "cs",
    "php", "scala", "swift", "c", "h", "cpp", "hpp", "cc",
];

fn is_source(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SOURCE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn snippet(line: &str) -> String {
    let t = line.trim();
    if t.chars().count() <= SNIPPET_MAX {
        t.to_string()
    } else {
        let mut s: String = t.chars().take(SNIPPET_MAX).collect();
        s.push('…');
        s
    }
}

/// Scan a path (file or directory). Honors `.gitignore` and skips hidden files, binaries
/// (non-UTF-8), and anything over [`MAX_FILE_BYTES`].
pub fn scan_path(root: &Path, rules: &[CompiledRule]) -> Report {
    let mut report = Report::default();

    for entry in WalkBuilder::new(root).build().flatten() {
        let path = entry.path();
        if !path.is_file() || !is_source(path) {
            continue;
        }
        if entry
            .metadata()
            .map(|m| m.len() > MAX_FILE_BYTES)
            .unwrap_or(false)
        {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue; // non-UTF-8 / unreadable -> skip
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        scan_content(&rel, &content, rules, &mut report.findings);
    }

    report.findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(b.level.cmp(&a.level))
    });
    report
}

/// Apply rules to one file's content. Pure — no filesystem — so it is directly testable.
pub fn scan_content(file: &str, content: &str, rules: &[CompiledRule], out: &mut Vec<Finding>) {
    // Per-line rules.
    for (idx, line) in content.lines().enumerate() {
        for cr in rules {
            if let CompiledKind::Line(re) = &cr.kind {
                if re.is_match(line) {
                    out.push(finding(cr, file, idx + 1, snippet(line)));
                }
            }
        }
    }
    // File-level present/absent rules.
    for cr in rules {
        if let CompiledKind::FilePresentAbsent { present, absent } = &cr.kind {
            if present.is_match(content) && !absent.is_match(content) {
                let line = content
                    .lines()
                    .position(|l| present.is_match(l))
                    .map(|i| i + 1)
                    .unwrap_or(1);
                out.push(finding(cr, file, line, String::new()));
            }
        }
    }
}

fn finding(cr: &CompiledRule, file: &str, line: usize, snippet: String) -> Finding {
    Finding {
        level: cr.spec.level,
        rule: cr.spec.id,
        sep: cr.spec.sep,
        file: file.to_string(),
        line,
        message: cr.spec.message,
        fix: cr.spec.fix,
        url: cr.spec.url,
        snippet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{compile, Level};

    fn scan(content: &str) -> Vec<Finding> {
        let rules = compile();
        let mut out = Vec::new();
        scan_content("server.ts", content, &rules, &mut out);
        out
    }

    #[test]
    fn flags_hardcoded_error_code() {
        let f = scan(r#"  throw new McpError(-32002, "not found");"#);
        assert!(f.iter().any(|f| f.rule == "spec.error_code"));
        assert_eq!(
            f.iter().find(|f| f.rule == "spec.error_code").unwrap().line,
            1
        );
        assert!(f.iter().any(|f| f.level == Level::Error));
    }

    #[test]
    fn flags_session_id_usage() {
        let f = scan("const sid = req.headers['Mcp-Session-Id'];");
        assert!(f.iter().any(|f| f.rule == "transport.stateful_session"));
    }

    #[test]
    fn flags_deprecated_sampling_and_roots() {
        let f =
            scan("await client.request('sampling/createMessage', p);\nlistRoots: 'roots/list',");
        assert!(f.iter().any(|f| f.rule == "deprecated.sampling"));
        assert!(f.iter().any(|f| f.rule == "deprecated.roots"));
    }

    #[test]
    fn oauth_without_resource_metadata_is_flagged_once() {
        let f = scan("res.setHeader('WWW-Authenticate', 'Bearer');\n// token_endpoint config");
        let hits: Vec<_> = f
            .iter()
            .filter(|f| f.rule == "auth.protected_resource_metadata")
            .collect();
        assert_eq!(hits.len(), 1, "file-level rule should fire once");
    }

    #[test]
    fn oauth_with_resource_metadata_is_not_flagged() {
        let f = scan(
            "res.setHeader('WWW-Authenticate', 'Bearer');\napp.get('/.well-known/oauth-protected-resource', h);",
        );
        assert!(!f
            .iter()
            .any(|f| f.rule == "auth.protected_resource_metadata"));
    }

    #[test]
    fn clean_modern_server_has_no_findings() {
        // The compliant stateless idiom (`sessionIdGenerator: undefined`, -32602) must not
        // trip any rule — otherwise the tool flags the very fix it recommends.
        let f = scan(
            "const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });\n// returns -32602 for unknown resources",
        );
        assert!(f.is_empty(), "unexpected findings: {f:?}");
    }

    #[test]
    fn stateful_generator_is_flagged() {
        let f =
            scan("new StreamableHTTPServerTransport({ sessionIdGenerator: () => randomUUID() });");
        assert!(f.iter().any(|f| f.rule == "transport.stateful_session"));
    }
}
