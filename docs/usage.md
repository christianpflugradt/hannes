# Usage

## First run

From the root of a Git repository (an unborn repository from `git init` works):

```sh
hannes fang an
hannes pass auf
hannes mach was
hannes was ist phase
hannes was ist phase --details
hannes so hilf mir doch
hannes --help
hannes --version
```

`fang an` creates `.hannes/` (lowercase), preserving all existing files. It never
installs hooks, stages files or commits. The enabled example uses `git --version`
and passes without Codex. The deliberately failing check and example review are
disabled. To demonstrate failure, change `enabled = false` to `enabled = true`
in `.hannes/checks/deliberately-failing.toml` and run `hannes mach was`.

Checks and reviews are collectively called **rules**. If neither is defined,
`mach was` prints `Du hast noch keine Regeln definiert, du Vogel!` alongside the
report. Defined but disabled rules do not trigger this message.

`pass auf` validates all files, even disabled entries, and reconciles declared
external state after confirmation, without running checks or reviews.
`mach was` validates and reconciles first, then executes selected enabled checks in ID order, then
selected active reviews in one call if checks passed. It selects rules assigned to the requested execution point. Disabled items never enter results.
Any failed check prevents spending review tokens; those reviews are `NOT_RUN`.

Use `hannes mach was --geiz-ist-geil` for an execution point containing checks only.
Points requiring reviews reject this option before running any checks.
The full configuration is still validated, but no review or LLM provider is called;
Hannes incurs no review-token usage. Progress counts only checks. Reviews are
excluded from results, and both the JSON (`checks_only: true`) and saved status
clearly identify the limited scope. PASS means the selected checks passed, not
that reviews passed. If no checks are enabled, the report explicitly says so.
The flag applies only to that invocation; an ordinary run still includes reviews.
No review result is cached or reused. Earlier reports default to the full scope.

## Select and preview rules

Use version 3 rules and rule sets to assign named execution points:

```sh
hannes mach was --at pre-push --plan
hannes mach was --at pre-push --plan --json
hannes mach was --at pre-push
```

The default point is pre-push. All selected rules are mandatory, including
reviews. Checks-only rejects a point requiring reviews. Empty points succeed.
See [execution points and plans](profiles-and-plans.md) and the
[configuration reference](configuration.md).

## Run results

Errors and failures block success. A valid run writes `.hannes/reports/last.json`
atomically, including on FAIL/ERROR. Invalid configuration leaves the previous
report untouched. Reports are ignored and remain local.

During `mach was`, progress appears immediately on stderr: each enabled rule has
an `[i/n] LÄUFT` line with its description, followed by its result, duration and
short summary. Disabled rules are excluded from the total. Skipped reviews get
NOT_RUN without a running message. Preparation and report-writing phases are
also shown so snapshot work is not mistaken for a hung check.

All selected reviews share one Codex call. This is explicitly announced;
the review start lines appear together and individual verdicts arrive when that
shared call completes. The numbering is rule order, not a percentage or a claim
that Codex reports per-review completion in real time. Raw check output and
provider traces are not streamed. The final compact report stays on stdout,
so `hannes mach was > report.txt` keeps progress visible in the terminal.
Both streams respect `NO_COLOR`, `--no-color` and their own TTY state.

Every result row includes a short summary. Review summaries come from the actual
Codex response, including PASS results; substantive verdicts are PASS/FAIL, while
provider problems or unavailable evidence remain ERROR. The AI's `reason` is one
nonempty line of at most 120 characters. Full findings and file/line references
stay in the local JSON, not in normal stdout. Long execution errors or legacy
summaries are shortened only for display; their complete content stays saved.

Use `hannes was ist phase --details` to inspect full saved findings and captured
command output separately. It does not run Codex or any check again. Old reports
remain readable; only `mach was` obtains new provider results.

`was ist phase` reads that report without executing checks or Codex. It reports
CURRENT, STALE or UNKNOWN, with up to five paths and exact changed-file counts
when both snapshots are complete. Line deltas are unavailable: Hannes stores
hashes, not prior file content. HEAD-relative diffs are never mislabeled as changes
since verification. See [snapshot design](design.md).

PASS and CURRENT are green, FAIL red, ERROR/STALE/UNKNOWN amber.
Help banners, commands and flags use cyan; headings are bold and secondary
metadata is muted. Piped streams, `NO_COLOR` (even empty) and `--no-color` disable
ANSI styling; stdout and stderr detect their terminal independently. Raw command
output is saved in local JSON and shown with `--details`, bounded to 64 KiB per
stream with byte totals/truncation indicators.
It is not automatically redacted; configure commands to avoid printing secrets.

