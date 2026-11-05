"""Package an already-built native executable; no network operations."""
import os
from pathlib import Path
import shutil
import tarfile
import tempfile
import tomllib
import zipfile

version = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
tag = os.environ["RELEASE_TAG"]
if tag != f"v{version}":
    raise SystemExit(f"Tag {tag!r} does not match Cargo version {version!r}")
target = os.environ["BUILD_TARGET"]
extension = os.environ.get("BINARY_EXTENSION", "")
name = f"hannes-{tag}-{target}"
output = Path("dist")
output.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory() as temporary:
    folder = Path(temporary) / name
    folder.mkdir()
    for source in (Path(f"target/{target}/release/hannes{extension}"), Path("README.md"), Path("LICENSE")):
        shutil.copy2(source, folder / source.name)
    if extension == ".exe":
        with zipfile.ZipFile(output / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
            for entry in sorted(folder.iterdir()):
                archive.write(entry, f"{name}/{entry.name}")
    else:
        with tarfile.open(output / f"{name}.tar.gz", "w:gz") as archive:
            archive.add(folder, arcname=name)
