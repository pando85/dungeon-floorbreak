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


# =============================================================================================
# Execution (spec sections 5, 13 and 16): the executor pieces, the atlases, the manifests, the contract.
# The keys above stay untouched (their pins are in docs/evidence/area-act1-01/gate/measurements.txt).
# =============================================================================================
import author_heat01 as heat  # noqa: E402  (AREA-HEAT-01: the shelf body the step faces derive from)
import author_trogarea01 as trog  # noqa: E402  (AREA-TROG-01: the step rule, ``end_of`` and ``darken``)

TERRAIN_GRID = ROOT / "assets/source/terrain/floor01/terrain.grid.json"
SOFFIT_GRID = ROOT / "assets/source/terrain/floor01/soffit.grid.json"
ENV02_LAYOUT = ROOT / "assets/levels/floor01/opening_route_proposal.json"
CATACOMB_KIT = ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json"
OPENING_KIT = ROOT / "assets/source/terrain/floor01/floor01_opening_kit.json"
REFUGE_KIT = ROOT / "assets/source/terrain/floor01/floor01_refuge_kit.json"
SOFFIT_ROWS = range(0, 6)                      # ENV-02 layout rows 0..5 (room y 272..367)
SOFFIT_COLS = range(61, 105)                   # columns 61..104 (room x 976..1679)
SOFFIT_AT = (976, 272)
SOFFIT_SLICES = (976, 1152, 1328, 1504)        # four 176 px slices
SOFFIT_NOTCHED: list = []                      # slices that got the one cleared corner pixel (recorded)


def _grid_pixels(path: Path) -> list:
    """The atlas grid at ``path`` as rows of (ramp, index) or None, every colour checked against the palette."""
    g = json.loads(path.read_text(encoding="utf-8"))
    names = {}
    for entry in g["palette"]:
        ramp, _, idx = entry["name"].rpartition("_")
        if ramp not in RAMPS or RAMPS[ramp][int(idx)] + "ff" != entry["rgba"]:
            raise SystemExit(f"{path}: colour {entry['name']} {entry['rgba']} is not on floor01-catacomb-v2")
        names[entry["key"]] = (ramp, int(idx))
    layer = g["layers"][0]
    return [[names[ch] if ch != "." else None for ch in row] for row in g["frames"][0]["cels"][layer]]


def _tile_rects() -> dict:
    """{tile id: (grid pixels name, rect)} for the terrain and soffit tiles of the catacomb and opening kits."""
    out = {}
    for kit_path in (CATACOMB_KIT, OPENING_KIT):
        for t in json.loads(kit_path.read_text(encoding="utf-8"))["tiles"]:
            out[t["id"]] = (t["atlas"], t["rect"])
    return out


def soffit_image() -> Pix:
    """The ENV-02 R soffit (collision rows 0..5, columns 61..104) copied pixel for pixel: 704 x 96."""
    layout = json.loads(ENV02_LAYOUT.read_text(encoding="utf-8"))
    col = layout["tile_layers"]["collision"]
    grids = {"floor01_terrain": _grid_pixels(TERRAIN_GRID), "floor01_soffit": _grid_pixels(SOFFIT_GRID)}
    tiles = _tile_rects()
    img = Pix(len(SOFFIT_COLS) * 16, len(SOFFIT_ROWS) * 16)
    for r, row in enumerate(SOFFIT_ROWS):
        for c, cc in enumerate(SOFFIT_COLS):
            tid = col[row][cc]
            if tid is None:
                continue
            atlas, (rx, ry, rw, rh) = tiles[tid]
            for y in range(rh):
                for x in range(rw):
                    img.px[r * 16 + y][c * 16 + x] = grids[atlas][ry + y][rx + x]
    return img


def _soffit_slice(k: int) -> Pix:
    full = soffit_image()
    c = Pix(176, 96)
    for y in range(96):
        for x in range(176):
            c.px[y][x] = full.px[y][k * 176 + x]
    if all(p is not None for row in c.px for p in row):
        c.clear(0, 0)                       # the one transparent pixel (every piece needs one), top-left
        if k not in SOFFIT_NOTCHED:
            SOFFIT_NOTCHED.append(k)
    return c


