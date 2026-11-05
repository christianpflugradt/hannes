# Hannes configuration, version 1

For the proposed, not yet implemented version-2 selection contract, see
[rule profiles and execution plans](profiles-and-plans.md).

Run Hannes from the Git working-tree root. `.Hannes` uses a capital H.
`config.toml` requires `version = 1`, a nonempty `project` and
`max_display_files = 1..5`. `provider.toml` requires `provider = "codex-cli"`,
`timeout_seconds = 1..3600` and an optional nonempty `model`. New scaffolds use
GPT-6.1 Sol, medium reasoning, and no MCP servers:

```toml
provider = "codex-cli"
timeout_seconds = 180
model = "gpt-6.1-sol"
mcp_servers = []

[review_profile]
reasoning_effort = "medium"
tool_output_token_limit = 2000
```

`mcp_servers` is an explicit allow list of server names already configured in
Codex CLI. Names use ASCII letters, digits, hyphens or underscores so CLI dotted-key
overrides address the original server safely. Empty or omitted disables every
discovered server for the review;
for example, `["memory"]` permits only that named server. Unknown names fail before
the model starts. Server definitions and credentials remain in Codex configuration.
Hannes first reads `codex mcp list --json` without starting a model or MCP server,
then overrides every discovered server individually. Discovery failures, malformed
or oversized listings fail closed. The normal saved Codex login is preserved;
API-key environment variables are removed as before.

Review invocations disable automatic skill instructions, bundled skills, skill
search/automatic MCP installation, apps/plugins, subagents and web search. Only
configured MCP servers are re-enabled. Project AGENTS.md and ordinary Codex safety
settings remain in effect. These settings apply only to Hannes child processes;
user configuration is never edited. `tool_output_token_limit` (256..16384) bounds
each tool output retained in Codex history, not total review tokens. A truncated
result must be read in smaller ranges when necessary. The existing provider timeout
covers MCP configuration discovery and the model invocation together.

`reasoning_effort` is optional; supported names are `none`, `minimal`, `low`,
`medium`, `high`, `xhigh`, `max`, `ultra`. Actual support depends on the selected
model/CLI. Omitting model or effort preserves the corresponding Codex default;
existing configurations without these additions remain readable. A configured
unsupported model/effort fails rather than silently substituting another model.

Each `checks/*.toml` requires unique `id`, nonempty `description`, boolean `enabled`,
`command` (1..256 argv strings) and `timeout_seconds` (1..3600).
`working_directory` defaults to `"."`. Commands execute directly, without shell expansion.
An explicit shell such as `["sh", "-c", "cargo test && cargo clippy"]` is possible
but platform dependent. Bare executables resolve through PATH; path executables must
be relative to the working directory, exist, and resolve inside the project.
Working directories and referenced files must resolve inside the root. Parent traversal
and absolute configuration paths are rejected; this is configuration validation,
not a sandbox for trusted deterministic commands. Checks can modify files or access secrets.

Each `reviews/*.toml` requires unique `id`, nonempty `description`, boolean `enabled`
and `instructions` referencing a UTF-8 `.md` file relative to `reviews/`.
Customize `product-consistency.md`, then set `enabled = true` in its TOML to enable
an optional review using your installed Codex CLI and existing `codex login`.
No separate API key is required. Review use may consume your account allowance.
The default examples never call Codex.

Reviews may specify up to 64 exact root-relative `files` as source entry points,
and `context` entries selecting requirement documents or Markdown sections:

```toml
files = ["Sources/App/Settings.swift", "Tests/SettingsTests.swift"]

[[context]]
path = "Product.md"
headings = ["Keyboard, accessibility, and state", "33. Reviewer decision model"]
```

Titles must match ATX Markdown headings (`#` through `######`) exactly, without
hash prefixes. A selection includes the heading and its subsections up to the
next heading of the same or higher level. Headings inside fenced or indented code
are ignored. Missing or ambiguous titles are errors, including for disabled reviews.
Omitting `headings` includes the whole UTF-8 file. Context is limited to 32 distinct
files of at most 1 MiB each and 64 KiB of selected text across all review definitions;
excess is rejected, never silently cut. Context sections shared or overlapping
between reviews are merged and sent once, with original path and line numbers.
Paths must resolve inside the project; missing files and directories are rejected.
Declared entry/context files must also belong to the verified file manifest before
any model call; ignored files outside that manifest cannot serve as evidence.

When every enabled review supplies `files`, initial manifest hints and the
HEAD-relative orientation diff are limited to the union of entry/context files.
Exact Git paths are literal, not wildcard patterns. Untracked source content still
requires direct inspection. A review without entry files retains project-wide
orientation, including in a mixed batch. The snapshot always covers the full
working tree. Entry files are starting points, not a read restriction: Codex must
follow necessary dependencies and missing requirements rather than infer PASS
from limited excerpts. Shared evidence is read once with targeted searches and
bounded numbered ranges. All active reviews still share one model call.


IDs use 1..80 ASCII letters, digits, hyphens or underscores and are unique across
checks and reviews; the `hannes-` prefix is reserved. Items run in ID order. Unknown fields and duplicate TOML keys
are errors, including in disabled items. `enabled` is the only activation key.
Metadata is limited to 64 KiB per file, 256 items per directory; instructions are
32 KiB per file and 256 KiB combined. Stdout/stderr retain the first 64 KiB each,
while draining the complete streams; byte totals and truncation are recorded.
Raw output lives only in the local JSON report; the terminal never dumps it.
It is not automatically redacted: commands should avoid printing secrets.

Reports are local and ignored. Do not commit them. There is no cache or repair loop.

Review output uses PASS/FAIL for substantive decisions, ERROR for unavailable
necessary evidence or provider problems. Codex must supply a `reason` summary on
one line (at most 120 characters), plus grounded `findings`. The summary is shown
for every review, including PASS. Inspect full findings and captured command
output with `hannes was ist phase --details`; this reads the saved report only.
