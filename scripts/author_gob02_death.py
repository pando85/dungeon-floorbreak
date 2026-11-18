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
    """Hold after the landing, facing right like keys 0 and 1 (review fix D1: authored head-left from the GOB-01 corpse,
    then mirrored with x' = 59 - x, the furthest a pivot mirror (x' = 56 - x) can go without clipping the snapped
    shin; the heap sits 3 px forward). A broken, low heap (rows 40 to 55) at the foot of the landed stone, not a paper
    cut-out. Pressed-flat top line on row 47, the long ear still standing up out of the heap (the 1x goblin cue), head
    on its cheek with one bulging eye and the tongue on the floor, near hand open, one shin snapped up with the sole
    showing. The club is not here: it is the ``club`` part, already on the floor somewhere else."""
    corpse = cel_pix(load_goblin()["frames"][25]["cels"]["body"])
    head = {(x, y): ch for (x, y), ch in corpse.items() if x <= (22 if y <= 50 else 18)}
    body = drop_dangling_outline(close_outline(overlay(head, literal(CRUSHED_REMAINS))))
    return mirror_cels(cels(body, {}, segs(CRUSHED_REMAINS_GORE)), REMAINS_MIRROR_X)


REMAINS_MIRROR_X = 59        # x' = 59 - x: the heap is drawn head-left, then mirrored to face right like keys 0 and 1


def mirror_cels(c: dict[str, list[str]], axis: int) -> dict[str, list[str]]:
    """Mirror every layer horizontally with x' = axis - x (pixels must stay inside the canvas)."""
    out = {}
    for name, rows in c.items():
        w = len(rows[0])
        new = []
        for y, row in enumerate(rows):
            r = ["."] * w
            for x, ch in enumerate(row):
                if ch != ".":
                    if not 0 <= axis - x < w:
                        raise ValueError(f"mirror moves ({x}, {y}) off the canvas")
                    r[axis - x] = ch
            new.append("".join(r))
        out[name] = new
    return out


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


# --------------------------------------------------------------------------- parts 3 to 23 (executor pass)
# Rule parts are GOB-01 A frame 0 rectangles translated whole (no resampling, no rotation); every cut surface is the
# deterministic ``cut_face`` rule on the gore layer; gibs, drops and decals are small literal bitmaps or rules.
PARTS_NEW_FIRST = 3
SKIN_DARKER = {"l": "m", "m": "s", "s": "a"}          # the far limb: the skin ramp one step darker (a stays a)


def f0_layer(layer: str) -> Pix:
    return cel_pix(load_goblin()["frames"][0]["cels"][layer])


def rect_pix(pix: Pix, x0: int, y0: int, x1: int, y1: int) -> Pix:
    return {p: ch for p, ch in pix.items() if x0 <= p[0] <= x1 and y0 <= p[1] <= y1}


def bbox_of_pix(pix: Pix) -> tuple[int, int, int, int]:
    xs = [p[0] for p in pix]
    ys = [p[1] for p in pix]
    return min(xs), min(ys), max(xs), max(ys)


def centring_shift(box: tuple[int, int, int, int]) -> tuple[int, int]:
    """Translation that puts the centre of ``box`` on the cell pivot (16, 16)."""
    return CELL_PIVOT[0] - (box[0] + box[2]) // 2, CELL_PIVOT[1] - (box[1] + box[3]) // 2


_EDGE = {   # edge -> (to (u, d), from (u, d)); d is the depth inward from the outline line, u runs along the edge
    "top": (lambda x, y, e: (x, y - e), lambda u, d, e: (u, e + d)),
    "bottom": (lambda x, y, e: (x, e - y), lambda u, d, e: (u, e - d)),
    "left": (lambda x, y, e: (y, x - e), lambda u, d, e: (e + d, u)),
    "right": (lambda x, y, e: (y, e - x), lambda u, d, e: (e - d, u)),
}


