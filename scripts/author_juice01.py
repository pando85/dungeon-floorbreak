#!/usr/bin/env python3
"""JUICE-01 authoring record: the Juicer (Floor 1 Gym troglodyte boss) palette and key poses.

Usage:

    python3 scripts/author_juice01.py frames [--check]
    python3 scripts/author_juice01.py key0-scale
    python3 scripts/author_juice01.py keys-x3

``frames`` rebuilds ``assets/source/characters/troglodyte/juicer/juice_01.grid.json`` (pixel-grid-v1, layer ``body``)
and ``assets/manifests/juice-01.json``: the six keys (idle, walk contact, tell hold, pound impact, hurt, defeat) in atlas
order, one grid frame each, until the in-betweens are drawn (``docs/02-art/juice-01-spec.md`` sections 4, 7 and 8).

The Juicer is redrawn at its own 100 px size from the troglodyte family construction (``docs/02-art/trog-01-spec.md``
section 2b, ``scripts/author_trog01.py``): the same 15 core colours, part order, 1 px outline, face order, mop and
rope-dread hair, pointed swept-back ear, ragged loincloth, grime and dirt-caked soles. It is never scaled up from
Pygmy or Basher pixels. The boss adds round muscle volumes: every limb and muscle blob is shaded from its own surface
normal (light from the upper left, the torch side), so the mass reads as lumps and creases instead of a flat fill.

Every pose is a *rule*: thick polylines and discs rasterized at pixel centres in design units (x right from the pivot
column, y up from the contact row) plus literal face stamps. No raster image-model output, no tracing of concept
pixels. ``--check`` compares instead of writing (exit 1 on a difference). Presentation-only art tooling; nothing here
is wired into the game.
"""
from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_trog01 as trog  # noqa: E402  (the family palette core and construction constants)

OUT_DIR = ROOT / "assets/source/characters/troglodyte/juicer"
GRID = OUT_DIR / "juice_01.grid.json"
MANIFEST = ROOT / "assets/manifests/juice-01.json"
CANVAS = (160, 136)
PIVOT = (72, 135)
H = 100                     # idle opaque height (orchestrator default, pending owner confirmation)

# trog-family-v0 core (15 colours, frozen, imported, never recoloured) plus four Juicer keys: 19 of 20.
EXTRA = [
    ("b", "boil_red", "#b3283aff", False),       # swollen boils (body detail, not an effect)
    ("v", "vein_purple", "#5d3878ff", False),    # swollen veins, 1 px strands on lit and mid skin
    ("i", "iron_deep", "#353a48ff", False),      # chain cuffs and waist chain, shade
    ("j", "iron_light", "#8b93a6ff", False),     # chain links, cold lit edge
]
PALETTE = list(trog.PALETTE) + EXTRA
RAMPS = dict(trog.RAMPS)
RAMPS["iron"] = ("K", "i", "i", "j")
RAMPS["farfist"] = ("K", "d", "s", "m")          # the far fist: one step darker than the near fist, still countable
PROTECTED = {k for k, _n, _c, acc in PALETTE if acc} | {"b", "q", "v", "i", "j"}


def palette_json(keys: set[str]) -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        if key in keys:
            entry = {"key": key, "name": name, "rgba": rgba}
            if accent:
                entry["accent"] = True
            out.append(entry)
    return out


class Canvas:
    """Back-to-front part canvas. Each filled pixel keeps its part id and a surface normal (from the nearest axis
    point of a thick line, or the centre of a disc) used for volume shading; polygons (cloth) have no normal and use
    the family's edge-rim rule."""

    def __init__(self):
        self.w, self.h = CANVAS
        self.px, self.bottom = PIVOT[0], PIVOT[1] - 1     # fill ends one row above the contact (outline) row
        self.part: list[list[int | None]] = [[None] * self.w for _ in range(self.h)]
        self.nrm: list[list[tuple[float, float] | None]] = [[None] * self.w for _ in range(self.h)]
        self.ramps: list[tuple] = []

    def P(self, x: float, y: float) -> tuple[float, float]:
        return (self.px + 0.5 + x, self.bottom + 1.0 - y)

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1

    def _cells(self, x0, x1, y0, y1):
        for y in range(max(0, int(y0) - 1), min(self.bottom + 1, int(y1) + 2)):
            for x in range(max(0, int(x0) - 1), min(self.w, int(x1) + 2)):
                yield x, y

    def disc(self, cx, cy, rx, ry, pid):
        cx, cy = self.P(cx, cy)
        for x, y in self._cells(cx - rx, cx + rx, cy - ry, cy + ry):
            nx, ny = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            if nx * nx + ny * ny <= 1.0:
                self.part[y][x], self.nrm[y][x] = pid, (nx, ny)

    def thick(self, pts, radii, pid):
        best: dict[tuple[int, int], tuple[float, tuple[float, float]]] = {}
        for i in range(len(pts) - 1):
            (ax, ay), (bx, by) = self.P(*pts[i]), self.P(*pts[i + 1])
            ra, rb = radii[i], radii[i + 1]
            dx, dy = bx - ax, by - ay
            ll = dx * dx + dy * dy or 1e-9
            r = max(ra, rb)
            for x, y in self._cells(min(ax, bx) - r, max(ax, bx) + r, min(ay, by) - r, max(ay, by) + r):
                qx, qy = x + 0.5, y + 0.5
                t = max(0.0, min(1.0, ((qx - ax) * dx + (qy - ay) * dy) / ll))
                rr = ra + (rb - ra) * t
                ox, oy = qx - (ax + dx * t), qy - (ay + dy * t)
                d = math.hypot(ox, oy) / rr
                if d <= 1.0 and ((x, y) not in best or d < best[(x, y)][0]):
                    best[(x, y)] = (d, (ox / rr, oy / rr))
        for (x, y), (_d, n) in best.items():
            self.part[y][x], self.nrm[y][x] = pid, n

    def poly(self, pts, pid):
        pts = [self.P(*p) for p in pts]

        def inside(qx, qy):
            c = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > qy) != (yb > qy) and qx < xa + (qy - ya) * (xb - xa) / (yb - ya):
                    c = not c
            return c
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        for x, y in self._cells(min(xs), max(xs), min(ys), max(ys)):
            if inside(x + 0.5, y + 0.5):
                self.part[y][x], self.nrm[y][x] = pid, None

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
                deep, shade, mid, light = self.ramps[pid]
                rim = n4[0] != pid or n4[1] != pid
                n = self.nrm[y][x]
                if n is None:                                   # cloth: the family edge rule
                    if rim:
                        out[y][x] = light
                    elif n4[2] != pid or n4[3] != pid or get(x + 1, y + 1) != pid:
                        out[y][x] = shade
                    else:
                        out[y][x] = mid
                    continue
                nx, ny = n
                z = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
                lam = -0.5 * nx - 0.62 * ny + 0.6 * z        # torch light from the upper left and the front
                if lam > 0.87 or (rim and lam > 0.25):
                    out[y][x] = light
                elif lam > 0.36:
                    out[y][x] = mid
                elif lam > -0.05:
                    out[y][x] = shade
                else:
                    out[y][x] = deep
        return out


