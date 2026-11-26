#!/usr/bin/env python3
"""AREA-RINGS-01 measurements (gate constraints C1, C4 and C5) on the Python lit render of the kit.

Uses the kit's own method (``scripts/env_kit_light.py``: the A1 lighting model on the exported atlases, actor
frame 0 on the walking line of its column, ``lookdev_compose.readability``: mean CIE L* of the actor pixels vs a
6 px ring around the actor box). The composed layouts of both rooms, plain and dressed, come from the ignored
test ``ring_kit::tests::dump_ring_layouts``:

    RINGS_LAYOUT_DIR=DIR cargo test --lib -- --ignored dump_ring_layouts
    python3 scripts/measure_rings01.py DIR [OUT.txt]

Prints (and writes) one line per measurement. The plain layout is the greybox room without the dressing: the
baseline of "no worse than on plain brick in the same light".
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import env_kit as ek  # noqa: E402
import env_kit_light as lit  # noqa: E402
from lookdev_compose import srgb_to_lab_l  # noqa: E402

CARL = "assets/manifests/carl_book1_h52.json"
GOBLIN = "assets/manifests/goblin_a_h36.json"
W = 580
# Walkable tops (room, name, x, y, w): the C1 platforms.
PLATFORMS = {
    "ring_court": [("step", 1056, 516, 64), ("walkway", 1120, 500, 192), ("shelf", 1536, 452, 96)],
    "ring_split": [("route", 336, 516, 384), ("r3_ledge", 896, 516, 96)],
}


def view_around(x: int, h: int = 360, width: int = 640, room_w: int = 2048) -> list[int]:
    left = max(0, min(room_w - width, x - width // 2))
    return [left, W - (288 if h == 360 else 308), width, h]


def load_layout(dir_: Path, room: str, kind: str) -> dict:
    return json.loads((dir_ / f"{room}.{kind}.layout.json").read_text(encoding="utf-8"))


def render(kit, layout, view, entries):
    actors, boxes = ek.actor_layer(kit, layout, entries)
    img = lit.render_lit(kit, layout, view, 1, actors, boxes=boxes)
    return img, lit.actor_mask(actors, view), boxes


def actor_lines(dir_: Path, kit, room: str, who: str, manifest: str, x: int) -> list[str]:
    out = []
    room_w = 2048 if room == "ring_court" else 1152
    for kind in ("plain", "dressed"):
        layout = load_layout(dir_, room, kind)
        view = view_around(x, room_w=room_w)
        img, mask, boxes = render(kit, layout, view, [{"manifest": manifest, "x": x}])
        m = lit.measure(img, mask, boxes, view)[0]
        out.append(f"ACTOR_SEPARATION room={room} actor={who} x={x} layout={kind} actor_L={m['actor_mean_L']} "
                   f"ring_L={m['surround_mean_L']} dL={m['delta_L']} contrast={m['luminance_contrast_ratio']} "
                   f"edge_dL={m['silhouette_edge_mean_abs_dL']}")
    return out


def run_means(img, y: int, x0: int, x1: int, run: int = 13) -> float:
    """The brightest mean L* of a ``run`` px horizontal run of row ``y`` between x0 and x1 (view px)."""
    px = img.convert("RGB").load()
    best = 0.0
    xs = list(range(max(0, x0), min(img.width, x1)))
    ls = [srgb_to_lab_l(px[x, y])[0] for x in xs]
    for i in range(0, max(1, len(ls) - run + 1)):
        window = ls[i:i + run]
        best = max(best, sum(window) / len(window))
    return best


def walkable_lines(dir_: Path, kit, room: str) -> list[str]:
    """C1: the platform's top arris against the brightest 13 px horizontal run within +-64 px of it (the
    platform's own rows 0..3 and the deck below the walking line excepted; torch flames are not in the layers
    of the render that has no actors)."""
    out = []
    room_w = 2048 if room == "ring_court" else 1152
    for kind in ("plain", "dressed"):
        layout = load_layout(dir_, room, kind)
        for name, px_, py, pw in PLATFORMS[room]:
            view = view_around(px_ + pw // 2, room_w=room_w)
            img = lit.render_lit(kit, layout, view, 1)
            vx, vy = view[0], view[1]
            x0, x1 = max(px_, vx) - vx, min(px_ + pw, vx + view[2]) - vx
            top_run = max(run_means(img, py + dy - vy, x0, x1) for dy in (0, 1))
            rival, rival_y = 0.0, 0
            for y in range(py - 64, min(py + 64, W - 24)):
                if py - 1 <= y <= py + 3 or not (0 <= y - vy < view[3]):
                    continue
                v = run_means(img, y - vy, x0, x1)
                if v > rival:
                    rival, rival_y = v, y
            out.append(f"WALKABLE_TOP room={room} platform={name} layout={kind} top_arris_run_L={top_run:.1f} "
                       f"brightest_rival_run_L={rival:.1f} at_y={rival_y} brightest={'top' if top_run >= rival else 'rival'}")
    return out


def main(argv: list[str]) -> int:
    dir_ = Path(argv[0])
    kit = ek.load_kit("assets/source/terrain/floor01/floor01_rings_kit.json")
    lines = []
    lines += actor_lines(dir_, kit, "ring_court", "carl", CARL, 1216)
    lines += actor_lines(dir_, kit, "ring_court", "goblin", GOBLIN, 1824)
    lines += actor_lines(dir_, kit, "ring_court", "goblin", GOBLIN, 1424)
    lines += actor_lines(dir_, kit, "ring_split", "goblin", GOBLIN, 560)
    lines += actor_lines(dir_, kit, "ring_split", "goblin", GOBLIN, 848)
    for room in PLATFORMS:
        lines += walkable_lines(dir_, kit, room)
    text = "\n".join(lines) + "\n"
    print(text, end="")
    if len(argv) > 1:
        Path(argv[1]).write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
