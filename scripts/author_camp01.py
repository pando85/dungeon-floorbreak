#!/usr/bin/env python3
"""Authoring record for AREA-CAMP-01: the dressing of the two Floor 01 quiet rooms ``rest_camp`` (the
survivors' camp) and ``rest_aftermath`` (the quiet after the Juicer).

Draws the frozen design keys of ``docs/02-art/area-camp-01-spec.md`` on ``floor01-catacomb-v2`` (no new
colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/camp_hearth.grid.json``  ``camp_hearth`` (hero key, spec section 6)
- ``assets/source/props/floor01/camp_fire.grid.json``    ``camp_fire`` 4-frame flame loop (spec section 7)
- ``assets/source/props/floor01/camp_figure.grid.json``  ``camp_figure_sentry`` (the figure language)
- ``assets/source/props/floor01/aft_wreck.grid.json``    ``aft_mirror_wreck`` (spec section 6)
- ``assets/source/props/floor01/aft_heavy_bag.grid.json`` ``aft_heavy_bag`` 5 sway frames (spec section 7)

The keys are frozen: execution adds the remaining pieces of spec section 5 as new functions and new
atlases, and must not redraw a key (a key change is a design change). Every pixel comes from rules (shapes,
ramps, a hash noise); nothing is traced or resampled from a concept image. No text, logos or likenesses;
the survivor figure is anonymous and faceless (REST-Q2).

    python3 scripts/author_camp01.py              # every atlas grid + .ase, manifest and the two kit contracts
    python3 scripts/author_camp01.py keys         # the key pins only
    python3 scripts/author_camp01.py gate CAPS OUT [BOARD]   # the gate sheets (design stage, see gate())
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
import author_gym01 as gym  # noqa: E402  (AREA-GYM-01: the mirror glass the wreck is torn from)

RAMPS = kit.RAMPS
SRC = ROOT / "assets/source/props/floor01"


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("camp01",) + key)).encode()).digest()
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


BAYER = ((0.0, 0.5), (0.75, 0.25))


def dith(f: float, x: int, y: int) -> int:
    """A smooth ramp value as an integer step with a 2 x 2 ordered dither at the step edges."""
    return int(f) + (1 if (f - int(f)) > BAYER[y % 2][x % 2] else 0)


# =============================================================================================
# Hero key camp_hearth (spec section 6): 96 x 64, prop layer, top-left placement (720, 496), bottom row 63 on
# the floor's back line (y 559). The camp's protected central light: a stone fire ring under a tripod with a
# dented pot, screened by two canvas wings toward the open arches, a woodpile on the left and a sitting log on
# the right. The flame itself is the separate animated piece camp_fire (drawn after the hearth).
# =============================================================================================
HEARTH_W, HEARTH_H = 96, 64
FIRE_C = (48, 55)            # the flame base (hearth-local): centre of the ember bed
FIRE_AT = (38, 30)           # camp_fire's top-left inside the hearth (20 x 26 canvas, bottom row on y 55)


def _warm(x: int, y: int) -> float:
    """0..1: how much of the fire's light reaches a hearth-local pixel (baked bounce on the near faces)."""
    d = math.hypot((x - FIRE_C[0]) * 0.8, (y - (FIRE_C[1] - 8)) * 1.1)
    return clamp(1.0 - d / 46.0, 0.0, 1.0)


def _wing(c: Pix, left: bool) -> None:
    """One canvas wing: a salvaged pole and a sheet lashed to it, the inner face lit by the fire. The left
    wing's top corner is torn; the right wing carries a square patch."""
    pole = 3 if left else 92
    inner = 23 if left else 72                      # the sheet's free edge toward the fire
    for y in range(14, HEARTH_H):                   # the pole, a little crooked
        x = pole + (1 if (left and y > 40) or (not left and y < 30) else 0)
        c.put(x, y, "wood", 2)
        c.put(x + (1 if left else -1), y, "wood", 3 if y % 7 else 2)
    c.box(pole - 1, 13, pole + 1, 14, "wood", 1)    # the pole cap
    c.put(pole, 12, "wood", 3)
    for y in range(17, 59):
        t = (y - 17) / 41
        # the top edge slopes down toward the fire; the bottom is pegged at y 58
        top = 17 + round(abs(inner - pole) * 0.36 * (0 if left else 0))
        x_free = inner + round(2 * math.sin(t * math.pi))           # the free edge bellies outward
        lo, hi = (pole + 1, x_free) if left else (x_free, pole - 1)
        for x in range(lo, hi + 1):
            # top edge: falls 7 px from the pole to the free edge
            frac = (x - pole) / (inner - pole)
            if y < 17 + round(7 * frac):
                continue
            fold = math.sin((x - pole) * 0.9 + y * 0.05)
            v = 5.2 + 0.8 * fold - 1.6 * t
            w = _warm(x, y)
            if frac > 0.82:                                         # the free edge catches the fire
                v += 1.0
            if w > 0.5:
                c.put(x, y, "gal_warm", clamp(1.4 + w * 3.4 + 0.5 * fold, 2, 5))
            elif w > 0.36 and (x + y) % 2 == 0:
                c.put(x, y, "gal_warm", 2)
            else:
                c.gput(x, y, "stone", clamp(v, 2.0, 7.0), "wing" + str(left))
        _ = top
    # the hem: a darker turned-up row and the pegs
    for x in range(min(pole, inner), max(pole, inner) + 1):
        if c.get(x, 58):
            c.put(x, 58, "stone", 2)
    peg = inner - 1 if left else inner + 1
    c.box(peg, 57, peg, 63, "wood", 1)
    c.put(peg, 57, "wood", 4)
    if left:
        for (x, y) in ((20, 17), (21, 17), (22, 17), (21, 18), (22, 18), (22, 19), (22, 20)):
            c.clear(x, y)                                           # torn top corner
        for k in range(4):                                          # the torn strip hanging from it
            c.put(23 + (k % 2), 21 + k, "stone", 4)
    else:
        for y in range(36, 45):                                     # the square patch, stitched
            for x in range(78, 87):
                edge = y in (36, 44) or x in (78, 86)
                c.put(x, y, "stone", 3 if edge else (4 if (x + y) % 3 else 5))
        for x in range(79, 86, 2):
            c.put(x, 36, "stone", 6)
            c.put(x, 44, "stone", 6)


def _tripod(c: Pix) -> None:
    apex = (48, 10)
    # the back leg first (darker, partly hidden), then the two front legs, inner faces lit by the fire
    c.line(apex[0], apex[1], 53, 52, "wood", 1)
    for (fx, fy, lit_dx) in ((29, 62, 1), (67, 62, -1)):
        n = max(abs(fx - apex[0]), abs(fy - apex[1]))
        for i in range(n + 1):
            x = round(apex[0] + (fx - apex[0]) * i / n)
            y = round(apex[1] + (fy - apex[1]) * i / n)
            c.put(x, y, "wood", 2)
            w = _warm(x + lit_dx, y)
            if w > 0.4:
                c.put(x + lit_dx, y, "gal_warm", 2 + round(w * 1.6))
            else:
                c.put(x + lit_dx, y, "wood", 3)
    c.box(46, 9, 50, 12, "wood", 1)                                 # the lashing at the apex
    for x in (46, 48, 50):
        c.put(x, 10, "stone", 5)
    c.put(47, 8, "wood", 3)
    c.put(49, 8, "wood", 3)


def _pot(c: Pix) -> None:
    """A dented pot on a short chain under the apex, its underside lit by the flames."""
    for y in range(13, 22):                                         # chain: alternate 2 px links
        c.put(48, y, "iron", 4 if y % 3 else 2)
        if y % 3 == 0:
            c.put(47, y, "iron", 3)
    for x, y in ((41, 24), (42, 23), (43, 22), (44, 22), (45, 21), (51, 21), (52, 22), (53, 22), (54, 23),
                 (55, 24)):
        c.put(x, y, "iron", 4)                                      # the bail
    c.put(46, 21, "iron", 3)
    c.put(50, 21, "iron", 3)
    c.put(47, 21, "iron", 5)
    c.put(48, 21, "iron", 5)
    c.put(49, 21, "iron", 3)
    for y in range(24, 34):                                         # body: x 40..56, rounded bottom
        ly = y - 24
        inset = 0 if ly < 6 else (1 if ly < 8 else 2 + (ly - 8))
        for x in range(40 + inset, 57 - inset):
            v = 3.2 + (1.4 if x < 44 else 0) - (1.2 if x > 52 else 0) - 0.12 * ly
            c.put(x, y, "iron", v)
        if ly >= 6:                                                 # firelight on the underside
            for x in range(41 + inset, 56 - inset):
                if (x + y) % 2 == 0 or ly == 9:
                    c.put(x, y, "gal_warm", 3 if ly < 9 else 4)
    for x in range(39, 58):                                         # the rim
        c.put(x, 24, "iron", 5 if x < 50 else 4)
    c.put(39, 25, "iron", 2)
    c.put(57, 25, "iron", 2)
    for (x, y) in ((52, 26), (53, 27), (53, 26), (54, 27)):        # the dent on the right shoulder
        c.put(x, y, "iron", 1)
    c.put(51, 26, "iron", 5)
    for x in range(43, 54):                                         # the lid, ajar: a dark gap at the left
        c.put(x, 23, "iron", 3 if x > 45 else 1)
    c.put(48, 22, "iron", 5)                                        # lid knob


def _fire_ring(c: Pix) -> None:
    # the ash bed and the back stones (above the logs, smaller), then the logs, then the front stones
    for x in range(31, 66):
        for y in range(51, 56):
            if c.get(x, y) is None or c.get(x, y)[0] == "wood":
                c.put(x, y, "mortar", 2 + (1 if rnd("ash", x, y) > 0.6 else 0))
    for k, sx in enumerate(range(33, 63, 6)):                      # back stones: 5 px wide, 3 tall
        for y in range(50, 53):
            for x in range(sx, sx + 5):
                if (x, y) in ((sx, 50), (sx + 4, 50)):
                    continue
                c.put(x, y, "stone", 3 + (1 if y == 50 else 0) + (1 if k in (1, 2) and y == 50 else 0))
    # two crossed logs, embers glowing on their tops
    for (x0, y0, x1, y1) in ((33, 56, 61, 51), (35, 51, 63, 56)):
        n = x1 - x0
        for i in range(n + 1):
            x = x0 + i
            y = round(y0 + (y1 - y0) * i / n)
            c.put(x, y, "wood", 3)
            c.put(x, y + 1, "wood", 1)
            if rnd("ember", x, y) > 0.55:
                c.put(x, y, "flame", 0 if rnd("e2", x) > 0.4 else 1)
    for x in (33, 34, 62, 63):                                      # charred log ends
        c.put(x, 54, "mortar", 1)
    # the front stones: seven, 5 or 6 px wide, 6 tall, lit on top by the fire, dark toward the floor
    x = 27
    for k in range(7):
        w = 5 + (1 if rnd("fs", k) > 0.5 else 0)
        h = 6 if k in (2, 3, 4) else 5
        for yy in range(63 - h + 1, 64):
            for xx in range(x, x + w):
                ly = yy - (63 - h + 1)
                if (xx in (x, x + w - 1)) and ly == 0:
                    continue                                        # rounded top corners
                if ly == 0:
                    c.put(xx, yy, "gal_warm", 4 if abs(xx - 48) < 12 else 3)
                elif ly == 1:
                    c.put(xx, yy, "stone", 6)
                else:
                    c.put(xx, yy, "stone", 4.6 - 0.5 * ly + (0.6 if xx == x else 0) - (0.8 if xx == x + w - 1 else 0))
        x += w + 1 if k % 3 == 0 else w