def L(pts, r):
    return {"pts": pts, "r": r}


def rot(x, y, deg):
    a = math.radians(deg)
    return (x * math.cos(a) - y * math.sin(a), x * math.sin(a) + y * math.cos(a))


# ------------------------------------------------------------------ head, face and hair (family construction, redrawn)
SKULL_R, JAW_R = 7.5, 7.0          # head width about 0.21 H: smaller than the Basher's 0.25 H (2b: small head, big body)


def head(hx, hy, tilt=-2.0):
    return L([(hx - 6, hy - tilt), (hx, hy)], [SKULL_R, JAW_R])


def dropped_jaw(hx, hy):
    """The roar's hanging lower jaw under the front circle (family ``jaw``, redrawn at 100 px)."""
    return L([(hx - 7, hy - 4), (hx + 5, hy - 8)], [3.5, 3.0])


# Face stamps, 15 columns; top-left at (hx - 7, hy + 6). Family order: heavy K brow, bulging bloodshot eye (white,
# red rim behind and below, pupil at the front), upper teeth, dark mouth that breaks the front outline, lower fangs,
# drool. "." leaves the pixel alone; the accents (e, w, t, T) and the tongue q may overhang the silhouette.
FACES = {
    # idle and walk: a 3-row half-open growl (canon: a roaring open mouth) with 2 px lower fangs; well under the roar
    "growl": ["...............",
              "....KKKKKKK....",
              "....eewwwwK....",
              ".....ewwwKK....",
              "......ee.......",
              "...........s...",
              "...tTttTttTtt..",
              "...oooooooooooo",
              "...oooooooooooo",
              "...ooooooooooo.",
              "...tT..tT..tT..",
              "...t...t...t...",
              "...w.......w...",
              "...w.......w...",
              "...........w..."],
    # tell and pound: the roar, a 7-row gape over the dropped jaw, tongue at the bottom, lower fangs, two drool strands
    "roar": ["...............",
             "....KKKKKKK....",
             "...eeewwwwK....",
             "....eewwwKK....",
             "......ee.......",
             "..tTttTttTttT..",
             ".oooooooooooooo",
             ".oooooooooooooo",
             ".oooooooooooooo",
             ".oooooooooooooo",
             ".oooooooooooooo",
             "..ooooqqqqqoooo",
             "...oooqqqqoo...",
             "...tT.tTt.tT...",
             "...t...t...t...",
             "...w.......w...",
             "...w.......w..."],
    # hurt: eye squeezed to a K slit (no red), a yelping open mouth, one drool strand
    "pain": ["...............",
             "....KKKKKKK....",
             ".....KKKKKs....",
             "...............",
             "...............",
             "...tTttTttT....",
             "..oooooooooooo.",
             "..oooooooooooo.",
             "..ooooooooooo..",
             "...tT..tT..t...",
             "...............",
             "........w......",
             "........w......"],
    # defeat: crossed eye, slack mouth, the tongue hanging out of the front corner
    "dead": ["...............",
             "...............",
             ".....K...K.....",
             "......K.K......",
             ".......K.......",
             "......K.K......",
             ".....K...K.....",
             "...............",
             "...ooooooooo...",
             "...t.tT.t.qqq..",
             "..........qqq..",
             "...........qq..",
             "...........q..."],
}

# Mop and rope dreads relative to the jaw centre: the Pygmy's rules at 1.3x, rope radius 1.5 (the Basher's). Ropes 0
# to 2 hang down the back of the skull; 4 and 5 drape over the trapezius hump and break its back outline; 6 falls in
# front of the cheek, behind the mouth. A ragged fringe of three 1 px strands hangs over the brow, and two 1 px notches
# split the dome, so the mop reads as clumped ropes, never a helmet. No point above the crown; every rope ends lower.
MOP = [(-12.5, 4), (-13, 9), (-10.5, 11.5), (-9.5, 14.5), (-7, 15), (-4.5, 17), (-2.5, 15.5), (-0.5, 16.2),
       (1.6, 13), (0.5, 10.5), (-3, 8.6), (-6.5, 6.6), (-9, 5)]
ROPES = [[(-10, 12), (-15, 10.5), (-20, 6), (-22.5, 0), (-23, -5)],
         [(-11, 8), (-15, 4.5), (-16.5, -1), (-16, -6)],
         [(-4, 14), (-8, 12.5), (-13, 10), (-17, 6)],
         [(0, 13), (2.6, 11), (3.6, 8.2)],
         [(-10, 12), (-17, 9), (-24, 4), (-29, -4), (-31, -12)],
         [(-7, 15), (-15, 16.5), (-24, 15.5), (-32, 11), (-38, 4), (-40, -3)],
         [(-4, 9), (-5.5, 3), (-6, -3), (-7, -7)]]
ROPE_R = 1.5
FRINGE = [[(-0.5, 10), (0, 7.6)], [(1.5, 12), (2, 8.6)], [(3.5, 11), (4.2, 9.4)]]
NOTCHES = [(-6, 15.8), (-1.6, 16.4)]
BEADS = {0: "t", 1: "w", 4: "t"}
EAR = [(-5.5, 6.5), (-12, 16), (-9.8, 5.5)]          # swept back and up, the tip 4 to 5 px outside the mop
EAR_LINE = [(-8, 7.5), (-10.4, 12.6)]


def hair(hx, hy, sweep=0.0, turn=0.0):
    """``turn`` rotates the whole hair set around the jaw centre (head thrown back, lying down); ``sweep`` streams the
    rope tips back and up (positive, motion) or throws them forward (negative, recoil), growing from 0 at the root."""
    def tr(pts, swept=False):
        n = max(1, len(pts) - 1)
        out = []
        for i, (x, y) in enumerate(pts):
            if swept:
                x, y = x - 4.0 * sweep * i / n, y + 3.0 * sweep * i / n
            x, y = rot(x, y, turn)
            out.append((hx + x, hy + y))
        return out
    return {"mop": tr(MOP), "ropes": [tr(r, True) for r in ROPES], "fringe": [tr(f) for f in FRINGE],
            "notches": tr(NOTCHES), "ear": tr(EAR), "ear_line": tr(EAR_LINE)}