def cut_face(body: Pix, edge: str) -> Pix:
    """The cut surface of a wound part on the ``gore`` layer, from the outlined ``body``. The outline line at ``edge``
    is overdrawn with meat (``R``, clotted ``B`` at the ends), the skin line behind it is meat with a bone dot ``w`` and
    a wet glint ``P``, and a short ``R``/``B`` drip runs back along the skin. The body underneath stays a plain
    skin-and-outline cap: hiding the gore layer leaves a closed, bloodless sprite."""
    to_ud, from_ud = _EDGE[edge]
    x0, y0, x1, y1 = bbox_of_pix(body)
    e = {"top": y0, "bottom": y1, "left": x0, "right": x1}[edge]
    inner = {}
    for (x, y), ch in body.items():
        u, d = to_ud(x, y, e)
        if d == 1 and ch != "K":
            inner[u] = ch
    span = sorted(inner)
    n = len(span)
    out: Pix = {}

    def put(u: int, d: int, key: str) -> None:
        out[from_ud(u, d, e)] = key

    for i, u in enumerate(span):
        end = n >= 4 and i in (0, n - 1)
        put(u, 0, "B" if end else "R")
        put(u, 1, "B" if end else "R")
    if n >= 3:
        put(span[n // 2], 1, "w")
        put(span[n - 2 if n >= 4 else n - 1], 1, "P")
        if from_ud(span[1], 2, e) in body and body[from_ud(span[1], 2, e)] != "K":
            put(span[1], 2, "R")
            if from_ud(span[1], 3, e) in body and body[from_ud(span[1], 3, e)] != "K":
                put(span[1], 3, "B")
    else:
        put(span[0], 1, "w")
        put(span[-1], 0, "P")
    return out


def finish_part(body: Pix, weapon: Pix, gore: Pix) -> dict[str, list[str]]:
    return cels(body, weapon, gore, CELL)


def wound_part(source: Pix, rect: tuple[int, int, int, int], cut_edges: list[str], mutate=None):
    """(cels, shift) for a frame-0 rectangle turned into a wound part: translated onto the cell, outline-closed, the
    cut face on every edge named in ``cut_edges``."""
    x0, y0, x1, y1 = rect
    shift_ = centring_shift(rect)
    body = shift(rect_pix(source, x0, y0, x1, y1), *shift_)
    if mutate:
        body = mutate(body)
    body = drop_dangling_outline(close_outline(body, size=CELL))
    gore: Pix = {}
    for edge in cut_edges:
        gore.update(cut_face(body, edge))
    return finish_part(body, {}, gore), shift_


ARM_RECT = (33, 34, 41, 41)
LEG_NEAR_RECT = (28, 47, 38, 55)
LEG_FAR_RECT = (19, 47, 26, 55)
EAR_RECT = (18, 20, 24, 24)
LOWER_RECT = (19, 42, 35, 46)
PINEAPPLE_RECT = (50, 47, 62, 55)          # GOB-01 A frame 25, weapon layer
FRUIT_ON_FRAME0 = (49, 32)                 # the fruit's centre on frame 0 (club part: fruit anchor (21, 14))
CELL_SHIFT_RECORD: dict[str, tuple[int, int]] = {}   # part id -> frame-0 -> cell translation (for frame0_origin)


def part_head_dead() -> dict[str, list[str]]:
    """Part 0 with GOB-01 frame 25's dead face: a 3 x 3 ``K`` X (white between the strokes) over each eye and a 2 x 3
    ``R`` tongue out of the mouth corner, outlined. Everything else is part 0, pixel for pixel."""
    head = part_head()
    body = cel_pix(head["body"])
    for x0 in (16, 20):                                # far eye box x 16..18, near eye box x 20..22 (rows 14..16)
        for dy, row in enumerate(("KwK", "wKw", "KwK")):
            for dx, ch in enumerate(row):
                body[(x0 + dx, 14 + dy)] = ch
    for y in (20, 21, 22):                             # the tongue crosses the jaw line at the mouth corner
        for x in (20, 21):
            body[(x, y)] = "R"
    body = close_outline(body, size=CELL)
    return finish_part(body, {}, cel_pix(head["gore"]))


def part_torso_lower() -> dict[str, list[str]]:
    """GOB-01 frame 0 rows 42 to 46, x 19 to 35 (rope belt and rag skirt) with the waist tear on top (the torso_upper
    tear, rows reversed) and two hip stumps under the hem. Part 1's tear is rows 21 to 23 of its cell."""
    f0 = f0_layer("body")
    belt = rect_pix(f0_layer("weapon"), 19, 42, 30, 42)
    src = overlay(rect_pix(f0, 19, 42, 35, 46), belt)
    shift_ = centring_shift(LOWER_RECT)
    CELL_SHIFT_RECORD["torso_lower"] = shift_
    body = drop_dangling_outline(close_outline(shift(src, *shift_), size=CELL))
    top = min(y for _x, y in body)                     # the outline row above the belt
    tear = {21: (8, "BBRRRBRRB"), 22: (8, "BwwRPRRRPBB"), 23: (9, "BRRBB.RRB")}
    gore: Pix = {}
    x_mid = (min(x for x, _y in body) + max(x for x, _y in body)) // 2
    for y, (x0, run) in tear.items():
        for i, ch in enumerate(run):
            if ch != ".":
                gore[(x_mid - 13 + x0 + i, top + (21 - y) + 0)] = ch       # row 21 on the outline, 22 and 23 above
    bottom = max(y for _x, y in body)
    for hx in HIP_STUMPS:                              # two hip stumps under the hem: 2 x 1, meat and bone
        gore[(hx[0] + shift_[0], bottom + 1)] = "R"
        gore[(hx[0] + 1 + shift_[0], bottom + 1)] = "w"
    return finish_part(body, {}, gore)


HIP_STUMPS = [(22, 47), (31, 47)]                  # frame-0 x of each stump's left pixel (far, near), under the hem


def part_arm_near() -> dict[str, list[str]]:
    cel, shift_ = wound_part(f0_layer("body"), ARM_RECT, ["top"])
    CELL_SHIFT_RECORD["arm_near"] = shift_
    return cel


def part_arm_far() -> dict[str, list[str]]:
    near = part_arm_near()
    body = {p: SKIN_DARKER.get(ch, ch) for p, ch in cel_pix(near["body"]).items()}
    return finish_part(body, {}, cel_pix(near["gore"]))


def part_leg_near() -> dict[str, list[str]]:
    cel, shift_ = wound_part(f0_layer("body"), LEG_NEAR_RECT, ["top"])
    CELL_SHIFT_RECORD["leg_near"] = shift_
    return cel


def part_leg_far() -> dict[str, list[str]]:
    cel, shift_ = wound_part(f0_layer("body"), LEG_FAR_RECT, ["top"])
    CELL_SHIFT_RECORD["leg_far"] = shift_
    return cel


def part_ear() -> dict[str, list[str]]:
    cel, shift_ = wound_part(f0_layer("body"), EAR_RECT, ["right"])
    CELL_SHIFT_RECORD["ear"] = shift_
    return cel


def part_club_stick() -> dict[str, list[str]]:
    """Part 2's shaft alone (rows 18 to 28 of its cell), the top end splintered into two 1 px ``y`` teeth."""
    club = cel_pix(part_club()["weapon"])
    shaft = {p: ch for p, ch in club.items() if p[1] >= 18 and ch in "xy"}
    shaft = overlay(shaft, literal({17: "....................y.y"}))
    shaft = close_outline(shaft, size=CELL)
    shaft = drop_dangling_outline(shaft)
    # re-centre on the cell
    sh = centring_shift(bbox_of_pix(shaft))
    shaft = shift(shaft, *sh)
    CELL_SHIFT_RECORD["club_stick"] = sh
    return {"body": rows_of({}, CELL), "weapon": rows_of(shaft, CELL), "gore": rows_of({}, CELL)}


def part_pineapple() -> dict[str, list[str]]:
    """GOB-01 A frame 25's rolled fruit, weapon layer rows 47 to 55, x 50 to 62, unchanged."""
    g = load_goblin()["frames"][25]["cels"]["weapon"]
    fruit = rect_pix(cel_pix(g), *PINEAPPLE_RECT)
    sh = centring_shift(PINEAPPLE_RECT)
    return {"body": rows_of({}, CELL), "weapon": rows_of(shift(fruit, *sh), CELL), "gore": rows_of({}, CELL)}


def small(body_rows: dict[int, str], gore_rows: dict[int, str] | None = None, weapon_rows: dict[int, str] | None = None):
    """A literal gib, drop or decal: rows written from (0, 0); the whole bitmap is centred on the cell pivot."""
    body, gore, weapon = literal(body_rows), literal(gore_rows or {}), literal(weapon_rows or {})
    union = {**body, **gore, **weapon}
    sh = centring_shift(bbox_of_pix(union))
    return finish_part(shift(body, *sh), shift(weapon, *sh), shift(gore, *sh)), sh


GIBS = {
    "gib_flesh_a": ({0: ".KKK.", 1: "KlmmK", 2: "KmmsK", 3: ".KKK."}, {1: "...P.", 2: "...R."}),
    "gib_flesh_b": ({0: ".KK.", 1: "KutK", 2: ".KK."}, {1: "..R."}),
    "gib_bone": ({0: "KKKK.", 1: "KwwwK", 2: ".KKKK"}, {}),
    "gib_rag": ({0: ".KKKK", 1: "KutrK", 2: ".KKK."}, {}),
}
BLOOD = {
    "drop_blood": ({}, {0: "RP", 1: "BR"}),
    "decal_splat_s": ({}, {0: ".RRPR.", 1: "BBRRBB"}),
    "decal_splat_m": ({}, {0: "RR...RPR..RR", 1: ".BRRRRRRRRRB", 2: "..BBBBBBBB.."}),
}


def decal_pool_rows() -> dict[str, dict[int, str]]:
    """Three nested pools: 12 x 2, 20 x 3, 28 x 4. Each is the previous one grown by one ring plus a wider run."""
    p1 = {16: "oBBRPRRBBBBo", 17: "oBBBBBBBBBBo"}
    p2 = {15: "..oBBRRRRBBo..".center(20, "."), 16: "oBBBRRRRPRRRRBBBBBBo", 17: "oBBBBBBBBBBBBBBBBBBo"}
    p3 = {14: "...oBBBRRRRBBBo...".center(28, "."), 15: ".oBBBBRRRRRRRRRRBBBBBBo.".center(28, "."),
          16: "oBBBBRRRRRRPRRRRRRRRBBBBBBBo", 17: "oBBBBBBBBBBBBBBBBBBBBBBBBBBo"}
    return {"decal_pool_1": p1, "decal_pool_2": p2, "decal_pool_3": p3}


def part_small(name: str):
    if name in GIBS:
        return small(*GIBS[name])
    if name in BLOOD:
        return small(*BLOOD[name])
    raise KeyError(name)


def part_pool(name: str):
    """Pools are placed, not centred: bottom row 17, shared centre, so a frame swap grows the stain in place."""
    rows = decal_pool_rows()[name]
    width = max(len(r) for r in rows.values())
    x0 = 16 - width // 2
    pix = literal(rows, x0)
    return finish_part({}, {}, pix), (x0, 0)


SPATTER_RAYS = [(8, 21), (20, 20), (32, 19), (45, 19), (58, 19), (70, 17), (82, 17)]    # (degrees up from the floor, length)
SPATTER_DRIPS = [(3, 5), (4, 4), (5, 6)]          # (ray index, length): a B tail 1 px wide hanging from that streak's tip


def part_wall_spatter() -> dict[str, list[str]]:
    """Back-wall spatter, 22 x 18: seven radial streaks from the lower-left origin (``B`` all along, ``R`` through the
    middle, two ``P`` glints) and three dripping tails (``B``, 1 px wide, 3 to 6 px long). Gore layer only."""
    import math
    w, h = 22, 18
    pix: Pix = {}
    for i, (deg, length) in enumerate(SPATTER_RAYS):
        a = math.radians(deg)
        for step in range(length * 2 + 1):
            t = step / 2
            x, y = round(t * math.cos(a)), h - 1 - round(t * math.sin(a))
            if 0 <= x < w and 0 <= y < h:
                pix[(x, y)] = "R" if length * 0.25 <= t <= length * 0.6 else "B"
    for (x, y), ch in list(pix.items()):
        if ch == "R" and (x + y) % 5 == 0 and (x + 1, y) in pix:
            pix[(x + 1, y)] = "R"
    for ray, n in SPATTER_DRIPS:                       # the tail hangs straight down from the streak tip
        deg, length = SPATTER_RAYS[ray]
        a = math.radians(deg)
        tx, ty = round(length * math.cos(a)), h - 1 - round(length * math.sin(a))
        for k in range(1, n + 1):
            if ty + k < h:
                pix.setdefault((tx, ty + k), "B")
    for (x, y) in ((6, h - 1 - 3), (13, h - 1 - 6)):
        pix[(x, y)] = "P"
    sh = (16 - w // 2, 16 - h // 2)
    return finish_part({}, {}, shift(pix, *sh))


def part_smear() -> dict[str, list[str]]:
    """A skid smear, 18 x 2: ``B`` fading to ``o`` at the tail; the head's roll leaves it."""
    rows = {15: "BRRPRRBBBBBBBooo..", 16: "BBBBBBBBBBBBBooooo"}
    return finish_part({}, {}, literal(rows, 7))

PART_BUILDERS = [         # (id, builder); indices are the contract (spec section 7)
    ("head", part_head), ("torso_upper", part_torso_upper), ("club", part_club),
    ("head_dead", None), ("torso_lower", None), ("arm_near", None), ("arm_far", None), ("leg_near", None),
    ("leg_far", None), ("club_stick", None), ("pineapple", None), ("ear", None),
    ("gib_flesh_a", None), ("gib_flesh_b", None), ("gib_bone", None), ("gib_rag", None), ("drop_blood", None),
    ("decal_splat_s", None), ("decal_splat_m", None), ("decal_pool_1", None), ("decal_pool_2", None),
    ("decal_pool_3", None), ("decal_wall_spatter", None), ("decal_smear", None),
]
PART_IDS = [p[0] for p in PART_BUILDERS]
PART_TAGS = [(pid, i, i) for i, pid in enumerate(PART_IDS)]


def build_part(pid: str) -> dict[str, list[str]]:
    fixed = {"head": part_head, "torso_upper": part_torso_upper, "club": part_club, "head_dead": part_head_dead,
             "torso_lower": part_torso_lower, "arm_near": part_arm_near, "arm_far": part_arm_far,
             "leg_near": part_leg_near, "leg_far": part_leg_far, "club_stick": part_club_stick,
             "pineapple": part_pineapple, "ear": part_ear, "decal_wall_spatter": part_wall_spatter,
             "decal_smear": part_smear}
    if pid in fixed:
        return fixed[pid]()
    if pid in decal_pool_rows():
        return part_pool(pid)[0]
    return part_small(pid)[0]


def parts_grid() -> dict:
    frames = [build_part(pid) for pid in PART_IDS]
    return grid_of(frames, [0] * len(frames), CELL, CELL_PIVOT, PART_TAGS)


def poses_grid() -> dict:
    k0 = crush_contact()
    k2 = crushed_remains()
    k3 = decap_contact()
    k5 = headless_collapse()
    frames = [k0, crush_flatten(), k2, k3, headless_buckle(), k5]
    ticks = [3, 3, 0, 4, 6, 0]
    return grid_of(frames, ticks, CANVAS, PIVOT, POSE_TAGS)



# --------------------------------------------------------------------------- pose frame 1: crush_contact (flatten)
def layer_pix(cels_: dict[str, list[str]], layer: str) -> Pix:
    return cel_pix(cels_[layer])


FLATTEN_GORE = {              # the mouth gush lengthened by 4 px with one more droplet pair; squeeze bursts pushed out
    44: [(56, "RR")],
    45: [(49, "RRPPRRRRRR"), (59, "R"), (61, "RR")],
    46: [(49, "BBRRBBBBRR"), (60, "RR"), (63, "R")],
    50: [(2, "R")],
    51: [(0, "RR")],
    53: [(2, "RRPR"), (45, "RPRR")],
    54: [(0, "BRRRRRR"), (43, "RRRRRRRRR"), (55, "RR")],
    55: [(1, "BBBBBB"), (43, "BBBBBBBBBB")],
    52: [(46, "R"), (49, "R")],
}


def crush_flatten() -> dict[str, list[str]]:
    """Frame 1 of ``crush_contact``: frame 0 with rows 33 to 45 pushed down 3 rows (onto 36 to 48, replacing the old rows
    46 to 48; the skirt hem and legs on rows 49 to 55 stay), the seams closed with outline, the near eye popped (the
    ``K`` pupil becomes ``w``) and the blood bursts pushed further out."""
    k = crush_contact()
    body = layer_pix(k, "body")
    moved = {(x, y): ch for (x, y), ch in body.items() if y >= 49 or (y >= 46 and x >= 50)}   # the open hand stays
    moved.update({(x, y + 3): ch for (x, y), ch in body.items() if 33 <= y <= 45})
    moved[(40, 39)] = "w"                                   # the popped eye: pupil (40, 36) moved down 3
    moved = close_outline(moved)
    gore = segs(FLATTEN_GORE)
    return cels(moved, {}, gore)


# --------------------------------------------------------------------------- pose frame 4: headless_collapse (buckle)
BUCKLE_JET = {                # the jet bent back (left) over 8 rows with two gobbets; the cut face is frame 3's, moved
    33: [(19, "BRPR")],
    32: [(18, "BRPR")],
    31: [(18, "BRR")],
    30: [(17, "BRP")],
    29: [(16, "BRR")],
    28: [(15, "BR")],
    27: [(14, "BR")],
    26: [(13, "RB")],
    24: [(10, "BR")],
    23: [(12, "R")],
}


def headless_buckle() -> dict[str, list[str]]:
    """Frame 4 of ``headless_collapse``: frame 3 with the legs replaced by GOB-01 A frame 24's kneeling legs, the torso,
    neck and arm moved down 4 rows and 2 px left, the forearm block (and the stick and fruit it holds) lowered so the
    fist sits near (14, 30), and the jet reduced to an 8 px column bent back with two gobbets."""
    k3 = decap_contact()
    f24 = cel_pix(load_goblin()["frames"][24]["cels"]["body"])
    body3, weapon3, gore3 = (layer_pix(k3, n) for n in ("body", "weapon", "gore"))
    legs = {(x - 5, y): ch for (x, y), ch in f24.items() if y >= 50 and x <= 41}
    upper = {(x - 2, y + 4): ch for (x, y), ch in body3.items() if y <= 46 and not (y <= 31 and x <= 15)}
    forearm = {(x + 2, y + 7): ch for (x, y), ch in body3.items() if y <= 31 and x <= 15}
    body = overlay(overlay(legs, upper), forearm)
    held = {(x + 2, y + 7): ch for (x, y), ch in weapon3.items() if x <= 12}
    belt = {(x - 2, y + 4): ch for (x, y), ch in weapon3.items() if x > 12}
    weapon = overlay(belt, held)
    cut = {(x - 2, y + 4): ch for (x, y), ch in gore3.items() if 30 <= y <= 33 and x >= 19}
    gore = overlay(cut, segs(BUCKLE_JET))
    weapon = close_outline(weapon, body)                 # the held fruit is closed by outline too
    body = drop_dangling_outline(close_outline(body, weapon))
    return cels(body, weapon, gore)

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



# --------------------------------------------------------------------------- manifests (spec section 7)
POSE_ANCHORS = {
    0: {"hit_origin": [28, 33], "hand_near": [55, 48], "mouth": [49, 43], "squeeze_back": [6, 54],
        "squeeze_front": [46, 54]},
    1: {"hit_origin": [28, 36], "hand_near": [55, 48], "mouth": [49, 46], "squeeze_back": [6, 54],
        "squeeze_front": [46, 54]},
    2: {"pool_origin": [29, 55]},
    3: {"neck": [23, 31], "hit_origin": [24, 31], "jet_base": [23, 30], "hand_near": [12, 23]},
    4: {"neck": [21, 35], "hit_origin": [22, 35], "jet_base": [21, 34], "hand_near": [14, 30]},
    5: {"neck": [17, 50], "pool_origin": [12, 55]},
}
POSE_TICKS = [3, 3, None, 4, 6, None]
POSE_AUTHOR = ["key", "executor", "key", "key", "executor", "key"]

# id -> (class, material, mass, spin, settle, bounce, swap_on_first_contact, stain, trail, ttl, presets)
PART_META = {
    "head": ("wound", "goblin_flesh", "medium", "fast", "roll", 0.35, "head_dead", "decal_splat_s", "decal_smear", "keep", [1, 2, 3, 4]),
    "torso_upper": ("wound", "goblin_flesh", "heavy", "slow", "flop", 0.2, None, "decal_splat_s", None, "keep", [2, 3, 4]),
    "club": ("clean", "wood_fruit", "medium", "fast", "clatter", 0.3, None, None, None, "keep", [1, 2, 3, 4]),
    "head_dead": ("wound", "goblin_flesh", "medium", "fast", "roll", 0.35, None, "decal_splat_s", "decal_smear", "keep", [1, 2, 3, 4]),
    "torso_lower": ("wound", "goblin_flesh", "heavy", "slow", "flop", 0.2, None, "decal_splat_s", None, "keep", [4]),
    "arm_near": ("wound", "goblin_flesh", "light", "fast", "flop", 0.3, None, "decal_splat_s", None, "long", [3, 4]),
    "arm_far": ("wound", "goblin_flesh", "light", "fast", "flop", 0.3, None, "decal_splat_s", None, "long", [3, 4]),
    "leg_near": ("wound", "goblin_flesh", "medium", "slow", "flop", 0.25, None, "decal_splat_s", None, "long", [3, 4]),
    "leg_far": ("wound", "goblin_flesh", "medium", "slow", "flop", 0.25, None, "decal_splat_s", None, "long", [3, 4]),
    "club_stick": ("clean", "wood", "light", "fast", "clatter", 0.3, None, None, None, "long", [4]),
    "pineapple": ("clean", "fruit", "light", "slow", "roll", 0.4, None, None, None, "long", [4]),
    "ear": ("wound", "goblin_flesh", "light", "slow", "flutter", 0.2, None, None, None, "short", [4]),
    "gib_flesh_a": ("gib", "goblin_flesh", "light", "fast", "bounce", 0.45, None, "decal_splat_s", None, "short", [3, 4]),
    "gib_flesh_b": ("gib", "goblin_flesh", "light", "fast", "bounce", 0.45, None, "decal_splat_s", None, "short", [3, 4]),
    "gib_bone": ("gib", "bone", "light", "fast", "clatter", 0.4, None, None, None, "short", [4]),
    "gib_rag": ("clean", "rag", "light", "slow", "flutter", 0.1, None, None, None, "short", [1, 2, 3, 4]),
    "drop_blood": ("blood", "blood", "light", "none", "static", 0.0, None, "decal_splat_s", None, "short", [2, 3, 4]),
    "decal_splat_s": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "long", [2, 3, 4]),
    "decal_splat_m": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "long", [3, 4]),
    "decal_pool_1": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "keep", [3, 4]),
    "decal_pool_2": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "keep", [3, 4]),
    "decal_pool_3": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "keep", [4]),
    "decal_wall_spatter": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "keep", [4]),
    "decal_smear": ("blood", "blood", "light", "none", "static", 0.0, None, None, None, "keep", [4]),
}
FIXED_ANCHORS = {"head": {"neck": [11, 20]}, "head_dead": {"neck": [11, 20]},
                 "torso_upper": {"neck": [17, 10], "waist": [13, 22]}, "club": {"grip": [8, 27], "fruit": [21, 14]}}
