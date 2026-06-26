#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
import venv
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Install and smoke-test a Copperlace wheel")
    parser.add_argument("wheel", type=Path)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="copperlace-wheel-smoke-") as temp:
        temp_dir = Path(temp)
        env_dir = temp_dir / "venv"
        venv.EnvBuilder(with_pip=True).create(env_dir)
        python = env_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
        subprocess.run([str(python), "-m", "pip", "install", "--no-index", str(args.wheel)], check=True)
        script = (
            "from copperlace import Copperlace\n"
            "with Copperlace.from_string('name = [\"Mia\"]\\norigin = \"Hello {name}\"') as c:\n"
            "    assert c.render('origin') == 'Hello Mia'\n"
        )
        subprocess.run([str(python), "-c", script], cwd=temp_dir, check=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
