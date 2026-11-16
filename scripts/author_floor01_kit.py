#!/usr/bin/env python3
"""Authoring record for the ENV-01 Floor 1 catacomb environment kit (pixel-grid-v1).

This script is the deterministic *authoring aid* that wrote the first version of
the kit's text pixel grids, the kit contract and the sample corridor layout:

- ``assets/source/terrain/floor01/terrain.grid.json``   collidable 16x16 tiles
- ``assets/source/terrain/floor01/backwall.grid.json``  non-collidable backwall tiles
- ``assets/source/terrain/floor01/structure.grid.json`` arch, pier modules, far arcade
- ``assets/source/props/floor01/props.grid.json``       pipes, valve, box, lichen, torch glow
- ``assets/source/props/floor01/torch.grid.json``       4-frame wall torch loop
- ``assets/source/terrain/floor01/floor01_catacomb_kit.json`` integration contract
- ``assets/levels/floor01/corridor_abc_sample.json``    sample layout (A -> B -> C)

The committed grids and the ``.ase`` files built from them are the editable
source of truth. Artists may hand-edit the grids afterwards; re-running this
script overwrites such edits, so only re-run it deliberately (``--force``).

Every pixel is placed by explicit rules or hand-drawn strings in this file:
brick coursing with staggered joints, per-brick tone/bevel/chip variation from a
fixed seed, mortar, stone edge treatment, voussoirs on a true circle, and
hand-drawn flame, valve, box and lichen shapes. Nothing is cropped, traced or
resampled from concept images.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TERRAIN_DIR = ROOT / "assets/source/terrain/floor01"
PROPS_DIR = ROOT / "assets/source/props/floor01"
LEVEL_DIR = ROOT / "assets/levels/floor01"
CONTRACT_PATH = TERRAIN_DIR / "floor01_catacomb_kit.json"
LAYOUT_PATH = LEVEL_DIR / "corridor_abc_sample.json"

KIT_ID = "floor01_catacomb"
PALETTE_ID = "floor01-catacomb-v1"
TILE = 16

# ---------------------------------------------------------------------------
# Shared palette (one palette for the whole kit, 47 opaque colours).
# Values were set by CIE L* (revision 1): brick faces sit at L* 27-33, clearly
# above the darkest actor masses (Carl's jacket and hair are L* ~17.7), and
# mortar is only one step below the faces so the wall stays low-contrast.
# ---------------------------------------------------------------------------
PALETTE = [
    # Muted, low-saturation red-brown brick (backwall fabric, module A)
    ("m", "brick_mortar", "#413433"),        # L* 23
    ("a", "brick_0_shadow", "#4b3836"),      # L* 25.5
    ("b", "brick_1_dark", "#513c39"),        # L* 27.5
    ("c", "brick_2_base", "#59413d"),        # L* 30
    ("d", "brick_3_light", "#604741"),       # L* 32.5
    ("e", "brick_4_highlight", "#684d46"),   # L* 35.5
    # Restrained warm torchlight on brick
    ("f", "torchlit_0", "#7c5545"),          # L* 40
    ("g", "torchlit_1", "#9b6547"),          # L* 48
    ("h", "torchlit_2", "#b87b4e"),          # L* 57
    # Stone edges (floor, ceiling, arches, piers): neutral warmed toward the
    # brick hue but far less saturated, so stone and brick read as one family
    ("s", "stone_0_joint", "#241e20"),       # L* 12
    ("t", "stone_1_shadow", "#312b2b"),      # L* 18
    ("u", "stone_2_dark", "#413a38"),        # L* 25
    ("v", "stone_3_base", "#544c49"),        # L* 33
    ("w", "stone_4_light", "#685f5a"),       # L* 41
    ("x", "stone_5_edge", "#7d726c"),        # L* 49
    ("y", "stone_6_lip", "#9e9185"),         # L* 61
    ("z", "stone_7_glint", "#c1b19f"),       # L* 73
    # Dry floor walking surface: same low chroma as the pier stone (reads gray)
    ("D", "floor_surface_0", "#4d4542"),     # L* 30, C* ~4
    ("E", "floor_surface_1", "#5a514d"),     # L* 35, C* ~5
    ("F", "floor_surface_2", "#695f59"),     # L* 41, C* ~6
    # Cold blue depth (module B recesses); actor-height haze is 3-4
    ("0", "depth_0_void", "#0c1527"),        # L* 7
    ("1", "depth_1_far_wall", "#0f1e33"),    # L* 11
    ("2", "depth_2_far_air", "#152843"),     # L* 16
    ("3", "depth_3_haze", "#1a3451"),        # L* 21
    ("4", "depth_4_far_edge", "#223f5f"),    # L* 26
    ("5", "depth_5_far_rim", "#2f4d6e"),     # L* 32
    # Torch flame
    ("R", "flame_red", "#a9341f"),
    ("O", "flame_orange", "#e2722b"),
    ("Y", "flame_yellow", "#ffb648"),
    ("W", "flame_core", "#fff0c0"),
    # Iron (torch sconce, pipes, box; module C)
    ("k", "iron_0_outline", "#16151b"),
    ("i", "iron_1_shadow", "#272731"),
    ("j", "iron_2_base", "#393a46"),
    ("l", "iron_3_light", "#50525f"),
    ("n", "iron_4_highlight", "#737582"),
    ("o", "iron_5_glint", "#a3a4ad"),
    # Rust, valve paint, label plate
    ("q", "rust_0", "#57301f"),
    ("r", "rust_1", "#7d4428"),
    ("p", "valve_paint_shadow", "#6a2120"),
    ("P", "valve_paint", "#9a3128"),
    ("T", "plate_ochre", "#8c6c32"),
    # Old timber (simple door)
    ("U", "wood_0", "#3c271b"),              # L* 18
    ("V", "wood_1", "#533726"),              # L* 26
    ("X", "wood_2", "#6a4931"),              # L* 34
    # Optional lichen accent
    ("G", "lichen_0", "#283326"),           # L* 20, a* -8
    ("H", "lichen_1", "#354130"),           # L* 26, a* -9
    ("L", "lichen_2_glow", "#5c7444"),      # L* 46, accent
]
ACCENT_KEYS = {"z", "o", "L"}


def palette_entries() -> list[dict]:
    out = []
    for key, name, hexrgb in PALETTE:
        entry = {"key": key, "name": name, "rgba": hexrgb + "ff"}
        if key in ACCENT_KEYS:
            entry["accent"] = True
        out.append(entry)
    return out


# ---------------------------------------------------------------------------
# Canvas helper
# ---------------------------------------------------------------------------
class Canvas:
    def __init__(self, w: int, h: int) -> None:
        self.w, self.h = w, h
        self.px = [["."] * w for _ in range(h)]

    def put(self, x: int, y: int, key: str) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = key

    def get(self, x: int, y: int) -> str:
        return self.px[y][x]

    def fill(self, x0: int, y0: int, x1: int, y1: int, key: str) -> None:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, key)

    def stamp(self, rows: list[str], ox: int, oy: int) -> None:
        width = len(rows[0])
        for dy, row in enumerate(rows):
            if len(row) != width:
                raise ValueError(f"ragged stamp row {dy}: {row!r}")
            for dx, ch in enumerate(row):
                if ch != ".":
                    self.put(ox + dx, oy + dy, ch)

    def rows(self) -> list[str]:
        return ["".join(r) for r in self.px]


def stable_hash(*parts) -> int:
    text = ":".join(str(p) for p in parts)
    return int.from_bytes(hashlib.sha256(text.encode()).digest()[:8], "big")


# ---------------------------------------------------------------------------
# Brick coursing (16x8 modules, running bond, light from upper left)
# ---------------------------------------------------------------------------
BRICK_RAMP = "mabcde"  # 0 = mortar, 1..5 = face ramp dark -> light


def ramp(index: int) -> str:
    return BRICK_RAMP[max(1, min(5, index))]


def draw_brick(c: Canvas, ox: int, oy: int, start: int, y0: int, width: int,
               tone: int, rng: random.Random | None, shift: int = 0,
               mortar: str = "m", full_bevel: bool = False) -> None:
    """One 15x7 brick face; ``rng=None`` draws the fixed seam brick."""
    if rng is None:
        hl0, hl_len, sh0, sh_len, chips, pit, crack, rshade = 0, 9, 9, 6, [], None, None, False
    else:
        # Bevel runs start at varied positions so the top-left highlight and
        # bottom-right shadow never line up into diagonals across courses.
        hl0 = 0 if full_bevel else rng.choice((0, 0, 1, 2, 4, 6))
        hl_len = 15 if full_bevel else rng.randint(3, 9)
        sh0 = rng.randint(0, 9)
        sh_len = rng.randint(3, 15 - sh0)
        roll = rng.random()
        corners = [(0, 0), (14, 0), (0, 6), (14, 6)]
        if roll < 0.28:
            chips = [rng.choice(corners)]
        elif roll < 0.38:
            chips = rng.sample(corners, 2)
        else:
            chips = []
        pit = (rng.randint(2, 11), rng.randint(2, 4)) if rng.random() < 0.3 else None
        crack = rng.randint(3, 10) if rng.random() < 0.07 else None
        rshade = full_bevel or rng.random() < 0.3
    for dy in range(7):
        for dx in range(15):
            t = tone
            if dy == 0 and hl0 <= dx < hl0 + hl_len:
                t = tone + 1
            elif full_bevel and dx == 0 and dy < 6:
                t = tone + 1
            elif dy == 6 and sh0 <= dx < sh0 + sh_len:
                t = tone - 1
            elif dx == 14 and rshade and dy > 0:
                t = tone - 1
            c.put(ox + (start + dx) % width, oy + y0 + dy, ramp(t + shift))
    for cx, cy in chips:
        c.put(ox + (start + cx) % width, oy + y0 + cy, mortar)
    if pit:
        px, py = pit
        c.put(ox + (start + px) % width, oy + y0 + py, ramp(tone - 1 + shift))
        c.put(ox + (start + px + 1) % width, oy + y0 + py, ramp(tone - 1 + shift))
        c.put(ox + (start + px + 1) % width, oy + y0 + py + 1, ramp(tone + 1 + shift))
    if crack is not None:
        for i, (dx, dy) in enumerate(((0, 0), (0, 1), (1, 2), (1, 3), (2, 4))):
            if i < 4 or tone > 2:
                c.put(ox + (start + crack + dx) % width, oy + y0 + dy, ramp(tone - 2 + shift))


def brick_strip(c: Canvas, ox: int, oy: int, width: int, courses: int, seed: str,
                shift_per_course: list[int] | None = None, mortar: str = "m",
                tones=(2, 3, 4), weights=(3, 4, 3), full_bevel: bool = False) -> None:
    """Toroidal running-bond strip: tiles seamlessly with itself and with any
    other strip of the same width because the wrap-crossing (seam) brick of
    every odd course is a fixed, featureless base-tone brick."""
    rng = random.Random(f"{KIT_ID}:{seed}")
    c.fill(ox, oy, ox + width - 1, oy + courses * 8 - 1, mortar)
    for k in range(courses):
        shift = shift_per_course[k] if shift_per_course else 0
        offset = 0 if k % 2 == 0 else 8
        for i in range(width // 16):
            start = offset + 16 * i
            if start + 14 >= width:  # crosses the wrap seam
                draw_brick(c, ox, oy, start, 8 * k, width, 3, None, shift, mortar)
            else:
                tone = rng.choices(tones, weights)[0]
                draw_brick(c, ox, oy, start, 8 * k, width, tone, rng, shift, mortar,
                           full_bevel)


# ---------------------------------------------------------------------------
# Collidable terrain tiles (opaque)
# ---------------------------------------------------------------------------

def chip(c: Canvas, x: int, y: int, joint: str, light: str, dx: int = 1) -> None:
    """A 3 px chipped block corner: two joint-coloured pixels on the top edge
    and one lit pixel tucked under them (no isolated single-pixel pocks)."""
    c.put(x, y, joint)
    c.put(x + dx, y, joint)
    c.put(x, y + 1, light)


def floor_top(c: Canvas, ox: int, oy: int, joints: list[int], seed: str,
              end: str | None = None, chip_at: int | None = None) -> None:
    """Walkable floor: 5 px warm torch-lit walking surface (rows 0-4), a bright
    neutral stone lip (row 5), a shadow line and a warm front face."""
    rng = random.Random(f"{KIT_ID}:{seed}")
    surface = ["F", "E", "E", "D", "D", "y", "s"]
    for y, key in enumerate(surface):
        c.fill(ox, oy + y, ox + 15, oy + y, key)
    c.fill(ox, oy + 7, ox + 15, oy + 14, "D")
    c.fill(ox, oy + 7, ox + 15, oy + 7, "E")
    c.fill(ox, oy + 15, ox + 15, oy + 15, "s")
    # one short wear streak on the walking surface (never the back edge or lip)
    sx, sy = rng.randint(2, 11), rng.choice((2, 3))
    c.put(ox + sx, oy + sy, "D")
    c.put(ox + sx + 1, oy + sy, "D")
    for jx in joints:
        for y in (1, 2, 3, 4):
            c.put(ox + jx, oy + y, "D" if y < 3 else "v")
        c.put(ox + jx, oy + 5, "x")
        for y in range(6, 16):
            c.put(ox + jx, oy + y, "s")
        if jx - 1 >= 0:
            for y in range(8, 15):
                c.put(ox + jx - 1, oy + y, "t")
        if jx + 1 <= 15:
            for y in range(7, 15):
                c.put(ox + jx + 1, oy + y, "E")
    if chip_at is not None:
        chip(c, ox + chip_at, oy + 7, "t", "E")
    if end == "l":
        for y in range(0, 5):
            c.put(ox, oy + y, "x")
        c.put(ox, oy + 5, "z")
        for y in range(6, 15):
            c.put(ox, oy + y, "w")
            c.put(ox + 1, oy + y, "E")
        c.put(ox, oy + 15, "s")
    elif end == "r":
        for y in range(0, 5):
            c.put(ox + 15, oy + y, "D")
        for y in range(6, 16):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t")


def floor_under_step(c: Canvas, ox: int, oy: int, joints: list[int],
                     corner: str | None = None) -> None:
    """Below a raised block that stands on the floor: the block's warm face
    (rows 0-5) is flush with the floor's front face (rows 7-15), so the lower
    floor's lip/shadow line continues as a single joint (row 6) and the block's
    lit side edge stops at the walking line instead of running down the face."""
    c.fill(ox, oy, ox + 15, oy + 5, "D")
    c.fill(ox, oy + 6, ox + 15, oy + 6, "s")
    c.fill(ox, oy + 7, ox + 15, oy + 7, "E")
    c.fill(ox, oy + 8, ox + 15, oy + 14, "D")
    c.fill(ox, oy + 15, ox + 15, oy + 15, "s")
    for jx in joints:
        for y in range(0, 6):
            c.put(ox + jx, oy + y, "s")
        if jx + 1 <= 15:
            for y in range(0, 6):
                c.put(ox + jx + 1, oy + y, "E")
    if corner == "l":
        for y in range(0, 5):
            c.put(ox, oy + y, "w")
            c.put(ox + 1, oy + y, "E")
        c.put(ox, oy + 5, "x")
    elif corner == "r":
        for y in range(0, 5):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t")
        c.put(ox + 15, oy + 5, "s")


def masonry_fill(c: Canvas, ox: int, oy: int, seed: str, face: str, light: str,
                 joint: str, side: str | None = None,
                 joint_cols: tuple = (15, None), chipped: bool = False) -> None:
    """Two 8 px courses of long dark blocks for fills. ``joint_cols`` gives the
    vertical joint per course (None = block continues into the next tile), so
    mixing variants yields irregular 16-48 px block lengths."""
    rng = random.Random(f"{KIT_ID}:{seed}")
    c.fill(ox, oy, ox + 15, oy + 15, face)
    for k in range(2):
        y0 = oy + 8 * k
        jx = joint_cols[k]
        c.fill(ox, y0 + 7, ox + 15, y0 + 7, joint)
        if jx is not None:
            for y in range(y0, y0 + 7):
                c.put(ox + jx, y, joint)
        # sparse partial top highlight (lit upper edge of each block)
        start = 0 if jx is None else jx + 1
        if start <= 15 and rng.random() < 0.7:
            run = rng.randint(2, 6)
            c.fill(ox + start, y0, ox + min(15, start + run - 1), y0, light)
    if chipped:
        chip(c, ox + 9, oy + 8, joint, light)
    if side == "l":
        for y in range(16):
            c.put(ox, oy + y, "w")
            if c.get(ox + 1, oy + y) == face:
                c.put(ox + 1, oy + y, "v")
    elif side == "r":
        for y in range(16):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t")


def ceiling_edge(c: Canvas, ox: int, oy: int, joints: list[int], seed: str,
                 chip_at: int | None = None, quoin: str | None = None,
                 end: str | None = None) -> None:
    """Ceiling mass (rows 0-6) over a stone lintel course whose underside rim
    (row 14) catches torchlight. ``quoin`` turns the course down into a wall
    face (inner corner); ``end`` finishes the course with a vertical rim
    (outer corner, where the ceiling steps up)."""
    c.fill(ox, oy, ox + 15, oy + 7, "t")
    c.put(ox + 15, oy + 0, "s")
    for y in range(0, 7):
        c.put(ox + 7, oy + y, "s")
    c.fill(ox, oy + 7, ox + 15, oy + 8, "s")
    c.fill(ox, oy + 9, ox + 15, oy + 12, "u")
    c.fill(ox, oy + 13, ox + 15, oy + 13, "v")
    c.fill(ox, oy + 14, ox + 15, oy + 14, "w")
    c.fill(ox, oy + 15, ox + 15, oy + 15, "s")
    for jx in joints:
        for y in range(9, 15):
            c.put(ox + jx, oy + y, "s")
        if jx + 1 <= 15:
            for y in range(9, 14):
                c.put(ox + jx + 1, oy + y, "v")
    if chip_at is not None:
        chip(c, ox + chip_at, oy + 9, "s", "v")
    if quoin == "l":  # wall on the left: its quoin runs up into the course
        c.fill(ox, oy + 9, ox + 4, oy + 15, "v")
        c.fill(ox + 1, oy + 9, ox + 4, oy + 9, "w")
        for y in range(9, 16):
            c.put(ox, oy + y, "x")
            c.put(ox + 5, oy + y, "s")
    elif quoin == "r":
        c.fill(ox + 11, oy + 9, ox + 15, oy + 15, "u")
        c.fill(ox + 11, oy + 9, ox + 14, oy + 9, "v")
        for y in range(9, 16):
            c.put(ox + 10, oy + y, "s")
            c.put(ox + 15, oy + y, "t")
    if end == "l":  # course ends here; open space continues to the left
        for y in range(0, 15):
            c.put(ox, oy + y, "w" if y >= 9 else "u")
            if y >= 9:
                c.put(ox + 1, oy + y, "v")
        c.put(ox, oy + 15, "s")
    elif end == "r":
        for y in range(0, 16):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t" if y < 9 else "u")


def ceiling_fill(c: Canvas, ox: int, oy: int, seed: str) -> None:
    c.fill(ox, oy, ox + 15, oy + 15, "t")
    for k in range(2):
        y0 = oy + 8 * k
        jx = 15 if k == 0 else 7
        c.fill(ox, y0 + 7, ox + 15, y0 + 7, "s")
        for y in range(y0, y0 + 7):
            c.put(ox + jx, y, "s")
    if seed.endswith("b"):
        chip(c, ox + 9, oy + 8, "s", "u")


def wall_bricks(c: Canvas, ox: int, oy: int, seed: str) -> None:
    """Collidable solid brick: lighter faces, full bevel and dark stone mortar,
    so solid geometry never reads as background."""
    strip = Canvas(16, 16)
    brick_strip(strip, 0, 0, 16, 2, seed, mortar="s", tones=(4, 5), weights=(1, 1),
                full_bevel=True)
    # 16-wide strip: the odd course is a single seam brick, so give it a
    # visible joint at x=7 and keep the tile self-contained.
    for y in range(8, 15):
        strip.put(7, y, "s")
    for y, row in enumerate(strip.rows()):
        for x, ch in enumerate(row):
            c.put(ox + x, oy + y, ch)


def quoin(c: Canvas, ox: int, oy: int, side: str, long_first: bool) -> None:
    """Stone quoins on the exposed vertical edge of a collidable brick wall:
    alternating long (5 px) and short (3 px) blocks, 8 px tall."""
    for k in range(2):
        y0 = oy + 8 * k
        width = 5 if (k == 0) == long_first else 3
        if side == "l":
            c.fill(ox, y0, ox + width - 1, y0 + 6, "v")
            c.fill(ox, y0, ox + width - 1, y0, "w")
            for y in range(y0, y0 + 7):
                c.put(ox, y, "x")
                c.put(ox + width, y, "s")
            c.fill(ox, y0 + 7, ox + width, y0 + 7, "s")
        else:
            c.fill(ox + 16 - width, y0, ox + 15, y0 + 6, "u")
            c.fill(ox + 16 - width, y0, ox + 15, y0, "v")
            for y in range(y0, y0 + 7):
                c.put(ox + 15, y, "t")
                c.put(ox + 15 - width, y, "s")
            c.fill(ox + 15 - width, y0 + 7, ox + 15, y0 + 7, "s")


def wall_cap(c: Canvas, ox: int, oy: int, seed: str, side: str | None) -> None:
    """Walkable cap stone on top of a collidable brick wall (rows 0-6), with the
    wall's brick below; ``side`` adds the lit (l) or shaded (r) corner."""
    wall_bricks(c, ox, oy, seed)
    for y, key in enumerate(["F", "E", "D", "D", "y", "s", "t"]):
        c.fill(ox, oy + y, ox + 15, oy + y, key)
    if side == "l":
        quoin(c, ox, oy, "l", True)
        for y in range(0, 4):
            c.put(ox, oy + y, "x")
        c.put(ox, oy + 4, "z")
        c.fill(ox, oy + 5, ox + 5, oy + 6, "s")
        c.put(ox, oy + 6, "x")
    elif side == "r":
        quoin(c, ox, oy, "r", True)
        for y in range(0, 5):
            c.put(ox + 15, oy + y, "t")
        c.fill(ox + 10, oy + 5, ox + 15, oy + 6, "s")


