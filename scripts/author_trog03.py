#!/usr/bin/env python3
"""TROG-03 authoring record: the Troglodyte Virtuoso (variant of the TROG-01 Pygmy base), palette slots and hero key 0.

Usage:

    python3 scripts/author_trog03.py frames [--check]
    python3 scripts/author_trog03.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/virtuoso/trog_03.grid.json`` (pixel-grid-v1, layer ``body``).
Design stage: only key 0 (``idle``) is drawn; the other keys and the in-betweens are added to ``POSES``, ``KEY_FRAMES``,
``INBETWEEN`` and ``TAGS`` by the execution stage from ``docs/02-art/trog-03-spec.md`` sections 4, 7, 8 and 14.

The family construction (part canvas, outline-by-part-order, rim light, despeckle, the 15-colour core palette, the
thick-line helper) is imported from ``scripts/author_trog01.py``; the palette helper and the necklace stamp follow
``scripts/author_trog02.py``. The Virtuoso is redrawn at 66 px from ``docs/02-art/trog-01-spec.md`` section 2b; nothing
is scaled up from Pygmy or Basher pixels. Every pose is a rule (thick polylines, discs and polygons in design units:
x right from the pivot column, y up from the contact row) plus literal face stamps. No raster image-model output, no
tracing of concept pixels. ``--check`` compares instead of writing. Presentation-only art tooling; nothing here is wired
into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import author_trog01 as t1  # noqa: E402  (shared family construction)

ROOT = t1.ROOT
OUT_DIR = ROOT / "assets/source/characters/troglodyte/virtuoso"
GRID = OUT_DIR / "trog_03.grid.json"
CANVAS = (128, 88)
PIVOT = (44, 87)
H = 66                                   # idle opaque height, hair included (family ladder, trog-01 spec 2b)

# trog-family-v0 core (15, frozen) + the Virtuoso's three slots: the ragged purple robe, deep, mid and torch-lit.
# Gold bells reuse t and T; the tongue reuses q with s and o for its shade. 18 of 20.
PALETTE = t1.PALETTE + [
    ("u", "robe_deep", "#2f1c3aff", False),
    ("v", "robe_mid", "#583768ff", False),
    ("V", "robe_light", "#8a5c8cff", False),
]
RAMPS = dict(t1.RAMPS)
RAMPS.update({
    "robe": ("u", "u", "v", "V"),        # shawl and skirt: grimy purple, dusty lit edge
    "bell": ("K", "T", "T", "t"),        # gold bell: dark-gold body, lit upper-left rim
    "cord": ("c", "c", "C", "C"),        # bell cord
    "tongue": ("o", "s", "q", "q"),      # attack frames only (execution stage)
})
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}
L = t1.L


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
    """The family part canvas on the Virtuoso frame and ramps."""

    def __init__(self):
        super().__init__(CANVAS[0], CANVAS[1], PIVOT[1])
        self.px = PIVOT[0]

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1


# ------------------------------------------------------------------ face stamps (Virtuoso, redrawn at 66 px)
# 13 columns; top-left at (hx - 5, hy + 6) where (hx, hy) is the jaw-circle centre. Brow 1 row over a bulging eye
# (3 white, 1 pupil, 3 red: bloodshot and staring); the mouth is narrower than the Basher's (hx - 2 .. hx + 6) and opens
# taller. The Virtuoso's own note: the tongue tip always lolls over the lower lip, with a drool strand (canon: a very
# long pink tongue). The tongue's length is only shown in the attack frames.
FACES = {
    "leer": [".............",
             "....KKKKKK...",
             "....ewwwwK...",
             "....ewwwKK...",
             "......ee.....",
             "..........s..",
             "...t.t.t.tt..",
             "..oooooooooo.",
             "..ooooooqqqo.",
             "...t.t..qqq..",
             "........qqq..",
             ".........qq..",
             ".........qq..",
             "..........q..",
             "..........W..",
             "..........W..",
             "..........W.."],
    # gape (feint 8, lash 9, strike 10): the eye wide, the mouth a 5-row opening, fangs top and bottom, two drool
    # strands. ``gape_tell`` holds the bunched tongue (q mass) in the lower half of the mouth; ``gape`` shows only the
    # tongue root at the mouth front, where the ``tongue`` part continues it. W is a drool strand (drawn as w, always).
    "gape_tell": [".............",
                  "....KKKKKK...",
                  "....eewwwK...",
                  ".....eKwwK...",
                  ".............",
                  "..t.t.t.t.tt.",
                  "..ooooooooooo",
                  "..ooooooooooo",
                  "..oqqqqqoooo.",
                  "..oqqqqqqooo.",
                  "..t.t.t.t.t..",
                  "..W.......W..",
                  "..W.......W.."],
    "gape": [".............",
             "....KKKKKK...",
             "....eewwwK...",
             ".....eKwwK...",
             ".............",
             "..t.t.t.t.tt.",
             "..ooooooooooo",
             "..ooooooooooo",
             "..oooooooqqqq",
             "..oooooooqqqq",
             "..t.t.t.t.t..",
             "..W.......W..",
             "..W.......W.."],
    # slack (retract 11): lid over the top half of the eye, one e under it, the mouth open 2 rows, tongue root at the
    # front corner.
    "slack": [".............",
              "....KKKKKK...",
              "....KKKKKK...",
              "....ewwKK....",
              ".............",
              "..........s..",
              "..t.t.t.t....",
              "..oooooooooo.",
              "..oooooooooo.",
              "...t.t...qqq.",
              ".........qqq.",
              "..W..........",
              "..W.........."],
    # pain (hurt 12, fall beat 13): eye a K slit, no e, a 3-row open yelp with fangs, 3 q behind the lower fangs
    "pain": [".............",
             "....KKKKKK...",
             "....sKKKKK...",
             ".............",
             ".............",
             "..t.t.t.t.t..",
             "..ooooooooo..",
             "..ooooooooo..",
             "..ooqqqoooo..",
             "..t.t.t.t.t..",
             ".W.........W."],
    # dead (14, 15): a crossed eye, slack mouth, the tongue root at the mouth corner (drawn rotated: the head lies on
    # its back)
    "dead": [".............",
             "....K.K......",
             ".....K.......",
             "....K.K......",
             ".............",
             ".............",
             "..ooooooooo..",
             "..t.t.t.t.qq.",
             "..........qq.",
             "...........q."],
}
for _name, _rows in FACES.items():
    assert all(len(r) == 13 for r in _rows), _name

HEAD_R = (6.0, 5.6)        # skull 0.09 H and jaw (0.93 of it); constant in every pose
HEAD_OFF = 4.0             # skull centre this far behind the jaw centre: head width 4 + 6 + 5.6 = 15.6 px (0.236 H)


def head(hx, hy, tilt=-1.5):
    """Skull circle 4 px behind and ``-tilt`` px above the jaw circle centred on (hx, hy)."""
    return L([(hx - HEAD_OFF, hy - tilt), (hx, hy)], list(HEAD_R))


def jaw_b(hx, hy, drop=1.0):
    """Dropped lower jaw for the open-mouth faces (0..1): the Virtuoso's opens tall, not wide."""
    return L([(hx - 4, hy - 3.5), (hx + 3, hy - 3.5 - 4.0 * drop)], [3.0, 2.6 + 0.3 * drop])


