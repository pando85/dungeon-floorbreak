"""Tests for the deterministic pixel-grid -> .ase converter (IP-neutral fixtures)."""
from __future__ import annotations

import copy
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from PIL import Image

import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import pixel_grid_to_ase as pg  # noqa: E402

FIXTURE = ROOT / "tests" / "fixtures" / "neutral_blob_grid.json"


def fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


class GridFormatTests(unittest.TestCase):
    def test_fixture_is_valid(self) -> None:
        grid = pg.load_grid(FIXTURE)
        self.assertEqual(grid["canvas"], [20, 22])

    def test_round_trip_grid_ase_grid(self) -> None:
        grid = fixture()
        data = pg.grid_to_ase(grid)
        self.assertEqual(pg.ase_to_grid(data), pg.normalize(grid))

    def test_round_trip_drops_only_empty_cels(self) -> None:
        grid = fixture()
        h, w = grid["canvas"][1], grid["canvas"][0]
        grid["frames"][0]["cels"]["gear"] = ["." * w] * h  # all transparent
        again = pg.ase_to_grid(pg.grid_to_ase(grid))
        self.assertNotIn("gear", again["frames"][0]["cels"])
        self.assertIn("gear", again["frames"][1]["cels"])
        self.assertEqual(again["layers"], grid["layers"])
        self.assertEqual(again["pivot"], grid["pivot"])
        self.assertEqual([t["name"] for t in again["tags"]], ["idle"])

    def test_ase_is_byte_identical_across_runs(self) -> None:
        self.assertEqual(pg.grid_to_ase(fixture()), pg.grid_to_ase(fixture()))
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp, "a.ase"), Path(tmp, "b.ase")
            self.assertEqual(pg.main(["build", str(FIXTURE), str(a)]), 0)
            self.assertEqual(pg.main(["build", str(FIXTURE), str(b)]), 0)
            self.assertEqual(a.read_bytes(), b.read_bytes())

    def test_header_and_named_palette(self) -> None:
        data = pg.grid_to_ase(fixture())
        parsed = pg.read_ase(data)
        self.assertEqual(data[4:6], b"\xe0\xa5")
        self.assertEqual(int.from_bytes(data[12:14], "little"), 32)  # RGBA depth
        self.assertEqual(parsed["layers"], ["feet", "body", "face", "gear"])
        self.assertEqual([f["duration_ms"] for f in parsed["frames"]], [200, 200])
        self.assertEqual(parsed["palette"][0][0], ".:transparent")
        self.assertIn(("O:outline", (0x1c, 0x20, 0x30, 255)), parsed["palette"])

    def test_only_opaque_or_transparent_pixels(self) -> None:
        parsed = pg.read_ase(pg.grid_to_ase(fixture()))
        alphas = {px[3] for f in parsed["frames"] for rows in f["cels"].values()
                  for row in rows for px in row}
        self.assertEqual(alphas, {0, 255})


class DeterministicCompressionTests(unittest.TestCase):
    def test_stream_inflates_to_input(self) -> None:
        import random
        import zlib
        rng = random.Random(7)
        for data, stride in ((b"", 4), (bytes(2000), 40),
                             (bytes(rng.randrange(3) for _ in range(4000)), 40),
                             (bytes(rng.randrange(256) for _ in range(1500)), 40)):
            self.assertEqual(zlib.decompress(pg._zlib_deterministic(data, stride)), data)


class CommittedSourceTests(unittest.TestCase):
    def test_committed_ase_matches_its_grid(self) -> None:
        """Every manifest with grid_source must ship an .ase equal to a fresh build."""
        checked = 0
        for path in sorted((ROOT / "assets" / "manifests").glob("*.json")):
            manifest = json.loads(path.read_text(encoding="utf-8"))
            if not manifest.get("grid_source"):
                continue
            grid = pg.load_grid(ROOT / manifest["grid_source"])
            ase = ROOT / manifest["source_path"]
            self.assertTrue(ase.is_file(), f"{ase} missing; run scripts/export_assets.sh and commit it")
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(grid),
                             f"{manifest['source_path']} is stale versus {manifest['grid_source']}")
            checked += 1
        self.assertGreaterEqual(checked, 1)  # the neutral blob fixture at least


