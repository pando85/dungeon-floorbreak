#!/usr/bin/env python3
"""ENG-01 authoring record: Goblin Engineer palette, part blocks and the idle hero key.

Usage:

    python3 scripts/author_eng01.py keys [--check]
    python3 scripts/author_eng01.py key0-scale

``keys`` (re)writes ``assets/source/characters/goblin/engineer/eng_01.grid.json`` from the literal key-0 rows below
(see ``docs/02-art/eng-01-spec.md`` sections 3, 6 and 7). Design stage: the grid holds only the hero key, frame 0
(``idle``), frozen in ``tests/fixtures/eng-01_keys.json``. The executor appends frames 1 to 17 in this script from the
part blocks (``POT``, ``HEAD``, ``TORSO``, ``LEGS``, ``WRENCH``) and the walk contact plan, extends ``TAGS``/``TICKS``,
the config and the fixture, and writes the manifest; frame 0 must stay byte-identical.

Construction (no concept pixels traced or sampled):

* rows 26 to 55 are the GOB-01 variant A idle body (``goblin_a_h36.grid.json`` frame 0, layer ``body``) unchanged,
  apart from the closed near fist (the pineapple stick is gone);
* rows 16 to 25 are the DOZ-01 engineer rider's saucepan, handle and pot-shadowed head row
  (``dozer_riders_h36.grid.json`` frame 2, ``engineer_hold``, rows 25 to 34), moved up 9 rows: the rider's seat offset
  undone, so the standalone sprite is the same individual pixel for pixel from the pot rim up to the chin;
* new: the ear tip pushed out under the handle, a rag belt with an iron buckle on row 42 (the stick strap's row), and
  the wrench stowed jaw-up in a belt loop at the back hip (layer ``prop``; it is never held or swung).

Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENG_DIR = ROOT / "assets/source/characters/goblin/engineer"
GRID = ENG_DIR / "eng_01.grid.json"
ASE = ENG_DIR / "eng_01.ase"
MANIFEST = ROOT / "assets/manifests/eng_01.json"
CANVAS = (64, 56)
PIVOT = (28, 55)
H = 40                                   # idle opaque height: the GOB-01 36 px body plus 4 rows of pot above the crown
LAYERS = ["body", "prop"]
TICK_HZ = 60

# eng-floor01-v0 (17 of 20). The first eleven are goblin-floor01-v0 keys with the same hex (family read), the iron ramp
# 2 to 7 is the DOZ-01 rider ramp with the same keys and hex (the pot is the rider's pot). Pot, buckle and wrench share
# the iron ramp, so the engineer's metal reads as one kit. No wood, pineapple or leaf keys: no stick.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "rag_dark", "#2c1e28ff", False),
    ("t", "rag_mid", "#4c3436ff", False),
    ("u", "rag_light", "#73524aff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
    ("2", "iron_2", "#2c3046ff", False),
    ("3", "iron_3", "#3f445eff", False),
    ("4", "iron_4", "#575e7aff", False),
    ("5", "iron_5", "#78809aff", False),
    ("6", "iron_6", "#a3abbfff", False),
    ("7", "iron_glint", "#dde2ecff", True),
]
KEYS = [p[0] for p in PALETTE]
TAGS = [("idle", 0, 2), ("walk", 3, 8), ("attack_tell", 9, 10), ("attack_ram", 11, 12), ("attack_recover", 13, 13),
        ("hurt", 14, 14), ("down", 15, 17), ("getup", 18, 19), ("attack_ram_run", 20, 21)]
# 60 Hz; 0 = held last frame (state-driven). Frames 18 to 21 are ENG-02 (docs/02-art/eng-02-spec.md section 6): the
# get-up keys at 10 and 12 ticks (the q04 get-up window), the ram-run pair at 4 ticks a frame of the ram clock.
TICKS = [14, 10, 14, 6, 6, 6, 6, 6, 6, 8, 24, 3, 6, 18, 12, 5, 10, 0, 10, 12, 4, 4]
KEY_FRAMES = (0, 3, 10, 12, 13, 14, 17)             # the ENG-01 keys, frozen in tests/fixtures/eng-01_keys.json
ENG02_KEY_FRAMES = (18, 19)                          # the ENG-02 get-up keys, frozen in tests/fixtures/eng-02_keys.json

# Part blocks of key 0 (inclusive x0, y0, x1, y1). In-betweens and keys move these as whole blocks wherever the spec
# says "offset"; nothing is rotated or rescaled.
POT = (13, 16, 42, 24)                   # saucepan, handle and rim lip (rider rows 25 to 33)
HEAD = (16, 25, 44, 33)                  # ear tip, brow, eyes, snout, teeth, jaw and chin outline
TORSO = (19, 30, 41, 45)                 # rag cape, neck, belly, near arm and fist, belt and rag skirt
LEGS = (19, 46, 38, 55)                  # far leg (deep ramp) and near leg (light ramp), feet on row 55
WRENCH = (14, 35, 18, 42)                # prop layer: jaw up, shaft down into the belt loop at the back hip

# --------------------------------------------------------------------------- frame 0: idle hero key (literal rows)
KEY0 = {
    16: "...........................KKKKKKKKKKKKKK.......................",
    17: "..........................K666666666666KKK......................",
    18: "..........................K56665555544432K......................",
    19: "..............KKKK........K56765555544432K......................",
    20: ".............K55K5KKKK....K56665555544432K......................",
    21: ".............K33335555KKKKK56665555544432K......................",
    22: "..............KKKK33335555K56665555544432K......................",
    23: "..................KKKK33336666666666555544K.....................",
    24: "..................KKKKKKKKKKKKKKKKKKKKKKKK......................",
    25: "................KKllaaKKKKsmmmmmmmmaaKKmmK......................",
    26: ".................KKssaaKKKsmmmmmmseeKaeKlmK.....................",
    27: "...................KKKKKKKlmmmmmmmsaammmlmmK....................",
    28: "........................KKlmmmmmssKKKKmmsmmsK...................",
    29: "........................KKKlmmmmsKwwowKKmsssK...................",
    30: "......................KKuuuKlmmsKooooowKKKKK....................",
    31: ".....................KuuuutrKssssKKKKKKsK.......................",
    32: "....................KuutttttrKsssssssssK........................",
    33: "....................KuttttttrrKKKKKKKKK.........................",
    34: "....................KuttttttrrKlmKmmmsK.........................",
    35: "...............K..KKutttrtttrKlmsKmmmmsK........................",
    36: "..............K5..4KuttttrttrKlmsKmmmsK.........................",
    37: "..............K6KK4KutttttrtrKlmsKmmmKKKK.......................",
    38: "..............K6554KuttttttrrKlmsKmmKllmmK......................",
    39: "...............K65KKutttttrrrKlmmKKmmmmmK.......................",
    40: "................K5KKrtrKrtrrrKslmmmsssK.........................",
    41: ".................K4KrrKKtrrKrrKssssKK...........................",
    42: ".................KKKuutuutuutuK56KuK............................",
    43: "...................KrrrrrrrtrrrrrrK.............................",
    44: "...................KrrrrrrtrrrrtrrK.............................",
    45: "....................KrrKrrtKKrrtKrrK............................",
    46: "....................KKK.KKK..KKK.KKK............................",
    47: ".....................KmaK.....KlsK..............................",
    48: ".....................KmaK.....KlsK..............................",
    49: "....................KmmaK.....KlmmK.............................",
    50: ".....................KmaK......KlmK.............................",
    51: ".....................KmaK......KlmK.............................",
    52: "....................KKmsmK...KlmmlKK............................",
    53: "...................KmmsssmK.KlmmmmllKK..........................",
    54: "...................KmaaaaaK.KlssssssllK.........................",
    55: "....................KKKKKK...KKKKKKKKK..........................",
}


def in_box(x: int, y: int, box: tuple[int, int, int, int]) -> bool:
    x0, y0, x1, y1 = box
    return x0 <= x <= x1 and y0 <= y <= y1


def frame_idle0() -> dict[str, list[str]]:
    """Key 0 split into the two disjoint layers: the wrench box left of the cape outline (x <= 18) is ``prop``."""
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    for y, row in KEY0.items():
        if len(row) != w:
            raise ValueError(f"row {y} is {len(row)} px wide")
        for x, ch in enumerate(row):
            if ch == ".":
                continue
            if ch not in KEYS:
                raise ValueError(f"unknown key {ch!r} at ({x}, {y})")
            layer = "prop" if in_box(x, y, WRENCH) else "body"
            cels[layer][y][x] = ch
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}

# --------------------------------------------------------------------------- execution: parts, composer, frames 1 to 17
# Frame builder: key 0 is split into whole pixel blocks (POT, HEAD, TORSO, ARM, LEGS, WRENCH). Every other frame moves
# those blocks by the section 7/8 offsets, redraws only what the spec says (legs, arms, the leaning sack in the ram), and
# closes outline seams with ``K``. No block is rotated or rescaled; nothing is edited by hand.
CAPE_RIGHT = {30: 27, 31: 28, 32: 29, 33: 29}   # rows where HEAD and the cape share the block box: cape pixels end here
HOLD_MS = 100                                    # preview duration of the held last frame (the runtime state holds it)
GOBLIN_GRID = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
RIDER_GRID = ROOT / "assets/source/enemies/dozer/dozer_riders_h36.grid.json"

Pix = dict  # {(x, y): key}


def _merged_cels(cels: dict[str, list[str]]) -> Pix:
    out: Pix = {}
    for rows in cels.values():
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch != ".":
                    out[(x, y)] = ch
    return out


def split_parts() -> dict[str, Pix]:
    """Partition key 0 into disjoint blocks. Rows 43 to 46 (skirt and hem) belong to TORSO, rows 47 to 55 to LEGS."""
    cels = frame_idle0()
    parts: dict[str, Pix] = {k: {} for k in ("POT", "HEAD", "TORSO", "ARM", "LEGS", "WRENCH")}
    last_cape = {}
    for y in range(34, 42):
        row = cels["body"][y]
        last_cape[y] = max(x for x in range(0, 32) if row[x] in "utr")
    for layer, rows in cels.items():
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch == ".":
                    continue
                if layer == "prop":
                    name = "WRENCH"
                elif y <= 24:
                    name = "POT"
                elif y <= 29:
                    name = "HEAD"
                elif y <= 33:
                    name = "TORSO" if x <= CAPE_RIGHT[y] else "HEAD"
                elif y <= 41:
                    name = "ARM" if x > last_cape[y] + 1 else "TORSO"
                elif y <= 46:
                    name = "TORSO"
                else:
                    name = "LEGS"
                parts[name][(x, y)] = ch
    return parts


PARTS = split_parts()


class Canvas:
    def __init__(self) -> None:
        self.body: Pix = {}
        self.prop: Pix = {}

    def paint(self, pix: Pix, dx: int = 0, dy: int = 0, layer: str = "body") -> "Canvas":
        tgt = self.body if layer == "body" else self.prop
        for (x, y), ch in pix.items():
            tgt[(x + dx, y + dy)] = ch
        return self

    def block(self, name: str, dx: int = 0, dy: int = 0) -> "Canvas":
        return self.paint(PARTS[name], dx, dy, "prop" if name == "WRENCH" else "body")

    def close(self) -> "Canvas":
        """Close outline seams: every empty 4-neighbour of a non-outline pixel becomes ``K``."""
        add = {}
        for (x, y), ch in self.body.items():
            if ch == "K":
                continue
            for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if n not in self.body and n not in self.prop and 0 <= n[0] < CANVAS[0] and 0 <= n[1] < CANVAS[1]:
                    add[n] = "K"
        self.body.update(add)
        return self

    def cels(self) -> dict[str, list[str]]:
        w, h = CANVAS
        out = {}
        for name, pix in (("body", self.body), ("prop", {k: v for k, v in self.prop.items() if k not in self.body})):
            rows = [["."] * w for _ in range(h)]
            for (x, y), ch in pix.items():
                if not (0 <= x < w and 0 <= y < h):
                    raise ValueError(f"pixel outside the frame: {(x, y)}")
                if ch not in KEYS:
                    raise ValueError(f"unknown key {ch!r}")
                rows[y][x] = ch
            out[name] = ["".join(r) for r in rows]
        return out


def thick_line(p0, p1, thick=2) -> list[tuple[int, int]]:
    (x0, y0), (x1, y1) = p0, p1
    n = max(abs(x1 - x0), abs(y1 - y0), 1)
    pts = {(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)) for i in range(n + 1)}
    return sorted({(x, y + k) for x, y in pts for k in range(thick)})


def shade_skin(points) -> Pix:
    """Skin shading for a drawn limb: light on the upper edge, shade on the lower edge, mid between."""
    pts = set(points)
    return {(x, y): ("l" if (x, y - 1) not in pts else "s" if (x, y + 1) not in pts else "m") for x, y in pts}


def fist(cx: int, cy: int) -> Pix:
    return shade_skin([(x, y) for x in range(cx - 1, cx + 2) for y in range(cy - 1, cy + 2)])


SHIN_RANGE = {"far": (-4, 13), "near": (-10, 2)}   # keeps the 4 px shin under the skirt hem; the foot may reach further


def legs(c: Canvas, near=(0, 0), far=(0, 0), top=0, far_heel_up=False, hem=None) -> None:
    """Legs from key 0 rows 47 to 55: each foot (rows 52 to 55) moves whole by (dx, dy), a lifted foot rises whole.
    The shin (rows 47 to 51) moves rigidly with the foot, clamped to ``SHIN_RANGE`` so it stays under the hem; the one
    row where shin and foot meet may step by the clamped remainder (the foot is wider than the shin)."""
    leg: Pix = {}
    for side, (fdx, fdy) in (("far", far), ("near", near)):
        for (x, y), ch in PARTS["LEGS"].items():
            if (x <= 27) != (side == "far"):
                continue
            lo, hi = SHIN_RANGE[side]
            shift = fdx if y >= 52 else max(lo, min(hi, fdx))
            ny = y + fdy
            if far_heel_up and side == "far" and y >= 52 and x - 20 < 3:
                ny -= 1
            if hem is None or ny > hem:                  # rows at or above the hem are hidden behind the skirt
                leg[(x + shift, ny)] = ch
    for (x, y), ch in list(leg.items()):                 # drop skin slivers the near leg leaves of the far leg
        if ch != "K" and not any(leg.get((x + a, y + b)) == ch for a in (-1, 0, 1) for b in (-1, 0, 1) if a or b):
            del leg[(x, y)]
    c.body.update(leg)


def mouth(pix: Pix, mode: str) -> Pix:
    out = dict(pix)
    if mode == "grit":              # tooth row continuous, the dark mouth rows closed to outline
        for (x, y), ch in pix.items():
            if ch == "o":
                out[(x, y)] = "w" if y == 29 else "K"
    elif mode == "slack":           # the tooth row dropped one row, the jaw hangs
        for (x, y), ch in pix.items():
            if ch == "w" and y == 29:
                out[(x, y)] = "o"
                out[(x, 30)] = "w"
    return out


def lean_sack(c: Canvas, neck: tuple[int, int], hip_x: int = 27) -> None:
    """Rag sack leaning forward: scanlines from the neck down to row 41, centre sliding from the neck to the hips, cape
    banding u/t/r as in key 0; skirt, belt and hem rows 42 to 46 are key 0's."""
    ny0, nx0 = neck[1], neck[0]
    for y in range(ny0, 42):
        cx = round(nx0 + (hip_x + 3 - nx0) * (y - ny0) / (42 - ny0))
        lo, hi = cx - 7, cx + 7
        for x in range(lo, hi + 1):
            i = x - lo - 1
            ch = "K" if x in (lo, hi) else "u" if i < 2 else "r" if x >= hi - 2 else "t"
            c.body[(x, y)] = ch
    for (x, y), ch in PARTS["TORSO"].items():
        if y >= 42:
            c.body[(x, y)] = ch


