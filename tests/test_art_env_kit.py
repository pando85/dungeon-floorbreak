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

    # ---- layout discovery ------------------------------------------------
    def test_find_layouts_finds_the_committed_layout(self) -> None:
        self.assertEqual([p.relative_to(ROOT).as_posix() for p in ek.find_layouts()], [LAYOUT])

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
