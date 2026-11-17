#!/usr/bin/env python3
"""Authoring record for GOB-01: the full 27-frame goblin set (variants A and B), phase 3.

Writes ``assets/source/characters/goblin/goblin_a_h36.grid.json`` and ``goblin_b_h36.grid.json``
(pixel-grid-v1, 27 frames each) from the frozen Opus keys in
``tests/fixtures/goblin_gob01_keys.json``. ``scripts/export_assets.sh`` turns each grid into the
``.ase`` and the atlas. The in-between list and pose notes are section 9 of
``docs/02-art/gob-01-goblin-spec.md``.

How the frames are made (no resampling, no rotation, no image model):

* **Frozen keys.** The 11 Opus keys (atlas frames 0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26) are copied
  from the fixture without any change, in both variants.
* **In-betweens (16 frames per variant)** are assembled from key pixels: a part (head, upper body,
  leg, foot, weapon, ...) is cut out of a key with a rectangle or a row-wise rule and pasted back
  translated; a few hand-placed pixels and ASCII stamps close the seams. Rows are only ever
  translated whole (the row-wise leg "shear" moves each row of a leg by an integer).
* **Layers.** A in-betweens are drawn flat and split into ``body`` and ``weapon`` with the same
  rule the keys were saved with (``author_goblin_gob01.split_layers``).
* **Variant B** reuses A's ``body`` and ``weapon`` cels unchanged and draws only the ``accessory``
  layer: the key accessory overlay (bandana, ear notch, missing tooth, vest patch) split into a
  head part and a vest-patch part, each translated with the part of the body it sits on.

The committed grids are the source of truth. Run ``python3 scripts/author_goblin_frames.py`` to
rewrite them (deterministic) or ``--check`` to verify that they equal the build output.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import author_goblin_gob01 as ag  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "tests/fixtures/goblin_gob01_keys.json"
GOBLIN_DIR = ROOT / "assets/source/characters/goblin"
OUT = {"A": GOBLIN_DIR / "goblin_a_h36.grid.json", "B": GOBLIN_DIR / "goblin_b_h36.grid.json"}
W, H = ag.CANVAS
TICK_HZ = 60
HOLD_MS = 100          # hold frames (airborne / lying): preview duration only, the real hold is state-driven

KEY_IDS = (0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26)

# (tag, [(atlas frame, ticks or None for a state-driven hold)]) -- the tag table of spec section 5
TAGS = [
    ("idle", [(0, 12), (1, 8), (2, 12), (3, 8)]),
    ("walk", [(4, 5), (5, 5), (6, 5), (7, 5), (8, 5), (9, 5)]),
    ("attack_tell", [(10, 6), (11, 18)]),
    ("attack_swing", [(12, 2), (13, 4)]),
    ("attack_recover", [(14, 12), (15, 8)]),
    ("hurt", [(16, 4), (17, 8)]),
    ("knockback", [(18, 4), (19, None)]),
    ("down", [(20, 4), (21, 8), (22, None)]),
    ("getup", [(23, 8), (24, 10)]),
    ("corpse", [(25, None)]),
    ("knockback_contact", [(26, 3)]),
]
PLAYBACK = {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once",
            "attack_recover": "once", "hurt": "once", "knockback": "hold", "down": "hold",
            "getup": "once", "corpse": "hold", "knockback_contact": "once"}
GROUNDED_TAGS = ["idle", "walk", "attack_tell", "attack_swing", "attack_recover", "hurt", "down",
                 "getup", "corpse"]
AIRBORNE_TAGS = ["knockback", "knockback_contact"]

Pix = dict


# ------------------------------------------------------------------ pixel helpers

def parse(text: str, ox: int = 0, oy: int = 0) -> Pix:
    rows = text.strip("\n").split("\n")
    return {(ox + x, oy + y): c for y, r in enumerate(rows) for x, c in enumerate(r) if c not in ". "}


def shift(p: Pix, dx: int, dy: int) -> Pix:
    return {(x + dx, y + dy): c for (x, y), c in p.items()}


def box(x0: int, y0: int, x1: int, y1: int):
    return lambda x, y: x0 <= x <= x1 and y0 <= y <= y1


def cut(p: Pix, pred) -> tuple[Pix, Pix]:
    """Split ``p`` into (pixels where pred(x, y), the rest)."""
    a = {k: c for k, c in p.items() if pred(*k)}
    return a, {k: c for k, c in p.items() if not pred(*k)}


def over(*layers: Pix) -> Pix:
    out: Pix = {}
    for layer in layers:
        out.update(layer)
    return out


def move(p: Pix, pred, dx: int, dy: int) -> Pix:
    """Translate the pixels selected by ``pred``; the moved part is drawn over the rest."""
    part, rest = cut(p, pred)
    return over(rest, shift(part, dx, dy))


def shear(p: Pix, pred, dx_of_row) -> Pix:
    """Row-wise translation of the selected pixels: every row of the part moves by ``dx_of_row(y)``."""
    part, rest = cut(p, pred)
    return over(rest, {(x + dx_of_row(y), y): c for (x, y), c in part.items()})


def dup_row(p: Pix, y: int, x0: int, x1: int, dy: int = 0) -> Pix:
    """Copy the pixels of row ``y`` in columns x0..x1 to row ``y + dy`` (drawn under what is there)."""
    row = {(x, y + dy): c for (x, yy), c in p.items() if yy == y and x0 <= x <= x1}
    return over(row, p)


def put(p: Pix, text: str, ox: int, oy: int) -> Pix:
    return over(p, parse(text, ox, oy))


def erase(p: Pix, pred) -> Pix:
    return {k: c for k, c in p.items() if not pred(*k)}


def rows_of(p: Pix) -> list[str]:
    grid = [["."] * W for _ in range(H)]
    for (x, y), c in p.items():
        if 0 <= x < W and 0 <= y < H:
            grid[y][x] = c
    return ["".join(r) for r in grid]


FIX = json.loads(FIXTURE.read_text(encoding="utf-8"))


def key_cels(a: int) -> dict[str, list[str]]:
    return FIX["keys"][str(a)]


def key_pix(a: int) -> Pix:
    c = key_cels(a)
    return over(parse("\n".join(c["body"])), parse("\n".join(c["weapon"])))


# ------------------------------------------------------------------ part masks (frame 0 / 4 / 7 geometry)

def head_mask(dy: int = 0):
    """The head of the idle-family keys (0, 4, 7): ear, skull, face and jaw. ``dy`` is the key's body offset."""
    def pred(x: int, y: int) -> bool:
        y -= dy
        return ((y <= 29 and x <= 45) or (30 <= y <= 31 and 28 <= x <= 45)
                or (y == 32 and 29 <= x <= 40) or (y == 33 and 30 <= x <= 40))
    return pred