## Configure checks and reviews

See the [configuration reference](configuration.md). To add a check, create
`.hannes/checks/test.toml`:

```toml
id = "test"
description = "Run project tests"
enabled = true
command = "cargo test --locked"
working_directory = "."
timeout_seconds = 300
```

Commands are strings parsed into one executable and its arguments, without an
implicit shell. Quotes and escaped spaces preserve single arguments. Validation
rejects shell operators/expansions and malformed quoting before execution; missing
executables fail at runtime. Use a task runner or dedicated script to combine
commands. See the [command syntax reference](../README.md#add-a-check).
Treat deterministic checks like scripts you trust to run yourself. They are not
sandboxed. Check outputs or formatters that change relevant files cause an
integrity ERROR: rerun on stable files.

For an optional Codex review:

1. Install a compatible Codex CLI and authenticate using `codex login`.
2. Customize `.hannes/reviews/product-consistency.md` for your project's criteria.
3. Set `enabled = true` in `.hannes/reviews/product-consistency.toml`.
4. Run `hannes pass auf`, then `hannes mach was`.

New provider configuration uses `codex-cli`, a 180-second timeout, `gpt-6.1-sol`,
medium reasoning and no MCP servers. Model and MCP allow list are configurable in
`.hannes/provider.toml`; see [review context and profile](configuration.md). A ChatGPT-backed local login works without a separate
API key. Usage consumes your account allowance; Hannes does not bypass limits.
Inherited `OPENAI_API_KEY`/`CODEX_API_KEY` variables are removed from the child;
your saved Codex authentication and user configuration still apply. Check
`codex login status` if you want to confirm the saved authentication mode.
There is no automatic real-model smoke test and no LLM cache.

The adapter uses documented [`codex exec`](https://learn.chatgpt.com/docs/non-interactive-mode)
flags: `--sandbox read-only`, `--ask-for-approval never`, `--ephemeral`,
`--json`, `--output-schema` and `--output-last-message`. Full final JSON, exact ID coverage,
allowed statuses and evidence references are checked locally. Missing CLI, auth
errors, timeouts, missing/malformed output and ungrounded responses become ERROR.
Provider traces are not stored. The `turn.completed.usage` event supplies the
CLI-reported input, cached input and output token counts, shown once per bundled
review invocation in the compact report. JSON `token_usage` is input plus output;
`provider_usage` retains the separate counters and optional reasoning/cache-write
counts. Cached tokens are already included in input; reasoning tokens are already
included in output. Neither is added to the total again. `--details` shows the
optional counters. These are CLI-reported usage figures, not a billing audit,
currency cost or remaining account allowance.

Missing, invalid or ambiguous usage stays null and displays as unavailable, never
as zero. It does not invalidate an otherwise valid review. Available usage is
retained even if the verdict fails validation or Codex exits unsuccessfully;
failed/interrupted calls may provide no usage despite consuming allowance.
Older reports remain readable, but their missing usage cannot be reconstructed.

Read-only applies to Codex shell execution, not a new security boundary around
all installed integrations. Your Codex configuration and account remain trusted;
review it before use, especially enabled MCP services or custom hooks. The built-in
prompt treats repo files and diffs as untrusted evidence, but prompt injection and
LLM mistakes cannot be eliminated. Validate findings yourself. Evidence paths and
line numbers are checked; Hannes cannot prove that reasoning is correct.

The [My_App example](../examples/my-app/README.md) illustrates Xcode checks
and a project review without coupling Hannes to that application.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Initialization/validation/help succeeded, or all required results PASS and status CURRENT |
| 1 | Verification FAIL, or status STALE |
| 2 | Configuration/reconciliation/execution/provider/report ERROR, declined or unavailable confirmation, status UNKNOWN, or invalid CLI arguments |
| 3 | Status requested before any report exists |

Status preserves a CURRENT report's original FAIL/ERROR exit code. UNKNOWN takes
precedence if snapshot coverage is incomplete, even when some differences are known.

## Optional Git hook

Git hooks are the first desired-state reconciliation domain. Declare their
execution-point mappings in version-3 `.hannes/config.toml`, then run `hannes pass auf`:

```toml
[hooks.git]
pre-commit = "local"
commit-msg = "message"
pre-push = "verify"
```

Assign rules to these execution points first. Hook names determine timing;
execution points select rules; check scripts implement validation. Pre-commit runs before
the message is finalized. Commit-msg receives the final message file. Pre-push
runs before references are transferred. All failures block Git with nonzero status,
including a missing `hannes` on PATH. Conventional Commits validation belongs in
a standalone checker, which must not invoke the hook again.

The lifecycle is **Validate -> Inspect -> Plan -> Confirm -> Apply -> Verify**.
`pass auf` performs validation and reconciliation without checks or provider calls.
`mach was` performs the same prerequisite, then executes its selected execution point.
Synchronized state succeeds without prompting or writing. Changes are displayed
as creation, managed-section update or removal and require `y`/`yes` in a terminal.
Declining, missing a terminal, conflicts or failures exit 2 and preserve the
previous report. Invalid configuration always fails before hook modification.
`--plan` previews rule execution only and never synchronizes hooks.

For a fresh CI checkout or other unattended setup, run:

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-push
```

The option explicitly approves the displayed integration plan without stdin or a
terminal. It defaults to off and is available only on `pass auf`. Invalid
configuration, ownership conflicts and apply/verification failures still exit 2;
recursion protection remains active. No checks or reviews run during setup, and
the previous report is preserved. Repeated setup performs no writes when already
synchronized. Auto-approval manages declared integrations only; it does not create
or repair governed files such as README, LICENSE or `.editorconfig`.

Generated wrappers supply explicit event and wrapper-path arguments validated by
Hannes against Git's effective directory and the declared execution point. Within hooks,
reconciliation is read-only: drift fails with instructions to run `hannes pass auf`
manually. No prompt or hook write occurs. A repository-scoped execution guard
blocks nested Hannes execution from checker processes.

Deterministic checks receive `HANNES_GIT_HOOK`, `HANNES_GIT_HOOK_ARG_COUNT` and
one-based `HANNES_GIT_HOOK_ARG_1`, `HANNES_GIT_HOOK_ARG_2`, etc. Values preserve
Git's arguments; relative paths are relative to the repository root, so checkers
using another working directory must resolve them accordingly. For a commit-msg
execution point, configure a dedicated script such as `command = "sh scripts/check-message.sh"`:

```sh
#!/bin/sh
set -eu
test "$HANNES_GIT_HOOK" = commit-msg
test "$HANNES_GIT_HOOK_ARG_COUNT" = 1
exec my-message-checker "$HANNES_GIT_HOOK_ARG_1"
```

Hannes does not expand environment variables in command strings. Checks retain
null stdin; pre-push reference data from stdin is not forwarded. Original Git
arguments still reach any unmanaged hook commands.

### Ownership and recovery

New hooks use `#!/bin/sh`, executable permissions, explicit
`# BEGIN HANNES MANAGED` / `# END HANNES MANAGED` markers and an ownership line
bound to the common Git repository. Existing executable `#!/bin/sh` hooks receive
a section immediately after the shebang; their remaining bytes and permissions
are preserved. Hannes runs first and propagates failure before custom commands.
Other shebangs, nonexecutable custom hooks, misplaced/malformed/duplicate markers
or foreign ownership are conflicts. Hannes reports the file and stops before
confirmation or writes. Inspect and correct the conflict manually, preserving
custom behavior, then rerun `hannes pass auf`. Do not copy managed wrappers between
repositories; declare mappings and let each clone generate its own wrappers.

Changing mappings updates managed sections. Removing mappings removes those
sections, preserving unmanaged content; whole-file deletion requires the exact
owned wrapper with no other content. Repeated synchronized runs preserve file
timestamps. A moved clone needs manual review of old ownership markers.

Git resolves the effective directory, including `core.hooksPath` and worktrees;
Hannes never changes Git configuration. Custom directories must be inside the
working-tree root, with no parent traversal or symlink components. External/shared
custom paths require manual integration or a repository-local `core.hooksPath`.
The default common-repository hooks directory is shared by linked worktrees;
all such worktrees must agree on mappings. Divergent worktree configurations cause
drift and must be reconciled deliberately. Symlinks and nonregular hooks fail
safely. Switching hook directories leaves the previous directory untouched;
remove its managed mappings before switching if you want Hannes to clean it up.

The complete plan is rechecked before application. Replacements are staged and
synced, then atomically replaced one file at a time. Multiple files are not one
atomic transaction. Failures report the affected operation/path, earlier operations
and rollback result. Rollback preserves concurrent edits and reports any paths it
cannot restore. After interruption, inspect the hooks and rerun `hannes pass auf`;
leftover `.hannes-hook-*` temporary files can be removed after ensuring no other
reconciliation is running. Missing directories are created after approval.

Local hooks are bypassable (`git commit --no-verify`, `git push --no-verify`).
They verify the working tree, not the exact staged index or pushed commit.
Exact-index verification is separate future work. CI can execute the same execution points
against a clean checkout; hosted authentication and deployment remain separate.
The repository's own versioned hook setup remains compatible with its pinned
published governor until native reconciliation is available in a published release.
