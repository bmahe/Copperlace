#!/usr/bin/env python3
from __future__ import annotations

import argparse
import zipfile
from pathlib import Path


RESOURCE_PREFIX = "dev/mahe/copperlace/native/"


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect Java native libraries from Copperlace native JARs")
    parser.add_argument("--output-dir", type=Path, default=Path("java/native-artifacts"))
    parser.add_argument("paths", type=Path, nargs="+")
    args = parser.parse_args()

    count = 0
    for jar in jars_under(args.paths):
        count += collect(jar, args.output_dir)

    if count == 0:
        raise SystemExit("no Copperlace native resources found")
    print(f"collected {count} native artifacts under {args.output_dir}")
    return 0


def jars_under(paths: list[Path]) -> list[Path]:
    jars: list[Path] = []
    for path in paths:
        if path.is_dir():
            jars.extend(sorted(path.rglob("*.jar")))
        else:
            jars.append(path)
    return jars


def collect(jar: Path, output_dir: Path) -> int:
    found = 0
    with zipfile.ZipFile(jar) as archive:
        for name in archive.namelist():
            if not name.startswith(RESOURCE_PREFIX) or name.endswith("/"):
                continue
            relative = name.removeprefix(RESOURCE_PREFIX)
            parts = relative.split("/")
            if len(parts) != 2:
                continue
            classifier, library = parts
            destination = output_dir / classifier
            destination.mkdir(parents=True, exist_ok=True)
            with archive.open(name) as source:
                (destination / library).write_bytes(source.read())
            found += 1
    return found


if __name__ == "__main__":
    raise SystemExit(main())