FIXED_ORIGINS = {"head": [33, 27], "torso_upper": [26, 36], "club": [44, 34], "head_dead": [33, 27],
                 "pineapple": list(FRUIT_ON_FRAME0)}
STATIC_ANCHOR_KIND = {"decal_splat_s": "centre_bottom", "decal_splat_m": "centre_bottom",
                      "decal_pool_1": "centre_bottom", "decal_pool_2": "centre_bottom", "decal_pool_3": "centre_bottom",
                      "decal_wall_spatter": "origin", "decal_smear": "start"}


def _px(cel_: dict, layer: str) -> Pix:
    return cel_pix(cel_[layer])


def part_anchors(pid: str, cel_: dict) -> dict:
    """Anchors in cell coordinates, measured from the drawn pixels (rule parts) or the spec table (keys)."""
    if pid in FIXED_ANCHORS:
        return FIXED_ANCHORS[pid]
    body, gore, weapon = _px(cel_, "body"), _px(cel_, "gore"), _px(cel_, "weapon")
    allpix = {**body, **weapon, **gore}
    x0, y0, x1, y1 = bbox_of_pix(allpix)
    cx = (x0 + x1) // 2
    if pid == "torso_lower":
        hips = sorted(p for p, ch in gore.items() if ch == "R" and p[1] == y1)
        top = min(y for _x, y in body)
        return {"waist": [cx, top], "hip_far": list(hips[0]), "hip_near": list(hips[-1])}
    if pid in ("arm_near", "arm_far"):
        gy = min(y for _x, y in gore)
        xs = [x for x, y in gore if y == gy]
        return {"shoulder": [(min(xs) + max(xs)) // 2, gy]}
    if pid in ("leg_near", "leg_far"):
        gy = min(y for _x, y in gore)
        xs = [x for x, y in gore if y == gy]
        feet = [x for x, y in body if y == y1]
        return {"hip": [(min(xs) + max(xs)) // 2, gy], "foot": [(min(feet) + max(feet)) // 2, y1]}
    if pid == "ear":
        gx = max(x for x, _y in gore)
        ys = [y for x, y in gore if x == gx]
        return {"base": [gx, (min(ys) + max(ys)) // 2]}
    if pid == "club_stick":
        butt = max(((x, y) for (x, y), ch in weapon.items() if ch == "x"), key=lambda p: p[1])
        top_y = min(y for (_x, y), ch in weapon.items() if ch == "y")
        teeth = [p for p, ch in weapon.items() if ch == "y" and p[1] == top_y]
        return {"grip": list(butt), "break": [(min(t[0] for t in teeth) + max(t[0] for t in teeth)) // 2,
                                                min(t[1] for t in teeth)]}
    kind = STATIC_ANCHOR_KIND.get(pid)
    if kind == "centre_bottom":
        return {"centre_bottom": [cx, y1]}
    if kind == "origin":
        return {"origin": [x0, y1]}
    if kind == "start":
        return {"start": [x0, (y0 + y1) // 2]}
    return {"centre": [16, 16]}


def part_frame0_origin(pid: str):
    if pid in FIXED_ORIGINS:
        return FIXED_ORIGINS[pid]
    if pid == "arm_far":
        pid = "arm_near"
    if pid == "club_stick":
        sh = CELL_SHIFT_RECORD["club_stick"]
        return [CELL_PIVOT[0] - sh[0] - CLUB_SHIFT[0], CELL_PIVOT[1] - sh[1] - CLUB_SHIFT[1]]
    if pid in CELL_SHIFT_RECORD:
        sh = CELL_SHIFT_RECORD[pid]
        return [CELL_PIVOT[0] - sh[0], CELL_PIVOT[1] - sh[1]]
    return None          # gibs, drops and decals are not parts of the live pose


def part_entries(grid: dict) -> list[dict]:
    out = []
    for i, pid in enumerate(PART_IDS):
        frame = grid["frames"][i]["cels"]
        cls, material, mass, spin, settle, bounce, swap, stain, trail, ttl, presets = PART_META[pid]
        has_gore = any(ch != "." for row in frame["gore"] for ch in row)
        entry = {"id": pid, "frame": i, "cell": list(CELL), "pivot": list(CELL_PIVOT),
                 "anchors": part_anchors(pid, frame), "frame0_origin": part_frame0_origin(pid),
                 "class": cls, "material": material, "presets": presets, "gore_layer": has_gore,
                 "mass": mass, "spin": spin, "settle": settle, "bounce": bounce}
        if swap:
            entry["swap_on_first_contact"] = swap
        if pid == "drop_blood":
            entry["on_contact"] = "decal_splat_s"
        if stain:
            entry["stain"] = stain
        if trail:
            entry["trail"] = trail
        entry["ttl"] = ttl
        out.append(entry)
    return out


PROVENANCE_ORIGIN = ("GOB-02-DEATH goblin A death art at intensity preset 4 (docs/02-art/gob-02-death-spec.md); "
                     "pixel-grid-v1 source written by scripts/author_gob02_death.py from GOB-01 A rectangles, shape "
                     "rules and literal rows; no image-model output, no tracing")
PROVENANCE_GEN = ("Design, spec and hero keys: Claude Opus. In-betweens, parts, manifests, export, validators and "
                  "evidence: Claude Sonnet. Review pending (separate top-tier agent)")
LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
           "no game/asset-pack distribution rights")


def manifest_base(mid: str, stem: str, grid: Path, ase: Path, frame, pivot, tags: list[str]) -> dict:
    return {
        "schema_version": 1, "id": mid, "type": "character_animation",
        "source_path": str(ase.relative_to(ROOT)), "grid_source": str(grid.relative_to(ROOT)),
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "goblin-death-floor01-v0", "frame_size": list(frame), "pivot": list(pivot),
        "animation_tags": tags, "required_alpha": True,
        "owner": "AI-authored enemy death art (fan-concept layer: common Floor 1 goblin; runtime role IP-neutral small melee enemy)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {"origin": PROVENANCE_ORIGIN, "generator": PROVENANCE_GEN, "generated_art": True,
                       "input_reference_ids": ["floor-01-opening-concept-references-v1"]},
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
    }


def pose_manifest() -> dict:
    grid = poses_grid()
    m = manifest_base("goblin_a_death_h36_candidate_v1", "goblin_a_death_h36", POSES, POSES_ASE, CANVAS, PIVOT,
                      [t[0] for t in POSE_TAGS])
    frames = []
    for i, (ticks, author) in enumerate(zip(POSE_TICKS, POSE_AUTHOR)):
        tag = next(n for n, a, b in POSE_TAGS if a <= i <= b)
        frames.append({"index": i, "tag": tag, "ticks": ticks, "duration_ms": grid["frames"][i]["duration_ms"],
                       "author": author, "anchors": POSE_ANCHORS[i]})
    m["death_poses"] = {
        "spec": "docs/02-art/gob-02-death-spec.md", "variant": "A", "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS,
        "status": "executor pass complete; independent review pending; awaiting owner review; not wired into any Bevy binary",
        "facing": "right", "mirror": "x' = 56 - x around the pivot column, anchors included",
        "key_frames": [0, 2, 3, 5], "key_fixture": "tests/fixtures/gob-02-death_keys.json",
        "layers": LAYERS, "default_preset": 4,
        "playback": {"crush_contact": "once", "crushed_remains": "hold", "decap_contact": "once",
                     "headless_collapse": "hold"},
        "frames": frames,
    }
    return m


def parts_manifest() -> dict:
    grid = parts_grid()
    m = manifest_base("goblin_a_death_parts_v1", "goblin_a_death_parts", PARTS, PARTS_ASE, CELL, CELL_PIVOT, PART_IDS)
    m["death_parts"] = {
        "spec": "docs/02-art/gob-02-death-spec.md", "variant": "A", "cell": list(CELL), "pivot": list(CELL_PIVOT),
        "layers": LAYERS, "mirror": "x' = 32 - x around the cell pivot",
        "status": "executor pass complete; independent review pending; not wired into any Bevy binary",
        "key_frames": [0, 1, 2], "key_fixture": "tests/fixtures/gob-02-death-parts_keys.json",
        "preset_filter": {"1": "clean parts and head", "2": "wound (gore muted), clean, blood drops and s decals",
                          "3": "wound, clean, gib, blood (pools 1 and 2)", "4": "all"},
        "parts": part_entries(grid),
    }
    return m


def write_manifests() -> list[Path]:
    paths = []
    for name, m in (("goblin_a_death_h36.json", pose_manifest()), ("goblin_a_death_parts.json", parts_manifest())):
        path = ROOT / "assets/manifests" / name
        path.write_text(json.dumps(m, indent=2) + "\n", encoding="utf-8")
        paths.append(path)
    return paths


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "scale", "manifests"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "manifests":
        sys.path.insert(0, str(ROOT))
        parts_grid()                                   # fills the frame-0 -> cell translations
        for path in write_manifests():
            print(f"wrote {path.relative_to(ROOT)}")
        return 0
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
