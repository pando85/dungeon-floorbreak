"""AREA-HAZARDS-01B: the teal acid, the ring tape backing, the acid entry splash, the corrosion
decals and the Rot marks (sources, configs, manifests, fixtures and the spec's palette rules)."""
from __future__ import annotations

import hashlib
import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_danger_fx as fx  # noqa: E402
import author_hazard_decals as hz  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
KEYS = PROPS / "hz01b_keys"
MANIFESTS = ROOT / "assets/manifests"
FIXTURES = ROOT / "tests/fixtures"
ROT = ROOT / "assets/source/characters/rot_sticker/rot_01.grid.json"

# Canvas, pivot and tags (name -> frame count) per grid: spec sections 3 and 3.1.
SHEETS = {
    "dfx_acid_splash": ((32, 24), (16, 23), {"splash": 5}),
    "dfx_corr_face": ((32, 10), (16, 0), {"corr_face": 1}),
    "dfx_corr_top": ((32, 6), (16, 5), {"corr_top": 1}),
    "dfx_corr_wall": ((12, 24), (0, 23), {"corr_wall": 1}),
    "dfx_rot_nest": ((32, 12), (16, 0), {"nest": 1}),
    "dfx_rot_trail": ((16, 3), (8, 0), {"trail": 1}),
    "dfx_rot_drip": ((8, 16), (4, 0), {"drip_run_c": 4, "drip_drop_c": 3, "drip_run_w": 4, "drip_drop_w": 3}),
}
SPLASH_TICKS = [2, 3, 3, 4, 4]
LIT_ACID = {"acid_2_mid", "acid_3_light", "acid_4_lit", "acid_sheen", "acid_fizz"}
ROT_FORBIDDEN = {"glow_ember", "glow_hot", "glow_core", "rib_red", "spike_tip", "flash_white"}
# SHA-256 of the dfx_acid rows before HZB-AD1 (the SLIME-01 borrow): the recolour changes
# palette entries only, every row is byte-identical.
ACID_ROWS_SHA256 = "c66ac174b21bf0e58fd9464673fd03f11c6c9c4aef14166afbec5cc0344ab139"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def names_of(grid: dict, frames=None) -> set[str]:
    """The palette entry names that frames of ``grid`` use."""
    by_key = {p["key"]: p["name"] for p in grid["palette"]}
    used: set[str] = set()
    for i, f in enumerate(grid["frames"]):
        if frames is not None and i not in frames:
            continue
        for row in f["cels"]["body"]:
            used.update(by_key[c] for c in row if c != ".")
    return used


def rows_digest(grid: dict) -> str:
    h = hashlib.sha256()
    for f in grid["frames"]:
        for row in f["cels"]["body"]:
            h.update((row + "\n").encode())
    return h.hexdigest()


