#!/usr/bin/env python3
"""SLIME-01 scale evidence: key 0 on the lit Floor 1 walking line next to Carl and the Scatterer, at 1x and 4x.

Needs the Floor 1 kit atlases (``bash scripts/export_assets.sh``). Writes ``docs/evidence/slime-01/key0-scale.png`` and the
keys contact sheet ``docs/evidence/slime-01/keys-x3.png`` (keys 0, 3, 8, 10, 12, 15 at 3x on brick-dark, with 1x row).
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from pixelart.common import flip, grid_images, label, load_grid_file  # noqa: E402
from pixelart.scene import Floor, reference_idle  # noqa: E402

OUT = ROOT / "docs" / "evidence" / "slime-01" / "key0-scale.png"
KEYS_OUT = OUT.with_name("keys-x3.png")
KEYS = (0, 3, 8, 10, 12, 15)
NAMES = ("0 idle", "3 creep", "8 tell", "10 lunge", "12 hurt", "15 down")
SLIME = ROOT / "assets/source/characters/slime/slime_01.grid.json"
SCAT = ROOT / "assets/source/characters/scatterer/scat-01.grid.json"


def main() -> int:
    floor = Floor()
    carl, carl_piv = reference_idle("carl")
    entries = [(carl, carl_piv, 452, 0)]
    for path, x in ((SCAT, 500), (SLIME, 580)):
        grid = load_grid_file(path)
        img, piv = flip(grid_images(grid)[0], tuple(grid["pivot"]))  # enemies face Carl on their left
        entries.append((img, piv, x, 0))
    scene = floor.render(entries, 1)
    ground = floor.ground(452)
    # crop a band around the walking line (view coordinates); the view origin is its top-left corner
    ground_in_view = ground - floor.view[1]
    box = (60, max(0, ground_in_view - 66), 60 + 320, min(scene.height, ground_in_view + 12))
    crop = scene.crop(box)
    big = crop.resize((crop.width * 4, crop.height * 4), Image.NEAREST)
    pad = 6
    sheet = Image.new("RGB", (big.width + 2 * pad, crop.height + big.height + 4 * pad + 30), (20, 18, 28))
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 2), "SLIME-01 key 0: Carl 52 px, Scatterer 15 px, Green Slime 30 px; lit Floor 1, 1x then 4x")
    sheet.paste(crop, (pad, 18))
    sheet.paste(big, (pad, 18 + crop.height + pad))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(OUT)
    print(f"wrote {OUT.relative_to(ROOT)} {sheet.size} crop {box}")
    keys_sheet()
    return 0


def keys_sheet() -> None:
    grid = load_grid_file(SLIME)
    imgs = grid_images(grid)
    w, h = grid["canvas"]
    s, pad, cols = 3, 8, 3
    rows = (len(KEYS) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (w * s + pad) + pad, 18 + rows * (h * s + 16 + pad) + h + 2 * pad), (48, 40, 38))
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 2), "SLIME-01 keys at 3x (frame 80 x 40, pivot 40,39, floor row 39); 1x row below")
    for i, k in enumerate(KEYS):
        x = pad + (i % cols) * (w * s + pad)
        y = 18 + (i // cols) * (h * s + 16 + pad)
        label(d, (x, y), NAMES[i])
        big = imgs[k].convert("RGBA").resize((w * s, h * s), Image.NEAREST)
        sheet.paste(big, (x, y + 14), big)
        d.line([(x, y + 14 + (h - 1) * s + s), (x + w * s - 1, y + 14 + (h - 1) * s + s)], fill=(90, 74, 66))
        one = imgs[k].convert("RGBA")
        sheet.paste(one, (pad + i * (w + 4), sheet.height - h - pad), one)
    sheet.save(KEYS_OUT)
    print(f"wrote {KEYS_OUT.relative_to(ROOT)} {sheet.size}")


if __name__ == "__main__":
    raise SystemExit(main())
