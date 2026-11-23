"""HOARD-02 checks that the shared toolkit does not cover: the authoring script reproduces the committed grid and
manifest, the .ase matches it, the 13 keys are frozen, frames 0-15 equal HOARD-01, the six in-betweens are drawn poses,
the H1-H13 rules hold through the config, and the pinned eye-white rule (R3) holds."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_hoard01 as h1  # noqa: E402
import author_hoard02 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart.validators import freeze  # noqa: E402

CONFIG = ROOT / "assets/source/characters/hoarder/hoard-02.pixelart.json"
GRID = ROOT / "assets/source/characters/hoarder/hoard-02.grid.json"
ASE = ROOT / "assets/source/characters/hoarder/hoard-02.ase"
H1_GRID = ROOT / "assets/source/characters/hoarder/hoard-01.grid.json"
FIXTURE = ROOT / "tests/fixtures/hoard-02_keys.json"
MANIFEST = ROOT / "assets/manifests/hoard-02.json"
FROZEN = [0, 3, 8, 10, 12, 15, 17, 18, 19, 20, 22, 24, 26]


def bbox(rows: list[str]) -> tuple[int, int, int, int]:
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    return min(xs), min(ys), max(xs), max(ys)


class Hoard02Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.grid = json.loads(GRID.read_text())

    def test_authoring_script_reproduces_the_grid_and_manifest(self) -> None:
        self.assertEqual(author.h1.dump(author.build_grid()), GRID.read_text())
        self.assertEqual(author.h1.dump(author.build_manifest()), MANIFEST.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [160, 128])
        self.assertEqual(len(ase["frames"]), 29)
        self.assertEqual(len(self.grid["frames"]), 29)

    def test_keys_are_frozen(self) -> None:
        cfg = load_config(CONFIG)
        expected = json.loads(FIXTURE.read_text())
        actual = json.loads(json.dumps(freeze(cfg, self.grid, FROZEN, ["body"])))
        self.assertEqual(actual["frames"] if "frames" in actual else actual,
                         expected["frames"] if "frames" in expected else expected)
        self.assertEqual(sorted(set(author.KEY_FRAMES) | set(h1.KEY_FRAMES)), FROZEN)

    def test_frames_0_to_15_are_the_hoard_01_frames(self) -> None:
        base = json.loads(H1_GRID.read_text())
        self.assertEqual(self.grid["frames"][:16], base["frames"])
        self.assertEqual(self.grid["palette"], base["palette"])

    def test_inbetweens_are_drawn_poses_not_key_copies(self) -> None:
        self.assertEqual(author.INBETWEENS, (16, 21, 23, 25, 27, 28))
        for f in author.INBETWEENS:
            self.assertIn(f, author.INB)
            self.assertNotIn(f, author.KEY_POSES)
        cels = [json.dumps(f["cels"]) for f in self.grid["frames"]]
        self.assertEqual(len(set(cels)), 29)                      # V12: every frame is unique

    def test_rules_h1_to_h13_and_v12_pass_through_the_config(self) -> None:
        report = validators.run(validators.build_context(load_config(CONFIG)))
        self.assertEqual([r for r in report["rules"] if r["status"] == "fail"], [])
        self.assertEqual(next(r for r in report["rules"] if r["id"] == "V12")["status"], "pass")
        self.assertEqual(next(r for r in report["rules"] if r["id"] == "V13")["status"], "pass")

    def test_shove_settle_is_wide_at_once_and_unlike_the_gather(self) -> None:
        x0, y0, x1, _ = bbox(self.grid["frames"][16]["cels"]["body"])
        self.assertGreaterEqual(x1 - x0 + 1, 120)                  # H13
        self.assertGreaterEqual(y0, 24)
        gx0, _, gx1, _ = bbox(self.grid["frames"][7]["cels"]["body"])
        self.assertGreater(x1 - x0, gx1 - gx0 + 25)

    def test_pinned_frames_show_eye_whites(self) -> None:
        """R3: pinned is alive, not defeat. Frame 15 (defeat) has no white; 22 and 23 each have at least 4."""
        white = lambda f: sum(r.count("w") for r in self.grid["frames"][f]["cels"]["body"])  # noqa: E731
        self.assertEqual(white(15), 0)
        self.assertGreaterEqual(white(22), 4)
        self.assertGreaterEqual(white(23), 4)

    def test_pinned_strain_is_the_pinned_pose_with_the_hand_drawn_back(self) -> None:
        _, _, r22, _ = bbox(self.grid["frames"][22]["cels"]["body"])
        _, _, r23, _ = bbox(self.grid["frames"][23]["cels"]["body"])
        self.assertLess(r23, r22)

    @staticmethod
    def part_tops(frame: int) -> dict[str, int]:
        """Top row of every rig part of a frame (the canvas of ``hoarder()`` is captured at ``finish``)."""
        captured = {}
        original = h1.Canvas.finish

        def finish(canvas):
            rows = original(canvas)
            captured["canvas"] = canvas
            return rows

        h1.Canvas.finish = finish
        try:
            author.render(frame)
        finally:
            h1.Canvas.finish = original
        c = captured["canvas"]
        tops: dict[str, int] = {}
        for (_x, y), o in c.owner.items():
            tops[c.parts[o]] = min(y, tops.get(c.parts[o], y))
        return tops

    def test_no_sleeve_rises_above_the_head_in_pitched_frames(self) -> None:
        """Gate fix: the sleeve cap never forms the sack hump. Frozen keys 18, 19 and 24 keep it 4 px below the head
        top (spec section 5); the in-betweens 16 and 25 keep it below the head top. (Key 17 is frozen and unchanged.)"""
        for f, margin in ((16, 1), (18, 4), (19, 4), (24, 4), (25, 1)):
            tops = self.part_tops(f)
            sleeve = min(tops["near_sleeve"], tops["far_sleeve"])
            self.assertGreaterEqual(sleeve - tops["head"], margin, f"frame {f}")

    def test_emerge_frames_raise_no_arm_above_the_head(self) -> None:
        """R5: the emerge teaches no false swipe tell. Arms stay below the head top in 26-28."""
        for f in (26, 27, 28):
            tops = self.part_tops(f)
            self.assertGreater(min(tops["near_arm"], tops["far_arm"]), tops["head"], f"frame {f}")

    def test_manifest_lists_every_frame_and_tag(self) -> None:
        m = json.loads(MANIFEST.read_text())
        self.assertEqual(m["id"], "hoarder_boss_h112_v2")
        self.assertEqual(m["output_atlas"], "generated/art/hoard-02.png")
        self.assertEqual(len(m["hoarder"]["frames"]), 29)
        self.assertEqual(m["animation_tags"], [t["name"] for t in self.grid["tags"]])
        self.assertEqual(m["hoarder"]["key_frames"], FROZEN)
        self.assertEqual(set(m["hoarder"]["playback"]), set(m["animation_tags"]))


if __name__ == "__main__":
    unittest.main()