def ragged_loin(x0, x1, y0, drop):
    """Ragged loincloth hanging down from the hip line between x0 and x1, four torn points on the hem."""
    w = x1 - x0
    return [(x0, y0), (x1, y0 - 1.5), (x1 - 0.05 * w, y0 - drop * 0.6), (x1 - 0.15 * w, y0 - drop * 0.95),
            (x1 - 0.3 * w, y0 - drop * 0.62), (x1 - 0.45 * w, y0 - drop), (x1 - 0.58 * w, y0 - drop * 0.55),
            (x1 - 0.72 * w, y0 - drop * 0.88), (x1 - 0.86 * w, y0 - drop * 0.5), (x0 + 0.02 * w, y0 - drop * 0.7)]


# ------------------------------------------------------------------ fists, open hands, cuffs
def knuckle_lines(cx, cy, r, ang, n=3):
    """Short creases on the knuckle face of a fist whose knuckles point at ``ang`` degrees (0 = forward, 90 = up)."""
    d = (math.cos(math.radians(ang)), math.sin(math.radians(ang)))
    p = (-d[1], d[0])
    ks = (-1, 0, 1) if n == 3 else (-0.5, 0.5)
    return [[(cx + d[0] * 0.4 * r + p[0] * k * 0.36 * r, cy + d[1] * 0.4 * r + p[1] * k * 0.36 * r),
             (cx + d[0] * 0.85 * r + p[0] * k * 0.36 * r, cy + d[1] * 0.85 * r + p[1] * k * 0.36 * r)] for k in ks]


def thumb_line(cx, cy, ang):
    """The thumb wrapped over the index knuckle (authored at -20 degrees, rotated with the fist)."""
    return [(cx + x, cy + y) for x, y in (rot(px, py, ang + 20) for px, py in ((-9, 6), (-2, 7), (3, 5)))]


def open_hand(x, y, r, ang):
    """Open, fanned hand (hurt): a palm disc and four fingers spread around ``ang``, nothing balled."""
    fingers = []
    for da in (-42, -14, 14, 42):
        a = math.radians(ang + da)
        fingers.append([(x + math.cos(a) * r * 0.6, y + math.sin(a) * r * 0.6),
                        (x + math.cos(a) * r * 2.2, y + math.sin(a) * r * 2.2)])
    return {"palm": (x, y, r, r), "fingers": fingers}


def cuff_line(elbow, wrist, half):
    ex, ey = elbow; wx, wy = wrist
    ln = math.hypot(wx - ex, wy - ey) or 1.0
    px, py = -(wy - ey) / ln, (wx - ex) / ln
    cx, cy = wx + (ex - wx) / ln * 1.5, wy + (ey - wy) / ln * 1.5
    return [(cx - px * half, cy - py * half), (cx + px * half, cy + py * half)]


def hanging_chain(cuff):
    """Broken chain from the back (wrist) end of the cuff, dropping behind the fist."""
    bx, by = min(cuff, key=lambda p: p[0])
    return [(bx, by), (bx - 2, by - 7), (bx - 1, by - 14), (bx - 3, by - 21)]


# Boils, veins and grime are anchored to named muscle masses (dx, dy from the mass centre), so every key carries the
# same body detail. A boil or vein only lands on visible skin.
BOILS = [("traps", -5, 5, 2.6), ("traps", 6, 4, 2.0), ("deltoid", -5, 7, 3.0), ("deltoid", 4, -4, 2.2),
         ("lat", -4, 7, 2.6), ("lat", -8, -6, 2.0), ("bicep", 2, -3, 2.0), ("traps", -8, 1, 1.6), ("calf", 0, 12, 1.8),
         ("gut", 5, -1, 1.6), ("glute", -5, 6, 1.8)]
VEINS = [("forearm", [(0, 6), (2, 1), (0, -3), (3, -7)]), ("bicep", [(-1, 6), (2, 0), (1, -5)]),
         ("pec", [(-5, -2), (-1, -5), (0, -9)]), ("calf", [(-6, 15), (-3, 11), (-1, 6)]), ("gut", [(-5, 7), (-2, 2)])]
GRIME = [("glute", -3, 12), ("lat", 6, 2), ("gut", -2, -5), ("forearm", -5, -3), ("lat", -2, -10), ("pec", -7, -5),
         ("calf", 2, -2), ("traps", -4, 9), ("fist", -6, -6), ("glute", -7, -8)]

POSES: dict[str, dict] = {}

