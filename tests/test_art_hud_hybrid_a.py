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
    "hud_art_02_keycaps": "6bcaed3b355fda18afa9c0f0acbb89174d4aeb5e3989c940d2a7f24a2b5495fd",
    "hud_art_02_chips": "d97f2fb477ef8fde4837a753642edba3a69d9d9083057cae21e6d1ca9f24e81b",
    "hud_art_02_notice_plate": "87bb7511e524da0cfdb2306f09ec2eaead3fe44452f79731b2286c11703d2fa5",
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
    "key_j", "key_k", "key_l", "key_q", "key_space", "key_left", "key_right", "key_down",
    "chip_dynamite", "chip_puddle",
]
FRAME_COUNTS = {"hud_portrait_carl": 1, "hud_portrait_donut": 1, "hud_system_crt": 3, "hud_glyphs": 9,
                "hud_tags": 3, "hud_icons": 6, "hud_plate_frame": 1, "hud_notice_frame": 1,
                "hud_strip_3slice": 1, "hud_keycaps": 8, "hud_chips": 2, **{n: 1 for n in author.VIEWER_SOURCES}}
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
        for name in KEY_SHA256:
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
                     "hud_glyphs", "hud_icons", "hud_tags", "hud_keycaps", "hud_chips", *author.VIEWER_SOURCES):
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
            "hud_tags": 3, "hud_icons": 6, "hud_keycaps": 8, "hud_chips": 2})
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
        self.assertEqual(len(self.regions), 57)
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


# HUD-ART-02 (keyboard key caps, item chips, the notice backing plate), restated independently of the
# authoring script. Spec: docs/02-art/hud-art-02-spec.md sections 2, 5 and 8.
CAP_NAMES = ["key_j", "key_k", "key_l", "key_q", "key_space", "key_left", "key_right", "key_down"]
CHIP_NAMES = ["chip_dynamite", "chip_puddle"]
# frozen key tag -> region name
KEY_TAG_TO_REGION = {"key_j": "key_j", "key_arrow_right": "key_right", "chip_dynamite": "chip_dynamite",
                     "chip_puddle": "chip_puddle"}
CAP_SIZES = {n: (19, 11) if n == "key_space" else (11, 11) for n in CAP_NAMES}
CAP_SYMBOL_ROWS = {  # 5x5 symbol (cols 3..7, rows 2..6) of every cap the executor drew
    "key_k": ["W...W", "W..W.", "WWW..", "W..W.", "W...W"],
    "key_l": ["W....", "W....", "W....", "W....", "WWWWW"],
    "key_q": [".WWW.", "W...W", "W.W.W", "W..W.", ".WW.W"],
    "key_left": ["..W..", ".W...", "WWWWW", ".W...", "..W.."],
    "key_down": ["..W..", "..W..", "W.W.W", ".WWW.", "..W.."],
}
TEXT_WHITE = (0xf4, 0xe6, 0xd2, 255)


def frame_tag(grid: dict, tag: str) -> int:
    return next(t["from"] for t in grid["tags"] if t["name"] == tag)


