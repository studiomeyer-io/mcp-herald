//! FALSE-POSITIVE / TRUE-NEGATIVE defense — the critical suite.
//!
//! Every case here is a compliant or innocuous pattern that the corresponding rule must
//! NOT flag. A regression that re-introduces a false positive (e.g. flagging the very fix
//! the tool recommends) fails loudly here.

mod common;
use common::{count_rule, fired, scan, scan_ts};

// ===========================================================================
// spec.error_code — the `\b` word boundary must keep -32002 from matching inside
// a longer number, while still catching it with trailing punctuation.
// ===========================================================================

#[test]
fn error_code_does_not_match_larger_number() {
    // -320021 contains "-32002" as a prefix, but is followed by a digit -> no word
    // boundary -> MUST NOT fire.
    assert!(!fired(&scan_ts("const C = -320021;"), "spec.error_code"));
    assert!(!fired(&scan_ts("x = -3200299"), "spec.error_code"));
}

#[test]
fn error_code_does_not_match_the_compliant_replacement() {
    // -32602 (Invalid Params) is the migration target — never flag it.
    assert!(!fired(
        &scan_ts("return new McpError(-32602, 'invalid params');"),
        "spec.error_code"
    ));
}

#[test]
fn error_code_ignores_unrelated_numbers() {
    assert!(!fired(&scan_ts("listen(32002)"), "spec.error_code")); // no minus
    assert!(!fired(&scan_ts("const PORT = 320020;"), "spec.error_code"));
}

#[test]
fn error_code_matches_with_trailing_punctuation() {
    // Positive control living next to the negatives: comma / paren / semicolon all match.
    assert!(fired(&scan_ts("e(-32002,)"), "spec.error_code"));
    assert!(fired(&scan_ts("e(-32002)"), "spec.error_code"));
    assert!(fired(&scan_ts("e = -32002;"), "spec.error_code"));
}

// ===========================================================================
// transport.stateful_session — the compliant stateless idiom must stay silent.
// ===========================================================================

#[test]
fn transport_does_not_flag_session_id_generator_undefined() {
    // This is exactly the fix the tool recommends — flagging it would be self-defeating.
    assert!(!fired(
        &scan_ts(
            "const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });"
        ),
        "transport.stateful_session"
    ));
}

#[test]
fn transport_does_not_flag_nullish_undefined_expression() {
    // `(x ?? undefined)` is a non-empty paren group; the `\(\s*\)` branch must not match it.
    assert!(!fired(
        &scan_ts("sessionIdGenerator: (x ?? undefined),"),
        "transport.stateful_session"
    ));
    assert!(!fired(
        &scan_ts("sessionIdGenerator: (config.id ?? undefined)"),
        "transport.stateful_session"
    ));
}

#[test]
fn transport_does_not_flag_bare_word_session() {
    // No "mcp-" prefix, no generator -> innocuous.
    assert!(!fired(
        &scan_ts("const session = createSession();"),
        "transport.stateful_session"
    ));
    assert!(!fired(
        &scan_ts("// reset the session token"),
        "transport.stateful_session"
    ));
}

// ===========================================================================
// deprecated.logging — the `["']` quote prefix prevents REST routes and prose.
// ===========================================================================

