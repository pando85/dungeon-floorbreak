"""ANIM-DONUT-01 candidate: structural and identity checks on the editable grid.

These pin what the art must keep (approved idle, master face and head pixels, palette,
pivot and contact convention, clean clusters). They do not judge artistic quality.
"""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import pixel_grid_to_ase as pg  # noqa: E402

MASTER = ROOT / "assets/source/characters/donut/donut_h40.grid.json"
CANDIDATE = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
MANIFEST = ROOT / "assets/manifests/donut_h40_locomotion.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pixels(rows: list[str]) -> set[tuple[int, int, str]]:
    return {(x, y, c) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def normalised(px: set[tuple[int, int, str]]) -> set[tuple[int, int, str]]:
    x0 = min(x for x, _, _ in px)
    y0 = min(y for _, y, _ in px)
    return {(x - x0, y - y0, c) for x, y, c in px}


class DonutLocomotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.master = load(MASTER)
        cls.grid = load(CANDIDATE)
        cls.manifest = load(MANIFEST)
        pg.validate_grid(cls.grid)
        cls.tags = {t["name"]: (t["from"], t["to"]) for t in cls.grid["tags"]}

    def frames(self, tag: str) -> list[int]:
        a, b = self.tags[tag]
        return list(range(a, b + 1))

    def test_manifest_matches_grid(self) -> None:
        self.assertEqual(self.manifest["frame_size"], self.grid["canvas"])
        self.assertEqual(self.manifest["pivot"], self.grid["pivot"])
        self.assertEqual(self.manifest["pivot"], [46, 43])
        self.assertEqual(set(self.manifest["animation_tags"]), set(self.tags))
        self.assertEqual(self.manifest["review_status"], "prototype_fixture_unapproved")
        self.assertEqual([len(self.frames(t)) for t in
                          ("idle", "run", "jump_start", "jump_rise", "jump_fall", "land")],
                         [4, 6, 2, 1, 1, 2])

    def test_palette_is_the_master_palette(self) -> None:
        self.assertEqual(self.grid["palette"], self.master["palette"])

    def test_idle_is_the_approved_master_idle(self) -> None:
        dx = self.grid["pivot"][0] - self.master["pivot"][0]
        self.assertEqual(self.grid["pivot"][1], self.master["pivot"][1])
        for i in range(4):
            mine = pg.composite_keys(self.grid, i)
            ref = pg.composite_keys(self.master, i)
            for y, row in enumerate(ref):
                self.assertEqual("".join(mine[y][dx:dx + len(row)]), "".join(row), f"idle {i} row {y}")
                self.assertTrue(all(c == "." for c in mine[y][:dx] + mine[y][dx + len(row):]))
            self.assertEqual(self.grid["frames"][i]["duration_ms"], self.master["frames"][i]["duration_ms"])

    def test_clean_clusters(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(report["orphan_count_per_frame"], [0] * len(self.grid["frames"]))
        self.assertEqual(report["isolated_count_per_frame"], [0] * len(self.grid["frames"]))

    def test_contact_rows(self) -> None:
        report = pg.lint_grid(self.grid)
        bottom = {f["frame"]: f["bottom_row"] for f in report["frames"]}
        py = self.grid["pivot"][1]
        run = self.frames("run")
        grounded = (self.frames("idle") + run[:4] + self.frames("jump_start") + self.frames("land")
                    + self.frames("jump_rise") + self.frames("jump_fall"))
        for i in grounded:
            self.assertEqual(bottom[i], py, f"frame {i} must touch the contact row")
        for i in run[4:]:  # gallop suspension: daylight under every paw
            self.assertLess(bottom[i], py, f"run frame {i} is the airborne phase")

    def test_face_and_head_are_master_pixels(self) -> None:
        def master_layer(layer: str, frame: int, rows: slice) -> set:
            return normalised(pixels(self.master["frames"][frame]["cels"][layer][rows]))

        head_ref = master_layer("head", 0, slice(4, 28))
        face_open = master_layer("face", 0, slice(0, 44))
        face_blink = master_layer("face", 2, slice(0, 44))
        blink_frame = self.frames("land")[0]
        for i in range(self.tags["run"][0], len(self.grid["frames"])):
            cels = self.grid["frames"][i]["cels"]
            face = normalised(pixels(cels["face"]))
            self.assertEqual(face, face_blink if i == blink_frame else face_open, f"face frame {i}")
            if i != blink_frame:
                self.assertEqual(normalised(pixels(cels["head"])), head_ref, f"head frame {i}")

    def test_face_stays_put_between_idle_and_run(self) -> None:
        def face_x(i: int) -> int:
            return min(x for x, _, _ in pixels(self.grid["frames"][i]["cels"]["face"]))
        self.assertEqual(face_x(0), face_x(self.frames("run")[0]))


if __name__ == "__main__":
    unittest.main()
