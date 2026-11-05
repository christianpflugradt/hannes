# Usage

## First run

From the root of a Git repository (an unborn repository from `git init` works):

```sh
hannes fang an
hannes pass auf
hannes mach mal
hannes was ist phase
hannes was ist phase --details
hannes so hilf mir doch
hannes --help
hannes --version
```

`fang an` creates `.Hannes/` (capital H), preserving all existing files. It never
installs hooks, stages files or commits. The enabled example uses `git --version`
and passes without Codex. The deliberately failing check and example review are
disabled. To demonstrate failure, change `enabled = false` to `enabled = true`
in `.Hannes/checks/deliberately-failing.toml` and run `hannes mach mal`.

Checks and reviews are collectively called **rules**. If neither is defined,
`mach mal` prints `Du hast noch keine Regeln definiert, du Vogel!` alongside the
report. Defined but disabled rules do not trigger this message.

`pass auf` validates all files, even disabled entries, and runs nothing.
`mach mal` validates first, executes all enabled checks in ID order, then all
active reviews in one call if checks passed. Disabled items never enter results.
Any failed check prevents spending review tokens; those reviews are `NOT_RUN`.

Use `hannes mach was --geiz-ist-geil` (also `hannes mach mal --geiz-ist-geil`)
to run only enabled deterministic checks. `mach was` is an alias for `mach mal`.
The full configuration is still validated, but no review or LLM provider is called;
Hannes incurs no review-token usage. Progress counts only checks. Reviews are
excluded from results, and both the JSON (`checks_only: true`) and saved status
clearly identify the limited scope. PASS means the selected checks passed, not
that reviews passed. If no checks are enabled, the report explicitly says so.
The flag applies only to that invocation; an ordinary run still includes reviews.
No review result is cached or reused. Earlier reports default to the full scope.

Errors and failures block success. A valid run writes `.Hannes/reports/last.json`
atomically, including on FAIL/ERROR. Invalid configuration leaves the previous
report untouched. Reports are ignored and remain local.

During `mach mal`, progress appears immediately on stderr: each enabled rule has
an `[i/n] LÄUFT` line with its description, followed by its result, duration and
short summary. Disabled rules are excluded from the total. Skipped reviews get
NOT_RUN without a running message. Preparation and report-writing phases are
also shown so snapshot work is not mistaken for a hung check.

All enabled reviews still share one Codex call. This is explicitly announced;
the review start lines appear together and individual verdicts arrive when that
shared call completes. The numbering is rule order, not a percentage or a claim
that Codex reports per-review completion in real time. Raw check output and
provider traces are not streamed. The final compact report stays on stdout,
so `hannes mach mal > report.txt` keeps progress visible in the terminal.
Both streams respect `NO_COLOR`, `--no-color` and their own TTY state.

Every result row includes a short summary. Review summaries come from the actual
Codex response, including PASS results; substantive verdicts are PASS/FAIL, while
provider problems or unavailable evidence remain ERROR. The AI's `reason` is one
nonempty line of at most 120 characters. Full findings and file/line references
stay in the local JSON, not in normal stdout. Long execution errors or legacy
summaries are shortened only for display; their complete content stays saved.

Use `hannes was ist phase --details` to inspect full saved findings and captured
command output separately. It does not run Codex or any check again. Old reports
remain readable; only `mach mal` obtains new provider results.

`was ist phase` reads that report without executing checks or Codex. It reports
CURRENT, STALE or UNKNOWN, with up to five paths and exact changed-file counts
when both snapshots are complete. Line deltas are unavailable: Hannes stores
hashes, not prior file content. HEAD-relative diffs are never mislabeled as changes
since verification. See [snapshot design](design.md).

PASS and CURRENT are green, FAIL red, ERROR/STALE/UNKNOWN amber.
Face colors follow the patience level. Help banners, commands and flags use cyan; headings are bold and secondary
metadata is muted. Piped streams, `NO_COLOR` (even empty) and `--no-color` disable
ANSI styling; stdout and stderr detect their terminal independently. Raw command output is available only in
local JSON, bounded to 64 KiB per stream with byte totals/truncation indicators.
It is not automatically redacted; configure commands to avoid printing secrets.

## Configure checks and reviews

See the [configuration reference](configuration.md). To add a check, create
`.Hannes/checks/test.toml`:

```toml
id = "test"
description = "Run project tests"
enabled = true
command = ["cargo", "test", "--locked"]
working_directory = "."
timeout_seconds = 300
```

Commands are argv arrays; a shell is used only if explicitly configured.
Treat deterministic checks like scripts you trust to run yourself. They are not
sandboxed. Check outputs or formatters that change relevant files cause an
integrity ERROR: rerun on stable files.

For an optional Codex review:

1. Install a compatible Codex CLI and authenticate using `codex login`.
2. Customize `.Hannes/reviews/product-consistency.md` for your project's criteria.
3. Set `enabled = true` in `.Hannes/reviews/product-consistency.toml`.
4. Run `hannes pass auf`, then `hannes mach mal`.

New provider configuration uses `codex-cli`, a 180-second timeout, `gpt-6.1-sol`,
medium reasoning and no MCP servers. Model and MCP allow list are configurable in
`.Hannes/provider.toml`; see [review context and profile](configuration.md). A ChatGPT-backed local login works without a separate
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

