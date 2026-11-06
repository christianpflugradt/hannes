"""Validate CI commit messages and a prospective squash title using the Git hook."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

head = os.environ.get("COMMIT_HEAD") or "HEAD"
base = os.environ.get("COMMIT_BASE", "")
if base and set(base) == {"0"}:
    base = ""
revision = [f"{base}..{head}"] if base else ["--max-count=1", head]
commits = subprocess.check_output(
    ["git", "rev-list", *revision, "--"], text=True).splitlines()
hook = Path(".githooks/commit-msg").resolve()
with tempfile.TemporaryDirectory(prefix="hannes-commits-") as temporary:
    message = Path(temporary) / "message"
    candidates = [(commit, subprocess.check_output(
        ["git", "show", "-s", "--format=%B", commit], text=True)) for commit in commits]
    if os.environ.get("PR_TITLE"):
        candidates.append(("PR squash title", os.environ["PR_TITLE"]))
    invalid = False
    for label, body in candidates:
        message.write_text(body)
        result = subprocess.run(["sh", str(hook), str(message)], capture_output=True, text=True)
        if result.returncode:
            invalid = True
            header = body.splitlines()[0] if body.splitlines() else ""
            print(f"Invalid commit message ({label}): {header!r}", file=sys.stderr)
            print(result.stderr, end="", file=sys.stderr)
        else:
            print(f"Valid conventional commit: {label}")
    if invalid:
        sys.exit(1)
