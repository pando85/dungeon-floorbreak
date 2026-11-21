"""MORDECAI-01 checks: the authoring script reproduces grid and manifest, the .ase matches the grid, the two keys match their
frozen digests, and the spec's size, pivot, tick and loop rules hold (docs/02-art/mordecai-01-spec.md)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_mordecai01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.validators import frame_digest  # noqa: E402

DIR = ROOT / "assets/source/characters/mordecai"
CONFIG = DIR / "mordecai_h44.pixelart.json"
GRID = DIR / "mordecai_h44.grid.json"
ASE = DIR / "mordecai_h44.ase"
FIXTURE = ROOT / "tests/fixtures/mordecai-01_keys.json"
MANIFEST = ROOT / "assets/manifests/mordecai_h44.json"


def opaque(cels):
    return [(x, y) for rows in cels.values() for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]


class Mordecai01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())
        self.fixture = json.loads(FIXTURE.read_text())

    def test_authoring_script_reproduces_grid_and_manifest(self) -> None:
        self.assertEqual(json.dumps(author.build(), indent=1) + "\n", GRID.read_text())
        self.assertEqual(json.dumps(author.manifest(), indent=2) + "\n", MANIFEST.read_text())

    def test_ase_matches_the_grid(self) -> None:
        self.assertEqual(pg.read_ase(ASE.read_bytes())["canvas"], [52, 48])
        self.assertEqual(pg.ase_to_grid(ASE.read_bytes())["frames"], self.grid["frames"])

    def test_keys_are_frozen(self) -> None:
        layers = self.cfg["rules"]["frozen_keys"]["layers"]
        self.assertEqual(sorted(int(k) for k in self.fixture["keys"]), [0, 4])
        for k in (0, 4):
            self.assertEqual(frame_digest(self.grid["frames"][k]["cels"], layers), self.fixture["keys"][str(k)]["sha256"], k)

    def test_size_pivot_contact_row_and_height_cap(self) -> None:
        self.assertEqual(self.grid["canvas"], [52, 48])
        self.assertEqual(self.grid["pivot"], [24, 47])
        tops = []
        for i, fr in enumerate(self.grid["frames"]):
            pts = opaque(fr["cels"])
            tops.append(min(y for _x, y in pts))
            self.assertEqual(max(y for _x, y in pts), 47, f"frame {i} off the contact row")
        self.assertEqual(tops[0], 4)                      # idle key: 44 px, rows 4 to 47
        self.assertEqual(tops[4], 4)
        self.assertGreaterEqual(min(tops), 3)             # at most 45 px
        self.assertEqual((tops[1], tops[3]), (3, 5))      # inhale up, exhale down

    def test_loop_ticks_and_tags(self) -> None:
        self.assertEqual(author.TICKS, [40, 22, 14, 22, 60])
        self.assertEqual(sum(author.TICKS[:4]), 98)
        tags = {t["name"]: (t["from"], t["to"]) for t in self.grid["tags"]}
        self.assertEqual(tags, {"idle": (0, 3), "talk": (4, 4)})
        rows = json.loads(MANIFEST.read_text())["mordecai"]["frames"]
        self.assertEqual([r["ticks"] for r in rows], author.TICKS)
        self.assertEqual([f["duration_ms"] for f in self.grid["frames"]], [round(t * 1000 / 60) for t in author.TICKS])

    def test_legs_and_tail_root_do_not_move_in_the_idle_loop(self) -> None:
        for layer in ("legs",):
            first = self.grid["frames"][0]["cels"][layer]
            for i in (1, 2, 3):
                self.assertEqual(self.grid["frames"][i]["cels"][layer], first, (layer, i))

    def test_palette_and_pointers(self) -> None:
        self.assertLessEqual(len(self.grid["palette"]), 20)
        m = json.loads(MANIFEST.read_text())
        self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
        self.assertEqual(m["animation_tags"], ["idle", "talk"])
        self.assertEqual(m["sha256"]["grid_source"], author._sha(GRID))
        self.assertEqual(m["sha256"]["source_path"], author._sha(ASE))


if __name__ == "__main__":
    unittest.main()
