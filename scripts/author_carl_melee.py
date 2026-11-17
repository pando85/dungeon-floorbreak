#!/usr/bin/env python3
"""Authoring record for ANIM-CARL-02: Carl 52 px unarmed melee candidate (phase 2).

Writes ``assets/source/characters/carl/carl_h52_melee.grid.json`` (pixel-grid-v1, 24 frames),
the editable source of the tags ``punch_a``, ``punch_b``, ``kick``, ``hurt``, ``knockback``,
``knockdown`` and ``getup`` defined in ``docs/02-art/carl-melee-anim-02-spec.md``.
``scripts/export_assets.sh`` turns the grid into the ``.ase`` and the atlas.

How the frames are made (no resampling, no rotation, no image model):

* **Frozen Opus keys.** ``tests/fixtures/carl_melee_phase1_keys.json`` holds the Opus key poses: the
  four phase 1 keys (a1, k1, k3, h0) and the four design-fix-round keys (b0, b1, kb, d0). They are
  copied into frames 1, 4, 5, 9, 11, 15, 18 and 19 without any change.
* **Copied from the approved locomotion idle** (``carl_h52_locomotion.grid.json``): hair, head
  (neutral face v0, glance v1), torso, jacket, pin and the boxer seat (master rows 31-38) are the
  master's own pixels, only translated (the spec's U(dx, dy)). The effort/wince/pain faces are
  taken from the Opus keys, translated back to idle position.
* **Stamps** (spec section 5): Crocs (flat, heel, toe, sole), fists, the back arm and the raised
  arm are cut out of the idle or the keys and placed at the offsets listed in the spec.
* **Hand-shaped parts** in this file: skin tubes (legs, bare arms) and leather sleeves are
  rasterised from a centre line and shaded with the master ramps; boxer hems and the few
  hand-placed pixels are ASCII art.

The committed grid is the source of truth. Run ``python3 scripts/author_carl_melee.py`` to rewrite
it (deterministic) or ``--check`` to verify that the committed grid equals the build output.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCO_PATH = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
KEYS_PATH = ROOT / "tests/fixtures/carl_melee_phase1_keys.json"
OUT_PATH = ROOT / "assets/source/characters/carl/carl_h52_melee.grid.json"

LAYERS = ["far_arm", "legs", "shoes", "torso", "boxers", "jacket", "head", "hair", "near_arm", "watch", "pin"]
W, H = 48, 56
TICK_HZ = 60

Pix = dict  # {(x, y): key}


# ------------------------------------------------------------------ pixel helpers

def parse(text: str, ox: int = 0, oy: int = 0) -> Pix:
    rows = text.strip("\n").split("\n")
    return {(ox + x, oy + y): c for y, r in enumerate(rows) for x, c in enumerate(r) if c not in ". "}


def shift(p: Pix, dx: int, dy: int) -> Pix:
    return {(x + dx, y + dy): c for (x, y), c in p.items()}


def rect(p: Pix, x0: int, y0: int, x1: int, y1: int) -> Pix:
    return {k: c for k, c in p.items() if x0 <= k[0] <= x1 and y0 <= k[1] <= y1}


def rows_of(p: Pix) -> list[str]:
    grid = [["."] * W for _ in range(H)]
    for (x, y), c in p.items():
        if 0 <= x < W and 0 <= y < H:
            grid[y][x] = c
    return ["".join(r) for r in grid]


def layers_from_cels(cels: dict) -> dict[str, Pix]:
    return {l: {(x, y): c for y, row in enumerate(cels.get(l, [])) for x, c in enumerate(row) if c != "."}
            for l in LAYERS}


LOCO = json.loads(LOCO_PATH.read_text(encoding="utf-8"))
KEYFILE = json.loads(KEYS_PATH.read_text(encoding="utf-8"))
IDLE0 = layers_from_cels(LOCO["frames"][0]["cels"])
IDLE2 = layers_from_cels(LOCO["frames"][2]["cels"])
KA = layers_from_cels(KEYFILE["keys"]["a1"]["cels"])
K1 = layers_from_cels(KEYFILE["keys"]["k1"]["cels"])
K3 = layers_from_cels(KEYFILE["keys"]["k3"]["cels"])
H0 = layers_from_cels(KEYFILE["keys"]["h0"]["cels"])
PALETTE = LOCO["palette"]

# Head variants in idle-0 coordinates (the keys' heads translated back by their U + head offset).
HEAD = {
    "v0": IDLE0["head"],
    "v1": shift(IDLE2["head"], 0, -1),          # glance back, closed mouth (master idle 2)
    "v2": shift(K3["head"], 3, -2),             # effort brow, open mouth (k3: U(-2,+2) + (-1,0))
    "v3": shift(KA["head"], -3, -2),            # effort brow, closed mouth (a1: U(+3,+2))
    "pain": shift(H0["head"], 7, -2),           # v0 with both eyes closed (h0: U(-5,+3) + (-2,-1))
}


class Frame:
    def __init__(self) -> None:
        self.L: dict[str, Pix] = {l: {} for l in LAYERS}
        self.locked: set[tuple[str, int, int]] = set()   # master pixels: never touched by cleanup

    def add(self, layer: str, pix: Pix, dx: int = 0, dy: int = 0, lock: bool = False) -> "Frame":
        for (x, y), c in pix.items():
            self.L[layer][(x + dx, y + dy)] = c
            if lock:
                self.locked.add((layer, x + dx, y + dy))
        return self

    def clear(self, layer: str, x0: int, y0: int, x1: int, y1: int) -> "Frame":
        for k in [k for k in self.L[layer] if x0 <= k[0] <= x1 and y0 <= k[1] <= y1]:
            del self.L[layer][k]
        return self

    def cels(self) -> dict[str, list[str]]:
        return {l: rows_of(self.L[l]) for l in LAYERS if self.L[l]}

    def flat(self) -> Pix:
        out: Pix = {}
        for l in LAYERS:
            out.update(self.L[l])
        return out


def upper(f: Frame, u: tuple[int, int], head: str, hoff: tuple[int, int] = (0, 0)) -> Frame:
    """Identity-locked upper block: master hair/head, torso, jacket, pin, boxer seat (rows 31-38)."""
    ux, uy = u
    f.add("torso", IDLE0["torso"], ux, uy, lock=True)
    f.add("jacket", IDLE0["jacket"], ux, uy, lock=True)
    f.add("pin", IDLE0["pin"], ux, uy, lock=True)
    f.add("boxers", rect(IDLE0["boxers"], 0, 31, W, 38), ux, uy, lock=True)
    f.add("head", HEAD[head], ux + hoff[0], uy + hoff[1], lock=True)
    f.add("hair", IDLE0["hair"], ux + hoff[0], uy + hoff[1], lock=True)
    return f


def idle_arms(f: Frame, d: tuple[int, int], far: tuple[int, int] | None = None) -> Frame:
    """Master idle far and near arms (and watch), translated."""
    fd = far if far is not None else d
    f.add("far_arm", IDLE0["far_arm"], *fd)
    f.add("near_arm", IDLE0["near_arm"], *d)
    f.add("watch", IDLE0["watch"], *d)
    return f


# ------------------------------------------------------------------ stamps

CROC_FLAT = shift(rect(IDLE0["shoes"], 15, 50, 24, 55), -15, 0)       # origin = leftmost X, row 50 = top
CROC_HEEL = shift(rect(H0["shoes"], 27, 50, 37, 55), -27, -50)   # origin = top-left of the 11 x 6 stamp
CROC_TOE = shift(rect(K1["shoes"], 33, 43, 38, 49), -33, -43)    # origin = top-left of the 6 x 7 stamp
CROC_SOLE = shift(rect(K3["shoes"], 40, 28, 46, 37), -40, -28)
FIST_BIG = shift(rect(KA["far_arm"], 41, 19, 46, 24), -41, -19)       # 6 x 6 with outline
FIST_SMALL = shift(rect(K1["far_arm"], 31, 16, 36, 19), -31, -16)     # 6 x 4 with outline
K1_FAR_ARM = rect(K1["far_arm"], 0, 0, W, 27)                          # fist + sleeve to the elbow
K3_FAR_ARM = K3["far_arm"]
BACK_ARM = {l: rect(KA[l], 0, 20, W, 34) for l in ("near_arm", "watch")}   # a1 near arm swung back
RAISED_ARM = {l: K3[l] for l in ("near_arm", "watch")}                      # k3 near arm up-back


def croc_flat(f: Frame, x: int) -> None:
    f.add("shoes", CROC_FLAT, x, 0)


def croc_toe_down(f: Frame, x: int, bottom: int = 55) -> None:
    """Back foot up on its toe (CROC_TOE stamp, bottom row on ``bottom``)."""
    f.add("shoes", CROC_TOE, x, bottom - 6)


# ------------------------------------------------------------------ rasterised limbs

def capsule(p0, p1, r0: float, r1: float | None = None) -> set:
    r1 = r0 if r1 is None else r1
    (x0, y0), (x1, y1) = p0, p1
    pad = int(max(r0, r1)) + 2
    out = set()
    dx, dy = x1 - x0, y1 - y0
    ln = dx * dx + dy * dy
    for y in range(int(min(y0, y1)) - pad, int(max(y0, y1)) + pad + 1):
        for x in range(int(min(x0, x1)) - pad, int(max(x0, x1)) + pad + 1):
            t = 0.0 if ln == 0 else max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / ln))
            cx, cy = x0 + t * dx, y0 + t * dy
            r = r0 + (r1 - r0) * t
            if math.hypot(x - cx, y - cy) <= r + 0.01:
                out.add((x, y))
    return out


def union(*masks) -> set:
    out: set = set()
    for m in masks:
        out |= m
    return out


def shade_skin(mask: set) -> Pix:
    """Master skin ramp: lit up-left edge S/L, base M, shadow down-right S then D."""
    def br_edge(x, y):
        return (x + 1, y) not in mask or (x, y + 1) not in mask

    def tl_edge(x, y):
        return (x - 1, y) not in mask or (x, y - 1) not in mask

    out: Pix = {}
    for (x, y) in mask:
        if br_edge(x, y):
            c = "D"
        elif ((x + 1, y) in mask and br_edge(x + 1, y)) or ((x, y + 1) in mask and br_edge(x, y + 1)):
            c = "S"
        elif tl_edge(x, y):
            c = "S"
        elif ((x - 1, y) in mask and tl_edge(x - 1, y)) or ((x, y - 1) in mask and tl_edge(x, y - 1)):
            c = "L"
        else:
            c = "M"
        out[(x, y)] = c
    return out


def shade_sleeve_rows(mask: set) -> Pix:
    """Diagonal or vertical sleeve: per row run K, T ... j ... J, K (the key's BACK_ARM style)."""
    ys = sorted({y for _, y in mask})
    out: Pix = {}
    for y in ys:
        xs = sorted(x for x, yy in mask if yy == y)
        runs, start = [], xs[0]
        for a, b in zip(xs, xs[1:] + [None]):
            if b is None or b != a + 1:
                runs.append((start, a))
                start = b
        for a, b in runs:
            for x in range(a, b + 1):
                if y in (ys[0], ys[-1]) or x in (a, b):
                    c = "K"
                elif x == a + 1:
                    c = "T"
                elif x == b - 1:
                    c = "J"
                else:
                    c = "j"
                out[(x, y)] = c
    return out


def shade_sleeve(mask: set) -> Pix:
    """Leather sleeve: K outline on the mask border, T sheen on the up-left rim, J shadow down-right."""
    def border(x, y):
        return any((x + dx, y + dy) not in mask for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))

    out: Pix = {}
    for (x, y) in mask:
        if border(x, y):
            c = "K"
        elif ((x - 1, y) in mask and border(x - 1, y)) or ((x, y - 1) in mask and border(x, y - 1)):
            c = "T"
        elif ((x + 1, y) in mask and border(x + 1, y)) or ((x, y + 1) in mask and border(x, y + 1)):
            c = "J"
        else:
            c = "j"
        out[(x, y)] = c
    return out


