#!/usr/bin/env python3
"""RAT-01 authoring record: Dungeon Rat palette, size block-ins and the six draft key poses.

Usage:

    python3 scripts/author_rat01.py frames [--check]
    python3 scripts/author_rat01.py size-study [--check]

``keys`` rebuilds ``assets/source/characters/rat/rat_h18.grid.json`` (pixel-grid-v1, layer ``body``):
one key per readability tag (idle, move contact, tell, attack, hurt, defeat). Every pose is a *rule*:
thick polylines (torso, neck, legs, tail) and polygons (ears) rasterized at pixel centres in design units
(x right from the pivot column, y up from the contact row), a 1 px outline, a simple upper-left rim light,
deterministic fur streaks/tufts and literal face stamps. No raster image-model output, no tracing of
concept pixels. ``size-study`` rebuilds the three flat-lit idle block-ins (14, 18, 22 px) used for the
size decision. ``--check`` compares instead of writing (exit 1 on a difference).

This is the *draft* (medium tier) for a top-tier edit; the keys are not frozen.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets/source/characters/rat"
KEYS = OUT_DIR / "rat_h18.grid.json"
MANIFEST = ROOT / "assets/manifests/rat-01.json"
STUDY = OUT_DIR / "rat_size_study.grid.json"
CANVAS = (64, 28)
PIVOT = (30, 27)

# rat-floor01-v1 (top-tier edit): split-temperature fur. The shadow end stays cool blue-gray (separation from
# the orange brick and floor), the lit end is the concept's warm gray-brown (canon), so the rat reads as a
# gray-brown animal under torch light and drops into cold shade on the ambient side. Saturated pink bald
# patches / tail / ears, ember-red eyes, tobacco-yellow teeth.
PALETTE = [
    ("K", "outline_plum", "#1a1218ff", False),
    ("a", "fur_deep", "#2b2830ff", False),
    ("s", "fur_shade", "#4a4550ff", False),
    ("m", "fur_mid", "#716760ff", False),
    ("l", "fur_light", "#a39581ff", False),
    ("p", "pink_deep", "#74303fff", False),
    ("q", "pink_mid", "#b8576aff", False),
    ("r", "pink_light", "#e39a96ff", False),
    ("e", "eye_ember", "#ff5a24ff", True),
    ("h", "eye_hot", "#ffd84aff", True),
    ("t", "tooth_yellow", "#e2b83eff", True),
    ("T", "tooth_shade", "#9a7424ff", False),
    ("o", "mouth_dark", "#4a1020ff", False),
    ("c", "claw_bone", "#cbb890ff", False),
    ("w", "eye_white", "#efe4c4ff", True),
]
RAMPS = {
    "fur": ("a", "s", "m", "l"),
    "far": ("K", "a", "a", "s"),
    "pink": ("p", "p", "q", "r"),
    "tail": ("p", "p", "q", "r"),
    "claw": ("K", "T", "c", "c"),
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


class Canvas:
    """Back-to-front part canvas (same rules as the GOB-01 block-in: later parts occlude earlier ones)."""

    def __init__(self, w: int, h: int, floor_row: int):
        self.w, self.h = w, h
        self.px, self.bottom = PIVOT[0], floor_row - 1          # fill ends one row above the outline row
        self.part: list[list[int | None]] = [[None] * w for _ in range(h)]
        self.ramps: list[tuple] = []
        self.names: list[str] = []

    def P(self, x: float, y: float) -> tuple[float, float]:
        return (self.px + 0.5 + x, self.bottom + 1.0 - y)

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        self.names.append(name)
        return len(self.ramps) - 1

    def _fill(self, test, x0, x1, y0, y1, pid):
        for y in range(max(0, int(y0) - 1), min(self.h, int(y1) + 2)):
            for x in range(max(0, int(x0) - 1), min(self.w, int(x1) + 2)):
                if test(x + 0.5, y + 0.5):
                    self.part[y][x] = pid

    def poly(self, pts, pid):
        pts = [self.P(*p) for p in pts]

        def inside(px, py):
            c = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > py) != (yb > py) and px < xa + (py - ya) * (xb - xa) / (yb - ya):
                    c = not c
            return c
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        self._fill(inside, min(xs), max(xs), min(ys), max(ys), pid)

    def disc(self, cx, cy, rx, ry, pid):
        cx, cy = self.P(cx, cy)
        self._fill(lambda px, py: ((px - cx) / rx) ** 2 + ((py - cy) / ry) ** 2 <= 1.0,
                   cx - rx, cx + rx, cy - ry, cy + ry, pid)

    def thick(self, pts, radii, pid):
        """Polyline through ``pts`` (design units) with a radius per point, as overlapping discs."""
        for i in range(len(pts) - 1):
            (xa, ya), (xb, yb) = pts[i], pts[i + 1]
            n = max(2, int(math.hypot(xb - xa, yb - ya) * 2))
            for k in range(n + 1):
                t = k / n
                r = radii[i] + (radii[i + 1] - radii[i]) * t
                self.disc(xa + (xb - xa) * t, ya + (yb - ya) * t, r, r, pid)

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
                if any(n is not None and n > pid for n in n4):
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


# ------------------------------------------------------------------ pose description

def build(pose: dict) -> list[str]:
    """Rasterize one pose dict (see POSES) into 28 row strings."""
    w, h = CANVAS
    c = Canvas(w, h, PIVOT[1])
    far = c.add("far"); tail = c.add("tail"); fur = c.add("fur"); pink = c.add("pink"); claw = c.add("claw")
    # back to front: far legs, tail, far ear, torso, belly pink, head, near legs, near ear
    for leg in pose.get("far_legs", []):
        c.thick(leg["pts"], leg["r"], far)
    c.thick(pose["tail"]["pts"], pose["tail"]["r"], tail)
    for ear in pose.get("far_ears", []):
        c.poly(ear, far)
    c.thick(pose["spine"], pose["spine_r"], fur)
    if pose.get("belly"):
        c.thick(*pose["belly"], pink)
    c.thick(pose["head"]["pts"], pose["head"]["r"], fur)
    for nose in pose.get("noses", []):
        c.disc(*nose, pink)
    for leg in pose.get("legs", []):
        c.thick(leg["pts"], leg["r"], fur)
    for cl in pose.get("claws", []):
        c.poly(cl, claw)
    for ear in pose.get("ears", []):
        c.poly(ear, pink)
    rows = c.render()
    # mangy patches: remap fur ramp to pink ramp inside ellipses (design units)
    for (cx, cy, rx, ry) in pose.get("patches", []):
        px, py = c.P(cx, cy)
        for y in range(h):
            for x in range(w):
                if ((x + 0.5 - px) / rx) ** 2 + ((y + 0.5 - py) / ry) ** 2 <= 1.0:
                    ch = rows[y][x]
                    if ch in "asml":
                        rows[y][x] = {"a": "p", "s": "p", "m": "q", "l": "r"}[ch]
    # fur streaks: deterministic shade strokes on mid fur
    for y in range(h):
        for x in range(w):
            if rows[y][x] == "m" and ((x // 2) * 3 + y * 5 + pose.get("seed", 0)) % 8 == 0:
                rows[y][x] = "s"
            elif rows[y][x] == "m" and ((x // 2) * 5 + y * 3 + pose.get("seed", 0)) % 13 == 0:
                rows[y][x] = "l"
    # ribs: short shade strokes (x, y_top, y_bottom in design units) on lit/mid fur only, the wiry flank
    for rx, ry0, ry1 in pose.get("ribs", []):
        col = int(c.P(rx, 0)[0])
        for y in range(int(c.P(0, ry0)[1]), int(c.P(0, ry1)[1]) + 1):
            if rows[y][col] in "ml":
                rows[y][col] = "s"
    # tufts: spiky fur on the silhouette at the given columns (x in design units, height in px)
    for tx, th in pose.get("tufts", []):
        col = int(c.P(tx, 0)[0])
        top = next((y for y in range(h) if rows[y][col] != "."), None)
        if top is None:
            continue
        for k in range(1, th + 1):
            if top - k - 1 >= 0:
                rows[top - k][col] = "s" if k < th else "m"
        if top - th - 1 >= 0:
            rows[top - th - 1][col] = "K"
    # literal stamps: (x, y, key) in design units, applied last
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        rows[int(py)][int(px)] = key
    despeckle(rows)
    return ["".join(r) for r in rows]


ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


def despeckle(rows: list[list[str]]) -> None:
    """Recolour orphan fill pixels (no same-key 8-neighbour) to the most common opaque neighbour key.

    Outline (K) and accent keys are left alone, so the silhouette and the eyes/teeth never move."""
    h, w = len(rows), len(rows[0])
    for _ in range(2):
        for y in range(h):
            for x in range(w):
                ch = rows[y][x]
                if ch in ".K" or ch in ACCENTS:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if ch in around:
                    continue
                cand = [a for a in around if a not in ".K"]
                if cand:
                    rows[y][x] = max(sorted(set(cand)), key=cand.count)


def ear(x, y, w, h_, lean=0.0, notch=False):
    """Triangle ear with a base centred at (x, y); ``notch`` bites a torn V out of the back edge."""
    left, tip, right = (x - w / 2, y), (x + lean, y + h_), (x + w / 2, y)
    if not notch:
        return [left, tip, right]
    a = (tip[0] + 0.35 * (right[0] - tip[0]), tip[1] + 0.35 * (right[1] - tip[1]))
    b = (tip[0] + 0.75 * (right[0] - tip[0]), tip[1] + 0.75 * (right[1] - tip[1]))
    n = (x + lean * 0.4, y + h_ * 0.45)          # the tear reaches almost to the ear's centre
    return [left, tip, a, n, b, right]


def far_ear(x, y, w, h_, lean=0.0):
    return ear(x, y, w, h_, lean)


# Each pose: design units (x right of the pivot column, y up from the contact row).
POSES: dict[str, dict] = {}

POSES["idle0"] = {
    "seed": 1,
    # editor: a true bridge arch (apex over the ribs, daylight under the belly), thin hips and neck so the
    # torso reads wiry, a blunt wedge head, a torn near ear and a mangy bald saddle on the hump.
    "spine": [(-13, 4.5), (-10, 7.8), (-6, 11), (-1, 12), (4, 10.4), (8.5, 7.6), (11.5, 6)],
    "spine_r": [2.0, 2.8, 3.1, 3.2, 2.9, 2.3, 2.1],
    "head": {"pts": [(12, 6.2), (16.5, 4.6), (20.5, 3.2)], "r": [3.1, 2.4, 1.2]},
    "noses": [(21.6, 3.2, 1.0, 1.0)],
    "tail": {"pts": [(-14, 3.6), (-18, 2), (-22, 2), (-25, 4), (-27.5, 5.5)], "r": [1.1, 0.9, 0.8, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-7, 6), (-6, 2.5), (-4.5, 0.5)], "r": [1.6, 0.9, 0.8]},
        {"pts": [(6, 6), (7.5, 2.5), (9.5, 0.5)], "r": [1.3, 0.8, 0.8]},
    ],
    "legs": [
        {"pts": [(-10, 6.5), (-11, 3), (-8, 0.5)], "r": [2.9, 1.3, 1.0]},
        {"pts": [(9, 5.5), (10, 2.4), (12.5, 0.5)], "r": [1.7, 0.9, 0.9]},
    ],
    "far_ears": [far_ear(11, 8.2, 3, 3.2, 0.5)],
    "ears": [ear(14, 7.6, 5.0, 5.0, -1.0, notch=True)],
    "patches": [(-4, 13.2, 3.2, 1.6), (3, 6.6, 2.0, 1.5)],
    "ribs": [(-2, 9.5, 7.5), (0, 9.5, 7.5), (2, 9, 7.5)],
    "tufts": [(-11, 1), (-8, 2), (-6, 1), (5, 1), (7, 2)],
    "stamps": [(17, 5.6, "e"), (18, 5.6, "e"), (17, 6.6, "K"), (18, 6.6, "K"),
               (19.5, 1.5, "t"), (20.5, 1.5, "t"), (19.5, 0.5, "t"), (20.5, 0.5, "T"),
               (13, 13.5, "."), (13, 12.5, "K"), (14, 10.5, "K"), (15, 10.5, ".")],
}

POSES["run1"] = {
    "seed": 2,
    "spine": [(-14, 6.5), (-8, 8.2), (-1, 8.8), (6, 8.2), (12, 7.2)],
    "spine_r": [2.2, 3.0, 3.2, 3.0, 2.4],
    "head": {"pts": [(13, 7), (18, 6.2), (23.5, 5)], "r": [3.2, 2.5, 1.2]},
    "noses": [(24.6, 5.0, 1.0, 1.0)],
    "tail": {"pts": [(-15, 6.5), (-20, 7.5), (-24, 8.5), (-28, 8.2)], "r": [1.1, 0.9, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-8, 5), (-5, 2.5), (-3, 0.6)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(9, 5), (6, 3), (6, 0.6)], "r": [1.8, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-12, 5.5), (-16, 2.5), (-19, 0.6)], "r": [2.6, 1.2, 1.0]},
        {"pts": [(10, 5.5), (14, 2.8), (18, 0.6)], "r": [1.9, 1.0, 0.9]},
    ],
    "far_ears": [],
    "ears": [ear(13, 8.0, 5.0, 4.4, -2.6, notch=True)],
    "patches": [(-6, 7.4, 3.2, 1.8)],
    "tufts": [(-6, 1), (1, 1)],
    "stamps": [(11, 12.5, "."), (11, 11.5, "K"), (19, 7, "e"), (20, 7, "e"), (21.5, 3.1, "t"), (21.5, 2.4, "t"), (23.5, 3.1, "t")],
}

POSES["tell2"] = {
    "seed": 3,
    # editor: a coiled wedge (rump high, chest and chin on the floor, no daylight under the belly), hackles
    # bristling on the rump, jaws gaping with a dark maw and four yellow teeth, tail hooked high. Idle is a
    # symmetric bridge; the tell is a forward-sloping wedge with an open mouth.
    "spine": [(-12, 6), (-9, 9.4), (-5, 10.4), (0, 8.6), (5, 6), (9, 4.6)],
    "spine_r": [2.6, 3.0, 3.2, 3.0, 2.7, 2.4],
    "head": {"pts": [(10, 5.2), (14.5, 5.2), (19.5, 4.6)], "r": [3.0, 2.2, 1.1]},
    "noses": [(20.6, 4.8, 1.0, 1.0)],
    "tail": {"pts": [(-13, 6), (-17, 8), (-19, 11), (-18, 14), (-15, 15.5)], "r": [1.1, 0.9, 0.8, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(6, 4), (9, 2), (12, 0.6)], "r": [1.6, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-9, 6), (-6, 2), (-3.5, 0.6)], "r": [3.2, 1.6, 1.0]},
        {"pts": [(-11, 4), (-11, 1.5), (-9, 0.6)], "r": [2.4, 1.2, 0.9]},
        {"pts": [(7, 3.5), (11, 1.6), (15, 0.6)], "r": [1.9, 1.0, 0.9]},
        {"pts": [(13.5, 2.4), (16.5, 1.6), (19, 1.2)], "r": [1.3, 1.0, 0.8]},
    ],
    "far_ears": [],
    "ears": [ear(10.5, 7.0, 5.0, 4.4, -2.8, notch=True)],
    "patches": [(-7, 9.6, 2.8, 1.6), (3, 5.2, 2.0, 1.4)],
    "ribs": [(-1, 7.5, 5.5), (1, 7, 5)],
    "tufts": [(-11, 1), (-9, 2), (-7, 3), (-5, 2), (-3, 1)],
    "stamps": [(8, 11.5, "."), (8, 10.5, "K"), (15, 6.2, "e"), (16, 6.2, "e"), (15, 7.2, "K"), (16, 7.2, "K"),
               (16.5, 3.4, "o"), (17.5, 3.4, "o"), (18.5, 3.4, "o"), (17.5, 2.6, "o"), (18.5, 2.6, "o"), (19.5, 2.6, "o"),
               (19.5, 3.6, "t"), (20.5, 3.6, "t"), (19.5, 1.8, "t"), (20.5, 1.8, "t")],
}

POSES["bite3"] = {
    "seed": 4,
    "spine": [(-13, 3.5), (-7, 5.5), (0, 8), (7, 10.5), (13, 12.5)],
    "spine_r": [2.4, 3.0, 3.2, 3.0, 2.6],
    "head": {"pts": [(14, 13), (19, 13.4), (23, 12.4)], "r": [3.3, 2.6, 1.3]},
    "noses": [(24.2, 12.6, 1.0, 1.0)],
    "tail": {"pts": [(-14, 3.5), (-19, 3.5), (-24, 4), (-28, 5)], "r": [1.1, 0.9, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-8, 4), (-12, 3), (-15, 2)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(10, 10), (15, 12), (19, 11)], "r": [1.6, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-11, 4), (-14, 2), (-17, 0.6)], "r": [2.6, 1.2, 1.0]},
        {"pts": [(12, 11), (17, 10), (21, 9)], "r": [1.9, 1.0, 0.9]},
        {"pts": [(15, 10), (19, 7.6), (22.5, 6.4)], "r": [1.6, 1.3, 0.9]},
    ],
    "far_ears": [],
    "ears": [ear(13.5, 14.6, 5.0, 4.4, -2.8, notch=True)],
    "patches": [(-4, 6, 3.4, 2.0)],
    "tufts": [(-6, 1), (2, 1)],
    "stamps": [(11, 19.5, "."), (11, 18.5, "K"), (19, 13.8, "e"), (20, 13.8, "e"), (19, 14.8, "K"),
               (18.5, 10.5, "o"), (19.5, 10.5, "o"), (20.5, 10.5, "o"), (21.5, 10.5, "o"), (22.5, 10.5, "o"),
               (19.5, 9.5, "o"), (20.5, 9.5, "o"), (21.5, 9.5, "o"),
               (20, 11.4, "t"), (22, 11.4, "t"), (23, 11.4, "t"), (21, 8.6, "t"), (22.5, 8.6, "t")],
}

POSES["hurt4"] = {
    "seed": 5,
    # editor: knocked back onto the haunches. The torso stands almost upright (about 70 degrees, the bite
    # is about 20), the head snaps back with the nose to the ceiling, the pink underbelly is flashed at the
    # camera, the front paws splay, the tail kinks up. Compact (about 24 px wide without the tail) where the
    # bite is long, and the eyes are squeezed shut (no ember).
    "spine": [(-9, 3.2), (-8.5, 7), (-7.6, 10.5), (-6.4, 13)],
    "spine_r": [3.4, 3.3, 3.0, 2.6],
    "belly": ([(-6.2, 4), (-5.4, 7.5), (-4.6, 10.8)], [1.3, 1.6, 1.2]),
    "head": {"pts": [(-6.2, 13.5), (-5, 16), (-3.8, 18.2)], "r": [2.9, 2.2, 1.1]},
    "noses": [(-3.4, 19.2, 1.0, 1.0)],
    "tail": {"pts": [(-12, 2), (-16, 3), (-18, 6.5), (-16.5, 9), (-19, 11)], "r": [1.1, 0.9, 0.8, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-6, 4), (-3, 1.4), (0, 0.6)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(-5, 12), (-1.5, 14.5), (1, 16.5)], "r": [1.2, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-8.5, 4), (-5.5, 1.4), (-2.5, 0.6)], "r": [2.8, 1.2, 1.0]},
        {"pts": [(-4.5, 10), (-1, 10.5), (2.5, 9)], "r": [1.4, 0.9, 0.8]},
    ],
    "far_ears": [],
    "ears": [ear(-8.2, 15.2, 3.4, 3.2, -1.6, notch=True)],
    "patches": [(-8, 9, 2.0, 1.6)],
    "ribs": [],
    "tufts": [(-11, 1)],
    "stamps": [(-5.5, 16.6, "K"), (-4.5, 17.2, "K"), (-2.5, 17.2, "o"), (-2.5, 16.4, "o"), (-1.5, 17.6, "t"), (-2.5, 18.2, "t")],
}

POSES["down5"] = {
    "seed": 6,
    "spine": [(-12, 3.6), (-5, 3.6), (2, 3.6), (8, 3.2)],
    "spine_r": [2.4, 3.2, 3.2, 2.6],
    "belly": ([(-8, 5.2), (-1, 5.6), (5, 5.0)], [1.4, 1.8, 1.4]),
    "head": {"pts": [(9, 3.0), (13, 2.6), (17, 1.8)], "r": [2.4, 2.0, 1.0]},
    "noses": [(18, 1.8, 1.0, 1.0)],
    "tail": {"pts": [(-13, 2.5), (-18, 1.4), (-22, 1.8), (-26, 1.0)], "r": [1.1, 0.9, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-4, 6.5), (-4, 9.5), (-2, 11)], "r": [1.0, 0.8, 0.7]},
        {"pts": [(7, 5.5), (9, 8), (11, 9)], "r": [1.0, 0.8, 0.7]},
    ],
    "legs": [
        {"pts": [(-8, 6.5), (-9, 9.5), (-7, 11)], "r": [1.2, 0.9, 0.8]},
        {"pts": [(4, 6), (5, 9), (8, 10.5)], "r": [1.2, 0.9, 0.8]},
    ],
    "far_ears": [],
    "ears": [ear(9, 4.4, 2.6, 2.4, -1.2)],
    "patches": [],
    "tufts": [],
    "stamps": [(11, 4, "K"), (13, 4, "K"), (12, 3, "K"), (11, 2, "K"), (13, 2, "K"),
               (18, 0.8, "q"), (19, 0.8, "q"), (17, 1.8, "o"), (16, 0.8, "t")],
}


# ------------------------------------------------------------------ in-betweens (executor, medium tier)
# Each in-between is a copy of a frozen key pose with named points moved (spec section 8). The key POSES above are
# never mutated: every helper works on a deep copy.

def derive(key: str, **over) -> dict:
    pose = copy.deepcopy(POSES[key])
    pose.update(over)
    return pose


def mv(pts, dx=0.0, dy=0.0):
    return [(x + dx, y + dy) for x, y in pts]


def warp(pose: dict, fn) -> dict:
    """Apply ``fn(x, y) -> (x, y)`` to every geometric point of a pose (stamps and tufts excluded)."""
    p = copy.deepcopy(pose)

    def ws(pts):
        return [fn(x, y) for x, y in pts]
    p["spine"] = ws(p["spine"])
    p["head"]["pts"] = ws(p["head"]["pts"])
    p["tail"]["pts"] = ws(p["tail"]["pts"])
    if p.get("belly"):
        p["belly"] = (ws(p["belly"][0]), p["belly"][1])
    p["noses"] = [(*fn(nx, ny), r1, r2) for nx, ny, r1, r2 in p.get("noses", [])]
    for key in ("legs", "far_legs"):
        for lg in p.get(key, []):
            lg["pts"] = ws(lg["pts"])
    p["ears"] = [ws(e) for e in p.get("ears", [])]
    p["far_ears"] = [ws(e) for e in p.get("far_ears", [])]
    p["patches"] = [(*fn(cx, cy), rx, ry) for cx, cy, rx, ry in p.get("patches", [])]
    return p


def swap_legs(pose: dict) -> dict:
    p = copy.deepcopy(pose)
    p["legs"], p["far_legs"] = p["far_legs"], p["legs"]
    return p


def feet_down(pose: dict) -> dict:
    """Clamp every leg point to y >= 0.6 so a body lowered with ``warp`` keeps its claws on the floor row."""
    for key in ("legs", "far_legs"):
        for lg in pose.get(key, []):
            lg["pts"] = [(x, max(y, 0.6)) for x, y in lg["pts"]]
    return pose


def lerp_pts(a, b, t=0.5):
    return [(ax + (bx - ax) * t, ay + (by - ay) * t) for (ax, ay), (bx, by) in zip(a, b)]


def gather(spine_lift, tail_lift, hind_foot_x, front_foot_x, knee):
    """Walk gather/peak pose from the sprint contact key: feet pulled under the body, spine arched."""
    # Executor choice: the walk key already sits at the V13 headroom limit (top row 12 of a minimum of 11), so the
    # spec's "spine middle +2 / +3" is spent as a relative arch: the whole body drops ``drop`` units and the spine
    # middle rises ``spine_lift`` (net +1 over the key at the apex, ends lower), head, ears and face follow the drop.
    drop = {2: 1, 3: 2}[spine_lift]
    p = warp(derive("run1"), lambda x, y: (x, y - drop))
    p["stamps"] = [(x, y - drop, c) for x, y, c in POSES["run1"]["stamps"]]
    sp = p["spine"]
    p["spine"] = [sp[0], (sp[1][0], sp[1][1] + spine_lift / 2), (sp[2][0], sp[2][1] + spine_lift),
                  (sp[3][0], sp[3][1] + spine_lift / 2), sp[4]]
    p["tail"]["pts"] = mv(POSES["run1"]["tail"]["pts"], 0, tail_lift)
    nh, nf, fh, ff = knee
    p["legs"] = [
        {"pts": [(-12, 5.5), (-11.5, nh), (hind_foot_x, 0.6 if nh < 3 else 3.0)], "r": [2.6, 1.2, 1.0]},
        {"pts": [(10, 5.5), (11, nf), (front_foot_x, 0.6 if nf < 3 else 3.0)], "r": [1.9, 1.0, 0.9]},
    ]
    p["far_legs"] = [
        {"pts": [(-8, 5), (-6.5, fh), (-5, 0.6 if fh < 3 else 3.0)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(9, 5), (8.5, ff), (8, 0.6 if ff < 3 else 3.0)], "r": [1.8, 0.9, 0.8]},
    ]
    return p


def smear(x, y):
    """Lunge smear transform: the whole body 4 units back, the head end 4 units lower (hind claws stay put)."""
    t = min(1.0, max(0.0, (x + 13) / 26.0))
    return (x - 4, y - 4 * t)


def inbetweens() -> dict[int, dict]:
    out: dict[int, dict] = {}

    # --- idle: 1 inhale, 2 head low with the ear flicked back
    p = derive("idle0")
    p["spine"] = [(x, y + 1 if 1 <= i <= 4 else y) for i, (x, y) in enumerate(p["spine"])]
    p["tail"]["pts"][-1] = (-27.5, 6.5)
    p["patches"] = [(-4, 14.2, 3.2, 1.6), (3, 6.6, 2.0, 1.5)]
    out[1] = p

    p = derive("idle0")
    p["head"]["pts"] = mv(p["head"]["pts"], 0, -1)
    p["noses"] = [(nx, ny - 1, a, b) for nx, ny, a, b in p["noses"]]
    p["far_ears"] = [mv(e, 0, -1) for e in p["far_ears"]]
    p["ears"] = [ear(14, 7.6, 5.0, 5.0, -1.8, notch=True)]
    p["tail"]["pts"][-1] = (-27.5, 4.5)
    p["stamps"] = [(17, 4.6, "e"), (18, 4.6, "e"), (17, 5.6, "K"), (18, 5.6, "K"),
                   (19.5, 0.5, "t"), (20.5, 0.5, "t"), (19.5, -0.5, "t"), (20.5, -0.5, "T"),
                   (12, 13.5, "."), (12, 12.5, "K"), (13, 10.5, "K"), (14, 10.5, ".")]
    out[2] = p

    # --- walk: key 3 is contact A; 4 gather A, 5 peak A, 6 contact B, 7 gather B, 8 peak B
    out[4] = gather(2, 1, -10, 12, (2.5, 2.5, 2.5, 2.5))
    p5 = gather(3, 2, -9, 14, (2.2, 4.0, 4.2, 4.5))
    p5["legs"][0]["pts"] = [(-12, 5.5), (-10.5, 3), (-9, 0.6)]     # the one pixel left on the floor row
    out[5] = p5

    p = derive("run1", seed=12)
    p["legs"], p["far_legs"] = (
        [{"pts": [(-8, 5), (-6.5, 2.5), (-5, 0.6)], "r": [2.0, 1.0, 0.9]},
         {"pts": [(9, 5), (8, 3), (8, 0.6)], "r": [1.8, 0.9, 0.8]}],
        [{"pts": [(-12, 5.5), (-16, 2.5), (-19, 0.6)], "r": [2.6, 1.2, 1.0]},
         {"pts": [(10, 5.5), (14, 2.8), (18, 0.6)], "r": [1.9, 1.0, 0.9]}])
    out[6] = p
    out[7] = swap_legs(out[4]); out[7]["seed"] = 13
    p = swap_legs(out[5]); p["seed"] = 14
    p["tail"]["pts"] = mv(p["tail"]["pts"], 0, -1)
    out[8] = p

    # --- attack_tell: 9 onset (halfway toward the wedge, jaws shut, hackles 1 px, tail halfway)
    k = POSES["tell2"]
    p = copy.deepcopy(k)
    n = len(k["spine"])
    p["spine"] = [(x, y + (-1.0 + 3.0 * i / (n - 1))) for i, (x, y) in enumerate(k["spine"])]
    # Executor choice: spine head end +2 as written, but the head and face only +1 (a +2 head sits above the idle head,
    # which reads as a head-up stalk instead of the front dropping toward the floor).
    p["head"]["pts"] = mv(k["head"]["pts"], 0, 1)
    p["noses"] = [(nx, ny + 1, a, b) for nx, ny, a, b in k["noses"]]
    p["ears"] = [ear(10.5, 8.0, 5.0, 4.4, -2.8, notch=True)]
    p["tail"] = {"pts": lerp_pts(POSES["idle0"]["tail"]["pts"], k["tail"]["pts"]),
                 "r": [(a + b) / 2 for a, b in zip(POSES["idle0"]["tail"]["r"], k["tail"]["r"])]}
    p["legs"][0]["pts"][0] = (-9, 5)
    p["legs"][1]["pts"][0] = (-11, 3)
    p["legs"][2]["pts"][0] = (7, 5.5)
    p["legs"][3]["pts"][0] = (13.5, 4.4)
    p["far_legs"][0]["pts"][0] = (6, 6)
    p["tufts"] = [(-11, 1), (-9, 1), (-7, 1), (-5, 1), (-3, 1)]
    p["patches"] = [(-7, 8.6, 2.8, 1.6), (3, 7.2, 2.0, 1.4)]
    p["stamps"] = [(8, 12.5, "."), (8, 11.5, "K"), (15, 7.2, "e"), (16, 7.2, "e"), (15, 8.2, "K"), (16, 8.2, "K"),
                   (19.5, 4.5, "t"), (20.5, 4.5, "t"), (19.5, 3.5, "t"), (20.5, 3.5, "T")]
    out[9] = p

    # --- attack_swing: 11 lunge smear (jaws half open, speed streak), 13 snap-back (jaws shut)
    b = POSES["bite3"]
    p = warp(b, smear)
    p["tufts"] = [(x - 4, h) for x, h in b["tufts"]]
    keep = [(x - 4, y - 4, c) for x, y, c in b["stamps"]
            if not (c == "o" and (y == 9.5 or x in (18.5, 22.5))) and not (c == "t" and y == 8.6)]
    keep += [(21 - 4, 9.8 - 4, "t"), (22.5 - 4, 9.8 - 4, "t"), (-19.5, 6.5, "l"), (-20.5, 6.5, "l"), (-21.5, 6.5, "l")]
    p["stamps"] = keep
    out[11] = feet_down(p)

    p = derive("bite3")
    p["spine"] = [(-13, 2.5), (-7, 4.5), (0, 7), (5, 9.5), (9.5, 11.5)]
    p["head"]["pts"] = [(10, 12), (15, 12.4), (19, 11.4)]
    p["noses"] = [(20.2, 11.6, 1.0, 1.0)]
    p["far_legs"] = [{"pts": [(-8, 3), (-12, 2), (-15, 1)], "r": [2.0, 1.0, 0.9]},
                     {"pts": [(8, 9.5), (10.5, 8), (12, 7)], "r": [1.6, 0.9, 0.8]}]
    p["legs"] = [{"pts": [(-11, 3), (-14, 1.5), (-17, 0.6)], "r": [2.6, 1.2, 1.0]},
                 {"pts": [(8.5, 10), (11, 8.5), (13, 7.2)], "r": [1.9, 1.0, 0.9]},
                 {"pts": [(10, 9.5), (12.5, 7.5), (14.5, 6.5)], "r": [1.6, 1.3, 0.9]}]
    p["tail"]["pts"] = mv(p["tail"]["pts"], 0, -1)
    p["ears"] = [ear(9.5, 13.6, 5.0, 4.4, -2.8, notch=True)]
    p["patches"] = [(-4, 5, 3.4, 2.0)]
    p["stamps"] = [(7, 18.5, "."), (7, 17.5, "K"), (15, 12.8, "e"), (16, 12.8, "e"), (15, 13.8, "K"),
                   (18.5, 10.5, "t"), (19.5, 10.5, "t"), (18.5, 9.5, "t"), (19.5, 9.5, "T")]
    out[13] = p

    # --- hurt: 15 settle
    p = derive("hurt4")
    p["spine"] = [(-9, 3.2), (-7.8, 6.5), (-6, 9.2), (-4.2, 10.8)]
    p["belly"] = ([(-6.2, 4), (-4.8, 6.6), (-3.2, 8.8)], [1.3, 1.6, 1.2])
    p["head"]["pts"] = [(-3.8, 11.4), (0, 10.8), (3.4, 10.0)]
    p["noses"] = [(4.5, 9.9, 1.0, 1.0)]
    p["tail"]["pts"] = [(-12, 2), (-16, 3), (-18, 5.5), (-16.5, 7.5), (-19, 9)]
    p["far_legs"] = [{"pts": [(-6, 4), (-3, 1.4), (0, 0.6)], "r": [2.0, 1.0, 0.9]},
                     {"pts": [(-3.5, 9), (-1, 9.2), (1.5, 8.2)], "r": [1.2, 0.9, 0.8]}]
    p["legs"] = [{"pts": [(-8.5, 4), (-5.5, 1.4), (-2.5, 0.6)], "r": [2.8, 1.2, 1.0]},
                 {"pts": [(-3.5, 7.5), (-0.5, 7.2), (2.5, 6.0)], "r": [1.4, 0.9, 0.8]}]
    p["ears"] = [ear(-3.6, 12.6, 3.4, 3.2, -1.6, notch=True)]
    p["patches"] = [(-7.8, 7, 2.0, 1.6)]
    p["stamps"] = [(0.5, 12.2, "w"), (1.5, 12.2, "K"), (-0.5, 13.2, "K"), (0.5, 13.2, "K"),
                   (3.0, 8.3, "o"), (3.5, 9.2, "t"), (2.5, 9.2, "t")]
    out[15] = p

    # --- down: 16 flop (rolled onto its side), 17 bounce (belly-up, legs half raised)
    d = POSES["down5"]
    p = copy.deepcopy(d)
    p["spine"] = [(-12, 3.0), (-5, 3.8), (2, 4.4), (8, 4.2)]
    p["belly"] = ([(-8, 5.6), (-1, 6.4), (5, 6.0)], [1.2, 1.5, 1.2])
    p["head"]["pts"] = [(9, 3.4), (13, 3.2), (17, 2.2)]
    p["noses"] = [(18, 2.2, 1.0, 1.0)]
    p["tail"]["pts"] = [(-13, 2.5), (-18, 1.4), (-22, 1.2), (-26, 0.9)]
    p["far_legs"] = [{"pts": [(-5, 6), (-6, 8.6), (-3.5, 10)], "r": [1.0, 0.8, 0.7]},
                     {"pts": [(6, 6), (8, 8.2), (11, 8.6)], "r": [1.0, 0.8, 0.7]}]
    p["legs"] = [{"pts": [(-9, 5.5), (-13, 6.6), (-16, 6.0)], "r": [1.2, 0.9, 0.8]},
                 {"pts": [(4, 6.4), (8, 7.4), (12, 6.4)], "r": [1.2, 0.9, 0.8]}]
    p["ears"] = [ear(9, 4.8, 2.6, 2.4, -1.2)]
    p["stamps"] = [(11, 4.4, "K"), (12, 4.4, "K"), (18, 1.2, "q"), (19, 1.2, "q"), (17, 2.2, "o"), (16, 1.2, "t")]
    out[16] = p

    p = copy.deepcopy(d)
    for key in ("legs", "far_legs"):
        for lg in p[key]:
            lg["pts"][-1] = (lg["pts"][-1][0], lg["pts"][-1][1] - 2)
            lg["pts"][-2] = (lg["pts"][-2][0], lg["pts"][-2][1] - 1.2)
    p["tail"] = {"pts": [(-13, 2.5), (-18, 1.4), (-22, 1.8), (-25.5, 3.4), (-25, 5.8)], "r": [1.1, 0.9, 0.7, 0.6, 0.6]}
    out[17] = p
    return out


KEY_FRAMES = {0: "idle0", 3: "run1", 10: "tell2", 12: "bite3", 14: "hurt4", 18: "down5"}
# (tag, [(frame, ticks)]); ticks None = hold. 60 Hz, spec section 4.
TAGS = [("idle", [(0, 10), (1, 8), (2, 10)]),
        ("walk", [(3, 4), (4, 4), (5, 4), (6, 4), (7, 4), (8, 4)]),
        ("attack_tell", [(9, 6), (10, 18)]),
        ("attack_swing", [(11, 2), (12, 4), (13, 6)]),
        ("hurt", [(14, 4), (15, 8)]),
        ("down", [(16, 4), (17, 8), (18, None)])]
TICK_HZ = 60
HOLD_MS = 100
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once", "hurt": "once", "down": "hold"}


def all_poses() -> list[dict]:
    mid = inbetweens()
    return [POSES[KEY_FRAMES[n]] if n in KEY_FRAMES else mid[n] for n in range(19)]


# ------------------------------------------------------------------ size study (flat block-ins)

def scaled_idle(height: int) -> dict:
    """The idle pose uniformly scaled so the opaque height lands near ``height`` px (study only)."""
    s = (height - 2) / 16.0
    base = POSES["idle0"]

    def sc(v):
        return [(p[0] * s, p[1] * s) for p in v]
    pose = {"seed": 1, "spine": sc(base["spine"]), "spine_r": [r * s for r in base["spine_r"]],
            "head": {"pts": sc(base["head"]["pts"]), "r": [r * s for r in base["head"]["r"]]},
            "tail": {"pts": sc(base["tail"]["pts"]), "r": [r * s for r in base["tail"]["r"]]},
            "legs": [{"pts": sc(lg["pts"]), "r": [r * s for r in lg["r"]]} for lg in base["legs"]],
            "far_legs": [{"pts": sc(lg["pts"]), "r": [r * s for r in lg["r"]]} for lg in base["far_legs"]],
            "ears": [ear(13.2 * s, 8.7 * s, 3.4 * s, 4.6 * s, -0.4 * s)]}
    return pose


def to_grid(frames: list[dict], tags: list[dict]) -> dict:
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": tags,
    }


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    ticks = frame_ticks()
    frames = [{"rows": build(pose), "ms": duration_ms(ticks[n])} for n, pose in enumerate(all_poses())]
    tags = [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS]
    return to_grid(frames, tags)


def study_grid() -> dict:
    heights = (14, 18, 22)
    frames = [{"rows": build(scaled_idle(hh)), "ms": 500} for hh in heights]
    tags = [{"name": f"h{hh}", "from": i, "to": i, "direction": "forward"} for i, hh in enumerate(heights)]
    return to_grid(frames, tags)


def manifest() -> dict:
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": "key" if n in KEY_FRAMES else "sonnet"} for n in range(19)]
    return {
        "schema_version": 1,
        "id": "rat_dungeon_h18_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/rat/rat_h18.ase",
        "grid_source": "assets/source/characters/rat/rat_h18.grid.json",
        "output_atlas": "generated/art/rat_h18.png",
        "output_metadata": "generated/art/rat_h18.json",
        "palette_id": "rat-floor01-v1",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Dungeon Rat; runtime role IP-neutral small fast melee nuisance)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("RAT-01 Dungeon Rat at the decided 18 px height: 6 frozen Opus-edited keys (atlas frames 0, 3, 10, 12, 14, 18, "
                       "fixture tests/fixtures/rat-01_keys.json) plus 13 in-betweens built by scripts/author_rat01.py from the key pose "
                       "rules (named points moved, a few literal stamps, a deterministic despeckle pass); layer body only"),
            "generator": ("Draft keys, size study, palette v0 and packet: Claude Sonnet. Key edit, palette v1 and final design: Claude Opus. "
                          "In-betweens, atlas, validators and evidence: Claude Sonnet, composed by scripts/author_rat01.py. Independent "
                          "top-tier review: pending. No image-model raster output, no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-dungeon-rat-concept-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "rat": {
            "status": "keys final, in-betweens drawn, independent top-tier review pending; Bevy integration pending; 19 frames, 6 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/rat-01-spec.md",
            "opaque_height_idle": 18,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _f in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/rat-01_keys.json",
            "attack_ticks": {"tell": [0, 23], "swing_smear": [24, 25], "swing_impact": [26, 29], "recover": [30, 35]},
            "boxes_px_from_pivot": {
                "standing": {"x": [-13, 15], "y": [0, 12]},
                "hurtbox_idle_move_tell": {"x": [-14, 20], "y": [0, 12]},
                "hurtbox_hurt": {"x": [-13, 5], "y": [0, 21]},
                "attack_hitbox_impact_frame_only": {"x": [14, 27], "y": [6, 15], "frame": 12},
            },
            "frames": frames,
        },
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "size-study"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "frames":
        targets = [(KEYS, dump(frames_grid())), (MANIFEST, json.dumps(manifest(), indent=2) + "\n")]
    else:
        targets = [(STUDY, dump(study_grid()))]
    status = 0
    for path, text in targets:
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name} up to date" if same else f"{path.name} differs from the generator")
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    return status


if __name__ == "__main__":
    sys.exit(main())
