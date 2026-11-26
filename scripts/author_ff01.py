#!/usr/bin/env python3
"""Authoring record for AREA-FASTFOOD-01: the fast-food refuge kit of Floor 01 room 1, ``rest_refuge``.

Draws the frozen design keys of ``docs/02-art/area-fastfood-01-spec.md`` on ``floor01-catacomb-v2``
(no new colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/ff_staff_door.grid.json``  ``ff_staff_door_closed`` and ``ff_staff_door_open``
  (hero key, spec section 6; one 24 x 228 canvas, two frames)
- ``assets/source/props/floor01/ff_reward_box.grid.json``  ``ff_reward_box_full_a``, ``_full_b`` and
  ``ff_reward_box_empty`` (the Light on Your Feet pickup, spec section 7; one 32 x 24 canvas, three frames)
- ``assets/source/props/floor01/ff_samples.grid.json``     ``ff_pass_ledge`` (96 x 16), ``ff_wainscot_a``
  (32 x 32) and ``ff_floor_checker_a`` (32 x 12): the language samples of spec section 8

Execution adds the remaining pieces of spec section 5 as new functions and new atlases, and must not redraw a
key (a key change is a design change). Every pixel comes from rules (ramps, a Bayer threshold, a hash noise);
nothing is traced or resampled from a concept image. No text, logos, brands or likenesses.

    python3 scripts/author_ff01.py              # the three key grids + .ase
    python3 scripts/author_ff01.py gate DIR      # silhouette gate sheets (1x and 4x, scene composites)
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

BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("ff01",) + key)).encode()).digest()
    return h[0] / 255.0


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


def bayer(x: int, y: int) -> float:
    return (BAYER[y % 4][x % 4] + 0.5) / 16.0


def vnoise(x: float, y: float, cell_x: float, cell_y: float, seed: str) -> float:
    """Smooth value noise over a hashed lattice (anisotropic cells: long in y gives vertical grain)."""
    gx, gy = x / cell_x, y / cell_y
    x0, y0 = math.floor(gx), math.floor(gy)
    fx, fy = gx - x0, gy - y0
    fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
    a, b = rnd(seed, x0, y0), rnd(seed, x0 + 1, y0)
    c, d = rnd(seed, x0, y0 + 1), rnd(seed, x0 + 1, y0 + 1)
    return (a + (b - a) * fx) * (1 - fy) + (c + (d - c) * fx) * fy


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

    def copy(self) -> "Pix":
        c = Pix(self.w, self.h)
        c.px = [row[:] for row in self.px]
        return c


def _steel(x: int, y: int, base: float, seed: str) -> float:
    """Brushed steel on the ``gal`` ramp: a per-column grain plus long vertical streaks (never horizontal
    bands, so a tall panel never reads as a ladder)."""
    grain = (rnd(seed, "col", x) - 0.5) * 1.1
    streak = 0.9 if vnoise(x * 5.0, y, 1.0, 22.0, seed + "s") > 0.74 else 0.0
    v = base + grain + streak
    return math.floor(v + bayer(x, y) - 0.5 + 0.5)


# =============================================================================================
# Hero key: the staff door (spec section 6). One 24 x 228 canvas for both states, placed top-left at
# world (1436, 352): the lock's solid 16 x 228 at x 1440..1455, y 352..579 is canvas x 4..19, rows 0..227
# (row 0 is the ceiling slab's underside + 0, row 227 the walking line's last pixel row, y 579). The panel
# fills the solid exactly; the jamb rails x 0..3 and 20..23 stand outside it in both states (the frame).
# =============================================================================================
DOOR_W, DOOR_H = 24, 228
PANEL = (4, 19)
SEAMS = (76, 152)                  # two section joints only (y 428, 504): a sectional panel, not rungs
PORTHOLE = (11.5, 176.0, 5.2)      # centre x, centre row (y 528, Carl's head band), outer radius
PUSH = (13, 16, 192, 203)          # push plate x0, x1, row0, row1 (hand height)
KICK = 208                         # kick plate from row 208 (y 560, the floor back line) to 227
LAMP = (21, 22, 112, 116)          # status lamp on the right jamb: cap row, three lit rows, cap row
OPEN_SHIFT = 224                   # the open panel is the closed panel raised 224 px: a 4-row lip stays
SILL = 226                         # floor track rows 226..227 under the panel (visible once open)
RISE = (19, 37, 59, 85, 112, 139, 165, 188, 205, 217, 222, 224)   # raise on ticks 1..12 after the open event


def _rails(c: Pix) -> None:
    for y in range(DOOR_H):
        c.put(0, y, "stone_cold", 0)
        c.put(1, y, "gal", 5)
        c.put(2, y, "gal", 7)
        c.put(3, y, "gal", 1)
        c.put(20, y, "gal", 1)
        c.put(21, y, "gal", 6)
        c.put(22, y, "gal", 4)
        c.put(23, y, "stone_cold", 0)
    for y in range(18, DOOR_H - 8, 38):            # rail bolts
        c.put(2, y, "gal", 9)
        c.put(21, y, "gal", 9)
    for x0 in (0, 20):                               # floor shoes
        c.box(x0, 224, x0 + 3, 227, "gal", 3)
        for x in range(x0, x0 + 4):
            c.put(x, 224, "gal", 8)
        c.put(x0, 227, "stone_cold", 0)
        c.put(x0 + 3, 227, "stone_cold", 0)


def _lamp(c: Pix, lit: bool) -> None:
    x0, x1, y0, y1 = LAMP
    for x in range(x0, x1 + 1):
        c.put(x, y0, "iron", 1)
        c.put(x, y1, "iron", 1)
    for y in range(y0 + 1, y1):
        if lit:                                      # closed: a red lamp
            c.put(x0, y, "flame", 2 if y == y0 + 1 else 1)
            c.put(x1, y, "flame", 1 if y == y0 + 1 else 0)
        else:                                        # open: a dim green lamp
            c.put(x0, y, "lichen", 2)
            c.put(x1, y, "lichen", 1)


def _panel() -> Pix:
    """The closed panel alone on the door canvas (cols 4..19, rows 0..227)."""
    c = Pix(DOOR_W, DOOR_H)
    x0, x1 = PANEL
    for y in range(DOOR_H):
        base = 6.4 - 1.3 * (y / DOOR_H)             # lit from the strips above: a little lighter at the top
        for x in range(x0 + 1, x1):
            c.put(x, y, "gal", clamp(_steel(x, y, base, "door"), 4, 8))
        c.put(x0, y, "gal", 8)                       # lit left edge
        c.put(x0 + 1, y, "gal", clamp(_steel(x0 + 1, y, base + 0.5, "door"), 5, 8))
        c.put(x1, y, "gal", 3)                       # shade edge
        c.put(x1 - 1, y, "gal", clamp(_steel(x1 - 1, y, base - 0.6, "door"), 4, 7))
    for x in range(x0, x1 + 1):                      # emerging from the ceiling: its shadow
        c.put(x, 0, "gal", 2)
        c.put(x, 1, "gal", 4)
    for s in SEAMS:                                  # two section joints
        for x in range(x0 + 1, x1):
            c.put(x, s, "gal", 2)
            c.put(x, s + 1, "gal", 8)
    cx, cy, r = PORTHOLE                             # the porthole: chrome rim, dark glass, one reflection
    for y in range(int(cy - r - 2), int(cy + r + 2)):
        for x in range(x0, x1 + 1):
            dx, dy = x - cx, y - cy
            d = math.hypot(dx, dy)
            if d <= 3.5:
                v = 1
                if -2.2 <= dx + dy <= -0.6:
                    v = 5
                c.put(x, y, "gal", v)
            elif d <= r:
                c.put(x, y, "gal", 10 if dx + dy < -1.0 else (7 if dx + dy < 2.0 else 4))
            elif d <= r + 0.9:
                c.put(x, y, "gal", 3)
    c.put(int(cx - 2), int(cy - 2), "gal", 9)        # the glint
    px0, px1, py0, py1 = PUSH                        # push plate
    for y in range(py0, py1 + 1):
        for x in range(px0, px1 + 1):
            c.put(x, y, "gal", 8)
        c.put(px1, y, "gal", 5)
    for x in range(px0, px1 + 1):
        c.put(x, py0, "gal", 10)
        c.put(x, py1, "gal", 4)
    for y in range(KICK, DOOR_H):                    # kick plate, scuffed
        for x in range(x0 + 1, x1):
            v = 4
            if y == KICK:
                v = 9
            elif y == KICK + 1:
                v = 6
            elif y >= DOOR_H - 2:
                v = 2 if y == DOOR_H - 2 else 1
            elif (x + 2 * y) % 7 == 0 and rnd("scuff", x, y) > 0.45:
                v = 3
            c.put(x, y, "gal", v)
    return c


def _with_rails(panel_rows: Pix, lit: bool, sill: bool) -> Pix:
    c = Pix(DOOR_W, DOOR_H)
    _rails(c)
    if sill:
        for x in range(PANEL[0], PANEL[1] + 1):
            c.put(x, SILL, "gal", 5)
            c.put(x, SILL + 1, "gal", 2)
    for y in range(DOOR_H):
        for x in range(PANEL[0], PANEL[1] + 1):
            p = panel_rows.get(x, y)
            if p is not None:
                c.px[y][x] = p
    _lamp(c, lit)
    return c


def door_raised(d: int) -> Pix:
    """The door with its panel raised by ``d`` px (0 = closed, ``OPEN_SHIFT`` = open). Every rising frame of
    spec section 6 is this exact crop: no drawn in-between."""
    panel = _panel()
    shifted = Pix(DOOR_W, DOOR_H)
    for y in range(DOOR_H):
        if y + d < DOOR_H:
            shifted.px[y] = panel.px[y + d][:]
    return _with_rails(shifted, lit=(d == 0), sill=(d > 0))


def ff_staff_door_closed() -> Pix:
    return door_raised(0)


def ff_staff_door_open() -> Pix:
    return door_raised(OPEN_SHIFT)


# =============================================================================================
# Hero key: the reward box, the Light on Your Feet pickup (spec section 7). One 32 x 24 canvas for the three
# frames, placed top-left at world (1204, 556): bottom row 23 on the walking line (y 579), centred on the
# grant column x 1200..1240 (centre 1220 = canvas x 16). Drawn under the actors.
# =============================================================================================
BOX_W, BOX_H = 32, 24
BODY_X = (3, 28)
STRAPS = (8, 22)


def _box_body(c: Pix, top: int, dim: int) -> None:
    """The gilded body from row ``top`` to row 22 and its contact line on row 23."""
    x0, x1 = BODY_X
    for y in range(top, 23):
        t = (y - top) / max(1, 22 - top)
        for x in range(x0, x1 + 1):
            v = 4 - dim - (1 if t + 0.35 * bayer(x, y) > 0.82 else 0)
            c.put(x, y, "brass", v)
        c.put(x0, y, "brass", 5 - dim)
        c.put(x1, y, "brass", 2)
        for sx in STRAPS:                            # raised gold bands
            c.put(sx, y, "brass", 5 - dim)
            c.put(sx + 1, y, "brass", 3 - dim)
    band = top + 4                                   # the dark iron band with two rivets
    for x in range(x0, x1 + 1):
        c.put(x, band, "iron", 3)
        c.put(x, band + 1, "iron", 2)
    c.put(x0 + 3, band, "brass", 6 - dim)
    c.put(x1 - 3, band, "brass", 6 - dim)
    for x in range(x0, x1 + 1):                      # bottom row and contact line
        c.put(x, 22, "brass", 2)
        c.put(x, 23, "iron", 1)
    for x in (x0, x1):
        c.put(x, 22, "iron", 3)


def _box_lid_closed(c: Pix) -> None:
    for x in range(3, 29):
        c.put(x, 5, "flame", 4)                      # the lid's lit top edge: the box's brightest line
    for y in (6, 7):
        for x in range(2, 30):
            c.put(x, y, "brass", 6)
    for x in range(3, 11):
        c.put(x, 6, "flame", 4)                      # a sheen on the lid top
    for y in (8, 9, 10):
        for x in range(2, 30):
            c.put(x, y, "brass", 5 if y == 8 else 4)
    for x in range(2, 30):
        c.put(x, 11, "brass", 2)                     # shadow under the lid lip
    for y in range(6, 11):
        c.put(2, y, "brass", 5)
        c.put(29, y, "brass", 2)
        for sx in STRAPS:
            c.put(sx, y, "brass", 6 if y < 8 else 5)
            c.put(sx + 1, y, "brass", 4 if y < 8 else 3)


def _clasp(c: Pix, y0: int) -> None:
    for y in range(y0, y0 + 5):
        for x in range(14, 18):
            edge = x in (14, 17) or y in (y0, y0 + 4)
            c.put(x, y, "iron", 1) if edge else c.put(x, y, "brass", 6)
    c.put(15, y0 + 2, "iron", 0)                     # keyhole


def _sparkle(c: Pix, x: int, y: int, big: bool) -> None:
    c.put(x, y, "flame", 5)
    for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        c.put(x + dx, y + dy, "brass", 6)
    if big:
        for dx, dy in ((-2, 0), (2, 0), (0, -2), (0, 2)):
            c.put(x + dx, y + dy, "flame", 4)


def _box_full() -> Pix:
    c = Pix(BOX_W, BOX_H)
    _box_body(c, 12, 0)
    _box_lid_closed(c)
    _clasp(c, 10)
    return c


def ff_reward_box_full_a() -> Pix:
    c = _box_full()
    _sparkle(c, 5, 2, big=True)
    return c


def ff_reward_box_full_b() -> Pix:
    c = _box_full()
    _sparkle(c, 26, 3, big=False)
    c.put(4, 13, "flame", 4)
    return c


def ff_reward_box_empty() -> Pix:
    """Opened and spent: the lid stands up behind the box showing its shadowed underside, the dark empty
    inside shows over the front rim, the clasp hangs open, the gold is one step duller, no sparkle."""
    c = Pix(BOX_W, BOX_H)
    _box_body(c, 14, 1)
    for x in range(4, 28):                           # the raised lid's underside
        c.put(x, 0, "brass", 4)
        for y in range(1, 11):
            c.put(x, y, "brass", 2 if 4 < x < 27 else 3)
        c.put(x, 11, "iron", 1)                      # hinge line
    for y in range(1, 11):
        for sx in STRAPS:
            c.put(sx, y, "brass", 3)
    for x in range(4, 28):                           # the empty inside, then the lit front rim
        c.put(x, 12, "iron", 0)
        c.put(x, 13, "iron", 1)
        c.put(x, 14, "brass", 5)
    c.put(3, 12, "brass", 3)
    c.put(28, 12, "brass", 2)
    c.put(3, 13, "brass", 3)
    c.put(28, 13, "brass", 2)
    for y in range(15, 19):                          # the clasp hangs open
        c.put(15, y, "iron", 2)
        c.put(16, y, "brass", 4)
    c.put(15, 18, "iron", 1)
    c.put(16, 18, "iron", 1)
    return c


# =============================================================================================
# Language samples (spec section 8): the pass-through ledge skin over the staff shelf, the tiled wainscot,
# the checker floor of the deck's back half.
# =============================================================================================
def ff_pass_ledge() -> Pix:
    """96 x 20 at world (1296, 448): the kit's plank tile of the shelf (grid y 448, art rows 448..463, lit row
    449) is a collidable tile that cannot be removed, so rows 0..15 are opaque across the width and cover it
    exactly. Rows 0..3 are the steel top face Donut stands in (her feet end on y 451, as on every kit plank);
    row 1 (y 449) is the stand line in ``flame`` 5, the plank's own lit row. Rows 4..15 are the fascia (the
    yellow pinstripe on row 9) and the dark underside; rows 16..19 hold two bracket straps only."""
    c = Pix(96, 20)
    for x in range(96):
        c.put(x, 0, "gal", 7)
        c.put(x, 1, "flame", 5)
        c.put(x, 2, "gal", 10)
        c.put(x, 3, "gal", clamp(_steel(x, 3, 9.0, "ledge"), 8, 10))
        c.put(x, 4, "gal", 3)
        for y in (5, 6, 7):
            c.put(x, y, "gal", clamp(_steel(x, y, 7.4, "ledge"), 6, 8))
        c.put(x, 8, "gal", 5)
        c.put(x, 9, "brass", 5)                      # the yellow pinstripe of the fascia
        c.put(x, 10, "gal", 5)
        c.put(x, 11, "gal", 4)
        c.put(x, 12, "gal", 2)
        for y in (13, 14):
            c.put(x, y, "gal", 1)
        c.put(x, 15, "stone_cold", 0)
    for x in (0, 95):                                # end caps
        for y in range(1, 15):
            c.put(x, y, "gal", 3 if x == 95 else 6)
    for bx in (10, 83):                              # bracket straps to the wall
        for y in range(13, 20):
            c.put(bx, y, "gal", 6)
            c.put(bx + 1, y, "gal", 5)
            c.put(bx + 2, y, "gal", 3)
        c.put(bx, 19, "gal", 4)
    return c


def ff_wainscot_a() -> Pix:
    """32 x 32 at y 528 (rows 16..31 over the base course), x = 32 n: a blue tile field under a blue trim.
    No white here: a bright horizontal line at y 528 would run behind the play deck tops (spec 9.1)."""
    c = Pix(32, 32)
    for x in range(32):
        c.put(x, 0, "gal", 10)
        c.put(x, 1, "gal", 8)
        c.put(x, 2, "gal", 4)
        for y in range(3, 32):
            tx, ty = x % 8, (y - 3) % 8
            tile = (x // 8, (y - 3) // 8)
            if tx == 7 or ty == 7:
                v = 3
            elif tx == 0 or ty == 0:
                v = 9
            else:
                v = 7 if rnd("tile", *tile) > 0.25 else 8
                if tx == 6 and ty == 6 and rnd("chip", *tile) > 0.7:
                    v = 5
            c.put(x, y, "gal", v)
    return c


def ff_floor_checker_a() -> Pix:
    """32 x 12 at y 561 (the deck's back half), x = 32 n: two rows of 7 x 5 tiles with 1 px grout; the
    pale squares are worn cream ``stone`` 10/9 (luma 201, kept well under the ``flame`` 5 stand lines),
    the dark ones ``gal`` 6/5."""
    c = Pix(32, 12)
    for y in range(12):
        row = y // 6
        for x in range(32):
            tx, ty = x % 8, y % 6
            pale = ((x // 8) + row) % 2 == 0
            if tx == 7 or ty == 5:
                c.put(x, y, "gal", 4)
            elif pale:
                c.put(x, y, "stone", 10 if ty < 4 else 9)
            else:
                c.put(x, y, "gal", 6 if ty < 4 else 5)
    return c


KEYS = {
    "ff_staff_door": [("ff_staff_door_closed", ff_staff_door_closed), ("ff_staff_door_open", ff_staff_door_open)],
    "ff_reward_box": [("ff_reward_box_full_a", ff_reward_box_full_a), ("ff_reward_box_full_b", ff_reward_box_full_b),
                      ("ff_reward_box_empty", ff_reward_box_empty)],
    "ff_samples": [("ff_pass_ledge", ff_pass_ledge), ("ff_wainscot_a", ff_wainscot_a),
                   ("ff_floor_checker_a", ff_floor_checker_a)],
}
FRAMED = ("ff_staff_door", "ff_reward_box")       # one canvas, several frames (the state swap)


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


def key_sha() -> dict:
    """The key pins: SHA-256 of each key's pixel list (the execution's byte-identity check)."""
    return {pid: hashlib.sha256(json.dumps(fn().px).encode()).hexdigest()
            for pieces in KEYS.values() for pid, fn in pieces}


def write() -> None:
    for stem, pieces in KEYS.items():
        imgs = [fn() for _, fn in pieces]
        if stem in FRAMED:
            grid = kit.grid_from(imgs, layer_names=["main"], durations=[100] * len(imgs), pivot=(0, 0))
        else:
            grid = kit.grid_from([_pack_row(imgs, pad=8)], layer_names=["main"], durations=[100], pivot=(0, 0))
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


def measurements() -> list[str]:
    """The numbers the gate and the review check against spec section 10."""
    out = []
    closed, opened = ff_staff_door_closed(), ff_staff_door_open()
    sx0, sx1 = PANEL
    cov_c = sum(closed.get(x, y) is not None for x in range(sx0, sx1 + 1) for y in range(DOOR_H))
    lane = [(x, y) for x in range(sx0, sx1 + 1) for y in range(4, SILL)]
    cov_o = sum(opened.get(x, y) is not None for x, y in lane)
    out.append(f"door_closed solid_coverage={cov_c}/{16 * DOOR_H} (target: all)")
    out.append(f"door_open lane_pixels_opaque={cov_o}/{len(lane)} rows 4..225 (target: 0)")
    rows_lip = [y for y in range(DOOR_H) if any(opened.get(x, y) for x in range(sx0, sx1 + 1))]
    out.append(f"door_open opaque_panel_rows={rows_lip} (lip rows 0..3, sill rows 226..227)")
    for nm, img in (("closed", closed), ("open", opened)):
        cols = {p for row in img.px for p in row if p}
        out.append(f"door_{nm} colours={len(cols)} max_luma={max(_luma(*c) for c in cols):.0f}")
    for fn in (ff_reward_box_full_a, ff_reward_box_full_b, ff_reward_box_empty):
        img = fn()
        cols = [p for row in img.px for p in row if p]
        mean = sum(_luma(*c) for c in cols) / len(cols)
        top = min(y for y in range(img.h) if any(img.px[y]))
        out.append(f"{fn.__name__} opaque={len(cols)} mean_luma={mean:.0f} top_row={top} "
                   f"bottom_row_opaque={sum(1 for p in img.px[BOX_H - 1] if p)}")
    led = ff_pass_ledge()
    out.append("pass_ledge rows 0..15 opaque: "
               + str(all(led.get(x, y) for x in range(96) for y in range(16))))
    lum = [max(_luma(*led.get(x, y)) for x in range(96)) for y in range(16)]
    out.append(f"pass_ledge brightest_row={lum.index(max(lum))} (y {448 + lum.index(max(lum))}) luma={max(lum):.0f}")
    for fn in (ff_wainscot_a, ff_floor_checker_a):
        img = fn()
        cols = [p for row in img.px for p in row if p]
        out.append(f"{fn.__name__} max_luma={max(_luma(*c) for c in cols):.0f} colours={len(set(cols))}")
    out.append("stand line luma flame5=%.0f; door max=%.0f; wainscot max=%.0f" % (
        _luma("flame", 5), max(_luma(*p) for row in closed.px for p in row if p),
        max(_luma(*p) for row in ff_wainscot_a().px for p in row if p)))
    for pid, sha in key_sha().items():
        out.append(f"KEY {pid} {sha}")
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


# The gate's scene stills are plain refuge window frames (greybox decor rectangles off) of two REST-WIRE pilots
# at the same view (left x 960, top y 292): ``carl_closed`` (``--rest-pilot carl_door`` tick 560: the door
# closed, the box untouched) and ``donut_open`` (``--rest-pilot donut`` tick 376: the box spent, the door open).
# Each is the other's actor-free reference: a pixel inside an actor box that differs from the other still is an
# actor pixel and is re-pasted over the keys (actors draw over props and decals).
VIEW_TOP = 292
DOOR_AT = (1436, 352)
BOX_AT = (1204, 556)
LEDGE_AT = (1296, 448)
GATE_VIEWS = {
    # still: (view left, door frame, box frame, reference still, actor boxes in screen px)
    "carl_closed": (960, "closed", "full_a", "donut_open", ((360, 200, 460, 300),)),
    "donut_open": (960, "open", "empty", "carl_closed", ((360, 110, 440, 200), (216, 220, 264, 300))),
}


def gate(out_dir: Path, views: dict) -> None:
    """Gate sheets: the keys at 1x and 4x, their silhouettes, the door rising strip, and the keys composed at
    1x into the plain refuge stills of ``GATE_VIEWS`` with Carl and Donut re-pasted on top."""
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
    # the door rising: the closed key, five crops of section 6's 12-tick table (ticks 1, 3, 5, 7, 9), the open key
    steps = [0, RISE[0], RISE[2], RISE[4], RISE[6], RISE[8], OPEN_SHIFT]
    imgs = [to_image(door_raised(d)) for d in steps]
    s = Image.new("RGBA", (len(imgs) * (DOOR_W + pad) + pad, DOOR_H + 2 * pad), (40, 36, 48, 255))
    for k, im in enumerate(imgs):
        s.alpha_composite(im, (pad + k * (DOOR_W + pad), pad))
    s.save(out_dir / "door_rising_1x.png")
    s.resize((s.width * 2, s.height * 2), Image.NEAREST).save(out_dir / "door_rising_2x.png")

    box = {"full_a": ff_reward_box_full_a(), "full_b": ff_reward_box_full_b(), "empty": ff_reward_box_empty()}
    door = {"closed": ff_staff_door_closed(), "open": ff_staff_door_open()}
    ledge = to_image(ff_pass_ledge())
    wains, check = to_image(ff_wainscot_a()), to_image(ff_floor_checker_a())
    for name, (vx, door_state, box_state, ref, actor_boxes) in views.items():
        still = Image.open(out_dir / f"scene_{name}_1x.png").convert("RGBA")
        other = Image.open(out_dir / f"scene_{ref}_1x.png").convert("RGBA")
        comp = still.copy()
        for wx in range(1248, 1440, 32):             # language samples on the staff wall (pool-free)
            comp.alpha_composite(wains, (wx - vx, 528 - VIEW_TOP))
        for wx in range(1120, 1440, 32):
            comp.alpha_composite(check, (wx - vx, 561 - VIEW_TOP))
        comp.alpha_composite(ledge, (LEDGE_AT[0] - vx, LEDGE_AT[1] - VIEW_TOP))
        comp.alpha_composite(to_image(box[box_state]), (BOX_AT[0] - vx, BOX_AT[1] - VIEW_TOP))
        comp.alpha_composite(to_image(door[door_state]), (DOOR_AT[0] - vx, DOOR_AT[1] - VIEW_TOP))
        for x0, y0, x1, y1 in actor_boxes:           # actors draw over props and decals
            for y in range(y0, y1):
                for x in range(x0, x1):
                    p, q = still.getpixel((x, y)), other.getpixel((x, y))
                    if sum(abs(p[i] - q[i]) for i in range(3)) > 40:
                        comp.putpixel((x, y), p)
        comp.save(out_dir / f"keys_in_{name}_1x.png")
        comp.resize((1280, 720), Image.NEAREST).save(out_dir / f"keys_in_{name}_2x.png")
    # the two state pairs side by side at gameplay scale: world x 1180..1470, y 340..584 of each composite
    crops = [Image.open(out_dir / f"keys_in_{n}_1x.png").crop((1180 - v[0], 340 - VIEW_TOP, 1470 - v[0], 584 - VIEW_TOP))
             for n, v in views.items()]
    pair = Image.new("RGBA", (sum(c.width for c in crops) + 8 * (len(crops) - 1), crops[0].height), (40, 36, 48, 255))
    for k, c in enumerate(crops):
        pair.paste(c, (k * (c.width + 8), 0))
    pair.save(out_dir / "states_in_scene_1x.png")
    pair.resize((pair.width * 4, pair.height * 4), Image.NEAREST).save(out_dir / "states_in_scene_4x.png")
    (out_dir / "measurements.txt").write_text("\n".join(measurements()) + "\n", encoding="utf-8")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "gate":
        gate(Path(sys.argv[2]), GATE_VIEWS)
    else:
        write()