def leg(p0, p1, r0=3.0, r1=2.0) -> Pix:
    return shade_skin(capsule(p0, p1, r0, r1))


def leg_bent(hip, knee, ankle, r0=3.0, rk=2.8, r1=2.0) -> Pix:
    return shade_skin(union(capsule(hip, knee, r0, rk), capsule(knee, ankle, rk, r1)))


# Boxer leg openings, cut from the a1 key (two rows: shaded hem line and deep hem).
HEM_BACK = ["KUWWWWWK", "KUUUUUUK"]
HEM_FRONT = ["KWWWWWWVVK", "KUUUUUUUK"]


def hem(f: Frame, x: int, y: int, rows: list[str]) -> None:
    f.add("boxers", parse("\n".join(rows), x, y))


def standing_legs(f: Frame, hem_y: int, back_hem: int, front_hem: int, back_foot: int, front_foot: int,
                  back_front_hem_rows=(HEM_BACK, HEM_FRONT)) -> None:
    """Two grounded legs: boxer hems at ``hem_y`` (2 rows), skin tubes down to flat Crocs at row 50-55."""
    hem(f, back_hem, hem_y, back_front_hem_rows[0])
    hem(f, front_hem, hem_y, back_front_hem_rows[1])
    top = hem_y + 1
    f.add("legs", leg((back_hem + 3, top), (back_foot + 4, 49)))
    f.add("legs", leg((front_hem + 4, top), (front_foot + 4, 49), 3.0, 2.0))
    croc_flat(f, back_foot)
    croc_flat(f, front_foot)


