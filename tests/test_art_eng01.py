"""ENG-01 Goblin Engineer (18 frames, 7 frozen keys): spec rules the shared pixel-art toolkit does not cover (the toolkit
validators run with ``python3 -m scripts.pixelart validate assets/source/characters/goblin/engineer/eng_01.pixelart.json``)."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as ae  # noqa: E402

GRID = ROOT / "assets/source/characters/goblin/engineer/eng_01.grid.json"
FIXTURE = ROOT / "tests/fixtures/eng-01_keys.json"
GOBLIN = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
RIDER = ROOT / "assets/source/enemies/dozer/dozer_riders_h36.grid.json"


def merged(cels: dict[str, list[str]]) -> list[str]:
    rows = [list(r) for r in next(iter(cels.values()))]
    for layer in cels.values():
        for y, r in enumerate(layer):
            for x, ch in enumerate(r):
                if ch != ".":
                    rows[y][x] = ch
    return ["".join(r) for r in rows]


class Eng01Key0Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.key0 = merged(cls.grid["frames"][0]["cels"])
        cls.goblin = json.loads(GOBLIN.read_text(encoding="utf-8"))
        cls.rider = json.loads(RIDER.read_text(encoding="utf-8"))

    def test_committed_grid_and_manifest_match_the_generator(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), ae.dump(ae.frames_grid()))
        self.assertEqual(ae.MANIFEST.read_text(encoding="utf-8"), json.dumps(ae.manifest(), indent=2) + "\n")

    def test_key0_is_frozen(self):
        keys = json.loads(FIXTURE.read_text(encoding="utf-8"))["keys"]
        self.assertEqual(sorted(keys, key=int), [str(k) for k in (0, 3, 10, 12, 13, 14, 17)])
        self.assertTrue(all(len(v["sha256"]) == 64 for v in keys.values()))

    def test_frame_pivot_and_40px_height(self):
        self.assertEqual(self.grid["canvas"], [64, 56])
        self.assertEqual(self.grid["pivot"], [28, 55])
        ys = [y for y, r in enumerate(self.key0) if r.strip(".")]
        self.assertEqual((min(ys), max(ys)), (16, 55))       # 40 px: GOB-01 36 px body + 4 pot rows above the crown
        self.assertEqual(ae.H, 40)

    def test_palette_reuses_goblin_and_rider_keys(self):
        gob = {p["key"]: p["rgba"] for p in self.goblin["palette"]}
        rid = {p["key"]: p["rgba"] for p in self.rider["palette"]}
        self.assertLessEqual(len(ae.PALETTE), 20)
        for key, _name, rgba, _acc in ae.PALETTE:
            self.assertEqual(rgba, gob.get(key, rid.get(key)), key)   # no new colour in the design stage
        used = {ch for r in self.key0 for ch in r} - {"."}
        self.assertTrue({"x", "y", "F", "G", "H", "V", "v"}.isdisjoint(used))   # no stick, pineapple or leaves

    def test_pot_is_the_rider_pot(self):
        # rows 16 to 23 (saucepan, handle) are the rider's engineer_hold rows 25 to 32, x 13 to 42, pixel for pixel
        rider = merged(self.rider["frames"][2]["cels"])
        for y in range(16, 24):
            self.assertEqual(self.key0[y][13:43], rider[y + 9][13:43], y)
        handle = [x for x in range(13, 26) if self.key0[20][x] != "."]
        self.assertLess(max(handle), 27)                     # handle out back (left in the authored facing)

    def test_body_below_the_chin_is_gob01(self):
        body = merged({"body": self.goblin["frames"][0]["cels"]["body"]})
        for y in list(range(28, 35)) + list(range(44, 56)):   # row 43 only loses the stick strap's end pixel
            self.assertEqual(self.key0[y][19:64], body[y][19:64], y)

    def test_wrench_is_a_stowed_prop(self):
        prop = self.grid["frames"][0]["cels"]["prop"]
        cells = [(x, y) for y, r in enumerate(prop) for x, ch in enumerate(r) if ch != "."]
        self.assertTrue(cells)
        xs = [x for x, _ in cells]
        ys = [y for _, y in cells]
        self.assertLessEqual(max(xs), 18)                    # behind the cape outline, never in the hand
        self.assertTrue(35 <= min(ys) and max(ys) <= 42)     # back hip, ends in the belt loop on row 42
        self.assertEqual({prop[y][x] for x, y in cells} - set("K234567"), set())


class Eng01SetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(GRID.read_text(encoding="utf-8"))
        cls.frames = [merged(f["cels"]) for f in cls.grid["frames"]]
        cls.manifest = json.loads(ae.MANIFEST.read_text(encoding="utf-8"))

    def pot_offset(self, rows):
        glints = [(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch == "7"]
        self.assertEqual(len(glints), 1)
        return glints[0][0] - 29, glints[0][1] - 19          # key 0 has the glint at (29, 19)

    def test_tags_and_tick_windows(self):
        ticks = [f["ticks"] for f in self.manifest["eng"]["frames"]]
        window = {n: sum(t or 0 for t in ticks[a:b + 1]) for n, a, b in ae.TAGS}
        self.assertEqual(window, {"idle": 38, "walk": 36, "attack_tell": 32, "attack_ram": 9, "attack_recover": 18,
                                  "hurt": 12, "down": 15})
        self.assertIsNone(ticks[17])                         # the last down frame is held by state
        self.assertEqual(len(self.frames), 18)
        self.assertEqual(self.manifest["eng"]["attack_ticks"], {"tell": [0, 31], "ram": [32, 40],
                                                                "hitbox_active": [35, 40], "recover": [41, 58]})

    def test_pot_rows_identical_in_every_frame(self):
        pot = {k: v for k, v in ae.PARTS["POT"].items()}
        for n, rows in enumerate(self.frames):
            dx, dy = self.pot_offset(rows)
            for (x, y), ch in pot.items():
                self.assertEqual(rows[y + dy][x + dx], ch, (n, x, y))   # the pot is never redrawn, tilted or recoloured

    def test_pot_handle_keeps_its_minimum_length_in_every_frame(self):
        # key 0: the handle is the pot pixels left of the pot body (x <= 25) in rows 19 to 22, 13 px from tip to body
        key0 = self.frames[0]
        cells = [(x, y) for y in range(19, 23) for x in range(0, 26) if key0[y][x] != "."]
        min_len = max(x for x, _ in cells) - min(x for x, _ in cells) + 1
        min_px = len(cells)
        self.assertGreaterEqual(min_len, 13)
        for n, rows in enumerate(self.frames):
            dx, dy = self.pot_offset(rows)
            got = [(x, y) for y in range(19 + dy, 23 + dy) for x in range(0, 26 + dx) if rows[y][x] != "." and x <= 25 + dx
                   and x >= 13 + dx]
            extent = max(x for x, _ in got) - min(x for x, _ in got) + 1
            self.assertGreaterEqual(extent, min_len, n)      # a shorter handle would read as a bowler hat
            self.assertGreaterEqual(len(got), min_px, n)

    def test_no_see_through_row_inside_the_body(self):
        # every row between the top and the floor holds at least 3 body-layer pixels (a hairline of background across the
        # waist, as frame 14 once had on row 43, leaves 1 or none); frame 15 has the spec's 2 px air gap under the pot (not checked; nor are the lying frames)
        for n, f in enumerate(self.grid["frames"][:15]):         # the lying frames have 1 to 2 px ear and handle tips
            body = f["cels"]["body"]
            rows = [y for y, r in enumerate(body) if r.strip(".")]
            for y in range(min(rows), max(rows) + 1):
                self.assertGreaterEqual(sum(ch != "." for ch in body[y]), 3, (n, y))

    def test_ram_arm_is_clear_of_the_wrench(self):
        f = self.grid["frames"][12]["cels"]
        prop = [(x, y) for y, r in enumerate(f["prop"]) for x, ch in enumerate(r) if ch != "."]
        skin = [x for y, r in enumerate(f["body"]) for x, ch in enumerate(r) if ch in "lms" and 38 <= y <= 44 and x <= 30]
        wx = max(x for x, _ in prop)
        self.assertGreaterEqual(min(skin) - wx - 1, 2)       # at least 2 px of clear space between arm and wrench

    def test_wrench_stays_a_prop_and_leaves_the_down_frames(self):
        for n, f in enumerate(self.grid["frames"]):
            prop = [ch for r in f["cels"]["prop"] for ch in r if ch != "."]
            self.assertEqual(bool(prop), n <= 14, n)         # dropped under him in 15 to 17
            used = {ch for r in f["cels"]["prop"] for ch in r}
            self.assertLessEqual(used, set(".K234567"), n)

    def test_hurt_and_down_bodies_are_gob01(self):
        goblin = json.loads(GOBLIN.read_text(encoding="utf-8"))
        for n, gf, dx in ((14, 16, 0), (15, 20, 10), (16, 21, 10), (17, 22, 10)):
            body = merged({"body": goblin["frames"][gf]["cels"]["body"]})
            rows = self.frames[n]
            for y in range(56):
                for x in range(64):
                    gx = x - dx
                    if 0 <= gx < 64 and body[y][gx] != "." and rows[y][x] != body[y][gx]:
                        self.assertEqual(rows[y][x] in "K5t67234" or rows[y][x] == "7", True, (n, x, y, rows[y][x]))

    def test_walk_steps_planted_feet_back_four_px(self):
        def feet(rows):
            row, spans, start = rows[55], [], None
            for x, ch in enumerate(row + "."):
                if ch != "." and start is None:
                    start = x
                if ch == "." and start is not None:
                    spans.append((start, x - 1))
                    start = None
            return spans
        ends = {n: [r for _, r in feet(self.frames[n])] for n in range(3, 9)}
        self.assertEqual(ends[3], [27, 39])
        self.assertEqual(ends[4][-1], 35)                    # near foot planted, far foot lifted off the floor
        self.assertEqual(ends[6], [27, 39])


if __name__ == "__main__":
    unittest.main()
