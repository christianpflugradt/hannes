# Contributing

Keep changes focused. For bugs, include your OS, `hannes --version`, a minimal
configuration and reproduction steps. For features, explain the problem and how
it fits the small CLI described in [SPECIFICATION.md](SPECIFICATION.md).

## Development

See the [README](README.md#build-locally) for tooling. With mise:

```sh
mise trust
mise install
mise run check
```

Or use your existing Rust toolchain and Python 3.11+:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
python3 scripts/smoke.py target/release/hannes
```

On Windows, use `python` and `target/release/hannes.exe` for the last command.
Tests must remain offline and credential-free; mock Codex instead of calling it.
Keep source, configuration and documentation in English. German CLI phrases are
intentional. Update relevant docs and focused tests when behavior changes.
Commit Cargo.lock; exclude reports, credentials and local MCP configuration.

## Commits

Use [Angular's commit-type allow list](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md#type): `build`, `ci`, `docs`, `feat`, `fix`,
`perf`, `refactor`, `test`. Other types, including `chore`, `style` and `revert`,
are rejected. Use `type(scope): summary`; the scope is optional and uses lowercase
letters, digits, dots, underscores, `/` or `-`. A breaking-change `!` before the
colon is supported. The summary must be nonempty without leading/trailing
whitespace. Write a short imperative summary; bodies and footers are optional.
Scopes refer to this project, not Angular's packages.

```sh
sh scripts/setup-git-hooks.sh
# Or: mise run hooks:install
```

The versioned `.githooks/commit-msg` checks the final message before Git creates
or amends a commit. `pre-commit` runs before the message is available, so it cannot
perform this check reliably. Setup changes only this checkout's `core.hooksPath`
and refuses to replace existing custom hooks/configuration. Each clone needs setup;
Git's `--no-verify` can bypass a local hook. Run `mise run hooks:test` to test the
policy without committing anything in this repository. Work on `main`.

Examples: `feat(reviews): add scoped context`, `fix(cli): preserve report output`,
`docs: clarify local installation`.

In your PR, describe the problem, resulting behavior and actual validation.
Contributions use the project's [Unlicense](LICENSE). See
[release instructions](docs/releasing.md) for packaging and publishing.
