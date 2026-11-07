# Hannes repository rules

This repository uses unversioned, self-contained deterministic rules, three
local rule sets and a pinned external commit-message rule set and eight execution points. No reviews or provider calls run.

`checks/*/rule.toml` retains the numbered rule IDs and their settings. Each check
has an `index.sh` entrypoint using the shared offline-aware command launcher.
`rule-sets/*.toml` groups tooling regressions, native validation and dependency
audits; the remaining checks declare their own run_at. `config.toml` declares
custom points and the external import. `commit-scopes.txt` supplies its required
scopes_file parameter and matches the scopes in `.githooks/commit-policy`. The
local message check additionally enforces the narrower Hannes type allowlist and
summary whitespace requirements. Git events use their fixed, identically named built-in points.

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-commit --plan
mise check
```

The governor is pinned to published Hannes v0.9.0, which supports --auto-approve.
Do not use a candidate built from this checkout as the governor.

See [repository self-integration](../docs/self-governance.md) for setup, required
coverage, reports and CI. The general [configuration reference](../docs/configuration.md)
describes the schema. Reports remain ignored.