LEGS_Y = 47


# ------------------------------------------------------------------ idle family (frames 1, 2, 3)

def f1() -> Pix:
    """Idle breath in: everything above row 46 up 1 px, the shorts hem stretched 1 px."""
    p = key_pix(0)
    top, rest = cut(p, lambda x, y: y <= 45)
    row45 = {(x, 45): c for (x, y), c in top.items() if y == 45}
    return over(shift(top, 0, -1), row45, rest)


def f2() -> Pix:
    """Idle hold: frame 0 with the ear tip (outermost pixels) twitched up 1 px, plum outline behind it."""
    p = key_pix(0)
    tip, rest = cut(p, lambda x, y: x <= 21 and y <= 22)
    p = over(rest, shift(tip, 0, -1))
    return put(p, "K", 21, 22) if (21, 22) not in p else p


def f3() -> Pix:
    """Idle breath out: the head (rows 20 to 33) 1 px down; the jaw outline merges into the vest collar."""
    p = key_pix(0)
    return move(p, head_mask(0), 0, 1)


# ------------------------------------------------------------------ walk family (frames 5, 6, 8, 9)

SWING_LEG = """
KmaK.........
.KmaK.KKKKKKK
..KmaKKmmmmmK
...KKKKaaaaaK
......KKKKKKK
"""


def swing_leg(light: bool) -> Pix:
    """The lifted leg swinging through: a 2 px wide shin and a 5 x 2 foot (7 x 4 with its outline) on rows 49 to
    52 at x 27 to 33."""
    p = parse(SWING_LEG, 21, 48)
    if light:
        p = {k: {"m": "l", "a": "s"}.get(c, c) for k, c in p.items()}
    return p


