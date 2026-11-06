#!/usr/bin/env python3
"""Exercise the header allow list and real Git rejection in a disposable repository."""
from pathlib import Path
import shutil
import json
import os
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
            result = subprocess.run(["sh", str(ROOT / "scripts/check-commit-message.sh"), str(message)], capture_output=True)
            assert (result.returncode == 0) == valid, (text, result.stderr)

    repository = root / "repository"
    repository.mkdir()
    git(repository, "init", "--quiet", "--initial-branch=main")
    git(repository, "config", "user.name", "Hook Test")
    git(repository, "config", "user.email", "hook-test@example.invalid")
    git(repository, "config", "commit.gpgsign", "false")
    hooks = repository / ".githooks"
    shutil.copytree(ROOT / ".githooks", hooks)
    scripts = repository / "scripts"
    scripts.mkdir()
    for name in ("run-gate.py", "check-commits.py", "check-commit-message.sh", "setup-git-hooks.sh"):
        shutil.copy2(ROOT / "scripts" / name, scripts / name)
    shutil.copy2(ROOT / ".hannes-version", repository / ".hannes-version")
    config = repository / ".hannes"
    (config / "checks").mkdir(parents=True)
    (config / "profiles").mkdir()
    (config / "reviews").mkdir()
    (config / "config.toml").write_text('version = 2\nproject = "Hook fixture"\nmax_display_files = 5\n')
    shutil.copy2(ROOT / ".hannes/provider.toml", config / "provider.toml")
    (repository / ".gitignore").write_text(".hannes/reports/\n__pycache__/\n")
    (scripts / "fixture-check.py").write_text("from pathlib import Path; import sys; sys.exit(1 if Path(sys.argv[1]).exists() else 0)\n")
    for profile in ("pre-commit", "commit-msg", "pre-push"):
        command = "python3 scripts/check-commits.py --message" if profile == "commit-msg" else f"python3 scripts/fixture-check.py fail-{profile}"
        (config / f"checks/{profile}.toml").write_text(f'id = "{profile}"\ndescription = "Fixture gate"\nenabled = true\ncommand = {json.dumps(command)}\ntimeout_seconds = 30\n')
        (config / f"profiles/{profile}.toml").write_text(f'id = "{profile}"\nrules = ["{profile}"]\ngate = true\n')
    git(repository, "config", "core.hooksPath", ".githooks")
    (repository / "tracked").write_text("unchanged source\n")
    git(repository, "add", ".")
    index = git(repository, "ls-files", "--stage")
    (repository / "fail-pre-commit").touch()
    git(repository, "commit", "-m", "feat: blocked by pre-commit", success=False)
    assert git(repository, "ls-files", "--stage") == index
    assert (repository / "tracked").read_text() == "unchanged source\n"
    (repository / "fail-pre-commit").unlink()
    governor = os.environ.get("HANNES_EXECUTABLE")
    os.environ["HANNES_EXECUTABLE"] = "/nonexistent/hannes"
    git(repository, "commit", "-m", "feat: missing governor", success=False)
    if governor is None:
        del os.environ["HANNES_EXECUTABLE"]
    else:
        os.environ["HANNES_EXECUTABLE"] = governor
    assert git(repository, "ls-files", "--stage") == index
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
    # A real push must propagate the pre-push gate failure without changing HEAD/index.
    remote = root / "remote.git"
    git(root, "init", "--bare", "--quiet", str(remote))
    git(repository, "remote", "add", "origin", str(remote))
    index = git(repository, "ls-files", "--stage")
    (repository / "fail-pre-push").touch()
    git(repository, "push", "origin", "main", success=False)
    assert git(repository, "rev-parse", "HEAD") == head
    assert git(repository, "ls-files", "--stage") == index
    assert (repository / "tracked").read_text() == "unchanged source\n"
    assert git(root, "--git-dir", str(remote), "for-each-ref") == ""
    (repository / "fail-pre-push").unlink()
    git(repository, "push", "origin", "main")
    assert git(root, "--git-dir", str(remote), "rev-parse", "main") == head
    # Explicit setup is idempotent and preserves custom hooks.
    git(repository, "config", "--unset", "core.hooksPath")
    subprocess.run(["sh", str(scripts / "setup-git-hooks.sh")], cwd=repository, check=True, capture_output=True)
    assert git(repository, "config", "--get", "core.hooksPath") == ".githooks"
    # Setup must preserve an existing custom hook configuration.
    git(repository, "config", "core.hooksPath", "custom-hooks")
    result = subprocess.run(["sh", str(ROOT / "scripts/setup-git-hooks.sh")], cwd=repository, capture_output=True)
    assert result.returncode != 0
    assert git(repository, "config", "--get", "core.hooksPath") == "custom-hooks"

print(f"Commit hook passed: {len(accepted)} accepted, {len(rejected)} rejected; real commit/amend/push gates and index preservation passed.")
