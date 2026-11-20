"""V17 (enclosed holes): flood fill, alpha-0 palette keys, the allowlist and the ``holes`` CLI."""
from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from scripts.pixelart import cli, validators as v  # noqa: E402
from scripts.pixelart.common import ToolkitError, pg  # noqa: E402
from scripts.pixelart.config import AssetConfig  # noqa: E402

SEALED = [".....",
          ".mmm.",
          ".m.m.",
          ".mmm.",
          "....."]
SEALED_PAIR = [".......",
               ".mmmm..",
               ".m..m..",
               ".mmmm..",
               "......."]
# the pocket at (2,2) has solid neighbours on all four sides; only its diagonal (3,3) is open
DIAGONAL_EXIT = ["......",
                 ".mmm..",
                 ".m.m..",
                 ".mm...",
                 "......",
                 "......"]
# a transparent pixel on the frame border is background, however many opaque pixels surround it
BORDER_NOTCH = ["m.mmm",
                "mmmmm",
                "mmmmm"]
TWO_POCKETS = [".......",
               ".mmmmmm",
               ".m.mm.m",
               ".mmmmmm",
               "......."]
CLEAR = [{"key": "z", "name": "clear", "rgba": "#00000000"}]


def flat(lines: list[str]) -> list[list[str]]:
    return [list(row) for row in lines]


def pockets(lines: list[str], connectivity: int = 4, transparent=frozenset(".")) -> list[list[tuple[int, int]]]:
    return v.find_holes(flat(lines), connectivity, transparent)


def grid_of(frames: list[list[str]], extra_palette=(), validate: bool = True) -> dict:
    """A one-layer pixel-grid-v1 document; each frame is a list of text rows, the canvas is the first frame's.

    ``validate=False`` skips the loader's checks, which refuse alpha-0 palette keys (V17 still handles them for grids
    that bypass the loader, such as in-memory builds).
    """
    h, w = len(frames[0]), len(frames[0][0])
    doc = ({
        "format": "pixel-grid-v1", "canvas": [w, h], "pivot": [0, h - 1], "layers": ["body"],
        "palette": [{"key": "m", "name": "mid", "rgba": "#7a9e36ff"},
                    {"key": "K", "name": "outline", "rgba": "#1b1124ff"}, *extra_palette],
        "frames": [{"duration_ms": 100, "cels": {"body": rows}} for rows in frames],
        "tags": [{"name": "a", "from": 0, "to": len(frames) - 1, "direction": "forward"}]})
    return pg.validate_grid(doc) if validate else doc


class FindHolesTests(unittest.TestCase):
    def test_sealed_pocket_is_found_at_both_connectivities(self):
        for conn in (4, 8):
            self.assertEqual(pockets(SEALED, conn), [[(2, 2)]], conn)

    def test_open_background_is_not_a_hole(self):
        self.assertEqual(pockets(["....", ".mm.", "...."]), [])
        self.assertEqual(pockets(["mm", "mm"]), [])          # no transparent pixel at all

    def test_a_multi_pixel_pocket_is_one_component(self):
        self.assertEqual(pockets(SEALED_PAIR), [[(2, 2), (3, 2)]])

    def test_two_pockets_are_two_components(self):
        self.assertEqual(pockets(TWO_POCKETS), [[(2, 2)], [(5, 2)]])

    def test_diagonal_exit_counts_at_connectivity_4_only(self):
        self.assertEqual(pockets(DIAGONAL_EXIT, 4), [[(2, 2)]])
        self.assertEqual(pockets(DIAGONAL_EXIT, 8), [])

    def test_pocket_touching_the_frame_border_is_background(self):
        self.assertEqual(pockets(BORDER_NOTCH, 4), [])
        self.assertEqual(pockets(BORDER_NOTCH, 8), [])

    def test_alpha_zero_palette_key_is_transparent(self):
        lines = [".....",
                 ".mmm.",
                 ".mzm.",
                 ".mmm.",
                 "....."]
        self.assertEqual(pockets(lines, transparent={".", "z"}), [[(2, 2)]])
        self.assertEqual(pockets(lines), [])                 # an undeclared 'z' is an opaque pixel
        border_z = ["z....",
                    ".mmm.",
                    ".m.m.",
                    ".mmm.",
                    "....z"]
        self.assertEqual(pockets(border_z, transparent={".", "z"}), [[(2, 2)]])

    def test_transparent_keys_reads_the_palette(self):
        self.assertEqual(v.transparent_keys(grid_of([SEALED], CLEAR, validate=False)), {".", "z"})
        self.assertEqual(v.transparent_keys(grid_of([SEALED])), {"."})

    def test_alpha_zero_key_in_a_real_grid(self):
        grid = grid_of([[".....", ".mmm.", ".mzm.", ".mmm.", "....."]], CLEAR, validate=False)
        self.assertEqual(v.unallowed_holes(grid, []), [(0, [(2, 2)])])

    def test_bad_connectivity_is_rejected(self):
        with self.assertRaises(ToolkitError):
            pockets(SEALED, 6)


