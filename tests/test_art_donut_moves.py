"""DONUT-MOVES-ART-01: the production sheets of Donut's reflex cues and Puddle Jumper
(docs/02-art/donut-moves-art-01-spec.md, sections 3 to 9).

The shared toolkit validators run with ``python3 -m scripts.pixelart validate <config>``; this file covers the spec
rules they do not: the authoring record is reproducible, the palettes and the canvas, the frozen keys, the
never-a-flip rules of the evade, the tail-swap limits and the VFX strip.
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_donut_missile as missile  # noqa: E402
import author_donut_moves as A  # noqa: E402
import author_donut_moves_keys as keys_script  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart.validators import frame_digest  # noqa: E402

BODY_LAYERS = ("legs_far", "body", "legs_near", "head", "face", "gear")


def pixels(rows: list[str] | None) -> dict:
    return {(x, y): c for y, r in enumerate(rows or []) for x, c in enumerate(r) if c != "."}


def flat(frame: dict) -> dict:
    out = {}
    for layer in A.LAYERS:
        out.update(pixels(frame["cels"].get(layer)))
    return out


def bbox(p: dict):
    xs, ys = [x for x, _ in p], [y for _, y in p]
    return min(xs), min(ys), max(xs), max(ys)


class DonutMovesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.loco = A.load(A.LOCO_GRID)
        cls.moves = A.load(A.MOVES_GRID)
        cls.tail = A.load(A.TAIL_GRID)
        cls.vfx = A.load(A.VFX_GRID)
        cls.keys = A.load(A.MOVES_KEYS_GRID)

    # --- authoring record and sources
    def test_authoring_scripts_reproduce_every_output(self) -> None:
        self.assertEqual(keys_script.main(["--check"]), 0)
        self.assertEqual(A.main(["keys", "--check"]), 0)

    def test_ase_files_match_the_grids(self) -> None:
        for grid_path in (A.MOVES_GRID, A.TAIL_GRID, A.VFX_GRID):
            grid = A.load(grid_path)
            data = pg.read_ase(grid_path.with_suffix("").with_suffix(".ase").read_bytes())
            self.assertEqual(data["canvas"], grid["canvas"], grid_path.name)
            self.assertEqual([f["duration_ms"] for f in data["frames"]],
                             [f["duration_ms"] for f in grid["frames"]], grid_path.name)

    def test_the_toolkit_validators_pass_on_every_config(self) -> None:
        for cfg in (A.MOVES_CFG, A.TAIL_CFG, A.VFX_CFG):
            done = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(cfg)], cwd=ROOT,
                                  capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    # --- palette, canvas, pivot, alpha (spec section 7 and 9.2)
    def test_donut_sheets_keep_the_locomotion_palette_canvas_and_pivot(self) -> None:
        for grid in (self.moves, self.tail):
            self.assertEqual(grid["canvas"], [72, 44])
            self.assertEqual(grid["pivot"], [46, 43])
            self.assertEqual(grid["palette"], self.loco["palette"])
            self.assertEqual(len(grid["palette"]), 22)
            self.assertTrue(all(e["rgba"].endswith("ff") for e in grid["palette"]), "binary alpha")
        self.assertEqual(self.moves["layers"], keys_script.LAYERS)
        self.assertEqual(self.tail["layers"], self.loco["layers"])

    def test_every_donut_frame_is_anchored_with_an_empty_margin(self) -> None:
        for n, frame in enumerate(self.moves["frames"]):
            x0, y0, x1, y1 = bbox(flat(frame))
            self.assertEqual(y1, 43, f"moves {n}: lowest row")
            self.assertGreaterEqual(y0, 1, f"moves {n}: row 0 empty")
            self.assertGreaterEqual(x0, 1, f"moves {n}: left column empty")
            self.assertLessEqual(x1, 70, f"moves {n}: right column empty")

    def test_frame_sizes_stay_inside_the_spec_bounds(self) -> None:
        for n in range(5):                                  # evade: x -44..+24 of the pivot, 42 rows
            x0, y0, x1, y1 = bbox(flat(self.moves["frames"][n]))
            self.assertGreaterEqual(x0 - 46, -44, n)
            self.assertLessEqual(x1 - 46, 24, n)
            self.assertLessEqual(y1 - y0 + 1, 42, n)
        # The puddle frames (b1, b2, a0, a1) stay inside the puddle mouth's -26..+25. b0, a2, f0 and f1 sit on the
        # approved melee, CF02 and locomotion bodies and keep their approved extents (adopted default DMA-Q11).
        for n in (6, 7, 8, 9):
            x0, _y0, x1, _y1 = bbox(flat(self.moves["frames"][n]))
            self.assertGreaterEqual(x0 - 46, -26, n)
            self.assertLessEqual(x1 - 46, 25, n)

    def test_lint_is_clean_on_the_moves_sheet(self) -> None:
        report = pg.lint_grid(self.moves)
        self.assertEqual(report["isolated_count_per_frame"], [0] * 13)
        self.assertEqual(report["orphan_count_per_frame"], [0] * 13)
        self.assertEqual(report["bottom_row_drift"], 0)
        done = subprocess.run([sys.executable, str(ROOT / "scripts/pixel_grid_to_ase.py"), "lint", "--strict",
                               str(A.MOVES_GRID)], capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    # --- frozen keys (spec 9.3)
    def test_frames_e2_and_a1_are_the_frozen_keys_byte_for_byte(self) -> None:
        self.assertEqual(self.moves["frames"][2]["cels"], self.keys["frames"][0]["cels"])
        self.assertEqual(self.moves["frames"][9]["cels"], self.keys["frames"][1]["cels"])
        cfg = json.loads(A.MOVES_CFG.read_text())
        spec = cfg["rules"]["frozen_keys"]
        self.assertEqual(spec["frames"], [2, 9])
        fixture = json.loads((ROOT / spec["fixture"]).read_text())
        self.assertEqual(set(fixture["keys"]), {"2", "9"})
        for n in (2, 9):
            got = frame_digest(self.moves["frames"][n]["cels"], spec["layers"])
            self.assertEqual(got, fixture["keys"][str(n)]["sha256"], n)

    # --- never a flip (spec 9.4)
    def test_the_evade_head_is_an_unrotated_shift_of_an_approved_head(self) -> None:
        """The same pixels as the approved master head translated, never rotated. The strict-lint orphan cleanup
        (author_donut_moves_keys.cleanup) may recolour a single outline pixel, so at most 2 colours differ."""
        master = pixels(self.loco["frames"][0]["cels"]["head"])
        for n in range(5):
            head = pixels(self.moves["frames"][n]["cels"]["head"])
            mx, my = min(master)
            hx, hy = min(head)
            dx, dy = hx - mx, hy - my
            moved = {(x + dx, y + dy): c for (x, y), c in master.items()}
            self.assertEqual(set(head), set(moved), f"evade frame {n}: same pixel set")
            self.assertLessEqual(sum(head[p] != moved[p] for p in head), 2, f"evade frame {n}")

    def test_the_evade_head_stays_above_the_rump_and_no_paw_rises_above_the_tiara(self) -> None:
        for n in range(5):
            cels = self.moves["frames"][n]["cels"]
            head_bottom = max(y for (_x, y) in pixels(cels["head"]))
            rump_bottom = max(y for (_x, y) in pixels(cels["body"]))
            self.assertLess(head_bottom, rump_bottom, f"evade frame {n}: head below the rump")
            tiara_top = min(y for (_x, y) in pixels(cels["gear"]))
            for layer in ("legs_far", "legs_near"):
                if cels.get(layer):
                    paw_top = min(y for (_x, y) in pixels(cels[layer]))
                    self.assertGreater(paw_top, tiara_top, f"evade frame {n}: {layer} above the tiara")

    def test_the_evade_frames_are_distinct_from_the_jump_and_the_hurt(self) -> None:
        melee = A.load(A.MELEE_GRID)
        banned = [flat(self.loco["frames"][12]), flat(self.loco["frames"][13]), flat(melee["frames"][5]),
                  flat(melee["frames"][8])]
        for n in range(5):
            self.assertNotIn(flat(self.moves["frames"][n]), banned, n)

    # --- the puddle frames (spec 5)
    def test_the_blink_frames_use_the_puddle_layers_as_specified(self) -> None:
        f = self.moves["frames"]
        self.assertTrue(f[5]["cels"].get("puddle_back"))
        self.assertFalse(f[5]["cels"].get("puddle_front"), "b0 is puddle_back only, a thin rim")
        for n in (6, 7, 8):
            self.assertTrue(f[n]["cels"].get("puddle_back") and f[n]["cels"].get("puddle_front"), n)
        # b1 and b2 hide her under the front rim: nothing of her body or head below the waterline
        for n in (6, 7):
            for layer in ("body", "head", "tail"):
                for (_x, y) in pixels(f[n]["cels"].get(layer)):
                    self.assertLess(y, A.WATERLINE, (n, layer))
        # b2: only the tail tip (about 8 px) above the rim
        tip = pixels(f[7]["cels"]["tail"])
        self.assertTrue(6 <= max(y for (_x, y) in tip) - min(y for (_x, y) in tip) + 1 <= 12)
        self.assertFalse(f[7]["cels"].get("body") or f[7]["cels"].get("head"))
        # a0: only ears, tiara and eyes remain: the head layers, no body, no tail, no paws
        a0 = f[8]["cels"]
        self.assertFalse(a0.get("body") or a0.get("tail") or a0.get("legs_near"))
        self.assertTrue(a0["head"] and a0["face"] and a0["gear"])
        # a2: the 2-row ring at half width, lilac over magenta
        ring = pixels(f[10]["cels"]["puddle_front"])
        self.assertEqual({y for (_x, y) in ring}, {42, 43})
        self.assertEqual({c for c in ring.values()}, {"L", "M"})
        full = bbox(pixels(f[9]["cels"]["puddle_back"]))
        self.assertLessEqual(max(x for (x, _y), c in ring.items() if c == "M") - min(x for (x, _y), c in ring.items()
                                                                                   if c == "M"),
                             (full[2] - full[0]) // 2 + 1)

    def test_the_puddle_mouth_uses_only_the_shared_donut_colours(self) -> None:
        allowed = set("MVLDK")
        for n in (5, 6, 7, 8, 9, 10):
            for layer in ("puddle_back", "puddle_front"):
                self.assertLessEqual(set(pixels(self.moves["frames"][n]["cels"].get(layer)).values()), allowed,
                                     (n, layer))

    def test_the_fizzle_uses_the_cf02_startle_heads(self) -> None:
        cf = A.load(A.CF02_GRID)
        f0 = self.moves["frames"][11]["cels"]
        puff = pixels(cf["frames"][0]["cels"]["face"])
        got = pixels(f0["face"])
        self.assertEqual(sorted(got.values()), sorted(puff.values()), "the startle_puff face, shifted")
        f1 = self.moves["frames"][12]["cels"]
        settle = cf["frames"][1]["cels"]
        for layer in ("head", "face", "gear", "body"):
            self.assertEqual(f1[layer], settle[layer], layer)
        self.assertNotEqual(f1["tail"], settle["tail"], "the tail is lashed to one side")
        self.assertFalse(f1.get("puddle_back") or f1.get("puddle_front"), "f1 has no puddle")

    def test_ticks_and_frame_maps_follow_the_spec_tables(self) -> None:
        man = json.loads(A.MOVES_MANIFEST.read_text())["moves"]
        ticks = {t: sum(k for _n, tag, k, _a in A.MOVES if tag == t) for t in ("evade", "blink_depart",
                                                                              "blink_arrive", "fizzle")}
        self.assertEqual(ticks, {"evade": 12, "blink_depart": 6, "blink_arrive": 7, "fizzle": 8})
        expect = []
        for n, (_name, tag, k, _a) in enumerate(A.MOVES):
            if tag == "evade":
                expect += [n] * k
        self.assertEqual(man["evade_frame_for_age"], expect)
        self.assertEqual(man["fizzle_frame_for_age"], [11] * 4 + [12] * 4)
        self.assertEqual([f["ticks"] for f in man["frames"]], [k for _n, _t, k, _a in A.MOVES])

    # --- tail swap (spec 3, acceptance 7)
    def test_the_tail_swap_changes_only_the_tail_at_most_24_pixels_and_keeps_the_root_half(self) -> None:
        self.assertEqual(len(self.tail["frames"]), 32)
        for i, frame in enumerate(self.tail["frames"]):
            base = self.loco["frames"][i % 16]["cels"]
            for layer in self.loco["layers"]:
                if layer != "tail":
                    self.assertEqual(frame["cels"].get(layer), base.get(layer), (i, layer))
            a, b = pixels(base["tail"]), pixels(frame["cels"]["tail"])
            changed = {p for p in set(a) | set(b) if a.get(p) != b.get(p)}
            self.assertTrue(0 < len(changed) <= A.MAX_TAIL_CHANGES, (i, len(changed)))
            _root, dist = A.tail_distances(a, pixels(base["body"]))
            half = max(dist.values()) / 2
            for p, d in dist.items():
                if d <= half:
                    self.assertEqual(b.get(p), a[p], f"frame {i}: root half changed at {p}")

    def test_the_two_phases_differ_from_the_base_and_from_each_other(self) -> None:
        for n in range(16):
            base = self.loco["frames"][n]["cels"]["tail"]
            up = self.tail["frames"][n]["cels"]["tail"]
            snap = self.tail["frames"][16 + n]["cels"]["tail"]
            self.assertEqual(len({tuple(base), tuple(up), tuple(snap)}), 3, n)

    def test_the_flick_up_raises_and_leans_back_and_the_snap_curls_forward_with_one_fur_pixel(self) -> None:
        for n in range(16):
            base = pixels(self.loco["frames"][n]["cels"]["tail"])
            up = pixels(self.tail["frames"][n]["cels"]["tail"])
            snap = pixels(self.tail["frames"][16 + n]["cels"]["tail"])
            self.assertEqual(min(y for (_x, y) in up), min(y for (_x, y) in base) - 2, f"up {n}: tip rises 2 px")
            self.assertGreaterEqual(min(y for (_x, y) in snap), min(y for (_x, y) in base), f"snap {n}: curls down")
            gained = {p for p in snap if snap[p] == "G" and base.get(p) != "G"}
            self.assertTrue(gained, f"snap {n}: the extra G fur pixel")

    def test_tail_tip_is_the_eight_farthest_pixels_by_path_distance(self) -> None:
        base = self.loco["frames"][0]["cels"]
        tail, body = pixels(base["tail"]), pixels(base["body"])
        tip = A.tail_tip(tail, body)
        self.assertEqual(len(tip), 8)
        _root, dist = A.tail_distances(tail, body)
        self.assertEqual(min(dist[p] for p in tip), sorted(dist.values())[-8])

    # --- VFX strip (spec 5, acceptance 2)
    def test_the_vfx_strip_keeps_the_magic_missile_palette_unchanged(self) -> None:
        missile_vfx = json.loads(missile.VFX_GRID.read_text())
        self.assertEqual(self.vfx["palette"], missile_vfx["palette"])
        self.assertEqual({e["key"] for e in self.vfx["palette"]}, set("kdmpcwvl"))
        self.assertTrue(all(e["rgba"].endswith("ff") for e in self.vfx["palette"]), "binary alpha")
        self.assertEqual(self.vfx["canvas"], [48, 16])
        self.assertEqual(self.vfx["pivot"], [24, 11])
        self.assertEqual(len(self.vfx["frames"]), 16)

    def test_the_vfx_frames_are_lint_clean_and_use_the_orphan_allowance(self) -> None:
        report = pg.lint_grid(self.vfx)
        self.assertEqual(report["isolated_count_per_frame"], [0] * 16)
        self.assertLessEqual(max(report["orphan_count_per_frame"]), 9)
        used = set(report["colors_used"])
        self.assertLessEqual(used, set("kdmpcwvl"))

    def test_the_vfx_cells_follow_the_spec_table(self) -> None:
        rows = [f["cels"]["body"] for f in self.vfx["frames"]]
        ring_open = [pixels(rows[n]) for n in range(4)]
        # concentric flat rings centred on the pivot column, half widths 6, 10, 14 and 18 (frame 3: the outer only)
        half = lambda p, y: (max(x for (x, yy) in p if yy == y) - min(x for (x, yy) in p if yy == y)) // 2  # noqa: E731
        self.assertEqual(half(ring_open[0], 11), 6)
        self.assertEqual(half(ring_open[1], 11), 10)
        self.assertEqual(half(ring_open[2], 11), 14)
        self.assertEqual(half(ring_open[3], 11), 18)
        for n, hw in ((4, 14), (5, 8), (6, 3)):
            self.assertEqual(half(pixels(rows[n]), 11), hw, f"ring_close {n}")
            self.assertEqual(sum(c == "l" for c in pixels(rows[n]).values()), 4, f"two 2 px lilac drops in {n}")
        for n, hw in ((9, 18), (10, 10), (11, 4)):
            self.assertEqual(half(pixels(rows[n]), 11), hw, f"fizzle {n}")
        # the tileable dash is 16 x 3 with the 2 px c head; the head piece is 4 x 3
        x0, y0, w, h = A.DASH_RECT
        dash = pixels(rows[7])
        self.assertEqual(bbox(dash), (x0, y0, x0 + w - 1, y0 + h - 1))
        self.assertEqual({dash[(x, y0 + 2)] for x in range(x0, x0 + w)}, {"d"})
        self.assertEqual({dash[(x, y0 + 1)] for x in range(x0, x0 + w - 2)}, {"p"})
        self.assertEqual({dash[(x, y0 + 1)] for x in range(x0 + w - 2, x0 + w)}, {"c"})
        hx, hy, hw, hh = A.HEAD_RECT
        self.assertEqual(bbox(pixels(rows[8])), (hx, hy, hx + hw - 1, hy + hh - 1))
        # the glint is a 3 x 3 plus: w centre, c arms; the twinkle is the small core
        glint = pixels(rows[12])
        self.assertEqual(glint[(24, 11)], "w")
        self.assertEqual(sorted(c for p, c in glint.items() if p != (24, 11)), ["c"] * 4)
        self.assertEqual(bbox(glint), (23, 10, 25, 12))
        self.assertLess(len(pixels(rows[13])), len(glint))
        # the swish is a thin arc in c with a w head and a k underside, and no magenta
        for n in (14, 15):
            p = pixels(rows[n])
            self.assertEqual(set(p.values()), {"c", "w", "k"})
            self.assertLessEqual(bbox(p)[3] - bbox(p)[1] + 1, 8)

    def test_vfx_ticks_and_tags_match_the_manifest(self) -> None:
        man = json.loads(A.VFX_MANIFEST.read_text())["vfx"]
        self.assertEqual([f["ticks"] for f in man["frames"]], A.VFX_TICKS)
        self.assertEqual(sum(A.VFX_TICKS[:4]), 12, "ring_open: 3 ticks a frame")
        self.assertEqual(sum(A.VFX_TICKS[4:7]), 9, "ring_close: 3 ticks a frame")
        self.assertEqual(sum(A.VFX_TICKS[9:12]), 8, "fizzle: 3 + 3 + 2 = FIZZLE_TICKS")
        self.assertEqual(man["surface_row"], 11)
        self.assertEqual(len({json.dumps(f["cels"], sort_keys=True) for f in self.vfx["frames"]}), 16)


if __name__ == "__main__":
    unittest.main()
