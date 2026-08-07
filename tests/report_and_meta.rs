//! Report model (`count` / `has_at_least` / empty) and ruleset meta-invariants
//! (every rule compiles, ids unique, levels as specced, url + fix present).

mod common;
use common::scan;

use mcp_herald::{compile, Level, Report, RULES};

// ---------------------------------------------------------------------------
// Report model
// ---------------------------------------------------------------------------

#[test]
fn empty_report_counts_zero_for_all_levels() {
    let r = Report::default();
    assert_eq!(r.count(Level::Error), 0);
    assert_eq!(r.count(Level::Warning), 0);
    assert_eq!(r.count(Level::Info), 0);
    assert!(!r.has_at_least(Level::Info));
    assert!(!r.has_at_least(Level::Warning));
    assert!(!r.has_at_least(Level::Error));
}

#[test]
fn count_is_per_exact_level() {
    // Build a report with one of each level.
    let findings = scan(
        "mix.ts",
        // error + warning + info, each on its own line
        "throw e(-32002);\nheaders['Mcp-Session-Id'];\nconst PV='2024-11-05';",
    );
    let r = Report { findings };
    assert_eq!(r.count(Level::Error), 1, "one -32002");
    assert_eq!(r.count(Level::Warning), 1, "one session header");
    assert_eq!(r.count(Level::Info), 1, "one old version");
}

#[test]
fn has_at_least_is_inclusive_upward() {
    // An Error finding satisfies has_at_least for Info, Warning, AND Error
    // (ordering Info < Warning < Error).
    let findings = scan("e.ts", "throw e(-32002);");
    let r = Report { findings };
    assert!(r.has_at_least(Level::Info));
    assert!(r.has_at_least(Level::Warning));
    assert!(r.has_at_least(Level::Error));
}

#[test]
fn has_at_least_info_only_does_not_satisfy_warning() {
    // A report whose highest severity is Info must NOT satisfy has_at_least(Warning).
    let findings = scan("i.ts", "const PV='2025-03-26';");
    let r = Report { findings };
    assert_eq!(r.count(Level::Info), 1);
    assert!(r.has_at_least(Level::Info));
    assert!(!r.has_at_least(Level::Warning));
    assert!(!r.has_at_least(Level::Error));
}

#[test]
fn has_at_least_warning_not_satisfied_only_by_warnings_for_error() {
    let findings = scan("w.ts", "headers['Mcp-Session-Id'];");
    let r = Report { findings };
    assert!(r.has_at_least(Level::Warning));
    assert!(!r.has_at_least(Level::Error));
}

// ---------------------------------------------------------------------------
// Level ordering (drives --fail-on thresholds)
// ---------------------------------------------------------------------------

#[test]
fn level_ordering_is_info_lt_warning_lt_error() {
    assert!(Level::Info < Level::Warning);
    assert!(Level::Warning < Level::Error);
    assert!(Level::Info < Level::Error);
}

#[test]
fn level_labels_are_stable() {
    assert_eq!(Level::Info.label(), "info");
    assert_eq!(Level::Warning.label(), "warning");
    assert_eq!(Level::Error.label(), "error");
}

// ---------------------------------------------------------------------------
// Ruleset meta-invariants
// ---------------------------------------------------------------------------

#[test]
fn all_rules_compile_and_count_matches_specs() {
    let compiled = compile();
    assert_eq!(compiled.len(), RULES.len());
    // There are exactly eight curated rules (seven at 0.1.0 plus the registration axis).
    assert_eq!(RULES.len(), 8, "expected 8 rules");
}

#[test]
fn rule_ids_are_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for r in RULES {
        assert!(seen.insert(r.id), "duplicate rule id {}", r.id);
    }
    assert_eq!(seen.len(), RULES.len());
}

#[test]
fn the_eight_expected_ids_exist() {
    let ids: std::collections::BTreeSet<&str> = RULES.iter().map(|r| r.id).collect();
    for expected in [
        "spec.error_code",
        "transport.stateful_session",
        "deprecated.sampling",
        "deprecated.roots",
        "deprecated.logging",
        "protocol.old_version",
        "auth.protected_resource_metadata",
        "auth.dcr_without_cimd",
    ] {
        assert!(ids.contains(expected), "missing rule id {expected}");
    }
}

#[test]
fn every_rule_has_https_url_and_non_empty_fix_and_message() {
    for r in RULES {
        assert!(
            r.url.starts_with("https://"),
            "rule {} lacks https url",
            r.id
        );
        assert!(!r.fix.is_empty(), "rule {} lacks a fix", r.id);
        assert!(!r.message.is_empty(), "rule {} lacks a message", r.id);
        assert!(!r.sep.is_empty(), "rule {} lacks a sep label", r.id);
    }
}

#[test]
fn rule_levels_match_specification() {
    let level_of = |id: &str| RULES.iter().find(|r| r.id == id).unwrap().level;
    assert_eq!(level_of("spec.error_code"), Level::Error);
    assert_eq!(level_of("transport.stateful_session"), Level::Warning);
    assert_eq!(level_of("deprecated.sampling"), Level::Warning);
    assert_eq!(level_of("deprecated.roots"), Level::Warning);
    assert_eq!(level_of("deprecated.logging"), Level::Warning);
    assert_eq!(level_of("protocol.old_version"), Level::Info);
    assert_eq!(level_of("auth.protected_resource_metadata"), Level::Info);
    // DCR stays at MAY in the 2026-07-28 spec — a warning, never an error.
    assert_eq!(level_of("auth.dcr_without_cimd"), Level::Warning);
}

#[test]
fn exactly_one_error_rule_and_two_file_level_rules() {
    let errors = RULES.iter().filter(|r| r.level == Level::Error).count();
    assert_eq!(errors, 1, "only spec.error_code is Error-level");

    use mcp_herald::rules::Kind;
    let file_level = RULES
        .iter()
        .filter(|r| matches!(r.kind, Kind::FilePresentAbsent { .. }))
        .count();
    assert_eq!(
        file_level, 2,
        "the two auth rules are file-level present/absent"
    );
}
