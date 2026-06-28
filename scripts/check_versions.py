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

    versions = package_versions()
    unique = set(versions.values())
    if len(unique) != 1:
        raise SystemExit(f"version mismatch: {versions}")

    version = unique.pop()
    if args.tag and args.tag != f"v{version}":
        raise SystemExit(f"tag {args.tag!r} does not match package version {version!r}")

    print(version)
    return 0


def package_versions() -> dict[str, str]:
    versions = {
        "rust-core/Cargo.toml": cargo_version(),
        "python/pyproject.toml": python_version(),
        "java/pom.xml": java_version(ROOT / "java" / "pom.xml"),
    }
    for path in sorted((ROOT / "java").glob("**/pom.xml")):
        if path == ROOT / "java" / "pom.xml":
            continue
        versions[str(path.relative_to(ROOT))] = java_parent_version(path)
    return versions


def cargo_version() -> str:
    return str(tomllib.loads((ROOT / "rust-core" / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"])


def python_version() -> str:
    return str(tomllib.loads((ROOT / "python" / "pyproject.toml").read_text(encoding="utf-8"))["project"]["version"])


def java_version(path: Path) -> str:
    root = ET.parse(path).getroot()
    namespace = {"m": "http://maven.apache.org/POM/4.0.0"}
    version = root.findtext("m:version", namespaces=namespace)
    if not version:
        raise SystemExit(f"{path.relative_to(ROOT)} does not declare project version")
    return version


def java_parent_version(path: Path) -> str:
    root = ET.parse(path).getroot()
    namespace = {"m": "http://maven.apache.org/POM/4.0.0"}
    version = root.findtext("m:parent/m:version", namespaces=namespace)
    if not version:
        raise SystemExit(f"{path.relative_to(ROOT)} does not declare parent version")
    return version


if __name__ == "__main__":
    raise SystemExit(main())