def _woodpile(c: Pix) -> None:
    """Split logs stacked against the left wing: end grain toward the viewer."""
    rows = ((6, 58, 4), (11, 58, 4), (16, 58, 4), (8, 53, 4), (13, 53, 4))
    for cx, cy, r in rows:
        c.disc(cx + 0.5, cy + 0.5, r - 0.5, "wood",
               lambda x, y, d, cx=cx, cy=cy: 4 if d > r - 1.6 else (3 if (round(d) % 2) else 2))
        c.put(cx, cy, "wood", 1)
    for x in range(3, 21):                                          # the floor row under the pile
        if c.get(x, 63) is None:
            c.put(x, 63, "wood", 0)


def _log_seat(c: Pix) -> None:
    """A sitting log on the right, bark on top, the cut face toward the fire (lit)."""
    x0, x1, y0, y1 = 71, 90, 56, 63
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            ly = y - y0
            v = 3.6 if ly == 0 else (2.6 if ly < 3 else 1.6)
            if rnd("bark", x // 2, y) > 0.8:
                v -= 1
            c.put(x, y, "wood", v)
    for y in range(y0, y1 + 1):                                     # the cut face (left end), lit
        for x in range(x0, x0 + 3):
            c.put(x, y, "gal_warm", 3 if (x + y) % 2 else 4)
    c.put(x0 + 1, y0 + 3, "wood", 3)
    c.put(x0 + 1, y0 + 4, "wood", 3)
    for x in range(x0 + 3, x1 + 1):                                 # the lit top edge of the bark
        if _warm(x, y0) > 0.2:
            c.put(x, y0, "gal_warm", 3)


def _floor_scorch(c: Pix) -> None:
    for x in range(22, 75):                                         # soot and spilled ash in the floor row
        if c.get(x, 63) is None and rnd("soot", x) > 0.35:
            c.put(x, 63, "mortar", 1)


def camp_hearth() -> Pix:
    c = Pix(HEARTH_W, HEARTH_H)
    _wing(c, True)
    _wing(c, False)
    _tripod(c)
    _pot(c)
    _woodpile(c)
    _log_seat(c)
    _fire_ring(c)
    _floor_scorch(c)
    return c


# =============================================================================================
# camp_fire (spec section 7): 20 x 26, 4 frames, a contract animation (the torch_wall pattern: animation +
# per-frame light flicker). Frame bottom row 25 sits on the ember bed (hearth y 55). Tongues per frame:
# (centre x, height, lean, half width at the base). Heat runs from a near-white core at the base to dark red
# tips; one or two loose sparks above.
# =============================================================================================
FIRE_W, FIRE_H = 20, 26
FIRE_DURATIONS = (110, 90, 130, 100)
FIRE_TONGUES = (
    ((10.0, 22, 0.0, 6.0), (6.0, 14, -2.0, 3.0), (14.5, 16, 2.0, 3.0)),
    ((10.5, 24, 1.5, 5.5), (5.5, 12, -1.0, 3.0), (14.0, 18, 3.0, 3.2)),
    ((9.5, 21, -2.0, 6.0), (5.0, 17, -3.0, 3.2), (15.0, 12, 1.0, 2.8)),
    ((10.0, 23, -0.5, 5.8), (6.5, 15, -1.5, 3.0), (14.0, 14, 1.5, 3.0)),
)
FIRE_SPARKS = (((12, 1),), ((6, 4), (15, 0)), ((13, 2),), ((8, 0), (4, 6)))


def _fire_frame(k: int) -> Pix:
    c = Pix(FIRE_W, FIRE_H)
    base = FIRE_H - 1
    heat = {}
    for cx, h, lean, hw in FIRE_TONGUES[k]:
        for y in range(base - h + 1, base + 1):
            t = (base - y) / h                                      # 0 at the base, 1 at the tip
            centre = cx + lean * t * t
            half = hw * (1 - t) ** 0.75 + 0.4
            for x in range(FIRE_W):
                d = abs(x + 0.5 - centre)
                if d <= half:
                    q = (1 - t) * (1 - 0.55 * d / half)             # heat: base and axis hottest
                    heat[(x, y)] = max(heat.get((x, y), 0.0), q)
    for (x, y), q in heat.items():
        v = 5 if q > 0.78 else (4 if q > 0.6 else (3 if q > 0.4 else (2 if q > 0.22 else (1 if q > 0.1 else 0))))
        c.put(x, y, "flame", v)
    for x in range(FIRE_W):                                         # the base row: a hot seam on the embers
        if c.get(x, base):
            c.put(x, base, "flame", 3 if abs(x - 10) < 5 else 1)
    for sx, sy in FIRE_SPARKS[k]:
        c.put(sx, sy, "flame", 3)
        c.put(sx, sy + 1, "flame", 1)
    return c


def camp_fire_frames() -> list[Pix]:
    return [_fire_frame(k) for k in range(4)]


# =============================================================================================
# camp_figure_sentry (spec section 8): 20 x 48, prop layer, top-left placement (500, 512), bottom row 47 on
# the floor's back line. The survivor figure language: anonymous, faceless, hooded, a long patched coat,
# tired weight on the back leg, a lashed pole-spear held upright in the front hand. Faces left (toward the
# entrance). Cold low values (gal 1..4) with a cool rim from the upper left and a warm rim on the fire side
# (right): readable against brick, never brighter than the heroes, never a monster palette.
# =============================================================================================
FIG_W, FIG_H = 20, 48


def _figure_body(c: Pix) -> None:
    # hood and head: a rounded hood, the opening on the left a flat dark void (no face)
    hood = {3: (8, 12), 4: (7, 13), 5: (6, 13), 6: (6, 14), 7: (6, 14), 8: (6, 14), 9: (6, 14), 10: (7, 14),
            11: (7, 14)}
    for y, (x0, x1) in hood.items():
        for x in range(x0, x1 + 1):
            v = 2.4 + (1.2 if x == x0 or y == 3 else 0) - (0.8 if x == x1 else 0)
            c.put(x, y, "gal", v)
    for (x, y) in ((6, 6), (6, 7), (7, 6), (7, 7), (7, 8), (6, 8), (7, 9)):
        c.put(x, y, "gal", 0)                                       # the hood opening (no features)
    c.put(14, 8, "gal_warm", 2)
    c.put(14, 9, "gal_warm", 2)
    # shoulders and the long coat: a slight hunch (the back shoulder higher), the hem widening
    for y in range(12, 41):
        ly = y - 12
        x0 = 6 - (1 if ly > 2 else 0) - (1 if ly > 20 else 0)
        x1 = 14 + (1 if ly < 3 else 0) + (1 if ly > 22 else 0)
        if ly == 0:
            x0, x1 = 7, 14
        for x in range(x0, x1 + 1):
            v = 2.0 + (0.9 if x == x0 else 0) - (0.6 if x >= x1 - 1 else 0)
            if ly > 16 and (x - x0) % 4 == 3:
                v -= 0.8                                            # long coat folds
            c.put(x, y, "gal", v)
        c.put(x1, y, "gal_warm", 1 if ly % 3 else 2)                # the warm rim on the fire side
    for x in range(6, 16):                                          # belt (rope) and the satchel strap
        c.put(x, 26, "wood", 1)
    for i in range(10):
        c.put(14 - i // 2 + 0, 13 + i, "wood", 2)
    c.box(14, 27, 16, 33, "wood", 2)                                # the satchel on the back hip
    c.put(14, 27, "wood", 3)
    c.put(16, 33, "wood", 1)
    c.box(8, 30, 11, 33, "stone", 3)                                # a lighter patch sewn on the coat
    c.put(8, 30, "stone", 4)
    # the front arm reaching to the pole, the wrapped hand
    for i, (x, y) in enumerate(((7, 14), (6, 15), (6, 16), (5, 17), (5, 18), (5, 19), (4, 20), (4, 21))):
        c.put(x, y, "gal", 3 if i < 3 else 2)
        c.put(x + 1, y, "gal", 1)
    c.box(3, 21, 4, 23, "wood", 3)
    c.put(3, 21, "wood", 4)
    # legs and boots: weight on the back leg, the front leg a step forward
    for y in range(41, 46):
        for x in (6, 7, 8):
            c.put(x - (1 if y > 43 else 0), y, "gal", 1 if x < 8 else 0)
        for x in (11, 12, 13):
            c.put(x, y, "gal", 1)
        c.put(13, y, "gal_warm", 1)
    for x in range(4, 9):
        c.put(x, 46, "wood", 1)
        c.put(x, 47, "iron", 1)
    for x in range(10, 15):
        c.put(x, 46, "wood", 1)
        c.put(x, 47, "iron", 1)
    c.put(4, 46, "wood", 2)


def _figure_spear(c: Pix) -> None:
    for y in range(2, 48):                                          # the shaft, held at the hand
        c.put(2, y, "wood", 3 if y % 9 else 2)
        if 21 <= y <= 23:
            continue
        c.put(3, y, "wood", 1) if y > 23 else None
    for (x, y, v) in ((2, 0, 6), (2, 1, 5), (1, 2, 4), (3, 2, 3), (1, 3, 3), (3, 3, 2)):
        c.put(x, y, "iron", v)                                      # the lashed blade
    c.box(1, 4, 3, 6, "stone", 4)                                   # the cloth lashing
    c.put(1, 4, "stone", 5)


def camp_figure_sentry() -> Pix:
    c = Pix(FIG_W, FIG_H)
    _figure_spear(c)
    _figure_body(c)
    return c


# =============================================================================================
# Hero key aft_mirror_wreck (spec section 6): 160 x 72, prop layer, top-left placement (368, 488), bottom row
# 71 on the floor's back line. A gym mirror bay (AREA-GYM-01 glass, reused pixels) torn off the wall: the left
# panel leans broken against the pier, a radial crack star from an impact, a missing wedge showing the
# plywood backing, a shard spill, a bent barbell with an edge-on plate, a rolled-off plate face-on, a twisted
# length of the aluminium frame. Torch columns at local x 32..47 and 128..143 stay clear above row 16 (the
# composer's torches at x 400 and 496 end at y 503).
# =============================================================================================
WRECK_W, WRECK_H = 160, 72
PANEL_X = 54                  # the leaning panel's foot (wreck-local)
IMPACT = (24, 26)             # the crack star's centre, panel-local
CLEAR_COLS = ((32, 47), (128, 143))
CLEAR_ROWS = 16


def _panel() -> list[list]:
    """The torn left panel of gym_mirror_bay: frame cols 0..47, rows 8..70 (the top 8 rows snapped off along
    a jagged break), cracked and with a missing wedge. Returns rows of pixels (None transparent)."""
    bay = gym.mirror_bay()
    rows = []
    for y in range(8, 71):
        row = [bay.px[y][x] for x in range(0, 48)]
        # torn off the wall, the glass no longer reflects the lit room: the reflected bricks, bench and
        # torch glows become dark glass (only the glass blues and the frame keep their pixels)
        for x, p in enumerate(row):
            if p is not None and p[0] not in ("gal", "stone_cold"):
                row[x] = ("gal", 3 if p[0] not in ("flame", "gal_warm") and _luma(*p) > 70 else 2)
        row[47] = ("stone_cold", 2)                                 # the seam becomes the panel's right frame
        rows.append(row)
    h = len(rows)
    # the jagged break along the top
    for x in range(48):
        cut = 2 + round(4 * abs(math.sin(x * 0.37)) + 3 * rnd("brk", x // 3))
        for y in range(cut):
            rows[y][x] = None
        if cut < h and rows[cut][x] is not None:
            rows[cut][x] = ("gal", 10) if x % 3 else ("stone_cold", 4)   # the bright broken edge
    # the crack star: rays of bright/dark pixel pairs, a partial ring
    cx, cy = IMPACT
    for k in range(9):
        a = k * 40 + 13 * rnd("ray", k) * 2
        ln = 9 + 22 * rnd("rl", k)
        for i in range(2, int(ln)):
            jit = (rnd("rj", k, i // 4) - 0.5) * 0.25
            x = round(cx + math.cos(math.radians(a) + jit) * i)
            y = round(cy + math.sin(math.radians(a) + jit) * i)
            if 0 < x < 47 and 0 <= y < h and rows[y][x] is not None:
                rows[y][x] = ("gal", 10) if i % 2 else ("stone_cold", 0)
    for k in range(28):
        a = k * (360 / 28)
        x = round(cx + math.cos(math.radians(a)) * 5.5)
        y = round(cy + math.sin(math.radians(a)) * 4.5)
        if rows[y][x] is not None and k % 4 != 3:
            rows[y][x] = ("stone_cold", 0) if k % 2 else ("gal", 9)
    rows[cy][cx] = ("gal", 10)
    # the missing wedge: the plywood backing between two rays, glue dabs
    for y in range(h):
        for x in range(1, 47):
            dx, dy = x - cx, y - cy
            d = math.hypot(dx, dy)
            a = math.degrees(math.atan2(dy, dx)) % 360
            if 3 < d < 15 and 118 < a < 156:
                rows[y][x] = ("wood", 3 if (y + x // 5) % 4 else 2)
                if d > 13.5 or abs(a - 118) < 3 or abs(a - 156) < 3:
                    rows[y][x] = ("wood", 1)                        # the wedge's shadowed rim
    for (x, y) in ((13, 33), (16, 36)):
        rows[y][x] = ("brass", 4)                                   # glue dabs on the backing
    return rows


def _lean(c: Pix, rows: list[list]) -> None:
    """The panel leans right against the pier: each row shifts right by its height above the foot / 6."""
    h = len(rows)
    for i, row in enumerate(rows):
        y = WRECK_H - h - 1 + i                                    # the bottom frame row lands on row 70
        shift = (h - 1 - i) // 6
        for x, p in enumerate(row):
            if p is not None:
                c.px[y][PANEL_X + x + shift] = p
    # the leaning top shades the wall: nothing; the foot sits on the floor row
    for x in range(PANEL_X - 1, PANEL_X + 49):
        c.put(x, WRECK_H - 1, "iron", 0)


def _barbell(c: Pix) -> None:
    """A bent bar lying in front of the panel: kinked up in the middle, an edge-on plate on its left end."""
    pts = [(6, 66), (40, 64), (52, 58), (64, 64), (100, 67)]
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = max(abs(x1 - x0), 1)
        for i in range(n + 1):
            x = x0 + i
            y = round(y0 + (y1 - y0) * i / n)
            c.put(x, y, "stone_cold", 4)
            c.put(x, y + 1, "stone_cold", 2)
    c.put(52, 57, "gal", 9)                                         # the stressed kink glints
    for x in range(10, 16):                                         # the edge-on plate (left end)
        for y in range(53, 71):
            ly = y - 53
            if (x in (10, 15)) and ly in (0, 17):
                continue
            v = 3 if x == 10 else (4 if x == 11 else (2 if x < 14 else 1))
            c.put(x, y, "iron", v)
    c.box(16, 63, 17, 68, "stone_cold", 3)                          # the collar
    c.put(16, 63, "stone_cold", 4)


def _plate(c: Pix) -> None:
    """The plate that rolled off, face-on against the wall: iron disc, raised rim, hub (no numerals)."""
    cx, cy, r = 133.0, 62.5, 9.0
    c.disc(cx, cy, r, "iron", lambda x, y, d: (4 if y < cy else 3) if d > r - 1.5 else
           (2 if d > 3.2 else (1 if d > 1.6 else 0)))
    for (x, y) in ((128, 56), (129, 55), (130, 55)):
        c.put(x, y, "iron", 6)                                      # the rim highlight, upper left
    for k in range(6):                                              # the cast ring on the face
        a = math.radians(k * 60 + 20)
        c.put(round(cx + math.cos(a) * 5.5), round(cy + math.sin(a) * 5.5), "iron", 3)


def _frame_strip(c: Pix) -> None:
    """A twisted length of the aluminium frame and a run of rubber skirting on the floor."""
    pts = [(104, 68), (112, 66), (118, 62), (121, 63), (124, 68)]
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        c.line(x0, y0, x1, y1, "stone_cold", 3)
        c.line(x0, y0 + 1, x1, y1 + 1, "stone_cold", 1)
    for x in range(144, 158):
        c.put(x, 69, "iron", 3)
        c.put(x, 70, "iron", 1)
        c.put(x, 71, "iron", 0)


def _shards(c: Pix) -> None:
    """Glass shards along the floor: small triangles (blue glass, a white glint, a warm reflection)."""
    spots = ((22, 70), (30, 69), (36, 70), (44, 68), (50, 70), (70, 69), (80, 70), (88, 68), (96, 70), (112, 70),
             (118, 69), (148, 67))
    for k, (x, y) in enumerate(spots):
        size = 2 + int(rnd("sh", k) * 3)
        for i in range(size):
            for j in range(size - i):
                xx, yy = x + j - (size - i) // 2, y - i
                if c.get(xx, yy) is None or c.get(xx, yy)[0] == "iron":
                    c.put(xx, yy, "stone_cold", 3 if j else 4)
        c.put(x, y - size + 1, "gal", 10 if k % 3 else 9)
        if k % 4 == 1:
            c.put(x + 1, y, "gal_warm", 3)
    for x in range(0, WRECK_W):                                     # grit on the floor row
        if c.get(x, 71) is None and rnd("grit", x) > 0.82:
            c.put(x, 71, "stone_cold", 1)


def aft_mirror_wreck() -> Pix:
    c = Pix(WRECK_W, WRECK_H)
    _lean(c, _panel())
    _barbell(c)
    _plate(c)
    _frame_strip(c)
    _shards(c)
    return c


# =============================================================================================
# aft_heavy_bag (spec section 7): 40 x 122, 5 frames (rest, r1, r2, l1, l2), prop layer, top-left placement
# (620, 450): the chain hooks under the girder at (640, 450), the bag body spans y 514..565, its sand trickle
# reaches y 571. Sway frames shear the pre-baked rest frame about the hook: the bottom moves 0, +2, +4, -2,
# -4 px; the sand trickle falls straight down from the split wherever the bag is. Red leather with two grey
# tape bands, a sewn patch and a split seam low on the right: worn, never new.
# =============================================================================================
BAG_W, BAG_H = 40, 122
BAG_HOOK = (20, 0)
BAG_BODY = (7, 64, 32, 115)          # x0, y0, x1, y1 of the cylinder
BAG_SWAY = (0, 2, 4, -2, -4)
BAG_FRAMES = ("rest", "r1", "r2", "l1", "l2")
SPLIT = (28, 104)                    # the split seam's low end (bag-local, rest frame)


def _bag_rest() -> Pix:
    c = Pix(BAG_W, BAG_H)
    # the hanging chain: alternate face-on (3 x 4, hollow) and edge-on (1 x 4) links, 4 px pitch
    for k, y in enumerate(range(0, 44, 4)):
        if k % 2 == 0:
            for (dx, dy) in ((-1, 0), (0, 0), (1, 0), (-1, 1), (1, 1), (-1, 2), (1, 2), (-1, 3), (0, 3), (1, 3)):
                c.put(20 + dx, y + dy, "iron", 4 if dx < 1 else 3)
        else:
            for dy in range(-1, 5):
                c.put(20, y + dy, "iron", 5 if dy < 2 else 3)
    c.box(19, 44, 21, 47, "iron", 3)                                # the swivel
    c.put(19, 44, "iron", 5)
    c.put(21, 47, "iron", 1)
    # four straps of short chain to the cap's D-rings (two visible per side)
    for (x1, y1) in ((9, 60), (14, 61), (26, 61), (31, 60)):
        n = max(abs(x1 - 20), abs(y1 - 48))
        for i in range(n + 1):
            x = round(20 + (x1 - 20) * i / n)
            y = round(48 + (y1 - 48) * i / n)
            c.put(x, y, "iron", 3 if i % 2 else (4 if x1 < 20 else 2))
    # the cap: a rounded leather crown, D-rings
    for y in range(60, 64):
        inset = (3, 1, 0, 0)[y - 60]
        for x in range(BAG_BODY[0] + inset, BAG_BODY[2] - inset + 1):
            c.put(x, y, "brick", 4 + (1 if y > 60 else 0) + (1 if x < 14 else 0) - (1 if x > 27 else 0))
    for x in (9, 31):
        c.put(x, 61, "iron", 5)
        c.put(x, 62, "iron", 3)
    # the cylinder: lit from the upper left, a narrow highlight band, a dark terminator on the right
    x0, y0, x1, y1 = BAG_BODY
    for y in range(y0, y1 + 1):
        ly = y - y0
        bulge = 1 if ly > 40 else 0                                 # sand settles low: the bottom swells
        for x in range(x0 - bulge, x1 + bulge + 1):
            u = (x - (x0 - bulge)) / ((x1 + bulge) - (x0 - bulge))   # 0 left .. 1 right
            f = 6.2 + 1.9 * math.cos((u - 0.25) * math.pi * 1.1) - 0.02 * ly
            if u > 0.86:
                f = 2.6
            if u < 0.05:
                f = 4.6                                             # the far-left edge turns away
            c.put(x, y, "brick", clamp(dith(f, x, y), 2, 8))
        # two vertical seams with stitches
        for sx in (14, 26):
            if ly % 3 != 2:
                c.put(sx, y, "brick", 4 if sx == 14 else 3)
    for y in range(y1 + 1, y1 + 5):                                 # the rounded bottom
        k = y - y1
        for x in range(x0 + k * 2 - 1, x1 - k * 2 + 2):
            c.put(x, y, "brick", 3 if k > 1 else 4)
    # tape bands: grey tape wrapped round, edges a pixel ragged, one loose end
    for (ty, th) in ((74, 5), (97, 4)):
        for y in range(ty, ty + th):
            for x in range(x0 - (1 if y > y0 + 40 else 0), x1 + (2 if y > y0 + 40 else 1)):
                if (y == ty or y == ty + th - 1) and rnd("tape", x, y) > 0.8:
                    continue
                u = (x - x0) / (x1 - x0)
                f = 5.6 + 1.6 * math.cos((u - 0.25) * math.pi * 1.1)
                if u > 0.86:
                    f = 3.0
                c.put(x, y, "stone", clamp(round(f), 2, 8))
    for (x, y) in ((32, 98), (33, 99), (33, 100), (34, 101)):       # the loose tape end
        c.put(x, y, "stone", 4)
    # the sewn patch, upper left
    for y in range(82, 89):
        for x in range(10, 16):
            edge = y in (82, 88) or x in (10, 15)
            c.put(x, y, "brick", 5 if edge else 6)
    for x in range(10, 16, 2):
        c.put(x, 82, "stone", 6)
        c.put(x, 88, "stone", 6)
    # the split seam: a dark slit low on the right, sand showing in it
    for i, y in enumerate(range(SPLIT[1] - 8, SPLIT[1] + 1)):
        x = SPLIT[0] - (1 if i < 3 else 0)
        c.put(x, y, "brick", 1)
        if 2 < i < 8:
            c.put(x + 1, y, "stone", 8 if i % 2 else 7)
    return c


def _shear(src: Pix, dx_bottom: int) -> Pix:
    """Shift each row right by round(dx_bottom * y / 115) (the bag swings about the hook)."""
    c = Pix(BAG_W, BAG_H)
    for y in range(BAG_H):
        s = round(dx_bottom * y / 115)
        for x in range(BAG_W):
            p = src.px[y][x]
            if p is not None and 0 <= x + s < BAG_W:
                c.px[y][x + s] = p
    return c


def _trickle(c: Pix, dx_bottom: int, k: int) -> None:
    """Sand falls straight down from the split to the floor (y 121); a sparse stream."""
    s = round(dx_bottom * SPLIT[1] / 115)
    x = SPLIT[0] + 1 + s
    for y in range(SPLIT[1] + 2, BAG_H):
        if (y + k) % 3 != 0:
            c.put(x, y, "stone", 8 if (y + k) % 3 == 1 else 7)


def aft_heavy_bag_frames() -> list[Pix]:
    rest = _bag_rest()
    out = []
    for k, dx in enumerate(BAG_SWAY):
        f = _shear(rest, dx)
        _trickle(f, dx, k)
        out.append(f)
    return out


# The presentation-only sway table (spec section 7, CAMP-AD7): frame per tick after a trigger (entry or a hit
# rising edge); a left push mirrors r <-> l. 38 ticks, then rest.
SWAY_TABLE = ("r1",) * 4 + ("r2",) * 6 + ("r1",) * 4 + ("rest",) * 4 + ("l1",) * 8 + ("rest",) * 4 + \
    ("r1",) * 4 + ("rest",) * 4


# =============================================================================================
# Keys, pins, writer
# =============================================================================================
KEYS = {
    "camp_hearth": [("camp_hearth", camp_hearth)],
    "camp_fire": [(f"camp_fire_{k}", (lambda k=k: _fire_frame(k))) for k in range(4)],
    "camp_figure": [("camp_figure_sentry", camp_figure_sentry)],
    "aft_wreck": [("aft_mirror_wreck", aft_mirror_wreck)],
    "aft_heavy_bag": [(f"aft_heavy_bag_{n}", (lambda i=i: aft_heavy_bag_frames()[i]))
                      for i, n in enumerate(BAG_FRAMES)],
}
FRAMED = {"camp_fire": list(FIRE_DURATIONS), "aft_heavy_bag": [100] * 5}   # one canvas, several frames


def key_sha() -> dict:
    """The key pins: SHA-256 of each key's pixel list (the execution's byte-identity check)."""
    return {pid: hashlib.sha256(json.dumps(fn().px).encode()).hexdigest()
            for pieces in KEYS.values() for pid, fn in pieces}


def _luma(ramp: str, v: int) -> float:
    h = RAMPS[ramp][v]
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def measurements() -> list[str]:
    """The numbers the gate and the review check against spec section 12."""
    out = []
    for stem, pieces in KEYS.items():
        for pid, fn in pieces:
            img = fn()
            cols = {p for row in img.px for p in row if p}
            opaque = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x]]
            xs, ys = [p[0] for p in opaque], [p[1] for p in opaque]
            out.append(f"{pid} {img.w}x{img.h} opaque={len(opaque)} bbox=({min(xs)},{min(ys)})-({max(xs)},{max(ys)}) "
                       f"colours={len(cols)} max_luma={max(_luma(*c) for c in cols):.0f} "
                       f"transparent_px={img.w * img.h - len(opaque)}")
    fig = camp_figure_sentry()
    vals = [_luma(*p) for row in fig.px for p in row if p]
    out.append(f"camp_figure_sentry mean_luma={sum(vals) / len(vals):.1f} (target <= 45: never brighter than the heroes)")
    wreck = aft_mirror_wreck()
    hits = [(x, y) for (x0, x1) in CLEAR_COLS for x in range(x0, x1 + 1) for y in range(CLEAR_ROWS) if wreck.get(x, y)]
    out.append(f"aft_mirror_wreck torch_column_pixels_above_row_{CLEAR_ROWS}={len(hits)} (target 0)")
    frames = aft_heavy_bag_frames()
    for n, f, dx in zip(BAG_FRAMES, frames, BAG_SWAY):
        body = [x for y in range(BAG_BODY[1], BAG_BODY[3] + 1) for x in range(BAG_W) if f.get(x, y)]
        out.append(f"aft_heavy_bag_{n} sway={dx:+d} body_x={min(body)}..{max(body)} hook_row0={[x for x in range(BAG_W) if f.get(x, 0)]}")
    out.append(f"sway_table ticks={len(SWAY_TABLE)} frames={''.join(s[0] if s != 'rest' else '.' for s in SWAY_TABLE)}")
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
# The gate. Stills are 640 x 360 window frames (view top y 292) of the two rooms with the walk pilot, taken
# with a scratch probe build (greybox decor rectangles off; the dressing of spec section 9 applied from a
# scratch contract with the reused pieces only, plus a light-only stand-in for the camp fire) twice per
# room: with actors and with the actor sprites not spawned (same camera, same tick). The keys are composed
# at 1x over the actor-free still and every pixel where the two stills differ is re-pasted from the actor
# still (actors draw over props).
# =============================================================================================
VIEW_TOP = 292
HEARTH_AT = (720, 496)
FIRE_PLACE = (HEARTH_AT[0] + FIRE_AT[0], HEARTH_AT[1] + FIRE_AT[1])
SENTRY_AT = (500, 512)
WRECK_AT = (368, 488)
BAG_AT = (620, 450)
GATE_VIEWS = {
    # name: (still stem, tick, view left, keys, fire frame, bag frame). The view left of each still is
    # measured from the torch flames in it (torch_wall flame centre = placement x + 7.5): tick 41 -> 0,
    # tick 161 -> 237, tick 281 -> 607. The ticks are picked so the heroes do not cover the hero keys.
    "camp_entrance": ("rest_camp", 41, 0, ("sentry",), 0, 0),
    "camp_hearth": ("rest_camp", 281, 607, ("hearth",), 1, 0),
    "aft_wreck": ("rest_aftermath", 161, 237, ("wreck", "bag"), 0, 2),
    "aft_lane": ("rest_aftermath", 281, 607, ("bag",), 0, 0),
}


def _still(caps: Path, room: str, mode: str, tick: int):
    from PIL import Image
    hit = sorted((caps / f"{room}-probe-{mode}").glob(f"frame_*_tick_{tick:05d}.png"))
    return Image.open(hit[0]).convert("RGBA")


def gate(caps: Path, out_dir: Path) -> None:
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    pad = 8
    all_keys = [fn() for pieces in KEYS.values() for _, fn in pieces]
    for silhouette, name in ((False, "keys"), (True, "silhouettes")):
        imgs = [to_image(k, silhouette) for k in all_keys]
        w = sum(i.width for i in imgs) + pad * (len(imgs) + 1)
        h = max(i.height for i in imgs) + pad * 2
        sheet = Image.new("RGBA", (w, h), (205, 198, 186, 255) if silhouette else (40, 36, 48, 255))
        x = pad
        for i in imgs:
            sheet.alpha_composite(i, (x, h - pad - i.height))
            x += i.width + pad
        sheet.save(out_dir / f"{name}_1x.png")
        sheet.resize((w * 4, h * 4), Image.NEAREST).save(out_dir / f"{name}_4x.png")
    # the bag sway strip: the five frames, then the 38-tick table as a strip of thumbnails
    frames = {n: to_image(f) for n, f in zip(BAG_FRAMES, aft_heavy_bag_frames())}
    s = Image.new("RGBA", (5 * (BAG_W + pad) + pad, BAG_H + 2 * pad), (40, 36, 48, 255))
    for k, n in enumerate(BAG_FRAMES):
        s.alpha_composite(frames[n], (pad + k * (BAG_W + pad), pad))
    s.save(out_dir / "bag_sway_1x.png")
    s.resize((s.width * 3, s.height * 3), Image.NEAREST).save(out_dir / "bag_sway_3x.png")
    fires = [to_image(f) for f in camp_fire_frames()]
    s = Image.new("RGBA", (4 * (FIRE_W + pad) + pad, FIRE_H + 2 * pad), (40, 36, 48, 255))
    for k, f in enumerate(fires):
        s.alpha_composite(f, (pad + k * (FIRE_W + pad), pad))
    s.resize((s.width * 4, s.height * 4), Image.NEAREST).save(out_dir / "fire_loop_4x.png")

    imgs = {"hearth": to_image(camp_hearth()), "sentry": to_image(camp_figure_sentry()),
            "wreck": to_image(aft_mirror_wreck())}
    places = {"hearth": HEARTH_AT, "sentry": SENTRY_AT, "wreck": WRECK_AT}
    for name, (room, tick, vx, keys, fire_k, bag_k) in GATE_VIEWS.items():
        lit, bare = _still(caps, room, "walk", tick), _still(caps, room, "noactors", tick)
        comp = bare.copy()
        for key in keys:
            if key == "bag":
                comp.alpha_composite(frames[BAG_FRAMES[bag_k]], (BAG_AT[0] - vx, BAG_AT[1] - VIEW_TOP))
                continue
            comp.alpha_composite(imgs[key], (places[key][0] - vx, places[key][1] - VIEW_TOP))
            if key == "hearth":
                comp.alpha_composite(fires[fire_k], (FIRE_PLACE[0] - vx, FIRE_PLACE[1] - VIEW_TOP))
        for y in range(comp.height):                                # actors over props
            for x in range(comp.width):
                p, q = lit.getpixel((x, y)), bare.getpixel((x, y))
                if p != q:
                    comp.putpixel((x, y), p)
        comp.save(out_dir / f"keys_in_{name}_1x.png")
        comp.resize((1280, 720), Image.NEAREST).save(out_dir / f"keys_in_{name}_2x.png")
    (out_dir / "measurements.txt").write_text("\n".join(measurements()) + "\n", encoding="utf-8")


def gate_board(out_dir: Path, dest: Path) -> None:
    """One board for the gate reviewer: the four scenes at 1x (2 x 2), then the keys at 3x and the sway strip."""
    from PIL import Image
    scenes = [Image.open(out_dir / f"keys_in_{n}_1x.png") for n in GATE_VIEWS]
    keys = Image.open(out_dir / "keys_1x.png")
    keys = keys.resize((keys.width * 2, keys.height * 2), Image.NEAREST)
    sway = Image.open(out_dir / "bag_sway_1x.png")
    sway = sway.resize((sway.width * 2, sway.height * 2), Image.NEAREST)
    w = max(1280 + 8, keys.width, sway.width) + 16
    h = 720 + 8 + keys.height + 8 + 16
    board = Image.new("RGBA", (w, h), (24, 22, 30, 255))
    for k, sc in enumerate(scenes):
        board.paste(sc, (8 + (k % 2) * 648, 8 + (k // 2) * 368))
    y = 8 + 2 * 368
    board.alpha_composite(keys, (8, y))
    if keys.width + sway.width + 24 <= w:
        board.alpha_composite(sway, (keys.width + 16, y))
    board.save(dest)




# =============================================================================================
# EXECUTION (AREA-CAMP-01, spec section 13): the executor pieces of spec section 5 (rows marked E), drawn below
# the frozen keys with the same figure language (hood void gal 0, coat gal 1..3 with folds, a warm rim gal_warm
# 1..2 only on the side facing the hearth, wrapped hands wood 3, boots iron 1, mean luma <= 45), the atlases,
# the manifests, the two kit contracts and the dressing. The key functions above are untouched.
# =============================================================================================
def _fig_fill(c: Pix, spans: dict, warm_left: bool, seed: str, fold_from: int = 99, fold_w: int = 4) -> None:
    """Fill a coat mass: ``spans`` is {row: (x0, x1)}. The hearth-facing edge column takes a warm rim
    (gal_warm 1..2); the other edge a cool rim (gal 3); a darker terminator on the far side; long folds."""
    for y, (x0, x1) in spans.items():
        for x in range(x0, x1 + 1):
            warm_edge = x == (x0 if warm_left else x1)
            cool_edge = x == (x1 if warm_left else x0)
            v = 2.0
            if cool_edge:
                v += 0.9
            elif x == (x1 - 1 if warm_left else x0 + 1):
                v -= 0.5
            if y >= fold_from and (x - x0) % fold_w == fold_w - 1:
                v -= 0.8
            if rnd("coat", seed, x, y) > 0.9:
                v -= 0.5
            if warm_edge:
                c.put(x, y, "gal_warm", 1 if (y + x) % 3 else 2)
            else:
                c.put(x, y, "gal", v)


def _hood(c: Pix, spans: dict, void: list, warm_left: bool, seed: str) -> None:
    """A rounded hood; the opening is a flat void (gal 0), never a feature."""
    for y, (x0, x1) in spans.items():
        for x in range(x0, x1 + 1):
            top = y == min(spans)
            warm_edge = x == (x0 if warm_left else x1) and not top
            v = 2.4 + (1.0 if top else 0) - (0.5 if x == (x1 if warm_left else x0) else 0)
            if warm_edge:
                c.put(x, y, "gal_warm", 1 if y % 2 else 2)
            else:
                c.put(x, y, "gal", v)
    for (x, y) in void:
        c.put(x, y, "gal", 0)


def _wrap_hand(c: Pix, x0: int, y0: int, x1: int, y1: int) -> None:
    """A wrapped hand: wood 3 with a darker binding row."""
    c.box(x0, y0, x1, y1, "wood", 3)
    c.put(x0, y0, "wood", 4)
    for x in range(x0, x1 + 1):
        c.put(x, y1, "wood", 2)


def _boots(c: Pix, y: int, spans: list) -> None:
    for (x0, x1) in spans:
        for x in range(x0, x1 + 1):
            c.put(x, y, "wood", 1)
            c.put(x, y + 1, "iron", 1)


def camp_figure_repair() -> Pix:
    """24 x 36: a figure kneeling over a pack, mending it with a cord, a shuttered tin lamp beside it (gate note
    G5: iron 2..4 body, one 2 x 2 flame-3 slit, no flame 4/5). Faces right; the hearth is to the left."""
    c = Pix(24, 36)
    # the shuttered lamp, left
    for k, (x, y) in enumerate(((1, 25), (2, 24), (3, 24), (4, 25))):     # the bail
        c.put(x, y, "iron", 3)
    c.box(0, 26, 5, 27, "iron", 3)                                         # the hood of the lamp
    c.put(0, 26, "iron", 2)
    c.box(0, 28, 5, 34, "iron", 2)
    for y in range(28, 35):
        c.put(0, y, "iron", 4)                                             # the lit edge
        c.put(5, y, "iron", 1)
    c.box(2, 30, 3, 31, "flame", 3)                                        # the 2 x 2 slit
    c.box(0, 35, 5, 35, "iron", 1)
    # the pack on the ground, right
    for y in range(27, 36):
        for x in range(18, 24):
            if (x, y) in ((18, 27), (23, 27)):
                continue
            v = 2.4 + (0.8 if y == 27 else 0) - (0.5 if x == 23 else 0)
            c.put(x, y, "wood", v)
    c.box(19, 30, 22, 32, "stone", 3)                                      # a lighter patch
    for x in range(18, 24):
        c.put(x, 29, "wood", 1)                                            # the strap
    c.put(21, 33, "stone", 5)                                              # the needle's thread
    c.put(22, 34, "stone", 5)
    # the back leg (a long shin on the ground), then the front leg and boots
    for y in (32, 33):
        for x in range(6, 15):
            c.put(x, y, "gal", 1)
    _boots(c, 34, [(3, 7)])
    for y in range(28, 33):
        for x in range(14, 18):
            c.put(x, y, "gal", 1.4 if x < 17 else 0.8)
    _boots(c, 33, [(14, 18)])
    # the bent back and coat
    spans = {}
    for y in range(17, 32):
        spans[y] = (5 - (1 if y > 22 else 0) - (1 if y > 27 else 0), 15 + (1 if y > 20 else 0) + (1 if y > 26 else 0))
    _fig_fill(c, spans, True, "rep", fold_from=24)
    c.box(6, 26, 16, 26, "wood", 1)                                        # the rope belt
    # the hood, bowed to the work; the opening faces right
    _hood(c, {9: (10, 14), 10: (9, 15), 11: (8, 16), 12: (8, 16), 13: (8, 17), 14: (8, 17), 15: (9, 17),
              16: (9, 16)}, [(16, 12), (17, 13), (17, 14), (16, 13), (16, 14), (16, 15), (15, 13), (15, 14)],
          True, "rephood")
    # the arm from the shoulder to the wrapped hand on the pack, a cord drawn tight
    for i, (x, y) in enumerate(((16, 18), (17, 19), (18, 21), (18, 23), (19, 25))):
        c.put(x, y, "gal", 3 if i < 2 else 2)
        c.put(x - 1, y, "gal", 1)
    _wrap_hand(c, 19, 26, 21, 28)
    c.line(21, 29, 23, 33, "stone", 5)
    return c


def camp_figure_seated() -> Pix:
    """20 x 30: a figure sitting on a low crate, the hood bunched on its shoulders over a bare dark head, both
    wrapped hands around a cup. Faces left (the fire); the hearth is to the left."""
    c = Pix(20, 30)
    for y in range(21, 30):                                                # the low crate
        for x in range(3, 17):
            v = 2.6 + (1.0 if y == 21 else 0) - (0.8 if x >= 15 else 0)
            if y in (24, 27):
                v = 1.2
            c.put(x, y, "wood", v)
    for y in range(22, 29):
        c.put(3, y, "wood", 4)                                             # the lit end grain edge
    c.put(9, 25, "iron", 3)                                                # a nail head
    # the seated body: torso, thighs forward, shins down
    spans = {y: (5, 14 + (1 if y < 14 else 0)) for y in range(12, 21)}
    _fig_fill(c, spans, True, "sea", fold_from=16, fold_w=3)
    for y in (19, 20, 21):                                                 # the thighs
        for x in range(1, 10):
            c.put(x, y, "gal", 1.8 if y < 21 else 1)
    c.put(1, 19, "gal_warm", 1)
    c.put(1, 20, "gal_warm", 2)
    for y in range(22, 28):                                                # the shins
        for x in (1, 2, 3):
            c.put(x, y, "gal", 1 if x > 1 else 0.5)
        c.put(1, y, "gal_warm", 1)
    _boots(c, 28, [(0, 4)])
    # the head: a bare dark oval and the hood heaped on the shoulders
    for y, (x0, x1) in {3: (7, 10), 4: (6, 11), 5: (6, 11), 6: (6, 11), 7: (6, 11), 8: (7, 11)}.items():
        for x in range(x0, x1 + 1):
            c.put(x, y, "gal", 0 if x < 10 else 1)
    for x in range(7, 11):
        c.put(x, 2, "gal", 3)
    for y, (x0, x1) in {9: (4, 14), 10: (4, 15), 11: (4, 15)}.items():     # the bunched hood
        for x in range(x0, x1 + 1):
            c.put(x, y, "gal_warm" if x == x0 else "gal", (1 if y % 2 else 2) if x == x0 else 2.6 - 0.4 * (y - 9))
    # the arms and the cup
    for (x, y) in ((6, 12), (5, 13), (4, 14), (3, 15)):
        c.put(x, y, "gal", 2)
        c.put(x + 1, y, "gal", 1)
    c.box(1, 14, 3, 17, "iron", 3)                                         # the cup
    c.put(1, 14, "iron", 5)
    c.put(2, 14, "gal_warm", 2)
    _wrap_hand(c, 0, 16, 1, 18)
    _wrap_hand(c, 4, 16, 5, 18)
    return c


def camp_toy_cart() -> Pix:
    """18 x 12: a salvage toy: a tin-can body on two bottle-cap wheels, a string loop to pull it by."""
    c = Pix(18, 12)
    for y in range(2, 9):                                                  # the can
        for x in range(3, 13):
            v = 3.4 - 0.9 * (x - 3) / 9
            c.put(x, y, "stone_cold", clamp(v, 1, 3))
    c.box(3, 2, 12, 2, "iron", 5)                                          # the rolled rim
    c.box(3, 8, 12, 8, "iron", 3)
    for x in range(4, 12):                                                 # a faded label band
        c.put(x, 5, "brick", 4)
        c.put(x, 6, "brick", 3)
    c.put(13, 4, "iron", 4)                                                # the can's open end, lit
    c.put(13, 5, "iron", 3)
    for cx in (5, 11):                                                     # bottle-cap wheels
        c.disc(cx + 0.5, 9.5, 2.3, "brass", lambda x, y, d: 3 if d < 1.2 else 2)
        c.put(cx, 9, "brass", 4)
        c.put(cx, 10, "brass", 1)
    c.line(2, 6, 0, 9, "wood", 3)                                          # the string loop
    c.line(0, 9, 3, 11, "wood", 2)
    return c


def camp_map_table() -> Pix:
    """44 x 30: a trestle table; the map on it is pale stone with ink lines, blots and pins. No words."""
    c = Pix(44, 30)
    for (lx, leaning) in ((3, 1), (35, -1)):                               # two A-frame trestles
        for y in range(9, 30):
            t = (y - 9) / 20
            for sx in (round(lx + 2 * t * leaning * -1), round(lx + 5 - 2 * t * leaning * -1)):
                c.put(sx, y, "wood", 2 if y % 7 else 1)
                c.put(sx + 1, y, "wood", 3)
        c.box(lx + 1, 19, lx + 5, 19, "wood", 1)                           # the cross brace
    c.box(2, 8, 41, 9, "wood", 3)                                          # the top plank edge
    c.box(2, 9, 41, 9, "wood", 1)
    for x in range(2, 42):
        c.put(x, 8, "wood", 4 if x % 5 else 3)
    for y in range(2, 8):                                                  # the map sheet
        for x in range(3, 41):
            if (x in (3, 40)) and y in (2, 7):
                continue
            v = 5.0 + 0.7 * math.sin(x * 0.4 + y) + (0.8 if y == 2 else 0) - (0.9 if y == 7 else 0)
            c.put(x, y, "stone", clamp(round(v), 4, 7))
    for k in range(7):                                                     # ink lines, a river, a road
        x = 5 + k * 5
        c.line(x, 6 - (k % 3), x + 6, 3 + (k % 2) * 2, "mortar", 1)
    c.line(4, 4, 38, 5, "mortar", 3)
    for (x, y) in ((9, 4), (10, 4), (22, 5), (23, 5), (23, 6), (31, 3), (32, 3), (32, 4)):
        c.put(x, y, "mortar", 0)                                           # blots
    for (x, y) in ((15, 3), (27, 5), (36, 3)):                             # pins
        c.put(x, y, "brick", 7)
        c.put(x, y - 1, "brick", 9)
    c.put(41, 5, "stone", 3)                                               # the curled corner
    c.put(41, 6, "stone", 2)
    return c


def camp_figure_map() -> Pix:
    """20 x 46: a figure standing over the map, leaning on one wrapped hand on the table, a rolled blanket
    on its back. Faces left; the hearth is to the left."""
    c = Pix(20, 46)
    for i in range(20):                                                    # the rolled blanket across the back
        x, y = 14 + i // 6, 13 + i
        for dx in (0, 1):
            c.put(x + dx, y, "stone", 1.6 if dx == 0 else 2.2)
    spans = {}
    for y in range(12, 41):
        ly = y - 12
        spans[y] = (5 - (1 if ly > 3 else 0) - (1 if ly > 21 else 0), 15 + (1 if ly > 21 else 0))
    _fig_fill(c, spans, True, "map", fold_from=28)
    for x in range(6, 16):
        c.put(x, 25, "wood", 1)                                            # the rope belt
    c.box(8, 29, 11, 33, "stone", 2)                                       # a sewn patch
    c.put(8, 29, "stone", 3)
    _hood(c, {3: (7, 11), 4: (6, 12), 5: (5, 12), 6: (5, 13), 7: (5, 13), 8: (5, 13), 9: (5, 13), 10: (6, 13),
              11: (6, 13)}, [(5, 6), (5, 7), (5, 8), (6, 6), (6, 7), (6, 8), (6, 9), (7, 7), (7, 8)], True, "maph")
    for i, (x, y) in enumerate(((6, 14), (5, 15), (4, 16), (3, 17), (3, 18), (2, 19))):   # the reaching arm
        c.put(x, y, "gal", 3 if i < 3 else 2)
        c.put(x + 1, y, "gal", 1)
    _wrap_hand(c, 0, 20, 4, 22)
    for y in range(40, 44):                                                # legs
        for x in (6, 7, 8, 9):
            c.put(x, y, "gal", 1)
        for x in (11, 12, 13):
            c.put(x, y, "gal", 1)
    c.put(6, 41, "gal_warm", 1)
    c.put(6, 42, "gal_warm", 1)
    _boots(c, 44, [(5, 10), (11, 15)])
    return c


def _stroke(c: Pix, pts: list, ramp: str, v: int, seed: str, gap: float = 0.1, width: int = 1) -> None:
    """A rough painted/chalk stroke along a polyline with ragged gaps (rules, not a trace)."""
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            x = round(x0 + (x1 - x0) * i / n)
            y = round(y0 + (y1 - y0) * i / n)
            if rnd("stk", seed, x, y) < gap:
                continue
            for d in range(width):
                c.put(x, y + d, ramp, v)


def camp_motifs() -> Pix:
    """80 x 48: crude strength motifs painted on the brick: a kettlebell, a bar with plates, a flexed arm. Brick
    paint (brick 7..9) with a chalk outline (stone 8); strokes only, so the wall shows through."""
    c = Pix(80, 48)
    # kettlebell: a round body and a handle arch
    for a in range(0, 360, 6):
        x = round(13 + 9.5 * math.cos(math.radians(a)))
        y = round(32 + 8.5 * math.sin(math.radians(a)))
        if rnd("kb", a) > 0.12:
            c.put(x, y, "brick", 8)
            c.put(x + 1, y, "brick", 7)
    for a in range(200, 341, 5):
        x = round(13 + 7 * math.cos(math.radians(a)))
        y = round(21 + 6 * math.sin(math.radians(a)))
        c.put(x, y, "brick", 8)
        c.put(x, y + 1, "brick", 7)
    for a in range(20, 160, 14):                                           # a chalk glint on the body
        c.put(round(13 + 6.5 * math.cos(math.radians(a))), round(32 + 5.5 * math.sin(math.radians(a))), "stone", 8)
    # barbell: bar, two plate stacks, collars
    _stroke(c, [(28, 31), (54, 30)], "brick", 8, "bar", 0.06, 2)
    for px in (30, 50):
        _stroke(c, [(px, 20), (px + 1, 40)], "brick", 8, f"pl{px}", 0.08, 3)
        _stroke(c, [(px + 4, 23), (px + 4, 37)], "brick", 7, f"pm{px}", 0.1, 2)
    _stroke(c, [(29, 19), (32, 19)], "stone", 8, "pc1", 0.0)
    _stroke(c, [(49, 19), (52, 19)], "stone", 8, "pc2", 0.0)
    # flexed arm: forearm up, a bicep bump, the fist
    _stroke(c, [(60, 42), (62, 31), (66, 25)], "brick", 8, "arm1", 0.07, 3)
    _stroke(c, [(66, 25), (70, 22), (74, 25), (73, 30), (68, 31)], "brick", 8, "arm2", 0.08, 2)
    for a in range(180, 361, 12):                                          # the bicep peak
        x = round(70 + 5 * math.cos(math.radians(a)))
        y = round(24 + 4 * math.sin(math.radians(a)))
        c.put(x, y, "stone", 8)
    c.box(58, 38, 63, 41, "brick", 9)                                      # the fist
    c.put(58, 38, "brick", 7)
    # paint drips
    for (x, y, n) in ((9, 40, 4), (17, 41, 3), (33, 41, 4), (55, 39, 3), (66, 33, 4)):
        for k in range(n):
            c.put(x, y + k, "brick", 7)
    return c


def camp_chalk_tally() -> Pix:
    """24 x 40: chalk tally strokes in groups of four with a diagonal, and an arrow toward the exit door."""
    c = Pix(24, 40)
    for gy, gx in ((3, 2), (3, 13), (15, 5)):
        for k in range(4):
            x = gx + k * 3
            _stroke(c, [(x, gy), (x + (k % 2), gy + 8)], "stone", 8 if k % 2 else 7, f"t{gx}{gy}{k}", 0.12)
        _stroke(c, [(gx - 1, gy + 7), (gx + 11, gy + 1)], "stone", 7, f"d{gx}{gy}", 0.1)
    # the arrow: a shaft and a head, pointing right
    _stroke(c, [(1, 33), (19, 33)], "stone", 8, "arr", 0.08, 2)
    _stroke(c, [(14, 28), (20, 33), (14, 38)], "stone", 8, "arh", 0.05, 2)
    for k in range(3):                                                     # chalk dust below
        c.put(4 + k * 5, 38, "stone", 4)
    return c


def camp_chalk_floor() -> Pix:
    """128 x 12: chalk chevrons and a dashed line on the deck's back half, pointing right to the exit. Strokes
    are at most 3 px tall (the lane rule, spec 10.3)."""
    c = Pix(128, 12)
    for k in range(5):
        x = 6 + k * 24
        for i in range(3):
            c.put(x + i, 3 + i, "stone", 7 if i != 1 else 8)
            c.put(x + i, 7 - i, "stone", 7 if i != 1 else 8)
        c.put(x + 3, 5, "stone", 8)
    for x in range(10, 126):
        if (x // 6) % 2 == 0 and x % 24 not in range(0, 8) and rnd("dash", x) > 0.1:
            c.put(x, 5, "stone", 6)
    for x in range(0, 128, 9):
        if rnd("dust", x) > 0.5:
            c.put(x, 9, "stone", 4)
    return c


# ----------------------------------------------------------------------------------------- aftermath
def aft_bench_bent() -> Pix:
    """48 x 20 from the gym bench's own pixels: the seat folded at a kink (the right half dropped 20 degrees),
    one leg sheared to a stub, a crack at the fold."""
    b = gym.bench()                                                        # 44 x 18
    c = Pix(48, 20)
    ox, oy = 2, 2
    kx, ky = 22, 9                                                         # the pivot in bench coordinates
    ang = math.radians(20)
    ca, sa = math.cos(ang), math.sin(ang)
    for y in range(18):
        for x in range(kx):
            if b.px[y][x] is not None:
                c.put(x + ox, y + oy, *b.px[y][x])
    for ty in range(20):
        for tx in range(48):
            dx, dy = tx - ox - kx, ty - oy - ky
            sx, sy = dx * ca + dy * sa, -dx * sa + dy * ca                 # inverse of the clockwise drop
            ix, iy = round(sx) + kx, round(sy) + ky
            if ix >= kx and 0 <= ix < 44 and 0 <= iy < 18 and b.px[iy][ix] is not None:
                c.put(tx, ty, *b.px[iy][ix])
    for y in range(20):                                                    # the sheared leg: a ragged stub
        for x in range(34, 48):
            p = c.px[y][x]
            if p is not None and p[0] == "stone_cold" and y > 13 + (x % 3):
                c.px[y][x] = None
    for y in range(ky + oy - 5, ky + oy + 6):                              # the crack at the fold
        c.put(kx + ox, y, "iron", 0)
    for (x, y) in ((kx + ox - 1, ky + oy + 2), (kx + ox + 1, ky + oy - 3)):
        c.put(x, y, "iron", 1)
    return c


def aft_sand_spill() -> Pix:
    """40 x 8: the sand the split seam has spilled: a low heap under the trickle, a fan of grains."""
    c = Pix(40, 8)
    for x in range(40):
        h = 5.0 * math.exp(-((x - 27) / 7.5) ** 2) + 1.6 * math.exp(-((x - 14) / 9.0) ** 2)
        top = 7 - round(h)
        for y in range(top, 8):
            if y == 7 and rnd("sp", x) < 0.25:
                continue
            v = 7.0 - 0.9 * (y - top) + (0.8 if y == top else 0) - (0.7 if x > 30 else 0)
            c.put(x, y, "stone", clamp(round(v), 3, 9))
    for x in range(0, 40, 3):
        if rnd("gr", x) > 0.55 and c.get(x, 7) is None:
            c.put(x, 7, "stone", 4)
    return c


def aft_lane_chalk() -> Pix:
    """116 x 12: the blink lane's two chalk ticks (x 768..771 and 876..879) and a dashed chevron between them.
    Every stroke is at most 3 px tall."""
    c = Pix(116, 12)
    for x0 in (2, 110):
        for y in (3, 4, 5):
            for x in range(x0, x0 + 4):
                c.put(x, y, "stone", 8 if y == 4 else 7)
    for k in range(4):                                                     # dashed chevrons
        x = 16 + k * 22
        for i in range(3):
            c.put(x + i, 3 + i, "stone", 7)
            c.put(x + i, 7 - i, "stone", 7)
    for x in range(10, 108):
        if (x // 5) % 2 == 1 and x % 22 not in range(14, 22) and rnd("ld", x) > 0.08:
            c.put(x, 5, "stone", 5)
    return c


GROOVE_C = (112, 112)                 # ring centre, piece-local: the exit portal's mid-height (1232, 512)
GROOVE_R = (82.0, 94.0, 106.0)


def aft_groove_rings() -> Pix:
    """160 x 160: concentric arcs cut into the back wall around the exit door: dark mortar cuts 2 px wide with a
    1 px stone 7 lit lip on the inner side of the upper arcs. Each ring is broken in a few places. The portal
    covers the middle; the arcs show round it. (The rings kit's Arc relief, 24 px bands over a 150..208 radius,
    cannot fit a 160 px piece, so the cut is drawn here: CAMP-AD14.)"""
    c = Pix(160, 160)
    cx, cy = GROOVE_C
    for k, R in enumerate(GROOVE_R):
        for y in range(160):
            for x in range(160):
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                r = math.hypot(dx, dy)
                a = math.degrees(math.atan2(dx, -dy)) % 360
                sector = int(a // 20)
                if rnd("gsec", k, sector) > 0.82:
                    continue                                               # a broken stretch
                if R - 1.0 <= r < R + 1.0:
                    c.put(x, y, "mortar", 0 if r < R else 1)
                elif R - 2.0 <= r < R - 1.0 and dy < 20:
                    c.put(x, y, "stone", 7 if rnd("lip", x, y) > 0.2 else 6)
    return c


# ----------------------------------------------------------------------------------------- atlases
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


def _bag(n: str):
    return lambda i=BAG_FRAMES.index(n): aft_heavy_bag_frames()[i]


# stem = the source file stem under assets/source/props/floor01; the key grids of the design stage keep their stems
ATLASES = {
    "floor01_camp_hearth": {"stem": "camp_hearth", "width": 96, "pieces": [("camp_hearth", camp_hearth)]},
    "floor01_camp_figures": {"stem": "camp_figure", "width": 112, "pieces": [
        ("camp_figure_sentry", camp_figure_sentry), ("camp_figure_repair", camp_figure_repair),
        ("camp_figure_seated", camp_figure_seated), ("camp_figure_map", camp_figure_map),
        ("camp_toy_cart", camp_toy_cart), ("camp_map_table", camp_map_table)]},
    "floor01_camp_wall": {"stem": "camp_wall", "width": 128, "pieces": [
        ("camp_motifs", camp_motifs), ("camp_chalk_tally", camp_chalk_tally), ("camp_chalk_floor", camp_chalk_floor)]},
    "floor01_aft_wreck": {"stem": "aft_wreck", "width": 160, "pieces": [
        ("aft_mirror_wreck", aft_mirror_wreck), ("aft_bench_bent", aft_bench_bent), ("aft_sand_spill", aft_sand_spill),
        ("aft_lane_chalk", aft_lane_chalk)]},
    "floor01_aft_bag": {"stem": "aft_heavy_bag", "width": 200, "pieces": [
        (f"aft_heavy_bag_{n}", _bag(n)) for n in BAG_FRAMES]},
    "floor01_aft_grooves": {"stem": "aft_grooves", "width": 160, "pieces": [("aft_groove_rings", aft_groove_rings)]},
}
FIRE_ATLAS = "floor01_camp_fire"       # the 4-frame loop: one canvas per frame (the torch pattern, tag "burn")
KIT_OF_ATLAS = {"floor01_camp_hearth": "camp", "floor01_camp_fire": "camp", "floor01_camp_figures": "camp",
                "floor01_camp_wall": "camp", "floor01_aft_wreck": "aft", "floor01_aft_bag": "aft",
                "floor01_aft_grooves": "aft"}
LAYER_OF_ATLAS = {"floor01_camp_hearth": ("prop", "prop"), "floor01_camp_figures": ("prop", "prop"),
                  "floor01_camp_wall": ("backwall", "backwall_decal"), "floor01_aft_wreck": ("prop", "prop"),
                  "floor01_aft_bag": ("prop", "prop"), "floor01_aft_grooves": ("backwall", "backwall_decal")}
# A piece that is not on its atlas's layer: the floor chalk and the spill draw above the collision deck (FF-AD26)
LAYER_OVERRIDE = {"camp_chalk_floor": ("structure", "structure"), "aft_sand_spill": ("structure", "structure"),
                  "aft_lane_chalk": ("structure", "structure")}
PRESENTER_PIECES = {f"aft_heavy_bag_{n}" for n in BAG_FRAMES}
EXEC_PIECES = ("camp_figure_repair", "camp_figure_seated", "camp_toy_cart", "camp_map_table", "camp_figure_map",
               "camp_motifs", "camp_chalk_tally", "camp_chalk_floor", "aft_bench_bent", "aft_sand_spill",
               "aft_lane_chalk", "aft_groove_rings")
KEY_PIECE_IDS = tuple(pid for pieces in KEYS.values() for pid, _ in pieces)
FIGURES = ("camp_figure_sentry", "camp_figure_repair", "camp_figure_seated", "camp_figure_map")

CAMP_CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_camp_kit.json"
AFT_CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_aftermath_kit.json"
MANIFESTS = ROOT / "assets/manifests"
OWNER = ("AI-authored environment pixel art (AREA-CAMP-01: design spec, keys and silhouette gate Claude Opus; "
         "execution Claude Sonnet; independent review pending)")
FIRE_LIGHTS = [
    {"id": "flame", "offset": [10, 22], "colour": "#ffb060", "radius": 240, "intensity": 0.40,
     "glow": {"radius": 40, "additive": 0.08}, "flicker": [1.0, 0.9, 1.08, 0.95], "group": "scene"},
    {"id": "floor_pool", "offset": [10, 22], "project": "walk_line", "project_dy": -6, "squash_y": 0.32,
     "colour": "#ffa95e", "radius": 150, "intensity": 0.34, "flicker": [1.0, 0.9, 1.08, 0.95], "group": "scene"},
]

# -- the dressing (spec sections 4 and 5, with the gate notes G1, G3, G4) --------------------------------------
CAMP_REMOVE_AT = [("prop", "torch_wall", 368, 448), ("backwall_decal", "torch_pool_b", 264, 364),
                  ("prop", "torch_wall", 704, 448), ("backwall_decal", "torch_pool_a_m", 600, 364),
                  ("prop", "torch_wall", 760, 432), ("backwall_decal", "torch_pool_a_m", 656, 348),
                  ("prop", "torch_wall", 800, 448), ("backwall_decal", "torch_pool_a_m", 696, 364)]
CART_X = 1048                           # gate note G4: the opaque gap to the seated figure is 4..8 px (measured below)
MAP_FIGURE_AT = (1152, 514)             # gate note G3: its reaching hand draws over the table's top (x 1152..1157)
CAMP_PLACE = [
    ("backwall_decal", "camp_motifs", 1344, 384), ("backwall_decal", "camp_chalk_tally", 1404, 488),
    ("structure", "camp_chalk_floor", 1320, 561),
    ("prop", "ring_marker_a", 156, 504), ("prop", "ring_camp_a", 352, 488), ("prop", "camp_figure_sentry", 500, 512),
    ("prop", "ring_crates_b", 652, 532), ("prop", "camp_hearth", 720, 496), ("prop", "camp_fire", 758, 526),
    ("prop", "camp_figure_repair", 872, 524), ("prop", "camp_figure_seated", 1024, 530),
    ("prop", "camp_toy_cart", CART_X, 548), ("prop", "camp_map_table", 1112, 530),
    ("prop", "camp_figure_map", MAP_FIGURE_AT[0], MAP_FIGURE_AT[1]),
]
AFT_REMOVE_AT = [("structure", "arch_chain", 560, 384), ("backwall_decal", "torch_pool_b_m", 1112, 348)]
AFT_FILL = [{"x": 512, "y": 352, "w": 208, "h": 208}]
# G1: a sixth girder at x 684, listed before the one at 676 so the fifth draws over it; only its columns 24..31
# (the plain web and the butt joint at 715) show, meeting the pier's shadow column at 716.
GIRDERS = [684, 548, 580, 612, 644, 676]
AFT_PLACE = [
    ("backwall_decal", "aft_groove_rings", 1120, 400), ("backwall_decal", "torch_pool_b_m", 1112, 348),
    *[("structure", "gym_girder", x, 440) for x in GIRDERS],
    ("structure", "aft_sand_spill", 622, 562), ("structure", "aft_lane_chalk", 766, 561),
    ("prop", "gym_tyres", 236, 542), ("prop", "aft_bench_bent", 284, 540), ("prop", "aft_mirror_wreck", 368, 488),
    ("prop", "gym_plate_tree", 1000, 524),
]
BAG_AT = (620, 450)                     # the bag's top-left (state group origin, drawn by the presenter)


def _at(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def dressing(kit: str) -> dict:
    if kit == "camp":
        return {"rest_camp": {
            "remove": [], "remove_at": [_at(*r) for r in CAMP_REMOVE_AT], "backwall_fill": [],
            "place": [_at(*t) for t in CAMP_PLACE], "parallax": {"parallax_far": "ring_tiers_far",
                                                                  "parallax_mid": "ring_tiers_mid"}}}
    return {"rest_aftermath": {
        "remove": [], "remove_at": [_at(*r) for r in AFT_REMOVE_AT], "backwall_fill": AFT_FILL,
        "place": [_at(*t) for t in AFT_PLACE], "parallax": {}}}


def build_pieces(name: str) -> list:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def fire_images() -> list:
    return camp_fire_frames()


def _opaque_all(img: Pix) -> bool:
    return all(p is not None for row in img.px for p in row)


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = LAYER_OVERRIDE.get(pid) or LAYER_OF_ATLAS[atlas]
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0], "anchor_name": "top_left (room placement, spec section 5)", "grid_snap": 1,
             "grammar": ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    if pid in PRESENTER_PIECES:
        entry["state_of"] = {"group": "aft_heavy_bag", "state": pid.rsplit("_", 1)[1]}
    return entry


def fire_entry() -> dict:
    frames = [{"frame": {"x": FIRE_W * k, "y": 0, "w": FIRE_W, "h": FIRE_H}, "duration": FIRE_DURATIONS[k]}
              for k in range(4)]
    return {"id": "camp_fire", "atlas": FIRE_ATLAS, "rect": [0, 0, FIRE_W, FIRE_H], "role": "prop", "layer": "prop",
            "collidable": False, "anchor": [10, 22], "anchor_name": "flame_base_centre", "grid_snap": 1,
            "grammar": ["A"], "lights": FIRE_LIGHTS,
            "animation": {"tag": {"name": "burn", "from": 0, "to": 3, "direction": "forward"}, "frames": frames}}


def contract(kit: str, rects: dict, colours: dict) -> dict:
    names = [n for n in list(ATLASES) + [FIRE_ATLAS] if KIT_OF_ATLAS[n] == kit]
    atlases, pieces = {}, []
    for name in names:
        stem = "camp_fire" if name == FIRE_ATLAS else ATLASES[name]["stem"]
        atlases[name] = {"grid": f"assets/source/props/floor01/{stem}.grid.json",
                         "manifest": f"assets/manifests/{name}.json", "image": f"generated/art/{name}.png",
                         "metadata": f"generated/art/{name}.json"}
        if name == FIRE_ATLAS:
            pieces.append(fire_entry())
            continue
        for pid, _ in ATLASES[name]["pieces"]:
            pieces.append(piece_entry(pid, name, rects[name][pid]))
    camp = kit == "camp"
    out = {
        "format": "env-kit-v2",
        "kit_id": "floor01_camp" if camp else "floor01_aftermath",
        "extends": ("assets/source/terrain/floor01/floor01_rings_kit.json" if camp
                    else "assets/source/terrain/floor01/floor01_gym_kit.json"),
        "status": "proposed - AREA-CAMP-01 kit (docs/02-art/area-camp-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": (
            "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema from the "
            "ENV-01 contract through the rings kit. Adds the survivors' camp atlases for Floor 01 room 8 "
            "(rest_camp): no tiles, nothing collidable, no new palette colour. The only new emitter is the "
            "camp fire (spec section 8); the dressing removes four torches and their pools."
            if camp else
            "Inherits palette, layers, coordinates, walking-line rule and lighting schema from the ENV-01 "
            "contract through the gym kit. Adds the quiet aftermath atlases for Floor 01 room 10 "
            "(rest_aftermath): no tiles, nothing collidable, no new palette colour, no new emitter. The heavy "
            "bag is drawn by the presenter."),
        "palette_use": {n: colours[n] for n in names},
        "atlases": atlases, "tiles": [], "pieces": pieces,
    }
    if not camp:
        out["state_groups"] = {"aft_heavy_bag": {
            "states": list(BAG_FRAMES), "anchor": list(BAG_AT),
            "note": "the heavy bag (spec section 7), five 40 x 122 frames at (620, 450): rest, r1, r2, l1, l2. "
                    "The dressing does NOT place it: the presenter (src/bin/floor01_scene/camp_art.rs) draws the "
                    "frame src/camp_kit.rs picks from the 38-tick sway table. Presentation only; the sim's "
                    "hit_target stays inert."}}
    out["room_dressing"] = dressing(kit)
    out["usage_rules"] = {("R1_rest_camp" if camp else "R1_rest_aftermath"):
                          ("room `rest_camp` only: the dressing never adds or moves a tile or a solid; props stand on "
                           "the floor back line (y 559); nothing stands across a window above y 520; floor chalk is "
                           "`structure` so it draws above the deck" if camp else
                           "room `rest_aftermath` only: the dressing never adds or moves a tile or a solid; B2 is "
                           "bricked up across both pier columns; floor decals are `structure`; the bag is not placed")}
    return out


def manifest(name: str, canvas: list, stem: str) -> dict:
    camp = KIT_OF_ATLAS[name] == "camp"
    anim = name == FIRE_ATLAS
    return {
        "schema_version": 1, "id": f"{name}_v1", "type": "prop_animation" if anim else "decor_atlas",
        "source_path": f"assets/source/props/floor01/{stem}.ase",
        "grid_source": f"assets/source/props/floor01/{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png", "output_metadata": f"generated/art/{name}.json",
        "palette_id": "floor01-catacomb-v2", "frame_size": canvas, "pivot": [0, 0],
        "animation_tags": ["burn"] if anim else [], "required_alpha": True,
        "kit_contract": f"assets/source/terrain/floor01/{'floor01_camp_kit.json' if camp else 'floor01_aftermath_kit.json'}",
        "owner": OWNER, "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 pieces for the Floor 1 quiet rooms rest_camp and rest_aftermath "
                      "(AREA-CAMP-01 on the ENV-01 kit); rule-based shapes and ramps, the aftermath wreck and bench "
                      "derived from AREA-GYM-01 pixels; nothing cropped, traced or resampled from concept art; no "
                      "text, logos, brands, faces or likenesses",
            "generator": "Deterministic authoring script scripts/author_camp01.py written by AI agents (spec and "
                         "keys designed by Claude Opus; execution by Claude Sonnet); no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1/08-circular-maze-survivor-camp and the juicer gym panorama (mood reference "
                "only; no pixel cropped, traced or resampled)"]},
    }


def write_kit() -> None:
    """Write every atlas grid + .ase, manifest and the two contracts; print the key pins."""
    rects, colours = {}, {}

    def emit(name, grid, stem):
        path = SRC / f"{stem}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{stem}.ase")], check=True)
        colours[name] = len(grid["palette"])
        (MANIFESTS / f"{name}.json").write_text(json.dumps(manifest(name, grid["canvas"], stem), indent=1) + "\n",
                                                encoding="utf-8")
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")

    for name, spec in ATLASES.items():
        sheet, r = _pack(build_pieces(name), spec["width"])
        rects[name] = r
        emit(name, kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0)), spec["stem"])
    emit(FIRE_ATLAS, kit.grid_from(fire_images(), layer_names=["main"], durations=list(FIRE_DURATIONS), pivot=(0, 0),
                                   tags=[{"name": "burn", "from": 0, "to": 3, "direction": "forward"}]),
         "camp_fire")
    CAMP_CONTRACT.write_text(json.dumps(contract("camp", rects, colours), indent=1) + "\n", encoding="utf-8")
    AFT_CONTRACT.write_text(json.dumps(contract("aft", rects, colours), indent=1) + "\n", encoding="utf-8")
    for pid, sha in key_sha().items():
        print(f"KEY {pid} {sha}")


def all_pieces() -> dict:
    """{piece id: Pix} for every piece this order draws (keys, executor pieces, the five bag frames)."""
    out = {pid: fn() for pieces in KEYS.values() for pid, fn in pieces}
    for name in ATLASES:
        out.update(dict(build_pieces(name)))
    return out


def mean_luma(img: Pix) -> float:
    vals = [_luma(*p) for row in img.px for p in row if p]
    return sum(vals) / len(vals)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "gate":
        gate(Path(sys.argv[2]), Path(sys.argv[3]))
        if len(sys.argv) > 4:
            gate_board(Path(sys.argv[3]), Path(sys.argv[4]))
    elif len(sys.argv) > 1 and sys.argv[1] == "keys":
        for pid, sha in key_sha().items():
            print(pid, sha)
    else:
        write_kit()