def sleeve_arm(f: Frame, layer: str, pts: list, r: float = 2.0) -> Pix:
    mask: set = set()
    for i in range(len(pts) - 1):
        mask |= capsule(pts[i], pts[i + 1], r, r)
    dx, dy = abs(pts[-1][0] - pts[0][0]), abs(pts[-1][1] - pts[0][1])
    px = shade_sleeve(mask) if dx >= 2.5 * dy else shade_sleeve_rows(mask)
    f.add(layer, px)
    return px


def from_key(key: dict[str, Pix], layers: tuple[str, ...]) -> Frame:
    f = Frame()
    for l in layers:
        f.add(l, key[l])
    return f


# ------------------------------------------------------------------ frames

def a0() -> Frame:
    f = upper(Frame(), (1, 1), "v3")
    standing_legs(f, 40, 16, 26, 14, 28)
    f.add("far_arm", K1_FAR_ARM, 2, 1)
    f.add("near_arm", IDLE0["near_arm"], -2, 1)
    f.add("watch", IDLE0["watch"], -2, 1)
    return f


def a1() -> Frame:
    return from_key(KA, tuple(LAYERS))


def a2() -> Frame:
    f = from_key(KA, ("torso", "boxers", "jacket", "head", "hair", "pin", "legs", "shoes"))
    # far arm bent half back: elbow ~x36 row 23, fist x37-41 rows 18-21
    sleeve_arm(f, "far_arm", [(31, 22), (35, 23), (37, 21)], 2.0)
    f.add("far_arm", FIST_SMALL, 37, 18)
    f.add("near_arm", BACK_ARM["near_arm"], 1, -1)
    f.add("watch", BACK_ARM["watch"], 1, -1)
    return f


