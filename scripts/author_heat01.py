#!/usr/bin/env python3
"""Authoring record for AREA-HEAT-01: the scorched corral kit of Floor 01 room 0, The Infernal Corral.

Draws the frozen design keys of ``docs/02-art/area-heat-01-spec.md`` on ``floor01-catacomb-v2``
(no new colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/heat_partition.grid.json``  ``heat_partition_intact``, ``_weakened``,
  ``_broken`` (hero key, spec section 6; one 40 x 48 canvas, three frames)
- ``assets/source/props/floor01/heat_scorch.grid.json``     ``heat_scorch_wall_a`` (64 x 96) and
  ``heat_scorch_floor_a`` (72 x 12) (the scorch sample, spec section 7)

The keys are frozen as of the gate (its K2, K3 and the R1..R8 redraw of ``heat_scorch_wall_a`` are applied
here): execution adds the remaining pieces of spec section 5 as new functions and new atlases, and must not
redraw a key (a key change is a design change). Every pixel comes from rules
(ramps, a Bayer threshold, a hash noise); nothing is traced or resampled from a concept image. No text,
logos or likenesses.

    python3 scripts/author_heat01.py              # the four atlas grids + .ase, manifests and the kit contract
    python3 scripts/author_heat01.py gate DIR      # silhouette gate sheets (1x and 4x, scene composites)
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

# 4 x 4 Bayer thresholds (0..15): the only dither in the kit, used for local falloffs inside a piece.
BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("heat01",) + key)).encode()).digest()
    return h[0] / 255.0


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


def bayer(x: int, y: int) -> float:
    return (BAYER[y % 4][x % 4] + 0.5) / 16.0


def vnoise(x: float, y: float, cell: int, seed: str) -> float:
    """Smooth value noise (bilinear over a hashed lattice of ``cell`` px): clumped, never a checker."""
    gx, gy = x / cell, y / cell
    x0, y0 = math.floor(gx), math.floor(gy)
    fx, fy = gx - x0, gy - y0
    fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
    a, b = rnd(seed, x0, y0), rnd(seed, x0 + 1, y0)
    c, d = rnd(seed, x0, y0 + 1), rnd(seed, x0 + 1, y0 + 1)
    return (a + (b - a) * fx) * (1 - fy) + (c + (d - c) * fx) * fy


def clump(x: int, y: int, seed: str) -> float:
    """Two octaves of value noise, 0..1: the threshold field for soot, rot and char edges."""
    return 0.65 * vnoise(x, y, 6, seed) + 0.35 * vnoise(x, y, 3, seed + "f")


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

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = None

    def box(self, x0, y0, x1, y1, ramp, v):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, ramp, v)

    def darken(self, x, y, steps=1):
        p = self.get(x, y)
        if p is not None:
            self.put(x, y, p[0], p[1] - steps)

    def copy(self) -> "Pix":
        c = Pix(self.w, self.h)
        c.px = [row[:] for row in self.px]
        return c


# =============================================================================================
# Hero key: the goblin-built partition (spec section 6). One 40 x 48 canvas for all three states, placed
# top-left at world (1380, 532): the CORRAL-D solid 16 x 48 at x 1392..1407, y 532..579 is canvas
# x 12..27, rows 0..47 (row 47 is the walking line's last pixel row, y 579). Inside the solid columns the
# drawn timber is the collider (the cap's top row is the bonk/stand line); the margins x 0..11 and
# 28..39 hold only soft things (a rope tail, ash, straw, flat debris at most 3 rows tall).
# =============================================================================================
PT_W, PT_H = 40, 48
SX0, SX1 = 12, 27                  # solid columns
POST_L = (12, 14)
POST_R = (25, 27)
CAP = (0, 3)                       # cap rail rows (row 0 = lit arris = the stand line)
BOARDS = tuple((4 + 8 * k, 4 + 8 * k + 6) for k in range(5))   # five 7-row boards, 1-row gaps
SILL = (44, 47)
BROKEN_TOP = 32                    # the 16 px remnant: rows 32..47


def _post(c: Pix, x0: int, x1: int, top: int = 0) -> None:
    for y in range(top, PT_H):
        c.put(x0, y, "wood", 5 if x0 == POST_L[0] else 4)      # lit face toward the key light (left)
        c.put(x0 + 1, y, "wood", 4)
        c.put(x1, y, "wood", 1 if x1 == POST_R[1] else 2)
        if rnd("post", x0, y) > 0.86:
            c.put(x0 + 1, y, "wood", 2)                         # grain


def _board(c: Pix, k: int, y0: int, y1: int, x0: int = SX0, x1: int = SX1) -> None:
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if y == y0:
                v = 5                                          # lit top edge
            elif y == y1:
                v = 2                                          # shaded bottom edge
            else:
                v = 4 if y <= y0 + 3 else 3                    # mid value: the partition must not sink into the dark lane
                if rnd("grain", k, y, x // 3) > 0.78:
                    v -= 1                                     # grain streaks run along the board
            c.put(x, y, "wood", v - (1 if k == 1 and v > 2 and rnd("tone", x, y) > 0.6 else 0))
    # end grain and a nail head on each board end (over the posts)
    c.put(x0, y0 + 1, "wood", 4)
    c.put(x0 + 1, y0 + 2, "iron", 6)
    c.put(x0 + 1, y0 + 3, "iron", 2)
    if x1 == SX1:
        c.put(x1 - 1, y0 + 3, "iron", 5)
        c.put(x1 - 1, y0 + 4, "iron", 2)


def _rot_and_soot(c: Pix, x0: int = SX0, x1: int = SX1, y_from: int = 26) -> None:
    """Rot on the lower boards and soot rising from the floor: darker toward the bottom (dithered)."""
    for y in range(y_from, SILL[0]):
        for x in range(x0, x1 + 1):
            p = c.get(x, y)
            if p is None or p[0] != "wood":
                continue
            d = (y - y_from) / (SILL[0] - y_from)
            if d * 0.9 > 0.25 + 0.75 * clump(x, y * 2, "soot"):         # stretched along the grain
                c.darken(x, y)
    rot = ((15, 36), (16, 36), (15, 37), (21, 41), (22, 41), (23, 41), (24, 41), (19, 31), (20, 31))
    for x, y in rot:
        if c.get(x, y) is not None:
            c.put(x, y, "wood", 0)
    for x, y in ((16, 37),):
        if c.get(x, y) is not None:
            c.put(x, y, "lichen", 0)                           # damp rot, one pixel, never a green patch (gate K2)


def _sill(c: Pix, x0: int = SX0, x1: int = SX1) -> None:
    for x in range(x0, x1 + 1):
        c.put(x, SILL[0], "wood", 3)
        c.put(x, SILL[0] + 1, "wood", 2)
        c.put(x, SILL[0] + 2, "wood", 1 if (x + 1) % 5 else 2)
        c.put(x, SILL[1], "wood", 0 if x in (x0, x1) else 1)
    # foot litter in the margins: ash (warm grey) and straw, two rows at most, flat on the deck
    for x, y, r, v in ((9, 47, "iron", 5), (10, 47, "iron", 4), (11, 46, "iron", 5), (11, 47, "iron", 3),
                       (7, 47, "brass", 3), (8, 47, "brass", 2), (28, 47, "iron", 4), (28, 46, "iron", 5),
                       (29, 47, "iron", 5), (30, 47, "brass", 3), (31, 47, "brass", 2), (33, 47, "iron", 4)):
        c.put(x, y, r, v)


def _cap(c: Pix) -> None:
    for x in range(SX0, SX1 + 1):
        c.put(x, 0, "stone", 8 if SX0 < x < SX1 else 7)        # lit arris: the brightest run of the piece
        c.put(x, 1, "wood", 5)
        c.put(x, 2, "wood", 4 if rnd("cap", x) > 0.25 else 3)
        c.put(x, 3, "wood", 1)                                 # shadow under the cap
    c.put(SX1, 0, "wood", 4)
    for y in (1, 2):                                           # end grain at both cap ends
        c.put(SX0, y, "wood", 3)
        c.put(SX1, y, "wood", 2)


def _lashing(c: Pix, x0: int, x1: int) -> None:
    """Rope wraps where the cap meets a post: two diagonal turns over rows 2..6."""
    for y in range(2, 7):
        for x in range(x0, x1 + 1):
            c.put(x, y, "stone", 6 if (x + y) % 3 else 4)
    c.put(x0, 7, "stone", 4)


def _brace(c: Pix, y_from: int = 5, y_to: int = 42, keep=lambda y: True) -> list:
    """The diagonal brace from the bottom-left nail (13, 42) to the top-right nail (26, 5), 3 px wide."""
    drawn = []
    for y in range(y_from, y_to + 1):
        if not keep(y):
            continue
        x = round(13 + (42 - y) * 12 / 37)
        c.put(x, y, "wood", 5)
        c.put(x + 1, y, "wood", 3)
        c.put(x + 2, y, "wood", 1)                             # its shadow on the boards
        drawn.append((x, y))
    return drawn


def _gaps(c: Pix) -> None:
    """The 1-row gaps between boards: dark depth between the posts (never transparent)."""
    for k in range(len(BOARDS) - 1):
        y = BOARDS[k][1] + 1
        for x in range(POST_L[1] + 1, POST_R[0]):
            c.put(x, y, "wood", 0)
    for x in range(POST_L[1] + 1, POST_R[0]):
        c.put(x, BOARDS[-1][1] + 1, "wood", 0)


def heat_partition_intact() -> Pix:
    c = Pix(PT_W, PT_H)
    _post(c, *POST_L)
    _post(c, *POST_R)
    _gaps(c)
    for k, (y0, y1) in enumerate(BOARDS):
        _board(c, k, y0, y1)
    _brace(c)
    c.put(13, 41, "iron", 6)                                   # brace nails
    c.put(25, 6, "iron", 6)
    _rot_and_soot(c)
    _sill(c)
    _cap(c)
    _lashing(c, *POST_L)
    _lashing(c, *POST_R)
    for y in range(5, 10):                                     # the left lashing's rope tail (soft, margin)
        c.put(11, y, "stone", 5 if y % 2 else 4)
    c.put(10, 10, "stone", 4)
    return c


def _splinter(c: Pix, x: int, y: int, v: int = 8) -> None:
    """A fresh broken face: pale raw wood (the only pale wood in the kit)."""
    c.put(x, y, "stone", v)


def heat_partition_weakened() -> Pix:
    c = Pix(PT_W, PT_H)
    _post(c, *POST_L)
    _post(c, *POST_R)
    _gaps(c)
    for k, (y0, y1) in enumerate(BOARDS):
        if k == 3:
            _board(c, k, y0, y1, SX0, 21)                      # board 3 snapped off at x 21
        else:
            _board(c, k, y0, y1)
    # board 3's missing right end: dark depth behind, the right post shows, pale broken end
    for y in range(BOARDS[3][0], BOARDS[3][1] + 1):
        for x in range(22, POST_R[0]):
            c.put(x, y, "wood", 0)
    for y, v in ((28, 8), (29, 9), (30, 8), (31, 7), (33, 8)):
        _splinter(c, 21 if y != 30 else 22, y, v)
    # board 2 cracked: a diagonal split, the right part sags one row; pale faces along the split
    y0, y1 = BOARDS[2]
    for x in range(26, 18, -1):
        for y in range(y1 + 1, y0, -1):
            c.px[y][x] = c.px[y - 1][x]
        c.put(x, y0, "wood", 0)
    crack = ((14, 22), (15, 22), (16, 23), (17, 23), (18, 24), (19, 24), (20, 25), (21, 25), (22, 26))
    for x, y in crack:
        c.put(x, y, "wood", 0)
        if (x + y) % 2 == 0:
            _splinter(c, x, y - 1, 7)
    # the brace snapped at row 24: the upper half fell, a stub hangs from the top nail
    lower = _brace(c, 25, 42)
    _brace(c, 5, 9)
    _splinter(c, lower[0][0], lower[0][1], 9)
    _splinter(c, lower[0][0] + 1, lower[0][1], 8)
    stub = [(x, y) for x, y in ((24, 9), (25, 9))]
    for x, y in stub:
        _splinter(c, x, y, 8)
    c.put(13, 41, "iron", 6)
    c.put(25, 6, "iron", 6)
    _rot_and_soot(c)
    _sill(c)
    _cap(c)
    _lashing(c, *POST_R)
    # the left lashing tore: a frayed stub on the post, the rope hangs longer into the left margin
    c.put(12, 2, "stone", 5)
    c.put(13, 3, "stone", 4)
    for y in range(4, 14):
        c.put(11 - (1 if 9 <= y <= 11 else 0), y, "stone", 5 if y % 2 else 4)
    c.put(11, 14, "stone", 4)
    return c


def heat_partition_broken() -> Pix:
    c = Pix(PT_W, PT_H)
    t = BROKEN_TOP
    # post stumps, snapped with short jagged tops (at most 3 px above the remnant top, 1 px wide)
    _post(c, *POST_L, top=t)
    _post(c, *POST_R, top=t)
    for x in range(POST_L[1] + 1, POST_R[0]):
        c.put(x, BOARDS[3][1] + 1, "wood", 0)
        c.put(x, BOARDS[4][1] + 1, "wood", 0)
    # the lower part of board 3, snapped along its grain: a flat pale broken edge on row 32
    for x in range(SX0, SX1 + 1):
        c.put(x, t, "stone", 8 if (x * 7) % 5 else 7)
        c.put(x, t + 1, "wood", 4)
        c.put(x, t + 2, "wood", 2)
    _board(c, 4, *BOARDS[4])
    _rot_and_soot(c, y_from=t + 3)
    _sill(c)
    for x, h in ((13, 3), (19, 2), (26, 3), (23, 1)):          # splinter spikes, dark-tipped so the edge stays flat
        for k in range(1, h + 1):
            c.put(x, t - k, "stone" if k == 1 else "wood", 7 if k == 1 else 1)
    # flat debris in the margins: at most 3 rows tall, lit tops, never a dark run longer than 2 px
    for x in range(0, 11):                                     # a board lying flat, left
        c.put(x, 45, "wood", 4)
        c.put(x, 46, "wood", 3 if x % 4 else 2)
        c.put(x, 47, "wood", 2)
    _splinter(c, 0, 45, 8)
    _splinter(c, 10, 45, 7)
    c.put(4, 46, "iron", 6)
    for x in range(30, 38):                                    # a shorter fragment, right
        c.put(x, 46, "wood", 4)
        c.put(x, 47, "wood", 2)
    _splinter(c, 30, 46, 8)
    for x, y in ((3, 44), (5, 44), (6, 44)):                   # the brace piece leaning on the board (thin)
        c.put(x, y, "wood", 5)
    for x, y, r, v in ((29, 47, "stone", 7), (38, 47, "wood", 3), (39, 47, "stone", 6), (28, 45, "wood", 3),
                       (11, 46, "stone", 7)):
        c.put(x, y, r, v)
    return c


# =============================================================================================
# The scorch sample (spec section 7): a wall bloom and a floor char, the rules every other scorch piece
# follows.
# =============================================================================================
SW_W, SW_H = 64, 96                # heat_scorch_wall_a, backwall_decal, bottom row at y 559 (the deck back line - 1)
# Gate redraw (R1..R8): a whole-brick re-shade of the corral wall in the ENV-01 bond, a soot plume that widens
# upward from a narrow foot and thins out brick by brick at the top, over a pale stone base course that only
# darkens. Placed at world (1088, 464): rows 0..79 are ten 8 px wall brick rows (two mid, four lower, four low
# rows of 8 px, bond offset 8 * (row % 2) as in the world), rows 80..95 are the base course (32 px blocks,
# joints at world x % 32 == 16, i.e. local x % 32 == 16). Plume bricks start at local x 16 (world 1104), clear
# of the torch pool of the torch at world x 1016, whose bricks sit 4 px off the wall bond.
SW_ROW_BAND = ["mid"] * 2 + ["lower"] * 4 + ["low"] * 4
SW_SOOT_FLOOR = 0.65               # every plume pixel at least the world's brick-jitter half-range darker: never a lighter brick
SW_HEAD_ROW = 60                   # pixel rows >= 60 lie behind the actors' heads and torsos: never darker than brick 3
# The plume function is shared by every wall scorch (spec section 5: wall_b and wall_c reuse it with their own
# parameter dict, ``SW_B`` and ``SW_C`` below). The keys' own parameters are the dict ``SW_PLUME``: its extra
# entries (canvas, band map, seed prefix, darkness floor, base course, crazing, embers) describe wall_a and
# leave its pixels unchanged (pin 8ea59587..., HEAT-AD8).
SW_PLUME = dict(
    foot_x=28.0, lean=0.3,         # core centre at the fire's foot; drift per px of height (positive: to the right)
    sig0=4.0, sig_grow=0.25,       # lateral spread of the soot at the foot, growth per px of height
    amp=4.2, amp_top=0.2,          # soot at the core just above the fire; fraction left at the top
    fade_top=40,                   # brick rows centred above this thin out brick by brick
    thr=1.4,                       # a brick joins the plume when its strongest sample reaches this soot
    base_s=1.0, base_rx=14.0, base_ry=18.0,   # stone re-shade dome on the base course (at most -1.0)
    x_min=16, base_x=(8, 32),      # plume bricks from local x 16; base-course re-shade over local x 8..31
    w=SW_W, h=SW_H, rows=80,       # canvas, and the brick pixel rows (the base course follows from row ``rows``)
    bands=SW_ROW_BAND, seed="",    # band per 8 px brick row; seed prefix ("" keeps wall_a's own seeds)
    head_row=SW_HEAD_ROW, lo_head=3, lo_above=2,   # darkest brick index at and above ``head_row`` (rule 3)
    base_y0=80,                    # first base-course row (None: no base course)
    crazing=(((-2, 91), (-3, 90), (-3, 89), (-4, 88), (-5, 87)), ((3, 90), (3, 89), (4, 88), (4, 87))),
    embers=((-3, 93, 0), (0, 94, 1), (2, 92, 0), (5, 94, 0)),   # (dx from foot_x, row, flame index)
)
# wall_b (48 x 64 at (1920, 480)): behind the dock's head band, so no brick darker than index 3, no base course,
# no crazing, no embers (the dock is llama A's glob height). Spec section 5. foot_x is 16 (HEAT-AD20, the review's
# F2): the spec's 22 put the core on the joint between the two bottom-row bricks, so the foot was 32 px wide.
SW_B = dict(SW_PLUME, foot_x=16.0, lean=0.2, sig0=3.5, sig_grow=0.22, amp=3.4, amp_top=0.25, fade_top=24,
            thr=1.2, x_min=0, w=48, h=64, rows=64, seed="wall_b", bands=["lower"] * 4 + ["low"] * 4,
            head_row=0, lo_head=3, lo_above=3, base_y0=None, crazing=(), embers=())
# wall_c (32 x 48 at x % 32 == 0, y 512): a small low lick, four `low` brick rows over a base course that only
# darkens (the same stone dome as wall_a), two `flame` 0 embers in rows 44..47, no crazing. Spec section 5.
SW_C = dict(SW_PLUME, foot_x=14.0, lean=0.15, sig0=3.0, sig_grow=0.2, amp=3.0, amp_top=0.3, fade_top=12,
            thr=1.1, x_min=0, base_x=(0, 32), w=32, h=48, rows=32, seed="wall_c", bands=["low"] * 4,
            head_row=0, lo_head=3, lo_above=3, base_y0=32, crazing=(), embers=((-1, 46, 0), (3, 45, 0)))


def _sk(p, *key):
    """A seed key: wall_a's own seeds carry no prefix (its pixels are frozen), the others start with their name."""
    return (p["seed"],) + key if p["seed"] else key


