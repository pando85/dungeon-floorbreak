#!/usr/bin/env python3
"""GOB-02-DEATH authoring record: goblin A death poses and the death-parts sheet (pixel-grid-v1 sources).

Usage:

    python3 scripts/author_gob02_death.py keys [--check]
    python3 scripts/author_gob02_death.py scale

``keys`` (re)writes the two sources from the rules and literal rows below (see ``docs/02-art/gob-02-death-spec.md``):

* ``assets/source/characters/goblin/death/goblin_a_death_h36.grid.json``: 6 frames of 64 x 56, pivot (28, 55), the GOB-01
  frame contract. Keys 0 ``crush_contact``, 2 ``crushed_remains``, 3 ``decap_contact`` and 5 ``headless_collapse`` (the
  hold). Frames 1 and 4 are **placeholders** (copies of keys 0 and 3) that the executor replaces with the in-betweens of
  spec section 9.
* ``assets/source/characters/goblin/death/goblin_a_death_parts.grid.json``: 32 x 32 cells, one part per frame. Keys 0
  ``head``, 1 ``torso_upper`` and 2 ``club``. The executor appends frames 3 onward (spec section 7) and the manifest.

``scale`` writes ``docs/evidence/gob-02-death/silhouette_scale_1x.png``: the four poses and the three key parts at 1x on
the lit Floor 1 brick next to Carl (52 px) and Donut (40 px), a 4x band of the same image, and
``keys_x4_preset4_vs_preset1.png`` (every key with and without the ``gore`` layer).

Construction (no concept pixels and no Death Lab pixels traced or sampled; no image-model output):

* ``decap_contact`` is GOB-01 A frame 16 (``hurt`` recoil) with the head removed above the collar, a new neck stump and a
  new gore layer;
* ``headless_collapse`` is GOB-01 A frame 25 (``corpse``) with the head columns removed, a new stump and a gore layer;
* ``crush_contact`` is new literal rows; ``crushed_remains`` is GOB-01 A frame 25's corpse head plus a new pressed body
  in literal rows;
* the ``head`` and ``torso_upper`` parts are the GOB-01 A frame 0 pixels of that part, outline-closed, with new cut
  surfaces on the gore layer; the ``club`` is frame 0's pineapple on a new straight shaft.

Layer rule (the preset contract): ``body`` and ``weapon`` alone are always closed, outlined, bloodless sprites; every
wound, blood and bone pixel is on ``gore``, the top layer. Preset 1 hides ``gore`` and preset 2 remaps it darker
(spec section 5); nothing is redrawn per preset.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEATH_DIR = ROOT / "assets/source/characters/goblin/death"
POSES = DEATH_DIR / "goblin_a_death_h36.grid.json"
POSES_ASE = DEATH_DIR / "goblin_a_death_h36.ase"
PARTS = DEATH_DIR / "goblin_a_death_parts.grid.json"
PARTS_ASE = DEATH_DIR / "goblin_a_death_parts.ase"
GOBLIN_A = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
SCALE_SHEET = ROOT / "docs/evidence/gob-02-death/silhouette_scale_1x.png"
CANVAS = (64, 56)
PIVOT = (28, 55)
CELL = (32, 32)
CELL_PIVOT = (16, 16)
LAYERS = ["body", "weapon", "gore"]
TICK_HZ = 60
HOLD_MS = 100

# goblin-death-floor01-v0 (20 of 20). The first 19 are goblin-floor01-v0 keys with the same hex, minus ``e`` (no living
# eye in this atlas: shocked or dead eyes are ``w`` and ``K``), plus ``B`` (variant B bandana shadow, reused as clotted
# blood) and one new key ``P`` (wet blood highlight). Blood ramp, dark to light: o, B, R, P. Bone and gristle: w.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "rag_dark", "#2c1e28ff", False),
    ("t", "rag_mid", "#4c3436ff", False),
    ("u", "rag_light", "#73524aff", False),
    ("w", "tooth_white", "#efe4c4ff", True),      # teeth, eye whites; bone ends on gore
    ("o", "mouth_dark", "#5c1426ff", False),      # mouth; blood rim, old stain
    ("R", "red_mid", "#b8322eff", False),         # tongue; fresh blood, meat
    ("x", "wood_dark", "#4a2c1cff", False),
    ("y", "wood_mid", "#8c5c32ff", False),
    ("F", "pineapple_dark", "#7a3c12ff", False),
    ("G", "pineapple_mid", "#d08826ff", False),
    ("H", "pineapple_light", "#f4c84aff", True),
    ("V", "leaf_dark", "#1d4a3cff", False),
    ("v", "leaf_mid", "#3e8a5cff", False),
    ("B", "blood_dark", "#6e1a22ff", False),      # = goblin-floor01-v0 rag_red_dark, same hex
    ("P", "blood_wet", "#e2434bff", True),        # new: wet highlight, gore layer only
]
KEYS = [p[0] for p in PALETTE]
GORE_ONLY = set("BP")                    # keys that may appear on the gore layer only
WEAPON_KEYS = set("xyFGHVv")

Pix = dict  # {(x, y): key}


# --------------------------------------------------------------------------- helpers
def load_goblin() -> dict:
    return json.loads(GOBLIN_A.read_text(encoding="utf-8"))


def cel_pix(rows: list[str]) -> Pix:
    return {(x, y): ch for y, row in enumerate(rows) for x, ch in enumerate(row) if ch != "."}


def literal(rows: dict[int, str], x0: int = 0) -> Pix:
    """Literal rows ``{y: "..K.."}`` starting at column ``x0``; ``.`` is transparent."""
    out: Pix = {}
    for y, row in rows.items():
        for i, ch in enumerate(row):
            if ch == ".":
                continue
            if ch not in KEYS:
                raise ValueError(f"unknown key {ch!r} at ({x0 + i}, {y})")
            out[(x0 + i, y)] = ch
    return out


def shift(pix: Pix, dx: int, dy: int) -> Pix:
    return {(x + dx, y + dy): ch for (x, y), ch in pix.items()}


def close_outline(pix: Pix, others: Pix | None = None, size: tuple[int, int] = CANVAS) -> Pix:
    """Every empty 4-neighbour (inside the canvas) of a non-outline pixel becomes ``K`` (seams after a cut)."""
    out = dict(pix)
    occupied = set(pix) | set(others or {})
    for (x, y), ch in pix.items():
        if ch == "K":
            continue
        for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if n not in occupied and 0 <= n[0] < size[0] and 0 <= n[1] < size[1]:
                out[n] = "K"
    return out


def drop_dangling_outline(pix: Pix) -> Pix:
    """Remove ``K`` pixels that no longer touch any coloured pixel (8-neighbourhood) after a cut."""
    out = {}
    for (x, y), ch in pix.items():
        if ch == "K" and not any(pix.get((x + a, y + b), "K") != "K"
                                 for a in (-1, 0, 1) for b in (-1, 0, 1) if a or b):
            continue
        out[(x, y)] = ch
    return out


def rows_of(pix: Pix, size: tuple[int, int]) -> list[str]:
    w, h = size
    grid = [["."] * w for _ in range(h)]
    for (x, y), ch in pix.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel outside the {w}x{h} canvas: {(x, y)}")
        if ch not in KEYS:
            raise ValueError(f"unknown key {ch!r}")
        grid[y][x] = ch
    return ["".join(r) for r in grid]


def cels(body: Pix, weapon: Pix | None = None, gore: Pix | None = None, size=CANVAS) -> dict[str, list[str]]:
    weapon = {k: v for k, v in (weapon or {}).items() if k not in body}
    for ch in list(body.values()) + list(weapon.values()):
        if ch in GORE_ONLY:
            raise ValueError(f"gore-only key {ch!r} outside the gore layer")
    return {"body": rows_of(body, size), "weapon": rows_of(weapon, size), "gore": rows_of(gore or {}, size)}


def overlay(base: Pix, top: Pix) -> Pix:
    out = dict(base)
    out.update(top)
    return out


# --------------------------------------------------------------------------- pose key 0: crush_contact
def segs(rows: dict[int, list[tuple[int, str]]]) -> Pix:
    """Literal rows written as ``{y: [(x, "keys"), ...]}`` (each run starts at an explicit column)."""
    out: Pix = {}
    for y, runs in rows.items():
        for x0, run in runs:
            out.update(literal({y: run}, x0))
    return out


CRUSH_CONTACT = {
    33: [(6, "K" * 42)],
    34: [(4, "KK"), (6, "llllllllllllllll"), (22, "K"), (23, "llllllllllll"), (35, "KKKKKKKK"), (43, "llll"), (47, "K")],
    35: [(3, "KllmmmmmmmmmmmmmmssK"), (23, "lmmmmmmmmmmm"), (35, "KwwKwwwK"), (43, "mmmlK")],
    36: [(4, "KKsssssmmmmmmmmssaK"), (23, "lmmmmmmmmmmm"), (35, "KKwKwKwK"), (43, "mmmmlK")],
    37: [(6, "KKKKaaaaasssssaaK"), (23, "lmmmmmmmmmmm"), (35, "KKKKwwwK"), (43, "mmmmmlKK")],
    38: [(10, "KKKKKKKKKKKK"), (22, "Klmmmmmmmmmmmmmm"), (38, "KKKKK"), (43, "mmmmmmllK")],
    39: [(9, "KuuuuuuuuuuuuKlmmmmmmmmmmmmmmmmmmmmmmmmmmmlK")],
    40: [(9, "KutttttttttttuK"), (24, "lmmmmmmmmmmmm"), (37, "KKKKKKKKKKK"), (48, "mmssK")],
    41: [(3, "KK"), (9, "KuttttttttttttuK"), (25, "lmmmmmmmmmmK"), (37, "wKwwKwwKwK"), (47, "KsmsK")],
    42: [(2, "KlsK"), (7, "KKuttttttttttttuK"), (24, "lmmmmmmmmmmKooooooooooK"), (47, "KsK")],
    43: [(1, "KlmmsKKmmKutttttttttttttttK"), (27, "lsmmmmmmmK"), (37, "oooRRRRoooK"), (48, "K")],
    44: [(0, "KlmsasmmssKutttttttttttttttKlssmmmmmmKowRRRRRRwKK")],
    45: [(0, ".KsaKKaaaKKutttrttttttttttttKlsssssssKKwRRRRRRRRK")],
    46: [(0, "..KK.KKKKKrtttrtttttttttttttKKKKKKKKKKKKKKKKKRRRRK")],
    47: [(0, "..........KrrttttttttttttttttttttKKKKllllllllllllKRRRK.K.K.K")],
    48: [(0, "..........KrrrrrtttttrrrrrrrrrrrKmmmmmmmmmmmmmmmmlKRRKKlKlKlK")],
    49: [(0, "...........KyyxxyyxxyyyxxyyxxyyKKsssssssssssssssmmKKKmlmlmlmK")],
    50: [(0, "...........KrrrrrrrrrrrrrrrrrrrrrKKKKKKKKKKKKKKKKKsssmmmmmmmK")],
    51: [(0, "..........KmKrrKrrrrKKrrrKrrrKrrrKKlmK...........KKssssssK")],
    52: [(0, ".........KmssKKKK.KKK..KKK..KKKKKKlmmlK............KKKKKK")],
    53: [(7, "KKmsssK"), (34, "KlmmlKK")],
    54: [(6, "KmmssssaK"), (34, "KlmmssllK")],
    55: [(6, "KKKKKKKKK"), (34, "KKKKKKKKK")],
}


def crush_contact() -> dict[str, list[str]]:
    """The landing tick of the block: the goblin is rammed down to 23 px (flat top on row 33, the stone's underside),
    head shoved forward and flattened, the long ear pressed flat back over the hump, eyes bulging, jaw sprung, tongue
    out, both arms flung out flat with open hands (the club is gone: it is the ``club`` part, launched on this tick),
    knees splayed. Gore: blood forced out of the mouth and from under the stone's edge."""
    return cels(close_outline(segs(CRUSH_CONTACT)), {}, segs(CRUSH_GORE))


def crushed_remains() -> dict[str, list[str]]:
    """Hold after the landing: a broken, low heap (14 px, rows 42 to 55) at the foot of the landed stone, not a paper
    cut-out. Pressed-flat top line on row 47, the long ear still standing up out of the heap (the 1x goblin cue), head
    on its cheek with one bulging eye and the tongue on the floor, near hand open, one shin snapped up with the sole
    showing. The club is not here: it is the ``club`` part, already on the floor somewhere else."""
    corpse = cel_pix(load_goblin()["frames"][25]["cels"]["body"])
    head = {(x, y): ch for (x, y), ch in corpse.items() if x <= (22 if y <= 50 else 18)}
    body = drop_dangling_outline(close_outline(overlay(head, literal(CRUSHED_REMAINS))))
    return cels(body, {}, segs(CRUSHED_REMAINS_GORE))


CRUSH_GORE = {               # preset 4 impact: mouth gush and squeeze-out bursts at both squeeze anchors
    41: [(56, "RR")],
    42: [(49, "RRPPRR"), (57, "RR")],
    43: [(49, "BBRRBB"), (59, "RR")],
    51: [(1, "RR")],
    53: [(3, "RRPR"), (44, "RPRR")],
    54: [(1, "BRRRRR"), (43, "RRRRRRB"), (52, "RR")],
    55: [(2, "BBBBB"), (43, "BBBBBBB")],
}

CRUSHED_REMAINS_GORE = {     # one continuous pressed-out seep under the whole heap (the pool decals spread it later)
    53: [(17, "RP")],
    54: [(15, "RRRRR"), (23, "RRRPRRR"), (42, "RRR")],
    55: [(9, "BBRRRRRRRBBRRRRPRRRRRRRRRRRRRRPRRRRRRBBB")],
}
CRUSHED_REMAINS = {   # the new pressed body, right of the reused GOB-01 frame-25 head (x 18 and up)
    42: "....................................................KKK",
    43: "...................................................KlmsK",
    44: "...................................................KlmsK",
    45: "..................................................KlmsK",
    46: "..................................................KlmK",
    47: ".................................KKKK............KlmsK",
    48: "......................KKKKKKKKKKKuuuuKKKKK..KKKKKlmsK",
    49: "......................KuuuuuttuuuuuuuuuutuuKKuuuuKlmsKK",
    50: "......................KutttttrttttttttttrttttuttKlmmsllK",
    51: "......................KuttttttrtttttttttrrtttKKlmmmmmmKK",
    52: "...................KKKKyyxxyyxxKKKKKKKKyyxxyyKlmsKKKKlmmlK",
    53: "...................KrrrrrrrrrrrKlllmmmKKrrrrKlmsK...KlmmmlK",
    54: "...................KrrrrrrrrrrKlmmmmmsssKrrKKlsK...KlssssllK",
    55: "....................KKKKKKKKKKKKKKKKKKKKKKKKKKKK.....KKKKKKKK",
}


# --------------------------------------------------------------------------- pose key 3: decap_contact
# GOB-01 A frame 16 (hurt recoil): the arm flung up and back is kept up to these columns in rows 22 to 35; everything
# right of them in those rows (the thrown-back head, the ear and the jaw) is the ``head`` part now.
F16_ARM_RIGHT = {22: 14, 23: 14, 24: 14, 25: 14, 26: 14, 27: 14, 28: 14, 29: 15, 30: 15, 31: 16, 32: 17, 33: 19, 34: 18,
                 35: 22}
DECAP_NECK = {
    31: [(20, "KKKKKKK")],
    32: [(19, "KlmmmmsK")],
    33: [(19, "KlmmmmssK")],
    34: [(19, "KllmmmssK")],
    35: [(19, "KrrrmmssKK")],
    36: [(24, "KmmmsK")],
}
DECAP_GORE = {
    12: [(17, "RB")],
    13: [(17, "BR")],
    15: [(14, "BR"), (21, "RB")],
    16: [(14, "BR"), (20, "RB")],
    18: [(17, "BRR"), (23, "RB")],
    19: [(17, "BRR"), (23, "BR")],
    20: [(18, "BRPR")],
    21: [(18, "BRPR")],
    22: [(19, "BRPRR")],
    23: [(19, "BRRPR")],
    24: [(20, "BRPRR")],
    25: [(20, "BRPRR")],
    26: [(20, "BRRPRR")],
    27: [(20, "BRRPRR")],
    28: [(20, "BRRRPRR")],
    29: [(20, "BRRPRRR")],
    30: [(20, "BBwwRRRR")],
    31: [(20, "BRwRRPRR")],
    32: [(24, "RRR")],
    33: [(25, "RR")],
}


def decap_contact() -> dict[str, list[str]]:
    """The blade's tick: GOB-01 frame 16's recoil (torso thrown back, arm flung up, stick trailing low behind) with the
    head gone. The head leaves as the ``head`` part from the ``neck`` anchor on this tick. A short neck stump, cut
    surface on the gore layer (spine bone ``w`` at the back), and the money shot: a jet straight up out of the stump that
    breaks into gobbets near the top, leaning back the way the head flew."""
    g = load_goblin()["frames"][16]["cels"]
    src = cel_pix(g["body"])
    body = {(x, y): ch for (x, y), ch in src.items() if y >= 36 or (y >= 22 and x <= F16_ARM_RIGHT[y])}
    weapon = cel_pix(g["weapon"])
    body = overlay(body, segs(DECAP_NECK))
    body = drop_dangling_outline(close_outline(body, weapon))
    return cels(body, weapon, segs(DECAP_GORE))


# --------------------------------------------------------------------------- pose key 5: headless_collapse (hold)
HEADLESS_NECK = {
    47: [(20, "KKK")],
    48: [(18, "KKllmK")],
    49: [(17, "KlmmmsK")],
    50: [(17, "KmmmssK")],
    51: [(17, "KKsssK")],
    52: [(18, "KKKK")],
}
HEADLESS_GORE = {
    48: [(17, "BB")],
    49: [(16, "BwR")],
    50: [(16, "BRP")],
    51: [(16, "BRR")],
    52: [(15, "BRR")],
    53: [(14, "BRR")],
    54: [(12, "BBRRR")],
    55: [(6, "BBBRRRRRPRRRBB")],
}


def headless_collapse() -> dict[str, list[str]]:
    """The hold after a beheading: GOB-01 frame 25 (``corpse``: on the back, knees up, the pineapple rolled off the
    stick) with the head columns removed, a short neck stump towards the camera-left, the cut face and a run of blood to
    a small puddle on the floor (gore layer). The spreading pool is a decal, not this frame."""
    g = load_goblin()["frames"][25]["cels"]
    src = cel_pix(g["body"])
    body = {(x, y): ch for (x, y), ch in src.items() if x >= (22 if y <= 50 else 19)}
    weapon = cel_pix(g["weapon"])
    body = overlay(body, segs(HEADLESS_NECK))
    body = drop_dangling_outline(close_outline(body, weapon))
    return cels(body, weapon, segs(HEADLESS_GORE))


# --------------------------------------------------------------------------- parts sheet keys (32 x 32 cells)
# Part masks on GOB-01 A frame 0 (idle): per row, the inclusive x range of the part's ``body`` pixels.
HEAD_ROWS = {y: (18, 63) for y in range(20, 30)} | {30: (27, 63), 31: (27, 63), 32: (28, 63), 33: (29, 63)}
TORSO_ROWS = {30: (19, 26), 31: (19, 27), 32: (19, 28), 33: (19, 29)} | {y: (17, 32) for y in range(34, 42)}
HEAD_SHIFT = (-17, -11)       # frame 0 -> cell: the skull centre (33, 27) lands on the cell pivot (16, 16)
TORSO_SHIFT = (-10, -20)      # the vest centre (26, 36) lands on (16, 16)
CLUB_SHIFT = (-28, -18)       # the club's mid-shaft (44, 34) lands on (16, 16)
HEAD_GORE = {                 # the neck cut, in cell coordinates (back-bottom of the skull)
    18: [(9, "BR")],
    19: [(9, "BRP")],
    20: [(10, "BRR")],
    21: [(11, "wRR")],
    22: [(12, "RR")],
}
TORSO_GORE = {                # neck stump on top, torn waist at the bottom, in cell coordinates
    10: [(16, "BRRB")],
    11: [(15, "BwPRB")],
    12: [(17, "RB")],
    21: [(8, "BBRRRBRRB")],
    22: [(8, "BwwRPRRRPBB")],
    23: [(9, "BRRBB.RRB")],
}


def mask_rows(pix: Pix, rows: dict[int, tuple[int, int]]) -> Pix:
    return {(x, y): ch for (x, y), ch in pix.items() if y in rows and rows[y][0] <= x <= rows[y][1]}


def part_head() -> dict[str, list[str]]:
    """GOB-01 A frame 0's head and ear, shocked: the yellow glare becomes white eyes with the ``K`` pupils (as in the
    GOB-01 hurt keys). Neck cut at the back-bottom of the skull (gore layer)."""
    f0 = cel_pix(load_goblin()["frames"][0]["cels"]["body"])
    head = {p: ("w" if ch == "e" else ch) for p, ch in mask_rows(f0, HEAD_ROWS).items()}
    head = drop_dangling_outline(close_outline(shift(head, *HEAD_SHIFT)))
    return cels(head, {}, segs(HEAD_GORE), CELL)


def part_torso_upper() -> dict[str, list[str]]:
    """GOB-01 A frame 0's leather vest hump and chest strip (the near arm and the head removed), neck stump on top, the
    waist torn open at the bottom (gore layer: spine bone at the back, meat, wet highlights)."""
    f0 = cel_pix(load_goblin()["frames"][0]["cels"]["body"])
    torso = drop_dangling_outline(close_outline(shift(mask_rows(f0, TORSO_ROWS), *TORSO_SHIFT)))
    return cels(torso, {}, segs(TORSO_GORE), CELL)


def part_club() -> dict[str, list[str]]:
    """GOB-01 A frame 0's pineapple (crown and fruit, unchanged) on a straight shaft. The idle shaft is half hidden by
    the fist and kinks under the fruit, so the loose club gets a new, straight 2 px shaft (light ``y`` on the upper
    edge, ``x`` below) at the idle angle, the butt end capped. No gore: the club is the clean, readable piece in every
    preset."""
    f0 = cel_pix(load_goblin()["frames"][0]["cels"]["weapon"])
    fruit = {(x, y): ch for (x, y), ch in f0.items() if y <= 35}         # crown and fruit, unchanged
    club = overlay(shift(fruit, *CLUB_SHIFT), literal(CLUB_SHAFT))
    club = drop_dangling_outline(close_outline(club))
    return {"body": rows_of({}, CELL), "weapon": rows_of(club, CELL), "gore": rows_of({}, CELL)}


CLUB_SHAFT = {                # cell coordinates; the fruit's bottom outline is row 18
    18: ".....................yx",
    19: "....................yx",
    20: "..................yyx",
    21: ".................yx",
    22: "...............yyx",
    23: "..............yx",
    24: "............yyx",
    25: "...........yx",
    26: ".........yyx",
    27: "........yx",
    28: "........x",
}

PART_TAGS = [("head", 0, 0), ("torso_upper", 1, 1), ("club", 2, 2)]


def parts_grid() -> dict:
    frames = [part_head(), part_torso_upper(), part_club()]
    return grid_of(frames, [0] * len(frames), CELL, CELL_PIVOT, PART_TAGS)


def poses_grid() -> dict:
    k0 = crush_contact()
    k2 = crushed_remains()
    k3 = decap_contact()
    k5 = headless_collapse()
    frames = [k0, k0, k2, k3, k3, k5]          # 1 and 4 are placeholders (copies) until the executor draws them
    ticks = [3, 3, 0, 4, 6, 0]
    return grid_of(frames, ticks, CANVAS, PIVOT, POSE_TAGS)


POSE_TAGS = [("crush_contact", 0, 1), ("crushed_remains", 2, 2), ("decap_contact", 3, 3), ("headless_collapse", 4, 5)]


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ) if ticks else HOLD_MS


