"""CARL-GAUNTLET-01: the four gauntlet overlay sheets of Carl (grids, keys, layering rules, manifests, export).

Spec: docs/02-art/carl-gauntlet-01-spec.md. The overlay is one ``gauntlet`` layer per Carl body sheet, same cells,
pivot, frame order, tags and durations. These tests check the 98 committed frames against the body sheets, the
three frozen Opus keys, the layering rules CG-L1..CG-L9 (through the toolkit validators), the manifests and the
exported atlases (run ``bash scripts/export_assets.sh`` first).
"""
from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_gauntlet as author  # noqa: E402
import pixel_grid_to_ase as pga  # noqa: E402
from scripts.pixelart import validators as val  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

SHEETS = author.SHEETS
FRAMES = {"locomotion": 16, "melee": 24, "cf02": 13, "dynamite": 45}
KEYS = ROOT / "tests/fixtures/art-carl-gauntlet_keys.json"
KEY_GRID = author.KEYS_GRID
ALLOWED = set("KJjTUVwH")
# the seven orphans that the approved melee frames 4 and 5 already carry (spec section 9, lint note)
INHERITED_ORPHANS = {
    ("melee", 4): {(18, 11), (18, 14), (24, 19)},
    ("melee", 5): {(27, 23), (27, 29), (30, 20), (33, 26)},
}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def grids(sheet: str) -> tuple[dict, dict]:
    return pga.load_grid(author.body_path(sheet)), pga.load_grid(author.overlay_path(sheet))


def manifest(sheet: str) -> dict:
    return load_json(ROOT / f"assets/manifests/carl_book1_h52_{sheet}_gauntlet.json")


def config(sheet: str):
    return load_config(ROOT / f"assets/source/characters/carl/carl_h52_{sheet}_gauntlet.pixelart.json")


class ShapeTests(unittest.TestCase):
    def test_98_frames_in_four_sheets_with_the_body_shape_tags_and_durations(self) -> None:
        total = 0
        for sheet in SHEETS:
            body, over = grids(sheet)
            self.assertEqual(len(over["frames"]), FRAMES[sheet], sheet)
            total += len(over["frames"])
            self.assertEqual(over["canvas"], body["canvas"], sheet)
            self.assertEqual(over["canvas"], [48, 56], sheet)
            self.assertEqual(over["pivot"], body["pivot"], sheet)
            self.assertEqual(over["pivot"], [25, 55], sheet)
            self.assertEqual(over["tags"], body["tags"], sheet)
            self.assertEqual([f["duration_ms"] for f in over["frames"]],
                             [f["duration_ms"] for f in body["frames"]], sheet)
            self.assertEqual(over["layers"], ["gauntlet"], f"{sheet}: the overlay file holds the gauntlet layer only")
        self.assertEqual(total, 98)

    def test_only_the_eight_gauntlet_keys_are_declared_and_used_and_no_skin(self) -> None:
        for sheet in SHEETS:
            _, over = grids(sheet)
            self.assertEqual({e["key"] for e in over["palette"]}, ALLOWED, sheet)
            used = {ch for f in over["frames"] for row in f["cels"]["gauntlet"] for ch in row} - {"."}
            self.assertLessEqual(used, ALLOWED, sheet)
            self.assertFalse(used & set("LMSD"), sheet)

    def test_the_overlay_palette_equals_the_body_palette_entries(self) -> None:
        for sheet in SHEETS:
            body, over = grids(sheet)
            by_key = {e["key"]: e for e in body["palette"]}
            for entry in over["palette"]:
                self.assertEqual(entry, by_key[entry["key"]], f"{sheet} {entry['key']}")

    def test_the_committed_grids_are_what_the_authoring_script_writes(self) -> None:
        keys = author.key_cels()
        for sheet in SHEETS:
            built, _ = author.build_overlay(sheet, keys)
            self.assertEqual(built, load_json(author.overlay_path(sheet)), sheet)

    def test_grids_use_indent_one_like_the_body_sheets(self) -> None:
        for sheet in SHEETS:
            text = author.overlay_path(sheet).read_text(encoding="utf-8")
            self.assertEqual(text, json.dumps(json.loads(text), indent=1) + "\n", sheet)


