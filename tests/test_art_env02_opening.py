#!/usr/bin/env python3
"""ENV-02 (design phase 1): extension contract, route proposal layouts and design rules."""
from __future__ import annotations

import copy
import hashlib
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


SOFFIT_IDS = {"soffit_end_l", "soffit_end_l_b", "soffit_end_r", "soffit_end_r_b", "soffit_corner_l",
              "soffit_corner_r", "soffit_joint_l", "soffit_joint_r"}
MARKER_IDS = [
    "player_spawn", "checkpoint_chase", "trigger_chase_start", "pursuer_spawn", "pursuer_path",
    "clearance_gauge_s3", "timber_set_a", "timber_set_b", "jump_hump_a", "jump_hump_b",
    "pursuer_charge_zone", "safe_pocket_alcove", "barricade", "pursuer_wedge", "checkpoint_post_chase",
    "breakable_shore", "falling_block", "hazard_fall_region", "enemy_spawn_a", "enemy_spawn_b",
    "enemy_spawn_c", "protected_exit_region", "exit_guild"]
KEYS_FIXTURE = ROOT / "tests/fixtures/env02_opus_keys.json"


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
        self.assertEqual({t["id"] for t in ext["tiles"]}, SOFFIT_IDS)
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
                                       "timber_barricade", "timber_barricade_full", "guild_portal"})
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


# ---------------------------------------------------------------------------
# Phase 2 (Sonnet execution): frozen keys, R1-R6, markers, CP-03 geometry
# ---------------------------------------------------------------------------

def grid_piece_hash(grid: dict, rect) -> str:
    """SHA-256 of the RGBA bytes of a rect (row-major, transparent = 00000000) from a
    pixel-grid-v1 source (cels.main of frame 0)."""
    palette = {e["key"]: bytes.fromhex(e["rgba"].lstrip("#")) for e in grid["palette"]}
    rows = grid["frames"][0]["cels"]["main"]
    x, y, w, h = rect
    buf = bytearray()
    for row in rows[y:y + h]:
        for key in row[x:x + w]:
            buf += palette.get(key, b"\x00\x00\x00\x00")
    return hashlib.sha256(bytes(buf)).hexdigest()


def contract_grids() -> tuple[dict, dict]:
    contract = ek.load_json(CONTRACT)
    return contract, {n: ek.load_json(ref["grid"]) for n, ref in contract["atlases"].items()}


