#!/usr/bin/env python3
"""DOZ-02 authoring record: Dozer damage states and the stop frame (design keys).

Usage:

    python3 scripts/author_dozer_doz02.py [--check] [--out-dir DIR]

Writes (deterministic, pixel-grid-v1, palette ``dozer-floor01-v0``) under ``assets/source/enemies/dozer/`` and
``assets/source/props/stop_frame/``, plus one manifest per grid under ``assets/manifests/`` and the damage-state
data contract ``dozer_damage_h120.json``:

- ``dozer_wheel_jammed_h120``: the jammed wheel (item 1): stopped, the frame's tie bar wedged across the upper
  spikes, the top spike and the 30 degree spike bent, the 150 degree spike snapped, every glint dulled.
- ``dozer_hull_damage_h120``: the hull body for ``impact_stunned`` and ``disabled`` (items 2 and 3): ``pitch_0``
  (front down 4, rear up 2), ``pitch_1`` (half) and ``disabled`` (level). All three carry the damage overlays:
  the cab dent, the boiler crack (steam-leak emitter), the stack knocked crooked with the bucket gone and the
  firebox dimmed to embers.
- ``dozer_front_cab_damage_h120`` and ``dozer_front_tub_damage_h120``: the hull front boxes at the same three
  states; the tub lip is crumpled where the frame struck it.
- ``dozer_yoke_damage_h120``: the bent yoke with the lamp glass broken (no fire colours).
- ``prop_stop_frame``: the hanging press frame (item 4): ``raised`` (beam, tie bar, chains, spreader) and
  ``pulley`` (the ceiling block and the haul-chain stub; drawn at the raised position in every state).
- ``prop_stop_frame_remnant``: the landed, bent remnant, 80 x 16: ``landed`` (a miss: the tie bar lies in
  front) and ``landed_hit`` (the tie bar is in the wheel).
- ``prop_stop_frame_brace``: the brace cleat on the back wall, ``intact``.

The DOZ-01 grids, manifests, rig contract and frozen keys are not touched: every DOZ-02 part is a new atlas whose
pivot sits on an existing DOZ-01 anchor (or on the room's stop-frame rectangles), so the DOZ-01 atlases stay
byte-identical.

How the pixels are made: the DOZ-01 construction (``author_dozer_doz01.py``) is rebuilt with the damaged parts
swapped in, then detailed with explicit pixel work. The pitch is a 1 px stair-step **column translation** (each
column moves up or down by a whole number of pixels: no rotation, no resampling, no new colours). The bent yoke
and the bent remnant use the same column translation with a kinked profile. The stop frame and the brace are
new constructions with the DOZ-01 ``Raster`` (parts, bevel, plum outline) and an explicit chain pattern.
No image-model output, tracing or resampling. Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import author_dozer_doz01 as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

ROOT = ad.ROOT
DOZER_DIR = ad.DOZER_DIR
PROP_DIR = ROOT / "assets/source/props/stop_frame"
MANIFEST_DIR = ad.MANIFEST_DIR

IRON, RUST, COPPER, BRASS = ad.IRON, ad.RUST, ad.COPPER, ad.BRASS
FIRE = set("jknqz")

# ------------------------------------------------------------------ pitch (item 2)
# Rig x of the hull's rear and front ends (the push box, DOZ-01 extents) and the pitch at full amplitude.
PITCH_REAR_X, PITCH_FRONT_X = -84, 83
PITCH_REAR_UP, PITCH_FRONT_DOWN = 2, 4
PITCH_AMPS = {"pitch_0": 1.0, "pitch_1": 0.5, "disabled": 0.0}


def rnd(v: float) -> int:
    """Round half away from zero (deterministic for the pitch table)."""
    return int(math.floor(abs(v) + 0.5)) * (1 if v >= 0 else -1)


def pitch_dy(x: float, amp: float) -> int:
    """Pixels up (negative: down) of the hull column at rig x for a pitch amplitude (1 = impact key)."""
    span = PITCH_FRONT_X - PITCH_REAR_X
    v = PITCH_REAR_UP - (PITCH_REAR_UP + PITCH_FRONT_DOWN) * (x - PITCH_REAR_X) / span
    return rnd(v * amp)


def shift_columns(rows: list[list[str]], shift_of_col) -> list[list[str]]:
    """Translate every column by a whole number of pixels (``shift_of_col(c)`` px up; negative = down)."""
    h, w = len(rows), len(rows[0])
    out = [["."] * w for _ in range(h)]
    for c in range(w):
        s = shift_of_col(c)
        for r in range(h):
            if rows[r][c] != ".":
                nr = r - s
                if not 0 <= nr < h:
                    raise ValueError(f"column {c} leaves the canvas at shift {s}")
                out[nr][c] = rows[r][c]
    return out


def pad(rows: list[list[str]], top: int, bottom: int) -> list[list[str]]:
    w = len(rows[0])
    return [["."] * w for _ in range(top)] + [r[:] for r in rows] + [["."] * w for _ in range(bottom)]


def outline(rows: list[list[str]]) -> list[list[str]]:
    """Add the plum outline on transparent pixels next to a non-outline pixel (4-neighbour)."""
    h, w = len(rows), len(rows[0])
    out = [r[:] for r in rows]
    for y in range(h):
        for x in range(w):
            if rows[y][x] == "." and any(0 <= x + dx < w and 0 <= y + dy < h and rows[y + dy][x + dx] not in ".K"
                                         for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                out[y][x] = "K"
    return out


def line_pts(p0, p1) -> list[tuple[int, int]]:
    """Integer pixel line (Bresenham) between two pixel positions."""
    (x0, y0), (x1, y1) = p0, p1
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    pts = []
    while True:
        pts.append((x0, y0))
        if (x0, y0) == (x1, y1):
            return pts
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


# ------------------------------------------------------------------ hull body damage (item 3)

def hull_damaged() -> list[list[str]]:
    """The hull body with the disabled overlays, level (the DOZ-01 canvas and pivot)."""
    w, h, ox, fy = ad.hull_canvas(1.0)
    rs = ad.Raster(w, h, ox, fy, 1.0)
    ids = ad.build_hull(rs, "body")
    # the stack is knocked crooked at its collar and the bucket cap is gone
    drop = {ids["stack"][1], ids["bucket"][0]}
    for r in range(h):
        for c in range(w):
            if rs.pid[r][c] in drop:
                rs.pid[r][c] = None
    lean = rs.poly([(84, 133), (92, 133), (87.5, 144), (78.5, 142.5)], RUST, level=2, outline=True)
    rs.render()
    ids = dict(ids)
    ids["stack"] = [ids["stack"][0], ids["stack"][2], lean]
    ids["bucket"] = []
    ad.detail_hull(rs, "body", ids)
    # torn stack mouth: soot band and two bites out of the rim
    lean_set = {lean}
    for x in range(79, 89):
        rs.put(x + 0.5, 141.5, "Q", lean_set)
        rs.put(x + 0.5, 140.5, "W", lean_set)
    for x, y in ((81.5, 142.5), (85.5, 143.5)):
        c, r = rs.to_px(x, y)
        rs.put_px(c, r, ".")
    # a kink crease where the stack folds over the collar
    for x in range(84, 92):
        rs.put(x + 0.5, 134.5, "Q", lean_set)
    rs.put(84.5, 135.5, "Y", lean_set); rs.put(85.5, 135.5, "T", lean_set)
    # boiler crack from the upper seam down past the gauge: a 1 px plum split, iron-5 lip on its lit side
    boiler = set(ids["boiler"])
    crack = [(80, 91), (78, 86), (80, 81), (77, 76), (79, 71), (76, 66)]
    for (xa, ya), (xb, yb) in zip(crack, crack[1:]):
        for c, r in line_pts(rs.to_px(xa, ya), rs.to_px(xb, yb)):
            if rs.pid[r][c] in boiler:
                rs.key[r][c] = "K"
                if rs.pid[r][c - 1] in boiler and rs.key[r][c - 1] != "K":
                    rs.key[r][c - 1] = "5"
                if rs.pid[r][c + 1] in boiler and rs.key[r][c + 1] != "K":
                    rs.key[r][c + 1] = "1"
    # the split widens at the seam (the steam leak), and a sprung rivet beside it
    for x, y, k in ((79.5, 91.5, "K"), (80.5, 91.5, "K"), (80.5, 90.5, "K"), (81.5, 91.5, "1"), (78.5, 92.5, "6")):
        rs.put(x, y, k, boiler)
    rs.put(83.5, 88.5, "1", boiler)
    # cab dent: a shallow dish in the rear cab plate (shadowed upper-left wall, lit lower-right lip)
    cab = set(ids["cab"])
    cx, cy, rad = 17.0, 68.0, 5.2
    for r in range(h):
        for c in range(w):
            if rs.pid[r][c] not in cab:
                continue
            x, y = rs.to_design(c, r)
            d = math.hypot(x - cx, y - cy)
            if d <= rad:
                side = ((x - cx) * 0.6 - (y - cy) * 0.8) / max(1e-6, d)   # > 0: lower right
                if d > rad - 1.2:
                    rs.key[r][c] = "T" if side > 0.2 else ("Q" if side < -0.2 else "W")
                else:
                    rs.key[r][c] = "Q" if side < -0.45 else "W"
    rs.put(cx + 0.5, cy + 0.5, "E", cab)
    # the firebox dies to embers: no bright fire colour on the disabled machine
    for r in range(h):
        for c in range(w):
            k = rs.key[r][c]
            if k in "qn":
                rs.key[r][c] = "k" if k == "q" else "j"
            elif k == "k":
                rs.key[r][c] = "j"
    return ad.despeckle(rs.key)


# Rig position (x from O, y up) of the steam-leak emitter at the crack's widest point.
LEAK_DESIGN = (80.0, 91.0)
LEAK = (round(LEAK_DESIGN[0] - ad.O_X), round(LEAK_DESIGN[1]))


def hull_damage_frames() -> tuple[list[list[list[str]]], tuple[int, int], tuple[int, int]]:
    w, h, ox, fy = ad.hull_canvas(1.0)
    pc = math.floor(ox + ad.O_X)
    base = hull_damaged()
    frames = [shift_columns(base, lambda c, a=amp: pitch_dy(c - pc, a)) for amp in PITCH_AMPS.values()]
    return frames, (w, h), (pc, fy)


# ------------------------------------------------------------------ front boxes (items 2 and 3)

FRONT_PAD = {"front_cab": (2, 0), "front_tub": (0, 4)}     # rows added (top, bottom) for the pitch


def crumple(rows: list[list[str]], c0: int, c1: int, depth: int, limit: int) -> list[list[str]]:
    """Push the top ``limit`` opaque rows of columns c0..c1 down by a tent profile (``depth`` px at the middle)."""
    out = [r[:] for r in rows]
    h = len(rows)
    mid = (c0 + c1) / 2
    for c in range(c0, c1 + 1):
        s = rnd(depth * (1 - abs(c - mid) / ((c1 - c0) / 2 + 0.5)))
        if s <= 0:
            continue
        col = [rows[r][c] for r in range(h)]
        top = next((r for r in range(h) if col[r] != "."), None)
        if top is None:
            continue
        for r in range(top, min(h, top + limit)):
            out[r][c] = "."
        for r in range(top, min(h, top + limit)):
            if r + s < h:
                out[r + s][c] = col[r]
    return out


def front_damage_frames(name: str) -> tuple[list[list[list[str]]], tuple[int, int], tuple[int, int], list[int]]:
    box = ad.front_boxes()[name]
    rows = [r[:] for r in box["rows"]]
    if name == "front_tub":
        # the frame struck the tub's front lip: crumple 14 columns down by up to 3 px, crease the fold
        w = len(rows[0])
        rows = crumple(rows, w - 22, w - 8, 3, 6)
        for c in range(w - 20, w - 9):
            top = next(r for r in range(len(rows)) if rows[r][c] != ".")
            if rows[top + 2][c] not in ".K":
                rows[top + 2][c] = "1"
            if rows[top + 3][c] not in ".K":
                rows[top + 3][c] = "5" if c % 3 else "4"
        rows = outline([["." if k == "K" and _bare(rows, x, y) else k for x, k in enumerate(r)]
                        for y, r in enumerate(rows)])
    top, bottom = FRONT_PAD[name]
    rows = pad(rows, top, bottom)
    ax = box["anchor"][0]
    frames = [shift_columns(rows, lambda c, a=amp: pitch_dy(ax + c, a)) for amp in PITCH_AMPS.values()]
    canvas = (len(rows[0]), len(rows))
    pivot = (0, box["pivot"][1] + top)
    return frames, canvas, pivot, list(box["anchor"])


def _bare(rows, x, y) -> bool:
    """An outline pixel with no opaque non-outline 4-neighbour (left over after a crumple)."""
    h, w = len(rows), len(rows[0])
    return not any(0 <= x + dx < w and 0 <= y + dy < h and rows[y + dy][x + dx] not in ".K"
                   for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))


# ------------------------------------------------------------------ yoke (item 3)

YOKE_KINK = (12, 27, 42)      # box columns: sag starts, deepest, ends (the axle cap starts at column 47)
YOKE_SAG = 3


def yoke_damaged() -> tuple[list[list[str]], tuple[int, int]]:
    rows, piv = ad.yoke_rows_polished()
    rows = [r[:] for r in rows]
    # broken lamp glass: dark empty housing, one shard catching the light, a notch knocked out of the rim
    glass = [(x, y) for y, r in enumerate(rows) for x, k in enumerate(r) if k in FIRE]
    for x, y in glass:
        rows[y][x] = "0"
    gx0, gy0 = min(p[0] for p in glass), min(p[1] for p in glass)
    rows[gy0 + 1][gx0] = "6"
    rows[gy0 + 2][gx0 + 1] = "1"
    rows[gy0 + 4][gx0] = "3"
    a, m, b = YOKE_KINK

    def sag(c: int) -> int:
        if c < a or c > b:
            return 0
        t = (c - a) / (m - a) if c <= m else (b - c) / (b - m)
        return -rnd(YOKE_SAG * t)
    rows = shift_columns(rows, sag)
    # the fold: a dark crease across the arm at the deepest column, a lit edge after it
    for y in range(len(rows)):
        if rows[y][m] in "0123456":
            rows[y][m] = "1"
            if rows[y][m + 1] in "0123456":
                rows[y][m + 1] = "5"
    return ad.despeckle(rows), piv


# ------------------------------------------------------------------ jammed wheel (item 1)

JAM_PHASE = 0.0                 # spikes at 90, 60, 30 ... degrees: one spike points straight up
# The stop frame's tie bar jammed through the spokes (wheel px from the axle, y up): it enters above the upper-right
# spikes and rests on the lower-left rim under the push arm. Radius in px.
JAM_BAR = ((44.0, 28.0), (-41.0, 5.0), 2.3)
SPIKE_DULL = {"7": "3", "6": "4", "5": "3", "4": "2"}    # two steps darker: no blade highlight survives
RIM_DULL = {"7": "2", "5": "3", "3": "2"}
RUSTED = (2, 4, 7, 9, 11)        # spikes (k: axis 90 - 30k degrees) with a rust run on their shadow face
BENT = {90: (41.0, -62.0, 7.0), 30: (42.5, -48.0, 6.0)}    # spike: (bend radius, direction offset, stub length)
SNAPPED = {150: 42.0}


def jammed_wheel() -> list[list[str]]:
    rows, c0 = ad.wheel_rows(JAM_PHASE)
    n = len(rows)
    rows = [["." if k == "K" else k for k in r] for r in rows]
    rim, tip = ad.WHEEL_RIM, ad.WHEEL_TIP

    def polar(c, r):
        x, y = c - c0, c0 - r
        return math.hypot(x, y), math.degrees(math.atan2(y, x)) % 360.0

    def off_axis(c, r, axis):
        return (polar(c, r)[1] - axis + 180.0) % 360.0 - 180.0

    # dull every spike and rim glint, rust runs on the shadow face of five spikes
    for r in range(n):
        for c in range(n):
            k = rows[r][c]
            d, ang = polar(c, r)
            if d > rim and k in SPIKE_DULL:
                rows[r][c] = SPIKE_DULL[k]
                near = round((90.0 - ang) / 30.0) % 12
                lat = off_axis(c, r, 90.0 - 30.0 * near)
                if near in RUSTED and lat < -2.0 and rim + 1.5 < d < tip - 3.0:
                    rows[r][c] = "Q" if d < rim + 5.0 else "W"
            elif rim - 3.0 < d <= rim and k in RIM_DULL:
                rows[r][c] = RIM_DULL[k]
    # cut the bent and snapped spikes beyond their break radius
    for axis, rb in [(a, v[0]) for a, v in BENT.items()] + list(SNAPPED.items()):
        for r in range(n):
            for c in range(n):
                if rows[r][c] != "." and polar(c, r)[0] > rb and abs(off_axis(c, r, axis)) <= 14.0:
                    rows[r][c] = "."
    # each bent spike: a short tapered stub folded off its axis (dull iron, lit upper edge)
    for axis, (rb, off, ln) in BENT.items():
        a0 = math.radians(axis)
        bx, by = rb * math.cos(a0), rb * math.sin(a0)
        a1 = math.radians(axis + off)
        ux, uy = math.cos(a1), math.sin(a1)
        nx, ny = -uy, ux
        if ny < 0:
            nx, ny = -nx, -ny                                   # the normal points up (towards the light)
        for i in range(int(ln * 4) + 1):
            t = i / 4.0
            half = 2.4 * (1.0 - t / (ln + 1.5))
            for j in range(-int(half * 4), int(half * 4) + 1):
                s = j / 4.0
                x, y = bx + t * ux + s * nx, by + t * uy + s * ny
                c, r = c0 + round(x), c0 - round(y)
                if 0 <= c < n and 0 <= r < n:
                    rows[r][c] = "4" if s > half - 1.0 else ("2" if s < -half + 1.0 else "3")
    # the snapped spike: a ragged break, raw metal on the upper face
    sa = math.radians(150.0)
    for t, k in ((SNAPPED[150] - 0.5, "4"), (SNAPPED[150] - 1.5, "3")):
        rows[c0 - round(t * math.sin(sa))][c0 + round(t * math.cos(sa))] = k
    # the tie bar through the spokes: rust primer, lit top line, a 1 px plum shadow cast on the wheel under it
    (xa, ya), (xb, yb), rad = JAM_BAR
    dx, dy = xb - xa, yb - ya
    bar = set()
    for r in range(n):
        for c in range(n):
            x, y = c - c0, c0 - r
            t = max(0.0, min(1.0, ((x - xa) * dx + (y - ya) * dy) / (dx * dx + dy * dy)))
            qx, qy = xa + t * dx, ya + t * dy
            if math.hypot(x - qx, y - qy) <= rad:
                u = (y - qy) * (-dx) / math.hypot(dx, dy) + (x - qx) * dy / math.hypot(dx, dy)  # + = above the axis
                rows[r][c] = "Y" if u > rad - 0.9 else ("T" if u > 0.0 else ("E" if u > -rad + 0.9 else "W"))
                bar.add((c, r))
    for c, r in sorted(bar):
        if (c, r + 1) not in bar and r + 1 < n and rows[r + 1][c] != ".":
            rows[r + 1][c] = "K"
    for (ex, ey) in ((xa, ya),):                         # the torn end where the hanger strap ripped off
        c, r = c0 + round(ex), c0 - round(ey)
        rows[r][c] = "5"
        rows[r + 1][c] = "4"
    return ad.despeckle(outline(rows))


# ------------------------------------------------------------------ the stop frame (item 4)

FRAME_CANVAS = (96, 60)
FRAME_PIVOT = (8, 59)          # the hanging mass rectangle's bottom-left pixel (80 x 16, 168 px up in room 3)
FRAME_W = 80
LUGS = (10.5, 69.5)            # lifting-lug centres (frame px from the mass rectangle's left edge)
HOOK = (40.0, 38.0)            # the sling hook: both sling legs meet here
CENTRE_X = 40
BLOCK = (32.0, 46.0, 48.0, 56.0)  # pulley housing x0, y0, x1, y1
CHAIN_V = ((".", "5", "."), (".", "4", "."), ("5", "K", "3"), ("4", "K", "2"))   # 4-row link repeat, lit left
CHAIN_LINE = (("5", "3"), ("4", "2"), ("3", "K"), ("4", "2"))                     # diagonal: (spine, under)


def chain_v(rows, col: int, r0: int, r1: int, phase: int = 0) -> None:
    """Vertical chain (rows r0..r1 inclusive) centred on ``col``: an edge-on link then a face link with a plum
    hole, lit from the left. Chains carry no outline: the plum link holes are their dark."""
    for r in range(r0, r1 + 1):
        for i, k in enumerate(CHAIN_V[(r - r0 + phase) % 4]):
            if k != "." and 0 <= r < len(rows) and 0 <= col - 1 + i < len(rows[0]):
                rows[r][col - 1 + i] = k


def chain_h(rows, row: int, c0: int, c1: int, phase: int = 0) -> None:
    """Horizontal chain (columns c0..c1) centred on ``row``; lit from above (the transpose of ``chain_v``)."""
    for c in range(c0, c1 + 1):
        for i, k in enumerate(CHAIN_V[(c - c0 + phase) % 4]):
            if k != "." and 0 <= row - 1 + i < len(rows) and 0 <= c < len(rows[0]):
                rows[row - 1 + i][c] = k


def chain_line(rows, p0, p1) -> None:
    """Sloped chain between two pixel positions: a 2 px band (a lit spine and a dark underside) whose colours
    repeat every 4 px along the line, so the links read at 1x without an outline."""
    for i, (c, r) in enumerate(line_pts(p0, p1)):
        spine, under = CHAIN_LINE[i % 4]
        rows[r][c] = spine
        if r + 1 < len(rows):
            rows[r + 1][c] = under


def frame_raster() -> "ad.Raster":
    w, h = FRAME_CANVAS
    return ad.Raster(w, h, FRAME_PIVOT[0], FRAME_PIVOT[1], 1.0)


def stop_frame_raised() -> list[list[str]]:
    """The hanging press frame: a riveted rust-primer I-beam (flanges and web) with hazard stencils at both
    ends, the round tie bar slung under it on two straps, two lifting lugs, a V sling of chain to one hook and
    the hook's chain up to the block."""
    rs = frame_raster()
    rs.capsule((2.6, 2.4), (77.4, 2.4), 1.6, RUST, mode="cyl_h", y0=0.8, y1=4.0, outline=True)
    straps = [rs.rect(11, 3.5, 15, 6.6, IRON, level=2, outline=True),
              rs.rect(65, 3.5, 69, 6.6, IRON, level=2, outline=True)]
    rs.rect(1, 6, 79, 8.6, RUST, mode="cyl_h", outline=True)
    web = rs.rect(2, 8.6, 78, 12.4, RUST, level=1)
    rs.rect(1, 12.4, 79, 15, RUST, mode="cyl_h", outline=True)
    lugs = [rs.rect(8, 15, 13, 19.6, IRON, level=3, outline=True),
            rs.rect(67, 15, 72, 19.6, IRON, level=3, outline=True)]
    hook = rs.disc(HOOK[0], HOOK[1], 2.8, IRON, outline=True)
    rs.render()
    web_set = {web}
    rs.hline(2, 77.5, 12.0, "Q", web_set)
    rs.rivets(6, 74, 10.4, 6, only_on=web_set)
    for x0 in (4, 66):                                    # hazard stencils: brass and iron-0 diagonals
        for c in range(10):
            for r in range(4):
                rs.put(x0 + c + 0.5, 8.6 + r + 0.5, "S" if ((c + r) // 2) % 2 == 0 else "0", web_set)
    for x, y0, y1 in ((27, 8.6, 12.0), (52, 8.6, 11.0)):  # rust runs
        rs.vline(x, y0, y1, "T", web_set)
    for lug in lugs:                                      # the shackle hole
        part = rs.parts[lug]
        rs.put((part["x0"] + part["x1"]) / 2, 17.5, "K", {lug})
    for s in straps:
        rs.put(rs.parts[s]["x0"] + 1.5, 5.0, "6", {s})
    rs.put(HOOK[0], HOOK[1], "K", {hook})
    rows = [r[:] for r in rs.key]
    hx, hy = rs.to_px(*HOOK)
    for i, lx in enumerate(LUGS):
        c, r = rs.to_px(lx, 19.6)
        chain_line(rows, (c, r - 1), (hx - 2 if i == 0 else hx + 2, hy + 1))
    _c, rb = rs.to_px(CENTRE_X, BLOCK[1])
    chain_v(rows, hx, rb + 1, hy - 3, phase=1)
    return ad.despeckle(rows)


def stop_frame_pulley() -> list[list[str]]:
    """The ceiling block: an iron housing with a brass sheave showing through its cheek, hung from a ceiling
    strap, and the haul chain leaving the sheave to the right along the ceiling (towards the brace)."""
    rs = frame_raster()
    x0, y0, x1, y1 = BLOCK
    strap = rs.rect(37, y1 - 0.5, 43, 60, IRON, level=2, outline=True)
    house = rs.poly([(x0, y0 + 2), (x0 + 2, y0), (x1 - 2, y0), (x1, y0 + 2), (x1, y1), (x0, y1)], IRON, level=2,
                    outline=True)
    sheave = rs.disc(CENTRE_X, 51.0, 4.6, BRASS, outline=True)
    rs.render()
    rs.put(CENTRE_X, 51.0, "L", {sheave})
    rs.put(CENTRE_X - 1, 52.0, "Z", {sheave})
    rs.rivets(x0 + 2, x1 - 2, y0 + 1.5, 6, only_on={house})
    rs.put(38.5, 58.0, "7", {strap}); rs.put(41.5, 58.0, "7", {strap})
    rows = [r[:] for r in rs.key]
    r_of = lambda y: FRAME_PIVOT[1] - math.floor(y)  # noqa: E731
    fx = FRAME_PIVOT[0]
    chain_h(rows, r_of(55.0), fx + math.ceil(x1) + 1, FRAME_CANVAS[0] - 1, phase=2)
    return ad.despeckle(rows)


# ------------------------------------------------------------------ the remnant (item 4)

REMNANT_CANVAS = (80, 16)
REMNANT_PIVOT = (0, 15)
REMNANT_KINK = (6, 44, 74)     # columns: bow starts, apex (the fold), bow ends
REMNANT_BOW = 5


def remnant(with_bar: bool) -> list[list[str]]:
    """The beam after the fall, bowed up into a fold left of centre, lying on the floor; for a miss the snapped
    tie bar lies in front of it."""
    w, h = REMNANT_CANVAS
    rs = ad.Raster(w, h, 0, h - 1, 1.0)
    low = rs.rect(1, 1, 79, 3.6, RUST, mode="cyl_h", outline=True)
    web = rs.rect(2, 3.6, 78, 7.4, RUST, level=1)
    top = rs.rect(1, 7.4, 79, 10, RUST, mode="cyl_h", outline=True)
    rs.render()
    web_set = {web}
    rs.hline(2, 77.5, 7.0, "Q", web_set)
    rs.rivets(6, 74, 5.4, 6, only_on=web_set)
    for x0 in (4, 66):
        for c in range(10):
            for r in range(4):
                rs.put(x0 + c + 0.5, 3.6 + r + 0.5, "S" if ((c + r) // 2) % 2 == 0 else "0", web_set)
    _ = (low, top)
    a, m, b = REMNANT_KINK

    def bow(c: int) -> int:
        if c < a or c > b:
            return 0
        t = (c - a) / (m - a) if c <= m else (b - c) / (b - m)
        return rnd(REMNANT_BOW * t)
    rows = shift_columns([r[:] for r in rs.key], bow)
    for y in range(h):                       # the fold: a dark crease and a scraped-bright edge
        if rows[y][m] not in ".K":
            rows[y][m] = "Q"
            if rows[y][m - 1] not in ".K":
                rows[y][m - 1] = "Y"
    if with_bar:
        bar_rows = [["."] * w for _ in range(h)]
        for c in range(5, 57):
            for r, k in ((h - 4, "Y"), (h - 3, "T"), (h - 2, "E")):
                bar_rows[r][c] = k
        for c, r in ((5, h - 4), (56, h - 2)):          # snapped ends
            bar_rows[r][c] = "4"
        bar_rows = outline(bar_rows)
        ad.overlay(rows, bar_rows, 0, 0)
    return ad.despeckle(rows)


# ------------------------------------------------------------------ the brace cleat (item 4)

BRACE_CANVAS = (16, 64)
BRACE_PIVOT = (0, 63)


def brace_intact() -> list[list[str]]:
    """The brace cleat: a timber post on an iron wall plate, two iron straps, a hazard band, and an iron horn
    cleat on top with the haul chain wrapped round it and running up to the ceiling."""
    w, h = BRACE_CANVAS
    rs = ad.Raster(w, h, 0, h - 1, 1.0)
    plate = rs.rect(2, 1, 14, 54, IRON, level=1, outline=True)
    post = rs.rect(4, 1, 12, 50, COPPER, mode="cyl_v", outline=True)
    straps = [rs.rect(3, 9, 13, 12, IRON, mode="cyl_h", outline=True), rs.rect(3, 38, 13, 41, IRON, mode="cyl_h",
                                                                                outline=True)]
    stem = rs.rect(6.5, 50, 9.5, 55, IRON, level=3, outline=True)
    horn = rs.capsule((3.5, 56.5), (12.5, 56.5), 1.4, IRON, mode="cyl_h", y0=55, y1=58, outline=True)
    rs.render()
    post_set = {post}
    for x in (6.0, 9.0):                                  # timber grain
        rs.vline(x, 13, 37, "c", post_set)
        rs.vline(x, 42, 49, "c", post_set)
    for c in range(8):                                    # hazard band: the thing to hit
        for r in range(7):
            rs.put(4 + c + 0.5, 22 + r + 0.5, "S" if ((c + r) // 2) % 2 == 0 else "0", post_set)
    for s in straps:
        part = rs.parts[s]
        rs.put(part["x0"] + 1.5, part["y0"] + 1.5, "7", {s})
    rs.put(2.5, 2.5, "7", {plate}); rs.put(13.5, 52.5, "7", {plate})
    _ = (stem, horn)
    rows = [r[:] for r in rs.key]
    # the haul chain: wrapped over the horn (two crossing links), then straight up off the canvas top
    for c, r, k in ((5, 7, "5"), (6, 6, "4"), (7, 5, "5"), (9, 7, "3"), (10, 6, "4"), (11, 5, "3"), (8, 6, "K"),
                    (8, 5, "4")):
        rows[r][c] = k
    chain_v(rows, 8, 0, 4, phase=0)
    return ad.despeckle(rows)


# ------------------------------------------------------------------ grids

TICK_HZ = ad.TICK_HZ
HOLD = None
STEMS = {
    "dozer_wheel_jammed_h120": ("dozer_wheel_jammed_h120_v0", "vehicle_part", DOZER_DIR,
                                "DOZ-02 jammed wheel: stopped, the stop frame's tie bar wedged across the upper "
                                "spikes, two spikes bent and one snapped, every glint dulled (a dead wheel)"),
    "dozer_hull_damage_h120": ("dozer_hull_damage_h120_v0", "vehicle_part", DOZER_DIR,
                               "DOZ-02 hull body after the stop frame: impact pitch keys (front down 4, rear up 2, "
                               "and half) and the level disabled frame, all with the cab dent, the boiler crack, the "
                               "crooked stack without its bucket and the firebox down to embers"),
    "dozer_front_cab_damage_h120": ("dozer_front_cab_damage_h120_v0", "vehicle_part", DOZER_DIR,
                                    "DOZ-02 hull front, cab rail box, pitched with the hull (pitch_0, pitch_1, level)"),
    "dozer_front_tub_damage_h120": ("dozer_front_tub_damage_h120_v0", "vehicle_part", DOZER_DIR,
                                    "DOZ-02 hull front, rider tub box with the lip crumpled where the frame struck, "
                                    "pitched with the hull (pitch_0, pitch_1, level)"),
    "dozer_yoke_damage_h120": ("dozer_yoke_damage_h120_v0", "vehicle_part", DOZER_DIR,
                               "DOZ-02 yoke bent down at mid-arm, lamp glass broken (no fire colours)"),
    "prop_stop_frame": ("prop_stop_frame_v0", "prop_animation", PROP_DIR,
                        "DOZ-02 stop frame, hanging: the press beam with its tie bar, chains and spreader (raised) "
                        "and the ceiling pulley block with the haul chain to the brace (pulley)"),
    "prop_stop_frame_remnant": ("prop_stop_frame_remnant_v0", "prop_animation", PROP_DIR,
                                "DOZ-02 stop frame remnant, 80 x 16: the beam bowed into a fold, with the snapped tie "
                                "bar (landed, a miss) or without it (landed_hit: the bar is in the wheel)"),
    "prop_stop_frame_brace": ("prop_stop_frame_brace_v0", "prop_animation", PROP_DIR,
                              "DOZ-02 brace cleat on the back wall: timber post, straps, hazard band and the horn "
                              "cleat holding the haul chain (intact)"),
}
FRAME_NAMES = ("pitch_0", "pitch_1", "disabled")
FRAME_TICKS: dict[str, list[int | None]] = {
    "dozer_wheel_jammed_h120": [HOLD],
    "dozer_hull_damage_h120": [6, 6, HOLD],
    "dozer_front_cab_damage_h120": [6, 6, HOLD],
    "dozer_front_tub_damage_h120": [6, 6, HOLD],
    "dozer_yoke_damage_h120": [HOLD],
    "prop_stop_frame": [HOLD, HOLD],
    "prop_stop_frame_remnant": [HOLD, HOLD],
    "prop_stop_frame_brace": [HOLD],
}


def _grid(stem: str, layer: str, frames, canvas, pivot, names, ranges=None) -> dict:
    ticks = FRAME_TICKS[stem]
    assert len(ticks) == len(frames) == len(names), stem
    cels = [({layer: f}, ad.duration_ms(t)) for f, t in zip(frames, ticks)]
    return ad.grid_from(cels, canvas, pivot, [layer], ad._tags(list(names), ranges))


def build_grids() -> dict[str, dict]:
    grids: dict[str, dict] = {}
    wheel = jammed_wheel()
    c0 = len(wheel) // 2
    grids["dozer_wheel_jammed_h120"] = _grid("dozer_wheel_jammed_h120", "wheel", [wheel], (len(wheel), len(wheel)),
                                             (c0, c0), ["jammed"])
    frames, canvas, pivot = hull_damage_frames()
    grids["dozer_hull_damage_h120"] = _grid("dozer_hull_damage_h120", "body", frames, canvas, pivot, FRAME_NAMES)
    for name in ("front_cab", "front_tub"):
        frames, canvas, pivot, _anchor = front_damage_frames(name)
        stem = f"dozer_{name}_damage_h120"
        grids[stem] = _grid(stem, "front", frames, canvas, pivot, FRAME_NAMES)
    yoke, ypiv = yoke_damaged()
    grids["dozer_yoke_damage_h120"] = _grid("dozer_yoke_damage_h120", "yoke", [yoke], (len(yoke[0]), len(yoke)),
                                            ypiv, ["bent"])
    grids["prop_stop_frame"] = _grid("prop_stop_frame", "frame", [stop_frame_raised(), stop_frame_pulley()],
                                     FRAME_CANVAS, FRAME_PIVOT, ["raised", "pulley"])
    grids["prop_stop_frame_remnant"] = _grid("prop_stop_frame_remnant", "remnant", [remnant(True), remnant(False)],
                                             REMNANT_CANVAS, REMNANT_PIVOT, ["landed", "landed_hit"])
    grids["prop_stop_frame_brace"] = _grid("prop_stop_frame_brace", "brace", [brace_intact()], BRACE_CANVAS,
                                           BRACE_PIVOT, ["intact"])
    return grids


# ------------------------------------------------------------------ damage-state data contract

def damage_contract(grids: dict[str, dict]) -> dict:
    fronts = {n: front_damage_frames(n) for n in ("front_cab", "front_tub")}
    seats = {name: [round(x - ad.O_X), round(y)] for name, (x, y) in ad.SEATS.items()}

    def seat_dy(amp):
        return {name: pitch_dy(x, amp) for name, (x, _y) in seats.items()}
    return {
        "format": "dozer-damage-v0",
        "status": "DOZ-02 design keys; the remaining frames, the export and the Bevy wiring follow the execution "
                  "brief; independent review pending",
        "spec": "docs/02-art/doz-02-dozer-damage-spec.md",
        "coordinates": "as dozer_rig_h120.json: px from the rig origin O, x forward (mirror when facing left), y up; "
                       "prop parts: px from the room's stop-frame rectangles (layout px, y down in the room file)",
        "replaces": {
            "wheel": {"atlas": "dozer_wheel_jammed_h120", "tag": "jammed", "anchor": "wheel (axle)",
                      "states": ["impact_stunned", "disabled"]},
            "hull_body": {"atlas": "dozer_hull_damage_h120", "anchor": "hull_body", "states": ["impact_stunned",
                                                                                                "disabled"]},
            "hull_front_cab": {"atlas": "dozer_front_cab_damage_h120", "anchor": fronts["front_cab"][3],
                               "frame_size": list(fronts["front_cab"][1]), "pivot": list(fronts["front_cab"][2])},
            "hull_front_tub": {"atlas": "dozer_front_tub_damage_h120", "anchor": fronts["front_tub"][3],
                               "frame_size": list(fronts["front_tub"][1]), "pivot": list(fronts["front_tub"][2])},
            "yoke": {"atlas": "dozer_yoke_damage_h120", "tag": "bent", "anchor": "yoke (axle)",
                     "states": ["impact_stunned", "disabled"]},
        },
        "pitch": {
            "rule": "each hull column moves up by round(amp * (2 - 6 * (x + 84) / 167)) px (half away from zero), "
                    "x = the column's rig x; the tread, wheel and yoke stay on the floor",
            "frames": {name: amp for name, amp in PITCH_AMPS.items()},
            "rider_seat_dy": {name: seat_dy(amp) for name, amp in PITCH_AMPS.items()},
        },
        "timing": {
            "tick_hz": TICK_HZ,
            "impact_stunned": {
                "ticks": 48,
                "hull": [["pitch_0", 0, 5], ["pitch_1", 6, 11], ["disabled", 12, 47]],
                "wheel": "jammed from tick 0", "yoke": "bent from tick 0",
                "hit_spark": "vfx_hit_spark_metal at the tub lip crumple, tick 0",
                "steam_leak": "steam burst (3 puffs, every 6 ticks) from the leak emitter, ticks 0..47",
                "stack": "one black puff on tick 0, then none (the stack is knocked crooked)",
                "riders": "lurch forward (DOZ-01 keys) offset by rider_seat_dy of the hull frame shown",
            },
            "disabled": {
                "hull": "disabled", "wheel": "jammed", "yoke": "bent",
                "steam_leak": "one steam_0..steam_2 puff every 40 ticks (slow, thin)",
                "stack": "one black_3..black_5 wisp every 60 ticks",
                "driver": "slumped hold (execution: the pot over his eyes)",
            },
        },
        "emitters": {"steam_leak": list(LEAK), "stack_crooked": [round(82.0 - ad.O_X), 144]},
        "stop_frame": {
            "atlas": "prop_stop_frame", "pivot": list(FRAME_PIVOT), "frame_size": list(FRAME_CANVAS),
            "anchor": "the hanging mass rectangle's bottom-left pixel (falls with it); the pulley frame stays at the "
                      "raised anchor in every state",
            "frames": {"raised": "Supported, Weakened", "pulley": "every state, at the raised anchor"},
            "remnant": {"atlas": "prop_stop_frame_remnant", "pivot": list(REMNANT_PIVOT),
                        "anchor": "the remnant rectangle's bottom-left pixel",
                        "frames": {"landed": "early miss and rear glance", "landed_hit": "after a hit (the bar is in "
                                   "the wheel)"}},
            "brace": {"atlas": "prop_stop_frame_brace", "pivot": list(BRACE_PIVOT),
                      "anchor": "the prop target rectangle's bottom-left pixel", "frames": {"intact": "Supported"}},
        },
    }


# ------------------------------------------------------------------ manifests

LICENSE = ad.LICENSE
GENERATOR = ("Design and keys: DOZ-02 design step, deterministic output of scripts/author_dozer_doz02.py built on "
             "the DOZ-01 construction (scripts/author_dozer_doz01.py). No image-model raster output, no tracing, "
             "no resampling. Independent review pending")


def manifest(stem: str, grid: dict) -> dict:
    ident, kind, folder, origin = STEMS[stem]
    src = folder.relative_to(ROOT).as_posix()
    m = {
        "schema_version": 1,
        "id": ident,
        "type": kind,
        "source_path": f"{src}/{stem}.ase",
        "grid_source": f"{src}/{stem}.grid.json",
        "output_atlas": f"generated/art/{stem}.png",
        "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "dozer-floor01-v0",
        "frame_size": grid["canvas"],
        "pivot": grid["pivot"],
        "animation_tags": [t["name"] for t in grid["tags"]],
        "required_alpha": True,
        "owner": "AI-authored vehicle enemy and set-piece art (fan-concept layer: Goblin Murder Dozer; runtime role "
                 "IP-neutral large pursuit machine and its stop frame)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": origin,
            "generator": GENERATOR,
            "generated_art": True,
            "input_reference_ids": ["floor-01-opening-concept-references-v1"],
        },
        "license": LICENSE,
        "review_status": "prototype_fixture_unapproved",
        "dozer": {"status": "DOZ-02 design keys, awaiting execution and independent review; not wired into any "
                            "Bevy binary",
                  "spec": "docs/02-art/doz-02-dozer-damage-spec.md"},
    }
    names = [t["name"] for t in grid["tags"] if t["from"] == t["to"]]
    ticks = FRAME_TICKS[stem]
    m["dozer"]["tick_hz"] = TICK_HZ
    m["dozer"]["hold_preview_ms"] = ad.HOLD_MS
    m["dozer"]["frames"] = [{"index": i, "tag": n, "ticks": tk, "duration_ms": ad.duration_ms(tk)}
                            for i, (n, tk) in enumerate(zip(names, ticks))]
    return m


# Validator colour limits: the toolkit default is 24; the hull carries the DOZ-01 vehicle palette (the DOZ-01 hull
# body itself uses 29 keys), so the damaged hull is held to 30.
MAX_COLORS = {"dozer_hull_damage_h120": 30}


def validator_config(stem: str, grid: dict) -> dict:
    """``pixelart-asset-v1`` config for ``python3 -m scripts.pixelart validate --config``."""
    folder = STEMS[stem][2].relative_to(ROOT).as_posix()
    tags = {t["name"]: {"playback": "once", "contact": "free",
                        "frames": [[t["from"], FRAME_TICKS[stem][t["from"]] or round(ad.HOLD_MS * TICK_HZ / 1000)]]}
            for t in grid["tags"]}
    return {"format": "pixelart-asset-v1", "asset_id": STEMS[stem][0],
            "title": f"DOZ-02 {stem} (design keys)",
            "grid": f"{folder}/{stem}.grid.json", "manifest": f"assets/manifests/{stem}.json",
            "frame_size": grid["canvas"], "pivot": grid["pivot"], "tick_hz": TICK_HZ, "hold_ms": ad.HOLD_MS,
            "tags": tags,
            "rules": {"max_colors": MAX_COLORS.get(stem, 24), "max_isolated_per_frame": 0,
                      "max_orphans_per_frame": 9, "unique_frames": True}}


def build_all() -> dict[Path, str]:
    grids = build_grids()
    out: dict[Path, str] = {}
    for stem, grid in grids.items():
        pg.validate_grid(grid)
        folder = STEMS[stem][2]
        out[folder / f"{stem}.grid.json"] = json.dumps(grid, indent=1) + "\n"
        out[folder / f"{stem}.pixelart.json"] = json.dumps(validator_config(stem, grid), indent=2) + "\n"
        out[MANIFEST_DIR / f"{stem}.json"] = json.dumps(manifest(stem, grid), indent=2) + "\n"
    out[DOZER_DIR / "dozer_damage_h120.json"] = json.dumps(damage_contract(grids), indent=2) + "\n"
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--check", action="store_true", help="compare instead of writing (exit 1 on a difference)")
    parser.add_argument("--out-dir", type=Path, default=None, help="write here instead of the source tree")
    args = parser.parse_args(argv)
    status = 0
    for path, text in build_all().items():
        if args.out_dir is not None:
            path = args.out_dir / path.name
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name}: " + ("matches the build" if same else "DIFFERS from the build"))
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return status


if __name__ == "__main__":
    sys.exit(main())
