# Hannes configuration, version 1

Run Hannes from the Git working-tree root. `.hannes` is lowercase.
`config.toml` requires `version = 1`, a nonempty `project` and
`max_display_files = 1..5`. `provider.toml` requires `provider = "codex-cli"`,
`timeout_seconds = 1..3600` and an optional nonempty `model`.

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

IDs use 1..80 ASCII letters, digits, hyphens or underscores and are unique across
checks and reviews; the `hannes-` prefix is reserved. Items run in ID order. Unknown fields and duplicate TOML keys
are errors, including in disabled items. `enabled` is the only activation key.
Metadata is limited to 64 KiB per file, 256 items per directory; instructions are
32 KiB per file and 256 KiB combined. Stdout/stderr retain the first 64 KiB each,
while draining the complete streams; byte totals and truncation are recorded.
Raw output lives only in the local JSON report; the terminal never dumps it.
It is not automatically redacted: commands should avoid printing secrets.

Reports are local and ignored. Do not commit them. There is no cache or repair loop.


For model/MCP settings, the lean review profile and per-review source/requirement
context, see [the configuration reference](../docs/configuration.md). This checkout
uses `gpt-6.1-sol`, medium reasoning and `mcp_servers = []` in `provider.toml`.
