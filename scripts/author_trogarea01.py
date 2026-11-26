#!/usr/bin/env python3
"""Authoring record for AREA-TROG-01: the dressing of the two Floor 01 troglodyte rooms ``trog_run`` (the
troglodyte territory, T0..T3) and ``trog_strength`` (the Test of Strength arena and its bench stack).

Draws the frozen design keys of ``docs/02-art/area-trog-01-spec.md`` on ``floor01-catacomb-v2`` (no new
colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/trog_fist_hole.grid.json``   ``trog_fist_hole`` (hero key, spec section 6)
- ``assets/source/props/floor01/trog_bell.grid.json``        ``trog_bell`` (spec section 6)
- ``assets/source/props/floor01/trog_wall_marks.grid.json``  ``trog_knuckle_marks``, ``trog_slam_crack``
- ``assets/source/props/floor01/trog_crater.grid.json``      ``trog_impact_crater`` (floor decal)
- ``assets/source/props/floor01/trog_threshold.grid.json``   ``trog_threshold`` (stone-to-tile strip)
- ``assets/source/props/floor01/trog_mirror_shards.grid.json`` ``trog_mirror_shards`` (spec section 6)
- ``assets/source/props/floor01/trog_bench.grid.json``       ``trog_bench_{intact,weakened,broken}`` (spec 7)
- ``assets/source/props/floor01/trog_bench_debris.grid.json`` ``trog_bench_debris_0..5`` (spec section 8)

The keys are frozen: execution adds the remaining pieces of spec section 5 as new functions and new
atlases, and must not redraw a key (a key change is a design change). Every pixel comes from rules (shapes,
ramps, a hash noise) or from committed kit pixels (the upended bench is ``gym_bench`` turned on its end, the
shards keep ``gym_mirror_bay``'s frame); nothing is traced or resampled from a concept image. No text, logos or likenesses.

    python3 scripts/author_trogarea01.py            # grids + .ase of the keys, key pins
    python3 scripts/author_trogarea01.py keys       # the key pins only
    python3 scripts/author_trogarea01.py measure    # the key measurements (gate/measurements.txt)

The design gate stills are scratch composites (the two rooms are not dressed in code yet): the executor
replaces them with runtime captures, spec section 12.
"""
from __future__ import annotations

import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402  (kit palette ramps and the grid writer)
import author_gym01 as gym  # noqa: E402  (AREA-GYM-01: the bench the stack is built from)

RAMPS = kit.RAMPS
SRC = ROOT / "assets/source/props/floor01"


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("trog01",) + key)).encode()).digest()
    return h[0] / 255.0


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


class Pix:
    """A piece canvas: (ramp, index) per pixel, ``None`` transparent."""

    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.px = [[None] * w for _ in range(h)]

    def put(self, x, y, ramp, v):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = (ramp, int(clamp(round(v), 0, len(RAMPS[ramp]) - 1)))

    def gput(self, x, y, ramp, f, seed):
        """A ramp value rounded to the nearest step with a sparse grain (no screen-wide dither)."""
        n = rnd("grain", seed, x, y)
        self.put(x, y, ramp, round(f) + (1 if n > 0.93 else (-1 if n < 0.07 else 0)))

    def get(self, x, y):
        return self.px[y][x] if 0 <= x < self.w and 0 <= y < self.h else None

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = None

    def box(self, x0, y0, x1, y1, ramp, v):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, ramp, v)

    def line(self, x0, y0, x1, y1, ramp, v):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            self.put(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n), ramp, v)

    def disc(self, cx, cy, r, ramp, v, r_in=-1.0):
        for y in range(int(cy - r - 1), int(cy + r + 2)):
            for x in range(int(cx - r - 1), int(cx + r + 2)):
                d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
                if r_in < d <= r:
                    self.put(x, y, ramp, v(x, y, d) if callable(v) else v)


