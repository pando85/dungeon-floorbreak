"""F01 danger FX: the acid, ring, wave, gate and room exit sources, configs, manifests and spec contract."""
from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_danger_fx as fx  # noqa: E402
import author_gwkit_keys as keys  # noqa: E402
import author_hazard_decals as hz  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
MANIFESTS = ROOT / "assets/manifests"

# Canvas, pivot and tags (name -> frame count) per grid: spec section 2.
SPEC = {
    "dfx_acid": ((80, 10), (0, 9), {"acid_idle": 4, "acid_drain": 2}),
    "dfx_ring_r48": ((103, 103), (51, 51), {"ring_march": 2, "ring_hot": 2}),
    "dfx_ring_r64": ((135, 135), (67, 67), {"ring_march": 2, "ring_hot": 2}),
    "dfx_wave_r48": ((103, 103), (51, 51), {"blast_wave": 6}),
    "dfx_wave_r64": ((135, 135), (67, 67), {"blast_wave": 6}),
    "dfx_gate": ((28, 224), (0, 223), {"gate_closed": 1, "gate_raised": 1, "gate_drop": 4}),
    "dfx_slam_dust": ((44, 20), (4, 19), {"slam_dust": 5}),
    "dfx_exit": ((96, 136), (0, 135), {"exit_closed": 1, "exit_open": 1}),
}
WAVE_TICKS = [2, 2, 2, 6, 5, 5]


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


class DangerFxSourcesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.built = fx.assets()
        cls.pal = fx.palette()

    def test_grids_configs_and_manifests_reproduce_byte_for_byte(self) -> None:
        # The AREA-HAZARDS-01B sheets are listed and checked in tests/test_art_hazard_decals.py.
        self.assertEqual(sorted(self.built), sorted(set(SPEC) | set(hz.HAZARD_SHEETS)))
        for name, (tags, pivot) in self.built.items():
            grid = fx.to_grid(tags, pivot, self.pal, hz.key_map(name))
            self.assertEqual(json.dumps(grid, indent=1) + "\n", (PROPS / f"{name}.grid.json").read_text(), name)
            cfg = json.dumps(fx.config_of(name, tags, grid), indent=2) + "\n"
            self.assertEqual(cfg, (PROPS / f"{name}.pixelart.json").read_text(), name)
            man = json.dumps(fx.manifest_of(name, tags, grid), indent=2) + "\n"
            self.assertEqual(man, (MANIFESTS / f"{fx.asset_stem(name)}.json").read_text(), name)

    def test_canvases_pivots_and_tags_follow_the_spec(self) -> None:
        for name, (size, pivot, tags) in SPEC.items():
            grid = load(PROPS / f"{name}.grid.json")
            self.assertEqual(tuple(grid["canvas"]), size, name)
            self.assertEqual(tuple(grid["pivot"]), pivot, name)
            got = {t["name"]: t["to"] - t["from"] + 1 for t in grid["tags"]}
            self.assertEqual(got, tags, name)
        man = load(MANIFESTS / "f01-dfx-wave-r64.json")
        self.assertEqual([f["ticks"] for f in man["prop_kit"]["frames"]], WAVE_TICKS)

    def test_palette_entries_are_named_and_valued_exactly(self) -> None:
        kit = keys.kit_palette()
        ramp = dict(hz.ACID_RAMP)  # HZB-AD1: the acid has its own teal ramp
        kit_names: set[str] = set()
        for name in SPEC:
            for p in load(PROPS / f"{name}.grid.json")["palette"]:
                if name == "dfx_acid":
                    self.assertEqual(ramp.get(p["name"]), p["rgba"], p["name"])
                else:
                    self.assertEqual(kit.get(p["name"]), p["rgba"], p["name"])
                    kit_names.add(p["name"])
                self.assertTrue(p["rgba"].endswith("ff"), "binary alpha: every palette entry is opaque")
        self.assertLessEqual(len(kit_names), 63)

    def test_every_config_passes_the_toolkit_validators(self) -> None:
        for name in SPEC:
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], f"{name}: {validators.summary(report)}")

    def test_the_ring_and_wave_reach_is_the_sim_radius(self) -> None:
        for r in fx.RADII:
            c = r + fx.MARGIN
            for kind in ("ring", "wave"):
                grid = load(PROPS / f"dfx_{kind}_r{r}.grid.json")
                for i, frame in enumerate(grid["frames"]):
                    for y, row in enumerate(frame["cels"]["body"]):
                        for x, k in enumerate(row):
                            if k == ".":
                                continue
                            d = math.hypot(x - c, y - c)
                            self.assertLess(d, r + 2 if kind == "ring" else r + 1, f"{kind} r{r} frame {i}")

    def test_the_gate_covers_its_collider_and_bites_the_floor(self) -> None:
        grid = load(PROPS / "dfx_gate.grid.json")
        closed, raised, *drop = (f["cels"]["body"] for f in grid["frames"])
        self.assertEqual(len(closed[0]), 28)
        self.assertTrue(closed[0].strip("."), "the closed gate reaches the ceiling")
        self.assertTrue(closed[223].strip("."), "the teeth bite the floor on the last row")
        self.assertTrue(all(not row.strip(".") for row in raised[fx.GATE_RAISED_ROWS:]),
                        "the raised gate hangs only under the lintel")

    def test_the_gate_drop_falls_from_the_raised_height_to_the_floor(self) -> None:
        """F01-SIM-TICKS: four drop frames between the raised gate (lowest grate row 19) and
        the closed one (row 219), the tips going down faster every frame (constant gravity),
        the grate columns of each frame a pure vertical shift of the closed grate's, and the
        jamb (columns 24..27) the closed frame's from the first drop frame."""
        grid = load(PROPS / "dfx_gate.grid.json")
        closed, raised, *drop = (f["cels"]["body"] for f in grid["frames"])
        self.assertEqual(len(drop), 4)
        self.assertEqual([f["duration_ms"] for f in grid["frames"][2:]], [33] * 4)

        def tips(frame: list[str]) -> int:
            return max(y for y, row in enumerate(frame) if row[:24].strip("."))

        rows = [tips(raised)] + [tips(f) for f in drop] + [tips(closed)]
        # 200 - 2 t^2 px above the closed position at t = 2, 4, 6, 8 ticks.
        self.assertEqual(rows, [19, 27, 51, 91, 147, 219])
        steps = [b - a for a, b in zip(rows, rows[1:])]
        self.assertEqual(steps, sorted(steps), "it accelerates")
        for frame, row in zip(drop, rows[1:-1]):
            shift = 219 - row
            for y in range(fx.GH):
                self.assertEqual(frame[y][24:], closed[y][24:], f"jamb row {y}")
                src = y + shift
                want = closed[src][:24] if src < fx.GH else "." * 24
                self.assertEqual(frame[y][:24], want, f"grate row {y} of the drop to row {row}")

    def test_the_slam_dust_stands_on_the_floor_and_rolls_away_from_the_impact(self) -> None:
        """F01-SIM-TICKS: every dust frame touches the walking line (the bottom row) or rises
        off it in the late frames only, nothing is drawn left of the pivot column but the
        4 px tucked under the object, and the 20 ticks play once."""
        grid = load(PROPS / "dfx_slam_dust.grid.json")
        frames = [f["cels"]["body"] for f in grid["frames"]]
        self.assertEqual([f["duration_ms"] for f in grid["frames"]], [33, 50, 67, 83, 100])
        pal = {p["key"]: p["name"] for p in grid["palette"]}
        self.assertTrue(set(pal.values()) <= {f"stone_0{i}" for i in range(3, 8)} | {"iron_01", "iron_03"})
        for i, frame in enumerate(frames):
            self.assertTrue(frame[19].strip("."), f"frame {i} stands on the floor")
            xs = [x for row in frame for x, k in enumerate(row) if k != "."]
            self.assertLessEqual(min(xs), 4, f"frame {i} starts at the impact edge")
        reach = [max(x for row in f for x, k in enumerate(row) if k != ".") for f in frames]
        self.assertEqual(reach[:3], sorted(reach[:3]), "the front rolls out")

    def test_the_exit_opening_is_the_exit_gate_and_the_grate_bites_the_floor(self) -> None:
        """ROOM-PORTALS-01: the opening is the 64 x 96 exit gate collider (columns 16..79,
        rows 40..135); closed, the grate's teeth reach the floor row in every bar column;
        open, the grate hangs only EXIT_RAISED_ROWS under the slot; the jambs and the
        header are identical in both frames (only the grate moves)."""
        grid = load(PROPS / "dfx_exit.grid.json")
        pal = {p["key"]: p["name"] for p in grid["palette"]}
        closed, opened = (f["cels"]["body"] for f in grid["frames"])
        steel = {"stone_cold_00", "stone_cold_01", "stone_cold_02", "stone_cold_03", "stone_cold_04"}
        self.assertEqual(fx.EW - 2 * fx.EJ, 64)
        self.assertEqual(fx.EH - fx.EO_TOP, 96)
        for bx in fx.EXIT_BARS:
            self.assertIn(pal[closed[fx.EH - 1][bx + 1]], steel, f"bar {bx} tooth on the floor row")
        for y in range(fx.EO_TOP + fx.EXIT_RAISED_ROWS, fx.EH):
            row = "".join(opened[y][fx.EJ:fx.EW - fx.EJ])
            self.assertFalse(any(pal[k] in steel for k in row), f"open row {y} is clear of the grate")
        for y in range(fx.EH):
            for x in list(range(fx.EJ)) + list(range(fx.EW - fx.EJ, fx.EW)):
                self.assertEqual(closed[y][x], opened[y][x], f"jamb ({x}, {y})")
        for y in range(fx.EO_TOP):
            self.assertEqual(closed[y], opened[y], f"header row {y}")
        corners = {(0, 0), (1, 0), (0, 1), (fx.EW - 1, 0), (fx.EW - 2, 0), (fx.EW - 1, 1)}
        for frame in (closed, opened):
            holes = {(x, y) for y, row in enumerate(frame) for x, k in enumerate(row) if k == "."}
            self.assertEqual(holes, corners, "opaque but for the cap's chamfered corners")


if __name__ == "__main__":
    unittest.main()
