# External Git rule sets

A reference selects one directory at one Git revision. A repository is a
distribution mechanism, not another governance entity. The
[README reference](../README.md#import-rule-sets-from-git) lists all import fields.

## A shared repository

This complete source example contains three independent sets:

```text
repository/
  ruleset.toml
  rules/readme.toml
software/
  ruleset.toml
  rules/changelog.toml
rust/
  ruleset.toml
  rules/formatting.toml
```

For repository/ruleset.toml:

```toml
[[rule_sets]]
id = "repository"
enabled = true
rules = ["readme"]
run_at = ["pre-push"]
```

For repository/rules/readme.toml:

```toml
[[checks]]
id = "readme"
description = "Require the consuming project's README"
enabled = true
command = "test -f README.md"
timeout_seconds = 5
```

For software/ruleset.toml:

```toml
[[rule_sets]]
id = "software"
enabled = true
rules = ["changelog"]
run_at = ["pre-push"]
```

For software/rules/changelog.toml:

```toml
[[checks]]
id = "changelog"
description = "Require the consuming project's changelog"
enabled = true
command = "test -f CHANGELOG.md"
timeout_seconds = 5
```

For rust/ruleset.toml:

```toml
[[rule_sets]]
id = "rust"
enabled = true
rules = ["formatting"]
run_at = ["check"]
```

For rust/rules/formatting.toml:

```toml
[[checks]]
id = "formatting"
description = "Check Rust formatting"
enabled = true
command = "cargo fmt --all -- --check"
timeout_seconds = 60
```

The consumer imports these directories as shown in the README and declares
`[execution_points] custom = ["check"]`. File names are arbitrary. Only selected
subtrees are parsed as governance configuration. Duplicate imports of the same
repository selector, normalized directory and reference are errors.

Sets contain rule IDs, never set IDs. They cannot reference sibling or consumer
rules. The existing resolver validates all references and disabled definitions
inside each boundary. An active rule referenced only by a disabled set still
needs its own assignment, exactly as with local definitions.

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

Imported IDs have the form `git:IMPORT_DIGEST:DIRECTORY:ORIGINAL_ID`. The digest
includes repository selector, ref and normalized directory. Local IDs are
unchanged. Plan/report origins retain the configured ref and full resolved commit
without internal cache paths. Saved input freshness compares the consuming working
tree; inspect provenance to see which external revision a report evaluated.

## Source resources and trust

Checks retain the consuming project's working directory. For a script stored
with the imported set, use an explicit launcher, for example:

```toml
command = "sh -c 'exec sh \"$HANNES_RULE_SET_SOURCE/scripts/check.sh\"'"
```

This accesses a source script while keeping the consumer as the working directory.
The variable is supplied only to imported checks. Hannes performs no implicit
argument rewriting. Review instructions resolve from the defining document,
contained within the selected subtree. Review entry/context paths and evidence
refer to consumer files.

Selected paths and instruction resources reject parent traversal and symlink
components. Discovery retains its normal hidden/cache-directory exclusions and
nesting limit. Git retrieval uses an empty initialization template, disables hooks
and never initializes submodules. Materialization disables system/global Git
filters and executes no checkout hooks. Host credential helpers and SSH
configuration remain trusted integrations.

Commands, including explicit shells and source scripts, execute as the Hannes
user. They can modify files and access the environment. Validation and retrieval
execute none of them. Hannes provides no command sandbox.

## Troubleshooting and limits

- Missing state or changed import: run `hannes pass auf`.
- Missing cached revision: reconnect and run `hannes pass auf`.
- Authentication failure: configure host Git credentials or SSH access and retry.
  Raw Git stderr is suppressed to avoid exposing credentials.
- Repository unavailable: check URL, connectivity and access permissions.
- Invalid reference: check the exact branch/tag/full commit. Unavailable or
  non-commit references fail.
- Missing directory: check `path` at the selected revision, including case.
- Invalid set/reference: ensure exactly one set, no nested imports, boundary-local
  rule references and centrally declared custom execution points.
- Corrupt cache: remove the affected repository-digest directory under the cache
  root, then run `hannes pass auf`. Valid immutable entries are never overwritten.
- Busy/permission failure: let the other process finish and check cache/Git
  directory permissions. Lock acquisition is bounded to 30 seconds.

Fetches have a 60-second deadline; local Git operations have a 30-second deadline.
Cached repositories use the snapshot bounds: 8 MiB per file, 128 MiB total,
100,000 entries, with 128 directory levels. Configuration retains its 16-level
discovery bound and 256-document/rule/set limits; the combined configuration has
at most 256 rules and sets. Resolution JSON is limited to 64 KiB; refs to 1024 bytes.

Supported selectors include HTTPS, HTTP, `ssh://`, SCP-style SSH, `git://`,
`file://` and local repository paths. Custom remote-helper schemes (including
`ext::`), URL queries/fragments and repositories containing submodules are
unsupported. External sets use Unix locking on supported macOS/Linux targets.

