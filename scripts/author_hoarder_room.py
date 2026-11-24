#!/usr/bin/env python3
"""Author the AREA-HOARDER-01 room kit (HOARD-02 props) as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/hoarder-hoard-02.md`` section 7. The design stage authored one hero key, the lair mound
in its ``sealed`` state (two draw layers: ``back`` drawn behind the boss, ``front`` drawn in front of her while she
is Dormant or emerging). The execution tier added the `opened` mound, bank A, the J-B post and stack, the door,
the entry gate and the dressing heaps on the same material rules. One atlas per piece, one frame per state (the
editor flattens layers, so the mound's two draw layers are two frames). Colours come only from the
floor01-catacomb-v2 ramps (``author_floor01_kit.RAMPS``); no new colours, no raster image-model output, no tracing.

    python3 scripts/author_hoarder_room.py            # write every piece: grid + .ase + manifest
    python3 scripts/author_hoarder_room.py --check    # exit 1 when a committed grid or manifest differs
    python3 scripts/author_hoarder_room.py --preview PNG   # every state of every piece, x3
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402

RAMPS = kit.RAMPS
OUT = ROOT / "assets/source/props/floor01"
W, H = 176, 72
PIVOT = (88, 71)          # centre of the floor contact; placed at the boss home x 608
LIGHT = (-0.55, -0.65, 0.52)   # torch key upper left, as on the boss and the ENV-01 kit
LIP_Y = 40                # front layer: everything at or below this row in the mound's near half


def rnd(*key) -> float:
    return int(hashlib.sha256(repr(key).encode()).hexdigest()[:8], 16) / 0xFFFFFFFF


def lum(nx, ny) -> float:
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    ln = math.sqrt(sum(v * v for v in LIGHT))
    return max(0.0, (nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]) / ln)


# Material bands: (ramp, darkest index, brightest index). The hoard is kept, not rotting: wood, iron, copper,
# brass, cold glass and grey rag; a few lichen flecks only on the floor-side underlayer.
MAT = {"rag": ("floor", 1, 5), "wood": ("wood", 0, 5), "iron": ("iron", 0, 5), "copper": ("copper", 1, 6),
       "brass": ("brass", 0, 3), "glass": ("gal", 3, 9), "stone": ("stone", 1, 6), "lichen": ("lichen", 0, 2)}
OUTLINE = ("mortar", 0)


def shade(mat: str, v: float, x: int, y: int) -> tuple[str, int]:
    ramp, lo, hi = MAT[mat]
    v = max(0.0, min(1.0, v + (rnd(mat, x, y) - 0.5) * 0.12))
    return ramp, lo + round(v * (hi - lo))


class Layer:
    def __init__(self, w: int = W, h: int = H) -> None:
        self.w, self.h = w, h
        self.px = [[None] * w for _ in range(h)]
        self.mat = [[None] * w for _ in range(h)]

    def put(self, x: int, y: int, ramp: str, idx: int) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = (ramp, max(0, min(len(RAMPS[ramp]) - 1, int(idx))))

    def rect(self, x0: int, y0: int, x1: int, y1: int, ramp: str, idx: int) -> None:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, ramp, idx)

    def fill(self, pts, mat, normal, gain=1.0, bias=0.0) -> None:
        for (x, y) in pts:
            if 0 <= x < self.w and 0 <= y < self.h:
                n = normal(x, y)
                self.px[y][x] = shade(mat, bias + gain * lum(*n), x, y)
                self.mat[y][x] = mat


def ellipse(cx, cy, rx, ry):
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1:
                yield x, y


def box(x0, y0, w, h, ang=0.0):
    ca, sa = math.cos(math.radians(ang)), math.sin(math.radians(ang))
    cx, cy = x0 + w / 2, y0 + h / 2
    r = int(max(w, h)) + 2
    for y in range(int(cy) - r, int(cy) + r):
        for x in range(int(cx) - r, int(cx) + r):
            lx = (x - cx) * ca + (y - cy) * sa
            ly = -(x - cx) * sa + (y - cy) * ca
            if abs(lx) <= w / 2 and abs(ly) <= h / 2:
                yield x, y


def capsule(x0, y0, x1, y1, r):
    dx, dy = x1 - x0, y1 - y0
    l2 = dx * dx + dy * dy or 1.0
    for y in range(int(min(y0, y1) - r) - 1, int(max(y0, y1) + r) + 2):
        for x in range(int(min(x0, x1) - r) - 1, int(max(x0, x1) + r) + 2):
            t = max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
            if math.hypot(x - x0 - t * dx, y - y0 - t * dy) <= r:
                yield x, y


def dome_n(cx, cy, rx, ry):
    return lambda x, y: (max(-1, min(1, (x - cx) / rx)) * 0.9, max(-1, min(1, (y - cy) / ry)) * 0.9)


FLAT_UP = lambda x, y: (-0.2, -0.75)          # noqa: E731  lit top faces
FLAT_SIDE = lambda x, y: (0.35, 0.2)          # noqa: E731  shadowed front faces

# Back heap items, far to near: (kind, geometry, material). Hand-placed, read left to right at 1x.
BACK_ITEMS = [
    ("dome", (92, 76, 84, 64), "rag"),                   # the rag-and-cloth body that binds the heap
    ("plank", (22, 40, 44, 6, -28), "wood"),             # a door plank leaning out of the left flank
    ("plank", (96, 20, 40, 5, 16), "wood"),              # a broken bed rail across the crown
    ("crate", (50, 28, 26, 20, -8), "wood"),             # a crate, lid gone
    ("pot", (126, 36, 13, 10), "iron"),                  # an iron cooking pot on its side
    ("pipe", (66, 10, 90, 30, 2.5), "copper"),           # a bent copper pipe, the one thing that sticks up
    ("frame", (84, 24, 16, 20, 6), "brass"),             # an empty brass mirror frame
    ("pot", (40, 54, 9, 7), "copper"),                   # a dented copper kettle
    ("bottle", (140, 50, 4, 9), "glass"),                # cold glass bottles at the right foot
    ("bottle", (149, 56, 4, 8), "glass"),
    ("plank", (144, 34, 28, 5, 52), "wood"),             # a chair leg
    ("drape", (60, 44, 22, 8), "rag"),                   # rag folds that bury the lower half of the items
    ("drape", (112, 48, 26, 9), "rag"),
    ("drape", (92, 40, 14, 6), "rag"),
]
POKES = {"pipe"}          # only these may break the heap silhouette

# Front lip: the near ridge that hides her below row LIP_Y while Dormant and through the emerge
FRONT_ITEMS = [
    ("ridge", (88, 74, 76, 30), "rag"),
    ("crate", (30, 48, 24, 16, 10), "wood"),
    ("pot", (112, 56, 12, 8), "iron"),
    ("plank", (66, 50, 44, 5, -6), "wood"),
    ("pipe", (126, 46, 160, 60, 2.0), "copper"),
    ("bottle", (96, 58, 4, 8), "glass"),
    ("stone", (150, 62, 10, 7), "stone"),
    ("stone", (16, 64, 9, 6), "stone"),
]


def draw(layer: Layer, items) -> None:
    W, H = layer.w, layer.h
    mask = None
    for kind, g, mat in items:
        if mask is not None and kind not in POKES:
            before = [row[:] for row in layer.px], [row[:] for row in layer.mat]
        else:
            before = None
        if kind in ("dome", "ridge"):
            cx, cy, rx, ry = g
            pts = [(x, y) for x, y in ellipse(cx, cy, rx, ry) if y < H]
            # a lumpy top edge: rag folds every few pixels
            pts = [(x, y) for x, y in pts if y >= cy - ry * math.sqrt(max(0, 1 - ((x - cx) / rx) ** 2))
                   + 2 + 2 * math.sin(x * 0.45) + 3 * rnd(kind, x // 5)]
            layer.fill(pts, mat, dome_n(cx, cy, rx, ry), gain=0.85, bias=0.05)
            # fold shadows: short dark diagonals
            for i in range(0, 2 * rx, 9):
                x = cx - rx + i + int(rnd(kind, i) * 5)
                for k in range(5):
                    y = int(cy - ry * 0.55 + rnd(kind, i, 1) * ry * 0.7) + k
                    if 0 <= y < H and 0 <= x + k < W and layer.mat[y][x + k] == mat:
                        layer.px[y][x + k] = (MAT[mat][0], MAT[mat][1])
        elif kind in ("plank", "crate"):
            x0, y0, w, h, ang = g
            pts = list(box(x0, y0, w, h, ang))
            top = min(y for _, y in pts)
            layer.fill(pts, mat, lambda x, y, t=top: FLAT_UP(x, y) if y - t < 3 else FLAT_SIDE(x, y), bias=0.1)
            if kind == "crate":   # board seams and the open top
                for x, y in pts:
                    if (x - x0) % 7 == 0 or y - top in (1,):
                        layer.px[y][x] = (MAT[mat][0], MAT[mat][1])
        elif kind == "pot":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)), mat, dome_n(cx, cy, rx, ry), gain=1.0)
            for x, y in ellipse(cx + rx * 0.35, cy, rx * 0.45, ry * 0.6):   # the dark open mouth
                if 0 <= y < H and 0 <= x < W:
                    layer.px[y][x] = (MAT[mat][0], MAT[mat][1])
        elif kind == "pipe":
            x0, y0, x1, y1, r = g
            pts = list(capsule(x0, y0, x1, y1, r))
            layer.fill(pts, mat, lambda x, y: (0.0, -0.8 if (y - y0 - (x - x0) * (y1 - y0) / max(1, x1 - x0)) < 0
                                               else 0.5), bias=0.1)
        elif kind == "frame":
            x0, y0, w, h, ang = g
            outer = set(box(x0, y0, w, h, ang))
            inner = set(box(x0 + 3, y0 + 3, w - 6, h - 6, ang))
            layer.fill(sorted(outer - inner), mat, lambda x, y: (-0.3, -0.5), bias=0.15)
            for x, y in inner:   # black void inside the frame: no reflection, no face
                if 0 <= y < H and 0 <= x < W:
                    layer.px[y][x] = ("gal", 1)
        elif kind == "bottle":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)) + list(box(cx - 1, cy - ry - 4, 2, 4)), mat,
                       dome_n(cx, cy, rx, ry), bias=0.1)
        elif kind == "stone":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)), mat, dome_n(cx, cy, rx, ry))
        elif kind == "drape":
            cx, cy, rx, ry = g
            pts = [(x, y) for x, y in ellipse(cx, cy, rx, ry) if y >= cy - ry * 0.2 + 2 * math.sin(x * 0.7)]
            layer.fill(pts, mat, dome_n(cx, cy - ry, rx, ry * 2), gain=0.7, bias=0.25)
        if kind in ("dome", "ridge") and mask is None:
            mask = {(x, y) for y in range(H) for x in range(W) if layer.px[y][x] is not None}
        elif before is not None:   # clip the item to the heap body grown by 2 px
            for y in range(H):
                for x in range(W):
                    if layer.px[y][x] != before[0][y][x] and not any(
                            (x + dx, y + dy) in mask for dx in (-2, 0, 2) for dy in (-2, 0, 2)):
                        layer.px[y][x], layer.mat[y][x] = before[0][y][x], before[1][y][x]


def outline(layer: Layer) -> None:
    W, H = layer.w, layer.h
    edge = []
    for y in range(H):
        for x in range(W):
            if layer.px[y][x] is None:
                continue
            if any(not (0 <= x + dx < W and 0 <= y + dy < H) or layer.px[y + dy][x + dx] is None
                   for dx, dy in ((1, 0), (-1, 0), (0, -1))):
                edge.append((x, y))
    for x, y in edge:
        layer.px[y][x] = OUTLINE


# --------------------------------------------------------------------------- the other pieces (execution)
# Every piece is one atlas: one pixel-grid-v1 with one layer ("main"), one frame per state (the editor flattens
# layers, so a state that needs two draw layers is two frames), pivot bottom-left unless noted.
CLEAR = None


def ci(v, lo, hi) -> int:
    return max(lo, min(hi, int(round(v))))


def boards(p: Layer, x0, y0, x1, y1, seed, base=3.0, slat=5, lit=1.0, grain=0.7) -> None:
    """Horizontal timber slats on the ``wood`` ramp: a lit top edge, a dark seam under each, a light grain."""
    y = y0
    while y <= y1:
        end = min(y + slat - 1, y1)
        for yy in range(y, end + 1):
            for x in range(x0, x1 + 1):
                v = base + (rnd(seed, x // 3, yy // 2) - 0.5) * grain * 2
                if yy == y:
                    v += lit
                if yy == end and end > y:
                    v -= 1.6
                p.put(x, yy, "wood", ci(v, 0, 5))
        y = end + 1


def crate(p: Layer, x0, y0, x1, y1, seed, brace=False, base=3.0) -> None:
    """A crate side: framed by darker posts, slatted inside, the left post lit, the right post in shade."""
    boards(p, x0, y0, x1, y1, seed, base=base)
    for y in range(y0, y1 + 1):
        p.put(x0, y, "wood", 4 if y > y0 else 5)
        p.put(x1, y, "wood", 1)
    for x in range(x0, x1 + 1):
        p.put(x, y1, "wood", 0)
    if brace:
        n = max(x1 - x0 - 3, 1)
        for i in range(n):
            x = x0 + 2 + i
            y = y1 - 2 - round(i * (y1 - y0 - 4) / n)
            p.put(x, y, "wood", 5)
            p.put(x, y + 1, "wood", 1)


def vplank(p: Layer, x0, y0, x1, y1, seed, base=4.0) -> None:
    """A standing plank: 3 px vertical boards, the left edge lit."""
    for x in range(x0, x1 + 1):
        for y in range(y0, y1 + 1):
            v = base + (rnd(seed, x // 3, y // 3) - 0.5) * 1.2
            if (x - x0) % 3 == 0:
                v += 0.8
            if (x - x0) % 3 == 2:
                v -= 1.4
            p.put(x, y, "wood", ci(v, 0, 5))
    for y in range(y0, y1 + 1):
        p.put(x0, y, "wood", 5)
        p.put(x1, y, "wood", 1)


def strap(p: Layer, x0, y, x1, h=3, seed="strap") -> None:
    for yy in range(y, y + h):
        for x in range(x0, x1 + 1):
            p.put(x, yy, "iron", 4 if yy == y else 2 if yy == y + h - 1 else 3)
    for x in range(x0 + 1, x1, 4):
        p.put(x, y + h // 2, "iron", 6)      # nail heads


def rope(p: Layer, x0, y0, x1, y1, seed="rope") -> None:
    """A rope on the ``floor`` ramp (rag fibre): lit strand and a shadow pixel under it."""
    n = max(abs(x1 - x0), abs(y1 - y0), 1)
    for i in range(n + 1):
        x, y = round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)
        p.put(x, y + 1, "floor", 2)
        p.put(x, y, "floor", 8 if i % 3 else 6)


def solid_edges(p: Layer, x0, y0, x1, y1, top_ramp=("wood", 5)) -> None:
    """R6, a solid's read: one unbroken lit top row, a dark lip under it, vertical outlined sides, a dark foot."""
    for x in range(x0, x1 + 1):
        p.put(x, y0, *top_ramp)
        p.put(x, y0 + 1, "wood", 1)
        p.put(x, y1, "wood", 0)
    for y in range(y0 + 1, y1 + 1):
        p.put(x0, y, "mortar", 0)
        p.put(x1, y, "mortar", 0)


