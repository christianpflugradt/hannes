"""Offline integration tests for the released governor, execution points and CI wiring."""
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
spec = importlib.util.spec_from_file_location("execution", ROOT / "scripts/run-execution-point.py")
execution = importlib.util.module_from_spec(spec)
spec.loader.exec_module(execution)


# Immutable pre-migration evidence, captured before converting active configuration.
BASELINE = json.loads((ROOT / "tests/fixtures/self-governance-v2.json").read_text())

def definitions():
    checks, sets = {}, []
    for path in sorted((ROOT / ".hannes").rglob("*.toml")):
        if "reports" in path.parts or path.name in ("config.toml", "provider.toml"):
            continue
        document = tomllib.loads(path.read_text())
        assert not document.get("reviews"), "Self-integration must not call a provider"
        for rule in document.get("checks", []):
            assert rule["id"] not in checks
            checks[rule["id"]] = rule
        sets.extend(document.get("rule_sets", []))
    return checks, sets


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

    def test_rules_preserve_previous_commands_settings_and_required_coverage(self):
        checks, sets = definitions()
        config = tomllib.loads((ROOT / ".hannes/config.toml").read_text())
        self.assertEqual(config["version"], 3)
        self.assertNotIn("default_profile", config)
        self.assertFalse((ROOT / ".hannes/profiles").exists())
        self.assertEqual(config["hooks"]["git"], {p: p for p in ("pre-commit", "commit-msg", "pre-push")})
        self.assertEqual(set(config["execution_points"]["custom"]), set(BASELINE["profiles"]) - set(config["hooks"]["git"]))
        self.assertEqual(set(checks), set(BASELINE["checks"]))
        for rule, check in checks.items():
            original = {key: value for key, value in check.items() if key != "run_at"}
            prefix = "python3 scripts/run-check.py "
            self.assertTrue(original["command"].startswith(prefix))
            original["command"] = original["command"].removeprefix(prefix)
            self.assertEqual(original, BASELINE["checks"][rule])
            self.assertNotRegex(original["command"], r"run-execution-point|\bmise\b|\bhannes mach\b")
        for group in sets:
            self.assertTrue(group["enabled"])
            self.assertTrue(group["run_at"])
            self.assertEqual(len(group["rules"]), len(set(group["rules"])))
            self.assertTrue(set(group["rules"]) <= set(checks))
        # The published-governor test below independently checks real resolution.
        for point, expected in BASELINE["profiles"].items():
            selected = set()
            for rule, check in checks.items():
                memberships = [group for group in sets if group["enabled"] and rule in group["rules"]]
                at = {p for group in memberships for p in group["run_at"]} if memberships else set(check.get("run_at", []))
                if check["enabled"] and point in at:
                    selected.add(rule)
            self.assertEqual(selected, set(expected), point)

    def test_published_governor_can_plan_every_execution_point_without_execution(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        self.assertTrue(executable, "Install the compatible published Hannes release before running governance acceptance")
        self.assertFalse(Path(executable).resolve().is_relative_to(ROOT))
        for point, expected in BASELINE["profiles"].items():
            with self.subTest(point=point):
                result = subprocess.run([executable, "mach", "was", "--at", point, "--plan", "--json"], cwd=ROOT, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
                plan = json.loads(result.stdout)
                self.assertEqual(plan["at"], point)
                self.assertEqual([rule["id"] for rule in plan["scheduled"]], sorted(expected))
                self.assertEqual(set(plan["required_ids"]), set(expected))
                self.assertEqual(plan["planned_model_calls"], 0)
                self.assertFalse(plan["omitted"])

    def test_ci_installs_pinned_governor_and_uploads_failure_reports(self):
        workflow = (ROOT / ".github/workflows/quality.yml").read_text()
        release = (ROOT / ".github/workflows/release.yml").read_text()
        for point in ("ci-native", "ci-tooling", "ci-msrv", "ci-audit"):
            self.assertIn(f"python3 scripts/run-execution-point.py {point}", workflow)
        self.assertEqual(workflow.count("uses: ./.github/actions/setup-hannes"), 4)
        self.assertEqual(workflow.count("if: always()"), 4)
        self.assertEqual(workflow.count("include-hidden-files: true"), 4)
        self.assertIn("python3 scripts/run-execution-point.py release-artifacts", release)
        self.assertIn("if: always()", release)
        self.assertIn("include-hidden-files: true", release)
        self.assertIn("rustup toolchain install stable --profile minimal --component rustfmt", release)
        self.assertLess(release.index("Prepare managed release commit hooks"), release.index("Publish versioned CLI archives and release notes"))
        for text in (workflow, release):
            self.assertNotIn("releases/latest", text)
            self.assertNotRegex(text, r"run: (cargo \+stable (test|clippy|audit)|npm (test|audit)|python3 scripts/(check-commits|verify-release|test-))")
        installer = (ROOT / "scripts/install-governor.py").read_text()
        self.assertIn('Path(".hannes-version")', installer)
        self.assertIn("deploy(tag=version", installer)
        self.assertNotIn("cargo", installer)
        action = (ROOT / ".github/actions/setup-hannes/action.yml").read_text()
        self.assertIn('"$HANNES_EXECUTABLE" pass auf --auto-approve', action)
        self.assertNotRegex(workflow + release + action, r"--profil(?:\s|=|$)")

    def test_hook_launcher_preserves_arguments_and_offline_environment(self):
        probe = "import json,os,sys; print(json.dumps([sys.argv[1:], {k:os.environ.get(k) for k in ['CARGO_NET_OFFLINE','npm_config_offline','RUSTUP_AUTO_INSTALL','HANNES_GIT_HOOK_ARG_1']}]))"
        for point in ("pre-commit", "commit-msg", "pre-push"):
            result = subprocess.run([sys.executable, str(ROOT / "scripts/run-check.py"), sys.executable, "-c", probe, "two words", "$literal"],
                env=dict(os.environ, HANNES_GIT_HOOK=point, HANNES_GIT_HOOK_ARG_1="message with spaces"), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            arguments, environment = json.loads(result.stdout)
            self.assertEqual(arguments, ["two words", "$literal"])
            self.assertEqual(environment, dict(CARGO_NET_OFFLINE="true", npm_config_offline="true", RUSTUP_AUTO_INSTALL="0", HANNES_GIT_HOOK_ARG_1="message with spaces"))
        env = {k:v for k,v in os.environ.items() if k not in ("HANNES_GIT_HOOK", "CARGO_NET_OFFLINE", "npm_config_offline", "RUSTUP_AUTO_INSTALL", "HANNES_GIT_HOOK_ARG_1")}
        result = subprocess.run([sys.executable, str(ROOT / "scripts/run-check.py"), sys.executable, "-c", probe], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(all(v is None for v in json.loads(result.stdout)[1].values()))

    def test_commit_message_checker_consumes_managed_hook_arguments(self):
        with tempfile.TemporaryDirectory() as temporary:
            message = Path(temporary) / "commit message.txt"
            env = dict(os.environ, HANNES_GIT_HOOK="commit-msg", HANNES_GIT_HOOK_ARG_COUNT="1", HANNES_GIT_HOOK_ARG_1=str(message))
            def check(environment):
                return subprocess.run([sys.executable, str(ROOT / "scripts/check-commits.py"), "--message"], env=environment, capture_output=True, text=True)
            message.write_text("feat: accept managed hook arguments\n")
            self.assertEqual(check(env).returncode, 0)
            message.write_text("Invalid message\n")
            self.assertNotEqual(check(env).returncode, 0)
            message.write_text("feat: valid message\n")
            self.assertNotEqual(check(dict(env, HANNES_GIT_HOOK="pre-commit")).returncode, 0)
            self.assertNotEqual(check(dict(env, HANNES_GIT_HOOK_ARG_COUNT="2")).returncode, 0)
            self.assertNotEqual(check(dict(env, HANNES_GIT_HOOK_ARG_1="")).returncode, 0)

    def test_setup_shim_preserves_custom_paths_and_retires_only_empty_legacy_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            (root / ".githooks").mkdir()
            fake = root / "governor"
            fake.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\"\n")
            fake.chmod(0o755)
            env = dict(os.environ, HANNES_EXECUTABLE=str(fake))
            def configure(value):
                subprocess.run(["git", "config", "--local", "core.hooksPath", value], cwd=root, check=True)
            def setup():
                return subprocess.run(["sh", str(ROOT / "scripts/setup-git-hooks.sh"), "--auto-approve"], cwd=root, env=env, capture_output=True, text=True)
            configure("custom-hooks")
            result = setup()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["pass", "auf", "--auto-approve"])
            self.assertEqual(subprocess.check_output(["git", "config", "--get", "core.hooksPath"], cwd=root, text=True).strip(), "custom-hooks")
            configure(".githooks")
            custom = root / ".githooks/pre-commit"
            custom.write_text("custom content")
            self.assertNotEqual(setup().returncode, 0)
            self.assertEqual(custom.read_text(), "custom content")
            custom.unlink()
            self.assertEqual(setup().returncode, 0)
            self.assertNotEqual(subprocess.run(["git", "config", "--local", "--get", "core.hooksPath"], cwd=root, capture_output=True).returncode, 0)

    def test_execution_rejects_missing_or_candidate_governors(self):
        with patch.dict(os.environ, HANNES_EXECUTABLE="/nonexistent/hannes"):
            with self.assertRaisesRegex(ValueError, "mise deploy"):
                execution.run("pre-commit")
        with patch.dict(os.environ, HANNES_EXECUTABLE=str(ROOT / "Cargo.toml")):
            with self.assertRaisesRegex(ValueError, "outside this checkout"):
                execution.run("pre-commit")

    def test_ci_version_mismatch_clears_stale_evidence_and_fails(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reports = root / ".hannes/reports"
            reports.mkdir(parents=True)
            (root / ".hannes-version").write_text("v999.0.0\n")
            (reports / "last.json").write_text('{"overall":"PASS"}')
            (reports / "execution.json").write_text('{"exit_code":0}')
            result = subprocess.run([sys.executable, str(ROOT / "scripts/run-execution-point.py"), "pre-commit"], cwd=root,
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
            (config / "reviews").mkdir()
            (root / ".gitignore").write_text(".hannes/reports/\n")
            (root / ".hannes-version").write_text((ROOT / ".hannes-version").read_text())
            (config / "config.toml").write_text('version = 3\nproject = "Failure fixture"\nmax_display_files = 5\n')
            shutil.copy2(ROOT / ".hannes/provider.toml", config / "provider.toml")
            (config / "checks/fail.toml").write_text('[[checks]]\nrun_at=["pre-commit"]\nid = "fail"\ndescription = "Deliberate failure"\nenabled = true\ncommand = "python3 -c \'raise SystemExit(1)\'"\ntimeout_seconds = 10\n')
            env = dict(os.environ, HANNES_EXECUTABLE=executable)
            result = subprocess.run([sys.executable, str(ROOT / "scripts/run-execution-point.py"), "pre-commit"], cwd=root, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads((config / "reports/last.json").read_text())
            metadata = json.loads((config / "reports/execution.json").read_text())
            self.assertEqual(report["overall"], "FAIL")
            self.assertEqual(metadata["execution_point"], "pre-commit")
            self.assertEqual(metadata["exit_code"], 1)
            self.assertRegex(metadata["governor_sha256"], r"^[0-9a-f]{64}$")
            # Missing tools/startup cannot masquerade as an earlier successful run.
            env["HANNES_EXECUTABLE"] = "/nonexistent/hannes"
            missing = subprocess.run([sys.executable, str(ROOT / "scripts/run-execution-point.py"), "pre-commit"], cwd=root, env=env, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("mise deploy", missing.stderr)


if __name__ == "__main__":
    unittest.main()
