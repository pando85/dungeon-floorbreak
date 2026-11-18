"""TROG-02 Troglodyte Basher (design stage): spec rules the shared pixel-art toolkit does not cover (the toolkit
validators run with ``python3 -m scripts.pixelart validate assets/source/characters/troglodyte/basher/trog_02.pixelart.json``)."""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_trog01 as t1  # noqa: E402
import author_trog02 as at  # noqa: E402

GRID = ROOT / "assets/source/characters/troglodyte/basher/trog_02.grid.json"
FIXTURE = ROOT / "tests/fixtures/trog-02_keys.json"


class Trog02BasherKey0Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.key0 = cls.grid["frames"][0]["cels"]["body"]

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), at.dump(at.frames_grid()))

    def test_key0_is_frozen(self):
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertIn("0", keys)
        self.assertEqual(len(keys["0"]["sha256"]), 64)

    def test_frame_pivot_and_family_height(self):
        self.assertEqual(self.grid["canvas"], [112, 88])
        self.assertEqual(self.grid["pivot"], [48, 87])
        ys = [y for y, r in enumerate(self.key0) if r.strip(".")]
        self.assertEqual(max(ys), 87)                        # knuckles and soles on the contact row
        self.assertEqual(max(ys) - min(ys) + 1, 66)          # family ladder: Basher 66 px (trog-01 spec 2b)

    def test_palette_is_the_family_core_plus_two_basher_slots(self):
        core = [k for k, *_ in t1.PALETTE]
        self.assertEqual([k for k, *_ in at.PALETTE], core + ["g", "G"])
        self.assertEqual(at.PALETTE[:15], t1.PALETTE)        # the core is shared, never recoloured
        used = {ch for r in self.key0 for ch in r} - {"."}
        self.assertLessEqual(used, set(core) | {"g", "G"})
        self.assertNotIn("q", used)                          # tongue stays out of the idle

    def test_small_head_on_a_big_body(self):
        # head width 0.25 H (16 to 17 px), never above 0.27 H
        skull_r, jaw_r = at.HEAD_R
        width = 4.5 + skull_r + jaw_r
        self.assertTrue(0.24 * at.H <= width <= 0.27 * at.H, width)
        # the jaw centre is sunk 0.12 to 0.14 H (8 to 9 px) below the trapezius peak and ahead of it (spec section 2:
        # deeper than the 0.05 to 0.08 H first set in the trog-01 spec 2b, flagged there for review)
        hx, hy = at.POSES["idle0"]["face_at"]
        hump_x, hump_top = max(((x, y + r) for x, y, r in at.POSES["idle0"]["masses"]), key=lambda m: m[1])
        self.assertTrue(0.12 * at.H <= hump_top - hy <= 0.14 * at.H, hump_top - hy)
        self.assertGreater(hx, hump_x)

    def test_idle_accents_read(self):
        flat = "".join(self.key0)
        self.assertGreaterEqual(flat.count("e"), 1)          # bloodshot eye
        self.assertGreaterEqual(flat.count("t"), 10)         # teeth, tusks and necklace
        self.assertGreaterEqual(flat.count("G"), 12)         # forearm wraps

    def test_face_templates_have_one_width(self):
        self.assertEqual({len(r) for rows in at.FACES.values() for r in rows}, {14})
        self.assertEqual({"snarl", "roar", "pant", "pain", "dead"}, set(at.FACES))


if __name__ == "__main__":
    unittest.main()