class HudArt02Sources(unittest.TestCase):
    """The authored caps, chips and notice frame equal the frozen design keys where they overlap."""

    def test_caps_and_chips_equal_the_frozen_keys(self) -> None:
        for key_file, asset in (("hud_art_02_keycaps", "hud_keycaps"), ("hud_art_02_chips", "hud_chips")):
            key = load(SRC / "keys" / f"{key_file}.grid.json")
            made = load(SRC / f"{asset}.grid.json")
            for tag, region in KEY_TAG_TO_REGION.items():
                if tag not in {t["name"] for t in key["tags"]}:
                    continue
                want = flat_rgba(key, frame_tag(key, tag))
                got = flat_rgba(made, frame_tag(made, region))
                for y, row in enumerate(want):
                    self.assertEqual(got[y][:len(row)], row, f"{region} row {y}")

    def test_notice_frame_equals_the_frozen_plate_key(self) -> None:
        key = load(SRC / "keys" / "hud_art_02_notice_plate.grid.json")
        made = load(SRC / "hud_notice_frame.grid.json")
        self.assertEqual(made["canvas"], [204, 32])
        self.assertEqual(flat_rgba(made, 0), flat_rgba(key, 0))

    def test_cap_sources_shape(self) -> None:
        caps = load(SRC / "hud_keycaps.grid.json")
        self.assertEqual(len(caps["frames"]), 8)
        self.assertEqual([t["name"] for t in caps["tags"]], CAP_NAMES)
        self.assertEqual(caps["pivot"], [0, 10])
        chips = load(SRC / "hud_chips.grid.json")
        self.assertEqual(len(chips["frames"]), 2)
        self.assertEqual([t["name"] for t in chips["tags"]], CHIP_NAMES)
        self.assertEqual(chips["pivot"], [0, 10])

    def test_drawn_caps_carry_their_spec_symbols(self) -> None:
        caps = load(SRC / "hud_keycaps.grid.json")
        for name, symbol in CAP_SYMBOL_ROWS.items():
            px = flat_rgba(caps, frame_tag(caps, name))
            white = {(x, y) for y, row in enumerate(px) for x, c in enumerate(row) if c == "#f4e6d2ff"}
            want = {(3 + x, 2 + y) for y, row in enumerate(symbol) for x, ch in enumerate(row) if ch == "W"}
            self.assertEqual(white, want, name)
        # key_left is key_right mirrored (symbol only): the bevel stays lit on top and left
        right = flat_rgba(caps, frame_tag(caps, "key_right"))
        left = flat_rgba(caps, frame_tag(caps, "key_left"))
        for y in range(2, 7):
            self.assertEqual(left[y][3:8], right[y][3:8][::-1], f"row {y}")
        self.assertEqual(left[1][1:10], right[1][1:10])
        # key_space: the 9x2 open bracket at cols 5..13, rows 5..6
        space = flat_rgba(caps, frame_tag(caps, "key_space"))
        white = {(x, y) for y, row in enumerate(space) for x, c in enumerate(row) if c == "#f4e6d2ff"}
        want = {(5 + x, 5) for x in (0, 8)} | {(5 + x, 6) for x in range(9)}
        self.assertEqual(white, want)

    def test_hud_art_02_palettes_are_hud_tokens_only(self) -> None:
        tokens = {h + "ff" for h in author.HUD.values()}
        for name in ("hud_keycaps", "hud_chips", "hud_notice_frame"):
            self.assertLessEqual(rgba_set(load(SRC / f"{name}.grid.json")), tokens, name)
        self.assertLessEqual(len(set(author.HUD) | set(author.CODE_TOKENS)), 36)  # swatch budget unchanged
        # the notice frame uses exactly the HA2-AD3 tokens
        used = {e["name"] for e in load(SRC / "hud_notice_frame.grid.json")["palette"]}
        self.assertEqual(used, {"hud_ink", "metal_hi", "metal_mid", "magenta", "magenta_dk", "notice_bg"})


