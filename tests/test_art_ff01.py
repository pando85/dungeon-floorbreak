"""AREA-FASTFOOD-01: the fast-food refuge kit of Floor 01 room 1 (sources, contract, sizes, palette, dressing, the eight
frozen keys, FF-A5 the door, FF-A6 the stand lines and the readability rules of spec section 10 with the gate notes
G2 and G3, the skins and the bell of G8)."""
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_ff01 as ff  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_fastfood_kit.json"
SIZES = {  # spec section 5 (the door and the box are the keys' canvases; the rising crops share the door's)
    "ff_staff_door_closed": (24, 228), "ff_staff_door_open": (24, 228),
    **{f"ff_staff_door_rise_{k:02d}": (24, 228) for k in range(1, 13)},
    "ff_reward_box_full_a": (32, 24), "ff_reward_box_full_b": (32, 24), "ff_reward_box_empty": (32, 24),
    "ff_reward_sparkle_a": (16, 16), "ff_reward_sparkle_b": (16, 16), "ff_reward_sparkle_c": (16, 16),
    "ff_bell_idle": (8, 6), "ff_bell_ding_a": (8, 6), "ff_bell_ding_b": (8, 6),
    "ff_pass_ledge": (96, 20), "ff_wainscot_a": (32, 32), "ff_wainscot_b": (32, 32), "ff_wall_tile_a": (32, 32),
    "ff_floor_checker_a": (32, 12), "ff_floor_checker_edge": (32, 12), "ff_floor_spill": (48, 12),
    "ff_tile_seam": (8, 192), "ff_ceiling_tile": (32, 16), "ff_soffit_stone": (32, 16), "ff_light_strip": (48, 6),
    "ff_counter_a": (128, 36), "ff_counter_canopy": (128, 40), "ff_deck_skin": (96, 16),
    "ff_slide_top_skin": (128, 16), "ff_play_posts": (224, 52), "ff_slide_chute": (64, 64), "ff_ball_pit": (96, 12),
    "ff_stray_ball": (6, 6), "ff_bin": (20, 32), "ff_window_frame": (160, 160), "ff_rock_fill": (152, 192),
    "ff_booth_a": (112, 40), "ff_service_hatch": (80, 48), "ff_under_panel": (96, 60),
}
# The eight keys: frozen at the silhouette gate (gate/measurements.txt `KEY` lines). A key change is a design change.
KEY_SHA = {
    "ff_staff_door_closed": "9a90146011e4544d0ba177411497d86b8816109a5e53f5b979ba9681c20d532a",
    "ff_staff_door_open": "284f7d9d167a68df234217ac63d0bdbcc7647e05e6aceafdf701080e2c08a5d6",
    "ff_reward_box_full_a": "034c4fea7072651c6db8fc5079ad928ddc7af4d0d75a25e71f4120cf8b912dec",
    "ff_reward_box_full_b": "c43b3f781963820397bb2383fcc92181aa5af1dfafcdc84076233d9001a701f1",
    "ff_reward_box_empty": "023f61643c72f6f29108bc2bf03e1029eef213c0603a8235cb1742d0db783f2b",
    "ff_pass_ledge": "15e47b581d9ac25cf03907ada7959fb60c1a1a5baefa36444f5e78d73bae165f",
    "ff_wainscot_a": "540d9f490a96e34b4a60d39045fb34046d56831ded9f831a57a6ce11bbcb9dfe",
    "ff_floor_checker_a": "fdb2dae81412e821ed0a01c84be53405cada140703a5e007002d66a2c9434147",
}
PRESENTER = ({f"ff_staff_door_rise_{k:02d}" for k in range(1, 13)}
             | {"ff_staff_door_closed", "ff_staff_door_open", "ff_reward_box_full_a", "ff_reward_box_full_b",
                "ff_reward_box_empty", "ff_reward_sparkle_a", "ff_reward_sparkle_b", "ff_reward_sparkle_c",
                "ff_bell_idle", "ff_bell_ding_a", "ff_bell_ding_b"})
