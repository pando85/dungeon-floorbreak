"""Negative tests for the Floor 1 scene runtime contract checker.

Each test mutates a valid fixture (or the committed contract) and proves the
matching rule fires. Not an art test: it needs no exported atlases.
"""
from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import check_floor01_scene_contract as scene  # noqa: E402


def actor_fixture(width=36, height=56, pivot=(19, 55), top=4):
    """A four-frame row whose body spans [top, height) and covers the pivot column."""
    image = Image.new("RGBA", (width * 4, height), (0, 0, 0, 0))
    for frame in range(4):
        for y in range(top, height):
            for x in range(frame * width + 8, frame * width + 31):
                image.putpixel((x, y), (200, 120, 80, 255))
    manifest = {"frame_size": [width, height], "pivot": list(pivot)}
    metadata = {"frames": [
        {"frame": {"x": i * width, "y": 0, "w": width, "h": height}} for i in range(4)]}
    return manifest, metadata, image


def check_carl(manifest, metadata, image):
    scene.check_actor("carl_book1_h52", (36, 56), (19, 55), 52, manifest, metadata, image)


def kit_fixture():
    """Minimal valid contract + layout + atlas set mirroring the real shape."""
    atlas = Image.new("RGBA", (32, 32), (10, 10, 10, 255))
    torch = Image.new("RGBA", (64, 56), (10, 10, 10, 255))
    contract = {
        "format": "env-kit-v2",
        "actor_draw_order": 7,
        "layers": [
            {"name": "parallax_far", "kind": "parallax", "draw_order": 0, "parallax": 0.35},
            {"name": "parallax_mid", "kind": "parallax", "draw_order": 1, "parallax": 0.6},
            {"name": "backwall", "kind": "tiles", "draw_order": 2},
            {"name": "backwall_decal", "kind": "pieces", "draw_order": 3},
            {"name": "collision", "kind": "tiles", "draw_order": 4},
            {"name": "structure", "kind": "pieces", "draw_order": 5},
            {"name": "prop", "kind": "pieces", "draw_order": 6},
            {"name": "foreground", "kind": "pieces", "draw_order": 8},
        ],
        "atlases": {"floor01_a": {"image": "generated/art/floor01_a.png"},
                    "floor01_torch": {"image": "generated/art/floor01_torch.png"}},
        "tiles": [
            {"id": "wall", "atlas": "floor01_a", "rect": [0, 0, 16, 16],
             "collidable": True, "collider": [0, 0, 16, 16]},
            {"id": "deck", "atlas": "floor01_a", "rect": [16, 0, 16, 16], "collidable": True,
             "walk_line_y": 4, "collider": [0, 4, 16, 12]},
            {"id": "back", "atlas": "floor01_a", "rect": [0, 16, 16, 16], "collidable": False},
        ],
        "pieces": [
            {"id": "gallery", "atlas": "floor01_a", "rect": [0, 0, 32, 32],
             "layer": "parallax_far", "tile_x": True},
            {"id": "torch", "atlas": "floor01_torch", "rect": [0, 0, 16, 56], "layer": "prop",
             "animation": {"frames": [{"frame": {"x": 16 * i, "y": 0, "w": 16, "h": 56},
                                       "duration": 100} for i in range(4)]}},
        ],
    }
    layout = {
        "format": "env-kit-layout-v2",
        "grid_size": [2, 2],
        "pixel_size": [32, 32],
        "base_walk_line_y": 20,
        "tile_layers": {"collision": [["wall", "deck"], ["back", None]], "backwall": [[None, None], [None, None]]},
        "piece_layers": {"prop": [{"piece": "torch", "x": 0, "y": 0}]},
        "parallax_layers": {"parallax_far": {"piece": "gallery", "x_origin": 0, "y": 0}},
        "markers": [{"kind": "spawn", "x": 4, "y": 19}, {"kind": "exit", "x": 20, "y": 19}],
    }
    images = {"floor01_a": atlas, "floor01_torch": torch}
    return contract, layout, images


class ActorContractTests(unittest.TestCase):
    def test_valid_actor_passes(self):
        check_carl(*actor_fixture())

    def test_changed_pivot_is_rejected(self):
        manifest, metadata, image = actor_fixture()
        manifest["pivot"] = [19, 54]
        with self.assertRaisesRegex(scene.ContractError, "pivot changed"):
            check_carl(manifest, metadata, image)

    def test_feet_above_pivot_row_would_float_and_is_rejected(self):
        # Opaque rows end one row above the pivot: the actor would hover.
        manifest, metadata, _ = actor_fixture()
        image = Image.new("RGBA", (144, 56), (0, 0, 0, 0))
        for frame in range(4):
            for y in range(4, 55):
                for x in range(frame * 36 + 8, frame * 36 + 31):
                    image.putpixel((x, y), (1, 2, 3, 255))
        with self.assertRaisesRegex(scene.ContractError, "lowest opaque row"):
            check_carl(manifest, metadata, image)

    def test_wrong_neutral_height_is_rejected(self):
        manifest, metadata, image = actor_fixture(top=8)
        with self.assertRaisesRegex(scene.ContractError, "neutral frame is 48 px"):
            check_carl(manifest, metadata, image)

    def test_wrong_frame_count_is_rejected(self):
        manifest, metadata, image = actor_fixture()
        metadata["frames"].pop()
        with self.assertRaisesRegex(scene.ContractError, "idle frames"):
            check_carl(manifest, metadata, image)

    def test_padded_atlas_is_rejected(self):
        manifest, metadata, image = actor_fixture()
        padded = Image.new("RGBA", (image.width + 4, image.height))
        with self.assertRaisesRegex(scene.ContractError, "unpadded"):
            check_carl(manifest, metadata, padded)


