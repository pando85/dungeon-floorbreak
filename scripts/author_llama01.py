#!/usr/bin/env python3
"""LLAMA-01 authoring record: Bad Llama palette, rule-based key poses, size study and spit key.

Usage:

    python3 scripts/author_llama01.py keys [--check] [--preview PNG]
    python3 scripts/author_llama01.py size-study [--check]

``keys`` (re)writes the key frames of ``assets/source/characters/llama/llama-01.grid.json``
(frames 0, 3, 8, 9, 11, 14, 15; see ``docs/02-art/llama-01-spec.md`` section 7). Every key is
rasterized natively from one parametric rig (capsules and ellipses at pixel centres, volumetric
ramp shading from an upper-left torch key, shag fringes, mange patches, a 1 px outline outside
the fill) plus literal face/teeth stamps. Non-key frames that are still empty are written empty;
frames already drawn by the execution tier are kept untouched, so the in-betweens survive a
re-run. ``--check`` compares the keys instead of writing (exit 1 on a difference).

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


def tuft(s: float, phase: float = 0.0) -> float:
    return (0.0, 1.0, 2.0, 1.0)[int(math.floor(s + phase)) % 4] / 2.0


class Part:
    def __init__(self, kind, geo, material, group="main", shag=None, glow=0.0, name=""):
        self.kind, self.geo, self.material, self.group = kind, geo, material, group
        self.shag, self.glow, self.name = shag, glow, name

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
            return max(-1, min(1, v)), max(-1, min(1, q)), t
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


MANGE = ((1, -2, 5.5, 2.6), (16, 4, 4.5, 1.8), (-3, 8, 2.4, 3.6))


def llama(c: Canvas, p: dict) -> None:
    far, main = [], []
    for leg in ("far_hind", "far_front"):
        pts = p[leg]
        for (x1, y1, r1), (x2, y2, r2) in zip(pts, pts[1:]):
            far.append(Part("cap", ((x1, y1), (x2, y2), r1, r2), "far", "far"))
        hx = p[leg + "_hoof"]
        far.append(Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof", "far"))
    ear = p["ear_far"]
    far.append(Part("cap", (ear[0], ear[1], 1.7, 1.1), "ear_in", "far"))
    for part in far:
        c.draw(part)
    c.outline("far")

    tail = p["tail"]
    main.append(Part("cap", (tail[0], tail[1], 3.0, 3.4), "fur", shag=((0, 0), 1.8)))
    rump, chest = p["rump"], p["chest"]
    main.append(Part("cap", (rump, chest, p["barrel_r"], p["barrel_r"] - 0.5), "fur", shag=((0, 1), 1.4)))
    main.append(Part("ell", p["belly"], "fur", shag=((0, 1), 3.0)))
    main.append(Part("ell", p["haunch"], "fur", shag=((-1, 0.3), 1.5)))
    main.append(Part("ell", p["shoulder"], "fur", shag=((0.6, 1), 2.2)))
    for leg in ("near_hind", "near_front"):
        pts = p[leg]
        for i, ((x1, y1, r1), (x2, y2, r2)) in enumerate(zip(pts, pts[1:])):
            mat = "fur" if i == 0 else "leg"
            main.append(Part("cap", ((x1, y1), (x2, y2), r1, r2), mat, shag=((0, 1), 1.0) if i == 0 else None))
        hx = p[leg + "_hoof"]
        main.append(Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof"))
    na, nb = p["neck"]
    nx, ny = nb[0] - na[0], nb[1] - na[1]
    ln = math.hypot(nx, ny)
    back = (ny / ln, -nx / ln) if ny < 0 else (-ny / ln, nx / ln)   # mane side: the back of the neck
    main.append(Part("cap", (na, nb, p["neck_r"][0], p["neck_r"][1]), "fur", shag=(back, 2.4), glow=p.get("glow", 0.0)))
    if p.get("throat"):
        main.append(Part("ell", p["throat"], "fur", glow=1.0, name="throat"))
    main.append(Part("ell", p["skull"], "fur"))
    ja, jb = p["jaw"]
    main.append(Part("cap", (ja, jb, 2.6, 2.0), "muzzle"))
    ma, mb = p["muzzle"]
    main.append(Part("cap", (ma, mb, *p.get("muzzle_r", (3.6, 2.9))), "muzzle"))
    if p.get("mouth"):
        main.append(Part("tri", p["mouth"], "mouth", glow=p.get("mouth_glow", 0.0)))
    ear = p["ear_near"]
    main.append(Part("cap", (ear[0], ear[1], 1.9, 1.3), "fur"))
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
    for dx, dy, ex, ey in p.get("mange", MANGE):
        c.mange(rx0 + dx, ry0 + dy, ex, ey)
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


def spit(c: Canvas) -> None:
    """Lava spit glob: hot core, cooling skin, a dark crust tail and one drip, travelling right."""
    parts = [
        Part("cap", ((88.0, 26.5), (80.0, 27.5), 2.6, 1.0), "fur", group="spit"),
        Part("ell", (89.5, 26.5, 4.2, 3.2), "fur", group="spit"),
        Part("cap", ((84.5, 28.5), (83.5, 31.5), 1.0, 0.8), "fur", group="spit"),
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
            d = math.hypot((x + 0.5 - 90.2) / 3.4, (y + 0.5 - 26.0) / 2.4)
            c.px[y][x] = "Y" if d < 0.5 else "O" if d < 0.85 else "R" if d < 1.25 or x > 86 else "r"
    c.outline("spit")
    c.stamp(80, 27, ["a"])
    c.stamp(83, 31, ["a"])
    c.cleanup()


# --------------------------------------------------------------------------- key poses
def hooves(**kw):
    return {k + "_hoof": v for k, v in kw.items()}


IDLE = {
    "rump": (27, 39), "chest": (53, 41), "barrel_r": 8.5,
    "belly": (41, 44, 14, 7.5), "haunch": (27, 43, 8, 9), "shoulder": (54, 44, 6, 8),
    "near_hind": [(29, 46, 4.5), (31, 54, 3.0), (27, 59, 2.0), (28, 67, 1.5)],
    "far_hind": [(35, 46, 3.5), (36, 54, 2.5), (33, 59, 1.8), (34, 67, 1.4)],
    "near_front": [(53, 47, 3.5), (52, 54, 2.4), (53, 60, 1.8), (54, 67, 1.5)],
    "far_front": [(58, 47, 3.0), (58.5, 54, 2.2), (60, 60, 1.7), (61, 67, 1.4)],
    **hooves(near_hind=(28, 69), far_hind=(34, 69), near_front=(54, 69), far_front=(61, 69)),
    "neck": ((56, 37), (66, 19)), "neck_r": (6.2, 4.4),
    "skull": (67, 16, 5.4, 4.4),
    "muzzle": ((69, 17), (77, 19)), "jaw": ((69, 23), (76, 25)), "mouth": ((70, 21.5), (80, 21.5), (77, 24.5)),
    "ear_near": ((65, 12.5), (61, 10.4)), "ear_far": ((67, 12.5), (64, 10.6)),
    "tail": ((21, 34), (14, 28), (-0.4, -1)),
    "mouth_line": None, "teeth": [("upper", (73, 20)), ("lower", (74, 24))],
    "eye": "angry", "eye_at": (65, 14), "nostril": (78, 16),
}


def shifted(base: dict, **over) -> dict:
    out = dict(base)
    out.update(over)
    return out


WALK_CONTACT = shifted(
    IDLE,
    rump=(27, 40), chest=(53, 42), belly=(41, 45, 14, 7.5), haunch=(27, 44, 8, 9), shoulder=(54, 45, 6, 8),
    near_front=[(54, 48, 3.5), (57, 54, 2.4), (60, 60, 1.8), (62, 67, 1.5)],
    far_front=[(57, 48, 3.0), (57, 54, 2.2), (57.5, 60, 1.7), (58, 67, 1.4)],
    near_hind=[(29, 47, 4.5), (28, 54, 3.0), (23, 59, 2.0), (22, 67, 1.5)],
    far_hind=[(35, 47, 3.5), (37, 54, 2.5), (34, 59, 1.8), (35, 67, 1.4)],
    **hooves(near_hind=(21, 69), far_hind=(35, 69), near_front=(62, 69), far_front=(58, 69)),
    neck=((57, 38), (67, 20)), skull=(68, 17, 5.4, 4.4), muzzle=((70, 18), (78, 20)), jaw=((70, 24), (77, 26)),
    mouth=((71, 22.5), (81, 22.5), (78, 25.5)),
    ear_near=((66, 13.5), (62, 11.4)), ear_far=((68, 13.5), (65, 11.6)),
    tail=((21, 35), (14, 31), (-0.6, -1)),
    teeth=[("upper", (74, 21)), ("lower", (75, 25))], eye_at=(66, 15), nostril=(79, 17),
)

TELL = shifted(
    IDLE,
    rump=(25, 41), chest=(51, 38), belly=(39, 44, 13.5, 7.0), haunch=(25, 45, 8, 9), shoulder=(52, 42, 6, 8),
    near_front=[(51, 45, 3.5), (54, 52, 2.4), (57, 59, 1.8), (59, 67, 1.5)],
    far_front=[(56, 45, 3.0), (59, 52, 2.2), (62, 59, 1.7), (64, 67, 1.4)],
    near_hind=[(27, 48, 4.5), (30, 55, 3.0), (25, 60, 2.0), (26, 67, 1.5)],
    far_hind=[(33, 48, 3.5), (35, 55, 2.5), (31, 60, 1.8), (32, 67, 1.4)],
    **hooves(near_hind=(26, 69), far_hind=(32, 69), near_front=(59, 69), far_front=(64, 69)),
    neck=((54, 34), (59, 12)), neck_r=(6.0, 4.2), glow=1.0, throat=(60.5, 22.5, 3.2, 6.0),
    skull=(60, 9, 4.8, 4.2), muzzle=((62, 10), (71, 12)), jaw=((62, 13), (70, 14.5)), muzzle_r=(3.8, 2.8),
    ear_near=((57, 6), (52, 4)), ear_far=((59, 6), (55, 4.2)),
    tail=((19, 36), (12, 27), (-0.5, -1)),
    mouth=None, mouth_line=None, teeth=[("clench", (68, 11))],
    eye="tell", eye_at=(58, 8), nostril=(71, 9),
)

SPIT = shifted(
    IDLE,
    rump=(29, 39), chest=(56, 41), belly=(43, 44, 14, 7.5), haunch=(29, 43, 8, 9), shoulder=(57, 44, 6, 8),
    near_front=[(56, 47, 3.5), (58, 54, 2.4), (60, 60, 1.8), (62, 67, 1.5)],
    far_front=[(61, 47, 3.0), (62, 54, 2.2), (64, 60, 1.7), (65, 67, 1.4)],
    near_hind=[(31, 46, 4.5), (28, 54, 3.0), (22, 59, 2.0), (20, 67, 1.5)],
    far_hind=[(37, 46, 3.5), (36, 54, 2.5), (31, 59, 1.8), (29, 67, 1.4)],
    **hooves(near_hind=(19, 69), far_hind=(28, 69), near_front=(62, 69), far_front=(65, 69)),
    neck=((59, 37), (74, 25)), neck_r=(5.5, 3.8), glow=0.67,
    skull=(77, 23, 4.6, 3.8), muzzle=((79, 23.5), (87, 23.5)), jaw=((79, 27), (86, 31)), muzzle_r=(3.2, 2.5),
    mouth=((79, 26), (89, 25), (87, 31)), mouth_glow=1.0,
    ear_near=((75, 20), (69, 18)), ear_far=((77, 20), (71, 18.4)),
    tail=((22, 34), (13, 31), (-1, -0.3)),
    mouth_line=None, teeth=[("upper", (85, 25)), ("lower", (83, 29))],
    eye="angry", eye_at=(75, 21), nostril=(87, 20),
    extra=[(89, 25, ["Y", "O"]), (90, 26, ["R"])],
)

HURT = shifted(
    IDLE,
    rump=(24, 41), chest=(51, 40), belly=(38, 45, 13.5, 7.5), haunch=(24, 45, 8, 9), shoulder=(52, 43, 6, 8),
    near_front=[(52, 47, 3.5), (55, 52, 2.4), (57, 57, 1.8), (55, 62, 1.5)],
    far_front=[(57, 47, 3.0), (58, 54, 2.2), (59, 60, 1.7), (60, 67, 1.4)],
    near_hind=[(26, 48, 4.5), (28, 55, 3.0), (24, 60, 2.0), (25, 67, 1.5)],
    far_hind=[(32, 48, 3.5), (34, 55, 2.5), (30, 60, 1.8), (31, 67, 1.4)],
    **hooves(near_hind=(25, 69), far_hind=(31, 69), near_front=(55, 63), far_front=(60, 69)),
    neck=((53, 37), (54, 15)), neck_r=(5.5, 3.8),
    skull=(54, 12, 4.6, 3.8), muzzle=((56, 12), (63, 7)), jaw=((56, 15), (63, 12)),
    mouth=((57, 13), (64, 8), (63, 12)),
    ear_near=((52, 10), (46, 9)), ear_far=((54, 10), (48, 9.6)),
    tail=((18, 38), (13, 43), (-1, 0.4)),
    mouth_line=None, teeth=[("upper", (61, 8))],
    eye="shut", eye_at=(52, 11), nostril=None,
    mange=((3, -4, 4.0, 2.2), (13, -1, 3.0, 2.0), (21, -5, 2.6, 1.8), (-2, 7, 2.6, 3.0)),
)

DEAD = shifted(
    IDLE,
    rump=(30, 60), chest=(54, 61), barrel_r=7.5, belly=(42, 64, 13, 6.5), haunch=(29, 62, 7, 6.5), shoulder=(54, 63, 5, 5.5),
    near_front=[(56, 65, 2.8), (63, 67.5, 2.0), (69, 68.5, 1.5)],
    far_front=[(55, 63, 2.4), (60, 64, 1.8)],
    near_hind=[(27, 64, 3.5), (21, 67.5, 2.2), (16, 68.5, 1.5)],
    far_hind=[(33, 62, 2.6), (29, 64, 1.8)],
    **hooves(near_hind=(14, 69), far_hind=(27, 64), near_front=(70, 69), far_front=(61, 64)),
    neck=((57, 60), (73, 64.5)), neck_r=(5.0, 3.6),
    skull=(76, 64.5, 4.4, 3.6), muzzle=((78, 65.5), (86, 67)), jaw=((78, 68.2), (85, 69.2)), muzzle_r=(3.0, 2.4),
    mouth=((79, 67), (87, 68), (85, 70)),
    ear_near=((74, 61.5), (69, 59.5)), ear_far=((76, 61.5), (71, 60)),
    tail=((24, 59), (17, 64), (-1, 0)),
    mouth_line=None, teeth=[("upper", (84, 67))],
    eye="dead", eye_at=(75, 63), nostril=(85, 65),
    mange=((3, -4, 4.0, 2.0), (13, -2, 3.0, 2.0), (21, -4, 2.6, 1.6)),
)

POSES = {0: IDLE, 3: WALK_CONTACT, 8: TELL, 9: SPIT, 11: HURT, 14: DEAD}


def render(frame: int) -> list[str]:
    c = Canvas(*CANVAS)
    if frame == 15:
        spit(c)
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
            out[key] = (pt(v[0]), pt(v[1]))
        elif key == "tail":
            out[key] = (pt(v[0]), pt(v[1]), v[2])
        elif key == "barrel_r":
            out[key] = v * k
        elif key == "neck_r":
            out[key] = (v[0] * k, v[1] * k)
        elif key == "mouth_line" and v:
            out[key] = tuple(tuple(round(c) for c in pt(q)) for q in v)
        elif key == "teeth":
            out[key] = [(n, tuple(round(c) for c in pt(q))) for n, q in v]
        elif key == "mange":
            out[key] = [(a * k, b * k, c * k, d * k) for a, b, c, d in v]
        else:
            out[key] = v
    out.setdefault("mange", [(a * k, b * k, c * k, d * k) for a, b, c, d in
                             ((3, -4, 4.0, 2.2), (13, -1, 3.0, 2.0), (21, -5, 2.6, 1.8), (-2, 7, 2.6, 3.0))])
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


def build_grid(existing: dict | None) -> dict:
    frames = []
    for i, ticks in enumerate(TICKS):
        ms = round((ticks if ticks else 6) * 1000 / TICK_HZ)
        if i in KEY_FRAMES:
            body = render(i)
        elif existing and i < len(existing["frames"]):
            body = existing["frames"][i]["cels"]["body"]
        else:
            body = blank(*CANVAS)
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
    ap.add_argument("mode", choices=("keys", "size-study"))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    if a.mode == "keys":
        existing = json.loads(GRID.read_text()) if GRID.exists() else None
        data = build_grid(existing)
        for f in KEY_FRAMES:
            report(data["frames"][f]["cels"]["body"], f"frame {f:2d}")
        if a.preview:
            preview(data, build_size_study(), a.preview)
        target, text = GRID, dump(data)
    else:
        data = build_size_study()
        for i, fr in enumerate(data["frames"]):
            report(fr["cels"]["body"], f"study {i}")
        target, text = SIZE_STUDY, dump(data)
    if a.check:
        if a.mode == "keys":
            old = json.loads(GRID.read_text())
            bad = [f for f in KEY_FRAMES if old["frames"][f]["cels"] != data["frames"][f]["cels"]]
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
