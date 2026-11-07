# v0.1 implementation decisions

## Modules

`config` defines the project, Rule and Rule Set schemas and validates loaded
objects. `discovery` reads only the fixed configuration paths and resolves Rule
resources within their own directories. Local and imported roots share Rule
loading; the central document is intentionally different: project settings locally,
a single Rule Set in an imported root. `rules` resolves effective assignments;
`planner` prepares selection and review context; `checks` uses the shared process
runner; `reviews` validates provider verdicts; `report` persists results; `terminal`
renders them. There are no alternative configuration loaders or execution engines.

Loaded Checks hold their validated absolute entrypoints and optional interpreters.
Loaded Reviews hold prompt text and consumer-relative context. There is no parallel
metadata registry. Blocking belongs to each Rule and follows it into plans/results.
Plan schema 3 and report schema 4 describe output formats, independently of config.

## Desired-state reconciliation

Configuration validation describes valid desired state; `reconciliation` compares
it with external state and builds an ordered plan. The CLI owns confirmation;
`pass auf` and direct Git-backed `mach was` points share
inspection/application/verification. Direct `check` and custom points retain the
repository recursion guard, then execute without inspecting hook locations or
synchronizing hooks. Managed hook invocations still validate hook context and
synchronization read-only; the execution point must equal the Git event.
The CLI exposes explicit unattended approval through `pass auf --auto-approve`;
it uses the same inspection, application and verification path as interactive
confirmation, including recursion protection and conflict detection.
For Git-backed points the rule engine starts only after that prerequisite.
Offline execution previews remain read-only and do not synchronize. No generic domain registry is needed:
Git hooks are the sole current reconciliation domain.

The plan records before/after hook bytes and modes. Application rechecks the
plan, stages replacements and uses per-file atomic renames with best-effort rollback.
It does not promise multi-file atomicity or protection from hostile concurrent
filesystem replacement. Ownership markers bind managed sections to the common
Git directory, allowing default hooks to serve linked worktrees of one repository.
Safe composition inserts a section before existing POSIX sh commands; no shell
parser or arbitrary rewriting is used. Repository-local custom paths are supported;
external paths, links and ambiguous ownership fail before mutation.

