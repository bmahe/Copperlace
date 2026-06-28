#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
COMMONS_LANG3_VERSION = "3.20.0"


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-test Copperlace Java release JARs")
    parser.add_argument("--api-jar", type=Path, required=True)
    parser.add_argument("--native-jar", type=Path, required=True)
    parser.add_argument("--all-jar", type=Path)
    args = parser.parse_args()
    api_jar = args.api_jar.resolve()
    native_jar = args.native_jar.resolve()
    all_jar = args.all_jar.resolve() if args.all_jar else None

    with tempfile.TemporaryDirectory(prefix="copperlace-java-smoke-") as temp:
        temp_dir = Path(temp)
        commons = temp_dir / f"commons-lang3-{COMMONS_LANG3_VERSION}.jar"
        subprocess.run(
            [
                command("mvn"),
                "-q",
                "dependency:copy",
                f"-Dartifact=org.apache.commons:commons-lang3:{COMMONS_LANG3_VERSION}",
                f"-DoutputDirectory={temp_dir}",
            ],
            check=True,
        )
        source = temp_dir / "Smoke.java"
        source.write_text(
            """
import dev.mahe.copperlace.Copperlace;

public final class Smoke {
    public static void main(final String[] args) {
        final String output = Copperlace.renderString("name = [\\"Mia\\"]\\norigin = \\"Hello {name}\\"", "origin");
        if (!"Hello Mia".equals(output)) {
            throw new IllegalStateException(output);
        }
    }
}
""".strip(),
            encoding="utf-8",
        )
        smoke(temp_dir, source, [api_jar, native_jar, commons])
        if all_jar is not None:
            smoke(temp_dir, source, [all_jar, api_jar, native_jar, commons])

    return 0


def smoke(temp_dir: Path, source: Path, dependencies: list[Path]) -> None:
    classpath = classpath_for(dependencies)
    subprocess.run([command("javac"), "--release", "25", "-cp", classpath, str(source)], cwd=temp_dir, check=True)
    subprocess.run(
        [
            command("java"),
            "--enable-native-access=ALL-UNNAMED",
            "-cp",
            classpath_for([temp_dir, *dependencies]),
            "Smoke",
        ],
        cwd=temp_dir,
        check=True,
    )


def classpath_for(paths: list[Path]) -> str:
    return (";" if platform.system() == "Windows" else ":").join(str(path) for path in paths)


def command(name: str) -> str:
    tool = shutil.which(name)
    if tool is None:
        raise RuntimeError(f"Could not find {name} on PATH")
    return tool


if __name__ == "__main__":
    raise SystemExit(main())
