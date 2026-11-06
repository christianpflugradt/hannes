"""Run a profile with an installed release; never build or download the governor."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import sys
import tomllib


def run(profile):
    root = Path.cwd().resolve()
    reports = root / ".hannes/reports"
    reports.mkdir(exist_ok=True)
    previous_report = (reports / "last.json").stat().st_mtime_ns if (reports / "last.json").exists() else None
    (reports / "execution.json").unlink(missing_ok=True)
    if os.environ.get("CI") == "true":
        # A failed CI startup must never upload a previous run as current evidence.
        (reports / "last.json").unlink(missing_ok=True)
        previous_report = None
    executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
    if not executable or not Path(executable).is_file():
        raise ValueError("Published Hannes is missing. Run mise deploy explicitly, then put its bin directory on PATH.")
    executable = Path(executable).resolve()
    if executable.is_relative_to(root):
        raise ValueError("The governing Hannes must be an installed published release outside this checkout. Run mise deploy; do not use target/ binaries.")
    version = subprocess.check_output([str(executable), "--version"], text=True).strip()
    if not version.startswith("hannes "):
        raise ValueError("Installed Hannes returned an invalid version. Reinstall with mise deploy.")
    expected = (root / ".hannes-version").read_text().strip()
    if os.environ.get("CI") == "true" and version != f"hannes {expected.removeprefix('v')}":
        raise ValueError(f"CI requires published Hannes {expected}; got {version}. Use the setup-hannes action.")
    definition = tomllib.loads((root / f".hannes/profiles/{profile}.toml").read_text())
    for rule in definition["rules"]:
        check = tomllib.loads((root / f".hannes/checks/{rule}.toml").read_text())
        command = shlex.split(check["command"])[0]
        if not shutil.which(command):
            raise ValueError(f"Missing {command} for {rule}. Run mise install for local development tools; CI also installs actionlint, cargo-audit and Rust 1.85.")
    env = dict(os.environ, HANNES_EXECUTABLE=str(executable))
    if profile in ("pre-commit", "commit-msg", "pre-push"):
        env.update(CARGO_NET_OFFLINE="true", npm_config_offline="true", RUSTUP_AUTO_INSTALL="0")
    result = subprocess.run([str(executable), "mach", "was", "--profil", profile, "--geiz-ist-geil"], env=env)
    report_path = reports / "last.json"
    fresh_report = report_path.exists() and report_path.stat().st_mtime_ns != previous_report
    report = json.loads(report_path.read_text()) if fresh_report else None
    metadata = dict(governor_version=version, governor_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
                    profile=profile, checked_commit=report["snapshot"]["head"] if report else None,
                    report="last.json" if report else None, exit_code=result.returncode)
    (reports / "execution.json").write_text(json.dumps(metadata, indent=2) + "\n")
    if result.returncode:
        print("Gate failed. Inspect hannes was ist phase --details. If configuration is incompatible, install the published governor with mise deploy.", file=sys.stderr)
    return result.returncode


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2 or not all(c.isalnum() or c == "-" for c in sys.argv[1]):
            raise ValueError("Usage: python3 scripts/run-gate.py PROFILE")
        sys.exit(run(sys.argv[1]))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        sys.exit(f"Hannes gate failed: {error}")