def point_arm() -> Pix:
    """The rider's pointing arm (DOZ-01 ``engineer_point`` rows 39 to 44, x 40 to 60; row 39 only x 44 and up, the chin
    outline of the rider's head is left out), moved to rows 30 to 35 and 2 px left."""
    rider = json.loads(RIDER_GRID.read_text(encoding="utf-8"))["frames"][0]["cels"]["rider"]
    out: Pix = {}
    for y in range(39, 45):
        for x in range(40, 61):
            if rider[y][x] != "." and not (y == 39 and x < 44):
                out[(x - 2, y - 9)] = rider[y][x]
    return out


def gob_body(frame: int) -> Pix:
    g = json.loads(GOBLIN_GRID.read_text(encoding="utf-8"))["frames"][frame]["cels"]["body"]
    return _merged_cels({"body": g})


def head_pot(c: Canvas, head, pot, order=("HEAD", "POT"), mouth_mode=None) -> None:
    """Paint HEAD and POT back to front in ``order`` (the last paints on top)."""
    for name in order:
        d = head if name == "HEAD" else pot
        if name == "HEAD" and mouth_mode:
            c.paint(mouth(PARTS["HEAD"], mouth_mode), *d)
        else:
            c.block(name, *d)


def base(dx=0, dy=0, wrench=True, arm=True) -> Canvas:
    c = Canvas()
    c.block("TORSO", dx, dy)
    if arm:
        c.block("ARM", dx, dy)
    if wrench:
        c.block("WRENCH", dx, dy)
    return c


