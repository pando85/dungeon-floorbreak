"""ANIM-DONUT-02 Princess Donut melee and damage reactions (8 frozen keys and 7 executed in-betweens).

The shared toolkit checks structure, ticks, palette, lint, contact rows and the frozen keys
(``python3 -m scripts.pixelart validate assets/source/characters/donut/donut_h40_melee.pixelart.json``).
These tests add the spec rules it does not cover: identity lock on every frame, frame contract with the locomotion
atlas, the section-6 in-between rules, manifest tick windows and the art reach.
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
INBETWEENS = [2, 3, 7, 9, 12, 13, 14]
ALL = list(range(15))


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

    def test_inbetweens_are_drawn_not_copied(self):
        """Every frame is unique (the config sets ``unique_frames``) and no in-between is a copy of a key."""
        cels = [json.dumps(f["cels"], sort_keys=True) for f in self.grid["frames"]]
        self.assertEqual(len(set(cels)), len(cels))
        self.assertEqual(sorted(KEYS + INBETWEENS), ALL)
        self.assertEqual(json.loads(CONFIG.read_text(encoding="utf-8"))["rules"]["unique_frames"], True)

    def test_identity_block_is_a_translation_of_a_locomotion_frame(self):
        """Head, face and gear (tiara, collar, charm) are locomotion pixels, moved as one block, never redrawn."""
        block = ["head", "face", "gear"]
        sources = [flat(f, block) for f in self.loco["frames"]]
        for k in ALL:
            got = flat(self.grid["frames"][k], block)
            self.assertTrue(any(translated(got, s) for s in sources), f"frame {k}: identity block was edited")

    def test_body_and_tail_are_translations_too(self):
        """Every frame except two: p0 (gate fix: rump up 2 px, plume re-pixelled) and the tail of d1 (tip flick)."""
        for k in ALL:
            for layer in ("body", "tail"):
                if k == 4 or (k == 12 and layer == "tail"):
                    continue
                got = pixels(self.grid["frames"][k]["cels"][layer])
                ok = any(translated(got, pixels(f["cels"][layer])) for f in self.loco["frames"] if layer in f["cels"])
                self.assertTrue(ok, f"frame {k}: {layer} was edited")

    def test_d1_plume_tip_flicks_two_pixels_up(self):
        d0 = pixels(self.grid["frames"][11]["cels"]["tail"])
        d1 = pixels(self.grid["frames"][12]["cels"]["tail"])
        top0, top1 = min(y for _x, y in d0), min(y for _x, y in d1)
        self.assertEqual(top0 - top1, 2)
        self.assertEqual(max(y for _x, y in d0), max(y for _x, y in d1))            # the root does not move
        self.assertEqual({p: c for p, c in d0.items() if p[1] >= top0 + 8}, {p: c for p, c in d1.items() if p[1] >= top0 + 8})

    def test_hurt_knockdown_getup_eyes(self):
        """h0, kb and d0 squeeze their eyes shut (the master blink face); h1, d1, g0 and g1 have them open."""
        face = lambda k: pixels(self.grid["frames"][k]["cels"]["face"])  # noqa: E731
        blink = pixels(self.loco["frames"][14]["cels"]["face"])
        for k in (8, 10, 11):
            self.assertTrue(translated(face(k), blink), k)
        for k in (9, 12, 13, 14):
            self.assertFalse(translated(face(k), blink), k)
            self.assertTrue(any(c in "Yyg" for c in face(k).values()), f"frame {k}: no amber/gold eye pixels")

    def test_airborne_frames_clear_the_floor_and_grounded_frames_touch_it(self):
        for k in ALL:
            rows = [y for (_x, y) in flat(self.grid["frames"][k], self.grid["layers"])]
            if am.FRAMES[k][1] in ("pounce_air", "knockback"):
                self.assertLessEqual(max(rows), 41, k)
            else:
                self.assertEqual(max(rows), 43, k)

    def test_paw_layer_is_used_only_where_the_spec_says(self):
        for k in ALL:
            has = "paw" in self.grid["frames"][k]["cels"]
            self.assertEqual(has, k in (0, 1, 2, 5), k)

    def test_s2_paw_is_pressed_flat_on_the_floor(self):
        """Spec section 6: paw x60-67, rows 40-43, claws hidden (no cream-light claw pixels), hind paws creep forward."""
        paw = pixels(self.grid["frames"][2]["cels"]["paw"])
        flatpaw = {p: c for p, c in paw.items() if p[1] >= 40 and p[0] >= 60}
        self.assertEqual((min(x for x, _y in flatpaw), max(x for x, _y in flatpaw)), (60, 67))
        self.assertEqual((min(y for _x, y in flatpaw), max(y for _x, y in flatpaw)), (40, 43))
        self.assertNotIn("W", paw.values())
        hind = lambda k: min(x for (x, y), _c in flat(self.grid["frames"][k], ["legs_near"]).items() if x < 44 and y == 43)  # noqa: E731
        self.assertEqual(hind(2) - hind(1), 2)
        self.assertEqual(hind(3), hind(1))                      # back in the loco-15 stance one frame later

    def test_s3_is_the_land_recovery_stance(self):
        """Hand-over to idle or run: s3 is the locomotion 15 pixels, nothing added."""
        self.assertEqual(self.grid["frames"][3]["cels"], {l: r for l, r in self.loco["frames"][15]["cels"].items()})

    def test_g1_is_the_land_recovery_stance_with_the_chin_up(self):
        g1, loco15 = self.grid["frames"][14]["cels"], self.loco["frames"][15]["cels"]
        for layer in ("tail", "legs_far", "body", "legs_near"):
            self.assertEqual(g1[layer], loco15[layer], layer)
        for layer in ("head", "face", "gear"):
            self.assertTrue(translated(pixels(g1[layer]), pixels(loco15[layer])), layer)
            a, b = pixels(g1[layer]), pixels(loco15[layer])
            self.assertEqual(min(y for _x, y in b) - min(y for _x, y in a), 1, layer)

    def test_p3_and_g0_use_the_locomotion_forelegs(self):
        """p3 (forepaws back under the chest) and g0 carry the locomotion 10 forelegs; hinds in g0 stay flat."""
        for k in (7, 13):
            for layer in ("legs_near", "legs_far"):
                got = [r[44:] for r in self.grid["frames"][k]["cels"][layer]]
                want = [r[44:] for r in self.loco["frames"][10]["cels"][layer]]
                self.assertEqual(got, want, (k, layer))
        g0 = pixels(self.grid["frames"][13]["cels"]["legs_near"])
        hind = {p for p in g0 if p[0] < 44}
        self.assertEqual(max(y for _x, y in hind) - min(y for _x, y in hind), 3)       # flat: 4 rows, as in d0

    def test_swat_is_a_downward_slap(self):
        cock = pixels(self.grid["frames"][0]["cels"]["paw"])
        hit = pixels(self.grid["frames"][1]["cels"]["paw"])
        # after the gate fix the cock is already forward of the head, so the swipe is a downward slap:
        # the paw drops at least 6 rows at the same reach
        tip = lambda px: min(y for x, y in px if x >= max(x for x, _y in px) - 5)  # noqa: E731  paw rows
        self.assertGreaterEqual(tip(hit) - tip(cock), 6)
        self.assertLessEqual(abs(max(x for x, _y in hit) - max(x for x, _y in cock)), 1)
        self.assertGreaterEqual(max(x for x, _y in hit) - am.PIVOT[0], 22)

    def test_p0_is_a_low_forward_wedge(self):
        """Gate fix: plume at or below the back line, tiara is the highest pixel, head 3 px below jump_start."""
        p0 = self.grid["frames"][4]["cels"]
        top = lambda rows: min(y for _x, y in pixels(rows))  # noqa: E731
        self.assertGreaterEqual(top(p0["tail"]), top(p0["body"]))
        everything = flat(self.grid["frames"][4], self.grid["layers"])
        self.assertEqual(min(y for _x, y in everything), top(p0["gear"]))
        self.assertEqual(top(p0["head"]) - top(self.loco["frames"][10]["cels"]["head"]), 3)
        s0_top = min(y for (x, y) in flat(self.grid["frames"][0], ["head", "body", "tail"]))
        self.assertGreater(min(y for (x, y) in flat(self.grid["frames"][4], ["head", "body", "tail"])), s0_top)

    def test_s0_paw_breaks_the_head_outline(self):
        """Gate fix: at least 3 px of the cocked paw sit right of the head, with a K top and front edge."""
        paw = pixels(self.grid["frames"][0]["cels"]["paw"])
        head_right = max(x for x, _y in pixels(self.grid["frames"][0]["cels"]["head"]))
        self.assertGreaterEqual(max(x for x, _y in paw) - head_right, 3)
        top = min(y for _x, y in paw)
        self.assertTrue(all(c == "K" for (x, y), c in paw.items() if y == top))
        front = max(x for x, _y in paw)
        self.assertTrue(all(c == "K" for (x, y), c in paw.items() if x == front))

    def test_pounce_reaches_forward(self):
        paw = pixels(self.grid["frames"][5]["cels"]["paw"])
        self.assertGreaterEqual(max(x for x, _y in paw) - am.PIVOT[0], 22)

    def test_manifest_hit_boxes_lie_on_the_art(self):
        """Each manifest box (pivot-relative, y up, inclusive) sits inside the bounding box of its paw's pixels."""
        boxes = self.manifest["melee"]["hit_boxes_px"]
        where = [(boxes["swat_contact"]["paw"], 1, "paw"), (boxes["pounce_flight"]["near_paw"], 5, "paw"),
                 (boxes["pounce_flight"]["far_paw"], 5, "legs_far"), (boxes["pounce_pin"]["near_paw"], 6, "legs_near"),
                 (boxes["pounce_pin"]["far_paw"], 6, "legs_far")]
        px, py = am.PIVOT
        for (x0, y0, x1, y1), frame, layer in where:
            art = {(x, y) for (x, y) in pixels(self.grid["frames"][frame]["cels"][layer]) if x >= 44}
            self.assertGreaterEqual(x0 + px, min(x for x, _y in art), (frame, layer))
            self.assertLessEqual(x1 + px, max(x for x, _y in art), (frame, layer))
            self.assertGreaterEqual(py - y1, min(y for _x, y in art), (frame, layer))
            self.assertLessEqual(py - y0, max(y for _x, y in art), (frame, layer))

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
