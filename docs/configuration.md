# Hannes configuration, version 3

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
Use `hannes pass auf --auto-approve` for explicit unattended approval of the
displayed integration plan; validation and conflict checks still apply.

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
See [execution and plan details](profiles-and-plans.md).

### Import rule sets from Git

Reference each external set with three required fields. Imports and local
definitions can coexist in the same document:

```toml
[[rule_sets]]
git = "https://github.com/company/hannes-governance.git"
path = "repository"
ref = "v1.2.0"

[[rule_sets]]
git = "https://github.com/company/hannes-governance.git"
path = "software"
ref = "v1.2.0"

[[rule_sets]]
git = "https://github.com/company/hannes-governance.git"
path = "rust"
ref = "main"
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `git` | Required | Repository selector using host Git authentication: HTTPS, HTTP, SSH, SCP-style SSH, Git protocol, file URL or local path. Local paths are consumer-relative. |
| `path` | Required | Directory within the repository, or `"."` for its root. Parent traversal and symlinks are rejected. |
| `ref` | Required | Exact branch, tag or full commit hash, 1–1024 bytes. No version ranges. |

A shared repository can contain independently addressable sets:

```text
hannes-governance/
  repository/
    ruleset.toml
    rules/readme.toml
  software/
    ruleset.toml
    rules/changelog.toml
  rust/
    ruleset.toml
    rules/formatting.toml
```

Each selected directory must define exactly one set using the same
`[[rule_sets]]`, `[[checks]]` and `[[reviews]]` syntax as local documents.
Rules may live in nested TOML files; file names carry no entity semantics.
References resolve only within that directory's subtree. Declare custom points
such as `check` in the consumer's `[execution_points]` table. Imports cannot
declare them or import further sets.

`hannes pass auf` resolves references, downloads missing revisions and validates
the combined configuration. It executes no rules. Only successful full validation
publishes resolved commits. Branches and movable tags can change on refresh; use
a full commit hash to pin contents. Sets at the same repository and commit share
one immutable cached checkout.

Execution, Git hooks and plans use the last validated revisions without network
access. Missing state, changed imports and missing/corrupt cached contents block
execution and request `hannes pass auf`. A failed refresh reports an error and
preserves previous state; later execution still uses that saved revision.
Removing an import removes it from execution immediately.

Consumers cannot disable or override imported definitions, membership or settings.
Normal source-side `enabled = false` semantics still apply. Local IDs retain their
meaning; imported IDs include a deterministic source namespace and original ID.
Reports retain repository identity, directory, configured ref and resolved commit.

Checks run in the consuming project with ordinary consumer-relative command and
working-directory paths. Review instructions load relative to their imported
document and must stay within the selected directory. Review entry/context paths
refer to consumer files. Imported checks receive `HANNES_RULE_SET_SOURCE` for
explicit access to source scripts/resources; Hannes does not rewrite arguments.

Importing a set trusts its commands to run with Hannes' permissions. Hannes
provides no command sandbox. Retrieval runs no downloaded hooks or initialization
scripts. Configure authentication through host Git or SSH; do not put passwords
or tokens in repository URLs.

See [complete source examples, cache locations and troubleshooting](external-rule-sets.md).


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