class KitContractTests(unittest.TestCase):
    def test_valid_kit_passes(self):
        summary = scene.check_kit(*kit_fixture())
        self.assertEqual(summary["atlases"], 2)

    def test_v1_contract_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["format"] = "env-kit-v1"
        with self.assertRaisesRegex(scene.ContractError, "env-kit-v2"):
            scene.check_kit(contract, layout, images)

    def test_missing_atlas_is_rejected(self):
        contract, layout, images = kit_fixture()
        del images["floor01_torch"]
        with self.assertRaisesRegex(scene.ContractError, "exported image missing"):
            scene.check_kit(contract, layout, images)

    def test_rect_outside_atlas_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["tiles"][0]["rect"] = [24, 0, 16, 16]
        with self.assertRaisesRegex(scene.ContractError, "outside atlas"):
            scene.check_kit(contract, layout, images)

    def test_animation_frame_outside_atlas_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["pieces"][1]["animation"]["frames"][3]["frame"]["x"] = 60
        with self.assertRaisesRegex(scene.ContractError, "outside atlas"):
            scene.check_kit(contract, layout, images)

    def test_wrong_collider_rule_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["tiles"][1]["collider"] = [0, 0, 16, 16]
        with self.assertRaisesRegex(scene.ContractError, "collider is not"):
            scene.check_kit(contract, layout, images)

    def test_collider_on_visual_tile_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["tiles"][2]["collider"] = [0, 0, 16, 16]
        with self.assertRaisesRegex(scene.ContractError, "non-collidable tile has a collider"):
            scene.check_kit(contract, layout, images)

    def test_z_table_drift_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["layers"][4]["draw_order"] = 6
        with self.assertRaisesRegex(scene.ContractError, "draw order is not 4"):
            scene.check_kit(contract, layout, images)

    def test_layer_on_the_actor_slot_is_rejected(self):
        contract, layout, images = kit_fixture()
        contract["layers"].append({"name": "extra", "kind": "pieces", "draw_order": 7})
        with self.assertRaisesRegex(scene.ContractError, "collides with actors"):
            scene.check_kit(contract, layout, images)

    def test_translucent_collision_tile_is_rejected(self):
        contract, layout, images = kit_fixture()
        images["floor01_a"].putpixel((2, 2), (10, 10, 10, 0))
        with self.assertRaisesRegex(scene.ContractError, "not fully opaque"):
            scene.check_kit(contract, layout, images)

    def test_unknown_tile_in_layout_is_rejected(self):
        contract, layout, images = kit_fixture()
        layout["tile_layers"]["collision"][0][0] = "nope"
        with self.assertRaisesRegex(scene.ContractError, "unknown tile"):
            scene.check_kit(contract, layout, images)

    def test_piece_in_wrong_layer_is_rejected(self):
        contract, layout, images = kit_fixture()
        layout["piece_layers"] = {"structure": layout["piece_layers"]["prop"]}
        with self.assertRaisesRegex(scene.ContractError, "wrong layer"):
            scene.check_kit(contract, layout, images)

    def test_parallax_piece_must_tile(self):
        contract, layout, images = kit_fixture()
        del contract["pieces"][0]["tile_x"]
        with self.assertRaisesRegex(scene.ContractError, "must tile"):
            scene.check_kit(contract, layout, images)

    def test_spawn_must_sit_on_the_walking_line(self):
        contract, layout, images = kit_fixture()
        layout["markers"][0]["y"] = 20
        with self.assertRaisesRegex(scene.ContractError, "walking line"):
            scene.check_kit(contract, layout, images)

    def test_grid_size_mismatch_is_rejected(self):
        contract, layout, images = kit_fixture()
        layout["tile_layers"]["collision"].pop()
        with self.assertRaisesRegex(scene.ContractError, "wrong grid size"):
            scene.check_kit(contract, layout, images)


class CommittedDataTests(unittest.TestCase):
    """The committed contract and layout satisfy the checker's static rules."""

    def test_committed_layout_markers_and_z_table(self):
        layout = json.loads((ROOT / scene.LAYOUT).read_text())
        contract = json.loads((ROOT / layout["kit_contract"]).read_text())
        # No atlas needed: only the rules that do not read pixels.
        order = {layer["name"]: layer["draw_order"] for layer in contract["layers"]}
        for name, expected in scene.Z_TABLE.items():
            self.assertEqual(order[name], expected)
        spawn = next(m for m in layout["markers"] if m["kind"] == "spawn")
        self.assertEqual(spawn["y"], layout["base_walk_line_y"] - 1)
        self.assertTrue(all(a["image"].startswith("generated/art/")
                            for a in contract["atlases"].values()))

    def test_mutated_committed_z_table_is_caught(self):
        layout = json.loads((ROOT / scene.LAYOUT).read_text())
        contract = json.loads((ROOT / layout["kit_contract"]).read_text())
        broken = copy.deepcopy(contract)
        broken["layers"][0]["draw_order"] = 3
        images = {name: Image.new("RGBA", (1, 1)) for name in broken["atlases"]}
        with self.assertRaises(scene.ContractError):
            scene.check_kit(broken, layout, images)


if __name__ == "__main__":
    unittest.main()