# ---- idle
def f_idle(n: int) -> dict:
    if n == 1:                                           # breath: pot, head, torso, wrench 1 down, legs fixed
        c = Canvas()
        c.block("LEGS")
        for p in ("TORSO", "ARM", "WRENCH", "HEAD", "POT"):
            c.block(p, 0, 1)
        return c.cels()
    c = Canvas()                                         # 2: pot rattle, pot 1 up, the rim row stays closed
    for p in ("LEGS", "TORSO", "ARM", "WRENCH", "HEAD"):
        c.block(p)
    c.paint({k: v for k, v in PARTS["POT"].items() if k[1] == 24})
    c.block("POT", 0, -1)
    return c.cels()


# ---- walk (section 8 contact plan): per frame (body dx, body dy, arm dx from key 0, near foot, far foot)
NEAR_END, FAR_END = 37, 25                               # right ends of the key-0 feet on row 55
WALK = {
    3: (1, 2, -2, (39, 0), (27, 0)),
    4: (1, 0, -1, (35, 0), (31, -3)),
    5: (1, 1, 0, (31, 0), (35, -2)),
    6: (1, 2, 2, (27, 0), (39, 0)),
    7: (1, 0, 1, (31, -3), (35, 0)),
    8: (1, 1, 0, (35, -2), (31, 0)),
}


