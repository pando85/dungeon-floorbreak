#!/usr/bin/env python3
"""ENG-01 authoring record: Goblin Engineer palette, part blocks and the idle hero key.

Usage:

    python3 scripts/author_eng01.py keys [--check]
    python3 scripts/author_eng01.py key0-scale

``keys`` (re)writes ``assets/source/characters/goblin/engineer/eng_01.grid.json`` from the literal key-0 rows below
(see ``docs/02-art/eng-01-spec.md`` sections 3, 6 and 7). Design stage: the grid holds only the hero key, frame 0
(``idle``), frozen in ``tests/fixtures/eng-01_keys.json``. The executor appends frames 1 to 17 in this script from the
part blocks (``POT``, ``HEAD``, ``TORSO``, ``LEGS``, ``WRENCH``) and the walk contact plan, extends ``TAGS``/``TICKS``,
the config and the fixture, and writes the manifest; frame 0 must stay byte-identical.

Construction (no concept pixels traced or sampled):

* rows 26 to 55 are the GOB-01 variant A idle body (``goblin_a_h36.grid.json`` frame 0, layer ``body``) unchanged,
  apart from the closed near fist (the pineapple stick is gone);
* rows 16 to 25 are the DOZ-01 engineer rider's saucepan, handle and pot-shadowed head row
  (``dozer_riders_h36.grid.json`` frame 2, ``engineer_hold``, rows 25 to 34), moved up 9 rows: the rider's seat offset
  undone, so the standalone sprite is the same individual pixel for pixel from the pot rim up to the chin;
* new: the ear tip pushed out under the handle, a rag belt with an iron buckle on row 42 (the stick strap's row), and
  the wrench stowed jaw-up in a belt loop at the back hip (layer ``prop``; it is never held or swung).

Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENG_DIR = ROOT / "assets/source/characters/goblin/engineer"
GRID = ENG_DIR / "eng_01.grid.json"
CANVAS = (64, 56)
PIVOT = (28, 55)
H = 40                                   # idle opaque height: the GOB-01 36 px body plus 4 rows of pot above the crown
LAYERS = ["body", "prop"]
TICK_HZ = 60

# eng-floor01-v0 (17 of 20). The first eleven are goblin-floor01-v0 keys with the same hex (family read), the iron ramp
# 2 to 7 is the DOZ-01 rider ramp with the same keys and hex (the pot is the rider's pot). Pot, buckle and wrench share
# the iron ramp, so the engineer's metal reads as one kit. No wood, pineapple or leaf keys: no stick.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "rag_dark", "#2c1e28ff", False),
    ("t", "rag_mid", "#4c3436ff", False),
    ("u", "rag_light", "#73524aff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
    ("2", "iron_2", "#2c3046ff", False),
    ("3", "iron_3", "#3f445eff", False),
    ("4", "iron_4", "#575e7aff", False),
    ("5", "iron_5", "#78809aff", False),
    ("6", "iron_6", "#a3abbfff", False),
    ("7", "iron_glint", "#dde2ecff", True),
]
KEYS = [p[0] for p in PALETTE]
TAGS = [("idle", 0, 0)]                  # design stage; the executor extends to the section 4 table of the spec
TICKS = [14]                             # 60 Hz
KEY_FRAMES = (0,)

# Part blocks of key 0 (inclusive x0, y0, x1, y1). In-betweens and keys move these as whole blocks wherever the spec
# says "offset"; nothing is rotated or rescaled.
POT = (13, 16, 42, 24)                   # saucepan, handle and rim lip (rider rows 25 to 33)
HEAD = (16, 25, 44, 33)                  # ear tip, brow, eyes, snout, teeth, jaw and chin outline
TORSO = (19, 30, 41, 45)                 # rag cape, neck, belly, near arm and fist, belt and rag skirt
LEGS = (19, 46, 38, 55)                  # far leg (deep ramp) and near leg (light ramp), feet on row 55
WRENCH = (14, 35, 18, 42)                # prop layer: jaw up, shaft down into the belt loop at the back hip

# --------------------------------------------------------------------------- frame 0: idle hero key (literal rows)
KEY0 = {
    16: "...........................KKKKKKKKKKKKKK.......................",
    17: "..........................K666666666666KKK......................",
    18: "..........................K56665555544432K......................",
    19: "..............KKKK........K56765555544432K......................",
    20: ".............K55K5KKKK....K56665555544432K......................",
    21: ".............K33335555KKKKK56665555544432K......................",
    22: "..............KKKK33335555K56665555544432K......................",
    23: "..................KKKK33336666666666555544K.....................",
    24: "..................KKKKKKKKKKKKKKKKKKKKKKKK......................",
    25: "................KKllaaKKKKsmmmmmmmmaaKKmmK......................",
    26: ".................KKssaaKKKsmmmmmmseeKaeKlmK.....................",
    27: "...................KKKKKKKlmmmmmmmsaammmlmmK....................",
    28: "........................KKlmmmmmssKKKKmmsmmsK...................",
    29: "........................KKKlmmmmsKwwowKKmsssK...................",
    30: "......................KKuuuKlmmsKooooowKKKKK....................",
    31: ".....................KuuuutrKssssKKKKKKsK.......................",
    32: "....................KuutttttrKsssssssssK........................",
    33: "....................KuttttttrrKKKKKKKKK.........................",
    34: "....................KuttttttrrKlmKmmmsK.........................",
    35: "...............K..KKutttrtttrKlmsKmmmmsK........................",
    36: "..............K5..4KuttttrttrKlmsKmmmsK.........................",
    37: "..............K6KK4KutttttrtrKlmsKmmmKKKK.......................",
    38: "..............K6554KuttttttrrKlmsKmmKllmmK......................",
    39: "...............K65KKutttttrrrKlmmKKmmmmmK.......................",
    40: "................K5KKrtrKrtrrrKslmmmsssK.........................",
    41: ".................K4KrrKKtrrKrrKssssKK...........................",
    42: ".................KKKuutuutuutuK56KuK............................",
    43: "...................KrrrrrrrtrrrrrrK.............................",
    44: "...................KrrrrrrtrrrrtrrK.............................",
    45: "....................KrrKrrtKKrrtKrrK............................",
    46: "....................KKK.KKK..KKK.KKK............................",
    47: ".....................KmaK.....KlsK..............................",
    48: ".....................KmaK.....KlsK..............................",
    49: "....................KmmaK.....KlmmK.............................",
    50: ".....................KmaK......KlmK.............................",
    51: ".....................KmaK......KlmK.............................",
    52: "....................KKmsmK...KlmmlKK............................",
    53: "...................KmmsssmK.KlmmmmllKK..........................",
    54: "...................KmaaaaaK.KlssssssllK.........................",
    55: "....................KKKKKK...KKKKKKKKK..........................",
}


def in_box(x: int, y: int, box: tuple[int, int, int, int]) -> bool:
    x0, y0, x1, y1 = box
    return x0 <= x <= x1 and y0 <= y <= y1


def frame_idle0() -> dict[str, list[str]]:
    """Key 0 split into the two disjoint layers: the wrench box left of the cape outline (x <= 18) is ``prop``."""
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    for y, row in KEY0.items():
        if len(row) != w:
            raise ValueError(f"row {y} is {len(row)} px wide")
        for x, ch in enumerate(row):
            if ch == ".":
                continue
            if ch not in KEYS:
                raise ValueError(f"unknown key {ch!r} at ({x}, {y})")
            layer = "prop" if in_box(x, y, WRENCH) else "body"
            cels[layer][y][x] = ch
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    frames = [frame_idle0()]
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": cels} for t, cels in zip(TICKS, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def write_key0_scale(out: Path = ROOT / "docs/evidence/eng-01/key0-scale.png") -> Path:
    """Key 0 at 1x on lit Floor 1 next to Carl, the GOB-01 goblin A and the Bomb Bard (key 0), plus a 4x copy."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    goblin = grid_images(load_grid_file(ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"))[0]
    bard = grid_images(load_grid_file(ROOT / "assets/source/characters/goblin/bard/bard_01.grid.json"))[0]
    eng = grid_images(load_grid_file(GRID))[0]
    entries = [(carl, carl_piv, CARL_X, 0)]
    for i, img in enumerate((goblin, bard, eng)):
        f_img, f_piv = flip(img, tuple(PIVOT))
        entries.append((f_img, f_piv, CARL_X + 70 + 50 * i, 0))
    scene = Floor().render(entries, 1)
    panel_w = eng.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, eng.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = eng.resize((panel_w, eng.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 0), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "key0-scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    text = dump(frames_grid())
    if args.check:
        same = GRID.is_file() and GRID.read_text(encoding="utf-8") == text
        print(f"{GRID.name} up to date" if same else f"{GRID.name} differs from the generator")
        return 0 if same else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    print(f"wrote {GRID.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
