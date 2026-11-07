#!/usr/bin/env python3
"""Exercise the header allow list and real Git rejection in a disposable repository."""
from pathlib import Path
import shutil
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
    for name in ("run-execution-point.py", "run-check.py", "check-commits.py", "check-commit-message.sh", "setup-git-hooks.sh"):
        shutil.copy2(ROOT / "scripts" / name, scripts / name)
    shutil.copy2(ROOT / ".hannes-version", repository / ".hannes-version")
    config = repository / ".hannes"
    (config / "checks").mkdir(parents=True)
    (config / "reviews").mkdir()
    (config / "reports").mkdir()
    (config / "config.toml").write_text('project = "Hook fixture"\nmax_display_files = 5\n')
    shutil.copy2(ROOT / ".hannes/provider.toml", config / "provider.toml")
    (repository / ".gitignore").write_text(".hannes/reports/\n__pycache__/\n")
    (scripts / "fixture-check.py").write_text("from pathlib import Path; import sys; sys.exit(1 if Path(sys.argv[1]).exists() else 0)\n")
    for point in ("pre-commit", "commit-msg", "pre-push"):
        command = "python3 scripts/run-check.py python3 scripts/check-commits.py --message" if point == "commit-msg" else f"python3 scripts/run-check.py python3 scripts/fixture-check.py fail-{point}"
        rule = config / "checks" / point
        rule.mkdir()
        (rule / "rule.toml").write_text(f'id = "{point}"\ndescription = "Fixture gate"\nrun_at = ["{point}"]\n[check]\nentrypoint = "index.sh"\ninterpreter = "sh"\ntimeout_seconds = 30\n')
        (rule / "index.sh").write_text(f"#!/bin/sh\nexec {command}\n")
    executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
    assert executable, "Install the compatible published Hannes governor first"
    # Exercise configured paths while retaining the separate versioned message policy.
    git(repository, "config", "core.hooksPath", "custom-hooks")
    setup = subprocess.run([executable, "pass", "auf", "--auto-approve"], cwd=repository, capture_output=True, text=True)
    assert setup.returncode == 0, setup.stderr
    assert git(repository, "config", "--get", "core.hooksPath") == "custom-hooks"
    (repository / "tracked").write_text("unchanged source\n")
    git(repository, "add", ".")
    index = git(repository, "ls-files", "--stage")
    (repository / "fail-pre-commit").touch()
    git(repository, "commit", "-m", "feat: blocked by pre-commit", success=False)
    assert git(repository, "ls-files", "--stage") == index
    assert (repository / "tracked").read_text() == "unchanged source\n"
    (repository / "fail-pre-commit").unlink()
    # Wrapper execution must fail closed when Hannes is missing from PATH.
    binaries = root / "git-only"
    binaries.mkdir()
    (binaries / "git").symlink_to(shutil.which("git"))
    missing = subprocess.run([str(binaries / "git"), "commit", "-m", "feat: missing governor"], cwd=repository,
                             env=dict(os.environ, PATH=str(binaries)), capture_output=True)
    assert missing.returncode != 0
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
    # Setup delegates to the manager and retains a custom effective path.
    before = (repository / "custom-hooks/pre-commit").read_bytes()
    result = subprocess.run(["sh", str(scripts / "setup-git-hooks.sh"), "--auto-approve"], cwd=repository, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    assert git(repository, "config", "--get", "core.hooksPath") == "custom-hooks"
    assert (repository / "custom-hooks/pre-commit").read_bytes() == before

print(f"Commit hook passed: {len(accepted)} accepted, {len(rejected)} rejected; managed commit/amend/push enforcement and index preservation passed.")
