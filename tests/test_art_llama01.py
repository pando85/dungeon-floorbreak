"""LLAMA-01 spec rules that the shared toolkit (``python3 -m scripts.pixelart validate``) does not cover:
the authoring record is reproducible, the .ase matches the grid, in-betweens stay out of the frozen set, the
spec timing contract holds and the review strip includes the spit projectile."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_llama01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

CONFIG = ROOT / "assets/source/characters/llama/llama-01.pixelart.json"
GRID = ROOT / "assets/source/characters/llama/llama-01.grid.json"
ASE = ROOT / "assets/source/characters/llama/llama-01.ase"
MANIFEST = ROOT / "assets/manifests/llama-01.json"
SPEC_TICKS = {"idle": 38, "walk": 24, "attack_tell": 30, "attack_swing": 20, "hurt": 12, "spit": 8}


class Llama01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())

    def test_authoring_script_reproduces_grid_and_manifest(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())
        self.assertEqual(json.dumps(author.build_manifest(), indent=2) + "\n", MANIFEST.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [96, 72])
        self.assertEqual(len(ase["frames"]), 17)
        self.assertEqual([f["duration_ms"] for f in ase["frames"]], [f["duration_ms"] for f in self.grid["frames"]])

    def test_inbetweens_are_not_frozen_and_keys_are_disjoint(self) -> None:
        frozen = set(self.cfg["rules"]["frozen_keys"]["frames"])
        self.assertEqual(frozen, set(author.KEY_FRAMES))
        self.assertEqual(frozen | set(author.INBETWEENS), set(range(17)))
        self.assertFalse(frozen & set(author.INBETWEENS))

    def test_tick_windows_match_the_spec(self) -> None:
        for tag, total in SPEC_TICKS.items():
            self.assertEqual(sum(t for _f, t in self.cfg["tags"][tag]["frames"]), total, tag)
        self.assertGreaterEqual(dict(map(tuple, self.cfg["tags"]["attack_tell"]["frames"]))[8], 18)  # glow peak
        self.assertIsNone(dict(map(tuple, self.cfg["tags"]["down"]["frames"]))[14])                  # defeat hold

    def test_review_strip_includes_the_spit_projectile(self) -> None:
        self.assertEqual(self.cfg["review"]["strip_tags"], ["attack_tell", "attack_swing", "spit"])
        self.assertEqual(self.cfg["tags"]["spit"]["contact"], "free")

    def test_every_grounded_frame_touches_row_71(self) -> None:
        for name, tag in self.cfg["tags"].items():
            if tag["contact"] != "grounded":
                continue
            for f, _t in tag["frames"]:
                rows = [y for y, r in enumerate(self.grid["frames"][f]["cels"]["body"]) if r.strip(".")]
                self.assertEqual(max(rows), 71, f"{name} frame {f}")


if __name__ == "__main__":
    unittest.main()
