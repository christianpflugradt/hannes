# Repository self-integration

Hannes checks this repository using an installed published Hannes executable.
Builds under target/ are candidates under test and never become the governor.
The repository retains its existing governance configuration, with no
reviews or provider calls.

The exact governor pin in [.hannes-version](../.hannes-version) is v0.5.0. This
published release supports the repository configuration, execution points and
pass auf --auto-approve for unattended hook reconciliation.

## Local setup

Prepare tools and dependencies explicitly:

```sh
mise trust
mise install
npm ci --ignore-scripts
cargo +stable fetch --locked
mise deploy
export PATH="$HOME/.local/bin:$PATH"
mise hooks:install
mise check
```

The setup task delegates hook management to hannes pass auf. If an old clone still
sets core.hooksPath to .githooks, the setup shim removes that local setting only
when all three obsolete event files are absent. If an event file remains, setup
stops and preserves it for manual integration. Other configured hook paths are
retained and reconciled by Hannes. No global Git settings are changed.
Fresh clones can run hannes pass auf directly. Interactive setup displays the
plan and requests confirmation. For unattended setup use:

```sh
hannes pass auf --auto-approve
```

Setup installs no tools or dependencies. Conflicts still fail. Generated wrappers
live at Git's effective hook path and carry local ownership; never check them in
or copy them between repositories. Only the commit policy remains versioned under
[.githooks](../.githooks).

Python 3.11+, Git, stable Rust with rustfmt/Clippy and the Node version in mise.toml
are required locally. Hooks inherit PATH and call the installed hannes binary.
Prepare dependencies online first. The [check launcher](../scripts/run-check.py)
sets Cargo/npm offline mode and disables Rustup auto-install for managed Git-hook
checks, then executes the original command without a shell. It preserves the
hook context, arguments and recursion guard. Outside hooks it preserves the
caller's environment. Checks neither install the governor nor repair project files.

## Execution points and required coverage

Every selected rule is mandatory. Each point preserves the exact check membership
of its configured execution point. Checks run once, ordered by their numbered IDs. The
[immutable migration fixture](../tests/fixtures/self-governance.json) records
the previous commands, activation states, timeouts and selections; governance
acceptance compares the published governor's actual plans with that evidence.

| Previous selection → execution point | Required checks | Assignment |
| --- | --- | --- |
| pre-commit → pre-commit | 01-format, 02-docs | Own run_at |
| commit-msg → commit-msg | 20-commit-message | Own run_at |
| pre-push → pre-push (default) | 01-format, 02-docs, 03-docs-tests, 04-governance-tests, 05-hooks-tests, 06-deploy-tests, 07-release-tests, 08-release-rules, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke | Own run_at plus tooling-regressions and native-validation |
| ci-native → ci-native | 01-format, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke | Own run_at plus native-validation |
| ci-tooling → ci-tooling | 02-docs, 03-docs-tests, 04-governance-tests, 05-hooks-tests, 06-deploy-tests, 07-release-tests, 08-release-rules, 21-commit-range, 22-workflows | Own run_at plus tooling-regressions |
| ci-msrv → ci-msrv | 23-msrv | Own run_at |
| ci-audit → ci-audit | 24-rust-audit, 25-npm-audit | dependency-audits |
| release-artifacts → release-artifacts | 30-release-artifacts | Own run_at |

The [checks](../.hannes/checks) retain their validation commands, timeout settings
and enabled state. Only the command launcher prefix is added to preserve offline
hook behavior. The [rule sets](../.hannes/rule-sets.toml) group checks sharing
execution points; standalone rules reference the same built-in and declared points.

Formatting, documentation, Clippy, Rust acceptance tests, deployment
and release regression tests, Conventional Commits, actionlint, minimum Rust 1.85,
Rust/npm dependency audits and release artifact verification remain required at
the points shown above. Audits need advisory feeds; release artifact verification
requires EXPECTED_RELEASE_VERSION, GITHUB_SHA and a complete dist/ artifact set.
These checks cannot all be meaningfully executed in an unprepared local checkout.

```sh
hannes mach was --at pre-commit
hannes mach was --at pre-push --plan
python3 scripts/run-execution-point.py ci-native
hannes was ist phase --details
```

mise check uses the execution wrapper at pre-push. Individual format, lint, test,
smoke and docs:check tasks remain available. Format explicitly edits source;
verification only checks formatting.

## Git hooks

The pinned published governor retains its existing hook declarations until a
separate release and self-migration. They select pre-commit, commit-msg and
pre-push at their identically named points. The candidate uses fixed hook semantics
and rejects those legacy declarations; its fixtures are isolated from repository
self-integration. The published governor manages this repository's wrappers
through pass auf, including configured hook paths, compatible unmanaged content,
ownership checks, argument forwarding and recursion protection. Wrappers inspect
synchronization read-only; drift tells the developer to rerun pass auf manually.

The commit-message checker consumes HANNES_GIT_HOOK_ARG_1 from commit-msg, requires
exactly one argument and uses the same standalone commit policy checker as CI.
Pre-commit runs before the final message is available. Hooks block on failure and
never modify the Git index. They verify working files, including unstaged edits,
which can differ from the staged or pushed commits. Hooks are bypassable; clean
CI checkouts remain authoritative. Exact index/commit inputs are future work.

## CI, pinning and Renovate

[.hannes-version](../.hannes-version) is the single exact governor pin. The
[setup action](../.github/actions/setup-hannes/action.yml) downloads that release,
verifies archive checksum and executable version, installs outside the checkout,
and runs pass auf --auto-approve once. It never resolves latest or builds the
governor. The execution wrapper rejects candidates inside the checkout and
checks CI's executable version against the pin. Local installation must support
the pinned configuration and unattended setup option.

The regex custom manager in [renovate.json](../renovate.json) retains the
GitHub-releases datasource and disabled automerge. Pin updates remain normal
reviewed dependency PRs. A format-breaking update may require configuration
changes; every execution point must validate and plan with the proposed release.

Push/PR jobs retain the native Linux x64, tooling, MSRV and audit checks. Releases
reuse those jobs and run native validation on all four supported hosts. Tool
installation, dependency preparation, release stamping, packaging and publication
remain separate operations. Before publication, the publisher installs stable Rust
with rustfmt and Clippy and fetches locked Cargo dependencies. Its automated
version commit and push invoke managed hooks with offline checks; release lifecycle
tests exercise hooks in a disposable repository. After artifact collection,
release-artifacts verifies
the complete set and its source/version binding; the publication plugin repeats
its integrity checks. No release process or permissions change is introduced.

Jobs retain their failure-report uploads. The publisher saves artifact-verification
evidence before publication and uploads hook reports afterward, even on failure,
as separate artifacts. Schema-3 last.json records execution
point, required rules, source state and results. The [execution wrapper](../scripts/run-execution-point.py)
adds execution.json with governor version/digest, execution point, checked commit
and exit code. Managed hooks and direct CLI invocations write the ordinary report;
the wrapper supplies companion evidence for CI and mise invocations.
