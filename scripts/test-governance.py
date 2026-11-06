"""Offline integration tests for the released governor, profiles and CI wiring."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("run_gate", ROOT / "scripts/run-gate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class GovernanceTests(unittest.TestCase):
    def test_renovate_extracts_one_published_version_without_automerge(self):
        config = json.loads((ROOT / "renovate.json").read_text())
        manager, = config["customManagers"]
        self.assertEqual(manager["managerFilePatterns"], [r"/^\.hannes-version$/"])
        pattern = manager["matchStrings"][0].replace("(?<currentValue>", "(?P<currentValue>")
        match, = list(re.finditer(pattern, (ROOT / ".hannes-version").read_text()))
        self.assertRegex(match["currentValue"], r"^v\d+\.\d+\.\d+$")
        self.assertEqual(manager["datasourceTemplate"], "github-releases")
        self.assertEqual(manager["depNameTemplate"], "christianpflugradt/hannes")
        self.assertEqual(manager["versioningTemplate"], "semver")
        rule, = [r for r in config["packageRules"] if "christianpflugradt/hannes" in r.get("matchPackageNames", [])]
        self.assertFalse(rule["automerge"])
        self.assertEqual(rule["matchDatasources"], ["github-releases"])
        self.assertIn("config:best-practices", config["extends"])

    def test_profiles_have_explicit_complete_coverage_without_recursion_or_reviews(self):
        checks = {tomllib.loads(p.read_text())["id"]: tomllib.loads(p.read_text()) for p in (ROOT / ".hannes/checks").glob("*.toml")}
        profiles = {p.stem: tomllib.loads(p.read_text()) for p in (ROOT / ".hannes/profiles").glob("*.toml")}
        self.assertEqual(tomllib.loads((ROOT / ".hannes/config.toml").read_text())["default_profile"], "pre-push")
        self.assertEqual(list((ROOT / ".hannes/reviews").glob("*.toml")), [])
        self.assertEqual(list((ROOT / ".hannes/reviews").glob("*.md")), [])
        covered = set()
        for profile in profiles.values():
            self.assertTrue(profile["gate"])
            self.assertTrue(profile["rules"])
            self.assertFalse(profile.get("include_tags"))
            for rule in profile["rules"]:
                self.assertTrue(checks[rule]["enabled"])
                covered.add(rule)
        self.assertEqual(covered, set(checks))
        self.assertEqual(set(profiles["pre-commit"]["rules"]), {"01-format", "02-docs"})
        self.assertTrue(set(profiles["pre-commit"]["rules"]) <= set(profiles["pre-push"]["rules"]))
        self.assertEqual(profiles["ci-audit"]["rules"], ["24-rust-audit", "25-npm-audit"])
        for check in checks.values():
            self.assertIsInstance(check["command"], str)
            self.assertNotRegex(check["command"], r"run-gate|\bmise\b|\bhannes mach\b")
        self.assertEqual(checks["25-npm-audit"]["command"], "npm audit --audit-level=high")

    def test_published_governor_can_plan_every_profile_without_execution(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        self.assertTrue(executable, "Install published Hannes using mise deploy before running governance tests")
        for path in sorted((ROOT / ".hannes/profiles").glob("*.toml")):
            with self.subTest(profile=path.stem):
                result = subprocess.run([executable, "mach", "was", "--profil", path.stem, "--plan", "--json"], cwd=ROOT, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
                plan = json.loads(result.stdout)
                self.assertEqual(plan["profile"], path.stem)
                self.assertEqual(plan["planned_model_calls"], 0)

    def test_ci_installs_pinned_governor_and_uploads_failure_reports(self):
        workflow = (ROOT / ".github/workflows/quality.yml").read_text()
        release = (ROOT / ".github/workflows/release.yml").read_text()
        for profile in ("ci-native", "ci-tooling", "ci-msrv", "ci-audit"):
            self.assertIn(f"python3 scripts/run-gate.py {profile}", workflow)
        self.assertEqual(workflow.count("uses: ./.github/actions/setup-hannes"), 4)
        self.assertEqual(workflow.count("if: always()"), 4)
        self.assertEqual(workflow.count("include-hidden-files: true"), 4)
        self.assertIn("python3 scripts/run-gate.py release-artifacts", release)
        self.assertIn("if: always()", release)
        self.assertIn("include-hidden-files: true", release)
        for text in (workflow, release):
            self.assertNotIn("releases/latest", text)
            self.assertNotRegex(text, r"run: (cargo \+stable (test|clippy|audit)|npm (test|audit)|python3 scripts/(check-commits|verify-release|test-))")
        installer = (ROOT / "scripts/install-governor.py").read_text()
        self.assertIn('Path(".hannes-version")', installer)
        self.assertIn("deploy(tag=version", installer)
        self.assertNotIn("cargo", installer)

    def test_gate_rejects_missing_or_candidate_governors(self):
        with patch.dict(os.environ, HANNES_EXECUTABLE="/nonexistent/hannes"):
            with self.assertRaisesRegex(ValueError, "mise deploy"):
                gate.run("pre-commit")
        with patch.dict(os.environ, HANNES_EXECUTABLE=str(ROOT / "Cargo.toml")):
            with self.assertRaisesRegex(ValueError, "outside this checkout"):
                gate.run("pre-commit")

    def test_ci_version_mismatch_clears_stale_evidence_and_fails(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reports = root / ".hannes/reports"
            reports.mkdir(parents=True)
            (root / ".hannes-version").write_text("v999.0.0\n")
            (reports / "last.json").write_text('{"overall":"PASS"}')
            (reports / "execution.json").write_text('{"exit_code":0}')
            result = subprocess.run([sys.executable, str(ROOT / "scripts/run-gate.py"), "pre-commit"], cwd=root,
                                    env=dict(os.environ, CI="true", HANNES_EXECUTABLE=executable), capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("CI requires published Hannes", result.stderr)
            self.assertFalse((reports / "last.json").exists())
            self.assertFalse((reports / "execution.json").exists())

    def test_failed_check_has_report_and_governor_metadata(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            config = root / ".hannes"
            (config / "checks").mkdir(parents=True)
            (config / "profiles").mkdir()
            (config / "reviews").mkdir()
            (root / ".gitignore").write_text(".hannes/reports/\n")
            (root / ".hannes-version").write_text((ROOT / ".hannes-version").read_text())
            (config / "config.toml").write_text('version = 2\nproject = "Failure fixture"\nmax_display_files = 5\n')
            shutil.copy2(ROOT / ".hannes/provider.toml", config / "provider.toml")
            (config / "checks/fail.toml").write_text('id = "fail"\ndescription = "Deliberate failure"\nenabled = true\ncommand = "python3 -c \'raise SystemExit(1)\'"\ntimeout_seconds = 10\n')
            (config / "profiles/pre-commit.toml").write_text('id = "pre-commit"\nrules = ["fail"]\ngate = true\n')
            env = dict(os.environ, HANNES_EXECUTABLE=executable)
            result = subprocess.run([sys.executable, str(ROOT / "scripts/run-gate.py"), "pre-commit"], cwd=root, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads((config / "reports/last.json").read_text())
            metadata = json.loads((config / "reports/execution.json").read_text())
            self.assertEqual(report["overall"], "FAIL")
            self.assertEqual(metadata["profile"], "pre-commit")
            self.assertEqual(metadata["exit_code"], 1)
            self.assertRegex(metadata["governor_sha256"], r"^[0-9a-f]{64}$")
            # Missing tools/startup cannot masquerade as an earlier successful run.
            env["HANNES_EXECUTABLE"] = "/nonexistent/hannes"
            missing = subprocess.run([sys.executable, str(ROOT / "scripts/run-gate.py"), "pre-commit"], cwd=root, env=env, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("mise deploy", missing.stderr)


if __name__ == "__main__":
    unittest.main()
