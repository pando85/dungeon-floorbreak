"""ANIM-MONGO-01 (design stage): structural and identity checks on the locomotion grid.

These pin what the art must keep (approved idle, master head pixels, palette, pivot and
contact convention, frozen keys, the run footfall and the 1 px head bob). They do not judge
artistic quality.
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

MASTER = ROOT / "assets/source/characters/mongo/mongo_adult_h76.grid.json"
GRID = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json"
ASE = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.ase"
MANIFEST = ROOT / "assets/manifests/mongo_adult_h76_locomotion.json"
CONFIG = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.pixelart.json"
KEYS = [4, 5, 6, 10, 12, 13, 14]
G = 87


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pixels(rows: list[str]) -> set[tuple[int, int, str]]:
    return {(x, y, c) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def normalised(px):
    x0 = min(x for x, _, _ in px)
    y0 = min(y for _, y, _ in px)
    return {(x - x0, y - y0, c) for x, y, c in px}


class MongoLocomotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.master = load(MASTER)
        cls.grid = load(GRID)
        cls.manifest = load(MANIFEST)
        pg.validate_grid(cls.grid)
        cls.tags = {t["name"]: (t["from"], t["to"]) for t in cls.grid["tags"]}

    def cels(self, i: int, layer: str) -> list[str]:
        w, h = self.grid["canvas"]
        return self.grid["frames"][i]["cels"].get(layer, ["." * w] * h)

    def test_manifest_and_layout(self) -> None:
        self.assertEqual(self.grid["canvas"], [128, 88])
        self.assertEqual(self.grid["pivot"], [76, 87])
        self.assertEqual(self.manifest["frame_size"], self.grid["canvas"])
        self.assertEqual(self.manifest["pivot"], self.grid["pivot"])
        self.assertEqual(self.manifest["review_status"], "prototype_fixture_unapproved")
        self.assertEqual(list(self.tags), ["idle", "run", "jump_start", "jump_rise", "jump_fall", "land"])
        self.assertEqual([b - a + 1 for a, b in self.tags.values()], [4, 6, 2, 1, 1, 2])
        self.assertEqual(self.grid["layers"], self.master["layers"])

    def test_palette_is_the_master_palette(self) -> None:
        self.assertEqual(self.grid["palette"], self.master["palette"])

    def test_idle_pixels_are_the_master_idle(self) -> None:
        # master 108x80 placed at (8, 8): master pivot (68, 79) -> (76, 87)
        for i in range(4):
            mine = pg.composite_keys(self.grid, i)
            ref = pg.composite_keys(self.master, i)
            for y in range(88):
                for x in range(128):
                    my, mx = y - 8, x - 8
                    want = ref[my][mx] if 0 <= my < 80 and 0 <= mx < 108 else "."
                    self.assertEqual(mine[y][x], want, f"idle {i} ({x},{y})")

    def test_head_is_a_rigid_copy_of_the_master_head(self) -> None:
        ref = normalised(pixels(self.master["frames"][0]["cels"]["head"]))
        for i in range(4, 16):
            self.assertEqual(normalised(pixels(self.cels(i, "head"))), ref, f"frame {i}")

    def test_keys_match_the_authoring_recipe(self) -> None:
        out = subprocess.run([sys.executable, str(ROOT / "scripts/author_mongo_locomotion.py"), "--check-keys"],
                             capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)

    def test_ase_round_trips_to_the_grid(self) -> None:
        self.assertEqual(pg.normalize(pg.ase_to_grid(ASE.read_bytes())), pg.normalize(self.grid))

    def test_validators_pass(self) -> None:
        cfg = load_config(CONFIG)
        report = validators.run(validators.build_context(cfg))
        self.assertTrue(report["passed"], validators.summary(report))

    def contact(self, i: int) -> dict[str, tuple[int, int]]:
        out = {}
        for layer in ("near_leg", "far_leg"):
            xs = [x for x, c in enumerate(self.cels(i, layer)[G]) if c != "."]
            if xs:
                out[layer] = (min(xs), max(xs))
        return out

    def test_run_footfall(self) -> None:
        # one foot on the floor in contact and passing frames, none in the airborne frames
        self.assertEqual(self.contact(4), {"near_leg": (84, 103)})
        self.assertEqual(self.contact(5), {"near_leg": (68, 87)})
        self.assertEqual(self.contact(6), {})
        self.assertEqual(self.contact(7), {"far_leg": (84, 103)})
        self.assertEqual(self.contact(8), {"far_leg": (68, 87)})
        self.assertEqual(self.contact(9), {})
        # planted foot travels back exactly 16 px per frame (the design D)
        self.assertEqual(self.contact(4)["near_leg"][0] - self.contact(5)["near_leg"][0], 16)

    def test_jump_frames_are_airborne_and_land_frames_grounded(self) -> None:
        report = pg.lint_grid(self.grid)
        bottoms = [f["bottom_row"] for f in report["frames"]]
        for i in (6, 9, 12, 13):
            self.assertLess(bottoms[i], G, f"frame {i}")
        for i in (0, 1, 2, 3, 4, 5, 7, 8, 10, 11, 14, 15):
            self.assertEqual(bottoms[i], G, f"frame {i}")

    def test_run_head_bob_is_at_most_one_pixel(self) -> None:
        tops = []
        for i in range(4, 10):
            rows = self.cels(i, "head")
            tops.append(min(y for y, r in enumerate(rows) if r.strip(".")))
        self.assertLessEqual(max(tops) - min(tops), 1, tops)

    def test_crouch_and_impact_differ(self) -> None:
        report = pg.lint_grid(self.grid)
        top = [f["bbox"][1] for f in report["frames"]]
        self.assertGreaterEqual(top[14] - top[10], 3, "impact must sit lower than the crouch")

    def test_clean_clusters(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(report["isolated_count_per_frame"], [0] * 16)
        for i in KEYS:
            self.assertLessEqual(report["orphan_count_per_frame"][i], 2, f"key {i}")


if __name__ == "__main__":
    unittest.main()
