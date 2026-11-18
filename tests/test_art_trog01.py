"""TROG-01 Troglodyte Pygmy: spec rules the shared pixel-art toolkit does not cover (run the toolkit validators for the rest:
``python3 -m scripts.pixelart validate assets/source/characters/troglodyte/trog_01.pixelart.json``)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_trog01 as at  # noqa: E402

GRID = ROOT / "assets/source/characters/troglodyte/trog_01.grid.json"
MANIFEST = ROOT / "assets/manifests/trog-01.json"
FIXTURE = ROOT / "tests/fixtures/trog-01_keys.json"
KEYS = (0, 3, 8, 10, 12, 15)


class Trog01PygmyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.rows = [f["cels"]["body"] for f in cls.grid["frames"]]

    def test_committed_files_match_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), at.dump(at.frames_grid()))
        self.assertEqual(MANIFEST.read_text(encoding="utf-8"), json.dumps(at.manifest(), indent=2) + "\n")

    def test_spec_tick_windows(self):
        ticks = {name: sum(t for _n, t in fr if t is not None) for name, fr in at.TAGS}
        self.assertEqual(ticks, {"idle": 32, "walk": 24, "attack_tell": 24, "attack_leap": 9, "attack_land": 14,
                                 "hurt": 12, "down": 12})
        self.assertEqual(len(self.rows), 16)
        pygmy = self.manifest["pygmy"]
        self.assertEqual(pygmy["key_frames"], list(KEYS))
        self.assertEqual([f["author"] for f in pygmy["frames"] if f["index"] in KEYS], ["key"] * 6)
        # tell 24 ticks (0-23), launch 3 (24-26), strike with the hitbox active 6 (27-32), land and recover 14 (33-46)
        self.assertEqual(pygmy["attack_ticks"], {"tell": [0, 23], "leap_launch": [24, 26], "leap_strike_impact": [27, 32],
                                                 "land_recover": [33, 46]})
        self.assertEqual(pygmy["boxes_px_from_pivot"]["attack_hitbox_impact_frame_only"]["frame"], 10)

    def test_size_frame_pivot_and_palette_budget(self):
        self.assertEqual(self.grid["canvas"], [80, 56])
        self.assertEqual(self.grid["pivot"], [30, 55])
        idle_rows = [y for y, r in enumerate(self.rows[0]) if r.strip(".")]
        self.assertEqual(max(idle_rows) - min(idle_rows) + 1, 40)       # the decided idle opaque height
        self.assertEqual(max(idle_rows), 55)                             # feet on the contact row
        self.assertLessEqual(len(self.grid["palette"]), 15)             # five of the 20 slots stay free for the variants

    def test_leap_is_airborne_and_every_other_frame_is_grounded(self):
        for n, rows in enumerate(self.rows):
            on_floor = bool(rows[55].strip("."))
            self.assertEqual(on_floor, n not in (9, 10), f"frame {n}")

    def test_hurt_and_defeat_eyes_are_not_red_and_tell_mouth_is_open(self):
        count = lambda n, ch: "".join(self.rows[n]).count(ch)   # noqa: E731
        self.assertEqual([count(n, "e") for n in (12, 13, 14, 15)], [0, 0, 0, 0])
        self.assertGreaterEqual(count(0, "e"), 1)
        self.assertGreaterEqual(count(8, "o"), 8)
        self.assertGreater(count(8, "o"), count(0, "o"))

    def test_tell_is_the_lowest_upright_grounded_pose(self):
        top = lambda n: min(y for y, r in enumerate(self.rows[n]) if r.strip("."))   # noqa: E731
        self.assertGreater(top(8), top(3))      # lower than the scuttle
        self.assertGreater(top(3), top(0))      # the scuttle is lower than the idle
        self.assertLess(top(12), top(0))        # the hurt snap is the tallest

    def test_landing_frame_is_the_tell_crouch_plus_dust_only(self):
        diff = [(y, x) for y, (a, b) in enumerate(zip(self.rows[8], self.rows[11])) for x, (p, q) in enumerate(zip(a, b)) if p != q]
        self.assertTrue(diff)
        self.assertLessEqual(len(diff), 12)
        self.assertGreaterEqual(sum(self.rows[11][y][x] in "cC" for y, x in diff), 5)   # the dust pixels

    def test_frozen_keys_match_the_fixture(self):
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(sorted(int(k) for k in fixture["keys"]), list(KEYS))


if __name__ == "__main__":
    unittest.main()
