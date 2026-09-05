#!/usr/bin/env python3
"""Install and smoke-test a Copperlace Hex package artifact.

The script unpacks a `mix hex.build` tarball into a temporary directory, adds
it as a path dependency to a throwaway Mix project, compiles it (which builds
the NIF shim), and renders a known configuration. It mirrors
smoke_python_artifact.py for the Elixir wrapper.

The Copperlace native library is resolved by the NIF at runtime. By default
the script points COPPERLACE_LIBRARY_PATH at the local rust-core release
build; override with --library.
"""
from __future__ import annotations

import argparse
import os
import platform
import subprocess
import tarfile
import tempfile
from pathlib import Path


def native_library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "copperlace.dll"
    if system == "Darwin":
        return "libcopperlace.dylib"
    return "libcopperlace.so"


def main() -> int:
    parser = argparse.ArgumentParser(description="Install and smoke-test a Copperlace Hex artifact")
    parser.add_argument("artifact", type=Path)
    parser.add_argument(
        "--library",
        type=Path,
        default=None,
        help="path to the Copperlace native library (defaults to rust-core release build)",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    library = args.library or (
        repo_root / "rust-core" / "target" / "release" / native_library_name()
    )
    if not library.exists():
        raise SystemExit(f"native library not found: {library}. Build it with `make rust-build`.")

    with tempfile.TemporaryDirectory(prefix="copperlace-elixir-smoke-") as temp:
        temp_dir = Path(temp)
        package_dir = temp_dir / "copperlace_pkg"
        package_dir.mkdir()

        # A Hex tarball wraps the package files in an inner contents.tar.gz;
        # the outer archive holds VERSION, CHECKSUM, metadata.config, and
        # contents.tar.gz. Extract the inner archive to get mix.exs etc.
        with tarfile.open(args.artifact, "r:*") as outer:
            outer.extractall(temp_dir / "outer", filter="data")  # noqa: S202

        inner = temp_dir / "outer" / "contents.tar.gz"
        if not inner.exists():
            raise SystemExit(f"{args.artifact} is not a Hex package: missing contents.tar.gz")
        with tarfile.open(inner, "r:gz") as tar:
            tar.extractall(package_dir, filter="data")  # noqa: S202

        project_dir = temp_dir / "smoke"
        subprocess.run(
            ["mix", "new", "smoke"],
            cwd=temp_dir,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        mix_exs = project_dir / "mix.exs"
        mix_content = mix_exs.read_text(encoding="utf-8")
        marker = "defp deps do\n    [\n"
        replacement = "defp deps do\n    [\n      {:copperlace, path: \"../copperlace_pkg\"}\n"
        if marker not in mix_content:
            raise SystemExit("could not inject copperlace dependency into smoke project")
        mix_exs.write_text(mix_content.replace(marker, replacement, 1), encoding="utf-8")

        env = {
            **os.environ,
            "COPPERLACE_LIBRARY_PATH": str(library),
            "MIX_ENV": "prod",
        }

        subprocess.run(["mix", "deps.get"], cwd=project_dir, env=env, check=True)
        subprocess.run(["mix", "compile"], cwd=project_dir, env=env, check=True)

        script = (
            "{:ok, c} = Copperlace.from_string(~s(name = [\"Mia\"]\\n"
            "origin = \"Hello {name}\"))\n"
            "{:ok, output} = Copperlace.render(c, \"origin\")\n"
            "IO.puts(output)\n"
        )
        result = subprocess.run(
            ["mix", "run", "--no-compile", "-e", script],
            cwd=project_dir,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        # `mix run` may still print dependency noise to stdout; assert the
        # rendered line is present on its own line.
        if "Hello Mia" not in result.stdout.splitlines():
            raise SystemExit(f"unexpected render output: {result.stdout!r}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
