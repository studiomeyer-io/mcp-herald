# Security Policy

## Reporting a vulnerability

Please report security issues privately to **security@studiomeyer.io** or via GitHub's
private vulnerability reporting ("Report a vulnerability" in the Security tab). We aim to
acknowledge within 72 hours.

## Scope & intent

`mcp-herald` is a **static analyzer**. It reads source files from a path you give it and
matches them against a fixed set of text patterns. It does not execute, import, compile or
network-fetch anything it scans, and it writes nothing back — output goes to stdout only.

Findings are heuristic prompts to verify against the linked spec, not authoritative verdicts;
a finding is never a reason to ship an automated change without review.

## Safety properties

- `#![forbid(unsafe_code)]` across the crate.
- No code execution: input files are treated purely as text.
- Files over 2 MiB and non-UTF-8 files are skipped; `.gitignore` and hidden files are honored
  via the `ignore` crate, so vendored/build output is not scanned by default.
- The ruleset is static, compiled-in data — there is no remote rule fetch.
