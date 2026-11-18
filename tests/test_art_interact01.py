"""ANIM-INTERACT-01 (design phase): Carl and Donut interact keys, docs/02-art/interact-anim-01-spec.md.

The shared toolkit validators run with ``python3 -m scripts.pixelart validate <config>`` on
``assets/source/characters/{carl/carl_h52_interact,donut/donut_h40_interact}.pixelart.json``; this file covers the
spec rules the toolkit does not (frame and pivot parity with locomotion, contact ticks, identity, contact boxes).
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_interact01 as ai  # noqa: E402

CARL_LOCO = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
DONUT_LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
CONFIGS = [ROOT / "assets/source/characters/carl/carl_h52_interact.pixelart.json",
           ROOT / "assets/source/characters/donut/donut_h40_interact.pixelart.json"]


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def layer_pix(frame: dict, layer: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(layer, [])) for x, c in enumerate(r) if c != "."}


def flat(grid: dict, i: int) -> dict:
    out = {}
    for l in grid["layers"]:
        out.update(layer_pix(grid["frames"][i], l))
    return out


def translation_of(a: dict, b: dict):
    """(dx, dy) if pixel set ``a`` is ``b`` translated exactly, else None."""
    if not a or len(a) != len(b):
        return None
    ax, ay = min(a)
    bx, by = min(b)
    dx, dy = ax - bx, ay - by
    return (dx, dy) if all(a.get((x + dx, y + dy)) == c for (x, y), c in b.items()) else None


class InteractGridTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.carl, cls.donut = load(ai.CARL_OUT), load(ai.DONUT_OUT)
        cls.carl_loco, cls.donut_loco = load(CARL_LOCO), load(DONUT_LOCO)

    def test_committed_grids_match_the_generator(self):
        for path, text in ai.outputs().items():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)

    def test_same_frame_pivot_palette_and_layers_as_locomotion(self):
        for g, loco in ((self.carl, self.carl_loco), (self.donut, self.donut_loco)):
            for k in ("canvas", "pivot", "palette", "layers"):
                self.assertEqual(g[k], loco[k], k)
        self.assertEqual(self.carl["canvas"], [48, 56])
        self.assertEqual(self.carl["pivot"], [25, 55])
        self.assertEqual(self.donut["canvas"], [72, 44])
        self.assertEqual(self.donut["pivot"], [46, 43])

    def test_tags_frames_and_tick_durations(self):
        for g, plan in ((self.carl, ai.CARL_PLAN), (self.donut, ai.DONUT_PLAN)):
            self.assertEqual([t["name"] for t in g["tags"]], [p[0] for p in plan])
            i = 0
            for tag, (name, rows, _) in zip(g["tags"], plan):
                self.assertEqual((tag["from"], tag["to"]), (i, i + len(rows) - 1), name)
                for _, ticks, _ in rows:
                    self.assertEqual(g["frames"][i]["duration_ms"], round(ticks * 1000 / 60))
                    i += 1
            self.assertEqual(len(g["frames"]), i)

    def test_contact_ticks(self):
        self.assertEqual(ai.contact_ticks(ai.CARL_PLAN),
                         {"interact_hand": 5, "interact_floor": 8, "interact_push": 6})
        self.assertEqual(ai.contact_ticks(ai.DONUT_PLAN), {"interact_paw": 6, "interact_nudge": 8})

    def test_config_ticks_match_the_grid(self):
        for cfg_path, g in zip(CONFIGS, (self.carl, self.donut)):
            cfg = load(cfg_path)
            for tag in g["tags"]:
                rows = cfg["tags"][tag["name"]]["frames"]
                self.assertEqual([r[0] for r in rows], list(range(tag["from"], tag["to"] + 1)))
                for idx, ticks in rows:
                    self.assertEqual(g["frames"][idx]["duration_ms"], round(ticks * 1000 / 60))

    def test_grounded_and_clipping(self):
        for g in (self.carl, self.donut):
            w, h = g["canvas"]
            for i in range(len(g["frames"])):
                pix = flat(g, i)
                self.assertEqual(max(y for _, y in pix), g["pivot"][1], f"frame {i} lowest row")
                self.assertFalse([p for p in pix if p[0] in (0, w - 1) or p[1] == 0], f"frame {i} touches the edge")

    def test_inbetweens_are_key_copies_in_the_design_phase(self):
        """Design phase only: the execution phase replaces this test with the drawn in-betweens."""
        for g, plan, keys in ((self.carl, ai.CARL_PLAN, {1, 2, 5, 9}), (self.donut, ai.DONUT_PLAN, {1, 5})):
            names = [n for _, rows, _ in plan for n, _, _ in rows]
            for i, (_, rows, _) in enumerate([(None, r, None) for _, rs, _ in plan for r in rs]):
                name, _, src = rows
                if src is None:
                    self.assertIn(i, keys)
                else:
                    self.assertEqual(g["frames"][i]["cels"], g["frames"][names.index(src)]["cels"], name)

    def test_carl_identity_parts_are_translated_master_pixels(self):
        idle = self.carl_loco["frames"][0]
        for i in (1, 2, 5, 9):
            fr = self.carl["frames"][i]
            for layer in ("jacket", "torso", "pin", "hair"):
                self.assertIsNotNone(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), (i, layer))

    def test_donut_identity_parts_are_translated_master_pixels(self):
        idle = self.donut_loco["frames"][0]
        for i in (1, 5):
            fr = self.donut["frames"][i]
            for layer in ("tail", "head", "face"):
                self.assertIsNotNone(translation_of(layer_pix(fr, layer), layer_pix(idle, layer)), (i, layer))

    def test_contact_boxes(self):
        """Spec section 4: the contact part reaches its object-height class (pivot-relative, y up)."""
        px, py = self.carl["pivot"]

        def reach(g, i, layer):
            pts = layer_pix(g["frames"][i], layer)
            right = max(x for x, _ in pts)
            ys = [py - y for x, y in pts if x >= right - 4]
            return right - px, min(ys), max(ys)

        seat = layer_pix(self.carl["frames"][5], "boxers")
        self.assertLessEqual(max(y for _, y in seat), py - 4)   # p1 seat clear of the floor (not sitting)

        r, lo, hi = reach(self.carl, 1, "far_arm")      # u1 grip at hand height
        self.assertTrue(16 <= r <= 18 and lo >= 24 and hi <= 33, (r, lo, hi))
        r, lo, hi = reach(self.carl, 5, "far_arm")      # p1 hand on a floor item
        self.assertTrue(r >= 18 and lo <= 6, (r, lo, hi))
        r, lo, hi = reach(self.carl, 9, "near_arm")     # s1 fists on the prop face
        self.assertTrue(r >= 19 and 16 <= lo <= 26, (r, lo, hi))
        r2, lo2, _ = reach(self.carl, 9, "far_arm")     # s1: the second fist sits higher (two contact heights)
        self.assertGreaterEqual(lo2 - lo, 4)
        dpx, dpy = self.donut["pivot"]
        paw = layer_pix(self.donut["frames"][1], "legs_near")
        self.assertGreaterEqual(max(x for x, _ in paw) - dpx, 20)
        self.assertLessEqual(dpy - max(y for _, y in paw), 6)

    def test_toolkit_validators_pass(self):
        for cfg in CONFIGS:
            out = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(cfg)], cwd=ROOT,
                                 capture_output=True, text=True)
            self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
            self.assertIn("verdict: PASS", out.stdout)


if __name__ == "__main__":
    unittest.main()