class HudArt02Atlas(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load(MANIFEST)
        assert ATLAS.is_file(), "run bash scripts/export_assets.sh first"
        cls.image = Image.open(ATLAS).convert("RGBA")
        cls.regions = {r["name"]: r for r in cls.manifest["hud"]["regions"]}

    def crop(self, name: str) -> list[list[tuple[int, int, int, int]]]:
        x, y, w, h = self.regions[name]["rect"]
        return [[self.image.getpixel((x + xx, y + yy)) for xx in range(w)] for yy in range(h)]

    def test_new_regions_exist_with_the_spec_size(self) -> None:
        meta = load(ROOT / "generated/art/hud_hybrid_a.json")
        self.assertEqual(len(meta["frames"]), 1)
        for name in CAP_NAMES:
            self.assertEqual(tuple(self.regions[name]["rect"][2:]), CAP_SIZES[name], name)
            self.assertEqual(self.regions[name]["source"], "hud_keycaps")
        for name in CHIP_NAMES:
            self.assertEqual(self.regions[name]["rect"][2:], [11, 11], name)
            self.assertEqual(self.regions[name]["source"], "hud_chips")
        self.assertEqual(self.regions["notice_frame"]["rect"][2:], [204, 32])

    def test_existing_regions_keep_name_size_and_position(self) -> None:
        self.assertEqual(self.regions["plate_frame"]["rect"], [0, 0, 164, 48])
        self.assertEqual(self.regions["notice_frame"]["rect"], [0, 60, 204, 32])
        self.assertEqual(self.regions["glyph_a"]["rect"], [0, 116, 11, 11])
        self.assertEqual(self.regions["viewer_digit_plus"]["rect"], [48 + 12 * 6, 140, 5, 7])

    def test_hud_art_02_regions_use_hud_tokens_only(self) -> None:
        hud = {tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in author.HUD.values()}
        for name in CAP_NAMES + CHIP_NAMES + ["notice_frame"]:
            colours = {p[:3] for row in self.crop(name) for p in row if p[3]}
            self.assertLessEqual(colours, hud, name)

    def test_caps_differ_from_the_pad_disks_by_silhouette(self) -> None:
        """AT-5: cut 1 px corners and two metal_dark skirt rows on every cap; the lit bevel on row 1."""
        dark = (0x2a, 0x24, 0x30, 255)
        hi = (0x8a, 0x7e, 0x8c, 255)
        for name in CAP_NAMES:
            px = self.crop(name)
            w, h = len(px[0]), len(px)
            for x, y in ((0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)):
                self.assertEqual(px[y][x][3], 0, f"{name} corner ({x},{y}) must be transparent")
            for y in (8, 9):
                self.assertTrue(all(p == dark for p in px[y][1:w - 1]), f"{name} skirt row {y}")
            self.assertTrue(all(p == hi for p in px[1][1:w - 2]), f"{name} bevel row 1")
            self.assertEqual(sum(1 for row in px for p in row if p[3] == 0), 4, f"{name}: only the 4 cut corners are transparent")
        disk_transparent = sum(1 for row in self.crop("glyph_x") for p in row if p[3] == 0)
        cap_transparent = sum(1 for row in self.crop("key_k") for p in row if p[3] == 0)
        self.assertEqual(cap_transparent, 4)
        self.assertGreater(disk_transparent, cap_transparent)

    def test_caps_are_square_keys_with_a_white_symbol(self) -> None:
        for name in CAP_NAMES:
            whites = [(x, y) for y, row in enumerate(self.crop(name)) for x, p in enumerate(row) if p == TEXT_WHITE]
            self.assertTrue(whites, name)
            self.assertTrue(all(2 <= y <= 6 and 3 <= x <= (13 if name == "key_space" else 7) for x, y in whites), name)

    def test_notice_frame_rings_follow_ha2_ad3(self) -> None:
        px = self.crop("notice_frame")
        ink, hi = (0x12, 0x0c, 0x18, 255), (0x8a, 0x7e, 0x8c, 255)
        mid, mag = (0x3d, 0x35, 0x42, 255), (0xe0, 0x33, 0x8f, 255)
        bg, rule = (0x1e, 0x0f, 0x22, 255), (0x6e, 0x1a, 0x4c, 255)
        w, h = 204, 32
        self.assertEqual((len(px[0]), len(px)), (w, h))
        # centre column of every side, ring 0 to ring 3
        self.assertEqual([px[i][100] for i in range(4)], [ink, hi, mag, ink])
        self.assertEqual([px[h - 1 - i][100] for i in range(4)], [ink, mid, mag, ink])
        self.assertEqual([px[16][i] for i in range(4)], [ink, hi, mag, ink])
        self.assertEqual([px[16][w - 1 - i] for i in range(4)], [ink, mid, mag, ink])
        # dotted rule on row +26, x +4..+199, 2-on / 2-off
        on = [x for x in range(w) if px[26][x] == rule]
        self.assertEqual(on[:4], [4, 5, 8, 9])
        self.assertEqual(on[-2:], [196, 197])
        self.assertTrue(all(((x // 2) % 2 == 0) for x in on) and len(on) == 98)
        self.assertEqual(px[27][100], bg)
        self.assertTrue(all(p[3] == 255 for row in px for p in row))


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
