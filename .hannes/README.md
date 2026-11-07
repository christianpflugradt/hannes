# Hannes repository rules

This repository uses unversioned, self-contained deterministic rules and pinned
external commit-message and Security Rule Sets at six execution points.
No reviews or provider calls run.

`checks/*/rule.toml` defines 15 local Rules using direct command arrays or Python
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

The governor is pinned to published Hannes v0.10.0, which supports direct commands and shared script paths.
Do not use a candidate built from this checkout as the governor.
Agents rely on pre-commit and pre-push feedback instead of running checks manually.
The manual `mise check` task remains available to developers and delegates to
Hannes at pre-push.

See [repository self-integration](../docs/self-governance.md) for setup, required
coverage, reports and CI. The general [configuration reference](../docs/configuration.md)
describes the schema. Reports remain ignored.