def act1_soffit_a() -> Pix:
    return _soffit_slice(0)


def act1_soffit_b() -> Pix:
    return _soffit_slice(1)


def act1_soffit_c() -> Pix:
    return _soffit_slice(2)


def act1_soffit_d() -> Pix:
    return _soffit_slice(3)


def _convert(p) -> Pix:
    c = Pix(p.w, p.h)
    for y in range(p.h):
        for x in range(p.w):
            c.px[y][x] = p.px[y][x]
    return c


def _step_body() -> "heat.Pix":
    """The 16 x 16 hump face: the top 16 rows of the heat shelf body (arris, slab lip, shadow row and the first
    course's head), one stone step darker (the trog step rule, TROG-ART-AD5), the transparent nick moved to
    its bottom right corner (ACT1-ART-AD15)."""
    src = heat._shelf_body()
    c = heat.Pix(16, 16)
    for y in range(16):
        for x in range(16):
            c.px[y][x] = src.px[y][x]
    c.clear(15, 15)
    return trog.darken(c)


def act1_step_mid() -> Pix:
    return _convert(_step_body())


def act1_step_end_l() -> Pix:
    return _convert(trog.end_of(_step_body(), True))


def act1_step_end_r() -> Pix:
    return _convert(trog.end_of(_step_body(), False))


OPENING_DECAL_GRID = ROOT / "assets/source/terrain/floor01/opening_decal.grid.json"
NICHE_RECT = (0, 0, 96, 96)                   # the ENV-02 `alcove_niche` rect in `floor01_opening_decal`


def act1_alcove_niche() -> Pix:
    """The ENV-02 ``alcove_niche`` pixels copied exactly (no new pixel). The contract validator refuses two pieces
    on one atlas rect, so the `structure` re-declaration is a copy in the act-1 atlas (ACT1-ART-AD14)."""
    rows = _grid_pixels(OPENING_DECAL_GRID)
    x0, y0, w, h = NICHE_RECT
    c = Pix(w, h)
    for y in range(h):
        for x in range(w):
            c.px[y][x] = rows[y0 + y][x0 + x]
    return c


FLOOR_Y = 580
HUMP_Y = 564
STEP_RUNS = ((1168, 64), (1456, 64), (2064, 96))        # (x0, width): the two humps and the alcove ledge statics

# The act-1 atlases (stem = the source file stem under assets/source/props/floor01).
ATLASES = {
    "floor01_act1_keys": {"stem": "act1_keys", "width": 192, "pieces": [
        ("act1_gallery_breach", act1_gallery_breach), ("act1_grant_box_closed", lambda: act1_grant_box("closed")),
        ("act1_grant_box_open", lambda: act1_grant_box("open"))]},
    "floor01_act1_soffit": {"stem": "act1_soffit", "width": 176, "pieces": [
        ("act1_soffit_a", act1_soffit_a), ("act1_soffit_b", act1_soffit_b), ("act1_soffit_c", act1_soffit_c),
        ("act1_soffit_d", act1_soffit_d)]},
    "floor01_act1_props": {"stem": "act1_props", "width": 104, "pieces": [
        ("act1_alcove_niche", act1_alcove_niche),
        ("act1_step_mid", act1_step_mid), ("act1_step_end_l", act1_step_end_l), ("act1_step_end_r", act1_step_end_r)]},
}
LAYER_OF_ATLAS = {"floor01_act1_keys": ("structure", "structure"), "floor01_act1_soffit": ("structure", "structure"),
                  "floor01_act1_props": ("structure", "structure")}
LAYER_OVERRIDE = {"act1_grant_box_closed": ("prop", "prop"), "act1_grant_box_open": ("prop", "prop")}
PAD = 8
EXEC_PIECES = ("act1_soffit_a", "act1_soffit_b", "act1_soffit_c", "act1_soffit_d", "act1_step_mid",
               "act1_step_end_l", "act1_step_end_r", "act1_alcove_niche")
