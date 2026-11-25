#!/usr/bin/env python3
"""Authoring record for AREA-GYM-01: the gym kit of the Floor 01 boss arena (room 5).

Draws every piece of ``docs/02-art/area-gym-01-spec.md`` section 4 on ``floor01-catacomb-v2`` (no new
colour) as five pixel-grid-v1 atlases plus their ``.ase`` sources:

- ``assets/source/props/floor01/gym_wall.grid.json``     mirror bay (hero key), gate lintel, ceiling girder
- ``assets/source/props/floor01/gym_fixture.grid.json``  the two tube fixtures (cold, accent)
- ``assets/source/props/floor01/gym_props.grid.json``    reception desk, turnstile, dumbbell rack, bench,
  plate tree, cable tower, tyres
- ``assets/source/props/floor01/gym_floor.grid.json``    rubber mats a and b
- ``assets/source/props/floor01/gym_exit.grid.json``     the exit door, closed and open

Every pixel comes from rules (shapes, ramps, a hash noise); nothing is traced or resampled from a
concept image. No text, logos or likenesses.

    python3 scripts/author_gym01.py            # grids + .ase
    python3 scripts/author_gym01.py sheet DIR  # 1x and 4x sheets of every atlas
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

RAMPS = kit.RAMPS
SRC = ROOT / "assets/source/props/floor01"


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("gym01",) + key)).encode()).digest()
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

    def get(self, x, y):
        return self.px[y][x] if 0 <= x < self.w and 0 <= y < self.h else None

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


# =============================================================================================
# Hero key: the mirror bay (spec section 10). 96 x 74: frame row 0 / row 70 and cols 0 / 95, glass rows
# 1..69 in cols 1..46 and 48..94, a hairline seam at col 47, rubber skirting rows 71..73.
# =============================================================================================
MIRROR_W, MIRROR_H = 96, 74
GLASS_ROWS = (1, 69)
GLASS_COLS = ((1, 46), (48, 94))
SEAM_COL = 47
# mirrored torch glows (panel-local x, local y) and the dim reversed bench silhouette, per panel
GLOWS = (((9, 9), (37, 24)), ((12, 27), (38, 12)))      # never level: no row of street lamps
BAYER = ((0.0, 0.5), (0.75, 0.25))        # 2 x 2 ordered dither for the smooth glass ramp
SHEENS = ((6, 27), (17, 44))              # diagonal offsets of the two streaks per panel


def mirror_glass(c: Pix, panel: int, x0: int, x1: int) -> None:
    g0, g1 = GLASS_ROWS
    for y in range(g0, g1 + 1):
        t = (y - g0) / (g1 - g0)
        ly = y - g0
        for x in range(x0, x1 + 1):
            lx = x - x0
            # silvered blue-grey, a little lighter toward the top (the tubes above): a smooth ramp with a
            # 2 x 2 ordered dither at each step, no block noise
            f = 3.3 - 1.8 * t
            ramp, v = "stone_cold", int(f) + (1 if (f - int(f)) > BAYER[ly % 2][lx % 2] else 0)
            # the lower two thirds take the warm room: every other pixel one step toward it
            if 29 <= ly < 56 and (lx + ly) % 2 == 0:
                ramp, v = "gal_warm", 2 if ly >= 40 else 1
            # faint mirrored brick courses of this room (1 px lines every 5 rows, staggered joints)
            if 30 <= ly < 56:
                course = (ly - 30) // 5
                if (ly - 30) % 5 == 4 or ((lx + (6 if course % 2 else 0)) % 12 == 0 and (ly - 30) % 5 != 4):
                    ramp, v = ("brick", 2) if (ly - 30) % 5 == 4 else ("gal_warm", 1)
            # the mirrored torch-lit floor: a warm band at the bottom
            if ly >= 56:
                ramp, v = "gal_warm", 2.4 - (ly - 56) * 0.12
                if ly == 56:
                    ramp, v = "gal_warm", 3
            # mirrored torch glows: warm blobs with a soft halo
            for gx, gy in GLOWS[panel]:
                d = math.hypot(lx - gx, (ly - gy) * 1.3)
                if d < 1.6:
                    ramp, v = "gal_warm", 6
                elif d < 3.4:
                    ramp, v = "gal_warm", 4.6
                elif d < 6.0 and ramp == "stone_cold" and rnd("halo", panel, lx, ly) > (d - 3.4) / 2.6:
                    ramp, v = "gal_warm", 3
            c.put(x, y, ramp, v)
    # the dim reversed silhouette of a bench and a plate stack standing in the room
    bx = x0 + (8 if panel == 0 else 20)
    for x in range(bx, bx + 18):
        c.put(x, 50, "stone_cold", 0)
        c.put(x, 51, "stone_cold", 0)
    for lx in (bx + 2, bx + 15):
        for y in range(52, 56):
            c.put(lx, y, "stone_cold", 0)
    for y in range(44, 56):
        for x in range(bx + 24, bx + 28):
            if abs(y - 50) < 6:
                c.put(x, y, "stone_cold", 0)
    # bold diagonal sheens: a near-white core with a pale-blue edge
    for y in range(g0, g1 + 1):
        ly = y - g0
        for x in range(x0, x1 + 1):
            d = (x - x0) + ly * 0.85
            for k, off in enumerate(SHEENS[panel]):
                m = (d - off) % 64
                width = 2 if k == 0 else 1
                if m < width:
                    c.put(x, y, "gal", 10 if k == 0 else 9)
                elif m < width + 1 and k == 0:
                    c.put(x, y, "stone_cold", 4)
    for x in range(x0, x1 + 1):                               # the lit top edge of the glass
        c.put(x, g0, "stone_cold", 4)


def mirror_bay() -> Pix:
    """Hero key (spec section 10): two silvered panels butted at a hairline seam in a thin aluminium frame,
    the glass runs down to the rubber skirting."""
    c = Pix(MIRROR_W, MIRROR_H)
    for panel, (x0, x1) in enumerate(GLASS_COLS):
        mirror_glass(c, panel, x0, x1)
    # hand smears at 1..1.5 m: small duller clusters on the glass
    for sx, sy in ((12, 36), (16, 40), (31, 34), (58, 42), (63, 38), (80, 35), (84, 43)):
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1), (2, 1)):
            here = c.get(sx + dx, sy + dy)
            if here and here[0] == "stone_cold" and rnd("sm", sx, sy, dx, dy) > 0.3:
                c.put(sx + dx, sy + dy, "stone_cold", here[1] - 1)
    # brown tape over a chipped corner, right panel lower right
    for y in range(60, 67):
        for x in range(85, 94):
            k = (x - 85) + (66 - y) * 1.2
            if 4 < k < 9:
                c.put(x, y, "brass", 2 + (1 if k < 6 else 0))
    # hairline seam between the panels
    for y in range(GLASS_ROWS[0], GLASS_ROWS[1] + 1):
        c.put(SEAM_COL, y, "stone_cold", 1)
    # thin aluminium frame: lit top, dark bottom, sides
    for x in range(MIRROR_W):
        c.put(x, 0, "stone_cold", 4)
        c.put(x, 70, "stone_cold", 1)
    for y in range(1, 70):
        c.put(0, y, "stone_cold", 3)
        c.put(95, y, "stone_cold", 2)
    # corner clips holding the glass
    for x0, x1 in GLASS_COLS:
        for cx in (x0 + 1, x1 - 2):
            for cy in (0, 70):
                c.put(cx, cy, "iron", 5)
                c.put(cx + 1, cy, "iron", 4)
    # rubber skirting, lit top row
    for x in range(MIRROR_W):
        c.put(x, 71, "iron", 3)
        c.put(x, 72, "iron", 1 + (1 if rnd("sk", x // 6) > 0.7 else 0))
        c.put(x, 73, "iron", 0)
    for x, y in ((0, 0), (95, 0)):                             # rounded outer frame corners
        c.px[y][x] = None
    return c


# ============================================================================================ wall
def gate_lintel() -> Pix:
    """48 x 18 cold-steel header over the gate (DFX F3); the gate slot shadow on rows 16..17, x 0..29."""
    c = Pix(48, 18)
    for y in range(0, 16):
        for x in range(0, 48):
            v = 2.2 + (rnd("lt", x // 4, y // 3) - 0.5) * 0.6
            if y == 0:
                v = 4
            elif y == 1:
                v = 3.2
            elif y in (14, 15):
                v = 1
            if x == 47:
                v -= 1
            c.put(x, y, "stone_cold", v)
    # a rust run under the rivets, from the copper ramp (the gate's language)
    for x in range(3, 45):
        if rnd("rust", x) > 0.55:
            c.put(x, 7 + (x % 3 == 0), "copper", 2 + (rnd("ru", x) > 0.7))
    for x in range(4, 47, 8):                               # rivet rows
        for y in (4, 11):
            c.put(x, y, "stone_cold", 4)
            c.put(x + 1, y + 1, "iron", 0)
    for y in (16, 17):                                      # the slot the grate rides in, and the brackets
        for x in range(0, 48):
            if x < 30:
                c.put(x, y, "iron", 0)
            elif x in (34, 35, 42, 43):
                c.put(x, y, "iron", 3 if y == 16 else 2)
    c.px[0][47] = None                                       # chamfered outer corner
    return c


def girder() -> Pix:
    """32 x 10 rolled I-beam span, tiles every 32 px; rows 8..9 are the ceiling line."""
    c = Pix(32, 10)
    for x in range(32):
        c.put(x, 0, "iron", 5)
        c.put(x, 1, "iron", 3)
        for y in range(2, 8):
            v = 1.4 + (rnd("web", x // 3, y // 2) - 0.5) * 0.6
            if y == 2:
                v = 0.6                                       # under the top flange
            c.put(x, y, "iron", v)
        c.put(x, 8, "iron", 2)
        c.put(x, 9, "iron", 4)                                # the underside catches the tubes and torches
    for rx in (6, 22):                                        # rivet pairs
        for ry in (3, 6):
            c.put(rx, ry, "iron", 5)
            c.put(rx + 1, ry, "iron", 0)
    c.put(31, 3, "iron", 0)                                   # the butt joint at the span end
    c.put(31, 4, "iron", 0)
    c.put(31, 5, "iron", 0)
    c.put(31, 6, "iron", 0)
    for y in range(3, 8):                                     # a rust streak from the lower rivet
        if y >= 6:
            c.put(23, y, "copper", 2)
    c.px[0][31] = None                                       # the butt joint notch in the top flange
    return c


# ========================================================================================= fixture
def fluoro(accent: bool) -> Pix:
    """40 x 10: chains rows 0..3, housing 4..6, tube 7..8, end caps; row 9 the diffuser lip."""
    c = Pix(40, 10)
    for cx in (6, 33):
        for y in range(0, 4):
            c.put(cx, y, "iron", 4 if y % 2 == 0 else 2)
    for x in range(2, 38):
        c.put(x, 4, "iron", 4)
        c.put(x, 5, "iron", 3)
        c.put(x, 6, "iron", 1)
    c.put(1, 5, "iron", 2)
    c.put(38, 5, "iron", 2)
    core = ("copper", 8) if accent else ("flame", 5)
    for x in range(4, 36):
        c.put(x, 7, *core)
        c.put(x, 8, "gal", 10) if not accent else c.put(x, 8, "brass", 6)
    for x in (2, 3, 36, 37):                                  # end caps
        c.put(x, 7, "iron", 4)
        c.put(x, 8, "iron", 2)
    if not accent:                                            # a flickering dead end on the cold one
        c.put(5, 7, "gal", 8)
        c.put(4, 7, "gal", 7)
    for x in range(3, 37):
        c.put(x, 9, "iron", 2 if x % 5 else 3)                # lip of the open diffuser frame
    return c


# =========================================================================================== props
def reception_desk() -> Pix:
    c = Pix(60, 32)
    # counter body, scuffed timber, rows 12..31
    for y in range(12, 32):
        for x in range(0, 60):
            v = 2.6 + (rnd("dk", x // 2, y // 6) - 0.5) * 0.6
            if x % 15 == 14:
                v = 1.0                                       # panel joints
            if y == 31:
                v = 0.6
            if x == 0:
                v += 0.8
            if x == 59:
                v -= 0.8
            c.put(x, y, "wood", v)
    for y in range(28, 32):                                   # kick plate, scuffed
        for x in range(1, 59):
            if rnd("kick", x, y) > 0.86:
                c.put(x, y, "wood", 3.6)
    # counter top slab, rows 9..11 (lighter laminate, chipped)
    for x in range(-0, 60):
        c.put(x, 9, "stone", 7 if rnd("ch", x) > 0.12 else 5)
        c.put(x, 10, "stone", 5)
        c.put(x, 11, "stone", 3)
    # pinned paper sign on the front (no text: three ruled lines)
    for y in range(16, 26):
        for x in range(36, 46):
            c.put(x, y, "stone", 8 if (y - 16) % 3 else 6)
    c.put(40, 16, "brick", 6)                                 # the pin
    c.put(45, 25, "stone", 5)                                 # curled corner
    # beige terminal on the counter: case rows 0..8, x 6..21
    for y in range(0, 9):
        for x in range(6, 22):
            v = 6.0 - (1.2 if x == 21 else 0) + (0.8 if y == 0 else 0)
            c.put(x, y, "stone", v)
    for y in range(2, 7):                                     # cold screen
        for x in range(8, 17):
            c.put(x, y, "gal", 7 if (y == 3 and x < 14) or (y == 5 and x < 12) else 5)
    c.put(19, 5, "brick", 7)                                  # power lamp
    for x in range(4, 24):                                    # keyboard
        c.put(x, 8, "stone", 4 if x % 2 else 5)
    # a chipped mug
    for y in range(4, 9):
        for x in range(28, 32):
            c.put(x, y, "stone", 7 if x < 31 else 5)
    c.put(32, 5, "stone", 6)
    c.put(32, 6, "stone", 6)
    c.put(28, 4, "stone", 8)
    return c


def turnstile() -> Pix:
    c = Pix(20, 30)
    # post x 6..11, rows 6..29
    for y in range(6, 30):
        for x in range(6, 12):
            v = 3.0 + (1.2 if x == 6 else 0) - (1.0 if x == 11 else 0)
            c.put(x, y, "stone_cold", v)
    for x in range(4, 14):                                    # base plate
        c.put(x, 29, "iron", 2)
        c.put(x, 28, "iron", 4 if x in (4, 13) else 3)
    # head box rows 2..9
    for y in range(2, 10):
        for x in range(4, 14):
            c.put(x, y, "stone_cold", 3.5 + (0.8 if y == 2 else 0) - (1.2 if y == 9 else 0)
                  + (0.6 if x == 4 else 0) - (0.6 if x == 13 else 0))
    c.put(7, 0, "brick", 4)                                   # the dull red lamp
    c.put(8, 0, "brick", 6)
    c.put(7, 1, "brick", 5)
    c.put(8, 1, "brick", 7)
    for x in range(6, 10):
        c.put(x, 1 if x in (6, 9) else 1, "brick", 5) if x in (6, 9) else None
    # rotor hub
    c.disc(13.5, 13.5, 2.2, "stone_cold", lambda x, y, d: 4 if y < 13 else 2)
    # three arms: one out to the right (the lane), one down-left, one toward the viewer (short, lit)
    for i in range(14, 20):
        c.put(i, 13, "stone_cold", 4)
        c.put(i, 14, "stone_cold", 2)
    c.line(13, 15, 16, 21, "stone_cold", 3)
    c.line(14, 15, 17, 21, "stone_cold", 1)
    c.put(15, 11, "stone_cold", 4)
    c.put(16, 10, "stone_cold", 4)
    c.put(17, 10, "stone_cold", 3)
    return c


def dumbbell(c: Pix, x0: int, y: int, size: int, seed: str) -> None:
    """A dumbbell seen end-on: two plates and a chrome handle stub; ``size`` 3..5 = plate radius."""
    for k, px in enumerate((x0, x0 + size * 2 + 3)):
        c.disc(px + size, y, size, "iron",
               lambda xx, yy, d, px=px: (3 if yy < y else 1) + (1 if d > size - 1 and yy < y else 0))
    for xx in range(x0 + size * 2 - 1, x0 + size * 2 + 4):
        c.put(xx, y, "stone_cold", 4)
        c.put(xx, y + 1, "stone_cold", 2)


def dumbbell_rack() -> Pix:
    c = Pix(56, 28)
    # frame: two A-frame ends and two shelves (y 12 and y 25)
    for side in (1, 54):
        for y in range(4, 28):
            c.put(side, y, "stone_cold", 3)
            c.put(side + 1, y, "stone_cold", 1)
    for sy in (12, 24):
        for x in range(0, 56):
            c.put(x, sy, "stone_cold", 3)
            c.put(x, sy + 1, "stone_cold", 1)
    for x in range(0, 4):
        c.put(x, 27, "iron", 2)
        c.put(52 + x, 27, "iron", 2)
    # top tier: three small dumbbells, one gap; lower tier: two heavy
    dumbbell(c, 4, 8, 3, "a")
    dumbbell(c, 18, 8, 3, "b")
    dumbbell(c, 40, 8, 3, "c")                                # gap at x 31..38
    dumbbell(c, 4, 19, 4, "d")
    dumbbell(c, 30, 19, 4, "e")
    return c


def bench() -> Pix:
    c = Pix(44, 18)
    # red vinyl pad, rows 2..8
    for y in range(2, 9):
        for x in range(1, 43):
            v = 5.0 + (1.6 if y == 2 else 0) - (1.6 if y == 8 else 0) + (rnd("vin", x // 3, y) - 0.5) * 0.6
            if x in (1, 42):
                v -= 1
            c.put(x, y, "brick", v)
    for x in range(14, 22):                                   # the split, taped over (grey tape)
        c.put(x, 3, "stone", 6)
        c.put(x, 4, "stone", 5)
    c.put(23, 4, "brick", 2)
    c.put(24, 5, "brick", 2)
    # steel frame under the pad and legs
    for x in range(4, 40):
        c.put(x, 9, "stone_cold", 3)
        c.put(x, 10, "stone_cold", 1)
    for lx in (5, 36):
        for y in range(10, 17):
            c.put(lx, y, "stone_cold", 3)
            c.put(lx + 1, y, "stone_cold", 1)
        for x in range(lx - 3, lx + 5):
            c.put(x, 17, "iron", 2)
            c.put(x, 16, "stone_cold", 2 if x in (lx - 3, lx + 4) else 3)
    return c


def plate_tree() -> Pix:
    """28 x 36: a post on a cross foot with two pegs a side; iron plates face-on (round, a hub)."""
    c = Pix(28, 36)
    for y in range(1, 34):
        c.put(13, y, "stone_cold", 3)
        c.put(14, y, "stone_cold", 1)
    for x in range(2, 26):                                    # cross foot
        c.put(x, 34, "stone_cold", 3)
        c.put(x, 35, "iron", 1)
    for x in range(12, 16):
        c.put(x, 0, "stone_cold", 4)
    for px, py, r in ((6.5, 11.5, 6.2), (21.5, 11.5, 6.2), (6.5, 26.5, 5.4), (21.5, 26.5, 5.4)):
        c.disc(px, py, r, "iron", lambda x, y, d, py=py, r=r: (3.2 if d > r - 1.2 and y < py else
                                                            (2.2 if d > r - 1.2 else 1.4)) + (0.6 if d < 2.2 else 0))
        c.put(int(px), int(py), "stone_cold", 4)
    return c


def cable_tower() -> Pix:
    c = Pix(44, 104)
    # uprights x 2..5 and 38..41
    for ux in (2, 38):
        for y in range(6, 102):
            c.put(ux, y, "stone_cold", 3)
            c.put(ux + 1, y, "stone_cold", 2)
            c.put(ux + 2, y, "stone_cold", 1)
            c.put(ux + 3, y, "iron", 1)
        for y in range(22, 98, 9):                           # rust at the bolt holes
            c.put(ux + 1, y, "copper", 2)
    # top beam rows 2..7
    for x in range(0, 44):
        c.put(x, 2, "stone_cold", 4)
        for y in (3, 4, 5):
            c.put(x, y, "stone_cold", 2.5)
        c.put(x, 6, "stone_cold", 1)
    # pulley wheels under the beam
    for wx in (12, 32):
        c.disc(wx, 11, 3.6, "iron", lambda x, y, d: 4 if d > 2.5 and y < 11 else 2)
        c.put(wx, 11, "stone_cold", 4)
    # cable: from the left wheel down to the stack top, from the right wheel down to the handle
    for y in range(14, 58):
        c.put(10, y, "stone", 6 if y % 4 else 5)
    for y in range(14, 70):
        c.put(34, y, "stone", 6 if y % 4 else 5)
    for x in range(10, 35):                                   # the top link over the wheels
        c.put(x, 7, "stone", 5)
    # selector stack between guide rods, rows 58..99
    for gx in (7, 16):
        for y in range(56, 101):
            c.put(gx, y, "stone_cold", 4)
    for i, sy in enumerate(range(60, 100, 4)):
        for y in range(sy, sy + 3):
            for x in range(8, 16):
                v = 3.2 if y == sy else (2 if y == sy + 1 else 1)
                c.put(x, y, "iron", v)
        if i == 5:
            c.put(15, sy + 1, "brick", 6)                      # the selector pin
    for x in range(8, 16):                                    # top plate the cable lifts
        c.put(x, 58, "stone_cold", 3)
        c.put(x, 59, "stone_cold", 1)
    # handle hanging from the right cable
    for x in range(30, 39):
        c.put(x, 71, "stone_cold", 4)
        c.put(x, 72, "iron", 1)
    c.put(30, 70, "stone_cold", 3)
    c.put(38, 70, "stone_cold", 3)
    c.put(34, 70, "stone_cold", 3)
    # base frame
    for x in range(0, 44):
        c.put(x, 101, "stone_cold", 3)
        c.put(x, 102, "stone_cold", 1)
        c.put(x, 103, "iron", 0)
    # a towel over the right upright (grey, frayed)
    for y in range(30, 42):
        for x in range(37, 43):
            if y < 34 or x in (37, 42) or (x == 41 and y < 40):
                c.put(x, y, "stone", 6 if x < 40 else 5)
    return c


def tyres() -> Pix:
    c = Pix(34, 18)
    for top, inset in ((0, 2), (9, 0)):                        # upper tyre slightly smaller, set back
        for y in range(top, top + 9):
            for x in range(inset, 34 - inset):
                v = 1.2
                if y == top:
                    v = 3
                elif y == top + 1:
                    v = 2.2
                elif y == top + 8:
                    v = 0
                if (x + y) % 4 == 0 and top < y < top + 8:
                    v += 1.1                                  # tread blocks
                if x in (inset, 33 - inset):
                    v -= 0.6
                c.put(x, y, "iron", v)
        for x in range(inset + 6, 28 - inset):                 # the bore seen on top
            if top == 0:
                c.put(x, 1, "iron", 0)
    return c


# =========================================================================================== floor
def mat(variant: str) -> Pix:
    """48 x 22 rubber tile over the floor surface: lit far row, dark front face (rows 20..21), seam col 47."""
    c = Pix(48, 22)
    for y in range(22):
        for x in range(48):
            v = 3.8 + (rnd("mat", variant, x // 8, y // 6) - 0.5) * 0.3 - 0.4 * (y / 19)
            if (x % 8 == 3) and (y % 6 == 3):
                v += 1                                        # moulded studs
            if y == 0:
                v = 4
            elif y == 1:
                v = 3.6
            elif y == 19:
                v = 4.4                                       # the front edge catches light
            elif y in (20, 21):
                v = 1 if y == 21 else 2
            if x == 47:
                v = 1
            c.put(x, y, "iron", v)
    if variant == "b":
        for y in range(2, 19):                                # grey duct tape down the middle seam
            c.put(23, y, "stone_cold", 2)
            c.put(24, y, "stone_cold", 3 if y % 5 else 2)
        for y in range(14, 19):                               # scuffed corner showing stone
            for x in range(40, 47):
                if (x - 40) + (18 - y) < 5:
                    c.put(x, y, "floor", 4)
    c.px[0][47] = None                                       # the seam opens at the far edge
    return c


# ============================================================================================ exit
EXIT_W, EXIT_H = 20, 96


def exit_door(open_: bool) -> Pix:
    """20 x 96: lamp box rows 0..5 (an 8 x 3 lamp: dull red when closed, lit brass with a halo when open),
    lintel rows 6..8, jamb cols 0..1; closed = riveted steel leaf, open = a warm lamp-lit passage with the
    leaf swung in against the jamb (the same "open = warm lit passage" cue as the rooms 2/4/6 exits)."""
    c = Pix(EXIT_W, EXIT_H)
    for y in range(6, EXIT_H):
        c.put(0, y, "iron", 3)
        c.put(1, y, "iron", 1)
    for x in range(0, EXIT_W):
        c.put(x, 6, "iron", 4)
        c.put(x, 7, "iron", 2)
        c.put(x, 8, "iron", 1)
    for y in range(0, 6):                                     # the lamp box over the door
        for x in range(4, 16):
            c.put(x, y, "iron", 2 if 0 < y < 5 else 3)
    for y in range(1, 5):
        for x in range(5, 15):
            core = 6 <= x <= 13 and 2 <= y <= 3
            if open_:
                c.put(x, y, "brass", 6 if core else 4)
            elif core:
                c.put(x, y, "brick", 3)
    if not open_:
        # riveted steel leaf with a push bar
        for y in range(9, EXIT_H):
            for x in range(2, EXIT_W):
                v = 1.4 + (rnd("lf", x // 6, y // 16) - 0.5) * 0.3 + (1.0 if x == 2 else 0) - (0.6 if x == EXIT_W - 1 else 0)
                c.put(x, y, "stone_cold", v)
        for y in (12, 50, 90):
            for x in range(3, EXIT_W, 4):
                c.put(x, y, "stone_cold", 4)
        for x in range(3, EXIT_W):
            c.put(x, 54, "stone_cold", 4)
            c.put(x, 55, "iron", 1)
        for y in range(20, 40):                               # a scuffed kick at hand height, rust run
            if rnd("rr", y) > 0.6:
                c.put(16, y, "copper", 2)
        for x in range(2, EXIT_W):
            c.put(x, EXIT_H - 1, "iron", 0)
    else:
        # a warm lamp-lit passage: brightest under the lamp, falling off downward and to the far wall
        for y in range(9, EXIT_H):
            t = (y - 9) / (EXIT_H - 9)
            for x in range(5, EXIT_W):
                if y > EXIT_H - 14:                           # the passage floor catches the lamp
                    v = 4.6 - 1.6 * (EXIT_H - 1 - y) / 13 + (0.6 if (x + y) % 3 == 0 else 0)
                    c.put(x, y, "gal_warm", v)
                else:
                    v = 5.0 - 2.6 * t - 0.12 * abs(x - 11)
                    if (x * 7 + y // 6) % 13 == 0:
                        v -= 1                                # a few block joints in the passage wall
                    c.put(x, y, "gal_warm", v)
            for x in range(2, 5):                             # the leaf, swung in, edge-on and dark
                c.put(x, y, "stone_cold", 3 if x == 2 else 1)
        for x in range(5, EXIT_W):                            # the lit threshold
            c.put(x, EXIT_H - 1, "brass", 4)
    return c


# ========================================================================================= atlases
ATLASES = {
    "floor01_gym_wall": {
        "stem": "gym_wall", "width": 144,
        "pieces": [("gym_mirror_bay", mirror_bay), ("gym_gate_lintel", gate_lintel), ("gym_girder", girder)],
    },
    "floor01_gym_fixture": {
        "stem": "gym_fixture", "width": 80,
        "pieces": [("gym_fluoro_cold", lambda: fluoro(False)), ("gym_fluoro_accent", lambda: fluoro(True))],
    },
    "floor01_gym_props": {
        "stem": "gym_props", "width": 200,
        "pieces": [("gym_cable_tower", cable_tower), ("gym_reception_desk", reception_desk),
                   ("gym_dumbbell_rack", dumbbell_rack), ("gym_turnstile", turnstile), ("gym_bench", bench),
                   ("gym_plate_tree", plate_tree), ("gym_tyres", tyres)],
    },
    "floor01_gym_floor": {
        "stem": "gym_floor", "width": 96,
        "pieces": [("gym_mat_a", lambda: mat("a")), ("gym_mat_b", lambda: mat("b"))],
    },
    "floor01_gym_exit": {
        "stem": "gym_exit", "width": 40,
        "pieces": [("gym_exit_closed", lambda: exit_door(False)), ("gym_exit_open", lambda: exit_door(True))],
    },
}


def pack(pieces: list[tuple[str, Pix]], width: int) -> tuple[Pix, dict]:
    """Shelf packing: left to right, a new shelf when the row is full."""
    x = y = shelf = 0
    placed = []
    for pid, img in pieces:
        if x + img.w > width:
            x, y, shelf = 0, y + shelf, 0
        placed.append((pid, img, x, y))
        x += img.w
        shelf = max(shelf, img.h)
    sheet_img = Pix(width, y + shelf)
    rects = {}
    for pid, img, px, py in placed:
        for yy in range(img.h):
            for xx in range(img.w):
                if img.px[yy][xx] is not None:
                    sheet_img.px[py + yy][px + xx] = img.px[yy][xx]
        rects[pid] = [px, py, img.w, img.h]
    return sheet_img, rects


def build(name: str) -> tuple[dict, dict]:
    spec = ATLASES[name]
    pieces = [(pid, fn()) for pid, fn in spec["pieces"]]
    sheet_img, rects = pack(pieces, spec["width"])
    return kit.grid_from([sheet_img], layer_names=["main"], durations=[100], pivot=(0, 0)), rects


# ======================================================================================== contract
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_gym_kit.json"
MANIFESTS = ROOT / "assets/manifests"
EXTENDS = "assets/source/terrain/floor01/floor01_opening_kit.json"

# piece id -> (role, layer); props and fixtures stand in the prop layer, mirrors on the back wall
ROLES = {
    "gym_mirror_bay": ("backwall", "backwall_decal"),
    "gym_gate_lintel": ("structure", "structure"),
    "gym_girder": ("structure", "structure"),
    "gym_mat_a": ("structure", "structure"),
    "gym_mat_b": ("structure", "structure"),
}
COLD_TUBE = [
    {"id": "tube", "offset": [20, 8], "colour": "#cfdcff", "radius": 150, "intensity": 0.26, "group": "scene",
     "glow": {"radius": 16, "additive": 0.10}},
    {"id": "tube_floor", "offset": [20, 8], "colour": "#bccbff", "radius": 120, "intensity": 0.16, "group": "scene",
     "project": "walk_line", "project_dy": -6, "squash_y": 0.3},
]
ACCENT_TUBE = [
    {"id": "tube", "offset": [20, 8], "colour": "#ff62cf", "radius": 150, "intensity": 0.30, "group": "scene",
     "glow": {"radius": 18, "additive": 0.18}},
    {"id": "tube_floor", "offset": [20, 8], "colour": "#ff7ad6", "radius": 110, "intensity": 0.12, "group": "scene",
     "project": "walk_line", "project_dy": -6, "squash_y": 0.3},
]
EXIT_LIGHTS = [  # GYM-AD8: always on (lighting reads no state), like the rooms 2/4/6 exits (EXIT-AD4)
    {"id": "door_lamp", "offset": [10, 3], "colour": "#ffc27a", "radius": 60, "intensity": 0.16, "group": "scene",
     "glow": {"radius": 8, "additive": 0.08}},
    {"id": "door_leak", "offset": [10, 94], "colour": "#ffc27a", "radius": 90, "intensity": 0.22, "group": "scene",
     "project": "walk_line", "squash_y": 0.3},
]
LIGHTS = {"gym_fluoro_cold": COLD_TUBE, "gym_fluoro_accent": ACCENT_TUBE, "gym_exit_open": EXIT_LIGHTS}
STATES = {"gym_exit_closed": "closed", "gym_exit_open": "open"}

# Spec section 4: the arena placements (layout px, top-left); floor back line y 400, ceiling line y 196.
FB = 400
PLACE = (
    [("backwall_decal", "gym_mirror_bay", 28, 326), ("backwall_decal", "gym_mirror_bay", 528, 326)]
    + [("structure", "gym_mat_a" if i % 2 == 0 else "gym_mat_b", 224 + 48 * i, 401) for i in range(6)]
    + [("structure", "gym_gate_lintel", 0, 180)]
    + [("structure", "gym_girder", x, 186) for x in range(48, 609, 32)]
    + [("prop", "gym_fluoro_cold", 232, 196), ("prop", "gym_fluoro_cold", 360, 196),
       ("prop", "gym_fluoro_accent", 520, 196)]
    + [("prop", pid, x, FB - h) for pid, x, h in (
        ("gym_reception_desk", 46, 32), ("gym_turnstile", 110, 30), ("gym_dumbbell_rack", 228, 28),
        ("gym_bench", 296, 18), ("gym_plate_tree", 352, 36), ("gym_cable_tower", 440, 104), ("gym_tyres", 490, 18))]
    + [("prop", "gym_exit_closed", 620, 324), ("prop", "gym_exit_open", 620, 324)]
)
REMOVE = ["door_timber", "guild_portal_closed"]
OWNER = ("AI-authored environment pixel art (AREA-GYM-01: design spec, mirror-bay hero key and execution "
         "Claude Opus; independent review by a separate Claude Opus agent pending)")


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = ROLES.get(pid, ("prop", "prop"))
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0], "anchor_name": "top_left (arena placement, spec section 4)", "grid_snap": 1,
             "grammar": ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    if pid in LIGHTS:
        entry["lights"] = LIGHTS[pid]
    if pid in STATES:
        entry["state_of"] = {"group": "gym_exit", "state": STATES[pid]}
    return entry


def contract(rects_by_atlas: dict) -> dict:
    atlases, pieces, palette_use = {}, [], {}
    for name, spec in ATLASES.items():
        stem = spec["stem"]
        atlases[name] = {"grid": f"assets/source/props/floor01/{stem}.grid.json",
                         "manifest": f"assets/manifests/{name}.json",
                         "image": f"generated/art/{name}.png", "metadata": f"generated/art/{name}.json"}
        palette_use[name] = rects_by_atlas[name]["colours"]
        for pid, _ in spec["pieces"]:
            pieces.append(piece_entry(pid, name, rects_by_atlas[name]["rects"][pid]))
    return {
        "format": "env-kit-v2",
        "kit_id": "floor01_gym",
        "extends": EXTENDS,
        "status": "proposed - AREA-GYM-01 kit (docs/02-art/area-gym-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema "
                          "from the ENV-01 contract through the opening kit. Adds five decor atlases and the gym "
                          "pieces; no tiles, nothing collidable and no new palette colour.",
        "palette_use": palette_use,
        "atlases": atlases,
        "tiles": [],
        "pieces": pieces,
        "state_groups": {"gym_exit": {"states": ["closed", "open"],
                                      "note": "drawn by src/gym_art.rs: closed while the boss lives, open once "
                                              "Boss::released() (presentation only)"}},
        "arena_dressing": {
            "layout_id": "floor01_boss_arena",
            "note": "applied by src/gym_kit.rs to the composed boss arena: pieces removed from the catacomb "
                    "composition, then placements appended per layer (layout px, top-left). Presentation only.",
            "remove": REMOVE,
            "place": [{"layer": l, "piece": pid, "x": x, "y": y} for l, pid, x, y in PLACE],
        },
        "usage_rules": {
            "R1_gym_arena": "the boss arena only: mirror bays reach both crash walls (the mirror_crack anchors), "
                            "props stand on the floor back line (y 400), mats keep rows 400 and 422..423 of the "
                            "floor visible, the girder's bottom row is the ceiling collider line (y 195), the exit "
                            "door states share one rect.",
        },
    }


def manifest(name: str, canvas: list) -> dict:
    stem = ATLASES[name]["stem"]
    return {
        "schema_version": 1,
        "id": f"{name}_v1",
        "type": "decor_atlas",
        "source_path": f"assets/source/props/floor01/{stem}.ase",
        "grid_source": f"assets/source/props/floor01/{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png",
        "output_metadata": f"generated/art/{name}.json",
        "palette_id": "floor01-catacomb-v2",
        "frame_size": canvas,
        "pivot": [0, 0],
        "animation_tags": [],
        "required_alpha": True,
        "kit_contract": "assets/source/terrain/floor01/floor01_gym_kit.json",
        "owner": OWNER,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 gym pieces for the Floor 1 boss arena (AREA-GYM-01 on the ENV-01 kit); "
                      "rule-based shapes and ramps, nothing cropped, traced or resampled from concept art; no text, "
                      "logos or likenesses",
            "generator": "Deterministic authoring script scripts/author_gym01.py written by an AI agent (designed and "
                         "executed by Claude Opus); no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-juicer-gym standalone panorama (owner-selected 2026-11-17; mood reference only)",
                "floor-01-art-atlas-v1/09-juicer-gym-atlas-excerpt (mood reference only)",
            ],
        },
    }


def write() -> dict:
    out = {}
    for name, spec in ATLASES.items():
        grid, rects = build(name)
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours {rects}")
        out[name] = {"rects": rects, "canvas": grid["canvas"], "colours": len(grid["palette"])}
        (MANIFESTS / f"{name}.json").write_text(json.dumps(manifest(name, grid["canvas"]), indent=1) + "\n",
                                                encoding="utf-8")
    CONTRACT.write_text(json.dumps(contract(out), indent=1) + "\n", encoding="utf-8")
    return out


def sheet(out_dir: Path) -> None:
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, spec in ATLASES.items():
        grid, _ = build(name)
        pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5)) for p in grid["palette"]}
        rows = grid["frames"][0]["cels"]["main"]
        img = Image.new("RGBA", (len(rows[0]), len(rows)), (40, 36, 48, 255))
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch != ".":
                    img.putpixel((x, y), pal[ch] + (255,))
        img.resize((img.width * 4, img.height * 4), Image.NEAREST).save(out_dir / f"{spec['stem']}_4x.png")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "sheet":
        sheet(Path(sys.argv[2]))
    else:
        write()