The [Passing By example](../examples/passing-by/README.md) illustrates Xcode checks
and a project review without coupling Hannes to that application.

Unsupported or incomplete commands print an ASCII face with one randomly chosen
dialect saying from the bundled list of 41, followed by a hint to run
`hannes so hilf mir doch`. This goes to stderr with exit code 2 and executes
no project rules.

### Patience

The first invalid command shows the friendly green face (0). Consecutive invalid
commands show 1, 2, 3, 4, then stay at 4. Every recognized command improves the
reaction by one step; the next invalid command shows that improved face before
subsequent errors escalate again. For example: invalid → 0, invalid → 1,
help → recovery, invalid → 0.

The 41 sayings are shared by all mood levels. Hannes stores the indices of the
last 20 sayings in the global `patience` state and randomly chooses from the
remaining entries: at least 20 other sayings must appear before one can repeat.
Valid commands preserve this history; the face-free farewell does not add an entry.
Selection and reservation share the existing state lock, including concurrent CLI
calls. Old state files start with an empty quote history without resetting mood.
The bounded list is removed with the external state during self-uninstall.
If state storage fails, the existing warning still allows an unrecorded random
fallback, without a repeat-avoidance guarantee for that invocation.
At level 4 the entire saying is
uppercased. Friendly is green, irritated/annoyed amber, angry/furious red,
respecting the usual color controls. Faces and sayings appear only for invalid
commands. Help contains no face, mood label, or numeric debug information. Before an
invalid-command response (including the farewell), Hannes clears the visible
terminal screen and returns the cursor to the top left. Existing scrollback is
preserved. This happens only when stderr is a terminal and `TERM` is not `dumb`;
redirected output has no clear sequences. Valid commands and help leave the screen
alone. `NO_COLOR` and `--no-color` disable colors, not this interactive screen control.

Recognized commands calm Hannes even if their checks or configuration fail.
Explicit help/version requests count as recognized; no arguments show help without
changing patience. Mood never blocks a command or changes verification results.

After three consecutive unknown-command reactions at the screaming level (4),
Hannes removes its own running executable and prints one red farewell line with
one blank line above and below, without a face or help hint. Any recognized command
(including help/version) improves the mood and resets this streak. No arguments
leave both unchanged. On Windows the installed name is removed immediately and a
hidden PowerShell helper deletes the renamed image after exit. A removal failure
prints an error, never a false success. Project files, reports and other binaries
are preserved. The external `patience` and `patience.lock` files are removed before
the executable; the state directory is removed only if empty. Foreign files are
preserved and directories are never deleted recursively. Repository paths and
`.Hannes` paths are rejected as state locations, including symlinked overrides.
If cleanup fails, the executable is kept and an error is reported.

From the Hannes source checkout, reinstall with `mise deploy`. Its forced
Cargo installation also works when the deleted executable still has an installation
receipt. The first invalid command after reinstall shows the green friendly face.
`mise install` alone installs the development toolchain, not the Hannes executable.


State is user-wide across repositories, stored in a small JSON file named `patience`:

- macOS: `~/Library/Application Support/Hannes/`
- Linux: `$XDG_STATE_HOME/hannes/`, or `~/.local/state/hannes/`
- Windows: `%LOCALAPPDATA%/Hannes/`

`HANNES_STATE_DIR` can override the directory with an absolute path, for example
for isolated tests. Keep it outside the project so it does not invalidate snapshots.
Deleting `patience` resets to the friendly first reaction; invalid contents also
reset. Existing integer state files are migrated automatically. A small OS file
lock serializes simultaneous updates, and writes use an atomic rename. Storage errors produce a warning, but the requested command still
runs; an invalid command then uses a temporary friendly face without persistence.
No database or additional dependencies are used.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Initialization/validation/help succeeded, or all required results PASS and status CURRENT |
| 1 | Verification FAIL, or status STALE |
| 2 | Configuration/execution/provider/report ERROR, status UNKNOWN, or invalid CLI arguments |
| 3 | Status requested before any report exists |

Status preserves a CURRENT report's original FAIL/ERROR exit code. UNKNOWN takes
precedence if snapshot coverage is incomplete, even when some differences are known.

## Optional Git hook

**Local hooks are bypassable (`git push --no-verify`). This hook checks the working
tree, which may differ from the commits being pushed. It is not a commit guarantee.**
The example is never installed automatically. From a project's root, after copying
`scripts/pre-push` from this repository to that project's scripts directory:

```sh
hook_dir="$(git rev-parse --git-path hooks)"
mkdir -p "$hook_dir"
# Refuse to overwrite an existing hook; integrate it manually if one exists.
(set -C; cat scripts/pre-push > "$hook_dir/pre-push")
chmod +x "$hook_dir/pre-push"
```

The script respects Git's selected hook directory (including `core.hooksPath`).
Inspect existing hook management before installing. A future CI gate can run the
same `hannes mach mal` against a clean checked-out commit. Personal ChatGPT tokens
and Codex authentication on hosted CI are out of scope. Hannes' own CI uses mocks
and never calls AI services.
