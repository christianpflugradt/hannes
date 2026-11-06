# Hannes repository checks

This is the repository's real configuration, governed by an installed published
Hannes release. It uses configuration version 2, explicit gate membership and no
reviews. `pre-push` is the default profile.

```sh
hannes pass auf
hannes mach was --profil pre-commit --plan
hannes mach was
```

See [repository quality gates](../docs/self-governance.md) for installation,
profiles, check coverage, reports, CI pinning and hook limitations. The general
[configuration reference](../docs/configuration.md) describes the supported fields.
Reports remain ignored; do not commit them.