# Presenter positions (spec section 5): the door's canvas, the box, the sparkle and the bell.
PRESENTER_AT = {"door": (1436, 352), "box": (1204, 556), "sparkle": (1212, 554), "bell": (1380, 442)}
# Gate note G2: the stand-line segments (x0, x1 inclusive, y).
STAND = (("ff_deck_skin", 464, 559, 513), ("ff_slide_top_skin", 560, 687, 497), ("ff_pass_ledge", 1296, 1391, 449))
REMOVE_AT = [  # spec section 8
    ("prop", "torch_wall", 336, 448), ("prop", "torch_wall", 848, 176), ("prop", "torch_wall", 848, 448),
    ("prop", "torch_wall", 1088, 448),
    ("backwall_decal", "torch_pool_a_m", 232, 364), ("backwall_decal", "torch_pool_a", 744, 92),
    ("backwall_decal", "torch_pool_a_m", 744, 364), ("backwall_decal", "torch_pool_a", 984, 364),
    ("backwall_decal", "grime_a", 600, 376),
]
OPAQUE = {"ff_staff_door_closed", "ff_wainscot_a", "ff_wainscot_b", "ff_wall_tile_a", "ff_floor_checker_a",
          "ff_ceiling_tile", "ff_soffit_stone", "ff_deck_skin", "ff_slide_top_skin"}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def all_pieces():
    out = {}
    for name in ff.ATLASES:
        out.update(dict(ff.build_pieces(name)))
    return out


def luma(p) -> float:
    return ff._luma(*p)


def placements():
    return load(CONTRACT)["room_dressing"]["rest_refuge"]["place"]


def sha(img) -> str:
    return hashlib.sha256(json.dumps(img.px).encode()).hexdigest()


def bright_runs(img, x0, y0, min_len=8, above=201.0):
    """Horizontal runs of ``min_len`` or more opaque pixels brighter than ``above``: (x0, x1, y, pixel list)."""
    out = []
    for y in range(img.h):
        x = 0
        while x < img.w:
            p = img.px[y][x]
            if p is not None and luma(p) > above:
                e = x
                while e + 1 < img.w and img.px[y][e + 1] is not None and luma(img.px[y][e + 1]) > above:
                    e += 1
                if e - x + 1 >= min_len:
                    out.append((x0 + x, x0 + e, y0 + y, [img.px[y][i] for i in range(x, e + 1)]))
                x = e + 1
            else:
                x += 1
    return out


def placed_instances():
    """(piece id, x, y) of every placed piece, the keys at their canonical spots, and the presenter pieces at theirs."""
    out = [(p["piece"], p["x"], p["y"]) for p in placements()]
    out += [("ff_staff_door_closed", *PRESENTER_AT["door"]), ("ff_staff_door_open", *PRESENTER_AT["door"]),
            ("ff_reward_box_full_a", *PRESENTER_AT["box"]), ("ff_reward_box_full_b", *PRESENTER_AT["box"]),
            ("ff_reward_box_empty", *PRESENTER_AT["box"]), ("ff_bell_idle", *PRESENTER_AT["bell"]),
            ("ff_bell_ding_a", *PRESENTER_AT["bell"]), ("ff_bell_ding_b", *PRESENTER_AT["bell"])]
    out += [(f"ff_reward_sparkle_{s}", *PRESENTER_AT["sparkle"]) for s in "abc"]
    out += [(f"ff_staff_door_rise_{k:02d}", *PRESENTER_AT["door"]) for k in range(1, 13)]
    return out


