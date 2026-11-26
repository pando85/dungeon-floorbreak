#!/usr/bin/env python3
"""Authoring record for AREA-RINGS-01: the Broken Ring kit of Floor 01 rooms 4a and 4b.

Draws the frozen design keys of ``docs/02-art/area-rings-01-spec.md`` on ``floor01-catacomb-v2``
(no new colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/ring_wall.grid.json``     ``ring_arc_wall_a`` (hero key, spec section 11)
- ``assets/source/props/floor01/ring_shutter.grid.json``  ``ring_shutter_closed`` (spec section 6)
- ``assets/source/props/floor01/ring_camp.grid.json``     ``ring_camp_a`` (spec section 5)

The keys are frozen: execution adds the remaining pieces of spec section 5 as new functions and new
atlases, and must not redraw a key (a key change is a design change). Every pixel comes from rules
(polar bands, ramps, a hash noise); nothing is traced or resampled from a concept image. No text,
logos or likenesses.

    python3 scripts/author_rings01.py              # grids + .ase of the keys
    python3 scripts/author_rings01.py gate DIR      # silhouette gate sheets (1x and 4x)
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
    h = hashlib.sha256("|".join(map(str, ("rings01",) + key)).encode()).digest()
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
        """Put a ramp value rounded to the nearest step with a sparse stone grain (no screen-wide dither)."""
        n = rnd("grain", seed, x, y)
        self.put(x, y, ramp, round(f) + (1 if n > 0.93 else (-1 if n < 0.07 else 0)))

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


# =============================================================================================
# The ring arc generator. Hero key ring_arc_wall_a (spec section 11): 408 x 164, backwall_decal. The upper
# arc of one of the great ring's carved courses; its centre sits 48 px below the piece bottom, behind the
# floor, so the arc dives into the floor at both ends and never closes into an arch opening. Execution
# parameterises the same generator for ring_arc_wall_b, the full ring and the upper arcs (spec section 5);
# the key is the parameter set ``ARC_A``.
# =============================================================================================
LIGHT = (-0.55, -0.83)     # key light from the upper left (unit vector, y down)


SOFT = 8                    # dithered falloff width of a local darkening, px


class Arc:
    """One ring: geometry (angles from the apex, + to the right), bands and weathering seeds."""

    def __init__(self, **kw):
        self.seed = "a"
        self.keystone = True
        # (r_in, r_out) of the three bands and the two grooves, outermost first
        self.outer = (184.0, 208.0)     # voussoir band, radial joints every 7.5 degrees (keystone wider)
        self.groove_a = (181.0, 184.0)
        self.notch = (167.0, 181.0)     # plain band carved with radial notches every 5 degrees
        self.groove_b = (164.0, 167.0)
        self.inner = (148.0, 164.0)     # voussoir band, joints every 9 degrees, offset half a block
        self.shadow = (143.0, 148.0)    # cast shadow of the relief on the brick below the inner band
        # the off-axis break: outer, notch and inner bands lose different spans so the broken end steps like
        # fractured masonry, never a clean cut
        self.brk = {"outer": (31.0, 51.0), "notch": (33.5, 48.0), "inner": (35.0, 46.5)}
        self.chains = ((-21.0, 5, 1), (17.5, 3, -1))     # (angle, links, torn-link side) hanging from the inner band
        self.cracks = ((-58.0, 206.0, 13, 0.9), (-12.0, 190.0, 9, -0.6), (22.0, 160.0, 8, 0.7),
                       (-40.0, 176.0, 7, 0.4), (61.0, 196.0, 11, -0.8), (-71.0, 158.0, 8, 0.5))
        self.chip_range = (-760, 760)
        self.lichen_rows = 34
        self.lift = (-0.9, 0.3)         # stone value lift at the top and at the foot (soffit shadow, torch light)
        self.base = (5.6, 4.7, 5.3)     # outer, notch, inner band base values (before the K1 shift)
        self.shift = 1                  # K1: every stone index drops by this much
        self.crown_deg = 30.0           # K1: |a| below this takes one more step on the outer and notch bands
        self.crown_extra = 1
        self.cap = 6                    # K1 target: stone 7 and 8 absent (13 grain pixels of the key land on 7)
        self.fallen = ()
        # gate C4: boxes (x0, y0, x1, y1, drop) where the stone drops further, behind an actor's head line
        self.local_dark = ()
        # spec section 8 / RNG-AD22: piece-local centres of the torch pools the piece overlaps. Stone within
        # TORCH_R[0] px of a centre rises two steps, within TORCH_R[1] px one step (dithered edge), capped at
        # stone 5 and never lowered, so the legs pick up the pool the brick around them shows.
        self.torch_lit = ()
        # piece-local x spans (inclusive) kept free of lichen: no green within 16 px of a goblin's x (gate K4)
        self.lichen_clear = ()
        self.__dict__.update(kw)


# Torch pool cores (layout px): the composer's pool decal top-left plus (122, 128), the centre of the pool's
# warm core (all four pool variants put it within 6 px of this offset).
POOL_CORE = (122, 128)
TORCH_R = (48.0, 100.0)


def pools_local(place: tuple, pools: tuple) -> tuple:
    """Piece-local pool centres of the pool decals ``pools`` (top-left, layout px) for a piece at ``place``."""
    return tuple((px + POOL_CORE[0] - place[0], py + POOL_CORE[1] - place[1]) for px, py in pools)


ARC_A_AT, ARC_B_AT, ARC_LM_AT = (1012, 400), (324, 400), (1680, 356)
ARC_W, ARC_H = 408, 164
# 4a: the bay torch at 976 (pool 872/364) and the free torch raised over the broken right leg at (1360, 368)
# (pool 1256/284, RNG-AD24)
ARC_A = Arc(w=ARC_W, h=ARC_H, cx=204.0, cy=212.0, torch_lit=pools_local(ARC_A_AT, ((872, 364), (1256, 284))))
# ring_arc_wall_b: the same geometry with its own break (left), seeds, cracks and one chain stub
ARC_B = Arc(w=ARC_W, h=ARC_H, cx=204.0, cy=212.0, seed="b",
            brk={"outer": (-51.0, -31.0), "notch": (-48.0, -33.5), "inner": (-46.5, -35.0)},
            chains=((20.0, 4, -1),),
            cracks=((52.0, 204.0, 12, -0.8), (14.0, 188.0, 10, 0.6), (-24.0, 158.0, 9, -0.7),
                    (38.0, 174.0, 8, 0.5), (-62.0, 198.0, 11, 0.9), (68.0, 160.0, 7, -0.5)),
            lichen_rows=30, torch_lit=pools_local(ARC_B_AT, ((168, 364),)))
# ring_relief_landmark: the one full ring, 208 x 208. The outer circumference is entirely inside rows
# 0..203 (spec gate C2a: the deck back row hides y >= 560 and the soffit hides y < 352 of a piece placed at
# y 356), so r_out is 102. Same band order, pitches and bevels as the key, interior transparent (C2b).
LM_CX, LM_CY = 104.0, 102.0
ARC_LM = Arc(w=208, h=208, cx=LM_CX, cy=LM_CY, seed="lm", keystone=False,
             outer=(84.0, 102.0), groove_a=(82.0, 84.0), notch=(70.0, 82.0), groove_b=(68.0, 70.0),
             inner=(54.0, 68.0), shadow=(49.0, 54.0),
             brk={"outer": (40.0, 68.0), "notch": (42.5, 65.0), "inner": (44.0, 63.5)},
             chains=(), cracks=((-58.0, 100.0, 9, 0.9), (-12.0, 90.0, 7, -0.6), (22.0, 76.0, 6, 0.7),
                                (-120.0, 92.0, 8, 0.5), (130.0, 98.0, 8, -0.8), (170.0, 74.0, 6, 0.4)),
            chip_range=(-1790, 1790), lichen_rows=26, lift=(-0.9, 0.2), crown_deg=0.0,
             # gate C4 (behind the goblin at x 1824, piece-local x about 144): recomputed by execution per band
             # after the move to 1680 (RNG-AD20); empty until then
             local_dark=((112, 160, 180, 204, (3, 3, 2)),),
             # 4a: the bay-1680 torch moved to 1656 (pool 1552/364, RNG-AD24) and the bay torch at 1936 (pool 1832/364)
             torch_lit=pools_local(ARC_LM_AT, ((1552, 364), (1832, 364))),
             # the goblin at x 1824 stands at piece-local x 144 since the move to 1680 (RNG-AD20)
             lichen_clear=((127, 161),))


def R(g: Arc, *key) -> float:
    """The key's own noise stays as authored (seed ``a``); other rings take their seed."""
    return rnd(*key) if g.seed == "a" else rnd(g.seed, *key)


def _band_value(g, r, a_deg, band, joint_step, joint_phase, base):
    """Stone value of one voussoir-band pixel: flat face, bevels toward both radii, joint shading."""
    r_in, r_out = band
    t = (r - r_in) / (r_out - r_in)                          # 0 inner edge .. 1 outer edge
    a = math.radians(a_deg)
    out_n = (math.sin(a), -math.cos(a))                      # outward normal (y down)
    lit_out = out_n[0] * LIGHT[0] + out_n[1] * LIGHT[1]      # > 0 when the outer edge faces the light
    v = base
    if r_out - r < 2.0:                                      # outer bevel
        v += 1.6 * lit_out - 0.3
    elif r - r_in < 2.0:                                     # inner bevel faces the other way
        v -= 1.6 * lit_out + 0.4
    # voussoir: slight tone per block, a soft pillow toward the block centre
    k = (a_deg - joint_phase) / joint_step
    block = math.floor(k)
    fk = k - block
    v += (R(g, band, block) - 0.5) * 0.9
    v += 0.35 * (1.0 - abs(fk - 0.5) * 2.0) - 0.15
    v -= 0.35 * abs(t - 0.55)
    return v, block, fk


def draw_arc(c: Pix, g: Arc) -> Pix:
    """Draw one ring on ``c`` (the canvas is not cleared; pixels the ring does not cover stay)."""
    OUTER, GROOVE_A, NOTCH, GROOVE_B = g.outer, g.groove_a, g.notch, g.groove_b
    INNER, SHADOW = g.inner, g.shadow
    inner_lo = INNER[0] if INNER else NOTCH[0]
    r_min = (SHADOW[0] if SHADOW else inner_lo)
    ARC_H = g.h
    for y in range(g.h):
        for x in range(g.w):
            dx, dy = x + 0.5 - g.cx, y + 0.5 - g.cy
            r = math.hypot(dx, dy)
            if r < r_min or r >= OUTER[1] + 4:
                continue
            a_deg = math.degrees(math.atan2(dx, -dy))           # 0 at the apex, + to the right
            # height falloff: the top of the arc sits in the soffit's shadow, the lower legs in torch light
            lift = g.lift[0] * (1.0 - y / ARC_H) + g.lift[1] * (y / ARC_H)
            if OUTER[0] <= r < OUTER[1] + 4:
                lo, hi = g.brk["outer"]
                key = g.keystone and abs(a_deg) < 4.2             # the keystone stands 4 px proud
                if r >= OUTER[1] and not key:
                    continue
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(g, min(r, OUTER[1] - 0.01), a_deg, OUTER, 7.5, 3.75, g.base[0] + lift)
                if key:
                    v = 6.2 + lift + (0.9 if r >= OUTER[1] - 1 else 0) - (1.2 if abs(a_deg) > 3.6 else 0)
                elif fk < 0.5 / (math.radians(7.5) * r) * 1.2:   # 1 px radial mortar joint
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 8.4), g.seed + "o" if g.seed != "a" else "o")
            elif GROOVE_A[0] <= r < GROOVE_A[1] or GROOVE_B[0] <= r < GROOVE_B[1]:
                lo, hi = g.brk["notch"]
                if lo - 1 <= a_deg <= hi + 1:
                    continue
                inner_lip = (r - GROOVE_A[0] < 1.0) if r >= GROOVE_A[0] else (r - GROOVE_B[0] < 1.0)
                c.put(x, y, "mortar", 4 if inner_lip and a_deg < 0 else 1)
            elif NOTCH[0] <= r < NOTCH[1]:
                lo, hi = g.brk["notch"]
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(g, r, a_deg, NOTCH, 5.0, 0.0, g.base[1] + lift)
                # a notch: a 4 px deep recess cut into the outer two thirds of the band
                if 0.18 < fk < 0.52 and r > NOTCH[0] + 4:
                    left_wall = fk < 0.26
                    c.put(x, y, "mortar", 3 if left_wall and a_deg < 0 else 2)
                    continue
                c.gput(x, y, "stone", clamp(v - 0.3, 1.0, 7.6), g.seed + "n" if g.seed != "a" else "n")
            elif INNER and INNER[0] <= r < INNER[1]:
                lo, hi = g.brk["inner"]
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(g, r, a_deg, INNER, 9.0, 4.5, g.base[2] + lift)
                if fk < 0.5 / (math.radians(9.0) * r) * 1.2:
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 8.2), g.seed + "i" if g.seed != "a" else "i")
            elif SHADOW and SHADOW[0] <= r < SHADOW[1]:
                lo, hi = g.brk["inner"]
                if lo <= a_deg <= hi:
                    continue
                # the relief's shadow on the brick: solid near the band, checker toward the brick
                depth = SHADOW[1] - r
                if depth < 2.0 or (depth < 4.0 and (x + y) % 2 == 0) or (depth < 5.0 and (x + y) % 4 == 0):
                    c.put(x, y, "mortar", 1)
    _arc_break(c, g)
    _arc_weather(c, g)
    _arc_chains(c, g)
    _arc_value_shift(c, g)
    return c