# Hair: a matted crown mop about 0.06 H above the skull top (the long head, not a tall mop, carries the height) and
# seven rope dreads (radius 1.5) hanging down and back over the neck, the hump and the shawl; the longest reaches
# 0.6 H behind the jaw centre, to the middle of the back. A short front lock ends above the brow.
MOP = [(-10.5, 3), (-11, 6.5), (-9.5, 8.5), (-8.5, 10.5), (-7, 8.4), (-6, 11), (-4, 11.8), (-3, 9.0), (-1.5, 11.4),
       (0.5, 11.2), (2.5, 9.8),
       (1.5, 8), (-1.5, 7.2), (-5, 5.6), (-8, 3.5)]
ROPES = [[(-9, 8), (-14, 5), (-18, -2), (-21, -10), (-22, -18), (-22, -25)],   # longest: 0.6 H, to mid back
         [(-6, 10), (-12, 9), (-17, 5), (-21, -1), (-23, -5)],                  # upper rope streaming back
         [(-10, 4), (-12, -1), (-12, -6), (-11, -11)],                          # over the nape
         [(-3, 10.5), (-8, 11.2), (-12, 10.5)],
         [(-7, 4), (-8, 0), (-7.5, -4)],                                        # short, behind the ear
         [(1.5, 10), (4, 9), (5, 7.5)]]                                         # the short front lock
ROPE_R = 1.5
BEADS = {0: "T", 1: "C", 2: "T"}   # dull knots (the Basher gate: a light bead in a dark rope reads as a second eye)
EAR = [(-6, 4), (-16, 7.5), (-9, 1)]          # long, pointed, straight back below the crown, skin over the mop


