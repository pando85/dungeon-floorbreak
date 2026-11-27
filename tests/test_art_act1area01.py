"""AREA-ACT1-01: the dressing of the Floor 01 act-1 rooms `act1_chase`, `act1_shore` and `act1_guild` (sources,
contract, manifests, sizes, palette, the dressing of spec section 3, the three frozen key frames, the soffit slices, the
step faces, the alcove copy, the re-declared refuge pieces and the evidence numbers)."""
import hashlib
import json
import os
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_act1area01 as act1  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
TERRAIN = ROOT / "assets/source/terrain/floor01"
CONTRACT = act1.CONTRACT
GATE = ROOT / "docs/evidence/area-act1-01/gate/measurements.txt"
EVIDENCE = ROOT / "docs/evidence/area-act1-01"
SIZES = {
    "act1_gallery_breach": (192, 218), "act1_grant_box_closed": (24, 28), "act1_grant_box_open": (24, 28),
    "act1_soffit_a": (176, 96), "act1_soffit_b": (176, 96), "act1_soffit_c": (176, 96), "act1_soffit_d": (176, 96),
    "act1_alcove_niche": (96, 96), "act1_step_mid": (16, 16), "act1_step_end_l": (8, 16), "act1_step_end_r": (8, 16),
}
# The three key frames: frozen at the silhouette gate (gate/measurements.txt `KEY` lines). A key change is a design change.
KEY_SHA = {
    "act1_grant_box_closed": "afcd32c654dec6d4126a5b7ba1b4ed86563621c4dd0947fd973fe694feaf058b",
    "act1_grant_box_open": "e07f0a7e7fcef934c5f3e038410db8224817e391a9c1c2e6a730e3566d8a0dae",
    "act1_gallery_breach": "e50a7f803bee66da91d43b6b4db1e4ae71d6a96c4089226a3f2613f512c7f3ef",
}
# The executor pieces' pins (the pixels spec section 5 rows E ask for, as drawn at the first execution).
EXEC_SHA = {
    "act1_soffit_a": "55c0f265723b67edb03a41a861b4f09a905d86be86e304467b16a159e5d86164",
    "act1_soffit_b": "0ba96f02a0a27e02037872d088e80e38028fbd725e73add6cb8e58e3dd66d696",
    "act1_soffit_c": "ea876ae57beb3becb9f8f7ba598342e1e15aee7ed78a1d874f6de55f33801e2a",
    "act1_soffit_d": "9313f8384fba93516bde915a955064137b0937f5ed946a490a51cdea65663795",
    "act1_step_mid": "0ab80698838112b93223416665ddf5797ba1da8d3712c83e001a0f9a44cb1f5b",
    "act1_step_end_l": "f846b8fa0d8cc3f709eb58d7fff9a20849ed0bf916885823a15190b0df6f90bc",
    "act1_step_end_r": "344c66deedb0b49311deca4e8ebe45b755b627d386af1fb5a694e86cb61a3526",
    "act1_alcove_niche": "1015eef44e49b89eabc7ffaa2a809fcfc9722a2208932889eabd4f45345d57ed",
}
OPEN = {"act1_chase": [(224, 375), (896, 1047), (1232, 1383), (1568, 1719), (1904, 2055), (2240, 2391), (2576, 2727)],
        "act1_shore": [(264, 415)], "act1_guild": [(200, 351)]}
REFUGE_ATLASES = ("floor01_refuge_hearth", "floor01_refuge_candle", "floor01_refuge_props", "floor01_refuge_wall")
REFUGE_PIECES = ("refuge_hearth", "refuge_hearth_flue", "refuge_desk", "refuge_candle_flame", "refuge_stool",
                 "refuge_barrel", "refuge_wall_map", "refuge_bookshelf")


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def sha(img) -> str:
    return hashlib.sha256(json.dumps(img.px).encode()).hexdigest()