class FrozenKeyTests(unittest.TestCase):
    def test_three_keys_are_in_the_overlays_byte_for_byte(self) -> None:
        fixture = load_json(KEYS)
        self.assertEqual(fixture["format"], "pixelart-keys-v1")
        self.assertEqual(fixture["layers"], ["gauntlet"])
        self.assertEqual(sorted(fixture["keys"]), ["idle_hand", "punch_b_contact", "punch_b_windup"])
        key_grid = load_json(KEY_GRID)
        tags = [t["name"] for t in key_grid["tags"]]
        for name, entry in fixture["keys"].items():
            _, over = grids(entry["sheet"])
            cel = over["frames"][entry["frame"]]["cels"]["gauntlet"]
            self.assertEqual(cel, key_grid["frames"][tags.index(name)]["cels"]["gauntlet"], name)
            self.assertEqual(val.frame_digest({"gauntlet": cel}, ["gauntlet"]), entry["sha256"], name)

    def test_the_key_file_still_has_the_frozen_cels(self) -> None:
        key_grid = load_json(KEY_GRID)
        fixture = load_json(KEYS)
        for i, tag in enumerate(key_grid["tags"]):
            self.assertEqual(val.frame_digest(key_grid["frames"][i]["cels"], ["gauntlet"]),
                             fixture["keys"][tag["name"]]["sha256"], tag["name"])

    def test_a_changed_key_pixel_breaks_the_freeze(self) -> None:
        entry = load_json(KEYS)["keys"]["idle_hand"]
        _, over = grids("locomotion")
        cel = list(over["frames"][0]["cels"]["gauntlet"])
        row = cel[35]
        cel[35] = row[:20] + ("U" if row[20] != "U" else "T") + row[21:]
        self.assertNotEqual(val.frame_digest({"gauntlet": cel}, ["gauntlet"]), entry["sha256"])


class LayeringTests(unittest.TestCase):
    """CG-L1..CG-L9 on every frame of the four sheets."""

    def test_every_frame_passes_every_rule(self) -> None:
        checked = 0
        for sheet in SHEETS:
            body, over = grids(sheet)
            params = val.overlay_params({"body_grid": "x"})
            exceptions = {int(k): v for k, v in manifest(sheet)["overlay"]["skin_exceptions"].items()}
            for i, frame in enumerate(over["frames"]):
                rows = frame["cels"]["gauntlet"]
                found = val.overlay_frame_findings(body["frames"][i]["cels"], rows, body["canvas"], params,
                                                   exceptions.get(i, ()))
                found["CG-L8"] = val.overlay_l8(body, rows, i, "gauntlet")[0]
                for rule, msgs in found.items():
                    self.assertEqual(msgs, [], f"{sheet}[{i}] {rule}")
                checked += 1
        self.assertEqual(checked, 98)

    def test_cg_l8_is_body_plus_overlay_minus_body_and_inherited_orphans_are_reported(self) -> None:
        body, over = grids("melee")
        for (sheet, frame), expected in INHERITED_ORPHANS.items():
            failures, inherited = val.overlay_l8(body, over["frames"][frame]["cels"]["gauntlet"], frame, "gauntlet")
            self.assertEqual(failures, [], f"{sheet}[{frame}]: the gauntlet adds no orphan")
            self.assertTrue(expected <= set(inherited), f"{sheet}[{frame}] inherited {sorted(inherited)}")

    def test_no_overlay_frame_is_empty_and_the_empty_list_is_recorded(self) -> None:
        for sheet in SHEETS:
            _, over = grids(sheet)
            empty = [i for i, f in enumerate(over["frames"]) if not any(set(r) != {"."} for r in f["cels"]["gauntlet"])]
            self.assertEqual(empty, manifest(sheet)["overlay"]["empty_frames"], sheet)
            self.assertEqual(empty, [], f"{sheet}: every body frame shows near-hand skin, so none is empty (CG-L9)")

    def test_skin_exceptions_are_near_arm_skin_not_on_the_hand(self) -> None:
        listed = 0
        for sheet in SHEETS:
            body, _ = grids(sheet)
            for frame, pts in manifest(sheet)["overlay"]["skin_exceptions"].items():
                near = val.cel_points(body["frames"][int(frame)]["cels"]["near_arm"])
                for x, y in pts:
                    self.assertIn(near.get((x, y)), "LMSD", f"{sheet}[{frame}] ({x}, {y})")
                    listed += 1
        self.assertEqual(listed, 1 + 2 + 7, "melee 5, melee 22, dynamite 6 and 39..44")

    def test_the_gauntlet_covers_every_near_hand_patch_and_stays_on_the_near_hand(self) -> None:
        for sheet in SHEETS:
            body, over = grids(sheet)
            for i, frame in enumerate(over["frames"]):
                cels = body["frames"][i]["cels"]
                pts = val.cel_points(frame["cels"]["gauntlet"])
                near = val.cel_points(cels["near_arm"])
                far = val.cel_points(cels.get("far_arm"))
                # most pixels sit on the near arm, none is far from it (CG-AD2: the far arm stays bare)
                on_near = sum(1 for p in pts if p in near)
                self.assertGreaterEqual(on_near, len(pts) - 12, f"{sheet}[{i}]")
                self.assertTrue(pts, f"{sheet}[{i}]")
                self.assertFalse(any(p in far and p not in near for p in pts if p not in val.dilate1(set(near), 48, 56)),
                                 f"{sheet}[{i}]")


