"""Tests for the env-kit-v1 contract / layout validator and the kit previews.

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

    # ---- positive -------------------------------------------------------
    def test_committed_kit_and_layout_are_valid(self) -> None:
        ek.validate_contract(self.base)
        ek.validate_layout(self.base, self.layout)

    def test_cli_validate(self) -> None:
        self.assertEqual(ek.main(["validate", "--contract", CONTRACT, "--layout", LAYOUT]), 0)

    def test_grids_share_the_kit_palette_and_ase_is_fresh(self) -> None:
        palette = self.base.contract["palette"]
        self.assertLessEqual(len(palette), 64)
        for name, atlas in self.base.atlases.items():
            grid = pg.load_grid(ROOT / atlas["ref"]["grid"])
            self.assertEqual([e["rgba"] for e in grid["palette"]], [e["rgba"] for e in palette], name)
            ase = ROOT / atlas["manifest"]["source_path"]
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(grid), f"{ase} is stale")

    def test_only_collision_tiles_are_collidable(self) -> None:
        for tile in self.base.contract["tiles"]:
            self.assertIs(tile["collidable"], tile["role"] == "collision", tile["id"])
        for piece in self.base.contract["pieces"]:
            self.assertIs(piece["collidable"], False, piece["id"])

    def test_layout_size_markers_and_segments(self) -> None:
        cols, rows = self.layout["grid_size"]
        self.assertGreaterEqual(cols, 128)
        self.assertGreaterEqual(rows, 25)
        kinds = {m["kind"] for m in self.layout["markers"]}
        self.assertTrue({"spawn", "exit"} <= kinds)
        self.assertEqual([s["grammar"] for s in self.layout["segments"]][:3], ["A", "B", "C"])

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

    def test_torch_animation_matches_export(self) -> None:
        torch = self.piece(self.base.contract, "torch_wall")
        meta = self.base.atlases["floor01_torch"]["metadata"]
        self.assertEqual([f["duration"] for f in torch["animation"]["frames"]],
                         [f["duration"] for f in meta["frames"]])

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
        self.tile(contract, "floor_top_b")["rect"] = [17, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "tile grid"):
            ek.validate_contract(self.kit(contract))

    def test_collidable_decor_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "pier_shaft")["collidable"] = True
        with self.assertRaisesRegex(ek.KitError, "never collidable"):
            ek.validate_contract(self.kit(contract))

    def test_collidable_backwall_tile_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.tile(contract, "brick_backwall_a1")["collidable"] = True
        with self.assertRaisesRegex(ek.KitError, "collidable"):
            ek.validate_contract(self.kit(contract))

    def test_overlapping_piece_rects_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "pipe_h_rust")["rect"] = [8, 0, 16, 16]
        with self.assertRaisesRegex(ek.KitError, "overlaps"):
            ek.validate_contract(self.kit(contract))

    def test_animation_duration_mismatch_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "torch_wall")["animation"]["frames"][1]["duration"] = 999
        with self.assertRaisesRegex(ek.KitError, "duration"):
            ek.validate_contract(self.kit(contract))

    def test_manifest_tile_order_mismatch_rejected(self) -> None:
        kit = self.kit()
        tiles = kit.atlases["floor01_terrain"]["manifest"]["tiles"]
        tiles[0], tiles[1] = tiles[1], tiles[0]
        with self.assertRaisesRegex(ek.KitError, "manifest tiles"):
            ek.validate_contract(kit)

    def test_non_neutral_id_rejected(self) -> None:
        contract = copy.deepcopy(self.base.contract)
        self.piece(contract, "maintenance_box")["id"] = "goblin_box"
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
        layout["tile_layers"]["collision"][10][3] = "brick_backwall_a1"
        with self.assertRaisesRegex(ek.KitError, "layer needs collision"):
            ek.validate_layout(self.base, layout)

    def test_broken_panel_continuity_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        row = layout["tile_layers"]["backwall"][6]
        self.assertEqual(row[0][:-1], row[1][:-1])
        row[1] = row[0]  # same panel cell twice in a row
        with self.assertRaisesRegex(ek.KitError, "continuity"):
            ek.validate_layout(self.base, layout)

    def test_pit_in_floor_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        for row in layout["tile_layers"]["collision"][3:]:
            row[10] = None
        with self.assertRaisesRegex(ek.KitError, "pit"):
            ek.validate_layout(self.base, layout)

    def test_tall_step_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid = layout["tile_layers"]["collision"]
        for r in (16, 17, 18, 19):
            grid[r][5] = "wall_fill_a"
        grid[15][5] = "floor_top_a"
        with self.assertRaisesRegex(ek.KitError, "step"):
            ek.validate_layout(self.base, layout)

    def test_brick_inside_arch_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["tile_layers"]["backwall"][8][63] = "brick_backwall_a1"
        with self.assertRaisesRegex(ek.KitError, "recess"):
            ek.validate_layout(self.base, layout)

    def test_off_grid_piece_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        torch = next(p for p in layout["piece_layers"]["prop"] if p["piece"] == "torch_wall")
        torch["x"] += 4
        with self.assertRaisesRegex(ek.KitError, "grid"):
            ek.validate_layout(self.base, layout)

    def test_missing_exit_marker_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["markers"] = [m for m in layout["markers"] if m["kind"] != "exit"]
        with self.assertRaisesRegex(ek.KitError, "exit"):
            ek.validate_layout(self.base, layout)

    def marker(self, layout: dict, kind: str) -> dict:
        return next(m for m in layout["markers"] if m["kind"] == kind)

    def test_layout_declares_the_loaded_contract(self) -> None:
        self.assertEqual(ek.repo_path(self.layout["kit_contract"]), self.base.path)
        layout = copy.deepcopy(self.layout)
        layout["kit_contract"] = "assets/source/terrain/floor01/other_kit.json"
        with self.assertRaisesRegex(ek.KitError, "kit_contract"):
            ek.validate_layout(self.base, layout)

    def test_base_floor_top_y_mismatch_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        layout["base_floor_top_y"] += 16
        with self.assertRaisesRegex(ek.KitError, "base_floor_top_y"):
            ek.validate_layout(self.base, layout)

    def test_marker_foot_row_must_match_surface(self) -> None:
        for kind in ("spawn", "exit"):
            layout = copy.deepcopy(self.layout)
            self.marker(layout, kind)["y"] += 2  # a 2 px foot sink is not allowed
            with self.assertRaisesRegex(ek.KitError, "foot contact row"):
                ek.validate_layout(self.base, layout)

    def test_marker_on_raised_block_uses_that_surface(self) -> None:
        layout = copy.deepcopy(self.layout)
        spawn = self.marker(layout, "spawn")
        spawn["x"] = 31 * 16 + 8  # raised block: surface is one row higher than the base
        with self.assertRaisesRegex(ek.KitError, "foot contact row"):
            ek.validate_layout(self.base, layout)
        spawn["y"] -= 16
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
        layout["tile_layers"]["collision"][10][20] = "wall_fill_a"
        with self.assertRaisesRegex(ek.KitError, "floating|runs"):
            ek.validate_layout(self.base, layout)

    def test_third_collision_run_in_a_column_rejected(self) -> None:
        layout = copy.deepcopy(self.layout)
        grid = layout["tile_layers"]["collision"]
        grid[6][20] = "wall_fill_a"
        grid[9][20] = "wall_fill_a"
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
        cases = {"broken.json": "{not json", "future.json": json.dumps({"format": "env-kit-layout-v2"}),
                 "contract.json": json.dumps({"format": "env-kit-v1"})}
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
        layout["preview"]["concept_reference"] = "assets/concept/missing/none.png"
        layout["preview"]["actor_check"] = [{"manifest": "assets/manifests/missing.json", "x": 800}]
        with tempfile.TemporaryDirectory() as tmp, \
                contextlib.redirect_stderr(io.StringIO()) as err:
            written = ek.render_all(self.base, layout, Path(tmp))
            names = {p.name for p in written}
        self.assertIn("corridor_overview_2048x400.png", names)
        self.assertFalse(any(n.startswith(("actor_check", "corridor_vs_concept")) for n in names))
        self.assertEqual(err.getvalue().count("WARNING"), 2)

    def test_present_preview_inputs_are_rendered(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            names = {p.name for p in ek.render_all(self.base, self.layout, Path(tmp))}
        self.assertIn("corridor_vs_concept_side_by_side.png", names)
        self.assertIn("actor_check_640x360.png", names)


if __name__ == "__main__":
    unittest.main()
