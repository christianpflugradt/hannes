# Hannes configuration

Hannes has three governance concepts: a **Rule** defines an expectation, a
**Rule Set** groups rules and assigns them to **Execution Points**. Checks and
Reviews are the two rule implementations. An import references a source Rule Set.

## Configuration

### Files and validation

Use this fixed layout at the Git working-tree root:

```text
.hannes/
  config.toml
  provider.toml
  checks/readme/
    rule.toml
    index.sh
  reviews/architecture/
    rule.toml
    prompt.md
  rule-sets/repository.toml
  reports/last.json
```

`checks/`, `reviews/` and `rule-sets/` are optional. Each immediate rule directory
requires `rule.toml`. Only `config.toml`, `provider.toml`, `checks/*/rule.toml`,
`reviews/*/rule.toml` and `rule-sets/*.toml` are governance configuration. Supporting
files such as `checks/secrets/gitleaks.toml` are data and are never parsed by Hannes.
Definitions cannot be placed arbitrarily or nested in other directories.

Unknown fields, duplicate keys/IDs and invalid references fail validation, even
for disabled rules. Text uses UTF-8; required text must be nonblank and contain no
NUL. Resource filenames must name regular files directly inside their own rule
directory: no empty names, separators, absolute paths, parent traversal or symlinks.
Rule directories, definitions and configuration paths also reject symlinks.

Run `hannes pass auf` after editing. Validation runs no checks or providers.
Required Git hooks are derived from effective Rule assignments and reconciled
after confirmation; use `--auto-approve` for unattended reconciliation. New projects initialized by `hannes fang an` include
one enabled passing check, one disabled failing check and one disabled review.
Initialization preserves existing files and installs no hooks.

### Project settings

`.hannes/config.toml`:

```toml
project = "My_App"

[execution_points]
custom = ["release"]
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `project` | Required | Nonblank display name. |
| `max_display_files` | Optional; `5` | Integer `1..5`: changed paths shown in status. |
| `execution_points` | Optional table | Additional execution contexts. |
| `execution_points.custom` | Optional; `[]` | Unique names of 1–80 lowercase ASCII letters, digits, `-`, `_`, starting with a letter/digit. Built-in names are reserved. |
| `hooks` | Optional; `null` (omitted in TOML) | Desired integrations. |
| `hooks.git` | Optional; `{}` | Optional explicit event mappings. Required hooks are automatic; `check` cannot be a target. A mapping must retain the built-in point when enabled Rules require it. |
| `imports` | Optional; `[]` | External references, declared only here as `[[imports]]`. |

Built-in points are `pre-commit`, `commit-msg`, `pre-push` and `check`. The first
three use Git hooks. `check` is invoked explicitly with `hannes mach was --at check`
and never triggered automatically. It needs no declaration or CI workflow.
Consumers decide where to invoke it, including multiple workflows; Hannes does
not generate workflows, discover them or validate CI integration. Declare custom
points in the consumer, including unmapped custom points used by imports.
`release` is an ordinary custom point. The default execution point is `pre-push`.

### Fields shared by checks and reviews

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Case-sensitive ID unique across checks/reviews in its source scope; 1–80 ASCII letters, digits, `-`, `_`; `hannes-` is reserved. |
| `description` | Required | Nonblank explanation. |
| `enabled` | Optional; `true` | False rules are validated but never executed or included in results. Plans list them under `disabled_rules`. |
| `blocking` | Optional; `true` | A completed FAIL prevents success when true. ERROR is always fatal. |
| `run_at` | Optional; `[]` | Own execution points; overridden by membership in Rule Sets. Enabled rules need an effective assignment. |
| `check` | Optional; `null` | Check implementation; required for a check rule and mutually exclusive with `review`. |
| `review` | Optional; `null` | Review implementation; required for a review rule and mutually exclusive with `check`. |

### Add a check

Create `.hannes/checks/readme/rule.toml`:

```toml
id = "repository-readme"
description = "Require a nonempty README"
run_at = ["pre-push", "check"]

