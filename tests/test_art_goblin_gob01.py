"""Contract tests for the GOB-01 goblin sources (size study, variant A and B keys)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_goblin_gob01 as ag  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN = ROOT / "assets/source/characters/goblin"
STUDY = GOBLIN / "goblin_size_study.grid.json"
WEAPON = set("xyFGHVv")
FRUIT = set("FG")  # pineapple body without the H facet / arc-trail key
IDLE_HEAD_TOP = 20


def bbox(flat, keys=None):
    pts = [(x, y) for y, r in enumerate(flat) for x, c in enumerate(r)
           if c != "." and (keys is None or c in keys)]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


class _Keys:
    variant = "a"

    def setUp(self) -> None:
        self.grid = pg.load_grid(GOBLIN / f"goblin_{self.variant}_h36_keys.grid.json")
        self.manifest = json.loads((ROOT / f"assets/manifests/goblin_{self.variant}_h36_keys.json")
                                   .read_text(encoding="utf-8"))
        self.atlas = self.manifest["keys"]["atlas_frames"]
        self.f = {a: pg.composite_keys(self.grid, i) for i, a in enumerate(self.atlas)}
        self.tag_of = {}
        for t in self.grid["tags"]:
            for i in range(t["from"], t["to"] + 1):
                self.tag_of[self.atlas[i]] = t["name"]

    def test_frame_pivot_tags_match_manifest(self) -> None:
        self.assertEqual(self.grid["canvas"], self.manifest["frame_size"])
        self.assertEqual(self.grid["pivot"], self.manifest["pivot"])
        self.assertEqual([t["name"] for t in self.grid["tags"]], self.manifest["animation_tags"])
        self.assertEqual(len(self.grid["frames"]), len(self.atlas))

    def test_idle_is_36_px_on_the_contact_row(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[0])
        self.assertEqual((y0, y1), (IDLE_HEAD_TOP, 55))

    def test_contact_rows(self) -> None:
        for a, flat in self.f.items():
            low = bbox(flat)[3]
            if self.tag_of[a] in self.manifest["keys"]["airborne_tags"]:
                self.assertLess(low, 55, a)
            else:
                self.assertEqual(low, 55, a)

    def test_r1_pineapple_above_the_head_only_in_tell_and_smear(self) -> None:
        self.assertLessEqual(bbox(self.f[11], FRUIT)[1], IDLE_HEAD_TOP - 12)
        for a in (0, 4, 7, 13, 16, 22, 25):
            self.assertGreater(bbox(self.f[a], FRUIT)[1], IDLE_HEAD_TOP, a)

    def test_smear_pineapple_overhead(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[12], FRUIT)
        self.assertTrue(26 <= x0 and x1 <= 31 and 2 <= y0 and y1 <= 10, (x0, y0, x1, y1))

    def test_swing_reads_as_an_overhead_bonk(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[13], FRUIT)
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        self.assertTrue(50 <= cx <= 53 and 31 <= cy <= 37, (cx, cy))
        self.assertGreaterEqual(x0 - self.grid["pivot"][0], 16)
        row55 = [x for x, c in enumerate(self.f[13][55]) if c != "." and x > self.grid["pivot"][0]]
        self.assertTrue(row55 and min(row55) >= 35 and max(row55) <= 42, row55)

    def test_hurt_eye_whites_and_toe_only_front_foot(self) -> None:
        flat = self.f[16]
        self.assertFalse(any(c == "e" for r in flat for c in r))
        front = [x for x, c in enumerate(flat[55]) if c != "." and x > self.grid["pivot"][0]]
        self.assertLessEqual(len(front), 5)
        heel = [x for x, c in enumerate(flat[53]) if c != "." and self.grid["pivot"][0] < x < min(front)]
        self.assertTrue(heel, "the lifted heel sits above row 55, behind the toe")

    def test_knockback_is_compact_with_the_head_leading(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[19])
        self.assertLessEqual(x1 - x0 + 1, 36)
        self.assertGreaterEqual(y1 - y0 + 1, 22)
        self.assertLessEqual(y1, 50)
        lead = {self.f[19][y][x0] for y in range(56)} - {"."}
        self.assertFalse(lead & WEAPON)
        tongue = sum(1 for r in self.f[19] for c in r if c == "R")
        self.assertGreaterEqual(tongue, 6)

    def test_lint_palette_and_orphans(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(pg.lint_errors(report), [])
        self.assertLessEqual(len(self.grid["palette"]), 20)
        self.assertTrue(all(n < 10 for n in report["orphan_count_per_frame"]), report["orphan_count_per_frame"])
        self.assertEqual({p["key"] for p in self.grid["palette"]} - {k for k, *_ in ag.PALETTE}, set())

    def test_layers_disjoint_and_weapon_keys_on_weapon(self) -> None:
        for frame in self.grid["frames"]:
            body, weapon = frame["cels"]["body"], frame["cels"]["weapon"]
            for rb, rw in zip(body, weapon):
                for cb, cw in zip(rb, rw):
                    self.assertFalse(cb != "." and cw != ".")
                    self.assertNotIn(cb, WEAPON)


class VariantAKeysTests(_Keys, unittest.TestCase):
    variant = "a"


class VariantBKeysTests(_Keys, unittest.TestCase):
    variant = "b"

    def test_b_is_a_plus_an_accessory_layer(self) -> None:
        a = pg.load_grid(GOBLIN / "goblin_a_h36_keys.grid.json")
        self.assertEqual(self.grid["layers"], ["body", "weapon", "accessory"])
        for fa, fb in zip(a["frames"], self.grid["frames"]):
            self.assertEqual(fa["cels"]["body"], fb["cels"]["body"])
            self.assertEqual(fa["cels"]["weapon"], fb["cels"]["weapon"])
            acc = "".join(fb["cels"]["accessory"])
            self.assertIn("R", acc)
            self.assertIn("B", acc)
        self.assertEqual(a["tags"], self.grid["tags"])


class GoblinSizeStudyTests(unittest.TestCase):
    def test_generator_matches_committed_grid(self) -> None:
        self.assertEqual(STUDY.read_text(encoding="utf-8"), ag.dump(ag.size_study_grid()))

    def test_option_heights(self) -> None:
        grid = pg.load_grid(STUDY)
        for i, h in enumerate(ag.HEIGHTS):
            _x0, y0, _x1, y1 = bbox(pg.composite_keys(grid, i))
            self.assertEqual((y1, y1 - y0 + 1), (55, h))


if __name__ == "__main__":
    unittest.main()
