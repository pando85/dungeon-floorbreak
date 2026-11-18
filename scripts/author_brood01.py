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


def all_frames() -> list[list[str]]:
    key0 = frame_idle0()
    # PLACEHOLDER: frames 1..15 are copies of key 0 until the execution tier draws them.
    return [list(key0) for _ in TICKS]


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


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    grid = build()
    GRID.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
