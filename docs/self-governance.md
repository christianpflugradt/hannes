# Repository self-integration

Hannes checks this repository using an installed published Hannes executable.
Builds under target/ are candidates under test and never become the governor.
The repository uses local Check definitions and a pinned external commit-message
Rule Set and Security Rule Set, with no reviews or provider calls.

The exact governor pin in [.hannes-version](../.hannes-version) is v0.9.0. This
published release supports the repository configuration, execution points and
shared external Rule references and pass auf --auto-approve for unattended hook
reconciliation.

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

Python 3.11+, Git, Gitleaks v8 with `gitleaks dir` (tested with 8.30.1), stable
Rust with rustfmt/Clippy and the Node version in mise.toml are required locally.
Install Gitleaks explicitly and put it on PATH before running the security checks
or pre-push gate; `gitleaks version` verifies availability. Hooks inherit PATH
and call the installed hannes binary.
Prepare dependencies online first. The [check launcher](../scripts/run-check.py)
sets Cargo/npm offline mode and disables Rustup auto-install for managed Git-hook
checks, then executes the original command without a shell. It preserves the
hook context, arguments and recursion guard. Outside hooks it preserves the
caller's environment. Checks neither install the governor nor repair project files.

## Execution points and required coverage

Every selected rule is mandatory. Each point preserves the exact check membership
of its configured execution point. Checks run once per binding, ordered by ID. The
[governance fixture](../tests/fixtures/self-governance.json) records
the commands, activation states, timeouts and required selections; governance
acceptance compares the local checks in the published governor's actual plans
with that evidence and separately verifies the imported commit-message and
security Rules.

| Execution point | Required checks |
| --- | --- |
| pre-commit | 01-format, 02-docs |
| commit-msg | 20-commit-message plus imported commit-message |
| pre-push (default) | 01-format, 02-docs, 03-docs-tests, 04-governance-tests, 05-hooks-tests, 06-deploy-tests, 07-release-tests, 08-release-rules, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke plus imported secret-scan and sensitive-files |
| check | All pre-push checks plus 21-commit-range, 22-workflows, 23-msrv, 24-rust-audit and 25-npm-audit: 19 rules total |
| release-build | 01-format, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke |
| release-artifacts | 30-release-artifacts |

The [checks](../.hannes/checks) retain their validation commands, timeout settings
and enabled state. Each Rule has an index.sh entrypoint that invokes the shared
command launcher to preserve offline hook behavior. Each local Rule declares
its execution points directly; this repository defines no local Rule Sets.
The external commit-message and Security Rule Sets retain their own assignments.
All Rust unit, integration, acceptance and documentation tests remain in one Rule.

Formatting, documentation, Clippy, Rust acceptance tests, deployment
and release regression tests, Conventional Commits, actionlint, minimum Rust 1.85,
Rust/npm dependency audits and release artifact verification remain required at
the points shown above. Audits need advisory feeds; release artifact verification
requires EXPECTED_RELEASE_VERSION, GITHUB_SHA and a complete dist/ artifact set.
These checks cannot all be meaningfully executed in an unprepared local checkout.

For manual developer use:

```sh
hannes mach was --at pre-commit
hannes mach was --at pre-push --plan
python3 scripts/run-execution-point.py check
hannes was ist phase --details
```

For explicit manual use, mise check delegates through the execution wrapper to
Hannes at pre-push. Individual format, lint, test, smoke and docs:check tasks
remain available to developers. Format explicitly edits source; verification
only checks formatting.

## Agent workflow

Agents receive validation feedback from Git commit and push. They must not run
checks, tests, linters or builds manually, including mise check, mise docs:check,
individual Cargo/npm commands or direct Hannes execution. Source formatting is
allowed. The fast formatting and documentation checks run at pre-commit; pre-push
runs the complete local suite, including those checks again. There is no result
cache, so a manual full run before pushing would repeat the suite.

If pre-commit or commit-msg fails, fix the issue and retry the commit. If pre-push
fails, fix the issue, amend only the agent's own unpublished commit when appropriate
and retry the push. Do not bypass hooks, force-push or rewrite unrelated work.
Read hook output or use hannes was ist phase --details for saved output without
rerunning rules. Address missing tools or hook drift, or report the blocker.
Prepare the intended commit before the final push: general local checks inspect
working files, while the imported pre-push security checks inspect HEAD.

## Git hooks

The published governor derives the three required Git hooks from effective Rule
assignments. Each hook runs its fixed, identically named execution point. The
repository configuration declares no hook mappings. The governor manages wrappers
through pass auf, including configured hook paths, compatible unmanaged content,
ownership checks, argument forwarding and recursion protection. Wrappers inspect
synchronization read-only; drift tells the developer to rerun pass auf manually.

