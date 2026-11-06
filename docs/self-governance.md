# Repository quality gates

Hannes checks this repository using a published Hannes executable. The installed
governor selects and executes `.hannes` checks; builds under `target/` are candidates
under test and never become the governor. There are no reviews or provider calls. The required empty `reviews/` directory
is retained with a `.gitkeep` file.

## Local setup

Install the development tools and dependencies explicitly, then install a published
governor. These are setup operations, not actions performed by Git hooks:

```sh
mise trust
mise install
npm ci --ignore-scripts
cargo +stable fetch --locked
mise deploy
export PATH="$HOME/.local/bin:$PATH"
hannes pass auf
mise check
```

`mise deploy` installs the latest published release, without compiling local source.
Python 3.11+, Git, stable Rust with rustfmt/Clippy, and the Node version in
`mise.toml` are required locally. Prepare Cargo dependencies while online before
using hooks. Hooks set Cargo and npm offline mode; they never install or download
the governor. Missing tools and dependencies block the operation with an error.

`mise check` delegates to the `pre-push` profile through `scripts/run-gate.py`.
Direct `hannes mach was` selects the same profile. Individual `mise format`,
`mise lint`, `mise test`, `mise smoke` and `mise docs:check` tasks remain available.
Format is an explicit editing command; verification only checks formatting.

## Profiles and checks

All profiles are gates with explicit required check IDs. A missing, disabled,
failed or unexecuted required check prevents success. Hannes validates the complete
configuration before running any profile. No profile inherits another profile;
shared check IDs refer to the same definitions.

| Profile | Checks |
| --- | --- |
| `pre-commit` | Rust formatting; documentation links and specification-to-test references |
| `commit-msg` | Final message against `.githooks/commit-policy` |
| `pre-push` (default) | Pre-commit checks; documentation-checker, governance, hook, deployment and release integrity regression tests; semantic release rules/lifecycle; Clippy; all Rust tests; optimized build; executable smoke test |
| `ci-native` | Rust formatting, Clippy, all Rust tests, optimized native build and smoke test |
| `ci-tooling` | Documentation validation; documentation-checker, governance, hook, deployment and release regression tests; semantic release rules/lifecycle; commit ranges and PR title; actionlint |
| `ci-msrv` | Compile-check with Rust 1.85 |
| `ci-audit` | cargo-audit and npm audit with high/critical blocking |
| `release-artifacts` | Complete archive set, hashes, checksums, contents, permissions, planned version and source identity |

The check files in [`.hannes/checks`](../.hannes/checks) define commands, order and
timeouts. Profiles in [`.hannes/profiles`](../.hannes/profiles) define membership.
Rust tests include acceptance, documentation and frozen compatibility contracts.
Release/deploy tests use local fixtures and never publish or download a release.

Run or inspect an individual gate from the Git working-tree root:

```sh
hannes mach was --profil pre-commit
hannes mach was --profil pre-push --plan
hannes mach was --profil ci-native
hannes was ist phase --details
```

CI-only profiles additionally need their prepared environment: actionlint, Rust
1.85, cargo-audit, npm dependencies and advisory feeds as appropriate. Audits fail
when their feeds are unavailable. The release artifact gate requires
`EXPECTED_RELEASE_VERSION`, `GITHUB_SHA` and the collected `dist/` directory.

## Git hooks

Install explicitly with `mise hooks:install`. Setup preserves custom hooks and
`core.hooksPath` settings rather than overwriting them. It installs pre-commit,
commit-msg and pre-push delegates. The final message is checked at commit-msg,
where it is available, using the same standalone checker as CI. All hooks block
on failure; none formats source or edits the Git index.

Hooks verify the working tree, including unstaged files. Partial staging can
therefore commit different content from the locally checked files. Pre-push does
not certify every outgoing commit or ref. Hooks are bypassable; clean CI checkouts
remain authoritative. Exact staged-index and pushed-commit verification are future
product work, not behavior supplied by these hooks.

## Published version and Renovate

[`.hannes-version`](../.hannes-version) is the single exact CI/release version pin.
The shared setup action uses the existing release installer with that tag,
validates the downloaded archive checksum and executable version, and installs
outside the checkout. It never resolves `latest` or builds the governor. Local
users may use the latest installed published release. The runner rejects candidate
executables inside the checkout and verifies CI's executable version against the pin.

The regex custom manager in [`renovate.json`](../renovate.json) extracts that pin
and uses Renovate's `github-releases` datasource for `christianpflugradt/hannes`.
Updates are ordinary dependency PRs with automerge disabled. Existing dependency
managers remain unchanged. The offline governance test verifies extraction and
configuration wiring; an actual Renovate run is needed to verify release discovery
and PR creation end to end.

New configuration features can only become required after the pinned published
release supports them. Publish first, then adopt the new pin and configuration in
the same coherent change. A format-breaking update may require edits to check
commands in addition to accepting Renovate's version update. Every profile must
validate and plan with the proposed governor before merging the update.

## CI, release operations and evidence

Push/PR CI runs the native gate on Linux x64 plus parallel tooling, MSRV and audit
jobs. Releases reuse these jobs and run the native gate on all four supported
hosts. Tool installation, dependency preparation and uploads belong to Actions;
check selection and pass/fail decisions belong to Hannes.

Release version stamping precedes verification. Packaging uses the tested candidate,
checks its reported version and records its binary/archive hashes and source SHA.
After collection, the artifact profile runs the existing release verifier. The
publication plugin repeats integrity/source guards immediately before publishing.
Planning, stamping, packaging, checksum creation and publication remain release
operations; none recursively invokes an aggregate quality task.

Each CI gate uploads `.hannes/reports/last.json` and `execution.json` with a unique
job/target artifact name, including on check failure when a report exists. The
unchanged schema-2 report records the selected profile, checked HEAD, checks and
results. The companion execution record adds the governor version and binary
SHA-256, exit code and the corresponding checked commit. It avoids requiring a new
report format from an already published executable. Local direct CLI invocations
write the ordinary report; the wrapper adds the companion record.
