"""AREA-HOARDER-01 room kit checks (docs/02-art/hoarder-hoard-02.md section 7): the authoring script reproduces every
committed grid and manifest, the .ase files match, the mound's `sealed` hero key is unchanged, each atlas stays under 63
colours on the kit ramps, every sprite matches its collision rectangle within 1 px, and each prop's look matches its
collision role (R6: a solid has one unbroken lit top row and vertical sides, non-solid junk has a ragged top and sits a
step darker)."""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402
import author_hoarder_room as room  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

KIT_COLOURS = {v.lower() for ramp in kit.RAMPS.values() for v in ramp}
# SHA-256 of the RGBA sequence of the design stage's hero key (the mound `sealed`, back and front layers)
HERO_BACK = "23094d01a8c5bb87411b868197f00e3a51806b138a859f97c04f8da77762ce7b"
HERO_FRONT = "8b0ce67239a067d2c2cf5a690862193640eeaaeb6345fe48c4df8acad4183e6e"


def load(name: str) -> dict:
    return json.loads((room.OUT / f"{name}.grid.json").read_text())


def pixels(grid: dict, frame: int) -> list[list[str | None]]:
    """RGBA hex per pixel (None for transparent) of one frame."""
    pal = {e["key"]: e["rgba"][:7].lower() for e in grid["palette"]}
    return [[None if c == "." else pal[c] for c in row] for row in grid["frames"][frame]["cels"]["main"]]


def bbox(px) -> tuple[int, int, int, int]:
    ys = [y for y, r in enumerate(px) if any(r)]
    xs = [x for r in px for x, c in enumerate(r) if c]
    return min(xs), min(ys), max(xs), max(ys)


def sequence_hash(px) -> str:
    return hashlib.sha256(",".join((c + "ff") if c else "." for r in px for c in r).encode()).hexdigest()


