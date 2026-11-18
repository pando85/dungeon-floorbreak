"""ROT-01 Rot Sticker: asset-specific rules of docs/02-art/rot-01-spec.md that the shared pixel-art validators
(V01 to V15, run by ``python3 -m scripts.pixelart validate``) do not cover."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_rot01 as a  # noqa: E402

GRID = ROOT / "assets/source/characters/rot_sticker/rot_01.grid.json"
MANIFEST = ROOT / "assets/manifests/rot-01.json"
FIXTURE = ROOT / "tests/fixtures/rot-01_keys.json"


def pix(rows, keys):
    return [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c in keys]


class RotStickerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text(encoding="utf-8"))
        self.frames = [f["cels"]["body"] for f in self.grid["frames"]]

    def test_committed_grid_matches_the_authoring_script(self) -> None:
        self.assertEqual(self.frames, a.all_frames())
        self.assertEqual(len(self.frames), 15)

    def test_keys_are_frozen_and_distinct(self) -> None:
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertEqual(sorted(int(k) for k in keys), [0, 3, 8, 10, 11, 14])
        self.assertEqual(len({tuple(self.frames[i]) for i in a.KEY_FRAMES}), 6)

    def test_size_frame_and_wall_contact(self) -> None:
        self.assertEqual((a.W, a.H, a.PIVOT), (32, 32, [6, 16]))
        ys = [y for y, r in enumerate(self.frames[0]) if r.strip(".")]
        self.assertEqual(ys[-1] - ys[0] + 1, 20)  # idle opaque height along the wall
        for i, f in enumerate(self.frames):
            self.assertTrue(all(r[: a.WALL_X] == "." * a.WALL_X for r in f), f"frame {i} draws left of the wall column")
            self.assertTrue(any(r[a.WALL_X] != "." for r in f), f"frame {i} does not touch the wall column")

    def test_frames_leave_the_square_edges_clear_for_rotation(self) -> None:
        for i, f in enumerate(self.frames):
            self.assertTrue(all(c == "." for c in f[0]) and all(c == "." for c in f[-1]), i)

    def test_palette_budget(self) -> None:
        used = {c for f in self.frames for r in f for c in r} - {"."}
        self.assertLessEqual(len(used), 20)

    def test_tell_is_the_glow_and_idle_is_dark(self) -> None:
        self.assertEqual(pix(self.frames[0], "G"), [])
        self.assertGreaterEqual(len(pix(self.frames[8], "G")), 4)
        self.assertGreaterEqual(len(pix(self.frames[8], "o")), 20)
        self.assertGreater(len(pix(self.frames[10], "G")), len(pix(self.frames[8], "G")))

    def test_dud_defeat_has_no_explosion_cue(self) -> None:
        for i in (12, 13, 14):
            self.assertEqual(pix(self.frames[i], "wGgo"), [], i)

    def test_tick_table_and_manifest(self) -> None:
        self.assertEqual(a.TICKS, [14, 12, 14, 8, 8, 8, 8, 6, 24, 2, 4, 10, 4, 8, 0])
        self.assertGreaterEqual(a.TICKS[7] + a.TICKS[8], 30)  # tell window, 500 ms
        m = json.loads(MANIFEST.read_text(encoding="utf-8"))["rot_sticker"]
        self.assertEqual([r["ticks"] for r in m["frames"]], [t or None for t in a.TICKS])
        self.assertEqual(m["boxes"]["blast_radius_px"], 40)
        self.assertEqual(m["key_frames"], [0, 3, 8, 10, 11, 14])


if __name__ == "__main__":
    unittest.main()
