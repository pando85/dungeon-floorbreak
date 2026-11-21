#!/usr/bin/env python3
"""Authoring record for AREA-REFUGE-01 (execution): the room kit props of the guild interior.

Draws every prop of ``docs/02-art/area-refuge-01-spec.md`` section 4 (except the frozen hearth key, owned
by ``scripts/author_refuge01.py``) on ``floor01-catacomb-v2``, with no new colour, as three pixel-grid-v1
atlases and their ``.ase`` sources:

- ``assets/source/props/floor01/refuge_props.grid.json``  bed, bookshelf, desk, stool, barrel, satchel
- ``assets/source/props/floor01/refuge_wall.grid.json``   flue tiles, window, wall map, portrait, landscape
- ``assets/source/props/floor01/refuge_door.grid.json``   the inside face of the guild portal, closed and open

Every pixel comes from rules (shapes, ramps, a hash noise); nothing is traced or resampled from a concept
image. ``refuge_frame_portrait`` shows two original humanoid bird-people (owner direction 2026-11-21).

    python3 scripts/author_refuge01_props.py            # grids + .ase
    python3 scripts/author_refuge01_props.py sheet DIR  # 3x sheets of every atlas
"""
from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402
import author_floor01_opening as opening  # noqa: E402  (portal geometry and frame shading)
from author_refuge01 import clamp, rnd  # noqa: E402

RAMPS = kit.RAMPS
SRC = ROOT / "assets/source/props/floor01"


class Pix:
    """A piece canvas: (ramp, index) per pixel, ``None`` transparent. Duck-typed like ``ld.Canvas``."""

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


