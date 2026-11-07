# Execution points and plans

Rules are checks or reviews. Rule sets reference rule IDs and assign execution
points. Each enabled rule uses the union of its Rule Sets' points; without
membership it uses its own run_at. An overridden own list generates a hint.
Disabled Rules never run. Present Rule Sets always apply. Every reference is
validated even when disabled. See the [configuration reference](configuration.md).

Use `hannes mach was --at pre-push --plan --json` to inspect an offline plan.
The default point is pre-push. Built-in points are pre-commit, commit-msg, pre-push and
check. Enabled effective assignments to the first three require managed hooks;
no explicit hook declaration is needed. check is invoked only explicitly with
`hannes mach was --at check`, without inspecting or synchronizing Git hooks.
Direct custom points behave the same way, including on fresh CI checkouts with
missing or outdated hooks. `hannes pass auf` manages hook installation and updates.
Hannes generates no workflow and requires no CI integration. Custom points, including release, must be centrally declared.
Empty points are valid.

The loader records source paths. The domain resolver computes effective
assignments once. Validation, CLI planning and execution share that result.
Checks run before reviews, ordered by ID and deduplicated. Every selected rule is
required. Checks-only execution rejects required reviews before any command starts.

Plans include the selected point, rule sources, effective points, contributing
sets, required IDs, input identity, bounded context and declared provider settings.
No check, Codex request, network access or reconciliation occurs during preview.
Incomplete snapshots or context preparation block execution. Preview does not
certify a later source state. Reports use schema 4 and plans use schema 3. Blocking metadata distinguishes
completed advisory failures from fatal errors; disabled Rules are listed separately.

External Git rule sets use the same resolver in isolated directory scopes. The
loader maps effective points per import, then namespaces validated rules before
combining the effective result. Mapping accepts only used custom sources and
consumer-declared custom targets. Unmapped names retain their meaning; many-to-one
mappings deduplicate execution. Built-ins cannot be mapped. Plans
use previously validated cached revisions; pass auf refreshes sources. Origins
retain the resolved commit. See [external sets](external-rule-sets.md).

The repository's self-integration retains its published governor and existing
configuration until a separate release and self-migration. Its CI setup calls pass auf --auto-approve before executing the configured
point. Published v0.5.0 provides that option and is the exact governor pin. See [repository self-integration](self-governance.md).
