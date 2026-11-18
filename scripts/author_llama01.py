#!/usr/bin/env python3
"""LLAMA-01 authoring record: Bad Llama palette, rule-based key poses, size study and spit key.

Usage:

    python3 scripts/author_llama01.py keys [--check] [--preview PNG]
    python3 scripts/author_llama01.py size-study [--check]

``keys`` (re)writes the key frames of ``assets/source/characters/llama/llama-01.grid.json``
(frames 0, 3, 8, 9, 11, 14, 15; see ``docs/02-art/llama-01-spec.md`` section 7) and the ten
in-betweens (frames 1, 2, 4, 5, 6, 7, 10, 12, 13, 16, section 8), plus ``assets/manifests/llama-01.json``
with the 60 Hz tick table. Every frame is
rasterized natively from one parametric rig (capsules and ellipses at pixel centres, volumetric
ramp shading from an upper-left torch key, shag fringes, mange patches, a 1 px outline outside
the fill) plus literal face/teeth stamps. The in-betweens (``INBETWEENS``) are derived from the keys through
``shifted()`` and are not frozen; the keys are guarded by the frozen-key fixture. ``--check``
compares grid and manifest instead of writing (exit 1 on a difference).

``size-study`` writes ``llama-01_size_study.grid.json``: the idle key rasterized at 56, 64 and
72 px opaque height from the same rig (canvas 96 x 80, pivot (44, 79)).

No concept pixels are traced or sampled; colours are chosen by eye. Presentation-only art
tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LLAMA_DIR = ROOT / "assets/source/characters/llama"
GRID = LLAMA_DIR / "llama-01.grid.json"
SIZE_STUDY = LLAMA_DIR / "llama-01_size_study.grid.json"
MIRROR_LIT = LLAMA_DIR / "llama-01_mirror_lit.grid.json"   # scene-only: same frames, torch from the upper right
CANVAS = (96, 72)
PIVOT = (44, 71)
TICK_HZ = 60

# llama-floor01-v0 (17 of 20). Fur is a pale, desaturated tan whose shadows cool toward plum
# (cold ambient) so it separates from the saturated orange brick by value; the throat glow ramp
# is the only saturated hot family on the body and its core is the brightest key in the atlas.
PALETTE = [
    ("K", "outline", "#1a1016ff", False),
    ("a", "fur_deep", "#3a2a32ff", False),
    ("s", "fur_shade", "#6e4e44ff", False),
    ("m", "fur_mid", "#a37c58ff", False),
    ("l", "fur_light", "#cfab7cff", False),
    ("L", "fur_top", "#ecd3a4ff", False),
    ("p", "mange", "#4f2c26ff", False),
    ("h", "hoof", "#4a3c44ff", False),
    ("T", "tooth", "#e6c447ff", True),
    ("t", "tooth_shade", "#9c7a26ff", False),
    ("o", "mouth", "#4a1020ff", False),
    ("g", "gum", "#a2404aff", False),
    ("e", "eye", "#f1ead0ff", True),
    ("r", "glow_deep", "#8c1e12ff", False),
    ("R", "glow_mid", "#e2491cff", False),
    ("O", "glow_hot", "#ff9d2eff", False),
    ("Y", "glow_core", "#ffe680ff", True),
]
KEYS = [p[0] for p in PALETTE]
ACCENTS = {p[0] for p in PALETTE if p[3]}
GLOW = ("r", "R", "O", "Y")
LIGHT = (-0.55, -0.835)          # unit vector toward the torch key (upper left)

TAGS = [  # name, first, last (tick windows live in llama-01.pixelart.json)
    ("idle", 0, 2), ("walk", 3, 6), ("attack_tell", 7, 8), ("attack_swing", 9, 10),
    ("hurt", 11, 11), ("down", 12, 14), ("spit", 15, 16),
]
TICKS = [14, 10, 14, 6, 6, 6, 6, 8, 22, 6, 14, 12, 6, 8, None, 4, 4]
KEY_FRAMES = (0, 3, 8, 9, 11, 14, 15)


# --------------------------------------------------------------------------- raster rig
def ramp(material: str, v: float) -> str:
    if material == "fur":
        return "L" if v > 0.6 else "l" if v > 0.18 else "m" if v > -0.32 else "s" if v > -0.72 else "a"
    if material == "muzzle":
        return "L" if v > 0.45 else "l" if v > -0.05 else "m" if v > -0.55 else "s"
    if material == "leg":
        return "l" if v > 0.5 else "m" if v > -0.05 else "s" if v > -0.6 else "a"
    if material == "far":
        return "m" if v > 0.55 else "s" if v > -0.25 else "a"
    if material == "hoof":
        return "h" if v > -0.3 else "a"
    if material == "ear_in":
        return "s" if v > 0.0 else "a"
    if material == "mouth":
        return "o"
    raise ValueError(material)


TUFT = (0, 2, 3, 1, 0, 1, 3, 2, 0, 0, 2, 1)   # irregular clumps: matted locks, not an even comb


def tuft(s: float, phase: float = 0.0) -> float:
    return TUFT[int(math.floor(s + phase)) % len(TUFT)] / 2.0


class Part:
    def __init__(self, kind, geo, material, group="main", shag=None, glow=0.0, name="", span=(0.0, 1.0)):
        self.kind, self.geo, self.material, self.group = kind, geo, material, group
        self.shag, self.glow, self.name, self.span = shag, glow, name, span

    def sample(self, px: float, py: float):
        """Return (inside, v, q, s): light value, front offset (-1..1) and axis position (0..1)."""
        if self.kind == "cap":
            (ax, ay), (bx, by), ra, rb = self.geo
            vx, vy = bx - ax, by - ay
            l2 = vx * vx + vy * vy
            t = max(0.0, min(1.0, ((px - ax) * vx + (py - ay) * vy) / l2)) if l2 else 0.0
            ox, oy = px - (ax + t * vx), py - (ay + t * vy)
            r = ra + (rb - ra) * t
            d = math.hypot(ox, oy)
            extra = 0.0
            if self.shag and d > 0:
                (sx, sy), amount = self.shag
                facing = 1.0 if (sx, sy) == (0, 0) else max(0.0, (ox * sx + oy * sy) / d)
                extra = amount * tuft(t * math.sqrt(l2) * 1.3, 0.5) * facing
            if d > r + extra:
                return None
            ln = math.sqrt(l2) or 1.0
            nx, ny = -vy / ln, vx / ln          # front normal (right of the axis direction)
            if nx < 0:
                nx, ny = -nx, -ny
            v = (ox * LIGHT[0] + oy * LIGHT[1]) / max(r, 0.8)
            q = (ox * nx + oy * ny) / max(r, 0.8)
            return max(-1, min(1, v)), max(-1, min(1, q)), self.span[0] + t * (self.span[1] - self.span[0])
        if self.kind == "ell":
            cx, cy, rx, ry = self.geo
            nx, ny = (px - cx) / rx, (py - cy) / ry
            d = math.hypot(nx, ny)
            limit = 1.0
            if self.shag and d > 0:
                (sx, sy), amount = self.shag
                facing = max(0.0, (nx * sx + ny * sy) / d)
                limit += amount * tuft(px if abs(sy) >= abs(sx) else py) * facing / max(rx, ry)
            if d > limit:
                return None
            return max(-1, min(1, nx * LIGHT[0] + ny * LIGHT[1])), nx, (ny + 1) / 2
        if self.kind == "tri":
            (x1, y1), (x2, y2), (x3, y3) = self.geo
            def side(ax, ay, bx, by):
                return (px - bx) * (ay - by) - (ax - bx) * (py - by)
            d1, d2, d3 = side(x1, y1, x2, y2), side(x2, y2, x3, y3), side(x3, y3, x1, y1)
            neg = d1 < 0 or d2 < 0 or d3 < 0
            pos = d1 > 0 or d2 > 0 or d3 > 0
            return None if (neg and pos) else (0.0, 0.0, 0.5)
        raise ValueError(self.kind)


def glow_key(q: float, s: float, strength: float) -> str | None:
    """Throat glow: a hot core stripe on the front of the neck, cooling toward the back edge."""
    if s < 0.08 or s > 0.97 or q < -0.3:
        return None
    level = 3 if 0.15 <= q <= 0.62 else 2 if -0.05 <= q <= 0.85 else 1 if q <= 0.95 else 0
    if s < 0.2 or s > 0.9:
        level -= 1
    level -= round((1.0 - strength) * 3)
    return None if level < 0 else GLOW[level]


def mouth_glow(v_unused, q, s, strength):
    return "Y" if strength > 0.9 else "O"


class Canvas:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.px = [["." for _ in range(w)] for _ in range(h)]
        self.group = [[None for _ in range(w)] for _ in range(h)]
        self.mat = [[None for _ in range(w)] for _ in range(h)]
        self.lock = [[False for _ in range(w)] for _ in range(h)]

    def draw(self, part: Part) -> None:
        for y in range(self.h):
            for x in range(self.w):
                hit = part.sample(x + 0.5, y + 0.5)
                if hit is None:
                    continue
                v, q, s = hit
                key = ramp(part.material, v)
                if part.glow:
                    g = glow_key(q, s, part.glow) if part.material != "mouth" else mouth_glow(v, q, s, part.glow)
                    if g:
                        key = g
                self.px[y][x], self.group[y][x], self.mat[y][x] = key, part.group, part.material

    def outline(self, group: str) -> None:
        ring = []
        for y in range(self.h):
            for x in range(self.w):
                if self.group[y][x] == group:
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < self.w and 0 <= ny < self.h and self.group[ny][nx] == group:
                        ring.append((x, y))
                        break
        for x, y in ring:
            self.px[y][x], self.group[y][x], self.mat[y][x] = "K", group + "_line", None

    def _furry(self, x: int, y: int) -> bool:
        return 0 <= x < self.w and 0 <= y < self.h and self.mat[y][x] == "fur" and self.px[y][x] not in GLOW

    def streak(self, x, y, length, slope) -> None:
        """A matted streak that follows the hair down and back: 2 px wide at the root, 1 px toward the tip."""
        n = int(round(length))
        for i in range(n):
            yy = int(round(y)) + i
            xx = int(round(x + slope * i + 0.6 * math.sin(yy * 2.1 + x * 0.7)))
            for k in ((0, 1) if i < n * 0.6 else (0,)):
                if self._furry(xx + k, yy):
                    self.px[yy][xx + k] = "a" if i == 0 and k == 0 else "p"

    def bald(self, cx, cy, rx, ry) -> None:
        """A bald, scabby patch: flat shade (no strands) inside a broken dark rim of matted fur."""
        for y in range(self.h):
            for x in range(self.w):
                if not self._furry(x, y):
                    continue
                d = math.hypot((x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry)
                n = 0.3 * math.sin(x * 2.3 + y * 1.7) + 0.2 * math.sin(x * 0.9 - y * 2.9)
                if d <= 0.6 + n:
                    self.px[y][x] = "s"
                elif d <= 1.05 + n and (x * 3 + y) % 3:
                    self.px[y][x] = "p"

    def mange(self, cx, cy, rx, ry) -> None:
        """A ragged dark streak of mangy, matted fur (never a clean round spot)."""
        for y in range(self.h):
            for x in range(self.w):
                if self.mat[y][x] != "fur" or self.px[y][x] in GLOW:
                    continue
                d = ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2
                if d <= 1.0 + 0.45 * math.sin(x * 1.9 + y * 2.7) + 0.25 * math.sin(x * 0.7 - y * 3.1):
                    self.px[y][x] = "a" if self.px[y][x] in "sap" or d < 0.25 else "p"

    def strands(self) -> None:
        """Hanging fur strands: short down-back diagonals one ramp step darker on lit fur."""
        darker = {"L": "l", "l": "m", "m": "s"}
        for y in range(self.h):
            for x in range(self.w):
                if self.mat[y][x] == "fur" and self.px[y][x] in darker and (x + y) % 4 == 0 and ((y - x) // 3) % 3 == 0:
                    self.px[y][x] = darker[self.px[y][x]]

    def stamp(self, x0: int, y0: int, rows: list[str]) -> None:
        for dy, row in enumerate(rows):
            for dx, c in enumerate(row):
                x, y = x0 + dx, y0 + dy
                if c != "." and 0 <= x < self.w and 0 <= y < self.h:
                    self.px[y][x], self.lock[y][x] = c, True

    def line(self, a, b, key="K") -> None:
        (x0, y0), (x1, y1) = a, b
        n = max(abs(x1 - x0), abs(y1 - y0)) or 1
        for i in range(n + 1):
            x, y = round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)
            self.px[y][x], self.lock[y][x] = key, True

    def cleanup(self) -> None:
        """Repaint unlocked one-pixel colour islands (orphans) with their commonest neighbour colour."""
        for _ in range(3):
            changes = []
            for y in range(self.h):
                for x in range(self.w):
                    c = self.px[y][x]
                    if c in ".K" or c in ACCENTS or self.lock[y][x]:
                        continue
                    around = [self.px[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                              if (dx or dy) and 0 <= x + dx < self.w and 0 <= y + dy < self.h]
                    if c in around:
                        continue
                    pool = [n for n in around if n not in ".K" and n not in ACCENTS]
                    if pool:
                        changes.append((x, y, max(sorted(set(pool)), key=pool.count)))
            for x, y, c in changes:
                self.px[y][x] = c
            if not changes:
                break

    def rows(self) -> list[str]:
        return ["".join(r) for r in self.px]


# --------------------------------------------------------------------------- face stamps
EYES = {
    "angry": ["KK....", "KKKKK.", ".eeKK.", ".KK..."],
    "tell": ["KKKK..", ".KKKKK", ".eYeK.", "..KK.."],
    "shut": ["K...", ".KK.", "K..."],
    "dead": ["K.K", ".K.", "K.K"],
}
TEETH = {
    "over": ["KTTKTTK", "KTTKTtK", "KTtKTtK", ".Kt.KK."],     # buck overbite over the lower lip
    "upper": ["KTTKTTK", "KTtKTtK", ".Tt.Tt.", ".K..K.."],    # mouth open, upper row
    "lower": [".K.K.", ".TKTK", "tTtTK"],                     # mouth open, lower row
    "clench": ["KTTKTTKT", "KTtKTtKt", "KKKKKKKK", "KTtTKtTK", ".KKKKKK."],
}


# (kind, dx, dy, a, b) relative to the rump centre. "s" = streak (top x, top y, length, x per row: negative runs
# back, the way the hair falls); "b" = bald patch (centre, rx, ry). Design fix round 1 (D3): no round spots.
MANGE = (
    ("s", -5, -5, 8, -0.45), ("s", 7, -6, 9, -0.35), ("s", 19, -6, 8, -0.4), ("s", -2, 3, 6, -0.3),
    ("b", 12, 1.5, 3.4, 1.8), ("b", -2, -2, 2.0, 2.4),
)


def poly(pts, radii, material, group="main", shag=None, glow=0.0):
    """A chain of capsules through ``pts`` (2 or more points); the glow axis position runs over the whole chain.

    ``radii`` has one radius per point, or two (start, end) that are interpolated. ``shag`` may be a function
    of the segment (a, b) so the mane always sits on the back edge of each segment."""
    pts = [tuple(q) for q in pts]
    n = len(pts)
    if len(radii) != n:
        radii = [radii[0] + (radii[-1] - radii[0]) * i / (n - 1) for i in range(n)]
    lens = [math.dist(a, b) for a, b in zip(pts, pts[1:])]
    total, acc, out = sum(lens) or 1.0, 0.0, []
    for a, b, ra, rb, ln in zip(pts, pts[1:], radii, radii[1:], lens):
        sh = shag(a, b) if callable(shag) else shag
        out.append(Part("cap", (a, b, ra, rb), material, group, shag=sh, glow=glow,
                        span=(acc / total, (acc + ln) / total)))
        acc += ln
    return out


def mane(a, b):
    nx, ny = b[0] - a[0], b[1] - a[1]
    ln = math.hypot(nx, ny) or 1.0
    back = (ny / ln, -nx / ln) if ny < 0 else (-ny / ln, nx / ln)   # mane side: the back of the neck
    return (back, 2.4)


def llama(c: Canvas, p: dict) -> None:
    far, main = [], []
    for leg in ("far_hind", "far_front"):
        pts = p[leg]
        for (x1, y1, r1), (x2, y2, r2) in zip(pts, pts[1:]):
            far.append(Part("cap", ((x1, y1), (x2, y2), r1, r2), "far", "far"))
        hx = p[leg + "_hoof"]
        far.append(Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof", "far"))
    er = p.get("ear_r", (1.8, 1.0))
    far += poly(p["ear_far"], (er[0] - 0.2, er[-1] - 0.1), "ear_in", "far")
    for part in far:
        c.draw(part)
    c.outline("far")

    tail = p["tail"]
    main.append(Part("cap", (tail[0], tail[1], 1.4, 2.4), "fur", shag=((0, 0), 2.6)))   # shaggy plume, thin root
    rump, chest = p["rump"], p["chest"]
    rr = p.get("barrel_rr") or (p["barrel_r"], p["barrel_r"] - 0.5)
    main.append(Part("cap", (rump, chest, *rr), "fur", shag=((0, 0), 1.0)))              # ragged all round
    main.append(Part("ell", p["belly"], "fur", shag=((0, 1), 2.0)))
    main.append(Part("ell", p["haunch"], "fur", shag=((-1, 0.3), 1.6)))
    main.append(Part("ell", p["shoulder"], "fur", shag=((0.6, 1), 2.0)))
    for leg in ("near_hind", "near_front"):
        pts = p[leg]
        for i, ((x1, y1, r1), (x2, y2, r2)) in enumerate(zip(pts, pts[1:])):
            mat = "fur" if i == 0 else "leg"
            main.append(Part("cap", ((x1, y1), (x2, y2), r1, r2), mat, shag=((0, 1), 1.0) if i == 0 else None))
        hx = p[leg + "_hoof"]
        main.append(Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof"))
    main += poly(p["neck"], p["neck_r"], "fur", shag=mane, glow=p.get("glow", 0.0))
    if p.get("throat"):
        main.append(Part("ell", p["throat"], "fur", glow=1.0, name="throat"))
    main.append(Part("ell", p["skull"], "fur"))
    ja, jb = p["jaw"]
    main.append(Part("cap", (ja, jb, 2.6, 2.0), "muzzle"))
    ma, mb = p["muzzle"]
    main.append(Part("cap", (ma, mb, *p.get("muzzle_r", (3.6, 2.9))), "muzzle"))
    if p.get("mouth"):
        main.append(Part("tri", p["mouth"], "mouth", glow=p.get("mouth_glow", 0.0)))
    main += poly(p["ear_near"], er, "fur")
    for part in main:
        c.draw(part)
    if p.get("throat"):  # the puffed throat sac glows from the inside: core in the middle
        cx, cy, rx, ry = p["throat"]
        for y in range(c.h):
            for x in range(c.w):
                d = math.hypot((x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry)
                if d <= 1.0:
                    c.px[y][x] = "Y" if d < 0.45 else "O" if d < 0.75 else "R"
    rx0, ry0 = rump
    c.strands()
    for kind, dx, dy, a, b in p.get("mange", MANGE):
        (c.streak if kind == "s" else c.bald)(rx0 + dx, ry0 + dy, a, b)
    c.outline("main")
    if p.get("mouth_line"):
        c.line(*p["mouth_line"])
    for name, (x, y) in p["teeth"]:
        c.stamp(x, y, TEETH[name])
    ex, ey = p["eye_at"]
    c.stamp(ex, ey, EYES[p["eye"]])
    if p.get("nostril"):
        c.stamp(*p["nostril"], ["KK"])
    for x, y, rows in p.get("extra", ()):
        c.stamp(x, y, rows)
    c.cleanup()


def spit(c: Canvas, phase: int = 0) -> None:
    """Lava spit glob: hot core, cooling skin, a dark crust tail and one drip, travelling right.

    ``phase`` 1 is the wobble frame (spec section 8, frame 16): the glob is stretched, the tail is
    1 px longer and the drip hangs 1 px lower.
    """
    if phase:
        tail_end, glob, drip, core, crust = (79.0, 27.5), (89.5, 26.5, 4.4, 3.0), ((83.5, 29.0), (82.5, 32.0)), (89.7, 26.5), ((79, 27), (82, 32))
    else:
        tail_end, glob, drip, core, crust = (80.0, 27.5), (89.5, 26.5, 4.2, 3.2), ((84.5, 28.5), (83.5, 31.5)), (90.2, 26.0), ((80, 27), (83, 31))
    parts = [
        Part("cap", ((88.0, 26.5), tail_end, 2.6, 1.0), "fur", group="spit"),
        Part("ell", glob, "fur", group="spit"),
        Part("cap", (*drip, 1.0, 0.8), "fur", group="spit"),
    ]
    for part in parts:
        for y in range(c.h):
            for x in range(c.w):
                hit = part.sample(x + 0.5, y + 0.5)
                if hit:
                    c.px[y][x], c.group[y][x] = "r", "spit"
    for y in range(c.h):
        for x in range(c.w):
            if c.group[y][x] != "spit":
                continue
            d = math.hypot((x + 0.5 - core[0]) / 3.4, (y + 0.5 - core[1]) / 2.4)
            c.px[y][x] = "Y" if d < 0.5 else "O" if d < 0.85 else "R" if d < 1.25 or x > 86 else "r"
    c.outline("spit")
    for x, y in crust:
        c.stamp(x, y, ["a"])
    c.cleanup()


# --------------------------------------------------------------------------- key poses
def hooves(**kw):
    return {k + "_hoof": v for k, v in kw.items()}


# Design fix round 1 (spec section 14): lean body with shoulder and hip masses and a tucked belly (D4), a thin
# S-curved neck carrying the head high (D2), long banana ears that prick forward when aggressive and pin back
# only on tell, release and hurt (D1), and streaky mange (D3, see MANGE).
IDLE = {
    "rump": (29, 41), "chest": (51, 41), "barrel_r": 6.0, "barrel_rr": (5.5, 6.5),
    "belly": (45, 45.5, 8, 4.0), "haunch": (28, 41, 6.5, 7.0), "shoulder": (52, 42, 5.5, 7.5),
    "near_hind": [(29, 45, 4.5), (32, 52, 3.0), (26, 59, 2.1), (27, 67, 1.3)],
    "far_hind": [(34, 45, 3.5), (36, 52, 2.5), (31.5, 59, 1.9), (33, 67, 1.2)],
    "near_front": [(52, 46, 3.6), (51, 53, 2.4), (53, 59, 2.1), (55, 67, 1.3)],
    "far_front": [(56, 46, 3.0), (57, 53, 2.2), (59.5, 59, 1.9), (62, 67, 1.2)],
    **hooves(near_hind=(27, 69), far_hind=(33, 69), near_front=(55, 69), far_front=(62, 69)),
    "neck": ((54, 40), (59, 31), (60, 23)), "neck_r": (4.8, 3.5, 3.1),
    "skull": (62, 19, 5.2, 4.2),
    "muzzle": ((64, 20), (72, 22)), "jaw": ((64, 26), (71, 28)), "mouth": ((65, 24.5), (75, 24.5), (72, 27.5)),
    "ear_near": ((60, 16), (59.5, 12.5), (61.5, 10.0)), "ear_far": ((62.5, 16), (63, 13), (65.5, 10.5)),
    "ear_r": (1.8, 1.0),
    "tail": ((22, 37), (13, 29), (-0.5, -1)),
    "mouth_line": None, "teeth": [("upper", (68, 23)), ("lower", (69, 27))],
    "eye": "angry", "eye_at": (60, 17), "nostril": (73, 19),
}


def shifted(base: dict, **over) -> dict:
    out = dict(base)
    out.update(over)
    return out


WALK_CONTACT = shifted(
    IDLE,
    rump=(29, 42), chest=(51, 42), belly=(45, 46.5, 8, 4.0), haunch=(28, 42, 6.5, 7.0), shoulder=(52, 43, 5.5, 7.5),
    near_front=[(52, 47, 3.6), (55, 53, 2.4), (58, 59, 2.1), (61, 67, 1.3)],
    far_front=[(56, 47, 3.0), (56, 53, 2.2), (56.5, 59, 1.9), (57, 67, 1.2)],
    near_hind=[(29, 46, 4.5), (28, 53, 3.0), (22, 59, 2.1), (21, 67, 1.3)],
    far_hind=[(34, 46, 3.5), (37, 53, 2.5), (33, 59, 1.9), (34, 67, 1.2)],
    **hooves(near_hind=(21, 69), far_hind=(34, 69), near_front=(61, 69), far_front=(57, 69)),
    neck=((54, 41), (60, 32), (61, 24)), skull=(63, 20, 5.2, 4.2), muzzle=((65, 21), (73, 23)), jaw=((65, 27), (72, 29)),
    mouth=((66, 25.5), (76, 25.5), (73, 28.5)),
    ear_near=((61, 17), (60.5, 13), (63, 10.0)), ear_far=((63.5, 17), (64.5, 13.5), (67.5, 10.5)),
    tail=((22, 38), (13, 32), (-0.6, -1)),
    teeth=[("upper", (69, 24)), ("lower", (70, 28))], eye_at=(61, 18), nostril=(74, 20),
)

TELL = shifted(
    IDLE,
    rump=(27, 42), chest=(49, 39), belly=(42, 45, 8, 4.0), haunch=(26, 43, 6.5, 7.0), shoulder=(50, 40, 5.5, 7.5),
    near_front=[(50, 44, 3.6), (53, 51, 2.4), (56, 58, 2.1), (59, 67, 1.3)],
    far_front=[(54, 44, 3.0), (58, 51, 2.2), (61, 58, 1.9), (64, 67, 1.2)],
    near_hind=[(27, 46, 4.5), (30, 53, 3.0), (24, 59, 2.1), (25, 67, 1.3)],
    far_hind=[(32, 46, 3.5), (35, 53, 2.5), (30, 59, 1.9), (31, 67, 1.2)],
    **hooves(near_hind=(25, 69), far_hind=(31, 69), near_front=(59, 69), far_front=(64, 69)),
    neck=((52, 36), (59, 12)), neck_r=(5.0, 3.8), glow=1.0, throat=(60.5, 22.5, 3.4, 6.2),
    skull=(60, 9, 4.8, 4.2), muzzle=((62, 10), (71, 12)), jaw=((62, 13), (70, 14.5)), muzzle_r=(3.8, 2.8),
    ear_near=((57, 7), (52.5, 5), (47.5, 5.5)), ear_far=((59, 7), (54.5, 5.2), (50, 6)),
    tail=((20, 38), (11, 28), (-0.5, -1)),
    mouth=None, mouth_line=None, teeth=[("clench", (68, 11))],
    eye="tell", eye_at=(58, 8), nostril=(71, 9),
)

SPIT = shifted(
    IDLE,
    rump=(31, 40), chest=(54, 40), belly=(47, 44.5, 8, 4.0), haunch=(30, 40, 6.5, 7.0), shoulder=(55, 41, 5.5, 7.5),
    near_front=[(55, 46, 3.6), (58, 53, 2.4), (60, 59, 2.1), (62, 67, 1.3)],
    far_front=[(59, 46, 3.0), (62, 53, 2.2), (64, 59, 1.9), (65, 67, 1.2)],
    near_hind=[(31, 45, 4.5), (28, 53, 3.0), (22, 59, 2.1), (20, 67, 1.3)],
    far_hind=[(36, 45, 3.5), (36, 53, 2.5), (31, 59, 1.9), (29, 67, 1.2)],
    **hooves(near_hind=(19, 69), far_hind=(28, 69), near_front=(62, 69), far_front=(65, 69)),
    neck=((57, 38), (74, 25)), neck_r=(5.0, 3.5), glow=0.67,
    skull=(77, 23, 4.6, 3.8), muzzle=((79, 23.5), (87, 23.5)), jaw=((79, 27), (86, 31)), muzzle_r=(3.2, 2.5),
    mouth=((79, 26), (89, 25), (87, 31)), mouth_glow=1.0,
    ear_near=((75, 20.5), (70, 18.5), (65, 19)), ear_far=((77, 20.5), (72, 18.8), (67, 19.5)),
    tail=((23, 36), (12, 31), (-1, -0.3)),
    mouth_line=None, teeth=[("upper", (85, 25)), ("lower", (83, 29))],
    eye="angry", eye_at=(75, 21), nostril=(87, 20),
    extra=[(89, 25, ["Y", "O"]), (90, 26, ["R"])],
)

HURT = shifted(
    IDLE,
    rump=(26, 42), chest=(49, 41), belly=(42, 45.5, 8, 4.0), haunch=(25, 43, 6.5, 7.0), shoulder=(50, 42, 5.5, 7.5),
    near_front=[(50, 46, 3.6), (54, 51, 2.4), (57, 56, 2.1), (55, 62, 1.3)],
    far_front=[(55, 46, 3.0), (57, 53, 2.2), (58, 59, 1.9), (60, 67, 1.2)],
    near_hind=[(27, 46, 4.5), (30, 53, 3.0), (24, 59, 2.1), (25, 67, 1.3)],
    far_hind=[(32, 46, 3.5), (35, 53, 2.5), (30, 59, 1.9), (31, 67, 1.2)],
    **hooves(near_hind=(25, 69), far_hind=(31, 69), near_front=(55, 63), far_front=(60, 69)),
    neck=((51, 39), (54, 27), (54, 15)), neck_r=(4.8, 3.5, 3.2),
    skull=(54, 12, 4.6, 3.8), muzzle=((56, 12), (63, 7)), jaw=((56, 15), (63, 12)),
    mouth=((57, 13), (64, 8), (63, 12)),
    ear_near=((52, 10), (47, 9), (42.5, 10.5)), ear_far=((54, 10), (49, 9.4), (44.5, 11)),
    tail=((19, 39), (12, 44), (-1, 0.4)),
    mouth_line=None, teeth=[("upper", (61, 8))],
    eye="shut", eye_at=(52, 11), nostril=None,
)

DEAD = shifted(
    IDLE,
    rump=(30, 61), chest=(54, 62), barrel_rr=(6.0, 6.5), belly=(42, 64.5, 11, 5.0), haunch=(29, 62, 6.5, 6.5),
    shoulder=(54, 63, 5, 5.5),
    near_front=[(56, 65, 2.8), (63, 67.5, 2.0), (69, 68.5, 1.3)],
    far_front=[(55, 63, 2.4), (60, 64, 1.8)],
    near_hind=[(27, 64, 3.5), (21, 67.5, 2.2), (16, 68.5, 1.3)],
    far_hind=[(33, 62, 2.6), (29, 64, 1.8)],
    **hooves(near_hind=(14, 69), far_hind=(27, 64), near_front=(70, 69), far_front=(61, 64)),
    neck=((57, 61), (73, 64.5)), neck_r=(4.6, 3.4),
    skull=(76, 64.5, 4.4, 3.6), muzzle=((78, 65.5), (86, 67)), jaw=((78, 68.2), (85, 69.2)), muzzle_r=(3.0, 2.4),
    mouth=((79, 67), (87, 68), (85, 70)),
    ear_near=((74, 61.5), (69, 60.5), (64.5, 62)), ear_far=((76, 61.5), (71, 61), (66.5, 62.5)),
    tail=((24, 60), (17, 65), (-1, 0)),
    mouth_line=None, teeth=[("upper", (84, 67))],
    eye="dead", eye_at=(75, 63), nostril=(85, 65),
)


# --------------------------------------------------------------------------- in-between poses
# Spec section 8 (executor tier). Every pose derives from a frozen key through shifted(); the
# keys themselves are never touched.
def mv(pt, dx=0, dy=0):
    return (pt[0] + dx, pt[1] + dy)


def mv2(pair, dx=0, dy=0):
    return tuple(mv(q, dx, dy) for q in pair)


def mv_leg(leg, dy=0):
    """Move only the first (hip/shoulder) point of a leg."""
    return [(leg[0][0], leg[0][1] + dy, leg[0][2])] + list(leg[1:])


def move_body(p: dict, dy: float) -> dict:
    out = {}
    for k in ("rump", "chest"):
        out[k] = mv(p[k], 0, dy)
    for k in ("belly", "haunch", "shoulder"):
        out[k] = (p[k][0], p[k][1] + dy, p[k][2], p[k][3])
    for k in ("near_hind", "far_hind", "near_front", "far_front"):
        out[k] = mv_leg(p[k], dy)
    return out


def move_head(p: dict, dx=0, dy=0) -> dict:
    """Shift the neck end and everything on the head (the neck start stays on the shoulder)."""
    out = {
        "neck": (p["neck"][0], *(mv(q, dx, dy) for q in p["neck"][1:])),
        "skull": (p["skull"][0] + dx, p["skull"][1] + dy, p["skull"][2], p["skull"][3]),
        "muzzle": mv2(p["muzzle"], dx, dy), "jaw": mv2(p["jaw"], dx, dy),
        "ear_near": mv2(p["ear_near"], dx, dy), "ear_far": mv2(p["ear_far"], dx, dy),
        "teeth": [(n, mv(q, dx, dy)) for n, q in p["teeth"]],
        "eye_at": mv(p["eye_at"], dx, dy),
    }
    if p.get("mouth"):
        out["mouth"] = tuple(mv(q, dx, dy) for q in p["mouth"])
    if p.get("nostril"):
        out["nostril"] = mv(p["nostril"], dx, dy)
    return out


def half(a: float, b: float) -> float:
    return round((a + b) / 2 * 2) / 2


def mean_pose(a: dict, b: dict, keys) -> dict:
    """Per-number mean of two poses for the listed keys, rounded to 0.5."""
    def avg(x, y):
        if isinstance(x, (int, float)):
            return half(x, y)
        return type(x)(avg(i, j) for i, j in zip(x, y))
    return {k: avg(a[k], b[k]) for k in keys}


INBETWEEN_BODY = ("rump", "chest", "barrel_r", "belly", "haunch", "shoulder", "near_hind", "far_hind", "near_front",
                  "far_front", "near_hind_hoof", "far_hind_hoof", "near_front_hoof", "far_front_hoof", "tail")

def _tips(p: dict, dx=0, dy=0) -> dict:
    """Move only the tip (third point) of both ears."""
    return {k: (p[k][0], p[k][1], mv(p[k][2], dx, dy)) for k in ("ear_near", "ear_far")}


_head_x1 = move_head(IDLE, dx=1)

# 1 Inhale: body, first leg points and head up 1 px, hooves stay.
POSE_1 = shifted(IDLE, **move_body(IDLE, -1), **move_head(IDLE, dy=-1),
                 tail=(IDLE["tail"][0], (13, 27), IDLE["tail"][2]))

# 2 Exhale and chew: head x -1, jaw end / mouth third point / lower teeth y +1, ear tips a further x -1.
_h2 = move_head(IDLE, dx=-1)
POSE_2 = shifted(IDLE, **_h2, tail=(IDLE["tail"][0], (14, 30), IDLE["tail"][2]))
POSE_2["jaw"] = (_h2["jaw"][0], mv(_h2["jaw"][1], 0, 1))
POSE_2["mouth"] = (_h2["mouth"][0], _h2["mouth"][1], mv(_h2["mouth"][2], 0, 1))
POSE_2["teeth"] = [(n, mv(q, 0, 1)) if n == "lower" else (n, q) for n, q in _h2["teeth"]]
POSE_2.update(_tips(_h2, dx=-1))

# 4 Passing A: IDLE body, head x +1, far fore and near hind lifted 4 to 5 px.
POSE_4 = shifted(
    IDLE, **_head_x1,
    far_front=[(56, 46, 3.0), (58, 52, 2.2), (60.5, 57, 1.9), (60, 63, 1.2)], far_front_hoof=(60, 65),
    near_hind=[(29, 45, 4.5), (31, 51, 3.0), (27, 56, 2.1), (28, 62, 1.3)], near_hind_hoof=(28, 64),
)
# 5 Contact B: all four hooves down in the opposite phase of the contact key.
POSE_5 = shifted(
    WALK_CONTACT,
    far_front=[(56, 47, 3.0), (59, 53, 2.2), (62, 59, 1.9), (64, 67, 1.2)], far_front_hoof=(64, 69),
    near_front=[(52, 47, 3.6), (51, 53, 2.4), (50, 59, 2.1), (50, 67, 1.3)], near_front_hoof=(50, 69),
    far_hind=[(34, 46, 3.5), (33, 53, 2.5), (27, 59, 1.9), (26, 67, 1.2)], far_hind_hoof=(26, 69),
    near_hind=[(29, 46, 4.5), (32, 53, 3.0), (28, 59, 2.1), (29, 67, 1.3)], near_hind_hoof=(29, 69),
)
# 6 Passing B: frame 4 with the lifted pair swapped (near fore and far hind up).
POSE_6 = shifted(
    POSE_4,
    near_front=[(52, 46, 3.6), (54, 52, 2.4), (56.5, 57, 2.1), (56, 63, 1.3)], near_front_hoof=(56, 65),
    far_hind=[(34, 45, 3.5), (36, 51, 2.5), (32.5, 56, 1.9), (33.5, 62, 1.2)], far_hind_hoof=(33, 64),
    far_front=IDLE["far_front"], far_front_hoof=IDLE["far_front_hoof"],
    near_hind=IDLE["near_hind"], near_hind_hoof=IDLE["near_hind_hoof"],
)

# 7 Anticipation: mean of IDLE and TELL for the body, thin 2-point neck, red stripe, no sac yet, ears going back.
POSE_7 = shifted(
    IDLE, **mean_pose(IDLE, TELL, INBETWEEN_BODY),
    neck=(mean_pose(IDLE, TELL, ("neck",))["neck"][0], (60, 16)), neck_r=(4.9, 3.6), glow=0.4, throat=None,
    skull=(61, 13, 5.0, 4.2), muzzle=((63, 14), (71.5, 16.5)), jaw=((63, 18), (70.5, 20)), muzzle_r=(3.7, 2.85),
    mouth=None, teeth=[("clench", (68, 15))],
    ear_near=((58.5, 11), (56, 8), (52.5, 7)), ear_far=((60.5, 11), (58, 8.2), (55, 7.5)),
    eye="tell", eye_at=(59, 12), nostril=(72, 13),
)

# 10 Recover: SPIT shifted back and up, dark mouth, no flash, ear tips start to come up.
_h10 = move_head(SPIT, dx=-1, dy=-1)
POSE_10 = shifted(SPIT, **_h10, glow=0.33, mouth_glow=0, extra=[])
POSE_10.update(_tips(_h10, dy=-1))
POSE_10["jaw"] = (_h10["jaw"][0], (85, 29.5))
POSE_10["mouth"] = ((78, 25), (87, 24.5), (85, 28))
POSE_10["teeth"] = [("upper", (84, 24)), ("lower", (82, 27))]

# 12 Buckle: fore knees on the floor, rump down 3 px, head 12 px lower, ears limp.
POSE_12 = shifted(
    IDLE,
    rump=(29, 44), haunch=(28, 44, 6.5, 7.0), chest=(51, 49), belly=(44, 51, 8, 4.0), shoulder=(52, 50, 5.5, 6.5),
    near_front=[(52, 54, 3.4), (55, 60, 2.3), (58, 67, 2.1), (53, 68.5, 1.3)], near_front_hoof=(51, 68),
    far_front=[(56, 54, 3.0), (60, 60, 2.2), (62, 67, 2.0), (57, 68.5, 1.2)], far_front_hoof=(55, 68),
    near_hind=[(29, 48, 4.5), (33, 55, 3.0), (26, 61, 2.1), (27, 67, 1.3)], near_hind_hoof=(27, 69),
    far_hind=[(34, 48, 3.5), (37, 55, 2.5), (31.5, 61, 1.9), (33, 67, 1.2)], far_hind_hoof=(33, 69),
    neck=((54, 48), (61, 40), (67, 33)), neck_r=(4.8, 3.5, 3.1),
    skull=(69, 31, 5.2, 4.2), muzzle=((71, 33), (78, 37)), jaw=((70, 38), (76, 40)),
    mouth=((72, 36), (80, 38), (76, 40)),
    ear_near=((67, 28), (62, 26), (58, 27)), ear_far=((69, 28), (64, 26.4), (60, 27.5)),
    eye="shut", eye_at=(66, 29), teeth=[("upper", (75, 35)), ("lower", (74, 39))], nostril=(78, 33),
    tail=((22, 40), (15, 44), (-1, 0.4)),
)

# 13 Impact: DEAD with the barrel lifted, ears half up and flopping, tail still up.
POSE_13 = shifted(
    DEAD,
    rump=(30, 59), chest=(54, 60), belly=(42, 62.5, 11, 5.0),
    neck=(DEAD["neck"][0], (73, 61)), skull=(76, 60.5, 4.4, 3.6),
    muzzle=((78, 61.5), (86, 63)), jaw=((78, 64.2), (85, 65.2)), mouth=((79, 63), (87, 64), (85, 66)),
    ear_near=((74, 57.5), (70, 55), (66, 55)), ear_far=((76, 57.5), (72, 55.5), (68, 56)),
    eye="shut", eye_at=(74, 59), teeth=[("upper", (84, 63))], nostril=(85, 61),
    near_front=[(56, 63, 2.8), (61, 66, 2.0), (66, 68.5, 1.3)], near_front_hoof=(67, 69),
    tail=((24, 58), (19, 53), (-1, 0)),
)

INBETWEENS = (1, 2, 4, 5, 6, 7, 10, 12, 13, 16)
POSES = {0: IDLE, 1: POSE_1, 2: POSE_2, 3: WALK_CONTACT, 4: POSE_4, 5: POSE_5, 6: POSE_6, 7: POSE_7, 8: TELL, 9: SPIT,
         10: POSE_10, 11: HURT, 12: POSE_12, 13: POSE_13, 14: DEAD}


def render(frame: int) -> list[str]:
    c = Canvas(*CANVAS)
    if frame in (15, 16):
        spit(c, frame - 15)
    else:
        llama(c, POSES[frame])
    return c.rows()


def scaled(pose: dict, k: float, pivot=(44, 71), to=(44, 79)) -> dict:
    def pt(v):
        return (to[0] + (v[0] - pivot[0]) * k, to[1] + (v[1] - pivot[1]) * k)
    out = {}
    for key, v in pose.items():
        if key in ("rump", "chest", "eye_at", "nostril") and v:
            out[key] = tuple(round(c) for c in pt(v))
        elif key.endswith("_hoof"):
            out[key] = (round(pt(v)[0]), to[1] - 2)
        elif key in ("belly", "haunch", "shoulder", "skull", "throat") and v:
            x, y = pt(v[:2])
            out[key] = (x, y, v[2] * k, v[3] * k)
        elif key in ("near_hind", "far_hind", "near_front", "far_front"):
            out[key] = [(*pt(q[:2]), q[2] * k) for q in v]
        elif key in ("neck", "muzzle", "jaw", "ear_near", "ear_far"):
            out[key] = tuple(pt(q) for q in v)
        elif key == "tail":
            out[key] = (pt(v[0]), pt(v[1]), v[2])
        elif key == "barrel_r":
            out[key] = v * k
        elif key in ("neck_r", "barrel_rr", "ear_r"):
            out[key] = tuple(r * k for r in v)
        elif key == "mouth_line" and v:
            out[key] = tuple(tuple(round(c) for c in pt(q)) for q in v)
        elif key == "teeth":
            out[key] = [(n, tuple(round(c) for c in pt(q))) for n, q in v]
        elif key == "mange":
            out[key] = [(m, a * k, b * k, c * k, d if m == "s" else d * k) for m, a, b, c, d in v]
        else:
            out[key] = v
    out.setdefault("mange", [(m, a * k, b * k, c * k, d if m == "s" else d * k) for m, a, b, c, d in MANGE])
    return out


# --------------------------------------------------------------------------- grid io
def palette_json():
    out = []
    for key, name, rgba, accent in PALETTE:
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def blank(w: int, h: int) -> list[str]:
    return ["." * w for _ in range(h)]


def build_grid() -> dict:
    frames = []
    for i, ticks in enumerate(TICKS):
        ms = round((ticks if ticks else 6) * 1000 / TICK_HZ)
        body = render(i)
        frames.append({"duration_ms": ms, "cels": {"body": body}})
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def build_size_study() -> dict:
    frames = []
    for height in (56, 64, 72):
        c = Canvas(96, 80)
        llama(c, scaled(IDLE, height / 64))
        frames.append({"duration_ms": 200, "cels": {"body": c.rows()}})
    return {"format": "pixel-grid-v1", "canvas": [96, 80], "pivot": [44, 79], "palette": palette_json(),
            "layers": ["body"], "frames": frames,
            "tags": [{"name": f"h{h}", "from": i, "to": i, "direction": "forward"} for i, h in enumerate((56, 64, 72))]}


MANIFEST = ROOT / "assets/manifests/llama-01.json"
GROUNDED_TAGS = ["idle", "walk", "attack_tell", "attack_swing", "hurt", "down"]
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once", "hurt": "once",
            "down": "hold", "spit": "loop"}


def build_manifest() -> dict:
    tag_of = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
    frames = [{"index": n, "tag": tag_of[n], "ticks": t, "duration_ms": round((t if t else 6) * 1000 / TICK_HZ),
               "author": "key" if n in KEY_FRAMES else "sonnet"} for n, t in enumerate(TICKS)]
    return {
        "schema_version": 1,
        "id": "llama_bad_llama_h64_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/llama/llama-01.ase",
        "grid_source": "assets/source/characters/llama/llama-01.grid.json",
        "output_atlas": "generated/art/llama-01.png",
        "output_metadata": "generated/art/llama-01.json",
        "palette_id": "llama-floor01-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Bad Llama; runtime role IP-neutral ranged spitter)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("LLAMA-01 Bad Llama at the decided 64 px height: 7 frozen Opus keys (atlas frames 0, 3, 8, 9, 11, 14, 15, "
                       "fixture tests/fixtures/llama-01_keys.json) plus 10 in-betweens (1, 2, 4, 5, 6, 7, 10, 12, 13, 16) "
                       "rasterized by scripts/author_llama01.py from the same parametric rig and literal stamps; layer body"),
            "generator": ("Design, size decision, palette and keys: Claude Opus. In-betweens, manifest, .ase, validators and "
                          "evidence: Claude Sonnet. Review: a separate Claude Opus agent (pending). No image-model raster "
                          "output, no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/bad-llama-c-plus-a-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "llama": {
            "status": "Keys frozen, in-betweens drawn, validators pass; independent Opus review pending; Bevy integration pending; 17 frames, 7 of them Opus keys; not wired into any Bevy binary",
            "spec": "docs/02-art/llama-01-spec.md",
            "opaque_height_idle": 64,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": 100,
            "grounded_tags": GROUNDED_TAGS,
            "airborne_tags": [],
            "free_tags": ["spit"],
            "playback": PLAYBACK,
            "key_frames": list(KEY_FRAMES),
            "inbetween_frames": list(INBETWEENS),
            "key_fixture": "tests/fixtures/llama-01_keys.json",
            "attack_ticks": {"tell": [0, 29], "spit_release": [30, 35], "recover": [36, 49]},
            "frames": frames,
        },
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def preview(grid: dict, study: dict, out: Path) -> None:
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}

    def img(rows, scale):
        h, w = len(rows), len(rows[0])
        im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        for y, r in enumerate(rows):
            for x, ch in enumerate(r):
                if ch != ".":
                    im.putpixel((x, y), pal[ch])
        return im.resize((w * scale, h * scale), Image.NEAREST)

    lineup = Image.open(ROOT / "docs/evidence/llama-01/packet/scale_lineup.png").convert("RGBA")
    sheet = Image.new("RGBA", (max(96 * 4 * 4, lineup.width + 96 * 3 * 3 + 40), 72 * 4 * 2 + 80 * 3 + lineup.height + 40),
                      (46, 30, 28, 255))
    for n, f in enumerate(KEY_FRAMES):
        x, y = (n % 4) * 96 * 4, (n // 4) * 72 * 4
        bg = Image.new("RGBA", (96 * 4, 72 * 4), (34, 40, 54, 255) if n % 2 else (92, 52, 38, 255))
        sheet.paste(bg, (x, y))
        sheet.alpha_composite(img(grid["frames"][f]["cels"]["body"], 4), (x, y))
    y0 = 72 * 4 * 2 + 20
    sheet.alpha_composite(lineup, (0, y0))
    for i, fr in enumerate(study["frames"]):
        sheet.alpha_composite(img(fr["cels"]["body"], 3), (lineup.width + 20 + i * 96 * 3, y0))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)


def report(rows: list[str], label: str) -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    import pixel_grid_to_ase as pg
    flat = [list(r) for r in rows]
    orphans, isolated = pg._stray_pixels(flat, ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    used = sorted({ch for r in rows for ch in r} - {"."})
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, w {max(xs) - min(xs) + 1}) "
          f"orphans {len(orphans)} isolated {len(isolated)} keys {''.join(used)}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "size-study", "mirror-lit"))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    if a.mode == "keys":
        data = build_grid()
        for f in range(len(TICKS)):
            report(data["frames"][f]["cels"]["body"], f"frame {f:2d}")
        if a.preview:
            preview(data, build_size_study(), a.preview)
        target, text = GRID, dump(data)
        manifest_text = json.dumps(build_manifest(), indent=2) + "\n"
    elif a.mode == "mirror-lit":
        # Lit scenes mirror the right-facing frames for enemies that face Carl on their left, which would put the
        # torch highlights on the right (wrong) side of the flipped llama. Render the same poses with the torch
        # vector mirrored in x; flipping that grid puts the highlights back toward the upper-left torch.
        global LIGHT
        LIGHT = (-LIGHT[0], LIGHT[1])
        data = build_grid()
        target, text = MIRROR_LIT, dump(data)
    else:
        data = build_size_study()
        for i, fr in enumerate(data["frames"]):
            report(fr["cels"]["body"], f"study {i}")
        target, text = SIZE_STUDY, dump(data)
    if a.check:
        if a.mode == "mirror-lit":
            bad = [] if target.read_text() == text else ["mirror-lit grid"]
        elif a.mode == "keys":
            old = json.loads(GRID.read_text())
            bad = [f for f in range(len(TICKS)) if old["frames"][f]["cels"] != data["frames"][f]["cels"]]
            if not MANIFEST.is_file() or MANIFEST.read_text() != manifest_text:
                bad.append("manifest")
        else:
            bad = [] if target.read_text() == text else ["size study"]
        if bad:
            print(f"differs: {bad}")
            return 1
        print("up to date")
        return 0
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    print(f"wrote {target.relative_to(ROOT)}")
    if a.mode == "keys":
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        MANIFEST.write_text(manifest_text, encoding="utf-8")
        print(f"wrote {MANIFEST.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
