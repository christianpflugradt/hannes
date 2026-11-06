# TODO 1: Profiles and execution plans

Historical record: profiles were replaced by rules, rule sets and execution points
in configuration version 3. The implementation details below describe the earlier
version-2 design, not current configuration.

Status: DONE; implementation, documentation and validation completed and pushed.
Depends on: none. Next: [TODO 2](02-review-mcp-and-sessions.md).

## Goal and decisions

Make rule selection explicit and inspectable before any expensive execution.
The authoritative detailed contract is
[profiles and execution plans](../profiles-and-plans.md).

- Version 2 adds common rule tags and `.hannes/profiles/*.toml` definitions.
- A profile selects the union of explicit IDs and any matching tag, deduplicated.
  Checks precede reviews; each kind is ordered by ID.
- `--profil` overrides `default_profile`. Reserved `all` preserves the current
  selection of all enabled rules when no custom default exists.
- Gates require every selected rule; disabled requirements and empty gates fail.
  Ordinary profiles explain disabled omissions. Executed failures always block.
- `--geiz-ist-geil` narrows ordinary runs to checks. It rejects gates requiring
  reviews before executing any check.
- `--plan` renders selection, omissions, input identity, context and declared
  provider settings offline. `--plan --json` exposes the same versioned plan.
- Initially support only worktree input and the existing shared review call.
  Other inputs remain explicit errors until TODO 3.

## Work

- [x] Add strict version-2 config, tags and profile validation; retain version 1.
- [x] Implement one deterministic planner shared by preview and execution.
- [x] Add CLI flags, readable plans and structured JSON diagnostics.
- [x] Record plan identity, profile, scheduled rules and omissions in reports;
      keep legacy reports readable and visibly unprofiled.
- [x] Update configuration, usage and generated examples without silently
      migrating existing projects.
- [x] Add acceptance tests for all cases in the detailed specification.

## Done when

The documented commands actually work. Reordering files/selectors cannot change
the semantic plan. Invalid selectors and incomplete gates execute nothing.
Preview executes neither checks nor provider discovery and preserves the last
report. Tests demonstrate version-1 compatibility, checks-only zero-provider
behavior, stable plan IDs and honest historical status rendering.

Implementation commit: `18897c9` (`feat(profiles): add rule selection and offline execution plans`).

Validation: `mise check` passed, including rustfmt, Clippy, 80 Rust tests and CLI
smoke tests. All 80 Rust tests also passed in Linux with Rust 1.85. README TOML
examples and local links were checked; the documented version-2 setup was
validated using the built CLI. No real provider or paid API was called.
