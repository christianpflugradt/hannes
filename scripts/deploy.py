"""Install the latest published native release without building local source."""
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tarfile
import tempfile
from urllib.request import Request, urlopen

REPOSITORY = "https://github.com/christianpflugradt/hannes"
API = "https://api.github.com/repos/christianpflugradt/hannes/releases/latest"


def detect_target():
    system, machine = platform.system(), platform.machine().lower()
    cpu = {"arm64": "aarch64", "aarch64": "aarch64", "amd64": "x86_64", "x86_64": "x86_64"}.get(machine)
    suffix = {"Darwin": "apple-darwin", "Linux": "unknown-linux-gnu"}.get(system)
    if not cpu or not suffix or (system == "Linux" and platform.libc_ver()[0] != "glibc"):
        raise ValueError(f"Unsupported platform: {system}/{machine}. Release downloads support macOS and glibc Linux on x86_64 or ARM64. Build from source on other systems; see {REPOSITORY}#build-locally.")
    return f"{cpu}-{suffix}"


def download(url):
    request = Request(url, headers={"User-Agent": "hannes-deploy", "Accept": "application/vnd.github+json"})
    try:
        with urlopen(request, timeout=60) as response:
            return response.read()
    except OSError as error:
        raise ValueError(f"Could not download {url}: {error}. Check network access and {REPOSITORY}/releases for an available release.") from error


def deploy():
    target = detect_target()  # Reject unsupported hosts before any network or writes.
    release = json.loads(download(API))
    tag = release["tag_name"]
    if not isinstance(tag, str) or not re.fullmatch(r"v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", tag) or release.get("draft") or release.get("prerelease"):
        raise ValueError("Latest release has no supported stable version tag.")
    name = f"hannes-{tag}-{target}"
    archive_name = f"{name}.tar.gz"
    assets = {asset["name"] for asset in release["assets"]}
    if not {archive_name, "SHA256SUMS"} <= assets:
        raise ValueError(f"Release {tag} is missing {archive_name} or SHA256SUMS; check {REPOSITORY}/releases/tag/{tag}.")
    base = f"{REPOSITORY}/releases/download/{tag}"
    sums = download(f"{base}/SHA256SUMS").decode()
    matches = re.findall(r"^([0-9a-f]{64})  " + re.escape(archive_name) + r"$", sums, re.M)
    if len(matches) != 1:
        raise ValueError(f"Missing or duplicate checksum for {archive_name}.")
    data = download(f"{base}/{archive_name}")
    if hashlib.sha256(data).hexdigest() != matches[0]:
        raise ValueError(f"SHA-256 checksum mismatch for {archive_name}; installation aborted.")
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        members = [member for member in archive.getmembers() if member.name == f"{name}/hannes"]
        if len(members) != 1 or not members[0].isfile():
            raise ValueError("Release archive must contain one regular hannes executable.")
        binary = archive.extractfile(members[0]).read()
    prefix = Path(os.environ.get("CARGO_INSTALL_ROOT") or Path.home() / ".local").expanduser()
    destination = prefix / "bin" / "hannes"
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Same filesystem for atomic replacement. Never extract archive paths to disk.
    with tempfile.TemporaryDirectory(prefix=".hannes-deploy-", dir=destination.parent) as temporary:
        candidate = Path(temporary) / "hannes"
        candidate.write_bytes(binary)
        candidate.chmod(0o755)
        try:
            result = subprocess.run([str(candidate.resolve()), "--version"], capture_output=True, text=True, timeout=15,
                                    env=dict(os.environ, HANNES_STATE_DIR=str(Path(temporary) / "state"), NO_COLOR="1"))
            if result.returncode or result.stdout.strip() != f"hannes {tag[1:]}":
                raise ValueError("Downloaded executable failed its version check.")
        except (OSError, subprocess.SubprocessError, ValueError) as error:
            raise ValueError(f"Release {tag} cannot run on this system ({target}); Linux requires a glibc version compatible with Ubuntu 24.04. Build from source instead. {error}") from error
        os.replace(candidate, destination)
    print(f"Installed hannes {tag[1:]} ({target}) to {destination}")


if __name__ == "__main__":
    try:
        deploy()
    except (OSError, ValueError, KeyError, TypeError, tarfile.TarError) as error:
        sys.exit(f"Hannes deploy failed: {error}")
