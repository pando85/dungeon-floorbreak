"""Tests for the gameplay-scale comparison scene (IP-neutral synthetic sprites)."""
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import make_scale_comparison as sc  # noqa: E402

HEIGHTS = (32, 40, 48)
FLOOR_ROW = 295
NEEDS_EXPORT = (ROOT / "generated/art/dungeon_materials.png").is_file()


def make_sprite(directory: Path, name: str, height: int, frames: int = 1,
                bottom_padding: int = 3) -> tuple[str, str]:
    """Synthetic blob with transparent padding below its feet (not frame-bottom aligned)."""
    width, frame_h = 24, height + 2 + bottom_padding
    atlas = Image.new("RGBA", (width * frames, frame_h), (0, 0, 0, 0))
    for n in range(frames):
        for y in range(2, 2 + height - n):
            half = 4 + (y * 7) % 3
            for x in range(12 - half, 12 + half):
                color = (60 + 30 * n, 120, 170, 255) if (x + y) % 5 else (230, 170, 60, 255)
                atlas.putpixel((n * width + x, y), color)
    png, meta = directory / f"{name}.png", directory / f"{name}.json"
    atlas.save(png)
    meta.write_text(json.dumps({"frames": [
        {"frame": {"x": n * width, "y": 0, "w": width, "h": frame_h}, "duration": 100}
        for n in range(frames)]}), encoding="utf-8")
    return str(png), str(meta)


def colors(image: Image.Image) -> set:
    return {rgb for _n, rgb in image.convert("RGB").getcolors(maxcolors=1 << 24)}