def a3() -> Frame:
    f = upper(Frame(), (1, 1), "v0")
    standing_legs(f, 40, 16, 26, 14, 28)
    f.add("far_arm", K3_FAR_ARM, 3, -1)
    f.add("near_arm", IDLE0["near_arm"], 1, 1)
    f.add("watch", IDLE0["watch"], 1, 1)
    return f


CUFF = shift(rect(KA["far_arm"], 38, 20, 41, 24), -41, -20)    # bunched T cuff, K column last


def flap(f: Frame, d: tuple[int, int], dark: bool = False, src: str = "k3") -> None:
    """Boxer leg opening of a kick key translated by ``d`` (replaces seat pixels, as in the key)."""
    pix = rect(K3["boxers"], 28, 34, 34, 40) if src == "k3" else rect(K1["boxers"], 29, 34, 36, 41)
    if dark:
        pix = {k: ("V" if c == "W" else "U" if c == "V" else c) for k, c in pix.items()}
    f.add("boxers", pix, *d)


def a1_stance(f: Frame) -> None:
    """Legs, hems and Crocs of key a1, unchanged (used by b0, a2)."""
    f.add("legs", KA["legs"])
    f.add("shoes", KA["shoes"])
    f.add("boxers", rect(KA["boxers"], 0, 41, W, H))


def b2() -> Frame:
    f = upper(Frame(), (6, 3), "v3", (1, 1))
    hem(f, 20, 42, HEM_BACK)
    hem(f, 30, 42, HEM_FRONT)
    f.add("legs", leg((23, 43), (15, 48), 3.0, 2.0))
    croc_toe_down(f, 12)
    f.add("legs", leg_bent((34, 43), (39, 47), (37, 49), 3.0, 2.8, 2.0))
    croc_flat(f, 33)
    sleeve_arm(f, "near_arm", [(23, 23), (34, 30)], 2.7)
    f.add("near_arm", FIST_BIG, 36, 30)
    return f