class AcidAndRingsTest(unittest.TestCase):
    """HZB-AD1 (the teal ramp) and HZB-AD2 (the dark tape backing)."""

    def test_the_acid_rows_are_unchanged_and_the_ramp_is_the_spec_table(self) -> None:
        grid = load(PROPS / "dfx_acid.grid.json")
        self.assertEqual(rows_digest(grid), ACID_ROWS_SHA256, "only the palette entries moved")
        want = [("#", "acid_outline", "#061418ff"), ("$", "acid_0_deep", "#0d3034ff"),
                ("%", "acid_1_shade", "#124c50ff"), ("&", "acid_2_mid", "#197a7eff"),
                ("'", "acid_3_light", "#2eb2a8ff"), ("(", "acid_4_lit", "#78e2d4ff"),
                (")", "acid_sheen", "#c6f8eeff"), ("*", "acid_fizz", "#eafffaff"),
                ("+", "fume_lit", "#a4c4c0ff"), (",", "fume_dark", "#587a78ff")]
        self.assertEqual([(p["key"], p["name"], p["rgba"]) for p in grid["palette"]], want)
        self.assertEqual(list(hz.ACID_RAMP), [(n, r) for _k, n, r in want])
        fixture = load(FIXTURES / "f01-dfx-acid_keys.json")
        self.assertEqual(fixture["keys"]["0"]["sha256"],
                         "6eae3b15444d26caa6d7b3f7c4ad785dcdf6ea7ded47fdf83bed92bb180e9c44",
                         "the acid fixture did not move")

    def test_the_outer_band_row_of_every_ring_is_the_dark_backing(self) -> None:
        for r in fx.RADII:
            c = r + fx.MARGIN
            grid = load(PROPS / f"dfx_ring_r{r}.grid.json")
            by_key = {p["key"]: p["name"] for p in grid["palette"]}
            self.assertLessEqual(len(grid["palette"]), 63)
            for i, frame in enumerate(grid["frames"]):
                seen_outer = set()
                for y, row in enumerate(frame["cels"]["body"]):
                    for x, k in enumerate(row):
                        if k == ".":
                            continue
                        d = math.hypot(x - c, y - c)
                        if r <= d < r + 1:
                            seen_outer.add(by_key[k])
                        elif i < 2 and r - 1 <= d < r:
                            self.assertIn(by_key[k], {"flame_02", "brass_05"}, f"r{r} march {i}")
                        elif i >= 2 and r - 1 <= d < r:
                            self.assertEqual(by_key[k], "flame_04" if i == 2 else "flame_05", f"r{r} hot {i}")
                self.assertEqual(seen_outer, {"flame_00"}, f"r{r} frame {i}: outer row")

    def test_the_ring_pins_moved_and_nothing_else_in_the_fixtures_did(self) -> None:
        for name in ("dfx_ring_r48", "dfx_ring_r64"):
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], f"{name}: {validators.summary(report)}")


class HazardSheetsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.built = fx.assets()
        cls.pal = fx.palette()

    def grid(self, name: str) -> dict:
        return load(PROPS / f"{name}.grid.json")

    def test_canvases_pivots_and_tags_follow_the_spec(self) -> None:
        for name, (size, pivot, tags) in SHEETS.items():
            grid = self.grid(name)
            self.assertEqual(tuple(grid["canvas"]), size, name)
            self.assertEqual(tuple(grid["pivot"]), pivot, name)
            got = {t["name"]: t["to"] - t["from"] + 1 for t in grid["tags"]}
            self.assertEqual(got, tags, name)
            self.assertEqual(sorted(SHEETS), sorted(hz.HAZARD_SHEETS))

    def test_the_splash_key_frame_is_the_key_verbatim_and_the_timing_is_16_ticks(self) -> None:
        grid = self.grid("dfx_acid_splash")
        key = load(KEYS / "acid_splash_key.grid.json")
        self.assertEqual(grid["frames"][2]["cels"], key["frames"][0]["cels"])
        self.assertEqual(grid["palette"], key["palette"])
        man = load(MANIFESTS / "f01-dfx-acid-splash.json")
        ticks = [f["ticks"] for f in man["prop_kit"]["frames"]]
        self.assertEqual(ticks, SPLASH_TICKS)
        self.assertEqual(sum(ticks), 16)
        self.assertEqual(man["prop_kit"]["playback"], {"splash": "once"})
        self.assertEqual([f["duration_ms"] for f in grid["frames"]], [round(t * 1000 / 60) for t in SPLASH_TICKS])
        # Frames 0..4 are all different and every drop is a chunk (no isolated pixel).
        self.assertEqual(len({json.dumps(f["cels"]) for f in grid["frames"]}), 5)

    def test_the_splash_uses_only_the_keys_ten_entries_and_stays_on_its_canvas(self) -> None:
        grid = self.grid("dfx_acid_splash")
        self.assertEqual(names_of(grid) - {n for n, _ in hz.ACID_RAMP}, set())
        for i, f in enumerate(grid["frames"]):
            rows = f["cels"]["body"]
            self.assertEqual((len(rows), {len(r) for r in rows}), (24, {32}), f"frame {i}")
            for y in range(19, 24):  # rows 19..23 never extend past the key's rim width
                cols = [x for x, c in enumerate(rows[y]) if c != "."]
                key_cols = [x for x, c in enumerate(load(KEYS / "acid_splash_key.grid.json")["frames"][0]["cels"]["body"][y]) if c != "."]
                if cols:
                    self.assertGreaterEqual(min(cols), min(key_cols), f"frame {i} row {y}")
                    self.assertLessEqual(max(cols), max(key_cols), f"frame {i} row {y}")
        # The sheets stay tall in frames 1 and 3 (the gate's note): the crown is not shrunk.
        def top(frame: int) -> int:
            rows = grid["frames"][frame]["cels"]["body"]
            return min(y for y, r in enumerate(rows) if r.strip("."))
        self.assertLessEqual(top(1), 10)
        self.assertLessEqual(top(3), 4)
        self.assertLessEqual(top(2), 1)

    def test_the_corrosion_face_is_its_key_and_the_fixtures_pin_the_keys(self) -> None:
        face = self.grid("dfx_corr_face")
        key = load(KEYS / "corrosion_key.grid.json")
        self.assertEqual(face["frames"][0]["cels"], key["frames"][0]["cels"])
        self.assertEqual(face["palette"], key["palette"])
        for name, key_name, frame in (("f01-dfx-corr-face", "corrosion_key.grid.json", "0"),
                                      ("f01-dfx-acid-splash", "acid_splash_key.grid.json", "2")):
            fixture = load(FIXTURES / f"{name}_keys.json")
            k = load(KEYS / key_name)
            self.assertEqual(fixture["keys"][frame]["sha256"],
                             validators.frame_digest(k["frames"][0]["cels"], ["body"]), name)

    def test_the_dry_rule_no_corrosion_pixel_is_lit_acid(self) -> None:
        for name in ("dfx_corr_face", "dfx_corr_top", "dfx_corr_wall"):
            used = names_of(self.grid(name))
            self.assertLessEqual(used, hz.DRY, name)
            self.assertFalse(used & LIT_ACID, name)
        # The floor-top stain and the wall trail are never lit at all (no fume_lit crust).
        self.assertNotIn("fume_lit", names_of(self.grid("dfx_corr_top")))
        self.assertNotIn("fume_lit", names_of(self.grid("dfx_corr_wall")))

    def test_the_corrosion_decals_follow_their_language(self) -> None:
        top = self.grid("dfx_corr_top")["frames"][0]["cels"]["body"]
        self.assertTrue(all(row[0] == "." and row[-1] == "." for row in top), "no outline at the ends")
        outline = {y: [x for x, c in enumerate(row) if c == "#"] for y, row in enumerate(top)}
        self.assertTrue(all(10 <= x <= 21 for xs in outline.values() for x in xs), "darkest pixels in the middle")
        wall = self.grid("dfx_corr_wall")["frames"][0]["cels"]["body"]
        rows = [y for y, r in enumerate(wall) if r.strip(".")]
        self.assertLessEqual(max(rows) - min(rows) + 1, 20, "at most 20 px up the backwall")
        self.assertEqual(max(rows), 23, "it starts on the floor line")
        face = self.grid("dfx_corr_face")["frames"][0]["cels"]["body"]
        self.assertTrue(face[0].strip("."), "the stain band is on the first solid row")

    def test_the_rot_marks_use_only_rot_01_entries(self) -> None:
        rot = {p["name"]: p["rgba"] for p in load(ROT)["palette"]}
        rot_keys = {p["name"]: p["key"] for p in load(ROT)["palette"]}
        for name in hz.ROT_FAMILY:
            grid = self.grid(name)
            for p in grid["palette"]:
                self.assertEqual(rot.get(p["name"]), p["rgba"], f"{name} {p['name']}")
                self.assertEqual(rot_keys[p["name"]], p["key"], "the ROT-01 key characters")
            self.assertFalse(names_of(grid) & ROT_FORBIDDEN, name)
        # rim_cold is only the single wet glint on a drip bead.
        for name in ("dfx_rot_nest", "dfx_rot_trail"):
            self.assertNotIn("rim_cold", names_of(self.grid(name)), name)
        drip = self.grid("dfx_rot_drip")
        glints = [sum(row.count(rot_keys["rim_cold"]) for row in f["cels"]["body"]) for f in drip["frames"]]
        self.assertTrue(all(g <= 1 for g in glints), glints)

    def test_the_nest_and_the_drip_follow_the_spec(self) -> None:
        nest = self.grid("dfx_rot_nest")["frames"][0]["cels"]["body"]
        widths = [sum(1 for c in r if c != ".") for r in nest]
        self.assertTrue(20 <= max(widths) <= 26, widths)
        self.assertEqual(widths[0], max(widths), "it is widest on the ceiling line")
        self.assertTrue(nest[0].strip("."), "row 0 is on the underside line")
        # Ceiling run: the strand grows 2 (splat), 4, 9 and 13 rows; the drop loop is bead, chunk, hang.
        drip = self.grid("dfx_rot_drip")
        def lowest(frame: int) -> int:
            rows = drip["frames"][frame]["cels"]["body"]
            return max(y for y, r in enumerate(rows) if r.strip("."))
        self.assertEqual([lowest(i) for i in range(4)], [1, 3, 8, 12])
        self.assertEqual([lowest(i) for i in range(4, 7)], [14, 15, 12])
        self.assertEqual([lowest(i) for i in range(7, 11)], [2, 3, 8, 12])
        self.assertEqual([lowest(i) for i in range(11, 14)], [14, 15, 12])
        man = load(MANIFESTS / "f01-dfx-rot-drip.json")
        ticks = {f["tag"]: f["ticks"] for f in man["prop_kit"]["frames"]}
        self.assertEqual(ticks, {"drip_run_c": 6, "drip_drop_c": 20, "drip_run_w": 6, "drip_drop_w": 20})
        self.assertEqual(man["prop_kit"]["playback"]["drip_run_c"], "once")
        self.assertEqual(man["prop_kit"]["playback"]["drip_drop_c"], "loop")
        pivots = {f["tag"]: f["pivot"] for f in man["prop_kit"]["frames"]}
        self.assertEqual(pivots["drip_run_c"], [4, 0])
        self.assertEqual(pivots["drip_run_w"], [0, 0], "the wall pivot")

    def test_the_lone_pixel_limit_and_the_orphan_budget_per_element(self) -> None:
        """Spec 9: no isolated pixel anywhere; the orphan limit is the largest measured frame
        plus 2, at most 32 (the splash key measures 26, the corrosion key 13)."""
        for name in hz.HAZARD_SHEETS:
            grid = self.grid(name)
            rep = pg.lint_grid(grid)
            self.assertTrue(all(n == 0 for n in rep["isolated_count_per_frame"]), f"{name}: isolated")
            cfg = load(PROPS / f"{name}.pixelart.json")
            limit = cfg["rules"]["max_orphans_per_frame"]
            self.assertEqual(limit, min(max(rep["orphan_count_per_frame"]) + 2, 32), name)
            self.assertEqual(cfg["rules"]["max_isolated_per_frame"], 0, name)
            self.assertEqual(cfg["rules"]["max_colors"], 63, name)
            self.assertLessEqual(len(grid["palette"]), 63, name)
        self.assertEqual(max(pg.lint_grid(self.grid("dfx_acid_splash"))["orphan_count_per_frame"]), 26)
        self.assertEqual(pg.lint_grid(self.grid("dfx_corr_face"))["orphan_count_per_frame"], [13])

    def test_every_config_passes_the_toolkit_validators(self) -> None:
        for name in hz.HAZARD_SHEETS:
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], f"{name}: {validators.summary(report)}")

    def test_the_frozen_keys_are_the_two_keys_only(self) -> None:
        frozen = {n: load(PROPS / f"{n}.pixelart.json")["rules"].get("frozen_keys") for n in hz.HAZARD_SHEETS}
        self.assertEqual(frozen["dfx_acid_splash"]["frames"], [2])
        self.assertEqual(frozen["dfx_corr_face"]["frames"], [0])
        for n in set(hz.HAZARD_SHEETS) - {"dfx_acid_splash", "dfx_corr_face"}:
            self.assertIsNone(frozen[n], n)

    def test_the_manifests_name_their_inputs_and_pin_every_frame(self) -> None:
        for name in hz.HAZARD_SHEETS:
            man = load(MANIFESTS / f"{fx.asset_stem(name)}.json")
            grid = self.grid(name)
            self.assertEqual(man["review_status"], "prototype_fixture_unapproved")
            self.assertEqual(man["grid_source"], f"assets/source/props/floor01/{name}.grid.json")
            self.assertIn("docs/02-art/area-hazards-01b-spec.md", man["provenance"]["input_reference_ids"])
            self.assertTrue((ROOT / man["source_path"]).is_file(), f"{name}: the .ase source is committed")
            for f, g in zip(man["prop_kit"]["frames"], grid["frames"]):
                self.assertEqual(f["sha256"], validators.frame_digest(g["cels"], ["body"]), name)
        self.assertEqual(load(MANIFESTS / "f01-dfx-acid-splash.json")["palette_id"], "hz01b-acid-teal-v1")
        self.assertEqual(load(MANIFESTS / "f01-dfx-rot-drip.json")["palette_id"], "rot-sticker-floor01-v1")


