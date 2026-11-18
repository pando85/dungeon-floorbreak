"""SWINE-01 phase-1 checks (silhouette gate): the authoring script reproduces the committed grid, the .ase matches
it, idle key 0 is frozen, and the size, contact and palette decisions of docs/02-art/swine-01-spec.md hold."""
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
        self.rows = self.grid["frames"][0]["cels"]["body"]

    def test_authoring_script_reproduces_the_grid(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [176, 144])
        self.assertEqual(len(ase["frames"]), len(self.grid["frames"]))

    def test_key0_is_frozen(self) -> None:
        cfg = load_config(CONFIG)
        expected = json.loads(FIXTURE.read_text())
        self.assertEqual(json.loads(json.dumps(freeze(cfg, self.grid, [0], ["body"]))), expected)

    def test_idle_key_size_contact_and_palette(self) -> None:
        ys = [y for y, r in enumerate(self.rows) if r.strip(".")]
        xs = [x for r in self.rows for x, ch in enumerate(r) if ch != "."]
        self.assertEqual(max(ys), author.PIVOT[1])                       # sits on the contact row
        self.assertTrue(124 <= max(ys) - min(ys) + 1 <= 134)              # 128 px ball plus rim lumps
        self.assertTrue(128 <= max(xs) - min(xs) + 1 <= 148)
        self.assertLessEqual(len(self.grid["palette"]), 20)
        orphans, isolated = pg._stray_pixels([list(r) for r in self.rows], author.ACCENTS)
        self.assertEqual((len(orphans), len(isolated)), (0, 0))

    def test_ball_reads_as_many_tusked_faces(self) -> None:
        text = "".join(self.rows)
        self.assertGreaterEqual(text.count("w"), 24)        # many small mean eyes, not one mascot face
        self.assertGreaterEqual(text.count("v") + text.count("u"), 250)   # ivory tusks are the brightest read
        self.assertGreaterEqual(text.count("h"), 140)       # lit snout discs
        for key in "nrq":                                     # top hat, sequins, flower stay in the mass
            self.assertGreater(text.count(key), 10)


if __name__ == "__main__":
    unittest.main()