def _plume_soot(x: float, y: float, p=SW_PLUME) -> float:
    """Soot at a pixel: a leaning, widening plume, darkest just above the fire. Low frequency only (no strands)."""
    top = p["rows"] - 0.5
    d = (top - y) / top
    sig = p["sig0"] + p["sig_grow"] * (top - y)
    a = p["amp"] * (1.0 - (1.0 - p["amp_top"]) * d ** 1.1)
    u = (x - (p["foot_x"] + p["lean"] * (top - y))) / sig
    taper = clamp((x - p["x_min"]) / 8.0, 0.0, 1.0) * clamp((p["w"] - x) / 8.0, 0.0, 1.0)
    return a * math.exp(-u * u) * taper * (0.85 + 0.3 * vnoise(x, y, 20, "plume_mod" if not p["seed"] else p["seed"] + ":plume_mod"))


def _plume_brick(r: int, col: int, p=SW_PLUME) -> bool:
    """A brick is in the plume or out (whole-brick rule); the top rows thin out brick by brick."""
    x0 = col * 16 - 8 * (r % 2)
    if x0 < p["x_min"] or x0 + 16 > p["w"]:
        return False
    peak = max(_plume_soot(x + 0.5, y + 0.5, p) for y in range(r * 8, r * 8 + 8, 2) for x in range(x0, x0 + 16, 2))
    if peak < p["thr"] + (rnd(*_sk(p, "plume_edge", r, col)) - 0.5) * 0.3:
        return False
    by = r * 8 + 4.0
    if by < p["fade_top"]:
        return rnd(*_sk(p, "plume_thin", r, col)) < (by / p["fade_top"]) ** 0.7 + 0.15
    return True


