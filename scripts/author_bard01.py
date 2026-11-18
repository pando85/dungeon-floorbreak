#!/usr/bin/env python3
"""BARD-01 authoring record: Goblin Bomb Bard palette, reusable prop stamps and the idle hero key.

Usage:

    python3 scripts/author_bard01.py keys [--check] [--preview PNG]

``keys`` (re)writes ``assets/source/characters/goblin/bard/bard_01.grid.json`` from literal pixel
segments (see ``docs/02-art/bard-01-spec.md`` sections 3, 6 and 7). Design stage: the grid holds only
the hero key, frame 0 (``idle``), which is frozen in ``tests/fixtures/bard-01_keys.json``. The
executor appends frames 1 to 15 in this script (same segment style, same stamps), extends the grid
tags and writes the manifest; frame 0 must stay byte-identical.

Layers (disjoint): ``body`` (goblin, headwrap, goggles, scarf, pack, belt, boots) and ``prop``
(horn, bombs, fuse spark, music notes). Drawing order: body, then prop (prop pixels clear the body
under them), then the ``front`` body pass (hands that grip the horn clear the prop under them).

The head construction (ear, brow, eyes, nose, teeth) is the GOB-01 variant A idle head
(``goblin_a_h36.grid.json`` frame 0) moved up 3 rows, under a new headwrap and goggles, with the jaw
dropped into an open grin. No concept pixels are traced or sampled; colours are chosen by eye.
``--check`` compares instead of writing (exit 1 on a difference). Presentation-only art tooling;
nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BARD_DIR = ROOT / "assets/source/characters/goblin/bard"
GRID = BARD_DIR / "bard_01.grid.json"
CANVAS = (64, 56)
PIVOT = (28, 55)
LAYERS = ["body", "prop"]

# bard-floor01-v0 (19 of 20). Skin, leather, eyes, teeth, mouth and the red pair are goblin-floor01-v0
# keys with the same hex (family read); brass, iron and the spark core are new. The bombs are a cool
# violet-grey iron so they read as metal, not as the plum outline; the brass is warm but darker and
# browner than the brick highlight; the spark core is the only near-white warm key and appears only
# in the tell and the throw.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "skin_deep", "#263a26ff", False),
    ("s", "skin_shade", "#46682eff", False),
    ("m", "skin_mid", "#7a9e36ff", False),
    ("l", "skin_light", "#b2cc56ff", False),
    ("r", "leather_dark", "#2c1e28ff", False),
    ("t", "leather_mid", "#4c3436ff", False),
    ("u", "leather_light", "#73524aff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
    ("R", "scarf_red", "#b8322eff", False),
    ("B", "scarf_red_dark", "#6e1a22ff", False),
    ("n", "brass_dark", "#6a4a1eff", False),
    ("b", "brass_mid", "#b8862eff", False),
    ("h", "brass_light", "#f0cf6aff", True),
    ("q", "iron_dark", "#3a3245ff", False),
    ("Q", "iron_light", "#766c86ff", False),
    ("Y", "spark_core", "#fff4b8ff", True),
]
KEYS = [p[0] for p in PALETTE]
TAGS = [("idle", 0, 0)]          # design stage; the full tag table is in bard_01.pixelart.json
TICKS = [14]                     # frame 0 of idle: 14 ticks at 60 Hz
KEY_FRAMES = (0,)                # hero key; the executor extends to (0, 3, 7, 8, 10, 12, 15)

# --------------------------------------------------------------------------- reusable stamps
# Iron bomb, 7 x 7, brass fuse cap in the top outline (cap at stamp x 3, row 0). Highlight upper left.
BOMB7 = [
    "..KbK..",
    ".KQqqK.",
    "KQqqqqK",
    "KqqqqqK",
    "KqqqqqK",
    ".KqqqK.",
    "..KKK..",
]
# Small belt bomb, 5 x 5 (cap at stamp x 2, row 0).
BOMB5 = [
    ".KbK.",
    "KQQqK",
    "KqqqK",
    "KqqqK",
    ".KKK.",
]
# Lit fuse spark (tell and throw only), 3 x 3 around the fuse tip: Y core, e halo, R ember.
SPARK = [
    "e.R",
    ".Ye",
    "R.e",
]
# Music note (horn toot, frame 7 only), 3 x 4, tooth white so it reads as a sound glyph, not as fire.
NOTE = [
    ".ww",
    ".w.",
    "ww.",
    "ww.",
]
# Horn bell, idle orientation (axis rising about 15 degrees forward), rows 29-41, x 44-53 when drawn at
# (44, 29). Rim column 'h', outer cone 'b' above the axis and 'n' below it.
BELL_IDLE = [
    ".......KK.",
    "......KhhK",
    ".....KhbhK",
    "....KhbbhK",
    "...KhbbbhK",
    ".KKhbbbnhK",
    "KhbbbbnnhK",
    "bbbbnnnnhK",
    "nnnnnnnnhK",
    "KKnnnnnnhK",
    "..KKnnnnhK",
    "....KKnnhK",
    "......KKKK",
]


class Canvas:
    def __init__(self) -> None:
        w, h = CANVAS
        self.cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}

    def put(self, layer: str, x: int, y: int, ch: str) -> None:
        """Set one pixel on ``layer`` and clear the same pixel on every other layer (layers stay disjoint)."""
        if ch == ".":
            return
        if ch not in KEYS:
            raise ValueError(f"unknown key {ch!r}")
        w, h = CANVAS
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel ({x}, {y}) outside the {w} x {h} frame")
        for name, rows in self.cels.items():
            rows[y][x] = ch if name == layer else "."

    def segs(self, layer: str, rows: dict[int, list[tuple[int, str]]]) -> None:
        for y, parts in rows.items():
            for x0, s in parts:
                for i, ch in enumerate(s):
                    self.put(layer, x0 + i, y, ch)

    def stamp(self, layer: str, x0: int, y0: int, rows: list[str]) -> None:
        for dy, s in enumerate(rows):
            for dx, ch in enumerate(s):
                self.put(layer, x0 + dx, y0 + dy, ch)

    def rows(self) -> dict[str, list[str]]:
        return {name: ["".join(r) for r in rows] for name, rows in self.cels.items()}


def horn_pipe(c: Canvas, x0: int, x1: int, top_at_x0: int, rise_every: int, band_x: int | None = None) -> None:
    """Two-row brass pipe ('b' over 'n') from x0 to x1, rising one row every ``rise_every`` px forward."""
    for x in range(x0, x1 + 1):
        top = top_at_x0 - (x - x0) // rise_every
        c.put("prop", x, top, "K" if x == band_x else "b")
        c.put("prop", x, top + 1, "K" if x == band_x else "n")


# --------------------------------------------------------------------------- frame 0: idle hero key
HEAD_IDLE = {  # headwrap, goggles, GOB-01 head moved up 3 rows, open grin
    16: [(30, "KKKKKK")],
    17: [(19, "K"), (29, "KBRRRRRK")],
    18: [(18, "KlKKK"), (28, "KBRRRRRRRK")],
    19: [(19, "KllaKKK"), (27, "KBRRRR"), (33, "bbKKbbK")],
    20: [(20, "KlsaaallBRRR"), (32, "bQwbbQwbK")],
    21: [(21, "KKssaammmmmm"), (33, "bbKKbbKK")],
    22: [(22, "KKsasmmmmmmmmaaKKmmK")],
    23: [(24, "KKsmmmmmmseeKaeKlmK")],
    24: [(25, "KlmmmmmmmsaammmlmmK")],
    25: [(24, "KKlmmmmmssKKKKmmsmmsK")],
    26: [(24, "KKKlmmmmsKwwowKKmsssK")],
    27: [(27, "KlmmsKooooowKKKKK")],
    28: [(27, "KssssKwoowKK")],
}
BODY_IDLE = {  # scarf, pack, slender upright torso, near arm, belt, shorts, legs, boots
    29: [(16, "KKKKKKKKKK"), (28, "KBBRKKKKKK")],
    30: [(16, "KuuuuuuutK"), (27, "KBBRRRRRRRK")],
    31: [(16, "KuuuuuuttK"), (26, "KtBRRRRRRBBK")],
    32: [(16, "KrrrbbrrrK"), (26, "KtuuuuKtRRBK")],
    33: [(16, "KutttbttrK"), (26, "KtKlmsKtRRBK")],
    34: [(16, "KuttttttrK"), (26, "KrKlmsKttRBK")],
    35: [(16, "KutttttrrK"), (26, "KrKlmsKtttBK")],
    36: [(16, "KutttttrrK"), (26, "KrKlmsKttttK")],
    37: [(16, "KrtttttrrK"), (26, "KrKmmsKttttK")],
    38: [(16, "KrrtttrrrK"), (26, "KrKlmmsKtttK")],
    39: [(16, "KrrrrrrrrK"), (26, "KrrKlmmsKttK")],
    40: [(16, "KKrrrrrrKK"), (26, "KrrrKlmmsKtK")],
    41: [(17, "KKKKKKKK"), (26, "KrrrrrrrrrK")],
    42: [(26, "KrrrrrrrrrK")],
    43: [(25, "KttrtttrtttK")],
    44: [(25, "KtrrtttrtttK")],
    45: [(25, "KrtKrrKtrKtK")],
    46: [(24, "KmaK"), (31, "KlmsK")],
    47: [(24, "KmaK"), (32, "KlmsK")],
    48: [(23, "KmaK"), (33, "KlmsK")],
    49: [(23, "KmaK"), (33, "KlmK")],
    50: [(22, "KmaK"), (33, "KlmK")],
    51: [(21, "KtutK"), (32, "KuutK")],
    52: [(21, "KtutK"), (32, "KutuK")],
    53: [(20, "KttuttK"), (31, "KuuttttK")],
    54: [(19, "KttttrrK"), (31, "KtttttrrK")],
    55: [(19, "KKKKKKKK"), (31, "KKKKKKKKK")],
}
FRONT_IDLE = {  # hands gripping the horn: near fist (skin ramp), far hand (shade only)
    36: [(40, "KK")],
    37: [(37, "KsssaK")],
    38: [(34, "KKK"), (39, "KsaK")],
    39: [(33, "KllsK"), (40, "KK")],
    40: [(33, "KmmsK")],
    41: [(33, "KssK")],
    42: [(34, "KK")],
}


def frame_idle0() -> dict[str, list[str]]:
    c = Canvas()
    c.segs("body", HEAD_IDLE)
    c.segs("body", BODY_IDLE)
    # pack bomb on top of the pack with an unlit fuse curling up and back; belt bomb at the back hip
    c.stamp("prop", 15, 22, BOMB7)
    for x, y, ch in ((18, 21, "t"), (17, 20, "t"), (16, 19, "t")):
        c.put("prop", x, y, ch)
    c.stamp("prop", 19, 41, BOMB5)
    # horn: mouthpiece cap, pipe across the body rising forward, one dark band, bell
    c.put("prop", 28, 41, "h")
    c.put("prop", 28, 42, "n")
    horn_pipe(c, 29, 43, 41, 3)
    c.stamp("prop", 44, 29, BELL_IDLE)
    c.segs("body", FRONT_IDLE)
    return c.rows()


FRAMES = [(frame_idle0, TICKS[0])]


# --------------------------------------------------------------------------- grid
def build_grid() -> dict:
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict(key=k, name=n, rgba=c, **({"accent": True} if a else {})) for k, n, c, a in PALETTE],
        "layers": list(LAYERS),
        "frames": [{"duration_ms": round(t * 1000 / 60), "cels": fn()} for fn, t in FRAMES],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def preview(grid: dict, out: Path, scale: int = 8) -> None:
    from PIL import Image  # preview only

    pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for p in grid["palette"]}
    w, h = grid["canvas"]
    n = len(grid["frames"])
    bg = (58, 34, 30, 255)  # warm brick tone, for the read only
    img = Image.new("RGBA", (w * n, h), bg)
    for i, fr in enumerate(grid["frames"]):
        for layer in grid["layers"]:
            for y, row in enumerate(fr["cels"].get(layer, [])):
                for x, ch in enumerate(row):
                    if ch != ".":
                        img.putpixel((i * w + x, y), pal[ch])
    big = img.resize((w * n * scale, h * scale), Image.NEAREST)
    sheet = Image.new("RGBA", (big.width + w * n * 2 + 8, big.height), bg)
    sheet.paste(big, (0, 0))
    sheet.paste(img, (big.width + 4, 0))
    sheet.paste(img.resize((w * n * 2, h * 2), Image.NEAREST), (big.width + 4, h + 8))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    k = sub.add_parser("keys")
    k.add_argument("--check", action="store_true")
    k.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    grid = build_grid()
    text = dump(grid)
    if a.preview:
        preview(grid, a.preview)
    if a.check:
        if not GRID.exists() or GRID.read_text() != text:
            print(f"{GRID.relative_to(ROOT)} differs from the authoring script", file=sys.stderr)
            return 1
        return 0
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text)
    print(f"wrote {GRID.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
