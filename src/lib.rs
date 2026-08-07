//! # mcp-herald
//!
//! A static migration linter for the **MCP 2026-07-28** spec. Point it at your server's
//! source tree and it reports the [breaking-change signatures](crate::rules) you need to
//! address for the new spec: the retired `-32002` error code, stateful-session reliance,
//! deprecated roots / sampling / logging, OAuth without Protected Resource Metadata, and
//! Dynamic Client Registration without Client ID Metadata Documents.
//!
//! Every finding links the SEP/RFC it derives from — it's a prompt to verify, not a verdict.
//! The [`scanner::scan_content`] entry point is pure (no filesystem), so the ruleset is
//! exhaustively unit-tested.
//!
//! Companion to [`mcp-covenant`](https://github.com/studiomeyer-io/mcp-covenant) (does *your
//! own* interface stay compatible over time) — `mcp-herald` answers the other question: does
//! your server break against the *new spec*.
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod finding;
pub mod report;
pub mod rules;
pub mod sarif;
pub mod scanner;

pub use finding::{Finding, Report};
pub use rules::{compile, CompiledRule, Level, RuleSpec, RULES};
pub use scanner::{scan_content, scan_path};
