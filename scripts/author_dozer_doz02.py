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


def stop_frame_raised(slack: bool = False) -> list[list[str]]:
    """The hanging press frame: a riveted rust-primer I-beam (flanges and web) with hazard stencils at both
    ends, the round tie bar slung under it on two straps, two lifting lugs, a V sling of chain to one hook and
    the hook's chain up to the block. ``slack`` is the ``falling`` frame (execution): both sling legs bow 2 px
    outward, the hook chain hangs in a loose S and a motion streak trails above each lug."""
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
    lug_cols = []
    for i, lx in enumerate(LUGS):
        c, r = rs.to_px(lx, 19.6)
        p0, p1 = (c, r - 1), (hx - 2 if i == 0 else hx + 2, hy + 1)
        if slack:
            mid = ((p0[0] + p1[0]) // 2 + (-SLING_BOW if i == 0 else SLING_BOW), (p0[1] + p1[1]) // 2)
            chain_line(rows, p0, mid)
            chain_line(rows, mid, p1)
        else:
            chain_line(rows, p0, p1)
        lug_cols.append((c, r))
    _c, rb = rs.to_px(CENTRE_X, BLOCK[1])
    if slack:
        for n, r in enumerate(range(rb + 1, hy - 2)):                       # the loose S of the hook chain
            off = HOOK_S[n % len(HOOK_S)]
            for k, key in enumerate(CHAIN_V[(n + 1) % 4]):
                if key != ".":
                    rows[r][hx - 1 + k + off] = key
        for c, r in lug_cols:                                               # a streak above each lug
            for rr in range(r - 8, r - 2):
                assert rows[rr][c + 1] == ".", (c + 1, rr)
                rows[rr][c + 1] = "3"
    else:
        chain_v(rows, hx, rb + 1, hy - 3, phase=1)
    return ad.despeckle(rows)


SLING_BOW = 2
HOOK_S = (0, 1, 1, 0, -1, -1, 0)


def shift_x(rows: list[list[str]], dx: int) -> list[list[str]]:
    """Translate every pixel ``dx`` columns (no resampling); a pixel leaving the canvas is an error."""
    h, w = len(rows), len(rows[0])
    out = [["."] * w for _ in range(h)]
    for r in range(h):
        for c in range(w):
            if rows[r][c] != ".":
                if not 0 <= c + dx < w:
                    raise ValueError(f"column {c} leaves the canvas at shift {dx}")
                out[r][c + dx] = rows[r][c]
    return out


# dust motes shaken loose by the warning: 2 or 3 px of `5`/`4` beside each lug (frame px, key); the two
# warning frames carry different sets, so the pair flickers as it alternates every 3 ticks
WARN_MOTES = {
    0: (((12, 41), "5"), ((11, 41), "4"), ((12, 42), "4"), ((83, 38), "5"), ((84, 38), "4")),
    1: (((11, 43), "5"), ((12, 43), "4"), ((84, 40), "5"), ((83, 41), "4"), ((84, 41), "4")),
}


def stop_frame_warn(i: int) -> list[list[str]]:
    """``warn_0`` / ``warn_1``: the raised key 1 px left / right, with dust motes (execution)."""
    rows = shift_x(stop_frame_raised(), -1 if i == 0 else 1)
    for (c, r), k in WARN_MOTES[i]:
        assert rows[r][c] == ".", (i, c, r)
        rows[r][c] = k
    return rows


def stop_frame_falling() -> list[list[str]]:
    """``falling`` (execution): the raised key with the sling slack and a motion streak above each lug."""
    return stop_frame_raised(slack=True)


# ------------------------------------------------------------------ the floor shadow (execution)

SHADOW_CANVAS = (80, 4)
SHADOW_PIVOT = (0, 3)
# Row widths (top to bottom) of the four shadow frames: the warning grows it 40 x 1 -> 80 x 4 px (alpha 0.3 -> 0.6)
SHADOW_ROWS = ((40,), (48, 53), (54, 64, 67), (58, 74, 80, 70))


def centred_rows(canvas_w: int, widths: tuple[int, ...], canvas_h: int, key: str = "K") -> list[list[str]]:
    """Rows of ``key`` with the given widths, each centred on the canvas, stacked on the canvas bottom."""
    rows = [["."] * canvas_w for _ in range(canvas_h)]
    for j, w in enumerate(widths):
        r = canvas_h - len(widths) + j
        c0 = (canvas_w - w) // 2
        for c in range(c0, c0 + w):
            rows[r][c] = key
    return rows


def shadow_frame(i: int) -> list[list[str]]:
    return centred_rows(SHADOW_CANVAS[0], SHADOW_ROWS[i], SHADOW_CANVAS[1])


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


# ------------------------------------------------------------------ the rear glance (execution, section 4.3)

GLANCE_CANVAS = (88, 88)
GLANCE_PIVOT = (44, 44)                    # the beam's centre
GLANCE_DEG = (0, 15, 35)                   # glance_0..2: the left end tips toward the rear (down)
GLANCE_DROP0 = 2                           # glance_0 sits 2 px lower than the raised key
GLANCE_SHADOW = (64, 3)
GLANCE_OFFSETS = ((0, 0), (-6, 0), (-16, -8))   # px from the strike position (x forward, y up) per frame
# The strike position in rig px (x from O, y up): the beam's centre over the hull's rear deck, its left end 8 px in
# from the rear of the push box (x -84) and its bottom on the rear deck (the cab roof, 108: 12 px under the push
# box's top): the presenter's rule at rig scale.
GLANCE_ANCHOR_RIG = (-36, 116)


def beam_rows() -> list[list[str]]:
    """The 80 x 16 press beam of the raised key (flanges, web, rivets, stencils), without lugs and sling."""
    rows = stop_frame_raised()
    px, py = FRAME_PIVOT
    return [r[px:px + FRAME_W] for r in rows[py - 15:py + 1]]


def glance_beam(deg: float, drop: int = 0) -> list[list[str]]:
    """The beam turned ``deg`` degrees about its centre, the left end down, redrawn column by column: the output
    column x takes the source column round((x - c) / cos), shifted by round((39.5 - u) * sin) rows. The lengthwise
    squeeze drops a source column here and there; a column that carries a rivet is never dropped, so the rivet
    row stays one clean line (no resampling, no new pixels, no new colours)."""
    beam = beam_rows()
    w, h = FRAME_W, len(beam)
    cw, ch = GLANCE_CANVAS
    a = math.radians(deg)
    cos, sin = math.cos(a), math.sin(a)
    cx = cw // 2                                   # the pivot column; the beam's centre sits between 43 and 44
    mid = cx - 0.5
    out = [["."] * cw for _ in range(ch)]
    src = {}
    for x in range(cw):
        u = math.floor((x - mid) / cos + (w - 1) / 2 + 0.5 + 1e-9)
        if 0 <= u < w:
            src[x] = u
    for u in range(w):
        if any(beam[r][u] == "7" for r in range(h)):
            src[math.floor(mid + (u - (w - 1) / 2) * cos + 0.5 + 1e-9)] = u
    for x, u in src.items():
        top = cx - h // 2 + round(((w - 1) / 2 - u) * sin) + drop
        for r in range(h):
            if beam[r][u] != ".":
                out[top + r][x] = beam[r][u]
    return ad.despeckle(out)


def glance_shadow() -> list[list[str]]:
    cw, ch = GLANCE_CANVAS
    w, h = GLANCE_SHADOW
    rows = [["."] * cw for _ in range(ch)]
    for j, ww in enumerate((w - 14, w, w - 8)):
        c0 = (cw - ww) // 2
        for c in range(c0, c0 + ww):
            rows[ch - h + j][c] = "K"
    return rows


# ------------------------------------------------------------------ the brace states (execution, section 4.4)

def brace_state(state: str) -> list[list[str]]:
    """``weakened`` (one hit of two): the horn bent 2 px down, the haul chain's wrap slipped 2 px, a 3 px crack in
    ``K`` across the post. ``snapped``: the horn broken off (a 4 x 3 piece lies at the plate foot), the chain end
    hanging free; ``snapped_swing`` is the same with the chain's lower half 1 px over."""
    rows = brace_intact()
    w, h = BRACE_CANVAS
    horn_rows = range(5, 9)
    if state == "weakened":
        moved = {(c, r): rows[r][c] for r in horn_rows for c in range(8, w) if rows[r][c] != "."}
        for (c, r) in moved:
            rows[r][c] = "."
        for (c, r), k in moved.items():
            rows[r + 2][c] = k
        for r in (5, 6):                                    # the chain still runs into the slipped wrap
            for c in (7, 8, 9):
                rows[r][c] = CHAIN_V[(r + 1) % 4][c - 7]
        for c, r in ((6, 18), (7, 18), (8, 19)):            # the crack across the post
            if rows[r][c] != ".":
                rows[r][c] = "K"
        return ad.despeckle(rows)
    for r in horn_rows:                                      # the horn and its wrap are gone
        for c in range(w):
            rows[r][c] = "."
    for c, r, k in ((6, 8, "K"), (7, 8, "4"), (8, 8, "3"), (9, 8, "K")):      # the stub of the stem
        rows[r][c] = k
    swing = 1 if state == "snapped_swing" else 0
    for r in range(0, 16):                                   # the chain hangs free down the post
        for k, key in enumerate(CHAIN_V[r % 4]):
            if key != ".":
                c = 7 + k + (swing if r >= 9 else 0)
                rows[r][c] = key
    for c, r, k in ((12, 61, "4"), (13, 61, "3"), (14, 61, "3"), (15, 61, "K"),
                    (12, 62, "K"), (13, 62, "2"), (14, 62, "2"), (15, 62, "K"),
                    (12, 63, "K"), (13, 63, "K"), (14, 63, "K")):             # the broken horn on the floor
        rows[r][c] = k
    return ad.despeckle(rows)


# ------------------------------------------------------------------ the room pieces (execution, section 4.5)

ENV_DIR = ROOT / "assets/source/props/dozer_room"
GATE_TILE = 32
DUST = (("5", "4"), ("4", "5"))


def gate_tile(state: str) -> list[list[str]]:
    """The rear gate, one 32 x 32 tile (stacked up the lane): two riveted iron doors on a centre seam, a cross
    bar with a brass hasp, rust runs. ``shake_0`` / ``shake_1`` move the doors 1 px left / right (the jambs stay)
    and shake dust loose; ``burst`` shows the doors bent outward at the jambs and the bar snapped."""
    n = GATE_TILE
    rows = [["."] * n for _ in range(n)]
    for r in range(n):
        rows[r][0], rows[r][n - 1] = "K", "K"
        rows[r][1], rows[r][n - 2] = "5", "1"
    doors = [["."] * n for _ in range(n)]
    for r in range(n):
        for c in range(2, n - 2):
            left = c < n // 2
            edge = c in (2, n // 2 + 1)
            shade = "4" if edge else ("3" if (c % 7) else "2")
            if c in (n // 2 - 2, n - 3):
                shade = "2"
            doors[r][c] = shade if left or c >= n // 2 else shade
        doors[r][n // 2 - 1], doors[r][n // 2] = "K", "K"
    for r in (4, 26):                                            # rivet rows
        for c in (5, 10, 21, 26):
            doors[r][c] = "7"
    for r in range(12, 20):                                      # the cross bar
        for c in range(2, n - 2):
            doors[r][c] = {12: "K", 13: "5", 14: "4", 15: "3", 16: "3", 17: "2", 18: "1", 19: "K"}[r]
    for c in range(13, 19):                                      # the brass hasp
        for r in range(13, 19):
            doors[r][c] = "K" if c in (13, 18) or r in (13, 18) else ("S" if (c + r) % 2 else "N")
    doors[15][15] = doors[15][16] = "K"
    for r in range(20, 31):                                      # rust runs below the bar
        for c in (6, 7):
            doors[r][c] = "W" if r < 27 else "E"
        if r < 25:
            doors[r][23] = "E"
    for r in range(3, 11):
        doors[r][14] = "W" if r % 2 else "E"
    if state == "burst":
        out = [["."] * n for _ in range(n)]
        for r in range(n):
            lean = 5 + round(1.5 * math.sin(r * 0.55))           # the bent left leaf
            for c in range(1, lean):
                out[r][c] = "4" if c == lean - 1 else ("3" if c > 2 else "5")
            out[r][lean] = "K"
            lean2 = n - 6 - round(1.5 * math.sin(r * 0.55 + 1.3))  # the bent right leaf
            for c in range(lean2 + 1, n - 1):
                out[r][c] = "2" if c == lean2 + 1 else ("3" if c < n - 3 else "1")
            out[r][lean2] = "K"
        for r in range(12, 20):                                  # the snapped bar: two stubs, ragged ends
            for c in range(1, 10 + (r % 3)):
                out[r][c] = {12: "K", 13: "5", 14: "4", 15: "3", 16: "3", 17: "2", 18: "1", 19: "K"}[r]
            for c in range(n - 10 - ((r + 1) % 3), n - 1):
                out[r][c] = {12: "K", 13: "5", 14: "4", 15: "3", 16: "3", 17: "2", 18: "1", 19: "K"}[r]
        for r in (4, 26):
            out[r][3] = out[r][n - 4] = "7"
        for r in range(1, n):                                    # jambs
            out[r][0], out[r][n - 1] = "K", "K"
        out[0][0] = out[0][n - 1] = "K"
        return ad.despeckle(out)
    if state in ("shake_0", "shake_1"):
        dx = -1 if state == "shake_0" else 1
        moved = [["."] * n for _ in range(n)]
        for r in range(n):
            for c in range(2, n - 2):
                if 2 <= c + dx < n - 2:
                    moved[r][c + dx] = doors[r][c]
            moved[r][2 if dx > 0 else n - 3] = doors[r][2 if dx > 0 else n - 3]
        doors = moved
    for r in range(n):
        for c in range(2, n - 2):
            if doors[r][c] != ".":
                rows[r][c] = doors[r][c]
    for c, r in ((0, 9), (0, 10), (n - 1, 22), (n - 1, 23)):     # chipped jambs (the tile is never fully opaque)
        rows[r][c] = "."
    if state in ("shake_0", "shake_1"):
        i = 0 if state == "shake_0" else 1
        for (c, r), k in zip(((9, 8), (10, 9), (22, 23), (23, 23), (22, 24))[i * 2:], ("5", "4", "5")):
            rows[r][c] = k
    return ad.despeckle(rows)


BUFFER_CANVAS = (16, 24)
BUFFER_PIVOT = (0, 23)


def buffer_stop() -> list[list[str]]:
    """The drawn buffer stop (16 x 24): an iron foot, a timber block with an S/0 hazard band, and an 8 px warning
    post with a hazard cap. Never a solid (DZ-AD4)."""
    w, h = BUFFER_CANVAS
    rs = ad.Raster(w, h, 0, h - 1, 1.0)
    foot = rs.rect(0, 0, 16, 4, IRON, mode="cyl_h", outline=True)
    block = rs.rect(1, 4, 15, 16, COPPER, mode="cyl_v", outline=True)
    post = rs.rect(6, 16, 10, 24, IRON, mode="cyl_v", outline=True)
    rs.render()
    block_set, post_set = {block}, {post}
    for c in range(12):                                          # hazard band across the block
        for r in range(5):
            rs.put(2 + c + 0.5, 8 + r + 0.5, "S" if ((c + r) // 2) % 2 == 0 else "0", block_set)
    for x in (4.0, 11.0):                                        # timber grain
        rs.vline(x, 5, 7, "c", block_set)
        rs.vline(x, 13, 15, "c", block_set)
    for c in range(4):                                           # hazard cap on the post
        for r in range(3):
            rs.put(6 + c + 0.5, 19 + r + 0.5, "S" if (c + r) % 2 == 0 else "0", post_set)
    rs.put(1.5, 1.5, "7", {foot}); rs.put(14.5, 1.5, "7", {foot})
    return ad.despeckle(rs.key)


LINTEL_TILE = 32


def lintel_tile(kind: str) -> list[list[str]]:
    """The tunnel mouth's lintel (32 x 32 tiles, tileable along x): ``mouth`` is the bottom course (stone face,
    a timber header with rivets, then 8 px of inner shadow in ``K``), ``fill`` the stone courses above it."""
    n = LINTEL_TILE
    rows = [["."] * n for _ in range(n)]

    def stone(r0: int, r1: int, offset: int) -> None:
        for r in range(r0, r1):
            for c in range(n):
                k = "3" if (c + r) % 5 else "4"
                if r in (r0, r0 + 1):
                    k = "5" if r == r0 else "4"
                if r == r1 - 1:
                    k = "1"
                if (c - offset) % 16 in (0, 15):
                    k = "0" if (c - offset) % 16 == 0 else "2"
                rows[r][c] = k
    if kind == "fill":
        stone(0, 16, 0)
        stone(16, 32, 8)
        for c, r in ((5, 6), (6, 6), (21, 9), (22, 9), (12, 23), (13, 23), (28, 27)):
            rows[r][c] = "2"
        for c, r in ((9, 3), (25, 22)):
            rows[r][c] = "5"
        for c, r in ((20, 5), (21, 5), (21, 6)):                 # a chipped stone (the tile is never fully opaque)
            rows[r][c] = "."
        return rows
    stone(0, 16, 0)
    for r in range(16, 24):                                      # the timber header
        for c in range(n):
            rows[r][c] = {16: "K", 17: "i", 18: "h", 19: "h", 20: "g", 21: "g", 22: "f", 23: "K"}[r]
    for c in range(3, n, 8):
        rows[19][c] = "7"
        rows[20][c] = "N"
    for c in range(n):                                           # the mouth's inner shadow, 8 px, K fading to iron-0
        for r in range(24, 32):
            rows[r][c] = "K" if r < 29 or (c + r) % 2 == 0 else ("0" if r < 31 else ".")
    return rows


HAUL_CANVAS = (12, 12)
HAUL_PIVOT = (0, 11)


def haul_tile(kind: str) -> list[list[str]]:
    """The haul line (execution of DZ2-AD6): ``h`` an 8 px run of horizontal chain and ``v`` an 8 px run of
    vertical chain (the authoring chain pattern, unoutlined, 8 x 8 of content in the top left of the 12 x 12
    atlas cell), ``idler`` the 12 x 12 brass idler pulley at the corner above the brace."""
    rows = [["."] * 12 for _ in range(12)]
    if kind == "h":
        chain_h(rows, 5, 0, 7, phase=0)
    elif kind == "v":
        chain_v(rows, 5, 0, 7, phase=0)
    else:
        rs = ad.Raster(12, 12, 0, 11, 1.0)
        rs.disc(6.0, 6.0, 5.4, BRASS, outline=True)
        rs.render()
        rs.put(6.0, 6.0, "L")
        rs.put(5.0, 7.0, "Z")
        rows = [r[:] for r in rs.key]
        rows[5][6] = rows[6][5] = "K"
    return rows


# ------------------------------------------------------------------ the crew (execution, section 5)

CREW_CANVAS = ad.RIDER_CANVAS            # (64, 56)
TUMBLE_PIVOT = (32, 55)                  # bottom centre
TUMBLE_GOB_FRAMES = (16, 19, 20)         # GOB-01: hurt (thrown), knockback (airborne, mirrored), down (on its back)


def mirror_x(rows: list[list[str]]) -> list[list[str]]:
    return [r[::-1] for r in rows]


def tumble_frame(variant: str, i: int) -> list[list[str]]:
    """``tumble_0`` thrown out of the seat (GOB-01 hurt: head back, arms out); ``tumble_1`` mid-air, turned about 90
    degrees head first (GOB-01 knockback, mirrored so he flies forward); ``tumble_2`` on his back on the floor
    (GOB-01 down). The GOB-01 pixels are used whole: no rotation, no resampling, no new pixels."""
    rows = [r[:] for r in ad._goblin(variant, TUMBLE_GOB_FRAMES[i])]
    return mirror_x(rows) if i == 1 else rows


def engineer_slump(i: int) -> list[list[str]]:
    """The driver slumped (execution): the ``engineer_hold`` build with the head dropped 2 px forward and 1 px down,
    the pot slid down over his eyes (no eye pixel shows), the arms off the levers; ``engineer_slump_1`` lifts the
    chest and head 1 px (breathing)."""
    rows = ad._canvas()
    torso = ad.torso_px("a", weapon=False)
    head = ad.head_px("a", "grin")
    ad._paint(rows, torso, 0, 1)
    ad._paint(rows, head, 2, 3)
    ad._paint(rows, ad.pot_px(lift=-6, dx=2), 0, 0)
    rows = ad.despeckle(rows)
    if i == 1:
        out = [r[:] for r in rows]
        for y in range(46, -1, -1):
            out[y] = rows[y + 1][:] if y + 1 < len(rows) else out[y]
        out[47] = rows[47][:]
        return out
    return rows


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
                              "cleat holding the haul chain (intact), the weakened and the snapped states"),
    "prop_stop_frame_shadow": ("prop_stop_frame_shadow_v0", "prop_animation", PROP_DIR,
                               "DOZ-02 floor shadow of the stop frame's warning, 80 x 4: a dark ellipse in four sizes "
                               "(40 x 1 to 80 x 4)"),
    "prop_stop_frame_glance": ("prop_stop_frame_glance_v0", "prop_animation", PROP_DIR,
                               "DOZ-02 rear glance, 88 x 88: the press beam striking the rear deck, tipping off "
                               "behind the exhaust (0, 15 and 35 degrees, redrawn column by column) and its shadow"),
    "env_dozer_gate": ("env_dozer_gate_v0", "prop_animation", ENV_DIR,
                       "DOZ-02 rear gate of room 3, one 32 x 32 tile stacked up the lane: closed, shaking and burst"),
    "env_dozer_buffer": ("env_dozer_buffer_v0", "prop_animation", ENV_DIR,
                         "DOZ-02 drawn buffer stop of room 3, 16 x 24 (never a solid)"),
    "env_dozer_lintel": ("env_dozer_lintel_v0", "prop_animation", ENV_DIR,
                         "DOZ-02 tunnel mouth lintel tiles, 32 x 32: the mouth course and the stone fill above it"),
    "env_dozer_haul": ("env_dozer_haul_v0", "prop_animation", ENV_DIR,
                       "DOZ-02 haul line of the stop frame: 8 px chain runs (h, v) and the 12 px idler pulley"),
    "dozer_rider_tumble_h120": ("dozer_rider_tumble_h120_v0", "vehicle_part", DOZER_DIR,
                                "DOZ-02 riders A and B thrown out of the tub: three poses each, GOB-01 pixels whole"),
    "dozer_engineer_slump_h120": ("dozer_engineer_slump_h120_v0", "vehicle_part", DOZER_DIR,
                                  "DOZ-02 the driver slumped in the disabled cab: pot over his eyes, two breathing "
                                  "frames"),
}
FRAME_NAMES = ("pitch_0", "pitch_1", "disabled")
FRAME_TICKS: dict[str, list[int | None]] = {
    "dozer_wheel_jammed_h120": [HOLD],
    "dozer_hull_damage_h120": [6, 6, HOLD],
    "dozer_front_cab_damage_h120": [6, 6, HOLD],
    "dozer_front_tub_damage_h120": [6, 6, HOLD],
    "dozer_yoke_damage_h120": [HOLD],
    "prop_stop_frame": [HOLD, HOLD, 3, 3, 8],
    "prop_stop_frame_remnant": [HOLD, HOLD],
    "prop_stop_frame_brace": [HOLD, HOLD, 8, 8],
    "prop_stop_frame_shadow": [8, 8, 7, 7],
    "prop_stop_frame_glance": [4, 4, 6, HOLD],
    "env_dozer_gate": [HOLD, 3, 3, HOLD],
    "env_dozer_buffer": [HOLD],
    "env_dozer_lintel": [HOLD, HOLD],
    "env_dozer_haul": [HOLD, HOLD, HOLD],
    "dozer_rider_tumble_h120": [6, 18, 12, 6, 18, 12],
    "dozer_engineer_slump_h120": [30, 30],
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
    grids["prop_stop_frame"] = _grid("prop_stop_frame", "frame",
                                     [stop_frame_raised(), stop_frame_pulley(), stop_frame_warn(0),
                                      stop_frame_warn(1), stop_frame_falling()],
                                     FRAME_CANVAS, FRAME_PIVOT, ["raised", "pulley", "warn_0", "warn_1", "falling"])
    grids["prop_stop_frame_remnant"] = _grid("prop_stop_frame_remnant", "remnant", [remnant(True), remnant(False)],
                                             REMNANT_CANVAS, REMNANT_PIVOT, ["landed", "landed_hit"])
    grids["prop_stop_frame_brace"] = _grid(
        "prop_stop_frame_brace", "brace",
        [brace_intact()] + [brace_state(t) for t in ("weakened", "snapped", "snapped_swing")], BRACE_CANVAS,
        BRACE_PIVOT, ["intact", "weakened", "snapped", "snapped_swing"])
    grids["prop_stop_frame_shadow"] = _grid("prop_stop_frame_shadow", "shadow", [shadow_frame(i) for i in range(4)],
                                            SHADOW_CANVAS, SHADOW_PIVOT, ["s0", "s1", "s2", "s3"])
    grids["prop_stop_frame_glance"] = _grid(
        "prop_stop_frame_glance", "glance",
        [glance_beam(d, GLANCE_DROP0 if d == 0 else 0) for d in GLANCE_DEG] + [glance_shadow()], GLANCE_CANVAS,
        GLANCE_PIVOT, ["glance_0", "glance_1", "glance_2", "shadow"])
    gate_tags = ["closed", "shake_0", "shake_1", "burst"]
    grids["env_dozer_gate"] = _grid("env_dozer_gate", "gate", [gate_tile(t) for t in gate_tags],
                                    (GATE_TILE, GATE_TILE), (0, GATE_TILE - 1), gate_tags)
    grids["env_dozer_buffer"] = _grid("env_dozer_buffer", "buffer", [buffer_stop()], BUFFER_CANVAS, BUFFER_PIVOT,
                                      ["idle"])
    grids["env_dozer_lintel"] = _grid("env_dozer_lintel", "lintel", [lintel_tile("mouth"), lintel_tile("fill")],
                                      (LINTEL_TILE, LINTEL_TILE), (0, LINTEL_TILE - 1), ["mouth", "fill"])
    grids["env_dozer_haul"] = _grid("env_dozer_haul", "haul", [haul_tile(t) for t in ("h", "v", "idler")],
                                    HAUL_CANVAS, HAUL_PIVOT, ["h", "v", "idler"])
    grids["dozer_rider_tumble_h120"] = _grid(
        "dozer_rider_tumble_h120", "riders", [tumble_frame(v, i) for v in "ab" for i in range(3)], CREW_CANVAS,
        TUMBLE_PIVOT, [f"rider_{v}_tumble_{i}" for v in "ab" for i in range(3)])
    grids["dozer_engineer_slump_h120"] = _grid("dozer_engineer_slump_h120", "engineer",
                                               [engineer_slump(0), engineer_slump(1)], CREW_CANVAS,
                                               ad.RIDER_PIVOT, ["engineer_slump_0", "engineer_slump_1"])
    return grids


# ------------------------------------------------------------------ damage-state data contract

def damage_contract(grids: dict[str, dict]) -> dict:
    fronts = {n: front_damage_frames(n) for n in ("front_cab", "front_tub")}
    seats = {name: [round(x - ad.O_X), round(y)] for name, (x, y) in ad.SEATS.items()}

    def seat_dy(amp):
        return {name: pitch_dy(x, amp) for name, (x, _y) in seats.items()}
    return {
        "format": "dozer-damage-v0",
        "status": "DOZ-02 design keys and execution frames, wired into room 3's presenter (src/dozer_art.rs); "
                  "independent review pending",
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
            "frames": {"raised": "Supported, Weakened", "pulley": "every state, at the raised anchor",
                       "warn_0": "Released, first 3 ticks of each 6 (shifted 1 px left)",
                       "warn_1": "Released, last 3 ticks of each 6 (shifted 1 px right)",
                       "falling": "Falling (8 ticks)"},
            "warning": {"ticks": 30, "alternate_ticks": 3, "frames": ["warn_0", "warn_1"],
                        "shadow": {"atlas": "prop_stop_frame_shadow", "pivot": list(SHADOW_PIVOT),
                                   "frames": ["s0", "s1", "s2", "s3"], "ticks_each": [8, 8, 7, 7],
                                   "size_px": [[max(w), len(w)] for w in SHADOW_ROWS],
                                   "alpha": [0.3, 0.6], "anchor": "the footprint's bottom-left pixel (80 x 4 canvas)"}},
            "falling": {"ticks": 8, "pulley": "stays at the raised anchor"},
            "glance": {"atlas": "prop_stop_frame_glance", "pivot": list(GLANCE_PIVOT),
                       "frame_size": list(GLANCE_CANVAS), "anchor": "the beam centre on the rear deck contact point",
                       "anchor_rig": list(GLANCE_ANCHOR_RIG),
                       "frames": [{"tag": "glance_0", "ticks": 4, "offset": list(GLANCE_OFFSETS[0]), "deg": 0,
                                   "fx": "vfx_hit_spark_metal at the contact on the deck"},
                                  {"tag": "glance_1", "ticks": 4, "offset": list(GLANCE_OFFSETS[1]), "deg": -15},
                                  {"tag": "glance_2", "ticks": 6, "offset": list(GLANCE_OFFSETS[2]), "deg": -35}],
                       "shadow": "the shadow frame under the beam, alpha 0.45",
                       "then": "the remnant landed drops in behind the machine; the hull shows no damage overlay",
                       "method": "the beam redrawn column by column (no resampling; the lengthwise squeeze drops "
                                 "columns, a rivet column is never dropped)"},
            "remnant": {"atlas": "prop_stop_frame_remnant", "pivot": list(REMNANT_PIVOT),
                        "anchor": "the remnant rectangle's bottom-left pixel",
                        "frames": {"landed": "early miss and rear glance", "landed_hit": "after a hit (the bar is in "
                                   "the wheel)"}},
            "brace": {"atlas": "prop_stop_frame_brace", "pivot": list(BRACE_PIVOT),
                      "anchor": "the prop target rectangle's bottom-left pixel",
                      "frames": {"intact": "Supported", "weakened": "Weakened (one hit of two)",
                                 "snapped": "Released onward", "snapped_swing": "Released onward, alternate"},
                      "swing": {"ticks_each": 8, "for_ticks": 48, "then": "snapped (still)"}},
        },
        "room_pieces": {
            "gate": {"atlas": "env_dozer_gate", "tile_px": GATE_TILE, "stack": "tiles from the walking line up to the "
                     "lane ceiling (228 px; the last tile is cropped)", "x": [0, 32],
                     "frames": {"closed": "dormant and every state before the rumble", "shake_0": "approaching, "
                                "alternate with shake_1 every 3 ticks", "shake_1": "approaching",
                                "burst": "from the emergence on"}, "solid": False},
            "buffer": {"atlas": "env_dozer_buffer", "size_px": list(BUFFER_CANVAS), "x": [1008, 1024],
                       "anchor": "the pivot pixel on the walking line at x 1008", "solid": False},
            "lintel": {"atlas": "env_dozer_lintel", "tile_px": LINTEL_TILE, "frames": ["mouth", "fill"],
                       "clear_px": 112, "inner_shadow_px": 8,
                       "wired": "no (the room's solid lintel is drawn by the Floor 1 kit)"},
            "haul": {"atlas": "env_dozer_haul", "frame_px": list(HAUL_CANVAS), "content_px": 8,
                     "frames": {"h": "horizontal run", "v": "vertical run", "idler": "corner pulley above the brace"},
                     "route": "the pulley stub along the ceiling to the idler over the brace, then down to the horn"},
        },
        "crew": {
            "tumble": {"atlas": "dozer_rider_tumble_h120", "pivot": list(TUMBLE_PIVOT),
                       "frames": {"rider_a": ["rider_a_tumble_0", "rider_a_tumble_1", "rider_a_tumble_2"],
                                  "rider_b": ["rider_b_tumble_0", "rider_b_tumble_1", "rider_b_tumble_2"]},
                       "start_tick": {"rider_a": 12, "rider_b": 16},
                       "airborne_ticks": 24, "arc_up_px": 12, "tumble_0_ticks": 6,
                       "design_forward_px": {"rider_a": 28, "rider_b": 36},
                       "runtime_landing": "the arc ends at the wheel front plus the roster's dzq20_land_dx (A 32, "
                                          "B 72), where the simulation's goblin lands on the disable tick (DZ2-Q1)",
                       "then": "tumble_2 held on the floor until the disable tick; the simulation's goblin takes over"},
            "slump": {"atlas": "dozer_engineer_slump_h120", "pivot": list(ad.RIDER_PIVOT),
                      "frames": ["engineer_slump_0", "engineer_slump_1"], "ticks_each": 30,
                      "from": "impact_stunned tick 12 and every disabled tick (loop)",
                      "before": "engineer_lurch_forward on ticks 0..11"},
        },
    }



# ------------------------------------------------------------------ manifests

LICENSE = ad.LICENSE
GENERATOR = ("Design and hero keys: DOZ-02 design step; the remaining frames: DOZ-02 execution step; both the "
             "deterministic output of scripts/author_dozer_doz02.py built on the DOZ-01 construction "
             "(scripts/author_dozer_doz01.py) and the GOB-01 goblin pixels (crew frames). No image-model raster "
             "output, no tracing, no resampling. Independent review pending")


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
        "dozer": {"status": "DOZ-02 design keys and execution frames, wired into room 3's presenter "
                            "(src/dozer_art.rs); independent review pending",
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
# The unoutlined chain pattern (CHAIN_V) leaves 6 single-key pixels per 4 px link repeat; the 8 px haul tiles hold
# two repeats (DZ2-AD10). Every other DOZ-02 source keeps the limit of 9.
MAX_ORPHANS = {"env_dozer_haul": 12}


def validator_config(stem: str, grid: dict) -> dict:
    """``pixelart-asset-v1`` config for ``python3 -m scripts.pixelart validate --config``."""
    folder = STEMS[stem][2].relative_to(ROOT).as_posix()
    tags = {t["name"]: {"playback": "once", "contact": "free",
                        "frames": [[t["from"], FRAME_TICKS[stem][t["from"]] or round(ad.HOLD_MS * TICK_HZ / 1000)]]}
            for t in grid["tags"]}
    return {"format": "pixelart-asset-v1", "asset_id": STEMS[stem][0],
            "title": f"DOZ-02 {stem}",
            "grid": f"{folder}/{stem}.grid.json", "manifest": f"assets/manifests/{stem}.json",
            "frame_size": grid["canvas"], "pivot": grid["pivot"], "tick_hz": TICK_HZ, "hold_ms": ad.HOLD_MS,
            "tags": tags,
            "rules": {"max_colors": MAX_COLORS.get(stem, 24), "max_isolated_per_frame": 0,
                      "max_orphans_per_frame": MAX_ORPHANS.get(stem, 9), "unique_frames": True}}


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