def build_terrain() -> tuple[dict, list[dict]]:
    w, h = 8 * TILE, 4 * TILE
    layers = {name: Canvas(w, h) for name in ("floor", "ceiling", "wall")}
    tiles: list[dict] = []

    def add(name: str, layer: str, col: int, row: int, extra: dict | None = None):
        tiles.append({"id": name, "layer": layer, "cell": [col, row], **(extra or {})})
        return layers[layer], col * TILE, row * TILE

    walk = {"walkable_top": True}
    # row 0: walkable floor tops and the ceiling course
    floor_top(*add("floor_top_a", "floor", 0, 0, walk), [15], "floor_top_a")
    floor_top(*add("floor_top_b", "floor", 1, 0, walk), [], "floor_top_b", chip_at=4)
    floor_top(*add("floor_top_c", "floor", 2, 0, walk), [7], "floor_top_c")
    floor_top(*add("floor_top_end_l", "floor", 3, 0, walk), [], "floor_top_end_l", end="l")
    floor_top(*add("floor_top_end_r", "floor", 4, 0, walk), [], "floor_top_end_r", end="r")
    ceiling_edge(*add("ceiling_edge_a", "ceiling", 5, 0), [15], "ceiling_edge_a")
    ceiling_edge(*add("ceiling_edge_b", "ceiling", 6, 0), [], "ceiling_edge_b", chip_at=5)
    ceiling_edge(*add("ceiling_edge_c", "ceiling", 7, 0), [7], "ceiling_edge_c")
    # row 1: fills and solid wall
    masonry_fill(*add("floor_fill_a", "floor", 0, 1), "floor_fill_a", "t", "u", "s")
    masonry_fill(*add("floor_fill_b", "floor", 1, 1), "floor_fill_b", "t", "u", "s",
                 joint_cols=(None, 7), chipped=True)
    masonry_fill(*add("floor_fill_side_l", "floor", 2, 1), "floor_fill_side_l", "t", "u", "s",
                 side="l", joint_cols=(None, 7))
    masonry_fill(*add("floor_fill_side_r", "floor", 3, 1), "floor_fill_side_r", "t", "u", "s",
                 side="r")
    ceiling_fill(*add("ceiling_fill_a", "ceiling", 4, 1), "ceiling_fill_a")
    ceiling_fill(*add("ceiling_fill_b", "ceiling", 5, 1), "ceiling_fill_b")
    wall_bricks(*add("wall_fill_a", "wall", 6, 1), "wall_fill_a")
    wall_bricks(*add("wall_fill_b", "wall", 7, 1), "wall_fill_b")
    # row 2: raised-block footing, vertical wall faces and caps
    floor_under_step(*add("floor_inner_corner_l", "floor", 0, 2), [], corner="l")
    floor_under_step(*add("floor_inner_corner_r", "floor", 1, 2), [], corner="r")
    floor_under_step(*add("floor_step_base_a", "floor", 2, 2), [9])
    floor_under_step(*add("floor_step_base_b", "floor", 3, 2), [])
    for col, name, side in ((4, "wall_face_l", "l"), (5, "wall_face_r", "r")):
        canvas, x, y = add(name, "wall", col, 2)
        wall_bricks(canvas, x, y, name)
        quoin(canvas, x, y, side, True)
    wall_cap(*add("wall_cap_l", "wall", 6, 2, walk), "wall_cap_l", "l")
    wall_cap(*add("wall_cap_r", "wall", 7, 2, walk), "wall_cap_r", "r")
    # row 3: cap middle, ceiling corners, second quoin phase, fourth floor top
    wall_cap(*add("wall_cap_mid", "wall", 0, 3, walk), "wall_cap_mid", None)
    ceiling_edge(*add("ceiling_corner_in_l", "ceiling", 1, 3), [], "ceiling_corner_in_l",
                 quoin="l")
    ceiling_edge(*add("ceiling_corner_in_r", "ceiling", 2, 3), [], "ceiling_corner_in_r",
                 quoin="r")
    ceiling_edge(*add("ceiling_corner_out_l", "ceiling", 3, 3), [], "ceiling_corner_out_l",
                 end="l")
    ceiling_edge(*add("ceiling_corner_out_r", "ceiling", 4, 3), [], "ceiling_corner_out_r",
                 end="r")
    for col, name, side in ((5, "wall_face_l_b", "l"), (6, "wall_face_r_b", "r")):
        canvas, x, y = add(name, "wall", col, 3)
        wall_bricks(canvas, x, y, name)
        quoin(canvas, x, y, side, False)
    floor_top(*add("floor_top_d", "floor", 7, 3, walk), [11], "floor_top_d", chip_at=13)
    grid = make_grid([w, h], [0, 0], layers, ["ceiling", "wall", "floor"])
    return grid, tiles


