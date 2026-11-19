"""HUD-HYBRID-A-01 art: sources, atlas bounds, transparency, palette membership, frame counts.

Needs the exported atlas: run ``bash scripts/export_assets.sh`` first (CI does).
Checks structure and contract, not artistic quality.
"""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_hud_hybrid_a as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

SRC = ROOT / "assets/source/hud"
MANIFEST = ROOT / "assets/manifests/hud_hybrid_a_v1.json"
ATLAS = ROOT / "generated/art/hud_hybrid_a.png"
EVIDENCE = ROOT / "docs/evidence/hud-hybrid-a-01"

KEY_SHA256 = {
    "carl_portrait": "7d13f3d2a5b4ef4e604a3bcea2be4b558eb41b7da51e9769cd4d32361ecaf306",
    "system_crt_face": "b5ec5a51332bcb3dc6641a0dd95f08e7d2817bb812f16f4a63a9387cc8dc71a3",
}
REGION_NAMES = [
    "plate_frame", "portrait_carl", "portrait_donut", "notice_frame", "crt_idle", "crt_blink", "crt_notice",
    "tag_p1", "tag_p2", "tag_ai", "boss_emblem", "warning",
    "glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle", "glyph_square",
    "glyph_triangle", "socket_locked", "heart", "heart_cracked", "spark", "spark_depleted",
    "strip_loc_left", "strip_loc_mid", "strip_loc_right", "strip_boss_left", "strip_boss_mid",
    "strip_boss_right",
]
FRAME_COUNTS = {"hud_portrait_carl": 1, "hud_portrait_donut": 1, "hud_system_crt": 3, "hud_glyphs": 9,
                "hud_tags": 3, "hud_icons": 6, "hud_plate_frame": 1, "hud_notice_frame": 1,
                "hud_strip_3slice": 1}
SOURCE_GRIDS = sorted(FRAME_COUNTS)


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def rgba_set(grid: dict) -> set[str]:
    return {e["rgba"] for e in grid["palette"]}


def flat_rgba(grid: dict, frame: int) -> list[list[str | None]]:
    g = pg.validate_grid(grid)
    pal = {e["key"]: e["rgba"] for e in g["palette"]}
    return [[None if ch == "." else pal[ch] for ch in row] for row in pg.composite_keys(g, frame)]


