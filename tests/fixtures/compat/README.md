# Frozen compatibility fixtures

These fixtures come from executing historical Hannes source, not serializing the
current report types:

| Fixture | Source | Contract |
| --- | --- | --- |
| [v1](v1/source.txt) | `29be82e3027c36509295f431f824fae4b2efe1b3`, immediately before profiles | Configuration 1, report 1, no plan |
| [v2](v2/source.txt) | `717a1102ca11b2c23c40e7cf9b7372a5a2d6c329`, release 1.0.0 | Configuration 2, report 2, required local profile |

Capture procedure: archive each pinned source, build its locked native executable,
initialize a temporary Git project named `My_App`, and run its initialized check
with all reviews disabled. For v2, explicitly select version 2 and a `local` gate
requiring `always-ok` before verification. No model calls or credentials are used.

The configuration files and review criterion are historical scaffold contents.
Generated guidance is omitted. Test setup installs the configuration under the
current `.hannes` directory: the directory rename is explicit, not an automatic
configuration migration. Old paths inside saved snapshots remain historical and
need not describe today's working tree.

For portability, repository identity is `__REPOSITORY__` and is substituted per
test. Timestamp, command duration and Git-version output are normalized; no report
fields are added from current code. `report-minimal.json` is a frozen variant of
v1 omitting the two documented optional fields `provider_usage` and `checks_only`.

`v1/report-review.json` additionally captures an enabled review through a temporary
fake Codex executable returning a grounded PASS. No real provider is invoked. The
same optional counters are omitted to exercise legacy review rendering.

Do not regenerate these files from current structs to fix a failing test. Preserve
existing fixtures, add fixtures for new supported versions, and specify intentional
compatibility changes before implementation. Tests must not run Git-history builds
or fetch historical sources: these files are the permanent offline evidence.
