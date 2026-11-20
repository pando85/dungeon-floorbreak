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
    "viewer_eye", "viewer_up", "viewer_down",
    "viewer_digit_0", "viewer_digit_1", "viewer_digit_2", "viewer_digit_3", "viewer_digit_4",
    "viewer_digit_5", "viewer_digit_6", "viewer_digit_7", "viewer_digit_8", "viewer_digit_9",
    "viewer_digit_k", "viewer_digit_m", "viewer_digit_plus",
]
FRAME_COUNTS = {"hud_portrait_carl": 1, "hud_portrait_donut": 1, "hud_system_crt": 3, "hud_glyphs": 9,
                "hud_tags": 3, "hud_icons": 6, "hud_plate_frame": 1, "hud_notice_frame": 1,
                "hud_strip_3slice": 1, **{n: 1 for n in author.VIEWER_SOURCES}}
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
                     "hud_glyphs", "hud_icons", "hud_tags", *author.VIEWER_SOURCES):
            self.assertLessEqual(rgba_set(load(SRC / f"{name}.grid.json")), tokens, name)

    def test_hud_token_palette_budget(self) -> None:
        self.assertLessEqual(len(set(author.HUD) | set(author.CODE_TOKENS)), 36)  # spec budget 34 + 2 wells

    def test_no_forbidden_controls_in_glyph_set(self) -> None:
        names = {t["name"] for t in load(SRC / "hud_glyphs.grid.json")["tags"]}
        self.assertEqual(names, {"glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle",
                                 "glyph_square", "glyph_triangle", "socket_locked"})


# Review keys (PR #118 design review, section 4), restated independently of the authoring script.
EYE_KEY = [".........", "..BBBBB..", ".B.HHH.B.", "B.HHkHH.B", ".B.HHH.B.", "..BBBBB..", "........."]
UP_KEY = ["..X..", ".XXX.", "XXXXX", "..X..", "..X..", "..X..", "..X.."]
DOWN_KEY = ["..X..", "..X..", "..X..", "..X..", "XXXXX", ".XXX.", "..X.."]
DIGIT_KEYS = {
    "0": ".XXX. X...X X..XX X.X.X XX..X X...X .XXX.", "1": "..X.. .XX.. ..X.. ..X.. ..X.. ..X.. .XXX.",
    "2": ".XXX. X...X ....X ...X. ..X.. .X... XXXXX", "3": "XXXXX ...X. ..X.. ...X. ....X X...X .XXX.",
    "4": "...X. ..XX. .X.X. X..X. XXXXX ...X. ...X.", "5": "XXXXX X.... XXXX. ....X ....X X...X .XXX.",
    "6": "..XX. .X... X.... XXXX. X...X X...X .XXX.", "7": "XXXXX ....X ...X. ..X.. .X... .X... .X...",
    "8": ".XXX. X...X X...X .XXX. X...X X...X .XXX.", "9": ".XXX. X...X X...X .XXXX ....X ...X. .XX..",
    "k": "X...X X..X. X.X.. XX... X.X.. X..X. X...X", "m": "X...X XX.XX X.X.X X.X.X X...X X...X X...X",
    "plus": "..... ..X.. ..X.. XXXXX ..X.. ..X.. .....",
}
VIEWER_COLOURS = {"B": "brass", "H": "brass_hi", "k": "hud_ink"}


def key_mask(rows: list[str], colours: dict[str, str]) -> list[list[str | None]]:
    return [[None if ch == "." else author.HUD[colours[ch]] + "ff" for ch in row] for row in rows]


class ViewerArt(unittest.TestCase):
    """AUDIENCE-01 part A: eye, trend arrows and digit masks match the review keys pixel for pixel."""

    def test_eye_arrows_and_digits_match_the_review_keys(self) -> None:
        cases = {"hud_viewer_eye": key_mask(EYE_KEY, VIEWER_COLOURS),
                 "hud_viewer_up": key_mask(UP_KEY, {"X": "magenta_hi"}),
                 "hud_viewer_down": key_mask(DOWN_KEY, {"X": "amber"})}
        for d, rows in DIGIT_KEYS.items():
            cases[f"hud_viewer_digit_{d}"] = key_mask(rows.split(), {"X": "text_white"})
        self.assertEqual(sorted(cases), sorted(author.VIEWER_SOURCES))
        for name, want in cases.items():
            self.assertEqual(flat_rgba(load(SRC / f"{name}.grid.json"), 0), want, name)

    def test_viewer_sources_are_single_frame_and_digits_are_5x7(self) -> None:
        for name in author.VIEWER_SOURCES:
            grid = load(SRC / f"{name}.grid.json")
            self.assertEqual(len(grid["frames"]), 1, name)
            self.assertEqual(grid["canvas"], [9, 7] if name == "hud_viewer_eye" else [5, 7], name)

    def test_no_decimal_point_in_the_atlas(self) -> None:
        # the dot is a code-placed 1x1 rect: a lone pixel fails the strict lint, so it is not art
        names = {r["name"] for r in load(MANIFEST)["hud"]["regions"]}
        self.assertFalse({n for n in names if "dot" in n or "point" in n})
        self.assertTrue(any("code-placed 1x1" in n for n in load(MANIFEST)["hud"]["notes"]))

    def test_viewer_masks_are_one_colour(self) -> None:
        for name in author.VIEWER_SOURCES:
            if name == "hud_viewer_eye":
                continue
            self.assertEqual(len({e["rgba"] for e in load(SRC / f"{name}.grid.json")["palette"]}), 1, name)


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
        for name in author.VIEWER_SOURCES:
            self.assertEqual(per_source[name], 1, name)

    def test_viewer_regions_sizes_and_layout(self) -> None:
        sizes = {n: self.regions[n]["rect"][2:] for n in self.regions if n.startswith("viewer_")}
        self.assertEqual(sizes.pop("viewer_eye"), [9, 7])
        self.assertEqual(set(map(tuple, sizes.values())), {(5, 7)})
        self.assertEqual(len(sizes) + 1, 16)
        self.assertEqual(len(self.regions), 47)
        # every viewer region is opaque only where its key is (eye keeps empty top and bottom rows)
        x, y, w, h = self.regions["viewer_eye"]["rect"]
        for yy in (0, 6):
            self.assertTrue(all(self.image.getpixel((x + xx, y + yy))[3] == 0 for xx in range(w)))
        self.assertEqual(self.image.getpixel((x + 4, y + 3))[:3], (0x12, 0x0c, 0x18))  # hud_ink pupil

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
