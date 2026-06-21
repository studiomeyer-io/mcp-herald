<!-- studiomeyer-mcp-stack-banner:start -->
> **Part of the [StudioMeyer MCP Stack](https://studiomeyer.io)** — Built in Mallorca 🌴 · ⭐ if you use it
<!-- studiomeyer-mcp-stack-banner:end -->

# mcp-herald

[![crates.io](https://img.shields.io/crates/v/mcp-herald.svg)](https://crates.io/crates/mcp-herald)
[![CI](https://github.com/studiomeyer-io/mcp-herald/actions/workflows/ci.yml/badge.svg)](https://github.com/studiomeyer-io/mcp-herald/actions/workflows/ci.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/studiomeyer-io/mcp-herald/badge)](https://scorecard.dev/viewer/?uri=github.com/studiomeyer-io/mcp-herald)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**A static migration linter for the [Model Context Protocol](https://modelcontextprotocol.io) 2026-07-28 spec.**

The 2026-07-28 spec is the biggest change since MCP launched: the transport goes **stateless**,
the resource-not-found error code moves from `-32002` to `-32602`, **roots / sampling / logging**
are deprecated, and OAuth servers now **must** publish Protected Resource Metadata. If you run a
server, you have until the deadline to react — and most of what breaks is invisible until a client
fails in production.

`mcp-herald` is one static binary that scans your server's **source** and tells you, file and line,
what the new spec breaks and how to fix it — every finding linked to the SEP/RFC that defines it.

```text
$ mcp-herald lint ./src
mcp-herald: 5 finding(s) in ./src — 1 error, 2 warning, 2 info

  ERROR (1)
    src/server.ts:6 — The 2026-07-28 spec moves resource-not-found from the MCP-custom code
      -32002 to the JSON-RPC standard -32602 (Invalid Params). …
      fix: Emit / expect -32602 for unknown resources; keep -32002 only behind a back-compat shim.
      SEP-2164 · https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2164
      | if (!found) throw new McpError(-32002, "Resource not found");
  WARNING (2)
    src/server.ts:3 — The 2026-07-28 transport is stateless … Sticky-session logic must be reviewed.
      …
# exit code 1 → the job fails
```

> Unlike the official [`@modelcontextprotocol/conformance`](https://github.com/modelcontextprotocol/conformance)
> suite, which black-box-tests a *running* server, `mcp-herald` reads **source** — so it works in a
> pre-merge PR check, before anything is deployed, and points you at the exact line to change. The two
> are complementary.

---

## Install

```sh
cargo install mcp-herald
```

Or build from source:

```sh
git clone https://github.com/studiomeyer-io/mcp-herald
cd mcp-herald && cargo build --release   # binary at ./target/release/mcp-herald
```

---

## Use

```sh
mcp-herald lint ./src                    # scan a tree (honors .gitignore)
mcp-herald lint ./src --fail-on warning  # gate CI on warnings too
mcp-herald lint ./src --format sarif     # GitHub code scanning, real file:line locations
mcp-herald rules                         # print the ruleset with its SEP/RFC sources
```

Exit code is `1` when a finding at or above `--fail-on` (default `error`) is present, `0` otherwise.

---

## What it checks

| rule | source | level | what it catches |
|---|---|---|---|
| `spec.error_code` | SEP-2164 | error | hardcoded `-32002` resource-not-found code (now `-32602`) |
| `transport.stateful_session` | SEP-2575 | warning | `Mcp-Session-Id` header / a stateful `sessionIdGenerator` |
| `deprecated.sampling` | SEP-2577 | warning | `sampling/createMessage` usage |
| `deprecated.roots` | SEP-2577 | warning | `roots/list` usage |
| `deprecated.logging` | SEP-2577 | warning | logging-capability usage (`logging/setLevel`, `notifications/message`) |
| `protocol.old_version` | — | info | references to superseded protocol-version dates |
| `auth.protected_resource_metadata` | RFC 9728 / 8707 | info | OAuth without Protected Resource Metadata |

Each finding links its source — `mcp-herald` sends you to the right doc, it isn't the authority.
Matching is conservative (specific protocol/SDK signatures, not generic words), and the compliant
idioms — `sessionIdGenerator: undefined`, an `/.well-known/oauth-protected-resource` endpoint — are
deliberately *not* flagged, so the tool never warns about the very fix it recommends.

---

## CI

### GitHub Action

```yaml
# .github/workflows/mcp-migration.yml
name: MCP migration
on: [pull_request]
jobs:
  herald:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: studiomeyer-io/mcp-herald@v0.1.0
        with:
          path: ./src
          fail-on: error
```

### SARIF → GitHub code scanning

```sh
mcp-herald lint ./src --format sarif > herald.sarif
# upload with github/codeql-action/upload-sarif → findings annotate the PR diff
```

---

## Scope

`mcp-herald` is a heuristic **static** analyzer: it reads source as text, never executes or
imports it, and writes nothing back. Findings are prompts to verify against the linked spec, not
verdicts. It is the migration-time companion to:

- [`mcp-covenant`](https://github.com/studiomeyer-io/mcp-covenant) — does *your own* interface stay
  backward-compatible over time (semver for MCP)?
- [`mcp-armor`](https://github.com/studiomeyer-io/mcp-armor) — runtime defense (prompt-injection,
  manifest signatures).
- [`mcp-gauntlet`](https://github.com/studiomeyer-io/mcp-gauntlet) — pre-deploy fuzz + load testing.
- [`mcp-passport`](https://github.com/studiomeyer-io/mcp-passport) — registry publish-readiness validator (the publish gate).

## License

MIT © [StudioMeyer](https://studiomeyer.io)
