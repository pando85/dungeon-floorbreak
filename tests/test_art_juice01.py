"""JUICE-01 The Juicer, phase 1 (idle key 0): spec rules the shared pixel-art toolkit does not cover (run the toolkit
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

    def test_key0_is_frozen(self):
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        digest = hashlib.sha256("\n".join(self.rows).encode("utf-8")).hexdigest()
        self.assertIn("0", fixture["keys"])
        self.assertEqual(len(fixture["keys"]["0"]["sha256"]), len(digest))

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
