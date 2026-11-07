## [0.8.0](https://github.com/christianpflugradt/hannes/compare/v0.7.0...v0.8.0) (2026-11-07)

### ⚠ BREAKING CHANGES

* **config:** External rule sets must replace rule IDs in rules with repository-relative rule directory paths. Local rule set references remain unchanged.

### Features

* **config:** share external rules across rule sets ([c2633bf](https://github.com/christianpflugradt/hannes/commit/c2633bf96bf31f59e5152d5271bcb85a7f3ef518))

## [0.7.0](https://github.com/christianpflugradt/hannes/compare/v0.6.0...v0.7.0) (2026-11-07)

### Features

* **config:** add rule parameters and execution error handling ([b4636da](https://github.com/christianpflugradt/hannes/commit/b4636da30ed3e2a760b2d1c1fc5f827cda33673e))

## [0.6.0](https://github.com/christianpflugradt/hannes/compare/v0.5.0...v0.6.0) (2026-11-07)

### ⚠ BREAKING CHANGES

* **git:** remove configurable hook mappings
* **config:** adopt unversioned self-contained rules

### Features

* **config:** add standard execution points and import mappings ([4572bb0](https://github.com/christianpflugradt/hannes/commit/4572bb038095a08e87adfbe37b2caa3245376919))
* **config:** adopt unversioned self-contained rules ([ce94514](https://github.com/christianpflugradt/hannes/commit/ce9451451245a23ba06eb74cf87c42b509139be1))
* **config:** support external Git rule sets ([9e92188](https://github.com/christianpflugradt/hannes/commit/9e92188286ed8e18649a0c6d462f26f381036c6b))

### Bug Fixes

* **cli:** skip hook reconciliation for explicit execution points ([982e248](https://github.com/christianpflugradt/hannes/commit/982e248df8a1444643c0e889a8ad4f5aac6378cf))
* **git:** remove configurable hook mappings ([13c8f07](https://github.com/christianpflugradt/hannes/commit/13c8f0784688648cc3a81e923eabdd1539bc6adb))
* **release:** prepare offline publication hooks ([a4745de](https://github.com/christianpflugradt/hannes/commit/a4745de1f6a8c3cb20db1029ec7b7d6ba3afb143))

### Maintenance

* **ci:** migrate self-governance to execution points ([5aca592](https://github.com/christianpflugradt/hannes/commit/5aca5925b3b4ae7d8a8a25e662c6543e5e20175d))

## [0.5.0](https://github.com/christianpflugradt/hannes/compare/v0.4.0...v0.5.0) (2026-11-06)

### Features

* **cli:** add explicit auto-approval for reconciliation ([3cdd489](https://github.com/christianpflugradt/hannes/commit/3cdd48936678c584c2907b382b31c8958c426523))

### Maintenance

* **deps:** update rust crate toml to v1.2.1 ([#17](https://github.com/christianpflugradt/hannes/issues/17)) ([3372e29](https://github.com/christianpflugradt/hannes/commit/3372e293f7a2cc3494f7058f0c44a3bda7858604))

## [0.4.0](https://github.com/christianpflugradt/hannes/compare/v0.3.0...v0.4.0) (2026-11-06)

### ⚠ BREAKING CHANGES

* **config:** replace profiles with rule sets and execution points

### Features

* **config:** replace profiles with rule sets and execution points ([8a5cd78](https://github.com/christianpflugradt/hannes/commit/8a5cd78ea9afb25a64fa7dab8c04a696bd5505d1))

### Maintenance

* **deps:** pin Hannes governor to v0.3.0 ([94cbb06](https://github.com/christianpflugradt/hannes/commit/94cbb06a583401b2c6142c5e89aeff1d9d6754ab))

## [0.3.0](https://github.com/christianpflugradt/hannes/compare/v0.2.0...v0.3.0) (2026-11-06)

### Features

* **git:** reconcile declared state and manage Git hooks ([95e8e1c](https://github.com/christianpflugradt/hannes/commit/95e8e1cc4fdf3e6efa41559d3ab236b6b0cb65c7))

### Build System

* **ci:** govern quality gates with published Hannes ([44a1d65](https://github.com/christianpflugradt/hannes/commit/44a1d6527a8a55b04123839cae0964523ca4c98e))

## [0.2.0](https://github.com/christianpflugradt/hannes/compare/v0.1.0...v0.2.0) (2026-11-06)

### ⚠ BREAKING CHANGES

* **checks:** configure commands as strings

### Features

* **checks:** configure commands as strings ([a2db259](https://github.com/christianpflugradt/hannes/commit/a2db2592ab1158d834988caffbc81ee2dc5f0bc7))

### Maintenance

* **deps:** update actions/setup-python action to v7 ([#14](https://github.com/christianpflugradt/hannes/issues/14)) ([b17ba77](https://github.com/christianpflugradt/hannes/commit/b17ba771bff5c122ea4a2d6cf3d599afb1c71b47))
* **deps:** update dependency python to v3.15.2 ([#10](https://github.com/christianpflugradt/hannes/issues/10)) ([ded8d9f](https://github.com/christianpflugradt/hannes/commit/ded8d9f3e48c071cc462e7530578b085e0ca8be1))
* **deps:** update node.js to v26 ([#16](https://github.com/christianpflugradt/hannes/issues/16)) ([80a5c5e](https://github.com/christianpflugradt/hannes/commit/80a5c5eb224fbcf1fdca1c4e16c7dbdc10185dd1))
* **deps:** update rust crate toml to v1.2.0 ([#11](https://github.com/christianpflugradt/hannes/issues/11)) ([2a4f9e3](https://github.com/christianpflugradt/hannes/commit/2a4f9e3d3af4614944fa52c7594f091ecf78a8cd))

### Build System

* **mise:** deploy the latest matching release binary ([ebed41c](https://github.com/christianpflugradt/hannes/commit/ebed41cede401561b492f13d5938e952eebce9ce))

# Changelog

## 0.1.0 (2026-11-06)

Initial development release. The erroneous 1.0.0 publication was withdrawn;
its source history is preserved.

- Define version-controlled governance rules as deterministic checks and AI reviews.
- Select rules with profiles, preview runs offline and enforce required coverage.
- Use the existing Codex login with configurable model, bounded context and MCP access.
- Save structured verdicts, evidence, check output and available token usage.
- Detect stale verification results through Git HEAD and file-content snapshots.
- Provide native macOS Apple Silicon/Intel and Linux x64/ARM64 downloads.

Subsequent release entries are generated by Semantic Release. License: Unlicense.
