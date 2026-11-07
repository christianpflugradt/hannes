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
[immutable migration fixture](../tests/fixtures/self-governance.json) records
the previous commands, activation states, timeouts and selections; governance
acceptance compares the local checks in the published governor's actual plans
with that evidence and separately verifies the imported commit-message and
security Rules.

| Previous selection → execution point | Required checks | Assignment |
| --- | --- | --- |
| pre-commit → pre-commit | 01-format, 02-docs | Own run_at |
| commit-msg → commit-msg | 20-commit-message plus imported commit-message | Own run_at plus external conventional-commits |
| pre-push → pre-push (default) | 01-format, 02-docs, 03-docs-tests, 04-governance-tests, 05-hooks-tests, 06-deploy-tests, 07-release-tests, 08-release-rules, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke plus imported secret-scan and sensitive-files | Own run_at plus tooling-regressions, native-validation and external security |
| check | Imported secret-scan and sensitive-files | External security |
| ci-native → ci-native | 01-format, 10-clippy, 11-rust-tests, 12-native-build, 13-native-smoke | Own run_at plus native-validation |
| ci-tooling → ci-tooling | 02-docs, 03-docs-tests, 04-governance-tests, 05-hooks-tests, 06-deploy-tests, 07-release-tests, 08-release-rules, 21-commit-range, 22-workflows | Own run_at plus tooling-regressions |
| ci-msrv → ci-msrv | 23-msrv | Own run_at |
| ci-audit → ci-audit | 24-rust-audit, 25-npm-audit | dependency-audits |
| release-artifacts → release-artifacts | 30-release-artifacts | Own run_at |

The [checks](../.hannes/checks) retain their validation commands, timeout settings
and enabled state. Each Rule has an index.sh entrypoint that invokes the shared
command launcher to preserve offline hook behavior. The
[rule sets](../.hannes/rule-sets) group checks sharing execution points; standalone rules reference the same built-in and declared points.

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

[.hannes-version](../.hannes-version) is the single exact governor pin. Workflows
read it and pass the exact tag through the public [Action](../action.yml)'s
`version` input, with `execution-point: check` and `run: 'false'` on Ubuntu.
The Action downloads the release, verifies its checksum and installs outside the
checkout. macOS release runners retain the [native installer](../scripts/install-governor.py),
since the public Action initially supports Ubuntu only. Each governance job
separately runs `hannes pass auf --auto-approve` to reconcile hooks and external
rules, then runs its existing execution point. The Ubuntu tooling job additionally
runs `check` after installing checksum-verified Gitleaks 8.30.1; Python 3 is already
prepared. It uploads security evidence as `quality-check` before ci-tooling
replaces the current report. The release publisher installs the same Gitleaks
binary before managed commit/push hooks can run. Other jobs do not execute the
Security Rule Set. No governance job installs the governor twice.
The execution wrapper rejects candidates inside the checkout and checks CI's
executable version against the pin.

The initial Action was published on main at
[`2d3cac2324ce54f076c7090e5bf80969eee4b37e`](https://github.com/christianpflugradt/hannes/commit/2d3cac2324ce54f076c7090e5bf80969eee4b37e)
before a second commit added public `uses: christianpflugradt/hannes@2d3cac2324ce54f076c7090e5bf80969eee4b37e`
references. This immutable publication revision contains `action.yml`; it has no
separate Action release tag or compiled artifact. No major-version tag is created.
The Action revision and CLI release pin are independent.

The reusable quality workflow also runs a disposable consumer fixture on Ubuntu
x64 and ARM64. It checks installation of the pinned CLI release, PATH in a subsequent step,
install-only mode, a custom execution point and the default execution/failure path
through the public Action. The expected failure is asserted explicitly; all
repository governance jobs and release gates remain required.

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
as separate artifacts. Schema-4 last.json records execution
point, required rules, source state and results. The [execution wrapper](../scripts/run-execution-point.py)
adds execution.json with governor version/digest, execution point, checked commit
and exit code. Managed hooks and direct CLI invocations write the ordinary report;
the wrapper supplies companion evidence for CI and mise invocations.
