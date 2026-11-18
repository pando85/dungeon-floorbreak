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
# Frame plan: (name, tag, ticks or None for hold, is_key)
# ---------------------------------------------------------------------------
FRAMES = [
    ("s0 cock", "swat", 4, True),
    ("s1 swat contact", "swat", 3, True),
    ("s2 follow-through", "swat", 4, False),
    ("s3 recover", "swat", 3, False),
    ("p0 crouch", "pounce", 10, True),
    ("p1 pounce flight", "pounce_air", None, True),
    ("p2 pin landing", "pounce_land", 8, True),
    ("p3 recover", "pounce_land", 12, False),
    ("h0 flinch", "hurt", 5, True),
    ("h1 recoil", "hurt", 11, False),
    ("kb thrown", "knockback", None, True),
    ("d0 sploot impact", "knockdown", 6, True),
    ("d1 sploot sulk", "knockdown", 30, False),
    ("g0 chest up", "getup", 10, False),
    ("g1 chin up", "getup", 10, False),
]
KEY_FRAMES = [i for i, f in enumerate(FRAMES) if f[3]]
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
    # gate fix: paw 2 px up and 3 px forward so it breaks the head outline; K on its top and front edges,
    # forearm continued down into the ruff to the shoulder
    put(f, "paw", 62, 26, """
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
KCcK.....
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
    shift(f, UPPER, 0, 3)                          # gate fix: chin 1 px lower, within 2 px of the forepaws
    # gate fix: rump up 2 px. The body behind the chest (x < REAR) rises; the hind legs grow 2 rows by
    # repeating their top (haunch) row, so the paws stay on row 43.
    body = [["."] * W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if f["body"][y][x] != ".":
                body[y - 2 if x < P0_REAR else y][x] = f["body"][y][x]
    f["body"] = body
    for layer in ("legs_near", "legs_far"):
        top = f[layer][41][:P0_REAR]
        for y in (39, 40):
            f[layer][y][:P0_REAR] = [c if c != "." else f[layer][y][x] for x, c in enumerate(top)]
    # gate fix: the plume is re-pixelled for p0 only, swept back and near-horizontal, crest at the back line
    # (the tail is outside the identity lock; same tortoiseshell keys as the master plume)
    f["tail"] = [["."] * W for _ in range(H)]
    put(f, "tail", 11, 19, """
.....KKKKKKKKK......
...KKGGOOBBBBDKK....
.KKCGOOOBBOOBBDDK...
KWWCOOOBBBOOOBBDDK..
KCCGGOBBBBBOOBBBDDK.
.KccOOOBBBBBBBDDDDK.
..KKcccBBBBDDDDKKK..
....KKKKKKKKKK......
""")
    return f


P0_REAR = 44                                       # p0: columns left of the chest that rise with the rump


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
    shift(f, UPPER, 1, 2)                          # gate option: head 1 px lower into the pin
    shift(f, ["tail"], 2, -2)
    clear(f, "legs_near", 44, 0)
    clear(f, "legs_far", 44, 0)
    # gate option: the far paw lands 2 px forward of and above the near one, in the shadow ramp
    put(f, "legs_far", 60, 37, """
KKK.......
KccKKKKK..
.KccccccKK
..KKcccccK
....KKKKKK
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
# The seven in-betweens (execution phase): same rules as the keys. Identity block, body and tail are
# translations of locomotion pixels (one documented exception: the d1 plume-tip flick); legs and paws are
# hand-placed with the master ramps. Each is built from the locomotion frame the spec names.
# ---------------------------------------------------------------------------

FORE_X = 44                                        # columns from here on are forelegs; hind legs end at x 40


def set_upper(f: dict, src: int, dx: int, dy: int) -> None:
    """Replace head, face and gear with locomotion frame ``src``'s block, moved as one."""
    g = base(src)
    shift(g, UPPER, dx, dy)
    for layer in UPPER:
        f[layer] = g[layer]


def set_fore(f: dict, src: int) -> None:
    """Restore both forelegs (columns >= FORE_X) to locomotion frame ``src``'s pixels."""
    g = base(src)
    for layer in ("legs_near", "legs_far"):
        for y in range(H):
            f[layer][y][FORE_X:] = g[layer][y][FORE_X:]


def shift_hind(f: dict, dx: int) -> None:
    """Move both hind legs (columns < FORE_X) sideways by ``dx`` px."""
    for layer in ("legs_near", "legs_far"):
        for y in range(H):
            row = f[layer][y]
            hind = row[:FORE_X]
            row[:FORE_X] = ["."] * FORE_X
            for x, ch in enumerate(hind):
                if ch != ".":
                    row[x + dx] = ch


def ib_s2() -> dict:
    """Swat follow-through: the slap carries on down; the paw ends pressed flat on the floor, claws hidden."""
    f = base(15)
    shift(f, UPPER + ["body", "tail"], 1, 1)       # leans into the slap: spec U (+1, +1)
    clear(f, "legs_near", FORE_X, 0)
    shift_hind(f, 2)                               # hind paws creep forward: hides the capped skate
    put(f, "paw", 55, 30, """
KKK..........
KCCK.........
KcCCK........
.KcCCK.......
.KcCCK.......
..KcCCK......
..KcCCK......
...KcCCK.....
...KcCCK.....
....KcCCKKK..
.....KcCCCCKK
.....KCCCCCCK
.....KcCKCCcK
.....KccKcccK
""")
    return f


def ib_s3() -> dict:
    """Swat recover: back in the land-recovery stance (locomotion 15 pixels); hands over to idle or run."""
    return base(15)


def ib_p3() -> dict:
    """Pounce recover: the pin releases into the land-recovery stance (locomotion 15: standing forelegs, head block
    at U (0, 0), rump at the loco-15 back line); the plume is still settling, halfway between p2 and loco 15."""
    f = base(15)
    shift(f, ["tail"], 4, -1)
    return f


def ib_h1() -> dict:
    """Hurt recoil: eyes open again, head easing forward, the near forepaw back on the floor."""
    f = base(14)
    set_upper(f, 10, -1, -1)                       # locomotion 10 has loco 14's head position with open eyes
    shift(f, ["body"], -1, 0)
    shift(f, ["tail"], -1, 0)
    return f


def ib_d1() -> dict:
    """Sploot sulk: the d0 pose with the open indignant stare and a tail-tip flick (plume tip +2 px up)."""
    f = key_d0()
    set_upper(f, 10, 0, 4)
    tail = f["tail"]
    top = min(y for y in range(H) if any(c != "." for c in tail[y]))
    out = [["."] * W for _ in range(H)]
    # shear: the top rows rise by 2 px, the next rows by 1 px; the gap rows repeat the row below to stay solid
    rise = lambda y: 2 if y < top + 4 else (1 if y < top + 8 else 0)  # noqa: E731
    for y in range(H):
        for x, ch in enumerate(tail[y]):
            if ch != ".":
                out[y - rise(y)][x] = ch
        if y + 1 < H and rise(y) > rise(y + 1):
            for x, ch in enumerate(tail[y]):
                if ch != "." and out[y - rise(y) + 1][x] == ".":
                    out[y - rise(y) + 1][x] = ch
    f["tail"] = out
    return f


def ib_g0() -> dict:
    """Get-up, chest up: forelegs under the chest, head up 3 px, plume rising; the hind legs half-tucked (the flat
    d0 hind drawn in under the hips, paw still behind the body) to spread the change into g1."""
    f = key_d0()
    shift(f, ["body"], 0, -1)                      # B (0, 0)
    set_upper(f, 10, 0, 1)                         # U (0, +1), eyes open
    shift(f, ["tail"], 0, -4)                      # plume rising from the sploot flop
    clear(f, "legs_near", FORE_X, 0)
    set_fore(f, 10)
    shift_hind(f, 8)                               # haunch under the hips, paw trailing back
    return f


def ib_g1() -> dict:
    """Get-up, chin up: the land-recovery stance with the chin 1 px up (dignity restored); hinds under the hips."""
    f = base(15)
    shift(f, UPPER, 0, -1)
    return f


INBETWEENS = {
    "s2 follow-through": ib_s2, "s3 recover": ib_s3, "p3 recover": ib_p3, "h1 recoil": ib_h1,
    "d1 sploot sulk": ib_d1, "g0 chest up": ib_g0, "g1 chin up": ib_g1,
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
    builders = KEYS | INBETWEENS
    frames = []
    for name, _tag, ticks, _key in FRAMES:
        frames.append({"duration_ms": ms(ticks), "cels": cels_of(builders[name]())})
    tags, start = [], 0
    for i, (_n, tag, _t, _k) in enumerate(FRAMES):
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

SCENE = [  # (key frame, walking-line x, lift); airborne keys over open floor, clear of the two wall blocks
    (0, 410, 0), (4, 464, 0), (5, 622, 9), (6, 680, 0), (8, 735, 0), (11, 790, 0), (10, 846, 8), (1, 904, 0)]
CARL_AT, GOBLIN_AT = 362, 937


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
    # gate check: s0, loco-10 jump_start and p0 side by side (the tell must be the lowest wedge)
    loco10 = grid_images(LOCO)[10]
    trio = [imgs[0], loco10, imgs[4]]
    row = Image.new("RGBA", (W * 3 + 8, H + 4), (52, 46, 60, 255))
    for n, im in enumerate(trio):
        row.alpha_composite(im, (n * (W + 4), 2))
    for k, name in ((1, "p0_vs_jump_start_x1.png"), (6, "p0_vs_jump_start_x6.png")):
        p = EVIDENCE / name
        row.resize((row.width * k, row.height * k), Image.NEAREST).convert("RGB").save(p)
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