def tiny_grid(rows: list[str], accent: bool = False) -> dict:
    """Single-layer grid: K = body key, A = optional accent key; pivot on the last row."""
    return {"format": "pixel-grid-v1", "canvas": [len(rows[0]), len(rows)],
            "pivot": [0, len(rows) - 1],
            "palette": [{"key": "K", "name": "body", "rgba": "#304050ff"},
                        {"key": "A", "name": "accent", "rgba": "#ffd040ff", "accent": accent}],
            "layers": ["l"], "frames": [{"duration_ms": 100, "cels": {"l": rows}}], "tags": []}


class OrphanRuleTests(unittest.TestCase):
    def test_diagonal_staircase_is_not_an_orphan(self) -> None:
        report = pg.lint_grid(tiny_grid(["K.....", ".K....", "..K...", "...K..", "....K.", ".....K"]))
        self.assertEqual(report["frame0_orphan_count"], 0)
        self.assertEqual(report["frame0_isolated_count"], 0)
        self.assertEqual(pg.lint_problems({**report, "contact_row_matches_pivot": True,
                                           "every_frame_bottom_equals_pivot": True}), [])

    def test_accent_key_is_exempt_from_orphan_rule(self) -> None:
        rows = ["KKK.", "KKKA", "KKK.", "KKK."]  # A touches K only (orthogonally)
        report = pg.lint_grid(tiny_grid(rows, accent=True))
        self.assertEqual(report["frame0_orphan_count"], 0)
        self.assertEqual(report["accent_keys"], ["A"])

    def test_true_orphan_is_flagged(self) -> None:
        rows = ["KKK.", "KKKA", "KKK.", "KKK."]
        report = pg.lint_grid(tiny_grid(rows, accent=False))
        self.assertEqual(report["frames"][0]["orphan_pixels"], [[3, 1]])
        self.assertEqual(report["frame0_isolated_count"], 0)
        self.assertTrue(any("orphan" in p for p in pg.lint_problems(report)))
        self.assertEqual(pg.lint_errors(report), [])  # orphans are strict-only problems

    def test_isolated_pixel_is_always_an_error(self) -> None:
        rows = ["KK..", "KK..", "....", "...K"]
        for accent_key in (False, True):
            rows_a = rows[:3] + ["...A"]
            report = pg.lint_grid(tiny_grid(rows_a, accent=accent_key))
            self.assertEqual(report["frames"][0]["isolated_pixels"], [[3, 3]])
            self.assertEqual(report["frame0_orphan_count"], 0)
            self.assertTrue(pg.lint_errors(report))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp, "iso.json")
            path.write_text(json.dumps(tiny_grid(rows)), encoding="utf-8")
            self.assertEqual(pg.main(["lint", str(path)]), 1)  # fails without --strict

    def test_accent_is_optional_and_validated(self) -> None:
        g = fixture()
        self.assertNotIn("accent", next(e for e in pg.validate_grid(g)["palette"] if e["key"] == "O"))
        g["palette"][0]["accent"] = "yes"
        with self.assertRaisesRegex(pg.GridError, "accent"):
            pg.validate_grid(g)

    def test_accent_survives_ase_round_trip(self) -> None:
        grid = tiny_grid(["KKK.", "KKKA", "KKK.", "KKK."], accent=True)
        again = pg.ase_to_grid(pg.grid_to_ase(grid))
        self.assertEqual(again, pg.normalize(grid))
        self.assertTrue(next(e for e in again["palette"] if e["key"] == "A")["accent"])


