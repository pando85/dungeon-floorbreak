#!/usr/bin/env python3
"""WORK-007 (phase 2): material tiles, extension contract, sample scene and evidence frames."""
from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_work007_materials as aw  # noqa: E402
import env_kit as ek  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
import work007_sample as ws  # noqa: E402
from PIL import Image, ImageChops  # noqa: E402
from pixelart.validators import frame_digest  # noqa: E402

GRID = ROOT / "assets/source/terrain/floor01/materials.grid.json"
CONTRACT = "assets/source/terrain/floor01/floor01_materials_kit.json"
MANIFEST = ROOT / "assets/manifests/floor01_materials.json"
KEYS = ROOT / "tests/fixtures/work-007-materials_keys.json"
EVIDENCE = ROOT / "docs/evidence/work-007"
EXPORTED = (ROOT / "generated/art/floor01_materials.png").is_file() and (
    ROOT / "generated/art/carl_book1_h52_locomotion.png").is_file()
WALK_LINE = 308
CARL_HEIGHT = 52
CARL_WIDTH = 24      # generous width of the 52 px Carl silhouette (idle 23 px)
JUMP_APEX = 92
REMNANT_OPAQUE_LIMIT = 0.50  # review ruling Q1 (PR #163): measured 47.7, 43.4, 35.2 and 25.0 %


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def frame_rows(grid: dict, n: int) -> list[str]:
    return grid["frames"][n]["cels"]["main"]


def names_by_key(grid: dict) -> dict[str, str]:
    return {e["key"]: e["name"] for e in grid["palette"]}


