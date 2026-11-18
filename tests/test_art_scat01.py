"""SCAT-01 Scatterer: asset-specific rules of docs/02-art/scat-01-spec.md that the shared pixel-art validators
(V01 to V15, run by ``python3 -m scripts.pixelart validate``) do not cover."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_scat01 as a  # noqa: E402

GRID = ROOT / "assets/source/characters/scatterer/scat-01.grid.json"
MANIFEST = ROOT / "assets/manifests/scat-01.json"
FIXED = (3, 8, 10, 12, 15)


def pix(rows, keys):
    return [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c in keys]


class ScattererTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text(encoding="utf-8"))
        self.frames = [f["cels"]["body"] for f in self.grid["frames"]]

    def test_committed_grid_matches_the_authoring_script(self) -> None:
        self.assertEqual(self.frames, a.all_frames())
        self.assertEqual(len(self.frames), 16)

    def test_palette_and_frame_zero_hero_key(self) -> None:
        used = {c for f in self.frames for r in f for c in r} - {"."}
        self.assertLessEqual(len(used), 16)
        self.assertEqual(self.frames[0], a.frame_idle0())
        self.assertEqual(len(pix(self.frames[0], "r")), 2)

    def test_tick_table_is_the_spec_table(self) -> None:
        self.assertEqual(a.TICKS, [10, 6, 10, 4, 4, 4, 4, 6, 20, 2, 5, 14, 10, 4, 8, 0])
        self.assertGreaterEqual(a.TICKS[8], 18)  # the readable tell window
        rows = json.loads(MANIFEST.read_text(encoding="utf-8"))["scatterer"]["frames"]
        self.assertEqual([r["ticks"] for r in rows], [t or None for t in a.TICKS])
        boxes = json.loads(MANIFEST.read_text(encoding="utf-8"))["scatterer"]["boxes"]
        self.assertEqual(boxes["attack_hitbox"], [40, 25, 8, 6])  # active on frame 10 only

    def test_bite_reaches_the_hitbox_only_on_frame_10(self) -> None:
        reach = [max([x for x, _y in pix(f, "H")] or [0]) for f in self.frames]  # mandible glints
        self.assertEqual([i for i, r in enumerate(reach) if r >= 45], [10])
        self.assertEqual(len([p for p in pix(self.frames[10], "H") if p[0] >= 45]), 3)  # closed, pinching mandibles

    def test_defeat_has_no_explosion_cue(self) -> None:
        for i in (13, 14, 15):
            self.assertEqual(pix(self.frames[i], "w"), [], i)  # no flash
            self.assertLessEqual(len(pix(self.frames[i], "H")), 1, i)  # no glint or burst
        for i in (14, 15):
            self.assertEqual(pix(self.frames[i], "r"), [], i)  # no glowing eye once settled
        used = {c for i in (13, 14, 15) for r in self.frames[i] for c in r}
        self.assertFalse(used & set("w"), "no yellow or glow keys in the defeat frames")

    def test_every_frame_stays_on_the_canvas_and_grounded(self) -> None:
        for i, f in enumerate(self.frames):
            self.assertTrue(any(c != "." for c in f[31]), f"frame {i} has nothing on the contact row")
            self.assertTrue(all(c == "." for c in f[0]) and len(f) == 32 and all(len(r) == 48 for r in f))

    def test_keys_are_distinct_poses(self) -> None:
        self.assertEqual(len({tuple(self.frames[i]) for i in (0,) + FIXED}), 6)


if __name__ == "__main__":
    unittest.main()
