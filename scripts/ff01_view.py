#!/usr/bin/env python3
"""AREA-FASTFOOD-01: the drawn view left of a runtime still, found from landmarks (the logged ``cam`` is not the drawn
view: the camera eases per rendered frame). The kit's lit model renders the dressed refuge once at full width; a 640 x 360
window slid over it is matched against the still's wall rows (the rows under the HUD plates and above the actors: screen
rows 64..190), where the static brick, tiles, strips and arches carry the position. The best left is the one with the least
trimmed squared error (actors, the notice and the door and box states, which the model does not draw, fall in the trimmed
tail); the view top is searched too.

    python3 scripts/ff01_view.py LAYOUT_DIR STILL.png [STILL.png ...]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import env_kit as ek  # noqa: E402
import env_kit_light as lit  # noqa: E402

CONTRACT = "assets/source/terrain/floor01/floor01_fastfood_kit.json"
ROOM_W, ROOM_H, ROWS = 1600, 672, (64, 190)
TOPS = range(280, 306)          # candidate drawn view tops (the gate's stills are 292)


def room_image(layout_dir: Path):
    """The dressed room, lit, over its whole height, and the kit (the view top differs per still)."""
    kit = ek.load_kit(CONTRACT)
    layout = json.loads((layout_dir / "rest_refuge.dressed.layout.json").read_text(encoding="utf-8"))
    return lit.render_lit(kit, layout, [0, 0, ROOM_W, ROOM_H], 1)


def room_strip(layout_dir: Path) -> np.ndarray:
    return np.asarray(room_image(layout_dir).convert("L"), dtype=np.float32)


def _err(win: np.ndarray, s: np.ndarray) -> float:
    diff = np.sort(np.abs(win - s), axis=None)
    return float(diff[: int(diff.size * 0.7)].mean())      # trimmed: actors, HUD text and presenter art drop out


def locate(strip: np.ndarray, still: Path) -> tuple[int, int, float]:
    """(left, top, error): a coarse 2D search on a 4x downsample, then a full-resolution refinement."""
    s = np.asarray(Image.open(still).convert("L"), dtype=np.float32)[ROWS[0]:ROWS[1]]
    s4 = s[::4, ::4]
    best = (1e30, 0, 0)
    for top in TOPS:
        for left in range(0, ROOM_W - 640 + 1, 4):
            win = strip[top + ROWS[0]:top + ROWS[1], left:left + 640][::4, ::4]
            err = _err(win, s4)
            if err < best[0]:
                best = (err, left, top)
    _, l0, t0 = best
    best = (1e30, 0, 0)
    for top in range(t0 - 4, t0 + 5):
        for left in range(max(0, l0 - 6), min(ROOM_W - 640, l0 + 6) + 1):
            err = _err(strip[top + ROWS[0]:top + ROWS[1], left:left + 640], s)
            if err < best[0]:
                best = (err, left, top)
    return best[1], best[2], best[0]


def main(argv: list[str]) -> int:
    strip = room_strip(Path(argv[0]))
    for f in argv[1:]:
        left, top, err = locate(strip, Path(f))
        print(f"{Path(f).name} drawn_view_left={left} drawn_view_top={top} trimmed_abs_err={err:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