Explicit hook context is checked against the fixed event/execution point and effective
wrapper path. Hook invocations inspect the same plan read-only; checker subprocesses
receive arguments as indexed environment values plus a repository recursion guard.
No provider or network participates. See [usage and recovery](usage.md#optional-git-hook).

## Snapshot definition

A snapshot is Git HEAD plus a sorted manifest of SHA-256 hashes, byte sizes and
executable flags for current tracked files and nonignored untracked files returned
by `git ls-files --cached --others --exclude-standard -z`. All `.hannes/` files
are explicitly included even if ignored, except `.hannes/reports/`. `.git/` is
excluded. Ignored build products, caches and secrets are outside scope; a tracked
file stays in scope even if an ignore rule matches it. Deleted files are absent
from the manifest. There are no temporary commits, Git index writes or content
backups. Git subprocesses use `GIT_OPTIONAL_LOCKS=0` and do not run diff helpers.

Git index/staging changes alone do not invalidate content: this verifies working
files, not staging state or a future pushed commit. HEAD changes do invalidate it,
even if all working files are identical. File additions, edits, deletes and renames
are found by comparing manifests; a rename is counted as two paths. Changes made
and reverted entirely between snapshots cannot be detected. Snapshots are not
filesystem transactions. Hannes captures before and after execution, and before an active provider call, and blocks
PASS if they differ; concurrent edits could evade this check if reverted.

Limits: 8 MiB per file, 128 MiB total contents, 100,000 paths, 64 KiB Git file-list
capture, and 16 levels of configuration nesting. Unsupported symlinks, submodules,
special files, non-UTF-8 paths, unreadable files or exceeded limits make coverage
incomplete. UNKNOWN and unavailable totals take precedence over known differences.
Incomplete snapshots also block verification with ERROR and prevent reviews.
Git command deadlines are 10..30 seconds. If enumeration itself fails, verification
writes an ERROR report with an explicitly incomplete baseline when possible.

No previous contents or line counts are stored. Therefore additions/deletions of
lines since verification cannot be derived reliably, and are explicitly unavailable
both per file and in aggregate. Paths and exact path totals are available for
complete baselines. Binary files up to the same size budget are hashed normally.

## Execution and reporting

Checks run their entrypoint directly, or pass it to one explicit interpreter.
No command-string grammar, implicit shell or language detection is involved.
The consumer remains the working directory; HANNES_RULE_DIRECTORY supplies the
absolute Rule directory. Exit 0 is PASS, 1 is FAIL and 2 is ERROR; unexpected
numeric exits produce an explicit FAIL diagnostic.
Only completed FAIL can be non-blocking; ERROR and NOT_RUN always prevent success.

Commands run sequentially without an implicit shell. Both pipes are drained
concurrently to EOF, retaining only the first 64 KiB each and their complete byte
totals. Truncation is recorded. Timeouts, signal termination, spawn failures and
capture failures are ERROR. POSIX processes run in separate groups
and descendants are killed on exit/timeout. Windows uses `taskkill /T /F` for timeout
cleanup; detached descendants may escape termination. Pipes remaining open beyond
two seconds produce ERROR. This is not a hostile-process sandbox.

Reports contain UTC timestamps, repository/project, HEAD, manifest/fingerprint,
per-item status/duration/evidence/output and overall result. ERROR takes precedence,
then blocking FAIL; NOT_RUN is blocking but does not replace an existing FAIL with ERROR.
Provider review durations reflect their batch invocation, not individual model timings;
do not sum them. Available token usage is summed across batches;
missing counters stay unavailable and are never estimated.

JSON is synced to a temporary file in the local reports directory and renamed
atomically over `last.json`; on Unix the directory is synced too. Normal file
permissions for tempfile (owner-only on Unix) protect local reports. Reports are
not signed or tamper-proof; repository configuration and local report storage are
trusted. Corrupt or incompatible reports fail explicitly. Configuration errors
leave a prior report untouched; status then sees changed configuration as STALE.

## Provider boundaries

The planner prepares provider-neutral review input once (`review_input`): selected
requirement excerpts, manifest hints and the bounded orientation diff. Execution
passes that exact preparation to the provider. Prompt rendering does no filesystem
reads or Git work; independent snapshot stability checks still guard execution.

Each batch of Reviews sharing a resolved parameter environment receives configured
criteria, deduplicated requirement excerpts with original line numbers, and bounded
orientation data. Exact per-review entry
files focus the initial manifest/diff when every active review is focused; otherwise
project-wide orientation remains. Markdown sections are selected by unique ATX
heading, excluding fenced/indented code, with overlaps merged and a strict 64 KiB
selected-text budget. No context is silently dropped. These are entry points,
not an evidence boundary; dependencies can be inspected using targeted read-only
queries. Full working-tree snapshots and file/line evidence validation remain.

Before each model call, a bounded token-free `codex mcp list --json` reads
configuration. Each discovered server is explicitly enabled only if named in
`provider.toml`'s `mcp_servers` allow list (default empty); a blank server table does
not override merged Codex configuration. Invalid discovery blocks the review.
Inline child-only overrides disable automatic skill context/search, skill-driven
MCP installation, apps/plugins, subagents and web search, and bound retained tool
outputs. Model/effort are configured separately; new scaffolds use `gpt-6.1-sol`
and `medium`. Authentication, AGENTS.md and Codex safety settings are retained.
No user configuration or credentials are copied or rewritten.

`review_contract` owns response wire types, allowed verdicts and the bounded output
schema sent to Codex. Local parsing retains byte, coverage and evidence checks:
JSON Schema character limits cannot enforce UTF-8 byte limits or file existence.
The schema is limited to the provider-supported Structured Outputs subset; an
incompatible model fails rather than silently dropping constraints.

The provider trait returns a raw final document so tests can inject successful,
failed or malformed responses without real credentials. Schema-shaped serde types
reject unknown fields, wrong types, invalid statuses and duplicate object keys.
Semantic checks require exact requested ID coverage, one verdict each, nonempty
reasons, at most 20 findings per review and real in-manifest file/line references
for substantive PASS/FAIL. Any invalid combined document makes every review in that batch
ERROR, even if another verdict looked valid. ERROR can express model uncertainty.
Grounded references are necessary, not proof of accurate conclusions.

Hannes inherits trusted local Codex login and applies the documented review overrides; it supplies `read-only`
and `never` approval settings explicitly. It does not load credentials itself,
copy tokens to CI, record raw provider traces or repair files. Read-only sandbox
support depends on Codex and the platform. Unsupported CLI versions fail closed.

Parameter declarations live in the existing Rule schema. Loaded Rules carry a
binding of those declarations and import-local values. The shared `parameters`
module resolves and validates only active Rules, and configures child environments.
The planner counts distinct valid review environments; execution groups Reviews
by that environment and reuses the prepared context. Inherited parameter variables
are removed before applying values. No global environment mutations occur.
Missing check entrypoints and review prompts are retained for per-rule ERROR
results; disabled Rules need neither implementation nor parameter resolution.

## External source boundaries

`external` supplies isolated documents to the existing discovery/validation flow.
`external::cache` owns host Git, shared object storage, immutable checkouts,
integrity receipts and OS locks. Each selected directory contains exactly one
local set. The ordinary resolver validates references against only that boundary's
rules and source execution-point names. Per-import mappings translate effective
assignments after inheritance; all resulting names must be consumer-known.
Mappings never affect cache keys or immutable source files. Namespacing follows validation,
before effective results merge. Local resolution runs first, preventing consumer
references from modifying imports. No second execution engine is introduced.

`pass auf` refreshes sources and atomically publishes per-project resolution state
only after complete configuration validation. Hook reconciliation still follows
validation; its failure does not invalidate successfully resolved configuration.
Execution/previews read state and validate cache integrity without fetching or
creating missing state. Project locks span validation and publication. Different
projects also lock shared cache writers.

Repositories are keyed by the selector's SHA-256; checkouts by resolved commit.
No URL is persisted in cache remotes or fetch metadata. Git plumbing materializes
commits without repository hooks, global filters or submodule initialization.
Failed validation can leave reusable cache contents but never changes project
resolution state. Corrupt immutable entries fail explicitly. See
[storage, trust and recovery](external-rule-sets.md).

Checks use the consumer root. Review prompts resolve within their Rule directories;
the loader enforces containment. Checks receive HANNES_RULE_DIRECTORY for resources. No argument rewriting or working-directory
switch occurs. Providers, snapshots, selection and reports remain shared.

The repository retains its active governance configuration and published governor
until a separate release and self-migration. Candidate binaries are exercised only
in isolated fixture repositories. No hook installation, remote services or releases
are triggered by loading configuration.
