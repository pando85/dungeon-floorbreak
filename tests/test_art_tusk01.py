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
GRID_GOWN = ROOT / "assets/source/characters/swine/tusklings/tusk_01_gown.grid.json"
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

    def test_all_frames_match_the_script_and_the_frozen_keys(self) -> None:
        self.assertEqual(len(self.grid["frames"]), 16)
        for i, frame in enumerate(self.grid["frames"]):
            self.assertEqual(frame["cels"], a.render(i), f"frame {i} differs from the script")
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(sorted(int(k) for k in fixture["keys"]), list(a.KEY_FRAMES))
        for k in a.KEY_FRAMES:
            self.assertEqual(fixture["keys"][str(k)]["sha256"],
                             frame_digest(self.grid["frames"][k]["cels"], ["body", "costume"]))

    def test_every_frame_is_clean_grounded_and_in_the_tux_and_gown_atlases(self) -> None:
        gown = json.loads(GRID_GOWN.read_text(encoding="utf-8"))
        for i, (ft, fg) in enumerate(zip(self.grid["frames"], gown["frames"])):
            self.assertEqual(fg["cels"], a.variant(ft["cels"], "gown", i), f"gown frame {i}")
            for label, cels in (("tux", ft["cels"]), ("gown", fg["cels"])):
                flat = a.flatten(cels)
                orphans, isolated = pg._stray_pixels([list(r) for r in flat], a.ACCENTS)
                self.assertEqual((len(orphans), len(isolated)), (0, 0), f"{label} frame {i}")
                self.assertTrue(any(c != "." for c in flat[39]), f"{label} frame {i} misses the contact row")
            self.assertEqual(pix(ft["cels"]["body"], "pq"), [], f"frame {i}: no flower in the tux body")
        self.assertEqual([t for _n, _a, t in a.TAGS], [2, 6, 8, 11, 12, 15])
        self.assertIsNone(a.TICKS[15])  # the down hold is state-driven

    def test_walk_is_a_trotter_cycle_not_a_slide(self) -> None:
        cfg = json.loads((ROOT / "assets/source/characters/swine/tusklings/tusk_01.pixelart.json").read_text())
        self.assertEqual(cfg["rules"]["foot_drift"]["tags"], ["scramble"])
        rows = [a.flatten(self.grid["frames"][i]["cels"])[38] for i in (3, 4, 5, 6)]
        self.assertEqual(len(set(rows)), 4)  # four distinct foot positions

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
        self.assertEqual((min(xs), max(xs)), (9, 42))   # 34 px wide (gate fix: no coat-tail flare), tusk tip at pivot + 19

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
