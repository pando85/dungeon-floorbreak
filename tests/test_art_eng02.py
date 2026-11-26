"""ENG-02 Goblin Engineer completion, design keys (docs/02-art/eng-02-spec.md): the two frozen get-up keys and the
part rules the executor builds on. The toolkit validators run with
``python3 -m scripts.pixelart validate assets/source/characters/goblin/engineer/eng_02_keys.pixelart.json``."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as e1  # noqa: E402
import author_eng02 as e2  # noqa: E402

FIXTURE = ROOT / "tests/fixtures/eng-02_keys.json"
CONFIG = ROOT / "assets/source/characters/goblin/engineer/eng_02_keys.pixelart.json"


def merged(cels: dict[str, list[str]]) -> list[str]:
    rows = [list(r) for r in next(iter(cels.values()))]
    for layer in cels.values():
        for y, r in enumerate(layer):
            for x, ch in enumerate(r):
                if ch != ".":
                    rows[y][x] = ch
    return ["".join(r) for r in rows]


class Eng02KeyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(e2.GRID.read_text(encoding="utf-8"))
        cls.sit = merged(cls.grid["frames"][0]["cels"])
        cls.shove = merged(cls.grid["frames"][1]["cels"])

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(e2.GRID.read_text(encoding="utf-8"), e2.dump(e2.frames_grid()))
        self.assertTrue(e2.ASE.is_file())

    def test_both_keys_are_frozen(self):
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["asset_id"], "eng-02-keys")
        self.assertEqual(sorted(fixture["keys"], key=int), ["0", "1"])
        cfg = json.loads(CONFIG.read_text(encoding="utf-8"))
        self.assertEqual(cfg["rules"]["frozen_keys"]["frames"], [0, 1])

    def test_canvas_pivot_palette_match_eng01(self):
        self.assertEqual(self.grid["canvas"], [64, 56])
        self.assertEqual(self.grid["pivot"], [28, 55])
        self.assertEqual([p["key"] for p in self.grid["palette"]], [k for k, *_ in e1.PALETTE])

    def test_shove_body_is_the_literal_hero_key(self):
        for y, row in e2.KEY_SHOVE.items():
            self.assertEqual(self.shove[y], row, y)
        self.assertFalse(any(r.strip(".") for r in self.shove[:16]))

    def test_getup_timing_fills_the_22_tick_window(self):
        self.assertEqual(sum(e2.TICKS), 22)
        self.assertEqual(e2.ATLAS_INDEX, (18, 19))
        self.assertEqual([f["duration_ms"] for f in self.grid["frames"]], [e2.duration_ms(t) for t in e2.TICKS])

    def test_sit_hides_the_eyes_and_drops_the_wrench(self):
        self.assertEqual(sum(r.count("e") for r in self.sit), 0)
        self.assertGreaterEqual(sum(r.count("w") for r in self.sit), 2)
        prop = self.grid["frames"][0]["cels"]["prop"]
        self.assertFalse(any(r.strip(".") for r in prop))

    def test_shove_carries_the_wrench_on_the_prop_layer(self):
        prop = self.grid["frames"][1]["cels"]["prop"]
        self.assertTrue(any(r.strip(".") for r in prop))
        self.assertGreaterEqual(sum(r.count("e") for r in self.shove), 3)

    def test_pot_part_and_flat_pot_rules(self):
        pot = e2.pot_part()
        xs, ys = [x for x, _ in pot], [y for _, y in pot]
        self.assertEqual((min(xs), max(xs), min(ys), max(ys)), (1, 30, 12, 20))   # whole pan inside the 32 x 32 cell
        self.assertEqual(sum(ch == "7" for ch in pot.values()), 1)
        flat = e2.flat_pot()
        fys = {y for _, y in flat}
        self.assertEqual(fys, {0, 1, 2, 3})                                       # 4-row pancake
        fxs = [x for x, _ in flat]
        self.assertLessEqual(max(fxs) - min(fxs) + 1, 30)                          # iron does not stretch
        self.assertEqual(sum(ch == "7" for ch in flat.values()), 0)

    def test_death_palette_adds_three_gore_keys(self):
        keys = [k for k, *_ in e2.DEATH_PALETTE]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual(keys[-3:], ["R", "B", "P"])
        self.assertLessEqual(len(keys), 20)


if __name__ == "__main__":
    unittest.main()
