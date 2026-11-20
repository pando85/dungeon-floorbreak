"""ART-DONUT-CF02: Donut's CF02 acting sheet (docs/02-art/cf02-acting-spec.md, section 4 and section 9).

The shared toolkit validators run with ``python3 -m scripts.pixelart validate
assets/source/characters/donut/donut_h40_cf02.pixelart.json``; this file covers the spec rules the toolkit does not
(keys copied pixel for pixel, the derived frame drawn from the named sources, identity, budget, manifest block).
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_donut_cf02 as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

KEYS = ROOT / "assets/source/characters/donut/cf02_keys/donut_h40_cf02_keys.grid.json"
LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
CONFIG = ROOT / "assets/source/characters/donut/donut_h40_cf02.pixelart.json"
MANIFEST = ROOT / "assets/manifests/donut_h40_cf02.json"
PUFF, SETTLE, SMUG = 0, 1, 2


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def layer_pix(frame: dict, layer: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(layer, [])) for x, c in enumerate(r) if c != "."}


def flat(frame: dict, layers: list[str]) -> dict:
    out = {}
    for layer in layers:
        out.update(layer_pix(frame, layer))
    return out


def shifted(p: dict, dx: int, dy: int) -> dict:
    return {(x + dx, y + dy): c for (x, y), c in p.items()}


def box(p: dict):
    xs, ys = [x for x, _ in p], [y for _, y in p]
    return min(xs), min(ys), max(xs), max(ys)


class DonutCf02Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.g = load(ad.OUT)
        cls.keys = load(KEYS)
        cls.idle = load(LOCO)["frames"][0]
        cls.layers = cls.g["layers"]
        cls.cfg = load(CONFIG)

    def test_committed_grid_matches_the_generator(self):
        for path, text in ad.outputs().items():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)

    def test_frame_pivot_palette_layers_match_locomotion_and_keys(self):
        loco = load(LOCO)
        for k in ("canvas", "pivot", "palette", "layers"):
            self.assertEqual(self.g[k], loco[k], k)
            self.assertEqual(self.g[k], self.keys[k], k)
        self.assertEqual(self.g["canvas"], [72, 44])
        self.assertEqual(self.g["pivot"], [46, 43])

    def test_tags_frames_and_tick_durations(self):
        self.assertEqual([(t["name"], t["from"], t["to"]) for t in self.g["tags"]],
                         [("startle", 0, 1), ("smug", 2, 2)])
        self.assertEqual([f["duration_ms"] for f in self.g["frames"]],
                         [round(6 * 1000 / 60), round(6 * 1000 / 60), round(20 * 1000 / 60)])

    def test_budget_is_three_of_four(self):
        self.assertEqual(len(self.g["frames"]), 3)
        self.assertLessEqual(len(self.g["frames"]), 4)
        man = load(MANIFEST)
        self.assertEqual(man["cf02"]["frame_budget"], {"q73_cap": 4, "used": 3, "spare": 1,
                                                       "note": man["cf02"]["frame_budget"]["note"]})

    def test_keys_are_copied_pixel_for_pixel(self):
        for sheet, key in ((PUFF, 0), (SMUG, 1)):
            self.assertEqual(self.g["frames"][sheet]["cels"], self.keys["frames"][key]["cels"], sheet)

    def test_grounded_on_the_pivot_row_and_clipping(self):
        w, _ = self.g["canvas"]
        for i, fr in enumerate(self.g["frames"]):
            pix = flat(fr, self.layers)
            self.assertEqual(max(y for _, y in pix), 43, f"frame {i} paws on row 43")
            self.assertFalse([p for p in pix if p[0] in (0, w - 1)], f"frame {i} touches a side column")
            self.assertGreaterEqual(min(y for _, y in pix), 1, f"frame {i}: the tiara never touches row 0")

    def test_settle_is_derived_by_translation_from_the_named_sources(self):
        s = self.g["frames"][SETTLE]
        puff = self.g["frames"][PUFF]
        idle = self.idle
        # head: the puff head 2 px down and 1 px toward the event side (and so the idle head at (+1, -1))
        self.assertEqual(layer_pix(s, "head"), shifted(layer_pix(puff, "head"), 1, 2))
        self.assertEqual(layer_pix(s, "head"), shifted(layer_pix(idle, "head"), 1, -1))
        # face: pupils back to the idle size (the idle face cel at the head's place)
        self.assertEqual(layer_pix(s, "face"), shifted(layer_pix(idle, "face"), 1, -1))
        # tail: still puffed (same pixels as the puff, fur rim kept), 2 px lower
        self.assertEqual(layer_pix(s, "tail"), shifted(layer_pix(puff, "tail"), 0, 2))
        # tiara moves with the head, collar and charm with the body
        pg_gear = layer_pix(puff, "gear")
        tiara = {p: c for p, c in pg_gear.items() if p[1] <= ad.TIARA_MAX_ROW}
        collar = {p: c for p, c in pg_gear.items() if p[1] > ad.TIARA_MAX_ROW}
        self.assertEqual(layer_pix(s, "gear"), {**shifted(tiara, 1, 2), **shifted(collar, 0, 2)})

    def test_settle_body_is_the_puff_body_lowered_with_two_shaft_rows_removed(self):
        s_body, p_body = layer_pix(self.g["frames"][SETTLE], "body"), layer_pix(self.g["frames"][PUFF], "body")
        closure = {p: c for p, c in s_body.items() if p in ad.OUTLINE_CLOSURE}
        self.assertEqual(len(closure), len(ad.OUTLINE_CLOSURE))
        rest = {p: c for p, c in s_body.items() if p not in ad.OUTLINE_CLOSURE}
        expected = {}
        for (x, y), c in p_body.items():
            if y < 40:
                expected[(x, y + 2)] = c
            elif y > 41:
                expected[(x, y)] = c
        self.assertEqual(rest, expected)
        # the legs are the idle shaft plus one row: the feet are the approved idle feet
        idle_body = layer_pix(self.idle, "body")
        for y in (42, 43):
            self.assertEqual({x: c for (x, yy), c in s_body.items() if yy == y},
                             {x: c for (x, yy), c in idle_body.items() if yy == y})

    def test_startle_puff_signature_and_settle_pupils(self):
        idle_face, puff, settle = (layer_pix(self.idle, "face"), layer_pix(self.g["frames"][PUFF], "face"),
                                   layer_pix(self.g["frames"][SETTLE], "face"))
        self.assertNotEqual(shifted(idle_face, 0, -3), puff, "the puff has slit pupils, not the idle ones")
        self.assertEqual(len(idle_face), len(settle))
        # body is 3 px above idle in the puff (the arch), 1 px above in the settle
        idle_top = box(layer_pix(self.idle, "head"))[1]
        self.assertEqual(box(layer_pix(self.g["frames"][PUFF], "head"))[1], idle_top - 3)
        self.assertEqual(box(layer_pix(self.g["frames"][SETTLE], "head"))[1], idle_top - 1)
        # the tail is lifted: the puff tail sits higher than the settle tail, which sits higher than idle's
        tops = [box(layer_pix(fr, "tail"))[1] for fr in (self.g["frames"][PUFF], self.g["frames"][SETTLE])]
        self.assertLess(tops[0], tops[1])
        self.assertLess(tops[1], box(layer_pix(self.idle, "tail"))[1])

    def test_identity_head_and_costume_are_approved_pixels(self):
        """No new head, face or costume: tiara, collar and the head layer are translated idle pixels in every frame."""
        for i, fr in enumerate(self.g["frames"]):
            head = layer_pix(fr, "head")
            idle_head = layer_pix(self.idle, "head")
            self.assertEqual(len(head), len(idle_head), i)
            (x0, y0), (ix0, iy0) = min(head, key=lambda p: (p[1], p[0])), min(idle_head, key=lambda p: (p[1], p[0]))
            moved = shifted(idle_head, x0 - ix0, y0 - iy0)
            diff = [p for p in set(head) | set(moved) if head.get(p) != moved.get(p)]
            # the frozen smug key repaints one cheek outline pixel (K to O) next to the eye; nothing else differs
            self.assertLessEqual(len(diff), 1 if i == SMUG else 0, f"frame {i}: head is the idle head translated: {diff}")
        # a cute face: no snarl colours were introduced (the palette is the approved Donut palette)
        used = {c for fr in self.g["frames"] for layer in self.layers for row in fr["cels"].get(layer, []) for c in row}
        self.assertLessEqual(used - {"."}, {p["key"] for p in load(LOCO)["palette"]})

    def test_smug_matches_its_spec_row(self):
        smug = self.g["frames"][SMUG]
        idle_face = layer_pix(self.idle, "face")
        self.assertEqual(len(layer_pix(smug, "face")), len(idle_face))
        self.assertEqual(box(layer_pix(smug, "head"))[1], box(layer_pix(self.idle, "head"))[1] - 1, "chin up: 1 px higher")
        self.assertEqual(box(layer_pix(smug, "tail"))[1], box(layer_pix(self.idle, "tail"))[1] - 2, "tail 2 px higher")

    def test_manifest_block(self):
        man = load(MANIFEST)
        cf = man["cf02"]
        self.assertEqual(man["frame_size"], [72, 44])
        self.assertEqual(man["pivot"], [46, 43])
        self.assertEqual(man["animation_tags"], ["startle", "smug"])
        self.assertEqual(cf["moves"]["startle"]["total_ticks"], 12)
        self.assertEqual([f["ticks"] for f in cf["moves"]["startle"]["frames"]], [[0, 5], [6, 11]])
        self.assertEqual(cf["moves"]["smug"]["total_ticks"], 20)
        self.assertEqual([f["ticks"] for f in cf["moves"]["smug"]["frames"]], [[0, 19]])
        self.assertEqual((cf["moves"]["startle"]["cooldown_ticks"], cf["moves"]["smug"]["cooldown_ticks"]), (180, 300))
        self.assertTrue(cf["moves"]["startle"]["idle_only"] and cf["moves"]["smug"]["idle_only"])
        self.assertEqual(cf["keys"], {"startle_puff": 0, "smug": 2})
        self.assertEqual(cf["derived"], {"startle_settle": 1})
        self.assertIn("independent review: pending (Claude Opus)", man["provenance"]["generator"])
        self.assertEqual(man["review_status"], "prototype_fixture_unapproved")

    def test_lint_is_strict_clean(self):
        self.assertEqual(pg.lint_problems(pg.lint_grid(self.g)), [])

    def test_toolkit_validators_pass(self):
        res = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(CONFIG)], cwd=ROOT,
                             capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        self.assertIn("verdict: PASS", res.stdout)


if __name__ == "__main__":
    unittest.main()
