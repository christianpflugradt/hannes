"""Exercise the real CLI in an isolated, uncommitted Git repository. No Codex."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="hannes-smoke-") as temporary:
    root = Path(temporary) / "repository"
    root.mkdir()
    subprocess.run(["git", "init", "--quiet", str(root)], check=True)
    environment = dict(os.environ, NO_COLOR="1", HANNES_STATE_DIR=str(Path(temporary) / "state"))

    def run(words, code=0):
        result = subprocess.run([str(binary), *words.split()], cwd=root,
                                env=environment, capture_output=True, text=True)
        if result.returncode != code:
            raise AssertionError(f"{words}: expected {code}, got {result.returncode}\n{result.stdout}\n{result.stderr}")
        if "\x1b" in result.stdout:
            raise AssertionError("NO_COLOR output contains ANSI escapes")
        print(f"[{code}] hannes {words}")
        return result.stdout

    run("--help")
    run("--version")
    run("so hilf mir doch")
    run("fang an")
    initial = (root / ".hannes/config.toml").read_bytes()
    run("fang an")
    assert (root / ".hannes/config.toml").read_bytes() == initial
    run("pass auf")
    run("was ist phase", 3)
    run("mach mal")
    report_path = root / ".hannes/reports/last.json"
    report = json.loads(report_path.read_text())
    assert report["overall"] == "PASS"
    assert [r["id"] for r in report["results"]] == ["always-ok"]
    assert "CURRENT" in run("was ist phase")
    (root / "tracked.txt").write_text("initial\n")
    subprocess.run(["git", "add", "tracked.txt"], cwd=root, check=True)
    run("mach mal")
    (root / "tracked.txt").write_text("modified\n")
    for index in range(7):
        (root / f"untracked-{index}.txt").write_text("new\n")
    subprocess.run(["git", "add", "untracked-0.txt"], cwd=root, check=True)
    stale = run("was ist phase", 1)
    assert "STALE" in stale and "changed files since verification: 8" in stale
    assert stale.count("(line counts unavailable)") == 5
    check_path = root / ".hannes/checks/deliberately-failing.toml"
    check_path.write_text(check_path.read_text().replace("enabled = false", "enabled = true"))
    review_path = root / ".hannes/reviews/product-consistency.toml"
    review_path.write_text(review_path.read_text().replace("enabled = false", "enabled = true"))
    # The failed check must skip the enabled review. Any provider call would fail this assertion.
    run("mach mal", 1)
    report = json.loads(report_path.read_text())
    assert report["overall"] == "FAIL"
    assert report["results"][-1]["status"] == "NOT_RUN"
    check_path.write_text(check_path.read_text().replace("enabled = true", "enabled = false"))
    review_path.write_text(review_path.read_text().replace("enabled = true", "enabled = false"))
    run("mach mal")
    assert "CURRENT" in run("was ist phase")
    assert not (root / ".git/index.lock").exists()
    commits = subprocess.run(["git", "rev-parse", "--verify", "HEAD"], cwd=root, capture_output=True)
    assert commits.returncode != 0
    print("Smoke test passed: all commands, PASS/FAIL reports, skipped reviews, CURRENT/STALE, exact totals and five-path limit. No commits or Codex calls.")
