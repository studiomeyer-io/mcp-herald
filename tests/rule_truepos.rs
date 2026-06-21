//! TRUE-POSITIVE coverage: each of the 7 rules must fire on a representative
//! breaking-change signature. Paired with `rule_trueneg.rs` (compliant code must NOT fire).

mod common;
use common::{fired, scan_ts};
use mcp_herald::Level;

// ---------------------------------------------------------------------------
// spec.error_code (Error) — retired -32002 resource-not-found code (SEP-2164)
// ---------------------------------------------------------------------------

#[test]
fn error_code_fires_on_mcperror_literal() {
    let f = scan_ts(r#"throw new McpError(-32002, "resource not found");"#);
    assert!(fired(&f, "spec.error_code"));
    // It is the flagship Error-level rule.
    let hit = f.iter().find(|f| f.rule == "spec.error_code").unwrap();
    assert_eq!(hit.level, Level::Error);
    assert_eq!(hit.sep, "SEP-2164");
}

#[test]
fn error_code_fires_when_followed_by_comma() {
    // Task contract: -32002 with a comma after MUST match.
    assert!(fired(
        &scan_ts("code: -32002, // legacy"),
        "spec.error_code"
    ));
}

#[test]
fn error_code_fires_when_followed_by_paren_or_semicolon() {
    assert!(fired(&scan_ts("return err(-32002);"), "spec.error_code"));
    assert!(fired(&scan_ts("const C = -32002"), "spec.error_code"));
}

// ---------------------------------------------------------------------------
// transport.stateful_session (Warning) — sticky sessions (SEP-2575)
// ---------------------------------------------------------------------------

#[test]
fn transport_fires_on_session_id_header_mixed_case() {
    let f = scan_ts("const sid = req.headers['Mcp-Session-Id'];");
    assert!(fired(&f, "transport.stateful_session"));
    assert_eq!(
        f.iter()
            .find(|f| f.rule == "transport.stateful_session")
            .unwrap()
            .level,
        Level::Warning
    );
}

#[test]
fn transport_fires_on_session_id_header_lowercase() {
    // The header match is case-insensitive.
    assert!(fired(
        &scan_ts("req.headers['mcp-session-id']"),
        "transport.stateful_session"
    ));
}

#[test]
fn transport_fires_on_real_generator() {
    // A wired-up generator (empty-arg arrow) is the breaking pattern.
    assert!(fired(
        &scan_ts("new StreamableHTTPServerTransport({ sessionIdGenerator: () => randomUUID() });"),
        "transport.stateful_session"
    ));
}

#[test]
fn transport_fires_on_named_generators() {
    for src in [
        "sessionIdGenerator: randomUUID",
        "sessionIdGenerator: uuid()",
        "sessionIdGenerator: nanoid",
        "sessionIdGenerator: crypto.randomUUID",
        "sessionIdGenerator: generateId",
        "sessionIdGenerator: async () => makeId()",
    ] {
        assert!(
            fired(&scan_ts(src), "transport.stateful_session"),
            "should fire on {src:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// deprecated.sampling (Warning) — SEP-2577
// ---------------------------------------------------------------------------

#[test]
fn sampling_fires_on_method_and_type() {
    assert!(fired(
        &scan_ts("await client.request('sampling/createMessage', p);"),
        "deprecated.sampling"
    ));
    assert!(fired(
        &scan_ts("function h(req: CreateMessageRequest) {}"),
        "deprecated.sampling"
    ));
}

// ---------------------------------------------------------------------------
// deprecated.roots (Warning) — SEP-2577
// ---------------------------------------------------------------------------

#[test]
fn roots_fires_on_method_and_type() {
    assert!(fired(&scan_ts("case 'roots/list':"), "deprecated.roots"));
    assert!(fired(
        &scan_ts("handler: (r: ListRootsRequest) => r"),
        "deprecated.roots"
    ));
}

// ---------------------------------------------------------------------------
// deprecated.logging (Warning) — SEP-2577
// ---------------------------------------------------------------------------

#[test]
fn logging_fires_on_set_level() {
    assert!(fired(
        &scan_ts("case 'logging/setLevel':"),
        "deprecated.logging"
    ));
    assert!(fired(
        &scan_ts("import { SetLevelRequest } from '@modelcontextprotocol/sdk';"),
        "deprecated.logging"
    ));
}

#[test]
fn logging_fires_on_quoted_method_literal() {
    // The method literal with a quote in front MUST fire (double and single quote).
    assert!(fired(
        &scan_ts(r#"server.notification("notifications/message", payload);"#),
        "deprecated.logging"
    ));
    assert!(fired(
        &scan_ts("server.notification('notifications/message', payload);"),
        "deprecated.logging"
    ));
}

// ---------------------------------------------------------------------------
// protocol.old_version (Info) — older protocol-version dates
// ---------------------------------------------------------------------------

#[test]
fn old_version_fires_on_each_legacy_date() {
    for date in ["2024-11-05", "2025-03-26", "2025-06-18"] {
        let f = scan_ts(&format!("const PROTOCOL_VERSION = '{date}';"));
        assert!(fired(&f, "protocol.old_version"), "should fire on {date}");
        assert_eq!(
            f.iter()
                .find(|f| f.rule == "protocol.old_version")
                .unwrap()
                .level,
            Level::Info
        );
    }
}

// ---------------------------------------------------------------------------
// auth.protected_resource_metadata (Info, file-level present/absent)
// ---------------------------------------------------------------------------

#[test]
fn auth_fires_when_www_authenticate_without_metadata() {
    let f = scan_ts(
        "res.status(401).setHeader('WWW-Authenticate', 'Bearer');\n// no resource metadata here",
    );
    assert!(fired(&f, "auth.protected_resource_metadata"));
    assert_eq!(
        f.iter()
            .find(|f| f.rule == "auth.protected_resource_metadata")
            .unwrap()
            .level,
        Level::Info
    );
}

#[test]
fn auth_present_is_case_insensitive() {
    // Lowercased and uppercased header spelling both count as "in scope".
    assert!(fired(
        &scan_ts("res.setHeader('www-authenticate', 'Bearer realm=x')"),
        "auth.protected_resource_metadata"
    ));
    assert!(fired(
        &scan_ts("WWW-AUTHENTICATE: Bearer"),
        "auth.protected_resource_metadata"
    ));
}

// ---------------------------------------------------------------------------
// All seven rules can fire at least once (smoke roll-up).
// ---------------------------------------------------------------------------

#[test]
fn all_seven_rules_are_reachable() {
    let combined = concat!(
        "throw new McpError(-32002);\n",             // spec.error_code
        "headers['Mcp-Session-Id'];\n",              // transport.stateful_session
        "'sampling/createMessage';\n",               // deprecated.sampling
        "'roots/list';\n",                           // deprecated.roots
        "'logging/setLevel';\n",                     // deprecated.logging
        "version='2024-11-05';\n",                   // protocol.old_version
        "setHeader('WWW-Authenticate','Bearer');\n"  // auth (file-level, no metadata)
    );
    let f = scan_ts(combined);
    for rule in [
        "spec.error_code",
        "transport.stateful_session",
        "deprecated.sampling",
        "deprecated.roots",
        "deprecated.logging",
        "protocol.old_version",
        "auth.protected_resource_metadata",
    ] {
        assert!(
            fired(&f, rule),
            "rule {rule} never fired in combined fixture"
        );
    }
}
