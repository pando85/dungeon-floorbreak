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


# =============================================================================================
# EXECUTION: the remaining pieces of spec section 5 (new functions only; the eight keys above stay
# byte-identical). Rules: palette ``floor01-catacomb-v2`` only, at most 63 colours per atlas, 1-bit alpha,
# every piece but the declared opaque tile pieces holds at least one transparent pixel, no ``flame`` 5 or
# ``copper`` 8 run of 8 px or more outside the three stand lines and the strip tubes (spec 10.1).
# Seeds come from ``rnd("<piece id>", ...)`` through ``_r``.
# =============================================================================================
def _r(pid: str, *key) -> float:
    return rnd(pid, *key)


def _clear_corners(c: Pix, n: int = 1) -> None:
    for k in range(n):
        for x in range(n - k):
            for cx, cy in ((x, k), (c.w - 1 - x, k), (x, c.h - 1 - k), (c.w - 1 - x, c.h - 1 - k)):
                c.clear(cx, cy)


def _grain(x: int, y: int, base: float, seed: str, lo: int, hi: int) -> int:
    return int(clamp(_steel(x, y, base, seed), lo, hi))


# ---- wall: tiles, wainscot, decals -------------------------------------------------------------
def ff_wainscot_b() -> Pix:
    """32 x 32, the wainscot field of ``ff_wainscot_a`` with a different chip pattern: the same trim and the same
    8 px tiles, other tile values, chips at hashed positions and one hairline crack. Never two in a row."""
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
                v = 7 if _r("ff_wainscot_b", "tile", *tile) > 0.3 else 8
                if _r("ff_wainscot_b", "chip", *tile) > 0.55:
                    cx = 2 + int(_r("ff_wainscot_b", "cx", *tile) * 4)
                    cy = 2 + int(_r("ff_wainscot_b", "cy", *tile) * 4)
                    if (tx, ty) == (cx, cy) or (tx, ty) == (cx + 1, cy):
                        v = 5
            c.put(x, y, "gal", v)
    for k in range(5):                                # one hairline crack down a tile face
        c.put(19 + (k // 2), 6 + k * 2, "gal", 4)
        c.put(19 + (k // 2), 7 + k * 2, "gal", 4)
    return c


def ff_wall_tile_a() -> Pix:
    """32 x 32 opaque wall tile: a pale field of 8 px tiles, ``gal`` 8/9 faces on ``gal`` 6 grout, sparse
    ``stone_cold`` 4 highlights. No white wall field (rule 10.2); the field stays mid-value (rule 10.3)."""
    c = Pix(32, 32)
    for y in range(32):
        for x in range(32):
            tx, ty = x % 8, y % 8
            tile = (x // 8, y // 8)
            if tx == 7 or ty == 7:
                c.put(x, y, "gal", 6)
                continue
            v = 9 if _r("ff_wall_tile_a", "face", *tile) > 0.5 else 8
            if tx >= 5 and ty >= 5 and bayer(x, y) > 0.5:
                v -= 1                                  # a step of shade toward the lower right corner
            c.put(x, y, "gal", v)
    for tx in range(4):
        for ty in range(4):
            if _r("ff_wall_tile_a", "hi", tx, ty) > 0.72:
                c.put(tx * 8 + 1, ty * 8 + 1, "stone_cold", 4)
    return c


def _checker_px(c: Pix, x: int, y: int) -> None:
    row = y // 6
    tx, ty = x % 8, y % 6
    pale = ((x // 8) + row) % 2 == 0
    if tx == 7 or ty == 5:
        c.put(x, y, "gal", 4)
    elif pale:
        c.put(x, y, "stone", 10 if ty < 4 else 9)
    else:
        c.put(x, y, "gal", 6 if ty < 4 else 5)


def ff_floor_checker_edge() -> Pix:
    """32 x 12 at (256, 561): the checker's broken left edge. Same 8 px grid and parity as ``ff_floor_checker_a``
    (world tile parity continues): the top row is gone up to x 13, the bottom row up to x 5, two loose tiles
    lie in the gap."""
    c = Pix(32, 12)
    for y in range(12):
        for x in range(32):
            gone = x < 14 if y < 6 else x < 6
            if y < 2 and x in (14, 15):
                gone = True
            if y > 9 and x in (6, 7):
                gone = True
            if not gone:
                _checker_px(c, x, y)
    for x in range(5, 10):                            # loose cream tile
        for y, v in ((1, 10), (2, 10), (3, 9)):
            c.put(x, y, "stone", v)
    for x in range(10, 13):                           # loose dark tile
        for y, v in ((3, 6), (4, 6), (5, 5)):
            c.put(x, y, "gal", v)
    return c


def ff_floor_spill() -> Pix:
    """48 x 12 at (168, 561): three loose cream tiles of 3 x 2 px (18 px of tile) on the hot side, each with a
    one-pixel shadow."""
    c = Pix(48, 12)
    for x0, y0 in ((4, 4), (21, 7), (38, 5)):
        for dx in range(3):
            c.put(x0 + dx, y0, "stone", 10)
            c.put(x0 + dx, y0 + 1, "stone", 9)
        for dx in range(1, 4):
            c.put(x0 + dx, y0 + 2, "gal", 3)
    return c


def ff_tile_seam() -> Pix:
    """8 x 192 at (376, 368): the tile wall's torn left edge over the stone pier's right face. Column 7 is the
    grout bead, column 6 the lit edge of the last whole tile, columns 2..5 the torn remnants (a hashed depth per
    row), a grout line every 8 rows."""
    c = Pix(8, 192)
    for y in range(192):
        grout = y % 8 == 7
        c.put(7, y, "gal", 4)
        c.put(6, y, "gal", 3 if grout else 9)
        depth = 1 + int(_r("ff_tile_seam", "d", y // 4) * 4)       # remnant columns torn off, 1..4
        for x in range(5, 1, -1):
            if (5 - x) < depth + 0:
                if grout:
                    c.put(x, y, "gal", 3)
                else:
                    c.put(x, y, "gal", 8 if x == 5 else (7 if x == 4 else 6))
        if depth >= 3:
            c.put(5 - depth + 1, y, "gal", 9 if not grout else 3)
        if _r("ff_tile_seam", "chip", y) > 0.93:
            c.put(1, y, "gal", 5)
    return c


# ---- wall: canopy, hatch, panel, chute ---------------------------------------------------------
def ff_counter_canopy() -> Pix:
    """128 x 40 at (704, 400): a backlit blank panel over the counter. ``gal`` 5..7 frame, a ``gal`` 9/10 field,
    three abstract food-like blobs in ``brass`` / ``copper``, no words and no glyphs. Nothing brighter than
    ``brass`` 5 (it lies within 64 px of the slide-top stand line)."""
    c = Pix(128, 40)
    for y in range(40):
        for x in range(128):
            edge = x in (0, 127) or y in (0, 39)
            edge2 = x in (1, 126) or y in (1, 38)
            if edge:
                c.put(x, y, "gal", 5)
            elif edge2:
                c.put(x, y, "gal", 7 if y in (1,) or x == 1 else 6)
            else:
                v = 9 if bayer(x, y) > 0.45 + 0.35 * (y / 40) else 10
                c.put(x, y, "gal", v)
    # blob 1 (x 20..43): a stacked round form: bun-like dome, a dark band, a base
    def ell(cx, cy, rx, ry, ramp, v, top_only=False, bot_only=False):
        for y in range(int(cy - ry), int(cy + ry) + 1):
            for x in range(int(cx - rx), int(cx + rx) + 1):
                d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
                if d <= 1.0 and not (top_only and y > cy) and not (bot_only and y < cy):
                    c.put(x, y, ramp, v)
    ell(32, 17, 12, 7, "copper", 5, top_only=True)
    ell(32, 17, 12, 7, "copper", 4, bot_only=True)
    for x in range(21, 44):
        c.put(x, 18, "brass", 4)
        c.put(x, 19, "brass", 3)
    for x in range(23, 41):
        c.put(x, 20, "copper", 4)
    for dx, dy in ((-6, -4), (-2, -5), (3, -4), (6, -3)):
        c.put(32 + dx, 17 + dy, "brass", 5)
    # blob 2 (x 52..75): a bundle of thin vertical forms in a cone
    for k in range(7):
        x0 = 54 + k * 3
        top = 9 + (abs(k - 3) * 2)
        for y in range(top, 28):
            c.put(x0, y, "brass", 4 if (y + k) % 4 else 5)
            c.put(x0 + 1, y, "brass", 3)
    for x in range(51, 76):
        c.put(x, 27, "gal", 5)
        c.put(x, 28, "gal", 4)
    # blob 3 (x 88..110): a tapered cup with a rounded top
    for y in range(13, 29):
        half = 8 - (y - 13) * 3 // 16
        for x in range(99 - half, 99 + half + 1):
            c.put(x, y, "gal", 7 if x < 99 else 6)
        c.put(99 - half, y, "gal", 8)
    ell(99, 11, 9, 4, "copper", 5, top_only=True)
    for x in range(91, 108):
        c.put(x, 19, "brass", 4)
        c.put(x, 20, "brass", 3)
    _clear_corners(c, 1)
    return c


def ff_service_hatch() -> Pix:
    """80 x 48 at (1304, 400): the dark kitchen opening over the ledge. ``iron`` 0..2 interior, a ``gal`` 5..8
    frame and sill, shelf and jar silhouettes in ``iron`` 3..4 and **one** dim ``copper`` 5 heat-lamp glow row.
    Never brighter than the ledge's stand line (nothing above ``gal`` 8 and ``copper`` 5)."""
    c = Pix(80, 48)
    for y in range(48):
        for x in range(80):
            edge_t = y in (0, 1)
            if y == 0:
                c.put(x, y, "gal", 7)
            elif y == 1:
                c.put(x, y, "gal", 5)
            elif x in (0, 79):
                c.put(x, y, "gal", 6 if x == 0 else 4)
            elif x in (1, 78):
                c.put(x, y, "gal", 4)
            elif y >= 46:
                c.put(x, y, "gal", 8 if y == 46 else 4)
            else:
                t = y / 46.0
                v = 0 if bayer(x, y) > 0.55 * (0.35 + t) else 1
                if t > 0.7 and bayer(x, y) > 0.5:
                    v = 2
                c.put(x, y, "iron", v)
    for x in range(8, 72):                            # the heat-lamp glow: one dim row (copper 5)
        c.put(x, 7, "copper", 5)
    for x in range(12, 68, 2):
        c.put(x, 8, "copper", 3)
    for y0, x0, x1 in ((22, 6, 40), (36, 30, 74)):    # shelf lines
        for x in range(x0, x1):
            c.put(x, y0, "iron", 4)
            c.put(x, y0 + 1, "iron", 3)
    for jx, jy, jw, jh in ((10, 15, 5, 7), (18, 17, 4, 5), (34, 16, 6, 6), (46, 29, 5, 7), (58, 30, 7, 6)):
        for y in range(jy, jy + jh):
            for x in range(jx, jx + jw):
                c.put(x, y, "iron", 3 if x == jx else 2)
        for x in range(jx, jx + jw):
            c.put(x, jy, "iron", 4)
    _clear_corners(c, 1)
    return c


def ff_under_panel() -> Pix:
    """96 x 60 at (1296, 468): the tiled panel under the ledge (the counter's back face), 8 px tiles of
    ``gal`` 6/7 on ``gal`` 3 grout, shaded under the ledge."""
    c = Pix(96, 60)
    for y in range(60):
        for x in range(96):
            tx, ty = x % 8, y % 8
            tile = (x // 8, y // 8)
            if tx == 7 or ty == 7:
                v = 3
            elif tx == 0 or ty == 0:
                v = 8
            else:
                v = 6 if _r("ff_under_panel", "t", *tile) > 0.3 else 7
            if y < 3:
                v = max(2, v - 3 + y)                  # the ledge's shadow
            if y >= 57:
                v = max(2, v - 2)
            c.put(x, y, "gal", v)
    for x in (0, 95):
        for y in range(60):
            c.put(x, y, "gal", 4 if x == 0 else 3)
    _clear_corners(c, 2)
    return c


def ff_slide_chute() -> Pix:
    """64 x 64 at (688, 496): the slide curving down from the slide top's right end behind the counter end.
    ``brass`` 3 and 4 only (rule 10.1: no ``flame`` 5 run), a band of 7 px along a quadratic ease."""
    c = Pix(64, 64)
    pts = [(63.0 * t / 255.0, 6.0 + 48.0 * (t / 255.0) ** 1.9) for t in range(256)]
    for y in range(64):
        for x in range(64):
            best, side = 1e9, 0.0
            for px, py in pts[::3]:
                d = math.hypot(x - px, y - py)
                if d < best:
                    best, side = d, (y - py)
            if best <= 3.6:
                v = 4 if (side < 0 and best > 1.2) else 3
                if best > 2.6 and side >= 0:
                    v = 3
                if side < 0 and best <= 1.2 and bayer(x, y) > 0.5:
                    v = 4
                c.put(x, y, "brass", v)
    return c


# ---- structure: ceiling, skins, posts, window --------------------------------------------------
def ff_ceiling_tile() -> Pix:
    """32 x 16 opaque at y 336: an acoustic drop-ceiling panel, ``gal`` 7..9 face with hashed pinholes, a
    ``gal`` 4 T-bar left and right, a lit lip on row 13, the shadowed slab underside on the bottom row."""
    c = Pix(32, 16)
    for y in range(16):
        for x in range(32):
            if y == 0:
                v = 5
            elif y == 1:
                v = 3
            elif y == 15:
                v = 2
            elif y == 14:
                v = 4
            elif y == 13:
                v = 6
            elif x == 0:
                v = 4
            elif x == 31:
                v = 3
            else:
                v = 8 if bayer(x, y) > 0.35 else 7
                if _r("ff_ceiling_tile", "hole", x // 2, y // 2) > 0.86 and (x + y) % 2 == 0:
                    v = 6
            c.put(x, y, "gal", v)
    return c


def ff_soffit_stone() -> Pix:
    """32 x 16 opaque at y 336 outside the restaurant: the slab in the catacomb's own stone, ``stone`` 3..5."""
    c = Pix(32, 16)
    for y in range(16):
        for x in range(32):
            n = vnoise(x, y, 7.0, 5.0, "ff_soffit_stone")
            v = 3.8 + (n - 0.5) * 2.4 + (bayer(x, y) - 0.5) * 0.9
            v = clamp(math.floor(v + 0.5), 3, 5)
            if y == 15:
                v = 2
            elif y == 14:
                v = 3
            c.put(x, y, "stone", v)
    for y in range(0, 14):
        if y % 7 != 3:
            c.put(15, y, "stone", 2)
    return c


def _skin(w: int) -> Pix:
    """A play-structure skin over a plank tile (spec 9.2, FF-AD10, gate note G8): rows 0..15 opaque across the
    width. Row 0 ``brass`` 3, row 1 ``flame`` 5 (the stand line), rows 2..3 ``brass`` 4, rows 4..11 the face
    (``brass`` 5/4 by Bayer, at most 5), row 12 ``brass`` 3, rows 13..14 ``brass`` 2, row 15 ``stone_cold`` 0.
    End caps on columns 0 and w-1 are ``brass`` 3 (rows 2..14, so the stand line runs the full width)."""
    c = Pix(w, 16)
    for x in range(w):
        c.put(x, 0, "brass", 3)
        c.put(x, 1, "flame", 5)
        c.put(x, 2, "brass", 4)
        c.put(x, 3, "brass", 4)
        for y in range(4, 12):
            c.put(x, y, "brass", 5 if bayer(x, y) > 0.5 else 4)
        c.put(x, 12, "brass", 3)
        c.put(x, 13, "brass", 2)
        c.put(x, 14, "brass", 2)
        c.put(x, 15, "stone_cold", 0)
    for x in (0, w - 1):
        for y in range(2, 15):
            c.put(x, y, "brass", 3)
    return c


def ff_deck_skin() -> Pix:
    return _skin(96)


def ff_slide_top_skin() -> Pix:
    return _skin(128)


def _play_posts_below() -> Pix:
    """The 224 x 52 post and net block that starts at y 528 (the first execution's ``ff_play_posts``): two posts under
    each skin and a net panel between them, dimmer than the skins' faces (``gal`` 4..7, ``brass`` 2..3, the net
    ``gal`` 5). Deck posts at x 6..11 and 84..89, slide posts at 104..109 and 210..215."""
    c = Pix(224, 52)
    posts = ((6, 84), (104, 210))
    nets = ((12, 83), (110, 209))
    for x0 in (p for pair in posts for p in pair):
        for y in range(52):
            for dx, v in enumerate((7, 6, 6, 5, 5, 4)):
                c.put(x0 + dx, y, "gal", v)
        for y in range(0, 14):                         # the foam pad on the upper post
            for dx in range(6):
                c.put(x0 + dx, y, "brass", 3 if dx < 2 else 2)
        for dx in range(-2, 8):                       # base plate
            c.put(x0 + dx, 50, "gal", 5)
            c.put(x0 + dx, 51, "gal", 3)
    for x0, x1 in nets:
        for x in range(x0, x1 + 1):
            c.put(x, 3, "brass", 2)
            c.put(x, 4, "brass", 2)
            c.put(x, 33, "brass", 2)
            for y in range(5, 33):
                if (x + y) % 4 == 0 or (x - y) % 4 == 0:
                    c.put(x, y, "gal", 5)
    return c


POSTS_UP = 16                                           # rows added above the first execution's block (review F3)


def ff_play_posts() -> Pix:
    """224 x 68 at (464, 512) (review F3): the 52-row block of the first execution moved down by 16 rows
    (byte-identical, rows 16..67), and the slide side's posts and net continued straight up to y 512 so the slide-top
    skin (y 496..511) stands on its posts with no wall gap. Rows 0..15 under the deck skin (columns 0..95, which the
    skin covers) are transparent. The slide side's top rail moves to new rows 0..1 (``brass`` 2); the net mesh and the
    posts continue with the same column pattern and ramps; nothing is brighter than the skin face."""
    c = Pix(224, 52 + POSTS_UP)
    below = _play_posts_below()
    for y in range(52):
        for x in range(224):
            c.px[y + POSTS_UP][x] = below.px[y][x]
    for x0 in (104, 210):                               # the slide-side posts, straight up
        for y in range(POSTS_UP):
            for dx, v in enumerate((7, 6, 6, 5, 5, 4)):
                c.put(x0 + dx, y, "gal", v)
            for dx in range(6):                          # the foam pad runs on up
                c.put(x0 + dx, y, "brass", 3 if dx < 2 else 2)
    for x in range(110, 210):                           # the slide-side net
        c.put(x, 0, "brass", 2)
        c.put(x, 1, "brass", 2)
        for y in range(2, POSTS_UP):
            if (x + y) % 4 == 0 or (x - y) % 4 == 0:
                c.put(x, y, "gal", 5)
    return c


def ff_window_frame() -> Pix:
    """160 x 160 at (900, 384): the window over bay B's opening. A 6 px stainless frame, two mullions (x 52..57
    and 102..107), a sill on rows 144..159 (y 528..543, it covers the wainscot row behind it), glass glints in the
    three panes. The panes are transparent: the rock shows through."""
    c = Pix(160, 160)

    def steel(x, y, base, lo=4, hi=9):
        return _grain(x, y, base, "ff_window_frame", lo, hi)

    for y in range(160):
        for x in range(160):
            border = y < 6 or x < 6 or x > 153
            if border and y < 144:
                if y == 0 or x == 0 or x == 159:
                    v = 9
                elif y == 1 or x == 1 or x == 158:
                    v = 8
                elif y == 5 or x == 5 or x == 154:
                    v = 4
                else:
                    v = steel(x, y, 6.6)
                c.put(x, y, "gal", v)
    for x0 in (52, 102):                              # mullions
        for y in range(6, 144):
            for dx, v in enumerate((8, 7, 6, 6, 5, 4)):
                c.put(x0 + dx, y, "gal", v)
    for y in range(144, 160):                         # sill
        for x in range(160):
            if y == 144:
                v = 10
            elif y == 145:
                v = 9
            elif y in (146, 147):
                v = 8
            elif y == 148:
                v = 4
            elif y == 159:
                v = 2
            else:
                v = steel(x, y, 6.2, 5, 8)
            c.put(x, y, "gal", v)
    for x in range(160):
        c.put(x, 159, "stone_cold", 0)
    for px0, px1 in ((6, 51), (58, 101), (108, 153)):  # glass glints
        for k in range(6):
            c.put(px0 + 8 + k, 14 + k, "gal", 9)
        for k in range(3):
            c.put(px0 + 17 + k, 14 + k, "gal", 9)
    _clear_corners(c, 1)
    return c


# ---- props -------------------------------------------------------------------------------------
def ff_light_strip() -> Pix:
    """48 x 6 at y 352: a fluorescent tube under a ``gal`` housing. The tube is ``copper`` 8 on rows 3 and 4 only
    (a strip tube is allowed the run of rule 10.1). The light entry is in the contract."""
    c = Pix(48, 6)
    for x in range(2, 46):
        c.put(x, 0, "gal", 5)
    for x in range(1, 47):
        c.put(x, 1, "gal", 7)
    for x in range(48):
        c.put(x, 2, "gal", 4)
    for x in range(3, 45):
        c.put(x, 3, "copper", 8)
        c.put(x, 4, "copper", 8)
    for y in (3, 4):
        for x in (1, 2, 45, 46):
            c.put(x, y, "gal", 5)
    for x in range(5, 43):
        c.put(x, 5, "copper", 5)
    return c


def ff_counter_a() -> Pix:
    """128 x 36 at (704, 524): the order counter. A ``gal`` 8..10 stainless top on rows 0..3 (darker than every
    stand line), a blue tiled front, a ``brass`` 3/4 kick strip, and a blank register block (no screen glyphs)
    set into the front."""
    c = Pix(128, 36)
    for x in range(128):
        c.put(x, 0, "gal", 10)
        c.put(x, 1, "gal", 9)
        c.put(x, 2, "gal", 8)
        c.put(x, 3, "gal", 4)
    for y in range(4, 31):
        for x in range(128):
            tx, ty = x % 8, (y - 4) % 8
            tile = (x // 8, (y - 4) // 8)
            if tx == 7 or ty == 7:
                v = 3
            elif tx == 0 or ty == 0:
                v = 7
            else:
                v = 6 if _r("ff_counter_a", "t", *tile) > 0.3 else 5
            c.put(x, y, "gal", v)
    for x in range(96, 120):                          # the blank register block
        for y in range(7, 23):
            edge = x in (96, 119) or y in (7, 22)
            c.put(x, y, "iron", 1 if edge else 3)
    for x in range(99, 117):
        for y in range(10, 17):
            c.put(x, y, "iron", 2)
    for x in range(100, 116, 3):                      # blank keys: three dim pads, no glyph
        for dx in range(2):
            c.put(x + dx, 19, "iron", 4)
            c.put(x + dx, 20, "iron", 4)
    for x in range(128):
        c.put(x, 31, "gal", 2)
        c.put(x, 32, "brass", 4)
        c.put(x, 33, "brass", 3)
        c.put(x, 34, "brass", 3)
        c.put(x, 35, "iron", 1)
    for x in (0, 127):
        for y in range(1, 31):
            c.put(x, y, "gal", 4 if x == 0 else 3)
    _clear_corners(c, 1)
    return c


BALL_PALETTE = (("brass", 4), ("brass", 4), ("brass", 4), ("gal", 8), ("gal", 8), ("brick", 6))


def ff_ball_pit() -> Pix:
    """96 x 12 at (560, 548): the pit's rim and a heap of single-pixel balls in 2 x 2 clusters (``brass`` 4,
    ``gal`` 8, ``brick`` 6; one ``brass`` 5 glint each on the yellow ones) under the slide top. The rim is at
    most 12 rows."""
    c = Pix(96, 12)
    for x in range(96):                               # the rim
        c.put(x, 6, "gal", 8)
        c.put(x, 7, "gal", 6)
        for y in (8, 9, 10):
            c.put(x, y, "gal", 6 if bayer(x, y) > 0.5 else 5)
        c.put(x, 11, "gal", 3)
    for y in range(6, 12):
        c.put(0, y, "gal", 4)
        c.put(95, y, "gal", 3)
    for cx in range(3, 93, 2):                        # the heap of balls: 2 x 2 clusters
        h = int(5.6 * math.sin(math.pi * (cx - 3) / 90.0) ** 0.7 + 0.5)
        for cy in range(6 - h, 6, 2):
            ramp, v = BALL_PALETTE[int(_r("ff_ball_pit", cx, cy) * len(BALL_PALETTE)) % len(BALL_PALETTE)]
            for dx in range(2):
                for dy in range(2):
                    if cy + dy < 6:
                        c.put(cx + dx, cy + dy, ramp, v)
            if ramp == "brass" and cy + 0 < 6:
                c.put(cx, cy, "brass", 5)
    return c


def ff_stray_ball() -> Pix:
    """6 x 6 at (1012, 554): one static yellow ball, ``brass`` 4 with a ``brass`` 2 shade and a single ``brass`` 6
    glint."""
    c = Pix(6, 6)
    rows = (".XXXX.", "XXXXXX", "XXXXXX", "XXXXXX", "XXXXXX", ".XXXX.")
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch == "X":
                v = 4
                if x >= 4 or y >= 4:
                    v = 2 if (x + y) >= 7 else 3
                c.put(x, y, "brass", v)
    c.put(1, 1, "brass", 6)
    c.put(2, 1, "brass", 5)
    c.put(1, 2, "brass", 5)
    return c


def ff_bin() -> Pix:
    """20 x 32 at (400, 528): a stainless bin with a swing flap and no text. ``gal`` 3..10 only (rows are darker
    than every stand line)."""
    c = Pix(20, 32)
    for y in range(32):
        for x in range(20):
            if y < 4:                                  # the lid
                if 1 <= x <= 18:
                    c.put(x, y, "gal", (10, 9, 8, 4)[y])
            elif 1 <= x <= 18:
                v = _grain(x, y, 6.2, "ff_bin", 4, 8)
                if x == 1:
                    v = 9
                elif x == 2:
                    v = 8
                elif x == 18:
                    v = 3
                elif x == 17:
                    v = 4
                if y >= 29:
                    v = max(2, v - 2)
                c.put(x, y, "gal", v)
    for x in range(4, 16):                             # the swing flap: a dark rounded door with a lit lip
        for y in range(9, 18):
            c.put(x, y, "gal", 2 if y > 9 else 8)
        c.put(x, 18, "gal", 3)
    for x in (4, 15):
        for y in range(9, 19):
            c.put(x, y, "gal", 4)
    for x in (9, 10):
        c.put(x, 8, "gal", 9)                          # hinge pin
    for x in range(2, 18):
        c.put(x, 31, "stone_cold", 0)
    for x in (1, 18):
        c.put(x, 30, "gal", 3)
        c.put(x, 31, "stone_cold", 0)
    return c


def ff_booth_a() -> Pix:
    """112 x 40 at (912, 520): two benches and a table. Blue vinyl ``gal`` 6..8, a ``brass`` 3/4 table edge (the
    table top is ``gal`` 9 at most), ``stone_cold`` chrome legs."""
    c = Pix(112, 40)

    def bench(x0: int, left: bool) -> None:
        bx0, bx1 = (x0, x0 + 12) if left else (x0 - 12 + 1, x0 + 1)
        for y in range(3, 38):                        # the backrest
            for x in range(bx0, bx1):
                edge = (x == bx0) if left else (x == bx1 - 1)
                v = 8 if edge else (_grain(x, y, 6.6, "ff_booth_a", 6, 7))
                if y == 3:
                    v = 8
                c.put(x, y, "gal", v)
        sx0, sx1 = (x0, x0 + 30) if left else (x0 - 29, x0 + 1)
        for y in range(23, 30):                       # the seat cushion
            for x in range(sx0, sx1):
                v = 7 if y < 25 else (6 if y < 28 else 5)
                if y == 23:
                    v = 8
                c.put(x, y, "gal", v)
        for y in range(30, 39):                       # base
            for x in range(sx0 + 1, sx1 - 1):
                c.put(x, y, "gal", 3 if y > 31 else 4)
    bench(0, True)
    bench(111, False)
    for x in range(32, 80):                           # the table top and its brass edge
        c.put(x, 15, "gal", 9)
        c.put(x, 16, "gal", 8)
        c.put(x, 17, "brass", 4)
        c.put(x, 18, "brass", 3)
    for y in range(19, 37):                           # chrome pedestal
        for dx, v in enumerate((4, 3, 3, 2)):
            c.put(54 + dx, y, "stone_cold", v)
    for x in range(46, 66):
        c.put(x, 37, "stone_cold", 3)
        c.put(x, 38, "stone_cold", 1)
        c.put(x, 39, "stone_cold", 0)
    for x in range(1, 111):
        if c.get(x, 39) is None and (x < 28 or x > 84):
            c.put(x, 39, "stone_cold", 0)
    _clear_corners(c, 1)
    return c


# ---- presenter-only: the bell, the sparkle -----------------------------------------------------
def _bell(frame: str) -> Pix:
    """8 x 6 service bell (gate note G8): ``gal`` 6..10 dome, ``brass`` 4 base, a ``gal`` 10 glint, no ``flame``
    and no ``copper``. ``ding_a`` pushes the plunger 1 px down; ``ding_b`` is idle plus two 1 px ``gal`` 10
    ticks beside the dome on row 1."""
    c = Pix(8, 6)
    plunger_row = 1 if frame == "ding_a" else 0
    c.put(3, plunger_row, "gal", 8)
    c.put(4, plunger_row, "gal", 8)
    for x in range(2, 6):
        if not (frame == "ding_a" and x in (3, 4)):
            c.put(x, 1, "gal", 9 if x < 4 else 8)
    for x in range(1, 7):
        c.put(x, 2, "gal", 8)
        c.put(x, 3, "gal", 7 if x < 4 else 6)
    c.put(2, 2, "gal", 10)
    for x in range(8):
        c.put(x, 4, "brass", 4)
    for x in range(1, 7):
        c.put(x, 5, "brass", 3)
    if frame == "ding_b":
        c.put(0, 1, "gal", 10)
        c.put(7, 1, "gal", 10)
    return c


def ff_bell_idle() -> Pix:
    return _bell("idle")


def ff_bell_ding_a() -> Pix:
    return _bell("ding_a")


def ff_bell_ding_b() -> Pix:
    return _bell("ding_b")


def _spark(c: Pix, pts) -> None:
    for x, y, ramp, v in pts:
        c.put(x, y, ramp, v)


def ff_reward_sparkle_a() -> Pix:
    """16 x 16, grant tick burst frame 1: a small 4-arm cross (``flame`` 5 centre, ``brass`` 6 arms) and two
    single ``brass`` 6 motes: 7 px."""
    c = Pix(16, 16)
    _spark(c, [(8, 8, "flame", 5), (7, 8, "brass", 6), (9, 8, "brass", 6), (8, 7, "brass", 6), (8, 9, "brass", 6),
               (5, 5, "brass", 6), (11, 10, "brass", 6)])
    return c


def ff_reward_sparkle_b() -> Pix:
    """Frame 2: the cross at full size (``brass`` 6 inner arms, ``flame`` 4 outer arms): 9 px."""
    c = Pix(16, 16)
    _spark(c, [(8, 8, "flame", 5), (7, 8, "brass", 6), (9, 8, "brass", 6), (8, 7, "brass", 6), (8, 9, "brass", 6),
               (6, 8, "flame", 4), (10, 8, "flame", 4), (8, 6, "flame", 4), (8, 10, "flame", 4)])
    return c


def ff_reward_sparkle_c() -> Pix:
    """Frame 3: the burst scattered and fading: four ``brass`` 6 motes and a ``flame`` 4 centre: 5 px."""
    c = Pix(16, 16)
    _spark(c, [(8, 8, "flame", 4), (3, 4, "brass", 6), (12, 5, "brass", 6), (11, 12, "brass", 6),
               (4, 11, "brass", 6)])
    return c


# ---- rock ---------------------------------------------------------------------------------------
def ff_rock_fill() -> Pix:
    """168 x 192 at (904, 352): raw rock behind the restaurant window (bay B's opening only, FF-AD16): ``stone``
    2..5 and ``stone_cold`` with damp ``lichen`` seams. The first 152 columns are the first execution's, pixel for
    pixel; 16 columns are appended on the right (review F5) so the rock also fills the gap inside bay B's right pier
    (x 1056..1071). The corners are transparent (the arch's stone covers them); the first execution's two right
    corners at columns 150..151 stay transparent."""
    c = Pix(168, 192)
    for y in range(192):
        for x in range(168):
            n = vnoise(x, y, 11.0, 8.0, "ff_rock_fill")
            m = vnoise(x, y, 4.0, 3.0, "ff_rock_fill_m")
            v = 3.1 + (n - 0.5) * 3.2 + (m - 0.5) * 1.1 + (bayer(x, y) - 0.5) * 0.7
            v = int(clamp(math.floor(v + 0.5), 1, 5))
            ramp = "stone"
            if vnoise(x, y, 17.0, 13.0, "ff_rock_fill_c") > 0.8:
                ramp, v = "stone_cold", int(clamp(v - 1, 1, 3))
            # fault seams: thin dark lines along slanted ridges
            ridge = abs(vnoise(x + y * 0.35, y, 23.0, 40.0, "ff_rock_fill_s") - 0.5)
            if ridge < 0.006:
                ramp, v = "stone", 0 if bayer(x, y) > 0.3 else 1
            # damp: lichen runs down a few seams
            damp = abs(vnoise(x - y * 0.2, y, 19.0, 55.0, "ff_rock_fill_l") - 0.5)
            if damp < 0.008 and _r("ff_rock_fill", "lichen", x, y // 3) > 0.35:
                ramp, v = "lichen", 0 if _r("ff_rock_fill", "lv", x, y) > 0.5 else 1
            c.put(x, y, ramp, v)
    for k in range(2):
        for x in range(2 - k):
            for cx, cy in ((x, k), (151 - x, k), (x, 191 - k), (151 - x, 191 - k), (167 - x, k), (167 - x, 191 - k)):
                c.clear(cx, cy)
    return c


# =============================================================================================
# Atlases, manifests, the contract and the room dressing (the heat and ring precedents)
# =============================================================================================
def _rise(k: int):
    return lambda: door_raised(RISE[k - 1])


ATLASES = {
    "floor01_ff_door": {
        "stem": "ff_door", "width": 24 * 14,
        "pieces": ([("ff_staff_door_closed", ff_staff_door_closed)]
                   + [(f"ff_staff_door_rise_{k:02d}", _rise(k)) for k in range(1, 13)]
                   + [("ff_staff_door_open", ff_staff_door_open)]),
    },
    "floor01_ff_pickup": {
        "stem": "ff_pickup", "width": 96,
        "pieces": [("ff_reward_box_full_a", ff_reward_box_full_a), ("ff_reward_box_full_b", ff_reward_box_full_b),
                   ("ff_reward_box_empty", ff_reward_box_empty), ("ff_reward_sparkle_a", ff_reward_sparkle_a),
                   ("ff_reward_sparkle_b", ff_reward_sparkle_b), ("ff_reward_sparkle_c", ff_reward_sparkle_c),
                   ("ff_bell_idle", ff_bell_idle), ("ff_bell_ding_a", ff_bell_ding_a),
                   ("ff_bell_ding_b", ff_bell_ding_b)],
    },
    "floor01_ff_wall": {
        "stem": "ff_wall", "width": 256,
        "pieces": [("ff_wainscot_a", ff_wainscot_a), ("ff_wainscot_b", ff_wainscot_b),
                   ("ff_wall_tile_a", ff_wall_tile_a), ("ff_floor_checker_a", ff_floor_checker_a),
                   ("ff_floor_checker_edge", ff_floor_checker_edge), ("ff_floor_spill", ff_floor_spill),
                   ("ff_counter_canopy", ff_counter_canopy), ("ff_slide_chute", ff_slide_chute),
                   ("ff_service_hatch", ff_service_hatch), ("ff_under_panel", ff_under_panel)],
    },
    "floor01_ff_structure": {
        "stem": "ff_structure", "width": 320,
        "pieces": [("ff_pass_ledge", ff_pass_ledge), ("ff_tile_seam", ff_tile_seam),
                   ("ff_ceiling_tile", ff_ceiling_tile), ("ff_soffit_stone", ff_soffit_stone),
                   ("ff_deck_skin", ff_deck_skin), ("ff_slide_top_skin", ff_slide_top_skin),
                   ("ff_play_posts", ff_play_posts), ("ff_window_frame", ff_window_frame)],
    },
    "floor01_ff_props": {
        "stem": "ff_props", "width": 256,
        "pieces": [("ff_light_strip", ff_light_strip), ("ff_counter_a", ff_counter_a), ("ff_ball_pit", ff_ball_pit),
                   ("ff_stray_ball", ff_stray_ball), ("ff_bin", ff_bin), ("ff_booth_a", ff_booth_a)],
    },
    "floor01_ff_rock": {
        "stem": "ff_rock", "width": 168,
        "pieces": [("ff_rock_fill", ff_rock_fill)],
    },
}
PRESENTER_PIECES = ({pid for pid, _ in ATLASES["floor01_ff_door"]["pieces"]}
                    | {pid for pid, _ in ATLASES["floor01_ff_pickup"]["pieces"]})
KEY_PIECE_IDS = ("ff_staff_door_closed", "ff_staff_door_open", "ff_reward_box_full_a", "ff_reward_box_full_b",
                 "ff_reward_box_empty", "ff_pass_ledge", "ff_wainscot_a", "ff_floor_checker_a")

CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_fastfood_kit.json"
MANIFESTS = ROOT / "assets/manifests"
EXTENDS = "assets/source/terrain/floor01/floor01_opening_kit.json"
OWNER = ("AI-authored environment pixel art (AREA-FASTFOOD-01: design spec, keys and silhouette gate Claude Opus; "
         "execution Claude Sonnet; independent review pending)")
LAYER_OF_ATLAS = {"floor01_ff_wall": ("backwall", "backwall_decal"), "floor01_ff_structure": ("structure", "structure"),
                  "floor01_ff_props": ("prop", "prop"), "floor01_ff_rock": ("backwall", "backwall_decal"),
                  "floor01_ff_door": ("prop", "prop"), "floor01_ff_pickup": ("prop", "prop")}
STRIP_LIGHT = {"id": "tube", "offset": [24, 3], "colour": "#d8e4ff", "radius": 150, "intensity": 0.24,
               "group": "scene", "glow": {"radius": 14, "additive": 0.08}}   # spec 8: steady, no flicker, no floor projection
STATE_OF = {"ff_staff_door_closed": ("ff_staff_door", "closed", None), "ff_staff_door_open": ("ff_staff_door", "open", None),
            "ff_reward_box_full_a": ("ff_reward_box", "full_a", None), "ff_reward_box_full_b": ("ff_reward_box", "full_b", None),
            "ff_reward_box_empty": ("ff_reward_box", "empty", None),
            "ff_bell_idle": ("ff_bell", "idle", None), "ff_bell_ding_a": ("ff_bell", "ding_a", None),
            "ff_bell_ding_b": ("ff_bell", "ding_b", None)}
STATE_OF.update({f"ff_staff_door_rise_{k:02d}": ("ff_staff_door", "rising", k) for k in range(1, 13)})
STATE_OF.update({f"ff_reward_sparkle_{s}": ("ff_reward_box", "sparkle", i + 1) for i, s in enumerate("abc")})

# Spec section 5 placements (room layout px, top-left). Presenter pieces (door, box, sparkle, bell) are NOT placed.
HATCH = (1304, 400, 80, 48)
CANOPY = (704, 400, 128, 40)
UNDER = (1296, 468, 96, 60)
# The full 32 px tile grid over 384..831 and 1136..1439 (review F4: the tiles run under the hatch, the canopy and the
# under-panel). The cell at x 1120 lies behind bay B's right pier and shows only x 1136..1151.
WALL_TILE_X = list(range(384, 832 - 31, 32)) + [1120] + list(range(1152, 1440 - 31, 32))
WALL_TILE_Y = list(range(368, 497, 32))


def _hits(x: int, y: int, w: int, h: int, r: tuple) -> bool:
    return x < r[0] + r[2] and x + w > r[0] and y < r[1] + r[3] and y + h > r[1]


def wainscot_places() -> list[tuple]:
    """Gate note G8: ``ff_wainscot_b`` at every tile index n with n % 3 == 2, counted from x 288."""
    return [("backwall_decal", "ff_wainscot_b" if n % 3 == 2 else "ff_wainscot_a", 288 + 32 * n, 528)
            for n in range(36)]


def wall_tile_places() -> list[tuple]:
    return [("backwall_decal", "ff_wall_tile_a", x, y) for y in WALL_TILE_Y for x in WALL_TILE_X]


REMOVE_AT = [  # spec section 8: the dining-room torches, their pools and the grime (each matches one composer piece)
    ("prop", "torch_wall", 336, 448), ("prop", "torch_wall", 848, 176), ("prop", "torch_wall", 848, 448),
    ("prop", "torch_wall", 1088, 448),
    ("backwall_decal", "torch_pool_a_m", 232, 364), ("backwall_decal", "torch_pool_a", 744, 92),
    ("backwall_decal", "torch_pool_a_m", 744, 364), ("backwall_decal", "torch_pool_a", 984, 364),
    ("backwall_decal", "grime_a", 600, 376),
    ("structure", "arch_chain", 912, 384),       # review F2: the catenary would hang across the window; bay A keeps its chain
]
BACKWALL_FILL = [{"x": 264, "y": 352, "w": 56, "h": 192}]       # bay A's opening behind the restaurant (264..319: also the gap inside its right pier, review F5)


def places() -> list[tuple]:
    p = [("backwall_decal", "ff_rock_fill", 904, 352)]
    p += wall_tile_places()
    p += wainscot_places()
    # the hatch, the canopy and the under-panel are placed after every wall tile, so they draw over the tiles (F4)
    p += [("backwall_decal", "ff_counter_canopy", 704, 400), ("backwall_decal", "ff_slide_chute", 688, 496),
          ("backwall_decal", "ff_service_hatch", 1304, 400), ("backwall_decal", "ff_under_panel", 1296, 468)]
    p += [("structure", "ff_tile_seam", 376, 368)]
    p += [("structure", "ff_ceiling_tile", 288 + 32 * n, 336) for n in range(36)]
    p += [("structure", "ff_soffit_stone", x, 336) for x in list(range(0, 288, 32)) + list(range(1440, 1600, 32))]
    # the floor decals are `structure` (F1) and come before the posts, so the posts' feet draw over the checker
    p += [("structure", "ff_floor_checker_a", 288 + 32 * n, 561) for n in range(36)]
    p += [("structure", "ff_floor_checker_edge", 256, 561), ("structure", "ff_floor_spill", 168, 561)]
    p += [("structure", "ff_deck_skin", 464, 512), ("structure", "ff_slide_top_skin", 560, 496),
          ("structure", "ff_play_posts", 464, 512), ("structure", "ff_window_frame", 900, 384),
          ("structure", "ff_pass_ledge", 1296, 448)]
    p += [("prop", "ff_light_strip", x, 352) for x in (400, 592, 784, 976, 1168, 1360)]
    p += [("prop", "ff_counter_a", 704, 524), ("prop", "ff_ball_pit", 560, 548), ("prop", "ff_stray_ball", 1012, 554),
          ("prop", "ff_bin", 400, 528), ("prop", "ff_booth_a", 912, 520)]
    return p


def _at(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def dressing() -> dict:
    return {"rest_refuge": {"remove": [], "remove_at": [_at(*r) for r in REMOVE_AT],
                            "backwall_fill": BACKWALL_FILL, "place": [_at(*t) for t in places()], "parallax": {}}}


def pack(pieces: list[tuple[str, Pix]], width: int) -> tuple[Pix, dict]:
    """Shelf packing, left to right: a piece's rect holds exactly its pixels (a key stays byte-identical)."""
    x = y = shelf = 0
    placed = []
    for pid, img in pieces:
        if x + img.w > width:
            x, y, shelf = 0, y + shelf, 0
        placed.append((pid, img, x, y))
        x += img.w
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


def build_pieces(name: str) -> list[tuple[str, Pix]]:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def build(name: str):
    pieces = build_pieces(name)
    sheet, rects = pack(pieces, ATLASES[name]["width"])
    return kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0)), rects, dict(pieces)


def _opaque(img: Pix) -> bool:
    return all(p is not None for row in img.px for p in row)


# Review F1 (the HEAT kit's precedent: its floor decals are `structure`): the checker floor, its broken edge and the spill
# draw above the collision floor deck (draw order 4); a `backwall_decal` (order 3) would sit under it, invisible.
FLOOR_DECALS = ("ff_floor_checker_a", "ff_floor_checker_edge", "ff_floor_spill")


def piece_entry(pid: str, atlas: str, rect: list, img: Pix) -> dict:
    role, layer = LAYER_OF_ATLAS[atlas]
    if pid in FLOOR_DECALS:
        role, layer = "structure", "structure"
    entry = {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
             "anchor": [0, 0], "anchor_name": "top_left (room placement, spec section 5)", "grid_snap": 1,
             "grammar": ["A"]}
    if layer == "backwall_decal":
        entry["decal"] = True
    if _opaque(img):
        entry["opaque"] = True          # a tile piece with no transparent pixel (FF-AD16): env_kit checks it fully opaque
    if pid in STATE_OF:
        group, state, frame = STATE_OF[pid]
        entry["state_of"] = {"group": group, "state": state}
        if frame is not None:
            entry["state_of"]["frame"] = frame
    if pid == "ff_light_strip":
        entry["lights"] = [STRIP_LIGHT]
    return entry


def contract(rects_by_atlas: dict, imgs: dict) -> dict:
    atlases, pieces, palette_use = {}, [], {}
    for name, spec in ATLASES.items():
        stem = spec["stem"]
        atlases[name] = {"grid": f"assets/source/props/floor01/{stem}.grid.json",
                         "manifest": f"assets/manifests/{name}.json",
                         "image": f"generated/art/{name}.png", "metadata": f"generated/art/{name}.json"}
        palette_use[name] = rects_by_atlas[name]["colours"]
        for pid, _ in spec["pieces"]:
            pieces.append(piece_entry(pid, name, rects_by_atlas[name]["rects"][pid], imgs[pid]))
    return {
        "format": "env-kit-v2",
        "kit_id": "floor01_fastfood",
        "extends": EXTENDS,
        "status": "proposed - AREA-FASTFOOD-01 kit (docs/02-art/area-fastfood-01-spec.md); independent review "
                  "pending; not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema "
                          "from the ENV-01 contract through the opening kit. Adds six decor atlases and the "
                          "fast-food refuge pieces for Floor 01 room 1 (rest_refuge, 'The Wrong Restaurant'); no "
                          "tiles, nothing collidable, no new palette colour. The only emitters are the six "
                          "ceiling strips (spec section 8); the dressing removes the four dining-room torches "
                          "and their pools.",
        "palette_use": palette_use,
        "atlases": atlases,
        "tiles": [],
        "pieces": pieces,
        "state_groups": {
            "ff_staff_door": {
                "states": ["closed", "rising", "open"], "anchor": [1436, 352],
                "note": "the staff door (spec section 6), 14 pre-baked 24 x 228 frames: closed, the 12 rising crops "
                        "door_raised(RISE[k-1]) and open. The dressing does NOT place it: the presenter draws it at "
                        "(1436, 352) from the lock's state (gate note G4), because the drawn art must equal the "
                        "16 x 228 solid of the lock (REST-AD6). Presentation only."},
            "ff_reward_box": {
                "states": ["full_a", "full_b", "empty", "sparkle"], "anchor": [1204, 556],
                "note": "the Light on Your Feet pickup (spec section 7), 32 x 24 frames at (1204, 556): full_a and "
                        "full_b alternate on a presentation timer, empty iff Donut holds the boost, and the three "
                        "16 x 16 sparkle frames are drawn over empty at (1212, 554) for the 12 ticks after the "
                        "grant. The dressing does NOT place it (REST-AD16). Presentation only."},
            "ff_bell": {
                "states": ["idle", "ding_a", "ding_b"], "anchor": [1380, 442],
                "note": "the service bell (spec section 6), 8 x 6 frames at (1380, 442): idle, then ding_a, ding_b, "
                        "ding_a for 4 ticks each from the door-open tick. The dressing does NOT place it. "
                        "Presentation only."},
        },
        "room_dressing": dressing(),
        "usage_rules": {
            "R1_rest_refuge": "room 1 `rest_refuge` only: the dressing never adds or moves a tile or a solid; wall "
                              "decals sit on the world bond; props stand on the floor back line (y 560); nothing "
                              "is placed over the lock rect x 1440..1455 (the door is drawn by the presenter); "
                              "the three platform skins sit on their plank tiles' grid rows and are opaque over "
                              "the whole tile with the stand line on row 1.",
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
        "kit_contract": "assets/source/terrain/floor01/floor01_fastfood_kit.json",
        "owner": OWNER,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 fast-food refuge pieces for Floor 1 room 1 (AREA-FASTFOOD-01 on the "
                      "ENV-01 kit); rule-based shapes and ramps, nothing cropped, traced or resampled from concept "
                      "art; no text, logos, brands or likenesses",
            "generator": "Deterministic authoring script scripts/author_ff01.py written by AI agents (spec and keys "
                         "designed by Claude Opus; execution by Claude Sonnet); no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1/04-fast-food-refuge (mood reference only; no pixel cropped, traced or "
                "resampled)",
            ],
        },
    }


def write_kit() -> dict:
    out, imgs = {}, {}
    for name, spec in ATLASES.items():
        grid, rects, pieces = build(name)
        imgs.update(pieces)
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")
        out[name] = {"rects": rects, "canvas": grid["canvas"], "colours": len(grid["palette"])}
        (MANIFESTS / f"{name}.json").write_text(json.dumps(manifest(name, grid["canvas"]), indent=1) + "\n",
                                                encoding="utf-8")
    CONTRACT.write_text(json.dumps(contract(out, imgs), indent=1) + "\n", encoding="utf-8")
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
    elif len(sys.argv) > 1 and sys.argv[1] == "kit":
        write_kit()
    else:
        write()