[check]
entrypoint = "index.sh"
```

Create `.hannes/checks/readme/index.sh`:

```sh
#!/bin/sh
test -s README.md
```

On Unix, make it executable with `chmod +x .hannes/checks/readme/index.sh`.
Hannes executes the file directly; the OS handles permissions and shebangs.
Hannes does not infer languages, parse shebangs or install runtimes. Direct
execution uses native OS behavior on Windows; Unix shell scripts may need an
explicitly installed shell and do not necessarily run natively.

Alternatively, use an explicit interpreter. For example, a Python check uses:

```toml
[check]
entrypoint = "index.py"
interpreter = "python3"
timeout_seconds = 30
```

Create `index.py` in that rule directory containing:

```python
from pathlib import Path
raise SystemExit(0 if Path("README.md").is_file() else 1)
```

This runs `python3 <absolute-entrypoint>` with no implicit shell. The interpreter
is one executable name or path, with no argument parsing. The entrypoint needs
neither a shebang nor executable permission in this mode. Missing interpreters
produce a clear startup ERROR.

| Field | Required / default | Meaning |
| --- | --- | --- |
| `check.entrypoint` | Required | Filename directly within this check directory. |
| `check.interpreter` | Optional; `null` | Executable used to run the entrypoint. Omitted means direct execution. |
| `check.timeout_seconds` | Optional; `30` | Integer `1..3600`; deadline including output capture. |

Checks inherit the environment, receive no stdin and keep the consuming project's
root as their working directory. `HANNES_RULE_DIRECTORY` is the absolute directory
of the check, for bundled resources such as a scanner configuration or helper.
For example, Python can read `Path(os.environ["HANNES_RULE_DIRECTORY"]) / "config.json"`.

Exit 0 is PASS; nonzero is FAIL except **exit 2**, which explicitly reports an
execution ERROR. A wrapper around a scanner must map its infrastructure failures
to exit 2. Spawn, capture and timeout failures are always ERROR. A non-blocking
FAIL remains visible and permits reviews and overall success; an ERROR remains
fatal regardless of blocking. Output capture and process cleanup are shared
across direct and interpreter execution.

Resource confinement is not process sandboxing. Checks execute with Hannes'
permissions and can access files, credentials and the network. Only import
executable checks from trusted sources. Changes to evaluated files during
execution prevent PASS.

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


`provider.toml` is optional; omission uses the generated provider settings above.
When the file is present its required fields must be supplied.

### Add a review

Create `.hannes/reviews/architecture/rule.toml`:

```toml
id = "architecture-review"
description = "Review architectural consistency"
blocking = false
run_at = ["pre-push"]

[review]
prompt = "prompt.md"
```

Create `.hannes/reviews/architecture/prompt.md`:

```markdown
Compare the implementation with the project's documented architectural boundaries.
Report concrete contradictions as FAIL, missing essential evidence as ERROR,
and PASS only with supporting file paths and line numbers.
```

The Markdown contains instructions only, with no frontmatter or repeated metadata.
Prompts resolve exclusively inside their review directory. Reviews evaluate the
consumer through the existing shared provider call, after deterministic checks.

| Field | Required / default | Meaning |
| --- | --- | --- |
| `review.prompt` | Required | Nonblank UTF-8 `.md` filename in the review directory. |
| `review.files` | Optional; `[]` | Up to 64 unique existing consumer-relative source files; exact paths, not globs. Starting points, not an evidence boundary. |
| `review.context` | Optional; `[]` | Up to 32 document selections as `[[review.context]]`. |
| `review.context.path` | Required | Existing UTF-8 consumer-relative file. |
| `review.context.headings` | Optional; `[]` | Up to 32 nonblank exact ATX Markdown headings; empty selects the whole document. |

Context includes subsections and preserves original lines. Code-block headings
are ignored; missing or ambiguous headings are errors. Overlapping selections
are merged. All context is bounded to 32 distinct files, 1 MiB per file and 64 KiB
selected text; disabled definitions are validated too. Entry/context paths must
stay within the consumer. Supporting review files require no resource-import system.

The provider must return exactly one PASS/FAIL/ERROR per requested ID. PASS/FAIL
need an existing in-snapshot file and positive line reference. Malformed JSON,
missing/duplicate verdicts, invalid evidence and provider failures produce ERROR.
Summaries are single lines up to 120 characters; at most 20 findings per review,
2000 bytes each. Non-blocking FAIL is visible; incomplete verification stays fatal.

### Group rules into rule sets

Create `.hannes/rule-sets/repository.toml`:

```toml
id = "repository"
description = "General repository governance"
run_at = ["pre-push", "check"]
rules = ["repository-readme", "architecture-review"]
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `id` | Required | Unique set ID: 1–80 lowercase ASCII letters, digits, `-`, `_`, starting with a letter/digit. Independent of rule IDs. |
| `description` | Required | Nonblank explanation. |
| `run_at` | Required | Nonempty list of known execution points. |
| `rules` | Required | Unique rule IDs defined in the same scope. Empty is allowed; set references are unsupported. |

