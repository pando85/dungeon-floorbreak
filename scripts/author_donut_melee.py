#!/usr/bin/env python3
"""ANIM-DONUT-02: Princess Donut melee and damage-reaction keys (40 px), deterministic authoring record.

Phase 1 (design): the eight key poses are built here from the owner-approved ANIM-DONUT-01 locomotion atlas
(`donut_h40_locomotion.grid.json`), which itself copies the approved 40 px master. Head, face, gear (tiara,
collar, charm), body and tail are the locomotion frames' own pixels, **translated only**; the only new pixels
are fore and hind legs and paws, placed by hand below. The in-between frames are copies of the nearest key,
left for the execution phase (see `docs/02-art/donut-melee-anim-02-spec.md`, section 6).

Usage:
    python3 scripts/author_donut_melee.py            # write the grid
    python3 scripts/author_donut_melee.py --check    # verify the committed grid matches
    python3 scripts/author_donut_melee.py --evidence # write docs/evidence/anim-donut-02/keys-*.png
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCO_PATH = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
OUT_PATH = ROOT / "assets/source/characters/donut/donut_h40_melee.grid.json"
EVIDENCE = ROOT / "docs/evidence/anim-donut-02"

LOCO = json.loads(LOCO_PATH.read_text(encoding="utf-8"))
W, H = LOCO["canvas"]                  # 72 x 44, unchanged
PIVOT = LOCO["pivot"]                  # (46, 43), unchanged
LAYERS = LOCO["layers"] + ["paw"]      # `paw`: a raised or reaching near forepaw drawn over the head and ruff
UPPER = ["head", "face", "gear"]       # the identity block (master pixels): translate only
TICK_HZ = 60

# ---------------------------------------------------------------------------
# Frame plan: (name, tag, ticks or None for hold, source: key name for copies)
# ---------------------------------------------------------------------------
FRAMES = [
    ("s0 cock", "swat", 4, None),
    ("s1 swat contact", "swat", 3, None),
    ("s2 follow-through", "swat", 4, "s1 swat contact"),
    ("s3 recover", "swat", 3, "s1 swat contact"),
    ("p0 crouch", "pounce", 10, None),
    ("p1 pounce flight", "pounce_air", None, None),
    ("p2 pin landing", "pounce_land", 8, None),
    ("p3 recover", "pounce_land", 12, "p2 pin landing"),
    ("h0 flinch", "hurt", 5, None),
    ("h1 recoil", "hurt", 11, "h0 flinch"),
    ("kb thrown", "knockback", None, None),
    ("d0 sploot impact", "knockdown", 6, None),
    ("d1 sploot sulk", "knockdown", 30, "d0 sploot impact"),
    ("g0 chest up", "getup", 10, "d0 sploot impact"),
    ("g1 chin up", "getup", 10, "d0 sploot impact"),
]
KEY_FRAMES = [i for i, f in enumerate(FRAMES) if f[3] is None]
HOLD_MS = 100


def ms(ticks):
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


# ---------------------------------------------------------------------------
# Pixel operations (whole-pixel translation and hand-placed parts only)
# ---------------------------------------------------------------------------

def part(text: str) -> list[str]:
    return [r for r in text.strip("\n").split("\n")]


def base(src: int) -> dict[str, list[list[str]]]:
    cels = LOCO["frames"][src]["cels"]
    return {l: [list(r) for r in cels.get(l, ["." * W] * H)] for l in LOCO["layers"]} | {
        "paw": [["."] * W for _ in range(H)]}


def shift(f: dict, layers: list[str], dx: int, dy: int) -> None:
    for l in layers:
        out = [["."] * W for _ in range(H)]
        for y, row in enumerate(f[l]):
            for x, ch in enumerate(row):
                if ch == ".":
                    continue
                if not (0 <= x + dx < W and 0 <= y + dy < H):
                    raise ValueError(f"{l}: pixel ({x},{y}) shifted off canvas")
                out[y + dy][x + dx] = ch
        f[l] = out


def clear(f: dict, layer: str, x0: int, y0: int, x1: int = W - 1, y1: int = H - 1) -> None:
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            f[layer][y][x] = "."


def put(f: dict, layer: str, x: int, y: int, text: str) -> None:
    for dy, row in enumerate(part(text)):
        for dx, ch in enumerate(row):
            if ch != ".":
                f[layer][y + dy][x + dx] = ch


# ---------------------------------------------------------------------------
# The eight keys
# ---------------------------------------------------------------------------

def key_s0() -> dict:
    """Swat cock: weight back 1 px, near forepaw raised beside the chin, paw curled (pads forward)."""
    f = base(15)                                   # land recovery: standing profile, chin up
    shift(f, UPPER + ["body", "tail"], -1, 1)
    clear(f, "legs_near", 44, 0)                    # the near fore becomes the swatting paw
    put(f, "paw", 59, 28, """
