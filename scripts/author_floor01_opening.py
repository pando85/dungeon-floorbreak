#!/usr/bin/env python3
"""Authoring record for ENV-02 (design phase 1): Floor 1 opening route pieces.

ENV-02 extends the owner-approved ENV-01 catacomb kit (``env-kit-v2``) with the
pieces the first playable opening needs, and proposes one full-route layout:

- ``assets/source/props/floor01/opening_props.grid.json``   raking shore (CP-03 breakable, 4 states),
  loose masonry block (seated / slipped / falling / landed), hazard grit, timber set and
  barricade (intact / smashed, smashed by the pursuer as presentation + authored events),
  maintenance hatch
- ``assets/source/terrain/floor01/opening_decal.grid.json`` alcove niche (dodge pocket), wall
  cavity behind the loose block
- ``assets/source/props/floor01/guild_exit.grid.json``      guild portal (closed / open), hanging sign
- ``assets/manifests/floor01_opening_props.json`` etc.     one manifest per atlas
- ``assets/source/terrain/floor01/floor01_opening_kit.json`` extension contract (``extends`` ENV-01)
- ``assets/levels/floor01/opening_route_proposal.json``     route proposal, ceiling option R
- ``assets/levels/floor01/opening_route_proposal_ceiling_t.json`` same route, ceiling option T

Same rules as ENV-01: 16 px grid, the ``floor01-catacomb-v2`` palette (no new colours),
the same layers and draw order, 1-bit alpha, deterministic rule-based shading quantised
onto the kit ramps. Nothing is cropped, traced or resampled from concept images.

These are **design keys** (phase 1 of the AGENTS.md art pipeline): Sonnet executes the
production pass from ``docs/02-art/env-02-opening-spec.md``; a separate Opus agent reviews.
The ENV-01 contract, grids and sample layout are not modified.

Usage::

    python3 scripts/author_floor01_opening.py --force   # grids, .ase, manifests, contract, layouts
"""
from __future__ import annotations

import argparse
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_floor01_kit as kit  # noqa: E402

ld = kit.ld
TILE = kit.TILE
canvas, blit, crop = kit.canvas, kit.blit, kit.crop
clamp, rnd = ld.clamp, ld.rnd

KIT_ID = "floor01_opening"
CONTRACT_REL = "assets/source/terrain/floor01/floor01_opening_kit.json"
CONTRACT_PATH = ROOT / CONTRACT_REL
LAYOUTS = {
    "R": ROOT / "assets/levels/floor01/opening_route_proposal.json",
    "T": ROOT / "assets/levels/floor01/opening_route_proposal_ceiling_t.json",
}
ATLAS_FILES = {
    "floor01_opening_props": ("assets/source/props/floor01/opening_props", "decor_atlas"),
    "floor01_opening_decal": ("assets/source/terrain/floor01/opening_decal", "decor_atlas"),
    "floor01_guild": ("assets/source/props/floor01/guild_exit", "decor_atlas"),
    "floor01_soffit": ("assets/source/terrain/floor01/soffit", "tileset"),
    "floor01_shore_dust": ("assets/source/props/floor01/shore_dust", "prop_animation"),
    "floor01_block_impact": ("assets/source/props/floor01/block_impact", "prop_animation"),
    "floor01_splinter_burst": ("assets/source/props/floor01/splinter_burst", "prop_animation"),
}
ATLAS_WIDTH = {"floor01_opening_props": 512, "floor01_opening_decal": 192, "floor01_guild": 272}
ATLAS_ORIGIN = {
    "floor01_soffit": "collision tiles for the two steps of the lowered chase-passage ceiling "
                      "(hanging soffit faces, cornice corners and junctions)",
    "floor01_shore_dust": "3-frame crumb trickle under the loose block",
    "floor01_block_impact": "4-frame dust cloud thrown out by the landing block",
    "floor01_splinter_burst": "4-frame burst of timber splinters",
    "floor01_opening_props": "raking timber shore (CP-03 breakable) in four states, loose masonry "
                             "block (seated/settling/slipped/falling/landed), hazard grit, timber set, "
                             "barricade (low and full-height, intact/smashed) and an iron maintenance hatch",
    "floor01_opening_decal": "alcove niche (dodge pocket) and wall cavity decals",
    "floor01_guild": "guild portal (closed/ajar/open) with warm interior light and a hanging sign "
                     "without text or logos",
}

# ---------------------------------------------------------------------------
# Route geometry (layout px; ENV-01 rows: floor back line y 288, walking line y 308)
# ---------------------------------------------------------------------------
FLOOR_ROW = kit.FLOOR_ROW          # 18
BACK_Y = FLOOR_ROW * TILE          # 288 floor back line
WALK_Y = BACK_Y + TILE + kit.WALK_LINE_Y   # 308
COLS, ROWS = 224, 25
SEGMENTS = [  # id, grammar (kit grammar), handoff beat, purpose, col0, col1
    ("S1_intro", "A", "A", "calm intro: plain brick, one pooled torch, a closed timber door", 0, 40),
    ("S2_arch", "B", "B", "arch bay; the pursuer enters from the gallery through this window", 40, 59),
    ("S3_chase", "C", "C", "maintenance passage: low ceiling (option R/T), copper run, valve, hatch, "
                           "two timber sets the pursuer smashes, two jump humps", 59, 107),
    ("S4_widened", "A", "D", "widened passage: full height, dodge alcove on a 1-tile ledge, timber "
                             "barricade the pursuer charges through (overshoot)", 107, 163),
    ("S5_skirmish", "B", "E", "arch bay where the pursuer wedges; on-foot skirmish with the CP-03 "
                              "raking shore and its fall region", 163, 206),
    ("S6_guild", "A", "E", "guild portal: safe, warm, protected exit", 206, 224),
]
BAYS = [40, 163]                       # arch bay origin columns (S2, S5)
# DOZ-01 interface (decided by a separate Opus agent, 2026-11-17): h120 body, 155 px tall
# including the stack, 259 px long; the charge closes about 82 px over 90 ticks and starts
# only when the gap is >= 140 px. Ceiling >= 164 px above the walking line (>= 200 px
# recommended so smoke and riders read); a refuge the pursuer cannot follow is 64..116 px clear.
PURSUER = {"total_height_px": 155, "length_px": 259, "charge_closes_px": 82, "charge_ticks": 90,
           "charge_min_gap_px": 140, "ceiling_min_px": 164, "ceiling_recommended_px": 200,
           "refuge_clear_height_px": [64, 116], "source": "DOZ-01 decision (Opus), h120"}
HERO_RUN_PX_S = 185          # src/anim.rs RUN_SPEED (greybox); 145 px/s is the slow check
CEILING_OPTIONS = {                    # rows of the lowered ceiling (last row = cornice)
    "R": {"rows": 6, "underside_y": 96, "clearance_px": WALK_Y - 96,
          "fits_pursuer_total_height_max_px": WALK_Y - 96 - 16,
          "note": "recommended: 212 px clear (DOZ-01 asks >= 200 so smoke and riders read); "
                  "57 px above the 155 px pursuer"},
    "T": {"rows": 8, "underside_y": 128, "clearance_px": WALK_Y - 128,
          "fits_pursuer_total_height_max_px": WALK_Y - 128 - 16,
          "note": "tight: 180 px clear (DOZ-01 minimum 164); 25 px above the 155 px pursuer, so "
                  "its smoke flattens against the soffit and the stack may scrape (spark VFX)"},
}
LOW_CEIL = (61, 104)                   # first / last column of the lowered ceiling in S3
HUMPS = [(73, 76, 1), (91, 94, 1)]     # jump humps in S3 (end_l, end_r, height)
LEDGE = (129, 134, 1)                  # alcove ledge in S4
PLATFORMS = HUMPS + [LEDGE]
WALL_TORCHES = [248, 1256, 1768, 2232, 3112]   # pooled wall torches (flame x)
POOL_PICK = ["torch_pool_a", "torch_pool_b_m", "torch_pool_a_m", "torch_pool_b", "torch_pool_a"]
UNLIT_SCONCES = [1584]
SHORE_FOOT_X = 2968                    # CP-03 shore foot (bottom of the timber, on the deck)
SHORE_HEAD = (SHORE_FOOT_X + 52, 168)  # top of the head cap = underside of the loose block
BLOCK_XY = (SHORE_FOOT_X + 40, 120)    # loose block piece top-left (64 x 56)
HAZARD = [SHORE_FOOT_X + 32, 168, 80, WALK_Y - 168]   # fall region rect (x, y, w, h)
STRIKE = {"x": SHORE_FOOT_X + 7, "y_band": [WALK_Y - 37, WALK_Y - 18]}
GUILD_X = 3368                         # portal piece left x (112 wide)
TIMBER_SETS = [1104, 1424]             # timber set piece x (80 wide) in S3
BARRICADE_X = 2304                     # barricade piece x (96 wide) at the end of S4
BARRICADE_FULL_Y = 32                  # full-height barricade piece y (96 x 256, closes up to the cornice)
HATCH_XY = (1200, 208)
ALCOVE_XY = (2064, 176)
CHARGE_X = 2064 - 176                  # charge trigger: alcove entrance 176 px ahead


# ---------------------------------------------------------------------------
# Small drawing helpers
# ---------------------------------------------------------------------------
TIMBER = [("wood", 0), ("wood", 1), ("wood", 2), ("wood", 3), ("wood", 4), ("wood", 5),
          ("floor", 7), ("floor", 8)]
SPLINTER = [("stone", 6), ("stone", 7), ("stone", 8), ("stone", 9)]
ROT = [("mortar", 1), ("mortar", 2), ("mortar", 3), ("iron", 3), ("iron", 4), ("iron", 5)]


def lut(c, x: int, y: int, table, v: float) -> None:
    r, i = table[int(clamp(round(v), 0, len(table) - 1))]
    c.put(x, y, r, i)


def clear(c, x: int, y: int) -> None:
    c.clear(x, y)


def beam_coords(px: float, py: float, foot, head):
    """(t along foot->head in 0..1, s across in px, length) for a pixel centre."""
    fx, fy = foot
    hx, hy = head
    length = math.hypot(hx - fx, hy - fy)
    ux, uy = (hx - fx) / length, (hy - fy) / length
    dx, dy = px - fx, py - fy
    return (dx * ux + dy * uy) / length, dx * -uy + dy * ux, length


def timber_value(t: float, s: float, half: float, length: float, seed: str, lit_side: int) -> float:
    """Rough-sawn timber: lit long edge, dark far edge, grain streaks, a knot or two."""
    edge_lit = (s * lit_side) < -(half - 1.5)
    edge_dark = (s * lit_side) > (half - 1.5)
    streak = rnd(seed, int(s + half), int(t * length / 7)) - 0.5
    v = 3.6 + (1.6 if edge_lit else 0.0) - (1.4 if edge_dark else 0.0) + streak * 0.9
    v -= 0.5 * abs(s) / half
    knot = rnd(seed, "knot", int(t * length / 24))
    kt = (t * length) % 24
    if knot > 0.6 and 10 < kt < 13 and abs(s) < 2:
        v -= 1.6
    return v


# ---------------------------------------------------------------------------
# CP-03 breakable: raking timber shore under a slumped masonry block
# ---------------------------------------------------------------------------
SHORE_W, SHORE_H = 136, 144
SHORE_ORIGIN = (SHORE_FOOT_X - 16, BACK_Y - 128)   # (2952, 160); foot local (16, 138)
FOOT_L = (16.0, 138.0)
HEAD_L = (SHORE_HEAD[0] - SHORE_ORIGIN[0] + 0.0, SHORE_HEAD[1] - SHORE_ORIGIN[1] + 4.0)
SHORE_HALF = 5.5
STRIKE_T = 0.15        # where Carl's fist/kick lands (18-37 px above the walking line)
ROT_T = 0.30           # dry-rotted lower part