class GridTests(unittest.TestCase):
    """Frozen keys, derived frames, palette and lint rules (no export needed)."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.grid = load(GRID)
        cls.keys = load(KEYS)
        cls.kit = load(ROOT / aw.KIT.relative_to(ROOT))
        cls.names = names_by_key(cls.grid)

    def test_frozen_keys_are_byte_identical(self) -> None:
        # frames 0 to 8 are the Opus keys, 9 to 21 the derived tiles frozen after the review (F-4)
        self.assertEqual(sorted(self.keys["keys"], key=int), [str(i) for i in range(22)])
        for n, entry in self.keys["keys"].items():
            with self.subTest(frame=n):
                self.assertEqual(frame_digest(self.grid["frames"][int(n)]["cels"], ["main"]),
                                 entry["sha256"], entry["tile"])

    def test_frame_table_matches_tile_table(self) -> None:
        self.assertEqual(len(self.grid["frames"]), 22)
        self.assertEqual([t[0] for t in aw.TILES], list(range(22)))
        self.assertEqual(self.grid["tags"], aw.TAGS)
        for n, entry in self.keys["keys"].items():
            self.assertEqual(entry["tile"], aw.TILES[int(n)][1])

    def test_derived_frames_and_contract_are_current(self) -> None:
        self.assertEqual(aw.main(["check"]), 0)

    def test_palette_is_a_subset_of_the_kit_and_within_the_atlas_limit(self) -> None:
        kit_palette = {e["name"]: e["rgba"] for e in self.kit["palette"]}
        self.assertEqual(self.kit["palette_id"], "floor01-catacomb-v2")
        for entry in self.grid["palette"]:
            self.assertEqual(kit_palette.get(entry["name"]), entry["rgba"], entry["name"])
        self.assertLessEqual(len(self.grid["palette"]), 63)
        self.assertEqual(len({e["key"] for e in self.grid["palette"]}), len(self.grid["palette"]))

    def test_first_28_palette_entries_are_the_untouched_key_palette(self) -> None:
        self.assertEqual(len(self.grid["palette"]), 42)
        used_by_keys = {ch for n in range(9) for row in frame_rows(self.grid, n) for ch in row} - {"."}
        self.assertEqual(len(used_by_keys), 28)
        self.assertLessEqual(used_by_keys, {e["key"] for e in self.grid["palette"][:28]})

    def test_no_isolated_pixels_and_few_orphans(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(report["isolated_count_per_frame"], [0] * 22)
        for n, count in enumerate(report["orphan_count_per_frame"]):
            with self.subTest(frame=n):
                self.assertLessEqual(count, 20)

    def test_iron_intact_and_damaged_share_the_full_opaque_outline(self) -> None:
        for n in (0, 1):
            self.assertTrue(all("." not in row for row in frame_rows(self.grid, n)), n)
            self.assertEqual(len(frame_rows(self.grid, n)), 16)
        for dim, lit in ((10, 16), (11, 17)):
            for n in (dim, lit):
                self.assertTrue(all("." not in row for row in frame_rows(self.grid, n)), n)

    def test_broken_remnants_are_at_most_50_percent_opaque(self) -> None:
        for tile in aw.TILES:
            frame, tid, state = tile[0], tile[1], tile[3]
            opaque = sum(ch != "." for row in frame_rows(self.grid, frame) for ch in row)
            if state in aw.BROKEN_STATES:
                self.assertLessEqual(opaque / 256, REMNANT_OPAQUE_LIMIT, tid)
                self.assertGreater(opaque, 0, tid)
            else:
                self.assertEqual(opaque, 256, tid)

    def test_tone_variants_follow_the_ramp_rule(self) -> None:
        table = aw.ramps(self.kit["palette"])
        accents = {e["name"] for e in self.grid["palette"] if e.get("accent")}
        for offset, step in ((10, -1), (16, +1)):
            for i, src in enumerate(aw.SHIFTED):
                a = frame_rows(self.grid, src)
                b = frame_rows(self.grid, offset + i)
                for ra, rb in zip(a, b):
                    for ca, cb in zip(ra, rb):
                        if ca == ".":
                            self.assertEqual(cb, ".")
                            continue
                        na, nb = self.names[ca], self.names[cb]
                        if na in accents:
                            self.assertIn(nb, (na, "gal_warm_03"))
                            if nb != na:
                                self.assertEqual((step, na), (1, "gal_10"))
                        else:
                            self.assertEqual(nb, aw.tone_name(na, step, table, accents))
                            # review ruling Q3: a shift never lands on an accent of its own ramp
                            self.assertNotIn(nb, accents)

    def test_lit_glass_warms_only_the_top_pixel_of_each_glint_streak(self) -> None:
        by_name = {e["name"]: e["key"] for e in self.grid["palette"]}
        warm, glint = by_name["gal_warm_03"], by_name["gal_10"]
        # streak tops: the diagonal glint under the lattice crossing, the corner glint that
        # continues into the neighbouring tile (col 15 row 13 with col 0 row 12), the cracked glint
        expected = {2: {(8, 4), (0, 12)}, 3: {(8, 4), (0, 12), (9, 7)}}
        for src, lit in ((2, 18), (3, 19)):
            rows = frame_rows(self.grid, lit)
            warmed = {(x, y) for y, row in enumerate(rows) for x, ch in enumerate(row) if ch == warm}
            self.assertEqual(warmed, expected[src], lit)
            # every other glint pixel of the mid key stays gal_10
            for y, row in enumerate(frame_rows(self.grid, src)):
                for x, ch in enumerate(row):
                    if ch == glint and (x, y) not in warmed:
                        self.assertEqual(rows[y][x], glint, (lit, x, y))

    def test_lit_iron_bevel_stays_below_the_rivet_accent(self) -> None:
        by_name = {e["name"]: e["key"] for e in self.grid["palette"]}
        for src, lit in ((0, 16), (1, 17)):
            # the rivet heads (iron_06) are the only iron_06 pixels: the bevel no longer joins them
            count = lambda n: sum(ch == by_name["iron_06"] for row in frame_rows(self.grid, n) for ch in row)  # noqa: E731
            self.assertEqual(count(lit), count(src), lit)
            for ra, rb in zip(frame_rows(self.grid, src), frame_rows(self.grid, lit)):
                for ca, cb in zip(ra, rb):
                    if self.names[ca] == "iron_05":
                        self.assertEqual(self.names[cb], "iron_05")

    def test_broken_r_is_redrawn_not_mirrored(self) -> None:
        r = frame_rows(self.grid, 9)
        left = [row[::-1] for row in frame_rows(self.grid, 8)]
        self.assertNotEqual(r, left)
        # stubs at the right jamb: nothing left of column 8, nails (A over z) at x 14 on all stubs
        for row in r:
            self.assertEqual(row[:8], "." * 8)
        nails = [y for y, row in enumerate(r) if row[14] == "A"]
        self.assertEqual(len(nails), 3)
        lengths = [sum(ch != "." for ch in r[y]) for y in (0, 5, 11)]
        self.assertEqual(lengths, [5, 3, 7])  # stub lengths of the spec (the top row of each board)

    def test_glass_lattice_phase_continues_across_tile_edges(self) -> None:
        formula = {(x, y) for y in range(16) for x in range(16)
                   if (x + y - 8) % 16 in (0, 15) or (x - y - 8) % 16 in (0, 15)}
        for frame in (2, 3, 12, 13, 18, 19):
            came = {(x, y) for y, row in enumerate(frame_rows(self.grid, frame))
                    for x, ch in enumerate(row) if self.names[ch].startswith("iron_")}
            self.assertEqual(came, formula, frame)
        # 3x3 tiling: the came of the tiled block follows the same formula on the 48 x 48 canvas, so the
        # lattice phase continues across every tile edge
        for frame in (2, 18):
            tile = {(x, y) for y, row in enumerate(frame_rows(self.grid, frame))
                    for x, ch in enumerate(row) if self.names[ch].startswith("iron_")}
            tiled = {(x + 16 * i, y + 16 * j) for (x, y) in tile for i in range(3) for j in range(3)}
            expected = {(x, y) for y in range(48) for x in range(48)
                        if (x + y - 8) % 16 in (0, 15) or (x - y - 8) % 16 in (0, 15)}
            self.assertEqual(tiled, expected, frame)

    def test_board_rows_continue_across_tile_edges(self) -> None:
        for frame in (6, 7, 14, 15, 20, 21):
            rows = frame_rows(self.grid, frame)
            gap_key = rows[4][0]
            self.assertTrue(self.names[gap_key].startswith("wood_"), frame)
            for y in (4, 10, 15):
                self.assertEqual(set(rows[y]), {gap_key}, (frame, y))
            for y in (0, 5, 11):  # the lit top row of each board runs edge to edge
                self.assertNotIn(".", rows[y])
        # vertical tiling: board 3 (rows 11..14) is followed by the gap row 15 and board 1 (row 0 lit top)
        rows = frame_rows(self.grid, 6)
        self.assertEqual(set(rows[15]), {rows[4][0]})
        self.assertNotEqual(set(rows[0]), {rows[4][0]})


@unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
class ContractTests(unittest.TestCase):
    """The extension contract validates against the exported atlas and merges onto ENV-01."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.kit = ek.load_kit(CONTRACT)

    def test_contract_validates(self) -> None:
        ek.validate_contract(self.kit)

    def test_manifest_and_contract_agree(self) -> None:
        manifest = load(MANIFEST)
        self.assertEqual(manifest["type"], ek.MATERIAL_TYPE)
        self.assertEqual(manifest["review_status"], "prototype_fixture_unapproved")
        self.assertEqual(manifest["palette_id"], "floor01-catacomb-v2")
        self.assertEqual(manifest["kit_contract"], CONTRACT)
        self.assertEqual(manifest["grid_source"], "assets/source/terrain/floor01/materials.grid.json")
        self.assertEqual(manifest["animation_tags"], [t["name"] for t in aw.TAGS])

    def test_material_tiles_carry_material_state_and_collider(self) -> None:
        tiles = {t["id"]: t for t in self.kit.contract["material_tiles"]}
        self.assertEqual(len(tiles), 22)
        self.assertEqual({t["material"] for t in tiles.values()}, {"reinforced", "glass", "wood"})
        for frame, tid, material, state, tone, source in aw.TILES:
            t = tiles[tid]
            self.assertEqual((t["frame_index"], t["material"], t["state"], t["tone"]),
                             (frame, material, state, tone))
            if state in aw.BROKEN_STATES:
                self.assertFalse(t["collidable"])
                self.assertNotIn("collider", t)
            else:
                self.assertEqual(t["collider"], [0, 0, 16, 16])
        groups = self.kit.contract["state_groups"]
        self.assertEqual(set(groups), {"iron_plate", "glass_leaded", "timber_boards"})

    def test_base_kit_is_unchanged_by_the_extension(self) -> None:
        base = ek.load_json(aw.KIT)
        self.assertNotIn("material_tiles", base)
        self.assertEqual(self.kit.contract["palette"], base["palette"])
        self.assertEqual(len(self.kit.contract["tiles"]), len(base["tiles"]))

    def test_validator_rejects_a_collidable_remnant_and_a_wrong_rect(self) -> None:
        def broken(mutate):
            data = copy.deepcopy(self.kit.contract)
            mutate(next(t for t in data["material_tiles"] if t["id"] == "timber_boards_broken_l"))
            kit = ek.Kit(data, self.kit.atlases, self.kit.path)
            with self.assertRaises(ek.KitError):
                ek._check_material_tiles(kit)
        broken(lambda t: t.update(collidable=True, collider=[0, 0, 16, 16]))
        broken(lambda t: t.update(rect=[0, 0, 16, 16]))
        broken(lambda t: t.update(material="stone"))

    def test_atlas_colours_within_the_limit(self) -> None:
        image = Image.open(ROOT / "generated/art/floor01_materials.png").convert("RGBA")
        opaque = {c[:3] for _n, c in image.getcolors(1 << 16) if c[3] == 255}
        self.assertLessEqual(len(opaque), 63)
        self.assertEqual(len(opaque), 42)


