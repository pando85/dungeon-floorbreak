"""TROG-02 Troglodyte Basher (key 0 and the execution stage): spec rules the shared pixel-art toolkit does not cover (the toolkit
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


class Trog02BasherExecutionTests(unittest.TestCase):
    """Execution stage: keys 3, 8, 10, 11, 12, 15 and the nine in-betweens (spec sections 4, 8, 9 and 14)."""

    KEYS = (0, 3, 8, 10, 11, 12, 15)

    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.cels = [f["cels"]["body"] for f in cls.grid["frames"]]
        cls.manifest = json.loads((ROOT / "assets/manifests/trog-02.json").read_text(encoding="utf-8"))

    @staticmethod
    def top(rows):
        return min(y for y, r in enumerate(rows) if r.strip("."))

    @staticmethod
    def width(rows):
        xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
        return max(xs) - min(xs) + 1

    def test_committed_files_match_the_generator(self):
        self.assertEqual((ROOT / "assets/manifests/trog-02.json").read_text(encoding="utf-8"),
                         json.dumps(at.manifest(), indent=2) + "\n")

    def test_sixteen_frames_with_the_spec_tick_windows(self):
        self.assertEqual(len(self.cels), 16)
        ticks = {n: t for tag, fr in at.TAGS for n, t in fr}
        by_tag = {tag: [t for _n, t in fr] for tag, fr in at.TAGS}
        self.assertEqual(sum(by_tag["idle"]), 38)
        self.assertEqual(sum(by_tag["walk"]), 32)
        self.assertEqual(sum(by_tag["attack_tell"]), 36)
        self.assertEqual(sum(by_tag["attack_smash"]), 11)
        self.assertEqual(by_tag["attack_recover"], [20])
        self.assertEqual(by_tag["hurt"], [14])
        self.assertEqual(by_tag["down"], [5, 10, None])
        self.assertEqual(ticks[8], 28)                       # the wind-up hold is the read window
        rows = {r["index"]: r["ticks"] for r in self.manifest["basher"]["frames"]}
        self.assertEqual(rows, ticks)
        self.assertEqual(self.manifest["basher"]["key_frames"], list(self.KEYS))

    def test_keys_are_frozen_and_in_betweens_are_not_keys(self):
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertEqual(sorted(int(k) for k in keys), list(self.KEYS))
        for k in self.KEYS:
            self.assertEqual(len(keys[str(k)]["sha256"]), 64)
        self.assertEqual(sorted(at.KEY_FRAMES), list(self.KEYS))
        self.assertEqual(sorted(at.INBETWEEN), [1, 2, 4, 5, 6, 7, 9, 13, 14])

    def test_every_frame_is_grounded_and_unique(self):
        for n, rows in enumerate(self.cels):
            self.assertTrue(rows[87].strip("."), f"frame {n} does not touch the contact row")
        self.assertEqual(len({"".join(r) for r in self.cels}), 16)

    def test_tell_is_the_tallest_and_the_impact_the_lowest_attack_pose(self):
        tops = [self.top(r) for r in self.cels]
        self.assertEqual(min(tops), tops[8])                  # the wind-up pillar is the tallest frame of the set
        self.assertGreater(tops[10], tops[9])
        self.assertGreater(tops[10], tops[8] + 25)            # tallest to lowest, told apart by height alone
        self.assertGreaterEqual(tops[11], tops[10] + 4)       # the spent head is lower than the impact head
        self.assertLess(tops[12], tops[0])                    # the hurt recoil is taller than the idle
        self.assertGreaterEqual(self.width(self.cels[10]), 70)
        self.assertLessEqual(self.width(self.cels[8]), 50)

    def test_faces_follow_the_spec_per_frame(self):
        count = lambda n, ch: "".join(self.cels[n]).count(ch)
        for n in range(0, 9):
            self.assertGreaterEqual(count(n, "e"), 1, n)      # awake red eye
        for n in (7,):
            self.assertLess(count(n, "o"), 18, n)             # the mouth opens only on the wind-up (snarl on 7)
        for n in (8, 9, 10):
            self.assertGreaterEqual(count(n, "o"), 10, n)     # roar
        self.assertLessEqual(count(11, "o"), 16)              # pant
        for n in range(12, 16):
            self.assertEqual(count(n, "e"), 0, n)             # no red eye in hurt and defeat
        self.assertGreaterEqual(count(15, "q"), 3)            # tongue out
        self.assertEqual(at.POSES[at.KEY_FRAMES[12]]["face"], "pain")
        self.assertEqual(at.POSES[at.INBETWEEN[13]]["face"], "pain")
        self.assertEqual(at.POSES[at.KEY_FRAMES[11]]["face"], "pant")
        self.assertEqual([at.pose_for(n)["face"] for n in (7, 8, 9, 10)], ["snarl", "roar", "roar", "roar"])
        self.assertEqual([at.pose_for(n)["face"] for n in (14, 15)], ["dead", "dead"])

    def test_hurt_head_sits_above_the_trapezius_and_nowhere_else_does(self):
        def head_over_mass(n):
            pose = at.pose_for(n)
            return pose["face_at"][1] - max(y + r for _x, y, r in pose["masses"])
        self.assertGreater(head_over_mass(12), -3)
        self.assertLess(head_over_mass(0), -7)
        self.assertLess(head_over_mass(8), -7)

    def test_wraps_stay_visible_wherever_the_forearm_is(self):
        for n in range(0, 15):
            self.assertGreaterEqual("".join(self.cels[n]).count("G"), 8, n)
        self.assertGreaterEqual("".join(self.cels[15]).count("G"), 8)

    def test_keys_carry_eight_to_ten_grime_smudges(self):
        for k in self.KEYS[1:]:
            self.assertTrue(8 <= len(at.pose_for(k)["grime"]) <= 10, k)

    def test_walk_is_a_stomp_cycle_with_a_limb_swap_not_a_slide(self):
        def planted(n, lo, hi):
            return any(ch != "." for ch in self.cels[n][87][48 + lo:48 + hi])
        self.assertTrue(planted(3, 28, 38))                    # near fist planted far ahead in contact A
        self.assertTrue(planted(6, 30, 38))                    # far fist planted ahead in contact B
        self.assertFalse(planted(5, 30, 40))                   # passing: no fist far ahead
        self.assertNotEqual(self.cels[3], self.cels[6])
        self.assertEqual(at.pose_for(6)["far_fingers"], at.planted_fist(30, 8))
        self.assertEqual(at.pose_for(3)["far_fingers"], at.planted_fist(2, 8))     # the far fist trails in A
        cfg = json.loads((ROOT / "assets/source/characters/troglodyte/basher/trog_02.pixelart.json").read_text())
        self.assertEqual(cfg["rules"]["foot_drift"], {"max_px": 12, "tags": ["walk"]})

    def test_in_betweens_copy_the_nearer_key(self):
        self.assertEqual(at.pose_for(2)["torso"], at.POSES["idle0"]["torso"])      # 2: key 0 with the ropes swayed
        self.assertNotEqual(at.pose_for(1)["hair"], at.POSES["idle0"]["hair"])
        self.assertEqual(self.top(self.cels[1]), self.top(self.cels[0]) + 1)       # breath: head and masses 1 px lower
        self.assertEqual(self.top(self.cels[4]) + 1, self.top(self.cels[3]))       # walk 4: body up 1 px
        self.assertLessEqual(self.top(self.cels[7]), 22)                           # rise: already as tall as the idle
        self.assertGreater(self.top(self.cels[7]), self.top(self.cels[8]))
        self.assertGreaterEqual(self.top(self.cels[13]), 40)                       # fall beat is below the hurt head
        self.assertEqual(self.top(self.cels[14]) + 1, self.top(self.cels[15]))     # impact bounce: 1 px up

    def test_no_rope_point_above_the_crown_except_the_hurt_whip(self):
        for n in (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11):
            hd = at.pose_for(n)["hair"]
            crown = max(y for _x, y in hd["mop"])
            for rope in hd["ropes"]:
                self.assertLessEqual(max(y for _x, y in rope), crown, n)


if __name__ == "__main__":
    unittest.main()
