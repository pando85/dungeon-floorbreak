"""JUICE-01 The Juicer, the 16-frame atlas (phase 2): spec rules the shared pixel-art toolkit does not cover (run the toolkit
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

    def test_six_keys_frozen_at_their_atlas_indices(self):
        sys.path.insert(0, str(ROOT))
        from scripts.pixelart.validators import frame_digest
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(self.grid["frames"]), 16)
        self.assertEqual(sorted(fixture["keys"], key=int), ["0", "3", "8", "10", "12", "15"])
        for index, key in fixture["keys"].items():
            self.assertEqual(key["sha256"], frame_digest(self.grid["frames"][int(index)]["cels"], ["body"]))
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))["juicer"]
        self.assertEqual(manifest["key_atlas_index"], {"0": 0, "3": 3, "8": 8, "10": 10, "12": 12, "15": 15})
        self.assertEqual([f["author"] for f in manifest["frames"]].count("in-between"), 10)
        self.assertEqual([t for _a, _tag, _p, t, _au in aj.FRAMES],
                         [14, 10, 14, 8, 8, 8, 8, 8, 28, 3, 6, 24, 14, 6, 10, None])

    def test_all_sixteen_frames_are_distinct_and_grounded(self):
        digests = {"".join(f["cels"]["body"]) for f in self.grid["frames"]}
        self.assertEqual(len(digests), 16)
        for f in self.grid["frames"]:
            self.assertTrue(f["cels"]["body"][135].strip("."))

    def test_walk_has_a_visible_bob(self):
        def top(i):
            return next(y for y, r in enumerate(self.grid["frames"][i]["cels"]["body"]) if r.strip("."))
        self.assertGreaterEqual(top(3) - top(4), 2)          # contact drops, passing rises: weight shift

    def test_tell_is_tallest_and_pound_lowest_widest(self):
        def box(rows):
            ys = [y for y, r in enumerate(rows) if r.strip(".")]
            xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
            return min(ys), max(xs) - min(xs) + 1
        tops = [box(f["cels"]["body"])[0] for f in self.grid["frames"]]
        self.assertEqual(min(tops), tops[8])                  # the tell (fists overhead) is the tallest frame
        self.assertEqual(max(tops[:13]), tops[10])            # the pound impact is the lowest standing frame
        self.assertGreaterEqual(box(self.grid["frames"][10]["cels"]["body"])[1], 100)

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
