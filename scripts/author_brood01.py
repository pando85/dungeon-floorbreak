#!/usr/bin/env python3
"""Deterministic authoring script for BROOD-01 (Scatterer Brood Guardian readability set).

Writes ``assets/source/characters/scatterer/brood/brood-01.grid.json`` (pixel-grid-v1) and
the matching ``.ase`` via ``scripts/pixel_grid_to_ase.py``. Design spec:
``docs/02-art/brood-01-spec.md``.

Key frame 0 (``idle``, the master silhouette, design round 2) is built from the parts below
(``k2_shell``, ``k2_head``, ``k2_mandibles``, legs, antennae); frames 1 to 15 (keys 3, 8, 10, 12, 15 and the ten
in-betweens) are composed from the same parts by the execution-tier functions (spec sections 7, 8 and 13).
Frame 0 and keys 3, 8, 10, 12, 15 are frozen in ``tests/fixtures/brood-01_keys.json``.
No concept pixels are traced or resampled; every pixel is placed by this file.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "source" / "characters" / "scatterer" / "brood"
GRID = OUT_DIR / "brood-01.grid.json"
ASE = OUT_DIR / "brood-01.ase"
MANIFEST = ROOT / "assets" / "manifests" / "brood-01.json"

W, H = 80, 44
PIVOT = [37, 43]
FLOOR = 43

# Palette `scatterer-floor01-v1` (16 shared keys, unchanged hex) + 2 Brood Guardian keys
# (`y`); 17 of 20 (the egg-case colour `o` was cut with the egg case in round 2).
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
    ("y", "crust_pale", "#c4a888ff", True),
]
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


class Cv:
    """Painter. Later puts win; out-of-canvas puts are a bug."""

    def __init__(self):
        self.p: dict = {}

    def put(self, x, y, c):
        assert 0 <= x < W and 0 <= y < H, (x, y, c)
        self.p[(x, y)] = c

    def putm(self, pix: dict):
        for (x, y), c in pix.items():
            self.put(x, y, c)

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


def _interp(pts, x):
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        if x0 <= x <= x1:
            return round(y0 + (y1 - y0) * (x - x0) / (x1 - x0)) if x1 != x0 else y0
    raise ValueError(x)


# ---------------------------------------------------------------------------
# Key 0 v2 parts (design round 2). Shorter, higher arched body (x 14..63, dome top row 15,
# belly row 34, 8 px clearance), big lowered head with two thick hooked mandibles, overlapping
# transverse plates (lifted posterior lips, lit rims, shadow behind), 2 px outlined legs in the
# chitin ramp, no egg case. Do not edit: frame 0 is frozen; copy these parts for other frames.
# ---------------------------------------------------------------------------
K2_TOP = [(14, 31), (15, 28), (17, 25), (20, 22), (24, 20), (29, 18), (34, 17), (39, 16), (46, 15), (50, 16),
          (54, 16), (58, 17), (61, 19), (63, 22), (64, 25)]
K2_BOT = [(14, 32), (16, 33), (20, 34), (55, 34), (59, 33), (64, 28)]
K2_SEAMS = [22, 30, 38, 46]  # abdominal plate seams at the dome top; the pronotum seam is at x 52
K2_PRONOTUM = 52


def _seal(px: dict, fill_keys: str) -> dict:
    out = dict(px)
    for (x, y), c in px.items():
        if c in fill_keys:
            for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                out.setdefault(q, "K")
    return out


def k2_shell() -> dict:
    px = {}
    x0, x1 = K2_TOP[0][0], K2_TOP[-1][0]
    lips = {s + 1 for s in K2_SEAMS} | {s + 2 for s in K2_SEAMS} | {K2_PRONOTUM + 1, K2_PRONOTUM + 2}
    tops = {}
    for x in range(x0, x1 + 1):
        top, bot = _interp(K2_TOP, x), _interp(K2_BOT, x)
        top -= 1 if x in lips else 0  # each plate's posterior lip stands 1 px proud of the plate behind it
        tops[x] = top
        for y in range(top, bot + 1):
            t = (y - top) / max(1, bot - top)
            px[(x, y)] = "K" if y in (top, bot) else ("f" if t < 0.16 else "d" if t < 0.36 else
                                                       "c" if t < 0.66 else "b" if t < 0.86 else "a")
    for x in range(x0 + 2, x1):  # torch sheen row on the upper dome and the pronotum crown
        if 22 <= x <= 61:
            px[(x, tops[x] + 1)] = "h"
    # transverse plates: a shadow line (`a`, `K` near the top) that curves back as it goes down, a lit rim
    # in front of it (the posterior edge of the plate in front: `h` high, `f` mid, `d` low)
    for s in K2_SEAMS + [K2_PRONOTUM]:
        top = tops[s]
        bot = _interp(K2_BOT, s)
        for y in range(top + 1, bot):
            t = (y - top) / max(1, bot - top)
            x = s - round(4 * t * t)
            px[(x, y)] = "K" if (t < 0.3 or s == K2_PRONOTUM) else "a"
            if t < 0.55:  # lit rim only on the upper half; lower down the seam is shadow only
                px[(x + 1, y)] = "h" if t < 0.25 else "f"
            if px.get((x - 1, y)) in ("c", "d", "f"):
                px[(x - 1, y)] = "b" if t > 0.3 else "c"  # the next plate tucks into shadow
    for p in [(33, 21), (34, 21), (35, 21), (34, 22), (35, 22), (56, 20), (57, 20), (58, 20)]:  # two crust scars
        px[p] = "y"
    for p in [(33, 22), (33, 23), (57, 21), (58, 21)]:  # pits, each a 2 px run (round 2 orphan cleanup)
        px[p] = "a"
    px[(15, 29)] = "K"  # rear-tip outline step extended to a 2 px run
    for x in (17, 18, 19, 20):  # cold rim only at the rear underside
        px[(x, _interp(K2_BOT, x) - 1)] = "v"
    for x in list(range(29, 35)) + list(range(41, 47)):  # cold sternites between the leg sockets
        px[(x, 33)] = "u"
    return px


def k2_head() -> dict:
    """Big lowered head x 57..72, rows 22..35: rounded, lit from above-front, 3 x 3 eye under a lowered brow."""
    px = {}
    cx, cy, rx, ry = 64.5, 28.5, 7.6, 6.8
    for y in range(21, 37):
        for x in range(56, 74):
            nx, ny = (x - cx) / rx, (y - cy) / ry
            d = nx * nx + ny * ny
            if d <= 1.0:
                lit = -0.55 * ny + 0.35 * nx  # light from above and slightly in front
                px[(x, y)] = "f" if lit > 0.55 else "d" if lit > 0.15 else "c" if lit > -0.3 else "b" if lit > -0.6 else "a"
    px = _seal(px, "fdcba")
    for p, c in {(63, 26): "r", (64, 26): "r", (65, 26): "H", (63, 27): "r", (64, 27): "r", (65, 27): "r",
                 (63, 28): "e", (64, 28): "e", (65, 28): "e"}.items():
        px[p] = c
    for p in [(61, 24), (62, 25), (63, 25), (64, 25), (65, 25), (66, 26), (66, 27), (62, 26), (62, 27)]:
        px[p] = "K"  # heavy brow, lowered toward the front, and the eye socket
    for p in [(66, 31), (67, 31), (68, 31), (69, 31)]:
        px[p] = "K"  # mouth line (clypeus)
    return px


def _hook(path, lit, edge, thick_to) -> dict:
    """A hooked mandible: the front edge `edge` along `path`, a `lit` body 1 px behind it (2 px behind for
    the first `thick_to` points), outlined `K`."""
    t = Cv()
    t.poly(path, edge)
    body = Cv()
    body.poly([(x - 1, y) for x, y in path], lit)
    body.poly([(x - 2, y) for x, y in path[:thick_to]], lit)
    px = {q: c for q, c in body.p.items()}
    px.update(t.p)
    return _seal(px, lit + edge)


NEAR_PATH0 = [(70, 32), (72, 33), (73, 35), (74, 37), (74, 39), (73, 40), (71, 41)]
FAR_PATH0 = [(66, 34), (67, 36), (68, 38), (67, 40), (65, 41)]


def _under(px: dict) -> dict:
    """1 px dark underside (`K`) under every cream mandible tip so it does not merge with the lit floor."""
    out = dict(px)
    for (x, y), c in px.items():
        if c == "H" and (x, y + 1) not in px and y + 1 <= FLOOR:
            out[(x, y + 1)] = "K"
    return out


def write_key0_scale(out: Path = ROOT / "docs" / "evidence" / "brood-01" / "key0-scale.png") -> Path:
    """Silhouette-gate evidence: key 0 at 1x on lit Floor 1 facing Carl and Donut, with the merged Scatterer
    idle for the species/scale read (640 x 360, no scaling). Run: ``python3 scripts/author_brood01.py key0-scale``."""
    sys.path.insert(0, str(ROOT / "scripts"))
    from pixelart.common import CARL_X, flip, grid_images, load_grid_file  # noqa: E402
    from pixelart.scene import Floor, reference_idle  # noqa: E402

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    brood = grid_images(load_grid_file(GRID))[0]
    scat_grid = load_grid_file(ROOT / "assets" / "source" / "characters" / "scatterer" / "scat-01.grid.json")
    scat = grid_images(scat_grid)[0]
    b_img, b_piv = flip(brood, tuple(PIVOT))
    s_img, s_piv = flip(scat, tuple(scat_grid["pivot"]))
    entries = [(donut, donut_piv, CARL_X - 40, 0), (carl, carl_piv, CARL_X, 0),
               (b_img, b_piv, CARL_X + 110, 0), (s_img, s_piv, CARL_X + 200, 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    Floor().render(entries, 1).save(out)
    return out


# ---------------------------------------------------------------------------
# Parts split from key 0 v2, and the pose helpers for frames 1 to 15 (execution tier, round 2).
# Whole parts are translated (never resampled or rotated); outlines are re-sealed with `K`; legs,
# antennae and mandibles are redrawn from short polylines. ``assemble()`` with its defaults is key 0.
# ---------------------------------------------------------------------------
SHELL0, HEAD0 = k2_shell(), k2_head()
EYE0 = [(63, 26), (64, 26), (65, 26), (63, 27), (64, 27), (65, 27), (63, 28), (64, 28), (65, 28)]
ANT_NEAR0 = [(68, 23), (69, 20), (70, 17), (71, 14), (72, 12), (74, 10), (76, 10), (78, 12), (78, 14)]
ANT_FAR0 = [(61, 23), (60, 20), (58, 17), (56, 14), (53, 12), (50, 11), (47, 12)]
NEAR0 = {"rear": ((25, 33), (15, 36), (7, 42), True, [(14, 34), (10, 37), (7, 40)]),
         "mid": ((38, 33), (33, 37), (29, 42), True, [(31, 36), (29, 39)]),
         "front": ((50, 33), (55, 37), (58, 42), True, [(56, 35), (60, 39)])}
FAR0 = {"rear": ((28, 33), (24, 37), (21, 42), False, ()),
        "mid": ((42, 33), (45, 37), (46, 42), False, ()),
        "front": ((54, 33), (60, 37), (65, 42), False, ())}


def mv(px, dx=0, dy=0, dyf=None):
    """Translate a part whole; dyf(original x) gives a per-column dy."""
    return {(x + dx, y + (dyf(x) if dyf else dy)): c for (x, y), c in px.items()}


def clip(px):
    return {(x, y): c for (x, y), c in px.items() if 0 <= x < W and 0 <= y < H}


def seal_px(px):
    """Close outline gaps: an empty pixel next to a non-outline pixel becomes outline K."""
    out = dict(px)
    for (x, y), c in px.items():
        if c != "K":
            for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if q not in px:
                    out[q] = "K"
    return clip(out)


def drop_row(px, row):
    """Squash by one row: delete `row`; the rows above it move down 1."""
    return {(x, y + 1 if y < row else y): c for (x, y), c in px.items() if y != row}


def body(dx=0, dyf=None, shell_px=None, dy=0):
    """Shell translated by (dx, dy) or by a per-column dyf; the outline is re-sealed only when columns move
    unevenly or the shell was edited (a plain translation keeps key 0's outline exactly)."""
    moved = mv(shell_px or SHELL0, dx, dy, dyf)
    return seal_px(moved) if (dyf is not None or shell_px is not None) else moved


def head_px(dx=0, dy=0, eye=None):
    core = dict(HEAD0)
    for p, c in (eye or {}).items():
        assert p in core, p
        core[p] = c
    return mv(core, dx, dy)


def shear(px, dx=0, dy=0, rows_from=0, tip_dx=0):
    """Translate a mandible whole; original rows >= rows_from get an extra tip_dx."""
    return {(x + dx + (tip_dx if y >= rows_from else 0), y + dy): c for (x, y), c in px.items()}


def mand_pair(dx=0, dy=0, n_near=7, n_far=5, near_tip=0, far_tip=0, rows_from=38):
    """Key-0 mandibles translated by (dx, dy), optionally truncated to the first n points of each hook
    and sheared at the tip rows (near_tip / far_tip px). Returns (near, far)."""
    near = _under(_hook([(x + dx, y + dy) for x, y in NEAR_PATH0[:n_near]], "h", "H", min(4, n_near)))
    far = _hook([(x + dx, y + dy) for x, y in FAR_PATH0[:n_far]], "j", "h", min(2, n_far))
    if near_tip:
        near = shear(near, rows_from=rows_from + dy, tip_dx=near_tip)
    if far_tip:
        far = shear(far, rows_from=rows_from + dy, tip_dx=far_tip)
    return near, far


def shift_pts(pts, start, dx=0, dy=0, end=99):
    return [(x + dx, y + dy) if start <= i < end else (x, y) for i, (x, y) in enumerate(pts)]


def beads_of(near):
    return [(near[1][0], near[1][1] - 1), near[4], (near[6][0] + 1, near[6][1] + 1)]


def auto_spines(knee, foot):
    return [(round(knee[0] + t * (foot[0] - knee[0])), round(knee[1] + t * (foot[1] - knee[1])) - 2)
            for t in (0.45, 0.8)]


def leg(cv, spec):
    """2 px femur and tibia, outlined `K`; near legs in the chitin ramp (`d` lit top, `b` under), far legs
    `b`/`a`; a 3 px `K` tarsus one row under the foot; `s` spine ticks outside the outline."""
    coxa, knee, foot, near, spines = spec
    top, low = ("d", "b") if near else ("b", "a")
    t = Cv()
    t.line(coxa, knee, top)
    t.line((coxa[0], coxa[1] + 1), (knee[0], knee[1] + 1), low)
    mid = ((knee[0] + foot[0]) // 2, (knee[1] + foot[1]) // 2)
    t.line(knee, foot, top)
    t.line((knee[0] + 1, knee[1]), (mid[0] + 1, mid[1]), low)
    px = {p: c for p, c in _seal(t.p, top + low).items() if 0 <= p[0] < W and p[1] <= FLOOR}
    for dx in (-1, 0, 1):
        px[(foot[0] + dx, min(foot[1] + 1, FLOOR))] = "K"
    cv.putm(px)
    for p in (auto_spines(knee, foot) if near and spines is None else spines or ()):
        cv.put(*p, "s")


def nl(coxa, knee, foot, spines=None):
    return (coxa, knee, foot, True, spines)


def fl(coxa, knee, foot):
    return (coxa, knee, foot, False, ())


def shifted(specs, dx, dy):
    return {k: ((c[0] + dx, c[1] + dy), (kn[0] + dx, kn[1] + dy), (f[0] + dx, f[1] + dy), n,
                [(sx + dx, sy + dy) for sx, sy in sp] if sp else sp) for k, (c, kn, f, n, sp) in specs.items()}


def assemble(far, ants, bod, hd, mands, near, post=None):
    """z-order of key 0: far legs, far antenna, far mandible, shell, head, near mandible, near antenna
    (2 px root, beads), near legs."""
    near_ant, far_ant = ants
    cv = Cv()
    for spec in far.values():
        leg(cv, spec)
    cv.poly(far_ant, "d")
    cv.putm(clip(mands[1]))
    cv.putm(bod)
    if post:
        post(cv)
    cv.putm(hd)
    cv.putm(clip(mands[0]))
    cv.poly(near_ant, "j")
    cv.poly([(near_ant[0][0] - 1, near_ant[0][1]), (near_ant[1][0] - 1, near_ant[1][1])], "j")
    for p in beads_of(near_ant):
        cv.put(*p, "s")
    for spec in near.values():
        leg(cv, spec)
    return cv


def key0() -> list[str]:
    cv = assemble(FAR0, (ANT_NEAR0, ANT_FAR0), body(), head_px(), mand_pair(), NEAR0)
    return cv.rows()


# ---- idle in-betweens ------------------------------------------------------
def frame_1():
    ants = (shift_pts(ANT_NEAR0, 5, 1, -1), shift_pts(ANT_FAR0, 3, -1, 0))
    return assemble(FAR0, ants, body(), head_px(), mand_pair(near_tip=1), NEAR0).rows()


def frame_2():
    moved = {(x, y): c for (x, y), c in SHELL0.items() if not (15 <= y <= 24 and 16 <= x <= 52)}
    for (x, y), c in SHELL0.items():  # the dome slice settles 1 and wins over the rows it lands on
        if 15 <= y <= 24 and 16 <= x <= 52:
            moved[(x, y + 1)] = c
    ants = (shift_pts(ANT_NEAR0, 5, 0, 1), ANT_FAR0)
    return assemble(FAR0, ants, body(shell_px=moved), head_px(), mand_pair(), NEAR0).rows()


# ---- walk -----------------------------------------------------------------
def walk(n):
    bx, by = 1, (-1 if n in (4, 6) else 0)
    a_down = n in (3, 4)  # tripod A (near rear, near front, far mid) planted in 3 and 4; tripod B in 5 and 6
    lift = {3: 40, 4: 39, 5: 40, 6: 39}[n]  # foot row of a lifted leg (tarsus one row lower)
    A = {"nrear": (8, 6), "nfront": (62, 60), "fmid": (43, 41)}  # (planted x, lifted x)
    B = {"nmid": (35, 33), "frear": (27, 25), "ffront": (71, 69)}
    xa = {k: (v[0] if a_down else v[1] + (1 if n == 6 else 0)) for k, v in A.items()}
    xb = {k: (v[1] + (1 if n == 4 else 0) if a_down else v[0]) for k, v in B.items()}
    ya, yb = (42, lift) if a_down else (lift, 42)
    ka, kb = (0 if a_down else -1), (0 if not a_down else -1)  # knees rise on the lifted tripod
    near = {"rear": nl((25 + bx, 33 + by), (14 + (0 if a_down else 1), 36 + ka + by), (xa["nrear"], ya)),
            "mid": nl((38 + bx, 33 + by), ((37 if n in (4,) else 38 if not a_down else 36), 36 + kb + by),
                      (xb["nmid"], yb)),
            "front": nl((50 + bx, 33 + by), (57, 37 + ka + by), (xa["nfront"], ya))}
    far = {"rear": fl((28 + bx, 33 + by), ((28 + bx + xb["frear"]) // 2, 37 + kb + by), (xb["frear"], yb)),
           "mid": fl((42 + bx, 33 + by), ((42 + bx + xa["fmid"]) // 2 + 1, 37 + ka + by), (xa["fmid"], ya)),
           "front": fl((54 + bx, 33 + by), ((54 + bx + xb["ffront"]) // 2 + 1, 37 + kb + by), (xb["ffront"], yb))}
    ex = (1, 0) if n in (4, 6) else (1, 1)
    near_ant = shift_pts([(x + bx, y + by) for x, y in ANT_NEAR0], 4, *ex, end=7)
    far_ant = [(x + bx, y + by) for x, y in ANT_FAR0]
    mands = mand_pair(bx, by)
    return assemble(far, (near_ant, far_ant), body(bx, dy=by), head_px(bx, by), mands, near).rows()


# ---- tell -----------------------------------------------------------------
UP_PATH = [(69, 24), (71, 22), (72, 20), (73, 18), (73, 16), (72, 15), (70, 15)]  # frame 8 absolute coordinates
LOW_PATH = [(69, 27), (71, 28), (73, 30), (74, 32), (74, 34), (73, 35), (71, 35)]


def v_mands(dx, dy, n):
    """Mandibles spread in a V (upper arm far, lower arm near), first n points of each hook, translated."""
    far = _under(_hook([(x + dx, y + dy) for x, y in UP_PATH[:n]], "h", "H", min(4, n)))
    near = _under(_hook([(x + dx, y + dy) for x, y in LOW_PATH[:n]], "h", "H", min(4, n)))
    return near, far


def frame_7():
    dyf = lambda x: 1 if x <= 30 else (0 if x <= 37 else (-1 if x <= 45 else (-2 if x <= 51 else -3)))
    near = {"rear": nl((24, 34), (14, 37), (6, 42)), "mid": nl((37, 32), (32, 36), (28, 42)),
            "front": nl((49, 31), (55, 35), (59, 42))}
    far = {"rear": fl((27, 34), (23, 37), (20, 42)), "mid": fl((41, 32), (44, 36), (46, 42)),
           "front": fl((53, 31), (60, 35), (65, 42))}
    ants = ([(67, 20), (68, 17), (69, 14), (70, 11), (71, 9), (72, 7), (74, 7), (76, 9), (76, 11)],
            [(60, 20), (59, 17), (57, 14), (55, 11), (52, 9), (49, 8), (46, 9)])
    return assemble(far, ants, body(-1, dyf), head_px(-1, -3, {(64, 28): "r"}), v_mands(1, 3, 4), near).rows()


def frame_8():
    dyf = lambda x: 2 if x <= 30 else (0 if x <= 37 else (-2 if x <= 45 else (-4 if x <= 51 else -6)))
    bod = body(-2, dyf)

    def post(cv):  # cold belly reveal under the raised belly line
        for x in range(36, 53):
            ys = [y for (xx, y) in bod if xx == x]
            if ys and (x, max(ys) + 1) not in cv.p:
                cv.put(x, max(ys) + 1, "u")

    near = {"rear": nl((23, 35), (13, 38), (3, 42)), "mid": nl((36, 31), (31, 36), (25, 42)),
            "front": nl((48, 29), (55, 25), (60, 28), [(57, 23), (59, 26)])}
    far = {"rear": fl((26, 35), (22, 38), (17, 42)), "mid": fl((40, 31), (44, 36), (49, 42)),
           "front": fl((52, 29), (58, 27), (62, 30))}
    ants = ([(66, 17), (67, 14), (68, 11), (69, 8), (70, 6), (71, 4), (73, 4), (75, 6), (75, 8)],
            [(59, 17), (58, 14), (56, 11), (54, 8), (51, 6), (48, 5), (45, 6)])
    hd = head_px(-2, -6, {(64, 28): "r"})
    return assemble(far, ants, bod, hd, v_mands(0, 0, 7), near, post=post).rows()


# ---- attack ---------------------------------------------------------------
def frame_9():
    near = {"rear": nl((28, 33), (19, 36), (10, 42)), "mid": nl((41, 33), (36, 37), (31, 42)),
            "front": nl((53, 33), (60, 37), (64, 42))}
    far = {"rear": fl((31, 33), (26, 37), (23, 42)), "mid": fl((45, 33), (48, 37), (49, 42)),
           "front": fl((57, 33), (64, 37), (68, 42))}
    ants = ([(70, 23), (70, 20), (69, 17), (68, 14), (66, 12), (63, 11), (60, 11), (57, 12), (55, 13)],
            [(63, 23), (61, 20), (58, 17), (55, 15), (52, 14), (49, 14), (46, 15)])
    return assemble(far, ants, body(3), head_px(2, 0), mand_pair(2, 0, 3, 3), near).rows()


def frame_10():
    bod = body(7, None, drop_row(SHELL0, 22))  # flattened: band row 22 deleted, the rows above drop 1
    near = {"rear": nl((32, 33), (21, 36), (10, 42)), "mid": nl((45, 33), (38, 37), (31, 42)),
            "front": nl((57, 33), (64, 37), (68, 42))}
    far = {"rear": fl((35, 33), (29, 37), (23, 42)), "mid": fl((49, 33), (52, 37), (53, 42)),
           "front": fl((61, 33), (68, 37), (72, 42))}
    ants = ([(72, 24), (70, 21), (67, 18), (64, 16), (61, 14), (58, 13), (56, 13), (54, 14), (52, 15)],
            [(70, 24), (68, 21), (66, 19), (64, 18)])
    near_m = mand_pair(4, 1)[0]  # thrust: the near mandible tip reaches x 78
    far_m = _hook([(70, 35), (72, 37), (74, 39), (76, 41), (78, 41)], "j", "h", 2)  # closes on it
    return assemble(far, ants, bod, head_px(4, 1), (near_m, far_m), near).rows()


def frame_11():
    ants = (shift_pts([(x + 1, y) for x, y in ANT_NEAR0], 5, 1, -1, end=7),
            shift_pts([(x + 1, y) for x, y in ANT_FAR0], 3, -1, 0))
    return assemble(shifted(FAR0, 1, 0), ants, body(1), head_px(1, 0), mand_pair(1, 0, far_tip=-1),
                    shifted(NEAR0, 1, 0)).rows()


# ---- hurt -----------------------------------------------------------------
def frame_12():
    dyf = lambda x: 0 if x <= 37 else (-1 if x <= 51 else -2)
    eye = {p: "w" for p in EYE0}
    eye.update({(63, 28): "K"})  # H glint kept at (65, 26)
    eye[(65, 26)] = "H"
    near = {"rear": nl((22, 33), (12, 36), (4, 42)), "mid": nl((35, 32), (30, 36), (26, 42)),
            "front": nl((47, 32), (52, 35), (56, 40))}
    far = {"rear": fl((25, 33), (21, 37), (18, 42)), "mid": fl((39, 32), (42, 36), (43, 42)),
           "front": fl((51, 31), (57, 35), (62, 42))}
    ants = ([(65, 21), (63, 18), (61, 15), (59, 13), (57, 12), (55, 12), (53, 13), (51, 14), (49, 16)],
            [(58, 21), (56, 19), (53, 16), (49, 14), (45, 13), (41, 13), (38, 14)])
    return assemble(far, ants, body(-3, dyf), head_px(-3, -2, eye), mand_pair(-3, -2, near_tip=1, far_tip=-1),
                    near).rows()


# ---- defeat ---------------------------------------------------------------
def frame_13():
    eye = {p: "a" for p in EYE0}
    eye.update({(63, 28): "e", (64, 28): "e", (65, 28): "e"})
    near = {"rear": nl((25, 37), (13, 38), (7, 42)), "mid": nl((38, 37), (31, 39), (29, 42)),
            "front": nl((50, 37), (57, 39), (58, 42))}
    far = {"rear": fl((28, 37), (22, 39), (21, 42)), "mid": fl((42, 37), (47, 39), (46, 42)),
           "front": fl((54, 37), (62, 39), (65, 42))}
    ants = ([(68, 27), (69, 24), (70, 21), (71, 19), (72, 18), (74, 18), (76, 19), (77, 21), (77, 23)],
            [(61, 27), (59, 24), (56, 21), (53, 20), (50, 20), (47, 21), (45, 23)])
    return assemble(far, ants, body(0, dy=4), head_px(0, 4, eye), mand_pair(0, 4, 5, 3), near).rows()


def frame_15(squash=False, near_ant=None, eye_e=()):
    """Settled defeat (also frame 14 with `squash`): belly-down, legs flat, eye dark, no glow."""
    sh = drop_row(SHELL0, 22) if squash else SHELL0  # frame 14: one band row of the dropped shell deleted
    eye = {p: "a" for p in EYE0}
    for p in eye_e:
        eye[p] = "e"
    if not eye_e:
        eye[(64, 27)] = "K"
    near = {"rear": nl((25, 41), (15, 42), (6, 42), []), "mid": nl((38, 41), (32, 42), (27, 42), []),
            "front": nl((50, 41), (56, 42), (62, 42), [])}
    far = {"rear": fl((28, 42), (25, 42), (21, 42)), "mid": fl((42, 42), (45, 42), (47, 42)),
           "front": fl((54, 42), (60, 42), (65, 42))}
    ants = (near_ant or [(70, 39), (72, 40), (74, 41), (75, 41), (76, 42), (77, 42), (78, 42), (78, 43), (79, 43)],
            [(60, 31), (60, 28), (58, 26), (56, 24), (54, 23), (52, 24), (50, 25)])
    mands = (_under(_hook([(72, 39), (73, 41), (73, 43)], "j", "H", 2)), {})  # short, closed, tips on row 43
    return assemble(far, ants, body(0, None, sh if squash else None, 8), head_px(0, 7, eye), mands, near).rows()


def frame_14():
    return frame_15(True, [(70, 32), (72, 30), (74, 29), (76, 29), (77, 30), (78, 32), (78, 34), (78, 36), (78, 38)],
                    ((64, 28), (65, 28)))


def frame_idle0() -> list[str]:
    return despeckle(key0())


# Frame table (spec section 4): index -> (tag, ticks). Ticks are 60 Hz; 0 = state-driven hold.
TAGS = [
    ("idle", 0, 2),
    ("walk", 3, 6),
    ("attack_tell", 7, 8),
    ("attack_swing", 9, 11),
    ("hurt", 12, 12),
    ("down", 13, 15),
]
TICKS = [14, 10, 14, 6, 6, 6, 6, 8, 24, 3, 6, 18, 12, 6, 10, 0]


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
                    around = [k for k in around if k != "K"] or around  # fold into a colour, not the outline
                    counts = {k: around.count(k) for k in dict.fromkeys(around)}
                    g[y][x] = max(counts, key=lambda k: counts[k])
                    changed = True
        if not changed:
            break
    return ["".join(r) for r in g]


def all_frames() -> list[list[str]]:
    drawn = [key0(), frame_1(), frame_2(), walk(3), walk(4), walk(5), walk(6), frame_7(), frame_8(),
             frame_9(), frame_10(), frame_11(), frame_12(), frame_13(), frame_14(), frame_15()]
    return [despeckle(f) for f in drawn]


def build() -> dict:
    frames = [{"duration_ms": max(1, round((t or 6) * 1000 / 60)), "cels": {"body": cel}}
              for t, cel in zip(TICKS, all_frames())]
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


AUTHORS = {0: "key (Claude Opus, frozen; orphan cleanup by Claude Sonnet per review)", 3: "key (Claude Sonnet, spec section 7)",
           8: "key (Claude Sonnet, spec section 7)", 10: "key (Claude Sonnet, spec section 7)",
           12: "key (Claude Sonnet, spec section 7)", 15: "key (Claude Sonnet, spec section 7)"}
BOXES = {  # frame pixels (x, y, w, h); pivot-relative values are in spec section 3
    "standing": [16, 15, 46, 29],
    "hurtbox": [14, 15, 59, 21],
    "hurtbox_tell_hold": [12, 9, 60, 28],
    "attack_hitbox": [66, 30, 14, 12],
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
        "id": "scatterer_brood_guardian_floor01_v1_candidate",
        "type": "character_animation",
        "source_path": "assets/source/characters/scatterer/brood/brood-01.ase",
        "grid_source": "assets/source/characters/scatterer/brood/brood-01.grid.json",
        "output_atlas": "generated/art/brood-01.png",
        "output_metadata": "generated/art/brood-01.json",
        "palette_id": "scatterer-floor01-v1",
        "frame_size": [W, H],
        "pivot": PIVOT,
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Scatterer Brood Guardian; "
                 "runtime role IP-neutral heavy nest-guard enemy)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "BROOD-01 Scatterer Brood Guardian readability set: frame 0 (idle) is the frozen Claude Opus hero "
                      "key; frames 1 to 15 (5 further keys and 10 in-betweens) are composed by "
                      "scripts/author_brood01.py from frame 0's part pixels (whole parts translated, legs, antennae "
                      "and fangs drawn as short polylines, a deterministic despeckle pass)",
            "generator": "Design, spec and hero key: Claude Opus. Other keys, in-betweens, atlas, validators and "
                         "evidence: Claude Sonnet, composed by scripts/author_brood01.py. Review: pending, a separate "
                         "top-tier agent. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "brood": {
            "status": "execution done, independent top-tier review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/brood-01-spec.md",
            "opaque_height_idle": 34,
            "tick_hz": 60,
            "hold_preview_ms": 100,
            "grounded_tags": [n for n, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once",
                         "hurt": "once", "down": "hold"},
            "key_frames": sorted(AUTHORS),
            "key_fixture": "tests/fixtures/brood-01_keys.json",
            "boxes": BOXES,
            "swing_hurtbox_dx": [3, 6, 3],
            "attack_ticks": {"tell": [0, 31], "swing_dash": [32, 34], "swing_impact": [35, 40], "recover": [41, 58]},
            "frames": rows,
        },
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    if sys.argv[1:] == ["key0-scale"]:
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
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