def walk_down(key: int) -> Pix:
    """Walk 'down' frame: the body one px lower than the contact key, the planted front foot 5 px back and
    flat, the trailing foot's heel lifted 2 px with its toe still down."""
    p = key_pix(key)
    up, legs = cut(p, lambda x, y: y <= 46)
    front, back = cut(legs, lambda x, y: x >= 27)
    dx = {47: 0, 48: 0, 49: -1, 50: -2, 51: -3, 52: -4, 53: -5, 54: -5, 55: -5}
    front = shear(front, lambda x, y: True, lambda y: dx[y])
    toe_x = min(x for (x, y) in back if y == 55)
    new_back: Pix = {}
    for (x, y), c in back.items():
        lift = 0
        if y >= 51:
            lift = 2 if x >= toe_x + 6 else (1 if x >= toe_x + 4 else 0)
        new_back[(x, y - lift)] = c
    return over(new_back, front, shift(up, 0, 1))


PLANTED_SHIN = {
    4: ("""
KlmsK.
""", [28, 27, 26, 25]),
    7: ("""
KmaK
""", [29, 27, 25, 23]),
}


def walk_pass(key: int) -> Pix:
    """Walk 'pass' frame: the body at idle height; the planted front foot another 5 px back (it travels back 5 px
    per frame at the 62 px/s chase speed: x 33, 28, 23 for frames 4 to 6 and x 31, 26, 21 for 7 to 9); the leg is
    the key's hip rows, a drawn 2 px shin running down to the key's foot rows shifted 10 px; the far leg swings
    through with a 5 x 2 foot; the pineapple 1 px higher."""
    p = key_pix(key)
    up, legs = cut(p, lambda x, y: y <= 46)
    front, _back = cut(legs, lambda x, y: x >= 27)
    hip = {k: c for k, c in front.items() if k[1] <= 48}
    foot = shift({k: c for k, c in front.items() if k[1] >= 53}, -10, 0)
    shin_text, xs = PLANTED_SHIN[key]
    shin: Pix = {}
    for i, x in enumerate(xs):
        shin.update(parse(shin_text, x, 49 + i))
    leg = over(foot, shin, hip)
    up = shift(up, 0, -1)
    up = move(up, lambda x, y: x >= 45 and y <= 36, 0, -1)
    swing = swing_leg(light=(key == 7))
    return over(swing, leg, up) if key == 4 else over(leg, swing, up)


def f5() -> Pix: return walk_down(4)
def f6() -> Pix: return walk_pass(4)
def f8() -> Pix: return walk_down(7)
def f9() -> Pix: return walk_pass(7)


# ------------------------------------------------------------------ attack (frames 10, 14, 15)

def outline_exposed(p: Pix, pred) -> Pix:
    """Add a plum outline pixel at every empty cell inside ``pred`` that touches a non-outline pixel."""
    add: Pix = {}
    for (x, y), c in p.items():
        if c == "K":
            continue
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            if q not in p and pred(*q):
                add[q] = "K"
    return over(add, p)


def insert_col(p: Pix, c: int) -> Pix:
    """Widen by one pixel: columns >= c move right 1 and column c repeats column c - 1."""
    keep = {k: v for k, v in p.items() if k[0] < c}
    moved = {(x + 1, y): v for (x, y), v in p.items() if x >= c}
    col = {(c, y): v for (x, y), v in p.items() if x == c - 1}
    return over(col, keep, moved)


def f10() -> Pix:
    """Tell lead-in, halfway from 0 to 11: torso and head 1 px back of the tell, the arm raised to shoulder
    height, the stick pointing up-back, the pineapple at x 10 to 18, rows 14 to 22, the mouth half open."""
    p = key_pix(11)
    # half-open mouth: the lowest dark mouth row closes into the lower lip
    om = [x for (x, y), c in p.items() if y == 33 and c == "o"]
    for x in om:
        p[(x, 33)] = "K"
    for x in range(min(om) - 1, max(om) + 2):
        if p.get((x, 34)) == "K":
            p[(x, 34)] = "s"
    top, rest = cut(p, lambda x, y: x <= 19 and y <= 21)
    rest = erase(rest, lambda x, y: x <= 19 and 22 <= y <= 31)
    top = shift(top, 5, 8)
    body, legs = cut(rest, lambda x, y: y <= 46)
    body = shift(body, -1, -1)
    body = over({(x, 46): c for (x, y), c in body.items() if y == 45}, body)
    back, front = cut(legs, lambda x, y: x <= 21)
    back = shear(back, lambda x, y: True, lambda y: {47: -1, 48: 0, 49: 0, 50: 1, 51: 2, 52: 3}.get(y, 4))
    front = shear(front, lambda x, y: True, lambda y: -1 if y <= 50 else 0)
    stick = parse("""
.K.
KyK
.Ky
..K
""", 18, 24)
    return over(back, front, body, top, stick)


