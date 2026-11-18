"""TROG-03 Troglodyte Virtuoso (16 frames): spec rules the shared pixel-art toolkit does not cover (the toolkit
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
        self.assertEqual(sorted(keys, key=int), ["0", "3", "8", "10", "11", "12", "15"])
        self.assertEqual(sorted(at.KEY_FRAMES), [0, 3, 8, 10, 11, 12, 15])
        for entry in keys.values():
            self.assertEqual(len(entry["sha256"]), 64)

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


class Trog03VirtuosoSetTests(unittest.TestCase):
    """The whole set: ticks, grounding, the tongue frames and the walk contact plan (spec sections 4, 8 and 9)."""

    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.cels = [f["cels"]["body"] for f in cls.grid["frames"]]

    @staticmethod
    def extent(rows):
        pts = [(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        return min(xs), max(xs), min(ys), max(ys)

    def test_tick_windows(self):
        ticks = at.frame_ticks()
        total = lambda tag: sum(t for n, t in dict(at.TAGS)[tag])  # noqa: E731
        self.assertEqual(len(self.cels), 16)
        self.assertEqual({tag: total(tag) for tag in ("idle", "walk", "attack_tell", "attack_tongue", "attack_recover", "hurt")},
                         {"idle": 34, "walk": 32, "attack_tell": 30, "attack_tongue": 8, "attack_recover": 18, "hurt": 14})
        self.assertEqual((ticks[13], ticks[14], ticks[15]), (5, 10, None))

    def test_every_frame_is_grounded_and_inside_the_canvas(self):
        for n, rows in enumerate(self.cels):
            x0, x1, y0, y1 = self.extent(rows)
            self.assertEqual(y1, 87, n)
            self.assertGreaterEqual(x0, 0, n)
            self.assertLess(x1, 128, n)

    def test_key0_drool_hangs_from_the_tongue_tip(self):
        key0 = self.cels[0]
        col = at.PIVOT[0] + 13
        self.assertEqual(key0[42][col], "q")
        self.assertEqual([key0[y][col] for y in (43, 44, 45)], ["w", "w", "w"])
        self.assertEqual(sum(row[at.PIVOT[0] + 15] == "w" for row in key0[41:44]), 0)   # nothing floats at x +15

    def test_feint_is_tallest_and_strike_is_the_widest_frame(self):
        top = lambda n: self.extent(self.cels[n])[2]  # noqa: E731
        width = lambda n: self.extent(self.cels[n])[1] - self.extent(self.cels[n])[0] + 1  # noqa: E731
        self.assertLess(top(8), min(top(n) for n in (7, 9, 10, 11)))
        self.assertGreaterEqual(width(10), 92)
        self.assertEqual(max(range(16), key=width), 10)
        # tongue tip x +74 to +77 and the first 45 px straight and level (within 2 px of y 43)
        self.assertTrue(74 <= self.extent(self.cels[10])[1] - at.PIVOT[0] <= 77)

    def test_strike_and_retract_tongues_differ_by_shape(self):
        def tongue_rows(n):
            return [y for y, r in enumerate(self.cels[n]) for x, ch in enumerate(r) if ch == "q" and x - at.PIVOT[0] > 28]
        strike, retract = tongue_rows(10), tongue_rows(11)
        self.assertLessEqual(max(strike) - min(strike), 12)       # level, then the curled tip
        self.assertGreater(max(retract) - min(retract), 20)       # hangs down to the floor

    def test_walk_contact_plan_planted_feet_move_back_six_per_frame(self):
        def runs(n):
            row, out, x = self.cels[n][87], [], 0
            while x < len(row):
                if row[x] == ".":
                    x += 1
                    continue
                a = x
                while x < len(row) and row[x] != ".":
                    x += 1
                out.append((a + x - 1) / 2 - at.PIVOT[0])
            return out
        self.assertEqual(runs(3), [-4, 8])
        self.assertEqual(runs(4), [2])
        self.assertEqual(runs(5), [-4, 8])
        self.assertEqual(runs(6), [2])
        # tracked by foot identity from the generator: the near foot 8, 2, -4 (frames 3, 4, 5), the far foot 8, 2, -4 (5, 6, 3)
        plan = at.WALK_PLAN
        near = [plan[n][1] for n in (3, 4, 5)]
        far = [plan[n][2] for n in (5, 6, 3)]
        for cx in (near, far):
            self.assertEqual([c[0] for c in cx], [8, 2, -4])
            self.assertEqual([c[1] for c in cx], [0, 0, 0])               # planted
        self.assertGreater(plan[6][1][0], 2)                               # the lifted foot passes 1 to 2 px ahead of the planted one
        self.assertGreater(plan[4][2][0], 2)
        self.assertGreater(plan[6][1][1], 0)
        self.assertGreater(plan[4][2][1], 0)
        # seam 6 to 3: the far foot moves 2 to -4
        self.assertEqual(plan[3][2][0] - plan[6][2][0], -6)
        # the 2 px bob: passing frames sit 2 px higher than contact frames
        self.assertEqual(self.extent(self.cels[3])[2] - self.extent(self.cels[4])[2], 2)

    def test_claws_stay_clear_of_the_floor_in_the_walk(self):
        for n in (3, 4, 5, 6):
            ahead = [y for y, r in enumerate(self.cels[n]) for x, ch in enumerate(r) if ch != "." and x - at.PIVOT[0] >= 14]
            self.assertGreaterEqual(at.PIVOT[1] - max(ahead), 7, n)

    def test_hurt_and_fallen_frames_have_no_red_eye_and_the_hurt_has_no_tongue(self):
        for n in (12, 13, 14, 15):
            self.assertEqual("".join(self.cels[n]).count("e"), 0, n)
        self.assertLessEqual("".join(self.cels[12]).count("q"), 4)
        self.assertGreaterEqual("".join(self.cels[15]).count("q"), 16)


if __name__ == "__main__":
    unittest.main()