KEY_PIECE_IDS = tuple(pid for pieces in KEYS.values() for pid, _ in pieces)
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_act1_kit.json"
MANIFESTS = ROOT / "assets/manifests"
OWNER = ("AI-authored environment pixel art (AREA-ACT1-01: design spec and keys Claude Opus, silhouette gate "
         "Claude Opus (the orchestrator, independent); execution Claude Sonnet; independent review pending)")
REFUGE_ATLASES = ("floor01_refuge_hearth", "floor01_refuge_candle", "floor01_refuge_props", "floor01_refuge_wall")
REFUGE_PIECES = ("refuge_hearth", "refuge_hearth_flue", "refuge_desk", "refuge_candle_flame", "refuge_stool",
                 "refuge_barrel", "refuge_wall_map", "refuge_bookshelf")
NICHE_LIGHTS = [{"id": "alcove_glow", "offset": [48, 56], "colour": "#f2b060", "radius": 96, "intensity": 0.4,
                 "group": "scene"}]


def _at(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def _step_run(x0: int, width: int) -> list:
    """end_l, mids, end_r side by side: 8 + 16 n + 8 = width, the static's width, at y 564."""
    n = (width - 16) // 16
    out = [("structure", "act1_step_end_l", x0, HUMP_Y)]
    out += [("structure", "act1_step_mid", x0 + 8 + 16 * i, HUMP_Y) for i in range(n)]
    out.append(("structure", "act1_step_end_r", x0 + width - 8, HUMP_Y))
    return out


# -- the dressing of `act1_chase` (spec section 3.1) ---------------------------------------------------------------
CHASE_REMOVE_AT = [
    ("structure", "arch_threshold", 576, 544), ("structure", "arch_chain", 592, 384),                 # S2: bricked bay
    ("prop", "torch_wall", 768, 448), ("prop", "torch_wall", 824, 432), ("prop", "torch_wall", 864, 448),
    ("backwall_decal", "torch_pool_b", 664, 364), ("backwall_decal", "torch_pool_b", 720, 348),
    ("backwall_decal", "torch_pool_a", 760, 364),                                                     # S2: pier_r torches
    ("prop", "torch_wall", 1104, 448), ("prop", "torch_wall", 1160, 432), ("prop", "torch_wall", 1440, 448),
    ("prop", "torch_wall", 1496, 432),                                                                # S3: inside a set
    ("backwall_decal", "torch_pool_a", 1000, 364), ("backwall_decal", "torch_pool_b_m", 1056, 348),
    ("backwall_decal", "torch_pool_b", 1336, 364), ("backwall_decal", "torch_pool_a_m", 1392, 348),
    ("prop", "torch_wall", 2112, 448), ("backwall_decal", "torch_pool_a", 2008, 364),                 # S4: over the ledge
]
CHASE_FILL = [{"x": 584, "y": 352, "w": 152, "h": 208}]
CHASE_PLACE = [
    ("structure", "act1_gallery_breach", *BR_AT),
    *[("structure", f"act1_soffit_{c}", x, SOFFIT_AT[1]) for c, x in zip("abcd", SOFFIT_SLICES)],
    *_step_run(*STEP_RUNS[0]), *_step_run(*STEP_RUNS[1]),
    ("structure", "act1_alcove_niche", 2064, 468),
    *_step_run(*STEP_RUNS[2]),
]

# -- the dressing of `act1_shore` (spec section 3.2) ---------------------------------------------------------------
SHORE_REMOVE_AT = [("prop", "torch_wall", 176, 448), ("backwall_decal", "torch_pool_b", 72, 364)]

# -- the dressing of `act1_guild` (spec section 3.3) ---------------------------------------------------------------
GUILD_REMOVE_AT = [("prop", "torch_wall", 48, 432), ("backwall_decal", "torch_pool_a_m", -56, 348)]
GUILD_PLACE = [
    ("backwall_decal", "refuge_hearth_flue", 80, 356 + 16 * i) for i in range(9)
] + [
    ("backwall_decal", "refuge_wall_map", 474, 484),
    ("prop", "refuge_hearth", 64, 500), ("prop", "refuge_barrel", 8, 552), ("prop", "refuge_stool", 150, 562),
    ("prop", "refuge_desk", 272, 554), ("prop", "refuge_candle_flame", 272 + 39, 554),
    ("prop", "refuge_bookshelf", 474, 524),
]


def dressing() -> dict:
    return {
        "act1_chase": {"remove": [], "remove_at": [_at(*r) for r in CHASE_REMOVE_AT], "backwall_fill": CHASE_FILL,
                       "place": [_at(*t) for t in CHASE_PLACE], "parallax": {}},
        "act1_shore": {"remove": [], "remove_at": [_at(*r) for r in SHORE_REMOVE_AT], "backwall_fill": [],
                       "place": [], "parallax": {}},
        "act1_guild": {"remove": [], "remove_at": [_at(*r) for r in GUILD_REMOVE_AT], "backwall_fill": [],
                       "place": [_at(*t) for t in GUILD_PLACE], "parallax": {}},
    }


def build_pieces(name: str) -> list:
    return [(pid, fn()) for pid, fn in ATLASES[name]["pieces"]]


def _pack(pieces: list, width: int, pad: int = 0):
    """Shelf packing, left to right: a piece's rect holds exactly its pixels (a key stays byte-identical)."""
    x = y = shelf = 0
    placed = []
    for pid, img in pieces:
        if x + img.w > width:
            x, y, shelf = 0, y + shelf + pad, 0
        placed.append((pid, img, x, y))
        x += img.w + pad
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


def piece_entry(pid: str, atlas: str, rect: list) -> dict:
    role, layer = LAYER_OVERRIDE.get(pid) or LAYER_OF_ATLAS[atlas]
    return {"id": pid, "atlas": atlas, "rect": rect, "role": role, "layer": layer, "collidable": False,
            "anchor": [0, 0], "anchor_name": "top_left (room placement, spec section 5)", "grid_snap": 1,
            "grammar": ["A"]}


def contract(rects: dict, colours: dict) -> dict:
    refuge = json.loads(REFUGE_KIT.read_text(encoding="utf-8"))
    atlases, pieces = {}, []
    for name, spec in ATLASES.items():
        atlases[name] = {"grid": f"assets/source/props/floor01/{spec['stem']}.grid.json",
                         "manifest": f"assets/manifests/{name}.json", "image": f"generated/art/{name}.png",
                         "metadata": f"generated/art/{name}.json"}
        for pid, _ in spec["pieces"]:
            entry = piece_entry(pid, name, rects[name][pid])
            if pid == "act1_alcove_niche":
                entry["lights"] = NICHE_LIGHTS
            pieces.append(entry)
    for name in REFUGE_ATLASES:                       # re-declared by the same definition (ACT1-ART-AD13)
        atlases[name] = refuge["atlases"][name]
    by_id = {p["id"]: p for p in refuge["pieces"]}
    pieces += [by_id[pid] for pid in REFUGE_PIECES]
    palette_use = {n: colours[n] for n in ATLASES}
    palette_use.update({n: refuge["palette_use"][n] for n in REFUGE_ATLASES})
    return {
        "format": "env-kit-v2", "kit_id": "floor01_act1",
        "extends": "assets/source/terrain/floor01/floor01_opening_kit.json",
        "status": "proposed - AREA-ACT1-01 kit (docs/02-art/area-act1-01-spec.md); independent review pending; "
                  "not owner-approved",
        "extension_note": (
            "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema from the "
            "ENV-01 contract through the opening kit (the timber sets, the full barricade, the alcove niche and the "
            "guild portal are its pieces). Adds the act-1 atlases for Floor 01 rooms `act1_chase`, `act1_shore` and "
            "`act1_guild`: no tiles, nothing collidable, no new palette colour. The refuge atlases and the refuge "
            "pieces the guild uses are declared again by the same definitions (the contract chain has no sibling "
            "reference, ACT1-ART-AD13). The chase's dressing bricks up the S2 bay behind the gallery breach; the "
            "timbers, the barricade, the grant box and the guild portal's open overlay are presenter sprites, "
            "not placed pieces."),
        "palette_use": palette_use,
        "atlases": atlases, "tiles": [], "pieces": pieces,
        "room_dressing": dressing(),
        "usage_rules": {
            "R1_act1_chase": (
                "room `act1_chase` only: the dressing never adds or moves a tile or a solid; the S2 bay 584..735 is "
                "bricked up behind the breach; the soffit slices and the step faces are non-collidable `structure` "
                "pieces on the sim rects; the timbers, the barricade and the machine are the presenter's"),
            "R2_act1_shore": "room `act1_shore` only: one torch and its pool are removed; the wedge is the presenter's",
            "R3_act1_guild": (
                "room `act1_guild` only: the furniture is the refuge's; the hearth stands on the spawn side, clear of "
                "the grant zone 336..376, the guide and the door; no piece on the ceiling slab y 336..351"),
            "R4_presenter_pieces": (
                "`act1_grant_box_closed` / `_open` are not placed: the scene draws them at (340, 552) from the "
                "guild door's lock state")},
    }


def manifest(name: str, canvas: list, stem: str) -> dict:
    origin = {
        "floor01_act1_keys": "Original pixel-grid-v1 keys for the Floor 1 act-1 rooms (AREA-ACT1-01 on the ENV-01 kit): "
                             "the gallery breach and the grant strongbox, rule-based shapes and ramps",
        "floor01_act1_soffit": "The ENV-02 R soffit (collision rows 0..5, columns 61..104 of the opening route "
                               "proposal) copied pixel for pixel from the ENV-01 and ENV-02 tile atlases, no new pixel",
        "floor01_act1_props": "The hump and ledge faces (AREA-ACT1-01), derived from the AREA-HEAT-01 shelf body by the "
                              "AREA-TROG-01 step rule",
    }[name]
    return {
        "schema_version": 1, "id": f"{name}_v1", "type": "decor_atlas",
        "source_path": f"assets/source/props/floor01/{stem}.ase",
        "grid_source": f"assets/source/props/floor01/{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png", "output_metadata": f"generated/art/{name}.json",
        "palette_id": "floor01-catacomb-v2", "frame_size": canvas, "pivot": [0, 0],
        "animation_tags": [], "required_alpha": True,
        "kit_contract": "assets/source/terrain/floor01/floor01_act1_kit.json",
        "owner": OWNER, "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack "
                   "distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": origin + "; nothing cropped, traced or resampled from concept art; no text, logos, brands, "
                               "faces or likenesses",
            "generator": "Deterministic authoring script scripts/author_act1area01.py written by AI agents (spec "
                         "and keys designed by Claude Opus; execution by Claude Sonnet); no image-model raster "
                         "output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1 act-1 panels (mood reference only; no pixel cropped, traced or resampled)"]},
    }


