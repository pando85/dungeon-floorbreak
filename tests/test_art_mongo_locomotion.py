"""ANIM-MONGO-01: structural and identity checks on the locomotion grid (keys and executor frames).

These pin what the art must keep (approved idle, master head pixels, palette, pivot and
contact convention, frozen keys, the run footfall and drift, the 1 px head bob, the tick
table, the in-between targets and the exported atlas). They do not judge artistic quality.
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

MASTER = ROOT / "assets/source/characters/mongo/mongo_adult_h76.grid.json"
GRID = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json"
ASE = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.ase"
MANIFEST = ROOT / "assets/manifests/mongo_adult_h76_locomotion.json"
CONFIG = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.pixelart.json"
KEYS = [4, 5, 6, 10, 12, 13, 14]
G = 87


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pixels(rows: list[str]) -> set[tuple[int, int, str]]:
    return {(x, y, c) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def normalised(px):
    x0 = min(x for x, _, _ in px)
    y0 = min(y for _, y, _ in px)
    return {(x - x0, y - y0, c) for x, y, c in px}


class MongoLocomotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.master = load(MASTER)
        cls.grid = load(GRID)
        cls.manifest = load(MANIFEST)
        pg.validate_grid(cls.grid)
        cls.tags = {t["name"]: (t["from"], t["to"]) for t in cls.grid["tags"]}

    def cels(self, i: int, layer: str) -> list[str]:
        w, h = self.grid["canvas"]
        return self.grid["frames"][i]["cels"].get(layer, ["." * w] * h)

    def test_manifest_and_layout(self) -> None:
        self.assertEqual(self.grid["canvas"], [128, 88])
        self.assertEqual(self.grid["pivot"], [76, 87])
        self.assertEqual(self.manifest["frame_size"], self.grid["canvas"])
        self.assertEqual(self.manifest["pivot"], self.grid["pivot"])
        self.assertEqual(self.manifest["review_status"], "prototype_fixture_unapproved")
        self.assertEqual(list(self.tags), ["idle", "run", "jump_start", "jump_rise", "jump_fall", "land"])
        self.assertEqual([b - a + 1 for a, b in self.tags.values()], [4, 6, 2, 1, 1, 2])
        self.assertEqual(self.grid["layers"], self.master["layers"])

    def test_palette_is_the_master_palette(self) -> None:
        self.assertEqual(self.grid["palette"], self.master["palette"])

    def test_idle_pixels_are_the_master_idle(self) -> None:
        # master 108x80 placed at (8, 8): master pivot (68, 79) -> (76, 87)
        for i in range(4):
            mine = pg.composite_keys(self.grid, i)
            ref = pg.composite_keys(self.master, i)
            for y in range(88):
                for x in range(128):
                    my, mx = y - 8, x - 8
                    want = ref[my][mx] if 0 <= my < 80 and 0 <= mx < 108 else "."
                    self.assertEqual(mine[y][x], want, f"idle {i} ({x},{y})")

    def test_head_is_a_rigid_copy_of_the_master_head(self) -> None:
        ref = normalised(pixels(self.master["frames"][0]["cels"]["head"]))
        for i in range(4, 16):
            self.assertEqual(normalised(pixels(self.cels(i, "head"))), ref, f"frame {i}")

    def test_keys_match_the_authoring_recipe(self) -> None:
        out = subprocess.run([sys.executable, str(ROOT / "scripts/author_mongo_locomotion.py"), "--check-keys"],
                             capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)

    def test_ase_round_trips_to_the_grid(self) -> None:
        self.assertEqual(pg.normalize(pg.ase_to_grid(ASE.read_bytes())), pg.normalize(self.grid))

    def test_validators_pass(self) -> None:
        cfg = load_config(CONFIG)
        report = validators.run(validators.build_context(cfg))
        self.assertTrue(report["passed"], validators.summary(report))

    def contact(self, i: int) -> dict[str, tuple[int, int]]:
        out = {}
        for layer in ("near_leg", "far_leg"):
            xs = [x for x, c in enumerate(self.cels(i, layer)[G]) if c != "."]
            if xs:
                out[layer] = (min(xs), max(xs))
        return out

    def test_run_footfall(self) -> None:
        # one foot on the floor in contact and passing frames, none in the airborne frames
        self.assertEqual(self.contact(4), {"near_leg": (84, 103)})
        self.assertEqual(self.contact(5), {"near_leg": (68, 87)})
        self.assertEqual(self.contact(6), {})
        self.assertEqual(self.contact(7), {"far_leg": (84, 103)})
        self.assertEqual(self.contact(8), {"far_leg": (68, 87)})
        self.assertEqual(self.contact(9), {})
        # planted foot travels back exactly 16 px per frame (the design D)
        self.assertEqual(self.contact(4)["near_leg"][0] - self.contact(5)["near_leg"][0], 16)

    def test_jump_frames_are_airborne_and_land_frames_grounded(self) -> None:
        report = pg.lint_grid(self.grid)
        bottoms = [f["bottom_row"] for f in report["frames"]]
        for i in (6, 9, 12, 13):
            self.assertLess(bottoms[i], G, f"frame {i}")
        for i in (0, 1, 2, 3, 4, 5, 7, 8, 10, 11, 14, 15):
            self.assertEqual(bottoms[i], G, f"frame {i}")

    def test_run_head_bob_is_at_most_one_pixel(self) -> None:
        tops = []
        for i in range(4, 10):
            rows = self.cels(i, "head")
            tops.append(min(y for y, r in enumerate(rows) if r.strip(".")))
        self.assertLessEqual(max(tops) - min(tops), 1, tops)

    def test_crouch_and_impact_differ(self) -> None:
        report = pg.lint_grid(self.grid)
        top = [f["bbox"][1] for f in report["frames"]]
        self.assertGreaterEqual(top[14] - top[10], 3, "impact must sit lower than the crouch")

    def test_impact_only_toes_touch_the_floor(self) -> None:
        # gate fix 1: frame 14 near leg on rows 86-87 is the foot stamp only (no hock on the floor)
        import author_mongo_locomotion as rig
        spec = rig.POSES[14]["near"]
        bx, by = spec["ball"]
        foot = {(x + bx, y + by) for (x, y) in rig.FEET[(spec["foot"], "near")]}
        rows = self.cels(14, "near_leg")
        low = {(x, y) for y in (86, 87) for x, c in enumerate(rows[y]) if c != "."}
        self.assertTrue(low, "frame 14 near foot must be planted")
        self.assertLessEqual(low, foot, sorted(low - foot))
        self.assertEqual(self.contact(14)["near_leg"], (86, 105))
        bdx, bdy = rig.POSES[14]["body"]
        tx, ty = spec["thigh"]
        hock = rig.solve_hock((rig.KNEE[0] + bdx + tx, rig.KNEE[1] + bdy + ty), spec["ball"])
        self.assertLessEqual(hock[1], 80, f"hock {hock} must sit at least 3 px above the ball row")

    def test_run_push_off_is_a_toe_off(self) -> None:
        # gate fix 2: in the airborne run frames both feet are clearly off the floor (lowest row <= 84)
        for i in (6, 9):
            for layer in ("near_leg", "far_leg"):
                rows = self.cels(i, layer)
                lowest = max(y for y, r in enumerate(rows) if r.strip("."))
                self.assertLessEqual(lowest, 84, f"frame {i} {layer}")
        self.assertEqual(__import__("author_mongo_locomotion").POSES[6]["near"]["foot"], "toe")

    def test_clean_clusters(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(report["isolated_count_per_frame"], [0] * 16)
        for i in range(16):
            self.assertLessEqual(report["orphan_count_per_frame"][i], 2, f"frame {i}")

    # ---- executor frames (7, 8, 9, 11, 15), tick table, drift, atlas ---------------------

    def head_top(self, i: int) -> int:
        rows = self.cels(i, "head")
        return min(y for y, r in enumerate(rows) if r.strip("."))

    def head_left(self, i: int) -> int:
        return min(x for r in self.cels(i, "head") for x, c in enumerate(r) if c != ".")

    def test_all_frames_match_the_authoring_recipe(self) -> None:
        import author_mongo_locomotion as rig
        for i, cels in rig.build_frames().items():
            for layer in rig.LAYERS:
                self.assertEqual(self.cels(i, layer), cels[layer], f"frame {i} {layer}")

    def test_ground_contact_rows(self) -> None:
        # (near, far) opaque run on row 87 per frame; None = that leg is off the floor
        want = {
            0: ((59, 78), (76, 92)), 4: ((84, 103), None), 5: ((68, 87), None), 6: (None, None),
            7: (None, (84, 103)), 8: (None, (68, 87)), 9: (None, None),
            10: ((61, 80), (75, 94)), 11: ((56, 57), (68, 69)), 12: (None, None), 13: (None, None),
            14: ((86, 105), (54, 73)), 15: ((74, 93), (60, 79)),
        }
        for i, (near, far) in want.items():
            got = self.contact(i)
            self.assertEqual(got.get("near_leg"), near, f"frame {i} near")
            self.assertEqual(got.get("far_leg"), far, f"frame {i} far")

    def test_planted_feet_touch_the_floor_with_foot_stamps_only(self) -> None:
        import author_mongo_locomotion as rig
        specs = {**rig.POSES, **rig.DRAFTS}
        for i in (4, 5, 7, 8, 10, 11, 14, 15):
            for side, layer in (("near", "near_leg"), ("far", "far_leg")):
                spec = specs[i][side]
                bx, by = spec["ball"]
                if spec["foot"] in ("curl", "reach"):
                    continue
                foot = {(x + bx, y + by) for (x, y) in rig.FEET[(spec["foot"], side)]}
                rows = self.cels(i, layer)
                low = {(x, y) for y in (86, 87) for x, c in enumerate(rows[y]) if c != "."}
                self.assertLessEqual(low, foot, f"frame {i} {side}: non-foot pixels {sorted(low - foot)}")

    def test_run_plant_drift_at_185_px_per_second(self) -> None:
        # Each foot is planted in two frames and moves back D = 16 px between them (sprite space).
        # The ground moves v * ticks / 60 per frame; the leftover is the visible slide.
        run_speed, ticks = 185.0, 5
        self.assertEqual(self.contact(4)["near_leg"][0] - self.contact(5)["near_leg"][0], 16)
        self.assertEqual(self.contact(7)["far_leg"][0] - self.contact(8)["far_leg"][0], 16)
        travel = run_speed * ticks / 60
        self.assertAlmostEqual(travel, 15.42, places=2)
        self.assertLessEqual(abs(16 - travel), 1.0, "planted foot slides more than 1 px per frame at RUN_SPEED")
        # the same foot is never planted in a third frame, so there is no extra slide
        self.assertEqual([i for i in range(4, 10) if "near_leg" in self.contact(i)], [4, 5])
        self.assertEqual([i for i in range(4, 10) if "far_leg" in self.contact(i)], [7, 8])
        # the two steps share one stride and one x
        self.assertEqual(self.contact(4)["near_leg"], self.contact(7)["far_leg"])
        self.assertEqual(self.contact(5)["near_leg"], self.contact(8)["far_leg"])

    def test_run_loop_seam_advances_one_stride(self) -> None:
        # 8 (far foot passing, x 68-87) -> 9 (air) -> 4 (near foot contact, x 84-103): the next plant
        # lands 16 px ahead of the passing foot, like 5 -> 7 (near passing -> far contact)
        self.assertEqual(self.contact(4)["near_leg"][0] - self.contact(8)["far_leg"][0], 16)
        self.assertEqual(self.contact(7)["far_leg"][0] - self.contact(5)["near_leg"][0], 16)

    def test_pivot_alignment_idle_to_run(self) -> None:
        idle = self.contact(0)
        lo = min(a for a, _ in idle.values())
        hi = max(b for _, b in idle.values())
        self.assertEqual(self.grid["pivot"], [76, 87])
        for i, leg in ((5, "near_leg"), (8, "far_leg")):  # weight-bearing passing frames
            a, b = self.contact(i)[leg]
            self.assertTrue(lo <= a and b <= hi, f"frame {i} foot {a}-{b} outside the idle stance {lo}-{hi}")
            self.assertTrue(a <= self.grid["pivot"][0] <= b, f"frame {i} foot does not cover the pivot column")
        # the whole run stays on the idle's floor row and the head moves at most the documented lean
        for i in range(4, 10):
            self.assertLessEqual(max(y for y, r in enumerate(pg.composite_keys(self.grid, i)) if any(c != "." for c in r)), G)
            self.assertLessEqual(abs(self.head_left(i) - self.head_left(0)), 10, f"frame {i}")
            self.assertLessEqual(abs(self.head_top(i) - self.head_top(0)), 4, f"frame {i}")

    def test_palette_is_the_idle_palette_in_every_frame(self) -> None:
        pal = [p["key"] for p in self.grid["palette"]]
        self.assertEqual(len(pal), 24)
        self.assertEqual(pal, [p["key"] for p in self.master["palette"]])
        for i in range(16):
            used = {c for layer in self.grid["layers"] for r in self.cels(i, layer) for c in r} - {"."}
            self.assertTrue(used <= set(pal), f"frame {i}: {sorted(used - set(pal))}")

    def test_frame_count_per_tag_and_tick_table(self) -> None:
        cfg = load_config(CONFIG)
        want = {"idle": [27, 18, 33, 10], "run": [5] * 6, "jump_start": [6, 4], "jump_rise": [None],
                "jump_fall": [None], "land": [5, 7]}
        for name, ticks in want.items():
            self.assertEqual([t for _, t in cfg.tags[name]["frames"]], ticks, name)
            a, b = self.tags[name]
            self.assertEqual(b - a + 1, len(ticks), name)
            for f, t in cfg.tags[name]["frames"]:
                if t is not None:
                    self.assertEqual(self.grid["frames"][f]["duration_ms"], round(t * 1000 / 60), f"frame {f}")

    def test_in_between_targets(self) -> None:
        # frame 11 (spec section 9, item 2): between crouch 10 and rise 12
        self.assertLessEqual(abs(self.head_top(11) - 21), 1)
        self.assertLessEqual(abs(self.head_left(11) - 93), 1)
        low = [x for leg in self.contact(11).values() for x in leg]
        self.assertTrue(54 <= min(low) and max(low) <= 81, low)
        tails = [min(x for r in self.cels(i, "tail") for x, c in enumerate(r) if c != ".") for i in (10, 11, 12)]
        self.assertLessEqual(abs(tails[1] - (tails[0] + tails[2]) / 2), 1.0, tails)
        # frame 15 (item 3): blends into the idle without a pop, hocks well above the floor
        self.assertLessEqual(abs(self.head_top(15) - 24), 1)
        self.assertLessEqual(abs(self.head_left(15) - 90), 1)
        self.assertLessEqual(abs(self.head_left(15) - self.head_left(0)), 6)
        self.assertLessEqual(abs(self.head_top(15) - self.head_top(0)), 6)
        import author_mongo_locomotion as rig
        spec = rig.DRAFTS[15]
        bdx, bdy = spec["body"]
        for side in ("near", "far"):
            tx, ty = spec[side]["thigh"]
            hock = rig.solve_hock((rig.KNEE[0] + bdx + tx, rig.KNEE[1] + bdy + ty), spec[side]["ball"])
            self.assertLessEqual(hock[1], 80, f"frame 15 {side} hock {hock}")

    def test_run_far_leg_reads_in_frames_7_and_8(self) -> None:
        for i in (7, 8):
            top = {}
            for layer in self.grid["layers"]:
                for y, r in enumerate(self.cels(i, layer)):
                    for x, c in enumerate(r):
                        if c != ".":
                            top[(x, y)] = layer
            rows = {y for (x, y), layer in top.items() if layer == "far_leg" and 66 <= y <= 80}
            self.assertGreaterEqual(len(rows), 3, f"frame {i}: far shin visible on rows {sorted(rows)}")

    def test_near_leg_does_not_overlap_the_planted_far_foot(self) -> None:
        import author_mongo_locomotion as rig
        for i in (7, 8):
            spec = rig.DRAFTS[i]["far"]
            bx, by = spec["ball"]
            foot = {(x + bx, y + by) for (x, y) in rig.FEET[(spec["foot"], "far")]}
            overlap = {(x, y) for x, y, _ in pixels(self.cels(i, "near_leg"))} & foot
            self.assertFalse(overlap, f"frame {i}: {sorted(overlap)}")

    def test_exported_atlas_matches_the_grid(self) -> None:
        atlas = ROOT / self.manifest["output_atlas"]
        meta = ROOT / self.manifest["output_metadata"]
        if not (atlas.exists() and meta.exists()):
            self.skipTest("run bash scripts/export_assets.sh to export the atlas")
        from PIL import Image
        data = load(meta)
        self.assertEqual(len(data["frames"]), 16)
        tags = {t["name"]: (t["from"], t["to"]) for t in data["meta"]["frameTags"]}
        self.assertEqual(tags, self.tags)
        pal = {p["key"]: pg.parse_rgba(p["rgba"]) for p in self.grid["palette"]}
        with Image.open(atlas) as opened:
            im = opened.convert("RGBA")
        self.assertLessEqual(len({c for _, c in im.getcolors(1 << 16) if c[3] == 255}), 24)
        for i, fr in enumerate(data["frames"]):
            r = fr["frame"]
            self.assertEqual((r["w"], r["h"]), (128, 88))
            crop = im.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))
            flat = pg.composite_keys(self.grid, i)
            for y in range(88):
                for x in range(128):
                    got = crop.getpixel((x, y))
                    if flat[y][x] == ".":
                        self.assertEqual(got[3], 0, f"frame {i} ({x},{y})")
                    else:
                        self.assertEqual(tuple(got), tuple(pal[flat[y][x]]), f"frame {i} ({x},{y})")


if __name__ == "__main__":
    unittest.main()
