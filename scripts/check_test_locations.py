#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


RUST_SOURCE_ROOT = Path("rust-core/src")
RUST_TEST_PATTERNS = (
    re.compile(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]"),
    re.compile(r"^\s*mod\s+tests\s*\{"),
)


def main() -> int:
    violations = rust_source_test_violations()
    if violations:
        for path, line_number, line in violations:
            print(
                f"{path}:{line_number}: tests must live outside production source files: {line}",
                file=sys.stderr,
            )
        return 1
    return 0


def rust_source_test_violations() -> list[tuple[Path, int, str]]:
    violations: list[tuple[Path, int, str]] = []
    for path in sorted(RUST_SOURCE_ROOT.rglob("*.rs")):
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if any(pattern.search(line) for pattern in RUST_TEST_PATTERNS):
                violations.append((path, line_number, line.strip()))
    return violations


if __name__ == "__main__":
    raise SystemExit(main())
