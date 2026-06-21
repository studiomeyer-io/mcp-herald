//! `scan_content` behavior: line numbering, multi-finding files, snippet shape,
//! case-insensitivity where it applies, and rule independence within a line.

mod common;
use common::{count_rule, fired, scan, scan_ts};
use mcp_herald::{compile, scan_content, Finding};

#[test]
fn line_numbers_are_one_based() {
    // Violation on the 3rd physical line -> finding.line == 3.
    let src = "// line 1\n// line 2\nthrow new McpError(-32002);\n// line 4";
    let f = scan_ts(src);
    let hit = f.iter().find(|f| f.rule == "spec.error_code").unwrap();
    assert_eq!(hit.line, 3);
}

#[test]
fn first_line_violation_reports_line_one() {
    let f = scan_ts("throw new McpError(-32002);");
    assert_eq!(
        f.iter().find(|f| f.rule == "spec.error_code").unwrap().line,
        1
    );
}

#[test]
fn multiple_findings_across_lines_keep_distinct_line_numbers() {
    let src = "headers['Mcp-Session-Id'];\n'sampling/createMessage';\n'roots/list';";
    let f = scan_ts(src);
    let line_of = |rule: &str| f.iter().find(|x| x.rule == rule).unwrap().line;
    assert_eq!(line_of("transport.stateful_session"), 1);
    assert_eq!(line_of("deprecated.sampling"), 2);
    assert_eq!(line_of("deprecated.roots"), 3);
}

#[test]
fn same_rule_fires_once_per_matching_line() {
    // Two separate lines each tripping spec.error_code -> two findings.
    let src = "a(-32002);\nb(-32002);";
    assert_eq!(count_rule(&scan_ts(src), "spec.error_code"), 2);
}

#[test]
fn multiple_rules_can_fire_on_a_single_line() {
    // One line carrying both a session header and a deprecated sampling literal.
    let line = "if (headers['Mcp-Session-Id']) handle('sampling/createMessage');";
    let f = scan_ts(line);
    assert!(fired(&f, "transport.stateful_session"));
    assert!(fired(&f, "deprecated.sampling"));
    // Both reported on line 1.
    assert!(f.iter().all(|x| x.line == 1));
}

#[test]
fn snippet_is_the_trimmed_line() {
    // Leading/trailing whitespace stripped; interior preserved.
    let f = scan_ts("        throw new McpError(-32002);    ");
    let hit = f.iter().find(|f| f.rule == "spec.error_code").unwrap();
    assert_eq!(hit.snippet, "throw new McpError(-32002);");
}

#[test]
fn file_level_finding_has_empty_snippet() {
    // The auth (present/absent) finding carries no source snippet.
    let f = scan_ts("res.setHeader('WWW-Authenticate', 'Bearer');");
    let hit = f
        .iter()
        .find(|f| f.rule == "auth.protected_resource_metadata")
        .unwrap();
    assert!(hit.snippet.is_empty());
}

#[test]
fn long_line_snippet_is_capped_with_ellipsis() {
    // SNIPPET_MAX is 200 chars; a longer matched line gets truncated + '…'.
    let pad = "x".repeat(400);
    let src = format!("const s = '{pad}'; // headers['Mcp-Session-Id']");
    let f = scan("big.ts", &src);
    let hit = f
        .iter()
        .find(|f| f.rule == "transport.stateful_session")
        .unwrap();
    assert!(
        hit.snippet.ends_with('…'),
        "snippet should end with ellipsis"
    );
    // 200 content chars + the ellipsis char.
    assert_eq!(hit.snippet.chars().count(), 201);
}

#[test]
fn case_insensitive_session_header_both_spellings() {
    assert!(fired(
        &scan_ts("MCP-SESSION-ID: abc"),
        "transport.stateful_session"
    ));
    assert!(fired(
        &scan_ts("mcp-session-id: abc"),
        "transport.stateful_session"
    ));
}

#[test]
fn deprecated_rules_are_case_sensitive() {
    // Sanity: the SEP-2577 type names have no (?i); a lowercased variant must not fire.
    assert!(!fired(
        &scan_ts("createmessagerequest"),
        "deprecated.sampling"
    ));
    assert!(!fired(&scan_ts("listrootsrequest"), "deprecated.roots"));
}

#[test]
fn scan_content_appends_and_does_not_clear_existing() {
    // The out-vec is appended to (the public signature takes &mut Vec); pre-existing
    // entries are preserved.
    let rules = compile();
    let mut out: Vec<Finding> = Vec::new();
    scan_content("a.ts", "throw e(-32002);", &rules, &mut out);
    let after_first = out.len();
    assert!(after_first >= 1);
    scan_content("b.ts", "headers['Mcp-Session-Id'];", &rules, &mut out);
    assert!(
        out.len() > after_first,
        "second scan must append, not replace"
    );
    // File names are carried through per finding.
    assert!(out.iter().any(|f| f.file == "a.ts"));
    assert!(out.iter().any(|f| f.file == "b.ts"));
}

#[test]
fn empty_content_yields_no_findings() {
    assert!(scan_ts("").is_empty());
    assert!(scan_ts("\n\n\n").is_empty());
}

#[test]
fn finding_carries_rule_metadata() {
    // Sanity that the finding mirrors the rule spec (sep/url/fix/message non-empty).
    let f = scan_ts("throw e(-32002);");
    let hit = f.iter().find(|f| f.rule == "spec.error_code").unwrap();
    assert_eq!(hit.sep, "SEP-2164");
    assert!(hit.url.starts_with("https://"));
    assert!(!hit.fix.is_empty());
    assert!(!hit.message.is_empty());
}
