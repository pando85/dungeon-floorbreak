"""TROG-03 Troglodyte Virtuoso (design stage, key 0): spec rules the shared pixel-art toolkit does not cover (the toolkit
validators run with ``python3 -m scripts.pixelart validate assets/source/characters/troglodyte/virtuoso/trog_03.pixelart.json``)."""
from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_trog01 as t1  # noqa: E402
import author_trog03 as at  # noqa: E402

GRID = ROOT / "assets/source/characters/troglodyte/virtuoso/trog_03.grid.json"
FIXTURE = ROOT / "tests/fixtures/trog-03_keys.json"


class Trog03VirtuosoKey0Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.key0 = cls.grid["frames"][0]["cels"]["body"]
        cls.pose = at.POSES["idle0"]

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), at.dump(at.frames_grid()))

    def test_key0_is_frozen(self):
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertIn("0", keys)
        self.assertEqual(len(keys["0"]["sha256"]), 64)

    def test_frame_pivot_and_family_height(self):
        self.assertEqual(self.grid["canvas"], [128, 88])
        self.assertEqual(self.grid["pivot"], [44, 87])
        ys = [y for y, r in enumerate(self.key0) if r.strip(".")]
        self.assertEqual(max(ys), 87)                        # soles on the contact row
        self.assertEqual(max(ys) - min(ys) + 1, 66)          # family ladder: Virtuoso 66 px, hair included

    def test_palette_is_the_family_core_plus_three_robe_slots(self):
        core = [k for k, *_ in t1.PALETTE]
        self.assertEqual([k for k, *_ in at.PALETTE], core + ["u", "v", "V"])
        self.assertEqual(at.PALETTE[:15], t1.PALETTE)        # the core is shared, never recoloured
        used = {ch for r in self.key0 for ch in r} - {"."}
        self.assertLessEqual(used, set(core) | {"u", "v", "V"})
        self.assertNotIn("g", used)                          # Basher slots never appear here

    def test_head_up_on_a_neck_above_the_hump(self):
        # head width 0.22 to 0.24 H; jaw centre 0.10 to 0.14 H above the hump top and ahead of it (trog-01 spec 2b)
        skull_r, jaw_r = at.HEAD_R
        width = at.HEAD_OFF + skull_r + jaw_r
        self.assertTrue(0.22 * at.H <= width <= 0.24 * at.H, width)
        hx, hy = self.pose["face_at"]
        hump_x, hump_y, hump_r = self.pose["masses"][0]
        self.assertTrue(0.10 * at.H <= hy - (hump_y + hump_r) <= 0.14 * at.H, hy - (hump_y + hump_r))
        self.assertGreater(hx, hump_x)
        (nx0, ny0), (nx1, ny1) = self.pose["neck"]["pts"]
        self.assertGreaterEqual(math.hypot(nx1 - nx0, ny1 - ny0), 0.08 * at.H)

    def test_spine_is_less_hunched(self):
        (x0, y0), _belly, (x1, y1) = self.pose["torso"]["pts"]
        angle = math.degrees(math.atan2(y1 - y0, x1 - x0))
        self.assertTrue(50 <= angle <= 60, angle)

    def test_hands_hang_clear_of_the_floor(self):
        # only the feet touch the contact row; the claws end about 0.15 H above it
        ahead = [y for y, r in enumerate(self.key0) for x, ch in enumerate(r) if ch != "." and x - at.PIVOT[0] >= 14]
        lift = at.PIVOT[1] - max(ahead)
        self.assertTrue(0.12 * at.H <= lift <= 0.18 * at.H, lift)

    def test_idle_accents_read(self):
        flat = "".join(self.key0)
        self.assertGreaterEqual(flat.count("e"), 1)          # bloodshot eye
        self.assertGreaterEqual(flat.count("q"), 6)          # the tongue tip lolls over the lip
        self.assertGreaterEqual(flat.count("t"), 12)         # fangs and gold bells
        self.assertGreaterEqual(flat.count("V"), 20)         # torch-lit robe

    def test_face_templates_have_one_width(self):
        self.assertEqual({len(r) for rows in at.FACES.values() for r in rows}, {13})


if __name__ == "__main__":
    unittest.main()
