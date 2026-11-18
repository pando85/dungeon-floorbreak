"""HOARD-01 key checks that the shared toolkit does not cover: the authoring script reproduces
the committed grid, the .ase matches it, the six keys are frozen, in-between slots are placeholders only, and the
size and palette decisions of the spec hold."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_hoard01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.validators import freeze  # noqa: E402

CONFIG = ROOT / "assets/source/characters/hoarder/hoard-01.pixelart.json"
GRID = ROOT / "assets/source/characters/hoarder/hoard-01.grid.json"
ASE = ROOT / "assets/source/characters/hoarder/hoard-01.ase"
FIXTURE = ROOT / "tests/fixtures/hoard-01_keys.json"


class Hoard01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text())

    def test_authoring_script_reproduces_the_grid(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [160, 128])
        self.assertEqual(len(ase["frames"]), len(self.grid["frames"]))

    def test_keys_are_frozen(self) -> None:
        cfg = load_config(CONFIG)
        expected = json.loads(FIXTURE.read_text())
        actual = json.loads(json.dumps(freeze(cfg, self.grid, list(author.KEY_FRAMES), ["body"])))
        self.assertEqual(actual["frames"] if "frames" in actual else actual,
                         expected["frames"] if "frames" in expected else expected)

    def test_idle_key_size_contact_and_palette(self) -> None:
        rows = self.grid["frames"][0]["cels"]["body"]
        ys = [y for y, r in enumerate(rows) if r.strip(".")]
        self.assertEqual((min(ys), max(ys)), (16, 127))          # 112 px opaque, feet on the contact row
        self.assertLessEqual(len(self.grid["palette"]), 20)
        orphans, isolated = pg._stray_pixels([list(r) for r in rows], author.ACCENTS)
        self.assertLessEqual(len(orphans), 6)                    # the scar-cluster shadow pixels
        self.assertEqual(len(isolated), 0)

    def test_inbetweens_are_drawn_poses_not_key_copies(self) -> None:
        self.assertEqual(set(author.KEY_FRAMES) | set(author.INBETWEENS), set(range(16)))
        self.assertFalse(set(author.KEY_FRAMES) & set(author.INBETWEENS))
        cels = [json.dumps(f["cels"]) for f in self.grid["frames"]]
        self.assertEqual(len(set(cels)), 16)                      # every frame is unique (V12)
        for f in author.INBETWEENS:
            self.assertIn(f, author.INB)                          # a pose dict, not a placeholder copy

    def test_walk_loop_has_no_foot_pop_at_the_seam(self) -> None:
        """Round 2: no planted foot moves more than 9 px across any walk step, 6 -> 3 included."""
        rows = [f["cels"]["body"][author.PIVOT[1]] for f in self.grid["frames"]]
        seq = [3, 4, 5, 6]
        drifts = [validators.foot_drift(rows[a], rows[b]) for a, b in zip(seq, seq[1:] + seq[:1])]
        self.assertLessEqual(max(drifts), 9, drifts)

    def test_defeat_is_staged_and_recovery_is_not_a_freeze(self) -> None:
        self.assertNotEqual(self.grid["frames"][11]["cels"], self.grid["frames"][10]["cels"])
        self.assertGreaterEqual(author.TICKS[14], 10)             # floor impact held 10-12 ticks
        tops = [min(y for y, r in enumerate(self.grid["frames"][f]["cels"]["body"]) if r.strip(".")) for f in (13, 14, 15)]
        self.assertEqual(tops, sorted(tops))                       # the body only goes down: 13 -> 14 -> 15

    def test_plain_shirt_has_no_print_colour(self) -> None:
        # the shirt is plain: no palette entry exists for a print or logo
        names = [e["name"] for e in self.grid["palette"]]
        self.assertFalse([n for n in names if "logo" in n or "print" in n])


if __name__ == "__main__":
    unittest.main()