# ---------------------------------------------------------------------------
# Backwall tiles (opaque, non-collidable)
# ---------------------------------------------------------------------------

def stepped(c: Canvas, ox: int, oy: int, y_mid: int, upper: str, lower: str,
            profile: tuple[int, ...], y0: int = 0, y1: int = 15) -> None:
    """Band change from ``upper`` to ``lower`` along a stepped edge: mostly a
    straight line with one 3 px notch per tile (``profile[x]`` offsets the row),
    staggered between tile kinds. No 1 px checkerboard, and no continuous
    undulation (which reads as a water line). Profiles wrap at 16 px."""
    for x in range(16):
        edge = y_mid + profile[x]
        for y in range(y0, y1 + 1):
            c.put(ox + x, oy + y, upper if y < edge else lower)


PROFILE_A = (0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0)
PROFILE_B = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, -1, -1, 0, 0, 0)
PROFILE_C = (0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)


def recess_tile(c: Canvas, ox: int, oy: int, kind: str) -> None:
    def band(y0: int, y1: int, key: str) -> None:
        c.fill(ox, oy + y0, ox + 15, oy + y1, key)

    if kind == "recess_vault":
        band(0, 15, "0")
        for x in (9, 10, 11):
            c.put(ox + x, oy + 6, "1")
    elif kind == "recess_vault_fade":
        stepped(c, ox, oy, 8, "0", "1", PROFILE_A)
    elif kind == "recess_wall_a":
        band(0, 15, "1")
        for x in (3, 4, 5):
            c.put(ox + x, oy + 9, "0")
    elif kind == "recess_wall_b":
        band(0, 15, "1")
        c.put(ox + 11, oy + 4, "2")
        c.put(ox + 12, oy + 4, "2")
    elif kind == "recess_wall_fade":
        stepped(c, ox, oy, 5, "1", "2", PROFILE_B)
        stepped(c, ox, oy, 12, "2", "3", PROFILE_C, y0=8)
    elif kind == "recess_low":
        stepped(c, ox, oy, 7, "3", "4", PROFILE_A)
    elif kind == "recess_haze":
        band(0, 15, "4")
        for x in (5, 6, 7):
            c.put(ox + x, oy + 10, "3")
    elif kind == "recess_floor":
        band(0, 2, "4")
        band(3, 3, "2")  # dark far-floor contact line (a bright line read as water)
        band(4, 15, "3")
        stepped(c, ox, oy, 10, "3", "2", PROFILE_B, y0=7)
    else:
        raise ValueError(kind)