The `commit-msg` point imports `commit-msg/conventional-commits` from
[the shared Ruleset repository](https://github.com/christianpflugradt/christianpflugradt-rulesets),
pinned to the full commit in `.hannes/config.toml`. It receives
`.hannes/commit-scopes.txt` through the `scopes_file` parameter. Governance tests
require this file to match the scopes in `.githooks/commit-policy`.
Both message checks consume HANNES_GIT_HOOK_ARG_1 and require exactly one
commit-msg argument. The imported Rule validates the header, type and scope.
The local `20-commit-message` check also enforces Hannes' narrower type allowlist
and summary whitespace requirements using the same policy checker as CI.
`pass auf` retrieves and validates the import during setup; subsequent checks and
hooks use its immutable cache offline. A fresh clone needs access to the source
repository during setup.
The same external repository supplies `multi-point/security`, pinned separately
in `.hannes/config.toml`. Its blocking `sensitive-files` and `secret-scan` checks
run at `check` and `pre-push`. `check` scans tracked and nonignored untracked
working-tree files, including unstaged changes; deleted files are excluded.
`pre-push` scans the current `HEAD` tree, without history or other pushed refs.
The filename policy rejects `.env` and `.env.*` except `.env.example`,
`.env.sample` and `.env.template`, plus Terraform state files. Gitleaks scans
contents, including allowed templates; symlink contents are excluded and
submodules cause ERROR. Scanner configuration comes from the selected snapshot.
The external Rule Set documents its complete policy and limitations.

Pre-commit runs before the final message is available. Hooks block on failure and
never modify the Git index. Local checks verify working files, including
unstaged edits, which can differ from staged or pushed commits. The imported
security checks explicitly read `HEAD` at pre-push. Hooks are bypassable; clean
CI checkouts remain authoritative. General Hannes index/commit inputs remain
future work.

## CI, pinning and Renovate

[.hannes-version](../.hannes-version) is the single exact governor pin. The
[CI workflow](../.github/workflows/ci.yml) has one **Quality checks** job on Ubuntu
x64. It prepares Python, Node and Go, then explicitly prepares Rust toolchains,
locked dependencies, actionlint, cargo-audit and checksum-verified Gitleaks through
[scripts/prepare-ci.sh](../scripts/prepare-ci.sh). Preparation is outside Hannes
Rule execution. The job installs the pinned published governor through the public
[Action](../action.yml), resolves imports once and invokes `check` once.
One `quality-check` artifact preserves reports even on failure.

The separate **GitHub Action compatibility** matrix tests the public Action on
Ubuntu x64 and ARM64 using a disposable consumer. It verifies PATH, install-only
mode, custom selection and failure propagation. These runner-specific tests remain
separate from repository checks.

The [release-build workflow](../.github/workflows/quality.yml) runs only for
releases, with four native runners and the `release-build` execution point.
The release workflow also reuses CI before publication. The publisher installs
Gitleaks for its managed pre-push hook. macOS release runners use the
[native installer](../scripts/install-governor.py). The execution wrapper rejects
candidate governors inside the checkout and checks CI's version against the pin.

The current v0.9.0 governor supports the consolidated execution points but predates
the new display names and progress modes. Repository rule documents keep using
its supported fields until a compatible release is published and the pin is
updated. CLI acceptance tests exercise the candidate's new display behavior in
disposable repositories; candidate builds never govern this checkout.

The initial Action was published on main at
[`2d3cac2324ce54f076c7090e5bf80969eee4b37e`](https://github.com/christianpflugradt/hannes/commit/2d3cac2324ce54f076c7090e5bf80969eee4b37e)
before a second commit added public `uses: christianpflugradt/hannes@2d3cac2324ce54f076c7090e5bf80969eee4b37e`
references. This immutable publication revision contains `action.yml`; it has no
separate Action release tag or compiled artifact. No major-version tag is created.
The Action revision and CLI release pin are independent.

The regex custom manager in [renovate.json](../renovate.json) retains the
GitHub-releases datasource and disabled automerge. Pin updates remain normal
reviewed dependency PRs. A format-breaking update may require configuration
changes; every execution point must validate and plan with the proposed release.

Push/PR checks retain native Linux x64, tooling, MSRV and audit coverage. Releases
reuse that CI workflow and run native validation on all four supported hosts. Tool
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
as separate artifacts. Schema-4 last.json records execution
point, required rules, source state and results. The [execution wrapper](../scripts/run-execution-point.py)
adds execution.json with governor version/digest, execution point, checked commit
and exit code. Managed hooks and direct CLI invocations write the ordinary report;
the wrapper supplies companion evidence for CI and mise invocations.
