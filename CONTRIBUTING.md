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

Use Conventional Commit types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`,
`perf`, `refactor`, `revert`, `style`, `test`. Other types are rejected.
Use `type(scope): summary`; the scope is optional, but when present
must come from the following list. A breaking-change `!` before the
colon is supported. The summary must be nonempty without leading/trailing
whitespace. Write a short imperative summary; bodies and footers are optional.
The shared [commit policy](.githooks/commit-policy) defines the accepted types and scopes.

| Type | Purpose | Release |
| --- | --- | --- |
| `feat` | New functionality | Minor |
| `fix` | Bug fixes, including hotfixes and security fixes | Patch |
| `perf` | Performance improvements | Patch |
| `chore` | Other maintenance, including routine dependency updates | Patch |
| `build` | Build system, packaging and toolchain | Patch |
| `refactor` | Code restructuring without intended behavior changes | Patch |
| `revert` | Undo an earlier change | Patch for effective reversions |
| `ci` | CI/CD workflows | None |
| `docs` | Documentation | None |
| `style` | Formatting and whitespace only | None |
| `test` | Tests and test infrastructure | None |

Use the most specific type; `chore` is the fallback. Dependency updates normally
use `chore(deps)`; use `fix(deps)` when fixing a known bug or security issue.
Any type with `!` or a `BREAKING CHANGE:` footer triggers a major release.
Reserve `build(release)` for automatically generated version commits, which do
not trigger another release. See [release policy](docs/releasing.md#version-policy)
for combined changes and reverts.

| Scopes | Use for |
| --- | --- |
| `checks`, `reviews`, `profiles` | Rules and rule selection |
| `cli`, `mood` | Commands, terminal output and CLI internals |
| `config`, `provider` | Configuration and provider integration |
| `report`, `snapshot` | Results and review input |
| `deps`, `release` | Dependency updates and release tooling |
| `git`, `mise`, `ci` | Git policy, local tasks and workflows |
| `docs`, `tests` | Documentation and test infrastructure |

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

Examples: `chore(deps): update dependencies`, `feat(reviews): add scoped context`, `fix(cli): preserve report output`,
`docs: clarify local installation`.

In your PR, describe the problem, resulting behavior and actual validation.
Contributions use the project's [Unlicense](LICENSE). See
[release instructions](docs/releasing.md) for packaging and publishing.

For release tooling changes, use Node 24.15+ (24.x), run `npm ci --ignore-scripts`,
`npm test` and `python3 scripts/test-release.py`. CI also validates all proposed
commit messages and the PR's squash title. Releases are manually started on main;
Semantic Release handles versions, tags, release notes and three native archives.
