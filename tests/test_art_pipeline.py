"""Unit tests of art asset contracts; no paid editor or GPU required."""
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from art_pipeline import AssetError, Manifest, make_preview, validate

ROOT = Path(__file__).resolve().parents[1]
SMOKE = ROOT / "tests" / "fixtures" / "neutral_smoke.manifest.json"


class ArtPipelineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.tmp = Path(self.temp.name)
        self.manifest = Manifest.load(SMOKE)
        self.atlas = self.tmp / "smoke.png"
        self.data = self.tmp / "smoke.json"
        image = Image.new("RGBA", (34, 16), (0, 0, 0, 0))
        for offset in (0, 18):
            for x in range(6 + (offset == 18), 10 + (offset == 18)):
                for y in range(6, 14):
                    image.putpixel((x + offset, y), (62, 161, 188, 255))
        image.save(self.atlas)
        self.meta = {
            "frames": [
                {"filename": f"frame{i}",
                 "frame": {"x": 18 * i, "y": 0, "w": 16, "h": 16},
                 "rotated": False, "trimmed": False,
                 "sourceSize": {"w": 16, "h": 16}, "duration": 120 + i * 40}
                for i in range(2)
            ],
            "meta": {"size": {"w": 34, "h": 16},
                     "frameTags": [{"name": "idle", "from": 0, "to": 1}]}
        }
        self.write()

    def write(self) -> None:
        self.data.write_text(json.dumps(self.meta), encoding="utf-8")

    def test_valid_atlas_and_previews(self) -> None:
        report = validate(self.manifest, atlas=self.atlas, data=self.data)
        self.assertEqual(report["frames"], 2)
        self.assertEqual(report["tags"], ["idle"])
        sheet, gif = make_preview(self.manifest, atlas=self.atlas, data=self.data,
                                  output_dir=self.tmp / "evidence")
        self.assertTrue(sheet.is_file())
        with Image.open(gif) as animation:
            self.assertEqual(animation.n_frames, 2)

    def test_reject_missing_tags(self) -> None:
        self.meta["meta"]["frameTags"] = []
        self.write()
        with self.assertRaisesRegex(AssetError, "frameTags"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_out_of_bounds(self) -> None:
        self.meta["frames"][1]["frame"]["x"] = 30
        self.write()
        with self.assertRaisesRegex(AssetError, "outside"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_overlap(self) -> None:
        self.meta["frames"][1]["frame"]["x"] = 8
        self.write()
        with self.assertRaisesRegex(AssetError, "overlaps"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_inconsistent_source_size(self) -> None:
        self.meta["frames"][0]["sourceSize"]["w"] = 15
        self.write()
        with self.assertRaisesRegex(AssetError, "sourceSize"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_empty_frame(self) -> None:
        with Image.open(self.atlas) as img:
            image = img.copy()
        for x in range(18, 34):
            for y in range(16):
                image.putpixel((x, y), (0, 0, 0, 0))
        image.save(self.atlas)
        with self.assertRaisesRegex(AssetError, "completely transparent"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_untagged_frame(self) -> None:
        self.meta["meta"]["frameTags"][0]["to"] = 0
        self.write()
        with self.assertRaisesRegex(AssetError, "no animation tag"):
            validate(self.manifest, atlas=self.atlas, data=self.data)

    def test_reject_manifest_path_traversal(self) -> None:
        manifest = json.loads(SMOKE.read_text())
        manifest["source_path"] = "../secret.aseprite"
        path = self.tmp / "evil.json"
        path.write_text(json.dumps(manifest))
        with self.assertRaisesRegex(AssetError, "escapes repository"):
            Manifest.load(path)

    def test_reject_pivot_outside_frame(self) -> None:
        manifest = json.loads(SMOKE.read_text())
        manifest["pivot"] = [16, 2]
        path = self.tmp / "bad.json"
        path.write_text(json.dumps(manifest))
        with self.assertRaisesRegex(AssetError, "Pivot"):
            Manifest.load(path)

    def test_reject_palette_budget(self) -> None:
        with Image.open(self.atlas) as img:
            image = img.copy()
        for i, xy in enumerate([(7, 7), (8, 8), (7, 8), (8, 9), (7, 9), (8, 10)]):
            image.putpixel(xy, (i * 30, 22, 44, 255))
        image.save(self.atlas)
        with self.assertRaisesRegex(AssetError, "Palette limit"):
            validate(self.manifest, atlas=self.atlas, data=self.data)


if __name__ == "__main__":
    unittest.main()
