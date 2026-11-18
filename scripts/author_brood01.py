#!/usr/bin/env python3
"""Deterministic authoring script for BROOD-01 (Scatterer Brood Guardian readability set).

Writes ``assets/source/characters/scatterer/brood/brood-01.grid.json`` (pixel-grid-v1) and
the matching ``.ase`` via ``scripts/pixel_grid_to_ase.py``. Design spec:
``docs/02-art/brood-01-spec.md``.

Design phase: key frame 0 (``idle``, the master silhouette) is built by the part
functions below (``shell``, ``ootheca``, ``head``, ``antennae``, ``legs``), each a
fixed list of pixels or polylines. Frame 0 is frozen in
``tests/fixtures/brood-01_keys.json``: do not edit the key-0 part functions; to
pose other frames, call them with offsets or copy them into new functions
(spec sections 7 and 8).

Frames 1 to 15 are PLACEHOLDER copies of frame 0 until the execution tier draws
them. No concept pixels are traced or resampled; every pixel is placed by this file.
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
# (`y`, `o`); 18 of 20, 2 slots stay free (spec section 6).
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
    ("o", "ootheca_brown", "#74492fff", False),
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
# Key 0 parts. Coordinates are frame pixels, facing right, idle pose.
# ---------------------------------------------------------------------------
# Dome top outline (x, row): rear tip x 11, highest rows 22..23 at x 27..41, pronotum
# shield x 51..65 dropping to the lip over the head.
SHELL_TOP = [(11, 34), (12, 31), (14, 29), (17, 27), (21, 25), (26, 23), (30, 22), (39, 22), (44, 23),
             (49, 24), (51, 25), (56, 25), (61, 27), (64, 29), (66, 31)]
# Belly outline (x, row): rear underside curls up to the tip; flat belly line on row 37.
SHELL_BOT = [(11, 35), (13, 36), (16, 37), (60, 37), (63, 36), (66, 33)]
# Plate seams: (x at the dome top, slope back per row). Four abdominal seams, one pronotum seam.
SEAMS = [(19, 3), (27, 3), (35, 3), (43, 3)]
PRONOTUM_X = 50  # the pronotum seam bows forward from (50, top) to (53, 36)
SPIKES = [(19, -1), (27, -1), (35, -1), (43, -1)]  # one serrated thorn at the top of each seam
CRUST = [(23, 29), (24, 29), (24, 30), (31, 27), (32, 27), (32, 28), (40, 31), (41, 31), (56, 29), (57, 29)]
PITS = [(25, 30), (33, 28), (42, 31)]  # dark pocks next to the crust patches (`a`)


def _band(t: float) -> str:
    """Shell band by relative depth t in [0, 1] from the dome top to the belly (A1 torch from above)."""
    if t < 0.13:
        return "f"
    if t < 0.32:
        return "d"
    if t < 0.62:
        return "c"
    if t < 0.84:
        return "b"
    return "a"


def shell() -> dict:
    """Outlined dome, plate seams, pronotum shield, crust, cold rim. Returns {(x, y): key}."""
    px = {}
    x0, x1 = SHELL_TOP[0][0], SHELL_TOP[-1][0]
    for x in range(x0, x1 + 1):
        top, bot = _interp(SHELL_TOP, x), _interp(SHELL_BOT, x)
        for y in range(top, bot + 1):
            if y in (top, bot):
                px[(x, y)] = "K"
            else:
                px[(x, y)] = _band((y - top) / max(1, bot - top))
    # torch sheen: the row under the top outline on the highest plates and the pronotum crown
    for x in range(x0, x1 + 1):
        top = _interp(SHELL_TOP, x)
        if 22 <= x <= 49 or 52 <= x <= 60:
            px[(x, top + 1)] = "h"
    # abdominal seams (`a`), slanting back as they go down; the plate edge in front of each seam is lit
    for sx, slope in SEAMS:
        top = _interp(SHELL_TOP, sx)
        for y in range(top + 1, 37):
            x = sx - (y - top) // slope
            px[(x, y)] = "a"
            if top + 2 <= y <= top + 3 and px.get((x + 1, y), "K") != "K":
                px[(x + 1, y)] = "f"
    # pronotum seam (`K`), bowing forward: separates the shield from the abdomen
    for y in range(_interp(SHELL_TOP, PRONOTUM_X), 37):
        x = PRONOTUM_X + (1 if y >= 28 else 0) + (1 if y >= 31 else 0) + (1 if y >= 34 else 0)
        px[(x, y)] = "K"
    # pronotum shield: brighter lit crown (`f`) over 3 rows, the lip shadowed (`b`) where it overhangs the head
    for x in range(PRONOTUM_X + 2, x1):
        top = _interp(SHELL_TOP, x)
        for y in (top + 2, top + 3):
            if px.get((x, y)) not in (None, "K"):
                px[(x, y)] = "f"
    for x, y in CRUST:
        px[(x, y)] = "y"
    for x, y in PITS:
        px[(x, y)] = "a"
    # cold rim (`v`) along the rear underside, belly shade (`u`) above the belly line
    for x in range(14, 30):
        px[(x, 36)] = "v"
    for x in range(30, 50):
        px[(x, 36)] = "u"
    # serrated thorns on the dome: a 2 px `K` hook leaning back at each seam top
    for sx, _ in SPIKES:
        top = _interp(SHELL_TOP, sx)
        px[(sx, top - 1)] = "K"
        px[(sx - 1, top - 2)] = "K"
    for (x, y), c in list(px.items()):  # seal: an empty pixel next to shell fill becomes outline `K`
        if c != "K":
            for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                px.setdefault(q, "K")
    return px


def ootheca() -> dict:
    """The egg case carried at the abdomen tip: a ridged capsule, x 4..11, rows 31..37."""
    rows = {
        31: (6, "KKKK"),
        32: (5, "KssssK"),
        33: (4, "KooooooK"),
        34: (4, "KogogogK"),
        35: (4, "KogogogK"),
        36: (5, "KooooK"),
        37: (6, "KKKK"),
    }
    return {(x0 + i, y): c for y, (x0, s) in rows.items() for i, c in enumerate(s)}


def head() -> dict:
    """Lowered head under the pronotum lip: x 59..69, rows 30..38; 3 x 3 red eye under a heavy `K` brow;
    two cream fangs (near x 68..72, far x 63..66) hooking down to row 42."""
    rows = {
        30: (61, "KKKK"),
        31: (60, "KcKKKKK"),
        32: (59, "KccKrrHKK"),
        33: (59, "KcccerrddK"),
        34: (59, "KbccceedddK"),
        35: (59, "KbbcccccdhK"),
        36: (59, "KabbccccdhK"),
        37: (60, "KabbcccdKhK"),
        38: (61, "KabbbKKKhHK"),
        39: (62, "KKKhKK.KhHK"),
        40: (63, "KhHK..KhHK"),
        41: (64, "KHK..KHHK"),
        42: (64, "KHK.KHK"),
    }
    px = {(x0 + i, y): c for y, (x0, s) in rows.items() for i, c in enumerate(s) if c != "."}
    return px


def antennae(cv: Cv) -> None:
    """Near antenna `j` up-forward from the head to the apex (row 14) and a drooping tip; far antenna `d`
    up and back over the pronotum (apex row 16)."""
    cv.poly([(56, 27), (55, 24), (53, 21), (51, 19), (48, 17), (45, 16), (42, 16), (40, 17)], "d")
    cv.poly([(65, 30), (66, 28), (67, 25), (68, 22), (69, 19), (70, 17), (71, 15), (73, 14), (75, 14),
             (77, 15), (78, 17), (78, 19)], "j")
    cv.put(65, 29, "j")
    for p in [(67, 24), (69, 18), (74, 14)]:  # segment beads, lighter
        cv.put(*p, "s")


def _leg(cv, coxa, knee, foot, back: bool, near: bool, spines=()):
    """Near legs: 2 px femur (`j` over `g`), tibia `j` with a `g` back edge, `s` spines, `g` tarsus.
    Far legs: 1 px `a`. Every foot ends in a 2 px tarsus on the floor row."""
    if near:
        cv.line(coxa, knee, "j")
        cv.line((coxa[0], coxa[1] + 1), (knee[0], knee[1] + 1), "g")
        d = 1 if back else -1  # the shaded back edge of the tibia, full length above the tarsus
        cv.line((knee[0] + d, knee[1] + 1), (foot[0] + d, foot[1] - 1), "g")
        cv.line(knee, foot, "j")
        cv.put(foot[0] + (-1 if back else 1), foot[1], "g")
        cv.put(*foot, "g")
        for p in spines:
            cv.put(*p, "s")
    else:
        cv.line(coxa, knee, "a")
        cv.line(knee, foot, "a")
        cv.put(foot[0] + (-1 if back else 1), foot[1], "a")


def far_legs(cv: Cv) -> None:
    _leg(cv, (27, 37), (21, 36), (17, 43), True, False)
    _leg(cv, (41, 37), (45, 38), (47, 43), False, False)
    _leg(cv, (56, 37), (60, 39), (63, 43), False, False)


def near_legs(cv: Cv) -> None:
    # rear: femur back and up along the belly to a raised knee, tibia down-back to the foot
    _leg(cv, (24, 37), (13, 34), (4, 43), True, True, spines=[(12, 33), (9, 37), (6, 40)])
    # mid: femur down-back, tibia down to the foot
    _leg(cv, (37, 37), (32, 39), (27, 43), True, True, spines=[(30, 40), (28, 41)])
    # front: femur down-forward, tibia forward to the foot under the head
    _leg(cv, (52, 37), (55, 40), (58, 43), False, True, spines=[(56, 39), (58, 41)])


def frame_idle0() -> list[str]:
    """Key 0, v2 (design round 2, after the FAIL-DESIGN review D1 to D3): the frozen hero key."""
    return k2_idle0()


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
    for p in [(33, 22), (57, 21)]:
        px[p] = "a"
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


def _mandible(spine, widths, lit="h", edge="H") -> dict:
    """A thick hooked mandible along `spine` (centre points), `widths` px wide (grown downward);
    body `lit`, front/top edge `edge`, outlined `K`."""
    px = {}
    for (x, y), w in zip(spine, widths):
        for k in range(w):
            px[(x, y + k)] = lit
        px[(x, y)] = edge
    return _seal(px, lit + edge)


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


def k2_mandibles() -> tuple[dict, dict]:
    """Near mandible: thick at the mouth, curving down-forward, the tip hooking back (Carl's shin height).
    Far mandible: the same hook 4 px behind, darker (`j` body, `h` edge)."""
    near = _hook([(70, 32), (72, 33), (73, 35), (74, 37), (74, 39), (73, 40), (71, 41)], "h", "H", 4)
    far = _hook([(66, 34), (67, 36), (68, 38), (67, 40), (65, 41)], "j", "h", 2)
    return near, far


def k2_leg(cv: Cv, coxa, knee, foot, near: bool, spines=()) -> None:
    """2 px femur and tibia, outlined `K`; near legs in the chitin ramp (`d` lit top, `b` under),
    far legs `b`/`a`; `K` tarsus on the floor; `s` spine ticks outside the outline."""
    top, low = ("d", "b") if near else ("b", "a")
    t = Cv()
    t.line(coxa, knee, top)
    t.line((coxa[0], coxa[1] + 1), (knee[0], knee[1] + 1), low)
    mid = ((knee[0] + foot[0]) // 2, (knee[1] + foot[1]) // 2)
    t.line(knee, foot, top)
    t.line((knee[0] + 1, knee[1]), (mid[0] + 1, mid[1]), low)
    px = _seal(t.p, top + low)
    px = {p: c for p, c in px.items() if p[1] <= FLOOR}
    for dx in (-1, 0, 1):
        px[(foot[0] + dx, FLOOR)] = "K"
    for p, c in px.items():
        cv.put(*p, c)
    for p in spines:
        cv.put(*p, "s")


def k2_idle0() -> list[str]:
    cv = Cv()
    # far legs (behind the body)
    k2_leg(cv, (28, 33), (24, 37), (21, 42), False)
    k2_leg(cv, (42, 33), (45, 37), (46, 42), False)
    k2_leg(cv, (54, 33), (60, 37), (65, 42), False)
    # far antenna, up and back over the pronotum
    cv.poly([(61, 23), (60, 20), (58, 17), (56, 14), (53, 12), (50, 11), (47, 12)], "d")
    far_m = k2_mandibles()[1]
    for p, c in far_m.items():
        cv.put(*p, c)
    for p, c in k2_shell().items():
        cv.put(*p, c)
    for p, c in k2_head().items():
        cv.put(*p, c)
    near_m = k2_mandibles()[0]
    for p, c in near_m.items():
        cv.put(*p, c)
    # near antenna: 2 px root, then up-forward to the apex (row 10) and a drooping tip
    cv.poly([(68, 23), (69, 20), (70, 17), (71, 14), (72, 12), (74, 10), (76, 10), (78, 12), (78, 14)], "j")
    cv.poly([(67, 23), (68, 20)], "j")
    for p in [(69, 19), (72, 12), (77, 11)]:
        cv.put(*p, "s")
    # near legs (in front): rear knee raised high and back, mid under the body, front braced under the head
    k2_leg(cv, (25, 33), (15, 36), (7, 42), True, spines=[(14, 34), (10, 37), (7, 40)])
    k2_leg(cv, (38, 33), (33, 37), (29, 42), True, spines=[(31, 36), (29, 39)])
    k2_leg(cv, (50, 33), (55, 37), (58, 42), True, spines=[(56, 35), (60, 39)])
    return cv.rows()


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


def frame_idle0_v1() -> list[str]:
    """Round-1 key 0 (failed review). Kept only because the pre-gate frames 1 to 15 are composed from its parts."""
    cv = Cv()
    far_legs(cv)
    antennae(cv)
    for p, c in ootheca().items():
        cv.put(*p, c)
    for p, c in shell().items():
        cv.put(*p, c)
    for p, c in head().items():
        cv.put(*p, c)
    near_legs(cv)
    return cv.rows()


# ---------------------------------------------------------------------------
# Execution tier: keys 3, 8, 10, 12, 15 and the 10 in-betweens (spec sections 7 and 8).
# Frame 0 above is frozen. The helpers below split its parts, translate them whole
# (never resample or rotate), re-seal the outline and redraw legs, antennae and fangs.
# ``assemble()`` with its defaults reproduces key 0 pixel for pixel (checked by the tests).
# ---------------------------------------------------------------------------
SHELL0, EGG0, HEAD0 = shell(), ootheca(), head()
NEAR_FANG0 = {p: c for p, c in HEAD0.items() if (p[0] >= 69 and p[1] >= 37) or (p[0] >= 68 and p[1] >= 40)}
FAR_FANG0 = {p: c for p, c in HEAD0.items() if p[0] <= 66 and p[1] >= 40}
HEAD_CORE0 = {p: c for p, c in HEAD0.items() if p not in NEAR_FANG0 and p not in FAR_FANG0}
EYE0 = [(63, 32), (64, 32), (65, 32), (63, 33), (64, 33), (65, 33), (64, 34), (65, 34)]
ANT_NEAR0 = [(65, 30), (66, 28), (67, 25), (68, 22), (69, 19), (70, 17), (71, 15), (73, 14), (75, 14), (77, 15),
             (78, 17), (78, 19)]
ANT_FAR0 = [(56, 27), (55, 24), (53, 21), (51, 19), (48, 17), (45, 16), (42, 16), (40, 17)]
BEADS0 = [(67, 24), (69, 18), (74, 14)]
# Leg specs: (coxa, knee, foot anchor, back, spines). The anchor is the tarsus pixel next to the leg line;
# the 2 px tarsus extends back (back=True) or forward from it. spines None = computed along the tibia.
NEAR0 = {"rear": ((24, 37), (13, 34), (4, 43), True, [(12, 33), (9, 37), (6, 40)]),
         "mid": ((37, 37), (32, 39), (27, 43), True, [(30, 40), (28, 41)]),
         "front": ((52, 37), (55, 40), (58, 43), False, [(56, 39), (58, 41)])}
FAR0 = {"rear": ((27, 37), (21, 36), (17, 43), True, None),
        "mid": ((41, 37), (45, 38), (47, 43), False, None),
        "front": ((56, 37), (60, 39), (63, 43), False, None)}


def mv(px, dx=0, dy=0, dyf=None):
    """Translate a part whole; dyf(original x) gives a per-column dy."""
    return {(x + dx, y + (dyf(x) if dyf else dy)): c for (x, y), c in px.items()}


def seal_px(px):
    """Close outline gaps: an empty pixel next to a non-outline pixel becomes outline K."""
    out = dict(px)
    for (x, y), c in px.items():
        if c != "K":
            for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if q not in px and 0 <= q[0] < W and 0 <= q[1] < H:
                    out[q] = "K"
    return out


def drop_row(px, row):
    """Squash by one row: delete `row`; the rows above it move down 1."""
    return {(x, y + 1 if y < row else y): c for (x, y), c in px.items() if y != row}


def body(dx=0, dyf=None, shell_px=None):
    out = {}
    for src in (EGG0, shell_px or SHELL0):
        out.update(mv(src, dx, dyf=dyf or (lambda x: 0)))
    return seal_px(out)


def head_core(dx=0, dy=0, eye=None, max_row=39):
    core = {p: c for p, c in HEAD_CORE0.items() if p[1] <= max_row}
    for p, c in (eye or {}).items():
        assert p in core, p
        core[p] = c
    return mv(core, dx, dy)


def shear(px, dx=0, dy=0, rows_from=0, tip_dx=0):
    """Translate a fang whole; original rows >= rows_from get an extra tip_dx; rows below the canvas are cut."""
    return {(x + dx + (tip_dx if y >= rows_from else 0), y + dy): c for (x, y), c in px.items() if y + dy < H}


def key0_fangs(dx=0, dy=0, near_dx=0, far_dx=0, rows_from=41):
    out = shear(FAR_FANG0, dx, dy, rows_from, far_dx)
    out.update(shear(NEAR_FANG0, dx, dy, rows_from, near_dx))
    return out


def shift_pts(pts, start, dx=0, dy=0, end=99):
    return [(x + dx, y + dy) if start <= i < end else (x, y) for i, (x, y) in enumerate(pts)]


def draw_ants(cv, near, far, beads=None):
    cv.poly(far, "d")
    cv.poly(near, "j")
    cv.put(near[0][0], near[0][1] - 1, "j")
    for p in beads or [near[2], near[-5], near[-4]]:
        cv.put(*p, "s")


def auto_spines(knee, foot):
    return [(round(knee[0] + t * (foot[0] - knee[0])), round(knee[1] + t * (foot[1] - knee[1])) - 1)
            for t in (0.4, 0.75)]


def tip(lo, row, back):
    """Foot anchor for a 2 px tarsus covering x lo..lo+1."""
    return (lo + 1, row) if back else (lo, row)


def put_legs(cv, specs, near):
    for coxa, knee, foot, back, spines in specs.values():
        _leg(cv, coxa, knee, foot, back, near, spines if spines is not None else (auto_spines(knee, foot) if near else ()))


def shifted(specs, dx, dy):
    return {k: ((c[0] + dx, c[1] + dy), (kn[0] + dx, kn[1] + dy), (f[0] + dx, f[1] + dy), b,
                [(sx + dx, sy + dy) for sx, sy in sp] if sp else sp) for k, (c, kn, f, b, sp) in specs.items()}


def put_outlined(cv, px):
    """Draw fang pixels, then wrap their empty neighbours in outline K."""
    for p, c in px.items():
        cv.put(*p, c)
    for (x, y), c in px.items():
        if c != "K":
            for q in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if q not in cv.p and 0 <= q[0] < W and 0 <= q[1] < H:
                    cv.put(*q, "K")


def v_fangs(fx, ru, rl, full=True):
    """Spread fangs in a V from the head front column fx: upper arm from row ru, lower arm from row rl."""
    if full:
        up_h = [(1, 0), (2, -1), (3, -2), (4, -3)]
        up_e = [(1, -1), (2, -2), (3, -3), (4, -4), (5, -3), (5, -2)]
        lo_h = [(1, 0), (2, 1), (3, 2), (4, 3)]
        lo_e = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 3), (5, 2)]
    elif full is None:  # short: the dash frame keeps every fang pixel left of x 76 (only the impact frame reaches it)
        up_h = [(1, 0), (2, -1)]
        up_e = [(1, -1), (2, -2), (3, -2)]
        lo_h = [(1, 0), (2, 1)]
        lo_e = [(1, 1), (2, 2), (3, 2)]
    else:
        up_h = [(1, 0), (2, -1), (3, -1)]
        up_e = [(1, -1), (2, -2), (3, -2), (4, -2)]
        lo_h = [(1, 0), (2, 1), (3, 1)]
        lo_e = [(1, 1), (2, 2), (3, 2), (4, 2)]
    px = {}
    for r, hs, es in ((ru, up_h, up_e), (rl, lo_h, lo_e)):
        for dx, dy in hs:
            px[(fx + dx, r + dy)] = "h"
        for dx, dy in es:
            px[(fx + dx, r + dy)] = "H"
    return px


def assemble(far, ants, bod, hd, fangs, near, rear_behind=False, post=None, plain=False):
    """z-order of key 0: far legs, antennae, egg case + shell, head, fangs, near legs (rear leg behind the
    egg case when rear_behind)."""
    cv = Cv()
    put_legs(cv, far, False)
    draw_ants(cv, *ants)
    if rear_behind:
        put_legs(cv, {"rear": near["rear"]}, True)
    cv.putm(bod)
    if post:
        post(cv)
    cv.putm(hd)
    if fangs:
        cv.putm(fangs) if plain else put_outlined(cv, fangs)
    put_legs(cv, {k: v for k, v in near.items() if not (rear_behind and k == "rear")}, True)
    return cv


# ---- idle in-betweens ------------------------------------------------------
def frame_1():
    ants = (shift_pts(ANT_NEAR0, 5, 1, -1), shift_pts(ANT_FAR0, 4, -1, 0), [(67, 24), (70, 17), (75, 13)])
    return assemble(FAR0, ants, body(), head_core(), key0_fangs(near_dx=1), NEAR0, plain=True).rows()


def frame_2():
    moved = {(x, y): c for (x, y), c in SHELL0.items() if not (20 <= y <= 28 and 12 <= x <= 50)}
    for (x, y), c in SHELL0.items():  # the dome slice drops 1 and wins over the rows it lands on
        if 20 <= y <= 28 and 12 <= x <= 50:
            moved[(x, y + 1)] = c
    ants = (shift_pts(ANT_NEAR0, 6, 0, 1), ANT_FAR0, [(67, 24), (69, 18), (74, 15)])
    return assemble(FAR0, ants, body(shell_px=moved), head_core(), key0_fangs(), NEAR0, plain=True).rows()


# ---- walk -----------------------------------------------------------------
def walk(n):
    bx, by = 1, (-1 if n in (4, 6) else 0)
    a_down = n in (3, 4)  # tripod A (near rear, near front, far mid) planted in 3 and 4; tripod B in 5 and 6
    lift = {3: 41, 4: 40, 5: 41, 6: 40}[n]
    A = {"nrear": (1, 1), "nfront": (63, 61), "fmid": (43, 41)}  # (planted lo, lifted lo)
    B = {"nmid": (36, 34), "frear": (24, 22), "ffront": (69, 67)}
    la = {k: (v[0] if a_down else v[1] + (1 if n == 6 else 0)) for k, v in A.items()}
    lb = {k: (v[1] + (1 if n == 4 else 0) if a_down else v[0]) for k, v in B.items()}
    ra, rb = (43, lift) if a_down else (lift, 43)
    near = {
        "rear": ((24 + bx, 37 + by), (12, (34 if a_down else 33) + by), tip(la["nrear"], ra, True), True, None),
        "mid": ((37 + bx, 37 + by), ((36 if a_down else 39), (38 if n == 4 else 39) + by),
                tip(lb["nmid"], rb, True), True, None),
        "front": ((52 + bx, 37 + by), (58, (40 if a_down else 39) + by), tip(la["nfront"], ra, False), False, None),
    }

    def fk(cx, lo, fwd):
        return ((cx + lo) // 2 + (1 if fwd else 0), 38 + by)

    far = {"rear": ((27 + bx, 37 + by), fk(27 + bx, lb["frear"], False), tip(lb["frear"], rb, True), True, None),
           "mid": ((41 + bx, 37 + by), fk(41 + bx, la["fmid"], True), tip(la["fmid"], ra, True), True, None),
           "front": ((56 + bx, 37 + by), fk(56 + bx, lb["ffront"], True), tip(lb["ffront"], rb, False), False, None)}
    ex = (1, 0) if n in (4, 6) else (1, 1)
    near_ant = shift_pts([(x + bx, y + by) for x, y in ANT_NEAR0], 5, *ex, end=10)
    far_ant = [(x + bx, y + by) for x, y in ANT_FAR0]
    beads = [(67 + bx, 24 + by), (70 + bx, 18 + by), (74 + bx + ex[0], 14 + by + ex[1])]
    return assemble(far, (near_ant, far_ant, beads), body(bx, lambda x: by), head_core(bx, by),
                    key0_fangs(bx, by), near, rear_behind=True, plain=True).rows()


# ---- tell -----------------------------------------------------------------
def frame_7():
    dyf = lambda x: 1 if x <= 18 else (0 if x <= 30 else (-1 if x <= 42 else (-2 if x <= 50 else -4)))
    near = {"rear": ((23, 37), (12, 35), tip(1, 43, True), True, None),
            "mid": ((36, 36), (31, 39), tip(25, 43, True), True, None),
            "front": ((51, 33), (55, 36), tip(58, 43, False), False, None)}
    far = {"rear": ((26, 37), (20, 36), tip(15, 43, True), True, None),
           "mid": ((40, 36), (44, 38), tip(46, 43, False), False, None),
           "front": ((55, 33), (60, 37), tip(63, 43, False), False, None)}
    ants = ([(63, 26), (64, 23), (65, 19), (66, 16), (67, 13), (68, 11), (70, 10), (72, 10), (74, 12), (75, 14)],
            [(55, 23), (52, 19), (49, 16), (46, 13), (44, 12), (42, 12), (40, 13)])
    return assemble(far, ants, body(-1, dyf), head_core(-1, -4, {(63, 33): "r"}), v_fangs(68, 31, 34, False),
                    near, rear_behind=True).rows()


def frame_8():
    dyf = lambda x: 2 if x <= 18 else (0 if x <= 30 else (-2 if x <= 42 else (-4 if x <= 50 else -7)))
    bod = body(-2, dyf)

    def post(cv):  # cold belly reveal under the raised belly line
        for x in range(30, 59):
            ys = [y for (xx, y) in bod if xx == x]
            if ys and (x, max(ys) + 1) not in cv.p:
                cv.put(x, max(ys) + 1, "u")

    near = {"rear": ((22, 37), (11, 35), tip(1, 43, True), True, None),
            "mid": ((35, 35), (30, 38), tip(24, 43, True), True, None),
            "front": ((48, 30), (55, 27), (60, 31), False, [(57, 26), (59, 29)])}
    far = {"rear": ((25, 37), (19, 36), tip(14, 43, True), True, None),
           "mid": ((39, 35), (43, 38), tip(45, 43, False), False, None),
           "front": ((48, 32), (55, 29), (60, 33), False, None)}
    ants = ([(62, 22), (63, 19), (64, 16), (66, 12), (68, 9), (69, 6), (71, 6), (73, 7), (75, 9)],
            [(52, 20), (50, 16), (48, 12), (46, 9), (45, 8), (44, 8), (42, 9), (41, 10)],
            [(64, 16), (68, 9), (72, 6)])
    hd = head_core(-2, -8, {(63, 33): "r", (64, 34): "r"})
    return assemble(far, ants, bod, hd, v_fangs(67, 27, 30, True), near, rear_behind=True, post=post).rows()


# ---- attack ---------------------------------------------------------------
def frame_9():
    near = {"rear": ((27, 37), (18, 39), tip(9, 43, True), True, None),
            "mid": ((40, 37), (35, 39), tip(29, 43, True), True, None),
            "front": ((55, 37), (61, 40), tip(66, 43, False), False, None)}
    far = {"rear": ((30, 37), (24, 38), tip(19, 43, True), True, None),
           "mid": ((44, 37), (47, 39), tip(48, 43, True), True, None),
           "front": ((59, 37), (66, 40), tip(70, 43, False), False, None)}
    ants = ([(68, 30), (68, 27), (67, 24), (66, 21), (64, 19), (61, 18), (58, 18), (56, 19)],
            [(59, 27), (58, 24), (56, 21), (53, 19), (50, 18), (47, 18), (44, 19)])
    return assemble(far, ants, body(3), head_core(3, 0), v_fangs(72, 35, 38, None), near).rows()


def frame_10():
    bod = body(6, None, drop_row(SHELL0, 26))  # flattened: band row 26 deleted, the rows above drop 1
    near = {"rear": ((30, 37), (19, 39), tip(6, 43, True), True, None),
            "mid": ((43, 37), (36, 39), tip(28, 43, True), True, None),
            "front": ((58, 37), (65, 40), tip(70, 43, False), False, None)}
    far = {"rear": ((33, 37), (27, 38), tip(20, 43, True), True, None),
           "mid": ((47, 37), (49, 39), tip(49, 43, True), True, None),
           "front": ((62, 37), (69, 40), tip(72, 43, False), False, None)}
    ants = ([(72, 30), (70, 27), (67, 24), (64, 22), (60, 21), (56, 21), (53, 22), (52, 23)],
            [(66, 28), (65, 27), (64, 26), (63, 25)])
    # fangs pinched shut: near fang from the cheek, far fang from the lower jaw; the tips meet at x 78..79
    fangs = {(77, 38): "h", (77, 39): "h", (78, 39): "H", (78, 40): "H", (79, 40): "H", (79, 39): "H",
             (75, 41): "h", (76, 41): "H", (77, 41): "H", (77, 40): "H"}
    return assemble(far, ants, bod, head_core(8, 1), fangs, near).rows()


def frame_11():
    near_ant = [(x + 3 - (round(3 * min(1, (30 - y) / 16)) if y < 30 else 0), y) for x, y in ANT_NEAR0]
    far_ant = [(x + 3 - (round(3 * min(1, (27 - y) / 11)) if y < 27 else 0), y) for x, y in ANT_FAR0]
    ants = (shift_pts(near_ant, 5, 1, -1, end=10), shift_pts(far_ant, 4, -1, 0), [(70, 24), (71, 17), (75, 13)])
    return assemble(shifted(FAR0, 3, 0), ants, body(3), head_core(3, 0), key0_fangs(3, 0, near_dx=1),
                    shifted(NEAR0, 3, 0), plain=True).rows()


# ---- hurt -----------------------------------------------------------------
def frame_12():
    dyf = lambda x: 0 if x <= 30 else (-1 if x <= 50 else -2)
    eye = {p: "w" for p in EYE0}
    eye.update({(63, 33): "K", (65, 32): "H"})
    near = {"rear": ((21, 37), (10, 34), tip(0, 43, True), True, None),
            "mid": ((34, 36), (29, 38), tip(23, 43, True), True, None),
            "front": ((49, 35), (52, 38), (56, 41), False, None)}
    far = {"rear": ((24, 37), (18, 36), tip(13, 43, True), True, None),
           "mid": ((38, 36), (42, 38), tip(44, 43, False), False, None),
           "front": ((53, 35), (57, 38), tip(60, 43, False), False, None)}
    ants = ([(62, 28), (63, 25), (64, 22), (65, 19), (66, 16), (65, 16), (64, 16), (62, 17), (60, 19)],
            [(53, 25), (50, 22), (46, 20), (42, 19), (38, 19), (34, 19)])
    return assemble(far, ants, body(-3, dyf), head_core(-3, -2, eye), key0_fangs(-3, -2, near_dx=1, far_dx=-1),
                    near, rear_behind=True, plain=True).rows()


# ---- defeat ---------------------------------------------------------------
def frame_13():
    eye = {p: "a" for p in EYE0}
    eye.update({(63, 33): "e", (64, 33): "e", (64, 34): "e", (65, 34): "e"})
    near = {"rear": ((24, 40), (11, 36), tip(3, 43, True), True, None),
            "mid": ((37, 40), (30, 41), tip(26, 43, True), True, None),
            "front": ((52, 40), (57, 42), tip(58, 43, False), False, None)}
    far = {"rear": ((27, 40), (22, 40), tip(16, 43, True), True, None),
           "mid": ((41, 40), (46, 41), tip(47, 43, False), False, None),
           "front": ((56, 40), (62, 42), tip(63, 43, False), False, None)}
    ants = ([(65, 33), (66, 30), (68, 26), (70, 22), (72, 19), (74, 18), (76, 19), (77, 22), (77, 25)],
            [(56, 30), (54, 27), (51, 24), (48, 22), (45, 21), (42, 22), (40, 24)])
    return assemble(far, ants, body(0, lambda x: 3), head_core(0, 3, eye), key0_fangs(0, 3), near,
                    rear_behind=True, plain=True).rows()


def frame_15(squash=False, ant_near=None, eye_e=()):
    """Settled defeat (also frame 14 with `squash`): the Guardian lies belly-down, legs flat, eye dark."""
    sh = drop_row(SHELL0, 25) if squash else SHELL0  # frame 14: band row 30 of the dropped shell is deleted
    eye = {p: "a" for p in EYE0}
    for p in eye_e:
        eye[p] = "e"
    if not eye_e:
        eye[(64, 33)] = "K"
    near = {"rear": ((24, 42), (12, 42), tip(0, 43, True), True, None),
            "mid": ((37, 42), (31, 42), tip(26, 43, True), True, None),
            "front": ((52, 42), (57, 42), tip(62, 43, False), False, None)}
    cv = Cv()
    for p in ((29, 43), (30, 43), (45, 43), (46, 43)):  # far legs show only their tips
        cv.put(*p, "a")
    ants_far = [(48, 27), (50, 25), (53, 25), (56, 26), (58, 28), (60, 30)]  # bent over the pronotum, ends row 30
    ants_near = ant_near or [(70, 39), (71, 40), (72, 41), (73, 41), (75, 42), (76, 42), (77, 43)]  # lies down
    draw_ants(cv, ants_near, ants_far, None if ant_near else [(72, 41), (76, 42)])
    put_legs(cv, {"rear": near["rear"]}, True)
    cv.putm(body(0, lambda x: 5, sh))
    cv.putm(head_core(0, 5, eye, max_row=38))
    put_outlined(cv, {(69, 41): "h", (69, 42): "h", (70, 42): "H", (70, 43): "H", (66, 43): "H", (65, 43): "h"})
    put_legs(cv, {k: v for k, v in near.items() if k != "rear"}, True)
    return cv.rows()


def frame_14():
    return frame_15(True, [(66, 35), (68, 31), (71, 29), (74, 29), (76, 31), (77, 34), (77, 37)],
                    ((64, 34), (65, 34)))


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


AUTHORS = {0: "key (Claude Opus, frozen)", 3: "key (Claude Sonnet, spec section 7)",
           8: "key (Claude Sonnet, spec section 7)", 10: "key (Claude Sonnet, spec section 7)",
           12: "key (Claude Sonnet, spec section 7)", 15: "key (Claude Sonnet, spec section 7)"}
BOXES = {  # frame pixels (x, y, w, h); pivot-relative values are in spec section 3
    "standing": [13, 24, 50, 20],
    "hurtbox": [4, 22, 66, 18],
    "hurtbox_tell_hold": [2, 14, 66, 28],
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
            "opaque_height_idle": 30,
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