def build_backwall() -> tuple[dict, list[dict]]:
    w, h = 8 * TILE, 8 * TILE
    layers = {name: Canvas(w, h) for name in ("brick", "strips", "recess")}
    tiles: list[dict] = []
    # Six 64x32 running-bond panels (4x2 tiles each), toroidal. Tone variation
    # is deliberately narrow (mostly base tone) so the wall stays calm and no
    # large-scale light/dark pattern emerges when panels repeat.
    names = "abcdef"
    for n, panel in enumerate(names):
        px, py = (n % 2) * 64, (n // 2) * 32
        brick_strip(layers["brick"], px, py, 64, 4, f"panel_{panel}", weights=(2, 5, 2))
        for cy in range(2):
            for cx in range(4):
                tiles.append({
                    "id": f"brick_backwall_{panel}{cy * 4 + cx + 1}", "layer": "brick",
                    "cell": [(px // TILE) + cx, (py // TILE) + cy],
                    "panel": {"id": f"brick_{panel}", "family": "brick_body",
                              "cell": [cx, cy], "size": [4, 2]},
                })
    sy = 6 * TILE
    # Top strip under the ceiling: darker first course under a solid two-row
    # contact shadow (no high-frequency teeth against the collision edge).
    s = layers["strips"]
    brick_strip(s, 0, sy, 64, 2, "strip_top", shift_per_course=[-1, 0], weights=(2, 5, 2))
    s.fill(0, sy, 63, sy, "m")
    s.fill(0, sy + 1, 63, sy + 1, "a")
    for cx in range(4):
        tiles.append({"id": f"brick_backwall_top_{cx + 1}", "layer": "strips",
                      "cell": [cx, 6],
                      "panel": {"id": "brick_top", "family": "brick_top",
                                "cell": [cx, 0], "size": [4, 1]}})
    # Base strip behind the floor contact line: one darker brick course over a
    # plain, solid stone plinth that is clearly darker than the floor surface.
    brick_strip(s, 64, sy, 64, 1, "strip_base", shift_per_course=[-1], weights=(2, 5, 2))
    s.fill(64, sy + 7, 127, sy + 7, "m")
    s.fill(64, sy + 8, 127, sy + 8, "u")
    s.fill(64, sy + 9, 127, sy + 13, "t")
    s.fill(64, sy + 14, 127, sy + 15, "s")
    for jx in (64 + 31, 64 + 63):
        for y in range(sy + 9, sy + 14):
            s.put(jx, y, "s")
    for cx in range(4):
        tiles.append({"id": f"brick_backwall_base_{cx + 1}", "layer": "strips",
                      "cell": [4 + cx, 6],
                      "panel": {"id": "brick_base", "family": "brick_base",
                                "cell": [cx, 0], "size": [4, 1]}})
    recess_kinds = ["recess_vault", "recess_vault_fade", "recess_wall_a", "recess_wall_b",
                    "recess_wall_fade", "recess_low", "recess_haze", "recess_floor"]
    for i, kind in enumerate(recess_kinds):
        recess_tile(layers["recess"], i * TILE, 7 * TILE, kind)
        tiles.append({"id": kind, "layer": "recess", "cell": [i, 7], "recess": True})
    grid = make_grid([w, h], [0, 0], layers, ["brick", "strips", "recess"])
    return grid, tiles


# ---------------------------------------------------------------------------
# Structure pieces (alpha, non-collidable): arch ring, pier modules, far arcade
# ---------------------------------------------------------------------------
ARCH_R_IN, ARCH_R_OUT, ARCH_VOUSSOIRS = 72, 96, 11
ARCH_SOFFIT = 3.5  # darker band on the inner edge: the arch's depth (soffit)


def draw_arch(c: Canvas, ox: int, oy: int) -> None:
    """Voussoir ring: stone d in [72, 94), extrados outline at 94, a 1 px contact
    shadow on the wall at 95. Only the upward-facing edge of each voussoir is
    lit; a darker soffit band runs along the inside of the ring."""
    cx, cy = 96.0, 96.0
    rng = random.Random(f"{KIT_ID}:arch")
    n = ARCH_VOUSSOIRS
    tones = [rng.choice("uvv") for _ in range(n)]
    tones[n // 2] = "v"  # keystone
    chipped = {i for i in range(n) if rng.random() < 0.34}
    for py in range(96):
        for px in range(192):
            dx, dy = px + 0.5 - cx, cy - (py + 0.5)
            d = math.hypot(dx, dy)
            if not (ARCH_R_IN <= d < ARCH_R_OUT) or dy <= 0:
                continue
            theta = math.atan2(dy, dx)  # 0 = right springing, pi = left
            pos = theta / math.pi * n
            idx = min(n - 1, int(pos))
            frac = pos - idx
            arc = math.pi / n * d  # voussoir arc length at this radius
            dist_lo, dist_hi = frac * arc, (1 - frac) * arc
            centre = (idx + 0.5) / n * math.pi
            crown = abs(centre - math.pi / 2) < 0.35
            # the joint side that faces upward (toward the crown)
            up_side = dist_hi if centre < math.pi / 2 else dist_lo
            key = tones[idx]
            if d >= ARCH_R_OUT - 1:
                key = "m"  # contact shadow on the wall just outside the ring
            elif d >= ARCH_R_OUT - 2:
                key = "s"  # extrados outline
            elif d < ARCH_R_IN + ARCH_SOFFIT:
                key = "t"  # soffit band (no inner highlight rim)
            elif min(dist_lo, dist_hi) < 0.75 and not (
                    dist_lo < 0.75 and idx == 0) and not (dist_hi < 0.75 and idx == n - 1):
                key = "s"  # radial mortar joint (none at the two springing faces)
            elif crown and d >= ARCH_R_OUT - 4:
                key = "w"  # crown voussoirs: lit top (extrados) edge
            elif not crown and up_side < 1.6:
                key = "w"  # lit upward-facing joint edge
            if idx in chipped and key in "uvw":
                # 3 px chip at the voussoir's outer, upward corner
                if d >= ARCH_R_OUT - 4.5 and up_side < 3.2 and not crown:
                    key = "t"
                elif crown and d >= ARCH_R_OUT - 4.5 and min(dist_lo, dist_hi) < 2.5 \
                        and dist_lo < dist_hi:
                    key = "t"
            c.put(ox + px, oy + py, key)


def draw_pier_capital(c: Canvas, ox: int, oy: int) -> None:
    c.fill(ox, oy, ox + 63, oy + 5, "v")
    c.fill(ox, oy, ox + 63, oy, "s")
    c.fill(ox + 1, oy + 1, ox + 62, oy + 1, "w")
    c.fill(ox, oy + 5, ox + 63, oy + 5, "t")
    for y in range(1, 6):
        c.put(ox, oy + y, "s")
        c.put(ox + 63, oy + y, "s")
        c.put(ox + 1, oy + y, "w")
        c.put(ox + 62, oy + y, "u")
    c.fill(ox + 4, oy + 6, ox + 59, oy + 15, "u")
    c.fill(ox + 4, oy + 6, ox + 59, oy + 6, "s")
    c.fill(ox + 5, oy + 7, ox + 58, oy + 7, "v")
    for y in range(6, 16):
        c.put(ox + 4, oy + y, "s")
        c.put(ox + 59, oy + y, "s")
    for y in range(7, 15):
        c.put(ox + 5, oy + y, "v")
        c.put(ox + 58, oy + y, "t")
    c.fill(ox + 5, oy + 15, ox + 58, oy + 15, "t")
    for jx in (ox + 31,):
        for y in range(7, 15):
            c.put(jx, oy + y, "s")
        for y in range(8, 15):
            c.put(jx + 1, oy + y, "v")


def ashlar_block(c: Canvas, x0: int, y0: int, x1: int, y1: int, tone: str,
                 rng: random.Random) -> None:
    """One dressed stone block with joint on its right/bottom edge. About a third
    of the blocks get a 3 px chipped top-left corner; no single-pixel pocks."""
    c.fill(x0, y0, x1, y1, tone)
    c.fill(x0, y1, x1, y1, "s")
    for y in range(y0, y1 + 1):
        c.put(x1, y, "s")
    run = rng.randint((x1 - x0) // 2, x1 - x0)
    c.fill(x0, y0, x0 + run - 1, y0, "w")
    for y in range(y0, y1 - 1):
        c.put(x0, y, "w")
    lower = "t" if tone == "u" else "u"
    c.fill(x1 - rng.randint(3, max(3, (x1 - x0) // 2)), y1 - 1, x1 - 1, y1 - 1, lower)
    if rng.random() < 0.34 and x1 - x0 > 8:
        cx = rng.randint(x0 + 2, x1 - 4)
        chip(c, cx, y0, "t", tone)


def draw_pier_shaft(c: Canvas, ox: int, oy: int) -> None:
    rng = random.Random(f"{KIT_ID}:pier_shaft")
    left, right = ox + 8, ox + 55
    # course 0: two 24 px blocks; course 1: 12 + 24 + 12 (running bond)
    ashlar_block(c, left, oy, left + 23, oy + 15, "v", rng)
    ashlar_block(c, left + 24, oy, right, oy + 15, "u", rng)
    ashlar_block(c, left, oy + 16, left + 11, oy + 31, "u", rng)
    ashlar_block(c, left + 12, oy + 16, left + 35, oy + 31, "v", rng)
    ashlar_block(c, left + 36, oy + 16, right, oy + 31, "v", rng)
    for y in range(oy, oy + 32):
        c.put(left, y, "s")
        c.put(left + 1, y, "w")
        c.put(right, y, "s")
        c.put(right - 1, y, "t")


def draw_pier_base(c: Canvas, ox: int, oy: int) -> None:
    rng = random.Random(f"{KIT_ID}:pier_base")
    # torus moulding stepping out from the 48 px shaft to a 60 px plinth
    c.fill(ox + 6, oy, ox + 57, oy + 3, "v")
    c.fill(ox + 6, oy, ox + 57, oy, "s")
    c.fill(ox + 7, oy + 1, ox + 56, oy + 1, "x")
    c.fill(ox + 7, oy + 2, ox + 56, oy + 2, "w")
    c.fill(ox + 6, oy + 3, ox + 57, oy + 3, "t")
    for y in range(0, 4):
        c.put(ox + 6, oy + y, "s")
        c.put(ox + 57, oy + y, "s")
    ashlar_block(c, ox + 2, oy + 4, ox + 31, oy + 17, "v", rng)
    ashlar_block(c, ox + 32, oy + 4, ox + 61, oy + 17, "u", rng)
    ashlar_block(c, ox + 2, oy + 18, ox + 17, oy + 31, "u", rng)
    ashlar_block(c, ox + 18, oy + 18, ox + 45, oy + 31, "u", rng)
    ashlar_block(c, ox + 46, oy + 18, ox + 61, oy + 31, "u", rng)
    for y in range(4, 32):
        c.put(ox + 2, oy + y, "s")
        c.put(ox + 3, oy + y, "w")
        c.put(ox + 61, oy + y, "s")
        c.put(ox + 60, oy + y, "t")
    c.fill(ox + 2, oy + 4, ox + 61, oy + 4, "s")
    # contact shadow where the plinth meets the walkable floor
    c.fill(ox + 3, oy + 30, ox + 60, oy + 30, "t")
    c.fill(ox + 2, oy + 31, ox + 61, oy + 31, "s")


FAR_TOP_PROFILE = (0,) * 16  # straight far cornice (a wavy edge reads as water)


def draw_far_arcade(c: Canvas, ox: int, oy: int, openings: list[tuple[int, int]],
                    spring: int, torches: list[tuple[int, int]]) -> None:
    """Distant arcade inside a recess. Figure/ground is inverted against the
    old "tombstone" read: the far wall, spandrels and piers are mid-blue
    (``3``, cornice ``4``) and the round arch heads are dark voids (``1``,
    ``2``). Lower down, the openings fill with lighter floor-level haze (``4``,
    L* ~26) so the actor-height band stays clearly lighter than dark actor
    masses; the piers then read as darker columns in fog. A 1 px dark contact
    line (``2``) marks the far floor inside each opening (a bright line over a
    darker band read as water). Rows above the cornice stay transparent."""
    w, h, floor_y = 144, 96, 86
    for px in range(w):
        top = 10 + FAR_TOP_PROFILE[px % 16]
        for py in range(top, h):
            x, y = px + 0.5, py + 0.5
            hole = False
            for acx, r_in in openings:
                inside = (math.hypot(x - acx, spring - y) < r_in) if y < spring \
                    else abs(x - acx) < r_in
                hole = hole or inside
            if py > floor_y:
                key = "3"  # far floor in front of the far wall
            elif py == floor_y:
                key = "2" if hole else "3"  # dark contact line at the far floor
            elif hole:
                # dark arch head, then a plain 2-row step, then floor haze
                if py < spring + 2:
                    key = "1"
                elif py < spring + 4:
                    key = "2"
                else:
                    key = "4"
            else:
                key = "3"
            c.put(ox + px, oy + py, key)
        c.put(ox + px, oy + top, "4")  # far cornice catches a little light
    for fx, fy in torches:  # tiny distant torches, 2 px each
        c.put(ox + fx, oy + fy, "O")
        c.put(ox + fx, oy + fy + 1, "R")


def draw_pilaster(c: Canvas, ox: int, oy: int) -> None:
    """Plain stone pilaster module (32x16 piece, 24 px shaft, repeats
    vertically): one dressed block per 16 px with the piers' stone and
    lighting, no alternating widths (an alternating quoin read as a ladder)."""
    x0, x1, y0, y1 = ox + 4, ox + 27, oy, oy + 15
    c.fill(x0, y0, x1, y1, "v")
    c.fill(x0, y1, x1, y1, "s")
    for y in range(y0, y1 + 1):
        c.put(x0, y, "s")
        c.put(x1, y, "s")
    for y in range(y0, y1):
        c.put(x0 + 1, y, "w")
        c.put(x1 - 1, y, "t")
    c.fill(x0 + 1, y0, x0 + 14, y0, "w")
    c.fill(x1 - 9, y1 - 1, x1 - 1, y1 - 1, "u")


def build_structure() -> tuple[dict, list[dict]]:
    w, h = 544, 96
    layers = {name: Canvas(w, h) for name in ("arch", "pier", "depth")}
    draw_arch(layers["arch"], 0, 0)
    draw_pier_capital(layers["pier"], 192, 0)
    draw_pier_shaft(layers["pier"], 192, 16)
    draw_pier_base(layers["pier"], 192, 48)
    draw_far_arcade(layers["depth"], 256, 0, [(24, 19), (72, 19), (120, 19)], 40, [])
    draw_far_arcade(layers["depth"], 400, 0, [(38, 25), (106, 25)], 44, [])
    draw_pilaster(layers["pier"], 192, 80)
    far = {"layer": "depth", "anchor": [0, 96],
           "anchor_name": "bottom_left (opening floor line)", "grid_snap": 16}
    pieces = [
        {"id": "arch_ring_r72", "layer": "arch", "rect": [0, 0, 192, 96],
         "anchor": [96, 96], "anchor_name": "springing_centre", "grid_snap": 8,
         "arch": {"radius_inner": ARCH_R_IN, "radius_outer": ARCH_R_OUT,
                  "voussoirs": ARCH_VOUSSOIRS, "backwall_inside": "recess"}},
        {"id": "pier_capital", "layer": "pier", "rect": [192, 0, 64, 16],
         "anchor": [32, 0], "anchor_name": "top_centre (arch springing line)",
         "grid_snap": 8},
        {"id": "pier_shaft", "layer": "pier", "rect": [192, 16, 64, 32],
         "anchor": [32, 0], "anchor_name": "top_centre (stack vertically, 32 px repeat)",
         "grid_snap": 8, "repeat": "vertical"},
        {"id": "pier_base", "layer": "pier", "rect": [192, 48, 64, 32],
         "anchor": [32, 32], "anchor_name": "bottom_centre (stands on floor top)",
         "grid_snap": 8},
        {"id": "pilaster_plain", "layer": "pier", "rect": [192, 80, 32, 16],
         "anchor": [16, 0], "anchor_name": "top_centre (24 px shaft; stack vertically, 16 px repeat)",
         "grid_snap": 8, "repeat": "vertical"},
        {"id": "far_arcade_a", "rect": [256, 0, 144, 96], **far},
        {"id": "far_arcade_b", "rect": [400, 0, 144, 96], **far},
    ]
    grid = make_grid([w, h], [0, 0], layers, ["depth", "pier", "arch"])
    return grid, pieces


# ---------------------------------------------------------------------------
# Props (alpha, non-collidable)
# ---------------------------------------------------------------------------
PIPE_ACROSS = "klnljjik"  # 8 px cylinder, lit from above / left


def pipe_h(c: Canvas, ox: int, oy: int, x0: int = 0, x1: int = 15) -> None:
    for i, key in enumerate(PIPE_ACROSS):
        c.fill(ox + x0, oy + 4 + i, ox + x1, oy + 4 + i, key)


def pipe_v(c: Canvas, ox: int, oy: int, y0: int = 0, y1: int = 15) -> None:
    for i, key in enumerate(PIPE_ACROSS):
        c.fill(ox + 4 + i, oy + y0, ox + 4 + i, oy + y1, key)


def elbow(c: Canvas, ox: int, oy: int, centre: tuple[float, float]) -> None:
    """Quarter-circle pipe bend with the same 8 px cross-section as the
    straights; ``centre`` is the bend's centre of curvature in tile coords."""
    for py in range(16):
        for px in range(16):
            d = math.hypot(px + 0.5 - centre[0], py + 0.5 - centre[1])
            if centre[1] == 16:   # bend centred below: outer edge is up/left
                band = int(12.5 - d) if 4.5 <= d < 12.5 else None
            else:                 # bend centred above-left: inner edge is up/left
                band = int(d - 4.5) if 4.5 <= d < 12.5 else None
            if band is not None and 0 <= band < 8:
                c.put(ox + px, oy + py, PIPE_ACROSS[band])


def collar_v(c: Canvas, ox: int, oy: int, y0: int, y1: int) -> None:
    """A bolted 10 px collar around a vertical pipe (floor or ceiling entry)."""
    c.fill(ox + 3, oy + y0, ox + 12, oy + y1, "j")
    c.fill(ox + 3, oy + y0, ox + 12, oy + y0, "k")
    c.fill(ox + 3, oy + y1, ox + 12, oy + y1, "k")
    for y in range(y0, y1 + 1):
        c.put(ox + 3, oy + y, "k")
        c.put(ox + 12, oy + y, "k")
    for y in range(y0 + 1, y1):
        c.put(ox + 4, oy + y, "l")
    c.fill(ox + 4, oy + y0 + 1, ox + 11, oy + y0 + 1, "l")
    c.put(ox + 6, oy + y0 + 2, "n")
    c.put(ox + 10, oy + y0 + 2, "n")


def build_props() -> tuple[dict, list[dict]]:
    w, h = 128, 128
    layers = {name: Canvas(w, h) for name in ("decal", "pipes", "box", "accent", "door",
                                              "sconce")}
    p = layers["pipes"]
    pieces: list[dict] = []

    def piece(pid, layer, rect, anchor, anchor_name, snap, role="prop", **extra):
        pieces.append({"id": pid, "layer": layer, "rect": rect, "anchor": anchor,
                       "anchor_name": anchor_name, "grid_snap": snap, "role": role, **extra})

    tile = ([0, 0], "top_left (tile)", 16)
    # pipe_h_a: plain run
    pipe_h(p, 0, 0)
    piece("pipe_h_a", "pipes", [0, 0, 16, 16], *tile)
    # pipe_h_rust: run with a rust bloom along the underside
    pipe_h(p, 16, 0)
    for x, y, k in ((19, 9, "q"), (20, 9, "r"), (21, 9, "q"), (20, 10, "q"), (21, 10, "q"),
                    (26, 10, "q"), (27, 9, "q"), (27, 10, "r"), (28, 10, "q")):
        p.put(x, y, k)
    piece("pipe_h_rust", "pipes", [16, 0, 16, 16], *tile)
    # pipe_h_flange: bolted coupling
    pipe_h(p, 32, 0)
    p.fill(32 + 6, 2, 32 + 9, 13, "j")
    for y in range(2, 14):
        p.put(32 + 6, y, "k")
        p.put(32 + 9, y, "k")
        p.put(32 + 7, y, "l")
    p.fill(32 + 6, 2, 32 + 9, 2, "k")
    p.fill(32 + 6, 13, 32 + 9, 13, "k")
    p.put(32 + 8, 3, "n")
    p.put(32 + 8, 12, "n")
    piece("pipe_h_flange", "pipes", [32, 0, 16, 16], *tile)
    # pipe_h_bracket: wall strap with fixing bolt above
    pipe_h(p, 48, 0)
    p.fill(48 + 6, 3, 48 + 8, 12, "i")
    for y in range(3, 13):
        p.put(48 + 6, y, "k")
        p.put(48 + 8, y, "k")
    p.put(48 + 7, 4, "l")
    p.fill(48 + 6, 1, 48 + 8, 2, "k")
    p.put(48 + 7, 1, "n")
    piece("pipe_h_bracket", "pipes", [48, 0, 16, 16], *tile)
    # pipe_v_a
    pipe_v(p, 64, 0)
    piece("pipe_v_a", "pipes", [64, 0, 16, 16], *tile)
    # pipe_v_bracket
    pipe_v(p, 80, 0)
    p.fill(80 + 3, 6, 80 + 12, 8, "i")
    for x in range(80 + 3, 80 + 13):
        p.put(x, 6, "k")
        p.put(x, 8, "k")
    p.put(80 + 4, 7, "l")
    p.fill(80 + 1, 6, 80 + 2, 8, "k")
    p.put(80 + 1, 7, "n")
    piece("pipe_v_bracket", "pipes", [80, 0, 16, 16], *tile)
    # pipe_elbow_r_d: from the right edge turning down to the bottom edge
    elbow(p, 96, 0, (16, 16))
    piece("pipe_elbow_r_d", "pipes", [96, 0, 16, 16], *tile)
    # pipe_elbow_l_u: from the left edge turning up to the top edge
    elbow(p, 112, 0, (0, 0))
    piece("pipe_elbow_l_u", "pipes", [112, 0, 16, 16], *tile)

    # valve_wheel: hand wheel on a vertical pipe (replaces a pipe_v tile)
    pipe_v(p, 0, 16)
    for py in range(16):
        for px in range(16):
            d = math.hypot(px + 0.5 - 8, py + 0.5 - 8)
            x, y = px, 16 + py
            if 6.4 <= d < 7.6:
                p.put(x, y, "k")
            elif 4.6 <= d < 6.4:
                upper_left = (px + py) < 15
                p.put(x, y, "P" if upper_left else "p")
            elif 4.0 <= d < 4.6:
                p.put(x, y, "k")
    for i in range(3, 13):
        if 2 <= abs(i - 7.5) + 0.5:
            p.put(i, 16 + 7, "p" if p.get(i, 16 + 7) not in "Pp" else p.get(i, 16 + 7))
            p.put(7, 16 + i, "p" if p.get(7, 16 + i) not in "Pp" else p.get(7, 16 + i))
    p.fill(6, 16 + 6, 9, 16 + 9, "k")
    p.fill(7, 16 + 7, 8, 16 + 8, "n")
    p.put(7, 16 + 7, "o")
    for x, y in ((5, 16 + 3), (4, 16 + 4)):
        p.put(x, y, "P")
    piece("valve_wheel", "pipes", [0, 16, 16, 16], *tile)
    # pipe_v_floor_collar: conduit entering the floor
    pipe_v(p, 16, 16, 0, 15)
    collar_v(p, 16, 16, 11, 15)
    piece("pipe_v_floor_collar", "pipes", [16, 16, 16, 16], *tile)
    # pipe_v_ceiling_collar: pipe rising into the ceiling course
    pipe_v(p, 32, 16, 0, 15)
    collar_v(p, 32, 16, 0, 4)
    piece("pipe_v_ceiling_collar", "pipes", [32, 16, 16, 16], *tile)
    # pipe_wall_collar_r: run ends in a round, face-on bolted collar where the
    # pipe turns into the wall (end-on bore visible)
    pipe_h(p, 48, 16, 0, 7)
    for py in range(16):
        for px in range(16):
            d = math.hypot(px + 0.5 - 10.5, py + 0.5 - 7.5)
            if d < 5.6:
                key = "k" if d >= 4.8 else ("l" if (px + py) < 17 else "j")
                if d < 2.6:
                    key = "i" if d >= 1.6 else "k"
                p.put(48 + px, 16 + py, key)
    for bx, by in ((10, 4), (13, 7), (10, 10), (7, 7)):
        p.put(48 + bx, 16 + by, "n")
    piece("pipe_wall_collar_r", "pipes", [48, 16, 16, 16], *tile)

    # maintenance_box: small riveted service cabinet with gauge and blank plate
    box = [
        "....kkllllkk....................",
        "...kjjnnjjjjk...................",
        ".kkkkkkkkkkkkkkkkkkkkkkkkkkkkkk.",
        "kllllllllllllllllllllllllllllllk",
        "kl.ljjjjjjjjjjjjjjjjjjjjjjjjljik",
        "kl.jjjjjjjjjjjjjjjjjjjjjjjjjjjik",
        "klj.kkkkkkkkkkkkkkkkkkkkkkkkjjik",
        "kljkkllllllllllllllllllllllkjjik",
        "kljnkljjjjkkkkkjjjjjjjjjjjjikjik",
        "kljnkljjjkkxxxkkjjjjjjjjjjjikjik",
        "kljkkljjkkxxxxxkkjjjjjjjjjjikjik",
        "kljjklljkxxxxRxxkjjjjjjjjjjikjik",
        "kljjkljjkxxxRxxxkjjjjjjjjjjikjik",
        "kljjkljjkxxxxxxxkjjjjjjjjjjikjik",
        "kljjkljjkkxxxxxkkjjjjjjjnnjikjik",
        "kljjkljjjkkxxxkkjjjjjjjjnojikjik",
        "kljjkljjjjkkkkkjjjjjjjjjnnjikjik",
        "kljjkljjjjjjjjjjjjjjjjjjkkjikjik",
        "kljjkljjjjjjjjjjjjjjjjjjjjjikjik",
        "kljnkljjjqqqqqqqqqqqjjjjjjjikjik",
        "kljnkljjjqTTTTTTTTTqjjjjjjjikjik",
        "kljkkljjjqTiTiiTiTTqjjjjjjjikjik",
        "kljjkljjjqTTTTTTTTTqjjjjjjjikjik",
        "kljjkljjjqqqqqqqqqqqjjjjjjjikjik",
        "kljjkljjjjjjjjjjjjjjjjjjjjjikjik",
        "kljjkiiiiiiiiiiiiiiiiiiiiiiikjik",
        "kljjkkkkkkkkkkkkkkkkkkkkkkkkkjik",
        "kl.jjjjjjjjjjjjqjjjjjjjjjjjjjjik",
        "kl.jjjjjjjjjjjjrjjjjjjjjjjjjjiik",
        ".kiiiiiiiiiiiiiiiiiiiiiiiiiiiik.",
        "..kkkkkkkkkkkkkkkkkkkkkkkkkkkk..",
        "....................kkllllkk....",
    ]
    layers["box"].stamp(box, 64, 16)
    for x, y in ((66, 20), (93, 20), (66, 43), (93, 43)):  # frame rivets
        layers["box"].put(x, y, "n")
    piece("maintenance_box", "box", [64, 16, 32, 32], [0, 0],
          "top_left (pipe enters at x 4..11, conduit leaves at x 20..27)", 16)

    # lichen_crust: tiny optional accent, a flat crust hugging a mortar joint
    # (row 7 lands on a mortar row when placed on the 8 px grid)
    crust = [
        "........",
        "........",
        "........",
        "........",
        "........",
        "..GHG...",
        ".GHHLHG.",
        "GGHGGHGG",
    ]
    layers["accent"].stamp(crust, 96, 16)
    piece("lichen_crust", "accent", [96, 16, 8, 8], [0, 7], "mortar_row (local row 7)", 8)

    # torch_wall_unlit: the same sconce with a charred stub and no flame
    unlit = Canvas(16, 32)
    unlit.stamp(TORCH_SCONCE, 0, 16)
    unlit.stamp([
        "......kUk.......",
        ".....kUVUk......",
        ".....UVVVU......",
    ], 0, 13)
    unlit.fill(4, 16, 11, 16, "j")  # cold rim, no warm light
    unlit.put(4, 16, "n")
    for y, row in enumerate(unlit.rows()):
        for x, ch in enumerate(row):
            if ch != ".":
                layers["sconce"].put(112 + x, 16 + y, ch)
    piece("torch_wall_unlit", "sconce", [112, 16, 16, 32], [8, 29],
          "wall_mount (same as torch_wall)", 16)

    # grime_patch_a/b: soot and grime over a few brick faces (follows the
    # global brick coursing, so it must sit on the 16 px grid; mortar untouched)
    d = layers["decal"]
    grime = {
        # (course, brick start x, key): only whole bricks inside the 48x16 decal
        "grime_patch_a": ((0, 96), [(0, 0, "a"), (0, 16, "m"), (0, 32, "a"), (1, 8, "a"),
                                    (1, 24, "a")]),
        "grime_patch_b": ((0, 112), [(0, 0, "a"), (0, 16, "a"), (1, 8, "m"), (1, 24, "a"),
                                     (0, 32, "a")]),
    }
    for pid, ((gx, gy), bricks) in grime.items():
        for course, start, key in bricks:
            for dy in range(7):
                for dx in range(15):
                    d.put(gx + start + dx, gy + course * 8 + dy, key)
        piece(pid, "decal", [gx, gy, 48, 16], [0, 0], "top_left (brick-grid aligned)", 16,
              role="backwall", decal=True)

    # torch_glow: minimal warm light decal, ~44 px wide, in three bands. It
    # touches brick faces and brick top edges only (mortar stays dark), and each
    # brick gets its own small radius jitter so the outline follows the bricks
    # irregularly instead of reading as a stamped disc. A Bevy additive light is
    # the intended richer option; this decal is the art-only fallback.
    gx0, gy0 = 0, 48
    jitter_rng = random.Random(f"{KIT_ID}:torch_glow")
    for course in range(6):
        offset = 0 if course % 2 == 0 else 8
        for start in range(offset - 16, 48, 16):
            jitter = jitter_rng.choice((-2, -1, 0, 0, 1, 2))
            for dyy in range(7):
                for dx in range(15):
                    px, py = start + dx, course * 8 + dyy
                    if not (0 <= px < 48):
                        continue
                    dy = py + 0.5 - 24.0
                    d = math.hypot(px + 0.5 - 24.0, dy * (0.9 if dy < 0 else 1.35)) - jitter
                    if course % 2 == 1 and d >= 9:
                        d += 2.5  # alternate courses stop short: ragged outer edge
                    top = dyy == 0
                    if d < 8:
                        key = "g"
                    elif d < 14:
                        key = "g" if top else "f"
                    elif d < 21 and top and dy < 8:
                        key = "f"
                    else:
                        key = None
                    if key:
                        layers["decal"].put(gx0 + px, gy0 + py, key)
    piece("torch_glow", "decal", [0, 48, 48, 48], [24, 24],
          "light_centre (= torch flame centre; place at torch tile -1,-1)", 16,
          role="backwall", decal=True)

    # door_plank: simple timber door in a stone frame (module A). The piece is
    # 40x80 with a 32x72 timber leaf, so a 52 px actor has ~20 px headroom.
    dx0, dy0, dw, dh = 48, 48, 40, 80
    lintel = [
        "." + "v" * (dw - 2) + ".",
        "v" + "w" * 19 + "s" + "v" + "w" * 17 + "u",
        "v" * 20 + "s" + "v" * 18 + "u",
        "v" * 20 + "s" + "v" * 18 + "u",
        "u" * 20 + "s" + "u" * 19,
        "s" * dw,
    ]
    jamb_l = ["xvvs", "wvvs", "wvvs", "wvvs", "wvvs", "wvvs", "wvus", "ssss"]
    jamb_r = ["svvu", "svvu", "svvu", "svvu", "svvu", "svvu", "suut", "ssss"]
    dc = layers["door"]
    dc.stamp(lintel, dx0, dy0)
    for y in range(6, dh):
        i = (y - 6) % 8
        dc.stamp([jamb_l[i]], dx0, dy0 + y)
        dc.stamp([jamb_r[i]], dx0 + dw - 4, dy0 + y)
    planks = "UXVVVVUXVVVUXVVVVUXVVVUXVVVVUXVV"
    for y in range(6, dh):
        for i, key in enumerate(planks):
            dc.put(dx0 + 4 + i, dy0 + y, key)
    for strap_y in (17, 59):
        dc.fill(dx0 + 4, dy0 + strap_y, dx0 + dw - 5, dy0 + strap_y + 2, "j")
        dc.fill(dx0 + 4, dy0 + strap_y, dx0 + dw - 5, dy0 + strap_y, "k")
        dc.fill(dx0 + 4, dy0 + strap_y + 2, dx0 + dw - 5, dy0 + strap_y + 2, "k")
        for sx in (7, 14, 21, 28, 33):
            dc.put(dx0 + sx, dy0 + strap_y + 1, "n")
    for x, y in ((29, 41), (30, 40), (31, 40), (32, 41), (32, 42), (31, 43), (30, 43),
                 (29, 42)):
        dc.put(dx0 + x, dy0 + y, "k")
    dc.put(dx0 + 30, dy0 + 41, "n")
    dc.fill(dx0 + 4, dy0 + dh - 2, dx0 + dw - 5, dy0 + dh - 1, "U")
    piece("door_plank", "door", [dx0, dy0, dw, dh], [dw // 2, dh],
          "bottom_centre (stands on floor top)", 16)
    grid = make_grid([w, h], [0, 0], layers,
                     ["decal", "accent", "pipes", "box", "door", "sconce"])
    return grid, pieces


TORCH_FLAMES = [
    [
        "................",
        "........R.......",
        ".......RR.......",
        ".......ROR......",
        "......ROOR......",
        "......ROYOR.....",
        ".....ROYYOR.....",
        ".....ROYWYOR....",
        "....ROYWWYOR....",
        "....ROYWWYOR....",
        "....ROYYWYOR....",
        "....RROYYYOR....",
        ".....ROOYOOR....",
        ".....RROOORR....",
        "......RRRRR.....",
        "......RRRRR.....",
    ],
    [
        "....R...........",
        "....RR..........",
        ".....RR.........",
        ".....ROR........",
        ".....ROOR.......",
        "....ROYOR.......",
        "....ROYYOR......",
        "....ROYWYOR.....",
        "....ROYWWYOR....",
        "....ROYWWYOR....",
        "....ROYYWYOR....",
        "....RROYYOOR....",
        ".....ROOOOOR....",
        ".....RROOORR....",
        "......RRRRR.....",
        "......RRRRR.....",
    ],
    [
        "..........Y.....",
        "................",
        "................",
        "........R.......",
        ".......RR.......",
        "......ROOR......",
        "......ROYOR.....",
        ".....ROYYOR.....",
        "....ROYYWYOR....",
        "....ROYWWYOR....",
        "...RROYWWYORR...",
        "...ROOYYWYOOR...",
        "....RROOYOORR...",
        ".....RROOORR....",
        "......RRRRR.....",
        "......RRRRR.....",
    ],
    [
        "................",
        ".........R......",
        "........RR......",
        "........ROR.....",
        ".......ROOR.....",
        "......ROYOR.....",
        "......ROYYOR....",
        ".....ROYWYOR....",
        "....ROYWWYOR....",
        "....ROYWWYOR....",
        "....ROYWWYOR....",
        "....RROYYYORR...",
        ".....ROOYOOR....",
        ".....RROOORR....",
        "......RRRRR.....",
        "......RRRRR.....",
    ],
]

TORCH_SCONCE = [
    "...kghhhhhhhhgk.",
    "...kjnjjjjjjjik.",
    "....kjjjjjjjik..",
    ".....kijjjjik...",
    "......kiijik....",
    ".......kjik.....",
    "........jk......",
    "........jk......",
    ".......kjjk.....",
    "......kljjik....",
    "......kljoik....",
    "......kljjik....",
    "......kljjik....",
    "......kljoik....",
    "......kiiiik....",
    ".......kkkk.....",
]
TORCH_DURATIONS = [120, 140, 100, 130]


def build_torch() -> dict:
    w, h = 16, 32
    frames = []
    for flame, duration in zip(TORCH_FLAMES, TORCH_DURATIONS):
        sconce = Canvas(w, h)
        sconce.stamp(TORCH_SCONCE, 0, 16)
        fl = Canvas(w, h)
        fl.stamp(flame, 0, 0)
        frames.append({"duration_ms": duration,
                       "cels": {"sconce": sconce.rows(), "flame": fl.rows()}})
    return {"format": "pixel-grid-v1", "canvas": [w, h], "pivot": [8, 29],
            "palette": palette_entries(), "layers": ["sconce", "flame"],
            "frames": frames,
            "tags": [{"name": "burn", "from": 0, "to": 3, "direction": "forward"}]}


def make_grid(canvas, pivot, layers: dict[str, Canvas], order: list[str]) -> dict:
    return {"format": "pixel-grid-v1", "canvas": canvas, "pivot": pivot,
            "palette": palette_entries(), "layers": order,
            "frames": [{"duration_ms": 100,
                        "cels": {name: layers[name].rows() for name in order}}],
            "tags": []}


# ---------------------------------------------------------------------------
# Contract and sample layout
# ---------------------------------------------------------------------------
ATLASES = {
    "floor01_terrain": {"grid": "assets/source/terrain/floor01/terrain.grid.json",
                        "manifest": "assets/manifests/floor01_terrain.json",
                        "image": "generated/art/floor01_terrain.png",
                        "metadata": "generated/art/floor01_terrain.json"},
    "floor01_backwall": {"grid": "assets/source/terrain/floor01/backwall.grid.json",
                         "manifest": "assets/manifests/floor01_backwall.json",
                         "image": "generated/art/floor01_backwall.png",
                         "metadata": "generated/art/floor01_backwall.json"},
    "floor01_structure": {"grid": "assets/source/terrain/floor01/structure.grid.json",
                          "manifest": "assets/manifests/floor01_structure.json",
                          "image": "generated/art/floor01_structure.png",
                          "metadata": "generated/art/floor01_structure.json"},
    "floor01_props": {"grid": "assets/source/props/floor01/props.grid.json",
                      "manifest": "assets/manifests/floor01_props.json",
                      "image": "generated/art/floor01_props.png",
                      "metadata": "generated/art/floor01_props.json"},
    "floor01_torch": {"grid": "assets/source/props/floor01/torch.grid.json",
                      "manifest": "assets/manifests/floor01_torch.json",
                      "image": "generated/art/floor01_torch.png",
                      "metadata": "generated/art/floor01_torch.json"},
}

GRAMMAR_HINTS = {
    "floor_top": ["A", "B", "C"], "floor_fill": ["A", "B", "C"],
    "floor_inner": ["A", "B", "C"], "floor_step": ["A", "B", "C"],
    "ceiling": ["A", "B", "C"], "wall_": ["A", "B", "C"], "door": ["A"], "grime": ["A"],
    "brick_backwall": ["A", "B", "C"], "recess": ["B"],
    "arch": ["B"], "pier": ["B"], "far_arcade": ["B"], "torch": ["A", "B", "C"],
    "pilaster": ["A", "B"],
    "pipe": ["C"], "valve": ["C"], "maintenance": ["C"], "lichen": ["C"],
}


def grammar_for(tile_id: str) -> list[str]:
    for prefix, grammar in GRAMMAR_HINTS.items():
        if tile_id.startswith(prefix):
            return grammar
    raise KeyError(tile_id)


def build_contract(terrain_tiles, backwall_tiles, structure_pieces, prop_pieces) -> dict:
    tiles = []
    for atlas, role, collidable, group in (("floor01_terrain", "collision", True, terrain_tiles),
                                           ("floor01_backwall", "backwall", False, backwall_tiles)):
        for index, t in enumerate(sorted(group, key=lambda t: (t["cell"][1], t["cell"][0]))):
            col, row = t["cell"]
            entry = {"id": t["id"], "atlas": atlas, "grid_index": row * 8 + col,
                     "rect": [col * TILE, row * TILE, TILE, TILE], "role": role,
                     "collidable": collidable, "grammar": grammar_for(t["id"])}
            if t.get("walkable_top"):
                entry["walkable_top"] = True
            if "panel" in t:
                entry["panel"] = t["panel"]
            if t.get("recess"):
                entry["recess"] = True
            tiles.append(entry)
    pieces = []
    for atlas, group in (("floor01_structure", structure_pieces), ("floor01_props", prop_pieces)):
        for p in group:
            entry = {"id": p["id"], "atlas": atlas, "rect": p["rect"],
                     "role": p.get("role", "structure"), "collidable": False,
                     "anchor": p["anchor"], "anchor_name": p["anchor_name"],
                     "grid_snap": p["grid_snap"], "grammar": grammar_for(p["id"])}
            for key in ("arch", "repeat", "decal"):
                if key in p:
                    entry[key] = p[key]
            pieces.append(entry)
    frames = []
    for i, duration in enumerate(TORCH_DURATIONS):
        frames.append({"frame": {"x": 16 * i, "y": 0, "w": 16, "h": 32}, "duration": duration})
    pieces.append({"id": "torch_wall", "atlas": "floor01_torch", "rect": [0, 0, 16, 32],
                   "role": "prop", "collidable": False, "anchor": [8, 29],
                   "anchor_name": "wall_mount (bottom rivet of the sconce plate)",
                   "grid_snap": 16, "grammar": grammar_for("torch_wall"),
                   "animation": {"tag": {"name": "burn", "from": 0, "to": 3,
                                         "direction": "forward"},
                                 "frames": frames}})
    return {
        "format": "env-kit-v1",
        "kit_id": KIT_ID,
        "status": "proposed - awaiting human art review",
        "palette_id": PALETTE_ID,
        "palette": palette_entries(),
        "tile_size": [TILE, TILE],
        "coordinate_system": {
            "origin": "top_left", "x_axis": "right", "y_axis": "down",
            "row_order": "top_to_bottom", "units": "virtual_px",
            "note": "Layout pixel (x, y) = (col * 16, row * 16). Atlas rects are [x, y, w, h] "
                    "in exported atlas pixels, top-left origin. Bevy (y-up) must flip y.",
        },
        "layers": [
            {"name": "backwall", "role": "backwall", "kind": "tiles", "draw_order": 0},
            {"name": "backwall_decal", "role": "backwall", "kind": "pieces", "draw_order": 1},
            {"name": "structure", "role": "structure", "kind": "pieces", "draw_order": 2},
            {"name": "prop", "role": "prop", "kind": "pieces", "draw_order": 3},
            {"name": "collision", "role": "collision", "kind": "tiles", "draw_order": 4},
            {"name": "foreground", "role": "foreground", "kind": "pieces", "draw_order": 6},
        ],
        "actor_draw_order": 5,
        "roles": {
            "collision": "Collidable gameplay terrain. Only role that may be collidable; tiles opaque.",
            "backwall": "Non-collidable wall behind actors; tiles opaque, decals use 1-bit alpha.",
            "structure": "Non-collidable midground architecture (arches, piers, far depth); 1-bit alpha.",
            "prop": "Non-collidable wall-mounted props (torches, pipes, valve, box, lichen); 1-bit alpha.",
            "foreground": "Reserved for restrained foreground overlays; none in ENV-01.",
        },
        "floor_contact": {
            "collider_top": "top edge of a walkable_top tile",
            "surface_rows": [0, 4],
            "recommended_visual_foot_sink_px": 0,
            "decision": "Owner decision 2026-11-16: COMP-01 draws actors with their pivot "
                        "(bottom opaque) row directly on the collider top (0 px foot sink).",
            "open_question": "An independent art-director review suggested a 2 px sink so "
                             "feet sit inside the 5 px walking surface; left for human art "
                             "review.",
        },
        "atlases": {name: {k: v for k, v in data.items()} for name, data in ATLASES.items()},
        "tiles": tiles,
        "pieces": pieces,
        "usage_rules": {
            "A": "Ordinary fabric: brick_backwall panels, top/base strips, floor_top_a-d, "
                 "sparse torch_wall + torch_glow at varied spacing (roughly every 10-20 "
                 "columns), an occasional torch_wall_unlit, door_plank or grime_patch_*, and "
                 "at most one plain pilaster_plain as a structural break in a long run.",
            "B": "Rhythm: pier_* stacks 192 px apart (centre to centre) carrying arch_ring_r72; "
                 "recess_* tiles fill the opening; far_arcade_a/b sit on the opening floor "
                 "line (alternate them between neighbouring openings).",
            "C": "Special function, sparing: one pipe run (pipe_*), one valve_wheel, one "
                 "maintenance_box; optional lichen_crust only next to a damp source. Every "
                 "pipe end terminates in a collar (pipe_v_floor_collar, pipe_v_ceiling_collar, "
                 "pipe_wall_collar_r) or the maintenance_box; never in mid-air.",
            "corridor_ends": "Close a corridor with wall_face_l/r (alternate *_b for the second "
                             "quoin phase), ceiling_corner_in_l/r where the ceiling meets it, "
                             "wall_cap_* on top of low walls and ceiling_corner_out_l/r where "
                             "a ceiling course ends.",
            "panels": "brick_backwall panel cells must be used in their native order horizontally "
                      "(cell x+1 follows cell x within a panel, the panel's last column may be "
                      "followed by column 0 of any panel of the same family). Any vertical order is "
                      "seamless.",
            "grid_snap": "Piece top-left positions must be multiples of grid_snap.",
        },
    }


def build_layout(contract: dict) -> dict:
    cols, rows = 128, 25
    ceiling_rows, top_strip_row, base_row, floor_row = (0, 1), 3, 19, 20
    panel_rows = range(4, 19)
    collision = [[None] * cols for _ in range(rows)]
    backwall = [[None] * cols for _ in range(rows)]

    def pick(options, *salt):
        return options[stable_hash(*salt) % len(options)]

    for x in range(cols):
        for r in ceiling_rows:
            collision[r][x] = pick(["ceiling_fill_a", "ceiling_fill_b"], "cf", x, r)
        collision[2][x] = pick(["ceiling_edge_a", "ceiling_edge_b", "ceiling_edge_c",
                                "ceiling_edge_b"], "ce", x)
        collision[floor_row][x] = pick(["floor_top_a", "floor_top_b", "floor_top_c",
                                        "floor_top_b", "floor_top_a", "floor_top_d"], "ft", x)
        for r in range(floor_row + 1, rows):
            collision[r][x] = pick(["floor_fill_a", "floor_fill_b"], "ff", x, r)
    # One 1-tile raised block in segment A (exercises end/side tiles; step <= 2 tiles)
    step0, step1 = 30, 37
    for x in range(step0, step1 + 1):
        if x == step0:
            top, face = "floor_top_end_l", "floor_inner_corner_l"
        elif x == step1:
            top, face = "floor_top_end_r", "floor_inner_corner_r"
        else:
            top, face = pick(["floor_top_a", "floor_top_b", "floor_top_c"], "st", x), \
                pick(["floor_step_base_a", "floor_step_base_b"], "sf", x)
        collision[base_row][x] = top
        collision[floor_row][x] = face

    # Backwall: panels per 4x2 block, never repeating the left or upper neighbour
    families = list("abcdef")
    chosen: dict[tuple[int, int], str] = {}
    for by, r0 in enumerate(range(panel_rows.start, panel_rows.stop, 2)):
        for bx in range(cols // 4):
            banned = {chosen.get((bx - 1, by)), chosen.get((bx, by - 1))}
            options = [f for f in families if f not in banned]
            chosen[(bx, by)] = pick(options, "panel", bx, by)
    for r in panel_rows:
        by = (r - panel_rows.start) // 2
        for x in range(cols):
            fam = chosen[(x // 4, by)]
            cell = (r % 2) * 4 + (x % 4) + 1
            backwall[r][x] = f"brick_backwall_{fam}{cell}"
    for x in range(cols):
        backwall[top_strip_row][x] = f"brick_backwall_top_{x % 4 + 1}"
        backwall[base_row][x] = f"brick_backwall_base_{x % 4 + 1}"

    structure, props, decals = [], [], []
    pier_cols = [56, 68, 100, 112, 124]
    spring_y = 176

    def place_pier(col: int) -> None:
        cx = col * TILE + 24
        structure.append({"piece": "pier_capital", "x": cx - 32, "y": spring_y})
        for i in range(3):
            structure.append({"piece": "pier_shaft", "x": cx - 32, "y": spring_y + 16 + 32 * i})
        structure.append({"piece": "pier_base", "x": cx - 32, "y": spring_y + 112})

    arches = []
    far_variants = ["far_arcade_a", "far_arcade_b", "far_arcade_a"]
    for (left, right), far_id in zip(((56, 68), (100, 112), (112, 124)), far_variants):
        open0, open1 = left + 3, right - 1  # opening columns inclusive
        centre_x = open0 * TILE + 72
        arches.append((open0, open1, centre_x))
        bands = {5: "recess_vault", 6: "recess_vault", 7: "recess_vault", 8: "recess_vault",
                 9: "recess_vault", 10: "recess_vault_fade", 15: "recess_wall_fade",
                 16: "recess_low", 17: "recess_low", 18: "recess_haze", 19: "recess_floor"}
        r_mid = (ARCH_R_IN + ARCH_R_OUT) / 2
        for r in range(4, 20):
            for x in range(open0 - 2, open1 + 3):
                cy = r * TILE + 8
                cxp = x * TILE + 8
                if cy >= spring_y:
                    inside = open0 <= x <= open1
                else:
                    inside = math.hypot(cxp - centre_x, cy - spring_y) < r_mid
                if not inside:
                    continue
                kind = bands.get(r) or pick(["recess_wall_a", "recess_wall_b"], "rw", x, r)
                backwall[r][x] = kind
        structure.append({"piece": far_id, "x": open0 * TILE,
                          "y": (base_row + 1) * TILE - 96})
    for col in pier_cols:
        place_pier(col)
    for open0, open1, centre_x in arches:
        structure.append({"piece": "arch_ring_r72", "x": centre_x - 96, "y": spring_y - 96})

    # A dressing: a few grime/soot patches on the brick and one plain door.
    for col, row in ((3, 14), (21, 6), (38, 12), (45, 15), (90, 13), (9, 8)):
        decals.append({"piece": "grime_patch_a" if (col + row) % 2 else "grime_patch_b",
                       "x": col * TILE, "y": row * TILE})
    props.append({"piece": "door_plank", "x": 12 * TILE, "y": (floor_row - 5) * TILE})
    # One plain stone pilaster breaks the long run of A brick (shaft x 412-435).
    for y in range(top_strip_row * TILE, (base_row + 1) * TILE, 16):
        structure.append({"piece": "pilaster_plain", "x": 408, "y": y})
    # Torches at deliberately uneven spacing; one burnt-out sconce.
    torch_cols = [(7, 10), (18, 10), (33, 10), (50, 10), (82, 10), (94, 10)]
    props.append({"piece": "torch_wall_unlit", "x": 42 * TILE, "y": 10 * TILE})
    for col, row in torch_cols:
        decals.append({"piece": "torch_glow", "x": (col - 1) * TILE, "y": (row - 1) * TILE})
        props.append({"piece": "torch_wall", "x": col * TILE, "y": row * TILE,
                      "frame_offset": stable_hash("torch", col) % 4})
    # Segment C: one pipe run with valve feeding a maintenance box.
    pipe_row, pipe_col = 7, 75
    # lichen first so the pipe overlaps its right edge (growth hugging the pipe)
    props.append({"piece": "lichen_crust", "x": pipe_col * TILE - 8, "y": 11 * TILE})
    props.append({"piece": "pipe_elbow_r_d", "x": pipe_col * TILE, "y": pipe_row * TILE})
    run = {76: "pipe_h_a", 77: "pipe_h_a", 78: "pipe_h_bracket", 79: "pipe_h_rust",
           80: "pipe_h_flange", 81: "pipe_h_a", 82: "pipe_h_a", 83: "pipe_h_bracket",
           84: "pipe_h_a", 85: "pipe_elbow_l_u"}
    for col, pid in run.items():
        props.append({"piece": pid, "x": col * TILE, "y": pipe_row * TILE})
    # the run rises into the ceiling course instead of ending in mid-air
    rise = {6: "pipe_v_a", 5: "pipe_v_bracket", 4: "pipe_v_a", 3: "pipe_v_ceiling_collar"}
    for row, pid in rise.items():
        props.append({"piece": pid, "x": 85 * TILE, "y": row * TILE})
    vertical = {8: "pipe_v_a", 9: "pipe_v_bracket", 10: "valve_wheel", 11: "pipe_v_a",
                12: "pipe_v_a"}
    for row, pid in vertical.items():
        props.append({"piece": pid, "x": pipe_col * TILE, "y": row * TILE})
    props.append({"piece": "maintenance_box", "x": pipe_col * TILE, "y": 13 * TILE})
    for row in range(15, 19):
        props.append({"piece": "pipe_v_a", "x": (pipe_col + 1) * TILE, "y": row * TILE})
    props.append({"piece": "pipe_v_floor_collar", "x": (pipe_col + 1) * TILE, "y": 19 * TILE})

    return {
        "format": "env-kit-layout-v1",
        "layout_id": "floor01_corridor_abc_sample",
        "kit": KIT_ID,
        "kit_contract": "assets/source/terrain/floor01/floor01_catacomb_kit.json",
        "status": "sample layout for art preview and COMP-01 integration; not authored level geometry",
        "tile_size": [TILE, TILE],
        "grid_size": [cols, rows],
        "pixel_size": [cols * TILE, rows * TILE],
        "coordinate_system": "top_left origin, x right, y down, rows top_to_bottom; "
                             "piece x/y = top-left of the piece rect in layout pixels",
        "base_floor_top_y": floor_row * TILE,
        "requirements": {"continuous_floor": True, "max_step_tiles": 2},
        "segments": [
            {"id": "A1", "grammar": "A", "x": [0, 56 * TILE]},
            {"id": "B1", "grammar": "B", "x": [56 * TILE, 71 * TILE]},
            {"id": "C1", "grammar": "C", "x": [71 * TILE, 88 * TILE]},
            {"id": "A2", "grammar": "A", "x": [88 * TILE, 100 * TILE]},
            {"id": "B2", "grammar": "B", "x": [100 * TILE, 128 * TILE]},
        ],
        "markers": [
            {"id": "player_spawn", "kind": "spawn", "x": 4 * TILE + 8,
             "y": floor_row * TILE - 1, "note": "actor pivot pixel (foot contact row)"},
            {"id": "exit_east", "kind": "exit", "x": 118 * TILE + 8,
             "y": floor_row * TILE - 1, "rect": [116 * TILE, 11 * TILE, 9 * TILE, 9 * TILE],
             "note": "walk into the far arch of B2"},
        ],
        "views": {
            "abc_640x360": [46 * TILE, 2 * TILE, 640, 360],
            "abc_640x400": [46 * TILE, 0, 640, 400],
        },
        "preview": {
            "note": "Art-preview inputs only (not runtime data): mood comparison image and "
                    "approved actor atlases composited for a contrast check.",
            "primary_view": "abc_640x360",
            "concept_reference": "assets/concept/floor-01/opening/01-catacomb-environment-clean.png",
            "actor_check": [
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": 800},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": 862},
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": 1012},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": 1262},
            ],
        },
        "tile_layers": {"backwall": backwall, "collision": collision},
        "piece_layers": {"backwall_decal": decals, "structure": structure,
                         "prop": props, "foreground": []},
    }


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
    pal = [f"    {json.dumps(e)}" for e in grid["palette"]]
    lines.append(",\n".join(pal))
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
    text = json.dumps(head, indent=2)[:-2]  # strip closing brace
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--force", action="store_true",
                        help="overwrite existing grids/contract/layout (discards hand edits)")
    args = parser.parse_args(argv)
    outputs = [TERRAIN_DIR / "terrain.grid.json", TERRAIN_DIR / "backwall.grid.json",
               TERRAIN_DIR / "structure.grid.json", PROPS_DIR / "props.grid.json",
               PROPS_DIR / "torch.grid.json", CONTRACT_PATH, LAYOUT_PATH]
    existing = [p for p in outputs if p.exists()]
    if existing and not args.force:
        print("Refusing to overwrite (pass --force):\n  " +
              "\n  ".join(str(p.relative_to(ROOT)) for p in existing), file=sys.stderr)
        return 1
    terrain, terrain_tiles = build_terrain()
    backwall, backwall_tiles = build_backwall()
    structure, structure_pieces = build_structure()
    props, prop_pieces = build_props()
    torch = build_torch()
    dump_grid(terrain, outputs[0])
    dump_grid(backwall, outputs[1])
    dump_grid(structure, outputs[2])
    dump_grid(props, outputs[3])
    dump_grid(torch, outputs[4])
    contract = build_contract(terrain_tiles, backwall_tiles, structure_pieces, prop_pieces)
    dump_json(contract, CONTRACT_PATH)
    dump_layout(build_layout(contract), LAYOUT_PATH)
    for p in outputs:
        print("wrote", p.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