....KKKK.
...KWCCCK
...KCCCcK
...KcCcK.
..KCCcK..
..KCCcK..
.KCCcK...
.KCCcK...
KCCcK....
KCcK.....
""")
    return f


def key_s1() -> dict:
    """Swat contact: the paw has come down and forward to ankle height, claws out; chest leans in 1 px."""
    f = base(15)
    shift(f, UPPER + ["body", "tail"], 1, 0)
    clear(f, "legs_near", 44, 0)
    put(f, "paw", 56, 30, """
KKK...........
KCCK..........
KcCCK.........
.KcCCKKKKK....
..KcCCCCCCKK..
...KcCCCCCCCWK
....KccCCCCcWK
.....KKccccKK.
.......KKKK...
""")
    return f


def key_p0() -> dict:
    """Pounce crouch (the tell): head 2 px lower over the paws, plume dropped low behind the rump."""
    f = base(10)                                   # jump_start crouch: chest low, rear up, hinds bunched
    shift(f, UPPER, 0, 2)
    shift(f, ["tail"], -2, 6)
    return f


def key_p1() -> dict:
    """Pounce flight: long body, both forepaws thrown forward at chest height with claws out."""
    f = base(12)                                   # jump_rise: long tilted body, hinds trailing
    clear(f, "legs_near", 44, 0)
    clear(f, "legs_far", 44, 0)
    put(f, "paw", 56, 29, """
..........KKKK.
........KKCCCWK
......KKCCCCCWK
....KKcCCCCcKK.
..KKcCCCcKKK...
KKcCCcKK.......
KcccKK.........
""")
    put(f, "legs_far", 55, 34, """
KKK...........
KccKK.........
.KcccKKKKK....
..KKcccccccKK.
....KKcccccCK.
......KKKKKK..
""")
    shift(f, LAYERS, 0, -2)                        # airborne: nothing on rows 42-43
    return f


def key_p2() -> dict:
    """Pin landing: both forepaws slammed down in front, head pushed in, plume up (got it)."""
    f = base(10)
    shift(f, UPPER, 1, 1)
    shift(f, ["tail"], 2, -2)
    clear(f, "legs_near", 44, 0)
    clear(f, "legs_far", 44, 0)
    put(f, "legs_far", 50, 39, """
KKK.......
KccK......
.KcccKKK..
..KccccccK
..KKKKKKKK
""")
    put(f, "legs_near", 55, 37, """
KKK...........
KCCK..........
KcCCK.........
.KcCCKKKKK....
..KcCCCCCCKK..
...KcCCCCCCWK.
...KKcccccWK..
""")
    return f


def key_h0() -> dict:
    """Hurt flinch: eyes squeezed shut, head snapped back and up, near forepaw jerked off the floor."""
    f = base(14)                                   # land impact: the master blink face ("oof")
    shift(f, UPPER, -3, -2)
    shift(f, ["body"], -2, -1)
    shift(f, ["tail"], -3, -1)
    clear(f, "legs_near", 44, 0)
    clear(f, "legs_far", 44, 0)
    put(f, "legs_far", 50, 39, """
