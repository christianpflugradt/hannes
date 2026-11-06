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
| `hannes pass auf` | Validate configuration and reconcile declared external state after confirmation, without checks or provider calls. |
| `hannes mach was` | Validate, reconcile, select rules, execute checks before reviews and save results. |
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
The intentional pre-1.0 command-string format break in CHK-001 applies to both
versions; old array-based commands are rejected, never silently migrated.

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

Check commands are strings parsed into one executable and its arguments, executed
without an implicit shell or stdin. Arrays are rejected in both configuration
versions. Support whitespace separation, single/double quotes, escaped whitespace
and quotes, empty quoted arguments, and quoted/escaped literal special characters.
Reject empty commands/missing executable names, unmatched quotes, trailing escapes, unquoted
shell operators (`&&`, `||`, `;`, pipes, redirection, background execution),
substitution, variable expansion and unquoted glob patterns during validation,
including disabled checks. Single quotes protect all literal content; double
quotes protect operators/globs but not unescaped shell expansions. Never expand
variables, globs or substitutions. Errors identify the affected check and explain
how to use a task runner or dedicated script for command composition.
They inherit the environment; bare executables resolve through PATH. Relative
executables resolve from the configured working directory and must remain inside
the project. Validation does not require an executable to exist or be available;
missing/non-executable programs fail at runtime. An explicit shell is permitted
but task runners/dedicated scripts are the recommended composition model.
Checks are trusted commands, not a
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

## PREP-001: Shared review input

Planning prepares bounded requirement excerpts, manifest hints and orientation
diff once. Execution passes that same prepared input to the provider; prompt
rendering must not reread context or recalculate the diff. Snapshot stability
checks still run before the call and after execution; preparation is not a cache
or permission to run against changed files. The provider boundary owns transport
and provider settings, not a second interpretation of evidence selection.

## OUT-001: Structured review outcomes

Require exactly one verdict per requested ID and no unexpected IDs. Verdicts are
PASS, FAIL or ERROR. Each has a nonblank one-line summary of at most 120 characters
and at most 20 findings, each with a nonblank message of at most 2000 UTF-8 bytes.
Finding path and line keys are required but nullable. PASS/FAIL require at least
one existing in-manifest file and positive, in-range line reference. Grounded
references do not prove an LLM conclusion correct. Invalid types, unknown/duplicate
keys, malformed/missing/duplicate results and incomplete coverage make the batch
ERROR. The final document is limited to 1 MiB.

Provider schema, wire types and local validation share one response contract and
limits. The schema constrains requested IDs/count, verdicts, summary length/line
shape, finding count and positive line numbers before generation. UTF-8 byte
limits, exact unique ID coverage and real evidence remain local checks. Schema
compatibility errors are ERROR; constraints are never silently weakened.

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
Linux x64 and ARM64 (GNU/glibc); Windows builds/releases are excluded. Linux ARM64
uses the native `ubuntu-24.04-arm` runner and `aarch64-unknown-linux-gnu` target,
with the same formatting, lint, tests, build and executable smoke gates. Build with stable
Rust/Cargo (declared minimum 1.85) into a native executable. Git and configured
external tools remain runtime dependencies; no Python/Node runtime is bundled.

`mise deploy` downloads the latest published GitHub release for the detected OS
and CPU (the four targets above), verifies SHA-256 and executable compatibility,
and installs to `${CARGO_INSTALL_ROOT:-$HOME/.local}/bin/hannes`. It never builds
local source. Unsupported platforms, missing downloads, invalid checksums and
incompatible binaries fail with an actionable error and preserve the installed binary.

Push/PR CI runs the Linux x64 native gates; release runs all four native hosts.
CI runs formatting, Clippy, offline unit/acceptance tests, native smoke tests,
minimum-Rust checks, dependency audits and release-tooling/workflow checks.
Releases are manually dispatched on main and reuse the quality workflow. The
initial release is 0.1.0, published through GitHub CLI only after artifact/source
verification; subsequent releases use Semantic Release. During 0.x, breaking
changes/features increment minor and fixes/maintenance increment patch. Stable
1.0.0 requires explicit policy graduation. Publish native archives with README,
Unlicense and checksums. Artifacts must match the tested
version/source. Normal CI is read-only; only release planning/publishing may
request write permission. Actions are pinned. See [release policy](docs/releasing.md).

This repository governs its quality gates through a published Hannes executable,
never a binary built from the checked source. `.hannes` defines explicit version-2
gate profiles, with `pre-push` as default and no reviews. Its command representation
must match the pinned published governor and CHK-001.
`.hannes-version` is the sole CI/release pin, updated through Renovate GitHub-release
PRs without automerge. New gate configuration features require published support.

Local hooks never install tools or access the network. They verify working files,
not exact staged/pushed trees. The commit-msg gate uses the same standalone policy
checker as CI. CI runs named profiles and preserves reports on failure, with a
companion execution record for governor version/digest and checked commit; report
schema 2 is unchanged. Missing tools, incomplete selection and check errors block
success. Environment setup and release operations remain outside gate selection.
See [repository quality gates](docs/self-governance.md) for stage coverage.

