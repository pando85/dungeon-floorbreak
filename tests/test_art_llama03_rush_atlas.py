"""LLAMA-03: the ten-frame Bad Llama rush atlas (the four frozen keys plus six in-betweens).

Spec: docs/02-art/llama-03-rush-spec.md. These tests check the committed atlas grid against its authoring script, the keys
against the key grid and the frozen fixture, the acceptance targets A1 to A9 over all ten frames, the manifest and the
exported atlas (when it has been exported). The toolkit validators run with
``python3 -m scripts.pixelart validate assets/source/characters/llama/rush/llama_rush_h64.pixelart.json``.
"""
from __future__ import annotations

import contextlib
import io
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

import author_llama01 as rig  # noqa: E402
import author_llama03_rush as author  # noqa: E402
import pixel_grid_to_ase as pga  # noqa: E402
from scripts.pixelart import validators as val  # noqa: E402

GRID = author.ATLAS_GRID
KEYS = author.KEYS_GRID
MANIFEST = ROOT / "assets/manifests/llama_rush_h64.json"
CONFIG = ROOT / "assets/source/characters/llama/rush/llama_rush_h64.pixelart.json"
LLAMA01 = ROOT / "assets/source/characters/llama/llama-01.grid.json"
EXPORTED = ROOT / "generated/art/llama_rush_h64.png"
NAMES = ["brace_in", "brace", "scrape", "coil", "run_contact", "run_gather", "run_push", "run_reach",
         "recover_skid", "recover_blown"]
GLOW = set("rRYO")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def body(grid: dict) -> list[list[str]]:
    return [f["cels"]["body"] for f in grid["frames"]]


