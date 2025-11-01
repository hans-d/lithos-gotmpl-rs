#!/usr/bin/env python3
"""Verify NOTICE includes all required third-party attributions."""

from __future__ import annotations

from pathlib import Path
import sys

SNIPPETS_FILE = Path("docs/legal/notice-snippets.txt")


def load_required_snippets() -> list[str]:
    if not SNIPPETS_FILE.exists():
        print(f"Required snippets file missing: {SNIPPETS_FILE}", file=sys.stderr)
        raise SystemExit(1)

    snippets: list[str] = []
    for raw_line in SNIPPETS_FILE.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        snippets.append(line)
    if not snippets:
        print(f"No snippets defined in {SNIPPETS_FILE}", file=sys.stderr)
        raise SystemExit(1)
    return snippets


def main() -> int:
    notice_path = Path("NOTICE")
    contents = notice_path.read_text(encoding="utf-8")
    snippets = load_required_snippets()

    missing = [snippet for snippet in snippets if snippet not in contents]
    if missing:
        joined = "\n- ".join(missing)
        print(f"NOTICE is missing required attribution snippets:\n- {joined}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
