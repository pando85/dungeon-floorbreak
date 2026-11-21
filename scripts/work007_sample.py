#!/usr/bin/env python3
"""WORK-007 phase 2 (E3/E4/E5): the native-resolution fidelity sample, composed with the A1 tools.

``scene``    writes ``assets/levels/floor01/work007_fidelity_sample.json`` (``work007-sample-v1``):
             the ENV-01 grammar A base corridor, the divider cells of the three states
             (F0 anticipation, F1 peak, F2 two routes), the actor and effect placements and the
             two views, all from spec section 4.
``frames``   composes F0, F1, F2 with ``env_kit.py`` (unlit layers) and ``env_kit_light.py``
             (the A1 lighting pass, profile ``floor01_torchlit_a1`` read unchanged from the ENV-01
             sample layout) and writes the 1x, 2x and 3x frames. Every upscale is a nearest-
             neighbour copy of the 1x frame.
``sheets``   writes the materials contact sheet and the small animation contact sheet.
``all``      scene + frames + sheets.

The frames are Pillow art-review compositions from the exported atlases, not Bevy captures.
Nothing here touches gameplay data.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import env_kit as ek  # noqa: E402
import env_kit_light as ekl  # noqa: E402

SCENE_PATH = ROOT / "assets/levels/floor01/work007_fidelity_sample.json"
CONTRACT = "assets/source/terrain/floor01/floor01_materials_kit.json"
LIGHT_SOURCE = "assets/levels/floor01/corridor_abc_sample.json"
OUT = ROOT / "docs/evidence/work-007"
SCENE_FORMAT = "work007-sample-v1"
STATES = ("F0", "F1", "F2")
VIEWS = {"640x360": [0, 20, 640, 360], "640x400": [0, 0, 640, 400]}
COLS, ROWS = 40, 25
TORCHES = (88, 560)  # flame centre x
DIVIDER_X = (384, 400, 416)

CARL = "assets/manifests/carl_book1_h52_locomotion.json"
GOBLIN = "assets/manifests/goblin_a_h36.json"
VFX = "assets/manifests/vfx-01.json"
SPLINTER = "assets/manifests/floor01_splinter_burst.json"


# ---------------------------------------------------------------------------
# Scene file (generator)
# ---------------------------------------------------------------------------

def _pick(tiles: dict, *names: str) -> str:
    for name in names:
        if name in tiles:
            return name
    raise KeyError(f"none of {names} is an ENV-01 tile")


def base_corridor(tiles: dict) -> dict:
    """ENV-01 grammar A over 40 columns: ceiling rows 0 and 1, brick bands, base course (row 17),
    deck rows 18 and 19, face rows 20 and 21, fill below. Edge columns follow the ENV-01 sample
    layout (left wall end at column 0, end caps at the right edge)."""
    back = [[None] * COLS for _ in range(ROWS)]
    coll = [[None] * COLS for _ in range(ROWS)]
    bands = {2: "brick_top", 3: "brick_top", 4: "brick_upper", 5: "brick_upper", 6: "brick_upper",
             7: "brick_upper", 8: "brick_mid", 9: "brick_mid", 10: "brick_mid", 11: "brick_mid",
             12: "brick_mid", 13: "brick_lower", 14: "brick_lower", 15: "brick_low", 16: "brick_low",
             17: "base_course"}
    abc = ek.load_json("assets/levels/floor01/corridor_abc_sample.json")["tile_layers"]["backwall"]
    for r, band in bands.items():
        letters = "ab" if band == "base_course" else "abcd"
        for c in range(1, 37):
            g, k = (c - 1) // 4, (c - 1) % 4 + 1
            back[r][c] = _pick(tiles, f"{band}_{letters[(g * 3 + r * 5) % len(letters)]}_{k}",
                               f"{band}_a_{k}")
        for c in range(37, 40):  # right-edge end caps, as in the ENV-01 sample layout
            back[r][c] = abc[r][125 + (c - 37)]
    for r in range(ROWS):
        coll[r][0] = "wall_end_l_b" if r % 2 else "wall_end_l"
    coll[0][1], coll[1][1] = "ceiling_fill_s", "ceiling_corner_in_l"
    for c in range(2, 38):
        k, g = (c - 2) % 4 + 1, (c - 2) // 4
        coll[0][c] = f"ceiling_fill_a_{k}"
        coll[1][c] = f"cornice_{'ab'[g % 2]}_{k}"
    for c in (38, 39):
        coll[0][c], coll[1][c] = "ceiling_fill_s", "cornice_s"
    for r, tid in zip(range(18, 25), ("deck_back_wall_l", "deck_front_wall_l", "face_a_wall_l",
                                      "face_b_wall_l", "fill_wall_l_1", "fill_wall_l_2", "fill_wall_l_2")):
        coll[r][1] = tid
    for c in range(2, 40):
        k, g = (c - 2) % 4 + 1, (c - 2) // 4
        distance = min(abs(c * 16 + 8 - x) for x in TORCHES)
        tone = "lit" if distance <= 56 else "mid" if distance <= 176 else "dim"
        v = "ab"[g % 2]
        coll[18][c] = f"deck_back_{tone}_{v}_{k}"
        coll[19][c] = f"deck_front_{tone}_{v}_{k}"
        coll[20][c] = f"face_a_{tone}_a_{k}"
        coll[21][c] = f"face_b_{v}_{k}"
        coll[22][c] = f"fill_1_j0_a_{k}"
        coll[23][c] = f"fill_2_j16_a_{k}"
        coll[24][c] = f"fill_2_j0_a_{k}"
    for layer in (back, coll):
        for row in layer:
            for c, tid in enumerate(row):
                if tid is not None and tid not in tiles:
                    raise KeyError(f"unknown ENV-01 tile {tid!r}")
    return {"backwall": back, "collision": coll}


def cell(tile: str, x: int, y: int, crop: list[int] | None = None) -> dict:
    out = {"tile": tile, "x": x, "y": y}
    if crop:
        out["crop"] = crop
    return out


def divider_cells(state: str) -> list[dict]:
    """The divider and the perch walkway for one state (spec section 4 table)."""
    cells = []
    masonry = ("wall_end_l", "wall_fill", "wall_end_r")
    for r in range(2, 9):
        for x, base in zip(DIVIDER_X, masonry):
            cells.append(cell(base + ("_b" if r % 2 else ""), x, r * 16))
    for x, base in zip(DIVIDER_X, masonry):  # the 4 px course cut from the same tiles, y 144 to 148
        cells.append(cell(base + "_b", x, 144, [0, 0, 16, 4]))
    for x in DIVIDER_X:
        cells.append(cell("iron_plate_intact", x, 148))
    for i in range(4):  # upper opening, glazed screen
        y = 164 + 16 * i
        for ci, x in enumerate(DIVIDER_X):
            if state == "F0":
                tile = "glass_leaded_intact"
            elif state == "F1":
                tile = "glass_leaded_intact" if i < 2 else "glass_leaded_cracked"
            else:
                tile = {0: "glass_leaded_broken_top", 3: "glass_leaded_broken_sill"}.get(i)
            if tile:
                cells.append(cell(tile, x, y))
    for i in range(16):  # perch walkway, x 272 to 528
        x = 272 + 16 * i
        damaged = state != "F0" and x in DIVIDER_X
        cells.append(cell("iron_plate_damaged" if damaged else "iron_plate_intact", x, 228))
    for i in range(4):  # lower opening, boarded
        y = 244 + 16 * i
        for ci, x in enumerate(DIVIDER_X):
            if state == "F0":
                tile = "timber_boards_intact"
            elif state == "F1":
                tile = "timber_boards_intact" if i < 2 else "timber_boards_cracked"
            else:
                tile = {0: "timber_boards_broken_l", 2: "timber_boards_broken_r"}.get(ci)
            if tile:
                cells.append(cell(tile, x, y))
    return cells


def sprite(manifest: str, frame: int, pivot: list[int], facing: str = "right", layer: str = "actors",
           **extra) -> dict:
    out = {"manifest": manifest, "atlas_frame": frame, "pivot": pivot, "facing": facing, "layer": layer}
    out.update(extra)
    return out


def build_scene(tiles: dict) -> dict:
    states = {
        "F0": {"label": "anticipation",
               "note": "Carl has left a lit bomb at the boards and flees his own fuse",
               "actors": [sprite(CARL, 6, [256, 307], "left", id="carl", tag="run"),
                          sprite(VFX, 0, [400, 300], layer="vfx", id="bomb_flight", tag="bomb_flight")]},
        "F1": {"label": "peak",
               "note": "one blast: wood cracks, glass cracks, iron scorches, stone holds",
               "actors": [sprite(CARL, 8, [176, 307], "left", id="carl", tag="run"),
                          sprite(SPLINTER, 1, [392, 276], layer="actors", id="splinter_burst", tag="burst"),
                          sprite(VFX, 5, [408, 284], layer="vfx", id="burst_peak", tag="burst")]},
        "F2": {"label": "two_routes",
               "note": "the lower breach is open, the cracked screen has fallen out, iron and masonry are unchanged; Carl took the upper route",
               "actors": [sprite(CARL, 0, [336, 227], "right", id="carl", tag="idle"),
                          sprite(VFX, 10, [408, 260], layer="vfx", id="burst_smoke", tag="burst"),
                          sprite(GOBLIN, 0, [488, 307], "left", id="goblin", tag="idle")]},
    }
    for state, data in states.items():
        data["cells"] = divider_cells(state)
    return {
        "format": SCENE_FORMAT,
        "scene_id": "work007_fidelity_sample",
        "status": "art-review composition for WORK-007 (phase 2); not authored level geometry, not an env-kit-layout (W007-D8: two walking surfaces)",
        "kit_contract": CONTRACT,
        "base_kit": "assets/source/terrain/floor01/floor01_catacomb_kit.json",
        "lighting_profile_from": LIGHT_SOURCE,
        "tile_size": 16,
        "grid_size": [COLS, ROWS],
        "pixel_size": [COLS * 16, ROWS * 16],
        "coordinate_system": "layout px, origin top-left, y down; cell x/y = top-left of the 16x16 tile; actor pivot = the pixel that touches the surface; effect sprites (pivot at their 48x48 centre) are placed by their centre",
        "base_walk_line_y": 308,
        "views": VIEWS,
        "parallax": "none (no arch window in this frame: the parallax budget stays at zero)",
        "geometry": {
            "walk_line_y": 308,
            "perch": {"x": [272, 528], "top_y": 228, "thickness": 16, "height_above_walk_line": 80},
            "upper_opening": {"x": [384, 432], "y": [164, 228], "clear": 64},
            "lower_opening": {"x": [384, 432], "y": [244, 308], "clear": 64},
            "lane_clear_under_perch": 64,
            "carl_height": 52,
            "jump_apex": 92,
        },
        "torches": [{"piece": "torch_wall", "x": fx - 8, "y": 160, "frame_offset": i,
                     "pool": ("torch_pool_a" if i == 0 else "torch_pool_b_m"), "pool_x": fx - 120, "pool_y": 48}
                    for i, fx in enumerate(TORCHES)],
        "tile_layers": base_corridor(tiles),
        "states": states,
    }


# ---------------------------------------------------------------------------
# Composition
# ---------------------------------------------------------------------------

def load_scene(path: Path = SCENE_PATH) -> dict:
    scene = json.loads(path.read_text(encoding="utf-8"))
    ek.check(scene.get("format") == SCENE_FORMAT, f"{path}: not a {SCENE_FORMAT} scene")
    return scene


def load_kit() -> ek.Kit:
    """The materials kit with the parallax layers removed (this frame has none) and the material tiles
    registered as in-memory ``tiles`` so the A1 renderer can crop them."""
    kit = ek.load_kit(CONTRACT)
    layers = [l for l in kit.contract["layers"] if l["kind"] != "parallax"]
    kit.contract = {**kit.contract, "layers": layers}
    kit.layers = {l["name"]: l for l in layers}
    return kit


def tile_rect(kit: ek.Kit, tile_id: str) -> tuple[str, list[int]]:
    for t in kit.contract.get("material_tiles", []):
        if t["id"] == tile_id:
            return t["atlas"], t["rect"]
    t = kit.tiles[tile_id]
    return t["atlas"], t["rect"]


def layout_for(kit: ek.Kit, scene: dict, state: str) -> dict:
    """An env-kit-layout-shaped dict for one state (in memory; the file itself is not a layout)."""
    light = ek.load_json(scene["lighting_profile_from"])["lighting"]
    structure = []
    for c in scene["states"][state]["cells"]:
        atlas, rect = tile_rect(kit, c["tile"])
        crop = c.get("crop")
        if crop:
            rect = [rect[0] + crop[0], rect[1] + crop[1], crop[2], crop[3]]
        key = f"cell:{c['tile']}:{rect[0]},{rect[1]},{rect[2]},{rect[3]}"
        kit.pieces.setdefault(key, {"id": key, "atlas": atlas, "rect": rect})
        structure.append({"piece": key, "x": c["x"], "y": c["y"]})
    decals = [{"piece": t["pool"], "x": t["pool_x"], "y": t["pool_y"]} for t in scene["torches"]]
    props = [{"piece": t["piece"], "x": t["x"], "y": t["y"], "frame_offset": t["frame_offset"]}
             for t in scene["torches"]]
    return {
        "pixel_size": scene["pixel_size"], "grid_size": scene["grid_size"], "lighting": light,
        "tile_layers": scene["tile_layers"], "parallax_layers": {},
        "piece_layers": {"backwall_decal": decals, "structure": structure, "prop": props,
                         "foreground": []},
    }


_ATLAS_CACHE: dict[str, tuple] = {}


def atlas_frame(manifest_path: str, index: int) -> tuple[Image.Image, list[int]]:
    if manifest_path not in _ATLAS_CACHE:
        manifest = json.loads((ROOT / manifest_path).read_text(encoding="utf-8"))
        meta = json.loads((ROOT / manifest["output_metadata"]).read_text(encoding="utf-8"))
        with Image.open(ROOT / manifest["output_atlas"]) as opened:
            _ATLAS_CACHE[manifest_path] = (opened.convert("RGBA"), meta["frames"], manifest["pivot"])
    atlas, frames, pivot = _ATLAS_CACHE[manifest_path]
    fr = frames[index]["frame"]
    return atlas.crop((fr["x"], fr["y"], fr["x"] + fr["w"], fr["y"] + fr["h"])), list(pivot)


def place(entry: dict) -> tuple[Image.Image, tuple[int, int]]:
    """Sprite image (mirrored when facing left) and its top-left layout position."""
    img, pivot = atlas_frame(entry["manifest"], entry["atlas_frame"])
    px, py = pivot
    if entry.get("facing") == "left":
        img = ImageOps.mirror(img)
        px = img.width - 1 - px
    tx, ty = entry["pivot"]
    return img, (tx - px, ty - py)


def actor_layers(scene: dict, state: str) -> tuple[Image.Image, Image.Image, list]:
    """(actors+debris layer, vfx layer, contact-shadow boxes) in layout px."""
    size = tuple(scene["pixel_size"])
    actors = Image.new("RGBA", size, (0, 0, 0, 0))
    vfx = Image.new("RGBA", size, (0, 0, 0, 0))
    boxes = []
    for entry in scene["states"][state]["actors"]:
        img, pos = place(entry)
        target = vfx if entry["layer"] == "vfx" else actors
        target.alpha_composite(img, pos)
        if entry.get("id") in ("carl", "goblin"):
            boxes.append((pos[0], pos[1], pos[0] + img.width, pos[1] + img.height))
    return actors, vfx, boxes


def render_state(kit: ek.Kit, scene: dict, state: str, view: list[int], lit: bool) -> Image.Image:
    """One 1x view of a state (RGB). VFX are self-lit and sit on top of the lighting pass."""
    layout = layout_for(kit, scene, state)
    actors, vfx, boxes = actor_layers(scene, state)
    img = ekl.render_lit(kit, layout, view, 1, actors=actors, lit=lit, boxes=boxes if lit else None)
    vx, vy, vw, vh = view
    out = img.convert("RGBA")
    out.alpha_composite(vfx.crop((vx, vy, vx + vw, vy + vh)))
    return out.convert("RGB")


def nearest(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def write_frames(out: Path, states=STATES) -> list[Path]:
    kit = load_kit()
    scene = load_scene()
    written: list[Path] = []
    for state in states:
        label = scene["states"][state]["label"]
        for vname, view in scene["views"].items():
            unlit = render_state(kit, scene, state, view, False)
            lit = render_state(kit, scene, state, view, True)
            stem = f"work007_{state.lower()}_{label}"
            for name, img in ((f"{stem}_unlit_1x_{vname}.png", unlit), (f"{stem}_lit_1x_{vname}.png", lit),
                              (f"{stem}_lit_2x_{vname}.png", nearest(lit, 2))):
                ek.save_png(img, out / name)
                written.append(out / name)
            if vname == "640x360":
                name = f"{stem}_lit_3x_{vname}.png"
                ek.save_png(nearest(lit, 3), out / name)
                written.append(out / name)
    return written

# ---------------------------------------------------------------------------
# Contact sheets (E5)
# ---------------------------------------------------------------------------

SHEET_BG = (26, 22, 32)
SHEET_PLATE = (52, 46, 60)
SHEET_INK = (226, 214, 200)
SHEET_DIM = (150, 140, 150)


def on_plate(tile: Image.Image, k: int) -> Image.Image:
    """The tile at k x on a mid-grey plate so partial tiles show their empty cells."""
    big = nearest(tile, k)
    plate = Image.new("RGBA", big.size, SHEET_PLATE + (255,))
    plate.alpha_composite(big)
    return plate.convert("RGB")


def tiled_3x3(tile: Image.Image) -> Image.Image:
    block = Image.new("RGBA", (tile.width * 3, tile.height * 3))
    for ty in range(3):
        for tx in range(3):
            block.paste(tile, (tx * tile.width, ty * tile.height))
    return block


def materials_sheet(kit: ek.Kit) -> Image.Image:
    """Every material tile at 1x and 4x, each full key also 3x3 tiled (shown at 2x), states left to
    right (intact, cracked or damaged, broken), then the dim / mid / lit tone rows at 4x."""
    tiles = {t["id"]: t for t in kit.contract["material_tiles"]}

    def img(tid: str) -> Image.Image:
        return kit.crop(tiles[tid]["atlas"], tiles[tid]["rect"])

    families = (
        ("riveted iron plate (reinforced; the damaged state is cosmetic)",
         ("iron_plate_intact", "iron_plate_damaged")),
        ("leaded quarry glass", ("glass_leaded_intact", "glass_leaded_cracked", "glass_leaded_broken_top",
                                 "glass_leaded_broken_sill")),
        ("nailed timber boards", ("timber_boards_intact", "timber_boards_cracked", "timber_boards_broken_l",
                                  "timber_boards_broken_r")),
    )
    margin, cell_w, line = 12, 196, 13
    width = margin * 2 + cell_w * 4
    height = margin + 2 * line + len(families) * (line + 100 + 2 * line + 8) + line + 2 * (64 + 2 * line + 6) + margin
    sheet = Image.new("RGB", (width, height), SHEET_BG)
    draw = ImageDraw.Draw(sheet)
    y = margin
    draw.text((margin, y), "WORK-007 material tiles: 4x tile, 1x tile beside it, full keys tiled 3x3 (2x); mid-grey plate = empty", fill=SHEET_INK)
    y += 2 * line
    for title, ids in families:
        draw.text((margin, y), title, fill=SHEET_DIM)
        y += line
        for i, tid in enumerate(ids):
            x = margin + i * cell_w
            tile = img(tid)
            sheet.paste(on_plate(tile, 4), (x, y))
            sheet.paste(on_plate(tile, 1), (x + 68, y))
            if tiles[tid]["collidable"]:
                sheet.paste(on_plate(tiled_3x3(tile), 2), (x + 92, y))
            draw.text((x, y + 102), tid, fill=SHEET_INK)
            draw.text((x, y + 102 + line), f"{tiles[tid]['state']}, {'solid' if tiles[tid]['collidable'] else 'remnant, no collider'}",
                      fill=SHEET_DIM)
        y += 100 + 2 * line + 8
    draw.text((margin, y), "tone variants (W007-D5), left to right: dim, mid, lit; a one-step ramp shift, lit glass takes a warm glint", fill=SHEET_DIM)
    y += line
    groups = (("iron_plate_intact", "iron_plate_damaged", "glass_leaded_intact"),
              ("glass_leaded_cracked", "timber_boards_intact", "timber_boards_cracked"))
    for row in groups:
        for gi, base in enumerate(row):
            gx = margin + gi * 3 * 70 + gi * 10
            for ti, suffix in enumerate(("_dim", "", "_lit")):
                sheet.paste(on_plate(img(base + suffix), 4), (gx + ti * 70, y))
            draw.text((gx, y + 66), base, fill=SHEET_INK)
        y += 64 + 2 * line + 6
    return sheet.crop((0, 0, width, y))


def animation_sheet() -> Image.Image:
    """Carl idle (4) and run (6), VFX-01 bomb_flight (3) and burst (9), the splinter burst (4) at 1x
    and 3x, frames labelled with their atlas index."""
    groups = (
        ("Carl idle (atlas 0 to 3)", CARL, range(0, 4)),
        ("Carl run (atlas 4 to 9)", CARL, range(4, 10)),
        ("VFX-01 bomb_flight (atlas 0 to 2)", VFX, range(0, 3)),
        ("VFX-01 burst (atlas 3 to 11)", VFX, range(3, 12)),
        ("ENV-02 timber_splinter_burst (atlas 0 to 3)", SPLINTER, range(0, 4)),
    )
    margin, line, gap = 12, 13, 8
    row_w = max(sum(atlas_frame(m, i)[0].width * 3 + gap for i in r) for _t, m, r in groups)
    width = margin * 2 + row_w
    height = margin + line
    for _title, m, _r in groups:
        h = atlas_frame(m, 0)[0].height
        height += line + h + gap + h * 3 + line + gap
    sheet = Image.new("RGB", (width, height + margin), SHEET_BG)
    draw = ImageDraw.Draw(sheet)
    draw.text((margin, margin), "WORK-007 animation frames: 1x above, 3x below each row (existing approved or proposed atlases, unchanged)", fill=SHEET_INK)
    y = margin + 2 * line
    for title, manifest, frames in groups:
        draw.text((margin, y), title, fill=SHEET_DIM)
        y += line
        x = margin
        h = atlas_frame(manifest, 0)[0].height
        for i in frames:
            frame = atlas_frame(manifest, i)[0]
            sheet.paste(on_plate(frame, 1), (x, y))
            sheet.paste(on_plate(frame, 3), (x, y + h + gap))
            draw.text((x, y + h + gap + h * 3 + 1), str(i), fill=SHEET_INK)
            x += frame.width * 3 + gap
        y += h + gap + h * 3 + line + gap
    return sheet


def write_sheets(out: Path) -> list[Path]:
    kit = load_kit()
    written = []
    for name, image in (("work007_materials_sheet.png", materials_sheet(kit)),
                        ("work007_animation_sheet.png", animation_sheet())):
        ek.save_png(image, out / name)
        written.append(out / name)
    return written


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("command", choices=("scene", "frames", "sheets", "all"))
    parser.add_argument("--out", type=Path, default=OUT)
    args = parser.parse_args(argv)
    if args.command in ("scene", "all"):
        base = ek.load_kit("assets/source/terrain/floor01/floor01_catacomb_kit.json", load_grids=False)
        scene = build_scene(base.tiles)
        SCENE_PATH.write_text(json.dumps(scene, indent=1) + "\n", encoding="utf-8")
        print(f"wrote {SCENE_PATH.relative_to(ROOT)}")
    if args.command in ("frames", "all"):
        for path in write_frames(args.out):
            print(f"wrote {path.relative_to(ROOT)}")
    if args.command in ("sheets", "all"):
        for path in write_sheets(args.out):
            print(f"wrote {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
