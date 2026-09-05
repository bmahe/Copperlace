#!/usr/bin/env python3
"""Stage the Copperlace native library into elixir/priv/native/ for precompilation.

Copies the release-built Rust dynamic library into the Elixir wrapper's
priv/native/ directory so that `mix elixir_make.precompile` bundles it into
the precompiled archive alongside the NIF shim.
"""
from __future__ import annotations

import argparse
import platform
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Stage a Copperlace native library for Elixir precompilation"
    )
    parser.add_argument("--output-dir", type=Path, default=ROOT / "elixir" / "priv" / "native")
    args = parser.parse_args()

    name = native_library_name()
    source = ROOT / "rust-core" / "target" / "release" / name
    if not source.exists():
        raise SystemExit(f"missing native library: {source}. Run `make rust-build` first.")

    destination = args.output_dir.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination / name)
    print(destination / name)
    return 0


def native_library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "copperlace.dll"
    if system == "Darwin":
        return "libcopperlace.dylib"
    return "libcopperlace.so"


if __name__ == "__main__":
    raise SystemExit(main())
