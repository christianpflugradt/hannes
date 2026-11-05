#!/usr/bin/env python3
"""Exercise the header allow list and real Git rejection in a disposable repository."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TYPES = ("build", "ci", "docs", "feat", "fix", "perf", "refactor", "test")


def git(root, *args, success=True):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
    return result.stdout.strip()


with tempfile.TemporaryDirectory(prefix="hannes-commit-hook-") as directory:
    root = Path(directory)
    message = root / "commit message.txt"
    accepted = [f"{kind}: describe the change" for kind in TYPES]
    accepted += ["feat(reviews): add scoped context", "fix(cli)!: change output", "feat!: change behavior", "docs: x\r\n\r\nDetails."]
    rejected = [f"{kind}: describe the change" for kind in ("chore", "style", "revert", "release", "feature", "hotfix", "FEAT")]
    rejected += ["", "\nfeat: hidden header", "feat:", "feat: ", "feat:  extra space", "feat: trailing ", "feat(no spaces here): bad scope", "feat(): empty scope", "feat(UPPER): bad scope", "feat: tab\there", "feat:no space", "fixup! feat: temporary", "Merge branch 'main'", "chore: bad\n\nfeat: cannot hide the type"]
    for valid, messages in [(True, accepted), (False, rejected)]:
        for text in messages:
            message.write_bytes(text.encode("utf-8"))
            result = subprocess.run(["sh", str(ROOT / ".githooks/commit-msg"), str(message)], capture_output=True)
            assert (result.returncode == 0) == valid, (text, result.stderr)

    repository = root / "repository"
    repository.mkdir()
    git(repository, "init", "--quiet", "--initial-branch=main")
    git(repository, "config", "user.name", "Hook Test")
    git(repository, "config", "user.email", "hook-test@example.invalid")
    git(repository, "config", "commit.gpgsign", "false")
    hooks = repository / ".githooks"
    hooks.mkdir()
    shutil.copy2(ROOT / ".githooks/commit-msg", hooks / "commit-msg")
    (hooks / "commit-msg").chmod(0o755)
    git(repository, "config", "core.hooksPath", ".githooks")
    git(repository, "commit", "--allow-empty", "-m", "chore: reject this commit", success=False)
    git(repository, "rev-parse", "--verify", "HEAD", success=False)
    git(repository, "commit", "--allow-empty", "-m", "feat: accept this commit")
    head = git(repository, "rev-parse", "HEAD")
    git(repository, "commit", "--allow-empty", "-m", "style: reject this commit", success=False)
    git(repository, "commit", "--amend", "-m", "invalid message", success=False)
    assert git(repository, "rev-parse", "HEAD") == head
    # Setup must preserve an existing custom hook configuration.
    git(repository, "config", "core.hooksPath", "custom-hooks")
    result = subprocess.run(["sh", str(ROOT / "scripts/setup-git-hooks.sh")], cwd=repository, capture_output=True)
    assert result.returncode != 0
    assert git(repository, "config", "--get", "core.hooksPath") == "custom-hooks"

print(f"Commit hook passed: {len(accepted)} accepted, {len(rejected)} rejected; real commits/amendments blocked correctly.")
