"""Offline integration tests for the released governor, execution points and CI wiring."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
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


# Expected checks and selections for the pinned published governor.
BASELINE = json.loads((ROOT / "tests/fixtures/self-governance.json").read_text())

def definitions():
    checks = {}
    for path in sorted((ROOT / ".hannes/checks").glob("*/rule.toml")):
        rule = tomllib.loads(path.read_text())
        assert rule["id"] not in checks
        checks[rule["id"]] = rule
    assert not list((ROOT / ".hannes/rule-sets").glob("*.toml")), "Local rules declare their execution points directly"
    assert [p.parent.name for p in (ROOT / ".hannes/reviews").glob("*/rule.toml")] == ["agents-quality"]
    assert not list((ROOT / ".hannes/checks").glob("*.toml")), "Obsolete check documents must be removed"
    assert not (ROOT / ".hannes/rule-sets.toml").exists(), "Obsolete rule-set document must be removed"
    return checks


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

    def test_self_governance_checks_settings_and_required_coverage(self):
        checks = definitions()
        config = tomllib.loads((ROOT / ".hannes/config.toml").read_text())
        self.assertEqual(set(config), {"project", "max_display_files", "execution_points", "imports"})
        self.assertEqual(set(config["execution_points"]["custom"]), set(BASELINE["execution_points"]) - {"pre-commit", "commit-msg", "pre-push", "check"})
        self.assertEqual(set(checks), set(BASELINE["checks"]))
        for rule, check in checks.items():
            implementation = check["check"]
            original = BASELINE["checks"][rule]
            self.assertEqual({key: value for key, value in check.items() if key != "check"},
                             {key: value for key, value in original.items() if key not in ("command", "timeout_seconds")})
            self.assertEqual(implementation["timeout_seconds"], original["timeout_seconds"])
            self.assertEqual(set(implementation), {"command", "timeout_seconds"})
            self.assertIsInstance(implementation["command"], str)
            # Local commands use the common quoting subset of shlex and Hannes.
            # Keep the pre-migration argument vectors as independent evidence.
            self.assertEqual(shlex.split(implementation["command"]), original["command"])
            self.assertFalse((ROOT / ".hannes/checks" / rule / "index.sh").exists())
        # The published-governor test below independently checks real resolution.
        for point, expected in BASELINE["execution_points"].items():
            selected = set()
            for rule, check in checks.items():
                if check["enabled"] and point in check["run_at"]:
                    selected.add(rule)
            self.assertEqual(selected, set(expected), point)
        task = tomllib.loads((ROOT / "mise.toml").read_text())["tasks"]["check"]
        self.assertEqual(task["run"], "python3 scripts/run-execution-point.py pre-push")
        self.assertNotIn("depends", task)

    def test_external_commit_message_import_is_pinned_and_scopes_match_policy(self):
        config = tomllib.loads((ROOT / ".hannes/config.toml").read_text())
        imported, = [item for item in config["imports"] if item["path"] == "commit-msg/conventional-commits"]
        self.assertEqual(imported["git"], "https://github.com/christianpflugradt/christianpflugradt-rulesets.git")
        self.assertEqual(imported["path"], "commit-msg/conventional-commits")
        self.assertRegex(imported["ref"], r"^[0-9a-f]{40}$")
        parameters = imported["rule_parameters"]["commit-message"]
        self.assertEqual(parameters, {"scopes_file": ".hannes/commit-scopes.txt"})
        expected = [line.split()[1] for line in (ROOT / ".githooks/commit-policy").read_text().splitlines() if line.startswith("scope ")]
        self.assertEqual((ROOT / parameters["scopes_file"]).read_text().splitlines(), expected)

    def test_security_import_is_pinned_and_ci_executes_check_with_dependencies(self):
        config = tomllib.loads((ROOT / ".hannes/config.toml").read_text())
        security, = [item for item in config["imports"] if item["path"] == "multi-point/security"]
        self.assertEqual(set(security), {"git", "path", "ref"})
        self.assertEqual(security["git"], "https://github.com/christianpflugradt/christianpflugradt-rulesets.git")
        self.assertRegex(security["ref"], r"^[0-9a-f]{40}$")
        self.assertFalse((ROOT / ".hannes/checks/secret-scan").exists())
        self.assertFalse((ROOT / ".hannes/checks/sensitive-files").exists())
        workflow = (ROOT / ".github/workflows/ci.yml").read_text().split("  action-consumer:")[0]
        run = "run: python3 scripts/run-execution-point.py check --reviews"
        self.assertEqual(workflow.count(run + "\n"), 1)
        self.assertLess(workflow.index("actions/setup-python@"), workflow.index(run))
        self.assertLess(workflow.index("bash scripts/prepare-ci.sh"), workflow.index(run))
        self.assertLess(workflow.index(run), workflow.index("name: quality-check"))
        preparation = (ROOT / "scripts/prepare-ci.sh").read_text()
        for text in (preparation, (ROOT / ".github/workflows/release.yml").read_text()):
            self.assertIn("v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz", text)
            self.assertIn("551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb", text)
            self.assertIn("sha256sum --check", text)
            self.assertIn('echo "$RUNNER_TEMP/gitleaks-bin" >> "$GITHUB_PATH"', text)
        for required in ("npm ci --ignore-scripts", "cargo +stable fetch --locked", "cargo +1.85.0 fetch --locked",
                         "cargo +stable install cargo-audit --version", "go install github.com/rhysd/actionlint"):
            self.assertIn(required, preparation)
        release = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertLess(release.index("Install pinned Gitleaks"), release.index("Publish versioned CLI"))

    def test_repository_conventions_import_is_pinned_and_license_target(self):
        config = tomllib.loads((ROOT / ".hannes/config.toml").read_text())
        imported, = [item for item in config["imports"] if item["path"] == "repository-conventions"]
        self.assertRegex(imported["ref"], r"^[0-9a-f]{40}$")
        self.assertEqual({key: value for key, value in imported.items() if key != "ref"}, {
            "git": "https://github.com/christianpflugradt/christianpflugradt-rulesets.git",
            "path": "repository-conventions",
            "rule_parameters": {"license": {"target": "LICENSE"}},
        })
        self.assertFalse((ROOT / ".hannes/checks/license").exists())

    def test_published_governor_can_plan_every_execution_point_without_execution(self):
        executable = os.environ.get("HANNES_EXECUTABLE") or shutil.which("hannes")
        self.assertTrue(executable, "Install the compatible published Hannes release before running governance acceptance")
        self.assertFalse(Path(executable).resolve().is_relative_to(ROOT))
        version = subprocess.check_output([executable, "--version"], text=True).strip()
        self.assertEqual(version, "hannes " + (ROOT / ".hannes-version").read_text().strip().removeprefix("v"))
        for point, expected in BASELINE["execution_points"].items():
            with self.subTest(point=point):
                result = subprocess.run([executable, "mach", "was", "--at", point, "--plan", "--json"], cwd=ROOT, env=dict(os.environ, HANNES_CHANGE_BASE="HEAD"), capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
                plan = json.loads(result.stdout)
                self.assertEqual(plan["at"], point)
                local = [rule for rule in plan["scheduled"] if not rule["origin"].get("external") and rule["kind"] == "check"]
                imported = [rule for rule in plan["scheduled"] if rule["origin"].get("external")]
                self.assertEqual([rule["id"] for rule in local], sorted(expected))
                expected_rules = {"commit-message"} if point == "commit-msg" else (
                    {"license", "secret-scan", "sensitive-files"} if point in ("check", "pre-push") else set())
                self.assertEqual({rule["id"].rsplit(":", 1)[-1] for rule in imported}, expected_rules)
                self.assertEqual(len(imported), len(expected_rules))
                declarations = tomllib.loads((ROOT / ".hannes/config.toml").read_text())["imports"]
                for rule in imported:
                    self.assertTrue(rule["blocking"])
                    origin = rule["origin"]["external"]
                    declaration, = [item for item in declarations if item["path"] == origin["path"]]
                    self.assertEqual(origin["commit"], declaration["ref"])
                reviews = [rule for rule in plan["scheduled"] if rule["kind"] == "review"]
                self.assertEqual([rule["id"] for rule in reviews], ["agents-quality"] if point == "check" and reviews else [])
                self.assertEqual(set(plan["required_ids"]), set(expected) | {rule["id"] for rule in imported + reviews})
                self.assertEqual(plan["planned_model_calls"], len(reviews))
                self.assertTrue(all(item["reason"] == "trigger_not_matched" for item in plan["omitted"]))

    def test_ci_installs_pinned_governor_and_uploads_failure_reports(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        checks = ci.split("  action-consumer:")[0]
        builds = (ROOT / ".github/workflows/quality.yml").read_text()
        release = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertIn("name: Quality checks", checks)
        self.assertNotIn("matrix:", checks)
        self.assertEqual(checks.count("scripts/run-execution-point.py"), 1)
        self.assertEqual(checks.count("run: hannes pass auf --auto-approve"), 1)
        self.assertEqual(checks.count("if: always()"), 2)
        self.assertEqual(checks.count("include-hidden-files: true"), 1)
        self.assertIn("name: GitHub Action compatibility /", ci)
        self.assertIn("os: [ubuntu-24.04, ubuntu-24.04-arm]", ci)
        self.assertIn("run: python3 scripts/run-execution-point.py release-build", builds)
        self.assertIn("if: runner.os == 'macOS'", builds)
        self.assertIn("uses: ./.github/workflows/ci.yml", release)
        self.assertIn("needs: [plan, checks, quality]", release)
        for obsolete in ("ci-native", "ci-tooling", "ci-msrv", "ci-audit"):
            self.assertNotIn(obsolete, ci + builds)
        for workflow in (checks, builds, release):
            self.assertRegex(workflow, r"uses: christianpflugradt/hannes@[0-9a-f]{40}")
            self.assertIn("run: 'false'", workflow)
            self.assertIn("version: ${{ steps.governor.outputs.version }}", workflow)
            self.assertIn("if: always()", workflow)
            self.assertNotIn("releases/latest", workflow)
            self.assertNotRegex(workflow, r"run: (cargo \+stable (test|clippy|audit)|npm (test|audit)|python3 scripts/(check-commits|verify-release|test-))")
        self.assertIn("python3 scripts/run-execution-point.py release-artifacts", release)
        installer = (ROOT / "scripts/install-governor.py").read_text()
        self.assertIn('Path(".hannes-version")', installer)
        self.assertIn("deploy(tag=version", installer)
        self.assertNotIn("cargo", installer)
        self.assertFalse((ROOT / ".github/actions/setup-hannes/action.yml").exists())

    def test_release_prepares_offline_hooks_and_uploads_reports_after_publication(self):
        release = (ROOT / ".github/workflows/release.yml").read_text()
        prepare, publish, upload = (
            release.index(name) for name in (
                "      - name: Prepare managed release hooks",
                "      - name: Publish versioned CLI archives and release notes",
                "      - name: Upload release hook evidence",
            )
        )
        self.assertLess(prepare, publish)
        self.assertLess(publish, upload)
        preparation = release[prepare:publish]
        self.assertIn("run: |", preparation)
        self.assertIn("rustup toolchain install stable --profile minimal --component rustfmt --component clippy", preparation)
        self.assertIn("cargo +stable fetch --locked", preparation)
        self.assertIn("rustup toolchain install 1.85.0 --profile minimal", preparation)
        self.assertIn("cargo +1.85.0 fetch --locked", preparation)
        self.assertIn("go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.12", preparation)
        self.assertIn('echo "$(go env GOPATH)/bin" >> "$GITHUB_PATH"', preparation)
        publisher = release.split("  publish:\n", 1)[1]
        self.assertIn("uses: actions/setup-go@", publisher[:publisher.index("      - name: Prepare managed release hooks")])
        self.assertLess(preparation.index("rustup toolchain install 1.85.0"), preparation.index("cargo +1.85.0 fetch"))
        self.assertLess(preparation.index("rustup toolchain install"), preparation.index("cargo +stable fetch"))
        evidence = release[upload:].split("\n      - ")[0]
        for required in ("if: always()", "uses: actions/upload-artifact@", "name: quality-release-hooks", "path: .hannes/reports/*.json", "include-hidden-files: true"):
            self.assertIn(required, evidence)

    def test_native_checks_remain_offline_without_a_launcher(self):
        checks = definitions()
        for rule in ("10-clippy", "11-rust-tests", "12-native-build", "23-msrv"):
            self.assertIn("--offline", shlex.split(checks[rule]["check"]["command"]))
        self.assertEqual(shlex.split(checks["08-release-rules"]["check"]["command"]), ["npm", "--offline", "test"])
        self.assertFalse((ROOT / "scripts/run-check.py").exists())
        for point in ("pre-push", "check"):
            self.assertTrue({"03-python-tests", "08-release-rules", "11-rust-tests", "22-workflows", "23-msrv"}.issubset(BASELINE["execution_points"][point]))

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
            (config / "config.toml").write_text('project = "Failure fixture"\nmax_display_files = 5\n')
            shutil.copy2(ROOT / ".hannes/provider.toml", config / "provider.toml")
            rule = config / "checks/fail"
            rule.mkdir()
            (rule / "rule.toml").write_text('id = "fail"\ndescription = "Deliberate failure"\nrun_at = ["pre-commit"]\n[check]\nentrypoint = "index.py"\n' + f'interpreter = {json.dumps(sys.executable)}\ntimeout_seconds = 10\n')
            (rule / "index.py").write_text("raise SystemExit(1)\n")
            setup = subprocess.run([executable, "pass", "auf", "--auto-approve"], cwd=root, capture_output=True, text=True)
            self.assertEqual(setup.returncode, 0, setup.stderr)
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
