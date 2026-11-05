# Hannes

```text
  /\  /\__ _ _ __  _ __   ___  ___
 / /_/ / _` | '_ \| '_ \ / _ \/ __|
/ __  / (_| | | | | | | |  __/\__ \
\/ /_/ \__,_|_| |_|_| |_|\___||___/

------ Der Harness ohne 'r' ------
```

*Hannes schaut nach dem Rechten.* A small Rust CLI for agent-agnostic project
verification: deterministic checks first, optional Codex reviews afterwards,
and an auditable PASS/FAIL/ERROR report.

## Build locally

You need **Git**, **stable Rust with Cargo** (edition 2024; Rust 1.85+), and a
native compiler/linker: Xcode Command Line Tools on macOS, GCC/Clang on Linux,
or Visual Studio C++ Build Tools on Windows.

```sh
cargo build --release --locked
./target/release/hannes --help
```

The executable is `target/release/hannes` (`hannes.exe` on Windows).
To install it on your Cargo PATH: `cargo install --path . --locked`.

Optional [mise](https://mise.jdx.dev/) setup installs Rust with rustfmt/Clippy and
Python 3.13 for developer scripts:

```sh
mise trust
mise install
mise run build
mise run install
mise run check
```

`mise install` prepares the tooling; `mise run install` builds and installs Hannes
to `~/.local/bin` on macOS/Linux, replacing an existing binary without prompting.
Add `export PATH="$HOME/.local/bin:$PATH"` to `~/.zshrc` if needed.
Override the prefix with `CARGO_INSTALL_ROOT=/your/prefix mise run install`
(the executable goes into its `bin/` directory). Windows uses Cargo's install root.

Python is only needed for smoke tests and release scripts, not for the CLI.
Codex CLI with an existing `codex login` is only needed for enabled LLM reviews.

## Use

Run from the root of a Git repository:

```sh
hannes fang an            # Initialize .Hannes/ without overwriting files
hannes pass auf           # Validate configuration; execute nothing
hannes mach mal           # Run checks and reviews; write the report
hannes mach was --geiz-ist-geil # Checks only; no LLM call
hannes was ist phase      # Show verdicts, short AI summaries and freshness
hannes was ist phase --details # Inspect saved findings without rerunning reviews
hannes so hilf mir doch   # Help; --help and --version also work
```

The generated example passes without Codex. `mach was` is an alias of `mach mal`.
Checks run sequentially before reviews; progress goes to stderr, the compact
report to stdout. Failed checks prevent the model call and leave reviews
`NOT_RUN`. `--geiz-ist-geil` runs checks only, with no provider discovery or model
call; the report is explicitly checks-only. Full configuration validation still
applies. `--no-color` or `NO_COLOR` disables colors; redirected output is uncolored.

Install the local commit-message check after cloning with
`sh scripts/setup-git-hooks.sh`. Commit types follow Angular's allow list;
see [Contributing](CONTRIBUTING.md#commits).

## Configuration: `.Hannes/`

Run from the Git working-tree root. The directory name is exactly `.Hannes`,
including the capital H. `hannes fang an` creates missing scaffold files and
preserves existing ones. All supported configuration is **version 1**:

```text
.Hannes/
  config.toml
  provider.toml
  checks/
    always-ok.toml
    deliberately-failing.toml
  reviews/
    product-consistency.toml
    product-consistency.md
  reports/
    .gitignore
    last.json                     # Generated; not configuration
  README.md                       # Editable documentation; not configuration
```

Add/remove rule TOML files directly in `checks/` or `reviews/`; filenames need not
match IDs. Only immediate files with the `.toml` extension are loaded as rules.
Checks are deterministic commands; reviews are LLM evaluations; both are **rules**.
Enabled checks run in ID order, then enabled reviews in ID order. Disabled rules
are omitted from execution/results but still fully validated.

TOML uses strict field names/types: unknown fields, duplicate keys and duplicate
rule IDs are errors. Text files must be UTF-8. IDs are case-sensitive, unique
across both rule kinds, and contain 1–80 ASCII letters, digits, `-` or `_`; prefix
`hannes-` is reserved. Descriptions and other required text must be nonblank and
contain no NUL. File/directory paths must exist, be relative to their documented
base, and resolve inside the project; absolute paths and `..` are rejected.

Validate edits with `hannes pass auf`: it reads configuration and referenced
files but runs no checks, provider CLI or model. It does not verify that a model
is available or that MCP names exist in Codex; those checks happen on execution.

### Project: `config.toml`

```toml
version = 1
project = "Passing By"
max_display_files = 5
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `version` | Required | Exactly `1`. |
| `project` | Required | Nonblank display name; initialization uses the directory name. |
| `max_display_files` | Required | Integer `1..5`; maximum changed paths shown in status, not a rule or verification limit. |

### Provider: `provider.toml`

Only **Codex CLI** is supported. These are the generated settings:

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
| `timeout_seconds` | Required | Integer `1..3600`; shared deadline for MCP configuration discovery and the model invocation. Discovery itself is limited to at most 10 seconds. |
| `model` | Optional; Codex default if omitted | Nonblank model identifier passed as `--model`. New scaffolds explicitly select GPT-6.1 Sol (`gpt-6.1-sol`); other identifiers must be supported by your CLI/account. |
| `mcp_servers` | Optional; `[]` | Allow list of MCP server names already defined in Codex. Empty disables all discovered servers. |
| `review_profile` | Optional table | Provider execution settings below; not a rule-selection profile. |
| `review_profile.reasoning_effort` | Optional; Codex default if omitted | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, or `ultra`. Actual availability depends on model/CLI. |
| `review_profile.tool_output_token_limit` | Optional; `2000` | Integer `256..16384`; retained tokens per tool output, not a total token/spending cap. |

Unsupported models/settings fail rather than silently switching models. Model,
effort, timeout and MCP settings apply to the single shared call for **all enabled
reviews**; per-review overrides are not supported.

#### Finding Codex and authentication

Hannes starts the executable named `codex` using the **PATH inherited by Hannes**,
with the project root as working directory. It does not open an interactive shell
or load shell startup files; aliases/functions do not supply an executable. There
is no configurable Codex binary path in `.Hannes`. Put the installed CLI's binary
directory on PATH before starting Hannes, including in an IDE, hook or CI runner.

From the same environment, check:

```sh
command -v codex                 # POSIX shells; Windows: where.exe codex
codex --version
codex login
```

Use an existing local Codex login; Hannes neither copies credentials nor performs
login itself. Child processes remove `OPENAI_API_KEY` and `CODEX_API_KEY` to avoid
accidentally using inherited API-key billing. Login-backed reviews can still
consume account allowance. Your CLI must support the invocation used by Hannes:
noninteractive `exec`, structured JSON output/schema, ephemeral sessions,
`read-only` sandbox, `never` approvals, and MCP configuration listing/overrides.
Missing executables, incompatible CLI/authentication or exhausted allowance yield
ERROR. Checks-only execution and disabled example reviews need no Codex process.

#### MCP access

To permit servers already configured in Codex, replace the empty list, for example:

```toml
mcp_servers = ["project-memory", "browser"]
```

This is a replacement field in `provider.toml`, not an additional table. Hannes
accepts at most 32 unique allowed names, each 1–256 ASCII letters, digits, `_` or
`-`. There is no wildcard. Server commands, URLs, transports and credentials
cannot be defined in `.Hannes`; configure them in Codex first.

Before the model call, Hannes reads `codex mcp list --json` without starting a
model or MCP server. It explicitly enables only listed servers and disables all
other discovered servers for the child process. Unknown allowed names, failed
discovery, more than 256 discovered servers or unsafe names in the discovered
list block the review. An omitted/empty allow list does not inherit all personal
MCPs. Every enabled review gets the same allowed set; per-review permissions and
separate MCP sessions are not implemented.

Review child processes disable automatic skill instructions, bundled skills,
skill search/automatic MCP installation, apps/plugins, subagents and web search.
Existing Codex authentication, project AGENTS.md and ordinary safety settings
remain in effect. Hannes does not rewrite the user's Codex configuration. Enabled
MCP servers are trusted capabilities, not an additional permission sandbox.

### Checks: `checks/*.toml`

```toml
id = "unit-tests"
description = "Run Rust unit and integration tests"
enabled = true
command = ["cargo", "test", "--locked"]
working_directory = "."
timeout_seconds = 120
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Unique rule ID under the rules above. |
| `description` | Required | Short, nonblank description shown in progress output. |
| `enabled` | Required | Boolean; `false` disables execution, not validation. |
| `command` | Required | Array of `1..256` argv strings; first element is the nonblank executable. Each string is at most 32768 bytes and contains no NUL. |
| `working_directory` | Optional; `"."` | Existing directory relative to the project root. |
| `timeout_seconds` | Required | Integer `1..3600` for this check. |

Commands run directly without a shell or stdin. Bare executable names resolve
through inherited PATH. Executable paths such as `./scripts/check.sh` resolve
relative to `working_directory`, must be files inside the project, and must be
runnable on the host. Shell expansion, pipes, `&&` and `$VARIABLE` require an
explicit shell, e.g. `command = ["sh", "-c", "cargo test && cargo clippy"]`.
Checks inherit the environment; they are trusted commands, not sandboxed code.
If a check changes verified files, final stability checks prevent overall PASS.

Exit `0` means PASS; nonzero exit/signal means FAIL; timeout, spawn/capture errors
mean ERROR. Remaining checks still run after a failure. Hannes captures stdout
and stderr, keeping the first 64 KiB of each and recording truncation/byte totals;
raw output is available in the saved report and `--details`, not default output.
It is not automatically redacted, so avoid printing secrets.

### Reviews: `reviews/*.toml` and instruction Markdown

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

This example assumes the listed files and heading exist. The generated
`product-consistency` review starts disabled: customize its `.md` before enabling.

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id`, `description`, `enabled` | Required | Same common fields as checks. |
| `instructions` | Required | Path relative to `.Hannes/reviews/` to a nonblank UTF-8 `.md` file that stays inside that directory. |
| `files` | Optional; `[]` | Up to 64 unique existing files relative to the project root; exact paths, not globs. Starting points for source inspection. |
| `context` | Optional; `[]` | Up to 32 requirement-document selections, using `[[context]]` tables. |
| `context.path` | Required per entry | Existing UTF-8 file relative to the project root. |
| `context.headings` | Optional; `[]` | Up to 32 nonblank exact Markdown heading titles. Omitted/empty selects the whole file. |

Write review criteria in the referenced Markdown, for example:

```markdown
Check the keyboard accessibility requirements in the supplied Product.md section.
Inspect the listed source/tests and necessary dependencies. Report FAIL for a
concrete violation, ERROR if essential evidence is unavailable, and PASS only
with grounded evidence. Do not invent additional requirements.
```

Headings use ATX Markdown (`#` through `######`). Supply the title without `#`;
matching is exact and must identify one heading. Its section includes subsections
until the next heading of equal/higher level. Headings inside fenced/indented
code are ignored. Missing/ambiguous titles are errors, even in disabled reviews.
Overlapping/shared selections are merged and sent once with original line numbers.

The whole configuration permits 32 distinct context files, each at most 1 MiB,
and at most 64 KiB of selected text after merging, including disabled definitions
during validation. Empty context files and oversized selections are errors;
required context is never silently dropped.

All active reviews share **one** read-only model invocation and deduplicated
context. If every active review has entry `files`, initial manifest hints and the
bounded HEAD-relative orientation diff focus on the union of entry/context paths.
Otherwise initial orientation remains project-wide. The model may follow
necessary dependencies: entry files/sections do not restrict reads or define an
exhaustive review boundary. Untracked contents require direct inspection.

Reviews evaluate **current working files**, not just the diff. Instructions are
criteria, not a way to change provider permissions or output format. Hannes
requires exactly one structured PASS/FAIL/ERROR per requested ID, a nonblank
single-line summary of at most 120 characters, and at most 20 findings (each
message at most 2000 bytes). PASS/FAIL require at least one existing verified
file/positive line reference. Invalid/missing/duplicate results make the whole
review batch ERROR. LLM judgments remain fallible; evidence references are not
proof that the judgment is correct.

### Reports, documentation and fixed limits

`reports/last.json` is generated atomically, contains verdicts, detailed findings,
captured check output, snapshot identity and available usage, and is not user
configuration. `was ist phase` reads it without rerunning anything; `--details`
adds findings/output. Freshness compares current working files and HEAD against
the saved snapshot: CURRENT, STALE or UNKNOWN. Line deltas since verification are
unavailable with the hash-only baseline. Configuration errors preserve the prior
report; deleting it resets saved status, not the configuration.

Keep reports local with the editable `reports/.gitignore`:

```gitignore
*
!.gitignore
```

Commit project configuration, criteria and requirement documents; do not commit
reports or credentials. `.Hannes/README.md` is editable guidance and has no parsed
settings. Other documentation/assets inside `.Hannes` have no implicit execution
meaning; Markdown instructions run only when referenced by review metadata.
Patience/quote history is global state outside repositories, never a setting in
`.Hannes`; configuration does not customize faces, sayings or uninstall behavior.

These are built-in limits, not additional configurable fields:

| Area | Limit / behavior |
| --- | --- |
| Rule/config metadata | 64 KiB per TOML file; 256 rule TOML files per kind. |
| Review instructions | 32 KiB per Markdown file; 256 KiB combined across all reviews, including disabled ones. |
| Initial orientation | At most 200 manifest paths and 32 KiB of diff; truncation is indicated, necessary evidence still requires inspection. |
| Snapshot | 8 MiB per file, 128 MiB total, 100,000 paths; bounded Git enumeration can impose a lower practical limit. |
| Snapshot coverage | Tracked and nonignored untracked working files plus `.Hannes` even when ignored; `.git` and `.Hannes/reports` excluded. Unsupported symlinks/submodules/paths or exceeded limits block PASS. |
| Stability | Files must match before/after verification and before a review call; Hannes does not verify staged state or pushed commits separately. |
| Token reporting | Available input/cached-input/output counters once per model call; total = input + output. Missing telemetry stays unavailable. No money/quota estimate or enforced total token budget. |

Not available yet: rule tags, selection profiles, `--profil`, `--plan`, per-review
model/MCP settings, true change-only review scope, index/commit inputs, configurable
run-wide budgets or result caching. No hooks are installed by initialization.
See the [ordered TODOs](docs/todo/README.md) for planned work.

## More

[Usage and Git hook](docs/usage.md) · [Configuration](docs/configuration.md) ·
[Planned work](docs/todo/README.md) ·
[Design and limits](docs/design.md) · [Passing By example](examples/passing-by/README.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Releasing](docs/releasing.md)

Released under the [Unlicense](LICENSE).

## Frequently Asked Questions

### Was unterscheidet Hannes von einem Harness?

Das R.

### Ist Hannes eine künstliche Intelligenz?

Nein. Künstliche Intelligenz gibt es schon genug. Hannes ist eine künstliche Emotionalität.

### Gibt es Hannes auch für Windows?

Ich verstehe die Frage nicht.
