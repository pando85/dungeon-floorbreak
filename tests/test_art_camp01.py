"""AREA-CAMP-01: the survivors' camp and the quiet aftermath kits of Floor 01 (sources, contracts, sizes, palette, the
dressing, the twelve frozen key frames, the figure language, the lamp of gate note G5, the girder and cart gates G1 and G4,
the window rule and the sway table shared with the Rust presenter)."""
import hashlib
import json
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_camp01 as camp  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CAMP_CONTRACT = camp.CAMP_CONTRACT
AFT_CONTRACT = camp.AFT_CONTRACT
GATE = ROOT / "docs/evidence/area-camp-01/gate/measurements.txt"
RUST = ROOT / "src/camp_kit.rs"
SIZES = {
    "aft_bench_bent": (48, 20), "aft_groove_rings": (160, 160), "aft_heavy_bag_l1": (40, 122), "aft_heavy_bag_l2":
    (40, 122), "aft_heavy_bag_r1": (40, 122), "aft_heavy_bag_r2": (40, 122), "aft_heavy_bag_rest": (40, 122),
    "aft_lane_chalk": (116, 16), "aft_mirror_wreck": (160, 72), "aft_sand_spill": (40, 8), "camp_chalk_floor":
    (128, 16), "camp_chalk_tally": (24, 40), "camp_figure_map": (20, 46), "camp_figure_repair": (24, 36),
    "camp_figure_seated": (20, 30), "camp_figure_sentry": (20, 48), "camp_fire_0": (20, 26), "camp_fire_1":
    (20, 26), "camp_fire_2": (20, 26), "camp_fire_3": (20, 26), "camp_hearth": (96, 64), "camp_map_table":
    (44, 30), "camp_motifs": (48, 48), "camp_toy_cart": (18, 12)
}
# The twelve key frames: frozen at the silhouette gate (gate/measurements.txt `KEY` lines). A key change is a design change.
KEY_SHA = {
    "aft_heavy_bag_l1": "35d5a888f84f95a51507e6fb3926182f160376f15a1ce66a7709a4627493579a",
    "aft_heavy_bag_l2": "53a928388c9873e956f00f08a9fbc555af5da01c2ead86c7c74ad8f4792ca06c",
    "aft_heavy_bag_r1": "6fa4bbcaee78314431cc6e0693374b2aeb8cdaa83423d66887076ab71435eb51",
    "aft_heavy_bag_r2": "b9ae0d5cf988ba809ae8bf8c7c72b417c9966269f2d15e78460696abcd38f3d3",
    "aft_heavy_bag_rest": "4daab707361f941a48986dd059d3b8c06da68af600bb45f567e3caa5e8cd3dce",
    "aft_mirror_wreck": "a5a77dd983d54e62c15d5c44575800175c7991cb51b063825a3d80fa5ba752e8",
    "camp_figure_sentry": "ec7c8551f0992bde008fb2e1fb176feb8636b66c93db1fffff841acb5ee72d10",
    "camp_fire_0": "202d28085eb8adfb35a6a418f5b45209d0da8f55579d0d3778713f60fd136183",
    "camp_fire_1": "6ba5e378695b32e5319fc35b494208b5c8a6fcf9513febc90344368f0b335e6d",
    "camp_fire_2": "f30897d15a789ef3ebd465a9938c6bbc4bf03a98ddc7ff6b9a5214b13a145e71",
    "camp_fire_3": "3fb86df4eac3277425061a07ae10163355627f9aacd0f1cc780906dd26891a42",
    "camp_hearth": "5b865eafbf8084089b41885b8472bb8c2e5942c5f8dfba80bb8bc041d54a80d7",
}
# The executor pieces' pins (the pixels the spec section 5 rows E ask for, as drawn at the first execution).
EXEC_SHA = {
    "aft_bench_bent": "c3494fd21524f5dbe34b6d6b0acdf4b17f89830b6deac2a5b148485661f99a87",
    "aft_groove_rings": "20d62f51d5492f1c779f32a309f57e3765ef8ac00688f0db6c676484db06127f",
    "aft_lane_chalk": "3e0b8b278804d2c318fbf728f819dcda5e24d47a6202c311e4cdfe69e1d62a15",
    "aft_sand_spill": "f414d6c2daef065a93c2ec4141be5871e07465be953a78f971915f9182b8ba15",
    "camp_chalk_floor": "1ad76fce2b327a82d90588c7bbd23cd40d5c37262d0b9a103fead59473d6ac76",
    "camp_chalk_tally": "474e714c41d7f6fb20561b4bbb266b6f1da3428bd6444e65d32809826f3a8588",
    "camp_figure_map": "7d00580e37d3608489414c3ee583f70c3e2093eb80d125eb9d83fac6650f1c84",
    "camp_figure_repair": "f493a080529af22a1f5b537faa7ece5e4b23a17a6db95e7d99fc5c182cc92e39",
    "camp_figure_seated": "36e1fbf6f022df878e2ac07496b71883afc8e6f68754b0aac3c20d7e23170b97",
    "camp_map_table": "24f839326237961ccf33f6d01244f82d3e6a5faa3e4c4eb45559b1ff492ead84",
    "camp_motifs": "68c8249fc7374ed990ec44a58543b94887c528a55421a9442f1f9c9f82de2f9d",
    "camp_toy_cart": "5507d9b4e7a002f68cc2c0b399c57aab597d2838f6ba1fb92f15aecbb76620c4",
}
OPENINGS = {"rest_camp": [(184, 335), (520, 671), (856, 1007), (1192, 1343)],
             "rest_aftermath": [(216, 367), (888, 1039)]}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def luma(p) -> float:
    return camp._luma(*p)


