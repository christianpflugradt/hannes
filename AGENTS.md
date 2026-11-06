# Working on Hannes

## Contract and scope

- Read SPECIFICATION.md before changing behavior. Explicit user instructions take
  precedence; the project license is Unlicense.
- Keep the CLI small and agent-agnostic. Do not add repair loops, services or
  speculative provider infrastructure.
- Preserve the exact nested German command paths and lowercase `.hannes/` directory.
  Source, configuration, file names and documentation stay in English.
- Deterministic checks run first. Provider failures, invalid JSON and incomplete
  snapshot coverage must never become PASS. Disabled items stay out of results.
- Commit and push completed coherent changes automatically as described below.
  Never tag, publish, install hooks or change other remote services unless explicitly
  requested. Do not run real Codex reviews without an explicit request.

## Code discovery

Prefer codebase-memory-mcp when available. At session start, confirm the nearest
project and index generation with `list_projects` or `index_status`.
Use Verify-tier, task-directed evidence by default: `search_graph`, relevant
`trace_path` directions, then `get_code_snippet`. Use `query_graph` for complex
relationships and `get_architecture` for an overview.
Call `check_index_coverage` with all evidence paths before relying on graph claims;
include scopes for negative/exhaustive claims and follow relevant pagination.
Read or search source for stale, partial, skipped, excluded or unknown coverage.
A clean coverage result is a best-effort signal, not proof of completeness.
If the tools are unavailable, say so and inspect source directly. Local MCP config
and graph data are ignored; they are not prerequisites for building Hannes.

## Changes and validation

Prefer focused functions and tests over framework abstractions. Keep dependencies
lean and Cargo.lock current. See docs/design.md for module boundaries and limits.

For Rust changes, run `mise run check`, or the equivalent commands in
CONTRIBUTING.md. Tests must not call Codex, the network or a paid API; Cargo may
fetch build dependencies. For documentation/config-only changes, validate affected
formats, links and commands. Keep README approachable, concise and complete for
supported CLI commands and every user-editable `.hannes` field, default and limit.
Update it alongside behavior/configuration changes, clearly separating current
features from planned work. Implementation details belong in docs/.
Keep LICENSE and Cargo.toml license metadata aligned. Do not add machine-specific
paths, credentials or per-developer reports to versioned files.

## Git conventions

- The main branch is `main` locally and on `origin`.
- Use only Conventional Commit types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`,
  `refactor`, `revert`, `style`, `test`. No additional types or automatic merge-message exceptions.
- Release policy: `feat` is minor; `fix`, `perf`, `chore`, `build`, `refactor`
  and effective `revert` are patch. `ci`, `docs`, `style`, `test` do not release.
  Breaking changes are major for every type. Reserve `build(release)` for generated
  version commits, which do not trigger another release. See docs/releasing.md.
- Header: `type(scope): summary` (scope optional; breaking-change `!` supported).
  When present, scope must be one of `checks`, `ci`, `cli`, `config`, `deps`,
  `docs`, `git`, `mise`, `mood`, `profiles`, `provider`, `release`, `report`,
  `reviews`, `snapshot`, `tests`. `.githooks/commit-policy` is the source of truth.
  See CONTRIBUTING.md. Install the local `commit-msg` hook with
  `sh scripts/setup-git-hooks.sh`; never bypass it to force an invalid message.
- After completing each coherent change, run the applicable validation, commit it
  on `main` using the allowed commit types, and push `main` to `origin` automatically.
  This applies to documentation-only changes too; no further approval is needed.
- Stage only the files belonging to that change. Preserve unrelated user edits,
  keep commits focused, and never bypass hooks or force-push to complete this flow.
- Report the commit and push result. If validation, commit or push fails, retain
  the work and report the failure; do not claim that an unpushed change is pushed.