def f_walk(n: int) -> dict:
    dx, dy, arm_dx, (ne, nlift), (fe, flift) = WALK[n]
    c = Canvas()
    legs(c, near=(ne - NEAR_END, nlift), far=(fe - FAR_END, flift), top=dx, hem=46 + dy)
    for p in ("TORSO", "WRENCH", "HEAD", "POT"):
        c.block(p, dx, dy)
    c.block("ARM", arm_dx, dy)
    return c.close().cels()


# ---- tell and ram
def f_tell9() -> dict:
    c = base(arm=False)
    legs(c, hem=46)
    head_pot(c, (-2, 0), (-2, 0))
    pts = thick_line((31, 36), (38, 36)) + thick_line((38, 36), (44, 32))
    c.paint(shade_skin(pts))
    c.paint(fist(46, 32))
    return c.close().cels()


def f_tell10() -> dict:
    c = base(-2, 0, arm=False)
    legs(c, near=(3, 0), far=(0, 0), top=-2, hem=46)
    head_pot(c, (-3, -1), (-3, -1))
    c.paint(point_arm())
    for y in range(30, 34):                              # close 1 to 2 px see-through gaps between chin and pointing arm
        for x in range(30, 40):
            if (x, y) not in c.body and (x - 1, y) in c.body and any((x + k, y) in c.body for k in (1, 2)):
                c.body[(x, y)] = "K"
    return c.close().cels()


