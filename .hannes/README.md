# Hannes repository rules

This repository uses configuration version 3 with deterministic rules, three
shared rule sets and eight execution points. No reviews or provider calls run.

`checks/` retains the numbered rule IDs and their commands/settings.
`rule-sets.toml` groups tooling regressions, native validation and dependency
audits; the remaining checks declare their own run_at. `config.toml` declares
custom points and maps each Git event to its corresponding built-in point.

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-commit --plan
mise check
```

The governor is pinned to published Hannes v0.5.0, which supports --auto-approve.
Do not use a candidate built from this checkout as the governor.

See [repository self-integration](../docs/self-governance.md) for setup, required
coverage, reports and CI. The general [configuration reference](../docs/configuration.md)
describes the schema. Reports remain ignored.
