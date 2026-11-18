#!/usr/bin/env python3
"""Deterministic authoring script for SCAT-01 (Scatterer readability set).

Writes ``assets/source/characters/scatterer/scat-01.grid.json`` (pixel-grid-v1) and
the matching ``.ase`` via ``scripts/pixel_grid_to_ase.py``. Design spec:
``docs/02-art/scat-01-spec.md``.

Design phase (top tier): key frame 0 (``idle``, the master silhouette) is authored
as literal pixel rows (``IDLE0``). Execution phase (medium tier): frames 1 to 15
are composed from frame 0's body (shell and head) with whole slices translated
(spec sections 7 and 8), plus legs and antennae drawn as short polylines. Frame 0
stays byte-identical: it is frozen in ``tests/fixtures/scat-01_keys.json``.

No concept pixels are traced or resampled; every pixel is placed by this file.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "source" / "characters" / "scatterer"
GRID = OUT_DIR / "scat-01.grid.json"
ASE = OUT_DIR / "scat-01.ase"
MANIFEST = ROOT / "assets" / "manifests" / "scat-01.json"

W, H = 48, 32
PIVOT = [24, 31]

# Palette `scatterer-floor01-v1`: 16 keys used, 4 slots reserved (spec section 6).
PALETTE = [
    # key, name, rgba, accent
    ("K", "outline_char", "#1c0e12ff", False),
    ("a", "chitin_0_deep", "#3d1519ff", False),
    ("b", "chitin_1_shade", "#5f1d1dff", False),
    ("c", "chitin_2_mid", "#84291fff", False),
    ("d", "chitin_3_light", "#ab4329ff", False),
    ("f", "chitin_4_lit", "#d0693aff", False),
    ("h", "sheen_torch", "#ea9c5cff", False),
    ("H", "glint_cream", "#f7dc9cff", True),
    ("g", "leg_dark", "#4a2a28ff", False),
    ("j", "leg_mid", "#8a5240ff", False),
    ("s", "leg_spine", "#b98660ff", True),
    ("u", "belly_cold", "#3e2f3aff", False),
    ("v", "rim_cold", "#66545eff", False),
    ("e", "eye_rim", "#b81c1cff", True),
    ("r", "eye_glow", "#ff4a32ff", True),
    ("w", "eye_flash", "#efe3d3ff", True),
]

# Frame 0 `idle` key, facing right, contact row 31, pivot (24, 31).
# Columns 0..47; rows 0..14 are empty. Authored by the top tier (design key).
IDLE0 = [
    # 0         1         2         3         4
    # 012345678901234567890123456789012345678901234567
    "................................................",  # 15
    "................................................",  # 16
    "............................ddd.................",  # 17
    "...............................dd...............",  # 18
    ".................................d..............",  # 19
    "................KKKKKKKKKK........d.........jj..",  # 20
    ".............KKKhhhahHhhafKKK......d.......j..j.",  # 21
    "...........KKfffffahhhhafffffKK.....d.....j...j.",  # 22
    "..........KdddddddaddddaddddddKKKK..d.....j.....",  # 23
    ".........KcccccccaccccacccccccKhhhKK.d...j......",  # 24
    ".........KcccccccaccccaccccccccKhfffKd...j......",  # 25
    ".......s.KbbbbbbabbbbabbbbbbbbbKfddddKKKj.......",  # 26
    ".......jj.KvvvvvaaaaaaaaaaaaaaaaKcccKarHcK......",  # 27
    "......g..j.KKKKjKKKKKKKjKKKKKKKKjKKKKaercK......",  # 28
    "....sg....jjjjj..a...gj....a.....jjs.KaHHK......",  # 29
    "....g...........a...gs......a......gg.KKKa......",  # 30
    "..gg..........aa..gg.........aa......gg...aa....",  # 31
]
FIRST_ROW = 15


def blank() -> list[str]:
    return ["." * W for _ in range(H)]


def frame_idle0() -> list[str]:
    rows = blank()
    for i, line in enumerate(IDLE0):
        assert len(line) == W, (i, len(line))
        rows[FIRST_ROW + i] = line
    return rows


# ---------------------------------------------------------------------------
# Execution tier (Claude Sonnet): frames 1 to 15, composed from frame 0.
# Pixels live in dicts {(x, y): key}; frame 0's body (shell + head), near
# antenna, far antenna and legs are separated once and re-placed per frame.
# ---------------------------------------------------------------------------
SHELL_FILL = set("abcdfhuv")


def _grid(rows: list[str]) -> dict:
    return {(x, y): c for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


# Far antenna pixels of frame 0 (recoloured `b` -> `d` in the review fix round, spec section 14).
FAR_ANT_XY = {(28, 17), (29, 17), (30, 17), (31, 18), (32, 18), (33, 19), (34, 20), (35, 21), (36, 22), (36, 23),
              (37, 24), (37, 25)}


def _split0():
    g = _grid(frame_idle0())
    body, near_ant, far_ant, legs = {}, {}, {}, {}
    rear_bits = {(7, 26), (7, 27), (8, 27), (6, 28), (9, 28)}
    for (x, y), c in g.items():
        if (x, y) in FAR_ANT_XY:
            far_ant[(x, y)] = c
        elif c == "j" and x >= 40 and y <= 26:
            near_ant[(x, y)] = c
        elif (y >= 29 and not (37 <= x <= 41 and y <= 30 and (x, y) != (41, 30))) \
                or (y == 28 and c == "j") or (x, y) in rear_bits:
            legs[(x, y)] = c
        else:
            body[(x, y)] = c
    # belly line K under the coxae (the legs own the j pixels on row 28)
    for x in (15, 23, 32):
        body[(x, 28)] = "K"
    return body, near_ant, far_ant, legs


BODY0, NEAR_ANT0, FAR_ANT0, LEGS0 = _split0()


class Cv:
    """Painter. Later puts win; out-of-canvas puts are a bug."""

    def __init__(self):
        self.p: dict = {}

    def put(self, x, y, c):
        assert 0 <= x < W and 0 <= y < H, (x, y, c)
        self.p[(x, y)] = c

    def putm(self, pix: dict, dx=0, dy=0):
        for (x, y), c in pix.items():
            self.put(x + dx, y + dy, c)

    def line(self, a, b, c):
        (x0, y0), (x1, y1) = a, b
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            self.put(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n), c)

    def poly(self, pts, c):
        for a, b in zip(pts, pts[1:]):
            self.line(a, b, c)

    def rows(self) -> list[str]:
        r = [["."] * W for _ in range(H)]
        for (x, y), c in self.p.items():
            r[y][x] = c
        return ["".join(q) for q in r]


def seal(body):
    """Close outline gaps: an empty pixel next to shell fill becomes outline K."""
    out = dict(body)
    for (x, y), c in body.items():
        if c in SHELL_FILL:
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if (nx, ny) not in body:
                    out[(nx, ny)] = "K"
    return out


def shift_body(dx, dyfn, src=None):
    """Translate whole columns of the body by (dx, dyfn(original x)); seal outline gaps with K."""
    out = {}
    for (x, y), c in (src or BODY0).items():
        out[(x + dx, y + dyfn(x))] = c
    return seal(out)


def far_leg(cv, start, inner, pad):
    cv.line(start, inner, "a")
    cv.put(*pad, "a")


def rear_leg(cv, bx, by, fx, fy):
    """Near rear leg: coxa, femur along the belly, raised knee, tibia to a 2 px foot at fx..fx+1."""
    cx, cy = 15 + bx, 28 + by
    kx = fx + 5
    cv.poly([(cx, cy), (cx - 1, cy + 1), (kx + 2, cy + 1), (kx + 1, cy), (kx, cy - 1)], "j")
    cv.poly([(kx - 1, cy), (fx + 1, fy)], "g")
    cv.put(fx, fy, "g")
    cv.put(kx, cy - 2, "s")
    cv.put(min(kx - 3, fx + 3), min(fy - 1, cy + 1), "s")


def mid_leg(cv, bx, by, fx, fy):
    cx, cy = 23 + bx, 28 + by
    if fx + 1 <= cx:  # backward leg, foot spans fx..fx+1
        cv.put(cx - 1, cy + 1, "j")
        cv.line((cx - 2, cy + 1), (fx + 1, fy), "g")
        cv.put(fx, fy, "g")
        cv.put(cx - 2, min(cy + 2, fy - 1), "s")
    else:  # forward leg, foot spans fx..fx+1
        cv.put(cx + 1, cy + 1, "j")
        cv.line((cx + 2, cy + 1), (fx, fy), "g")
        cv.put(fx + 1, fy, "g")
        cv.put(cx + 2, min(cy + 2, fy - 1), "s")


def front_leg(cv, bx, by, fx, fy):
    cx, cy = 32 + bx, 28 + by
    cv.line((cx + 1, cy + 1), (cx + 2, cy + 1), "j")
    cv.put(cx + 3, cy + 1, "s")
    cv.line((cx + 3, cy + 2), (fx, fy), "g")
    cv.put(fx + 1, fy, "g")


def far_set(cv, rear, mid, front):
    """Far legs in `a`: rear/mid/front given as (outer foot x, foot row)."""
    fx, fy = rear
    far_leg(cv, (fx + 3, 29), (fx + 1, fy), (fx, fy))
    fx, fy = mid
    far_leg(cv, (fx - 2, 29), (fx, fy), (fx + 1, fy))
    fx, fy = front
    far_leg(cv, (fx - 1, fy - 1), (fx, fy), (fx + 1, fy))


def ant(cv, near=None, far=None):
    if far:  # `d` (lit) so it reads at 1x; `b` only where it crosses the shell
        t = Cv()
        t.poly(far, "d")
        for p, c in t.p.items():
            cv.put(*p, "b" if p in cv.p else c)
    if near:
        cv.poly(near, "j")


def frame_1():
    cv = Cv()
    cv.putm(BODY0)
    cv.putm(LEGS0)
    for (x, y), c in FAR_ANT0.items():
        cv.put(x - 1 if y < 21 else x, y, c)
    for (x, y), c in NEAR_ANT0.items():
        if y < 24:
            cv.put(x + 1, y - 1, c)
        else:
            cv.put(x, y, c)
    return cv.rows()


def frame_2():
    cv = Cv()
    top, rest = {}, {}
    for (x, y), c in list(BODY0.items()) + list(FAR_ANT0.items()):
        (top if 17 <= y <= 21 and 9 <= x <= 30 else rest)[(x, y)] = c
    cv.putm(rest)
    cv.putm(top, 0, 1)  # the row-21 band overwrites the row-22 band where they overlap
    cv.putm(LEGS0)
    for (x, y), c in NEAR_ANT0.items():
        cv.put(x, y + 1, c)
    cv.put(40, 26, "j")  # keep the antenna stem on the head
    return cv.rows()


def _mand(cv, up, down, mid=None):
    for p in list(up) + list(down):
        cv.put(*p, "H")
    if mid:
        cv.put(*mid, "K")


# ---- walk -----------------------------------------------------------------
WALK_NEAR = {  # frame -> (rear fx, fy), (mid fx, fy), (front fx, fy)
    3: ((2, 31), (25, 30), (40, 31)),
    4: ((0, 31), (26, 30), (38, 31)),
    5: ((0, 30), (27, 31), (36, 30)),
    6: ((1, 30), (25, 31), (38, 30)),
}
WALK_FAR = {  # far rear, far mid, far front (outer foot x, row)
    3: ((16, 30), (29, 31), (42, 30)),
    4: ((14, 30), (27, 31), (43, 30)),
    5: ((13, 31), (26, 30), (43, 31)),
    6: ((11, 31), (28, 30), (41, 31)),
}


def walk(n):
    by = -1 if n in (4, 6) else 0
    cv = Cv()
    far_set(cv, *WALK_FAR[n])
    cv.putm(shift_body(1, lambda x: by))
    r, m, f = WALK_NEAR[n]
    rear_leg(cv, 1, by, *r)
    mid_leg(cv, 1, by, *m)
    front_leg(cv, 1, by, *f)
    ap = 21 + by
    ant(cv, near=[(41, 26 + by), (42, 25 + by), (42, 24 + by), (42, 23 + by), (43, ap + 1), (43, ap), (44, ap),
                  (45, ap + 1), (45, ap + 2)],
        far=[(38, 25 + by), (37, 23 + by), (36, 21 + by), (33, 19 + by), (30, 18 + by), (28, 18 + by),
             (26, 18 + by)])
    return cv.rows()


# ---- tell -----------------------------------------------------------------
def frame_8():
    dyf = lambda x: 0 if x <= 16 else (-1 if x <= 23 else (-2 if x <= 29 else (-3 if x == 30 else (-4 if x <= 36 else -5))))
    body = shift_body(-1, dyf)
    cv = Cv()
    far_set(cv, (14, 31), (28, 31), (42, 31))
    cv.putm(body)
    # belly reveal: u strip under the raised front, along the new belly line
    for x in range(26, 36):
        by = max(y for (xx, y), c in body.items() if xx == x and c == "K")
        if (x, by + 1) not in cv.p:
            cv.put(x, by + 1, "u")
    # eye flare and open V mandibles (head now x 36..40, rows 21..25)
    for p, c in {(37, 22): "r", (38, 22): "r", (38, 23): "r", (37, 23): "e", (37, 21): "H"}.items():
        cv.put(*p, c)
    rear_leg(cv, -1, 0, 1, 31)
    mid_leg(cv, -1, -1, 17, 31)
    # raised, pawing near front leg (drawn forward of the belly, under the head)
    cv.poly([(31, 24), (32, 25), (33, 25), (34, 26)], "j")
    cv.poly([(35, 27), (36, 26), (37, 26), (38, 26), (39, 26), (40, 26)], "g")
    cv.put(38, 27, "s")
    cv.poly([(36, 28), (38, 28), (40, 27), (41, 28)], "a")
    ant(cv, near=[(39, 20), (39, 19), (40, 18), (40, 16), (41, 15), (41, 14), (40, 13), (39, 12), (38, 12)],
        far=[(35, 20), (34, 17), (33, 14), (33, 13), (31, 13), (30, 14), (28, 14)])
    # open V mouth, one connected shape that starts on the head front (each arm is a touching 3 px `H` line)
    _mand(cv, [(41, 22), (42, 22), (43, 21)], [(41, 24), (42, 25), (43, 26)], (41, 23))
    return cv.rows()


def frame_7():
    dyf = lambda x: 0 if x <= 23 else (-1 if x <= 29 else -2)
    body = shift_body(-1, dyf)
    cv = Cv()
    far_set(cv, (14, 31), (28, 31), (42, 31))
    cv.putm(body)
    # half-open mandibles: H, K, H stacked at the head front
    for p, c in {(41, 24): "H", (40, 24): "H", (41, 25): "K", (40, 26): "H", (41, 26): "H"}.items():
        cv.put(*p, c)
    rear_leg(cv, -1, 0, 1, 31)
    mid_leg(cv, -1, 0, 18, 31)
    front_leg(cv, -1, -1, 36, 30)
    ant(cv, near=[(40, 24), (41, 22), (41, 20), (41, 18), (42, 16), (43, 15), (44, 15), (44, 17)],
        far=[(36, 22), (34, 20), (32, 18), (30, 16), (28, 15), (26, 15)])
    return cv.rows()


# ---- attack ---------------------------------------------------------------
def _flat_shell(body):
    """Delete the row-22 band and lower rows 20..21 by 1 (flatter dome)."""
    out = {}
    for (x, y), c in body.items():
        if y == 22:
            continue
        out[(x, y + 1 if y < 22 else y)] = c
    return seal(out)


def frame_9():
    cv = Cv()
    far_set(cv, (15, 31), (28, 31), (44, 30))
    cv.putm(shift_body(3, lambda x: 0))
    rear_leg(cv, 3, 0, 2, 31)
    mid_leg(cv, 3, 0, 19, 31)
    front_leg(cv, 3, 0, 40, 30)
    ant(cv, near=[(43, 26), (44, 24), (45, 22), (46, 20), (47, 19), (47, 21)],
        far=[(40, 25), (39, 23), (37, 21), (34, 19), (32, 19), (30, 19)])
    return cv.rows()


def _diag(cv, a, b, n_j, foot):
    t = Cv()
    t.line(a, b, "x")
    for i, p in enumerate(sorted(t.p, key=lambda q: abs(q[0] - a[0]))):
        cv.put(*p, "j" if i < n_j else "g")
    cv.put(*foot, "g")


def frame_10():
    body = _flat_shell(shift_body(5, lambda x: 0))
    cv = Cv()
    far_set(cv, (14, 31), (26, 31), (46, 30))
    # antennae first: the shell is drawn over them. Near antenna streams back along row 20, above the dome;
    # the far antenna is a short stub that hides behind the shell.
    ant(cv, near=[(45, 26), (44, 24), (43, 22), (42, 21), (41, 20), (35, 20)],
        far=[(44, 25), (43, 23), (41, 22), (39, 22)])
    cv.putm(body)
    for p in [(45, 29), (46, 29), (46, 28)]:
        cv.put(*p, "H")
    # legs: long diagonals, femur j then tibia g
    _diag(cv, (20, 28), (3, 31), 9, (2, 31))
    _diag(cv, (28, 28), (21, 31), 3, (20, 31))
    _diag(cv, (37, 28), (44, 31), 3, (45, 31))
    cv.put(25, 30, "s")
    cv.put(10, 30, "s")
    cv.put(41, 29, "s")
    return cv.rows()


def frame_11():
    cv = Cv()
    cv.putm(BODY0, 3, 0)
    cv.putm({k: v for k, v in LEGS0.items() if k not in {(2, 31), (3, 31), (4, 30), (4, 29), (5, 29), (6, 28)}}, 3, 0)
    # near rear foot re-planted at x 5..6
    cv.poly([(8, 28), (7, 29), (6, 30), (5, 31)], "g")
    cv.put(6, 31, "g")
    # mandibles 1 px open
    cv.put(42, 29, "H")
    cv.put(43, 29, "K")
    cv.put(44, 29, "H")
    # antennae as frame 1, compressed to stay on the canvas
    ant(cv, near=[(44, 26), (45, 24), (46, 22), (47, 21), (47, 23)],
        far=[(40, 25), (40, 24), (39, 22), (38, 21), (36, 19), (34, 18), (31, 17), (30, 17), (29, 17)])
    return cv.rows()


# ---- hurt -----------------------------------------------------------------
def frame_12():
    dyf = lambda x: 0 if x <= 23 else (-1 if x <= 30 else -2)
    body = shift_body(-2, dyf)
    cv = Cv()
    far_set(cv, (13, 31), (27, 31), (42, 29))
    cv.putm(body)
    # eye flash: w, w, w with one K pupil at the rear-bottom (eye at x 36..37, rows 25..26)
    for p, c in {(36, 25): "w", (37, 25): "w", (37, 26): "w", (36, 26): "K"}.items():
        cv.put(*p, c)
    rear_leg(cv, -2, 0, 0, 31)
    mid_leg(cv, -2, 0, 15, 31)
    # both front legs off the floor pointing forward-down
    cv.poly([(31, 26), (34, 27), (37, 27)], "j")
    cv.poly([(38, 27), (41, 28), (43, 29)], "g")
    cv.put(40, 27, "s")
    cv.poly([(33, 28), (37, 28), (40, 29)], "a")
    ant(cv, near=[(38, 24), (38, 23), (38, 22), (38, 21), (35, 21), (33, 21)],
        far=[(35, 22), (33, 20), (31, 18), (26, 18)])
    # gaping mandibles: two touching 3 px `H` arms that start on the head front (column x 39, rows 24..28)
    _mand(cv, [(40, 26), (41, 25), (42, 24)], [(40, 28), (41, 29), (42, 30)])
    return cv.rows()


# ---- defeat ---------------------------------------------------------------
def _flipped():
    """Frame 0 shell and head mirrored vertically, resting on row 31 (spec section 7, frame 15)."""
    out = {(x, 51 - y): c for (x, y), c in BODY0.items()}
    for (x, y), c in list(out.items()):
        if y == 24 and c in "abcdfhv":
            out[(x, y)] = "a" if x >= 36 else ("v" if x % 6 in (0, 1) else "u")
        elif y in (29, 30) and c in "bcdfhua":
            out[(x, y)] = "a"
        elif x >= 36 and c in "bcdfh":
            out[(x, y)] = "a"
        elif c in "hH" and y >= 25:
            out[(x, y)] = "c" if c == "h" else "a"
    return out


def frame_15():
    body = _flipped()
    for p in [(38, 24), (39, 24), (38, 23), (39, 23)]:
        body[p] = "a"
    body[(38, 23)] = "K"
    body[(40, 22)] = "H"
    body[(39, 22)] = "a"
    cv = Cv()
    for x in (15, 23, 32):  # far legs, behind the shell: 3 px up, hooked toward the centre, in `a`
        d = 1 if x < 25 else -1
        cv.poly([(x + 3, 22), (x + 3, 21), (x + 3, 20), (x + 3 + d, 19)], "a")
    cv.putm(body)
    for x in (15, 23, 32):  # near legs: 4 px up (femur `j`, knee `s`), then hooked toward the shell centre
        d = 1 if x < 25 else -1
        cv.line((x, 22), (x, 19), "j")
        cv.put(x, 18, "s")
        cv.put(x + d, 18, "j")
        cv.put(x + 2 * d, 19, "j")
        cv.put(x + 2 * d, 20, "g")
    ant(cv, near=[(41, 25), (41, 26), (42, 27), (42, 28), (42, 29), (42, 30), (42, 31), (43, 31), (44, 31), (45, 31),
                  (46, 31), (47, 31)],
        far=[(40, 26), (41, 27), (42, 28), (43, 29), (43, 30), (44, 30), (45, 30), (46, 30)])
    return cv.rows()


def frame_13():
    body = {}
    for (x, y), c in _flipped().items():  # squash: delete the row-27 band, lower rows 23..26 by 1
        if y == 27:
            continue
        body[(x, y + 1 if y < 27 else y)] = c
    body = seal(body)
    for p in [(38, 24), (39, 24)]:
        body[p] = "e"
    body[(38, 25)] = "r"
    body[(39, 25)] = "r"
    body[(39, 23)] = "a"  # mandibles closed: one H left
    cv = Cv()
    # far legs (`a`): leaning differently, 1 px shorter than the near ones
    cv.poly([(17, 23), (17, 22), (16, 21), (16, 20)], "a")
    cv.poly([(26, 23), (26, 22), (27, 21)], "a")
    cv.poly([(35, 23), (35, 22), (36, 21), (37, 20)], "a")
    cv.putm(body)
    # near legs flail unevenly: rear leans back and is tallest, mid is vertical and shortest, front leans forward
    cv.poly([(15, 23), (14, 22), (14, 21), (13, 20)], "j")
    cv.put(12, 19, "s")
    cv.poly([(23, 23), (23, 22), (23, 21)], "j")
    cv.put(23, 20, "s")
    cv.poly([(32, 23), (33, 22), (33, 21), (34, 20), (35, 19)], "j")
    cv.put(36, 18, "s")
    ant(cv, near=[(41, 26), (42, 27), (43, 28), (44, 29), (45, 29), (46, 30), (47, 31)],
        far=[(40, 27), (41, 28), (42, 29), (43, 30), (44, 30), (45, 31)])
    return cv.rows()


def frame_14():
    body = _flipped()
    for p in [(38, 24), (39, 24), (38, 23), (39, 23)]:
        body[p] = "a"
    body[(38, 24)] = "e"
    body[(38, 23)] = "e"
    body[(39, 23)] = "K"
    body[(40, 22)] = "H"
    body[(39, 22)] = "a"
    cv = Cv()
    for x in (15, 23, 32):
        cv.line((x + 3, 22), (x + 3, 20), "a")
    cv.putm(body)
    for x in (15, 23, 32):  # half curled: 3 px up (femur `j`, knee `s`) and one hook pixel, no tip
        d = 1 if x < 25 else -1
        cv.line((x, 22), (x, 20), "j")
        cv.put(x, 19, "s")
        cv.put(x + d, 19, "j")
    ant(cv, near=[(41, 25), (41, 26), (42, 27), (42, 28), (43, 29), (43, 30), (44, 31), (45, 31), (46, 31), (47, 31)],
        far=[(40, 26), (41, 27), (42, 28), (43, 29), (44, 30), (45, 30), (46, 30)])
    return cv.rows()


ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


def despeckle(rows: list[str]) -> list[str]:
    """Deterministic cleanup: a non-accent pixel with no same-key neighbour takes the commonest neighbour key."""
    g = [list(r) for r in rows]
    for _ in range(3):
        changed = False
        for y in range(H):
            for x in range(W):
                c = g[y][x]
                if c == "." or c in ACCENTS:
                    continue
                around = [g[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < W and 0 <= y + dy < H and g[y + dy][x + dx] != "."]
                if around and c not in around:
                    counts = {k: around.count(k) for k in dict.fromkeys(around)}
                    g[y][x] = max(counts, key=lambda k: counts[k])
                    changed = True
        if not changed:
            break
    return ["".join(r) for r in g]


def all_frames() -> list[list[str]]:
    drawn = [frame_idle0(), frame_1(), frame_2(), walk(3), walk(4), walk(5), walk(6), frame_7(), frame_8(),
             frame_9(), frame_10(), frame_11(), frame_12(), frame_13(), frame_14(), frame_15()]
    return [drawn[0]] + [despeckle(f) for f in drawn[1:]]


# Frame table (spec section 4): index -> (tag, ticks). Ticks are 60 Hz.
TAGS = [
    ("idle", 0, 2),
    ("walk", 3, 6),
    ("attack_tell", 7, 8),
    ("attack_swing", 9, 11),
    ("hurt", 12, 12),
    ("down", 13, 15),
]
TICKS = [10, 6, 10, 4, 4, 4, 4, 6, 20, 2, 5, 14, 10, 4, 8, 0]


def build() -> dict:
    frames = []
    for ticks, cel in zip(TICKS, all_frames()):
        frames.append({"duration_ms": max(1, round((ticks or 6) * 1000 / 60)), "cels": {"body": cel}})
    return {
        "format": "pixel-grid-v1",
        "canvas": [W, H],
        "pivot": PIVOT,
        "palette": [
            {"key": k, "name": n, "rgba": c, **({"accent": True} if acc else {})}
            for k, n, c, acc in PALETTE
        ],
        "layers": ["body"],
        "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


AUTHORS = {0: "key (Claude Opus, frozen)", 3: "key (Claude Sonnet, spec section 7)", 8: "key (Claude Sonnet, spec section 7)",
           10: "key (Claude Sonnet, spec section 7)", 12: "key (Claude Sonnet, spec section 7)",
           15: "key (Claude Sonnet, spec section 7)"}
BOXES = {  # frame pixels (x, y, w, h); pivot-relative values are in spec section 3
    "standing": [10, 22, 28, 10],
    "hurtbox": [9, 20, 30, 9],
    "hurtbox_tell_hold": [8, 16, 31, 13],
    "attack_hitbox": [40, 25, 8, 6],
}


def write_manifest() -> None:
    rows = []
    for name, a, b in TAGS:
        for i in range(a, b + 1):
            ticks = TICKS[i] or None
            rows.append({"index": i, "tag": name, "ticks": ticks,
                         "duration_ms": max(1, round((ticks or 6) * 1000 / 60)),
                         "author": "key" if i in AUTHORS else "sonnet"})
    manifest = {
        "schema_version": 1,
        "id": "scatterer_floor01_v1_candidate",
        "type": "character_animation",
        "source_path": "assets/source/characters/scatterer/scat-01.ase",
        "grid_source": "assets/source/characters/scatterer/scat-01.grid.json",
        "output_atlas": "generated/art/scat-01.png",
        "output_metadata": "generated/art/scat-01.json",
        "palette_id": "scatterer-floor01-v1",
        "frame_size": [W, H],
        "pivot": PIVOT,
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: small Floor 1 cockroach Scatterer; "
                 "runtime role IP-neutral fast ankle-biter swarm enemy)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "SCAT-01 Scatterer readability set: frame 0 (idle) is the frozen Claude Opus hero key; frames 1 to 15 "
                      "(5 further keys and 10 in-betweens) are composed by scripts/author_scat01.py from frame 0's body "
                      "pixels (whole slices translated, legs and antennae drawn as short polylines, a deterministic "
                      "despeckle pass)",
            "generator": "Design, spec and hero key: Claude Opus. Other keys, in-betweens, atlas, validators and evidence: "
                         "Claude Sonnet, composed by scripts/author_scat01.py. Review: pending, a separate top-tier agent. "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "scatterer": {
            "status": "execution done, independent top-tier review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/scat-01-spec.md",
            "opaque_height_idle": 15,
            "tick_hz": 60,
            "hold_preview_ms": 100,
            "grounded_tags": [n for n, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once",
                         "hurt": "once", "down": "hold"},
            "key_frames": sorted(AUTHORS),
            "key_fixture": "tests/fixtures/scat-01_keys.json",
            "boxes": BOXES,
            "attack_ticks": {"tell": [0, 25], "swing_dash": [26, 27], "swing_impact": [28, 32], "recover": [33, 46]},
            "frames": rows,
        },
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    grid = build()
    GRID.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    write_manifest()
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
