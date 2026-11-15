#!/usr/bin/env python3
"""Documentation-only repository checks (no third-party Python dependencies)."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
MD_LINK = re.compile(r"(?<!!)\[[^]]+\]\(([^)]+)\)")
REQUIRED = [
    "README.md", "AGENTS.md", "NOTICE.md",
    "docs/01-design/vision.md", "docs/01-design/characters.md",
    "docs/02-art/art-direction.md", "docs/03-tech/destruction.md",
    "docs/04-production/vertical-slice.md", "docs/04-production/backlog.md",
]


def main() -> int:
    failures: list[str] = []
    md_files = sorted(ROOT.rglob("*.md"))
    json_files = sorted(ROOT.rglob("*.json"))
    for rel in REQUIRED:
        if not (ROOT / rel).is_file():
            failures.append(f"Missing required file: {rel}")
    for file in md_files:
        text = file.read_text(encoding="utf-8")
        if not text.endswith("\n"):
            failures.append(f"Missing final newline: {file.relative_to(ROOT)}")
        if not text.startswith("# "):
            failures.append(f"No level-one heading: {file.relative_to(ROOT)}")
        for match in MD_LINK.finditer(text):
            dest = match.group(1).strip().split(maxsplit=1)[0].strip("<>")
            if dest.startswith(("https://", "http://", "mailto:", "#")) or not dest:
                continue
            location = unquote(dest.split("#", 1)[0].split("?", 1)[0])
            if location and not (file.parent / location).exists():
                failures.append(f"Broken local link: {file.relative_to(ROOT)} -> {dest}")
        if "TODO_FILL_ME" in text or "lorem ipsum" in text.lower():
            failures.append(f"Unfinished placeholder: {file.relative_to(ROOT)}")
    for file in json_files:
        try:
            json.loads(file.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            failures.append(f"Bad JSON: {file.relative_to(ROOT)}: {exc}")
    if failures:
        print("Documentation checks FAILED:")
        for fail in failures:
            print(" -", fail)
        return 1
    print(f"Documentation checks passed: {len(md_files)} Markdown files, {len(json_files)} JSON fixtures, {len(REQUIRED)} required files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
