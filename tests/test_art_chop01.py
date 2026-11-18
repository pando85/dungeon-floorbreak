"""CHOP-01 contract tests: the authoring record is reproducible, the .ase matches the grid, the keys are frozen,
the 60 Hz tick table matches the spec, the palette stays within 20 colours and shares the GOB-01 crew keys, the
club is not a pineapple, and the six keys read as tell / attack / hurt / defeat by simple bbox and colour rules."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_chop01 as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

CHOP = ROOT / "assets/source/characters/goblin/chopper"
CONFIG = CHOP / "chop_01.pixelart.json"
GRID = CHOP / "chop_01.grid.json"
ASE = CHOP / "chop_01.ase"
MANIFEST = ROOT / "assets/manifests/chop-01.json"
FIXTURE = ROOT / "tests/fixtures/chop-01_keys.json"
GOB_GRID = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
SPEC_TICKS = {"idle": 32, "walk": 20, "attack_tell": 30, "attack_swing": 12, "hurt": 12, "down": 16}
PINEAPPLE = set("FGHVv")  # GOB-01 pineapple and leaf keys


def flat(grid: dict, n: int) -> list[list[str]]:
    return pg.composite_keys(grid, n)


def bbox(rows) -> tuple[int, int, int, int]:
    pts = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


class Chop01Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = json.loads(CONFIG.read_text())
        self.grid = json.loads(GRID.read_text())

    def test_authoring_script_reproduces_grid_and_manifest(self) -> None:
        self.assertEqual(author.dump(author.build_grid()), GRID.read_text())
        self.assertEqual(json.dumps(author.build_manifest(), indent=2) + "\n", MANIFEST.read_text())

    def test_ase_matches_the_grid(self) -> None:
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [112, 64])
        self.assertEqual(len(ase["frames"]), 16)
        self.assertEqual([f["duration_ms"] for f in ase["frames"]], [f["duration_ms"] for f in self.grid["frames"]])

    def test_frame_size_pivot_and_layers(self) -> None:
        self.assertEqual(self.grid["canvas"], [112, 64])
        self.assertEqual(self.grid["pivot"], [56, 63])
        self.assertEqual(self.grid["layers"], ["machine", "crew", "weapon"])
        self.assertEqual(json.loads(MANIFEST.read_text())["pivot"], [56, 63])

    def test_keys_are_frozen_and_inbetweens_are_copies(self) -> None:
        fixture = json.loads(FIXTURE.read_text())
        self.assertEqual(set(self.cfg["rules"]["frozen_keys"]["frames"]), set(author.KEY_FRAMES))
        self.assertEqual(set(author.KEY_FRAMES) | set(author.INBETWEENS), set(range(16)))
        for n, src in author.COPIES.items():
            self.assertEqual(self.grid["frames"][n]["cels"], self.grid["frames"][src]["cels"], n)
        self.assertTrue(fixture)

    def test_tick_windows_match_the_spec(self) -> None:
        for tag, total in SPEC_TICKS.items():
            ticks = [t for _f, t in self.cfg["tags"][tag]["frames"] if t is not None]
            self.assertEqual(sum(ticks), total, tag)
        self.assertGreaterEqual(dict(map(tuple, self.cfg["tags"]["attack_tell"]["frames"]))[8], 22)
        self.assertIsNone(dict(map(tuple, self.cfg["tags"]["down"]["frames"]))[15])
        att = json.loads(MANIFEST.read_text())["chopper"]["attack_ticks"]
        self.assertEqual(att, {"tell": [0, 29], "swing_smear": [30, 32], "swing_impact": [33, 41], "hitbox_active": [33, 38]})

    def test_palette_within_budget_and_crew_keys_match_gob01(self) -> None:
        keys = {e["key"]: e["rgba"] for e in self.grid["palette"]}
        self.assertLessEqual(len(keys), 20)
        gob = {e["key"]: e["rgba"] for e in json.loads(GOB_GRID.read_text())["palette"]}
        for k in "KasmlewRtu":  # outline, skin ramp, eye, tooth, scarf red and rags are the GOB-01 values
            self.assertEqual(keys[k], gob[k], k)

    def test_club_is_iron_not_a_pineapple(self) -> None:
        for n in author.KEY_FRAMES:
            for row in self.grid["frames"][n]["cels"]["weapon"]:
                self.assertFalse(PINEAPPLE & set(row), f"frame {n}")
        self.assertTrue(any(set(r) - {"."} for r in self.grid["frames"][0]["cels"]["weapon"]))

    def test_every_frame_touches_the_contact_row_and_stays_inside(self) -> None:
        for n in range(16):
            x0, y0, x1, y1 = bbox(flat(self.grid, n))
            self.assertEqual(y1, 63, n)
            self.assertGreaterEqual(x0, 2, n)
            self.assertLessEqual(x1, 111, n)

    def test_size_decision_idle(self) -> None:
        x0, y0, x1, y1 = bbox(flat(self.grid, 0))
        self.assertEqual((x1 - x0 + 1, y1 - y0 + 1), (95, 52))
        self.assertEqual(min(y for y, r in enumerate(self.grid["frames"][0]["cels"]["crew"])
                             if r.strip(".")), 17)  # helmet outline row: 47 px body height (rows 17 to 63)

    def test_tell_attack_hurt_defeat_read(self) -> None:
        top = {n: bbox(flat(self.grid, n))[1] for n in author.KEY_FRAMES}
        self.assertEqual(min(top, key=top.get), 8)                 # tell is the highest pose
        right = {n: bbox(flat(self.grid, n))[2] for n in author.KEY_FRAMES}
        self.assertEqual(max(right, key=right.get), 10)            # the ram lunge reaches furthest forward
        self.assertGreaterEqual(right[10] - right[0], 5)           # by at least 5 px over idle
        self.assertLess(right[11], right[0])                       # hurt knocks the machine back
        self.assertGreater(top[15], top[0] + 5)                    # wreck sits low, grounded (no floating parts)

    def test_bike_wheels_sit_in_front_of_the_tub_and_the_wreck_is_grounded(self) -> None:
        # rear bike wheel hub (x 58) is drawn over the tub, so both bike wheels read at 1x
        machine = self.grid["frames"][0]["cels"]["machine"]
        hub = {machine[y][x] for y in range(48, 59) for x in range(53, 64) if (x - 58) ** 2 + (y - 53) ** 2 <= 25}
        self.assertFalse(hub & set("Rp"), hub)  # no tub red over the rear wheel
        self.assertTrue(hub & set("cCD"), hub)
        rows = flat(self.grid, 15)
        for x in (20, 40, 60, 90):  # tub, engine and burst tyres all rest on the contact row region
            self.assertTrue(any(rows[y][x] != "." for y in range(60, 64)), x)

    def test_clean_pixels(self) -> None:
        self.assertLessEqual(self.cfg["rules"]["max_orphans_per_frame"], 4)
        for n in author.KEY_FRAMES:
            orphans, isolated = pg._stray_pixels(flat(self.grid, n), author.ACCENTS)
            self.assertEqual((len(orphans), len(isolated)), (0, 0), n)

    def test_glow_escalates_into_the_tell_and_dies_in_the_wreck(self) -> None:
        def count(n, key):
            return sum(r.count(key) for layer in self.grid["frames"][n]["cels"].values() for r in layer)
        self.assertGreater(count(8, "Y"), count(0, "Y"))
        self.assertEqual(count(15, "Y"), 0)
        self.assertEqual(count(11, "e"), 0)


if __name__ == "__main__":
    unittest.main()