class SceneTests(unittest.TestCase):
    """Geometry and state rules of the sample scene file (no export needed)."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.scene = ws.load_scene()
        cls.grid = load(GRID)
        cls.names = names_by_key(cls.grid)
        cls.tiles = {t[1]: t for t in aw.TILES}

    def cells(self, state: str) -> list[dict]:
        return self.scene["states"][state]["cells"]

    def opaque_extent(self, tile_id: str) -> tuple[int, int, int, int]:
        rows = frame_rows(self.grid, self.tiles[tile_id][0])
        xs = [x for row in rows for x, ch in enumerate(row) if ch != "."]
        ys = [y for y, row in enumerate(rows) for ch in row if ch != "."]
        return min(xs), min(ys), max(xs), max(ys)

    def solid_rects(self, state: str) -> list[tuple[int, int, int, int]]:
        rects = []
        for c in self.cells(state):
            if c["tile"] in self.tiles and self.tiles[c["tile"]][3] in aw.BROKEN_STATES:
                x0, y0, x1, y1 = self.opaque_extent(c["tile"])
                rects.append((c["x"] + x0, c["y"] + y0, c["x"] + x1 + 1, c["y"] + y1 + 1))
            else:
                crop = c.get("crop") or [0, 0, 16, 16]
                rects.append((c["x"], c["y"], c["x"] + crop[2], c["y"] + crop[3]))
        return rects

    def test_scene_format_and_views(self) -> None:
        self.assertEqual(self.scene["format"], ws.SCENE_FORMAT)
        self.assertEqual(self.scene["pixel_size"], [640, 400])
        self.assertEqual(self.scene["views"], {"640x360": [0, 20, 640, 360], "640x400": [0, 0, 640, 400]})
        self.assertEqual(set(self.scene["states"]), {"F0", "F1", "F2"})
        self.assertEqual(self.scene["parallax"][:4], "none")

    @unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
    def test_scene_file_is_what_the_generator_writes(self) -> None:
        base = ek.load_kit("assets/source/terrain/floor01/floor01_catacomb_kit.json", load_grids=False)
        self.assertEqual(json.loads(json.dumps(ws.build_scene(base.tiles))), self.scene)

    def test_base_walk_line_comes_from_the_tile_layers(self) -> None:
        self.assertEqual(self.scene["base_walk_line_y"], WALK_LINE)
        if not EXPORTED:
            return
        kit = ek.load_kit("assets/source/terrain/floor01/floor01_catacomb_kit.json")
        cols, rows = self.scene["grid_size"]
        tops = ek.collider_tops(kit, self.scene["tile_layers"]["collision"], cols, rows)
        self.assertTrue(all(t == WALK_LINE for t in tops[2:]), set(tops[2:]))

    def test_perch_height_and_clear_heights(self) -> None:
        geo = self.scene["geometry"]
        perch = [c for c in self.cells("F0") if c["tile"].startswith("iron_plate") and c["y"] == 228]
        self.assertEqual(len(perch), 16)
        self.assertEqual(min(c["x"] for c in perch), 272)
        self.assertEqual(max(c["x"] for c in perch) + 16, 528)
        self.assertEqual(WALK_LINE - 228, 80)                    # CF02-Q45: top 80 px above the walking line
        self.assertEqual(geo["perch"]["height_above_walk_line"], 80)
        self.assertEqual(228 + 16, 244)                          # 16 px slab
        lane_clear = WALK_LINE - 244
        self.assertEqual(lane_clear, 64)                         # under the slab, CF02-F: 68
        self.assertGreaterEqual(lane_clear - CARL_HEIGHT, 12)    # Carl clears by 12
        upper = [c for c in self.cells("F0") if c["tile"].startswith("glass_leaded")]
        lower = [c for c in self.cells("F0") if c["tile"].startswith("timber_boards")]
        self.assertEqual((min(c["y"] for c in upper), max(c["y"] for c in upper) + 16), (164, 228))
        self.assertEqual((min(c["y"] for c in lower), max(c["y"] for c in lower) + 16), (244, WALK_LINE))
        self.assertEqual(len(upper), 12)
        self.assertEqual(len(lower), 12)
        self.assertGreaterEqual(228 - 164 - CARL_HEIGHT, 12)     # 64 clear above the perch
        self.assertLess(80, JUMP_APEX)                           # the jump onto the perch fits the apex

    def test_lower_route_is_open_after_the_breach(self) -> None:
        f0 = self.solid_rects("F0")
        f2 = self.solid_rects("F2")

        def blocks(rects, x0, x1, y0, y1):
            return [r for r in rects if r[0] < x1 and x0 < r[2] and r[1] < y1 and y0 < r[3]]
        # F0: every column of the lower opening is walled; F2: a passage at least Carl wide is free
        for x in range(384, 432):
            self.assertTrue(blocks(f0, x, x + 1, 244, WALK_LINE), x)
        free = [x for x in range(384, 432) if not blocks(f2, x, x + 1, 244, WALK_LINE)]
        self.assertGreaterEqual(len(free), CARL_WIDTH)
        self.assertEqual(free, list(range(free[0], free[-1] + 1)))
        # upper route: the screen has fallen out of the whole 64 px opening between rows 1 and 4
        free_up = [x for x in range(384, 432) if not blocks(f2, x, x + 1, 180, 228 - 16)]
        self.assertEqual(len(free_up), 48)
        for x in range(384, 432):
            self.assertTrue(blocks(f0, x, x + 1, 164, 228), x)

    def test_breached_cells_are_remnants_or_empty(self) -> None:
        for c in self.cells("F2"):
            if c["tile"].startswith(("glass_leaded", "timber_boards")):
                self.assertIn(self.tiles[c["tile"]][3], aw.BROKEN_STATES, c)
        self.assertEqual(sum(c["tile"].startswith("timber_boards") for c in self.cells("F2")), 8)
        self.assertEqual(sum(c["tile"].startswith("glass_leaded") for c in self.cells("F2")), 6)

    def test_stone_and_iron_cells_do_not_change_between_f0_and_f2(self) -> None:
        def fixed(state: str) -> dict:
            out = {}
            for c in self.cells(state):
                if c["tile"].startswith(("wall_", "iron_plate")):
                    out[(c["x"], c["y"], tuple(c.get("crop") or ()))] = c["tile"].split("_")[0]
            return out
        self.assertEqual(fixed("F0"), fixed("F1"))
        self.assertEqual(fixed("F0"), fixed("F2"))
        self.assertEqual(sum(1 for c in self.cells("F2") if c["tile"].startswith("iron_plate")), 19)
        self.assertEqual(sum(1 for c in self.cells("F2") if c["tile"].startswith("wall_")), 24)
        for state, damaged in (("F0", 0), ("F1", 3), ("F2", 3)):
            self.assertEqual(sum(c["tile"] == "iron_plate_damaged" for c in self.cells(state)), damaged)
        for c in self.cells("F1"):
            if c["tile"] == "iron_plate_damaged":
                self.assertIn(c["x"], (384, 400, 416))
                self.assertEqual(c["y"], 228)

    def test_state_progression_f1(self) -> None:
        f1 = {(c["x"], c["y"]): c["tile"] for c in self.cells("F1")}
        for x in (384, 400, 416):
            self.assertEqual([f1[(x, y)] for y in (164, 180, 196, 212)],
                             ["glass_leaded_intact"] * 2 + ["glass_leaded_cracked"] * 2)
            self.assertEqual([f1[(x, y)] for y in (244, 260, 276, 292)],
                             ["timber_boards_intact"] * 2 + ["timber_boards_cracked"] * 2)

    def test_actors_and_effects(self) -> None:
        f0, f1, f2 = (self.scene["states"][s]["actors"] for s in ("F0", "F1", "F2"))
        by_id = lambda lst: {a["id"]: a for a in lst}  # noqa: E731
        self.assertEqual(by_id(f0)["carl"]["pivot"], [256, 307])
        self.assertEqual(by_id(f0)["bomb_flight"]["pivot"], [400, 300])
        self.assertEqual((by_id(f1)["carl"]["atlas_frame"], by_id(f1)["carl"]["pivot"]), (8, [176, 307]))
        self.assertEqual(by_id(f1)["burst_peak"]["pivot"], [408, 284])
        self.assertEqual(by_id(f1)["burst_peak"]["atlas_frame"], 5)
        self.assertEqual(by_id(f1)["splinter_burst"]["atlas_frame"], 1)
        self.assertEqual((by_id(f2)["carl"]["pivot"], by_id(f2)["carl"]["facing"]), ([336, 227], "right"))
        self.assertEqual(by_id(f2)["burst_smoke"]["atlas_frame"], 10)
        self.assertEqual(by_id(f1)["burst_peak"]["pivot"][0] - by_id(f1)["carl"]["pivot"][0], 232)

    def test_lighting_profile_is_the_unchanged_env01_profile(self) -> None:
        light = ek.load_json(self.scene["lighting_profile_from"])["lighting"]
        self.assertEqual(light["profile_id"], "floor01_torchlit_a1")
        torches = self.scene["torches"]
        self.assertEqual([t["x"] + 8 for t in torches], [88, 560])
        self.assertEqual(560 - 88, 472)
        self.assertEqual([t["pool"] for t in torches], ["torch_pool_a", "torch_pool_b_m"])


@unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
class FrameTests(unittest.TestCase):
    """Evidence frames: sizes, exact nearest-neighbour upscales and reproducibility."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.scene = ws.load_scene()

    def frame(self, state: str, kind: str, view: str) -> Image.Image:
        label = self.scene["states"][state]["label"]
        path = EVIDENCE / f"work007_{state.lower()}_{label}_{kind}_{view}.png"
        self.assertTrue(path.is_file(), path)
        return Image.open(path).convert("RGB")

    def test_all_frames_exist_with_exact_sizes(self) -> None:
        for state in ws.STATES:
            for view, (w, h) in (("640x360", (640, 360)), ("640x400", (640, 400))):
                self.assertEqual(self.frame(state, "unlit_1x", view).size, (w, h))
                self.assertEqual(self.frame(state, "lit_1x", view).size, (w, h))
                self.assertEqual(self.frame(state, "lit_2x", view).size, (w * 2, h * 2))
            self.assertEqual(self.frame(state, "lit_3x", "640x360").size, (1920, 1080))

    def test_2x_and_3x_are_nearest_neighbour_copies_of_the_1x_frame(self) -> None:
        for state in ws.STATES:
            for view in ("640x360", "640x400"):
                one = self.frame(state, "lit_1x", view)
                self.assertIsNone(ImageChops.difference(self.frame(state, "lit_2x", view),
                                                        ws.nearest(one, 2)).getbbox(), (state, view))
            one = self.frame(state, "lit_1x", "640x360")
            self.assertIsNone(ImageChops.difference(self.frame(state, "lit_3x", "640x360"),
                                                    ws.nearest(one, 3)).getbbox(), state)

    def test_frames_are_reproducible_from_the_scene_file(self) -> None:
        kit = ws.load_kit()
        view = self.scene["views"]["640x360"]
        for state in ws.STATES:
            unlit = ws.render_state(kit, self.scene, state, view, False)
            self.assertIsNone(ImageChops.difference(unlit, self.frame(state, "unlit_1x", "640x360")).getbbox(),
                              state)
            # The lit pass uses float image maths: allow a small difference across Pillow versions.
            lit = ws.render_state(kit, self.scene, state, view, True)
            diff = ImageChops.difference(lit, self.frame(state, "lit_1x", "640x360")).convert("L")
            off = sum(diff.histogram()[3:])
            self.assertLessEqual(off / (640 * 360), 0.001, state)

    def test_views_are_crops_of_one_layout(self) -> None:
        for state in ws.STATES:
            deck = self.frame(state, "unlit_1x", "640x400").crop((0, 20, 640, 380))
            self.assertIsNone(ImageChops.difference(deck, self.frame(state, "unlit_1x", "640x360")).getbbox())

    def test_staging_vfx_does_not_overlap_carl_in_f1(self) -> None:
        f1 = {a["id"]: a for a in self.scene["states"]["F1"]["actors"]}
        carl_img, carl_pos = ws.place(f1["carl"])
        burst_img, burst_pos = ws.place(f1["burst_peak"])
        carl_box = carl_img.getchannel("A").getbbox()
        burst_box = burst_img.getchannel("A").getbbox()
        carl = (carl_pos[0] + carl_box[0], carl_pos[1] + carl_box[1], carl_pos[0] + carl_box[2], carl_pos[1] + carl_box[3])
        burst = (burst_pos[0] + burst_box[0], burst_pos[1] + burst_box[1], burst_pos[0] + burst_box[2], burst_pos[1] + burst_box[3])
        self.assertTrue(carl[2] <= burst[0] or burst[2] <= carl[0] or carl[3] <= burst[1] or burst[3] <= carl[1])

    def test_sheets_exist(self) -> None:
        for name in ("work007_materials_sheet.png", "work007_animation_sheet.png"):
            self.assertTrue((EVIDENCE / name).is_file(), name)

    def test_checksums_match_the_evidence_files(self) -> None:
        sums = (EVIDENCE / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
        self.assertGreaterEqual(len(sums), 23)
        for line in sums:
            digest, name = line.split("  ", 1)
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)


if __name__ == "__main__":
    unittest.main()
