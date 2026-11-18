"""TUSK-01 Tuskling: hero-key freeze and design rules of docs/02-art/tusk-01-spec.md that the shared pixel-art
validators (``python3 -m scripts.pixelart validate``) do not cover."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_tusk01 as a  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from pixelart.validators import frame_digest  # noqa: E402

GRID = ROOT / "assets/source/characters/swine/tusklings/tusk_01.grid.json"
FIXTURE = ROOT / "tests/fixtures/tusk_01_keys.json"
SWINE_GRID = ROOT / "assets/source/characters/swine/swine-01.grid.json"


def pix(rows, keys):
    return [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c in keys]


class TusklingHeroKeyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text(encoding="utf-8"))
        self.cels = self.grid["frames"][0]["cels"]
        self.key0 = a.flatten(self.cels)

    def test_hero_key_matches_the_script_and_the_frozen_digest(self) -> None:
        self.assertEqual(self.cels, a.render(0))
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["keys"]["0"]["sha256"], frame_digest(self.cels, ["body", "costume"]))

    def test_canvas_pivot_and_layers(self) -> None:
        self.assertEqual(self.grid["canvas"], [56, 40])
        self.assertEqual(self.grid["pivot"], [22, 39])
        self.assertEqual(self.grid["layers"], ["body", "costume"])
        body, cos = self.cels["body"], self.cels["costume"]
        self.assertEqual([p for p in pix(body, set("Kcdsmlhtuvabnorgpqw")) if p in set(pix(cos, set("Kcdsmlhtuvabnorgpqw")))], [])

    def test_hero_key_size(self) -> None:
        ys = [y for _x, y in pix(self.key0, set("Kcdsmlhtuvabnorgpqw"))]
        xs = [x for x, _y in pix(self.key0, set("Kcdsmlhtuvabnorgpqw"))]
        self.assertEqual((min(ys), max(ys)), (10, 39))  # 30 px idle height on the contact row
        self.assertEqual((min(xs), max(xs)), (7, 42))   # 36 px wide, tusk tip at pivot + 19

    def test_family_features_read_like_the_balls_heads(self) -> None:
        self.assertGreaterEqual(len([p for p in pix(self.key0, "uv") if p[0] >= 38]), 6)  # far tusk hooks past the snout
        self.assertGreaterEqual(len(pix(self.key0, "h")), 5)   # lit snout disc
        self.assertGreaterEqual(len(pix(self.key0, "r")), 12)  # gums, tongue and bow tie
        self.assertLessEqual(len(pix(self.key0, "w")), 2)      # small sunk eye, never big cute eyes
        self.assertGreaterEqual(len(pix(self.key0, "ab")), 8)  # coarse bristle on hump and crown
        self.assertEqual(pix(self.key0, "q"), [])              # the flower belongs to the gown variant only

    def test_shared_family_palette_unchanged(self) -> None:
        swine = {p["key"]: p["rgba"] for p in json.loads(SWINE_GRID.read_text(encoding="utf-8"))["palette"]}
        mine = {p["key"]: p["rgba"] for p in self.grid["palette"]}
        self.assertEqual(mine, swine)
        self.assertEqual(len(mine), 19)  # slot 20 stays free

    def test_gown_variant_is_a_clean_rule_on_the_same_body(self) -> None:
        gown = a.variant(self.cels, "gown")
        flat = a.flatten(gown)
        orphans, isolated = pg._stray_pixels([list(r) for r in flat], a.ACCENTS)
        self.assertEqual((len(orphans), len(isolated)), (0, 0))
        self.assertEqual(pix(gown["costume"], "no"), [])       # no black cloth left in the costume
        self.assertGreaterEqual(len(pix(gown["costume"], "r")), 120)  # red sequin gown
        self.assertTrue(1 <= len(pix(flat, "q")) <= 6)          # ear flower is a detail
        self.assertGreaterEqual(len(pix(gown["costume"], "v")), 3)    # pearl string
        same = [(x, y) for y, (r0, r1) in enumerate(zip(self.key0, flat)) for x, (c0, c1) in enumerate(zip(r0, r1))
                if (c0 == ".") != (c1 == ".")]
        self.assertLessEqual(len(same), 4)  # the silhouette is the base body's (only the flower tip may add)


if __name__ == "__main__":
    unittest.main()