def f14() -> Pix:
    """Follow-through: frame 13's body 2 px lower and 1 px forward, the arms straight down-forward (fists just
    under the chin, so the clenched face stays visible), the pineapple on the floor crown first."""
    p = key_pix(13)
    fruit, rest = cut(p, lambda x, y: x >= 46 or (x >= 41 and y <= 31))
    rest = erase(rest, lambda x, y: y <= 28 and x >= 27)
    upper_part, legs = cut(rest, lambda x, y: y <= 45)
    upper_part = outline_exposed(shift(upper_part, 1, 2), lambda x, y: y <= 31 and x >= 27)
    fruit = shift(erase(fruit, lambda x, y: x < 46), -3, 15)
    arms = parse("""
KmllllsK.
.KlmmmsK
...KlmmK
....KKKK
""", 34, 45)
    stick = parse("""
K..
KyK
.KyK
..KyK
...KyK
....KK
""", 41, 47)
    return over(legs, upper_part, arms, stick, fruit)


def f15() -> Pix:
    """Yank back, halfway from 14 to 0: torso upright (frame 0's body, 1 px lower), eyes open again, the
    stick pulled back across the body with the pineapple at x 40 to 46, rows 36 to 44."""
    p = key_pix(0)
    fruit, rest = cut(p, lambda x, y: x >= 45 and y <= 37)
    rest = erase(rest, lambda x, y: x >= 40 and 34 <= y <= 40 and c_in(rest, x, y, "KxyG"))
    body, legs = cut(rest, lambda x, y: y <= 45)
    head, body = cut(body, head_mask(0))
    body = over(body, head)
    fruit = shift(fruit, -6, 7)
    return over(legs, shift(body, 0, 1), fruit)


def c_in(p: Pix, x: int, y: int, chars: str) -> bool:
    return p.get((x, y)) in tuple(chars)


# ------------------------------------------------------------------ hurt, launch (frames 17, 18)

def key_weapon(a: int) -> Pix:
    return parse("\n".join(key_cels(a)["weapon"]))


def f17() -> Pix:
    """Hurt stagger, halfway from 16 to 0: the head half way back (2 px forward of frame 16), the arm lowered to
    shoulder height, the pineapple hanging low behind at x 2 to 10, the back foot at x 15 to 22 (frame 16's back
    leg, 4 px forward) and the flat front foot at x 28 to 36 (frame 0's front leg, 1 px back)."""
    p = key_pix(16)
    weapon = key_weapon(16)
    stick_fruit = {k: c for k, c in weapon.items() if k[0] <= 12}      # the rope belt stays where it is
    fruit = shift({k: c for k, c in stick_fruit.items() if k[1] >= 35}, 2, 0)
    p = erase(p, lambda x, y: (x, y) in stick_fruit)
    head, rest = cut(p, lambda x, y: x >= 19 and y <= 34)
    rest = erase(rest, lambda x, y: x <= 18 and y <= 34)
    back, rest = cut(rest, lambda x, y: x <= 24 and y >= 47)
    back = shear(back, lambda x, y: True, lambda y: {47: 0, 48: 1, 49: 1, 50: 2, 51: 3}.get(y, 4))
    rest = erase(rest, lambda x, y: x >= 24 and y >= 47)
    leg = shift({k: c for k, c in key_pix(0).items() if k[0] >= 28 and k[1] >= 47}, -1, 0)
    arm = parse("""
..KKKKK...
.KllllsK..
.KlmmmsK..
..KKlmsK..
....KlmsK.
....KlmsK.
....KlmmK.
""", 10, 29)
    stick = parse("""
..K.
.KyK
KyK.
yK..
""", 8, 31)
    head = outline_exposed(shift(head, 2, 0), lambda x, y: y <= 34 and 19 <= x <= 23)
    return over(leg, back, rest, arm, head, stick, fruit)


def lean18(x: int, y: int) -> tuple[int, int]:
    """Frame 18's lean: every row is translated by an integer that grows with its height above row 46."""
    return x - round(0.6 * (46 - y)) + 12, y - 2


def idle_pineapple() -> Pix:
    """The idle pineapple stamp (leaf, fruit and outline) cut out of key 0."""
    return {k: c for k, c in key_pix(0).items() if k[0] >= 45 and k[1] <= 36}