def plume_members(p=SW_PLUME) -> dict:
    """The bricks of the plume, (row, column) -> bool: bottom-up, a brick joins only if it rests on a plume brick
    (no floaters)."""
    n_rows, n_cols = p["rows"] // 8, p["w"] // 16 + 2
    members = {}
    for r in range(n_rows - 1, -1, -1):
        for col in range(n_cols):
            x0 = col * 16 - 8 * (r % 2)
            below = r == n_rows - 1 or any(
                members.get((r + 1, k)) and abs(k * 16 - 8 * ((r + 1) % 2) - x0) < 16 for k in range(n_cols))
            members[(r, col)] = below and _plume_brick(r, col, p)
    return members


def wall_scorch(p=SW_PLUME) -> Pix:
    import lookdev_floor01_author as ld  # noqa: PLC0415  (the kit's shading core: bevel normals)
    c = kit.canvas(p["w"], p["h"])
    members = plume_members(p)
    for r in range(p["rows"] // 8):
        _rows, base, mortar = kit.BANDS[p["bands"][r]]
        for col in range(p["w"] // 16 + 2):
            if not members[(r, col)]:
                continue
            x0 = col * 16 - 8 * (r % 2)
            for py in range(r * 8, r * 8 + 8):
                for px in range(x0, x0 + 16):
                    kit.brick_pixel(c, px, py, px - x0, py - r * 8, base=base, mortar_base=mortar, jit=0.0,
                                    seed=("heat_plume", r, col) if not p["seed"] else (p["seed"], "plume", r, col),
                                    chips=False, soot=max(SW_SOOT_FLOOR, _plume_soot(px + 0.5, py + 0.5, p)))
                    q = c.px[py][px]
                    lo = p["lo_head"] if py >= p["head_row"] else p["lo_above"]
                    if q and q[0] == "brick" and q[1] < lo:
                        c.px[py][px] = ("brick", lo)
    fx = p["foot_x"]
    if p["base_y0"] is not None:
        # the base course stays pale stone: a low dome of re-shade at the fire's foot, at most -1.0
        y0 = p["base_y0"]
        for py in range(y0, p["h"]):
            for px in range(*p["base_x"]):
                u = ((px + 0.5 - fx) / p["base_rx"]) ** 2 + ((p["h"] - 0.5 - py) / p["base_ry"]) ** 2
                s = p["base_s"] * (1.0 - u)
                if s < 0.3:
                    continue
                du, dv = (px + 16) % 32, py - y0
                if du == 0 or dv == 0:
                    kit.put(c, px, py, "mortar", (2.1 if dv else 1.4) - 0.5 * s)
                elif dv == 15:
                    kit.put(c, px, py, "stone", 2.0)
                else:
                    nrm, _e = ld.bevel_normal(du - 1, dv - 1, 31, 14, 1)
                    kit.put(c, px, py, "stone", 6.8 + 1.6 * kit.key_term(nrm) - 0.12 * dv - s)
    # heat crazing: 1 px hairlines licking up from the fire on the stone (diagonal, never a horizontal pair)
    fx = int(fx)
    for path in p["crazing"]:
        for dx, y in path:
            q = c.px[y][fx + dx]
            if q is not None and q[0] == "stone":
                kit.put(c, fx + dx, y, "mortar", 1)
    for dx, y, v in p["embers"]:                                        # embers: flame 0..1, single pixels
        kit.put(c, fx + dx, y, "flame", v)
    out = Pix(p["w"], p["h"])
    out.px = [row[:] for row in c.px]
    return out


def heat_scorch_wall_a() -> Pix:
    return wall_scorch(SW_PLUME)


def heat_scorch_wall_b() -> Pix:
    return wall_scorch(SW_B)


def heat_scorch_wall_c() -> Pix:
    return wall_scorch(SW_C)


SF_W, SF_H = 72, 12                # heat_scorch_floor_a, structure layer, top row at y 561 (back half of the deck)


def heat_scorch_floor_a() -> Pix:
    c = Pix(SF_W, SF_H)
    cx, cy, rx, ry = 35.5, 5.5, 35.0, 5.6
    for y in range(SF_H):
        for x in range(SF_W):
            d = 1 - ((x - cx) / rx) ** 2 - ((y - cy) / ry) ** 2
            if d <= 0:
                continue
            n = clump(x, y * 3, "sf")                          # clumps stretched along the deck
            if d > 0.12 + 0.75 * n:
                c.put(x, y, "floor", 2 if d > 0.5 + 0.3 * n else (3 if d > 0.3 + 0.3 * n else 4))
    # ash flecks on the char (warm grey), a slab joint carried through the char
    for x, y in ((14, 4), (22, 7), (27, 3), (33, 6), (40, 4), (45, 8), (51, 5), (58, 6), (31, 9), (19, 6)):
        if c.get(x, y) is not None:
            c.put(x, y, "iron", 6 if (x + y) % 3 else 5)
    for y in range(1, SF_H - 1):
        if c.get(38, y) is not None:
            c.put(38, y, "floor", 0)
    return c


# =============================================================================================
# Execution: every non-key piece of spec section 5 (rules and parameters of sections 11.4..11.6 and the
# spec rows). Seeds are ``rnd("<piece id>", ...)``: deterministic, no global RNG.
# =============================================================================================
def heat_trace_wall_a() -> Pix:
    """A spit splash on the brick, 16 x 16: a ragged dark star 9..11 px wide (a ``brick`` 3 core of about 12
    px, a ``brick`` 4 rim, three 1 px drips down 2..3 px), centred at (8, 10); one ``flame`` 0 and one
    ``flame`` 1 pixel at rows 11..13."""
    c = Pix(16, 16)
    cx, cy = 8.0, 8.0
    for y in range(16):
        for x in range(16):
            dx, dy = x + 0.5 - cx - 0.5, y + 0.5 - cy - 0.5
            r = math.hypot(dx, dy)
            ang = math.atan2(dy, dx)
            wob = 0.9 * math.sin(3 * ang + 5 * rnd("wall_a_trace", "ph")) + 0.5 * (rnd("wall_a_trace", x, y) - 0.5)
            if r < 2.0 + 0.2 * wob:
                c.put(x, y, "brick", 3)
            elif r < 4.6 + wob:
                c.put(x, y, "brick", 4)
    # three drips from the lowest rim pixels of columns 5, 8, 11 (2..3 px)
    for col, length in ((5, 2), (8, 3), (11, 2)):
        ys = [y for y in range(16) if c.get(col, y) is not None]
        if not ys:
            continue
        y0 = max(ys)
        for k in range(1, length + 1):
            c.put(col, min(15, y0 + k), "brick", 3 if k > 1 else 4)
    c.put(7, 12, "flame", 0)
    c.put(9, 12, "flame", 1)
    return c


def _rows_in(img: Pix, pred) -> int:
    return sum(1 for row in img.px for q in row if q and pred(q))


def _char_patch(w, h, cx, cy, rx, ry, seed) -> Pix:
    """``heat_scorch_floor_a``'s method for a smaller patch: an ellipse whose edge is broken by clump noise (sampled
    at ``(x, 3 y)``), values ``floor`` 2/3/4, darkest at the centre."""
    c = Pix(w, h)
    for y in range(h):
        for x in range(w):
            d = 1 - ((x - cx) / rx) ** 2 - ((y - cy) / ry) ** 2
            if d <= 0:
                continue
            n = clump(x, y * 3, seed)
            if d > 0.12 + 0.75 * n:
                c.put(x, y, "floor", 2 if d > 0.5 + 0.3 * n else (3 if d > 0.3 + 0.3 * n else 4))
    return c


def _flecks(c: Pix, seed: str, n: int, ramp_pick) -> None:
    """``n`` single-pixel ash flecks on the opaque pixels, at least 4 px apart, chosen by hash."""
    cands = sorted(((rnd(seed, "fleck", x, y), x, y) for y in range(c.h) for x in range(c.w) if c.px[y][x] is not None),
                   reverse=True)
    chosen = []
    for _, x, y in cands:
        if all(abs(x - a) + abs(y - b) >= 4 for a, b in chosen):
            chosen.append((x, y))
            c.put(x, y, "iron", ramp_pick(x, y))
        if len(chosen) == n:
            break


def heat_scorch_floor_b() -> Pix:
    c = _char_patch(48, 10, 23.5, 5.0, 22.0, 4.2, "floor_b")
    _flecks(c, "floor_b", 5, lambda x, y: 6 if rnd("floor_b", "tone", x, y) > 0.4 else 5)
    return c


def _ash_drift(w: int, h: int, seed: str, centre: float) -> Pix:
    """A drift of ash on the deck: ``iron`` 5 and 6 flecks (1..2 px each, never a horizontal run over 3 px, at most
    30 % of the pixels) over a sparse ``floor`` 4 smudge (at most 20 %), densest at ``centre``, fading to single
    flecks at both ends."""
    c = Pix(w, h)
    half = max(centre, w - 1 - centre)
    dens = [max(0.0, 1.0 - abs(x - centre) / (half + 2.0)) ** 1.3 for x in range(w)]
    for y in range(h):
        for x in range(w):
            if rnd(seed, "smudge", x, y) < 0.02 + 0.22 * dens[x] * (0.6 + 0.8 * vnoise(x, y * 2, 4, seed)):
                c.put(x, y, "floor", 4)
    for y in range(h):
        x = 0
        while x < w:
            if rnd(seed, "fleck", x, y) < 0.02 + 0.30 * dens[x]:
                length = 2 if x + 1 < w and rnd(seed, "len", x, y) > 0.65 else 1      # 1..2 px per fleck
                for k in range(length):
                    c.put(x + k, y, "iron", 6 if rnd(seed, "tone", x + k, y) > 0.45 else 5)
                x += length + 1                                  # a free pixel after each fleck: runs stay at 2
            else:
                x += 1
    return c


def heat_ash_floor_a() -> Pix:
    return _ash_drift(32, 6, "ash_a", 15.5)


def heat_ash_floor_b() -> Pix:
    return _ash_drift(24, 6, "ash_b", 8.5)


def _blot(w: int, h: int, cx: float, cy: float, rx: float, ry: float, seed: str, spatter: int) -> Pix:
    c = Pix(w, h)
    for y in range(h):
        for x in range(w):
            d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
            if d < 1.0 + 0.5 * (rnd(seed, "edge", x, y) - 0.5):
                c.put(x, y, "floor", 2)
    dots = sorted(((rnd(seed, "dot", x, y), x, y) for y in range(h) for x in range(w) if c.px[y][x] is None),
                  reverse=True)
    placed = []
    for _, x, y in dots:
        if all(abs(x - a) + abs(y - b) >= 3 for a, b in placed):
            near = any(c.get(x + dx, y + dy) is not None for dx in (-1, 0, 1) for dy in (-1, 0, 1))
            if near or len(placed) >= spatter // 2:
                placed.append((x, y))
                c.put(x, y, "floor", 3)
        if len(placed) == spatter:
            break
    return c


def heat_trace_spit_a() -> Pix:
    c = _blot(16, 6, 7.5, 2.5, 4.6, 1.7, "spit_a", 6)
    c.put(7, 2, "flame", 0)
    c.put(9, 2, "flame", 1)
    return c


def heat_trace_spit_b() -> Pix:
    c = _blot(12, 6, 5.5, 2.5, 3.1, 1.2, "spit_b", 3)
    c.put(5, 2, "flame", 0)
    return c


SHELF_W, SHELF_H = 16, 32


def _shelf_body() -> Pix:
    """``heat_shelf_mid_a`` (spec section 11.4): the walkable arris, the slab lip, two block courses."""
    c = Pix(SHELF_W, SHELF_H)
    for x in range(SHELF_W):
        c.put(x, 0, "stone", 8)                                 # the walkable arris (HEAT-AD7)
        c.put(x, 1, "stone", 6)
        c.put(x, 2, "stone", 6)
        for y in range(3, 9):
            c.put(x, y, "stone", 4 if rnd("shelf", "grain", x, y) < 0.10 else 5)
        c.put(x, 9, "mortar", 0)                                # the shadow under the slab lip
        c.put(x, 20, "mortar", 1)                               # the bed joint between the courses
    for top, bot, joint in ((10, 19, 0), (21, 31, 8)):
        for y in range(top, bot + 1):
            for x in range(SHELF_W):
                if x == joint:
                    c.put(x, y, "mortar", 1)
                elif y == top:
                    c.put(x, y, "stone", 4)
                elif y == bot:
                    c.put(x, y, "stone", 2)
                elif x == (joint + 1) % SHELF_W:
                    c.put(x, y, "stone", 4)                     # the lit left arris right of a head joint
                else:
                    c.put(x, y, "stone", 3)
    c.clear(15, 31)                                             # the transparent nick
    return c


def heat_shelf_mid_a() -> Pix:
    return _shelf_body()


def heat_shelf_mid_b() -> Pix:
    """``_a`` plus char: a soot tongue rising from row 31 to row 14 over columns 3..11 (clump noise, ``stone``
    1..2, densest at the bottom) and one burnt-red ``brick`` 5 pixel pair at its foot."""
    c = _shelf_body()
    for y in range(14, 32):
        t = (31 - y) / 17.0                                      # 0 at the foot, 1 at the top of the tongue
        for x in range(3, 12):
            q = c.get(x, y)
            if q is None or q[0] != "stone":
                continue
            lat = abs(x - 7.0) / 4.5
            n = clump(x, y, "shelf_b")
            dark = (1.0 - t) * (1.0 - 0.45 * lat)
            if n < dark:
                c.put(x, y, "stone", 1 if dark - n > 0.28 else 2)
    c.put(6, 30, "brick", 5)
    c.put(7, 30, "brick", 5)
    return c


def _shelf_end(left: bool) -> Pix:
    """``_a``'s rows 8 px wide (the left cap takes columns 8..15 of the body, the right cap columns 0..7, so the
    courses run on), the outer two columns one step darker (the return face), the outer pixel of row 0 ``stone``
    6 (a rounded corner), one transparent nick at the outer bottom pixel. Row 0 stays ``stone`` 8 elsewhere
    (HEAT-AD7: every walkable top's first row)."""
    body = _shelf_body()
    c = Pix(8, SHELF_H)
    x0 = 8 if left else 0
    for y in range(SHELF_H):
        for x in range(8):
            q = body.get(x0 + x, y)
            if q is None and (x0 + x, y) == (15, 31):
                q = ("stone", 2)                                 # the body's nick is not this piece's
            c.px[y][x] = q
    outer = (0, 1) if left else (6, 7)
    for y in range(1, SHELF_H):
        for x in outer:
            q = c.px[y][x]
            if q is not None:
                c.put(x, y, q[0], q[1] - 1)
    c.put(0 if left else 7, 0, "stone", 6)
    c.clear(0 if left else 7, SHELF_H - 1)
    return c


def heat_shelf_end_l() -> Pix:
    return _shelf_end(True)


def heat_shelf_end_r() -> Pix:
    return _shelf_end(False)


def heat_bumper() -> Pix:
    """The dock bumper (spec section 11.5), 32 x 80: goblin-stacked squared timbers seen end-on, capped with a
    salvaged slab."""
    c = Pix(32, 80)
    for x in range(32):
        c.put(x, 0, "stone", 8)                                 # the walkable top
        c.put(x, 1, "stone", 6)
        c.put(x, 2, "stone", 5)
        c.put(x, 3, "mortar", 0)
    blocks = [(bx, by, 16 if by < 68 else 12) for by in (4, 20, 36, 52, 68) for bx in (0, 16)]
    for n, (bx, by, bh) in enumerate(blocks):
        for y in range(bh):
            for x in range(16):
                edge = x in (0, 15) or y in (0, bh - 1)
                c.put(bx + x, by + y, "wood", 0 if edge else 3)
        for y in range(1, bh - 1):                              # the lit top-left arris
            c.put(bx + 1, by + y, "wood", 4)
        for x in range(1, 15):
            c.put(bx + x, by + 1, "wood", 4)
        cx, cy = 7.5, 7.5
        for r in (3, 6):                                        # two growth rings (squares, rounded corners)
            for y in range(16):
                for x in range(16):
                    ax, ay = abs(x - cx), abs(y - cy)
                    if max(ax, ay) == r + 0.5 and min(ax, ay) < r + 0.5 and 1 < x < 14 and 1 < y < bh - 2:
                        c.put(bx + x, by + y, "wood", 2)       # (the four corners of each square are left out)
        ang = rnd("bumper", "crack", n) * 2 * math.pi            # one radial check crack, 3..5 px
        ln = 3 + int(rnd("bumper", "len", n) * 3)
        for k in range(1, ln + 1):
            x, y = int(round(cx + k * math.cos(ang))), int(round(cy + k * math.sin(ang)))
            if 1 < x < 14 and 1 < y < bh - 2:
                c.put(bx + x, by + y, "wood", 0)
    for y0 in (35, 67):                                          # two iron straps across the full width
        for dy in (0, 1):
            for x in range(32):
                c.put(x, y0 + dy, "iron", 2)
        for x in (4, 12, 20, 28):
            c.put(x, y0, "iron", 5)
    for y in range(52, 80):                                      # char on the two lowest rows of timbers
        for x in range(32):
            q = c.get(x, y)
            if q is None or q[0] != "wood" or c.get(x, y)[1] == 0:
                continue
            t = (y - 52) / 27.0
            if clump(x, y, "bumper_char") < 0.30 + 0.35 * t:
                c.put(x, y, "wood", q[1] - 2)
    c.clear(31, 79)
    return c


def _post_px(c: Pix, x0: int) -> None:
    """A rail post: 4 px wide, 28 rows: ``wood`` 4 lit left column, ``wood`` 3, ``wood`` 2 right column, a ``wood`` 1
    bottom of 2 rows."""
    for y in range(28):
        for k, v in enumerate((4, 3, 3, 2)):
            c.put(x0 + k, y, "wood", 1 if y >= 26 else v)


def _rail(c: Pix, x0: int, x1: int, top: int, sag=lambda x: 0, burnt=lambda x: False) -> None:
    """A 3 px rail from x0..x1: ``wood`` 4 top row, ``wood`` 3, ``wood`` 1 bottom; burnt columns ``wood`` 1/0/0."""
    for x in range(x0, x1 + 1):
        y = top + sag(x)
        if burnt(x):
            c.put(x, y, "wood", 1)
            c.put(x, y + 1, "wood", 0)
            c.put(x, y + 2, "wood", 0)
        else:
            c.put(x, y, "wood", 4)
            c.put(x, y + 1, "wood", 3)
            c.put(x, y + 2, "wood", 1)


def _rail_lashing(c: Pix, x: int, y: int) -> None:
    """Rope lashing at a rail crossing, 2 x 3 px, ``stone`` 6/4."""
    for dy in range(3):
        for dx in range(2):
            c.put(x + dx, y + dy, "stone", 6 if (dx + dy) % 2 == 0 else 4)


def _pen_rail(variant: str) -> Pix:
    c = Pix(64, 28)
    sag = lambda x: 1 if 20 <= x <= 44 else 0                    # the lower rail sags 1 px between columns 20 and 44
    if variant == "a":
        burnt = lambda x: x >= 43                               # the right third charred
    else:
        burnt = lambda x: x <= 20                               # the left third charred instead
    _post_px(c, 2)
    _post_px(c, 58)
    upper_end = 63 if variant == "a" else 39
    _rail(c, 0, upper_end, 6)
    _rail(c, 0, 63, 17, sag, burnt)
    if variant == "b":
        # the upper rail snapped at column 40: its right part hangs from the right post, the free end down to row 16
        for x in range(40, 64):
            drop = round(max(0, 58 - x) * 8 / 18)
            top = 6 + drop
            c.put(x, top, "wood", 4)
            c.put(x, top + 1, "wood", 3)
            c.put(x, top + 2, "wood", 1)
        c.put(40, 15, "stone", 7)                               # the pale break face
    else:
        c.clear(50, 17)                                         # the 1 px gap in the charred rail
        c.clear(50, 18)
        c.clear(50, 19)
    # rope lashings at the four crossings (below the top row of the upper rail: rows 0..6 stay wood 4 at most)
    for x in (2, 58):
        _rail_lashing(c, x + 1, 7)
        _rail_lashing(c, x + 1, 17)
    return c


def heat_pen_rail_a() -> Pix:
    return _pen_rail("a")


def heat_pen_rail_b() -> Pix:
    return _pen_rail("b")


def heat_timber_debris_a() -> Pix:
    """Two charred plank offcuts and a splinter lying on the back line, 32 x 8: planks 3 px tall (``wood`` 3 top
    row, ``wood`` 2 body, ``wood`` 0 burnt ends of 2..4 px), one lying on the other at a 1 px step, a 1 px
    ``stone`` 7 fresh-break pixel on one end, no dark run longer than 4 px."""
    c = Pix(32, 8)

    def plank(x0, x1, top, burnt_left=0, burnt_right=0):
        for x in range(x0, x1 + 1):
            c.put(x, top, "wood", 3)
            c.put(x, top + 1, "wood", 2)
            c.put(x, top + 2, "wood", 2)
        for k in range(burnt_left):
            for dy in range(3):
                c.put(x0 + k, top + dy, "wood", 0)
        for k in range(burnt_right):
            for dy in range(3):
                c.put(x1 - k, top + dy, "wood", 0)

    plank(1, 19, 5, burnt_right=4)               # the lower plank, on the back line (rows 5..7)
    plank(11, 29, 2, burnt_right=3)              # the upper one rests on it, its right end on the lower's far side
    for x in range(20, 30):                      # ...and carries on down to the ground at a 1 px step
        c.put(x, 5 if x > 24 else 4, "wood", 3 if x > 24 else 2)
    for x in range(26, 30):
        c.put(x, 5, "wood", 0)
        c.put(x, 4, "wood", 2)
    c.put(1, 6, "stone", 7)                      # the fresh break
    for x in (30, 31):                           # a splinter on the ground
        c.put(x, 7, "wood", 3)
    c.put(24, 7, "wood", 2)
    c.put(25, 7, "wood", 3)
    return c


KEYS = {
    "heat_partition": [("heat_partition_intact", heat_partition_intact),
                       ("heat_partition_weakened", heat_partition_weakened),
                       ("heat_partition_broken", heat_partition_broken)],
    "heat_scorch": [("heat_scorch_wall_a", heat_scorch_wall_a), ("heat_scorch_floor_a", heat_scorch_floor_a)],
}


def _pack_row(imgs: list[Pix], pad: int = 0) -> Pix:
    w = sum(i.w for i in imgs) + pad * (len(imgs) - 1)
    h = max(i.h for i in imgs)
    out = Pix(w, h)
    x0 = 0
    for img in imgs:
        for y in range(img.h):
            for x in range(img.w):
                out.px[h - img.h + y][x0 + x] = img.px[y][x]
        x0 += img.w + pad
    return out


# =============================================================================================
# Atlases, manifests and the contract (the Broken Ring precedent, scripts/author_rings01.py)
# =============================================================================================
ATLASES = {
    "floor01_heat_partition": {
        "stem": "heat_partition", "width": 120,
        "pieces": [("heat_partition_intact", heat_partition_intact), ("heat_partition_weakened", heat_partition_weakened),
                   ("heat_partition_broken", heat_partition_broken)],
    },
    "floor01_heat_wall": {
        "stem": "heat_wall", "width": 160,
        "pieces": [("heat_scorch_wall_a", heat_scorch_wall_a), ("heat_scorch_wall_b", heat_scorch_wall_b),
                   ("heat_scorch_wall_c", heat_scorch_wall_c), ("heat_trace_wall_a", heat_trace_wall_a)],
    },
    "floor01_heat_structure": {
        "stem": "heat_structure", "width": 160,
        "pieces": [("heat_scorch_floor_a", heat_scorch_floor_a), ("heat_scorch_floor_b", heat_scorch_floor_b),
                   ("heat_ash_floor_a", heat_ash_floor_a), ("heat_ash_floor_b", heat_ash_floor_b),
                   ("heat_trace_spit_a", heat_trace_spit_a), ("heat_trace_spit_b", heat_trace_spit_b),
                   ("heat_shelf_mid_a", heat_shelf_mid_a), ("heat_shelf_mid_b", heat_shelf_mid_b),
                   ("heat_shelf_end_l", heat_shelf_end_l), ("heat_shelf_end_r", heat_shelf_end_r),
                   ("heat_bumper", heat_bumper)],
    },
    "floor01_heat_props": {
        "stem": "heat_props", "width": 128,
        "pieces": [("heat_pen_rail_a", heat_pen_rail_a), ("heat_pen_rail_b", heat_pen_rail_b),
                   ("heat_timber_debris_a", heat_timber_debris_a)],
    },
}
KEY_IDS = ("heat_partition_intact", "heat_partition_weakened", "heat_partition_broken", "heat_scorch_wall_a",
           "heat_scorch_floor_a")
PARTITION_IDS = ("heat_partition_intact", "heat_partition_weakened", "heat_partition_broken")


def pack(pieces: list[tuple[str, Pix]], width: int) -> tuple[Pix, dict]:
    """Shelf packing: left to right, a new shelf when the row is full. A piece's rect holds exactly its pixels."""
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


def build_pieces(name: str) -> list[tuple[str, Pix]]:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def build(name: str):
    pieces = build_pieces(name)
    sheet_img, rects = pack(pieces, ATLASES[name]["width"])
    return kit.grid_from([sheet_img], layer_names=["main"], durations=[100], pivot=(0, 0)), rects, dict(pieces)


CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_heat_kit.json"
MANIFESTS = ROOT / "assets/manifests"
EXTENDS = "assets/source/terrain/floor01/floor01_opening_kit.json"
WALL_PIECES = ("heat_scorch_wall_a", "heat_scorch_wall_b", "heat_scorch_wall_c", "heat_trace_wall_a")
STRUCTURE_PIECES = tuple(pid for pid, _ in ATLASES["floor01_heat_structure"]["pieces"])
ROLES = {pid: ("backwall", "backwall_decal") for pid in WALL_PIECES}
ROLES.update({pid: ("structure", "structure") for pid in STRUCTURE_PIECES})
FB, W_LINE = 560, 580
OWNER = ("AI-authored environment pixel art (AREA-HEAT-01: design spec and keys Claude Opus; gate verdicts and "
         "K2/K3/R1..R8 edits Claude Opus; execution Claude Sonnet; independent review pending)")
STATES = {"heat_partition_intact": "intact", "heat_partition_weakened": "weakened", "heat_partition_broken": "broken"}

# Spec section 5 placements (room layout px, top-left). The partition is NOT placed (HEAT-AD1).
# Wall decals: top-left on the world bond (x % 32 == 0, y % 16 == 0) and clear of every torch pool's visible
# re-shade (HEAT-AD12); the measured spans are in docs/evidence/area-heat-01/measurements.txt.
# `heat_scorch_wall_c` and `heat_trace_wall_a` are drawn and exported but NOT placed: the pooled-pixel
# measurement (docs/evidence/area-heat-01/measurements.txt, "FREE_SPANS") finds no 32 px of unpooled brick
# between the bays outside the pen entry's span (x 1112..1143, taken by wall_a), and the spec's trace spot
# (392, 544) is off the bond, inside two torch pools and on the base course (AD12: not placed, recorded).
WALL_PLACES = [("heat_scorch_wall_a", 1088, 464), ("heat_scorch_wall_b", 1920, 480)]
FLOOR_PLACES = (
    [("heat_scorch_floor_a", x, 561) for x in (1044, 1456)]
    + [("heat_scorch_floor_b", x, 562) for x in (560, 1264, 1680)]
    + [("heat_ash_floor_a", 288, 563)]
    + [("heat_ash_floor_b", x, 564) for x in (880, 1792)]
    + [("heat_trace_spit_a", x, 564) for x in (968, 1600)]
    + [("heat_trace_spit_b", 1336, 565)]
)
SHELF_SEQUENCE = "abaab"                          # entry shelf mids at x 1128..1192 step 16
DOCK_B = {1, 4, 9, 12, 17, 22}                    # dock mids at x 1888..2288 step 16 (26 tiles)


def step_places() -> list[tuple]:
    out = [("structure", "heat_shelf_end_l", 1120, 548)]
    out += [("structure", f"heat_shelf_mid_{v}", 1128 + 16 * i, 548) for i, v in enumerate(SHELF_SEQUENCE)]
    out += [("structure", "heat_shelf_end_r", 1208, 548)]
    out += [("structure", "heat_shelf_mid_" + ("b" if i in DOCK_B else "a"), 1888 + 16 * i, 548) for i in range(26)]
    out += [("structure", "heat_bumper", 1856, 500)]
    return out


PROP_PLACES = ([("prop", "heat_pen_rail_a", x, 532) for x in (1240, 1720)] + [("prop", "heat_pen_rail_b", 1560, 532)]
               + [("prop", "heat_timber_debris_a", x, 552) for x in (1316, 1648)])


def _at(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def dressing() -> dict:
    place = ([_at("backwall_decal", *w) for w in WALL_PLACES] + [_at("structure", *f) for f in FLOOR_PLACES]
             + [_at(*t) for t in step_places()] + [_at(*t) for t in PROP_PLACES])
    return {"corral": {"remove": [], "place": place, "parallax": {}}}


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = ROLES.get(pid, ("prop", "prop"))
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0], "anchor_name": "top_left (room placement, spec section 5)", "grid_snap": 1,
             "grammar": ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    if pid in STATES:
        entry["state_of"] = {"group": "heat_partition", "state": STATES[pid]}
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
        "kit_id": "floor01_heat",
        "extends": EXTENDS,
        "status": "proposed - AREA-HEAT-01 kit (docs/02-art/area-heat-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema "
                          "from the ENV-01 contract through the opening kit. Adds four decor atlases and the "
                          "scorched-corral pieces for room 0 (the Infernal Corral); no tiles, nothing collidable, "
                          "no new palette colour and no new light emitter.",
        "palette_use": palette_use,
        "atlases": atlases,
        "tiles": [],
        "pieces": pieces,
        "state_groups": {"heat_partition": {
            "states": ["intact", "weakened", "broken"],
            "note": "the goblin timber partition (spec section 6), three 40 x 48 frames. The dressing does NOT place "
                    "it (HEAT-AD1): CORRAL-D's presenter draws the frame of the prop's state at (1380, 532) "
                    "(presentation only; weakened after the first counted hit, broken on the third hit or a rush "
                    "contact), because drawn geometry must equal the 16 x 48 solid it will carry"}},
        "room_dressing": dressing(),
        "usage_rules": {
            "R1_corral": "room 0 `corral` only: wall scorches are backwall_decal on the world bond (x % 32 == 0, "
                         "y % 16 == 0) and outside every torch pool; floor decals lie in the deck's back half "
                         "(y 561..572) and never on a step; the step fronts lie exactly on the step rects (their "
                         "top row, stone 8, is the walkable arris); props stand on the floor back line (y 560); "
                         "nothing at x 1372..1428 (the partition, CORRAL-D). The partition frames are drawn by "
                         "the presenter, never placed.",
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
        "kit_contract": "assets/source/terrain/floor01/floor01_heat_kit.json",
        "owner": OWNER,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 scorched-corral pieces for Floor 1 room 0 (AREA-HEAT-01 on the ENV-01 "
                      "kit); rule-based shapes and ramps, nothing cropped, traced or resampled from concept art; no "
                      "text, logos or likenesses",
            "generator": "Deterministic authoring script scripts/author_heat01.py written by AI agents (keys, spec "
                         "and gate edits designed by Claude Opus; execution by Claude Sonnet); no image-model raster "
                         "output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1/03-bad-llamas-scorched-corridor (mood reference only; no pixel cropped, "
                "traced or resampled)",
            ],
        },
    }


def write() -> dict:
    out = {}
    for name, spec in ATLASES.items():
        grid, rects, _ = build(name)
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")
        out[name] = {"rects": rects, "canvas": grid["canvas"], "colours": len(grid["palette"])}
        (MANIFESTS / f"{name}.json").write_text(json.dumps(manifest(name, grid["canvas"]), indent=1) + "\n",
                                                encoding="utf-8")
    CONTRACT.write_text(json.dumps(contract(out), indent=1) + "\n", encoding="utf-8")
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


# The gate's scene still: the corral pilot's window frame at tick 510 (scripts/room_capture.sh corral corral
# 420:30:7), copied to docs/evidence/area-heat-01/gate/scene_pen_before_1x.png. Its drawn view is x 1038, y 289:
# x from the entry shelf's edges (x 1120..1215 at screen 82..177), y from the wall itself (the base course's top
# mortar row, world y 544, is screen row 255; the wall's mortar rows sit at screen y % 8 == 7). The first gate
# used y 292, which put every wall decal 3 px off the world bond.
VIEW = (1038, 289)
FRONT = (  # what draws in front of a backwall decal or a structure-layer floor decal in that still
    ("shelf", (82, 256, 178, 268)),
    ("donut", (184, 206, 246, 252)),
    ("carl", (254, 238, 292, 292)),
    ("llama", (414, 224, 488, 292)),
)
PARTITION_AT = (1380, 532)
SCORCH_WALL_AT = (1088, 464)     # plume bricks from world x 1104: clear of the torch pools at 1016 and 1240 (C4)
SCORCH_FLOOR_AT = (1044, 561)
GOBLIN_AT = (40, 288)              # staged: screen x of the goblin's feet centre (M0 goblins do not reach the pen)
CARL_AT = (72, 288)                # staged for the wall-scorch contrast crop: Carl at the pen entry, beside the plume


def _goblin():
    """Frame 0 of the exported goblin atlas, trimmed to its opaque box (feet on the bottom row)."""
    from PIL import Image
    sheet = Image.open(ROOT / "generated/art/goblin_a_h36.png").convert("RGBA")
    meta = json.loads((ROOT / "generated/art/goblin_a_h36.json").read_text(encoding="utf-8"))
    f = meta["frames"][0]["frame"]
    cell = sheet.crop((f["x"], f["y"], f["x"] + f["w"], f["y"] + f["h"]))
    return cell.crop(cell.getbbox())


def _sprite(name: str):
    """Frame 0 of an exported atlas, trimmed to its opaque box (feet on the bottom row)."""
    from PIL import Image
    sheet = Image.open(ROOT / f"generated/art/{name}.png").convert("RGBA")
    meta = json.loads((ROOT / f"generated/art/{name}.json").read_text(encoding="utf-8"))
    frames = meta["frames"]
    f = frames[0]["frame"] if isinstance(frames, list) else next(iter(frames.values()))["frame"]
    cell = sheet.crop((f["x"], f["y"], f["x"] + f["w"], f["y"] + f["h"]))
    return cell.crop(cell.getbbox())


def _lit_wall(img: Pix, still, at: tuple) -> "object":
    """The wall decal under the still's light: each brick (or base-course block) of the decal is scaled per
    channel by the light the still shows on that same brick (mean face colour over the band's base colour).
    An approximation of the runtime's lit backwall for the gate sheets, not a capture."""
    vx, vy = VIEW
    ox, oy = at[0] - vx, at[1] - vy

    def cell(x, y):
        if y >= 80:
            return ("base", (x + 16) // 32)
        r = y // 8
        return ("b", r, (x + 8 * (r % 2)) // 16)

    def face(x, y):
        if y >= 80:
            return y not in (80, 95) and (x + 16) % 32 != 0
        return y % 8 != 0 and (x + 8 * ((y // 8) % 2)) % 16 != 0

    def hexrgb(h):
        return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))

    acc = {}
    for y in range(img.h):
        for x in range(img.w):
            if face(x, y):
                a = acc.setdefault(cell(x, y), [0, 0, 0, 0])
                p = still.getpixel((ox + x, oy + y))
                for i in range(3):
                    a[i] += p[i]
                a[3] += 1
    mul = {}
    for k, a in acc.items():
        ref = hexrgb(RAMPS["stone"][6] if k[0] == "base" else RAMPS["brick"][round(kit.BANDS[SW_ROW_BAND[k[1]]][1])])
        mul[k] = [a[i] / a[3] / max(1, ref[i]) for i in range(3)]
    im = to_image(img)
    px = im.load()
    for y in range(img.h):
        for x in range(img.w):
            r, g, b, al = px[x, y]
            if al:
                m = mul[cell(x, y)]
                px[x, y] = (min(255, int(r * m[0])), min(255, int(g * m[1])), min(255, int(b * m[2])), 255)
    return im


def gate(out_dir: Path) -> None:
    """Gate sheets: the keys at 1x and 4x, their silhouettes, the three partition states side by side, and
    the keys composed at 1x into the native-scale pen still with Carl, Donut, llama B and a staged goblin."""
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    keys = [fn() for pieces in KEYS.values() for _, fn in pieces]
    pad = 8
    for silhouette, name in ((False, "keys"), (True, "silhouettes")):
        imgs = [to_image(k, silhouette) for k in keys]
        w = sum(i.width for i in imgs) + pad * (len(imgs) + 1)
        h = max(i.height for i in imgs) + pad * 2
        sheet = Image.new("RGBA", (w, h), (205, 198, 186, 255) if silhouette else (40, 36, 48, 255))
        x = pad
        for i in imgs:
            sheet.alpha_composite(i, (x, h - pad - i.height))
            x += i.width + pad
        sheet.save(out_dir / f"{name}_1x.png")
        sheet.resize((w * 4, h * 4), Image.NEAREST).save(out_dir / f"{name}_4x.png")

    still = Image.open(out_dir / "scene_pen_before_1x.png").convert("RGBA")
    vx, vy = VIEW
    gob = _goblin()
    wall, floor = _lit_wall(heat_scorch_wall_a(), still.convert("RGB"), SCORCH_WALL_AT), to_image(heat_scorch_floor_a())
    states = (("intact", heat_partition_intact), ("weakened", heat_partition_weakened),
              ("broken", heat_partition_broken))
    strip = []
    for name, fn in states:
        comp = still.copy()
        comp.alpha_composite(wall, (SCORCH_WALL_AT[0] - vx, SCORCH_WALL_AT[1] - vy))
        comp.paste(still.crop(FRONT[0][1]), FRONT[0][1][:2])            # the shelf draws over a wall decal
        comp.alpha_composite(floor, (SCORCH_FLOOR_AT[0] - vx, SCORCH_FLOOR_AT[1] - vy))
        comp.alpha_composite(to_image(fn()), (PARTITION_AT[0] - vx, PARTITION_AT[1] - vy))
        for _, box in FRONT[1:]:                                       # actors draw over props and decals
            comp.paste(still.crop(box), box[:2])
        comp.alpha_composite(gob, (GOBLIN_AT[0] - gob.width // 2, GOBLIN_AT[1] - gob.height))
        comp.save(out_dir / f"scene_pen_{name}_1x.png")
        comp.resize((1280, 720), Image.NEAREST).save(out_dir / f"scene_pen_{name}_2x.png")
        strip.append(comp.crop((300, 200, 440, 300)))
    # the three states at gameplay scale, cropped around the partition, then enlarged 4x
    s = Image.new("RGBA", (len(strip) * 148 + 8, 108), (40, 36, 48, 255))
    for k, im in enumerate(strip):
        s.alpha_composite(im, (8 + k * 148, 4))
    s.save(out_dir / "states_in_scene_1x.png")
    s.resize((s.width * 4, s.height * 4), Image.NEAREST).save(out_dir / "states_in_scene_4x.png")
    # the wall scorch before and after at gameplay scale: without actors (the "not a door" check), with the
    # staged goblin at its gate position, the goblin over the plume, and Carl at the pen entry; before | after
    # per row, 1x and 3x
    carl = _sprite("carl_book1_h48")
    box = (10, 150, 150, 290)
    rows = []
    for actors in ((), ((gob, GOBLIN_AT),), ((gob, CARL_AT),), ((carl, CARL_AT),)):
        pair = []
        for with_scorch in (False, True):
            comp = still.copy()
            if with_scorch:
                comp.alpha_composite(wall, (SCORCH_WALL_AT[0] - vx, SCORCH_WALL_AT[1] - vy))
                comp.paste(still.crop(FRONT[0][1]), FRONT[0][1][:2])
            for spr, (fx, fy) in actors:
                comp.alpha_composite(spr, (fx - spr.width // 2, fy - spr.height))
            pair.append(comp.crop(box))
        rows.append(pair)
    cw, ch = box[2] - box[0], box[3] - box[1]
    s = Image.new("RGBA", (cw * 2 + 12, ch * len(rows) + 4 * (len(rows) + 1)), (40, 36, 48, 255))
    for i, pair in enumerate(rows):
        for j, im in enumerate(pair):
            s.alpha_composite(im, (4 + j * (cw + 4), 4 + i * (ch + 4)))
    s.save(out_dir / "scorch_wall_before_after_1x.png")
    s.resize((s.width * 3, s.height * 3), Image.NEAREST).save(out_dir / "scorch_wall_before_after_3x.png")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "gate":
        gate(Path(sys.argv[2]))
    else:
        write()
