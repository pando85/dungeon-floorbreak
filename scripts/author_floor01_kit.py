#!/usr/bin/env python3
"""Authoring record for the ENV-01 Floor 1 catacomb kit, rework round 3 (A1 target).

This deterministic *authoring aid* writes the kit's editable ``pixel-grid-v1``
sources, their asset manifests, the ``env-kit-v2`` integration contract and the
``env-kit-layout-v2`` sample corridor:

- ``assets/source/terrain/floor01/terrain.grid.json``    collision tiles (deep floor, steps, ceiling, wall ends)
- ``assets/source/terrain/floor01/backwall.grid.json``   brick backwall tiles (four height bands + base course)
- ``assets/source/terrain/floor01/walldecal.grid.json``  torch-pool, grime and rubble pieces
- ``assets/source/terrain/floor01/structure.grid.json``  arch rings, piers, threshold, pilaster
- ``assets/source/terrain/floor01/parallax.grid.json``   far and mid gallery strips (tile horizontally)
- ``assets/source/props/floor01/props.grid.json``        copper/brass pipe set, door, unlit sconce, lichen
- ``assets/source/props/floor01/torch.grid.json``        4-frame wall torch loop
- ``assets/manifests/floor01_*.json``                    one manifest per atlas
- ``assets/source/terrain/floor01/floor01_catacomb_kit.json`` integration contract (env-kit-v2)
- ``assets/levels/floor01/corridor_abc_sample.json``     sample layout (env-kit-layout-v2)

The look matches the owner-approved A1 look-dev frame (2026-11-16): warm orange
torch-lit brick falling into cold blue-violet shadow, thick copper/brass pipes, a
thick sculpted arch and a deep layered gallery. Pixels are produced by the same
deterministic "pixel shader" as the A1 look-dev (``scripts/lookdev_floor01_author.py``,
imported here as the shared shading core): every surface pixel has a position, a
depth and a normal, light is evaluated with a windowed falloff and quantised onto
hand-picked hue-shifted ramps, and flat faces are shaded per brick/block so the
art keeps clean clusters. The arch, piers and gallery strips are the A1 look-dev
geometry re-rendered as modules (with a local light rig); everything else is
rebuilt as tile-aligned modules.

Nothing is cropped, traced or resampled from the concept images; they were mood
references for the A1 frame only and are never read by any script.

The committed grids are the source of truth. Re-running overwrites hand edits, so
only re-run deliberately (``--force``).
"""
from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import lookdev_floor01_author as ld  # noqa: E402  (shared A1 shading core)

TERRAIN_DIR = ROOT / "assets/source/terrain/floor01"
PROPS_DIR = ROOT / "assets/source/props/floor01"
LEVEL_DIR = ROOT / "assets/levels/floor01"
MANIFEST_DIR = ROOT / "assets/manifests"
CONTRACT_PATH = TERRAIN_DIR / "floor01_catacomb_kit.json"
LAYOUT_PATH = LEVEL_DIR / "corridor_abc_sample.json"
CONTRACT_REL = "assets/source/terrain/floor01/floor01_catacomb_kit.json"

KIT_ID = "floor01_catacomb"
PALETTE_ID = "floor01-catacomb-v2"
TILE = 16
MAX_ATLAS_COLOURS = 63  # validate_assets.py counts transparency as one of its 64

# ---------------------------------------------------------------------------
# Palette: the A1 look-dev ramps plus timber and lichen (index 0 = darkest).
# ---------------------------------------------------------------------------
EXTRA_RAMPS = {
    "wood": ["#170d0c", "#261612", "#3a2219", "#52301f", "#6d4127", "#8d5631"],
    "lichen": ["#1e2a24", "#34452f", "#58703f"],
}
for _name, _ramp in EXTRA_RAMPS.items():
    ld.RAMPS.setdefault(_name, _ramp)
RAMPS = ld.RAMPS
RAMP_ORDER = list(RAMPS)
GRID_KEYS = "".join(chr(c) for c in range(33, 127) if chr(c) not in '."\\')


def colour_name(ramp: str, index: int) -> str:
    return f"{ramp}_{index:02d}"


def combined_palette() -> list[dict]:
    return [{"name": colour_name(r, i), "rgba": RAMPS[r][i] + "ff", "ramp": r, "index": i}
            for r in RAMP_ORDER for i in range(len(RAMPS[r]))]


# ---------------------------------------------------------------------------
# Kit geometry (layout pixels, y down). Design units of the A1 frame map to
# layout pixels with x' = x + bay offset and y' = y + DY.
# ---------------------------------------------------------------------------
DY = 20                 # A1 design y -> layout y
CEILING_ROWS = (0, 1)   # row 1 carries the cornice
WALL_TOP = 32           # first backwall pixel row
BANDS = {               # brick height bands: tile rows and base value on the brick ramp
    "top": ((2, 3), 1.7, 0.4),
    "upper": ((4, 7), 3.1, 0.9),
    "mid": ((8, 12), 4.1, 1.2),
    "lower": ((13, 14), 5.3, 1.8),
    "low": ((15, 16), 7.0, 2.6),   # lit band behind actor heads (floor-pool bounce)
}
BASE_COURSE_ROW = 17    # pale dressed-stone course behind actor height
FLOOR_ROW = 18          # deck_back row of the base floor (floor back line y = 288)
WALK_LINE_Y = 4         # walking line inside a deck_front tile (y = 308 on the base floor)
WALL_TORCH_FLAME_Y = 178
PIER_TORCH_FLAME_Y = 194
TORCH_FLAME_LOCAL = (8, 18)  # flame centre inside the 16x56 torch piece
POOL_RADIUS = 118.0
POOL_W, POOL_H = 240, 224
POOL_TOP_Y = 48
POOL_CENTRE_LOCAL = (120, WALL_TORCH_FLAME_Y - POOL_TOP_Y)

# Deck geometry relative to the floor back line (y = 0): slab rows, lip, face courses.
SLAB_ROWS = [0, 4, 9, 16, 24]
LIP = (24, 26)
FACE_COURSES = [26, 48, 64]   # course 1 = 26..48, course 2 = 48..64, fill below
SLAB_W = 32
DECK_TONES = {"dim": 0.28, "mid": 0.42, "lit": 0.58}
DECK_NEUTRAL = 0.42
ROW_TYPES = ["deck_back", "deck_front", "face_a", "face_b", "fill"]

# Arch bay (A1 geometry, design x 184..488 -> bay origin + 0..304)
BAY_DESIGN_X = 184
BAY_W = 304
ARCH_PIER_TORCHES = [{"x": 216.0, "y": float(PIER_TORCH_FLAME_Y - DY), "z": 40.0, "p": 0.85},
                     {"x": 456.0, "y": float(PIER_TORCH_FLAME_Y - DY), "z": 40.0, "p": 0.85}]


# ---------------------------------------------------------------------------
# Canvas helpers (ld.Canvas keeps (ramp, index) per pixel)
# ---------------------------------------------------------------------------

def canvas(w: int, h: int, ox: float = 0.0, oy: float = 0.0, name: str = "kit") -> "ld.Canvas":
    ld.DENSITIES["kit"] = {"s": 1.0, "canvas": (w, h), "origin": (ox, oy), "views": {}}
    return ld.Canvas(name, "kit")


def blit(dst: "ld.Canvas", src: "ld.Canvas", x: int, y: int, mirror_x: bool = False,
         mirror_y: bool = False) -> None:
    for sy in range(src.h):
        for sx in range(src.w):
            p = src.px[sy][sx]
            if p is None:
                continue
            tx = x + (src.w - 1 - sx if mirror_x else sx)
            ty = y + (src.h - 1 - sy if mirror_y else sy)
            dst.px[ty][tx] = p


def crop(src: "ld.Canvas", x: int, y: int, w: int, h: int) -> "ld.Canvas":
    out = canvas(w, h)
    for yy in range(h):
        for xx in range(w):
            if 0 <= x + xx < src.w and 0 <= y + yy < src.h:
                out.px[yy][xx] = src.px[y + yy][x + xx]
    return out


def put(c: "ld.Canvas", x: int, y: int, ramp: str, value: float) -> None:
    c.put(x, y, ramp, value)


@contextmanager
def light_rig(torches: list[dict], bounce: dict | None = None, **overrides):
    """Temporarily swap the shared shading core's light list and geometry globals."""
    saved_t = list(ld.TORCHES)
    saved_b = dict(ld.COLD_BOUNCE)
    saved_g = {k: getattr(ld, k) for k in overrides}
    ld.TORCHES[:] = torches
    if bounce is not None:
        ld.COLD_BOUNCE.clear()
        ld.COLD_BOUNCE.update(bounce)
    for k, v in overrides.items():
        setattr(ld, k, v)
    try:
        yield
    finally:
        ld.TORCHES[:] = saved_t
        ld.COLD_BOUNCE.clear()
        ld.COLD_BOUNCE.update(saved_b)
        for k, v in saved_g.items():
            setattr(ld, k, v)


KEY_DIR = ld.norm(-0.35, -0.62, 0.7)  # generic key from upper-left-front for bevels


def key_term(n: tuple) -> float:
    """Bevel shading relative to a flat face (positive on lit edges)."""
    return (n[0] * KEY_DIR[0] + n[1] * KEY_DIR[1] + n[2] * KEY_DIR[2]) - KEY_DIR[2]


# ---------------------------------------------------------------------------
# Brick masonry (16x8 running bond, joints at x = 0 / 8 mod 16, rows at y = 0 mod 8)
# ---------------------------------------------------------------------------
BRICK_W, BRICK_H = 16, 8