def _polar(g, a_deg, r):
    a = math.radians(a_deg)
    return g.cx + r * math.sin(a), g.cy - r * math.cos(a)


def _arc_break(c: Pix, g: Arc) -> None:
    """The missing segment: raw fracture faces on the surviving block ends and the scarred bed behind."""
    bands = [("outer", g.outer), ("notch", g.notch)] + ([("inner", g.inner)] if g.inner else [])
    for name, band in bands:
        lo, hi = g.brk[name]
        for r10 in range(int(band[0] * 10), int(band[1] * 10)):
            r = r10 / 10.0
            # jagged fracture: the edge wanders by up to 2 degrees with radius
            jag = (R(g, "jag", name, int(r / 3)) - 0.5) * 3.2
            for edge, side in ((lo + jag, -1), (hi + jag * 0.6, +1)):
                for k in range(3):
                    x, y = _polar(g, edge + side * (k * 0.5 / max(r, 1) * 57.3), r)
                    ix, iy = int(x), int(y)
                    if c.get(ix, iy) is None:
                        continue
                    # fresh fracture: pale on the face lit by the key light (the left-hand end), dark on the other
                    c.put(ix, iy, "stone", (7 - k) if side < 0 else (3 - k))
    # the bed the voussoirs were set in: a narrow rough rim of old mortar just past each broken end
    # (2..4 px, ragged); the rest of the gap is transparent so the brick behind shows through
    r_lo = g.inner[0] if g.inner else g.notch[0]
    for y in range(c.h):
        for x in range(c.w):
            if c.get(x, y) is not None:
                continue
            dx, dy = x + 0.5 - g.cx, y + 0.5 - g.cy
            r = math.hypot(dx, dy)
            if not (r_lo <= r < g.outer[1]):
                continue
            a_deg = math.degrees(math.atan2(dx, -dy))
            px_from = min(abs(a_deg - lo) for lo in (g.brk["outer"][0], g.brk["outer"][1])) * r / 57.3
            if g.brk["outer"][0] < a_deg < g.brk["outer"][1] and px_from < 2.0 + 2.0 * R(g, "rim", int(r / 2)):
                c.put(x, y, "mortar", 2 if R(g, "rim2", x, y) > 0.3 else 3)


