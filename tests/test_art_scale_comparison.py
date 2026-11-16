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
        with self.assertRaisesRegex(sc.SceneError, "At least one candidate"):
            sc.build([], None, self.dir / "o")
        png, meta = make_sprite(self.dir, "s", 32)
        with self.assertRaisesRegex(sc.SceneError, "1 to 6"):
            sc.build([f"S{n}={png}:{meta}" for n in range(7)], None, self.dir / "o")
        with self.assertRaisesRegex(sc.SceneError, "1 to 6"):
            sc.build([f"S{n}={png}:{meta}" for n in range(4)], None, self.dir / "o",
                     [f"C{n}={png}:{meta}" for n in range(3)])
        with self.assertRaisesRegex(sc.SceneError, "At least one candidate"):
            sc.build([], None, self.dir / "o", [f"C={png}:{meta}"])

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


def write_manifest(directory: Path, name: str, png: str, meta: str, study) -> None:
    manifest = {"id": name, "output_atlas": png, "output_metadata": meta}
    if study is not None:
        manifest["scale_study"] = study
    (directory / f"{name}.json").write_text(json.dumps(manifest), encoding="utf-8")


class GroupFixture(unittest.TestCase):
    """Temporary manifest directory with synthetic sprites (no tests of its own)."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.manifests = self.dir / "manifests"
        self.manifests.mkdir()

    def add(self, name: str, study, height: int = 32) -> None:
        png, meta = make_sprite(self.dir, name, height)
        write_manifest(self.manifests, name, png, meta, study)

    def groups(self) -> dict:
        return sc.collect_groups(self.manifests)


class ScaleStudyGroupTests(GroupFixture):
    """Manifest-declared scale_study groups (validation never needs the export)."""

    def test_no_scale_study_means_no_groups(self) -> None:
        self.add("plain", None)
        self.assertEqual(self.groups(), {})

    def test_object_and_list_forms_and_multiple_groups(self) -> None:
        self.add("a", {"group": "g1", "order": 2, "role": "candidate"})
        self.add("b", [{"group": "g1", "order": 1, "role": "candidate"},
                       {"group": "g2", "role": "companion"}])
        self.add("c", [{"group": "g2", "order": 5, "role": "candidate"}])
        groups = self.groups()
        self.assertEqual(sorted(groups), ["g1", "g2"])
        self.assertEqual([m["id"] for m in groups["g1"]], ["a", "b"])
        self.assertEqual({m["id"]: m["order"] for m in groups["g2"]}, {"b": 0, "c": 5})

    def test_invalid_declarations_rejected(self) -> None:
        cases = {
            "must match": {"group": "bad id!", "role": "candidate"},
            "must match ": {"group": "", "role": "candidate"},
            "role": {"group": "g", "role": "hero"},
            "order": {"group": "g", "role": "candidate", "order": "1"},
            "unknown keys": {"group": "g", "role": "candidate", "extra": 1},
            "non-empty list": [],
            "must be objects": ["g"],
            "same group twice": [{"group": "g", "role": "candidate"},
                                 {"group": "g", "role": "companion"}],
        }
        for pattern, study in cases.items():
            with self.subTest(pattern=pattern):
                for old in self.manifests.glob("*.json"):
                    old.unlink()
                self.add("m", study)
                with self.assertRaisesRegex(sc.SceneError, pattern.strip()):
                    self.groups()

    def test_group_needs_candidate_and_at_most_one_reference(self) -> None:
        self.add("ref", {"group": "g", "role": "reference"})
        self.add("comp", {"group": "g", "role": "companion"})
        with self.assertRaisesRegex(sc.SceneError, "'g' has no candidate"):
            self.groups()
        self.add("cand", {"group": "g", "role": "candidate"})
        self.assertEqual(sorted(self.groups()), ["g"])
        self.add("ref2", {"group": "g", "role": "reference"})
        with self.assertRaisesRegex(sc.SceneError, "more than one reference"):
            self.groups()

    def test_cli_reports_clear_errors_and_lists_groups(self) -> None:
        self.add("m", {"group": "g", "role": "nope"})
        self.assertEqual(sc.main(["--list-groups", "--manifest-dir", str(self.manifests)]), 1)
        (self.manifests / "m.json").unlink()
        self.add("m", {"group": "g", "role": "candidate"})
        self.assertEqual(sc.main(["--list-groups", "--manifest-dir", str(self.manifests)]), 0)
        self.assertEqual(sc.main(["--group", "missing", "--manifest-dir", str(self.manifests),
                                  "--out", str(self.dir / "o")]), 1)

    def test_repository_manifests_declare_neutral_fixture_group(self) -> None:
        groups = sc.collect_groups()
        members = {m["id"]: m["role"] for m in groups["neutral_fixture"]}
        self.assertEqual(members, {"neutral_blob_fixture_v0": "candidate",
                                   "height_mannequin_fixture_v0": "reference"})


@unittest.skipUnless(NEEDS_EXPORT, "run scripts/export_assets.sh first (dungeon_materials atlas)")
class ScaleStudyRenderTests(GroupFixture):
    def render(self, group: str = "g") -> dict:
        out = self.dir / "out"
        return sc.build_group(group, self.groups()[group], out)

    def test_order_roles_reference_and_shared_floor(self) -> None:
        self.add("ref", {"group": "g", "role": "reference"}, 52)
        self.add("tall", {"group": "g", "order": 3, "role": "candidate"}, 40)
        self.add("friend", {"group": "g", "order": 1, "role": "companion"}, 44)
        self.add("short", {"group": "g", "order": 2, "role": "candidate"}, 36)
        report = self.render()
        actors = report["actors"]
        self.assertEqual([a["label"] for a in actors], ["ref", "friend", "short", "tall"])
        self.assertEqual([a["role"] for a in actors],
                         ["reference", "companion", "candidate", "candidate"])
        self.assertEqual([a["opaque_height_px"] for a in actors], [52, 44, 36, 40])
        self.assertEqual({a["scene_bottom_opaque_row"] for a in actors}, {FLOOR_ROW})
        # Close-ups exist for candidates only; the companion appears in scene and legend.
        out = self.dir / "out"
        self.assertTrue((out / "closeup_short_6x.png").is_file())
        self.assertTrue((out / "closeup_tall_6x.png").is_file())
        self.assertFalse((out / "closeup_friend_6x.png").exists())
        self.assertEqual(Image.open(out / "scene_640x360.png").size, (640, 360))

    def test_default_reference_when_none_declared(self) -> None:
        if not (ROOT / "generated/art/height_mannequin.png").is_file():
            self.skipTest("height_mannequin export missing")
        self.add("solo", {"group": "g", "role": "candidate"}, 40)
        first = self.render()["actors"][0]
        self.assertEqual(first["role"], "reference")
        self.assertEqual(first["opaque_height_px"], 52)

    def test_six_actors_plus_reference_fit_evenly(self) -> None:
        self.add("ref", {"group": "g", "role": "reference"}, 52)
        for n in range(4):
            self.add(f"cand{n}", {"group": "g", "order": n, "role": "candidate"}, 32 + 4 * n)
        for n in range(2):
            self.add(f"comp{n}", {"group": "g", "order": 10 + n, "role": "companion"}, 40)
        actors = self.render()["actors"]
        self.assertEqual(len(actors), 7)
        centres = [a["scene_position"][0] + a["pivot"][0] for a in actors]
        gaps = {b - a for a, b in zip(centres, centres[1:])}
        self.assertLessEqual(max(gaps) - min(gaps), 1)

    def test_seven_non_reference_actors_rejected(self) -> None:
        for n in range(7):
            self.add(f"c{n}", {"group": "g", "order": n, "role": "candidate"})
        with self.assertRaisesRegex(sc.SceneError, "group 'g'.*1 to 6"):
            self.render()

    def add_wide(self, name: str, width: int, study: dict) -> None:
        png, meta = make_sprite(self.dir, name, 40)
        sprite = Image.open(png).convert("RGBA")
        canvas = Image.new("RGBA", (width, sprite.height), (0, 0, 0, 0))
        for x in range(0, width - 24 + 1, 24):
            canvas.alpha_composite(sprite, (x, 0))
        canvas.save(png)
        Path(meta).write_text(json.dumps({"frames": [{"frame": {"x": 0, "y": 0, "w": width,
                                                                  "h": sprite.height}}]}))
        write_manifest(self.manifests, name, png, meta, study)

    def test_actors_that_cannot_fit_in_640_px_are_rejected(self) -> None:
        self.add_wide("wide", 640, {"group": "g", "role": "candidate"})
        self.add("other", {"group": "g", "order": 1, "role": "candidate"})
        with self.assertRaisesRegex(sc.SceneError, "group 'g'.*(overlap|do(es)? not fit)"):
            self.render()

    def test_wide_actors_use_width_aware_fallback_without_overlap(self) -> None:
        # Even pivot spacing (every 160 px) would overlap the two 220 px wide actors.
        self.add_wide("wide1", 220, {"group": "g", "order": 1, "role": "candidate"})
        self.add_wide("wide2", 220, {"group": "g", "order": 2, "role": "candidate"})
        actors = self.render()["actors"]
        self.assertEqual(len(actors), 3)
        self.assertEqual({a["scene_bottom_opaque_row"] for a in actors}, {FLOOR_ROW})
        spans = [(a["scene_position"][0] + a["opaque_bbox"][0],
                  a["scene_position"][0] + a["opaque_bbox"][2]) for a in actors]
        self.assertGreaterEqual(spans[0][0], sc.MIN_GAP)
        self.assertLess(spans[-1][1], 640 - sc.MIN_GAP)
        gaps = [right[0] - left[1] - 1 for left, right in zip(spans, spans[1:])]
        gaps += [spans[0][0], 640 - spans[-1][1] - 1]
        self.assertGreaterEqual(min(gaps), sc.MIN_GAP)
        self.assertLessEqual(max(gaps) - min(gaps), 1)
        self.assertEqual(Image.open(self.dir / "out/scene_640x400.png").size, (640, 400))

    def test_group_render_is_deterministic(self) -> None:
        self.add("a", {"group": "g", "order": 1, "role": "candidate"}, 36)
        self.add("b", {"group": "g", "order": 2, "role": "companion"}, 44)
        first = self.dir / "one"
        second = self.dir / "two"
        sc.build_group("g", self.groups()["g"], first)
        sc.build_group("g", self.groups()["g"], second)
        for path in sorted(first.iterdir()):
            self.assertEqual(path.read_bytes(), (second / path.name).read_bytes(), path.name)


if __name__ == "__main__":
    unittest.main()
