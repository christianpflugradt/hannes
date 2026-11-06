"""Offline release installation tests; never download or replace a real install."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("deploy", Path(__file__).with_name("deploy.py"))
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class DeployTests(unittest.TestCase):
    def test_platforms(self):
        for system, machine, target in [
            ("Darwin", "arm64", "aarch64-apple-darwin"),
            ("Darwin", "x86_64", "x86_64-apple-darwin"),
            ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
            ("Linux", "aarch64", "aarch64-unknown-linux-gnu"),
        ]:
            with self.subTest(target=target), patch.object(deploy.platform, "system", return_value=system), patch.object(deploy.platform, "machine", return_value=machine), patch.object(deploy.platform, "libc_ver", return_value=("glibc", "2.39")):
                self.assertEqual(deploy.detect_target(), target)
        for system, machine, libc in [("Windows", "AMD64", ""), ("Linux", "riscv64", "glibc"), ("Linux", "x86_64", "musl")]:
            with patch.object(deploy.platform, "system", return_value=system), patch.object(deploy.platform, "machine", return_value=machine), patch.object(deploy.platform, "libc_ver", return_value=(libc, "")), patch.object(deploy, "download") as download:
                with self.assertRaisesRegex(ValueError, "Unsupported platform"):
                    deploy.deploy()
                download.assert_not_called()

    def test_pinned_install_never_looks_up_latest_and_rejects_substitution(self):
        for value in ("latest", "main", "v1.2.3-beta", "../v1.2.3"):
            with patch.object(deploy, "detect_target", return_value="aarch64-apple-darwin"), patch.object(deploy, "download") as download:
                with self.assertRaisesRegex(ValueError, "exact stable release"):
                    deploy.deploy(tag=value)
                download.assert_not_called()
        with patch.object(deploy, "detect_target", return_value="aarch64-apple-darwin"), patch.object(deploy, "download", return_value=b'{"tag_name":"v9.9.9"}') as download:
            with self.assertRaisesRegex(ValueError, "expected 'v1.2.3'"):
                deploy.deploy(tag="v1.2.3")
            download.assert_called_once_with(deploy.API.removesuffix("latest") + "tags/v1.2.3")

    def test_install_and_failures_preserve_existing_binary(self):
        target = "aarch64-apple-darwin"
        name = f"hannes-v1.2.3-{target}"
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            data = b"release binary"
            member = tarfile.TarInfo(f"{name}/hannes")
            member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
        packed = buffer.getvalue()
        metadata = dict(tag_name="v1.2.3", assets=[dict(name=f"{name}.tar.gz"), dict(name="SHA256SUMS")])
        sums = f"{hashlib.sha256(packed).hexdigest()}  {name}.tar.gz\n".encode()
        with tempfile.TemporaryDirectory() as temporary, patch.dict(os.environ, CARGO_INSTALL_ROOT=temporary), patch.object(deploy, "detect_target", return_value=target):
            destination = Path(temporary) / "bin/hannes"
            destination.parent.mkdir()
            for scenario in ("checksum", "missing", "network", "incompatible", "success"):
                destination.write_bytes(b"previous binary")
                latest = dict(metadata, assets=[] if scenario == "missing" else metadata["assets"])
                responses = [json.dumps(latest).encode(), sums, b"broken" if scenario == "checksum" else packed]
                with self.subTest(scenario=scenario), patch.object(deploy, "download", side_effect=OSError("network failed") if scenario == "network" else responses) as download, patch.object(deploy.subprocess, "run", return_value=deploy.subprocess.CompletedProcess([], 1 if scenario == "incompatible" else 0, "hannes 1.2.3\n", "")):
                    if scenario == "success":
                        deploy.deploy()
                        self.assertEqual([call.args[0] for call in download.call_args_list], [deploy.API, f"{deploy.REPOSITORY}/releases/download/v1.2.3/SHA256SUMS", f"{deploy.REPOSITORY}/releases/download/v1.2.3/{name}.tar.gz"])
                        self.assertEqual(destination.read_bytes(), data)
                        self.assertTrue(destination.stat().st_mode & 0o111)
                    else:
                        with self.assertRaises((ValueError, OSError)):
                            deploy.deploy()
                        self.assertEqual(destination.read_bytes(), b"previous binary")
                self.assertEqual(list(destination.parent.iterdir()), [destination])


if __name__ == "__main__":
    unittest.main()
