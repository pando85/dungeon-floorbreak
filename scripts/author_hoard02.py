#!/usr/bin/env python3
"""Author HOARD-02 The Hoarder production frames as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/hoarder-hoard-02.md``. HOARD-02 extends the HOARD-01 rig (``author_hoard01.hoarder``):
atlas frames 0-15 are the HOARD-01 frames unchanged, frames 16-28 are the shove, jam, pinned, get-up and emerge
states. Keys 17, 18, 19, 20, 22, 24 and 26 are frozen design keys; frames 16, 21, 23, 25, 27 and 28 hold a
placeholder copy of the nearest key until the execution tier draws the in-betweens (spec section 5).
No raster image-model output, no tracing or sampling of concept pixels.

    python3 scripts/author_hoard02.py               # write grid + .ase
    python3 scripts/author_hoard02.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_hoard02.py --sheet PNG   # keys sheet (x3) next to HOARD-01 keys 0, 7, 8, 15
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_hoard01 as h1  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

HOARD_DIR = ROOT / "assets/source/characters/hoarder"
GRID = HOARD_DIR / "hoard-02.grid.json"
ASE = HOARD_DIR / "hoard-02.ase"
H1_GRID = HOARD_DIR / "hoard-01.grid.json"

# pinned: eyes open (whites), brows knitted down toward the nose, teeth gritted; never the shut defeat face
h1.FACES.setdefault("pinned", (
    (79, 29, "w"), (80, 29, "w"), (81, 29, "K"), (79, 30, "x"), (80, 30, "x"),
    (84, 29, "w"), (85, 29, "w"), (86, 29, "K"),
    (78, 26, "K"), (79, 27, "K"), (80, 27, "K"), (81, 28, "K"),
    (84, 28, "K"), (85, 27, "K"), (86, 26, "K"), (87, 25, "K"),
    (87, 31, "h"), (87, 33, "s"), (88, 33, "s"),
    (83, 36, "K"), (84, 36, "w"), (85, 36, "w"), (86, 36, "w"), (87, 36, "K"),
    (84, 37, "K"), (85, 37, "K"), (86, 37, "K")))

# snarl (shove brace and lunge): glaring eyes under knitted brows, a wide mouth with bared upper and lower teeth;
# wider than tall so it never reads as a startled round mouth at 1x
h1.FACES.setdefault("snarl", (
    (79, 29, "w"), (80, 29, "w"), (81, 29, "K"), (79, 30, "x"), (80, 30, "x"),
    (84, 29, "w"), (85, 29, "w"), (86, 29, "K"),
    (78, 26, "K"), (79, 27, "K"), (80, 27, "K"), (81, 28, "K"),
    (84, 28, "K"), (85, 27, "K"), (86, 26, "K"), (87, 25, "K"),
    (87, 31, "h"), (86, 32, "K"), (87, 33, "s"), (88, 33, "s"),
    (82, 35, "K"), (83, 35, "K"), (84, 34, "K"), (85, 34, "K"), (86, 34, "K"), (87, 34, "K"), (88, 35, "K"),
    (82, 36, "K"), (83, 36, "w"), (84, 35, "w"), (85, 35, "w"), (86, 35, "w"), (87, 35, "w"), (88, 36, "K"),
    (82, 37, "K"), (83, 37, "d"), (84, 36, "d"), (85, 36, "d"), (86, 36, "d"), (87, 36, "w"), (88, 37, "K"),
    (83, 38, "K"), (84, 37, "w"), (85, 37, "d"), (86, 37, "w"), (87, 37, "K"),
    (84, 38, "K"), (85, 38, "K"), (86, 38, "K")))

I = h1.IDLE
# pitched-torso rig options (gate fix): the near shoulder sits lower in the torso with a small sleeve cap, so no
# sleeve dome rises above the head; the tear, stains and neckline that read as a second figure under the head are off
PITCHED = dict(near_sh=(58, 56, 8), near_cap=(9, 0.35), tear=False, stains=False, neckline=False)
# 17 shove brace: low, wide, torso pitched forward with the lead (far) shoulder ahead, both arms spread low
BRACE = dict(I, body=(26, 8, 8, 1.0), head=(-16, 0, 2), swing=-6,
             near_arm=((40, 96), (22, 104), (14, 106), 60), far_arm=((124, 92), (138, 102), (143, 106), -60),
             near_leg=((42, 112), (28, 117), 0), far_leg=((102, 112), (110, 117), 0), face="snarl", sleeves="cap")
# 18 lunge heave: torso near flat at hip height, lead shoulder driving, back leg straight, far arm tucked
HEAVE = dict(I, body=(60, 10, 18, 1.0), head=(-46, 2, -9), swing=-14,
             near_arm=((92, 100), (74, 110), (68, 112), 80), far_arm=((132, 104), (124, 114), (120, 116), -20),
             near_leg=((38, 114), (16, 117), 20), far_leg=((104, 112), (114, 117), 0), face="snarl", sleeves="cap",
             **PITCHED)
# 19 lunge drag: same drive, the back foot dragged on its toes, far arm thrown forward-low
DRAG = dict(HEAVE, body=(56, 8, 19, 1.0), head=(-42, 4, -8), swing=-10,
            near_arm=((90, 98), (70, 104), (62, 104), 90), far_arm=((128, 102), (138, 110), (141, 114), -60),
            near_leg=((44, 116), (22, 118), 70), far_leg=((98, 112), (104, 117), 0))
# 20 jammed impact: stopped dead on the obstacle face, torso crumpled back, head snapped back, arms flung out
JAMMED = dict(I, body=(-10, 4, 6, 0.92), head=(-24, -2, 0), swing=10,
              near_arm=((28, 60), (16, 52), (10, 48), 220), far_arm=((118, 70), (130, 62), (134, 58), -140),
              near_leg=((40, 112), (32, 117), 0), far_leg=((98, 112), (104, 117), 0), face="roar", sleeves="cap",
              far_front=True)
# 22 pinned: prone like the defeat pose but the head is raised, eyes open, the near arm clawing forward
PINNED = dict(h1.DEFEAT, body=(80, -28, 17, 0.62), head=(-64, 6, -6), swing=0,
              near_arm=((120, 112), (134, 117), (139, 119), -80), far_arm=((110, 121), (118, 116), (121, 113), -120),
              near_leg=((58, 121), (30, 121), 85), far_leg=((66, 123), (40, 124), 85), face="pinned")
# 24 get-up: hands and knees; the near thigh drops from the raised hip to a knee planted under it, the shins trail,
# both palms are planted flat under the shoulders, the head is raised toward the target with gritted teeth
GETUP = dict(I, body=(76, 4, 5, 0.85), head=(-62, 4, -9), swing=-4,
             near_arm=((114, 104), (115, 116), (118, 120), -90), far_arm=((131, 112), (134, 118), (138, 121), -90),
             near_leg=((64, 121), (38, 121), 80), far_leg=((72, 122), (48, 122), 80), face="pinned", sleeves="cap",
             **PITCHED)
# 26 emerge: kneeling deep inside the mound, shoulders first, palms planted on the rim, head bowed, eyes open
EMERGE = dict(I, body=(14, 2, 34, 1.0), head=(16, 0, 2), swing=8,
              near_arm=((62, 82), (82, 94), (88, 97), -60), far_arm=((112, 82), (124, 94), (128, 97), -60),
              near_leg=((56, 121), (34, 121), 70), far_leg=((94, 121), (72, 122), 70), face="idle", sleeves="cap")

KEY_POSES = {17: BRACE, 18: HEAVE, 19: DRAG, 20: JAMMED, 22: PINNED, 24: GETUP, 26: EMERGE}
KEY_FRAMES = tuple(sorted(KEY_POSES))
# placeholder in-betweens: nearest key until the execution tier draws them (spec section 5)
PLACEHOLDERS = {16: 17, 21: 20, 23: 22, 25: 24, 27: 26, 28: 26}
INBETWEENS = tuple(sorted(PLACEHOLDERS))
# atlas frame -> authored ticks at 60 Hz (the presenter maps sim mode ticks onto these, spec section 4)
TICKS = {16: 10, 17: 30, 18: 6, 19: 6, 20: 12, 21: 60, 22: 20, 23: 20, 24: 12, 25: 12, 26: 30, 27: 30, 28: 30}
TAGS = [("shove_tell", 16, 17), ("lunge", 18, 19), ("jammed", 20, 21), ("pinned", 22, 23), ("get_up", 24, 25),
        ("emerge", 26, 28)]


def render(frame: int) -> list[str]:
    return h1.hoarder(KEY_POSES[PLACEHOLDERS.get(frame, frame)])


def build_grid() -> dict:
    base = json.loads(H1_GRID.read_text())
    frames = list(base["frames"])
    for f in range(16, 29):
        frames.append({"duration_ms": round(TICKS[f] * 1000 / h1.TICK_HZ), "cels": {"body": render(f)}})
    tags = base["tags"] + [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS]
    return dict(base, frames=frames, tags=tags)


def sheet(grid: dict, out: Path) -> None:
    from PIL import Image
    order = [0, 7, 8, 15] + list(KEY_FRAMES)
    cw, ch = h1.CANVAS
    s = 3
    im = Image.new("RGBA", (cw * s * 4, ch * s * 3), (24, 20, 30, 255))
    for n, f in enumerate(order):
        x, y = (n % 4) * cw * s, (n // 4) * ch * s
        bg = (112, 58, 40, 255) if (n + n // 4) % 2 else (36, 42, 60, 255)
        im.paste(Image.new("RGBA", (cw * s, ch * s), bg), (x, y))
        spr = h1._image(grid["frames"][f]["cels"]["body"], grid["palette"])
        im.alpha_composite(spr.resize((cw * s, ch * s), Image.NEAREST), (x, y))
    one = Image.new("RGBA", (cw * len(order), ch), (36, 42, 60, 255))
    for n, f in enumerate(order):
        one.alpha_composite(h1._image(grid["frames"][f]["cels"]["body"], grid["palette"]), (n * cw, 0))
    im.save(out)
    one.save(out.with_name(out.stem + "-1x.png"))


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--sheet", type=Path)
    a = ap.parse_args(argv)
    grid = build_grid()
    for f in KEY_FRAMES:
        h1.report(grid["frames"][f]["cels"]["body"], f"frame {f:2d}")
    if a.sheet:
        sheet(grid, a.sheet)
        return 0
    text = h1.dump(grid)
    if a.check:
        ok = GRID.is_file() and GRID.read_text() == text
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    GRID.write_text(text, encoding="utf-8")
    pg.write_ase(grid, ASE)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
