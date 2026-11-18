"""BARD-01 design-stage checks: the authoring script reproduces the grid, the .ase matches it, the idle hero key
matches its frozen digest and the spec's size and layer rules hold (docs/02-art/bard-01-spec.md)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_bard01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.validators import frame_digest  # noqa: E402

CONFIG = ROOT / "assets/source/characters/goblin/bard/bard_01.pixelart.json"
GRID = ROOT / "assets/source/characters/goblin/bard/bard_01.grid.json"
ASE = ROOT / "assets/source/characters/goblin/bard/bard_01.ase"
FIXTURE = ROOT / "tests/fixtures/bard-01_keys.json"


class Bard01DesignTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())
        self.fixture = json.loads(FIXTURE.read_text())

    def test_authoring_script_reproduces_the_grid(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [64, 56])
        self.assertEqual(len(ase["frames"]), len(self.grid["frames"]))

    def test_hero_key_is_frozen(self) -> None:
        layers = self.cfg["rules"]["frozen_keys"]["layers"]
        self.assertEqual(self.fixture["layers"], layers)
        self.assertIn(0, self.cfg["rules"]["frozen_keys"]["frames"])
        self.assertEqual(frame_digest(self.grid["frames"][0]["cels"], layers), self.fixture["keys"]["0"]["sha256"])

    def test_hero_key_size_pivot_and_layers(self) -> None:
        cels = self.grid["frames"][0]["cels"]
        opaque = [(x, y) for rows in cels.values() for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
        self.assertEqual(min(y for _x, y in opaque), 16)      # 40 px idle height
        self.assertEqual(max(y for _x, y in opaque), 55)      # contact row
        for y in range(56):                                   # body and prop never share a pixel
            for b, p in zip(cels["body"][y], cels["prop"][y]):
                self.assertFalse(b != "." and p != ".", f"row {y}")
        self.assertEqual(sum(r.count("Y") for rows in cels.values() for r in rows), 0)  # no spark at idle
        self.assertLessEqual(len(self.grid["palette"]), 20)


if __name__ == "__main__":
    unittest.main()
