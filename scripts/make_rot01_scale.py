#!/usr/bin/env python3
"""ROT-01 evidence on the lit Floor 1 wall.

- ``key0-scale.png``: key 0 at 1x next to Carl, a 4x copy, and the ceiling (rotated) and right-wall (mirrored)
  runtime transforms of key 0 at 4x; a second row puts key 0 on dark stone (1x crop and 4x).
- ``keys-x3.png``: the six keys (0, 3, 8, 10, 11, 14), each lit on the same brick spot, shown at 3x.

Usage: python3 scripts/make_rot01_scale.py [OUT_DIR]   (default docs/evidence/rot-01)
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from scripts.pixelart import common  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart.scene import Floor, reference_idle  # noqa: E402

CFG = ROOT / "assets/source/characters/rot_sticker/rot_01.pixelart.json"
STICKER_X, LIFT = 482, 30  # wall-mounted at Carl's chest height, 30 px above the walking line
DARK_X = STICKER_X + 110  # the dark blue stone wall section
KEYS = (0, 3, 8, 10, 11, 14)
LABELS = ("0 idle", "3 creep", "8 tell", "10 burst", "11 hurt", "14 dud")


def _cell(scene: Image.Image, floor: Floor, x: int, piv, size: int, k: int) -> Image.Image:
    vx, vy = floor.view[0], floor.view[1]
    sx, sy = x - piv[0] - 1 - vx, floor.ground(x) - piv[1] - LIFT - 1 - vy
    return scene.crop((sx, sy, sx + size, sy + size)).resize((size * k, size * k), Image.NEAREST)


def main(out_dir: Path) -> None:
    cfg = load_config(str(CFG))
    frames = common.grid_images(cfg.load_grid())
    piv = tuple(cfg.pivot)
    floor = Floor()
    carl, carl_piv = reference_idle("carl")
    key0 = frames[0]
    # ceiling: rotate 90 degrees clockwise about the pivot; right wall: mirror about the pivot column
    ceiling = key0.rotate(-90)
    ceil_piv = (key0.height - 1 - piv[1], piv[0])
    mirror = key0.transpose(Image.FLIP_LEFT_RIGHT)
    mirror_piv = (key0.width - 1 - piv[0], piv[1])
    entries = [(carl, carl_piv, common.CARL_X, 0), (key0, piv, STICKER_X, LIFT),
               (ceiling, ceil_piv, STICKER_X + 60, LIFT), (mirror, mirror_piv, STICKER_X + 110, LIFT)]
    scene = floor.render(entries, 1)
    vx, vy = floor.view[0], floor.view[1]
    gy = floor.ground(STICKER_X)
    cx0, cy0 = common.CARL_X - 70 - vx, gy - 80 - vy
    crop = scene.crop((cx0, cy0, cx0 + 170, cy0 + 90))
    z = [_cell(scene, floor, STICKER_X, piv, 34, 4),
         _cell(scene, floor, STICKER_X + 60, ceil_piv, 34, 4),
         _cell(scene, floor, STICKER_X + 110, mirror_piv, 34, 4)]
    sheet = Image.new("RGB", (12 + 170 + 12 + 3 * (136 + 12), 190 + 176), common.BG)
    sheet.paste(crop, (12, 28))
    d = ImageDraw.Draw(sheet)
    d.text((12, 8), "key 0 at 1x beside Carl (52 px)", fill=common.WHITE)
    for i, (img, label) in enumerate(zip(z, ("key 0, 4x", "ceiling (rot 90 cw), 4x", "right wall (mirror), 4x"))):
        x = 12 + 170 + 12 + i * 148
        sheet.paste(img, (x, 28))
        d.text((x, 8), label, fill=common.WHITE)
    dark = floor.render([(key0, piv, DARK_X, LIFT)], 1)
    dx0, dy0 = DARK_X - 85 - vx, floor.ground(DARK_X) - 80 - vy
    sheet.paste(dark.crop((dx0, dy0, dx0 + 170, dy0 + 90)), (12, 214))
    sheet.paste(_cell(dark, floor, DARK_X, piv, 34, 4), (12 + 170 + 12, 214))
    d.text((12, 194), "key 0 at 1x on dark stone", fill=common.WHITE)
    d.text((12 + 170 + 12, 194), "dark stone, 4x", fill=common.WHITE)
    out_dir.mkdir(parents=True, exist_ok=True)
    sheet.save(out_dir / "key0-scale.png")

    # one render per key at the same brick spot, so every key is judged on the same background and light
    cells = [_cell(floor.render([(frames[f], piv, STICKER_X, LIFT)], 1), floor, STICKER_X, piv, 34, 3) for f in KEYS]
    sheet = Image.new("RGB", (12 + len(KEYS) * (102 + 8), 102 + 40), common.BG)
    d = ImageDraw.Draw(sheet)
    for i, (img, label) in enumerate(zip(cells, LABELS)):
        sheet.paste(img, (12 + i * 110, 28))
        d.text((12 + i * 110, 8), label, fill=common.WHITE)
    sheet.save(out_dir / "keys-x3.png")
    print(f"wrote {out_dir / 'key0-scale.png'} and {out_dir / 'keys-x3.png'}")


if __name__ == "__main__":
    main(Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs/evidence/rot-01")