def f18() -> Pix:
    """Knockback launch, halfway from 16 to 19: frame 16's body leaned back about 35 degrees by translating its
    rows (the legs tucked: frame 0's front leg, frame 16's back leg), the feet just off the floor (lowest row 53).
    The weapon starts its arc: the stick stands vertical above the hand and the idle pineapple is upright on it
    (fruit interior from x 10, row 10)."""
    p = key_pix(16)
    weapon = key_weapon(16)
    p = erase(p, lambda x, y: (x, y) in weapon and x <= 12)
    p = erase(p, lambda x, y: x >= 24 and y >= 47)
    p = over(shift({k: c for k, c in key_pix(0).items() if k[0] >= 28 and k[1] >= 47}, -3, 0), p)
    out: Pix = {}
    for (x, y), c in p.items():
        out[lean18(x, y)] = c
    fruit = idle_pineapple()
    inner = [k for k, c in fruit.items() if c in "FGH"]
    fx, fy = min(x for x, _ in inner), min(y for _, y in inner)
    fruit = shift(fruit, 10 - fx, 10 - fy)
    fruit_bottom = max(y for _, y in fruit)
    cx = (min(x for x, _ in fruit) + max(x for x, _ in fruit)) // 2
    hand_top = min(y for (x, y), c in out.items() if 7 <= x <= 15 and y >= fruit_bottom + 2 and c != "K")
    stick = {}
    for y in range(fruit_bottom, hand_top):
        stick[(cx, y)] = "y"
        stick[(cx - 1, y)] = "K"
        stick[(cx + 1, y)] = "K"
    return over(stick, out, fruit)


# ------------------------------------------------------------------ down, getup (frames 20, 21, 23, 24)

def squash22(p: Pix) -> Pix:
    """Frame 22 squashed: everything above the floor row 1 px lower, the knees 2 px lower, 1 px wider at both ends."""
    low, rest = cut(p, lambda x, y: y <= 54)
    knees, body = cut(low, lambda x, y: 33 <= x <= 43)
    p = over(rest, shift(body, 0, 1), shift(knees, 0, 2))
    p = insert_col(p, 9)
    return insert_col(p, 41)


def settle22(p: Pix) -> Pix:
    """Frame 22 settled: the head 1 px up, the far knee 1 px down."""
    head, rest = cut(p, lambda x, y: x <= 20 and y <= 54)
    p = over(rest, shift(head, 0, -1))
    return move(p, lambda x, y: 33 <= x <= 36 and y <= 50, 0, 1)


def f20() -> Pix:
    """Floor impact: frame 22 squashed (body 1 px lower, knees 2 px lower), 1 px wider at both ends, eyes shut."""
    p = key_pix(22)
    p[(14, 49)] = "K"
    p[(15, 49)] = "K"
    for x in (13, 14, 15):
        if p.get((x, 48)) == "K":
            p[(x, 48)] = "m"
    return squash22(p)


def f21() -> Pix:
    """Settle: frame 22 with the head lifted 1 px and the far knee 1 px lower; eyes dazed."""
    return settle22(key_pix(22))


def f23() -> Pix:
    """Push up: the torso sheared to about 45 degrees (sitting up), an upright idle head with dazed eyes, the
    arms behind as props, the legs along the floor, the stick on the floor."""
    p0 = key_pix(0)
    head, rest = cut(p0, head_mask(0))
    for (x, y), c in list(head.items()):
        if c == "e":
            head[(x, y)] = "w"
            if head.get((x, y - 1)) not in (None, "K"):
                head[(x, y - 1)] = "K"
    torso = {k: c for k, c in rest.items() if 30 <= k[1] <= 46 and k[0] <= 35}
    sheared = {(x - round(0.7 * (46 - y)) + 6, y + 8): c for (x, y), c in torso.items()}
    head = shift(head, -5, 8)
    props = parse("""
.....KK.
....KlmK
...KlmsK
..KlmsK.
.KlmsK..
KllsKK..
KllsK...
KKKK....
""", 15, 48)
    legs = parse("""
....KKKK.....
...KllllK....
..KlmmmmssK..
.KlmmmsKKsmK.
KlmmmsK..KsmK
KmmmsK....KsmK
KKssK......KsmK
...........KsmK
..........KmmmK
..........KsssK
""", 32, 46)
    weapon = shift(key_weapon(22), 4, 0)
    return over(weapon, props, legs, sheared, head)


