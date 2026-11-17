#!/usr/bin/env python3
"""ENV-02 (design phase 1): extension contract, route proposal layouts and design rules."""
from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import env_kit as ek  # noqa: E402

BASE = "assets/source/terrain/floor01/floor01_catacomb_kit.json"
CONTRACT = "assets/source/terrain/floor01/floor01_opening_kit.json"
LAYOUT_R = "assets/levels/floor01/opening_route_proposal.json"
LAYOUT_T = "assets/levels/floor01/opening_route_proposal_ceiling_t.json"
EXPORTED = (ROOT / "generated/art/floor01_opening_props.png").is_file()
CARL_JAB_REACH = (16, 22)      # src/combat.rs CARL_JAB fist box, px ahead of the pivot
CARL_HALF_WIDTH = 12           # generous half-width of the 52 px Carl idle silhouette


def marker(layout: dict, mid: str) -> dict:
    return next(m for m in layout["markers"] if m["id"] == mid)


def overlaps(a, b) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah


class ExtendsTests(unittest.TestCase):
    """``extends`` merges an extension contract onto its base without touching the base."""

    def write(self, tmp: Path, name: str, data: dict) -> Path:
        path = tmp / name
        path.write_text(json.dumps(data), encoding="utf-8")
        return path

    def resolve(self, data: dict):
        (ROOT / "generated").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "generated") as tmp:
            path = self.write(Path(tmp), "ext.json", data)
            return ek.resolve_extends(data, path.resolve())

    def ext(self, **extra) -> dict:
        data = {"format": "env-kit-v2", "kit_id": "ext_kit", "extends": BASE, "atlases": {},
                "tiles": [], "pieces": []}
        data.update(extra)
        return data

    def test_merges_base_and_extension(self) -> None:
        base = ek.load_json(BASE)
        merged = self.resolve(self.ext(pieces=[{"id": "new_piece"}]))
        self.assertEqual(merged["kit_id"], "ext_kit")
        self.assertEqual(merged["palette"], base["palette"])
        self.assertEqual(merged["layers"], base["layers"])
        self.assertEqual(len(merged["pieces"]), len(base["pieces"]) + 1)
        self.assertEqual(merged["tiles"], base["tiles"])
        self.assertEqual(merged["extends_chain"], [BASE])

    def test_rejects_redeclared_inherited_fields(self) -> None:
        for key in ("palette", "layers", "walk_line", "lighting", "coordinate_system"):
            with self.subTest(key=key), self.assertRaisesRegex(ek.KitError, "redeclares"):
                self.resolve(self.ext(**{key: {}}))

    def test_rejects_a_base_atlas_redefinition(self) -> None:
        with self.assertRaisesRegex(ek.KitError, "redefines base atlas"):
            self.resolve(self.ext(atlases={"floor01_props": {}}))

    def test_rejects_self_extension_and_cycles(self) -> None:
        (ROOT / "generated").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "generated") as tmp:
            rel = Path(tmp).resolve().relative_to(ROOT).as_posix()
            a = {"format": "env-kit-v2", "kit_id": "a", "extends": f"{rel}/b.json"}
            b = {"format": "env-kit-v2", "kit_id": "b", "extends": f"{rel}/a.json"}
            pa = self.write(Path(tmp), "a.json", a)
            self.write(Path(tmp), "b.json", b)
            with self.assertRaisesRegex(ek.KitError, "cycle|extends itself"):
                ek.resolve_extends(a, pa.resolve())
            selfref = {"format": "env-kit-v2", "kit_id": "s", "extends": f"{rel}/s.json"}
            ps = self.write(Path(tmp), "s.json", selfref)
            with self.assertRaisesRegex(ek.KitError, "extends itself"):
                ek.resolve_extends(selfref, ps.resolve())

    def test_base_contract_has_no_extends_and_is_unchanged_by_loading(self) -> None:
        before = (ROOT / BASE).read_bytes()
        self.assertNotIn("extends", ek.load_json(BASE))
        if EXPORTED:
            ek.load_kit(CONTRACT)
        self.assertEqual((ROOT / BASE).read_bytes(), before)


@unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
class OpeningRouteTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.kit = ek.load_kit(CONTRACT)
        cls.r = ek.load_layout(LAYOUT_R)
        cls.t = ek.load_layout(LAYOUT_T)

    def test_contract_and_both_ceiling_options_validate(self) -> None:
        ek.validate_contract(self.kit)
        ek.validate_layout(self.kit, self.r)
        ek.validate_layout(self.kit, self.t)

    def test_extension_adds_pieces_only_and_no_palette_colours(self) -> None:
        ext = ek.load_json(CONTRACT)
        self.assertEqual(ext["tiles"], [])
        for key in ek.INHERITED_KEYS:
            self.assertNotIn(key, ext)
        base_ids = {p["id"] for p in ek.load_json(BASE)["pieces"]}
        self.assertFalse(base_ids & {p["id"] for p in ext["pieces"]})

    def test_duplicate_piece_id_is_rejected(self) -> None:
        kit = ek.Kit(copy.deepcopy(self.kit.contract), self.kit.atlases, self.kit.path)
        dup = copy.deepcopy(next(p for p in kit.contract["pieces"] if p["id"] == "hazard_grit"))
        dup["id"] = "torch_wall_unlit"
        kit.contract["pieces"].append(dup)
        with self.assertRaisesRegex(ek.KitError, "duplicate id"):
            ek.validate_contract(kit)

    def test_ids_are_ip_neutral(self) -> None:
        ids = [p["id"] for p in ek.load_json(CONTRACT)["pieces"]]
        for layout in (self.r, self.t):
            ids += [m["id"] for m in layout["markers"]] + [s["id"] for s in layout["segments"]]
            ids += [m.get("archetype", "") for m in layout["markers"]]
            ids.append(layout["layout_id"])
        for i in ids:
            self.assertIsNone(ek.NON_NEUTRAL.search(i), i)

    def test_state_groups_share_one_rect_size(self) -> None:
        groups: dict[str, set] = {}
        for p in ek.load_json(CONTRACT)["pieces"]:
            if "state_of" in p and p["state_of"]["state"] not in ("falling", "landed"):
                groups.setdefault(p["state_of"]["group"], set()).add(tuple(p["rect"][2:]))
        self.assertEqual(set(groups), {"raking_shore", "raking_shore_load", "timber_set",
                                       "timber_barricade", "guild_portal"})
        for g, sizes in groups.items():
            self.assertEqual(len(sizes), 1, g)

    def test_options_differ_only_in_the_lowered_ceiling(self) -> None:
        lo, hi = 61, 104
        for name in ("collision", "backwall"):
            a, b = self.r["tile_layers"][name], self.t["tile_layers"][name]
            for r, (ra, rb) in enumerate(zip(a, b)):
                for c, (ta, tb) in enumerate(zip(ra, rb)):
                    if not lo <= c <= hi:
                        self.assertEqual(ta, tb, (name, r, c))

    def test_clearance_matches_the_design_parameter(self) -> None:
        cols, rows = self.r["grid_size"]
        for layout in (self.r, self.t):
            opt = layout["design_parameters"]["pursuer_clearance"]
            spec = opt["options"][opt["selected"]]
            gauge = marker(layout, "clearance_gauge_s3")
            grid = layout["tile_layers"]["collision"]
            tops = ek.collider_tops(self.kit, grid, cols, rows)
            x0, w = gauge["rect"][0], gauge["rect"][2]
            for c in range(x0 // 16, (x0 + w) // 16):
                under = 0
                while ek._is_solid(self.kit, grid[under][c]):
                    under += 1
                self.assertEqual(under * 16, spec["underside_y"], c)
                # the pursuer path runs on the base walking line (it crushes the 1-tile humps)
                if tops[c] == layout["base_walk_line_y"]:
                    self.assertEqual(tops[c] - under * 16, spec["clearance_px"], c)
                else:
                    self.assertEqual(tops[c], layout["base_walk_line_y"] - 16, c)
            self.assertGreaterEqual(spec["clearance_px"] - spec["fits_pursuer_total_height_max_px"], 16)
        doz = self.r["design_parameters"]["pursuer_interface"]
        for layout in (self.r, self.t):
            for spec in layout["design_parameters"]["pursuer_clearance"]["options"].values():
                self.assertGreaterEqual(spec["clearance_px"], doz["ceiling_min_px"])
                self.assertGreaterEqual(spec["fits_pursuer_total_height_max_px"], doz["total_height_px"])
        self.assertGreaterEqual(self.r["design_parameters"]["pursuer_clearance"]["options"]["R"]
                                ["clearance_px"], doz["ceiling_recommended_px"])

    def test_refuge_is_too_low_for_the_pursuer_and_reachable_inside_the_charge(self) -> None:
        doz = self.r["design_parameters"]["pursuer_interface"]
        pocket = marker(self.r, "safe_pocket_alcove")
        lo, hi = doz["refuge_clear_height_px"]
        self.assertTrue(lo <= pocket["clear_height_px"] <= hi)
        self.assertEqual(pocket["clear_height_px"], pocket["rect"][3])
        self.assertLess(pocket["clear_height_px"], doz["total_height_px"])
        self.assertGreaterEqual(pocket["rect"][2], 24 + 44)      # Carl and Donut side by side
        charge = marker(self.r, "pursuer_charge_zone")
        dist = charge["refuge_entry_x"] - charge["x"]
        self.assertEqual(dist, charge["refuge_distance_px"])
        for speed in (145, self.r["design_parameters"]["hero_run_px_s"]):
            ticks = dist / speed * 60
            self.assertLess(ticks, doz["charge_ticks"] - 10, speed)   # 10-tick margin for the hop
        # when the pursuer's front hits the barricade its body still covers the refuge entrance,
        # so the barricade opens before anyone can leave the alcove into its path
        barricade = marker(self.r, "barricade")
        self.assertLess(barricade["rect"][0] - charge["refuge_entry_x"], doz["length_px"])
        self.assertEqual(barricade["rect"][1], 32)               # full height: nobody hops over it

    def test_strike_position_is_outside_the_fall_region(self) -> None:
        for layout in (self.r, self.t):
            hazard = marker(layout, "hazard_fall_region")["rect"]
            shore = marker(layout, "breakable_shore")
            block = marker(layout, "falling_block")
            carl_pivot = shore["x"] - (CARL_JAB_REACH[0] + CARL_JAB_REACH[1]) // 2
            self.assertLess(carl_pivot + CARL_HALF_WIDTH, hazard[0])
            self.assertEqual(shore["hits_to_break"], 3)
            walk = layout["base_walk_line_y"]
            band = shore["strike_band_y"]
            self.assertTrue(walk - 37 <= band[0] < band[1] <= walk - 18)   # jab and kick heights
            bx, _by, bw, _bh = block["rect"]
            self.assertTrue(hazard[0] <= bx and bx + bw <= hazard[0] + hazard[2])
            self.assertEqual(hazard[1] + hazard[3], walk)
            self.assertEqual(block["land_y"], walk)

    def test_protected_exit_is_clear_of_hazards_and_scripted_pieces(self) -> None:
        for layout in (self.r, self.t):
            protected = marker(layout, "protected_exit_region")["rect"]
            exit_m = marker(layout, "exit_guild")
            self.assertTrue(protected[0] <= exit_m["rect"][0])
            for m in layout["markers"]:
                if m["kind"] in ("hazard", "breakable_prop", "falling_debris", "scripted_blocker",
                                 "pursuer_breakable", "pursuer_end", "enemy_spawn"):
                    self.assertFalse(overlaps(m.get("rect", [m["x"], m["y"], 1, 1]), protected), m["id"])

    def test_safe_pocket_sits_on_the_ledge(self) -> None:
        cols, rows = self.r["grid_size"]
        tops = ek.collider_tops(self.kit, self.r["tile_layers"]["collision"], cols, rows)
        pocket = marker(self.r, "safe_pocket_alcove")
        rx, ry, rw, rh = pocket["rect"]
        for c in range(rx // 16, (rx + rw) // 16):
            self.assertEqual(tops[c], self.r["base_walk_line_y"] - 16)
        self.assertEqual(ry + rh, self.r["base_walk_line_y"] - 16)
        self.assertEqual(pocket["y"], tops[pocket["x"] // 16] - 1)

    def test_route_markers_run_in_story_order(self) -> None:
        order = ["player_spawn", "pursuer_spawn", "trigger_chase_start", "safe_pocket_alcove",
                 "barricade", "pursuer_wedge", "breakable_shore", "hazard_fall_region",
                 "enemy_spawn_a", "exit_guild"]
        xs = [marker(self.r, m)["x"] for m in order]
        self.assertEqual(xs, sorted(xs))

    def test_authoring_script_reproduces_the_committed_sources(self) -> None:
        import author_floor01_opening as author
        built = author.build_all()
        self.assertEqual(built["contract"], ek.load_json(CONTRACT))
        for opt, path in author.LAYOUTS.items():
            self.assertEqual(built["layouts"][opt], ek.load_json(path), opt)
        for name, (stem, _kind) in author.ATLAS_FILES.items():
            committed = ek.load_json(ROOT / f"{stem}.grid.json")
            self.assertEqual(built["grids"][name], committed, name)


if __name__ == "__main__":
    unittest.main()