class FrozenKeyTests(unittest.TestCase):
    """The Opus keys stay pixel-identical (fixture ``env02_opus_keys.json``)."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(KEYS_FIXTURE.read_text(encoding="utf-8"))
        cls.contract, cls.grids = contract_grids()
        cls.pieces = {p["id"]: p for p in cls.contract["pieces"]}

    def check_piece(self, pid: str, entry: dict) -> None:
        piece = self.pieces[pid]
        self.assertEqual(piece["atlas"], entry["atlas"], pid)
        self.assertEqual(piece["rect"], entry["rect"], pid)
        self.assertEqual(piece["anchor"], entry["anchor"], pid)
        self.assertEqual(piece["layer"], entry["layer"], pid)

    def test_frozen_keys_are_pixel_identical(self) -> None:
        self.assertEqual(len(self.fixture["frozen"]), 19)
        for pid, entry in self.fixture["frozen"].items():
            with self.subTest(piece=pid):
                self.check_piece(pid, entry)
                got = grid_piece_hash(self.grids[entry["atlas"]], entry["rect"])
                self.assertEqual(got, entry["sha256"], f"{pid}: an Opus key changed")

    def test_polished_keys_keep_their_rect_and_match_the_frozen_polish(self) -> None:
        self.assertEqual(set(self.fixture["polished"]), {"loose_block_landed"})
        for pid, entry in self.fixture["polished"].items():
            with self.subTest(piece=pid):
                self.check_piece(pid, entry)
                got = grid_piece_hash(self.grids[entry["atlas"]], entry["rect"])
                self.assertEqual(got, entry["sha256"], f"{pid}: polished art changed")
                self.assertNotEqual(entry["sha256"], entry["opus_key_sha256"], pid)

    def test_every_opus_piece_is_accounted_for(self) -> None:
        keys = set(self.fixture["frozen"]) | set(self.fixture["polished"])
        self.assertEqual(len(keys), 20)
        self.assertTrue(keys <= set(self.pieces))


@unittest.skipUnless(EXPORTED, "run bash scripts/export_assets.sh first")
class PhaseTwoTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.kit = ek.load_kit(CONTRACT)
        cls.r = ek.load_layout(LAYOUT_R)
        cls.t = ek.load_layout(LAYOUT_T)
        cls.contract, cls.grids = contract_grids()

    def both(self):
        return (("R", self.r), ("T", self.t))

    # --- every entry resolves ------------------------------------------------------------
    def test_every_layout_tile_and_piece_entry_resolves(self) -> None:
        groups = {p["state_of"]["group"] for p in self.contract["pieces"] if "state_of" in p}
        for name, layout in self.both():
            for lname, grid in layout["tile_layers"].items():
                role = self.kit.layers[lname]["role"]
                for r, row in enumerate(grid):
                    for c, tid in enumerate(row):
                        if tid is not None:
                            self.assertIn(tid, self.kit.tiles, (name, lname, r, c))
                            self.assertEqual(self.kit.tiles[tid]["role"], role, (name, lname, r, c))
            for lname, items in layout["piece_layers"].items():
                for item in items:
                    piece = self.kit.pieces[item["piece"]]
                    self.assertEqual(piece["layer"], lname, item)
                    self.assertEqual(piece["role"], self.kit.layers[lname]["role"], item)
            for m in layout["markers"]:
                if "art" in m:
                    self.assertIn(m["art"], groups, m["id"])

    def test_every_contract_entry_resolves_to_a_manifest_atlas_and_pixels(self) -> None:
        for p in self.contract["pieces"]:
            atlas = self.kit.atlases[p["atlas"]]
            self.assertTrue(ek._inside(p["rect"], atlas["image"]), p["id"])
            low, high = ek._alpha_extrema(self.kit.crop(p["atlas"], p["rect"]))
            self.assertEqual((low, high), (0, 255), p["id"])
        for t in self.contract["tiles"]:
            self.assertIn(t["atlas"], self.kit.atlases)
            self.assertTrue(t["collidable"] and t["collider"] == [0, 0, 16, 16], t["id"])

    def test_both_route_layouts_use_the_full_height_barricade(self) -> None:
        for name, layout in self.both():
            used = {i["piece"] for i in layout["piece_layers"]["prop"]}
            self.assertIn("timber_barricade_full_intact", used, name)
            self.assertNotIn("timber_barricade_intact", used, name)
            self.assertEqual(marker(layout, "barricade")["art"], "timber_barricade_full")

    # --- markers --------------------------------------------------------------------------
    def test_23_markers_are_present_in_both_layouts(self) -> None:
        for name, layout in self.both():
            ids = [m["id"] for m in layout["markers"]]
            self.assertEqual(len(ids), 23, name)
            self.assertEqual(sorted(ids), sorted(MARKER_IDS), name)

    # --- CP-03 geometry facts -------------------------------------------------------------
    def test_cp03_hit_point_is_outside_the_fall_region(self) -> None:
        for name, layout in self.both():
            hazard = marker(layout, "hazard_fall_region")["rect"]
            shore = marker(layout, "breakable_shore")
            strike_x = shore["x"]
            self.assertLess(strike_x, hazard[0], name)
            reached = [pivot for pivot in range(strike_x - 40, strike_x + 1)
                       if pivot + 16 <= strike_x < pivot + 22 or pivot + 15 <= strike_x < pivot + 22]
            self.assertTrue(reached, name)
            # Carl's whole body (pivot + half width) stays left of the region at every pivot
            # from which the jab or the kick reaches the strike point
            self.assertLess(max(reached) + CARL_HALF_WIDTH, hazard[0], name)

    def test_cp03_exit_stays_reachable_in_any_break_order(self) -> None:
        """Every shore and block state piece is non-collidable and the walkable surface is
        continuous with steps of at most one tile, so no break order can close the route."""
        states = [p for p in self.contract["pieces"] if "state_of" in p
                  and p["state_of"]["group"] in ("raking_shore", "raking_shore_load")]
        self.assertEqual(len(states), 9)       # 4 shore + seated/settling/slipped/falling/landed
        self.assertTrue(all(p["collidable"] is False for p in states))
        for name, layout in self.both():
            cols, rows = layout["grid_size"]
            tops = ek.collider_tops(self.kit, layout["tile_layers"]["collision"], cols, rows)
            spawn = marker(layout, "player_spawn")["x"] // 16
            exit_col = marker(layout, "exit_guild")["x"] // 16
            seen, frontier = {spawn}, [spawn]
            while frontier:
                c = frontier.pop()
                for n in (c - 1, c + 1):
                    if 0 <= n < cols and n not in seen and isinstance(tops[n], int) \
                            and abs(tops[n] - tops[c]) <= 16:
                        seen.add(n)
                        frontier.append(n)
            self.assertIn(exit_col, seen, name)
            shore, hazard = marker(layout, "breakable_shore"), marker(layout, "hazard_fall_region")
            for m in layout["markers"]:           # the only blocker lies before the shore
                if m["kind"] == "scripted_blocker":
                    self.assertLess(m["rect"][0] + m["rect"][2], shore["rect"][0], m["id"])
            self.assertLess(hazard["rect"][0] + hazard["rect"][2], marker(layout, "exit_guild")["rect"][0])

    def test_alcove_clear_height_and_chase_ceiling(self) -> None:
        for name, layout in self.both():
            pocket = marker(layout, "safe_pocket_alcove")
            self.assertTrue(64 <= pocket["clear_height_px"] <= 116, name)
            opt = layout["design_parameters"]["pursuer_clearance"]
            self.assertGreaterEqual(opt["options"][opt["selected"]]["clearance_px"], 164, name)
        selected = self.r["design_parameters"]["pursuer_clearance"]
        self.assertEqual(selected["selected"], "R")
        self.assertGreaterEqual(selected["options"]["R"]["clearance_px"], 200)

    # --- R1..R6 ---------------------------------------------------------------------------
    def test_r1_each_state_group_shares_one_rect(self) -> None:
        groups: dict[str, list] = {}
        for p in self.contract["pieces"]:
            if "state_of" in p:
                groups.setdefault(p["state_of"]["group"], []).append(p)
        for g, pieces in groups.items():
            if g == "raking_shore_load":       # falling is a debris sprite, landed sits on the deck
                pieces = [p for p in pieces if p["state_of"]["state"] in ("seated", "settling", "slipped")]
            self.assertEqual(len({tuple(p["rect"][2:]) for p in pieces}), 1, g)
            self.assertEqual(len({tuple(p["anchor"]) for p in pieces}), 1, g)
        self.assertEqual({p["state_of"]["state"] for p in groups["guild_portal"]}, {"closed", "ajar", "open"})

    def test_r2_no_new_palette_colours(self) -> None:
        base = {e["name"]: e["rgba"] for e in ek.load_json(BASE)["palette"]}
        for name, grid in self.grids.items():
            self.assertLessEqual(len(grid["palette"]), 63, name)
            for e in grid["palette"]:
                self.assertEqual(base.get(e["name"]), e["rgba"], (name, e["name"]))
            image = self.kit.atlases[name]["image"]
            allowed = {tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5)) for e in grid["palette"]}
            for _n, rgba in image.getcolors(1 << 16):
                if rgba[3] == 255:
                    self.assertIn(rgba[:3], allowed, name)

    def test_r3_fall_region_lies_below_the_block_and_above_the_walking_line(self) -> None:
        for name, layout in self.both():
            hazard = marker(layout, "hazard_fall_region")["rect"]
            block = marker(layout, "falling_block")["rect"]
            walk = layout["base_walk_line_y"]
            self.assertGreaterEqual(hazard[1], block[1] + block[3], name)
            self.assertEqual(hazard[1] + hazard[3], walk, name)
            self.assertTrue(hazard[0] <= block[0] and block[0] + block[2] <= hazard[0] + hazard[2], name)

    def test_r4_strike_band_is_reachable_by_the_jab_and_the_kick(self) -> None:
        for name, layout in self.both():
            shore = marker(layout, "breakable_shore")
            y0, y1 = shore["strike_band_y"]
            feet = layout["base_walk_line_y"] - 1
            # src/combat.rs boxes: jab [16, 22) x [31, 37), push kick [15, 22) x [18, 28) above the feet
            for label, (xa, xb), (ya, yb) in (("jab", (16, 22), (31, 37)), ("kick", (15, 22), (18, 28))):
                self.assertTrue(feet - yb < y1 and y0 < feet - ya, (name, label))
                self.assertTrue(any(p + xa <= shore["x"] < p + xb for p in range(shore["x"] - 30, shore["x"])),
                                (name, label))
            sx, sy, sw, sh = shore["rect"]
            self.assertTrue(sx <= shore["x"] < sx + sw and sy <= y0 and y1 <= sy + sh)

    def test_r5_full_height_barricade_art_covers_the_blocker_rect(self) -> None:
        for name, layout in self.both():
            blocker = marker(layout, "barricade")["rect"]
            item = next(i for i in layout["piece_layers"]["prop"] if i["piece"] == "timber_barricade_full_intact")
            piece = self.kit.pieces[item["piece"]]
            self.assertEqual(piece["rect"][2:], [96, 280])
            self.assertEqual(piece["anchor"], [48, 280])
            self.assertEqual(item["y"], 32)                       # top at the cornice underside
            # bottom on the deck's front lip: the art crosses the whole walking lane
            self.assertEqual(item["y"] + 280, layout["base_walk_line_y"] + 4)
            self.assertTrue(item["x"] <= blocker[0] and blocker[0] + blocker[2] <= item["x"] + 96)
            self.assertLessEqual(item["y"], blocker[1])
            art = self.kit.crop(piece["atlas"], piece["rect"]).getchannel("A")
            bx0, bx1 = blocker[0] - item["x"], blocker[0] + blocker[2] - item["x"]
            opaque = 0
            for y in range(280):
                row = [art.getpixel((x, y)) for x in range(bx0, bx1)]
                self.assertTrue(any(v == 255 for v in row), f"{name}: row {y} of the barricade is empty")
                opaque += sum(1 for v in row if v == 255)
            self.assertGreater(opaque / (280 * (bx1 - bx0)), 0.85)   # no invisible band
            # the lane band (deck back line to front lip) is covered: it blocks the passage
            lane = [art.getpixel((x, y)) for y in range(256, 280) for x in range(bx0, bx1)]
            self.assertGreater(sum(1 for v in lane if v == 255) / len(lane), 0.8)
            self.assertEqual(self.kit.pieces["timber_barricade_full_smashed"]["rect"][2:], [96, 280])

    def test_r6_soffit_tiles_appear_only_at_the_step_columns(self) -> None:
        lo, hi = 61, 104
        for name, layout in self.both():
            opt = layout["design_parameters"]["pursuer_clearance"]
            low = opt["options"][opt["selected"]]["rows"]
            grid = layout["tile_layers"]["collision"]
            found = set()
            for r, row in enumerate(grid):
                for c, tid in enumerate(row):
                    if not (tid and tid.startswith("soffit_")):
                        continue
                    found.add(tid)
                    side = "l" if "_l" in tid else "r"
                    if tid.startswith(("soffit_end_", "soffit_corner_")):
                        self.assertEqual(c, lo if side == "l" else hi, (name, tid, r, c))
                    else:                              # junction: high cornice next to the face
                        self.assertEqual((r, c), (1, lo - 1 if side == "l" else hi + 1), (name, tid))
                    self.assertLessEqual(r, low - 1, (name, tid, r))
            self.assertEqual(found, SOFFIT_IDS, name)
            for r in range(1, low):                    # the borrowed wall-end tiles are gone
                for c in (lo, hi):
                    self.assertTrue(grid[r][c].startswith("soffit_"), (name, r, c))
            for r in range(1, low - 1):
                self.assertEqual(grid[r][lo], "soffit_end_l" if r % 2 else "soffit_end_l_b")
                self.assertEqual(grid[r][hi], "soffit_end_r" if r % 2 else "soffit_end_r_b")

    def test_new_animations_are_declared_with_manifest_tags(self) -> None:
        expect = {"shore_dust_trickle": (3, [8, 24]), "block_impact_dust": (4, [112, 40]),
                  "timber_splinter_burst": (4, [48, 48])}
        for pid, (n, size) in expect.items():
            piece = self.kit.pieces[pid]
            self.assertEqual(len(piece["animation"]["frames"]), n, pid)
            self.assertEqual(piece["rect"][2:], size, pid)
            manifest = self.kit.atlases[piece["atlas"]]["manifest"]
            self.assertIn(piece["animation"]["tag"]["name"], manifest["animation_tags"], pid)
        ajar, closed = self.kit.pieces["guild_portal_ajar"], self.kit.pieces["guild_portal_closed"]
        self.assertEqual(ajar["rect"][2:], closed["rect"][2:])


if __name__ == "__main__":
    unittest.main()