def f24() -> Pix:
    """Kneel: frame 0's upper body 4 px lower, the near knee on the floor at (34, 55), the far foot planted, the
    fist closing on the stick, the pineapple on the floor in front."""
    p = key_pix(0)
    fruit = shift({k: c for k, c in p.items() if k[0] >= 45 and k[1] <= 37}, 1, 18)
    body = erase(p, lambda x, y: (x >= 41 and y >= 34) or (x >= 45 and y <= 37) or y >= 46)
    body = shift(body, 0, 4)
    stick = parse("""
KK.........
KyKK.......
.KyyKK.....
...KyyyKK..
.....KyyyKK
.......KyyK
.........KK
""", 37, 46)
    far = parse("""
.KaaK.......
.KaaK.......
.KmaKK......
..KmaaKKK...
..KKmmaaaK..
....KKKKKKK.
""", 34, 50)
    near = parse("""
KK.............KKKK
KlK......KKKKKKKllK
KllKKKKKKllllllllmmK
KsssssssssssssssssK
""", 17, 52)
    return over(far, near, body, stick, fruit)


FRAMES = {1: f1, 2: f2, 3: f3, 5: f5, 6: f6, 8: f8, 9: f9, 10: f10, 14: f14, 15: f15, 17: f17, 18: f18,
          20: f20, 21: f21, 23: f23, 24: f24}


ACCENTS = {"e", "w", "H"}
NB8 = ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1))


def orphans(p: Pix) -> list[tuple[int, int]]:
    """Pixels the lint counts as orphans: no same-key pixel in the 8-neighbourhood (accents exempt)."""
    out = []
    for (x, y), c in sorted(p.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        around = [p[(x + dx, y + dy)] for dx, dy in NB8 if (x + dx, y + dy) in p]
        if c not in ACCENTS and around and c not in around:
            out.append((x, y))
        elif not around:
            out.append((x, y))
    return out


def despeckle(p: Pix, editable=None, passes: int = 4) -> Pix:
    """Recolour orphan pixels to the commonest neighbouring key (delete isolated ones). ``editable`` limits
    which pixels may be touched (default: all)."""
    p = dict(p)
    for _ in range(passes):
        changed = False
        for (x, y) in orphans(p):
            if editable is not None and (x, y) not in editable:
                continue
            around = [p[(x + dx, y + dy)] for dx, dy in NB8 if (x + dx, y + dy) in p]
            if not around:
                del p[(x, y)]
            else:
                best = max(sorted(set(around)), key=lambda k: (around.count(k), k != "K"))
                p[(x, y)] = best
            changed = True
        if not changed:
            break
    return p


def build_a_pix(n: int) -> Pix:
    return key_pix(n) if n in KEY_IDS else despeckle(FRAMES[n]())


# ------------------------------------------------------------------ variant B accessory overlay

# vest-patch box (x0, y0, x1, y1) of each key's accessory layer; everything else on it follows the head
PATCH_BOX = {0: (22, 34, 24, 36), 4: (22, 35, 24, 37), 7: (22, 35, 24, 37), 11: (17, 37, 19, 39),
             12: (17, 37, 19, 39), 13: (16, 31, 18, 33), 16: (14, 37, 16, 39), 19: (24, 42, 27, 44),
             22: (21, 53, 24, 55), 25: (21, 53, 24, 55), 26: (25, 40, 27, 43)}


def key_acc(a: int) -> tuple[Pix, Pix]:
    """The key's accessory overlay split into (head part, vest-patch part)."""
    acc = parse("\n".join(key_cels(a)["accessory"]))
    patch, head = cut(acc, box(*PATCH_BOX[a]))
    return head, patch


def acc_follow(a: int, head_xf, patch_xf) -> Pix:
    head, patch = key_acc(a)
    return over(patch_xf(patch), head_xf(head))


def tr(dx: int, dy: int):
    return lambda p: shift(p, dx, dy)


def acc2() -> Pix:
    """Frame 2: the ear notch rides with the twitching ear tip."""
    head, patch = key_acc(0)
    notch = {k: c for k, c in head.items() if k in ((21, 22), (22, 23))}
    return over(patch, {k: c for k, c in head.items() if k not in notch}, shift(notch, 0, -1))


def acc23() -> Pix:
    head, patch = key_acc(0)
    sheared = {(x - round(0.7 * (46 - y)) + 6, y + 8): c for (x, y), c in patch.items()}
    return over(sheared, shift(head, -5, 8))


def acc18() -> Pix:
    return {lean18(x, y): c for (x, y), c in over(*key_acc(16)).items()}


def acc17() -> Pix:
    """Frame 17: the head overlay follows the head; the hand-tidy pixels next to the old raised hand drop out."""
    head, patch = key_acc(16)
    head = {k: c for k, c in head.items() if k not in ((12, 24), (13, 24))}
    return over(patch, shift(head, 2, 0))


ACC = {
    1: lambda: acc_follow(0, tr(0, -1), tr(0, -1)),
    2: acc2,
    3: lambda: acc_follow(0, tr(0, 1), tr(0, 0)),
    5: lambda: acc_follow(4, tr(0, 1), tr(0, 1)),
    6: lambda: acc_follow(4, tr(0, -1), tr(0, -1)),
    8: lambda: acc_follow(7, tr(0, 1), tr(0, 1)),
    9: lambda: acc_follow(7, tr(0, -1), tr(0, -1)),
    10: lambda: acc_follow(11, tr(-1, -1), tr(-1, -1)),
    14: lambda: acc_follow(13, tr(1, 2), tr(1, 2)),
    15: lambda: acc_follow(0, tr(0, 1), tr(0, 1)),
    17: acc17,
    18: acc18,
    20: lambda: squash22(over(*key_acc(22))),
    21: lambda: settle22(over(*key_acc(22))),
    23: acc23,
    24: lambda: acc_follow(0, tr(0, 4), tr(0, 4)),
}


def build_acc_pix(n: int) -> Pix:
    return over(*key_acc(n)) if n in KEY_IDS else ACC[n]()


def clean_acc(a_flat: Pix, acc: Pix) -> Pix:
    """Despeckle the B composite, touching only accessory pixels (A's body and weapon stay as they are)."""
    comp = over(a_flat, acc)
    fixed = despeckle(comp, editable=set(acc))
    return {k: fixed[k] for k in acc if k in fixed}


# ------------------------------------------------------------------ grids

def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, frames in TAGS for n, t in frames}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def a_cels(n: int) -> dict[str, list[str]]:
    if n in KEY_IDS:
        c = key_cels(n)
        return {"body": list(c["body"]), "weapon": list(c["weapon"])}
    return belt_to_body(ag.split_layers(rows_of(build_a_pix(n))))