def f_ram11() -> dict:
    c = Canvas()
    legs(c, near=(5, 0), far=(-3, 0), top=0, far_heel_up=True, hem=46)
    lean_sack(c, (31, 35))
    c.block("WRENCH", 1, 2)
    c.block("ARM", -4, 1)                                # the key-0 near arm hanging at the side (fist about (34, 40)), cape banding left of it
    head_pot(c, (3, 3), (3, 3))
    return c.close().cels()


def f_ram12() -> dict:
    c = Canvas()
    legs(c, near=(5, 0), far=(-3, 0), top=0, far_heel_up=True, hem=46)
    lean_sack(c, (37, 37))
    c.block("WRENCH", 2, 3)
    bent = thick_line((35, 38), (31, 42)) + thick_line((31, 43), (27, 41)) + [(x, y) for x in range(25, 28) for y in range(39, 42)]
    c.paint(shade_skin(bent))                            # arm bent at the elbow, fist at (26, 40), 3 px clear of the wrench
    head_pot(c, (8, 6), (8, 6), mouth_mode="grit")
    return c.close().cels()


def f_recover13() -> dict:
    c = base(arm=False)
    legs(c, hem=46)
    arm = PARTS["ARM"]
    c.paint(arm, 5, 0)
    for y in range(34, 42):                              # stretch the near arm out to the cape
        xs = [x for (x, yy) in arm if yy == y]
        for x in range(min(xs) + 1, min(xs) + 5):
            c.body.setdefault((x, y), "m")
    c.paint({(18, 34): "m", (19, 34): "m"})              # the far hand behind the cape
    head_pot(c, (2, 1), (3, 4), order=("HEAD", "POT"), mouth_mode="slack")
    return c.close().cels()


