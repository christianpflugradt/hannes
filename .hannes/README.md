# Hannes configuration, version 1

Run Hannes from the Git working-tree root. `.hannes` is lowercase.
`config.toml` requires `version = 1`, a nonempty `project` and
`max_display_files = 1..5`. `provider.toml` requires `provider = "codex-cli"`,
`timeout_seconds = 1..3600` and an optional nonempty `model`.

Each `checks/*.toml` requires unique `id`, nonempty `description`, boolean `enabled`,
`command` (a string) and `timeout_seconds` (1..3600):

```toml
command = "cargo test --locked"
```

Hannes executes one executable with arguments directly, without an implicit shell.
Whitespace separates arguments; single/double quotes preserve spaces and escaped
whitespace/quotes are supported. Single quotes preserve all literal characters;
double quotes protect operators/globs but not unescaped `$HOME`, `$(...)` or backticks.
TOML literal strings preserve backslashes; basic strings require TOML escaping too.
For example, `command = 'my-tool --message "Hello; world"'` passes one message argument.
Empty quoted arguments are allowed. Commands have 1..256 executable/argument elements,
each at most 32768 bytes without NUL; the executable must be nonblank. Arrays are rejected.

`hannes pass auf` rejects malformed quoting, trailing escapes, empty executables,
unquoted `&&`, `||`, `;`, pipes, redirection, background execution, variable/command
expansions, globs, shell comments and tilde expansion, including disabled checks.
Quote or escape special characters meant literally. Use npm scripts, Gradle tasks,
Makefiles or dedicated scripts to combine commands. Explicit shells remain possible;
Hannes does not interpret shell expressions.

`working_directory` defaults to `"."`. Bare executables resolve through PATH; path
executables must be relative to the working directory and resolve inside the project.
Availability and execute permissions are checked at runtime, not validation: missing
executables become ERROR. Working directories and referenced review files must exist
inside the root. Parent traversal and absolute configuration paths are rejected;
this is configuration validation, not a sandbox for trusted deterministic commands.
Checks can modify files or access secrets.

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
