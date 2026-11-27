#!/usr/bin/env python3
"""AREA-POLISH-01: writes the polish kit contract (reuse only, no new pixel).

The contract extends the opening kit and re-declares, by the same definitions, the approved pieces of
the corral (heat) kit the two dressed rooms reuse (the contract chain has no sibling reference, the
act-1 precedent ACT1-ART-AD13). Each room's `room_dressing` block is data in this file; the kit is
applied by src/polish_kit.rs. Run `python3 scripts/author_polish01.py` to rewrite the contract and
`python3 scripts/author_polish01.py check` to verify that the file on disk is what this script writes.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KITS = ROOT / "assets/source/terrain/floor01"
OUT = KITS / "floor01_polish_kit.json"
PARENT = "assets/source/terrain/floor01/floor01_opening_kit.json"

# The pieces re-declared from the heat kit (corral, approved pixel art, #294).
HEAT_PIECES = [
    "heat_scorch_wall_a",
    "heat_scorch_wall_b",
    "heat_scorch_wall_c",
    "heat_scorch_floor_a",
    "heat_scorch_floor_b",
    "heat_ash_floor_a",
    "heat_ash_floor_b",
    "heat_trace_spit_a",
    "heat_trace_spit_b",
]


def put(layer, piece, x, y):
    return {"layer": layer, "piece": piece, "x": x, "y": y}


def block(remove_at=(), backwall_fill=(), place=()):
    return {
        "remove": [],
        "remove_at": list(remove_at),
        "backwall_fill": list(backwall_fill),
        "place": list(place),
        "parallax": {},
    }


# ---------------------------------------------------------------------------------------------
# mastery (room 17): the CF02-F microarena, a 1152 px arena whose only composed bay is the first.
# ---------------------------------------------------------------------------------------------
MASTERY = block(
    remove_at=[],
    backwall_fill=[],
    place=[
        # two blind bays of the arcade (catacomb pieces, the composer's bay geometry) over the plain
        # brick wall: no threshold and no chain, so no sill under a bricked bay
        put("structure", "pier_l", 416, 304),
        put("structure", "arch_ring_a", 432, 304),
        put("structure", "pier_r", 640, 304),
        put("structure", "pier_l", 784, 304),
        put("structure", "arch_ring_b", 800, 304),
        put("structure", "pier_r", 1008, 304),
        # torches on the piers, clear of the perches (x 544..672 and 896..992) and the parapet
        # (x 736..768); the pools are the composer's (x - 104, y - 84)
        put("prop", "torch_wall", 440, 448),
        put("backwall_decal", "torch_pool_a_m", 336, 364),
        put("prop", "torch_wall", 696, 448),
        put("backwall_decal", "torch_pool_a_m", 592, 364),
        put("prop", "torch_wall", 800, 448),
        put("backwall_decal", "torch_pool_a_m", 696, 364),
        # the corral's scorch (heat kit) in the llama yards: the wall behind perch 1 and the exit
        # side, the floor decals between the piers (the llama spawns are x 616 and 1000)
        put("backwall_decal", "heat_scorch_wall_a", 576, 432),
        put("backwall_decal", "heat_scorch_wall_b", 944, 448),
        put("structure", "heat_scorch_floor_a", 552, 561),
        put("structure", "heat_scorch_floor_b", 936, 562),
        put("structure", "heat_ash_floor_b", 880, 564),
        put("structure", "heat_trace_spit_a", 500, 564),
        # ENV-02: the wall cavity of the opening lane, twice
        put("backwall_decal", "wall_cavity", 328, 392),
        put("backwall_decal", "wall_cavity", 872, 400),
    ],
)

# ---------------------------------------------------------------------------------------------
# dozer (room 8): the catacomb arcade shell around the DOZ-01 rig.
# ---------------------------------------------------------------------------------------------
DOZER = block(
    remove_at=[
        put("structure", "arch_threshold", 416, 544),
        put("structure", "arch_chain", 432, 384),
    ],
    backwall_fill=[{"x": 384, "y": 352, "w": 208, "h": 208}],
    place=[
        put("backwall_decal", "wall_cavity", 456, 424),
        put("structure", "heat_scorch_floor_a", 128, 561),
    ],
)


def build():
    heat = json.loads((KITS / "floor01_heat_kit.json").read_text())
    pieces = [p for p in heat["pieces"] if p["id"] in HEAT_PIECES]
    assert [p["id"] for p in pieces] == HEAT_PIECES, "a heat piece is missing"
    atlases = sorted({p["atlas"] for p in pieces})
    return {
        "format": "env-kit-v2",
        "kit_id": "floor01_polish",
        "extends": PARENT,
        "status": "proposed - AREA-POLISH-01 kit (docs/02-art/area-polish-01-spec.md); independent review pending; not owner-approved",
        "extension_note": "Inherits palette, layers, draw order, coordinates, walking-line rule and lighting schema from the ENV-01 contract through the opening kit. Reuse only: the heat kit's scorch and ash pieces are declared again by the same definitions (the contract chain has no sibling reference), nothing else is added: no atlas, no tile, nothing collidable, no new palette colour and no new light emitter. Dresses the mastery arena (room 17) and the Dozer's room shell (room 8).",
        "palette_use": {a: heat["palette_use"][a] for a in atlases},
        "atlases": {a: heat["atlases"][a] for a in atlases},
        "tiles": [],
        "pieces": pieces,
        "usage_rules": {
            "R1_mastery": "room `mastery` only: pieces never produce solids; a blind bay of the arcade is placed over the plain brick wall (no threshold, no chain: no sill under a bricked bay); no torch flame over the perches or the parapet",
            "R2_dozer": "room `dozer` only: one bay is bricked up (its sill and chain removed) and dressed; the rig, the rear gate, the lintel tiles and the stop frame are the presenter's and are not touched",
        },
        "room_dressing": {"mastery": MASTERY, "dozer": DOZER},
    }


def main():
    text = json.dumps(build(), indent=1) + "\n"
    if len(sys.argv) > 1 and sys.argv[1] == "check":
        if OUT.read_text() != text:
            sys.exit(f"{OUT} differs from what this script writes")
        print("ok")
        return
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
