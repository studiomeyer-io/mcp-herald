# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-07

Adds the registration axis of the 2026-07-28 authorization changes. The ruleset goes from
seven rules to eight.

### Added

- `auth.dcr_without_cimd` (**warning**, SEP-991 / spec 2026-07-28) — a file that advertises a
  `registration_endpoint` (Dynamic Client Registration, RFC 7591) with no sign of OAuth Client ID
  Metadata Documents. The 2026-07-28 spec deprecates DCR in favour of CIMD; DCR keeps working for
  backwards compatibility, so this is a warning and not an error, and **no removal deadline is
  claimed** — none has been published.

  The rule matches only the `registration_endpoint` metadata field. Prose such as "RFC 7591" or
  "Dynamic Client Registration" in a comment, and plain `/register` sign-up routes, are
  deliberately *not* matched: measured against two production MCP servers, the prose variants fired
  on file header comments in modules that implement no registration at all. `redirect_uri`
  allowlists are also not matched — a local allowlist is a security control, and warning about it
  would warn about a correct defense.

### Changed

- Post-release wording. The 2026-07-28 spec shipped on schedule, so the tool no longer speaks of
  it as upcoming: the `protocol.old_version` message, the crate description, the README intro and
  the crate-level docs now describe the revision as final and current. No rule logic or matched
  pattern changed. Whether `2025-11-25` itself belongs in the superseded-date set is a separate
  question and is deliberately left open.

### Note for CI

Pipelines running `--fail-on warning` (or `info`) against an OAuth-issuing MCP server will see
**new findings** after upgrading. That is intended: it is the point of the rule. The default
threshold is `error`, so pipelines on the default gate are unaffected.

### Known limitations

`mcp-herald` scans source files. An authorization server that ships
`/.well-known/oauth-authorization-server` as a static `.json` asset keeps its
`registration_endpoint` in a file the scanner does not read, so `auth.dcr_without_cimd` will not
fire on it. Widening the scanner to `.json` affects all eight rules and is not part of this
release.

The two halves of `auth.dcr_without_cimd` are held to different standards. The `present` side
ignores prose, because a comment does not prove an implementation. The `absent` side accepts any
mention of CIMD, including a comment, so a `registration_endpoint` sitting next to
`// TODO: migrate to CIMD` produces no finding. The forgiving direction is intentional, but it
means a silent file is not evidence that a migration happened. Documented in README → "On the DCR
rule"; tightening the `absent` side to implementation-bearing signals only is on the backlog.

## [0.1.0] - 2026-06-21

Initial release.

### Added

- `lint <path>` — scan a file or directory for MCP 2026-07-28 migration issues, honoring
  `.gitignore`. Human, **SARIF 2.1.0** (real `file:line` locations) and JSON output, with a
  configurable `--fail-on` severity gate for CI.
- `rules` — print the full ruleset with its SEP/RFC sources.
- Seven rules across four spec changes:
  - `spec.error_code` (SEP-2164) — hardcoded `-32002` resource-not-found code.
  - `transport.stateful_session` (SEP-2575) — `Mcp-Session-Id` / stateful `sessionIdGenerator`.
  - `deprecated.sampling` / `deprecated.roots` / `deprecated.logging` (SEP-2577).
  - `protocol.old_version` — references to superseded protocol-version dates.
  - `auth.protected_resource_metadata` (RFC 9728 / RFC 8707) — OAuth without resource metadata.
- A reusable composite GitHub Action (`action.yml`).

[Unreleased]: https://github.com/studiomeyer-io/mcp-herald/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/studiomeyer-io/mcp-herald/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/studiomeyer-io/mcp-herald/releases/tag/v0.1.0
