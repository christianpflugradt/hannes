#!/usr/bin/env python3
"""Exercise the header allow list and real Git rejection in a disposable repository."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TYPES = ("build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test")
SCOPES = ("checks", "ci", "cli", "config", "deps", "docs", "git", "mise", "mood",
          "profiles", "provider", "release", "report", "reviews", "snapshot", "tests")
policy = [line.split() for line in (ROOT / ".githooks/commit-policy").read_text().splitlines()]
assert tuple(value for kind, value in policy if kind == "type") == TYPES
assert tuple(value for kind, value in policy if kind == "scope") == SCOPES


def git(root, *args, success=True):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
    return result.stdout.strip()


with tempfile.TemporaryDirectory(prefix="hannes-commit-hook-") as directory:
    root = Path(directory)
    message = root / "commit message.txt"
    accepted = [f"{kind}: describe the change" for kind in TYPES]
    accepted += [f"{kind}({scope}): describe the change" for kind in TYPES for scope in SCOPES]
    accepted += ["feat(reviews): add scoped context", "fix(cli)!: change output", "feat!: change behavior", "docs: x\r\n\r\nDetails."]
    rejected = [f"{kind}: describe the change" for kind in ("release", "feature", "hotfix", "FEAT")]
    rejected += [f"chore({scope}): invalid scope" for scope in ("unknown", "dep", "deps-dev", "deps/cargo", "ci.github", "cli,config", "*")]
    rejected += ["", "\nfeat: hidden header", "feat:", "feat: ", "feat:  extra space", "feat: trailing ", "feat(no spaces here): bad scope", "feat(): empty scope", "feat(UPPER): bad scope", "feat: tab\there", "feat:no space", "fixup! feat: temporary", "Merge branch 'main'", "unknown: bad\n\nfeat: cannot hide the type"]
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
    shutil.copy2(ROOT / ".githooks/commit-policy", hooks / "commit-policy")
    (hooks / "commit-msg").chmod(0o755)
    git(repository, "config", "core.hooksPath", ".githooks")
    git(repository, "commit", "--allow-empty", "-m", "unknown: reject this commit", success=False)
    git(repository, "rev-parse", "--verify", "HEAD", success=False)
    git(repository, "commit", "--allow-empty", "-m", "feat: accept this commit")
    git(repository, "commit", "--allow-empty", "-m", "chore(deps): update rust crate toml to v1.2.0")
    head = git(repository, "rev-parse", "HEAD")
    git(repository, "commit", "--allow-empty", "-m", "unknown: reject this commit", success=False)
    git(repository, "commit", "--amend", "-m", "invalid message", success=False)
    git(repository, "commit", "--allow-empty", "-m", "fix(unknown): reject unknown scope", success=False)
    assert git(repository, "rev-parse", "HEAD") == head
    # A missing or malformed policy must fail closed, never disable validation.
    (hooks / "commit-policy").unlink()
    git(repository, "commit", "--allow-empty", "-m", "feat: missing policy", success=False)
    (hooks / "commit-policy").write_text("type feat\nscope\n")
    git(repository, "commit", "--allow-empty", "-m", "feat: malformed policy", success=False)
    assert git(repository, "rev-parse", "HEAD") == head
    shutil.copy2(ROOT / ".githooks/commit-policy", hooks / "commit-policy")
    # Setup must preserve an existing custom hook configuration.
    git(repository, "config", "core.hooksPath", "custom-hooks")
    result = subprocess.run(["sh", str(ROOT / "scripts/setup-git-hooks.sh")], cwd=repository, capture_output=True)
    assert result.returncode != 0
    assert git(repository, "config", "--get", "core.hooksPath") == "custom-hooks"

print(f"Commit hook passed: {len(accepted)} accepted, {len(rejected)} rejected; real commits/amendments blocked correctly.")