def b3() -> Frame:
    f = upper(Frame(), (2, 1), "v0")
    standing_legs(f, 40, 18, 28, 15, 29)
    idle_arms(f, (2, 1))
    return f


def k0() -> Frame:
    f = upper(Frame(), (0, 2), "v3")
    hem(f, 16, 41, HEM_BACK)
    hem(f, 26, 41, HEM_FRONT)
    f.add("legs", leg((19, 42), (19, 49)))
    croc_flat(f, 15)
    f.add("legs", leg_bent((30, 42), (31, 45), (31, 48), 3.0, 2.6, 2.0))
    croc_toe_down(f, 28)
    f.add("near_arm", K1["near_arm"], 1, -2)
    f.add("watch", K1["watch"], 1, -2)
    f.add("far_arm", K1_FAR_ARM, 1, 2)
    return f


def k1() -> Frame:
    return from_key(K1, tuple(LAYERS))


def k3_support(f: Frame) -> None:
    """Support leg, hem and Croc of key k3 (identical from k0 to k5 for x15-24)."""
    f.add("legs", rect(K3["legs"], 0, 43, W, H))
    f.add("shoes", rect(K3["shoes"], 0, 50, W, H))
    f.add("boxers", rect(K3["boxers"], 0, 41, 26, H))


def k2() -> Frame:
    f = upper(Frame(), (-2, 2), "v2", (-1, 0))
    k3_support(f)
    flap(f, (0, 0))
    f.add("legs", leg_bent((33, 36), (36, 37), (38, 38), 3.0, 2.6, 2.4))
    f.add("shoes", CROC_SOLE, 37, 31)
    sleeve_arm(f, "near_arm", [(15, 22), (8, 21)], 2.0)
    f.add("near_arm", FIST_SMALL, 2, 19)
    sleeve_arm(f, "far_arm", [(28, 20), (32, 23)], 2.0)
    f.add("far_arm", FIST_SMALL, 31, 21)
    return f


def k3() -> Frame:
    return from_key(K3, tuple(LAYERS))


def k4() -> Frame:
    f = upper(Frame(), (-1, 2), "v3", (-1, 0))
    k3_support(f)
    flap(f, (0, -1), src="k1")
    f.add("legs", leg_bent((33, 35), (37, 37), (35, 43), 3.0, 2.6, 2.0))
    f.add("shoes", CROC_TOE, 32, 44)
    sleeve_arm(f, "near_arm", [(16, 23), (10, 22)], 2.0)
    f.add("near_arm", FIST_SMALL, 4, 20)
    f.add("far_arm", K3_FAR_ARM, 1, 0)
    return f


def k5() -> Frame:
    f = upper(Frame(), (3, 2), "v0", (1, 0))
    f.add("boxers", rect(KA["boxers"], 0, 41, W, H))
    f.add("legs", rect(KA["legs"], 26, 43, W, H))
    f.add("shoes", rect(KA["shoes"], 30, 50, W, H))
    f.add("legs", leg((21, 42), (19, 49)))
    croc_flat(f, 15)
    sleeve_arm(f, "near_arm", [(20, 24), (28, 29)], 2.0)
    f.add("near_arm", FIST_SMALL, 29, 29)
    return f


def k6() -> Frame:
    f = upper(Frame(), (1, 1), "v3")
    standing_legs(f, 40, 16, 25, 15, 27)
    idle_arms(f, (1, 1))
    return f


def h1() -> Frame:
    f = upper(Frame(), (-3, 2), "v3", (-1, 0))
    standing_legs(f, 41, 13, 22, 10, 24)
    f.add("near_arm", BACK_ARM["near_arm"], 3, -1)
    f.add("watch", BACK_ARM["watch"], 3, -1)
    f.add("far_arm", K3_FAR_ARM, -1, 0)
    return f


def h2() -> Frame:
    f = upper(Frame(), (-1, 1), "v3")
    standing_legs(f, 40, 15, 25, 13, 26)
    idle_arms(f, (-1, 1))
    return f


def h0() -> Frame:
    return from_key(H0, tuple(LAYERS))


