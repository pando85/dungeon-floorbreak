#!/usr/bin/env python3
"""GOB-01 authoring record: goblin palette, size-study block-ins and layer split.

Usage:

    python3 scripts/author_goblin_gob01.py size-study [--check]

``size-study`` rebuilds ``assets/source/characters/goblin/goblin_size_study.grid.json``:
three flat-lit **block-in** idle silhouettes at 36, 40 and 44 px opaque height, rasterized
natively at each height from one parametric pose (polygons and capsules at pixel centres,
1 px plum outline outside the fill, simple upper-left rim light, a fixed face stamp). They
exist only so the three heights can be compared with one method; they are not production
pixels. ``--check`` compares instead of writing (exit 1 on a difference).

The variant A key poses (``goblin_a_h36_keys.grid.json``) are **not** written by this script.
They are hand-authored in the grid, which is their source of truth; this module only
provides the shared palette and the body/weapon layer split used when they were saved.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOBLIN_DIR = ROOT / "assets/source/characters/goblin"
SIZE_STUDY = GOBLIN_DIR / "goblin_size_study.grid.json"
CANVAS = (64, 56)
PIVOT = (28, 55)
HEIGHTS = (36, 40, 44)

# goblin-floor01-v0: shared outline with Carl; green skin ramp cooled toward blue in shadow
# (ADR-0007 cold ambient) and warmed in the light (torch side); cool, dark leather rags so
# the goblin separates from the warm orange brick; pineapple gold + teal-green crown.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "rag_dark", "#2c1e28ff", False),
    ("t", "rag_mid", "#4c3436ff", False),
    ("u", "rag_light", "#73524aff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
    ("R", "red_mid", "#b8322eff", False),  # tongue; variant B bandana
    ("x", "wood_dark", "#4a2c1cff", False),
    ("y", "wood_mid", "#8c5c32ff", False),
    ("F", "pineapple_dark", "#7a3c12ff", False),
    ("G", "pineapple_mid", "#d08826ff", False),
    ("H", "pineapple_light", "#f4c84aff", True),  # accent: single-pixel facet highlights
    ("V", "leaf_dark", "#1d4a3cff", False),
    ("v", "leaf_mid", "#3e8a5cff", False),
    ("B", "rag_red_dark", "#6e1a22ff", False),  # variant B bandana shadow (R is its mid tone)
]
WEAPON_KEYS = set("xyFGHVv")
RAMPS = {
    "skin": ("a", "s", "m", "l"),
    "skin_far": ("K", "a", "s", "m"),
    "rag": ("K", "r", "t", "u"),
    "wood": ("K", "x", "y", "y"),
    "pine": ("F", "F", "G", "H"),
    "leaf": ("K", "V", "v", "v"),
}


def palette_json(keys: set[str] | None = None) -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        if keys is not None and key not in keys:
            continue
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def split_layers(rows: list[str]) -> dict[str, list[str]]:
    """Split one flat frame into disjoint ``body`` and ``weapon`` cels.

    Weapon keys go to ``weapon``; an outline pixel goes to ``weapon`` when it touches a weapon
    key (8-neighbourhood) and no body colour (4-neighbourhood). Everything else is ``body``."""
    h, w = len(rows), len(rows[0])

    def at(x: int, y: int) -> str:
        return rows[y][x] if 0 <= x < w and 0 <= y < h else "."

    body = [["."] * w for _ in range(h)]
    weapon = [["."] * w for _ in range(h)]
    for y in range(h):
        for x in range(w):
            ch = rows[y][x]
            if ch == ".":
                continue
            to_weapon = ch in WEAPON_KEYS
            if ch == "K":
                n8 = [at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
                n4 = [at(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
                to_weapon = any(c in WEAPON_KEYS for c in n8) and not any(
                    c not in WEAPON_KEYS and c not in ".K" for c in n4)
            (weapon if to_weapon else body)[y][x] = ch
    return {"body": ["".join(r) for r in body], "weapon": ["".join(r) for r in weapon]}


# ------------------------------------------------------------------ block-in rasterizer

class _Canvas:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.part: list[list[int | None]] = [[None] * w for _ in range(h)]
        self.ramps: list[tuple[str, str, str, str]] = []

    def add(self, ramp: tuple[str, str, str, str]) -> int:
        self.ramps.append(ramp)
        return len(self.ramps) - 1

    def _fill(self, test, x0, x1, y0, y1, pid):
        for y in range(max(0, int(y0) - 1), min(self.h, int(y1) + 2)):
            for x in range(max(0, int(x0) - 1), min(self.w, int(x1) + 2)):
                if test(x + 0.5, y + 0.5):
                    self.part[y][x] = pid

    def poly(self, pts, pid):
        def inside(px, py):
            c = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > py) != (yb > py) and px < xa + (py - ya) * (xb - xa) / (yb - ya):
                    c = not c
            return c
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        self._fill(inside, min(xs), max(xs), min(ys), max(ys), pid)

    def capsule(self, p0, p1, r, pid):
        (xa, ya), (xb, yb) = p0, p1

        def near(px, py):
            dx, dy = xb - xa, yb - ya
            ln = dx * dx + dy * dy
            t = 0.0 if ln == 0 else max(0.0, min(1.0, ((px - xa) * dx + (py - ya) * dy) / ln))
            return math.hypot(px - (xa + t * dx), py - (ya + t * dy)) <= r
        self._fill(near, min(xa, xb) - r, max(xa, xb) + r, min(ya, yb) - r, max(ya, yb) + r, pid)

    def ellipse(self, cx, cy, rx, ry, pid):
        self._fill(lambda px, py: ((px - cx) / rx) ** 2 + ((py - cy) / ry) ** 2 <= 1.0,
                   cx - rx, cx + rx, cy - ry, cy + ry, pid)

    def render(self) -> list[list[str]]:
        out = [["."] * self.w for _ in range(self.h)]

        def get(x, y):
            return self.part[y][x] if 0 <= x < self.w and 0 <= y < self.h else None
        for y in range(self.h):
            for x in range(self.w):
                pid = self.part[y][x]
                if pid is None:
                    if any(get(x + dx, y + dy) is not None for dx, dy in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                        out[y][x] = "K"
                    continue
                n4 = [get(x, y - 1), get(x - 1, y), get(x + 1, y), get(x, y + 1)]
                if any(n is not None and n > pid for n in n4):  # a part in front: separation line
                    out[y][x] = "K"
                    continue
                _deep, shade, mid, light = self.ramps[pid]
                up, left, right, down = n4
                if up != pid or left != pid:
                    out[y][x] = light
                elif right != pid or down != pid or get(x + 1, y + 1) != pid:
                    out[y][x] = shade
                else:
                    out[y][x] = mid
        return out


def blockin_idle(height: int) -> list[str]:
    """Native block-in idle at ``height`` px opaque (fill height + 2 px outline)."""
    w, h = CANVAS
    s = (height - 2) / 34.0
    c = _Canvas(w, h)
    bottom = h - 2

    def P(x, y):  # design units: x right, y up; y = 0 is the lowest fill row
        return (PIVOT[0] + 0.5 + x * s, bottom + 1.0 - y * s)
    far = c.add(RAMPS["skin_far"])
    torso = c.add(RAMPS["skin"])
    rag = c.add(RAMPS["rag"])
    head = c.add(RAMPS["skin"])
    near = c.add(RAMPS["skin"])
    wood = c.add(RAMPS["wood"])
    pine = c.add(RAMPS["pine"])
    leaf = c.add(RAMPS["leaf"])
    c.capsule(P(-3, 10), P(-5.5, 5.5), 1.3 * s, far)
    c.capsule(P(-5.5, 5.5), P(-5, 2), 1.1 * s, far)
    c.poly([P(-9, 0), P(-8.5, 2.2), P(-3.5, 2.4), P(-2, 0)], far)
    c.capsule(P(-3, 22), P(-6, 15), 1.3 * s, far)
    c.capsule(P(-6, 15), P(-5, 10), 1.1 * s, far)
    c.ellipse(*P(-5, 9), 1.7 * s, 1.6 * s, far)
    c.poly([P(-6, 11), P(-6.5, 18), P(-4, 23), P(0, 25), P(5, 24), P(7, 19), P(7, 14), P(5, 11)], torso)
    c.poly([P(-6.6, 13), P(-6.8, 18.5), P(-4.3, 23.3), P(0, 25.3), P(2.5, 24), P(1.5, 19), P(3, 14),
            P(1.5, 12.5)], rag)
    c.poly([P(-6.8, 14), P(6.2, 14), P(6.6, 10), P(5, 8), P(3.5, 9.5), P(1.5, 7.5), P(-0.5, 9.5),
            P(-3, 7.5), P(-5, 9.5), P(-7, 8.5)], rag)
    c.ellipse(*P(5.5, 28.5), 6.3 * s, 5.4 * s, head)
    c.poly([P(2.5, 26), P(11.5, 26.5), P(12, 24), P(9, 21.7), P(3.5, 22.5)], head)
    c.poly([P(0, 30.5), P(-9.5, 34), P(-8.5, 32), P(-0.5, 26.5)], head)
    c.capsule(P(2, 9), P(4.5, 5.5), 1.4 * s, near)
    c.capsule(P(4.5, 5.5), P(4, 2), 1.2 * s, near)
    c.poly([P(1.5, 0), P(1.8, 2), P(3.5, 2.8), P(7, 2.4), P(10.5, 0.8), P(10.5, 0)], near)
    c.capsule(P(9, 16.5), P(17.5, 19.5), 0.75 * s, wood)
    c.ellipse(*P(21.5, 23), 2.9 * s, 3.4 * s, pine)
    c.poly([P(19.5, 26), P(19, 31.5), P(21, 28), P(22.5, 32), P(23, 28), P(25, 30.5), P(23.5, 25.5)], leaf)
    c.poly([P(9.5, 30), P(16.5, 27.2), P(16, 26), P(10, 26.5)], head)
    c.capsule(P(2, 22), P(3.5, 16.5), 1.4 * s, near)
    c.capsule(P(3.5, 16.5), P(9, 17), 1.2 * s, near)
    c.ellipse(*P(10, 17), 1.8 * s, 1.7 * s, near)
    rows = c.render()

    def put(x, y, k):
        px, py = P(x, y)
        rows[int(py)][int(px)] = k
    for dx in (0, 1, 2):  # brow
        put(5.2 + dx * s, 30.6 + dx * 0.4, "K")
    put(5.2, 29.4, "e"); put(5.2 + s, 29.4, "e"); put(5.2 + 2 * s, 29.4, "K")
    put(9.2, 29.4, "e"); put(9.2 + s, 29.4, "K")
    for i in range(6):  # mouth: lip line, upper teeth, dark mouth
        put(5 + i * s, 26.6, "K")
        put(5 + i * s, 25.6, "w" if i % 3 != 2 else "o")
        put(5 + i * s, 24.6, "o" if i < 5 else "w")
    return ["".join(r) for r in rows]


def size_study_grid() -> dict:
    frames, used = [], set()
    for height in HEIGHTS:
        rows = blockin_idle(height)
        used |= {ch for r in rows for ch in r if ch != "."}
        frames.append({"duration_ms": 500, "cels": split_layers(rows)})
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body", "weapon"],
        "frames": frames,
        "tags": [{"name": f"h{h}", "from": i, "to": i, "direction": "forward"}
                 for i, h in enumerate(HEIGHTS)],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["size-study"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    text = dump(size_study_grid())
    if args.check:
        same = SIZE_STUDY.is_file() and SIZE_STUDY.read_text(encoding="utf-8") == text
        print("size study grid up to date" if same else "size study grid differs from the generator")
        return 0 if same else 1
    SIZE_STUDY.parent.mkdir(parents=True, exist_ok=True)
    SIZE_STUDY.write_text(text, encoding="utf-8")
    print(f"wrote {SIZE_STUDY.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
