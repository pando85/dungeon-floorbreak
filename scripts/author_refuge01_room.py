#!/usr/bin/env python3
"""Authoring record for AREA-REFUGE-01 (execution): kit contract, manifests, room layout and room file.

Reads the piece rectangles the atlas packer of ``scripts/author_refuge01_props.py`` produced and writes

- ``assets/source/terrain/floor01/floor01_refuge_kit.json``  env-kit-v2 extension of the ENV-01 contract
- ``assets/manifests/floor01_refuge_{hearth,props,wall,door}.json``
- ``assets/levels/floor01/refuge_guild.json``                 env-kit-layout-v2, 40 x 25 tiles (640 x 400)
- ``assets/levels/floor01/refuge_guild.scene.json``            the Bevy room: marks, door, exit trigger

The layout reuses the A-grammar tile columns of the ENV-02 corridor (``opening_route_proposal.json``, S1)
and its right end wall, so the room reads as the space behind the guild door. Design:
``docs/02-art/area-refuge-01-spec.md`` (sections 3, 7, 8, 9).

    python3 scripts/author_refuge01.py && python3 scripts/author_refuge01_props.py && python3 scripts/author_refuge01_room.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_refuge01 as hearth  # noqa: E402
import author_refuge01_props as props  # noqa: E402

KIT_ID = "floor01_refuge"
CONTRACT_REL = "assets/source/terrain/floor01/floor01_refuge_kit.json"
BASE_REL = "assets/source/terrain/floor01/floor01_catacomb_kit.json"
LAYOUT_REL = "assets/levels/floor01/refuge_guild.json"
ROOM_REL = "assets/levels/floor01/refuge_guild.scene.json"
SOURCE_LAYOUT = ROOT / "assets/levels/floor01/opening_route_proposal.json"
PALETTE_ID = "floor01-catacomb-v2"
LY = 20                                    # layout y = view y + 20 (spec section 3)
BACK_Y = 288                               # floor back line (layout y); prop bases sit on row 287
FLICKER = [1.0, 0.93, 1.05, 0.96]          # spec section 7, one value per `burn` frame
STATUS = ("proposed - AREA-REFUGE-01 kit executed (Claude Sonnet) on the Opus spec and hearth key; "
          "independent review pending; not owner-approved")

HEARTH_ATLAS = "floor01_refuge_hearth"
ATLAS_STEMS = {HEARTH_ATLAS: ("refuge_hearth", "prop_animation"),
               "floor01_refuge_candle": ("refuge_candle", "prop_animation"),
               "floor01_refuge_props": ("refuge_props", "decor_atlas"),
               "floor01_refuge_wall": ("refuge_wall", "decor_atlas"),
               "floor01_refuge_door": ("refuge_door", "decor_atlas")}


def light(lid, offset, colour, radius, intensity, **extra):
    d = {"id": lid, "offset": list(offset), "colour": colour, "radius": radius, "intensity": intensity,
         "group": "scene"}
    d.update(extra)
    return d


def piece(pid, atlas, rect, layer, anchor, anchor_name, **extra):
    d = {"id": pid, "atlas": atlas, "rect": list(rect), "role": "backwall" if layer == "backwall_decal" else "prop",
         "layer": layer, "collidable": False, "anchor": list(anchor), "anchor_name": anchor_name,
         "grid_snap": 1, "grammar": ["A"]}
    if layer == "backwall_decal":
        d["decal"] = True
    d.update(extra)
    return d


def build_pieces(rects: dict[str, dict]) -> list[dict]:
    r = {pid: rect for atlas in rects.values() for pid, rect in atlas["rects"].items()}
    P = {a: v["rects"] for a, v in rects.items()}
    bc = "bottom centre on the floor back line (view y 267, layout y 287)"
    out = []
    frames = [{"frame": {"x": 80 * i, "y": 0, "w": 80, "h": 80}, "duration": 120} for i in range(4)]
    out.append(piece("refuge_hearth", HEARTH_ATLAS, [0, 0, 80, 80], "prop", (40, 79), bc, animation={
        "tag": {"name": "burn", "from": 0, "to": 3, "direction": "forward"}, "frames": frames},
        lights=[light("hearth_fire", (40, 58), "#ffb060", 230, 0.5, glow={"radius": 26, "additive": 0.12},
                      flicker=FLICKER),
                light("hearth_floor", (40, 79), "#ffb060", 150, 0.32, project="walk_line", project_dy=-4,
                      squash_y=0.3)]))
    pr = P["floor01_refuge_props"]
    out.append(piece("refuge_bed", "floor01_refuge_props", pr["refuge_bed"], "prop", (32, 27), bc))
    out.append(piece("refuge_bookshelf", "floor01_refuge_props", pr["refuge_bookshelf"], "prop", (18, 55), bc))
    out.append(piece("refuge_desk", "floor01_refuge_props", pr["refuge_desk"], "prop", (28, 25), bc))
    out.append(piece("refuge_candle_flame", "floor01_refuge_candle", [0, 0, 3, 2], "prop", (1, 0),
                     "candle tip; placed at the desk's top-left + (39, 0)", animation={
        "tag": {"name": "burn", "from": 0, "to": 3, "direction": "forward"},
        "frames": [{"frame": {"x": 3 * i, "y": 0, "w": 3, "h": 2}, "duration": 120} for i in range(4)],
        "note": "four identical frames: the sprite is static, only the light flickers (spec section 5)"},
        lights=[light("desk_candle", (1, 0), "#ffc27a", 44, 0.12, glow={"radius": 6, "additive": 0.05},
                      flicker=[1.0, 0.95, 1.03, 0.97])]))
    out.append(piece("refuge_stool", "floor01_refuge_props", pr["refuge_stool"], "prop", (7, 17), bc))
    out.append(piece("refuge_barrel", "floor01_refuge_props", pr["refuge_barrel"], "prop", (11, 27), bc))
    out.append(piece("refuge_satchel", "floor01_refuge_props", pr["refuge_satchel"], "backwall_decal", (0, 0),
                     "top_left (the wall peg)"))
    wl = P["floor01_refuge_wall"]
    tl = "top_left"
    out.append(piece("refuge_window", "floor01_refuge_wall", wl["refuge_window"], "backwall_decal", (0, 0), tl,
                     lights=[light("window_cold", (48, 60), "#6a8bb5", 110, 0.07)]))
    out.append(piece("refuge_wall_map", "floor01_refuge_wall", wl["refuge_wall_map"], "backwall_decal", (0, 0), tl))
    out.append(piece("refuge_frame_portrait", "floor01_refuge_wall", wl["refuge_frame_portrait"], "backwall_decal",
                     (0, 0), tl))
    out.append(piece("refuge_frame_landscape", "floor01_refuge_wall", wl["refuge_frame_landscape"], "backwall_decal",
                     (0, 0), tl))
    for pid in ("refuge_hearth_flue", "refuge_hearth_flue_s1", "refuge_hearth_flue_s2", "refuge_hearth_flue_s3"):
        out.append(piece(pid, "floor01_refuge_wall", wl[pid], "backwall_decal", (0, 0), tl))
    dr = P["floor01_refuge_door"]
    out.append(piece("refuge_door_inside_closed", "floor01_refuge_door", dr["refuge_door_inside_closed"], "prop",
                     (56, 136), "bottom centre on the floor back line (same opening as guild_portal_*)",
                     state_of={"group": "refuge_door", "state": "closed"}))
    out.append(piece("refuge_door_inside_open", "floor01_refuge_door", dr["refuge_door_inside_open"], "prop",
                     (56, 136), "bottom centre on the floor back line (same opening as guild_portal_*)",
                     state_of={"group": "refuge_door", "state": "open"},
                     lights=[light("door_cold", (56, 120), "#42557a", 90, 0.06, project="walk_line",
                                   squash_y=0.3)]))
    return out


def atlas_ref(name: str) -> dict:
    stem, _ = ATLAS_STEMS[name]
    return {"grid": f"assets/source/props/floor01/{stem}.grid.json", "manifest": f"assets/manifests/floor01_refuge_{stem[7:]}.json",
            "image": f"generated/art/{name}.png", "metadata": f"generated/art/{name}.json"}


def build_contract(rects: dict, colours: dict) -> dict:
    return {
        "format": "env-kit-v2", "kit_id": KIT_ID, "extends": BASE_REL, "status": STATUS,
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema "
                          "from the ENV-01 contract (env_kit.py resolve_extends; EnvKit::load does the same merge). "
                          "Adds four atlases and the guild-room pieces; no tiles and no new palette colours.",
        "palette_use": colours,
        "atlases": {n: atlas_ref(n) for n in ATLAS_STEMS},
        "tiles": [],
        "pieces": build_pieces(rects),
        "state_groups": {"refuge_door": {"states": ["closed", "open"],
                                         "note": "the inside face of guild_portal: the exit trigger opens it"}},
        "usage_rules": {
            "R1_guild_room": "one fixed 640 x 400 screen: props stand on the floor back line (layout y 287), all are "
                             "decor without colliders; the flue tiles stack from the ceiling string course to the "
                             "hearth hood, soot level rising downwards (refuge_hearth_flue, _s1, _s2, _s3); the "
                             "door states share one rect; windows and wall art are backwall decals."},
    }


def manifest(name: str, grid: dict) -> dict:
    stem, kind = ATLAS_STEMS[name]
    animated = kind == "prop_animation"
    fsize, fpivot = ([3, 2], [0, 0]) if name.endswith("candle") else ([80, 80], [40, 79])
    m = {
        "schema_version": 1, "id": f"{name}_v1", "type": kind,
        "source_path": f"assets/source/props/floor01/{stem}.ase",
        "grid_source": f"assets/source/props/floor01/{stem}.grid.json",
        "output_atlas": f"generated/art/{name}.png", "output_metadata": f"generated/art/{name}.json",
        "palette_id": PALETTE_ID,
        "frame_size": fsize if animated else list(grid["canvas"]),
        "pivot": fpivot if animated else [0, 0],
        "animation_tags": ["burn"] if animated else [],
        "required_alpha": True, "kit_contract": CONTRACT_REL,
        "owner": "AI-authored environment pixel art (AREA-REFUGE-01: design spec and hearth key Claude Opus, "
                 "execution Claude Sonnet; independent review pending)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "Original pixel-grid-v1 guild-room props for the Floor 1 refuge (AREA-REFUGE-01 on the ENV-01 "
                      "kit); rule-based shapes and ramps, nothing cropped, traced or resampled from concept art. "
                      "The wall portrait shows two original humanoid bird-people (owner direction 2026-11-21)",
            "generator": "Deterministic authoring scripts scripts/author_refuge01.py, scripts/author_refuge01_props.py "
                         "and scripts/author_refuge01_room.py written by AI agents (designed by Claude Opus: spec "
                         "and hearth key; executed by Claude Sonnet), reusing the ENV-01 shading helpers; no "
                         "image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-art-atlas-v1/02-tutorial-guild-old-guide (room mood reference only)"],
        },
    }
    return m


# ----------------------------------------------------------------------------------------- layout
def build_layout() -> dict:
    src = json.loads(SOURCE_LAYOUT.read_text(encoding="utf-8"))
    cols = 40
    tl = {}
    for name in ("backwall", "collision"):
        grid = []
        for r, row in enumerate(src["tile_layers"][name]):
            if name == "backwall":
                # S1 columns; the last column is empty (the right pier covers it), so no panel is cut by an end tile
                grid.append(row[:cols - 1] + [None])
            else:
                # S1 columns, then the kit's right pier column (the mirror of the left one: wall_end_l / _l_b)
                last = "wall_end_r" if r % 2 == 0 else "wall_end_r_b"
                grid.append(row[:cols - 1] + [last])
        if name == "collision":
            grid[1][cols - 2] = "ceiling_corner_in_r"
        tl[name] = grid
    pc = [{"piece": "torch_wall", "x": 128, "y": 160, "frame_offset": 0},
          {"piece": "torch_wall", "x": 568, "y": 160, "frame_offset": 2}]
    flue_levels = ["refuge_hearth_flue"] * 4 + ["refuge_hearth_flue_s1"] * 2 + ["refuge_hearth_flue_s2"] * 3 \
        + ["refuge_hearth_flue_s3"] * 2
    decal = [{"piece": "refuge_window", "x": 176, "y": 20 + LY}, {"piece": "refuge_window", "x": 384, "y": 20 + LY}]
    decal += [{"piece": pid, "x": 296, "y": 32 + 16 * k} for k, pid in enumerate(flue_levels)]
    decal += [{"piece": "refuge_wall_map", "x": 164, "y": 160 + LY},
              {"piece": "refuge_frame_portrait", "x": 436, "y": 164 + LY},
              {"piece": "refuge_frame_landscape", "x": 466, "y": 168 + LY},
              {"piece": "refuge_satchel", "x": 146, "y": 202 + LY}]
    base = BACK_Y - 1
    prop = [{"piece": "refuge_door_inside_closed", "x": 16, "y": base + 1 - 136},
            {"piece": "refuge_bookshelf", "x": 234 - 18, "y": base - 55},
            {"piece": "refuge_bed", "x": 180 - 32, "y": base - 27},
            {"piece": "refuge_hearth", "x": 320 - 40, "y": base - 79},
            {"piece": "refuge_desk", "x": 452 - 28, "y": base - 25},
            {"piece": "refuge_candle_flame", "x": 452 - 28 + 39, "y": base - 25},
            {"piece": "refuge_stool", "x": 492 - 7, "y": base - 17},
            {"piece": "refuge_barrel", "x": 532 - 11, "y": base - 27}] + pc
    lighting = src["lighting"]
    layout = {
        "format": "env-kit-layout-v2", "layout_id": "floor01_refuge_guild", "kit": KIT_ID, "kit_contract": CONTRACT_REL,
        "status": "AREA-REFUGE-01 guild room: one fixed screen, decor only; the Bevy room is refuge_guild.scene.json",
        "tile_size": [16, 16], "grid_size": [cols, 25], "pixel_size": [640, 400],
        "coordinate_system": src["coordinate_system"], "base_walk_line_y": 308,
        "requirements": {"continuous_floor": True, "max_step_tiles": 2},
        "segments": [{"id": "R1_guild_room", "grammar": "A", "beat": "A",
                      "purpose": "the guild interior: hearth focus, bed, shelf, desk, open central floor", "x": [0, 640]}],
        "markers": [{"id": "player_spawn", "kind": "spawn", "x": 258, "y": 307,
                     "note": "Carl's mark (room marks live in refuge_guild.scene.json)"},
                    {"id": "exit_guild_return", "kind": "exit", "x": 72, "y": 307, "rect": [40, 228 + LY, 64, 60],
                     "note": "enter the rect: refuge_door_inside closed -> open, return to the corridor's exit_guild"}],
        "views": {"refuge_640x360": [0, LY, 640, 360], "refuge_640x400": [0, 0, 640, 400],
                  "abc_640x360": [0, LY, 640, 360], "abc_640x400": [0, 0, 640, 400]},
        "parallax_layers": src["parallax_layers"], "parallax_rule": src["parallax_rule"],
        "tile_layers": tl,
        "piece_layers": {"backwall_decal": decal, "structure": [], "prop": prop, "foreground": []},
        "lighting": lighting,
    }
    return layout


def build_room() -> dict:
    return {
        "schema": "floor01-refuge-room-v1",
        "id": "refuge_guild",
        "status": "AREA-REFUGE-01: Mordecai's Tutorial Guild room. Its own room: not in the f01_late playtest manifest.",
        "layout": LAYOUT_REL,
        "view_rule": "layout y = view y + 20; one fixed 640 x 360 screen (REF-Q4)",
        "walk_line_view_y": 288,
        "marks": {
            "donut": {"x": 226, "facing": "right", "note": "in front of the bookshelf"},
            "carl": {"x": 258, "facing": "right", "note": "left of the hearth, clear of the fire opening"},
            "mordecai": {"x": 380, "facing": "left", "enabled": True,
                         "sprite": "assets/manifests/mordecai_h44.json", "height_px": 44,
                         "note": "REF-Q1: host side of the hearth, facing the party (authored facing right, "
                                 "mirrored). 'enabled': false switches him off (the loader then ignores the "
                                 "sprite); enabled without a sprite is refused"},
        },
        "clearances_view_x": {"hearth_opening": [302, 337], "desk": [424, 480], "free_floor": [200, 420]},
        "door": {"states": {"closed": "refuge_door_inside_closed", "open": "refuge_door_inside_open"},
                 "default": "closed",
                 "exit_trigger": {"id": "exit_guild_return", "rect_view": [40, 228, 64, 60], "opens": "open",
                                  "returns_to": "exit_guild"}},
    }


def sha(path: Path) -> str:
    import hashlib
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write() -> None:
    rects = {}
    for name, spec in props.ATLASES.items():
        grid, rr = props.build(name)
        rects[name] = {"rects": rr, "colours": len(grid["palette"]), "grid": grid}
    hgrid = hearth.build()
    rects[HEARTH_ATLAS] = {"rects": {}, "colours": len(hgrid["palette"]), "grid": hgrid}
    cgrid = props.candle_grid()
    rects["floor01_refuge_candle"] = {"rects": {}, "colours": len(cgrid["palette"]), "grid": cgrid}
    contract = build_contract(rects, {n: v["colours"] for n, v in rects.items()})
    (ROOT / CONTRACT_REL).write_text(json.dumps(contract, indent=1) + "\n", encoding="utf-8")
    for name, v in rects.items():
        stem = ATLAS_STEMS[name][0]
        path = ROOT / f"assets/manifests/floor01_refuge_{stem[7:]}.json"
        path.write_text(json.dumps(manifest(name, v["grid"]), indent=2) + "\n", encoding="utf-8")
    (ROOT / LAYOUT_REL).write_text(json.dumps(build_layout(), separators=(",", ":")) + "\n", encoding="utf-8")
    (ROOT / ROOM_REL).write_text(json.dumps(build_room(), indent=2) + "\n", encoding="utf-8")
    print("wrote contract, 4 manifests, layout and room file")


if __name__ == "__main__":
    write()
