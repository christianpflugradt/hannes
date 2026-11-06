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
  - [Group rules into profiles](#group-rules-into-profiles)
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

Verify the archive against its checksum, then extract and install it. Replace
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

With [mise](https://mise.jdx.dev/), the equivalent setup is:

```sh
mise trust
mise install                  # Install development tools.
mise build                    # Build target/release/hannes.
mise deploy                   # Build and install to ~/.local/bin; replace existing binary.
```

Set `CARGO_INSTALL_ROOT=/your/prefix mise deploy` to change the install prefix;
the binary goes into its `bin/` directory. `mise install` installs the toolchain,
not Hannes itself. See [Development](#development) for checks and release tasks.

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

`pass auf` checks the configuration without executing anything. The first
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
| `hannes pass auf` | Validate every configuration file and referenced path; run no checks or reviews. |
| `hannes mach was` | Run the selected rules and save their results. |
| `hannes was ist phase` | Show the last saved report and whether it still matches the working files. |
| `hannes so hilf mir doch` | Show help. |

| Option | Used with | Meaning |
| --- | --- | --- |
| `--geiz-ist-geil` | `mach was` | Run only checks; do not call an AI provider. |
| `--profil NAME` | `mach was` | Select a named group of rules instead of the default. |
| `--plan` | `mach was` | Preview what would run without executing rules or calling the provider. |
| `--json` | `mach was --plan` | Print the plan or plan errors as JSON. |
| `--details` | `was ist phase` | Show full findings and captured check output. |
| `--no-color` | Any command | Disable colors. `NO_COLOR` and redirected output also disable colors. |
| `-h`, `--help` | Any command | Show help. |
| `-V`, `--version` | `hannes` | Show the version. |

Without a profile, all enabled checks run first, followed by all enabled reviews.
Within each kind, rules run in ID order. Disabled rules are validated but are not
executed or included in results. Selected reviews share one Codex invocation.

If neither checks nor reviews are defined, Hannes additionally prints
`Du hast noch keine Regeln definiert, du Vogel!`. A successful empty run does not
establish that your project passed any quality rules.

### Run checks without AI

```sh
hannes mach was --geiz-ist-geil
```

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
| `2` | Configuration, execution, provider or report error; unknown freshness; invalid arguments; blocked/invalid plan. |
| `3` | No saved report exists for the status command. |

### Choose rules and preview a run

A **profile** is a named selection of rules, such as a quick local selection and
a stricter release selection. Profiles and rule **tags** (labels used for selection)
require configuration version 2. They do not install Git hooks or change provider
permissions. See [profile configuration](#group-rules-into-profiles).

```sh
hannes mach was --profil local --plan
hannes mach was --profil local --plan --json
hannes mach was --profil local
```

A plan shows selected rules, order, selection reasons, omissions, required rules,
model/MCP settings and bounded context/diff sizes. It runs no checks, provider CLI,
MCP servers or network calls, and preserves the saved report. It cannot confirm
model availability, login or inherited Codex defaults. JSON plans use schema 1.

Each actual run builds a fresh plan. A matching plan ID is neither a cached result
nor proof that a later working tree is unchanged.

### Use a Git hook or CI

The optional [pre-push example](scripts/pre-push) runs `hannes mach was` and blocks
a push on failure. Install it manually, preserving any existing hook; initialization
never installs it. Local hooks are bypassable and verify the working files, which
may differ from the pushed commit. See [Git integration](docs/usage.md#optional-git-hook).

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
  checks/*.toml               # One check per file.
  reviews/*.toml              # One review per file.
  reviews/*.md                # Criteria referenced by reviews.
  profiles/*.toml             # Optional rule selections; version 2 only.
  reports/.gitignore          # Keeps generated reports local.
  reports/last.json           # Generated results, not configuration.
  README.md                   # Editable guidance, not parsed settings.
```

Settings use TOML. Each file below has its own example and field table.
“Required” means the field must be present. A default applies when an optional
field is omitted; generated files may explicitly choose a different value.

Only immediate `.toml` files in rule/profile directories are loaded; subdirectories
are not searched. Filenames need not match IDs. Unknown fields, duplicate TOML
keys/IDs and invalid values are errors, including in disabled rules and unused
profiles. Required nonblank text fields must not contain NUL; text files use UTF-8.

Referenced paths must exist, be relative to their documented base, and resolve
inside the project. Absolute paths and parent traversal (`..`) are rejected.
Review evidence must also be included in the verified snapshot; ignored files
outside it cannot serve as evidence.

After editing, run `hannes pass auf`. This validates settings, files and selected
Markdown headings without executing rules. Provider availability is checked only
when reviews execute.

For existing projects using `.Hannes`, rename it to `.hannes` and update path
references in scripts, criteria and ignore rules. On case-insensitive filesystems,
rename via an unused temporary name. Keep configuration and reports; run verification
again to refresh saved paths.

### Project settings

`.hannes/config.toml`:

```toml
version = 1
project = "My_App"
max_display_files = 5
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `version` | Required | `1` for ordinary checks/reviews, or `2` to enable tags/profiles. No automatic migration. |
| `project` | Required | Nonblank display name. Initialization uses the project directory name. |
| `max_display_files` | Required | Integer `1..5`: maximum changed paths shown in status, not a verification limit. |
| `default_profile` | Optional; `"all"` | Version 2 only: profile selected unless `--profil` overrides it. |

### Fields shared by checks and reviews

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Case-sensitive ID, unique across checks and reviews: 1–80 ASCII letters, digits, `-` or `_`. Prefix `hannes-` is reserved. |
| `description` | Required | Nonblank explanation shown in progress output. |
| `enabled` | Required | `true` to make the rule available for execution; `false` to validate it without running it. |
| `tags` | Optional; `[]` | Version 2 only: up to 32 unique labels for profile selection. Each uses 1–32 lowercase ASCII letters, digits, `-` or `_`, starting with a letter/digit. |

Version 1 rejects `tags`, even an empty list. Enabling a rule does not force it
into every profile; the selected profile still decides which enabled rules run.

### Add a check

For a Rust application, create `.hannes/checks/unit-tests.toml`:

```toml
id = "unit-tests"
description = "Run unit and integration tests"
enabled = true
command = ["cargo", "test", "--locked"]
working_directory = "."
timeout_seconds = 120
```

Use commands appropriate to your application. Separate tests, formatting and
compilation into individual checks when you want individual progress and results.
Validate with `hannes pass auf`, then execute with `hannes mach was --geiz-ist-geil`.

In addition to the [shared fields](#fields-shared-by-checks-and-reviews):

| Field | Required / default | Meaning |
| --- | --- | --- |
| `command` | Required | List of `1..256` strings: executable first, then its arguments. Each string is at most 32768 bytes and contains no NUL; the executable must be nonblank. |
| `working_directory` | Optional; `"."` | Existing working directory relative to the project root. |
| `timeout_seconds` | Required | Integer `1..3600`: this check's time limit in seconds. |

For example, `["cargo", "test", "--locked"]` runs `cargo test --locked` directly.
There is no shell expansion: pipes, `&&` and environment-variable expansion need
an explicit shell, such as `["sh", "-c", "cargo test && cargo clippy"]`.
Commands inherit the environment and receive no stdin. Bare executable names use
PATH; relative executables such as `./scripts/check.sh` resolve from
`working_directory` and must be runnable files inside the project.

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
| `review_profile` | Optional table | Settings for the shared AI call. This is separate from a rule-selection profile. |
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
id = "cli-help"
description = "Compare documented commands with their implementation"
enabled = true
instructions = "cli-help.md"
files = ["src/main.rs"]

[[context]]
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
| `instructions` | Required | Nonblank UTF-8 `.md` criteria file relative to `.hannes/reviews/`. It must stay inside that directory. |
| `files` | Optional; `[]` | Up to 64 unique existing source files relative to the project root. Exact paths, not globs. These are starting points for inspection. |
| `context` | Optional; `[]` | Up to 32 document selections, written as `[[context]]` entries. They supply requirements or other background to the AI. |
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
not a custom output schema; Hannes handles the structured response.

### Group rules into profiles

Use profiles when different runs need different rule selections, for example a
small local selection and a mandatory release selection. To enable profiles and
tags, set `version = 2` in `config.toml`, keeping its other required fields.
Optionally add `default_profile = "local"`.

To select the check from the earlier example, create `.hannes/profiles/local.toml`:

```toml
id = "local"
rules = ["unit-tests"]
include_tags = []
gate = false
input = "worktree"
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Unique profile ID: 1–32 lowercase ASCII letters, digits, `-` or `_`, starting with a letter/digit. `all` is reserved. |
| `rules` | Optional; `[]` | Unique IDs of existing checks or reviews to select. |
| `include_tags` | Optional; `[]` | Unique tags that occur on at least one defined rule. Selects rules matching any listed tag. |
| `gate` | Optional; `false` | Treat all selected rules as mandatory. Reject empty selections and disabled required rules. |
| `input` | Optional; `"worktree"` | Verify the current working files. Other inputs, including `index` and `commit`, are rejected. |

Explicit IDs and tag matches are combined without duplicates. To select by tag,
add `tags = ["local"]` to the relevant version-2 rule definitions, then use
`include_tags = ["local"]` in the profile. Unknown selectors and duplicates are
errors, even in unused profiles.

`--profil NAME` overrides `default_profile`. With neither, built-in `all` selects
all enabled rules. Version 1 rejects profile TOML files, `default_profile` and
rule tags. Profiles have no inheritance, exclusions or implicit Git-hook behavior.

A **gate** makes omissions an error: it cannot select disabled rules, be empty,
or use `--geiz-ist-geil` if reviews are required. A checks-only gate can use that
flag. Ordinary profiles omit disabled rules and permit checks-only runs with
reviews omitted. Actual failures block success in either mode; a gate is about
requiring coverage, not making failures count differently. A working-tree gate
does not prove that staged or pushed code was verified.

An empty ordinary profile explicitly reports that no rules ran. Plans and schema-2
reports record selections and omissions. See [profile and plan details](docs/profiles-and-plans.md).

### Reports and other files

`.hannes/reports/last.json` is generated atomically and replaced by each run that
can produce a report. It contains verdicts, short summaries, detailed findings,
captured check output, available token usage and input identity. Invalid
configuration preserves the previous report. Delete the report to clear saved
status; use the status commands to read it rather than editing its contents.

Schema 2 also records the profile/plan and omissions. Status displays the saved
selection, not today's default profile. Selection does not mean every rule ran:
failed checks still leave reviews NOT_RUN. Older schema-1 reports remain readable
as unprofiled evidence, but cannot certify a profile gate.

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
| Configuration metadata | 64 KiB per TOML file; up to 256 checks, 256 reviews and 256 profiles. |
| Review criteria | 32 KiB per instructions file; 256 KiB combined, including disabled reviews. |
| Initial AI hints | Up to 200 file-list entries and 32 KiB of diff, with truncation indicated. Necessary evidence still needs inspection. |
| Snapshot | 8 MiB per file, 128 MiB total, 100,000 paths; Git file enumeration capped at 64 KiB; `.hannes` nesting limited to 16 levels. |
| Snapshot coverage | Tracked and nonignored untracked files, plus `.hannes` even if ignored. `.git` and `.hannes/reports` are excluded. Unsupported symlinks, submodules, paths or exceeded limits prevent PASS. |
| Input stability | Working files and HEAD must match before/after execution and before the AI call. Staging and pushed commits are not separately verified. |
| Token usage | Available counters are saved once per shared call. Optional reasoning/cache-write counters are subsets, not additions to the total. Missing telemetry remains unavailable. |

Not currently supported: other AI providers, per-review model/MCP settings,
reviews restricted to a diff, staged/index or commit input, configurable run-wide
token budgets, monetary cost estimates and cached results. A token-efficient
shared call is not a result cache or a guaranteed prompt-cache hit. Planned work
is listed in the [TODOs](docs/todo/README.md).

## Development

With mise, `mise check` runs hook-policy tests, formatting checks, Clippy, Rust
tests, an optimized build and a CLI smoke test. Tests use fake providers, not
real Codex calls. The toolchain includes Rust with rustfmt/Clippy, Python 3.13
for scripts and Node 24 for release tooling.

`mise docs:check` checks documentation links, specification test references, all
configuration fields/defaults and executable README examples without AI calls.
These checks also run in `mise check` and CI.

Other tasks: `format`, `format:check`, `lint`, `test`, `smoke`, `hooks:install`,
`hooks:test`. `mise format` formats Rust; built-in `mise fmt` formats `mise.toml`.
`mise run TASK` works for every project task. Install this repository's optional
commit-message hook with `mise hooks:install`; accepted types, scopes and release
effects are in [Contributing](CONTRIBUTING.md#commits).

`mise release` uses an authenticated `gh` CLI to start the release workflow for
**remote main**, then returns. It does not commit/push local work or wait for
publication. Semantic Release handles versions, changelog and native archives;
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