class ToolkitTests(unittest.TestCase):
    def test_the_toolkit_validators_pass_on_each_overlay_config(self) -> None:
        for sheet in SHEETS:
            cfg = config(sheet)
            report = val.run(val.build_context(cfg))
            self.assertTrue(report["passed"], f"{sheet}: {val.summary(report)}")
            by_id = {r["id"]: r["status"] for r in report["rules"]}
            self.assertEqual(by_id["V18"], "pass", sheet)
            self.assertEqual(report["frames"], FRAMES[sheet])

    def test_a_pixel_on_the_head_fails_v18_through_the_config(self) -> None:
        sheet = "melee"
        cfg = config(sheet)
        grid = copy.deepcopy(cfg.load_grid())
        body = pga.load_grid(author.body_path(sheet))
        head = val.cel_points(body["frames"][4]["cels"]["head"])
        near = val.cel_points(body["frames"][4]["cels"]["near_arm"])
        x, y = sorted(p for p in head if p not in near and p in val.dilate1(set(near), 48, 56))[0]
        rows = [list(r) for r in grid["frames"][4]["cels"]["gauntlet"]]
        rows[y][x] = "U"
        grid["frames"][4]["cels"]["gauntlet"] = ["".join(r) for r in rows]
        report = val.run(val.build_context(cfg, grid=grid))
        self.assertFalse(report["passed"])
        failures = [f for r in report["rules"] if r["id"] == "V18" for f in r["failures"]]
        self.assertTrue(any("CG-L3" in f and "frame 4" in f for f in failures), failures)


