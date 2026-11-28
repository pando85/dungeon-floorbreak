"""AREA-TROG-01 stages 1 and 2: the troglodyte run (`trog_run`) and the Test of Strength (`trog_strength`) kit of Floor 01
(sources, contract, manifests, sizes, palette, the dressing, the sixteen frozen key frames, the step and shelf faces, the
window rule and the evidence numbers). The bench stack is the sidecar's 80 px partition (TROGB-AD1, TROG-Q14): its three
state frames and the six debris frames are packed in `floor01_trog_bench` and drawn by the presenter, never placed by the
dressing."""
import hashlib
import json
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_trogarea01 as trog  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CONTRACT = trog.CONTRACT
GATE = ROOT / "docs/evidence/area-trog-01/gate/measurements.txt"
EVIDENCE = ROOT / "docs/evidence/area-trog-01"
SIZES = {
    "trog_fist_hole": (64, 80), "trog_bell": (40, 64), "trog_knuckle_marks": (96, 40), "trog_slam_crack": (48, 48),
    "trog_impact_crater": (72, 20), "trog_step_mid": (16, 32), "trog_step_end_l": (8, 32), "trog_step_end_r": (8, 32),
    "trog_threshold": (80, 19), "trog_mirror_shards": (96, 74), "trog_bench_intact": (32, 80),
    "trog_bench_weakened": (32, 80), "trog_bench_broken": (32, 80), **{f"trog_bench_debris_{f}": (112, 96) for f in range(6)},
    "trog_shelf48_mid": (16, 48), "trog_shelf48_end_l": (8, 48), "trog_shelf48_end_r": (8, 48),
}
# The sixteen key frames: frozen at the silhouette gate (gate/measurements.txt `KEY` lines). A key change is a design change.
KEY_SHA = {
    "trog_fist_hole": "8633c21d8a0a1106a54c67216e7025d190fcd0dba56a2bac7760eefeb626a2fe",
    "trog_bell": "55073b24ed83939760046dc69159b7b1d1dd376cc2db2bd66044d097d3872a3d",
    "trog_knuckle_marks": "08ef5614592edab894bef8367bd90006f21d9f9c724202aff93d805bb69d6ec1",
    "trog_slam_crack": "b211a5c896d29dbdb15451034211002441275a527621cb3fd6e0dfaf988034f8",
    "trog_impact_crater": "562c6783b1e33613e2b83e14b530ebe78eaba272c6e39fd65e48120f8daa6a25",
    "trog_threshold": "5169396ee926cb91a2873ada9abb76b4a179d9c9ee5a5dba140520c83b4996fd",
    "trog_mirror_shards": "8d41fec088e3034f97c17ca8452c25f74fcdea1e42c076b15ab4ff141a6ff53d",
    "trog_bench_intact": "6c10c65bc747af81e4e0398646a7d024ac6c6a2e1b2b887544a156df639a17fc",
    "trog_bench_weakened": "30bd51e0ebc4189c82298be1f25c2ef9d04f9220bd3edba4f24f1dfa6405fb4f",
    "trog_bench_broken": "d73a20e656c417cf16203b157e5fbd407f0c22787ba92d770873b825ed8ef8c3",
    "trog_bench_debris_0": "b39ea020d3f29a98af52128cdfe941fe971f50302e04faaa85e4d1f5d1fee29e",
    "trog_bench_debris_1": "a257dc74606385f4a74512608c3d1a736fdce506a73ccd92cd6961f9cca0bbf0",
    "trog_bench_debris_2": "78bfa600bb34a16ccd026a3ae0975f2b3937f838eee5d09cbdc0fde000f4d1cd",
    "trog_bench_debris_3": "762ab7aa3cfd078f0e0369f6d6a1e2dd67de2c22069258bc73ee0558d11cf256",
    "trog_bench_debris_4": "b120548f00b4eff1e7f062191b946f7fe71699102cded089200f25ec0b947ebc",
    "trog_bench_debris_5": "8efbb0efbaabbae57d8783ea2b465ea58ee4741a95588e70b216de322f80643b",
}
# The executor pieces' pins (the pixels spec section 5 rows E ask for, as drawn at the first execution).
EXEC_SHA = {
    "trog_step_mid": "2e70563f39ea970b21f39fd23be2dc1049680df67f309e1d9dd80b210801175f",
    "trog_step_end_l": "8178d79ebbc27aeebc3525a283fe1f9994292969deb484aba7def91c27614ae6",
    "trog_step_end_r": "3ea26498d84c8c4b4552088fcdc173a9d361f0300c7925feb888f85db1e24754",
    "trog_shelf48_mid": "323a5459390f50822da22c350b06e9f9cdec024d0398f20446c39e411901b302",
    "trog_shelf48_end_l": "c5a33c888bc65163ed45d829ece07a375b5b390bb631710c5dacce7ed55af9b3",
    "trog_shelf48_end_r": "16bb1a5a72a65d9026a7e9c13e64098e216ad56b24649a2ccce5cee459199172",
}
OPEN = [(520, 671), (856, 1007), (1528, 1679), (2024, 2175)]        # the run's openings that are not bricked up
OPEN_STRENGTH = [(56, 207), (392, 543)]                             # the strength room's (bay 3, 728..879, is bricked)


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def sha(img) -> str:
    return hashlib.sha256(json.dumps(img.px).encode()).hexdigest()