# KEY 0 idle: a hunched, wide block: short bowed legs planted wide, the trapezius hump as the top of the silhouette with
# the small head sunk in front of it, both arms hanging forward so two very large fists hang at knee height ahead of
# the feet (the far fist peeks out ahead of and above the near one). Iron cuffs on both wrists, a broken chain from
# the near cuff dropping behind the fist, a chain sagging across the hips over the rag.
POSES["idle0"] = {
    "far_legs": [L([(-14, 34), (-23, 18), (-21, 5), (-11, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(14, 68), (28, 52), (45, 42)], [8.0, 7.0, 6.5])],
    "far_fist": (51, 36, 9.0, -20),
    "torso": L([(-8, 36), (-6, 62), (-1, 82)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-15, 38, 12, 11), "lat": (-10, 68, 13, 13), "gut": (8, 45, 14, 13), "pec": (13, 63, 12, 10),
              "traps": (2, 87, 14, 11)},
    "near_legs": [L([(0, 34), (13, 19), (10, 5), (21, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (12, 12, 4.5, 6.0),
    "loin": ragged_loin(-15, 13, 35, 21),
    "waist_chain": [(-21, 38), (-8, 33), (4, 32), (17, 35)],
    "head_at": (28, 80), "tilt": -2.0, "face": "growl", "hair": {},
    "near_arm": L([(9, 70), (23, 53), (31, 38)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (7, 73, 12, 11.5), "bicep": (17, 60, 8, 9), "forearm": (27, 47, 8.5, 9)},
    "fist": (36, 26, 11.5, -20),
}

# KEY 3 walk contact: a heavy stomp. Trunk 2 px lower and 3 px forward, near foot planted forward, far foot pushing
# off behind, near fist swung back over the thigh, far fist swung forward; ropes streaming back a little.
POSES["walk3"] = {
    "far_legs": [L([(-11, 32), (-17, 17), (-27, 5), (-19, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(17, 66), (34, 52), (48, 40)], [8.0, 7.0, 6.5])],
    "far_fist": (54, 33, 9.0, -20),
    "torso": L([(-5, 34), (-3, 60), (2, 80)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-12, 36, 12, 11), "lat": (-7, 66, 13, 13), "gut": (11, 43, 14, 13), "pec": (16, 61, 12, 10),
              "traps": (5, 85, 14, 11)},
    "near_legs": [L([(3, 32), (17, 18), (21, 5), (31, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (19, 11, 4.5, 6.0),
    "loin": ragged_loin(-12, 16, 33, 20),
    "waist_chain": [(-18, 36), (-5, 31), (7, 30), (20, 33)],
    "head_at": (31, 78), "tilt": -2.0, "face": "growl", "hair": {"sweep": 0.4},
    "near_arm": L([(12, 68), (23, 52), (28, 37)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (10, 71, 12, 11.5), "bicep": (18, 59, 8, 9), "forearm": (25, 45, 8.5, 9)},
    "fist": (32, 24, 11.5, -35),
}

# KEY 8 tell hold (the study's roar): torso straightened, both fists raised overhead in a V, head tipped back with the
# jaw dropped in the full roar, legs braced wider. The tallest frame of the set: "the hammer is up".
POSES["tell8"] = {
    "far_legs": [L([(-14, 34), (-26, 18), (-26, 5), (-16, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(-6, 84), (-16, 98), (-12, 110)], [8.0, 7.0, 6.5])],
    "far_fist": (-10, 118, 9.5, 100),
    "torso": L([(-6, 38), (-4, 64), (-1, 84)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-14, 38, 12, 11), "lat": (-11, 70, 13, 13), "gut": (8, 48, 13, 13), "pec": (12, 70, 12, 10),
              "traps": (0, 90, 13, 10)},
    "near_legs": [L([(2, 34), (16, 20), (16, 5), (27, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (15, 12, 4.5, 6.0),
    "loin": ragged_loin(-14, 12, 36, 21),
    "waist_chain": [(-20, 40), (-8, 35), (4, 34), (16, 37)],
    "head_at": (28, 86), "tilt": 0.0, "face": "roar", "jaw_drop": True, "hair": {"turn": 12},
    "near_arm": L([(6, 82), (8, 100), (14, 110)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (6, 84, 12, 11), "bicep": (7, 96, 8, 9), "forearm": (11, 105, 8, 8.5)},
    "fist": (16, 118, 11.0, 80),
}

# KEY 10 pound impact: both fists smashed into the floor ahead of the feet, trunk folded flat forward with the rump up,
# head low between the arms still roaring. The lowest, widest grounded pose. (The debris burst is a VFX order.)
POSES["pound10"] = {
    "far_legs": [L([(-18, 38), (-27, 22), (-35, 6), (-27, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(26, 56), (42, 38), (53, 20)], [8.0, 7.0, 6.5])],
    "far_fist": (57, 10, 9.5, -70),
    "torso": L([(-14, 40), (0, 52), (14, 56)], [13.0, 15.0, 13.0]),
    "blobs": {"glute": (-17, 43, 11, 10), "lat": (-2, 58, 13, 10), "gut": (4, 42, 13, 11), "pec": (18, 46, 11, 9),
              "traps": (12, 59, 13, 9)},
    "near_legs": [L([(-10, 38), (4, 22), (-2, 5), (8, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (1, 13, 4.5, 6.0),
    "loin": ragged_loin(-20, 4, 40, 20),
    "waist_chain": [(-24, 44), (-12, 38), (0, 37), (10, 41)],
    "head_at": (40, 46), "tilt": -1.0, "face": "roar", "jaw_drop": True, "hair": {"sweep": -0.3},
    "near_arm": L([(22, 52), (30, 32), (40, 18)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (20, 54, 12, 11), "bicep": (25, 43, 8, 9), "forearm": (35, 26, 8.5, 8.5)},
    "fist": (44, 11, 11.0, -80),
}

# KEY 12 hurt: recoil back from the hips, the head thrown up above the hump with the eye squeezed shut, both hands
# flung up and back and opened (nothing points at the player), weight on the back foot, ropes whipped forward.
POSES["hurt12"] = {
    "far_legs": [L([(-12, 34), (-18, 18), (-22, 5), (-13, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(-8, 84), (-22, 92), (-32, 97)], [8.0, 7.0, 6.5])],
    "far_hand": open_hand(-36, 100, 4.0, 150),
    "torso": L([(-4, 36), (-10, 60), (-14, 80)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-12, 36, 12, 11), "lat": (-20, 64, 12, 12), "gut": (4, 48, 13, 12), "pec": (2, 66, 12, 10),
              "traps": (-12, 86, 13, 10), "neck": (-3, 88, 7, 7)},
    "near_legs": [L([(2, 34), (14, 20), (18, 5), (28, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (17, 12, 4.5, 6.0),
    "loin": ragged_loin(-14, 12, 35, 21),
    "waist_chain": [(-20, 38), (-8, 33), (4, 32), (16, 35)],
    "head_at": (4, 96), "tilt": 0.0, "face": "pain", "jaw_drop": True, "hair": {"turn": 18, "sweep": -0.6},
    "near_arm": L([(-4, 80), (-20, 83), (-33, 84)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (-3, 80, 11, 10), "bicep": (-12, 82, 8, 8), "forearm": (-26, 84, 8, 7.5)},
    "hand": open_hand(-38, 86, 5.0, 165),
}

# KEY 15 defeat: on his back, fallen backward (the hurt direction): hips toward the player, the hump and the head on
# the floor behind, chin on the chest, the mop spilled on the floor; near knee up, far leg out flat; near arm slack
# along the floor with the fist by the hips, far fist on the floor behind the head; crossed eye, tongue out.
POSES["down15"] = {
    "far_legs": [L([(8, 8), (30, 14), (42, 4), (50, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(-28, 18), (-40, 22), (-52, 14)], [8.0, 7.0, 6.5])],
    "far_fist": (-56, 8, 8.0, 200),
    "torso": L([(10, 10), (-10, 13), (-30, 12)], [12.0, 14.0, 12.0]),
    "blobs": {"glute": (14, 9, 10, 8), "lat": (-18, 9, 12, 8), "gut": (-4, 18, 14, 10), "pec": (-20, 19, 11, 8),
              "traps": (-34, 12, 9, 9)},
    "near_legs": [L([(12, 10), (24, 25), (32, 10), (40, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (28, 17, 4.5, 6.0),
    "loin": ragged_loin(4, 20, 18, 12),
    "waist_chain": [(-4, 24), (4, 25), (12, 22)],
    "head_at": (-44, 9), "tilt": -1.0, "face": "dead", "hair": {"turn": 75},
    "near_arm": L([(-24, 18), (-14, 8), (0, 5)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (-24, 18, 11, 9), "bicep": (-19, 13, 8, 7), "forearm": (-7, 6, 8, 6)},
    "fist": (6, 8, 10.0, -10),
}

# ------------------------------------------------------------------ the pose rig: in-betweens (spec section 8)
def lerp(a, b, t):
    """Blend two pose values of the same shape (numbers, tuples, lists, dicts, limb dicts); other values come from ``b``
    from t 0.5 on."""
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        return a + (b - a) * t
    if isinstance(a, dict) and isinstance(b, dict):
        return {k: lerp(a[k], b[k], t) for k in a if k in b}
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)) and len(a) == len(b):
        out = [lerp(x, y, t) for x, y in zip(a, b)]
        return tuple(out) if isinstance(a, tuple) else out
    return b if t >= 0.5 else a


def shift(pose, parts, dx, dy):
    """Add (dx, dy) to every point of the listed parts (point lists, limb dicts, blob dicts, single points)."""
    def mv(v):
        if isinstance(v, dict) and "pts" in v:
            return {"pts": [(x + dx, y + dy) for x, y in v["pts"]], "r": v["r"]}
        if isinstance(v, dict):
            return {k: mv(w) for k, w in v.items()}
        if isinstance(v, list) and v and isinstance(v[0], dict):
            return [mv(w) for w in v]
        if isinstance(v, list):
            return [(x + dx, y + dy) for x, y in v]
        return (v[0] + dx, v[1] + dy) + tuple(v[2:])          # blob, fist or head point
    for part in parts:
        pose[part] = mv(pose[part])
    return pose


def arms_follow(pose, fist, far_fist, base_fist, base_far_fist):
    """Move the near and far fists to the given centres and let the arm ends, forearm blob and cuff follow (a swing)."""
    for key, new, old in (("fist", fist, base_fist), ("far_fist", far_fist, base_far_fist)):
        pose[key] = (new[0], new[1]) + tuple(pose[key][2:])
    d = (fist[0] - base_fist[0], fist[1] - base_fist[1])
    pts = list(pose["near_arm"]["pts"])
    pts[-1] = (pts[-1][0] + d[0], pts[-1][1] + d[1]); pts[-2] = (pts[-2][0] + 0.4 * d[0], pts[-2][1] + 0.4 * d[1])
    pose["near_arm"] = L(pts, pose["near_arm"]["r"])
    fb = list(pose["arm_blobs"]["forearm"]); pose["arm_blobs"]["forearm"] = (fb[0] + 0.8 * d[0], fb[1] + 0.8 * d[1], fb[2], fb[3])
    bb = list(pose["arm_blobs"]["bicep"]); pose["arm_blobs"]["bicep"] = (bb[0] + 0.3 * d[0], bb[1] + 0.3 * d[1], bb[2], bb[3])
    e = (far_fist[0] - base_far_fist[0], far_fist[1] - base_far_fist[1])
    fp = list(pose["far_arms"][0]["pts"])
    fp[-1] = (fp[-1][0] + e[0], fp[-1][1] + e[1]); fp[-2] = (fp[-2][0] + 0.4 * e[0], fp[-2][1] + 0.4 * e[1])
    pose["far_arms"] = [L(fp, pose["far_arms"][0]["r"])]
    return pose


def set_calf(pose):
    """The calf disc sits between the near knee and ankle (as in every key)."""
    (_k0, (kx, ky), (ax, ay), _f) = pose["near_legs"][0]["pts"]
    pose["calf"] = ((kx + ax) / 2, (ky + ay) / 2 - 0.5, pose["calf"][2], pose["calf"][3])


def fit_arm_blobs(pose):
    """Deltoid at the shoulder, bicep and forearm between the arm points (the keys' rule)."""
    p0, p1, p2 = pose["near_arm"]["pts"]
    pose["arm_blobs"] = {"deltoid": (p0[0] - 1, p0[1] + 2, 12, 11.5),
                         "bicep": ((p0[0] + p1[0]) / 2 + 0.5, (p0[1] + p1[1]) / 2 - 1, 8, 9),
                         "forearm": ((p1[0] + p2[0]) / 2, (p1[1] + p2[1]) / 2 - 1, 8.5, 9)}


TRUNK = ["torso", "blobs", "near_arm", "arm_blobs", "far_arms", "head_at", "loin", "waist_chain"]
BASE_FIST, BASE_FAR = (32, 24), (54, 33)          # walk3
HIP_PARTS = ["torso", "blobs", "near_legs", "far_legs", "calf", "loin", "waist_chain"]


def walk_pose(near_leg, far_leg, fist, far_fist, sweep, bob=(0, 0)):
    p = copy.deepcopy(POSES["walk3"])
    p["near_legs"] = [L(near_leg, p["near_legs"][0]["r"])]
    p["far_legs"] = [L(far_leg, p["far_legs"][0]["r"])]
    shift(p, TRUNK, *bob)
    set_calf(p)
    p["hair"] = {"sweep": sweep}
    return arms_follow(p, fist, far_fist, (BASE_FIST[0] + bob[0], BASE_FIST[1] + bob[1]),
                       (BASE_FAR[0] + bob[0], BASE_FAR[1] + bob[1]))


# 1, 2 idle breath
p = copy.deepcopy(POSES["idle0"])
shift(p, ["torso", "blobs", "near_arm", "arm_blobs", "far_arms"], 0, 1)
p["head_at"] = (28, 81)
p["fist"] = (35, 27, 11.5, -20); p["far_fist"] = (50, 37, 9.0, -20)
POSES["breath1"] = p
p = copy.deepcopy(POSES["idle0"])
g = p["blobs"]["gut"]; p["blobs"] = dict(p["blobs"], gut=(g[0], g[1] - 1, g[2], g[3]))
p["fist"] = (37, 26, 11.5, -20); p["far_fist"] = (52, 36, 9.0, -20)
POSES["breath2"] = p

# 4, 5, 6 walk (passing, contact B, passing B). Even planted-foot travel of 19 to 20 px per 8 ticks around key 3's
# frozen wide stride (toe points: near 31 -> 12 -> -8, far 0 -> 20 -> 0 -> -19), and the swinging foot lifted off the
# contact row on both passing frames (toe at y 6), so the gait is a heavy stomp, not a shuffle or a limp.
POSES["pass4"] = walk_pose([(2, 34), (10, 20), (2, 5), (12, 1.5)], [(-12, 34), (-4, 18), (-8, 9.5), (0, 6)],
                           (30, 27), (52, 35), 0.2, bob=(-1, 2))
POSES["contactb5"] = walk_pose([(3, 32), (-4, 17), (-18, 5), (-8, 1.5)], [(-11, 32), (2, 18), (12, 5), (20, 1.5)],
                               (40, 30), (46, 26), 0.4)
POSES["passb6"] = walk_pose([(2, 34), (12, 19), (2, 9.5), (12, 6)], [(-12, 34), (-4, 20), (-8, 5), (0, 1.5)],
                            (35, 26), (49, 33), 0.2, bob=(-1, 2))

# 7 tell rise: torso, blobs and legs at the midpoint of idle0 and tell8; the fists pulled up to the chest
p = lerp(POSES["idle0"], POSES["tell8"], 0.5)
# executor note: the section 8 near arm and fist are drawn 6 px further back than listed, so the wrist cuff and the
# fist clear the mouth (at the listed x the cuff cut across the face for the 8 ticks of this frame)
p["near_arm"] = L([(2, 76), (10, 74), (12, 84)], [10.5, 9.0, 8.0])
p["fist"] = (14, 90, 11.2, 60)
p["far_arms"] = [L([(-2, 80), (-6, 88), (-2, 92)], [8.0, 7.0, 6.5])]
p["far_fist"] = (2, 96, 9.2, 80)
fit_arm_blobs(p)
p["head_at"] = (28, 84); p["tilt"] = -1.0; p["face"] = "growl"; p["jaw_drop"] = True; p["hair"] = {"turn": 6}
set_calf(p)
POSES["rise7"] = p

# 9 smear: torso at 0.6 of the way tell8 -> pound10, legs as pound10
p = copy.deepcopy(POSES["pound10"])
mix = lerp(POSES["tell8"], POSES["pound10"], 0.6)
for part in ("torso", "blobs", "loin", "waist_chain"):
    p[part] = mix[part]
p["near_arm"] = L([(16, 70), (30, 66), (40, 56)], [10.5, 9.0, 8.0])
p["fist"] = (44, 48, 11.0, 0)
p["far_arms"] = [L([(18, 74), (36, 66), (50, 56)], [8.0, 7.0, 6.5])]
p["far_fist"] = (54, 50, 9.5, -10)
fit_arm_blobs(p)
p["head_at"] = (36, 66); p["tilt"] = -0.6; p["hair"] = {"sweep": -0.18, "turn": 5}
POSES["smear9"] = p

# 11 recover: pound10, head up, panting growl, traps and lat up, rump up; the fists stay planted
p = copy.deepcopy(POSES["pound10"])
x, y = p["head_at"]; p["head_at"] = (x, y + 6); p["tilt"] = -2.0; p["face"] = "growl"; p["jaw_drop"] = False
b = dict(p["blobs"])
for name, dy in (("traps", 3), ("lat", 3), ("glute", 2)):
    b[name] = (b[name][0], b[name][1] + dy, b[name][2], b[name][3])
p["blobs"] = b
POSES["recover11"] = p

# 13, 14 down: the fall beat (t 0.35, open hands still) and the floor impact (t 0.8, fists)
def fall(t, hurt_hands):
    h, d = POSES["hurt12"], POSES["down15"]
    p = lerp({k: v for k, v in h.items() if k not in ("far_hand", "hand", "face", "hair", "jaw_drop")},
             {k: v for k, v in d.items() if k not in ("far_fist", "fist", "face", "hair", "jaw_drop")}, t)
    far_end = lerp(h["far_hand"]["palm"], d["far_fist"], t)
    near_end = lerp(h["hand"]["palm"], d["fist"], t)
    if hurt_hands:
        p["far_hand"] = open_hand(far_end[0], far_end[1], far_end[2], 150 + 50 * t)
        p["hand"] = open_hand(near_end[0], near_end[1], near_end[2], 165 + 35 * t)
    else:
        p["far_fist"] = (far_end[0], far_end[1], far_end[2], lerp(-30, 200, t))
        p["fist"] = (near_end[0], near_end[1], near_end[2], lerp(-30, -10, t))
    return p, lerp(h["hair"].get("sweep", 0), 0, t)


p, sw = fall(0.35, True)
p["face"] = "pain"; p["jaw_drop"] = True; p["hair"] = {"turn": 40, "sweep": sw}
POSES["fall13"] = p
p, sw = fall(0.8, False)
k = list(p["near_legs"][0]["pts"]); k[1] = (k[1][0], k[1][1] + 4)
p["near_legs"] = [L(k, p["near_legs"][0]["r"])]
set_calf(p)
p["face"] = "dead"; p["jaw_drop"] = False; p["hair"] = {"turn": 65}
POSES["impact14"] = p

# The full atlas, in order: (atlas index, tag, pose, ticks, author)
FRAMES = [(0, "idle", "idle0", 14, "key"), (1, "idle", "breath1", 10, "in-between"), (2, "idle", "breath2", 14, "in-between"),
          (3, "walk", "walk3", 8, "key"), (4, "walk", "pass4", 8, "in-between"), (5, "walk", "contactb5", 8, "in-between"),
          (6, "walk", "passb6", 8, "in-between"), (7, "attack_tell", "rise7", 8, "in-between"),
          (8, "attack_tell", "tell8", 28, "key"), (9, "attack_pound", "smear9", 3, "in-between"),
          (10, "attack_pound", "pound10", 6, "key"), (11, "attack_recover", "recover11", 24, "in-between"),
          (12, "hurt", "hurt12", 14, "key"), (13, "down", "fall13", 6, "in-between"),
          (14, "down", "impact14", 10, "in-between"), (15, "down", "down15", None, "key")]
KEYS = [f for f in FRAMES if f[4] == "key"]
TAGS: list[tuple[str, list[tuple[int, int | None]]]] = []
for _a, _tag, _p, _t, _au in FRAMES:
    if TAGS and TAGS[-1][0] == _tag:
        TAGS[-1][1].append((_a, _t))
    else:
        TAGS.append((_tag, [(_a, _t)]))
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_pound": "once", "attack_recover": "once",
            "hurt": "once", "down": "hold"}
TICK_HZ = 60
HOLD_MS = 100


def stroke(rows, c, pts, key, on, dark=None):
    """1 px polyline in design units; recolours pixels whose key is in ``on`` (``dark`` replaces ``key`` on d)."""
    for i in range(len(pts) - 1):
        (ax, ay), (bx, by) = c.P(*pts[i]), c.P(*pts[i + 1])
        n = max(1, int(max(abs(bx - ax), abs(by - ay)) * 2))
        for k in range(n + 1):
            x, y = int(ax + (bx - ax) * k / n), int(ay + (by - ay) * k / n)
            if 0 <= x < c.w and 0 <= y < c.h and rows[y][x] in on:
                rows[y][x] = dark if (dark and rows[y][x] == "d") else key


def build(pose: dict) -> list[str]:
    c = Canvas()
    far = c.add("far"); farfist = c.add("farfist"); far2 = c.add("farfist"); far_iron = c.add("iron")
    skin = c.add("skin"); leg = c.add("skin"); cloth = c.add("cloth"); waist = c.add("iron"); hd = c.add("skin")
    hair_id = c.add("hair"); arm = c.add("skin"); arm2 = c.add("skin"); chain = c.add("iron"); fist = c.add("skin")
    thumb = c.add("skin"); iron = c.add("iron"); ear = c.add("skin")
    hx, hy = pose["head_at"]
    hr = hair(hx, hy, **pose["hair"])
    anchors = dict(pose["blobs"]); anchors.update(pose["arm_blobs"]); anchors["calf"] = pose["calf"]
    # back to front: far leg and arm, far fist or hand, far cuff
    for limb in pose["far_legs"] + pose["far_arms"]:
        c.thick(limb["pts"], limb["r"], far)
    if "far_fist" in pose:
        fx, fy, fr, _a = pose["far_fist"]
        c.disc(fx, fy, fr, fr, farfist)
    if "far_hand" in pose:
        c.disc(*pose["far_hand"]["palm"], farfist)
        for i, f in enumerate(pose["far_hand"]["fingers"]):
            c.thick(f, [1.4, 1.0], far2 if i % 2 else farfist)
    fa = pose["far_arms"][0]["pts"]
    c.thick(cuff_line(fa[-2], fa[-1], 7.0), [2.1, 2.1], far_iron)
    # trunk, near leg, rag, waist chain
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for b in pose["blobs"].values():
        c.disc(*b, skin)
    for limb in pose["near_legs"]:
        c.thick(limb["pts"], limb["r"], leg)
    c.disc(*pose["calf"], leg)
    c.poly(pose["loin"], cloth)
    c.thick(pose["waist_chain"], [1.3] * len(pose["waist_chain"]), waist)
    # head, jaw, hair (mop, ropes, fringe), notches in the dome
    h = head(hx, hy, pose["tilt"])
    c.thick(h["pts"], h["r"], hd)
    if pose.get("jaw_drop"):
        j = dropped_jaw(hx, hy)
        c.thick(j["pts"], j["r"], hd)
    c.poly(hr["mop"], hair_id)
    for rope in hr["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    for f in hr["fringe"]:
        c.thick(f, [0.75, 0.6], hair_id)
    for nx, ny in hr["notches"]:
        px, py = c.P(nx, ny)
        for dy in (0, 1):
            if 0 <= int(py) + dy < c.h and c.part[int(py) + dy][int(px)] == hair_id:
                c.part[int(py) + dy][int(px)] = None
    # near arm, chain behind the fist, fist or open hand, thumb, cuff, ear
    c.thick(pose["near_arm"]["pts"], pose["near_arm"]["r"], arm)
    for b in pose["arm_blobs"].values():
        c.disc(*b, arm)
    na = pose["near_arm"]["pts"]
    cuff = cuff_line(na[-2], na[-1], 8.3)
    chain_pts = hanging_chain(cuff)
    c.thick(chain_pts, [1.1] * len(chain_pts), chain)
    if "fist" in pose:
        fx, fy, fr, fa_ = pose["fist"]
        c.disc(fx, fy, fr, fr, fist)
        anchors["fist"] = (fx, fy, fr, fr)
        c.thick(thumb_line(fx, fy, fa_), [2.2, 2.0, 1.7], thumb)
    if "hand" in pose:
        c.disc(*pose["hand"]["palm"], fist)
        for i, f in enumerate(pose["hand"]["fingers"]):
            c.thick(f, [1.7, 1.2], thumb if i % 2 else arm2)
    c.thick(cuff, [2.3, 2.3], iron)
    c.poly(hr["ear"], ear)
    rows = c.render()
    # dread texture and beads (family rule)
    for y in range(c.h):
        for x in range(c.w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
    for i, key in BEADS.items():
        bx, by = c.P(*hr["ropes"][i][-2])
        if 0 <= int(by) < c.h and c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    # chain links: alternate lit and dark pairs so the iron reads as links, not a rod
    for y in range(c.h):
        for x in range(c.w):
            if c.part[y][x] in (chain, waist) and rows[y][x] in "ij":
                rows[y][x] = "j" if ((x if c.part[y][x] == waist else y) // 2) % 2 == 0 else "i"
    # mottled rim (family rule): no run of more than 5 clean lit skin pixels
    for y in range(c.h):
        run = 0
        for x in range(c.w + 1):
            if x < c.w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    despeckle(rows)
    # inner-ear line, knuckle creases (near fist 3, far fist 2)
    stroke(rows, c, hr["ear_line"], "s", "ml")
    if "fist" in pose:
        for ln in knuckle_lines(*pose["fist"]):
            stroke(rows, c, ln, "d", "sml", dark="K")
    if "far_fist" in pose:
        fx, fy, fr, fa_ = pose["far_fist"]
        for ln in knuckle_lines(fx, fy, fr, fa_, n=2):
            stroke(rows, c, ln, "K", "dsm")
    # veins: 1 px purple strands on lit and mid skin
    for name, pts in VEINS:
        ax, ay = anchors[name][:2]
        stroke(rows, c, [(ax + x, ay + y) for x, y in pts], "v", "mls")
    # boils: raised red domes: a light-skin highlight upper left, a pink inner rim, a dark rim under the lower edge
    skin_parts = {skin, leg, hd, arm, fist}
    for name, dx, dy, r in BOILS:
        ax, ay = anchors[name][:2]
        cx, cy = c.P(ax + dx, ay + dy)
        hl = (int(cx - 0.45 * r), int(cy - 0.45 * r))
        for y in range(int(cy - r) - 2, int(cy + r) + 3):
            for x in range(int(cx - r) - 2, int(cx + r) + 3):
                if not (0 <= x < c.w and 0 <= y < c.h) or c.part[y][x] not in skin_parts or rows[y][x] not in "dsml":
                    continue
                ox, oy = x + 0.5 - cx, y + 0.5 - cy
                d = math.hypot(ox, oy)
                if d <= r:
                    rows[y][x] = "l" if (r >= 2.0 and (x, y) == hl) else ("q" if (-ox - oy) / r > 0.75 else "b")
                elif d <= r + 1.0 and r >= 2.0 and ox + oy > 0.3 * r:
                    rows[y][x] = "d"
    # grime smudges and dirt-caked soles and knuckles (family rule)
    for name, gx, gy in GRIME:
        if name not in anchors:
            continue
        ax, ay = anchors[name][:2]
        px, py = c.P(ax + gx, ay + gy)
        for dx in (0, 1):
            if 0 <= int(py) < c.h and 0 <= int(px) + dx < c.w and rows[int(py)][int(px) + dx] in "ml":
                rows[int(py)][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    # face
    fx, fy = c.P(hx - 7, hy + 6)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            x, y = int(fx) + rx, int(fy) + ry
            if ch != "." and 0 <= y < c.h and (rows[y][x] != "." or ch in "ewtTq"):
                rows[y][x] = ch
    return ["".join(r) for r in rows]


def despeckle(rows: list[list[str]]) -> None:
    """Recolour orphan fill pixels to the most common neighbour key; outline, accents and body details never move."""
    h, w = len(rows), len(rows[0])
    for _ in range(2):
        for y in range(h):
            for x in range(w):
                ch = rows[y][x]
                if ch in ".K" or ch in PROTECTED:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if ch in around:
                    continue
                cand = [a for a in around if a not in ".K"]
                if cand:
                    rows[y][x] = max(sorted(set(cand)), key=cand.count)


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    frames = [{"rows": build(POSES[p]), "ms": duration_ms(t)} for _a, _tag, p, t, _au in FRAMES]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS],
    }


def manifest() -> dict:
    return {
        "schema_version": 1,
        "id": "troglodyte_juicer_h100_candidate_v0",
        "type": "character_animation",
        "source_path": "assets/source/characters/troglodyte/juicer/juice_01.ase",
        "grid_source": "assets/source/characters/troglodyte/juicer/juice_01.grid.json",
        "output_atlas": "generated/art/juice_01.png",
        "output_metadata": "generated/art/juice_01.json",
        "palette_id": "trog-family-v0+juicer",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored boss animation candidate (fan-concept layer: Floor 1 Gym troglodyte boss; runtime role IP-neutral heavy boss)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("JUICE-01 phase 2: the six frozen keys (atlas frames 0, 3, 8, 10, 12, 15) and ten in-betweens at a "
                       "100 px idle height, rasterized from pose rules by scripts/author_juice01.py; 16 frames; layer "
                       "body only"),
            "generator": "Spec and keys: Claude Opus. In-betweens and execution: Claude Sonnet. Silhouette gate on key 0: passed after fixes. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "juicer": {
            "status": "phase 2: 16 frames (six frozen keys, ten in-betweens); not wired into any Bevy binary",
            "spec": "docs/02-art/juice-01-spec.md",
            "opaque_height_idle": H,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "playback": PLAYBACK,
            "key_atlas_index": {str(a): a for a, _t, _p, _k, _au in KEYS},
            "key_fixture": "tests/fixtures/juice-01_keys.json",
            "frames": [{"index": i, "tag": tag, "ticks": t, "duration_ms": duration_ms(t), "author": au,
                        "atlas_index": a} for i, (a, tag, _p, t, au) in enumerate(FRAMES)],
        },
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/juice-01/key0-scale.png") -> Path:
    """Scale evidence: key 0 at 1x on lit Floor 1 next to Carl and the TROG-01 Pygmy (640 x 360, no scaling), plus a
    4x copy of key 0 on a side panel. Needs the Floor 1 kit atlases (``bash scripts/export_assets.sh``)."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    pygmy = grid_images(load_grid_file(trog.GRID))[0]
    juicer = grid_images(load_grid_file(GRID))[0]
    p_img, p_piv = flip(pygmy, tuple(trog.PIVOT))
    j_img, j_piv = flip(juicer, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (p_img, p_piv, CARL_X + 70, 0), (j_img, j_piv, CARL_X + 190, 0)]
    scene = Floor().render(entries, 1)
    crop = juicer.crop(juicer.getbbox())
    big = crop.resize((crop.width * 4, crop.height * 4), Image.NEAREST)
    sheet = Image.new("RGB", (scene.width + big.width + 16, max(scene.height, big.height + 16)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    sheet.paste(big, (scene.width + 8, 8), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def write_keys_x3(out: Path = ROOT / "docs/evidence/juice-01/keys-x3.png") -> Path:
    """The six keys in atlas order (0, 3, 8, 10, 12, 15), three per row on lit Floor 1 brick at 1x, cropped around
    the actors and enlarged 3x (nearest neighbour). Run: ``python3 scripts/author_juice01.py keys-x3``."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor

    imgs = [grid_images(load_grid_file(GRID))[a] for a, *_rest in KEYS]
    strips = []
    for row in (imgs[0:3], imgs[3:6]):
        entries = [(img, tuple(PIVOT), CARL_X - 40 + i * 175, 0) for i, img in enumerate(row)]
        floor = Floor()
        lit = floor.render(entries, 1)
        ground = max(floor.ground(x) for _i, _p, x, _l in entries)
        strips.append(lit.crop((0, max(0, ground - 150), lit.width, ground + 6)))
    sheet = Image.new("RGB", (strips[0].width, sum(s.height for s in strips)))
    y = 0
    for s in strips:
        sheet.paste(s, (0, y)); y += s.height
    big = sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST)
    out.parent.mkdir(parents=True, exist_ok=True)
    big.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale", "keys-x3"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    if args.command == "keys-x3":
        print(f"wrote {write_keys_x3().relative_to(ROOT)}")
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
