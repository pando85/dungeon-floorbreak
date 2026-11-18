"""JUICE-01 The Juicer, the six keys (phase 2): spec rules the shared pixel-art toolkit does not cover (run the toolkit
validators for the rest: ``python3 -m scripts.pixelart validate
assets/source/characters/troglodyte/juicer/juice_01.pixelart.json``)."""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_juice01 as aj  # noqa: E402
import author_trog01 as at  # noqa: E402

GRID = ROOT / "assets/source/characters/troglodyte/juicer/juice_01.grid.json"
MANIFEST = ROOT / "assets/manifests/juice-01.json"
FIXTURE = ROOT / "tests/fixtures/juice-01_keys.json"


class Juice01IdleKeyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.rows = cls.grid["frames"][0]["cels"]["body"]

    def test_committed_files_match_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), aj.dump(aj.frames_grid()))
        self.assertEqual(MANIFEST.read_text(encoding="utf-8"), json.dumps(aj.manifest(), indent=2) + "\n")

    def test_six_keys_frozen_in_atlas_order(self):
        sys.path.insert(0, str(ROOT))
        from scripts.pixelart.validators import frame_digest
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(self.grid["frames"]), 6)
        for i, frame in enumerate(self.grid["frames"]):
            self.assertEqual(fixture["keys"][str(i)]["sha256"], frame_digest(frame["cels"], ["body"]))
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))["juicer"]
        self.assertEqual(manifest["key_atlas_index"], {"0": 0, "1": 3, "2": 8, "3": 10, "4": 12, "5": 15})
        self.assertEqual([t for _a, _tag, _p, t in aj.KEYS], [14, 8, 28, 6, 14, None])

    def test_tell_is_tallest_and_pound_lowest_widest(self):
        def box(rows):
            ys = [y for y, r in enumerate(rows) if r.strip(".")]
            xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
            return min(ys), max(xs) - min(xs) + 1
        tops = [box(f["cels"]["body"])[0] for f in self.grid["frames"]]
        self.assertEqual(min(tops), tops[2])                  # the tell (fists overhead) is the tallest frame
        self.assertEqual(max(tops[:5]), tops[3])              # the pound impact is the lowest standing frame
        self.assertGreaterEqual(box(self.grid["frames"][3]["cels"]["body"])[1], 100)

    def test_size_frame_pivot(self):
        self.assertEqual(self.grid["canvas"], [160, 136])
        self.assertEqual(self.grid["pivot"], [72, 135])
        ys = [y for y, r in enumerate(self.rows) if r.strip(".")]
        self.assertEqual(max(ys) - min(ys) + 1, 100)        # orchestrator default height, pending owner confirmation
        self.assertEqual(max(ys), 135)                      # feet on the contact row
        self.assertGreaterEqual(100, round(66 * 1.2))       # out-masses the Basher (66 px) by at least 20 %

    def test_palette_shares_the_family_core(self):
        core = {k: rgba for k, _n, rgba, _a in at.PALETTE}
        used = {p["key"]: p["rgba"] for p in self.grid["palette"]}
        self.assertLessEqual(len(aj.PALETTE), 20)
        for key, rgba in used.items():
            if key in core:
                self.assertEqual(rgba, core[key], key)      # the 15 core colours are never recoloured
        self.assertTrue({"K", "m", "l", "H", "c", "w", "e", "t", "o"} <= set(used))
        self.assertTrue({"b", "v", "i", "j"} <= set(used))  # boils, veins, iron


if __name__ == "__main__":
    unittest.main()