def opaque_bbox(img):
    pts = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x] is not None]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), max(xs), min(ys), max(ys)


def gate_pins() -> dict:
    out = {}
    for line in GATE.read_text(encoding="utf-8").splitlines():
        m = re.match(r"KEY (\S+) sha256=([0-9a-f]+)", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


class TrogKitTest(unittest.TestCase):
    pieces = None

    @classmethod
    def setUpClass(cls):
        cls.pieces = trog.all_pieces()
        cls.keys = {pid: fn() for pieces in trog.KEYS.values() for pid, fn in pieces}

    # ------------------------------------------------------------------------------ sources
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects, colours = {}, {}
        for name, spec in trog.ATLASES.items():
            sheet, r = trog._pack(trog.build_pieces(name), spec["width"], trog.PAD)
            grid = trog.kit.grid_from([sheet], layer_names=["main"], durations=[100], pivot=(0, 0))
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name], colours[name] = r, len(grid["palette"])
            self.assertEqual(trog.manifest(name, grid["canvas"], spec["stem"]),
                             load(ROOT / f"assets/manifests/{name}.json"))
            m = load(ROOT / f"assets/manifests/{name}.json")
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            self.assertEqual(m["palette_id"], "floor01-catacomb-v2")
            self.assertEqual(m["type"], "decor_atlas")
        self.assertEqual(trog.contract(rects, colours), load(CONTRACT))

    def test_the_key_grids_of_the_design_step_are_unchanged(self):
        # `write()` rewrites the eight key grids; they must equal the committed ones byte for byte.
        for stem, pieces in trog.KEYS.items():
            imgs = [fn() for _, fn in pieces]
            if len({(i.w, i.h) for i in imgs}) > 1:
                w, h = max(i.w for i in imgs), max(i.h for i in imgs)
                padded = []
                for i in imgs:
                    p = trog.Pix(w, h)
                    for y in range(i.h):
                        p.px[y][:i.w] = i.px[y]
                    padded.append(p)
                imgs = padded
            grid = trog.kit.grid_from(imgs, layer_names=["main"], durations=trog.FRAMED.get(stem, [100] * len(imgs)),
                                      pivot=(0, 0))
            self.assertEqual(grid, load(PROPS / f"{stem}.grid.json"), stem)

    # ------------------------------------------------------------------------------ TROG-ART-A10: the keys
    def test_the_sixteen_key_frames_hold_the_gates_pins(self):
        gate = gate_pins()
        self.assertEqual(len(gate), 16)
        self.assertEqual(set(gate), set(KEY_SHA))
        self.assertEqual(trog.key_sha(), gate, "key_sha() is the gate's pins (name and digest, not the raw line, G4)")
        for pid, want in KEY_SHA.items():
            if want is not None:
                self.assertEqual(gate[pid], want, pid)
        for pid, img in self.keys.items():
            self.assertEqual(sha(img), gate[pid], pid)
        self.assertEqual(set(trog.KEY_PIECE_IDS), set(gate))

    def test_the_executor_pieces_hold_their_pins(self):
        self.assertEqual(set(EXEC_SHA), set(trog.EXEC_PIECES))
        for pid, want in EXEC_SHA.items():
            self.assertEqual(sha(self.pieces[pid]), want, pid)

    def test_every_piece_sits_exactly_in_its_atlas_rect(self):
        contract = load(CONTRACT)
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
                        self.assertEqual(palette[ch], trog.kit.colour_name(*px), (pid, xx, yy))

    # ------------------------------------------------------------------------------ TROG-ART-A11: palette and alpha
    def test_native_sizes_colour_budget_and_alpha(self):
        got = {p["id"]: tuple(p["rect"][2:]) for p in load(CONTRACT)["pieces"]}
        self.assertEqual(got, SIZES)
        for spec in trog.ATLASES.values():
            grid = load(PROPS / f"{spec['stem']}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, spec["stem"])
            self.assertTrue(all(p["rgba"].endswith("ff") for p in grid["palette"]), "1-bit alpha")
        for pid, img in self.pieces.items():
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is not None for p in flat), pid)
            self.assertTrue(any(p is None for p in flat), f"{pid}: one transparent pixel at least")

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in trog.RAMPS.values() for c in ramp}
        for spec in trog.ATLASES.values():
            for p in load(PROPS / f"{spec['stem']}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{spec['stem']} {p['name']}")

    # ------------------------------------------------------------------------------ the contract
    def test_contract_shape_decor_only_and_neutral_ids(self):
        kit = load(CONTRACT)
        self.assertEqual(kit["format"], "env-kit-v2")
        self.assertEqual(kit["extends"], "assets/source/terrain/floor01/floor01_gym_kit.json")
        self.assertEqual(kit["tiles"], [])
        for inherited in ("palette_id", "palette", "palette_policy", "tile_size", "coordinate_system", "layers",
                          "actor_draw_order", "actor_light_group", "roles", "walk_line", "lighting"):
            self.assertNotIn(inherited, kit)
        self.assertNotIn("state_groups", kit, "the bench frames are the presenter's: no state group")
        self.assertEqual(set(kit["room_dressing"]), {"trog_run", "trog_strength"})
        for p in kit["pieces"]:
            self.assertFalse(p["collidable"], p["id"])
            self.assertEqual(p["anchor"], [0, 0], p["id"])
            self.assertRegex(p["id"], r"^trog_[a-z_0-9]+$")
        for a in kit["atlases"].values():
            self.assertRegex(a["manifest"], r"floor01_trog_")
        layers = {p["id"]: p["layer"] for p in kit["pieces"]}
        self.assertEqual({k for k, v in layers.items() if v == "structure"},
                         {"trog_impact_crater", "trog_step_mid", "trog_step_end_l", "trog_step_end_r", "trog_threshold",
                          "trog_shelf48_mid", "trog_shelf48_end_l", "trog_shelf48_end_r"},
                         "the floor pieces draw above the collision deck")
        self.assertEqual({k for k, v in layers.items() if v == "backwall_decal"},
                         {"trog_fist_hole", "trog_knuckle_marks", "trog_slam_crack", "trog_mirror_shards"})
        self.assertEqual({k for k, v in layers.items() if v == "prop"}, {"trog_bell", "trog_bench_intact", "trog_bench_weakened", "trog_bench_broken",
                          *{f"trog_bench_debris_{f}" for f in range(6)}})

    def test_the_dressing_is_the_spec_sections_4_and_5(self):
        d = load(CONTRACT)["room_dressing"]["trog_run"]
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in d["remove_at"]}, set(trog.RUN_REMOVE_AT))
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in d["remove_at"]},
                         {("structure", "arch_chain", 224, 384), ("structure", "arch_chain", 1232, 384),
                          ("structure", "arch_threshold", 208, 544), ("structure", "arch_threshold", 1216, 544),
                          ("prop", "torch_wall", 2240, 432), ("backwall_decal", "torch_pool_a", 2136, 348)})
        self.assertEqual(d["backwall_fill"], [{"x": 176, "y": 352, "w": 208, "h": 208},
                                              {"x": 1184, "y": 352, "w": 208, "h": 208}])
        self.assertEqual(d["parallax"], {})
        self.assertEqual(d["remove"], [])
        order = ["backwall_decal", "structure", "prop"]
        layers = [p["layer"] for p in d["place"]]
        self.assertEqual(layers, sorted(layers, key=order.index), "backwall decals, then structure, then props")
        at = {(p["piece"], p["x"], p["y"]) for p in d["place"]}
        for want in (("trog_slam_crack", 296, 400), ("trog_knuckle_marks", 272, 488), ("trog_fist_hole", 1312, 440),
                     ("trog_impact_crater", 560, 560), ("trog_bell", 1800, 420)):
            self.assertIn(want, at)
        for x0 in (352, 1840):
            self.assertIn(("trog_step_end_l", x0, 548), at)
            for k in (8, 24, 40):
                self.assertIn(("trog_step_mid", x0 + k, 548), at)
            self.assertIn(("trog_step_end_r", x0 + 56, 548), at)
        self.assertEqual(len(d["place"]), 15)

    def test_the_strength_dressing_is_the_spec_sections_4_and_5(self):
        d = load(CONTRACT)["room_dressing"]["trog_strength"]
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in d["remove_at"]},
                         set(trog.STRENGTH_REMOVE_AT))
        self.assertEqual({(r["layer"], r["piece"], r["x"], r["y"]) for r in d["remove_at"]},
                         {("structure", "arch_chain", 768, 384), ("structure", "arch_threshold", 752, 544),
                          ("prop", "torch_wall", 48, 432), ("backwall_decal", "torch_pool_a_m", -56, 348),
                          ("prop", "torch_wall", 960, 432), ("backwall_decal", "torch_pool_b", 856, 348),
                          ("prop", "torch_wall", 664, 432), ("backwall_decal", "torch_pool_a", 560, 348),
                          ("prop", "torch_wall", 328, 432), ("backwall_decal", "torch_pool_b", 224, 348)})
        self.assertEqual(d["backwall_fill"], [{"x": 720, "y": 352, "w": 208, "h": 208}])
        self.assertEqual(d["parallax"], {})
        order = ["backwall_decal", "structure", "prop"]
        layers = [p["layer"] for p in d["place"]]
        self.assertEqual(layers, sorted(layers, key=order.index), "backwall decals, then structure, then props")
        girders = [p["x"] for p in d["place"] if p["piece"] == "gym_girder"]
        self.assertEqual(girders, [892, 756, 788, 820, 852, 884], "the first girder, drawn under the last, closes the gap")
        props = [(p["piece"], p["x"], p["y"]) for p in d["place"] if p["layer"] == "prop"]
        self.assertEqual(props, [("gym_dumbbell_rack", 812, 532),
                                 ("gym_fluoro_cold", 820, 450), ("gym_plate_tree", 892, 524)])
        at = {(p["piece"], p["x"], p["y"]) for p in d["place"]}
        for want in (("trog_mirror_shards", 792, 470), ("trog_threshold", 672, 561), ("gym_mat_a", 760, 561),
                     ("gym_mat_b", 808, 561), ("gym_mat_a", 856, 561), ("trog_shelf48_end_l", 288, 532),
                     ("trog_shelf48_end_r", 344, 532)):
            self.assertIn(want, at)
        for k in (8, 24, 40):
            self.assertIn(("trog_shelf48_mid", 288 + k, 532), at)
        self.assertEqual(len(d["place"]), 19)

    def test_the_bench_stack_frames_are_the_presenters_and_sit_on_the_partition_prop(self):
        """TROGB-AD1: the three state frames are the partition prop's (target {640, 500, 32, 80}, remnant {640, 564, 32,
        16}); the dressing places none of them; the burst frames are the 112 x 96 canvas of spec section 7."""
        enc = load(ROOT / "assets/levels/floor01/f01_trog_strength.encounter.json")
        self.assertEqual([s["id"] for s in enc["statics"]], ["shelf"], "the bench is no static any more")
        prop = enc["props"][0]
        self.assertEqual((prop["id"], prop["kind"]), ("bench_stack", "partition"))
        self.assertEqual(prop["target"], {"x": 640, "y": 500, "w": 32, "h": 80})
        self.assertEqual(prop["remnant"], {"x": 640, "y": 564, "w": 32, "h": 16})
        for pid in ("trog_bench_intact", "trog_bench_weakened"):
            x0, x1, y0, y1 = opaque_bbox(self.pieces[pid])
            self.assertEqual((640 + x0, 500 + y0, x1 - x0 + 1, y1 - y0 + 1), (640, 500, 32, 80), pid)
        x0, x1, y0, y1 = opaque_bbox(self.pieces["trog_bench_broken"])
        self.assertEqual((640 + x0, 500 + y0, x1 - x0 + 1, y1 - y0 + 1), (640, 564, 32, 16))
        atlas = {p["id"]: p["atlas"] for p in load(CONTRACT)["pieces"]}
        for pid in [p for p, _ in trog.KEYS["trog_bench"]] + [p for p, _ in trog.KEYS["trog_bench_debris"]]:
            self.assertEqual(atlas[pid], "floor01_trog_bench", pid)
        d = load(CONTRACT)["room_dressing"]["trog_strength"]
        self.assertEqual([p["piece"] for p in d["place"] if p["piece"].startswith("trog_bench")], [])

    # ------------------------------------------------------------------------------ the step faces (spec section 5)
    def test_every_step_piece_has_one_transparent_pixel_and_no_bright_stone_below_the_arris(self):
        for pid in trog.EXEC_PIECES:
            img = self.pieces[pid]
            flat = [p for row in img.px for p in row]
            self.assertEqual(sum(1 for p in flat if p is None), 1, pid)
            for y in range(1, img.h):
                for p in img.px[y]:
                    if p is not None and p[0] == "stone":
                        self.assertLessEqual(p[1], 7, (pid, y))
        mid = self.pieces["trog_step_mid"]
        self.assertEqual((mid.w, mid.h), (16, 32))
        # The heat shelf body one stone step darker: the arris row (row 0) is kept.
        import author_heat01 as heat
        body = heat._shelf_body()
        self.assertEqual(mid.px[0], body.px[0])
        for y in range(1, 32):
            for x in range(16):
                q, b = mid.px[y][x], body.px[y][x]
                if b is None:
                    self.assertIsNone(q)
                elif b[0] == "stone":
                    self.assertEqual(q, ("stone", max(0, b[1] - 1)))
                else:
                    self.assertEqual(q, b)

    def test_the_shelf_run_is_the_sidecars_static_and_the_shelf_mid_has_three_courses(self):
        x = trog.SHELF_RUN[0]
        widths = {"trog_shelf48_end_l": 8, "trog_shelf48_end_r": 8, "trog_shelf48_mid": 16}
        for _, pid, px, py in trog._shelf_run(*trog.SHELF_RUN):
            self.assertEqual((px, py), (x, 532))
            x += widths[pid]
        self.assertEqual(x, 288 + 64)
        self.assertEqual(trog.SHELF_RUN[1] + 48, trog.FLOOR_Y)
        mid = self.pieces["trog_shelf48_mid"]
        self.assertEqual((mid.w, mid.h), (16, 48))
        for y in (22, 35):                                           # the bed joints of the three courses
            self.assertTrue(all(p == ("mortar", 1) for p in mid.px[y] if p is not None), y)

    def test_the_step_runs_are_the_sidecars_statics(self):
        enc = load(ROOT / "assets/levels/floor01/f01_trog_run.encounter.json")
        solids = {s["id"]: s["rect"] for s in enc["statics"] if s["kind"] == "solid"}
        self.assertEqual(solids["refuge"], {"x": 352, "y": 548, "w": 64, "h": 32})
        self.assertEqual(solids["bell_step"], {"x": 1840, "y": 548, "w": 64, "h": 32})
        self.assertEqual({(x, y) for (x, y) in trog.STEP_RUNS}, {(352, 548), (1840, 548)})
        for x0, y0 in trog.STEP_RUNS:
            run = trog._step_run(x0, y0)
            widths = {"trog_step_end_l": 8, "trog_step_end_r": 8, "trog_step_mid": 16}
            x = x0
            for _, pid, px, py in run:
                self.assertEqual((px, py), (x, y0))
                x += widths[pid]
            self.assertEqual(x, x0 + 64)
            self.assertEqual(y0 + 32, trog.FLOOR_Y)

    # ------------------------------------------------------------------------------ the keys' read rules
    def test_the_fist_holes_void_stays_above_the_bashers_sprite_top(self):
        void = trog._fist_mask()
        ys = [y for _, y in void]
        self.assertLess(trog.HOLE_AT[1] + max(ys), 493)
        self.assertEqual((trog.HOLE_AT[0] + 63), 1375, "its right edge meets the pier shadow column at 1376")

    def test_no_text_glyph_or_face_pieces_and_no_new_ramp(self):
        kit_ramps = set(trog.RAMPS)
        for pid, img in self.pieces.items():
            ramps = {p[0] for row in img.px for p in row if p}
            self.assertLessEqual(ramps, kit_ramps, pid)

    # ------------------------------------------------------------------------------ the window rule (TROG-ART-A4)
    def test_no_prop_pixel_stands_in_an_open_window_above_y_520(self):
        for p in load(CONTRACT)["room_dressing"]["trog_run"]["place"]:
            if p["layer"] != "prop":
                continue
            img = self.pieces[p["piece"]]
            for y in range(img.h):
                for x in range(img.w):
                    if img.px[y][x] is None or p["y"] + y >= 520:
                        continue
                    for a, b in OPEN:
                        self.assertFalse(a <= p["x"] + x <= b, (p["piece"], x, y))

    def test_no_strength_prop_pixel_stands_in_an_open_window_above_y_520(self):
        for p in load(CONTRACT)["room_dressing"]["trog_strength"]["place"]:
            if p["layer"] != "prop" or p["piece"] not in self.pieces:
                continue
            img = self.pieces[p["piece"]]
            for y in range(img.h):
                for x in range(img.w):
                    if img.px[y][x] is None or p["y"] + y >= 520:
                        continue
                    for a, b in OPEN_STRENGTH:
                        self.assertFalse(a <= p["x"] + x <= b, (p["piece"], x, y))

    def test_floor_pieces_are_structure_and_the_crater_is_inside_an_opening(self):
        contract = load(CONTRACT)
        layer = {p["id"]: p["layer"] for p in contract["pieces"]}
        n = 0
        for p in contract["room_dressing"]["trog_run"]["place"]:
            img = self.pieces[p["piece"]]
            if p["y"] < 580 and p["y"] + img.h > 561:
                self.assertEqual(layer[p["piece"]], "structure", p["piece"])
                n += 1
        self.assertEqual(n, 11)
        x0 = trog.CRATER_AT[0]
        self.assertTrue(any(a <= x0 and x0 + 72 - 1 <= b for a, b in OPEN), "the crater lies within one opening")

    # ------------------------------------------------------------------------------ the evidence numbers (when present)
    def test_the_evidence_measurements_pass_when_present(self):
        meas = EVIDENCE / "measurements.txt"
        if not meas.is_file():
            self.skipTest("the evidence has not been generated")
        text = meas.read_text(encoding="utf-8")
        vis = [l for l in text.splitlines() if l.startswith("visibility ")]
        self.assertEqual(len(vis), 15 + 8, "one line per placed trog_ piece: 15 in the run, 8 in the strength room")
        for line in vis:
            self.assertTrue(line.endswith(" ok"), line)
            self.assertIn("backwall_null_px=0", line)
        keys = {m.group(1): m.group(2) for m in re.finditer(r"^KEY (\S+) sha256=([0-9a-f]+)$", text, re.M)}
        self.assertEqual(keys, gate_pins())

    def test_the_captures_are_byte_identical_and_the_hashes_match_when_present(self):
        lines = EVIDENCE / "capture_lines.txt"
        if not lines.is_file():
            self.skipTest("the evidence has not been generated")
        rows = [l for l in lines.read_text(encoding="utf-8").splitlines() if l.startswith("TROG_ART_STILL ")]
        self.assertTrue(rows)
        for line in rows:
            self.assertIn(" hash_match=yes ", line, line)
            self.assertIn(" passes_byte_identical=yes ", line, line)
            self.assertNotIn(" troga_match=no ", line, line)
            self.assertNotIn(" trogb_match=no ", line, line)


if __name__ == "__main__":
    unittest.main()