.cccK.
KcccK.
KcccK.
KccccK
KccccK
""")
    put(f, "legs_near", 52, 37, """
.KKK.
KCCCK
KCCcK
KcccK
.KKK.
""")
    return f


def key_kb() -> dict:
    """Knockback: thrown back and up, eyes shut, all four legs splayed (startled-cat starfish)."""
    f = base(14)
    clear(f, "legs_near", 0, 0)
    clear(f, "legs_far", 0, 0)
    shift(f, UPPER + ["body", "tail"], -4, -4)
    put(f, "legs_far", 20, 35, """
..KDDK
.KDDK.
KccK..
KccK..
.KK...
""")
    put(f, "legs_far", 56, 31, """
..KKKK..
KKccccCK
.KKcccCK
...KKKK.
""")
    put(f, "legs_near", 30, 36, """
KKK...
KOoKK.
.KOoK.
.KCCcK
.KcccK
..KKK.
""")
    put(f, "legs_near", 57, 35, """
KKK.......
KCCKKKKK..
.KcCCCCCWK
..KKccccWK
....KKKKK.
""")
    return f


def key_d0() -> dict:
    """Knockdown: belly-flop sploot, forepaws flat in front, hind legs flat behind, head upright, eyes shut.

    Donut never lies on her side: the head is master pixels and is never rotated, so she lands belly-down.
    """
    f = base(14)
    clear(f, "legs_near", 0, 0)
    clear(f, "legs_far", 0, 0)
    shift(f, ["body"], 0, 1)
    shift(f, UPPER, 0, 4)
    shift(f, ["tail"], -4, 8)
    put(f, "legs_near", 57, 40, """
...KKKKKK..
..KCCCCCCKK
KKcCCCCCCWK
KcccccccccK
""")
    put(f, "legs_near", 13, 40, """
