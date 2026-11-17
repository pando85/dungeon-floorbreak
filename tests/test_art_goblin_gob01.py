"""Contract tests for the GOB-01 phase 1 goblin sources (size study and variant A keys)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_goblin_gob01 as ag  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

KEYS = ROOT / "assets/source/characters/goblin/goblin_a_h36_keys.grid.json"
STUDY = ROOT / "assets/source/characters/goblin/goblin_size_study.grid.json"
KEYS_MANIFEST = ROOT / "assets/manifests/goblin_a_h36_keys.json"
WEAPON = set("xyFGHVv")
PINE = set("FGH")


def bbox(flat, keys=None):
    pts = [(x, y) for y, r in enumerate(flat) for x, c in enumerate(r)
           if c != "." and (keys is None or c in keys)]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


class GoblinKeysTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = pg.load_grid(KEYS)
        self.manifest = json.loads(KEYS_MANIFEST.read_text(encoding="utf-8"))
        self.tags = {t["name"]: t["from"] for t in self.grid["tags"]}
        self.flat = {name: pg.composite_keys(self.grid, i) for name, i in self.tags.items()}

    def test_frame_pivot_and_tags_match_manifest(self) -> None:
        self.assertEqual(self.grid["canvas"], self.manifest["frame_size"])
        self.assertEqual(self.grid["pivot"], self.manifest["pivot"])
        self.assertEqual(list(self.tags), self.manifest["animation_tags"])

    def test_idle_is_36_px_on_the_contact_row(self) -> None:
        _x0, y0, _x1, y1 = bbox(self.flat["idle"])
        self.assertEqual(y1, 55)
        self.assertEqual(y1 - y0 + 1, 36)

    def test_grounded_and_airborne_contact(self) -> None:
        for tag in self.manifest["keys"]["grounded_tags"]:
            self.assertEqual(bbox(self.flat[tag])[3], 55, tag)
        for tag in self.manifest["keys"]["airborne_tags"]:
            self.assertLess(bbox(self.flat[tag])[3], 55, tag)

    def test_tell_lifts_the_pineapple_above_the_head_only_in_the_tell(self) -> None:
        idle_top = bbox(self.flat["idle"])[1]
        tell_pine_top = bbox(self.flat["attack_tell"], PINE)[1]
        self.assertLessEqual(tell_pine_top, idle_top - 12)
        for tag in ("idle", "attack_swing", "hurt"):
            self.assertGreater(bbox(self.flat[tag], PINE)[1], idle_top, tag)

    def test_swing_reaches_forward_of_the_pivot(self) -> None:
        px = self.grid["pivot"][0]
        self.assertGreaterEqual(bbox(self.flat["attack_swing"], PINE)[0] - px, 16)

    def test_hurt_uses_eye_whites_not_yellow_eyes(self) -> None:
        flat = self.flat["hurt"]
        self.assertFalse(any(c == "e" for r in flat for c in r))

    def test_lint_has_no_isolated_pixels_and_palette_is_small(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(pg.lint_errors(report), [])
        self.assertLessEqual(len(self.grid["palette"]), 20)
        self.assertEqual({p["key"] for p in self.grid["palette"]} - {k for k, *_ in ag.PALETTE}, set())

    def test_layers_are_disjoint_and_weapon_keys_live_on_weapon(self) -> None:
        for frame in self.grid["frames"]:
            body, weapon = frame["cels"]["body"], frame["cels"]["weapon"]
            for rb, rw in zip(body, weapon):
                for cb, cw in zip(rb, rw):
                    self.assertFalse(cb != "." and cw != ".")
                    self.assertNotIn(cb, WEAPON)


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
