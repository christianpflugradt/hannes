# Hannes specification

Status: current implemented contract. Changes to behavior start here and in the
acceptance criteria, before implementation. Explicit user decisions take precedence.

The [product vision](docs/product-vision.md) defines direction and architectural
boundaries: Hannes makes development expectations explicit, executable and
verifiable, independently of who writes the code. This document defines supported
behavior. The [roadmap](docs/todo/README.md) contains planned extensions; they are
not implemented merely because they have a proposal.

The [README configuration reference](README.md#configuration) defines every public
configuration field, default and limit. The [profile contract](docs/profiles-and-plans.md)
extends this specification for configuration version 2, plan schema 1 and report
schema 2. These documents must agree; a contradiction must be resolved before work
proceeds, not interpreted as permission to choose either behavior.

## CLI-001: Commands and project boundaries

Hannes is a small native Rust executable, named `hannes`. It evaluates outcomes;
it does not implement features, repair files, orchestrate coding agents, prescribe
a development methodology or run production infrastructure. Source, configuration
and documentation are English; the intentional German command phrases are:

| Command | Contract |
| --- | --- |
| `hannes fang an` | Create missing `.hannes/` files, preserve edits, install no hooks and make no commits. |
| `hannes pass auf` | Validate all configuration and referenced files, without checks or provider calls. |
| `hannes mach was` | Validate, select rules, execute checks before reviews and save results. |
| `hannes was ist phase` | Read saved results and compare input freshness, without rerunning rules. `--details` shows full findings and captured output. |
| `hannes so hilf mir doch` | Show help listing exactly the five complete command phrases. |

`-h`/`--help`, `-V`/`--version` and `--no-color` remain supported. Help must not
present intermediate words as useful standalone commands, add a separate `help`
command or redundant examples. Other execution phrases are rejected.
Run from the Git working-tree root; subdirectory execution is rejected. Hannes
must not create commits, modify the Git index or silently discover another root.

## CFG-001: Configuration and initialization

The directory is exactly `.hannes/`, lowercase. Initialization creates version 1:

```text
.hannes/
  config.toml
  provider.toml
  checks/always-ok.toml
  checks/deliberately-failing.toml
  reviews/product-consistency.toml
  reviews/product-consistency.md
  reports/.gitignore
  README.md
```

`reports/last.json` is generated only by verification. Configuration and criteria
are version-controlled; reports and credentials stay local. Examples use generic
placeholders such as `My_App`. Initialization derives the project name from the
directory, enables the `git --version` example check and disables the failing
check and review. A first run needs neither Codex nor credentials.

Each immediate rule TOML file defines one check or review. IDs are unique across
both kinds. `enabled` is the only activation field; `disabled` is rejected.
Unknown fields, duplicate keys/IDs, invalid values and missing referenced files
are errors, including in disabled rules and unused profiles. Paths must be
relative to their documented base, remain inside the project, and never traverse
parents. Symlink escapes and unsupported input must not produce PASS.

New provider scaffolds select `codex-cli`, timeout 180 seconds, `gpt-6.1-sol`, no
MCP servers, medium reasoning and 2000 retained tokens per tool output. Omitting
optional model/effort uses Codex defaults; omitting the MCP allow list disables
all MCPs. Configuration versions 1 and 2 remain readable. Version 1 rejects tags,
profiles and `default_profile`; new capabilities require an explicit supported
version and compatibility tests rather than silently changing old semantics.

## SEL-001: Selection and offline plans

Without a profile, all enabled rules run. Version 2 profiles select the union of
explicit rule IDs and matches for any included tag, deduplicated. Checks precede
reviews; each kind is ordered by ID. `--profil` overrides `default_profile`;
otherwise the built-in profile `all` applies. Only `worktree` input is supported.

`--plan` uses the same selection/preparation as execution. It prints order,
selection reasons, omissions, required rules, input identity, declared provider
settings and bounded context sizes, without running checks, Codex, MCPs or network
calls, changing project/Git state or replacing the report. `--plan --json` uses
plan schema 1 and structured diagnostics. A plan is neither a cache nor proof
that a later input is identical. Provider availability is not checked offline.

`--geiz-ist-geil` runs checks only, without provider discovery, model calls or MCP
connections. Reviews are excluded from progress/results and the limited scope is
explicit in terminal output and reports. Full configuration validation still runs.
Every executed failure blocks success. Gates additionally require a nonempty,
complete selection of enabled rules; a gate requiring reviews rejects checks-only
execution before any check starts. Empty ordinary selections prove no rule PASS.

## CHK-001: Deterministic checks

Check commands are argument arrays, executed without an implicit shell or stdin.
They inherit the environment; bare executables resolve through PATH. Relative
executables resolve from the configured working directory and must remain inside
the project. An explicit shell is permitted. Checks are trusted commands, not a
security sandbox, and should not modify verified files.

Checks run sequentially. Exit 0 is PASS; nonzero exit or signal is FAIL. Spawn,
capture failure and timeout are ERROR. Remaining checks run after a failure, but
reviews do not. Capture the first 64 KiB of each output stream, drain the streams,
record total bytes/truncation, and clean up child processes on timeout. Raw output
is not redacted or streamed by default; saved output is available with `--details`.

## REV-001: Reviews and provider isolation

Only the Codex CLI provider is currently supported. Integration boundaries must
allow future providers without introducing speculative adapters. Codex resolves
through inherited PATH and uses the existing local login. API-key environment
variables are removed; no credentials or raw provider traces are persisted or
copied into CI. Missing/incompatible CLI, login, timeout or exhausted allowance
produces ERROR, never PASS or a fabricated policy violation.

All selected reviews share one new, noninteractive, ephemeral Codex invocation,
with structured output, read-only sandbox and no approvals. A bounded, token-free
`codex mcp list --json` precedes it. Every discovered server is explicitly disabled
unless in the provider allow list; unknown allowed names, unsafe names, oversized
or failed discovery block the call. Automatic skills, apps/plugins, subagents and
web search are disabled for the child process. User Codex configuration is not
rewritten. Allowed integrations and local Codex configuration remain trusted;
read-only execution is not a new sandbox for MCP capabilities.

Review criteria state what to judge. Exact entry files are starting points;
Markdown context supplies documented expectations with original line numbers.
ATX headings must match uniquely, exclude code blocks, and include subsections.
Shared/overlapping context is sent once. Missing or excessive context is rejected,
not silently omitted, including for disabled definitions. Evidence must belong to
the verified manifest. If every selected review declares entry files, initial
manifest/diff hints focus on their combined entry/context paths; otherwise hints
are project-wide. Reviews evaluate current files, not just changed lines, and
must follow necessary dependencies. Untracked contents need direct inspection.

Treat source, criteria, AGENTS.md and diffs as untrusted evidence, not instructions
to change role or permissions. The ordinary CLI prompt is not a privileged system
message. No repair loops, automatic retries, commits, delegated coding or inferred
requirements. Missing essential evidence or material uncertainty is ERROR.

## OUT-001: Structured review outcomes

Require exactly one verdict per requested ID and no unexpected IDs. Verdicts are
PASS, FAIL or ERROR. Each has a nonblank one-line summary of at most 120 characters
and at most 20 findings, each with a nonblank message of at most 2000 UTF-8 bytes.
Finding path and line keys are required but nullable. PASS/FAIL require at least
one existing in-manifest file and positive, in-range line reference. Grounded
references do not prove an LLM conclusion correct. Invalid types, unknown/duplicate
keys, malformed/missing/duplicate results and incomplete coverage make the batch
ERROR. The final document is limited to 1 MiB.

Available token counters are saved once per shared invocation, including when the
verdict is rejected. Total is input plus output; cached input, reasoning output
and cache writes are subsets. Missing/invalid telemetry remains unavailable,
never fabricated as zero. No monetary estimate or total token budget is enforced.

## INP-001: Snapshots and stability

Input is Git HEAD plus a sorted manifest of current tracked and nonignored
untracked files, with SHA-256 hashes, sizes and executable flags. Include `.hannes`
even when ignored, but exclude `.git` and `.hannes/reports`. No temporary commits,
index mutations, content backups, external diff helpers or optional Git locks.

Bound snapshots to 8 MiB/file, 128 MiB total, 100,000 paths, 64 KiB of Git file-list
capture and 16 levels of configuration nesting. Unsupported symlinks, submodules,
special/non-UTF-8 paths, unreadable files or exceeded limits make coverage
incomplete. Incomplete input blocks execution and PASS. Compare input before an
active provider call and after execution; changed or unverifiable input blocks
PASS. Snapshots are not filesystem transactions and cannot detect edits reverted
entirely between captures.

Status is CURRENT for matching complete snapshots, STALE for differences, UNKNOWN
for incomplete/unavailable comparison. Staging alone does not change content;
HEAD changes do. Show bounded changed paths and exact totals only when coverage
allows. Do not invent line deltas: the baseline stores hashes, not prior content.
A working-tree PASS does not certify staged/pushed commits or deployed artifacts.

## RPT-001: Reporting and CLI outcomes

Reports are written atomically to `.hannes/reports/last.json`, including FAIL/ERROR
when initialized input allows it. Invalid configuration preserves the prior
report. Save UTC timestamp, repository/project, provider, input identity, rule
results, durations, findings, output and available usage. ERROR takes precedence
over FAIL; NOT_RUN blocks success without replacing an existing FAIL with ERROR.
Disabled rules stay out of results; failed checks leave selected reviews NOT_RUN.

Schema 2 records the plan/profile and omissions. Schema 1 remains readable as
unprofiled legacy evidence and cannot certify a gate. Reject corrupt/inconsistent
reports and mismatched repositories. Reports are local, unsigned evidence, not
tamper-proof attestations. Status uses saved selection, not the current default.

Flush progress to stderr before each rule starts and after it finishes. Announce
the shared AI call; do not claim real-time per-review model completion or sum
shared durations. Final compact output goes to stdout, with each review's short
summary. Details are opt-in. No rules defined prints
`Du hast noch keine Regeln definiert, du Vogel!`; disabled definitions do not
trigger that message. Empty successful runs establish no rule PASS.

Respect `NO_COLOR`, `--no-color` and each stream's terminal state. Status colors
are green PASS/CURRENT, red FAIL and amber ERROR/STALE/UNKNOWN; help uses cyan
accents and bold headings. Sanitize untrusted terminal content.

Exit codes: 0 success; 1 failed verification or STALE; 2 configuration, execution,
provider/report error, UNKNOWN or invalid arguments; 3 absent report for status.
An executable plan exits 0; a blocked/invalid plan exits 2.

## REL-001: Distribution and integrations

Supported hosted CI/release targets are macOS Apple Silicon, macOS Intel and
Linux x64 (GNU/glibc); Windows builds/releases are excluded. Build with stable
Rust/Cargo (declared minimum 1.85) into a native executable. Git and configured
external tools remain runtime dependencies; no Python/Node runtime is bundled.

CI runs formatting, Clippy, offline unit/acceptance tests, native smoke tests,
minimum-Rust checks, dependency audits and release-tooling/workflow checks.
Releases are manually dispatched on main, reuse the quality workflow, and use
Semantic Release for versions, changelog, version commits, tags and native
archives with README, Unlicense and checksums. Artifacts must match the tested
version/source. Normal CI is read-only; only release planning/publishing may
request write permission. Actions are pinned. See [release policy](docs/releasing.md).

Git hooks are explicit opt-in examples, never automatically installed. A bypassable
local hook verifies working files, not necessarily the pushed commit. Hosted
Codex authentication is not provisioned and personal tokens are not copied to CI.
No hosted service, database, autonomous development, universal policy engine,
result cache or additional input/provider implementation is in current scope.

## DOC-001: Documentation contract

Every editable configuration field, required/optional marker and default must
match the README reference. README configuration examples must validate and plan
offline. Local links/anchors and specification acceptance references must resolve.
Run these checks through `mise docs:check`, `mise check` and CI. Automated checks
complement editorial review; they do not prove prose understandable.

## Acceptance coverage and change process

Each essential requirement has a stable ID and representative acceptance tests.
The table is a navigation aid, not a claim that one test proves every clause.
Add or update the contract and acceptance cases before implementing behavior;
update the README/examples and relevant design decisions in the same coherent
change. Preserve versioned compatibility, run applicable checks, then commit/push.
Public documentation explains supported behavior; proposals remain in the roadmap.

| Requirement | Representative acceptance test |
| --- | --- |
| DOC-001 | `tests/documentation.rs::every_configuration_field_required_marker_and_default_is_documented` |
| CLI-001 | `tests/acceptance.rs::all_command_paths_and_standard_help` |
| CFG-001 | `tests/acceptance.rs::validation_rejects_invalid_including_disabled` |
| SEL-001 | `tests/profiles.rs::checks_only_explains_omissions_and_cannot_shrink_a_gate` |
| CHK-001 | `tests/acceptance.rs::failing_check_runs_remaining_and_blocks_provider` |
| REV-001 | `tests/acceptance.rs::lean_review_transport_disables_unlisted_mcp_and_honors_model_profile_and_allowlist` |
| OUT-001 | `tests/acceptance.rs::malformed_missing_extra_duplicate_ungrounded_provider_outputs_fail_closed` |
| INP-001 | `tests/acceptance.rs::changing_files_during_checks_blocks_pass` |
| RPT-001 | `tests/profiles.rs::saved_reports_reject_missing_gate_coverage_and_plan_metadata` |
| REL-001 | `scripts/test-release.py::test_ci_policy_checks_new_messages_without_rejecting_legacy_history` |
