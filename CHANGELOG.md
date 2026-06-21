# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/studiomeyer-io/mcp-herald/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/studiomeyer-io/mcp-herald/releases/tag/v0.1.0
