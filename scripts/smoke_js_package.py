#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Install and smoke-test a Copperlace JS package tarball")
    parser.add_argument("package", type=Path)
    args = parser.parse_args()
    package = args.package.resolve()

    with tempfile.TemporaryDirectory(prefix="copperlace-js-smoke-") as temp:
        temp_dir = Path(temp)
        (temp_dir / "package.json").write_text(json.dumps({"type": "module"}), encoding="utf-8")
        subprocess.run(["npm", "install", "--ignore-scripts", str(package)], cwd=temp_dir, check=True)
        script = temp_dir / "smoke.mjs"
        script.write_text(
            "import { Copperlace } from 'copperlace';\n"
            "const c = new Copperlace('name = [\"Mia\"]\\norigin = \"Hello {name}\"');\n"
            "const output = c.render('origin');\n"
            "if (output !== 'Hello Mia') throw new Error(`unexpected output: ${output}`);\n",
            encoding="utf-8",
        )
        subprocess.run(["node", "--experimental-wasm-modules", str(script)], cwd=temp_dir, check=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
