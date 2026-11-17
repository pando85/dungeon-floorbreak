"""Contract tests for the GOB-01 goblin: size study, the full 27-frame sets of variants A and B, the frozen
Opus keys, the tick table and the readability rules R1 to R10 of docs/02-art/gob-01-goblin-spec.md."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_goblin_frames as af  # noqa: E402
import author_goblin_gob01 as ag  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN = ROOT / "assets/source/characters/goblin"
STUDY = GOBLIN / "goblin_size_study.grid.json"
FIXTURE = ROOT / "tests/fixtures/goblin_gob01_keys.json"
WEAPON = set("xyFGHVv")
FRUIT = set("FG")  # pineapple body without the H facet / arc-trail key
SKIN = set("asml")
IDLE_HEAD_TOP = 20
KEYS = (0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26)

# The spec's tag table (section 5): tag -> [(atlas frame, ticks or None for a state-driven hold)]
SPEC_TABLE = {
    "idle": [(0, 12), (1, 8), (2, 12), (3, 8)],
    "walk": [(4, 5), (5, 5), (6, 5), (7, 5), (8, 5), (9, 5)],
    "attack_tell": [(10, 6), (11, 18)],
    "attack_swing": [(12, 2), (13, 4)],
    "attack_recover": [(14, 12), (15, 8)],
    "hurt": [(16, 4), (17, 8)],
    "knockback": [(18, 4), (19, None)],
    "down": [(20, 4), (21, 8), (22, None)],
    "getup": [(23, 8), (24, 10)],
    "corpse": [(25, None)],
    "knockback_contact": [(26, 3)],
}
GROUNDED = ["idle", "walk", "attack_tell", "attack_swing", "attack_recover", "hurt", "down", "getup", "corpse"]
AIRBORNE = ["knockback", "knockback_contact"]


def bbox(flat, keys=None):
    pts = [(x, y) for y, r in enumerate(flat) for x, c in enumerate(r)
           if c != "." and (keys is None or c in keys)]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def load(variant: str) -> dict:
    return pg.load_grid(GOBLIN / f"goblin_{variant}_h36.grid.json")


def layer_pix(frame: dict, layer: str) -> dict:
    return {(x, y): c for y, row in enumerate(frame["cels"].get(layer, [])) for x, c in enumerate(row) if c != "."}


class _FullSet:
    """Checks that hold for both variants (flat composite of every layer)."""
    variant = "a"

    def setUp(self) -> None:
        self.grid = load(self.variant)
        self.manifest = json.loads((ROOT / f"assets/manifests/goblin_{self.variant}_h36.json")
                                   .read_text(encoding="utf-8"))
        self.f = [pg.composite_keys(self.grid, i) for i in range(27)]
        self.tag_of = {}
        for t in self.grid["tags"]:
            for i in range(t["from"], t["to"] + 1):
                self.tag_of[i] = t["name"]

    # ---- structure, tags, ticks
    def test_27_frames_pivot_and_tags(self) -> None:
        self.assertEqual(len(self.grid["frames"]), 27)
        self.assertEqual(self.grid["canvas"], [64, 56])
        self.assertEqual(self.grid["pivot"], [28, 55])
        self.assertEqual(self.manifest["frame_size"], [64, 56])
        self.assertEqual(self.manifest["pivot"], [28, 55])
        self.assertEqual([t["name"] for t in self.grid["tags"]], list(SPEC_TABLE))
        self.assertEqual(self.manifest["animation_tags"], list(SPEC_TABLE))
        for t in self.grid["tags"]:
            self.assertEqual([n for n, _ in SPEC_TABLE[t["name"]]], list(range(t["from"], t["to"] + 1)), t)

    def test_durations_are_the_tick_table_at_60_hz(self) -> None:
        for tag, frames in SPEC_TABLE.items():
            for n, ticks in frames:
                want = af.HOLD_MS if ticks is None else round(ticks * 1000 / 60)
                self.assertEqual(self.grid["frames"][n]["duration_ms"], want, (tag, n))
        rows = {r["index"]: r for r in self.manifest["goblin"]["frames"]}
        self.assertEqual(self.manifest["goblin"]["tick_hz"], 60)
        for tag, frames in SPEC_TABLE.items():
            for n, ticks in frames:
                self.assertEqual((rows[n]["tag"], rows[n]["ticks"]), (tag, ticks))
                self.assertEqual(rows[n]["duration_ms"], self.grid["frames"][n]["duration_ms"])

    def test_attack_is_24_plus_2_plus_4_plus_20_ticks(self) -> None:
        t = {n: s for fr in SPEC_TABLE.values() for n, s in fr}
        self.assertEqual(t[10] + t[11], 24)
        self.assertEqual(t[12], 2)
        self.assertEqual(t[13], 4)   # frame 13 is shown on exactly the hitbox-active ticks
        self.assertEqual(t[14] + t[15], 20)
        self.assertEqual(self.manifest["goblin"]["attack_ticks"],
                         {"tell": [0, 23], "swing_smear": [24, 25], "swing_impact": [26, 29], "recover": [30, 49]})
        self.assertEqual(t[4] * 6, 30)  # a 30-tick walk cycle (31 px at 62 px/s)

    def test_palette_and_layers(self) -> None:
        self.assertLessEqual(len(self.grid["palette"]), 20)
        used = {c for fl in self.f for r in fl for c in r} - {"."}
        self.assertLessEqual(len(used), 20)
        self.assertEqual({p["key"] for p in self.grid["palette"]} - {k for k, *_ in ag.PALETTE}, set())
        for frame in self.grid["frames"]:
            body, weapon = layer_pix(frame, "body"), layer_pix(frame, "weapon")
            self.assertFalse(set(body) & set(weapon), "body and weapon cels overlap")
            n = self.grid["frames"].index(frame)
            wood = {c for c in body.values() if c in WEAPON}
            self.assertEqual(wood, set() if n in KEYS else wood & set("xy"), "weapon key on the body layer")

    # ---- contact rows and airborne frames
    def test_grounded_frames_end_on_row_55(self) -> None:
        for tag in GROUNDED:
            for n, _ in SPEC_TABLE[tag]:
                self.assertEqual(bbox(self.f[n])[3], 55, (tag, n))

    def test_airborne_frames_are_off_the_floor(self) -> None:
        for tag in AIRBORNE:
            for n, _ in SPEC_TABLE[tag]:
                self.assertTrue(all(c == "." for c in self.f[n][55]), (tag, n))
                self.assertLess(bbox(self.f[n])[3], 55)
        self.assertEqual(bbox(self.f[18])[3], 53)   # launch: the feet are just leaving the floor
        self.assertLessEqual(bbox(self.f[19])[3], 50)
        self.assertEqual(self.manifest["goblin"]["grounded_tags"], GROUNDED)
        self.assertEqual(self.manifest["goblin"]["airborne_tags"], AIRBORNE)

    def test_idle_is_36_px(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[0])
        self.assertEqual((y0, y1), (IDLE_HEAD_TOP, 55))

    def test_every_frame_fits_the_canvas_with_a_margin(self) -> None:
        for n, fl in enumerate(self.f):
            x0, y0, x1, y1 = bbox(fl)
            self.assertTrue(x0 >= 0 and x1 <= 63 and y0 >= 0, n)

    # ---- lint
    def test_lint_isolated_and_orphans(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(pg.lint_errors(report), [])
        self.assertEqual(report["isolated_count_per_frame"], [0] * 27)
        self.assertTrue(all(n < 10 for n in report["orphan_count_per_frame"]), report["orphan_count_per_frame"])

    # ---- readability rules
    def fruit_top(self, n: int) -> int:
        return bbox(layer_flat(self.grid, n, "weapon"), FRUIT)[1]

    def test_r1_pineapple_above_the_head_only_in_tell_and_smear(self) -> None:
        self.assertLessEqual(self.fruit_top(11), IDLE_HEAD_TOP - 12)               # the tell hold
        self.assertLess(self.fruit_top(10), IDLE_HEAD_TOP)                          # the lead-in
        for n in (10, 11):                                                          # highest opaque point = pineapple
            top_pix = [(x, y) for y, r in enumerate(self.f[n]) for x, c in enumerate(r) if c != "." and y == bbox(self.f[n])[1]]
            self.assertTrue(all(c in WEAPON or c == "K" for c in (self.f[n][y][x] for x, y in top_pix)), n)
        for n in range(27):
            if self.tag_of[n] in GROUNDED and n not in (10, 11, 12):
                self.assertGreater(self.fruit_top(n), IDLE_HEAD_TOP, n)
        self.assertTrue(2 <= self.fruit_top(12) <= 8)                               # smear

    def test_r2_tell_mouth_open_and_torso_leans_back(self) -> None:
        def o_block(fl):  # best 4-wide, 2-tall block of mouth-dark pixels
            return max((sum(1 for dx in range(4) for dy in range(2) if fl[y + dy][x + dx] == "o")
                        for y in range(54) for x in range(60)), default=0)
        self.assertEqual(o_block(self.f[11]), 8)
        self.assertGreaterEqual(sum(c == "o" for r in self.f[10] for c in r), 4)    # half open: one dark row
        self.assertLess(sum(c == "o" for r in self.f[10] for c in r), sum(c == "o" for r in self.f[11] for c in r))

        def rag_left(fl):
            return min(x for y in range(37, 42) for x, c in enumerate(fl[y]) if c in "utr")
        for n in (10, 11):
            self.assertLessEqual(rag_left(self.f[n]), rag_left(self.f[0]) - 2, n)

    def test_r3_idle_and_walk_pineapple_is_in_front_below_the_head(self) -> None:
        for n in list(range(0, 10)):
            self.assertGreater(self.fruit_top(n), IDLE_HEAD_TOP, n)
            body = layer_flat(self.grid, n, "body")
            nose = max(x for y, r in enumerate(body) for x, c in enumerate(r) if c in SKIN and y <= 34)
            fruit_x = bbox(layer_flat(self.grid, n, "weapon"), FRUIT)[0]
            self.assertGreaterEqual(fruit_x - nose - 1, 3, n)

    def test_r4_impact_frame(self) -> None:
        x0, y0, x1, y1 = bbox(layer_flat(self.grid, 13, "weapon"), FRUIT)
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        self.assertTrue(50 <= cx <= 53 and 31 <= cy <= 37, (cx, cy))
        self.assertGreaterEqual(x0 - self.grid["pivot"][0], 16)
        row55 = [x for x, c in enumerate(self.f[13][55]) if c != "." and x > self.grid["pivot"][0]]
        self.assertTrue(row55 and min(row55) >= 35 and max(row55) <= 42, row55)
        self.assertEqual(self.tag_of[13], "attack_swing")

    def test_r5_hurt_eye_whites_no_yellow_and_toe_only_on_the_key(self) -> None:
        for n in (16, 17):
            self.assertFalse(any(c == "e" for r in self.f[n] for c in r), n)
            self.assertTrue(any(c == "w" for r in self.f[n] for c in r), n)
        front = [x for x, c in enumerate(self.f[16][55]) if c != "." and x > self.grid["pivot"][0]]
        self.assertLessEqual(len(front), 5)
        heel = [x for x, c in enumerate(self.f[16][53]) if c != "." and self.grid["pivot"][0] < x < min(front)]
        self.assertTrue(heel, "the lifted heel sits above row 55, behind the toe")
        front17 = [x for x, c in enumerate(self.f[17][55]) if c != "." and x > self.grid["pivot"][0] - 2]
        self.assertGreaterEqual(len(front17), 6, "frame 17: the heel is down, a flat foot on row 55")
        self.assertGreater(self.fruit_top(16), IDLE_HEAD_TOP)
        self.assertGreater(self.fruit_top(17), IDLE_HEAD_TOP)

    def test_r6_knockback_air_frame(self) -> None:
        x0, y0, x1, y1 = bbox(self.f[19])
        self.assertLessEqual(x1 - x0 + 1, 36)
        self.assertGreaterEqual(y1 - y0 + 1, 22)
        self.assertLessEqual(y1, 50)
        lead = {self.f[19][y][x0] for y in range(56)} - {"."}
        self.assertFalse(lead & WEAPON)
        self.assertGreaterEqual(sum(1 for r in self.f[19] for c in r if c == "R"), 6)   # tongue
        for n in (19, 26):   # the compact airborne keys stay inside a 40 px wide cell
            x0, y0, x1, y1 = bbox(self.f[n])
            self.assertLessEqual(x1 - x0 + 1, 40, n)

    def test_r10_down_frames_stay_low(self) -> None:
        for n in (20, 21, 22, 25):
            self.assertGreaterEqual(bbox(self.f[n])[1], 38, n)
            self.assertLessEqual(bbox(self.f[n])[2] - bbox(self.f[n])[0] + 1, 60)

    def row_runs(self, n: int, y: int = 55) -> list[tuple[int, int]]:
        runs: list[list[int]] = []
        for x, c in enumerate(self.f[n][y]):
            if c != ".":
                if runs and x == runs[-1][1] + 1:
                    runs[-1][1] = x
                else:
                    runs.append([x, x])
        return [(a, b) for a, b in runs]

    def test_d1_planted_foot_travels_back_at_most_6_px_per_frame_without_skating(self) -> None:
        """Review D1: the planted foot moves back about 5 px per frame (62 px/s at 60 Hz is about 1 px per tick)."""
        for frames in ((4, 5, 6), (7, 8, 9)):
            xs = [self.row_runs(n)[-1][0] for n in frames]           # left edge of the rightmost row-55 run
            steps = [b - a for a, b in zip(xs, xs[1:])]
            self.assertTrue(all(-6 <= d <= 0 for d in steps), (frames, xs))   # never forward, at most 6 px back
        self.assertEqual([self.row_runs(n)[-1][0] for n in (4, 5, 6)], [33, 28, 23])
        self.assertEqual([self.row_runs(n)[-1][0] for n in (7, 8, 9)], [31, 26, 21])
        self.assertEqual(self.row_runs(6)[-1], (23, 30))
        self.assertEqual(self.row_runs(9)[-1], (21, 28))

    def test_d1_swinging_foot_is_a_5_by_2_foot_on_rows_49_to_52(self) -> None:
        for n in (6, 9):
            skin = {(x, y) for y in (50, 51) for x in range(28, 33)
                    if self.f[n][y][x] in "lms" or self.f[n][y][x] in "a"}
            self.assertGreaterEqual(len(skin), 6, n)      # most of the 5 x 2 interior is visible
            self.assertGreaterEqual(self.f[n][49][27:34].count("K"), 4, n)   # top edge (the planted shin may cover some)
            self.assertGreaterEqual(self.f[n][52][27:34].count("K"), 4, n)   # sole outline

    def test_d6_frame_17_feet_sit_between_hurt_and_idle(self) -> None:
        self.assertEqual(self.row_runs(17), [(15, 22), (28, 36)])

    def test_d7_rope_belt_is_on_the_body_layer_in_every_in_between(self) -> None:
        for n, frame in enumerate(self.grid["frames"]):
            if n in KEYS:
                continue                                      # the frozen Opus keys carry the belt on weapon
            weapon = layer_flat(self.grid, n, "weapon")
            for y, row in enumerate(weapon):
                run = best = 0
                for c in row:
                    run = run + 1 if c in "xy" else 0
                    best = max(best, run)
                self.assertLess(best, 7, f"belt pixels on the weapon layer: frame {n} row {y}")

    def test_in_betweens_differ_from_every_other_frame(self) -> None:
        seen = {}
        for n, fl in enumerate(self.f):
            key = "".join("".join(r) for r in fl)
            self.assertNotIn(key, seen, f"frame {n} equals frame {seen.get(key)}")
            seen[key] = n

    def test_key_frames_are_frozen(self) -> None:
        fix = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(sorted(int(k) for k in fix["keys"]), list(KEYS))
        for n in KEYS:
            cels = fix["keys"][str(n)]
            frame = self.grid["frames"][n]
            self.assertEqual(frame["cels"]["body"], cels["body"], n)
            self.assertEqual(frame["cels"]["weapon"], cels["weapon"], n)
            if self.variant == "b":
                self.assertEqual(frame["cels"]["accessory"], cels["accessory"], n)

    def test_sources_match_the_generator(self) -> None:
        v = self.variant.upper()
        self.assertEqual((GOBLIN / f"goblin_{self.variant}_h36.grid.json").read_text(encoding="utf-8"),
                         af.dump(af.build(v)))
        self.assertEqual(self.manifest, af.manifest(v))

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase((GOBLIN / f"goblin_{self.variant}_h36.ase").read_bytes())
        self.assertEqual(ase["canvas"], [64, 56])
        self.assertEqual(len(ase["frames"]), 27)
        self.assertEqual([f["duration_ms"] for f in ase["frames"]],
                         [f["duration_ms"] for f in self.grid["frames"]])


def layer_flat(grid: dict, n: int, layer: str) -> list[str]:
    w, h = grid["canvas"]
    rows = grid["frames"][n]["cels"].get(layer) or ["." * w] * h
    return list(rows)


class VariantAFullSetTests(_FullSet, unittest.TestCase):
    variant = "a"

    def test_a_has_two_layers_and_19_colours(self) -> None:
        self.assertEqual(self.grid["layers"], ["body", "weapon"])
        self.assertEqual(len({c for fl in self.f for r in fl for c in r} - {"."}), 19)


class VariantBFullSetTests(_FullSet, unittest.TestCase):
    variant = "b"

    def test_b_body_and_weapon_equal_a_in_every_frame(self) -> None:
        a = load("a")
        self.assertEqual(self.grid["layers"], ["body", "weapon", "accessory"])
        self.assertEqual(self.grid["tags"], a["tags"])
        for n, (fa, fb) in enumerate(zip(a["frames"], self.grid["frames"])):
            self.assertEqual(fa["cels"]["body"], fb["cels"]["body"], n)
            self.assertEqual(fa["cels"]["weapon"], fb["cels"]["weapon"], n)
            self.assertEqual(fa["duration_ms"], fb["duration_ms"], n)

    def test_r9_every_b_frame_carries_the_red_bandana(self) -> None:
        for n, frame in enumerate(self.grid["frames"]):
            acc = "".join(frame["cels"]["accessory"])
            self.assertGreaterEqual(acc.count("R"), 5, n)
            self.assertGreaterEqual(acc.count("B"), 5, n)

    def test_b_uses_20_colours(self) -> None:
        self.assertEqual(len({c for fl in self.f for r in fl for c in r} - {"."}), 20)


class GoblinFrameBuilderTests(unittest.TestCase):
    """The authoring helpers themselves (translation only, deterministic)."""

    def test_build_is_deterministic(self) -> None:
        self.assertEqual(af.dump(af.build("A")), af.dump(af.build("A")))

    def test_despeckle_removes_orphans_but_keeps_accents(self) -> None:
        p = {(5, 5): "m", (6, 5): "m", (5, 6): "m", (6, 6): "m", (7, 5): "s", (6, 7): "e"}
        self.assertEqual(af.orphans(p), [(7, 5)])
        fixed = af.despeckle(p)
        self.assertEqual(fixed[(7, 5)], "m")
        self.assertEqual(fixed[(6, 7)], "e")

    def test_shift_and_cut_roundtrip(self) -> None:
        p = af.parse("mm\nss")
        part, rest = af.cut(p, af.box(0, 0, 1, 0))
        self.assertEqual(af.over(rest, part), p)
        self.assertEqual(af.shift(p, 2, 1)[(2, 1)], "m")

    def test_rejects_a_tampered_key(self) -> None:
        """A changed key must not equal the frozen fixture (the freeze test would fail)."""
        fix = json.loads(FIXTURE.read_text(encoding="utf-8"))
        tampered = list(fix["keys"]["0"]["body"])
        tampered[30] = tampered[30][:10] + "K" + tampered[30][11:]
        self.assertNotEqual(tampered, load("a")["frames"][0]["cels"]["body"])


class GoblinSizeStudyTests(unittest.TestCase):
    def test_generator_matches_committed_grid(self) -> None:
        self.assertEqual(STUDY.read_text(encoding="utf-8"), ag.dump(ag.size_study_grid()))

    def test_option_heights(self) -> None:
        grid = pg.load_grid(STUDY)
        for i, h in enumerate(ag.HEIGHTS):
            _x0, y0, _x1, y1 = bbox(pg.composite_keys(grid, i))
            self.assertEqual((y1, y1 - y0 + 1), (55, h))


if __name__ == "__main__":
    unittest.main()
