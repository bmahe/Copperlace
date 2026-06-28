#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
import venv
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Install and smoke-test a Copperlace Python artifact")
    parser.add_argument("artifact", type=Path)
    parser.add_argument("--no-index", action="store_true", help="install without consulting package indexes")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="copperlace-python-smoke-") as temp:
        temp_dir = Path(temp)
        env_dir = temp_dir / "venv"
        venv.EnvBuilder(with_pip=True).create(env_dir)
        python = env_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
        install_command = [str(python), "-m", "pip", "install"]
        if args.no_index:
            install_command.append("--no-index")
        install_command.append(str(args.artifact))
        subprocess.run(install_command, check=True)
        script = (
            "from copperlace import Copperlace\n"
            "with Copperlace.from_string('name = [\"Mia\"]\\norigin = \"Hello {name}\"') as c:\n"
            "    assert c.render('origin') == 'Hello Mia'\n"
        )
        subprocess.run([str(python), "-c", script], cwd=temp_dir, check=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
