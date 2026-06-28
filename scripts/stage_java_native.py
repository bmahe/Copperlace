#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description="Stage a Copperlace native library for Java packaging")
    parser.add_argument("--classifier", help="native classifier to stage; defaults to the current platform")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "java" / "native-artifacts")
    parser.add_argument("--print-classifier", action="store_true", help="print the selected classifier and exit")
    args = parser.parse_args()

    classifier = args.classifier or current_classifier()
    if args.print_classifier:
        print(classifier)
        return 0

    library = native_library_name(classifier)
    source = ROOT / "rust-core" / "target" / "release" / library
    if not source.exists():
        raise SystemExit(f"missing native library: {source}")

    destination = args.output_dir.resolve() / classifier
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination / library)
    print(destination / library)
    return 0


def current_classifier() -> str:
    system = platform.system().lower()
    machine = normalize_arch(platform.machine())
    if system == "linux":
        return f"linux-{machine}"
    if system == "darwin":
        return f"macos-{machine}"
    if system == "windows":
        return f"windows-{machine}"
    raise SystemExit(f"unsupported native OS: {platform.system()}")


def normalize_arch(raw: str) -> str:
    arch = raw.lower()
    if arch in {"amd64", "x86_64"}:
        return "x86_64"
    if arch in {"aarch64", "arm64"}:
        return "aarch64"
    raise SystemExit(f"unsupported native architecture: {raw}")


def native_library_name(classifier: str) -> str:
    if classifier.startswith("linux-"):
        return "libcopperlace.so"
    if classifier.startswith("macos-"):
        return "libcopperlace.dylib"
    if classifier.startswith("windows-"):
        return "copperlace.dll"
    raise SystemExit(f"unsupported native classifier: {classifier}")


if __name__ == "__main__":
    raise SystemExit(main())
