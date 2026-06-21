//! SARIF 2.1.0 output coverage: schema version, level mapping (error/warning/note),
//! startLine >= 1, correct ruleId, only-fired-rules as descriptors, helpUri set.

mod common;
use common::scan;

use mcp_herald::sarif::to_sarif;
use mcp_herald::{compile, scan_content, Finding, Report};

fn sarif_for(file: &str, content: &str) -> serde_json::Value {
    let findings = scan(file, content);
    to_sarif(&Report { findings })
}

#[test]
fn version_and_schema_are_set() {
    let s = sarif_for("s.ts", "throw e(-32002);");
    assert_eq!(s["version"], "2.1.0");
    assert!(s["$schema"].as_str().unwrap().contains("sarif-2.1.0"));
}

#[test]
fn driver_metadata_present() {
    let s = sarif_for("s.ts", "throw e(-32002);");
    let driver = &s["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "mcp-herald");
    assert!(driver["informationUri"]
        .as_str()
        .unwrap()
        .starts_with("https://"));
    // Version comes from CARGO_PKG_VERSION and must be a non-empty string.
    assert!(!driver["version"].as_str().unwrap().is_empty());
}

#[test]
fn error_level_maps_to_error() {
    let s = sarif_for("src/server.ts", "throw new McpError(-32002);");
    let res = &s["runs"][0]["results"][0];
    assert_eq!(res["ruleId"], "spec.error_code");
    assert_eq!(res["level"], "error");
}

#[test]
fn warning_level_maps_to_warning() {
    let s = sarif_for("s.ts", "headers['Mcp-Session-Id'];");
    let result = s["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ruleId"] == "transport.stateful_session")
        .expect("a transport result");
    assert_eq!(result["level"], "warning");
}

#[test]
fn info_level_maps_to_note() {
    // protocol.old_version is Info -> SARIF "note".
    let s = sarif_for("s.ts", "const PV='2024-11-05';");
    let result = s["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ruleId"] == "protocol.old_version")
        .expect("an old-version result");
    assert_eq!(result["level"], "note");
}

#[test]
fn auth_info_rule_maps_to_note() {
    let s = sarif_for("s.ts", "res.setHeader('WWW-Authenticate', 'Bearer');");
    let result = s["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ruleId"] == "auth.protected_resource_metadata")
        .expect("an auth result");
    assert_eq!(result["level"], "note");
}

#[test]
fn start_line_is_correct_and_at_least_one() {
    let rules = compile();
    let mut findings = Vec::new();
    scan_content(
        "src/server.ts",
        "pad\npad\nthrow e(-32002);",
        &rules,
        &mut findings,
    );
    let s = to_sarif(&Report { findings });
    let region = &s["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
    assert_eq!(region["startLine"], 3);
}

#[test]
fn start_line_clamped_to_one_for_zero() {
    // Defensive: even a (hypothetical) line 0 finding renders startLine >= 1.
    let f = Finding {
        level: mcp_herald::Level::Error,
        rule: "spec.error_code",
        sep: "SEP-2164",
        file: "x.ts".into(),
        line: 0,
        message: "m",
        fix: "f",
        url: "https://example.test",
        snippet: String::new(),
    };
    let s = to_sarif(&Report { findings: vec![f] });
    let start =
        &s["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"];
    assert_eq!(*start, serde_json::json!(1));
}

#[test]
fn artifact_uri_carries_file_path() {
    let s = sarif_for("packages/server/index.ts", "throw e(-32002);");
    let uri =
        &s["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"];
    assert_eq!(uri, "packages/server/index.ts");
}

#[test]
fn only_fired_rules_become_descriptors() {
    // A file that trips exactly two rules -> exactly two rule descriptors, both correct ids.
    let s = sarif_for("s.ts", "throw e(-32002);\n'sampling/createMessage';");
    let descriptors = s["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
    let ids: Vec<&str> = descriptors
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "only fired rules, got {ids:?}");
    assert!(ids.contains(&"spec.error_code"));
    assert!(ids.contains(&"deprecated.sampling"));
    // Rules that did NOT fire are absent.
    assert!(!ids.contains(&"protocol.old_version"));
    assert!(!ids.contains(&"deprecated.roots"));
}

#[test]
fn descriptors_are_deduplicated() {
    // Same rule firing twice -> one descriptor, two results.
    let s = sarif_for("s.ts", "a(-32002);\nb(-32002);");
    let descriptors = s["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(s["runs"][0]["results"].as_array().unwrap().len(), 2);
}

#[test]
fn each_descriptor_has_help_uri_and_default_level() {
    let s = sarif_for("s.ts", "throw e(-32002);");
    let d = &s["runs"][0]["tool"]["driver"]["rules"][0];
    assert!(d["helpUri"].as_str().unwrap().starts_with("https://"));
    assert_eq!(d["defaultConfiguration"]["level"], "error");
    assert!(!d["shortDescription"]["text"].as_str().unwrap().is_empty());
}

#[test]
fn result_message_includes_sep_and_fix() {
    let s = sarif_for("s.ts", "throw e(-32002);");
    let msg = s["runs"][0]["results"][0]["message"]["text"]
        .as_str()
        .unwrap();
    assert!(
        msg.contains("SEP-2164"),
        "message should cite the SEP: {msg}"
    );
    assert!(msg.contains("Fix:"), "message should include a fix: {msg}");
}

#[test]
fn empty_report_renders_valid_empty_sarif() {
    let s = to_sarif(&Report::default());
    assert_eq!(s["version"], "2.1.0");
    assert_eq!(s["runs"][0]["results"].as_array().unwrap().len(), 0);
    assert_eq!(
        s["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
