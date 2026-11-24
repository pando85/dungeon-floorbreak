#!/usr/bin/env python3
"""AREA-GOBLINWORKSHOP-01 evidence: the steam plume over actors standing in the lane (acceptance 6).

A composite, not a window capture: every Boiler pilot keeps the actors out of the lane
while the steam is Active, so the committed baseline ``09_lane_actors_no_plume_640x360.png``
(real Carl, Donut and a goblin on the lit Floor 1 lane, rendered by the pixel-art toolkit)
gets one ``steam_plume`` frame from the exported atlas at the in-game alpha (0x90, the
``STEAM_ALPHA`` of ``src/env_prop_art.rs``) over the 80 x 96 lane at (72, 191).

Usage: ``python3 scripts/make_gwkit_plume_composite.py [--frame 2]`` (after ``bash scripts/export_assets.sh``)
"""
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SCENE = ROOT / "docs/evidence/f01-gwkit/scene"
ATLAS = ROOT / "generated/art/f01-gw-tell-plume.png"
LANE = (72, 191)
FRAME_W, FRAME_H = 80, 96
STEAM_ALPHA = 0x90
CROP = (40, 140, 200, 300)  # 160 x 160 around the lane for the side-by-side


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--frame", type=int, default=2)
    args = ap.parse_args()
    base = Image.open(SCENE / "09_lane_actors_no_plume_640x360.png").convert("RGBA")
    atlas = Image.open(ATLAS).convert("RGBA")
    x0 = args.frame * FRAME_W
    plume = atlas.crop((x0, 0, x0 + FRAME_W, FRAME_H))
    alpha = plume.getchannel("A").point(lambda a: STEAM_ALPHA if a else 0)
    plume.putalpha(alpha)
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    layer.paste(plume, LANE)
    comp = Image.alpha_composite(base, layer)
    comp.convert("RGB").save(SCENE / "09_plume_over_lane_actors_composite_640x360.png", optimize=True)
    comp.resize((1920, 1080), Image.NEAREST).convert("RGB").save(
        SCENE / "09_plume_over_lane_actors_composite_1920x1080.png", optimize=True)
    side = Image.new("RGB", (970, 480), (0, 0, 0))
    side.paste(base.crop(CROP).resize((480, 480), Image.NEAREST).convert("RGB"), (0, 0))
    side.paste(comp.crop(CROP).resize((480, 480), Image.NEAREST).convert("RGB"), (490, 0))
    side.save(SCENE / "09_plume_vs_baseline_crop_x3.png", optimize=True)


if __name__ == "__main__":
    main()
