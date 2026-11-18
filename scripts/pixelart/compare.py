"""Concept art next to pixel frames (folded in from the owner's ``compare_concept_vs_pixel.py``).

The original read ``.ase`` files with a third-party reader from a private virtual environment. This version reads
the same sources with the repository's own ``pixel_grid_to_ase`` reader (``.grid.json`` or ``.ase``), so it needs
nothing beyond Pillow. It writes a sheet with the concept scaled on the left and, on the right, the first frame
of every tag of every source at a native-integer scale (every frame with ``all_frames``).
"""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

from .common import (BG, CELL_BG, GREY, WHITE, ToolkitError, grid_images, label, load_grid_file, load_json, repo_path, up)
from .config import FORMAT, load_config

PRESETS = {
    "carl": ("assets/concept/carl/approved/Carl_Concept_Approved_v1.png",
             ["assets/source/characters/carl/carl_h48.grid.json", "assets/source/characters/carl/carl_h52.grid.json",
              "assets/source/characters/carl/carl_h52_locomotion.grid.json", "assets/source/characters/carl/carl_h52_melee.grid.json",
              "assets/source/characters/carl/carl_h56.grid.json"]),
    "donut": ("assets/concept/donut/approved/Princess_Donut_Evolution_Gear_Bible_v1.png",
              ["assets/source/characters/donut/donut_h32.grid.json", "assets/source/characters/donut/donut_h40.grid.json",
               "assets/source/characters/donut/donut_h40_locomotion.grid.json", "assets/source/characters/donut/donut_h48.grid.json"]),
    "mongo": ("assets/concept/mongo/approved/Mongo_Adult_Approved_v1.png",
              ["assets/source/characters/mongo/mongo_adult_h64.grid.json", "assets/source/characters/mongo/mongo_adult_h76.grid.json",
               "assets/source/characters/mongo/mongo_adult_h88.grid.json"]),
    "goblin": ("assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png",
               ["assets/source/characters/goblin/goblin_a_h36.grid.json", "assets/source/characters/goblin/goblin_b_h36.grid.json",
                "assets/source/characters/goblin/goblin_size_study.grid.json"]),
}


def resolve_source(path: Path) -> tuple[str, dict]:
    """``(label, grid)`` of a ``.grid.json``, ``.ase`` or ``pixelart-asset-v1`` config path."""
    if path.suffix == ".json" and load_json(path).get("format") == FORMAT:
        cfg = load_config(path)
        return cfg.asset_id, cfg.load_grid()
    return path.name.split(".")[0], load_grid_file(path)


def sample_frames(grid: dict, all_frames: bool) -> list[tuple[str, Image.Image]]:
    imgs = grid_images(grid)
    if all_frames or not grid["tags"]:
        names = {}
        for t in grid["tags"]:
            for i in range(t["from"], t["to"] + 1):
                names[i] = t["name"]
        return [(f"{names.get(i, 'frame')} f{i}", im) for i, im in enumerate(imgs)]
    return [(f"{t['name']} f{t['from']}", imgs[t["from"]]) for t in grid["tags"]]


def compare_sheet(concept: Path | None, sources: list[Path], scale: int = 4, all_frames: bool = False,
                  columns: int = 6, title: str = "") -> Image.Image:
    if not sources:
        raise ToolkitError("compare needs at least one pixel source")
    blocks = []
    for src in sources:
        name, grid = resolve_source(src)
        blocks.append((name, grid, sample_frames(grid, all_frames)))
    pad, gap, head, cap = 14, 8, 28, 16
    cell_w = max(g["canvas"][0] for _n, g, _s in blocks) * scale
    cell_h = max(g["canvas"][1] for _n, g, _s in blocks) * scale
    block_h = []
    for _n, _g, samples in blocks:
        rows = -(-len(samples) // columns)
        block_h.append(head + rows * (cell_h + cap + gap))
    right_w = columns * (cell_w + gap)
    right_h = sum(block_h)
    left = None
    if concept is not None:
        try:
            img = Image.open(concept).convert("RGB")
        except OSError as exc:
            raise ToolkitError(f"cannot open the concept image: {exc}") from exc
        target_h = min(max(right_h, 400), 1400)
        factor = min(target_h / img.height, 1.0)
        left = img.resize((max(1, round(img.width * factor)), max(1, round(img.height * factor))), Image.Resampling.LANCZOS)
    left_w = left.width + pad if left else 0
    sheet = Image.new("RGB", (pad + left_w + right_w + pad, 36 + max(right_h, left.height if left else 0) + pad), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 10), title or "Concept vs pixel art", WHITE, 14)
    if left:
        label(d, (pad, 30), "concept (scaled, reference only)", GREY, 11)
        sheet.paste(left, (pad, 46))
    x0, y = pad + left_w, 36
    for (name, grid, samples), bh in zip(blocks, block_h):
        label(d, (x0, y + 6), f"{name}: {grid['canvas'][0]}x{grid['canvas'][1]}, pivot {tuple(grid['pivot'])}, x{scale}", (100, 150, 255), 12)
        for i, (cap_text, img) in enumerate(samples):
            r, c = divmod(i, columns)
            cx, cy = x0 + c * (cell_w + gap), y + head + r * (cell_h + cap + gap)
            tile = Image.new("RGB", (cell_w, cell_h), CELL_BG)
            big = up(img, scale)
            tile.paste(big, (0, cell_h - big.height), big)
            sheet.paste(tile, (cx, cy))
            label(d, (cx + 1, cy + cell_h + 1), cap_text, GREY, 11)
        y += bh
    return sheet


def preset_inputs(name: str) -> tuple[Path, list[Path]]:
    if name not in PRESETS:
        raise ToolkitError(f"unknown preset {name!r}; choose one of {', '.join(PRESETS)}")
    concept, sources = PRESETS[name]
    return repo_path(concept), [repo_path(s) for s in sources if repo_path(s).is_file()]
