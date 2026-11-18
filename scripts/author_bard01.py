#!/usr/bin/env python3
"""BARD-01 authoring record: Goblin Bomb Bard palette, reusable prop stamps and the idle hero key.

Usage:

    python3 scripts/author_bard01.py keys [--check] [--preview PNG]

``keys`` (re)writes ``assets/source/characters/goblin/bard/bard_01.grid.json`` from literal pixel
segments (see ``docs/02-art/bard-01-spec.md`` sections 3, 6 and 7). Design stage: the grid holds only
the hero key, frame 0 (``idle``), which is frozen in ``tests/fixtures/bard-01_keys.json``. The
executor appends frames 1 to 15 in this script (same segment style, same stamps), extends the grid
tags and writes the manifest; frame 0 must stay byte-identical.

Layers (disjoint): ``body`` (goblin, headwrap, goggles, scarf, pack, belt, boots) and ``prop``
(horn, bombs, fuse spark, music notes). Drawing order: body, then prop (prop pixels clear the body
under them), then the ``front`` body pass (hands that grip the horn clear the prop under them).

The head construction (ear, brow, eyes, nose, teeth) is the GOB-01 variant A idle head
(``goblin_a_h36.grid.json`` frame 0) moved up 3 rows, under a new headwrap and goggles, with the jaw
dropped into an open grin. No concept pixels are traced or sampled; colours are chosen by eye.
``--check`` compares instead of writing (exit 1 on a difference). Presentation-only art tooling;
nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BARD_DIR = ROOT / "assets/source/characters/goblin/bard"
GRID = BARD_DIR / "bard_01.grid.json"
CANVAS = (64, 56)
PIVOT = (28, 55)
LAYERS = ["body", "prop"]

# bard-floor01-v0 (19 of 20). Skin, leather, eyes, teeth, mouth and the red pair are goblin-floor01-v0
# keys with the same hex (family read); brass, iron and the spark core are new. The bombs are a cool
# violet-grey iron so they read as metal, not as the plum outline; the brass is warm but darker and
# browner than the brick highlight; the spark core is the only near-white warm key and appears only
# in the tell and the throw.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "leather_dark", "#2c1e28ff", False),
    ("t", "leather_mid", "#4c3436ff", False),
    ("u", "leather_light", "#73524aff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
    ("R", "scarf_red", "#b8322eff", False),
    ("B", "scarf_red_dark", "#6e1a22ff", False),
    ("n", "brass_dark", "#6a4a1eff", False),
    ("b", "brass_mid", "#b8862eff", False),
    ("h", "brass_light", "#f0cf6aff", True),
    ("q", "iron_dark", "#3a3245ff", False),
    ("Q", "iron_light", "#766c86ff", False),
    ("Y", "spark_core", "#fff4b8ff", True),
]
KEYS = [p[0] for p in PALETTE]
TAGS = [("idle", 0, 2), ("walk", 3, 6), ("attack_tell", 7, 8), ("attack_swing", 9, 10),
        ("attack_recover", 11, 11), ("hurt", 12, 12), ("down", 13, 15)]
TICKS = [14, 10, 14, 5, 5, 5, 5, 10, 20, 3, 4, 14, 12, 4, 8, None]   # 60 Hz; None = state-driven hold
KEY_FRAMES = (0, 3, 7, 8, 10, 12, 15)
HOLD_MS = 100                    # preview duration of the hold frame (same as ``hold_ms`` in the config)

# --------------------------------------------------------------------------- reusable stamps
# Iron bomb, 7 x 7, brass fuse cap in the top outline (cap at stamp x 3, row 0). Highlight upper left.
BOMB7 = [
    "..KbK..",
    ".KQqqK.",
    "KQqqqqK",
    "KqqqqqK",
    "KqqqqqK",
    ".KqqqK.",
    "..KKK..",
]
# Small belt bomb, 5 x 5 (cap at stamp x 2, row 0).
BOMB5 = [
    ".KbK.",
    "KQQqK",
    "KqqqK",
    "KqqqK",
    ".KKK.",
]
# Lit fuse spark (tell and throw only), 3 x 3 around the fuse tip: Y core, e halo, R ember.
SPARK = [
    "e.R",
    ".Ye",
    "R.e",
]
# Music note (horn toot, frame 7 only), 3 x 4, tooth white so it reads as a sound glyph, not as fire.
NOTE = [
    ".ww",
    ".w.",
    "ww.",
    "ww.",
]
# Horn bell, idle orientation (axis rising about 15 degrees forward), rows 29-41, x 44-53 when drawn at
# (44, 29). Rim column 'h', outer cone 'b' above the axis and 'n' below it.
BELL_IDLE = [
    ".......KK.",
    "......KhhK",
    ".....KhbhK",
    "....KhbbhK",
    "...KhbbbhK",
    ".KKhbbbnhK",
    "KhbbbbnnhK",
    "bbbbnnnnhK",
    "nnnnnnnnhK",
    "KKnnnnnnhK",
    "..KKnnnnhK",
    "....KKnnhK",
    "......KKKK",
]


class Canvas:
    def __init__(self) -> None:
        w, h = CANVAS
        self.cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}

    def put(self, layer: str, x: int, y: int, ch: str) -> None:
        """Set one pixel on ``layer`` and clear the same pixel on every other layer (layers stay disjoint)."""
        if ch == ".":
            return
        if ch not in KEYS:
            raise ValueError(f"unknown key {ch!r}")
        w, h = CANVAS
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel ({x}, {y}) outside the {w} x {h} frame")
        for name, rows in self.cels.items():
            rows[y][x] = ch if name == layer else "."

    def segs(self, layer: str, rows: dict[int, list[tuple[int, str]]]) -> None:
        for y, parts in rows.items():
            for x0, s in parts:
                for i, ch in enumerate(s):
                    self.put(layer, x0 + i, y, ch)

    def stamp(self, layer: str, x0: int, y0: int, rows: list[str]) -> None:
        for dy, s in enumerate(rows):
            for dx, ch in enumerate(s):
                self.put(layer, x0 + dx, y0 + dy, ch)

    def rows(self) -> dict[str, list[str]]:
        return {name: ["".join(r) for r in rows] for name, rows in self.cels.items()}


def horn_pipe(c: Canvas, x0: int, x1: int, top_at_x0: int, rise_every: int, band_x: int | None = None) -> None:
    """Two-row brass pipe ('b' over 'n') from x0 to x1, rising one row every ``rise_every`` px forward."""
    for x in range(x0, x1 + 1):
        top = top_at_x0 - (x - x0) // rise_every
        c.put("prop", x, top, "K" if x == band_x else "b")
        c.put("prop", x, top + 1, "K" if x == band_x else "n")


# --------------------------------------------------------------------------- frame 0: idle hero key
HEAD_IDLE = {  # headwrap, goggles, GOB-01 head moved up 3 rows, open grin
    16: [(30, "KKKKKK")],
    17: [(19, "K"), (29, "KBRRRRRK")],
    18: [(18, "KlKKK"), (28, "KBRRRRRRRK")],
    19: [(19, "KllaKKK"), (27, "KBRRRR"), (33, "bbKKbbK")],
    20: [(20, "KlsaaallBRRR"), (32, "bQwbbQwbK")],
    21: [(21, "KKssaammmmmm"), (33, "bbKKbbKK")],
    22: [(22, "KKsasmmmmmmmmaaKKmmK")],
    23: [(24, "KKsmmmmmmseeKaeKlmK")],
    24: [(25, "KlmmmmmmmsaammmlmmK")],
    25: [(24, "KKlmmmmmssKKKKmmsmmsK")],
    26: [(24, "KKKlmmmmsKwwowKKmsssK")],
    27: [(27, "KlmmsKooooowKKKKK")],
    28: [(27, "KssssKwoowKK")],
}
BODY_IDLE = {  # scarf, pack, slender upright torso, near arm, belt, shorts, legs, boots
    29: [(16, "KKKKKKKKKK"), (28, "KBBRKKKKKK")],
    30: [(16, "KuuuuuuutK"), (27, "KBBRRRRRRRK")],
    31: [(16, "KuuuuuuttK"), (26, "KtBRRRRRRBBK")],
    32: [(16, "KrrrbbrrrK"), (26, "KtuuuuKtRRBK")],
    33: [(16, "KutttbttrK"), (26, "KtKlmsKtRRBK")],
    34: [(16, "KuttttttrK"), (26, "KrKlmsKttRBK")],
    35: [(16, "KutttttrrK"), (26, "KrKlmsKtttBK")],
    36: [(16, "KutttttrrK"), (26, "KrKlmsKttttK")],
    37: [(16, "KrtttttrrK"), (26, "KrKmmsKttttK")],
    38: [(16, "KrrtttrrrK"), (26, "KrKlmmsKtttK")],
    39: [(16, "KrrrrrrrrK"), (26, "KrrKlmmsKttK")],
    40: [(16, "KKrrrrrrKK"), (26, "KrrrKlmmsKtK")],
    41: [(17, "KKKKKKKK"), (26, "KrrrrrrrrrK")],
    42: [(26, "KrrrrrrrrrK")],
    43: [(25, "KttrtttrtttK")],
    44: [(25, "KtrrtttrtttK")],
    45: [(25, "KrtKrrKtrKtK")],
    46: [(24, "KmaK"), (31, "KlmsK")],
    47: [(24, "KmaK"), (32, "KlmsK")],
    48: [(23, "KmaK"), (33, "KlmsK")],
    49: [(23, "KmaK"), (33, "KlmK")],
    50: [(22, "KmaK"), (33, "KlmK")],
    51: [(21, "KtutK"), (32, "KuutK")],
    52: [(21, "KtutK"), (32, "KutuK")],
    53: [(20, "KttuttK"), (31, "KuuttttK")],
    54: [(19, "KttttrrK"), (31, "KtttttrrK")],
    55: [(19, "KKKKKKKK"), (31, "KKKKKKKKK")],
}
FRONT_IDLE = {  # hands gripping the horn: near fist (skin ramp), far hand (shade only)
    36: [(40, "KK")],
    37: [(37, "KsssaK")],
    38: [(34, "KKK"), (39, "KsaK")],
    39: [(33, "KllsK"), (40, "KK")],
    40: [(33, "KmmsK")],
    41: [(33, "KssK")],
    42: [(34, "KK")],
}


def frame_idle0() -> dict[str, list[str]]:
    c = Canvas()
    c.segs("body", HEAD_IDLE)
    c.segs("body", BODY_IDLE)
    # pack bomb on top of the pack with an unlit fuse curling up and back; belt bomb at the back hip
    c.stamp("prop", 15, 22, BOMB7)
    for x, y, ch in ((18, 21, "t"), (17, 20, "t"), (16, 19, "t")):
        c.put("prop", x, y, ch)
    c.stamp("prop", 19, 41, BOMB5)
    # horn: mouthpiece cap, pipe across the body rising forward, one dark band, bell
    c.put("prop", 28, 41, "h")
    c.put("prop", 28, 42, "n")
    horn_pipe(c, 29, 43, 41, 3)
    c.stamp("prop", 44, 29, BELL_IDLE)
    c.segs("body", FRONT_IDLE)
    return c.rows()


# --------------------------------------------------------------------------- executor: pixel-map helpers
# Frames 1 to 15 are composed from the frozen key 0 segment tables above (moved, edited or replaced
# part by part) plus a few literal pieces. Pixel maps are {(x, y): key}. Nothing is sampled from the concept.
def pm_segs(table):
    return {(x0 + i, y): ch for y, parts in table.items() for x0, s in parts for i, ch in enumerate(s) if ch != "."}


def pm_stamp(x0, y0, rows):
    return {(x0 + dx, y0 + dy): ch for dy, s in enumerate(rows) for dx, ch in enumerate(s) if ch != "."}


def pm_flip(rows):
    return [r[::-1] for r in rows]


def put_pm(c, layer, p, dx=0, dy=0):
    for (x, y), ch in p.items():
        c.put(layer, x + dx, y + dy, ch)


def occupied(c, x, y):
    return any(rows[y][x] != "." for rows in c.cels.values())


def line_mask(p0, p1, thick):
    """Pixels whose centre lies within ``thick / 2`` of the segment p0 to p1 (continuous coordinates)."""
    (x0, y0), (x1, y1) = p0, p1
    dx, dy = x1 - x0, y1 - y0
    ln = dx * dx + dy * dy or 1.0
    out = set()
    for y in range(int(min(y0, y1) - thick) - 1, int(max(y0, y1) + thick) + 2):
        for x in range(int(min(x0, x1) - thick) - 1, int(max(x0, x1) + thick) + 2):
            t = max(0.0, min(1.0, ((x + .5 - x0) * dx + (y + .5 - y0) * dy) / ln))
            if (x + .5 - x0 - t * dx) ** 2 + (y + .5 - y0 - t * dy) ** 2 <= (thick / 2) ** 2:
                out.add((x, y))
    return out


def disc(cx, cy, rx, ry):
    return {(x, y) for y in range(int(cy - ry) - 1, int(cy + ry) + 2) for x in range(int(cx - rx) - 1, int(cx + rx) + 2)
            if ((x + .5 - cx) / rx) ** 2 + ((y + .5 - cy) / ry) ** 2 <= 1.0}


def rect(x0, y0, x1, y1):
    return {(x, y) for y in range(y0, y1 + 1) for x in range(x0, x1 + 1)}


def edge_shade(light, mid, dark):
    def f(x, y, m):
        up, dn = (x, y - 1) not in m, (x, y + 1) not in m
        return mid if up == dn else (light if up else dark)
    return f


SKIN_NEAR = edge_shade("l", "m", "s")
SKIN_FAR = edge_shade("s", "a", "a")
VEST = edge_shade("u", "t", "r")


def paint(c, layer, mask, fn, outline=True):
    w, h = CANVAS
    for x, y in mask:
        c.put(layer, x, y, fn(x, y, mask) if callable(fn) else fn)
    if outline:
        for x, y in mask:
            for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if (nx, ny) not in mask and 0 <= nx < w and 0 <= ny < h and not occupied(c, nx, ny):
                    c.put(layer, nx, ny, "K")


def pipe_line(c, x0, y0, x1, y1):
    """Two-row brass pipe ('b' over 'n') between two points."""
    for x in range(x0, x1 + 1):
        t = (x - x0) / max(1, x1 - x0)
        y = round(y0 + (y1 - y0) * t)
        c.put("prop", x, y, "b")
        c.put("prop", x, y + 1, "n")


def bell(c, n, r, w_neck, w_rim):
    """Hand-rolled rotated horn bell: neck centre ``n`` to rim centre ``r``; rim column 'h', lit half 'b' with an
    'h' edge, shaded half 'n', 1 px 'K' outline (same colour logic as BELL_IDLE)."""
    ux, uy = r[0] - n[0], r[1] - n[1]
    ln = (ux * ux + uy * uy) ** .5
    ux, uy = ux / ln, uy / ln
    vx, vy = -uy, ux
    if (vy, -vx) > (-vy, vx):                      # lit side normal: higher on screen, else toward +x
        vx, vy = -vx, -vy
    mask, col = set(), {}
    for y in range(int(min(n[1], r[1]) - w_rim) - 2, int(max(n[1], r[1]) + w_rim) + 3):
        for x in range(int(min(n[0], r[0]) - w_rim) - 2, int(max(n[0], r[0]) + w_rim) + 3):
            rx, ry = x + .5 - n[0], y + .5 - n[1]
            t, s = (rx * ux + ry * uy) / ln, rx * -vx + ry * -vy   # s > 0 on the lit side
            hw = w_neck + (w_rim - w_neck) * max(t, 0) ** 1.6
            if 0 <= t <= 1 and abs(s) <= hw:
                mask.add((x, y))
                col[(x, y)] = "h" if (t > 1 - 1.3 / ln or s > hw - 0.9) else ("b" if s > -hw * .15 else "n")
    paint(c, "prop", mask, lambda x, y, m: col[(x, y)])


# --------------------------------------------------------------------------- key 0 parts (for reuse)
HEAD0 = pm_segs(HEAD_IDLE)
_BODY0 = pm_segs(BODY_IDLE)
UPPER0 = {k: v for k, v in _BODY0.items() if k[1] <= 45}
UPPER_NA = {k: ("t" if v in "lms" else v) for k, v in UPPER0.items()}     # near arm column filled with vest
_FRONT0 = pm_segs(FRONT_IDLE)
FAR_HAND0 = {k: v for k, v in _FRONT0.items() if k[1] <= 37 or (k[1] == 38 and k[0] >= 39) or (k[1] == 39 and k[0] >= 40)}
FIST0 = {k: v for k, v in _FRONT0.items() if k not in FAR_HAND0}
LEG_FAR0 = {k: v for k, v in _BODY0.items() if k[1] >= 46 and k[0] < 29}
LEG_NEAR0 = {k: v for k, v in _BODY0.items() if k[1] >= 46 and k[0] >= 29}
PACKBOMB0 = {**pm_stamp(15, 22, BOMB7), (18, 21): "t", (17, 20): "t", (16, 19): "t"}
BELT0 = pm_stamp(19, 41, BOMB5)


def _horn0():
    p = {(28, 41): "h", (28, 42): "n"}
    for x in range(29, 44):
        top = 41 - (x - 29) // 3
        p[(x, top)], p[(x, top + 1)] = "b", "n"
    p.update(pm_stamp(44, 29, BELL_IDLE))
    return p


HORN0 = _horn0()

BOOT_FAR = [(2, "KtutK"), (2, "KtutK"), (1, "KttuttK"), (0, "KttttrrK"), (0, "KKKKKKKK")]
BOOT_NEAR = [(1, "KuutK"), (1, "KutuK"), (0, "KuuttttK"), (0, "KtttttrrK"), (0, "KKKKKKKKK")]


def boot(c, x0, y0, spec):
    for i, (xo, s) in enumerate(spec):
        c.segs("body", {y0 + i: [(x0 + xo, s)]})


def leg(c, rows, far):
    """Thigh or shin rows ``(y, x, inner width)``: 'K' + skin + 'K' (far leg ma/maa, near leg lm/lms)."""
    for y, x, w in rows:
        inner = ("m" + "a" * (w - 1)) if far else ("lm" + "s" * (w - 2))
        c.segs("body", {y: [(x, "K" + inner + "K")]})


def legs_idle(c, heel_lift=False, rows_from=46):
    far = {k: v for k, v in LEG_FAR0.items() if k[1] >= rows_from}
    near = {k: v for k, v in LEG_NEAR0.items() if k[1] >= rows_from}
    if rows_from < 46:
        for y in range(rows_from, 46):
            far.update({(24 + i, y): ch for i, ch in enumerate("KmaK")})
            near.update({(31 + i, y): ch for i, ch in enumerate("KlmsK")})
    if heel_lift:
        for x in range(31, 35):
            near.pop((x, 55), None)
            near[(x, 54)] = "K"
    put_pm(c, "body", far)
    put_pm(c, "body", near)


def std(c, head=(0, 0), upper=(0, 0), fist=(0, 0), hand=(0, 0), horn=(0, 0), pbomb=(0, 0), bbomb=(0, 0),
        head_pm=None, upper_pm=None, pack_bomb=True, fists=True):
    """Compose key 0 parts, each moved by its own (dx, dy); legs are drawn by the caller first."""
    put_pm(c, "body", upper_pm or UPPER0, *upper)
    put_pm(c, "body", head_pm or HEAD0, *head)
    if pack_bomb:
        put_pm(c, "prop", PACKBOMB0, *pbomb)
    put_pm(c, "prop", BELT0, *bbomb)
    put_pm(c, "prop", HORN0, *horn)
    if fists:
        put_pm(c, "body", FIST0, *fist)
        put_pm(c, "body", FAR_HAND0, *hand)


ACCENT = {p[0] for p in PALETTE if p[3]}
KEEP = ACCENT | {"b"}                 # the brass fuse caps on the bombs stay (they are single brass pixels by design)
_NB = [(dx, dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if dx or dy]


def despeckle(cels: dict[str, list[str]]) -> dict[str, list[str]]:
    """Production clean-up for frames 1 to 15 (never frame 0): a pixel with no opaque neighbour is dropped; a non-accent
    pixel with no same-key neighbour takes the most common neighbouring key (ties: palette order). Layers stay disjoint
    because each pixel is changed in its own layer. Two passes, deterministic."""
    w, h = CANVAS
    rows = {n: [list(r) for r in rs] for n, rs in cels.items()}
    for _ in range(2):
        flat = [["."] * w for _ in range(h)]
        for rs in rows.values():
            for y in range(h):
                for x in range(w):
                    if rs[y][x] != ".":
                        flat[y][x] = rs[y][x]
        for y in range(h):
            for x in range(w):
                ch = flat[y][x]
                if ch == ".":
                    continue
                around = [flat[y + dy][x + dx] for dx, dy in _NB if 0 <= x + dx < w and 0 <= y + dy < h]
                around = [a for a in around if a != "."]
                new = None
                if not around:
                    new = "."
                elif ch not in KEEP and ch not in around:
                    new = max(sorted(set(around), key=KEYS.index), key=around.count)
                if new is not None:
                    for rs in rows.values():
                        if rs[y][x] != ".":
                            rs[y][x] = new
    return {n: ["".join(r) for r in rs] for n, rs in rows.items()}


# --------------------------------------------------------------------------- idle (frames 1, 2)
def frame_idle1() -> dict[str, list[str]]:
    c = Canvas()
    legs_idle(c, rows_from=47)
    pb = dict(PACKBOMB0)
    pb[(15, 19)] = "t"                                   # fuse tip swings 1 px back (key 0 pixel kept)
    put_pm(c, "body", UPPER0, 0, 1)
    put_pm(c, "body", HEAD0, 0, 1)
    put_pm(c, "prop", pb)                                # pack bomb lags: stays on its key 0 rows
    put_pm(c, "prop", BELT0, 0, 1)
    put_pm(c, "prop", HORN0, 0, 1)
    put_pm(c, "body", FIST0, 0, 1)
    put_pm(c, "body", FAR_HAND0, 0, 0)                   # fingers drum: 1 px up against the body shift
    return c.rows()


def frame_idle2() -> dict[str, list[str]]:
    c = Canvas()
    legs_idle(c)
    head = dict(HEAD0)
    head.pop((19, 17))                                   # ear tip 1 px lower
    std(c, head=(1, 0), head_pm=head)
    put_pm(c, "prop", {(17, 19): "t"})                   # fuse tip 1 px forward
    return c.rows()


# --------------------------------------------------------------------------- walk (key 3, in-betweens 4 to 6)
def walk_upper(c):
    std(c, head=(1, 1), upper=(1, 1), fist=(1, 1), hand=(1, 1), horn=(1, 1), pbomb=(1, 0), bbomb=(0, 1))


NEAR_FWD = [(47, 33, 3), (48, 34, 3), (49, 35, 3), (50, 36, 2)]
NEAR_FWD_BOOT = [(51, 36, "KuutK"), (52, 36, "KutuK"), (53, 36, "KuuttttK"), (54, 36, "KtttKKKKK"), (55, 36, "KKKKK")]
FAR_BACK = [(47, 25, 2), (48, 24, 2), (49, 23, 2), (50, 21, 3), (51, 17, 3)]
FAR_BACK_BOOT = [(52, 14, "KtutK"), (53, 14, "KttutttK"), (54, 14, "KKKttrrK"), (55, 18, "KKKK")]


def frame_walk3() -> dict[str, list[str]]:
    c = Canvas()
    leg(c, FAR_BACK, True)
    c.segs("body", {y: [(x, s)] for y, x, s in FAR_BACK_BOOT})
    leg(c, NEAR_FWD, False)
    c.segs("body", {y: [(x, s)] for y, x, s in NEAR_FWD_BOOT})
    walk_upper(c)
    return c.rows()


def frame_walk4() -> dict[str, list[str]]:
    c = Canvas()
    leg(c, [(46, 24, 2), (47, 23, 2), (48, 22, 2)], True)
    boot(c, 18, 49, BOOT_FAR[1:])                        # raised 4-row boot, x 18 to 25, rows 49 to 52
    leg(c, [(y, 28, 3) for y in range(46, 51)], False)
    boot(c, 27, 51, BOOT_NEAR)                           # boot x 27 to 35 on row 55
    std(c, pbomb=(0, 1))
    return c.rows()


def frame_walk5() -> dict[str, list[str]]:
    c = Canvas()
    leg(c, [(47, 26, 3), (48, 25, 3), (49, 24, 3), (50, 22, 3), (51, 18, 3)], False)
    c.segs("body", {y: [(x + 1, s)] for y, x, s in FAR_BACK_BOOT})
    leg(c, [(47, 32, 2), (48, 33, 2), (49, 34, 2), (50, 34, 2)], True)
    c.segs("body", {y: [(x - 2, s)] for y, x, s in NEAR_FWD_BOOT})   # far boot x 34 to 42
    walk_upper(c)
    return c.rows()


def frame_walk6() -> dict[str, list[str]]:
    c = Canvas()
    leg(c, [(46, 24, 3), (47, 23, 3), (48, 22, 3)], False)
    boot(c, 18, 49, BOOT_FAR[1:])
    leg(c, [(y, 28, 2) for y in range(46, 51)], True)
    boot(c, 27, 51, BOOT_NEAR)
    std(c, pbomb=(0, 1))
    return c.rows()


# --------------------------------------------------------------------------- tell (keys 7, 8)
def frame_toot7() -> dict[str, list[str]]:
    c = Canvas()
    head = dict(HEAD0)
    for x in (34, 35, 38):
        head[(x, 23)] = "K"                              # eyes squeezed: 2 px K lines on the eye row
    for y, s in ((26, "KmllmK"), (27, "KmllmK"), (28, "mmllKK")):
        for i, ch in enumerate(s):
            head[(33 + i, y)] = ch                       # mouth closed, 2 x 3 'l' cheek bulge
    legs_idle(c, heel_lift=True, rows_from=45)
    put_pm(c, "body", UPPER0, 0, -1)
    put_pm(c, "body", head, 0, -1)
    put_pm(c, "prop", PACKBOMB0, 0, -1)
    put_pm(c, "prop", BELT0, 0, -1)
    c.put("prop", 38, 27, "h")
    c.put("prop", 38, 28, "n")
    pipe_line(c, 39, 27, 44, 25)
    bell(c, (44.5, 25.5), (51.0, 22.0), 1.5, 5.0)
    c.stamp("prop", 55, 13, NOTE)
    put_pm(c, "body", FIST0, 3, -9)                      # near fist on the pipe, x 36 to 40, rows 29 to 33
    c.segs("body", {25: [(43, "KK")], 26: [(42, "KsaK")], 27: [(42, "KsaK")], 28: [(43, "KK")]})   # far hand
    return c.rows()


def frame_lit8() -> dict[str, list[str]]:
    c = Canvas()
    head = dict(HEAD0)
    head[(37, 23)] = "e"                                 # eyes wide: 2 px each
    leg(c, [(46, 24, 2), (47, 23, 2), (48, 23, 2), (49, 22, 2), (50, 22, 2)], True)
    boot(c, 19, 51, BOOT_FAR)
    leg(c, [(46, 31, 3), (47, 32, 3), (48, 34, 3), (49, 35, 2), (50, 35, 2)], False)
    boot(c, 33, 51, BOOT_NEAR)                           # braced front boot x 33 to 41
    put_pm(c, "body", UPPER_NA, -3, 0)
    put_pm(c, "body", head, -3, 1)
    put_pm(c, "prop", BELT0, -3, 0)
    put_pm(c, "prop", HORN0, -3, 2)
    c.stamp("prop", 4, 27, BOMB7)
    c.stamp("prop", 6, 24, SPARK)                        # Y at (7, 25)
    paint(c, "body", line_mask((27, 31.5), (21, 33.5), 2.4) | line_mask((21, 33.5), (15, 30.5), 2.4), SKIN_NEAR)   # elbow bend (review fix)
    c.segs("body", {29: [(11, "KKKK")], 30: [(11, "KlmK")], 31: [(11, "KmsK")], 32: [(11, "KKKK")]})   # hand on the bomb
    put_pm(c, "body", FAR_HAND0, -3, 2)                  # far hand keeps the horn
    return c.rows()


# --------------------------------------------------------------------------- swing (9, key 10) and recover (11)
def horn_back(c, y0):
    """Horn swung back: mirrored BELL_IDLE (rim 'h' at x 8, rows y0 + 1 to y0 + 11), pipe to the far hand."""
    c.stamp("prop", 7, y0, pm_flip(BELL_IDLE))
    for x in range(17, 22):
        c.put("prop", x, y0 + 7, "b")
        c.put("prop", x, y0 + 8, "n")
    c.segs("body", {y0 + 5: [(22, "KKKK")], y0 + 6: [(22, "KsaK")], y0 + 7: [(22, "KsaK")], y0 + 8: [(22, "KKKK")]})


def frame_whip9() -> dict[str, list[str]]:
    c = Canvas()
    legs_idle(c)
    put_pm(c, "body", UPPER_NA, 1, 0)
    put_pm(c, "prop", BELT0, 1, 0)
    c.segs("body", {y: [(26, "KsK")] for y in (37, 38)})
    horn_back(c, 31)
    c.stamp("prop", 26, 14, BOMB7)
    for x, y in ((28, 13), (27, 13), (26, 14), (25, 14), (25, 15)):
        c.put("prop", x, y, "t")
    c.stamp("prop", 22, 15, SPARK)                       # Y at (23, 16)
    put_pm(c, "body", HEAD0, 1, 1)
    paint(c, "body", line_mask((31.5, 31), (31.5, 21.5), 3.0), SKIN_NEAR)
    c.segs("body", {18: [(30, "KKKK")], 19: [(30, "KllK")], 20: [(30, "KmsK")], 21: [(30, "KKKK")]})   # hand x 30 to 33
    return c.rows()


def frame_release10() -> dict[str, list[str]]:
    c = Canvas()
    for y, x in ((47, 27), (48, 25), (49, 23), (50, 21), (51, 19)):                  # back leg, 3 px inner width, lit 'mm' (review fix)
        c.segs("body", {y: [(x, "KmmsK")]})
    c.segs("body", {52: [(15, "KtutK")], 53: [(14, "KttuttK")], 54: [(14, "KttttrK")], 55: [(14, "KKKKKKK")]})
    leg(c, [(47, 33, 3), (48, 35, 3), (49, 37, 3), (50, 38, 2)], False)
    boot(c, 37, 51, BOOT_NEAR)                           # lunging boot x 37 to 45
    put_pm(c, "body", UPPER_NA, 3, 1)
    paint(c, "body", line_mask((33, 32), (39, 29), 2.4) | line_mask((39, 29), (44, 26), 2.4), SKIN_NEAR)   # arm behind the face
    put_pm(c, "body", HEAD0, 3, 1)
    put_pm(c, "prop", BELT0, 3, 1)
    horn_back(c, 33)
    c.segs("body", {y: [(26, "KsK")] for y in (39, 40)})
    c.stamp("prop", 50, 17, BOMB7)                       # centre (53, 20): projectile spawn point
    for x, y in ((52, 16), (51, 15)):
        c.put("prop", x, y, "t")
    c.stamp("prop", 49, 13, SPARK)                       # Y at (50, 14)
    c.segs("body", {24: [(44, "KlKl")], 25: [(44, "lllK")], 26: [(44, "KmmK")], 27: [(45, "KK")]})   # open hand
    return c.rows()


def frame_follow11() -> dict[str, list[str]]:
    c = Canvas()
    legs_idle(c)
    std(c, head=(2, 0), upper=(2, 0), horn=(0, 2), bbomb=(2, 0), pack_bomb=False, fists=False)
    put_pm(c, "body", FAR_HAND0, -5, 2)
    c.segs("body", {40: [(38, "KllK")], 41: [(38, "KlmK")], 42: [(38, "KmsK")], 43: [(38, "KsKK")]})   # empty open hand
    return c.rows()


# --------------------------------------------------------------------------- hurt (key 12)
def frame_hurt12() -> dict[str, list[str]]:
    c = Canvas()
    head = dict(HEAD0)
    gog = [(x, y) for y, x0, x1 in ((19, 33, 39), (20, 32, 40), (21, 33, 40)) for x in range(x0, x1 + 1)]
    moved = {(x, y - 1): head[(x, y)] for x, y in gog if (x, y) in head}
    for k in gog:
        head.pop(k, None)
    head.update(moved)                                   # goggles knocked up 1 px
    for x in range(33, 40):
        head[(x, 21)] = "m"
    head[(40, 21)] = "K"
    for x, ch in ((34, "w"), (35, "K"), (37, "w"), (38, "K")):
        head[(x, 23)] = ch                               # eye whites with a K pupil
    for y in (26, 27, 28):                               # O mouth: 3 x 3 'o' ringed by K
        for x in range(34, 37):
            head[(x, y)] = "o"
        head[(33, y)] = head[(37, y)] = head[(38, y)] = "K"
    head.pop((44, 26), None)
    head[(43, 26)], head[(44, 24)] = "K", "K"            # nose tip 1 px higher
    legs_idle(c, heel_lift=True)
    put_pm(c, "body", UPPER_NA, -3, 0)
    put_pm(c, "body", head, -4, 0)
    put_pm(c, "prop", PACKBOMB0, -3, -2)
    put_pm(c, "prop", BELT0, -3, 0)
    pipe_line(c, 40, 28, 44, 26)
    bell(c, (44.5, 26.0), (47.0, 19.5), 1.5, 4.2)        # bell flung up and forward, pipe stays visible (review fix)
    c.segs("body", {29: [(35, "m")], 30: [(35, "m")]})
    c.segs("body", {28: [(36, "KKKK")], 29: [(36, "KlmK")], 30: [(36, "KmsK")], 31: [(36, "KKKK")]})   # near fist x 36 to 39
    return c.rows()


# --------------------------------------------------------------------------- down (13, 14, key 15)
def rotate_pm(p, pivot, dest, deg):
    """Nearest-neighbour rotation of a pixel map counter-clockwise on screen by ``deg`` around ``pivot`` to ``dest``."""
    import math
    cs, sn = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    out = {}
    for y in range(CANVAS[1]):
        for x in range(CANVAS[0]):
            rx, ry = x + .5 - dest[0], y + .5 - dest[1]
            sx, sy = rx * cs - ry * sn + pivot[0], rx * sn + ry * cs + pivot[1]
            k = (int(sx // 1), int(sy // 1))
            if k in p:
                out[(x, y)] = p[k]
    return out


def frame_fall13() -> dict[str, list[str]]:
    """Mid-fall, 45 degrees: legs still planted (key 0), torso, pack and scarf rule-drawn (clean inner outlines), the head the
    only rotated key-0 part (rotation of the whole upper body made muddy inner outlines; review fix)."""
    c = Canvas()
    legs_idle(c)
    paint(c, "body", line_mask((25.5, 49.0), (17.0, 42.0), 6.5), VEST)                     # pack, on the downhill (back) side
    paint(c, "body", line_mask((29.5, 46.0), (22.5, 38.5), 8.0), VEST)                     # torso
    paint(c, "body", rect(19, 36, 22, 40), lambda x, y, m: "R" if y < 38 else "B")         # scarf at the neck
    put_pm(c, "body", rotate_pm(HEAD0, (37.0, 24.0), (14.5, 32.5), 42.0))
    c.stamp("prop", 11, 41, BOMB7)                                                         # pack bomb, unlit, rides the pack
    pipe_line(c, 27, 40, 31, 38)
    bell(c, (31.5, 37.0), (36.0, 28.5), 1.5, 4.5)        # horn flung up and forward, pipe visible
    c.segs("body", {37: [(24, "KKKK")], 38: [(24, "KlmK")], 39: [(24, "KmsK")], 40: [(24, "KKKK")]})   # near fist on the pipe
    return c.rows()


def lying(lift: int, bomb: tuple[int, int]) -> dict[str, list[str]]:
    c = Canvas()
    paint(c, "body", line_mask((36, 51), (42, 46), 4) | line_mask((42, 46), (42, 52), 4), SKIN_FAR)   # far leg
    boot(c, 38, 51, BOOT_FAR)
    paint(c, "body", line_mask((39, 51), (46, 43), 4) | line_mask((46, 43), (47, 52), 4), SKIN_NEAR)  # near leg, knee up
    boot(c, 43, 51, BOOT_NEAR)
    paint(c, "body", rect(33, 49, 39, 54), "r")                                                      # shorts
    dy = -lift
    paint(c, "body", rect(17, 50 + dy, 27, 54 + dy), VEST)                                           # pack under the shoulders
    top = {20: 47, 21: 46, 22: 45, 23: 45, 24: 45, 25: 45, 26: 45, 27: 46, 28: 46, 29: 46, 30: 47, 31: 47, 32: 48, 33: 48, 34: 48, 35: 49}
    paint(c, "body", {(x, y + dy) for x, t in top.items() for y in range(t, 53 if x <= 28 else 54)}, VEST)
    paint(c, "body", rect(17, 45 + dy, 19, 49 + dy), lambda x, y, m: "R" if y < 47 + dy else "B")    # neck scarf
    paint(c, "body", disc(12, 47 + dy, 6.5, 5.2), SKIN_NEAR)                                          # head, face up
    c.segs("body", {y + dy: [(7, s)] for y, s in ((44, "RRRRRRRRRRR"), (45, "BBBBBBBBBBB"))})        # headwrap over the eyes
    for y, x, s in ((41, 8, "KbbK"), (42, 8, "bQwb"), (43, 8, "KbbK"), (40, 13, "KbbK"), (41, 13, "bQwb"), (42, 13, "KbbK")):
        c.segs("body", {y + dy: [(x, s)]})                                                           # goggles askew on top
    c.segs("body", {y + dy: [(x, s)] for y, x, s in ((49, 14, "KooK"), (50, 14, "owwo"))})           # open mouth, lower teeth
    c.segs("body", {53 + dy: [(6, "KlK")], 54 + dy: [(4, "KlmmK")], 55 + dy: [(4, "KKKKK")]})        # ear flat along the floor
    bell(c, (37.0, 49.5), (43.0, 43.0 + dy), 1.4, 4.0)                                               # horn across the belly, bell tilted up and forward
    c.put("prop", 26, 46 + dy, "h")
    c.put("prop", 26, 47 + dy, "n")
    pipe_line(c, 27, 46 + dy, 37, 49 + dy)
    paint(c, "body", line_mask((23, 48 + dy), (32, 50 + dy), 3), SKIN_NEAR)                          # arm over the horn
    c.stamp("prop", bomb[0], bomb[1], BOMB7)                                                         # rolled bomb, unlit
    return c.rows()


def frame_down14() -> dict[str, list[str]]:
    return lying(2, (52, 49))


def frame_down15() -> dict[str, list[str]]:
    return lying(0, (57, 49))


FRAMES = list(zip(
    [frame_idle0, frame_idle1, frame_idle2, frame_walk3, frame_walk4, frame_walk5, frame_walk6, frame_toot7,
     frame_lit8, frame_whip9, frame_release10, frame_follow11, frame_hurt12, frame_fall13, frame_down14, frame_down15],
    TICKS))



# --------------------------------------------------------------------------- grid
def build_grid() -> dict:
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict(key=k, name=n, rgba=c, **({"accent": True} if a else {})) for k, n, c, a in PALETTE],
        "layers": list(LAYERS),
        "frames": [{"duration_ms": round((t if t else HOLD_MS * 60 / 1000) * 1000 / 60),
                    "cels": fn() if i == 0 else despeckle(fn())} for i, (fn, t) in enumerate(FRAMES)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


MANIFEST = ROOT / "assets/manifests/bard_01.json"
ASE = BARD_DIR / "bard_01.ase"
TAG_OF = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once", "attack_recover": "once",
            "hurt": "once", "down": "hold"}


def manifest() -> dict:
    frames = [{"index": n, "tag": TAG_OF[n], "ticks": TICKS[n],
               "duration_ms": round((TICKS[n] if TICKS[n] else HOLD_MS * 60 / 1000) * 1000 / 60),
               "author": "key" if n in KEY_FRAMES else "sonnet"} for n in range(len(FRAMES))]
    return {
        "schema_version": 1,
        "id": "bard_goblin_h40_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/goblin/bard/bard_01.ase",
        "grid_source": "assets/source/characters/goblin/bard/bard_01.grid.json",
        "output_atlas": "generated/art/bard_01.png",
        "output_metadata": "generated/art/bard_01.json",
        "palette_id": "bard-floor01-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Goblin Bomb Bard; runtime role IP-neutral ranged thrower)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("BARD-01 Goblin Bomb Bard at 40 px idle height: 7 frozen keys (atlas frames 0, 3, 7, 8, 10, 12, 15, fixture "
                       "tests/fixtures/bard-01_keys.json) plus 9 in-betweens built by scripts/author_bard01.py from literal pixel "
                       "segments, moved or edited key 0 parts and a few rule-drawn shapes; layers body and prop"),
            "generator": ("Spec and hero key 0: Claude Opus. Keys 3/7/8/10/12/15, in-betweens, atlas, validators and evidence: Claude Sonnet, "
                          "composed by scripts/author_bard01.py. Independent top-tier review: pass with execution items (19/25), items applied by Claude Sonnet. No image-model raster output, "
                          "no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-goblin-bomb-bard-concept-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "bard": {
            "status": "keys and in-betweens drawn, independent review passed with execution items (applied); Bevy integration pending; 16 frames, 7 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/bard-01-spec.md",
            "opaque_height_idle": 40,
            "height_cap_px": 43,
            "tick_hz": 60,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/bard-01_keys.json",
            "attack_ticks": {"tell": [0, 29], "swing": [30, 36], "projectile_spawn_tick": 36, "recover": [37, 50]},
            "projectile_spawn_px_from_pivot": {"x": 25, "y_up": 35, "frame": 10, "facing_right_authored": True},
            "boxes_px_from_pivot": {
                "standing": {"x": [-12, 11], "y": [0, 37]},
                "hurtbox_idle_move_tell": {"x": [-10, 9], "y": [0, 35]},
                "hurtbox_hurt": {"x": [-13, 6], "y": [0, 35]},
                "attack_hitbox": None,
            },
            "frames": frames,
        },
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def preview(grid: dict, out: Path, scale: int = 8) -> None:
    from PIL import Image  # preview only

    pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for p in grid["palette"]}
    w, h = grid["canvas"]
    n = len(grid["frames"])
    bg = (58, 34, 30, 255)  # warm brick tone, for the read only
    img = Image.new("RGBA", (w * n, h), bg)
    for i, fr in enumerate(grid["frames"]):
        for layer in grid["layers"]:
            for y, row in enumerate(fr["cels"].get(layer, [])):
                for x, ch in enumerate(row):
                    if ch != ".":
                        img.putpixel((i * w + x, y), pal[ch])
    big = img.resize((w * n * scale, h * scale), Image.NEAREST)
    sheet = Image.new("RGBA", (big.width + w * n * 2 + 8, big.height), bg)
    sheet.paste(big, (0, 0))
    sheet.paste(img, (big.width + 4, 0))
    sheet.paste(img.resize((w * n * 2, h * 2), Image.NEAREST), (big.width + 4, h + 8))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    k = sub.add_parser("keys")
    k.add_argument("--check", action="store_true")
    k.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    grid = build_grid()
    text = dump(grid)
    if a.preview:
        preview(grid, a.preview)
    if a.check:
        if not GRID.exists() or GRID.read_text() != text:
            print(f"{GRID.relative_to(ROOT)} differs from the authoring script", file=sys.stderr)
            return 1
        if not MANIFEST.exists() or MANIFEST.read_text() != json.dumps(manifest(), indent=2) + "\n":
            print(f"{MANIFEST.relative_to(ROOT)} differs from the authoring script", file=sys.stderr)
            return 1
        return 0
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text)
    sys.path.insert(0, str(ROOT / "scripts"))
    import pixel_grid_to_ase as pg  # noqa: E402
    pg.write_ase(grid, ASE)
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest(), indent=2) + "\n")
    print(f"wrote {GRID.relative_to(ROOT)}, {ASE.relative_to(ROOT)}, {MANIFEST.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
