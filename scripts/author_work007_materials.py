#!/usr/bin/env python3
"""WORK-007 phase 2 (E1/E2): derive the material tiles and write the extension contract.

Reads the frozen Opus keys (frames 0 to 8 of ``materials.grid.json``, never modified) and
appends the derived frames, then writes ``floor01_materials_kit.json`` from the one tile
table below. Everything is deterministic; no image-model output.

Derived frames (spec ``docs/02-art/work-007-fidelity-sample-spec.md`` section 6, E1):

* frame 9, ``timber_boards_broken_r``: hand-set rows (stubs at the right jamb, redrawn, not
  mirrored: lit board tops, fracture faces on the left end of each stub, nails at x 14,
  stub lengths 5, 3 and 7 px);
* frames 10 to 15 (``_dim``) and 16 to 21 (``_lit``) of frames 0, 1, 2, 3, 6, 7: every
  non-accent key shifts one index down (dim) or up (lit) inside its own palette ramp,
  clamped at the ramp ends; accent keys stay; lit glass turns each ``gal_10`` glint pixel
  into ``gal_warm_03`` and nothing else.

Usage::

    python3 scripts/author_work007_materials.py derive     # rewrite the grid (frames 9..21)
    python3 scripts/author_work007_materials.py contract   # rewrite the extension contract
    python3 scripts/author_work007_materials.py check      # fail if either file is stale
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GRID = ROOT / "assets/source/terrain/floor01/materials.grid.json"
KIT = ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json"
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_materials_kit.json"
KEYS = ROOT / "tests/fixtures/work-007-materials_keys.json"

FROZEN_FRAMES = 9
SHIFTED = (0, 1, 2, 3, 6, 7)  # frames that get _dim and _lit variants
EXTRA_KEYS = "jtEFGHIJKLMNOPQRSTUVWXYZ0123456789"

# Frame 9: timber_boards_broken_r (16 rows x 16). Drawn by hand against the keys: light from
# the top left (lit board tops 'z', shaded undersides 'w'), fracture faces 'B' on the left end
# of each stub with a fresh-splinter tip 'C', one nail ('A' head over a 'z' glint) at x 14 on
# every stub, a dark split 'u' in the middle stub. Rows 4, 10 and 15 are the open board gaps.
BROKEN_R = [
    "...........Bzzzz",
    "..........BCxyAy",
    "...........Bxxzz",
    "............Bwww",
    "................",
    ".............Bzz",
    "............BxAx",
    "............uBzy",
    ".............Bxz",
    "..............Bw",
    "................",
    ".........Bzzzzzz",
    "........BCyyvyAy",
    ".........Bxxvxzx",
    "..........Bwwwww",
    "................",
]

# One table for contract, tests and sheets: (frame, id, material, state, tone, derived_from).
TILES = [
    (0, "iron_plate_intact", "reinforced", "intact", "mid", None),
    (1, "iron_plate_damaged", "reinforced", "damaged", "mid", None),
    (2, "glass_leaded_intact", "glass", "intact", "mid", None),
    (3, "glass_leaded_cracked", "glass", "cracked", "mid", None),
    (4, "glass_leaded_broken_top", "glass", "broken_top", "mid", None),
    (5, "glass_leaded_broken_sill", "glass", "broken_sill", "mid", None),
    (6, "timber_boards_intact", "wood", "intact", "mid", None),
    (7, "timber_boards_cracked", "wood", "cracked", "mid", None),
    (8, "timber_boards_broken_l", "wood", "broken_l", "mid", None),
    (9, "timber_boards_broken_r", "wood", "broken_r", "mid", None),
]
for _i, _src in enumerate(SHIFTED):
    _base = TILES[_src]
    TILES.append((10 + _i, f"{_base[1]}_dim", _base[2], _base[3], "dim", _base[1]))
for _i, _src in enumerate(SHIFTED):
    _base = TILES[_src]
    TILES.append((16 + _i, f"{_base[1]}_lit", _base[2], _base[3], "lit", _base[1]))

TAGS = [
    {"name": "iron_plate", "from": 0, "to": 1, "direction": "forward"},
    {"name": "glass_leaded", "from": 2, "to": 5, "direction": "forward"},
    {"name": "timber_boards", "from": 6, "to": 9, "direction": "forward"},
    {"name": "tone_dim", "from": 10, "to": 15, "direction": "forward"},
    {"name": "tone_lit", "from": 16, "to": 21, "direction": "forward"},
]
FAMILY = {"reinforced": "iron_plate", "glass": "glass_leaded", "wood": "timber_boards"}
BROKEN_STATES = {"broken_top", "broken_sill", "broken_l", "broken_r"}
NAME_RE = re.compile(r"^(.*)_(\d+)$")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def ramps(kit_palette: list[dict]) -> dict[str, list[tuple[int, str]]]:
    """ramp -> sorted [(index, colour name)] from the ENV-01 palette."""
    found: dict[str, list[tuple[int, str]]] = {}
    for entry in kit_palette:
        m = NAME_RE.match(entry["name"])
        if m:
            found.setdefault(m.group(1), []).append((int(m.group(2)), entry["name"]))
    return {k: sorted(v) for k, v in found.items()}


def shifted_name(name: str, step: int, ramp_table: dict[str, list[tuple[int, str]]]) -> str:
    m = NAME_RE.match(name)
    ramp = ramp_table[m.group(1)]
    names = [n for _i, n in ramp]
    pos = names.index(name) + step
    return names[max(0, min(len(names) - 1, pos))]


def derive(grid: dict, kit: dict) -> dict:
    """Return the grid with frames 9..21 rebuilt from the frozen keys (0 to 8 untouched)."""
    ramp_table = ramps(kit["palette"])
    kit_rgba = {e["name"]: e["rgba"] for e in kit["palette"]}
    # The first 28 entries are the frozen key palette; anything after is derived and rebuilt.
    base_palette = grid["palette"][:28]
    by_name = {e["name"]: e["key"] for e in base_palette}
    by_key = {e["key"]: e for e in base_palette}
    accents = {e["name"] for e in base_palette if e.get("accent")}
    free_keys = [c for c in EXTRA_KEYS if c not in by_key]
    new_entries: list[dict] = []

    def key_for(name: str) -> str:
        if name not in by_name:
            key = free_keys.pop(0)
            entry = {"key": key, "name": name, "rgba": kit_rgba[name]}
            if name == "gal_warm_03":
                entry["accent"] = True  # the torch glint on lit glass: a deliberate small detail
            new_entries.append(entry)
            by_name[name] = key
            by_key[key] = entry
        return by_name[name]

    frames = [json.loads(json.dumps(f)) for f in grid["frames"][:FROZEN_FRAMES]]
    frames.append({"duration_ms": 100, "cels": {"main": list(BROKEN_R)}})
    names_by_key = {e["key"]: e["name"] for e in base_palette}

    def tone(frame_no: int, step: int) -> dict:
        rows = []
        for row in grid["frames"][frame_no]["cels"]["main"]:
            out = []
            for ch in row:
                if ch == ".":
                    out.append(ch)
                    continue
                name = names_by_key[ch]
                if name in accents:
                    if step > 0 and frame_no in (2, 3) and name == "gal_10":
                        out.append(key_for("gal_warm_03"))
                    else:
                        out.append(ch)
                else:
                    out.append(key_for(shifted_name(name, step, ramp_table)))
            rows.append("".join(out))
        return {"duration_ms": 100, "cels": {"main": rows}}

    for src in SHIFTED:
        frames.append(tone(src, -1))
    for src in SHIFTED:
        frames.append(tone(src, +1))
    out = {k: v for k, v in grid.items() if k not in ("palette", "frames", "tags")}
    out["palette"] = base_palette + new_entries
    out["frames"] = frames
    out["tags"] = TAGS
    return {k: out[k] for k in ("format", "canvas", "pivot", "palette", "layers", "frames", "tags")}


def contract(grid: dict) -> dict:
    """The env-kit-v2 extension contract for the exported atlas ``floor01_materials``."""
    tiles = []
    for frame, tid, material, state, tone_level, source in TILES:
        if frame >= len(grid["frames"]):
            continue  # the keys-only grid (before E1) has frames 0 to 8
        collidable = state not in BROKEN_STATES
        tile = {
            "id": tid, "atlas": "floor01_materials", "frame_index": frame, "rect": [frame * 16, 0, 16, 16],
            "role": "collision", "material": material, "state": state, "tone": tone_level,
            "collidable": collidable, "grammar": ["A", "B", "C"],
        }
        if collidable:
            tile["collider"] = [0, 0, 16, 16]
        else:
            tile["partial_alpha"] = True
            tile["note"] = "edge remnant (1-bit alpha); the cells inside a broken opening are empty, not this tile"
        if source:
            tile["derived_from"] = source
        tiles.append(tile)
    groups = {}
    for family, ids in (("iron_plate", ("intact", "damaged")),
                        ("glass_leaded", ("intact", "cracked", "broken_top", "broken_sill")),
                        ("timber_boards", ("intact", "cracked", "broken_l", "broken_r"))):
        groups[family] = {"states": list(ids), "note": "swap the tile per state; tone variants (_dim, _lit) exist for the intact and cracked or damaged states only"}
    groups["iron_plate"]["note"] = "reinforced is immune in v1: damaged is cosmetic, same opaque outline and collider as intact (W007-D2a)"
    return {
        "format": "env-kit-v2",
        "kit_id": "floor01_materials",
        "extends": "assets/source/terrain/floor01/floor01_catacomb_kit.json",
        "status": "proposed - WORK-007 phase 2 drawn (Claude Sonnet) on the Opus keys; awaiting independent Opus review; not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema from the ENV-01 contract (env_kit.py resolve_extends). Adds one frame-per-tile atlas (floor01_materials, one 16x16 tile per Aseprite frame, exported as a rows sheet) and the material_tiles section: 16x16 material tiles that may be partial (edge remnants of broken states) and are placed by pixel position in the WORK-007 sample scene (assets/levels/floor01/work007_fidelity_sample.json). They are not part of the env-kit-layout tile grid; no new palette colours.",
        "palette_use": {"floor01_materials": len(grid["palette"])},
        "atlases": {
            "floor01_materials": {
                "grid": "assets/source/terrain/floor01/materials.grid.json",
                "manifest": "assets/manifests/floor01_materials.json",
                "image": "generated/art/floor01_materials.png",
                "metadata": "generated/art/floor01_materials.json",
            }
        },
        "tiles": [],
        "pieces": [],
        "material_tiles": tiles,
        "state_groups": groups,
        "usage_rules": {
            "W007_scene": "Place by pixel position in the sample scene file. Full tiles carry collider [0, 0, 16, 16]; broken tiles carry none. Reinforced (iron_plate) never changes outline or collider. Stone and iron cells never change between the intact and breached frames.",
            "tones": "_dim and _lit are one-step ramp shifts of the mid key (W007-D5); accent keys stay; lit glass turns the gal_10 glint into gal_warm_03.",
        },
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=2) + "\n"


def grid_text(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("command", choices=("derive", "contract", "check"))
    args = parser.parse_args(argv)
    grid = load(GRID)
    kit = load(KIT)
    keys = load(KEYS)
    # The frozen frames must still match their recorded digests before anything is derived.
    sys.path.insert(0, str(ROOT / "scripts"))
    from pixelart.validators import frame_digest  # noqa: E402
    for n, entry in keys["keys"].items():
        got = frame_digest(grid["frames"][int(n)]["cels"], ["main"])
        if got != entry["sha256"]:
            print(f"frozen key frame {n} changed: refusing to continue", file=sys.stderr)
            return 1
    derived = derive(grid, kit) if args.command != "contract" or len(grid["frames"]) > FROZEN_FRAMES else grid
    if args.command == "derive":
        GRID.write_text(grid_text(derived), encoding="utf-8")
        print(f"wrote {GRID.relative_to(ROOT)}: {len(derived['frames'])} frames, "
              f"{len(derived['palette'])} palette entries")
        return 0
    text = dump(contract(derived))
    if args.command == "contract":
        CONTRACT.write_text(text, encoding="utf-8")
        print(f"wrote {CONTRACT.relative_to(ROOT)}: {len(load(CONTRACT)['material_tiles'])} material tiles")
        return 0
    stale = []
    if grid_text(derived) != GRID.read_text(encoding="utf-8"):
        stale.append(str(GRID.relative_to(ROOT)))
    if not CONTRACT.is_file() or text != CONTRACT.read_text(encoding="utf-8"):
        stale.append(str(CONTRACT.relative_to(ROOT)))
    if stale:
        print("stale (rerun derive/contract): " + ", ".join(stale), file=sys.stderr)
        return 1
    print("materials grid and contract are current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
