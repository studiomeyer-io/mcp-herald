//! `auth.dcr_without_cimd` — the registration axis of the 2026-07-28 spec.
//!
//! The spec deprecates Dynamic Client Registration in favour of Client ID Metadata Documents
//! (SEP-991). This suite pins the three things that make the rule trustworthy:
//!
//! 1. it fires on a real authorization-server metadata block (true positive),
//! 2. it goes silent as soon as CIMD is advertised — the migration target must never be
//!    flagged, or the tool warns about the very fix it recommends,
//! 3. it stays out of prose and out of the OAuth-hardening signals owned by
//!    `mcp-stateless-migrator` (`r07-oauth-hardening`), so nobody gets double-reported.

mod common;
use common::{count_rule, fired, scan, scan_ts};
use mcp_herald::Level;

const RULE: &str = "auth.dcr_without_cimd";

// ---------------------------------------------------------------------------
// True positive
// ---------------------------------------------------------------------------

#[test]
fn fires_on_authorization_server_metadata() {
    let f = scan_ts("registration_endpoint: `${BASE_URL}/register`,");
    assert!(fired(&f, RULE));
}

#[test]
fn fires_on_the_real_world_metadata_block() {
    // Shape taken from a production MCP server's /.well-known/oauth-authorization-server
    // handler: DCR advertised, no CIMD anywhere in the file.
    let src = r#"
res.json({
  issuer: BASE_URL,
  authorization_endpoint: `${BASE_URL}/authorize`,
  token_endpoint: `${BASE_URL}/token`,
  registration_endpoint: `${BASE_URL}/register`,
  code_challenge_methods_supported: ["S256"],
});
"#;
    let f = scan("auth/oauth.ts", src);
    assert_eq!(count_rule(&f, RULE), 1);
}

#[test]
fn carries_the_expected_metadata() {
    let f = scan_ts("registration_endpoint: '/register',");
    let hit = f.iter().find(|f| f.rule == RULE).expect("a dcr finding");
    // Warning, not Error: DCR stays at MAY in 2026-07-28, so nothing breaks on the date.
    assert_eq!(hit.level, Level::Warning);
    // A2: the SEP label must not invent a number for the deprecation itself.
    assert_eq!(hit.sep, "SEP-991 / spec 2026-07-28");
    assert!(hit.url.starts_with("https://modelcontextprotocol.io/"));
}

#[test]
fn is_case_insensitive_on_the_field_name() {
    assert!(fired(&scan_ts("REGISTRATION_ENDPOINT: url"), RULE));
    assert!(fired(&scan_ts("\"Registration_Endpoint\": url"), RULE));
}

// ---------------------------------------------------------------------------
// The absent side — the migration target must be silent
// ---------------------------------------------------------------------------

#[test]
fn cimd_advertised_suppresses_the_finding() {
    // The endorsed end state: CIMD primary, DCR kept as a documented fallback.
    let src = "registration_endpoint: `${BASE_URL}/register`,\n\
               client_id_metadata_document_supported: true,";
    assert!(
        !fired(&scan_ts(src), RULE),
        "advertising CIMD must clear the finding"
    );
}

#[test]
fn any_cimd_spelling_suppresses_the_finding() {
    for marker in [
        "client_id_metadata_document_supported: true,",
        "// resolve clientIdMetadata documents by URL",
        "// CIMD support lives in ./cimd.ts",
        "const clientIdMetadataUrl = new URL(clientId);",
    ] {
        let src = format!("registration_endpoint: '/register',\n{marker}");
        assert!(
            !fired(&scan_ts(&src), RULE),
            "marker {marker:?} should suppress the finding"
        );
    }
}

// ---------------------------------------------------------------------------
// False-positive defense — prose must not trip the rule
// ---------------------------------------------------------------------------

