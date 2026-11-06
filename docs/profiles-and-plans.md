# Execution points and plans

Rules are checks or reviews. Rule sets reference rule IDs and assign execution
points. Each active rule uses the union of its active rule sets' points; without
active membership it uses its own run_at. An overridden own list generates a hint.
Disabled rules never run. Disabled sets do not assign rules. Every reference is
validated even when disabled. See the [configuration reference](configuration.md).

Use `hannes mach was --at pre-push --plan --json` to inspect an offline plan.
The default point is pre-push. Built-in points are pre-commit, commit-msg and
pre-push; custom points must be centrally declared. Empty points are valid.

The loader records source paths. The domain resolver computes effective
assignments once. Validation, CLI planning and execution share that result.
Checks run before reviews, ordered by ID and deduplicated. Every selected rule is
required. Checks-only execution rejects required reviews before any command starts.

Plans include the selected point, rule sources, effective points, contributing
sets, required IDs, input identity, bounded context and declared provider settings.
No check, Codex request, network access or reconciliation occurs during preview.
Incomplete snapshots or context preparation block execution. Preview does not
certify a later source state. Current reports use schema 3 and plan schema 2;
historical report schemas remain readable without providing current assignments.

External Git rule sets are planned for phase 2. Loading is independent of domain
resolution so later loaders can supply rules with source provenance. No provider
framework, imports or download behavior exists yet. External source restrictions
must be enforced by the phase-2 loader before using the shared resolver.

The repository's own pinned published-governor hooks and CI still use their
published profile configuration. Their coordinated migration is phase 3. They
must not execute the candidate binary against that old configuration. The
candidate's managed hook reconciliation already passes --at and retains its
existing ownership, confirmation and argument-forwarding behavior.
