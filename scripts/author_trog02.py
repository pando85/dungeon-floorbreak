#!/usr/bin/env python3
"""TROG-02 authoring record: the Troglodyte Basher (variant of the TROG-01 Pygmy base), palette slots and hero key 0.

Usage:

    python3 scripts/author_trog02.py frames [--check]
    python3 scripts/author_trog02.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/basher/trog_02.grid.json`` (pixel-grid-v1, layer ``body``).
Design stage: only key 0 (``idle``) is drawn; the other keys and the in-betweens are added to ``POSES``, ``KEY_FRAMES``,
``INBETWEEN`` and ``TAGS`` by the execution stage from ``docs/02-art/trog-02-spec.md`` sections 4 and 7.

The family construction (part canvas, outline-by-part-order, rim light, despeckle, the 15-colour core palette) is
imported from ``scripts/author_trog01.py`` and redrawn at the Basher's 66 px from the rules in
``docs/02-art/trog-01-spec.md`` section 2b; nothing is scaled up from Pygmy pixels. Every pose is a rule (thick polylines,
discs and polygons in design units: x right from the pivot column, y up from the contact row) plus literal face
stamps. No raster image-model output, no tracing of concept pixels. ``--check`` compares instead of writing.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import author_trog01 as t1  # noqa: E402  (shared family construction)

ROOT = t1.ROOT
OUT_DIR = ROOT / "assets/source/characters/troglodyte/basher"
GRID = OUT_DIR / "trog_02.grid.json"
CANVAS = (112, 88)
PIVOT = (48, 87)
H = 66                                   # idle opaque height (family ladder, trog-01 spec 2b)

# trog-family-v0 core (15, frozen) + the Basher's two slots: forearm wrap mid and light. Necklace and shoulder trash
# reuse t, w, c, C. 17 of 20.
PALETTE = t1.PALETTE + [
    ("g", "wrap_mid", "#6b604fff", False),
    ("G", "wrap_light", "#9c8f74ff", False),
]
RAMPS = dict(t1.RAMPS)
RAMPS.update({
    "wrap": ("c", "c", "g", "G"),        # near forearm wraps: dirty linen over a rag-brown shade
    "farwrap": ("K", "c", "c", "g"),     # far forearm wraps in the far-limb light
    "trash": ("c", "c", "C", "C"),       # back junk heap (rubble, rags); can and bone are stamps
})
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


class Canvas(t1.Canvas):
    """The family part canvas on the Basher frame and ramps."""

    def __init__(self):
        super().__init__(CANVAS[0], CANVAS[1], PIVOT[1])
        self.px = PIVOT[0]

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1


L = t1.L

# ------------------------------------------------------------------ face stamps (Basher, redrawn at 66 px)
# 14 columns; top-left at (hx - 5, hy + 5) where (hx, hy) is the jaw-circle centre. Brow 2 rows over the eye at
# hx - 1 .. hx + 4; eye 2 x 2 white + 1 pupil column + 3 red (bloodshot); mouth from the back of the cheek (hx - 3) to
# the front edge (hx + 6). The Basher's own note: an underbite with two lower tusks (cols 3 and 10) rising through the
# gash past the upper teeth, so the teeth band is 3 rows (upper row, gash, lower row) plus the tusk tips.
FACES = {
    "snarl": ["....KKKKKK....",
              ".....KKKKK....",
              "....eewwK.....",
              ".....ewwK.....",
              ".....e....s...",
              "......t...t...",
              "....t.t.t.t...",
              ".ooototototo..",
              "..ooootoooto..",
              "..............",
              "...w..........",
              "...w.........."],
    # tell and smash: the jaw drops (jaw_b drop 1), 4-row gaping mouth that opens the front outline, tusks low
    "roar": ["....KKKKKK....",
             ".....KKKKK....",
             "....eewwK.....",
             ".....ewwK.....",
             "....TtTtTTtt..",
             "..oooooooooooo",
             "..oooooooooooo",
             "..oooooooooooo",
             "..ooooqqqqooo.",
             "..tTt.tT.tTt..",
             "...t......t...",
             "...w......w...",
             "...w.........."],
    # recover (after the smash): lid dropped over the eye, slack 1-row mouth short of the front, loose tusks, drool
    "pant": ["..............",
             "....KKKKKK....",
             "....eKKwK.....",
             ".....ewwK.....",
             "..........s...",
             ".......t..t...",
             ".....tT.tT....",
             "..ooooooooo...",
             "....t..t..t...",
             "....w..w......",
             "....w..w......",
             ".......w......"],
    # hurt: eye squeezed to a K slit, no red; open yelp, no tongue
    "pain": ["...KKKKKK.....",
             "....sKKKKK....",
             "..............",
             "..............",
             "..TtTtTtTt....",
             "..oooooooooo..",
             "..oooooooooo..",
             "..ooooooooo...",
             "..tTt.tT.tT...",
             "..............",
             "......w......."],
    # defeat: crossed eye, slack jaw, tongue out of the mouth corner (q >= 3)
    "dead": ["..............",
             ".....K.K......",
             "......K.......",
             ".....K.K......",
             "..............",
             "..............",
             "..ooooooooo...",
             "..tTt.tT.qq...",
             ".........qq...",
             "..........q..."],
}
for _name, _rows in FACES.items():
    assert all(len(r) == 14 for r in _rows), _name

HEAD_R = (6.5, 6.0)        # skull 0.10 H and jaw (0.93 of it); constant in every pose


def head(hx, hy, tilt=-1.6):
    """Skull circle 4.5 px behind and ``-tilt`` px above the jaw circle centred on (hx, hy)."""
    return L([(hx - 4.5, hy - tilt), (hx, hy)], list(HEAD_R))


def jaw_b(hx, hy, drop=1.0):
    """Dropped lower jaw for the roar and pain faces (0..1)."""
    return L([(hx - 6, hy - 3.5), (hx + 4, hy - 3.5 - 3.0 * drop)], [3.4, 2.8 + 0.3 * drop])


# Hair: a flatter mop than the Pygmy's (about 0.09 H above the skull top, so the small head stays small) and five rope
# dreads (radius 1.5) hanging down and back over the nape and the trash heap; longest rope 0.45 H behind the jaw.
MOP = [(-11, 4), (-11.5, 7.5), (-9.5, 9), (-8, 10.8), (-7, 8.8), (-6, 10.6), (-4, 11.4), (-3, 9.2), (-2, 10.8),
       (0, 11), (2, 9.6),
       (1, 8), (-2, 7), (-5.5, 5.5), (-8, 4)]
ROPES = [[(-9, 9), (-14, 8), (-19, 5), (-24, 0), (-26, -5)],
         [(-10, 6), (-14, 2), (-16, -3), (-16, -7)],
         [(-5, 10.5), (-10, 10.5), (-15, 9.5), (-20, 6)],
         [(-7, 7), (-10, 3), (-11, -2)],
         [(1, 9.6), (3.5, 8.6), (4.5, 7)]]          # the short front lock, ends above the brow
ROPE_R = 1.5
BEADS = {0: "T", 1: "C", 2: "T"}   # dull knots: a light bead on a rope behind the head reads as a second eye
EAR = [(-6, 4), (-14.5, 6), (-8.5, 1.5)]          # pointed, straight back, below the crown, skin over the mop


def hair(hx, hy, sweep=0.0, sway=0.0):
    mop = [(hx + x, hy + y) for x, y in MOP]
    ropes = []
    for rope in ROPES:
        n = len(rope) - 1
        ropes.append([(hx + x - 4.0 * sweep * i / n - sway * i / n, hy + y + 3.0 * sweep * i / n + 0.4 * sway * i / n)
                      for i, (x, y) in enumerate(rope)])
    return {"mop": mop, "ropes": ropes, "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def ragged_loin(x0, y0, drop=12.0, half=6.0, lean=0.0):
    """Wide ragged rag hanging down from just below the hip centre, four torn points."""
    return [(x0 - half, y0 - 0.5), (x0 + half, y0 - 0.5), (x0 + half - 0.4 + lean, y0 - drop * 0.7),
            (x0 + half * 0.6 + lean, y0 - drop * 0.95), (x0 + half * 0.25 + lean, y0 - drop * 0.6),
            (x0 - half * 0.1 + lean, y0 - drop), (x0 - half * 0.45 + lean, y0 - drop * 0.62),
            (x0 - half * 0.75 + lean, y0 - drop * 0.9), (x0 - half + lean, y0 - drop * 0.5)]


def planted_fist(x, y):
    """Knuckle-planted fist: a fist mass under the wrist, a 2 to 3 px knuckle block resting on the contact row and two
    1 to 2 px claw tips at its front (the third claw is hidden behind the block in profile)."""
    return [L([(x, y), (x + 2.8, 4.0)], [3.6, 3.6]),
            L([(x + 0.2, 1.6), (x + 5.6, 1.6)], [1.6, 1.5]),
            L([(x + 6.0, 1.1), (x + 7.4, 0.8)], [0.55, 0.55]),
            L([(x + 5.8, 4.4), (x + 7.0, 3.6)], [0.55, 0.55])]


def wrap_bands(rows, part, ids, bands, P):
    """Cloth wraps as colour bands on the forearm skin (no silhouette of their own): inside each band (design y range)
    lit skin goes to G, mid skin to G near the lit edge and g elsewhere, shade to g; the band's lowest row is the c fold.
    Skin rows stay between the bands."""
    for y_lo, y_hi in bands:
        top, bot = int(P(0, y_hi)[1]), int(P(0, y_lo)[1])
        for y in range(top, bot + 1):
            for x in range(len(rows[0])):
                if part[y][x] not in ids or rows[y][x] not in "lmsd":
                    continue
                if y == bot:
                    rows[y][x] = "c"
                elif rows[y][x] == "l" or (rows[y][x] == "m" and part[y][x - 2] not in ids):
                    rows[y][x] = "G"
                else:
                    rows[y][x] = "g" if rows[y][x] in "ms" else "c"


def build(pose: dict) -> list[str]:
    w, h = CANVAS
    c = Canvas()
    far = c.add("far"); far_wrap_ids = [c.add("farwrap") for _ in pose.get("far_wraps", [])]; skin = c.add("skin"); trash = c.add("trash")
    cloth = c.add("cloth"); head_id = c.add("skin"); hair_id = c.add("hair"); arm = c.add("skin")
    ear = c.add("skin")
    hd = pose["hair"]
    # back to front: far limbs (+ far wraps), torso and trapezius, trash heap, near leg, rag, head and jaw, mop and
    # ropes, near arm, near wrap, fingers, ear; then texture, grime, face and stamps
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for limb in pose.get("far_limbs_fore", []):
        c.thick(limb["pts"], limb["r"], far)
    for limb, wid in zip(pose.get("far_wraps", []), far_wrap_ids):
        c.thick(limb["pts"], limb["r"], wid)
    for fin in pose.get("far_fingers", []):
        c.thick(fin["pts"], fin["r"], far)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for cx, cy, r in pose.get("masses", []):
        c.disc(cx, cy, r, r, skin)
    for poly in pose.get("trash", []):
        c.poly(poly, trash)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["loin"], cloth)
    c.thick(pose["head"]["pts"], pose["head"]["r"], head_id)
    if "jaw" in pose:
        c.thick(pose["jaw"]["pts"], pose["jaw"]["r"], head_id)
    c.poly(hd["mop"], hair_id)
    for rope in hd["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    if "deltoid" in pose:                                # near shoulder mass, rimmed where the arm hangs from it
        dx_, dy_, dr = pose["deltoid"]
        c.disc(dx_, dy_, dr, dr, c.add("skin"))
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], arm)
    fore_ids = set()
    for limb in pose.get("near_arms_fore", []):
        fore_ids.add(c.add("skin"))
        c.thick(limb["pts"], limb["r"], max(fore_ids))
    for i, fin in enumerate(pose.get("fingers", [])):     # fist mass, knuckle block, claw tips: each its own part
        c.thick(fin["pts"], fin["r"], c.add("skin"))
    for e in hd["ears"]:
        c.poly(e, ear)
    rows = c.render()
    for y in range(h):                                   # rope texture (family rule)
        for x in range(w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
    for i, key in BEADS.items():
        bx, by = c.P(*hd["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    wrap_bands(rows, c.part, fore_ids, pose.get("wrap_bands", []), c.P)
    for y in range(h):                                   # mottled rim: no clean lit run longer than 5
        run = 0
        for x in range(w + 1):
            if x < w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    for crease in pose.get("creases", []):                # muscle separations: d strokes; bold ones are 2 px wide
        (x0, y0), (x1, y1), width = crease if len(crease) == 3 else (*crease, 1)
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for k in range(n + 1):
            px, py = c.P(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n)
            for dx in range(width):
                if rows[int(py)][int(px) + dx] in "mls":
                    rows[int(py)][int(px) + dx] = "d"
    for gx, gy in pose.get("grime", []):                  # 2 x 2 dirt smudges (deep skin) on any lit/mid/shade skin
        px, py = c.P(gx, gy)
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            if rows[int(py) + dy][int(px) + dx] in "mls":
                rows[int(py) + dy][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):               # dirt-caked soles and knuckles
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 5, hy + 5)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if rows[y][x] != "." or ch in "ewtTq":
                    rows[y][x] = ch
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        rows[int(py)][int(px)] = key
    t1.despeckle(rows)
    return ["".join(r) for r in rows]


def necklace(pts):
    """Stamped necklace: a 1 px string of alternating t (tooth) and w (bone) beads; the pendant (w) is stamped below."""
    out = []
    for i, (x, y) in enumerate(pts):
        out.append((x, y, "t" if i % 2 == 0 else "w"))
    return out


POSES: dict[str, dict] = {}

# KEY 0 idle: a hunched brute. Spine about 35 degrees from the hips (-16, 31) to the trapezius peak (2, 55); the small
# head sunk 4.5 px below that peak and 9 px ahead of it; both fists knuckle-planted; wrapped forearms thicker than the
# upper arms; a junk heap on the upper back under the hanging ropes; a tooth-and-bone necklace across the chest.
POSES["idle0"] = {
    "far_limbs": [L([(-20, 29), (-23, 17), (-27, 6), (-24, 1.6), (-19, 1.4)], [6.4, 4.8, 3.6, 2.2, 1.6]),     # far leg
                  L([(0, 41), (4, 32), (8, 25)], [4.6, 4.0, 4.0])],                                          # far upper arm
    "far_limbs_fore": [L([(8, 25), (10, 16), (11.5, 9)], [4.0, 3.6, 2.8])],
    "far_fingers": planted_fist(11.5, 8),
    "torso": L([(-18, 31), (-7, 40), (5, 47)], [8.0, 10.0, 9.5]),
    "masses": [(-6, 47, 8.5), (-1, 51.5, 9.0), (5, 50, 8.0)],                                      # trapezius masses
    "trash": [[(-20, 47), (-18, 51.5), (-15, 52.5), (-13, 50), (-15, 46.5)],
              [(-14, 53), (-12, 57.5), (-9, 58.5), (-7.5, 55.5), (-10, 52)],
              [(-24, 42), (-23, 45.5), (-20, 46.5), (-19, 43)]],
    "near_legs": [L([(-13, 30), (-4, 20), (-7, 8), (-7, 3), (0, 1.8), (3, 1.5)], [7.5, 5.6, 4.5, 2.8, 1.8, 1.3])],
    "loin": ragged_loin(-13, 28, 13.0, 7.0),
    "head": head(14, 52), "hair": hair(14, 52),
    "deltoid": (3, 39, 5.5),
    "near_arms": [L([(4, 38), (12, 31), (17, 25)], [5.0, 4.6, 4.6])],
    "near_arms_fore": [L([(17, 25), (20, 17), (22, 11)], [4.8, 4.4, 3.6])],
    "wrap_bands": [(21.5, 25), (16.5, 19.5), (11, 14.5)],
    "fingers": planted_fist(22, 9),
    "creases": [((-9, 34), (-5, 31)), ((8, 44), (12, 41), 2), ((11, 33), (14, 30), 2), ((-2, 40), (2, 37))],
    "grime": [(19, 20.5), (21, 15), (-4, 21), (-6, 17), (-9, 36), (-3, 33), (-7, 5), (1, 4), (-13, 25), (-14, 41)],
    "stamps": necklace([(9, 44), (10, 43), (11, 42), (12, 41), (13, 41), (14, 42), (15, 43)]) + [
        (12.5, 40, "w"), (12.5, 39, "w"), (-19, 47, "t"), (-18, 47, "t"), (-19, 46, "T"), (-18, 46, "T"),
        (-12, 55, "w"), (-11, 55.5, "w"), (-10, 56, "w")],
    "face": "snarl", "face_at": (14, 52),
}

# ------------------------------------------------------------------ keys 3 to 15 and the in-betweens (execution stage)
# Every pose reuses the key-0 parts through ``mk``: the same radii, deltoid, wrap bands, fist parts, face stamps, grime,
# creases, trash heap and necklace, moved with the joints. Joints are design units (x forward from the pivot, y up).
NEAR_ARM_R = [5.0, 4.6, 4.6]
NEAR_FORE_R = [4.8, 4.4, 3.6]
FAR_ARM_R = [4.6, 4.0, 4.0]
FAR_FORE_R = [4.0, 3.6, 2.8]
NEAR_LEG_R = [7.5, 5.6, 4.5, 2.8, 1.8, 1.3]
FAR_LEG_R = [6.4, 4.8, 3.6, 2.2, 1.6]
TORSO_R = [8.0, 10.0, 9.5]
K0 = POSES["idle0"]
K0_TRASH_AT = (-1, 51.5)                 # key-0 middle trapezius mass: the trash heap rides with it
K0_JAW = (14, 52)
K0_NECK = [(9, 44), (10, 43), (11, 42), (12, 41), (13, 41), (14, 42), (15, 43)]
K0_PENDANT = [(12.5, 40), (12.5, 39)]
K0_TRASH_STAMPS = [(-19, 47, "t"), (-18, 47, "t"), (-19, 46, "T"), (-18, 46, "T"), (-12, 55, "w"), (-11, 55.5, "w"),
                   (-10, 56, "w")]


def closed_fist(x, y):
    """Off the floor: the fist mass only (r 3.6) and one claw tip."""
    return [L([(x, y), (x + 1.2, y - 0.8)], [3.6, 3.4]), L([(x + 3.8, y - 1.6), (x + 4.8, y - 2.2)], [0.55, 0.55])]


def open_hand(x, y, fan):
    """The hurt hand: a palm and three 1 px fingers (r 0.55), 4 to 5 px long, fanned by ``fan`` (dx, dy per finger)."""
    out = [L([(x, y), (x, y + 1.2)], [3.1, 2.9])]
    for dx, dy in fan:
        out.append(L([(x + dx * 0.4, y + 3.0), (x + dx, y + 3.0 + dy)], [0.55, 0.55]))
    return out


def bands_for(elbow, wrist, spans=((0.0, 3.5), (5.0, 8.5), (10.5, 14.0))):
    """Wrap bands as design-y ranges along the forearm (counted from the wrist), dropping any band under 2 rows."""
    length = math.hypot(elbow[0] - wrist[0], elbow[1] - wrist[1])
    out = []
    for d0, d1 in spans:
        if d1 > length + 1.0:
            continue
        ya = wrist[1] + (elbow[1] - wrist[1]) * d0 / length
        yb = wrist[1] + (elbow[1] - wrist[1]) * min(d1, length) / length
        lo, hi = min(ya, yb), max(ya, yb)
        if hi - lo >= 2.0:
            out.append((round(lo, 1), round(hi, 1)))
    return out


def sh(pts, dx, dy):
    return [(x + dx, y + dy) for x, y in pts]


def mk(*, torso, masses, jaw, face, near_arm, near_fore, near_fist, far_arm, far_fore, far_fist, near_leg, far_leg,
       loin, deltoid, grime, creases, sweep=0.0, sway=0.0, jawdrop=None, neck_at=(0, 0), tr_off=(0, 0), tilt=-1.6,
       bands=None, far_wrap=False, trash=True, torso_r=None, neck_abs=None):
    hx, hy = jaw
    tdx = masses[1][0] - K0_TRASH_AT[0] + tr_off[0]
    tdy = masses[1][1] - K0_TRASH_AT[1] + tr_off[1]
    nx, ny = neck_abs or (hx - K0_JAW[0] + neck_at[0], hy - K0_JAW[1] + neck_at[1])
    pose = {
        "far_limbs": [L(far_leg, FAR_LEG_R), L(far_arm, FAR_ARM_R)],
        "far_limbs_fore": [L(far_fore, FAR_FORE_R)],
        "far_fingers": far_fist,
        "torso": L(torso, torso_r or TORSO_R),
        "masses": masses,
        "trash": [sh(p, tdx, tdy) for p in K0["trash"]] if trash else [],
        "near_legs": [L(near_leg, NEAR_LEG_R)],
        "loin": loin,
        "head": head(hx, hy, tilt), "hair": hair(hx, hy, sweep, sway),
        "deltoid": deltoid,
        "near_arms": [L(near_arm, NEAR_ARM_R)],
        "near_arms_fore": [L(near_fore, NEAR_FORE_R)],
        "wrap_bands": bands if bands is not None else bands_for(near_fore[0], near_fore[-1]),
        "fingers": near_fist,
        "creases": creases,
        "grime": grime,
        "stamps": necklace(sh(K0_NECK, nx, ny)) + [(x, y, "w") for x, y in sh(K0_PENDANT, nx, ny)] + (
            [(x + tdx, y + tdy, k) for x, y, k in K0_TRASH_STAMPS] if trash else []),
        "face": face, "face_at": jaw,
    }
    if jawdrop is not None:
        pose["jaw"] = jaw_b(hx, hy, jawdrop)
    if far_wrap:
        pose["far_wraps"] = [L(far_fore[:2], [FAR_FORE_R[0] - 0.4, FAR_FORE_R[1] - 0.4])]
    return pose


# ---- KEY 3: walk contact A. Body 2 down and 3 forward, near fist planted far ahead, far fist planted behind it.
POSES["walk3"] = mk(
    torso=[(-17, 29), (-5, 38), (8, 45)], masses=[(-3, 45, 8.5), (2, 49.5, 9.0), (8, 48, 8.0)], jaw=(17, 50),
    face="snarl", sweep=0.5,
    near_arm=[(6, 36), (12, 30), (18, 24)], near_fore=[(18, 24), (23, 17), (28, 10)], near_fist=planted_fist(28, 8),
    far_arm=[(3, 39), (2, 32), (1, 25)], far_fore=[(1, 25), (1.5, 17), (2, 10)], far_fist=planted_fist(2, 8),
    near_leg=[(-12, 28), (-6, 18), (-12, 9), (-14, 5), (-11, 2), (-8, 1.5)],
    far_leg=[(-19, 28), (-14, 17), (-15, 6), (-12, 2.6), (-9, 1.5)],
    loin=ragged_loin(-13, 26, 13.0, 7.0, lean=-1.0), deltoid=(6, 37, 5.5),
    creases=[((-9, 32), (-5, 29)), ((11, 42), (15, 39), 2), ((13, 31), (16, 28), 2), ((-2, 38), (2, 35))],
    grime=[(23, 19), (25, 13), (-1, 19), (-8, 15), (-9, 34), (-3, 31), (-9, 5), (3, 20), (-13, 23), (-14, 39)],
)

# ---- KEY 8: wind-up. Reared up on a wide braced stance, both fists locked overhead, mouth gaping.
POSES["tell8"] = mk(
    torso=[(-10, 30), (-6, 42), (-2, 54)], masses=[(-8, 53, 8.5), (-6, 58, 8.5), (0, 57, 8.0)], jaw=(6, 58),
    face="roar", jawdrop=1.0, sweep=0.0,
    near_arm=[(-4, 52), (-6, 59), (-6, 66)], near_fore=[(-6, 66), (-4, 70), (-1, 73)], near_fist=closed_fist(0, 75),
    far_arm=[(-4, 52), (0, 59), (4, 66)], far_fore=[(4, 66), (2, 70), (-1, 73)], far_fist=closed_fist(-3, 75.5),
    near_leg=[(-6, 28), (2, 17), (1, 8), (1, 4), (5, 1.8), (8, 1.5)],
    far_leg=[(-14, 28), (-20, 16), (-22, 6), (-20, 2.6), (-16, 1.5)],
    loin=ragged_loin(-10, 28, 13.0, 7.0), deltoid=(-4, 52, 5.5), neck_at=(-2, -2),
    creases=[((-2, 47), (2, 44), 2), ((-8, 44), (-4, 41)), ((-5, 58), (-1, 55))],
    grime=[(-6, 63), (-3, 68), (-9, 40), (-2, 44), (-12, 48), (-12, 33), (4, 20), (-4, 24), (-3, 10), (-18, 12)],
    far_wrap=True,
)

# ---- KEY 10: smash impact. Pitched forward and down, both fists flat on the floor far ahead.
POSES["smash10"] = mk(
    torso=[(-16, 30), (-4, 34), (8, 35)], masses=[(-2, 38, 8.5), (4, 40, 8.0), (10, 38, 7.5)], jaw=(21, 35),
    face="roar", jawdrop=1.0, sweep=-0.8,
    near_arm=[(9, 33), (17, 28), (24, 22)], near_fore=[(24, 22), (29, 16), (33, 10)], near_fist=planted_fist(33, 8),
    far_arm=[(6, 34), (14, 28), (21, 22)], far_fore=[(21, 22), (26, 16), (30, 10)], far_fist=planted_fist(30, 8),
    near_leg=[(-12, 28), (-4, 17), (-9, 7), (-8, 3), (-3, 1.8), (0, 1.5)],
    far_leg=[(-20, 28), (-27, 16), (-33, 7), (-34, 3), (-30, 1.5)],
    loin=ragged_loin(-16, 28, 11.0, 7.0, lean=3.0), deltoid=(9, 33, 5.5), neck_at=(2, -2),
    creases=[((-9, 33), (-5, 30)), ((12, 38), (16, 35), 2), ((16, 28), (19, 25), 2), ((0, 36), (4, 33))],
    grime=[(27, 20), (30, 13), (-4, 22), (-8, 12), (-9, 34), (-3, 31), (-6, 5), (3, 20), (-12, 36), (-12, 24)],
    far_wrap=True, tr_off=(0, -2),
)

# ---- KEY 11: spent. Key 10 slumped: hips and shoulders lower, head dropped 5 px, panting, fingers relaxed.
POSES["recover11"] = mk(
    torso=[(-15, 26), (-4, 30), (8, 32)], masses=[(-2, 34, 8.5), (4, 36, 8.0), (9, 34, 7.5)], jaw=(18, 31),
    face="pant", sweep=0.0,
    near_arm=[(7, 29), (14, 25), (21, 20)], near_fore=[(21, 20), (25, 15), (29, 10)], near_fist=planted_fist(29, 8),
    far_arm=[(5, 31), (12, 26), (18, 21)], far_fore=[(18, 21), (22, 16), (26, 10)], far_fist=planted_fist(26, 8),
    near_leg=[(-11, 25), (-3, 15), (-8, 6), (-8, 3), (-3, 1.8), (0, 1.5)],
    far_leg=[(-19, 25), (-22, 14), (-25, 6), (-24, 3), (-20, 1.5)],
    loin=ragged_loin(-15, 25, 11.0, 7.0, lean=1.0), deltoid=(7, 29, 5.5),
    creases=[((-9, 30), (-5, 27)), ((9, 34), (13, 31), 2), ((13, 25), (16, 22), 2), ((0, 32), (4, 29))],
    grime=[(23, 15), (26, 12), (-4, 20), (-8, 11), (-9, 29), (-3, 27), (-6, 5), (3, 17), (-12, 22), (-12, 33)],
    tr_off=(0, -2), neck_at=(0, -1),
)

# ---- KEY 12: hurt. Thrown back, torso tilted back, head above the trapezius, near arm flung back and up.
POSES["hurt12"] = mk(
    torso=[(-6, 28), (-10, 39), (-14, 50)], masses=[(-17, 47, 8.5), (-13, 50, 8.5), (-9, 48, 8.0)], jaw=(-8, 60),
    face="pain", jawdrop=0.6, sweep=-0.6, tilt=0.0,
    near_arm=[(-13, 47), (-22, 45), (-30, 44)], near_fore=[(-30, 44), (-29, 50), (-28, 56)], near_fist=open_hand(-28, 57, [(-3, 2.2), (0, 2.8), (3, 2.2)]),
    far_arm=[(-12, 48), (-9, 42), (-7, 36)], far_fore=[(-7, 36), (-3, 31), (2, 26)], far_fist=closed_fist(2, 24),
    near_leg=[(-6, 28), (4, 17), (2, 8), (3, 4), (7, 1.8), (10, 1.5)],
    far_leg=[(-10, 28), (-13, 17), (-13, 7), (-11, 3), (-7, 1.5)],
    loin=ragged_loin(-6, 28, 13.0, 7.0, lean=2.0), deltoid=(-13, 47, 5.5), neck_at=(2, -3),
    creases=[((-17, 45), (-13, 42), 2), ((-9, 33), (-5, 30)), ((-12, 41), (-8, 38))],
    grime=[(-26, 49), (-8, 22), (-10, 36), (-8, 41), (-14, 54), (-3, 28), (-7, 5), (3, 13), (-26, 42), (-11, 32)],
    far_wrap=True, tr_off=(1, 0),
)

# ---- KEY 15: fallen on its back, feet toward the player, tongue out. The head (-26, 8) is behind the shoulders.
_h15 = hair(-26, 8)
_h15["ropes"] = [[(-31, 10), (-37, 9), (-42, 7), (-45, 5)], [(-30, 7), (-36, 6), (-41, 4.5)],
                 [(-31, 13), (-37, 14), (-41, 12.5)]]
POSES["down15"] = mk(
    torso=[(6, 7), (-4, 9), (-14, 10)], masses=[(-8, 11, 6.5), (-3, 13, 6.5), (2, 11.5, 6.0)], jaw=(-26, 8),
    face="dead", sweep=0.0, tilt=0.0,
    near_arm=[(-12, 11), (-16, 14), (-19, 17)], near_fore=[(-19, 17), (-12, 20), (-6, 21)], near_fist=closed_fist(-5, 20.5), bands=[(19.2, 22.2), (16.2, 19.2)],
    far_arm=[(-10, 9), (-2, 5), (6, 3)], far_fore=[(6, 3), (12, 2.6), (16, 2)], far_fist=closed_fist(17, 3),
    near_leg=[(6, 8), (10, 16), (15, 10), (19, 4), (22, 2.2), (25, 1.5)],
    far_leg=[(4, 6), (11, 5), (17, 3), (22, 1.8), (25, 1.4)],
    loin=ragged_loin(4, 6, 3.5, 5.0, lean=1.0), deltoid=(-12, 11, 5.5), neck_abs=(-15, -29),
    creases=[((-9, 8), (-5, 6)), ((-12, 14), (-8, 12))],
    grime=[(-15, 17), (-9, 20), (9, 13), (12, 8), (-8, 13), (0, 9), (18, 3), (-12, 7), (4, 6), (-3, 10)],
    trash=False,
)
POSES["down15"]["hair"] = _h15
POSES["down15"]["trash"] = [[(-33, 0.5), (-32.5, 3.5), (-30, 4), (-29, 0.5)], [(-39, 0.5), (-38.5, 3.8), (-36, 4), (-35, 0.5)],
                            [(-45, 0.5), (-44.5, 3), (-42, 3.5), (-41, 0.5)]]

# ---- in-betweens
I0 = POSES["idle0"]
POSES["idle1"] = I0 | {
    "torso": L([(-18, 30), (-7, 39), (5, 46)], TORSO_R),
    "masses": [(-6, 46, 8.5), (-1, 50.5, 9.0), (5, 49, 8.0)],
    "head": head(14, 51), "hair": hair(14, 51, sway=1.0), "face_at": (14, 51),
    "trash": [sh(p, 0, -1) for p in I0["trash"]],
    "stamps": [(x, y - 1, k) for x, y, k in I0["stamps"]],
    "deltoid": (3, 38, 5.5),
}
POSES["idle2"] = I0 | {"hair": hair(14, 52, sway=-1.0)}

W3 = POSES["walk3"]
POSES["walk4"] = mk(
    torso=[(-17, 30), (-5, 39), (8, 46)], masses=[(-3, 46, 8.5), (2, 50.5, 9.0), (8, 49, 8.0)], jaw=(17, 51),
    face="snarl", sweep=0.5,
    near_arm=[(6, 37), (11, 31), (16, 25)], near_fore=[(16, 25), (21, 18), (25, 10)], near_fist=planted_fist(25, 8),
    far_arm=[(3, 40), (7, 33), (10, 26)], far_fore=[(10, 26), (12, 19), (13, 12)], far_fist=closed_fist(13.5, 8.6),
    near_leg=[(-13, 29), (-8, 19), (-15, 9), (-17, 5), (-14, 2), (-11, 1.5)],
    far_leg=[(-19, 29), (-12, 19), (-8, 10), (-6, 5), (-3, 1.5)],
    loin=ragged_loin(-13, 27, 13.0, 7.0, lean=-1.0), deltoid=(6, 38, 5.5),
    creases=[((-9, 33), (-5, 30)), ((11, 43), (15, 40), 2), ((12, 32), (15, 29), 2), ((-2, 39), (2, 36))],
    grime=[(21, 19), (23, 13), (7, 21), (-8, 16), (-9, 35), (-3, 32), (-9, 5), (3, 21), (-13, 24), (-14, 40)],
)
POSES["walk5"] = mk(
    torso=[(-17, 30.5), (-6, 39.5), (6, 46.5)], masses=[(-5, 46, 8.5), (0, 50.5, 9.0), (6, 49, 8.0)], jaw=(15, 51),
    face="snarl", sweep=0.35,
    near_arm=[(4, 38), (9, 31), (13, 25)], near_fore=[(13, 25), (14, 18), (15, 12)], near_fist=closed_fist(15, 8.2),
    far_arm=[(2, 40), (9, 33), (15, 27)], far_fore=[(15, 27), (18, 20), (21, 12)], far_fist=planted_fist(20, 8),
    near_leg=[(-12, 29), (-9, 19), (-17, 12), (-19, 8), (-17, 5), (-14, 4.5)],
    far_leg=[(-18, 29), (-12, 18), (-8, 8), (-6, 3), (-3, 1.5)],
    loin=ragged_loin(-13, 28, 13.0, 7.0, lean=-0.5), deltoid=(4, 38, 5.5),
    creases=[((-9, 33), (-5, 30)), ((9, 43), (13, 40), 2), ((10, 33), (13, 30), 2), ((-3, 39), (1, 36))],
    grime=[(10, 20), (13, 15), (15, 22), (-9, 17), (-9, 35), (-3, 32), (-5, 6), (2, 21), (-13, 26), (-14, 41)],
)
POSES["walk6"] = mk(
    torso=W3["torso"]["pts"], masses=W3["masses"], jaw=(17, 50),
    face="snarl", sweep=0.5,
    near_arm=[(6, 36), (8, 29), (10, 23)], near_fore=[(10, 23), (14, 17), (18, 11)], near_fist=closed_fist(18, 8.4),
    far_arm=[(3, 39), (12, 30), (20, 24)], far_fore=[(20, 24), (25, 17), (30, 10)], far_fist=planted_fist(30, 8),
    near_leg=[(-17, 28), (-10, 18), (-8, 9), (-8, 5), (-6, 2), (-3, 1.5)],
    far_leg=[(-12, 28), (-9, 17), (-16, 6), (-17, 2.6), (-14, 1.5)],
    loin=ragged_loin(-13, 26, 13.0, 7.0, lean=-1.0), deltoid=(6, 37, 5.5),
    creases=[((-9, 32), (-5, 29)), ((11, 42), (15, 39), 2), ((13, 31), (16, 28), 2), ((-2, 38), (2, 35))],
    grime=[(13, 18), (16, 13), (-1, 19), (-8, 15), (-9, 34), (-3, 31), (-9, 5), (3, 20), (-13, 23), (-14, 39)],
)

# 7: rise, halfway from 0 to 8: hips up 1, shoulders up 6, forearms raised to the chest, mouth still closed
POSES["tell7"] = mk(
    torso=[(-14, 31), (-6, 41), (1, 50)], masses=[(-7, 49, 8.5), (-3, 54.5, 9.0), (3, 53, 8.0)], jaw=(10, 55),
    face="snarl", sweep=0.0,
    near_arm=[(0, 44), (7, 38), (11, 33)], near_fore=[(11, 33), (13, 39), (12, 46)], near_fist=closed_fist(12.5, 47),
    far_arm=[(-2, 45), (4, 40), (8, 35)], far_fore=[(8, 35), (9, 41), (9, 47)], far_fist=closed_fist(9, 48),
    near_leg=[(-10, 29), (-2, 19), (-4, 8), (-3, 4), (2, 1.8), (5, 1.5)],
    far_leg=[(-17, 29), (-21, 17), (-24, 6), (-22, 2.6), (-18, 1.5)],
    loin=ragged_loin(-11, 27, 13.0, 7.0), deltoid=(0, 44, 5.5), neck_at=(0, -1),
    creases=[((-8, 36), (-4, 33)), ((4, 47), (8, 44), 2), ((8, 36), (11, 33), 2), ((-3, 43), (1, 40))],
    grime=[(12, 36), (11, 41), (-3, 24), (-6, 18), (-9, 38), (-3, 35), (-8, 5), (2, 6), (-13, 29), (-14, 45)],
    tr_off=(0, 0),
)

# 9: downswing, 0.6 of the way from 8 to 10: fists locked ahead of the head, body pitched about 45 degrees
POSES["smash9"] = mk(
    torso=[(-14, 30), (-5, 40), (4, 47)], masses=[(-6, 47, 8.5), (-1, 52, 9.0), (5, 51, 8.0)], jaw=(12, 54),
    face="roar", jawdrop=1.0, sweep=0.6,
    near_arm=[(4, 43), (9, 44), (14, 44)], near_fore=[(14, 44), (19, 42.5), (24, 39.5)], near_fist=closed_fist(25, 38.8), bands=[(41.4, 45.6), (38.8, 41.6)],
    far_arm=[(2, 44), (8, 46), (13, 46)], far_fore=[(13, 46), (18, 44), (22, 41)], far_fist=closed_fist(22, 40.5),
    near_leg=[(-10, 29), (-2, 18), (-7, 7), (-6, 3), (-1, 1.8), (2, 1.5)],
    far_leg=[(-17, 29), (-24, 15), (-29, 6), (-28, 2.6), (-24, 1.5)],
    loin=ragged_loin(-13, 28, 12.0, 7.0, lean=1.5), deltoid=(4, 43, 5.5), neck_at=(0, -2),
    creases=[((-7, 36), (-3, 33)), ((7, 46), (11, 43), 2), ((16, 43), (19, 40), 2), ((-2, 41), (2, 38))],
    grime=[(22, 42), (18, 43), (-3, 24), (-6, 18), (-9, 38), (-3, 35), (-6, 5), (2, 8), (-13, 27), (-14, 42)],
)

# 13: fall beat, about 45 degrees back, toes still on the floor, near arm flung back
POSES["down13"] = mk(
    torso=[(2, 14), (-4, 21), (-10, 28)], masses=[(-14, 26, 8.0), (-10, 29, 8.0), (-6, 27, 7.5)], jaw=(-14, 33),
    face="pain", jawdrop=0.6, sweep=-0.4, tilt=0.0,
    near_arm=[(-9, 26), (-16, 25), (-23, 25)], near_fore=[(-23, 25), (-27, 28), (-31, 31)], near_fist=open_hand(-31, 31, [(-3, 2), (0, 2.6), (3, 2)]), bands=[(28.4, 32.5), (24.4, 27.6)],
    far_arm=[(-8, 27), (-4, 21), (0, 15)], far_fore=[(0, 15), (4, 11), (8, 7)], far_fist=closed_fist(9, 5),
    near_leg=[(3, 13), (9, 12), (8, 6), (8, 3), (11, 1.8), (14, 1.5)],
    far_leg=[(2, 12), (4, 6), (7, 3), (11, 1.6), (14, 1.4)],
    loin=ragged_loin(2, 12, 7.0, 6.0, lean=3.0), deltoid=(-9, 26, 5.5), neck_at=(0, -2),
    creases=[((-8, 22), (-4, 19)), ((-14, 24), (-10, 22), 2)],
    grime=[(-24, 26), (-8, 17), (-3, 21), (-5, 24), (-13, 30), (3, 14), (8, 4), (9, 6), (-17, 22), (-8, 27)],
    tr_off=(0, 0),
)

# 14: floor impact on the back: the key-15 layout, head and belly 1 px up, raised knee 1 higher, arms wider
D15 = POSES["down15"]
_h14 = hair(-26, 9)
_h14["ropes"] = [[(x, y + 1) for x, y in rope] for rope in _h15["ropes"]]
POSES["down14"] = mk(
    torso=[(6, 8), (-4, 10), (-14, 11)], masses=[(-8, 12, 6.5), (-3, 14, 6.5), (2, 12.5, 6.0)], jaw=(-26, 9),
    face="dead", sweep=0.0, tilt=0.0,
    near_arm=[(-12, 12), (-17, 15), (-21, 18)], near_fore=[(-21, 18), (-13, 21), (-7, 22)], near_fist=closed_fist(-6, 21.5), bands=[(20.2, 23.2), (17.2, 20.2)],
    far_arm=[(-10, 10), (-1, 6), (7, 4)], far_fore=[(7, 4), (13, 3.4), (18, 3)], far_fist=closed_fist(19, 3.6),
    near_leg=[(6, 9), (10, 17), (15, 11), (19, 4), (22, 2.2), (25, 1.5)],
    far_leg=[(4, 6), (11, 5), (17, 3), (22, 1.8), (25, 1.4)],
    loin=ragged_loin(4, 7, 3.5, 5.0, lean=1.0), deltoid=(-12, 12, 5.5), neck_abs=(-15, -29),
    creases=[((-9, 9), (-5, 7)), ((-12, 15), (-8, 13))],
    grime=[(-15, 18), (-9, 21), (9, 14), (12, 9), (-8, 14), (0, 10), (18, 3), (-12, 8), (4, 7), (-3, 11)],
    trash=False,
)
POSES["down14"]["hair"] = _h14
POSES["down14"]["trash"] = D15["trash"]

KEY_FRAMES = {0: "idle0", 3: "walk3", 8: "tell8", 10: "smash10", 11: "recover11", 12: "hurt12", 15: "down15"}
INBETWEEN = {1: "idle1", 2: "idle2", 4: "walk4", 5: "walk5", 6: "walk6", 7: "tell7", 9: "smash9", 13: "down13", 14: "down14"}
TAGS = [("idle", [(0, 14), (1, 10), (2, 14)]),
        ("walk", [(3, 8), (4, 8), (5, 8), (6, 8)]),
        ("attack_tell", [(7, 8), (8, 28)]),
        ("attack_smash", [(9, 3), (10, 8)]),
        ("attack_recover", [(11, 20)]),
        ("hurt", [(12, 14)]),
        ("down", [(13, 5), (14, 10), (15, None)])]
TICK_HZ = 60
HOLD_MS = 100


def pose_for(n: int) -> dict:
    return POSES[KEY_FRAMES[n]] if n in KEY_FRAMES else POSES[INBETWEEN[n]]


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    ticks = frame_ticks()
    frames = [{"rows": build(pose_for(n)), "ms": duration_ms(ticks[n])} for n in range(len(ticks))]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS],
    }


MANIFEST = ROOT / "assets/manifests/trog-02.json"
SOURCE_ASE = "assets/source/characters/troglodyte/basher/trog_02.ase"
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_smash": "once", "attack_recover": "once",
            "hurt": "once", "down": "hold"}


def _extent(rows: list[str], y_max_up: int | None = None, x_min: int | None = None) -> tuple[int, int, int, int]:
    """Opaque extent of one frame as (x0, x1, y0, y1) in px from the pivot (x forward, y up), optionally limited to the
    pixels at most ``y_max_up`` above the contact row and at least ``x_min`` ahead of the pivot."""
    pts = [(x - PIVOT[0], PIVOT[1] - y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
    if y_max_up is not None:
        pts = [p for p in pts if p[1] <= y_max_up]
    if x_min is not None:
        pts = [p for p in pts if p[0] >= x_min]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), max(xs), min(ys), max(ys)


def boxes(grid: dict) -> dict:
    """Placeholder boxes in px from the pivot (x forward, y up), re-measured from the drawn frames: the spec table
    (section 3) clipped to the measured opaque extent of the frames it covers. Not approved mechanics."""
    cels = [f["cels"]["body"] for f in grid["frames"]]

    def clip(spec_x, spec_y, frames):
        ext = [_extent(cels[n]) for n in frames]
        x0, x1 = min(e[0] for e in ext), max(e[1] for e in ext)
        top = max(e[3] for e in ext)
        return {"x": [max(spec_x[0], x0), min(spec_x[1], x1)], "y": [spec_y[0], min(spec_y[1], top)]}

    hit = _extent(cels[10], y_max_up=14, x_min=24)
    return {
        "standing": {"x": list(_extent(cels[0])[:2]), "y": [0, _extent(cels[0])[3]]},
        "hurtbox_idle_walk": clip((-26, 24), (0, 58), range(0, 7)),
        "hurtbox_tell": clip((-22, 18), (0, 70), (7, 8)),
        "hurtbox_smash": clip((-24, 30), (0, 48), (9, 10)),
        "hurtbox_recover": clip((-24, 30), (0, 44), (11,)),
        "hurtbox_hurt": clip((-26, 8), (0, 70), (12,)),
        "hurtbox_down": clip((-40, 24), (0, 24), (13, 14, 15)),
        "attack_hitbox_impact_frame_only": {"x": [24, min(46, hit[1])], "y": [0, 14], "frame": 10},
    }


def manifest() -> dict:
    grid = frames_grid()
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": "key" if n in KEY_FRAMES else "inbetween"} for n in range(len(ticks))]
    return {
        "schema_version": 1,
        "id": "troglodyte_basher_h66_candidate_v0",
        "type": "character_animation",
        "source_path": SOURCE_ASE,
        "grid_source": "assets/source/characters/troglodyte/basher/trog_02.grid.json",
        "output_atlas": "generated/art/trog_02.png",
        "output_metadata": "generated/art/trog_02.json",
        "palette_id": "trog-family-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 troglodyte Basher; runtime role IP-neutral slow heavy slammer)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("TROG-02 Basher at a 66 px idle height: 7 keys (atlas frames 0, 3, 8, 10, 11, 12, 15) rasterized from pose "
                       "rules by scripts/author_trog02.py; the other nine slots are in-betweens drawn from the same part rules; "
                       "smash debris and dust are deferred to the VFX order; layer body only"),
            "generator": "Spec and key 0: Claude Opus. Keys 3/8/10/11/12/15, in-betweens and execution: Claude Sonnet. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "basher": {
            "status": "seven frozen keys plus nine drawn in-betweens, not yet reviewed by an independent art director; Bevy integration pending; 16 frames, 7 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/trog-02-spec.md",
            "opaque_height_idle": H,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _f in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/trog-02_keys.json",
            "attack_ticks": {"tell_rise": [0, 7], "tell_hold": [8, 35], "downswing": [36, 38], "impact_hitbox": [39, 46],
                             "recover_punish": [47, 66]},
            "boxes_px_from_pivot": boxes(grid),
            "frames": frames,
        },
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-02/key0-scale.png") -> Path:
    """Key 0 at 1x on lit Floor 1 next to Carl and the Pygmy (TROG-01 key 0), 640 x 360 unscaled, plus a 4x copy."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    pygmy = grid_images(load_grid_file(t1.GRID))[0]
    basher = grid_images(load_grid_file(GRID))[0]
    p_img, p_piv = flip(pygmy, tuple(t1.PIVOT))
    b_img, b_piv = flip(basher, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (p_img, p_piv, CARL_X + 70, 0), (b_img, b_piv, CARL_X + 160, 0)]
    scene = Floor().render(entries, 1)
    panel_w = basher.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, basher.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = basher.resize((panel_w, basher.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 0), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
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