def d1() -> Frame:
    f = upper(Frame(), (-4, 15), "v0", (0, 1))
    f.add("boxers", parse("KUUUUUUUUUUUUUUUK", 12, 54))
    f.add("boxers", parse("KKKKKKKKKKKKKKK", 13, 55))
    # far leg, knee up
    f.add("legs", leg_bent((28, 50), (32, 45), (34, 49), 2.6, 2.4, 2.0))
    croc_flat(f, 30)
    # near leg straight forward on the floor, toe-up Croc ending by x45
    f.add("legs", leg((26, 52), (37, 53), 2.0, 2.0))
    f.add("shoes", CROC_HEEL, 35, 50)
    # near hand propped behind on the floor
    sleeve_arm(f, "near_arm", [(14, 37), (10, 46)], 2.0)
    f.add("near_arm", leg((10, 46), (8, 51), 1.5, 1.5))
    f.add("near_arm", FIST_SMALL, 5, 52)
    # far hand resting on the far knee
    sleeve_arm(f, "near_arm", [(26, 36), (31, 42)], 2.0)
    f.add("near_arm", FIST_SMALL, 30, 43)
    return f


def g0() -> Frame:
    f = upper(Frame(), (-1, 8), "v3")
    hem(f, 17, 47, HEM_BACK)
    hem(f, 25, 47, HEM_FRONT)
    # near leg kneeling: knee on the floor, shin back along the floor, toe on the floor
    f.add("legs", leg((20, 48), (25, 52), 3.0, 2.5))
    f.add("legs", leg((24, 53), (14, 53), 1.5, 1.5))
    croc_toe_down(f, 8)
    # far foot planted forward, knee up
    f.add("legs", leg_bent((29, 48), (31, 44), (31, 49), 3.0, 2.6, 2.0))
    croc_flat(f, 27)
    sleeve_arm(f, "near_arm", [(17, 30), (27, 39)], 2.3)
    f.add("near_arm", FIST_SMALL, 28, 41)
    return f


def g1() -> Frame:
    f = upper(Frame(), (1, 4), "v3")
    hem(f, 17, 43, HEM_BACK)
    hem(f, 27, 43, HEM_FRONT)
    f.add("legs", leg((20, 44), (15, 48), 3.0, 2.0))
    croc_toe_down(f, 12)
    f.add("legs", leg_bent((31, 44), (33, 46), (31, 49), 3.0, 2.6, 2.0))
    croc_flat(f, 27)
    sleeve_arm(f, "near_arm", [(18, 26), (28, 36)], 2.3)
    f.add("near_arm", FIST_SMALL, 30, 41)
    return f


def g2() -> Frame:
    f = upper(Frame(), (1, 1), "v1")
    standing_legs(f, 40, 15, 25, 14, 27)
    idle_arms(f, (-1, 1), (2, 1))
    return f


ACCENTS = {e["key"] for e in PALETTE if e.get("accent")}
NEIGHBOURS = ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1))


def cleanup(f: Frame, passes: int = 4) -> Frame:
    """Merge orphan pixels of hand-shaped parts into the neighbouring cluster (master pixels stay)."""
    for _ in range(passes):
        flat: Pix = {}
        top: dict = {}
        for l in LAYERS:
            for k, c in f.L[l].items():
                flat[k] = c
                top[k] = l
        changed = False
        for (x, y), c in sorted(flat.items()):
            if c in ACCENTS or (top[(x, y)], x, y) in f.locked:
                continue
            around = [flat[(x + dx, y + dy)] for dx, dy in NEIGHBOURS if (x + dx, y + dy) in flat]
            if not around:
                del f.L[top[(x, y)]][(x, y)]
                changed = True
            elif c not in around:
                best = max(sorted(set(around)), key=around.count)
                f.L[top[(x, y)]][(x, y)] = best
                changed = True
        if not changed:
            break
    return f


def occlude_pin(f: Frame) -> Frame:
    """The pin sits on the jacket: an arm drawn across it hides it."""
    for k in [k for k in f.L["pin"] if k in f.L["near_arm"]]:
        del f.L["pin"][k]
    return f


FRAMES = {"a0": a0, "a1": a1, "a2": a2, "a3": a3, "b2": b2, "b3": b3,
          "k0": k0, "k1": k1, "k2": k2, "k3": k3, "k4": k4, "k5": k5, "k6": k6,
          "h0": h0, "h1": h1, "h2": h2, "d1": d1, "g0": g0, "g1": g1, "g2": g2}

# ------------------------------------------------------------------ atlas layout and build