class HeightMannequinTests(unittest.TestCase):
    def test_mannequin_is_exactly_52_px_and_clean(self) -> None:
        grid = pg.load_grid(ROOT / "assets/source/characters/placeholder/height_mannequin.grid.json")
        report = pg.lint_grid(grid)
        self.assertEqual(report["frame0_silhouette_height"], 52)
        self.assertTrue(report["contact_row_matches_pivot"])
        self.assertEqual(report["frame0_orphan_count"], 0)
        self.assertEqual(report["frame0_isolated_count"], 0)
        self.assertEqual(report["frame_count"], 1)
        self.assertEqual([t["name"] for t in grid["tags"]], ["idle"])
        self.assertLessEqual(len(report["colors_used"]), 3)  # plain neutral grey-blue


class GridRejectionTests(unittest.TestCase):
    def assertRejected(self, grid: dict, pattern: str) -> None:
        with self.assertRaisesRegex(pg.GridError, pattern):
            pg.validate_grid(grid)

    def test_partial_alpha_palette(self) -> None:
        g = fixture()
        g["palette"][0]["rgba"] = "#1c203080"
        self.assertRejected(g, "opaque")

    def test_bad_row_length(self) -> None:
        g = fixture()
        g["frames"][0]["cels"]["body"][3] = "..."
        self.assertRejected(g, "row 3.*chars")

    def test_bad_row_count(self) -> None:
        g = fixture()
        g["frames"][1]["cels"]["body"].pop()
        self.assertRejected(g, "rows")

    def test_unknown_key(self) -> None:
        g = fixture()
        row = g["frames"][0]["cels"]["body"][10]
        g["frames"][0]["cels"]["body"][10] = "Z" + row[1:]
        self.assertRejected(g, "unknown key 'Z'")

    def test_unknown_layer(self) -> None:
        g = fixture()
        g["frames"][0]["cels"]["ghost"] = g["frames"][0]["cels"]["body"]
        self.assertRejected(g, "unknown layer")

    def test_duplicate_keys_and_layers(self) -> None:
        g = fixture()
        g["palette"].append(dict(g["palette"][0]))
        self.assertRejected(g, "duplicate palette key")
        g = fixture()
        g["layers"].append("body")
        self.assertRejected(g, "duplicate layer")

    def test_reserved_dot_key(self) -> None:
        g = fixture()
        g["palette"][0]["key"] = "."
        self.assertRejected(g, "reserved")

    def test_tag_out_of_range(self) -> None:
        g = fixture()
        g["tags"][0]["to"] = 2
        self.assertRejected(g, "range")
        g = fixture()
        g["tags"][0]["from"], g["tags"][0]["to"] = 1, 0
        self.assertRejected(g, "range")

    def test_pivot_outside_canvas(self) -> None:
        for pivot in ([20, 21], [10, 22], [-1, 0]):
            g = fixture()
            g["pivot"] = pivot
            self.assertRejected(g, "pivot")

    def test_bad_duration_and_format(self) -> None:
        g = fixture()
        g["frames"][0]["duration_ms"] = 0
        self.assertRejected(g, "duration")
        g = fixture()
        g["format"] = "other"
        self.assertRejected(g, "format")

    def test_cli_reports_errors(self) -> None:
        g = fixture()
        g["pivot"] = [99, 99]
        with tempfile.TemporaryDirectory() as tmp:
            bad = Path(tmp, "bad.json")
            bad.write_text(json.dumps(g), encoding="utf-8")
            self.assertEqual(pg.main(["build", str(bad), str(Path(tmp, "x.ase"))]), 1)
            self.assertFalse(Path(tmp, "x.ase").exists())


