"""AREA-HEAT-01: the scorched corral kit of Floor 01 room 0 (sources, contract, sizes, palette, dressing, the frozen
keys, the readability rules of spec section 10 and the gate constraints K2, K3, R1..R8, C2 and C4)."""
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_heat01 as heat  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_heat_kit.json"
SIZES = {  # spec section 5
    "heat_partition_intact": (40, 48), "heat_partition_weakened": (40, 48), "heat_partition_broken": (40, 48),
    "heat_scorch_wall_a": (64, 96), "heat_scorch_wall_b": (48, 64), "heat_scorch_wall_c": (32, 48),
    "heat_trace_wall_a": (16, 16), "heat_scorch_floor_a": (72, 12), "heat_scorch_floor_b": (48, 10),
    "heat_ash_floor_a": (32, 6), "heat_ash_floor_b": (24, 6), "heat_trace_spit_a": (16, 6),
    "heat_trace_spit_b": (12, 6), "heat_shelf_mid_a": (16, 32), "heat_shelf_mid_b": (16, 32),
    "heat_shelf_end_l": (8, 32), "heat_shelf_end_r": (8, 32), "heat_bumper": (32, 80),
    "heat_pen_rail_a": (64, 28), "heat_pen_rail_b": (64, 28), "heat_timber_debris_a": (32, 8),
}
# HEAT-AD8: the key pins after K2, K3 and the R-redraw (sha256 of the piece's pixel grid, ``json.dumps(px)``).
KEY_SHA = {
    "heat_partition_intact": "0baf3cacb6ddb59e714f9608db946f880ccfdd3a6971e1792b0bee6bc1d33b70",
    "heat_partition_weakened": "ea9d8fc0cb5979b57c8f9dc609775843ff146b29b3ee551adece5ee6f6722183",
    "heat_partition_broken": "a371405cc02dc8eaec72fd719fa129840bf6d3ef696fb999af36d07c6ff7d780",
    "heat_scorch_wall_a": "8ea5958749d9662742cb1e7ebd843213f853f4648dd4cdc281f2278fc03b4b78",
    "heat_scorch_floor_a": "30261053739064530cbb0adc4ce6d637b3888a51bda0720656a29ec9e4b2456d",
}
# Frozen after execution: wall_b with the review's foot_x 16 (HEAT-AD20).
PIECE_SHA = {"heat_scorch_wall_b": "2c04b99a89fe1453557cf45d6fa8dca3d4bfd48d768d24da9d21dc62f88e0288"}
# Spec section 5 placements (the pieces `heat_scorch_wall_c` and `heat_trace_wall_a` have no free span: AD12).
WALLS = {"heat_scorch_wall_a": [(1088, 464)], "heat_scorch_wall_b": [(1920, 480)]}
NOT_PLACED = ("heat_scorch_wall_c", "heat_trace_wall_a")
CANON = {"heat_scorch_wall_c": (1120, 512), "heat_trace_wall_a": (392, 544)}   # where the spec draws them
FLOORS = {
    "heat_scorch_floor_a": [(1044, 561), (1456, 561)], "heat_scorch_floor_b": [(560, 562), (1264, 562), (1680, 562)],
    "heat_ash_floor_a": [(288, 563)], "heat_ash_floor_b": [(880, 564), (1792, 564)],
    "heat_trace_spit_a": [(968, 564), (1600, 564)], "heat_trace_spit_b": [(1336, 565)],
}
PROPS_AT = {"heat_pen_rail_a": [(1240, 532), (1720, 532)], "heat_pen_rail_b": [(1560, 532)],
            "heat_timber_debris_a": [(1316, 552), (1648, 552)]}
BAYS = ((112, 320), (448, 656), (784, 992), (1248, 1456), (1584, 1792))
ROOM_W = 2320
DOCK_B = {1, 4, 9, 12, 17, 22}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def pieces_of(name):
    return dict(heat.build_pieces(name))


def all_pieces():
    out = {}
    for name in heat.ATLASES:
        out.update(pieces_of(name))
    return out