# ---- hurt and down: GOB-01 A bodies plus the pot (and, in the hurt, the belt row and the wrench)
def f_hurt14() -> dict:
    c = Canvas()
    c.paint(gob_body(16))
    for x in range(14, 32):                              # close the waist row 43 (the GOB-01 frame has a transparent row there)
        if (x, 42) in c.body and (x, 44) in c.body and (x, 43) not in c.body:
            c.body[(x, 43)] = "r"
    for x in range(11, 29):                              # belt row on the hip row: rag mid and a 2 px iron buckle
        if (x, 44) in c.body and c.body[(x, 44)] != "K":
            c.body[(x, 44)] = "5" if x in (23, 24) else "t"
    c.block("WRENCH", -8, 2)
    c.block("POT", -8, -3)
    return c.cels()


def f_down(n: int) -> dict:
    c = Canvas()
    c.paint(gob_body({15: 20, 16: 21, 17: 22}[n]), 10, 0)
    dy = {15: 14, 16: 30, 17: 31}[n]
    c.block("POT", -10, dy)
    return c.cels()


def build_frames_eng01() -> list[dict]:
    """ENG-01 frames 0 to 17 (frozen; unchanged by ENG-02)."""
    return [frame_idle0(), f_idle(1), f_idle(2)] + [f_walk(n) for n in range(3, 9)] + [
        f_tell9(), f_tell10(), f_ram11(), f_ram12(), f_recover13(), f_hurt14(), f_down(15), f_down(16), f_down(17)]