#[test]
fn prose_mentioning_dcr_does_not_fire() {
    // This is why `present` is `registration_endpoint` alone. Measured against two real MCP
    // servers, matching "RFC 7591" / "Dynamic Client Registration" fired on file *header
    // comments* in modules that implement no registration at all.
    for prose in [
        " *   POST /register → Dynamic Client Registration (RFC 7591)",
        "// see RFC 7591 for the legacy flow",
        "/** Dynamic Client Registration is deprecated; we are migrating away. */",
        "// the registration endpoint moved to the auth service",
    ] {
        assert!(
            !fired(&scan_ts(prose), RULE),
            "prose {prose:?} must not fire"
        );
    }
}

#[test]
fn a_plain_register_route_does_not_fire() {
    // Every app with user sign-up has this route; on its own it says nothing about DCR.
    assert!(!fired(&scan_ts("app.post('/register', handler);"), RULE));
    assert!(!fired(&scan_ts("router.post('/register', signup);"), RULE));
}

#[test]
fn a_file_without_oauth_never_fires() {
    assert!(!fired(
        &scan_ts("export function add(a: number, b: number) { return a + b; }"),
        RULE
    ));
}

// ---------------------------------------------------------------------------
// Anti-overlap (A4) — these signals belong to mcp-stateless-migrator r07-oauth-hardening
// ---------------------------------------------------------------------------

#[test]
fn oauth_hardening_signals_do_not_fire_this_rule() {
    let src = r#"
const params = {
  iss: ISSUER,
  application_type: "native",
  id_token: token,
  refresh_token: refresh,
};
"#;
    let f = scan("hardening.ts", src);
    assert!(
        !fired(&f, RULE),
        "iss / application_type / id_token / refresh_token are r07-oauth-hardening's job"
    );
}

#[test]
fn redirect_uri_allowlist_does_not_fire() {
    // A local allowlist is a security control (it blocks the 1-click account-takeover class).
    // Warning about it would warn about a correct defense.
    let src = "const ALLOWED_REDIRECT_URIS = ['https://claude.ai/api/mcp/auth_callback'];\n\
               if (!ALLOWED_REDIRECT_URIS.includes(redirect_uri)) throw new Error('bad');";
    assert!(!fired(&scan_ts(src), RULE));
}

// ---------------------------------------------------------------------------
// File-level semantics
// ---------------------------------------------------------------------------

#[test]
fn fires_exactly_once_per_file() {
    // Several DCR signals in one file still produce a single file-level finding.
    let src = " *   POST /register → Dynamic Client Registration (RFC 7591)\n\
                registration_endpoint: `${BASE_URL}/register`,\n\
                // registration_endpoint again, for the second issuer\n\
                registration_endpoint: `${ALT_URL}/register`,";
    assert_eq!(count_rule(&scan_ts(src), RULE), 1);
}

#[test]
fn finding_points_at_the_first_matching_line() {
    let src = "// header\n// header\nregistration_endpoint: '/register',";
    let hit = scan_ts(src)
        .into_iter()
        .find(|f| f.rule == RULE)
        .expect("a dcr finding");
    assert_eq!(hit.line, 3);
    // File-level rules carry no source snippet.
    assert!(hit.snippet.is_empty());
}

// ---------------------------------------------------------------------------
// No deadline claimed (A3)
// ---------------------------------------------------------------------------

#[test]
fn message_and_fix_claim_no_removal_deadline() {
    // The SEP-2577 rules cite a published 12-month window. DCR has none — the release blog
    // says only "a future version". The rule must not invent one.
    let hit = scan_ts("registration_endpoint: '/register',")
        .into_iter()
        .find(|f| f.rule == RULE)
        .expect("a dcr finding");
    let text = format!("{} {}", hit.message, hit.fix).to_lowercase();
    for claim in ["12-month", "12 month", "removal window", "deadline"] {
        assert!(!text.contains(claim), "must not claim {claim:?}");
    }
    // It does say what is true: DCR still works.
    assert!(text.contains("backwards compatibility"));
}
