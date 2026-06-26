#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Write SHA256SUMS for release artifacts")
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    output = args.output or args.directory / "SHA256SUMS"
    entries = []
    for path in sorted(args.directory.iterdir()):
        if path.is_file() and path.name != output.name:
            entries.append(f"{sha256(path)}  {path.name}")
    output.write_text("\n".join(entries) + "\n", encoding="utf-8")
    print(output)
    return 0


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


if __name__ == "__main__":
    raise SystemExit(main())
