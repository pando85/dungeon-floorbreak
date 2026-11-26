#!/usr/bin/env python3
"""AREA-HEAT-01: the drawn view left of a runtime still, found from landmarks (the logged ``cam`` is not the drawn
view). The kit's lit model renders the dressed room once at full width; a 640 x 360 window slid over it is matched
against the still's wall rows (the rows under the HUD plates and above the actors: screen rows 64..190), where the
static brick, arches and torch pools carry the position. The best left is the one with the least squared error;
the drawn view top is 289 (spec section 2).

    python3 scripts/heat01_view.py LAYOUT_DIR STILL.png [STILL.png ...]
"""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import env_kit as ek  # noqa: E402
import env_kit_light as lit  # noqa: E402

CONTRACT = "assets/source/terrain/floor01/floor01_heat_kit.json"
ROOM_W, VIEW_TOP, ROWS = 2320, 289, (64, 190)


def room_strip(layout_dir: Path) -> np.ndarray:
    import json
    kit = ek.load_kit(CONTRACT)
    layout = json.loads((layout_dir / "corral.dressed.layout.json").read_text(encoding="utf-8"))
    img = lit.render_lit(kit, layout, [0, VIEW_TOP, ROOM_W, 360], 1)
    return np.asarray(img.convert("L"), dtype=np.float32)


def locate(strip: np.ndarray, still: Path) -> tuple[int, float]:
    s = np.asarray(Image.open(still).convert("L"), dtype=np.float32)[ROWS[0]:ROWS[1]]
    best = (1e30, 0)
    for left in range(0, ROOM_W - 640 + 1):
        win = strip[ROWS[0]:ROWS[1], left:left + 640]
        # the HUD plates cover the top corners: rows 64.. are clear of them, but the actors are not static
        diff = np.sort(np.abs(win - s), axis=None)
        err = float(diff[: int(diff.size * 0.7)].mean())          # trimmed: actors, HUD text and flames drop out
        if err < best[0]:
            best = (err, left)
    return best[1], best[0]


def main(argv: list[str]) -> int:
    strip = room_strip(Path(argv[0]))
    for f in argv[1:]:
        left, err = locate(strip, Path(f))
        print(f"{Path(f).name} drawn_view_left={left} trimmed_abs_err={err:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
