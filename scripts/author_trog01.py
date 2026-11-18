#!/usr/bin/env python3
"""TROG-01 authoring record: the Troglodyte Pygmy (family base) palette and the six draft key poses.

Usage:

    python3 scripts/author_trog01.py frames [--check]
    python3 scripts/author_trog01.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/trog_01.grid.json`` (pixel-grid-v1, layer ``body``)
and ``assets/manifests/trog-01.json``: one key per readability tag (idle, move contact, tell, attack, hurt,
defeat) and in-between slots that are plain copies of the nearest key for now. Every pose is a *rule*: thick
polylines (torso, limbs, dreadlocks) and polygons (ears, loincloth) rasterized at pixel centres in design units
(x right from the pivot column, y up from the contact row), a 1 px outline, a simple upper-left rim light and
literal face stamps. No raster image-model output, no tracing of concept pixels. ``--check`` compares instead
of writing (exit 1 on a difference).

Draft keys (first-draft design). The Basher and Virtuoso derive from the shared construction recorded in
``docs/02-art/trog-01-spec.md`` section 2b; they are not drawn here.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets/source/characters/troglodyte"
GRID = OUT_DIR / "trog_01.grid.json"
MANIFEST = ROOT / "assets/manifests/trog-01.json"
CANVAS = (80, 56)
PIVOT = (30, 55)

# trog-family-v0: one skin ramp, hair, rag cloth, fangs and mouth. 15 of 20 slots used; the other five are
# kept free for the family variants (Basher wraps/trash, Virtuoso robe/bells/tongue).
PALETTE = [
    ("K", "outline_plum", "#1a1017ff", False),
    ("d", "skin_deep", "#5a2c3fff", False),
    ("s", "skin_shade", "#8c4651ff", False),
    ("m", "skin_mid", "#bf6f64ff", False),
    ("l", "skin_light", "#e5a08cff", False),
    ("H", "hair_deep", "#1d1721ff", False),
    ("I", "hair_mid", "#403442ff", False),
    ("c", "cloth_deep", "#3b2f2cff", False),
    ("C", "cloth_mid", "#72604dff", False),
    ("w", "eye_white_drool", "#efe4c4ff", True),
    ("e", "eye_red", "#d83a22ff", True),
    ("t", "fang_yellow", "#e6bd3aff", True),
    ("T", "fang_shade", "#967022ff", False),
    ("o", "mouth_dark", "#4a1020ff", False),
    ("q", "tongue_pink", "#c8566aff", False),
]
RAMPS = {
    "skin": ("d", "s", "m", "l"),
    "far": ("K", "d", "d", "s"),
    "hair": ("H", "H", "I", "I"),
    "cloth": ("c", "c", "C", "C"),
}
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


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
    """Back-to-front part canvas (later parts occlude earlier ones; the outline follows part order)."""

    def __init__(self, w: int, h: int, floor_row: int):
        self.w, self.h = w, h
        self.px, self.bottom = PIVOT[0], floor_row - 1          # fill ends one row above the outline row
        self.part: list[list[int | None]] = [[None] * w for _ in range(h)]
        self.ramps: list[tuple] = []

    def P(self, x: float, y: float) -> tuple[float, float]:
        return (self.px + 0.5 + x, self.bottom + 1.0 - y)

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
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


# ------------------------------------------------------------------ face stamps
# Literal templates, top-left at (head_x - 3, head_y + 3) in design units; "." leaves the pixel alone.
FACES = {
    # closed snarl: brow shadow, red-rimmed eye, wide dark mouth line with two hanging fangs, a drool strand
    "snarl": ["..KKK...",
              "..wwK...",
              "...ew...",
              ".....s..",
              "..oooooo",
              "..t.t.tT",
              ".....w..",
              ".....w.."],
    # roar: stretched eye, teeth row over a dark open mouth, lower fangs, drool
    "roar": ["..KKK...",
             "..wwK...",
             "...ew...",
             "..ttttt.",
             ".ooooooo",
             ".ooooooo",
             "..t.t.tT",
             "...w..w."],
    # pain: eye squeezed to a stroke, mouth open flat
    "pain": ["..KKK...",
             "..KKK...",
             "........",
             "..ttttt.",
             ".ooooooo",
             ".ooqqoo.",
             "..t...t.",
             "......w."],
    # dead: crossed eye, slack jaw, tongue out
    "dead": ["........",
             "..K.K...",
             "...K....",
             "..K.K...",
             "..ooooo.",
             "..oqqqqq",
             "..t.qqq.",
             "......q."],
}


def build(pose: dict) -> list[str]:
    """Rasterize one pose dict into 56 row strings."""
    w, h = CANVAS
    c = Canvas(w, h, PIVOT[1])
    far = c.add("far"); hair = c.add("hair"); skin = c.add("skin"); cloth = c.add("cloth")
    # back to front: far limbs, back dreadlocks, far ear, torso, loincloth, head, forelock, near leg, near arm, near ear
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for dread in pose["dreads_back"]:
        c.thick(dread, [0.75] * len(dread), hair)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["loin"], cloth)
    c.thick(pose["head"]["pts"], pose["head"]["r"], skin)
    for dread in pose.get("forelock", []):
        c.thick(dread, [0.7] * len(dread), hair)
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], skin)
    for fin in pose.get("fingers", []):
        c.thick(fin, [0.55] * len(fin), skin)
    for ear in pose.get("ears", []):
        c.poly(ear, skin)
    rows = c.render()
    # ribs: short shade strokes (x, y_top, y_bottom) on lit/mid skin only: the lean flank
    for rx, ry0, ry1 in pose.get("ribs", []):
        col = int(c.P(rx, 0)[0])
        for y in range(int(c.P(0, ry0)[1]), int(c.P(0, ry1)[1]) + 1):
            if rows[y][col] in "ml":
                rows[y][col] = "s"
    # face template
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 3, hy + 3)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if rows[y][x] != "." or ch in "ewtT":
                    rows[y][x] = ch
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        rows[int(py)][int(px)] = key
    despeckle(rows)
    return ["".join(r) for r in rows]


def despeckle(rows: list[list[str]]) -> None:
    """Recolour orphan fill pixels to the most common neighbour key; outline and accent keys never move."""
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


# ------------------------------------------------------------------ the six key poses (design units)
def L(pts, r):
    return {"pts": pts, "r": r}


def ragged_loin(x0, y0, drop=7.0, lean=0.0):
    """Ragged loincloth hanging from the hips: x0, y0 is the hip centre; points hang with torn hems."""
    return [(x0 - 3.5, y0 + 2), (x0 + 3.5, y0 + 2), (x0 + 3 + lean, y0 - drop * 0.6),
            (x0 + 1.5 + lean, y0 - drop * 0.35), (x0 + 0.5 + lean, y0 - drop),
            (x0 - 1.2 + lean, y0 - drop * 0.5), (x0 - 2.5 + lean, y0 - drop * 0.85), (x0 - 3.5 + lean, y0 - drop * 0.3)]


def hand(x, y, spread=1.0, down=True):
    """Three long fingers from a wrist point; tips reach toward the ground when ``down``."""
    s = -1 if down else 1
    return [[(x, y), (x + 2.2 * spread, y + s * 1.2), (x + 4.2 * spread, y + s * 1.8)],
            [(x, y), (x + 2.0 * spread, y + 0.2), (x + 4.4 * spread, y + s * 0.2)],
            [(x, y), (x + 1.5 * spread, y - s * 1.0), (x + 3.2 * spread, y - s * 1.8)]]


POSES: dict[str, dict] = {}

# KEY 0 idle: hunched crouch, knuckles near the floor, head thrust forward, dreads standing up and back.
POSES["idle0"] = {
    "far_limbs": [L([(-5, 15), (1, 10), (0, 3), (5, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 21), (10, 14), (14, 8), (17, 5)], [1.7, 1.4, 1.1, 0.8])],
    "dreads_back": [[(7, 31), (5, 34), (2, 36)], [(9, 32), (9, 35), (8, 37)], [(5, 30), (1, 33), (-4, 33)],
                    [(6, 30), (3, 31), (-2, 29)]],
    "torso": L([(-10, 15), (-4, 21), (3, 25)], [4.0, 4.5, 4.1]),
    "near_legs": [L([(-9, 15), (-3, 9), (-6, 2), (-1, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 14, 7.5),
    "head": L([(9, 27), (12, 26)], [5.0, 4.6]),
    "forelock": [[(9, 31), (12, 31), (14, 29)]],
    "near_arms": [L([(4, 22), (9, 14), (12, 7), (15, 3)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(15, 3, 1.0),
    "ears": [[(8, 30), (3, 35), (5, 28)]],
    "ribs": [(-4, 14, 19), (-1, 15, 21), (2, 17, 22)],
    "face": "snarl", "face_at": (12, 27),
}

# KEY 1 walk contact (crouched scuttle): body low and forward, near arm planting ahead, far leg trailing, dreads streaming.
POSES["walk1"] = {
    "far_limbs": [L([(-8, 13), (-14, 8), (-17, 2), (-13, 0.8)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 17), (4, 10), (7, 4), (10, 1)], [1.7, 1.4, 1.1, 0.8])],
    "dreads_back": [[(9, 26), (4, 29), (-2, 29)], [(10, 27), (6, 32), (1, 33)], [(8, 25), (3, 26), (-3, 24)]],
    "torso": L([(-10, 13), (-3, 16), (5, 20)], [4.0, 4.4, 4.0]),
    "near_legs": [L([(-9, 13), (-1, 9), (2, 3), (7, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 12, 6.5, -2.0),
    "head": L([(11, 22), (14, 21)], [4.5, 4.2]),
    "forelock": [[(11, 26), (14, 26), (16, 24)]],
    "near_arms": [L([(5, 18), (12, 14), (18, 8), (21, 3)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(21, 3, 1.0),
    "ears": [[(10, 25), (5, 30), (7, 23)]],
    "ribs": [(-3, 12, 16), (0, 13, 18), (3, 15, 19)],
    "face": "snarl", "face_at": (14, 22),
}

# KEY 2 tell hold: coiled to spring, haunches raised and loaded behind, shoulders and head dropped low and thrust
# forward, both hands planted wide, mouth wide open. The lowest, most compact grounded pose: it reads as "about to go".
POSES["tell2"] = {
    "far_limbs": [L([(-9, 15), (-3, 9), (-7, 1.8), (-2, 0.8)], [2.0, 1.6, 1.1, 0.8]),
                  L([(5, 13), (10, 7), (15, 3), (19, 0.8)], [1.7, 1.4, 1.1, 0.8])],
    "dreads_back": [[(8, 20), (5, 25), (2, 27)], [(10, 21), (9, 26), (8, 28)], [(6, 19), (1, 22), (-4, 21)],
                    [(7, 18), (2, 17), (-4, 15)]],
    "torso": L([(-12, 16), (-5, 15), (3, 13)], [4.3, 4.6, 4.1]),
    "near_legs": [L([(-11, 16), (-5, 10), (-9, 2), (-4, 0.8)], [2.7, 2.2, 1.4, 1.0])],
    "loin": ragged_loin(-10, 16, 6.5, -1.0),
    "head": L([(9, 16), (12, 15)], [5.0, 4.7]),
    "forelock": [[(9, 20), (12, 21), (14, 19)]],
    "near_arms": [L([(4, 14), (9, 8), (14, 3), (17, 0.8)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(17, 0.8, 1.0),
    "ears": [[(7, 19), (2, 25), (4, 17)]],
    "ribs": [(-5, 12, 16), (-2, 11, 15), (1, 10, 14)],
    "face": "roar", "face_at": (12, 15),
}

# KEY 3 attack strike (airborne): body stretched flat, arms thrown forward with spread fingers, legs trailing, mouth open.
POSES["leap3"] = {
    "far_limbs": [L([(-12, 19), (-18, 15), (-24, 12), (-28, 11)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 22), (13, 25), (20, 26), (24, 24)], [1.7, 1.4, 1.1, 0.8])],
    "dreads_back": [[(11, 29), (5, 31), (-1, 31)], [(12, 30), (7, 34), (2, 35)], [(10, 28), (4, 28), (-2, 26)]],
    "torso": L([(-13, 17), (-5, 20), (4, 23)], [3.8, 4.3, 3.9]),
    "near_legs": [L([(-12, 18), (-17, 13), (-21, 8), (-25, 6)], [2.3, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(-12, 17, 5.5, -4.0),
    "head": L([(11, 25), (14, 25)], [4.6, 4.3]),
    "forelock": [[(11, 29), (14, 30), (16, 28)]],
    "near_arms": [L([(5, 23), (13, 26), (20, 25), (25, 22)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(25, 22, 1.0, down=True),
    "ears": [[(10, 28), (5, 33), (7, 26)]],
    "ribs": [(-5, 17, 20), (-2, 18, 22), (1, 19, 23)],
    "face": "roar", "face_at": (14, 25),
}

# KEY 4 hurt: thrown upright and back, head snapped up, arms flung, eyes shut tight, mouth open.
POSES["hurt4"] = {
    "far_limbs": [L([(-5, 15), (1, 9), (1, 3), (6, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(2, 25), (-3, 30), (-7, 33), (-9, 36)], [1.7, 1.4, 1.1, 0.8])],
    "dreads_back": [[(5, 36), (1, 39), (-4, 39)], [(7, 37), (5, 40), (2, 41)], [(3, 35), (-2, 36), (-7, 35)]],
    "torso": L([(-8, 14), (-4, 21), (0, 28)], [4.0, 4.3, 3.9]),
    "near_legs": [L([(-7, 14), (-2, 8), (-7, 2), (-2, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-6, 13, 7.0, 1.0),
    "head": L([(2, 33), (5, 34)], [5.0, 4.6]),
    "forelock": [[(2, 37), (5, 38), (7, 36)]],
    "near_arms": [L([(2, 26), (8, 28), (13, 26), (17, 22)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(17, 22, 1.0, down=True),
    "ears": [[(1, 36), (-4, 41), (-2, 34)]],
    "ribs": [(-6, 14, 19), (-3, 17, 23), (0, 21, 27)],
    "face": "pain", "face_at": (5, 34),
}

# KEY 5 defeat (final): on its back on the floor, limbs splayed, tongue out, eyes crossed, loincloth torn open.
POSES["down5"] = {
    "far_limbs": [L([(-12, 4), (-14, 9), (-9, 12), (-6, 8)], [1.8, 1.5, 1.1, 0.8]),
                  L([(4, 5), (11, 3), (17, 1.5), (21, 1)], [1.6, 1.3, 1.0, 0.8])],
    "dreads_back": [[(11, 6), (15, 4), (20, 2)], [(11, 5), (16, 6), (21, 5)], [(10, 7), (14, 9), (18, 9)]],
    "torso": L([(-15, 4), (-6, 5), (4, 5.5)], [3.4, 3.6, 3.4]),
    "near_legs": [L([(-14, 4), (-9, 10), (-3, 8), (0, 2)], [2.2, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(-13, 5, 3.5, 3.0),
    "head": L([(9, 6), (11, 6)], [4.2, 4.0]),
    "forelock": [[(7, 9), (10, 10), (13, 8)]],
    "near_arms": [L([(4, 6), (8, 9), (11, 13), (10, 17)], [1.9, 1.5, 1.1, 0.8])],
    "fingers": hand(10, 17, 0.7, down=False),
    "ears": [[(8, 9), (4, 14), (6, 7)]],
    "ribs": [(-8, 3, 7), (-5, 3, 8), (-2, 3, 8)],
    "face": "dead", "face_at": (10, 6),
}

KEY_FRAMES = {0: "idle0", 3: "walk1", 8: "tell2", 10: "leap3", 12: "hurt4", 15: "down5"}
# in-between slots are copies of the nearest key until the executor draws them (spec section 8)
COPY_OF = {1: 0, 2: 0, 4: 3, 5: 3, 6: 3, 7: 8, 9: 10, 13: 15, 14: 15}
LAND_FRAME = 11          # grounded landing: the tell crouch plus a little dust inside the frame
TAGS = [("idle", [(0, 12), (1, 8), (2, 12)]),
        ("walk", [(3, 6), (4, 6), (5, 6), (6, 6)]),
        ("attack_tell", [(7, 6), (8, 18)]),
        ("attack_leap", [(9, 3), (10, 6)]),
        ("attack_land", [(11, 14)]),
        ("hurt", [(12, 12)]),
        ("down", [(13, 4), (14, 8), (15, None)])]
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_leap": "once", "attack_land": "once",
            "hurt": "once", "down": "hold"}
AIRBORNE = ("attack_leap",)
TICK_HZ = 60
HOLD_MS = 100

# placeholder boxes (px from the pivot: x forward, y up), measured from the key bounding boxes; not approved mechanics
BOXES = {
    "standing": {"x": [-14, 20], "y": [0, 39]},
    "hurtbox_idle_walk": {"x": [-14, 17], "y": [0, 36]},
    "hurtbox_tell_land": {"x": [-17, 14], "y": [0, 24]},
    "hurtbox_leap": {"x": [-24, 18], "y": [5, 37]},
    "hurtbox_hurt": {"x": [-12, 12], "y": [0, 43]},
    "hurtbox_down": {"x": [-19, 14], "y": [0, 20]},
    "attack_hitbox_impact_frame_only": {"x": [12, 30], "y": [16, 34], "frame": 10},
}
DUST = [(-10, 1, "C"), (-9, 0, "c"), (-12, 2, "C"), (-13, 1, "c"), (-3, 0, "C"), (-1, 1, "c"), (17, 1, "C"), (20, 0, "c"), (21, 2, "C")]


def pose_for(n: int) -> dict:
    if n in KEY_FRAMES:
        return POSES[KEY_FRAMES[n]]
    if n == LAND_FRAME:
        return POSES["tell2"] | {"stamps": DUST}
    return POSES[KEY_FRAMES[COPY_OF[n]]]


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    ticks = frame_ticks()
    n_frames = len(ticks)
    frames = [{"rows": build(pose_for(n)), "ms": duration_ms(ticks[n])} for n in range(n_frames)]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    tags = [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS]
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": tags,
    }


def manifest() -> dict:
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": "key" if n in KEY_FRAMES else "copy"} for n in range(len(ticks))]
    return {
        "schema_version": 1,
        "id": "troglodyte_pygmy_h40_candidate_v0",
        "type": "character_animation",
        "source_path": "assets/source/characters/troglodyte/trog_01.ase",
        "grid_source": "assets/source/characters/troglodyte/trog_01.grid.json",
        "output_atlas": "generated/art/trog_01.png",
        "output_metadata": "generated/art/trog_01.json",
        "palette_id": "trog-family-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 troglodyte Pygmy; runtime role IP-neutral small fast pouncer)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("TROG-01 Pygmy at a 40 px idle height: 6 draft keys (atlas frames 0, 3, 8, 10, 12, 15) rasterized from pose rules by "
                       "scripts/author_trog01.py; the other slots are copies of the nearest key (frame 11 is the tell crouch plus landing dust); "
                       "layer body only"),
            "generator": "Draft spec and keys: Claude Sonnet. Review and key edit: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "pygmy": {
            "status": "draft keys only, in-betweens are copies, not reviewed, not frozen by a review; Bevy integration pending; 16 frames, 6 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/trog-01-spec.md",
            "opaque_height_idle": 40,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _f in TAGS if name not in AIRBORNE],
            "airborne_tags": list(AIRBORNE),
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/trog-01_keys.json",
            "attack_ticks": {"tell": [0, 23], "leap_launch": [24, 26], "leap_strike_impact": [27, 32], "land_recover": [33, 46]},
            "boxes_px_from_pivot": BOXES,
            "frames": frames,
        },
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-01/key0-scale.png") -> Path:
    """Scale evidence: key 0 at 1x on lit Floor 1 next to Carl and GOB-01 (640 x 360, no scaling), plus a 4x
    copy of key 0 on a side panel. Needs the Floor 1 kit atlases (``bash scripts/export_assets.sh``).
    Run: ``python3 scripts/author_trog01.py key0-scale``."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    gob, gob_piv = reference_idle("goblin")
    grid = load_grid_file(GRID)
    pygmy = grid_images(grid)[0]
    g_img, g_piv = flip(gob, gob_piv)
    p_img, p_piv = flip(pygmy, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (g_img, g_piv, CARL_X + 70, 0), (p_img, p_piv, CARL_X + 150, 0)]
    scene = Floor().render(entries, 1)
    panel_w = pygmy.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, scene.height), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = pygmy.resize((panel_w, pygmy.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 40), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    targets = [(GRID, dump(frames_grid())), (MANIFEST, json.dumps(manifest(), indent=2) + "\n")]
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
