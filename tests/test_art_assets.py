"""Negative tests for generated frame geometry, pivots, tags and permissions."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from validate_assets import AssetError, safe_path, validate  # noqa: E402


class ArtValidationTests(unittest.TestCase):
    def setUp(self):
        self.manifest = {
            "schema_version": 1, "id": "test", "type": "character_animation",
            "source_path": "assets/source/characters/placeholder/worker.ase",
            "output_atlas": "generated/art/neutral_worker.png",
            "output_metadata": "generated/art/neutral_worker.json",
            "palette_id": "dungeon-proto-v0", "frame_size": [32, 52],
            "pivot": [16, 51], "animation_tags": ["walk"],
            "required_alpha": True, "owner": "art-fixture",
            "tool_version": "LibreSprite=1.3",
            "provenance": {"origin": "original editable fixture"},
            "license": "Original prototype fixture; repo-only",
            "review_status": "prototype_fixture_unapproved",
        }
        self.meta = {
            "frames": [{
                "frame": {"x": 0, "y": 0, "w": 32, "h": 52},
                "duration": 120, "rotated": False, "trimmed": False,
            }],
            "meta": {"frameTags": [{"name": "walk", "from": 0, "to": 0}]},
        }
        self.image = Image.new("RGBA", (32, 52), (0, 0, 0, 0))
        for y in range(8, 30):
            for x in range(8, 20):
                self.image.putpixel((x, y), (200, 110, 65, 255))

    def test_minimal_fixture_is_valid(self):
        validate(self.manifest, self.meta, self.image)

    def test_path_escape_is_rejected(self):
        with self.assertRaises(AssetError):
            safe_path("../outside.ase")

    def test_wrong_pivot_is_rejected(self):
        self.manifest["pivot"] = [32, 51]
        with self.assertRaisesRegex(AssetError, "Pivot"):
            validate(self.manifest, self.meta, self.image)

    def test_frame_beyond_atlas_is_rejected(self):
        self.meta["frames"][0]["frame"]["x"] = 1
        with self.assertRaisesRegex(AssetError, "outside atlas"):
            validate(self.manifest, self.meta, self.image)

    def test_absent_animation_tag_is_rejected(self):
        self.manifest["animation_tags"] = ["cast"]
        with self.assertRaisesRegex(AssetError, "Missing animation"):
            validate(self.manifest, self.meta, self.image)

    def test_tag_range_is_rejected(self):
        self.meta["meta"]["frameTags"][0]["to"] = 99
        with self.assertRaisesRegex(AssetError, "Tag range"):
            validate(self.manifest, self.meta, self.image)

    def test_trimmed_frame_is_rejected(self):
        self.meta["frames"][0]["trimmed"] = True
        with self.assertRaisesRegex(AssetError, "Trimmed"):
            validate(self.manifest, self.meta, self.image)

    def test_transparency_absence_is_rejected(self):
        self.image = Image.new("RGBA", (32, 52), (255, 0, 0, 255))
        with self.assertRaisesRegex(AssetError, "transparent"):
            validate(self.manifest, self.meta, self.image)

    def test_wrong_review_status_is_rejected(self):
        self.manifest["review_status"] = "approved_final"
        with self.assertRaisesRegex(AssetError, "not be labeled final"):
            validate(self.manifest, self.meta, self.image)


if __name__ == "__main__":
    unittest.main()