def hair(hx, hy, sweep=0.0, sway=0.0):
    mop = [(hx + x, hy + y) for x, y in MOP]
    ropes = []
    for rope in ROPES:
        n = len(rope) - 1
        ropes.append([(hx + x - 4.0 * sweep * i / n - sway * i / n, hy + y + 3.0 * sweep * i / n + 0.4 * sway * i / n)
                      for i, (x, y) in enumerate(rope)])
    return {"mop": mop, "ropes": ropes, "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def bell(bx, by, s=1.0):
    """A hand bell hanging from (bx, by) (top of the crown): crown, flared skirt, about 7 x 7 px at s = 1."""
    return [(bx - 1.2 * s, by), (bx + 1.2 * s, by), (bx + 2.2 * s, by - 2.0 * s), (bx + 2.6 * s, by - 4.6 * s),
            (bx + 3.6 * s, by - 6.4 * s), (bx - 3.6 * s, by - 6.4 * s), (bx - 2.6 * s, by - 4.6 * s),
            (bx - 2.2 * s, by - 2.0 * s)]


def skirt(x0, y0, drop=16.0, half=6.5, lean=0.0):
    """Ragged robe wrap hanging down from the hips (gravity): short at the front, long at the back, with three deep
    torn cuts so the hem reads as strips, never as a box. ``lean`` trails the hem (motion)."""
    return [(x0 + half, y0 + 1), (x0 + half + 0.5 + lean, y0 - drop * 0.45), (x0 + half * 0.55 + lean, y0 - drop * 0.25),
            (x0 + half * 0.3 + lean, y0 - drop * 0.7), (x0 + lean, y0 - drop * 0.4),
            (x0 - half * 0.3 + lean, y0 - drop * 0.95), (x0 - half * 0.55 + lean, y0 - drop * 0.55),
            (x0 - half * 0.85 + lean, y0 - drop * 1.05), (x0 - half - 0.8 + lean, y0 - drop * 0.7), (x0 - half, y0 + 1)]


def drooping_hand(x, y, droop=1.0):
    """Hanging hand: a small palm at the wrist and three long 1 px claws (0.12 H, 8 px) curling down and forward."""
    return [L([(x, y), (x + 1.2, y - 0.8)], [1.6, 1.4]),
            L([(x + 1.0, y - 0.6), (x + 3.6, y - 2.4 * droop), (x + 5.2, y - 6.4 * droop)], [0.55, 0.55, 0.55]),
            L([(x + 1.6, y - 0.2), (x + 5.0, y - 1.6 * droop), (x + 7.4, y - 4.8 * droop)], [0.55, 0.55, 0.55]),
            L([(x + 0.2, y - 1.0), (x + 1.4, y - 3.6 * droop), (x + 2.2, y - 7.0 * droop)], [0.55, 0.55, 0.55])]


def build(pose: dict) -> list[str]:
    w, h = CANVAS
    c = Canvas()
    far = c.add("far"); cord = c.add("cord"); bell_id = c.add("bell"); skin = c.add("skin"); shawl = c.add("robe")
    neck = c.add("skin"); robe = c.add("robe"); head_id = c.add("skin"); hair_id = c.add("hair"); arm = c.add("skin")
    ear = c.add("skin")
    hd = pose["hair"]
    # back to front: far limbs and far hand, bell cord, torso and hump, shawl, near leg, robe skirt, back bell,
    # neck, head and jaw, mop and ropes, near arm, near hand claws (each its own part), ear; then texture, creases,
    # grime, face and stamps
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for fin in pose.get("far_fingers", []):
        c.thick(fin["pts"], fin["r"], far)
    for line in pose.get("cords", []):
        c.thick(line, [0.6] * len(line), cord)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for cx, cy, r in pose.get("masses", []):
        c.disc(cx, cy, r, r, skin)
    if "shawl" in pose:
        c.poly(pose["shawl"], shawl)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["skirt"], robe)
    for poly in pose.get("bells", []):                    # the back bell hangs at the near side of the back hip
        c.poly(poly, bell_id)
    c.thick(pose["neck"]["pts"], pose["neck"]["r"], neck)
    c.thick(pose["head"]["pts"], pose["head"]["r"], head_id)
    if "jaw" in pose:
        c.thick(pose["jaw"]["pts"], pose["jaw"]["r"], head_id)
    c.poly(hd["mop"], hair_id)
    for rope in hd["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], arm)
    for fin in pose.get("fingers", []):                   # palm and claws: each its own part, so K gaps separate them
        c.thick(fin["pts"], fin["r"], c.add("skin"))
    tongue_id = None
    if "tongue" in pose:                                  # the strike, retract and fallen frames
        tongue_id = c.add("tongue")
        c.thick(pose["tongue"]["pts"], pose["tongue"]["r"], tongue_id)
    for e in hd["ears"]:
        c.poly(e, ear)
    rows = c.render()
    for y in range(h):                                   # rope texture (family rule)
        for x in range(w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
            if c.part[y][x] in (shawl, robe) and rows[y][x] == "v" and (x + 2 * y) % 7 == 0:
                rows[y][x] = "u"                         # robe weave: sparse deep flecks keep the cloth grimy
    for i, key in BEADS.items():
        bx, by = c.P(*hd["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    for y in range(h):                                   # mottled rim: no clean lit run longer than 5
        run = 0
        for x in range(w + 1):
            if x < w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    for (x0, y0), (x1, y1) in pose.get("creases", []):   # rib strokes and joint creases: 1 px d on skin
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for k in range(n + 1):
            px, py = c.P(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n)
            if rows[int(py)][int(px)] in "mls":
                rows[int(py)][int(px)] = "d"
    for gx, gy in pose.get("grime", []):                  # 2 x 2 dirt smudges (deep skin) on lit/mid/shade skin
        px, py = c.P(gx, gy)
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            if rows[int(py) + dy][int(px) + dx] in "mls":
                rows[int(py) + dy][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):               # dirt-caked soles
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 5, hy + 6)
    jx, jy = (int(v) for v in c.P(hx, hy))
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if pose.get("face_rot"):                 # the head lies on its back: the stamp turns 90 degrees ccw
                    x, y = jx + (y - jy), jy - (x - jx)
                force = ch in "ewtTqW"
                ch = "w" if ch == "W" else ch
                if 0 <= x < w and 0 <= y < h and (rows[y][x] != "." or force):
                    rows[y][x] = ch
    if "tongue" in pose:                                 # wet glints: 1 px w on the top edge, at most one per 12 px
        cols = sorted({x for y in range(h) for x in range(w) if c.part[y][x] == tongue_id})
        for x in cols[7::12]:
            for y in range(h):
                if c.part[y][x] == tongue_id:
                    if rows[y][x] == "q":
                        rows[y][x] = "w"
                    break
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        if 0 <= int(py) < h and 0 <= int(px) < w:
            rows[int(py)][int(px)] = key
    t1.despeckle(rows)
    return ["".join(r) for r in rows]


def small_bell(x, y):
    """A 2 x 3 px gold bell stamp hanging from (x, y): lit top, dark-gold skirt, K mouth."""
    return [(x, y, "t"), (x - 1, y - 1, "t"), (x, y - 1, "T"), (x - 1, y - 2, "T"), (x, y - 2, "T")]


POSES: dict[str, dict] = {}

# KEY 0 idle: a lean, tall, less hunched stalker. Spine about 55 degrees from the hips (-9, 26) to the shoulder (2, 41);
# the head held 7.5 px above the hump on a visible neck, so the mouth has a clear line ahead; long arms hanging forward
# with the claws 0.14 H above the floor; a ragged purple shawl over the hump and a torn purple skirt; a big gold bell
# hanging on a cord off the back; two small bells on the skirt and one on the chest cord; the tongue tip lolling.
POSES["idle0"] = {
    "far_limbs": [L([(-10, 23), (-7, 13), (-14, 4.5), (-14, 2.2), (-10, 1.4), (-6, 1.3)], [2.6, 2.1, 1.5, 1.3, 1.3, 1.2]),
                  L([(0, 38), (1, 20), (18, 16)], [1.9, 1.6, 1.3])],               # far leg; far arm
    "far_fingers": drooping_hand(18, 16, 0.8),
    "cords": [[(-10, 33), (-14, 26), (-17, 20)]],
    "bells": [bell(-17, 20, 1.0)],
    "torso": L([(-8, 24), (-3, 31.5), (2, 38)], [4.4, 4.8, 4.6]),
    "masses": [(0, 39, 5.0)],                                                          # shoulder hump
    "shawl": [(-6, 44), (-1, 44.5), (4, 42), (6, 37), (4, 33.5), (1, 34.5), (-2, 32), (-5, 30.5), (-7, 27),
              (-9.5, 29), (-11, 26), (-12.5, 30), (-11.5, 35), (-9, 40)],
    "near_legs": [L([(-7, 23), (2, 13.5), (-3, 4.5), (-3, 2.2), (1, 1.4), (6, 1.3)], [2.8, 2.2, 1.6, 1.3, 1.3, 1.2])],
    "skirt": skirt(-7, 23, 16.0, 6.5),
    "neck": L([(3, 40), (6.5, 47)], [2.6, 2.3]),
    "head": head(8, 51.5), "hair": hair(8, 51.5),
    "near_arms": [L([(3, 37), (6, 19), (24, 15)], [2.2, 1.8, 1.4])],
    "fingers": drooping_hand(24, 15, 0.8),
    "creases": [((-3, 28), (-1, 30)), ((-2, 25), (0, 27)), ((4, 20), (7, 19)), ((1, 13), (3, 12))],
    "grime": [(10, 18), (18, 16), (0, 15), (-5, 7), (-2, 33), (1, 30), (-1, 4), (3, 35), (6, 44), (-9, 20)],
    "stamps": small_bell(-5, 11.5) + small_bell(-9, 7.5) + small_bell(5.5, 42),
    "face": "leer", "face_at": (8, 51.5),
}

# ------------------------------------------------------------------ execution stage: keys 3, 8, 10, 11, 12, 15 and
# the in-betweens (spec sections 7, 8 and 14). Everything below reuses the key-0 parts through ``mk``.
I0 = POSES["idle0"]
NEAR_ARM_R = [2.2, 1.8, 1.4]
FAR_ARM_R = [1.9, 1.6, 1.3]
NEAR_LEG_R = [2.8, 2.2, 1.6, 1.3, 1.3, 1.2]
FAR_LEG_R = [2.6, 2.1, 1.5, 1.3, 1.3, 1.2]


def shifted(pts, dx=0.0, dy=0.0):
    return [(x + dx, y + dy) for x, y in pts]


def leg(hip, knee, ankle, toe, radii, heel_y=None):
    """Hip, knee, ankle, heel, toe-mid, toe-tip: the heel and the toes sit at the ankle's lift above the floor."""
    lift = max(0.0, ankle[1] - 4.0)
    return L([hip, knee, ankle, (ankle[0], (heel_y if heel_y is not None else 2.2) + lift),
              ((ankle[0] + toe[0]) / 2, 1.4 + lift), (toe[0], 1.3 + lift)], radii)


def foot(cx, lift=0.0):
    """A walk foot centred on ``cx`` (contact-run centre): the ankle and the toe tip; the heel is on the floor too."""
    return (cx - 3.5, 4.0 + lift), (cx + 3.5, 1.3 + lift)


def bell_lying(bx, by):
    """The back bell fallen on its side, the crown at (bx, by) and the mouth toward the player (+x)."""
    return [(bx - y, by + x) for x, y in bell(0, 0, 1.0)]


def mk(*, torso, hump, neck, jaw, face, near_arm, near_hand, far_arm, far_hand, near_leg, far_leg, sweep=0.0, sway=0.0,
       jawdrop=None, tilt=-1.5, lean=0.0, skirt_drop=16.0, bell_dx=0.0, tongue=None, hair_=None, shawl=None, cord_at=None,
       bell_poly=None, face_rot=False, head_pose=None, stamps=None):
    """One pose from joints. The radii, the robe, the bells, the grime and the creases ride with the joints."""
    dx, dy = hump[0], hump[1] - 39
    shawl = shawl or shifted(I0["shawl"], dx, dy)
    sx, sy = cord_at or (-10 + dx, 33 + dy)
    bx, by = -17 + dx + bell_dx, 20 + dy
    hx, hy = jaw
    hip = torso[0]
    nel, nwr = near_arm[1], near_arm[2]
    fel, fwr = far_arm[1], far_arm[2]
    nk = near_leg["pts"][1]
    pose = {
        "far_limbs": [far_leg, L(far_arm, FAR_ARM_R)],
        "far_fingers": drooping_hand(*far_hand),
        "cords": [[(sx, sy), ((sx + bx) / 2 - 0.5, (sy + by) / 2), (bx, by)]],
        "bells": [bell_poly or bell(bx, by, 1.0)],
        "torso": L(torso, [4.4, 4.8, 4.6]),
        "masses": [(hump[0], hump[1], 5.0)],
        "shawl": shawl,
        "near_legs": [near_leg],
        "skirt": skirt(hip[0] + 1, hip[1] - 1, skirt_drop, 6.5, lean),
        "neck": L(neck, [2.6, 2.3]),
        "head": head_pose or head(hx, hy, tilt),
        "hair": hair_ or hair(hx, hy, sweep, sway),
        "near_arms": [L(near_arm, NEAR_ARM_R)],
        "fingers": drooping_hand(*near_hand),
        "creases": [((torso[1][0], torso[1][1] - 3.5), (torso[1][0] + 2, torso[1][1] - 1.5)),
                    ((torso[1][0] + 1, torso[1][1] - 6.5), (torso[1][0] + 3, torso[1][1] - 4.5)),
                    ((nel[0] - 2, nel[1] + 1), (nel[0] + 1, nel[1])),
                    ((nk[0] - 1, nk[1] - 0.5), (nk[0] + 1, nk[1] - 1.5))],
        "grime": [(torso[1][0] + 2, torso[1][1] + 1.5), (torso[2][0] - 1, torso[2][1] - 1), (neck[1][0] - 0.5, neck[1][1] - 3),
                  ((nel[0] + nwr[0]) / 2, (nel[1] + nwr[1]) / 2), (nwr[0] - 1, nwr[1]),
                  ((fel[0] + fwr[0]) / 2, (fel[1] + fwr[1]) / 2), nk, far_leg["pts"][1],
                  (near_leg["pts"][2][0] - 1, near_leg["pts"][2][1] - 1), (hip[0] - 1, hip[1] - 3)],
        "stamps": stamps if stamps is not None else (small_bell(hip[0] + 3, hip[1] - 12.5) + small_bell(hip[0] - 1, hip[1] - 16.5)
                                                      + small_bell(hump[0] + 5.5, hump[1] + 3)),
        "face": face, "face_at": jaw,
    }
    if jawdrop is not None:
        pose["jaw"] = jaw_b(hx, hy, jawdrop)
    if tongue:
        pose["tongue"] = tongue
    if face_rot:
        pose["face_rot"] = True
    return pose


# ---- KEY 3: walk contact A (spec 7). Body 2 low and 2 forward; near foot just planted ahead (run centre +8), far foot at
# the end of its stance (-4); arms counter-swing. Frames 4, 5 and 6 derive from the same body by ``walk_frame`` (spec 8
# contact plan: every planted foot moves back 6 px per frame, loop seam included; 2 px bob on the passing frames).
K3 = dict(torso=[(-7, 22), (-1, 29.5), (4, 36)], hump=(2, 37), neck=[(5, 38), (8.5, 44.5)], jaw=(10, 49.0))
# frame: body dy, near foot (cx, lift), far foot (cx, lift), near wrist x, far wrist x
WALK_PLAN = {3: (0, (8, 0), (-4, 0), 20, 22), 4: (2, (2, 0), (2, 3), 23, 19), 5: (0, (-4, 0), (8, 0), 26, 16),
             6: (2, (2, 3), (2, 0), 23, 19)}


def walk_frame(n: int) -> dict:
    dy, (ncx, nl), (fcx, fl), nw, fw = WALK_PLAN[n]
    up = lambda pts: [(x, y + dy) for x, y in pts]   # noqa: E731
    n_ankle, n_toe = foot(ncx, nl)
    f_ankle, f_toe = foot(fcx, fl)
    n_knee = (6 + 0.5 * (ncx - 8), 12 + 0.5 * dy + 0.5 * nl)       # knees take half the foot move
    f_knee = (-9 + 0.5 * (fcx + 4), 11 + 0.5 * dy + 0.5 * fl)
    n_el = (5 + 0.5 * (nw - 20), 18 + 0.5 * dy)
    f_el = (5 + 0.5 * (fw - 22), 19 + 0.5 * dy)
    return mk(torso=up(K3["torso"]), hump=(K3["hump"][0], K3["hump"][1] + dy), neck=up(K3["neck"]),
              jaw=(K3["jaw"][0], K3["jaw"][1] + dy), face="leer", sweep=0.4, lean=-1.5,
              near_arm=[(5, 35 + dy), n_el, (nw, 14)], near_hand=(nw, 14, 0.8),
              far_arm=[(2, 36 + dy), f_el, (fw, 16)], far_hand=(fw, 16, 0.8),
              near_leg=leg((-5, 21 + dy), n_knee, n_ankle, n_toe, NEAR_LEG_R, heel_y=1.5),
              far_leg=leg((-9, 21 + dy), f_knee, f_ankle, f_toe, FAR_LEG_R, heel_y=1.5))


for _n in (3, 4, 5, 6):
    POSES[f"walk{_n}"] = walk_frame(_n)

# ---- KEY 8: the feint. Reared back on the back leg, head pulled up and back, the mouth gaping with the tongue bunched
# inside, the near claws raised (curling up), the far arm drawn back and down.
POSES["feint8"] = mk(
    torso=[(-8, 25), (-6, 34), (-2, 43)], hump=(-4, 44), neck=[(-1, 45), (2, 52)], jaw=(4, 57), face="gape_tell", jawdrop=1.0, bell_dx=0.0,
    near_arm=[(-1, 42), (8, 40), (15, 46)], near_hand=(15, 46, -0.6),
    far_arm=[(-3, 42), (-9, 30), (-5, 21)], far_hand=(-5, 21, 0.8),
    near_leg=leg((-5, 24), (3, 14), (3, 4), (10, 1.3), NEAR_LEG_R),
    far_leg=leg((-9, 24), (-14, 13), (-19, 4), (-14, 1.3), FAR_LEG_R))

# ---- KEY 10: the strike. Lunging, the head thrust ahead, the tongue shot out straight and level to its full reach.
POSES["strike10"] = mk(
    torso=[(-6, 22), (2, 29), (10, 34)], hump=(8, 36), neck=[(11, 37), (15, 42)], jaw=(18, 46), face="gape", jawdrop=1.0,
    sweep=0.8, lean=-2.5, bell_dx=-3.0,
    near_arm=[(11, 33), (16, 21), (27, 19)], near_hand=(27, 19, 0.8),
    far_arm=[(7, 34), (0, 23), (-7, 18)], far_hand=(-7, 18, 0.8),
    near_leg=leg((-4, 21), (8, 13), (8, 4), (15, 1.3), NEAR_LEG_R),
    far_leg=leg((-8, 21), (-15, 11), (-24, 4), (-18, 1.3), FAR_LEG_R, heel_y=2.5),
    tongue=L([(23, 43), (50, 43.5), (70, 42), (74, 40), (73, 37.5)], [2.0, 1.8, 1.5, 2.2, 1.6]))

# ---- KEY 11: the retract. Slumped, head low, the tongue hanging slack from the mouth to the floor.
POSES["retract11"] = mk(
    torso=[(-8, 22), (-2, 29), (5, 35)], hump=(3, 36), neck=[(6, 37), (10, 41)], jaw=(12, 44), face="slack",
    near_arm=[(6, 34), (9, 18), (17, 11)], near_hand=(17, 11, 1.0),
    far_arm=[(3, 35), (5, 18), (12, 12)], far_hand=(12, 12, 1.0),
    near_leg=leg((-6, 21), (2, 12.5), (-3, 4), (6, 1.3), NEAR_LEG_R),
    far_leg=leg((-9, 21), (-7, 12), (-14, 4.5), (-6, 1.3), FAR_LEG_R), lean=0.5,
    tongue=L([(17, 41), (24, 31), (32, 16), (38, 5), (44, 1.6), (52, 1.6)], [1.8, 1.6, 1.4, 1.3, 1.3, 1.2]))

# ---- KEY 12: hurt. Thrown back, torso tilted back, head above the hump and back, the tongue snapped in, arms flung.
POSES["hurt12"] = mk(
    torso=[(-6, 26), (-8, 37), (-10, 46)], hump=(-10, 47), neck=[(-8, 49), (-6, 54)], jaw=(-6, 58), face="pain", jawdrop=0.6,
    tilt=0.0, sweep=-0.6, lean=2.0, bell_dx=-3.0,
    near_arm=[(-9, 45), (-18, 50), (-26, 58)], near_hand=(-26, 58, -1.0),
    far_arm=[(-10, 46), (-3, 36), (6, 26)], far_hand=(6, 26, 0.8),
    near_leg=leg((-6, 25), (4, 14), (2, 4), (9, 1.3), NEAR_LEG_R),
    far_leg=leg((-9, 25), (-12, 14), (-14, 4), (-10, 1.3), FAR_LEG_R))


# ---- the fallen frames: the head lies behind the shoulders, the crown pointing back along the floor.
def hair_down(hx, hy):
    """The mop and ear turned 90 degrees ccw about the head (crown back) and flattened onto the floor; ropes spill back."""
    turn = lambda pts: [(hx - y, max(1.5, hy + x)) for x, y in pts]   # noqa: E731
    ropes = [[(hx - 5, hy + 2), (hx - 11, hy + 1), (hx - 16, hy - 1), (hx - 19, hy - 3)],
             [(hx - 4, hy - 1), (hx - 10, hy - 2), (hx - 15, hy - 4), (hx - 17, hy - 6)],
             [(hx - 5, hy + 5), (hx - 11, hy + 5.5), (hx - 15, hy + 4)]]
    ropes = [[(x, max(1.5, y)) for x, y in r] for r in ropes]
    return {"mop": turn(MOP), "ropes": ropes, "ears": [turn(EAR)]}


def head_down(hx, hy):
    return L([(hx - 4.5, hy + 0.5), (hx, hy)], list(HEAD_R))


def down_pose(r: float, tongue_pts, tongue_r) -> dict:
    """Key 15 layout; ``r`` lifts the head, chest and knees (key 14 is the 1 px bounce)."""
    return mk(
        torso=[(6, 6), (-4, 7 + r), (-14, 7 + r)], hump=(-12, 9.5 + r), neck=[(-17, 8 + r), (-22, 8 + r)], jaw=(-26, 8 + r),
        face="dead", face_rot=True, tilt=0.0, head_pose=head_down(-26, 8 + r), hair_=hair_down(-26, 8 + r),
        near_arm=[(-13, 10 + r), (-14, 16 + r), (-10, 21 + r)], near_hand=(-10, 21 + r, 0.5),
        far_arm=[(-10, 8), (0, 5), (8, 4)], far_hand=(8, 4.5, 0.2),
        near_leg=L([(6, 6), (10, 15 + r), (17, 8), (17, 3), (20.5, 1.4), (24, 1.3)], NEAR_LEG_R),
        far_leg=L([(4, 5), (14, 13 + r), (19, 6), (19, 3), (22, 1.4), (25, 1.3)], FAR_LEG_R),
        shawl=[(-20, 3), (-18, 11 + r), (-10, 14 + r), (-3, 11 + r), (-1, 6), (-6, 2.5), (-12, 3)],
        cord_at=(-12, 5), bell_poly=bell_lying(-3.0, 4.0), skirt_drop=3.5, lean=1.0, tongue=L(tongue_pts, tongue_r),
        stamps=small_bell(3, 5.5 + r) + small_bell(8, 4.5))


POSES["down15"] = down_pose(0, [(-24, 13), (-21, 6), (-15, 1.8), (-8, 1.6)], [1.8, 1.6, 1.3, 1.2])

# ---- in-betweens
# 1: idle breath: torso end, hump, shawl, neck and head 1 px lower; ropes sway 1; back bell and cord end 1 px back
POSES["idle1"] = I0 | {
    "torso": L([(-8, 24), (-3, 31), (2, 37)], I0["torso"]["r"]),
    "masses": [(0, 38, 5.0)],
    "shawl": shifted(I0["shawl"], 0, -1),
    "neck": L([(3, 39), (6.5, 46)], I0["neck"]["r"]),
    "head": head(8, 50.5), "hair": hair(8, 50.5, sway=1.0), "face_at": (8, 50.5),
    "near_arms": [L([(3, 36), (6, 19), (24, 15)], I0["near_arms"][0]["r"])],
    "far_limbs": [I0["far_limbs"][0], L([(0, 37), (1, 20), (18, 16)], I0["far_limbs"][1]["r"])],
    "cords": [[(-10, 32), (-14.5, 26), (-18, 20)]],
    "bells": [bell(-18, 20, 1.0)],
    "stamps": I0["stamps"][:-5] + small_bell(5.5, 41),
}
# 2: the key posture with the ropes swayed the other way and the back bell 1 px forward
POSES["idle2"] = I0 | {"hair": hair(8, 51.5, sway=-1.0), "cords": [[(-10, 33), (-13.5, 26), (-16, 20)]],
                       "bells": [bell(-16, 20, 1.0)]}
# 7: feint onset, halfway from 0 to 8 (face stays the leer)
POSES["feint7"] = t1.blend(I0, POSES["feint8"], 0.5) | {"face": "leer"}
POSES["feint7"].pop("jaw", None)
# 9: lash: body halfway from 8 to 10, the tongue half out in a whipping S curve
POSES["lash9"] = t1.blend(POSES["feint8"], POSES["strike10"], 0.5) | {
    "face": "gape", "tongue": L([(16, 49), (26, 53), (36, 47), (44, 50)], [2.0, 1.8, 1.6, 2.0])}
# 13: fall beat, about 50 degrees back, toes on the floor, near arm flung back, far arm forward and down
POSES["down13"] = mk(
    torso=[(0, 14), (-6, 22), (-12, 30)], hump=(-13, 31), neck=[(-12.5, 32), (-13.5, 34.5)], jaw=(-14, 36), face="pain",
    jawdrop=0.6, tilt=0.0, sweep=-0.4, lean=2.0, bell_dx=-2.0,
    near_arm=[(-12, 29), (-21, 32), (-30, 34)], near_hand=(-30, 34, -1.0),
    far_arm=[(-10, 30), (-2, 20), (8, 8)], far_hand=(8, 8, 0.5),
    near_leg=leg((0, 13), (8, 10), (10, 4), (14, 1.3), NEAR_LEG_R),
    far_leg=leg((-2, 12), (5, 8), (8, 4), (12, 1.3), FAR_LEG_R), skirt_drop=10.0)
# 14: floor impact: the key-15 layout, the head and chest 1 px up, the tongue not yet fully spilled
POSES["down14"] = down_pose(1, [(-24, 14), (-21, 7), (-17, 2.5), (-12, 1.8)], [1.8, 1.6, 1.3, 1.2])

KEY_FRAMES = {0: "idle0", 3: "walk3", 8: "feint8", 10: "strike10", 11: "retract11", 12: "hurt12", 15: "down15"}
INBETWEEN = {1: "idle1", 2: "idle2", 4: "walk4", 5: "walk5", 6: "walk6", 7: "feint7", 9: "lash9", 13: "down13", 14: "down14"}
TAGS = [("idle", [(0, 12), (1, 10), (2, 12)]),
        ("walk", [(3, 8), (4, 8), (5, 8), (6, 8)]),
        ("attack_tell", [(7, 6), (8, 24)]),
        ("attack_tongue", [(9, 2), (10, 6)]),
        ("attack_recover", [(11, 18)]),
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


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-03/key0-scale.png") -> Path:
    """Key 0 at 1x on the torch-lit brick bay of Floor 1 (world x 800 to 960) next to Carl, the Pygmy (TROG-01) and the
    Basher (TROG-02), 640 x 360 unscaled, plus a 4x copy of key 0 on the right."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    entries = [(carl, carl_piv, 806, 0)]
    others = [(t1.GRID, t1.PIVOT, 846)]
    basher = ROOT / "assets/source/characters/troglodyte/basher/trog_02.grid.json"
    if basher.is_file():
        others.append((basher, (48, 87), 892))
    for path, piv, x in others:
        img, p = flip(grid_images(load_grid_file(path))[0], tuple(piv))
        entries.append((img, p, x, 0))
    virt = grid_images(load_grid_file(GRID))[0]
    v_img, v_piv = flip(virt, tuple(PIVOT))
    entries.append((v_img, v_piv, 946, 0))
    floor = Floor()
    floor.view = [480, 20, 640, 360]          # the lit brick bay, world x 800 to 960, inside the view
    scene = floor.render(entries, 1)
    panel_w = virt.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, virt.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = virt.resize((panel_w, virt.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 0), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


MANIFEST = ROOT / "assets/manifests/trog-03.json"
SOURCE_ASE = "assets/source/characters/troglodyte/virtuoso/trog_03.ase"
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_tongue": "once", "attack_recover": "once",
            "hurt": "once", "down": "hold"}


def _extent(rows: list[str], x_min: int | None = None) -> tuple[int, int, int, int]:
    """Opaque extent of one frame as (x0, x1, y0, y1) in px from the pivot (x forward, y up), optionally only the pixels at
    least ``x_min`` ahead of the pivot."""
    pts = [(x - PIVOT[0], PIVOT[1] - y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
    if x_min is not None:
        pts = [p for p in pts if p[0] >= x_min]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), max(xs), min(ys), max(ys)


def boxes(grid: dict) -> dict:
    """Placeholder boxes in px from the pivot (x forward, y up): the spec table (section 3) clipped to the measured opaque
    extent of the frames it covers. Not approved mechanics."""
    cels = [f["cels"]["body"] for f in grid["frames"]]

    def clip(spec_x, spec_y, frames):
        ext = [_extent(cels[n]) for n in frames]
        x0, x1 = min(e[0] for e in ext), max(e[1] for e in ext)
        top = max(e[3] for e in ext)
        return {"x": [max(spec_x[0], x0), min(spec_x[1], x1)], "y": [spec_y[0], min(spec_y[1], top)]}

    tip = _extent(cels[10], x_min=56)
    return {
        "standing": {"x": list(_extent(cels[0])[:2]), "y": [0, _extent(cels[0])[3]]},
        "hurtbox_idle_walk": clip((-14, 16), (0, 60), range(0, 7)),
        "hurtbox_feint": clip((-12, 14), (0, 68), (7, 8)),
        "hurtbox_lash_strike": clip((-14, 26), (0, 56), (9, 10)),
        "hurtbox_retract": clip((-14, 20), (0, 52), (11,)),
        "hurtbox_hurt": clip((-18, 6), (0, 70), (12,)),
        "hurtbox_down": clip((-40, 22), (0, 22), (13, 14, 15)),
        "tongue_hitbox_strike_frame_only": {"x": [56, min(78, tip[1])], "y": [max(34, tip[2]), min(48, tip[3])], "frame": 10},
    }


def manifest() -> dict:
    grid = frames_grid()
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": "key" if n in KEY_FRAMES else "inbetween"} for n in range(len(ticks))]
    return {
        "schema_version": 1,
        "id": "troglodyte_virtuoso_h66_candidate_v0",
        "type": "character_animation",
        "source_path": SOURCE_ASE,
        "grid_source": "assets/source/characters/troglodyte/virtuoso/trog_03.grid.json",
        "output_atlas": "generated/art/trog_03.png",
        "output_metadata": "generated/art/trog_03.json",
        "palette_id": "trog-family-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 troglodyte Virtuoso; runtime role IP-neutral fast long-reach tongue striker)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("TROG-03 Virtuoso at a 66 px idle height: 7 keys (atlas frames 0, 3, 8, 10, 11, 12, 15) rasterized from pose "
                       "rules by scripts/author_trog03.py; the other nine slots are in-betweens drawn from the same part rules; "
                       "tongue slime, spit and whip trail are deferred to the VFX order; layer body only"),
            "generator": "Spec and key 0: Claude Opus. Keys 3/8/10/11/12/15, in-betweens and execution: Claude Sonnet. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "virtuoso": {
            "status": "seven frozen keys plus nine drawn in-betweens, not yet reviewed by an independent art director; Bevy integration pending; 16 frames, 7 of them keys; not wired into any Bevy binary",
            "spec": "docs/02-art/trog-03-spec.md",
            "opaque_height_idle": H,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [name for name, _f in TAGS],
            "airborne_tags": [],
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/trog-03_keys.json",
            "attack_ticks": {"feint_onset": [0, 5], "feint_hold": [6, 29], "lash": [30, 31], "strike_hitbox": [32, 37],
                             "retract_punish": [38, 55]},
            "boxes_px_from_pivot": boxes(grid),
            "frames": frames,
        },
    }


def _strip(order: list[int], out: Path, sc: int, gif: Path | None = None, gif_ticks: list[int] | None = None) -> list[Path]:
    """Frames side by side at ``sc`` x on a fixed ground, a pivot tick under each frame; optional looping GIF."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import grid_images, load_grid_file

    imgs = grid_images(load_grid_file(GRID))
    w, h = CANVAS
    strip = Image.new("RGBA", (w * sc * len(order), (h + 4) * sc), (42, 40, 54, 255))
    draw = ImageDraw.Draw(strip)
    for i, n in enumerate(order):
        big = imgs[n].resize((w * sc, h * sc), Image.NEAREST)
        strip.paste(big, (i * w * sc, 0), big)
        draw.rectangle([i * w * sc, h * sc, (i + 1) * w * sc - 1, (h + 4) * sc], fill=(92, 70, 58, 255))
        px = i * w * sc + PIVOT[0] * sc
        draw.rectangle([px, h * sc, px + sc - 1, (h + 4) * sc], fill=(230, 220, 190, 255))
    out.parent.mkdir(parents=True, exist_ok=True)
    strip.save(out)
    paths = [out]
    if gif:
        frames = []
        for n in order[:len(gif_ticks)]:
            f = Image.new("RGBA", (w * sc, h * sc), (42, 40, 54, 255))
            big = imgs[n].resize((w * sc, h * sc), Image.NEAREST)
            f.paste(big, (0, 0), big)
            frames.append(f.convert("P", palette=Image.ADAPTIVE))
        frames[0].save(gif, save_all=True, append_images=frames[1:], loop=0,
                       duration=[round(t * 1000 / TICK_HZ) for t in gif_ticks])
        paths.append(gif)
    return paths


def write_walk_strip(out_dir: Path = ROOT / "docs/evidence/trog-03") -> list[Path]:
    """Walk contact check: frames 3, 4, 5, 6 and 3 again (the loop seam) at 3x, plus the looping GIF at 8 ticks a frame."""
    return _strip([3, 4, 5, 6, 3], out_dir / "walk_strip_x3.png", 3, out_dir / "gif_walk_x3.gif", [8, 8, 8, 8])


def write_attack_strip(out_dir: Path = ROOT / "docs/evidence/trog-03") -> list[Path]:
    """Tell-strike-retract: 7 (onset), 8 (feint), 9 (lash), 10 (strike), 11 (retract) at 2x, plus the GIF at tick timing."""
    return _strip([7, 8, 9, 10, 11], out_dir / "tell_strike_strip_x2.png", 2, out_dir / "gif_attack_x2.gif", [6, 24, 2, 6, 18])


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale", "walk-strip", "attack-strip"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command in ("walk-strip", "attack-strip"):
        for path in (write_walk_strip() if args.command == "walk-strip" else write_attack_strip()):
            print(f"wrote {path.relative_to(ROOT)}")
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
