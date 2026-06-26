#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import shutil
import tarfile
import tomllib
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description="Package the Copperlace CLI and native library")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)
    version = cargo_version()
    target = target_name()
    archive_root = f"copperlace-{version}-{target}-cli"
    staging = ROOT / "target" / "release-artifacts" / archive_root
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)

    for path in [cli_binary(), native_library(), ROOT / "README.adoc", ROOT / "LICENSE"]:
        shutil.copy2(path, staging / path.name)

    if platform.system() == "Windows":
        archive = args.output_dir / f"{archive_root}.zip"
        if archive.exists():
            archive.unlink()
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zip_file:
            for path in sorted(staging.rglob("*")):
                zip_file.write(path, path.relative_to(staging.parent))
    else:
        archive = args.output_dir / f"{archive_root}.tar.gz"
        if archive.exists():
            archive.unlink()
        with tarfile.open(archive, "w:gz") as tar_file:
            tar_file.add(staging, arcname=archive_root)

    print(archive)
    return 0


def cargo_version() -> str:
    manifest = tomllib.loads((ROOT / "rust-core" / "Cargo.toml").read_text(encoding="utf-8"))
    return str(manifest["package"]["version"])


def target_name() -> str:
    system = platform.system()
    machine = platform.machine().lower()
    arch = "aarch64" if machine in {"aarch64", "arm64"} else "x86_64"
    if system == "Darwin":
        return f"macos-{arch}"
    if system == "Windows":
        return f"windows-{arch}"
    return f"linux-{arch}"


def cli_binary() -> Path:
    name = "copperlace.exe" if platform.system() == "Windows" else "copperlace"
    path = ROOT / "rust-core" / "target" / "release" / name
    if not path.exists():
        raise SystemExit(f"missing CLI binary: {path}")
    return path


def native_library() -> Path:
    system = platform.system()
    if system == "Windows":
        name = "copperlace.dll"
    elif system == "Darwin":
        name = "libcopperlace.dylib"
    else:
        name = "libcopperlace.so"
    path = ROOT / "rust-core" / "target" / "release" / name
    if not path.exists():
        raise SystemExit(f"missing native library: {path}")
    return path


if __name__ == "__main__":
    raise SystemExit(main())
