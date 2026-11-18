#!/usr/bin/env python3
"""Deterministic authoring script for SCAT-01 (Scatterer readability set).

Writes ``assets/source/characters/scatterer/scat-01.grid.json`` (pixel-grid-v1) and
the matching ``.ase`` via ``scripts/pixel_grid_to_ase.py``. Design spec:
``docs/02-art/scat-01-spec.md``.

Design phase (top tier): only key frame 0 (``idle``, the master silhouette) is
authored here, as literal pixel rows. Frames 1 to 15 are PLACEHOLDER copies of
frame 0 so that the tag layout and tooling work; the execution tier replaces
each of them from the pose descriptions in spec section 7 and 8 (add one
``frame_N()`` function per frame and keep frame 0 byte-identical: it is frozen
in ``tests/fixtures/scat-01_keys.json``).

No concept pixels are traced or resampled; every pixel is placed by this file.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "source" / "characters" / "scatterer"
GRID = OUT_DIR / "scat-01.grid.json"
ASE = OUT_DIR / "scat-01.ase"

W, H = 48, 32
PIVOT = [24, 31]

# Palette `scatterer-floor01-v1`: 16 keys used, 4 slots reserved (spec section 6).
PALETTE = [
    # key, name, rgba, accent
    ("K", "outline_char", "#1c0e12ff", False),
    ("a", "chitin_0_deep", "#3d1519ff", False),
    ("b", "chitin_1_shade", "#5f1d1dff", False),
    ("c", "chitin_2_mid", "#84291fff", False),
    ("d", "chitin_3_light", "#ab4329ff", False),
    ("f", "chitin_4_lit", "#d0693aff", False),
    ("h", "sheen_torch", "#ea9c5cff", False),
    ("H", "glint_cream", "#f7dc9cff", True),
    ("g", "leg_dark", "#4a2a28ff", False),
    ("j", "leg_mid", "#8a5240ff", False),
    ("s", "leg_spine", "#b98660ff", True),
    ("u", "belly_cold", "#3e2f3aff", False),
    ("v", "rim_cold", "#66545eff", False),
    ("e", "eye_rim", "#b81c1cff", True),
    ("r", "eye_glow", "#ff4a32ff", True),
    ("w", "eye_flash", "#efe3d3ff", True),
]

# Frame 0 `idle` key, facing right, contact row 31, pivot (24, 31).
# Columns 0..47; rows 0..14 are empty. Authored by the top tier (design key).
IDLE0 = [
    # 0         1         2         3         4
    # 012345678901234567890123456789012345678901234567
    "................................................",  # 15
    "................................................",  # 16
    "............................bbb.................",  # 17
    "...............................bb...............",  # 18
    ".................................b..............",  # 19
    "................KKKKKKKKKK........b.........jj..",  # 20
    ".............KKKhhhahHhhafKKK......b.......j..j.",  # 21
    "...........KKfffffahhhhafffffKK.....b.....j...j.",  # 22
    "..........KdddddddaddddaddddddKKKK..b.....j.....",  # 23
    ".........KcccccccaccccacccccccKhhhKK.b...j......",  # 24
    ".........KcccccccaccccaccccccccKhfffKb...j......",  # 25
    ".......s.KbbbbbbabbbbabbbbbbbbbKfddddKKKj.......",  # 26
    ".......jj.KvvvvvaaaaaaaaaaaaaaaaKcccKarHcK......",  # 27
    "......g..j.KKKKjKKKKKKKjKKKKKKKKjKKKKaercK......",  # 28
    "....sg....jjjjj..a...gj....a.....jjs.KaHHK......",  # 29
    "....g...........a...gs......a......gg.KKKa......",  # 30
    "..gg..........aa..gg.........aa......gg...aa....",  # 31
]
FIRST_ROW = 15


def blank() -> list[str]:
    return ["." * W for _ in range(H)]


def frame_idle0() -> list[str]:
    rows = blank()
    for i, line in enumerate(IDLE0):
        assert len(line) == W, (i, len(line))
        rows[FIRST_ROW + i] = line
    return rows


# Frame table (spec section 4): index -> (tag, ticks). Ticks are 60 Hz.
TAGS = [
    ("idle", 0, 2),
    ("walk", 3, 6),
    ("attack_tell", 7, 8),
    ("attack_swing", 9, 11),
    ("hurt", 12, 12),
    ("down", 13, 15),
]
TICKS = [10, 6, 10, 4, 4, 4, 4, 6, 20, 2, 5, 14, 10, 4, 8, 0]


def build() -> dict:
    key0 = frame_idle0()
    frames = []
    for i, ticks in enumerate(TICKS):
        # PLACEHOLDER: frames 1..15 are copies of key 0 until the execution tier draws them.
        cel = list(key0)
        frames.append({"duration_ms": max(1, round((ticks or 6) * 1000 / 60)), "cels": {"body": cel}})
    return {
        "format": "pixel-grid-v1",
        "canvas": [W, H],
        "pivot": PIVOT,
        "palette": [
            {"key": k, "name": n, "rgba": c, **({"accent": True} if acc else {})}
            for k, n, c, acc in PALETTE
        ],
        "layers": ["body"],
        "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    grid = build()
    GRID.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