def build_frames() -> list[dict]:
    """Frames 0 to 17 plus ENG-02's 18 to 21 (the get-up keys byte for byte, then the ram-run pair)."""
    sys.path.insert(0, str(ROOT / "scripts"))
    import author_eng02 as eng02                         # noqa: E402  (eng02 imports this module: import it late)
    return build_frames_eng01() + eng02.atlas_frames()


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ) if ticks else HOLD_MS


def frames_grid() -> dict:
    frames = build_frames()
    assert len(frames) == len(TICKS) == TAGS[-1][2] + 1
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": cels} for t, cels in zip(TICKS, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# --------------------------------------------------------------------------- manifest
TAG_OF = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_ram": "once", "attack_recover": "once",
            "hurt": "once", "down": "hold", "getup": "once", "attack_ram_run": "loop"}


def manifest() -> dict:
    frames = [{"index": n, "tag": TAG_OF[n], "ticks": TICKS[n] or None, "duration_ms": duration_ms(TICKS[n]),
               "author": "key" if n in KEY_FRAMES + ENG02_KEY_FRAMES else "sonnet"} for n in range(len(TICKS))]
    return {
        "schema_version": 1,
        "id": "engineer_goblin_h40_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/goblin/engineer/eng_01.ase",
        "grid_source": "assets/source/characters/goblin/engineer/eng_01.grid.json",
        "output_atlas": "generated/art/eng_01.png",
        "output_metadata": "generated/art/eng_01.json",
        "palette_id": "eng-floor01-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _a, _b in TAGS],
        "required_alpha": True,
        "owner": ("AI-authored enemy animation candidate (fan-concept layer: Floor 1 Goblin Engineer; runtime role "
                  "IP-neutral foreman, pot-ram placeholder)"),
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("ENG-01 standalone Goblin Engineer at 40 px idle height: 7 frozen keys (atlas frames 0, 3, 10, 12, 13, 14, 17, "
                       "fixture tests/fixtures/eng-01_keys.json) plus 11 in-betweens built by scripts/author_eng01.py from key 0 "
                       "pixel blocks, GOB-01 A frames 16 and 20 to 22, the DOZ-01 rider's pointing arm and a few rule-drawn shapes; "
                       "layers body and prop. ENG-02 (docs/02-art/eng-02-spec.md) appends frames 18 and 19, the two frozen "
                       "get-up keys (fixture tests/fixtures/eng-02_keys.json, copied byte for byte), and frames 20 and 21, the "
                       "ram-run pair: frame 12's blocks with the ENG-01 walk 7 and walk 6 legs"),
            "generator": ("Spec and hero key 0: Claude Opus. Remaining keys, in-betweens, atlas, validators and evidence: Claude Sonnet, "
                          "composed by scripts/author_eng01.py. ENG-02 keys 18 and 19 and its spec: Claude Opus; its design gate: "
                          "Claude Opus (independent); frames 20 and 21: Claude Sonnet via scripts/author_eng02.py. Independent "
                          "review pending. No image-model raster output, no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png"],
        },
        "license": ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                    "no game/asset-pack distribution rights"),
        "review_status": "prototype_fixture_unapproved",
        "eng": {
            "status": "keys and in-betweens drawn; 22 frames (ENG-01 frames 0 to 17, ENG-02 frames 18 to 21: the get-up pair and "
                      "the ram-run pair); wired into the Bevy scene; independent review pending",
            "spec": "docs/02-art/eng-01-spec.md",
            "opaque_height_idle": 40,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/eng-01_keys.json",
            "eng02": {"spec": "docs/02-art/eng-02-spec.md", "key_frames": list(ENG02_KEY_FRAMES),
                      "key_fixture": "tests/fixtures/eng-02_keys.json",
                      "ram_run_cycle": [12, 20, 21, 20], "ram_run_ticks_per_frame": 4,
                      "getup_ticks": [10, 12]},
            "walk_speed_px_per_s": 40,
            "attack_ticks": {"tell": [0, 31], "ram": [32, 40], "hitbox_active": [35, 40], "recover": [41, 58]},
            "boxes_px_from_pivot": BOXES,
            "boxes_status": "spec placeholders, not re-measured from the drawn frames (owner game-feel review sets the final boxes)",
            "frames": frames,
        },
    }


