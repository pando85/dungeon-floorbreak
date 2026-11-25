#!/usr/bin/env python3
"""AREA-GYM-01 offline evidence sheets (deterministic, from the committed grids and the exported atlases).

    python3 scripts/make_gym01_sheets.py OUT_DIR INTACT_640x360.png LEFT_CRACK_640x360.png

Writes into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every kit piece at native size on a neutral ground
  (the 4x sheet labelled);
- ``hero_key_mirror_bay_x4.png``: the frozen hero key enlarged;
- ``exit_states_x4.png``: the exit door pair (closed, open);
- ``crop_left_crack_x4.png``: the runtime left-wall crack on the mirror (crop of the real capture);
- ``composite_right_crack_x4.png``: **composite, not runtime** (no pilot crashes into the right wall): the
  three ``mirror_crack`` frames pasted at the right-wall pivot of the real intact capture;
- ``layout_diagram_x2.png``: the arena schema (lines, zones, crack zones, piece rectangles) over the real
  intact capture.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_gym01 as gym  # noqa: E402

BG = (58, 54, 66, 255)
VIEW_TOP = 132          # layout y of the 640x360 view's top row (walk line 420 - 288)
WALK = 420


def piece_images() -> list[tuple[str, Image.Image]]:
    out = []
    for name, spec in gym.ATLASES.items():
        grid, rects = gym.build(name)
        pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5)) + (255,) for p in grid["palette"]}
        rows = grid["frames"][0]["cels"]["main"]
        sheet = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch != ".":
                    sheet.putpixel((x, y), pal[ch])
        for pid, (x, y, w, h) in rects.items():
            out.append((pid, sheet.crop((x, y, x + w, y + h))))
    return out


def contact(pieces, scale: int, labels: bool) -> Image.Image:
    pad, label_h, width = 6, (12 if labels else 0), 330
    x = y = row_h = 0
    spots = []
    for pid, img in pieces:
        w, h = img.width, img.height
        if x + w + pad > width:
            x, y, row_h = 0, y + row_h + pad + label_h, 0
        spots.append((pid, img, x + pad, y + pad + label_h))
        x += w + pad
        row_h = max(row_h, h)
    total_h = y + row_h + 2 * pad + label_h
    out = Image.new("RGBA", (width * scale, total_h * scale), BG)
    draw = ImageDraw.Draw(out)
    for pid, img, px, py in spots:
        out.alpha_composite(img.resize((img.width * scale, img.height * scale), Image.NEAREST), (px * scale, py * scale))
        if labels:
            draw.text((px * scale, (py - label_h) * scale + 2), pid, fill=(236, 226, 210, 255))
    return out


def crack_frames() -> list[tuple[Image.Image, tuple[int, int]]]:
    meta = json.loads((ROOT / "assets/manifests/boss-f01-vfx.json").read_text())
    atlas = Image.open(ROOT / "generated/art/brute_boss_f01_vfx.png").convert("RGBA")
    out = []
    for f in meta["boss"]["frames"]:
        if f["tag"] == "mirror_crack":
            r = f["rect"]
            out.append((atlas.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"])), tuple(f["pivot"])))
    return out


def main(argv: list[str]) -> int:
    out_dir, intact_path, crack_path = Path(argv[0]), Path(argv[1]), Path(argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    pieces = piece_images()
    contact(pieces, 1, False).save(out_dir / "contact_sheet_x1.png")
    contact(pieces, 4, True).save(out_dir / "contact_sheet_x4.png")
    by_id = dict(pieces)
    key = by_id["gym_mirror_bay"]
    hero = Image.new("RGBA", (key.width + 8, key.height + 8), BG)
    hero.alpha_composite(key, (4, 4))
    hero.resize((hero.width * 4, hero.height * 4), Image.NEAREST).save(out_dir / "hero_key_mirror_bay_x4.png")
    pair = Image.new("RGBA", (56, 104), BG)
    pair.alpha_composite(by_id["gym_exit_closed"], (6, 4))
    pair.alpha_composite(by_id["gym_exit_open"], (30, 4))
    pair.resize((pair.width * 4, pair.height * 4), Image.NEAREST).save(out_dir / "exit_states_x4.png")

    intact = Image.open(intact_path).convert("RGBA")
    crack = Image.open(crack_path).convert("RGBA")
    crack.crop((0, 200, 140, 290)).resize((560, 360), Image.NEAREST).save(out_dir / "crop_left_crack_x4.png")
    # composite: the three held crack states at the right-wall pivot (608, walk - 40), not flipped
    frames = crack_frames()
    strip = Image.new("RGBA", (3 * 100, 90), BG)
    for i, (img, (px, py)) in enumerate(frames):
        scene = intact.copy()
        scene.alpha_composite(img, (608 - px, WALK - 40 - VIEW_TOP - py))
        strip.alpha_composite(scene.crop((530, 200, 630, 290)), (i * 100, 0))
    big = strip.resize((strip.width * 4, strip.height * 4), Image.NEAREST)
    ImageDraw.Draw(big).text((6, 6), "COMPOSITE, NOT RUNTIME: mirror_crack 0/1/2 at the right wall", fill=(255, 230, 120, 255))
    big.save(out_dir / "composite_right_crack_x4.png")

    # schema over the real intact capture (layout coordinates, view top 132)
    contract = json.loads(gym.CONTRACT.read_text())
    sizes = {p["id"]: p["rect"][2:] for p in contract["pieces"]}
    diag = intact.resize((1280, 720), Image.NEAREST)
    d = ImageDraw.Draw(diag)

    def sy(y):
        return (y - VIEW_TOP) * 2

    for y, label, col in ((420, "walking line W 420", (120, 255, 120)), (400, "floor back line 400", (120, 200, 255)),
                          (196, "ceiling collider 196", (255, 120, 120))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
        d.text((520, sy(y) - 12), label, fill=col + (255,))
    for place in contract["arena_dressing"]["place"]:
        w, h = sizes[place["piece"]]
        if place["piece"] == "gym_girder":
            continue
        d.rectangle((place["x"] * 2, sy(place["y"]), (place["x"] + w) * 2 - 1, sy(place["y"] + h) - 1),
                    outline=(255, 210, 90, 255))
    for x0, x1, label in ((597, 624, "crack R"), (16, 43, "crack L")):
        d.rectangle((x0 * 2, sy(WALK - 58), x1 * 2, sy(WALK - 25)), outline=(255, 80, 200, 255), width=2)
        d.text((x0 * 2, sy(WALK - 58) - 12), label, fill=(255, 80, 200, 255))
    d.rectangle((83 * 2, sy(WALK - 6), 163 * 2, sy(WALK)), outline=(255, 255, 255, 255))
    d.text((83 * 2, sy(WALK) + 4), "left rack footprint 83..163", fill=(255, 255, 255, 255))
    diag.save(out_dir / "layout_diagram_x2.png")
    print(f"sheets in {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