def lstar(hex_colour: str) -> float:
    c = [int(hex_colour[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    c = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in c]
    y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    return 116 * (y ** (1 / 3) if y > 216 / 24389 else (24389 / 27 * y + 16) / 116) - 16


def lum(p) -> float:
    return lstar(heat.RAMPS[p[0]][p[1]])


def placements():
    return load(CONTRACT)["room_dressing"]["corral"]["place"]


def placed_at(pid):
    return [(p["x"], p["y"]) for p in placements() if p["piece"] == pid]


def spans(img, rows):
    out = []
    for y in range(rows):
        xs = [x for x in range(img.w) if img.px[y][x] is not None]
        out.append((xs[0], xs[-1] + 1) if xs else None)
    return out


def longest_edge_run(edges):
    """The longest run of consecutive rows whose edge stays within 2 px of the same x."""
    best = 0
    for i, e in enumerate(edges):
        if e is None:
            continue
        lo = hi = e
        j = i
        while j < len(edges) and edges[j] is not None and max(hi, edges[j]) - min(lo, edges[j]) <= 2:
            hi, lo = max(hi, edges[j]), min(lo, edges[j])
            j += 1
        best = max(best, j - i)
    return best


def brick_row_widths(img, p):
    sp = spans(img, p["rows"])
    return [(sp[r * 8 + 4][1] - sp[r * 8 + 4][0]) if sp[r * 8 + 4] else 0 for r in range(p["rows"] // 8)]


def components(img, pred, y0=0, y1=None):
    """4-connected components of pixels with ``pred`` in rows y0..y1-1: lists of (x, y)."""
    y1 = img.h if y1 is None else y1
    seen, out = set(), []
    for y in range(y0, y1):
        for x in range(img.w):
            if (x, y) in seen or not img.px[y][x] or not pred(img.px[y][x]):
                continue
            stack, comp = [(x, y)], []
            seen.add((x, y))
            while stack:
                cx, cy = stack.pop()
                comp.append((cx, cy))
                for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                    if y0 <= ny < y1 and 0 <= nx < img.w and (nx, ny) not in seen and img.px[ny][nx] \
                            and pred(img.px[ny][nx]):
                        seen.add((nx, ny))
                        stack.append((nx, ny))
            out.append(comp)
    return out


class HeatKitTest(unittest.TestCase):
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects = {}
        for name, spec in heat.ATLASES.items():
            grid, r, _ = heat.build(name)
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name] = {"rects": r, "canvas": grid["canvas"], "colours": len(grid["palette"])}
            self.assertEqual(heat.manifest(name, grid["canvas"]), load(ROOT / f"assets/manifests/{name}.json"))
            self.assertEqual(load(ROOT / f"assets/manifests/{name}.json")["review_status"],
                             "prototype_fixture_unapproved")
        self.assertEqual(heat.contract(rects), load(CONTRACT))
        self.assertFalse((PROPS / "heat_scorch.grid.json").exists(), "the design-stage scorch grid is replaced")

    def test_the_keys_hold_their_pins_and_sit_exactly_in_their_atlas_rects(self):
        pieces = all_pieces()
        contract = load(CONTRACT)
        rect = {p["id"]: p["rect"] for p in contract["pieces"]}
        for pid, want in KEY_SHA.items():
            self.assertEqual(hashlib.sha256(json.dumps(pieces[pid].px).encode()).hexdigest(), want, pid)
            atlas = next(p["atlas"] for p in contract["pieces"] if p["id"] == pid)
            grid = load(PROPS / f"{heat.ATLASES[atlas]['stem']}.grid.json")
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
                        self.assertEqual(palette[ch], heat.kit.colour_name(*px), (pid, xx, yy))

    def test_the_wall_scorch_generalisation_keeps_wall_a_byte_identical(self):
        # the shared plume function takes its canvas, band map and parameters: wall_a's own dict is the key
        self.assertEqual(hashlib.sha256(json.dumps(heat.wall_scorch(heat.SW_PLUME).px).encode()).hexdigest(),
                         KEY_SHA["heat_scorch_wall_a"])
        for p in (heat.SW_B, heat.SW_C):
            self.assertNotEqual(p["seed"], "")

    def test_native_sizes_colour_budget_and_alpha(self):
        kit = load(CONTRACT)
        sizes = {p["id"]: tuple(p["rect"][2:]) for p in kit["pieces"]}
        self.assertEqual(sizes, SIZES)
        for spec in heat.ATLASES.values():
            grid = load(PROPS / f"{spec['stem']}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, spec["stem"])
            self.assertTrue(all(p["rgba"].endswith("ff") for p in grid["palette"]), "1-bit alpha")
        for pid, img in all_pieces().items():
            self.assertEqual((img.w, img.h), SIZES[pid], pid)
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is None for p in flat), f"{pid}: one transparent pixel at least")
            self.assertTrue(any(p is not None for p in flat), pid)

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in heat.RAMPS.values() for c in ramp}
        for spec in heat.ATLASES.values():
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
            self.assertTrue(p["id"].startswith("heat_"), p["id"])
        layer = {p["id"]: p["layer"] for p in kit["pieces"]}
        for pid in heat.WALL_PIECES:
            self.assertEqual(layer[pid], "backwall_decal")
        for pid in (pid for pid, _ in heat.ATLASES["floor01_heat_structure"]["pieces"]):
            self.assertEqual(layer[pid], "structure")
        for pid in list(PROPS_AT) + list(heat.PARTITION_IDS):
            self.assertEqual(layer[pid], "prop")
        self.assertEqual(set(kit["room_dressing"]), {"corral"})
        d = kit["room_dressing"]["corral"]
        self.assertEqual((d["remove"], d["parallax"]), ([], {}))
        self.assertNotIn("remove_at", d)

    def test_the_partition_is_a_state_group_and_is_not_placed(self):
        kit = load(CONTRACT)
        group = kit["state_groups"]["heat_partition"]
        self.assertEqual(group["states"], ["intact", "weakened", "broken"])
        self.assertIn("CORRAL-D", group["note"])
        self.assertIn("1380, 532", group["note"])
        by_id = {p["id"]: p for p in kit["pieces"]}
        for pid, state in (("heat_partition_intact", "intact"), ("heat_partition_weakened", "weakened"),
                           ("heat_partition_broken", "broken")):
            self.assertEqual(by_id[pid]["state_of"], {"group": "heat_partition", "state": state})
            self.assertNotIn(pid, {p["piece"] for p in placements()}, "HEAT-AD1: not before CORRAL-D")

    def test_dressing_equals_the_spec_list(self):
        got = {}
        for p in placements():
            got.setdefault(p["piece"], []).append((p["x"], p["y"]))
        want = {}
        for table in (WALLS, FLOORS, PROPS_AT):
            for pid, pos in table.items():
                want[pid] = list(pos)
        want["heat_shelf_end_l"] = [(1120, 548)]
        want["heat_shelf_end_r"] = [(1208, 548)]
        shelf = list(zip(range(1128, 1208, 16), "abaab"))
        dock = [(1888 + 16 * i, "b" if i in DOCK_B else "a") for i in range(26)]
        for v in "ab":
            want[f"heat_shelf_mid_{v}"] = ([(x, 548) for x, s in shelf if s == v]
                                           + [(x, 548) for x, s in dock if s == v])
        want["heat_bumper"] = [(1856, 500)]
        self.assertEqual({k: sorted(v) for k, v in got.items()}, {k: sorted(v) for k, v in want.items()})
        for pid in NOT_PLACED:
            self.assertNotIn(pid, got, f"{pid} has no free span (HEAT-AD12)")
        layer = {p["id"]: p["layer"] for p in load(CONTRACT)["pieces"]}
        for p in placements():
            self.assertEqual(layer[p["piece"]], p["layer"], p)
            w, h = SIZES[p["piece"]]
            self.assertTrue(0 <= p["x"] and p["x"] + w <= ROOM_W and 0 <= p["y"] and p["y"] + h <= 580, p)
            self.assertFalse(p["x"] < 1428 and p["x"] + w > 1372, f"{p}: the partition's clear zone")

    def test_wall_decals_sit_on_the_world_bond_outside_the_bay_openings(self):
        for p in placements():
            if p["layer"] != "backwall_decal":
                continue
            w, h = SIZES[p["piece"]]
            self.assertEqual((p["x"] % 32, p["y"] % 16), (0, 0), p)
            for a, b in BAYS:
                self.assertFalse(p["x"] < b and p["x"] + w > a and p["y"] < 576 and p["y"] + h > 352, (p, a, b))

    def test_floor_decals_stay_in_the_back_half_and_off_the_steps(self):
        for p in placements():
            if p["piece"] not in FLOORS:
                continue
            w, h = SIZES[p["piece"]]
            self.assertTrue(561 <= p["y"] and p["y"] + h <= 573, p)
            self.assertFalse(p["x"] < 1216 and p["x"] + w > 1120, p)
            self.assertFalse(p["x"] < 2304 and p["x"] + w > 1856, p)

    def test_the_step_fronts_lie_on_the_step_rects(self):
        mids = sorted((p["x"], p["piece"]) for p in placements() if p["piece"].startswith("heat_shelf_mid"))
        shelf = [x for x, _ in mids if x < 1216]
        dock = [x for x, _ in mids if x >= 1888]
        self.assertEqual(shelf, list(range(1128, 1208, 16)))
        self.assertEqual(dock, list(range(1888, 2304, 16)))
        self.assertEqual([p for x, p in mids if 1128 <= x < 1208], ["heat_shelf_mid_" + c for c in "abaab"])
        self.assertEqual({p["y"] for p in placements() if p["piece"].startswith(("heat_shelf", "heat_bumper"))
                          and p["piece"] != "heat_bumper"}, {548})

    # ------------------------------------------------------------------------- readability rules
    def placed_pixels(self, pid):
        """(world x, world y, pixel) for every placement of a piece (the canonical spot for an unplaced one)."""
        img = all_pieces()[pid]
        spots = placed_at(pid) or [CANON[pid]]
        for x0, y0 in spots:
            for y in range(img.h):
                for x in range(img.w):
                    if img.px[y][x] is not None:
                        yield x0 + x, y0 + y, img.px[y][x]

    def test_rule_3_actors_read_over_soot(self):
        # no wall decal pixel darker than brick 3 in the head and torso band (y 524..559); brick 2 only in
        # heat_scorch_wall_a rows 0..59 (its head band starts at row 60)
        for pid in heat.WALL_PIECES:
            for x, y, p in self.placed_pixels(pid):
                if p[0] == "brick" and y >= 524:
                    self.assertGreaterEqual(p[1], 3, (pid, x, y))
        img = pieces_of("floor01_heat_wall")
        for pid in ("heat_scorch_wall_b", "heat_scorch_wall_c", "heat_trace_wall_a"):
            self.assertTrue(all(p[1] >= 3 for row in img[pid].px for p in row if p and p[0] == "brick"), pid)
        a = img["heat_scorch_wall_a"]
        self.assertTrue(all(p[1] >= 3 for row in a.px[60:] for p in row if p and p[0] == "brick"))
        self.assertTrue(any(p[1] == 2 for row in a.px[:60] for p in row if p and p[0] == "brick"))
        self.assertTrue(all(p[1] >= 2 for row in a.px for p in row if p and p[0] == "brick"))

    def test_rule_4_embers_and_the_glob_bands(self):
        wall = pieces_of("floor01_heat_wall")
        struct = pieces_of("floor01_heat_structure")
        def flames(img):
            return [(x, y, p[1]) for y, row in enumerate(img.px) for x, p in enumerate(row) if p and p[0] == "flame"]
        for pid, img in list(wall.items()) + list(struct.items()):
            for x, y, v in flames(img):
                self.assertLessEqual(v, 1, pid)
                self.assertGreaterEqual(y, img.h - 6, f"{pid}: embers stay in the bottom 6 rows")
        self.assertEqual(flames(wall["heat_scorch_wall_b"]), [], "wall_b has no embers (the dock bloom)")
        self.assertEqual(len(flames(wall["heat_scorch_wall_a"])), 4)
        self.assertLessEqual(len(flames(struct["heat_trace_spit_a"])), 2)
        self.assertLessEqual(len(flames(struct["heat_trace_spit_b"])), 1)
        # placed embers lie at y 552 or lower
        for pid in list(wall) + list(struct):
            for x0, y0 in (placed_at(pid) or ([CANON[pid]] if pid in CANON else [])):
                img = all_pieces()[pid]
                for x, y, v in flames(img):
                    self.assertGreaterEqual(y0 + y, 552, (pid, x0, y0))
        # no flame, no wood above 4, no stone above 5 in a glob band (532..538 floor, 500..506 dock); the bumper's
        # cap (rows 0..3) stops globs and is exempt
        pieces = all_pieces()
        for p in placements():
            img = pieces[p["piece"]]
            for y in range(img.h):
                wy = p["y"] + y
                if not (532 <= wy <= 538 or 500 <= wy <= 506):
                    continue
                if p["piece"] == "heat_bumper" and y <= 3:
                    continue
                for x in range(img.w):
                    q = img.px[y][x]
                    if q is None:
                        continue
                    self.assertNotEqual(q[0], "flame", p)
                    if q[0] == "wood":
                        self.assertLessEqual(q[1], 4, (p, x, y))
                    if q[0] == "stone":
                        self.assertLessEqual(q[1], 5, (p, x, y))

    def test_rule_1_walkable_tops_are_stone_8(self):
        struct = pieces_of("floor01_heat_structure")
        for pid in ("heat_shelf_mid_a", "heat_shelf_mid_b", "heat_bumper"):
            self.assertTrue(all(p == ("stone", 8) for p in struct[pid].px[0]), pid)
        for pid, outer in (("heat_shelf_end_l", 0), ("heat_shelf_end_r", 7)):
            row = struct[pid].px[0]
            self.assertEqual(row[outer], ("stone", 6), "the rounded corner")
            self.assertTrue(all(p == ("stone", 8) for i, p in enumerate(row) if i != outer), pid)
        # nothing else in the kit is as light as the tops except the partition's pale break faces
        for pid, img in all_pieces().items():
            if pid.startswith("heat_partition") or pid.startswith("heat_shelf") or pid == "heat_bumper":
                continue
            for row in img.px:
                for q in row:
                    if q and q[0] == "stone":
                        self.assertLessEqual(q[1], 7, pid)

    def test_floor_decals_have_no_hard_straight_edge_or_long_runs(self):
        struct = pieces_of("floor01_heat_structure")
        for pid in ("heat_ash_floor_a", "heat_ash_floor_b"):
            img = struct[pid]
            n = sum(1 for row in img.px for q in row if q)
            fleck = sum(1 for row in img.px for q in row if q and q[0] == "iron")
            smudge = sum(1 for row in img.px for q in row if q and q[0] == "floor")
            self.assertLessEqual(fleck / (img.w * img.h), 0.30, pid)
            self.assertLessEqual(smudge / (img.w * img.h), 0.20, pid)
            self.assertGreater(n, 0)
            for row in img.px:
                run = 0
                for q in row:
                    run = run + 1 if q and q[0] == "iron" else 0
                    self.assertLessEqual(run, 3, f"{pid}: an ash run over 3 px")
            for row in img.px:
                for q in row:
                    self.assertTrue(q is None or q[0] in ("iron", "floor"), pid)
            # densest at the centre (b: left of centre), fading to single flecks at the ends
            cols = [sum(1 for y in range(img.h) if img.px[y][x]) for x in range(img.w)]
            third = img.w // 3
            self.assertGreater(sum(cols[third:img.w - third]), sum(cols[:third]) * 0.5, pid)

    def test_pen_rails_rows_0_to_6_are_never_lighter_than_wood_4(self):
        props = pieces_of("floor01_heat_props")
        ref = lum(("wood", 4))
        for pid in ("heat_pen_rail_a", "heat_pen_rail_b"):
            for row in props[pid].px[:7]:
                for q in row:
                    if q:
                        self.assertLessEqual(lum(q), ref + 1e-6, pid)

    def test_debris_has_no_dark_run_over_4_and_a_fresh_break(self):
        img = pieces_of("floor01_heat_props")["heat_timber_debris_a"]
        self.assertLessEqual(img.h, 8)
        for row in img.px:
            run = 0
            for q in row:
                run = run + 1 if q == ("wood", 0) else 0
                self.assertLessEqual(run, 4)
        self.assertEqual(sum(1 for row in img.px for q in row if q == ("stone", 7)), 1)

    # --------------------------------------------------------------------------- the gate: K2 and K3
    def test_k2_no_pair_of_dark_blobs_reads_as_eyes(self):
        for pid in heat.PARTITION_IDS:
            img = all_pieces()[pid]
            # the board gap rows (a full-width 1 px dark line between boards) are structure, not rot
            gaps = {b[1] + 1 for b in heat.BOARDS}
            seen = set()
            blobs = []
            for c in components(img, lambda q: q in (("wood", 0), ("lichen", 0)), 26, 44):
                c = [(x, y) for x, y in c if y not in gaps]
                if 2 <= len(c) <= 4 and not (set(c) & seen):
                    blobs.append(c)
                    seen |= set(c)
            for i, a in enumerate(blobs):
                for b in blobs[i + 1:]:
                    ya, yb = {y for _, y in a}, {y for _, y in b}
                    if ya & yb:
                        gap = max(min(x for x, _ in b) - max(x for x, _ in a),
                                  min(x for x, _ in a) - max(x for x, _ in b))
                        self.assertGreater(gap, 8, (pid, a, b))
            lichen = sum(1 for row in img.px for q in row if q and q[0] == "lichen")
            self.assertLessEqual(lichen, 1, pid)

    def test_k3_no_pale_pixel_stands_above_the_broken_edge(self):
        img = all_pieces()["heat_partition_broken"]
        ref = lum(("wood", 1))
        for y in range(heat.BROKEN_TOP):
            for x, q in enumerate(img.px[y]):
                if not q:
                    continue
                if q == ("stone", 7) and y == heat.BROKEN_TOP - 1:
                    continue                                      # k = 1: directly above the edge
                self.assertLessEqual(lum(q), ref + 1e-6, (x, y, q))
        self.assertEqual({q for q in img.px[heat.BROKEN_TOP] if q} <= {("stone", 8), ("stone", 7)}, True)

    def test_the_partition_cap_is_the_brightest_line_of_intact_and_weakened(self):
        for pid in ("heat_partition_intact", "heat_partition_weakened"):
            img = all_pieces()[pid]
            self.assertTrue(all(q == ("stone", 8) for q in img.px[0][13:27]), pid)     # the lit arris
            self.assertTrue(all(q is None for q in img.px[0][:12] + img.px[0][28:]), "margins are empty at row 0")
            top = lum(("stone", 8))
            # no pixel of the solid columns outside the arris row is as light, the pale break faces of the
            # weakened state excepted (isolated pixels, stone 7..9)
            for y in range(1, img.h):
                for q in img.px[y][12:28]:
                    if q and q[0] == "stone" and q[1] >= 8:
                        self.assertEqual(pid, "heat_partition_weakened", (pid, y, q))
                    elif q and q[0] != "stone":
                        self.assertLess(lum(q), top, (pid, y, q))

    # ------------------------------------------------------------- the gate: the wall scorch R-checks
    def wall_checks(self, p, pid):
        img = pieces_of("floor01_heat_wall")[pid]
        rows = p["rows"]
        sp = spans(img, rows)
        # R3: no hole inside the plume span of a brick row
        for y in range(rows):
            if sp[y] is None:
                continue
            for x in range(*sp[y]):
                self.assertIsNotNone(img.px[y][x], (pid, x, y))
        # no brick that does not rest on a plume brick below (brick (r, col) overlaps one of row r + 1)
        n = rows // 8
        bricks = {}
        for r in range(n):
            for col in range(p["w"] // 16 + 2):
                x0 = col * 16 - 8 * (r % 2)
                if 0 <= x0 and x0 + 16 <= p["w"] and img.px[r * 8 + 4][x0 + 8] is not None:
                    bricks[(r, col)] = x0
        for (r, col), x0 in bricks.items():
            if r == n - 1:
                continue
            self.assertTrue(any(rr == r + 1 and abs(x1 - x0) < 16 for (rr, _), x1 in bricks.items()),
                            f"{pid}: brick ({r}, {col}) floats")
        # R2: no vertical edge run over 16 px
        self.assertLessEqual(longest_edge_run([s[0] if s else None for s in sp]), 16, pid)
        self.assertLessEqual(longest_edge_run([s[1] if s else None for s in sp]), 16, pid)
        # R5: no brick lighter than its band base without the bevel term (the soot floor)
        for r in range(n):
            band = p["bands"][r]
            base = heat.kit.BANDS[band][1]
            top = round(base + 1.5 * 0.9 - heat.SW_SOOT_FLOOR)
            for y in range(r * 8, r * 8 + 8):
                for q in img.px[y]:
                    if q and q[0] == "brick":
                        self.assertLessEqual(q[1], top, (pid, band, y))
        return brick_row_widths(img, p)

    def test_wall_a_r_checks(self):
        widths = self.wall_checks(heat.SW_PLUME, "heat_scorch_wall_a")
        self.assertGreaterEqual(max(widths), 1.6 * widths[-1])
        self.assertEqual((widths[-1], max(widths)), (16, 48))
        img = pieces_of("floor01_heat_wall")["heat_scorch_wall_a"]
        # R4: no brick index anywhere in rows 80..95
        for y in range(80, 96):
            for q in img.px[y]:
                self.assertTrue(q is None or q[0] in ("stone", "mortar", "flame"), (y, q))
        self.assertEqual(sum(1 for y in range(80, 96) for q in img.px[y] if q and q[0] == "mortar" and q[1] == 1) > 0,
                         True)
        # the base course re-shade is at most one step: stone 6 pale body, nothing under stone 2
        stones = [q[1] for y in range(80, 96) for q in img.px[y] if q and q[0] == "stone"]
        self.assertGreaterEqual(min(stones), 2)

    def test_wall_c_r_checks(self):
        widths = self.wall_checks(heat.SW_C, "heat_scorch_wall_c")
        self.assertGreaterEqual(max(widths), 1.6 * widths[-1])
        img = pieces_of("floor01_heat_wall")["heat_scorch_wall_c"]
        for y in range(32, 48):
            for q in img.px[y]:
                self.assertTrue(q is None or q[0] in ("stone", "mortar", "flame"), (y, q))

    def test_wall_b_r_checks(self):
        """HEAT-AD20 (review F2): ``SW_B.foot_x`` 16 puts the core inside the bottom-row brick 8..23, so the foot
        is 16 px and the widest row 32 px (2.0x); no hole, no floater, edge runs short, no brick below 3."""
        widths = self.wall_checks(heat.SW_B, "heat_scorch_wall_b")
        self.assertEqual(widths, [0, 16, 16, 32, 32, 32, 32, 16])
        self.assertGreaterEqual(max(widths), 1.6 * widths[-1])
        img = pieces_of("floor01_heat_wall")["heat_scorch_wall_b"]
        self.assertFalse(any(q and q[0] == "flame" for row in img.px for q in row), "no embers")
        self.assertEqual(hashlib.sha256(json.dumps(img.px).encode()).hexdigest(), PIECE_SHA["heat_scorch_wall_b"])

    def test_wall_decal_rows_over_the_base_course_are_stone_mortar_or_flame(self):
        pieces = all_pieces()
        for p in placements():
            if p["layer"] != "backwall_decal":
                continue
            img = pieces[p["piece"]]
            for y in range(img.h):
                if 544 <= p["y"] + y <= 559:
                    for q in img.px[y]:
                        self.assertTrue(q is None or q[0] in ("stone", "mortar", "flame"), (p, y, q))

    def test_the_unplaced_trace_would_break_the_base_course_rule(self):
        # why heat_trace_wall_a is not placed at the spec's spot: its brick pixels would land on y 544..559
        img = pieces_of("floor01_heat_wall")["heat_trace_wall_a"]
        self.assertTrue(any(q and q[0] == "brick" for row in img.px for q in row))
        self.assertEqual(CANON["heat_trace_wall_a"][1], 544)

    # ---------------------------------------------------------------------------- shelf and bumper
    def test_shelf_courses_and_the_charred_variant(self):
        struct = pieces_of("floor01_heat_structure")
        a, b = struct["heat_shelf_mid_a"], struct["heat_shelf_mid_b"]
        self.assertIsNone(a.px[31][15])
        self.assertTrue(all(q == ("mortar", 0) for q in a.px[9]))
        self.assertTrue(all(q == ("mortar", 1) for q in a.px[20]))
        self.assertEqual(a.px[10][0], ("mortar", 1))
        self.assertEqual(a.px[22][8], ("mortar", 1))
        diff = [(x, y) for y in range(32) for x in range(16) if a.px[y][x] != b.px[y][x]]
        self.assertTrue(diff)
        self.assertTrue(all(3 <= x <= 11 and y >= 14 for x, y in diff), "the char is a tongue over columns 3..11")
        self.assertEqual(sum(1 for row in b.px for q in row if q == ("brick", 5)), 2)

    def test_bumper_straps_and_nick(self):
        img = pieces_of("floor01_heat_structure")["heat_bumper"]
        for y in (35, 36, 67, 68):
            self.assertTrue(all(q and q[0] == "iron" for q in img.px[y]), y)
        for x in (4, 12, 20, 28):
            self.assertEqual(img.px[35][x], ("iron", 5))
        self.assertIsNone(img.px[79][31])
        self.assertEqual(img.px[3][0], ("mortar", 0))


if __name__ == "__main__":
    unittest.main()
