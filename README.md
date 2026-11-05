# Hannes

```text
  /\  /\__ _ _ __  _ __   ___  ___
 / /_/ / _` | '_ \| '_ \ / _ \/ __|
/ __  / (_| | | | | | | |  __/\__ \
\/ /_/ \__,_|_| |_|_| |_|\___||___/

------ Der Harness ohne 'r' ------
```

*Hannes schaut nach dem Rechten.* A Rust CLI for project verification:
**checks** run deterministic commands, **reviews** ask an LLM for a structured
verdict. Both are **rules**. Hannes reports PASS, FAIL or ERROR; it does not repair code.

## Install or build

[GitHub Releases](https://github.com/christianpflugradt/hannes/releases) target
**macOS Apple Silicon**, **macOS Intel** and **Linux x64**. Choose the matching
archive, verify it against `SHA256SUMS`, extract it and install:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 hannes-vVERSION-TARGET/hannes "$HOME/.local/bin/hannes"
```

Replace `VERSION`/`TARGET` with the archive's values. Add
`export PATH="$HOME/.local/bin:$PATH"` to `~/.zshrc` if needed.
Linux downloads use glibc and are built on Ubuntu 24.04; binaries are not signed
or notarized. See [release details](docs/releasing.md).

To build locally, install **Git**, **stable Rust/Cargo 1.85+** (edition 2024) and
a native compiler/linker: Xcode Command Line Tools on macOS, GCC/Clang on Linux,
or Visual Studio C++ Build Tools on Windows:

```sh
cargo build --release --locked
./target/release/hannes --help
```

The binary is `target/release/hannes` (`hannes.exe` on Windows).
`cargo install --path . --locked` installs it on your Cargo PATH.
Git remains a runtime dependency; Codex CLI/login is needed only for LLM reviews.

Optional [mise](https://mise.jdx.dev/) setup installs Rust with rustfmt/Clippy,
Python 3.13 for developer scripts and Node 24 for release tooling:

```sh
mise trust
mise install                  # Development tools
mise build                    # Release binary
mise deploy                   # Build/install to ~/.local/bin; overwrite existing binary
mise check                    # Formatting, Clippy, tests, build and smoke test
```

Override the install prefix with `CARGO_INSTALL_ROOT=/your/prefix mise deploy`;
the executable goes into its `bin/` directory. Windows uses Cargo's install root.
Python and Node are not needed to run Hannes or build it directly with Cargo.
Other tasks: `format`, `format:check`, `lint`, `test`, `smoke`, `hooks:install`,
`hooks:test`. `mise format` formats Rust; built-in `mise fmt` formats `mise.toml`.
`mise run TASK` also works for every project task.

`mise release` uses the authenticated `gh` CLI to dispatch the release workflow
for **remote main**, then returns. It does not commit/push local changes or wait
for publication. Semantic Release handles versions, changelog and native archives;
see [release instructions](docs/releasing.md).

## Use

Run from the Git working-tree root:

```sh
hannes fang an                # Create missing .Hannes files; preserve existing ones
hannes pass auf               # Validate configuration; execute no rules/provider
hannes mach mal               # Execute selected rules and save a report
hannes was ist phase          # Show saved results and freshness
hannes so hilf mir doch        # Help; --help and --version also work
```

`mach was` is an alias of `mach mal`. Initialization creates a passing check and
a disabled review, so the first run needs no Codex. Selected checks run first in
ID order; selected reviews follow in one shared model call only if checks pass.
Failed checks leave reviews NOT_RUN. Progress goes to stderr; compact results and
short AI summaries go to stdout. Raw output stays in the report/`--details` view.
With no check or review definitions, Hannes also prints
`Du hast noch keine Regeln definiert, du Vogel!`.

| Option | Applies to | Effect |
| --- | --- | --- |
| `--profil NAME` | `mach mal` / `mach was` | Override the configured rule-selection profile. |
| `--plan` | `mach mal` / `mach was` | Preview selection and preparation without executing rules or provider calls. |
| `--json` | With `--plan` | Machine-readable plan or structured error. |
| `--geiz-ist-geil` | `mach mal` / `mach was` | Checks only; no provider discovery, MCP connection or review-token usage. |
| `--details` | `was ist phase` | Show saved findings and captured output without rerunning anything. |
| `--no-color` | Global | Disable colors; `NO_COLOR` and redirected output also disable them. |

Exit codes: **0** success; **1** verification FAIL or stale saved results;
**2** configuration/execution/provider errors or UNKNOWN freshness;
**3** no saved report for status. A plan exits 0 if executable, 2 if blocked/invalid.

Unknown commands show a face and a saying, avoiding the last 20 sayings globally.
Repeated errors worsen the mood across invocations; recognized commands improve
it by one step. Five moods range from green to red; maximum anger shouts in capitals.
The third consecutive maximum-anger reaction actually uninstalls
Hannes and removes its external mood/history state. Repository files and `.Hannes`
are preserved. Reinstall with `mise deploy`. See [usage details](docs/usage.md).

## Configuration: `.Hannes/`

Use exactly the capitalized directory name. Versions **1 and 2** are supported;
initialization creates version 1. Version 2 enables tags and profiles explicitly.

```text
.Hannes/
  config.toml
  provider.toml
  checks/*.toml
  reviews/*.toml
  reviews/*.md                 # Referenced review criteria
  profiles/*.toml              # Optional; version 2 only
  reports/.gitignore
  reports/last.json            # Generated, not configuration
  README.md                    # Editable guidance, not parsed settings
```

Only immediate `.toml` files in rule/profile directories are loaded. Filenames
need not match IDs. Unknown fields, duplicate TOML keys/IDs and invalid values are
errors, including in disabled rules and unused profiles. Text files are UTF-8.
Paths must exist, be relative to their documented base and resolve inside the
project; absolute paths and `..` are rejected.

`hannes pass auf` validates configuration and referenced files without executing
checks or Codex. Model availability and MCP discovery are checked during execution.

### Project: `config.toml`

```toml
version = 1
project = "Passing By"
max_display_files = 5
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `version` | Required | `1`, or `2` for tags/profiles. No automatic migration. |
| `project` | Required | Nonblank display name; initialization uses the directory name. |
| `max_display_files` | Required | Integer `1..5`; changed paths shown in status, not a verification limit. |
| `default_profile` | Optional; `"all"` | Version 2 only: default profile for execution and planning. |

### Common rule fields

Checks and reviews share these fields:

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Case-sensitive, unique across both kinds: 1–80 ASCII letters, digits, `-` or `_`; `hannes-` is reserved. |
| `description` | Required | Nonblank progress description. |
| `enabled` | Required | Boolean; disabled rules are validated but omitted from execution/results. |
| `tags` | Optional; `[]` | Version 2 only: up to 32 unique lowercase ASCII tags, each 1–32 characters, starting with a letter/digit and otherwise using letters, digits, `-` or `_`. |

Required nonblank text fields reject NUL. Version 1 rejects `tags`, even `[]`.
Execution uses selected enabled checks first, then reviews, each kind sorted by ID.

### Checks: `checks/*.toml`

```toml
id = "unit-tests"
description = "Run Rust unit and integration tests"
enabled = true
command = ["cargo", "test", "--locked"]
working_directory = "."
timeout_seconds = 120
```

In addition to the common fields:

| Field | Required / default | Meaning |
| --- | --- | --- |
| `command` | Required | `1..256` argv strings; first is a nonblank executable. Each string is at most 32768 bytes and contains no NUL. |
| `working_directory` | Optional; `"."` | Existing directory relative to the project root. |
| `timeout_seconds` | Required | Integer `1..3600` for this check. |

Commands run without a shell or stdin and inherit the environment. Bare executable
names resolve through PATH; paths such as `./scripts/check.sh` resolve relative to
`working_directory` and must be runnable files inside the project. Pipes, `&&` and
variable expansion require an explicit shell, e.g.
`command = ["sh", "-c", "cargo test && cargo clippy"]`.

Checks are trusted commands, not sandboxed code. Exit 0 means PASS, nonzero/signal
means FAIL, and timeout/spawn/capture problems mean ERROR. Remaining checks still
run after a failure. Changes to verified files prevent overall PASS. Captured
stdout/stderr are limited to 64 KiB each, with truncation/byte totals recorded;
output is not automatically redacted, so avoid printing secrets.

### Reviews: `reviews/*.toml` and criteria Markdown

```toml
id = "accessibility"
description = "Check documented keyboard accessibility requirements"
enabled = true
instructions = "accessibility.md"
files = ["Sources/App/Settings.swift", "Tests/SettingsTests.swift"]

[[context]]
path = "Product.md"
headings = ["Keyboard accessibility"]
```

This example needs the listed files and heading. The generated `product-consistency`
review starts disabled; customize its criteria before enabling it.

| Field | Required / default | Meaning |
| --- | --- | --- |
| `instructions` | Required | Nonblank UTF-8 `.md` file relative to `.Hannes/reviews/`, staying inside that directory. |
| `files` | Optional; `[]` | Up to 64 unique existing files relative to the project root; exact paths, no globs. Source entry points. |
| `context` | Optional; `[]` | Up to 32 requirement-document selections using `[[context]]`. |
| `context.path` | Required per entry | Existing UTF-8 file relative to the project root. |
| `context.headings` | Optional; `[]` | Up to 32 nonblank exact Markdown heading titles; empty/omitted selects the whole file. |

Write the criterion in `reviews/accessibility.md`, for example:

```markdown
Check the keyboard requirements in the supplied Product.md section. Inspect the
listed source/tests and necessary dependencies. FAIL for a concrete violation,
ERROR for missing essential evidence, PASS only with grounded evidence.
```

Heading selections match one ATX title (`#`–`######`) without its `#` prefix,
including subsections until the next heading of equal/higher level. Code-block
headings are ignored; missing/ambiguous titles are errors. Shared/overlapping
selections are merged with original line numbers. Across all definitions, including
disabled ones, limits are 32 distinct context files, 1 MiB per file and 64 KiB of
selected text. Empty files and excessive context fail; nothing is silently dropped.

Selected reviews share one read-only Codex call. If all declare `files`, initial
manifest/diff hints focus on their combined entry/context paths; otherwise they
remain project-wide. Entry points do not restrict further dependency reads.
Reviews evaluate current working files, not just the diff; untracked contents
need direct inspection. Criteria cannot change provider permissions/output format.

Each requested review must return exactly one PASS/FAIL/ERROR, a nonblank summary
of at most 120 characters on one line, and up to 20 findings of at most 2000 bytes
each. PASS/FAIL require an existing verified file and positive line reference.
Malformed/missing/duplicate results make the batch ERROR. LLM judgments are fallible.

### Profiles: `profiles/*.toml` (version 2)

Set `version = 2` in `config.toml`, preserving its other fields. Optionally set
`default_profile = "local"`, then create `profiles/local.toml`:

```toml
id = "local"
rules = ["always-ok"]
include_tags = []
gate = false
input = "worktree"
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Unique profile ID using the tag syntax; `all` is reserved. |
| `rules` | Optional; `[]` | Unique existing rule IDs. |
| `include_tags` | Optional; `[]` | Unique tags occurring in at least one rule definition. |
| `gate` | Optional; `false` | Every selected rule is required; empty gates and disabled requirements are errors. |
| `input` | Optional; `"worktree"` | Current working files only; `index` and `commit` are rejected. |

Selection is the union of explicit IDs and rules matching **any** listed tag,
without duplicates. To use tags, add e.g. `tags = ["local"]` to a rule and
`include_tags = ["local"]` to the profile. Unknown selectors/duplicates fail.
`--profil` overrides `default_profile`; absent both, `all` selects all enabled rules.
Version 1 rejects profile TOML files and `default_profile`. No inheritance,
exclusions, implicit hooks or provider permissions are attached to profiles.

```sh
hannes mach was --profil local --plan
hannes mach was --profil local --plan --json
hannes mach mal --profil local
hannes mach was --profil local --geiz-ist-geil
```

Plans show order, selection reasons, omissions, gate requirements, input identity,
model/MCP settings and bounded context/diff sizes. No checks, provider CLI, MCP
servers or network calls run; the existing report is preserved. Availability and
inherited Codex defaults remain unchecked/unresolved. Normal mood handling applies.
JSON uses plan schema 1 with `planned`/`blocked` status and structured diagnostics.
Each execution rebuilds its plan; a plan ID is not a cached result or permission
to run a later changed tree. See [plan details](docs/profiles-and-plans.md).

Checks-only runs omit reviews and label the limited scope. Gates requiring reviews
reject `--geiz-ist-geil` before checks; checks-only gates may use it. Executed
failures always block, including outside gate mode. Empty ordinary profiles
explicitly prove no rule passed. A worktree gate does not verify staged/pushed code.

### Provider: `provider.toml`

Only **Codex CLI** is supported. Generated settings:

```toml
provider = "codex-cli"
timeout_seconds = 180
model = "gpt-6.1-sol"
mcp_servers = []

[review_profile]
reasoning_effort = "medium"
tool_output_token_limit = 2000
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `provider` | Required | Exactly `"codex-cli"`. |
| `timeout_seconds` | Required | Integer `1..3600` for discovery plus model invocation; discovery itself has a maximum of 10 seconds. |
| `model` | Optional; Codex default | Nonblank identifier passed as `--model`. New scaffolds select GPT-6.1 Sol (`gpt-6.1-sol`); alternatives must be supported by your CLI/account. |
| `mcp_servers` | Optional; `[]` | Allow list of server names already configured in Codex; empty/omitted disables all. |
| `review_profile` | Optional table | Provider settings below; not a selection profile. |
| `review_profile.reasoning_effort` | Optional; Codex default | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, or `ultra`; availability depends on model/CLI. |
| `review_profile.tool_output_token_limit` | Optional; `2000` | Integer `256..16384`, retained tokens per tool output, not a total token/spending cap. |

These settings apply to the shared call for selected reviews. Unsupported models
or settings fail; Hannes does not silently substitute another model.

**Finding Codex.** Hannes starts `codex` through its inherited PATH from the project
root, without an interactive shell, startup files or aliases/functions. There is
no configurable binary path. Ensure the CLI directory is on PATH in the same
terminal, IDE or hook environment, then check:

```sh
command -v codex                 # POSIX shells; Windows: where.exe codex
codex --version
codex login
```

Hannes uses the existing local login with a compatible noninteractive CLI,
structured output, ephemeral sessions, read-only sandbox and no approvals.
It removes `OPENAI_API_KEY`/`CODEX_API_KEY` from Codex child processes; login-backed
reviews still consume account allowance. Missing CLI/authentication, unsupported
options or exhausted allowance yield ERROR. Credentials are never copied to CI.

**MCP access.** Replace the top-level empty list to allow existing Codex servers:

```toml
mcp_servers = ["project-memory", "browser"]
```

At most 32 unique names, each 1–256 ASCII letters, digits, `_` or `-`; no wildcard.
Server commands, URLs, transports and credentials are configured in Codex, not
`.Hannes`. All selected reviews get the same allow list; no per-review overrides.

Before the model call, `codex mcp list --json` reads configuration without starting
a model/server. Hannes disables every unlisted server in the child process.
Unknown allowed names, failed discovery, more than 256 discovered servers or unsafe
names anywhere in that list block the review. Empty does not inherit personal MCPs.
Automatic skill instructions, bundled skills, skill search/installation,
apps/plugins, subagents and web search are disabled. Codex login, project AGENTS.md
and ordinary safety settings remain; user Codex configuration is not rewritten.
Enabled MCPs remain trusted capabilities rather than an additional permission sandbox.

### Reports and other editable files

`reports/last.json` is generated atomically with verdicts, short summaries, full
findings, captured check output, available usage and input identity. Schema 2 saves
the selected profile/plan and omissions. Status displays that historical scope,
not today's default. Selection completeness does not mean execution completeness:
failed checks leave reviews NOT_RUN. Old schema-1 reports remain readable as
unprofiled legacy evidence and cannot certify a profile gate.

Status compares working files and HEAD with the saved snapshot: CURRENT, STALE or
UNKNOWN. Line deltas are unavailable with the hash-only baseline. Invalid
configuration preserves the previous report; deleting it clears saved status.

Keep reports local with the editable `reports/.gitignore`:

```gitignore
*
!.gitignore
```

Commit configuration/criteria, not reports or credentials. `.Hannes/README.md` is
editable guidance with no parsed settings. Other assets have no implicit execution
meaning; criteria Markdown must be referenced by a review. Mood/quote state lives
outside repositories; faces, sayings and uninstall behavior have no `.Hannes` setting.

### Fixed limits

These are built in, not configurable fields:

| Area | Limit / behavior |
| --- | --- |
| Metadata | 64 KiB per TOML file; up to 256 checks, 256 reviews and 256 profiles. |
| Review instructions | 32 KiB per file; 256 KiB combined, including disabled reviews. |
| Initial orientation | Up to 200 manifest paths and 32 KiB of diff, with truncation indicated. Required evidence still needs inspection. |
| Snapshot | 8 MiB/file, 128 MiB total, 100,000 paths; Git file enumeration is capped at 64 KiB; `.Hannes` nesting at 16 levels. |
| Coverage | Tracked and nonignored untracked files, plus `.Hannes` even if ignored; `.git` and `.Hannes/reports` excluded. Unsupported symlinks/submodules/paths or exceeded limits block PASS. |
| Stability | Working files/HEAD must match before/after execution and before a model call. This does not separately verify staging or pushed commits. |
| Usage | Available input/cached-input/output counters once per call; total = input + output. Optional reasoning/cache-write counters are subsets. Missing telemetry stays unavailable; no money/quota estimate or enforced total token budget. |

Not implemented: per-review model/MCP settings, true change-only reviews,
index/commit inputs, configurable run-wide budgets and result caching. See the
[remaining TODOs](docs/todo/README.md).

## More

[Usage and Git hook](docs/usage.md) · [Configuration](docs/configuration.md) ·
[Design and limits](docs/design.md) · [Passing By example](examples/passing-by/README.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Releasing](docs/releasing.md)

Install the local Angular commit-message check with `mise hooks:install`;
see [Contributing](CONTRIBUTING.md#commits). Released under the [Unlicense](LICENSE).

## Frequently Asked Questions

### What is the difference between Hannes and a harness?

The R.

### Is Hannes an artificial intelligence?

No. There is enough artificial intelligence already. Hannes is artificial emotionality.

### Is Hannes available for Windows?

I don't understand the question.
