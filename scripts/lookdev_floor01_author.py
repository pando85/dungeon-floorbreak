#!/usr/bin/env python3
"""Author the Floor 1 look-dev resolution-spike pixel layers (deterministic).

LOOK-DEV ONLY. This is not production art, not a tile kit and not runtime data.
It writes editable ``pixel-grid-v1`` sources (one grid per layer and density)
plus a ``scene.json`` describing layer offsets, draw order and parallax, then
builds a native ``.ase`` for every grid with ``scripts/pixel_grid_to_ase.py``.

One composition is authored at two pixel densities from the same design
coordinates (640x360 virtual "design units"):

* ``a640`` -- 1 design unit = 1 px, 640x400 canvas (the 640x360 view is rows
  20..379; the 640x400 view is the whole canvas, for 16:10 displays).
* ``b960`` -- 1 design unit = 1.5 px, 960x540 canvas. This is a *redraw*: every
  pixel is evaluated at the higher density with density-specific detailing
  (bevel depth, pits, chips, links, flame shape). Nothing is resampled.

Lighting is baked into the pixels with a tiny deterministic "pixel shader":
each surface pixel has a design-space position, a depth and a normal; torches
and a cold bounce from the gallery are evaluated with a smooth falloff and the
result is quantised onto hand-picked, hue-shifted colour ramps (cold blue-violet
shadows -> warm orange torch light). Flat faces are shaded per brick/block so
the result keeps clean pixel-art clusters instead of per-pixel gradients.

The concept image ``assets/concept/floor-01/opening/01-catacomb-environment-clean.png``
was used as a *mood* reference only (lighting, colour temperature, volume). It
is never read by this script.

Usage::

    python3 scripts/lookdev_floor01_author.py            # write grids + .ase
    python3 scripts/lookdev_floor01_author.py --no-ase   # grids + scene only
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets" / "source" / "lookdev" / "floor01"
SCENE_FILE = OUT_DIR / "scene.json"

# ---------------------------------------------------------------------------
# Colour ramps: index 0 = deepest cold shadow, last = hottest torch light.
# Hue shifts toward blue-violet in shadow and toward yellow-orange in light.
# ---------------------------------------------------------------------------
RAMPS: dict[str, list[str]] = {
    "brick": ["#110e1f", "#1a1630", "#271c3a", "#3a223d", "#522a3b", "#6c3434",
              "#8a4330", "#aa5631", "#c86d36", "#e08b42", "#f3ad5a", "#ffd28a"],
    "brick_cold": ["#1c1d33", "#262b48", "#33405f", "#42557a"],
    "mortar": ["#0c0912", "#150f1b", "#201622", "#2d1d26", "#3e2629", "#57352c", "#74472f"],
    "stone": ["#15131f", "#221c2b", "#312636", "#45313c", "#5d4045", "#7a5249",
              "#9a684f", "#b98256", "#d0965c", "#e3aa68", "#f2c486"],
    "stone_cold": ["#1f2338", "#2a3250", "#38456a", "#4b5d86", "#647ca4"],
    "floor": ["#110d18", "#1b1522", "#281d2b", "#382530", "#4c3034", "#663e36",
              "#844f38", "#a5633b", "#c3763c", "#da8d44", "#eeac58"],
    "floor_cold": ["#1a1b2e", "#242a42", "#2f3a57", "#3c4a6a"],
    "gal": ["#070a14", "#0b1020", "#10172c", "#162038", "#1d2a47", "#253657",
            "#2f4469", "#3a537c", "#47648f", "#5676a2", "#6a8bb5"],
    "gal_warm": ["#2e2436", "#4a2f38", "#6e3f37", "#9a5735", "#c97a3a", "#f2b060", "#ffe2a0"],
    "copper": ["#1e0f10", "#3a1a14", "#5c2a18", "#823e1e", "#a85626", "#cc7332",
               "#e89748", "#fbc070", "#fff0c8"],
    "copper_cold": ["#33283f", "#4b3d5c"],
    "brass": ["#221a10", "#43331a", "#6a5224", "#93752e", "#bc9a3a", "#e0c25a", "#fff0a6"],
    "iron": ["#0e0b10", "#1b1519", "#2b2226", "#3e3133", "#574340", "#7a5c4e", "#a07a5c"],
    "flame": ["#8a2a12", "#d4501a", "#f58a26", "#ffc048", "#ffe98a", "#fffbe6"],
}

# ---------------------------------------------------------------------------
# Composition in design units (640x360 view; y may run -20..380 for 640x400).
# ---------------------------------------------------------------------------
FLOOR_BACK = 268      # wall base line on the floor plane
FLOOR_LIP = 292       # front edge of the walkable floor plane
WALK_Y = 288          # actor contact line (pivot row = WALK_Y - 1)
DEPTH_PER_ROW = 3.0   # world depth per screen row on the floor plane
VP = (336.0, 200.0)   # vanishing point used for the slight top-down volume

ARCH_CX, ARCH_SPRING = 336.0, 146.0
R_IN, R_OUT = 96.0, 122.0
BACK_CY, R_BACK = 152.0, 86.0
PILLAR_Z = 24.0       # pillars and arch ring protrude from the wall plane
L_PILLAR = (196, 240)  # front face x-range of the left pillar
JAMB_W = 10
IMPOST_Y = (146, 160)
STRING_Y = (8, 18)
WALL_BASE_Y = (252, 268)

TORCHES = [  # flame centre, design units; z = protrusion from the wall
    {"x": 112.0, "y": 158.0, "z": 16.0},
    {"x": 218.0, "y": 176.0, "z": 16.0 + PILLAR_Z, "p": 0.75},
    {"x": 454.0, "y": 176.0, "z": 16.0 + PILLAR_Z, "p": 0.8},
    {"x": 600.0, "y": 158.0, "z": 16.0, "p": 0.85},
]
TORCH_RADIUS = 215.0
TORCH_POWER = 1.2
COLD_BOUNCE = {"x": 336.0, "y": 196.0, "z": -70.0, "r": 170.0, "i": 1.0}
MID_TORCH = {"x": 293.0, "y": 206.0}
FAR_TORCH_XS = (270.0, 314.0, 358.0, 402.0)  # far piers visible between mid pillars
FAR_TORCH_Y = 204.0

PIPE_X = (539.0, 553.0)
PIPE_RUN_Y = (33.0, 47.0)
PIPE_BOTTOM = 270.0
VALVE = (546.0, 158.0)
CHAIN = ((268.0, 100.0), (404.0, 100.0), 30.0)  # ends and sag, design units

ACTOR_MARKS = [  # contact shadows (x, half width) baked into the floor
    (64.0, 13.0), (126.0, 18.0),
]

DENSITIES = {
    "a640": {"s": 1.0, "canvas": (640, 400), "origin": (0.0, -20.0),
             "views": {"640x360": [0, 20, 640, 360], "640x400": [0, 0, 640, 400]}},
    "b960": {"s": 1.5, "canvas": (960, 540), "origin": (0.0, 0.0),
             "views": {"960x540": [0, 0, 960, 540]}},
}

LAYERS = [  # draw order bottom -> top (actors are composited from character sources)
    ("far_gallery", 0.35, "gallery_far"),
    ("mid_gallery", 0.6, "gallery_mid"),
    ("backwall", 1.0, "scene"),
    ("terrain", 1.0, "scene"),
    ("structure", 1.0, "scene"),
    ("props", 1.0, "scene"),
    ("foreground", 1.15, "foreground"),
]
ACTOR_LAYER_INDEX = 6  # actors are drawn after props, before foreground

KEY_ALPHABET = "".join(chr(c) for c in range(33, 127) if chr(c) not in '."\\') + \
    "".join(chr(c) for c in range(0xC0, 0x100))


# ---------------------------------------------------------------------------
# Deterministic helpers
# ---------------------------------------------------------------------------

def rnd(*key) -> float:
    """Stable pseudo-random float in [0, 1) for a key tuple."""
    digest = hashlib.blake2b(repr(key).encode(), digest_size=8).digest()
    return int.from_bytes(digest, "little") / 2.0 ** 64


def clamp(v: float, lo: float, hi: float) -> float:
    return lo if v < lo else hi if v > hi else v


def norm(x: float, y: float, z: float) -> tuple[float, float, float]:
    d = math.sqrt(x * x + y * y + z * z) or 1.0
    return x / d, y / d, z / d


def falloff(d: float, r: float) -> float:
    """Windowed inverse-square-like falloff: hot core, long dim tail, exactly 0 at r."""
    t = d / r
    if t >= 1.0:
        return 0.0
    return (1.0 - t * t) ** 2 / (1.0 + 6.0 * t * t)


def warm_light(x: float, y: float, z: float, n: tuple, wrap: float = 0.5,
               torches=TORCHES, radius: float = TORCH_RADIUS, power: float = TORCH_POWER) -> float:
    total = 0.0
    for t in torches:
        dx, dy, dz = t["x"] - x, t["y"] - y, t["z"] - z
        d = math.sqrt(dx * dx + dy * dy + dz * dz) or 1e-6
        f = falloff(d, radius)
        if f <= 0.0:
            continue
        ndl = (n[0] * dx + n[1] * dy + n[2] * dz) / d
        lam = max(0.0, (ndl + wrap) / (1.0 + wrap))
        total += power * t.get("p", 1.0) * f * lam
    return total


def cold_light(x: float, y: float, z: float, n: tuple) -> float:
    c = COLD_BOUNCE
    dx, dy, dz = c["x"] - x, c["y"] - y, c["z"] - z
    d = math.sqrt(dx * dx + dy * dy + dz * dz) or 1e-6
    f = falloff(d, c["r"])
    if f <= 0.0:
        return 0.0
    ndl = (n[0] * dx + n[1] * dy + n[2] * dz) / d
    return c["i"] * f * max(0.0, ndl)


def ramp_index(ramp: str, value: float) -> int:
    return int(clamp(round(value), 0, len(RAMPS[ramp]) - 1))


# ---------------------------------------------------------------------------
# Layer canvas
# ---------------------------------------------------------------------------

class Canvas:
    def __init__(self, name: str, density: str):
        cfg = DENSITIES[density]
        self.name = name
        self.density = density
        self.s = cfg["s"]
        self.w, self.h = cfg["canvas"]
        self.ox, self.oy = cfg["origin"]
        self.px: list[list[tuple[str, int] | None]] = [[None] * self.w for _ in range(self.h)]
        self.hi = self.s > 1.0  # density-specific detailing switch

    # design <-> pixel
    def dx(self, px: int) -> float:
        return self.ox + (px + 0.5) / self.s

    def dy(self, py: int) -> float:
        return self.oy + (py + 0.5) / self.s

    def pxr(self, x0: float, x1: float) -> range:
        a = max(0, math.floor((x0 - self.ox) * self.s))
        b = min(self.w, math.ceil((x1 - self.ox) * self.s))
        return range(a, b)

    def pyr(self, y0: float, y1: float) -> range:
        a = max(0, math.floor((y0 - self.oy) * self.s))
        b = min(self.h, math.ceil((y1 - self.oy) * self.s))
        return range(a, b)

    def gx(self, px: int) -> int:  # global density-pixel coordinate (pattern space)
        return px + round(self.ox * self.s)

    def gy(self, py: int) -> int:
        return py + round(self.oy * self.s)

    def put(self, px: int, py: int, ramp: str, idx: float) -> None:
        if 0 <= px < self.w and 0 <= py < self.h:
            self.px[py][px] = (ramp, ramp_index(ramp, idx))

    def get(self, px: int, py: int):
        if 0 <= px < self.w and 0 <= py < self.h:
            return self.px[py][px]
        return None

    def clear(self, px: int, py: int) -> None:
        if 0 <= px < self.w and 0 <= py < self.h:
            self.px[py][px] = None

    def unit(self, v: float) -> int:
        """A design length in whole pixels at this density (at least 1)."""
        return max(1, round(v * self.s))


# ---------------------------------------------------------------------------
# Shared geometry predicates (design units)
# ---------------------------------------------------------------------------

def in_front_opening(x: float, y: float) -> bool:
    """Inside the arch opening as seen on the front (pillar) plane."""
    lx, rx = L_PILLAR[1], 672 - L_PILLAR[1]
    if lx <= x < rx and ARCH_SPRING <= y < FLOOR_BACK + 12:
        return True
    return y < ARCH_SPRING and (x - ARCH_CX) ** 2 + (y - ARCH_SPRING) ** 2 < R_IN ** 2


def in_back_aperture(x: float, y: float) -> bool:
    lx, rx = L_PILLAR[1] + JAMB_W, 672 - L_PILLAR[1] - JAMB_W
    if lx <= x < rx and BACK_CY <= y < FLOOR_BACK + 12:
        return True
    return y < BACK_CY and (x - ARCH_CX) ** 2 + (y - BACK_CY) ** 2 < R_BACK ** 2


def pillar_front(x: float) -> bool:
    return L_PILLAR[0] <= x < L_PILLAR[1] or 672 - L_PILLAR[1] <= x < 672 - L_PILLAR[0]


# ---------------------------------------------------------------------------
# Masonry shading
# ---------------------------------------------------------------------------

def bevel_normal(du: int, dv: int, bw: int, bh: int, depth: int):
    """Normal for a block pixel at local (du, dv) inside a bw x bh face (mortar excluded).

    ``depth`` is the bevel width in pixels (1 at 640, 2 at 960).
    Returns (normal, edge_strength) where edge_strength 0 = flat face.
    """
    left, right = du, bw - 1 - du
    top, bottom = dv, bh - 1 - dv
    m = min(left, right, top, bottom)
    if m >= depth:
        return (0.0, 0.0, 1.0), 0
    tilt = 0.75 if m == 0 else 0.4
    nx = ny = 0.0
    if left == m:
        nx = -tilt
    elif right == m:
        nx = tilt
    if top == m:
        ny = -tilt
    elif bottom == m:
        ny = tilt
    return norm(nx, ny, 1.0), 1 if m == 0 else 2


def masonry_pixel(c: Canvas, px: int, py: int, *, ramp: str, cold_ramp: str | None,
                  bw: int, bh: int, z: float, seed: str, offset_rows: bool = True,
                  gain: float = 10.2, base: float = 1.2, pits: bool = True,
                  col_shift: int = 0, row_shift: int = 0, soot=None) -> None:
    """Shade one pixel of a running-bond block wall on a plane facing the camera."""
    gx, gy = c.gx(px) + col_shift, c.gy(py) + row_shift
    row = gy // bh
    off = (bw // 2) * (row % 2) if offset_rows else 0
    col = (gx + off) // bw
    du, dv = (gx + off) % bw, gy % bh
    # brick-centre design coordinates (flat faces are lit per block)
    cx_px = col * bw - off + bw / 2 - col_shift
    cy_px = row * bh + bh / 2 - row_shift
    bx = c.ox + (cx_px - round(c.ox * c.s)) / c.s
    by = c.oy + (cy_px - round(c.oy * c.s)) / c.s
    x, y = c.dx(px), c.dy(py)
    jit = (rnd(seed, row, col) - 0.5) * 1.3
    special = rnd(seed, row, col, "kind")
    if special < 0.05:
        jit -= 1.1
    elif special > 0.97:
        jit += 0.8
    sootv = soot(bx, by) if soot else 0.0
    if du == 0 or dv == 0:  # mortar joint
        w = warm_light(x, y, z - 2, (0.0, 0.0, 1.0))
        c.put(px, py, "mortar", base * 0.4 + gain * 0.5 * w - sootv * 0.6)
        return
    # chipped corners: a few blocks lose a small corner wedge to the mortar
    chip = rnd(seed, row, col, "chip")
    if chip < 0.16:
        size = 3 if c.hi else 1
        corner = int(chip * 25) % 4
        cu = du - 1 if corner in (0, 2) else bw - 1 - du
        cv = dv - 1 if corner in (0, 1) else bh - 1 - dv
        if cu + cv < size:
            w = warm_light(x, y, z - 2, (0.0, 0.0, 1.0))
            c.put(px, py, "mortar", base * 0.4 + gain * 0.5 * w - sootv * 0.6 + 0.6)
            return
    n, edge = bevel_normal(du - 1, dv - 1, bw - 1, bh - 1, 2 if c.hi else 1)
    wface = warm_light(bx, by, z, (0.0, 0.0, 1.0))
    if edge:
        wpx = warm_light(x, y, z, n)
        w = wpx * (1.0 if edge == 1 else 0.5) + wface * (0.0 if edge == 1 else 0.5)
    else:
        w = wface
    value = base + gain * (w ** 0.7) + jit - sootv
    if c.hi and not edge:
        # the denser grid can afford a gentle in-block light gradient near torches
        wpx = warm_light(x, y, z, (0.0, 0.0, 1.0))
        value += 0.45 * gain * (wpx ** 0.7 - wface ** 0.7)
        # occasional hairline crack, one step darker
        if rnd(seed, row, col, "crack") < 0.035:
            cu0 = 3 + int(rnd(seed, row, col, "cu") * (bw - 12))
            if (du - cu0) - (dv - 3) // 2 == 0 and 3 <= dv < min(bh - 3, 9):
                value -= 1.4
    # surface pits: small 2-pixel clusters, one step darker with a lit lower lip
    if pits and not edge:
        count = 1 + int(rnd(seed, row, col, "n") * (4 if c.hi else 2.2))
        for k in range(count):
            pu = 2 + int(rnd(seed, row, col, k, "u") * max(1, bw - 6))
            pv = 2 + int(rnd(seed, row, col, k, "v") * max(1, bh - 5))
            size = 3 if c.hi and rnd(seed, row, col, k, "s") > 0.5 else 2
            if pu <= du < pu + size and pv <= dv < pv + (2 if c.hi else 1):
                value -= 1.2
            elif c.hi and pu <= du < pu + size and dv == pv + 2 and w > 0.35:
                value += 0.7
    if cold_ramp:
        cold = cold_light(x, y, z, n)
        if cold > 0.32 and cold > w * 1.6:
            c.put(px, py, cold_ramp, cold * 3.4 - 0.6 + jit * 0.4)
            return
    c.put(px, py, ramp, value)


def soot_above_torches(bx: float, by: float) -> float:
    """Darkening of the bricks directly above each torch flame."""
    total = 0.0
    for t in TORCHES:
        dy = t["y"] - by
        if 6 < dy < 90:
            half = 7 + dy * 0.12
            if abs(bx - t["x"]) < half:
                total += 1.6 * (1.0 - dy / 90.0)
    return total


# ---------------------------------------------------------------------------
# Layers
# ---------------------------------------------------------------------------

def layer_backwall(c: Canvas) -> None:
    bw, bh = c.unit(20), c.unit(10)
    for py in range(c.h):
        y = c.dy(py)
        for px in range(c.w):
            x = c.dx(px)
            if in_front_opening(x, y):
                continue
            if y >= FLOOR_BACK:
                continue
            if STRING_Y[0] <= y < STRING_Y[1]:
                string_course(c, px, py, x, y)
                continue
            if WALL_BASE_Y[0] <= y < WALL_BASE_Y[1]:
                masonry_pixel(c, px, py, ramp="stone", cold_ramp=None, bw=c.unit(40),
                              bh=c.unit(16), z=4.0, seed="base", gain=8.4, base=1.0,
                              row_shift=-round((WALL_BASE_Y[0] - c.oy) * c.s) % c.unit(16))
                continue
            masonry_pixel(c, px, py, ramp="brick", cold_ramp=None, bw=bw, bh=bh, z=0.0,
                          seed="wall", soot=soot_above_torches)


def string_course(c: Canvas, px: int, py: int, x: float, y: float) -> None:
    """A protruding stone band; we see its underside (it is above the eye line)."""
    t = (y - STRING_Y[0]) / (STRING_Y[1] - STRING_Y[0])
    if t > 0.78:  # underside, facing down
        w = warm_light(x, y, 6.0, (0.0, 1.0, 0.3))
        c.put(px, py, "stone", 0.5 + 6.0 * w)
        return
    bw = c.unit(48)
    gx = c.gx(px)
    if gx % bw == 0:
        c.put(px, py, "mortar", 1.5 + 4.0 * warm_light(x, y, 6.0, (0, 0, 1)))
        return
    n = (0.0, -0.6, 0.8) if t < 0.15 else (0.0, 0.0, 1.0)
    w = warm_light(x, y, 8.0, n)
    jit = (rnd("string", gx // bw) - 0.5) * 0.9
    c.put(px, py, "stone", 1.2 + 9.0 * w + jit + (0.8 if t < 0.15 else 0.0))


def layer_structure(c: Canvas) -> None:
    for side in (0, 1):
        pillar(c, side)
    arch_ring(c)


def mirror_x(x: float, side: int) -> float:
    return x if side == 0 else 672.0 - x


def pillar(c: Canvas, side: int) -> None:
    f0, f1 = L_PILLAR
    stone_bw = c.unit(22)
    course_h = c.unit(16)
    z = PILLAR_Z
    # x range in design units covering plinth, face and jamb (left pillar coordinates)
    lo, hi = 186.0, 252.0
    xs = (lo, hi) if side == 0 else (672.0 - hi, 672.0 - lo)
    for py in c.pyr(STRING_Y[1], 282):
        y = c.dy(py)
        for px in c.pxr(*xs):
            x = c.dx(px)
            lx = mirror_x(x, side)  # left-pillar local design x
            sign = 1.0 if side == 0 else -1.0
            # plinth: two protruding steps standing on the floor plane
            if 255 <= y < 280 and 188 <= lx < 248:
                plinth_pixel(c, px, py, x, y, lx, side)
                continue
            # impost (capital): a projecting block where the arch springs
            if IMPOST_Y[0] - 3 <= y < IMPOST_Y[1] + 2 and 190 <= lx < 246 + JAMB_W - 4:
                impost_pixel(c, px, py, x, y, lx, side)
                continue
            # jamb: inner side face of the pillar, facing the opening
            if f1 <= lx < f1 + JAMB_W and IMPOST_Y[1] + 2 <= y:
                depth_t = (lx - f1) / JAMB_W
                bottom = 278 - (lx - f1) * 1.6
                if y >= bottom:
                    continue
                jamb_pixel(c, px, py, x, y, depth_t, sign, course_h)
                continue
            # front face: shaft below impost, pilaster above it
            if f0 <= lx < f1 and y < 255:
                if y < IMPOST_Y[0] and (x - ARCH_CX) ** 2 + (y - ARCH_SPRING) ** 2 < (R_OUT + 1) ** 2:
                    continue  # the arch ring is drawn on top
                block_face(c, px, py, x, y, z, stone_bw, course_h, side, top_ref=STRING_Y[1])


def block_face(c: Canvas, px, py, x, y, z, bw, bh, side, top_ref) -> None:
    """Large dressed stone blocks, two per course in alternating bond."""
    ref_py = round((top_ref - c.oy) * c.s)
    local_y = py - ref_py
    row = local_y // bh
    dv = local_y % bh
    face0 = round((mirror_x(L_PILLAR[0], side) if side == 0 else 672 - L_PILLAR[1] - c.ox) * c.s) \
        if side == 1 else round((L_PILLAR[0] - c.ox) * c.s)
    local_x = px - face0
    width = round((L_PILLAR[1] - L_PILLAR[0]) * c.s)
    joints = [0, width // 2] if row % 2 == 0 else [0, width // 4, (3 * width) // 4]
    start = max(j for j in joints if j <= local_x)
    ends = [j for j in joints if j > local_x] + [width]
    end = min(ends)
    du = local_x - start
    blk_w = end - start
    if dv == 0 or du == 0:
        w = warm_light(x, y, z - 2, (0.0, 0.0, 1.0))
        c.put(px, py, "mortar", 0.8 + 5.4 * w)
        return
    n, edge = bevel_normal(du - 1, dv - 1, blk_w - 1, bh - 1, 2 if c.hi else 1)
    # block centre
    bx = c.ox + (face0 + start + blk_w / 2) / c.s
    by = c.oy + (ref_py + row * bh + bh / 2) / c.s
    jit = (rnd("pillar", side, row, start) - 0.5) * 1.0
    wf = warm_light(bx, by, z, (0.0, 0.0, 1.0))
    if edge:
        wp = warm_light(x, y, z, n)
        w = wp if edge == 1 else 0.5 * (wp + wf)
    else:
        w = wf
    value = 1.2 + 10.0 * (w ** 0.7) + jit
    if not edge:  # weathering cluster per block
        pu = int(rnd("pillar", side, row, start, "u") * max(1, blk_w - 6)) + 2
        pv = int(rnd("pillar", side, row, start, "v") * max(1, bh - 6)) + 2
        if pu <= du < pu + (4 if c.hi else 2) and pv <= dv < pv + (2 if c.hi else 1):
            value -= 1.1
    # cold rim on the inner front edge (catches the gallery glow)
    inner_edge = (side == 0 and local_x >= width - (2 if c.hi else 1)) or \
        (side == 1 and local_x <= (1 if c.hi else 0))
    if inner_edge and y > IMPOST_Y[1]:
        cold = cold_light(x, y, z, (0.6 if side == 0 else -0.6, 0.0, 0.8))
        if cold > 0.15:
            c.put(px, py, "stone_cold", 1.2 + cold * 3.0)
            return
    c.put(px, py, "stone", value)


def jamb_pixel(c: Canvas, px, py, x, y, depth_t, sign, course_h) -> None:
    z = PILLAR_Z - depth_t * 64.0
    n = (sign, 0.0, 0.0)
    ref_py = round((STRING_Y[1] - c.oy) * c.s)
    dv = (py - ref_py) % course_h
    w = warm_light(x, y, z, n, wrap=0.2)
    cold = cold_light(x, y, z, n)
    if dv == 0:
        c.put(px, py, "mortar", 0.5 + 4 * w + 1.5 * cold)
        return
    if cold > w * 0.9:
        c.put(px, py, "stone_cold", 0.2 + cold * 3.6 - (0.6 if dv == 1 else 0))
    else:
        c.put(px, py, "stone", 1.0 + 8.0 * w + (0.6 if dv == 1 else 0.0))


def impost_pixel(c: Canvas, px, py, x, y, lx, side) -> None:
    y0, y1 = IMPOST_Y
    z = PILLAR_Z + 4.0
    if y < y0:  # thin top fillet
        n = (0.0, -0.7, 0.7)
    elif y >= y1:  # underside (above the eye line we see it)
        n = (0.0, 0.8, 0.6)
    elif y >= y1 - 4:  # chamfer
        n = (0.0, 0.55, 0.83)
    else:
        n = (0.0, 0.0, 1.0)
    if lx >= L_PILLAR[1] + 2:  # side of the impost toward the opening
        n = ((1.0 if side == 0 else -1.0) * 0.8, n[1], 0.6)
    w = warm_light(x, y, z, n)
    seg = c.unit(28)
    if c.gx(px) % seg == 0 and y0 <= y < y1 - 4:
        c.put(px, py, "mortar", 1.0 + 5.0 * w)
        return
    bonus = 0.9 if y < y0 + (2 if c.hi else 1) else 0.0
    c.put(px, py, "stone", 1.3 + 9.4 * w + bonus)


def plinth_pixel(c: Canvas, px, py, x, y, lx, side) -> None:
    # upper step: 192..244 x 255..264 ; lower step: 188..248 x 264..280
    upper = 192 <= lx < 244 and y < 264
    lower = y >= 264
    if not (upper or lower):
        return
    if upper:
        z, top = PILLAR_Z + 3.0, 255.0
    else:
        z, top = PILLAR_Z + 7.0, 264.0
    if y < top + 2.0:  # visible top surface of the step (below the eye line)
        n = (0.0, -1.0, 0.25)
        w = warm_light(x, y, z, n)
        c.put(px, py, "stone", 2.0 + 9.0 * w)
        return
    if lower and y >= 278:
        c.put(px, py, "stone", 0.3)  # contact shadow line on the floor
        return
    n = (0.0, 0.0, 1.0)
    edge_l = lx < (190 if upper else 186) + 4
    w = warm_light(x, y, z, n)
    seg = c.unit(30)
    if c.gx(px) % seg == 0:
        c.put(px, py, "mortar", 1.0 + 5 * w)
        return
    shade = -0.6 * ((y - top) / 14.0)
    c.put(px, py, "stone", 1.2 + 9.0 * w + shade - (0.4 if edge_l else 0))


def arch_ring(c: Canvas) -> None:
    n_vous = 15
    step = math.pi / n_vous
    zf = PILLAR_Z
    for py in c.pyr(ARCH_SPRING - R_OUT - 6, ARCH_SPRING + 1):
        y = c.dy(py)
        for px in c.pxr(ARCH_CX - R_OUT - 6, ARCH_CX + R_OUT + 6):
            x = c.dx(px)
            rx, ry = x - ARCH_CX, ARCH_SPRING - y
            if ry < 0:
                continue
            r = math.hypot(rx, ry)
            theta = math.atan2(ry, rx)  # 0 right .. pi left
            k = min(n_vous - 1, int((math.pi - theta) / step))
            keystone = k == n_vous // 2
            r_out = R_OUT + (5.0 if keystone else 0.0)
            if R_IN <= r < r_out:
                vous_pixel(c, px, py, x, y, r, theta, k, step, r_out, keystone)
            elif r < R_IN:
                soffit_pixel(c, px, py, x, y, r, theta)
            elif r < r_out + 1.0 / c.s * 1.0 and not keystone:
                # dark outline where the ring meets the wall
                c.put(px, py, "mortar", 1.0 + 3.0 * warm_light(x, y, 0.0, (0, 0, 1)))


def vous_pixel(c, px, py, x, y, r, theta, k, step, r_out, keystone) -> None:
    z = PILLAR_Z + (3.0 if keystone else 0.0)
    pos = (math.pi - theta) / step
    frac = pos - k
    arc_px = r * step * c.s
    d_lo, d_hi = frac * arc_px, (1.0 - frac) * arc_px
    d_in, d_out = (r - R_IN) * c.s, (r_out - r) * c.s
    if min(d_lo, d_hi) < 1.0:
        w = warm_light(x, y, z - 2, (0, 0, 1))
        c.put(px, py, "mortar", 0.8 + 5.0 * w)
        return
    radial = (math.cos(theta), -math.sin(theta))  # outward in screen space
    tangent = (-math.sin(theta), -math.cos(theta))
    bev = 2.0 if c.hi else 1.0
    n = (0.0, 0.0, 1.0)
    edge = 0.0
    m = min(d_lo - 1.0, d_hi - 1.0, d_in, d_out)
    if m < bev:
        tilt = 0.75 if m < 1.0 else 0.4
        edge = 1.0 if m < 1.0 else 0.5
        if m == d_in:
            n = norm(-radial[0] * tilt, -radial[1] * tilt, 1.0)
        elif m == d_out:
            n = norm(radial[0] * tilt, radial[1] * tilt, 1.0)
        elif m == d_lo - 1.0:
            n = norm(tangent[0] * tilt, tangent[1] * tilt, 1.0)
        else:
            n = norm(-tangent[0] * tilt, -tangent[1] * tilt, 1.0)
    mid_theta = math.pi - (k + 0.5) * step
    mr = (R_IN + r_out) / 2
    bx, by = ARCH_CX + math.cos(mid_theta) * mr, ARCH_SPRING - math.sin(mid_theta) * mr
    wf = warm_light(bx, by, z, (0, 0, 1))
    w = wf if edge == 0 else (warm_light(x, y, z, n) * edge + wf * (1 - edge))
    jit = (rnd("vous", k) - 0.5) * 1.1 + (0.5 if keystone else 0.0)
    value = 1.4 + 10.0 * (w ** 0.7) + jit
    if edge == 0:
        pu = rnd("vous", k, "u")
        if abs(frac - (0.25 + 0.5 * pu)) * arc_px < (1.5 if c.hi else 1.0) and \
                abs((r - R_IN) / (r_out - R_IN) - 0.55) * (r_out - R_IN) * c.s < (1.5 if c.hi else 1.0):
            value -= 1.1
    # cold rim on the intrados edge
    if d_in < (2.0 if c.hi else 1.0):
        cold = cold_light(x, y, z, (-radial[0] * 0.7, -radial[1] * 0.7, 0.7))
        if cold > 0.2 and cold > w:
            c.put(px, py, "stone_cold", 1.0 + 3.0 * cold)
            return
    c.put(px, py, "stone", value)


def soffit_pixel(c, px, py, x, y, r, theta) -> None:
    """Underside of the thick arch: receding brick courses, warm front, cold back."""
    # distance to the back aperture circle decides the depth fraction
    rb = math.hypot(x - ARCH_CX, y - BACK_CY)
    if rb < R_BACK:
        return  # inside the back aperture: gallery shows through
    # depth fraction: 0 at the front intrados, 1 at the back edge
    front_gap = R_IN - r
    back_gap = rb - R_BACK
    thick_px = (front_gap + back_gap) * c.s
    t = front_gap / max(1e-6, front_gap + back_gap)
    z = PILLAR_Z - t * 64.0
    n = norm(-(x - ARCH_CX), -(y - ARCH_SPRING), 0.0)  # faces the arch centre
    if back_gap * c.s < 1.0:  # shadowed back edge where the soffit meets the aperture
        c.put(px, py, "mortar", 0.6)
        return
    # courses across the depth, joints continuing the front voussoir divisions
    rows = 2 if not c.hi else 3
    row = min(rows - 1, int(t * rows))
    seam = any(abs(t - k / rows) * thick_px < 0.5 for k in range(1, rows))
    step = math.pi / 15
    pos = (math.pi - theta) / step + (0.5 if row % 2 else 0.0)
    joint = abs(pos - round(pos)) * R_IN * step * c.s < 0.55
    w = warm_light(x, y, z, n, wrap=0.3)
    cold = cold_light(x, y, z, n) * (0.5 + 0.9 * t)
    jit = (rnd("sof", row, round(pos)) - 0.5) * 0.8
    if joint or seam:
        c.put(px, py, "mortar", 0.4 + 3.0 * w + 1.2 * cold)
        return
    if cold > w * 1.2:
        c.put(px, py, "stone_cold", 0.2 + cold * 3.4 + jit - t * 0.6)
    else:
        c.put(px, py, "stone", 1.6 + 8.0 * (w ** 0.7) + jit - t * 1.2)


# ---------------------------------------------------------------------------
# Gallery (seen through the arch)
# ---------------------------------------------------------------------------

def layer_far_gallery(c: Canvas) -> None:
    x0, x1 = 196.0, 476.0
    for py in c.pyr(40, 272):
        y = c.dy(py)
        for px in c.pxr(x0, x1):
            x = c.dx(px)
            c.put(px, py, "gal", far_gallery_value(c, x, y))
    for tx in FAR_TORCH_XS:
        far_torch(c, tx, FAR_TORCH_Y, small=True)


def far_gallery_value(c: Canvas, x: float, y: float) -> float:
    # far floor with ground fog near the horizon
    if y >= 252:
        fog = clamp(1.0 - (y - 252) / 18.0, 0.0, 1.0)
        val = 3.6 + fog * 2.4
        for tx in FAR_TORCH_XS:  # faint reflection column under each torch
            if abs(x - tx) < 2.0 and y > 255:
                val += 1.4
        return val
    # lower arcade: piers every 44, openings 28 wide, arches spring at 200
    period, open_w = 44.0, 28.0
    u = (x - ARCH_CX + period / 2) % period
    centre = x - u + period / 2  # openings centred on the arch axis, piers between
    in_open = abs(x - centre) < open_w / 2
    spring, rad = 200.0, open_w / 2
    if 150 <= y < 252:
        if in_open and (y >= spring or (x - centre) ** 2 + (y - spring) ** 2 < rad ** 2):
            # deep opening: very dark, fog rising from the floor
            fog = clamp((y - 212) / 40.0, 0.0, 1.0)
            return 1.6 + fog * 3.4
        # pier face: hazy mid-blue, a lighter edge where it meets the openings
        edge = abs(abs(x - centre) - open_w / 2) < 1.2 / c.s * (1.5 if c.hi else 1.0)
        course = (round((y - 150) * c.s) % c.unit(8)) == 0
        val = 4.8 + (1.0 if edge else 0.0) - (0.7 if course else 0.0)
        return val
    # cornice line under the balcony level
    if 136 <= y < 150:
        return 5.6 if y < 139 else 4.4
    # upper level: tall arched windows with the coldest, brightest haze (farthest space)
    period2, open2 = 40.0, 30.0
    u2 = (x - 336.0 + period2 / 2) % period2
    c2 = x - u2 + period2 / 2
    spring2, rad2 = 92.0, open2 / 2
    if abs(x - c2) < open2 / 2 and (y >= spring2 or (x - c2) ** 2 + (y - spring2) ** 2 < rad2 ** 2):
        # receding inner arch: a second, smaller ring a step darker
        inner = abs(x - c2) < open2 / 2 - 5 and (y >= spring2 + 6 or
                                                  (x - c2) ** 2 + (y - spring2 - 6) ** 2 < (rad2 - 5) ** 2)
        haze = clamp((y - 80) / 56.0, 0.0, 1.0)
        return (9.0 + haze * 1.2) if inner else (7.4 + haze * 0.8)
    course = (round((y - 40) * c.s) % c.unit(10)) == 0
    return 5.4 - (0.8 if course else 0.0)


def far_torch(c: Canvas, tx: float, ty: float, small: bool) -> None:
    """Distant torch: hot core cluster, warm ring, faint warm halo on the cold wall."""
    s = c.s
    halo = 5.0 if small else 9.0
    for py in c.pyr(ty - halo - 1, ty + halo + 1):
        for px in c.pxr(tx - halo - 1, tx + halo + 1):
            d = math.hypot(c.dx(px) - tx, (c.dy(py) - ty) * 1.2)
            if d < 1.1 + (0.4 if c.hi else 0.0):
                c.put(px, py, "flame", 5)
            elif d < 2.0 + (0.4 if c.hi else 0.0):
                c.put(px, py, "flame", 3)
            elif d < 3.2:
                c.put(px, py, "gal_warm", 4)
            elif d < halo * 0.7:
                cur = c.get(px, py)
                c.put(px, py, "gal_warm", 1 + (1 if d < halo * 0.5 else 0))
            elif d < halo:
                cur = c.get(px, py)
                if cur and cur[0] == "gal" and cur[1] <= 4:
                    c.put(px, py, "gal_warm", 0)
    del s


def layer_mid_gallery(c: Canvas) -> None:
    pillars = [(198.0, 216.0), (284.0, 302.0), (370.0, 388.0), (456.0, 474.0)]
    bays = [(216.0, 284.0), (302.0, 370.0), (388.0, 456.0)]
    for py in c.pyr(40, 272):
        y = c.dy(py)
        for px in c.pxr(190, 482):
            x = c.dx(px)
            v = mid_gallery_value(c, x, y, pillars, bays)
            if v is not None:
                c.put(px, py, v[0], v[1])
    far_torch(c, MID_TORCH["x"], MID_TORCH["y"], small=False)
    sconce(c, MID_TORCH["x"], MID_TORCH["y"] + 3, scale=0.55, ramp_override="gal")


def mid_gallery_value(c, x, y, pillars, bays):
    gx = MID_TORCH["x"]
    warm = falloff(math.hypot(x - gx, (y - MID_TORCH["y"]) * 0.9), 46.0)
    # gallery floor strip
    if 258 <= y < 272:
        v = 2.2 + 0.8 * (y - 258) / 14.0
        if warm > 0.05:
            return ("gal_warm", 0.3 + warm * 4.0)
        return ("gal", v)
    for (a, b) in pillars:
        if a <= x < b and 60 <= y < 258:
            u = (x - a) / (b - a)
            upx = (x - a) * c.s
            course = (round((y - 60) * c.s) % c.unit(12)) == 0
            if warm > 0.16 and upx >= 1.0:
                return ("gal_warm", -0.4 + warm * 5.2 - (0.8 if course else 0.0) - u * 0.6)
            # rim on the edge facing the gallery centre, cold haze from beyond
            facing_centre = (a + b) / 2 < ARCH_CX
            rim_w = 2.0 if c.hi else 1.0
            on_rim = ((b - x) * c.s <= rim_w) if facing_centre else (upx < rim_w)
            if on_rim:
                return ("gal", 7.0)
            base = 2.0 + (0.5 if (u < 0.3) == facing_centre else 0.0)
            return ("gal", base - (0.8 if course else 0.0))
    # balcony slab with a lit top edge and dark underside
    if 142 <= y < 154:
        if y < 144:
            return ("gal", 7.4)
        if y >= 152:
            return ("gal", 1.4)
        return ("gal", 3.5 - (0.7 if (round((x - 190) * c.s) % c.unit(16)) == 0 else 0.0))
    # railing on the balcony: rail, balusters, newels (silhouettes against the haze)
    if 124 <= y < 142:
        if y < 126.5:
            return ("gal", 1.8 if y >= 125 else 3.6)
        if y >= 139:
            return ("gal", 1.6)
        pitch = c.unit(6)
        if round((x - 190) * c.s) % pitch == 0:
            return ("gal", 1.2)
        return None
    # arches under the balcony between pillars
    if 154 <= y < 258:
        for (a, b) in bays:
            if a <= x < b:
                cx, rad, spring = (a + b) / 2, (b - a) / 2, 196.0
                if y >= spring:
                    return None
                d = math.hypot(x - cx, y - spring)
                if d < rad:
                    return None
                if d < rad + 3:  # arch ring edge catching the far haze
                    return ("gal", 5.6 if d < rad + 1.5 else 4.0)
                return ("gal", 2.6 - (0.8 if (round((y - 154) * c.s) % c.unit(10)) == 0 else 0))
        return None
    return None


# ---------------------------------------------------------------------------
# Props: torches, copper pipework, chain
# ---------------------------------------------------------------------------

def layer_props(c: Canvas) -> None:
    for t in TORCHES:
        sconce(c, t["x"], t["y"] + 7, scale=1.15)
        flame(c, t["x"], t["y"] + 1, scale=1.3, seed=f"t{t['x']}")
    pipework(c)
    chain(c)


def sconce(c: Canvas, x: float, y: float, scale: float, ramp_override: str | None = None) -> None:
    """Iron cup torch on a wall plate: cup top at y, plate below."""
    cup_w, cup_h = 11.0 * scale, 7.0 * scale
    for py in c.pyr(y - 1, y + 24 * scale):
        yy = c.dy(py)
        for px in c.pxr(x - cup_w, x + cup_w):
            xx = c.dx(px)
            dx = xx - x
            t = (yy - y) / cup_h
            ramp = ramp_override or "iron"
            if 0 <= t < 1.0:  # flared cup
                half = cup_w / 2 * (1.0 - 0.45 * t)
                if abs(dx) < half:
                    if t < 0.22:  # rim, lit from the flame above
                        val = 6 if ramp == "iron" else 7
                    else:
                        nx = dx / max(half, 1e-6)
                        val = 3.0 + 2.2 * max(0.0, 0.6 - abs(nx + 0.2)) - t * 1.4
                        if ramp == "gal":
                            val = 2.0 + 2.0 * max(0.0, 0.6 - abs(nx))
                    c.put(px, py, ramp, val)
                continue
            if 1.0 <= t and yy < y + 24 * scale:  # stem and wall plate
                stem = abs(dx) < 1.6 * scale + (0.3 if c.hi else 0)
                plate = abs(dx) < 3.4 * scale and y + 13 * scale <= yy < y + 23 * scale
                if plate:
                    bolt = c.hi and abs(dx) < 1.2 and abs(yy - (y + 18 * scale)) < 1.2
                    val = 1.6 if bolt else (3.0 if dx < 0 else 2.2)
                    if yy < y + 14.5 * scale:
                        val = 4.4
                    c.put(px, py, ramp, val)
                elif stem:
                    c.put(px, py, ramp, 3.6 if dx < 0 else 2.4)


def flame(c: Canvas, x: float, y: float, scale: float, seed: str) -> None:
    """Teardrop flame with a hot core; base sits on the cup rim at y + 6."""
    h = 15.0 * scale
    base = y + 6.0
    for py in c.pyr(base - h - 1, base + 1):
        yy = c.dy(py)
        t = (base - yy) / h  # 0 at base, 1 at tip
        if not 0.0 <= t <= 1.0:
            continue
        lean = math.sin(t * 2.6 + rnd(seed) * 6.0) * 1.2 * t
        half = 4.4 * scale * (math.sin(math.pi * min(1.0, t * 1.15 + 0.12)) ** 0.8) * (1.0 - 0.55 * t)
        tongue = 1.0 if (c.hi and 0.55 < t < 0.8 and rnd(seed, py) > 0.5) else 0.0
        for px in c.pxr(x - 7 * scale, x + 7 * scale):
            dx = c.dx(px) - (x + lean)
            a = abs(dx)
            if a > half + tongue * 0.6:
                continue
            core = a / max(half, 0.5)
            if t < 0.38 and core < 0.45:
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


def cylinder_shade(c: Canvas, px: int, py: int, x: float, y: float, u: float, axis: str,
                   ramp: str, z: float, gloss: float = 1.0, base: float = 1.0,
                   gain: float = 7.0) -> None:
    """Shade a metal cylinder pixel. u in [-1, 1] across the diameter."""
    u = clamp(u, -0.999, 0.999)
    a = math.asin(u)
    if axis == "v":
        n = (math.sin(a), 0.0, math.cos(a))
    else:
        n = (0.0, math.sin(a), math.cos(a))
    w = warm_light(x, y, z, n, wrap=0.15, radius=TORCH_RADIUS, power=1.6)
    spec = 0.0
    for t in TORCHES:
        lv = norm(t["x"] - x, t["y"] - y, t["z"] - z)
        hv = norm(lv[0], lv[1], lv[2] + 1.0)
        nh = max(0.0, n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2])
        dist = math.dist((t["x"], t["y"]), (x, y))
        spec += (nh ** 30) * falloff(dist, TORCH_RADIUS) * 7.0
    value = base + gain * (w ** 0.75) + gloss * spec
    edge = abs(u) > 0.86
    if edge and w < 0.25 and ramp == "copper":
        c.put(px, py, "copper_cold", 0.6 + abs(u))
        return
    c.put(px, py, ramp, value - (0.6 if edge else 0.0))


def pipework(c: Canvas) -> None:
    x0, x1 = PIPE_X
    cx = (x0 + x1) / 2
    rad = (x1 - x0) / 2
    y0, y1 = PIPE_RUN_Y
    cy_run = (y0 + y1) / 2
    elbow_c = (x1 + 6.0, y1 + 6.0)  # centre of the bend (inner radius 6)
    z = 10.0
    # horizontal run to the right edge
    for py in c.pyr(y0, y1):
        y = c.dy(py)
        for px in c.pxr(elbow_c[0], 680):
            x = c.dx(px)
            cylinder_shade(c, px, py, x, y, (y - cy_run) / rad, "h", "copper", z)
    # elbow: quarter torus, inner radius 6, outer radius 6 + 12
    rin, rout = 6.0, 6.0 + 2 * rad
    for py in c.pyr(y0, elbow_c[1] + 0.5):
        y = c.dy(py)
        for px in c.pxr(x0, elbow_c[0] + 0.5):
            x = c.dx(px)
            dx, dy = x - elbow_c[0], y - elbow_c[1]
            if dx > 0 or dy > 0:
                continue
            d = math.hypot(dx, dy)
            if rin <= d < rout:
                u = (d - (rin + rout) / 2) / rad
                ang = math.atan2(-dy, -dx)  # 0 = horizontal-left, pi/2 = up
                # blend normal orientation between vertical and horizontal pipe
                nx = math.cos(ang) * math.sin(math.asin(clamp(u, -0.999, 0.999)))
                ny = math.sin(ang) * math.sin(math.asin(clamp(u, -0.999, 0.999)))
                uu = clamp(-nx if abs(nx) > abs(ny) else -ny, -0.999, 0.999)
                cylinder_shade(c, px, py, x, y, uu if abs(nx) > abs(ny) else uu,
                               "v" if abs(nx) > abs(ny) else "h", "copper", z)
    # vertical drop
    for py in c.pyr(elbow_c[1], PIPE_BOTTOM):
        y = c.dy(py)
        for px in c.pxr(x0, x1):
            x = c.dx(px)
            cylinder_shade(c, px, py, x, y, (x - cx) / rad, "v", "copper", z)
    # weld seams on the vertical drop (thin darker rings)
    for sy in (126.0, 236.0):
        for py in c.pyr(sy, sy + 1.0):
            for px in c.pxr(x0, x1):
                cur = c.get(px, py)
                if cur and cur[0] == "copper":
                    c.put(px, py, "copper", cur[1] - 2)
    # brass flanges (wider rings with bolts)
    for fy in (88.0, 210.0):
        flange(c, cx, fy, rad + 3.0, 5.0, "v", z + 1)
    flange(c, elbow_c[0] + 30.0, cy_run, rad + 3.0, 5.0, "h", z + 1)
    # floor flange where the pipe enters the floor plane
    flange(c, cx, PIPE_BOTTOM - 4.0, rad + 5.0, 6.0, "v", z + 1)
    # iron wall brackets
    for by in (62.0, 246.0):
        bracket(c, cx, by, rad + 2.0, z + 0.5)
    valve(c, VALVE[0], VALVE[1], z + 2)


def flange(c, cx, cy, half, thick, axis, z) -> None:
    if axis == "v":
        xr, yr = (cx - half, cx + half), (cy - thick / 2, cy + thick / 2)
    else:
        xr, yr = (cx - thick / 2, cx + thick / 2), (cy - half, cy + half)
    for py in c.pyr(*yr):
        y = c.dy(py)
        for px in c.pxr(*xr):
            x = c.dx(px)
            if axis == "v":
                u, along = (x - cx) / half, (y - cy) / (thick / 2)
            else:
                u, along = (y - cy) / half, (x - cx) / (thick / 2)
            cylinder_shade(c, px, py, x, y, u, axis, "brass", z, gloss=1.2, base=1.0, gain=5.4)
            if abs(along) > 0.7:  # chamfered rim edges
                cur = c.get(px, py)
                c.put(px, py, "brass", cur[1] - 1)
            bolt_u = (-0.62, 0.0, 0.62) if c.hi else (-0.55, 0.55)
            for bu in bolt_u:
                if abs(u - bu) * half * c.s < (1.0 if c.hi else 0.6) and abs(along) < 0.45:
                    c.put(px, py, "iron", 2)


def bracket(c, cx, cy, half, z) -> None:
    for py in c.pyr(cy - 2.0, cy + 2.0):
        y = c.dy(py)
        for px in c.pxr(cx - half - 4.0, cx + half + 4.0):
            x = c.dx(px)
            w = warm_light(x, y, z, (0, -0.3 if y < cy else 0.3, 1.0))
            if abs(x - cx) > half:  # wall lugs
                bolt = abs(abs(x - cx) - half - 2.0) < 0.9 and c.hi and abs(y - cy) < 0.9
                c.put(px, py, "iron", 1.0 if bolt else 1.6 + 4.0 * w)
            else:
                c.put(px, py, "iron", 2.0 + 5.0 * w + (0.7 if y < cy - 1.0 else 0.0))


def valve(c, vx, vy, z) -> None:
    # valve body: a bulge on the pipe
    body_h, body_w = 16.0, 6.0
    for py in c.pyr(vy - body_h / 2, vy + body_h / 2):
        y = c.dy(py)
        tt = (y - vy) / (body_h / 2)
        half = body_w * math.sqrt(max(0.0, 1.0 - tt * tt * 0.6))
        for px in c.pxr(vx - body_w, vx + body_w):
            x = c.dx(px)
            if abs(x - vx) < half:
                cylinder_shade(c, px, py, x, y, (x - vx) / half, "v", "copper", z, gloss=0.8,
                               base=0.0, gain=4.6)
    # hand wheel facing the viewer: rim, four spokes, hub
    r_out, r_in = 13.0, 10.4 if not c.hi else 11.0
    for py in c.pyr(vy - r_out - 1, vy + r_out + 1):
        y = c.dy(py)
        for px in c.pxr(vx - r_out - 1, vx + r_out + 1):
            x = c.dx(px)
            dx, dy = x - vx, y - vy
            d = math.hypot(dx, dy)
            spoke_w = 1.0 if not c.hi else 0.85
            on_spoke = d < r_in and (abs(dx - dy) < spoke_w * 1.2 or abs(dx + dy) < spoke_w * 1.2)
            hub = d < (2.6 if not c.hi else 3.0)
            if r_in <= d < r_out:
                n = norm(dx / d * 0.8, dy / d * 0.8, 0.6) if d > (r_in + r_out) / 2 else \
                    norm(-dx / d * 0.6, -dy / d * 0.6, 0.8)
                w = warm_light(x, y, z + 4, n, wrap=0.1)
                c.put(px, py, "brass", 1.8 + 6.0 * (w ** 0.6))
            elif hub:
                w = warm_light(x, y, z + 5, norm(-dx * 0.2, -dy * 0.2, 1.0))
                c.put(px, py, "iron" if d < 1.2 else "brass", (3.0 if d < 1.2 else 1.4 + 5.0 * w))
            elif on_spoke:
                w = warm_light(x, y, z + 4, (0, -0.4, 0.9))
                c.put(px, py, "brass", 1.4 + 5.0 * (w ** 0.6))


def chain(c: Canvas) -> None:
    (ax, ay), (bx, by), sag = CHAIN
    link = 4.0 if not c.hi else 4.0
    length = bx - ax
    steps = int(length * c.s * 2)
    seen: set[tuple[int, int]] = set()
    for i in range(steps + 1):
        t = i / steps
        x = ax + length * t
        y = ay + (by - ay) * t + sag * 4 * t * (1 - t)
        along = (x - ax) / link
        k = int(along)
        open_link = k % 2 == 0
        px, py = math.floor((x - c.ox) * c.s), math.floor((y - c.oy) * c.s)
        w = warm_light(x, y, PILLAR_Z - 10, (0, -0.5, 0.8))
        top = 1.8 + 6.0 * w
        if open_link:
            # oval link seen from the side: two rows, hollow centre at higher density
            frac = along - k
            hollow = c.hi and 0.3 < frac < 0.7
            if (px, py - 1) not in seen:
                c.put(px, py - 1, "iron", top)
                seen.add((px, py - 1))
            if not hollow:
                c.put(px, py, "iron", top - 1.6)
            c.put(px, py + 1, "iron", 1.0)
        else:
            c.put(px, py, "iron", top - 0.8)
    # hooks at the soffit
    for hx, hy in ((ax, ay), (bx, by)):
        for py in c.pyr(hy - 4, hy + 1):
            for px in c.pxr(hx - 1, hx + 1):
                c.put(px, py, "iron", 2.5)


# ---------------------------------------------------------------------------
# Terrain: walkable floor plane seen slightly from above + front ledge
# ---------------------------------------------------------------------------

FLOOR_ROWS = [268.0, 272.0, 277.0, 284.0, 292.0]


FLOOR_LIGHT_RADIUS = 290.0  # torches sit high above the floor: wider, softer pools
FLOOR_LIGHT_POWER = 1.75


def floor_world(x: float, y: float) -> tuple[float, float, float]:
    """Screen point on the floor plane -> (world x, world height y, depth z)."""
    return x, float(FLOOR_BACK), (y - FLOOR_BACK) * DEPTH_PER_ROW


def layer_terrain(c: Canvas) -> None:
    lip_h = 2.0
    # threshold: the floor receding into the gallery between the jambs; it fades
    # into the cold haze with distance and catches a little warm light near the front
    for py in c.pyr(252, FLOOR_BACK):
        y = c.dy(py)
        t = (FLOOR_BACK - y) / 16.0  # 0 at the wall line, 1 deep in the gallery
        seam = (round((FLOOR_BACK - y) * c.s) % c.unit(5)) == 0
        for px in c.pxr(250, 422):
            x = c.dx(px)
            w = warm_light(x, FLOOR_BACK, -(FLOOR_BACK - y) * 4.0, (0, -1, 0), wrap=0.1) * (1 - t)
            if w > 0.2:
                c.put(px, py, "floor", 0.8 + 7.0 * w - (0.8 if seam else 0))
            else:
                c.put(px, py, "floor_cold", 0.6 + t * 2.2 - (0.8 if seam else 0))
    for py in c.pyr(FLOOR_BACK, 381):
        y = c.dy(py)
        for px in range(c.w):
            x = c.dx(px)
            if y < FLOOR_LIP:
                floor_plane_pixel(c, px, py, x, y)
            elif y < FLOOR_LIP + lip_h:
                lip_pixel(c, px, py, x, y)
            else:
                ledge_pixel(c, px, py, x, y)


def floor_plane_pixel(c: Canvas, px, py, x, y) -> None:
    row = 0
    while row < len(FLOOR_ROWS) - 2 and y >= FLOOR_ROWS[row + 1]:
        row += 1
    ry0, ry1 = FLOOR_ROWS[row], FLOOR_ROWS[row + 1]
    # slab joints converge slightly toward a vanishing point above the scene
    vy = -200.0
    k = (y - vy) / (FLOOR_LIP - vy)
    slab = 32.0
    xf = VP[0] + (x - VP[0]) / k  # x projected onto the lip line
    stagger = slab / 2 if row % 2 else 0.0
    col = math.floor((xf + stagger) / slab)
    u = ((xf + stagger) % slab) * k * c.s
    v = (y - ry0) * c.s
    v_h = (ry1 - ry0) * c.s
    wxz = floor_world(x, y)
    n = (0.0, -1.0, 0.0)
    # slab centre for flat per-slab light
    scx = (col * slab - stagger + slab / 2 - VP[0]) * ((ry0 + ry1) / 2 - vy) / (FLOOR_LIP - vy) + VP[0]
    scy = (ry0 + ry1) / 2
    wf = warm_light(scx, FLOOR_BACK, (scy - FLOOR_BACK) * DEPTH_PER_ROW, n, wrap=0.1,
                    radius=FLOOR_LIGHT_RADIUS, power=FLOOR_LIGHT_POWER)
    w = warm_light(wxz[0], wxz[1], wxz[2], n, wrap=0.1,
                   radius=FLOOR_LIGHT_RADIUS, power=FLOOR_LIGHT_POWER)
    w = 0.65 * wf + 0.35 * w
    jit = (rnd("slab", row, col) - 0.5) * 1.0
    ao = 0.0
    if row == 0:
        ao += 1.2 - (y - ry0) * 0.25  # wall-base occlusion
    for (ax, half) in ACTOR_MARKS:
        dx = (x - ax) / half
        dy = (y - (WALK_Y + 0.5)) / 2.6
        if dx * dx + dy * dy < 1.0:
            ao += 3.0 if dx * dx + dy * dy < 0.45 else 1.8
    for pcx in (218.0, 454.0):  # pillar bases occlude the floor around them
        if abs(x - pcx) < 34 and y < 283:
            ao += 1.2 * (1.0 - abs(x - pcx) / 34)
    cold = cold_light(x, FLOOR_BACK, wxz[2], (0, -0.6, 0.8)) * (1.0 - (y - FLOOR_BACK) / 30)
    if u < 1.0 or v < 1.0:  # groove
        c.put(px, py, "mortar", 0.6 + 5.4 * w - ao * 0.5)
        return
    value = 1.2 + 9.4 * (w ** 0.8) + jit - ao
    if v < 2.0 or (c.hi and v < 3.0 and row == 3):
        value += 0.9  # lit back edge of each slab
    if not c.hi and row == 3 and v >= v_h - 1:
        value -= 0.4
    # worn texture clusters
    pu = rnd("slab", row, col, "u")
    if abs(u / max(1.0, k * c.s) - (6 + pu * 18)) < (1.6 if c.hi else 1.0) and \
            abs(v - v_h * 0.55) < (1.0 if c.hi else 0.6):
        value -= 1.0
    if cold > 0.25 and cold > w * 1.4:
        c.put(px, py, "floor_cold", cold * 3.2 - 0.4 - ao * 0.4 + jit * 0.4)
        return
    c.put(px, py, "floor", value)


def lip_pixel(c: Canvas, px, py, x, y) -> None:
    w = warm_light(x, FLOOR_BACK, (FLOOR_LIP - FLOOR_BACK) * DEPTH_PER_ROW, (0, -0.7, 0.7), wrap=0.3)
    first = y < FLOOR_LIP + 1.0
    c.put(px, py, "floor", (2.6 if first else 1.6) + 9.0 * (w ** 0.8))


def ledge_pixel(c: Canvas, px, py, x, y) -> None:
    """Front face of the floor ledge: big blocks lit from above, fading to cold dark."""
    top = FLOOR_LIP + 2.0
    rows = [top, top + 22.0, top + 44.0, top + 66.0, 400.0]
    row = 0
    while y >= rows[row + 1]:
        row += 1
    by0 = rows[row]
    bw = 40.0
    stagger = 20.0 if row % 2 else 0.0
    col = math.floor((x + stagger) / bw)
    u = ((x + stagger) % bw) * c.s
    v = (y - by0) * c.s
    # light spilling over the lip from the torches, measured per block
    bcx = col * bw - stagger + bw / 2
    w_top = warm_light(bcx, FLOOR_BACK - 10, 70.0, (0, -0.3, 1.0), wrap=0.5)
    row_gain = (1.0, 0.42, 0.14, 0.0)[row]
    jit = (rnd("ledge", row, col) - 0.5) * 0.9
    if u < 1.0 or v < 1.0:
        c.put(px, py, "mortar", 0.2 + 4.0 * (w_top ** 0.7) * row_gain)
        return
    if row_gain <= 0.15:
        c.put(px, py, "floor_cold", 0.9 - row * 0.3 + jit * 0.4 + (0.6 if v < 2 else 0.0)
              + 2.0 * w_top * row_gain)
        return
    vh = 22.0 * c.s
    grad = 1.0 - v / vh  # brighter just under the lip / the joint above
    value = 0.8 + row_gain * (2.0 + 8.0 * (w_top ** 0.7)) + jit + grad * 1.2 * row_gain
    if v < (2.0 if c.hi else 1.0):
        value += 1.0 * row_gain
    c.put(px, py, "floor", value)


# ---------------------------------------------------------------------------
# Foreground: restrained low rubble silhouettes in front of the ledge corners
# ---------------------------------------------------------------------------

RUBBLE = [  # (cx, base_y, half width, height, seed)
    (14.0, 380.0, 30.0, 26.0, "r1"), (52.0, 380.0, 18.0, 15.0, "r2"),
    (612.0, 380.0, 36.0, 34.0, "r3"), (574.0, 380.0, 16.0, 13.0, "r4"),
]


def layer_foreground(c: Canvas) -> None:
    for (cx, base, half, height, seed) in RUBBLE:
        for py in c.pyr(base - height - 2, base):
            y = c.dy(py)
            for px in c.pxr(cx - half - 2, cx + half + 2):
                x = c.dx(px)
                t = (x - cx) / half
                if abs(t) >= 1.0:
                    continue
                facets = 3 + int(rnd(seed) * 3)
                fi = int((t + 1) / 2 * facets)
                bump = 0.75 + 0.25 * rnd(seed, fi)
                top_y = base - height * bump * math.sqrt(1 - t * t)
                if y < top_y:
                    continue
                rim = y < top_y + (2.0 if c.hi else 1.0) * (1 / c.s) * 1.5
                if rim:
                    w = warm_light(x, y - 60, 40.0, (0, -1, 0.3), wrap=0.4)
                    c.put(px, py, "brick", 1.0 + 6.0 * w)
                else:
                    c.put(px, py, "brick", 0.0 if (fi % 2 or y > top_y + 6) else 1.0)


# ---------------------------------------------------------------------------
# Export
# ---------------------------------------------------------------------------

LAYER_FUNCS = {
    "far_gallery": layer_far_gallery,
    "mid_gallery": layer_mid_gallery,
    "backwall": layer_backwall,
    "terrain": layer_terrain,
    "structure": layer_structure,
    "props": layer_props,
    "foreground": layer_foreground,
}


def to_grid(c: Canvas) -> tuple[dict, list[int]]:
    """Crop to the opaque bounding box and encode as pixel-grid-v1."""
    ys = [y for y in range(c.h) if any(p is not None for p in c.px[y])]
    xs = [x for y in ys for x in range(c.w) if c.px[y][x] is not None]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    used = sorted({p for row in c.px for p in row if p is not None},
                  key=lambda p: (list(RAMPS).index(p[0]), p[1]))
    if len(used) > len(KEY_ALPHABET):
        raise SystemExit(f"{c.density}/{c.name}: {len(used)} colours exceed the key alphabet")
    keys = {p: KEY_ALPHABET[i] for i, p in enumerate(used)}
    palette = [{"key": keys[p], "name": f"{p[0]}_{p[1]:02d}", "rgba": RAMPS[p[0]][p[1]] + "ff"}
               for p in used]
    rows = ["".join(keys[p] if p is not None else "." for p in c.px[y][x0:x1 + 1])
            for y in range(y0, y1 + 1)]
    grid = {
        "format": "pixel-grid-v1",
        "canvas": [x1 - x0 + 1, y1 - y0 + 1],
        "pivot": [0, 0],
        "palette": palette,
        "layers": [c.name],
        "frames": [{"duration_ms": 100, "cels": {c.name: rows}}],
        "tags": [],
    }
    return grid, [x0, y0]


def write_grid(path: Path, grid: dict) -> None:
    # one row per line keeps diffs readable
    head = {k: v for k, v in grid.items() if k != "frames"}
    lines = ["{"]
    for k, v in head.items():
        lines.append(f"  {json.dumps(k)}: {json.dumps(v, ensure_ascii=False)},")
    frame = grid["frames"][0]
    layer = grid["layers"][0]
    lines.append('  "frames": [{"duration_ms": %d, "cels": {%s: [' % (frame["duration_ms"], json.dumps(layer)))
    rows = frame["cels"][layer]
    for i, row in enumerate(rows):
        lines.append("    " + json.dumps(row, ensure_ascii=False) + ("," if i < len(rows) - 1 else ""))
    lines.append("  ]}}]")
    lines.append("}")
    text = "\n".join(lines) + "\n"
    # move "tags" before frames for valid ordering independence (JSON is unordered)
    path.write_text(text.replace(',\n  "frames"', ',\n  "frames"'), encoding="utf-8")


def build(no_ase: bool) -> dict:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    scene = {
        "format": "lookdev-scene-v1",
        "note": ("LOOK-DEV ONLY: Floor 1 resolution and lighting spike. Not production art, "
                 "not runtime data. Layer offsets are in canvas pixels of each density."),
        "design_units": "640x360 virtual pixels; b960 renders 1 design unit as 1.5 px",
        "provenance": ("Authored by Claude Opus with scripts/lookdev_floor01_author.py; "
                       "concept 01-catacomb-environment-clean.png used for mood only, never sampled."),
        "walk_line_design_y": WALK_Y,
        "densities": {},
    }
    for dname, cfg in DENSITIES.items():
        entry = {"density": cfg["s"], "canvas": list(cfg["canvas"]),
                 "design_origin": list(cfg["origin"]), "views": cfg["views"], "layers": []}
        for order, (lname, parallax, group) in enumerate(LAYERS):
            canvas = Canvas(lname, dname)
            LAYER_FUNCS[lname](canvas)
            grid, offset = to_grid(canvas)
            gpath = OUT_DIR / f"{dname}_{lname}.grid.json"
            write_grid(gpath, grid)
            json.loads(gpath.read_text(encoding="utf-8"))  # sanity
            if not no_ase:
                subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"),
                                "build", str(gpath), str(gpath.with_suffix("").with_suffix(".ase"))],
                               check=True, stdout=subprocess.DEVNULL)
            entry["layers"].append({
                "name": lname, "grid": gpath.name,
                "ase": gpath.with_suffix("").with_suffix(".ase").name,
                "offset": offset, "parallax": parallax, "light_group": group,
                "colours": len(grid["palette"]),
            })
            print(f"{dname}/{lname}: {grid['canvas'][0]}x{grid['canvas'][1]} at {offset}, "
                  f"{len(grid['palette'])} colours")
        entry["actor_insert_index"] = ACTOR_LAYER_INDEX
        scene["densities"][dname] = entry
    SCENE_FILE.write_text(json.dumps(scene, indent=2) + "\n", encoding="utf-8")
    return scene


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--no-ase", action="store_true", help="skip building .ase files")
    args = parser.parse_args(argv)
    build(args.no_ase)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
