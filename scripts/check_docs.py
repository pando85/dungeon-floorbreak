#!/usr/bin/env python3
"""Documentation-only repository checks (no third-party Python dependencies)."""
from __future__ import annotations

import hashlib
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
    # Verify version-controlled approved concept image(s) against their manifests.
    # This is intentionally strict: silently changing the approved master is not allowed.
    for sums_file in sorted(ROOT.rglob("SHA256SUMS")):
        for line_number, record in enumerate(
            sums_file.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if not record.strip() or record.startswith("#"):
                continue
            match = re.fullmatch(r"([0-9a-fA-F]{64})  (.+)", record)
            if not match:
                failures.append(f"Invalid checksum manifest entry: {sums_file.relative_to(ROOT)}:{line_number}")
                continue
            expected_hash, filename = match.groups()
            target = (sums_file.parent / filename).resolve()
            if not target.is_relative_to(sums_file.parent.resolve()) or not target.is_file():
                failures.append(f"Missing or invalid approved asset: {sums_file.relative_to(ROOT)}:{line_number} -> {filename}")
                continue
            actual_hash = hashlib.sha256(target.read_bytes()).hexdigest()
            if actual_hash.lower() != expected_hash.lower():
                failures.append(f"Approved asset checksum mismatch: {target.relative_to(ROOT)}")

    # Approved *mood* references must retain their original file identities.
    # The background-only file can be absent during restoration, but an incorrect
    # substitute or an accidentally duplicated action image must fail CI.
    mood_dir = ROOT / "assets/references/mood"
    expected_mood_hashes = {
        "a_detailed_pixel_art_game_screenshot_scene_a_vibr.png":
            "9d6cd32fcee3d9e5646fbc85b8562da3ca842236b2a665f9b1d6804b19f39847",
        "a_highly_detailed_pixel_art_background_matte.png":
            "6d1867a67a9893c46d1fc66f862a5145c1d4080bb805405ec08200b900805e72",
    }
    available_hashes = {}
    for name, expected_hash in expected_mood_hashes.items():
        file = mood_dir / name
        if not file.exists():
            if name.startswith("a_detailed_"):
                failures.append(f"Required mood reference missing: {file.relative_to(ROOT)}")
            else:
                print(f"Mood reference awaiting restoration: {file.relative_to(ROOT)}")
            continue
        actual_hash = hashlib.sha256(file.read_bytes()).hexdigest()
        available_hashes[name] = actual_hash
        if actual_hash != expected_hash:
            failures.append(f"Mood reference checksum mismatch: {file.relative_to(ROOT)}")
    if len(available_hashes) > 1 and len(set(available_hashes.values())) < len(available_hashes):
        failures.append("Duplicate mood reference binaries: expected distinct images")

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