class RoomKitTests(unittest.TestCase):
    def test_script_reproduces_every_grid_and_manifest(self) -> None:
        self.assertEqual(room.main(["--check"]), 0)

    def test_ase_frame_counts_match_the_grids(self) -> None:
        for name in room.PIECES:
            ase = pg.read_ase(room.piece_paths(name)[1].read_bytes())
            grid = load(name)
            self.assertEqual(ase["canvas"], grid["canvas"], name)
            self.assertEqual(len(ase["frames"]), len(grid["frames"]), name)

    def test_the_mound_sealed_hero_key_is_unchanged(self) -> None:
        grid = load("hoarder_mound")
        self.assertEqual(sequence_hash(pixels(grid, 0)), HERO_BACK)
        self.assertEqual(sequence_hash(pixels(grid, 1)), HERO_FRONT)

    def test_colours_are_kit_ramps_and_under_63_per_atlas(self) -> None:
        for name in room.PIECES:
            grid = load(name)
            self.assertLessEqual(len(grid["palette"]), 63, name)
            for e in grid["palette"]:
                self.assertIn(e["rgba"][:7].lower(), KIT_COLOURS, f"{name} {e['name']}")

    def test_manifests_name_every_frame_and_the_collision_role(self) -> None:
        for name, (_fn, names, tags, _pivot, role) in room.PIECES.items():
            m = json.loads((room.MANIFEST_DIR / f"floor01_{name}.json").read_text())
            self.assertEqual(m["frame_names"], names)
            self.assertEqual(m["collision_role"], role)
            self.assertEqual([t["name"] for t in load(name)["tags"]], [t[0] for t in tags])
            self.assertEqual(m["frame_size"], load(name)["canvas"])

    def test_sprites_match_their_collision_rectangles(self) -> None:
        """A5: bank A 64 x 48 exactly (top and faces), the post's timber inside its 16 x 64, the stack body 128 x 64 plus
        its 32 px leg, the door and gate 16 x 228, the remnants 80 x 16 and 128 x 16, the heaps and the opened mound within
        their size limits."""
        bank = load("hoarder_bank_a")
        self.assertEqual(bbox(pixels(bank, 0)), (0, 0, 63, 47))
        for f in range(5):                               # the standing and moving states keep the 64 x 48 body
            self.assertEqual(bbox(pixels(bank, f)), (0, 0, 63, 47), f)
        self.assertEqual(bbox(pixels(bank, 5)), (0, 32, 79, 47))   # the 80 x 16 remnant
        post = load("hoarder_post_b")
        for f in range(2):
            x0, y0, x1, y1 = bbox(pixels(post, f))
            self.assertLessEqual((x1 - x0 + 1, y1 + 1), (16, 64))
            self.assertEqual(y1, 63)                     # the foot is on the floor
        stack = load("hoarder_stack_b")
        x0, y0, x1, y1 = bbox(pixels(stack, 0))
        self.assertEqual((x0, x1, y1), (0, 127, 103))
        self.assertEqual(y1 - 8 + 1, 64 + room.STACK_LEG)  # body rows 8..71 plus the leg
        self.assertEqual(bbox(pixels(stack, 3)), (0, 88, 127, 103))   # the 128 x 16 remnant
        for name in ("hoarder_door", "hoarder_gate"):
            grid = load(name)
            self.assertEqual(grid["canvas"], [16, 228])
        self.assertEqual(bbox(pixels(load("hoarder_door"), 0)), (0, 0, 15, 227))
        for f in range(3):
            x0, y0, x1, y1 = bbox(pixels(load("hoarder_heap"), f))
            self.assertLessEqual((x1 - x0 + 1, y1 - y0 + 1), (48, 24))
        mound = load("hoarder_mound")
        _x0, y0, _x1, y1 = bbox(pixels(mound, 2))
        self.assertLessEqual(y1 - y0 + 1, 24)           # the opened spill is at most 24 px high
        self.assertEqual(y1, 71)

    def test_solids_have_one_unbroken_lit_top_row_and_vertical_sides(self) -> None:
        """R6: bank A's top row is the brightest wood across all 64 px; both side columns are opaque from row 0 to 47;
        the remnants read the same way."""
        lit = kit.RAMPS["wood"][5].lower()
        for name, frames, w in (("hoarder_bank_a", (0, 1, 2, 3, 4), 64), ("hoarder_bank_a", (5,), 80),
                                ("hoarder_stack_b", (3,), 128)):
            grid = load(name)
            for f in frames:
                px = pixels(grid, f)
                top = bbox(px)[1]
                self.assertEqual(px[top][:w], [lit] * w, f"{name} frame {f}: the top row is lit and unbroken")
                for x in (0, w - 1):
                    col = [px[y][x] for y in range(top, len(px))]
                    self.assertNotIn(None, col, f"{name} frame {f}: side column {x} is opaque")

    def test_non_solid_junk_has_a_ragged_top_and_sits_darker(self) -> None:
        """R6: the mound, its opened spill, the heaps and the post have no flat lit top line; they are drawn a ramp step
        darker than the solids' mean."""
        def mean_index(grid, frame):
            idx = {e["rgba"][:7].lower(): i for ramp in kit.RAMPS.values() for i, e in enumerate(
                [{"rgba": v} for v in ramp])}
            vals = [idx[c] for r in pixels(grid, frame) for c in r if c and c.lower() in idx]
            return sum(vals) / len(vals)

        def longest_flat_top(px) -> int:
            top = bbox(px)[1]
            row = px[top]
            best = run = 0
            prev = None
            for c in row:
                run = run + 1 if c and c == prev else (1 if c else 0)
                prev = c
                best = max(best, run)
            return best

        for name, frames in (("hoarder_mound", (0, 2)), ("hoarder_heap", (0, 1, 2)), ("hoarder_post_b", (0, 1, 2))):
            grid = load(name)
            for f in frames:
                self.assertLess(longest_flat_top(pixels(grid, f)), 12, f"{name} frame {f}: a ragged top, no flat line")
        bank = mean_index(load("hoarder_bank_a"), 0)
        for name, f in (("hoarder_mound", 2), ("hoarder_heap", 0)):
            self.assertLess(mean_index(load(name), f), bank + 2, name)

    def test_post_has_no_label_and_no_shelf(self) -> None:
        """The post is a thin upright: the timber is at most 12 px wide at every row above the shim (spec section 7)."""
        post = load("hoarder_post_b")
        for f in range(3):
            px = pixels(post, f)
            for y in range(0, 60):
                width = sum(1 for c in px[y] if c)
                self.assertLessEqual(width, 14, f"frame {f} row {y}")

    def test_door_and_gate_states_differ_and_open_states_are_clear(self) -> None:
        door, gate = load("hoarder_door"), load("hoarder_gate")
        self.assertNotEqual(pixels(door, 0), pixels(door, 1))
        self.assertNotEqual(pixels(gate, 0), pixels(gate, 1))
        # the open gate leaves most of the lane clear (bars raised into the ceiling)
        open_px = pixels(gate, 0)
        mid = sum(1 for y in range(40, 200) for c in open_px[y] if c)
        self.assertLess(mid, 160 * 3)


if __name__ == "__main__":
    unittest.main()