......KKKKKKK.
..KKKKoOOOOoK.
KKCCCcoOOOoK..
KcccccKKKKK...
""")
    return f


KEYS = {
    "s0 cock": key_s0, "s1 swat contact": key_s1, "p0 crouch": key_p0, "p1 pounce flight": key_p1,
    "p2 pin landing": key_p2, "h0 flinch": key_h0, "kb thrown": key_kb, "d0 sploot impact": key_d0,
}


# ---------------------------------------------------------------------------
# Grid assembly
# ---------------------------------------------------------------------------

def cels_of(f: dict) -> dict[str, list[str]]:
    out = {}
    for l in LAYERS:
        rows = ["".join(r) for r in f[l]]
        if any(c != "." for r in rows for c in r):
            out[l] = rows
    return out


def build() -> dict:
    keys = {name: cels_of(fn()) for name, fn in KEYS.items()}
    frames = []
    for name, _tag, ticks, src in FRAMES:
        frames.append({"duration_ms": ms(ticks), "cels": copy.deepcopy(keys[src or name])})
    tags, start = [], 0
    for i, (_n, tag, _t, _s) in enumerate(FRAMES):
        if i + 1 == len(FRAMES) or FRAMES[i + 1][1] != tag:
            tags.append({"name": tag, "from": start, "to": i, "direction": "forward"})
            start = i + 1
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": PIVOT, "palette": LOCO["palette"],
            "layers": LAYERS, "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    lines = ["{", f'  "format": "{grid["format"]}",', f'  "canvas": {json.dumps(grid["canvas"])},',
             f'  "pivot": {json.dumps(grid["pivot"])},', '  "palette": [']
    lines += [f"    {json.dumps(p)}," for p in grid["palette"]]
    lines[-1] = lines[-1].rstrip(",")
    lines += ["  ],", f'  "layers": {json.dumps(grid["layers"])},', '  "frames": [']
    for fi, fr in enumerate(grid["frames"]):
        lines += ["    {", f'      "duration_ms": {fr["duration_ms"]},', '      "cels": {']
        names = list(fr["cels"])
        for li, l in enumerate(names):
            lines.append(f'        "{l}": [')
            rows = fr["cels"][l]
            lines += [f'          "{r}"' + ("," if ri + 1 < len(rows) else "") for ri, r in enumerate(rows)]
            lines.append("        ]" + ("," if li + 1 < len(names) else ""))
        lines += ["      }", "    }" + ("," if fi + 1 < len(grid["frames"]) else "")]
    lines += ["  ],", '  "tags": [']
    lines += [f"    {json.dumps(t)}" + ("," if i + 1 < len(grid["tags"]) else "") for i, t in enumerate(grid["tags"])]
    lines += ["  ]", "}"]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Evidence: keys at 1x on lit Floor 1 next to Carl and a GOB-01 goblin, plus a 4x copy and a 6x key sheet
# ---------------------------------------------------------------------------

SCENE = [  # (key frame, walking-line x, lift)
    (0, 420, 0), (4, 478, 0), (5, 540, 10), (6, 604, 0), (8, 664, 0), (10, 724, 8), (11, 786, 0), (1, 870, 0)]
CARL_AT, GOBLIN_AT = 362, 903


def evidence() -> list[Path]:
    sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor

    grid = json.loads(OUT_PATH.read_text(encoding="utf-8"))
    imgs = grid_images(grid)
    carl = load_grid_file(ROOT / "assets/source/characters/carl/carl_h52.grid.json")
    gob = load_grid_file(ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json")
    gimg, gpiv = flip(grid_images(gob)[0], tuple(gob["pivot"]))
    entries = [(grid_images(carl)[0], tuple(carl["pivot"]), CARL_AT, 0)]
    entries += [(imgs[f], tuple(PIVOT), x, lift) for f, x, lift in SCENE]
    entries.append((gimg, gpiv, GOBLIN_AT, 0))
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    out = []
    scene = Floor().render(entries, 1)
    p = EVIDENCE / "keys-scale.png"
    scene.save(p)
    out.append(p)
    p = EVIDENCE / "keys-scale-x4.png"
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(p)
    out.append(p)
    k, pad = 6, 8
    sheet = Image.new("RGB", ((W * k + pad) * 4 + pad, (H * k + pad + 16) * 2 + pad), (24, 20, 30))
    d = ImageDraw.Draw(sheet)
    for n, f in enumerate(KEY_FRAMES):
        x, y = pad + (n % 4) * (W * k + pad), pad + (n // 4) * (H * k + pad + 16)
        cellimg = Image.new("RGBA", (W, H), (52, 46, 60, 255))
        cellimg.alpha_composite(imgs[f])
        sheet.paste(cellimg.resize((W * k, H * k), Image.NEAREST), (x, y + 16))
        d.line([(x, y + 16 + (PIVOT[1] + 1) * k - 1), (x + W * k, y + 16 + (PIVOT[1] + 1) * k - 1)], fill=(120, 110, 90))
        d.rectangle([x + PIVOT[0] * k, y + 16 + PIVOT[1] * k, x + PIVOT[0] * k + k - 1, y + 16 + PIVOT[1] * k + k - 1],
                    outline=(80, 220, 120))
        d.text((x, y + 2), f"{f} {FRAMES[f][0]} ({FRAMES[f][1]})", fill=(235, 235, 235))
    p = EVIDENCE / "keys_x6.png"
    sheet.save(p)
    out.append(p)
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--evidence", action="store_true")
    a = ap.parse_args(argv)
    text = dump(build())
    if a.check:
        ok = OUT_PATH.read_text(encoding="utf-8") == text
        print("grid matches" if ok else "grid differs from the generator")
        return 0 if ok else 1
    if a.evidence:
        for p in evidence():
            print(p.relative_to(ROOT))
        return 0
    OUT_PATH.write_text(text, encoding="utf-8")
    print(OUT_PATH.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
