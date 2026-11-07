# Hannes repository rules

This repository uses unversioned, self-contained deterministic rules, three
shared rule sets and eight execution points. No reviews or provider calls run.

`checks/*/rule.toml` retains the numbered rule IDs and their settings. Each check
has an `index.sh` entrypoint using the shared offline-aware command launcher.
`rule-sets/*.toml` groups tooling regressions, native validation and dependency
audits; the remaining checks declare their own run_at. `config.toml` declares
custom points. Git events use their fixed, identically named built-in points.

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-commit --plan
mise check
```

The governor is pinned to published Hannes v0.7.0, which supports --auto-approve.
Do not use a candidate built from this checkout as the governor.

See [repository self-integration](../docs/self-governance.md) for setup, required
coverage, reports and CI. The general [configuration reference](../docs/configuration.md)
describes the schema. Reports remain ignored.