Present sets always apply; they have no enabled, blocking or source attributes.
Remove a set or its rule references to stop using it. Set membership overrides a
rule's own assignments; multiple memberships contribute their union. Each rule
runs at most once at a point. Standalone enabled rules use their own `run_at`.
Disabled rules never execute. All references, even overridden ones, are validated.

```sh
hannes pass auf
hannes mach was --at pre-commit
hannes mach was --at commit-msg
hannes mach was --at pre-push
hannes mach was --at check
hannes mach was --at release --plan --json
```

Checks precede reviews, ordered by ID. Empty points succeed. Every selected rule
must execute; `--geiz-ist-geil` cannot omit required reviews. An offline plan
prepares the same selection without rules, provider calls or reconciliation.

### Import rule sets from Git

Add imports only to `.hannes/config.toml`:

```toml
project = "My_App"

[execution_points]
custom = ["verify", "audit"]

[[imports]]
git = "https://github.com/example/governance.git"
path = "repository"
ref = "v1.0.0"

[[imports]]
git = "https://github.com/example/governance.git"
path = "software-quality"
ref = "v1.0.0"

[imports.execution_points]
quality = "verify"

[[imports]]
git = "https://github.com/example/governance.git"
path = "security"
ref = "v2.1.0"

[imports.execution_points]
scan = "audit"
report = "verify"
```

| Field | Required / default | Meaning |
| --- | --- | --- |
| `git` | Required | Repository selector using host Git authentication: HTTP(S), SSH, SCP-style SSH, Git protocol, file URL or consumer-relative local repository. |
| `path` | Required | Selected directory, or `"."` for repository root; no parents or symlinks. |
| `ref` | Required | Exact branch, tag or full commit hash, 1–1024 bytes. |
| `execution_points` | Optional; `{}` | Per-import table mapping external custom names to consumer-declared custom names. |

The table belongs only to the immediately preceding `[[imports]]`. Sources must
be custom names used by that imported Rule Set; targets must already appear in
the consumer's `execution_points.custom`. Names follow the same syntax and limits
as custom declarations. Built-ins are forbidden on both sides. Unknown names,
empty/invalid names and structures other than a table of strings fail validation.
Unmapped points keep their names and must be known to the consumer.

Mappings apply after scope-local Rule Set inheritance. Multiple source points may
map to one target; a Rule runs once there. Separate imports may map the same source
name differently or share targets. No global or individual Rule mapping exists.
Mapping preserves Rule identity, enabled/blocking state, resources and provenance;
it changes neither cached files nor revision identity and works offline without
fetching. No other import fields or Rule overrides are accepted. Remove the import to
stop using it. A selected directory is its own governance root, with exactly one
set in `config.toml` and its own `checks/` and `reviews/`. It has no nested
`.hannes/`, `rule-sets/`, project settings or imports. Its `config.toml` uses the
same four fields shown for a local Rule Set. References stay within that scope.
Identical rule IDs across independent sources do not collide; imported identities
and commit provenance remain deterministic.

`pass auf` resolves refs and validates all sources before atomically publishing
resolution state. A failed refresh preserves prior state. Execution, hooks and
plans reuse the validated immutable cache offline; missing/changed state or corrupt
contents require `hannes pass auf`. Multiple sets at one repository/commit share
one checkout. Full hashes pin content; branches/tags can move when refreshed.

See [external source structure, cache and trust](https://github.com/christianpflugradt/hannes/blob/main/docs/external-rule-sets.md).

### Reports and other files

Reports use schema 4 and plans use schema 3. These output schema numbers describe
machine-readable results, not configuration variants. Results record blocking,
status, evidence, durations and captured output. ERROR always takes precedence;
blocking FAIL follows; unexecuted selected rules prevent success. Disabled rules
are listed in plans and stay out of results. JSON records FAIL even when the
overall outcome is PASS because that violation is non-blocking.

`reports/last.json` is replaced atomically after a run. Invalid configuration
preserves a previous report. Keep reports and credentials local. Stdout/stderr
retain the first 64 KiB each with full byte totals/truncation indicators; output
is not automatically redacted. `--details` displays retained output and findings.
Status compares the saved working-tree snapshot without executing rules again.