def bbox(rows: list[str]) -> tuple[int, int, int, int]:
    pts = [(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def count(rows: list[str], key: str) -> int:
    return sum(r.count(key) for r in rows)


class AtlasContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = load(GRID)
        cls.frames = body(cls.grid)

    def test_the_grid_is_the_authoring_script_output(self) -> None:
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(author.main(["atlas", "--check"]), 0, "run: python3 scripts/author_llama03_rush.py atlas")

    def test_ten_frames_on_the_llama01_frame_pivot_and_palette(self) -> None:
        l1 = load(LLAMA01)
        self.assertEqual(len(self.frames), 10)
        self.assertEqual(self.grid["canvas"], [96, 72])
        self.assertEqual(self.grid["pivot"], [44, 71])
        self.assertEqual(self.grid["palette"], l1["palette"])
        self.assertEqual(self.grid["layers"], ["body"])
        self.assertEqual([(t["name"], t["from"], t["to"]) for t in self.grid["tags"]],
                         [("rush_brace", 0, 3), ("rush_run", 4, 7), ("rush_recover", 8, 9)])
        self.assertEqual([f["duration_ms"] for f in self.grid["frames"]], [67] * 4 + [50] * 4 + [167, 500])

    def test_the_ase_source_round_trips_to_the_grid(self) -> None:
        ase = pga.normalize(pga.ase_to_grid(author.ATLAS_ASE.read_bytes()))
        self.assertEqual([f["cels"] for f in ase["frames"]], [f["cels"] for f in pga.load_grid(GRID)["frames"]])

    def test_the_keys_are_the_key_grid_byte_for_byte(self) -> None:
        keys = body(load(KEYS))
        for i, atlas in enumerate((1, 2, 4, 8)):
            self.assertEqual(self.frames[atlas], keys[i], f"atlas frame {atlas} ({NAMES[atlas]})")

    def test_the_keys_match_the_frozen_fixture(self) -> None:
        fixture = load(author.FIXTURE)["keys"]
        for atlas in (1, 2, 4, 8):
            self.assertEqual(fixture[str(atlas)]["sha256"], author.digest(self.grid["frames"][atlas]["cels"]), atlas)

    def test_every_frame_is_unique(self) -> None:
        self.assertEqual(len({"".join(f) for f in self.frames}), 10)


class AtlasTargetTests(unittest.TestCase):
    """A1 to A9 of spec section 9 over all ten frames."""

    @classmethod
    def setUpClass(cls):
        cls.frames = body(load(GRID))

    def test_a2_no_glow_key(self) -> None:
        for i, rows in enumerate(self.frames):
            self.assertEqual(sum(count(rows, k) for k in GLOW), 0, NAMES[i])

    def test_a3_tops_and_grounding(self) -> None:
        for i, rows in enumerate(self.frames):
            top, bottom = bbox(rows)[1], bbox(rows)[3]
            self.assertEqual(bottom, 71, NAMES[i])
            self.assertGreaterEqual(top, {0: 14}.get(i, 20 if i < 8 else 28), NAMES[i])
            self.assertGreaterEqual(sum(r.count("h") for r in rows[68:72]), 3, f"{NAMES[i]}: a hoof stamp on the floor")

    def test_a4_the_head_points_down_the_lane_at_chest_height(self) -> None:
        for i in range(1, 8):
            ahead = [y for y, r in enumerate(self.frames[i]) for x in range(76, 96) if r[x] != "."]
            self.assertTrue(ahead, NAMES[i])
            self.assertGreaterEqual(min(ahead), 33, NAMES[i])
            self.assertLessEqual(max(ahead), 56, NAMES[i])
        for i in range(4, 8):
            self.assertGreaterEqual(bbox(self.frames[i])[2], 86, f"{NAMES[i]} reaches far ahead of the pivot")

    def test_a5_the_recover_head_is_low(self) -> None:
        for i in (8, 9):
            ahead = [y for y, r in enumerate(self.frames[i]) for x in range(76, 96) if r[x] != "."]
            self.assertGreaterEqual(min(ahead), 40, NAMES[i])

    def test_a6_identity_teeth_and_eye(self) -> None:
        for i, rows in enumerate(self.frames):
            self.assertGreaterEqual(count(rows, "T"), 6, NAMES[i])
            if i < 8:
                self.assertGreaterEqual(count(rows, "e"), 2, NAMES[i])
            else:
                self.assertEqual(count(rows, "e"), 0, f"{NAMES[i]}: the winded squint")

    def test_a8_orphans_isolated_and_pockets(self) -> None:
        for i, rows in enumerate(self.frames):
            orphans, isolated = pga._stray_pixels([list(r) for r in rows], rig.ACCENTS)
            self.assertLessEqual(len(orphans), 9, NAMES[i])
            self.assertEqual(len(isolated), 0, NAMES[i])
            for pocket in val.find_holes([list(r) for r in rows], 4):
                xs, ys = [p[0] for p in pocket], [p[1] for p in pocket]
                tail = max(xs) <= 24 and max(ys) <= 46          # the rig's tail-mane pinholes, as on LLAMA-01
                legs = len(pocket) <= 4 and min(ys) >= 64       # the gap between two touching hooves
                self.assertTrue(tail or legs, f"{NAMES[i]}: unexpected enclosed pocket {pocket}")

    def test_a1_the_palette_is_the_17_key_llama_palette(self) -> None:
        used = {ch for rows in self.frames for r in rows for ch in r} - {"."}
        self.assertLessEqual(len(used), 17)
        self.assertLessEqual(used, {e["key"] for e in load(LLAMA01)["palette"]})


class RunCycleTests(unittest.TestCase):
    """The B1 far legs: the run cycle never skates a planted hoof (world space, 12 px a frame)."""

    @classmethod
    def setUpClass(cls):
        cls.frames = body(load(GRID))

    def test_every_run_frame_keeps_a_hoof_on_the_floor(self) -> None:
        for i in range(4, 8):
            self.assertGreaterEqual(sum(r.count("h") for r in self.frames[i][68:72]), 3, NAMES[i])

    def test_the_scrape_hind_hooves_stay_on_the_brace_footprint(self) -> None:
        """Gate K2 / spec rule 5: rows 66 to 71, columns 0 to 39 of `scrape` equal `brace`."""
        brace, scrape = self.frames[1], self.frames[2]
        for y in range(66, 72):
            self.assertEqual(scrape[y][:40], brace[y][:40], f"row {y}")


class ManifestTests(unittest.TestCase):
    def test_the_manifest_names_the_atlas_and_the_tick_map(self) -> None:
        m = load(MANIFEST)
        self.assertEqual(m["id"], "llama_rush_h64_candidate_v1")
        self.assertEqual(m["source_path"], "assets/source/characters/llama/rush/llama_rush_h64.ase")
        self.assertEqual(m["grid_source"], "assets/source/characters/llama/rush/llama_rush_h64.grid.json")
        self.assertEqual(m["output_atlas"], "generated/art/llama_rush_h64.png")
        self.assertEqual(m["palette_id"], "llama-floor01-v0")
        self.assertEqual(m["animation_tags"], ["rush_brace", "rush_run", "rush_recover"])
        self.assertEqual(m["frame_size"], [96, 72])
        self.assertEqual(m["pivot"], [44, 71])
        self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
        poses = m["rush_poses"]
        self.assertEqual(poses["key_frames"], [1, 2, 4, 8])
        self.assertEqual(poses["tell"]["slots"], [0, 1, 2, 1, 2, 1, 2, 3])
        self.assertEqual(poses["committed"], {"cycle": [6, 7, 4, 5], "ticks_per_frame": 3})
        self.assertEqual(poses["recover"], [[0, 8], [10, 9]])
        self.assertEqual(poses["bonk"], [[0, "llama-01:11"], [12, 9]])

    def test_the_toolkit_config_validates(self) -> None:
        from scripts.pixelart import config as cfg_mod
        report = val.run(val.build_context(cfg_mod.load_config(CONFIG)))
        self.assertTrue(report["passed"], val.summary(report))
        self.assertEqual(report["counts"]["fail"], 0)


@unittest.skipUnless(EXPORTED.is_file(), "run scripts/export_assets.sh first")
class ExportedAtlasTests(unittest.TestCase):
    def test_the_exported_atlas_has_ten_frames_and_no_glow_pixel(self) -> None:
        from PIL import Image
        img = Image.open(EXPORTED).convert("RGBA")
        self.assertEqual(img.size, (96 * 10, 72))
        glow = set()
        for entry in load(GRID)["palette"]:
            if entry["key"] in GLOW:
                hexa = entry["rgba"].lstrip("#")
                glow.add(tuple(int(hexa[i:i + 2], 16) for i in (0, 2, 4)))
        self.assertEqual(len(glow), 4)
        raw = img.tobytes()
        hits = [tuple(raw[i:i + 3]) for i in range(0, len(raw), 4) if raw[i + 3] and tuple(raw[i:i + 3]) in glow]
        self.assertEqual(hits, [], "the rush atlas never carries a glow colour")
        meta = json.loads(EXPORTED.with_suffix(".json").read_text(encoding="utf-8"))
        tags = {t["name"]: (t["from"], t["to"]) for t in meta["meta"]["frameTags"]}
        self.assertEqual(tags, {"rush_brace": (0, 3), "rush_run": (4, 7), "rush_recover": (8, 9)})


if __name__ == "__main__":
    unittest.main()
