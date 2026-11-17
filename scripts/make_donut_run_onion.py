#!/usr/bin/env python3
"""Back-line and rump onion skin for the ANIM-DONUT-01 run (Pillow only, deterministic).

Overlays the `body` layer of run frames 4-9 of the committed grid, enlarged, one colour per
frame: a translucent silhouette, the back-top line (thick) and the rump's left edge (a tick
on the ground line) so the owner can see how the spine and rump move between frames. The
face's left column is marked too, because body length = face x - rump left edge. A review
aid, not an approval.

Usage::

    python3 scripts/make_donut_run_onion.py --out docs/evidence/anim-donut-01/run_onion_backline_rump_16x.png
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
GRID = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
FRAMES = [4, 5, 6, 7, 8, 9]
COLOURS = {4: (255, 214, 64), 5: (255, 140, 60), 6: (240, 80, 110), 7: (190, 110, 255),
           8: (90, 190, 255), 9: (90, 230, 150)}
SCALE = 16
X0, X1, Y0, Y1 = 8, 60, 14, 46     # crop (columns 8..51, rows 14..45)
MATTE = (25, 26, 44)
GRIDLINE = (36, 38, 62)
TEXT = (235, 235, 245)


def font(size: int):
    return ImageFont.load_default(size=size)


def body_top(rows: list[str], x: int) -> int | None:
    return next((y for y, r in enumerate(rows) if r[x] != "."), None)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    grid = json.loads(GRID.read_text(encoding="utf-8"))
    w, h = (X1 - X0) * SCALE, (Y1 - Y0) * SCALE
    head, foot = 64, 150
    img = Image.new("RGB", (max(w, 1330), head + h + foot), (10, 10, 14))
    pic = Image.new("RGBA", (w, h), MATTE + (255,))
    d = ImageDraw.Draw(pic)
    for x in range(X0, X1 + 1):
        d.line(((x - X0) * SCALE, 0, (x - X0) * SCALE, h), fill=GRIDLINE + (255,), width=3 if x % 10 == 0 else 1)
    for y in range(Y0, Y1 + 1):
        d.line((0, (y - Y0) * SCALE, w, (y - Y0) * SCALE), fill=GRIDLINE + (255,), width=3 if y % 10 == 0 else 1)
    for x in range((X0 + 9) // 10 * 10, X1, 10):
        d.text(((x - X0) * SCALE + 4, 2), str(x), font=font(16), fill=(120, 120, 160, 255))
    for y in range((Y0 + 9) // 10 * 10, Y1, 10):
        d.text((4, (y - Y0) * SCALE + 2), str(y), font=font(16), fill=(120, 120, 160, 255))
    ground_y = (44 - Y0) * SCALE
    d.line((0, ground_y, w, ground_y), fill=(150, 112, 90, 255), width=3)
    summary = []
    for frame in FRAMES:
        cels = grid["frames"][frame]["cels"]
        body, face = cels["body"], cels["face"]
        col = COLOURS[frame]
        layer = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        for y, row in enumerate(body):
            for x, c in enumerate(row):
                if c != "." and X0 <= x < X1 and Y0 <= y < Y1:
                    ld.rectangle(((x - X0) * SCALE, (y - Y0) * SCALE, (x - X0 + 1) * SCALE - 1,
                                  (y - Y0 + 1) * SCALE - 1), fill=col + (46,))
        pic.alpha_composite(layer)
        d = ImageDraw.Draw(pic)
        pts = []
        edge = min(x for x, r in ((x, [row[x] for row in cels["head"][17:21]]) for x in range(72))
                   if any(c != "." for c in r))
        for x in range(X0, min(X1, edge)):
            t = body_top(body, x)
            if t is not None and Y0 <= t < Y1:
                pts += [((x - X0) * SCALE, (t - Y0) * SCALE + 2), ((x - X0 + 1) * SCALE, (t - Y0) * SCALE + 2)]
        d.line(pts, fill=col + (255,), width=4)
        rump = min(x for r in body for x, c in enumerate(r) if c != ".")
        fx = min(x for r in face for x, c in enumerate(r) if c != ".")
        rx = (rump - X0) * SCALE
        d.line((rx, (30 - Y0) * SCALE, rx, ground_y + 24), fill=col + (255,), width=3)
        d.text((rx + 4, ground_y + 6 + (frame - 4) * 0), str(frame), font=font(20), fill=col + (255,))
        fxx = (fx - X0) * SCALE
        d.line((fxx, (21 - Y0) * SCALE, fxx, (29 - Y0) * SCALE), fill=col + (255,), width=3)
        summary.append((frame, rump, fx, fx - rump))
    img.paste(pic.convert("RGB"), (0, head))
    d = ImageDraw.Draw(img)
    d.text((10, 6), "ANIM-DONUT-01 run, body layer of frames 4-9 overlaid at 16x. Thick line = back top (up to the "
           "head), vertical line down to the", font=font(19), fill=TEXT)
    d.text((10, 32), "ground = rump left edge (labelled by frame), short vertical line at the head = face left column.",
           font=font(19), fill=TEXT)
    y = head + h + 10
    x = 10
    for frame, rump, fx, length in summary:
        col = COLOURS[frame]
        d.rectangle((x, y + 4, x + 18, y + 22), fill=col)
        d.text((x + 26, y), f"frame {frame}", font=font(20), fill=col)
        d.text((x, y + 30), f"rump x {rump}", font=font(18), fill=TEXT)
        d.text((x, y + 54), f"face x {fx}", font=font(18), fill=TEXT)
        d.text((x, y + 78), f"body length {length}", font=font(18), fill=TEXT)
        x += 200
    d.text((10, y + 108), "Rump and head move together (rump steps at most 3 px, body length 18-25 px), so the "
           "stretch reads as the spine flexing, not as the rump sliding under a fixed head.", font=font(18), fill=TEXT)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    img.save(args.out, optimize=False)
    print(args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