class ManifestTests(unittest.TestCase):
    def test_manifests_are_overlays_of_the_body_manifests_with_provenance(self) -> None:
        for sheet in SHEETS:
            m = manifest(sheet)
            body_m = load_json(ROOT / f"assets/manifests/carl_book1_h52_{sheet}.json")
            self.assertEqual(m["id"], f"carl_book1_h52_{sheet}_gauntlet_candidate_v1")
            self.assertEqual(m["overlay_of"], body_m["id"])
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            self.assertEqual(m["frame_size"], body_m["frame_size"])
            self.assertEqual(m["pivot"], body_m["pivot"])
            self.assertEqual(m["animation_tags"], body_m["animation_tags"])
            self.assertEqual(m["output_atlas"], f"generated/art/carl_book1_h52_{sheet}_gauntlet.png")
            self.assertEqual(m["output_metadata"], f"generated/art/carl_book1_h52_{sheet}_gauntlet.json")
            self.assertEqual(m["license"], body_m["license"])
            self.assertIn("ART-PUBLICATION-ALL-V1", m["license"])
            self.assertIn("Claude Opus", m["provenance"]["generator"])
            self.assertIn("Claude Sonnet", m["provenance"]["generator"])
            self.assertIn("scripts/author_carl_gauntlet.py", m["provenance"]["generator"])
            self.assertEqual(m["provenance"]["input_reference_ids"], ["CARL-CONCEPT-APPROVED-V1"])
            self.assertTrue((ROOT / m["source_path"]).is_file(), m["source_path"])
            self.assertEqual(m["overlay"]["body_grid"], f"assets/source/characters/carl/carl_h52_{sheet}.grid.json")
            self.assertEqual(m["overlay"]["layer"], "gauntlet")

    def test_the_ase_sources_are_the_deterministic_build_of_the_grids(self) -> None:
        for sheet in SHEETS:
            _, over = grids(sheet)
            ase = (ROOT / manifest(sheet)["source_path"]).read_bytes()
            self.assertEqual(ase, pga.grid_to_ase(over), sheet)

    def test_the_four_manifests_are_the_only_gauntlet_manifests(self) -> None:
        found = sorted(p.name for p in (ROOT / "assets/manifests").glob("*_gauntlet.json"))
        self.assertEqual(found, [f"carl_book1_h52_{s}_gauntlet.json" for s in sorted(SHEETS)])


class AtlasTests(unittest.TestCase):
    def test_exported_overlay_atlases_have_the_body_atlas_size_and_cell_grid(self) -> None:
        for sheet in SHEETS:
            png = ROOT / f"generated/art/carl_book1_h52_{sheet}_gauntlet.png"
            meta = ROOT / f"generated/art/carl_book1_h52_{sheet}_gauntlet.json"
            body_png = ROOT / f"generated/art/carl_book1_h52_{sheet}.png"
            body_meta = ROOT / f"generated/art/carl_book1_h52_{sheet}.json"
            if not (png.is_file() and meta.is_file() and body_png.is_file()):
                self.skipTest("run bash scripts/export_assets.sh first")
            with Image.open(png) as a, Image.open(body_png) as b:
                self.assertEqual(a.size, b.size, sheet)
            frames_a = [f["frame"] for f in load_json(meta)["frames"]]
            frames_b = [f["frame"] for f in load_json(body_meta)["frames"]]
            self.assertEqual(frames_a, frames_b, f"{sheet}: same cell rectangles")
            self.assertEqual(len(frames_a), FRAMES[sheet])
            self.assertEqual(load_json(meta)["meta"]["frameTags"], load_json(body_meta)["meta"]["frameTags"])

    def test_atlas_cells_hold_exactly_the_grid_pixels(self) -> None:
        for sheet in SHEETS:
            png = ROOT / f"generated/art/carl_book1_h52_{sheet}_gauntlet.png"
            meta = ROOT / f"generated/art/carl_book1_h52_{sheet}_gauntlet.json"
            if not (png.is_file() and meta.is_file()):
                self.skipTest("run bash scripts/export_assets.sh first")
            _, over = grids(sheet)
            palette = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in over["palette"]}
            with Image.open(png) as image:
                atlas = image.convert("RGBA")
            for i, entry in enumerate(load_json(meta)["frames"]):
                r = entry["frame"]
                for y, row in enumerate(over["frames"][i]["cels"]["gauntlet"]):
                    for x, ch in enumerate(row):
                        got = atlas.getpixel((r["x"] + x, r["y"] + y))
                        want = palette[ch] if ch != "." else None
                        if want is None:
                            self.assertEqual(got[3], 0, f"{sheet}[{i}] ({x}, {y})")
                        else:
                            self.assertEqual(got, want, f"{sheet}[{i}] ({x}, {y})")


if __name__ == "__main__":
    unittest.main()
