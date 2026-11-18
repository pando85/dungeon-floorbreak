#!/usr/bin/env python3
"""ROT-01 evidence: key 0 at 1x on the lit Floor 1 wall next to Carl, plus a 4x copy.

Usage: python3 scripts/make_rot01_scale.py [OUT.png]   (default docs/evidence/rot-01/key0-scale.png)
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


def main(out: Path) -> None:
    cfg = load_config(str(CFG))
    sprite = common.grid_images(cfg.load_grid())[0]
    carl, carl_piv = reference_idle("carl")
    entries = [(carl, carl_piv, common.CARL_X, 0), (sprite, tuple(cfg.pivot), STICKER_X, LIFT)]
    scene = Floor().render(entries, 1)  # 640x360, lit
    floor = Floor()
    gy = floor.ground(STICKER_X)
    vx, vy = floor.view[0], floor.view[1]  # world -> image offset
    cx0, cy0 = common.CARL_X - 70 - vx, gy - 80 - vy
    crop = scene.crop((cx0, cy0, cx0 + 170, cy0 + 90))
    sx, sy = STICKER_X - cfg.pivot[0] - 4 - vx, gy - cfg.pivot[1] - LIFT - 4 - vy
    zoom = scene.crop((sx, sy, sx + 36, sy + 36)).resize((144, 144), Image.NEAREST)
    sheet = Image.new("RGB", (170 + 12 + 144 + 24, 160), common.BG)
    sheet.paste(crop, (12, 28))
    sheet.paste(zoom, (170 + 24, 28))
    d = ImageDraw.Draw(sheet)
    d.text((12, 8), "key 0 at 1x beside Carl (52 px), lit wall", fill=common.WHITE)
    d.text((170 + 24, 8), "same key, 4x", fill=common.WHITE)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    main(Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs/evidence/rot-01/key0-scale.png")
