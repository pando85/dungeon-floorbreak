#!/usr/bin/env python3
"""Evidence for ANIM-MONGO-01 (adult Mongo 76 px locomotion, design stage).

Writes into ``docs/evidence/anim-mongo-01/`` (or ``--out``):

* ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: all 16 frames (toolkit sheet; keys in yellow).
* ``keys_x6.png``: the seven design keys and the master idle at 6x, ground row and pivot marked.
* ``scale_lineup_x1.png`` / ``scale_lineup_x3.png``: Carl 52 px and Donut 40 px (their approved
  locomotion grids: idle and a run contact) beside Mongo idle, run contact and jump rise, one floor line.
* ``run_x1.gif`` / ``run_x3.gif`` and ``jump_land_x3.gif``: draft playback at the 60 Hz tick table,
  written with an exact indexed palette (the grid palette plus three backdrop colours, no quantising).
* ``measurements.json``: per-frame bounding box, head position and row-87 contact runs.
* ``SHA256SUMS``.

All renders are nearest-neighbour integer enlargements of the grid pixels.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.common import font, gif_delays, grid_images  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart.evidence import contact_sheet  # noqa: E402

CONFIG = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.pixelart.json"
CARL = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
DONUT = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
OUT = ROOT / "docs/evidence/anim-mongo-01"

BG = (25, 26, 44)          # #191a2c, the darkest Floor 1 backdrop used by the scale study
FLOOR = (58, 50, 66)
FLOOR_EDGE = (96, 82, 92)
KEYS = [4, 5, 6, 10, 12, 13, 14]
NAMES = {4: "run F0 contact", 5: "run F1 passing", 6: "run F2 airborne", 10: "J0 crouch",
         12: "jump_rise", 13: "jump_fall", 14: "L0 impact", 0: "idle (master)"}


def up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def on_floor(img: Image.Image, contact_row: int, k: int, lift: int = 0) -> Image.Image:
    w, h = img.size
    base = Image.new("RGBA", (w, h + 4), BG + (255,))
    d = ImageDraw.Draw(base)
    d.rectangle([0, contact_row + 1, w - 1, contact_row + 1], fill=FLOOR_EDGE)
    d.rectangle([0, contact_row + 2, w - 1, h + 3], fill=FLOOR)
    base.alpha_composite(img, (0, -lift) if lift else (0, 0))
    return up(base, k)


def exact_gif(frames: list[Image.Image], delays: list[int], grid: dict, path: Path) -> None:
    """Indexed GIF with a fixed palette: every grid colour and the backdrop colours, exactly."""
    colours = [BG, FLOOR, FLOOR_EDGE] + [pg.parse_rgba(p["rgba"])[:3] for p in grid["palette"]]
    flat = [c for rgb in colours for c in rgb] + [0, 0, 0] * (256 - len(colours))
    index = {rgb: i for i, rgb in enumerate(colours)}
    out = []
    for im in frames:
        rgb = im.convert("RGB")
        p = Image.new("P", rgb.size)
        p.putpalette(flat)
        raw = rgb.tobytes()
        p.putdata([index[c] for c in zip(raw[0::3], raw[1::3], raw[2::3])])  # KeyError = colour outside the palette
        out.append(p)
    out[0].save(path, save_all=True, append_images=out[1:], duration=delays, loop=0, disposal=1, optimize=False)


def keys_sheet(imgs, pivot, contact_row) -> Image.Image:
    k, cols = 6, 4
    order = [0] + KEYS
    w, h = imgs[0].size
    cw, ch = w * k + 8, h * k + 26
    sheet = Image.new("RGB", (cols * cw, -(-len(order) // cols) * ch), BG)
    d = ImageDraw.Draw(sheet)
    for i, f in enumerate(order):
        x, y = (i % cols) * cw, (i // cols) * ch
        base = Image.new("RGBA", (w, h), (44, 40, 65, 255))
        base.alpha_composite(imgs[f])
        big = up(base, k)
        bd = ImageDraw.Draw(big)
        bd.line([(0, contact_row * k + k - 1), (big.width, contact_row * k + k - 1)], fill=(0, 200, 220))
        bd.rectangle([pivot[0] * k, pivot[1] * k, pivot[0] * k + k - 1, pivot[1] * k + k - 1], outline=(255, 220, 0))
        sheet.paste(big.convert("RGB"), (x, y + 22))
        d.text((x + 4, y + 4), f"{f:02d} {NAMES[f]}" + (" KEY" if f in KEYS else ""), fill=(240, 220, 120),
               font=font(14))
    return sheet


def lineup(mongo_imgs, mongo_grid, k: int) -> Image.Image:
    """Carl 52 and Donut 40 (approved locomotion grids) beside Mongo 76, pivots on one floor row."""
    carl, donut = pg.load_grid(CARL), pg.load_grid(DONUT)
    cast = [(carl, 0, "Carl 52 idle"), (carl, 4, "Carl run"), (donut, 0, "Donut 40 idle"), (donut, 4, "Donut run"),
            (mongo_grid, 0, "Mongo 76 idle"), (mongo_grid, 4, "Mongo run F0"), (mongo_grid, 5, "Mongo run F1"),
            (mongo_grid, 12, "Mongo rise")]
    floor_y, gap = 100, 10
    widths = [g["canvas"][0] for g, _, _ in cast]
    W = sum(widths) + gap * (len(cast) + 1)
    H = floor_y + 18
    scene = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(scene)
    d.rectangle([0, floor_y + 1, W - 1, floor_y + 1], fill=FLOOR_EDGE)
    d.rectangle([0, floor_y + 2, W - 1, H - 1], fill=FLOOR)
    x = gap
    labels = []
    for (g, f, name), w in zip(cast, widths):
        img = mongo_imgs[f] if g is mongo_grid else grid_images(g)[f]
        px, py = g["pivot"]
        lift = 18 if name == "Mongo rise" else 0
        scene.alpha_composite(img, (x, floor_y - py - lift))
        labels.append((x, name))
        x += w + gap
    big = up(scene, k)
    if k > 1:
        bd = ImageDraw.Draw(big)
        for lx, name in labels:
            bd.text((lx * k + 4, (floor_y + 4) * k), name, fill=(235, 235, 235), font=font(11 * max(1, k // 2)))
    return big


def measurements(grid: dict) -> dict:
    out = []
    for i, fr in enumerate(grid["frames"]):
        flat = pg.composite_keys(grid, i)
        ys = [y for y, r in enumerate(flat) if any(c != "." for c in r)]
        xs = [x for r in flat for x, c in enumerate(r) if c != "."]
        head = fr["cels"]["head"]
        hy = [y for y, r in enumerate(head) if r.strip(".")]
        hx = [x for r in head for x, c in enumerate(r) if c != "."]
        contact = {}
        for layer in ("near_leg", "far_leg"):
            row = fr["cels"].get(layer, ["." * grid["canvas"][0]] * grid["canvas"][1])[grid["pivot"][1]]
            run = [x for x, c in enumerate(row) if c != "."]
            if run:
                contact[layer] = [min(run), max(run)]
        out.append({"frame": i, "duration_ms": fr["duration_ms"], "bbox": [min(xs), min(ys), max(xs), max(ys)],
                    "height": max(ys) - min(ys) + 1, "head_top_left": [min(hx), min(hy)],
                    "contact_row_runs": contact})
    return {"canvas": grid["canvas"], "pivot": grid["pivot"], "frames": out}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args(argv)
    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)
    cfg = load_config(CONFIG)
    grid = cfg.load_grid()
    imgs = grid_images(grid)
    pivot, row = grid["pivot"], grid["pivot"][1]
    files = []

    def save(img, name):
        img.save(out / name)
        files.append(out / name)

    for k in (1, 4):
        save(contact_sheet(cfg, imgs, k), f"contact_sheet_x{k}.png")
    save(keys_sheet(imgs, pivot, row), "keys_x6.png")
    for k in (1, 3):
        save(lineup(imgs, grid, k).convert("RGB"), f"scale_lineup_x{k}.png")
    ticks = {f: t for tag in cfg.tags.values() for f, t in tag["frames"]}
    run = list(range(4, 10)) * 2
    for k in (1, 3):
        path = out / f"run_x{k}.gif"
        exact_gif([on_floor(imgs[f], row, k) for f in run], gif_delays([ticks[f] for f in run]), grid, path)
        files.append(path)
    seq = [0, 10, 11, 12, 12, 13, 13, 14, 15, 0]
    lifts = {0: 0, 10: 0, 11: 0, 12: 14, 13: 10, 14: 0, 15: 0}
    tk = [ticks[f] or 9 for f in seq]
    tk[0], tk[-1] = 30, 40
    path = out / "jump_land_x3.gif"
    exact_gif([on_floor(imgs[f], row, 3, lifts[f]) for f in seq], gif_delays(tk), grid, path)
    files.append(path)
    path = out / "measurements.json"
    path.write_text(json.dumps(measurements(grid), indent=1) + "\n", encoding="utf-8")
    files.append(path)
    sums = out / "SHA256SUMS"
    sums.write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
                            for p in sorted(files + [q for q in out.glob("*.md")])), encoding="utf-8")
    for p in files + [sums]:
        print(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