MANIFEST_PATH = ROOT / "assets/manifests/carl_book1_h52_melee.json"
ORDER = ["a0", "a1", "a2", "a3", "b0", "b1", "b2", "b3", "k0", "k1", "k2", "k3", "k4", "k5", "k6",
         "h0", "h1", "h2", "kb", "d0", "d1", "g0", "g1", "g2"]
TAGS = [("punch_a", "a0", "a3"), ("punch_b", "b0", "b3"), ("kick", "k0", "k6"), ("hurt", "h0", "h2"),
        ("knockback", "kb", "kb"), ("knockdown", "d0", "d1"), ("getup", "g0", "g2")]
KEY_FRAMES = {"a1": "a1", "b0": "b0", "b1": "b1", "k1": "k1", "k3": "k3", "h0": "h0", "kb": "kb", "d0": "d0"}
KB_HOLD_MS = 100  # knockback hold frame: preview only, the real hold lasts while airborne


def tick_ranges() -> dict[str, tuple[int, int] | None]:
    """Frame id -> inclusive tick range, from the manifest ``melee.moves`` tick contract."""
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    out: dict[str, tuple[int, int] | None] = {}
    for move in manifest["melee"]["moves"].values():
        for fr in move["frames"]:
            out[fr["name"].split()[0]] = tuple(fr["ticks"]) if fr["ticks"] else None
    return out


def duration_ms(ticks: tuple[int, int] | None) -> int:
    return KB_HOLD_MS if ticks is None else round((ticks[1] - ticks[0] + 1) * 1000 / TICK_HZ)


def frame_cels(fid: str) -> dict[str, list[str]]:
    if fid in KEY_FRAMES:  # Opus keys, verbatim
        return {l: KEYFILE["keys"][KEY_FRAMES[fid]]["cels"][l] for l in LAYERS}
    f = cleanup(occlude_pin(FRAMES[fid]()))
    return {l: rows_of(f.L[l]) for l in LAYERS}


def build() -> dict:
    ticks = tick_ranges()
    frames = [{"duration_ms": duration_ms(ticks[fid]), "cels": frame_cels(fid)} for fid in ORDER]
    index = {fid: i for i, fid in enumerate(ORDER)}
    return {
        "format": "pixel-grid-v1", "canvas": [W, H], "pivot": [25, 55],
        "palette": PALETTE, "layers": LAYERS, "frames": frames,
        "tags": [{"name": n, "from": index[a], "to": index[b], "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    """Diff-friendly layout: one pixel row per line."""
    lines = ["{", f' "format": "{grid["format"]}",', f' "canvas": {json.dumps(grid["canvas"])},',
             f' "pivot": {json.dumps(grid["pivot"])},', ' "palette": [']
    pal = grid["palette"]
    lines += [f"  {json.dumps(e)}" + ("," if i < len(pal) - 1 else "") for i, e in enumerate(pal)]
    lines += [" ],", f' "layers": {json.dumps(grid["layers"])},', ' "frames": [']
    for fi, fr in enumerate(grid["frames"]):
        lines.append(f'  {{"duration_ms": {fr["duration_ms"]}, "cels": {{')
        names = [n for n in grid["layers"] if n in fr["cels"]]
        for li, name in enumerate(names):
            rows = fr["cels"][name]
            lines.append(f'   "{name}": [')
            lines += [f'    "{r}"' + ("," if ri < len(rows) - 1 else "") for ri, r in enumerate(rows)]
            lines.append("   ]" + ("," if li < len(names) - 1 else ""))
        lines.append("  }}" + ("," if fi < len(grid["frames"]) - 1 else ""))
    tags = grid["tags"]
    lines += [" ],", ' "tags": [']
    lines += [f"  {json.dumps(t)}" + ("," if i < len(tags) - 1 else "") for i, t in enumerate(tags)]
    lines += [" ]", "}"]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=OUT_PATH, help="grid to write (default: the committed source)")
    parser.add_argument("--check", action="store_true", help="exit 1 if the committed grid differs from the build")
    args = parser.parse_args(argv)
    text = dump(build())
    if args.check:
        same = args.out.is_file() and args.out.read_text(encoding="utf-8") == text
        print("carl melee grid matches the build" if same else "carl melee grid DIFFERS from the build")
        return 0 if same else 1
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({len(ORDER)} frames)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
