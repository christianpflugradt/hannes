# Hannes repository rules

This repository uses unversioned, self-contained deterministic rules and pinned
external commit-message and Security Rule Sets at six execution points, plus
the unpinned repository-conventions Rule Set at check and pre-push. Its only
Rule compares LICENSE against the bundled license references byte-for-byte.
A conditional AGENTS.md quality Review runs only at check through a named Gemini
CLI provider. Git hooks and release-build points select checks only.

`checks/*/rule.toml` defines 16 local Rules using direct command arrays or Python
file entrypoints with an explicit interpreter. Cargo, Python helper and JavaScript
tests each have one Rule and run at pre-push and again in CI. Cargo build, test,
Clippy and MSRV commands and npm tests explicitly select offline mode.
Each local Rule declares its own run_at; there are no local Rule Sets.
`config.toml` declares custom points and the external imports.
`commit-scopes.txt` supplies the commit-message import's required
scopes_file parameter and matches the scopes in `.githooks/commit-policy`. The
local message check additionally enforces the narrower Hannes type allowlist and
summary whitespace requirements. Git events use their fixed, identically named built-in points.

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-commit --plan
hannes was ist phase --details
```

The exact published governor pin is recorded in .hannes-version. It must support
named CLI providers and file-change triggers.
Do not use a candidate built from this checkout as the governor.
Agents rely on pre-commit and pre-push feedback instead of running checks manually.
The manual `mise check` task remains available to developers and delegates to
Hannes at pre-push.

See [repository self-integration](../docs/self-governance.md) for setup, required
coverage, reports and CI. The general [configuration reference](../docs/configuration.md)
describes the schema. Reports remain ignored.

The blocking 04-agents-size Check limits AGENTS.md to 1200 words. The
[quality-review example](../docs/agents-quality-review.md) explains counting,
full-document context, the rubric, Gemini setup, triggers and CI summaries.
