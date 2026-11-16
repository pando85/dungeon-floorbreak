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
# Shared palette (one palette for the whole kit, 44 opaque colours)
# ---------------------------------------------------------------------------
PALETTE = [
    # Muted red-brown brick (backwall fabric, module A)
    ("m", "brick_mortar", "#251a1f"),
    ("a", "brick_0_shadow", "#312026"),
    ("b", "brick_1_dark", "#3b2428"),
    ("c", "brick_2_base", "#45292a"),
    ("d", "brick_3_light", "#4f302d"),
    ("e", "brick_4_highlight", "#5b3831"),
    # Restrained warm torchlight on brick
    ("f", "torchlit_0", "#6e402e"),
    ("g", "torchlit_1", "#8a5332"),
    ("h", "torchlit_2", "#a86a3a"),
    # Gray / neutral stone edges (floor, ceiling, arches, piers)
    ("s", "stone_0_joint", "#1f1c24"),
    ("t", "stone_1_shadow", "#2e2a33"),
    ("u", "stone_2_dark", "#3d3842"),
    ("v", "stone_3_base", "#4f4851"),
    ("w", "stone_4_light", "#635a61"),
    ("x", "stone_5_edge", "#7c7171"),
    ("y", "stone_6_lip", "#9d8f84"),
    ("z", "stone_7_glint", "#c0ae98"),
    # Dry floor walking surface, warmed by torchlight
    ("D", "floor_surface_0", "#4b3e3c"),
    ("E", "floor_surface_1", "#5b4b46"),
    ("F", "floor_surface_2", "#6c5850"),
    # Cold blue depth (module B recesses)
    ("0", "depth_0_void", "#0e1322"),
    ("1", "depth_1_far_wall", "#141b2f"),
    ("2", "depth_2_far_air", "#1b2641"),
    ("3", "depth_3_haze", "#253553"),
    ("4", "depth_4_far_edge", "#33486b"),
    ("5", "depth_5_far_rim", "#4a6188"),
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
    # Optional lichen accent
    ("G", "lichen_0", "#23402a"),
    ("H", "lichen_1", "#3f7a3a"),
    ("L", "lichen_2_glow", "#92d86a"),
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
        pit = (rng.randint(2, 11), rng.randint(2, 4)) if rng.random() < 0.45 else None
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

def floor_top(c: Canvas, ox: int, oy: int, joints: list[int], seed: str,
              end: str | None = None) -> None:
    """Walkable floor: 5 px warm torch-lit walking surface (rows 0-4), a bright
    neutral stone lip (row 5), a shadow line and a calm dark front face."""
    rng = random.Random(f"{KIT_ID}:{seed}")
    surface = ["F", "E", "E", "D", "D", "y", "s"]
    for y, key in enumerate(surface):
        c.fill(ox, oy + y, ox + 15, oy + y, key)
    c.fill(ox, oy + 7, ox + 15, oy + 14, "D")
    c.fill(ox, oy + 7, ox + 15, oy + 7, "E")
    c.fill(ox, oy + 15, ox + 15, oy + 15, "s")
    # sparse wear on the walking surface, never on the back edge or the lip
    for _ in range(2):
        sx, sy = rng.randint(1, 13), rng.choice((2, 3, 4))
        c.put(ox + sx, oy + sy, "E" if sy == 4 else "D")
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
    # one small pit cluster on the front face
    px, py = rng.randint(3, 11), rng.randint(9, 12)
    if all(abs(px - j) > 2 for j in joints):
        c.put(ox + px, oy + py, "t")
        c.put(ox + px + 1, oy + py, "t")
        c.put(ox + px + 1, oy + py + 1, "E")
    if end == "l":
        for y in range(0, 5):
            c.put(ox, oy + y, "x")
        c.put(ox, oy + 5, "z")
        for y in range(6, 15):
            c.put(ox, oy + y, "w")
            c.put(ox + 1, oy + y, "v")
        c.put(ox, oy + 15, "s")
    elif end == "r":
        for y in range(0, 5):
            c.put(ox + 15, oy + y, "D")
        for y in range(6, 16):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t")


def masonry_fill(c: Canvas, ox: int, oy: int, seed: str, face: str, light: str,
                 joint: str, side: str | None = None,
                 joint_cols: tuple = (15, None)) -> None:
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
        if rng.random() < 0.4:
            px = rng.randint(2, 12)
            if jx is None or px not in (jx - 1, jx, jx + 1):
                c.put(ox + px, y0 + rng.randint(2, 5), joint)
                c.put(ox + px + 1, y0 + rng.randint(2, 5), face)
    if side == "l":
        for y in range(16):
            c.put(ox, oy + y, "w")
            if c.get(ox + 1, oy + y) == face:
                c.put(ox + 1, oy + y, "v")
    elif side == "r":
        for y in range(16):
            c.put(ox + 15, oy + y, "s")
            c.put(ox + 14, oy + y, "t")


def ceiling_edge(c: Canvas, ox: int, oy: int, joints: list[int], seed: str) -> None:
    rng = random.Random(f"{KIT_ID}:{seed}")
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
    px = rng.randint(2, 12)
    if all(abs(px - j) > 1 for j in joints):
        c.put(ox + px, oy + 11, "t")
        c.put(ox + px + 1, oy + 11, "t")


def ceiling_fill(c: Canvas, ox: int, oy: int, seed: str) -> None:
    rng = random.Random(f"{KIT_ID}:{seed}")
    c.fill(ox, oy, ox + 15, oy + 15, "t")
    for k in range(2):
        y0 = oy + 8 * k
        jx = 15 if k == 0 else 7
        c.fill(ox, y0 + 7, ox + 15, y0 + 7, "s")
        for y in range(y0, y0 + 7):
            c.put(ox + jx, y, "s")
        if rng.random() < 0.5:
            c.put(ox + rng.randint(1, 5), y0 + rng.randint(2, 5), "s")


def build_terrain() -> tuple[dict, list[dict]]:
    w, h = 8 * TILE, 2 * TILE
    layers = {name: Canvas(w, h) for name in ("floor", "ceiling", "wall")}
    tiles: list[dict] = []

    def add(name: str, layer: str, col: int, row: int, extra: dict) -> tuple[int, int]:
        tiles.append({"id": name, "layer": layer, "cell": [col, row], **extra})
        return col * TILE, row * TILE

    walk = {"walkable_top": True}
    ox, oy = add("floor_top_a", "floor", 0, 0, walk)
    floor_top(layers["floor"], ox, oy, [15], "floor_top_a")
    ox, oy = add("floor_top_b", "floor", 1, 0, walk)
    floor_top(layers["floor"], ox, oy, [], "floor_top_b")
    ox, oy = add("floor_top_c", "floor", 2, 0, walk)
    floor_top(layers["floor"], ox, oy, [7], "floor_top_c")
    ox, oy = add("floor_top_end_l", "floor", 3, 0, walk)
    floor_top(layers["floor"], ox, oy, [], "floor_top_end_l", end="l")
    ox, oy = add("floor_top_end_r", "floor", 4, 0, walk)
    floor_top(layers["floor"], ox, oy, [], "floor_top_end_r", end="r")
    ox, oy = add("ceiling_edge_a", "ceiling", 5, 0, {})
    ceiling_edge(layers["ceiling"], ox, oy, [15], "ceiling_edge_a")
    ox, oy = add("ceiling_edge_b", "ceiling", 6, 0, {})
    ceiling_edge(layers["ceiling"], ox, oy, [], "ceiling_edge_b")
    ox, oy = add("ceiling_edge_c", "ceiling", 7, 0, {})
    ceiling_edge(layers["ceiling"], ox, oy, [7], "ceiling_edge_c")

    ox, oy = add("floor_fill_a", "floor", 0, 1, {})
    masonry_fill(layers["floor"], ox, oy, "floor_fill_a", "t", "u", "s")
    ox, oy = add("floor_fill_b", "floor", 1, 1, {})
    masonry_fill(layers["floor"], ox, oy, "floor_fill_b", "t", "u", "s", joint_cols=(None, 7))
    ox, oy = add("floor_fill_side_l", "floor", 2, 1, {})
    masonry_fill(layers["floor"], ox, oy, "floor_fill_side_l", "t", "u", "s", side="l", joint_cols=(None, 7))
    ox, oy = add("floor_fill_side_r", "floor", 3, 1, {})
    masonry_fill(layers["floor"], ox, oy, "floor_fill_side_r", "t", "u", "s", side="r")
    ox, oy = add("ceiling_fill_a", "ceiling", 4, 1, {})
    ceiling_fill(layers["ceiling"], ox, oy, "ceiling_fill_a")
    ox, oy = add("ceiling_fill_b", "ceiling", 5, 1, {})
    ceiling_fill(layers["ceiling"], ox, oy, "ceiling_fill_b")
    # Solid collidable brick (vertical walls, raised blocks): brighter and
    # crisper than the backwall so solid geometry never reads as background.
    for i, name in enumerate(("wall_fill_a", "wall_fill_b")):
        ox, oy = add(name, "wall", 6 + i, 1, {})
        strip = Canvas(16, 16)
        brick_strip(strip, 0, 0, 16, 2, name, mortar="a", tones=(3, 4, 5),
                    weights=(3, 4, 3), full_bevel=True)
        # 16-wide strip: the odd course is a single seam brick, so give it a
        # visible joint at x=7 instead and keep the tile self-contained.
        for y in range(8, 15):
            strip.put(7, y, "a")
        for y, row in enumerate(strip.rows()):
            for x, ch in enumerate(row):
                layers["wall"].put(ox + x, oy + y, ch)
    grid = make_grid([w, h], [0, 0], layers, ["ceiling", "wall", "floor"])
    return grid, tiles


# ---------------------------------------------------------------------------
# Backwall tiles (opaque, non-collidable)
# ---------------------------------------------------------------------------

def recess_tile(c: Canvas, ox: int, oy: int, kind: str) -> None:
    def band(y0: int, y1: int, key: str) -> None:
        c.fill(ox, oy + y0, ox + 15, oy + y1, key)

    def checker(y0: int, y1: int, a: str, b: str) -> None:
        for y in range(y0, y1 + 1):
            for x in range(16):
                c.put(ox + x, oy + y, a if (x + y) % 2 == 0 else b)

    if kind == "recess_vault":
        band(0, 15, "0")
        for x, y in ((3, 5), (4, 5), (11, 12), (12, 12)):
            c.put(ox + x, oy + y, "1")
    elif kind == "recess_vault_fade":
        band(0, 7, "0")
        for y in range(8, 12):
            for x in range(16):
                if (x + y) % 2 == 0 or (y >= 10 and (x + y) % 4 == 1):
                    c.put(ox + x, oy + y, "1")
                else:
                    c.put(ox + x, oy + y, "0")
        band(12, 15, "1")
    elif kind in ("recess_wall_a", "recess_wall_b"):
        band(0, 15, "1")
        # far masonry, barely there: two short dark joint dashes and one faint
        # lit block edge per tile (no continuous lines, no scanline read)
        if kind.endswith("a"):
            marks = [(2, 5, 4), (10, 12, 3)]
            lit = (11, 6)
        else:
            marks = [(7, 3, 3), (1, 10, 4)]
            lit = (3, 11)
        for x0, y, n in marks:
            for x in range(x0, x0 + n):
                c.put(ox + x, oy + y, "0")
        c.put(ox + lit[0], oy + lit[1], "2")
        c.put(ox + lit[0] + 1, oy + lit[1], "2")
    elif kind == "recess_wall_fade":
        band(0, 7, "1")
        checker(8, 11, "1", "2")
        band(12, 15, "2")
        for x in range(16):
            if 4 <= x <= 6:
                c.put(ox + x, oy + 3, "0")
    elif kind == "recess_low":
        band(0, 15, "2")
        for x in range(16):
            if 9 <= x <= 11:
                c.put(ox + x, oy + 7, "1")
    elif kind == "recess_haze":
        band(0, 7, "2")
        checker(8, 11, "2", "3")
        band(12, 15, "3")
    elif kind == "recess_floor":
        band(0, 2, "3")
        band(3, 3, "4")
        band(4, 9, "3")
        band(10, 13, "2")
        band(14, 15, "1")
        for x in (2, 10):
            c.put(ox + x, oy + 4, "2")
            c.put(ox + x + 1, oy + 5, "2")
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
    # Top strip under the ceiling: darker first course, contact shadow line.
    s = layers["strips"]
    brick_strip(s, 0, sy, 64, 2, "strip_top", shift_per_course=[-1, 0], weights=(2, 5, 2))
    s.fill(0, sy, 63, sy, "m")
    for x in range(64):
        if x % 2 == 0:
            s.put(x, sy + 1, "a")
    for cx in range(4):
        tiles.append({"id": f"brick_backwall_top_{cx + 1}", "layer": "strips",
                      "cell": [cx, 6],
                      "panel": {"id": "brick_top", "family": "brick_top",
                                "cell": [cx, 0], "size": [4, 1]}})
    # Base strip behind the floor contact line: one darker brick course and a
    # plain, low-detail plinth band that fades into the floor contact shadow.
    brick_strip(s, 64, sy, 64, 1, "strip_base", shift_per_course=[-1], weights=(2, 5, 2))
    s.fill(64, sy + 7, 127, sy + 7, "m")
    s.fill(64, sy + 8, 127, sy + 8, "b")
    s.fill(64, sy + 9, 127, sy + 12, "a")
    for x in range(64, 128):
        s.put(x, sy + 13, "a" if x % 2 == 0 else "m")
    s.fill(64, sy + 14, 127, sy + 15, "m")
    for jx in (64 + 31, 64 + 63):
        for y in range(sy + 8, sy + 13):
            s.put(jx, y, "m")
    for jx in (64 + 0, 64 + 32):
        s.put(jx, sy + 8, "d")
        s.put(jx + 1, sy + 8, "c")
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


def draw_arch(c: Canvas, ox: int, oy: int) -> None:
    cx, cy = 96.0, 96.0
    rng = random.Random(f"{KIT_ID}:arch")
    n = ARCH_VOUSSOIRS
    tones = [rng.choice("uvv") for _ in range(n)]
    tones[n // 2] = "v"  # keystone
    pits = {i: (rng.uniform(0.25, 0.75), rng.uniform(78, 90)) for i in range(n)
            if rng.random() < 0.5}
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
            key = tones[idx]
            if d >= ARCH_R_OUT - 1:
                key = "s"
            elif d < ARCH_R_IN + 1:
                key = "w"
            elif d < ARCH_R_IN + 2:
                key = "u" if key == "v" else "t"
            elif min(dist_lo, dist_hi) < 0.75 and not (
                    dist_lo < 0.75 and idx == 0) and not (dist_hi < 0.75 and idx == n - 1):
                key = "s"  # radial mortar joint (none at the two springing faces)
            elif dist_hi < 1.9:
                key = "w"  # upper-left lit side of each voussoir
            elif dist_lo < 1.9:
                key = "t" if key == "u" else "u"
            elif d >= ARCH_R_OUT - 3 and theta > math.pi / 2:
                key = "w"
            if idx in pits and key in "uv":
                pf, pr = pits[idx]
                if abs(frac - pf) * arc < 1.0 and abs(d - pr) < 1.0:
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
    """One dressed stone block with joint on its right/bottom edge."""
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
    if rng.random() < 0.6 and x1 - x0 > 8:
        px, py = rng.randint(x0 + 3, x1 - 4), rng.randint(y0 + 3, y1 - 4)
        c.put(px, py, lower)
        c.put(px + 1, py, lower)


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


def draw_far_arcade(c: Canvas, ox: int, oy: int) -> None:
    """Distant arcade silhouette inside a recess (sits over recess tiles)."""
    w, h = 144, 96
    spring, r_in = 40, 19
    for py in range(h):
        for px in range(w):
            x, y = px + 0.5, py + 0.5
            key = None
            # far wall around three round-headed far openings
            if y >= 12:
                in_open = False
                for k in range(3):
                    acx = 24 + 48 * k
                    if y < spring:
                        if math.hypot(x - acx, spring - y) < r_in:
                            in_open = True
                    elif abs(x - acx) < r_in:
                        in_open = True
                if not in_open and y < 86:
                    key = "1"
            if key:
                c.put(ox + px, oy + py, key)
    # far rims: faint cold light catching the right edge of each opening
    for k in range(3):
        acx = 24 + 48 * k
        for py in range(12, 86):
            y = py + 0.5
            if y < spring:
                dx = math.sqrt(max(0.0, r_in * r_in - (spring - y) ** 2))
                px = int(acx + dx)
            else:
                px = acx + r_in
            if 0 <= px < w and c.get(ox + px, oy + py) == "1" and py > 22:
                c.put(ox + px, oy + py, "3")
    # base of the far wall, receding into haze
    for px in range(w):
        if c.get(ox + px, oy + 85) == "1" and px % 2 == 0:
            c.put(ox + px, oy + 85, "2")
    # two tiny distant torches (warm points, kept to 2 px each)
    for fx, fy in ((48, 46), (96, 46)):
        c.put(ox + fx - 1, oy + fy, "O")
        c.put(ox + fx - 1, oy + fy + 1, "R")
    # keep the far edge columns clean against the near pier faces
    for py in range(h):
        for px in (0, w - 1):
            if c.get(ox + px, oy + py) == "3":
                c.put(ox + px, oy + py, "1")


def build_structure() -> tuple[dict, list[dict]]:
    w, h = 400, 96
    layers = {name: Canvas(w, h) for name in ("arch", "pier", "depth")}
    draw_arch(layers["arch"], 0, 0)
    draw_pier_capital(layers["pier"], 192, 0)
    draw_pier_shaft(layers["pier"], 192, 16)
    draw_pier_base(layers["pier"], 192, 48)
    draw_far_arcade(layers["depth"], 256, 0)
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
        {"id": "far_arcade", "layer": "depth", "rect": [256, 0, 144, 96],
         "anchor": [0, 96], "anchor_name": "bottom_left (opening floor line)",
         "grid_snap": 16},
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


def build_props() -> tuple[dict, list[dict]]:
    w, h = 128, 80
    layers = {name: Canvas(w, h) for name in ("glow", "pipes", "box", "lichen")}
    p = layers["pipes"]
    pieces: list[dict] = []

    def piece(pid, layer, rect, anchor, anchor_name, snap, role="prop", **extra):
        pieces.append({"id": pid, "layer": layer, "rect": rect, "anchor": anchor,
                       "anchor_name": anchor_name, "grid_snap": snap, "role": role, **extra})

    # pipe_h_a: plain run
    pipe_h(p, 0, 0)
    piece("pipe_h_a", "pipes", [0, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_h_rust: run with a rust bloom along the underside
    pipe_h(p, 16, 0)
    for x, y, k in ((19, 9, "q"), (20, 9, "r"), (21, 9, "q"), (20, 10, "q"), (21, 10, "q"),
                    (26, 10, "q"), (27, 9, "q"), (27, 10, "r"), (28, 10, "q")):
        p.put(x, y, k)
    piece("pipe_h_rust", "pipes", [16, 0, 16, 16], [0, 0], "top_left (tile)", 16)
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
    piece("pipe_h_flange", "pipes", [32, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_h_bracket: wall strap with fixing bolt above
    pipe_h(p, 48, 0)
    p.fill(48 + 6, 3, 48 + 8, 12, "i")
    for y in range(3, 13):
        p.put(48 + 6, y, "k")
        p.put(48 + 8, y, "k")
    p.put(48 + 7, 4, "l")
    p.fill(48 + 6, 1, 48 + 8, 2, "k")
    p.put(48 + 7, 1, "n")
    piece("pipe_h_bracket", "pipes", [48, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_v_a
    pipe_v(p, 64, 0)
    piece("pipe_v_a", "pipes", [64, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_v_bracket
    pipe_v(p, 80, 0)
    p.fill(80 + 3, 6, 80 + 12, 8, "i")
    for x in range(80 + 3, 80 + 13):
        p.put(x, 6, "k")
        p.put(x, 8, "k")
    p.put(80 + 4, 7, "l")
    p.fill(80 + 1, 6, 80 + 2, 8, "k")
    p.put(80 + 1, 7, "n")
    piece("pipe_v_bracket", "pipes", [80, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_elbow_r_d: from the right edge turning down to the bottom edge
    for py in range(16):
        for px in range(16):
            d = math.hypot(px + 0.5 - 16, py + 0.5 - 16)
            band = int(12.5 - d) if 4.5 <= d < 12.5 else None
            if band is not None and 0 <= band < 8:
                p.put(96 + px, py, PIPE_ACROSS[band])
    piece("pipe_elbow_r_d", "pipes", [96, 0, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_wall_entry_r: pipe disappears into a bolted wall plate on the right
    pipe_h(p, 112, 0, 0, 10)
    p.fill(112 + 10, 1, 112 + 14, 14, "j")
    for y in range(1, 15):
        p.put(112 + 10, y, "k")
        p.put(112 + 14, y, "k")
        p.put(112 + 11, y, "l")
    p.fill(112 + 10, 1, 112 + 14, 1, "k")
    p.fill(112 + 10, 14, 112 + 14, 14, "k")
    p.put(112 + 12, 3, "n")
    p.put(112 + 12, 12, "n")
    p.put(112 + 13, 13, "q")
    piece("pipe_wall_entry_r", "pipes", [112, 0, 16, 16], [0, 0], "top_left (tile)", 16)

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
    piece("valve_wheel", "pipes", [0, 16, 16, 16], [0, 0], "top_left (tile)", 16)
    # pipe_v_floor_collar: conduit entering the floor
    pipe_v(p, 16, 16, 0, 15)
    p.fill(16 + 3, 16 + 11, 16 + 12, 16 + 15, "j")
    p.fill(16 + 3, 16 + 11, 16 + 12, 16 + 11, "k")
    for y in range(16 + 11, 16 + 16):
        p.put(16 + 3, y, "k")
        p.put(16 + 12, y, "k")
        p.put(16 + 4, y, "l")
    p.fill(16 + 4, 16 + 12, 16 + 11, 16 + 12, "l")
    piece("pipe_v_floor_collar", "pipes", [16, 16, 16, 16], [0, 0], "top_left (tile)", 16)

    # lichen_patch: tiny optional accent (damp growth below a leaking valve).
    # Column 12 is meant to tuck under a vertical pipe's left outline; row 7
    # follows a mortar joint when placed on the 8 px grid.
    lichen = [
        "................",
        "..........GG....",
        "........GGHHG...",
        ".......GHHLHG...",
        "......GHHHHHG...",
        ".....GGHGHHHG...",
        "....GHG.GGHHG...",
        "..GGGG...GHHG...",
        "..G.......GHG...",
        "..........GHG...",
        "...........GG...",
        "...........G....",
        "................",
        "................",
        "................",
        "................",
    ]
    layers["lichen"].stamp(lichen, 32, 16)
    piece("lichen_patch", "lichen", [32, 16, 16, 16], [12, 0],
          "pipe_hug (column 12 sits under a vertical pipe's left outline)", 8)

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
    # rivets at the corners of the outer frame
    for x, y in ((66, 20), (93, 20), (66, 43), (93, 43)):
        layers["box"].put(x, y, "n")
    piece("maintenance_box", "box", [64, 16, 32, 32], [0, 0],
          "top_left (pipe enters at x 4..11, conduit leaves at x 20..27)", 16)

    # torch_glow: warm light decal over brick faces only. It follows the
    # global brick coursing, so it must be placed on the 16 px grid.
    g = layers["glow"]
    gx0, gy0 = 0, 32
    centre = (24.0, 24.0)
    for py in range(48):
        for px in range(48):
            dy = py + 0.5 - centre[1]
            # light reaches a little further upward than downward
            d = math.hypot(px + 0.5 - centre[0], dy * (0.95 if dy < 0 else 1.2))
            if d >= 23:
                continue
            course = py // 8
            row_in = py % 8
            joint_x = 15 if course % 2 == 0 else 7
            if row_in == 7 or px % 16 == joint_x:
                continue  # mortar stays dark
            top_row = row_in == 0
            if d < 9:
                key = "h" if top_row else "g"
            elif d < 15:
                key = "g" if top_row else "f"
            elif d < 18.5 and not row_in == 6:
                key = "f"
            else:
                # outer falloff: only the lit upper edge of each brick
                key = "f" if top_row and d < 21 else None
            if key:
                g.put(gx0 + px, gy0 + py, key)
    piece("torch_glow", "glow", [0, 32, 48, 48], [24, 24],
          "light_centre (= torch flame centre; place at torch tile -1,-1)", 16,
          role="backwall", decal=True)

    grid = make_grid([w, h], [0, 0], layers, ["glow", "lichen", "pipes", "box"])
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
        "......R.........",
        "......RR........",
        "......ROR.......",
        ".....ROOR.......",
        ".....ROYOR......",
        ".....ROYYOR.....",
        "....ROYYYOR.....",
        "....ROYWWYOR....",
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
        "................",
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
    "ceiling": ["A", "B", "C"], "wall_fill": ["A", "B", "C"],
    "brick_backwall": ["A", "B", "C"], "recess": ["B"],
    "arch": ["B"], "pier": ["B"], "far_arcade": ["B"], "torch": ["A", "B", "C"],
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
            "surface_rows": [0, 3],
            "recommended_visual_foot_sink_px": 2,
            "note": "Rows 0-3 of floor_top_* show the walkable surface; drawing actors with "
                    "their pivot row 2 px below the collider top plants the feet on the "
                    "surface. Proposal only; COMP-01 decides in Bevy.",
        },
        "atlases": {name: {k: v for k, v in data.items()} for name, data in ATLASES.items()},
        "tiles": tiles,
        "pieces": pieces,
        "usage_rules": {
            "A": "Ordinary fabric: brick_backwall panels, top/base strips, floor_top_a/b/c, "
                 "sparse torch_wall + torch_glow (roughly one per 10-16 columns).",
            "B": "Rhythm: pier_* stacks 192 px apart (centre to centre) carrying arch_ring_r72; "
                 "recess_* tiles fill the opening; far_arcade sits on the opening floor line.",
            "C": "Special function, sparing: one pipe run (pipe_*), one valve_wheel, one "
                 "maintenance_box; optional lichen_patch only next to a damp source.",
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
                                        "floor_top_b", "floor_top_a"], "ft", x)
        for r in range(floor_row + 1, rows):
            collision[r][x] = pick(["floor_fill_a", "floor_fill_b"], "ff", x, r)
    # One 1-tile raised block in segment A (exercises end/side tiles; step <= 2 tiles)
    step0, step1 = 30, 37
    for x in range(step0, step1 + 1):
        if x == step0:
            top, face = "floor_top_end_l", "floor_fill_side_l"
        elif x == step1:
            top, face = "floor_top_end_r", "floor_fill_side_r"
        else:
            top, face = pick(["floor_top_a", "floor_top_b", "floor_top_c"], "st", x), \
                pick(["floor_fill_a", "floor_fill_b"], "sf", x)
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
    for left, right in ((56, 68), (100, 112), (112, 124)):
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
        decals_far = {"piece": "far_arcade", "x": open0 * TILE, "y": (base_row + 1) * TILE - 96}
        structure.append(decals_far)
    for col in pier_cols:
        place_pier(col)
    for open0, open1, centre_x in arches:
        structure.append({"piece": "arch_ring_r72", "x": centre_x - 96, "y": spring_y - 96})

    torch_cols = [(8, 10), (20, 10), (33, 10), (50, 10), (82, 10), (94, 10)]
    for col, row in torch_cols:
        decals.append({"piece": "torch_glow", "x": (col - 1) * TILE, "y": (row - 1) * TILE})
        props.append({"piece": "torch_wall", "x": col * TILE, "y": row * TILE,
                      "frame_offset": stable_hash("torch", col) % 4})
    # Segment C: one pipe run with valve feeding a maintenance box.
    pipe_row, pipe_col = 7, 75
    # lichen first so the pipe overlaps its right edge (growth hugging the pipe)
    props.append({"piece": "lichen_patch", "x": pipe_col * TILE - 8, "y": 11 * TILE})
    props.append({"piece": "pipe_elbow_r_d", "x": pipe_col * TILE, "y": pipe_row * TILE})
    run = {76: "pipe_h_a", 77: "pipe_h_a", 78: "pipe_h_bracket", 79: "pipe_h_rust",
           80: "pipe_h_flange", 81: "pipe_h_a", 82: "pipe_h_a", 83: "pipe_h_bracket",
           84: "pipe_h_a", 85: "pipe_wall_entry_r"}
    for col, pid in run.items():
        props.append({"piece": pid, "x": col * TILE, "y": pipe_row * TILE})
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
        "floor_top_y": floor_row * TILE,
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
