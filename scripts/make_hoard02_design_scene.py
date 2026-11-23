#!/usr/bin/env python3
"""HOARD-02 design evidence: the gate-fixed boss keys and the lair mound on lit Floor 1 brick, at gameplay scale.

Offline composites from the committed grids (not runtime captures): the A1 lighting pass of the Floor 1 kit via
``scripts/pixelart/scene.py``, Carl and Donut at their approved scale, the boss mirrored to face them (the runtime
rule). Needs the kit atlases (``bash scripts/export_assets.sh``). Writes 640x360 and 1920x1080 PNGs per scene.

    python3 scripts/make_hoard02_design_scene.py docs/evidence/hoard-02-design
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]

from PIL import Image  # noqa: E402

from scripts.pixelart import scene  # noqa: E402
from scripts.pixelart.common import CARL_X, DONUT_X, flip, grid_images  # noqa: E402

BOSS = ROOT / "assets/source/characters/hoarder/hoard-02.grid.json"
MOUND = ROOT / "assets/source/props/floor01/hoarder_mound.grid.json"

# scene name -> boss (frame, x) pairs; the mound scene adds the sealed mound around the emerge key
SCENES = {
    "lit-brace-lunge": [(17, 600), (18, 800)],
    "lit-drag-getup": [(19, 600), (24, 800)],
    "lit-mound-emerge": [(26, 640)],
}


def layer_images(grid: dict) -> dict[str, Image.Image]:
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}
    out = {}
    for name in grid["layers"]:
        rows = grid["frames"][0]["cels"][name]
        im = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch != ".":
                    im.putpixel((x, y), pal[ch])
        out[name] = im
    return out


def main(argv=None) -> int:
    out = Path((argv or sys.argv[1:])[0])
    out.mkdir(parents=True, exist_ok=True)
    boss = json.loads(BOSS.read_text())
    frames, pivot = grid_images(boss), tuple(boss["pivot"])
    mound = json.loads(MOUND.read_text())
    mlayers, mpivot = layer_images(mound), tuple(mound["pivot"])
    carl, carl_piv = scene.reference_idle("carl")
    donut, donut_piv = scene.reference_idle("donut")
    floor = scene.Floor()
    for name, actors in SCENES.items():
        entries = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
        for frame, x in actors:
            img, piv = flip(frames[frame], pivot)
            if name == "lit-mound-emerge":
                back, bpiv = flip(mlayers["back"], mpivot)
                front, fpiv = flip(mlayers["front"], mpivot)
                entries += [(back, bpiv, x, 0, False), (img, piv, x, 0, False), (front, fpiv, x, 0, True)]
            else:
                entries.append((img, piv, x, 0))
        for k, size in ((1, "640x360"), (3, "1920x1080")):
            path = out / f"{name}_{size}.png"
            floor.render(entries, k).save(path)
            print(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
