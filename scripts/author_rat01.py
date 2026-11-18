#!/usr/bin/env python3
"""RAT-01 authoring record: Dungeon Rat palette, size block-ins and the six draft key poses.

Usage:

    python3 scripts/author_rat01.py keys [--check]
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
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets/source/characters/rat"
KEYS = OUT_DIR / "rat_h18.grid.json"
STUDY = OUT_DIR / "rat_size_study.grid.json"
CANVAS = (64, 28)
PIVOT = (30, 27)

# rat-floor01-v0: cold blue-gray fur (so it does not melt into the warm orange brick), warm rim on the torch
# side, saturated pink bald patches / tail / ears, ember-red eyes, tobacco-yellow teeth.
PALETTE = [
    ("K", "outline_plum", "#1a1218ff", False),
    ("a", "fur_deep", "#2b2830ff", False),
    ("s", "fur_shade", "#4a4550ff", False),
    ("m", "fur_mid", "#6e6670ff", False),
    ("l", "fur_light", "#9a8f86ff", False),
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
    """Triangle ear with a base centred at (x, y)."""
    pts = [(x - w / 2, y), (x + lean, y + h_), (x + w / 2, y)]
    return pts


def far_ear(x, y, w, h_, lean=0.0):
    return ear(x, y, w, h_, lean)


# Each pose: design units (x right of the pivot column, y up from the contact row).
POSES: dict[str, dict] = {}

POSES["idle0"] = {
    "seed": 1,
    "spine": [(-13, 5), (-9, 8.4), (-3, 10.8), (3, 10.2), (9, 7.6), (12, 5.8)],
    "spine_r": [2.4, 3.4, 3.5, 3.4, 2.8, 2.4],
    "head": {"pts": [(12, 5.8), (16, 4.6), (20.5, 3.2)], "r": [3.4, 2.7, 1.2]},
    "noses": [(21.6, 3.2, 1.0, 1.0)],
    "tail": {"pts": [(-14, 4), (-18, 3), (-22, 4), (-25, 6.5), (-27, 5)], "r": [1.1, 0.9, 0.8, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-8, 5), (-7, 2), (-5, 0.5)], "r": [2.2, 1.0, 0.9]},
        {"pts": [(6, 4), (8, 2), (10, 0.5)], "r": [1.8, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-11, 5), (-10, 2.5), (-7, 0.5)], "r": [2.8, 1.2, 1.0]},
        {"pts": [(9, 4.5), (10, 2.2), (12.5, 0.5)], "r": [2.0, 1.0, 0.9]},
    ],
    "far_ears": [far_ear(11, 7.8, 3, 3.5, 0.5)],
    "ears": [ear(13.4, 8.2, 3.4, 4.4, -0.4)],
    "patches": [(-7, 8.5, 3.6, 2.2), (4, 6, 2.6, 1.8)],
    "tufts": [(-9, 1), (-4, 1), (2, 1), (7, 1)],
    "stamps": [(17, 5.6, "e"), (18, 5.6, "e"), (17, 6.6, "K"), (19.5, 1.9, "t"), (19.5, 1.1, "t"), (21.4, 1.6, "t"), (21.4, 0.9, "t")],
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
    "ears": [ear(13, 8.4, 3, 3.4, -2.2)],
    "patches": [(-6, 7.4, 3.2, 1.8)],
    "tufts": [(-6, 1), (1, 1)],
    "stamps": [(19, 7, "e"), (20, 7, "e"), (21.5, 3.1, "t"), (21.5, 2.4, "t"), (23.5, 3.1, "t")],
}

POSES["tell2"] = {
    "seed": 3,
    "spine": [(-12, 7), (-8, 7.2), (-2, 6), (4, 5), (9, 4.4)],
    "spine_r": [3.0, 3.4, 3.2, 3.0, 2.6],
    "head": {"pts": [(10, 5), (15, 4.5), (20, 3.6)], "r": [3.0, 2.5, 1.2]},
    "noses": [(21, 3.8, 1.0, 1.0)],
    "tail": {"pts": [(-13, 6), (-17, 8), (-19, 11), (-18, 14), (-15, 15.5)], "r": [1.1, 0.9, 0.8, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(6, 3.8), (9, 2), (12, 0.6)], "r": [1.8, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-11, 5), (-9, 1.5), (-5, 0.6)], "r": [3.2, 1.6, 1.0]},
        {"pts": [(8, 3.8), (12, 1.8), (16, 0.6)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(12, 2.8), (16, 1.6), (19.5, 1.0)], "r": [1.4, 1.0, 0.7]},
    ],
    "far_ears": [],
    "ears": [ear(10, 7.2, 3, 3, -2.5)],
    "patches": [(-8, 7, 3.2, 2.0)],
    "tufts": [(-10, 1), (-5, 1)],
    "stamps": [(16, 5.4, "e"), (17, 5.4, "e"), (16, 6.4, "K"), (18, 2.4, "o"), (19, 2.4, "o"), (20, 2.4, "o"), (18.5, 3.2, "t"), (20.5, 3.2, "t"), (19, 1.6, "t"), (20, 1.6, "t")],
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
    "ears": [ear(13.5, 15, 3, 3.2, -2.4)],
    "patches": [(-4, 6, 3.4, 2.0)],
    "tufts": [(-6, 1), (2, 1)],
    "stamps": [(19, 13.8, "e"), (20, 13.8, "e"), (19, 14.8, "K"),
               (18.5, 10.5, "o"), (19.5, 10.5, "o"), (20.5, 10.5, "o"), (21.5, 10.5, "o"), (22.5, 10.5, "o"),
               (19.5, 9.5, "o"), (20.5, 9.5, "o"), (21.5, 9.5, "o"),
               (20, 11.4, "t"), (22, 11.4, "t"), (23, 11.4, "t"), (21, 8.6, "t"), (22.5, 8.6, "t")],
}

POSES["hurt4"] = {
    "seed": 5,
    "spine": [(-13, 4.5), (-8, 6.5), (-3, 9), (2, 12), (5, 14.5)],
    "spine_r": [2.4, 3.2, 3.4, 3.2, 2.6],
    "head": {"pts": [(6, 14.6), (9.5, 16.2), (13, 17.2)], "r": [3.2, 2.5, 1.2]},
    "noses": [(14, 17.6, 1.0, 1.0)],
    "tail": {"pts": [(-14, 4), (-19, 5), (-22, 8), (-20, 11)], "r": [1.1, 0.9, 0.7, 0.6]},
    "far_legs": [
        {"pts": [(-8, 5), (-5, 2), (-3, 0.6)], "r": [2.0, 1.0, 0.9]},
        {"pts": [(3, 10), (6, 12.5), (8, 11.5)], "r": [1.6, 0.9, 0.8]},
    ],
    "legs": [
        {"pts": [(-11, 4.5), (-9, 2), (-6, 0.6)], "r": [2.8, 1.2, 1.0]},
        {"pts": [(5, 11), (9, 12.5), (12, 11)], "r": [1.8, 0.9, 0.8]},
    ],
    "far_ears": [],
    "ears": [ear(6.5, 16.2, 3, 3.2, -2.4)],
    "patches": [(-6, 7, 3.2, 2.0)],
    "tufts": [(-7, 1)],
    "stamps": [(9.5, 17.8, "K"), (10.5, 17.2, "K"), (11.5, 17.8, "K"), (11.5, 15.0, "o"), (12.5, 15.4, "o"),
               (12.5, 14.4, "t"), (11.5, 14.0, "T")],
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


def keys_grid() -> dict:
    order = [("idle", "idle0", 167), ("walk", "run1", 67), ("attack_tell", "tell2", 300),
             ("attack_swing", "bite3", 67), ("hurt", "hurt4", 67), ("down", "down5", None)]
    frames, tags = [], []
    for i, (tag, key, ms) in enumerate(order):
        if key not in POSES:
            continue
        frames.append({"rows": build(POSES[key]), "ms": ms if ms is not None else 100})
        tags.append({"name": tag, "from": len(frames) - 1, "to": len(frames) - 1, "direction": "forward"})
    return to_grid(frames, tags)


def study_grid() -> dict:
    heights = (14, 18, 22)
    frames = [{"rows": build(scaled_idle(hh)), "ms": 500} for hh in heights]
    tags = [{"name": f"h{hh}", "from": i, "to": i, "direction": "forward"} for i, hh in enumerate(heights)]
    return to_grid(frames, tags)


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "size-study"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    path, grid = (KEYS, keys_grid()) if args.command == "keys" else (STUDY, study_grid())
    text = dump(grid)
    if args.check:
        same = path.is_file() and path.read_text(encoding="utf-8") == text
        print(f"{path.name} up to date" if same else f"{path.name} differs from the generator")
        return 0 if same else 1
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"wrote {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
