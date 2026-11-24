#!/usr/bin/env python3
"""F01 danger FX: author the Floor 1 danger visuals (acid pool, blast rings, blast waves, room 5 gate).

Writes one ``pixel-grid-v1`` source per canvas under ``assets/source/props/floor01/``
(``dfx_*.grid.json``) and, with ``--meta``, the per-grid toolkit configs
(``dfx_*.pixelart.json``) and the manifests (``assets/manifests/f01-dfx-*.json``).
The design, the frozen keys and the acceptance targets are in
``docs/02-art/floor01-danger-fx-spec.md``.

Every pixel is drawn from explicit primitives (distance bands, angular dashes,
rectangles and a tiny integer hash for the broken patterns) in named palette
entries: the Floor 01 kit ramps (``floor01_catacomb_kit.json``) and, for the acid
only, the SLIME-01 gel ramp (``slime_01.grid.json``), because the pool is the
slime's own fluid (spec, adopted default DFX-AD1). No concept pixel is cropped,
traced or resampled; no image-model raster output.

The rings and waves are generated **per sim radius** (48 px: the Rot Sticker and the
Boiler drum; 64 px: the thrown dynamite's outer ring), so the drawn edge sits on the
simulation's real reach. The script is deterministic (no randomness, no timestamps):
re-running it reproduces the committed files byte for byte (``tests/test_art_danger_fx.py``).
Timings are ticks at 60 Hz; ``duration_ms = round(ticks * 1000 / 60)``; ``None`` is a
state-driven hold (preview 100 ms).

Usage: ``python3 scripts/author_danger_fx.py [--out DIR] [--meta]``
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import author_gwkit_keys as K  # noqa: E402
from author_gwkit_keys import Canvas  # noqa: E402

ROOT = K.ROOT
OUT = K.OUT
SLIME_GRID = ROOT / "assets/source/characters/slime/slime_01.grid.json"
HOLD_MS = 100
SPEC = "docs/02-art/floor01-danger-fx-spec.md"
GRID_DIR_REL = "assets/source/props/floor01"
MANIFESTS = ROOT / "assets/manifests"

Frame = tuple[Canvas, "int | None"]
TagSpec = tuple[str, list[Frame], str]

# The sim radii the rings and waves are drawn for (px).
RADII = (48, 64)
# Ring and wave canvases: the radius plus a 3 px margin on every side, odd, centred.
MARGIN = 3


def ms_of(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / 60)


def h32(a: int, b: int, c: int) -> int:
    """A tiny deterministic integer hash (no randomness)."""
    v = (a * 73856093) ^ (b * 19349663) ^ (c * 83492791)
    v = (v ^ (v >> 13)) * 1274126177 & 0xFFFFFFFF
    return v ^ (v >> 16)


def palette() -> dict[str, str]:
    """The kit palette, then the SLIME-01 gel entries the acid uses (named as in its grid)."""
    pal = K.kit_palette()
    slime = json.loads(SLIME_GRID.read_text())
    for p in slime["palette"]:
        if p["name"] in ACID_COLOURS:
            pal[p["name"]] = p["rgba"]
    return pal


# ================================================================== ACID ====
# 80 x 10: rows 0..3 are fumes and bubbles above the surface, rows 4..9 are the
# 6 px volume (HAZ-Q18). The presenter crops the canvas to the pool's current width
# (left part from the left cap, right part from the right cap), so the drawn pool is
# exactly the volume's x range at every drain step.
AW, AH = 80, 10
SURF = 4  # the surface row (the volume's top row)
ACID_COLOURS = ("outline_murk", "gel_0_deep", "gel_1_shade", "gel_2_mid", "gel_3_light",
                "gel_4_lit", "sheen_torch", "acid_hot", "bone_gel_lit", "bone_gel_dark")


def acid_body(cv: Canvas, drain: bool) -> None:
    lit, mid, shade, deep = (("gel_2_mid", "gel_1_shade", "gel_0_deep", "gel_0_deep") if drain
                             else ("gel_3_light", "gel_2_mid", "gel_1_shade", "gel_0_deep"))
    for x in range(AW):
        # the meniscus: the surface row is 2 px in from each end, row 5 one in.
        edge = min(x, AW - 1 - x)
        for y in range(SURF, AH):
            if y == SURF and edge < 2:
                continue
            if y == SURF + 1 and edge < 1:
                continue
            if y == AH - 1 or (edge == 0) or (y == SURF + 1 and edge == 1) or (y == SURF and edge == 2):
                c = "outline_murk"
            elif y == SURF:
                c = lit
            else:
                # horizontal runs (4 px segments, hashed): liquid layers, not a dither.
                seg = h32(x // 4, y, 3) % 8
                if y == SURF + 1:
                    c = mid if seg == 0 else lit
                elif y == SURF + 2:
                    c = lit if seg == 1 else (shade if seg == 2 else mid)
                elif y == SURF + 3:
                    c = shade if seg < 3 else mid
                else:
                    c = deep if seg < 3 else shade
            cv.set(x, y, c)


def acid_frame(phase: int, drain: bool) -> Canvas:
    cv = Canvas(AW, AH)
    acid_body(cv, drain)
    sheen = "gel_3_light" if drain else "sheen_torch"
    # the rolling sheen: 3 px streaks on the surface, moving 2 px right per frame.
    for x0 in range(3, AW - 6, 11):
        x = x0 + (2 * phase) % 6
        if 2 < x < AW - 5:
            for k in range(3 if not drain else 2):
                cv.set(x + k, SURF, sheen)
    # acid sparks just under the surface (the hot yellow, sparse).
    if not drain:
        for x0 in range(9, AW - 9, 23):
            x = x0 + (phase * 5) % 9
            cv.set(x, SURF + 1, "acid_hot")
            cv.set(x + 1, SURF + 1, "gel_4_lit")
    # bubbles: each site cycles rise (2 px dome on the surface), swell (3 px), pop (fume).
    sites = [7, 22, 36, 51, 66] if not drain else [12, 30, 47, 63]
    for i, sx in enumerate(sites):
        step = (phase + i) % 4
        if step == 1:  # rise
            cv.set(sx, SURF - 1, "gel_4_lit")
            cv.set(sx + 1, SURF - 1, "gel_3_light")
        elif step == 2:  # swell
            cv.set(sx - 1, SURF - 1, "gel_3_light")
            cv.set(sx, SURF - 1, "sheen_torch" if not drain else "gel_4_lit")
            cv.set(sx + 1, SURF - 1, "gel_4_lit")
            cv.set(sx, SURF - 2, "gel_4_lit")
            cv.set(sx + 1, SURF - 2, "gel_3_light")
        elif step == 3:  # pop: a 2 px fume wisp drifting up
            fume = "bone_gel_lit" if not drain else "bone_gel_dark"
            cv.set(sx + 1, SURF - 2, fume)
            cv.set(sx + 1, SURF - 3, fume)
            cv.set(sx + 2, SURF - 3, "bone_gel_dark")
    if drain:
        # evaporation: more fumes over the whole pool, rising one row per frame.
        for i, x0 in enumerate(range(5, AW - 8, 11)):
            x = x0 + h32(i, phase, 9) % 4
            y = SURF - 1 - (phase + i) % 2
            cv.set(x, y, "bone_gel_dark")
            cv.set(x + 1, y - 1, "bone_gel_dark")
            cv.set(x + 1, y - 2, "bone_gel_lit")
    return cv


def acid_assets() -> list[TagSpec]:
    return [
        ("acid_idle", [(acid_frame(k, False), 8) for k in range(4)], "loop"),
        ("acid_drain", [(acid_frame(k, True), 6) for k in range(2)], "loop"),
    ]


# ============================================================ RING TELLS ====
def polar(cv: Canvas, c: int):
    for y in range(cv.h):
        for x in range(cv.w):
            dx, dy = x - c, y - c
            d = math.hypot(dx, dy)
            a = (math.atan2(dy, dx) / (2 * math.pi)) % 1.0
            yield x, y, d, a


def dash_count(r: int, period: float = 12.0) -> int:
    n = max(8, round(2 * math.pi * r / period))
    return n + (n % 2)  # even, so the two colours alternate without a seam


def ring_march(r: int, phase: int) -> Canvas:
    """Marching hazard dashes on the reach: 2 px of orange/yellow paint at r-1..r+1 with a
    1 px soot line on both sides, 8 of every 12 px painted; ``phase`` shifts half a dash."""
    n = 2 * (r + MARGIN) + 1
    c = r + MARGIN
    cv = Canvas(n, n)
    k = dash_count(r)
    for x, y, d, a in polar(cv, c):
        u = a * k + 0.5 * phase
        i, f = int(u), u - int(u)
        if f >= 0.66:
            continue
        # every yellow dash carries a 2 px tooth pointing at the centre: "inside is the blast".
        inner = r - 3 if (i % 2 == 1 and 0.24 <= f < 0.42) else r - 1
        if inner <= d < r + 1:
            cv.set(x, y, "flame_02" if i % 2 == 0 else "brass_05")
        elif r + 1 <= d < r + 2 or inner - 1 <= d < inner:
            cv.set(x, y, "iron_00")
    return cv


def ring_hot(r: int, flicker: int) -> Canvas:
    """The last ticks before the blast: the ring closes into one solid hot band."""
    n = 2 * (r + MARGIN) + 1
    c = r + MARGIN
    cv = Canvas(n, n)
    outer, inner = ("flame_03", "flame_04") if flicker == 0 else ("flame_04", "flame_05")
    for x, y, d, _a in polar(cv, c):
        if r <= d < r + 1:
            cv.set(x, y, outer)
        elif r - 1 <= d < r:
            cv.set(x, y, inner)
        elif r + 1 <= d < r + 2 or r - 2 <= d < r - 1:
            cv.set(x, y, "iron_00")
    return cv


def ring_assets(r: int) -> list[TagSpec]:
    return [
        ("ring_march", [(ring_march(r, 0), 4), (ring_march(r, 1), 4)], "loop"),
        ("ring_hot", [(ring_hot(r, 0), 2), (ring_hot(r, 1), 2)], "loop"),
    ]


# ============================================================ BLAST WAVE ====
# One shock front per frame (radius as a fraction of the reach), drawn behind the
# actors under the VFX-01 core burst. Nothing is ever drawn outside the reach R: the
# reach frame puts its bright edge on R-2..R and a 1 px soot rim on R..R+1.
def wave_front(cv: Canvas, c: int, r: float, bands: list[tuple[float, float, str, float]], seed: int) -> None:
    """``bands``: (inner offset, outer offset, colour, keep share) relative to ``r``, front
    first. A broken band (keep < 1) is cut into 4 px angular chunks: a chunk is kept when its
    hash share passes, and its inner edge reaches 0..2 px deeper (flame tongues, not blocks)."""
    chunks = max(12, int(2 * math.pi * r / 4))
    for x, y, d, a in polar(cv, c):
        chunk = int(a * chunks)
        for j, (lo, hi, col, keep) in enumerate(bands):
            if keep < 1.0:
                lo -= h32(chunk, j, seed + 40) % 3
            if r + lo <= d < r + hi:
                if keep >= 1.0 or (h32(chunk, j, seed) % 100) < keep * 100:
                    cv.set(x, y, col)
                    break


def puffs(cv: Canvas, c: int, rr: float, count: int, size: int, cols: tuple[str, str], seed: int) -> None:
    """Dust puffs: ``count`` round-cornered clumps (size x size, lit upper left, shaded
    lower right) spaced round the circle ``rr``, jittered by the hash."""
    for i in range(count):
        a = (i + (h32(i, seed, 7) % 100) / 250.0) / count
        rj = rr - h32(i, seed, 8) % 3
        px = c + round(rj * math.cos(2 * math.pi * a)) - size // 2
        py = c + round(rj * math.sin(2 * math.pi * a)) - size // 2
        # a rolled dust cloud: a wide lobe and a smaller one riding up-left of it, lit on
        # top, shaded underneath (no hard diagonal split: dust, not rock).
        lobes = [(px + size / 2, py + size / 2 + 0.5, size / 2 + 0.6, size / 2.8),
                 (px + size / 2 - size / 4, py + size / 2 - size / 4, size / 3.2, size / 3.2)]
        for yy in range(py - 2, py + size + 2):
            for xx in range(px - 2, px + size + 2):
                inside = any(((xx + 0.5 - lx) / rx) ** 2 + ((yy + 0.5 - ly) / ry) ** 2 <= 1.0
                             for lx, ly, rx, ry in lobes)
                if inside:
                    cv.set(xx, yy, cols[0] if yy < py + size / 2 else cols[1])


def clip_to_reach(cv: Canvas, c: int, reach: int) -> Canvas:
    for x, y, d, _a in polar(cv, c):
        if d >= reach + 1:
            cv.set(x, y, None)
    return cv


def wave_frames(R: int) -> list[Frame]:
    n = 2 * (R + MARGIN) + 1
    c = R + MARGIN
    out: list[Frame] = []
    # f0 (2 ticks): the flash ring leaving the core at 0.42 R, white-hot.
    cv = Canvas(n, n)
    wave_front(cv, c, 0.42 * R, [(-1, 1, "flame_05", 1.0), (-3, -1, "flame_04", 1.0),
                                 (-6, -3, "flame_03", 0.6)], 1)
    out.append((clip_to_reach(cv, c, R), 2))
    # f1 (2 ticks): the front at 0.64 R, yellow, a hot band and flame tongues behind.
    cv = Canvas(n, n)
    wave_front(cv, c, 0.64 * R, [(-1, 1, "flame_05", 1.0), (-3, -1, "flame_04", 1.0),
                                 (-6, -3, "flame_03", 0.85), (-10, -6, "flame_02", 0.5)], 2)
    out.append((clip_to_reach(cv, c, R), 2))
    # f2 (2 ticks): 0.84 R, cooling to orange, the first soot behind the tongues.
    cv = Canvas(n, n)
    wave_front(cv, c, 0.84 * R, [(-1, 1, "flame_04", 1.0), (-3, -1, "flame_03", 1.0),
                                 (-6, -3, "flame_02", 0.85), (-10, -6, "flame_01", 0.5),
                                 (-14, -10, "iron_03", 0.3)], 3)
    out.append((clip_to_reach(cv, c, R), 2))
    # f3 (6 ticks) THE REACH (key): the bright front ends exactly on R, a soot rim on R..R+1.
    cv = Canvas(n, n)
    wave_front(cv, c, R, [(0, 1, "iron_00", 1.0), (-2, 0, "flame_03", 1.0), (-4, -2, "flame_02", 1.0),
                          (-7, -4, "flame_01", 0.85), (-11, -7, "flame_00", 0.5),
                          (-16, -11, "iron_03", 0.3)], 4)
    out.append((clip_to_reach(cv, c, R), 6))
    # f4 (5 ticks): the front breaks into warm catacomb dust and embers on the reach.
    cv = Canvas(n, n)
    wave_front(cv, c, R, [(-2, 0, "flame_01", 0.4), (-5, -2, "flame_00", 0.35)], 5)
    puffs(cv, c, R - 5, max(12, R // 4), 7, ("stone_06", "stone_04"), 5)
    out.append((clip_to_reach(cv, c, R), 5))
    # f5 (5 ticks): thin settling dust, sparse, darker.
    cv = Canvas(n, n)
    puffs(cv, c, R - 6, max(9, R // 6), 5, ("stone_05", "stone_03"), 6)
    out.append((clip_to_reach(cv, c, R), 5))
    for cv, _t in out:
        drop_isolated(cv)
    return out


def drop_isolated(cv: Canvas) -> None:
    """Clear opaque pixels with no opaque 8-neighbour (the validators reject them)."""
    lone = []
    for y in range(cv.h):
        for x in range(cv.w):
            if cv.p[y][x] is None:
                continue
            if all(cv.get(x + dx, y + dy) is None
                   for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx, dy) != (0, 0)):
                lone.append((x, y))
    for x, y in lone:
        cv.p[y][x] = None


def wave_assets(R: int) -> list[TagSpec]:
    return [("blast_wave", wave_frames(R), "once")]


# ================================================================== GATE ====
# Room 5 (the Juicer's arena) entrance gate: 28 x 224 = the closed gate's collider
# {0, floor - 224, 28, 224}. Five riveted cold-steel bars, four cross straps, a toothed
# bottom rail biting the floor, a jamb channel on the arena side, rust from the rivets.
GW_, GH = 28, 224
BARS = [1, 6, 11, 16, 21]  # left column of each 3 px bar
STRAPS = [10, 58, 106, 154]  # top row of each 6 px strap (the bottom rail is separate)
RAIL = 204  # top row of the 8 px bottom rail; teeth below it to row 223
GATE_RAISED_ROWS = 24  # the raised gate's rail and teeth hang this far under the lintel


def gate_grate(cv: Canvas, top: int) -> None:
    """Draw the grate with its bottom (teeth tips) on row ``top + 223``; rows above the
    canvas are clipped, so ``top < 0`` draws only the lower part (the raised gate)."""
    for bx in BARS:
        for y in range(top, top + RAIL):
            cv.set(bx, y, "stone_cold_04")
            cv.set(bx + 1, y, "stone_cold_03")
            cv.set(bx + 2, y, "stone_cold_01")
    for sy in STRAPS:
        y = top + sy
        cv.hline(0, 23, y, "iron_00")
        cv.hline(0, 23, y + 1, "stone_cold_04")
        cv.rect(0, y + 2, 23, y + 4, "stone_cold_02")
        cv.hline(0, 23, y + 5, "iron_00")
        for bx in BARS:  # a rivet on every bar crossing: lit head, shadow below-right
            cv.set(bx + 1, y + 2, "stone_cold_04")
            cv.set(bx + 2, y + 3, "iron_00")
    # the bottom rail: heavier, a lit top edge, two rows of shade, soot at the base.
    y = top + RAIL
    cv.hline(0, 23, y, "iron_00")
    cv.hline(0, 23, y + 1, "stone_cold_04")
    cv.rect(0, y + 2, 23, y + 5, "stone_cold_02")
    cv.hline(0, 23, y + 4, "stone_cold_01")
    cv.hline(0, 23, y + 6, "stone_cold_00")
    cv.hline(0, 23, y + 7, "iron_00")
    for bx in BARS:
        cv.set(bx + 1, y + 2, "stone_cold_04")
        cv.set(bx + 2, y + 3, "iron_00")
    # teeth: each bar ends in a spike, 3 px wide tapering to 1 px, rows RAIL+8..223.
    for bx in BARS:
        for k in range(8):
            yy = top + RAIL + 8 + k
            if k < 3:
                cv.set(bx, yy, "stone_cold_04")
                cv.set(bx + 1, yy, "stone_cold_03")
                cv.set(bx + 2, yy, "stone_cold_01")
            elif k < 6:
                cv.set(bx + 1, yy, "stone_cold_03")
                cv.set(bx + 2, yy, "stone_cold_01")
            else:
                cv.set(bx + 1, yy, "stone_cold_02")
                cv.set(bx + 2, yy, "iron_00")
    # rust: short runs down a bar under some rivets (copper ramp, never on the outline).
    for i, bx in enumerate(BARS):
        for j, sy in enumerate(STRAPS + [RAIL]):
            if h32(i, j, 11) % 3 == 0:
                length = 3 + h32(i, j, 12) % 6
                for k in range(length):
                    yy = top + sy + 6 + k
                    if yy >= top + RAIL:
                        break
                    cv.set(bx + 1, yy, "copper_03" if k < length // 2 else "copper_02")
    # grime: the bars darken toward the floor (the lit column steps down one value).
    for bx in BARS:
        for y in range(top + RAIL - 40, top + RAIL):
            if cv.get(bx, y) == "stone_cold_04" and (y + bx) % 3 == 0:
                cv.set(bx, y, "stone_cold_03")


def gate_jamb(cv: Canvas) -> None:
    """The jamb channel on the arena side (x 24..27): the slot the gate drops into."""
    for y in range(GH):
        cv.set(24, y, "iron_00")
        cv.set(25, y, "iron_03" if y % 32 else "iron_05")
        cv.set(26, y, "iron_02")
        cv.set(27, y, "iron_00")
        if y % 32 == 16:  # a bolt every 32 rows
            cv.set(26, y, "iron_06")


def gate_closed() -> Canvas:
    cv = Canvas(GW_, GH)
    gate_grate(cv, 0)
    gate_jamb(cv)
    return cv


def gate_raised() -> Canvas:
    """The gate hauled up into the lintel: only the rail and the teeth hang below the top
    of the doorway (``GATE_RAISED_ROWS``): the threat before it drops."""
    cv = Canvas(GW_, GH)
    gate_grate(cv, GATE_RAISED_ROWS - GH)
    return cv


def gate_assets() -> list[TagSpec]:
    return [("gate_closed", [(gate_closed(), None)], "hold"),
            ("gate_raised", [(gate_raised(), None)], "hold")]


# ================================================================ OUTPUT ====
def assets() -> dict[str, tuple[list[TagSpec], tuple[int, int]]]:
    out: dict[str, tuple[list[TagSpec], tuple[int, int]]] = {
        "dfx_acid": (acid_assets(), (0, AH - 1)),
    }
    for r in RADII:
        c = r + MARGIN
        out[f"dfx_ring_r{r}"] = (ring_assets(r), (c, c))
        out[f"dfx_wave_r{r}"] = (wave_assets(r), (c, c))
    out["dfx_gate"] = (gate_assets(), (0, GH - 1))
    return out


# Frozen design keys (grid frame index): the acid idle key, both march rings, the reach
# frame of both waves, the closed gate.
FROZEN = {"dfx_acid": [0], "dfx_ring_r48": [0], "dfx_ring_r64": [0], "dfx_wave_r48": [3],
          "dfx_wave_r64": [3], "dfx_gate": [0]}
TITLES = {
    "dfx_acid": "acid pool strip (idle, draining)",
    "dfx_ring_r48": "blast reach ring, 48 px (march, hot)",
    "dfx_ring_r64": "blast reach ring, 64 px (march, hot)",
    "dfx_wave_r48": "blast wave to the 48 px reach",
    "dfx_wave_r64": "blast wave to the 64 px reach",
    "dfx_gate": "room 5 entrance gate (closed, raised)",
}
# Evidence scene rows: (frame, x of the pivot column, lift).
SCENES = {
    "dfx_acid": ([(0, 520, 0), (4, 640, 0)], False),
    "dfx_ring_r48": ([(0, 560, 40), (2, 700, 40)], False),
    "dfx_ring_r64": ([(0, 560, 56), (2, 720, 56)], False),
    "dfx_wave_r48": ([(1, 520, 40), (3, 660, 40)], False),
    "dfx_wave_r64": ([(1, 520, 56), (3, 700, 56)], False),
    "dfx_gate": ([(0, 560, 0), (1, 640, 0)], False),
}
STRIPS = {"dfx_acid": ["acid_idle", "acid_drain"], "dfx_ring_r48": ["ring_march", "ring_hot"],
          "dfx_ring_r64": ["ring_march", "ring_hot"], "dfx_wave_r48": ["blast_wave"],
          "dfx_wave_r64": ["blast_wave"], "dfx_gate": ["gate_closed", "gate_raised"]}


def to_grid(tags: list[TagSpec], pivot: tuple[int, int], pal: dict[str, str]) -> dict:
    flat: list[Frame] = []
    tag_rows = []
    for name, frames, _playback in tags:
        tag_rows.append({"name": name, "from": len(flat), "to": len(flat) + len(frames) - 1,
                         "direction": "forward"})
        flat.extend(frames)
    names: list[str] = []
    for cv, _ in flat:
        for row in cv.p:
            for v in row:
                if v is not None and v not in names:
                    names.append(v)
    order = list(pal)
    names.sort(key=order.index)
    if len(names) > len(K.KEYS):
        raise SystemExit("too many colours")
    key_of = {n: K.KEYS[i] for i, n in enumerate(names)}
    w, h = flat[0][0].w, flat[0][0].h
    frames_out = []
    for cv, ticks in flat:
        assert (cv.w, cv.h) == (w, h)
        rows = ["".join("." if v is None else key_of[v] for v in row) for row in cv.p]
        frames_out.append({"duration_ms": ms_of(ticks), "cels": {"body": rows}})
    return {"format": "pixel-grid-v1", "canvas": [w, h], "pivot": list(pivot),
            "palette": [{"key": key_of[n], "name": n, "rgba": pal[n]} for n in names],
            "layers": ["body"], "frames": frames_out, "tags": tag_rows}


def asset_stem(name: str) -> str:
    return "f01-" + name.replace("_", "-")


def tick_tags(tags: list[TagSpec]) -> dict:
    out, i = {}, 0
    for tag, frames, playback in tags:
        out[tag] = {"playback": playback, "contact": "free",
                    "frames": [[i + k, t] for k, (_cv, t) in enumerate(frames)]}
        i += len(frames)
    return out


def config_of(name: str, tags: list[TagSpec], grid: dict) -> dict:
    import pixel_grid_to_ase as pg
    rep = pg.lint_grid(grid)
    w, h = grid["canvas"]
    rules: dict = {"max_colors": 63, "max_isolated_per_frame": 0,
                   "max_orphans_per_frame": max(rep["orphan_count_per_frame"]), "unique_frames": True,
                   "frozen_keys": {"fixture": f"tests/fixtures/{asset_stem(name)}_keys.json",
                                   "frames": FROZEN[name], "layers": ["body"]}}
    actors, shadow = SCENES[name]
    return {
        "format": "pixelart-asset-v1", "asset_id": asset_stem(name),
        "title": f"F01 danger FX {TITLES[name]}",
        "grid": f"{GRID_DIR_REL}/{name}.grid.json", "manifest": f"assets/manifests/{asset_stem(name)}.json",
        "frame_size": [w, h], "pivot": list(grid["pivot"]), "contact_row": None, "tick_hz": 60,
        "hold_ms": HOLD_MS, "facing": "left", "tags": tick_tags(tags), "rules": rules,
        "scene": {"shadow": shadow, "actors": [{"frame": f, "x": x, "lift": lift} for f, x, lift in actors]},
        "review": {"strip_name": "03_state_strip.png", "strip_tags": STRIPS[name]},
    }


def manifest_of(name: str, tags: list[TagSpec], grid: dict) -> dict:
    from scripts.pixelart.validators import frame_digest
    stem = asset_stem(name)
    w, h = grid["canvas"]
    flat = [(tag, k, t) for tag, frames, _p in tags for k, (_cv, t) in enumerate(frames)]
    frames = []
    for i, (tag, k, ticks) in enumerate(flat):
        rows = grid["frames"][i]["cels"]["body"]
        xs = [x for r in rows for x, c in enumerate(r) if c != "."]
        ys = [y for y, r in enumerate(rows) if r.strip(".")]
        frames.append({"index": i, "name": f"{tag}_{k}", "tag": tag, "ticks": ticks,
                       "duration_ms": grid["frames"][i]["duration_ms"], "pivot": list(grid["pivot"]),
                       "rect": {"x": i * w, "y": 0, "w": w, "h": h},
                       "content": [min(xs), min(ys), max(xs), max(ys)],
                       "sha256": frame_digest(grid["frames"][i]["cels"], ["body"])})
    acid = name == "dfx_acid"
    palette_note = ("floor01-catacomb-v2 + SLIME-01 gel ramp (adopted default DFX-AD1)" if acid
                    else "floor01-catacomb-v2")
    return {
        "schema_version": 1, "id": f"{stem.replace('-', '_')}_candidate_v0", "type": "prop_animation",
        "source_path": f"{GRID_DIR_REL}/{name}.ase", "grid_source": f"{GRID_DIR_REL}/{name}.grid.json",
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": palette_note, "frame_size": [w, h], "pivot": list(grid["pivot"]),
        "animation_tags": [t for t, _f, _p in tags], "required_alpha": True,
        "owner": "AI-authored Floor 1 danger FX (fan-concept layer; runtime ids IP-neutral dfx_*)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": f"F01 danger FX {TITLES[name]}, drawn from explicit primitives in named palette entries "
                      "by scripts/author_danger_fx.py",
            "generator": "designed and executed by Claude Opus (spec, keys, frames); independent top-tier review "
                         "pending. No concept pixels copied, no image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": [SPEC, "assets/source/terrain/floor01/floor01_catacomb_kit.json"]
            + (["assets/source/characters/slime/slime_01.grid.json"] if acid else []),
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; "
                   "no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "prop_kit": {
            "status": f"{len(flat)} frames drawn; independent top-tier review pending; drawn by presentation-only "
                      "wiring in src/danger_fx_art.rs",
            "spec": SPEC, "tick_hz": 60, "hold_preview_ms": HOLD_MS,
            "playback": {tag: p for tag, _f, p in tags},
            "frames": frames,
        },
    }


def build(out: Path, meta: bool = False) -> list[Path]:
    pal = palette()
    out.mkdir(parents=True, exist_ok=True)
    written = []
    for name, (tags, pivot) in assets().items():
        grid = to_grid(tags, pivot, pal)
        path = out / f"{name}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n")
        written.append(path)
        if meta:
            cfg = out / f"{name}.pixelart.json"
            cfg.write_text(json.dumps(config_of(name, tags, grid), indent=2) + "\n")
            written.append(cfg)
            man = MANIFESTS / f"{asset_stem(name)}.json"
            man.write_text(json.dumps(manifest_of(name, tags, grid), indent=2) + "\n")
            written.append(man)
    return written


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--meta", action="store_true",
                    help="also write the per-grid pixelart configs next to the grids and the manifests")
    args = ap.parse_args()
    for p in build(args.out, args.meta):
        print(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p)


if __name__ == "__main__":
    main()