def brick_cell(gx: int, gy: int) -> tuple[int, int, int, int]:
    row = gy // BRICK_H
    off = (BRICK_W // 2) * (row % 2)
    col = (gx + off) // BRICK_W
    return row, col, (gx + off) % BRICK_W, gy % BRICK_H


def brick_pixel(c, px, py, du, dv, *, base, mortar_base, jit, seed, torch=None,
                soot=0.0, pits=True, chips=True, x=0.0, y=0.0) -> bool:
    """Shade one brick-cell pixel. ``torch(x, y, n) -> extra value``. Returns True if drawn."""
    t_face = torch(None, None, (0.0, 0.0, 1.0)) if torch else 0.0
    if du == 0 or dv == 0:
        put(c, px, py, "mortar", mortar_base + 0.5 * t_face - soot * 0.5)
        return True
    if chips:
        chip = ld.rnd(seed, "chip")
        if chip < 0.14:
            corner = int(chip * 28) % 4
            cu = du - 1 if corner in (0, 2) else BRICK_W - 1 - du
            cv = dv - 1 if corner in (0, 1) else BRICK_H - 1 - dv
            if cu + cv < 1:
                put(c, px, py, "mortar", mortar_base + 0.5 * t_face - soot * 0.5 + 0.6)
                return True
    n, edge = ld.bevel_normal(du - 1, dv - 1, BRICK_W - 1, BRICK_H - 1, 1)
    value = base + jit + 1.5 * key_term(n) - soot
    if torch:
        value += torch(x, y, n) if edge else t_face
    if pits and not edge:
        count = 1 + int(ld.rnd(seed, "n") * 2.2)
        for k in range(count):
            pu = 2 + int(ld.rnd(seed, k, "u") * (BRICK_W - 6))
            pv = 2 + int(ld.rnd(seed, k, "v") * (BRICK_H - 5))
            if pu <= du < pu + 2 and dv == pv:
                value -= 1.2
    put(c, px, py, "brick", value)
    return True


def brick_jitter(seed) -> float:
    jit = (ld.rnd(seed) - 0.5) * 1.3
    special = ld.rnd(seed, "kind")
    if special < 0.05:
        jit -= 1.1
    elif special > 0.97:
        jit += 0.8
    return jit


def brick_panel(band: str, variant: str, width_tiles: int) -> "ld.Canvas":
    """One tile row of a brick band. Bricks cut by the panel edge get a neutral tone."""
    _rows, base, mortar = BANDS[band]
    w = width_tiles * TILE
    c = canvas(w, TILE)
    for py in range(TILE):
        for px in range(w):
            row, col, du, dv = brick_cell(px, py)
            off = (BRICK_W // 2) * (row % 2)
            start = col * BRICK_W - off
            straddler = start < 0 or start + BRICK_W > w
            seed = ("brick", band, variant, row, col)
            jit = 0.0 if straddler else brick_jitter(seed)
            brick_pixel(c, px, py, du, dv, base=base, mortar_base=mortar, jit=jit,
                        seed=seed, pits=not straddler, chips=not straddler)
    return c


def band_of_row(row: int) -> str:
    for name, ((r0, r1), _b, _m) in BANDS.items():
        if r0 <= row <= r1:
            return name
    raise ValueError(row)


# (radius, power, light x offset): a full and a smaller pool; "_m" variants mirror the
# off-centre light, the soot lean and the ragged-edge noise (bricks are re-shaded, not
# pixel-mirrored, so the running bond stays on the wall grid).
POOL_VARIANTS = {"a": (POOL_RADIUS, 1.3, 5.0), "b": (102.0, 1.18, -4.0),
                 "a_m": (POOL_RADIUS, 1.3, -5.0), "b_m": (102.0, 1.18, 4.0)}


def pool_torch_value(t: dict, x: float, y: float, n: tuple) -> float:
    radius, power, dx = POOL_VARIANTS[t["variant"]]
    lt = {"x": t["x"] + dx, "y": t["y"], "z": t["z"], "p": t["p"]}
    w = ld.warm_light(x, y, 0.0, n, torches=[lt], radius=radius, power=power)
    return 9.2 * (w ** 0.7)


def torch_pool(variant: str) -> "ld.Canvas":
    """Warm brick pool around a wall torch: whole bricks only (brick-shaped 1-bit alpha).

    Placed with its top-left at (flame_x - 120, 48); the brick tones include the band
    base of every row, the torch light and soot above the flame. Bricks that would not
    change by at least ~0.55 ramp steps are left out so the pool fades into any panel.
    """
    c = canvas(POOL_W, POOL_H)
    cx, cy = POOL_CENTRE_LOCAL
    t = {"x": float(cx), "y": float(cy), "z": 16.0, "p": 1.0, "variant": variant}
    bricks: dict[tuple[int, int], list[tuple[int, int, int, int]]] = {}
    for py in range(POOL_H):
        for px in range(POOL_W):
            row, col, du, dv = brick_cell(px, py)
            bricks.setdefault((row, col), []).append((px, py, du, dv))
    for (row, col), pixels in sorted(bricks.items()):
        off = (BRICK_W // 2) * (row % 2)
        start = col * BRICK_W - off
        if start < 0 or start + BRICK_W > POOL_W:
            continue  # only whole bricks
        bx, by = start + BRICK_W / 2, row * BRICK_H + BRICK_H / 2
        tile_row = (POOL_TOP_Y + row * BRICK_H) // TILE
        if tile_row > BANDS["low"][0][1]:
            continue
        band = band_of_row(tile_row)
        _r, base, mortar = BANDS[band]
        lift = pool_torch_value(t, bx, by, (0.0, 0.0, 1.0))
        dy = cy - by
        mirror = variant.endswith("_m")
        lean = (-1.0 if mirror else 1.0) * 0.08 * max(0.0, dy)  # soot plume leans with the draught
        soot = 0.0
        if 6 < dy < 92 and abs(bx - cx - lean) < 7 + dy * 0.12:
            soot = 1.7 * (1.0 - dy / 92.0)
        mx = POOL_W - 1 - bx if mirror else bx
        noise = ld.rnd("pool_edge", variant.replace("_m", ""), row, int(mx // BRICK_W))
        # ragged edge: above the flame the cut-off rises with height and a per-brick noise
        cut = 0.55 + (max(0.0, dy - 30.0) / 70.0) * 1.6 * noise
        if lift - soot < cut and soot < 0.6:
            continue
        seed = ("pool", variant, row, col)
        jit = brick_jitter(seed)

        def torch(x, y, n, _bx=bx, _by=by):
            if x is None:
                return pool_torch_value(t, _bx, _by, n)
            return 0.5 * pool_torch_value(t, x, y, n) + 0.5 * pool_torch_value(t, _bx, _by, (0, 0, 1))

        for px, py, du, dv in pixels:
            brick_pixel(c, px, py, du, dv, base=base, mortar_base=mortar, jit=jit, seed=seed,
                        torch=torch, soot=soot, x=px + 0.5, y=py + 0.5)
    return c


SPILL_W, SPILL_TOP_Y, SPILL_H = 64, 96, 176


def pier_spill(side: str) -> "ld.Canvas":
    """Narrow warm brick spill beside a pier whose torch has no wall pool next to it.

    ``side`` "l": placed at (bay - 48, 96), lit by the pier_l torch; "r": placed at
    (bay + 288, 96), lit by the pier_r torch. Whole bricks only."""
    c = canvas(SPILL_W, SPILL_H)
    fx = 80.0 if side == "l" else -16.0
    t = {"x": fx, "y": float(PIER_TORCH_FLAME_Y - SPILL_TOP_Y), "z": 40.0, "p": 0.9}
    bricks: dict[tuple[int, int], list] = {}
    for py in range(SPILL_H):
        for px in range(SPILL_W):
            row, col, du, dv = brick_cell(px, py)
            bricks.setdefault((row, col), []).append((px, py, du, dv))
    for (row, col), pixels in sorted(bricks.items()):
        off = (BRICK_W // 2) * (row % 2)
        start = col * BRICK_W - off
        if start < 0 or start + BRICK_W > SPILL_W:
            continue
        bx, by = start + BRICK_W / 2, row * BRICK_H + BRICK_H / 2
        tile_row = (SPILL_TOP_Y + row * BRICK_H) // TILE
        if tile_row > BANDS["low"][0][1]:
            continue
        _r, base, mortar = BANDS[band_of_row(tile_row)]

        def lift_at(x, y, n):
            w = ld.warm_light(x, y, 0.0, n, torches=[t], radius=104.0, power=1.0)
            return 8.0 * (w ** 0.7)

        lift = lift_at(bx, by, (0.0, 0.0, 1.0))
        if lift < 0.55 + 0.6 * ld.rnd("spill_edge", side, row, col):
            continue
        seed = ("spill", side, row, col)
        jit = brick_jitter(seed)

        def torch(x, y, n, _bx=bx, _by=by):
            if x is None:
                return lift_at(_bx, _by, n)
            return 0.5 * lift_at(x, y, n) + 0.5 * lift_at(_bx, _by, (0, 0, 1))

        for px, py, du, dv in pixels:
            brick_pixel(c, px, py, du, dv, base=base, mortar_base=mortar, jit=jit, seed=seed,
                        torch=torch, x=px + 0.5, y=py + 0.5)
    return c


def grime_patch(variant: str) -> "ld.Canvas":
    """A few darkened whole bricks for the mid band (soot and old damp, dry now)."""
    c = canvas(48, 16)
    _r, base, mortar = BANDS["mid"]
    for py in range(16):
        for px in range(48):
            row, col, du, dv = brick_cell(px, py)
            off = (BRICK_W // 2) * (row % 2)
            start = col * BRICK_W - off
            if start < 0 or start + BRICK_W > 48:
                continue
            seed = ("grime", variant, row, col)
            if ld.rnd(seed, "keep") < 0.28:
                continue
            darker = 0.6 + ld.rnd(seed, "d") * 0.6
            brick_pixel(c, px, py, du, dv, base=base - darker, mortar_base=mortar - 0.4,
                        jit=(ld.rnd(seed) - 0.5) * 0.6, seed=seed)
    return c


def base_course(variant: str, width_tiles: int) -> "ld.Canvas":
    """Pale dressed-stone course under the brick: a lit backdrop for actor height."""
    w = width_tiles * TILE
    c = canvas(w, TILE)
    bw = 32
    for py in range(TILE):
        for px in range(w):
            col = px // bw
            du, dv = px % bw, py
            straddler = (col * bw + bw > w)
            seed = ("base", variant, col)
            jit = 0.0 if straddler else (ld.rnd(seed) - 0.5) * 0.9
            if du == 0 or dv == 0:
                put(c, px, py, "mortar", 2.1 if dv else 1.4)
                continue
            if dv == 15:
                put(c, px, py, "stone", 2.0)  # contact shadow line where the course meets the floor
                continue
            n, edge = ld.bevel_normal(du - 1, dv - 1, bw - 1, 14, 1)
            value = 6.8 + jit + 1.6 * key_term(n) - 0.12 * dv  # cooler, darker toward the deck
            if not edge and not straddler:
                pu = 3 + int(ld.rnd(seed, "u") * 22)
                pv = 3 + int(ld.rnd(seed, "v") * 8)
                if pu <= du < pu + 3 and dv == pv:
                    value -= 1.0
            put(c, px, py, "stone", value)
    return c


# ---------------------------------------------------------------------------
# Terrain: deep floor (deck), face courses, fill, steps, ceiling, wall ends
# ---------------------------------------------------------------------------

def deck_pixel(c, px, py, gx, ry, tone_w, seed_base, straddle_fn, ao=0.0) -> None:
    """One pixel of the floor column at relative y ``ry`` (0 = floor back line)."""
    if ry < LIP[0]:
        k = 0
        while ry >= SLAB_ROWS[k + 1]:
            k += 1
        y0, y1 = SLAB_ROWS[k], SLAB_ROWS[k + 1]
        stagger = SLAB_W // 2 if k % 2 else 0
        v = ry - y0
        shift = v // 2          # joints lean 1 px per 2 rows: a receding floor plane (A1)
        max_shift = (y1 - y0 - 1) // 2
        col = (gx - shift + stagger) // SLAB_W
        u = (gx - shift + stagger) % SLAB_W
        straddler = straddle_fn(col * SLAB_W - stagger, SLAB_W + max_shift)
        seed = (seed_base, "slab", k, col)
        w = DECK_NEUTRAL if straddler else tone_w
        jit = 0.0 if straddler else (ld.rnd(seed) - 0.5) * 1.0
        a = ao
        if k == 0:
            a += 1.2 - v * 0.25
        if u == 0 or v == 0:
            put(c, px, py, "mortar", 0.6 + 5.4 * w - a * 0.5)
            return
        value = 1.2 + 9.4 * (w ** 0.8) + jit - a
        if v < 2:
            value += 0.8
        if not straddler:
            pu = ld.rnd(seed, "u")
            if abs(u - (5 + pu * 20)) < 1.0 and v == (y1 - y0) // 2:
                value -= 1.0
        put(c, px, py, "floor", value)
        return
    if ry < LIP[1]:
        w = tone_w * 0.92
        put(c, px, py, "floor", (2.7 if ry == LIP[0] else 1.5) + 9.0 * (w ** 0.8) - ao * 0.4)
        return
    if ry < FACE_COURSES[1]:
        face_pixel(c, px, py, gx, ry - FACE_COURSES[0], 22, 0, 1.0, tone_w, seed_base, straddle_fn, ao)
        return
    if ry < FACE_COURSES[2]:
        face_pixel(c, px, py, gx, ry - FACE_COURSES[1], 16, SLAB_W // 2, 0.42, 0.4, seed_base,
                   straddle_fn, ao)
        return
    raise ValueError(f"deck_pixel covers deck and face rows only (ry {ry})")


def face_pixel(c, px, py, gx, v, vh, stagger, row_gain, tone_w, seed_base, straddle_fn, ao) -> None:
    col = (gx + stagger) // SLAB_W
    u = (gx + stagger) % SLAB_W
    straddler = straddle_fn(col * SLAB_W - stagger, SLAB_W)
    seed = (seed_base, "face", vh, col)
    w = (DECK_NEUTRAL if straddler else tone_w) * 0.42
    jit = 0.0 if straddler else (ld.rnd(seed) - 0.5) * 0.9
    if u == 0 or v == 0:
        put(c, px, py, "mortar", 0.2 + 4.0 * (w ** 0.7) * row_gain)
        return
    grad = 1.0 - v / vh
    value = 0.5 + row_gain * (1.8 + 8.0 * (w ** 0.7)) + jit + grad * 0.6 * row_gain - ao * 0.5
    if v < 1:
        value += 1.0 * row_gain
    put(c, px, py, "floor", value)


FILL_LEVELS = {  # fade toward the bottom of the view: (face colour options, joint colour)
    0: ([("stone", 0), ("brick", 0)], ("mortar", 0)),   # L* 6.5 / 4.8, b* -8 / -11
    1: ([("brick", 0)], ("mortar", 0)),                 # L* 4.8, b* -11
    2: ([("brick", 0), ("mortar", 1)], ("mortar", 0)),  # near black, still cool
}


def fill_level(row: int) -> int:
    """Fill tone by absolute layout row (phase-locked to the world, not to the floor top)."""
    return 0 if row <= 21 else 1 if row == 22 else 2


def fill_tile_row(level: int, joint: int, width_tiles: int, variant: str) -> "ld.Canvas":
    """One course of 32x16 dark ledge blocks. ``joint`` (0 or 16) is the local x of the
    first vertical joint; the layout picks it so joints sit on the world running bond."""
    w = width_tiles * TILE
    c = canvas(w, TILE)
    faces, mortar = FILL_LEVELS[level]
    for py in range(TILE):
        for px in range(w):
            u = (px - joint) % SLAB_W
            col = (px - joint) // SLAB_W
            start = col * SLAB_W + joint
            straddler = start < 0 or start + SLAB_W > w
            if u == 0 or py == 0:
                c.px[py][px] = mortar
                continue
            pick = 0 if straddler else int(ld.rnd("fill", level, joint, variant, col) * len(faces))
            cell = faces[pick]
            if level == 0 and py == 1:
                cell = ("stone", 0)  # faint top arris
            c.px[py][px] = cell
    return c


def floor_column(width_tiles: int, tone: str, seed: str, ao_fn=None) -> "ld.Canvas":
    """A panel of the floor stack rows deck_back, deck_front, face_a, face_b (fill is separate)."""
    w = width_tiles * TILE
    c = canvas(w, TILE * (len(ROW_TYPES) - 1))
    tone_w = DECK_TONES[tone]

    def straddle(start, size):
        return start < 0 or start + size > w

    for py in range(c.h):
        for px in range(w):
            ao = ao_fn(px, py) if ao_fn else 0.0
            deck_pixel(c, px, py, px, py, tone_w, seed, straddle, ao)
    return c


def edge_column(side: str, kind: str) -> "ld.Canvas":
    """Single floor-stack tiles: platform ends ("end"), lower floor next to a step
    ("inner") and floor next to a corridor end wall ("wall"). Patterns use the neutral
    tone so they join the edge column of any panel."""
    def straddle(start, size):
        return True

    def ao_fn(px, py):
        if kind == "end":
            return 0.0
        d = (15 - px) if side == "r" else px  # distance from the step / wall side
        if py >= TILE * 2 and kind == "inner":
            return 0.0
        return max(0.0, 2.4 - 0.55 * d)

    c = canvas(TILE, TILE * len(ROW_TYPES))
    for py in range(c.h):
        for px in range(TILE):
            if py >= FACE_COURSES[2]:
                continue  # fill rows are continuous world-phase panels, no edge singles
            deck_pixel(c, px, py, px, py, DECK_NEUTRAL, f"edge_{kind}_{side}", straddle,
                       ao_fn(px, py))
    if kind == "end":  # lit corner on the exposed end, dark seam on the outside pixel
        ex, ix = (0, 1) if side == "l" else (15, 14)
        for py in range(c.h):
            cur = c.get(ex, py)
            if cur is None:
                continue
            ramp = cur[0] if cur[0] != "mortar" else "floor"
            if py < LIP[1]:
                c.put(ex, py, "floor", 8.4 if py % 16 < 15 else 7.0)
                c.put(ix, py, "floor", 6.4)
            else:
                c.put(ex, py, "floor", 6.0 - min(3.0, (py - LIP[1]) / 18.0))
    return c


def ceiling_fill(variant: str, width_tiles: int) -> "ld.Canvas":
    w = width_tiles * TILE
    c = canvas(w, TILE)
    for py in range(TILE):
        course = py // 8
        stagger = 8 if course % 2 else 0
        for px in range(w):
            col = (px + stagger) // 16
            u, dv = (px + stagger) % 16, py % 8
            start = col * 16 - stagger
            straddler = start < 0 or start + 16 > w
            jit = 0.0 if straddler else (ld.rnd("ceil", variant, course, col) - 0.5) * 0.8
            if u == 0 or dv == 0:
                put(c, px, py, "stone_cold", 0.0)
            else:
                put(c, px, py, "stone_cold", 1.0 + jit + (0.4 if dv == 1 else 0.0))
    return c


def cornice(variant: str, width_tiles: int, end: str | None = None) -> "ld.Canvas":
    """Ceiling edge: vault blocks above a protruding stone cornice (seen from below)."""
    w = width_tiles * TILE
    c = canvas(w, TILE)
    top = ceiling_fill(variant, width_tiles)
    for py in range(TILE):
        for px in range(w):
            if py < 4:
                c.px[py][px] = top.px[py][px]
                continue
            if py == 4:
                put(c, px, py, "mortar", 0.4)  # shadow line where the cornice meets the vault
                continue
            seg = px // 16
            straddler = False
            jit = (ld.rnd("cornice", variant, seg) - 0.5) * 0.8 if not straddler else 0.0
            if px % 16 == 0 and 6 <= py < 13:
                put(c, px, py, "mortar", 1.4)
                continue
            if py <= 5:
                value = 6.4 + jit          # lit top fillet
            elif py < 13:
                value = 4.0 + jit - (py - 6) * 0.18
            else:
                value = 2.6 + (py - 13) * 0.7 + jit * 0.5  # underside warmed by torches below
            put(c, px, py, "stone", value)
    if end:
        ex = 0 if end == "l" else w - 1
        for py in range(4, TILE):
            put(c, ex, py, "stone", 7.0 if py < 13 else 4.0)
            put(c, ex + (1 if end == "l" else -1), py, "stone", 3.0)
    return c


def wall_end(side: str, phase: int) -> "ld.Canvas":
    """Corridor end wall face: dark dressed-stone mass with lit quoins on the exposed side."""
    c = canvas(TILE, TILE)
    for py in range(TILE):
        course = (py // 8 + phase) % 2
        for px in range(TILE):
            lx = px if side == "r" else 15 - px      # 0 = exposed edge
            long_q = course == 0
            q_w = 12 if long_q else 7
            dv = py % 8
            if dv == 0:
                put(c, px, py, "mortar", 1.0)
                continue
            if lx < q_w:
                if lx == q_w - 1:
                    put(c, px, py, "mortar", 1.0)
                    continue
                n = (0.0, -0.7, 0.7) if dv == 1 else ((-0.6 if side == "r" else 0.6), 0.0, 0.8) \
                    if lx == 0 else (0.0, 0.0, 1.0)
                value = 3.8 + 1.8 * key_term(n) + (1.4 if lx == 0 else 0.0) + (0.6 if dv == 1 else 0)
                put(c, px, py, "stone", value)
            else:
                put(c, px, py, "stone_cold", 0.6 + (0.4 if dv == 1 else 0.0))
    return c


def wall_fill(phase: int = 0) -> "ld.Canvas":
    c = canvas(TILE, TILE)
    for py in range(TILE):
        for px in range(TILE):
            dv = py % 8
            u = (px + (8 if (py // 8 + phase) % 2 else 0)) % 16
            put(c, px, py, "stone_cold", 0.0 if (dv == 0 or u == 0) else 0.6)
    return c


def ceiling_corner(side: str, kind: str) -> "ld.Canvas":
    """in: cornice runs into an end wall on ``side``; out: cornice ends (return face)."""
    if kind == "out":
        return cornice("a", 1, end=side)
    c = cornice("a", 1)
    wall = wall_end("r" if side == "l" else "l", 0)
    for py in range(TILE):
        for lx in range(5):
            px = lx if side == "l" else 15 - lx
            c.px[py][px] = wall.px[py][px] if lx < 4 else ("mortar", 0)
    return c


# ---------------------------------------------------------------------------
# Structure: A1 arch bay modules, threshold, plain pilaster
# ---------------------------------------------------------------------------
ARCH_RIG_BOUNCE = {"x": 336.0, "y": 196.0, "z": -70.0, "r": 170.0, "i": 1.0}


def arch_canvas(x0: float, y0: float, w: int, h: int) -> "ld.Canvas":
    return canvas(w, h, ox=x0, oy=y0)


def render_arch_piece(kind: str, variant: str = "a") -> "ld.Canvas":
    with light_rig(ARCH_PIER_TORCHES, ARCH_RIG_BOUNCE, STRING_Y=(2, 12)):
        if kind == "pier_l":
            c = arch_canvas(184, 12, 72, 272)
            ld.pillar(c, 0)
        elif kind == "pier_r":
            c = arch_canvas(408, 12, 80, 272)
            ld.pillar(c, 1)
        elif kind == "ring":
            orig_rnd = ld.rnd
            if variant != "a":
                ld.rnd = lambda *key: orig_rnd(variant, *key)  # noqa: E731
            try:
                c = arch_canvas(200, 12, 272, 144)
                ld.arch_ring(c)
            finally:
                ld.rnd = orig_rnd
        elif kind == "threshold":
            c = arch_canvas(248, 252, 176, 16)
            threshold(c)
        elif kind == "chain":
            c = arch_canvas(264, 92, 144, 48)
            ld.chain(c)
        else:
            raise ValueError(kind)
    if kind in ("pier_l", "pier_r"):
        fade_outer_edge(c, from_left=kind == "pier_l")
    return c


def fade_outer_edge(c, from_left: bool, steps=(2.2, 1.4, 0.7)) -> None:
    """Baked pier light falls off over the last 3 px of the outer face instead of stopping
    at a hard edge against an unlit wall."""
    for py in range(c.h):
        xs = [px for px in range(c.w) if c.px[py][px] is not None]
        if not xs:
            continue
        order = xs if from_left else xs[::-1]
        for k, px in enumerate(order[:len(steps)]):
            ramp, idx = c.px[py][px]
            if ramp in ("stone", "mortar"):
                c.put(px, py, ramp, max(1.0, idx - steps[k]) if idx > 2 else idx)


def threshold(c) -> None:
    """Floor receding into the gallery between the jambs (A1 look-dev terrain)."""
    for py in c.pyr(252, ld.FLOOR_BACK):
        y = c.dy(py)
        t = (ld.FLOOR_BACK - y) / 16.0
        seam = (round((ld.FLOOR_BACK - y) * c.s) % c.unit(5)) == 0
        for px in c.pxr(250, 422):
            x = c.dx(px)
            w = ld.warm_light(x, ld.FLOOR_BACK, -(ld.FLOOR_BACK - y) * 4.0, (0, -1, 0), wrap=0.1) * (1 - t)
            if w > 0.2:
                c.put(px, py, "floor", 0.8 + 7.0 * w - (0.8 if seam else 0))
            else:
                c.put(px, py, "floor_cold", 0.6 + t * 2.2 - (0.8 if seam else 0))


def pilaster() -> "ld.Canvas":
    """Plain dressed-stone pilaster for long A runs (dim, no torch baked in)."""
    w, h = 32, 256
    c = canvas(w, h)
    for py in range(h):
        for px in range(w):
            if py < 10 or py >= h - 14:  # capital and base: wider blocks, full width
                band_top = py < 10
                v = py if band_top else py - (h - 14)
                if (band_top and v == 9) or (not band_top and v == 0):
                    put(c, px, py, "mortar", 1.2)
                    continue
                n = (0.0, -0.7, 0.7) if (band_top and v < 2) or (not band_top and v < 3) else (0.0, 0.0, 1.0)
                edge = px in (0, w - 1)
                put(c, px, py, "stone", 3.3 + 1.8 * key_term(n) + (0.8 if v < 2 else 0.0)
                    - (0.8 if edge else 0.0))
                continue
            if px < 3 or px >= w - 3:
                continue  # shaft is narrower than capital and base
            sx = px - 3
            sw = w - 6
            course = (py - 10) // 16
            dv = (py - 10) % 16
            joints = [0, sw // 2] if course % 2 == 0 else [0]
            if dv == 0 or sx in joints:
                put(c, px, py, "mortar", 1.0)
                continue
            start = max(j for j in joints if j <= sx)
            end = min([j for j in joints if j > sx] + [sw])
            n, edge = ld.bevel_normal(sx - start - 1, dv - 1, end - start - 1, 15, 1)
            jit = (ld.rnd("pilaster", course, start) - 0.5) * 0.9
            put(c, px, py, "stone", 3.0 + jit + 1.8 * key_term(n) - 0.004 * py)
    return c


# ---------------------------------------------------------------------------
# Parallax gallery strips (A1 look-dev gallery, made horizontally periodic)
# ---------------------------------------------------------------------------
FAR_W, MID_W, GALLERY_Y0, GALLERY_H = 440, 344, 40, 232
FAR_TORCH_PIERS = (1, 2, 4, 6, 7, 8, 9)
MID_TORCH_X = 107.0


def parallax_far() -> "ld.Canvas":
    c = canvas(FAR_W, GALLERY_H, ox=0.0, oy=GALLERY_Y0)
    xs = tuple(6.0 + 44.0 * k for k in FAR_TORCH_PIERS)
    with light_rig([], FAR_TORCH_XS=xs):
        for py in range(c.h):
            y = c.dy(py)
            for px in range(c.w):
                c.put(px, py, "gal", ld.far_gallery_value(c, c.dx(px), y))
        for tx in xs:
            ld.far_torch(c, tx, ld.FAR_TORCH_Y, small=True)
    return c


def parallax_mid() -> "ld.Canvas":
    c = canvas(MID_W, GALLERY_H, ox=0.0, oy=GALLERY_Y0)
    pillars = [(12.0 + 86.0 * k, 30.0 + 86.0 * k) for k in range(4)]
    bays = [(-56.0, 12.0)] + [(30.0 + 86.0 * k, 98.0 + 86.0 * k) for k in range(4)]
    torch = {"x": MID_TORCH_X, "y": 206.0}
    with light_rig([], MID_TORCH=torch):
        for py in range(c.h):
            y = c.dy(py)
            for px in range(c.w):
                v = ld.mid_gallery_value(c, c.dx(px), y, pillars, bays)
                if v is not None:
                    c.put(px, py, v[0], v[1])
        ld.far_torch(c, torch["x"], torch["y"], small=False)
        ld.sconce(c, torch["x"], torch["y"] + 3, scale=0.55, ramp_override="gal")
    return c


# ---------------------------------------------------------------------------
# Torch (animated), unlit sconce
# ---------------------------------------------------------------------------
TORCH_W, TORCH_H = 16, 56
TORCH_FRAMES = [("f0", 1.30, 120), ("f1", 1.24, 140), ("f2", 1.36, 100), ("f3", 1.27, 130)]
TORCH_FLICKER = [1.0, 0.93, 1.07, 0.97]


def tapered_flame(c, x: float, y: float, scale: float, seed: str) -> None:
    """The A1 teardrop flame with a tapered (not rectangular) white-hot core."""
    h = 15.0 * scale
    base = y + 6.0
    for py in c.pyr(base - h - 1, base + 1):
        yy = c.dy(py)
        t = (base - yy) / h
        if not 0.0 <= t <= 1.0:
            continue
        lean = math.sin(t * 2.6 + ld.rnd(seed) * 6.0) * 1.2 * t
        half = 4.4 * scale * (math.sin(math.pi * min(1.0, t * 1.15 + 0.12)) ** 0.8) * (1.0 - 0.55 * t)
        for px in c.pxr(x - 7 * scale, x + 7 * scale):
            a = abs(c.dx(px) - (x + lean))
            if a > half:
                continue
            core = a / max(half, 0.5)
            if t < 0.42 and core < 0.5 * (1.0 - t / 0.42) + 0.08:
                val = 5
            elif core < 0.55 and t < 0.7:
                val = 4
            elif core < 0.8:
                val = 3
            else:
                val = 2 if t < 0.75 else 1
            if t > 0.86:
                val = min(val, 2)
            c.put(px, py, "flame", val)


def torch_rig() -> list[dict]:
    fx, fy = TORCH_FLAME_LOCAL
    return [{"x": fx, "y": fy, "z": 16.0, "p": 1.0}]


def torch_frame(scale: float, seed: str, lit: bool = True) -> tuple["ld.Canvas", "ld.Canvas"]:
    fx, fy = TORCH_FLAME_LOCAL
    sconce = canvas(TORCH_W, TORCH_H)
    flame = canvas(TORCH_W, TORCH_H)
    with light_rig(torch_rig() if lit else []):
        ld.sconce(sconce, float(fx), fy + 7.0, scale=1.15)
        if lit:
            tapered_flame(flame, float(fx), fy + 1.0, scale=scale, seed=seed)
        else:  # charred stub in a cold cup
            for px in range(fx - 3, fx + 3):
                sconce.put(px, fy + 6, "iron", 1.0)
                sconce.put(px, fy + 7, "iron", 0.0)
    return sconce, flame


# ---------------------------------------------------------------------------
# Copper/brass pipe set (directional key so straight pieces tile seamlessly)
# ---------------------------------------------------------------------------
PIPE_R = 7.0
# Torch side convention: +x for vertical pieces, +y (below) for horizontal pieces; the
# elbow normals blend both, so every piece of a run shares one key.
PIPE_KEY = ld.norm(0.62, 0.42, 0.66)   # warm torchlight from the right / below, in front


def metal_value(n: tuple, ramp: str, base: float, gain: float, gloss: float) -> float:
    lam = max(0.0, n[0] * PIPE_KEY[0] + n[1] * PIPE_KEY[1] + n[2] * PIPE_KEY[2])
    hv = ld.norm(PIPE_KEY[0], PIPE_KEY[1], PIPE_KEY[2] + 1.0)
    spec = max(0.0, n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2]) ** 24
    return base + gain * (lam ** 0.75) + gloss * 2.6 * spec


def pipe_px(c, px, py, u: float, axis: str, ramp: str = "copper", base=0.2, gain=5.1,
            gloss=1.0, darken=0.0) -> None:
    u = ld.clamp(u, -0.999, 0.999)
    a = math.asin(u)
    n = (math.sin(a), 0.0, math.cos(a)) if axis == "v" else (0.0, math.sin(a), math.cos(a))
    value = metal_value(n, ramp, base, gain, gloss) - darken
    rim(c, px, py, n, u, ramp, value)


def rim(c, px, py, n: tuple, u: float, ramp: str, value: float) -> None:
    """Outermost pixel of a cylinder: hot rim on the torch side, cold rim on the far side."""
    if abs(u) > 0.86:
        toward = n[0] * PIPE_KEY[0] + n[1] * PIPE_KEY[1]
        if toward > 0:
            c.put(px, py, ramp, max(value, 6.2 if ramp == "copper" else 5.2))
            return
        if ramp == "copper":
            c.put(px, py, "copper_cold", 0.6)
            return
        value -= 1.0
    c.put(px, py, ramp, value)


def pipe_straight(axis: str, seam: bool = False) -> "ld.Canvas":
    c = canvas(16, 16)
    for py in range(16):
        for px in range(16):
            along, across = (py, px) if axis == "v" else (px, py)
            u = (across + 0.5 - 8.0) / PIPE_R
            if abs(u) >= 1.0:
                continue
            pipe_px(c, px, py, u, axis, darken=(1.6 if seam and along in (7, 8) else 0.0))
    return c


def pipe_elbow() -> "ld.Canvas":
    """Elbow with arms to the right (centre y 8) and down (centre x 8); bend centre (24, 24)."""
    c = canvas(32, 32)
    bcx, bcy, rc = 24.0, 24.0, 16.0
    for py in range(32):
        for px in range(32):
            x, y = px + 0.5, py + 0.5
            if x >= bcx and y < 16:  # straight horizontal arm
                u = (y - 8.0) / PIPE_R
                if abs(u) < 1.0:
                    pipe_px(c, px, py, u, "h")
                continue
            if y >= bcy and x < 16:  # straight vertical arm
                u = (x - 8.0) / PIPE_R
                if abs(u) < 1.0:
                    pipe_px(c, px, py, u, "v")
                continue
            dx, dy = x - bcx, y - bcy
            if dx > 0 or dy > 0:
                continue
            d = math.hypot(dx, dy)
            u = (rc - d) / PIPE_R  # +1 at the outer edge (toward the corner)
            if abs(u) >= 1.0:
                continue
            uu = ld.clamp(u, -0.999, 0.999)
            a = math.asin(uu)
            rx, ry = -dx / max(d, 1e-6), -dy / max(d, 1e-6)  # outward (toward the corner)
            n = (rx * math.sin(a), ry * math.sin(a), math.cos(a))
            value = metal_value(n, "copper", 0.2, 5.1, 1.0)
            rim(c, px, py, n, u, "copper", value)
    return c


def mirrored(src: "ld.Canvas", mx: bool, my: bool) -> "ld.Canvas":
    out = canvas(src.w, src.h)
    blit(out, src, 0, 0, mirror_x=mx, mirror_y=my)
    return out


def brass_ring(axis: str, half: float, thick: float, w: int, h: int, bolts: bool = True,
               ramp: str = "brass") -> "ld.Canvas":
    """Flange/collar ring around a pipe: ``axis`` is the pipe direction."""
    c = canvas(w, h)
    cx, cy = w / 2, h / 2
    for py in range(h):
        for px in range(w):
            x, y = px + 0.5, py + 0.5
            if axis == "v":
                u, along = (x - cx) / half, (y - cy) / (thick / 2)
            else:
                u, along = (y - cy) / half, (x - cx) / (thick / 2)
            if abs(u) >= 1.0 or abs(along) >= 1.0:
                continue
            pipe_px(c, px, py, u, axis, ramp=ramp, base=1.0, gain=4.8, gloss=1.2,
                    darken=(1.0 if abs(along) > 0.62 else 0.0))
            if bolts and abs(along) < 0.4:
                for bu in (-0.6, 0.6):
                    if abs(u - bu) * half < 0.75:
                        c.put(px, py, "iron", 2.0)
    return c


def bracket(axis: str) -> "ld.Canvas":
    """Iron wall clamp around a pipe (``axis`` = pipe direction), with two wall lugs."""
    if axis == "v":
        c = canvas(32, 8)
        for py in range(1, 6):
            for px in range(2, 30):
                lug = px < 7 or px >= 25
                if lug:
                    bolt = px in (4, 27) and py == 3
                    c.put(px, py, "iron", 1.0 if bolt else (3.2 if py == 1 else 2.0))
                else:
                    u = (px + 0.5 - 16) / 9.0
                    pipe_px(c, px, py, u, "v", ramp="iron", base=1.2, gain=4.0, gloss=0.6,
                            darken=(0.8 if py == 5 else 0.0))
        return c
    return mirrored_transpose(bracket("v"))


def mirrored_transpose(src: "ld.Canvas") -> "ld.Canvas":
    out = canvas(src.h, src.w)
    for y in range(src.h):
        for x in range(src.w):
            out.px[x][y] = src.px[y][x]
    return out


def valve_wheel() -> "ld.Canvas":
    c = canvas(32, 32)
    with light_rig([{"x": 34.0, "y": -6.0, "z": 30.0, "p": 0.6}]):
        ld.valve(c, 16.0, 16.0, 14.0)
    return c


def collar_plate(kind: str) -> "ld.Canvas":
    """Brass plate where a vertical pipe enters the ceiling (top) or the floor (bottom)."""
    c = canvas(32, 8)
    for py in range(8):
        for px in range(32):
            x = px + 0.5
            if kind == "ceiling":
                half = 12.0 if py < 3 else 10.0
                if py > 5:
                    continue
            else:
                half = 10.0 if py < 4 else 13.0
                if py < 2:
                    continue
            u = (x - 16.0) / half
            if abs(u) >= 1.0:
                continue
            pipe_px(c, px, py, u, "v", ramp="brass", base=0.8, gain=4.6, gloss=1.2,
                    darken=(1.2 if (kind == "ceiling" and py == 5) or (kind == "floor" and py == 7) else 0))
            if abs(abs(u) - 0.78) * half < 0.7 and py in ((1, 2) if kind == "ceiling" else (5, 6)):
                c.put(px, py, "iron", 2.0)
    return c


# ---------------------------------------------------------------------------
# Door, lichen, rubble
# ---------------------------------------------------------------------------
DOOR_W, DOOR_H = 48, 88


def door() -> "ld.Canvas":
    """Plain round-headed timber door in a dressed-stone frame (46x88 opening for 52 px actors)."""
    c = canvas(DOOR_W, DOOR_H)
    cx, spring = DOOR_W / 2, 26.0
    r_frame, r_leaf = 24.0, 18.0
    for py in range(DOOR_H):
        for px in range(DOOR_W):
            x, y = px + 0.5, py + 0.5
            d = math.hypot(x - cx, y - spring)
            inside_frame = (y >= spring and abs(x - cx) < r_frame) or (y < spring and d < r_frame)
            inside_leaf = (y >= spring and abs(x - cx) < r_leaf) or (y < spring and d < r_leaf)
            if not inside_frame:
                continue
            if not inside_leaf:
                # voussoir-like frame stones
                if y < spring:
                    k = int(((math.atan2(spring - y, x - cx)) / math.pi) * 7)
                    joint = abs(((math.atan2(spring - y, x - cx)) / math.pi) * 7 - round(
                        (math.atan2(spring - y, x - cx)) / math.pi * 7)) * d * 0.45 < 0.5
                    seed = ("doorframe", k)
                else:
                    k = int((y - spring) // 14)
                    joint = (y - spring) % 14 < 1
                    seed = ("doorjamb", k, x < cx)
                if joint:
                    put(c, px, py, "mortar", 1.6)
                    continue
                edge_out = d > r_frame - 1.2 if y < spring else abs(x - cx) > r_frame - 1.2
                edge_in = d < r_leaf + 1.2 if y < spring else abs(x - cx) < r_leaf + 1.2
                value = 4.0 + (ld.rnd(seed) - 0.5) * 0.9 + (1.2 if edge_out and x < cx else 0) \
                    + (0.8 if x < cx - 6 else 0) \
                    - (1.0 if edge_in else 0) - (0.8 if edge_out and x >= cx else 0)
                if py >= DOOR_H - 2:
                    value = 2.4
                put(c, px, py, "stone", value)
                continue
            # leaf: vertical planks, two iron straps, ring pull, dark gap at the bottom
            lx = x - (cx - r_leaf)
            plank = int(lx // 6)
            pu = lx % 6
            if py >= DOOR_H - 2:
                put(c, px, py, "wood", 0)
                continue
            if 34 <= py <= 36 or 66 <= py <= 68:
                rivet = (int(lx) % 9 == 4) and py in (35, 67)
                put(c, px, py, "iron", 4.0 if rivet else (3.0 if py in (34, 66) else 1.6))
                continue
            ring_d = math.hypot(x - (cx + 10.5), y - 52.0)
            if 1.6 < ring_d < 3.0:
                put(c, px, py, "iron", 4.0 if y < 52 else 2.2)
                continue
            if pu < 1.0:
                put(c, px, py, "wood", 0.4)
                continue
            grain = ld.rnd("plank", plank, int(py // 7)) < 0.18 and pu > 3
            value = 3.0 + (ld.rnd("plank", plank) - 0.5) * 0.9 + (0.6 if pu < 2 else 0) \
                - (0.8 if grain else 0) - 0.012 * py
            put(c, px, py, "wood", value)
    return c


def lichen() -> "ld.Canvas":
    """A dry, muted olive crust spreading from a pipe joint (soft irregular blob)."""
    c = canvas(24, 8)
    for py in range(8):
        for px in range(24):
            dx, dy = (px + 0.5 - 12.0) / 11.5, (py + 0.5 - 5.0) / 3.6
            d = dx * dx + dy * dy + (ld.rnd("lichen", px // 2, py // 2) - 0.5) * 0.6
            if d >= 1.0:
                continue
            spot = ld.rnd("lichen_s", px, py)
            idx = 1 if (d < 0.5 and spot > 0.72) else 0
            c.put(px, py, "lichen", idx)
    return c


RUBBLE = {"rubble_a": (64, 32, [(16.0, 16.0, 13.0, "r1"), (44.0, 18.0, 11.0, "r2")]),
          "rubble_b": (48, 24, [(22.0, 22.0, 16.0, "r3"), (40.0, 8.0, 7.0, "r4")])}


def rubble(name: str) -> "ld.Canvas":
    """Low dark rubble silhouettes for the foreground corners (A1 look-dev style)."""
    w, h, mounds = RUBBLE[name]
    c = canvas(w, h)
    for (cx, half, height, seed) in mounds:
        for py in range(h):
            y = py + 0.5
            for px in range(w):
                x = px + 0.5
                t = (x - cx) / half
                if abs(t) >= 1.0:
                    continue
                facets = 3 + int(ld.rnd(seed) * 3)
                fi = int((t + 1) / 2 * facets)
                bump = 0.75 + 0.25 * ld.rnd(seed, fi)
                top_y = h - height * bump * math.sqrt(1 - t * t)
                if y < top_y:
                    continue
                edge = y < top_y + 1.5
                c.put(px, py, "brick", (4.6 + (0.8 if t < 0 else 0.0)) if edge else
                      (0.0 if (fi % 2 or y > top_y + 6) else 1.6 if y < top_y + 3 else 1.0))
    return c


# ---------------------------------------------------------------------------
# Atlas assembly
# ---------------------------------------------------------------------------

class Atlas:
    def __init__(self, name: str, kind: str):
        self.name = name
        self.kind = kind          # tileset | decor_atlas | prop_animation
        self.tiles: list[dict] = []
        self.pieces: list[dict] = []

    def add_tile(self, tid: str, img: "ld.Canvas", **meta) -> None:
        assert img.w == TILE and img.h == TILE, tid
        self.tiles.append({"id": tid, "img": img, **meta})

    def add_piece(self, pid: str, img: "ld.Canvas", **meta) -> None:
        self.pieces.append({"id": pid, "img": img, **meta})


def split_panel(atlas: Atlas, prefix: str, img: "ld.Canvas", family: str, panel_id: str,
                row_y: int = 0, **meta) -> None:
    cols = img.w // TILE
    for i in range(cols):
        tile = crop(img, i * TILE, row_y, TILE, TILE)
        tid = f"{prefix}_{i + 1}" if cols > 1 else prefix
        atlas.add_tile(tid, tile, panel={"id": panel_id, "family": family, "cell": [i, 0],
                                         "size": [cols, 1]}, **meta)


def pack_tiles(atlas: Atlas, cols: int) -> tuple["ld.Canvas", list[dict]]:
    n = len(atlas.tiles)
    rows = (n + cols - 1) // cols
    if n % cols:
        raise SystemExit(f"{atlas.name}: {n} tiles do not fill {cols}-column rows; add variants")
    img = canvas(cols * TILE, rows * TILE)
    out = []
    for i, t in enumerate(atlas.tiles):
        x, y = (i % cols) * TILE, (i // cols) * TILE
        blit(img, t["img"], x, y)
        entry = {k: v for k, v in t.items() if k != "img"}
        entry["grid_index"] = i
        entry["rect"] = [x, y, TILE, TILE]
        out.append(entry)
    return img, out


def pack_pieces(atlas: Atlas, width: int) -> tuple["ld.Canvas", list[dict]]:
    """Shelf packing in declaration order; pieces never overlap."""
    x = y = shelf = 0
    placed = []
    for p in atlas.pieces:
        w, h = p["img"].w, p["img"].h
        if x + w > width:
            x, y, shelf = 0, y + shelf, 0
        placed.append((p, x, y))
        x += w
        shelf = max(shelf, h)
    height = y + shelf
    img = canvas(width, height)
    out = []
    for p, px, py in placed:
        blit(img, p["img"], px, py)
        entry = {k: v for k, v in p.items() if k != "img"}
        entry["rect"] = [px, py, p["img"].w, p["img"].h]
        out.append(entry)
    return img, out


def grid_from(images: list["ld.Canvas"], layer_names: list[str] | None = None,
              durations: list[int] | None = None, tags=None, pivot=(0, 0),
              frames_layers: list[dict] | None = None) -> dict:
    """pixel-grid-v1 with an atlas-local palette (subset of the kit palette)."""
    sources = frames_layers or [{"main": img} for img in images]
    used = set()
    for fr in sources:
        for img in fr.values():
            for row in img.px:
                used.update(p for p in row if p is not None)
    ordered = sorted(used, key=lambda p: (RAMP_ORDER.index(p[0]), p[1]))
    if len(ordered) > MAX_ATLAS_COLOURS:
        raise SystemExit(f"{len(ordered)} colours exceed the per-atlas budget of {MAX_ATLAS_COLOURS}")
    keys = {p: GRID_KEYS[i] for i, p in enumerate(ordered)}
    palette = [{"key": keys[p], "name": colour_name(*p), "rgba": RAMPS[p[0]][p[1]] + "ff"}
               for p in ordered]
    first = next(iter(sources[0].values()))
    names = layer_names or list(sources[0].keys())
    frames = []
    for i, fr in enumerate(sources):
        cels = {}
        for name in names:
            img = fr[name]
            cels[name] = ["".join(keys[p] if p is not None else "." for p in row) for row in img.px]
        frames.append({"duration_ms": (durations or [100] * len(sources))[i], "cels": cels})
    return {"format": "pixel-grid-v1", "canvas": [first.w, first.h], "pivot": list(pivot),
            "palette": palette, "layers": names, "frames": frames, "tags": tags or []}


# ---------------------------------------------------------------------------
# Build every atlas
# ---------------------------------------------------------------------------

def build_terrain() -> Atlas:
    a = Atlas("floor01_terrain", "tileset")
    # Deck / face / fill panels from full floor stacks
    for tone in ("dim", "mid", "lit"):
        for variant, width in (("a", 4), ("b", 4), ("c", 2)):
            stack = floor_column(width, tone, f"deck_{tone}_{variant}")
            for ri, row_type in enumerate(("deck_back", "deck_front")):
                meta = {}
                if row_type == "deck_front":
                    meta = {"walkable_top": True, "walk_line_y": WALK_LINE_Y}
                else:
                    meta = {"deck_back": True}
                split_panel(a, f"{row_type}_{tone}_{variant}", crop(stack, 0, ri * TILE, stack.w, TILE),
                            row_type, f"{row_type}_{tone}_{variant}", **meta)
        for variant, width in (("a", 4), ("c", 2)):
            stack = floor_column(width, tone, f"face_{tone}_{variant}")
            split_panel(a, f"face_a_{tone}_{variant}", crop(stack, 0, 2 * TILE, stack.w, TILE),
                        "face_a", f"face_a_{tone}_{variant}")
    for variant, width in (("a", 4), ("b", 4), ("c", 2)):
        stack = floor_column(width, "mid", f"lower_{variant}")
        split_panel(a, f"face_b_{variant}", crop(stack, 0, 3 * TILE, stack.w, TILE), "face_b",
                    f"face_b_{variant}")
    # Fill: 32x16 dark blocks; level by absolute row, joint phase chosen by the layout
    for level in FILL_LEVELS:
        for joint in (0, 16):
            for variant, width in (("a", 4), ("c", 2)):
                pid = f"fill_{level}_j{joint}_{variant}"
                split_panel(a, pid, fill_tile_row(level, joint, width, variant),
                            f"fill_{level}_j{joint}", pid, fill_level=level)
        img = fill_tile_row(level, 0, 1, "wall")
        for py in range(TILE):  # occlusion beside the end wall
            for px in range(3):
                img.px[py][px] = ("mortar", 0)
        a.add_tile(f"fill_wall_l_{level}", img,
                   edge={"kind": "wall", "side": "l", "row_type": "fill"}, fill_level=level)
    # Single edge tiles for steps and the corridor end wall
    for kind in ("end", "inner", "wall"):
        for side in ("l", "r"):
            if kind == "wall" and side == "r":
                continue
            stack = edge_column(side, kind)
            for ri, row_type in enumerate(ROW_TYPES[:-1]):
                meta: dict = {"edge": {"kind": kind, "side": side, "row_type": row_type}}
                if row_type == "deck_front":
                    meta.update(walkable_top=True, walk_line_y=WALK_LINE_Y)
                if row_type == "deck_back":
                    meta["deck_back"] = True
                a.add_tile(f"{row_type}_{kind}_{side}", crop(stack, 0, ri * TILE, TILE, TILE), **meta)
    # Ceiling
    split_panel(a, "ceiling_fill_a", ceiling_fill("a", 4), "ceiling_fill", "ceiling_fill_a")
    split_panel(a, "ceiling_fill_s", ceiling_fill("s", 1), "ceiling_fill", "ceiling_fill_s")
    split_panel(a, "ceiling_fill_t", ceiling_fill("t", 1), "ceiling_fill", "ceiling_fill_t")
    split_panel(a, "cornice_a", cornice("a", 4), "cornice", "cornice_a")
    split_panel(a, "cornice_b", cornice("b", 4), "cornice", "cornice_b")
    split_panel(a, "cornice_s", cornice("s", 1), "cornice", "cornice_s")
    for side in ("l", "r"):
        a.add_tile(f"ceiling_corner_in_{side}", ceiling_corner(side, "in"))
        a.add_tile(f"ceiling_corner_out_{side}", ceiling_corner(side, "out"))
    # Corridor end walls
    for side in ("l", "r"):
        a.add_tile(f"wall_end_{side}", wall_end(side, 0))
        a.add_tile(f"wall_end_{side}_b", wall_end(side, 1))
    a.add_tile("wall_fill", wall_fill())
    a.add_tile("wall_fill_b", wall_fill(phase=1))
    extra = iter("uvwxyz")
    while len(a.tiles) % 16:  # fill the last atlas row with extra solo ceiling variants
        v = next(extra)
        split_panel(a, f"ceiling_fill_{v}", ceiling_fill(v, 1), "ceiling_fill", f"ceiling_fill_{v}")
    return a


def build_backwall() -> Atlas:
    a = Atlas("floor01_backwall", "tileset")
    for band in BANDS:
        for variant in ("a", "b", "c", "d"):
            split_panel(a, f"brick_{band}_{variant}", brick_panel(band, variant, 4),
                        f"brick_{band}", f"brick_{band}_{variant}", band=band)
        for solo in ("s", "t"):
            split_panel(a, f"brick_{band}_{solo}", brick_panel(band, solo, 1),
                        f"brick_{band}", f"brick_{band}_{solo}", band=band)
    for variant in ("a", "b"):
        split_panel(a, f"base_course_{variant}", base_course(variant, 4), "base_course",
                    f"base_course_{variant}")
    for solo in ("s", "t", "u", "v"):
        split_panel(a, f"base_course_{solo}", base_course(solo, 1), "base_course", f"base_course_{solo}")
    return a


def build_walldecal() -> Atlas:
    a = Atlas("floor01_walldecal", "decor_atlas")
    for variant in ("a", "b", "a_m", "b_m"):
        a.add_piece(f"torch_pool_{variant}", torch_pool(variant), role="backwall", layer="backwall_decal",
                    anchor=list(POOL_CENTRE_LOCAL), anchor_name="light_centre (= wall torch flame centre)",
                    grid_snap=16, grammar=["A", "C"], decal=True, fixed_y=POOL_TOP_Y,
                    pairs_with="torch_wall")
    for side, dx in (("l", -48), ("r", 288)):
        a.add_piece(f"pier_spill_{side}", pier_spill(side), role="backwall", layer="backwall_decal",
                    anchor=[0, 0], anchor_name=f"top_left = bay origin + ({dx}, 64); y = {SPILL_TOP_Y}",
                    grid_snap=16, grammar=["B"], decal=True, fixed_y=SPILL_TOP_Y,
                    pairs_with=f"pier_{side} (only where no torch_pool is adjacent)")
    for variant in ("a", "b"):
        a.add_piece(f"grime_{variant}", grime_patch(variant), role="backwall", layer="backwall_decal",
                    anchor=[0, 0], anchor_name="top_left (brick-aligned, mid band rows 8-12)",
                    grid_snap=16, grammar=["A"], decal=True, band="mid")
    for name in RUBBLE:
        img = rubble(name)
        a.add_piece(name, img, role="foreground", layer="foreground", anchor=[img.w // 2, img.h],
                    anchor_name="bottom_centre (stands on the bottom edge of the view)",
                    grid_snap=8, grammar=["A", "B", "C"])
    return a


def window_spec() -> dict:
    """Parallax window of an arch ring piece, in ring-local pixels (ring rect origin)."""
    ox, oy = 200, 12
    return {"arc_centre": [int(ld.ARCH_CX - ox), int(ld.ARCH_SPRING - oy)],
            "arc_radius": int(ld.R_IN),
            "rect": [int(ld.L_PILLAR[1] - ox), int(ld.ARCH_SPRING - oy),
                     int(672 - 2 * ld.L_PILLAR[1]), int(ld.FLOOR_BACK - ld.ARCH_SPRING)],
            "note": "Window = (circle above the arc centre line) U rect. Backwall tiles and "
                    "decals must leave it empty; the parallax gallery shows through."}


def build_structure() -> Atlas:
    a = Atlas("floor01_structure", "decor_atlas")
    bounce = {"id": "gallery_bounce", "offset": [136, 202], "colour": "#5b7cc4",
              "radius": 150, "intensity": 0.32, "group": "scene"}
    sky = {"id": "gallery_sky", "offset": [136, 98], "colour": "#6f92d0", "radius": 120,
           "intensity": 0.3, "group": "scene"}
    for variant in ("a", "b"):
        a.add_piece(f"arch_ring_{variant}", render_arch_piece("ring", variant), role="structure",
                    layer="structure", anchor=[0, 0],
                    anchor_name="top_left = bay origin + (16, 0); bay origin y = 32",
                    grid_snap=16, grammar=["B"], window=window_spec(), lights=[bounce, sky],
                    arch={"radius_inner": int(ld.R_IN), "radius_outer": int(ld.R_OUT),
                          "voussoirs": 15, "bay_offset": [16, 0]})
    a.add_piece("pier_l", render_arch_piece("pier_l"), role="structure", layer="structure",
                anchor=[0, 0], anchor_name="top_left = bay origin + (0, 0)", grid_snap=16,
                grammar=["B"], bay_offset=[0, 0], light_baked_from="torch_wall at bay origin + (24, 176)")
    a.add_piece("pier_r", render_arch_piece("pier_r"), role="structure", layer="structure",
                anchor=[0, 0], anchor_name="top_left = bay origin + (224, 0)", grid_snap=16,
                grammar=["B"], bay_offset=[224, 0], light_baked_from="torch_wall at bay origin + (264, 176)")
    a.add_piece("arch_threshold", render_arch_piece("threshold"), role="structure", layer="structure",
                anchor=[0, 0], anchor_name="top_left = bay origin + (64, 240)", grid_snap=16,
                grammar=["B"], bay_offset=[64, 240])
    a.add_piece("arch_chain", render_arch_piece("chain"), role="structure", layer="structure",
                anchor=[0, 0], anchor_name="top_left = bay origin + (80, 80); optional, some bays only",
                grid_snap=16, grammar=["B"], bay_offset=[80, 80])
    a.add_piece("pilaster_plain", pilaster(), role="structure", layer="structure", anchor=[16, 256],
                anchor_name="bottom_centre (stands on the floor back line, top at the wall top)",
                grid_snap=16, grammar=["A"])
    return a


def build_parallax() -> Atlas:
    a = Atlas("floor01_parallax", "decor_atlas")
    far_lights = []
    for i, k in enumerate(FAR_TORCH_PIERS):
        x = int(6 + 44 * k)
        far_lights.append({"id": f"far_torch_{i}", "offset": [x, int(ld.FAR_TORCH_Y - GALLERY_Y0)],
                           "colour": "#ff9a50", "radius": 30, "intensity": 0.5,
                           "glow": {"radius": 9, "additive": 0.22}, "group": "parallax_far"})
    a.add_piece("gallery_far", parallax_far(), role="parallax_far", layer="parallax_far",
                anchor=[0, 0], anchor_name="top_left (strip origin)", grid_snap=1, grammar=["B"],
                tile_x=True, lights=far_lights)
    a.add_piece("gallery_mid", parallax_mid(), role="parallax_mid", layer="parallax_mid",
                anchor=[0, 0], anchor_name="top_left (strip origin)", grid_snap=1, grammar=["B"],
                tile_x=True,
                lights=[{"id": "mid_torch", "offset": [int(MID_TORCH_X), int(206 - GALLERY_Y0)],
                         "colour": "#ff9a50", "radius": 70, "intensity": 0.7,
                         "glow": {"radius": 16, "additive": 0.2}, "group": "parallax_mid"}])
    return a


def build_props() -> Atlas:
    a = Atlas("floor01_props", "decor_atlas")
    snap = 16
    a.add_piece("pipe_h", pipe_straight("h"), anchor=[0, 8], anchor_name="left end, pipe centreline",
                grid_snap=snap)
    a.add_piece("pipe_h_seam", pipe_straight("h", seam=True), anchor=[0, 8],
                anchor_name="left end, pipe centreline", grid_snap=snap)
    a.add_piece("pipe_v", pipe_straight("v"), anchor=[8, 0], anchor_name="top end, pipe centreline",
                grid_snap=snap)
    a.add_piece("pipe_v_seam", pipe_straight("v", seam=True), anchor=[8, 0],
                anchor_name="top end, pipe centreline", grid_snap=snap)
    base = pipe_elbow()
    for name, mx, my in (("pipe_elbow_rd", False, False), ("pipe_elbow_ld", True, False),
                         ("pipe_elbow_ru", False, True), ("pipe_elbow_lu", True, True)):
        a.add_piece(name, mirrored(base, mx, my), anchor=[0, 0],
                    anchor_name="top_left; arms end on tile centrelines (8 or 24 px)", grid_snap=snap)
    a.add_piece("pipe_flange_v", brass_ring("v", 10.0, 6.0, 32, 8), anchor=[16, 4],
                anchor_name="centre on a vertical pipe (piece x = pipe tile x - 8)", grid_snap=8)
    a.add_piece("pipe_flange_h", brass_ring("h", 10.0, 6.0, 8, 32), anchor=[4, 16],
                anchor_name="centre on a horizontal pipe (piece y = pipe tile y - 8)", grid_snap=8)
    a.add_piece("pipe_bracket_v", bracket("v"), anchor=[16, 4],
                anchor_name="centre on a vertical pipe (piece x = pipe tile x - 8)", grid_snap=8)
    a.add_piece("pipe_bracket_h", bracket("h"), anchor=[4, 16],
                anchor_name="centre on a horizontal pipe (piece y = pipe tile y - 8)", grid_snap=8)
    a.add_piece("pipe_collar_ceiling", collar_plate("ceiling"), anchor=[16, 0],
                anchor_name="top_centre against the ceiling underside (y = 32)", grid_snap=8)
    a.add_piece("pipe_collar_floor", collar_plate("floor"), anchor=[16, 8],
                anchor_name="bottom_centre on the floor back line (y = 288)", grid_snap=8)
    a.add_piece("valve_wheel", valve_wheel(), anchor=[16, 16],
                anchor_name="centre on a vertical pipe (piece x = pipe tile x - 8)", grid_snap=8)
    a.add_piece("door_timber", door(), anchor=[DOOR_W // 2, DOOR_H],
                anchor_name="bottom_centre on the floor back line (y = 288)", grid_snap=8,
                grammar=["A"])
    sconce, _flame = torch_frame(1.3, "unlit", lit=False)
    a.add_piece("torch_wall_unlit", sconce, anchor=list(TORCH_FLAME_LOCAL),
                anchor_name="flame_centre (same mount as torch_wall)", grid_snap=8,
                grammar=["A", "B", "C"])
    a.add_piece("lichen_crust", lichen(), anchor=[0, 7], anchor_name="bottom_left", grid_snap=8,
                grammar=["C"])
    for p in a.pieces:
        p.setdefault("role", "prop")
        p.setdefault("layer", "prop")
        p.setdefault("grammar", ["C"])
    return a


def torch_lights() -> list[dict]:
    fx, fy = TORCH_FLAME_LOCAL
    return [
        {"id": "flame", "offset": [fx, fy], "colour": "#ffbd7a", "radius": 250, "intensity": 0.36,
         "glow": {"radius": 44, "additive": 0.08}, "flicker": TORCH_FLICKER, "group": "scene"},
        {"id": "floor_pool", "offset": [fx, fy], "project": "walk_line", "project_dy": -6,
         "squash_y": 0.32, "colour": "#ffb36b", "radius": 130, "intensity": 0.3,
         "flicker": TORCH_FLICKER, "group": "scene"},
    ]


def build_torch() -> tuple[Atlas, dict]:
    a = Atlas("floor01_torch", "prop_animation")
    frames = []
    for seed, scale, duration in TORCH_FRAMES:
        sconce, flame = torch_frame(scale, seed)
        frames.append(({"sconce": sconce, "flame": flame}, duration))
    grid = grid_from([], layer_names=["sconce", "flame"], durations=[d for _f, d in frames],
                     tags=[{"name": "burn", "from": 0, "to": len(frames) - 1, "direction": "forward"}],
                     pivot=TORCH_FLAME_LOCAL, frames_layers=[f for f, _d in frames])
    return a, grid


# ---------------------------------------------------------------------------
# Contract, manifests, layout
# ---------------------------------------------------------------------------
ATLAS_FILES = {
    "floor01_terrain": ("assets/source/terrain/floor01/terrain", "tileset"),
    "floor01_backwall": ("assets/source/terrain/floor01/backwall", "tileset"),
    "floor01_walldecal": ("assets/source/terrain/floor01/walldecal", "decor_atlas"),
    "floor01_structure": ("assets/source/terrain/floor01/structure", "decor_atlas"),
    "floor01_parallax": ("assets/source/terrain/floor01/parallax", "decor_atlas"),
    "floor01_props": ("assets/source/props/floor01/props", "decor_atlas"),
    "floor01_torch": ("assets/source/props/floor01/torch", "prop_animation"),
}
ATLAS_ORIGIN = {
    "floor01_terrain": "collision tiles: deep floor deck, face courses, fill, step/edge singles, ceiling and cornice, corridor end walls",
    "floor01_backwall": "brick backwall tiles in four height bands plus the pale base course",
    "floor01_walldecal": "torch pools, grime patches and foreground rubble",
    "floor01_structure": "arch rings, piers, threshold and plain pilaster (A1 look-dev arch geometry as modules)",
    "floor01_parallax": "far and mid gallery parallax strips (A1 look-dev gallery made horizontally periodic)",
    "floor01_props": "copper/brass pipe set, valve, collars, brackets, timber door, unlit sconce and lichen",
    "floor01_torch": "4-frame wall torch loop",
}


def atlas_ref(name: str) -> dict:
    stem, _kind = ATLAS_FILES[name]
    return {"grid": f"{stem}.grid.json", "manifest": f"assets/manifests/{name}.json",
            "image": f"generated/art/{name}.png", "metadata": f"generated/art/{name}.json"}


def manifest(name: str, grid: dict, tiles: list[str] | None = None, frame_size=None,
             pivot=(0, 0), tags=()) -> dict:
    stem, kind = ATLAS_FILES[name]
    m = {
        "schema_version": 1,
        "id": f"{name}_v2",
        "type": kind,
        "source_path": f"{stem}.ase",
        "grid_source": f"{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png",
        "output_metadata": f"generated/art/{name}.json",
        "palette_id": PALETTE_ID,
        "frame_size": list(frame_size or grid["canvas"]),
    }
    if kind == "tileset":
        m["tile_size"] = [TILE, TILE]
        m["tiles"] = tiles
    m.update({
        "pivot": list(pivot),
        "animation_tags": list(tags),
        "required_alpha": kind != "tileset",
        "kit_contract": CONTRACT_REL,
        "owner": "AI-authored environment pixel art (ENV-01 rework round 3, Claude Opus)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": f"Original pixel-grid-v1 {ATLAS_ORIGIN[name]} for the Floor 1 catacomb "
                      "environment kit (ENV-01 rework round 3, A1 target frame); not cropped, traced "
                      "or resampled from concept art",
            "generator": "Deterministic authoring script scripts/author_floor01_kit.py written by an "
                         "AI agent (Claude Opus), using the A1 look-dev shading core "
                         "scripts/lookdev_floor01_author.py (rule-based light evaluation quantised to "
                         "hand-picked ramps); no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "lookdev-floor01-resolution-spike/A1 (owner-approved target frame, 2026-11-16)",
                "floor-01-opening/01-catacomb-environment-clean (mood reference of the A1 frame only)",
            ],
        },
    })
    return m


def build_all() -> dict:
    terrain = build_terrain()
    backwall = build_backwall()
    walldecal = build_walldecal()
    structure = build_structure()
    parallax = build_parallax()
    props = build_props()
    _torch_atlas, torch_grid = build_torch()

    grids, manifests, tiles, pieces = {}, {}, [], []
    for atlas, cols in ((terrain, 16), (backwall, 17)):
        img, entries = pack_tiles(atlas, cols)
        grids[atlas.name] = grid_from([img])
        role = "collision" if atlas.name == "floor01_terrain" else "backwall"
        for e in entries:
            entry = {"id": e["id"], "atlas": atlas.name, "grid_index": e["grid_index"],
                     "rect": e["rect"], "role": role,
                     "collidable": role == "collision" and not e.get("deck_back", False),
                     "grammar": ["A", "B", "C"]}
            for key in ("deck_back", "walkable_top", "walk_line_y", "panel", "edge", "band",
                        "fill_level"):
                if key in e:
                    entry[key] = e[key]
            if entry["collidable"]:
                wl = e.get("walk_line_y", 0)
                entry["collider"] = [0, wl, TILE, TILE - wl]
            tiles.append(entry)
        manifests[atlas.name] = manifest(atlas.name, grids[atlas.name],
                                         tiles=[e["id"] for e in entries])
    for atlas, width in ((walldecal, 528), (structure, 576), (parallax, 448), (props, 256)):
        img, entries = pack_pieces(atlas, width)
        grids[atlas.name] = grid_from([img])
        for e in entries:
            entry = {"id": e["id"], "atlas": atlas.name, "rect": e["rect"], "role": e["role"],
                     "layer": e["layer"], "collidable": False, "anchor": e["anchor"],
                     "anchor_name": e["anchor_name"], "grid_snap": e["grid_snap"],
                     "grammar": e.get("grammar", ["A", "B", "C"])}
            for key in ("decal", "fixed_y", "pairs_with", "window", "lights", "arch", "bay_offset",
                        "light_baked_from", "tile_x", "band"):
                if key in e:
                    entry[key] = e[key]
            pieces.append(entry)
        manifests[atlas.name] = manifest(atlas.name, grids[atlas.name])
    grids["floor01_torch"] = torch_grid
    frames = []
    for i, (_seed, _scale, duration) in enumerate(TORCH_FRAMES):
        frames.append({"frame": {"x": TORCH_W * i, "y": 0, "w": TORCH_W, "h": TORCH_H},
                       "duration": duration})
    pieces.append({"id": "torch_wall", "atlas": "floor01_torch", "rect": [0, 0, TORCH_W, TORCH_H],
                   "role": "prop", "layer": "prop", "collidable": False,
                   "anchor": list(TORCH_FLAME_LOCAL), "anchor_name": "flame_centre",
                   "grid_snap": 8, "grammar": ["A", "B", "C"],
                   "mount_y": {"wall": WALL_TORCH_FLAME_Y - TORCH_FLAME_LOCAL[1],
                               "pier": PIER_TORCH_FLAME_Y - TORCH_FLAME_LOCAL[1]},
                   "lights": torch_lights(),
                   "animation": {"tag": {"name": "burn", "from": 0, "to": len(frames) - 1,
                                         "direction": "forward"}, "frames": frames}})
    manifests["floor01_torch"] = manifest("floor01_torch", torch_grid,
                                          frame_size=[TORCH_W, TORCH_H],
                                          pivot=TORCH_FLAME_LOCAL, tags=["burn"])
    contract = build_contract(tiles, pieces, grids)
    layout = build_layout(contract)
    return {"grids": grids, "manifests": manifests, "contract": contract, "layout": layout}


def build_contract(tiles: list[dict], pieces: list[dict], grids: dict) -> dict:
    per_atlas = {name: len(g["palette"]) for name, g in grids.items()}
    return {
        "format": "env-kit-v2",
        "kit_id": KIT_ID,
        "status": "proposed - rework round 3 against the owner-approved A1 target frame; awaiting human art review",
        "migration": {
            "from": "env-kit-v1",
            "changes": [
                "palette: one combined kit palette (named ramp colours); each atlas grid uses a subset of at most 63 opaque colours (was one shared <= 64 palette)",
                "layers: new parallax_far / parallax_mid roles (kind 'parallax', horizontally tiled strips with a parallax factor); collision now draws before structure and props; every layer names a light_group",
                "floor: the walking line replaces the tile top as the collision surface (walk_line_y per walkable tile, collider rect per collidable tile); deck_back tiles are non-collidable visual floor in the collision layer",
                "arch openings: backwall tiles are absent inside an arch window and the parallax gallery shows through (recess_* tiles and far_arcade_* pieces removed)",
                "lights: emitter metadata on pieces and a layout-level lighting profile (presentation only)",
            ],
            "removed_ids": "all v1 tile and piece ids (no v1 id keeps its meaning)",
        },
        "palette_id": PALETTE_ID,
        "palette": combined_palette(),
        "palette_policy": {"max_opaque_colours_per_atlas": MAX_ATLAS_COLOURS,
                           "colours_per_atlas": per_atlas,
                           "note": "Grid palette entries must match a kit palette entry by name and rgba; "
                                   "grid keys are atlas-local."},
        "tile_size": [TILE, TILE],
        "coordinate_system": {
            "origin": "top_left", "x_axis": "right", "y_axis": "down",
            "row_order": "top_to_bottom", "units": "virtual_px",
            "note": "Layout pixel (x, y) = (col * 16, row * 16). Atlas rects are [x, y, w, h] in "
                    "exported atlas pixels, top-left origin. Bevy (y-up) must flip y.",
        },
        "layers": [
            {"name": "parallax_far", "role": "parallax_far", "kind": "parallax", "draw_order": 0,
             "parallax": 0.35, "light_group": "parallax_far"},
            {"name": "parallax_mid", "role": "parallax_mid", "kind": "parallax", "draw_order": 1,
             "parallax": 0.6, "light_group": "parallax_mid"},
            {"name": "backwall", "role": "backwall", "kind": "tiles", "draw_order": 2, "light_group": "scene"},
            {"name": "backwall_decal", "role": "backwall", "kind": "pieces", "draw_order": 3,
             "light_group": "scene"},
            {"name": "collision", "role": "collision", "kind": "tiles", "draw_order": 4, "light_group": "scene"},
            {"name": "structure", "role": "structure", "kind": "pieces", "draw_order": 5, "light_group": "scene"},
            {"name": "prop", "role": "prop", "kind": "pieces", "draw_order": 6, "light_group": "scene"},
            {"name": "foreground", "role": "foreground", "kind": "pieces", "draw_order": 8,
             "light_group": "foreground"},
        ],
        "actor_draw_order": 7,
        "actor_light_group": "actors",
        "roles": {
            "parallax_far": "Farthest gallery strip seen through arch windows; tiles horizontally; moves at its layer's parallax factor.",
            "parallax_mid": "Nearer gallery strip (balcony, pillars, arches) with 1-bit alpha over parallax_far.",
            "collision": "Collision-layer terrain. Collidable tiles are opaque and carry a collider rect; deck_back tiles are opaque but non-collidable (visual floor behind the walking line).",
            "backwall": "Non-collidable wall behind actors; tiles opaque, decals 1-bit alpha; never inside an arch window.",
            "structure": "Non-collidable architecture (arch rings, piers, threshold, pilaster); 1-bit alpha.",
            "prop": "Non-collidable props (torches, pipes, valve, collars, door, lichen); 1-bit alpha.",
            "foreground": "Restrained foreground silhouettes drawn over actors (low rubble only).",
        },
        "walk_line": {
            "definition": "The collision surface of a floor column is its walking line, not the top of its first opaque tile. A walkable tile declares walk_line_y (0..15, tile-local); the collider top in layout pixels is row * 16 + walk_line_y. The deep floor shows a 24 px top surface: the deck_back tile (16 px, non-collidable) plus 8 px of the deck_front tile; the walking line sits 20 px behind... i.e. 4 px behind the front edge (lip) of the surface.",
            "collider_rule": "Every collidable tile has collider [x, y, w, h] (tile-local px). Bevy: for each collidable cell, spawn (or merge per row) a static AABB at (col * 16 + x, row * 16 + y, w, h), flipping y for y-up. A deck_front tile's collider is [0, 4, 16, 12]; full tiles use [0, 0, 16, 16].",
            "actor_contact": "Actors are drawn with their pivot row (bottom opaque row) at collider_top - 1: feet touch the walking line with 0 px sink. Spawn/exit markers use y = collider_top - 1 of their column.",
            "base_floor": {"floor_back_y": FLOOR_ROW * TILE, "walk_line_y": FLOOR_ROW * TILE + TILE + WALK_LINE_Y,
                           "front_edge_y": FLOOR_ROW * TILE + TILE + 8},
            "wall_columns": "A column that is collidable from the top row to the bottom row is a wall (corridor end), not floor.",
        },
        "lighting": {
            "presentation_only": True,
            "note": "Lights never change collision, gameplay or authoritative state. Pixel art carries baked light (A1); emitters drive the screen-resolution lighting pass (cold ambient multiply per light group, warm pools, additive glow, flame bloom, per-layer haze).",
            "emitter_schema": {
                "id": "unique within the piece",
                "offset": "[x, y] piece-local px of the light centre",
                "colour": "#rrggbb",
                "radius": "falloff radius in virtual px (windowed inverse-square-like)",
                "intensity": "multiplier added to the group's ambient light map",
                "glow": "optional {radius, additive}: additive halo",
                "flicker": "optional list of intensity multipliers, one per animation frame",
                "project": "optional 'walk_line': the light centre moves straight down to the column's walking line plus project_dy",
                "squash_y": "optional vertical squash of the falloff (flat floor pools)",
                "group": "light group the emitter lights (scene, parallax_far, parallax_mid); scene emitters also light actors at the actors group's light_scale",
            },
            "groups": ["parallax_far", "parallax_mid", "scene", "actors", "foreground"],
        },
        "atlases": {name: atlas_ref(name) for name in ATLAS_FILES},
        "tiles": tiles,
        "pieces": pieces,
        "usage_rules": {
            "rows": "Base corridor: rows 0-1 ceiling (ceiling_fill, cornice), rows 2-16 brick bands (top 2-3, upper 4-7, mid 8-12, lower 13-14, low 15-16: the lit band behind actor heads), row 17 base_course, row 18 deck_back, row 19 deck_front (walking line y = 308), row 20 face_a, row 21 face_b, rows 22+ fill. A raised platform shifts its own floor stack up by 1 or 2 rows.",
            "panels": "Within a row a panel cell is followed by its right neighbour; a panel's last column may be followed by column 0 of any panel of the same family (solo panels are one tile wide). Cut bricks and slabs at panel edges use a neutral tone, so panels join seamlessly.",
            "edges": "Floor-stack singles: *_wall_l right of a corridor end wall; *_inner_r / *_inner_l on the lower floor left / right of a platform; *_end_l / *_end_r as the first / last column of a platform. A single must touch panel edge columns (last column on its left, column 0 on its right).",
            "A": "Ordinary fabric: brick bands, base course, deck tones dim/mid/lit (lit under a torch, mid next to it), torch_wall + torch_pool_a/b at wall mount height, at least 240 px between pooled torches; occasional grime_*, door_timber, torch_wall_unlit or one pilaster_plain.",
            "B": "Arch bay, 304 px wide from a 16 px aligned bay origin (y = 32): pier_l (+0,0), arch_ring_a/b (+16,0), pier_r (+224,0), arch_threshold (+64,240), torch_wall on both piers (+24,176) and (+264,176). Backwall cells that touch the ring window stay empty; the parallax strips show through. Piers bake their own torch light; do not add pools at pier torches.",
            "C": "Special function, sparing: one copper run with elbows, at least one flange and bracket, a valve_wheel on a vertical drop; every end terminates in pipe_collar_ceiling / pipe_collar_floor; optional lichen_crust near a pipe joint only. Copper only in C.",
            "lights": "Torch pools are baked for wall torches only (torch_pool_*), centred on the flame; their decal is placed at (flame_x - 120, 48).",
            "grid_snap": "Piece top-left positions must be multiples of grid_snap; fixed_y pieces must use that y.",
        },
    }


SEGMENTS = [("A1", "A", 0, 32), ("B1", "B", 32, 51), ("C1", "C", 51, 80), ("A2", "A", 80, 100),
            ("B2", "B", 100, 128)]
BAYS = [32, 102]              # bay origin columns
WALL_TORCHES = [136, 440, 920]   # flame x (wall torches with pools); C1/A2 east of 1040 is an unlit stretch
POOL_PICK = ["torch_pool_b_m", "torch_pool_a", "torch_pool_a_m"]
UNLIT_SCONCES = [1416, 1984]      # flame x of burnt-out sconces
SPILLS = [(1, "l"), (1, "r")]     # (bay index, side): pier spills where no pool is adjacent
CHAIN_BAYS = [0]                  # optional chain on B1 only
PLATFORMS = [(7, 12, 1), (87, 94, 2)]        # (end_l col, end_r col, height in tiles)


def stable(*key) -> int:
    return int(ld.rnd("layout", *key) * 1_000_003)


def build_layout(contract: dict) -> dict:
    cols, rows = 128, 25
    collision: list[list[str | None]] = [[None] * cols for _ in range(rows)]
    backwall: list[list[str | None]] = [[None] * cols for _ in range(rows)]
    tiles = {t["id"]: t for t in contract["tiles"]}
    torches_flame = list(WALL_TORCHES) + [b * TILE + 32 for b in BAYS] + [b * TILE + 272 for b in BAYS]

    def tone_at(x0: int, x1: int) -> str:
        cx = (x0 + x1) / 2 * TILE
        d = min(abs(cx - f) for f in torches_flame)
        return "lit" if d < 60 else "mid" if d < 124 else "dim"

    def fill_run(row: int, c0: int, c1: int, family_prefix: str, variants: list[str],
                 grid=collision, tone: bool = False, salt: str = "") -> None:
        """Fill columns [c0, c1) of a row with panels (4-wide, then 2- or 1-wide to finish)."""
        x = c0
        last = None
        while x < c1:
            left = c1 - x
            fam_tiles = []
            for v in variants:
                for t_id, t in tiles.items():
                    p = t.get("panel")
                    if p and p["id"] == (f"{family_prefix}_{tone_at(x, x + 4)}_{v}" if tone else
                                         f"{family_prefix}_{v}") and p["cell"] == [0, 0]:
                        fam_tiles.append(p)
            fits = [p for p in fam_tiles if p["size"][0] <= left]
            wide = [p for p in fits if p["size"][0] == max(q["size"][0] for q in fits)] if fits else []
            if left >= 4:
                wide = [p for p in fits if p["size"][0] == 4]
            elif left in (2, 3):
                wide = [p for p in fits if p["size"][0] == 2] or wide
            if not wide:
                raise SystemExit(f"row {row}: cannot fill {left} columns from {x} with {family_prefix}")
            choice = [p for p in wide if p["id"] != last] or wide
            p = choice[stable(row, x, salt, family_prefix) % len(choice)]
            last = p["id"]
            for i in range(p["size"][0]):
                tid = next(t_id for t_id, t in tiles.items() if t.get("panel", {}).get("id") == p["id"]
                           and t["panel"]["cell"] == [i, 0])
                grid[row][x + i] = tid
            x += p["size"][0]

    # --- collision: ceiling, floor stack, platforms, left end wall --------------------
    for r in range(rows):
        collision[r][0] = "wall_end_l" if r % 2 == 0 else "wall_end_l_b"
    collision[0][1] = "ceiling_fill_s"
    fill_run(0, 2, cols, "ceiling_fill", ["a", "s"])
    collision[1][1] = "ceiling_corner_in_l"
    fill_run(1, 2, cols, "cornice", ["a", "b", "s"])
    plats = sorted(PLATFORMS)
    deck_variants = {"deck_back": ["a", "b", "c"], "deck_front": ["a", "b", "c"],
                     "face_a": ["a", "c"], "face_b": ["a", "b", "c"]}
    top = [FLOOR_ROW] * cols          # deck_back row of every column
    for (c0, c1, h) in plats:
        for c in range(c0, c1 + 1):
            top[c] = FLOOR_ROW - h

    def single_for(c: int):
        if c == 1:
            return ("wall", "l")
        for (c0, c1, _h) in plats:
            if c == c0 - 1:
                return ("inner", "r")
            if c == c0:
                return ("end", "l")
            if c == c1:
                return ("end", "r")
            if c == c1 + 1:
                return ("inner", "l")
        return None

    def row_type(c: int, r: int):
        rel = r - top[c]
        return None if rel < 0 else ROW_TYPES[min(rel, 4)]

    for r in range(min(top[1:]), rows):
        c = 1
        while c < cols:
            rt = row_type(c, r)
            if rt is None:
                c += 1
                continue
            single = single_for(c)
            if rt == "fill" and c == 1:
                collision[r][c] = f"fill_wall_l_{fill_level(r)}"
                c += 1
                continue
            if rt != "fill" and single:
                collision[r][c] = f"{rt}_{single[0]}_{single[1]}"
                c += 1
                continue
            end = c + 1
            while end < cols and row_type(end, r) == rt and (rt == "fill" or not single_for(end)):
                end += 1
            if rt == "fill":
                # world running bond: joints at x = 16 * (r % 2) mod 32 whatever the run start
                joint = (16 * (r % 2) - c * TILE) % SLAB_W
                fill_run(r, c, end, f"fill_{fill_level(r)}_j{joint}", ["a", "c"], salt=f"f{r}")
            else:
                fill_run(r, c, end, rt, deck_variants[rt],
                         tone=rt in ("deck_back", "deck_front", "face_a"), salt=f"r{r}c{c}")
            c = end

    # --- backwall: bands, base course; empty inside arch windows ---------------------
    windows = []
    for b in BAYS:
        ox, oy = b * TILE + 16, WALL_TOP
        spec = window_spec()
        windows.append(((ox + spec["arc_centre"][0], oy + spec["arc_centre"][1], spec["arc_radius"]),
                        (ox + spec["rect"][0], oy + spec["rect"][1], spec["rect"][2], spec["rect"][3])))

    import env_kit  # same window rasterisation as the validator
    mask = env_kit.window_mask((cols * TILE, rows * TILE), windows)

    def cell_in_window(cx: int, r: int) -> bool:
        return env_kit._cell_hits_mask(mask, cx, r)

    for r in range(2, BASE_COURSE_ROW + 1):
        x = 1
        while x < cols:
            if collision[r][x] is not None or cell_in_window(x, r):
                x += 1
                continue
            end = x
            while end < cols and collision[r][end] is None and not cell_in_window(end, r):
                end += 1
            if r == BASE_COURSE_ROW:
                fill_run(r, x, end, "base_course", ["a", "b", "s", "t", "u", "v"], grid=backwall,
                         salt="base")
            else:
                band = band_of_row(r)
                fill_run(r, x, end, f"brick_{band}", ["a", "b", "c", "d", "s", "t"], grid=backwall,
                         salt=f"b{r}")
            x = end
    # the base floor's deck_back row hides the backwall; platforms cover their own rows

    # --- pieces ----------------------------------------------------------------------
    decals, structure, props, foreground = [], [], [], []
    for i, fx in enumerate(WALL_TORCHES):
        decals.append({"piece": POOL_PICK[i], "x": fx - POOL_CENTRE_LOCAL[0], "y": POOL_TOP_Y})
        props.append({"piece": "torch_wall", "x": fx - TORCH_FLAME_LOCAL[0],
                      "y": WALL_TORCH_FLAME_Y - TORCH_FLAME_LOCAL[1], "frame_offset": i % 4})
    for (gx, gy, variant) in ((76, 10, "b"), (97, 9, "a"), (123, 8, "b")):
        decals.append({"piece": f"grime_{variant}", "x": gx * TILE, "y": gy * TILE})
    for bi, side in SPILLS:
        decals.append({"piece": f"pier_spill_{side}",
                       "x": BAYS[bi] * TILE + (-48 if side == "l" else 288), "y": SPILL_TOP_Y})
    for j, b in enumerate(BAYS):
        ox = b * TILE
        structure += [
            {"piece": "pier_l", "x": ox, "y": WALL_TOP},
            {"piece": "arch_ring_a" if j == 0 else "arch_ring_b", "x": ox + 16, "y": WALL_TOP},
            {"piece": "pier_r", "x": ox + 224, "y": WALL_TOP},
            {"piece": "arch_threshold", "x": ox + 64, "y": WALL_TOP + 240},
        ]
        if j in CHAIN_BAYS:
            structure.append({"piece": "arch_chain", "x": ox + 80, "y": WALL_TOP + 80})
        for k, dx in enumerate((24, 264)):
            props.append({"piece": "torch_wall", "x": ox + dx, "y": PIER_TORCH_FLAME_Y - TORCH_FLAME_LOCAL[1],
                          "frame_offset": (j * 2 + k + 1) % 4})
    structure.append({"piece": "pilaster_plain", "x": 288, "y": WALL_TOP})
    structure.append({"piece": "pilaster_plain", "x": 1296, "y": WALL_TOP})
    props.append({"piece": "door_timber", "x": 216, "y": FLOOR_ROW * TILE - DOOR_H})
    for fx in UNLIT_SCONCES:
        props.append({"piece": "torch_wall_unlit", "x": fx - TORCH_FLAME_LOCAL[0],
                      "y": WALL_TORCH_FLAME_Y - TORCH_FLAME_LOCAL[1]})
    # C: one copper run. Vertical drop at column 54 (centre x 872), run along row 3 into the ceiling.
    pc, run_row = 54, 3
    # lichen_crust stays in the kit as an optional piece; the sample leaves the C wall clean
    props.append({"piece": "pipe_elbow_rd", "x": pc * TILE, "y": run_row * TILE})
    for c in range(pc + 2, 69):
        props.append({"piece": "pipe_h_seam" if c == 61 else "pipe_h", "x": c * TILE, "y": run_row * TILE})
    props.append({"piece": "pipe_elbow_lu", "x": 69 * TILE, "y": (run_row - 1) * TILE})
    props.append({"piece": "pipe_collar_ceiling", "x": 70 * TILE - 8, "y": WALL_TOP})
    for r in range(run_row + 2, BASE_COURSE_ROW + 1):
        props.append({"piece": "pipe_v_seam" if r == 13 else "pipe_v", "x": pc * TILE, "y": r * TILE})
    props.append({"piece": "pipe_collar_floor", "x": pc * TILE - 8, "y": FLOOR_ROW * TILE - 8})
    props.append({"piece": "pipe_flange_v", "x": pc * TILE - 8, "y": 104})
    props.append({"piece": "pipe_flange_v", "x": pc * TILE - 8, "y": 232})
    props.append({"piece": "pipe_bracket_v", "x": pc * TILE - 8, "y": 128})
    props.append({"piece": "pipe_bracket_v", "x": pc * TILE - 8, "y": 256})
    props.append({"piece": "valve_wheel", "x": pc * TILE - 8, "y": 160})
    props.append({"piece": "pipe_flange_h", "x": 63 * TILE - 8, "y": run_row * TILE - 8})
    props.append({"piece": "pipe_bracket_h", "x": 58 * TILE + 8, "y": run_row * TILE - 8})
    props.append({"piece": "pipe_bracket_h", "x": 66 * TILE + 8, "y": run_row * TILE - 8})
    foreground.append({"piece": "rubble_a", "x": 336, "y": 400 - 32})
    foreground.append({"piece": "rubble_b", "x": 1232, "y": 400 - 24})

    walk_y = FLOOR_ROW * TILE + TILE + WALK_LINE_Y
    exit_x = BAYS[1] * TILE + 152
    return {
        "format": "env-kit-layout-v2",
        "layout_id": "floor01_corridor_abc_sample",
        "kit": KIT_ID,
        "kit_contract": CONTRACT_REL,
        "status": "sample layout for art preview and COMP-01 integration; not authored level geometry",
        "migration": "env-kit-layout-v1 -> v2: base_floor_top_y replaced by base_walk_line_y; markers sit on the walking line; parallax_layers and lighting added; recess backwall tiles removed (arch windows are empty)",
        "tile_size": [TILE, TILE],
        "grid_size": [cols, rows],
        "pixel_size": [cols * TILE, rows * TILE],
        "coordinate_system": "top_left origin, x right, y down, rows top_to_bottom; piece x/y = top-left of the piece rect in layout pixels",
        "base_walk_line_y": walk_y,
        "requirements": {"continuous_floor": True, "max_step_tiles": 2},
        "segments": [{"id": sid, "grammar": g, "x": [c0 * TILE, c1 * TILE]} for sid, g, c0, c1 in SEGMENTS],
        "markers": [
            {"id": "player_spawn", "kind": "spawn", "x": 4 * TILE + 8, "y": walk_y - 1,
             "note": "actor pivot pixel (foot contact row = walking line - 1)"},
            {"id": "exit_east", "kind": "exit", "x": exit_x, "y": walk_y - 1,
             "rect": [exit_x - 64, 176, 128, 128], "note": "walk into the B2 arch"},
        ],
        "views": {
            "abc_640x360": [328, 20, 640, 360],
            "abc_640x400": [328, 0, 640, 400],
        },
        "parallax_layers": {
            "parallax_far": {"piece": "gallery_far", "x_origin": 115, "y": GALLERY_Y0 + DY},
            "parallax_mid": {"piece": "gallery_mid", "x_origin": 39, "y": GALLERY_Y0 + DY},
        },
        "parallax_rule": "A parallax strip pixel at strip-local u is drawn at screen x = x_origin + k * strip_width + u - camera_x * parallax (repeated for every integer k); camera_x is the view's left edge in layout px; y is fixed. Overview renders use camera_x = 0.",
        "lighting": {
            "profile_id": "floor01_torchlit_a1",
            "presentation_only": True,
            "groups": {
                "parallax_far": {"ambient": [0.74, 0.84, 1.05],
                                 "haze": {"colour": "#35517f", "amount": 0.26,
                                          "floor_fog": {"from_y": 246, "to_y": 288, "amount": 0.3}}},
                "parallax_mid": {"ambient": [0.62, 0.72, 0.98],
                                 "haze": {"colour": "#2b4069", "amount": 0.1,
                                          "floor_fog": {"from_y": 256, "to_y": 292, "amount": 0.2}}},
                "scene": {"ambient": [0.6, 0.62, 0.82]},
                "actors": {"ambient": [0.9, 0.92, 1.0], "light_scale": 0.3},
                "foreground": {"ambient": [0.5, 0.55, 0.75], "light_scale": 0.5},
            },
            "actor_contact_shadow": {"width_scale": 1.3, "half_height": 8.0, "strength": 0.7,
                                     "note": "soft ellipse centred on each actor's walking-line contact; darkens the scene group only"},
            "bloom": {"emissive_layers": ["prop", "parallax_mid", "parallax_far"],
                      "emissive_colours": RAMPS["flame"] + RAMPS["gal_warm"][3:],
                      "threshold": 214,
                      "intensity": 0.9, "radius_virtual": 2.5, "downsample_per_virtual_px": 0.5},
        },
        "preview": {
            "note": "Art-preview inputs only (not runtime data): approved actor atlases for the readability check and the owner-approved A1 target frame for the side-by-side.",
            "primary_view": "abc_640x360",
            "target_frame": "docs/evidence/lookdev-floor01-resolution-spike/native/a1_640x360.png",
            "actor_check": [
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": 328 + 64},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": 328 + 126},
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": 1012},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": 1440},
            ],
        },
        "tile_layers": {"backwall": backwall, "collision": collision},
        "piece_layers": {"backwall_decal": decals, "structure": structure, "prop": props,
                         "foreground": foreground},
    }


# ---------------------------------------------------------------------------
# Writers
# ---------------------------------------------------------------------------

def dump_json(data, path: Path) -> None:
    """Indented JSON with short scalar arrays and flat objects kept on one line."""
    path.parent.mkdir(parents=True, exist_ok=True)

    def inline(value) -> bool:
        if isinstance(value, list):
            return all(not isinstance(v, (list, dict)) for v in value)
        if isinstance(value, dict):
            return len(json.dumps(value)) <= 96 and all(inline(v) or not isinstance(v, (list, dict))
                                                        for v in value.values())
        return True

    def render(value, indent: int) -> str:
        pad = "  " * (indent + 1)
        if inline(value) or not isinstance(value, (list, dict)):
            return json.dumps(value)
        if isinstance(value, list):
            body = ",\n".join(pad + render(v, indent + 1) for v in value)
            return "[\n" + body + "\n" + "  " * indent + "]"
        body = ",\n".join(f"{pad}{json.dumps(k)}: {render(v, indent + 1)}" for k, v in value.items())
        return "{\n" + body + "\n" + "  " * indent + "}"

    path.write_text(render(data, 0) + "\n", encoding="utf-8")


def dump_grid(grid: dict, path: Path) -> None:
    """pixel-grid-v1 with one pixel row per line (diff friendly)."""
    lines = ["{", f'  "format": "{grid["format"]}",',
             f'  "canvas": {json.dumps(grid["canvas"])},',
             f'  "pivot": {json.dumps(grid["pivot"])},', '  "palette": [']
    lines.append(",\n".join(f"    {json.dumps(e)}" for e in grid["palette"]))
    lines.append("  ],")
    lines.append(f'  "layers": {json.dumps(grid["layers"])},')
    lines.append('  "frames": [')
    frame_txt = []
    for frame in grid["frames"]:
        cel_txt = []
        for layer, rows in frame["cels"].items():
            body = ",\n".join(f"        {json.dumps(r)}" for r in rows)
            cel_txt.append(f'      {json.dumps(layer)}: [\n{body}\n      ]')
        frame_txt.append('    {"duration_ms": %d, "cels": {\n%s\n    }}'
                         % (frame["duration_ms"], ",\n".join(cel_txt)))
    lines.append(",\n".join(frame_txt))
    lines.append("  ],")
    lines.append(f'  "tags": {json.dumps(grid["tags"])}')
    lines.append("}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def dump_layout(layout: dict, path: Path) -> None:
    head = {k: v for k, v in layout.items() if k not in ("tile_layers", "piece_layers")}
    text = json.dumps(head, indent=2)[:-2]
    parts = [text + ","]
    parts.append('  "tile_layers": {')
    layer_txt = []
    for name, grid in layout["tile_layers"].items():
        rows = ",\n".join(f"      {json.dumps(row)}" for row in grid)
        layer_txt.append(f'    "{name}": [\n{rows}\n    ]')
    parts.append(",\n".join(layer_txt))
    parts.append("  },")
    parts.append('  "piece_layers": {')
    layer_txt = []
    for name, items in layout["piece_layers"].items():
        if not items:
            layer_txt.append(f'    "{name}": []')
            continue
        body = ",\n".join(f"      {json.dumps(item)}" for item in items)
        layer_txt.append(f'    "{name}": [\n{body}\n    ]')
    parts.append(",\n".join(layer_txt))
    parts.append("  }")
    parts.append("}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(parts) + "\n", encoding="utf-8")


STALE = [TERRAIN_DIR / "recess.grid.json"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--force", action="store_true",
                        help="overwrite existing grids/manifests/contract/layout (discards hand edits)")
    parser.add_argument("--no-ase", action="store_true", help="skip rebuilding .ase files")
    args = parser.parse_args(argv)
    grid_paths = {name: ROOT / f"{stem}.grid.json" for name, (stem, _k) in ATLAS_FILES.items()}
    outputs = list(grid_paths.values()) + [CONTRACT_PATH, LAYOUT_PATH]
    existing = [p for p in outputs if p.exists()]
    if existing and not args.force:
        print("Refusing to overwrite (pass --force):\n  " +
              "\n  ".join(str(p.relative_to(ROOT)) for p in existing), file=sys.stderr)
        return 1
    built = build_all()
    for name, grid in built["grids"].items():
        dump_grid(grid, grid_paths[name])
        if not args.no_ase:
            ase = grid_paths[name].with_suffix("").with_suffix(".ase")
            subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build",
                            str(grid_paths[name]), str(ase)], check=True, stdout=subprocess.DEVNULL)
        print(f"wrote {grid_paths[name].relative_to(ROOT)} ({grid['canvas'][0]}x{grid['canvas'][1]}, "
              f"{len(grid['palette'])} colours)")
    for name, m in built["manifests"].items():
        dump_json(m, MANIFEST_DIR / f"{name}.json")
    dump_json(built["contract"], CONTRACT_PATH)
    dump_layout(built["layout"], LAYOUT_PATH)
    print("wrote manifests, contract and layout")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
