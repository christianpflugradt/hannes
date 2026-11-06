# v0.1 implementation decisions

## Modules

`cli` parses nested words; `config` loads and strictly validates TOML and initializes
examples; `checks` executes deterministic commands; `process` handles deadlines
and capture; `provider` invokes Codex; `reviews` validates final JSON; `git_snapshot`
fingerprints working files; `report` serializes/persists; `terminal` renders both
run and status. `planner` resolves profiles/tags, prepares bounded offline context
and fingerprints the plan; preview and `lib::verify` share that selection. Plan
metadata is canonicalized separately from raw snapshots to keep semantic IDs
stable across TOML formatting and selector order. Report schema 2 saves the plan;
schema 1 stays readable as unprofiled legacy evidence.

Configuration defaults and bounds are shared through `limits` and typed scaffold
serialization in `config`. Documentation tests check public fields, defaults,
bounds and examples against those types and constants.

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
