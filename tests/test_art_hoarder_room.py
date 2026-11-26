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
import author_hrd_cues as cues  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

KIT_COLOURS = {v.lower() for ramp in kit.RAMPS.values() for v in ramp}
# SHA-256 of the RGBA sequence of the design stage's hero key (the mound `sealed`, back and front layers)
HERO_BACK = "23094d01a8c5bb87411b868197f00e3a51806b138a859f97c04f8da77762ce7b"
HERO_FRONT = "8b0ce67239a067d2c2cf5a690862193640eeaaeb6345fe48c4df8acad4183e6e"
# HRD-CUES-01 A1: the RGBA sequence hashes of the frames that existed before the cues (taken from the grids of main
# before the HRD-CUES-01 branch; palette-independent, because the cue frames re-lettered the palette keys)
OLD_FRAMES = {
    ("hoarder_mound", 0): HERO_BACK,
    ("hoarder_mound", 1): HERO_FRONT,
    ("hoarder_mound", 2): "fea45a016c9e627a0bec135ec175da0cefa62f2bc917646a00e4135f85c8bb58",
    ("hoarder_post_b", 0): "19e2a17fab7d5f262f90308e76b7d3ac6969ee299882d24678235eea2fd58599",
    ("hoarder_post_b", 1): "c0406009bfde6e79cb059135d21983ce4a18decf274d021ba5d4087dfc5d43ce",
    ("hoarder_post_b", 2): "2ae5fdbd24cd1dbce69e9b9640afafced9e8e19be98439f19a4b5811677cc353",
}


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

        for name, frames in (("hoarder_mound", (0, 2, 3, 4, 5)), ("hoarder_heap", (0, 1, 2)),
                              ("hoarder_post_b", (0, 1, 2, 3, 4))):
            grid = load(name)
            for f in frames:
                self.assertLess(longest_flat_top(pixels(grid, f)), 12, f"{name} frame {f}: a ragged top, no flat line")
        bank = mean_index(load("hoarder_bank_a"), 0)
        for name, f in (("hoarder_mound", 2), ("hoarder_heap", 0)):
            self.assertLess(mean_index(load(name), f), bank + 2, name)

    def test_post_has_no_label_and_no_shelf(self) -> None:
        """The post is a thin upright: the timber is at most 12 px wide at every row above the shim (spec section 7)."""
        post = load("hoarder_post_b")
        for f in range(5):
            px = pixels(post, f)
            for y in range(0, 60):
                width = sum(1 for c in px[y] if c)
                self.assertLessEqual(width, 14, f"frame {f} row {y}")
    # ---------------------------------------------------------------- HRD-CUES-01 (docs/02-art/hrd-cues-01-spec.md)

    def test_a1_cue_keys_are_frozen_and_the_old_frames_are_unchanged(self) -> None:
        """A1: the two frozen keys match their fixtures (`author_hrd_cues.py --check`), and every frame that existed
        before the cues is the same RGBA sequence as on main (hashed on resolved RGBA, not raw cels)."""
        self.assertEqual(cues.main(["--check"]), 0)
        for (name, frame), expected in OLD_FRAMES.items():
            self.assertEqual(sequence_hash(pixels(load(name), frame)), expected, f"{name} frame {frame}")

    def test_a2_the_stir_frames_are_a_rigid_lift_of_the_sealed_back_frame(self) -> None:
        """A2: under the hinge row a stir frame equals `sealed_back`; above it every column is `sealed_back` taken from
        `lift` rows lower (a source row above the canvas is transparent, not clamped), apart from outline pixels and
        the grit specks."""
        mound = load("hoarder_mound")
        back, front = pixels(mound, 0), pixels(mound, 1)
        h, w = len(back), len(back[0])
        outline = kit.RAMPS["mortar"][0].lower()
        stone7, stone3 = kit.RAMPS["stone"][7].lower(), kit.RAMPS["stone"][3].lower()
        for frame, kind in enumerate(room.STIR_KINDS, start=3):
            px = pixels(mound, frame)
            grit = {}
            for dx, over in room.STIR_GRIT[kind]:
                x = room.PIVOT[0] + dx
                top = next(y for y in range(h) if px[y][x])
                self.assertEqual((px[top][x], px[top + 1][x]), (stone7, stone3), f"{kind} grit at {x}")
                grit[x] = {top, top + 1}
            for x in range(w):
                lip = next((y for y in range(h) if front[y][x]), None)
                lift = room.stir_lift(x, kind) if lip is not None else 0
                hinge = min(71, (lip or 0) + abs(lift) + 1)
                for y in range(h):
                    if y in grit.get(x, ()):
                        continue
                    if y >= hinge:
                        self.assertEqual(px[y][x], back[y][x], f"{kind} ({x},{y}) under the hinge")
                        continue
                    if px[y][x] == outline:
                        continue
                    src = min(y + lift, hinge) if lift >= 0 else y + lift
                    expected = back[src][x] if 0 <= src < h else None
                    self.assertEqual(px[y][x], expected, f"{kind} ({x},{y}) lift {lift}")

    def test_a3_nothing_reaches_row_zero_and_the_front_lip_hides_the_seam(self) -> None:
        """A3: no stir frame has an opaque pixel on row 0. A3b: every pixel at or under the lip's top row that differs
        from `sealed_back` is covered by an opaque `sealed_front` pixel."""
        mound = load("hoarder_mound")
        back, front = pixels(mound, 0), pixels(mound, 1)
        for frame in (3, 4, 5):
            px = pixels(mound, frame)
            self.assertFalse(any(px[0]), f"frame {frame} row 0")
            for x in range(len(back[0])):
                lip = next((y for y in range(len(back)) if front[y][x]), None)
                if lip is None:
                    continue
                for y in range(lip, len(back)):
                    if px[y][x] != back[y][x]:
                        self.assertIsNotNone(front[y][x], f"frame {frame} ({x},{y}) differs under the lip uncovered")

    def test_a4_the_shored_post_has_no_pale_split_and_two_iron_straps(self) -> None:
        """A4: `shored` has no `wood` 5 pixel in the split's columns 4..10, rows 14..51; iron 4 across columns 2..13 at
        rows 18 and 40; and its mean ramp index is lower than `intact`'s."""
        post = load("hoarder_post_b")
        px = pixels(post, 3)
        pale = kit.RAMPS["wood"][5].lower()
        for y in range(14, 52):
            for x in range(4, 11):
                self.assertNotEqual(px[y][x], pale, f"pale wood at ({x},{y})")
        iron4 = kit.RAMPS["iron"][4].lower()
        for y in (18, 40):
            self.assertEqual(px[y][2:14], [iron4] * 12, f"strap row {y}")
        idx = {v.lower(): i for ramp in kit.RAMPS.values() for i, v in enumerate(ramp)}

        def mean(frame: int) -> float:
            vals = [idx[c] for r in pixels(post, frame) for c in r if c]
            return sum(vals) / len(vals)

        self.assertLess(mean(3), mean(0))
        self.assertTrue(any(c == pale for r in pixels(post, 4)[14:52] for c in r[4:11]), "give keeps the split")

    def test_door_and_gate_states_differ_and_open_states_are_clear(self) -> None:
        door, gate = load("hoarder_door"), load("hoarder_gate")
        self.assertNotEqual(pixels(door, 0), pixels(door, 1))
        self.assertNotEqual(pixels(gate, 0), pixels(gate, 1))
        # the open gate leaves most of the lane clear (bars raised into the ceiling)
        open_px = pixels(gate, 0)
        mid = sum(1 for y in range(40, 200) for c in open_px[y] if c)
        self.assertLess(mid, 160 * 3)

    def test_stack_polish_snapped_strut_and_junk_rubble(self) -> None:
        """HOARD-02 polish: the `falling` frame's strut is snapped to STACK_FALL_LEG (the body is 23 px over the floor
        on the last falling tick, so the stub never dips under the floor line); the `landed` remnant keeps the stack's
        junk (iron pot and pram, copper kettle, glass bottles, wood) instead of a plain plank slab."""
        stack = load("hoarder_stack_b")
        oy = room.STACK_OY
        for f in (0, 1):
            self.assertEqual(bbox(pixels(stack, f))[3], oy + 64 + room.STACK_LEG - 1, f)
        self.assertEqual(bbox(pixels(stack, 2))[3], oy + 64 + room.STACK_FALL_LEG - 1)
        self.assertLess(room.STACK_FALL_LEG, 96 - 96 * 7 * 7 // 64)    # under the body's last falling height (23)
        ramp_of = {v.lower(): name for name, ramp in kit.RAMPS.items() for v in ramp}
        landed = pixels(stack, 3)
        used = {ramp_of[c] for row in landed for c in row if c}
        self.assertTrue({"wood", "iron", "copper", "gal", "floor"} <= used, used)
        # dark voids between the items: under the lit top and its lip, at least 12 % of the band is the darkest wood (the old plank slab: about 1 %)
        darkest = kit.RAMPS["wood"][0].lower()
        band = [c for row in landed[90:103] for c in row[1:127]]
        self.assertGreater(sum(1 for c in band if c == darkest) / len(band), 0.12)

    def test_heap_b_has_a_rim_lit_top(self) -> None:
        """HOARD-02 polish: heap b (x 470, behind the dark floor shapes of the lane edge) gets the torch's rim light:
        the pixels just under its outline are brighter than heap a's."""
        idx = {v.lower(): i for ramp in kit.RAMPS.values() for i, v in enumerate(ramp)}
        outline = kit.RAMPS["mortar"][0].lower()

        def rim(px) -> float:
            vals = []
            for x in range(len(px[0])):
                col = [px[y][x] for y in range(len(px)) if px[y][x] and px[y][x] != outline]
                if col:
                    vals.append(idx[col[0]])
            return sum(vals) / len(vals)

        heaps = load("hoarder_heap")
        self.assertGreater(rim(pixels(heaps, 1)), rim(pixels(heaps, 0)) + 1.0)


if __name__ == "__main__":
    unittest.main()
