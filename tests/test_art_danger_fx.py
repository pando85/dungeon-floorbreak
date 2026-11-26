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
    "dfx_gate": ((28, 224), (0, 223), {"gate_closed": 1, "gate_raised": 1}),
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
        closed, raised = (f["cels"]["body"] for f in grid["frames"])
        self.assertEqual(len(closed[0]), 28)
        self.assertTrue(closed[0].strip("."), "the closed gate reaches the ceiling")
        self.assertTrue(closed[223].strip("."), "the teeth bite the floor on the last row")
        self.assertTrue(all(not row.strip(".") for row in raised[fx.GATE_RAISED_ROWS:]),
                        "the raised gate hangs only under the lintel")

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
