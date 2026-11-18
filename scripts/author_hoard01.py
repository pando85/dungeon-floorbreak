#!/usr/bin/env python3
"""Author HOARD-01 The Hoarder (Floor 01 boss) keys as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/hoard-01-spec.md``. The keys are built from geometric rules (ellipses, tapered
capsules, polygons), shaded by one fixed light vector and finished with explicit detail pixels. No raster
image-model output, no tracing or sampling of concept pixels.

Keys 0, 3, 8, 10, 12 and 15 are authored here; frames 1, 2, 4-7, 9, 11, 13 and 14 hold placeholder copies of
the nearest key until the execution tier draws the in-betweens (spec section 8).

    python3 scripts/author_hoard01.py               # write grid + .ase
    python3 scripts/author_hoard01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_hoard01.py --scale DIR   # write key0 1x / 4x scale images next to Carl
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import pixel_grid_to_ase as pg  # noqa: E402

HOARD_DIR = ROOT / "assets/source/characters/hoarder"
GRID = HOARD_DIR / "hoard-01.grid.json"
ASE = HOARD_DIR / "hoard-01.ase"
CARL_GRID = ROOT / "assets/source/characters/carl/carl_h52.grid.json"
CANVAS = (160, 128)
PIVOT = (70, 127)
TICK_HZ = 60

# key, name, rgba, accent (accents are exempt from the orphan rule)
PALETTE = [
    ("K", "outline", "#1c1218ff", False),
    ("d", "skin_deep", "#5c2e2aff", False),
    ("s", "skin_shade", "#94523fff", False),
    ("m", "skin_mid", "#c67c5aff", False),
    ("h", "skin_light", "#e6a982ff", False),
    ("x", "scar_dark", "#5e3442ff", False),
    ("y", "scar_light", "#8f5a5eff", True),
    ("a", "hair_deep", "#241e2cff", False),
    ("b", "hair_mid", "#3c3550ff", False),
    ("c", "hair_rim", "#5a6078ff", False),
    ("t", "shirt_shade", "#6e604eff", False),
    ("u", "shirt_mid", "#a39276ff", False),
    ("v", "shirt_light", "#d4c4a2ff", False),
    ("p", "pants_deep", "#262944ff", False),
    ("q", "pants_mid", "#3e4468ff", False),
    ("r", "pants_light", "#626a94ff", False),
    ("g", "grime", "#5a4a2cff", False),
    ("w", "eye_white", "#ece2caff", True),
    ("n", "tin_scrap", "#94a4a8ff", True),
]
ACCENTS = {p[0] for p in PALETTE if p[3]}
RAMPS = {"skin": "smh", "shirt": "tuv", "pants": "pqr", "hair": "abc"}
DARKER = {"h": "m", "m": "s", "s": "d", "d": "d", "v": "u", "u": "t", "t": "g", "r": "q", "q": "p", "p": "p",
          "c": "b", "b": "a", "a": "a", "y": "x", "x": "x", "g": "g"}
_l = (-0.5, -0.62, 0.6)
LIGHT = tuple(c / math.sqrt(sum(v * v for v in _l)) for c in _l)   # torch key upper left, same for every key

TAGS = [("idle", 0, 2), ("walk", 3, 6), ("attack_tell", 7, 8), ("attack_swing", 9, 11), ("hurt", 12, 12),
        ("down", 13, 15)]
TICKS = [16, 12, 16, 8, 8, 8, 8, 8, 26, 3, 6, 24, 12, 8, 12, None]
KEY_FRAMES = (0, 3, 8, 10, 12, 15)


# --------------------------------------------------------------------------- primitives
def ellipse(cx, cy, rx, ry) -> dict:
    out = {}
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            nx, ny = (x - cx) / rx, (y - cy) / ry
            if nx * nx + ny * ny <= 1.0:
                out[(x, y)] = (nx, ny)
    return out


def capsule(x0, y0, r0, x1, y1, r1) -> dict:
    out = {}
    dx, dy = x1 - x0, y1 - y0
    l2 = dx * dx + dy * dy or 1.0
    rmax = max(r0, r1)
    for y in range(int(min(y0, y1) - rmax) - 1, int(max(y0, y1) + rmax) + 2):
        for x in range(int(min(x0, x1) - rmax) - 1, int(max(x0, x1) + rmax) + 2):
            t = max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
            cx, cy, r = x0 + t * dx, y0 + t * dy, r0 + (r1 - r0) * t
            d = math.hypot(x - cx, y - cy)
            if d <= r:
                out[(x, y)] = ((x - cx) / r, (y - cy) / r)
    return out


def polygon(pts, centre, radii) -> dict:
    out = {}
    ys = [p[1] for p in pts]
    cx, cy = centre
    rx, ry = radii
    for y in range(int(min(ys)), int(max(ys)) + 1):
        xs = []
        for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
            if (ay <= y < by) or (by <= y < ay):
                xs.append(ax + (y - ay) * (bx - ax) / (by - ay))
        xs.sort()
        for a, b in zip(xs[::2], xs[1::2]):
            for x in range(math.ceil(a), math.floor(b) + 1):
                out[(x, y)] = (max(-1, min(1, (x - cx) / rx)), max(-1, min(1, (y - cy) / ry)))
    return out


def line(x0, y0, x1, y1) -> dict:
    n = max(abs(x1 - x0), abs(y1 - y0), 1)
    return {(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)): (0.0, 0.0) for i in range(n + 1)}


def union(*shapes) -> dict:
    out = {}
    for s in shapes:
        out.update(s)
    return out


def lum(n) -> float:
    nx, ny = n
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    d = nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]
    return 0.3 + 0.7 * max(0.0, d)


# --------------------------------------------------------------------------- canvas
class Canvas:
    def __init__(self, w: int = CANVAS[0], h: int = CANVAS[1]):
        self.w, self.h = w, h
        self.owner: dict = {}    # pixel -> part order
        self.family: dict = {}   # pixel -> material family
        self.normal: dict = {}
        self.parts: list = []
        self.key: dict = {}

    def part(self, name: str, family: str, px: dict, clip=None) -> None:
        idx = len(self.parts)
        self.parts.append(name)
        for p, n in px.items():
            if not (0 <= p[0] < self.w and 0 <= p[1] < self.h):
                continue
            if clip and not clip(*p):
                continue
            self.owner[p], self.family[p], self.normal[p] = idx, family, n

    def erase(self, pred) -> None:
        for p in [p for p in self.owner if pred(*p)]:
            del self.owner[p], self.family[p], self.normal[p]

    def shade(self, hair_sheen=None) -> None:
        for p, fam in self.family.items():
            v = lum(self.normal[p])
            ramp = RAMPS[fam]
            if fam == "hair":
                k = "a"
                if v > 0.8:
                    k = "b"
                if hair_sheen and hair_sheen(*p) and v > 0.55:
                    k = "c" if v > 0.78 else "b"
            else:
                k = ramp[0] if v < 0.5 else ramp[1] if v < 0.8 else ramp[2]
            self.key[p] = k
        # cast shadow: a front part of another family up-left of a pixel darkens it one step
        snap = dict(self.key)
        for (x, y), k in snap.items():
            mine = self.owner[(x, y)]
            for q in ((x - 1, y - 1), (x, y - 1), (x, y - 2), (x - 1, y - 2)):
                o = self.owner.get(q)
                if o is not None and o > mine and self.family[q] != self.family[(x, y)]:
                    self.key[(x, y)] = DARKER[k]
                    break
        # internal separation: the front part's edge against a different family goes to the outline key
        for (x, y) in list(self.key):
            mine = self.owner[(x, y)]
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                o = self.owner.get(q)
                if o is not None and o < mine and self.family[q] != self.family[(x, y)]:
                    self.key[(x, y)] = "K"
                    break

    def put(self, x: int, y: int, k: str) -> None:
        if (x, y) in self.owner:
            self.key[(x, y)] = k

    def finish(self) -> list[str]:
        outer = {(x, y) for (x, y) in self.key
                 if any(q not in self.owner for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)))}
        for p in outer:
            self.key[p] = "K"
        # 1 px blue-grey rim on the top and back edges of the hair: reads against the dark navy arch
        for (x, y), fam in self.family.items():
            if fam == "hair" and (x, y) not in outer and self.key[(x, y)] != "K" and \
                    ((x, y - 1) in outer or (x - 1, y) in outer):
                self.key[(x, y)] = "c"
        rows = [["."] * self.w for _ in range(self.h)]
        for (x, y), k in self.key.items():
            rows[y][x] = k
        return ["".join(r) for r in despeckle(rows)]


def despeckle(rows: list[list[str]], passes: int = 4) -> list[list[str]]:
    """Replace non-accent pixels that have no same-key 8-neighbour with the commonest opaque neighbour."""
    h, w = len(rows), len(rows[0])
    for _ in range(passes):
        changed = False
        for y in range(h):
            for x in range(w):
                c = rows[y][x]
                if c == "." or c in ACCENTS:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if c in around:
                    continue
                opaque = [n for n in around if n != "." and n not in ACCENTS]
                if opaque:
                    rows[y][x] = max(sorted(set(opaque)), key=opaque.count)
                    changed = True
        if not changed:
            break
    return rows


# --------------------------------------------------------------------------- rig: affine transforms
# Every key is the key-0 construction moved by a pose: the body group (torso, hips, sleeves, details) by one
# rigid transform about the hip centre, the head group (head, hair, face) by a second transform about the neck,
# limbs by explicit joint points. The identity pose reproduces key 0 exactly.
BP = (68, 100)   # body pivot: hip centre
HP = (82, 40)    # head pivot: neck


def rig(px, py, ang=0.0, dx=0.0, dy=0.0, sx=1.0):
    ca, sa = math.cos(math.radians(ang)), math.sin(math.radians(ang))
    a, b, d, e = ca * sx, -sa, sa * sx, ca
    return (a, b, px + dx - (a * px + b * py), d, e, py + dy - (d * px + e * py))


def head_rig(body, ang, dx, dy):
    hx, hy = apply(body, *HP)
    ca, sa = math.cos(math.radians(ang)), math.sin(math.radians(ang))
    return (ca, -sa, hx + dx - (ca * HP[0] - sa * HP[1]), sa, ca, hy + dy - (sa * HP[0] + ca * HP[1]))


def apply(m, x, y):
    return (m[0] * x + m[1] * y + m[2], m[3] * x + m[4] * y + m[5])


def inv(m):
    a, b, c, d, e, f = m
    det = a * e - b * d
    ia, ib, id_, ie = e / det, -b / det, -d / det, a / det
    return (ia, ib, -(ia * c + ib * f), id_, ie, -(id_ * c + ie * f))


def rotn(m, nx, ny):
    return (m[0] * nx + m[1] * ny, m[3] * nx + m[4] * ny)


def t_ellipse(m, cx, cy, rx, ry) -> dict:
    mi = inv(m)
    pts = [apply(m, cx + sx * rx, cy + sy * ry) for sx in (-1, 1) for sy in (-1, 1)]
    out = {}
    for y in range(int(min(p[1] for p in pts)) - 2, int(max(p[1] for p in pts)) + 3):
        for x in range(int(min(p[0] for p in pts)) - 2, int(max(p[0] for p in pts)) + 3):
            lx, ly = apply(mi, x, y)
            nx, ny = (lx - cx) / rx, (ly - cy) / ry
            if nx * nx + ny * ny <= 1.0:
                out[(x, y)] = rotn(m, nx, ny)
    return out


def t_capsule(m, x0, y0, r0, x1, y1, r1) -> dict:
    a, b = apply(m, x0, y0), apply(m, x1, y1)
    return capsule(a[0], a[1], r0, b[0], b[1], r1)


def t_polygon(m, pts, centre, radii) -> dict:
    return polygon([apply(m, *p) for p in pts], apply(m, *centre), radii)


def t_line(m, x0, y0, x1, y1) -> dict:
    a, b = apply(m, x0, y0), apply(m, x1, y1)
    return line(round(a[0]), round(a[1]), round(b[0]), round(b[1]))


def in_polygon(pts, x, y) -> bool:
    xs = []
    for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
        if (ay <= y < by) or (by <= y < ay):
            xs.append(ax + (y - ay) * (bx - ax) / (by - ay))
    xs.sort()
    return any(a <= x <= b for a, b in zip(xs[::2], xs[1::2]))


def rot_about(cx, cy, rx, ry, ang):
    ca, sa = math.cos(math.radians(ang)), math.sin(math.radians(ang))
    return (cx + (rx * ca - ry * sa), cy + (rx * sa + ry * ca))


def swung(pts, swing):
    """Hair points shifted by ``swing`` px, weighted from 0 at the nape (y 34) to 1 at y 80 (head-local)."""
    return [(x + swing * max(0.0, min(1.0, (y - 34) / 46)), y) for x, y in pts]


# --------------------------------------------------------------------------- the Hoarder
def hem_y(x) -> float:
    """Ragged shirt hem: a 6 px saw-tooth plus one deep rip at the belly front (body-local)."""
    y = 95 + abs((x % 6) - 3)
    if 83 <= x <= 90:
        y = min(y, 88 + abs(x - 86))
    return y


NOTCH = (0, 2, 4, 1, 0, 3, 5, 2, 0, 1, 4, 2, 0, 3)
LOCK_GROUPS = (   # head-local strand groups of the back hair over the near shoulder: uneven width, length and spacing
    [(43, 38), (53, 38), (54, 50), (52, 60), (51, 70), (49, 66), (47, 74), (45, 67), (43, 58), (42, 48)],
    [(57, 38), (61, 38), (61, 56), (60, 72), (59, 87), (58, 74), (57, 58)],
    [(66, 38), (74, 40), (74, 50), (72, 58), (70, 63), (68, 59), (66, 64), (65, 52)],
    [(52, 62), (54, 62), (55, 72), (53, 80), (52, 70)],
)
NEAR_FINGERS = (((-5, 3), (-7, 10)), ((-2, 4), (-3, 12)), ((2, 4), (3, 12)), ((5, 2), (7, 9)))
NEAR_THUMB = ((5, -3), (9, 2))
FAR_FINGERS = (((-4, 3), (-6, 9)), ((-1, 4), (-1, 11)), ((2, 4), (4, 10)), ((5, 1), (8, 6)))
FAR_THUMB = ((-4, -3), (-7, 2))
NEAR_FOOT = ((-12, 11), (-12, 5), (-6, 2), (6, 1), (12, 4), (14, 11))
FAR_FOOT = ((-8, 11), (-8, 4), (-2, 1), (8, 2), (15, 6), (17, 11))
NEAR_TOES = (7, 10, 12)
FAR_TOES = (9, 12, 14)
BURN_CLUSTERS = (((77, 25), (78, 25)), ((77, 31),), ((79, 32), (80, 32)), ((77, 35), (78, 35)), ((81, 26),))
STAINS = ((60, 70, 3, 2), (76, 63, 2, 3), (90, 74, 3, 2), (64, 84, 4, 2), (54, 60, 2, 2), (95, 58, 2, 2),
          (72, 90, 2, 2))
TEAR_IN = [(88, 80), (95, 77), (97, 80), (93, 84), (89, 85)]
TEAR_OUT = [(86, 80), (95, 75), (98, 80), (94, 86), (88, 86)]
FACES = {
    # head-local pixels: eyes (two whites 3 px apart; the burned eye keeps a white, no lid or brow), nose, mouth
    "idle": ((79, 29, "w"), (80, 29, "w"), (81, 29, "K"), (81, 28, "K"), (79, 30, "x"), (80, 30, "x"),
             (84, 29, "w"), (85, 29, "w"), (86, 29, "K"), (84, 28, "K"), (85, 28, "K"), (86, 28, "K"),
             (84, 26, "K"), (85, 25, "K"), (86, 25, "K"), (87, 26, "K"),
             (87, 30, "h"), (87, 31, "h"), (87, 33, "s"), (88, 33, "s"), (86, 33, "s"),
             (84, 36, "K"), (85, 36, "K"), (86, 36, "K"), (84, 37, "d"), (85, 37, "d"), (86, 37, "d"),
             (84, 38, "s"), (85, 38, "s"), (86, 38, "s")),
    # the tell: whites enlarged, brow up, mouth wide with teeth (variant C roar)
    "roar": ((79, 28, "w"), (80, 28, "w"), (79, 29, "w"), (80, 29, "w"), (81, 29, "K"), (81, 28, "K"),
             (84, 28, "w"), (85, 28, "w"), (84, 29, "w"), (85, 29, "w"), (86, 29, "K"), (86, 28, "K"),
             (84, 27, "K"), (85, 27, "K"), (84, 24, "K"), (85, 24, "K"), (86, 24, "K"), (87, 25, "K"),
             (87, 31, "h"), (87, 33, "s"), (88, 33, "s"),
             (83, 35, "K"), (84, 35, "w"), (85, 35, "w"), (86, 35, "w"), (87, 35, "K"),
             (83, 36, "K"), (84, 36, "d"), (85, 36, "d"), (86, 36, "d"), (87, 36, "K"),
             (83, 37, "K"), (84, 37, "d"), (85, 37, "d"), (86, 37, "d"), (87, 37, "K"),
             (83, 38, "K"), (84, 38, "w"), (85, 38, "d"), (86, 38, "w"), (87, 38, "K"),
             (84, 39, "K"), (85, 39, "K"), (86, 39, "K")),
    # hurt and defeat: eyes squeezed shut, knitted brow, mouth a pained line; no white anywhere
    "shut": ((78, 29, "K"), (79, 29, "K"), (80, 30, "K"), (81, 29, "K"),
             (84, 29, "K"), (85, 30, "K"), (86, 29, "K"), (87, 29, "K"),
             (84, 26, "K"), (85, 27, "K"), (86, 26, "K"),
             (87, 31, "h"), (87, 33, "s"), (88, 33, "s"),
             (83, 37, "K"), (84, 36, "K"), (85, 36, "K"), (86, 36, "K"), (87, 37, "K")),
}

IDLE = {
    "body": (0, 0, 0, 1.0), "head": (0, 0, 0), "swing": 0,
    "near_arm": ((36, 70), (31, 88), (30, 94), 0), "far_arm": ((101, 68), (106, 86), (108, 91), 0),
    "near_leg": ((44, 112), (40, 117), 0), "far_leg": ((96, 111), (100, 117), 0),
    "face": "idle", "sleeves": "poly", "far_front": False, "stripe": True, "cuffs": True,
}
WALK = dict(IDLE, body=(4, 4, 2, 1.0), swing=-3,
            near_arm=((44, 71), (45, 88), (44, 94), -6), far_arm=((101, 70), (104, 88), (105, 93), 8),
            near_leg=((64, 109), (61, 113), 20), far_leg=((104, 110), (107, 117), 0), sleeves="cap")
TELL = dict(IDLE, body=(-6, -2, -6, 1.0), head=(-6, 0, 0), swing=-12,
            near_arm=((36, 30), (50, 22), (56, 17), 180), far_arm=((100, 28), (100, 19), (100, 15), 160),
            near_leg=((44, 108), (38, 117), 0), far_leg=((96, 108), (102, 117), 0), face="roar", sleeves="cap")
ATTACK = dict(IDLE, body=(35, -4, 6, 1.0), head=(-25, 0, 0), swing=6,
              near_arm=((100, 82), (120, 110), (126, 117), -40), far_arm=((126, 92), (138, 110), (141, 117), -50),
              near_leg=((36, 108), (24, 116), 25), far_leg=((94, 108), (100, 117), 0), face="roar", sleeves="cap")
HURT = dict(IDLE, body=(-5, -4, -1, 1.0), head=(-12, -3, 0), swing=4,
            near_arm=((52, 68), (64, 58), (68, 54), 160), far_arm=((98, 64), (90, 54), (88, 50), 200),
            near_leg=((42, 110), (36, 117), 0), far_leg=((88, 110), (90, 117), 0), face="shut", sleeves="cap",
            far_front=True)
DEFEAT = dict(IDLE, body=(90, -28, 16, 0.55), head=(-45, 0, 0), swing=0,
              near_arm=((116, 121), (130, 123), (134, 123), -90), far_arm=((108, 125), (118, 126), (121, 125), -90),
              near_leg=((60, 121), (32, 121), 80), far_leg=((68, 123), (42, 124), 80), face="shut", sleeves="cap",
              stripe=False, cuffs=False)


def hand(palm, ang, fingers, thumb, rf, rt) -> dict:
    px, py = palm
    shapes = []
    for (a, b), r in zip(list(fingers) + [thumb], [rf] * len(fingers) + [rt]):
        p0, p1 = rot_about(px, py, *a, ang), rot_about(px, py, *b, ang)
        shapes.append(capsule(p0[0], p0[1], r[0], p1[0], p1[1], r[1]))
    return union(*shapes)


def hoarder(pose: dict) -> list[str]:
    ang, dx, dy, sx = pose["body"]
    B = rig(*BP, ang, dx, dy, sx)
    H = head_rig(B, ang + pose["head"][0], pose["head"][1], pose["head"][2])
    Bi, Hi = inv(B), inv(H)
    sw = pose["swing"]
    c = Canvas()

    def arm(side):
        elbow, wrist, palm, hang = pose[side + "_arm"]
        if side == "near":
            sh = apply(B, 48, 46)
            return union(capsule(*sh, 11, *elbow, 9), capsule(*elbow, 9, *wrist, 6), ellipse(*palm, 6, 5),
                         hand(palm, hang, NEAR_FINGERS, NEAR_THUMB, (1.8, 1.5), (1.9, 1.6)))
        sh = apply(B, 88, 46)
        return union(capsule(*sh, 9, *elbow, 8), capsule(*elbow, 8, *wrist, 5.5), ellipse(*palm, 5, 5),
                     hand(palm, hang, FAR_FINGERS, FAR_THUMB, (1.7, 1.4), (1.8, 1.5)))

    def sleeve(side):
        if pose["sleeves"] == "poly":
            if side == "near":
                return t_polygon(B, [(40, 34), (58, 36), (60, 50), (52, 60), (47, 57), (42, 61), (36, 55), (35, 44)],
                                 (50, 44), (14, 14))
            return t_polygon(B, [(82, 38), (96, 44), (99, 56), (95, 58), (92, 55), (88, 59), (82, 52)],
                             (88, 46), (10, 12))
        elbow = pose[side + "_arm"][0]
        sh = apply(B, 48, 46) if side == "near" else apply(B, 88, 46)
        r = 11 if side == "near" else 9
        end = (sh[0] + (elbow[0] - sh[0]) * 0.45, sh[1] + (elbow[1] - sh[1]) * 0.45)
        return capsule(*sh, r + 1.5, *end, r + 0.5)

    def leg(side):
        knee, ankle, _f = pose[side + "_leg"]
        if side == "near":
            hip = apply(B, 54, 100)
            return union(capsule(*hip, 14, *knee, 11), capsule(*knee, 10, *ankle, 9),
                         ellipse(ankle[0], ankle[1] - 1, 11.5, 3))
        hip = apply(B, 82, 100)
        return union(capsule(*hip, 13, *knee, 10), capsule(*knee, 9.5, *ankle, 8.5),
                     ellipse(ankle[0], ankle[1] - 1, 10.5, 3))

    def foot(side):
        _k, (ax, ay), fang = pose[side + "_leg"]
        rel = NEAR_FOOT if side == "near" else FAR_FOOT
        pts = [rot_about(ax, ay, rx, ry, fang) for rx, ry in rel]
        centre = (ax, ay + 4) if side == "near" else (ax + 2, ay + 4)
        return polygon(pts, rot_about(ax, ay, centre[0] - ax, centre[1] - ay, fang),
                       (14, 8) if side == "near" else (13, 8))

    if not pose["far_front"]:
        c.part("far_arm", "skin", arm("far"))
        c.part("far_sleeve", "shirt", sleeve("far"))
    c.part("far_leg", "pants", leg("far"))
    c.part("far_foot", "skin", foot("far"))
    c.part("hips", "pants", t_ellipse(B, 68, 100, 30, 9))
    c.part("near_leg", "pants", leg("near"))
    c.part("near_foot", "skin", foot("near"))
    torso = union(t_ellipse(B, 56, 46, 17, 13), t_ellipse(B, 72, 52, 22, 16), t_ellipse(B, 72, 77, 30, 20),
                  t_ellipse(B, 68, 91, 29, 7))
    c.part("torso", "shirt", torso, clip=lambda x, y: (lambda l: l[1] <= hem_y(l[0]))(apply(Bi, x, y)))
    c.part("neckline", "skin", t_polygon(B, [(80, 42), (89, 42), (88, 46), (86, 48), (85, 46), (83, 48), (81, 46)],
                                         (85, 43), (6, 4)))
    c.part("hair_back", "hair", union(
        t_polygon(H, swung([(68, 18), (58, 18), (50, 23), (44, 30), (40, 40), (37, 52), (36, 62), (39, 70),
                            (44, 66), (48, 58), (54, 48), (62, 40), (70, 34)], sw), (60, 30), (22, 26)),
        *(t_line(H, *swung([a], sw)[0], *swung([b], sw)[0]) for a, b in
          (((60, 19), (53, 16)), ((55, 21), (46, 18)), ((38, 64), (33, 74)), ((42, 66), (40, 77)),
           ((46, 60), (46, 71))))))
    c.part("near_arm", "skin", arm("near"))
    c.part("near_sleeve", "shirt", sleeve("near"))
    c.part("locks_far", "hair", union(t_capsule(H, 92, 34, 2.2, 94, 48, 2.0), t_capsule(H, 94, 48, 2.0, 93, 62, 1.0),
                                      t_capsule(H, 93, 24, 2.0, 96, 42, 2.2), t_capsule(H, 96, 42, 2.2, 95, 60, 1.0),
                                      t_capsule(H, 96, 32, 1.6, 98, 46, 1.6), t_capsule(H, 98, 46, 1.6, 97, 56, 0.8)))
    # head: 24 x 23, turned about 25 degrees toward the camera; the jaw tapers to a soft, narrow chin
    def jaw_ok(x, y):
        lx, ly = apply(Hi, x, y)
        return ly <= 34 or abs(lx - 83) <= 11 - (ly - 34) * 1.1

    c.part("head", "skin", union(t_ellipse(H, 81, 29, 11.75, 11.5), t_ellipse(H, 83.5, 38.5, 5, 3)), clip=jaw_ok)

    def cap_keep(x, y):
        lx, ly = apply(Hi, x, y)
        top = 16 + (NOTCH[int(round(lx)) % 14] if ly <= 22 else 0)
        return (lx < 74 - max(0, ly - 36) or ly <= 23) and ly >= top

    cap = {p: n for p, n in t_ellipse(H, 79, 27.5, 13.5, 11.5).items() if cap_keep(*p)}
    c.part("hair_cap", "hair", union(cap, t_line(H, 70, 20, 66, 16), t_line(H, 76, 18, 79, 16),
                                     t_line(H, 86, 19, 88, 16), t_line(H, 74, 24, 76, 38), t_line(H, 77, 23, 78, 26)))
    # back hair over the near shoulder: a root band and three tapering strand groups, with shirt showing between
    c.part("locks_back", "hair", union(
        t_polygon(H, swung([(45, 33), (71, 35), (71, 40), (45, 40)], sw), (58, 36), (14, 6)),
        *(t_polygon(H, swung(g, sw), (sum(x for x, _ in g) / len(g), 56), (4, 22)) for g in LOCK_GROUPS)))
    if pose["far_front"]:
        c.part("far_arm", "skin", arm("far"))
        c.part("far_sleeve", "shirt", sleeve("far"))

    c.shade(hair_sheen=lambda x, y: (x * 2 + y // 3) % 9 == 0)
    # blue-grey highlight on the lit (left) edge of each back strand group
    for (x, y) in [p for p in c.key if c.parts[c.owner[p]] == "locks_back"]:
        if c.key[(x, y)] != "K" and c.key.get((x - 1, y)) == "K":
            c.key[(x, y)] = "c"

    def where(family, m, pred, key, keep=("K",)):
        mi = inv(m)
        for p in [p for p, f in c.family.items() if f == family]:
            if c.key[p] not in keep and pred(*apply(mi, *p)):
                c.key[p] = key

    # burn: an irregular patch on the near half only, dark plum, with a soft rose-brown edge on the nose side
    def burn_in(lx, ly):
        e = lambda cx, cy, rx, ry: ((lx - cx) / rx) ** 2 + ((ly - cy) / ry) ** 2 <= 1  # noqa: E731
        edge = 80.4 + (int(ly) * 5 % 3) * 0.5
        return (e(77.6, 30.6, 3.3, 5.4) or e(76.8, 25.6, 2.2, 1.5) or e(79.2, 35, 2.1, 1.6)) and lx <= edge

    burned = set()
    hi_ = Hi
    for p in [p for p, f in c.family.items() if f == "skin" and c.parts[c.owner[p]] == "head"]:
        if c.key[p] != "K" and burn_in(*apply(hi_, *p)):
            c.key[p] = "x"
            burned.add(p)
    for (x, y) in burned:
        nxt = (x + 1, y)
        if nxt not in burned and c.family.get(nxt) == "skin" and c.key.get(nxt) != "K":
            c.key[(x, y)] = "y"
            if c.key[nxt] in ("m", "h"):
                c.key[nxt] = "s"
    for lx, ly, k in FACES[pose["face"]]:
        x, y = apply(H, lx, ly)
        c.put(round(x), round(y), k)
    # chest mass above the belly, read through light and shade on the baggy shirt (no cleavage, no outline change)
    for cx, cy, rx, ry in ((76, 51, 5.5, 4.5), (87, 50, 4, 4)):
        def bust(lx, ly, cx=cx, cy=cy, rx=rx, ry=ry):
            nx, ny = (lx - cx) / rx, (ly - cy) / ry
            return nx * nx + ny * ny <= 1, nx, ny
        where("shirt", B, lambda lx, ly, f=bust: f(lx, ly)[0] and f(lx, ly)[1] + f(lx, ly)[2] < -0.45, "v",
              keep=("K", "g"))
        where("shirt", B, lambda lx, ly, f=bust: f(lx, ly)[0] and f(lx, ly)[2] > 0.5, "t", keep=("K", "g"))
    # grime stains on the plain shirt (no print, no logo)
    for sx_, sy_, sw_, sh_ in STAINS:
        where("shirt", B, lambda lx, ly, a=sx_, b=sy_, w=sw_, h=sh_: a <= round(lx) < a + w and b <= round(ly) < b + h,
              "g", keep=("K", "s", "m", "h"))
    # side tear in the shirt showing skin
    for p in [p for p, f in c.family.items() if f == "shirt"]:
        lx, ly = apply(Bi, *p)
        if in_polygon(TEAR_IN, lx, ly):
            c.key[p] = "s" if ly > 81 else "m"
        elif in_polygon(TEAR_OUT, lx, ly):
            c.key[p] = "K"
    # filth drips from the hem onto the pants
    for x_, y0, n in ((50, 97, 4), (74, 98, 4), (93, 96, 3), (60, 99, 3)):
        where("pants", B, lambda lx, ly, a=x_, b=y0, n=n: round(lx) == a and b <= round(ly) < b + n, "g")
    # near knee tear
    kx, ky = pose["near_leg"][0]
    for (x, y) in ellipse(kx - 1, ky - 1, 3.5, 2.6):
        if c.family.get((x, y)) == "pants":
            c.key[(x, y)] = "m" if y < ky - 1 else "s"
    # faded sweatpants side stripe, hip to ankle (2 px on the thigh, 1 px below the knee)
    if pose["stripe"]:
        hip_y = round(apply(B, 54, 100)[1])
        ankle_y = pose["near_leg"][1][1]
        for y in range(hip_y - 7, ankle_y - 1):
            xs = [x for x in range(0, kx + 22) if c.family.get((x, y)) == "pants" and c.key.get((x, y)) != "K"]
            if xs:
                for d in ((2, 3) if y < ky - 6 else (2,)):
                    if c.key.get((min(xs) + d, y)) in ("p", "q", "r"):
                        c.key[(min(xs) + d, y)] = "u"
    # ankle cuffs gathered over the feet: a crease and the cuff edge
    if pose["cuffs"]:
        for side, (o0, o1) in (("near", (-9, 9)), ("far", (-8, 9))):
            ax, ay = pose[side + "_leg"][1]
            for x in range(ax + o0, ax + o1):
                c.put(x, ay + 1, "K")
            for x in range(ax + o0 + 3, ax + o1 - 3, 3):
                c.put(x, ay - 2, "p")
                c.put(x + 1, ay - 2, "p")
    # toes
    for side, toes in (("near", NEAR_TOES), ("far", FAR_TOES)):
        _k, (ax, ay), fang = pose[side + "_leg"]
        for t in toes:
            for ty in (8, 9):
                x, y = rot_about(ax, ay, t, ty, fang)
                c.put(round(x), round(y), "K")
    # a scrap of tin foil snagged in the back hair (the hoard clings to her)
    for lx, ly in ((57, 27), (58, 27), (58, 28), (59, 28), (59, 29)):
        x, y = apply(H, lx, ly)
        c.put(round(x), round(y), "n")
    rows = c.finish()
    # raised scar clusters (rose-brown) each with a dark shadow pixel below, after the clean-up pass
    grid = [list(r) for r in rows]
    for cluster in BURN_CLUSTERS:
        pts = [tuple(round(v) for v in apply(H, *p)) for p in cluster]
        if all(0 <= y + 1 < len(grid) and grid[y][x] in ("x", "y") for x, y in pts):
            for x, y in pts:
                grid[y][x] = "y"
            sx0, sy0 = pts[0]
            if grid[sy0 + 1][sx0] in ("x", "y"):
                grid[sy0 + 1][sx0] = "K"
    return ["".join(r) for r in grid]


# in-betweens (spec section 8): each is a pose dict for hoarder(), interpolated from the neighbouring keys
INB = {
    # idle breath: exhale (body 1 px down, hands 1 px lower), inhale (body 1 px up, hair swing -1); feet unchanged
    1: dict(IDLE, body=(0, 0, 1, 1.0),
            near_arm=((36, 71), (31, 89), (30, 95), 0), far_arm=((101, 69), (106, 87), (108, 92), 0)),
    2: dict(IDLE, body=(0, 0, -1, 1.0), swing=-1),
    # walk: down (weight onto the far leg, near foot flat and dragging), passing, up (near lands, far heel lifts)
    4: dict(WALK, body=(3, 3, 4, 1.0), swing=-2,
            near_arm=((44, 72), (45, 89), (44, 95), -4), far_arm=((101, 70), (104, 88), (105, 93), 6),
            near_leg=((68, 110), (66, 114), 12), far_leg=((99, 111), (101, 117), 0)),
    5: dict(WALK, body=(1, 2, 1, 1.0), swing=-1,
            near_arm=((42, 71), (43, 88), (42, 94), -2), far_arm=((101, 69), (105, 87), (106, 92), 4),
            near_leg=((70, 111), (72, 117), 0), far_leg=((93, 111), (95, 117), 0)),
    6: dict(WALK, body=(3, 3, 0, 1.0), swing=-2,
            near_arm=((44, 72), (45, 89), (44, 95), -4), far_arm=((101, 70), (104, 88), (105, 93), 6),
            near_leg=((66, 111), (66, 117), 0), far_leg=((100, 108), (99, 114), 25)),
    # tell: gather (body 4 px down, arms drawn back to the hips, head down 6 degrees)
    7: dict(TELL, body=(0, 0, 4, 1.0), head=(6, 0, 3), swing=-4, face="idle",
            near_arm=((36, 76), (41, 92), (46, 100), 0), far_arm=((98, 78), (96, 92), (96, 100), 0),
            near_leg=((43, 110), (39, 117), 0), far_leg=((97, 110), (101, 117), 0)),
    # attack: smear (arms as one arc, halfway down), recovery (key 10 with the head lifted 4 px)
    9: dict(ATTACK, body=(15, -3, 0, 1.0), head=(-16, 0, 0), swing=-3,
            near_arm=((88, 52), (108, 70), (114, 76), -110), far_arm=((112, 58), (126, 74), (131, 78), -130),
            near_leg=((40, 108), (30, 116), 12), far_leg=((95, 108), (101, 117), 0)),
    11: dict(ATTACK, body=(36, -4, 8, 0.97), head=(-25, 0, -4), swing=3),
    # defeat: buckle (knees give, body 18 px down, near hand out), floor impact (body ~60 degrees, squash 0.8)
    13: dict(DEFEAT, body=(20, -6, 18, 1.0), head=(-10, 0, 0), swing=-2,
             near_arm=((78, 86), (96, 104), (102, 110), -30), far_arm=((96, 98), (104, 112), (108, 117), -20),
             near_leg=((66, 114), (42, 117), 0), far_leg=((100, 114), (92, 117), 0),
             stripe=True, cuffs=True),
    14: dict(DEFEAT, body=(60, -18, 26, 0.75), head=(-25, 0, 0), swing=0,
             near_arm=((106, 108), (118, 118), (122, 121), -70), far_arm=((100, 112), (110, 120), (113, 121), -70),
             near_leg=((60, 119), (36, 120), 60), far_leg=((68, 121), (46, 122), 60)),
}
POSES = {0: IDLE, 3: WALK, 8: TELL, 10: ATTACK, 12: HURT, 15: DEFEAT, **INB}
INBETWEENS = tuple(sorted(INB))


def render(frame: int) -> list[str]:
    return hoarder(POSES[frame])




# --------------------------------------------------------------------------- grid io
def palette_json() -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def build_grid() -> dict:
    frames = []
    for i, ticks in enumerate(TICKS):
        frames.append({"duration_ms": round((ticks or 6) * 1000 / TICK_HZ), "cels": {"body": render(i)}})
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


MANIFEST = ROOT / "assets/manifests/hoard-01.json"
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once", "hurt": "once", "down": "hold"}


def build_manifest() -> dict:
    tag_of = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
    frames = [{"index": n, "tag": tag_of[n], "ticks": t, "duration_ms": round((t or 6) * 1000 / TICK_HZ),
               "author": "key" if n in KEY_FRAMES else "sonnet"} for n, t in enumerate(TICKS)]
    return {
        "schema_version": 1,
        "id": "hoarder_boss_h112_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/hoarder/hoard-01.ase",
        "grid_source": "assets/source/characters/hoarder/hoard-01.grid.json",
        "output_atlas": "generated/art/hoard-01.png",
        "output_metadata": "generated/art/hoard-01.json",
        "palette_id": "hoarder-floor01-v1",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 01 boss The Hoarder; runtime role IP-neutral boss)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("HOARD-01 The Hoarder at the decided 112 px idle height: 6 frozen keys (atlas frames 0, 3, 8, 10, 12, 15, "
                       "fixture tests/fixtures/hoard-01_keys.json) plus 10 in-betweens (1, 2, 4, 5, 6, 7, 9, 11, 13, 14) "
                       "rasterized by scripts/author_hoard01.py from the same pose rig hoarder(pose); layer body"),
            "generator": ("Design, size decision, palette and keys: Claude Opus. In-betweens (pose dicts), manifest, .ase, "
                          "validators and evidence: Claude Sonnet. Review: a separate Claude Opus agent (pending). No "
                          "image-model raster output, no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": [],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "hoarder": {
            "status": "Keys frozen, in-betweens drawn, validators pass; independent Opus review pending; Bevy integration pending; 16 frames, 6 of them Opus keys; not wired into any Bevy binary",
            "spec": "docs/02-art/hoard-01-spec.md",
            "opaque_height_idle": 112,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": 100,
            "grounded_tags": [n for n, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": list(KEY_FRAMES),
            "inbetween_frames": list(INBETWEENS),
            "key_fixture": "tests/fixtures/hoard-01_keys.json",
            "attack_ticks": {"tell": [0, 33], "swing_smear": [0, 2], "swing_impact_hitbox": [3, 8], "swing_recover": [9, 32]},
            "frames": frames,
        },
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def report(rows: list[str], label: str) -> None:
    flat = [list(r) for r in rows]
    orphans, isolated = pg._stray_pixels(flat, ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    used = sorted({ch for r in rows for ch in r} - {"."})
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} keys {''.join(used)}")


# --------------------------------------------------------------------------- scale evidence
def _image(rows: list[str], palette: list[dict]):
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in palette}
    im = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch != ".":
                im.putpixel((x, y), pal[ch])
    return im


def scale_images(grid: dict, out_dir: Path) -> dict:
    """Key 0 next to Carl (h52 frame 0) on the floor line: a 1x image and the same scene at 4x.

    The scene shows the pair twice, on a warm brick-toned panel and on a cold arch-toned panel."""
    from PIL import Image
    carl = pg.load_grid(CARL_GRID)
    carl_im = _image(["".join(r) for r in pg.composite_keys(carl, 0)], carl["palette"])
    boss_im = _image(grid["frames"][0]["cels"]["body"], grid["palette"])
    panel_w, h = 220, 140
    scene = Image.new("RGBA", (panel_w * 2, h), (0, 0, 0, 255))
    floor = h - 8
    for i, bg in enumerate(((112, 58, 40, 255), (36, 42, 60, 255))):
        x0 = i * panel_w
        scene.paste(Image.new("RGBA", (panel_w, h), bg), (x0, 0))
        scene.paste(Image.new("RGBA", (panel_w, 8), (40, 30, 30, 255)), (x0, floor + 1))
        cx = x0 + 20 + carl["pivot"][0]
        scene.alpha_composite(carl_im, (cx - carl["pivot"][0], floor - carl["pivot"][1]))
        bx = x0 + 70 + PIVOT[0]
        scene.alpha_composite(boss_im, (bx - PIVOT[0], floor - PIVOT[1]))
    out_dir.mkdir(parents=True, exist_ok=True)
    one = out_dir / "key0-scale-1x.png"
    four = out_dir / "key0-scale-4x.png"
    scene.save(one)
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    combo = Image.new("RGBA", (scene.width * 4, scene.height * 5 + 8), (20, 16, 20, 255))
    combo.alpha_composite(scene, (0, 0))
    combo.alpha_composite(scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST), (0, scene.height + 8))
    combo_path = out_dir / "key0-scale.png"
    combo.save(combo_path)
    sheet = Image.new("RGBA", (CANVAS[0] * 3 * 3, CANVAS[1] * 3 * 2), (0, 0, 0, 255))
    for n, f in enumerate(KEY_FRAMES):
        x, y = (n % 3) * CANVAS[0] * 3, (n // 3) * CANVAS[1] * 3
        sheet.paste(Image.new("RGBA", (CANVAS[0] * 3, CANVAS[1] * 3), (36, 42, 60, 255) if n % 2 else
                              (112, 58, 40, 255)), (x, y))
        im = _image(grid["frames"][f]["cels"]["body"], grid["palette"])
        sheet.alpha_composite(im.resize((CANVAS[0] * 3, CANVAS[1] * 3), Image.NEAREST), (x, y))
    keys_path = out_dir / "keys-x3.png"
    sheet.save(keys_path)
    return {"1x": one, "4x": four, "combo": combo_path, "keys": keys_path}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--scale", type=Path, help="directory for the key0 scale images")
    a = ap.parse_args(argv)
    grid = build_grid()
    for f in range(len(TICKS)):
        report(grid["frames"][f]["cels"]["body"], f"frame {f:2d}")
    text = dump(grid)
    if a.scale:
        for k, p in scale_images(grid, a.scale).items():
            print(f"{k}: {p}")
    if a.check:
        ok = GRID.is_file() and GRID.read_text() == text
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    pg.write_ase(grid, ASE)
    MANIFEST.write_text(dump(build_manifest()), encoding="utf-8")
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
