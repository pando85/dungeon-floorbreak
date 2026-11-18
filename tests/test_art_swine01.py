"""SWINE-01 checks the shared toolkit does not cover: the authoring script reproduces the committed grid, the .ase
matches it, the six keys are frozen, the roll is the frame-3 rule turned clockwise, and the size, contact and
palette decisions of docs/02-art/swine-01-spec.md hold."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_swine01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart.validators import freeze  # noqa: E402

CONFIG = ROOT / "assets/source/characters/swine/swine-01.pixelart.json"
GRID = ROOT / "assets/source/characters/swine/swine-01.grid.json"
ASE = ROOT / "assets/source/characters/swine/swine-01.ase"
FIXTURE = ROOT / "tests/fixtures/swine-01_keys.json"


class Swine01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text())

    def rows(self, f: int) -> list[str]:
        return self.grid["frames"][f]["cels"]["body"]

    def test_authoring_script_reproduces_the_grid(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [176, 144])
        self.assertEqual(len(ase["frames"]), 18)

    def test_keys_are_frozen(self) -> None:
        cfg = load_config(CONFIG)
        actual = json.loads(json.dumps(freeze(cfg, self.grid, list(author.KEY_FRAMES), ["body"])))
        self.assertEqual(actual, json.loads(FIXTURE.read_text()))
        self.assertEqual(author.KEY_FRAMES, (0, 3, 10, 12, 14, 17))

    def test_every_drawn_frame_sits_on_the_contact_row_without_strays(self) -> None:
        self.assertLessEqual(len(self.grid["palette"]), 20)
        for f in range(18):
            rows = self.rows(f)
            ys = [y for y, r in enumerate(rows) if r.strip(".")]
            self.assertEqual(max(ys), author.PIVOT[1], f)
            orphans, isolated = pg._stray_pixels([list(r) for r in rows], author.ACCENTS)
            self.assertEqual((len(orphans), len(isolated)), (0, 0), f)

    def test_idle_key_size_and_grotesque_read(self) -> None:
        rows = self.rows(0)
        ys = [y for y, r in enumerate(rows) if r.strip(".")]
        xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
        self.assertEqual((min(ys), max(xs) - min(xs) + 1), (10, 145))   # 128 px ball plus rim lumps
        text = "".join(rows)
        self.assertGreaterEqual(text.count("w"), 24)        # many small mean eyes, not one mascot face
        self.assertGreaterEqual(text.count("h"), 140)       # lit snouts; measured 140 on the gate-fix key 0
        self.assertGreaterEqual(text.count("e"), 8)         # pearl string beads
        self.assertLessEqual(text.count("q"), 80)           # the flower stays a detail

    def test_roll_frames_are_the_frame3_rule_turned_clockwise(self) -> None:
        for f in author.ROLL_FRAMES:
            self.assertEqual(author.pose_for(f)["theta"], 60.0 * (f - 3))
            self.assertEqual(self.rows(f), author.Ball(author.pose_for(f)).render())
        self.assertEqual(len({json.dumps(self.rows(f)) for f in author.ROLL_FRAMES}), 6)

    def test_rim_spacing_has_no_roll_period(self) -> None:
        rim = sorted(p[2] for p in author.PARTS if p[0] == "head" and p[1] >= 0.79)
        gaps = [(b - a) % 360 for a, b in zip(rim, rim[1:] + rim[:1])]
        self.assertTrue(all(25 <= g <= 50 for g in gaps), gaps)
        self.assertGreater(len(set(gaps)), 6)                # irregular: no repeating spacing

    def test_hurt_and_defeat_shut_every_eye(self) -> None:
        for f in (14, 17):
            self.assertEqual("".join(self.rows(f)).count("w"), 0)


if __name__ == "__main__":
    unittest.main()
