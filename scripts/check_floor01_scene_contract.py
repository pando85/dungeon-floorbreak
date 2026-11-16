#!/usr/bin/env python3
"""Validate what the Floor 1 corridor scene (COMP-01) loads at runtime.

Checks the pinned actor atlases (Carl 52 px, Donut 40 px: pivots, four idle
frames, lowest opaque row == pivot row, i.e. 0 px foot sink) and the exported
environment kit atlases against the contract and the sample layout (every atlas
present, every referenced tile/piece rect inside its atlas, collision and
backwall tiles fully opaque, collider rule, z table). Run after
`bash scripts/export_assets.sh`.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"

# name, frame size, pivot (x, lowest opaque row), approved opaque height
ACTORS = (
    ("carl_book1_h52", (36, 56), (19, 55), 52),
    ("donut_scale_h40", (46, 44), (22, 43), 40),
)
FRAMES = 4
Z_TABLE = {
    "parallax_far": 0, "parallax_mid": 1, "backwall": 2, "backwall_decal": 3,
    "collision": 4, "structure": 5, "prop": 6, "foreground": 8,
}
ACTOR_ORDER = 7


class ContractError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def require_binary_alpha(name: str, image: Image.Image) -> None:
    """The lighting pass reads slot textures that were alpha-blended onto a
    transparent target; that is only exact for 1-bit alpha (it un-premultiplies
    anyway, but 8-bit rounding makes partial alpha lossy). Kit and actor atlases
    must therefore have no partially transparent pixel."""
    histogram = image.convert("RGBA").getchannel("A").histogram()
    partial = sum(histogram[1:255])
    require(partial == 0, f"{name}: {partial} partially transparent pixels (alpha must be 0 or 255)")


def check_actor(name: str, size, pivot, opaque_height: int, manifest: dict,
                metadata: dict, image: Image.Image) -> None:
    require(tuple(manifest["frame_size"]) == tuple(size), f"{name}: canvas changed")
    require(tuple(manifest["pivot"]) == tuple(pivot), f"{name}: pivot changed")
    require(name.endswith(f"h{opaque_height}"), f"{name}: approved height changed")
    frames = metadata["frames"]
    require(len(frames) == FRAMES, f"{name}: expected {FRAMES} idle frames")
    require(image.size == (size[0] * FRAMES, size[1]),
            f"{name}: atlas is {image.size}, Bevy expects {FRAMES} unpadded frames in one row")
    require_binary_alpha(name, image)
    alpha = image.convert("RGBA").getchannel("A")
    heights = []
    for index, frame in enumerate(frames):
        rect = frame["frame"]
        require((rect["x"], rect["y"], rect["w"], rect["h"]) == (index * size[0], 0, *size),
                f"{name}: frame {index} is not at the row-major position")
        cell = alpha.crop((index * size[0], 0, (index + 1) * size[0], size[1]))
        bbox = cell.point(lambda a: 255 if a else 0).getbbox()
        require(bbox is not None, f"{name}: frame {index} is empty")
        # bbox bottom is exclusive: the lowest opaque row is bottom - 1.
        require(bbox[3] - 1 == pivot[1],
                f"{name}: frame {index} lowest opaque row {bbox[3] - 1} != pivot row {pivot[1]} "
                "(feet would sink or float on the walking line)")
        require(bbox[0] <= pivot[0] < bbox[2],
                f"{name}: frame {index} pivot column outside the opaque width")
        heights.append(bbox[3] - bbox[1])
    # Frame 0 is the neutral pose at the approved height; the other idle frames
    # may bob by at most one pixel (Donut's hair/crown), never hanging below the pivot.
    require(heights[0] == opaque_height,
            f"{name}: neutral frame is {heights[0]} px tall, approved {opaque_height} px")
    require(all(opaque_height - 1 <= h <= opaque_height + 1 for h in heights),
            f"{name}: idle frame heights {heights} stray more than 1 px from {opaque_height}")


def check_kit(contract: dict, layout: dict, images: dict[str, Image.Image]) -> dict:
    require(contract.get("format") == "env-kit-v2", "contract is not env-kit-v2")
    require(layout.get("format") == "env-kit-layout-v2", "layout is not env-kit-layout-v2")
    layers = {layer["name"]: layer for layer in contract["layers"]}
    for name, order in Z_TABLE.items():
        require(name in layers and layers[name]["draw_order"] == order,
                f"layer {name}: draw order is not {order}")
    require(contract["actor_draw_order"] == ACTOR_ORDER, "actor draw order changed")
    for layer in layers.values():
        require(layer["draw_order"] != ACTOR_ORDER, f"layer {layer['name']} collides with actors")
    atlases = contract["atlases"]
    for name, atlas in atlases.items():
        require(atlas["image"].startswith("generated/art/"),
                f"atlas {name}: {atlas['image']} is not below generated/art/")
        require(name in images, f"atlas {name}: exported image missing")
    tiles = {tile["id"]: tile for tile in contract["tiles"]}
    pieces = {piece["id"]: piece for piece in contract["pieces"]}

    def inside(rect, atlas_name, what):
        x, y, w, h = rect
        width, height = images[atlas_name].size
        require(w > 0 and h > 0 and x >= 0 and y >= 0 and x + w <= width and y + h <= height,
                f"{what}: rect {rect} outside atlas {atlas_name} {width}x{height}")

    for tile in tiles.values():
        require(tile["atlas"] in images, f"tile {tile['id']}: unknown atlas {tile['atlas']}")
        require(tile["rect"][2:] == [16, 16], f"tile {tile['id']}: not 16x16")
        inside(tile["rect"], tile["atlas"], f"tile {tile['id']}")
        if tile.get("collidable"):
            walk = tile.get("walk_line_y", 0)
            require(tile.get("collider") == [0, walk, 16, 16 - walk],
                    f"tile {tile['id']}: collider is not [0, walk_line_y, 16, 16 - walk_line_y]")
        else:
            require("collider" not in tile, f"tile {tile['id']}: non-collidable tile has a collider")
    for piece in pieces.values():
        require(piece["atlas"] in images, f"piece {piece['id']}: unknown atlas {piece['atlas']}")
        inside(piece["rect"], piece["atlas"], f"piece {piece['id']}")
        for frame in piece.get("animation", {}).get("frames", []):
            f = frame["frame"]
            inside([f["x"], f["y"], f["w"], f["h"]], piece["atlas"], f"piece {piece['id']} frame")
            require(frame["duration"] > 0, f"piece {piece['id']}: zero frame duration")

    width, height = layout["pixel_size"]
    require(width == layout["grid_size"][0] * 16 and height == layout["grid_size"][1] * 16,
            "layout pixel_size does not match grid_size")
    for atlas_name, atlas_image in images.items():
        require_binary_alpha(f"atlas {atlas_name}", atlas_image)
    used_tiles = 0
    opaque_cache: dict[tuple, bool] = {}
    for layer_name, grid in layout["tile_layers"].items():
        require(layer_name in layers and layers[layer_name]["kind"] == "tiles",
                f"tile layer {layer_name} is not a contract tile layer")
        require(len(grid) == layout["grid_size"][1]
                and all(len(row) == layout["grid_size"][0] for row in grid),
                f"tile layer {layer_name}: wrong grid size")
        for row in grid:
            for tile_id in row:
                if tile_id is None:
                    continue
                require(tile_id in tiles, f"tile layer {layer_name}: unknown tile {tile_id}")
                used_tiles += 1
                tile = tiles[tile_id]
                if layer_name in ("collision", "backwall"):
                    # Opaque tiles: a hole here would show the clear colour.
                    key = (tile["atlas"], tuple(tile["rect"]))
                    if key not in opaque_cache:
                        x, y, w, h = tile["rect"]
                        alpha = images[tile["atlas"]].convert("RGBA").getchannel("A")
                        opaque_cache[key] = alpha.crop((x, y, x + w, y + h)).getextrema()[0] == 255
                    require(opaque_cache[key], f"tile {tile_id} in {layer_name} is not fully opaque")
    for layer_name, placements in layout["piece_layers"].items():
        require(layer_name in layers and layers[layer_name]["kind"] == "pieces",
                f"piece layer {layer_name} is not a contract piece layer")
        for placement in placements:
            require(placement["piece"] in pieces, f"unknown piece {placement['piece']}")
            require(pieces[placement["piece"]]["layer"] == layer_name,
                    f"piece {placement['piece']} placed in the wrong layer {layer_name}")
    for name, placement in layout["parallax_layers"].items():
        piece = pieces[placement["piece"]]
        require(piece.get("tile_x") is True, f"parallax piece {piece['id']} must tile in x")
        require(0.0 < layers[name]["parallax"] < 1.0, f"layer {name}: bad parallax factor")
    markers = {m["kind"]: m for m in layout["markers"]}
    require("spawn" in markers and "exit" in markers, "layout needs spawn and exit markers")
    require(markers["spawn"]["y"] == layout["base_walk_line_y"] - 1,
            "spawn marker is not on the walking line (y must be walk line - 1)")
    return {"tiles": used_tiles, "atlases": len(atlases)}



def run(root: Path = ROOT) -> None:
    for name, size, pivot, height in ACTORS:
        manifest = json.loads((root / "assets/manifests" / f"{name}.json").read_text())
        metadata = json.loads((root / manifest["output_metadata"]).read_text())
        image_path = root / manifest["output_atlas"]
        require(image_path.is_file(), f"{name}: exported atlas missing (run export_assets.sh)")
        check_actor(name, size, pivot, height, manifest, metadata, Image.open(image_path))
        print(f"FLOOR1_CONTRACT_OK: actor {name} {height}px, pivot {pivot}, feet on the pivot row")
    layout = json.loads((root / LAYOUT).read_text())
    contract = json.loads((root / layout["kit_contract"]).read_text())
    images = {}
    for name, atlas in contract["atlases"].items():
        path = root / atlas["image"]
        require(path.is_file(), f"atlas {name}: {atlas['image']} missing (run export_assets.sh)")
        images[name] = Image.open(path)
    summary = check_kit(contract, layout, images)
    print(f"FLOOR1_CONTRACT_OK: kit {summary['atlases']} atlases, {summary['tiles']} placed tiles")


def main() -> int:
    try:
        run()
    except ContractError as error:
        print(f"FLOOR1_CONTRACT_FAILURE: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
