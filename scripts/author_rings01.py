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
# Hero key: ring_arc_wall_a (spec section 11). 408 x 164, backwall_decal. The upper arc of one of the
# great ring's carved courses; its centre sits 48 px below the piece bottom, behind the floor, so the
# arc dives into the floor at both ends and never closes into an arch opening.
# =============================================================================================
ARC_W, ARC_H = 408, 164
ARC_CX, ARC_CY = 204.0, 212.0
# (r_in, r_out) of the three bands and the two grooves, outermost first
OUTER = (184.0, 208.0)     # voussoir band, radial joints every 7.5 degrees (keystone wider)
GROOVE_A = (181.0, 184.0)
NOTCH = (167.0, 181.0)     # plain band carved with radial notches every 5 degrees
GROOVE_B = (164.0, 167.0)
INNER = (148.0, 164.0)     # voussoir band, joints every 9 degrees, offset half a block
SHADOW = (143.0, 148.0)    # cast shadow of the relief on the brick below the inner band
# the off-axis break, degrees from the apex (+ = right): outer, notch and inner bands lose different
# spans so the broken end steps like fractured masonry, never a clean cut
BREAK = {"outer": (31.0, 51.0), "notch": (33.5, 48.0), "inner": (35.0, 46.5)}
CHAINS = (-21.0, 17.5)     # chain stubs hang from the inner band at these angles
LIGHT = (-0.55, -0.83)     # key light from the upper left (unit vector, y down)


def _band_value(r, a_deg, band, joint_step, joint_phase, base):
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
    v += (rnd(band, block) - 0.5) * 0.9
    v += 0.35 * (1.0 - abs(fk - 0.5) * 2.0) - 0.15
    v -= 0.35 * abs(t - 0.55)
    return v, block, fk


def ring_arc_wall() -> Pix:
    c = Pix(ARC_W, ARC_H)
    for y in range(ARC_H):
        for x in range(ARC_W):
            dx, dy = x + 0.5 - ARC_CX, y + 0.5 - ARC_CY
            r = math.hypot(dx, dy)
            if r < SHADOW[0] or r >= OUTER[1] + 4:
                continue
            a_deg = math.degrees(math.atan2(dx, -dy))           # 0 at the apex, + to the right
            # height falloff: the top of the arc sits in the soffit's shadow, the lower legs in torch light
            lift = -0.9 * (1.0 - y / ARC_H) + 0.3 * (y / ARC_H)
            if OUTER[0] <= r < OUTER[1] + 4:
                lo, hi = BREAK["outer"]
                key = abs(a_deg) < 4.2                            # the keystone stands 4 px proud
                if r >= OUTER[1] and not key:
                    continue
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(min(r, OUTER[1] - 0.01), a_deg, OUTER, 7.5, 3.75, 5.6 + lift)
                if key:
                    v = 6.2 + lift + (0.9 if r >= OUTER[1] - 1 else 0) - (1.2 if abs(a_deg) > 3.6 else 0)
                elif fk < 0.5 / (math.radians(7.5) * r) * 1.2:   # 1 px radial mortar joint
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 8.4), "o")
            elif GROOVE_A[0] <= r < GROOVE_A[1] or GROOVE_B[0] <= r < GROOVE_B[1]:
                lo, hi = BREAK["notch"]
                if lo - 1 <= a_deg <= hi + 1:
                    continue
                inner_lip = (r - GROOVE_A[0] < 1.0) if r >= GROOVE_A[0] else (r - GROOVE_B[0] < 1.0)
                c.put(x, y, "mortar", 4 if inner_lip and a_deg < 0 else 1)
            elif NOTCH[0] <= r < NOTCH[1]:
                lo, hi = BREAK["notch"]
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(r, a_deg, NOTCH, 5.0, 0.0, 4.7 + lift)
                # a notch: a 4 px deep recess cut into the outer two thirds of the band
                if 0.18 < fk < 0.52 and r > NOTCH[0] + 4:
                    left_wall = fk < 0.26
                    c.put(x, y, "mortar", 3 if left_wall and a_deg < 0 else 2)
                    continue
                c.gput(x, y, "stone", clamp(v - 0.3, 1.0, 7.6), "n")
            elif INNER[0] <= r < INNER[1]:
                lo, hi = BREAK["inner"]
                if lo <= a_deg <= hi:
                    continue
                v, block, fk = _band_value(r, a_deg, INNER, 9.0, 4.5, 5.3 + lift)
                if fk < 0.5 / (math.radians(9.0) * r) * 1.2:
                    c.put(x, y, "mortar", 2)
                    continue
                c.gput(x, y, "stone", clamp(v, 1.0, 8.2), "i")
            elif SHADOW[0] <= r < SHADOW[1]:
                lo, hi = BREAK["inner"]
                if lo <= a_deg <= hi:
                    continue
                # the relief's shadow on the brick: solid near the band, checker toward the brick
                depth = SHADOW[1] - r
                if depth < 2.0 or (depth < 4.0 and (x + y) % 2 == 0) or (depth < 5.0 and (x + y) % 4 == 0):
                    c.put(x, y, "mortar", 1)
    _arc_break(c)
    _arc_weather(c)
    _arc_chains(c)
    return c


