#!/usr/bin/env python3
"""AREA-GOBLINWORKSHOP-01: author the full Goblin Workshop prop kit grids (keys + execution frames).

Writes one ``pixel-grid-v1`` source per canvas under ``assets/source/props/floor01/``
(``gw_*.grid.json``). The frozen key frames come straight from the functions of
``author_gwkit_keys.py`` (they stay byte-identical; the only changes since the
silhouette gate are the gate-approved edits listed in the spec, section 8). Every
execution frame (``E`` in the spec) is derived from its key by explicit pixel
operations in the named Floor 01 kit palette; no concept pixel is cropped, traced
or resampled.

The script is deterministic (no randomness, no timestamps): re-running it must
reproduce the committed grids byte for byte. Timings are in ticks at 60 Hz and
become ``duration_ms = round(ticks * 1000 / 60)``; ``None`` ticks is a state-driven
hold (preview duration 100 ms).

Usage: ``python3 scripts/author_gwkit_frames.py [--out DIR]``
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import author_gwkit_keys as K  # noqa: E402
from author_gwkit_keys import Canvas  # noqa: E402

ROOT = K.ROOT
OUT = K.OUT
HOLD_MS = 100

# (tag name, [(canvas, ticks or None), ...], playback, contact) per grid; the key frames
# come first in their tag. ``FROZEN[grid]`` lists the key frame indices.
Frame = tuple[Canvas, "int | None"]
TagSpec = tuple[str, list[Frame], str, str]


def ms_of(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / 60)


def px(cv: Canvas, pts, c: str) -> Canvas:
    for x, y in pts:
        cv.set(x, y, c)
    return cv


# ================================================================ BOILER ====
FLAME = ["flame_00", "flame_01", "flame_02", "flame_03", "flame_04"]


def firebox_step(cv: Canvas, n: int) -> Canvas:
    """Brighten the firebox glow by ``n`` ramp steps (capped at flame_04)."""
    out = cv.copy()
    mapping = {FLAME[i]: FLAME[min(i + n, 4)] for i in range(5)}
    out.recolor(68, 175, 124, 203, mapping)
    return out


def move_ball(cv: Canvas, dy: int) -> Canvas:
    """Move the lever ball (x 84..93, y 3..13) ``dy`` px; the lever arm stays."""
    out = cv.copy()
    ball = {(x, y): cv.get(x, y) for x in range(84, 94) for y in range(3, 14) if cv.get(x, y) is not None}
    for x in range(84, 94):
        for y in range(3, 14):
            out.set(x, y, None)
    for x in range(84, 91):  # lever arm rows
        out.set(x, 7, "iron_00")
        out.set(x, 8, "iron_00")
    for (x, y), v in ball.items():
        out.set(x, y + dy, v)
    return out


def rattle(cv: Canvas) -> Canvas:
    """Shift the shell (rows 0..167, columns 0..171) 1 px right; plinth, flange and nozzle stay."""
    out = cv.copy()
    for y in range(0, 168):
        row = cv.p[y][:]
        for x in range(0, 172):
            out.p[y][x + 1] = row[x]
        out.p[y][0] = None
    return out


def boiler_frames():
    b0 = K.boiler_intact()
    bd = K.boiler_damaged(b0)
    gx, gy = 48, 66
    idle1 = firebox_step(b0, 1)
    idle2 = firebox_step(b0, 1)
    px(idle2, [(gx + 5, gy - 4)], "iron_00")  # gauge needle +1 px
    idle3 = move_ball(b0, -1)
    s1 = move_ball(firebox_step(b0, 2), -2)
    px(s1, [(53, 62), (54, 61), (55, 60)], "iron_00")  # needle swings into the red arc
    px(s1, [(70, 11), (71, 10), (71, 11), (72, 9)], "stone_10")
    px(s1, [(70, 10), (72, 10)], "gal_10")
    s2 = rattle(move_ball(firebox_step(b0, 2), -2))
    px(s2, [(54, 61), (55, 60), (54, 62), (55, 61)], "iron_00")
    px(s2, [(70, 11), (71, 10), (72, 10), (73, 9), (73, 10)], "stone_10")
    px(s2, [(71, 11), (72, 11)], "gal_10")
    # Damaged 2: embers flicker, one soot drip 1 px longer.
    d2 = bd.copy()
    for x, y in ((85, 196), (90, 197), (95, 196), (100, 196), (105, 197), (110, 196)):
        if d2.get(x, y) == "flame_01":
            d2.set(x, y, "flame_00")
    for x, y in ((79, 196), (88, 197), (98, 197), (108, 197), (82, 197)):
        if d2.get(x, y) in ("flame_00", "iron_01", "iron_03", "iron_02"):
            d2.set(x, y, "flame_01")
    ys = [y for y in range(33, 75) for x in (92, 93, 94) if d2.get(x, y) in ("iron_01", "iron_02")]
    ylow = max(ys)
    for x in (92, 93):
        if d2.get(x, ylow) in ("iron_01", "iron_02") and d2.get(x, ylow + 1) is not None \
                and d2.get(x, ylow + 1).startswith("copper_"):
            d2.set(x, ylow + 1, "iron_02")
            break
    return [
        ("boiler_idle", [(b0, 8), (idle1, 8), (idle2, 8), (idle3, 8)], "loop", "grounded"),
        ("boiler_strain", [(s1, 4), (s2, 4)], "loop", "grounded"),
        ("boiler_damaged", [(bd, 10), (d2, 10)], "loop", "grounded"),
    ]


# ================================================================= VALVE ====
def valve_wheel_rotated(cv: Canvas) -> Canvas:
    out = cv.copy()
    out.hline(9, 22, 5, "iron_00")
    out.rect(15, 4, 16, 6, "brass_03")
    out.set(14, 5, "brass_03")
    out.set(17, 5, "brass_03")
    px(out, [(14, 6), (13, 6), (12, 7), (11, 7)], "iron_02")  # spoke to lower left
    px(out, [(17, 4), (18, 4), (19, 3), (20, 3)], "iron_02")  # spoke to upper right
    px(out, [(14, 4), (13, 4), (12, 3)], "iron_02")           # spoke to upper left
    px(out, [(17, 6), (18, 6), (19, 7)], "iron_02")           # spoke to lower right
    return out


def shift_valve_right(cv: Canvas) -> Canvas:
    """Body 1 px right (rows 0..43); the bolted base plate (rows 44..47) stays."""
    out = cv.copy()
    for y in range(0, 44):
        row = cv.p[y][:]
        for x in range(0, 31):
            out.p[y][x + 1] = row[x]
        out.p[y][0] = None
    return out


def valve_frames():
    v0 = K.valve_intact()
    spent = K.valve_spent()
    puff = [(22, 11), (22, 12), (23, 11)]
    wa = v0.copy()
    px(wa, puff, "stone_10")
    wb = shift_valve_right(valve_wheel_rotated(v0))
    px(wb, [(23, 11), (23, 12), (24, 11)], "stone_10")

    def jets(neck_rows: int, side_len: int, side_up: bool) -> Canvas:
        cv = spent.copy()
        # Neck jet: a short white spit straight up from the torn neck.
        for i in range(neck_rows):
            y = 11 - i
            half = 1 + (i // 2)
            for x in range(15 - half, 19 + half - 1):
                if cv.get(x, y) is None:
                    cv.set(x, y, "stone_10" if abs(x - 16.5) < half else "gal_10")
        # Side jet: spits out of the split side pipe, between the petals.
        for i in range(side_len):
            x = 28 + i
            for y in range(19 - (1 if side_up else 0), 25 + (1 if side_up else 0)):
                if cv.get(x, y) is None:
                    cv.set(x, y, "stone_10" if 20 <= y <= 23 else "gal_10")
        return cv

    vent = [jets(3, 3, False), jets(5, 4, True), jets(4, 3, True)]
    w1 = spent.copy()
    px(w1, [(15, 11), (16, 10), (16, 9)], "stone_10")
    w2 = spent.copy()
    px(w2, [(16, 11), (17, 10), (17, 9), (18, 8)], "stone_10")
    px(w2, [(15, 11)], "gal_10")
    return [
        ("valve_intact", [(v0, None)], "hold", "grounded"),
        ("valve_warning", [(wa, 4), (wb, 4)], "loop", "grounded"),
        ("valve_venting", [(v, 4) for v in vent], "loop", "grounded"),
        ("valve_spent", [(spent, 10), (w1, 10), (w2, 10)], "loop", "grounded"),
    ]


# ================================================================= BRACE ====
def brace_frames():
    sup = K.brace_supported()
    weak = K.brace_weakened()
    stump = K.brace_stump()
    creak = weak.copy()
    # The wedge pin (x 0..6) jiggles 1 px up; two dust grains fall from the split.
    pin = {(x, y): weak.get(x, y) for x in range(0, 7) for y in range(15, 24)}
    for (x, y) in pin:
        creak.set(x, y, None)
    for (x, y), v in pin.items():
        if v is not None:
            creak.set(x, y - 1, v)
    px(creak, [(8, 24)], "wood_05")
    px(creak, [(23, 22)], "wood_05")
    return [
        ("brace_supported", [(sup, None)], "hold", "grounded"),
        ("brace_weakened", [(weak, 6), (creak, 6)], "loop", "grounded"),
        ("brace_stump", [(stump, None)], "hold", "grounded"),
    ]


# ================================================================ WEIGHT ====
def weight_frames():
    hang = K.weight_hang()
    fall = Canvas(K.WW, K.WH)
    K.weight_body(fall, 16)
    fall.rect(31, 6, 32, 16, "stone_cold_03")  # shackle pin straight up, chain gone
    fall.outline(K.OUT_C)
    for x, y0, y1, c in ((20, 3, 12, "stone_cold_02"), (44, 1, 9, "stone_cold_01"),
                         (54, 5, 14, "stone_cold_02")):
        for y in range(y0, y1 + 1):
            fall.set(x, y, c)
    return [
        ("weight_hang", [(hang, None)], "hold", "grounded"),
        ("weight_fall", [(fall, None)], "hold", "grounded"),
    ]


def remnant_frames():
    return [("remnant", [(K.remnant(), None)], "hold", "grounded")]


# =================================================================== RIG ====
def rig_frames():
    rig = K.rig()
    base = rig.copy()
    base.rect(21, 19, 66, 23, None)  # the return chain is gone from under the beam

    # The return chain ran into the weight sheave from the left, so its loose end
    # leaves the pulley rim on the left (x 66..68, rows 14..15) and hangs down the
    # left cheek of the sheave block, 3 px wide, to the canvas bottom: an 11 px path
    # inside the frozen 24-row canvas, clear of the weight chain below the lead-out.
    def rim(cv: Canvas) -> Canvas:
        px(cv, [(67, 14), (68, 14)], "stone_cold_03")
        px(cv, [(66, 15), (67, 15)], "stone_cold_02")
        px(cv, [(66, 14)], "stone_cold_04")
        return cv

    def link_face(cv: Canvas, x: int, y: int) -> None:
        """A face-on link, 3 x 4 with a 1 x 2 eye (the gw_chain tile's look, one px narrower)."""
        cv.rect(x, y, x + 2, y + 3, "stone_cold_02")
        cv.vline(x, y + 1, y + 2, "stone_cold_04")
        cv.vline(x + 1, y + 1, y + 2, "iron_00")
        cv.set(x + 2, y + 3, "stone_cold_00")

    settled = rim(base.copy())
    link_face(settled, 64, 16)
    settled.vline(65, 20, 21, "stone_cold_03")  # edge-on link
    settled.set(64, 20, "stone_cold_01")
    link_face(settled, 64, 22)  # the last face link, cut by the canvas bottom
    swung = rim(base.copy())
    # Swung out to the left: links stepping 1 px left every 2 rows.
    for i, (x, y) in enumerate(((65, 16), (64, 18), (63, 20), (62, 22))):
        swung.rect(x, y, x + 1, y + 1, "stone_cold_03" if i % 2 == 0 else "stone_cold_02")
        swung.set(x, y, "stone_cold_04" if i % 2 == 0 else "stone_cold_03")
        swung.set(x + 2, y + 1, "stone_cold_00")
    return [
        ("rig", [(rig, None)], "hold", "free"),
        ("rig_run", [(swung, 6), (settled, 6)], "once", "free"),
    ]


def chain_frames():
    return [("chain", [(K.chain_tile(), None)], "hold", "free")]


# ================================================================== DRUM ====
def drum_frames():
    armed = K.drum_armed()
    ta = armed.copy()
    for hy in (13, 26):
        ta.hline(3, 32, hy, "flame_03")
        ta.hline(3, 32, hy + 1, "flame_02")
    px(ta, [(25, 3), (24, 2), (26, 2)], "stone_10")
    tb = armed.copy()
    for hy in (13, 26):
        tb.hline(3, 32, hy, "flame_03")
        tb.hline(3, 32, hy + 1, "flame_02")
    # Bulge: lid 1 px up, body 1 px into each side margin (bottom chime rows unchanged).
    for y in range(6, 33):
        left = next(x for x in range(0, 36) if tb.get(x, y) is not None)
        right = max(x for x in range(0, 36) if tb.get(x, y) is not None)
        inner_l, inner_r = tb.get(left + 1, y), tb.get(right - 1, y)
        tb.set(left - 1, y, "iron_00")
        tb.set(left, y, inner_l)
        tb.set(right + 1, y, "iron_00")
        tb.set(right, y, inner_r)
    for x in range(0, 36):
        if tb.get(x, 4) is not None:
            tb.set(x, 3, tb.get(x, 4))
            tb.set(x, 4, tb.get(x, 5))
    px(tb, [(25, 2), (24, 1), (26, 1)], "stone_10")
    return [
        ("drum_armed", [(armed, None)], "hold", "grounded"),
        ("drum_telegraph", [(ta, 4), (tb, 4)], "loop", "grounded"),
    ]


def scorch_frames():
    """The key (with its last embers), then the ember fade: 3 x 10 ticks from the drum's
    detonation tick, holding the cold last frame (spec section 4, optional E)."""
    key = K.drum_scorch()
    dim = key.copy()
    px(dim, [(19, 5)], "flame_00")
    px(dim, [(20, 5), (14, 5)], "brick_04")
    px(dim, [(15, 5)], "mortar_00")
    cold = key.copy()
    px(cold, [(19, 5)], "brick_04")
    px(cold, [(20, 5), (14, 5), (15, 5)], "mortar_00")
    return [("scorch", [(key, 10), (dim, 10), (cold, 10)], "once", "grounded")]


# ================================================================= TELLS ====
def _h(a: int, b: int, c: int) -> int:
    """A tiny deterministic integer hash (no randomness)."""
    v = (a * 73856093) ^ (b * 19349663) ^ (c * 83492791)
    v = (v ^ (v >> 13)) * 1274126177 & 0xFFFFFFFF
    return v ^ (v >> 16)


def sputter_frames():
    frames = []
    shapes = [
        [(1, 8), (2, 8), (1, 9), (2, 9)],
        [(3, 7), (4, 7), (5, 7), (3, 8), (4, 8), (5, 8), (4, 9), (5, 9)],
        [(7, 5), (8, 5), (7, 6), (8, 6), (9, 6), (6, 7), (7, 7), (8, 7), (9, 7), (10, 7), (7, 8), (8, 8),
         (9, 8), (8, 9)],
    ]
    for shape in shapes:
        cv = Canvas(16, 16)
        for i, (x, y) in enumerate(shape):
            cv.set(x, y, "stone_10" if i % 3 else "gal_10")
        frames.append((cv, 4))
    return [("steam_sputter", frames, "loop", "free")]


def drop_isolated(cv: Canvas) -> None:
    """Remove pixels with no opaque 8-neighbour (the sources allow no isolated pixels)."""
    lone = [(x, y) for y in range(cv.h) for x in range(cv.w) if cv.p[y][x] is not None
            and all(cv.get(x + dx, y + dy) is None for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy)]
    for x, y in lone:
        cv.p[y][x] = None


# Steam plume (PR 256 review, follow-up 2): one continuous jet instead of strings of
# round puffs. The jet leaves the nozzle bore (local y 31..44, centre 37.5) and rises
# and widens to the right as overlapping billows on a continuous core; a ground fog
# rolls along the floor at actor height. Billows are spaced 16 px and travel 4 px per
# frame, so the 4-frame loop is seamless. Light from the upper left: each billow is a
# gal_10 shadow (offset down-right), a stone_10 body and a near-white flame_05 core
# (offset up-left; paler than the peach body), the fog is gal_10 with stone_10 tops,
# and gal_08 only edges the thin far fog.
PLUME_STEP = 4
PLUME_SPACING = 16


def plume_centre(x: float) -> float:
    return 37.5 - 0.14 * x


def plume_radius(x: float) -> float:
    return 6.0 + 0.40 * max(0.0, x)


def plume_frames():
    w, h = 80, 96
    frames = []
    for f in range(4):
        cv = Canvas(w, h)
        shift = PLUME_STEP * f
        # (cx, cy, r) billows: the main line on the centre, an upper and a lower
        # line half a spacing out of phase.
        main = []
        for i in range(-1, w // PLUME_SPACING + 2):
            cx = i * PLUME_SPACING + shift
            r = plume_radius(cx)
            main.append((cx, plume_centre(cx), r * 0.62))
            ox = cx + PLUME_SPACING / 2
            ro = plume_radius(ox)
            main.append((ox, plume_centre(ox) - ro * 0.42, ro * 0.48))
            main.append((ox, plume_centre(ox) + ro * 0.48, ro * 0.52))
            if cx > 24:  # spill: billows rolling down from the jet towards the fog
                main.append((cx + 4, plume_centre(cx) + r * 1.0, r * 0.40))
        fog = []
        for i in range(-1, w // PLUME_SPACING + 2):
            cx = i * PLUME_SPACING + shift + 6
            fog.append((cx, 93.0, 8.0))
            fog.append((cx + PLUME_SPACING / 2, 95.0, 6.0))
        # The continuous core from the bore: a cone 7 px tall at the nozzle.
        def core(colour: str, grow: float, dx: float, dy: float) -> None:
            for x in range(w):
                yc = plume_centre(x) + dy
                half = (3.5 + 0.10 * x) * grow
                for y in range(int(yc - half) - 1, int(yc + half) + 2):
                    if abs(y + 0.5 - yc) <= half and 0 <= x + int(dx) < w:
                        cv.set(x + int(dx), y, colour)

        for cx, cy, r in fog:  # fog: shadow then body then lit tops
            cv.ellipse(cx + 1, cy + 1, r, r * 0.8, "gal_08")
        for cx, cy, r in fog:
            cv.ellipse(cx, cy, r - 1, (r - 1) * 0.8, "gal_10")
        for cx, cy, r in fog:
            cv.ellipse(cx - 1, cy - 1.5, r * 0.45, r * 0.3, "stone_10")
        for cx, cy, r in main:
            cv.ellipse(cx + 1.5, cy + 1.5, r, r * 0.9, "gal_10")
        core("gal_10", 1.25, 1, 1)
        for cx, cy, r in main:
            cv.ellipse(cx, cy, r, r * 0.9, "stone_10")
        core("stone_10", 1.2, 0, 0)
        for cx, cy, r in main:
            cv.ellipse(cx - r * 0.2, cy - r * 0.25, r * 0.6, r * 0.5, "flame_05")
        core("flame_05", 0.6, 0, -1)
        # The lane ends at the canvas: round the plume's right and top edges into
        # travelling lobes so the canvas border never shows as a straight cut.
        def lobe(u: float) -> float:  # a half-circle bump over u in [0, 1)
            return (1.0 - (2.0 * u - 1.0) ** 2) ** 0.5

        for y in range(h):
            right = 72 + 7 * lobe(((y + 6 * f) % 24) / 24)
            for x in range(w):
                top = 5 - 5 * lobe(((x - 4 * f) % 16) / 16)
                if x + 0.5 > right or y + 0.5 < top:
                    cv.set(x, y, None)
        drop_isolated(cv)
        frames.append((cv, 4))
    return [("steam_plume", frames, "loop", "free")]


def shadow_frames():
    def inside(x: int, y: int) -> bool:
        dx = (x + 0.5 - 32) / 31.0
        dy = (y + 0.5 - 4) / 3.6
        return 0 <= x < 64 and 0 <= y < 8 and dx * dx + dy * dy <= 1.0

    def ellipse(phase: int | None) -> Canvas:
        """Solid for the landing; for the tell a solid 1 px rim (PR 256 review, follow-up 4:
        the all-dithered ellipse was faint) around a dithered inside that swaps phase."""
        cv = Canvas(64, 8)
        for y in range(8):
            for x in range(64):
                if inside(x, y):
                    rim = any(not inside(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
                    if phase is None or rim or (x + y + phase) % 2 == 0:
                        cv.set(x, y, "iron_00")
        return cv

    def ticks(cv: Canvas, phase: int) -> Canvas:
        for x0 in (0, 61):
            for y in range(2, 6):
                for x in range(x0, x0 + 3):
                    cv.set(x, y, "brass_05" if (x + y + phase) % 2 == 0 else "iron_00")
        return cv

    return [
        ("fall_shadow_tell", [(ticks(ellipse(0), 0), 4), (ticks(ellipse(1), 1), 4)], "loop", "free"),
        ("fall_shadow", [(ellipse(None), None)], "hold", "free"),
    ]


def ring_frames():
    def ring(phase: int) -> Canvas:
        cv = Canvas(96, 8)
        for y in range(8):
            for x in range(96):
                dx = (x + 0.5 - 48) / 47.5
                dy = (y + 0.5 - 4) / 3.9
                d = dx * dx + dy * dy
                if 0.62 <= d <= 1.0:
                    k = (x // 3 + phase) % 2
                    cv.set(x, y, "brass_05" if k == 0 else "flame_02")
        return cv

    return [("drum_blast_tell", [(ring(0), 4), (ring(1), 4)], "loop", "free")]


# ================================================================ OUTPUT ====
# grid name -> (tag specs, pivot)
def assets() -> dict[str, tuple[list[TagSpec], tuple[int, int]]]:
    return {
        "gw_boiler": (boiler_frames(), (96, K.BH - 1)),
        "gw_valve": (valve_frames(), (16, K.VH - 1)),
        "gw_brace": (brace_frames(), (16, K.RH - 1)),
        "gw_weight": (weight_frames(), (32, K.WH - 1)),
        "gw_remnant": (remnant_frames(), (32, K.MH - 1)),
        "gw_drum": (drum_frames(), (18, K.DH - 1)),
        "gw_scorch": (scorch_frames(), (24, K.SH - 1)),
        "gw_rig": (rig_frames(), (0, 0)),
        "gw_chain": (chain_frames(), (0, 0)),
        "gw_tell_sputter": (sputter_frames(), (0, 0)),
        "gw_tell_plume": (plume_frames(), (0, 0)),
        "gw_tell_shadow": (shadow_frames(), (0, 0)),
        "gw_tell_ring": (ring_frames(), (0, 0)),
    }


def to_grid(tags: list[TagSpec], pivot: tuple[int, int], pal: dict[str, str]) -> dict:
    flat: list[Frame] = []
    tag_rows = []
    for name, frames, _playback, _contact in tags:
        tag_rows.append({"name": name, "from": len(flat), "to": len(flat) + len(frames) - 1,
                         "direction": "forward"})
        flat.extend(frames)
    names: list[str] = []
    for cv, _ in flat:
        for row in cv.p:
            for v in row:
                if v is not None and v not in names:
                    names.append(v)
    names.sort(key=lambda n: list(pal).index(n))
    if len(names) > len(K.KEYS):
        raise SystemExit("too many colours")
    key_of = {n: K.KEYS[i] for i, n in enumerate(names)}
    w, h = flat[0][0].w, flat[0][0].h
    layer = "body"
    frames_out = []
    for cv, ticks in flat:
        assert (cv.w, cv.h) == (w, h)
        rows = ["".join("." if v is None else key_of[v] for v in row) for row in cv.p]
        frames_out.append({"duration_ms": ms_of(ticks), "cels": {layer: rows}})
    return {
        "format": "pixel-grid-v1",
        "canvas": [w, h],
        "pivot": list(pivot),
        "palette": [{"key": key_of[n], "name": n, "rgba": pal[n]} for n in names],
        "layers": [layer],
        "frames": frames_out,
        "tags": tag_rows,
    }


# ----------------------------------------------------------- configs, manifests ----
MANIFESTS = ROOT / "assets/manifests"
GRID_DIR_REL = "assets/source/props/floor01"
SPEC = "docs/02-art/goblin-workshop-prop-kit.md"

# Frozen key frames per grid (frame index in the full grid; spec section 4).
FROZEN = {"gw_boiler": [0, 6], "gw_valve": [0, 6], "gw_brace": [0, 1, 3], "gw_weight": [0],
          "gw_remnant": [0], "gw_drum": [0], "gw_scorch": [0], "gw_rig": [0], "gw_chain": [0]}

# Evidence scene: representative frames on the lit Floor 1 lane (x = pivot column on the walking line).
SCENES = {
    "gw_boiler": ([(0, 560, 0), (6, 800, 0)], True),
    "gw_valve": ([(0, 540, 0), (2, 610, 0), (4, 680, 0), (6, 750, 0)], True),
    "gw_brace": ([(0, 560, 0), (1, 640, 0), (3, 720, 0)], True),
    "gw_weight": ([(0, 600, 4), (1, 700, 4)], False),
    "gw_remnant": ([(0, 640, 0)], True),
    "gw_drum": ([(0, 600, 0), (2, 680, 0)], True),
    "gw_scorch": ([(0, 640, 0)], True),
    "gw_rig": ([(0, 520, 110), (1, 640, 110), (2, 760, 110)], False),
    "gw_chain": ([(0, 640, 20)], False),
    "gw_tell_sputter": ([(0, 560, 20), (1, 600, 20), (2, 640, 20)], False),
    "gw_tell_plume": ([(0, 520, 96), (2, 660, 96)], False),
    "gw_tell_shadow": ([(0, 520, 0), (1, 620, 0), (2, 720, 0)], False),
    "gw_tell_ring": ([(0, 520, 0), (1, 660, 0)], False),
}
STRIPS = {"gw_boiler": ["boiler_idle", "boiler_strain"], "gw_valve": ["valve_warning", "valve_venting"],
          "gw_brace": ["brace_weakened", "brace_stump"], "gw_weight": ["weight_hang", "weight_fall"],
          "gw_remnant": ["remnant"], "gw_drum": ["drum_armed", "drum_telegraph"], "gw_scorch": ["scorch"],
          "gw_rig": ["rig", "rig_run"], "gw_chain": ["chain"], "gw_tell_sputter": ["steam_sputter"],
          "gw_tell_plume": ["steam_plume"], "gw_tell_shadow": ["fall_shadow_tell", "fall_shadow"],
          "gw_tell_ring": ["drum_blast_tell"]}
TITLES = {
    "gw_boiler": "riveted copper boiler (idle loop, strain, damaged loop)",
    "gw_valve": "steam valve (intact, warning, venting, spent)",
    "gw_brace": "timber brace (supported, weakened creak, stump)",
    "gw_weight": "falling weight (hanging, falling)", "gw_remnant": "landed weight remnant",
    "gw_drum": "fuel drum (armed, telegraph)", "gw_scorch": "drum scorch mark",
    "gw_rig": "ceiling beam rig with two sheaves (intact, run)", "gw_chain": "vertical chain tile",
    "gw_tell_sputter": "steam sputter tell at the nozzle", "gw_tell_plume": "steam plume over the lane",
    "gw_tell_shadow": "weight fall shadow (tell, landing)", "gw_tell_ring": "drum blast floor ring tell",
}


def asset_stem(name: str) -> str:
    return "f01-" + name.replace("_", "-")


def tick_tags(tags: list[TagSpec], pivot_y: int | None):
    out, i = {}, 0
    for tag, frames, playback, contact in tags:
        rows = [[i + k, t] for k, (_cv, t) in enumerate(frames)]
        out[tag] = {"playback": playback, "contact": contact, "frames": rows}
        i += len(frames)
    return out


def config_of(name: str, tags: list[TagSpec], grid: dict) -> dict:
    import pixel_grid_to_ase as pg
    rep = pg.lint_grid(grid)
    w, h = grid["canvas"]
    grounded = any(c == "grounded" for _t, _f, _p, c in tags)
    rules: dict = {"max_colors": 63, "max_isolated_per_frame": 0,
                   "max_orphans_per_frame": max(rep["orphan_count_per_frame"]),
                   "unique_frames": True}
    if name in FROZEN:
        rules["frozen_keys"] = {"fixture": f"tests/fixtures/area-gw-{name[3:]}_keys.json",
                                "frames": FROZEN[name], "layers": ["body"]}
    actors, shadow = SCENES[name]
    cfg = {
        "format": "pixelart-asset-v1", "asset_id": f"area-gw-{name[3:].replace('_', '-')}",
        "title": f"AREA-GOBLINWORKSHOP-01 {TITLES[name]}",
        "grid": f"{GRID_DIR_REL}/{name}.grid.json", "manifest": f"assets/manifests/{asset_stem(name)}.json",
        "frame_size": [w, h], "pivot": list(grid["pivot"]),
        "contact_row": grid["pivot"][1] if grounded else None, "tick_hz": 60, "hold_ms": 100,
        "facing": "left", "tags": tick_tags(tags, grid["pivot"][1]), "rules": rules,
        "scene": {"shadow": shadow, "actors": [{"frame": f, "x": x, "lift": lift} for f, x, lift in actors]},
        "review": {"strip_name": "03_state_strip.png", "strip_tags": STRIPS[name]},
    }
    return cfg


def manifest_of(name: str, tags: list[TagSpec], grid: dict) -> dict:
    from scripts.pixelart.validators import frame_digest
    stem = asset_stem(name)
    w, h = grid["canvas"]
    n = len(grid["frames"])
    flat = [(tag, k, t) for tag, frames, _p, _c in tags for k, (_cv, t) in enumerate(frames)]
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
    return {
        "schema_version": 1, "id": f"{stem.replace('-', '_')}_candidate_v0", "type": "prop_animation",
        "source_path": f"{GRID_DIR_REL}/{name}.ase", "grid_source": f"{GRID_DIR_REL}/{name}.grid.json",
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "floor01-catacomb-v2", "frame_size": [w, h], "pivot": list(grid["pivot"]),
        "animation_tags": [t for t, _f, _p, _c in tags], "required_alpha": True,
        "owner": "AI-authored Floor 1 Goblin Workshop prop kit (fan-concept layer; runtime ids IP-neutral gw_*)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": f"AREA-GOBLINWORKSHOP-01 {TITLES[name]}, drawn from explicit primitives in the named Floor 01 kit "
                      f"palette by scripts/author_gwkit_keys.py and scripts/author_gwkit_frames.py",
            "generator": "designed by Claude Opus (keys, spec, silhouette gate), executed by Claude Sonnet; "
                         "independent top-tier review passed (Claude Opus, PR 256, no design defect); review follow-ups "
                         "(weight silhouette, rig run, plume, shadow tell, scorch ember fade) designed and executed by "
                         "Claude Opus. No concept pixels copied, no image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": [SPEC, "assets/source/terrain/floor01/floor01_catacomb_kit.json"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; "
                   "no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "prop_kit": {
            "status": f"{n} frames drawn; independent top-tier review passed (PR 256), owner review pending; "
                      "drawn by presentation-only wiring in src/env_prop_art.rs",
            "spec": SPEC, "tick_hz": 60, "hold_preview_ms": HOLD_MS,
            "playback": {tag: p for tag, _f, p, _c in tags},
            "frames": frames,
        },
    }


def build(out: Path, meta: bool = False) -> list[Path]:
    pal = K.kit_palette()
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