BELT_RUN = 7   # the rope belt is a horizontal run of at least 7 wood pixels; the stick never runs that long


def belt_pixels(weapon: Pix) -> Pix:
    """The rope belt inside a weapon cel: horizontal runs of >= BELT_RUN consecutive wood (x, y) pixels."""
    out: Pix = {}
    for y in sorted({y for _x, y in weapon}):
        xs = sorted(x for (x, yy), c in weapon.items() if yy == y and c in "xy")
        run: list[int] = []
        for x in xs + [None]:
            if run and (x is None or x != run[-1] + 1):
                if len(run) >= BELT_RUN:
                    out.update({(rx, y): weapon[(rx, y)] for rx in run})
                run = []
            if x is not None:
                run.append(x)
    return out


def belt_to_body(cels: dict[str, list[str]]) -> dict[str, list[str]]:
    """Review D7: the rope belt belongs to the body. Applied to in-between frames only; the frozen keys keep it
    on the weapon layer exactly as the Opus keys were saved."""
    weapon = parse("\n".join(cels["weapon"]))
    belt = belt_pixels(weapon)
    body = over(parse("\n".join(cels["body"])), belt)
    return {"body": rows_of(body), "weapon": rows_of({k: c for k, c in weapon.items() if k not in belt})}


def build(variant: str) -> dict:
    ticks = frame_ticks()
    frames = []
    for n in range(27):
        cels = a_cels(n)
        if variant == "B":
            cels["accessory"] = (list(key_cels(n)["accessory"]) if n in KEY_IDS
                                 else rows_of(clean_acc(build_a_pix(n), build_acc_pix(n))))
        frames.append({"duration_ms": duration_ms(ticks[n]), "cels": cels})
    used = {ch for f in frames for rows in f["cels"].values() for r in rows for ch in r if ch != "."}
    pal = FIX["palette_a" if variant == "A" else "palette_b"]
    assert used <= {e["key"] for e in pal}, used - {e["key"] for e in pal}
    tags, i = [], 0
    for name, fr in TAGS:
        assert [n for n, _t in fr] == list(range(i, i + len(fr)))
        tags.append({"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"})
        i += len(fr)
    layers = ["body", "weapon"] + (["accessory"] if variant == "B" else [])
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": list(ag.PIVOT), "palette": pal,
            "layers": layers, "frames": frames, "tags": tags}


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


