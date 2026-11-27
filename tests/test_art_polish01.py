"""AREA-POLISH-01: the reuse-only polish kit (contract equals what the authoring script writes, the re-declared
heat pieces are the heat kit's own definitions, no tile, no atlas of its own, nothing collidable, every placement
names a declared piece on its own layer inside the room) and the Juicer exit lintel placement."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_polish01 as polish  # noqa: E402

KITS = ROOT / "assets/source/terrain/floor01"
CONTRACT = KITS / "floor01_polish_kit.json"
ROOM_WIDTH = {"mastery": 1152, "dozer": 1344}


def load(path):
    return json.loads(Path(path).read_text())


def declared_pieces():
    """The pieces a placement may name: this contract, the opening kit and ENV-01 (the extends chain)."""
    out = {}
    path = CONTRACT
    while True:
        kit = load(path)
        for p in kit["pieces"]:
            out.setdefault(p["id"], p)
        if not kit.get("extends"):
            return out
        path = ROOT / kit["extends"]


class PolishKit(unittest.TestCase):
    def test_contract_is_what_the_script_writes(self):
        self.assertEqual(CONTRACT.read_text(), json.dumps(polish.build(), indent=1) + "\n")

    def test_reuse_only(self):
        kit = load(CONTRACT)
        heat = {p["id"]: p for p in load(KITS / "floor01_heat_kit.json")["pieces"]}
        self.assertEqual(kit["extends"], polish.PARENT)
        self.assertEqual(kit["tiles"], [])
        self.assertEqual([p["id"] for p in kit["pieces"]], polish.HEAT_PIECES)
        for p in kit["pieces"]:
            self.assertEqual(p, heat[p["id"]], p["id"])      # the same definition, no pixel of its own
            self.assertFalse(p["collidable"], p["id"])
        heat_atlases = load(KITS / "floor01_heat_kit.json")["atlases"]
        for name, atlas in kit["atlases"].items():
            self.assertEqual(atlas, heat_atlases[name], name)
        self.assertEqual(sorted(kit["atlases"]), ["floor01_heat_structure", "floor01_heat_wall"])

    def test_placements_name_declared_pieces_on_their_layer_inside_the_room(self):
        kit = load(CONTRACT)
        pieces = declared_pieces()
        for room, block in kit["room_dressing"].items():
            self.assertEqual(block["parallax"], {})
            for place in block["place"] + block["remove_at"]:
                piece = pieces[place["piece"]]
                self.assertEqual(piece["layer"], place["layer"], place)
                if not place["piece"].startswith("torch_pool"):
                    self.assertTrue(0 <= place["x"] and place["x"] + piece["rect"][2] <= ROOM_WIDTH[room], place)
            for fill in block["backwall_fill"]:
                self.assertTrue(all(fill[k] % 16 == 0 for k in "xywh"), fill)

    def test_mastery_arcade_has_no_sill_and_dozer_removes_the_bricked_bays_sill(self):
        kit = load(CONTRACT)["room_dressing"]
        mastery = {p["piece"] for p in kit["mastery"]["place"]}
        self.assertFalse({"arch_threshold", "arch_chain"} & mastery)
        removed = {(p["piece"], p["x"], p["y"]) for p in kit["dozer"]["remove_at"]}
        self.assertEqual(removed, {("arch_threshold", 416, 544), ("arch_chain", 432, 384)})

    def test_juicer_exit_lintel_is_the_last_gym_placement_on_the_doors_top_row(self):
        gym = load(KITS / "floor01_gym_kit.json")
        place = gym["arena_dressing"]["place"]
        lintels = [p for p in place if p["piece"] == "gym_gate_lintel"]
        self.assertEqual(len(lintels), 2)
        self.assertEqual((lintels[0]["x"], lintels[0]["y"]), (0, 180))
        cap = lintels[1]
        door = next(p for p in place if p["piece"] == "gym_exit_closed")
        size = next(p for p in gym["pieces"] if p["id"] == "gym_gate_lintel")["rect"]
        self.assertEqual(cap["y"] + size[3], door["y"])
        self.assertEqual(cap["x"] + size[2], 640)
        self.assertIs(place[-1], cap)


if __name__ == "__main__":
    unittest.main()
