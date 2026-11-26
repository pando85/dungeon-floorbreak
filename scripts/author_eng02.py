#!/usr/bin/env python3
"""ENG-02 authoring record: the Goblin Engineer's get-up keys and the pot rules of his death set.

Usage:

    python3 scripts/author_eng02.py keys [--check]
    python3 scripts/author_eng02.py gate OUT.png

``keys`` (re)writes ``assets/source/characters/goblin/engineer/eng_02_keys.grid.json`` (and its ``.ase``) from the
rule-composed get-up sit (key grid frame 0, ENG-01 atlas frame 18) and the literal get-up shove rows below (key grid
frame 1, ENG-01 atlas frame 19). Both are frozen in ``tests/fixtures/eng-02_keys.json``. See
``docs/02-art/eng-02-spec.md`` sections 3 to 6.

Design stage: the key grid holds only these two frames. The executor appends them byte-identical to the ENG-01 atlas
as frames 18 and 19 (tag ``getup``), draws the two ram-run frames 20 and 21, and builds the death poses, the slammed
sheet and the parts sheet from the GOB-02 A and CF02 slammed A cels with the rules of this module (``pot_part``,
``flat_pot``, ``belt_recolour``) and of the spec.

Construction (no concept pixels traced or sampled):

* get-up sit (frame 18): the GOB-01 variant A get-up frame 23 body (``goblin_a_h36.grid.json``, layer ``body``), the
  rope belt recoloured to the ENG-01 rag belt with its iron buckle, and the ENG-01 ``POT`` block moved (-4, +11) so the
  pan sits jammed down over the eyes, rim on the snout, the teeth showing under it; no wrench (behind the hip);
* get-up shove (frame 19, hero key): the GOB-01 A get-up frame 24 crouch (legs rows 49 to 55, the bald crown rows 25 to
  28 that the lifted pot uncovers), the ENG-01 ``TORSO``, ``ARM``, ``WRENCH`` and ``HEAD`` blocks moved (0, +4), the
  ``POT`` block moved (-3, 0) and the far arm, new, raised behind the head with the fist on the pot handle: he levers
  the pot up off his eyes by its handle while he rises.

Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as eng01  # noqa: E402
from author_eng01 import PARTS, Canvas, gob_body  # noqa: E402

ENG_DIR = ROOT / "assets/source/characters/goblin/engineer"
GRID = ENG_DIR / "eng_02_keys.grid.json"
ASE = ENG_DIR / "eng_02_keys.ase"
CANVAS = eng01.CANVAS
PIVOT = eng01.PIVOT
LAYERS = eng01.LAYERS
TICK_HZ = 60
ATLAS_INDEX = (18, 19)                    # where the executor appends the two keys in the ENG-01 atlas
TICKS = (10, 12)                          # the get-up window: GETUP_TICKS / q04_getup_ticks = 22
TAGS = [("getup", 0, 1)]

# eng-death-floor01-v0 (20 of 20): the 17 eng-floor01-v0 keys, unchanged, plus the three shared GOB-02 gore keys with
# the same hex. The death poses, the slammed sheet and the parts sheet use it; the get-up keys use only the first 17.
GORE_KEYS = [
    ("R", "red_mid", "#b8322eff", False),
    ("B", "blood_dark", "#6e1a22ff", False),
    ("P", "blood_wet", "#e2434bff", False),
]
DEATH_PALETTE = eng01.PALETTE + GORE_KEYS

# --------------------------------------------------------------------------- key grid frame 1: the shove (literal rows)
KEY_SHOVE = {
    16: "........................KKKKKKKKKKKKKK..........................",
    17: ".......................K666666666666KKK.........................",
    18: ".......................K56665555544432K.........................",
    19: "...........KKKK........K56765555544432K.........................",
    20: "..........K55K5KKKK....K56665555544432K.........................",
    21: "..........K3333mmm5KKKKK56665555544432K.........................",
    22: "...........KKKKsss35555K56665555544432K.........................",
    23: "..............KaaaK33336666666666555544K........................",
    24: "...............KssKKKKKKKKKKKKKKKKKKKKK.........................",
    25: "...............KssK..........KKlllllK...........................",
    26: "...............KssK.......KKKllmmmmmmKK.........................",
    27: "..............KsssK......KllmmmmmmmmmmmK........................",
    28: "..............KssKKKKK...KlmmmmmmmmmKKmmK.......................",
    29: "..............KsKKllaaKKKKsmmmmmmmmaaKKmmK......................",
    30: "..............KasKKssaaKKKsmmmmmmseeKaeKlmK.....................",
    31: "...............KamKKKKKKKKlmmmmmmmsaammmlmmK....................",
    32: "................KammK...KKlmmmmmssKKKKmmsmmsK...................",
    33: ".................KaamK..KKKlmmmmsKwwowKKmsssK...................",
    34: "..................KKamKKuuuKlmmsKooooowKKKKK....................",
    35: "....................KKuuuutrKssssKKKKKKsK.......................",
    36: "....................KuutttttrKsssssssssK........................",
    37: "....................KuttttttrrKKKKKKKKK.........................",
    38: "....................KuttttttrrKlmKmmmsK.........................",
    39: "...............K..KKutttrtttrKlmsKmmmmsK........................",
    40: "..............K5..4KuttttrttrKlmsKmmmsK.........................",
    41: "..............K6KK4KutttttrtrKlmsKmmmKKKK.......................",
    42: "..............K6554KuttttttrrKlmsKmmKllmmK......................",
    43: "...............K65KKutttttrrrKlmmKKmmmmmK.......................",
    44: "................K5KKrtrKrtrrrKslmmmsssKK........................",
    45: ".................K4KrrKKtrrKrrKssssKKK..........................",
    46: ".................KKKuutuutuutuK56KuK............................",
    47: "...................KrrrrrrrtrrrrrrK.............................",
    48: "...................KrrrrrrtrrrrtrrK.............................",
    49: "....................KrrrrrtKKrrtrrrKKK..........................",
    50: ".....................KKKKKK..KKKKKKKaaK.........................",
    51: "...................................KaaK.........................",
    52: ".................KK.............KKKKmaKK........................",
    53: ".................KlK......KKKKKKKllKKmaaKKK.....................",
    54: ".................KllKKKKKKllllllllmmKKmmaaaK....................",
    55: ".................KsssssssssssssssssK..KKKKKKK...................",
}
WRENCH_SHOVE = (14, 39, 18, 46)           # ENG-01 WRENCH box moved (0, +4): the same body/prop split rule as key 0


def frame_shove() -> dict[str, list[str]]:
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    for y, row in KEY_SHOVE.items():
        if len(row) != w:
            raise ValueError(f"row {y} is {len(row)} px wide")
        for x, ch in enumerate(row):
            if ch == ".":
                continue
            if ch not in eng01.KEYS:
                raise ValueError(f"unknown key {ch!r} at ({x}, {y})")
            cels["prop" if eng01.in_box(x, y, WRENCH_SHOVE) else "body"][y][x] = ch
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}


# --------------------------------------------------------------------------- rules shared with the executor
BELT_BUCKLE = ("5", "6")


def belt_recolour(pix: dict, buckle_at: tuple[int, int] | None = None) -> dict:
    """GOB rope belt to the ENG-01 rag belt: ``x``/``y`` become ``u``, every third pixel along the row ``t`` (the
    ENG-01 key 0 row 42 rhythm ``uut``); ``buckle_at`` (the left pixel) gets the 2 px iron buckle ``56``."""
    out = dict(pix)
    for (x, y), ch in pix.items():
        if ch in ("x", "y"):
            out[(x, y)] = "u" if x % 3 else "t"
    if buckle_at:
        bx, by = buckle_at
        out[(bx, by)], out[(bx + 1, by)] = BELT_BUCKLE
    return out


def frame_sit() -> dict[str, list[str]]:
    """Get-up sit, key grid frame 0 (atlas 18): GOB-01 A frame 23 body, the ENG belt, the pot jammed over the eyes."""
    c = Canvas()
    c.paint(belt_recolour(gob_body(23), buckle_at=(29, 50)))
    c.block("POT", -4, 11)
    return c.cels()


def pot_part() -> dict:
    """The ``pot`` part cel (32 x 32, pivot (16, 16)): the ENG-01 ``POT`` block (x 13 to 42, rows 16 to 24), whole and
    unrotated, moved (-12, -4) so the block sits in cell columns 1 to 30, rows 12 to 20. The presenter spins it."""
    return {(x - 12, y - 4): ch for (x, y), ch in PARTS["POT"].items()}


FLAT_ROWS = (16, 17, 23, 24)              # the pan's rim top, light rim band, lower band and lip


def flat_pot() -> dict:
    """The flattened pot of the crush and the stomp: a 4-row pancake of the ``POT`` block at the block's own width (iron
    does not stretch). Rows 16, 17, 23 and 24 of the pan are stacked; the handle keeps only its row 21, laid on the
    third pancake row left of the pan. Origin: the block's top-left (13, 16) maps to (0, 0)."""
    out: dict = {}
    pot = PARTS["POT"]
    pan_left = 26                                        # the pan's left outline column in key 0
    for i, src in enumerate(FLAT_ROWS):
        for (x, y), ch in pot.items():
            if y == src and x >= pan_left - (2 if src >= 23 else 0):
                out[(x - 13, i)] = ch
    for (x, y), ch in pot.items():
        if y == 21 and x < pan_left:
            out.setdefault((x - 13, 2), ch)
    for (x, y), ch in list(out.items()):                 # the handle line gets its outline above and below
        if y == 2 and ch != "K":
            out.setdefault((x, 1), "K")
            out.setdefault((x, 3), "K")
    return out


