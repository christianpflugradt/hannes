"""Credential-free release integrity tests; never tag or publish a real repository."""
import hashlib
import importlib.util
import io
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import unittest

SCRIPTS = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


set_version = load("set-release-version").set_version
verification = load("verify-release")


class ReleaseTests(unittest.TestCase):
    def test_ci_policy_checks_new_messages_without_rejecting_legacy_history(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / ".githooks").mkdir()
            (root / ".githooks/commit-msg").write_bytes((SCRIPTS.parent / ".githooks/commit-msg").read_bytes())
            (root / ".githooks/commit-policy").write_bytes((SCRIPTS.parent / ".githooks/commit-policy").read_bytes())
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True, stderr=subprocess.DEVNULL).strip()
            git("init", "--quiet")
            git("config", "user.name", "Policy Fixture")
            git("config", "user.email", "fixture@example.invalid")
            git("config", "commit.gpgsign", "false")
            git("commit", "--allow-empty", "-m", "Legacy message")
            base = git("rev-parse", "HEAD")
            git("commit", "--allow-empty", "-m", "feat: valid change")
            def validate(base="", title=""):
                env = dict(os.environ, COMMIT_BASE=base, COMMIT_HEAD="HEAD", PR_TITLE=title)
                return subprocess.run([sys.executable, str(SCRIPTS / "check-commits.py")], cwd=root,
                                      env=env, capture_output=True).returncode
            self.assertEqual(validate(), 0)
            self.assertEqual(validate(base), 0)
            self.assertEqual(validate(base, "chore(deps): update rust crate toml to v1.2.0"), 0)
            self.assertNotEqual(validate(base, "Invalid squash title"), 0)
            git("commit", "--allow-empty", "-m", "Invalid new message")
            self.assertNotEqual(validate(), 0)
            output = subprocess.run([sys.executable, str(SCRIPTS / "check-commits.py")], cwd=root,
                                    env=dict(os.environ, COMMIT_BASE="", COMMIT_HEAD="HEAD", PR_TITLE=""),
                                    capture_output=True, text=True)
            self.assertIn(git("rev-parse", "HEAD"), output.stderr)
            self.assertIn("Invalid new message", output.stderr)
            self.assertIn("Optional scopes:", output.stderr)
            self.assertNotIn("Traceback", output.stderr)
            self.assertNotEqual(validate(base, "chore(unknown): reject scope"), 0)

    def test_only_root_version_changes_and_invalid_versions_leave_files_untouched(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = '[package]\nname = "hannes"\nversion = "0.1.0"\n\n[dependencies]\nserde = "1"\n'
            dependency = '\n[[package]]\nname = "serde"\nversion = "1.2.3"\nsource = "registry+example"\n'
            lock = 'version = 4\n\n[[package]]\nname = "hannes"\nversion = "0.1.0"\n' + dependency
            (root / "Cargo.toml").write_text(manifest)
            (root / "Cargo.lock").write_text(lock)
            for invalid in ("v1.0.0", "1.0.0;echo bad", "01.2.3", "1.0.0-beta.1"):
                with self.assertRaises(ValueError):
                    set_version(invalid, root)
                self.assertEqual((root / "Cargo.toml").read_text(), manifest)
            set_version("2.3.4", root)
            self.assertEqual(tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"], "2.3.4")
            self.assertTrue((root / "Cargo.lock").read_text().endswith(dependency))
            set_version("2.3.4", root)  # idempotent
            (root / "Cargo.lock").write_text(lock)
            with self.assertRaises(ValueError):
                set_version("3.0.0", root)

    def test_complete_archives_checksums_and_revision_binding(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dist = root / "dist"
            dist.mkdir()
            (dist / "metadata").mkdir()
            for target in verification.TARGETS:
                name = f"hannes-v1.2.3-{target}"
                archive = dist / f"{name}.tar.gz"
                binary = f"fixture executable for {target}".encode()
                with tarfile.open(archive, "w:gz") as output:
                    for file, data in (("hannes", binary), ("README.md", Path("README.md").read_bytes()),
                                       ("LICENSE", Path("LICENSE").read_bytes())):
                        member = tarfile.TarInfo(f"{name}/{file}")
                        member.mode = 0o755 if file == "hannes" else 0o644
                        member.size = len(data)
                        output.addfile(member, io.BytesIO(data))
                metadata = dict(version="1.2.3", source_sha="a" * 40, target=target,
                                archive_sha256=verification.digest(archive.read_bytes()),
                                binary_sha256=verification.digest(binary))
                (dist / "metadata" / f"{target}.json").write_text(json.dumps(metadata))
            sums = "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
                           for p in sorted(dist.glob("*.tar.gz")))
            subprocess.run([sys.executable, str(SCRIPTS / "checksums.py")], cwd=root, check=True)
            self.assertEqual((dist / "SHA256SUMS").read_text(), sums)
            verification.verify("1.2.3", "a" * 40, dist)
            with self.assertRaises(ValueError):
                verification.verify("1.2.3", "b" * 40, dist)
            with self.assertRaises(ValueError):
                verification.verify("9.9.9", "a" * 40, dist)
            (dist / "SHA256SUMS").write_text("tampered\n")
            with self.assertRaises(ValueError):
                verification.verify("1.2.3", "a" * 40, dist)
            (dist / "SHA256SUMS").write_text(sums)
            next(dist.glob("*.tar.gz")).write_bytes(b"tampered")
            with self.assertRaises(ValueError):
                verification.verify("1.2.3", "a" * 40, dist)

    def test_native_matrix_and_archive_contract_cover_the_same_platforms(self):
        workflow = (SCRIPTS.parent / ".github/workflows/quality.yml").read_text()
        matrices = re.findall(r"'([\[][^']+)'", workflow)
        release, ordinary = [json.loads(value) for value in matrices]
        self.assertEqual({entry["target"] for entry in release}, set(verification.TARGETS))
        self.assertEqual(len(release), len(verification.TARGETS))
        self.assertIn(dict(os="ubuntu-24.04-arm", target="aarch64-unknown-linux-gnu"), release)
        self.assertEqual(ordinary, [dict(os="ubuntu-24.04", target="x86_64-unknown-linux-gnu")])

    def test_missing_or_extra_archives_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            dist = Path(temporary)
            with self.assertRaises(ValueError):
                verification.verify("1.0.0", "a" * 40, dist)
            (dist / "dist").mkdir()
            result = subprocess.run([sys.executable, str(SCRIPTS / "checksums.py")], cwd=dist,
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("No native archives found", result.stderr)


if __name__ == "__main__":
    unittest.main()