def gate_pins() -> dict:
    out = {}
    for line in GATE.read_text(encoding="utf-8").splitlines():
        m = re.match(r"KEY (\S+) sha256=([0-9a-f]+)", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def grid_pixels(path):
    """The first cel of a pixel-grid source as rows of (ramp, index) or None."""
    g = load(path)
    names = {}
    for entry in g["palette"]:
        ramp, _, idx = entry["name"].rpartition("_")
        names[entry["key"]] = (ramp, int(idx))
    return [[names[ch] if ch != "." else None for ch in row] for row in g["frames"][0]["cels"][g["layers"][0]]]


class Act1KitTest(unittest.TestCase):
    pieces = None

    @classmethod
    def setUpClass(cls):
        cls.pieces = act1.all_pieces()
        cls.keys = {pid: fn() for pieces in act1.KEYS.values() for pid, fn in pieces}

    # ------------------------------------------------------------------------------ sources
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects, colours = {}, {}
        for name, spec in act1.ATLASES.items():
            sheet, r = act1._pack(act1.build_pieces(name), spec["width"], act1.PAD)
            grid = act1.kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0))
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name], colours[name] = r, len(grid["palette"])
            self.assertEqual(act1.manifest(name, grid["canvas"], spec["stem"]),
                             load(ROOT / f"assets/manifests/{name}.json"))
            m = load(ROOT / f"assets/manifests/{name}.json")
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            self.assertEqual(m["palette_id"], "floor01-catacomb-v2")
            self.assertEqual(m["type"], "decor_atlas")
        self.assertEqual(act1.contract(rects, colours), load(CONTRACT))

    def test_the_key_grids_of_the_design_step_are_unchanged(self):
        # `write()` rewrites the two key grids; they must equal the committed ones byte for byte.
        for stem, pieces in act1.KEYS.items():
            imgs = [fn() for _, fn in pieces]
            grid = act1.kit.grid_from(imgs, layer_names=["main"], durations=act1.FRAMED.get(stem, [100] * len(imgs)),
                                      pivot=(0, 0))
            self.assertEqual(grid, load(PROPS / f"{stem}.grid.json"), stem)

    # ------------------------------------------------------------------------------ ACT1-ART-A7: the keys
    def test_the_three_key_frames_hold_the_gates_pins(self):
        gate = gate_pins()
        self.assertEqual(gate, KEY_SHA)
        self.assertEqual(act1.key_sha(), gate, "key_sha() is the gate's pins")
        for pid, img in self.keys.items():
            self.assertEqual(sha(img), gate[pid], pid)
        self.assertEqual(set(act1.KEY_PIECE_IDS), set(gate))

    def test_the_keys_in_the_atlas_are_the_keys(self):
        contract = load(CONTRACT)
        by_id = {p["id"]: p for p in contract["pieces"]}
        rows = grid_pixels(PROPS / "act1_keys.grid.json")
        for pid, img in self.keys.items():
            x, y, w, h = by_id[pid]["rect"]
            got = [[rows[y + yy][x + xx] for xx in range(w)] for yy in range(h)]
            self.assertEqual(got, [[tuple(p) if p else None for p in row] for row in img.px], pid)

    def test_the_executor_pieces_hold_their_pins(self):
        self.assertEqual(set(EXEC_SHA), set(act1.EXEC_PIECES))
        for pid, want in EXEC_SHA.items():
            self.assertEqual(sha(self.pieces[pid]), want, pid)

    def test_every_piece_sits_exactly_in_its_atlas_rect(self):
        contract = load(CONTRACT)
        for p in contract["pieces"]:
            pid = p["id"]
            if pid not in self.pieces:
                continue                                              # the re-declared refuge pieces
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
                        self.assertEqual(palette[ch], act1.kit.colour_name(*px), (pid, xx, yy))

    # ------------------------------------------------------------------------------ ACT1-ART-A8: palette and alpha
    def test_native_sizes_colour_budget_and_alpha(self):
        got = {p["id"]: tuple(p["rect"][2:]) for p in load(CONTRACT)["pieces"] if p["id"].startswith("act1_")}
        self.assertEqual(got, SIZES)
        for spec in act1.ATLASES.values():
            grid = load(PROPS / f"{spec['stem']}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, spec["stem"])
            self.assertTrue(all(p["rgba"].endswith("ff") for p in grid["palette"]), "1-bit alpha")
        for pid, img in self.pieces.items():
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is not None for p in flat), pid)
            self.assertTrue(any(p is None for p in flat), f"{pid}: one transparent pixel at least")

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in act1.RAMPS.values() for c in ramp}
        for spec in act1.ATLASES.values():
            for p in load(PROPS / f"{spec['stem']}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{spec['stem']} {p['name']}")

    def test_no_text_glyph_or_face_pieces_and_no_new_ramp(self):
        kit_ramps = set(act1.RAMPS)
        for pid, img in self.pieces.items():
            ramps = {p[0] for row in img.px for p in row if p}
            self.assertLessEqual(ramps, kit_ramps, pid)

    # ------------------------------------------------------------------------------ derived pieces
    def test_the_soffit_slices_are_the_env02_tiles_pixel_for_pixel(self):
        layout = load(ROOT / "assets/levels/floor01/opening_route_proposal.json")
        col = layout["tile_layers"]["collision"]
        rects = {}
        for kit_path in ("floor01_catacomb_kit.json", "floor01_opening_kit.json"):
            for t in load(TERRAIN / kit_path)["tiles"]:
                rects[t["id"]] = (t["atlas"], t["rect"])
        grids = {"floor01_terrain": grid_pixels(TERRAIN / "terrain.grid.json"),
                 "floor01_soffit": grid_pixels(TERRAIN / "soffit.grid.json")}
        pieces = [self.pieces[f"act1_soffit_{c}"] for c in "abcd"]
        notched = 0
        for r in range(6):
            for c, cc in enumerate(range(61, 105)):
                atlas, (rx, ry, rw, rh) = rects[col[r][cc]]
                for y in range(16):
                    for x in range(16):
                        gx, gy = c * 16 + x, r * 16 + y
                        got = pieces[gx // 176].px[gy][gx % 176]
                        want = grids[atlas][ry + y][rx + x]
                        if (gx % 176, gy) == (0, 0):
                            self.assertIsNone(got, "the one cleared corner pixel")
                            notched += 1
                        else:
                            self.assertEqual(got, want, (gx, gy))
        self.assertEqual(notched, 4)
        self.assertEqual(act1.SOFFIT_NOTCHED and sorted(act1.SOFFIT_NOTCHED), [0, 1, 2, 3])

    def test_the_alcove_niche_is_the_env02_rect_copied_exactly(self):
        opening = load(TERRAIN / "floor01_opening_kit.json")
        want_rect = next(p["rect"] for p in opening["pieces"] if p["id"] == "alcove_niche")
        self.assertEqual(want_rect, [0, 0, 96, 96])
        rows = grid_pixels(TERRAIN / "opening_decal.grid.json")
        niche = self.pieces["act1_alcove_niche"]
        for y in range(96):
            for x in range(96):
                self.assertEqual(niche.px[y][x], rows[y][x], (x, y))
        piece = next(p for p in load(CONTRACT)["pieces"] if p["id"] == "act1_alcove_niche")
        self.assertEqual(piece["layer"], "structure")
        (light,) = piece["lights"]
        self.assertEqual((light["id"], light["offset"], light["colour"], light["radius"], light["intensity"],
                          light["group"]), ("alcove_glow", [48, 56], "#f2b060", 96, 0.4, "scene"))
        self.assertNotIn("glow", light, "no flame, no glow sprite (ACT1-ART-Q4)")

    def test_every_step_piece_has_one_transparent_pixel_and_no_bright_stone_below_the_arris(self):
        for pid in ("act1_step_mid", "act1_step_end_l", "act1_step_end_r"):
            img = self.pieces[pid]
            flat = [p for row in img.px for p in row]
            self.assertEqual(sum(1 for p in flat if p is None), 1, pid)
            for y in range(1, img.h):
                for p in img.px[y]:
                    if p is not None and p[0] == "stone":
                        self.assertLessEqual(p[1], 7, (pid, y))
        import author_heat01 as heat
        body = heat._shelf_body()
        mid = self.pieces["act1_step_mid"]
        self.assertEqual((mid.w, mid.h), (16, 16))
        self.assertEqual(mid.px[0], body.px[0], "the arris row is kept")
        for y in range(1, 15):
            for x in range(16):
                q, b = mid.px[y][x], body.px[y][x]
                if b is None:
                    self.assertIsNone(q)
                elif b[0] == "stone":
                    self.assertEqual(q, ("stone", max(0, b[1] - 1)), (x, y))
                else:
                    self.assertEqual(q, b, (x, y))

    def test_the_step_runs_are_the_sidecars_statics(self):
        enc = load(ROOT / "assets/levels/floor01/f01_act1_chase.encounter.json")
        solids = {s["id"]: s["rect"] for s in enc["statics"] if s["kind"] == "solid"}
        self.assertEqual(solids["hump_a"], {"x": 1168, "y": 564, "w": 64, "h": 16})
        self.assertEqual(solids["hump_b"], {"x": 1456, "y": 564, "w": 64, "h": 16})
        self.assertEqual(solids["alcove_ledge"], {"x": 2064, "y": 564, "w": 96, "h": 16})
        self.assertEqual(act1.STEP_RUNS, ((1168, 64), (1456, 64), (2064, 96)))
        widths = {"act1_step_end_l": 8, "act1_step_end_r": 8, "act1_step_mid": 16}
        for x0, w in act1.STEP_RUNS:
            x = x0
            for _, pid, px, py in act1._step_run(x0, w):
                self.assertEqual((px, py), (x, 564))
                x += widths[pid]
            self.assertEqual(x, x0 + w)
        self.assertEqual(act1.HUMP_Y + 16, act1.FLOOR_Y)

    def test_the_grant_box_floor_contact_is_y_579(self):
        for pid in ("act1_grant_box_closed", "act1_grant_box_open"):
            img = self.keys[pid]
            rows = [y for y in range(img.h) if any(p is not None for p in img.px[y])]
            self.assertEqual(act1.BOX_AT[1] + max(rows), 579, pid)
        self.assertEqual(act1.BOX_AT, (340, 552))

    # ------------------------------------------------------------------------------ the contract
    def test_contract_shape_decor_only_and_neutral_ids(self):
        kit = load(CONTRACT)
        self.assertEqual(kit["format"], "env-kit-v2")
        self.assertEqual(kit["extends"], "assets/source/terrain/floor01/floor01_opening_kit.json")
        self.assertEqual(kit["tiles"], [])
        for inherited in ("palette_id", "palette", "palette_policy", "tile_size", "coordinate_system", "layers",
                          "actor_draw_order", "actor_light_group", "roles", "walk_line", "lighting"):
            self.assertNotIn(inherited, kit)
        self.assertEqual(set(kit["room_dressing"]), {"act1_chase", "act1_shore", "act1_guild"})
        for p in kit["pieces"]:
            self.assertFalse(p["collidable"], p["id"])
            self.assertRegex(p["id"], r"^(act1|refuge)_[a-z_0-9]+$")
        for name in act1.ATLASES:
            self.assertRegex(kit["atlases"][name]["manifest"], r"floor01_act1_")
        layers = {p["id"]: p["layer"] for p in kit["pieces"] if p["id"].startswith("act1_")}
        self.assertEqual({k for k, v in layers.items() if v == "structure"},
                         {"act1_gallery_breach", "act1_alcove_niche", "act1_soffit_a", "act1_soffit_b",
                          "act1_soffit_c", "act1_soffit_d", "act1_step_mid", "act1_step_end_l", "act1_step_end_r"})
        self.assertEqual({k for k, v in layers.items() if v == "prop"}, {"act1_grant_box_closed", "act1_grant_box_open"})

    def test_the_refuge_pieces_are_redeclared_by_the_same_definitions(self):
        kit, refuge = load(CONTRACT), load(TERRAIN / "floor01_refuge_kit.json")
        for name in REFUGE_ATLASES:
            self.assertEqual(kit["atlases"][name], refuge["atlases"][name], name)
            self.assertEqual(kit["palette_use"][name], refuge["palette_use"][name], name)
        by_id = {p["id"]: p for p in refuge["pieces"]}
        mine = {p["id"]: p for p in kit["pieces"]}
        for pid in REFUGE_PIECES:
            self.assertEqual(mine[pid], by_id[pid], pid)
        self.assertNotIn("refuge_frame_portrait", mine, "the portrait frame is a face: not used")
        self.assertNotIn("refuge_frame_landscape", mine)

    def test_the_dressing_is_the_spec_section_3(self):
        kit = load(CONTRACT)["room_dressing"]
        key = lambda r: (r["layer"], r["piece"], r["x"], r["y"])      # noqa: E731
        chase = kit["act1_chase"]
        self.assertEqual({key(r) for r in chase["remove_at"]}, set(act1.CHASE_REMOVE_AT))
        self.assertEqual({key(r) for r in chase["remove_at"]}, {
            ("structure", "arch_threshold", 576, 544), ("structure", "arch_chain", 592, 384),
            ("prop", "torch_wall", 768, 448), ("prop", "torch_wall", 824, 432), ("prop", "torch_wall", 864, 448),
            ("backwall_decal", "torch_pool_b", 664, 364), ("backwall_decal", "torch_pool_b", 720, 348),
            ("backwall_decal", "torch_pool_a", 760, 364),
            ("prop", "torch_wall", 1104, 448), ("prop", "torch_wall", 1160, 432), ("prop", "torch_wall", 1440, 448),
            ("prop", "torch_wall", 1496, 432),
            ("backwall_decal", "torch_pool_a", 1000, 364), ("backwall_decal", "torch_pool_b_m", 1056, 348),
            ("backwall_decal", "torch_pool_b", 1336, 364), ("backwall_decal", "torch_pool_a_m", 1392, 348),
            ("prop", "torch_wall", 2112, 448), ("backwall_decal", "torch_pool_a", 2008, 364)})
        self.assertEqual(chase["backwall_fill"], [{"x": 584, "y": 352, "w": 152, "h": 208}])
        at = [(p["layer"], p["piece"], p["x"], p["y"]) for p in chase["place"]]
        for want in (("structure", "act1_gallery_breach", 696, 342), ("structure", "act1_alcove_niche", 2064, 468),
                     *[("structure", f"act1_soffit_{c}", x, 272) for c, x in zip("abcd", (976, 1152, 1328, 1504))]):
            self.assertIn(want, at)
        for x0, w in ((1168, 64), (1456, 64), (2064, 96)):
            self.assertIn(("structure", "act1_step_end_l", x0, 564), at)
            for k in range(8, w - 8, 16):
                self.assertIn(("structure", "act1_step_mid", x0 + k, 564), at)
            self.assertIn(("structure", "act1_step_end_r", x0 + w - 8, 564), at)
        self.assertEqual(len(chase["place"]), 23)
        self.assertLess(at.index(("structure", "act1_gallery_breach", 696, 342)), at.index(
            ("structure", "act1_soffit_a", 976, 272)))
        shore = kit["act1_shore"]
        self.assertEqual({key(r) for r in shore["remove_at"]},
                         {("prop", "torch_wall", 176, 448), ("backwall_decal", "torch_pool_b", 72, 364)})
        self.assertEqual((shore["place"], shore["backwall_fill"]), ([], []))
        guild = kit["act1_guild"]
        self.assertEqual({key(r) for r in guild["remove_at"]},
                         {("prop", "torch_wall", 48, 432), ("backwall_decal", "torch_pool_a_m", -56, 348)})
        placed = {(p["piece"], p["x"], p["y"]) for p in guild["place"]}
        for want in (("refuge_hearth", 64, 500), ("refuge_barrel", 8, 552), ("refuge_stool", 150, 562),
                     ("refuge_desk", 272, 554), ("refuge_candle_flame", 311, 554), ("refuge_wall_map", 474, 484),
                     ("refuge_bookshelf", 474, 524), *[("refuge_hearth_flue", 80, 356 + 16 * i) for i in range(9)]):
            self.assertIn(want, placed)
        self.assertEqual(len(guild["place"]), 16)
        self.assertFalse(any(p["y"] < 352 for p in guild["place"]),
                         "no piece on the ceiling slab y 336..351")
        for room in kit.values():
            self.assertEqual((room["parallax"], room["remove"]), ({}, []))

    def test_the_presenter_pieces_are_not_placed(self):
        placed = {p["piece"] for d in load(CONTRACT)["room_dressing"].values() for p in d["place"]}
        for pid in ("timber_set_intact", "timber_set_smashed", "timber_barricade_full_intact",
                    "timber_barricade_full_smashed", "act1_grant_box_closed", "act1_grant_box_open",
                    "guild_portal_open", "guild_portal_closed"):
            self.assertNotIn(pid, placed, pid)

    # ------------------------------------------------------------------------------ the window rule
    def test_no_prop_pixel_stands_in_an_open_window_above_y_520(self):
        contract = load(CONTRACT)
        by_id = {p["id"]: p for p in contract["pieces"]}
        rows_of = {}
        for room, d in contract["room_dressing"].items():
            for p in d["place"]:
                if p["layer"] != "prop":
                    continue
                piece = by_id[p["piece"]]
                grid = rows_of.setdefault(piece["atlas"], grid_pixels(ROOT / contract["atlases"][piece["atlas"]]["grid"]))
                x0, y0, w, h = piece["rect"]
                for y in range(h):
                    for x in range(w):
                        if grid[y0 + y][x0 + x] is None or p["y"] + y >= 520:
                            continue
                        for a, b in OPEN[room]:
                            self.assertFalse(a <= p["x"] + x <= b, (room, p["piece"], x, y))

    # ------------------------------------------------------------------------------ the composed layouts (when dumped)
    def test_the_composed_layouts_when_dumped(self):
        d = os.environ.get("ACT1_LAYOUT_DIR")
        if not d or not (Path(d) / "act1_chase.dressed.layout.json").is_file():
            self.skipTest("run `ACT1_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_act1_layouts` first")
        torches = {"act1_chase": (25, 17), "act1_shore": (7, 6), "act1_guild": (4, 3)}
        for room, (before, after) in torches.items():
            plain = load(Path(d) / f"{room}.plain.layout.json")
            dressed = load(Path(d) / f"{room}.dressed.layout.json")
            count = lambda lay: sum(1 for p in lay["piece_layers"]["prop"] if p["piece"] == "torch_wall")  # noqa: E731
            self.assertEqual((count(plain), count(dressed)), (before, after), room)
            self.assertEqual(plain["tile_layers"]["collision"], dressed["tile_layers"]["collision"], room)
        chase = load(Path(d) / "act1_chase.dressed.layout.json")
        back = chase["tile_layers"]["backwall"]
        for row in range(22, 35):
            for c in range(584 // 16, 736 // 16):
                self.assertIsNotNone(back[row][c], (row, c))

    # ------------------------------------------------------------------------------ the evidence numbers (when present)
    def test_the_evidence_measurements_pass_when_present(self):
        meas = EVIDENCE / "measurements.txt"
        if not meas.is_file():
            self.skipTest("the evidence has not been generated")
        text = meas.read_text(encoding="utf-8")
        vis = [l for l in text.splitlines() if l.startswith("visibility ")]
        self.assertTrue(vis)
        for line in vis:
            self.assertTrue(line.endswith(" ok"), line)
        # The open lit-read findings (ACT1-ART-Q7, reported to the orchestrator, not hidden): S4's niche against the piers
        # and the guild's box against the desk and hearth against the guide's face. Nothing else may fail.
        fails = [l for l in text.splitlines() if " FAIL" in l]
        self.assertEqual(len(fails), 3, fails)
        self.assertTrue(all(l.startswith(("  S4 verdict", "guild guild_door_")) for l in fails), fails)
        self.assertEqual(sum(l.count("PASS") for l in text.splitlines() if l.startswith("  S2 verdict")), 4)
        self.assertIn("S3 verdict (target 3)", text)
        keys = {m.group(1): m.group(2) for m in re.finditer(r"^KEY (\S+) sha256=([0-9a-f]+)$", text, re.M)}
        self.assertEqual(keys, gate_pins())

    def test_the_captures_are_byte_identical_and_the_hashes_match_when_present(self):
        lines = EVIDENCE / "capture_lines.txt"
        if not lines.is_file():
            self.skipTest("the evidence has not been generated")
        rows = [l for l in lines.read_text(encoding="utf-8").splitlines() if l.startswith("ACT1_ART_STILL ")]
        self.assertTrue(rows)
        for line in rows:
            self.assertIn(" hash_match=yes ", line, line)
            self.assertIn(" passes_byte_identical=yes ", line, line)
            self.assertNotIn(" act1_match=no ", line, line)


if __name__ == "__main__":
    unittest.main()