def wood_box(c, x0, y0, x1, y1, base, seed, lit="r", grain=0.35):
    """A timber slab: lit edge towards ``lit`` (l/r/t), shadow edge opposite, light grain."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            v = base + (rnd(seed, x // 2, y) - 0.5) * grain * 2
            if lit == "r" and x == x1:
                v += 1
            if lit == "l" and x == x0:
                v += 1
            if y == y0:
                v += 0.8
            if y == y1:
                v -= 0.8
            c.put(x, y, "wood", v)


# ------------------------------------------------------------------------------------- bed
def bed() -> Pix:
    c = Pix(64, 28)
    # trunk under the foot end (drawn first, the blanket hangs in front of its top)
    wood_box(c, 44, 20, 59, 27, 3.0, "trunk", lit="l")
    for x in (47, 56):
        c.box(x, 20, x, 27, "iron", 3)
        c.put(x, 20, "iron", 4.5)
    c.put(51, 23, "brass", 4)
    c.put(52, 23, "brass", 3)
    # legs and posts: headboard post tall at the left, footboard low at the right
    wood_box(c, 0, 3, 5, 27, 3.0, "head", lit="r")
    c.box(1, 2, 4, 2, "wood", 4)                      # post cap
    wood_box(c, 58, 12, 63, 27, 3.0, "foot", lit="l")
    c.box(59, 11, 62, 11, "wood", 4)
    # side rail along the front
    wood_box(c, 6, 17, 57, 21, 2.6, "rail", lit="t", grain=0.25)
    c.box(6, 22, 57, 22, "wood", 0.5)                 # shadow under the rail
    # mattress slab, linen
    for y in range(12, 17):
        for x in range(6, 58):
            v = 6.6 + (rnd("lin", x // 3, y) - 0.5) * 0.8 - (1.6 if y == 16 else 0) + (0.7 if y == 12 else 0)
            c.put(x, y, "stone", v)
    # pillow at the head
    for y in range(8, 13):
        for x in range(7, 20):
            if (y in (8, 12)) and (x in (7, 8, 18, 19)):
                continue
            v = 8.0 + (0.9 if y == 8 else 0) - (1.4 if y == 12 else 0) - (0.8 if x > 16 else 0)
            c.put(x, y, "stone", v + (rnd("pil", x, y) - 0.5) * 0.4)
    # patched red blanket: folded over the mattress and hanging 6 px over the front rail
    for y in range(10, 24):
        for x in range(22, 58):
            hang = y >= 17
            lo = 17 + 3 + (2 if (x * 7) % 5 < 2 else 0) + (1 if x % 4 == 0 else 0)   # frayed edge
            if hang and y > lo + 3:
                continue
            if not hang and y < 11 + (1 if x % 5 == 0 else 0):
                continue
            v = 5.2 + (rnd("bl", x // 2, y // 2) - 0.5) * 0.7
            if y == 11:
                v += 0.9
            if hang and y > lo + 1:
                v -= 1.2                                                # fraying shadow
            if hang and (x - 22) % 9 == 8:
                v -= 0.8                                                # fold line
            c.put(x, y, "brick", v)
    for (x0, y0, x1, y1, tone) in ((26, 12, 32, 15, 3.6), (45, 18, 51, 21, 6.6)):   # two patches
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                edge = x in (x0, x1) or y in (y0, y1)
                c.put(x, y, "brick", tone - (0.8 if edge else 0))
                if edge and (x + y) % 2 == 0:
                    c.put(x, y, "stone", 6)                              # stitches
    return c


# ------------------------------------------------------------------------------------- bookshelf
BOOK_TONES = [("brick", 3), ("brick", 5), ("copper", 3), ("stone", 4), ("wood", 3), ("brick", 4), ("copper", 4),
              ("stone", 5), ("wood", 4), ("brick", 6)]


def books(c, x0, x1, y_floor, y_top, seed, lean_at=None):
    x = x0
    i = 0
    while x <= x1 - 1:
        w = 2 + int(rnd(seed, "w", i) * 2)
        h = min(y_floor - y_top + 1, 7 + int(rnd(seed, "h", i) * 5))
        ramp, v = BOOK_TONES[int(rnd(seed, "t", i) * len(BOOK_TONES))]
        if x + w > x1 + 1:
            w = x1 + 1 - x
            if w < 2:
                break
        if lean_at is not None and i == lean_at:                       # a leaning book
            for yy in range(h):
                for xx in range(w):
                    c.put(x + xx + (h - 1 - yy) // 3 - 1, y_floor - yy, ramp, v + (0.6 if xx == 0 else 0))
            x += w + 2
            i += 1
            continue
        for yy in range(h):
            for xx in range(w):
                vv = v + (0.6 if xx == 0 else (-0.6 if xx == w - 1 else 0))
                if yy == h - 1 or (yy == 2 and w >= 3 and rnd(seed, "b", i) > 0.5):
                    vv -= 0.9                                           # head / a blank binding band (no text)
                c.put(x + xx, y_floor - yy, ramp, vv)
        x += w
        i += 1


def bookshelf() -> Pix:
    c = Pix(36, 56)
    c.box(5, 3, 30, 51, "wood", 0.8)                                            # back panel
    wood_box(c, 2, 3, 4, 55, 3.0, "sideL", lit="r")
    wood_box(c, 31, 3, 33, 55, 3.0, "sideR", lit="l")
    wood_box(c, 0, 0, 35, 2, 3.4, "top", lit="t", grain=0.2)                    # top board overhangs 1 px each side
    for y0 in (14, 28, 42):
        wood_box(c, 5, y0, 30, y0 + 2, 3.2, f"sh{y0}", lit="t", grain=0.2)
        c.box(5, y0 + 3, 30, y0 + 3, "wood", 0.3)
    wood_box(c, 2, 52, 33, 55, 2.4, "base", lit="t", grain=0.2)
    books(c, 5, 30, 13, 3, "b1")
    books(c, 5, 30, 27, 17, "b2", lean_at=4)
    books(c, 5, 30, 41, 31, "b3")
    # bottom bay: a rolled scroll bundle (three tubes) and a stacked pair of boxes
    for k, (x0, y) in enumerate(((6, 51), (6, 49), (9, 50))):
        for yy in (y, y - 1):
            for xx in range(x0, x0 + 11):
                c.put(xx, yy, "stone", 8.4 - (0.9 if yy == y else 0) + (0.4 if k == 2 else 0))
        c.put(x0, y, "stone", 5)
        c.put(x0 + 10, y - 1, "stone", 6)
    wood_box(c, 20, 47, 28, 51, 3.4, "box1", lit="l")
    wood_box(c, 21, 45, 27, 46, 3.9, "box2", lit="l")
    c.put(24, 49, "brass", 4)
    return c


# ------------------------------------------------------------------------------------- desk
def desk() -> Pix:
    c = Pix(56, 26)
    for (xa, xb) in ((8, 4), (47, 51)):                                         # trestle legs, splayed
        for y in range(7, 26):
            x = round(xa + (xb - xa) * (y - 7) / 18)
            for dx in range(3):
                c.put(x + dx, y, "wood", 2.6 + (0.9 if dx == 0 else (-0.6 if dx == 2 else 0)))
    wood_box(c, 7, 17, 48, 19, 2.8, "stretch", lit="t", grain=0.2)               # cross stretcher
    wood_box(c, 0, 4, 55, 6, 4.0, "dtop", lit="r", grain=0.3)                    # top, 3 px thick
    c.box(1, 7, 54, 7, "wood", 0.5)
    for x in range(18, 36):                                                      # a rolled-out map (no letters)
        c.put(x, 3, "stone", 8.6 - (0.8 if x in (18, 35) else 0))
        if x in (18, 19, 34, 35):
            c.put(x, 2, "stone", 7.4)                                            # curled ends
    for x in (23, 27, 31):
        c.put(x, 3, "brick", 5)                                                  # route ink
    wood_box(c, 10, 1, 14, 3, 2.0, "inkpot", lit="l")
    c.put(11, 1, "iron", 4)
    c.box(38, 3, 43, 3, "brass", 4)                                              # candle dish
    c.put(38, 3, "brass", 3)
    c.box(40, 2, 41, 2, "stone", 9)                                              # candle stub (the flame is its own piece)
    return c


def candle_grid() -> dict:
    """The desk candle flame: 3 x 2 px, four identical frames (the sprite is static; the light flickers)."""
    frames = []
    for _ in range(4):
        f = Pix(3, 2)
        f.put(1, 0, "flame", 3)
        f.put(0, 1, "flame", 2)
        f.put(1, 1, "flame", 4)
        f.put(2, 1, "flame", 2)
        frames.append({"main": f})
    return kit.grid_from([], layer_names=["main"], frames_layers=frames, durations=[120] * 4, pivot=(0, 0),
                         tags=[{"name": "burn", "from": 0, "to": 3, "direction": "forward"}])


# ------------------------------------------------------------------------------------- stool, barrel
def stool() -> Pix:
    c = Pix(14, 18)
    for y in range(0, 4):                                                        # round seat (a flat ellipse)
        for x in range(0, 14):
            half = 7.0 if y in (1, 2) else 5.5
            if abs(x + 0.5 - 7) > half:
                continue
            v = 4.6 if y < 3 else 2.6
            if y == 0:
                v += 0.8
            c.put(x, y, "wood", v + (rnd("seat", x, y) - 0.5) * 0.4 + (0.6 if x > 9 and y < 3 else 0))
    for (xa, xb, tone) in ((2, 1, 2.6), (11, 12, 3.4), (7, 7, 1.8)):             # three splayed legs
        yb = 15 if xa == 7 else 17
        for y in range(4, yb + 1):
            x = round(xa + (xb - xa) * (y - 4) / (yb - 4))
            c.put(x, y, "wood", tone)
            c.put(x + 1, y, "wood", tone - 0.7)
    return c


def barrel() -> Pix:
    c = Pix(22, 28)
    for y in range(0, 28):
        t = y / 27.0
        half = 8.8 + 2.2 * math.sin(math.pi * t)                                 # a 2 px bulge
        for x in range(22):
            dx = x + 0.5 - 11
            if abs(dx) > half:
                continue
            stave = int((dx + 12) // 3)
            v = 3.1 + (rnd("stv", stave) - 0.5) * 0.6
            v += 1.0 if dx > half - 2.2 else 0                                   # torch-side highlight
            v -= 1.0 if dx < -half + 1.6 else 0
            if (dx + 12) % 3 < 0.9:
                v -= 0.9                                                         # stave joint
            c.put(x, y, "wood", v)
    for x in range(3, 19):                                                       # lid edge catching the torch
        c.put(x, 0, "wood", 5)
        c.put(x, 1, "wood", 4)
    for y0 in (4, 12, 22):                                                       # three iron hoops
        for y in (y0, y0 + 1):
            for x in range(22):
                if c.get(x, y):
                    dx = x + 0.5 - 11
                    c.put(x, y, "iron", (4.6 if y == y0 else 2.4) + (1 if dx > 5 else 0))
    return c


# ------------------------------------------------------------------------------------- wall pieces
def wall_map() -> Pix:
    c = Pix(32, 24)
    for y in range(0, 24):
        for x in range(0, 32):
            edge = x in (0, 31) or y in (0, 23)
            v = 8.6 + (rnd("map", x // 2, y // 2) - 0.5) * 0.8
            if y < 3 or y > 20:
                v = 7.0 if y in (1, 22) else 9.0                                 # curled top and bottom
            if x in (0, 31):
                v -= 1.6
            if edge:
                v -= 1.0
            c.put(x, y, "stone", v)
    c.box(0, 0, 31, 0, "mortar", 3)
    c.box(0, 23, 31, 23, "mortar", 3)
    pts = [(4, 8), (9, 12), (14, 10), (19, 15), (24, 13), (27, 18)]              # a dotted route
    for a, b in zip(pts, pts[1:]):
        n = max(abs(b[0] - a[0]), abs(b[1] - a[1]))
        for i in range(n + 1):
            if i % 2 == 0:
                c.put(round(a[0] + (b[0] - a[0]) * i / n), round(a[1] + (b[1] - a[1]) * i / n), "brick", 5)
    cx, cy = 26, 7                                                               # the circle mark
    for dx, dy in ((-2, 0), (2, 0), (0, -2), (0, 2), (-1, -1), (1, -1), (-1, 1), (1, 1)):
        c.put(cx + dx, cy + dy, "brick", 5)
    for (x, y) in ((6, 15), (17, 19), (11, 6)):                                  # stray hatching
        c.line(x, y, x + 2, y + 1, "stone", 6)
    c.put(1, 2, "iron", 5)                                                       # the two pins
    c.put(30, 2, "iron", 5)
    for (x, y) in ((0, 0), (31, 23)):                                            # clipped corners (1-bit alpha rule)
        c.px[y][x] = None
    return c


def frame_ring(c, w, h, border=2):
    for (x, y) in ((0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)):              # rounded outer corners stay clear
        c.px[y][x] = None
    for y in range(h):
        for x in range(w):
            d = min(x, y, w - 1 - x, h - 1 - y)
            if d >= border or (x, y) in ((0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)):
                continue
            if d == 0:
                v = 3.0 + (0.9 if (y == 0 or x == 0) else 0)
            else:
                v = 4.0 - (0.7 if (y == h - 2 or x == w - 2) else 0)
            c.put(x, y, "wood", v + (rnd("fr", x, y) - 0.5) * 0.3)


def frame_portrait() -> Pix:
    c = Pix(24, 28)
    frame_ring(c, 24, 28)
    c.box(2, 2, 21, 25, "mortar", 2)                                             # inner mat line
    ox, oy = 3, 3                                                                # picture area 18 x 22
    for y in range(22):
        for x in range(18):
            corner = min(x, 17 - x) + min(y, 21 - y)
            c.put(ox + x, oy + y, "stone", 6 if corner < 3 else 7)               # flat sepia wash, vignette corners
    for x in range(18):                                                          # floor line
        c.put(ox + x, oy + 20, "stone", 5)
        c.put(ox + x, oy + 21, "stone", 4)

    def P(x, y, ramp, v):
        c.put(ox + x, oy + y, ramp, v)

    # left bird-person: 14 px tall (rows 6..19), swept-back 2 px crest, 2 px beak pointing right
    for y in range(6, 11):                                                       # head, 5 x 5 rounded
        for x in range(2, 7):
            if (y in (6, 10)) and x in (2, 6):
                continue
            P(x, y, "stone", 5)
    for (x, y) in ((1, 5), (2, 5), (1, 6)):                                      # crest
        P(x, y, "stone", 3)
    for (x, y) in ((7, 8), (8, 8)):                                              # beak, 2 px
        P(x, y, "mortar", 4)
    P(5, 7, "stone", 10)                                                         # eye
    for y in range(11, 20):                                                      # coat: sloped shoulders, long robe
        lo, hi = (2, 6) if y == 11 else ((1, 7) if y < 16 else (1, 8))
        for x in range(lo, hi + 1):
            P(x, y, "stone", 3 if x < hi else 4)
    # right bird-person: 12 px tall (rows 8..19), round head, no crest, 2 px beak pointing left
    for y in range(8, 12):
        for x in range(12, 16):
            if (y in (8, 11)) and x in (12, 15):
                continue
            P(x, y, "stone", 5)
    for (x, y) in ((10, 9), (11, 9)):                                            # beak, 2 px
        P(x, y, "mortar", 4)
    P(13, 9, "stone", 10)                                                        # eye
    for y in range(12, 20):
        lo, hi = (11, 16) if y < 16 else (11, 17)
        for x in range(lo, hi + 1):
            P(x, y, "stone", 3 if x > lo else 4)
    P(12, 12, "brick", 4)                                                        # the one brick accent: a scarf
    P(13, 12, "brick", 4)
    for (x, y) in ((10, 13), (9, 13), (8, 12)):                                  # a hand on the other's shoulder
        P(x, y, "mortar", 5)
    for (x, y) in ((4, 2), (15, 4), (9, 17), (16, 14)):                          # scratches
        P(x, y, "stone", 9)
    c.put(11, 1, "brass", 3)                                                     # nail on the frame, top centre
    c.put(12, 1, "brass", 4)
    return c


def frame_landscape() -> Pix:
    c = Pix(28, 20)
    frame_ring(c, 28, 20)
    c.box(2, 2, 25, 17, "mortar", 2)
    ox, oy = 3, 2
    for y in range(16):
        for x in range(24):
            c.put(ox + x, oy + y, "stone", 7.6 + 0.3 * (1 - y / 15) + (rnd("lsk", x, y) - 0.5) * 0.4)
    base = 12
    for (x0, x1, top) in ((2, 4, 6), (8, 10, 5), (14, 16, 7), (20, 22, 9)):     # arcade piers
        for y in range(top, base + 1):
            for x in range(x0, x1 + 1):
                c.put(ox + x, oy + y, "stone", 5.0 - (0.6 if x == x1 else 0) + (0.4 if x == x0 else 0))
    for (xa, xb) in ((3, 9), (9, 15)):                                           # two surviving arches
        cx, r = (xa + xb) / 2, (xb - xa) / 2
        for x in range(xa, xb + 1):
            y = 5 - round(math.sqrt(max(0.0, r * r - (x - cx) ** 2)) * 0.5)
            for yy in (y + 1, y + 2):
                c.put(ox + x, oy + yy, "stone", 4.6 if yy == y + 1 else 5.4)
    for x in range(14, 17):                                                      # the broken third arch
        c.put(ox + x, oy + 6, "stone", 5)
    for y in range(base + 1, 16):                                                # ground
        for x in range(24):
            c.put(ox + x, oy + y, "stone", 5.2 + (rnd("lg", x, y) - 0.5) * 0.6 - (0.7 if y > 13 else 0))
    for (x, y) in ((6, 11), (12, 12), (18, 12)):                                 # rubble
        c.put(ox + x, oy + y, "stone", 5.6)
    return c


def satchel() -> Pix:
    c = Pix(12, 18)
    c.box(5, 0, 6, 1, "iron", 4)                                                 # wall peg
    c.put(5, 0, "iron", 5)
    for y in range(2, 9):                                                        # strap
        c.put(5 + (y // 5), y, "wood", 3.6 if y % 2 else 2.6)
    for y in range(8, 18):                                                       # bag
        for x in range(1, 11):
            if y == 17 and x in (1, 10):
                continue
            v = 3.4 + (rnd("sat", x, y) - 0.5) * 0.5 + (0.7 if x > 7 else 0) - (0.6 if x < 3 else 0)
            c.put(x, y, "wood", v + (0.3 if y >= 13 else 0))
    for x in range(1, 11):                                                       # flap edge and stitching
        c.put(x, 13, "wood", 1.6)
    for x in range(2, 10, 2):
        c.put(x, 14, "copper", 3)
    c.box(5, 12, 6, 14, "copper", 5)                                             # buckle
    c.put(5, 13, "copper", 2)
    return c


# ------------------------------------------------------------------------------------- window
def window() -> Pix:
    W, H = 96, 100
    cx, spring = 48.0, 52.0
    c = Pix(W, H)
    open_r, ring_r = 38.0, 48.0

    def inside(x, y, r):
        return (y >= spring and abs(x - cx) < r) or (y < spring and math.hypot(x - cx, y - spring) < r)

    for py in range(H):
        for px in range(W):
            x, y = px + 0.5, py + 0.5
            if py >= 94:                                                         # sill, 6 px stone
                v = 6.0 + (0.9 if py == 94 else (-1.4 if py >= 98 else 0)) + (rnd("sill", px // 6, py) - 0.5) * 0.5
                if px % 24 == 11:
                    v = 2.4
                c.put(px, py, "stone", v)
                continue
            if not inside(x, y, ring_r) or inside(x, y, open_r):
                continue
            if y < spring:
                theta = math.atan2(spring - y, x - cx) / math.pi
                k = int(theta * 13)
                joint = abs(theta * 13 - round(theta * 13)) * math.hypot(x - cx, y - spring) * 0.3 < 0.45
                seed = ("wv", k)
            else:
                k = int((y - spring) // 12)
                joint = (y - spring) % 12 < 1
                seed = ("wj", k, x < cx)
            if joint:
                c.put(px, py, "mortar", 2.0)
                continue
            v = 5.6 + (rnd(*seed) - 0.5) * 0.7
            d = math.hypot(x - cx, y - spring) if y < spring else abs(x - cx)
            if d < open_r + 2.5:
                v -= 1.0                                                         # reveal in shade, cold place
            if d > ring_r - 2.5:
                v -= 1.0
            c.put(px, py, "stone", v)
    for py in range(H):                                                          # the far gallery, gal 1..5
        for px in range(W):
            x, y = px + 0.5, py + 0.5
            if py >= 94 or not inside(x, y, open_r):
                continue
            v = 2.0 + 1.2 * (y / 94.0)
            u = (x - 10) % 19
            if 5 <= y <= 93 and u < 1.0:
                v = 1.0                                                          # far pier
            if y > 56 and (u < 5 or u > 14):
                v = 1.4 + 0.6 * (y > 80)                                         # arcade bay in shade
            if y > 56 and 5 <= u <= 14:
                v = 3.2 + 1.3 * ((y - 56) / 38.0)                                # a lighter opening beyond
            v += (rnd("gal", px // 2, py // 2) - 0.5) * 0.6
            c.put(px, py, "gal", clamp(v, 1, 5))
    for py in range(70, 75):                                                     # iron railing: rails and balusters
        for px in range(10, 86):
            if not inside(px + 0.5, py + 0.5, open_r):
                continue
            if py in (70, 74):
                c.put(px, py, "iron", 3.4 if py == 70 else 2.2)
            elif px % 6 == 2:
                c.put(px, py, "iron", 2.6)
    for px in range(24, 73):                                                     # sagging chain inside the arch top
        y = 28 + 12 * math.sin(math.pi * (px - 24) / 48.0)
        if inside(px + 0.5, y, open_r):
            c.put(px, round(y), "iron", 3.8 if px % 3 else 2.4)
    for py in range(14, 20):                                                     # arch-top shade
        for px in range(10, 86):
            p = c.get(px, py)
            if p and p[0] == "gal":
                c.put(px, py, "gal", max(1, p[1] - 1))
    return c


# ------------------------------------------------------------------------------------- flue tiles
FLUE_W, FLUE_H = 48, 16


def flue(level: int) -> Pix:
    """A 48 x 16 vertical repeat tile: courses are 7 px of stone and a 1 px joint (two fill 16), running bond
    with a half-block offset of 8, a mortar contour on both long edges, and a soot plume down the centre."""
    c = Pix(FLUE_W, FLUE_H)
    for y in range(FLUE_H):
        course, row = y // 8, y % 8
        for x in range(FLUE_W):
            off = 8 if course % 2 else 0
            bx = (x + off) % 16
            if x in (0, FLUE_W - 1):
                c.put(x, y, "mortar", 1.2)
                continue
            if row == 7 or bx == 15:
                c.put(x, y, "mortar", 2.0)
                continue
            v = 4.6 + (rnd("flue", (x + off) // 16, course) - 0.5) * 0.9
            if row == 0:
                v += 1.0
            if x in (1, FLUE_W - 2):
                v -= 0.8
            soot = clamp(1 - abs(x - 23.5) / 13.0, 0, 1) * (0.5 + 0.9 * level)
            v -= soot * (1.0 + 0.5 * rnd("sootn", x // 2, y // 2))
            c.put(x, y, "stone", v)
    c.px[0][0] = None                                                            # one clear pixel (1-bit alpha rule)
    return c


# ------------------------------------------------------------------------------------- door, inside face
LEAF_X0, LEAF_X1 = 22, 89     # the leaf spans the opening: 56 +- 34


def door_frame(c, state_open: bool) -> None:
    """The ENV-02 portal frame seen from the room: same arch, voussoirs and jamb courses as ``guild_exit``
    (``portal_frame_pixel`` is reused unchanged; the shape is symmetric, so the hinge side is a leaf detail)."""
    for py in range(opening.PORTAL_H):
        for px in range(opening.PORTAL_W):
            x, y = px + 0.5, py + 0.5
            if not opening.in_arch(x, y, opening.FRAME_HALF, opening.FRAME_R):
                continue
            if (not opening.in_arch(x, y, opening.OPEN_HALF, opening.OPEN_R)
                    or py >= opening.PORTAL_H - 8 + (8 if state_open else 0)):
                opening.portal_frame_pixel(c, px, py, x, y)


def door_closed() -> Pix:
    c = Pix(opening.PORTAL_W, opening.PORTAL_H)
    door_frame(c, False)
    cx = opening.PORTAL_CX
    for py in range(opening.PORTAL_H - 8):
        for px in range(opening.PORTAL_W):
            x, y = px + 0.5, py + 0.5
            if not opening.in_arch(x, y, opening.OPEN_HALF, opening.OPEN_R):
                continue
            lx = x - (cx - opening.OPEN_HALF)
            plank_i, pu = int(lx // 8.5), lx % 8.5
            if py >= opening.PORTAL_H - 10:
                c.put(px, py, "wood", 0.4)                                       # dark gap under the leaf
                continue
            if 74 <= y < 82:                                                     # the drop bar
                v = 4.2 - (1.2 if y >= 80 else 0) + (0.8 if y < 75.5 else 0) + (rnd("bar", int(lx // 6)) - 0.5) * 0.5
                c.put(px, py, "wood", v)
                continue
            if 52 <= y < 60 or 100 <= y < 108:                                   # battens instead of outer studs
                v = 2.6 + (0.9 if int(y) in (52, 100) else 0) - (0.9 if int(y) in (59, 107) else 0)
                c.put(px, py, "wood", v + (rnd("bat", int(lx // 7), int(y)) - 0.5) * 0.3)
                if int(lx) % 11 == 4 and int(y) in (55, 103):
                    c.put(px, py, "iron", 5)                                     # nail head
                continue
            if pu < 1.0:
                c.put(px, py, "wood", 0.5)
                continue
            v = 3.1 + (rnd("dpl", plank_i) - 0.5) * 0.8 + (0.6 if pu < 2.2 else 0) - 0.006 * py
            if rnd("dgr", plank_i, int(py // 9)) < 0.15 and pu > 4:
                v -= 0.8
            c.put(px, py, "wood", v)
    for y in (56, 104):                                                          # hinge straps on the room's left
        for x in range(LEAF_X0, LEAF_X0 + 26):
            c.put(x, y - 1, "iron", 4.4)
            c.put(x, y, "iron", 2.4)
        c.put(LEAF_X0 + 25, y - 1, "iron", 4.4)
    for bx in (LEAF_X0, LEAF_X1 - 3):                                            # drop-bar brackets
        for y in range(70, 86):
            c.put(bx, y, "iron", 3.6)
            c.put(bx + 1, y, "iron", 2.4)
            c.put(bx + 2, y, "iron", 3.0)
    for dy in (-2, -1, 0, 1, 2):                                                 # pull ring
        for dx in (-2, -1, 0, 1, 2):
            if 1.4 < math.hypot(dx, dy) < 2.6:
                c.put(75 + dx, 92 + dy, "brass", 5 if dy < 0 else 3)
    return c


def door_open() -> Pix:
    c = Pix(opening.PORTAL_W, opening.PORTAL_H)
    door_frame(c, True)
    cx = opening.PORTAL_CX
    for py in range(opening.PORTAL_H):                                           # the threshold rows too: same silhouette as guild_portal_open
        for px in range(opening.PORTAL_W):
            x, y = px + 0.5, py + 0.5
            if not opening.in_arch(x, y, opening.OPEN_HALF, opening.OPEN_R):
                continue
            lx = x - (cx - opening.OPEN_HALF)
            if lx < 7 and y > 18:                                                # leaf swung inward against the left jamb
                c.put(px, py, "wood", 1.6 + (1.0 if lx > 5.5 else 0) + (rnd("ol", px, int(py // 6)) - 0.5) * 0.5)
                continue
            if y > opening.PORTAL_H - 22:                                        # corridor floor, cold
                t = (y - (opening.PORTAL_H - 22)) / 22.0
                c.put(px, py, "floor_cold", 1.0 + 2.0 * t + (rnd("cf", px // 3, py) - 0.5) * 0.4)
                continue
            if abs(x - cx) < 15 and y > 50:                                      # a far arch, gal 2-4
                c.put(px, py, "gal", 2 + 0.9 * clamp((y - 50) / 40.0, 0, 1))
                continue
            course = int(y // 7)
            if int(y) % 7 == 0 or (int(x) + 6 * (course % 2)) % 12 == 0:
                c.put(px, py, "brick_cold", 0.4)
            else:
                c.put(px, py, "brick_cold", 1.2 + 1.1 * clamp(1 - abs(x - cx) / 34.0, 0, 1)
                      + (rnd("cb", int(x // 6), course) - 0.5) * 0.6)
    return c


# ------------------------------------------------------------------------------------- atlases
ATLASES = {
    "floor01_refuge_props": {
        "stem": "refuge_props", "width": 160,
        "pieces": [("refuge_bed", bed), ("refuge_bookshelf", bookshelf), ("refuge_desk", desk),
                   ("refuge_stool", stool), ("refuge_barrel", barrel), ("refuge_satchel", satchel)],
    },
    "floor01_refuge_wall": {
        "stem": "refuge_wall", "width": 192,
        "pieces": [("refuge_window", window), ("refuge_wall_map", wall_map), ("refuge_frame_portrait", frame_portrait),
                   ("refuge_frame_landscape", frame_landscape),
                   ("refuge_hearth_flue", lambda: flue(0)), ("refuge_hearth_flue_s1", lambda: flue(1)),
                   ("refuge_hearth_flue_s2", lambda: flue(2)), ("refuge_hearth_flue_s3", lambda: flue(3))],
    },
    "floor01_refuge_door": {
        "stem": "refuge_door", "width": 224,
        "pieces": [("refuge_door_inside_closed", door_closed), ("refuge_door_inside_open", door_open)],
    },
}


def pack(pieces: list[tuple[str, Pix]], width: int) -> tuple[Pix, dict]:
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


def write_candle() -> None:
    grid = candle_grid()
    path = SRC / "refuge_candle.grid.json"
    path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                    str(SRC / "refuge_candle.ase")], check=True)


def write() -> dict:
    write_candle()
    out = {}
    for name, spec in ATLASES.items():
        grid, rects = build(name)
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")
        out[name] = {"rects": rects, "canvas": grid["canvas"], "colours": len(grid["palette"])}
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
        img.resize((img.width * 3, img.height * 3), Image.NEAREST).save(out_dir / f"{spec['stem']}_3x.png")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "sheet":
        sheet(Path(sys.argv[2]))
    else:
        write()