class AllowlistTests(unittest.TestCase):
    PAIR = [(2, 2), (3, 2)]

    def setUp(self):
        self.grid = grid_of([SEALED_PAIR, SEALED_PAIR, SEALED_PAIR])

    def holes(self, entries):
        return v.unallowed_holes(self.grid, entries)

    def test_no_allowlist_reports_every_frame(self):
        self.assertEqual(self.holes([]), [(0, self.PAIR), (1, self.PAIR), (2, self.PAIR)])

    def test_entry_that_covers_the_pocket_allows_it(self):
        self.assertEqual(self.holes([{"pixels": [[2, 2], [3, 2]], "reason": "intentional"}]), [])

    def test_entry_may_list_more_pixels_than_the_pocket(self):
        self.assertEqual(self.holes([{"pixels": [[2, 2], [3, 2], [9, 9]], "reason": "intentional"}]), [])

    def test_partial_cover_still_reports_the_pocket(self):
        found = self.holes([{"pixels": [[2, 2]], "reason": "only half of it"}])
        self.assertEqual(found, [(0, self.PAIR), (1, self.PAIR), (2, self.PAIR)])

    def test_frames_filter(self):
        entry = {"frames": [0, 2], "pixels": [[2, 2], [3, 2]], "reason": "frames 0 and 2 only"}
        self.assertEqual(self.holes([entry]), [(1, self.PAIR)])
        self.assertEqual(self.holes([{**entry, "frames": "all"}]), [])

    def test_waive_allows_any_pocket_of_the_listed_frames(self):
        self.assertEqual(self.holes([{"waive": True, "reason": "organic debris sheet"}]), [])
        self.assertEqual(self.holes([{"waive": True, "frames": [1], "reason": "frame 1 only"}]),
                         [(0, self.PAIR), (2, self.PAIR)])

    def test_each_pocket_needs_an_entry(self):
        grid = grid_of([TWO_POCKETS])
        left = {"pixels": [[2, 2]], "reason": "left"}
        right = {"pixels": [[5, 2]], "reason": "right"}
        self.assertEqual(len(v.unallowed_holes(grid, [])), 2)
        self.assertEqual(v.unallowed_holes(grid, [left]), [(0, [(5, 2)])])
        self.assertEqual(v.unallowed_holes(grid, [left, right]), [])

    def test_missing_reason_is_rejected(self):
        for entry in ({"pixels": [[1, 1]]}, {"pixels": [[1, 1]], "reason": "  "}, {"waive": True}):
            with self.assertRaises(ToolkitError, msg=entry):
                v.parse_hole_allowlist({"format": v.HOLE_ALLOWLIST_FORMAT, "grids": {"g.grid.json": [entry]}})

    def test_entry_without_pixels_or_waive_is_rejected(self):
        with self.assertRaises(ToolkitError):
            v.parse_hole_allowlist({"format": v.HOLE_ALLOWLIST_FORMAT, "grids": {"g.grid.json": [{"reason": "why"}]}})

    def test_wrong_format_is_rejected(self):
        with self.assertRaises(ToolkitError):
            v.parse_hole_allowlist({"format": "something-else", "grids": {}})

    def test_valid_document_is_keyed_by_grid_path(self):
        doc = {"format": v.HOLE_ALLOWLIST_FORMAT, "grids": {"g.grid.json": [{"pixels": [[1, 1]], "reason": "why"}]}}
        self.assertEqual(v.parse_hole_allowlist(doc), doc["grids"])

    def test_rule_honours_inline_allow_entries(self):
        def cfg(holes):
            return AssetConfig({
                "format": "pixelart-asset-v1", "asset_id": "synthetic", "grid": "unused.grid.json",
                "frame_size": [5, 5], "pivot": [0, 4], "contact_row": 4, "tick_hz": 60, "hold_ms": 100,
                "tags": {"a": {"playback": "loop", "contact": "free", "frames": [[0, 6]]}},
                "rules": {"holes": holes}})
        grid = grid_of([SEALED])
        found = v.v17_holes(v.Context(cfg(True), grid))
        self.assertEqual(len(found), 1)
        self.assertIn("pinhole (2,2)", found[0])
        allowed = {"connectivity": 4, "allow": [{"pixels": [[2, 2]], "reason": "inline"}]}
        self.assertEqual(v.v17_holes(v.Context(cfg(allowed), grid)), [])