#[test]
fn logging_does_not_flag_rest_route() {
    // app.post('/notifications/message', ...) — the slash after the quote breaks the match.
    assert!(!fired(
        &scan_ts("app.post('/notifications/message', handler);"),
        "deprecated.logging"
    ));
    assert!(!fired(
        &scan_ts(r#"router.get("/notifications/message", h)"#),
        "deprecated.logging"
    ));
}

#[test]
fn logging_does_not_flag_prose_comment() {
    // A comment mentioning the method (no quote immediately in front) must stay silent.
    assert!(!fired(
        &scan_ts("// send a notifications/message when the level changes"),
        "deprecated.logging"
    ));
    assert!(!fired(
        &scan_ts(" * emits notifications/message events"),
        "deprecated.logging"
    ));
}

#[test]
fn logging_quoted_literal_still_fires_next_to_negatives() {
    // Positive control: with a quote directly in front it IS the method literal.
    assert!(fired(
        &scan_ts(r#"method === "notifications/message""#),
        "deprecated.logging"
    ));
}

#[test]
fn logging_is_case_sensitive_for_types() {
    // No (?i) flag: a lowercased type name is not the SDK symbol and must not fire.
    assert!(!fired(
        &scan_ts("const setlevelrequest = 1;"),
        "deprecated.logging"
    ));
}

// ===========================================================================
// auth.protected_resource_metadata — only when WWW-Authenticate present AND no metadata.
// ===========================================================================

#[test]
fn auth_does_not_flag_when_well_known_endpoint_present() {
    // Compliant resource server: serves the discovery endpoint.
    assert!(!fired(
        &scan_ts(
            "res.setHeader('WWW-Authenticate', 'Bearer');\napp.get('/.well-known/oauth-protected-resource', handler);"
        ),
        "auth.protected_resource_metadata"
    ));
}

#[test]
fn auth_suppressed_by_any_absent_signal() {
    // Each of the "already-compliant" markers suppresses the finding.
    for marker in [
        "const resource_metadata = {};",
        "// resource indicator per resource_indicator",
        "// audience binding, see RFC 8707",
        "// metadata per RFC 9728",
        "// protected-resource-metadata served elsewhere",
    ] {
        let src = format!("res.setHeader('WWW-Authenticate', 'Bearer');\n{marker}");
        assert!(
            !fired(&scan_ts(&src), "auth.protected_resource_metadata"),
            "marker {marker:?} should suppress the finding"
        );
    }
}

#[test]
fn auth_does_not_flag_file_without_oauth() {
    // No WWW-Authenticate anywhere -> not in scope -> never fires.
    assert!(!fired(
        &scan_ts("export function add(a: number, b: number) { return a + b; }"),
        "auth.protected_resource_metadata"
    ));
}

#[test]
fn auth_fires_exactly_once_with_multiple_headers() {
    // File-level rule: even with several WWW-Authenticate lines, exactly one finding.
    let src = "WWW-Authenticate: Bearer\nWWW-Authenticate: Basic\nWWW-Authenticate: Negotiate\n";
    assert_eq!(
        count_rule(&scan_ts(src), "auth.protected_resource_metadata"),
        1
    );
}

// ===========================================================================
// protocol.old_version — the current date must not trip the legacy-date rule.
// ===========================================================================

#[test]
fn old_version_does_not_flag_current_date() {
    // 2025-11-25 is current; the 2026-07-28 final date is also not in the legacy set.
    assert!(!fired(
        &scan_ts("const PV = '2025-11-25';"),
        "protocol.old_version"
    ));
    assert!(!fired(
        &scan_ts("// targeting 2026-07-28"),
        "protocol.old_version"
    ));
}

#[test]
fn old_version_ignores_unrelated_dates() {
    assert!(!fired(
        &scan_ts("const released = '2023-01-01';"),
        "protocol.old_version"
    ));
}

// ===========================================================================
// Cross-cutting: a fully compliant modern server produces ZERO findings.
// ===========================================================================

#[test]
fn fully_compliant_server_is_clean() {
    let src = r#"
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";

// Stateless transport per SEP-2575.
const transport = new StreamableHTTPServerTransport({
  sessionIdGenerator: undefined,
  enableJsonResponse: true,
});

// Resource-not-found now uses the JSON-RPC standard code.
function notFound() {
  return { error: { code: -32602, message: "invalid params" } };
}

// Current protocol version.
const PROTOCOL_VERSION = "2025-11-25";

// OAuth resource server with discovery endpoint (RFC 9728).
app.get("/.well-known/oauth-protected-resource", (_req, res) => res.json(meta));
res.setHeader("WWW-Authenticate", 'Bearer resource_metadata="..."');
"#;
    let f = scan("modern-server.ts", src);
    assert!(f.is_empty(), "compliant server should be clean, got: {f:?}");
}

#[test]
fn non_source_like_text_without_signatures_is_clean() {
    let f = scan_ts("const greeting = 'hello world';\nexport default greeting;");
    assert!(f.is_empty(), "unexpected findings: {f:?}");
}
