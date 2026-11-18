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
MANIFEST = ROOT / "assets/manifests/brood-01.json"
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

    def test_committed_grid_matches_the_authoring_script(self) -> None:
        frames = [f["cels"]["body"] for f in self.grid["frames"]]
        self.assertEqual(frames, a.all_frames())
        self.assertEqual(len(frames), 16)

    def test_default_composition_reproduces_key_0(self) -> None:
        self.assertEqual(a.despeckle(a.key0()), a.frame_idle0())
        self.assertEqual(self.key0, a.frame_idle0())

    def test_no_egg_case_anywhere(self) -> None:
        self.assertNotIn("o", {p["key"] for p in self.grid["palette"]})  # palette is 17 of 20
        for i, f in enumerate(fr["cels"]["body"] for fr in self.grid["frames"]):
            self.assertEqual(pix(f, "o"), [], i)

    def test_all_six_keys_are_frozen_and_distinct(self) -> None:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(sorted(int(k) for k in fixture["keys"]), [0, 3, 8, 10, 12, 15])
        for k, rec in fixture["keys"].items():
            self.assertEqual(rec["sha256"], frame_digest(self.grid["frames"][int(k)]["cels"], ["body"]), k)
        self.assertEqual(len({tuple(self.grid["frames"][i]["cels"]["body"]) for i in (0, 3, 8, 10, 12, 15)}), 6)

    def test_tick_table_and_manifest_are_the_spec_table(self) -> None:
        self.assertEqual(a.TICKS, [14, 10, 14, 6, 6, 6, 6, 8, 24, 3, 6, 18, 12, 6, 10, 0])
        m = json.loads(MANIFEST.read_text(encoding="utf-8"))["brood"]
        self.assertEqual([r["ticks"] for r in m["frames"]], [t or None for t in a.TICKS])
        self.assertEqual(m["boxes"]["attack_hitbox"], [66, 30, 14, 12])
        self.assertEqual(m["boxes"]["standing"], [16, 15, 46, 29])  # active on frame 10 only
        self.assertEqual(m["key_frames"], [0, 3, 8, 10, 12, 15])

    def test_fang_reach_only_on_the_impact_frame(self) -> None:
        frames = [f["cels"]["body"] for f in self.grid["frames"]]
        reach = [[x for x, _y in pix(f, "H") if x >= 76] for f in frames]
        self.assertEqual([i for i, r in enumerate(reach) if r], [10])
        self.assertGreaterEqual(len(reach[10]), 4)

    def test_defeat_has_no_explosion_cue(self) -> None:
        frames = [f["cels"]["body"] for f in self.grid["frames"]]
        for i in (13, 14, 15):
            self.assertEqual(pix(frames[i], "w"), [], i)  # no flash
            self.assertLessEqual(len(pix(frames[i], "H")), 3 if i == 15 else 9, i)  # mandible edges only, no glint or burst
        for i in (14, 15):
            self.assertEqual(pix(frames[i], "r"), [], i)  # no glowing eye once settled

    def test_every_frame_stays_on_the_canvas_and_grounded(self) -> None:
        for i, f in enumerate(f["cels"]["body"] for f in self.grid["frames"]):
            self.assertEqual((len(f), len(f[0])), (44, 80), i)
            self.assertTrue(any(c != "." for c in f[43]), f"frame {i} has nothing on the contact row")

    def test_hero_key_size_and_canvas(self) -> None:
        self.assertEqual((len(self.key0), len(self.key0[0])), (44, 80))
        ys = [y for _x, y in pix(self.key0, set(a.ACCENTS) | set("KabcdfhgjuvH"))]
        self.assertEqual((min(ys), max(ys)), (10, 43))  # 34 px idle height, standing on the contact row
        self.assertLess(max(ys) - min(ys) + 1, 40)  # stays under the 40 px difficulty threshold

    def test_hero_key_canon_features(self) -> None:
        self.assertEqual(len(pix(self.key0, "r")), 5)  # the 3 x 3 red eye (5 r, 1 H glint, 3 e)
        self.assertEqual(pix(self.key0, "o"), [])  # no egg case (cut in design round 2)
        self.assertGreaterEqual(len([q for q in pix(self.key0, "H") if q[1] >= 36]), 5)  # mandibles at shin height
        self.assertEqual(pix(self.key0, "w"), [])  # flash eye only on hurt

    def test_shared_species_palette(self) -> None:
        scat = {p["key"]: p["rgba"] for p in json.loads(SCAT_PALETTE.read_text(encoding="utf-8"))["palette"]}
        mine = {p["key"]: p["rgba"] for p in self.grid["palette"]}
        self.assertEqual({k: mine[k] for k in scat}, scat)  # the 16 Scatterer keys are reused unchanged
        self.assertLessEqual(len(set(mine) - set(scat)), 4)
        self.assertLessEqual(len(mine), 20)


if __name__ == "__main__":
    unittest.main()
