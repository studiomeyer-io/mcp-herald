# Contributing to mcp-herald

Thanks for considering a contribution. `mcp-herald` warns about MCP **2026-07-28** spec
migrations, so the bar for a rule is: it maps to a specific SEP/RFC, it has a low false-
positive rate on real servers, and it ships with both a positive and a clean test case.

## Quick Start

```sh
git clone https://github.com/studiomeyer-io/mcp-herald
cd mcp-herald
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
# try it:
cargo run -- lint ./some-mcp-server
cargo run -- rules
```

MSRV is **Rust 1.86** — CI checks it on a pinned 1.86 toolchain plus stable.

## Adding a rule

A rule is a `RuleSpec` in [`src/rules.rs`](src/rules.rs):

```rust
RuleSpec {
    id: "category.short_name",          // stable, dotted
    sep: "SEP-XXXX",                    // or "RFC NNNN"
    level: Level::Warning,             // Error | Warning | Info
    kind: Kind::Line(r"regex"),        // or Kind::FilePresentAbsent { present, absent }
    message: "what changed in the spec",
    fix:     "what to do about it",
    url:     "https://… the source to verify against",
}
```

Then add tests in [`src/scanner.rs`](src/scanner.rs) using `scan_content` (no filesystem
needed): one input that **must** fire, and — critically — one realistic *compliant* input
that **must not**. A rule that flags the recommended fix (as the session rule almost did)
is a bug.

## Principles

- **Every rule cites a source.** `url` must point at the SEP/RFC/spec page. The tool's job
  is to send people to the right doc, not to be the authority.
- **Conservative matching.** Prefer unambiguous protocol/SDK signatures (`sampling/createMessage`,
  `Mcp-Session-Id`) over generic words (`sampling`, `session`).
- **Low noise beats high recall.** A linter people mute is worse than one with a gap.
