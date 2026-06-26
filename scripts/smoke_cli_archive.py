#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-test a Copperlace CLI archive")
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="copperlace-cli-smoke-") as temp:
        temp_dir = Path(temp)
        extract(args.archive, temp_dir)
        executable = find_executable(temp_dir)
        config = temp_dir / "smoke.conf"
        config.write_text('name = ["Mia"]\norigin = "Hello {name}"\n', encoding="utf-8")
        env = os.environ.copy()
        env["COPPERLACE_LIBRARY_PATH"] = str(find_native_library(temp_dir))
        output = subprocess.check_output(
            [str(executable), "render", "--config", str(config), "--rule", "origin"],
            cwd=temp_dir,
            env=env,
            text=True,
        ).strip()
        if output != "Hello Mia":
            raise SystemExit(f"unexpected CLI output: {output!r}")

    return 0


def extract(archive: Path, destination: Path) -> None:
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as zip_file:
            zip_file.extractall(destination)
    else:
        with tarfile.open(archive) as tar_file:
            tar_file.extractall(destination)


def find_executable(root: Path) -> Path:
    name = "copperlace.exe" if platform.system() == "Windows" else "copperlace"
    matches = list(root.rglob(name))
    if not matches:
        raise SystemExit(f"archive does not contain {name}")
    executable = matches[0]
    if platform.system() != "Windows":
        executable.chmod(executable.stat().st_mode | 0o755)
    return executable


def find_native_library(root: Path) -> Path:
    system = platform.system()
    if system == "Windows":
        name = "copperlace.dll"
    elif system == "Darwin":
        name = "libcopperlace.dylib"
    else:
        name = "libcopperlace.so"
    matches = list(root.rglob(name))
    if not matches:
        raise SystemExit(f"archive does not contain {name}")
    return matches[0]


if __name__ == "__main__":
    raise SystemExit(main())
