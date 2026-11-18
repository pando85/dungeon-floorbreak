"""BROOD-01 Scatterer Brood Guardian: key-freeze and hero-key rules of docs/02-art/brood-01-spec.md
that the shared pixel-art validators (``python3 -m scripts.pixelart validate``) do not cover."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_brood01 as a  # noqa: E402
from pixelart.validators import frame_digest  # noqa: E402

GRID = ROOT / "assets/source/characters/scatterer/brood/brood-01.grid.json"
FIXTURE = ROOT / "tests/fixtures/brood-01_keys.json"
SCAT_PALETTE = ROOT / "assets/source/characters/scatterer/scat-01.grid.json"


def pix(rows, keys):
    return [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c in keys]


class BroodGuardianKeyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text(encoding="utf-8"))
        self.key0 = self.grid["frames"][0]["cels"]["body"]

    def test_hero_key_matches_the_script_and_the_frozen_digest(self) -> None:
        self.assertEqual(self.key0, a.frame_idle0())
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["keys"]["0"]["sha256"], frame_digest(self.grid["frames"][0]["cels"], ["body"]))

    def test_hero_key_size_and_canvas(self) -> None:
        self.assertEqual((len(self.key0), len(self.key0[0])), (44, 80))
        ys = [y for _x, y in pix(self.key0, set(a.ACCENTS) | set("KabcdfhgjuvoH"))]
        self.assertEqual((min(ys), max(ys)), (14, 43))  # 30 px idle height, standing on the contact row
        self.assertLess(max(ys) - min(ys) + 1, 40)  # stays under the 40 px difficulty threshold

    def test_hero_key_canon_features(self) -> None:
        self.assertEqual(len(pix(self.key0, "r")), 4)  # the red eye
        self.assertGreaterEqual(len(pix(self.key0, "o")), 12)  # the carried egg case
        self.assertEqual(pix(self.key0, "w"), [])  # flash eye only on hurt

    def test_shared_species_palette(self) -> None:
        scat = {p["key"]: p["rgba"] for p in json.loads(SCAT_PALETTE.read_text(encoding="utf-8"))["palette"]}
        mine = {p["key"]: p["rgba"] for p in self.grid["palette"]}
        self.assertEqual({k: mine[k] for k in scat}, scat)  # the 16 Scatterer keys are reused unchanged
        self.assertLessEqual(len(set(mine) - set(scat)), 4)
        self.assertLessEqual(len(mine), 20)


if __name__ == "__main__":
    unittest.main()
