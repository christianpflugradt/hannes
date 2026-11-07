# Hannes specification

Status: current implemented contract. Changes to behavior start here and in the
acceptance criteria, before implementation. Explicit user decisions take precedence.

The [product vision](docs/product-vision.md) defines direction and architectural
boundaries: Hannes makes development expectations explicit, executable and
verifiable, independently of who writes the code. This document defines supported
behavior. The [roadmap](docs/todo/README.md) contains planned extensions; they are
not implemented merely because they have a proposal.

The [README configuration reference](README.md#configuration) defines every public
configuration field, default and limit. The [execution-point contract](docs/execution-points-and-plans.md)
extends this specification for plan schema 3 and report schema 4. These documents must agree; a contradiction must be resolved before work
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
| `hannes mach was` | Validate, select rules, execute checks before reviews and save results; reconcile hooks only for Git-backed points. |
| `hannes was ist phase` | Read saved results and compare input freshness, without rerunning rules. `--details` shows full findings and captured output. |
| `hannes so hilf mir doch` | Show help listing exactly the five complete command phrases. |

`-h`/`--help`, `-V`/`--version` and `--no-color` remain supported. Help must not
present intermediate words as useful standalone commands, add a separate `help`
command or redundant examples. Other execution phrases are rejected.
Run from the Git working-tree root; subdirectory execution is rejected. Hannes
must not create commits, modify the Git index or silently discover another root.

## RULE-001: Rules, Rule Sets and Execution Points

Rules are implemented as Checks or Reviews. Each declares id, optional name, description,
enabled (default true), blocking (default true), optional run_at and exactly one
implementation table. Rule IDs are unique within a governance scope. Rule Sets
require id, description, rules and nonempty run_at; no enabled/blocking/source
fields are accepted. Set IDs are independently unique.

A Rule's optional name is a short display label: 1..=80 characters, without
control characters or leading/trailing whitespace. Its default is the original
scope-local ID, including for imported rules. Names do not change identity,
selection, ordering, parameters or provider IDs. Duplicate display names receive
a short origin suffix and, where needed, an occurrence number. Plans save names
so progress, results and saved status agree; technical IDs and full provenance
remain available in JSON and details. Reports without optional names remain readable.

A present set overrides its rules' own run_at; memberships contribute their union.
Standalone enabled rules use their own assignments. Repeated references within
a Rule Set contribute one membership. Enabled rules need an effective
point. Disabled rules remain validated but never run. All references must resolve
within the same scope, including disabled and overridden assignments. A rule runs
once per point within each binding; checks precede reviews, each kind sorted by ID.
Built-in points
are pre-commit, commit-msg, pre-push and check. The first three use managed Git
hooks; check is invoked explicitly and never automatically. Custom names come only
from the consumer; release has no special behavior.
The default is pre-push. Empty points succeed without claiming rule PASS.

## EXT-001: External Git Rule Sets

Only the consumer's config.toml accepts [[imports]], with git/path/ref
and optional per-import execution_points mapping and rule_parameters tables.
Each import is a separate binding, even when it selects the same source. Values
never merge across imports. Rule identities distinguish these bindings while
repeated references within a binding still execute once.
No Rule overrides or nested imports are supported. Mapping sources are custom
points used in that import; targets must be consumer-declared custom points.
Built-ins cannot be sources or targets. Resolve scope-local assignments first,
then map effective points and deduplicate before merging. Unmapped names must
still be known to the consumer. Each import has an independent mapping namespace;
many sources may share a target. Mapping never changes cached definitions,
revision identity or provenance and works offline without fetching.
Each selected directory defines exactly one Rule Set in config.toml. Its rules
are paths relative to the external repository root, such as
rules/checks/secret-scan or rules/reviews/architecture. Each path names a Rule
directory directly under checks/ or reviews/; it contains rule.toml and its
resources. Multiple sets may reference the same directory. Only referenced Rules
are loaded, repeated paths load once, and distinct directories with duplicate
Rule IDs fail within a binding. References are resolved to IDs before ordinary
assignment validation. Each import remains a separate execution/parameter binding.
No nested imports or references to other sets are supported. Absolute paths,
parent traversal, symlinks and references outside the repository fail. Local sets
continue to reference IDs. This replaces external sets' former directory-local
ID references; migrate each entry to its repository-relative Rule directory.
Rules are namespaced only after scope-local validation; provenance retains source
identity, selected path/ref and resolved commit.

pass auf resolves exact branches, tags and full hashes using host Git, materializes
shared immutable revisions and validates everything before atomic publication of
project resolution state. Failure preserves prior state. Execution, hooks and
plans use the validated cache offline, never substitute revisions, and fail when
state or contents are missing/corrupt. Git-directory state, shared caches, bounded
OS locks, integrity receipts and retrieval trust boundaries remain in effect.
Checks run in the consumer root; HANNES_RULE_DIRECTORY locates bundled resources.
Imported code has Hannes' permissions and must be trusted; path confinement is
not a sandbox. Credential-bearing HTTP URLs are rejected and diagnostics never
echo Git stderr or secret repository selectors.

## CFG-001: Configuration and initialization

One unversioned configuration model uses lowercase .hannes/config.toml, optional
provider.toml, providers/*.toml, checks/*/rule.toml, reviews/*/rule.toml and rule-sets/*.toml.
There is no configuration version field. Unknown fields, duplicate keys/IDs,
invalid references and unsafe rule resources fail, including disabled Rules.
Missing check entrypoints or review prompts are execution errors only when
selected and enabled.
No recursive TOML discovery is supported. External sets explicitly select Rule
directories as defined in EXT-001; local definitions use only the paths above.
Supporting files inside rule directories are data, never governance documents.

Project settings are project, max_display_files (default 5), execution_points
and imports. The README defines all fields and bounds. Review prompt filenames
must be single nonempty names directly in a Rule directory: no separators,
absolute paths, parent traversal, symlinks or special files. Review prompts are
UTF-8 Markdown without frontmatter requirements. Check entrypoints follow CHK-001.
Checks may bundle helpers.

fang an creates missing files without replacing edits or installing hooks.
It scaffolds a passing enabled Check, a disabled failing Check and disabled
product-consistency Review in self-contained directories. Reports remain local.
Provider omission uses scaffold defaults: codex-cli, 180 seconds, gpt-6.1-sol,
medium effort, empty MCP allow list and 2000 retained tool-output tokens.

## PROV-001: Named provider configurations

Local `.hannes/providers/<id>.toml` files define complete provider configurations
using the same fields, defaults and limits as `provider.toml`. IDs use the Rule ID
syntax and are global across local definitions and explicitly imported sets.
Each external selected directory may contain `providers/<id>.toml`; these files
are loaded through the existing immutable cache and source-boundary checks.
Repeated imports of the same immutable provider file share one definition;
distinct definitions with the same ID fail, even if their settings match. Local
overrides, per-review model overrides and credentials are unsupported.

`review.provider` optionally references one named configuration. Resolve references
only after all explicit imports are loaded, independently of import order, before
publishing resolution state. Unknown references fail even for disabled reviews.
Unreferenced reviews continue using the consumer's `provider.toml`, or its scaffold
defaults when omitted. Named configurations never become an implicit default.
`codex-cli` and generic `cli` providers are supported. Codex batches require the same provider reference and
resolved parameter environment; each invocation uses its resolved model, timeout,
review profile and MCP allow list. Plans and saved reports retain the reference
and effective named configuration per review; existing schema 3/4 documents
without these optional fields remain readable. Provider settings contain no
credential fields; authentication remains in the existing Codex login.

Acceptance: `tests/named_providers.rs` covers local/imported resolution, conflicts,
legacy defaults, batching, invocation settings and saved plan/report metadata.

## PROV-002: Generic CLI execution and runtime requirements

`provider = "cli"` uses a required nonblank `command` and optional literal `args`
(default empty). No shell, interpolation or argument expansion occurs. The consumer
root is the working directory. Optional `model` is delivered as `HANNES_MODEL`;
wrappers may translate it into tool-specific flags. The existing required
`timeout_seconds` (1..=3600) applies to the entire invocation, including stdin delivery.
Codex-only MCP/profile settings must remain at their omitted defaults for CLI.

Each generic CLI review executes individually, preserving provider references and
parameter isolation. Stdin receives the complete shared review prompt plus the JSON
response schema; stdout must be exactly the existing structured verdict document.
No envelopes, Markdown fences or natural-language extraction are supported. Output
is bounded by the shared 64 KiB capture limit. Malformed, incomplete, oversized or
ungrounded verdicts, missing commands, nonzero exits and timeouts produce ERROR.
Raw stdout/stderr and invalid-response details are never persisted as diagnostics.
The shared process runner drains both pipes and cleans up process trees.

Both provider kinds accept optional `[requirements]` with `env` and `executables`
arrays (default empty). Names are validated during configuration loading, but
availability is checked only for selected, enabled reviews after deterministic
gating and parameter resolution, immediately before their invocation. Environment
requirements use the child environment and must be nonempty. Executables resolve
through that environment's PATH or explicit paths relative to the consumer root.
No credential values are configured, discovered or logged. Generic tools inherit
the execution environment; Codex retains its login-only credential policy.

Imported CLI definitions follow EXT-001's existing explicit import trust boundary:
refresh and validate before publication, immutable offline cache, no local overrides.
Imported commands have the consumer's permissions, just like imported checks.
Generic CLIs provide no sandbox or Codex MCP controls; consumers must trust their
tools and configure read-only behavior themselves. No HTTP transport, retries or new orchestration is introduced.

Acceptance: `tests/cli_providers.rs` covers configuration, stdin/JSON execution,
runtime gating, isolation, process errors and timeout cleanup; PROV-001 retains
Codex compatibility coverage.

## SEL-001: Selection and offline plans

`--at NAME` selects a known execution point, defaulting to pre-push. Selection
uses only centrally resolved effective assignments. Only worktree input is
supported. Rules run once, checks first and each kind ordered by ID.

`--plan` uses the same selection/preparation as execution. It prints order,
selection reasons, omissions, required rules, input identity, declared provider
settings and bounded context sizes, without running checks, Codex, MCPs or network
calls, changing project/Git state or replacing the report. `--plan --json` uses
plan schema 3 and structured diagnostics. Every JSON preview includes
plan_schema_version, status and blocking_reasons. Configuration or selection
errors return status blocked with nonempty blocking_reasons and exit 2; unavailable
plan metadata is omitted. Prepared plans retain their complete metadata, including
when blocked by an incomplete snapshot. Acceptance:
`tests/acceptance.rs::json_plan_errors_use_current_schema_and_blocking_reasons`.
A plan is neither a cache nor proof
that a later input is identical. Provider availability is not checked offline.

`--geiz-ist-geil` runs checks only, without provider discovery, model calls or MCP
connections. Reviews are excluded from progress/results and the limited scope is
recorded in plans and reports; terminal output warns only about actual omissions.
Full configuration validation still runs.
Every selected rule must execute; only completed non-blocking FAIL is nonfatal. A point requiring reviews rejects checks-only
execution before any check starts. Empty execution points succeed without proving
any rule PASS.

Checks and AI provider subprocesses receive `HANNES_EXECUTION_POINT` from the
validated execution plan, including custom points and mapped imported assignments.
Hannes overrides any inherited value. This is runtime context, not a configurable
rule parameter or TOML field; parameters, rule resources and Git hook arguments
retain their existing contracts.

## CHK-001: Deterministic checks

[check] requires exactly one of entrypoint or command, including for disabled
Rules. command is a nonempty array of strings: executable first, then literal
arguments. The executable must be nonblank and no element may contain NUL. Empty
arguments are allowed. Programs resolve through PATH or an explicit executable
path, with the consumer root as working directory. Hannes performs no shell
parsing, interpolation, globbing or runtime installation. interpreter is accepted
only with entrypoint. Command availability is checked only during execution;
missing executables produce per-rule ERROR and offline plans never launch them.
Parameters, rule-directory context, timeouts, blocking and results use the same
execution path for commands and files. Review resolution is unchanged.

Acceptance: `tests/configuration.rs::commands_validate_without_execution`
and `tests/configuration.rs::commands_preserve_literal_arguments_and_rule_context`
cover command validation, literal arguments, parameters and per-rule results.

For file entrypoints, a filename such as index.sh resolves in the declaring
Rule directory. A relative path containing `/`, such as scripts/check.py, resolves
from the rule-source root: the consumer repository root for local Rules and the
imported repository root for Git Rules, regardless of the selected Rule Set path.
Resolution depends only on syntax; there is no fallback search. Paths use `/`;
empty paths, absolute paths, backslashes, drive prefixes, parent traversal,
symlinks in any component and nonregular files are rejected, including for disabled
Rules. Missing entrypoints remain per-rule execution ERRORs only when selected and
enabled. Source containment and regular-file checks run again before execution.
Review prompt resolution is unchanged.

Multiple Rules may use one entrypoint. Each retains its identity, parameters,
timeout, blocking behavior, execution and result; sharing never deduplicates
executions. HANNES_RULE_DIRECTORY always identifies the declaring Rule directory,
including when the entrypoint lives elsewhere in the source. Cached Git imports
support the same resolution offline.

Acceptance: `tests/configuration.rs::local_rules_share_entrypoints_with_independent_results`
and `tests/external_rule_sets.rs::shared_entrypoints_execute_each_rule_offline`
cover shared scripts, independent parameters/results and declaring directories;
path confinement, execution-time replacement and missing/disabled cases are covered
in those test files alongside filename-only compatibility.

[check] accepts optional interpreter and timeout_seconds
(default 30, 1..=3600). Direct execution uses native OS behavior with no shebang
parsing or language detection. Explicit interpreters receive the absolute
entrypoint as one argument; no implicit shell, runtime installation or interpreter
argument parsing occurs. The consuming root stays the working directory and
HANNES_RULE_DIRECTORY identifies bundled resources. Unix scripts may need an
installed interpreter on Windows; arbitrary scripts are not universally portable.

Exit 0 is PASS, 1 is FAIL and 2 explicitly reports an execution ERROR.
Unexpected numeric exit codes remain FAIL with an explicit diagnostic; signal
termination is ERROR. Wrappers map scanner infrastructure errors to 2. Spawn,
capture and timeout failures are always ERROR. Non-blocking FAIL remains visible and allows success
and subsequent Reviews. ERROR is fatal regardless of blocking. Stdout/stderr
capture, process deadlines and cancellation remain shared in process.rs.

## PARAM-001: Rule parameters

Rules declare named `parameters` tables with `type` (`string` or `file`),
`required` (default false) and optional `description`. Local declarations provide
`value` directly. Imported declarations must omit `value`; consumers supply values
in that import's `rule_parameters.<rule-id>` table. There is no global value table.
Names use ASCII letters, digits and underscores, starting with a letter; their
uppercase forms must be unique within a rule. Values must be strings without NUL.

Before an enabled selected rule executes, validate required values, supported
types and undeclared supplied names. Optional omissions stay absent. Failures
produce rule-specific ERROR results identifying the parameter and reason, even
for nonblocking rules. Disabled rules do not resolve parameters or dependencies.
File values must be repository-root-relative paths to existing regular files;
absolute paths, parent traversal and symlink traversal are rejected. Deliver
canonical absolute paths for files and unchanged strings through environment
variables named `HANNES_PARAM_` plus the uppercase parameter name. Clear inherited
variables with this prefix so omitted/undeclared values cannot leak into rules.
Checks and provider subprocesses use the same delivery mechanism. Different review
environments use separate calls; prepared context remains shared. Review prompts
and filename-only Check entrypoints continue resolving within their own
directories; shared Check paths follow CHK-001.

Acceptance: `tests/parameters.rs` and parameterized imports in
`tests/external_rule_sets.rs` exercise loading, binding and execution.

## REV-001: Reviews and provider isolation

The built-in Codex CLI and generic CLI providers execute reviews. Codex resolves
through inherited PATH and uses the existing local login. API-key environment
variables are removed; no credentials or raw provider traces are persisted or
copied into CI. Missing/incompatible CLI, login, timeout or exhausted allowance
produces ERROR, never PASS or a fabricated policy violation.

Selected reviews with the same provider reference and identical resolved parameter
environments share one new,
noninteractive, ephemeral Codex invocation with structured output, read-only sandbox and no approvals. A bounded, token-free
`codex mcp list --json` precedes it. Every discovered server is explicitly disabled
unless in the provider allow list; unknown allowed names, unsafe names, oversized
or failed discovery block the call. Automatic skills, apps/plugins, subagents and
web search are disabled for the child process, including when multi-agent support
is enabled in the user's configuration. User Codex configuration is not
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

## TRIG-001: File-change review selection

Optional `[review.trigger]` has required `changed_files` (1–64 patterns, each
1–1024 UTF-8 bytes). Paths are consumer-repository-relative with `/` separators:
exact components, `*` and `?` within components, and whole-component `**` for zero
or more directories. Matching is case-sensitive; hidden files match normally.
Empty/dot/parent components, absolute paths, backslashes, colons, control
characters, brackets, braces and embedded `**` are rejected, including imports.
Reviews without triggers retain ordinary selection.

For enabled reviews at the requested point, `HANNES_CHANGE_BASE` explicitly names
a locally available Git commit/ref. Resolve it once to a commit ID; compare that
commit directly to current working files, including staged/unstaged changes and
nonignored untracked files (plus ignored `.hannes/` files except reports).
Use net differences, not per-commit history; renames match both old and new paths.
CI supplies a fetched baseline commit, such as the PR merge base computed by its
caller. Never infer HEAD^, a remote branch or GitHub context. Shallow repositories
work only when the named baseline commit and tree are locally available. No fetch.
Index conflicts and assume-unchanged/skip-worktree flags block comparison.
Missing/invalid refs, unavailable trees, invalid filename encoding, incomplete
snapshots, timeouts or truncated Git output block planning/execution, never skip.
Git uses direct argument arrays, NUL filename output and existing 64 KiB capture
and 30-second deadlines. Snapshot/context limits remain unchanged.

Apply triggers before context preparation, parameter resolution and model-call
estimation. Nonmatches have omission reason `trigger_not_matched` in preview and
saved report plans, no result and no required ID; they do not require provider
credentials/executables. Disabled and inapplicable rules need no comparison.
Record the resolved baseline in the plan and its identity. Deterministic checks
still gate all scheduled reviews globally. Acceptance: `tests/review_triggers.rs`.

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

Available token counters are summed across parameter batches, retaining unknown
counters as unavailable, including when the verdict is rejected. Total is input plus output; cached input, reasoning output
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
over blocking FAIL; NOT_RUN blocks success without replacing an existing blocking FAIL with ERROR.
Disabled Rules stay out of results; blocking failed Checks leave selected Reviews NOT_RUN.

Report schema 4 records plan schema 3, required IDs, blocking and rule provenance.
Only the current report and plan schemas are accepted. Reject corrupt/inconsistent
reports and mismatched repositories. Reports are local, unsigned evidence, not
tamper-proof attestations. Status uses saved selection, not the current default.

Flush progress to stderr before each rule starts and after it finishes. Display
a short name, description, status and elapsed time. `--progress=auto|plain|tty`
controls verification progress (default auto). Auto redraws in an interactive
stderr terminal, except CI=true/1 or TERM=dumb; otherwise it appends plain lines.
Plain never emits cursor controls. Tty redraws rule rows, replacing their running
state on completion; descriptions must not wrap the row. Color is independent.
Announce shared provider calls without claiming individual model completion or
summing shared durations. Hide internal phases and successful process-exit reasons.
Failures show actionable reasons and findings; raw captured output stays in details.
Execution ends with one overall status/count/duration line, without a duplicate
table, routine report path, timestamps or checks-only boilerplate. Integrity
failures remain visible even when no rule failed. Empty points explicitly say no
rules ran and do not claim rule PASS. Saved status lists results with display
names; details retain IDs, provenance, output and usage. No rules defined prints
`Du hast noch keine Regeln definiert, du Vogel!`; disabled definitions do not
trigger that message. Acceptance: tests/progress.rs covers names, plain/TTY
rendering, concise execution and actionable errors.

Respect `NO_COLOR`, `--no-color` and each stream's terminal state. Status colors
are green PASS/CURRENT, red FAIL and amber ERROR/STALE/UNKNOWN; help uses cyan
accents and bold headings. Sanitize untrusted terminal content.

Exit codes: 0 success; 1 failed verification or STALE; 2 configuration, execution,
provider/report error, UNKNOWN or invalid arguments; 3 absent report for status.
An executable plan exits 0; a blocked/invalid plan exits 2.

## REL-001: Distribution and integrations

The root `action.yml` is a public composite GitHub Action for GitHub-hosted
Ubuntu 24.04 x64 and ARM64 runners. Required `version` is an exact stable release
tag such as `v0.9.0`. Optional `execution-point` defaults to `check`; `run` defaults
to `true` and accepts only `true` or `false`. Installation downloads the matching
native archive and SHA256SUMS, verifies the checksum, and adds the executable to
PATH for subsequent steps. With `run: true` it executes
`hannes mach was --at <execution-point>` in the consuming checkout and preserves
its exit status. Installation errors fail with diagnostics. It installs no rule
dependencies, initializes no rules or hooks, and has no installed-version
detection, comparison, upgrade logic or persistent cache.
Acceptance: `scripts/test_action.py` and the public Action consumer job in CI.

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

Repository self-integration continues to use its pinned published governor.
The governor is an installed, previously published binary outside the checkout;
local candidate builds never govern the source being checked. `.hannes-version`
is the sole exact release pin, passed explicitly to the public Action on Ubuntu
and the native installer on macOS. Renovate updates it through GitHub-release
PRs without automerge. CI never resolves latest.
Pinned installation downloads the exact release's native archive and SHA256SUMS
directly, without a GitHub Releases API lookup or fallback to latest. Only latest
discovery uses the API, with optional GITHUB_TOKEN Bearer authentication; asset
requests carry no token. Missing releases/assets, network/API failures and missing,
malformed or mismatched checksums fail installation and preserve the installed copy.
Acceptance: `scripts/test_deploy.py` covers direct pinned URLs, optional API
authentication, safe diagnostics and installation failures with mocked downloads.
The governor uses the exact published release in .hannes-version. Repository rules use the unversioned,
self-contained Rule model and fixed Git-hook execution points.

The three Git-backed built-in points serve Git hooks. Normal CI uses one check
point for formatting, documentation, tooling, native validation, minimum Rust,
dependency audits and security. Local Rules declare their execution points
directly, without local Rule Sets. Pre-commit runs formatting and documentation
validation; pre-push runs the complete local suite, keeping all Rust tests in
one Rule. Python helper tests use one unittest discovery Rule and JavaScript
release tests use one npm Rule. All three test suites, workflow validation and
minimum-Rust checks run at pre-push and again in CI. The repository has 16 local
Checks and one conditional document Review; dependency audits remain at check with advisory access. Agents use Git
commit/push feedback instead of manually running checks,
tests, linters or builds. They may read saved reports without rerunning Rules.
The manual mise check task delegates to published Hannes at pre-push.
External Rule Set imports, pins, parameters and assignments remain unchanged.
The custom release-build point selects the five native
checks on each release host; release-artifacts checks the collected archives.
The self-governance fixture records local rule definitions and selections.
The commit-msg point additionally imports the conventional-commits set at a full
Git commit pin. Its scopes_file parameter uses .hannes/commit-scopes.txt, which
must match the scopes in .githooks/commit-policy. The existing local message check
retains Hannes' narrower type allowlist and stricter summary whitespace policy.
The check and pre-push points also import multi-point/security at a full commit
pin. Its sensitive-files and secret-scan checks inspect the working tree at check
and HEAD contents at pre-push, without history. The CI quality job explicitly
runs check with Python 3 and a checksum-verified Gitleaks binary available. The
release publisher prepares Gitleaks for its managed push hook. Rules and Hannes
never install these dependencies automatically. The same points import
repository-conventions from main without a commit or tag pin. It currently
contains only the blocking license Rule, with target = "LICENSE". The license
must match an SPDX reference byte-for-byte: Apache-2.0,
PolyForm-Noncommercial-1.0.0 or Unlicense. Hannes uses the exact Unlicense text.
The Rule reads the working tree at check and HEAD at pre-push. Missing or
nonmatching licenses produce FAIL; technical errors produce ERROR.
Acceptance: scripts/test_governance.py covers the import and effective selection.
Checks remain mandatory. The agents-quality Review runs at check only when its
file trigger matches, as defined in GOV-001. Missing tools and errors
block success. Setup, dependency installation and release publication remain
outside rule execution.

Hooks are managed by pass auf at Git's effective hook path, including forwarding
commit-message arguments and preserving compatible unmanaged content. A setup
shim only retires this checkout's obsolete .githooks setting when its old event
files are absent, then delegates to pass auf; custom hook paths are preserved.
Check subprocesses retain the inherited environment and recursion guard. Local
Rules use direct command arrays or Python file entrypoints, without per-rule shell
or Python launchers. Cargo build/test/Clippy/MSRV commands and npm tests explicitly
select offline mode. The execution-point wrapper additionally sets Cargo/npm
offline mode and disables Rustup auto-install for Git-backed invocations.
CI invokes pass auf --auto-approve once before execution. Execution never repairs
hooks noninteractively. The publish job prepares stable Rust with rustfmt and
Clippy, Rust 1.85 and actionlint, and fetches locked Cargo dependencies for both
toolchains before publication, so its managed commit and push hooks can run
offline. Report artifacts are uploaded after
publication with an always condition, including when a release hook fails.
Reports use schema 4 and a companion execution record
stores governor version/digest, execution point, checked commit and exit code.
See [repository self-integration](docs/self-governance.md) for coverage and setup.

Git hooks required by enabled effective assignments are derived automatically
and reconciled only after explicit confirmation (or pass auf --auto-approve). A bypassable
local hook verifies working files, not necessarily the pushed commit. Hosted
Codex authentication is not provisioned and personal tokens are not copied to CI.
No hosted service, database, autonomous development, universal policy engine,
result cache or additional input/provider implementation is in current scope.

## REC-001: Declarative reconciliation and Git hooks

Configuration validation, desired-state reconciliation and quality-check execution
are separate responsibilities. `pass auf` and direct execution of Git-backed
points through `mach was` use Validate -> Inspect -> Plan -> Confirm -> Apply ->
Verify before any checks. Direct execution of `check` and custom points validates
and resolves Rules, then executes without inspecting or reconciling Git hooks.
Missing, outdated or unsafe hook paths do not block these explicit points; they
never prompt for hook synchronization or modify hook files. Repository recursion
protection still applies. Managed hooks accept only their identically named
built-in execution point.
No discrepancy means no prompt or write. Rejection, unavailable terminal, conflict
or failure exits 2 and preserves the previous report. `--plan` remains read-only
and does not reconcile. No provider is called by reconciliation.

`hannes pass auf --auto-approve` explicitly approves the displayed reconciliation
plan without reading stdin or requiring a terminal. It is scoped to pass auf and
defaults to false. Validation, conflict detection, recursion protection, apply-time
state checks, rollback and final verification still apply. It creates, updates or
removes only declared managed integration state, never governed project files,
and executes no checks or reviews. Repeating it on synchronized state performs no
writes. Without the flag, noninteractive drift still exits 2. CI can run this
command once before Git-backed execution; explicit points such as `check` need
no hook setup in CI. Execution itself has no auto-approval.
Acceptance: `tests/reconciliation.rs::auto_approve_reconciles_noninteractively_without_executing_rules`.

Hannes supports exactly three Git hooks: pre-commit, commit-msg and pre-push.
Each invokes its identically named built-in execution point. This relationship is
fixed and cannot be configured. Enabled effective assignments determine required
hooks after Rule Set inheritance and external imports resolve. Removing those
assignments retires the corresponding managed hook content.
Project configuration accepts no hooks section; obsolete hook declarations fail
as unknown fields, with the configuration path and supported fields in the error.
check and custom execution points have no Git hook and are invoked explicitly.
External import mappings still apply only to custom points and cannot map built-ins.
check has no workflow discovery or CI validation; consumers choose where to invoke
it, including multiple workflows. Hannes generates no CI workflows.
Acceptance: tests/reconciliation.rs covers fixed event invocation, automatic
installation, removal, obsolete configuration rejection and explicit-point isolation;
tests/external_rule_sets.rs covers imported hook Rules and per-import mappings.

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
`mach was`. Hannes validates this context against the repository, identically
named execution point and hook state, verifies reconciliation read-only and never prompts or edits hooks. Drift
instructs the user to run `hannes pass auf` manually. Nested execution from checks
is rejected using a repository-scoped guard, including checks started by direct
execution of Git-backed, check and custom points. The guard also blocks nested
reconciliation in that repository. Acceptance:
`tests/reconciliation.rs::direct_execution_blocks_nested_execution_and_reconciliation`.
Checks receive `HANNES_GIT_HOOK`,
`HANNES_GIT_HOOK_ARG_COUNT` and one-based `HANNES_GIT_HOOK_ARG_1`, etc.; a dedicated
checker script reads these without shell expansion by Hannes. `commit-msg` supplies
the message file in argument 1, unlike pre-commit; no commit policy is hardcoded.
Pre-push stdin is not forwarded to checks. All execution points still verify working files,
not the exact staged index or pushed commit.

## ARC-001: Shared configuration contract

Defaults, resource bounds and the configuration directory have one source in
Rust. Initialization serializes the actual configuration types rather than
maintaining independent TOML templates. README defaults, limits and scaffold
examples are checked against those values. Validation, planning and execution
reuse the same bounds; this does not add configurable fields or change defaults.

## DOC-001: Documentation contract

Every editable configuration field, required/optional marker and default must
match the README reference. README configuration examples must validate and plan
offline. Local links/anchors and specification acceptance references must resolve.
Repository Git hooks run link/specification checks at pre-commit and pre-push;
pre-push also runs the checker tests and Rust documentation acceptance tests.
CI runs them through check. The manual `mise docs:check` and `mise check` tasks
remain available to developers; agents rely on Git hooks. Automated checks
complement editorial review; they do not prove prose understandable.

## GOV-001: AGENTS.md quality review

The repository example uses a blocking command Check to limit root AGENTS.md to
1200 words. Words are maximal runs separated by ASCII whitespace (space, tab,
LF, CR, vertical tab and form feed), including Markdown syntax and code. Exactly
1200 passes; excess reports the actual count and limit. The initial document has
1050 words and is not shortened for this gate.

A named generic CLI Gemini provider evaluates the complete document against a
concise rubric, using whole-file context and the existing structured contract.
Only substantive, evidenced governance defects warrant FAIL. The review is
non-blocking; technical ERROR still blocks. The existing global check gate applies.
File triggers select root AGENTS.md or the rubric; other changes omit the review
with trigger_not_matched. CI uses an explicit verified baseline, preferring the
PR merge-base. Summaries distinguish PASS, non-blocking FAIL, ERROR and SKIP.
No live provider is required by automated tests.

Acceptance: `scripts/test_agents_review.py` and `tests/agents_review.rs` cover the
repository example and its local mocked provider.

## Acceptance coverage and change process

Each essential requirement has a stable ID and representative acceptance tests.
The table is a navigation aid, not a claim that one test proves every clause.
Add or update the contract and acceptance cases before implementing behavior;
update the README/examples and relevant design decisions in the same coherent
change. Keep one coherent configuration model and run applicable checks. Follow the user's Git instructions.
Public documentation explains supported behavior; proposals remain in the roadmap.

| Requirement | Representative acceptance test |
| --- | --- |
| GOV-001 | `tests/agents_review.rs::repository_review_uses_complete_context_and_triggered_cli` |
| TRIG-001 | `tests/review_triggers.rs::preview_and_execution_agree_and_nonmatches_need_no_provider_or_context` |
| PROV-002 | `tests/cli_providers.rs::stdin_results_and_provider_isolation` |
| PROV-001 | `tests/named_providers.rs::named_references_load_without_changing_legacy_defaults` |
| EXT-001 | `tests/external_rule_sets.rs::shared_rules_are_reused_across_sets_and_points_offline` |
| RULE-001 | `tests/execution_points.rs::resolution_obeys_membership_override_union_and_disabled_rules` |
| REC-001 | `tests/reconciliation.rs::approval_updates_removals_and_idempotency` |
| ARC-001 | `tests/documentation.rs::documented_provider_scaffold_matches_the_implementation` |
| DOC-001 | `tests/documentation.rs::every_configuration_field_required_marker_and_default_is_documented` |
| CLI-001 | `tests/acceptance.rs::all_command_paths_and_standard_help` |
| PARAM-001 | `tests/parameters.rs::local_parameters_deliver_strings_and_consumer_files_without_changing_rule_resources` |
| CFG-001 | `tests/configuration.rs::strict_configuration_and_implementation_validation` |
| SEL-001 | `tests/configuration.rs::checks_only_requires_complete_selection` |
| CHK-001 | `tests/configuration.rs::direct_and_interpreter_execution_keep_consumer_and_resources` |
| REV-001 | `tests/acceptance.rs::lean_review_transport_disables_unlisted_mcp_and_honors_model_profile_and_allowlist` |
| PREP-001 | `tests/configuration.rs::prepared_input_is_shared_without_rereading_resources` |
| OUT-001 | `tests/review_contract.rs::review_contract_enforces_boundaries_and_semantic_evidence_locally` |
| INP-001 | `tests/acceptance.rs::changing_files_during_checks_blocks_pass` |
| RPT-001 | `tests/configuration.rs::saved_reports_reject_inconsistent_metadata` |
| REL-001 | `scripts/test_release.py::test_ci_policy_checks_new_messages_without_rejecting_legacy_history` |