@unittest.skipUnless(NEEDS_EXPORT, "run scripts/export_assets.sh first (dungeon_materials atlas)")
class ScaleComparisonTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.specs = []
        for h in HEIGHTS:
            png, meta = make_sprite(self.dir, f"blob{h}", h)
            self.specs.append(f"Blob {h} px={png}:{meta}")
        png, meta = make_sprite(self.dir, "ref", 52)
        self.reference = f"Reference placeholder={png}:{meta}"
        self.out = self.dir / "out"
        self.report = sc.build(self.specs, self.reference, self.out)

    def image(self, name: str) -> Image.Image:
        return Image.open(self.out / name).convert("RGB")

    def test_output_sizes(self) -> None:
        for name, size in (("scene_640x360.png", (640, 360)),
                           ("scene_640x360_annotated.png", (640, 360)),
                           ("scene_1280x720_2x.png", (1280, 720)),
                           ("scene_640x400.png", (640, 400)),
                           ("scene_1280x800_2x.png", (1280, 800))):
            self.assertEqual(self.image(name).size, size, name)
        for h in HEIGHTS:
            slug = f"Blob_{h}_px"
            small = self.image(f"closeup_{slug}_6x.png")
            self.assertEqual(small.size, self.image(f"closeup_{slug}_6x_grid.png").size)
            self.assertEqual((small.width % 6, small.height % 6), (0, 0))
        self.assertTrue((self.out / "closeups_side_by_side_6x.png").is_file())
        self.assertTrue((self.out / "comparison_sheet.png").is_file())

    def test_measured_heights_and_shared_floor(self) -> None:
        actors = self.report["actors"]
        self.assertEqual([a["reference"] for a in actors], [True, False, False, False])
        self.assertEqual([a["opaque_height_px"] for a in actors], [52, 32, 40, 48])
        self.assertEqual({a["scene_bottom_opaque_row"] for a in actors}, {FLOOR_ROW})
        data = json.loads((self.out / "comparison.json").read_text(encoding="utf-8"))
        self.assertEqual(data["scene"]["floor_contact_row"], FLOOR_ROW)
        scene = self.image("scene_640x360.png")
        deck = self.image("scene_640x400.png")
        for a in actors:
            px, py = a["pivot"]
            sx, sy = a["scene_position"]
            # Feet pixel sits on the floor row, directly above the first floor row.
            self.assertEqual(py, a["opaque_bbox"][3])
            self.assertEqual(scene.getpixel((sx + px, FLOOR_ROW)), deck.getpixel((sx + px, FLOOR_ROW + 40)))
            sprite = Image.open(self.dir / Path(a["atlas"]).name).convert("RGBA")
            self.assertEqual(scene.getpixel((sx + px, FLOOR_ROW)), sprite.getpixel((px, py))[:3])
            self.assertNotEqual(scene.getpixel((sx + px, FLOOR_ROW + 1)), sprite.getpixel((px, py))[:3])
        centres = [a["scene_position"][0] + a["pivot"][0] for a in actors]
        gaps = {b - a for a, b in zip(centres, centres[1:])}
        self.assertLessEqual(max(gaps) - min(gaps), 1)

    def test_floor_row_identical_in_steam_deck_framing(self) -> None:
        scene = self.image("scene_640x360.png")
        deck = self.image("scene_640x400.png")
        self.assertEqual(scene.crop((0, 300, 640, 360)).tobytes(), deck.crop((0, 340, 640, 400)).tobytes())
        self.assertEqual(self.image("scene_1280x720_2x.png").tobytes(),
                         scene.resize((1280, 720), Image.Resampling.NEAREST).tobytes())

    def test_annotated_differs_only_by_guides(self) -> None:
        clean, annotated = self.image("scene_640x360.png"), self.image("scene_640x360_annotated.png")
        rows = {y for y in range(360) for x in range(640)
                if clean.getpixel((x, y)) != annotated.getpixel((x, y))}
        self.assertEqual(rows, {FLOOR_ROW - 47, FLOOR_ROW - 55})

    def test_closeups_use_no_smoothing(self) -> None:
        for h in HEIGHTS:
            big = self.image(f"closeup_Blob_{h}_px_6x.png")
            native = big.resize((big.width // 6, big.height // 6), Image.Resampling.NEAREST)
            self.assertLessEqual(colors(big), colors(native))
            self.assertEqual(big.tobytes(), native.resize(big.size, Image.Resampling.NEAREST).tobytes())
            grid = self.image(f"closeup_Blob_{h}_px_6x_grid.png")
            for x in range(1, big.width, 7):
                for y in range(1, big.height, 5):
                    if x % 6 and y % 6:
                        self.assertEqual(grid.getpixel((x, y)), big.getpixel((x, y)))
            self.assertNotEqual(grid.tobytes(), big.tobytes())

    def test_no_labels_inside_scene_and_sheet_has_legend_band(self) -> None:
        sheet = self.image("comparison_sheet.png")
        self.assertGreater(sheet.height, 360 * 3)
        scene = self.image("scene_640x360.png")
        # The 1x scene is pasted pixel-exactly into the sheet (no overlay painted on it).
        header_height = 12 + 20 * 2  # two header text lines above the 1x scene
        self.assertEqual(sheet.crop((16, header_height, 656, header_height + 360)).tobytes(),
                         scene.tobytes())
        self.assertEqual(sheet.crop((16, header_height + 376, 1296, header_height + 376 + 720)).tobytes(),
                         scene.resize((1280, 720), Image.Resampling.NEAREST).tobytes())

    def test_deterministic_and_includes_colours(self) -> None:
        again = self.dir / "again"
        sc.build(self.specs, self.reference, again)
        for path in sorted(self.out.iterdir()):
            self.assertEqual(path.read_bytes(), (again / path.name).read_bytes(), path.name)
        for a in self.report["actors"]:
            self.assertEqual(a["color_count"], len(a["colors_used"]))
            self.assertGreaterEqual(a["color_count"], 2)

    def test_default_reference_is_52px_mannequin_labelled_not_carl(self) -> None:
        if not (ROOT / "generated/art/height_mannequin.png").is_file():
            self.skipTest("height_mannequin export missing")
        out = self.dir / "default_ref"
        png, meta = make_sprite(self.dir, "solo", 40)
        self.assertEqual(sc.main(["--sprite", f"Solo={png}:{meta}", "--out", str(out)]), 0)
        first = json.loads((out / "comparison.json").read_text(encoding="utf-8"))["actors"][0]
        self.assertTrue(first["reference"])
        self.assertEqual(first["label"], "Carl-height proxy (neutral 52 px mannequin, NOT Carl)")
        self.assertEqual(first["metadata"], "generated/art/height_mannequin.json")
        self.assertEqual(first["opaque_height_px"], 52)
        self.assertEqual(first["scene_bottom_opaque_row"], FLOOR_ROW)


class ScaleComparisonInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)

    def test_bad_specs_rejected(self) -> None:
        for spec in ("nolabel.png:a.json", "=a.png:a.json", "L=a.png", "L=a.png:a.json:x"):
            with self.assertRaises(sc.SceneError, msg=spec):
                sc.parse_spec(spec)
        self.assertEqual(sc.parse_spec("L 1=a.png:a.json:2"), ("L 1", "a.png", "a.json", 2))

    def test_sprite_count_limits(self) -> None:
        with self.assertRaisesRegex(sc.SceneError, "1 to 4"):
            sc.build([], None, self.dir / "o")
        png, meta = make_sprite(self.dir, "s", 32)
        with self.assertRaisesRegex(sc.SceneError, "1 to 4"):
            sc.build([f"S{n}={png}:{meta}" for n in range(5)], None, self.dir / "o")

    def test_partial_alpha_rejected(self) -> None:
        png, meta = make_sprite(self.dir, "soft", 32)
        image = Image.open(png).convert("RGBA")
        image.putpixel((12, 10), (10, 10, 10, 128))
        image.save(png)
        with self.assertRaisesRegex(sc.SceneError, "partially transparent"):
            sc.load_actor("soft", png, meta, 0)

    def test_frame_out_of_range_rejected(self) -> None:
        png, meta = make_sprite(self.dir, "one", 32)
        with self.assertRaisesRegex(sc.SceneError, "frame 3"):
            sc.load_actor("one", png, meta, 3)


if __name__ == "__main__":
    unittest.main()