class FastFoodKitTest(unittest.TestCase):
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects, imgs = {}, {}
        for name, spec in ff.ATLASES.items():
            grid, r, pieces = ff.build(name)
            imgs.update(pieces)
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name] = {"rects": r, "canvas": grid["canvas"], "colours": len(grid["palette"])}
            self.assertEqual(ff.manifest(name, grid["canvas"]), load(ROOT / f"assets/manifests/{name}.json"))
            self.assertEqual(load(ROOT / f"assets/manifests/{name}.json")["review_status"],
                             "prototype_fixture_unapproved")
        self.assertEqual(ff.contract(rects, imgs), load(CONTRACT))
        for stem in ("ff_staff_door", "ff_reward_box", "ff_samples"):       # the design-stage key records stay
            self.assertTrue((PROPS / f"{stem}.grid.json").is_file(), stem)

    def test_the_eight_keys_hold_their_pins_and_sit_exactly_in_their_atlas_rects(self):
        self.assertEqual(ff.key_sha(), KEY_SHA, "key_sha() is the gate's pins")
        pieces = all_pieces()
        contract = load(CONTRACT)
        rect = {p["id"]: p["rect"] for p in contract["pieces"]}
        for pid, want in KEY_SHA.items():
            self.assertEqual(sha(pieces[pid]), want, pid)
            atlas = next(p["atlas"] for p in contract["pieces"] if p["id"] == pid)
            grid = load(PROPS / f"{ff.ATLASES[atlas]['stem']}.grid.json")
            palette = {p["key"]: p["name"] for p in grid["palette"]}
            rows = grid["frames"][0]["cels"]["main"]
            x, y, w, h = rect[pid]
            key = pieces[pid]
            for yy in range(h):
                for xx in range(w):
                    ch = rows[y + yy][x + xx]
                    px = key.px[yy][xx]
                    self.assertEqual(ch == ".", px is None, (pid, xx, yy))
                    if px:
                        self.assertEqual(palette[ch], ff.kit.colour_name(*px), (pid, xx, yy))

    def test_every_piece_sits_exactly_in_its_atlas_rect(self):
        contract = load(CONTRACT)
        rect = {p["id"]: (p["atlas"], p["rect"]) for p in contract["pieces"]}
        for pid, img in all_pieces().items():
            atlas, (x, y, w, h) = rect[pid]
            grid = load(PROPS / f"{ff.ATLASES[atlas]['stem']}.grid.json")
            palette = {p["key"]: p["name"] for p in grid["palette"]}
            rows = grid["frames"][0]["cels"]["main"]
            self.assertEqual((w, h), (img.w, img.h), pid)
            for yy in range(h):
                for xx in range(w):
                    ch, px = rows[y + yy][x + xx], img.px[yy][xx]
                    self.assertEqual(ch == ".", px is None, (pid, xx, yy))
                    if px:
                        self.assertEqual(palette[ch], ff.kit.colour_name(*px), (pid, xx, yy))

    def test_native_sizes_colour_budget_and_alpha(self):
        kit = load(CONTRACT)
        self.assertEqual({p["id"]: tuple(p["rect"][2:]) for p in kit["pieces"]}, SIZES)
        for spec in ff.ATLASES.values():
            grid = load(PROPS / f"{spec['stem']}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, spec["stem"])
            self.assertTrue(all(p["rgba"].endswith("ff") for p in grid["palette"]), "1-bit alpha")
        declared = {p["id"] for p in kit["pieces"] if p.get("opaque")}
        for pid, img in all_pieces().items():
            self.assertEqual((img.w, img.h), SIZES[pid], pid)
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is not None for p in flat), pid)
            if pid in declared:
                self.assertTrue(all(p is not None for p in flat), f"{pid}: declared opaque")
            else:
                self.assertTrue(any(p is None for p in flat), f"{pid}: one transparent pixel at least")
        # FF-AD16: only tile pieces are opaque (the wainscot, checker and closed-door keys are opaque by design)
        self.assertEqual(declared, OPAQUE)

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in ff.RAMPS.values() for c in ramp}
        for spec in ff.ATLASES.values():
            for p in load(PROPS / f"{spec['stem']}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{spec['stem']} {p['name']}")

    # ------------------------------------------------------------------------------ the contract
    def test_contract_shape_decor_only_and_neutral_ids(self):
        kit = load(CONTRACT)
        self.assertEqual(kit["format"], "env-kit-v2")
        self.assertEqual(kit["extends"], "assets/source/terrain/floor01/floor01_opening_kit.json")
        self.assertEqual(kit["tiles"], [])
        self.assertNotIn("palette", kit)
        self.assertIn("not owner-approved", kit["status"])
        for p in kit["pieces"]:
            self.assertIs(p["collidable"], False, p["id"])
            self.assertTrue(p["id"].startswith("ff_"), p["id"])
            self.assertNotIn("refuge_", p["id"])
        for a in kit["atlases"]:
            self.assertTrue(a.startswith("floor01_ff_"), a)
        layers = {p["id"]: p["layer"] for p in kit["pieces"]}
        wall = {"ff_wainscot_a", "ff_wainscot_b", "ff_wall_tile_a", "ff_floor_checker_a", "ff_floor_checker_edge",
                "ff_floor_spill", "ff_counter_canopy", "ff_slide_chute", "ff_service_hatch", "ff_under_panel",
                "ff_rock_fill"}
        structure = {"ff_pass_ledge", "ff_tile_seam", "ff_ceiling_tile", "ff_soffit_stone", "ff_deck_skin",
                     "ff_slide_top_skin", "ff_play_posts", "ff_window_frame"}
        props = {"ff_light_strip", "ff_counter_a", "ff_ball_pit", "ff_stray_ball", "ff_bin", "ff_booth_a"}
        for pid in wall:
            self.assertEqual(layers[pid], "backwall_decal", pid)
        for pid in structure:
            self.assertEqual(layers[pid], "structure", pid)
        for pid in props | PRESENTER:
            self.assertEqual(layers[pid], "prop", pid)
        self.assertEqual(set(kit["room_dressing"]), {"rest_refuge"})

    def test_the_strips_carry_the_light_entry_of_spec_section_8(self):
        by_id = {p["id"]: p for p in load(CONTRACT)["pieces"]}
        for pid, piece in by_id.items():
            if pid == "ff_light_strip":
                self.assertEqual(piece["lights"], [{
                    "id": "tube", "offset": [24, 3], "colour": "#d8e4ff", "radius": 150, "intensity": 0.24,
                    "group": "scene", "glow": {"radius": 14, "additive": 0.08}}])
            else:
                self.assertNotIn("lights", piece, f"{pid}: the box and every other piece have no emitter (FF-AD6)")
        # no flicker, no floor projection on the strips (gate note G8)
        light = by_id["ff_light_strip"]["lights"][0]
        self.assertNotIn("flicker", light)
        self.assertNotIn("project", light)

    def test_the_state_groups_and_the_unplaced_presenter_pieces(self):
        kit = load(CONTRACT)
        groups = kit["state_groups"]
        self.assertEqual(set(groups), {"ff_staff_door", "ff_reward_box", "ff_bell"})
        self.assertEqual(groups["ff_staff_door"]["states"], ["closed", "rising", "open"])
        self.assertEqual(groups["ff_staff_door"]["anchor"], [1436, 352])
        self.assertEqual(groups["ff_reward_box"]["states"], ["full_a", "full_b", "empty", "sparkle"])
        self.assertEqual(groups["ff_reward_box"]["anchor"], [1204, 556])
        self.assertEqual(groups["ff_bell"]["states"], ["idle", "ding_a", "ding_b"])
        for g in groups.values():
            self.assertIn("presentation", g["note"].lower())
            self.assertIn("does not place", g["note"].lower())
        by_id = {p["id"]: p for p in kit["pieces"]}
        rising = [by_id[f"ff_staff_door_rise_{k:02d}"] for k in range(1, 13)]
        self.assertEqual([p["state_of"] for p in rising],
                         [{"group": "ff_staff_door", "state": "rising", "frame": k} for k in range(1, 13)])
        placed = {p["piece"] for p in placements()}
        self.assertEqual(placed & PRESENTER, set(), "the door, the box, the sparkle and the bell are not placed")

    def test_remove_at_equals_the_spec_list_and_the_rest_of_the_dressing_block(self):
        d = load(CONTRACT)["room_dressing"]["rest_refuge"]
        self.assertEqual([(r["layer"], r["piece"], r["x"], r["y"]) for r in d["remove_at"]], REMOVE_AT)
        self.assertEqual((d["remove"], d["parallax"]), ([], {}), "FF-AD16: no parallax swap")
        self.assertEqual(d["backwall_fill"], [{"x": 264, "y": 352, "w": 40, "h": 192}])

    def test_dressing_equals_the_spec_list(self):
        got = {}
        for p in placements():
            got.setdefault(p["piece"], []).append((p["x"], p["y"], p["layer"]))
        want = {
            "ff_floor_checker_a": [(288 + 32 * n, 561, "backwall_decal") for n in range(36)],
            "ff_floor_checker_edge": [(256, 561, "backwall_decal")],
            "ff_floor_spill": [(168, 561, "backwall_decal")],
            "ff_counter_canopy": [(704, 400, "backwall_decal")], "ff_slide_chute": [(688, 496, "backwall_decal")],
            "ff_service_hatch": [(1304, 400, "backwall_decal")], "ff_under_panel": [(1296, 468, "backwall_decal")],
            "ff_rock_fill": [(904, 352, "backwall_decal")],
            "ff_tile_seam": [(376, 368, "structure")],
            "ff_ceiling_tile": [(288 + 32 * n, 336, "structure") for n in range(36)],
            "ff_soffit_stone": [(x, 336, "structure") for x in list(range(0, 288, 32)) + list(range(1440, 1600, 32))],
            "ff_deck_skin": [(464, 512, "structure")], "ff_slide_top_skin": [(560, 496, "structure")],
            "ff_play_posts": [(464, 528, "structure")], "ff_window_frame": [(900, 384, "structure")],
            "ff_pass_ledge": [(1296, 448, "structure")],
            "ff_light_strip": [(x, 352, "prop") for x in (400, 592, 784, 976, 1168, 1360)],
            "ff_counter_a": [(704, 524, "prop")], "ff_ball_pit": [(560, 548, "prop")],
            "ff_stray_ball": [(1012, 554, "prop")], "ff_bin": [(400, 528, "prop")],
            "ff_booth_a": [(912, 520, "prop")],
            # gate note G8: `_b` at every tile index n with n % 3 == 2, counted from x 288
            "ff_wainscot_a": [(288 + 32 * n, 528, "backwall_decal") for n in range(36) if n % 3 != 2],
            "ff_wainscot_b": [(288 + 32 * n, 528, "backwall_decal") for n in range(36) if n % 3 == 2],
        }
        rects = {"hatch": (1304, 400, 80, 48), "canopy": (704, 400, 128, 40), "under": (1296, 468, 96, 60)}
        tiles = []
        for y in range(368, 497, 32):
            for x in list(range(384, 832, 32)) + list(range(1152, 1440, 32)):
                if all(not (x < r[0] + r[2] and x + 32 > r[0] and y < r[1] + r[3] and y + 32 > r[1])
                       for r in rects.values()):
                    tiles.append((x, y, "backwall_decal"))
        want["ff_wall_tile_a"] = tiles
        self.assertEqual({k: sorted(v) for k, v in got.items()}, {k: sorted(v) for k, v in want.items()})
        # the wainscot never has two `_b` in a row, and the checker never leaves 288..1439
        order = [p["piece"] for p in sorted((p for p in placements() if p["piece"].startswith("ff_wainscot")),
                                            key=lambda p: p["x"])]
        self.assertEqual(len(order), 36)
        self.assertFalse(any(a == b == "ff_wainscot_b" for a, b in zip(order, order[1:])))
        layer = {p["id"]: p["layer"] for p in load(CONTRACT)["pieces"]}
        for p in placements():
            self.assertEqual(layer[p["piece"]], p["layer"], p)
            w, h = SIZES[p["piece"]]
            self.assertTrue(0 <= p["x"] and p["x"] + w <= 1600 and 0 <= p["y"] and p["y"] + h <= 580, p)
            self.assertFalse(p["x"] < 1456 and p["x"] + w > 1440 and p["y"] < 580 and p["y"] + h > 352,
                             f"{p}: nothing over the lock rect (1440, 352, 16, 228)")

    def test_wall_decals_and_props_sit_where_the_spec_says(self):
        for p in placements():
            w, h = SIZES[p["piece"]]
            if p["piece"] in ("ff_wall_tile_a", "ff_wainscot_a", "ff_wainscot_b"):
                self.assertEqual((p["x"] % 32, p["y"] % 16), (0, 0), p)
            if p["layer"] == "prop" and p["piece"] != "ff_light_strip":
                self.assertEqual(p["y"] + h, 560, f"{p}: props stand on the back line")
            if p["piece"].startswith("ff_floor_checker") or p["piece"] == "ff_floor_spill":
                self.assertTrue(561 <= p["y"] and p["y"] + h <= 573, f"{p}: the deck's back half")
        # the wall tile rows lie in y 368..527 and skip the hatch, the canopy and the under-panel
        for p in placements():
            if p["piece"] == "ff_wall_tile_a":
                self.assertTrue(368 <= p["y"] and p["y"] + 32 <= 528, p)

    # -------------------------------------------------------------------- FF-A5: the staff door
    def test_ff_a5_the_door_covers_the_solid_and_opens_the_lane(self):
        closed, opened = ff.ff_staff_door_closed(), ff.ff_staff_door_open()
        cov = sum(closed.get(x, y) is not None for x in range(4, 20) for y in range(228))
        self.assertEqual(cov, 16 * 228, "closed: 3648 of 3648 solid px covered")
        lane = [(x, y) for x in range(4, 20) for y in range(4, 226)]
        self.assertEqual(sum(opened.get(x, y) is not None for x, y in lane), 0, "open: rows 4..225 of the lane clear")
        self.assertEqual(len(lane), 3552)

    def test_every_rising_frame_is_the_pre_baked_door_raised_crop(self):
        pieces = all_pieces()
        for k in range(1, 13):
            self.assertEqual(pieces[f"ff_staff_door_rise_{k:02d}"].px, ff.door_raised(ff.RISE[k - 1]).px, k)
        self.assertEqual(pieces["ff_staff_door_rise_12"].px, pieces["ff_staff_door_open"].px)
        self.assertEqual(ff.RISE, (19, 37, 59, 85, 112, 139, 165, 188, 205, 217, 222, 224))
        # the jamb rails (columns 0..3 and 20..23) stand in every frame
        for pid, img in pieces.items():
            if pid.startswith("ff_staff_door"):
                for x in (0, 1, 2, 3, 20, 21, 22, 23):
                    self.assertTrue(all(img.get(x, y) is not None for y in range(228)), (pid, x))
        # the door atlas holds 14 frames side by side: closed, 12 rising, open
        rects = {p["id"]: p["rect"] for p in load(CONTRACT)["pieces"]}
        order = ["ff_staff_door_closed"] + [f"ff_staff_door_rise_{k:02d}" for k in range(1, 13)] + ["ff_staff_door_open"]
        self.assertEqual([rects[i] for i in order], [[24 * i, 0, 24, 228] for i in range(14)])
        # the panel only rises: each frame's lowest panel row climbs with the table
        for k in range(12):
            before = 0 if k == 0 else ff.RISE[k - 1]
            self.assertGreater(ff.RISE[k], before)
        # Donut (40 px) clears on tick 2 and Carl (52 px) on tick 3, with margin
        self.assertGreaterEqual(ff.RISE[1], 37)
        self.assertGreaterEqual(ff.RISE[2], 52 + 7)

    # --------------------------------------------------------------- FF-A6: the stand lines, rule 10.1
    def test_the_skins_follow_the_pass_ledge_rule(self):
        pieces = all_pieces()
        for pid, w in (("ff_deck_skin", 96), ("ff_slide_top_skin", 128)):
            img = pieces[pid]
            self.assertEqual((img.w, img.h), (w, 16))
            for y in range(16):
                for x in range(w):
                    self.assertIsNotNone(img.px[y][x], (pid, x, y))
            for x in range(w):
                self.assertEqual(img.px[1][x], ("flame", 5), (pid, x))      # row 1 is the stand line, full width
                self.assertEqual(img.px[0][x], ("brass", 3))
                self.assertEqual(img.px[15][x], ("stone_cold", 0))
                for y, v in ((2, 4), (3, 4), (12, 3), (13, 2), (14, 2)):
                    if 0 < x < w - 1:
                        self.assertEqual(img.px[y][x], ("brass", v), (pid, x, y))
                for y in range(4, 12):
                    if 0 < x < w - 1:
                        self.assertIn(img.px[y][x], (("brass", 4), ("brass", 5)), (pid, x, y))
            for y in range(2, 15):
                self.assertEqual((img.px[y][0], img.px[y][w - 1]), (("brass", 3), ("brass", 3)))
        led = pieces["ff_pass_ledge"]
        self.assertTrue(all(led.px[1][x] == ("flame", 5) for x in range(1, 95)))
        self.assertTrue(all(led.px[y][x] is not None for y in range(16) for x in range(96)))

    def test_no_flame5_or_copper8_run_of_8_px_outside_the_stand_lines_and_the_strip_tubes(self):
        for pid, img in all_pieces().items():
            for y in range(img.h):
                x = 0
                while x < img.w:
                    p = img.px[y][x]
                    if p in (("flame", 5), ("copper", 8)):
                        e = x
                        while e + 1 < img.w and img.px[y][e + 1] == p:
                            e += 1
                        if e - x + 1 >= 8:
                            self.assertIn(pid, ("ff_deck_skin", "ff_slide_top_skin", "ff_pass_ledge", "ff_light_strip"),
                                          f"{pid}: a {p} run of {e - x + 1} px on row {y}")
                            if pid == "ff_light_strip":
                                self.assertEqual(p, ("copper", 8))
                                self.assertIn(y, (3, 4), "the tube is two rows")
                            else:
                                self.assertEqual((p, y), (("flame", 5), 1), pid)
                        x = e + 1
                    else:
                        x += 1

    def test_gate_note_g2_no_rival_run_brighter_than_201_within_64_px_of_a_stand_line(self):
        pieces = all_pieces()
        runs = []
        for pid, x, y in placed_instances():
            for x0, x1, yy, pix in bright_runs(pieces[pid], x, y):
                runs.append((pid, x0, x1, yy))
        self.assertTrue(runs, "the model sees the stand lines and the strips")
        for pid, x0, x1, yy in runs:
            own = [s for s in STAND if s[0] == pid and s[3] == yy and x0 >= s[1] and x1 <= s[2]]
            if own:
                continue                                                   # the stand line itself
            for _, sx0, sx1, sy in STAND:
                # the run's rect intersects the stand-line segment's rect grown by 64 px on every side
                near = x0 <= sx1 + 64 and x1 >= sx0 - 64 and yy <= sy + 64 and yy >= sy - 64
                self.assertFalse(near, f"{pid} run x {x0}..{x1} y {yy} is within 64 px of the stand line at y {sy}")
        # each stand line is the brightest run in its envelope: flame 5, luma 250
        self.assertAlmostEqual(luma(("flame", 5)), 250, delta=1)
        names = {r[0] for r in runs}
        self.assertTrue({"ff_deck_skin", "ff_slide_top_skin", "ff_pass_ledge", "ff_light_strip"} <= names)

    def test_the_counter_top_the_booth_table_and_the_bin_are_darker_than_every_stand_line(self):
        pieces = all_pieces()
        for pid in ("ff_counter_a", "ff_booth_a", "ff_bin"):
            top = max(luma(p) for row in pieces[pid].px for p in row if p)
            self.assertLess(top, 201, pid)
        self.assertLessEqual(max(luma(p) for p in pieces["ff_counter_a"].px[0] if p), 135.1)
        # the booth's table top is not brighter than gal 9
        for x in range(32, 80):
            self.assertLessEqual(luma(pieces["ff_booth_a"].px[15][x]), luma(("gal", 9)))

    def test_rule_10_5_nothing_warm_stands_within_48_px_of_the_box(self):
        # gate note G3: the box rect grown by 48 px is x 1156..1283, y 508..579; only the box and the sparkle may
        # be warm there (the checker's cream `stone` is the floor tile and is exempt)
        pieces = all_pieces()
        warm = {"brass", "copper", "flame", "gal_warm"}
        for pid, x, y in placed_instances():
            if pid.startswith(("ff_reward_box", "ff_reward_sparkle")):
                continue
            img = pieces[pid]
            if not (x <= 1283 and x + img.w - 1 >= 1156 and y <= 579 and y + img.h - 1 >= 508):
                continue
            for row in img.px:
                for p in row:
                    if p:
                        self.assertNotIn(p[0], warm, f"{pid}: a warm pixel near the box")
                        self.assertFalse(p[0] == "brick" and p[1] >= 7, pid)
        # the box itself is the warm object: the brightest runs of the box are well away from the shelf line (G1)
        box = pieces["ff_reward_box_full_a"]
        top = max(luma(p) for row in box.px for p in row if p)
        self.assertAlmostEqual(top, luma(("flame", 5)), delta=1, msg="the glint is flame 5")
        lid = [p for p in box.px[7] if p]
        self.assertEqual(len(lid), 28)
        self.assertEqual(sum(p == ("brass", 6) for p in lid), 24, "G1: the lid band (row 7) is brass 6, luma 238")
        self.assertTrue(all(luma(p) < 201 for p in lid if p != ("brass", 6)), "the strap shades and the lid's sides")
        self.assertLess(luma(("flame", 4)), luma(("brass", 6)))

    # ------------------------------------------------------------------------ the other pieces
    def test_the_sparkle_frames_and_the_bell(self):
        pieces = all_pieces()
        for s in "abc":
            img = pieces[f"ff_reward_sparkle_{s}"]
            pix = [p for row in img.px for p in row if p]
            self.assertLessEqual(len(pix), 10, s)
            self.assertTrue(all(p in (("brass", 6), ("flame", 4), ("flame", 5)) for p in pix), s)
        for f in ("idle", "ding_a", "ding_b"):
            img = pieces[f"ff_bell_{f}"]
            self.assertEqual((img.w, img.h), (8, 6))
            pix = [p for row in img.px for p in row if p]
            self.assertTrue(all(p[0] in ("gal", "brass") for p in pix), f"{f}: no flame and no copper")
            self.assertTrue(all(p[1] >= 6 for p in pix if p[0] == "gal"))
            self.assertEqual({p for p in pix if p[0] == "brass"} - {("brass", 4), ("brass", 3)}, set())
            self.assertEqual(bright_runs(img, 0, 0, 8, 201.0), [], f"{f}: no 8 px run brighter than 201")
            self.assertTrue(any(p == ("gal", 10) for p in pix), f"{f}: a glint")
        idle, a, b = (pieces[f"ff_bell_{f}"] for f in ("idle", "ding_a", "ding_b"))
        # ding_a: the plunger moves 1 px down (row 0 is empty, the plunger sits on row 1)
        self.assertTrue(any(idle.px[0]) and not any(a.px[0]))
        self.assertTrue(a.px[1][3] == a.px[1][4] == ("gal", 8))
        # ding_b: idle plus two 1 px gal 10 ticks beside the dome on row 1
        diff = [(x, y) for y in range(6) for x in range(8) if idle.px[y][x] != b.px[y][x]]
        self.assertEqual(sorted(diff), [(0, 1), (7, 1)])
        self.assertTrue(all(b.px[y][x] == ("gal", 10) for x, y in diff))

    def test_the_wainscot_variant_differs_but_shares_the_trim(self):
        a, b = ff.ff_wainscot_a(), ff.ff_wainscot_b()
        self.assertNotEqual(a.px, b.px)
        self.assertEqual(a.px[:3], b.px[:3], "the same top trim")
        self.assertTrue(all(luma(p) <= 135.1 for row in b.px for p in row))

    def test_the_wall_tile_stays_pale_blue_and_mid_value(self):
        t = ff.ff_wall_tile_a()
        pix = {p for row in t.px for p in row}
        self.assertTrue(pix <= {("gal", 6), ("gal", 7), ("gal", 8), ("gal", 9), ("stone_cold", 4)}, pix)
        self.assertLessEqual(max(luma(p) for p in pix), 135.1, "rule 10.2: no white wall field")

    def test_nothing_is_brighter_than_the_stand_lines_in_the_actor_band(self):
        # rule 10.3: wall art between y 524 and 559 stays mid-value (gal 5..9, brick, stone 8 at most)
        pieces = all_pieces()
        for pid, x, y in placed_instances():
            if pid in PRESENTER or pid in ("ff_counter_a", "ff_booth_a", "ff_bin", "ff_ball_pit", "ff_stray_ball",
                                           "ff_play_posts", "ff_floor_checker_a", "ff_floor_checker_edge",
                                           "ff_floor_spill", "ff_slide_chute", "ff_under_panel", "ff_window_frame"):
                continue
            img = pieces[pid]
            for yy in range(img.h):
                if 524 <= y + yy <= 559:
                    for p in img.px[yy]:
                        if p:
                            self.assertLessEqual(luma(p), 140, (pid, y + yy))

    def test_the_key_composites_still_come_from_the_committed_gate_stills(self):
        gate = ROOT / "docs/evidence/area-fastfood-01/gate"
        for name in ("scene_carl_closed_1x.png", "scene_donut_open_1x.png", "measurements.txt"):
            self.assertTrue((gate / name).is_file(), name)
        pins = {line.split()[1]: line.split()[2] for line in (gate / "measurements.txt").read_text().splitlines()
                if line.startswith("KEY ")}
        self.assertEqual(pins, KEY_SHA)


if __name__ == "__main__":
    unittest.main()
