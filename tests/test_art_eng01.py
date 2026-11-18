"""ENG-01 Goblin Engineer (design stage, key 0): spec rules the shared pixel-art toolkit does not cover (the toolkit
validators run with ``python3 -m scripts.pixelart validate assets/source/characters/goblin/engineer/eng_01.pixelart.json``)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as ae  # noqa: E402

GRID = ROOT / "assets/source/characters/goblin/engineer/eng_01.grid.json"
FIXTURE = ROOT / "tests/fixtures/eng-01_keys.json"
GOBLIN = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
RIDER = ROOT / "assets/source/enemies/dozer/dozer_riders_h36.grid.json"


def merged(cels: dict[str, list[str]]) -> list[str]:
    rows = [list(r) for r in next(iter(cels.values()))]
    for layer in cels.values():
        for y, r in enumerate(layer):
            for x, ch in enumerate(r):
                if ch != ".":
                    rows[y][x] = ch
    return ["".join(r) for r in rows]


class Eng01Key0Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.key0 = merged(cls.grid["frames"][0]["cels"])
        cls.goblin = json.loads(GOBLIN.read_text(encoding="utf-8"))
        cls.rider = json.loads(RIDER.read_text(encoding="utf-8"))

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), ae.dump(ae.frames_grid()))

    def test_key0_is_frozen(self):
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertIn("0", keys)
        self.assertEqual(len(keys["0"]["sha256"]), 64)

    def test_frame_pivot_and_40px_height(self):
        self.assertEqual(self.grid["canvas"], [64, 56])
        self.assertEqual(self.grid["pivot"], [28, 55])
        ys = [y for y, r in enumerate(self.key0) if r.strip(".")]
        self.assertEqual((min(ys), max(ys)), (16, 55))       # 40 px: GOB-01 36 px body + 4 pot rows above the crown
        self.assertEqual(ae.H, 40)

    def test_palette_reuses_goblin_and_rider_keys(self):
        gob = {p["key"]: p["rgba"] for p in self.goblin["palette"]}
        rid = {p["key"]: p["rgba"] for p in self.rider["palette"]}
        self.assertLessEqual(len(ae.PALETTE), 20)
        for key, _name, rgba, _acc in ae.PALETTE:
            self.assertEqual(rgba, gob.get(key, rid.get(key)), key)   # no new colour in the design stage
        used = {ch for r in self.key0 for ch in r} - {"."}
        self.assertTrue({"x", "y", "F", "G", "H", "V", "v"}.isdisjoint(used))   # no stick, pineapple or leaves

    def test_pot_is_the_rider_pot(self):
        # rows 16 to 23 (saucepan, handle) are the rider's engineer_hold rows 25 to 32, x 13 to 42, pixel for pixel
        rider = merged(self.rider["frames"][2]["cels"])
        for y in range(16, 24):
            self.assertEqual(self.key0[y][13:43], rider[y + 9][13:43], y)
        handle = [x for x in range(13, 26) if self.key0[20][x] != "."]
        self.assertLess(max(handle), 27)                     # handle out back (left in the authored facing)

    def test_body_below_the_chin_is_gob01(self):
        body = merged({"body": self.goblin["frames"][0]["cels"]["body"]})
        for y in list(range(28, 35)) + list(range(44, 56)):   # row 43 only loses the stick strap's end pixel
            self.assertEqual(self.key0[y][19:64], body[y][19:64], y)

    def test_wrench_is_a_stowed_prop(self):
        prop = self.grid["frames"][0]["cels"]["prop"]
        cells = [(x, y) for y, r in enumerate(prop) for x, ch in enumerate(r) if ch != "."]
        self.assertTrue(cells)
        xs = [x for x, _ in cells]
        ys = [y for _, y in cells]
        self.assertLessEqual(max(xs), 18)                    # behind the cape outline, never in the hand
        self.assertTrue(35 <= min(ys) and max(ys) <= 42)     # back hip, ends in the belt loop on row 42
        self.assertEqual({prop[y][x] for x, y in cells} - set("K234567"), set())


if __name__ == "__main__":
    unittest.main()
