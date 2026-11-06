"""Smoke-test the candidate path corresponding to native-build.py."""
import os
from pathlib import Path
import subprocess
import sys

target = os.environ.get("BUILD_TARGET")
binary = Path("target") / target / "release/hannes" if target else Path("target/release/hannes")
subprocess.run([sys.executable, "scripts/smoke.py", str(binary)], check=True)