def _arc_weather(c: Pix, g: Arc) -> None:
    """Hairline cracks, chipped arrises and a little damp lichen on the lower legs (dry floor, old damp)."""
    for i, (a0, r0, length, turn) in enumerate(g.cracks):
        a, r = a0, r0
        for s in range(length):
            x, y = _polar(g, a, r)
            if c.get(int(x), int(y)) and c.get(int(x), int(y))[0] == "stone":
                c.put(int(x), int(y), "mortar", 2)
            r -= 1.0
            a += turn * (0.5 if R(g, "ck", i, s) > 0.5 else 0.15)
    # chipped arrises: a few outer-edge pixels lost along the band (transparent: the brick shows)
    for a10 in range(g.chip_range[0], g.chip_range[1], 7):
        a = a10 / 10.0
        if R(g, "chip", a10) > 0.86:
            x, y = _polar(g, a, g.outer[1] - 0.5)
            if 0 <= int(y) < c.h and 0 <= int(x) < c.w:
                c.px[int(y)][int(x)] = None
    # lichen: only near the floor, on the lower legs, below the torch line
    for y in range(c.h - g.lichen_rows, c.h):
        for x in range(c.w):
            here = c.get(x, y)
            if any(x0 <= x <= x1 for x0, x1 in g.lichen_clear):
                continue
            if here and here[0] == "stone" and R(g, "li", x // 2, y // 2) > 0.95 and R(g, "li2", x, y) > 0.35:
                c.put(x, y, "lichen", 1 if R(g, "li3", x, y) > 0.5 else 0)


def _arc_chains(c: Pix, g: Arc) -> None:
    """Weathered chain stubs hang from iron staples in the inner band (oval links, 5 x 3 px)."""
    for a, links, tail in g.chains:
        x0, y0 = _polar(g, a, g.inner[0] + 3)
        x0, y0 = int(x0), int(y0)
        c.box(x0 - 1, y0 - 2, x0 + 1, y0, "iron", 2)                  # the staple
        c.put(x0, y0 - 2, "iron", 5)
        y = y0 + 1
        for k in range(links):
            if k % 2 == 0:                                             # link seen face-on: an oval ring
                for yy in range(5):
                    c.put(x0 - 1, y + yy, "iron", 5 if yy else 6)
                    c.put(x0 + 1, y + yy, "iron", 3)
                c.put(x0, y, "iron", 5)
                c.put(x0, y + 4, "iron", 2)
            else:                                                      # link seen edge-on: a short bar
                for yy in range(5):
                    c.put(x0, y + yy, "iron", 6 if yy < 2 else 4)
            y += 4
        c.put(x0 + tail, y, "iron", 2)                                 # the torn last link


def _arc_value_shift(c: Pix, g: Arc) -> None:
    """K1 (gate verdict, spec section 16): every ``stone`` index drops by ``g.shift`` (floor 1); on the outer
    and notch bands of the crown sector (|a| < ``crown_deg`` from the apex) it drops by ``crown_extra`` more
    (the soffit's shadow). ``mortar``, ``iron`` and ``lichen`` stay."""
    if g.shift == 0 and g.crown_extra == 0:
        _local_dark(c, g)
        return
    for y in range(c.h):
        for x in range(c.w):
            p = c.px[y][x]
            if p is None or p[0] != "stone":
                continue
            dx, dy = x + 0.5 - g.cx, y + 0.5 - g.cy
            r = math.hypot(dx, dy)
            a_deg = abs(math.degrees(math.atan2(dx, -dy)))
            drop = g.shift
            if a_deg < g.crown_deg and (r >= g.outer[0] or g.notch[0] <= r < g.notch[1]):
                drop += g.crown_extra
            c.px[y][x] = ("stone", min(g.cap, max(1, p[1] - drop)))
    _torch_bake(c, g)
    _local_dark(c, g)


def _local_dark(c: Pix, g: Arc) -> None:
    # gate C4 / review finding 8: a local darkening behind an actor's head line, per band (outer, notch, inner
    # drops) with a dithered falloff over ``SOFT`` px at every edge, never a hard-edged wedge
    for (x0, y0, x1, y1, drops) in g.local_dark:
        for y in range(max(0, y0 - SOFT), min(y1 + SOFT, c.h)):
            for x in range(max(0, x0 - SOFT), min(x1 + SOFT, c.w)):
                p = c.px[y][x]
                if p is None or p[0] != "stone":
                    continue
                inside = min(x - x0, x1 - 1 - x, y - y0, y1 - 1 - y)        # < 0 outside the box
                if inside >= SOFT:
                    w = 1.0
                else:
                    w = (inside + SOFT) / (2.0 * SOFT)                       # 0.5 at the edge, 0 at SOFT outside
                    if R(g, "ld", x, y) >= w:
                        continue
                r = math.hypot(x + 0.5 - g.cx, y + 0.5 - g.cy)
                band = 0 if r >= g.outer[0] else (1 if r >= g.notch[0] else 2)
                c.px[y][x] = ("stone", max(1, p[1] - drops[band]))


def _torch_bake(c: Pix, g: Arc) -> None:
    """Spec section 8 / RNG-AD22: the torch pools light the ring stone they overlap (+2 near, +1 far, a 12 px
    dithered edge on each radius), capped at stone 5 and never lowered."""
    if not g.torch_lit:
        return
    for y in range(c.h):
        for x in range(c.w):
            p = c.px[y][x]
            if p is None or p[0] != "stone":
                continue
            d = min(math.hypot(x + 0.5 - tx, y + 0.5 - ty) for tx, ty in g.torch_lit)
            n = R(g, "torch", x, y)
            add = sum(1 for rad in TORCH_R if d < rad - 12 or (d < rad and n < (rad - d) / 12))
            if add and p[1] < 5:
                c.px[y][x] = ("stone", min(5, p[1] + add))


def ring_arc(g: Arc) -> Pix:
    return draw_arc(Pix(g.w, g.h), g)


def ring_arc_wall() -> Pix:
    return ring_arc(ARC_A)


def ring_arc_wall_b() -> Pix:
    return ring_arc(ARC_B)


# =============================================================================================
# ring_shutter_closed (spec section 6). 32 x 68, prop. A dressed stone slab in running bond with one iron
# band and an iron-shod toothed lower edge; the moving frames are row crops of this key keyed off Shutter::edge_px.
# =============================================================================================
SH_W, SH_H = 32, 68
SH_COURSES = (0, 17, 34, 51)     # four 17 px courses: one course per 17 px of travel, slices stay 4 px


def ring_shutter_closed() -> Pix:
    c = Pix(SH_W, SH_H)
    for y in range(SH_H):
        for x in range(SH_W):
            course = max(i for i, top in enumerate(SH_COURSES) if top <= y)
            cy = y - SH_COURSES[course]
            # slab face (K2): the darkest, heaviest mass in the frame, stone 2..3 body
            v = 2.2 + 0.5 * (y / SH_H) + (rnd("shc", course) - 0.5) * 0.6
            v += (rnd("shn", x // 3, y // 2) - 0.5) * 0.5
            if cy == 0:
                v = 1.0                                                # course joint (dark)
            elif cy == 1:
                v += 1.4                                               # lit top arris of each course (stone 4)
            elif cy == 16:
                v -= 0.9
            if x == 1:
                v += 1.6                                               # lit left arris (stone 4)
            if x == SH_W - 2:
                v -= 1.1
            c.gput(x, y, "stone", clamp(v, 1.0, 4.4), "d1")
        c.put(0, y, "mortar", 1)                                       # hard outline: reads against brick
        c.put(SH_W - 1, y, "mortar", 0)
    # top: the slot shadow under the walkway
    for y in range(3):
        for x in range(SH_W):
            c.put(x, y, "mortar", y)
    # RNG-AD23 (review finding 6): no vertical straps, they framed the slab like a barred door leaf. Each
    # course is cut into dressed blocks in running bond (a dark joint, the block to its right lit on its left
    # arris), so the face reads as masonry: a slab, not a leaf.
    for course, top in enumerate(SH_COURSES):
        joints = (15,) if course % 2 == 0 else (7, 23)
        for jx in joints:
            for y in range(max(top + 1, 3), min(top + 17, SH_H - 8)):
                c.put(jx, y, "mortar", 1)
                p = c.px[y][jx + 1]
                if p is not None and p[0] == "stone":
                    c.put(jx + 1, y, "stone", p[1] + 1)
    for y in range(3, SH_H - 8):                                        # K2: stone 2..3 body, stone 4 at most
        for x in range(SH_W):
            p = c.px[y][x]
            if p is not None and p[0] == "stone" and p[1] > 4:
                c.px[y][x] = ("stone", 4)
    # K3: no carved mark on the slab. Course 2 carries one horizontal iron band across the full width
    # (horizontal = slab, vertical-only = door) with a rivet every 6 px; it hides that course's block joint.
    for x in range(1, SH_W - 1):
        c.put(x, 24, "iron", 3)
        c.put(x, 25, "iron", 2)
        c.put(x, 26, "iron", 2)
        c.put(x, 27, "iron", 1)
    for x in range(3, SH_W - 2, 6):
        c.put(x, 25, "iron", 5)
        c.put(x, 26, "iron", 1)
    # iron shoe: rows 60..67. Lit top row 60, body 61..63, the bright scuffed sweep edge on row 64 (the
    # lower-edge cue), four blunt teeth on rows 65..67 with transparent gaps between them
    for y in range(SH_H - 8, SH_H):
        for x in range(SH_W):
            ly = y - (SH_H - 8)
            if ly == 0:
                c.put(x, y, "iron", 5)
            elif ly < 4:
                c.put(x, y, "iron", 2 if x > SH_W - 4 else 3)
            elif ly == 4:
                c.put(x, y, "iron", 6 if 1 <= x <= SH_W - 2 else 4)
            else:
                tooth = (x - 1) % 8
                if 0 < x < SH_W - 1 and tooth < 5:
                    c.put(x, y, "iron", 4 if tooth == 0 else (1 if ly == 7 else 2))
                else:
                    c.px[y][x] = None
    return c


# =============================================================================================
# ring_camp_a (spec section 5, room 4b). 144 x 72, prop, bottom row on the floor back line FB (W - 20).
# A lean-to of torn canvas on a salvaged pole, a rolled bedroll under it, two stacked salvage crates
# with the camp lantern, and a tied cloth bundle. No figure: the survivors stay off-screen (KR14/KR16).
# =============================================================================================
CAMP_W, CAMP_H = 144, 72


def _canvas_lean_to(c: Pix) -> None:
    # the pole: a salvaged timber leaning a little to the right, lashed at the top
    for y in range(6, CAMP_H):
        x = 9 + (CAMP_H - y) // 22
        c.put(x, y, "wood", 3)
        c.put(x + 1, y, "wood", 2)
        c.put(x - 1, y, "wood", 4 if y % 9 else 3)
    c.box(9, 9, 12, 11, "wood", 1)                                     # lashing
    c.put(10, 9, "stone", 5)
    # the canvas: a sheet hung from the pole top, pegged at the floor to the right; folds every ~9 px
    top = (12, 8)
    peg = (64, CAMP_H - 3)
    for y in range(top[1], peg[1] + 1):
        t = (y - top[1]) / (peg[1] - top[1])
        x_end = int(top[0] + (peg[0] - top[0]) * t ** 0.85)
        sag = int(3 * math.sin(t * math.pi))
        for x in range(top[0], x_end + 1 + sag):
            # the hanging back curtain: the sheet's lower edge is torn short over the bedroll opening
            fold = math.sin((x - top[0]) * 0.7 + y * 0.12)
            v = 6.1 + 1.2 * fold - 1.8 * t
            if x >= x_end + sag - 1:
                v += 1.0                                               # lit outer edge facing the lantern
            c.gput(x, y, "stone", clamp(v, 2.0, 8.6), "d2")
    # the inside of the lean-to: the dark under the sheet, open at the floor
    for y in range(40, CAMP_H):
        x_in = top[0] + 2
        x_out = int(top[0] + (peg[0] - top[0]) * ((y - top[1]) / (peg[1] - top[1])) ** 0.85) - 7
        for x in range(x_in, max(x_in, x_out)):
            c.put(x, y, "mortar", 1 if (x + y) % 5 else 2)
    # torn hem: ragged tongues along the outer edge
    for y in range(18, peg[1] - 2, 5):
        t = (y - top[1]) / (peg[1] - top[1])
        x = int(top[0] + (peg[0] - top[0]) * t ** 0.85) + 2
        for k in range(1 + int(rnd("hem", y) * 3)):
            c.put(x + k, y + k, "stone", 5)
            c.put(x + k, y + k + 1, "stone", 3)
    # peg and guy line
    c.box(peg[0], peg[1] - 2, peg[0] + 1, CAMP_H - 1, "wood", 2)
    c.put(peg[0] + 1, peg[1] - 2, "wood", 4)


def _bedroll(c: Pix) -> None:
    """K4: a rolled blanket lying on its side as a horizontal cylinder: flat top and bottom rows, squared ends,
    the rolled end face on the left with one curl line, two dark leather ties. More than half of its length lies
    outside the lean-to's dark interior (x 14..53), against the canvas and the crate side."""
    x0, x1, y0, y1 = 44, 68, 62, 71
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            ly = y - y0
            if ly == 0:
                v = 2.0                                                # lit top row
            elif ly <= 2:
                v = 2.0 if R_BED(x, y) > 0.55 else 1.0
            elif ly <= 6:
                v = 1.0
            else:
                v = 0.0 if ly == y1 - y0 else (0.0 if R_BED(x, y) > 0.5 else 1.0)
            if x == x1:
                v = 0.0                                                # squared right end
            c.put(x, y, "lichen", v)
    # the rolled end face: a lighter end cap with a single dark curl line
    for y in range(y0, y1 + 1):
        for x in range(x0, x0 + 4):
            c.put(x, y, "lichen", 2 if y - y0 < 6 else 1)
    for (x, y) in ((x0 + 1, y0 + 2), (x0 + 2, y0 + 2), (x0 + 2, y0 + 3), (x0 + 2, y0 + 4), (x0 + 1, y0 + 5),
                   (x0 + 1, y0 + 6)):
        c.put(x, y, "lichen", 0)
    # two dark leather ties across the full height
    for tx in (52, 62):
        for y in range(y0, y1 + 1):
            c.put(tx, y, "wood", 1)
            c.put(tx + 1, y, "wood", 0)


def R_BED(x, y) -> float:
    return rnd("bed", x, y)


def _crate(c: Pix, x0, y0, w, h, seed) -> None:
    for y in range(y0, y0 + h):
        for x in range(x0, x0 + w):
            plank = (y - y0) // 6
            v = 3.4 + (rnd("pl", seed, plank) - 0.5) * 1.0 + (0.8 if x - x0 < 2 else 0) - (0.9 if x0 + w - x < 3 else 0)
            if (y - y0) % 6 == 5:
                v = 1                                              # plank gap
            c.gput(x, y, "wood", clamp(v, 0.0, 5.0), "d4")
    # diagonal brace and frame
    c.line(x0 + 2, y0 + h - 3, x0 + w - 3, y0 + 2, "wood", 5)
    c.line(x0 + 2, y0 + h - 2, x0 + w - 3, y0 + 3, "wood", 1)
    for x in range(x0, x0 + w):
        c.put(x, y0, "wood", 5)
        c.put(x, y0 + h - 1, "wood", 0)
    for y in range(y0, y0 + h):
        c.put(x0, y, "wood", 4)
        c.put(x0 + w - 1, y, "wood", 0)
    for bx, by in ((x0, y0), (x0 + w - 3, y0), (x0, y0 + h - 3), (x0 + w - 3, y0 + h - 3)):
        c.box(bx, by, bx + 2, by + 2, "iron", 3)                   # iron corner brackets
        c.put(bx + 1, by + 1, "iron", 6)


def _crates(c: Pix) -> None:
    crate = lambda *args: _crate(c, *args)  # noqa: E731
    crate(70, 46, 36, 26, "a")                                         # bottom crate on the floor
    crate(76, 24, 26, 22, "b")                                         # smaller crate on top, set back
    # a strip of the same canvas thrown over the top crate's corner: the shared REST-B marker cloth
    for y in range(24, 40):
        for x in range(96, 104):
            if x - 96 < (40 - y) * 0.6 + 2:
                c.gput(x, y, "stone", 6.6 - (y - 24) * 0.12 + (0.8 if x == 96 else 0), "d5")
    for y in range(40, 47):                                             # torn tail hanging down the side
        x = 103 - (y - 40) // 3
        c.put(x, y, "stone", 6)
        c.put(x + 1, y, "stone", 3)


def lantern_at(c: Pix, x0: int, y0: int) -> None:
    # the camp lantern: brass frame, horn panes glowing warm, a wire bail (origin = top-left of the bail row)
    c.box(x0 + 1, y0, x0 + 5, y0, "brass", 4)                          # bail
    c.put(x0, y0 + 1, "brass", 3)
    c.put(x0 + 6, y0 + 1, "brass", 3)
    c.box(x0, y0 + 2, x0 + 6, y0 + 3, "brass", 3)                      # cap
    c.put(x0 + 1, y0 + 2, "brass", 5)
    for y in range(y0 + 4, y0 + 12):                                    # panes
        for x in range(x0, x0 + 7):
            if x in (x0, x0 + 6):
                c.put(x, y, "brass", 2)
            elif x == x0 + 3:
                c.put(x, y, "brass", 3)
            else:
                d = math.hypot(x - (x0 + 3), (y - (y0 + 8)) * 0.8)
                c.put(x, y, "flame", 4 if d < 1.6 else 3) if d < 2.6 else c.put(x, y, "gal_warm", 5)
    c.box(x0, y0 + 12, x0 + 6, y0 + 13, "brass", 2)                    # base
    c.put(x0 + 1, y0 + 12, "brass", 4)
    # the warm spill: a few glow pixels on the crate top and the canvas edge (1-bit, dithered)
    for y in range(y0 + 2, y0 + 15):
        for x in range(x0 - 6, x0 + 13):
            if c.get(x, y) is None and (x - x0 + y - y0) % 2 == 0 and math.hypot(x - (x0 + 3), y - (y0 + 8)) < 6.5:
                c.put(x, y, "gal_warm", 3)


def _lantern(c: Pix) -> None:
    # the camp lantern on the top crate (the shared marker lantern, ring_marker_a)
    lantern_at(c, 84, 10)
    for x in range(76, 102):                                            # lit crate lid next to the lantern
        if c.get(x, 24) and abs(x - 87) < 9:
            c.put(x, 24, "gal_warm", 4)


def _bundle(c: Pix) -> None:
    # a cloth bundle tied with cord, and a dented pot beside it
    cx, cy = 122.0, 63.0
    for y in range(52, CAMP_H):
        for x in range(110, 136):
            e = ((x + 0.5 - cx) / 12.5) ** 2 + ((y + 0.5 - cy) / 9.5) ** 2
            if e <= 1.0 and y < CAMP_H:
                v = 5.3 - (y - 52) * 0.11 - (0.9 if e > 0.78 else 0) + (0.6 if x < cx - 5 else 0)
                c.gput(x, y, "stone", clamp(v, 2.0, 7.0), "d6")
    c.box(120, 52, 123, 54, "stone", 3)                                # knot
    c.line(115, 56, 129, 70, "wood", 2)                                 # cord
    for y in range(64, CAMP_H):                                         # pot
        for x in range(136, 143):
            c.put(x, y, "iron", 4 if x == 136 else (2 if x > 140 else 3))
    c.box(135, 63, 143, 63, "iron", 5)


def ring_camp() -> Pix:
    c = Pix(CAMP_W, CAMP_H)
    _canvas_lean_to(c)
    _bedroll(c)
    _crates(c)
    _lantern(c)
    _bundle(c)
    return c


# =============================================================================================
# Execution pieces (spec section 5). Same rules as the keys: palette ramps only, 1-bit alpha, flat faceless
# silhouettes, nothing traced. Key light from the upper left; backwall pieces stay in the stone 1..5 band
# with stone 6 on at most 3 % of their pixels (gate finding 1, C1).
# =============================================================================================
def _stone_run(c: Pix, x0, y0, x1, y1, base, seed, lit=0.0, shade=0.0, cap=6.0):
    """A stone patch with a sparse grain; ``lit`` raises the top row and left column, ``shade`` lowers the
    bottom row and right column."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            v = base
            if y == y0 or x == x0:
                v += lit
            if y == y1 or x == x1:
                v -= shade
            c.gput(x, y, "stone", clamp(v, 1.0, cap), seed)


def silhouette_standing(c: Pix, x: int, y_base: int, h: int = 18) -> None:
    """A flat faceless figure (``gal`` 0 with a 1 px ``gal`` 1 rim on the left): a round head, a cloak widening
    to the hem, two feet. ``x`` is the centre column."""
    top = y_base - h + 1
    for dy in range(4):                                                 # head
        w = (2, 4, 4, 2)[dy]
        for dx in range(w):
            c.put(x - w // 2 + dx, top + dy, "gal", 0)
    for dy in range(h - 6):                                             # cloak
        w = 6 + (dy * 3) // max(1, h - 7)
        for dx in range(w):
            c.put(x - w // 2 + dx, top + 4 + dy, "gal", 1 if dx == 0 else 0)
    for dx in (-2, 1):                                                  # feet
        c.put(x + dx, y_base, "gal", 0)
        c.put(x + dx, y_base - 1, "gal", 0)


def silhouette_seated(c: Pix, x: int, y_base: int, h: int = 14) -> None:
    """A flat faceless seated figure, knees drawn up: head, hunched back, a lap and two knees."""
    top = y_base - h + 1
    for dy in range(4):                                                 # head
        w = (2, 4, 4, 2)[dy]
        for dx in range(w):
            c.put(x - w // 2 + dx, top + dy, "gal", 0)
    for dy in range(h - 4):                                             # back and lap
        w = 6 + min(dy, 3) + (3 if dy >= h - 7 else 0)
        for dx in range(w):
            c.put(x - 3 + dx, top + 4 + dy, "gal", 1 if dx == 0 else 0)
    for dy in range(3):                                                 # knees
        c.put(x + 6, y_base - dy, "gal", 0)
        c.put(x + 7, y_base - dy, "gal", 0)


# --------------------------------------------------------------------------------- keystone and shield
KS_W, KS_H = 24, 30
KS_BITE = 6                     # the bottom rows that lie on the door arch's apex stone (RNG-AD21)


def ring_keystone_crowned() -> Pix:
    """24 x 30, prop (RNG-AD21). The entry landmark: an oversized keystone that *is* the apex stone of the
    entry door's arch. Its bottom ``KS_BITE`` rows (10 px wide) cover the door's apex voussoir, the tapered body
    rises 15 px above the arch on the bricked-up bay, and a crown of three blunt points tops it (the right one
    snapped off, a pale fracture face). No side voussoirs and no flat top, so it never reads as a ledge. No
    emblem."""
    c = Pix(KS_W, KS_H)
    left = lambda y: 4 + ((y - 9) * 3) // 20                            # noqa: E731  16 px at row 9, 10 at 29
    right = lambda y: 19 - ((y - 9) * 3) // 20                          # noqa: E731
    for y in range(9, KS_H):                                            # the tapered body
        for x in range(left(y), right(y) + 1):
            if x in (left(y), right(y)):
                c.put(x, y, "mortar", 1)                                # dark outline against the brick
                continue
            v = 4.1 + (rnd("kk", y // 5) - 0.5) * 0.4                   # the value of the door's arch stones
            if x == left(y) + 1:
                v += 1.0                                                # lit left arris
            elif x == right(y) - 1:
                v -= 0.9
            c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "kk")
    for x in range(left(KS_H - 1), right(KS_H - 1) + 1):               # the bed joint on the door's arch
        c.put(x, KS_H - 1, "mortar", 1)
    # the crown: three blunt points overlapping at the base (row 8), clipped to the body's top width
    for tx, top in ((7.5, 2), (11.5, 0), (15.5, 3)):
        for y in range(top, 9):
            hw = 1.0 + (y - top) * 0.5
            for x in range(int(round(tx - hw)), int(round(tx + hw)) + 1):
                if not (left(9) <= x <= right(9)):
                    continue
                v = 4.1
                if x <= int(round(tx - hw)) or y == top:
                    v += 1.1                                            # lit left slope and tip
                elif x >= int(round(tx + hw)):
                    v -= 1.0
                c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "kc")
    for x, y in ((9, 6), (9, 7), (13, 6), (13, 7), (13, 8)):           # the valleys between the points
        if c.px[y][x] is not None:
            c.put(x, y, "mortar", 2)
    # the right point is snapped: the fracture runs from its inner top down to the outer side
    for x in range(14, 21):
        line = 4.0 + (x - 14) * 0.6
        for y in range(0, 9):
            if c.px[y][x] is None:
                continue
            if y < line:
                c.px[y][x] = None
            elif y < line + 1.2:
                c.put(x, y, "stone", 5)                                 # the fresh, pale fracture edge
    return c


def ring_shield_notched() -> Pix:
    """32 x 40. A plain shield in relief on a dressed panel, three V notches cut in its top edge. No glyph."""
    c = Pix(32, 40)
    for y in range(40):                                                 # the dressed panel
        for x in range(32):
            v = 3.0 + (rnd("sp", x // 8, y // 8) - 0.5) * 0.6
            if y == 0 or x == 0:
                v += 0.9
            if y == 39 or x == 31:
                v -= 1.2
            if y == 20:
                c.put(x, y, "mortar", 2)                                 # one horizontal joint
                continue
            c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "sp")
    for y in range(5, 34):                                              # the shield relief
        hw = 10.0 if y < 20 else 10.0 * (1.0 - ((y - 19) / 14.0) ** 1.7)
        x0, x1 = int(round(15.5 - hw)), int(round(15.5 + hw)) - 1
        for x in range(x0, x1 + 1):
            if y == 5 and (8 <= x <= 11 or 14 <= x <= 17 or 20 <= x <= 23):
                c.put(x, y, "mortar", 1)                                 # V notch cut into the top edge, row 1
                continue
            if y == 6 and x in (9, 10, 15, 16, 21, 22):
                c.put(x, y, "mortar", 1)                                 # V notch, row 2
                continue
            if y == 7 and x in (9, 15, 21):
                c.put(x, y, "mortar", 1)                                 # the notch's tip
                continue
            v = 4.2 + (rnd("sh", x, y) - 0.5) * 0.4
            if x - x0 < 2 or y - 5 < 1:
                v += 1.0                                                 # lit upper left arrises
            if x1 - x < 2:
                v -= 1.4
            if y > 31:
                v -= 1.0
            c.gput(x, y, "stone", clamp(v, 1.0, 6.0), "sh")
        c.put(x1 + 1, y, "mortar", 1)                                    # relief shadow, right and below
    for (nx, ny) in ((0, 0), (1, 0), (0, 1), (31, 39)):                  # chipped panel corners (the brick shows)
        c.px[ny][nx] = None
    for x in range(8, 24):                                              # the carved inner line, 2 px in
        c.put(x, 8, "mortar", 2)
    for y in range(8, 24):
        c.put(8, y, "mortar", 2)
        c.put(23, y, "mortar", 2)
    return c


# -------------------------------------------------------------------------------------- the breach (R3)
def ring_breach_r3() -> Pix:
    """176 x 112. A hole broken through the ring wall: fractured voussoirs frame it on the left and top, ragged
    rubble on the right and below. Inside, the far camp seen from the ledge: a stone gate draped with cloth, two
    lantern dots, two stacked crates and two anonymous silhouettes (a, standing; b, seated), cold ``gal`` beyond."""
    W, H = 176, 112
    c = Pix(W, H)
    cx, cy, rx, ry = 94.0, 56.0, 66.0, 42.0
    light_dir = math.radians(-135.0)
    hole = {}
    for y in range(H):
        for x in range(W):
            ex, ey = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            th = math.atan2(ey, ex)
            wob = 0.05 * math.sin(3 * th + 0.7) + 0.04 * math.sin(7 * th + 2.1) + 0.03 * math.sin(13 * th)
            d = math.hypot(ex, ey) + wob
            side = max(0.0, min(1.0, (math.cos(th - light_dir) + 0.2) / 1.2))   # 1 upper left, 0 lower right
            fw = 0.07 + 0.26 * side
            hole[(x, y)] = (d, th, fw, side)
    # the frame: radial joints every 15 degrees, two courses, offset
    for y in range(H):
        for x in range(W):
            d, th, fw, side = hole[(x, y)]
            if 1.0 <= d < 1.0 + fw:
                u = (d - 1.0) / fw
                if u > 0.82 and rnd("br_chip", x // 2, y // 2) > 0.55:
                    continue                                             # a broken outer edge
                course = 0 if u < 0.5 else 1
                ang = (math.degrees(th) + (7.0 if course else 0.0)) / 15.0
                fk = ang - math.floor(ang)
                v = 3.2 + (rnd("br", int(math.floor(ang)), course) - 0.5) * 0.9
                if d < 1.045:
                    v = 1.6 if side > 0.5 else 4.6                       # inner arris: dark on the lit side, lit opposite
                elif abs(u - 0.5) < 0.04 or fk < 0.03:
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "br")
    # the view inside the hole
    for y in range(H):
        for x in range(W):
            d = hole[(x, y)][0]
            if d >= 1.0:
                continue
            t = y / 98.0
            base = 3.9 - 1.7 * t                                          # cold gal 2..4, lighter high up
            step = int(base)
            bayer = ((0.0, 0.5), (0.75, 0.25))[y % 2][x % 2]
            c.put(x, y, "gal", step + (1 if base - step > bayer else 0))
    # the far ring wall: a low arcade of dark arches across the back
    for x in range(26, 170):
        for y in range(40, 80):
            if hole[(x, y)][0] >= 1.0:
                continue
            k = (x - 26) % 20
            arch_top = 46 + (abs(k - 10) ** 2) // 8 if 3 <= k <= 17 else 99
            if y >= arch_top and 3 <= k <= 17:
                c.put(x, y, "gal", 1)
            elif y >= 44:
                c.put(x, y, "gal", 3 if y % 6 else 2)
    # the camp ground: a dim floor line below
    for y in range(80, 100):
        for x in range(0, W):
            if hole[(x, y)][0] < 1.0:
                c.put(x, y, "gal", 2 if y > 81 else 4)
    # the carved stone gate (centre) draped with cloth
    gx0, gx1 = 68, 114
    for y in range(36, 82):
        for x in range(gx0, gx1 + 1):
            if hole[(x, y)][0] >= 1.0:
                continue
            post = x < gx0 + 8 or x > gx1 - 8
            if y < 44 or post:
                v = 4.6 + (rnd("gt", x // 3, y // 3) - 0.5) * 0.7
                if x == gx0 or y == 36:
                    v += 0.6
                if x == gx1:
                    v -= 1.2
                c.gput(x, y, "stone", clamp(v, 1.0, 6.0), "gt")
            else:
                c.put(x, y, "gal", 0 if y < 70 else 1)                   # the dark passage
    for y in range(70, 82):                                              # a warm glow low in the passage
        for x in range(gx0 + 8, gx1 - 7):
            if (x + y) % 2 == 0 and y > 74 - (rnd("gw", x) * 3):
                c.put(x, y, "gal_warm", 2 if y > 77 else 1)
    cloth = [(60, 33, 12, 40), (108, 33, 16, 34)]                        # (x, top, w, length): strips over the lintel
    for sx, top, w, length in cloth:
        for y in range(top, top + length):
            for x in range(sx, sx + w):
                if hole[(x, y)][0] >= 1.0:
                    continue
                fold = math.sin((x - sx) * 1.1 + y * 0.08)
                v = 5.7 + 0.9 * fold - 0.015 * (y - top)
                if y > top + length - 6 and (x + y) % 3 == 0:
                    continue                                              # torn hem
                c.gput(x, y, "stone", clamp(v, 4.0, 7.0), "cl")
    for x in range(60, 124):                                     # the cloth over the lintel: a short sag
        for y in range(33, 37 + int(2 * math.sin((x - 60) / 64.0 * math.pi))):
            if hole[(x, y)][0] < 1.0:
                c.gput(x, y, "stone", 5.2 + (0.8 if x % 5 < 2 else 0), "cl")
    # crates stacked at the right, a pole with a lantern at the left, a second lantern on the crates
    for (x0, y0, w, h) in ((124, 70, 22, 14), (128, 60, 15, 10)):
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w):
                if hole[(x, y)][0] < 1.0:
                    c.put(x, y, "wood", 3 if (y - y0) % 5 else 1)
        for x in range(x0, x0 + w):
            if hole[(x, y0)][0] < 1.0:
                c.put(x, y0, "wood", 2)
    for y in range(58, 84):                                               # lantern pole
        c.put(44, y, "gal", 2)
    for (lx, ly) in ((44, 56), (136, 57)):
        for dx, dy, v in ((0, 0, 5), (1, 0, 5), (0, 1, 5), (1, 1, 6)):
            c.put(lx + dx, ly + dy, "gal_warm", v)
        for dy in range(-3, 6):
            for dx in range(-4, 6):
                if c.get(lx + dx, ly + dy) is None or c.get(lx + dx, ly + dy)[0] == "gal":
                    if (dx + dy) % 2 == 0 and math.hypot(dx - 0.5, dy - 0.5) < 4.2:
                        c.put(lx + dx, ly + dy, "gal_warm", 3)
    silhouette_standing(c, 56, 83, 19)                                    # survivor_silhouette_a: standing
    silhouette_seated(c, 120, 83, 13)                                     # survivor_silhouette_b: seated, by the crates
    # rubble on the bottom edge: a few fallen blocks leaning into the hole
    for (bx, by, bw, bh) in ((22, 88, 13, 8), (30, 94, 16, 8), (122, 90, 14, 7), (136, 94, 12, 7), (92, 96, 12, 6)):
        for y in range(by, by + bh):
            for x in range(bx, bx + bw):
                v = 3.0 + (rnd("rb", bx, x // 4, y // 4) - 0.5) * 0.8
                if y == by:
                    v += 0.9
                if x == bx + bw - 1 or y == by + bh - 1:
                    v -= 1.4
                c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "rb")
    return c


# ------------------------------------------------------------------------ soffit, ledges, jambs, housing
def ring_soffit() -> Pix:
    """32 x 16, tiles seamlessly. The flat underside of the ring's walkway course: a dressed-stone fascia. The
    underside row is lit warm from below (stone 7), the face is stone 3..5 with a joint every 32 px and a
    running-bond course, the top row is dark mortar (RNG-AD4: no walkable-top cue)."""
    c = Pix(32, 16)
    for y in range(16):
        for x in range(32):
            if y == 0:
                c.put(x, y, "mortar", 1)
            elif y == 15:
                c.put(x, y, "stone", 7)
            else:
                course = 0 if y < 8 else 1
                v = 3.7 + (rnd("sf", x // 8, course) - 0.5) * 0.8
                if y in (1, 9):
                    v += 0.8
                if y == 14:
                    v -= 0.8
                joint = (x == 0) if course == 0 else (x == 16)
                if y == 8 or joint:
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "sf")
    c.px[0][0] = None                                                   # a nick in the dark top row at each joint
    return c


def _ledge_body(c: Pix, w: int, seed: str, x_off: int = 0) -> None:
    """The ring-stone ledge front (gate C1): a full-width lit top arris of stone 7, a stone 4..5 face with a
    course joint and bond joints, and a 4 px mortar 0..1 underside."""
    for y in range(16):
        for x in range(w):
            ax = x + x_off
            if y == 0:
                c.put(x, y, "stone", 7)
            elif y == 1:
                c.put(x, y, "stone", 6)
            elif y >= 12:
                c.put(x, y, "mortar", 1 if y < 14 else 0)
            else:
                course = 0 if y < 7 else 1
                v = 4.4 + (rnd(seed, ax // 8, course) - 0.5) * 0.9
                if y in (2, 8):
                    v += 0.5
                joint = (ax % 32 == 0) if course == 0 else (ax % 32 == 16)
                if y == 7 or joint:
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 4.0, 5.0), seed)
                c.px[y][x] = ("stone", clamp(c.px[y][x][1], 4, 5))             # the face is stone 4..5 (gate C1)


def ring_ledge_mid() -> Pix:
    c = Pix(32, 16)
    _ledge_body(c, 32, "lg")
    c.px[15][0] = None                                                  # a nick in the shadowed underside
    return c


def ring_ledge_end(left: bool) -> Pix:
    """8 x 16 end cap (opaque: it draws over the run's first or last 8 px). The return is a lit chamfer on the
    outer edge; the corner pixels are dark mortar so the end reads rounded against the brick."""
    c = Pix(8, 16)
    _ledge_body(c, 8, "lg", 0 if left else 24)
    outer = (lambda x: x) if left else (lambda x: 7 - x)                    # noqa: E731
    for y in range(16):
        c.put(outer(0), y, "stone", 3 if 2 <= y < 12 else 0)                # the return face, a step darker
        if y in (2, 3, 4):
            c.put(outer(1), y, "stone", 4)
    for (dx, y, v) in ((0, 0, 1), (1, 0, 5), (0, 1, 1), (0, 14, 0), (1, 15, 0)):
        c.put(outer(dx), y, "mortar" if v < 5 else "stone", v if v < 5 else 6)
    c.px[15][outer(0)] = None                                           # the rounded lower corner: the brick shows
    return c


def ring_shutter_jamb(left: bool) -> Pix:
    """8 x 68. A guide channel of the arch: dressed stone with a dark 2 px slot facing the shutter."""
    c = Pix(8, 68)
    for y in range(68):
        course = y // 17
        for x in range(8):
            slot = x >= 6 if left else x < 2
            if slot:
                c.put(x, y, "mortar", 0 if (x == 6 or x == 0) else 1)
                continue
            v = 3.3 + (rnd("jb", course, left) - 0.5) * 0.6
            if y % 17 == 0:
                c.put(x, y, "mortar", 2)
                continue
            if y % 17 == 1:
                v += 0.9
            if x == (0 if left else 7):
                v += 0.8
            c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "jb")
    c.px[67][0 if left else 7] = None                                   # a chipped outer foot
    return c


def ring_shutter_housing() -> Pix:
    """48 x 6. The slot lintel under the walkway: it draws over the shutter's top 6 rows so a closing slab emerges
    from under it. Dark, with a mortar shadow line along its bottom."""
    c = Pix(48, 6)
    for y in range(6):
        for x in range(48):
            if y == 0:
                c.put(x, y, "mortar", 1)
            elif y == 5:
                c.put(x, y, "mortar", 0)
            elif y == 4:
                c.put(x, y, "stone", 1)
            else:
                v = 2.7 + (rnd("hs", x // 6, y) - 0.5) * 0.7
                if x in (0, 47):
                    v -= 1.0
                c.gput(x, y, "stone", clamp(v, 1.0, 4.0), "hs")
    c.px[0][0] = None
    c.px[0][47] = None                                                  # chipped lintel corners
    return c



# ------------------------------------------------------------------------------- the shutter's extras
def ring_shutter_grit(variant: str) -> Pix:
    """32 x 28 (drawn at x, y + 6 during Warning): 4..6 falling grit specks (stone 5..7, mortar 2) and a 1 px
    dust streak under the slot; ``a`` and ``b`` alternate every 6 ticks."""
    c = Pix(32, 28)
    spots = {"a": ((5, 3), (12, 9), (19, 4), (26, 12), (9, 17), (22, 21)),
             "b": ((8, 6), (15, 2), (23, 10), (4, 14), (17, 19), (28, 24))}[variant]
    for i, (x, y) in enumerate(spots):
        c.put(x, y, "stone", (6, 5, 7, 5, 6, 5)[i])
        if i % 2 == 0:
            c.put(x, y + 1, "mortar", 2)
    sx = 13 if variant == "a" else 18
    for y in range(0, 6):
        c.put(sx, y, "stone", 5)
    return c


def ring_shutter_shadow(step: int) -> Pix:
    """40 x 4: the slab's shadow gathering on the floor under the slot. ``mortar`` 0 at 25 %, 50 %, 75 % checker
    density for ``a``, ``b``, ``c``."""
    c = Pix(40, 4)
    for y in range(4):
        for x in range(40):
            if step == 0:
                on = x % 2 == 0 and y % 2 == 0
            elif step == 1:
                on = (x + y) % 2 == 0
            else:
                on = not (x % 2 == 1 and y % 2 == 1)
            if on and 2 <= x < 38:
                c.put(x, y, "mortar", 0)
    return c


def ring_shutter_dust(variant: str) -> Pix:
    """48 x 10, foreground: scrape dust at the slab's lower edge (the slab spans x 8..39 of the piece). ``a`` and
    ``b`` alternate; ``c`` is the floor-contact puff on entering Closed. Stone 5..7 puffs, 1-bit."""
    c = Pix(48, 10)
    puffs = {
        "a": ((5, 6, 3.4), (2, 8, 2.2), (42, 7, 2.8), (45, 5, 1.6), (13, 9, 1.4)),
        "b": ((4, 7, 2.6), (7, 4, 2.0), (43, 6, 3.4), (46, 8, 2.2), (35, 9, 1.4)),
        "c": ((4, 7, 3.6), (9, 8, 3.0), (2, 5, 2.0), (43, 7, 3.6), (38, 8, 3.0), (46, 5, 2.0)),
    }[variant]
    for (px, py, r) in puffs:
        for y in range(10):
            for x in range(48):
                d = math.hypot(x + 0.5 - px, (y + 0.5 - py) * 1.25)
                if d < r and rnd("du", variant, x, y) > 0.28 + 0.12 * d / r:
                    c.put(x, y, "stone", 7 if d < r * 0.4 else (6 if d < r * 0.75 else 5))
    return c


# ----------------------------------------------------------------------------------- camp-atlas pieces
def ring_fallen_voussoir(variant: str) -> Pix:
    """Broken outer-band blocks lying on the floor (at most 14 px tall, no full-width lit top). ``a`` 32 x 16 is a
    wedge lying flat with two joints; ``b`` 24 x 14 lies on its side showing a notch."""
    if variant == "a":
        c = Pix(32, 16)
        for y in range(2, 16):
            for x in range(1, 31):
                top = 3 + (x - 1) // 9 if x < 20 else 5 + (31 - x) // 4
                if y < top:
                    continue
                if x > 28 - max(0, 15 - y) // 2 * 0:
                    pass
                v = 3.1 + (rnd("fa", x // 6, y // 5) - 0.5) * 0.7
                if y == top and (x // 5) % 2 == 0:
                    v += 1.0                                              # broken lit run, 5 px at most
                if y >= 14 or x >= 29:
                    v -= 1.3
                if x in (11, 21):
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "fa")
        return c
    c = Pix(24, 14)
    for y in range(1, 14):
        for x in range(1, 23):
            if y < 2 + (x // 8):
                continue
            v = 3.0 + (rnd("fb", x // 5, y // 5) - 0.5) * 0.7
            if y == 2 + (x // 8) and x % 6 < 3:
                v += 1.0
            if y >= 12 or x >= 21:
                v -= 1.3
            if 8 <= x <= 14 and 4 <= y <= 9:                                # the carved notch, seen end on
                c.put(x, y, "mortar", 3 if y == 4 or x == 8 else 1)
                continue
            c.gput(x, y, "stone", clamp(v, 1.0, 5.0), "fb")
    return c


def ring_crates_b() -> Pix:
    """56 x 28 (gate C5: no taller than 28 px, so it never reads as the R3 ledge's continuation). Two salvage
    crates side by side, the key's crate rules; a rope coil and a canvas corner break the top edge."""
    c = Pix(56, 28)
    _crate(c, 0, 6, 26, 22, "e")
    _crate(c, 28, 10, 28, 18, "f")
    for (cx, cy) in ((8, 3), (13, 3)):                                  # the rope coil on the left crate: stacked loops
        for dy in range(-3, 4):
            for dx in range(-3, 4):
                d = math.hypot(dx, dy * 1.4)
                if 1.6 <= d < 3.6 and 0 <= cy + dy < 7:
                    c.put(cx + dx, cy + dy + 2, "wood", 4 if (dx + dy) % 2 else 3)
    for y in range(6, 16):                                              # canvas thrown over the right crate's corner
        for x in range(44, 56):
            if x - 44 < (16 - y) * 1.0 + 3 and y < 6 + (x - 44) * 0.5 + 4:
                c.gput(x, y, "stone", 5.4 - (y - 6) * 0.1 + (0.6 if x == 44 else 0), "cv")
    return c


def ring_marker_a() -> Pix:
    """24 x 56. The shared REST-B marker (KR14): a salvaged pole with a torn strip of the camp canvas tied at the
    top and the camp lantern hung below it (pixel-identical to the camp key's lantern)."""
    c = Pix(24, 56)
    for y in range(0, 56):                                              # the pole
        c.put(3, y, "wood", 4 if y % 11 else 3)
        c.put(4, y, "wood", 3)
        c.put(5, y, "wood", 1)
    for y in range(3, 24):                                              # the torn canvas strip tied at the top
        for x in range(6, 15):
            tear = 14 - (y - 3) // 3 - (1 if rnd("mk", y) > 0.6 else 0)
            if x > tear:
                continue
            if y > 19 and (x + y) % 3 == 0:
                continue
            fold = math.sin((x - 6) * 1.0 + y * 0.35)
            c.gput(x, y, "stone", 5.8 + 0.8 * fold - (y - 3) * 0.04, "mk")
    c.box(3, 3, 6, 5, "wood", 1)                                         # the lashing
    c.put(5, 4, "iron", 4)
    for x in range(6, 14):                                              # the bracket arm and the hook line
        c.put(x, 27, "iron", 3)
    c.put(13, 28, "iron", 3)
    lantern_at(c, 12, 29)
    for (sx, sy, sw, sh) in ((0, 50, 9, 6), (8, 52, 8, 4), (14, 53, 7, 3)):  # the cairn at the foot
        _stone_run(c, sx, sy, sx + sw - 1, sy + sh - 1, 3.0, "ca", lit=0.6, shade=1.2, cap=4.0)
    return c



# ================================================================================================ parallax
PAR_FAR_W, PAR_MID_W, PAR_H = 440, 344, 232
FAR_LAMPS = (50, 94, 182, 270, 314, 358, 402)       # the opening kit's far torch offsets (y 164): lamp pixels sit on them
MID_LAMP = (107, 166)


def _bow(x: int, amp: float, period: int) -> int:
    return int(round(amp * math.cos(2 * math.pi * x / period)))


def _arch_open(c: Pix, x0: int, x1: int, y_top: int, y_bot: int, fill, rise: int = None) -> None:
    """A rounded-top opening between columns ``x0``..``x1`` filled by ``fill(x, y)`` (None = transparent)."""
    w = x1 - x0 + 1
    rise = rise if rise is not None else w // 2
    for y in range(y_top, y_bot + 1):
        for x in range(x0, x1 + 1):
            dy = y - (y_top + rise)
            if dy < 0:
                dx = abs(x + 0.5 - (x0 + w / 2.0))
                if dx * dx + dy * dy > (w / 2.0) ** 2:
                    continue
            r = fill(x, y)
            if r is None:
                if 0 <= y < c.h and 0 <= x < c.w:
                    c.px[y][x] = None
            else:
                c.put(x, y, *r)


def ring_tiers_far() -> Pix:
    """440 x 232, parallax_far, tiles in x. The far ring seen through the arches: curved colosseum-like tiers in
    cold ``gal`` with arched openings, dark grates and seven single warm lamp pixels on the opening kit's torch
    offsets. No figures."""
    c = Pix(PAR_FAR_W, PAR_H)
    for y in range(PAR_H):                                              # the back wall: cold gal, lighter high up
        base = 3.2 - 1.6 * y / PAR_H
        for x in range(PAR_FAR_W):
            bayer = ((0.0, 0.5), (0.75, 0.25))[y % 2][x % 2]
            c.put(x, y, "gal", int(base) + (1 if base - int(base) > bayer else 0))
    for x in range(PAR_FAR_W):
        b1, b2 = _bow(x, 3, 220), _bow(x, 3, 220)
        for y in range(10 + b1, 16 + b1):
            c.put(x, y, "gal", 3)                                         # cornice of tier 1
        for y in range(16 + b1, 20 + b1):
            c.put(x, y, "gal", 2)
        # tier 2 cornice
        for y in range(92 + b2, 97 + b2):
            c.put(x, y, "gal", 3 if y < 94 + b2 else 2)
        # tier 3 balustrade and base
        for y in range(184, 192):
            c.put(x, y, "gal", 4 if y == 184 else 3)
        for y in range(192, PAR_H):
            c.put(x, y, "gal", 1 if (y // 16) % 2 else 2)               # horizontal bands only: no seam
    # tier 1 windows (drawn per arch so the rounded tops are whole): pitch 22, 14 wide
    for k in range(PAR_FAR_W // 22):
        x0 = k * 22 + 4
        b = _bow(x0 + 7, 3, 220)
        _arch_open(c, x0, x0 + 13, 22 + b, 62 + b,
                   lambda xx, yy, b=b: ("gal", 6 if yy < 30 + b else (5 if yy < 48 + b else 4)), rise=7)
        for y in range(22 + b, 63 + b):                                    # mullion
            c.put(x0 + 7, y, "gal", 3)
    # tier 2: the arcade. Piers at 44 k + 6 (16 wide, the wrap seam falls inside the first pier), arched openings
    # 28 wide between them
    for k in range(PAR_FAR_W // 44):
        px = 44 * k + 6
        x0 = px + 8
        b = _bow(px + 22, 3, 220)
        top = 100 + b

        def opening(xx, yy, k=k, x0=x0):
            if k % 2 == 1 and (xx - x0) % 4 == 2:
                return ("gal", 2)                                          # the dark grate
            return ("gal", 0 if yy < 150 else 1)

        _arch_open(c, x0, x0 + 27, top, 182, opening, rise=14)
        for y in range(top, 183):                                           # pier faces: lit left edge
            for x in range(px - 8, px + 8):
                c.put(x % PAR_FAR_W, y, "gal", 4 if x == px - 8 else 3)
    for lx in FAR_LAMPS:                                                    # seven warm lamp pixels on the lamp offsets
        c.put(lx, 163, "gal", 1)
        c.put(lx, 164, "gal_warm", 5)
        c.put(lx, 165, "gal", 1)
    return c


def ring_tiers_mid() -> Pix:
    """344 x 232, parallax_mid, tiles in x. A nearer curved arcade bridge: transparent arch openings under a deck
    with a balustrade, two stone-toned cloth strips hanging from the deck, cold ``gal``. The one warm lamp pixel
    sits on the opening kit's mid torch offset."""
    c = Pix(PAR_MID_W, PAR_H)
    for x in range(PAR_MID_W):
        b = _bow(x, 3, 344)
        for y in range(84 + b, 98 + b):                                    # balustrade: rail and balusters
            if y == 84 + b or y == 85 + b:
                c.put(x, y, "gal", 5 if y == 84 + b else 4)
            elif x % 8 in (2, 3, 4):
                c.put(x, y, "gal", 3 if x % 8 == 2 else 2)
        for y in range(98 + b, 120 + b):                                   # deck face and corbels
            v = 3 if y < 102 + b else (2 if y < 116 + b else 1)
            if (x % 43) in (0, 1) and y >= 108 + b:
                v = 4
            c.put(x, y, "gal", v)
        for y in range(218, PAR_H):                                        # the base slab
            c.put(x, y, "gal", 2 if y == 218 else 1)
    for k in range(4):                                                      # piers 16 wide, pitch 86, arches between
        px = 12 + 86 * k
        x_open0, x_open1 = px + 16, px + 16 + 69
        b = _bow(px + 43, 3, 344)
        for y in range(120 + b, 218):
            for x in range(px, px + 16):
                c.put(x % PAR_MID_W, y, "gal", 3 if x == px else (2 if x < px + 12 else 1))
        # the arch ring around each opening (the opening itself stays transparent)
        for x in range(x_open0, x_open1 + 1):
            for y in range(120 + b, 156 + b):
                dx = abs(x + 0.5 - (x_open0 + 35))
                dy = y - (155 + b)
                r = math.hypot(dx, dy)
                if 35 <= r < 41 and dy < 0:
                    c.put(x % PAR_MID_W, y, "gal", 3 if dx < 0 else 2)
        # keep the spandrels dark between the ring and the deck
        for x in range(x_open0, x_open1 + 1):
            for y in range(120 + b, 156 + b):
                dx = abs(x + 0.5 - (x_open0 + 35))
                dy = y - (155 + b)
                if math.hypot(dx, dy) >= 41 and c.get(x % PAR_MID_W, y) is None:
                    c.put(x % PAR_MID_W, y, "gal", 1)
    for cx in (62, 234):                                                    # two stone-toned cloth strips
        for y in range(120, 168):
            for x in range(cx, cx + 9):
                if y > 160 and (x + y) % 3 == 0:
                    continue
                fold = math.sin((x - cx) * 1.3 + y * 0.1)
                c.gput(x, y, "gal", 5.0 + 0.9 * fold - (y - 120) * 0.01, "mc")
    lx, ly = MID_LAMP
    c.put(lx, ly - 2, "gal", 1)
    c.put(lx, ly - 1, "gal", 1)
    c.put(lx, ly, "gal_warm", 5)
    return c


def ring_camp_far() -> Pix:
    """440 x 232, parallax_far, tiles in x (4b). The far camp beyond the ring: two canvas awnings, lantern dots, a
    campfire glow (``flame`` 2..3 core, ``gal_warm`` 3..4 spill, 1-bit) and ``survivor_silhouette_c`` (standing) and
    ``_d`` (seated), flat ``gal`` 0..1."""
    c = Pix(PAR_FAR_W, PAR_H)
    for y in range(PAR_H):
        base = 2.6 - 1.2 * y / PAR_H
        for x in range(PAR_FAR_W):
            bayer = ((0.0, 0.5), (0.75, 0.25))[y % 2][x % 2]
            c.put(x, y, "gal", int(base) + (1 if base - int(base) > bayer else 0))
    for k in range(PAR_FAR_W // 44):                                       # the far wall: dark arcade, no windows
        px = 44 * k + 6
        for xx in range(px + 8, px + 36):
            for yy in range(60, 172):
                dy = yy - 74
                dx = abs(xx + 0.5 - (px + 22))
                if dy < 0 and dx * dx + dy * dy > 196:
                    continue
                c.put(xx % PAR_FAR_W, yy, "gal", 1)
        for yy in range(52, 172):
            for xx in range(px - 8, px + 8):
                c.put(xx % PAR_FAR_W, yy, "gal", 3 if xx == px - 8 else 2)
    for x in range(PAR_FAR_W):                                              # the ground
        for y in range(172, PAR_H):
            c.put(x, y, "gal", 2 if y < 178 else 1)
        c.put(x, 172, "gal", 3)
    for (ax0, ax1) in ((36, 112), (256, 332)):                              # the two canvas awnings
        mid = (ax0 + ax1) / 2.0
        for x in range(ax0, ax1 + 1):
            roof_y = 126 + int(abs(x - mid) / (mid - ax0) * 16)
            for y in range(roof_y, roof_y + 7):
                fold = math.sin((x - ax0) * 0.55)
                c.gput(x, y, "gal", 5.2 + 0.8 * fold - (y - roof_y) * 0.25, "aw")
            for y in range(roof_y + 7, 168):                                # the dark under the awning
                c.put(x, y, "gal", 0)
            if (x - ax0) % 9 == 0:
                for y in range(roof_y + 7, 172):
                    c.put(x, y, "gal", 2)
        for px in (ax0 + 2, ax1 - 2):                                        # poles
            for y in range(140, 172):
                c.put(px, y, "gal", 3)
    for pole in (358, 402):                                                 # lone lantern posts
        for y in range(160, 172):
            c.put(pole, y, "gal", 3)
    for lx in FAR_LAMPS:
        if lx == 182:
            continue
        c.put(lx, 163, "gal", 1)
        c.put(lx, 164, "gal_warm", 5)
        c.put(lx, 165, "gal", 1)
    # the campfire: logs, a flame core on the fire offset (182, 164), a 1-bit warm spill
    for dx in range(-5, 6):
        c.put(182 + dx, 170, "wood", 1 if abs(dx) > 2 else 2)
        c.put(182 + dx, 171, "wood", 0)
    for (dx, dy, v) in ((0, -1, 3), (-1, 0, 2), (0, 0, 3), (1, 0, 2), (0, 1, 2), (-1, 1, 2), (1, 1, 2), (0, 2, 2), (0, -2, 2)):
        c.put(182 + dx, 164 + dy, "flame", v)
    for y in range(152, 172):
        for x in range(170, 194):
            d = math.hypot(x - 182, (y - 165) * 1.2)
            if c.get(x, y) and c.get(x, y)[0] == "gal" and (x + y) % 2 == 0 and d < 10:
                c.put(x, y, "gal_warm", 4 if d < 4.5 else 3)
    silhouette_standing(c, 198, 176, 22)                                    # survivor_silhouette_c
    silhouette_seated(c, 160, 176, 13)                                      # survivor_silhouette_d
    return c

# ===================================================================================== upper arcs
def ring_arc_upper(variant: str) -> Pix:
    """256 x 44. The strip above the soffit: segments of two larger rings (r about 400) crossing as shallow arcs,
    outer voussoirs and a notch band only, K1 shifted one step again (the soffit's shadow). Transparent elsewhere.
    Ring P (crest at local x 128 of ``a``) and ring Q (crest at local x 128 of ``b``) are the two halves of a
    512 px pattern, so ``a``, ``b``, ``a``, ``b`` chain into one run of crossing arcs."""
    c = Pix(256, 44)
    base = dict(w=256, h=44, seed="u" + variant, keystone=False, outer=(388.0, 400.0), groove_a=(386.0, 388.0),
                notch=(378.0, 386.0), groove_b=(376.0, 378.0), inner=None, shadow=None,
                chains=(), cracks=(), chip_range=(0, 0), lichen_rows=0, lift=(-0.3, -0.3), shift=2,
                crown_deg=0.0, base=(4.6, 4.2, 4.2), brk={"outer": (900.0, 901.0), "notch": (900.0, 901.0),
                                                           "inner": (900.0, 901.0)})
    cy = 404.0                                              # the crest's outer edge sits 4 px below the top edge
    if variant == "b":
        # review finding 7: `b` is its own ring pair, not a re-seeded copy of `a`. A tighter radius (340) with the
        # crest lifted so the arcs still meet `a`'s at the piece edges (the same y at x 0 and x 256), its own
        # joint pitch (the angular pitch is the same, the blocks are shorter) and its own hairline cracks.
        base.update(outer=(328.0, 340.0), groove_a=(326.0, 328.0), notch=(318.0, 326.0), groove_b=(316.0, 318.0),
                    cracks=((-17.0, 336.0, 11, 0.7), (11.0, 322.0, 8, -0.6), (-31.0, 330.0, 9, 0.5),
                            (27.0, 336.0, 10, -0.8)))
        cy = 340.0
    # ring crests at local x 128, -128 and 384 (the flank of the neighbours): the arcs cross at the piece edges
    for cx in (128.0, -128.0, 384.0):
        draw_arc(c, Arc(cx=cx, cy=cy, **base))
    return c



# ======================================================================================== atlases
ATLASES = {
    "floor01_ring_wall": {
        "stem": "ring_wall", "width": 416,
        "pieces": [("ring_arc_wall_a", ring_arc_wall), ("ring_arc_wall_b", ring_arc_wall_b),
                   ("ring_relief_landmark", lambda: ring_arc(ARC_LM)),
                   ("ring_keystone_crowned", ring_keystone_crowned), ("ring_shield_notched", ring_shield_notched),
                   ("ring_breach_r3", ring_breach_r3),
                   ("ring_arc_upper_a", lambda: ring_arc_upper("a")), ("ring_arc_upper_b", lambda: ring_arc_upper("b"))],
    },
    "floor01_ring_structure": {
        "stem": "ring_structure", "width": 128,
        "pieces": [("ring_soffit", ring_soffit), ("ring_ledge_mid", ring_ledge_mid),
                   ("ring_ledge_end_l", lambda: ring_ledge_end(True)), ("ring_ledge_end_r", lambda: ring_ledge_end(False)),
                   ("ring_shutter_jamb_l", lambda: ring_shutter_jamb(True)),
                   ("ring_shutter_jamb_r", lambda: ring_shutter_jamb(False)),
                   ("ring_shutter_housing", ring_shutter_housing)],
    },
    "floor01_ring_shutter": {
        "stem": "ring_shutter", "width": 128,
        "pieces": [("ring_shutter_closed", ring_shutter_closed),
                   ("ring_shutter_grit_a", lambda: ring_shutter_grit("a")), ("ring_shutter_grit_b", lambda: ring_shutter_grit("b")),
                   ("ring_shutter_shadow_a", lambda: ring_shutter_shadow(0)), ("ring_shutter_shadow_b", lambda: ring_shutter_shadow(1)),
                   ("ring_shutter_shadow_c", lambda: ring_shutter_shadow(2)),
                   ("ring_shutter_dust_a", lambda: ring_shutter_dust("a")), ("ring_shutter_dust_b", lambda: ring_shutter_dust("b")),
                   ("ring_shutter_dust_c", lambda: ring_shutter_dust("c"))],
    },
    "floor01_ring_camp": {
        "stem": "ring_camp", "width": 256,
        "pieces": [("ring_camp_a", ring_camp), ("ring_crates_b", ring_crates_b), ("ring_marker_a", ring_marker_a),
                   ("ring_fallen_voussoir_a", lambda: ring_fallen_voussoir("a")),
                   ("ring_fallen_voussoir_b", lambda: ring_fallen_voussoir("b"))],
    },
    "floor01_ring_parallax": {
        "stem": "ring_parallax", "width": 448,
        "pieces": [("ring_tiers_far", ring_tiers_far), ("ring_tiers_mid", ring_tiers_mid), ("ring_camp_far", ring_camp_far)],
    },
}
KEY_IDS = ("ring_arc_wall_a", "ring_shutter_closed", "ring_camp_a")
KEY_FNS = {"ring_arc_wall_a": ring_arc_wall, "ring_shutter_closed": ring_shutter_closed, "ring_camp_a": ring_camp}


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


def build_pieces(name: str) -> list[tuple[str, Pix]]:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def build(name: str):
    pieces = build_pieces(name)
    sheet_img, rects = pack(pieces, ATLASES[name]["width"])
    return kit.grid_from([sheet_img], layer_names=["main"], durations=[100], pivot=(0, 0)), rects, dict(pieces)


# ======================================================================================== contract
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_rings_kit.json"
MANIFESTS = ROOT / "assets/manifests"
EXTENDS = "assets/source/terrain/floor01/floor01_opening_kit.json"
BASE_CATACOMB = ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json"

# piece id -> (role, layer)
WALL_PIECES = ("ring_arc_wall_a", "ring_arc_wall_b", "ring_relief_landmark", "ring_arc_upper_a", "ring_arc_upper_b",
               "ring_shield_notched", "ring_breach_r3")
# ring_keystone_crowned is a prop (RNG-AD21): it is drawn after door_timber, over the door's apex stone
STRUCTURE_PIECES = ("ring_soffit", "ring_ledge_mid", "ring_ledge_end_l", "ring_ledge_end_r", "ring_shutter_jamb_l",
                    "ring_shutter_jamb_r")
ROLES = {pid: ("backwall", "backwall_decal") for pid in WALL_PIECES}
ROLES.update({pid: ("structure", "structure") for pid in STRUCTURE_PIECES})
ROLES.update({f"ring_shutter_dust_{v}": ("foreground", "foreground") for v in "abc"})
ROLES["ring_tiers_far"] = ("parallax_far", "parallax_far")
ROLES["ring_camp_far"] = ("parallax_far", "parallax_far")
ROLES["ring_tiers_mid"] = ("parallax_mid", "parallax_mid")
FB, W_LINE = 560, 580
OWNER = ("AI-authored environment pixel art (AREA-RINGS-01: design spec and keys Claude Opus; gate verdict "
         "Claude Opus; execution Claude Sonnet; independent review pending)")
STATES = {"ring_shutter_closed": "closed"}

# Spec section 4/5 placements (room layout px, top-left). Ledge fronts are expanded per platform rect.
PLATFORMS = {
    "ring_court": [(1056, 516, 64), (1120, 500, 192), (1536, 452, 96)],
    "ring_split": [(336, 516, 384), (896, 516, 96)],
}
ROOM_W = {"ring_court": 2048, "ring_split": 1152}


def ledge_places(room: str) -> list[tuple]:
    out = []
    for x, y, w in PLATFORMS[room]:
        out += [("structure", "ring_ledge_mid", x + 32 * n, y) for n in range(w // 32)]
        out += [("structure", "ring_ledge_end_l", x, y), ("structure", "ring_ledge_end_r", x + w - 8, y)]
    return out


def soffit_places(room: str) -> list[tuple]:
    return [("structure", "ring_soffit", 32 * n, 336) for n in range(ROOM_W[room] // 32)]


def upper_places(room: str) -> list[tuple]:
    return [("backwall_decal", "ring_arc_upper_" + "ab"[n % 2], 256 * n, 292) for n in range(ROOM_W[room] // 256)]


# RNG-AD20: the court's ground bay at x 1680 is suppressed (both piers, the arch ring, threshold and chain: a lone
# pier would carry half an arch springing into nothing) and its opening bricked up, so the full ring sits on brick. The
# bay's left torch moves beside the ring and the free torch off the great arc's right leg; both keep their animation
# phase, so the emitter count is unchanged.
# RNG-AD24: final torch spots. The bay torch stands at (1656, 448), 10..19 px left of the ring's edge: at 1648 its
# pool's warm core lit the brick under the Donut shelf brighter than the shelf's top arris (C1). The free torch hangs
# high over the broken right leg at (1360, 368), 16 px under the soffit: at (1432, 432) its pool core sat right behind
# the goblin at x 1424 (dL* -1.4 against 2.1 on plain brick).
# RNG-AD21: the court's entry bay opening (x 80..288) is bricked up behind the door; the keystone is the door's
# apex stone. 4b keeps its bay and has no keystone.
def _at(layer, piece, x, y, frame_offset=None):
    d = {"layer": layer, "piece": piece, "x": x, "y": y}
    if frame_offset is not None:
        d["frame_offset"] = frame_offset
    return d


COURT_REMOVE_AT = [
    _at("structure", "pier_l", 1680, 304), _at("structure", "arch_ring_a", 1696, 304),
    _at("structure", "pier_r", 1904, 304), _at("structure", "arch_threshold", 1744, 544),
    _at("structure", "arch_chain", 1760, 384),
    _at("structure", "arch_threshold", 112, 544),           # RNG-AD21: the blind entry arch has no open sill
    _at("prop", "torch_wall", 1696, 448), _at("backwall_decal", "torch_pool_b", 1592, 364),
    _at("prop", "torch_wall", 1344, 432), _at("backwall_decal", "torch_pool_b_m", 1240, 348),
]
COURT_TORCHES = [
    _at("backwall_decal", "torch_pool_b", 1552, 364), _at("prop", "torch_wall", 1656, 448, 3),
    _at("backwall_decal", "torch_pool_b_m", 1256, 284), _at("prop", "torch_wall", 1360, 368, 1),
]
# the two bricked-up ground openings (the composer's OPENING mask lies inside cols c0+2..c0+14, rows 22..35)
COURT_FILL = [{"x": 80, "y": 352, "w": 208, "h": 208}, {"x": 1712, "y": 352, "w": 208, "h": 208}]
KEYSTONE_AT = (228, 466)                 # bottom rows on door_timber's apex stone (layout x 235..244, y 490..495)


def dressing() -> dict:
    court = (
        [("backwall_decal", "ring_arc_wall_a", *ARC_A_AT),
         ("backwall_decal", "ring_relief_landmark", *ARC_LM_AT), ("backwall_decal", "ring_shield_notched", 1256, 524)]
        + upper_places("ring_court") + soffit_places("ring_court") + ledge_places("ring_court")
        + [("structure", "ring_shutter_jamb_l", 1192, 512), ("structure", "ring_shutter_jamb_r", 1232, 512),
           ("prop", "ring_shutter_housing", 1192, 512),
           ("prop", "ring_fallen_voussoir_a", 1328, FB - 16), ("prop", "ring_fallen_voussoir_b", 1358, FB - 14),
           ("prop", "ring_keystone_crowned", *KEYSTONE_AT)])
    split = (
        [("backwall_decal", "ring_arc_wall_b", *ARC_B_AT), ("backwall_decal", "ring_breach_r3", 896, 380)]
        + upper_places("ring_split") + soffit_places("ring_split") + ledge_places("ring_split")
        + [("prop", "ring_camp_a", 744, 488), ("prop", "ring_crates_b", 992, FB - 28),
           ("prop", "ring_marker_a", 1048, 504)])
    return {
        "ring_court": {"remove": [], "remove_at": COURT_REMOVE_AT, "backwall_fill": COURT_FILL,
                       "place": COURT_TORCHES + [_at(*t) for t in court],
                       "parallax": {"parallax_far": "ring_tiers_far", "parallax_mid": "ring_tiers_mid"}},
        "ring_split": {"remove": [], "place": [_at(*t) for t in split],
                       "parallax": {"parallax_far": "ring_camp_far", "parallax_mid": "ring_tiers_mid"}},
    }


def base_lights() -> dict:
    base = json.loads(BASE_CATACOMB.read_text(encoding="utf-8"))
    return {p["id"]: p["lights"] for p in base["pieces"] if p["id"] in ("gallery_far", "gallery_mid")}


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = ROLES.get(pid, ("prop", "prop"))
    parallax = role.startswith("parallax")
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0],
             "anchor_name": "top_left (strip origin)" if parallax else "top_left (room placement, spec section 5)",
             "grid_snap": 1, "grammar": ["B"] if parallax else ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    if parallax:
        entry["tile_x"] = True
        lights = base_lights()["gallery_mid" if role == "parallax_mid" else "gallery_far"]
        entry["lights"] = lights                      # the same torch offsets: the emitter count is unchanged
    if pid in STATES:
        entry["state_of"] = {"group": "ring_shutter", "state": STATES[pid]}
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
        "kit_id": "floor01_rings",
        "extends": EXTENDS,
        "status": "proposed - AREA-RINGS-01 kit (docs/02-art/area-rings-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema "
                          "from the ENV-01 contract through the opening kit. Adds five decor atlases and the Broken "
                          "Ring pieces for rooms 4a and 4b; no tiles, nothing collidable, no new palette colour and "
                          "no new light emitter (the parallax strips carry the opening kit's torch offsets).",
        "palette_use": palette_use,
        "atlases": atlases,
        "tiles": [],
        "pieces": pieces,
        "state_groups": {"ring_shutter": {"states": ["closed"],
                                          "note": "drawn by src/ring_shutter_art.rs and the scene's env_prop_art from "
                                                  "Shutter::phase()/edge_px() (presentation only): ring_shutter_closed "
                                                  "cropped to its bottom edge_px rows, plus the grit, shadow and dust "
                                                  "extras of spec section 6"}},
        "room_dressing": dressing(),
        "usage_rules": {
            "R1_rings": "rooms 4a ring_court and 4b ring_split only: arcs, the full ring and the breach are "
                        "backwall_decal over the brick, props stand on the floor back line (y 560), the soffit tiles "
                        "lie exactly over the ceiling slab (y 336..352), ledge fronts lie on the platform rects, "
                        "the shutter pieces are drawn by the presenter, never placed. remove_at drops exact "
                        "composer instances (each matches one piece) and backwall_fill bricks up a rect with the "
                        "composer's own brick tile ids (RNG-AD20, RNG-AD21); neither touches the collision layer.",
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
        "kit_contract": "assets/source/terrain/floor01/floor01_rings_kit.json",
        "owner": OWNER,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 Broken Ring pieces for Floor 1 rooms 4a and 4b (AREA-RINGS-01 on the "
                      "ENV-01 kit); rule-based shapes and ramps, nothing cropped, traced or resampled from concept "
                      "art; no text, logos or likenesses",
            "generator": "Deterministic authoring script scripts/author_rings01.py written by AI agents (keys, spec "
                         "and gate verdict designed by Claude Opus; execution by Claude Sonnet); no image-model "
                         "raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1/08-circular-maze-survivor-camp (mood reference only; no pixel cropped, "
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


# ======================================================================================== output
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


def lstar(hex_colour: str) -> float:
    """CIE L* of a #rrggbb colour."""
    lin = []
    for i in (1, 3, 5):
        c = int(hex_colour[i:i + 2], 16) / 255.0
        lin.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    y = 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]
    return 116.0 * y ** (1.0 / 3.0) - 16.0 if y > 0.008856 else 903.3 * y


def mean_lstar(img: Pix) -> float:
    vals = [lstar(RAMPS[p[0]][p[1]]) for row in img.px for p in row if p]
    return sum(vals) / len(vals)


def stone_share(img: Pix) -> dict:
    """The share of the opaque pixels on each ``stone`` index (gate K1 target: 7 and 8 absent, 6 at most 3 %)."""
    n = sum(1 for row in img.px for p in row if p)
    out = {}
    for row in img.px:
        for p in row:
            if p and p[0] == "stone":
                out[p[1]] = out.get(p[1], 0) + 1
    return {k: round(100.0 * v / n, 2) for k, v in sorted(out.items())}


def shutter_frame(edge_px: int):
    """The presenter rule of spec section 6: the visible slab is the key's bottom ``edge_px`` rows."""
    from PIL import Image
    key = to_image(ring_shutter_closed())
    frame = Image.new("RGBA", key.size, (0, 0, 0, 0))
    if edge_px > 0:
        frame.paste(key.crop((0, SH_H - edge_px, SH_W, SH_H)), (0, 0))
    return frame


def gate(out_dir: Path) -> None:
    """Gate sheets: the keys at 1x and 4x, their silhouettes, the shutter travel strip, the keys composed at 1x
    into the native-scale court still (docs/evidence/f01-ring) and the measured gate numbers (K1, K2, K4)."""
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    keys = [KEY_FNS[k]() for k in KEY_IDS]
    bg = (205, 198, 186, 255)
    pad = 8
    for silhouette, name in ((False, "keys"), (True, "silhouettes")):
        imgs = [to_image(k, silhouette) for k in keys]
        w = sum(i.width for i in imgs) + pad * (len(imgs) + 1)
        h = max(i.height for i in imgs) + pad * 2
        sheet = Image.new("RGBA", (w, h), bg if silhouette else (40, 36, 48, 255))
        x = pad
        for i in imgs:
            sheet.alpha_composite(i, (x, h - pad - i.height))
            x += i.width + pad
        sheet.save(out_dir / f"{name}_1x.png")
        sheet.resize((w * 4, h * 4), Image.NEAREST).save(out_dir / f"{name}_4x.png")
    # shutter travel: edge_px 0, 17, 34, 51, 68 (closing reads left to right, opening right to left)
    steps = (0, 17, 34, 51, 68)
    strip = Image.new("RGBA", (len(steps) * (SH_W + pad) + pad, SH_H + 2 * pad), (58, 34, 59, 255))
    for k, e in enumerate(steps):
        strip.alpha_composite(shutter_frame(e), (pad + k * (SH_W + pad), pad))
    strip.resize((strip.width * 4, strip.height * 4), Image.NEAREST).save(out_dir / "shutter_travel_4x.png")
    # in-scene composite at gameplay scale over the existing court still (view x 850, y 292)
    still = Image.open(ROOT / "docs/evidence/f01-ring/still-court-shutter-tick420.png").convert("RGBA")
    still = still.resize((640, 360), Image.NEAREST)
    comp = still.copy()
    vx, vy = 850, 292
    w_line = 580
    comp.alpha_composite(to_image(keys[0]), (1012 - vx, w_line - 16 - ARC_H - vy))
    # restore what draws in front of a backwall decal in the still: the step and walkway ledges, the actors
    # and the floor deck
    front = ((200, 200, 470, 228), (280, 228, 350, 292), (540, 246, 590, 292))
    for box in front + ((0, 270, 640, 360),):
        comp.paste(still.crop(box), box[:2])
    comp.alpha_composite(to_image(keys[1]), (1200 - vx, 512 - vy))     # replaces the grey stand-in
    # the camp is 4b dressing: staged here on the same deck only to judge it at gameplay scale
    comp.alpha_composite(to_image(keys[2]), (1320 - vx, w_line - 20 - CAMP_H - vy))
    for box in front[1:]:                                              # actors draw over props
        comp.paste(still.crop(box), box[:2])
    comp.save(out_dir / "scene_court_1x.png")
    comp.resize((1280, 720), Image.NEAREST).save(out_dir / "scene_court_2x.png")
    still.save(out_dir / "scene_court_before_1x.png")
    # measured gate numbers (palette L* of the opaque pixels; the brick is the still's own lit brick)
    arc, shutter = keys[0], keys[1]
    def brick_mean(box):
        px = [still.getpixel((x, y))[:3] for y in range(box[1], box[3]) for x in range(box[0], box[2])]
        return sum(lstar("#%02x%02x%02x" % p) for p in px) / len(px)
    sx, sy = 1200 - vx, 512 - vy
    beside = (brick_mean((sx - 36, sy + 6, sx - 4, sy + 50)) + brick_mean((sx + SH_W + 4, sy + 6, sx + SH_W + 36, sy + 50))) / 2
    legs = mean_lstar(Pix_rows(arc, ARC_H - 60, ARC_H))
    crown = mean_lstar(Pix_rows(arc, 0, 50))
    lines = [
        f"arc_a opaque mean L* (all pixels): {mean_lstar(arc):.1f}  [gate target <= 31; before K1: 35 crown, 39 legs]",
        f"arc_a crown rows 0..49 mean L*: {crown:.1f}; leg rows {ARC_H - 60}..{ARC_H - 1} mean L*: {legs:.1f}",
        f"arc_a stone share of opaque pixels (%): {stone_share(arc)}  [target: 7 and 8 absent, 6 <= 3]",
        f"shutter key opaque mean L* (the gate's measure; before K2: 24.8): {mean_lstar(shutter):.1f}  [gate target <= 19]",
        f"shutter slab stone only (rows 3..59) mean L*: {slab_body_lstar(shutter):.1f}",
        f"court brick beside the shutter (lit still) mean L*: {beside:.1f}  => key {beside - mean_lstar(shutter):.1f} "
        f"below the brick, slab stone {beside - slab_body_lstar(shutter):.1f} below  [gate target >= 8]",
    ]
    (out_dir / "measurements.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


def Pix_rows(img: Pix, y0: int, y1: int) -> Pix:
    out = Pix(img.w, y1 - y0)
    out.px = [row[:] for row in img.px[y0:y1]]
    return out


def slab_body_lstar(shutter: Pix) -> float:
    """Mean L* of the slab stone (the ``stone`` pixels of rows 3..59): the face the gate K2 target speaks of."""
    vals = [lstar(RAMPS[p[0]][p[1]]) for row in shutter.px[3:60] for p in row if p and p[0] == "stone"]
    return sum(vals) / len(vals)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "gate":
        gate(Path(sys.argv[2]))
    else:
        write()