def chips(p: Layer, pts, seed) -> None:
    """Falling scraps: a lit chip with a shadow pixel under it."""
    for i, (x, y) in enumerate(pts):
        p.put(x, y, "wood", 5 if i % 2 else 4)
        p.put(x + 1, y, "wood", 3)
        p.put(x, y + 1, "wood", 1)


def dust(p: Layer, pts) -> None:
    for i, (x, y) in enumerate(pts):
        p.put(x, y, "floor", 7 if i % 2 else 5)


BANK_W, BANK_H = 64, 48
BANK_CANVAS = (80, 48)


def bank_body(p: Layer, phase=None) -> None:
    """Bank A standing: 64 x 48 at the canvas origin. Crate sides and a standing door plank form a vertical wall;
    the top row (y 0) is one unbroken lit row across all 64 px (the standing line), nothing pokes above it."""
    seed = "bank"
    tiers = [
        [(0, 2, 23, 15, False), (24, 2, 45, 15, False), (46, 2, 63, 15, True)],
        [(0, 16, 17, 31, True), (18, 16, 40, 31, False), (41, 16, 63, 31, False)],
        [(0, 32, 27, 47, False), (28, 32, 47, 47, True), (48, 32, 63, 47, False)],
    ]
    for t, row in enumerate(tiers):
        for n, (x0, y0, x1, y1, brace) in enumerate(row):
            crate(p, x0, y0, x1, y1, (seed, t, n), brace=brace, base=3.0 - 0.3 * t)
    vplank(p, 49, 3, 58, 47, (seed, "door"), base=4.0)           # the door plank standing in the wall
    for y in (10, 38):
        strap(p, 49, y, 58)
    p.rect(21, 24, 29, 28, "floor", 3)                           # rag stuffed in a crate gap, kept inside
    for x in range(21, 30):
        p.put(x, 24, "floor", 4)
    for x, y in ((24, 26), (27, 25)):
        p.put(x, y, "floor", 2)
    p.rect(8, 38, 13, 40, "brass", 1)                            # a brass handle plate
    p.rect(9, 38, 12, 38, "brass", 3)
    rope(p, 2, 20, 16, 29)                                       # a rope end tied through the brace
    if phase is not None:                                        # tumble: a gap opens and dust trails the lower edge
        gx = 24 + 8 * phase
        for i in range(12):
            p.put(gx + i // 2 + 3 * (phase == 0), 18 + i, "wood", 0)
        dust(p, [(2 + 5 * k, 41 + (k + phase) % 3 * 2) for k in range(6)])
    solid_edges(p, 0, 0, 63, 47)


def remnant(p: Layer, w, h, y0, seed, wheel=False) -> None:
    """A low solid remnant ``h`` px tall from row ``y0``: flat lit top across the full width, tumbled boards inside."""
    for x0 in range(0, w, 27):
        crate(p, x0, y0 + 2, min(x0 + 26, w - 1), y0 + h - 1, (seed, x0), brace=x0 // 27 % 2 == 1, base=2.6)
    for i, (x, ang) in enumerate(((10, 22), (38, -18), (w - 22, 14))):
        for px, py in box(x, y0 + 5, 22, 3, ang):
            if 1 <= px < w - 1 and y0 + 2 <= py < y0 + h - 1:
                p.put(px, py, "wood", 4 if (px + i) % 5 else 2)
    if wheel:
        for px, py in ellipse(w - 30, y0 + h - 6, 6, 5):
            r = math.hypot(px - (w - 30), (py - (y0 + h - 6)) * 1.1)
            if 1 <= px < w - 1 and 3.5 <= r <= 6:
                p.put(px, py, "iron", 5 if px < w - 30 else 3)
    solid_edges(p, 0, y0, w - 1, y0 + h - 1)


def bank_sheet() -> list[Layer]:
    out = []
    for kind in ("stable", "disturbed_a", "disturbed_b", "slid_a", "slid_b", "settled"):
        p = Layer(*BANK_CANVAS)
        if kind == "settled":
            remnant(p, 80, 16, BANK_CANVAS[1] - 16, "bank_rem")
        else:
            bank_body(p, phase={"slid_a": 0, "slid_b": 1}.get(kind))
            if kind == "disturbed_a":
                chips(p, [(12, 9), (36, 20), (53, 5), (30, 34)], kind)
            if kind == "disturbed_b":
                chips(p, [(13, 15), (37, 28), (54, 12), (31, 41)], kind)
        out.append(p)
    return out


BANK_FRAMES = ["stable", "disturbed_a", "disturbed_b", "slid_a", "slid_b", "settled"]
BANK_TAGS = [("stable", 0, 0), ("disturbed", 1, 2), ("slid", 3, 4), ("settled", 5, 5)]

POST_CANVAS = (16, 64)


def post(state: str) -> Layer:
    """J-B post: one thin split timber (10 px wide) on the back plane, foot on a stone shim, the top lashed under the
    stack's beam. No flat lit top and no shelf. The lane-side face carries the pale fresh split (the hittable cue)."""
    p = Layer(*POST_CANVAS)
    top = 0 if state != "broken" else 34
    for y in range(top, 60):
        for x in range(3, 13):
            v = 3.2 + (rnd("post", x, y // 2) - 0.5) * 1.1
            if x == 3:
                v += 1.5
            if x >= 11:
                v -= 1.6
            p.put(x, y, "wood", ci(v, 1, 5))
    for y in range(top, 60):
        p.put(2, y, "mortar", 0)
        p.put(13, y, "mortar", 0)
    if state == "broken":                       # a jagged snapped stump, splinters standing out of the break
        for x in range(2, 14):
            cut = top + (2, 0, 3, 1, 4, 0, 2, 5, 1, 3, 0, 2)[x - 2]
            for y in range(top, cut):
                p.px[y][x] = None
        for x, y in ((3, 31), (5, 29), (8, 30), (10, 28), (12, 31)):
            p.put(x, y, "wood", 5)
            p.put(x, y + 1, "wood", 3)
    else:
        for x in range(2, 14):                  # a ragged lashed top (no flat lit line)
            cut = (3, 1, 4, 0, 2, 5, 1, 3, 0, 4, 2, 1)[x - 2]
            for y in range(0, cut):
                p.px[y][x] = None
        for k, y in enumerate(range(1, 10, 2)):  # rope lashing: diagonal wraps
            for x in range(2, 14):
                p.put(x, y + (x // 4) % 2, "floor", 8 if (x + k) % 3 else 6)
                p.put(x, y + 1 + (x // 4) % 2, "floor", 3)
        # the fresh split on the lane-side face
        top_s, bot_s = (22, 44) if state == "intact" else (14, 52)
        for y in range(top_s, bot_s):
            w = 1 if state == "intact" else 2 + (y // 3) % 2
            x = 7 - (y // 6) % 2
            for k in range(w):
                p.put(x + k, y, "wood", 5)
            p.put(x - 1, y, "wood", 0)
            p.put(x + w, y, "wood", 1)
        if state == "cracked":                  # splinters out of the split, a pale chip on the floor side
            for x, y in ((1, 24), (12, 30), (2, 40), (13, 46)):
                p.put(x, y, "wood", 5)
                p.put(x + (1 if x < 8 else -1), y + 1, "wood", 3)
    for x in range(1, 15):                      # the stone shim the foot rests on
        for y in range(60, 64):
            p.put(x, y, "stone", 3 + (1 if y == 60 else 0) + (x + y) % 2 - (1 if y == 63 else 0))
    for x in range(1, 15):
        p.put(x, 63, "mortar", 0)
    return p


POST_FRAMES = ["intact", "cracked", "broken"]
POST_TAGS = [("intact", 0, 0), ("cracked", 1, 1), ("broken", 2, 2)]

STACK_CANVAS = (128, 104)
STACK_OY = 8            # rows 0..7 hold motion streaks and dust; the hanging body is rows 8..71
STACK_LEG = 32          # rows 72..103: the strut that carries the beam down to the post's head (the sim's post top
                        # is 32 px under the stack's footprint, so the stack carries its own leg; spec section 7)
STACK_FALL_LEG = 20     # HOARD-02 polish: the strut once the post has broken (snapped). The body's bottom is 23 px over
                        # the floor on the last falling tick (drop 96 * 7^2 / 8^2 = 73), so a 20 px stub stays above it


def stack_body(p: Layer, leg: int = STACK_LEG) -> None:
    """J-B stack, hanging: a beam lashed to the post (the only support), a broken shelf unit, crates and a pram
    frame, roped. The body is 128 x 64 at rows STACK_OY..STACK_OY + 63. ``leg`` is the strut's length under the beam:
    the full STACK_LEG down to the post's head while it hangs and warns, STACK_FALL_LEG (snapped, a splintered end)
    while it falls, so the strut never dips under the floor line on the last falling tick."""
    oy = STACK_OY
    seed = "stack"
    # the lashed beam: the stack's only support, directly above the post at x 56..71
    for y in range(oy + 56, oy + 64):
        for x in range(0, 128):
            v = 3.2 + (rnd(seed, "beam", x // 4, y) - 0.5) * 0.9 + (0.9 if y == oy + 56 else 0) - (1.4 if y == oy + 63 else 0)
            p.put(x, y, "wood", ci(v, 0, 5))
    # broken shelf unit (left): uprights, three shelves, the middle one dropped at the right end
    for y in range(oy + 4, oy + 56):
        for x in (3, 4, 49, 50):
            p.put(x, y, "wood", 2 if x in (4, 50) else 4)
    for sy in (4, 24, 44):
        for x in range(3, 51):
            drop = round((x - 3) * 0.08) if sy == 24 else 0
            for t in range(3):
                p.put(x, oy + sy + drop + t, "wood", (5 if t == 0 else 3 if t == 1 else 1))
    crate(p, 8, oy + 9, 22, oy + 23, (seed, 1), brace=True)
    crate(p, 8, oy + 29, 30, oy + 43, (seed, 2))
    crate(p, 33, oy + 49, 47, oy + 55, (seed, 3))
    for px, py in ellipse(34, oy + 18, 6, 5):              # an iron pot on the top shelf
        p.put(px, py, "iron", 5 if px + py < 52 else 2 if px + py > 56 else 3)
    for x in range(30, 38):
        p.put(x, oy + 14, "iron", 6)
    for bx in (42, 45):                                    # cold bottles
        for y in range(oy + 11, oy + 24):
            p.put(bx, y, "gal", 8 if y > oy + 13 else 6)
            p.put(bx + 1, y, "gal", 4)
    for px, py in ellipse(40, oy + 37, 5, 6):              # a dented copper kettle
        p.put(px, py, "copper", 5 if px + py < 72 else 2 if px + py > 78 else 3)
    # crates (middle): two stacked crates tied together
    crate(p, 52, oy + 30, 91, oy + 55, (seed, 4), brace=True)
    crate(p, 58, oy + 6, 88, oy + 29, (seed, 5))
    strap(p, 52, oy + 40, 91)
    # pram frame (right): hood arc, two iron wheels and a handle bar
    for px, py in ellipse(111, oy + 36, 15, 13):
        if py <= oy + 40 and py >= oy + 24:
            r = math.hypot((px - 111) / 15, (py - oy - 36) / 13)
            if r >= 0.72:
                p.put(px, py, "iron", 5 if px < 108 else 3)
            elif (px + py) % 3 == 0:
                p.put(px, py, "floor", 4)
    for cx in (101, 121):
        for px, py in ellipse(cx, oy + 48, 7, 7):
            r = math.hypot(px - cx, py - oy - 48)
            if r >= 4.5:
                p.put(px, py, "iron", 5 if px < cx else 3)
            elif r < 1.5:
                p.put(px, py, "iron", 6)
    for x in range(96, 126):
        p.put(x, oy + 41, "iron", 4)
    for i in range(14):
        p.put(94 - i // 2, oy + 41 - i, "iron", 4)           # the handle bar
    # ropes: two diagonals crossing the whole stack, lashed to the beam
    rope(p, 2, oy + 14, 126, oy + 52)
    rope(p, 126, oy + 10, 2, oy + 50)
    for k in range(3):                                        # the lashing around the post's head
        rope(p, 55, oy + 56 + k * 2, 72, oy + 57 + k * 2)
    for x in range(0, 128):
        p.put(x, oy + 63, "wood", 0)
    # the leg: a split timber strut under the beam, lashed to it, ending on the post's head (x 56..71)
    for y in range(oy + 64, oy + 64 + leg):
        for x in range(57, 71):
            v = 3.0 + (rnd(seed, "leg", x, y // 2) - 0.5) * 1.0 + (1.2 if x == 57 else 0) - (1.4 if x >= 69 else 0)
            p.put(x, y, "wood", ci(v, 1, 5))
        p.put(56, y, "mortar", 0)
        p.put(71, y, "mortar", 0)
    if leg < STACK_LEG:                  # the snapped end: a jagged cut, pale fresh splinters (as the broken post)
        end = oy + 64 + leg
        for x in range(56, 72):
            cut = (4, 1, 3, 0, 2, 4, 1, 3, 0, 2, 4, 1, 3, 0, 2, 4)[x - 56]
            for y in range(end - cut, end):
                p.px[y][x] = None
            if 57 <= x <= 70 and cut < 4:
                p.put(x, end - cut - 1, "wood", 5 if x % 2 else 4)
    for k, y in enumerate(range(oy + 64, oy + 74, 2)):
        for x in range(56, 72):
            p.put(x, y + (x // 4) % 2, "floor", 8 if (x + k) % 3 else 6)
            p.put(x, y + 1 + (x // 4) % 2, "floor", 3)


def stack_rubble(p: Layer, y0: int) -> None:
    """J-B `landed` (HOARD-02 polish): the stack's own junk crushed into the 128 x 16 remnant band from row ``y0``.

    E4 keeps the solid's read: the beam lies flat on top (one unbroken lit row across all 128 px, a dark lip under
    it), both side columns are opaque and the foot is dark. Under the beam the stack stays recognisable instead of a
    plank slab: a crate caved in at one corner, the iron pot on its side, the dropped shelf board and the snapped
    strut, a burst crate, the copper kettle, and the pram's crushed hood with one round and one buckled wheel, all
    separated by dark voids and tied by the rope."""
    seed = "stack_rubble"
    h = 16
    for y in range(y0 + 2, y0 + h - 1):            # the voids between the crushed items
        for x in range(1, 127):
            p.put(x, y, "wood", 0 if rnd(seed, "void", x // 3, y) < 0.35 else 1)
    # 1. a crate caved in at its right corner: tilted slats, the dark break
    for px, py in box(2, y0 + 3, 25, 12, -6):
        if 1 <= px <= 28 and y0 + 2 <= py <= y0 + 14:
            ly = (py - (y0 + 3)) - (px - 2) * 0.1
            v = 3.0 + (rnd(seed, "c1", px // 3, py) - 0.5) * 0.8 + (0.9 if int(ly) % 4 == 0 else 0) - (1.3 if int(ly) % 4 == 3 else 0)
            p.put(px, py, "wood", ci(v, 1, 5))
    for i in range(10):                            # its brace, lit, and the left post
        p.put(4 + 2 * i, y0 + 13 - i, "wood", 5)
        p.put(4 + 2 * i + 1, y0 + 13 - i, "wood", 4)
    for py in range(y0 + 3, y0 + 15):
        p.put(2, py, "wood", 4)
    for k in range(6):                             # the caved corner: a dark wedge with two splinters
        for x in range(22 + k, 29):
            p.put(x, y0 + 2 + k, "wood", 0)
    p.put(24, y0 + 4, "wood", 5)
    p.put(25, y0 + 5, "wood", 4)
    # 2. the iron pot on its side, its dark mouth to the left
    for px, py in ellipse(36, y0 + 9, 6, 5):
        if y0 + 2 <= py <= y0 + 14:
            p.put(px, py, "iron", 5 if px + py < 36 + y0 + 6 else 2 if px + py > 36 + y0 + 12 else 4)
    for px, py in ellipse(33, y0 + 9, 2.5, 3.2):
        p.put(px, py, "iron", 0)
    p.put(41, y0 + 6, "iron", 6)
    # 3. the dropped shelf board, slanting, and the snapped strut lying under it
    for i in range(26):
        x, y = 42 + i, y0 + 13 - round(i * 0.36)
        p.put(x, y, "wood", 5)
        p.put(x, y + 1, "wood", 2)
    for y in range(y0 + 9, y0 + 15):
        for x in range(46, 52):
            p.put(x, y, "wood", 4 if x == 46 else 3 if x < 50 else 1)
    for x, y in ((46, y0 + 8), (48, y0 + 7), (50, y0 + 8)):
        p.put(x, y, "wood", 5)
    for x in (54, 57):                             # two cold bottles, one broken at the neck
        for y in range(y0 + 8, y0 + 14):
            p.put(x, y, "gal", 8 if y > y0 + 9 else 6)
            p.put(x + 1, y, "gal", 4)
    p.put(57, y0 + 7, "gal", 9)
    # 4. a burst crate: the front slats sprung open, one plank sticking out at an angle
    for px, py in box(64, y0 + 4, 22, 10, 7):
        if y0 + 2 <= py <= y0 + 14 and 60 <= px <= 90:
            gap = (px - 64) % 7 in (3,) or (px > 78 and py < y0 + 8)
            if gap:
                p.put(px, py, "wood", 0)
            else:
                v = 3.0 + (rnd(seed, "c2", px // 3, py) - 0.5) * 0.8 + (0.8 if (py - y0) % 4 == 0 else 0)
                p.put(px, py, "wood", ci(v, 1, 5))
    for i in range(12):                            # the sprung plank
        p.put(70 + i, y0 + 3 + i // 3, "wood", 5 if i % 3 else 4)
        p.put(70 + i, y0 + 4 + i // 3, "wood", 2)
    # 5. the dented copper kettle
    for px, py in ellipse(95, y0 + 10, 4.5, 3.8):
        p.put(px, py, "copper", 5 if px + py < 95 + y0 + 8 else 2 if px + py > 95 + y0 + 12 else 3)
    for i in range(3):
        p.put(90 - i, y0 + 9 - i, "copper", 4)
    p.put(97, y0 + 7, "copper", 1)                 # the dent
    # 6. the pram: the hood crushed to a low arc, one round wheel and one buckled, the handle bar
    for px, py in ellipse(112, y0 + 9, 13, 6):
        r = math.hypot((px - 112) / 13, (py - y0 - 9) / 6)
        if py <= y0 + 9 and r >= 0.7:
            p.put(px, py, "iron", 5 if px < 110 else 3)
        elif py <= y0 + 9 and (px + py) % 3 == 0:
            p.put(px, py, "floor", 4)
    for x in range(100, 126):
        p.put(x, y0 + 10, "iron", 4)
    for px, py in ellipse(106, y0 + 12, 3.5, 3.5):
        r = math.hypot(px - 106, py - y0 - 12)
        p.put(px, py, "iron", (5 if px < 106 else 3) if r >= 2.2 else 0)
    p.put(106, y0 + 12, "iron", 6)
    for px, py in ellipse(120, y0 + 13, 4.5, 2):   # the buckled wheel, squashed flat
        r = math.hypot((px - 120) / 4.5, (py - y0 - 13) / 2)
        if r >= 0.55:
            p.put(px, py, "iron", 4 if px < 120 else 2)
    for i in range(8):                             # the handle bar, bent down over the junk
        p.put(99 - i, y0 + 4 + i // 2, "iron", 4)
    # the rope that held it all, slack across the pile
    rope(p, 60, y0 + 12, 100, y0 + 6)
    solid_edges(p, 0, y0, 127, y0 + h - 1)


def stack_sheet() -> list[Layer]:
    out = []
    for kind in ("supported", "released", "falling", "landed"):
        p = Layer(*STACK_CANVAS)
        if kind == "landed":
            stack_rubble(p, STACK_CANVAS[1] - 16)
        else:
            stack_body(p, leg=STACK_FALL_LEG if kind == "falling" else STACK_LEG)
            if kind == "released":       # dust shaken out of the junk and loose scraps
                dust(p, [(5 + 9 * k, 6 - (k * 5) % 6) for k in range(14)])
                chips(p, [(30, 4), (84, 5), (112, 3)], kind)
            if kind == "falling":        # motion streaks above the body
                for k in range(18):
                    x = 4 + 7 * k + (k * 3) % 4
                    for y in range(0, 1 + (k * 5) % 7 + 1):
                        p.put(x, y, "floor", 6 if y % 2 else 4)
                chips(p, [(12, 3), (60, 2), (100, 4)], kind)
        out.append(p)
    return out


STACK_FRAMES = ["supported", "released", "falling", "landed"]
STACK_TAGS = [("supported", 0, 0), ("released", 1, 1), ("falling", 2, 2), ("landed", 3, 3)]

DOOR_CANVAS = (16, 228)


def jamb(p: Layer, x0, x1, h, seed) -> None:
    """Stone jamb courses (ENV-01 stone ramp, mortar joints every 12 px, the left jamb lit)."""
    for y in range(h):
        for x in range(x0, x1 + 1):
            v = 4.5 + (rnd(seed, x, y // 12) - 0.5) * 1.5 - (1.2 if x == x1 else 0)
            if y % 12 == 11:
                p.put(x, y, "mortar", 3)
            else:
                p.put(x, y, "stone", ci(v, 2, 7))
    for y in range(h):
        p.put(x0 if x0 == 0 else x1, y, "mortar", 0)


def door(state: str) -> Layer:
    p = Layer(*DOOR_CANVAS)
    h = DOOR_CANVAS[1]
    jamb(p, 0, 2, h, "door_l")
    jamb(p, 13, 15, h, "door_r")
    for x in range(0, 16):                        # lintel
        for y in range(0, 4):
            p.put(x, y, "stone", 7 if y == 0 else 4 - (y > 2))
    if state == "closed":                          # iron-banded planks, barred
        vplank(p, 3, 4, 12, h - 1, "door_p", base=3.2)
        for y in (30, 118, 200):
            strap(p, 3, y, 12, h=4)
        for y in range(110, 117):                  # the bar across the middle, in its brackets
            for x in range(1, 15):
                p.put(x, y, "iron", 5 if y == 110 else 2 if y == 116 else 3)
        for y in range(60, 180, 20):
            p.put(5, y, "iron", 6)
            p.put(10, y, "iron", 6)
        p.rect(7, 134, 9, 138, "iron", 3)          # lock plate
        p.put(8, 136, "iron", 6)
    else:                                          # open: the jamb only, a dark doorway
        for y in range(4, h):
            for x in range(3, 13):
                p.put(x, y, "mortar", 0 if y < h - 14 else 1)
        for y in range(h - 6, h):
            for x in range(3, 13):
                p.put(x, y, "stone", 2 + (x + y) % 2)
    for x in range(0, 16):
        p.put(x, h - 1, "mortar", 0)
    for cx, cy in ((0, 0), (15, 0)):                # chamfered lintel corners
        p.px[cy][cx] = None
    return p


DOOR_FRAMES = ["closed", "open"]
DOOR_TAGS = [("closed", 0, 0), ("open", 1, 1)]


def gate(state: str) -> Layer:
    p = Layer(*DOOR_CANVAS)
    h = DOOR_CANVAS[1]
    bars = (1, 6, 11)
    if state == "closed":                          # iron bars to the floor, two cross bands
        for bx in bars:
            for y in range(4, h):
                for x in range(bx, bx + 3):
                    p.put(x, y, "iron", 5 if x == bx else 2 if x == bx + 2 else 4)
            p.put(bx + 1, 3, "iron", 6)            # spike tips
            p.put(bx + 1, 2, "iron", 5)
        for y in (60, 150):
            for x in range(0, 16):
                p.put(x, y, "iron", 5)
                p.put(x, y + 1, "iron", 3)
                p.put(x, y + 2, "iron", 1)
        for x in range(0, 16):                    # floor socket
            p.put(x, h - 1, "stone", 2)
    else:                                          # open: the bar tips hang in the ceiling slot, grooves in the floor
        for bx in bars:
            for y in range(0, 20):
                for x in range(bx, bx + 3):
                    p.put(x, y, "iron", 5 if x == bx else 2 if x == bx + 2 else 4)
            p.put(bx + 1, 20, "iron", 5)
            p.put(bx + 1, 21, "iron", 3)
        for x in range(0, 16):
            p.put(x, 0, "mortar", 1)
        for y in range(0, h):                      # the side grooves the bars ride in
            p.put(0, y, "mortar", 1)
            p.put(15, y, "mortar", 1)
        for x in range(0, 16):                     # floor grooves
            p.put(x, h - 1, "mortar", 0)
            p.put(x, h - 2, "stone", 2 if x % 5 else 0)
    return p


GATE_FRAMES = ["open", "closed"]
GATE_TAGS = [("open", 0, 0), ("closed", 1, 1)]

HEAP_CANVAS = (48, 24)
HEAP_ITEMS = {
    "a": [("dome", (24, 34, 23, 22), "rag"), ("plank", (4, 12, 22, 4, -24), "wood"), ("pot", (34, 15, 6, 5), "iron"),
          ("bottle", (14, 16, 2, 5), "glass"), ("drape", (22, 22, 12, 5), "rag")],
    "b": [("dome", (24, 36, 22, 24), "rag"), ("crate", (6, 10, 14, 9, 8), "wood"), ("pot", (30, 14, 5, 4), "copper"),
          ("plank", (22, 8, 20, 3, 18), "wood"), ("drape", (26, 21, 12, 5), "rag")],
    "c": [("dome", (24, 36, 24, 25), "rag"), ("plank", (14, 9, 24, 3, -8), "wood"), ("bottle", (36, 14, 2, 5), "glass"),
          ("bottle", (9, 15, 2, 4), "glass"), ("stone", (27, 20, 5, 4), "stone"), ("drape", (16, 22, 10, 5), "rag")],
}
HEAP_FRAMES = ["heap_a", "heap_b", "heap_c"]
HEAP_TAGS = [("heap_a", 0, 0), ("heap_b", 1, 1), ("heap_c", 2, 2)]

OPENED_ITEMS = [
    ("dome", (88, 90, 78, 42), "rag"),
    ("plank", (24, 59, 40, 4, -12), "wood"),
    ("plank", (96, 56, 34, 4, 9), "wood"),
    ("pot", (122, 63, 9, 6), "iron"),
    ("bottle", (138, 62, 3, 6), "glass"),
    ("pot", (50, 64, 7, 5), "copper"),
    ("drape", (70, 64, 24, 6), "rag"),
]


def darken(layer: Layer, steps: int = 1) -> None:
    """R6: non-solid junk sits about one ramp step darker than the solids."""
    for y in range(layer.h):
        for x in range(layer.w):
            c = layer.px[y][x]
            if c is not None and c[0] != "mortar":
                layer.px[y][x] = (c[0], max(MAT_FLOOR.get(c[0], 0), c[1] - steps))


MAT_FLOOR = {"floor": 1, "wood": 0, "iron": 0, "copper": 1, "brass": 0, "gal": 3, "stone": 1}


def rim_light(layer: Layer, lifts=(3, 2, 1)) -> None:
    """HOARD-02 polish: the torch (upper left) catches the top of a heap. Under the outline, the first pixels of each
    column are lifted by ``lifts`` ramp steps (within the material's band), so a low dark heap separates from the dark
    floor shapes at the lane edge. The outline stays on top, so the silhouette keeps its ragged dark edge."""
    for x in range(layer.w):
        ys = [y for y in range(layer.h) if layer.px[y][x] is not None and layer.px[y][x] != OUTLINE]
        for k, y in enumerate(ys[:len(lifts)]):
            ramp, idx = layer.px[y][x]
            hi = MAT[layer.mat[y][x]][2] if layer.mat[y][x] in MAT else len(RAMPS[ramp]) - 1
            layer.px[y][x] = (ramp, min(hi, idx + lifts[k]))


# HOARD-02 polish (PR #258 review follow-up 3): heap b stands at x 470, right behind the dark rounded floor shapes of
# the lane edge, and read as one more dark blob. It keeps its items one step darker than the solids but gets the
# torch's rim light on its ragged top.
RIM_LIT_HEAPS = {"b"}


def heap(kind: str) -> Layer:
    p = Layer(*HEAP_CANVAS)
    draw(p, HEAP_ITEMS[kind])
    darken(p)
    outline(p)
    if kind in RIM_LIT_HEAPS:
        rim_light(p)
    return p


def mound_opened() -> Layer:
    """The mound after the reveal: a low spill (at most 24 px) in the bottom rows of the 176 x 72 canvas,
    ragged top, one step darker, behind her and the heroes."""
    p = Layer(W, H)
    draw(p, OPENED_ITEMS)
    for y in range(0, H - 24):
        for x in range(W):
            p.px[y][x] = None
    darken(p)
    outline(p)
    return p


def mound_frames() -> list[Layer]:
    back, front = Layer(), Layer()
    draw(back, BACK_ITEMS)
    draw(front, FRONT_ITEMS)
    for y in range(H):              # the front layer keeps only the near lip
        for x in range(W):
            if y < LIP_Y + (abs(x - PIVOT[0]) // 6):
                front.px[y][x] = None
    outline(back)
    outline(front)
    return [back, front, mound_opened()]


MOUND_FRAMES = ["sealed_back", "sealed_front", "opened"]
MOUND_TAGS = [("sealed_back", 0, 0), ("sealed_front", 1, 1), ("opened", 2, 2)]

# piece id -> (frames, frame names, tags, pivot, role). Roles (spec section 7): solid, target, dressing, door, gate.
PIECES = {
    "hoarder_mound": (mound_frames, MOUND_FRAMES, MOUND_TAGS, PIVOT, "dressing"),
    "hoarder_bank_a": (bank_sheet, BANK_FRAMES, BANK_TAGS, (0, 47), "solid"),
    "hoarder_post_b": (lambda: [post(s) for s in POST_FRAMES], POST_FRAMES, POST_TAGS, (0, 63), "target"),
    "hoarder_stack_b": (stack_sheet, STACK_FRAMES, STACK_TAGS, (0, STACK_CANVAS[1] - 1), "falling"),
    "hoarder_door": (lambda: [door(s) for s in DOOR_FRAMES], DOOR_FRAMES, DOOR_TAGS, (0, 227), "solid_door"),
    "hoarder_gate": (lambda: [gate(s) for s in GATE_FRAMES], GATE_FRAMES, GATE_TAGS, (0, 227), "solid_gate"),
    "hoarder_heap": (lambda: [heap(k) for k in "abc"], HEAP_FRAMES, HEAP_TAGS, (0, 23), "dressing"),
}


def piece_grid(name: str) -> dict:
    frames_fn, _names, tags, pivot, _role = PIECES[name]
    imgs = frames_fn()
    return kit.grid_from(imgs, layer_names=["main"], durations=[100] * len(imgs), pivot=pivot,
                         tags=[{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in tags])


def piece_paths(name: str) -> tuple[Path, Path]:
    return OUT / f"{name}.grid.json", OUT / f"{name}.ase"


MANIFEST_DIR = ROOT / "assets/manifests"
SPEC_SIZES = {   # (canvas, collision rect (w, h) the sprite must match, or None for dressing)
    "hoarder_mound": ((W, H), None), "hoarder_bank_a": (BANK_CANVAS, (64, 48)), "hoarder_post_b": (POST_CANVAS, (16, 64)),
    "hoarder_stack_b": (STACK_CANVAS, (128, 64 + STACK_LEG)), "hoarder_door": (DOOR_CANVAS, (16, 228)),
    "hoarder_gate": (DOOR_CANVAS, (16, 228)), "hoarder_heap": (HEAP_CANVAS, None),
}


def piece_manifest(name: str, grid: dict) -> dict:
    _fn, names, tags, pivot, role = PIECES[name]
    ase = piece_paths(name)[1].relative_to(ROOT).as_posix()
    gridp = piece_paths(name)[0].relative_to(ROOT).as_posix()
    return {
        "schema_version": 1,
        "id": f"floor01_{name}_v1",
        "type": "prop_animation",
        "source_path": ase,
        "grid_source": gridp,
        "output_atlas": f"generated/art/floor01_{name}.png",
        "output_metadata": f"generated/art/floor01_{name}.json",
        "palette_id": "floor01-catacomb-v2",
        "frame_size": grid["canvas"],
        "pivot": list(pivot),
        "animation_tags": [n for n, _a, _b in tags],
        "required_alpha": True,
        "frame_names": names,
        "collision_role": role,
        "owner": "AI-authored environment pixel art (AREA-HOARDER-01: design spec and mound hero key Claude Opus, execution Claude Sonnet; independent review pending)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": ("Original pixel-grid-v1 room kit piece of the Floor 1 Hoarder room (docs/02-art/hoarder-hoard-02.md section 7) on the "
                       "floor01-catacomb-v2 ramps; rule-based shapes and a hash noise, nothing cropped, traced or resampled from concept art"),
            "generator": ("scripts/author_hoarder_room.py: the lair mound `sealed` hero key by Claude Opus, every other piece and state "
                          "by Claude Sonnet; independent review pending; no image-model raster output"),
            "generated_art": True,
            "input_reference_ids": ["floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)"],
        },
    }


def build() -> dict:
    """The lair mound grid (the hero key, frames 0 and 1, plus the `opened` spill)."""
    return piece_grid("hoarder_mound")


def frame_image(grid: dict, index: int):
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}
    w, h = grid["canvas"]
    im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    for name in grid["layers"]:
        for y, row in enumerate(grid["frames"][index]["cels"][name]):
            for x, ch in enumerate(row):
                if ch != ".":
                    im.putpixel((x, y), pal[ch])
    return im


def preview(out: Path, scale: int = 3) -> None:
    """Every state of every piece on a mid-brick panel, one row per piece, frames left to right."""
    from PIL import Image
    rows = []
    for name in PIECES:
        g = piece_grid(name)
        rows.append([frame_image(g, i) for i in range(len(g["frames"]))])
    pad = 6
    width = max(sum(im.width for im in row) + pad * (len(row) + 1) for row in rows) * scale
    height = sum(max(im.height for im in row) + pad for row in rows) * scale + pad * scale
    sheet = Image.new("RGBA", (width, height), (58, 40, 40, 255))
    y = pad * scale
    for row in rows:
        x = pad * scale
        for im in row:
            sheet.alpha_composite(im.resize((im.width * scale, im.height * scale), Image.NEAREST), (x, y))
            x += (im.width + pad) * scale
        y += (max(im.height for im in row) + pad) * scale
    sheet.save(out)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    if a.preview:
        preview(a.preview)
        return 0
    bad = 0
    for name in PIECES:
        grid = piece_grid(name)
        text = json.dumps(grid, indent=1) + "\n"
        gp, ase = piece_paths(name)
        mp = MANIFEST_DIR / f"floor01_{name}.json"
        mtext = json.dumps(piece_manifest(name, grid), indent=1) + "\n"
        print(f"{name}: {len(grid['palette'])} colours, {len(grid['frames'])} frames")
        if a.check:
            ok = gp.is_file() and gp.read_text() == text and mp.is_file() and mp.read_text() == mtext
            print("  up to date" if ok else "  differs")
            bad += 0 if ok else 1
            continue
        gp.write_text(text, encoding="utf-8")
        mp.write_text(mtext, encoding="utf-8")
        subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(gp), str(ase)],
                       check=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
