//! The migration ruleset — the curated heart of `mcp-herald`.
//!
//! Each rule encodes one signature of a change introduced by the **MCP 2026-07-28** spec,
//! tied to the SEP / RFC that defines it, with a concrete fix. Rules are intentionally
//! conservative grep-style matchers, not a type checker: a finding is a *prompt to verify*,
//! and every one links the source so you can confirm it against the spec.
//!
//! Sources are recorded per rule (the `url` field). The flagship facts:
//! resource-not-found moved from the MCP-custom `-32002` to the JSON-RPC standard `-32602`
//! (SEP-2164); the transport became stateless (SEP-2575); roots / sampling / logging were
//! deprecated (SEP-2577); OAuth servers must publish Protected Resource Metadata (RFC 9728)
//! and honor Resource Indicators (RFC 8707).

use regex::Regex;
use serde::Serialize;

/// Severity of a finding, ordered so `--fail-on` thresholds compare naturally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Worth knowing; not a hard break.
    Info,
    /// Likely to need a change before 2026-07-28.
    Warning,
    /// Will break against the new spec if left as-is.
    Error,
}

impl Level {
    /// Short label for human output.
    pub fn label(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

/// How a rule decides whether a file (or a line) trips it.
#[derive(Debug, Clone)]
pub enum Kind {
    /// Fires once per matching line. Carries a line + column location.
    Line(&'static str),
    /// Fires once per file when `present` matches **and** `absent` does not — for
    /// "you do X but not the required Y" checks (e.g. OAuth without resource metadata).
    FilePresentAbsent {
        /// Pattern that signals the file is in scope.
        present: &'static str,
        /// Pattern whose presence means the file is already compliant.
        absent: &'static str,
    },
}

/// A rule definition (static data).
#[derive(Debug)]
pub struct RuleSpec {
    /// Stable rule id, e.g. `spec.error_code`.
    pub id: &'static str,
    /// The SEP/RFC this rule is derived from.
    pub sep: &'static str,
    /// Severity.
    pub level: Level,
    /// How it matches.
    pub kind: Kind,
    /// What changed.
    pub message: &'static str,
    /// What to do about it.
    pub fix: &'static str,
    /// Source to verify against.
    pub url: &'static str,
}

/// The full ruleset.
pub static RULES: &[RuleSpec] = &[
    RuleSpec {
        id: "spec.error_code",
        sep: "SEP-2164",
        level: Level::Error,
        kind: Kind::Line(r"-32002\b"),
        message: "The 2026-07-28 spec moves resource-not-found from the MCP-custom code \
                  -32002 to the JSON-RPC standard -32602 (Invalid Params). Error handling \
                  that hardcodes -32002 will mismatch.",
        fix: "Emit / expect -32602 for unknown resources; keep -32002 only behind an \
              explicit back-compat shim.",
        url: "https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2164",
    },
    RuleSpec {
        id: "transport.stateful_session",
        sep: "SEP-2575",
        level: Level::Warning,
        // The Mcp-Session-Id header, or a sessionIdGenerator wired to an actual generator.
        // The compliant `sessionIdGenerator: undefined` form is deliberately NOT matched —
        // and `\(\s*\)` (empty-arg arrow) avoids flagging `(x ?? undefined)`.
        kind: Kind::Line(
            r"(?i)mcp-session-id|sessionidgenerator\s*:\s*(?:\(\s*\)|async|randomuuid|uuid|nanoid|crypto|generate)",
        ),
        message: "The 2026-07-28 stateless-transport wave (SEP-2575 and the related session \
                  changes) removes the initialize/session handshake, and the Mcp-Session-Id \
                  header is going away. Sticky-session logic must be reviewed so the server \
                  works per-request.",
        fix: "In the TS SDK set `sessionIdGenerator: undefined` and `enableJsonResponse: \
              true`, create a fresh server+transport per request, and drop any session store.",
        url: "https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/",
    },
    RuleSpec {
        id: "deprecated.sampling",
        sep: "SEP-2577",
        level: Level::Warning,
        kind: Kind::Line(r"sampling/createMessage|CreateMessageRequest"),
        message: "Sampling is deprecated in the 2026-07-28 spec (12-month removal window).",
        fix: "Replace server-initiated sampling with a direct LLM API call where you control \
              the model and budget.",
        url: "https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md",
    },
    RuleSpec {
        id: "deprecated.roots",
        sep: "SEP-2577",
        level: Level::Warning,
        kind: Kind::Line(r"roots/list|ListRootsRequest"),
        message: "Roots is deprecated in the 2026-07-28 spec (12-month removal window).",
        fix: "Pass paths as explicit tool parameters or resource URIs instead of relying on \
              client roots.",
        url: "https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md",
    },
    RuleSpec {
        id: "deprecated.logging",
        sep: "SEP-2577",
        level: Level::Warning,
        // `notifications/message` only as a quoted method literal — not a REST route
        // ("/notifications/message") or a prose comment.
        kind: Kind::Line(r#"logging/setLevel|SetLevelRequest|["']notifications/message"#),
        message: "Protocol logging is deprecated in the 2026-07-28 spec (12-month removal \
                  window).",
        fix: "Log to stderr (stdio servers) or emit OpenTelemetry spans instead of the \
              logging capability.",
        url: "https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2577-deprecate-roots-sampling-and-logging.md",
    },
    RuleSpec {
        id: "protocol.old_version",
        sep: "—",
        level: Level::Info,
        kind: Kind::Line(r"2024-11-05|2025-03-26|2025-06-18"),
        message: "References an older MCP protocol-version date. 2025-11-25 is current and \
                  the 2026-07-28 final is approaching.",
        fix: "Advertise the latest protocol version you support and negotiate down; avoid \
              hardcoding an old date in the handshake.",
        url: "https://modelcontextprotocol.io/specification",
    },
    RuleSpec {
        id: "auth.protected_resource_metadata",
        sep: "RFC 9728 / RFC 8707",
        level: Level::Info,
        // `WWW-Authenticate` is the strong, low-false-positive signal of an OAuth resource
        // server (it sends that header on 401). Broader OAuth-metadata field names were
        // dropped because they also appear in comments/docs without a real implementation.
        kind: Kind::FilePresentAbsent {
            present: r"(?i)www-authenticate",
            absent: r"(?i)oauth-protected-resource|protected.?resource.?metadata|resource_metadata|resource_?indicator|8707|9728",
        },
        message: "This file looks like an OAuth resource server (it sends WWW-Authenticate) \
                  but shows no Protected Resource Metadata. The MCP authorization spec \
                  requires RFC 9728 (a /.well-known/oauth-protected-resource discovery \
                  endpoint) and RFC 8707 Resource Indicators.",
        fix: "Serve oauth-protected-resource metadata pointing at your auth server, and \
              include the resource indicator in token requests to prevent audience confusion.",
        url: "https://workos.com/blog/mcp-2026-spec-agent-authentication",
    },
];

/// A rule compiled for matching.
#[derive(Debug)]
pub struct CompiledRule {
    /// The originating spec.
    pub spec: &'static RuleSpec,
    /// Compiled matcher.
    pub kind: CompiledKind,
}

/// Compiled form of [`Kind`].
#[derive(Debug)]
pub enum CompiledKind {
    /// Per-line regex.
    Line(Regex),
    /// File-level present/absent pair.
    FilePresentAbsent {
        /// In-scope signal.
        present: Regex,
        /// Already-compliant signal.
        absent: Regex,
    },
}

/// Compile the full ruleset. Patterns are authored in this crate, so a failure is a bug —
/// surfaced via `expect` rather than a `Result` the caller must thread through.
pub fn compile() -> Vec<CompiledRule> {
    RULES
        .iter()
        .map(|spec| {
            let kind = match &spec.kind {
                Kind::Line(p) => CompiledKind::Line(build(p, spec.id)),
                Kind::FilePresentAbsent { present, absent } => CompiledKind::FilePresentAbsent {
                    present: build(present, spec.id),
                    absent: build(absent, spec.id),
                },
            };
            CompiledRule { spec, kind }
        })
        .collect()
}

fn build(pattern: &str, rule_id: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("invalid built-in regex for rule {rule_id}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rules_compile() {
        let compiled = compile();
        assert_eq!(compiled.len(), RULES.len());
    }

    #[test]
    fn rule_ids_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for r in RULES {
            assert!(seen.insert(r.id), "duplicate rule id {}", r.id);
        }
    }

    #[test]
    fn every_rule_has_a_source_url() {
        for r in RULES {
            assert!(r.url.starts_with("https://"), "rule {} lacks a url", r.id);
            assert!(!r.fix.is_empty(), "rule {} lacks a fix", r.id);
        }
    }
}
