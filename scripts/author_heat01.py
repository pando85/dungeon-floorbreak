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

    python3 scripts/author_heat01.py              # grids + .ase of the keys
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
SW_PLUME = dict(
    foot_x=28.0, lean=0.3,         # core centre at the fire's foot; drift per px of height (positive: to the right)
    sig0=4.0, sig_grow=0.25,       # lateral spread of the soot at the foot, growth per px of height
    amp=4.2, amp_top=0.2,          # soot at the core just above the fire; fraction left at the top
    fade_top=40,                   # brick rows centred above this thin out brick by brick
    thr=1.4,                       # a brick joins the plume when its strongest sample reaches this soot
    base_s=1.0, base_rx=14.0, base_ry=18.0,   # stone re-shade dome on the base course (at most -1.0)
    x_min=16, base_x=(8, 32),      # plume bricks from local x 16; base-course re-shade over local x 8..31
)


def _plume_soot(x: float, y: float, p=SW_PLUME) -> float:
    """Soot at a pixel: a leaning, widening plume, darkest just above the fire. Low frequency only (no strands)."""
    d = (79.5 - y) / 79.5
    sig = p["sig0"] + p["sig_grow"] * (79.5 - y)
    a = p["amp"] * (1.0 - (1.0 - p["amp_top"]) * d ** 1.1)
    u = (x - (p["foot_x"] + p["lean"] * (79.5 - y))) / sig
    taper = clamp((x - p["x_min"]) / 8.0, 0.0, 1.0) * clamp((SW_W - x) / 8.0, 0.0, 1.0)
    return a * math.exp(-u * u) * taper * (0.85 + 0.3 * vnoise(x, y, 20, "plume_mod"))


def _plume_brick(r: int, col: int, p=SW_PLUME) -> bool:
    """A brick is in the plume or out (whole-brick rule); the top rows thin out brick by brick."""
    x0 = col * 16 - 8 * (r % 2)
    if x0 < p["x_min"] or x0 + 16 > SW_W:
        return False
    peak = max(_plume_soot(x + 0.5, y + 0.5, p) for y in range(r * 8, r * 8 + 8, 2) for x in range(x0, x0 + 16, 2))
    if peak < p["thr"] + (rnd("plume_edge", r, col) - 0.5) * 0.3:
        return False
    by = r * 8 + 4.0
    if by < p["fade_top"]:
        return rnd("plume_thin", r, col) < (by / p["fade_top"]) ** 0.7 + 0.15
    return True


def heat_scorch_wall_a() -> Pix:
    import lookdev_floor01_author as ld  # noqa: PLC0415  (the kit's shading core: bevel normals)
    p = SW_PLUME
    c = kit.canvas(SW_W, SW_H)
    members = {}
    for r in range(9, -1, -1):          # bottom-up: a brick joins only if it rests on a plume brick (no floaters)
        for col in range(6):
            x0 = col * 16 - 8 * (r % 2)
            below = r == 9 or any(members.get((r + 1, k)) and abs(k * 16 - 8 * ((r + 1) % 2) - x0) < 16
                                  for k in range(6))
            members[(r, col)] = below and _plume_brick(r, col)
    for r in range(10):
        _rows, base, mortar = kit.BANDS[SW_ROW_BAND[r]]
        for col in range(6):
            if not members[(r, col)]:
                continue
            x0 = col * 16 - 8 * (r % 2)
            for py in range(r * 8, r * 8 + 8):
                for px in range(x0, x0 + 16):
                    kit.brick_pixel(c, px, py, px - x0, py - r * 8, base=base, mortar_base=mortar, jit=0.0,
                                    seed=("heat_plume", r, col), chips=False,
                                    soot=max(SW_SOOT_FLOOR, _plume_soot(px + 0.5, py + 0.5)))
                    q = c.px[py][px]
                    lo = 3 if py >= SW_HEAD_ROW else 2
                    if q and q[0] == "brick" and q[1] < lo:
                        c.px[py][px] = ("brick", lo)
    # the base course (rows 80..95) stays pale stone: a low dome of re-shade at the fire's foot, at most -1.0
    fx = p["foot_x"]
    for py in range(80, 96):
        for px in range(*p["base_x"]):
            u = ((px + 0.5 - fx) / p["base_rx"]) ** 2 + ((95.5 - py) / p["base_ry"]) ** 2
            s = p["base_s"] * (1.0 - u)
            if s < 0.3:
                continue
            du, dv = (px + 16) % 32, py - 80
            if du == 0 or dv == 0:
                kit.put(c, px, py, "mortar", (2.1 if dv else 1.4) - 0.5 * s)
            elif dv == 15:
                kit.put(c, px, py, "stone", 2.0)
            else:
                nrm, _e = ld.bevel_normal(du - 1, dv - 1, 31, 14, 1)
                kit.put(c, px, py, "stone", 6.8 + 1.6 * kit.key_term(nrm) - 0.12 * dv - s)
    # heat crazing: two 1 px hairlines licking up from the fire on the stone (diagonal, never a horizontal pair)
    fx = int(fx)
    for path in (((-2, 91), (-3, 90), (-3, 89), (-4, 88), (-5, 87)), ((3, 90), (3, 89), (4, 88), (4, 87))):
        for dx, y in path:
            q = c.px[y][fx + dx]
            if q is not None and q[0] == "stone":
                kit.put(c, fx + dx, y, "mortar", 1)
    for dx, y, v in ((-3, 93, 0), (0, 94, 1), (2, 92, 0), (5, 94, 0)):   # embers: flame 0..1, single pixels
        kit.put(c, fx + dx, y, "flame", v)
    out = Pix(SW_W, SW_H)
    out.px = [row[:] for row in c.px]
    return out


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


def write() -> None:
    for stem, pieces in KEYS.items():
        imgs = [fn() for _, fn in pieces]
        if stem == "heat_partition":                            # one canvas, three frames (the state swap)
            grid = kit.grid_from(imgs, layer_names=["main"], durations=[100] * len(imgs), pivot=(0, 0))
        else:                                                   # two pieces side by side, bottom-aligned
            grid = kit.grid_from([_pack_row(imgs, pad=8)], layer_names=["main"], durations=[100], pivot=(0, 0))
        path = SRC / f"{stem}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{stem}.ase")], check=True)
        print(f"{stem}: {[pid for pid, _ in pieces]} {grid['canvas']} {len(grid['palette'])} colours")


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
