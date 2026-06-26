#!/usr/bin/env python3
from __future__ import annotations

import argparse
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(description="Check Copperlace package version consistency")
    parser.add_argument("--tag", help="optional release tag, such as v0.1.0")
    args = parser.parse_args()

    versions = {
        "cargo": cargo_version(),
        "python": python_version(),
        "java": java_version(),
    }
    unique = set(versions.values())
    if len(unique) != 1:
        raise SystemExit(f"version mismatch: {versions}")

    version = unique.pop()
    if args.tag and args.tag != f"v{version}":
        raise SystemExit(f"tag {args.tag!r} does not match package version {version!r}")

    print(version)
    return 0


def cargo_version() -> str:
    return str(tomllib.loads((ROOT / "rust-core" / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"])


def python_version() -> str:
    return str(tomllib.loads((ROOT / "python" / "pyproject.toml").read_text(encoding="utf-8"))["project"]["version"])


def java_version() -> str:
    root = ET.parse(ROOT / "java" / "pom.xml").getroot()
    namespace = {"m": "http://maven.apache.org/POM/4.0.0"}
    version = root.findtext("m:version", namespaces=namespace)
    if not version:
        raise SystemExit("java/pom.xml does not declare project version")
    return version


if __name__ == "__main__":
    raise SystemExit(main())
