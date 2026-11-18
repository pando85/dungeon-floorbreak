"""ANIM-DONUT-02 Princess Donut melee and damage reactions (phase 1 keys).

The shared toolkit checks structure, ticks, palette, lint, contact rows and the frozen keys
(``python3 -m scripts.pixelart validate assets/source/characters/donut/donut_h40_melee.pixelart.json``).
These tests add the spec rules it does not cover: identity lock, frame contract with the locomotion atlas,
manifest tick windows and the art reach.
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_donut_melee as am  # noqa: E402

GRID = ROOT / "assets/source/characters/donut/donut_h40_melee.grid.json"
LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
MANIFEST = ROOT / "assets/manifests/donut_h40_melee.json"
CONFIG = ROOT / "assets/source/characters/donut/donut_h40_melee.pixelart.json"
FIXTURE = ROOT / "tests/fixtures/anim-donut-02_keys.json"
KEYS = [0, 1, 4, 5, 6, 8, 10, 11]


def pixels(rows):
    return {(x, y): c for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def flat(frame, layers):
    out = {}
    for layer in layers:
        out.update(pixels(frame["cels"].get(layer, [])))
    return out


def translated(a: dict, b: dict) -> bool:
    """True when pixel set ``a`` is ``b`` moved by one whole-pixel offset."""
    if len(a) != len(b) or not a:
        return False
    (ax, ay), (bx, by) = min(a), min(b)
    dx, dy = ax - bx, ay - by
    return all(a.get((x + dx, y + dy)) == c for (x, y), c in b.items())


class DonutMeleeKeysTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.loco = json.loads(LOCO.read_text(encoding="utf-8"))
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), am.dump(am.build()))

    def test_toolkit_validation_passes(self):
        res = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(CONFIG)],
                             cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_frame_and_pivot_match_the_locomotion_atlas(self):
        self.assertEqual(self.grid["canvas"], self.loco["canvas"])
        self.assertEqual(self.grid["pivot"], self.loco["pivot"])
        self.assertEqual(self.grid["palette"], self.loco["palette"])
        self.assertEqual(self.grid["layers"][:7], self.loco["layers"])

    def test_eight_keys_are_frozen(self):
        self.assertEqual(am.KEY_FRAMES, KEYS)
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertEqual(sorted(map(int, keys)), KEYS)

    def test_inbetweens_are_key_copies_until_executed(self):
        for i, (_name, _tag, _ticks, src) in enumerate(am.FRAMES):
            if src is not None:
                k = [n for n, f in enumerate(am.FRAMES) if f[0] == src][0]
                self.assertEqual(self.grid["frames"][i]["cels"], self.grid["frames"][k]["cels"], i)

    def test_identity_block_is_a_translation_of_a_locomotion_frame(self):
        """Head, face and gear (tiara, collar, charm) are locomotion pixels, moved as one block, never redrawn."""
        block = ["head", "face", "gear"]
        sources = [flat(f, block) for f in self.loco["frames"]]
        for k in KEYS:
            got = flat(self.grid["frames"][k], block)
            self.assertTrue(any(translated(got, s) for s in sources), f"key {k}: identity block was edited")

    def test_body_and_tail_are_translations_too(self):
        for k in KEYS:
            for layer in ("body", "tail"):
                got = pixels(self.grid["frames"][k]["cels"][layer])
                ok = any(translated(got, pixels(f["cels"][layer])) for f in self.loco["frames"] if layer in f["cels"])
                self.assertTrue(ok, f"key {k}: {layer} was edited")

    def test_airborne_keys_clear_the_floor_and_grounded_keys_touch_it(self):
        for k in KEYS:
            rows = [y for (_x, y) in flat(self.grid["frames"][k], self.grid["layers"])]
            if am.FRAMES[k][1] in ("pounce_air", "knockback"):
                self.assertLessEqual(max(rows), 41, k)
            else:
                self.assertEqual(max(rows), 43, k)

    def test_swat_arc_goes_from_high_and_near_to_low_and_far(self):
        cock = pixels(self.grid["frames"][0]["cels"]["paw"])
        hit = pixels(self.grid["frames"][1]["cels"]["paw"])
        self.assertLess(min(y for _x, y in cock), min(y for _x, y in hit))
        self.assertGreater(max(x for x, _y in hit), max(x for x, _y in cock))
        self.assertGreaterEqual(max(x for x, _y in hit) - am.PIVOT[0], 22)

    def test_pounce_reaches_forward(self):
        paw = pixels(self.grid["frames"][5]["cels"]["paw"])
        self.assertGreaterEqual(max(x for x, _y in paw) - am.PIVOT[0], 22)

    def test_manifest_tick_windows(self):
        melee = self.manifest["melee"]
        self.assertEqual(melee["tick_hz"], 60)
        for name, move in melee["moves"].items():
            spans = [f["ticks"] for f in move["frames"] if f["ticks"] is not None]
            for (_a0, a1), (b0, _b1) in zip(spans, spans[1:]):
                self.assertEqual(b0, a1 + 1, name)
            if spans:
                self.assertEqual(spans[0][0], 0, name)
            total = move.get("total_ticks") or move.get("total_ticks_nominal")
            if total:
                self.assertEqual(spans[-1][1] + 1, total, name)
        swat = melee["moves"]["swat"]
        self.assertEqual((swat["startup_ticks"], swat["active_ticks"], swat["total_ticks"]), (4, [4, 6], 14))
        self.assertEqual([r["index"] for r in melee["frames"]], list(range(len(am.FRAMES))))
        for row, frame in zip(melee["frames"], self.grid["frames"]):
            self.assertEqual(row["duration_ms"], frame["duration_ms"])


if __name__ == "__main__":
    unittest.main()