# --------------------------------------------------------------------------- grid
def build_frames() -> list[dict]:
    return [frame_sit(), frame_shove()]


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    frames = build_frames()
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in eng01.PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": cels} for t, cels in zip(TICKS, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# --------------------------------------------------------------------------- gate image (review only, not committed)
def write_gate(out: Path) -> Path:
    """ENG-01 idle (0) and down (17) next to the two get-up keys, plus the pot part and the flattened pot rule previews:
    4x colour, 4x black silhouette, and a 1x strip on the Floor 1 ground tone."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import grid_images, label, load_grid_file

    eng = grid_images(load_grid_file(eng01.GRID))
    keys = grid_images(frames_grid())
    rgba = {k: tuple(int(c[i:i + 2], 16) for i in (1, 3, 5, 7)) for k, _n, c, _a in DEATH_PALETTE}

    def from_pix(pix: dict, size=(32, 32), at=(0, 0)) -> Image.Image:
        im = Image.new("RGBA", size, (0, 0, 0, 0))
        for (x, y), ch in pix.items():
            im.putpixel((x + at[0], y + at[1]), rgba[ch])
        return im

    cells = [("ENG-01 idle 0", eng[0]), ("ENG-01 down 17", eng[17]), ("18 getup sit", keys[0]),
             ("19 getup shove (hero)", keys[1])]
    extras = [("pot part (rule)", from_pix(pot_part())), ("flat pot (rule)", from_pix(flat_pot(), (32, 8)))]
    k = 4
    bg, sil_bg, floor = (58, 34, 30, 255), (236, 232, 240, 255), (74, 68, 100, 255)
    cw, ch = 64 * k, 56 * k
    pad, top = 12, 22
    width = len(cells) * (cw + pad) + 2 * (32 * k + pad) + pad
    height = top + ch + top + ch + top + 56 + 16
    sheet = Image.new("RGBA", (width, height), (24, 23, 34, 255))
    d = ImageDraw.Draw(sheet)
    for i, (name, im) in enumerate(cells):
        x = pad + i * (cw + pad)
        label(d, (x, 4), name)
        sheet.paste(Image.new("RGBA", (cw, ch), bg), (x, top))
        big = im.resize((cw, ch), Image.NEAREST)
        sheet.paste(big, (x, top), big)
        sil = Image.new("RGBA", (cw, ch), sil_bg)
        mask = big.split()[3]
        sil.paste(Image.new("RGBA", (cw, ch), (0, 0, 0, 255)), (0, 0), mask)
        sheet.paste(sil, (x, top + ch + top))
    x = pad + len(cells) * (cw + pad)
    for name, im in extras:
        label(d, (x, 4), name)
        sheet.paste(Image.new("RGBA", (32 * k, 32 * k), bg), (x, top))
        big = im.resize((im.width * k, im.height * k), Image.NEAREST)
        sheet.paste(big, (x, top), big)
        x += 32 * k + pad
    y1 = top + ch + top + ch + top - 16
    label(d, (pad, y1), "1x on the Floor 1 ground tone: idle, down, sit, shove")
    strip = Image.new("RGBA", (64 * 4 + 24, 56), floor)
    for i, (_n, im) in enumerate(cells):
        strip.paste(im, (i * 64 + 6 * i, 0), im)
    sheet.paste(strip, (pad, y1 + 16))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "gate"])
    parser.add_argument("out", nargs="?", help="gate: output PNG path")
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "gate":
        if not args.out:
            parser.error("gate needs an output path")
        print(f"wrote {write_gate(Path(args.out))}")
        return 0
    text = dump(frames_grid())
    if args.check:
        same = GRID.is_file() and GRID.read_text(encoding="utf-8") == text
        print(f"{GRID.name} up to date" if same else f"{GRID.name} differs from the generator")
        return 0 if same else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    import pixel_grid_to_ase as pg  # noqa: E402
    pg.write_ase(frames_grid(), ASE)
    print(f"wrote {GRID.relative_to(ROOT)}, {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
