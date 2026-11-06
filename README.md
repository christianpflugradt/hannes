# Hannes

```text
  /\  /\__ _ _ __  _ __   ___  ___
 / /_/ / _` | '_ \| '_ \ / _ \/ __|
/ __  / (_| | | | | | | |  __/\__ \
\/ /_/ \__,_|_| |_|_| |_|\___||___/

------ Der Harness ohne 'r' ------
```

*Hannes schaut nach dem Rechten.* Hannes is an open-source, agent-agnostic
governance framework for software development: you define the expectations that
development results must satisfy as version-controlled rules alongside your code.
The same rules apply whether a person or a coding agent writes the code.
Today, its native CLI evaluates these rules in Git projects through deterministic
checks and AI reviews, recording verdicts and supporting evidence.

Use it when a successful build is only part of your definition of “ready”: tests
and linters can verify concrete conditions, while an AI review can compare your
implementation with written requirements and architectural decisions. Developers
and coding agents choose how to implement changes; Hannes evaluates whether the
results meet the configured expectations. See the [product vision](docs/product-vision.md)
for its principles and long-term direction.

## Contents

- [How Hannes works](#how-hannes-works)
  - [Terms](#terms)
- [Getting started](#getting-started)
  - [Requirements](#requirements)
  - [Install a release](#install-a-release)
  - [Build locally](#build-locally)
  - [Initialize your project](#initialize-your-project)
- [Using Hannes](#using-hannes)
  - [Commands and options](#commands-and-options)
  - [Run checks without AI](#run-checks-without-ai)
  - [Read results](#read-results)
  - [Choose rules and preview a run](#choose-rules-and-preview-a-run)
  - [Use a Git hook or CI](#use-a-git-hook-or-ci)
- [Configuration](#configuration)
  - [Files and validation](#files-and-validation)
  - [Project settings](#project-settings)
  - [Fields shared by checks and reviews](#fields-shared-by-checks-and-reviews)
  - [Add a check](#add-a-check)
  - [Configure the AI provider](#configure-the-ai-provider)
  - [Add a review](#add-a-review)
  - [Group rules into rule sets](#group-rules-into-rule-sets)
  - [Reports and other files](#reports-and-other-files)
- [Limits and unsupported features](#limits-and-unsupported-features)
- [Development](#development)
- [Frequently Asked Questions](#frequently-asked-questions)

## How Hannes works

Your project's governance rules live in `.hannes/` alongside the source code,
where they can be reviewed and versioned together. Run `hannes mach was` to
evaluate the current working files against those expectations.

Hannes validates the configuration, runs the selected checks, and then asks the
AI to perform the selected reviews. If a check fails, reviews are skipped to avoid
spending tokens on a run that has already failed. All remaining checks still run.
The terminal shows progress as each rule starts and finishes, followed by a compact
report. Detailed results are saved locally for later inspection.

Start with checks for things your tools can establish directly, such as tests,
compilation or formatting. Add reviews for questions that need interpretation,
such as whether code follows requirements in a product document. AI judgments
can be wrong; a review PASS is evidence to inspect, not a guarantee of correctness.

### Terms

| Term | Meaning | Example |
| --- | --- | --- |
| **Rule** | One thing Hannes verifies. Every rule is either a check or a review. | “Unit tests pass” or “The documented CLI matches the implementation.” |
| **Check** | A command whose exit status determines the result. It uses no AI provider and has no review-token cost. | Run `cargo test --locked`. |
| **Review** | An AI request to evaluate written criteria against project files and return a structured verdict with a short explanation. | Compare documented commands with their implementation. |
| **Provider** | The integration that executes AI reviews. Currently this is your locally installed, authenticated Codex CLI. | Codex using `gpt-6.1-sol`. |
| **Report** | The saved results of a run, including explanations, detailed output and the files that were verified. | `.hannes/reports/last.json`. |

A check is deterministic in how Hannes processes its exit status; its reliability
still depends on the command you choose. A review is an LLM (large language model)
judgment and can differ between runs. Use checks wherever a command can answer
the question reliably.

## Getting started

### Requirements

- **Git**, with your project already in a Git working tree. Run Hannes from its
  root directory, not a subdirectory.
- The tools used by your checks, such as Cargo, npm or a linter, on your PATH.
- **Codex CLI and a local login only if you enable reviews.** Checks work without
  Codex. See [provider setup](#configure-the-ai-provider).

The Hannes executable needs no Rust, Python or Node runtime. Rust is needed to
build it from source; Python and Node are used by Hannes' development tooling.

### Install a release

Download the matching archive and `SHA256SUMS` from
[GitHub Releases](https://github.com/christianpflugradt/hannes/releases):

| Platform | Target in the archive name |
| --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Linux x64 | `x86_64-unknown-linux-gnu` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |

From a source checkout, `mise deploy` downloads the latest published release,
selects the current OS/CPU, verifies its checksum and executable compatibility,
and installs it to `~/.local/bin`, replacing an existing binary. It requires
Python 3.11+ and network access, and never builds local source. Unsupported
platforms (including Windows and musl Linux) fail with a helpful error.
Set `CARGO_INSTALL_ROOT=/your/prefix mise deploy` to install into that prefix's
`bin/` directory. Download or compatibility failures preserve the installed copy.

For manual installation, verify the checksum, then extract and install it. Replace
`VERSION` and `TARGET` below with the downloaded archive's values:

```sh
# macOS; use sha256sum instead on Linux and compare with SHA256SUMS.
shasum -a 256 hannes-vVERSION-TARGET.tar.gz

tar -xzf hannes-vVERSION-TARGET.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 hannes-vVERSION-TARGET/hannes "$HOME/.local/bin/hannes"
```

Ensure `~/.local/bin` is on PATH; for zsh, add
`export PATH="$HOME/.local/bin:$PATH"` to `~/.zshrc` and open a new terminal.
Check the installation with `hannes --version`.

Linux archives use glibc and are built on Ubuntu 24.04. Downloads are not signed
or notarized. Windows release builds are not provided.

### Build locally

Install **Git**, **stable Rust/Cargo 1.85+** (edition 2024) and a native compiler/linker:
Xcode Command Line Tools on macOS or GCC/Clang on Linux. From a source checkout:

```sh
cargo build --release --locked
./target/release/hannes --help
```

The executable is `target/release/hannes`. To install it on your Cargo PATH,
run `cargo install --path . --locked`.

With [mise](https://mise.jdx.dev/), development setup and release installation are:

```sh
mise trust
mise install                  # Install development tools.
mise build                    # Build target/release/hannes.
mise deploy                   # Download and install the latest published release.
```

`mise install` installs the toolchain, not Hannes itself. See [Development](#development) for checks and release tasks.

### Initialize your project

In your application's Git root:

```sh
cd /path/to/My_App
hannes fang an
hannes pass auf
hannes mach was
hannes was ist phase
```

`fang an` creates `.hannes/` with a passing example check (`git --version`) and
a disabled example review. Running it again only creates missing files; it
preserves your edits. It does not install hooks or make a Git commit.

`pass auf` validates configuration and reconciles declared external state after
confirmation. It runs no checks or reviews. The first
`mach was` should pass without Codex because the example review is disabled.
This confirms that Hannes works; it does not yet verify your application's quality.

Next, [add a real check](#add-a-check) and replace or remove the example check.
When you need AI judgment, [configure Codex](#configure-the-ai-provider),
[write a review](#add-a-review), validate again, then run `hannes mach was`.
Commit your rules and criteria so other users can run the same verification.

## Using Hannes

### Commands and options

The command phrases are German and must be entered as shown, without quotes:

| Command | What it does |
| --- | --- |
| `hannes fang an` | Create missing configuration files, preserving existing files. |
| `hannes pass auf` | Validate configuration and reconcile declared state after confirmation; run no checks or reviews. |
| `hannes mach was` | Validate and reconcile first, then run selected rules and save results. |
| `hannes was ist phase` | Show the last saved report and whether it still matches the working files. |
| `hannes so hilf mir doch` | Show help. |

| Option | Used with | Meaning |
| --- | --- | --- |
| `--auto-approve` | `pass auf` | Approve the displayed integration changes without a prompt, including in CI. Defaults to off. |
| `--geiz-ist-geil` | `mach was` | Run only checks; do not call an AI provider. |
| `--at NAME` | `mach was` | Select a known execution point (default `pre-push`). |
| `--plan` | `mach was` | Preview what would run without executing rules or calling the provider. |
| `--json` | `mach was --plan` | Print the plan or plan errors as JSON. |
| `--details` | `was ist phase` | Show full findings and captured check output. |
| `--no-color` | Any command | Disable colors. `NO_COLOR` and redirected output also disable colors. |
| `-h`, `--help` | Any command | Show help. |
| `-V`, `--version` | `hannes` | Show the version. |

At the selected execution point, enabled checks run first, followed by reviews.
Within each kind, rules run in ID order. Disabled rules are validated but are not
executed or included in results. Selected reviews share one Codex invocation.

If neither checks nor reviews are defined, Hannes additionally prints
`Du hast noch keine Regeln definiert, du Vogel!`. A successful empty run does not
establish that your project passed any quality rules.

### Run checks without AI

```sh
hannes mach was --geiz-ist-geil
```

The selected execution point must contain checks only. A required review makes
this invocation fail before any rule runs.

This runs selected enabled checks only: no Codex discovery, MCP connections or
review-token usage. The report is explicitly marked as checks-only and omits
reviews. A checks-only PASS says nothing about your review criteria.
The whole configuration is still validated, including disabled and omitted rules.

### Read results

Progress goes to stderr; the final compact report goes to stdout. Redirecting
`hannes mach was > report.txt` keeps progress visible in the terminal. Raw command
output is saved instead of flooding the terminal.

| Rule result | Meaning |
| --- | --- |
| `PASS` | The check exited successfully, or the AI found the review criterion satisfied with evidence. |
| `FAIL` | The check failed, or the AI found a concrete violation of the criterion. |
| `ERROR` | Verification could not be completed, for example because of a timeout, missing executable, invalid AI output or missing evidence. |
| `NOT_RUN` | A review was skipped because checks failed. It has no review verdict. |

A run succeeds only if its required results pass and its input remains stable.
Check failures and errors prevent overall PASS. A review's one-line explanation
appears even when it passes. Read more without running anything again:

```sh
hannes was ist phase
hannes was ist phase --details
```

Status compares the saved **snapshot** (file hashes and Git HEAD recorded during
the run) with the current working files. `CURRENT` means they match, `STALE` means
they changed, and `UNKNOWN` means the comparison could not be completed reliably.
CURRENT describes freshness, not success: a current report can still contain failures.
Hannes checks the working files, including uncommitted changes; this does not
prove that the staged or pushed commit contains the same content.

For reviews, available input, cached-input and output token counts are recorded
once for the shared provider call. Total tokens are input plus output; cached
input is a subset of input. Missing usage stays unavailable. Hannes does not
calculate money, remaining account quota or enforce a total token budget.

| Exit code | Meaning |
| --- | --- |
| `0` | Command succeeded, verification passed, or saved results passed and are current. An executable plan also exits 0. |
| `1` | Verification failed, or saved results are stale. |
| `2` | Configuration, execution, provider or report error; unknown freshness; invalid arguments; blocked/invalid plan; declined, unavailable or failed reconciliation. |
| `3` | No saved report exists for the status command. |

### Choose rules and preview a run

Execution points select the rules assigned to a named context. Use `--plan` for
an offline preview of the same selection and prepared review input:

```sh
hannes mach was --at pre-push --plan
hannes mach was --at pre-push --plan --json
hannes mach was --at pre-push
```

See [rule sets and execution points](#group-rules-into-rule-sets) for assignments.

### Use a Git hook or CI

Declare Git events and their execution points in version-3 configuration:

```toml
[hooks.git]
pre-commit = "pre-commit"
commit-msg = "commit-msg"
pre-push = "pre-push"
```

Assign rules to the referenced execution points first. These are the three supported events;
`commit-msg` receives the final message file, while `pre-commit` runs before it is
available. Hannes delegates message validation to your configured checker.

`hannes pass auf` validates configuration, inspects actual state, plans changes,
asks for confirmation, applies the approved plan and verifies it. `mach was` does
the same before executing its execution point. Synchronized repositories need no prompt
or writes. Declining, conflicts, failed reconciliation or a required confirmation
without a terminal exit 2 before checks; run `hannes pass auf` interactively.
`mach was --plan` remains a read-only execution preview and does not synchronize.

For unattended setup, explicitly approve declared integration changes:

```sh
hannes pass auf --auto-approve
hannes mach was --at pre-push
```

`--auto-approve` prints and approves the reconciliation plan without reading stdin
or requiring a terminal. Validation, conflict detection and state verification
still apply. It runs no checks or reviews and leaves an existing report untouched.
Synchronized state causes no writes. The option is available only on `pass auf`;
ordinary execution retains its existing confirmation behavior.

Hannes marks its sections explicitly, preserves unmanaged commands, and refuses
ambiguous ownership or unsafe composition. Removed mappings remove only managed
content. Generated hooks verify synchronization read-only and direct you to
`hannes pass auf` on drift. Local hooks are bypassable and verify working files,
which may differ from staged or pushed commits. See
[Git integration and recovery](docs/usage.md#optional-git-hook).
A CI job can run the same command against a clean checkout. Checks-only runs need
no AI credentials. Hannes does not provision Codex login on hosted runners, and
personal login credentials should not be copied into CI.

## Configuration

### Files and validation

Hannes reads the lowercase `.hannes/` directory in the Git working-tree root:

```text
.hannes/
  config.toml                 # Project settings and configuration version.
  provider.toml               # Codex model, timeout and allowed integrations.
  checks/*.toml               # Example layout; names and directories are freely chosen.
  reviews/*.toml              # Definitions use [[reviews]].
  reviews/*.md                # Criteria referenced by reviews.
  security/quality.toml       # Example nested rules and rule sets.
  reports/.gitignore          # Keeps generated reports local.
  reports/last.json           # Generated results, not configuration.
  README.md                   # Editable guidance, not parsed settings.
```

Settings use TOML. Each file below has its own example and field table.
“Required” means the field must be present. A default applies when an optional
field is omitted; generated files may explicitly choose a different value.

TOML files are discovered recursively in sorted order. Each document may contain
`[[checks]]`, `[[reviews]]` and `[[rule_sets]]` tables, including `config.toml`.
File names do not determine the content type. The central `config.toml` is read
once; `provider.toml` is reserved for provider settings. Hidden entries and
`reports`, `cache`, `caches` directories are excluded. Symlinks and special files
are rejected, including internal links; directory nesting is bounded.
Unknown fields, duplicate keys/IDs and invalid references are errors, including
in disabled definitions. Errors identify the source document and affected entity
or schema field. Required nonblank text fields contain no NUL; files use UTF-8.

Referenced paths must exist, be relative to their documented base, and resolve
inside the project. Absolute paths and parent traversal (`..`) are rejected.
Review evidence must also be included in the verified snapshot; ignored files
outside it cannot serve as evidence.

After editing, run `hannes pass auf`. This validates settings, files and selected
Markdown headings without executing rules. Provider availability is checked only
when reviews execute. Declared external state is then reconciled after confirmation.

For existing projects using `.Hannes`, rename it to `.hannes` and update path
references in scripts, criteria and ignore rules. On case-insensitive filesystems,
rename via an unused temporary name. Keep configuration and reports; run verification
again to refresh saved paths.

### Project settings

`.hannes/config.toml`:

```toml
version = 3
project = "My_App"
max_display_files = 5

[execution_points]
custom = ["check", "release"]
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `version` | Required | Exactly `3`. Earlier profile configurations require manual migration. |
| `project` | Required | Nonblank display name. Initialization uses the project directory name. |
| `max_display_files` | Required | Integer `1..5`: maximum changed paths shown in status, not a verification limit. |
| `execution_points` | Optional table | Declares additional execution contexts. |
| `execution_points.custom` | Optional; `[]` | Unique custom names: 1–80 lowercase ASCII letters, digits, `-`, `_`, starting with a letter/digit. Built-in names are reserved. |
| `hooks` | Optional; `null` (omitted in TOML) | Desired external integrations. No mappings by default. |
| `hooks.git` | Optional; `{}` | Git event-to-execution-point table; keys are `pre-commit`, `commit-msg`, `pre-push`. Values must be known execution points. |

### Fields shared by checks and reviews

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Case-sensitive ID, unique across checks and reviews: 1–80 ASCII letters, digits, `-` or `_`. Prefix `hannes-` is reserved. |
| `description` | Required | Nonblank explanation shown in progress output. |
| `enabled` | Required | `true` to make the rule available for execution; `false` to validate it without running it. |
| `tags` | Optional; `[]` | Descriptive metadata: up to 32 unique labels. Each uses 1–32 lowercase ASCII letters, digits, `-` or `_`, starting with a letter/digit. |
| `run_at` | Optional; `[]` | Known execution points. Required effectively for an active rule; active rule sets override this list. |

A **Rule** is either a deterministic check or an AI review. A **Rule Set** names
a group of rule IDs. An **Execution Point** names an independent execution context.
`pre-commit`, `commit-msg` and `pre-push` are always available and cannot be
redeclared. Other names must appear in `execution_points.custom`.


### Add a check

For a Rust application, create `.hannes/checks/unit-tests.toml`:

```toml
[[checks]]
id = "unit-tests"
run_at = ["pre-push", "check"]
description = "Run unit and integration tests"
enabled = true
command = "cargo test --locked"
working_directory = "."
timeout_seconds = 120
```

Use commands appropriate to your application. Separate tests, formatting and
compilation into individual checks when you want individual progress and results.
Validate with `hannes pass auf`, then execute with `hannes mach was --geiz-ist-geil`.

In addition to the [shared fields](#fields-shared-by-checks-and-reviews):

| Field | Required / default | Meaning |
| --- | --- | --- |
| `command` | Required | String parsed into `1..256` elements: executable first, then arguments. Each parsed element is at most 32768 bytes and contains no NUL; the executable must be nonblank. Arrays are rejected. |
| `working_directory` | Optional; `"."` | Existing working directory relative to the project root. |
| `timeout_seconds` | Required | Integer `1..3600`: this check's time limit in seconds. |

Hannes executes exactly one program with arguments, directly without a shell.
Whitespace separates arguments; single/double quotes keep spaces within one
argument. Backslash escapes whitespace or quotes outside quotes. Inside single
quotes everything is literal; inside double quotes backslash escapes `"`, `\`,
`$` and backticks. Quoted empty arguments are preserved. These command-string
rules apply after TOML decoding: TOML literal strings (`'…'`) preserve backslashes;
TOML basic strings (`"…"`) require their own escaping.

| Command configuration | Supported? |
| --- | --- |
| `command = "cargo test --locked"` | Yes: executable and two arguments. |
| `command = "npm run test"` | Yes: use the project's task runner. |
| `command = "git log -1 --format='%h %s'"` | Yes: the format stays one argument. |
| `command = 'git commit -m "Fix broken tests"'` | Yes: quoted argument (but checks should avoid changing Git state). |
| `command = 'my-tool --path "directory with spaces/file.txt"'` | Yes: quoted path; executable paths can be quoted too. |
| `command = 'my-tool --message "Hello; world"'` | Yes: the semicolon is literal. |
| `command = 'my-tool directory\ with\ spaces/file.txt'` | Yes: escaped spaces. |
| `cargo test && cargo clippy`, `test \|\| fallback`, `test; next` | No: command composition. |
| `test \| tee out`, `test > out`, `test < in`, `test &` | No: pipes, redirection or background execution. |
| `echo $HOME`, `echo $(date)`, `` echo `date` ``, `ls *.rs` | No: expansions or substitutions. |

`hannes pass auf` rejects these shell constructs, unmatched quotes, trailing
escapes, empty commands and missing executable names, including in disabled checks. Unquoted
`?`/`[ab]` globs, shell comments and tilde expansion are also rejected. Quote or
escape special characters intended as literal arguments. Single quotes protect
`$HOME` and substitutions; double quotes alone do not.
To combine commands, use an npm script, Gradle task, Makefile or dedicated script.
An explicitly invoked shell is possible, but Hannes supplies no shell semantics.

Commands inherit the environment and receive no stdin. Bare executable names use
PATH; relative executables such as `./scripts/check.sh` resolve from
`working_directory` and must remain inside the project. Absolute executable paths
and parent traversal are rejected. Validation checks syntax and path containment,
not executable availability or permissions; a missing or non-executable program
is a runtime ERROR.

Exit 0 becomes PASS; a nonzero exit or signal becomes FAIL. Timeout, startup or
output-capture problems become ERROR. Checks are trusted local commands and are
not sandboxed: they can access files and the network. Avoid commands that modify
the files being verified, because changes during a run prevent overall PASS.

### Configure the AI provider

The **provider** runs the AI requests. Hannes currently supports only **Codex CLI**
and uses its existing local login. Install and authenticate Codex before enabling
a review. In the same terminal or environment that will run Hannes, check:

```sh
command -v codex
codex --version
codex login
```

Hannes starts the executable named `codex` through its inherited PATH, from the
project root. Shell aliases, functions and startup files are not loaded. There
is no configurable executable path. IDEs and Git hooks must also have the Codex
installation directory on PATH.

Generated `.hannes/provider.toml`:

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
| `timeout_seconds` | Required | Integer `1..3600`: total seconds for MCP discovery and the model call. Discovery itself is capped at 10 seconds. |
| `model` | Optional; Codex default | Model identifier passed as `--model`. Generated files select GPT-6.1 Sol (`gpt-6.1-sol`); alternatives must be supported by your CLI/account. |
| `mcp_servers` | Optional; `[]` | Names of Codex MCP servers allowed for reviews. Empty or omitted disables all of them. See below. |
| `review_profile` | Optional table | Settings for the shared AI call. These settings apply to all reviews. |
| `review_profile.reasoning_effort` | Optional; Codex default | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` or `ultra`. Actual support depends on the model/CLI. |
| `review_profile.tool_output_token_limit` | Optional; `2000` | Integer `256..16384`: tokens retained per tool output in Codex history. This is not a limit on total tokens or cost. |

Omitting `model` or `reasoning_effort` uses the corresponding Codex default.
Unsupported models or settings produce ERROR; Hannes does not silently substitute
another model. The configuration applies to all selected reviews in their shared call.

**MCP integrations.** MCP (Model Context Protocol) servers give Codex access to
additional tools or data, such as a project knowledge graph. Define server commands,
URLs, transports and credentials in Codex configuration. Hannes only chooses which
already configured servers a review call may use. To allow two, replace the
top-level empty list in `provider.toml` with:

```toml
mcp_servers = ["project-memory", "browser"]
```

Use actual configured names: at most 32 unique names, each 1–256 ASCII letters,
digits, `_` or `-`; wildcards are not supported. All selected reviews receive the
same list. There is currently no per-review MCP selection.

Before the AI call, Hannes reads `codex mcp list --json` without starting a model
or server. It disables all unlisted servers for the child process. An unknown
allowed name, failed discovery, more than 256 discovered servers or unsafe server
names in that listing blocks the review. An empty list never inherits all of
your personal integrations.

**Provider execution.** Hannes uses noninteractive, ephemeral Codex sessions with
structured output, a read-only sandbox and no approvals. Automatic skill instructions,
bundled skills, skill search/installation, apps/plugins, subagents and web search
are disabled. Project AGENTS.md and ordinary Codex safety settings still apply.
Allowed MCPs are trusted capabilities; they are not an additional permission sandbox.

Hannes does not rewrite your Codex configuration. It removes `OPENAI_API_KEY` and
`CODEX_API_KEY` from Codex child processes; login-backed reviews still consume your
account allowance. Missing CLI/login, incompatible CLI options or exhausted allowance
produce ERROR. No credentials are copied to CI.

### Add a review

A review consists of a TOML definition and a Markdown file containing the
criterion. Decide what the AI should establish and where it can find the evidence.
“Compare documented CLI options with the implementation” is more useful than
“check code quality.” The generated `product-consistency` review stays disabled
until you customize and enable it.

For example, if `My_App` is a Rust CLI with `src/main.rs` and a `Usage` section
in its README, create `.hannes/reviews/cli-help.toml`:

```toml
[[reviews]]
id = "cli-help"
run_at = ["pre-push"]
description = "Compare documented commands with their implementation"
enabled = true
instructions = "cli-help.md"
files = ["src/main.rs"]

[[reviews.context]]
path = "README.md"
headings = ["Usage"]
```

Create `.hannes/reviews/cli-help.md`:

```markdown
Compare the commands and options documented in the supplied README Usage section
with their implementation. Start with the listed source file and follow relevant
dependencies. Report FAIL for a concrete contradiction, ERROR when essential
evidence is unavailable, and PASS only with supporting file and line references.
```

Replace the paths, heading and criterion with ones that exist in your project.
After [provider setup](#configure-the-ai-provider), run `hannes pass auf`, then
`hannes mach was`. All selected reviews use one AI call, after checks pass.

In addition to the [shared fields](#fields-shared-by-checks-and-reviews):

| Field | Required / default | Meaning |
| --- | --- | --- |
| `instructions` | Required | Nonblank UTF-8 `.md` criteria file relative to its defining TOML document. It must stay inside `.hannes/`. |
| `files` | Optional; `[]` | Up to 64 unique existing source files relative to the project root. Exact paths, not globs. These are starting points for inspection. |
| `context` | Optional; `[]` | Up to 32 document selections, written as `[[reviews.context]]` entries. They supply requirements or other background to the AI. |
| `context.path` | Required per entry | Existing UTF-8 file relative to the project root. |
| `context.headings` | Optional; `[]` | Up to 32 nonblank exact Markdown heading titles. Empty or omitted includes the whole document. |

`instructions` says **what to judge**; `files` says **where to start reading code**;
`context` supplies **the documented expectations**. Entry files do not restrict
further dependency reads. Criteria do not change provider permissions or the
required response format.

Heading titles match ATX Markdown headings (`#` through `######`) without the
hash prefix. Each selection includes its subsections until the next heading of
equal or higher level. Headings in code blocks are ignored; missing or ambiguous
titles are errors. Shared or overlapping selections are sent once with their
original line numbers. Across all definitions, including disabled reviews, context
is limited to 32 distinct files, 1 MiB per file and 64 KiB of selected text.
Missing or excessive context is rejected, not silently dropped.

When every selected review declares `files`, the initial file list and diff hints
focus on the combined entry/context paths. Otherwise these hints cover the project.
The AI still evaluates the current working files, not only changed lines.
Untracked content needs direct inspection. The snapshot always covers the full
relevant working tree, regardless of the hints supplied to the AI.

Hannes asks for exactly one PASS/FAIL/ERROR per requested review, a nonblank one-line
summary of at most 120 characters, and up to 20 detailed findings of at most 2000
bytes each. PASS/FAIL need a verified existing file and a positive line reference.
Missing, duplicate or malformed results make the batch ERROR. You write criteria,
not a custom output schema; Hannes sends these response constraints to the provider
and checks the returned result locally, including evidence and UTF-8 byte limits.

### Group rules into rule sets

Create a TOML document anywhere under `.hannes/`, for example
`.hannes/backend/quality.toml`, referencing the earlier `unit-tests` rule:

```toml
[[rule_sets]]
id = "quality"
enabled = true
rules = ["unit-tests"]
run_at = ["pre-push", "check"]
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Unique rule-set ID: 1–80 lowercase ASCII letters, digits, `-` or `_`, starting with a letter/digit. Independent of rule IDs. |
| `enabled` | Required | `false` disables this set's assignments. |
| `rules` | Required | Unique existing rule IDs. Other rule sets cannot be referenced. An empty list is allowed. |
| `run_at` | Optional; `[]` | Known execution points; must be nonempty for an active set. |

An active rule with no active set membership uses its own `run_at`. Membership
in one or more active sets replaces the rule's own list with the union of those
sets' lists. Hannes reports overridden own assignments as a hint. A rule runs
once per invocation, even when multiple sets assign it to the same point.

Disabled sets contribute nothing. A rule referenced only by disabled sets needs
its own `run_at` when active. A disabled rule never runs and needs no assignment,
even when included in active sets. All references are still validated: typos such
as `chekc`, duplicate declarations/references and missing rule IDs are errors.

```sh
hannes pass auf
hannes mach was --at pre-commit
hannes mach was --at commit-msg
hannes mach was --at pre-push
hannes mach was --at check
hannes mach was --at release --plan --json
```

Without `--at`, Hannes selects `pre-push`. Each selected rule is mandatory;
FAIL, ERROR or NOT_RUN prevents success. An empty execution point succeeds
without establishing any rule PASS. Checks run before reviews, each ordered by
ID. `--geiz-ist-geil` is accepted only when the point requires no reviews.
`--plan` prepares the same selection offline without execution or reconciliation.

External rule sets from Git repositories are planned for phase 2. There are no
remote downloads, imports or local overrides of external rules in phase 1.
See [execution and plan details](docs/profiles-and-plans.md).

### Reports and other files

`.hannes/reports/last.json` is generated atomically and replaced by each run that
can produce a report. It contains verdicts, short summaries, detailed findings,
captured check output, available token usage and input identity. Invalid
configuration preserves the previous report. Delete the report to clear saved
status; use the status commands to read it rather than editing its contents.

Schema 3 records the execution point, plan, source documents and effective
assignments. Status displays the saved selection. Failed checks leave reviews
NOT_RUN. Historical schema-1 and schema-2 reports remain readable as saved evidence.

Check stdout/stderr are each captured up to 64 KiB, with byte totals and truncation
recorded. Full findings and retained output appear with `--details`. Output is
not automatically redacted; avoid printing secrets from check commands. Status
cannot calculate line deltas from its hash-only baseline.

Keep reports local with `.hannes/reports/.gitignore`:

```gitignore
*
!.gitignore
```

Commit configuration, rule definitions and criteria; exclude reports and credentials.
`.hannes/README.md` is editable guidance and has no parsed settings. Extra files
have no implicit execution behavior; a review must reference its criteria Markdown.

## Limits and unsupported features

These limits are built in, not additional configuration fields:

| Area | Limit or behavior |
| --- | --- |
| Configuration metadata | 64 KiB per TOML file; up to 256 configuration documents, 256 rules and 256 rule sets. |
| Review criteria | 32 KiB per instructions file; 256 KiB combined, including disabled reviews. |
| Initial AI hints | Up to 200 file-list entries and 32 KiB of diff, with truncation indicated. Necessary evidence still needs inspection. |
| Snapshot | 8 MiB per file, 128 MiB total, 100,000 paths; Git file enumeration capped at 64 KiB; `.hannes` nesting limited to 16 levels. |
| Snapshot coverage | Tracked and nonignored untracked files, plus `.hannes` even if ignored. `.git` and `.hannes/reports` are excluded. Unsupported symlinks, submodules, paths or exceeded limits prevent PASS. |
| Input stability | Working files and HEAD must match before/after execution and before the AI call. Staging and pushed commits are not separately verified. |
| AI response | 1 MiB final JSON document; summaries and findings have the limits described under reviews. |
| Saved report input | 128 MiB maximum when reading a saved report. |
| Token usage | Available counters are saved once per shared call. Optional reasoning/cache-write counters are subsets, not additions to the total. Missing telemetry remains unavailable. |

Not currently supported: other AI providers, per-review model/MCP settings,
reviews restricted to a diff, staged/index or commit input, configurable run-wide
token budgets, monetary cost estimates and cached results. A token-efficient
shared call is not a result cache or a guaranteed prompt-cache hit. Planned work
is listed in the [TODOs](docs/todo/README.md).

## Development

Pending phase 3, this repository's deterministic quality gates run through an installed published
Hannes release. Install development tools with `mise install`, dependencies with
`npm ci --ignore-scripts` and `cargo +stable fetch --locked`, then the governor
with `mise deploy`. Ensure its `bin/` directory is on PATH.

`mise check` uses the pinned published governor and its `pre-push` profile:
formatting, documentation validation, Clippy, Rust tests, release/deployment/hook
regression tests, an optimized candidate build and smoke test. No reviews or real
provider calls run. The governor never comes from the candidate build.

`mise docs:check` checks links, specification references, configuration fields and
executable documentation examples. Individual tasks remain available: `format`,
`format:check`, `lint`, `test`, `smoke`, `hooks:test`. Install the optional
pre-commit, commit-msg and pre-push hooks explicitly with `mise hooks:install`.
Local hooks verify working files; CI checks clean checkouts.

See [repository quality gates](docs/self-governance.md) for setup, every profile,
the Renovate-managed CI version pin, reports and hook limitations. Accepted commit
types, scopes and release effects are in [Contributing](CONTRIBUTING.md#commits).
`mise run TASK` works for every task; `mise format` formats Rust, whereas the
built-in `mise fmt` formats `mise.toml`.

`mise release` uses an authenticated `gh` CLI to start the release workflow for
**remote main**, then returns. It does not commit/push local work or wait for
publication. Hannes starts at 0.1.0; features and breaking changes increment minor
during 0.x, fixes and maintenance increment patch. Semantic Release handles
subsequent versions, changelog and native archives;
see [Releasing](docs/releasing.md).

[Usage and Git hooks](docs/usage.md) · [Design and limits](docs/design.md) ·
[macOS application example](examples/my-app/README.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Unlicense](LICENSE)

## Frequently Asked Questions

### What is the difference between Hannes and a harness?

The R.

### Is Hannes an artificial intelligence?

No. There is enough artificial intelligence already. Hannes is artificial emotionality.

### Is Hannes available for Windows?

I don't understand the question.
