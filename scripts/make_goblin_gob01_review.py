#!/usr/bin/env python3
"""Review renders for GOB-01 phase 1 (goblin size study and variant A key poses).

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases):

    python3 scripts/make_goblin_gob01_review.py [--out DIR]

Reads the pixel-grid sources directly (goblin size study, goblin variant A keys and the
approved Carl 52 px / Donut 40 px idle masters) and writes, deterministically, into DIR
(default generated/art/gob01_review):

- ``floor01_size_hNN_640x360.png`` / ``_1920x1080.png``: Donut, Carl and two goblins of one
  height option on the lit Floor 1 floor (A1 compositor ``scripts/env_kit_light.py``);
- ``floor01_lineup_640x360.png`` / ``_1920x1080.png``: Donut, Carl and all three options;
- ``floor01_keys_640x360.png`` / ``_1920x1080.png``: Carl facing the five key poses;
- ``size_study_sheet.png`` and ``key_pose_sheet.png``: labelled review sheets;
- ``measurements.json``: opaque bboxes, heights, areas, colours and file hashes.

Composed PNGs, not Bevy captures. Presentation-only art preview; nothing here is wired
into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN_KEYS = {"A": "assets/source/characters/goblin/goblin_a_h36_keys.grid.json",
               "B": "assets/source/characters/goblin/goblin_b_h36_keys.grid.json"}
GOBLIN_STUDY = "assets/source/characters/goblin/goblin_size_study.grid.json"
CARL = "assets/source/characters/carl/carl_h52.grid.json"
DONUT = "assets/source/characters/donut/donut_h40.grid.json"
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
DEFAULT_OUT = ROOT / "generated" / "art" / "gob01_review"
HEIGHTS = (36, 40, 44)
ATLAS = [0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26]   # atlas frame of each key in the key grids
KEY_LABELS = {0: "idle", 4: "walk contact", 7: "walk contact (far)", 11: "attack tell", 12: "swing smear",
              13: "swing impact", 16: "hurt", 19: "knockback air", 22: "down", 25: "corpse",
              26: "knockback contact"}
AIRBORNE = {19: 18, 26: 12}    # atlas frame -> lift in px when staged in flight
# walking-line x positions in the Floor 1 layout (view x 328..968)
DONUT_X, CARL_X = 404, 452
PAIR_X = (506, 556)           # two goblins facing Carl: 54 px (Carl's reach) and 104 px away
LINEUP_X = (540, 640, 740)
# Carl facing a mixed group on lit Floor 1: (variant, atlas frame, walking-line x)
KEY_SCENE = [("A", 11, 500), ("A", 13, 556), ("B", 16, 628), ("A", 19, 690), ("B", 4, 752), ("A", 0, 812),
             ("B", 22, 876)]
BG = (24, 23, 34)
GROUND_LIGHT, GROUND_DARK = (74, 68, 100), (52, 48, 74)
CYAN, YELLOW, WHITE, GREY = (0, 220, 255), (255, 214, 0), (236, 232, 240), (150, 146, 170)


class ReviewError(RuntimeError):
    pass


def grid_frames(path: str) -> tuple[list[Image.Image], tuple[int, int], dict]:
    grid = pg.load_grid(ROOT / path)
    pal = {p["key"]: pg.parse_rgba(p["rgba"]) for p in grid["palette"]}
    w, h = grid["canvas"]
    out = []
    for i in range(len(grid["frames"])):
        keys = pg.composite_keys(grid, i)
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = img.load()
        for y in range(h):
            for x in range(w):
                if keys[y][x] != ".":
                    px[x, y] = pal[keys[y][x]]
        out.append(img)
    return out, tuple(grid["pivot"]), grid


def flip(img: Image.Image, pivot: tuple[int, int]) -> tuple[Image.Image, tuple[int, int]]:
    """Mirror for facing left around the pivot column (same rule as the runtime)."""
    return img.transpose(Image.Transpose.FLIP_LEFT_RIGHT), (img.width - 1 - pivot[0], pivot[1])


def up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def bbox_info(img: Image.Image) -> dict:
    box = img.getchannel("A").getbbox()
    if box is None:
        raise ReviewError("empty frame")
    alpha = img.getchannel("A").histogram()
    area = img.width * img.height - alpha[0]
    colours = len([c for _n, c in img.getcolors(img.width * img.height) if c[3]])
    return {"bbox": list(box), "opaque_height": box[3] - box[1], "opaque_width": box[2] - box[0],
            "opaque_area_px": area, "colours": colours, "lowest_row": box[3] - 1}


def font() -> ImageFont.ImageFont:
    return ImageFont.load_default()


# ------------------------------------------------------------------ Floor 1 scenes

class Floor:
    def __init__(self) -> None:
        import env_kit as ek
        import env_kit_light as ekl
        self.ek, self.ekl = ek, ekl
        self.layout = ek.load_json(LAYOUT)
        self.kit = ek.load_kit(self.layout["kit_contract"])
        ek.validate_contract(self.kit)
        ek.validate_layout(self.kit, self.layout)
        cols, rows = self.layout["grid_size"]
        self.tops = ek.collider_tops(self.kit, self.layout["tile_layers"]["collision"], cols, rows)
        self.view = self.layout["views"]["abc_640x360"]

    def ground(self, x: int) -> int:
        top = self.tops[x // self.ek.TILE]
        if not isinstance(top, int):
            raise ReviewError(f"no walking line at x={x}")
        return top - 1

    def render(self, entries, k: int) -> Image.Image:
        """entries: (sprite, pivot, x, lift). Pivot row sits on the walking line."""
        layer = Image.new("RGBA", tuple(self.layout["pixel_size"]), (0, 0, 0, 0))
        boxes = []
        for sprite, piv, x, lift in entries:
            pos = (x - piv[0], self.ground(x) - piv[1] - lift)
            layer.alpha_composite(sprite, pos)
            boxes.append((pos[0], pos[1] + lift, pos[0] + sprite.width, pos[1] + lift + sprite.height))
        img = self.ekl.render_lit(self.kit, self.layout, self.view, k, layer, boxes=boxes)
        return img.convert("RGB")

    def view_xy(self, x: int, y: int) -> tuple[int, int]:
        return x - self.view[0], y - self.view[1]


# ------------------------------------------------------------------ sheets

def label(d: ImageDraw.ImageDraw, xy, text: str, fill=WHITE) -> None:
    d.text(xy, text, fill=fill, font=font())


def ground_strip(sprites, gap: int, k: int, guides: dict[int, tuple[int, int, int]], pad: int = 6):
    """Sprites (img, pivot, caption) on one flat floor at native size, scaled by k, with
    horizontal height guides (px above the contact row)."""
    widths = []
    for img, piv, _ in sprites:
        box = img.getchannel("A").getbbox()
        widths.append((box[0] - piv[0], box[2] - piv[0]))
    top = max(piv[1] - img.getchannel("A").getbbox()[1] for img, piv, _ in sprites) + 1
    height = top + pad + 4
    total = pad + sum(b - a for a, b in widths) + gap * (len(sprites) - 1) + pad
    strip = Image.new("RGBA", (total, height), BG + (255,))
    d = ImageDraw.Draw(strip)
    floor_y = height - 4
    d.rectangle([0, floor_y + 1, total - 1, floor_y + 1], fill=GROUND_LIGHT)
    d.rectangle([0, floor_y + 2, total - 1, height - 1], fill=GROUND_DARK)
    x = pad
    anchors = []
    for (img, piv, cap), (a, b) in zip(sprites, widths):
        px = x - a
        strip.alpha_composite(img, (px - piv[0], floor_y - piv[1]))
        anchors.append((px, cap))
        x += (b - a) + gap
    big = up(strip, k)
    d = ImageDraw.Draw(big)
    for h, colour in guides.items():
        y = (floor_y + 1 - h) * k
        for gx in range(0, big.width, 6):
            d.line([(gx, y), (gx + 2, y)], fill=colour)
        label(d, (2, y - 11), f"{h} px", colour)
    return big, [(px * k, cap) for px, cap in anchors]


def size_sheet(carl, donut, study, scenes, floor: Floor) -> Image.Image:
    k = 6
    sprites = [(carl[0], carl[1], "Carl 52 (approved)"), (donut[0], donut[1], "Donut 40 (approved)")]
    for img, h in zip(study[0], HEIGHTS):
        sprites.append((img, study[1], f"goblin {h} (block-in)"))
    guides = {52: YELLOW, 44: (255, 140, 90), 40: (200, 120, 255), 36: CYAN}
    strip, anchors = ground_strip(sprites, 10, k, guides)
    crops = []
    for h in HEIGHTS:
        img = scenes[h]
        x0, _ = floor.view_xy(DONUT_X - 40, 0)
        x1, _ = floor.view_xy(PAIR_X[1] + 40, 0)
        g = floor.ground(CARL_X) - floor.view[1]
        crop = img.crop((x0 * 3, (g - 64) * 3, x1 * 3, (g + 14) * 3))
        crops.append((h, crop))
    head = 28
    width = max(strip.width, sum(c.width for _, c in crops) + 16 * (len(crops) + 1))
    height = head + strip.height + 22 + head + max(c.height for _, c in crops) + 30
    sheet = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (8, 8), "GOB-01 size study: native idle silhouettes x6 on one floor (contact row = pivot row). "
                     "Goblins are block-ins drawn with one method at each height; not production pixels.")
    sheet.paste(strip, (0, head))
    for px, cap in anchors:
        label(d, (px, head + strip.height + 4), cap)
    y = head + strip.height + 22
    label(d, (8, y + 8), "Lit Floor 1 at x3 (1920x1080 crop): Donut, Carl, two goblins facing him at 54 and 104 px.")
    x = 16
    for h, crop in crops:
        sheet.paste(crop, (x, y + head))
        label(d, (x, y + head + crop.height + 6), f"option {h} px" + ("  (decided)" if h == 36 else ""),
              YELLOW if h == 36 else WHITE)
        x += crop.width + 16
    return sheet


def key_sheet(variants, scene_keys: Image.Image, floor: Floor) -> Image.Image:
    """variants: {"A": (frames, pivot, grid), "B": ...}; frames follow ATLAS."""
    k = 3
    frames_a, piv, _grid = variants["A"]
    cell_w, cell_h = frames_a[0].width * k, frames_a[0].height * k
    head = 28
    n = len(ATLAS)
    sheet_w = 16 + (cell_w + 12) * n
    strip_h = 160
    x1_strip = Image.new("RGB", (sheet_w, strip_h), BG)
    d1 = ImageDraw.Draw(x1_strip)
    label(d1, (8, 4), "1x (native) on a flat floor, top A, bottom B; airborne keys lifted as in flight. "
                      "Tell (3rd) and hurt (4th) sit side by side for the 1x readability check.")
    for row, var in enumerate(("A", "B")):
        floor_y = 76 + row * 72
        d1.rectangle([0, floor_y + 1, sheet_w - 1, floor_y + 1], fill=GROUND_LIGHT)
        d1.rectangle([0, floor_y + 2, sheet_w - 1, floor_y + 5], fill=GROUND_DARK)
        order = [0, 4, 11, 16, 13, 12, 7, 19, 26, 22, 25]
        for i, a in enumerate(order):
            img = variants[var][0][ATLAS.index(a)]
            x1_strip.paste(img, (40 + i * 62 - piv[0], floor_y - piv[1] - AIRBORNE.get(a, 0)), img)
    blocks = []
    for var in ("A", "B"):
        cells = Image.new("RGB", (sheet_w, cell_h + 34), BG)
        dc = ImageDraw.Draw(cells)
        for i, a in enumerate(ATLAS):
            img = variants[var][0][i]
            x = 16 + i * (cell_w + 12)
            cell = Image.new("RGBA", img.size, (36, 34, 50, 255))
            cell.alpha_composite(img)
            big = up(cell, k)
            db = ImageDraw.Draw(big)
            db.line([(0, piv[1] * k + k - 1), (big.width, piv[1] * k + k - 1)], fill=CYAN)
            db.rectangle([piv[0] * k, piv[1] * k, piv[0] * k + k - 1, piv[1] * k + k - 1], outline=YELLOW)
            cells.paste(big, (x, 0))
            info = bbox_info(img)
            label(dc, (x, cell_h + 4), f"{var} {a}: {KEY_LABELS[a]}")
            label(dc, (x, cell_h + 18), f"h={info['opaque_height']} w={info['opaque_width']} low={info['lowest_row']}", GREY)
        blocks.append(cells)
    g = floor.ground(CARL_X) - floor.view[1]
    x0, _ = floor.view_xy(CARL_X - 40, 0)
    x1, _ = floor.view_xy(KEY_SCENE[-1][2] + 50, 0)
    crop1 = scene_keys.crop((x0, g - 70, x1, g + 12))
    crop3 = up(crop1, 3)
    height = head + x1_strip.height + sum(b.height for b in blocks) + 30 + crop1.height + 12 + crop3.height + 16
    width = max(sheet_w, crop3.width + 32)
    sheet = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (8, 8), "GOB-01 keys, 36 px (decided), frame 64x56, pivot (28,55); x3 cells: cyan = contact row 55, "
                     "yellow box = pivot. Variant A above, variant B (accessory overlay) below. Facing right; flipped at runtime.")
    y = head
    sheet.paste(x1_strip, (0, y)); y += x1_strip.height
    for b in blocks:
        sheet.paste(b, (0, y)); y += b.height
    label(d, (8, y + 8), "Lit Floor 1 at 1x and x3: Carl (52) facing A tell, A swing, B hurt, A knockback, B walk, A idle, B down.")
    y += 30
    sheet.paste(crop1, (16, y)); y += crop1.height + 12
    sheet.paste(crop3, (16, y))
    return sheet


# ------------------------------------------------------------------ main

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="GOB-01 phase 1 review renders")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    out = args.out if args.out.is_absolute() else ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)

    carl_frames, carl_piv, _ = grid_frames(CARL)
    donut_frames, donut_piv, _ = grid_frames(DONUT)
    study = grid_frames(GOBLIN_STUDY)
    keys = {v: grid_frames(p) for v, p in GOBLIN_KEYS.items()}
    carl = (carl_frames[0], carl_piv)
    donut = (donut_frames[0], donut_piv)
    floor = Floor()
    written: list[Path] = []

    def save(img: Image.Image, name: str) -> Image.Image:
        path = out / name
        img.save(path, format="PNG", optimize=False)
        written.append(path)
        return img

    def both(entries, stem: str) -> Image.Image:
        native = save(floor.render(entries, 1), f"{stem}_640x360.png")
        save(floor.render(entries, 3), f"{stem}_1920x1080.png")
        return native

    base = [(donut[0], donut[1], DONUT_X, 0), (carl[0], carl[1], CARL_X, 0)]
    scenes3 = {}
    for img, h in zip(study[0], HEIGHTS):
        g, gp = flip(img, study[1])
        entries = base + [(g, gp, PAIR_X[0], 0), (g, gp, PAIR_X[1], 0)]
        both(entries, f"floor01_size_h{h}")
        scenes3[h] = Image.open(out / f"floor01_size_h{h}_1920x1080.png").convert("RGB")
    lineup = base + [(*flip(img, study[1]), x, 0) for img, x in zip(study[0], LINEUP_X)]
    both(lineup, "floor01_lineup")
    key_entries = base[1:] + [(*flip(keys[v][0][ATLAS.index(a)], keys[v][1]), x, AIRBORNE.get(a, 0))
                              for v, a, x in KEY_SCENE]
    scene_keys = both(key_entries, "floor01_keys")

    save(size_sheet(carl, donut, study, scenes3, floor), "size_study_sheet.png")
    save(key_sheet(keys, scene_keys, floor), "key_pose_sheet.png")

    carl_h = bbox_info(carl[0])["opaque_height"]
    measurements = {
        "note": "Composed review renders (A1 lighting via scripts/env_kit_light.py), not Bevy captures.",
        "reference": {"carl_h52_idle": bbox_info(carl[0]), "donut_h40_idle": bbox_info(donut[0])},
        "size_options": {},
        "keys_h36": {"A": {}, "B": {}},
        "files": {},
    }
    for img, h in zip(study[0], HEIGHTS):
        info = bbox_info(img)
        info["ratio_to_carl"] = round(info["opaque_height"] / carl_h, 3)
        info["implied_feet_if_carl_is_6ft"] = round(6 * info["opaque_height"] / carl_h, 2)
        measurements["size_options"][f"h{h}"] = info
    for v in ("A", "B"):
        for a, img in zip(ATLAS, keys[v][0]):
            measurements["keys_h36"][v][f"{a:02d} {KEY_LABELS[a]}"] = bbox_info(img)
    for path in written:
        measurements["files"][path.name] = sha(path)
    mpath = out / "measurements.json"
    mpath.write_text(json.dumps(measurements, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for path in written + [mpath]:
        print(f"  {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ReviewError as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)
