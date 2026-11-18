"""RAT-01 Dungeon Rat: spec rules the shared pixel-art toolkit does not cover (run the toolkit validators for the rest:
``python3 -m scripts.pixelart validate assets/source/characters/rat/rat-01.pixelart.json``)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_rat01 as ar  # noqa: E402

GRID = ROOT / "assets/source/characters/rat/rat_h18.grid.json"
MANIFEST = ROOT / "assets/manifests/rat-01.json"
KEYS = (0, 3, 10, 12, 14, 18)


class Rat01Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.rows = [f["cels"]["body"] for f in cls.grid["frames"]]

    def test_committed_files_match_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), ar.dump(ar.frames_grid()))
        self.assertEqual(MANIFEST.read_text(encoding="utf-8"), json.dumps(ar.manifest(), indent=2) + "\n")

    def test_spec_tick_windows(self):
        ticks = {name: sum(t for _n, t in fr if t is not None) for name, fr in ar.TAGS}
        self.assertEqual(ticks, {"idle": 28, "walk": 24, "attack_tell": 24, "attack_swing": 12, "hurt": 12, "down": 12})
        self.assertEqual(len(self.rows), 19)
        rat = self.manifest["rat"]
        self.assertEqual(rat["key_frames"], list(KEYS))
        self.assertEqual([f["author"] for f in rat["frames"] if f["index"] in KEYS], ["key"] * 6)
        # the bite hitbox lives on the impact frame (12) only: 4 ticks right after the 24-tick tell and the 2-tick smear
        self.assertEqual(rat["attack_ticks"], {"tell": [0, 23], "swing_smear": [24, 25], "swing_impact": [26, 29], "recover": [30, 35]})
        self.assertEqual(rat["boxes_px_from_pivot"]["attack_hitbox_impact_frame_only"]["frame"], 12)

    def test_white_eye_pixel_and_reserved_glint(self):
        counts = ["".join(r).count("w") for r in self.rows]
        self.assertEqual([i for i, c in enumerate(counts) if c], [15])
        self.assertEqual(counts[15], 1)
        self.assertEqual(sum("".join(r).count("h") for r in self.rows), 0)   # eye_hot is reserved

    def test_every_frame_touches_the_contact_row_inside_the_pivot_span(self):
        for n, rows in enumerate(self.rows):
            self.assertTrue(rows[27].strip("."), f"frame {n} has no pixel on row 27")

    def test_walk_cycle_alternates_leg_sides(self):
        far = lambda n: "".join(self.rows[n]).count("a")   # noqa: E731  (far-limb ramp pixels)
        self.assertNotEqual(self.rows[3], self.rows[6])
        self.assertNotEqual(self.rows[4], self.rows[7])
        self.assertNotEqual(self.rows[5], self.rows[8])
        self.assertGreater(far(6), far(3))   # contact B draws the extended legs in the far ramp


if __name__ == "__main__":
    unittest.main()