Git hooks are declared in version-2 configuration and reconciled only after explicit confirmation. A bypassable
local hook verifies working files, not necessarily the pushed commit. Hosted
Codex authentication is not provisioned and personal tokens are not copied to CI.
No hosted service, database, autonomous development, universal policy engine,
result cache or additional input/provider implementation is in current scope.

## REC-001: Declarative reconciliation and Git hooks

Configuration validation, desired-state reconciliation and quality-check execution
are separate responsibilities. Both `pass auf` and execution through `mach was`
use Validate -> Inspect -> Plan -> Confirm -> Apply -> Verify before any checks.
No discrepancy means no prompt or write. Rejection, unavailable terminal, conflict
or failure exits 2 and preserves the previous report. `--plan` remains read-only
and does not reconcile. No provider is called by reconciliation.

Version 2 optionally maps `hooks.git` keys `pre-commit`, `commit-msg`, `pre-push`
to existing valid profile IDs (including built-in `all`). Version 1 rejects the
hooks field even when empty. Unknown events, duplicate mappings, invalid values
and missing profiles fail during full configuration validation.

Git resolves the effective hook path, including worktrees and `core.hooksPath`.
Default common-repository hooks are shared by its worktrees; custom paths must
remain within this working tree. Symlink components, nonregular files, malformed
or duplicate ownership markers and foreign repository ownership are conflicts.
No Git configuration is changed. Only the three supported event files are inspected.

Managed sections have explicit markers and repository ownership. New hooks use
POSIX sh. Existing executable POSIX sh hooks can be composed by inserting a
managed section immediately after the shebang, preserving the remaining bytes
and checking Hannes failure before existing commands. Other layouts require manual
integration. Removal preserves unmanaged bytes and permissions; whole-file deletion
requires exact ownership of the wrapper. Plans are deterministic and revalidated
before writes. Files are staged, synced and replaced individually, with best-effort
rollback on failure; a multi-file operation is not an atomic transaction. An
interrupted process may leave drift; rerun `pass auf` and inspect the reported paths.

Generated wrappers pass explicit event, wrapper path and original arguments to
`mach was`. Hannes validates this context against the repository, mapping and hook
state, verifies reconciliation read-only and never prompts or edits hooks. Drift
instructs the user to run `hannes pass auf` manually. Nested execution from checks
is rejected using a repository-scoped guard. Checks receive `HANNES_GIT_HOOK`,
`HANNES_GIT_HOOK_ARG_COUNT` and one-based `HANNES_GIT_HOOK_ARG_1`, etc.; a dedicated
checker script reads these without shell expansion by Hannes. `commit-msg` supplies
the message file in argument 1, unlike pre-commit; no commit policy is hardcoded.
Pre-push stdin is not forwarded to checks. All profiles still verify working files,
not the exact staged index or pushed commit.

## ARC-001: Shared configuration contract

Defaults, resource bounds and the configuration directory have one source in
Rust. Initialization serializes the actual configuration types rather than
maintaining independent TOML templates. README defaults, limits and scaffold
examples are checked against those values. Validation, planning and execution
reuse the same bounds; this does not add configurable fields or change defaults.

## COMPAT-001: Versioned compatibility evidence

Keep immutable configuration/report fixtures from historical source versions.
Current code must read supported reports and compatible configuration fields
without silently changing their meaning. The command-string change intentionally
rejects historical array commands in versions 1 and 2; keep frozen arrays as
rejection evidence, while historical reports remain readable. Tests may substitute
repository installation identity, not synthesize
old field sets by serializing current structs. Preserve old fixtures when adding
a new version; deliberate breaking changes require an explicit specification.

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
| REC-001 | `tests/reconciliation.rs::approval_updates_removals_and_idempotency` |
| ARC-001 | `tests/documentation.rs::documented_bounds_and_provider_scaffold_match_the_implementation` |
| COMPAT-001 | `tests/compatibility.rs::historical_array_commands_are_rejected_but_reports_remain_readable` |
| DOC-001 | `tests/documentation.rs::every_configuration_field_required_marker_and_default_is_documented` |
| CLI-001 | `tests/acceptance.rs::all_command_paths_and_standard_help` |
| CFG-001 | `tests/acceptance.rs::validation_rejects_invalid_including_disabled` |
| SEL-001 | `tests/profiles.rs::checks_only_explains_omissions_and_cannot_shrink_a_gate` |
| CHK-001 | `tests/check_commands.rs::pass_auf_rejects_invalid_commands_even_when_disabled_without_execution` |
| REV-001 | `tests/acceptance.rs::lean_review_transport_disables_unlisted_mcp_and_honors_model_profile_and_allowlist` |
| PREP-001 | `tests/profiles.rs::prepared_input_is_shared_with_the_provider_without_rereading_context` |
| OUT-001 | `tests/review_contract.rs::review_contract_enforces_boundaries_and_semantic_evidence_locally` |
| INP-001 | `tests/acceptance.rs::changing_files_during_checks_blocks_pass` |
| RPT-001 | `tests/profiles.rs::saved_reports_reject_missing_gate_coverage_and_plan_metadata` |
| REL-001 | `scripts/test-release.py::test_ci_policy_checks_new_messages_without_rejecting_legacy_history` |