class HudSources(unittest.TestCase):
    def test_keys_are_frozen_and_recorded(self) -> None:
        manifest = load(MANIFEST)
        for name in ("carl_portrait", "system_crt_face"):
            digest = hashlib.sha256((SRC / "keys" / f"{name}.grid.json").read_bytes()).hexdigest()
            self.assertEqual(manifest["provenance"]["key_sha256"][name], digest, name)
            self.assertEqual(KEY_SHA256[name], digest, f"{name}: frozen key changed")

    def test_authoring_script_reproduces_committed_files(self) -> None:
        self.assertEqual(author.write_all(check=True), 0)

    def test_every_source_grid_passes_strict_lint(self) -> None:
        for name in SOURCE_GRIDS + ["hud_hybrid_a"]:
            grid = load(SRC / f"{name}.grid.json")
            report = pg.lint_grid(grid)
            self.assertEqual(pg.lint_problems(report), [], name)

    def test_frame_counts(self) -> None:
        for name, count in FRAME_COUNTS.items():
            self.assertEqual(len(load(SRC / f"{name}.grid.json")["frames"]), count, name)
        crt = load(SRC / "hud_system_crt.grid.json")
        self.assertEqual([t["name"] for t in crt["tags"]], ["idle", "blink", "notice"])

    def test_keys_keep_every_pixel_colour_and_position(self) -> None:
        for key_name, asset, frames in (("carl_portrait", "hud_portrait_carl", 1),
                                        ("system_crt_face", "hud_system_crt", 3)):
            key = load(SRC / "keys" / f"{key_name}.grid.json")
            made = load(SRC / f"{asset}.grid.json")
            self.assertEqual(made["canvas"], key["canvas"])
            for f in range(frames):
                self.assertEqual(flat_rgba(made, f), flat_rgba(key, f), f"{asset} frame {f}")

    def test_portrait_palettes_come_from_the_sprite_masters(self) -> None:
        carl = rgba_set(load(author.CARL_MASTER))
        donut = rgba_set(load(author.DONUT_MASTER))
        self.assertLessEqual(rgba_set(load(SRC / "hud_portrait_carl.grid.json")), carl)
        self.assertLessEqual(rgba_set(load(SRC / "hud_portrait_donut.grid.json")), donut)

    def test_donut_portrait_is_a_1_to_1_crop_of_the_master_frame_0(self) -> None:
        master = load(author.DONUT_MASTER)
        full = flat_rgba(dict(master, frames=master["frames"]), 0)
        made = flat_rgba(load(SRC / "hud_portrait_donut.grid.json"), 0)
        x0, y0 = 14, 4
        differing = [(x, y) for y in range(28) for x in range(28)
                     if made[y][x] != full[y0 + y][x0 + x]]
        self.assertEqual(sorted(differing), [(0, 21), (10, 27), (23, 27), (27, 21)])

    def test_hud_pieces_use_only_hud_tokens(self) -> None:
        tokens = {h + "ff" for h in author.HUD.values()}
        for name in ("hud_system_crt", "hud_plate_frame", "hud_strip_3slice", "hud_notice_frame",
                     "hud_glyphs", "hud_icons", "hud_tags"):
            self.assertLessEqual(rgba_set(load(SRC / f"{name}.grid.json")), tokens, name)

    def test_hud_token_palette_budget(self) -> None:
        self.assertLessEqual(len(set(author.HUD) | set(author.CODE_TOKENS)), 36)  # spec budget 34 + 2 wells

    def test_no_forbidden_controls_in_glyph_set(self) -> None:
        names = {t["name"] for t in load(SRC / "hud_glyphs.grid.json")["tags"]}
        self.assertEqual(names, {"glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle",
                                 "glyph_square", "glyph_triangle", "socket_locked"})


