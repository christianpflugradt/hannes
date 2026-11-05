# Publishing the first release

Nothing in local initialization or development creates remote services or commits.
Review and commit the source yourself, create an empty GitHub repository, and add
its remote when you are ready. Do not include `.codex/`, `.codebase-memory-local/`
or reports; the repository ignore rules exclude these local development files.

After pushing reviewed source and confirming CI is green, verify the package
version in Cargo.toml matches the intended tag. The release workflow checks this.
Then explicitly publish the tag yourself:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The tag workflow builds and tests native executables on `macos-15` (ARM64),
`macos-15-intel` (x64), `ubuntu-24.04` (x64 GNU/Linux) and `windows-2025` (x64 MSVC).
These labels follow GitHub's [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
Each archive is named with its Rust target triple and includes README and LICENSE.
One SHA256SUMS file covers all archives; standalone README/LICENSE are also attached.
Verify downloaded archive hashes using `shasum -a 256` or `sha256sum` (PowerShell:
`Get-FileHash -Algorithm SHA256`). This verifies integrity, not a signed identity.

CI runs format, Clippy and tests on the same host matrix, without AI credentials.
The release job is the only job granted `contents: write`; build jobs are read-only
and checkout does not persist credentials. Actions are pinned to full commit SHAs
from their stable v4 tags. Update pins deliberately. Release creation uses GitHub's
preinstalled `gh` CLI and the scoped job token. No separate release service needed.
Tag values enter shell commands through environment variables, not interpolation
into shell code. Existing releases are not overwritten automatically.

No hosted workflows have been run as part of local implementation. The macOS
ARM64 build is verified locally; the other platforms are configured for hosted
verification. Downloaded native binaries still require Git and, for active reviews,
a compatible Codex CLI with local authentication. No code signing/notarization,
package-manager publication or auto-update mechanism is included.