def write_kit() -> None:
    """Write every atlas grid + .ase, manifest and the contract."""
    rects, colours = {}, {}
    for name, spec in ATLASES.items():
        sheet, r = _pack(build_pieces(name), spec["width"], PAD)
        rects[name] = r
        grid = kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0))
        path = SRC / f"{spec['stem']}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                        str(SRC / f"{spec['stem']}.ase")], check=True)
        colours[name] = len(grid["palette"])
        (MANIFESTS / f"{name}.json").write_text(
            json.dumps(manifest(name, grid["canvas"], spec["stem"]), indent=1) + "\n", encoding="utf-8")
        print(f"{name}: {grid['canvas']} {len(grid['palette'])} colours")
    CONTRACT.write_text(json.dumps(contract(rects, colours), indent=1) + "\n", encoding="utf-8")
    print(f"soffit slices with the one cleared corner pixel: {SOFFIT_NOTCHED}")


def all_pieces() -> dict:
    """{piece id: Pix} for every piece of the act-1 atlases (keys and executor pieces)."""
    out = {}
    for name in ATLASES:
        out.update(dict(build_pieces(name)))
    return out


def mean_luma(img) -> float:
    vals = [_luma(*p) for row in img.px for p in row if p]
    return sum(vals) / len(vals)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "keys":
        for pid, sha in key_sha().items():
            print(f"KEY {pid} {sha}")
    elif len(sys.argv) > 1 and sys.argv[1] == "measure":
        print("\n".join(measurements()))
    else:
        write()
        write_kit()
