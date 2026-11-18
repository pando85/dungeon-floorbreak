#!/usr/bin/env python3
"""TROG-01 authoring record: the Troglodyte Pygmy (family base) palette and the six draft key poses.

Usage:

    python3 scripts/author_trog01.py frames [--check]
    python3 scripts/author_trog01.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/trog_01.grid.json`` (pixel-grid-v1, layer ``body``)
and ``assets/manifests/trog-01.json``: one key per readability tag (idle, move contact, tell, attack, hurt,
defeat) and the ten in-betweens drawn from the same part rules (blends of two keys or hand-set limbs). Every pose is a *rule*: thick
polylines (torso, limbs, dreadlocks) and polygons (ears, loincloth) rasterized at pixel centres in design units
(x right from the pivot column, y up from the contact row), a 1 px outline, a simple upper-left rim light and
literal face stamps. No raster image-model output, no tracing of concept pixels. ``--check`` compares instead
of writing (exit 1 on a difference).

Keys after the design edit (head, hair, face and construction redrawn). The Basher and Virtuoso derive from the shared construction recorded in
``docs/02-art/trog-01-spec.md`` section 2b; they are not drawn here.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets/source/characters/troglodyte"
GRID = OUT_DIR / "trog_01.grid.json"
MANIFEST = ROOT / "assets/manifests/trog-01.json"
CANVAS = (80, 56)
PIVOT = (30, 55)

# trog-family-v0: one skin ramp, hair, rag cloth, fangs and mouth. 15 of 20 slots used; the other five are
# kept free for the family variants (Basher wraps/trash, Virtuoso robe/bells/tongue).
PALETTE = [
    ("K", "outline_plum", "#1a1017ff", False),
    ("d", "skin_deep", "#5a2c3fff", False),
    ("s", "skin_shade", "#8c4651ff", False),
    ("m", "skin_mid", "#bf6f64ff", False),
    ("l", "skin_light", "#e5a08cff", False),
    ("H", "hair_deep", "#1d1721ff", False),
    ("I", "hair_mid", "#403442ff", False),
    ("c", "cloth_deep", "#3b2f2cff", False),
    ("C", "cloth_mid", "#72604dff", False),
    ("w", "eye_white_drool", "#efe4c4ff", True),
    ("e", "eye_red", "#d83a22ff", True),
    ("t", "fang_yellow", "#e6bd3aff", True),
    ("T", "fang_shade", "#967022ff", False),
    ("o", "mouth_dark", "#4a1020ff", False),
    ("q", "tongue_pink", "#c8566aff", False),
]
RAMPS = {
    "skin": ("d", "s", "m", "l"),
    "far": ("K", "d", "d", "s"),
    "hair": ("H", "H", "I", "I"),
    "cloth": ("c", "c", "c", "C"),
}
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


def palette_json(keys: set[str] | None = None) -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        if keys is not None and key not in keys:
            continue
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


class Canvas:
    """Back-to-front part canvas (later parts occlude earlier ones; the outline follows part order)."""

    def __init__(self, w: int, h: int, floor_row: int):
        self.w, self.h = w, h
        self.px, self.bottom = PIVOT[0], floor_row - 1          # fill ends one row above the outline row
        self.part: list[list[int | None]] = [[None] * w for _ in range(h)]
        self.ramps: list[tuple] = []

    def P(self, x: float, y: float) -> tuple[float, float]:
        return (self.px + 0.5 + x, self.bottom + 1.0 - y)

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1

    def _fill(self, test, x0, x1, y0, y1, pid):
        for y in range(max(0, int(y0) - 1), min(self.h, int(y1) + 2)):
            for x in range(max(0, int(x0) - 1), min(self.w, int(x1) + 2)):
                if test(x + 0.5, y + 0.5):
                    self.part[y][x] = pid

    def poly(self, pts, pid):
        pts = [self.P(*p) for p in pts]

        def inside(px, py):
            c = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > py) != (yb > py) and px < xa + (py - ya) * (xb - xa) / (yb - ya):
                    c = not c
            return c
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        self._fill(inside, min(xs), max(xs), min(ys), max(ys), pid)

    def disc(self, cx, cy, rx, ry, pid):
        cx, cy = self.P(cx, cy)
        self._fill(lambda px, py: ((px - cx) / rx) ** 2 + ((py - cy) / ry) ** 2 <= 1.0,
                   cx - rx, cx + rx, cy - ry, cy + ry, pid)

    def thick(self, pts, radii, pid):
        for i in range(len(pts) - 1):
            (xa, ya), (xb, yb) = pts[i], pts[i + 1]
            n = max(2, int(math.hypot(xb - xa, yb - ya) * 2))
            for k in range(n + 1):
                t = k / n
                r = radii[i] + (radii[i + 1] - radii[i]) * t
                self.disc(xa + (xb - xa) * t, ya + (yb - ya) * t, r, r, pid)

    def render(self) -> list[list[str]]:
        out = [["."] * self.w for _ in range(self.h)]

        def get(x, y):
            return self.part[y][x] if 0 <= x < self.w and 0 <= y < self.h else None
        for y in range(self.h):
            for x in range(self.w):
                pid = self.part[y][x]
                if pid is None:
                    if any(get(x + dx, y + dy) is not None for dx, dy in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                        out[y][x] = "K"
                    continue
                n4 = [get(x, y - 1), get(x - 1, y), get(x + 1, y), get(x, y + 1)]
                if any(n is not None and n > pid for n in n4):
                    out[y][x] = "K"
                    continue
                _deep, shade, mid, light = self.ramps[pid]
                up, left, right, down = n4
                if up != pid or left != pid:
                    out[y][x] = light
                elif right != pid or down != pid or get(x + 1, y + 1) != pid:
                    out[y][x] = shade
                else:
                    out[y][x] = mid
        return out


# ------------------------------------------------------------------ face stamps
# Literal templates, 11 columns x up to 11 rows; top-left at (hx - 4, hy + 4) in design units, where (hx, hy) is the
# centre of the front (jaw) circle of the head. Brow and eye sit at hx - 1 .. hx + 3; the mouth runs from the back of
# the cheek (hx - 3) to the front edge (hx + 6, where the roar opens the outline). "." leaves the pixel alone; the
# accents (e, w, t, T) and the tongue q may overhang the silhouette.
FACES = {
    # idle grimace: heavy brow, bulging bloodshot eye (3 red, 4 white, 2 pupil), clenched teeth over a dark gash, drool
    "snarl": ["....KKKK...",
              "...eewwK...",
              "....ewwK...",
              "........s..",
              "..ttTtTtt..",
              ".ooooooooo.",
              "..t.tT.tt..",
              "...w.......",
              "...w......."],
    # roar (tell, leap): same eye, upper teeth row, a 3-row gaping mouth that opens the front outline, lower fangs, drool
    "roar": ["....KKKK...",
             "...eewwK...",
             "....ewwK...",
             "..tTttTtt..",
             ".oooooooooo",
             ".oooooooooo",
             ".oooqqqooo.",
             "..t.tT.tT..",
             "...........",
             "...w....w..",
             "...w......."],
    # pain (hurt): eye squeezed to a dark slit, no red; mouth open in a yelp
    "pain": ["...KKKK....",
             "....sKKK...",
             "...........",
             "..ttTttt...",
             ".ooooooooo.",
             ".ooooooooo.",
             ".oooooooo..",
             "..t.tT.t...",
             "...........",
             "......w...."],
    # pant (landing recovery): lid dropped over the eye, slack 1-row mouth short of the front, loose fangs, drool
    "pant": ["...........",
             "...KKKKK...",
             "...eKwwK...",
             "........s..",
             "...........",
             "..tT.t.t...",
             ".ooooooo...",
             "...........",
             "...w..w....",
             "...w..w....",
             "......w...."],
    # dead (defeat): crossed eye, slack jaw, a short tongue (5 px) hanging out of the mouth corner (not a stripe along the jaw)
    "dead": ["...........",
             "....K.K....",
             ".....K.....",
             "....K.K....",
             "...........",
             "..ooooooo..",
             "..t.tt.qq..",
             ".......qq..",
             "........q.."],
}

HEAD_R = (5.6, 5.3)        # skull (back) and jaw (front) circle radii; the same in every pose (the volume never changes)


def L(pts, r):
    return {"pts": pts, "r": r}


def head(hx, hy, tilt=-1.0):
    """The family head: a skull circle 4 px behind and ``-tilt`` px above the jaw circle centred on (hx, hy)."""
    return L([(hx - 4, hy - tilt), (hx, hy)], list(HEAD_R))


def jaw(hx, hy, drop=1.0):
    """Dropped lower jaw for the open-mouth faces (roar, pain); ``drop`` 0..1 scales how far it hangs."""
    return L([(hx - 5, hy - 3), (hx + 3, hy - 3 - 2.5 * drop)], [2.8, 2.2 + 0.2 * drop])


# Hair: a matted crown mop (polygon) plus heavy rope dreads hanging DOWN and back from it, and one front lock over the
# brow. Offsets from the jaw centre (hx, hy). Every rope ends lower than it starts (gravity) and no point rises above
# the crown, so the silhouette is a lumpy mop with hanging ropes, never horns. ``sweep`` streams the tips back and
# lifts them toward the horizontal (motion); a negative sweep throws them forward (hurt whiplash).
MOP = [(-9, 3), (-9.5, 7), (-7.6, 8.6), (-7, 11), (-5.4, 11.1), (-3.5, 13), (-2, 11.7), (-0.5, 12.4), (1.2, 10), (0, 8.2),
       (-2.5, 6.8), (-5, 5.2), (-6.5, 4)]
ROPES = [[(-7, 9), (-11, 8), (-15, 5), (-17, 1), (-17.5, -2)],
         [(-8, 6), (-11, 3.5), (-12, 0), (-11.5, -3)],
         [(-3, 11), (-7, 11), (-11, 9.5), (-14, 7)],
         [(0, 10), (2, 8.6), (3, 6.4)]]
ROPE_R = 0.95
BEADS = {0: "t", 1: "w", 2: "t"}   # 1 px bead or knot near the tip of ropes 0, 1 and 2 (the concept's beaded dreads)
EAR = [(-5, 3.5), (-12, 5), (-7, 1.5)]          # pointed, swept straight back below the crown line, skin over the mop


def hair(hx, hy, sweep=0.0, sway=0.0):
    """``sway`` shifts the rope tips ``sway`` px back (negative: forward), growing from 0 at the root (idle breathing)."""
    mop = [(hx + x, hy + y) for x, y in MOP]
    ropes = []
    for rope in ROPES:
        n = len(rope) - 1
        ropes.append([(hx + x - 3.0 * sweep * i / n - sway * i / n, hy + y + 2.2 * sweep * i / n + 0.4 * sway * i / n)
                      for i, (x, y) in enumerate(rope)])
    return {"mop": mop, "ropes": ropes, "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def build(pose: dict) -> list[str]:
    """Rasterize one pose dict into 56 row strings."""
    w, h = CANVAS
    c = Canvas(w, h, PIVOT[1])
    far = c.add("far"); skin = c.add("skin"); cloth = c.add("cloth"); hair_id = c.add("hair"); arm = c.add("skin")
    ear = c.add("skin")
    hd = pose["hair"]
    # back to front: far limbs, torso, near leg, loincloth, head and jaw, mop and ropes, near arm, ear
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for fin in pose.get("far_fingers", []):      # claws of a far hand that is planted on the floor (walk contact B)
        c.thick(fin, [0.55] * len(fin), far)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["loin"], cloth)
    c.thick(pose["head"]["pts"], pose["head"]["r"], skin)
    if "jaw" in pose:
        c.thick(pose["jaw"]["pts"], pose["jaw"]["r"], skin)
    c.poly(hd["mop"], hair_id)
    for rope in hd["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], arm)
    for i, fin in enumerate(pose.get("fingers", [])):
        c.thick(fin, [0.55] * len(fin), c.add("skin") if i % 3 == 1 else arm)
    for e in hd["ears"]:
        c.poly(e, ear)
    rows = c.render()
    # dread texture: every third column of the lit hair falls to the deep tone, so the mop reads as separate hanging
    # ropes (vertical strands with lumps), not a smooth wig or a solid helmet
    for y in range(h):
        for x in range(w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
    # beads: the second-to-last point of ropes 0 to 2 carries a 1 px bead or knot
    for i, key in BEADS.items():
        bx, by = c.P(*hd["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    # mottled rim: no skin run of more than 5 clean lit pixels; the 4th and 5th of a longer run drop to mid
    for y in range(h):
        run = 0
        for x in range(w + 1):
            if x < w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    # ribs: short shade strokes (x, y_top, y_bottom) on lit/mid skin only: the lean flank
    for rx, ry0, ry1 in pose.get("ribs", []):
        col = int(c.P(rx, 0)[0])
        for y in range(int(c.P(0, ry0)[1]), int(c.P(0, ry1)[1]) + 1):
            if rows[y][col] in "ml":
                rows[y][col] = "d" if rows[y][col] == "l" else "s"
    # grime: 2 px dirt smudges (deep skin) on lit or mid skin, and dirt-caked soles and knuckles on the two floor rows
    for gx, gy in pose.get("grime", []):
        px, py = c.P(gx, gy)
        for dx in (0, 1):
            if rows[int(py)][int(px) + dx] in "ml":
                rows[int(py)][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    # face template
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 4, hy + 4)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if rows[y][x] != "." or ch in "ewtTq":
                    rows[y][x] = ch
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        rows[int(py)][int(px)] = key
    despeckle(rows)
    return ["".join(r) for r in rows]


def despeckle(rows: list[list[str]]) -> None:
    """Recolour orphan fill pixels to the most common neighbour key; outline and accent keys never move."""
    h, w = len(rows), len(rows[0])
    for _ in range(2):
        for y in range(h):
            for x in range(w):
                ch = rows[y][x]
                if ch in ".K" or ch in ACCENTS:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if ch in around:
                    continue
                cand = [a for a in around if a not in ".K"]
                if cand:
                    rows[y][x] = max(sorted(set(cand)), key=cand.count)


# ------------------------------------------------------------------ the six key poses (design units)
def ragged_loin(x0, y0, drop=7.0, lean=0.0):
    """Ragged loincloth hanging down from the hips (gravity): the top edge sits just below the hip centre so the rump
    skin shows above it, and the hem has three torn points. ``lean`` trails the hem (motion)."""
    return [(x0 - 3.5, y0 - 0.5), (x0 + 3.5, y0 - 0.5), (x0 + 3.2 + lean, y0 - drop * 0.7),
            (x0 + 2.0 + lean, y0 - drop * 0.95), (x0 + 1.0 + lean, y0 - drop * 0.62), (x0 - 0.5 + lean, y0 - drop),
            (x0 - 1.6 + lean, y0 - drop * 0.58), (x0 - 2.8 + lean, y0 - drop * 0.9), (x0 - 3.6 + lean, y0 - drop * 0.45)]


def planted_hand(x, y):
    """Planted claw: three 1 px fingers 4 to 5 px long splayed forward along the contact row from a wrist at (x, y)."""
    return [[(x, y), (x + 2.0, y - 1.4), (x + 4.6, 0.6)],
            [(x, y), (x + 2.6, y - 0.2), (x + 5.4, y - 0.6)],
            [(x, y), (x + 1.8, y + 1.2), (x + 4.4, y + 2.0)]]


def hand(x, y, spread=1.0, down=True):
    """Three long fingers from a wrist point; tips reach toward the ground when ``down``."""
    s = -1 if down else 1
    return [[(x, y), (x + 2.2 * spread, y + s * 1.2), (x + 4.2 * spread, y + s * 1.8)],
            [(x, y), (x + 2.0 * spread, y + 0.2), (x + 4.4 * spread, y + s * 0.2)],
            [(x, y), (x + 1.5 * spread, y - s * 1.0), (x + 3.2 * spread, y - s * 1.8)]]


POSES: dict[str, dict] = {}

# KEY 0 idle: hunched crouch, knuckles near the floor, the big head thrust forward and low at shoulder height, a matted
# dread mop hanging down the back, bloodshot eye, clenched teeth and drool.
POSES["idle0"] = {
    "far_limbs": [L([(-5, 15), (1, 10), (0, 3), (5, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 21), (11, 14), (15, 8), (18, 4)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-10, 15), (-4, 21), (3, 24)], [4.0, 4.5, 4.1]),
    "near_legs": [L([(-9, 15), (-3, 9), (-6, 2), (-1, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 14, 8.5),
    "head": head(13, 25), "hair": hair(13, 25),
    "near_arms": [L([(4, 21), (9, 14), (12, 7), (15, 3)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": planted_hand(15, 3),
    "ribs": [(-4, 14, 20), (-1, 15, 21), (2, 16, 22)],
    "grime": [(-13, 13), (-6, 20), (9, 11), (-4, 6), (-9, 17), (13, 6), (-1, 26), (-7, 11)],
    "face": "snarl", "face_at": (13, 25),
}

# KEY 3 walk contact (crouched scuttle): body low and forward, near arm planting ahead, far leg trailing, dreads streaming.
POSES["walk1"] = {
    "far_limbs": [L([(-8, 13), (-14, 8), (-17, 2), (-13, 0.8)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 16), (5, 10), (8, 4), (11, 1)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-10, 13), (-3, 16), (5, 19)], [4.0, 4.4, 4.0]),
    "near_legs": [L([(-9, 13), (-1, 9), (2, 3), (7, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 12, 7.5, -1.5),
    "head": head(16, 20), "hair": hair(16, 20, sweep=0.6),
    "near_arms": [L([(5, 17), (12, 12), (18, 7), (21, 3)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": planted_hand(21, 3),
    "ribs": [(-3, 12, 17), (0, 13, 18), (3, 14, 19)],
    "grime": [(-12, 11), (-5, 15), (0, 6), (-9, 15), (16, 6), (1, 22), (-6, 10)],
    "face": "snarl", "face_at": (16, 20),
}

# KEY 8 tell hold: coiled to spring, haunches raised and loaded behind, shoulders and head dropped low and thrust
# forward, jaw hanging open, both hands planted wide. The lowest, most compact grounded pose: it reads as "about to go".
POSES["tell2"] = {
    "far_limbs": [L([(-9, 16.5), (-3, 9.5), (-7, 1.8), (-2, 0.8)], [2.0, 1.6, 1.1, 0.8]),
                  L([(5, 12), (11, 7), (16, 3), (19, 2.4)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-12, 17.5), (-5, 16), (3, 12)], [4.3, 4.6, 4.1]),
    "near_legs": [L([(-11, 17.5), (-5, 10.5), (-9, 2), (-4, 0.8)], [2.7, 2.2, 1.4, 1.0])],
    "loin": ragged_loin(-11, 17.5, 7.0, 0.0),
    "head": head(13, 14), "jaw": jaw(13, 14, 0.7), "hair": hair(13, 14),
    "near_arms": [L([(4, 12), (8, 7), (12, 4), (15, 2.6)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": planted_hand(15, 2.6),
    "ribs": [(-5, 12, 18), (-2, 11, 17), (1, 10, 15)],
    "grime": [(-15, 16), (-8, 20), (-6, 6), (-1, 17), (-12, 9)],
    "face": "roar", "face_at": (13, 14),
}

# KEY 10 attack strike (airborne): body stretched flat, one claw high and one low with the gaping mouth between them,
# legs trailing, dreads streaming straight back.
POSES["leap3"] = {
    "far_limbs": [L([(-12, 19), (-18, 15), (-24, 12), (-28, 11)], [2.0, 1.6, 1.1, 0.8]),
                  L([(7, 25), (14, 30), (21, 31), (25, 29)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-13, 17), (-5, 20), (4, 22)], [3.8, 4.3, 3.9]),
    "near_legs": [L([(-12, 18), (-17, 13), (-21, 8), (-25, 6)], [2.3, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(-12, 17, 5.5, -4.0),
    "head": head(15, 24), "jaw": jaw(15, 24, 0.9), "hair": hair(15, 24, sweep=1.0),
    "near_arms": [L([(4, 20), (11, 16), (18, 15), (24, 16)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(24, 16, 1.4, down=False) + hand(25, 29, 1.4, down=True),
    "ribs": [(-5, 17, 20), (-2, 18, 21), (1, 19, 22)],
    "grime": [(-10, 17), (-20, 10), (-3, 22), (-7, 23), (1, 25)],
    "face": "roar", "face_at": (15, 24),
}

# KEY 12 hurt: thrown upright and back, head snapped up, dreads whipping forward, near arm flung, eye squeezed shut.
POSES["hurt4"] = {
    "far_limbs": [L([(1, 15), (6, 9), (7, 3), (10, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(-5, 23), (-1, 19), (1, 14), (2, 10)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-1, 14), (-4, 21), (-8, 26)], [4.0, 4.3, 3.9]),
    "near_legs": [L([(-1, 14), (4, 9), (0, 4), (-4, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-1, 14, 7.5, 1.0),
    "head": head(-4, 31, tilt=-2.5), "jaw": jaw(-4, 31, 0.8), "hair": hair(-4, 31, sweep=-0.6),
    "near_arms": [L([(-7, 24), (-12, 27), (-16, 31), (-18, 35)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(-18, 35, -1.0, down=False),
    "ribs": [(-5, 15, 21), (-2, 16, 22)],
    "grime": [(-9, 18), (0, 10), (-3, 5), (-14, 29)],
    "face": "pain", "face_at": (-4, 31),
}

# KEY 15 defeat (final): fallen backward the way the hurt throws it: on its back, head behind at x -10 with the chin
# on the chest and the mop spilled on the floor behind it, knee raised and feet toward the player, near arm flung up
# and back, tongue lolling, eye crossed.
POSES["down5"] = {
    "far_limbs": [L([(6, 4), (11, 3), (16, 1.5), (19, 1)], [1.8, 1.5, 1.1, 0.8]),
                  L([(-4, 4), (0, 2.5), (4, 1.5), (8, 1)], [1.6, 1.3, 1.0, 0.8])],
    "torso": L([(7, 4), (0, 5), (-6, 5.5)], [3.4, 3.6, 3.4]),
    "near_legs": [L([(7, 4), (10, 10), (14, 7), (17, 1.5)], [2.2, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(6, 5, 3.5, 0.0),
    "head": head(-10, 6, tilt=0.0), "hair": hair(-10, 6),
    "near_arms": [L([(-3, 7), (-3, 11), (-1, 15), (2, 17)], [1.9, 1.5, 1.1, 0.8])],
    "fingers": hand(2, 17, 0.7, down=False),
    "ribs": [(-3, 3, 7), (0, 3, 8), (3, 3, 7)],
    "grime": [(2, 5), (-3, 3), (10, 7), (15, 3)],
    "face": "dead", "face_at": (-10, 6),
}

# KEY 11 land (attack_land, the punish window): off balance and spent, never the tell. Body squashed flat and long,
# rump settled LOW (not loaded), head dipped below the tell height between the splayed claws, the far leg skidding out
# behind, ropes flopped forward by the impact, half-lidded eye and a slack panting mouth (no roar).
POSES["land1"] = {
    "far_limbs": [L([(-11, 9), (-17, 5), (-22, 2), (-25, 0.8)], [2.0, 1.6, 1.1, 0.8]),
                  L([(3, 9), (4, 5), (2, 2), (0, 1)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-11, 9), (-4, 11), (4, 10)], [4.2, 4.6, 4.2]),
    "near_legs": [L([(-10, 9), (-4, 6), (-9, 1.5), (-4, 0.8)], [2.7, 2.2, 1.4, 1.0])],
    "loin": ragged_loin(-11, 9, 6.0, 0.0),
    "head": head(15, 8), "jaw": jaw(15, 8, 0.3), "hair": hair(15, 8, sweep=-0.8),
    "near_arms": [L([(4, 10), (11, 7), (18, 4), (23, 2.4)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": planted_hand(23, 2.4),
    "ribs": [(-6, 7, 12), (-3, 7, 13), (0, 7, 12)],
    "grime": [(-13, 10), (-6, 12), (-1, 13), (-18, 4), (8, 5)],
    "face": "pant", "face_at": (15, 8),
}

# ------------------------------------------------------------------ the ten in-betweens (spec section 8)
def blend(a, b, t):
    """Interpolate two pose structures of the same shape; numbers lerp, anything of a different shape or any string
    snaps to ``a`` before the halfway point and to ``b`` after it."""
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a + (b - a) * t
    if isinstance(a, dict) and isinstance(b, dict):
        return {k: blend(a[k], b[k], t) if k in b else a[k] for k in a} | {k: b[k] for k in b if k not in a and t >= 0.5}
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)) and len(a) == len(b):
        return [blend(x, y, t) for x, y in zip(a, b)]
    return a if t < 0.5 else b


def shifted(pts, dx=0.0, dy=0.0):
    return [(x + dx, y + dy) for x, y in pts]


I0 = POSES["idle0"]
# 1: idle breath: head and shoulders 1 px lower, dread tips swayed 1 px back; the hand and the feet stay planted
POSES["idle1"] = I0 | {
    "far_limbs": [I0["far_limbs"][0], L([(6, 20), (11, 14), (15, 8), (18, 4)], I0["far_limbs"][1]["r"])],
    "torso": L([(-10, 15), (-4, 20.2), (3, 23)], I0["torso"]["r"]),
    "head": head(13, 24), "hair": hair(13, 24, sway=1.0), "face_at": (13, 24),
    "near_arms": [L([(4, 19), (9, 13.6), (12, 7), (15, 3)], I0["near_arms"][0]["r"])],
    "ribs": [(-4, 14, 19), (-1, 15, 20), (2, 16, 21)],
}
# 2: back to the key posture with the dread tips swayed 1 px the other way (forward)
POSES["idle2"] = I0 | {"hair": hair(13, 25, sway=-1.0)}

# walk: a crouched scuttle. 3 (key) = contact A; 4 = near leg and hand planted and sliding back under the body, far arm
# swinging forward, body up 1; 5 = passing, legs gathered under the hips, body highest; 6 = contact B, the mirror of 3
# with the limbs swapped (far leg and far arm planted ahead, near leg trailing and lifting off).
W3 = POSES["walk1"]
POSES["walk2"] = W3 | {
    "far_limbs": [L([(-8, 14), (-12, 9.5), (-11, 4), (-7, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 17), (10, 14), (15, 12.5), (19, 10)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-10, 14), (-3, 17), (5, 20)], [4.0, 4.4, 4.0]),
    "near_legs": [L([(-9, 14), (-4, 9.5), (-2, 4), (2, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 13, 7.5, -1.0),
    "head": head(16, 21), "hair": hair(16, 21, sweep=0.5), "face_at": (16, 21),
    "near_arms": [L([(5, 18), (9, 13), (13, 8), (16, 3)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": planted_hand(16, 3),
    "ribs": [(-3, 13, 18), (0, 14, 19), (3, 15, 20)],
    "grime": [(-12, 12), (-5, 16), (-2, 6), (-9, 16), (12, 6), (1, 23), (-6, 11)],
}
POSES["walk3"] = W3 | {
    "far_limbs": [L([(-8, 15), (-6, 10), (-7, 5), (-5.2, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 18), (12, 13), (17, 7), (20, 1.2)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-10, 15), (-3, 18), (5, 21)], [4.0, 4.4, 4.0]),
    "near_legs": [L([(-9, 15), (-5, 10), (-3, 5), (-1, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(-8, 14, 7.5, -0.5),
    "head": head(15, 22), "hair": hair(15, 22, sweep=0.35), "face_at": (15, 22),
    "near_arms": [L([(5, 19), (9, 14), (12, 9), (14, 4.5)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(14, 4.5, 0.8, down=True),
    "ribs": [(-3, 14, 19), (0, 15, 20), (3, 16, 21)],
    "grime": [(-12, 13), (-5, 17), (-3, 6), (-9, 17), (9, 6), (1, 24), (-6, 12)],
}
POSES["walk4"] = W3 | {
    "far_limbs": [L([(-8, 13), (-1, 9), (2, 3), (7, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 16), (12, 12), (18, 7), (21, 3)], [1.7, 1.4, 1.1, 0.8])],
    "near_legs": [L([(-9, 13), (-14, 8.5), (-18, 3), (-14.5, 0.8)], [2.4, 1.9, 1.3, 0.9])],
    "near_arms": [L([(5, 17), (8, 14), (11, 9), (12, 6)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(12, 6, 0.8, down=True),
    "far_fingers": planted_hand(21, 3),
    "grime": [(-12, 11), (-5, 15), (3, 6), (-9, 15), (10, 6), (1, 22), (-6, 10)],
}

# 7: tell onset, halfway from the idle to the coil, the mouth still closed
POSES["tell1"] = blend(I0, POSES["tell2"], 0.5) | {"face": "snarl"}
POSES["tell1"].pop("jaw", None)
# 9: launch, a nose-up push-off: body angled about 25 degrees up from the hips (-11, 10) to the shoulders (3, 16), hind legs
# thrown down and back with the toes off the floor, both claws coming up past the head
POSES["leap2"] = {
    "far_limbs": [L([(-10, 11), (-14, 8), (-18, 6), (-21, 4.6)], [2.0, 1.6, 1.1, 0.8]),
                  L([(6, 18), (12, 22), (18, 25), (22, 26)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(-11, 10), (-4, 13), (3, 16)], [4.0, 4.4, 4.0]),
    "near_legs": [L([(-11, 10), (-15, 7), (-19, 5), (-22, 4.2)], [2.3, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(-11, 10, 6.0, -3.0),
    "head": head(14, 21), "jaw": jaw(14, 21, 0.9), "hair": hair(14, 21, sweep=0.8),
    "near_arms": [L([(4, 15), (10, 16), (16, 18), (21, 20)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(21, 20, 1.2, down=False) + hand(22, 26, 1.2, down=True),
    "ribs": [(-6, 10, 14), (-3, 11, 15), (0, 12, 16)],
    "grime": [(-10, 11), (-18, 6), (-2, 14), (-6, 13), (3, 17)],
    "face": "roar", "face_at": (14, 21),
}
# 13, 14: the defeat falls backward the way the hurt throws it. 13 = the fall beat (shoulders and head behind the hips,
# toes still on the floor, near arm flung up and back, pain face); 14 = floor impact, key 15 bounced 1 px with wider arms
POSES["down3"] = {
    "far_limbs": [L([(1, 10), (6, 6), (8, 3), (11, 1)], [2.0, 1.6, 1.1, 0.8]),
                  L([(-3, 16), (0, 12), (2, 8), (3, 4)], [1.7, 1.4, 1.1, 0.8])],
    "torso": L([(2, 9), (-2, 13), (-6, 17)], [4.0, 4.3, 3.9]),
    "near_legs": [L([(2, 9), (7, 7), (9, 4), (8, 1)], [2.4, 1.9, 1.3, 0.9])],
    "loin": ragged_loin(2, 9, 6.0, 1.0),
    "head": head(-8, 21, tilt=-2.0), "jaw": jaw(-8, 21, 0.8), "hair": hair(-8, 21, sweep=-0.6),
    "near_arms": [L([(-5, 16), (-9, 21), (-12, 25), (-14, 28)], [2.0, 1.6, 1.2, 0.9])],
    "fingers": hand(-14, 28, -1.0, down=False),
    "ribs": [(-2, 10, 14), (1, 9, 13)],
    "grime": [(-4, 12), (2, 5), (-10, 19), (7, 6)],
    "face": "pain", "face_at": (-8, 21),
}
D15 = POSES["down5"]
POSES["down4"] = D15 | {      # impact bounce: the whole body rebounds off the floor, limbs flung up
    "far_limbs": [L([(6, 6), (11, 5), (17, 2.5), (21, 1)], [1.8, 1.5, 1.1, 0.8]),
                  L([(-4, 6), (0, 4.5), (5, 2.5), (10, 1)], [1.6, 1.3, 1.0, 0.8])],
    "torso": L([(7, 6), (0, 7), (-6, 7.5)], [3.4, 3.6, 3.4]),
    "near_legs": [L([(7, 6), (10, 12.5), (14, 10), (17, 4)], [2.2, 1.8, 1.3, 0.9])],
    "loin": ragged_loin(6, 7, 3.5, 0.0),
    "head": head(-10, 8.5, tilt=0.0), "hair": hair(-10, 8.5, sway=1.5), "face_at": (-10, 8.5),
    "near_arms": [L([(-3, 9), (-6, 13), (-6, 17), (-5, 19)], [1.9, 1.5, 1.1, 0.8])],
    "fingers": hand(-5, 19, 0.7, down=False),
    "ribs": [(-3, 5, 9), (0, 5, 10), (3, 5, 9)],
}

KEY_FRAMES = {0: "idle0", 3: "walk1", 8: "tell2", 10: "leap3", 11: "land1", 12: "hurt4", 15: "down5"}
# the ten in-between frames, drawn as parts (spec section 8); frame 11 is generated from the tell pose plus dust
INBETWEEN = {1: "idle1", 2: "idle2", 4: "walk2", 5: "walk3", 6: "walk4", 7: "tell1", 9: "leap2", 13: "down3", 14: "down4"}
TAGS = [("idle", [(0, 12), (1, 8), (2, 12)]),
        ("walk", [(3, 6), (4, 6), (5, 6), (6, 6)]),
        ("attack_tell", [(7, 6), (8, 18)]),
        ("attack_leap", [(9, 3), (10, 8)]),
        ("attack_land", [(11, 14)]),
        ("hurt", [(12, 12)]),
        ("down", [(13, 4), (14, 8), (15, None)])]
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_leap": "once", "attack_land": "once",
            "hurt": "once", "down": "hold"}
AIRBORNE = ("attack_leap",)
TICK_HZ = 60
HOLD_MS = 100

# placeholder boxes (px from the pivot: x forward, y up), measured from the key bounding boxes; not approved mechanics
BOXES = {
    "standing": {"x": [-14, 21], "y": [0, 39]},
    "hurtbox_idle_walk": {"x": [-14, 21], "y": [0, 33]},
    "hurtbox_tell": {"x": [-16, 19], "y": [0, 22]},
    "hurtbox_land": {"x": [-14, 20], "y": [0, 15]},
    "hurtbox_leap": {"x": [-22, 21], "y": [6, 31]},
    "hurtbox_hurt": {"x": [-14, 2], "y": [0, 39]},
    "hurtbox_down": {"x": [-22, 19], "y": [0, 12]},
    "attack_hitbox_impact_frame_only": {"x": [14, 32], "y": [13, 33], "frame": 10},
}


def pose_for(n: int) -> dict:
    if n in KEY_FRAMES:
        return POSES[KEY_FRAMES[n]]
    return POSES[INBETWEEN[n]]


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    ticks = frame_ticks()
    n_frames = len(ticks)
    frames = [{"rows": build(pose_for(n)), "ms": duration_ms(ticks[n])} for n in range(n_frames)]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    tags = [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS]
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": tags,
    }


def manifest() -> dict:
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": "key" if n in KEY_FRAMES else "inbetween"} for n in range(len(ticks))]
    return {
        "schema_version": 1,
        "id": "troglodyte_pygmy_h40_candidate_v0",
        "type": "character_animation",
        "source_path": "assets/source/characters/troglodyte/trog_01.ase",
        "grid_source": "assets/source/characters/troglodyte/trog_01.grid.json",
        "output_atlas": "generated/art/trog_01.png",
        "output_metadata": "generated/art/trog_01.json",
        "palette_id": "trog-family-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 troglodyte Pygmy; runtime role IP-neutral small fast pouncer)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("TROG-01 Pygmy at a 40 px idle height: 7 keys (atlas frames 0, 3, 8, 10, 11, 12, 15) rasterized from pose rules by "
                       "scripts/author_trog01.py; the other ten slots are in-betweens drawn from the same part rules (frame 11, the landing, is a key of its own; landing dust is deferred to the VFX order); "
                       "layer body only"),
            "generator": "Draft spec and keys: Claude Sonnet. Design edit (construction, six keys): Claude Opus. In-betweens and execution: Claude Sonnet. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "pygmy": {
            "status": "six frozen keys plus ten drawn in-betweens, not yet reviewed by an independent art director; Bevy integration pending; 16 frames, 6 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/trog-01-spec.md",
            "opaque_height_idle": 40,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _f in TAGS if name not in AIRBORNE],
            "airborne_tags": list(AIRBORNE),
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/trog-01_keys.json",
            "attack_ticks": {"tell": [0, 23], "leap_launch": [24, 26], "leap_strike_impact": [27, 34], "land_recover": [35, 48]},
            "boxes_px_from_pivot": BOXES,
            "frames": frames,
        },
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-01/key0-scale.png") -> Path:
    """Scale evidence: key 0 at 1x on lit Floor 1 next to Carl and GOB-01 (640 x 360, no scaling), plus a 4x
    copy of key 0 on a side panel. Needs the Floor 1 kit atlases (``bash scripts/export_assets.sh``).
    Run: ``python3 scripts/author_trog01.py key0-scale``."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    gob, gob_piv = reference_idle("goblin")
    grid = load_grid_file(GRID)
    pygmy = grid_images(grid)[0]
    g_img, g_piv = flip(gob, gob_piv)
    p_img, p_piv = flip(pygmy, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (g_img, g_piv, CARL_X + 70, 0), (p_img, p_piv, CARL_X + 150, 0)]
    scene = Floor().render(entries, 1)
    panel_w = pygmy.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, scene.height), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = pygmy.resize((panel_w, pygmy.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 40), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def write_keys_x3(out: Path = ROOT / "docs/evidence/trog-01/keys-x3.png") -> Path:
    """The six keys left to right (0, 3, 8, 10, 12, 15) on lit Floor 1 brick, cropped around the actors and enlarged
    3x (nearest neighbour). Run: ``python3 scripts/author_trog01.py keys-x3``."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor

    imgs = grid_images(load_grid_file(GRID))
    entries = [(imgs[n], tuple(PIVOT), CARL_X - 80 + i * 82, 6 if n in (9, 10) else 0)
               for i, n in enumerate(sorted(KEY_FRAMES))]
    floor = Floor()
    lit = floor.render(entries, 1)
    ground = max(floor.ground(x) for _i, _p, x, _l in entries)
    crop = lit.crop((0, ground - 72, lit.width, ground + 10))
    big = crop.resize((crop.width * 3, crop.height * 3), Image.NEAREST)
    out.parent.mkdir(parents=True, exist_ok=True)
    big.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale", "keys-x3"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "keys-x3":
        print(f"wrote {write_keys_x3().relative_to(ROOT)}")
        return 0
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    targets = [(GRID, dump(frames_grid())), (MANIFEST, json.dumps(manifest(), indent=2) + "\n")]
    status = 0
    for path, text in targets:
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name} up to date" if same else f"{path.name} differs from the generator")
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    return status


if __name__ == "__main__":
    sys.exit(main())
