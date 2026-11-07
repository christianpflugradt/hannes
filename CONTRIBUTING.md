# Contributing

Keep changes focused. For bugs, include your OS, `hannes --version`, a minimal
configuration and reproduction steps. For features, explain the problem and how
it fits the small CLI described in [SPECIFICATION.md](SPECIFICATION.md).

## Development

See the [README](README.md#build-locally) for tooling. With mise:

```sh
mise trust
mise install
npm ci --ignore-scripts
cargo +stable fetch --locked
mise deploy
export PATH="$HOME/.local/bin:$PATH"
mise run check
```

The governor must be a published release; building the source is a check, never
how the governor is obtained. See [repository quality gates](docs/self-governance.md)
for execution points, tool prerequisites, CI pinning and Renovate updates. Local hooks
use offline dependency mode; prepare dependencies explicitly before running them.

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
Any type with `!` or a `BREAKING CHANGE:` footer triggers a minor release during
the 0.x development series, and a major release after explicit stable graduation.
The initial release is 0.1.0.
Reserve `build(release)` for automatically generated version commits, which do
not trigger another release. See [release policy](docs/releasing.md#version-policy)
for combined changes and reverts.

| Scopes | Use for |
| --- | --- |
| `checks`, `reviews`, `profiles` | Rules and rule selection; `profiles` remains an accepted historical commit scope. |
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

Hannes manages the three hooks through pass auf and corresponding execution points. Pre-commit checks
formatting and documentation; commit-msg checks the final message before Git
creates or amends a commit; pre-push runs the complete local suite. The shared
message checker reads `.githooks/commit-policy` and never calls a hook. The setup shim retires only the obsolete local `.githooks` setting when its
event files are absent, then delegates to `hannes pass auf`. Other custom hook
paths remain configured; Hannes preserves compatible unmanaged hook content. Each clone needs setup;
Git's `--no-verify` can bypass a local hook. Run `mise run hooks:test` to test the
policy without committing anything in this repository. Work on `main`.

Examples: `chore(deps): update dependencies`, `feat(reviews): add scoped context`, `fix(cli): preserve report output`,
`docs: clarify local installation`.

In your PR, describe the problem, resulting behavior and actual validation.
Contributions use the project's [Unlicense](LICENSE). See
[release instructions](docs/releasing.md) for packaging and publishing.

Native reconciliation tests use temporary repositories and real Git commits/pushes;
run `cargo test --locked --test reconciliation`. Keep tests offline and never
change developer/global Git configuration. The repository integration uses published Hannes v0.7.0 with `--auto-approve`.
Do not substitute a candidate built from this checkout.

For release tooling changes, use the Node version in `mise.toml`, prepare npm
dependencies with `npm ci --ignore-scripts` and run `mise check`. CI also validates all proposed
commit messages and the PR's squash title. Releases are manually started on main;
Semantic Release handles versions, tags, release notes and four native archives.