def grid_of(frames, ticks, canvas, pivot, tags) -> dict:
    return {
        "format": "pixel-grid-v1",
        "canvas": list(canvas),
        "pivot": list(pivot),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": c} for t, c in zip(ticks, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in tags],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# --------------------------------------------------------------------------- 1x silhouette and scale sheet
OPENING_KIT = ROOT / "assets/source/terrain/floor01/floor01_opening_kit.json"


def block_piece(piece_id: str):
    """(image, anchor) of an ENV-02 opening-kit piece, cropped from its committed pixel-grid source."""
    from scripts.pixelart.common import grid_images, load_grid_file
    kit = json.loads(OPENING_KIT.read_text(encoding="utf-8"))
    piece = next(p for p in kit["pieces"] if p["id"] == piece_id)
    atlas = grid_images(load_grid_file(ROOT / kit["atlases"][piece["atlas"]]["grid"]))[0]
    x, y, w, h = piece["rect"]
    return atlas.crop((x, y, x + w, y + h)), tuple(piece["anchor"])


def write_scale_sheet(out: Path = SCALE_SHEET) -> Path:
    """Two lit Floor 1 views at 1x, stacked. Top: Donut (40 px) and Carl (52 px), then the crush with the ENV-02 stone
    composited (``loose_block_falling`` with its underside on frame row 33 over ``crush_contact``, and
    ``crushed_remains`` in front of the foot of ``loose_block_landed``), the two decapitation keys and the three part
    keys settled on the walking line. Bottom: the uncomposited lineup with the GOB-01 A idle for scale. A second file
    holds the walking-line bands of both views at 4x."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, DONUT_X, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    gob = grid_images(load_grid_file(GOBLIN_A))[0]
    poses = grid_images(poses_grid())
    parts = grid_images(parts_grid())
    falling, falling_anchor = block_piece("loose_block_falling")
    landed, landed_anchor = block_piece("loose_block_landed")
    piv = tuple(PIVOT)
    cast = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]

    def settled_parts(x: int) -> list:
        out = []
        for img in parts:                 # settle each part on the walking line: its lowest opaque row on the floor
            out.append((img, (CELL_PIVOT[0], img.getbbox()[3] - 1), x, 0, False))
            x += 30
        return out

    composed = cast + [
        (falling, falling_anchor, CARL_X + 88, PIVOT[1] - 33, False),     # stone underside on frame row 33
        (poses[0], piv, CARL_X + 88, 0),
        (landed, landed_anchor, CARL_X + 208, 0, False),
        (poses[2], piv, CARL_X + 208, 0),
        (poses[3], piv, CARL_X + 298, 0),
        (poses[5], piv, CARL_X + 358, 0),
    ] + settled_parts(CARL_X + 408)
    plain = cast + [(gob, piv, CARL_X + 60, 0)]
    x = CARL_X + 124
    for n in POSE_KEYS:
        plain.append((poses[n], piv, x, 0))
        x += 66 if n in (0, 2) else 56
    plain += settled_parts(x)
    floor = Floor()
    scenes = [floor.render(composed, 1), floor.render(plain, 1)]
    ground = floor.ground(CARL_X) - floor.view[1]                 # walking line in view pixels
    w, h = scenes[0].size
    sheet = Image.new("RGB", (w, h * 2), (24, 23, 34))
    bands = []
    for i, sc in enumerate(scenes):
        sheet.paste(sc, (0, i * h))
        bands.append(sc.crop((0, max(0, ground - 74), w, min(h, ground + 14))))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    bw, bh = bands[0].size
    big = Image.new("RGB", (bw * 4, bh * 8 + 8), (24, 23, 34))
    for i, band in enumerate(bands):
        big.paste(band.resize((bw * 4, bh * 4), Image.NEAREST), (0, i * (bh * 4 + 8)))
    big.save(out.with_name("silhouette_scale_4x.png"))
    return out


def strip_gore(grid: dict) -> dict:
    """The preset 1/2 view of a source: the same grid with the ``gore`` layer hidden (nothing redrawn)."""
    out = json.loads(json.dumps(grid))
    for f in out["frames"]:
        f["cels"].pop("gore", None)
    return out


def write_key_sheet(out: Path = SCALE_SHEET.with_name("keys_x4_preset4_vs_preset1.png")) -> Path:
    """Each key at 4x on a flat brick tone: the top row of each pair as authored (preset 4, gore layer shown), the
    bottom row the same pixels with the gore layer hidden (preset 1). Bottom strip: every key at 1x, preset 4 then
    preset 1."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import grid_images
    k = 4
    rows = []
    for grid in (poses_grid(), parts_grid()):
        full, bare = grid_images(grid), grid_images(strip_gore(grid))
        keys = POSE_KEYS if grid["canvas"] == list(CANVAS) else PART_KEYS
        rows.append([(full[n], bare[n]) for n in keys])
    cw, ch = CANVAS[0] * k, CANVAS[1] * k
    strip_h = CANVAS[1] + 16                 # bottom strip: every key at 1x, preset 4 then preset 1, side by side
    sheet = Image.new("RGB", (cw * 4 + 5 * 8, ch * 4 + 5 * 8 + strip_h), (40, 30, 34))
    x = 8
    for a, b in rows[0] + rows[1]:
        for j, img in enumerate((a, b)):
            tile = Image.new("RGB", (img.width, CANVAS[1]), (110, 64, 44) if j == 0 else (96, 58, 42))
            tile.paste(img, (0, CANVAS[1] - img.height), img)
            sheet.paste(tile, (x, ch * 4 + 5 * 8 + 8))
            x += img.width + 2
        x += 6
    for col, (a, b) in enumerate(rows[0] + rows[1]):
        r, c = divmod(col, 4)
        for j, img in enumerate((a, b)):
            tile = Image.new("RGB", (cw, ch), (110, 64, 44) if j == 0 else (96, 58, 42))
            big = img.resize((img.width * k, img.height * k), Image.NEAREST)
            tile.paste(big, ((cw - big.width) // 2, ch - big.height), big)
            sheet.paste(tile, (8 + c * (cw + 8), 8 + (r * 2 + j) * (ch + 8)))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


POSE_KEYS = (0, 2, 3, 5)
PART_KEYS = (0, 1, 2)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "scale":
        print(f"wrote {write_scale_sheet().relative_to(ROOT)}, {write_key_sheet().relative_to(ROOT)}")
        return 0
    outputs = [(POSES, POSES_ASE, poses_grid()), (PARTS, PARTS_ASE, parts_grid())]
    if args.check:
        same = all(p.is_file() and p.read_text(encoding="utf-8") == dump(g) for p, _a, g in outputs)
        print("death sources up to date" if same else "death sources differ from the generator")
        return 0 if same else 1
    sys.path.insert(0, str(ROOT / "scripts"))
    import pixel_grid_to_ase as pga  # noqa: E402
    for path, ase, grid in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(dump(grid), encoding="utf-8")
        pga.write_ase(grid, ase)
        print(f"wrote {path.relative_to(ROOT)}, {ase.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