KIT = ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json"
SCENE = ROOT / "docs/evidence/area-hazards-01b/scene"
# The window captures show layout rows 292.. at 1x (the camera row printed as `cam=0,292`), so the
# walking line (layout row 580) is screen row 288.
CAMERA_ROW, WALK_LINE_ROW = 292, 580


class DeckEdgeTest(unittest.TestCase):
    """HZB-AD17: the wall trail's pivot row is the last backwall row above the deck's back edge,
    `floor_y - DECK_TOP_PX - 1`, with `DECK_TOP_PX` taken from the catacomb kit's deep floor."""

    def test_the_deck_depth_is_the_kits_and_the_captured_deck_edge_agrees(self) -> None:
        base = load(KIT)["walk_line"]["base_floor"]
        deck_top = base["walk_line_y"] - base["floor_back_y"]
        self.assertEqual(deck_top, 20)
        walk_row = WALK_LINE_ROW - CAMERA_ROW
        edge_row = walk_row - deck_top  # the deck's dark back-edge outline row
        from PIL import Image

        def lum(im: "Image.Image", x: int, y: int) -> int:
            return sum(im.getpixel((x, y))[:3]) // 3

        # Columns clear of actors, decals and props in the committed window stills: the deck's
        # back edge is the dark row that the brighter deck top follows.
        for still, columns in (("r4_corrosion_drained", (120, 150, 400)), ("r6_corrosion_drained", (500, 560))):
            im = Image.open(SCENE / f"{still}_640x360.png").convert("RGB")
            for x in columns:
                edges = [y for y in range(262, 271) if lum(im, x, y) <= 40 and lum(im, x, y + 1) >= lum(im, x, y) + 25]
                self.assertEqual(edges, [edge_row], f"{still} column {x}")
        # The trail's pivot row is the row just above that edge: layout row floor_y - 20 - 1.
        self.assertEqual(WALK_LINE_ROW - deck_top - 1 - CAMERA_ROW, edge_row - 1)


if __name__ == "__main__":
    unittest.main()
