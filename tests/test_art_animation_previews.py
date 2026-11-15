"""Tests for tag-indexed animation GIF previews (IP-neutral synthetic sprites)."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from make_animation_previews import render_tag  # noqa: E402
from validate_assets import AssetError  # noqa: E402


class AnimatedPreviewTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.output = Path(self.tmp.name) / "tagged.gif"
        self.atlas = Image.new("RGBA", (34, 16), (0, 0, 0, 0))
        for frame, color in enumerate(((70, 150, 180, 255), (210, 125, 80, 255))):
            for y in range(4, 11):
                for x in range(4, 12):
                    self.atlas.putpixel((x + frame * 18, y), color)
        self.data = {"frames": [
            {"frame": {"x": 18 * n, "y": 0, "w": 16, "h": 16},
             "duration": 120 + 40 * n}
            for n in range(2)
        ]}
        self.tag = {"name": "idle", "from": 0, "to": 1}

    def test_tag_generates_multi_frame_animation_with_timing(self) -> None:
        render_tag(self.atlas, self.data, self.tag, self.output)
        with Image.open(self.output) as gif:
            self.assertEqual(gif.format, "GIF")
            self.assertEqual(gif.n_frames, 2)
            self.assertEqual(gif.size, (96, 96))
            self.assertEqual(gif.info["loop"], 0)
            durations = []
            for n in range(gif.n_frames):
                gif.seek(n)
                durations.append(gif.info["duration"])
            self.assertEqual(durations, [120, 160])

    def test_unsafe_tag_name_rejected(self) -> None:
        self.tag["name"] = "../outside"
        with self.assertRaisesRegex(AssetError, "Unsafe"):
            render_tag(self.atlas, self.data, self.tag, self.output)

    def test_tag_range_exceeds_frames(self) -> None:
        self.tag["to"] = 99
        with self.assertRaisesRegex(AssetError, "Invalid frames"):
            render_tag(self.atlas, self.data, self.tag, self.output)

    def test_frame_exceeds_atlas(self) -> None:
        self.data["frames"][1]["frame"]["x"] = 32
        with self.assertRaisesRegex(AssetError, "rectangle"):
            render_tag(self.atlas, self.data, self.tag, self.output)

    def test_invalid_frame_duration(self) -> None:
        self.data["frames"][0]["duration"] = 0
        with self.assertRaisesRegex(AssetError, "duration"):
            render_tag(self.atlas, self.data, self.tag, self.output)


if __name__ == "__main__":
    unittest.main()