MANIFEST = {"A": ROOT / "assets/manifests/goblin_a_h36.json", "B": ROOT / "assets/manifests/goblin_b_h36.json"}
AUTHOR = {n: ("key" if n in KEY_IDS else "sonnet") for n in range(27)}


def manifest(variant: str) -> dict:
    v = variant.lower()
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    frames = [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": duration_ms(ticks[n]),
               "author": AUTHOR[n]} for n in range(27)]
    if variant == "A":
        origin = ("GOB-01 common goblin variant A at the decided 36 px height: 11 frozen Opus keys (atlas frames "
                  "0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26, copied unchanged from tests/fixtures/goblin_gob01_keys.json) "
                  "plus 16 in-betweens assembled by scripts/author_goblin_frames.py from key pixels (whole parts and "
                  "rows translated, a few hand-placed pixels and ASCII stamps, a deterministic despeckle pass); "
                  "layers body and weapon")
    else:
        origin = ("GOB-01 common goblin variant B: variant A's body and weapon cels unchanged plus an accessory layer "
                  "(red rag bandana with two tails, ear notch, missing upper tooth, stitched vest patch) for all 27 "
                  "frames: the 11 key overlays are frozen, the 16 in-between overlays are the key overlay translated "
                  "with the head and the vest of each frame; visual variation only, same frames, timing, pivot and boxes")
    return {
        "schema_version": 1,
        "id": f"goblin_common_{v}_h36_candidate_v1",
        "type": "character_animation",
        "source_path": f"assets/source/characters/goblin/goblin_{v}_h36.ase",
        "grid_source": f"assets/source/characters/goblin/goblin_{v}_h36.grid.json",
        "output_atlas": f"generated/art/goblin_{v}_h36.png",
        "output_metadata": f"generated/art/goblin_{v}_h36.json",
        "palette_id": "goblin-floor01-v0",
        "frame_size": [W, H],
        "pivot": list(ag.PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: common Floor 1 goblin; runtime role IP-neutral small melee enemy)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": origin,
            "generator": ("Design, size study and keys: Claude Opus (two separate agents propose and decide the size). "
                          "In-betweens, atlases, validators and evidence: Claude Sonnet, composed by "
                          "scripts/author_goblin_frames.py. Review: Claude Opus, PASS 2026-11-17 (a separate agent from the designer). No image-model "
                          "raster output, no tracing, no resampling"),
            "generated_art": True,
            "input_reference_ids": ["floor-01-opening-concept-references-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "goblin": {
            "status": "Opus review PASSED 2026-11-17; awaiting owner review; Bevy integration pending; 27 frames, 11 of them Opus keys; not wired into any Bevy binary",
            "spec": "docs/02-art/gob-01-goblin-spec.md",
            "variant": variant,
            "opaque_height_idle": 36,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": GROUNDED_TAGS,
            "airborne_tags": AIRBORNE_TAGS,
            "playback": PLAYBACK,
            "key_frames": list(KEY_IDS),
            "key_fixture": "tests/fixtures/goblin_gob01_keys.json",
            "attack_ticks": {"tell": [0, 23], "swing_smear": [24, 25], "swing_impact": [26, 29], "recover": [30, 49]},
            "frames": frames,
        },
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true", help="exit 1 if a committed grid differs from the build")
    parser.add_argument("--out-dir", type=Path, default=None, help="write the grids here instead of the sources")
    args = parser.parse_args(argv)
    status = 0
    for variant in ("A", "B"):
        for kind, path, text in (
                ("grid", OUT[variant], dump(build(variant))),
                ("manifest", MANIFEST[variant], json.dumps(manifest(variant), indent=2) + "\n")):
            if args.out_dir is not None:
                path = args.out_dir / path.name
            if args.check:
                same = path.is_file() and path.read_text(encoding="utf-8") == text
                print(f"goblin {variant} {kind} " + ("matches the build" if same else "DIFFERS from the build"))
                status |= 0 if same else 1
            else:
                path.write_text(text, encoding="utf-8")
                print(f"wrote {path}")
    return status


if __name__ == "__main__":
    sys.exit(main())


