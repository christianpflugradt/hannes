"""Compute SHA-256 checksums for downloaded release archives."""
import hashlib
from pathlib import Path

files = sorted(p for p in Path("dist").iterdir() if p.name.endswith((".tar.gz", ".zip")))
if len(files) != 4:
    raise SystemExit(f"Expected four native archives, found {len(files)}")
with Path("dist/SHA256SUMS").open("w", newline="\n") as output:
    for path in files:
        output.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n")
