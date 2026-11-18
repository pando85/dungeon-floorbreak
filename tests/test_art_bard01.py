"""BARD-01 checks: the authoring script reproduces the grid and manifest, the .ase matches the grid, the 7 keys match their
frozen digests and the spec's size, layer, spark and tick rules hold (docs/02-art/bard-01-spec.md)."""
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
MANIFEST = ROOT / "assets/manifests/bard_01.json"
KEYS = (0, 3, 7, 8, 10, 12, 15)


class Bard01DesignTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())
        self.fixture = json.loads(FIXTURE.read_text())

    def test_authoring_script_reproduces_the_grid(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())

    def test_manifest_is_reproducible_and_matches_ticks(self) -> None:
        self.assertEqual(json.dumps(author.manifest(), indent=2) + "\n", MANIFEST.read_text())
        rows = json.loads(MANIFEST.read_text())["bard"]["frames"]
        self.assertEqual([r["ticks"] for r in rows], author.TICKS)
        self.assertEqual(sum(t for t in author.TICKS if t), 14 + 10 + 14 + 20 + 30 + 7 + 14 + 12 + 12)

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [64, 56])
        self.assertEqual(len(ase["frames"]), len(self.grid["frames"]))
        self.assertEqual(pg.ase_to_grid(ASE.read_bytes())["frames"], self.grid["frames"])

    def test_sixteen_frames_all_keys_frozen(self) -> None:
        self.assertEqual(len(self.grid["frames"]), 16)
        layers = self.cfg["rules"]["frozen_keys"]["layers"]
        self.assertEqual(sorted(int(k) for k in self.fixture["keys"]), list(KEYS))
        self.assertEqual(self.cfg["rules"]["frozen_keys"]["frames"], list(KEYS))
        for k in KEYS:
            self.assertEqual(frame_digest(self.grid["frames"][k]["cels"], layers), self.fixture["keys"][str(k)]["sha256"], k)

    def test_height_cap_contact_row_layers_and_spark(self) -> None:
        for i, fr in enumerate(self.grid["frames"]):
            cels = fr["cels"]
            opaque = [(x, y) for rows in cels.values() for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
            self.assertGreaterEqual(min(y for _x, y in opaque), 13, f"frame {i} taller than 43 px")
            self.assertEqual(max(y for _x, y in opaque), 55, f"frame {i} off the contact row")
            for y in range(56):
                for b, p in zip(cels["body"][y], cels["prop"][y]):
                    self.assertFalse(b != "." and p != ".", f"frame {i} row {y}")
            y_count = sum(r.count("Y") for rows in cels.values() for r in rows)
            self.assertEqual(y_count > 0, i in (8, 9, 10), f"spark core in frame {i}")
        self.assertEqual(sum(r.count("w") for r in self.grid["frames"][7]["cels"]["prop"]), 7)   # one music note

    def test_projectile_spawn_point_is_the_frame_10_bomb_centre(self) -> None:
        prop = self.grid["frames"][10]["cels"]["prop"]
        self.assertEqual(prop[17][53], "b")           # bomb fuse cap at the top of BOMB7 placed at (50, 17)
        self.assertEqual(prop[20][53], "q")           # bomb centre (53, 20), offset (+25, -35) from the pivot

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
