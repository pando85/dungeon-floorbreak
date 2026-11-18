"""PIPE-01: the design packet, concept comparison and freeze command (smoke and unit tests)."""
from __future__ import annotations

import json
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from scripts.pixelart import cli, compare, packet, validators as v  # noqa: E402
from scripts.pixelart.common import ToolkitError  # noqa: E402
from scripts.pixelart.config import AssetConfig, load_config  # noqa: E402

CONCEPT = ROOT / "assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png"
GOBLIN_CONFIG = ROOT / "assets/source/characters/goblin/goblin_a_h36.pixelart.json"


class PacketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.tmp = Path(tempfile.mkdtemp())
        cls.out = cls.tmp / "packet"
        cls.files = packet.build_packet(
            "test-goblin", CONCEPT, [packet.parse_crop("0,0,360,365"), packet.parse_crop("360,0,360,365")], cls.out,
            heights=[34, 36, 40], excerpts=["docs/02-art/gob-01-goblin-spec.md#1. Canon"])

    @classmethod
    def tearDownClass(cls) -> None:
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def test_every_packet_file_is_written(self) -> None:
        self.assertEqual(sorted(f.name for f in self.files), sorted([
            "concept_crop_1.png", "concept_crop_2.png", "palette_candidates.json", "palette_swatches.png",
            "scale_lineup.png", "frame_pivot_template.json", "spec_template.md", "excerpts.md"]))
        self.assertEqual(Image.open(self.out / "concept_crop_1.png").size, (360, 365))

    def test_palette_candidates_are_at_most_24_hex_colours_per_crop(self) -> None:
        cands = json.loads((self.out / "palette_candidates.json").read_text(encoding="utf-8"))
        self.assertEqual(len(cands["crops"]), 2)
        for colours in [c["colours"] for c in cands["crops"]] + [cands["merged"]]:
            self.assertTrue(1 <= len(colours) <= 24)
            self.assertTrue(all(re.fullmatch(r"#[0-9a-f]{6}", c["hex"]) for c in colours))
            self.assertAlmostEqual(sum(c["share"] for c in colours), 1.0, delta=0.01)
            self.assertEqual([c["share"] for c in colours], sorted((c["share"] for c in colours), reverse=True))

    def test_palette_is_deterministic_and_capped(self) -> None:
        img = Image.open(CONCEPT).convert("RGBA")
        pixels = packet.opaque_pixels(img)
        self.assertEqual(packet.quantize(pixels, 8), packet.quantize(pixels, 8))
        self.assertLessEqual(len(packet.quantize(pixels, 99)), 24)
        self.assertEqual(packet.quantize([], 8), [])
        two = [(255, 0, 0)] * 30 + [(0, 0, 255)] * 10
        self.assertEqual([c["hex"] for c in packet.quantize(two, 2)], ["#ff0000", "#0000ff"])

    def test_transparent_pixels_are_ignored(self) -> None:
        img = Image.new("RGBA", (4, 1), (0, 0, 0, 0))
        img.putpixel((0, 0), (10, 20, 30, 255))
        self.assertEqual(packet.opaque_pixels(img), [(10, 20, 30)])

    def test_scale_lineup_has_two_scales_and_the_three_references(self) -> None:
        sheet = Image.open(self.out / "scale_lineup.png")
        self.assertGreater(sheet.width, 1000)
        self.assertGreater(sheet.height, 350)
        sizes = {name: h for name, _img, _piv, h in packet.reference_sprites()}
        self.assertEqual(sizes, {"carl": 52, "donut": 40, "goblin": 36})

    def test_frame_pivot_template_is_a_valid_config_with_the_readability_set(self) -> None:
        cfg = AssetConfig(json.loads((self.out / "frame_pivot_template.json").read_text(encoding="utf-8")))
        self.assertEqual(set(cfg.data["roles"]), {"idle", "move", "tell", "attack", "hurt", "defeat"})
        self.assertEqual(cfg.frame_count, sum(packet.PROPOSED_FRAMES.values()))
        self.assertTrue(12 <= cfg.frame_count <= 20)
        self.assertEqual(cfg.ticks_of(cfg.pose("tell")), 18)
        self.assertEqual([t for t, s in cfg.tags.items() if s["playback"] == "hold"], ["down"])

    def test_spec_template_follows_the_gob01_sections(self) -> None:
        text = (self.out / "spec_template.md").read_text(encoding="utf-8")
        self.assertTrue(text.startswith("# TEST-GOBLIN: design spec (template)"))
        headings = re.findall(r"^## (\d+)\. (.+)$", text, flags=re.M)
        self.assertEqual([int(n) for n, _ in headings], list(range(1, 13)))
        for needle in ("Canon and tone", "Size decision", "Frame, pivot", "Readability-set animation list", "State diagram",
                       "Palette", "Key poses", "In-between list", "Validators", "Evidence", "Files", "Rights and provenance",
                       "stateDiagram-v2", "60 Hz", "| A | 34 px", "`attack_tell`", "frozen"):
            self.assertIn(needle, text)
        self.assertNotIn("TODO_FILL_ME", text)
        self.assertTrue(text.endswith("\n"))

    def test_excerpt_takes_the_matching_section_only(self) -> None:
        text = (self.out / "excerpts.md").read_text(encoding="utf-8")
        self.assertIn("Common goblins are green", text)
        self.assertNotIn("Size study and decision", text)

    def test_bad_inputs_raise_toolkit_errors(self) -> None:
        for bad in ("1,2,3", "a,b,c,d", "0,0,0,5", "-1,0,5,5"):
            with self.assertRaises(ToolkitError):
                packet.parse_crop(bad)
        with self.assertRaises(ToolkitError):
            packet.build_packet("Bad Name", CONCEPT, [], self.tmp / "x")
        with self.assertRaises(ToolkitError):
            packet.build_packet("ok-name", CONCEPT, [(0, 0, 9999, 9999)], self.tmp / "x")
        with self.assertRaises(ToolkitError):
            packet.build_packet("ok-name", self.tmp / "missing.png", [], self.tmp / "x")
        with self.assertRaises(ToolkitError):
            packet.excerpt("docs/02-art/gob-01-goblin-spec.md#No Such Heading")

    def test_whole_image_packet_caps_the_crop_size(self) -> None:
        out = self.tmp / "whole"
        packet.build_packet("whole-image", CONCEPT, [], out)
        self.assertLessEqual(max(Image.open(out / "concept_crop_1.png").size), packet.CONCEPT_LONG_SIDE)

    def test_cli_packet_command(self) -> None:
        out = self.tmp / "cli"
        self.assertEqual(cli.main(["packet", "cli-asset", "--concept", str(CONCEPT), "--crop", "0,0,200,200",
                                   "--out", str(out), "--colors", "12"]), 0)
        cands = json.loads((out / "palette_candidates.json").read_text(encoding="utf-8"))
        self.assertLessEqual(len(cands["merged"]), 12)
        self.assertEqual(cli.main(["packet", "Bad Id", "--concept", str(CONCEPT), "--out", str(out)]), 2)


class CompareAndFreezeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def test_compare_renders_concept_next_to_the_first_frame_of_every_tag(self) -> None:
        grid = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
        sheet = compare.compare_sheet(CONCEPT, [grid], scale=2, title="t")
        self.assertGreater(sheet.width, 400)
        names = [n for n, _ in compare.sample_frames(compare.resolve_source(grid)[1], False)]
        self.assertEqual(len(names), 11)
        self.assertEqual(names[0], "idle f0")
        self.assertEqual(len(compare.sample_frames(compare.resolve_source(grid)[1], True)), 27)

    def test_compare_reads_ase_and_asset_configs_without_a_third_party_reader(self) -> None:
        ase = ROOT / "assets/source/characters/goblin/goblin_a_h36.ase"
        name, grid = compare.resolve_source(ase)
        self.assertEqual((name, len(grid["frames"])), ("goblin_a_h36", 27))
        self.assertEqual(compare.resolve_source(GOBLIN_CONFIG)[0], "gob-01-goblin-a")
        self.assertNotIn("aseprite_reader", sys.modules)
        text = (ROOT / "scripts/pixelart/compare.py").read_text(encoding="utf-8")
        self.assertNotIn("/tmp/", text)
        self.assertNotIn("site-packages", text)

    def test_compare_command_and_presets(self) -> None:
        out = self.tmp / "cmp.png"
        self.assertEqual(cli.main(["compare", "--preset", "goblin", "--out", str(out), "--scale", "2"]), 0)
        self.assertTrue(out.is_file())
        for name in compare.PRESETS:
            concept, sources = compare.preset_inputs(name)
            self.assertTrue(concept.is_file(), name)
            self.assertTrue(sources, name)
        with self.assertRaises(ToolkitError):
            compare.compare_sheet(None, [])
        with self.assertRaises(ToolkitError):
            compare.preset_inputs("nobody")

    def test_freeze_writes_digests_and_refuses_to_overwrite(self) -> None:
        out = self.tmp / "keys.json"
        self.assertEqual(cli.main(["freeze", str(GOBLIN_CONFIG), "--frames", "0,11", "--out", str(out)]), 0)
        fixture = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(sorted(fixture["keys"]), ["0", "11"])
        self.assertEqual(fixture["layers"], ["body", "weapon"])
        cfg = load_config(GOBLIN_CONFIG)
        cfg.data["rules"]["frozen_keys"]["frames"] = [0, 11]
        report = v.run(v.build_context(cfg, fixture=fixture))
        self.assertEqual(next(r["status"] for r in report["rules"] if r["id"] == "V09"), "pass")
        self.assertEqual(cli.main(["freeze", str(GOBLIN_CONFIG), "--frames", "0", "--out", str(out)]), 2)
        self.assertEqual(cli.main(["freeze", str(GOBLIN_CONFIG), "--frames", "0", "--out", str(out), "--force"]), 0)
        self.assertEqual(cli.main(["freeze", str(GOBLIN_CONFIG), "--out", str(self.tmp / "all.json")]), 0)
        self.assertEqual(len(json.loads((self.tmp / "all.json").read_text(encoding="utf-8"))["keys"]), 11)

    def test_validate_command_writes_reports_and_reports_failure(self) -> None:
        out = self.tmp / "rep"
        self.assertEqual(cli.main(["validate", str(GOBLIN_CONFIG), "--out", str(out)]), 0)
        self.assertTrue((out / "validator_report.json").is_file())
        self.assertTrue((out / "validator_summary.txt").is_file())
        data = json.loads(GOBLIN_CONFIG.read_text(encoding="utf-8"))
        data["rules"]["max_colors"] = 3
        bad = self.tmp / "bad.pixelart.json"
        bad.write_text(json.dumps(data), encoding="utf-8")
        self.assertEqual(cli.main(["validate", str(bad)]), 1)
        self.assertEqual(cli.main(["validate", str(ROOT / "README.md")]), 2)    # not JSON: a clean error, no traceback
        self.assertEqual(cli.main(["validate", str(ROOT / "docs/07-agents/cost-ledger.csv")]), 2)


if __name__ == "__main__":
    unittest.main()
