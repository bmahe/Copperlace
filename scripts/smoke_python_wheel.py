#!/usr/bin/env python3
from __future__ import annotations

from __future__ import annotations

import sys

from smoke_python_artifact import main as smoke_main


def main() -> int:
    if "--no-index" not in sys.argv:
        sys.argv.insert(1, "--no-index")
    return smoke_main()



if __name__ == "__main__":
    raise SystemExit(main())
