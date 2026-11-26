#!/usr/bin/env python3
"""HRD-CUES-01 evidence: contact strips and GIFs of the cues from the 1920x1080 window sequences (scale 3).

    hrd_cues_strip.py S1_DIR S2_DIR GIVE_DIR OUT_DIR

Each strip crops the same box out of the 3x frames of one tick-per-frame sequence (nearest-neighbour only: the window
already renders at 3x, no resampling filter is used) and labels each cell with its tick; the GIFs play the whole sequence
cropped the same way at 100 ms per tick (slowed down 6x)."""
from __future__ import annotations

import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw

# crop boxes on the 1920x1080 frames (left, top, right, bottom): the visible part of the mound, the post and its stack
BOXES = {
    "s1": (1320, 640, 1920, 1000),
    "s2": (1260, 700, 1920, 1060),
    "give": (1500, 330, 1920, 900),
}
TICKS = {
    "s1": [42, 45, 48, 51, 54, 60, 66],
    "s2": [107, 108, 110, 112, 114, 115],
    "give": [205, 206, 208, 210, 213, 214, 216],
}
NAMES = {"s1": "stir_s1", "s2": "stir_s2", "give": "give"}


def frames(directory: Path) -> dict[int, Path]:
    found = {}
    for p in sorted(directory.glob("frame_*_tick_*.png")):
        found[int(re.search(r"tick_(\d+)", p.name).group(1))] = p
    return found


def main(argv: list[str]) -> int:
    *dirs, out = argv
    out = Path(out)
    for key, directory in zip(("s1", "s2", "give"), dirs):
        seq = frames(Path(directory))
        box = BOXES[key]
        crops = {t: Image.open(p).convert("RGB").crop(box) for t, p in seq.items()}
        w, h = crops[next(iter(crops))].size
        cells = []
        for t in TICKS[key]:
            cell = crops[t].copy()
            ImageDraw.Draw(cell).text((6, 4), f"t{t}", fill=(255, 255, 255))
            cells.append(cell)
        strip = Image.new("RGB", (w * len(cells) + 4 * (len(cells) - 1), h), (0, 0, 0))
        for i, cell in enumerate(cells):
            strip.paste(cell, (i * (w + 4), 0))
        strip.save(out / f"strip_{NAMES[key]}.png", optimize=True)
        gif = [crops[t] for t in sorted(crops)]
        gif[0].save(out / f"{NAMES[key]}.gif", save_all=True, append_images=gif[1:], duration=100, loop=0, optimize=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