def _polar(a_deg, r):
    a = math.radians(a_deg)
    return ARC_CX + r * math.sin(a), ARC_CY - r * math.cos(a)


def _arc_break(c: Pix) -> None:
    """The missing segment: raw fracture faces on the surviving block ends and the scarred bed behind."""
    for name, band in (("outer", OUTER), ("notch", NOTCH), ("inner", INNER)):
        lo, hi = BREAK[name]
        for r10 in range(int(band[0] * 10), int(band[1] * 10)):
            r = r10 / 10.0
            # jagged fracture: the edge wanders by up to 2 degrees with radius
            jag = (rnd("jag", name, int(r / 3)) - 0.5) * 3.2
            for edge, side in ((lo + jag, -1), (hi + jag * 0.6, +1)):
                for k in range(3):
                    x, y = _polar(edge + side * (k * 0.5 / max(r, 1) * 57.3), r)
                    ix, iy = int(x), int(y)
                    if c.get(ix, iy) is None:
                        continue
                    # fresh fracture: pale on the face lit by the key light (the left-hand end), dark on the other
                    c.put(ix, iy, "stone", (7 - k) if side < 0 else (3 - k))
    # the bed the voussoirs were set in: a narrow rough rim of old mortar just past each broken end
    # (2..4 px, ragged); the rest of the gap is transparent so the brick behind shows through
    for y in range(c.h):
        for x in range(c.w):
            if c.get(x, y) is not None:
                continue
            dx, dy = x + 0.5 - ARC_CX, y + 0.5 - ARC_CY
            r = math.hypot(dx, dy)
            if not (INNER[0] <= r < OUTER[1]):
                continue
            a_deg = math.degrees(math.atan2(dx, -dy))
            px_from = min(abs(a_deg - lo) for lo in (BREAK["outer"][0], BREAK["outer"][1])) * r / 57.3
            if BREAK["outer"][0] < a_deg < BREAK["outer"][1] and px_from < 2.0 + 2.0 * rnd("rim", int(r / 2)):
                c.put(x, y, "mortar", 2 if rnd("rim2", x, y) > 0.3 else 3)


