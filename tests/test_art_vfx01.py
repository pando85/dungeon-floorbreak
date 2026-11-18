"""VFX-01 spec rules that the shared toolkit (``python3 -m scripts.pixelart validate``) does not cover: the authoring
record is reproducible, the .ase matches the grid, in-betweens stay out of the frozen set, the 60 Hz tick contract
holds, the palette shares the Bard's and the Rot Sticker's colours, and the glow is baked and capped."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_vfx01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

CONFIG = ROOT / "assets/source/vfx/explosion/vfx_01.pixelart.json"
GRID = ROOT / "assets/source/vfx/explosion/vfx_01.grid.json"
ASE = ROOT / "assets/source/vfx/explosion/vfx_01.ase"
MANIFEST = ROOT / "assets/manifests/vfx-01.json"
FIXTURE = ROOT / "tests/fixtures/vfx-01_keys.json"
BARD_GRID = ROOT / "assets/source/characters/goblin/bard/bard_01.grid.json"
SPEC_TICKS = {"bomb_flight": 12, "burst": 42}


def hexes(grid: dict) -> dict[str, str]:
    return {e["key"]: e["rgba"] for e in grid["palette"]}


class Vfx01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())

    def test_authoring_script_reproduces_grid_and_manifest(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())
        self.assertEqual(json.dumps(author.build_manifest(), indent=2) + "\n", MANIFEST.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [48, 48])
        self.assertEqual(len(ase["frames"]), 11)
        self.assertEqual([f["duration_ms"] for f in ase["frames"]], [f["duration_ms"] for f in self.grid["frames"]])

    def test_keys_are_frozen_and_inbetweens_are_copies(self) -> None:
        frozen = set(self.cfg["rules"]["frozen_keys"]["frames"])
        self.assertEqual(frozen, set(author.KEY_FRAMES))
        self.assertEqual(json.loads(FIXTURE.read_text())["keys"].keys(), {str(k) for k in frozen})
        self.assertFalse(frozen & set(author.COPIES))
        self.assertEqual(frozen | set(author.COPIES), set(range(11)))
        for slot, key in author.COPIES.items():
            self.assertEqual(self.grid["frames"][slot]["cels"], self.grid["frames"][key]["cels"])

    def test_tick_windows_match_the_spec(self) -> None:
        for tag, total in SPEC_TICKS.items():
            self.assertEqual(sum(t for _f, t in self.cfg["tags"][tag]["frames"]), total, tag)
        burst = dict(map(tuple, self.cfg["tags"]["burst"]["frames"]))
        lo, hi = author.DAMAGE_ACTIVE_TICKS
        self.assertEqual(sum(t for f, t in burst.items() if f <= 5), hi - lo + 1)   # frames 3-5 are the placeholder window
        self.assertEqual(self.cfg["tags"]["bomb_flight"]["playback"], "loop")
        self.assertEqual(self.cfg["tags"]["burst"]["playback"], "once")
        self.assertEqual({t["contact"] for t in self.cfg["tags"].values()}, {"free"})

    def test_frame_and_pivot_are_centred(self) -> None:
        self.assertEqual(self.cfg["frame_size"], [48, 48])
        self.assertEqual(self.cfg["pivot"], [24, 24])
        self.assertEqual(self.grid["pivot"], [24, 24])

    def test_palette_is_small_and_shares_the_bard_colours(self) -> None:
        pal = hexes(self.grid)
        self.assertLessEqual(len(pal), 20)
        bard = hexes(json.loads(BARD_GRID.read_text()))
        for key in ("K", "q", "Q", "n", "b", "h", "Y"):
            self.assertEqual(pal[key], bard[key], key)

    def test_glow_is_baked_capped_and_cools_after_the_flash(self) -> None:
        def count(f: int, k: str) -> int:
            return sum(r.count(k) for r in self.grid["frames"][f]["cels"]["body"])

        self.assertLessEqual(count(3, "w"), 12)       # white only on a few pixels of the flash core
        self.assertEqual(count(5, "w"), 0)            # peak: no white, hottest value is the small G core
        self.assertEqual(count(10, "w") + count(10, "G") + count(10, "g"), 0)   # smoke fade: no hot values
        for f in (0, 3, 5, 10):
            body = self.grid["frames"][f]["cels"]["body"]
            filled = sum(ch != "." for r in body for ch in r)
            self.assertLess(filled / (48 * 48), 0.45, f"frame {f} covers too much of the cell")

    def test_every_frame_stays_inside_the_cell(self) -> None:
        for i, f in enumerate(self.grid["frames"]):
            rows = f["cels"]["body"]
            self.assertEqual((len(rows), {len(r) for r in rows}), (48, {48}), f"frame {i}")


if __name__ == "__main__":
    unittest.main()
