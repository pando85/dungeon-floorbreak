#!/usr/bin/env python3
"""Footfall diagram for the ANIM-DONUT-01 run (Pillow only, deterministic).

Draws run frames 4-9 of the committed grid (``donut_h40_locomotion.grid.json``) enlarged
and side by side, with the ground line (bottom edge of row 43), the pivot pixel, the paw
that touches row 43 highlighted and labelled, and the airborne frames labelled with the
gap between their lowest paw and the ground. A review aid for the owner, not an approval.

Usage::

    python3 scripts/make_donut_run_diagram.py --out docs/evidence/anim-donut-01/run_footfall_diagram.png
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import pixel_grid_to_ase as pg  # noqa: E402

GRID = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
FRAMES = [4, 5, 6, 7, 8, 9]
NAMES = {4: "F0 near fore", 5: "F1 far fore", 6: "F2 near hind", 7: "F3 far hind"}
LEG_LAYER = {4: "legs_near", 5: "legs_far", 6: "legs_near", 7: "legs_far"}
SCALE = 8
X0, X1 = 4, 70                       # crop columns (the sprite lives in 6..69)
Y0 = 2                               # crop top row
MATTE = (25, 26, 44)
FLOOR = (40, 36, 52)
GROUND = (150, 112, 90)
PIVOT = (60, 255, 120)
HILITE = (255, 214, 64)
AIR = (110, 190, 255)
TEXT = (235, 235, 245)
DIM = (170, 170, 190)


def font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    return ImageFont.load_default(size=size)


def paw_box(grid: dict, frame: int) -> tuple[int, int, int, int]:
    """Bounding box (x0, y0, x1, y1) of the leg touching row 43, flood-filled in its layer."""
    rows = grid["frames"][frame]["cels"][LEG_LAYER[frame]]
    seen: set[tuple[int, int]] = set()
    stack = [(x, 43) for x, c in enumerate(rows[43]) if c != "."]
    while stack:
        x, y = stack.pop()
        if (x, y) in seen or not (0 <= y < len(rows) and 0 <= x < len(rows[0])) or rows[y][x] == ".":
            continue
        seen.add((x, y))
        stack += [(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
    xs, ys = [p[0] for p in seen], [p[1] for p in seen]
    return min(xs), min(ys), max(xs), max(ys)


def lowest_leg_pixel(grid: dict, frame: int) -> tuple[int, int]:
    """(x, y) of the lowest leg pixel (leftmost on ties)."""
    cels = grid["frames"][frame]["cels"]
    best = (-1, -1)
    for layer in ("legs_near", "legs_far"):
        for y, row in enumerate(cels[layer]):
            for x, c in enumerate(row):
                if c != "." and (y > best[1] or (y == best[1] and x < best[0])):
                    best = (x, y)
    return best


BAND = 78   # caption band under the picture, in output pixels


def panel(grid: dict, frame: int, palette: dict[str, tuple[int, int, int]]) -> Image.Image:
    px, py = grid["pivot"]
    flat = pg.composite_keys(grid, frame)
    w, pic_h = (X1 - X0) * SCALE, (44 - Y0 + 2) * SCALE
    img = Image.new("RGB", (w, pic_h + BAND), MATTE)
    d = ImageDraw.Draw(img)
    ground_y = (py + 1 - Y0) * SCALE
    d.rectangle((0, ground_y, w, pic_h), fill=FLOOR)
    for y, row in enumerate(flat):
        for x, key in enumerate(row):
            if key != "." and X0 <= x < X1 and y >= Y0:
                cx, cy = (x - X0) * SCALE, (y - Y0) * SCALE
                d.rectangle((cx, cy, cx + SCALE - 1, cy + SCALE - 1), fill=palette[key])
    d.line((0, ground_y, w, ground_y), fill=GROUND, width=3)
    # Pivot pixel and a faint vertical guide through it.
    cx, cy = (px - X0) * SCALE, (py - Y0) * SCALE
    d.line((cx + SCALE // 2, 0, cx + SCALE // 2, cy), fill=PIVOT, width=1)
    d.rectangle((cx - 1, cy - 1, cx + SCALE, cy + SCALE), outline=PIVOT, width=3)
    d.text((cx + SCALE + 6, cy + 12), "pivot", font=font(16), fill=PIVOT)
    d.text((6, ground_y + 6), "ground line (row 43)", font=font(16), fill=GROUND)
    small, mid = font(17), font(21)
    ty = pic_h + 8
    if frame in LEG_LAYER:
        bx0, by0, bx1, by1 = paw_box(grid, frame)
        d.rectangle(((bx0 - X0) * SCALE - 4, (by0 - Y0) * SCALE - 4,
                     (bx1 + 1 - X0) * SCALE + 3, (by1 + 1 - Y0) * SCALE + 3), outline=HILITE, width=4)
        contact = [x for x, c in enumerate(grid["frames"][frame]["cels"][LEG_LAYER[frame]][43]) if c != "."]
        d.text((8, ty), f"CONTACT: {NAMES[frame]}", font=mid, fill=HILITE)
        d.text((8, ty + 26), f"paw on row 43, x {min(contact)}-{max(contact)}", font=small, fill=HILITE)
        d.text((8, ty + 48), "the only paw on the ground: one contact frame", font=small, fill=DIM)
    else:
        lx, ly = lowest_leg_pixel(grid, frame)
        gap = py - ly
        x = (lx - X0) * SCALE + SCALE // 2
        y0 = (ly + 1 - Y0) * SCALE
        d.line((x, y0, x, ground_y), fill=AIR, width=3)
        d.line((x - 8, y0, x + 8, y0), fill=AIR, width=3)
        d.line((x - 8, ground_y - 2, x + 8, ground_y - 2), fill=AIR, width=3)
        d.text((x + 12, (y0 + ground_y) // 2 - 9), f"{gap} px", font=small, fill=AIR)
        d.text((8, ty), "AIRBORNE: no paw on row 43", font=mid, fill=AIR)
        d.text((8, ty + 26), f"lowest paw row {ly}, {gap} px above the ground", font=small, fill=AIR)
        d.text((8, ty + 48), "nothing planted", font=small, fill=DIM)
    return img


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    grid = json.loads(GRID.read_text(encoding="utf-8"))
    palette = {e["key"]: tuple(int(e["rgba"].lstrip("#")[i:i + 2], 16) for i in (0, 2, 4))
               for e in grid["palette"]}
    panels = [panel(grid, f, palette) for f in FRAMES]
    pw, ph = panels[0].size
    gap, head, foot = 10, 52, 62
    cols = 3
    rows = 2
    img = Image.new("RGB", (cols * pw + (cols + 1) * gap, head + rows * (ph + 34) + foot), (10, 10, 14))
    d = ImageDraw.Draw(img)
    d.text((gap, 10), "ANIM-DONUT-01 run, frames 4-9, 8x. Ground line = bottom edge of row 43; "
           "green box = pivot (46, 43); yellow = the paw on row 43.",
           font=font(22), fill=TEXT)
    for i, (frame, p) in enumerate(zip(FRAMES, panels)):
        cx = gap + (i % cols) * (pw + gap)
        cy = head + (i // cols) * (ph + 34)
        ms = grid["frames"][frame]["duration_ms"]
        d.text((cx, cy + 6), f"frame {frame}   {ms} ms", font=font(21), fill=TEXT)
        img.paste(p, (cx, cy + 34))
    d.text((gap, img.height - foot + 10),
           "Run order 4 > 5 > 6 > 7 > 8 > 9 > 4. Each paw is on row 43 in exactly one frame, so a planted paw has no "
           "frame-to-frame slide; it still glides 14.8 px of ground during that frame.", font=font(20), fill=TEXT)
    d.text((gap, img.height - foot + 36),
           "Playback: frame_ms = clamp(round(w * 1000 * 14.8 / |vx|), 50, 130), w = 1 except frame 8 = 1.125 "
           "(80 ms, frame 8 90 ms at 185 px/s).", font=font(20), fill=DIM)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    img.save(args.out, optimize=False)
    print(args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
