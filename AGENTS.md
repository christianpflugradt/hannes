# Working on Hannes

## Core principles

Consistency and no AI slop are the highest quality priorities for every change.

- Follow the user's decisions and established project terminology. Keep code,
  CLI behavior, configuration, examples and documentation consistent; update all
  affected places when a convention changes.
- Implement the requested behavior precisely. Do not invent names, aliases,
  features, requirements or abstractions without a concrete need. When intent
  is unclear, inspect existing decisions and ask rather than fabricate.
- Write clear, concise and complete documentation. Avoid filler, buzzwords,
  boilerplate and unsupported claims. Examples must be usable, generic and
  consistent with the actual implementation.
- Before finishing, check the whole change for contradictions, stale references
  and unnecessary additions. Prefer the smallest coherent solution.
- Explain user-facing concepts before using them. Give readers a clear path from
  purpose and setup to everyday use and the complete configuration reference.
  Keep hidden behavior out of public documentation.

## Contract and scope

- Read docs/product-vision.md and SPECIFICATION.md before changing behavior or
  architecture. The vision defines product direction and architectural boundaries;
  the specification defines the supported behavior. Evaluate proposed changes
  against the vision's guiding question. Future possibilities are not permission
  to implement speculative features. Explicit user instructions take precedence;
  the project license is Unlicense.
- Develop from the specification: define behavior and acceptance criteria before
  implementation, and keep the specification, tests and README aligned in the same
  coherent change. Resolve contradictions rather than choosing a convenient reading.
- Keep the CLI small and agent-agnostic. Do not add repair loops, services or
  speculative provider infrastructure.
- Preserve the exact nested German command paths and lowercase `.hannes/` directory.
  The execution command is `hannes mach was`; do not add alternative phrases.
  Source, configuration, file names and documentation stay in English.
- Deterministic checks run first. Provider failures, invalid JSON and incomplete
  snapshot coverage must never become PASS. Disabled items stay out of results.
- Commit and push completed coherent changes automatically as described below.
  Never tag, publish, install hooks or change other remote services unless explicitly
  requested. Do not run real Codex reviews without an explicit request.

## Workflow runs

- Never monitor, poll, watch or wait for GitHub Actions or other remote workflow
  runs unless the user explicitly asks for monitoring.
- If further work depends on a workflow finishing, pause and ask the user to let
  you know when it has finished. Continue after the user provides that update.
- Apply this rule in every session, including fresh contexts. A request to run,
  validate, commit or push changes does not authorize workflow monitoring.

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
lean and Cargo.lock current. Keep tests focused on the single supported
configuration model. See docs/design.md for module boundaries and limits.

Use the installed Hannes Git hooks for validation. Do not manually run checks,
tests, linters or builds, including `mise run check`, `mise docs:check`, Cargo/npm
test commands or direct `hannes mach was` execution. Pre-commit checks formatting
and documentation; pre-push runs the complete local suite. Formatting commands
that edit source are allowed. Read hook output and, when needed, inspect the saved
report with `hannes was ist phase --details`; this does not rerun checks.
Tests must not call Codex, the network or a paid API; Cargo may fetch build
dependencies during explicit preparation. Keep README approachable, concise and
complete for supported CLI commands and every user-editable `.hannes` field,
default and limit.
Update it alongside behavior/configuration changes, clearly separating current
features from planned work. Implementation details belong in docs/.
Keep LICENSE and Cargo.toml license metadata aligned. Do not add machine-specific
paths, credentials or per-developer reports to versioned files. Use generic
placeholders such as `My_App` in public documentation and examples; do not name
private projects.

## Git conventions

- The main branch is `main` locally and on `origin`.
- Use only Conventional Commit types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`,
  `refactor`, `revert`, `style`, `test`. No additional types or automatic merge-message exceptions.
- Release policy: `feat` is minor; `fix`, `perf`, `chore`, `build`, `refactor`
  and effective `revert` are patch. `ci`, `docs`, `style`, `test` do not release.
  Hannes starts at 0.1.0. During 0.x, breaking changes increment minor; stable
  1.0.0 requires an explicit policy change. After graduation, breaking changes are major. Reserve `build(release)` for generated
  version commits, which do not trigger another release. See docs/releasing.md.
- Header: `type(scope): summary` (scope optional; breaking-change `!` supported).
  When present, scope must be one of `checks`, `ci`, `cli`, `config`, `deps`,
  `docs`, `git`, `mise`, `mood`, `profiles`, `provider`, `release`, `report`,
  `reviews`, `snapshot`, `tests`. `.githooks/commit-policy` is the source of truth.
  See CONTRIBUTING.md. Install the project Git hooks explicitly with
  `sh scripts/setup-git-hooks.sh`; never bypass it to force an invalid message.
- After completing each coherent change, commit it on `main` using the allowed
  commit types and push `main` to `origin` automatically. Git hooks run validation;
  do not run the same checks before committing or pushing.
  This applies to documentation-only changes too; no further approval is needed.
- Stage only the files belonging to that change. Preserve unrelated user edits,
  keep commits focused, and never bypass hooks or force-push to complete this flow.
- If pre-commit or commit-msg fails, fix the reported problem and retry the commit.
  If pre-push fails, fix the problem and amend only your own unpublished commit
  when appropriate, then retry the push. Do not reset or rewrite other work.
  Report missing tools or broken hooks if they cannot be repaired within the
  authorized scope; never replace hook validation with manual check runs.
- Report the commit and push result. If validation, commit or push fails, retain
  the work and report the failure; do not claim that an unpushed change is pushed.
