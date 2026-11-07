# External Git Rule Sets

An import selects one directory at one Git revision. The selected directory is
an independent governance root. The [configuration reference](configuration.md)
lists consumer fields, Rule definitions and execution semantics.

## A shared repository

```text
hannes-governance/
  repository/
    config.toml
    checks/readme/rule.toml
    checks/readme/index.sh
  releasable-software/
    config.toml
    checks/changelog/rule.toml
    checks/changelog/index.sh
```

`repository/config.toml`:

```toml
id = "repository"
description = "General repository governance"
run_at = ["pre-push", "check"]
rules = ["repository-readme"]
```

`repository/checks/readme/rule.toml`:

```toml
id = "repository-readme"
description = "Require the consumer README"
[check]
entrypoint = "index.sh"
interpreter = "sh"
```

`repository/checks/readme/index.sh`:

```sh
test -s README.md
```

`releasable-software/config.toml`:

```toml
id = "releasable-software"
description = "Release documentation"
run_at = ["pre-push", "release"]
rules = ["release-changelog"]
```

`releasable-software/checks/changelog/rule.toml`:

```toml
id = "release-changelog"
description = "Require the consumer changelog"
[check]
entrypoint = "index.sh"
interpreter = "sh"
```

`releasable-software/checks/changelog/index.sh`:

```sh
test -s CHANGELOG.md
```

The consumer declares custom points and imports these directories:

```toml
project = "My_App"
[execution_points]
custom = ["release"]

[[imports]]
git = "https://github.com/company/hannes-governance.git"
path = "repository"
ref = "v1.2.0"

[[imports]]
git = "https://github.com/company/hannes-governance.git"
path = "releasable-software"
ref = "v1.2.0"
```

Each selected root defines exactly one Rule Set in config.toml and discovers Rules
from immediate checks/ and reviews/ directories. It has no extra .hannes directory,
rule-sets directory, consumer settings or nested imports. The set can reference
only its own Rules. All references are validated, including disabled Rules.
IDs need only be unique within their own scope. No source or consumer can override
another scope's Rules. Removing an import stops using that source.

Per-import `[imports.execution_points]` maps external custom names to custom names
already declared by the consumer. The table belongs only to the preceding import.
Resolve inheritance first, map effective assignments once, then deduplicate and
namespace Rules. Built-ins (`pre-commit`, `commit-msg`, `pre-push`, `check`) cannot
appear on either side. Unknown sources/targets and invalid names are errors.
Unmapped names stay unchanged and must be consumer-known. Separate imports may
map the same source differently; multiple sources may share one target. Mapping
never edits source/cache files or changes import identity, and works offline.
See the [complete three-import example](../README.md#import-rule-sets-from-git).

Each import's `rule_parameters.<rule-id>` tables supply only that binding's
values. External declarations omit `value`. File parameters refer to existing
files in the consumer; bundled assets still belong to the source rule. Repeated
imports execute separately and retain distinguishable IDs in plans and reports;
repeated references within one binding execute once. See
[parameter declarations and delivery](../README.md#rule-parameters).

## Refresh, storage and provenance

Host Git fetches exact selectors into shared bare object storage and resolves them
to commits before loading definitions. Tags are peeled to commits; a full hash
must identify that exact commit. Failed selectors never fall back to another ref.
Already available objects are reused. A cached full commit may be available
without remote contact.

Default cache roots are:

- macOS: `$HOME/Library/Caches/hannes`.
- Linux: `$HOME/.cache/hannes`.
- An absolute `XDG_CACHE_HOME` overrides these, using `XDG_CACHE_HOME/hannes`.

The layout is `git/REPOSITORY_DIGEST/COMMIT/contents`, with an integrity receipt
beside contents. Imports at the same commit share this checkout, including imports
through different refs. Repository selectors are hashed rather than persisted in
cache metadata. Equivalent URL aliases are separate identities. HTTP credentials
in URLs and URL queries/fragments are rejected. Hannes issues no authentication
prompts; configure host Git credentials or SSH access.

Project resolution state lives at `hannes/external-rule-sets.json` inside the
project's Git directory (per worktree). It records repository digest, configured
ref, selected directory and resolved commit. It stays outside tracked files and
working-tree snapshots. Refresh atomically replaces complete state after all
local and imported rules validate. Removed references are dropped. Cached
revisions remain available; no automatic eviction is performed.

OS locks serialize writers, including different projects sharing a cache. Locks
release on process exit. Readers check integrity receipts, file kinds and
executable flags before use; partially populated revisions are never published.
Receipts detect accidental corruption, not an attacker who can rewrite both
contents and receipts. State and reports are local, unsigned evidence.

Imported IDs have the form `git:BINDING_DIGEST:DIRECTORY:OCCURRENCE:ORIGINAL_ID`.
The binding digest includes the repository selector, ref, normalized directory
and per-import parameter values. Repeated identical bindings have separate
one-based occurrence numbers. Execution-point mappings do not enter the digest.
Definition resolution and cached checkouts remain shared across bindings. Local IDs are
unchanged. Plan/report origins retain the configured ref and full resolved commit
without internal cache paths. Saved input freshness compares the consuming working
tree; inspect provenance to see which external revision a report evaluated.

## Source resources and trust

Checks keep the consuming project's working directory. HANNES_RULE_DIRECTORY
locates bundled resources inside the executing Check directory. Explicit
interpreters receive absolute entrypoint paths. Review prompts load from their
own Rule directories; context paths and evidence refer to the consumer.

Selected roots, Rule directories, definitions, entrypoints and prompts reject
symlinks, parent traversal and unexpected file types. Only designated definitions
are interpreted; supporting TOML is data. Git retrieval disables downloaded hooks
and initialization scripts and never initializes submodules. Host credential
helpers and SSH configuration remain trusted integrations.

Checks execute with the Hannes user's permissions. Resource confinement does not
prevent scripts from accessing other paths, credentials or the network. Import
executable Rules only from trusted sources. There is no process sandbox.

## Recovery and limits

Missing or changed resolution state, missing cached revisions or integrity failures
require `hannes pass auf`. A failed refresh preserves the last valid state. For
corrupt immutable entries, remove the affected repository-digest cache directory
and refresh; Hannes never silently replaces validated revisions. Authentication
comes from host Git/SSH; Git stderr is suppressed to avoid credential disclosure.

Fetches have a 60-second deadline; local Git operations and lock waits have
30-second deadlines. Cached repositories allow 8 MiB/file, 128 MiB total,
100,000 entries and 128 directory levels. Metadata is limited to 64 KiB/file;
the combined configuration allows 256 Rules and 256 Rule Sets. Resolution state
is limited to 64 KiB; refs to 1024 bytes. Shared-cache locking currently uses Unix
OS locks; external imports on Windows remain unsupported.