# placeholder boxes from the spec (px from the pivot, x forward, y up), see section 3; re-measured below by the tests
BOXES = {
    "standing": {"x": [-15, 16], "y": [0, 39]},
    "hurtbox_idle_walk": {"x": [-9, 14], "y": [0, 37]},
    "hurtbox_tell": {"x": [-11, 12], "y": [0, 38]},
    "hurtbox_ram": {"x": [-6, 22], "y": [0, 33]},
    "hurtbox_recover": {"x": [-7, 16], "y": [0, 35]},
    "hurtbox_hurt": {"x": [-16, 10], "y": [0, 33]},
    "hurtbox_down": {"x": [-14, 26], "y": [0, 12]},
    "attack_hitbox_ram_frame_12": {"x": [14, 24], "y": [14, 33]},
}


def _scene_base():
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    return Image


def write_strips(out_dir: Path = ROOT / "docs/evidence/eng-01") -> list[Path]:
    """Walk strip (frames 3 to 8) and tell to ram strip (frames 9 to 13) at 4x on a flat warm brick tone."""
    Image = _scene_base()
    from scripts.pixelart.common import grid_images, load_grid_file
    imgs = grid_images(load_grid_file(GRID))
    out = []
    for name, rng in (("walk_strip.png", range(3, 9)), ("tell_ram_strip.png", range(9, 14))):
        sheet = Image.new("RGBA", (64 * 4 * len(rng), 56 * 4), (58, 34, 30, 255))
        for i, f in enumerate(rng):
            big = imgs[f].convert("RGBA").resize((64 * 4, 56 * 4), Image.NEAREST)
            sheet.paste(big, (i * 64 * 4, 0), big)
        out_dir.mkdir(parents=True, exist_ok=True)
        sheet.save(out_dir / name)
        out.append(out_dir / name)
    return out


def write_key0_scale(out: Path = ROOT / "docs/evidence/eng-01/key0-scale.png") -> Path:
    """Key 0 at 1x on lit Floor 1 next to Carl, the GOB-01 goblin A and the Bomb Bard (key 0), plus a 4x copy."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    goblin = grid_images(load_grid_file(ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"))[0]
    bard = grid_images(load_grid_file(ROOT / "assets/source/characters/goblin/bard/bard_01.grid.json"))[0]
    eng = grid_images(load_grid_file(GRID))[0]
    entries = [(carl, carl_piv, CARL_X, 0)]
    for i, img in enumerate((goblin, bard, eng)):
        f_img, f_piv = flip(img, tuple(PIVOT))
        entries.append((f_img, f_piv, CARL_X + 70 + 50 * i, 0))
    scene = Floor().render(entries, 1)
    panel_w = eng.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, eng.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = eng.resize((panel_w, eng.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 0), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "key0-scale", "strips"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    if args.command == "strips":
        print("wrote " + ", ".join(str(p.relative_to(ROOT)) for p in write_strips()))
        return 0
    text = dump(frames_grid())
    mtext = json.dumps(manifest(), indent=2) + "\n"
    if args.check:
        same = (GRID.is_file() and GRID.read_text(encoding="utf-8") == text and MANIFEST.is_file()
                and MANIFEST.read_text(encoding="utf-8") == mtext)
        print(f"{GRID.name} up to date" if same else f"{GRID.name} differs from the generator")
        return 0 if same else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    sys.path.insert(0, str(ROOT / "scripts"))
    import pixel_grid_to_ase as pg  # noqa: E402
    pg.write_ase(frames_grid(), ASE)
    MANIFEST.write_text(mtext, encoding="utf-8")
    print(f"wrote {GRID.relative_to(ROOT)}, {ASE.relative_to(ROOT)}, {MANIFEST.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
