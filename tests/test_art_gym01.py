"""AREA-GYM-01: the gym kit of the boss arena (sources, contract, sizes, palette, dressing, determinism)."""
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_gym01 as gym  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_gym_kit.json"
SIZES = {  # spec section 4: native size per piece
    "gym_mirror_bay": (96, 74), "gym_gate_lintel": (48, 18), "gym_girder": (32, 10),
    "gym_fluoro_cold": (40, 10), "gym_fluoro_accent": (40, 10), "gym_reception_desk": (60, 32),
    "gym_turnstile": (20, 30), "gym_dumbbell_rack": (56, 28), "gym_bench": (44, 18), "gym_plate_tree": (28, 36),
    "gym_cable_tower": (44, 104), "gym_tyres": (34, 18), "gym_mat_a": (48, 22), "gym_mat_b": (48, 22),
    "gym_exit_closed": (20, 96), "gym_exit_open": (20, 96),
}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


class GymKitTest(unittest.TestCase):
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects = {}
        for name, spec in gym.ATLASES.items():
            grid, r = gym.build(name)
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name] = {"rects": r, "canvas": grid["canvas"], "colours": len(grid["palette"])}
            self.assertEqual(gym.manifest(name, grid["canvas"]), load(ROOT / f"assets/manifests/{name}.json"))
        self.assertEqual(gym.contract(rects), load(CONTRACT))

    def test_native_sizes_and_colour_budget(self):
        kit = load(CONTRACT)
        sizes = {p["id"]: tuple(p["rect"][2:]) for p in kit["pieces"]}
        self.assertEqual(sizes, SIZES)
        for spec in gym.ATLASES.values():
            self.assertLessEqual(len(load(PROPS / f"{spec['stem']}.grid.json")["palette"]), 63, spec["stem"])

    def test_palette_is_the_kit_palette(self):
        kit_colours = {c for ramp in gym.RAMPS.values() for c in ramp}
        for spec in gym.ATLASES.values():
            for p in load(PROPS / f"{spec['stem']}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{spec['stem']} {p['name']}")

    def test_contract_shape_decor_only_and_neutral_ids(self):
        kit = load(CONTRACT)
        self.assertEqual(kit["extends"], "assets/source/terrain/floor01/floor01_opening_kit.json")
        self.assertEqual(kit["tiles"], [])
        self.assertNotIn("palette", kit)
        for p in kit["pieces"]:
            self.assertIs(p["collidable"], False, p["id"])
            self.assertTrue(p["id"].startswith("gym_"), p["id"])
            self.assertNotIn("juicer", json.dumps(p).lower())
        states = {p["id"]: p.get("state_of") for p in kit["pieces"]}
        self.assertEqual(states["gym_exit_closed"], {"group": "gym_exit", "state": "closed"})
        self.assertEqual(states["gym_exit_open"], {"group": "gym_exit", "state": "open"})

    def test_dressing_places_known_pieces_in_their_layers(self):
        kit = load(CONTRACT)
        layer = {p["id"]: p["layer"] for p in kit["pieces"]}
        dressing = kit["arena_dressing"]
        self.assertEqual(dressing["remove"], ["door_timber", "guild_portal_closed"])
        for place in dressing["place"]:
            self.assertEqual(layer[place["piece"]], place["layer"], place)
            w, h = SIZES[place["piece"]]
            self.assertTrue(0 <= place["x"] and place["x"] + w <= 640, place)
            if place["piece"].startswith("gym_mat_"):    # on the floor surface: rows 401..422, lip 423 visible
                self.assertTrue(place["y"] > 400 and place["y"] + h <= 423, place)
            else:
                self.assertLessEqual(place["y"] + h, 420, f"{place} crosses the walking line")
        placed = [p["piece"] for p in dressing["place"]]
        self.assertEqual(placed.count("gym_mirror_bay"), 2)
        self.assertEqual(placed.count("gym_girder"), 18)
        for pid in SIZES:
            self.assertIn(pid, placed, pid)

    def test_lights_are_on_the_fixtures_and_the_exit_only(self):
        kit = load(CONTRACT)
        lit = sorted(p["id"] for p in kit["pieces"] if "lights" in p)
        self.assertEqual(lit, ["gym_exit_open", "gym_fluoro_accent", "gym_fluoro_cold"])

    def test_contract_validates_against_the_exported_atlases(self):
        if not (ROOT / "generated/art/floor01_gym_wall.png").is_file():
            self.skipTest("atlases not exported (bash scripts/export_assets.sh)")
        out = subprocess.run([sys.executable, str(ROOT / "scripts/env_kit.py"), "validate", "--contract",
                              str(CONTRACT.relative_to(ROOT))], cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr)


if __name__ == "__main__":
    unittest.main()
