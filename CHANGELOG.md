## [0.11.0](https://github.com/christianpflugradt/hannes/compare/v0.10.1...v0.11.0) (2026-11-07)

### Features

* **provider:** execute generic CLI reviews with runtime requirements ([3c10d90](https://github.com/christianpflugradt/hannes/commit/3c10d90be3cb11524d9952b88a1bb2c348938ced))
* **provider:** resolve named configurations across rule sets ([3e19086](https://github.com/christianpflugradt/hannes/commit/3e19086f406a21a2b541b7b0aadc68112a92b27a))
* **reviews:** select reviews with file-change triggers ([6b53d42](https://github.com/christianpflugradt/hannes/commit/6b53d42ea8f2a20ed7d40d50633784c7e25a916e))

## [0.10.1](https://github.com/christianpflugradt/hannes/compare/v0.10.0...v0.10.1) (2026-11-07)

### Bug Fixes

* **checks:** guard direct execution against recursive checks ([9d1628b](https://github.com/christianpflugradt/hannes/commit/9d1628b7eaa8a225c5a35f5ac11a04eb15bbbd64))
* **cli:** keep JSON plan diagnostics on the current schema ([adaf1cb](https://github.com/christianpflugradt/hannes/commit/adaf1cba3082f4bbe8932a79e710f6a17406cdcb))
* **provider:** disable the multi-agent feature in reviews ([b1f8465](https://github.com/christianpflugradt/hannes/commit/b1f846543ce0d696636527dce025fc2ec94a2ddd))
* **release:** download pinned assets without API lookups ([9119026](https://github.com/christianpflugradt/hannes/commit/9119026fa5868b4b5b43efb1ee44a8890e90793d))
* **release:** prepare all managed push hook dependencies ([55dfae5](https://github.com/christianpflugradt/hannes/commit/55dfae543d0a7f84b30f4789fdd6b971b2aafee4))

### Maintenance

* **checks:** import repository-conventions license rule set ([b21e287](https://github.com/christianpflugradt/hannes/commit/b21e2870a2eb3b6ea3080b2c2bbf29d8c92a17f2))
* **deps:** update christianpflugradt/hannes digest to b21e287 ([#19](https://github.com/christianpflugradt/hannes/issues/19)) ([c1bedc7](https://github.com/christianpflugradt/hannes/commit/c1bedc737c8d8f0a746cd964923c67ac5d60d976))
* **deps:** update christianpflugradt/hannes digest to c1bedc7 ([#20](https://github.com/christianpflugradt/hannes/issues/20)) ([4720f11](https://github.com/christianpflugradt/hannes/commit/4720f11f0047c458bc7e03be9e6c7241611d601a))
* **git:** ignore local IDE files and generated hooks ([9d23526](https://github.com/christianpflugradt/hannes/commit/9d23526d0a8afc5a23fedeca480886ae97655988))

### Code Refactoring

* **checks:** simplify local rules and consolidate Python tests ([f25d34a](https://github.com/christianpflugradt/hannes/commit/f25d34aa2acc83d8c1861a6081ef9bdcc48d9a44))

## [0.10.0](https://github.com/christianpflugradt/hannes/compare/v0.9.0...v0.10.0) (2026-11-07)

### Features

* **checks:** execute native commands without script launchers ([e6ccc50](https://github.com/christianpflugradt/hannes/commit/e6ccc503ab8dc63a8ba8b14ca0b0264b9a19f3e3))
* **checks:** support shared source-relative entrypoints ([df7a65f](https://github.com/christianpflugradt/hannes/commit/df7a65fbd971cb3a60bc4d434aa2e7fdc3e0809a))
* **ci:** add the public Hannes composite Action ([2d3cac2](https://github.com/christianpflugradt/hannes/commit/2d3cac2324ce54f076c7090e5bf80969eee4b37e))
* **cli:** streamline rule output and CI checks ([cb7986c](https://github.com/christianpflugradt/hannes/commit/cb7986ce9e32da8e7505a0285c5159f2d11733d9))

### Code Refactoring

* **checks:** rely on Git hooks for local validation ([5cb1149](https://github.com/christianpflugradt/hannes/commit/5cb11494ca276694663991d95fa5b84842ed8d74))

## [0.9.0](https://github.com/christianpflugradt/hannes/compare/v0.8.0...v0.9.0) (2026-11-07)

### Features

* propagate execution-point context to rules ([10d3a23](https://github.com/christianpflugradt/hannes/commit/10d3a23b069d8617f9a06b3c324c9cffe60f5c38))

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
