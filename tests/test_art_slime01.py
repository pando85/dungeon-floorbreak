"""SLIME-01 Green Slime: asset-specific rules of docs/02-art/slime-01-spec.md that the shared pixel-art validators
(V01 to V15, run by ``python3 -m scripts.pixelart validate``) do not cover."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_slime01 as a  # noqa: E402

GRID = ROOT / "assets/source/characters/slime/slime_01.grid.json"
MANIFEST = ROOT / "assets/manifests/slime-01.json"
FIXTURE = ROOT / "tests/fixtures/slime-01_keys.json"
KEYS = (0, 3, 8, 10, 12, 15)


def count(rows, keys):
    return sum(c in keys for r in rows for c in r)


class SlimeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text(encoding="utf-8"))
        self.frames = [f["cels"]["body"] for f in self.grid["frames"]]

    def test_committed_grid_matches_the_authoring_script(self) -> None:
        self.assertEqual(self.frames, a.all_frames())
        self.assertEqual(len(self.frames), 16)
        self.assertEqual(self.grid["canvas"], [80, 40])
        self.assertEqual(self.grid["pivot"], [40, 39])

    def test_palette_is_within_budget(self) -> None:
        used = {c for f in self.frames for r in f for c in r} - {"."}
        self.assertLessEqual(len(self.grid["palette"]), 20)
        self.assertLessEqual(len(used), 20)
        self.assertTrue(used <= {p["key"] for p in self.grid["palette"]})

    def test_six_keys_are_distinct_and_frozen(self) -> None:
        self.assertEqual(len({tuple(self.frames[i]) for i in KEYS}), 6)
        self.assertEqual(sorted(a.KEYS), list(KEYS))
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertTrue(fixture, "the frozen-key fixture is empty")

    def test_every_frame_is_distinct_and_in_betweens_are_drawn(self) -> None:
        self.assertEqual(len({tuple(f) for f in self.frames}), 16)
        self.assertEqual(sorted(a.BETWEENS), [1, 2, 4, 5, 6, 7, 9, 11, 13, 14])
        self.assertEqual(sorted({*a.KEYS, *a.BETWEENS}), list(range(16)))
        m = json.loads(MANIFEST.read_text(encoding="utf-8"))["slime"]
        self.assertEqual([r["author"] for r in m["frames"]],
                         ["key" if i in a.KEYS else "in_between" for i in range(16)])

    def test_gel_pockets_are_rounded_not_crosses(self) -> None:
        for shape in a.POCKET_SHAPES.values():
            self.assertLessEqual(max(len(shape), len(shape[0])), 4)  # 2 to 4 px lenses
            cells = {(x, y) for y, row in enumerate(shape) for x, c in enumerate(row) if c == "#"}
            # no plus sign: no cell has all four 4-neighbours inside a 3-wide shape while its corners are empty
            for x, y in cells:
                if all((x + dx, y + dy) in cells for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                    self.assertTrue(any((x + dx, y + dy) in cells for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1))))

    def test_tick_table_is_the_spec_table(self) -> None:
        self.assertEqual(a.TICKS, [12, 8, 12, 8, 8, 8, 8, 6, 24, 2, 6, 20, 12, 6, 10, 0])
        self.assertGreaterEqual(a.TICKS[8], 18)  # the readable tell hold
        m = json.loads(MANIFEST.read_text(encoding="utf-8"))["slime"]
        self.assertEqual([r["ticks"] for r in m["frames"]], [t or None for t in a.TICKS])
        self.assertEqual(m["attack_ticks"], {"tell": [0, 29], "launch": [30, 31], "impact": [32, 37], "recover": [38, 57]})
        self.assertEqual(m["boxes"]["attack_hitbox"], [58, 25, 21, 15])  # active on frame 10 only
        self.assertEqual(a.TICKS[7] + a.TICKS[8], 30)  # tell 30 ticks, then launch 2, impact 6, recover 20

    def test_tell_is_the_tallest_pose_and_the_lunge_the_longest(self) -> None:
        box = {k: a.bbox(self.frames[k]) for k in KEYS}
        heights = {k: b[3] - b[1] + 1 for k, b in box.items()}
        widths = {k: b[2] - b[0] + 1 for k, b in box.items()}
        self.assertEqual(max(heights, key=heights.get), 8)
        self.assertEqual(heights[0], 30)
        self.assertEqual(max(widths, key=widths.get), 10)
        self.assertEqual(max(b[2] for b in box.values()), box[10][2])  # lunge reaches furthest right

    def test_corrosion_cue_and_inner_skull_live_in_every_alive_key(self) -> None:
        for k in (0, 3, 8, 10):
            self.assertGreaterEqual(count(self.frames[k], "v"), 4, k)
        self.assertGreaterEqual(count(self.frames[8], "v"), 9)  # acid beading is the tell
        for k in (0, 3, 8, 10, 12):
            self.assertGreaterEqual(count(self.frames[k], "q"), 10, k)  # the skull reads inside the gel

    def test_hurt_flashes_and_defeat_is_dull(self) -> None:
        self.assertGreaterEqual(count(self.frames[12], "w"), 8)
        for i, f in enumerate(self.frames):
            if i != 12:
                self.assertEqual(count(f, "w"), 0, i)
        for i in (13, 14, 15):
            self.assertEqual(count(self.frames[i], "vwH"), 0, i)  # no acid, flash or glint once defeated

    def test_no_split_off_pieces(self) -> None:
        # one body per frame: the grounded pixels of every frame form a single 8-connected component
        for i in KEYS:
            rows = self.frames[i]
            pts = {(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}
            seen, todo = set(), [min(pts)]
            while todo:
                p = todo.pop()
                if p in seen:
                    continue
                seen.add(p)
                todo += [(p[0] + dx, p[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (p[0] + dx, p[1] + dy) in pts]
            self.assertEqual(seen, pts, f"frame {i} has a detached piece")

    def test_every_frame_stays_on_the_canvas_and_grounded(self) -> None:
        for i, f in enumerate(self.frames):
            self.assertTrue(any(c != "." for c in f[39]), f"frame {i} has nothing on the contact row")
            self.assertTrue(len(f) == 40 and all(len(r) == 80 for r in f))


if __name__ == "__main__":
    unittest.main()
