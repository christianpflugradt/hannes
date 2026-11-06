# Rule profiles and execution plans

Status: implemented for configuration version 2. Version 1 remains supported;
see [configuration.md](configuration.md). Explicitly opt in to version 2 before
using the new fields; version-1 projects reject them.

Implementation tracking: [TODO 1](todo/01-profiles-and-plans.md). The ordered
[roadmap](todo/README.md) records subsequent work and agreed decisions.

This specification defines selection and planning before adding more provider
features. Checks remain deterministic processes; reviews remain LLM requests with
structured results. Both are rules. A profile selects rules; it neither defines
new rules nor grants provider permissions.

## Configuration

Version 2 adds optional `tags` to both check and review metadata. Tags are unique,
case-sensitive strings matching `[a-z0-9][a-z0-9_-]{0,31}`, with at most 32 per
rule; omission means an empty list. Existing rule IDs and other fields retain
their validation rules. IDs remain unique across checks and reviews.

```toml
# Additional fields in a check or review definition
tags = ["local", "critical"]
```

Profiles live in `.hannes/profiles/*.toml`, one definition per file:

```toml
# .hannes/profiles/local.toml
id = "local"
rules = ["unit-tests"]
include_tags = ["local"]
gate = false
input = "worktree"
```

```toml
# .hannes/profiles/verify.toml
id = "verify"
rules = []
include_tags = ["critical"]
gate = true
input = "worktree"
```

Profile IDs use the tag syntax, must be unique, and cannot be `all`. `rules` and
`include_tags` default to empty lists; `gate` defaults to false and `input` to
`worktree`. Unknown fields, duplicate selectors, unknown rule IDs and tags that
occur in no rule definition are configuration errors. All definitions, including
disabled rules and unused profiles, are validated before execution.

```toml
# .hannes/config.toml
version = 2
project = "My_App"
max_display_files = 5
default_profile = "local"
```

Omitting `default_profile` selects the reserved implicit profile `all`: all enabled
rules, worktree input, no gate. Version-1 projects retain that behavior without
migration. Explicit `--profil NAME` overrides the configured default. Version-2
features require an explicit version change; never reinterpret unknown fields in
version 1. No automatic configuration migration is part of this change.

## Selection and completeness

A profile selects the union of its explicit rule IDs and all rules matching
**any** `include_tags` value. Deduplicate by rule ID. Run selected enabled checks
in ID order, then selected enabled reviews in ID order. File enumeration order
must not change the result. There is no inheritance, exclusion language or
per-rule hook configuration in this version.

For ordinary profiles, selected disabled rules are omitted and explained in the
plan. For a gate, every selected rule is required: selecting a disabled rule is
an error. An empty gate is also an error. This prevents silently removing a gate
requirement by disabling its definition. A new rule tagged `critical` therefore
joins the example `verify` gate automatically.

All executed rules remain blocking; `gate = false` does not turn failures into
warnings. Gate mode additionally requires complete selection and execution:
every required rule must actually PASS against the recorded, stable input.
FAIL, ERROR, NOT_RUN or incomplete snapshot evidence prevents gate success.

`--geiz-ist-geil` applies after profile selection. For ordinary profiles it omits
reviews, records why, and labels results as checks-only. It never starts Codex,
performs provider discovery or connects to MCP servers. If it would omit a
required gate review, reject the invocation before executing any check. A gate
containing only checks can use the flag. An empty ordinary selection is explicit
and proves no rule passed; preserve the existing special message for projects
with no rule definitions at all.

## CLI and offline planning

These flags are supported by `hannes mach was`:

```sh
hannes mach was --profil local --plan
hannes mach was --profil local --plan --json
hannes mach was --profil local --geiz-ist-geil
hannes mach was --profil verify
```

`--plan` validates configuration, resolves selection and prepares bounded local
context and input identity. It executes no check, provider CLI, model request or
MCP connection, and performs no network access. It does not overwrite reports,
install hooks, write project files or mutate Git state. Provider availability and credentials
are explicitly **not checked**. `--json` initially requires `--plan`.

The human-readable plan shows:

- Effective profile and whether chosen explicitly or through a default; gate
  mode, input kind, HEAD and snapshot fingerprint/coverage.
- Ordered scheduled rules with kind, ID, description, tags and selection reasons;
  selected-but-omitted definitions with reasons (`disabled` or `checks_only`).
- Required gate IDs, selected scope and whether selection is complete. A
  checks-only plan must not look like a complete review verification.
- Declared provider/model settings and MCP allow list, without credentials;
  declared defaults are labelled unresolved if their effective value is unknown.
- Review entry paths, requirement sections, prepared context byte counts and
  applicable existing limits/truncation warnings. Do not print full source text.
- Planned model request count: zero without scheduled reviews, otherwise one
  shared request under the current batching contract. No invented token or cost
  estimate; a plan is not usage telemetry.

JSON uses a separately versioned `plan_schema_version = 1`, stable ordering and
explicit `planned` or `blocked` status. Include machine-readable omissions and
blocking reasons, never fabricated rule verdicts. Return 0 for an executable
plan and 2 for configuration errors, incomplete input, unsupported input kinds or
a blocked gate. Invalid configuration produces a structured diagnostic when
`--json` was requested, rather than pretending a valid plan exists.

Normal execution uses the same planning logic before starting checks. A
`plan_id` hashes canonical semantic inputs: effective profile, rule definitions
and instructions, prepared context, provider settings and input identity. Exclude
timestamps, absolute checkout paths and runtime telemetry. Rebuild and validate
the plan for every execution; a previously displayed plan does not authorize or
certify a later changed tree. There is no saved-plan execution mechanism yet.

Metadata TOML is canonicalized by decoded fields, IDs and sorted set selectors;
metadata filenames/formatting do not affect plan identity. All other in-scope
file hashes and HEAD form the code identity. The raw snapshot fingerprint remains
separate and detects exact metadata changes for execution stability and freshness.

## Reports and integration boundaries

Report schema 2 records the plan ID, effective profile, gate mode, input
identity, ordered scheduled rules, omissions and required gate IDs. Distinguish
selection completeness from actual execution: a full plan can still leave
reviews NOT_RUN when checks fail. Preserve existing rule statuses, short AI
summaries, detailed findings and actual provider usage counters.

`was ist phase` renders the saved run's profile and scope, not today's default
selection. Old schema-1 reports remain readable as legacy, unprofiled reports;
they cannot certify a new profile gate. A scoped PASS must never be presented as
verification of unselected rules.

Only `worktree` input is specified for the first implementation. It retains the
current full-working-tree stability checks. Future `index` and `commit` inputs
must be rejected until implemented; never substitute a worktree. In particular,
a local worktree gate is not proof about staged or pushed commits. Hooks choose
a profile explicitly through version-2 `hooks.git` mappings; see
[Git reconciliation](usage.md#optional-git-hook). Profiles select rules;
reconciliation maintains hooks. CI authentication and deployment integration remain separate work.

Provider-wide MCP permissions and the existing shared review call are unchanged.
Per-review MCP subsets and separate sessions, true diff/snapshot review scopes,
additional runtime/token budgets and result caching are subsequent steps.
Profiles cannot silently enable any of them.

## Acceptance criteria for implementation

1. A version-1 project schedules the same rules in the same order as today.
2. ID/tag union deduplicates rules; filenames and selector order do not affect
   scheduling or plan identity. New matching rules join the next plan.
3. Unknown profiles, IDs, tags, duplicate selectors and invalid disabled
   definitions fail before checks or provider activity.
4. Ordinary profiles explain disabled omissions; gates reject disabled required
   rules and empty selections.
5. Checks-only ordinary runs omit every review and make zero provider calls;
   checks-only gates requiring reviews reject before the first check.
6. Plan rendering never executes checks/provider discovery, changes Git or
   overwrites a report. JSON and text describe the same selection.
7. Repeated planning over identical semantic inputs has the same plan ID;
   changing selected criteria, context, provider settings or input changes it.
8. A failing check leaves planned reviews NOT_RUN; no gate PASS is possible.
9. Status preserves historical scope; legacy reports do not satisfy profile
   gates. Unsupported `index`/`commit` inputs never fall back to worktree.