def sole_plate(c, slip: int = 0) -> None:
    """Sole plate on the deck plus a driven wedge under the shore foot."""
    fx, fy = FOOT_L
    for py in range(int(fy) - 1, int(fy) + 5):
        for px in range(int(fx) - 13, int(fx) + 15):
            top = py == int(fy) - 1
            v = 4.4 if top else 2.4 - 0.4 * (py - fy)
            if px in (int(fx) - 13, int(fx) + 14):
                v -= 1.0
            v += (rnd("sole", px // 5) - 0.5) * 0.8
            lut(c, px, py, TIMBER, v)
    for py in range(int(fy) - 6, int(fy) - 1):    # wedge, left of the foot
        w = (int(fy) - 1 - py) * 2
        for px in range(int(fx) - 9, int(fx) - 9 + w):
            lut(c, px, py, TIMBER, 4.8 if px == int(fx) - 9 + w - 1 else 3.2)


def rot_profile(t: float, length: float) -> tuple[float, float]:
    """How much the dry rot has eaten into each long edge (left, right) at t: a neck of
    about 2.5 px at the strike band, plus 1-2 px bites in irregular 2-3 px runs, worse toward
    the foot. Zero above ROT_T. The foot is the same in every state (keys are re-cut on it)."""
    if t >= ROT_T:
        return 0.0, 0.0
    pr = clamp((ROT_T - t) / ROT_T, 0, 1)
    fade = clamp((ROT_T - t) / 0.06, 0, 1)
    neck = 1.3 * math.exp(-((t - STRIKE_T) / 0.06) ** 2)
    seg = int(t * length / (2 + rnd("bseg", int(t * length / 7)) * 2))
    p = 0.12 + 0.35 * pr + 0.35 * math.exp(-((t - STRIKE_T) / 0.07) ** 2)
    bl = (1 if rnd("biteL", seg) < p else 0) + (1 if rnd("biteL2", seg) < p * 0.4 else 0)
    br = (1 if rnd("biteR", seg) < p * 0.8 else 0) + (1 if rnd("biteR2", seg) < p * 0.3 else 0)
    return (neck + bl) * fade, (neck + br) * fade


def rot_pixel(c, px: int, py: int, t: float, s: float, lo: float, hi: float, length: float,
              bitten: tuple[bool, bool]) -> None:
    """Dry-rotted foot: warm wood_02/03 (never metal grey), soft streaks along the grain,
    dark pits, pale stone_06 torn fibres on the eaten edges; no regular banding."""
    pr = clamp((ROT_T - t) / ROT_T, 0, 1)
    edge_l, edge_r = s - lo < 1.0, hi - s <= 1.0
    if edge_l and bitten[0]:
        lut(c, px, py, TIMBER, 1)                      # dark rim of a bite (cavity depth)
        return
    if edge_r and bitten[1]:
        lut(c, px, py, SPLINTER, 0 if rnd("rfr", px, py) < 0.7 else 1)   # torn pale fibres
        return
    if rnd("rpit", px, py) < 0.05 + 0.07 * pr:
        lut(c, px, py, TIMBER, 0.6)
        return
    grain = rnd("rgr", int(s + 6), int(t * length / (3 + 3 * rnd("rgl", int(s + 6))))) - 0.5
    if rnd("rfib", int(s + 6), int(t * length / 3)) < 0.06 + 0.16 * pr:
        lut(c, px, py, SPLINTER, 0)                    # loose fibre strands along the grain
        return
    v = 2.9 + 0.8 * grain - 0.4 * pr + (1.2 if edge_r else 0.0) - (0.8 if edge_l else 0.0)
    lut(c, px, py, TIMBER, v)


def foot_crumbs(c) -> None:
    """Soft, crumbling base: rotten crumbs and fibres spilled on the sole plate."""
    fx, fy = FOOT_L
    for k in range(14):
        x = int(fx - 7 + rnd("fcx", k) * 18)
        y = int(fy - 2 + rnd("fcy", k) * 2.5)
        if k % 3 == 0:
            lut(c, x, y, SPLINTER, 0)
        else:
            lut(c, x, y, TIMBER, 2 if k % 2 else 3)


def shore_beam(c, state: str) -> None:
    fx, fy = FOOT_L
    hx, hy = HEAD_L
    bow = {"intact": 0.0, "damaged_1": 0.0, "damaged_2": 1.6, "broken": 0.0}[state]
    slip = {"intact": 0, "damaged_1": 0, "damaged_2": 2, "broken": 0}[state]
    head = (hx, hy + slip)
    for py in range(c.h):
        for px in range(c.w):
            x, y = px + 0.5, py + 0.5
            t, s, length = beam_coords(x, y, (fx, fy), head)
            s -= bow * math.sin(math.pi * clamp(t, 0, 1))
            if not 0.0 <= t <= 1.0:
                continue
            cut_l, cut_r = rot_profile(t, length)
            lo, hi = -SHORE_HALF + cut_l, SHORE_HALF - cut_r
            if t < 0.035:                      # soft, splayed, crumbling base
                lo, hi = lo - 0.8, hi + 0.8
                if (s - lo < 1.3 or hi - s < 1.3) and rnd("crumb_base", px, py) < 0.4:
                    continue
            if not lo <= s < hi:
                continue
            if state == "broken" and t > STRIKE_T - 0.01 - 0.03 * (s + SHORE_HALF) / SHORE_HALF \
                    + 0.05 * rnd("jag", int(s + 5)):
                continue                       # only the jagged stub stays standing
            v = timber_value(t, s, SHORE_HALF, length, "shore", lit_side=-1) + 0.6
            if t < ROT_T and not (t > ROT_T - 0.04 and rnd("rotmix", px, py) < 0.5):
                rot_pixel(c, px, py, t, s, lo, hi, length, (cut_l >= 0.9, cut_r >= 0.9))
                continue
            lut(c, px, py, TIMBER, v)
            # a long drying check up the shaft (grain split): reads as "old timber"
            if abs(s - 0.8) < 0.5 and 0.35 < t < 0.62:
                lut(c, px, py, TIMBER, 0.5)
    # damage: notch / split at the strike point, pale fresh fibres, crack up the grain
    if state in ("damaged_1", "damaged_2"):
        depth = 4.6 if state == "damaged_1" else 7.0
        span = 0.047 if state == "damaged_1" else 0.05
        for py in range(c.h):
            for px in range(c.w):
                x, y = px + 0.5, py + 0.5
                t, s, length = beam_coords(x, y, (fx, fy), head)
                s -= bow * math.sin(math.pi * clamp(t, 0, 1))
                if not (-SHORE_HALF <= s < SHORE_HALF):
                    continue
                dt = abs(t - STRIKE_T)
                cut = (s + SHORE_HALF) < depth * (1 - dt / span) if dt < span else False
                if cut:
                    clear(c, px, py)
                    continue
                # pale fresh fibres ringing the notch
                if dt < span + 0.012 and (s + SHORE_HALF) < depth + 1.2 and c.get(px, py):
                    lut(c, px, py, SPLINTER, 2.6 if rnd("fib", px, py) > 0.3 else 1)
                if state == "damaged_2" and STRIKE_T < t < 0.58 and abs(s - (-0.6 + 6 * (t - STRIKE_T))) < 0.55 \
                        and abs(s) < SHORE_HALF - 0.6:
                    lut(c, px, py, ROT, 0)     # split running up the grain
        # splinters sticking out of the notch (lit side)
        n = 4 if state == "damaged_1" else 6
        for k in range(n):
            t = STRIKE_T + (rnd("spl", state, k) - 0.5) * span * 1.4
            length = math.hypot(head[0] - fx, head[1] - fy)
            ux, uy = (head[0] - fx) / length, (head[1] - fy) / length
            bx, by = fx + ux * t * length - (-uy) * (SHORE_HALF - depth), fy + uy * t * length - ux * (SHORE_HALF - depth)
            for j in range(2 + int(rnd("spl_l", state, k) * 3)):
                c.put(int(bx - 1 - j), int(by - (j if k % 2 else 0) * 0.5), *SPLINTER[3 if j == 0 else 2])
    # head cap (a short board under the block); it falls with the block in the broken state
    for py in (range(int(head[1]) - 4, int(head[1])) if state != "broken" else ()):
        for px in range(int(head[0]) - 9, int(head[0]) + 9):
            v = 4.6 if py == int(head[1]) - 4 else 3.0
            if px in (int(head[0]) - 9, int(head[0]) + 8):
                v -= 1.2
            lut(c, px, py, TIMBER, v)


def fallen_upper(c) -> None:
    """Broken state: the upper shaft lies on the deck (right of the stub), lit from above."""
    a, b = (30.0, 136.0), (122.0, 131.0)
    for py in range(c.h):
        for px in range(c.w):
            t, s, length = beam_coords(px + 0.5, py + 0.5, a, b)
            if not (0.0 <= t <= 1.0 and -3.5 <= s < 3.5):
                continue
            if t < 0.04 and rnd("fjag", int(s + 4)) > 0.4:
                continue
            v = 3.3 + (1.7 if s < -2.4 else 0) - (1.6 if s > 2.4 else 0) \
                + (rnd("fall", int(s + 4), int(t * length / 7)) - 0.5) * 0.8
            lut(c, px, py, TIMBER, v)
    for px in range(28, 34):           # pale broken end
        for py in range(132, 140):
            if c.get(px, py) and rnd("fend", px, py) > 0.35:
                lut(c, px, py, SPLINTER, 2)


def shore(state: str) -> "ld.Canvas":
    c = canvas(SHORE_W, SHORE_H)
    sole_plate(c)
    shore_beam(c, state)
    foot_crumbs(c)
    if state == "broken":
        fallen_upper(c)
        # splintered top of the stub
        fx, fy = FOOT_L
        for k in range(5):
            x = int(fx + 3 + k * 1.2 - 2)
            y = int(fy - STRIKE_T * 136 - 1 - rnd("stub", k) * 4)
            for j in range(3):
                c.put(x, y + j, *SPLINTER[3 - min(j, 2)])
    return c


# --- loose masonry block --------------------------------------------------------------
# A heavy 3D chunk of masonry torn half out of the wall: an irregular front face of three
# dressed stones (block-local px), extruded up-right so a cold top face and a torch-lit
# right face show, with a dark underside. The S5 wall torch sits right of and below it.
BLOCK_W, BLOCK_H = 64, 56
FRONT = [(2, 13), (9, 11), (12, 13), (18, 9), (26, 9), (28, 11), (34, 8), (47, 9), (54, 11),
         (56, 15), (56, 39), (52, 43), (36, 44), (31, 42), (24, 44), (11, 44), (6, 41), (4, 35),
         (1, 33), (3, 27), (1, 20)]
EXTRUDE = (6.0, -5.0)          # depth direction of the chunk (shows the top and right faces)
PIVOT = (56.0, 12.0)           # still keyed into the wall at the upper right while it slips
BLOCK_CRACK = [(44, 10), (41, 17), (43, 23), (39, 30), (41, 37), (38, 44)]
BLOCK_HAIR = [(10, 13), (14, 19), (13, 23)]
BLOCK_STATES = {               # state: (tilt degrees, drop px, crumbs)
    "seated": (0.0, 0.0, 0), "settling": (-1.5, 1.0, 4), "slipped": (-4.0, 3.0, 10)}


def in_poly(x: float, y: float, poly=FRONT) -> bool:
    inside = False
    n = len(poly)
    for i in range(n):
        x0, y0 = poly[i]
        x1, y1 = poly[(i + 1) % n]
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            inside = not inside
    return inside


def near_line(x: float, y: float, line, width: float = 0.75) -> float | None:
    """Signed horizontal offset to a polyline (None when farther than width)."""
    for (x0, y0), (x1, y1) in zip(line, line[1:]):
        if min(y0, y1) <= y <= max(y0, y1) and y1 != y0:
            cx = x0 + (y - y0) * (x1 - x0) / (y1 - y0)
            if abs(x - cx) <= width:
                return x - cx
    return None


def chunk_sample(x: float, y: float):
    """(ramp, value) of the chunk at block-local (x, y), or None outside it."""
    if in_poly(x, y):
        joint_x = 29 + (y - 10) * 0.08
        if abs(x - joint_x) < 0.6 or (x < joint_x and abs(y - 27) < 0.6):
            return ("mortar", 1)
        c = near_line(x, y, BLOCK_CRACK)
        if c is not None:
            return ("mortar", 0) if c > -0.25 else ("stone", 9.6)   # lit lip on the torch side
        if near_line(x, y, BLOCK_HAIR, 0.5) is not None:
            return ("mortar", 1)
        if near_line(x, y, BLOCK_CRACK, 2.2) is not None and rnd("crumb", int(x), int(y)) < 0.3:
            return ("mortar", 5)                           # crumbling mortar along the split
        stone = 0 if x < joint_x and y < 27 else 1 if x < joint_x else 2
        v = 4.5 + 2.4 * (x / 56) + (rnd("bst", stone) - 0.5) * 0.9
        # bevels: torch-facing (right / lower) stone edges catch light, upper / left edges dim
        if not in_poly(x + 1.5, y) or abs(x + 1.2 - joint_x) < 0.6:
            v += 1.4
        if not in_poly(x, y + 1.5) or (x < joint_x and abs(y + 1.2 - 27) < 0.6):
            v += 1.0
        if not in_poly(x - 1.2, y) or not in_poly(x, y - 1.2):
            v -= 1.0
        if rnd("bpit", int(x), int(y)) < 0.05:
            v -= 1.5
        elif rnd("bfleck", int(x), int(y)) < 0.04:
            v += 0.9
        return ("stone", v)
    # extruded volume: the cold top face or the torch-lit right face
    for k in range(1, 9):
        t = k / 8
        qx, qy = x - EXTRUDE[0] * t, y - EXTRUDE[1] * t
        if in_poly(qx, qy):
            if not in_poly(qx + 1.0, qy) and in_poly(qx, qy - 1.0):
                return ("stone", 8.6 - 2.2 * t + (0.6 if rnd("rf", int(y) // 3) > 0.6 else 0))
            if k == 1:
                return ("stone", 6.6)                      # front lip of the top face
            return ("stone", 3.6 - 1.2 * t)                # top face, turned away from the torch
    # underside: a short shadowed soffit under the bed, seen from below
    for k in range(1, 4):
        if in_poly(x, y - k):
            return ("stone_cold", 0.8) if k < 3 else ("mortar", 0)
    return None


def chunk(c, tilt: float, drop: float, ox: float = 0.0, oy: float = 0.0) -> None:
    """Rasterise the chunk rotated by ``tilt`` degrees about PIVOT and dropped ``drop`` px."""
    a = math.radians(tilt)
    ca, sa = math.cos(a), math.sin(a)
    for py in range(c.h):
        for px in range(c.w):
            dx, dy = px + 0.5 - ox - PIVOT[0], py + 0.5 - oy - drop - PIVOT[1]
            sx = dx * ca + dy * sa + PIVOT[0]
            sy = -dx * sa + dy * ca + PIVOT[1]
            sample = chunk_sample(sx, sy)
            if sample:
                c.put(px, py, *sample)


def crumbs(c, n: int, seed: str, y0: int, spread: int = 0) -> None:
    """Mortar crumbs spilling from the crack and the bed joint."""
    for k in range(n):
        x = 30 + int(rnd(seed, k) * 16) - spread // 2 + int(rnd(seed, "s", k) * spread)
        y = y0 + int(rnd(seed, "y", k) * 7)
        c.put(x, y, "mortar", 6 if k % 3 else 5)
        if k % 2:
            c.put(x + 1, y, "stone", 8)


def loose_block(state: str) -> "ld.Canvas":
    c = canvas(BLOCK_W, BLOCK_H)
    tilt, drop, n = BLOCK_STATES[state]
    chunk(c, tilt, drop)
    if n:
        crumbs(c, n, state, 47 + int(drop), spread=8 if state == "slipped" else 0)
    return c


def falling_block() -> "ld.Canvas":
    """The chunk tipping as it drops (12 degrees), with crumbs trailing above it."""
    c = canvas(BLOCK_W + 8, BLOCK_H + 8)
    chunk(c, -12.0, 2.0, ox=4.0, oy=4.0)
    for k in range(7):
        x, y = 8 + int(rnd("fb", k) * 56), int(rnd("fby", k) * 5)
        c.put(x, y, "mortar", 6)
        c.put(x + 1, y, "stone", 8 if k % 2 else 6)
    return c


RUBBLE_RIGHT = [(48, 30), (47, 24), (50, 18), (49, 12), (55, 7), (64, 4), (73, 5), (78, 1), (87, 4),
                (93, 10), (95, 17), (93, 23), (95, 27), (95, 30)]
RUBBLE_RIGHT_CRACK = [(67, 4), (65, 12), (68, 20), (63, 30)]
RUBBLE_LEFT_LOW = [(3, 30), (2, 25), (6, 20), (17, 17), (30, 18), (41, 21), (46, 26), (45, 30)]
RUBBLE_LEFT_UP = [(9, 19), (10, 11), (15, 6), (24, 3), (33, 6), (38, 12), (39, 18), (35, 21), (15, 21)]


def rubble_stone(c, poly, seed: str, crack=None, base: float = 5.0) -> None:
    """One landed chunk: torch-lit top and right bevels, dim left bevel, shadowed foot."""
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    for py in range(y0, y1 + 1):
        for px in range(x0, x1 + 1):
            x, y = px + 0.5, py + 0.5
            if not in_poly(x, y, poly):
                continue
            if crack:
                cr = near_line(x, y, crack)
                if cr is not None:
                    c.put(px, py, "mortar", 0) if cr > -0.25 else c.put(px, py, "stone", 9.6)
                    continue
                if near_line(x, y, crack, 2.0) is not None and rnd("rcr", px, py) < 0.3:
                    c.put(px, py, "mortar", 5)
                    continue
            v = base + 1.6 * (x - x0) / (x1 - x0) + (rnd(seed, px // 4, py // 4) - 0.5) * 1.0
            v -= 1.4 * (y - y0) / (y1 - y0)
            if not in_poly(x, y - 2.0, poly):
                v = 8.5 - 0.02 * x                       # lit top face
            elif not in_poly(x - 1.4, y, poly):
                v -= 1.0
            elif not in_poly(x + 1.4, y, poly):
                v += 1.0
            if rnd("rpit", px, py) < 0.05:
                v -= 1.5
            c.put(px, py, "stone", v)


def landed_rubble() -> "ld.Canvas":
    """Broken masonry on the deck, following the v2 split of the block: the right stone
    (with the big crack) lying on its side and the left pair (two stones, one on the other),
    plus crumbs, a few bricks and a dust skirt. Non-collidable."""
    c = canvas(96, 32)
    for py in range(32):                 # dust skirt first, chunks cover it
        for px in range(96):
            y = py + 0.5
            if y > 23 and rnd("dust", px, py) < 0.55 * (1 - abs(px + 0.5 - 48) / 56):
                c.put(px, py, "stone", 6.5 if rnd("dc", px) > 0.5 else 5)
    rubble_stone(c, RUBBLE_LEFT_LOW, "rl2")
    rubble_stone(c, RUBBLE_LEFT_UP, "rl1")
    for px in range(10, 36):             # the bed joint between the two left stones
        if in_poly(px + 0.5, 21.5, RUBBLE_LEFT_LOW) and in_poly(px + 0.5, 20.5, RUBBLE_LEFT_UP):
            c.put(px, 21, "mortar", 1)
    rubble_stone(c, RUBBLE_RIGHT, "rr", crack=RUBBLE_RIGHT_CRACK, base=5.2)
    for bx, by in ((0, 26), (44, 25), (45, 28), (86, 28), (20, 28)):      # loose bricks, the spill
        for py in range(by, min(by + 4, 30)):
            for px in range(bx, bx + 7):
                if not c.get(px, py):
                    c.put(px, py, "brick", 7.4 if py == by else 5.2 - (0.8 if px == bx + 6 else 0))
    for k in range(30):                  # crumbs around the heap
        x = 2 + int(rnd("rcx", k) * 92)
        y = 16 + int(rnd("rcy", k) * 15)
        if not c.get(x, y):
            c.put(x, y, "mortar", 6 if k % 3 else 5)
            if k % 2 and not c.get(x + 1, y):
                c.put(x + 1, y, "stone", 8)
    return c


def hazard_grit() -> "ld.Canvas":
    """Pale grit, 2x2 chips and a scuff line lying where bits fell: an in-world 'danger below'
    tell. Densest under the block (piece x 48), with hard ends at the fall region (x 8 and 88)."""
    c = canvas(96, 16)
    for py in range(16):
        for px in range(8, 88):
            w = max(0.0, 1 - abs(px + 0.5 - 48) / 40)
            r = rnd("grit2", px, py)
            if r < 0.034 + 0.06 * w * w:
                c.put(px, py, "stone", 8 if r < 0.06 else 7 if r < 0.12 else 6)
            elif r > 0.97:
                c.put(px, py, "brick", 6)
    for k in range(30):                   # 2x2 pale chips with a 1 px shade, weighted to the middle
        x = 9 + int((0.5 + (rnd("chipx", k) - 0.5) * (1.0 - 0.45 * rnd("chipw", k))) * 76)
        y = 2 + int(rnd("chipy", k) * 11)
        x = min(max(x, 8), 85)
        for dx in range(2):
            for dy in range(2):
                c.put(x + dx, y + dy, "stone", 9 if (dx == 0 and dy == 0) else 8)
        c.put(x + 2, y + 1, "stone", 4)
        c.put(x, y + 2, "stone", 3)
    y0 = 10                               # a scuff line dragged through the grit
    for x in range(30, 64):
        y = y0 + (1 if (x // 7) % 2 else 0)
        c.put(x, y, "mortar", 4 if x % 3 else 3)
        if x % 5 == 0:
            c.put(x, y - 1, "stone", 5)
    return c


def wall_cavity() -> "ld.Canvas":
    """Backwall decal: the socket the chunk is tearing out of, drawn with depth. A lit left
    inner reveal of broken brick ends (it faces the torch), a shadowed top reveal, a dark
    recessed back with faint bricks, a lit sill, ragged edges and cracks. With the chunk
    seated only uneven slivers show (wide above, a reveal on the left, nothing on the
    right); after the fall the whole socket reads as a hole into the wall."""
    c = canvas(80, 64)
    ox, oy = 8, 8          # block-local (0, 0) sits at decal (8, 8)

    def hole(x: float, y: float) -> bool:      # block-local
        grow_top = 2 + 3 * rnd("cvt", int(x) // 5)
        grow_left = 1 + 2 * rnd("cvl", int(y) // 4)
        if y > 44.5:
            return False
        return (in_poly(x, y) or in_poly(x - EXTRUDE[0] * 0.6, y - EXTRUDE[1] * 0.6)
                or in_poly(x, y + grow_top) or in_poly(x + grow_left, y))

    for py in range(64):
        for px in range(80):
            x, y = px + 0.5 - ox, py + 0.5 - oy
            if not hole(x, y):
                continue
            left_reveal = any(not hole(x - k, y) for k in (1, 2, 3, 4))
            top_reveal = any(not hole(x, y - k) for k in (1, 2, 3))
            if not hole(x, y + 1.0):
                c.put(px, py, "brick", 6.2)                      # lit sill
            elif left_reveal and not top_reveal:
                broken = (int(y) % 8 == 0) or rnd("lrv", int(y) // 8, int(x)) < 0.15
                if broken:
                    c.put(px, py, "mortar", 2)
                else:
                    depth = sum(1 for k in (1, 2, 3) if hole(x - k, y))
                    c.put(px, py, "brick", 7.0 - 0.9 * depth)
            elif top_reveal:
                c.put(px, py, "brick", 1.0 if rnd("trv", px, py) > 0.2 else 2.0)
            else:
                # recessed back wall: faint cold bricks, darker toward the upper left
                gy = int(y + 2)
                row = gy // 6
                joint = gy % 6 == 0 or (int(x) + 6 * (row % 2)) % 12 == 0
                if joint:
                    c.put(px, py, "mortar", 0)
                else:
                    c.put(px, py, "brick", 0.4 + 0.012 * (x + y))
    cracks = [[(6, 30), (2, 35), (1, 43)], [(73, 10), (77, 5), (79, 0)], [(25, 52), (23, 58), (25, 63)],
              [(55, 53), (59, 60)], [(10, 9), (5, 4), (4, 0)]]
    for line in cracks:
        for (x0, y0), (x1, y1) in zip(line, line[1:]):
            steps = max(abs(x1 - x0), abs(y1 - y0))
            for k in range(steps + 1):
                x = round(x0 + (x1 - x0) * k / steps)
                y = round(y0 + (y1 - y0) * k / steps)
                if 0 <= x < 80 and 0 <= y < 64 and not c.get(x, y):
                    c.put(x, y, "mortar", 0)
    return c


# ---------------------------------------------------------------------------
# Timber the pursuer smashes (presentation + authored events)
# ---------------------------------------------------------------------------

def plank(c, x0: float, y0: float, x1: float, y1: float, half: float, seed: str,
          lit_side: int = 1, nails: bool = True, bias: float = 0.0) -> None:
    for py in range(max(0, int(min(y0, y1) - half - 2)), min(c.h, int(max(y0, y1) + half + 2))):
        for px in range(max(0, int(min(x0, x1) - half - 2)), min(c.w, int(max(x0, x1) + half + 2))):
            t, s, length = beam_coords(px + 0.5, py + 0.5, (x0, y0), (x1, y1))
            if 0 <= t <= 1 and -half <= s < half:
                lut(c, px, py, TIMBER, timber_value(t, s, half, length, seed, lit_side) + bias)
    if nails:
        for t in (0.06, 0.94):
            nx, ny = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            c.put(int(nx), int(ny), "iron", 5)
            c.put(int(nx) + 1, int(ny) + 1, "iron", 1)


def timber_set(state: str) -> "ld.Canvas":
    """Mine-style shoring set: two posts and a header hanging about 104 px above the deck
    back line (about 120 px above the walking line): every pursuer size smashes it."""
    c = canvas(80, 104)
    if state == "intact":
        plank(c, 8, 103, 8, 8, 4.0, "tsl", nails=False)
        plank(c, 72, 103, 72, 8, 4.0, "tsr", nails=False)
        plank(c, 0, 5, 80, 5, 5.0, "tsh")
        plank(c, 12, 22, 24, 9, 1.6, "tkb", nails=False)
        plank(c, 68, 22, 56, 9, 1.6, "tkb2", nails=False)
    else:
        plank(c, 8, 103, 8, 80, 4.0, "tsl", nails=False)
        plank(c, 72, 103, 72, 86, 4.0, "tsr", nails=False)
        plank(c, 6, 99, 46, 90, 4.0, "tsh1", nails=False)      # header pieces on the floor
        plank(c, 50, 101, 78, 96, 3.5, "tsh2", nails=False)
        for x, top in ((8, 80), (72, 86)):
            for k in range(7):
                y = top - 1 - int(rnd("tss", x, k) * 6)
                c.put(x - 3 + k, y, *SPLINTER[3 if k % 2 else 2])
                c.put(x - 3 + k, y + 1, *SPLINTER[1])
    return c


def barricade(state: str) -> "ld.Canvas":
    """Planks nailed across two posts, closing the end of the widened passage. Gameplay
    blocks the lane with an authored blocker rect until the pursuer charges through it."""
    c = canvas(96, 176)
    if state == "intact":
        plank(c, 12, 175, 12, 6, 6.0, "bpl", nails=False)
        plank(c, 84, 175, 84, 10, 6.0, "bpr", nails=False)
        ys = [22, 40, 55, 74, 92, 108, 127, 144, 160]
        for k, y in enumerate(ys):
            tilt = (rnd("btilt", k) - 0.5) * 6
            plank(c, 2, y - tilt, 94, y + tilt, 4.5 if k % 3 else 5.0, f"bp{k}")
        plank(c, 16, 168, 80, 30, 3.0, "bdiag", nails=False)
    else:
        plank(c, 12, 175, 12, 128, 6.0, "bpl", nails=False)
        plank(c, 84, 175, 74, 60, 6.0, "bpr", nails=False)    # leaning right post
        for k, (a, b) in enumerate([((4, 170), (52, 160)), ((44, 172), (92, 166)),
                                    ((20, 162), (60, 150)), ((60, 158), (90, 142)),
                                    ((78, 62), (94, 112))]):
            plank(c, a[0], a[1], b[0], b[1], 4.0, f"bs{k}", nails=k == 4)
        for k in range(9):
            x = 6 + k * 1.4
            y = 128 - 1 - int(rnd("bss", k) * 6)
            c.put(int(x), y, *SPLINTER[3 if k % 2 else 2])
    return c


BARRICADE_FULL_H = 280      # cornice underside (y 32) down to the deck's front lip (y 312)
BARRICADE_BACK = 256        # piece-local row of the floor back line (y 288)


BARRICADE_GAPS = [2, 3, 2, 7, 3, 2, 4, 2, 3, 7, 2, 3, 2, 4, 3, 2]   # px between boards, never equal twice running
BARRICADE_PATCHED = (3, 9)       # indices of the two wide (7 px) gaps that a short vertical slat patches
BARRICADE_OFF_ANGLE = [(62, 9.0, 3.4), (132, -7.0, 3.0), (196, 11.0, 3.6)]   # centre y, degrees, half width
BARRICADE_ZONE = (11, 253)       # piece-local rows between the header and the lane boards


def barricade_gap_fill(c, x0: int, x1: int, y0: int, y1: int) -> None:
    """Narrow shadow between the boards: mortar_00 (and a little mortar_01), so a gap reads as
    a seam and not as a black void."""
    for py in range(y0, y1):
        for px in range(x0, x1):
            c.put(px, py, "mortar", 0 if rnd("bgap", px, py) < 0.88 else 1)


def barricade_boards(c) -> None:
    """The boards between the posts: mixed thickness (8 to 14 px) with an uneven rhythm of
    2 to 7 px gaps, tone variety, overhang past the posts (2 to 6 px), two wide gaps patched by
    short vertical slats and three off-angle boards nailed over their neighbours. No board
    repeats its neighbour's thickness, and nothing runs post to post as a diagonal."""
    z0, z1 = BARRICADE_ZONE
    barricade_gap_fill(c, 6, 90, z0, z1)
    y, k = float(z0), 0
    gaps = []
    while True:
        gap = BARRICADE_GAPS[k % len(BARRICADE_GAPS)]
        thick = 8 + int(rnd("bth", k) * 7)                     # 8..14
        if k and thick == prev:
            thick = 8 + (thick - 8 + 3) % 7
        if y + gap + thick > z1 - 6:
            thick = z1 - (y + gap)                              # the last board closes the zone
            thick = max(thick, 8)
        top = y + gap
        mid = top + thick / 2
        tilt = (rnd("bfl", k) - 0.5) * 3.0
        o_l, o_r = 2 + int(rnd("bol", k) * 5), 2 + int(rnd("bor", k) * 5)
        bias = (rnd("bft", k) - 0.5) * 1.8 + (-1.0 if k % 5 == 2 else 0.7 if k % 5 == 4 else 0.0)
        plank(c, 6 - o_l, mid - tilt, 90 + o_r, mid + tilt, thick / 2, f"bf{k}", bias=bias)
        gaps.append((y, top))
        prev = thick
        y = top + thick
        k += 1
        if y >= z1 - 6:
            break
    for idx, x in zip(BARRICADE_PATCHED, (46, 30)):           # short vertical slats patching wide gaps
        g0, g1 = gaps[idx]
        plank(c, x, g0 - 4, x + 1, g1 + 4, 3.2, f"bslat{idx}", bias=0.6 - 0.9 * (idx % 2))
    for k, (cy, deg, half) in enumerate(BARRICADE_OFF_ANGLE):   # off-angle boards nailed over
        a = math.radians(deg)
        dy = math.tan(a) * 44
        plank(c, 5, cy - dy, 91, cy + dy, half, f"boff{k}", bias=(-0.6, 0.5, -0.2)[k])


def barricade_full(state: str) -> "ld.Canvas":
    """Full-height barricade built ACROSS the passage (96 x 280): it stands from the cornice
    underside down to the deck's front lip (y 312), so it covers the whole walking lane. Posts
    run down to the lip, a sill board and two kicker braces sit on the deck surface (y 288-312)
    and the lowest boards hide the deck between the posts: it reads as blocking the lane, not
    as a boarded doorway in the back wall. Drawn in the ``prop`` layer (behind actors): the
    blocker rect stops actors at its left face, so nothing ever overlaps it. The boards between
    the posts follow spec task 9 (uneven rhythm, patched gaps, off-angle boards)."""
    h, back = BARRICADE_FULL_H, BARRICADE_BACK
    c = canvas(96, h)
    if state == "intact":
        plank(c, 12, h - 1, 12, 0, 6.0, "bpl", nails=False)
        plank(c, 84, h - 1, 84, 0, 6.0, "bpr", nails=False)
        barricade_boards(c)
        # lane footing: two wide boards over the deck depth, a sill on the front lip
        plank(c, 4, back + 3, 92, back + 4, 5.0, "bflane1", bias=-0.2)
        plank(c, 3, back + 13, 93, back + 12, 5.0, "bflane2", bias=0.2)
        plank(c, 0, h - 4, 96, h - 4, 4.0, "bfsill", bias=0.5)      # sill on the deck front
        # kicker braces from the posts down to the deck front: propped in the lane
        plank(c, 12, back - 26, 3, h - 2, 2.5, "bfkick_l", nails=False, bias=0.4)
        plank(c, 84, back - 26, 93, h - 2, 2.5, "bfkick_r", nails=False, bias=-0.3)
        plank(c, 0, 5, 96, 5, 5.5, "bfh", bias=-0.4)                 # header under the cornice
    else:
        # left post snapped 30 px above the back line with a splintered top; the right post is
        # thrown over to lean into the next bay (top 18 px further right than its foot)
        plank(c, 12, h - 1, 12, back - 30, 6.0, "bpl", nails=False)
        for x in range(6, 18):                                  # jagged, pale splintered break
            top = back - 30 - int(rnd("bsn", x) * 6) - (3 if x % 4 == 1 else 0)
            for y in range(top, back - 29):
                if c.get(x, y) is None:
                    c.put(x, y, *SPLINTER[3 if (x + y) % 3 else 2])
        plank(c, 73, h - 1, 91, back - 100, 5.0, "bpr", nails=False)
        for y0, x0 in ((back - 96, 91), (back - 98, 93), (back - 100, 89)):   # splintered top
            c.put(x0, y0, *SPLINTER[3])
            c.put(x0 + 1, y0 - 1, *SPLINTER[2])
        # four to six boards thrown across the lane band, lying low and flat (actors walk over them)
        for k, (a0, b0, half) in enumerate([((2, h - 6), (50, h - 11), 3.6), ((40, h - 3), (94, h - 8), 3.8),
                                            ((6, back + 8), (56, back + 2), 3.5),
                                            ((52, back + 5), (92, back - 3), 3.6),
                                            ((20, back - 1), (48, back - 7), 3.2)]):
            plank(c, a0[0], a0[1], b0[0], b0[1], half, f"bfs{k}", bias=(rnd("bfsb", k) - 0.5) * 1.6)
        # one board still hangs by a nail from the leaning post
        plank(c, 80, 168, 94, 214, 3.4, "bfs_hang", nails=True, bias=-0.4)
        # the header is torn off but two stubs still hang from the cornice
        plank(c, 0, 5, 30, 6, 5.0, "bfh1", bias=-0.4)
        plank(c, 66, 5, 96, 4, 5.0, "bfh2", bias=-0.4)
        for x in (31, 32, 65, 64):
            for j in range(3):
                c.put(x, 1 + j * 3 + (x % 2), *SPLINTER[3 if j % 2 else 2])
    return c


def hatch() -> "ld.Canvas":
    """Iron maintenance hatch in a stone frame: rivets, strap hinges, a small brass wheel."""
    c = canvas(32, 32)
    for py in range(32):
        for px in range(32):
            frame = px < 3 or px > 28 or py < 3 or py > 28
            if frame:
                v = 6.2 if (px < 3 or py > 28) else 3.6
                c.put(px, py, "stone", v + (rnd("hf", px // 8, py // 8) - 0.5))
                continue
            edge = px == 3 or py == 3
            v = 3.4 + (1.2 if edge else 0) - (1.0 if px == 28 or py == 28 else 0) - 0.03 * (px + py)
            rivet = (px in (5, 26) or py in (5, 26)) and (px + py) % 5 == 0
            c.put(px, py, "iron", 5.6 if rivet else v)
    for py in (9, 10, 21, 22):       # strap hinges on the left
        for px in range(3, 14):
            c.put(px, py, "iron", 4.8 if py in (9, 21) else 2.2)
    for k in range(16):              # brass wheel (radius 4) right of centre
        a = k / 16 * math.tau
        c.put(round(21 + 4 * math.cos(a)), round(16 + 4 * math.sin(a)), "brass", 5 if a > math.pi else 3)
    for px in range(18, 25):
        c.put(px, 16, "brass", 2)
    for py in range(13, 20):
        c.put(21, py, "brass", 2)
    c.put(21, 16, "brass", 6)
    for x, y in ((0, 0), (31, 0), (0, 31), (31, 31)):   # chamfered frame corners
        c.clear(x, y)
    return c


# ---------------------------------------------------------------------------
# Dodge alcove (backwall decal)
# ---------------------------------------------------------------------------
ALCOVE_W, ALCOVE_H = 96, 96


def alcove() -> "ld.Canvas":
    """Recessed niche in the brick wall: segmental stone head, stone jambs, a cold deep
    interior (out of the torchlight) and a lit inner reveal. The ledge deck covers its foot."""
    c = canvas(ALCOVE_W, ALCOVE_H)
    cx, spring, r_in, r_out = 48.0, 44.0, 40.0, 50.0
    jamb = 8
    for py in range(ALCOVE_H):
        for px in range(ALCOVE_W):
            x, y = px + 0.5, py + 0.5
            d = math.hypot(x - cx, (y - spring) * 1.25)
            in_open = (y >= spring and jamb + 2 <= x < ALCOVE_W - jamb - 2) or (y < spring and d < r_in)
            in_frame = (y >= spring and 0 <= x < ALCOVE_W) or (y < spring and d < r_out)
            if not in_frame:
                continue
            if in_open:
                # interior: inner reveal (lit, right-facing, from the torch on the right) on the
                # left, cold back wall with faint cold bricks, darker toward the top
                reveal = x < jamb + 8 + (0 if y >= spring else 0)
                if reveal and y >= spring - 4:
                    v = 3.2 + 1.6 * (1 - (x - jamb - 2) / 8) - 0.012 * (ALCOVE_H - y)
                    c.put(px, py, "stone", v)
                    continue
                gx, gy = int(x), int(y)
                row = gy // 8
                joint = gy % 8 == 0 or (gx + 8 * (row % 2)) % 16 == 0
                deep = clamp((y - 10) / 100, 0, 1)
                if joint:
                    c.put(px, py, "brick_cold", 0)
                else:
                    c.put(px, py, "brick_cold", 0.6 + 1.5 * deep + (rnd("alc", row, (gx + 8 * (row % 2)) // 16) - 0.5) * 0.8)
                continue
            # stone frame: voussoirs above the spring line, jamb blocks below
            if y < spring:
                theta = math.atan2(spring - y, x - cx) / math.pi
                k = int(theta * 9)
                joint = abs(theta * 9 - round(theta * 9)) * d * 0.35 < 0.45
                seed = ("alcv", k)
                outer = d > r_out - 1.5
                inner = d < r_in + 1.5
            else:
                k = int((y - spring) // 12)
                joint = (y - spring) % 12 < 1 or (jamb - 1 <= x < jamb or ALCOVE_W - jamb <= x < ALCOVE_W - jamb + 1) and False
                seed = ("alcj", k, x < cx)
                outer = x < 1.5 or x > ALCOVE_W - 1.5
                inner = jamb <= x < jamb + 2 or ALCOVE_W - jamb - 2 <= x < ALCOVE_W - jamb
                if not (x < jamb + 2 or x >= ALCOVE_W - jamb - 2):
                    continue
            if joint:
                c.put(px, py, "mortar", 1.6)
                continue
            light = clamp((x - 20) / 76, 0, 1)    # torch to the right of the niche
            v = 4.0 + 2.0 * light + (rnd(*seed) - 0.5) * 0.9 - (1.2 if inner else 0) + (0.8 if outer and y < spring else 0)
            c.put(px, py, "stone", v)
    return c


# ---------------------------------------------------------------------------
# Guild exit
# ---------------------------------------------------------------------------
PORTAL_W, PORTAL_H = 112, 136
PORTAL_CX, PORTAL_SPRING = 56.0, 44.0
OPEN_HALF, OPEN_R = 34.0, 34.0
FRAME_HALF, FRAME_R = 52.0, 52.0
GRILLE = (66, 60, 12, 10)            # x, y, w, h on the right leaf
WARM = ["#ffc27a"]


def in_arch(x: float, y: float, half: float, r: float) -> bool:
    if y >= PORTAL_SPRING:
        return abs(x - PORTAL_CX) < half
    return math.hypot(x - PORTAL_CX, y - PORTAL_SPRING) < r


def portal_frame_pixel(c, px: int, py: int, x: float, y: float) -> None:
    """Pale dressed-stone portal (cared for: the guild keeps it clean), lit warm from inside
    on its inner edge and by the torch on its left; a carved lantern on the keystone."""
    d = math.hypot(x - PORTAL_CX, y - PORTAL_SPRING)
    if y < PORTAL_SPRING:
        theta = math.atan2(PORTAL_SPRING - y, x - PORTAL_CX) / math.pi
        k = int(theta * 11)
        joint = abs(theta * 11 - round(theta * 11)) * d * 0.3 < 0.45
        keystone = k == 5
        seed = ("pv", k)
        inner = d < OPEN_R + 3
        outer = d > FRAME_R - 2
    else:
        k = int((y - PORTAL_SPRING) // 13)
        joint = (y - PORTAL_SPRING) % 13 < 1
        keystone = False
        seed = ("pj", k, x < PORTAL_CX)
        inner = abs(x - PORTAL_CX) < OPEN_HALF + 3
        outer = abs(x - PORTAL_CX) > FRAME_HALF - 2
    if py >= PORTAL_H - 8:                 # plinth blocks
        v = 5.2 - (1.6 if py >= PORTAL_H - 2 else 0) + (rnd("pp", x < PORTAL_CX) - 0.5) * 0.6
        c.put(px, py, "stone", v)
        return
    if joint:
        c.put(px, py, "mortar", 2.0)
        return
    v = 6.4 + (rnd(*seed) - 0.5) * 0.8 - 0.012 * abs(x - PORTAL_CX) * 4 / 3
    if inner:
        v += 1.6                           # warm bounce from the interior / leak
    if outer:
        v -= 1.4
    if keystone and 8 < PORTAL_SPRING - y < 40:
        v += 0.6
        lx, ly = x - PORTAL_CX, PORTAL_SPRING - y
        # carved lantern glyph: a ring handle, a body and a base (no letters, no logo)
        body = abs(lx) < 4.5 and 18 < ly < 29
        ring = 1.2 < math.hypot(lx, ly - 31.5) < 2.6 and ly > 30
        base = abs(lx) < 6 and 16 < ly <= 18
        if body or ring or base:
            c.put(px, py, "stone", 3.2 if (body and abs(lx) < 2.5 and 20 < ly < 27) else 8.6)
            return
    c.put(px, py, "stone", v)


def portal(state: str) -> "ld.Canvas":
    c = canvas(PORTAL_W, PORTAL_H)
    for py in range(PORTAL_H):
        for px in range(PORTAL_W):
            x, y = px + 0.5, py + 0.5
            if not in_arch(x, y, FRAME_HALF, FRAME_R):
                continue
            if not in_arch(x, y, OPEN_HALF, OPEN_R) or py >= PORTAL_H - 8 + (8 if state == "open" else 0):
                portal_frame_pixel(c, px, py, x, y)
                continue
            lx = x - (PORTAL_CX - OPEN_HALF)       # 0..68 across the opening
            if state == "closed":
                # double leaf: vertical planks, iron straps and studs, bright meeting-stile leak
                if abs(x - PORTAL_CX) < 0.6:
                    c.put(px, py, "flame", 4 if y > 30 else 3)
                    continue
                if py >= PORTAL_H - 10:
                    c.put(px, py, "flame", 3 if py == PORTAL_H - 9 else 2)
                    continue
                gx, gy, gw, gh = GRILLE
                if gx <= px < gx + gw and gy <= py < gy + gh:
                    bar = (px - gx) % 4 == 0 or py in (gy, gy + gh - 1)
                    c.put(px, py, "iron", 2) if bar else c.put(px, py, "flame", 4 if (py - gy) < 5 else 3)
                    continue
                strap = int(y) in (54, 55, 56, 96, 97, 98)
                if strap:
                    stud = int(lx) % 8 == 3 and int(y) in (55, 97)
                    c.put(px, py, "iron", 6 if stud else (4.2 if int(y) in (54, 96) else 2.2))
                    continue
                ring_d = math.hypot(x - (PORTAL_CX - 8), y - 78)
                if 2.0 < ring_d < 3.6:
                    c.put(px, py, "brass", 5 if y < 78 else 3)
                    continue
                pu = lx % 8.5
                plank_i = int(lx // 8.5)
                if pu < 1.0:
                    c.put(px, py, "wood", 0.4)
                    continue
                edge_to_leak = abs(x - PORTAL_CX) < 2.5
                v = 3.3 + (rnd("gpl", plank_i) - 0.5) * 0.8 + (0.6 if pu < 2.2 else 0) \
                    - 0.008 * py + (1.4 if edge_to_leak else 0)
                if rnd("ggr", plank_i, int(py // 9)) < 0.15 and pu > 4:
                    v -= 0.8
                lut(c, px, py, TIMBER, v)
            else:
                # open: leaves swung inward show as narrow dark edges; warm interior with a
                # far doorway, a hanging lamp, a bench and a floor catching the light
                edge = abs(x - PORTAL_CX) > OPEN_HALF - 4
                if edge and y > 20:
                    lut(c, px, py, TIMBER, 1.6 if abs(x - PORTAL_CX) > OPEN_HALF - 1.5 else 3.0)
                    continue
                if y > PORTAL_H - 22:      # interior floor, brightest at the threshold
                    t = (y - (PORTAL_H - 22)) / 22
                    if y > PORTAL_H - 5:       # bright threshold strip: the only warm bloom besides the lamp
                        c.put(px, py, "gal_warm", 5 if y > PORTAL_H - 3 else 4)
                    else:
                        c.put(px, py, "floor", 7.0 + 2.0 * t)
                    continue
                lamp_d = math.hypot(x - PORTAL_CX, y - 50)
                if lamp_d < 3.5:
                    c.put(px, py, "flame", 5 if lamp_d < 1.8 else 4)
                    continue
                if lamp_d < 7.5:               # the lamp's own glow
                    c.put(px, py, "gal_warm", 6 if lamp_d < 5 else 5)
                    continue
                if abs(x - PORTAL_CX) < 0.6 and 22 < y < 46:
                    c.put(px, py, "iron", 2)
                    continue
                far = abs(x - PORTAL_CX - 10) < 10 and y > 66
                bench = 6 < lx < 26 and PORTAL_H - 32 < y < PORTAL_H - 26
                if bench:
                    c.put(px, py, "wood", 2 if y > PORTAL_H - 29 else 4)
                    continue
                if far:
                    c.put(px, py, "stone", 7 if abs(x - PORTAL_CX - 10) < 7 else 6)
                    continue
                glow = 1 - clamp(lamp_d / 70, 0, 1)
                c.put(px, py, "stone", 6.0 + 2.0 * glow - 0.35 * ((px + py) % 2))
    return c


def guild_sign() -> "ld.Canvas":
    """Iron wall bracket with a hanging painted board: a lantern pictogram, no text or logo."""
    c = canvas(40, 44)
    for px in range(0, 36):               # bracket arm
        c.put(px, 2, "iron", 4.6)
        c.put(px, 3, "iron", 2.2)
    for py in range(0, 10):               # wall plate
        c.put(0, py, "iron", 3)
        c.put(1, py, "iron", 4.2)
    for k in range(9):                    # diagonal stay
        c.put(2 + k * 2, 12 - k, "iron", 3.4)
        c.put(3 + k * 2, 12 - k, "iron", 3.4)
    for x in (10, 30):                    # chains
        for py in range(4, 13):
            c.put(x, py, "iron", 5 if py % 3 == 0 else 3)
    for py in range(13, 41):              # board
        for px in range(5, 36):
            edge = px in (5, 35) or py in (13, 40)
            v = 3.6 + (rnd("sgn", py // 5) - 0.5) * 0.6 - (1.4 if edge else 0) + (0.8 if py == 14 else 0)
            lut(c, px, py, TIMBER, v)
    # painted lantern (brass outline, warm glass), centred at (20, 27)
    for py in range(17, 38):
        for px in range(9, 32):
            x, y = px + 0.5 - 20.5, py + 0.5 - 27.0
            ring = 1.0 < math.hypot(x, y + 7.5) < 2.6 and y < -7
            frame = (abs(x) < 6 and abs(y) < 5.5) and not (abs(x) < 4.5 and abs(y) < 4.0)
            cap = abs(y + 6.0) < 1.0 and abs(x) < 5
            foot = abs(y - 6.5) < 1.0 and abs(x) < 7
            glass = abs(x) < 4.5 and abs(y) < 4.0
            if ring or frame or cap or foot:
                c.put(px, py, "brass", 5 if y < 0 else 4)
            elif glass:
                c.put(px, py, "flame", 4 if abs(x) < 1.5 and abs(y) < 2.5 else 3)
    return c


# ---------------------------------------------------------------------------
# Guild portal, ajar (the 1-frame in-between: leaves part, a warm slit shows)
# ---------------------------------------------------------------------------
AJAR_HALF = 7.0          # half width of the slit between the leaves (px)


def portal_ajar() -> "ld.Canvas":
    """Closed leaves with a 14 px warm slit down the meeting stile: the interior of the open
    portal shows through it, bordered by dark leaf edges."""
    closed, opened = portal("closed"), portal("open")
    c = canvas(PORTAL_W, PORTAL_H)
    for py in range(PORTAL_H):
        for px in range(PORTAL_W):
            x = px + 0.5
            dx = abs(x - PORTAL_CX)
            base = closed.px[py][px]
            inside = py < PORTAL_H - 8 and in_arch(x, py + 0.5, OPEN_HALF, OPEN_R)
            if inside and dx < AJAR_HALF and opened.px[py][px] is not None:
                c.px[py][px] = opened.px[py][px]
            elif inside and dx < AJAR_HALF + 1.6 and py > 20:
                lut(c, px, py, TIMBER, 5.0 if x < PORTAL_CX else 1.4)     # swung leaf edges
            else:
                c.px[py][px] = base
    return c


# ---------------------------------------------------------------------------
# Soffit tiles: the two steps of the lowered chase-passage ceiling (collision tileset)
# ---------------------------------------------------------------------------
# side "l" is the LEFT step (column 61: lit exposed edge at the tile's left), side "r" the
# RIGHT step (column 104: exposed edge at the right). The mass behind is the dark fill
# masonry of the lowered span (brick_00 faces, mortar joints), so the step reads as a heavy
# lintel face and not as a corridor end wall; the lit quoins are narrow (3 and 5 px).

def soffit_pixel(c, px: int, py: int, side: str, phase: int) -> None:
    lx = px if side == "l" else 15 - px                  # 0 = exposed edge
    course = (py // 8 + phase) % 2
    qw = 7 if course == 0 else 5                         # 6 px and 4 px lit quoins + a 1 px dark return
    dv = py % 8
    if dv == 0:
        c.put(px, py, "mortar", 3)                       # course joint: clearly lighter than the face
        return
    if lx < qw:
        if lx == qw - 1:
            c.put(px, py, "mortar", 0)                   # the 1 px dark return behind the quoin
            return
        edge = lx == 0
        v = 4.4 + (2.4 if edge else 0.0) + (0.7 if dv == 1 else 0.0) \
            + (rnd("sq", side, course, py // 8) - 0.5) * 0.6
        c.put(px, py, "stone", v)
        return
    if lx == 15 and course == 0:
        c.put(px, py, "mortar", 3)                       # block joint on the running bond
        return
    c.put(px, py, "brick", 0)
    if rnd("sbr", side, phase, px, py) < 0.08:
        c.put(px, py, "mortar", 1)


def soffit_face(side: str, phase: int, cut_end: bool = False) -> "ld.Canvas":
    """cut_end: the row just above the low cornice, where the blue ceiling course of the
    lowered span runs into the face: the course (ceiling_fill pixels) stops at the quoin's dark
    return, so it reads as a cut end."""
    c = canvas(TILE, TILE)
    cold = kit.ceiling_fill("s", 1)
    for py in range(TILE):
        for px in range(TILE):
            lx = px if side == "l" else 15 - px
            if cut_end and lx >= (7 if (py // 8 + phase) % 2 == 0 else 5):
                c.px[py][px] = cold.px[py][px]
            else:
                soffit_pixel(c, px, py, side, phase)
    return c


def soffit_corner(side: str) -> "ld.Canvas":
    """Low cornice wrapping round the outer corner of the soffit: the vault course keeps the
    quoin face of the tiles above; the cornice below turns the corner with a 4 px lit return."""
    c = kit.cornice("a", 1)
    for py in range(TILE):
        for px in range(TILE):
            lx = px if side == "l" else 15 - px
            if py < 4:
                soffit_pixel(c, px, py, side, 1)           # row low-1 follows an odd row: phase b
            elif lx < 4 and py > 4:
                c.put(px, py, "stone", (7.6, 6.2, 4.8, 3.6)[lx] - (0.5 if py > 12 else 0.0)
                      + (rnd("scr", side, py) - 0.5) * 0.4)
            elif lx < 4:
                c.put(px, py, "mortar", 0)
    return c


def soffit_joint(side: str) -> "ld.Canvas":
    """High cornice meeting the hanging face: its last 5 px (toward the step) are cut by the
    dark face with a lit quoin edge, so the face reads as standing proud of the cornice."""
    c = kit.cornice("a", 1)
    for py in range(TILE):
        for px in range(TILE):
            lx = (15 - px) if side == "l" else px          # 0 = the end that meets the face
            if lx > 4:
                continue
            if lx == 4:
                c.put(px, py, "mortar", 0)
            elif lx == 3 and py >= 4:
                c.put(px, py, "stone", 5.6 + (0.6 if py < 7 else 0.0))
            else:
                c.px[py][px] = ("brick", 0) if rnd("sj", side, px, py) > 0.1 else ("mortar", 1)
    return c


SOFFIT_TILES = {                                          # id -> (builder, args)
    "soffit_end_l": ("face", ("l", 0)), "soffit_end_l_b": ("face", ("l", 1)),
    "soffit_end_l_c": ("face", ("l", 1, True)),
    "soffit_end_r": ("face", ("r", 0)), "soffit_end_r_b": ("face", ("r", 1)),
    "soffit_end_r_c": ("face", ("r", 1, True)),
    "soffit_corner_l": ("corner", ("l",)), "soffit_corner_r": ("corner", ("r",)),
    "soffit_joint_l": ("joint", ("l",)), "soffit_joint_r": ("joint", ("r",)),
}


SOFFIT_FACING = ("drawn for the exposed edge on the %s (the %s step, column %s): lit quoins and the "
                 "return sit on that side; the tile is a pair with its mirror-side tile, not a mirror "
                 "of it: do not mirror it")


def soffit_facing(tid: str) -> str:
    left = "_l" in tid
    return SOFFIT_FACING % (("left", "left", "61") if left else ("right", "right", "104"))


def build_soffit() -> "kit.Atlas":
    a = kit.Atlas("floor01_soffit", "tileset")
    build = {"face": soffit_face, "corner": soffit_corner, "joint": soffit_joint}
    for tid, (kind, args) in SOFFIT_TILES.items():
        a.add_tile(tid, build[kind](*args))
    return a


# ---------------------------------------------------------------------------
# Missing frames: dust trickle, impact dust, timber splinter burst (animated pieces)
# ---------------------------------------------------------------------------
FX = {   # atlas -> (piece id, tag, frame w, h, frame count, duration ms, anchor, anchor note)
    "floor01_shore_dust": ("shore_dust_trickle", "trickle", 8, 24, 3, 117, [4, 0],
                           "top centre; place under the block's split (block + (41, 46)) from damaged_1 on"),
    "floor01_block_impact": ("block_impact_dust", "impact", 112, 40, 4, 83, [56, 36],
                             "bottom centre on the walking line at the centre of the fall region"),
    "floor01_splinter_burst": ("timber_splinter_burst", "burst", 48, 48, 4, 67, [24, 24],
                               "centre of the break (the hit point of the timber)"),
}


def shore_dust_frame(f: int) -> "ld.Canvas":
    """A thin stream of crumbs falling from the block: each crumb drops 8 px per frame and
    wraps after 24 px, so three frames loop seamlessly."""
    c = canvas(8, 24)
    for k in range(7):
        y = (int(rnd("sd", k) * 24) // 8 * 8 + (k * 3) % 8 + 8 * f) % 24
        x = 3 + int(rnd("sdx", k) * 3) - (1 if k % 3 == 0 else 0)
        c.put(x, y, "mortar", 6 if k % 3 else 5)
        if k % 2 == 0 and y > 0:
            c.put(x, y - 1, "stone", 7)
        if k % 3 == 1 and x + 1 < 8:
            c.put(x + 1, y, "stone", 8)
    return c


def block_impact_frame(f: int) -> "ld.Canvas":
    """Dust thrown out along the deck by the landing: a low cloud that widens, rises and thins.
    The late frames are built from 2x2 and 2x1 puff clusters that fade through darker tones, so
    they stay a cloud and do not break into single pixels."""
    c = canvas(112, 40)
    hw = (16, 30, 42, 50)[f]
    hh = (9, 16, 22, 26)[f]
    keep = (1.0, 0.85, 0.7, 0.55)[f]
    step = 1 if f < 2 else 2
    for cy in range(0, 40, step):
        for cx in range(0, 112, step):
            dx = (cx + step / 2 - 56) / hw
            dy = (37 - (cy + step / 2)) / hh
            if dy < -0.12:
                continue
            d = math.hypot(dx, max(dy, 0.0) * 1.1)
            if d >= 1.0:
                continue
            p = (1 - d) ** 0.7 * keep + 0.08 * keep
            if rnd("bi", f, cx, cy) >= p * 0.85:
                continue
            if f < 2:
                if d < 0.35:
                    c.put(cx, cy, "stone", 8)
                elif d < 0.7:
                    c.put(cx, cy, "stone", 6.4 if rnd("bic", f, cx, cy) > 0.3 else 5)
                else:
                    c.put(cx, cy, "mortar", 6 if rnd("bic", f, cx, cy) > 0.5 else 5)
                continue
            if f == 2:
                tone = ("stone", 7.0) if d < 0.4 else ("stone", 5.5) if d < 0.75 else ("mortar", 6)
            else:
                tone = ("stone", 5.5) if d < 0.4 else ("stone", 4.0) if d < 0.75 else ("mortar", 5)
            w2 = 2 if rnd("biw", f, cx, cy) > 0.35 else 1                   # 2x2 or 2x1 clusters
            for dx2 in range(2):
                for dy2 in range(w2):
                    c.put(cx + dx2, cy + dy2, *tone)
    for k in range(5 if f < 3 else 0):                   # chips thrown out and falling back
        ang = (k + 0.5) / 5 * math.pi
        r = 12 + f * 9 + rnd("bik", k) * 6
        x = 56 - math.cos(ang) * r * (1.4 if k % 2 else 1.0)
        y = 37 - math.sin(ang) * (14 + 4 * f) * (1.0 if f < 2 else 0.5) + (f * f) * 1.5
        for dx in range(2):
            for dy in range(2):
                c.put(int(x) + dx, int(y) + dy, "stone", 8.6 if dy == 0 else 5.5)
    if f == 3:                                           # rebalance the drift: mass left of the anchor
        pts = [(x, y) for y in range(40) for x in range(112) if c.px[y][x] is not None]
        if pts and sum(x for x, _y in pts) / len(pts) > 56:
            c = blit_mirror(c)
    return c


def blit_mirror(src: "ld.Canvas") -> "ld.Canvas":
    out = canvas(src.w, src.h)
    kit.blit(out, src, 0, 0, mirror_x=True)
    return out


def line_px(x0: float, y0: float, x1: float, y1: float):
    n = max(1, int(max(abs(x1 - x0), abs(y1 - y0))))
    for i in range(n + 1):
        yield round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)


def splinter_frame(f: int) -> "ld.Canvas":
    """Timber splinters flying out of a hit, falling under gravity, with a pale core flash."""
    c = canvas(48, 48)
    n = 11 if f < 3 else 8
    for k in range(n):
        ang = (k / n + (rnd("spa", k) - 0.5) * 0.12) * math.tau
        r0 = 3 + f * 5.2 + rnd("spr", k) * 3
        ln = 3 + rnd("spl", k) * 4 if f < 3 else 2.0
        sag = f * f * 0.9
        x0, y0 = 24 + math.cos(ang) * r0, 24 + math.sin(ang) * r0 * 0.9 + sag
        x1, y1 = 24 + math.cos(ang) * (r0 + ln), 24 + math.sin(ang) * (r0 + ln) * 0.9 + sag
        if f == 3:                       # slivers: exactly 2 touching pixels along the flight direction
            sx = int(round(math.cos(ang)))
            sy = int(round(math.sin(ang)))
            sx, sy = (sx, sy) if (sx or sy) else (1, 0)
            pts = [(round(x0), round(y0)), (round(x0) + sx, round(y0) + sy)]
        else:
            pts = list(line_px(x0, y0, x1, y1))
        for i, (x, y) in enumerate(pts):
            if k % 3 == 0:
                c.put(x, y, "stone", 9 if i == 0 else 7)       # pale fresh fibres
            else:
                lut(c, x, y, TIMBER, 4.6 if i % 2 == 0 else 2.6)
        if k % 2 == 0 and f < 3:
            c.put(round(x1), round(y1), "stone", 8.6)
    if f == 0:                                           # impact flash at the hit
        for dx, dy in ((0, 0), (-1, 0), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)):
            c.put(24 + dx, 24 + dy, "stone", 10 if dx == 0 and dy == 0 else 8.6)
    for k in range(4 + f if f < 3 else 0):                # grit (none in the last frame: slivers only)
        x = 24 + int((rnd("spg", f, k) - 0.5) * (14 + f * 10))
        y = 24 + int((rnd("spgy", f, k) - 0.5) * (12 + f * 8)) + f
        c.put(x, y, "mortar", 6)
    return c


def fx_frames(name: str) -> list:
    fn = {"floor01_shore_dust": shore_dust_frame, "floor01_block_impact": block_impact_frame,
          "floor01_splinter_burst": splinter_frame}[name]
    return [fn(f) for f in range(FX[name][4])]


# ---------------------------------------------------------------------------
# Atlases, contract
# ---------------------------------------------------------------------------

def warm_light(lid: str, offset, radius: int, intensity: float, **extra) -> dict:
    light = {"id": lid, "offset": list(offset), "colour": WARM[0], "radius": radius,
             "intensity": intensity, "group": "scene"}
    light.update(extra)
    return light


def build_atlases() -> dict:
    props = kit.Atlas("floor01_opening_props", "decor_atlas")
    shore_anchor = [int(FOOT_L[0]), int(FOOT_L[1])]
    for state in ("intact", "damaged_1", "damaged_2", "broken"):
        props.add_piece(f"shore_post_{state}", shore(state), anchor=shore_anchor,
                        anchor_name="shore foot (bottom of the timber on the deck); all four states "
                                    "share this rect and placement", grid_snap=8, grammar=["A", "B"],
                        breakable={"group": "raking_shore", "state": state})
    for state in ("seated", "settling", "slipped"):
        props.add_piece(f"loose_block_{state}", loose_block(state), anchor=[32, 48],
                        anchor_name="bottom centre of the block (rests on the shore head cap)",
                        grid_snap=8, grammar=["A", "B"],
                        breakable={"group": "raking_shore_load", "state": state})
    props.add_piece("loose_block_falling", falling_block(), anchor=[36, 52],
                    anchor_name="bottom centre of the tipping block (debris sprite, moved by gameplay)",
                    grid_snap=1, grammar=["A", "B"],
                    breakable={"group": "raking_shore_load", "state": "falling"})
    props.add_piece("loose_block_landed", landed_rubble(), anchor=[48, 30],
                    anchor_name="bottom centre on the walking line (non-collidable rubble)",
                    grid_snap=8, grammar=["A", "B"],
                    breakable={"group": "raking_shore_load", "state": "landed"})
    props.add_piece("hazard_grit", hazard_grit(), anchor=[48, 8],
                    anchor_name="centre of the fall region on the deck (y 288..304)", grid_snap=8,
                    grammar=["A", "B"])
    for state in ("intact", "smashed"):
        props.add_piece(f"timber_set_{state}", timber_set(state), anchor=[40, 104],
                        anchor_name="bottom centre on the floor back line (y = 288)", grid_snap=8,
                        grammar=["C"], breakable={"group": "timber_set", "state": state})
    for state in ("intact", "smashed"):
        props.add_piece(f"timber_barricade_{state}", barricade(state), anchor=[48, 176],
                        anchor_name="bottom centre on the floor back line (y = 288)", grid_snap=8,
                        grammar=["A"], breakable={"group": "timber_barricade", "state": state})
    props.add_piece("maint_hatch", hatch(), anchor=[0, 0], anchor_name="top_left (brick-aligned)",
                    grid_snap=8, grammar=["C"])
    for state in ("intact", "smashed"):
        props.add_piece(f"timber_barricade_full_{state}", barricade_full(state), anchor=[48, 280],
                        anchor_name="bottom centre on the deck's front lip (y = 312): the piece "
                                    "crosses the walking lane; the top meets the cornice underside "
                                    "(y = 32)", grid_snap=8, grammar=["A"],
                        breakable={"group": "timber_barricade_full", "state": state})
    for p in props.pieces:
        p.setdefault("role", "prop")
        p.setdefault("layer", "prop")

    decal = kit.Atlas("floor01_opening_decal", "decor_atlas")
    decal.add_piece("alcove_niche", alcove(), anchor=[48, ALCOVE_H],
                    anchor_name="bottom centre on the ledge's deck back line (ledge row top)",
                    grid_snap=16, grammar=["A"], decal=True, role="backwall", layer="backwall_decal")
    decal.add_piece("wall_cavity", wall_cavity(), anchor=[8, 8],
                    anchor_name="top_left of the loose block minus (8, 8): drawn behind the block",
                    grid_snap=8, grammar=["A", "B"], decal=True, role="backwall", layer="backwall_decal")

    guild = kit.Atlas("floor01_guild", "decor_atlas")
    cx = int(PORTAL_CX)
    guild.add_piece("guild_portal_closed", portal("closed"), anchor=[cx, PORTAL_H],
                    anchor_name="bottom centre on the floor back line (y = 288)", grid_snap=8,
                    grammar=["A"], breakable={"group": "guild_portal", "state": "closed"},
                    lights=[warm_light("grille", [GRILLE[0] + 6, GRILLE[1] + 5], 70, 0.22,
                                       glow={"radius": 14, "additive": 0.06}),
                            warm_light("door_leak", [cx, PORTAL_H - 4], 110, 0.2,
                                       project="walk_line", project_dy=-4, squash_y=0.3)])
    guild.add_piece("guild_portal_open", portal("open"), anchor=[cx, PORTAL_H],
                    anchor_name="bottom centre on the floor back line (y = 288)", grid_snap=8,
                    grammar=["A"], breakable={"group": "guild_portal", "state": "open"},
                    lights=[warm_light("interior", [cx, 60], 240, 0.5,
                                       glow={"radius": 28, "additive": 0.1}),
                            warm_light("spill", [cx, PORTAL_H - 4], 170, 0.5,
                                       project="walk_line", project_dy=-4, squash_y=0.34)])
    guild.add_piece("guild_sign", guild_sign(), anchor=[0, 2],
                    anchor_name="bracket wall plate (top_left + (0, 2))", grid_snap=8, grammar=["A"])
    guild.add_piece("guild_portal_ajar", portal_ajar(), anchor=[cx, PORTAL_H],
                    anchor_name="bottom centre on the floor back line (y = 288)", grid_snap=8,
                    grammar=["A"], breakable={"group": "guild_portal", "state": "ajar"},
                    lights=[warm_light("interior", [cx, 60], 170, 0.32,
                                       glow={"radius": 28, "additive": 0.07}),
                            warm_light("spill", [cx, PORTAL_H - 4], 130, 0.3,
                                       project="walk_line", project_dy=-4, squash_y=0.32)])
    for p in guild.pieces:
        p.setdefault("role", "prop")
        p.setdefault("layer", "prop")
    return {"floor01_opening_props": props, "floor01_opening_decal": decal, "floor01_guild": guild}


def atlas_ref(name: str) -> dict:
    stem, _kind = ATLAS_FILES[name]
    return {"grid": f"{stem}.grid.json", "manifest": f"assets/manifests/{name}.json",
            "image": f"generated/art/{name}.png", "metadata": f"generated/art/{name}.json"}


def manifest(name: str, grid: dict, tiles: list[str] | None = None, frame_size=None,
             pivot=(0, 0), tags=()) -> dict:
    stem, kind = ATLAS_FILES[name]
    m = {
        "schema_version": 1,
        "id": f"{name}_v1",
        "type": kind,
        "source_path": f"{stem}.ase",
        "grid_source": f"{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png",
        "output_metadata": f"generated/art/{name}.json",
        "palette_id": kit.PALETTE_ID,
        "frame_size": list(frame_size or grid["canvas"]),
        "pivot": list(pivot),
        "animation_tags": list(tags),
        "required_alpha": kind != "tileset",
        "kit_contract": CONTRACT_REL,
        "owner": "AI-authored environment pixel art (ENV-02: design and keys Claude Opus, "
                 "full-set execution Claude Sonnet; independent review pending)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": f"Original pixel-grid-v1 {ATLAS_ORIGIN[name]} for the Floor 1 opening route "
                      "(ENV-02 on the ENV-01 kit); not cropped, traced or resampled from concept art",
            "generator": "Deterministic authoring script scripts/author_floor01_opening.py written by "
                         "AI agents (design and keys Claude Opus, full-set execution Claude Sonnet), "
                         "reusing the ENV-01 shading helpers (scripts/author_floor01_kit.py, "
                         "scripts/lookdev_floor01_author.py); no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "lookdev-floor01-resolution-spike/A1 (owner-approved target frame, 2026-11-16)",
            ],
        },
    }
    if kind == "tileset":
        m["tile_size"] = [TILE, TILE]
        m["tiles"] = tiles
    return m


STATUS = ("proposed - ENV-02 full set drawn (Claude Sonnet) on the Opus keys; awaiting independent "
          "Opus review; not owner-approved")


def build_contract(tiles: list[dict], pieces: list[dict], grids: dict) -> dict:
    return {
        "format": "env-kit-v2",
        "kit_id": KIT_ID,
        "extends": kit.CONTRACT_REL,
        "status": STATUS,
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and "
                          "lighting schema from the ENV-01 contract (env_kit.py resolve_extends). "
                          "Adds atlases, pieces and the 10 soffit tiles of the chase-passage ceiling "
                          "steps; no new palette colours. Soffit tile sides: l = the left step "
                          "(column 61), r = the right step (column 104); soffit_joint_* sit one "
                          "column outside the step (60 / 105) at the high cornice.",
        "palette_use": {name: len(g["palette"]) for name, g in grids.items()},
        "atlases": {name: atlas_ref(name) for name in ATLAS_FILES},
        "tiles": tiles,
        "pieces": pieces,
        "state_groups": {
            "raking_shore": {"states": ["intact", "damaged_1", "damaged_2", "broken"],
                             "note": "same rect and placement; gameplay swaps the piece per hit"},
            "raking_shore_load": {"states": ["seated", "settling", "slipped", "falling", "landed"],
                                  "note": "seated/slipped share a rect; falling is the debris sprite "
                                          "moved by gameplay; landed is placed on the walking line"},
            "timber_set": {"states": ["intact", "smashed"]},
            "timber_barricade": {"states": ["intact", "smashed"],
                                 "note": "96x176 lower option; the layouts use the full-height pair"},
            "timber_barricade_full": {"states": ["intact", "smashed"],
                                      "note": "96x280 from the cornice (y 32) to the deck's front lip "
                                              "(y 312): crosses the walking lane and covers the "
                                              "full-height barricade blocker"},
            "guild_portal": {"states": ["closed", "ajar", "open"],
                             "note": "ajar is the 1-frame in-between played when a player enters "
                                     "exit_guild; same rect as closed and open"},
        },
        "usage_rules": {
            "S3_chase": "C grammar under a lowered ceiling: copper run along the cornice, valve on a "
                        "drop, maint_hatch at chest height, timber_set_* every 320 px or so (bottom "
                        "on the floor back line), jump humps as 1-tile platforms. The two ceiling "
                        "steps use the soffit tiles: soffit_end_l/_b/_c (left step), "
                        "soffit_end_r/_b/_c (right step; _c is the cut end of the blue ceiling "
                        "course just above the low cornice), soffit_corner_* at the low cornice, "
                        "soffit_joint_* at the high cornice next to the face. Tiles are drawn per "
                        "side: do not mirror them.",
            "S4_widened": "A grammar at full height: alcove_niche on a 1-tile ledge (6 columns), a "
                          "pooled torch right of the niche, timber_barricade_full_* (96x280, y 32 to the lip) "
                          "closing the end.",
            "S5_skirmish": "raking shore set: wall_cavity at block - (8, 8), loose_block_* at the "
                           "shore head, shore_post_* foot on the deck, hazard_grit centred on the fall "
                           "region; keep a pier or wall torch within 120 px so the set reads lit. "
                           "Presentation frames: shore_dust_trickle under the block's split from "
                           "damaged_1, timber_splinter_burst at each hit, block_impact_dust on the "
                           "landing.",
            "S6_guild": "guild_portal_* bottom on the floor back line, guild_sign right of the "
                        "portal at y 168, nothing breakable within 160 px of the portal.",
        },
    }


# ---------------------------------------------------------------------------
# Layout
# ---------------------------------------------------------------------------

def stable(*key) -> int:
    return int(rnd("opening", *key) * 1_000_003)


def build_layout(contract: dict, option: str) -> dict:
    cols, rows = COLS, ROWS
    collision: list[list[str | None]] = [[None] * cols for _ in range(rows)]
    backwall: list[list[str | None]] = [[None] * cols for _ in range(rows)]
    tiles = {t["id"]: t for t in contract["tiles"]}
    panels: dict[str, dict] = {}
    for t_id, t in tiles.items():
        p = t.get("panel")
        if p:
            panels.setdefault(p["id"], {"size": p["size"], "cells": {}})["cells"][p["cell"][0]] = t_id
    pier_flames = [b * TILE + 32 for b in BAYS] + [b * TILE + 272 for b in BAYS]
    lights_x = WALL_TORCHES + pier_flames + [GUILD_X + 56]

    def tone_at(x0: int, x1: int) -> str:
        cx = (x0 + x1) / 2 * TILE
        d = min(abs(cx - f) for f in lights_x)
        return "lit" if d < 60 else "mid" if d < 124 else "dim"

    def fill_run(row, c0, c1, prefix, variants, grid=collision, tone=False, salt=""):
        x, last = c0, None
        while x < c1:
            left = c1 - x
            ids = [(f"{prefix}_{tone_at(x, x + 4)}_{v}" if tone else f"{prefix}_{v}") for v in variants]
            fits = [i for i in ids if i in panels and panels[i]["size"][0] <= left]
            if left >= 4:
                wide = [i for i in fits if panels[i]["size"][0] == 4]
            elif left in (2, 3):
                wide = [i for i in fits if panels[i]["size"][0] == 2] or \
                    [i for i in fits if panels[i]["size"][0] == max(panels[j]["size"][0] for j in fits)]
            else:
                wide = [i for i in fits if panels[i]["size"][0] == 1]
            if not wide:
                raise SystemExit(f"row {row}: cannot fill {left} columns from {x} with {prefix}")
            choice = [i for i in wide if i != last] or wide
            pid = choice[stable(row, x, salt, prefix) % len(choice)]
            last = pid
            for i in range(panels[pid]["size"][0]):
                grid[row][x + i] = panels[pid]["cells"][i]
            x += panels[pid]["size"][0]

    # --- ceiling ---------------------------------------------------------------------
    low = CEILING_OPTIONS[option]["rows"]
    ceil = [2] * cols
    for c in range(LOW_CEIL[0], LOW_CEIL[1] + 1):
        ceil[c] = low
    for r in range(rows):
        collision[r][0] = "wall_end_l" if r % 2 == 0 else "wall_end_l_b"
    # row 0 everywhere; deeper ceiling-fill rows inside the lowered span
    collision[0][1] = "ceiling_fill_s"
    fill_run(0, 2, cols, "ceiling_fill", ["a", "s"], salt="c0")
    cl, cr = LOW_CEIL
    for r in range(1, low - 1):
        # the row above the low cornice is the cut end of the blue ceiling course (_c)
        suffix = "_c" if r == low - 2 else "" if r % 2 else "_b"
        collision[r][cl] = f"soffit_end_l{suffix}"
        collision[r][cr] = f"soffit_end_r{suffix}"
        if r == low - 2:       # the course right above the low cornice keeps the ceiling look
            fill_run(r, cl + 1, cr, "ceiling_fill", ["a", "s"], salt=f"c{r}")
        else:                  # deeper soffit mass: the dark fill blocks of the floor stack
            joint = (16 * (r % 2) - (cl + 1) * TILE) % kit.SLAB_W
            level = 1 if r == low - 3 else 2
            fill_run(r, cl + 1, cr, f"fill_{level}_j{joint}", ["a", "c"], salt=f"c{r}")
    # cornice rows: high cornice (row 1) on both sides, low cornice (row low-1) inside
    collision[1][1] = "ceiling_corner_in_l"
    fill_run(1, 2, cl - 1, "cornice", ["a", "b", "s"], salt="k1a")
    collision[1][cl - 1] = "soffit_joint_l"
    collision[1][cr + 1] = "soffit_joint_r"
    fill_run(1, cr + 2, cols, "cornice", ["a", "b", "s"], salt="k1b")
    collision[low - 1][cl] = "soffit_corner_l"
    collision[low - 1][cr] = "soffit_corner_r"
    fill_run(low - 1, cl + 1, cr, "cornice", ["a", "b", "s"], salt="klow")

    # --- floor stack -----------------------------------------------------------------
    top = [FLOOR_ROW] * cols
    for (c0, c1, h) in PLATFORMS:
        for c in range(c0, c1 + 1):
            top[c] = FLOOR_ROW - h

    def single_for(c):
        if c == 1:
            return ("wall", "l")
        for (c0, c1, _h) in PLATFORMS:
            if c == c0 - 1:
                return ("inner", "r")
            if c == c0:
                return ("end", "l")
            if c == c1:
                return ("end", "r")
            if c == c1 + 1:
                return ("inner", "l")
        return None

    def row_type(c, r):
        rel = r - top[c]
        return None if rel < 0 else kit.ROW_TYPES[min(rel, 4)]

    deck_variants = {"deck_back": ["a", "b", "c"], "deck_front": ["a", "b", "c"],
                     "face_a": ["a", "c"], "face_b": ["a", "b", "c"]}
    for r in range(min(top[1:]), rows):
        c = 1
        while c < cols:
            rt = row_type(c, r)
            if rt is None:
                c += 1
                continue
            single = single_for(c)
            if rt == "fill" and c == 1:
                collision[r][c] = f"fill_wall_l_{kit.fill_level(r)}"
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
                joint = (16 * (r % 2) - c * TILE) % kit.SLAB_W
                fill_run(r, c, end, f"fill_{kit.fill_level(r)}_j{joint}", ["a", "c"], salt=f"f{r}")
            else:
                fill_run(r, c, end, rt, deck_variants[rt],
                         tone=rt in ("deck_back", "deck_front", "face_a"), salt=f"r{r}c{c}")
            c = end

    # --- backwall --------------------------------------------------------------------
    windows = []
    spec = kit.window_spec()
    for b in BAYS:
        ox, oy = b * TILE + 16, kit.WALL_TOP
        windows.append(((ox + spec["arc_centre"][0], oy + spec["arc_centre"][1], spec["arc_radius"]),
                        (ox + spec["rect"][0], oy + spec["rect"][1], spec["rect"][2], spec["rect"][3])))
    import env_kit
    mask = env_kit.window_mask((cols * TILE, rows * TILE), windows)

    def band_for(c: int, r: int) -> str:
        """ENV-01 bands by absolute row; under a lowered ceiling the same sequence (cold top,
        upper, mid, lower, lit low band behind actor heads) is compressed below the cornice."""
        if ceil[c] == 2:
            return kit.band_of_row(r)
        k, n = r - ceil[c], kit.BASE_COURSE_ROW - ceil[c]
        if k < 2:
            return "top"
        if k >= n - 2:
            return "low"
        if k >= n - 4:
            return "lower"
        return "upper" if k < 2 + (n - 6) // 2 else "mid"

    for r in range(2, kit.BASE_COURSE_ROW + 1):
        x = 1
        while x < cols:
            if collision[r][x] is not None or env_kit._cell_hits_mask(mask, x, r):
                x += 1
                continue
            band = "base" if r == kit.BASE_COURSE_ROW else band_for(x, r)
            end = x
            while end < cols and collision[r][end] is None and not env_kit._cell_hits_mask(mask, end, r) \
                    and ("base" if r == kit.BASE_COURSE_ROW else band_for(end, r)) == band \
                    and (end == x or end not in (LOW_CEIL[0], LOW_CEIL[1] + 1)):
                end += 1
            if band == "base":
                fill_run(r, x, end, "base_course", ["a", "b", "s", "t", "u", "v"], grid=backwall, salt="base")
            else:
                fill_run(r, x, end, f"brick_{band}", ["a", "b", "c", "d", "s", "t"], grid=backwall,
                         salt=f"b{r}")
            x = end

    # --- pieces ----------------------------------------------------------------------
    decals, structure, props, foreground = [], [], [], []
    tf = kit.TORCH_FLAME_LOCAL
    for i, fx in enumerate(WALL_TORCHES):
        decals.append({"piece": POOL_PICK[i], "x": fx - kit.POOL_CENTRE_LOCAL[0], "y": kit.POOL_TOP_Y})
        props.append({"piece": "torch_wall", "x": fx - tf[0], "y": kit.WALL_TORCH_FLAME_Y - tf[1],
                      "frame_offset": i % 4})
    for fx in UNLIT_SCONCES:
        props.append({"piece": "torch_wall_unlit", "x": fx - tf[0], "y": kit.WALL_TORCH_FLAME_Y - tf[1]})
    for j, b in enumerate(BAYS):
        ox = b * TILE
        structure += [
            {"piece": "pier_l", "x": ox, "y": kit.WALL_TOP},
            {"piece": "arch_ring_a" if j == 0 else "arch_ring_b", "x": ox + 16, "y": kit.WALL_TOP},
            {"piece": "pier_r", "x": ox + 224, "y": kit.WALL_TOP},
            {"piece": "arch_threshold", "x": ox + 64, "y": kit.WALL_TOP + 240},
        ]
        if j == 0:
            structure.append({"piece": "arch_chain", "x": ox + 80, "y": kit.WALL_TOP + 80})
        for k, dx in enumerate((24, 264)):
            props.append({"piece": "torch_wall", "x": ox + dx, "y": kit.PIER_TORCH_FLAME_Y - tf[1],
                          "frame_offset": (j * 2 + k + 1) % 4})
    # pier spills where a pier has no adjacent wall pool
    decals.append({"piece": "pier_spill_r", "x": BAYS[0] * TILE + 288, "y": kit.SPILL_TOP_Y})
    decals.append({"piece": "pier_spill_l", "x": BAYS[1] * TILE - 48, "y": kit.SPILL_TOP_Y})
    for (gx, gy, v) in ((22, 10, "a"), (150, 9, "b")):
        decals.append({"piece": f"grime_{v}", "x": gx * TILE, "y": gy * TILE})
    # S1: door and pilaster
    props.append({"piece": "door_timber", "x": 424, "y": BACK_Y - kit.DOOR_H})
    structure.append({"piece": "pilaster_plain", "x": 512, "y": kit.WALL_TOP})
    # S3: copper run along the low cornice, a valve on a drop, hatch, timber sets
    under = CEILING_OPTIONS[option]["underside_y"]
    run_row = under // TILE + 1                   # pipe row (elbow_lu meets the cornice underside)
    pc = 64                                       # vertical drop column
    props.append({"piece": "pipe_elbow_rd", "x": pc * TILE, "y": run_row * TILE})
    for c in range(pc + 2, 100):
        props.append({"piece": "pipe_h_seam" if c in (78, 90) else "pipe_h", "x": c * TILE,
                      "y": run_row * TILE})
    props.append({"piece": "pipe_elbow_lu", "x": 100 * TILE, "y": (run_row - 1) * TILE})
    props.append({"piece": "pipe_collar_ceiling", "x": 101 * TILE - 8, "y": under - 0})
    for r in range(run_row + 2, kit.BASE_COURSE_ROW + 1):
        props.append({"piece": "pipe_v_seam" if r == 14 else "pipe_v", "x": pc * TILE, "y": r * TILE})
    props.append({"piece": "pipe_collar_floor", "x": pc * TILE - 8, "y": BACK_Y - 8})
    props.append({"piece": "valve_wheel", "x": pc * TILE - 8, "y": 232})
    props.append({"piece": "pipe_flange_v", "x": pc * TILE - 8, "y": 216})
    props.append({"piece": "pipe_bracket_v", "x": pc * TILE - 8, "y": 264})
    props.append({"piece": "pipe_flange_h", "x": 84 * TILE - 8, "y": run_row * TILE - 8})
    props.append({"piece": "pipe_bracket_h", "x": 72 * TILE + 8, "y": run_row * TILE - 8})
    props.append({"piece": "pipe_bracket_h", "x": 95 * TILE + 8, "y": run_row * TILE - 8})
    props.append({"piece": "maint_hatch", "x": HATCH_XY[0], "y": HATCH_XY[1]})
    for x in TIMBER_SETS:
        props.append({"piece": "timber_set_intact", "x": x, "y": BACK_Y - 104})
    # S4: alcove niche on the ledge, barricade
    decals.append({"piece": "alcove_niche", "x": ALCOVE_XY[0], "y": ALCOVE_XY[1]})
    props.append({"piece": "timber_barricade_full_intact", "x": BARRICADE_X, "y": BARRICADE_FULL_Y})
    # S5: the CP-03 raking shore set
    decals.append({"piece": "wall_cavity", "x": BLOCK_XY[0] - 8, "y": BLOCK_XY[1] - 8})
    props.append({"piece": "hazard_grit", "x": HAZARD[0] - 8, "y": BACK_Y})
    props.append({"piece": "loose_block_seated", "x": BLOCK_XY[0], "y": BLOCK_XY[1]})
    props.append({"piece": "shore_post_intact", "x": SHORE_ORIGIN[0], "y": SHORE_ORIGIN[1]})
    # S6: guild portal and sign
    props.append({"piece": "guild_portal_closed", "x": GUILD_X, "y": BACK_Y - PORTAL_H})
    props.append({"piece": "guild_sign", "x": GUILD_X + PORTAL_W, "y": 168})
    foreground.append({"piece": "rubble_b", "x": 1336, "y": 400 - 24})
    foreground.append({"piece": "rubble_a", "x": 2296, "y": 400 - 32})

    walk = WALK_Y
    ledge_walk = walk - TILE
    guild_cx = GUILD_X + int(PORTAL_CX)
    markers = [
        {"id": "player_spawn", "kind": "spawn", "x": 72, "y": walk - 1,
         "note": "actor pivot pixel (foot contact row = walking line - 1)"},
        {"id": "checkpoint_chase", "kind": "checkpoint", "x": 1008, "y": walk - 1,
         "note": "respawn for a failed chase: inside S3, before the trigger"},
        {"id": "trigger_chase_start", "kind": "trigger", "x": 1072, "y": walk - 1,
         "rect": [1072, under, 16, walk - under],
         "note": "leading player crosses -> pursuer spawns behind (authored event)"},
        {"id": "pursuer_spawn", "kind": "pursuer_spawn", "x": BAYS[0] * TILE + 152, "y": walk - 1,
         "rect": [BAYS[0] * TILE + 56, kit.WALL_TOP + 38, 192, 218],
         "note": "pursuer emerges from the gallery through the S2 arch window (turn-in is "
                 "presentation); body enters the lane at the window centre"},
        {"id": "pursuer_path", "kind": "pursuer_path", "x": BAYS[0] * TILE + 152, "y": walk - 1,
         "points": [[BAYS[0] * TILE + 152, walk], [1712, walk], [CHARGE_X, walk], [BARRICADE_X + 48, walk],
                    [BAYS[1] * TILE + 152, walk]],
         "note": "lane path at the walking line; ceiling clearance in S3 is design_parameters.pursuer_clearance"},
        {"id": "clearance_gauge_s3", "kind": "clearance_gauge", "x": LOW_CEIL[0] * TILE, "y": under,
         "rect": [LOW_CEIL[0] * TILE, under, (LOW_CEIL[1] + 1 - LOW_CEIL[0]) * TILE, walk - under],
         "note": f"ceiling option {option}: walking line to cornice underside"},
        {"id": "timber_set_a", "kind": "pursuer_breakable", "x": TIMBER_SETS[0] + 40, "y": BACK_Y - 1,
         "rect": [TIMBER_SETS[0], BACK_Y - 104, 80, 104], "art": "timber_set",
         "note": "presentation + authored event: swap to timber_set_smashed when the pursuer front "
                 "crosses rect x; no collision, no damage"},
        {"id": "timber_set_b", "kind": "pursuer_breakable", "x": TIMBER_SETS[1] + 40, "y": BACK_Y - 1,
         "rect": [TIMBER_SETS[1], BACK_Y - 104, 80, 104], "art": "timber_set"},
        {"id": "jump_hump_a", "kind": "jump_lane", "x": HUMPS[0][0] * TILE, "y": walk - TILE - 1,
         "rect": [HUMPS[0][0] * TILE, walk - TILE, (HUMPS[0][1] - HUMPS[0][0] + 1) * TILE, TILE],
         "note": "1-tile raised paving: hop or step while fleeing"},
        {"id": "jump_hump_b", "kind": "jump_lane", "x": HUMPS[1][0] * TILE, "y": walk - TILE - 1,
         "rect": [HUMPS[1][0] * TILE, walk - TILE, (HUMPS[1][1] - HUMPS[1][0] + 1) * TILE, TILE]},
        {"id": "pursuer_charge_zone", "kind": "pursuer_charge", "x": CHARGE_X, "y": walk - 1,
         "rect": [CHARGE_X, 32, BARRICADE_X + 96 - CHARGE_X, walk - 32],
         "refuge_entry_x": LEDGE[0] * TILE, "refuge_distance_px": LEDGE[0] * TILE - CHARGE_X,
         "note": "leading player crosses x -> the pursuer charges if the gap is >= 140 px (DOZ-01); "
                 "the refuge entrance is 176 px ahead: 57 ticks at 185 px/s, 73 at 145 px/s, inside "
                 "the 90-tick charge; momentum carries it past the alcove into the barricade"},
        {"id": "safe_pocket_alcove", "kind": "safe_pocket", "x": LEDGE[0] * TILE + 48, "y": ledge_walk - 1,
         "rect": [LEDGE[0] * TILE + 8, ALCOVE_XY[1] + 12, 80, ledge_walk - ALCOVE_XY[1] - 12],
         "clear_height_px": ledge_walk - ALCOVE_XY[1] - 12,
         "note": "refuge the pursuer cannot follow (clear height 104 px < 155 px pursuer; DOZ-01 "
                 "range 64..116). 80 px wide: Carl and Donut fit together. Actors whose feet are on "
                 "the ledge inside it are skipped by pursuer contact and drawn behind it"},
        {"id": "barricade", "kind": "scripted_blocker", "x": BARRICADE_X + 48, "y": BACK_Y - 1,
         "rect": [BARRICADE_X + 8, 32, 80, walk - 32], "art": "timber_barricade_full",
         "note": "blocks actors until the pursuer reaches it (or a fallback timer); then removed "
                 "and swapped to timber_barricade_smashed"},
        {"id": "pursuer_wedge", "kind": "pursuer_end", "x": BAYS[1] * TILE + 152, "y": walk - 1,
         "rect": [BAYS[1] * TILE + 56, kit.WALL_TOP + 38, 192, 218],
         "note": "overshoot end: the pursuer cannot make the turn and noses into the S5 arch window; "
                 "it leaves the lane (no collision) and its riders bail out"},
        {"id": "checkpoint_post_chase", "kind": "checkpoint", "x": 2640, "y": walk - 1},
        {"id": "breakable_shore", "kind": "breakable_prop", "x": STRIKE["x"], "y": walk - 1,
         "rect": [SHORE_ORIGIN[0], SHORE_ORIGIN[1], SHORE_W, SHORE_H], "art": "raking_shore",
         "hits_to_break": 3, "strike_band_y": STRIKE["y_band"],
         "note": "CP-03: any melee hit on the shore advances intact -> damaged_1 -> damaged_2 -> broken"},
        {"id": "falling_block", "kind": "falling_debris", "x": BLOCK_XY[0] + 32, "y": BLOCK_XY[1] + 48,
         "rect": [BLOCK_XY[0], BLOCK_XY[1], 64, 48], "art": "raking_shore_load",
         "land_y": walk, "note": "drops straight down after the break; one impact event on landing"},
        {"id": "hazard_fall_region", "kind": "hazard", "x": HAZARD[0], "y": HAZARD[1], "rect": HAZARD,
         "note": "evaluated once, on the landing tick; the strike position is outside it"},
        {"id": "enemy_spawn_a", "kind": "enemy_spawn", "x": 3136, "y": walk - 1, "archetype": "small_melee"},
        {"id": "enemy_spawn_b", "kind": "enemy_spawn", "x": 3176, "y": walk - 1, "archetype": "small_melee"},
        {"id": "enemy_spawn_c", "kind": "enemy_spawn", "x": 3216, "y": walk - 1,
         "archetype": "small_engineer", "note": "optional third rider"},
        {"id": "protected_exit_region", "kind": "protected", "x": 3232, "y": walk - 1,
         "rect": [3232, 32, cols * TILE - 3232, walk - 32],
         "note": "no breakable, hazard or scripted blocker may overlap; floor is reinforced"},
        {"id": "exit_guild", "kind": "exit", "x": guild_cx, "y": walk - 1,
         "rect": [guild_cx - 40, BACK_Y - 112, 80, 112 + (walk - BACK_Y)],
         "note": "walk into the guild portal; it opens (closed -> open) when a player enters the rect"},
    ]
    views = {
        "s1_intro_640x360": [0, 20, 640, 360],
        "s2_arch_640x360": [496, 20, 640, 360],
        "s3_chase_640x360": [944, 20, 640, 360],
        "s4_widened_640x360": [1968, 20, 640, 360],
        "s5_skirmish_640x360": [2768, 20, 640, 360],
        "s6_guild_640x360": [2944, 20, 640, 360],
        "s5_skirmish_640x400": [2768, 0, 640, 400],
    }
    sample_lighting = ek_sample_lighting()
    return {
        "format": "env-kit-layout-v2",
        "layout_id": "floor01_opening_route_proposal" + ("" if option == "R" else "_ceiling_t"),
        "kit": KIT_ID,
        "kit_contract": CONTRACT_REL,
        "status": "ENV-02 route proposal, full piece set drawn (Claude Sonnet) on the Opus keys, "
                  "awaiting independent Opus review: full opening route for CHASE-01 / CP-03 "
                  "planning; not authored level geometry, not wired into floor01_scene",
        "tile_size": [TILE, TILE],
        "grid_size": [cols, rows],
        "pixel_size": [cols * TILE, rows * TILE],
        "coordinate_system": "top_left origin, x right, y down, rows top_to_bottom; piece x/y = top-left of the piece rect in layout pixels",
        "base_walk_line_y": walk,
        "requirements": {"continuous_floor": True, "max_step_tiles": 2},
        "design_parameters": {
            "pursuer_clearance": {"selected": option, "options": CEILING_OPTIONS,
                                  "rule": "clearance = walking line - lowered cornice underside; DOZ-01 "
                                          "needs >= 164 px (>= 200 recommended): R (212) is the "
                                          "default, T (180) the tight alternative"},
            "pursuer_interface": PURSUER,
            "hero_run_px_s": HERO_RUN_PX_S,
            "hazard": {"rect": HAZARD, "strike": STRIKE, "hits_to_break": 3},
        },
        "segments": [{"id": sid, "grammar": g, "beat": beat, "purpose": purpose,
                      "x": [c0 * TILE, c1 * TILE]} for sid, g, beat, purpose, c0, c1 in SEGMENTS],
        "markers": markers,
        "views": views,
        "parallax_layers": {
            "parallax_far": {"piece": "gallery_far", "x_origin": 115, "y": kit.GALLERY_Y0 + kit.DY},
            "parallax_mid": {"piece": "gallery_mid", "x_origin": 39, "y": kit.GALLERY_Y0 + kit.DY},
        },
        "parallax_rule": "Same as ENV-01: screen x = x_origin + k * strip_width + u - camera_x * parallax.",
        "lighting": sample_lighting,
        "preview": {
            "note": "Art-preview inputs only (not runtime data): approved actor atlases for the readability check.",
            "primary_view": "s5_skirmish_640x360",
            "actor_check": [
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": SHORE_FOOT_X - 12},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": 2880},
                {"manifest": "assets/manifests/carl_book1_h52.json", "x": LEDGE[0] * TILE + 40},
                {"manifest": "assets/manifests/donut_scale_h40.json", "x": LEDGE[0] * TILE + 72},
            ],
        },
        "tile_layers": {"backwall": backwall, "collision": collision},
        "piece_layers": {"backwall_decal": decals, "structure": structure, "prop": props,
                         "foreground": foreground},
    }


def ek_sample_lighting() -> dict:
    import json
    sample = json.loads((ROOT / "assets/levels/floor01/corridor_abc_sample.json").read_text(encoding="utf-8"))
    lighting = dict(sample["lighting"])
    lighting["profile_id"] = "floor01_torchlit_a1_opening"
    return lighting


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def build_all() -> dict:
    atlases = build_atlases()
    grids, manifests, pieces, tiles = {}, {}, [], []
    for name, atlas in atlases.items():
        img, entries = kit.pack_pieces(atlas, ATLAS_WIDTH[name])
        grids[name] = kit.grid_from([img])
        for e in entries:
            entry = {"id": e["id"], "atlas": name, "rect": e["rect"], "role": e["role"],
                     "layer": e["layer"], "collidable": False, "anchor": e["anchor"],
                     "anchor_name": e["anchor_name"], "grid_snap": e["grid_snap"],
                     "grammar": e.get("grammar", ["A", "B", "C"])}
            for key in ("decal", "lights", "breakable"):
                if key in e:
                    entry["state_of" if key == "breakable" else key] = e[key]
            pieces.append(entry)
        manifests[name] = manifest(name, grids[name])
    # soffit tileset (collision role: opaque, collidable, [0, 0, 16, 16] collider)
    soffit = build_soffit()
    img, entries = kit.pack_tiles(soffit, 5)
    grids["floor01_soffit"] = kit.grid_from([img])
    for e in entries:
        tiles.append({"id": e["id"], "atlas": "floor01_soffit", "grid_index": e["grid_index"],
                      "rect": e["rect"], "role": "collision", "collidable": True,
                      "grammar": ["A", "B", "C"], "collider": [0, 0, TILE, TILE],
                      "facing": soffit_facing(e["id"])})
    manifests["floor01_soffit"] = manifest("floor01_soffit", grids["floor01_soffit"],
                                           tiles=[e["id"] for e in entries])
    # animated pieces: one atlas per size (the exporter needs one canvas size per sprite)
    for name, (pid, tag, w, h, n, dur, anchor, anchor_name) in FX.items():
        frames = fx_frames(name)
        grids[name] = kit.grid_from([], layer_names=["main"], durations=[dur] * n,
                                    tags=[{"name": tag, "from": 0, "to": n - 1, "direction": "forward"}],
                                    pivot=tuple(anchor), frames_layers=[{"main": f} for f in frames])
        pieces.append({"id": pid, "atlas": name, "rect": [0, 0, w, h], "role": "prop", "layer": "prop",
                       "collidable": False, "anchor": list(anchor), "anchor_name": anchor_name,
                       "grid_snap": 1, "grammar": ["A", "B", "C"],
                       "animation": {"tag": {"name": tag, "from": 0, "to": n - 1, "direction": "forward"},
                                     "frames": [{"frame": {"x": w * i, "y": 0, "w": w, "h": h},
                                                 "duration": dur} for i in range(n)]}})
        manifests[name] = manifest(name, grids[name], frame_size=[w, h], pivot=anchor, tags=[tag])
    contract = build_contract(tiles, pieces, grids)
    base = __import__("json").loads((ROOT / kit.CONTRACT_REL).read_text(encoding="utf-8"))
    merged = dict(base)
    merged["tiles"] = base["tiles"]
    layouts = {opt: build_layout(merged, opt) for opt in LAYOUTS}
    return {"grids": grids, "manifests": manifests, "contract": contract, "layouts": layouts}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--force", action="store_true", help="overwrite existing outputs")
    parser.add_argument("--no-ase", action="store_true", help="skip rebuilding .ase files")
    args = parser.parse_args(argv)
    grid_paths = {name: ROOT / f"{stem}.grid.json" for name, (stem, _k) in ATLAS_FILES.items()}
    outputs = list(grid_paths.values()) + [CONTRACT_PATH] + list(LAYOUTS.values())
    existing = [p for p in outputs if p.exists()]
    if existing and not args.force:
        print("Refusing to overwrite (pass --force):\n  " +
              "\n  ".join(str(p.relative_to(ROOT)) for p in existing), file=sys.stderr)
        return 1
    built = build_all()
    for name, grid in built["grids"].items():
        kit.dump_grid(grid, grid_paths[name])
        if not args.no_ase:
            ase = grid_paths[name].with_suffix("").with_suffix(".ase")
            subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build",
                            str(grid_paths[name]), str(ase)], check=True, stdout=subprocess.DEVNULL)
        print(f"wrote {grid_paths[name].relative_to(ROOT)} ({grid['canvas'][0]}x{grid['canvas'][1]}, "
              f"{len(grid['palette'])} colours)")
    for name, m in built["manifests"].items():
        kit.dump_json(m, kit.MANIFEST_DIR / f"{name}.json")
    kit.dump_json(built["contract"], CONTRACT_PATH)
    for opt, layout in built["layouts"].items():
        kit.dump_layout(layout, LAYOUTS[opt])
    print("wrote manifests, contract and layouts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
