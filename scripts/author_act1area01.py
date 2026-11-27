#!/usr/bin/env python3
"""Authoring record for AREA-ACT1-01: the dressing of the three Floor 01 act-1 rooms ``act1_chase``,
``act1_shore`` and ``act1_guild``.

Draws the frozen design keys of ``docs/02-art/area-act1-01-spec.md`` on ``floor01-catacomb-v2`` (no new
colour) as pixel-grid-v1 sources plus ``.ase``:

- ``assets/source/props/floor01/act1_grant_box.grid.json``      ``act1_grant_box_{closed,open}`` (spec 6.1)
- ``assets/source/props/floor01/act1_gallery_breach.grid.json`` ``act1_gallery_breach`` (spec 6.2)

Everything else in the three rooms is reused approved art (ENV-01, ENV-02, the refuge kit, DOZ-01); the
execution adds only derived pieces (soffit slices copied from the ENV-02 layout, step faces from the trog
step rule) as new functions below the keys and must not redraw a key (a key change is a design change).
Every pixel comes from rules (shapes, ramps, a hash noise); nothing is traced or resampled from a concept
image. No text, logos or likenesses.

    python3 scripts/author_act1area01.py            # grids + .ase of the keys, key pins
    python3 scripts/author_act1area01.py keys       # the key pins only
    python3 scripts/author_act1area01.py measure    # the key measurements (gate/measurements.txt)
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
    h = hashlib.sha256("|".join(map(str, ("act1area01",) + key)).encode()).digest()
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

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = None

    def box(self, x0, y0, x1, y1, ramp, v):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, ramp, v)

    def line(self, x0, y0, x1, y1, ramp, v):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            self.put(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n), ramp, v)


def joint(wx: int, wy: int) -> bool:
    """The catacomb back wall's running bond (16 x 8 bricks) at a world pixel: bed or head joint."""
    return wy % 8 == 0 or wx % 16 == 8 * ((wy // 8) % 2)


# =============================================================================================
# Key act1_grant_box (spec 6.1): 24 x 28 per frame, prop layer, top-left (340, 552) in act1_guild, on the
# floor of the grant zone (x 336..376) left of the guide. A plain, battered strongbox: horizontal planks,
# two iron bands, iron corner caps, a hasp with a blank brass plate (no symbol). Frame ``closed`` until the
# guild door opens on ``donut_unlocked``; frame ``open`` after: the lid thrown back, a dark interior and a
# small warm glint (what was in it is in Donut's paws now). Light from the upper left.
# =============================================================================================
BOX_W, BOX_H = 24, 28
BOX_AT = (340, 552)
BODY_Y0 = 13                       # body rows 13..27 (15 tall), floor contact on row 27
BANDS = (4, 18)                    # iron band columns (2 px each)


def _box_body(c: Pix) -> None:
    for y in range(BODY_Y0, BOX_H):
        for x in range(1, BOX_W - 1):
            plank = (y - BODY_Y0) // 5                                   # three planks
            seam = (y - BODY_Y0) % 5 == 4
            v = 3 - (1 if x > 15 else 0) + (1 if y < BODY_Y0 + 2 else 0)  # lit upper left
            if seam:
                v = 1
            elif rnd("grain", x // 3, plank, y) > 0.86:
                v -= 1
            c.put(x, y, "wood", v)
        c.put(0, y, "wood", 1)                                           # side edges
        c.put(BOX_W - 1, y, "wood", 0)
    for x in range(0, BOX_W):                                            # floor contact
        c.put(x, BOX_H - 1, "wood", 0)
    for bx in BANDS:                                                     # iron bands
        for y in range(BODY_Y0, BOX_H - 1):
            c.put(bx, y, "iron", 4)
            c.put(bx + 1, y, "iron", 3)
        c.put(bx, BODY_Y0 + 7, "iron", 6)                                # rivets
        c.put(bx, BODY_Y0 + 12, "iron", 5)
    for (x0, x1) in ((0, 2), (BOX_W - 3, BOX_W - 1)):                     # corner caps
        for x in range(x0, x1 + 1):
            c.put(x, BOX_H - 2, "iron", 3)
            c.put(x, BOX_H - 1, "iron", 1)
    c.put(BOX_W - 2, BODY_Y0 + 9, "wood", 1)                             # battered: a gouge and a split
    c.put(BOX_W - 3, BODY_Y0 + 10, "wood", 1)
    c.line(7, BODY_Y0 + 6, 10, BODY_Y0 + 7, "wood", 2)


def _box_lid_closed(c: Pix) -> None:
    for y in range(8, BODY_Y0):
        inset = 1 if y == 8 else 0
        for x in range(1 + inset, BOX_W - 1 - inset):
            v = 5 if y == 8 else (4 if x < 15 else 3)
            if y == BODY_Y0 - 1:
                v = 1                                                    # the lid's lower lip shadow
            c.put(x, y, "wood", v)
        c.put(0, y, "wood", 1)
        c.put(BOX_W - 1, y, "wood", 0)
    for bx in BANDS:
        for y in range(8, BODY_Y0):
            c.put(bx, y, "iron", 5 if y == 8 else 4)
            c.put(bx + 1, y, "iron", 3)
    c.clear(1, 8)                                                        # a chipped corner
    c.put(2, 9, "wood", 2)
    c.box(11, 11, 12, 15, "iron", 3)                                     # the hasp, shut
    c.box(10, 14, 13, 16, "brass", 3)                                    # blank lock plate
    c.put(10, 14, "brass", 5)


def _box_lid_open(c: Pix) -> None:
    for y in range(BODY_Y0, BODY_Y0 + 2):                                # dark interior behind the rim
        for x in range(2, BOX_W - 2):
            c.put(x, y, "mortar", 0 if y == BODY_Y0 else 1)
    c.put(8, BODY_Y0, "brass", 4)                                        # a warm glint left inside
    c.put(9, BODY_Y0, "brass", 6)
    c.put(15, BODY_Y0 + 1, "brass", 3)
    for i, y in enumerate(range(0, BODY_Y0)):                            # the lid thrown back, its underside
        shift = (BODY_Y0 - y) // 4                                       # leaning back to the right
        for x in range(2 + shift, BOX_W - 2 + min(shift, 1)):
            v = 2 if y > 1 else 3
            if (y % 4) == 3:
                v = 1
            c.put(x, y, "wood", v)
        c.put(1 + shift, y, "wood", 1)
        for bx in BANDS:
            c.put(bx + shift, y, "iron", 2)
            c.put(bx + 1 + shift, y, "iron", 2)
    c.box(11, BODY_Y0 + 1, 12, BODY_Y0 + 4, "iron", 3)                   # the hasp hanging loose
    c.box(10, BODY_Y0 + 3, 13, BODY_Y0 + 5, "brass", 2)


def act1_grant_box(state: str) -> Pix:
    c = Pix(BOX_W, BOX_H)
    _box_body(c)
    (_box_lid_closed if state == "closed" else _box_lid_open)(c)
    return c


# =============================================================================================
# Key act1_gallery_breach (spec 6.2): 192 x 218, structure layer, top-left (696, 342) in act1_chase (the
# sim's S2 window rect {696, 342, 192, 218}), placed after the arcade so it covers the pier and the ring
# foot it crosses. A hole torn through the wall where the machine came through: a ragged, bond-stepped edge
# (whole and half bricks gone, never smooth), a crushed brick lip, the right jamb's thickness lit from the
# upper left, a dark soffit, and behind it the far gallery in near-black with two pit props and a lintel.
# The void reaches the floor (row 217 = room y 559, the deck starts at 560).
# =============================================================================================
BR_W, BR_H = 192, 218
BR_AT = (696, 342)
LIP = 10                                                        # lip band width outside the void


def _breach_void() -> set:
    """Void pixels: a burst outline (wide in the middle, an uneven crown, open at the floor) whose edge steps
    with the bond: per 8 px course the side edge moves by whole or half bricks, per 16 px brick the crown
    drops by a course or two. Never a smooth curve, never a straight jamb longer than two courses."""
    void = set()
    cx = BR_W / 2
    for y in range(BR_H):
        course = (BR_AT[1] + y) // 8
        foot = y > 200                                              # the bottom two courses: swept clear
        jl = 0 if foot else (-12, -4, 4, 12)[int(rnd("l", course) * 4)]
        jr = 0 if foot else (-12, -4, 4, 12)[int(rnd("r", course) * 4)]
        for x in range(max(int(cx - 80) + jl, 6), min(int(cx + 80) - jr, BR_W - 6)):
            wx = BR_AT[0] + x
            dx = min(abs(x + 0.5 - cx) / 92.0, 1.0)
            crown = 10 + 112 * (1 - math.sqrt(1 - dx * dx))           # a burst crown, highest in the middle
            crown = 8 * int(crown / 8) + 8 * int(rnd("t", wx // 16) * 3)   # stepped by courses, a brick or two lost
            if y >= crown:
                void.add((x, y))
    return void


def _dist(void: set, reach: int) -> dict:
    """Chebyshev distance (1..reach) of each non-void pixel from the void."""
    d, ring = {}, set(void)
    for k in range(1, reach + 1):
        nxt = set()
        for (x, y) in ring:
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    q = (x + dx, y + dy)
                    if 0 <= q[0] < BR_W and 0 <= q[1] < BR_H and q not in void and q not in d:
                        d[q] = k
                        nxt.add(q)
        ring = nxt
    return d


def act1_gallery_breach() -> Pix:
    c = Pix(BR_W, BR_H)
    void = _breach_void()
    # Lip: broken brick in the wall's bond around the void, 4..12 px out, ragged per brick.
    for (x, y), d in _dist(void, 12).items():
        wx, wy = BR_AT[0] + x, BR_AT[1] + y
        if d > 4 + int(rnd("lip", wx // 16, wy // 8) * 9):
            continue
        if joint(wx, wy):
            c.put(x, y, "mortar", 1 if d <= 3 else 2)
        else:
            lit = 1 if (wx % 16) < 6 and (wy % 8) < 3 else 0            # upper-left catch on each brick
            v = 4 + lit - (2 if d <= 2 else 0) - (1 if rnd("chip", wx, wy) > 0.88 else 0)
            c.put(x, y, "brick", v)
    # Void: the far gallery in near-black, a floor line, two pit props and a lintel.
    for (x, y) in void:
        c.put(x, y, "mortar", 0)
    for (x, y) in void:
        if y >= BR_H - 14:
            c.put(x, y, "stone", 1 if y > BR_H - 10 else 0)               # the gallery floor going back
        for px0 in (58, 128):
            if px0 <= x < px0 + 5 and y > 60:
                c.put(x, y, "wood", 1 if x == px0 else 0)                 # pit props, barely lit
        if 52 <= y <= 56 and 50 <= x <= 140:
            c.put(x, y, "wood", 1 if y == 52 else 0)                      # lintel across the props
    # Thickness: the right jamb's inner face lit from the upper left; the soffit in shadow.
    for y in range(BR_H):
        xs = [x for x in range(BR_W) if (x, y) in void]
        if not xs:
            continue
        xr = max(xs)
        for k in range(6):
            if (xr - k, y) in void:
                c.put(xr - k, y, "stone", 5 - k // 2 if k < 5 else 2)
    for x in range(BR_W):
        ys = [y for y in range(BR_H) if (x, y) in void]
        if ys:
            for k in range(3):
                c.put(x, min(ys) + k, "mortar", 1 + (k == 2))
    # Crushed lip: dislodged half bricks hanging into the void at the top, rubble at the foot.
    for i, (bx, by) in enumerate(((40, 30), (96, 22), (150, 34))):
        if (bx, by + 2) in void:
            c.box(bx, by, bx + 7, by + 3, "brick", 5)
            c.line(bx, by + 4, bx + 7, by + 4, "brick", 2)
    for i in range(26):
        rx = int(rnd("rub", i, "x") * (BR_W - 8))
        rw = 4 + int(rnd("rub", i, "w") * 6)
        side = abs(rx + rw / 2 - BR_W / 2) / (BR_W / 2)                  # heaps higher towards the jambs
        rh = 2 + int(rnd("rub", i, "h") * 3) + int(side * 8)
        for x in range(rx, rx + rw):
            for y in range(BR_H - rh, BR_H):
                c.put(x, y, "brick", 3 + (y == BR_H - rh) * 2 - (x == rx + rw - 1))
    return c


# =============================================================================================
# Keys, pins, writer
# =============================================================================================
KEYS = {
    "act1_grant_box": [("act1_grant_box_closed", lambda: act1_grant_box("closed")),
                       ("act1_grant_box_open", lambda: act1_grant_box("open"))],
    "act1_gallery_breach": [("act1_gallery_breach", act1_gallery_breach)],
}
FRAMED = {"act1_grant_box": [100, 100]}


def key_sha() -> dict:
    """The key pins: SHA-256 of each key's pixel list (the execution's byte-identity check)."""
    return {pid: hashlib.sha256(json.dumps(fn().px).encode()).hexdigest()
            for pieces in KEYS.values() for pid, fn in pieces}


def write() -> None:
    for stem, pieces in KEYS.items():
        imgs = [fn() for _, fn in pieces]
        grid = kit.grid_from(imgs, layer_names=["main"], durations=FRAMED.get(stem, [100] * len(imgs)),
                             pivot=(0, 0))
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


def _bbox(img: Pix):
    opaque = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x]]
    xs, ys = [p[0] for p in opaque], [p[1] for p in opaque]
    return (min(xs), min(ys), max(xs), max(ys)), len(opaque)


def measurements() -> list[str]:
    """The numbers the gate and the review check against spec section 12 (key part)."""
    out = []
    for stem, pieces in KEYS.items():
        for pid, fn in pieces:
            img = fn()
            cols = {p for row in img.px for p in row if p}
            (x0, y0, x1, y1), n = _bbox(img)
            out.append(f"{pid} {img.w}x{img.h} opaque={n} bbox=({x0},{y0})-({x1},{y1}) colours={len(cols)} "
                       f"max_luma={max(_luma(*c) for c in cols):.0f} transparent_px={img.w * img.h - n}")
    for s in ("closed", "open"):
        b, _ = _bbox(act1_grant_box(s))
        out.append(f"act1_grant_box_{s} floor contact row {b[3]} = room y {BOX_AT[1] + b[3]} (target 579): "
                   f"{'ok' if BOX_AT[1] + b[3] == 579 else 'FAIL'}; x {BOX_AT[0] + b[0]}..{BOX_AT[0] + b[2]} "
                   f"(grant zone 336..376, guide drawn from 368)")
    void = _breach_void()
    ys = [y for _, y in void]
    xs_low = [x for x, y in void if y == BR_H - 1]
    out.append(f"act1_gallery_breach void rows {min(ys)}..{max(ys)} = room y {BR_AT[1] + min(ys)}..{BR_AT[1] + max(ys)} "
               f"(target: reaches 559); floor-row void x {BR_AT[0] + min(xs_low)}..{BR_AT[0] + max(xs_low)}")
    core = [(x, y) for y in range(80, BR_H) for x in range(40, BR_W - 40)]
    out.append(f"act1_gallery_breach core {{736,422,112,138}} void cover "
               f"{sum(p in void for p in core) / len(core):.3f} (target 1.000)")
    pal = set()
    for pieces in KEYS.values():
        for _, fn in pieces:
            pal |= {p for row in fn().px for p in row if p}
    out.append(f"all keys: {len(pal)} distinct palette entries (per-atlas budget 63)")
    out += [f"KEY {pid} sha256={sha}" for pid, sha in key_sha().items()]
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


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "keys":
        for pid, sha in key_sha().items():
            print(f"KEY {pid} {sha}")
    elif len(sys.argv) > 1 and sys.argv[1] == "measure":
        print("\n".join(measurements()))
    else:
        write()
