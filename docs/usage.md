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

`pass auf` validates all files, even disabled entries, and runs nothing.
`mach was` validates first, executes selected enabled checks in ID order, then
selected active reviews in one call if checks passed. Without a profile it selects
all enabled rules. Disabled items never enter results.
Any failed check prevents spending review tokens; those reviews are `NOT_RUN`.

Use `hannes mach was --geiz-ist-geil` to run only enabled deterministic checks.
The full configuration is still validated, but no review or LLM provider is called;
Hannes incurs no review-token usage. Progress counts only checks. Reviews are
excluded from results, and both the JSON (`checks_only: true`) and saved status
clearly identify the limited scope. PASS means the selected checks passed, not
that reviews passed. If no checks are enabled, the report explicitly says so.
The flag applies only to that invocation; an ordinary run still includes reviews.
No review result is cached or reused. Earlier reports default to the full scope.

## Select and preview rules

Opt in to configuration version 2 to define rule tags and profiles; see
[profiles and plans](profiles-and-plans.md). Existing version-1 projects retain
their selection of all enabled rules. Initialization still creates version 1.

```sh
hannes mach was --profil local --plan
hannes mach was --profil local --plan --json
hannes mach was --profil local
```

`--profil` overrides `default_profile`; otherwise the implicit profile is `all`.
Preview validates and reads local evidence without executing rules, Codex, MCP
or network calls, and preserves the previous report. It shows the selected scope,
omissions, requirements, input fingerprint and declared provider/context settings.
JSON errors are structured; executable plans exit 0, blocked/invalid plans exit 2.
`--json` requires `--plan`. Each execution rebuilds the same deterministic plan.

Ordinary profiles support checks-only execution. A gate requires every selected
rule, rejects empty/disabled requirements, and rejects `--geiz-ist-geil` if it
would omit a review. Current profiles verify working files only; staging/commit
inputs and per-review MCP permissions remain future work.

New reports retain the profile, plan ID and omissions. Status uses that saved
scope even after the default profile changes; legacy reports cannot certify a
profile gate. `--details` includes plan identity and omission reasons.

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
same `hannes mach was` against a clean checked-out commit. Personal ChatGPT tokens
and Codex authentication on hosted CI are out of scope. Hannes' own CI uses mocks
and never calls AI services.