def _arc_weather(c: Pix) -> None:
    """Hairline cracks, chipped arrises and a little damp lichen on the lower legs (dry floor, old damp)."""
    for i, (a0, r0, length, turn) in enumerate(((-58.0, 206.0, 13, 0.9), (-12.0, 190.0, 9, -0.6),
                                                 (22.0, 160.0, 8, 0.7), (-40.0, 176.0, 7, 0.4),
                                                 (61.0, 196.0, 11, -0.8), (-71.0, 158.0, 8, 0.5))):
        a, r = a0, r0
        for s in range(length):
            x, y = _polar(a, r)
            if c.get(int(x), int(y)) and c.get(int(x), int(y))[0] == "stone":
                c.put(int(x), int(y), "mortar", 2)
            r -= 1.0
            a += turn * (0.5 if rnd("ck", i, s) > 0.5 else 0.15)
    # chipped arrises: a few outer-edge pixels lost along the band (transparent: the brick shows)
    for a10 in range(-760, 760, 7):
        a = a10 / 10.0
        if rnd("chip", a10) > 0.86:
            x, y = _polar(a, OUTER[1] - 0.5)
            c.px[int(y)][int(x)] = None if 0 <= int(y) < c.h and 0 <= int(x) < c.w else None
    # lichen: only near the floor, on the lower legs, below the torch line
    for y in range(c.h - 34, c.h):
        for x in range(c.w):
            here = c.get(x, y)
            if here and here[0] == "stone" and rnd("li", x // 2, y // 2) > 0.95 and rnd("li2", x, y) > 0.35:
                c.put(x, y, "lichen", 1 if rnd("li3", x, y) > 0.5 else 0)


def _arc_chains(c: Pix) -> None:
    """Two weathered chain stubs hang from iron staples in the inner band (oval links, 5 x 3 px)."""
    for n, a in enumerate(CHAINS):
        x0, y0 = _polar(a, INNER[0] + 3)
        x0, y0 = int(x0), int(y0)
        c.box(x0 - 1, y0 - 2, x0 + 1, y0, "iron", 2)                  # the staple
        c.put(x0, y0 - 2, "iron", 5)
        links = 5 if n == 0 else 3                                     # the right one broke shorter
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
        c.put(x0 + (1 if n == 0 else -1), y, "iron", 2)                # the torn last link


# =============================================================================================
# ring_shutter_closed (spec section 6). 32 x 68, prop. A dressed stone slab on iron straps with an
# iron-shod toothed lower edge; the moving frames are row crops of this key keyed off Shutter::edge_px.
# =============================================================================================
SH_W, SH_H = 32, 68
SH_COURSES = (0, 17, 34, 51)     # four 17 px courses: one course per 17 px of travel, slices stay 4 px


def ring_shutter_closed() -> Pix:
    c = Pix(SH_W, SH_H)
    for y in range(SH_H):
        for x in range(SH_W):
            course = max(i for i, top in enumerate(SH_COURSES) if top <= y)
            cy = y - SH_COURSES[course]
            # slab face: darker and cooler than the ring relief so the moving mass separates from the wall
            v = 3.7 + 0.5 * (y / SH_H) + (rnd("shc", course) - 0.5) * 0.6
            v += (rnd("shn", x // 3, y // 2) - 0.5) * 0.5
            if cy == 0:
                v = 2.0                                                # course joint (dark)
            elif cy == 1:
                v += 1.4                                               # lit top arris of each course
            elif cy == 16:
                v -= 0.9
            if x == 1:
                v += 1.2                                               # lit left arris
            if x == SH_W - 2:
                v -= 1.1
            c.gput(x, y, "stone", clamp(v, 1.0, 7.0), "d1")
        c.put(0, y, "mortar", 1)                                       # hard outline: reads against brick
        c.put(SH_W - 1, y, "mortar", 0)
    # top: the slot shadow under the walkway
    for y in range(3):
        for x in range(SH_W):
            c.put(x, y, "mortar", y)
    # two iron straps with rivets
    for sx in (6, 23):
        for y in range(3, SH_H - 8):
            c.put(sx, y, "iron", 4)
            c.put(sx + 1, y, "iron", 3)
            c.put(sx + 2, y, "iron", 1)
        for ry in (5, 22, 39, 56):
            c.put(sx + 1, ry, "iron", 6)
            c.put(sx + 1, ry + 1, "iron", 2)
    # a small carved broken ring roundel in the second course (the court's mark, no glyph)
    rcx, rcy = 15.5, 25.5
    for y in range(18, 34):
        for x in range(10, 22):
            d = math.hypot(x + 0.5 - rcx - 0.5, y + 0.5 - rcy - 0.5)
            a = math.degrees(math.atan2(x + 0.5 - rcx - 0.5, -(y + 0.5 - rcy - 0.5)))
            if 3.0 <= d < 5.6 and not (30 <= a <= 62):
                c.put(x, y, "mortar", 3 if (x + 0.5 - rcx) < 0 else 2)
                if d < 3.8 and a > 90:
                    c.put(x, y, "stone", 6)                            # lit lower lip of the groove
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
    # a rolled blanket on its side under the lean-to: an ellipse of dull green with two leather ties
    cx, cy, rx, ry = 31.0, 64.0, 15.0, 6.6
    for y in range(int(cy - ry) - 1, CAMP_H):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            e = ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2
            if e <= 1.0:
                v = 1.6 - 1.2 * (y - cy) / ry - (0.8 if e > 0.75 else 0)
                c.gput(x, y, "lichen", clamp(v, 0.0, 2.0), "d3")
    for tx in (22, 39):
        for y in range(int(cy - ry), CAMP_H - 1):
            c.put(tx, y, "wood", 2)
    # the rolled end: a spiral seen end-on at the left
    for k, (dx, dy) in enumerate(((0, 0), (1, -1), (2, 0), (1, 1), (0, 2), (-1, 1))):
        c.put(18 + dx, 63 + dy, "lichen", 0 if k % 2 else 2)


def _crates(c: Pix) -> None:
    def crate(x0, y0, w, h, seed):
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


def _lantern(c: Pix) -> None:
    # the camp lantern on the top crate: brass frame, horn panes glowing warm, a wire bail
    x0, y0 = 84, 10
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
            if c.get(x, y) is None and (x + y) % 2 == 0 and math.hypot(x - (x0 + 3), y - (y0 + 8)) < 6.5:
                c.put(x, y, "gal_warm", 3)
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


# ======================================================================================== output
KEYS = {
    "ring_wall": [("ring_arc_wall_a", ring_arc_wall)],
    "ring_shutter": [("ring_shutter_closed", ring_shutter_closed)],
    "ring_camp": [("ring_camp_a", ring_camp)],
}


def write() -> None:
    for stem, pieces in KEYS.items():
        (pid, fn), = pieces
        img = fn()
        grid = kit.grid_from([img], layer_names=["main"], durations=[100], pivot=(0, 0))
        path = SRC / f"{stem}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{stem}.ase")], check=True)
        print(f"{stem}: {pid} {grid['canvas']} {len(grid['palette'])} colours")


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


def shutter_frame(edge_px: int):
    """The presenter rule of spec section 6: the visible slab is the key's bottom ``edge_px`` rows."""
    from PIL import Image
    key = to_image(ring_shutter_closed())
    frame = Image.new("RGBA", key.size, (0, 0, 0, 0))
    if edge_px > 0:
        frame.paste(key.crop((0, SH_H - edge_px, SH_W, SH_H)), (0, 0))
    return frame


def gate(out_dir: Path) -> None:
    """Silhouette gate sheets: the keys at 1x and 4x, their silhouettes, the shutter travel strip and the
    keys composed at 1x into the native-scale court still (docs/evidence/f01-ring)."""
    from PIL import Image
    out_dir.mkdir(parents=True, exist_ok=True)
    keys = [fn() for pieces in KEYS.values() for _, fn in pieces]
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


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "gate":
        gate(Path(sys.argv[2]))
    else:
        write()