class CommittedAllowlistTests(unittest.TestCase):
    def test_committed_allowlist_parses_and_every_entry_cites_its_review(self):
        allow = v.load_hole_allowlist()
        self.assertTrue(allow)
        for grid_path, entries in allow.items():
            self.assertTrue((ROOT / grid_path).is_file(), grid_path)
            for entry in entries:
                self.assertIn("#148", entry["reason"])

    def test_goblin_cf02_sheets_are_clean_after_the_allowlist(self):
        base = ROOT / "assets/source/characters/goblin"
        grids = [base / "cf02_keys/goblin_a_h36_cf02_keys_acting.grid.json",
                 base / "cf02_keys/goblin_a_h36_cf02_keys_slammed.grid.json",
                 base / "goblin_a_h36_cf02.grid.json", base / "goblin_b_h36_cf02.grid.json",
                 base / "death/goblin_a_slammed_h36.grid.json", base / "death/goblin_b_slammed_h36.grid.json"]
        self.assertEqual(v.scan_holes(grids), {})

    def test_gob01_corpse_key_25_pinhole_is_closed(self):
        for name in ("goblin_a_h36", "goblin_b_h36"):
            grid = pg.load_grid(ROOT / f"assets/source/characters/goblin/{name}.grid.json")
            self.assertEqual([p for f, p in v.unallowed_holes(grid, []) if f == 25], [], name)
            self.assertEqual(pg.composite_keys(grid, 25)[43][19], "K", name)


class CliTests(unittest.TestCase):
    def run_cli(self, *args: str) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = cli.main(["holes", *args])
        return code, out.getvalue()

    @staticmethod
    def write(tmp: str, name: str, lines: list[str]) -> str:
        path = Path(tmp) / name
        path.write_text(json.dumps(grid_of([lines])), encoding="utf-8")
        return str(path)

    def test_exit_code_is_one_with_a_hole_and_zero_without(self):
        with tempfile.TemporaryDirectory() as tmp:
            holed = self.write(tmp, "holed.grid.json", SEALED)
            clean = self.write(tmp, "clean.grid.json", ["....", ".mm.", "...."])
            code, out = self.run_cli(holed, "--detail")
            self.assertEqual(code, 1)
            self.assertIn("pinhole (2,2)", out)
            self.assertIn("1 unallowed pocket(s)", out)
            self.assertEqual(self.run_cli(clean)[0], 0)
            self.assertEqual(self.run_cli(clean, holed)[0], 1)

    def test_connectivity_flag(self):
        with tempfile.TemporaryDirectory() as tmp:
            diag = self.write(tmp, "diag.grid.json", DIAGONAL_EXIT)
            self.assertEqual(self.run_cli(diag)[0], 1)
            self.assertEqual(self.run_cli(diag, "--connectivity", "8")[0], 0)

    def test_committed_allowlisted_grid_exits_zero(self):
        code, out = self.run_cli("assets/source/characters/goblin/goblin_a_h36_cf02.grid.json")
        self.assertEqual(code, 0, out)


if __name__ == "__main__":
    unittest.main()