class GridLintTests(unittest.TestCase):
    def test_height_bbox_and_contact(self) -> None:
        report = pg.lint_grid(fixture())
        self.assertEqual(report["frame0_silhouette_height"], 18)
        self.assertEqual(report["frame0_bbox"], [1, 4, 18, 21])
        self.assertTrue(report["contact_row_matches_pivot"])
        self.assertEqual(report["bottom_row_drift"], 0)
        self.assertEqual(report["colors_used"]["A"]["pixels"], 4)
        self.assertEqual(report["colors_declared_unused"], [])

    def test_breathing_frame_keeps_contact_but_changes_height(self) -> None:
        report = pg.lint_grid(fixture())
        self.assertEqual([f["height"] for f in report["frames"]], [18, 17])
        self.assertEqual([f["bottom_row"] for f in report["frames"]], [21, 21])

    def test_drift_and_pivot_mismatch_reported(self) -> None:
        g = fixture()
        g["frames"][1]["cels"]["feet"] = ["." * 20] * 22  # no feet -> body ends row 20
        g["frames"][1]["cels"]["body"][20] = "." * 20
        report = pg.lint_grid(g)
        self.assertGreaterEqual(report["bottom_row_drift"], 1)
        self.assertTrue(pg.lint_problems(report))
        g = fixture()
        g["pivot"] = [10, 20]
        report = pg.lint_grid(g)
        self.assertFalse(report["contact_row_matches_pivot"])
        self.assertTrue(any("pivot" in p for p in pg.lint_problems(report)))

    def test_cli_strict_and_expect_height(self) -> None:
        self.assertEqual(pg.main(["lint", str(FIXTURE), "--strict", "--expect-height", "18"]), 0)
        self.assertEqual(pg.main(["lint", str(FIXTURE), "--strict", "--expect-height", "40"]), 1)


class GridRenderTests(unittest.TestCase):
    def test_render_nearest_neighbour_on_background(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            paths = pg.render_frames(pg.load_grid(FIXTURE), Path(tmp), scale=4,
                                     bg="#102030", stem="blob")
            self.assertEqual([p.name for p in paths],
                             ["blob_f00.png", "blob_f01.png", "blob_strip.png"])
            with Image.open(paths[0]) as im:
                self.assertEqual(im.size, (80, 88))
                self.assertEqual(im.getpixel((0, 0)), (0x10, 0x20, 0x30, 255))
                self.assertEqual(len(im.getcolors(1000)), 8)  # 7 palette colours + bg
            with Image.open(paths[2]) as strip:
                self.assertEqual(strip.size, (160, 88))


@unittest.skipUnless(os.environ.get("ART_CLI"), "ART_CLI not set; LibreSprite/Aseprite export skipped")
class CliExportEqualityTests(unittest.TestCase):
    def test_libresprite_export_matches_grid_pixels(self) -> None:
        grid = pg.validate_grid(fixture())
        w, h = grid["canvas"]
        keys = {e["key"]: pg.parse_rgba(e["rgba"]) for e in grid["palette"]}
        with tempfile.TemporaryDirectory() as tmp:
            ase = Path(tmp, "blob.ase")
            pg.write_ase(grid, ase)
            sheet, data = Path(tmp, "blob.png"), Path(tmp, "blob.json")
            subprocess.run(
                [os.environ["ART_CLI"], "-b", "--list-tags", str(ase), "--sheet", str(sheet),
                 "--data", str(data), "--format", "json-array", "--sheet-type", "rows"],
                check=True, capture_output=True, timeout=120)
            meta = json.loads(data.read_text(encoding="utf-8"))
            self.assertEqual([f["duration"] for f in meta["frames"]], [200, 200])
            self.assertEqual(meta["meta"]["frameTags"],
                             [{"name": "idle", "from": 0, "to": 1, "direction": "forward"}])
            atlas = Image.open(sheet).convert("RGBA")
            for index, item in enumerate(meta["frames"]):
                r = item["frame"]
                self.assertEqual((r["w"], r["h"]), (w, h))
                crop = atlas.crop((r["x"], r["y"], r["x"] + w, r["y"] + h))
                flat = pg.composite_keys(grid, index)
                for y in range(h):
                    for x in range(w):
                        expected = (0, 0, 0, 0) if flat[y][x] == "." else keys[flat[y][x]]
                        got = crop.getpixel((x, y))
                        if expected[3] == 0:
                            self.assertEqual(got[3], 0, (index, x, y))
                        else:
                            self.assertEqual(got, expected, (index, x, y))


if __name__ == "__main__":
    unittest.main()