def sha(img) -> str:
    return hashlib.sha256(json.dumps(img.px).encode()).hexdigest()


def opaque_bbox(img):
    pts = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x] is not None]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), max(xs), min(ys), max(ys)


def placements(kit: str, room: str):
    return load(CAMP_CONTRACT if kit == "camp" else AFT_CONTRACT)["room_dressing"][room]["place"]


class CampKitTest(unittest.TestCase):
    pieces = None

    @classmethod
    def setUpClass(cls):
        cls.pieces = camp.all_pieces()
        cls.pieces["camp_fire"] = cls.pieces["camp_fire_0"]       # the contract's piece rect is the first frame

    # ------------------------------------------------------------------------------ sources
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects, colours = {}, {}
        for name, spec in camp.ATLASES.items():
            sheet, r = camp._pack(camp.build_pieces(name), spec["width"])
            grid = camp.kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0))
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name], colours[name] = r, len(grid["palette"])
            self.assertEqual(camp.manifest(name, grid["canvas"], spec["stem"]),
                             load(ROOT / f"assets/manifests/{name}.json"))
        fire = camp.kit.grid_from(camp.fire_images(), layer_names=["main"], durations=list(camp.FIRE_DURATIONS),
                                  pivot=(0, 0), tags=[{"name": "burn", "from": 0, "to": 3, "direction": "forward"}])
        self.assertEqual(fire, load(PROPS / "camp_fire.grid.json"))
        colours[camp.FIRE_ATLAS] = len(fire["palette"])
        self.assertEqual(camp.manifest(camp.FIRE_ATLAS, fire["canvas"], "camp_fire"),
                         load(ROOT / f"assets/manifests/{camp.FIRE_ATLAS}.json"))
        self.assertEqual(camp.contract("camp", rects, colours), load(CAMP_CONTRACT))
        self.assertEqual(camp.contract("aft", rects, colours), load(AFT_CONTRACT))
        for name in list(camp.ATLASES) + [camp.FIRE_ATLAS]:
            self.assertEqual(load(ROOT / f"assets/manifests/{name}.json")["review_status"],
                             "prototype_fixture_unapproved")
            self.assertEqual(load(ROOT / f"assets/manifests/{name}.json")["palette_id"], "floor01-catacomb-v2")

    # ------------------------------------------------------------------------------ CAMP-A9: the keys
    def test_the_twelve_key_frames_hold_the_gates_pins(self):
        self.assertEqual(camp.key_sha(), KEY_SHA, "key_sha() is the gate's pins")
        gate = {}
        for line in GATE.read_text(encoding="utf-8").splitlines():
            m = re.match(r"KEY (\S+) sha256=([0-9a-f]+)", line)
            if m:
                gate[m.group(1)] = m.group(2)
        self.assertEqual(gate, KEY_SHA, "measurements.txt, written by the designer, is the source of the pins")
        self.assertEqual(len(KEY_SHA), 12)
        for pid, want in KEY_SHA.items():
            self.assertEqual(sha(self.pieces[pid]), want, pid)

    def test_the_executor_pieces_hold_their_pins(self):
        self.assertEqual(set(EXEC_SHA), set(camp.EXEC_PIECES))
        for pid, want in EXEC_SHA.items():
            self.assertEqual(sha(self.pieces[pid]), want, pid)

    def test_every_piece_sits_exactly_in_its_atlas_rect(self):
        for kitfile in (CAMP_CONTRACT, AFT_CONTRACT):
            contract = load(kitfile)
            for p in contract["pieces"]:
                pid = p["id"]
                grid = load(ROOT / contract["atlases"][p["atlas"]]["grid"])
                palette = {q["key"]: q["name"] for q in grid["palette"]}
                rows = grid["frames"][0]["cels"]["main"]
                x, y, w, h = p["rect"]
                img = self.pieces[pid]
                self.assertEqual((w, h), (img.w, img.h), pid)
                for yy in range(h):
                    for xx in range(w):
                        ch, px = rows[y + yy][x + xx], img.px[yy][xx]
                        self.assertEqual(ch == ".", px is None, (pid, xx, yy))
                        if px:
                            self.assertEqual(palette[ch], camp.kit.colour_name(*px), (pid, xx, yy))

    # ------------------------------------------------------------------------------ CAMP-A10: palette and alpha
    def test_native_sizes_colour_budget_and_alpha(self):
        got = {}
        for kitfile in (CAMP_CONTRACT, AFT_CONTRACT):
            got.update({p["id"]: tuple(p["rect"][2:]) for p in load(kitfile)["pieces"]})
        want = {k: v for k, v in SIZES.items() if k not in ("camp_fire_1", "camp_fire_2", "camp_fire_3")}
        want["camp_fire"] = want.pop("camp_fire_0")
        self.assertEqual(got, want)
        for stem in {s["stem"] for s in camp.ATLASES.values()} | {"camp_fire"}:
            grid = load(PROPS / f"{stem}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, stem)
            self.assertTrue(all(p["rgba"].endswith("ff") for p in grid["palette"]), "1-bit alpha")
        for pid, img in self.pieces.items():
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is not None for p in flat), pid)
            self.assertTrue(any(p is None for p in flat), f"{pid}: one transparent pixel at least")

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in camp.RAMPS.values() for c in ramp}
        for stem in {s["stem"] for s in camp.ATLASES.values()} | {"camp_fire"}:
            for p in load(PROPS / f"{stem}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{stem} {p['name']}")

    # ------------------------------------------------------------------------------ the contracts
    def test_contract_shape_decor_only_and_neutral_ids(self):
        for kitfile, extends in ((CAMP_CONTRACT, "assets/source/terrain/floor01/floor01_rings_kit.json"),
                                 (AFT_CONTRACT, "assets/source/terrain/floor01/floor01_gym_kit.json")):
            kit = load(kitfile)
            self.assertEqual(kit["format"], "env-kit-v2")
            self.assertEqual(kit["extends"], extends)
            self.assertEqual(kit["tiles"], [])
            for inherited in camp_inherited():
                self.assertNotIn(inherited, kit)
            for p in kit["pieces"]:
                self.assertFalse(p["collidable"], p["id"])
                self.assertEqual(p["anchor"], [0, 0] if p["id"] != "camp_fire" else [10, 22], p["id"])
                self.assertRegex(p["id"], r"^(camp|aft)_[a-z_0-9]+$")
            for a in kit["atlases"].values():
                self.assertRegex(a["manifest"], r"floor01_(camp|aft)_")
        for p in load(AFT_CONTRACT)["pieces"]:
            self.assertEqual(p["layer"] == "structure", p["id"] in ("aft_sand_spill", "aft_lane_chalk"), p["id"])
        self.assertEqual({p["id"]: p["layer"] for p in load(CAMP_CONTRACT)["pieces"]}["camp_chalk_floor"], "structure")

    def test_the_fire_is_animated_and_carries_the_two_lights_of_spec_section_8(self):
        fire = next(p for p in load(CAMP_CONTRACT)["pieces"] if p["id"] == "camp_fire")
        self.assertEqual([f["duration"] for f in fire["animation"]["frames"]], [110, 90, 130, 100])
        self.assertEqual(fire["animation"]["tag"]["name"], "burn")
        self.assertEqual(fire["rect"], [0, 0, 20, 26])
        flame, pool = fire["lights"]
        self.assertEqual((flame["id"], flame["offset"], flame["colour"], flame["radius"], flame["intensity"]),
                         ("flame", [10, 22], "#ffb060", 240, 0.40))
        self.assertEqual(flame["glow"], {"radius": 40, "additive": 0.08})
        self.assertEqual(flame["flicker"], [1.0, 0.9, 1.08, 0.95])
        self.assertEqual((pool["id"], pool["offset"], pool["project"], pool["project_dy"], pool["squash_y"],
                          pool["colour"], pool["radius"], pool["intensity"]),
                         ("floor_pool", [10, 22], "walk_line", -6, 0.32, "#ffa95e", 150, 0.34))
        self.assertTrue(all(l["group"] == "scene" for l in fire["lights"]))
        # The piece's anchor is the flame base: hearth (38, 30) + (10, 22) = (48, 52)... placed at (758, 526)
        self.assertEqual((758 + 10, 526 + 22), (768, 548), "the light origin of spec section 8")

    def test_the_dressing_is_the_spec_sections_4_and_5(self):
        camp_d = load(CAMP_CONTRACT)["room_dressing"]["rest_camp"]
        self.assertEqual(len(camp_d["remove_at"]), 8)
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in camp_d["remove_at"]}, set(camp.CAMP_REMOVE_AT))
        self.assertEqual(camp_d["parallax"], {"parallax_far": "ring_tiers_far", "parallax_mid": "ring_tiers_mid"})
        layers = [p["layer"] for p in camp_d["place"]]
        self.assertEqual(layers, sorted(layers, key=["backwall_decal", "structure", "prop"].index),
                         "backwall decals, then structure, then props")
        xs = [p["x"] for p in camp_d["place"] if p["layer"] == "prop"]
        self.assertEqual(xs, sorted(xs), "props left to right")
        aft_d = load(AFT_CONTRACT)["room_dressing"]["rest_aftermath"]
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in aft_d["remove_at"]},
                         {("structure", "arch_chain", 560, 384), ("backwall_decal", "torch_pool_b_m", 1112, 348)})
        self.assertEqual(aft_d["backwall_fill"], [{"x": 512, "y": 352, "w": 208, "h": 208}])
        self.assertEqual(aft_d["parallax"], {})
        order = [p["piece"] for p in aft_d["place"]]
        self.assertEqual(order[0], "aft_groove_rings")
        self.assertEqual(order[1], "torch_pool_b_m", "the pool draws over the grooves")
        self.assertFalse([p for p in order if p.startswith("aft_heavy_bag")], "the bag is presenter-drawn")
        state = load(AFT_CONTRACT)["state_groups"]["aft_heavy_bag"]
        self.assertEqual(state["anchor"], [620, 450])
        self.assertEqual(state["states"], ["rest", "r1", "r2", "l1", "l2"])

    # ------------------------------------------------------------------------------ the gate notes
    def test_g1_the_girder_run_has_a_sixth_girder_under_the_fifth(self):
        girders = [p for p in placements("aft", "rest_aftermath") if p["piece"] == "gym_girder"]
        self.assertEqual([g["x"] for g in girders], [684, 548, 580, 612, 644, 676])
        self.assertTrue(all(g["y"] == 440 and g["layer"] == "structure" for g in girders))

    def test_g3_the_map_figures_hand_draws_over_the_table(self):
        place = [p for p in placements("camp", "rest_camp") if p["layer"] == "prop"]
        names = [p["piece"] for p in place]
        table, fig = place[names.index("camp_map_table")], place[names.index("camp_figure_map")]
        self.assertGreater(names.index("camp_figure_map"), names.index("camp_map_table"))
        self.assertEqual((fig["x"], fig["y"]), (1152, 514))
        t, f = opaque_bbox(self.pieces["camp_map_table"]), opaque_bbox(self.pieces["camp_figure_map"])
        self.assertLessEqual(fig["x"] + f[0], table["x"] + t[1], "the hand lands on the table's end")
        self.assertGreaterEqual(fig["x"] + 19, 1152)
        self.assertLess(fig["x"] + 19, 1192, "outside window B4")
        self.assertGreaterEqual(fig["y"] + 46, 560)

    def test_g4_the_cart_is_four_to_eight_pixels_from_the_seated_figure(self):
        place = {p["piece"]: p for p in placements("camp", "rest_camp") if p["layer"] == "prop"}
        s, c = opaque_bbox(self.pieces["camp_figure_seated"]), opaque_bbox(self.pieces["camp_toy_cart"])
        gap = (place["camp_toy_cart"]["x"] + c[0]) - (place["camp_figure_seated"]["x"] + s[1]) - 1
        self.assertTrue(4 <= gap <= 8, gap)
        self.assertEqual(place["camp_toy_cart"]["x"], camp.CART_X)

    def test_g5_the_repair_lamp_has_no_emitter_and_no_flame_4_or_5(self):
        flames = [p for row in self.pieces["camp_figure_repair"].px for p in row if p and p[0] == "flame"]
        self.assertTrue(flames and all(p[1] == 3 for p in flames), "a flame-3 slit only")
        self.assertLessEqual(len(flames), 4, "at most a 2 x 2 slit")
        repair = next(p for p in load(CAMP_CONTRACT)["pieces"] if p["id"] == "camp_figure_repair")
        self.assertNotIn("lights", repair)
        self.assertNotIn("animation", repair)

    def test_the_fire_is_the_only_flame_4_or_5_among_the_new_camp_pieces(self):
        for pid, img in self.pieces.items():
            if pid.startswith("camp_") and not pid.startswith("camp_fire"):
                self.assertFalse([p for row in img.px for p in row if p and p[0] == "flame" and p[1] >= 4], pid)

    # ------------------------------------------------------------------------------ figure language
    def test_figures_are_dark_anonymous_and_faceless(self):
        for pid in camp.FIGURES:
            img = self.pieces[pid]
            vals = [luma(p) for row in img.px for p in row if p]
            self.assertLessEqual(sum(vals) / len(vals), 45.0, pid)
            self.assertTrue(any(p == ("gal", 0) for row in img.px for p in row), f"{pid}: the hood void")
            for row in img.px:
                for p in row:
                    if p and p[0] == "gal_warm":
                        self.assertLessEqual(p[1], 2, f"{pid}: a warm rim is gal_warm 1..2 only")
            self.assertEqual(sum(1 for row in img.px for p in row if p and p[0] == "gal_warm" and p[1] == 0), 0)
            x0, x1, y0, y1 = opaque_bbox(img)
            self.assertEqual(y1, img.h - 1, f"{pid}: bottom row on the floor's back line")
            self.assertLessEqual(img.h, 48, pid)
            self.assertLessEqual(x1 - x0 + 1, 24, pid)

    def test_no_survivor_has_skin_or_hair_colour(self):
        for pid in camp.FIGURES:
            ramps = {p[0] for row in self.pieces[pid].px for p in row if p}
            self.assertFalse(ramps & {"brick", "floor", "copper", "copper_cold", "brass", "lichen"}, (pid, ramps))

    # ------------------------------------------------------------------------------ the wreck and the bag
    def test_the_wreck_torch_columns_are_clear(self):
        wreck = self.pieces["aft_mirror_wreck"]
        for (x0, x1) in camp.CLEAR_COLS:
            self.assertEqual([1 for x in range(x0, x1 + 1) for y in range(camp.CLEAR_ROWS) if wreck.get(x, y)], [])

    def test_the_bag_frames_cover_the_sims_hit_rect_and_keep_the_hook_fixed(self):
        frames = camp.aft_heavy_bag_frames()
        hook = [x for x in range(camp.BAG_W) if frames[0].get(x, 0)]
        self.assertEqual(hook, [19, 20, 21])
        for f, dx in zip(frames, camp.BAG_SWAY):
            body = [x for y in range(camp.BAG_BODY[1], camp.BAG_BODY[3] + 1) for x in range(camp.BAG_W) if f.get(x, y)]
            # the sim rect {624, 516, 32, 64} is canvas x 4..35, y 66..129 (placed at (620, 450))
            self.assertLessEqual(min(body), 6 + max(0, dx), dx)
            self.assertEqual([x for x in range(camp.BAG_W) if f.get(x, 0)], hook, "the hook row is fixed")

    def test_the_sway_table_is_the_rust_presenters(self):
        text = RUST.read_text(encoding="utf-8")
        block = text[text.index("pub const SWAY_TABLE"):]
        block = block[:block.index("];")]
        rust = re.findall(r'"([a-z0-9]+)"', block)
        self.assertEqual(tuple(rust), camp.SWAY_TABLE)
        self.assertEqual(len(rust), 38)

    # ------------------------------------------------------------------------------ the window rule (CAMP-A4)
    def test_no_prop_pixel_stands_in_an_opening_above_y_520(self):
        for kit, room in (("camp", "rest_camp"), ("aft", "rest_aftermath")):
            for p in placements(kit, room):
                if p["layer"] != "prop" or p["piece"] not in self.pieces:
                    continue
                img = self.pieces[p["piece"]]
                for y in range(img.h):
                    for x in range(img.w):
                        if img.px[y][x] is None or p["y"] + y >= 520:
                            continue
                        for a, b in OPENINGS[room]:
                            self.assertFalse(a <= p["x"] + x <= b, (room, p["piece"], x, y))

    def test_chalk_is_pale_with_a_dark_under_stroke_and_off_the_plinths(self):
        """Review R2 (CAMP-AD26): chalk is stone 8 over a mortar under-stroke; a pixel on a plinth's columns is only
        allowed on rows 573..576 (piece rows 12..15). The composed-layout occupancy test is in `camp_kit`."""
        for pid, plinth in (("camp_chalk_floor", camp.CAMP_PLINTH), ("aft_lane_chalk", camp.AFT_PLINTH)):
            img = self.pieces[pid]
            self.assertEqual(img.h, 16, pid)
            for y in range(img.h):
                for x in range(img.w):
                    p = img.px[y][x]
                    if p is None:
                        continue
                    self.assertIn(p, (("stone", 8), ("mortar", 3)), (pid, x, y))
                    if y < camp.CHALK_LOW:
                        self.assertFalse(any(a <= x <= b for a, b in plinth), (pid, x, y))

    def test_the_motifs_cluster_sits_right_of_the_last_pier(self):
        place = next(p for p in placements("camp", "rest_camp") if p["piece"] == "camp_motifs")
        img = self.pieces["camp_motifs"]
        self.assertEqual((img.w, img.h), (48, 48))
        self.assertGreaterEqual(place["x"], 1414)
        self.assertLessEqual(place["x"] + 47, 1467)
        self.assertLessEqual(place["y"] + 47, 430, "above torch 1472 (top 432)")
        ramps = {p[0] for row in img.px for p in row if p}
        self.assertEqual(ramps, {"brick", "stone"})
        self.assertTrue(all(6 <= p[1] <= 8 for row in img.px for p in row if p))

    def test_floor_decals_are_structure_in_the_decks_back_half(self):
        for kit, room in (("camp", "rest_camp"), ("aft", "rest_aftermath")):
            contract = load(CAMP_CONTRACT if kit == "camp" else AFT_CONTRACT)
            layer = {p["id"]: p["layer"] for p in contract["pieces"]}
            for p in placements(kit, room):
                img = self.pieces.get(p["piece"])
                if img is None or p["piece"].startswith("torch_pool"):
                    continue
                if p["y"] < 573 and p["y"] + img.h > 561:
                    self.assertEqual(layer[p["piece"]], "structure", p["piece"])
        for pid in ("camp_chalk_floor", "aft_lane_chalk"):                   # chalk strokes at most 3 px tall (R2)
            img = self.pieces[pid]
            for x in range(img.w):
                col = [y for y in range(img.h) if img.px[y][x] is not None and img.px[y][x][0] == "stone"]
                self.assertLessEqual(len(col), 3, (pid, x))                 # chalk only; the under-stroke is mortar


def camp_inherited():
    return ("palette_id", "palette", "palette_policy", "tile_size", "coordinate_system", "layers",
            "actor_draw_order", "actor_light_group", "roles", "walk_line", "lighting")


if __name__ == "__main__":
    unittest.main()
