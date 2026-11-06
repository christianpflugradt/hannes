"""Build only the candidate under test, never the quality-gate governor."""
import os
import subprocess

command = ["cargo", "+stable", "build", "--release", "--locked"]
if os.environ.get("BUILD_TARGET"):
    command += ["--target", os.environ["BUILD_TARGET"]]
subprocess.run(command, check=True)
