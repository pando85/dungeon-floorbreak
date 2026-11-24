"""AREA-GOBLINWORKSHOP-01: the Goblin Workshop prop kit sources, configs, manifests and spec contract."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_gwkit_frames as frames  # noqa: E402
import author_gwkit_keys as keys  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
MANIFESTS = ROOT / "assets/manifests"
GRIDS = sorted(frames.assets())

# Canvas and pivot per grid: spec section 3 (the tells: section 5).
SPEC = {
    "gw_boiler": ((192, 208), (96, 207)), "gw_valve": ((32, 48), (16, 47)), "gw_brace": ((32, 40), (16, 39)),
    "gw_weight": ((64, 48), (32, 47)), "gw_remnant": ((64, 32), (32, 31)), "gw_drum": ((36, 36), (18, 35)),
    "gw_scorch": ((48, 8), (24, 7)), "gw_rig": ((112, 24), (0, 0)), "gw_chain": ((4, 8), (0, 0)),
    "gw_tell_sputter": ((16, 16), (0, 0)), "gw_tell_plume": ((80, 96), (0, 0)),
    "gw_tell_shadow": ((64, 8), (0, 0)), "gw_tell_ring": ((96, 8), (0, 0)),
}
# The frozen key frames (grid frame index) of the 13 keys of spec section 4.
KEYS = {"gw_boiler": [0, 6], "gw_valve": [0, 6], "gw_brace": [0, 1, 3], "gw_weight": [0], "gw_remnant": [0],
        "gw_drum": [0], "gw_scorch": [0], "gw_rig": [0], "gw_chain": [0]}
# Frames of the kit that exist only to show a state: tag -> frame count, from the spec tables.
TAGS = {
    "gw_boiler": {"boiler_idle": 4, "boiler_strain": 2, "boiler_damaged": 2},
    "gw_valve": {"valve_intact": 1, "valve_warning": 2, "valve_venting": 3, "valve_spent": 3},
    "gw_brace": {"brace_supported": 1, "brace_weakened": 2, "brace_stump": 1},
    "gw_weight": {"weight_hang": 1, "weight_fall": 1}, "gw_remnant": {"remnant": 1},
    "gw_drum": {"drum_armed": 1, "drum_telegraph": 2}, "gw_scorch": {"scorch": 3},
    "gw_rig": {"rig": 1, "rig_run": 2}, "gw_chain": {"chain": 1},
    "gw_tell_sputter": {"steam_sputter": 3}, "gw_tell_plume": {"steam_plume": 4},
    "gw_tell_shadow": {"fall_shadow_tell": 2, "fall_shadow": 1}, "gw_tell_ring": {"drum_blast_tell": 2},
}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


class KitSourcesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.built = frames.assets()
        cls.pal = keys.kit_palette()

    def test_grids_configs_and_manifests_reproduce_byte_for_byte(self) -> None:
        for name, (tags, pivot) in self.built.items():
            grid = frames.to_grid(tags, pivot, self.pal)
            self.assertEqual(json.dumps(grid, indent=1) + "\n", (PROPS / f"{name}.grid.json").read_text(), name)
            self.assertEqual(json.dumps(frames.config_of(name, tags, grid), indent=2) + "\n",
                             (PROPS / f"{name}.pixelart.json").read_text(), name)
            self.assertEqual(json.dumps(frames.manifest_of(name, tags, grid), indent=2) + "\n",
                             (MANIFESTS / f"{frames.asset_stem(name)}.json").read_text(), name)

    def test_canvas_pivot_and_tags_follow_the_spec(self) -> None:
        for name in GRIDS:
            grid = load(PROPS / f"{name}.grid.json")
            self.assertEqual((tuple(grid["canvas"]), tuple(grid["pivot"])), SPEC[name], name)
            self.assertEqual({t["name"]: t["to"] - t["from"] + 1 for t in grid["tags"]}, TAGS[name], name)

    def test_palette_is_the_named_kit_palette_and_stays_within_the_budget(self) -> None:
        used: set[str] = set()
        for name in GRIDS:
            grid = load(PROPS / f"{name}.grid.json")
            for entry in grid["palette"]:
                self.assertEqual(self.pal[entry["name"]], entry["rgba"], f"{name} {entry['name']}")
                self.assertTrue(entry["rgba"].endswith("ff"), "binary alpha")
                used.add(entry["name"])
        self.assertLessEqual(len(used), 63, sorted(used))

    def test_no_isolated_pixels_and_every_frame_has_a_transparent_corner(self) -> None:
        for name in GRIDS:
            grid = load(PROPS / f"{name}.grid.json")
            report = pg.lint_grid(grid)
            self.assertEqual(sum(report["isolated_count_per_frame"]), 0, name)

    def test_frozen_keys_are_the_key_functions_and_the_fixtures_hold(self) -> None:
        for name, indices in KEYS.items():
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            self.assertEqual(cfg.rules["frozen_keys"]["frames"], indices, name)
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], f"{name}: {validators.summary(report)}")
        # Spot check: the key frames of the grids equal what the key script draws.
        grid = load(PROPS / "gw_boiler.grid.json")
        intact = keys.boiler_intact()
        names = {p["key"]: p["name"] for p in grid["palette"]}
        rows = [[names.get(c) for c in row] for row in grid["frames"][0]["cels"]["body"]]
        self.assertEqual(rows, intact.p)

    def test_every_config_validates(self) -> None:
        for name in GRIDS:
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], f"{name}: {validators.summary(report)}")

    def test_grounded_props_keep_their_bottom_row(self) -> None:
        """Bottom-row drift 0 within a prop (spec acceptance 1): every frame of a grounded tag ends on the pivot row."""
        for name in GRIDS:
            cfg = load_config(PROPS / f"{name}.pixelart.json")
            ctx = validators.build_context(cfg)
            for tag, spec in cfg.tags.items():
                if spec["contact"] != "grounded":
                    continue
                for f, _ in spec["frames"]:
                    self.assertEqual(ctx.boxes[f][3], cfg.pivot[1], f"{name} {tag} frame {f}")

    def test_manifest_frames_carry_the_digests_of_the_grid(self) -> None:
        for name in GRIDS:
            grid = load(PROPS / f"{name}.grid.json")
            man = load(MANIFESTS / f"{frames.asset_stem(name)}.json")
            self.assertEqual(man["review_status"], "prototype_fixture_unapproved")
            self.assertEqual(man["grid_source"], f"assets/source/props/floor01/{name}.grid.json")
            for i, row in enumerate(man["prop_kit"]["frames"]):
                self.assertEqual(row["sha256"], validators.frame_digest(grid["frames"][i]["cels"], ["body"]))
                self.assertEqual(row["duration_ms"], grid["frames"][i]["duration_ms"])

    def test_durations_are_the_tick_tables(self) -> None:
        """Timings in ticks at 60 Hz (spec tables): loops 8 / 4 / 10 / 6 ticks per frame."""
        expect = {("gw_boiler", "boiler_idle"): 8, ("gw_boiler", "boiler_strain"): 4, ("gw_boiler", "boiler_damaged"): 10,
                  ("gw_valve", "valve_warning"): 4, ("gw_valve", "valve_venting"): 4, ("gw_valve", "valve_spent"): 10,
                  ("gw_brace", "brace_weakened"): 6, ("gw_rig", "rig_run"): 6, ("gw_drum", "drum_telegraph"): 4,
                  ("gw_scorch", "scorch"): 10,
                  ("gw_tell_sputter", "steam_sputter"): 4, ("gw_tell_plume", "steam_plume"): 4,
                  ("gw_tell_shadow", "fall_shadow_tell"): 4, ("gw_tell_ring", "drum_blast_tell"): 4}
        for (name, tag), ticks in expect.items():
            grid = load(PROPS / f"{name}.grid.json")
            t = next(t for t in grid["tags"] if t["name"] == tag)
            for i in range(t["from"], t["to"] + 1):
                self.assertEqual(grid["frames"][i]["duration_ms"], round(ticks * 1000 / 60), f"{name} {tag}")

    def test_the_gate_approved_edits_are_the_only_key_changes(self) -> None:
        """valve_spent and the remnant are the two keys re-frozen under the gate approval (spec section 8)."""
        valve = load(PROPS / "gw_valve.grid.json")
        names = {p["key"]: p["name"] for p in valve["palette"]}
        spent = [[names.get(c) for c in row] for row in valve["frames"][6]["cels"]["body"]]
        self.assertEqual(spent, keys.valve_spent().p)
        petals = {spent[y][x] for y in range(10, 33) for x in range(27, 31)}
        self.assertTrue({"copper_02", "copper_03"} <= petals, "the torn petals are filled with copper")

    def test_the_review_approved_weight_is_a_slab_stack_with_vertical_sides(self) -> None:
        """PR 256 review, follow-up 3: the weight key is re-frozen as a slab stack (no flared lamp-shade sides)."""
        grid = load(PROPS / "gw_weight.grid.json")
        names = {p["key"]: p["name"] for p in grid["palette"]}
        hang = [[names.get(c) for c in row] for row in grid["frames"][0]["cels"]["body"]]
        self.assertEqual(hang, keys.weight_hang().p)
        slab_rows = [16 + y for y in (*range(8, 15), *range(16, 24), *range(25, 32))]
        edges = {(row.index(next(c for c in row if c)), 63 - row[::-1].index(next(c for c in row[::-1] if c)))
                 for row in (hang[y] for y in slab_rows)}
        self.assertEqual(edges, {(2, 61)}, "every slab row spans the same 60 px: vertical sides")

    def test_the_scorch_fade_keeps_the_key_and_cools_the_embers(self) -> None:
        grid = load(PROPS / "gw_scorch.grid.json")
        names = {p["key"]: p["name"] for p in grid["palette"]}
        rows = [[[names.get(c) for c in row] for row in f["cels"]["body"]] for f in grid["frames"]]
        self.assertEqual(rows[0], keys.drum_scorch().p)
        hot = [sum(c in ("flame_01", "flame_02") for row in f for c in row) for f in rows]
        self.assertEqual(hot[-1], 0, "the held last frame has no glowing ember")
        self.assertTrue(hot[0] > hot[1] >= hot[2])


if __name__ == "__main__":
    unittest.main()
