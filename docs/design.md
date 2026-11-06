# v0.1 implementation decisions

## Modules

`cli` parses nested words; `config` loads and strictly validates TOML and initializes
examples; `checks` executes deterministic commands; `process` handles deadlines
and capture; `provider` invokes Codex; `reviews` validates final JSON; `git_snapshot`
fingerprints working files; `report` serializes/persists; `terminal` renders both
run and status. `discovery` loads version-3 documents with source paths; `rules` validates domain
references and resolves effective execution points; `planner` selects one point and prepares bounded offline context
and fingerprints the plan; preview and `lib::verify` share that selection. Plan
metadata is canonicalized separately from raw snapshots to keep semantic IDs
stable across TOML formatting and selector order. Report schema 3 saves execution-point plans; historical report schemas stay readable.
Legacy profile/gate fields in plan deserialization exist only to read historical
reports and never participate in configuration or rule selection.

Configuration defaults and bounds are shared through `limits` and typed scaffold
serialization in `config`. Documentation tests check public fields, defaults,
bounds and examples against those types and constants.

Compatibility is checked with frozen configuration/report fixtures captured from
pinned historical source. Tests substitute only the repository identity and do
not generate old schemas from current structs; fixture provenance is recorded in
[tests/fixtures/compat](../tests/fixtures/compat/README.md).

## Desired-state reconciliation

Configuration validation describes valid desired state; `reconciliation` compares
it with external state and builds an ordered plan. The CLI owns confirmation;
both `pass auf` and executing `mach was` share inspection/application/verification.
The CLI exposes explicit unattended approval through `pass auf --auto-approve`;
it uses the same inspection, application and verification path as interactive
confirmation, including recursion protection and conflict detection.
The rule engine starts only after that prerequisite. Offline execution previews
remain read-only and do not synchronize. No generic domain registry is needed:
Git hooks are the sole current reconciliation domain.

The plan records before/after hook bytes and modes. Application rechecks the
plan, stages replacements and uses per-file atomic renames with best-effort rollback.
It does not promise multi-file atomicity or protection from hostile concurrent
filesystem replacement. Ownership markers bind managed sections to the common
Git directory, allowing default hooks to serve linked worktrees of one repository.
Safe composition inserts a section before existing POSIX sh commands; no shell
parser or arbitrary rewriting is used. Repository-local custom paths are supported;
external paths, links and ambiguous ownership fail before mutation.

Explicit hook context is checked against the configured event/execution point and effective
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

`check_command` validates lexical shell constructs before `shell-words` removes
quotes and splits arguments. Configuration validation and execution use the same
parser; the original command string is serialized in configuration. Arrays are
intentionally unsupported in both configuration versions. Quoted/escaped literals
remain arguments; shell evaluation is never implicit. Executable availability is
checked only by process creation, while configuration still validates containment
of relative paths (including existing ancestors of missing executables).

Commands run sequentially without an implicit shell. Both pipes are drained
concurrently to EOF, retaining only the first 64 KiB each and their complete byte
totals. Truncation is recorded. Timeouts are ERROR; signal termination/nonzero exit
is FAIL; spawn or capture failure is ERROR. POSIX processes run in separate groups
and descendants are killed on exit/timeout. Windows uses `taskkill /T /F` for timeout
cleanup; detached descendants may escape termination. Pipes remaining open beyond
two seconds produce ERROR. This is not a hostile-process sandbox.

Reports contain UTC timestamps, repository/project, HEAD, manifest/fingerprint,
per-item status/duration/evidence/output and overall result. ERROR takes precedence,
then FAIL; NOT_RUN is blocking but does not replace an existing FAIL with ERROR.
Provider review durations reflect the shared invocation, not individual model timings;
do not sum them. Available token usage is recorded once for the shared call;
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

One fresh Codex call receives configured criteria, deduplicated requirement excerpts
with original line numbers, and bounded orientation data. Exact per-review entry
files focus the initial manifest/diff when every active review is focused; otherwise
project-wide orientation remains. Markdown sections are selected by unique ATX
heading, excluding fenced/indented code, with overlaps merged and a strict 64 KiB
selected-text budget. No context is silently dropped. These are entry points,
not an evidence boundary; dependencies can be inspected using targeted read-only
queries. Full working-tree snapshots and file/line evidence validation remain.

Before the single model call, a bounded token-free `codex mcp list --json` reads
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
for substantive PASS/FAIL. Any invalid combined document makes every active review
ERROR, even if another verdict looked valid. ERROR can express model uncertainty.
Grounded references are necessary, not proof of accurate conclusions.

Hannes inherits trusted local Codex login and applies the documented review overrides; it supplies `read-only`
and `never` approval settings explicitly. It does not load credentials itself,
copy tokens to CI, record raw provider traces or repair files. Read-only sandbox
support depends on Codex and the platform. Unsupported CLI versions fail closed.

## Phase boundaries

Phase 1 uses local TOML documents only. Discovery supplies assignments and source
paths to a pure resolver; it never resolves remote repositories. Rule sets contain
rule IDs only, with no nesting or back-references on rules. Phase 2 can supply
external definitions after enforcing same-source references, exactly one set per
repository, and immutable consumer assignments. Source identity and Git reference
handling belong to that loader; no speculative provider framework is introduced.

Phase 3 coordinates migration of pinned self-governance configuration, versioned
hooks, GitHub Actions and historical roadmap/release documentation. These continue
using the published governor until a compatible release is available. No hooks,
remote services, releases or governed project files are automatically installed or
repaired by phase 1.
