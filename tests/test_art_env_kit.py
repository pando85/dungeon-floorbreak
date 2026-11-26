"""Tests for the env-kit-v2 contract / layout validator, the kit previews and the
presentation-only lighting pass.

Positive tests run against the committed Floor 1 catacomb kit and its exported
atlases (``bash scripts/export_assets.sh`` must have run; CI does that first).
Negative tests mutate in-memory copies so each rule is proven to fire.
"""
from __future__ import annotations

import contextlib
import copy
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import env_kit as ek  # noqa: E402
import env_kit_light as el  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

CONTRACT = "assets/source/terrain/floor01/floor01_catacomb_kit.json"
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
EXPORTED = (ROOT / "generated/art/floor01_terrain.png").is_file()


@unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
class EnvKitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.base = ek.load_kit(CONTRACT)
        cls.layout = ek.load_json(LAYOUT)

    def kit(self, contract=None, images: dict | None = None) -> ek.Kit:
        atlases = {}
        for name, atlas in self.base.atlases.items():
            entry = dict(atlas)
            entry["manifest"] = copy.deepcopy(atlas["manifest"])
            entry["metadata"] = copy.deepcopy(atlas["metadata"])
            if images and name in images:
                entry["image"] = images[name]
            atlases[name] = entry
        return ek.Kit(copy.deepcopy(contract or self.base.contract), atlases, self.base.path)

    def tile(self, contract: dict, tid: str) -> dict:
        return next(t for t in contract["tiles"] if t["id"] == tid)

    def piece(self, contract: dict, pid: str) -> dict:
        return next(p for p in contract["pieces"] if p["id"] == pid)

    def marker(self, layout: dict, kind: str) -> dict:
        return next(m for m in layout["markers"] if m["kind"] == kind)

    # ---- positive -------------------------------------------------------
    def test_committed_kit_and_layout_are_valid(self) -> None:
        ek.validate_contract(self.base)
        ek.validate_layout(self.base, self.layout)

    def test_cli_validate(self) -> None:
        self.assertEqual(ek.main(["validate", "--contract", CONTRACT, "--layout", LAYOUT]), 0)

    def test_grids_use_kit_palette_subsets_and_ase_is_fresh(self) -> None:
        palette = {e["name"]: e["rgba"] for e in self.base.contract["palette"]}
        self.assertEqual(self.base.contract["format"], "env-kit-v2")
        for name, atlas in self.base.atlases.items():
            grid = pg.load_grid(ROOT / atlas["ref"]["grid"])
            self.assertLessEqual(len(grid["palette"]), 63, name)
            for entry in grid["palette"]:
                self.assertEqual(palette[entry["name"]], entry["rgba"], name)
            ase = ROOT / atlas["manifest"]["source_path"]
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(grid), f"{ase} is stale")

    def test_collision_semantics_and_walk_line(self) -> None:
        for tile in self.base.contract["tiles"]:
            if tile["role"] == "backwall":
                self.assertIs(tile["collidable"], False, tile["id"])
            elif tile.get("deck_back"):
                self.assertIs(tile["collidable"], False, tile["id"])
            else:
                self.assertIs(tile["collidable"], True, tile["id"])
                wl = tile.get("walk_line_y", 0)
                self.assertEqual(tile["collider"], [0, wl, 16, 16 - wl], tile["id"])
        for piece in self.base.contract["pieces"]:
            self.assertIs(piece["collidable"], False, piece["id"])
        cols, rows = self.layout["grid_size"]
        tops = ek.collider_tops(self.base, self.layout["tile_layers"]["collision"], cols, rows)
        self.assertEqual(self.layout["base_walk_line_y"], 308)
        self.assertEqual(tops[0], "wall")
        self.assertEqual(sorted({t for t in tops if isinstance(t, int)}), [276, 292, 308])

    def test_layout_size_markers_segments_and_parallax(self) -> None:
        cols, rows = self.layout["grid_size"]
        self.assertEqual((cols, rows), (128, 25))
        kinds = {m["kind"] for m in self.layout["markers"]}
        self.assertTrue({"spawn", "exit"} <= kinds)
        self.assertEqual([s["grammar"] for s in self.layout["segments"]][:3], ["A", "B", "C"])
        factors = {l["name"]: l["parallax"] for l in self.base.contract["layers"] if l["kind"] == "parallax"}
        self.assertEqual(factors, {"parallax_far": 0.35, "parallax_mid": 0.6})

    def test_fill_bond_is_phase_locked_to_the_world_grid(self) -> None:
        # Platforms read as raised floor: every fill course has its joints at world
        # x = 16 * (row % 2) mod 32 and its tone level set by the absolute row.
        grid = self.layout["tile_layers"]["collision"]
        seen = 0
        for r, row in enumerate(grid):
            for c, tid in enumerate(row):
                tile = self.base.tiles.get(tid) if tid else None
                if not tile or "fill_level" not in tile:
                    continue
                self.assertEqual(tile["fill_level"], 0 if r <= 21 else 1 if r == 22 else 2, (r, c, tid))
                if "panel" not in tile:
                    continue
                joint = int(tile["panel"]["id"].split("_j")[1].split("_")[0])
                start = c - tile["panel"]["cell"][0]
                self.assertEqual((start * 16 + joint) % 32, 16 * (r % 2), (r, c, tid))
                seen += 1
        self.assertGreater(seen, 300)

    def test_bloom_is_limited_to_flame_colours(self) -> None:
        bloom = self.layout["lighting"]["bloom"]
        flame = {e["rgba"][:7] for e in self.base.contract["palette"] if e["ramp"] == "flame"}
        self.assertTrue(flame <= set(bloom["emissive_colours"]))
        copper = {e["rgba"][:7] for e in self.base.contract["palette"] if e["ramp"] in ("copper", "brass")}
        self.assertFalse(copper & set(bloom["emissive_colours"]))

    def test_copper_only_in_segment_c(self) -> None:
        c_seg = next(s for s in self.layout["segments"] if s["grammar"] == "C")
        for item in self.layout["piece_layers"]["prop"]:
            if item["piece"].startswith("pipe_") or item["piece"] == "valve_wheel":
                self.assertTrue(c_seg["x"][0] <= item["x"] < c_seg["x"][1], item)

    def test_render_is_deterministic_and_views_scale_exactly(self) -> None:
        a = ek.render_layout(self.base, self.layout)
        b = ek.render_layout(self.base, self.layout)
        self.assertEqual(a.tobytes(), b.tobytes())
        self.assertEqual(a.size, tuple(self.layout["pixel_size"]))
        with tempfile.TemporaryDirectory() as tmp:
            first = ek.render_all(self.base, self.layout, Path(tmp) / "a", actors=False)
            second = ek.render_all(self.base, self.layout, Path(tmp) / "b", actors=False)
            for p, q in zip(first, second):
                self.assertEqual(p.read_bytes(), q.read_bytes(), p.name)
            one = Image.open(Path(tmp) / "a" / "corridor_abc_640x360.png")
            two = Image.open(Path(tmp) / "a" / "corridor_abc_1280x720_2x.png")
            self.assertEqual(one.size, (640, 360))
            self.assertEqual(two.tobytes(), one.resize((1280, 720), Image.Resampling.NEAREST).tobytes())

    def test_parallax_strip_moves_with_camera_at_its_factor(self) -> None:
        near = ek.render_layers(self.base, self.layout, camera_x=0, names=("parallax_far",))["parallax_far"]
        far = ek.render_layers(self.base, self.layout, camera_x=100, names=("parallax_far",))["parallax_far"]
        # camera +100 px: a 0.35 layer moves 35 px left on screen -> 65 px right in layout space
        y = self.layout["parallax_layers"]["parallax_far"]["y"] + 100
        self.assertEqual(near.crop((200, y, 600, y + 40)).tobytes(), far.crop((265, y, 665, y + 40)).tobytes())

    def test_torch_animation_matches_export(self) -> None:
        torch = self.piece(self.base.contract, "torch_wall")
        meta = self.base.atlases["floor01_torch"]["metadata"]
        self.assertEqual([f["duration"] for f in torch["animation"]["frames"]],
                         [f["duration"] for f in meta["frames"]])
        flicker = next(l for l in torch["lights"] if l["id"] == "flame")["flicker"]
        self.assertEqual(len(flicker), len(meta["frames"]))

    def test_lights_are_gathered_and_floor_pools_sit_on_the_walking_line(self) -> None:
        view = self.layout["views"]["abc_640x360"]
        lights = el.gather_lights(self.base, self.layout, view)
        groups = {l["group"] for l in lights}
        self.assertTrue({"scene", "parallax_far", "parallax_mid"} <= groups)
        pools = [l for l in lights if l["squash_y"] < 1.0]
        self.assertTrue(pools)
        for pool in pools:
            self.assertEqual(pool["y"] + view[1], 308 - 6)

    def test_lit_render_is_deterministic(self) -> None:
        view = [600, 120, 160, 120]
        a = el.render_lit(self.base, self.layout, view, 2)
        b = el.render_lit(self.base, self.layout, view, 2)
        self.assertEqual(a.size, (320, 240))
        self.assertEqual(a.tobytes(), b.tobytes())
        unlit = el.render_lit(self.base, self.layout, view, 1, lit=False)
        self.assertEqual(unlit.tobytes(), ek.render_view(self.base, self.layout, view).convert("RGB").tobytes())

    # ---- negative: contract ----------------------------------------------
    def test_off_palette_pixel_rejected(self) -> None:
        image = self.base.atlases["floor01_props"]["image"].copy()
        image.putpixel((1, 5), (0, 255, 0, 255))
        with self.assertRaisesRegex(ek.KitError, "not in kit palette"):
            ek.validate_contract(self.kit(images={"floor01_props": image}))

    def test_partial_alpha_rejected(self) -> None:
        image = self.base.atlases["floor01_props"]["image"].copy()
        image.putpixel((1, 5), (57, 58, 70, 128))
        with self.assertRaisesRegex(ek.KitError, "partial alpha"):
            ek.validate_contract(self.kit(images={"floor01_props": image}))

    def test_transparent_collision_tile_rejected(self) -> None:
        image = self.base.atlases["floor01_terrain"]["image"].copy()
        image.putpixel((3, 3), (0, 0, 0, 0))
        with self.assertRaisesRegex(ek.KitError, "fully opaque"):
            ek.validate_contract(self.kit(images={"floor01_terrain": image}))

    def test_opaque_option_is_opt_in_and_never_weakens_the_alpha_check(self) -> None:
        """AREA-FASTFOOD-01 (FF-AD17): a piece declared ``opaque`` must be fully opaque, and a fully opaque decor
        piece that is not declared still fails the one-transparent-pixel rule."""
        contract = copy.deepcopy(self.base.contract)
        pier = self.piece(contract, "pier_l")                      # holds transparent pixels
        pier["opaque"] = True
        with self.assertRaisesRegex(ek.KitError, "declared opaque but holds a transparent pixel"):
            ek.validate_contract(self.kit(contract))
        contract = copy.deepcopy(self.base.contract)
        grime = self.piece(contract, "grime_a")
        x, y, w, h = grime["rect"]
        image = self.base.atlases[grime["atlas"]]["image"].copy()
        colour = next(image.getpixel((x + i, y + j)) for j in range(h) for i in range(w)
                      if image.getpixel((x + i, y + j))[3] == 255)
        for j in range(h):
            for i in range(w):
                image.putpixel((x + i, y + j), colour)
        with self.assertRaisesRegex(ek.KitError, "decor pieces must use"):
            ek.validate_contract(self.kit(contract, images={grime["atlas"]: image}))
        grime["opaque"] = True                                     # declared and fully opaque: accepted
        ek.validate_contract(self.kit(contract, images={grime["atlas"]: image}))

    def test_misaligned_tile_rect_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        contract["tiles"][1]["rect"][0] += 1
        with self.assertRaisesRegex(ek.KitError, "tile grid"):
            ek.validate_contract(self.kit(contract))

    def test_collidable_decor_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "pier_l")["collidable"] = True
        with self.assertRaisesRegex(ek.KitError, "never collidable"):
            ek.validate_contract(self.kit(contract))

    def test_collidable_backwall_tile_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "brick_mid_a_1")["collidable"] = True
        with self.assertRaisesRegex(ek.KitError, "collidable"):
            ek.validate_contract(self.kit(contract))

    def test_collidable_deck_back_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        tile = self.tile(contract, "deck_back_mid_a_1")
        tile["collidable"] = True
        tile["collider"] = [0, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "deck_back"):
            ek.validate_contract(self.kit(contract))

    def test_collider_must_follow_walk_line(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "deck_front_mid_a_1")["collider"] = [0, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "walking line rule"):
            ek.validate_contract(self.kit(contract))

    def test_overlapping_piece_rects_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        rect = self.piece(contract, "pipe_h")["rect"]
        self.piece(contract, "pipe_h_seam")["rect"] = [rect[0] + 8, rect[1], 16, 16]
        with self.assertRaisesRegex(ek.KitError, "overlaps"):
            ek.validate_contract(self.kit(contract))

    def test_animation_duration_mismatch_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "torch_wall")["animation"]["frames"][1]["duration"] = 999
        with self.assertRaisesRegex(ek.KitError, "duration"):
            ek.validate_contract(self.kit(contract))

    def test_bad_light_metadata_rejected(self) -> None:
        cases = [("colour", "orange", "colour"), ("flicker", [1.0, 1.0], "flicker"),
                 ("group", "actors", "group"), ("radius", 0, "radius")]
        for key, value, msg in cases:
            with self.subTest(key):
                contract = copy.deepcopy(self.base.contract)
                self.piece(contract, "torch_wall")["lights"][0][key] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_contract(self.kit(contract))

    def test_parallax_piece_needs_tile_x(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        del self.piece(contract, "gallery_mid")["tile_x"]
        with self.assertRaisesRegex(ek.KitError, "tile_x"):
            ek.validate_contract(self.kit(contract))

    def test_manifest_tile_order_mismatch_rejected(self) -> None:
        kit = self.kit()
        tiles = kit.atlases["floor01_terrain"]["manifest"]["tiles"]
        tiles[0], tiles[1] = tiles[1], tiles[0]
        with self.assertRaisesRegex(ek.KitError, "manifest tiles"):
            ek.validate_contract(kit)

    def test_non_neutral_id_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "valve_wheel")["id"] = "goblin_valve"
        with self.assertRaisesRegex(ek.KitError, "IP-neutral"):
            ek.validate_contract(self.kit(contract))

    # ---- negative: layout ------------------------------------------------
    def test_unknown_tile_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["backwall"][5][3] = "brick_backwall_z9"
        with self.assertRaisesRegex(ek.KitError, "unknown tile"):
            ek.validate_layout(self.base, layout)

    def test_backwall_tile_in_collision_layer_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["collision"][10][3] = "brick_mid_a_1"
        with self.assertRaisesRegex(ek.KitError, "layer needs collision"):
            ek.validate_layout(self.base, layout)

    def test_broken_panel_continuity_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        row = layout["tile_layers"]["backwall"][6]
        x = next(i for i, t in enumerate(row) if t and t.endswith("_1") and row[i + 1].endswith("_2"))
        row[x + 1] = row[x]  # the same panel cell twice in a row
        with self.assertRaisesRegex(ek.KitError, "continuity"):
            ek.validate_layout(self.base, layout)

    def test_edge_tile_must_touch_panel_edges(self) -> None:
        layout = copy.deepcopy(self.layout)
        row = layout["tile_layers"]["collision"][18]
        x = row.index("deck_back_inner_r")
        row[x - 1] = row[x - 2]  # left neighbour is now not a panel's last column
        with self.assertRaisesRegex(ek.KitError, "continuity|last column"):
            ek.validate_layout(self.base, layout)

    def test_pit_in_floor_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        for row in layout["tile_layers"]["collision"][3:]:
            row[20] = None
        with self.assertRaisesRegex(ek.KitError, "pit"):
            ek.validate_layout(self.base, layout)

    def test_walkable_tile_needs_deck_back(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["collision"][18][20] = None
        with self.assertRaisesRegex(ek.KitError, "deck_back"):
            ek.validate_layout(self.base, layout)

    def test_tall_step_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid = layout["tile_layers"]["collision"]
        grid[14][25] = "deck_back_mid_c_1"
        grid[15][25] = "deck_front_mid_c_1"
        for r in (16, 17, 18, 19):
            grid[r][25] = "wall_fill"
        with self.assertRaisesRegex(ek.KitError, "step"):
            ek.validate_layout(self.base, layout)

    def test_brick_inside_arch_window_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["backwall"][8][41] = "brick_mid_s"
        with self.assertRaisesRegex(ek.KitError, "arch window"):
            ek.validate_layout(self.base, layout)

    def test_decal_inside_arch_window_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        pool = next(p for p in layout["piece_layers"]["backwall_decal"] if p["piece"].startswith("torch_pool"))
        pool["x"] = 560
        with self.assertRaisesRegex(ek.KitError, "arch window"):
            ek.validate_layout(self.base, layout)

    def test_missing_pier_leaves_holes_outside_windows(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["piece_layers"]["structure"] = [s for s in layout["piece_layers"]["structure"]
                                               if not (s["piece"] == "pier_l" and s["x"] == 512)]
        with self.assertRaisesRegex(ek.KitError, "transparent pixels outside"):
            ek.validate_layout(self.base, layout)

    def test_parallax_strip_must_cover_windows(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["parallax_layers"]["parallax_far"]["y"] += 100
        with self.assertRaisesRegex(ek.KitError, "does not cover a window"):
            ek.validate_layout(self.base, layout)

    def test_lighting_profile_must_be_presentation_only_and_complete(self) -> None:
        layout = copy.deepcopy(self.layout)
        del layout["lighting"]["groups"]["actors"]
        with self.assertRaisesRegex(ek.KitError, "lighting groups"):
            ek.validate_layout(self.base, layout)
        layout = copy.deepcopy(self.layout)
        layout["lighting"]["presentation_only"] = False
        with self.assertRaisesRegex(ek.KitError, "presentation_only"):
            ek.validate_layout(self.base, layout)

    def test_off_grid_piece_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        torch = next(p for p in layout["piece_layers"]["prop"] if p["piece"] == "torch_wall")
        torch["x"] += 4
        with self.assertRaisesRegex(ek.KitError, "grid"):
            ek.validate_layout(self.base, layout)

    def test_fixed_y_piece_rejected_elsewhere(self) -> None:
        layout = copy.deepcopy(self.layout)
        pool = next(p for p in layout["piece_layers"]["backwall_decal"] if p["piece"].startswith("torch_pool"))
        pool["y"] += 16
        with self.assertRaisesRegex(ek.KitError, "must be placed at"):
            ek.validate_layout(self.base, layout)

    def test_missing_exit_marker_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["markers"] = [m for m in layout["markers"] if m["kind"] != "exit"]
        with self.assertRaisesRegex(ek.KitError, "exit"):
            ek.validate_layout(self.base, layout)

    def test_layout_declares_the_loaded_contract(self) -> None:
        self.assertEqual(ek.repo_path(self.layout["kit_contract"]), self.base.path)
        layout = copy.deepcopy(self.layout)
        layout["kit_contract"] = "assets/source/terrain/floor01/other_kit.json"
        with self.assertRaisesRegex(ek.KitError, "kit_contract"):
            ek.validate_layout(self.base, layout)

    def test_base_walk_line_mismatch_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["base_walk_line_y"] += 4
        with self.assertRaisesRegex(ek.KitError, "base_walk_line_y"):
            ek.validate_layout(self.base, layout)

    def test_marker_foot_row_must_match_walk_line(self) -> None:
        for kind in ("spawn", "exit"):
            layout = copy.deepcopy(self.layout)
            self.marker(layout, kind)["y"] += 2  # a 2 px foot sink is not allowed
            with self.assertRaisesRegex(ek.KitError, "foot contact row"):
                ek.validate_layout(self.base, layout)

    def test_marker_on_raised_platform_uses_its_walk_line(self) -> None:
        layout = copy.deepcopy(self.layout)
        spawn = self.marker(layout, "spawn")
        spawn["x"] = 9 * 16 + 8  # one-tile platform: walking line 292 instead of 308
        with self.assertRaisesRegex(ek.KitError, "foot contact row"):
            ek.validate_layout(self.base, layout)
        spawn["y"] = 291
        ek.validate_layout(self.base, layout)

    def test_marker_rect_outside_layout_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        self.marker(layout, "exit")["rect"] = [layout["pixel_size"][0] - 8, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "rect"):
            ek.validate_layout(self.base, layout)

    def test_segment_boundary_off_tile_grid_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["segments"][0]["x"][1] += 8
        layout["segments"][1]["x"][0] += 8
        with self.assertRaisesRegex(ek.KitError, "multiples of 16"):
            ek.validate_layout(self.base, layout)

    def test_floating_collision_tile_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["collision"][10][20] = "wall_fill"
        with self.assertRaisesRegex(ek.KitError, "floating|runs"):
            ek.validate_layout(self.base, layout)

    def test_third_collision_run_in_a_column_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid = layout["tile_layers"]["collision"]
        grid[6][20] = "wall_fill"
        grid[9][20] = "wall_fill"
        with self.assertRaisesRegex(ek.KitError, "runs"):
            ek.validate_layout(self.base, layout)

    def test_backwall_hole_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["backwall"][8][20] = None
        with self.assertRaisesRegex(ek.KitError, "hole"):
            ek.validate_layout(self.base, layout)

    # ---- negative: walking line / collider semantics (tooling round) -------
    def collision(self, layout: dict) -> tuple[list, int, int]:
        cols, rows = layout["grid_size"]
        return layout["tile_layers"]["collision"], cols, rows

    def test_second_walking_surface_in_a_floor_column_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid, cols, rows = self.collision(layout)
        grid[21][20] = "deck_front_mid_a_1"  # a second walkable tile below the real surface
        with self.assertRaisesRegex(ek.KitError, "exactly one walking surface"):
            ek._check_walking_surfaces(self.base, grid, cols, rows)

    def test_walkable_tile_in_ceiling_or_wall_column_rejected(self) -> None:
        for r, x in ((1, 20), (10, 0)):  # ceiling run, left end wall column
            with self.subTest(row=r, col=x):
                layout = copy.deepcopy(self.layout)
                grid, cols, rows = self.collision(layout)
                grid[r][x] = "deck_front_mid_a_1"
                with self.assertRaisesRegex(ek.KitError, "outside a floor run"):
                    ek._check_walking_surfaces(self.base, grid, cols, rows)

    def test_orphan_deck_back_tile_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid, cols, rows = self.collision(layout)
        grid[10][20] = "deck_back_mid_a_1"
        with self.assertRaisesRegex(ek.KitError, "no walkable tile directly below"):
            ek._check_walking_surfaces(self.base, grid, cols, rows)

    def test_gap_under_the_walking_surface_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["collision"][21][20] = None  # floor face cut open under the lip
        with self.assertRaisesRegex(ek.KitError, "walkable_top"):
            ek.validate_layout(self.base, layout)

    def test_floating_walkable_slab_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid, cols, rows = self.collision(layout)
        grid[12][20] = "deck_back_mid_a_1"
        grid[13][20] = "deck_front_mid_a_1"
        with self.assertRaisesRegex(ek.KitError, "floating|runs"):
            ek._check_collision_topology(self.base, grid, cols, rows)

    def test_walkable_flag_and_walk_line_range_rejected(self) -> None:
        cases = [("walkable_top", "yes", "boolean"), ("walk_line_y", 16, "walk_line_y 0..15"),
                 ("walk_line_y", -1, "walk_line_y 0..15"), ("walk_line_y", True, "walk_line_y 0..15")]
        for key, value, msg in cases:
            with self.subTest(key=key, value=value):
                contract = copy.deepcopy(self.base.contract)
                self.tile(contract, "deck_front_mid_a_1")[key] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_contract(self.kit(contract))

    def test_walk_line_on_a_non_walkable_tile_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "wall_fill")["walk_line_y"] = 4
        with self.assertRaisesRegex(ek.KitError, "without walkable_top"):
            ek.validate_contract(self.kit(contract))

    def test_collider_rect_must_be_exact_integers_and_match_the_tile_kind(self) -> None:
        cases = [("deck_front_mid_a_1", [0, 4, 16, 11]), ("deck_front_mid_a_1", [0, 4.0, 16, 12]),
                 ("deck_front_mid_a_1", [0, 4, 16]), ("wall_fill", [0, 4, 16, 12]),
                 ("wall_fill", [0, 0, 16, 15])]
        for tid, collider in cases:
            with self.subTest(tile=tid, collider=collider):
                contract = copy.deepcopy(self.base.contract)
                self.tile(contract, tid)["collider"] = collider
                with self.assertRaisesRegex(ek.KitError, "walking line rule"):
                    ek.validate_contract(self.kit(contract))

    def test_non_collidable_tile_cannot_carry_a_collider(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "deck_back_mid_a_1")["collider"] = [0, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "no collider"):
            ek.validate_contract(self.kit(contract))

    def test_floor_check_cannot_be_disabled_and_step_limit_is_capped(self) -> None:
        for reqs, msg in (({"max_step_tiles": 2}, "continuous_floor"),
                          ({"continuous_floor": False, "max_step_tiles": 2}, "continuous_floor"),
                          ({"continuous_floor": True, "max_step_tiles": 3}, "max_step_tiles"),
                          ({"continuous_floor": True, "max_step_tiles": 0}, "max_step_tiles")):
            with self.subTest(reqs=reqs):
                layout = copy.deepcopy(self.layout)
                layout["requirements"] = reqs
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_layout(self.base, layout)

    def test_two_tile_step_is_the_largest_accepted(self) -> None:
        layout = copy.deepcopy(self.layout)
        cols, rows = layout["grid_size"]
        tops = ek.collider_tops(self.base, layout["tile_layers"]["collision"], cols, rows)
        steps = [abs(a - b) for a, b in zip(tops, tops[1:]) if isinstance(a, int) and isinstance(b, int)]
        self.assertEqual(max(steps), 2 * 16)  # the committed A2 platform

    def test_base_walk_line_must_match_the_contract_base_floor(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        contract["walk_line"]["base_floor"]["walk_line_y"] = 309
        contract["walk_line"]["base_floor"]["front_edge_y"] = 313
        with self.assertRaisesRegex(ek.KitError, "base_walk_line_y"):
            ek.validate_layout(self.kit(contract), self.layout)

    def test_marker_kinds_stand_on_the_walking_line_of_a_walkable_column(self) -> None:
        layout = copy.deepcopy(self.layout)
        self.marker(layout, "spawn")["x"] = 8  # column 0 is the end wall: no walking line
        with self.assertRaisesRegex(ek.KitError, "foot contact row"):
            ek.validate_layout(self.base, layout)

    def test_actor_check_entries_need_a_walking_line(self) -> None:
        for x in (8, 1 << 20, "400"):
            with self.subTest(x=x):
                layout = copy.deepcopy(self.layout)
                layout["preview"]["actor_check"][0]["x"] = x
                with self.assertRaisesRegex(ek.KitError, "actor_check"):
                    ek.validate_layout(self.base, layout)

    # ---- negative: parallax -----------------------------------------------
    def test_parallax_factor_must_be_inside_the_open_interval(self) -> None:
        for value in (0, 0.0, 1, 1.0, 1.5, -0.2, True, "0.5", float("nan")):
            with self.subTest(value=value):
                contract = copy.deepcopy(self.base.contract)
                next(l for l in contract["layers"] if l["name"] == "parallax_far")["parallax"] = value
                with self.assertRaisesRegex(ek.KitError, "parallax"):
                    ek.validate_contract(self.kit(contract))

    def test_parallax_factors_must_increase_with_depth(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        next(l for l in contract["layers"] if l["name"] == "parallax_far")["parallax"] = 0.7
        with self.assertRaisesRegex(ek.KitError, "increase with draw order"):
            ek.validate_contract(self.kit(contract))

    def test_far_strip_must_be_fully_opaque(self) -> None:
        image = self.base.atlases["floor01_parallax"]["image"].copy()
        image.putpixel((5, 5), (0, 0, 0, 0))
        with self.assertRaisesRegex(ek.KitError, "fully opaque"):
            ek.validate_contract(self.kit(images={"floor01_parallax": image}))

    def test_strip_that_does_not_wrap_seamlessly_rejected(self) -> None:
        for pid, rect_x, rect_y, donor in (("gallery_far", 0, 0, 100), ("gallery_mid", 0, 232, 200)):
            with self.subTest(pid):
                piece = self.piece(self.base.contract, pid)
                w, h = piece["rect"][2:]
                image = self.base.atlases["floor01_parallax"]["image"].copy()
                for y in range(h):  # last column replaced by an unrelated column of the strip
                    image.putpixel((w - 1, rect_y + y), image.getpixel((donor, rect_y + y)))
                with self.assertRaisesRegex(ek.KitError, "does not tile horizontally"):
                    ek.validate_contract(self.kit(images={"floor01_parallax": image}))

    def test_wrap_seam_rule_accepts_the_committed_strips(self) -> None:
        for pid in ("gallery_far", "gallery_mid"):
            piece = self.base.pieces[pid]
            wrap, allowed = ek._wrap_seam_scores(self.base.crop(piece["atlas"], piece["rect"]))
            self.assertLessEqual(wrap, allowed, pid)

    def test_parallax_piece_must_be_static_and_wide_enough(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "gallery_mid")["rect"][2] = 16
        with self.assertRaisesRegex(ek.KitError, "at least 32 px"):
            ek.validate_contract(self.kit(contract))

    def test_parallax_origin_must_be_a_phase_inside_the_layout(self) -> None:
        cases = [("x_origin", 440, "phase"), ("x_origin", -1, "phase"), ("x_origin", 1.5, "integers"),
                 ("y", -1, "outside layout height"), ("y", 300, "outside layout height")]
        for key, value, msg in cases:
            with self.subTest(key=key, value=value):
                layout = copy.deepcopy(self.layout)
                layout["parallax_layers"]["parallax_far"][key] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_layout(self.base, layout)

    def test_parallax_layer_must_use_a_strip_of_that_layer(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["parallax_layers"]["parallax_far"]["piece"] = "gallery_mid"
        with self.assertRaisesRegex(ek.KitError, "does not belong"):
            ek.validate_layout(self.base, layout)

    def test_far_strip_must_cover_every_window_at_both_camera_ends(self) -> None:
        width, height = self.layout["pixel_size"]
        windows = ek.window_regions(self.base, self.layout)
        ek._check_parallax_coverage(self.base, self.layout, windows, width, height)  # committed: ok
        layout = copy.deepcopy(self.layout)
        layout["parallax_layers"]["parallax_far"]["y"] += 100  # strip slides below the windows
        with self.assertRaisesRegex(ek.KitError, "empty sky"):
            ek._check_parallax_coverage(self.base, layout, windows, width, height)
        image = self.base.atlases["floor01_parallax"]["image"].copy()
        for x in range(0, 60):  # a transparent vertical band in the strip
            for y in range(232):
                image.putpixel((x, y), (0, 0, 0, 0))
        with self.assertRaisesRegex(ek.KitError, "empty sky"):
            ek._check_parallax_coverage(self.kit(images={"floor01_parallax": image}), self.layout,
                                        windows, width, height)

    def test_windows_need_a_far_parallax_layer(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        contract["layers"] = [l for l in contract["layers"] if l["name"] != "parallax_far"]
        layout = copy.deepcopy(self.layout)
        del layout["parallax_layers"]["parallax_far"]
        with self.assertRaisesRegex(ek.KitError, "need a parallax_far layer"):
            ek.validate_layout(self.kit(contract), layout)

    # ---- negative: lights -------------------------------------------------
    def test_light_schema_rejects_bad_values(self) -> None:
        cases = [("torch_wall", "flame", "blur", 2, "unknown field"),
                 ("torch_wall", "flame", "radius", 5000, "radius"),
                 ("torch_wall", "flame", "radius", float("inf"), "radius"),
                 ("torch_wall", "flame", "radius", True, "radius"),
                 ("torch_wall", "flame", "intensity", 9, "intensity"),
                 ("torch_wall", "flame", "intensity", -0.1, "intensity"),
                 ("torch_wall", "flame", "offset", [8.5, 18], "offset"),
                 ("torch_wall", "flame", "offset", [8], "offset"),
                 ("torch_wall", "flame", "colour", "#ff9", "colour"),
                 ("torch_wall", "flame", "glow", {"radius": 3}, "glow"),
                 ("torch_wall", "flame", "glow", {"radius": 3, "additive": 2}, "glow additive"),
                 ("torch_wall", "flame", "flicker", [1, 1, 1, 1, 1], "one multiplier per animation frame"),
                 ("torch_wall", "flame", "flicker", [1, 1, 1], "one multiplier per animation frame"),
                 ("torch_wall", "flame", "flicker", [1, 1, 1, 3.5], "flicker"),
                 ("torch_wall", "flame", "group", "parallax_far", "only parallax strips"),
                 ("torch_wall", "flame", "project", "ground", "project must be"),
                 ("torch_wall", "flame", "project_dy", 1.5, "project_dy"),
                 ("torch_wall", "floor_pool", "squash_y", 0, "squash_y"),
                 ("gallery_far", "far_torch_0", "group", "scene", "own group"),
                 ("gallery_far", "far_torch_0", "flicker", [1.0], "animated piece"),
                 ("gallery_far", "far_torch_0", "project", "walk_line", "cannot project")]
        for pid, lid, key, value, msg in cases:
            with self.subTest(piece=pid, key=key, value=value):
                contract = copy.deepcopy(self.base.contract)
                next(l for l in self.piece(contract, pid)["lights"] if l["id"] == lid)[key] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_contract(self.kit(contract))

    def test_light_structure_rejected(self) -> None:
        cases = [("lights", [], "non-empty list"), ("lights", ["flame"], "must be an object"),
                 ("lights", "torch", "non-empty list")]
        for key, value, msg in cases:
            with self.subTest(value=value):
                contract = copy.deepcopy(self.base.contract)
                self.piece(contract, "torch_wall")[key] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_contract(self.kit(contract))
        contract = copy.deepcopy(self.base.contract)
        lights = self.piece(contract, "torch_wall")["lights"]
        lights[1]["id"] = lights[0]["id"]
        with self.assertRaisesRegex(ek.KitError, "unique"):
            ek.validate_contract(self.kit(contract))
        contract = copy.deepcopy(self.base.contract)
        del self.piece(contract, "torch_wall")["lights"][1]["project"]  # project_dy without project
        with self.assertRaisesRegex(ek.KitError, "project_dy needs project"):
            ek.validate_contract(self.kit(contract))

    def test_flicker_must_match_the_torch_animation_frame_count(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        torch = self.piece(contract, "torch_wall")
        self.assertEqual(len(torch["animation"]["frames"]), 4)
        for light in torch["lights"]:
            light["flicker"] = light["flicker"] + [1.0]
        with self.assertRaisesRegex(ek.KitError, r"flicker needs one multiplier per animation frame \(4\)"):
            ek.validate_contract(self.kit(contract))

    def test_lights_are_anchored_to_piece_instances_only(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["lights"] = [{"id": "free", "x": 10, "y": 10}]
        with self.assertRaisesRegex(ek.KitError, "free-floating"):
            ek.validate_layout(self.base, layout)
        layout = copy.deepcopy(self.layout)
        layout["piece_layers"]["prop"][0]["lights"] = [{"id": "extra"}]
        with self.assertRaisesRegex(ek.KitError, "may only carry"):
            ek.validate_layout(self.base, layout)

    def test_projected_light_needs_a_walking_line_column(self) -> None:
        layout = copy.deepcopy(self.layout)
        torch = next(p for p in layout["piece_layers"]["prop"] if p["piece"] == "torch_wall")
        torch["x"] = 0  # emitter column 0 is the end wall: nothing to project onto
        with self.assertRaisesRegex(ek.KitError, "projects onto column 0"):
            ek.validate_layout(self.base, layout)

    def test_lighting_profile_carries_presentation_data_only(self) -> None:
        cases = [(("collidable",), True, "unknown field"), (("collider",), [0, 0, 16, 16], "unknown field"),
                 (("groups", "scene", "collidable"), True, "unknown field"),
                 (("profile_id",), "", "profile_id"), (("groups", "scene", "light_scale"), 2, "light_scale")]
        for path, value, msg in cases:
            with self.subTest(path=path):
                layout = copy.deepcopy(self.layout)
                node = layout["lighting"]
                for key in path[:-1]:
                    node = node[key]
                node[path[-1]] = value
                with self.assertRaisesRegex(ek.KitError, msg):
                    ek.validate_layout(self.base, layout)

    def test_collision_and_gameplay_data_must_not_reference_lighting(self) -> None:
        profile = self.layout["lighting"]["profile_id"]
        cases = [("markers", lambda l: l["markers"][0].update({"light": 1})),
                 ("markers", lambda l: l["markers"][0].update({"note": profile})),
                 ("segments", lambda l: l["segments"][0].update({"light_group": "scene"})),
                 ("requirements", lambda l: l["requirements"].update({"lighting": profile}))]
        for section, mutate in cases:
            with self.subTest(section=section):
                layout = copy.deepcopy(self.layout)
                mutate(layout)
                with self.assertRaisesRegex(ek.KitError, "references lighting"):
                    ek.validate_layout(self.base, layout)
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "deck_front_mid_a_1")["light_group"] = "scene"
        with self.assertRaisesRegex(ek.KitError, "references lighting"):
            ek.validate_contract(self.kit(contract))
        contract = copy.deepcopy(self.base.contract)
        contract["walk_line"]["emitters"] = []
        with self.assertRaisesRegex(ek.KitError, "references lighting"):
            ek.validate_contract(self.kit(contract))

    def test_contract_lighting_block_must_be_presentation_only(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        contract["lighting"]["presentation_only"] = False
        with self.assertRaisesRegex(ek.KitError, "presentation_only"):
            ek.validate_contract(self.kit(contract))

    def test_lights_never_change_collision_or_the_unlit_pixels(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        for piece in contract["pieces"]:
            piece.pop("lights", None)
        stripped = self.kit(contract)
        ek.validate_contract(stripped)
        cols, rows = self.layout["grid_size"]
        grid = self.layout["tile_layers"]["collision"]
        self.assertEqual(ek.collider_tops(stripped, grid, cols, rows),
                         ek.collider_tops(self.base, grid, cols, rows))
        self.assertEqual(ek.render_layout(stripped, self.layout).tobytes(),
                         ek.render_layout(self.base, self.layout).tobytes())

    # ---- negative: arch windows (tooling round) ----------------------------
    def test_collision_tile_inside_an_arch_window_rejected(self) -> None:
        width, height = self.layout["pixel_size"]
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["collision"][8][41] = "wall_fill"
        windows = ek.window_regions(self.base, layout)
        with self.assertRaisesRegex(ek.KitError, "collision .* intrudes into an arch window"):
            ek._check_windows(self.base, layout, windows, width, height)

    def test_window_declaration_schema_rejected(self) -> None:
        cases = [("arc_radius", 0), ("arc_radius", -4), ("arc_radius", 96.5),
                 ("arc_centre", [136]), ("arc_centre", [136.0, 134]),
                 ("rect", [40, 134, 0, 122]), ("rect", [40, 134, 192]), ("rect", [-1, 134, 192, 122])]
        for key, value in cases:
            with self.subTest(key=key, value=value):
                contract = copy.deepcopy(self.base.contract)
                self.piece(contract, "arch_ring_a")["window"][key] = value
                with self.assertRaisesRegex(ek.KitError, "window|arc_radius"):
                    ek.validate_contract(self.kit(contract))

    def test_window_must_belong_to_a_structure_piece_and_fit_the_layout(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "torch_wall")["window"] = copy.deepcopy(
            self.piece(contract, "arch_ring_a")["window"])
        with self.assertRaisesRegex(ek.KitError, "structure pieces"):
            ek.validate_contract(self.kit(contract))
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "arch_ring_a")["window"]["rect"][2] = 5000
        with self.assertRaisesRegex(ek.KitError, "outside the layout"):
            ek.validate_layout(self.kit(contract), self.layout)

    # ---- negative: v1 formats and the migration note -----------------------
    def test_v1_files_are_rejected_with_a_pointer_to_the_migration_notes(self) -> None:
        doc = (ROOT / ek.MIGRATION_DOC).read_text(encoding="utf-8")
        self.assertIn("## Migration from v1", doc)
        with tempfile.TemporaryDirectory() as tmp:
            old_contract = Path(tmp) / "kit.json"
            old_contract.write_text(json.dumps({"format": "env-kit-v1", "kit_id": "x"}), encoding="utf-8")
            old_layout = Path(tmp) / "layout.json"
            old_layout.write_text(json.dumps({"format": "env-kit-layout-v1"}), encoding="utf-8")
            attempts = [lambda: ek.load_kit(old_contract), lambda: ek.load_layout(old_layout),
                        lambda: ek.validate_contract(self.kit({"format": "env-kit-v1"})),
                        lambda: ek.validate_layout(self.base, {"format": "env-kit-layout-v1"})]
            for attempt in attempts:
                with self.assertRaises(ek.KitError) as ctx:
                    attempt()
                message = str(ctx.exception)
                self.assertIn("no longer supported", message)
                self.assertIn("Migration from v1", message)
                self.assertIn(ek.MIGRATION_DOC, message)

    def test_unknown_or_malformed_format_is_rejected(self) -> None:
        for data in ({"format": "env-kit-v9"}, {"format": None}, {}, [], "env-kit-v2"):
            with self.subTest(data=data), self.assertRaises(ek.KitError) as ctx:
                ek.check_format(data, ek.CONTRACT_FORMAT, "contract")
            if isinstance(data, dict):
                self.assertIn("Migration from v1", str(ctx.exception))

    def test_v1_cli_exit_code_and_message(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT / "generated") as tmp, \
                contextlib.redirect_stderr(io.StringIO()) as err:
            old = Path(tmp) / "old_kit.json"
            old.write_text(json.dumps({"format": "env-kit-v1"}), encoding="utf-8")
            code = ek.main(["validate", "--contract", old.relative_to(ROOT).as_posix()])
        self.assertEqual(code, 1)
        self.assertIn("ENV_KIT_ERROR", err.getvalue())
        self.assertIn("Migration from v1", err.getvalue())

    # ---- determinism and CI wiring ----------------------------------------
    def test_full_export_previews_including_actor_checks_are_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            first = ek.render_all(self.base, self.layout, Path(tmp) / "a")
            second = ek.render_all(self.base, self.layout, Path(tmp) / "b")
            self.assertEqual([p.name for p in first], [p.name for p in second])
            self.assertTrue(any(p.name.startswith("actor_check") for p in first))
            for p, q in zip(first, second):
                self.assertEqual(p.read_bytes(), q.read_bytes(), p.name)

    def test_ci_checks_the_whole_generated_art_tree_and_a_clean_worktree(self) -> None:
        workflow = (ROOT / ".github/workflows/art.yml").read_text(encoding="utf-8")
        self.assertIn("find generated/art -type f", workflow)  # recursive hash of every artifact
        self.assertIn("generated/art/env_kit/", workflow)  # previews must exist, not just hash equal
        self.assertIn("git status --porcelain", workflow)
        self.assertNotIn("git status --porcelain --", workflow)  # whole tree, not one directory
        self.assertIn("python3 scripts/check_docs.py", workflow)  # verifies every SHA256SUMS file
        export = (ROOT / "scripts/export_assets.sh").read_text(encoding="utf-8")
        self.assertIn("python3 scripts/env_kit.py all", export)

    # ---- layout discovery ------------------------------------------------
    def test_find_layouts_finds_the_committed_layout(self) -> None:
        self.assertEqual([p.relative_to(ROOT).as_posix() for p in ek.find_layouts()], [
            LAYOUT,
            "assets/levels/floor01/opening_route_proposal.json",            # ENV-02 proposal
            "assets/levels/floor01/opening_route_proposal_ceiling_t.json",  # ENV-02 option T
            "assets/levels/floor01/refuge_guild.json",                      # AREA-REFUGE-01 guild room
        ])

    def test_find_layouts_fails_on_unparsable_and_unknown_versions(self) -> None:
        cases = {"broken.json": "{not json",
                 "old.json": json.dumps({"format": "env-kit-layout-v1"}),
                 "future.json": json.dumps({"format": "env-kit-layout-v3"}),
                 "contract.json": json.dumps({"format": "env-kit-v2"})}
        for name, text in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                (Path(tmp) / "nested").mkdir()
                (Path(tmp) / "nested" / name).write_text(text, encoding="utf-8")
                with self.assertRaises(ek.KitError):
                    ek.find_layouts(Path(tmp))

    def test_find_layouts_ignores_unrelated_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "other.json").write_text(json.dumps({"format": "ldtk"}), encoding="utf-8")
            (Path(tmp) / "list.json").write_text("[1, 2]", encoding="utf-8")
            self.assertEqual(ek.find_layouts(Path(tmp)), [])

    # ---- best-effort previews --------------------------------------------
    def test_missing_preview_inputs_warn_but_do_not_fail(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["preview"]["target_frame"] = "docs/evidence/missing/none.png"
        layout["preview"]["actor_check"] = [{"manifest": "assets/manifests/missing.json", "x": 800}]
        with tempfile.TemporaryDirectory() as tmp, \
                contextlib.redirect_stderr(io.StringIO()) as err:
            written = ek.render_all(self.base, layout, Path(tmp))
            names = {p.name for p in written}
        self.assertIn("corridor_overview_2048x400.png", names)
        self.assertFalse(any(n.startswith(("actor_check", "corridor_unlit_vs")) for n in names))
        self.assertEqual(err.getvalue().count("WARNING"), 2)

    def test_present_preview_inputs_are_rendered(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            names = {p.name for p in ek.render_all(self.base, self.layout, Path(tmp))}
        self.assertIn("corridor_unlit_vs_a1_target.png", names)
        self.assertIn("actor_check_640x360.png", names)


if __name__ == "__main__":
    unittest.main()
