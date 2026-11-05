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

The generated example passes without Codex. To enable a review, customize
`.Hannes/reviews/product-consistency.md` and set `enabled = true` in its TOML.
Model (`gpt-6.1-sol` by default) and MCP access (disabled by default) are configured
in `.Hannes/provider.toml`. Reviews can select source entry files and requirement
sections; see the configuration reference. Reports stay local in `.Hannes/reports/last.json`. LLM findings are fallible;
line deltas since verification are unavailable with the hash-only baseline.

Install the local commit-message check after cloning with
`sh scripts/setup-git-hooks.sh`. Commit types follow Angular's allow list;
see [Contributing](CONTRIBUTING.md#commits).

## More

[Usage and Git hook](docs/usage.md) · [Configuration](docs/configuration.md) ·
[Design and limits](docs/design.md) · [Passing By example](examples/passing-by/README.md) ·
[Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md) · [Releasing](docs/releasing.md)

Released under the [Unlicense](LICENSE).
