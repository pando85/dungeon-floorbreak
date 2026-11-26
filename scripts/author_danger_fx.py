#!/usr/bin/env python3
"""F01 danger FX: author the Floor 1 danger visuals (acid pool, blast rings, blast waves, room 5 gate, room exit).

Writes one ``pixel-grid-v1`` source per canvas under ``assets/source/props/floor01/``
(``dfx_*.grid.json``) and, with ``--meta``, the per-grid toolkit configs
(``dfx_*.pixelart.json``) and the manifests (``assets/manifests/f01-dfx-*.json``).
The design, the frozen keys and the acceptance targets are in
``docs/02-art/floor01-danger-fx-spec.md``.

Every pixel is drawn from explicit primitives (distance bands, angular dashes,
rectangles and a tiny integer hash for the broken patterns) in named palette
entries: the Floor 01 kit ramps (``floor01_catacomb_kit.json``) and, for the acid
family, the teal ``ACID_RAMP`` (AREA-HAZARDS-01B, adopted default HZB-AD1, which
supersedes the SLIME-01 borrow of DFX-AD1: the acid is its own substance). The splash,
the corrosion decals and the Rot marks of AREA-HAZARDS-01B are drawn by
``scripts/author_hazard_decals.py`` and written by this script's ``build`` with the
other sheets (spec ``docs/02-art/area-hazards-01b-spec.md``). No concept pixel is
cropped, traced or resampled; no image-model raster output.

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
import author_hazard_decals as H  # noqa: E402
from author_gwkit_keys import Canvas  # noqa: E402

ROOT = K.ROOT
OUT = K.OUT
HOLD_MS = 100
SPEC = "docs/02-art/floor01-danger-fx-spec.md"
HZB_SPEC = H.SPEC
# The room exit door has its own spec (ROOM-PORTALS-01); the other sheets share SPEC.
EXIT_SPEC = "docs/02-art/floor01-room-exit-spec.md"
# F01-SIM-TICKS: the gate drop and the slam dust have their own spec.
TICKS_SPEC = "docs/02-art/f01-sim-ticks-spec.md"
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
    """The kit palette, the teal acid ramp (HZB-AD1) and the ROT-01 entries the Rot marks use."""
    pal = K.kit_palette()
    for name, rgba in H.ACID_RAMP:
        assert name not in pal, name
        pal[name] = rgba
    for name, rgba in H.rot_palette().items():
        assert name not in pal, name
        pal[name] = rgba
    return pal


# ================================================================== ACID ====
# 80 x 10: rows 0..3 are fumes and bubbles above the surface, rows 4..9 are the
# 6 px volume (HAZ-Q18). The presenter crops the canvas to the pool's current width
# (left part from the left cap, right part from the right cap), so the drawn pool is
# exactly the volume's x range at every drain step.
AW, AH = 80, 10
SURF = 4  # the surface row (the volume's top row)


def acid_body(cv: Canvas, drain: bool) -> None:
    lit, mid, shade, deep = (("acid_2_mid", "acid_1_shade", "acid_0_deep", "acid_0_deep") if drain
                             else ("acid_3_light", "acid_2_mid", "acid_1_shade", "acid_0_deep"))
    for x in range(AW):
        # the meniscus: the surface row is 2 px in from each end, row 5 one in.
        edge = min(x, AW - 1 - x)
        for y in range(SURF, AH):
            if y == SURF and edge < 2:
                continue
            if y == SURF + 1 and edge < 1:
                continue
            if y == AH - 1 or (edge == 0) or (y == SURF + 1 and edge == 1) or (y == SURF and edge == 2):
                c = "acid_outline"
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
    sheen = "acid_3_light" if drain else "acid_sheen"
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
            cv.set(x, SURF + 1, "acid_fizz")
            cv.set(x + 1, SURF + 1, "acid_4_lit")
    # bubbles: each site cycles rise (2 px dome on the surface), swell (3 px), pop (fume).
    sites = [7, 22, 36, 51, 66] if not drain else [12, 30, 47, 63]
    for i, sx in enumerate(sites):
        step = (phase + i) % 4
        if step == 1:  # rise
            cv.set(sx, SURF - 1, "acid_4_lit")
            cv.set(sx + 1, SURF - 1, "acid_3_light")
        elif step == 2:  # swell
            cv.set(sx - 1, SURF - 1, "acid_3_light")
            cv.set(sx, SURF - 1, "acid_sheen" if not drain else "acid_4_lit")
            cv.set(sx + 1, SURF - 1, "acid_4_lit")
            cv.set(sx, SURF - 2, "acid_4_lit")
            cv.set(sx + 1, SURF - 2, "acid_3_light")
        elif step == 3:  # pop: a 2 px fume wisp drifting up
            fume = "fume_lit" if not drain else "fume_dark"
            cv.set(sx + 1, SURF - 2, fume)
            cv.set(sx + 1, SURF - 3, fume)
            cv.set(sx + 2, SURF - 3, "fume_dark")
    if drain:
        # evaporation: more fumes over the whole pool, rising one row per frame.
        for i, x0 in enumerate(range(5, AW - 8, 11)):
            x = x0 + h32(i, phase, 9) % 4
            y = SURF - 1 - (phase + i) % 2
            cv.set(x, y, "fume_dark")
            cv.set(x + 1, y - 1, "fume_dark")
            cv.set(x + 1, y - 2, "fume_lit")
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
        if r <= d < r + 1:
            # HZB-AD2: the outer band row is the dark tape backing in both dash colours, so
            # the dashes keep an edge on the lit brick (finding F1).
            cv.set(x, y, "flame_00")
        elif inner <= d < r:
            cv.set(x, y, "flame_02" if i % 2 == 0 else "brass_05")
        elif r + 1 <= d < r + 2 or inner - 1 <= d < inner:
            cv.set(x, y, "iron_00")
    return cv


def ring_hot(r: int, flicker: int) -> Canvas:
    """The last ticks before the blast: the ring closes into one solid hot band."""
    n = 2 * (r + MARGIN) + 1
    c = r + MARGIN
    cv = Canvas(n, n)
    # HZB-AD2: the outer row is the dark tape backing (``flame_00``) in both flicker frames; the
    # inner row flickers flame_04 / flame_05.
    outer, inner = "flame_00", ("flame_04" if flicker == 0 else "flame_05")
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


def grate(cv: Canvas, top: int, x0: int, x1: int, bars: list[int], straps: list[int], rail: int,
          salt: int, clip_top: int = 0) -> None:
    """A drop grate between columns ``x0..x1``: 3 px cold-steel ``bars`` (left columns),
    riveted 6 px ``straps`` (rows from the grate's top), an 8 px bottom ``rail`` and 8 rows
    of teeth, its top row on canvas row ``top``. Rows above ``clip_top`` are not drawn (the
    part hidden in the lintel). ``salt`` keys the rust runs (a fixed integer hash)."""
    def put(x: int, y: int, c: str) -> None:
        if y >= clip_top:
            cv.set(x, y, c)

    def hl(y: int, c: str) -> None:
        for x in range(x0, x1 + 1):
            put(x, y, c)

    for bx in bars:
        for y in range(top, top + rail):
            put(bx, y, "stone_cold_04")
            put(bx + 1, y, "stone_cold_03")
            put(bx + 2, y, "stone_cold_01")
    for sy in straps:
        y = top + sy
        hl(y, "iron_00")
        hl(y + 1, "stone_cold_04")
        for yy in range(y + 2, y + 5):
            hl(yy, "stone_cold_02")
        hl(y + 5, "iron_00")
        for bx in bars:  # a rivet on every bar crossing: lit head, shadow below-right
            put(bx + 1, y + 2, "stone_cold_04")
            put(bx + 2, y + 3, "iron_00")
    # the bottom rail: heavier, a lit top edge, two rows of shade, soot at the base.
    y = top + rail
    hl(y, "iron_00")
    hl(y + 1, "stone_cold_04")
    for yy in range(y + 2, y + 6):
        hl(yy, "stone_cold_02")
    hl(y + 4, "stone_cold_01")
    hl(y + 6, "stone_cold_00")
    hl(y + 7, "iron_00")
    for bx in bars:
        put(bx + 1, y + 2, "stone_cold_04")
        put(bx + 2, y + 3, "iron_00")
    # teeth: each bar ends in a spike, 3 px wide tapering to 1 px, 8 rows under the rail.
    for bx in bars:
        for k in range(8):
            yy = top + rail + 8 + k
            if k < 3:
                put(bx, yy, "stone_cold_04")
                put(bx + 1, yy, "stone_cold_03")
                put(bx + 2, yy, "stone_cold_01")
            elif k < 6:
                put(bx + 1, yy, "stone_cold_03")
                put(bx + 2, yy, "stone_cold_01")
            else:
                put(bx + 1, yy, "stone_cold_02")
                put(bx + 2, yy, "iron_00")
    # rust: short runs down a bar under some rivets (copper ramp, never on the outline).
    for i, bx in enumerate(bars):
        for j, sy in enumerate(straps + [rail]):
            if h32(i, j, salt) % 3 == 0:
                length = 3 + h32(i, j, salt + 1) % 6
                for k in range(length):
                    yy = top + sy + 6 + k
                    if yy >= top + rail:
                        break
                    put(bx + 1, yy, "copper_03" if k < length // 2 else "copper_02")
    # grime: the bars darken toward the floor (the lit column steps down one value).
    for bx in bars:
        for y in range(top + rail - 40, top + rail):
            if y >= clip_top and cv.get(bx, y) == "stone_cold_04" and (y + bx) % 3 == 0:
                cv.set(bx, y, "stone_cold_03")


def gate_grate(cv: Canvas, top: int) -> None:
    """Draw the grate with its bottom (teeth tips) on row ``top + 223``; rows above the
    canvas are clipped, so ``top < 0`` draws only the lower part (the raised gate)."""
    grate(cv, top, 0, 23, BARS, STRAPS, RAIL, 11)


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


# F01-SIM-TICKS (spec docs/02-art/f01-sim-ticks-spec.md): the drop, played once from the
# simulation's close tick. The grate falls from the raised height (teeth tips 200 px above
# the floor) on the Boiler weight's gravity, 4 px per tick squared: ``200 - 2 t^2`` at t = 2,
# 4, 6, 8 ticks, 2 ticks per frame; the closed frame (the impact) follows on tick 8. The jamb
# is drawn from the first drop frame: the collider is solid from the close tick on.
GATE_DROP_TICKS = 2
GATE_DROP_HEIGHTS = [200 - 2 * (GATE_DROP_TICKS * (k + 1)) ** 2 for k in range(4)]  # 192, 168, 128, 72


def gate_drop(height: int) -> Canvas:
    """The grate falling: the closed grate lifted ``height`` px (a rigid shift of its rows, so
    the rust and the grime ride with the bars), the jamb in place."""
    grate_only = Canvas(GW_, GH)
    gate_grate(grate_only, 0)
    cv = Canvas(GW_, GH)
    for y in range(GH - height):
        cv.p[y] = grate_only.p[y + height][:]
    gate_jamb(cv)
    return cv


def gate_assets() -> list[TagSpec]:
    return [("gate_closed", [(gate_closed(), None)], "hold"),
            ("gate_raised", [(gate_raised(), None)], "hold"),
            ("gate_drop", [(gate_drop(h), GATE_DROP_TICKS) for h in GATE_DROP_HEIGHTS], "once")]


# ============================================================ SLAM DUST ====
# F01-SIM-TICKS (spec docs/02-art/f01-sim-ticks-spec.md): the floor dust a heavy mass throws
# when it slams down (the Boiler weight on its landing tick, the room 5 gate on its impact).
# 44 x 20, pivot (4, 19): the pivot column is the impact edge (columns 0..3 tuck under the
# object's base), the bottom row stands on the walking line, the plume rolls toward +x (the
# presenter mirrors it for the other side). Five frames played once, 20 ticks: a flat burst
# hugging the floor, the rolling head (the key), the billow, the thinning cloud, the settle.
# Catacomb stone dust in the kit's warm stone ramp (the blast wave's settling puffs use the
# same entries) with iron grit; no glow, no sparkle, no colour of its own.
DW, DH = 44, 20
DUST_PIVOT = (4, DH - 1)
DUST_TICKS = [2, 3, 4, 5, 6]
DUST_KEY = 1  # the rolling head


def cloud(cv: Canvas, lobes: list[tuple[float, float, float, float]], cols: tuple[str, str, str],
          breakup: int, seed: int) -> None:
    """A dust cloud: the union of the ellipses ``lobes`` (cx, cy, rx, ry), each pixel in the
    lit, body or shade entry of ``cols`` by where it sits on its nearest lobe (lit up and to
    the left, toward the torches; shade low and to the right). ``breakup`` percent of 2 x 2
    cells away from the floor row are left open (a thinning cloud has holes, not dither)."""
    for y in range(DH):
        for x in range(DW):
            best = None
            for lx, ly, rx, ry in lobes:
                dx, dy = (x + 0.5 - lx) / rx, (y + 0.5 - ly) / ry
                d = dx * dx + dy * dy
                if d <= 1.0 and (best is None or d < best[0]):
                    best = (d, dx, dy)
            if best is None:
                continue
            if breakup and y < DH - 2 and h32(x // 2, y // 2, seed) % 100 < breakup:
                continue
            _d, dx, dy = best
            tone = 0.8 * dy + 0.4 * dx
            cv.set(x, y, cols[0] if tone < -0.35 else cols[2] if tone > 0.45 else cols[1])


def grit(cv: Canvas, chunks: list[tuple[int, int, int]]) -> None:
    """Thrown grit: 2 x 2 (2 x 1 unless ``tall``) mortar chunks, a dark edge under a stone top."""
    for x, y, tall in chunks:
        cv.set(x, y, "stone_03")
        cv.set(x + 1, y, "iron_03")
        if tall:
            cv.set(x, y + 1, "iron_03")
            cv.set(x + 1, y + 1, "iron_01")


def dust_frames() -> list[Frame]:
    lit, mid, low = ("stone_07", "stone_06", "stone_05"), ("stone_06", "stone_05", "stone_04"), \
        ("stone_05", "stone_04", "stone_03")
    plan = [
        # f0 burst (2 ticks): a flat, fast jet hugging the floor; the first grit flies ahead.
        ([(6, 17, 5, 3), (12, 17.5, 5.5, 2.5), (19, 18.2, 5, 1.9), (25, 18.6, 3.5, 1.5)], lit, 0,
         [(29, 15, 1), (33, 17, 0)]),
        # f1 roll (3 ticks) KEY: the front curls up into a rolling head; grit arcs over it.
        ([(5, 16.5, 5, 3.6), (12, 15.8, 6, 4.3), (20, 15.2, 6, 4.9), (27, 16, 5, 4),
          (29, 12.8, 3.2, 2.8), (33, 17.8, 3.5, 2.2)], lit, 0, [(36, 9, 1), (39, 14, 1), (31, 7, 0)]),
        # f2 billow (4 ticks): puffed up and slowing, the grit down on the floor ahead.
        ([(6, 15.5, 5, 4.5), (13, 13.8, 6, 6), (21, 13.2, 6.5, 6.2), (29, 14.2, 6, 5.6),
          (35, 16.4, 4.5, 3.6), (39, 18.2, 3, 1.8)], mid, 4, [(41, 18, 1)]),
        # f3 thin (5 ticks): risen, darker and torn into separate puffs.
        ([(8, 13.2, 4.2, 4), (16.5, 11, 4.8, 4.8), (26, 10.4, 5, 4.6), (34.5, 12.6, 4, 3.6),
          (40, 17.4, 2.8, 2.3), (15, 18.6, 7, 1.5)], low, 0, [(41, 18, 1)]),
        # f4 settle (6 ticks): three high wisps fading and a haze line on the floor.
        ([(14, 8.6, 3.8, 2.4), (24.5, 7.8, 4.6, 2.8), (33.5, 9.6, 3.4, 2.3), (18, 18.7, 14, 1.4)],
         ("stone_04", "stone_04", "stone_03"), 0, []),
    ]
    out: list[Frame] = []
    for i, (lobes, cols, breakup, chunks) in enumerate(plan):
        cv = Canvas(DW, DH)
        cloud(cv, lobes, cols, breakup, 60 + i)
        grit(cv, chunks)
        drop_isolated(cv)
        out.append((cv, DUST_TICKS[i]))
    return out


def dust_assets() -> list[TagSpec]:
    return [("slam_dust", dust_frames(), "once")]


# ================================================================== EXIT ====
# The room exit of the f01 rooms 2, 4, 4a, 4b and 6 (ROOM-PORTALS-01; spec
# docs/02-art/floor01-room-exit-spec.md): 96 x 136 (the replaced Guild door's height), a
# dressed-stone doorway around the sidecar's 64 x 96 exit (the opening is exactly the
# `defeat_roster` exit gate's collider), a two-course header with the grate's slot, a
# lamp-lit passage behind it and the drop grate of the room 5 gate (same bars, straps,
# rail, teeth and rust) closing the opening.
EW, EH = 96, 136
EJ = 16  # jamb width: the opening is columns EJ..EW - EJ - 1
EO_TOP = 40  # first row of the opening (rows above: cap, two header courses, the slot)
EXIT_BARS = [EJ + 1 + 6 * k for k in range(11)]  # 3 px bars, 3 px gaps
EXIT_STRAPS = [12, 46]
EXIT_RAIL = 80  # grate rows: bars to 79, rail 80..87, teeth 88..95 (tips on the floor row)
EXIT_RAISED_ROWS = 16  # the raised grate's rail and teeth hang this far under the slot
LAMP_X = 67  # the passage lamp's cage (columns 67..70), between two bars
LAMP_Y = EO_TOP + 18  # the cage's top row (the Guild door's grille light sits on it)


def ashlar_block(cv: Canvas, x0: int, y0: int, x1: int, y1: int, seed: int) -> None:
    """One dressed warm-stone block lit from the top left: a lit top row and left column,
    a shaded bottom row and right column, a few pits from a fixed hash."""
    cv.rect(x0, y0, x1, y1, "stone_05")
    cv.hline(x0, x1, y0, "stone_07")
    cv.vline(x0, y0, y1, "stone_06")
    cv.hline(x0, x1, y1, "stone_03")
    cv.vline(x1, y0 + 1, y1, "stone_04")
    for y in range(y0 + 2, y1 - 1):
        for x in range(x0 + 2, x1 - 1):
            if h32(x, y, seed) % 23 == 0:
                cv.set(x, y, "stone_04")


def exit_frame_stone(cv: Canvas) -> None:
    """The jambs (16-row courses laid up from the floor, bonded every other course), the
    cap course, a header course and the lintel with its keystone, the grate's iron slot
    under it, mortar between the blocks."""
    right = EW - EJ
    cv.rect(0, 0, EW - 1, EO_TOP - 1, "mortar_02")
    for x0, x1 in ((0, EJ - 1), (right, EW - 1)):
        cv.rect(x0, EO_TOP, x1, EH - 1, "mortar_02")
        for c, y1 in enumerate(range(EH - 1, EO_TOP, -16)):
            y0 = max(y1 - 15, EO_TOP)
            if c % 2:  # a bond joint: two short blocks
                mid = x0 + 6 if x0 == 0 else x0 + 9
                ashlar_block(cv, x0, y0, mid - 1, y1 - 1, 31 + c + x0)
                ashlar_block(cv, mid + 1, y0, x1, y1 - 1, 41 + c + x0)
            else:
                ashlar_block(cv, x0, y0, x1, y1 - 1, 51 + c + x0)
    # the cap course: a projecting ledge across the whole width.
    cv.hline(0, EW - 1, 0, "stone_00")
    cv.hline(0, EW - 1, 1, "stone_08")
    cv.rect(0, 2, EW - 1, 3, "stone_06")
    cv.hline(0, EW - 1, 4, "stone_02")
    # the header course: three blocks, rows 5..18.
    ashlar_block(cv, 0, 5, 27, 18, 64)
    ashlar_block(cv, 29, 5, 66, 18, 65)
    ashlar_block(cv, 68, 5, EW - 1, 18, 66)
    # the lintel: two long blocks and a keystone breaking the header joint, rows 20..39.
    ashlar_block(cv, 0, 20, 37, 39, 61)
    ashlar_block(cv, 58, 20, EW - 1, 39, 62)
    ashlar_block(cv, 39, 16, 56, 39, 63)
    # the grate's slot: an iron channel across the opening, two bolts.
    s = EO_TOP - 4
    cv.hline(EJ - 2, right + 1, s, "iron_00")
    cv.rect(EJ - 2, s + 1, right + 1, s + 2, "iron_02")
    cv.hline(EJ - 2, right + 1, s + 3, "iron_00")
    for bx in (EJ + 2, right - 3):
        cv.set(bx, s + 1, "iron_06")
        cv.set(bx + 1, s + 2, "iron_00")
    # the guide grooves the grate runs in (the inner jamb faces).
    cv.vline(EJ - 1, EO_TOP, EH - 1, "iron_00")
    cv.vline(right, EO_TOP, EH - 1, "iron_00")
    # the outer outline against the room's brick.
    cv.vline(0, 1, EH - 1, "stone_00")
    cv.vline(EW - 1, 1, EH - 1, "stone_00")
    # the cap's chamfered ends: the room's brick shows in the two top corners.
    for x, y in ((0, 0), (1, 0), (0, 1)):
        cv.set(x, y, None)
        cv.set(EW - 1 - x, y, None)
    cv.set(1, 1, "stone_00")
    cv.set(EW - 2, 1, "stone_00")


def exit_passage(cv: Canvas) -> None:
    """The passage behind the opening: dark brick courses lit warm by one caged lamp on the
    right reveal, a shadowed left reveal, the lintel's soffit and the passage floor."""
    x0, x1 = EJ, EW - EJ - 1
    lamp = (LAMP_X + 1.5, LAMP_Y + 6.0)
    for y in range(EO_TOP, EH):
        for x in range(x0, x1 + 1):
            d = math.hypot(x - lamp[0], (y - lamp[1]) * 1.3)
            course = (y - EO_TOP) // 6
            joint = (y - EO_TOP) % 6 == 5 or (x + 6 * (course % 2)) % 12 == 0
            if d < 7:
                c = "brick_05" if not joint else "brick_03"
            elif d < 13:
                c = "brick_04" if not joint else "brick_02"
            elif d < 21:
                c = "brick_03" if not joint else "brick_01"
            elif d < 32:
                c = "brick_02" if not joint else "brick_00"
            else:
                c = "brick_01" if not joint else "brick_00"
            cv.set(x, y, c)
    # the soffit under the lintel and the two reveals (the opening's depth).
    cv.rect(x0, EO_TOP, x1, EO_TOP + 2, "stone_01")
    cv.rect(x0, EO_TOP, x0 + 2, EH - 1, "stone_01")
    cv.vline(x0 + 3, EO_TOP + 3, EH - 1, "stone_00")
    cv.rect(x1 - 2, EO_TOP, x1, EH - 1, "stone_02")
    for y in range(EO_TOP + 3, EH):
        if abs(y - lamp[1]) < 14:
            cv.set(x1 - 2, y, "stone_04")
    # the passage floor, a lit spill under the lamp.
    for y in range(EH - 8, EH):
        for x in range(x0 + 4, x1 - 2):
            e = ((x - lamp[0]) / 18) ** 2 + ((y - (EH - 1)) / 6) ** 2
            if e < 0.45:
                c = "floor_05"
            elif e < 1.0:
                c = "floor_04"
            else:
                c = "floor_02" if y > EH - 5 else "floor_01"
            cv.set(x, y, c)
    cv.hline(x0 + 4, x1 - 3, EH - 9, "floor_00")
    # the lamp: a bracket into the right reveal, a brass cage, a small flame.
    lx, ly = LAMP_X, LAMP_Y
    cv.hline(lx + 3, x1 - 1, ly + 2, "iron_02")
    cv.hline(lx, lx + 3, ly + 1, "brass_02")
    cv.vline(lx, ly + 2, ly + 8, "brass_02")
    cv.vline(lx + 3, ly + 2, ly + 8, "brass_02")
    cv.hline(lx, lx + 3, ly + 9, "brass_01")
    cv.rect(lx + 1, ly + 2, lx + 2, ly + 8, "flame_02")
    cv.rect(lx + 1, ly + 4, lx + 2, ly + 7, "flame_03")
    cv.set(lx + 1, ly + 5, "flame_04")
    cv.set(lx + 1, ly, "brass_04")
    cv.set(lx + 2, ly, "brass_03")


def exit_closed() -> Canvas:
    """The roster lives: the grate is down over the whole opening, teeth on the floor row."""
    cv = Canvas(EW, EH)
    exit_passage(cv)
    grate(cv, EH - 96, EJ, EW - EJ - 1, EXIT_BARS, EXIT_STRAPS, EXIT_RAIL, 21, EO_TOP)
    exit_frame_stone(cv)
    return cv


def exit_open() -> Canvas:
    """The way is open: the grate hauled up into the lintel, only its rail and teeth hang
    under the slot (``EXIT_RAISED_ROWS``); the lamp-lit passage reads through."""
    cv = Canvas(EW, EH)
    exit_passage(cv)
    grate(cv, EO_TOP + EXIT_RAISED_ROWS - 96, EJ, EW - EJ - 1, EXIT_BARS, EXIT_STRAPS,
          EXIT_RAIL, 21, EO_TOP)
    exit_frame_stone(cv)
    return cv


def exit_assets() -> list[TagSpec]:
    return [("exit_closed", [(exit_closed(), None)], "hold"),
            ("exit_open", [(exit_open(), None)], "hold")]


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
    out["dfx_slam_dust"] = (dust_assets(), DUST_PIVOT)
    out["dfx_exit"] = (exit_assets(), (0, EH - 1))
    out.update(H.assets())  # AREA-HAZARDS-01B: the splash, the corrosion decals, the Rot marks
    return out


# Frozen design keys (grid frame index): the acid idle key, both march rings, the reach
# frame of both waves, the closed gate.
FROZEN = {"dfx_acid": [0], "dfx_ring_r48": [0], "dfx_ring_r64": [0], "dfx_wave_r48": [3],
          "dfx_wave_r64": [3], "dfx_gate": [0], "dfx_exit": [0, 1], "dfx_slam_dust": [DUST_KEY], **H.FROZEN}
TITLES = {
    "dfx_acid": "acid pool strip (idle, draining)",
    "dfx_ring_r48": "blast reach ring, 48 px (march, hot)",
    "dfx_ring_r64": "blast reach ring, 64 px (march, hot)",
    "dfx_wave_r48": "blast wave to the 48 px reach",
    "dfx_wave_r64": "blast wave to the 64 px reach",
    "dfx_gate": "room 5 entrance gate (closed, raised, drop)",
    "dfx_slam_dust": "slam dust (Boiler weight landing, room 5 gate impact)",
    "dfx_exit": "room exit door, rooms 2, 4, 4a, 4b and 6 (closed, open)",
    **H.TITLES,
}
# Evidence scene rows: (frame, x of the pivot column, lift).
SCENES = {
    "dfx_acid": ([(0, 520, 0), (4, 640, 0)], False),
    "dfx_ring_r48": ([(0, 560, 40), (2, 700, 40)], False),
    "dfx_ring_r64": ([(0, 560, 56), (2, 720, 56)], False),
    "dfx_wave_r48": ([(1, 520, 40), (3, 660, 40)], False),
    "dfx_wave_r64": ([(1, 520, 56), (3, 700, 56)], False),
    "dfx_gate": ([(0, 560, 0), (1, 640, 0)], False),
    "dfx_slam_dust": ([(1, 520, 0), (2, 600, 0)], False),
    "dfx_exit": ([(0, 520, 0), (1, 660, 0)], False),
    **H.SCENES,
}
STRIPS = {"dfx_acid": ["acid_idle", "acid_drain"], "dfx_ring_r48": ["ring_march", "ring_hot"],
          "dfx_ring_r64": ["ring_march", "ring_hot"], "dfx_wave_r48": ["blast_wave"],
          "dfx_wave_r64": ["blast_wave"], "dfx_gate": ["gate_closed", "gate_raised", "gate_drop"],
          "dfx_slam_dust": ["slam_dust"],
          "dfx_exit": ["exit_closed", "exit_open"], **H.STRIPS}


def to_grid(tags: list[TagSpec], pivot: tuple[int, int], pal: dict[str, str],
            fixed: dict[str, str] | None = None) -> dict:
    """The ``pixel-grid-v1`` source of one sheet. Keys are assigned in palette order, unless
    ``fixed`` (name -> key character) pins them: the acid family keeps the key characters of
    ``ACID_RAMP`` (so every frozen key is a verbatim row copy) and the Rot marks keep the
    ROT-01 grid's."""
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
    key_of = {n: K.KEYS[i] for i, n in enumerate(names)} if fixed is None else {n: fixed[n] for n in names}
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
    orphans = max(rep["orphan_count_per_frame"])
    if name in H.HAZARD_SHEETS:  # spec 9: the largest measured frame plus 2, at most 32
        orphans = min(orphans + 2, 32)
    rules: dict = {"max_colors": 63, "max_isolated_per_frame": 0,
                   "max_orphans_per_frame": orphans, "unique_frames": True}
    if name in FROZEN:
        rules["frozen_keys"] = {"fixture": f"tests/fixtures/{asset_stem(name)}_keys.json",
                                "frames": FROZEN[name], "layers": ["body"]}
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
    exit_door = name == "dfx_exit"
    hazard = name in H.HAZARD_SHEETS  # AREA-HAZARDS-01B sheets
    dust = name == "dfx_slam_dust"  # F01-SIM-TICKS
    gate = name == "dfx_gate"
    ring = name.startswith("dfx_ring")
    spec = EXIT_SPEC if exit_door else TICKS_SPEC if dust else (HZB_SPEC if hazard else SPEC)
    if hazard or dust:
        review = "independent top-tier art review pending"
    elif gate:
        review = ("independent top-tier review passed on the closed and raised frames; the F01-SIM-TICKS "
                  "drop frames await their review")
    elif acid or ring:
        review = "independent top-tier review passed on the first version; the AREA-HAZARDS-01B change awaits its review"
    else:
        review = "independent top-tier review passed"
    # The acid family carries the teal ramp's palette id (HZB-AD1), the Rot marks ROT-01's; the
    # kit sheets carry the kit's.
    palette_note = H.palette_id(name) or "floor01-catacomb-v2"
    used = ""
    if hazard and name in H.ROT_FAMILY:
        used = "ROT-01 entries only (adopted default HZB-AD5)"
        refs = [HZB_SPEC, H.ROT_GRID_REL]
    elif hazard:
        used = "the teal acid ramp (adopted default HZB-AD1)" + (
            "; the dry subset only (HZB-AD4)" if name != "dfx_acid_splash" else "")
        refs = [HZB_SPEC] + H.KEY_FILES.get(name, [])
    elif acid:
        used = "the teal acid ramp (adopted default HZB-AD1)"
        refs = [SPEC, HZB_SPEC]
    else:
        refs = [spec, "assets/source/terrain/floor01/floor01_catacomb_kit.json"]
    if exit_door:
        refs += [SPEC, "assets/source/props/floor01/dfx_gate.grid.json"]
    if gate or dust:
        refs += [TICKS_SPEC] if gate else [SPEC]
    wall_pivot = [0, 0]
    for f in frames:
        if f["tag"].endswith("_w"):
            f["pivot"] = wall_pivot
    if hazard:
        generator = ("designed by Claude Opus (spec, keys); executed by Claude Sonnet (remaining frames, "
                     "export, validators); independent art-director review by a separate Claude Opus agent "
                     "pending. No concept pixels copied, no image-model raster output, no tracing, no resampling")
    elif dust:
        generator = ("designed and executed by Claude Opus (spec, key, frames); independent art-director review "
                     "by a separate Claude Opus agent pending. No concept pixels copied, no image-model raster "
                     "output, no tracing, no resampling")
    else:
        generator = ("designed and executed by Claude Opus (spec, keys, frames); independent art-director review "
                     "by a separate Claude Opus agent. No concept pixels copied, no image-model raster output, "
                     "no tracing, no resampling")
    if gate:
        generator += ("; the drop frames (F01-SIM-TICKS) designed and executed by Claude Opus, their independent "
                      "review pending")
    generator += ("; the grate reuses the room 5 gate's drawing code (dfx_gate)" if exit_door else "")
    if acid:
        generator += "; recoloured to the teal ramp, rows unchanged (adopted default HZB-AD1, executed by Claude Sonnet)"
    if ring:
        generator += "; outer band row backed with flame_00 (adopted default HZB-AD2, executed by Claude Sonnet)"
    wiring = ("src/room_exit_art.rs" if exit_door else "src/acid_splash.rs" if name == "dfx_acid_splash"
              else "src/hazard_decals.rs" if hazard else "src/danger_fx_art.rs")
    tail = f"; palette: {used}" if used else ""
    return {
        "schema_version": 1, "id": f"{stem.replace('-', '_')}_candidate_v0", "type": "prop_animation",
        "source_path": f"{GRID_DIR_REL}/{name}.ase", "grid_source": f"{GRID_DIR_REL}/{name}.grid.json",
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": palette_note, "frame_size": [w, h], "pivot": list(grid["pivot"]),
        "animation_tags": [t for t, _f, _p in tags], "required_alpha": True,
        "owner": "AI-authored Floor 1 danger FX (fan-concept layer; runtime ids IP-neutral dfx_*)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": f"F01 danger FX {TITLES[name]}, drawn from explicit primitives in named palette "
                      f"entries by scripts/{'author_hazard_decals.py via ' if hazard else ''}author_danger_fx.py{tail}",
            "generator": generator,
            "generated_art": True,
            "input_reference_ids": refs,
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; "
                   "no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "prop_kit": {
            "status": f"{len(flat)} frames drawn; {review}, owner art and game-feel review "
                      "pending; drawn by presentation-only "
                      f"wiring in {wiring}",
            "spec": spec, "tick_hz": 60, "hold_preview_ms": HOLD_MS,
            "playback": {tag: p for tag, _f, p in tags},
            "frames": frames,
        },
    }


def build(out: Path, meta: bool = False) -> list[Path]:
    pal = palette()
    out.mkdir(parents=True, exist_ok=True)
    written = []
    for name, (tags, pivot) in assets().items():
        grid = to_grid(tags, pivot, pal, H.key_map(name))
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