def joint(wx: int, wy: int) -> bool:
    """The catacomb back wall's running bond (16 x 8 bricks) at a world pixel: bed or head joint."""
    return wy % 8 == 0 or wx % 16 == 8 * ((wy // 8) % 2)


def stair_crack(c: Pix, ox: int, oy: int, x: int, y: int, dx: int, dy: int, n: int, seed) -> None:
    """A stepped crack that follows the bond from (x, y) outward: along a bed joint to the next head joint,
    then along the head joint to the next bed joint, ``n`` steps; a crack through a brick (one pixel off the
    joint) now and then. ``ox, oy`` is the piece's world origin (the bond is world-anchored)."""
    for k in range(n):
        for _ in range(16):                                   # along the bed joint (horizontal run)
            if not (0 <= x < c.w and 0 <= y < c.h):
                return
            c.put(x, y, "mortar", 0)
            x += dx
            wx, wy = ox + x, oy + y
            if wx % 16 == 8 * ((wy // 8) % 2) or (wx % 16 == 8 * (((wy + dy) // 8) % 2)):
                break
        for _ in range(9):                                    # along the head joint (vertical run)
            if not (0 <= x < c.w and 0 <= y < c.h):
                return
            c.put(x, y, "mortar", 0)
            y += dy
            if (oy + y) % 8 == 0:
                break
        if rnd("crk", seed, k) > 0.7:                        # a crack through a brick face
            c.put(x + dx, y, "mortar", 1)


# =============================================================================================
# Hero key trog_fist_hole (spec section 6): 64 x 80, backwall_decal, top-left (1312, 440) in trog_run's
# bricked bay 4 (fill 1184..1391). A fist-sized breach punched through the infill from the far side: four
# knuckle lobes on top, the fist body, the thumb across the bottom; inside, a black void; the sill and the
# right jamb show the wall's thickness lit from the upper left; the outer lip crushed (dark upper left, lit
# lower right); stepped cracks along the bond; a brick pushed out of its course and a half-brick socket
# below; dust streaks under the sill. The void stays above the Basher's head (sprite top world 493).
# =============================================================================================
HOLE_W, HOLE_H = 64, 80
HOLE_AT = (1312, 440)
KNUCKLES = ((16.5, 14.5, 6.0), (27.5, 12.0, 6.3), (38.5, 12.5, 6.3), (49.0, 15.5, 5.6))
VALLEYS = (22.0, 33.0, 44.0)            # the V notches between the knuckles (the wall left standing)
HOLE_BODY = (11, 14, 53, 47)            # x0, y0, x1, y1 of the fist body (rounded lower corners)
THUMB = (12.0, 35.0, 7.0, 6.5)          # the thumb bulging out of the left side: centre x, y, rx, ry


def _fist_mask() -> set:
    m = set()
    x0, y0, x1, y1 = HOLE_BODY
    for y in range(HOLE_H):
        for x in range(HOLE_W):
            px, py = x + 0.5, y + 0.5
            inside = any(math.hypot(px - cx, py - cy) <= r for cx, cy, r in KNUCKLES)
            if x0 <= x <= x1 and y0 <= y <= y1:
                cl, cr = (x - x0, y1 - y), (x1 - x, y1 - y)          # the rounded lower corners (r 5)
                inside = inside or not ((cl[0] < 5 and cl[1] < 5 and math.hypot(5 - cl[0], 5 - cl[1]) > 5.2)
                                        or (cr[0] < 5 and cr[1] < 5 and math.hypot(5 - cr[0], 5 - cr[1]) > 5.2))
            tx, ty, rx, ry = THUMB
            if ((px - tx) / rx) ** 2 + ((py - ty) / ry) ** 2 <= 1.0:
                inside = True
            if inside:
                m.add((x, y))
    for vx in VALLEYS:                                            # the finger webs: a 2 px wall sliver down
        for y in range(0, 18):                                    # from each knuckle valley, 1 px below row 15
            for x in ((int(vx) - 1, int(vx)) if y < 15 else (int(vx),)):
                m.discard((x, y))
    for (x, y) in ((6, 29), (7, 29), (6, 41), (7, 41), (8, 41)):  # the creases where the thumb meets the fist
        m.discard((x, y))
    # +-1 px ragged edge: knock out or add boundary pixels by the hash (never inside a knuckle notch)
    edge = [(x, y) for y in range(HOLE_H) for x in range(HOLE_W)
            if ((x, y) in m) != ((x + 1, y) in m) or ((x, y) in m) != ((x, y + 1) in m)]
    for (x, y) in edge:
        n = rnd("rag", x, y)
        if y > 16 and n > 0.84:
            m.discard((x, y))
        elif y > 16 and n < 0.12:
            m.add((x, y))
    return m


def trog_fist_hole() -> Pix:
    c = Pix(HOLE_W, HOLE_H)
    ox, oy = HOLE_AT
    m = _fist_mask()
    inside = lambda x, y: (x, y) in m                                   # noqa: E731
    near = {}
    for y in range(HOLE_H):
        for x in range(HOLE_W):
            if inside(x, y):
                continue
            d = min((math.hypot(x - a, y - b) for (a, b) in m if abs(x - a) < 7 and abs(y - b) < 7), default=99)
            near[(x, y)] = d
    # spall zone around the breach: the weathered brick face knocked off, fresh fired clay showing
    for (x, y), d in near.items():
        if d <= 5.5 and not joint(ox + x, oy + y):
            n = rnd("spall", x // 2, y // 2)
            if n < 0.55 - d * 0.08:
                lit = (x - 32) + (y - 28) > 0                           # lower right faces the light
                c.put(x, y, "brick", 7 if lit and n < 0.25 else (6 if lit else 4))
    # the crushed outer lip
    for (x, y), d in near.items():
        if d <= 1.5:
            lower_right = (x - 32) * 0.6 + (y - 26) > 4
            upper_left = (x - 32) * 0.6 + (y - 26) < -6
            c.put(x, y, "brick", (7 if rnd("lit", x, y) > 0.5 else 6) if lower_right else (3 if upper_left else 5))
        elif d <= 2.5 and rnd("lip", x, y) > 0.5:
            c.put(x, y, "brick", 2 if (y < 26) else 6)
    # the void and the reveal
    for (x, y) in m:
        below = sum(1 for k in range(1, 4) if not inside(x, y + k))
        right = sum(1 for k in range(1, 3) if not inside(x + k, y))
        top = not inside(x, y - 1)
        left = not inside(x - 1, y)
        if below >= 2:                                              # the sill: the wall's thickness, lit
            c.put(x, y, "brick", 6 if below == 3 else 5)
        elif below == 1:
            c.put(x, y, "brick", 3)
        elif right:                                                 # the right jamb
            c.put(x, y, "brick", 4 if right == 2 else 3)
        elif top or left:                                           # the head and left jamb in shadow
            c.put(x, y, "brick", 2)
        else:
            n = rnd("void", x, y)
            if n > 0.93 and y > 26:
                c.put(x, y, "gal", 1)                               # a draught of the dark beyond
            else:
                c.put(x, y, "mortar", 1 if n > 0.8 else 0)
    # stepped cracks along the bond from the lip, and a short crack up from each knuckle valley
    starts = ((4, 26, -1, -1), (6, 44, -1, 1), (57, 20, 1, -1), (57, 40, 1, 1), (22, 51, -1, 1),
              (56, 50, 1, 1), (10, 10, -1, -1))
    for k, (x, y, dx, dy) in enumerate(starts):
        stair_crack(c, ox, oy, x, y, dx, dy, 1 + int(rnd("cn", k) * 2), k)
    for vx in VALLEYS:                                              # a short crack up from each knuckle notch
        top = min(y for (x, y) in m if abs(x + 0.5 - vx) < 4)
        for k in range(1, 3 + int(rnd("vc", vx) * 3)):
            c.put(int(vx) + (1 if k > 2 and vx > 30 else 0), top - 1 - k, "mortar", 0)
    # a brick pushed out of its course below the fist (world 1345..1359 x 497..503: drawn 1 px low, a dark
    # gap above it), and the empty socket of a half-brick (world 1361..1367 x 497..503)
    for x in range(33, 48):
        c.put(x, 57, "mortar", 0)
        for y in range(58, 65):
            c.put(x, y, "brick", 6 if y == 58 else (2 if y == 64 else (4 if x > 34 else 5)))
    for x in range(49, 56):
        for y in range(57, 64):
            c.put(x, y, "mortar", 1 if (y == 57 or x == 49) else 0)
    c.put(55, 63, "brick", 6)
    # dust from the breach: short streaks under the sill, densest at the top
    for k, sx in enumerate((15, 21, 28, 41, 52)):
        y0 = max(y for (x, y) in m if x == sx) + 2 if any(x == sx for (x, y) in m) else 50
        ln = 4 + int(rnd("dl", k) * 9)
        for y in range(y0, min(HOLE_H, y0 + ln)):
            if rnd("dd", k, y) > 0.2 + (y - y0) / ln * 0.7 and c.get(sx, y) is None:
                c.put(sx, y, "stone", 6 if y < y0 + 3 else 5)
    return c


# =============================================================================================
# trog_bell (spec section 6): 40 x 64, prop layer, top-left (1800, 420) on the T3 brick run (1792..1983),
# left of the torch at 1880 and above the left end of the bell step (1840..1904). An old cast bell on a
# wrought bracket: a riveted wall plate, an arm with a strut, the yoke, a tarnished brass bell with verdigris
# runs and a dented lip, the clapper, a frayed lanyard hanging to the bottom row (world 483). Decor only.
# =============================================================================================
BELL_W, BELL_H = 40, 64
BELL_AT = (1800, 420)
BELL_CX = 26.0


def trog_bell() -> Pix:
    c = Pix(BELL_W, BELL_H)
    # the wall plate with four rivets
    for y in range(2, 18):
        for x in range(0, 8):
            c.put(x, y, "iron", 3 if x not in (0, 7) and y not in (2, 17) else (4 if x == 0 or y == 2 else 1))
    for (x, y) in ((1, 3), (6, 3), (1, 16), (6, 16)):
        c.put(x, y, "iron", 6)
    # the arm: a flat bar from the plate to the yoke, a diagonal strut under it, a curled tip
    for x in range(8, 31):
        c.put(x, 6, "iron", 5 if x < 20 else 4)
        c.put(x, 7, "iron", 2)
    c.put(31, 6, "iron", 3)
    c.put(32, 5, "iron", 3)
    c.put(32, 4, "iron", 4)
    c.put(31, 3, "iron", 3)
    for i in range(13):
        x, y = 7 + i, 15 - round(i * 7 / 12)
        c.put(x, y, "iron", 4)
        c.put(x, y + 1, "iron", 1)
    # the yoke and the crown loop
    c.box(24, 8, 28, 9, "iron", 3)
    c.put(24, 8, "iron", 5)
    for (x, y) in ((24, 10), (28, 10), (24, 11), (28, 11), (25, 12), (26, 12), (27, 12)):
        c.put(x, y, "iron", 2)
    c.put(26, 10, "iron", 1)
    # the bell body: crown rows 13..14, a waisted body flaring to the lip (rows 35..38)
    for y in range(13, 39):
        t = (y - 13) / 25
        hw = 4.0 + 3.2 * t + 4.8 * t ** 4
        if y >= 36:
            hw = 12.0
        x0, x1 = round(BELL_CX - hw), round(BELL_CX + hw) - 1
        for x in range(x0, x1 + 1):
            u = (x - x0) / max(1, x1 - x0)                          # 0 left .. 1 right
            f = 2.5 + 1.3 * math.cos((u - 0.22) * math.pi * 1.15) - 0.5 * t
            if u > 0.88:
                f = 1.2
            if y == 13:
                f = 3.6 if u < 0.6 else 2.2
            if y in (35, 36):                                       # the sound bow: a raised ring
                f += 0.9
            if y == 37:
                f = 3.8 if u < 0.7 else 2.2
            if y == 38:
                f = 1.6
            if abs(u - 0.22) < 0.07 and 15 < y < 34:
                f = 4.0 if rnd("hl", y) > 0.25 else 3.0             # the highlight band, broken by tarnish
            c.put(x, y, "brass", clamp(f - 1.5, 0, 3))                # tarnished: under new brass
    for x in range(16, 37):                                         # the dark mouth seen from just below
        c.put(x, 38, "iron", 0)
    for (x, y) in ((34, 37), (35, 37), (35, 36)):                   # a dent knocked into the lip
        c.put(x, y, "brass", 0)
    c.put(33, 37, "brass", 3)
    # verdigris runs from the crown and the shoulder (thin, broken), tarnish on the right
    for k, sx in enumerate((23, 27, 30, 33)):
        y0 = 14 + int(rnd("vg", k) * 10)
        for y in range(y0, min(36, y0 + 3 + int(rnd("vl", k) * 9))):
            if c.get(sx, y) and c.get(sx, y)[0] == "brass" and rnd("vd", k, y) > 0.45:
                c.put(sx + (1 if y > y0 + 5 else 0), y, "lichen", 1)
    # the clapper below the mouth, then the frayed lanyard to the bottom row
    c.box(25, 39, 27, 41, "iron", 3)
    c.put(25, 39, "iron", 5)
    c.put(27, 41, "iron", 1)
    for y in range(42, 60):
        x = 26 - (1 if y > 52 else 0)
        c.put(x, y, "wood", 4 if (y // 2) % 2 else 3)
    for (x, y, v) in ((25, 60, 4), (24, 61, 3), (24, 62, 4), (23, 63, 3), (26, 60, 5), (26, 61, 4), (26, 62, 3),
                      (25, 61, 3), (27, 63, 4)):
        c.put(x, y, "wood", v)
    return c


# =============================================================================================
# trog_knuckle_marks (spec section 6): 96 x 40, backwall_decal, top-left (272, 488) on bricked bay 1, right
# of the entry door (216..263) and left of the pier (382). Punches at troglodyte chest height: rows of four
# knuckle dents (dark socket, crushed upper-left rim, pale lip lower right), older ones faded to bruises;
# knuckle drags (three parallel scrapes, sloping down). Irregular, never in letter-like rows.
# trog_slam_crack: 48 x 48, backwall_decal, top-left (296, 400): where something heavy was slammed into the
# infill above the marks (the T0 "slam" beat, TROG-Q7): a crushed bowl, bricks pushed in, radial cracks
# along the bond and across faces, a dust fan below.
# =============================================================================================
MARKS_W, MARKS_H = 96, 40
MARKS_AT = (272, 488)
PUNCHES = (  # (x, y, angle degrees, fresh)
    (8, 10, -8, True), (24, 18, 4, True), (30, 6, -14, False), (46, 22, 10, True), (55, 9, -4, True),
    (70, 26, 16, False), (76, 13, -10, True), (62, 31, 2, False))
DRAGS = ((14, 27, 22, 6), (66, 3, 20, 9), (84, 22, 12, 6))   # (x, y, length, drop)
SLAM_W, SLAM_H = 48, 48
SLAM_AT = (296, 400)


def _dent(c: Pix, x: float, y: float, fresh: bool) -> None:
    """One knuckle dent: 3 x 3 socket, upper-left rim dark, lower-right lip pale."""
    xi, yi = round(x), round(y)
    if fresh:
        c.box(xi, yi, xi + 1, yi + 1, "mortar", 0)
        c.put(xi + 2, yi, "mortar", 1)
        c.put(xi, yi + 2, "mortar", 1)
        c.put(xi - 1, yi, "brick", 2)
        c.put(xi, yi - 1, "brick", 2)
        c.put(xi + 2, yi + 1, "stone", 6)
        c.put(xi + 1, yi + 2, "stone", 6)
        c.put(xi + 2, yi + 2, "stone", 5)
    else:
        c.box(xi, yi, xi + 1, yi + 1, "brick", 2)
        c.put(xi + 2, yi + 1, "brick", 6)


def trog_knuckle_marks() -> Pix:
    c = Pix(MARKS_W, MARKS_H)
    for k, (x, y, ang, fresh) in enumerate(PUNCHES):
        a = math.radians(ang)
        for j in range(4):                                         # four knuckles 4 px apart along the fist
            jx = x + j * 4 * math.cos(a)
            jy = y + j * 4 * math.sin(a) + (-1 if j in (1, 2) else (1 if j == 3 else 0))
            _dent(c, jx, jy, fresh and not (j == 3 and rnd("pinky", k) > 0.5))
        if fresh and rnd("hc", k) > 0.5:                            # a short crushed chip off the middle knuckle
            sx, sy = round(x + 5 * math.cos(a)), round(y + 5 * math.sin(a))
            c.put(sx + 1, sy - 2, "brick", 3)
            c.put(sx + 2, sy - 2, "brick", 2)
    for k, (x, y, ln, drop) in enumerate(DRAGS):                    # knuckle drags: three parallel scrapes
        for s in range(3):
            for i in range(ln - s * 2):
                xx = x + i
                yy = y + s * 3 + round(i * drop / ln)
                c.put(xx, yy, "brick", 2 if i < ln * 0.6 else 3)       # the groove, shallower as it trails off
                if i < ln * 0.75:
                    c.put(xx, yy + 1, "stone", 5 if i < ln // 2 else 4)  # its pale lower lip
    return c


def trog_slam_crack() -> Pix:
    c = Pix(SLAM_W, SLAM_H)
    ox, oy = SLAM_AT
    cx, cy = 23.5, 21.5
    # dust fan first (the bowl and cracks draw over it)
    for y in range(SLAM_H):
        for x in range(SLAM_W):
            dx, dy = x + 0.5 - cx, y + 0.5 - cy
            d = math.hypot(dx * 0.8, dy)
            if 9 < d < 22 and dy > -4 and rnd("dust", x, y) > 0.72 + 0.012 * d - (0.1 if dy > 8 else 0):
                c.put(x, y, "stone", 6 if d < 14 else 5)
    # bricks pushed in around the bowl: a 1 px dark gap on their outer edges, faces a step darker
    for y in range(SLAM_H):
        for x in range(SLAM_W):
            d = math.hypot((x + 0.5 - cx) * 0.85, y + 0.5 - cy)
            if 7 < d < 12.5 and not joint(ox + x, oy + y):
                c.put(x, y, "brick", 3 if (x + 0.5 - cx) + (y + 0.5 - cy) < 0 else 4)
            elif 7 < d < 13.5 and joint(ox + x, oy + y):
                c.put(x, y, "mortar", 0)
    # the crushed bowl: rubble chips over a dark core, the lower-right inner face lit
    for y in range(SLAM_H):
        for x in range(SLAM_W):
            dx, dy = x + 0.5 - cx, y + 0.5 - cy
            d = math.hypot(dx * 0.85, dy)
            if d <= 7.5 + (rnd("bowl", x, y) - 0.5) * 1.6:
                n = rnd("chip", x // 2, y // 2)
                if d < 3.2:
                    c.put(x, y, "mortar", 0)
                elif dx + dy > 4:
                    c.put(x, y, "brick", 7 if n > 0.5 else 6)
                elif n > 0.66:
                    c.put(x, y, "brick", 4)
                else:
                    c.put(x, y, "mortar", 1 if n > 0.3 else 0)
    # radial cracks across brick faces and along the bond
    for k in range(9):
        a = math.radians(k * 40 + 25 * rnd("ra", k))
        ln = 12 + 11 * rnd("rl", k)
        for i in range(8, int(ln)):
            jit = (rnd("rj", k, i // 3) - 0.5) * 0.35
            x = round(cx + math.cos(a + jit) * i)
            y = round(cy + math.sin(a + jit) * i)
            c.put(x, y, "mortar", 0)
            if i % 4 == 0 and rnd("rb", k, i) > 0.5:
                c.put(x + 1, y, "mortar", 1)
    for k, (x, y, dx, dy) in enumerate(((10, 34, -1, 1), (37, 34, 1, 1))):
        stair_crack(c, ox, oy, x, y, dx, dy, 1, 50 + k)
    return c


# =============================================================================================
# trog_impact_crater (spec section 6): 72 x 20, structure layer (a floor decal), top-left (560, 560) in
# trog_run's bay-2 floor (plinth-free 544..719), where the Pygmy lands. A shallow depression in the deck
# flags: the far inner slope faces the viewer and catches the light, the near slope falls into shadow, a
# broken raised rim, flag fragments in the bottom, cracks out across the flags.
# =============================================================================================
CRATER_W, CRATER_H = 72, 20
CRATER_AT = (560, 560)
CRATER_C = (36.0, 8.5, 26.0, 6.0)      # centre x, y, radius x, radius y


def trog_impact_crater() -> Pix:
    c = Pix(CRATER_W, CRATER_H)
    cx, cy, rx, ry = CRATER_C
    # cracks first (the bowl draws over their roots)
    for k in range(8):
        a = math.radians(k * 45 + 20 * rnd("ca", k))
        ln = 1.05 + 0.4 * rnd("cl", k)
        for i in range(60):
            t = i / 59 * ln
            jit = (rnd("cj", k, i // 6) - 0.5) * 0.3
            x = round(cx + math.cos(a + jit) * rx * t)
            y = round(cy + math.sin(a + jit) * ry * t)
            if t > 0.8 and 0 <= y < CRATER_H:
                c.put(x, y, "mortar", 2 if t < 1.2 else 3)
    for y in range(CRATER_H):
        for x in range(CRATER_W):
            dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            e = math.hypot(dx, dy) + (rnd("rim", x // 3, y) - 0.5) * 0.12
            if e > 1.12:
                continue
            if e > 0.9:                                             # the broken raised rim
                if dy > 0.2:
                    c.put(x, y, "floor", 7 if rnd("rl", x) > 0.4 else 6)   # near rim: its top catches light
                elif dy < -0.2:
                    c.put(x, y, "mortar", 2)                        # far rim: the lip's shadow line
                else:
                    c.put(x, y, "floor", 4)
            elif dy < -0.05:                                        # the far inner slope faces the viewer
                c.put(x, y, "floor", 6 if e > 0.6 else 5)
            elif dy > 0.4:                                          # the near inner slope: in shadow
                c.put(x, y, "floor", 1 if e > 0.65 else 2)
            else:                                                   # the bottom: crushed flag, grit
                n = rnd("bot", x // 2, y)
                c.put(x, y, "floor", 3 if n > 0.35 else (5 if n > 0.15 else 2))
    for k in range(7):                                              # broken flag fragments tilted in the bowl
        fx = round(cx - 16 + 32 * rnd("fx", k))
        fy = round(cy - 1 + 3 * rnd("fy", k))
        w = 2 + int(rnd("fw", k) * 3)
        for i in range(w):
            c.put(fx + i, fy, "floor", 6)
            c.put(fx + i, fy + 1, "floor", 2)
    return c


# =============================================================================================
# trog_threshold (spec section 6): 80 x 19, structure layer, top-left (672, 561) in trog_strength: where the
# catacomb flags stop and the gym's ceramic tile starts, at the bench's right face (x 672). An L: local cols
# 0..19 over the full depth (world 672..691, the plinth-free floor right of the bench), cols 20..79 only on
# rows 11..18 (world 572..579, in front of the bay-3 plinth 692..751). An iron nosing strip, then cold tile
# in foreshortened courses (rows 0..4, 6..10, 12..18; grout rows 5 and 11), cracked and missing tiles thick
# near the bench, fewer to the right.
# =============================================================================================
THR_W, THR_H = 80, 19
THR_AT = (672, 561)
THR_NOTCH = (20, 11)                    # cols >= 20 start at row 11
TILE_W = (9, 10, 12)                    # tile widths per course (nearer courses read wider)
COURSES = ((0, 4), (6, 10), (12, 18))


def _thr_in(x: int, y: int) -> bool:
    return x < THR_NOTCH[0] or y >= THR_NOTCH[1]


def trog_threshold() -> Pix:
    c = Pix(THR_W, THR_H)
    for y in range(THR_H):
        for x in range(THR_W):
            if not _thr_in(x, y):
                continue
            if y in (5, 11):
                c.put(x, y, "mortar", 1)                            # grout between courses
                continue
            k = 0 if y <= 4 else (1 if y <= 10 else 2)
            top, bot = COURSES[k]
            tw = TILE_W[k]
            sx = x + (k * tw // 2)                                  # the head joints stagger by half a tile
            if sx % tw == 0:
                c.put(x, y, "mortar", 1)
                continue
            tile = (sx // tw, k)
            v = 3 if y == top else (1 if y == bot else 2)
            if y == top and sx % tw in (1, 2):
                v = 4                                               # the glaze glint at a tile's far-left corner
            if rnd("dirty", *tile) > 0.65 and y != top:
                v -= 1
            if rnd("grime", x, y) > 0.9:
                c.put(x, y, "mortar", 3)                            # grime ground into the glaze
                continue
            c.put(x, y, "stone_cold", clamp(v, 0, 4))
            # broken or missing tiles: dense near the bench (x small), thinning to the right
            p = 0.75 - x / 110.0
            r = rnd("break", *tile)
            if r < p * 0.45:                                        # missing: the mortar bed, chipped
                c.put(x, y, "mortar", 2 if y != top else 1)
                if rnd("crumb", x, y) > 0.75:
                    c.put(x, y, "mortar", 4)
            elif r < p * 0.9 and (sx * 3 + y * 5) % 11 == 0:
                c.put(x, y, "stone_cold", 0)                        # a crack across the glaze
    # the iron nosing: the edge strip the tile was laid to, along the bench's right face
    for y in range(THR_H):
        c.put(0, y, "iron", 2)
        c.put(1, y, "iron", 4 if y % 6 else 5)
        c.put(2, y, "iron", 1)
    for y in (3, 9, 15):
        c.put(1, y, "iron", 6)                                      # the screw heads
    return c


# =============================================================================================
# The bench stack (spec section 7): trog_bench_{intact,weakened,broken}, 32 x 80 each, origin = the sim's
# target rect (640, 500); the broken frame draws only the remnant on rows 64..79 (world {640, 564, 32, 16}).
# The gameplay rect is the drawn rect: intact and weakened fill rows 0..79 and cols 0..31; the remnant fills
# rows 64..79 and cols 0..31. Built from AREA-GYM-01: a flat pad laid across the top (the walkable arris on
# row 0), a gym_bench turned on its end (its pixels rotated, legs out to the right), a face-on plate hung on a
# leg, an edge-on stack of plates under it, all lashed with one ratchet strap; the gaps are filled with the
# stack's own shadow so the block reads solid. Presentation only (keyed off the sim's partition state).
# =============================================================================================
BENCH_W, BENCH_H = 32, 80
BENCH_AT = (640, 500)
REMNANT_ROW = 64
BENCH_STATES = ("intact", "weakened", "broken")
PLATES = ((48, 4, "iron"), (52, 5, "rubber"), (57, 4, "iron"), (61, 5, "iron"), (66, 4, "rubber"),
          (70, 5, "iron"), (75, 5, "iron"))   # (top row, thickness, kind): rows 48..79


def _pad_cap(c: Pix, torn: bool) -> None:
    for x in range(BENCH_W):
        c.put(x, 0, "brick", 9)                                     # the walkable arris (row 0, full width)
        c.put(x, 1, "brick", 7)
        for y in (2, 3, 4):
            c.put(x, y, "brick", 5 if x not in (0, 31) else 4)
        c.put(x, 5, "brick", 2)
    for x in range(2, 30, 3):
        c.put(x, 3, "brick", 4)                                     # the stitched seam
    c.box(19, 2, 24, 3, "stone", 6)                                 # a tape patch (the gym bench's language)
    c.put(19, 2, "stone", 7)
    if torn:                                                        # a gash with the foam showing
        for x in range(5, 15):
            c.put(x, 2 + (1 if 7 < x < 12 else 0), "stone", 8 if x % 3 else 9)
            if 6 < x < 13:
                c.put(x, 3 + (1 if 7 < x < 12 else 0), "stone", 7)
        c.put(4, 2, "brick", 3)
        c.put(15, 3, "brick", 3)


def _upended(c: Pix, torn: bool) -> None:
    """gym_bench (44 x 18) turned on its end: (x, y) -> (y, 43 - x); the pad on the left, legs to the right.
    Placed at cols 0..17, rows 6..49 (the rotated foot rows run past the plates' top and are covered)."""
    b = gym.bench()
    for y in range(b.h):
        for x in range(b.w):
            p = b.px[y][x]
            if p is not None:
                c.px[6 + (43 - x)][y] = p if (6 + 43 - x) < 48 else c.px[6 + 43 - x][y]
    if torn:                                                        # the upright pad split down its face
        for y in range(14, 30):
            x = 4 + (1 if y > 21 else 0)
            c.put(x, y, "stone", 8 if y % 4 else 9)
            c.put(x + 1, y, "stone", 7 if y % 3 else 8)
        c.put(4, 13, "brick", 2)


def _shadow_fill(c: Pix) -> None:
    """Every hole left inside the block (between the legs, behind the plate) is the stack's own shadow."""
    for y in range(6, 48):
        for x in range(BENCH_W):
            if c.get(x, y) is None:
                c.put(x, y, "iron", 1 if rnd("shade", x, y) > 0.86 else 0)


def _plates(c: Pix, gap: int | None) -> None:
    """Edge-on plates lying flat, each a slab with rounded ends and a hub collar standing proud at its
    centre; widths vary (the stack's shadow fills the ends, so the block's outline stays the full rect)."""
    widths = (32, 28, 30, 26, 32, 29, 32)
    centres = (15.5, 15.0, 16.5, 14.5, 15.5, 16.0, 15.5)
    for y in range(PLATES[0][0], BENCH_H):
        for x in range(BENCH_W):
            c.put(x, y, "iron", 0)
    for k, (top, th, kind) in enumerate(PLATES):
        w, cx = widths[k], centres[k]
        x0, x1 = round(cx - w / 2), round(cx + w / 2) - 1
        for y in range(top, top + th):
            ly = y - top
            for x in range(max(0, x0), min(BENCH_W - 1, x1) + 1):
                if (x in (x0, x1)) and ly in (0, th - 1):
                    continue                                        # rounded ends
                hub = abs(x + 0.5 - cx) < 3
                if kind == "iron":
                    v = 5 if ly == 0 else (1 if ly == th - 1 else (4 if hub else 3))
                    if x == x0:
                        v = min(v + 1, 5)                           # the lit left end
                    c.put(x, y, "iron", v)
                else:                                               # a rubber bumper: black, a blue sheen
                    if ly == 0:
                        c.put(x, y, "gal", 5 if x < cx - 4 else 3)
                    else:
                        c.put(x, y, "iron", 2 if (hub and ly == 1) else (1 if ly < th - 1 else 0))
        if gap == k:                                                # a plate knocked half out: a dark slot
            for x in range(4, 28):
                c.put(x, top, "mortar", 0)
                c.put(x, top + 1, "mortar", 0 if x > 8 else 1)


def _disc(c: Pix, cx: float, cy: float) -> None:
    r = 9.0
    c.disc(cx, cy, r, "iron", lambda x, y, d: (5 if y < cy - 2 else 4 if y < cy else 3) if d > r - 1.5 else
           (2 if d > 3.2 else (1 if d > 1.6 else 0)))
    for (x, y) in ((round(cx - 6), round(cy - 6)), (round(cx - 5), round(cy - 7)), (round(cx - 4), round(cy - 7))):
        c.put(x, y, "iron", 6)
    for k in range(6):
        a = math.radians(k * 60 + 20)
        c.put(round(cx + math.cos(a) * 5.5), round(cy + math.sin(a) * 5.5), "iron", 3)
    c.put(round(cx - 0.5), round(cy - 0.5), "stone_cold", 4)        # the hub on its peg


STRAP = ((31, 12), (0, 72))


def _strap_pts():
    (x0, y0), (x1, y1) = STRAP
    n = y1 - y0
    return [(round(x0 + (x1 - x0) * i / n), y0 + i) for i in range(n + 1)]


def _strap(c: Pix, snapped: bool) -> None:
    """One ratchet strap, 2 px wide plus a 1 px shadow, lashed diagonally round the whole stack."""
    pts = _strap_pts()
    buckle = pts[len(pts) // 2]
    for (x, y) in pts:
        if snapped and y > buckle[1]:
            continue
        c.put(x - 1, y, "brass", 4)
        c.put(x, y, "brass", 4 if y % 4 else 3)
        c.put(x + 1, y, "brass", 2)
        c.put(x + 2, y, "iron", 0)
    bx, by = buckle
    c.box(bx - 3, by - 2, bx + 2, by + 2, "iron", 3)                # the ratchet buckle and its lever
    c.box(bx - 3, by - 2, bx + 2, by - 2, "iron", 5)
    c.put(bx - 3, by - 2, "iron", 6)
    c.put(bx + 2, by + 2, "iron", 1)
    c.box(bx - 2, by, bx + 1, by, "iron", 1)
    c.put(bx + 3, by - 3, "iron", 4)
    c.put(bx + 4, by - 4, "iron", 4)
    if snapped:                                                     # the cut end hangs straight down
        for y in range(by + 3, by + 14):
            xx = bx + (1 if y > by + 9 else 0)
            c.put(xx - 1, y, "brass", 4)
            c.put(xx, y, "brass", 2)
        c.put(bx, by + 14, "brass", 3)
        for i, y in enumerate(range(70, 74)):                       # the lower stub, slack at the left edge
            c.put(i, y, "brass", 3)
            c.put(i, y + 1, "brass", 1)
    for (x, y) in ((31, 10), (31, 11), (0, 73), (0, 74)):           # the strap wraps round the block's edges
        c.put(x, y, "brass", 2)


def trog_bench_intact() -> Pix:
    c = Pix(BENCH_W, BENCH_H)
    _upended(c, False)
    _shadow_fill(c)
    _plates(c, None)
    _disc(c, 22.0, 28.0)
    _pad_cap(c, False)
    _strap(c, False)
    c.clear(31, 46)                                                 # the one transparent nick
    c.clear(31, 47)
    return c


def trog_bench_weakened() -> Pix:
    c = Pix(BENCH_W, BENCH_H)
    _upended(c, True)
    _shadow_fill(c)
    _plates(c, 3)
    _disc(c, 20.0, 61.0)                                            # the plate slid down its leg
    _pad_cap(c, True)
    _strap(c, True)
    for (x, y) in ((27, 31), (28, 32), (28, 33), (29, 34)):        # a crack in the leg's weld
        c.put(x, y, "mortar", 0)
    c.clear(31, 46)
    c.clear(31, 47)
    return c


def trog_bench_broken() -> Pix:
    """The remnant only, rows 64..79: plates flat on the floor, the crushed pad, a bent bar, the loose
    strap, a disc half buried. Touches rows 64 and 79 and cols 0 and 31."""
    c = Pix(BENCH_W, BENCH_H)
    for k, (top, th, kind) in enumerate(((72, 4, "iron"), (76, 4, "rubber"))):
        for y in range(top, top + th):
            ly = y - top
            for x in range(BENCH_W):
                if kind == "iron":
                    c.put(x, y, "iron", 5 if ly == 0 else (1 if ly == th - 1 else 3))
                else:
                    c.put(x, y, "gal", 3) if ly == 0 else c.put(x, y, "iron", 1 if ly < th - 1 else 0)
    for y in range(69, 72):                                         # a smaller plate tipped on the pile
        for x in range(3 + (71 - y), 15):
            c.put(x, y, "iron", 5 if y == 69 else 3)
    # the crushed pad, folded on the pile, foam bursting
    for y in range(66, 72):
        for x in range(14, 27):
            if (y == 66 and x in (14, 26)) or (y == 71 and x < 16):
                continue
            c.put(x, y, "brick", 7 if y == 66 else (5 if y < 70 else 3))
    for (x, y) in ((17, 67), (18, 67), (18, 68), (21, 68), (22, 67), (23, 68)):
        c.put(x, y, "stone", 8)
    c.put(19, 66, "stone", 9)
    # the bent bar across the pile, its high end on row 64
    for (x0, y0, x1, y1) in ((1, 70, 9, 66), (9, 66, 13, 64), (13, 64, 31, 69)):
        c.line(x0, y0, x1, y1, "stone_cold", 4)
    c.put(13, 65, "stone_cold", 2)
    # the disc half buried, standing on its edge
    c.disc(27.5, 72.5, 4.5, "iron", lambda x, y, d: 4 if d > 3.2 else (2 if d > 1.4 else 0))
    c.put(25, 69, "iron", 6)
    # the loose strap
    for (x, y) in ((2, 71), (3, 71), (4, 72), (5, 72), (6, 71), (7, 71), (8, 72)):
        c.put(x, y, "brass", 3)
    for x in range(BENCH_W):                                        # dust settled on the floor row
        if rnd("rdust", x) > 0.75:
            c.put(x, 79, "stone", 5)
    return c


BENCH_FRAMES = {"intact": trog_bench_intact, "weakened": trog_bench_weakened, "broken": trog_bench_broken}


# =============================================================================================
# trog_bench_debris (spec section 8): 6 frames of 112 x 96, prop layer above the bench, origin (600, 484):
# bottom-centre local (56, 95) is world (656, 579), the bench's centre on the walking line. Played once on the
# Broken rising edge: burst, arcs, falling, bounce, settling, then the settled litter (rows 93..95, at most 3
# px tall) held while the bench stays Broken. Pieces are deterministic ballistic samples (fixed start, speed,
# gravity 0.5 px/tick^2) biased left, away from the Basher's side. Durations in 60 Hz ticks.
# =============================================================================================
DEB_W, DEB_H = 112, 96
DEB_AT = (600, 484)
DEB_TICKS = (3, 4, 4, 5, 6)           # frames 0..4; frame 5 is held
DEB_SAMPLE = (1, 5, 9, 14, 20)        # the tick each moving frame shows (the middle of its span)
DEB_GRAVITY = 0.5
DEB_KINDS = ("plate", "plate", "vinyl", "vinyl", "foam", "foam", "foam", "bar", "strap", "strap", "plate",
             "vinyl", "foam", "foam", "strap", "foam")


def _deb_pieces():
    out = []
    for k, kind in enumerate(DEB_KINDS):
        x0 = 42 + 28 * rnd("dx0", k)                                # inside the bench (local cols 40..71)
        y0 = 24 + 40 * rnd("dy0", k)
        vx = -2.6 + 3.4 * rnd("dvx", k)                             # biased left
        vy = -4.5 + 3.0 * rnd("dvy", k)
        out.append((kind, x0, y0, vx, vy, k))
    return out


def _deb_pos(x0, y0, vx, vy, t):
    """Ballistic with one damped bounce on the floor row (95) and floor friction after it."""
    x, y, ux, uy = x0, y0, vx, vy
    for _ in range(t):
        x += ux
        uy += DEB_GRAVITY
        y += uy
        if y >= 94:
            y = 94
            uy = -uy * 0.3 if abs(uy) > 1.5 else 0.0
            ux *= 0.6
    return x, y


def _deb_draw(c: Pix, kind, x, y, k, settled: bool):
    xi, yi = round(x), round(y)
    if kind == "plate":                                             # a plate fragment, edge-on, 5..6 px
        w = 5 if k % 2 else 6
        c.box(xi, yi, xi + w - 1, yi, "iron", 5)
        c.box(xi, yi + 1, xi + w - 1, yi + 1, "iron", 2)
        if not settled:
            c.put(xi + w - 1, yi, "iron", 3)
    elif kind == "vinyl":                                           # a torn vinyl scrap with foam on its edge
        c.box(xi, yi, xi + 2, yi + (0 if settled else 1), "brick", 5)
        c.put(xi, yi, "brick", 7)
        c.put(xi + 2, yi + (0 if settled else 1), "stone", 8)
    elif kind == "foam":
        c.put(xi, yi, "stone", 8 if k % 2 else 9)
        if not settled:
            c.put(xi + 1, yi, "stone", 7)
    elif kind == "bar":
        if settled:
            c.line(xi - 5, yi + 1, xi + 4, yi + 1, "stone_cold", 4)
            c.line(xi - 5, yi + 2, xi + 4, yi + 2, "stone_cold", 1)
        else:
            a = 0.9 * y / 10.0                                      # the bar tumbles
            c.line(round(x - 5 * math.cos(a)), round(y - 5 * math.sin(a)), round(x + 5 * math.cos(a)),
                   round(y + 5 * math.sin(a)), "stone_cold", 4)
    elif kind == "strap":
        c.put(xi, yi, "brass", 4)
        c.put(xi + 1, yi + (0 if settled else 1), "brass", 3)
        c.put(xi + 2, yi + (0 if settled else 1), "brass", 2)


DEB_PUFFS = ((-14, 0, 7), (-4, -3, 8), (7, 0, 6), (-22, 1, 5), (14, 1, 4))   # (dx, dy, r) from (56, 95)


def _deb_dust(c: Pix, f: int) -> None:
    """The dust cloud from the bench's foot: round puffs (lit tops) that spread left and thin out over frames
    0..3 (frame 3 is broken wisps), gone after."""
    if f > 3:
        return
    grow = (0.7, 1.1, 1.45, 1.7)[f]
    drift = (0, -3, -6, -9)[f]
    holes = (0.0, 0.08, 0.3, 0.6)[f]
    for k, (dx, dy, r) in enumerate(DEB_PUFFS):
        if f == 0 and k > 2:
            continue
        pcx, pcy, rr = 52 + dx * grow + drift, 95 + dy - r * grow * 0.55, r * grow
        for y in range(int(pcy - rr) - 1, int(pcy + rr) + 2):
            for x in range(int(pcx - rr) - 1, int(pcx + rr) + 2):
                d = math.hypot(x + 0.5 - pcx, y + 0.5 - pcy)
                if d > rr or y >= DEB_H or rnd("hole", f, x // 2, y // 2) < holes:
                    continue
                top = (y + 0.5 - pcy) < -rr * 0.45
                if not top and f > 0 and (x + y) % 2 and d < rr * 0.8:
                    continue                                        # the body thins to a 50 % screen
                c.put(x, y, "stone", 6 if top else 5)


def trog_bench_debris_frame(f: int) -> Pix:
    c = Pix(DEB_W, DEB_H)
    _deb_dust(c, f)
    for kind, x0, y0, vx, vy, k in _deb_pieces():
        if f < 5:
            x, y = _deb_pos(x0, y0, vx, vy, DEB_SAMPLE[f])
            _deb_draw(c, kind, clamp(x, 0, DEB_W - 6), min(y, 94), k, False)
        else:
            x, _ = _deb_pos(x0, y0, vx, vy, 80)
            _deb_draw(c, kind, clamp(x, 0, DEB_W - 6), 93 if kind in ("plate", "bar") else 94, k, True)
    return c


# =============================================================================================
# trog_mirror_shards (spec section 6): 96 x 74, backwall_decal, top-left (792, 470) in bricked bay 3 of
# trog_strength (the plan's "hanging mirror fragments"). The gym's own mirror bay (frame, clips, skirting,
# tape) with most of the glass gone: the dark plywood backing shows through with glue dabs, and five shards
# still hang from the top frame, the clips and the taped corner. The surviving glass is dimmed to dark
# glass (``gal`` 3..4, the sheens ``gal`` 7, the camp wreck's rule) so the inset never outshines a
# troglodyte (spec section 10); the frame stays as drawn.
# =============================================================================================
SHARDS = (                                  # polygons in mirror-local px, clockwise; the glass that is left
    ((1, 1), (46, 1), (46, 9), (31, 19), (24, 17), (12, 34), (1, 44)),      # the left panel's top-left sheet
    ((48, 1), (68, 1), (64, 22), (59, 41), (55, 26)),                       # a dagger hanging from the seam clip
    ((74, 1), (94, 1), (94, 21), (88, 30), (83, 25), (79, 12)),             # the right clip's shard
    ((78, 69), (82, 58), (89, 54), (94, 49), (94, 69)),                     # the taped corner, held by the tape
    ((14, 69), (18, 63), (26, 66), (31, 69)),                               # a shard still in the bottom channel
)


def _in_poly(x: float, y: float, poly) -> bool:
    inside = False
    for i in range(len(poly)):
        (x0, y0), (x1, y1) = poly[i], poly[(i + 1) % len(poly)]
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            inside = not inside
    return inside


def trog_mirror_shards() -> Pix:
    src = gym.mirror_bay()
    c = Pix(gym.MIRROR_W, gym.MIRROR_H)
    for y in range(c.h):
        c.px[y] = list(src.px[y])
    g0, g1 = gym.GLASS_ROWS
    glass = lambda x, y: g0 < y <= g1 and any(a <= x <= b for a, b in gym.GLASS_COLS)  # noqa: E731
    kept = lambda x, y: any(_in_poly(x + 0.5, y + 0.5, p) for p in SHARDS)          # noqa: E731
    for y in range(c.h):
        for x in range(c.w):
            if not glass(x, y) or x == gym.SEAM_COL:
                continue
            q = src.get(x, y)
            if q and q[0] == "brass":                                   # the tape stays as drawn
                continue
            if kept(x, y):
                if q and q[0] == "gal" and q[1] >= 9:                    # a sheen: the only light glass
                    c.put(x, y, "gal", 7)
                else:
                    lum = _luma(*q) if q else 0
                    c.put(x, y, "gal", 4 if lum > 95 else 3)
                edge = not (kept(x - 1, y) and kept(x + 1, y) and kept(x, y - 1) and kept(x, y + 1))
                if edge and y > g0 + 1:
                    up = not kept(x, y - 1) or not kept(x - 1, y)
                    c.put(x, y, "stone_cold", 2 if up else 0)            # the broken edge: lit above, dark below
            else:                                                       # the plywood backing and its glue dabs
                dab = any(math.hypot(x - dx, (y - dy) * 1.4) < 2.2 for dx, dy in
                          ((10, 52), (30, 48), (38, 24), (60, 54), (70, 30), (86, 40), (22, 10)))
                c.put(x, y, "mortar", 3 if dab else (1 if rnd("ply", x, y // 3) > 0.25 else 2))
    for x in range(c.w):                                                # the glass channel's shadow under the frame
        if glass(x, g0 + 1) and not kept(x, g0 + 1) and x != gym.SEAM_COL:
            c.put(x, g0 + 1, "mortar", 0)
    for x, y in ((40, 69), (44, 68), (52, 69), (63, 69), (66, 68)):  # fallen slivers in the bottom channel
        c.put(x, y, "gal", 4)
        c.put(x + 1, y, "stone_cold", 2)
    return c


# =============================================================================================
# Keys, pins, writer
# =============================================================================================
KEYS = {
    "trog_fist_hole": [("trog_fist_hole", trog_fist_hole)],
    "trog_bell": [("trog_bell", trog_bell)],
    "trog_wall_marks": [("trog_knuckle_marks", trog_knuckle_marks), ("trog_slam_crack", trog_slam_crack)],
    "trog_crater": [("trog_impact_crater", trog_impact_crater)],
    "trog_threshold": [("trog_threshold", trog_threshold)],
    "trog_mirror_shards": [("trog_mirror_shards", trog_mirror_shards)],
    "trog_bench": [(f"trog_bench_{s}", BENCH_FRAMES[s]) for s in BENCH_STATES],
    "trog_bench_debris": [(f"trog_bench_debris_{f}", (lambda f=f: trog_bench_debris_frame(f))) for f in range(6)],
}
# One canvas, several frames: multi-piece stems store each piece as a frame (sizes must match per stem).
FRAMED = {"trog_bench": [100] * 3, "trog_bench_debris": [round(t * 1000 / 60) for t in DEB_TICKS] + [100]}


def key_sha() -> dict:
    """The key pins: SHA-256 of each key's pixel list (the execution's byte-identity check)."""
    return {pid: hashlib.sha256(json.dumps(fn().px).encode()).hexdigest()
            for pieces in KEYS.values() for pid, fn in pieces}


def write() -> None:
    for stem, pieces in KEYS.items():
        imgs = [fn() for _, fn in pieces]
        if len({(i.w, i.h) for i in imgs}) > 1:                     # different sizes: one layer per piece
            w, h = max(i.w for i in imgs), max(i.h for i in imgs)
            padded = []
            for i in imgs:
                p = Pix(w, h)
                for y in range(i.h):
                    p.px[y][:i.w] = i.px[y]
                padded.append(p)
            imgs = padded
        grid = kit.grid_from(imgs, layer_names=["main"], durations=FRAMED.get(stem, [100] * len(imgs)),
                             pivot=(0, 0))
        path = SRC / f"{stem}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{stem}.ase")], check=True)
        print(f"{stem}: {[pid for pid, _ in pieces]} {grid['canvas']} {len(grid['palette'])} colours")
    for pid, sha in key_sha().items():
        print(f"KEY {pid} {sha}")


def _luma(ramp: str, v: int) -> float:
    h = RAMPS[ramp][v]
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def _bbox(img: Pix):
    opaque = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x]]
    xs, ys = [p[0] for p in opaque], [p[1] for p in opaque]
    return (min(xs), min(ys), max(xs), max(ys)), len(opaque)


def measurements() -> list[str]:
    """The numbers the gate and the review check against spec section 12 (key part)."""
    out = []
    for stem, pieces in KEYS.items():
        for pid, fn in pieces:
            img = fn()
            cols = {p for row in img.px for p in row if p}
            (x0, y0, x1, y1), n = _bbox(img)
            out.append(f"{pid} {img.w}x{img.h} opaque={n} bbox=({x0},{y0})-({x1},{y1}) colours={len(cols)} "
                       f"max_luma={max(_luma(*c) for c in cols):.0f} transparent_px={img.w * img.h - n}")
    for s in ("intact", "weakened"):
        b, _ = _bbox(BENCH_FRAMES[s]())
        out.append(f"trog_bench_{s} drawn bbox={b} (target (0, 0, 31, 79) = world {{640,500,32,80}}): "
                   f"{'ok' if b == (0, 0, 31, 79) else 'FAIL'}")
    b, _ = _bbox(trog_bench_broken())
    out.append(f"trog_bench_broken drawn bbox={b} (target (0, 64, 31, 79) = world {{640,564,32,16}}): "
               f"{'ok' if b == (0, REMNANT_ROW, 31, 79) else 'FAIL'}")
    top = [trog_bench_intact().get(x, 0) for x in range(BENCH_W)]
    out.append(f"trog_bench_intact row0 walkable arris: {sorted(set(top))} (target one entry, brick 9)")
    last = trog_bench_debris_frame(5)
    (x0, y0, x1, y1), _ = _bbox(last)
    out.append(f"trog_bench_debris_5 settled litter rows {y0}..{y1} (target within 93..95)")
    m = _fist_mask()
    ys = [y for _, y in m]
    out.append(f"trog_fist_hole void rows {min(ys)}..{max(ys)} = world {HOLE_AT[1] + min(ys)}..{HOLE_AT[1] + max(ys)} "
               f"(the Basher's sprite top at spawn is world 493)")
    thr = trog_threshold()
    bad = [(x, y) for y in range(THR_H) for x in range(THR_W) if thr.get(x, y) and not _thr_in(x, y)]
    out.append(f"trog_threshold pixels over the plinth (cols >= 20, rows < 11)={len(bad)} (target 0)")
    pal = set()
    for pieces in KEYS.values():
        for _, fn in pieces:
            pal |= {p for row in fn().px for p in row if p}
    out.append(f"all keys: {len(pal)} distinct palette entries (per-atlas budget 63)")
    out += [f"KEY {pid} sha256={sha}" for pid, sha in key_sha().items()]
    return out


def to_image(img: Pix, silhouette: bool = False):
    from PIL import Image
    out = Image.new("RGBA", (img.w, img.h), (0, 0, 0, 0))
    for y in range(img.h):
        for x in range(img.w):
            p = img.px[y][x]
            if p is None:
                continue
            if silhouette:
                out.putpixel((x, y), (12, 10, 16, 255))
            else:
                h = RAMPS[p[0]][p[1]]
                out.putpixel((x, y), (int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16), 255))
    return out


# =============================================================================================
# Execution (spec sections 5, 13 and 16): the executor pieces, the atlases, the manifests, the contract.
# The keys above stay untouched. Stage 1 dresses `trog_run`; the stage-2 pieces (the 48 px shelf, the
# threshold, the shards, the bench and its debris) wait for TROG-B and have no atlas yet.
# =============================================================================================
import author_heat01 as heat  # noqa: E402  (AREA-HEAT-01: the shelf body the step faces derive from)


def end_of(body: "heat.Pix", left: bool) -> "heat.Pix":
    """One 8 px end of a step or shelf face: half of its mid, the outer two columns one step darker below
    row 0, a ``stone`` 6 corner on row 0 and the bottom outer corner cleared (spec section 5)."""
    h = body.h
    c = heat.Pix(8, h)
    x0 = 8 if left else 0
    for y in range(h):
        for x in range(8):
            q = body.get(x0 + x, y)
            if q is None and (x0 + x, y) == (15, h - 1):
                q = ("stone", 2)
            c.px[y][x] = q
    for y in range(1, h):
        for x in ((0, 1) if left else (6, 7)):
            q = c.px[y][x]
            if q is not None:
                c.put(x, y, q[0], q[1] - 1)
    c.put(0 if left else 7, 0, "stone", 6)
    c.clear(0 if left else 7, h - 1)
    return c


def darken(p: "heat.Pix") -> "heat.Pix":
    """One stone step darker below the arris row (TROG-ART-AD5): the face never outshines a troglodyte."""
    for y in range(1, p.h):
        for x in range(p.w):
            q = p.px[y][x]
            if q is not None and q[0] == "stone":
                p.px[y][x] = ("stone", max(0, q[1] - 1))
    return p


def trog_step_mid() -> "heat.Pix":
    return darken(heat._shelf_body())


def trog_step_end_l() -> "heat.Pix":
    return end_of(trog_step_mid(), True)


def trog_step_end_r() -> "heat.Pix":
    return end_of(trog_step_mid(), False)


FLOOR_Y = 580
STEP_RUNS = ((352, 548), (1840, 548))     # the low refuge and the bell step (sidecar statics, x0 and top y)

# The run atlases (stage 1): stem = the source file stem under assets/source/props/floor01.
ATLASES = {
    "floor01_trog_run_wall": {"stem": "trog_run_wall", "width": 176, "pieces": [
        ("trog_fist_hole", trog_fist_hole), ("trog_knuckle_marks", trog_knuckle_marks),
        ("trog_slam_crack", trog_slam_crack)]},
    "floor01_trog_run_props": {"stem": "trog_run_props", "width": 128, "pieces": [
        ("trog_bell", trog_bell), ("trog_impact_crater", trog_impact_crater), ("trog_step_mid", trog_step_mid),
        ("trog_step_end_l", trog_step_end_l), ("trog_step_end_r", trog_step_end_r)]},
}
LAYER_OF_ATLAS = {"floor01_trog_run_wall": ("backwall", "backwall_decal"), "floor01_trog_run_props": ("prop", "prop")}
# Floor-level pieces draw above the collision deck (FF-AD26): the crater and the step faces are `structure`.
LAYER_OVERRIDE = {"trog_impact_crater": ("structure", "structure"), "trog_step_mid": ("structure", "structure"),
                  "trog_step_end_l": ("structure", "structure"), "trog_step_end_r": ("structure", "structure")}
PAD = 8
EXEC_PIECES = ("trog_step_mid", "trog_step_end_l", "trog_step_end_r")
KEY_PIECE_IDS = tuple(pid for pieces in KEYS.values() for pid, _ in pieces)
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_trog_kit.json"
MANIFESTS = ROOT / "assets/manifests"
OWNER = ("AI-authored environment pixel art (AREA-TROG-01: design spec and keys Claude Opus, silhouette gate "
         "Claude Opus (independent); execution Claude Sonnet; independent review pending)")

# -- the dressing of `trog_run` (spec section 4) -------------------------------------------------------------
RUN_REMOVE_AT = [("structure", "arch_chain", 224, 384), ("structure", "arch_chain", 1232, 384),
                 ("prop", "torch_wall", 2240, 432), ("backwall_decal", "torch_pool_a", 2136, 348)]
RUN_FILL = [{"x": 176, "y": 352, "w": 208, "h": 208}, {"x": 1184, "y": 352, "w": 208, "h": 208}]


def _step_run(x0: int, y: int) -> list:
    """end_l, three mids, end_r side by side: 8 + 3 * 16 + 8 = 64 px, the static's width."""
    return [("structure", "trog_step_end_l", x0, y), ("structure", "trog_step_mid", x0 + 8, y),
            ("structure", "trog_step_mid", x0 + 24, y), ("structure", "trog_step_mid", x0 + 40, y),
            ("structure", "trog_step_end_r", x0 + 56, y)]


RUN_PLACE = [
    ("backwall_decal", "trog_slam_crack", *SLAM_AT), ("backwall_decal", "trog_knuckle_marks", *MARKS_AT),
    ("backwall_decal", "trog_fist_hole", *HOLE_AT),
    *_step_run(*STEP_RUNS[0]), ("structure", "trog_impact_crater", *CRATER_AT), *_step_run(*STEP_RUNS[1]),
    ("prop", "trog_bell", *BELL_AT),
]


def _at(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def dressing() -> dict:
    return {"trog_run": {"remove": [], "remove_at": [_at(*r) for r in RUN_REMOVE_AT], "backwall_fill": RUN_FILL,
                         "place": [_at(*t) for t in RUN_PLACE], "parallax": {}}}


def build_pieces(name: str) -> list:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def _pack(pieces: list, width: int, pad: int = 0):
    """Shelf packing, left to right: a piece's rect holds exactly its pixels (a key stays byte-identical)."""
    x = y = shelf = 0
    placed = []
    for pid, img in pieces:
        if x + img.w > width:
            x, y, shelf = 0, y + shelf + pad, 0
        placed.append((pid, img, x, y))
        x += img.w + pad
        shelf = max(shelf, img.h)
    sheet = Pix(width, y + shelf)
    rects = {}
    for pid, img, px, py in placed:
        for yy in range(img.h):
            for xx in range(img.w):
                if img.px[yy][xx] is not None:
                    sheet.px[py + yy][px + xx] = img.px[yy][xx]
        rects[pid] = [px, py, img.w, img.h]
    return sheet, rects


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = LAYER_OVERRIDE.get(pid) or LAYER_OF_ATLAS[atlas]
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0], "anchor_name": "top_left (room placement, spec section 5)", "grid_snap": 1,
             "grammar": ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    return entry


def contract(rects: dict, colours: dict) -> dict:
    atlases, pieces = {}, []
    for name, spec in ATLASES.items():
        atlases[name] = {"grid": f"assets/source/props/floor01/{spec['stem']}.grid.json",
                         "manifest": f"assets/manifests/{name}.json", "image": f"generated/art/{name}.png",
                         "metadata": f"generated/art/{name}.json"}
        for pid, _ in spec["pieces"]:
            pieces.append(piece_entry(pid, name, rects[name][pid]))
    return {
        "format": "env-kit-v2", "kit_id": "floor01_trog",
        "extends": "assets/source/terrain/floor01/floor01_gym_kit.json",
        "status": "proposed - AREA-TROG-01 kit (docs/02-art/area-trog-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": (
            "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema from the "
            "ENV-01 contract through the gym kit. Adds the troglodyte territory atlases for Floor 01 room "
            "`trog_run` (stage 1): no tiles, nothing collidable, no new palette colour, no new emitter. The "
            "dressing bricks up bays 1 and 4, removes two chains and one torch with its pool; the stage-2 "
            "strength room adds its own block."),
        "palette_use": {n: colours[n] for n in ATLASES},
        "atlases": atlases, "tiles": [], "pieces": pieces,
        "room_dressing": dressing(),
        "usage_rules": {"R1_trog_run": (
            "room `trog_run` only: the dressing never adds or moves a tile or a solid; bays 1 and 4 are bricked up "
            "across both pier columns; nothing stands across an open window above y 520; floor pieces (the crater "
            "and the step faces) are `structure` so they draw above the deck")},
    }


def manifest(name: str, canvas: list, stem: str) -> dict:
    return {
        "schema_version": 1, "id": f"{name}_v1", "type": "decor_atlas",
        "source_path": f"assets/source/props/floor01/{stem}.ase",
        "grid_source": f"assets/source/props/floor01/{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png", "output_metadata": f"generated/art/{name}.json",
        "palette_id": "floor01-catacomb-v2", "frame_size": canvas, "pivot": [0, 0],
        "animation_tags": [], "required_alpha": True,
        "kit_contract": "assets/source/terrain/floor01/floor01_trog_kit.json",
        "owner": OWNER, "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 pieces for the Floor 1 troglodyte run (AREA-TROG-01 on the ENV-01 "
                      "kit); rule-based shapes and ramps, the step faces derived from the AREA-HEAT-01 shelf body; "
                      "nothing cropped, traced or resampled from concept art; no text, logos, brands, faces or "
                      "likenesses",
            "generator": "Deterministic authoring script scripts/author_trogarea01.py written by AI agents (spec "
                         "and keys designed by Claude Opus; execution by Claude Sonnet); no image-model raster "
                         "output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1 troglodyte territory panel (mood reference only; no pixel cropped, traced "
                "or resampled)"]},
    }


def write_kit() -> None:
    """Write every stage-1 atlas grid + .ase, manifest and the contract."""
    rects, colours = {}, {}
    for name, spec in ATLASES.items():
        sheet, r = _pack(build_pieces(name), spec["width"], PAD)
        rects[name] = r
        grid = kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0))
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        colours[name] = len(grid["palette"])
        (MANIFESTS / f"{name}.json").write_text(
            json.dumps(manifest(name, grid["canvas"], spec["stem"]), indent=1) + "\n", encoding="utf-8")
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")
    CONTRACT.write_text(json.dumps(contract(rects, colours), indent=1) + "\n", encoding="utf-8")


def all_pieces() -> dict:
    """{piece id: Pix} for every stage-1 piece (keys and executor pieces)."""
    out = {}
    for name in ATLASES:
        out.update(dict(build_pieces(name)))
    return out


def mean_luma(img) -> float:
    vals = [_luma(*p) for row in img.px for p in row if p]
    return sum(vals) / len(vals)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "keys":
        for pid, sha in key_sha().items():
            print(f"KEY {pid} {sha}")
    elif len(sys.argv) > 1 and sys.argv[1] == "measure":
        print("\n".join(measurements()))
    else:
        write()
        write_kit()
