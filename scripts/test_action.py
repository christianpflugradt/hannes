"""Offline tests of the public Action installer and its execution command."""
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ActionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.tools = self.root / "tools"
        self.tools.mkdir()
        self.assets = self.root / "assets"
        self.assets.mkdir()
        self.path_file = self.root / "github-path"
        self.env = dict(os.environ, PATH=str(self.tools) + os.pathsep + os.environ["PATH"],
                        RUNNER_TEMP=str(self.root), RUNNER_OS="Linux", RUNNER_ARCH="X64",
                        HANNES_VERSION="v0.14.0", HANNES_RUN="false", GITHUB_PATH=str(self.path_file),
                        ASSETS=str(self.assets))
        curl = self.tools / "curl"
        curl.write_text('#!/bin/bash\nset -eu\nurl="${@: -3:1}"\ncp "$ASSETS/${url##*/}" "${@: -1}"\n')
        curl.chmod(0o755)
        self.archive("x86_64-unknown-linux-gnu")

    def archive(self, target):
        name = f"hannes-v0.14.0-{target}"
        data = b'#!/bin/sh\nprintf "%s\\n" "$@" > "$INVOCATION"\nexit "${RESULT:-0}"\n'
        with tarfile.open(self.assets / f"{name}.tar.gz", "w:gz") as archive:
            info = tarfile.TarInfo(f"{name}/hannes")
            info.size = len(data)
            info.mode = 0o755
            archive.addfile(info, io.BytesIO(data))
        digest = hashlib.sha256((self.assets / f"{name}.tar.gz").read_bytes()).hexdigest()
        (self.assets / "SHA256SUMS").write_text(f"{digest}  {name}.tar.gz\n")

    def install(self, **env):
        return subprocess.run(["bash", str(ROOT / "scripts/install-action.sh")],
                              env=dict(self.env, **env), capture_output=True, text=True)

    def test_install_only_exports_path_without_execution(self):
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = Path(self.path_file.read_text().strip()) / "hannes"
        self.assertTrue(os.access(installed, os.X_OK))
        self.assertFalse((self.root / "invocation").exists())
        self.assertFalse(list(self.root.glob("hannes-download.*")))
        # A new process consumes GITHUB_PATH, as a subsequent workflow step does.
        result = subprocess.run(["bash", "-c", "command -v hannes"],
                                env=dict(self.env, PATH=str(installed.parent) + os.pathsep + self.env["PATH"]),
                                capture_output=True, text=True)
        self.assertEqual(result.stdout.strip(), str(installed))

    def test_execution_point_is_one_argument_and_status_is_preserved(self):
        self.assertEqual(self.install().returncode, 0)
        metadata = (ROOT / "action.yml").read_text()
        self.assertIn("if: inputs.run == 'true'", metadata)
        command = metadata.split("      run: ")[-1].strip()
        for status in (0, 1, 2, 7):
            invocation = self.root / "invocation"
            point = 'custom point; $(touch unexpected)'
            result = subprocess.run(["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", command],
                                    env=dict(self.env, PATH=self.path_file.read_text().strip() + os.pathsep + self.env["PATH"],
                                             HANNES_EXECUTION_POINT_INPUT=point, INVOCATION=str(invocation), RESULT=str(status)),
                                    cwd=self.root, capture_output=True, text=True)
            self.assertEqual(result.returncode, status, result.stderr)
            self.assertEqual(invocation.read_text().splitlines(), ["mach", "was", "--at", point])
            self.assertFalse((self.root / "unexpected").exists())

    def test_arm64_archive(self):
        self.archive("aarch64-unknown-linux-gnu")
        self.assertEqual(self.install(RUNNER_ARCH="ARM64").returncode, 0)

    def test_invalid_inputs_and_platforms_fail_before_installation(self):
        for env in ({"HANNES_VERSION": ""}, {"HANNES_VERSION": "latest"},
                    {"HANNES_VERSION": "v0.14.0;echo injected"}, {"HANNES_RUN": "yes"},
                    {"RUNNER_OS": "macOS"}, {"RUNNER_ARCH": "other"}):
            with self.subTest(env=env):
                result = self.install(**env)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("::error::", result.stderr)
                self.assertFalse(self.path_file.exists())

    def test_download_and_checksum_failures_do_not_export_path(self):
        for sums in ("", "0" * 64 + "  hannes-v0.14.0-x86_64-unknown-linux-gnu.tar.gz\n"):
            (self.assets / "SHA256SUMS").write_text(sums)
            result = self.install()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Hannes installation failed", result.stderr)
            self.assertFalse(self.path_file.exists())
        (self.assets / "SHA256SUMS").unlink()
        self.assertNotEqual(self.install().returncode, 0)
        self.assertFalse(self.path_file.exists())


if __name__ == "__main__":
    unittest.main()