class HudAtlas(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load(MANIFEST)
        assert ATLAS.is_file(), "run bash scripts/export_assets.sh first"
        cls.image = Image.open(ATLAS).convert("RGBA")
        cls.regions = {r["name"]: r for r in cls.manifest["hud"]["regions"]}

    def test_region_names_are_the_contract(self) -> None:
        self.assertEqual([r["name"] for r in self.manifest["hud"]["regions"]], REGION_NAMES)

    def test_atlas_size_and_metadata(self) -> None:
        meta = load(ROOT / "generated/art/hud_hybrid_a.json")
        self.assertEqual(list(self.image.size), self.manifest["hud"]["atlas_size"])
        self.assertEqual(len(meta["frames"]), 1)
        self.assertEqual(meta["frames"][0]["frame"]["w"], self.image.width)

    def test_regions_inside_atlas_and_disjoint(self) -> None:
        rects = []
        for r in self.regions.values():
            x, y, w, h = r["rect"]
            self.assertTrue(x >= 0 and y >= 0 and w > 0 and h > 0, r["name"])
            self.assertTrue(x + w <= self.image.width and y + h <= self.image.height, r["name"])
            rects.append((r["name"], x, y, x + w, y + h))
        for i, a in enumerate(rects):
            for b in rects[i + 1:]:
                self.assertTrue(a[3] <= b[1] or b[3] <= a[1] or a[4] <= b[2] or b[4] <= a[2],
                                f"{a[0]} overlaps {b[0]}")

    def test_region_pixels_match_manifest_hash(self) -> None:
        for r in self.regions.values():
            x, y, w, h = r["rect"]
            crop = self.image.crop((x, y, x + w, y + h))
            data = b"".join(bytes(p[:4]) if p[3] else b"\x00\x00\x00\x00" for p in crop.getdata())
            self.assertEqual(hashlib.sha256(data).hexdigest(), r["sha256"], r["name"])

    def test_no_partial_alpha_and_no_stray_pixels_outside_regions(self) -> None:
        alpha = self.image.getchannel("A")
        self.assertEqual(set(alpha.getdata()) - {0, 255}, set())
        inside = set()
        for r in self.regions.values():
            x, y, w, h = r["rect"]
            inside |= {(xx, yy) for yy in range(y, y + h) for xx in range(x, x + w)}
        stray = [(x, y) for y in range(self.image.height) for x in range(self.image.width)
                 if self.image.getpixel((x, y))[3] and (x, y) not in inside]
        self.assertEqual(stray, [])

    def test_every_region_has_opaque_pixels_and_expected_transparency(self) -> None:
        def opaque(name: str, x: int, y: int, w: int, h: int) -> int:
            rx, ry, _, _ = self.regions[name]["rect"]
            return sum(1 for yy in range(h) for xx in range(w)
                       if self.image.getpixel((rx + x + xx, ry + y + yy))[3])
        for r in self.regions.values():
            self.assertGreater(opaque(r["name"], 0, 0, r["rect"][2], r["rect"][3]), 0, r["name"])
        # the plate leaves the 28x28 well interior and the 32x9 tag area transparent
        self.assertEqual(opaque("plate_frame", 6, 6, 28, 28), 0)
        self.assertEqual(opaque("plate_frame", 4, 36, 32, 9), 0)
        # portraits keep a transparent background (corners of the Carl key are empty)
        self.assertLess(opaque("portrait_carl", 0, 0, 28, 28), 28 * 28)
        # frames are fully opaque except the declared holes
        self.assertEqual(opaque("notice_frame", 0, 0, 204, 32), 204 * 32)
        self.assertEqual(opaque("tag_p1", 0, 0, 32, 9), 32 * 9)

    def test_palette_membership(self) -> None:
        carl = {tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in
                (e["rgba"] for e in load(author.CARL_MASTER)["palette"])}
        donut = {tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in
                 (e["rgba"] for e in load(author.DONUT_MASTER)["palette"])}
        hud = {tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in author.HUD.values()}

        def colours(name: str) -> set:
            x, y, w, h = self.regions[name]["rect"]
            return {p[:3] for p in self.image.crop((x, y, x + w, y + h)).getdata() if p[3]}
        self.assertLessEqual(colours("portrait_carl"), carl)
        self.assertLessEqual(colours("portrait_donut"), donut)
        for name in REGION_NAMES:
            if not name.startswith("portrait_"):
                self.assertLessEqual(colours(name), hud, name)
        opaque_colours = {p[:3] for p in self.image.getdata() if p[3]}
        self.assertLessEqual(len(opaque_colours), 63)

    def test_frame_counts_in_manifest(self) -> None:
        self.assertEqual(self.manifest["hud"]["frame_counts"], {
            "hud_portrait_carl": 1, "hud_portrait_donut": 1, "hud_system_crt": 3, "hud_glyphs": 9,
            "hud_tags": 3, "hud_icons": 6})
        per_source: dict[str, int] = {}
        for r in self.manifest["hud"]["regions"]:
            per_source[r["source"]] = per_source.get(r["source"], 0) + 1
        self.assertEqual(per_source["hud_system_crt"], 3)
        self.assertEqual(per_source["hud_glyphs"], 9)
        self.assertEqual(per_source["hud_tags"], 3)

    def test_manifest_provenance_and_status(self) -> None:
        m = self.manifest
        self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
        self.assertIn("no AI concept pixels copied", m["provenance"]["origin"])
        self.assertIn("Claude Opus", m["provenance"]["generator"])
        self.assertIn("Claude Sonnet", m["provenance"]["generator"])


class HudEvidence(unittest.TestCase):
    def test_evidence_files_match_checksums(self) -> None:
        sums = (EVIDENCE / "SHA256SUMS").read_text(encoding="utf-8").split("\n")
        listed = {}
        for line in sums:
            if line.strip():
                digest, name = line.split("  ", 1)
                listed[name] = digest
        self.assertTrue({"contact_sheet_x1.png", "contact_sheet_x4.png", "portraits_x8.png",
                         "gate_carl.png", "gate_donut.png", "gate_crt.png"} <= set(listed))
        for name, digest in listed.items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)


if __name__ == "__main__":
    unittest.main()
