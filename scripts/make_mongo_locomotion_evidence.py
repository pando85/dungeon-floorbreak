#!/usr/bin/env python3
"""Evidence for ANIM-MONGO-01 (adult Mongo 76 px locomotion, all 16 frames).

Writes into ``docs/evidence/anim-mongo-01/`` (or ``--out``):

* ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: all 16 frames (toolkit sheet; keys in yellow).
* ``keys_x6.png``: the seven design keys and the master idle at 6x, ground row and pivot marked.
* ``scale_lineup_x1.png`` / ``scale_lineup_x3.png``: Carl 52 px and Donut 40 px (their approved
  locomotion grids: idle and a run contact) beside Mongo idle, run contact and jump rise, one floor line.
* ``run_footfall_diagram.png`` / ``run_onion_skin_x8.png``: the planted foot per run frame and the back
  line and feet of frames 4-9 on one ghost.
* ``run_x1.gif`` / ``run_x3.gif`` and ``jump_land_x1.gif`` / ``jump_land_x3.gif``: playback at the 60 Hz
  tick table, written with an exact indexed palette (the grid palette plus three backdrop colours).
* ``floor01_scene_640x360.png`` / ``floor01_scene_1920x1080.png``: Carl, Donut and Mongo poses in the lit Floor 1
  catacomb (toolkit scene: Floor 1 kit contract and the A1 lighting pass; needs ``bash scripts/export_assets.sh``).
* ``review-pack/``: ``python3 -m scripts.pixelart review-pack`` (three images and the validator summary).
* ``measurements.json``: per-frame bounding box, head position and row-87 contact runs.
* ``SHA256SUMS`` (every file here including ``review-pack/``).

All renders are nearest-neighbour integer enlargements of the grid pixels.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
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


def on_floor(img: Image.Image, contact_row: int, k: int, lift: int = 0, headroom: int = 10) -> Image.Image:
    """Frame on a floor strip; ``headroom`` rows above the canvas keep a lifted crest in view (all GIF frames
    share the size)."""
    w, h = img.size
    base = Image.new("RGBA", (w, h + 4 + headroom), BG + (255,))
    d = ImageDraw.Draw(base)
    d.rectangle([0, contact_row + 1 + headroom, w - 1, contact_row + 1 + headroom], fill=FLOOR_EDGE)
    d.rectangle([0, contact_row + 2 + headroom, w - 1, h + 3 + headroom], fill=FLOOR)
    base.alpha_composite(img, (0, headroom - lift))
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


RUN = [4, 5, 6, 7, 8, 9]
TINTS = [(255, 214, 64), (110, 190, 255), (120, 255, 150), (255, 130, 130), (200, 150, 255), (255, 170, 80)]


def leg_rows(grid: dict, f: int, layer: str) -> list[str]:
    w, h = grid["canvas"]
    return grid["frames"][f]["cels"].get(layer, ["." * w] * h)


def planted(grid: dict, f: int):
    """(layer, x0, x1) of the leg on the contact row, else None."""
    row = grid["pivot"][1]
    for layer in ("near_leg", "far_leg"):
        xs = [x for x, c in enumerate(leg_rows(grid, f, layer)[row]) if c != "."]
        if xs:
            return layer, min(xs), max(xs)
    return None


def lowest(grid: dict, f: int) -> tuple[int, int]:
    best = (-1, -1)
    for layer in ("near_leg", "far_leg"):
        for y, r in enumerate(leg_rows(grid, f, layer)):
            for x, c in enumerate(r):
                if c != "." and (y > best[1] or (y == best[1] and x < best[0])):
                    best = (x, y)
    return best


def footfall_diagram(grid: dict, imgs: list[Image.Image]) -> Image.Image:
    """Run frames 4-9 side by side: the planted foot boxed and labelled, airborne frames labelled with the gap."""
    k, x0, x1, y0 = 5, 30, 124, 40
    row = grid["pivot"][1]
    pw, ph = (x1 - x0) * k, (row + 2 - y0) * k
    band, gap, head = 62, 10, 56
    cols, rows = 3, 2
    sheet = Image.new("RGB", (cols * pw + (cols + 1) * gap, head + rows * (ph + band + 30) + 30), (10, 10, 14))
    d = ImageDraw.Draw(sheet)
    d.text((gap, 12), "ANIM-MONGO-01 run, frames 4-9 at 5x. Ground = bottom edge of row 87; green = pivot (76, 87); "
           "yellow = the foot on row 87; blue = airborne gap.", font=font(18), fill=(235, 235, 245))
    for n, f in enumerate(RUN):
        ox, oy = gap + (n % cols) * (pw + gap), head + (n // cols) * (ph + band + 30)
        d.text((ox, oy + 4), f"frame {f}   5 ticks (83 ms)", font=font(17), fill=(235, 235, 245))
        panel = Image.new("RGBA", ((x1 - x0), row + 2 - y0), BG + (255,))
        pd = ImageDraw.Draw(panel)
        pd.rectangle([0, row + 1 - y0, panel.width, panel.height], fill=FLOOR)
        panel.alpha_composite(imgs[f].crop((x0, y0, x1, row + 2)))
        big = up(panel, k)
        bd = ImageDraw.Draw(big)
        gy = (row + 1 - y0) * k
        bd.line([(0, gy), (big.width, gy)], fill=FLOOR_EDGE, width=3)
        px = (grid["pivot"][0] - x0) * k
        bd.rectangle([px, (row - y0) * k, px + k - 1, (row - y0) * k + k - 1], outline=(60, 255, 120), width=2)
        ty = oy + 28 + ph + 6
        p = planted(grid, f)
        if p:
            bd.rectangle([(p[1] - x0) * k - 2, (row - 12 - y0) * k, (p[2] + 1 - x0) * k + 1, gy],
                         outline=(255, 214, 64), width=3)
        sheet.paste(big.convert("RGB"), (ox, oy + 28))
        if p:
            layer, a, b = p
            d.text((ox, ty), f"CONTACT: {layer.split('_')[0]} foot on row {row}, x {a}-{b}", font=font(17), fill=(255, 214, 64))
            d.text((ox, ty + 24), "planted in exactly two frames per step", font=font(15), fill=(170, 170, 190))
        else:
            lx, ly = lowest(grid, f)
            d.text((ox, ty), f"AIRBORNE: lowest leg pixel row {ly}, {row - ly} px above the ground", font=font(17),
                   fill=(110, 190, 255))
            d.text((ox, ty + 24), "toe-off / reach: nothing planted", font=font(15), fill=(170, 170, 190))
            ax = ox + (lx - x0) * k + k // 2
            ay0, ay1 = oy + 28 + (ly + 1 - y0) * k, oy + 28 + gy
            d.line([(ax, ay0), (ax, ay1)], fill=(110, 190, 255), width=2)
    d.text((gap, sheet.height - 26), "Run order 4 > 5 > 6 > 7 > 8 > 9 > 4. Planted foot travels 16 px per frame in the sprite; "
           "at 185 px/s and 5 ticks the ground moves 15.4 px, a 0.6 px slide.", font=font(16), fill=(170, 170, 190))
    return sheet


def onion_skin(grid: dict, imgs: list[Image.Image]) -> Image.Image:
    """Run frames 4-9 over one ghost frame: back line (top body row per column) and feet, one tint per frame."""
    k, x0, x1, y0 = 8, 30, 124, 14
    row = grid["pivot"][1]
    w, h = x1 - x0, row + 6 - y0
    base = Image.new("RGBA", (w, h), BG + (255,))
    ImageDraw.Draw(base).rectangle([0, row + 1 - y0, w, h], fill=FLOOR)
    ghost = imgs[5].crop((x0, y0, x1, row + 6)).copy()
    ghost.putalpha(ghost.getchannel("A").point(lambda a: 70 if a else 0))
    base.alpha_composite(ghost)
    big = up(base, k)
    d = ImageDraw.Draw(big)
    d.line([(0, (row + 1 - y0) * k), (big.width, (row + 1 - y0) * k)], fill=FLOOR_EDGE, width=3)
    for n, f in enumerate(RUN):
        tint = TINTS[n]
        body = grid["frames"][f]["cels"]["body"]
        pts = []
        for x in range(x0 + 6, 100):
            ys = [y for y in range(len(body)) if body[y][x] != "."]
            if ys:
                pts.append(((x - x0) * k + k // 2, (min(ys) - y0) * k + n * 3))
        d.line(pts, fill=tint, width=3)
        lx, ly = lowest(grid, f)
        pl = planted(grid, f)
        if pl:
            _, a, b = pl
            d.rectangle([(a - x0) * k, (row + 1 - y0) * k + 4 + n * 4, (b + 1 - x0) * k, (row + 1 - y0) * k + 8 + n * 4],
                        fill=tint)
        else:
            cx, cy = (lx - x0) * k + k // 2, (ly - y0) * k + k // 2
            d.ellipse([cx - 7, cy - 7, cx + 7, cy + 7], outline=tint, width=3)
    head = 78
    img = Image.new("RGB", (big.width, big.height + head), (10, 10, 14))
    img.paste(big.convert("RGB"), (0, head))
    dd = ImageDraw.Draw(img)
    dd.text((10, 6), "ANIM-MONGO-01 run onion skin, frames 4-9 at 8x: back line and feet.", font=font(17),
            fill=(235, 235, 245))
    dd.text((10, 28), "bars under the floor = planted foot, rings = airborne toe "
            "(frames 6 and 9 share one ring: same toe position)", font=font(14), fill=(170, 170, 190))
    for n, f in enumerate(RUN):
        dd.text((10 + n * 100, 52), f"frame {f}", font=font(15), fill=TINTS[n])
    return img


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
    save(footfall_diagram(grid, imgs), "run_footfall_diagram.png")
    save(onion_skin(grid, imgs), "run_onion_skin_x8.png")
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
    for k in (1, 3):
        path = out / f"jump_land_x{k}.gif"
        exact_gif([on_floor(imgs[f], row, k, lifts[f]) for f in seq], gif_delays(tk), grid, path)
        files.append(path)
    # lit Floor 1 compositions (toolkit scene: the Floor 1 kit contract and the A1 lighting pass)
    from scripts.pixelart import scene
    floor = scene.Floor()
    for name, k in (("floor01_scene_640x360.png", 1), ("floor01_scene_1920x1080.png", 3)):
        scene.render_scene(cfg, imgs, k, floor).save(out / name)
        files.append(out / name)
    path = out / "measurements.json"
    path.write_text(json.dumps(measurements(grid), indent=1) + "\n", encoding="utf-8")
    files.append(path)
    # review pack: exactly three images plus the validator report (toolkit command)
    pack = out / "review-pack"
    subprocess.run([sys.executable, "-m", "scripts.pixelart", "review-pack", str(CONFIG), "--out", str(pack)],
                   cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    files += sorted(q for q in pack.iterdir() if q.is_file())
    sums = out / "SHA256SUMS"
    listed = sorted(set(files + list(out.glob("*.md"))))
    sums.write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(out).as_posix()}\n"
                            for p in listed), encoding="utf-8")
    for p in files + [sums]:
        print(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
