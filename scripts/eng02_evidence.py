#!/usr/bin/env python3
"""ENG-02 evidence renders: still sheets and pose GIFs composed from the committed sources (not Bevy captures).

    python3 scripts/eng02_evidence.py          # writes docs/evidence/eng-02/*.png and *.gif

Outputs (deterministic; nothing is random):

* ``contact_sheet_x4.png``: every new frame at 4x on the Floor 1 ground tone, labelled: ENG-01 atlas frames 18 to 21,
  the six slammed cels and the six death cels;
* ``layer_split_x4.png``: the slammed and death cels split by layer (body, prop, pot, gore);
* ``crush_with_stone_x1.png`` and ``crush_with_stone_x4.png``: the three crush cels composited with the ENV-02 opening
  kit stone (``loose_block_falling`` with its underside on the cel's stone row, ``loose_block_landed`` for the
  remains) on the lit Floor 1 view, at 1x and at 4x (spec E2-4: a crush sheet without the stone does not pass);
* ``parts_sheet_x4.png``: the 27 parts at 4x with the ``frame0_origin`` markers and the pivot cross;
* ``schema.png``: frame layout, pivots, the death anchors and the slam launch points on the cels;
* ``getup.gif``, ``ram_run.gif``, ``slam_stomped.gif``, ``slam_wall.gif``, ``slam_flung.gif``, ``crush.gif``,
  ``decap.gif``: the pose sequences at game speed (60 Hz tick counts, whole centiseconds), 2x on the ground tone.
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw  # noqa: E402

import author_eng01 as e1  # noqa: E402
import author_eng02 as e2  # noqa: E402
from scripts.pixelart.common import CARL_X, gif_delays, grid_images, label, load_grid_file, save_gif  # noqa: E402

OUT = ROOT / "docs/evidence/eng-02"
GROUND = (74, 68, 100)                    # the Floor 1 ground tone used by the review sheets
PANEL = (24, 23, 34)
PIV = tuple(e2.PIVOT)


def tile(img: Image.Image, k: int, bg=GROUND) -> Image.Image:
    out = Image.new("RGB", (img.width * k, img.height * k), bg)
    big = img.convert("RGBA").resize((img.width * k, img.height * k), Image.NEAREST)
    out.paste(big, (0, 0), big)
    return out


def layer_image(grid: dict, n: int, layers: list[str]) -> Image.Image:
    sub = {**grid, "frames": [{"duration_ms": 100, "cels": {l: grid["frames"][n]["cels"].get(l, []) for l in layers
                                                            if l in grid["frames"][n]["cels"]}}]}
    sub["layers"] = [l for l in grid["layers"] if l in layers]
    return grid_images(sub)[0]


def sheet_of(tiles: list[tuple[str, Image.Image]], cols: int, k: int, pad: int = 8) -> Image.Image:
    w, h = tiles[0][1].size
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (w * k + pad) + pad, rows * (h * k + pad + 14) + pad), PANEL)
    d = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(tiles):
        x, y = pad + (i % cols) * (w * k + pad), pad + (i // cols) * (h * k + pad + 14)
        label(d, (x, y), name)
        sheet.paste(tile(img, k), (x, y + 14))
    return sheet


def grids() -> dict:
    return {"atlas": load_grid_file(e1.GRID), "slam": e2.slammed_grid(), "death": e2.death_grid(),
            "parts": e2.parts_grid()}


def contact_sheet(g: dict) -> Path:
    atlas, slam, death = grid_images(g["atlas"]), grid_images(g["slam"]), grid_images(g["death"])
    names = {18: "18 getup sit (10 ticks)", 19: "19 getup shove (12)", 20: "20 ram run passing (4)", 21: "21 ram run contact (4)"}
    tiles = [(f"atlas {n}: {names[n]}", atlas[n]) for n in (18, 19, 20, 21)]
    for i, tag in enumerate(["stomped", "stomped rest", "wall", "wall rest", "flung", "flung rest"]):
        tiles.append((f"slammed {i}: {tag}", slam[i]))
    for i, tag in enumerate(["crush contact 0", "crush contact 1", "crushed remains", "decap contact", "headless 4",
                             "headless 5 (held)"]):
        tiles.append((f"death {i}: {tag}", death[i]))
    out = OUT / "contact_sheet_x4.png"
    sheet_of(tiles, 4, 4).save(out)
    return out


def layer_split(g: dict) -> Path:
    tiles = []
    for name, grid, layers in (("slammed", g["slam"], ["body", "gore"]), ("death", g["death"], ["body", "prop", "pot", "gore"])):
        for n in range(6):
            tiles.append((f"{name} {n}: all", layer_image(grid, n, grid["layers"])))
            for layer in layers:
                tiles.append((f"{name} {n}: {layer}", layer_image(grid, n, [layer])))
    out = OUT / "layer_split_x4.png"
    sheet_of(tiles, 5, 3).save(out)
    return out


def crush_with_stone() -> tuple[Path, Path]:
    from scripts.pixelart.scene import Floor
    sys.path.insert(0, str(ROOT))
    import author_gob02_death as ad
    cels = grid_images(e2.death_grid())
    falling, fa = ad.block_piece("loose_block_falling")
    landed, la = ad.block_piece("loose_block_landed")
    floor = Floor()
    x0 = CARL_X + 40
    entries = []
    for n, (dx, underside) in enumerate(((0, 33), (110, 36))):
        entries += [(falling, fa, x0 + dx, PIV[1] - underside, False), (cels[n], PIV, x0 + dx, 0)]
    entries += [(landed, la, x0 + 230, 0, False), (cels[2], PIV, x0 + 230, 0)]
    one = floor.render(entries, 1)
    one.save(OUT / "crush_with_stone_x1.png")
    ground = floor.ground(CARL_X) - floor.view[1]
    four = floor.render(entries, 4)
    band = four.crop((0, max(0, (ground - 60) * 4), four.width, min(four.height, (ground + 6) * 4)))
    band.save(OUT / "crush_with_stone_x4.png")
    return OUT / "crush_with_stone_x1.png", OUT / "crush_with_stone_x4.png"


def parts_sheet(g: dict) -> Path:
    parts = grid_images(g["parts"])
    manifest = e2.parts_manifest()["death_parts"]["parts"]
    tiles = []
    for p, img in zip(manifest, parts):
        base = img.convert("RGBA")
        d = ImageDraw.Draw(base)
        d.rectangle([15, 15, 16, 16], outline=(255, 235, 60, 255))                      # the (16, 16) cell pivot
        for name, (ax, ay) in p["anchors"].items():
            d.point((ax, ay), fill=(255, 0, 255, 255))
        origin = p.get("frame0_origin")
        tiles.append((f"{p['frame']} {p['id']}" + (f" o{tuple(origin)}" if origin else ""), base))
    out = OUT / "parts_sheet_x4.png"
    sheet_of(tiles, 6, 4).save(out)
    return out


def schema(g: dict) -> Path:
    death, slam = grid_images(g["death"]), grid_images(g["slam"])
    dm = e2.death_manifest()["death_poses"]
    k = 4
    cw, ch = 64 * k, 56 * k
    cells = []
    for i in range(6):
        im = tile(death[i], k)
        d = ImageDraw.Draw(im)
        d.line([(PIV[0] * k - 6, PIV[1] * k + 2), (PIV[0] * k + 6, PIV[1] * k + 2)], fill=(255, 235, 60), width=2)
        d.line([(PIV[0] * k, PIV[1] * k - 4), (PIV[0] * k, PIV[1] * k + 6)], fill=(255, 235, 60), width=2)
        for name, (ax, ay) in dm["frames"][i]["anchors"].items():
            d.ellipse([ax * k - 4, ay * k - 4, ax * k + 4, ay * k + 4], outline=(255, 0, 255), width=2)
            d.text((ax * k + 6, ay * k - 5), name, fill=(255, 255, 255))
        cells.append((f"death {i}: anchors (magenta), pivot (28, 55) (yellow)", im))
    hints = dm["spawner_hints"]["slam"]["poses"]
    for pose, h in hints.items():
        im = tile(slam[h["frame"]], k)
        d = ImageDraw.Draw(im)
        for name, col in (("pot_launch", (80, 220, 255)), ("wrench_launch", (255, 140, 40))):
            x, y = h[name]
            d.ellipse([x * k - 5, y * k - 5, x * k + 5, y * k + 5], outline=col, width=2)
            d.text((x * k + 7, y * k - 5), name, fill=col)
        cells.append((f"slammed {h['frame']} ({pose}): launch points", im))
    cols = 3
    rows = (len(cells) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (cw + 8) + 8, rows * (ch + 22) + 8), PANEL)
    sd = ImageDraw.Draw(sheet)
    for i, (name, im) in enumerate(cells):
        x, y = 8 + (i % cols) * (cw + 8), 8 + (i // cols) * (ch + 22)
        label(sd, (x, y), name)
        sheet.paste(im, (x, y + 14))
    out = OUT / "schema.png"
    sheet.save(out)
    return out


def gif(name: str, frames: list[tuple[Image.Image, int]], k: int = 2) -> Path:
    out = OUT / name
    images = [tile(img, k) for img, _t in frames]
    save_gif(images, gif_delays([t for _i, t in frames]), out)
    return out


def gifs(g: dict) -> list[Path]:
    atlas, slam, death = grid_images(g["atlas"]), grid_images(g["slam"]), grid_images(g["death"])
    out = [
        gif("getup.gif", [(atlas[15], 5), (atlas[16], 35), (atlas[18], 10), (atlas[19], 12), (atlas[0], 14)]),
        gif("ram_run.gif", [(atlas[11], 3)] + [(atlas[n], 4) for n in (12, 20, 21, 20) * 2] + [(atlas[13], 18)]),
        gif("slam_stomped.gif", [(slam[0], 8), (slam[1], 52)]),
        gif("slam_wall.gif", [(slam[2], 8), (slam[3], 52)]),
        gif("slam_flung.gif", [(atlas[14], 24), (slam[4], 8), (slam[5], 52)]),
        gif("crush.gif", [(death[0], 3), (death[1], 3), (death[2], 54)]),
        gif("decap.gif", [(death[3], 4), (death[4], 6), (death[5], 50)]),
    ]
    return out


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    g = grids()
    written = [contact_sheet(g), layer_split(g), *crush_with_stone(), parts_sheet(g), schema(g), *gifs(g)]
    for path in written:
        print(f"wrote {path.relative_to(ROOT)} ({path.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
