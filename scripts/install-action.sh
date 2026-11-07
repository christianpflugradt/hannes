#!/usr/bin/env bash
set -euo pipefail
trap 'echo "::error::Hannes installation failed at line $LINENO. Check the release tag, release assets and network access." >&2' ERR

if [[ ! "$HANNES_VERSION" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "::error::version must be an exact stable release tag, such as v0.12.0." >&2
  exit 1
fi
if [[ "$HANNES_RUN" != true && "$HANNES_RUN" != false ]]; then
  echo "::error::run must be true or false." >&2
  exit 1
fi
if [[ "$RUNNER_OS" != Linux ]]; then
  echo "::error::The Hannes Action supports GitHub-hosted Ubuntu 24.04 runners only." >&2
  exit 1
fi
case "$RUNNER_ARCH" in
  X64) target=x86_64-unknown-linux-gnu ;;
  ARM64) target=aarch64-unknown-linux-gnu ;;
  *) echo "::error::Unsupported Ubuntu runner architecture: $RUNNER_ARCH" >&2; exit 1 ;;
esac

work=$(mktemp -d "$RUNNER_TEMP/hannes-download.XXXXXX")
trap 'rm -rf "$work"' EXIT
name="hannes-$HANNES_VERSION-$target"
archive="$name.tar.gz"
base="https://github.com/christianpflugradt/hannes/releases/download/$HANNES_VERSION"
curl --fail --show-error --silent --location --retry 3 --connect-timeout 15 --max-time 120 "$base/$archive" -o "$work/$archive"
curl --fail --show-error --silent --location --retry 3 --connect-timeout 15 --max-time 120 "$base/SHA256SUMS" -o "$work/SHA256SUMS"
# Select exactly this archive; other platform downloads are not needed.
awk -v archive="$archive" '$2 == archive && $1 ~ /^[0-9a-f]+$/ && length($1) == 64 { print; count++ } END { if (count != 1) exit 1 }' "$work/SHA256SUMS" > "$work/selected-sum"
(cd "$work" && sha256sum --check selected-sum)
tar -xzf "$work/$archive" -C "$work" -- "$name/hannes"
if [[ ! -f "$work/$name/hannes" || -L "$work/$name/hannes" ]]; then
  echo "::error::Release archive has no regular hannes executable." >&2
  exit 1
fi
bin="$RUNNER_TEMP/hannes/bin"
mkdir -p "$bin"
install -m 755 "$work/$name/hannes" "$bin/hannes"
printf '%s\n' "$bin" >> "$GITHUB_PATH"
echo "Installed Hannes $HANNES_VERSION ($target)."
