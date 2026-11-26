"""AREA-RINGS-01: the Broken Ring kit of Floor 01 rooms 4a and 4b (sources, contract, sizes, palette, dressing,
the frozen keys and the gate constraints K1..K4 and C1..C5)."""
import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_rings01 as rings  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_rings_kit.json"
SIZES = {  # spec section 5 (crates_b is 56 x 28 after gate C5)
    "ring_arc_wall_a": (408, 164), "ring_arc_wall_b": (408, 164), "ring_relief_landmark": (208, 208),
    "ring_arc_upper_a": (256, 44), "ring_arc_upper_b": (256, 44), "ring_keystone_crowned": (48, 40),
    "ring_shield_notched": (32, 40), "ring_breach_r3": (176, 112), "ring_soffit": (32, 16),
    "ring_ledge_mid": (32, 16), "ring_ledge_end_l": (8, 16), "ring_ledge_end_r": (8, 16),
    "ring_shutter_jamb_l": (8, 68), "ring_shutter_jamb_r": (8, 68), "ring_shutter_housing": (48, 6),
    "ring_shutter_closed": (32, 68), "ring_shutter_grit_a": (32, 28), "ring_shutter_grit_b": (32, 28),
    "ring_shutter_shadow_a": (40, 4), "ring_shutter_shadow_b": (40, 4), "ring_shutter_shadow_c": (40, 4),
    "ring_shutter_dust_a": (48, 10), "ring_shutter_dust_b": (48, 10), "ring_shutter_dust_c": (48, 10),
    "ring_fallen_voussoir_a": (32, 16), "ring_fallen_voussoir_b": (24, 14), "ring_camp_a": (144, 72),
    "ring_crates_b": (56, 28), "ring_marker_a": (24, 56), "ring_tiers_far": (440, 232),
    "ring_tiers_mid": (344, 232), "ring_camp_far": (440, 232),
}
# The keys after the gate edits K1..K4: frozen from here on (sha256 of the piece's pixel grid).
KEY_SHA = {
    "ring_arc_wall_a": "a9c99dc39eee459f0d91be0e59c66053f16c70349a0f783d0b6ee6a952e7a23d",
    "ring_shutter_closed": "2683ac9a8488d9ef566e5fef7b63a961ac3245580cd3477133db4e5d9987e9f1",
    "ring_camp_a": "52fa622a013f313262af1699cb082a5b56bf44de1043fae32017e49a541acce4",
}
ROOM_W = {"ring_court": 2048, "ring_split": 1152}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def pieces_of(name):
    return dict(rings.build_pieces(name))


def all_pieces():
    out = {}
    for name in rings.ATLASES:
        out.update(pieces_of(name))
    return out


def stone_share(img):
    n = sum(1 for row in img.px for p in row if p)
    out = {}
    for row in img.px:
        for p in row:
            if p and p[0] == "stone":
                out[p[1]] = out.get(p[1], 0) + 1
    return {k: v / n for k, v in out.items()}


def longest_run(img, pred):
    best = 0
    for row in img.px:
        run = 0
        for p in row:
            run = run + 1 if p and pred(p) else 0
            best = max(best, run)
    return best


class RingsKitTest(unittest.TestCase):
    def test_sources_contract_and_manifests_are_deterministic(self):
        rects = {}
        for name, spec in rings.ATLASES.items():
            grid, r, _ = rings.build(name)
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
            self.assertTrue((PROPS / f"{spec['stem']}.ase").is_file(), name)
            rects[name] = {"rects": r, "canvas": grid["canvas"], "colours": len(grid["palette"])}
            self.assertEqual(rings.manifest(name, grid["canvas"]), load(ROOT / f"assets/manifests/{name}.json"))
            self.assertEqual(load(ROOT / f"assets/manifests/{name}.json")["review_status"],
                             "prototype_fixture_unapproved")
        self.assertEqual(rings.contract(rects), load(CONTRACT))

    def test_the_keys_are_frozen_after_k1_to_k4_and_sit_exactly_in_their_atlas_rects(self):
        pieces = all_pieces()
        contract = load(CONTRACT)
        rect = {p["id"]: p["rect"] for p in contract["pieces"]}
        for pid, want in KEY_SHA.items():
            self.assertEqual(hashlib.sha256(json.dumps(pieces[pid].px).encode()).hexdigest(), want, pid)
            atlas = next(p["atlas"] for p in contract["pieces"] if p["id"] == pid)
            grid = load(PROPS / f"{rings.ATLASES[atlas]['stem']}.grid.json")
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
                        self.assertEqual(palette[ch], rings.kit.colour_name(*px), (pid, xx, yy))

    def test_native_sizes_colour_budget_and_alpha(self):
        kit = load(CONTRACT)
        sizes = {p["id"]: tuple(p["rect"][2:]) for p in kit["pieces"]}
        self.assertEqual(sizes, SIZES)
        for spec in rings.ATLASES.values():
            self.assertLessEqual(len(load(PROPS / f"{spec['stem']}.grid.json")["palette"]), 63, spec["stem"])
        for pid, img in all_pieces().items():
            self.assertEqual((img.w, img.h), SIZES[pid], pid)
            flat = [p for row in img.px for p in row]
            self.assertTrue(any(p is None for p in flat) or pid in ("ring_tiers_far", "ring_camp_far"), pid)
            self.assertTrue(any(p is not None for p in flat), pid)

    def test_palette_is_the_kit_palette_and_there_is_no_new_value(self):
        kit_colours = {c for ramp in rings.RAMPS.values() for c in ramp}
        for spec in rings.ATLASES.values():
            for p in load(PROPS / f"{spec['stem']}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{spec['stem']} {p['name']}")

    def test_contract_shape_decor_only_and_neutral_ids(self):
        kit = load(CONTRACT)
        self.assertEqual(kit["extends"], "assets/source/terrain/floor01/floor01_opening_kit.json")
        self.assertEqual(kit["tiles"], [])
        self.assertNotIn("palette", kit)
        for p in kit["pieces"]:
            self.assertIs(p["collidable"], False, p["id"])
            self.assertTrue(p["id"].startswith("ring_"), p["id"])
        self.assertEqual(set(kit["room_dressing"]), {"ring_court", "ring_split"})

    def test_dressing_places_known_pieces_in_their_layers_inside_the_room(self):
        kit = load(CONTRACT)
        layer = {p["id"]: p["layer"] for p in kit["pieces"]}
        for room, d in kit["room_dressing"].items():
            self.assertEqual(d["remove"], [])
            soffit = sorted(p["x"] for p in d["place"] if p["piece"] == "ring_soffit")
            self.assertEqual(soffit, list(range(0, ROOM_W[room], 32)), room)
            for p in d["place"]:
                self.assertEqual(layer[p["piece"]], p["layer"], p)
                w, h = SIZES[p["piece"]]
                self.assertTrue(0 <= p["x"] and p["x"] + w <= ROOM_W[room], (room, p))
                self.assertTrue(0 <= p["y"] and p["y"] + h <= 660, (room, p))
                # nothing stands below the walking line; props stand on the floor back line (y 560)
                self.assertLessEqual(p["y"] + h, 580 if p["layer"] != "prop" else 580, (room, p))
            placed = {p["piece"] for p in d["place"]}
            # the shutter slab and its extras are drawn by the presenter, never placed (spec section 6)
            for pid in SIZES:
                if pid.startswith("ring_shutter_") and pid not in ("ring_shutter_jamb_l", "ring_shutter_jamb_r",
                                                                  "ring_shutter_housing"):
                    self.assertNotIn(pid, placed, pid)
            self.assertEqual(set(d["parallax"]), {"parallax_far", "parallax_mid"})
        court = {(p["piece"], p["x"], p["y"]) for p in kit["room_dressing"]["ring_court"]["place"]}
        split = {(p["piece"], p["x"], p["y"]) for p in kit["room_dressing"]["ring_split"]["place"]}
        for want in (("ring_arc_wall_a", 1012, 400), ("ring_relief_landmark", 1656, 356),
                     ("ring_shield_notched", 1256, 524), ("ring_keystone_crowned", 216, 440),
                     ("ring_shutter_jamb_l", 1192, 512), ("ring_shutter_jamb_r", 1232, 512),
                     ("ring_shutter_housing", 1192, 512), ("ring_fallen_voussoir_a", 1328, 544),
                     ("ring_fallen_voussoir_b", 1358, 546)):
            self.assertIn(want, court)
        for want in (("ring_arc_wall_b", 324, 400), ("ring_breach_r3", 896, 380), ("ring_camp_a", 744, 488),
                     ("ring_crates_b", 992, 532), ("ring_marker_a", 1048, 504), ("ring_keystone_crowned", 216, 440)):
            self.assertIn(want, split)
        # ledge fronts: 32 px tiles and two 8 px caps per platform rect
        for room, plats in rings.PLATFORMS.items():
            places = kit["room_dressing"][room]["place"]
            for x, y, w in plats:
                mids = [p for p in places if p["piece"] == "ring_ledge_mid" and p["y"] == y and x <= p["x"] < x + w]
                self.assertEqual(len(mids) * 32, w, (room, x, y))
                self.assertIn(("ring_ledge_end_l", x), [(p["piece"], p["x"]) for p in places if p["y"] == y])
                self.assertIn(("ring_ledge_end_r", x + w - 8), [(p["piece"], p["x"]) for p in places if p["y"] == y])

    def test_no_new_light_emitter_the_parallax_strips_carry_the_gallery_torch_offsets(self):
        kit = load(CONTRACT)
        base = {p["id"]: p["lights"] for p in load(rings.BASE_CATACOMB)["pieces"] if p["id"].startswith("gallery_")}
        lit = {p["id"]: p["lights"] for p in kit["pieces"] if "lights" in p}
        self.assertEqual(sorted(lit), ["ring_camp_far", "ring_tiers_far", "ring_tiers_mid"])
        self.assertEqual(lit["ring_tiers_far"], base["gallery_far"])
        self.assertEqual(lit["ring_camp_far"], base["gallery_far"])
        self.assertEqual(lit["ring_tiers_mid"], base["gallery_mid"])

    def test_k1_the_arc_value_hierarchy(self):
        arc = all_pieces()["ring_arc_wall_a"]
        share = stone_share(arc)
        self.assertNotIn(7, share)
        self.assertNotIn(8, share)
        self.assertLessEqual(share.get(6, 0), 0.03)
        self.assertLessEqual(rings.mean_lstar(arc), 31.0)
        upper = pieces_of("floor01_ring_wall")["ring_arc_upper_a"]
        self.assertLess(rings.mean_lstar(upper), rings.mean_lstar(arc))
        for pid in ("ring_arc_wall_b", "ring_relief_landmark", "ring_keystone_crowned", "ring_shield_notched",
                    "ring_breach_r3"):
            img = all_pieces()[pid]
            self.assertLessEqual(stone_share(img).get(6, 0) + stone_share(img).get(7, 0), 0.06 if pid == "ring_breach_r3" else 0.03, pid)

    def test_c1_no_backwall_piece_has_a_long_lit_horizontal_edge(self):
        pieces = all_pieces()
        for pid in rings.WALL_PIECES:
            if pid == "ring_breach_r3":
                continue                                    # the far camp seen through the hole (gate C1 lists the arcs, ring, keystone, shield)
            self.assertLessEqual(longest_run(pieces[pid], lambda p: p[0] == "stone" and p[1] >= 6), 12, pid)
        # the ledge fronts: a continuous full-width lit top arris of stone 7, a 4 px mortar 0..1 underside
        mid = pieces["ring_ledge_mid"]
        self.assertTrue(all(mid.px[0][x] == ("stone", 7) for x in range(32)))
        for y in (12, 13, 14, 15):
            self.assertTrue(all(p is None or (p[0] == "mortar" and p[1] <= 1) for p in mid.px[y]), y)
        for x in range(32):
            self.assertTrue(4 <= mid.px[5][x][1] <= 6 or mid.px[5][x][0] == "mortar", x)

    def test_c2_the_full_ring(self):
        lm = all_pieces()["ring_relief_landmark"]
        opaque = [(x, y) for y, row in enumerate(lm.px) for x, p in enumerate(row) if p]
        # the whole outer circumference lies between the soffit (top >= 352) and the deck back row (bottom <= 559)
        self.assertGreaterEqual(min(y for _, y in opaque) + 356, 352)
        self.assertLessEqual(max(y for _, y in opaque) + 356, 559)
        # the interior is transparent (no disc, no boss, no stone_cold)
        self.assertIsNone(lm.px[102][104])
        self.assertIsNone(lm.px[60][104])
        self.assertFalse(any(p and p[0] == "stone_cold" for row in lm.px for p in row))
        # r <= 102, the key's band order and pitches: outer 7.5 / notch 5 / inner 9 degrees
        self.assertEqual(rings.ARC_LM.outer[1], 102.0)
        self.assertEqual([rings.ARC_LM.outer[0] > rings.ARC_LM.notch[1], rings.ARC_LM.notch[0] > rings.ARC_LM.inner[1]],
                         [True, True])
        b = all_pieces()["ring_arc_wall_b"]
        self.assertTrue(all(rings.ARC_B.brk[k][1] < 0 for k in rings.ARC_B.brk), "b breaks on the left")
        self.assertEqual((rings.ARC_B.outer, rings.ARC_B.notch, rings.ARC_B.inner),
                         (rings.ARC_A.outer, rings.ARC_A.notch, rings.ARC_A.inner))
        self.assertNotEqual(b.px, all_pieces()["ring_arc_wall_a"].px)

    def test_k2_k3_the_shutter_is_the_darkest_mass_and_has_no_mark(self):
        sh = all_pieces()["ring_shutter_closed"]
        self.assertLessEqual(rings.mean_lstar(sh), 19.0)
        body = [p for row in sh.px[3:60] for p in row if p and p[0] == "stone"]
        self.assertTrue(body)
        self.assertLessEqual(max(p[1] for p in body), 4)
        # K3: no carved mark: the old roundel area has no mortar 2/3 arc; course 2 holds only stone and the band
        for y in range(18, 34):
            for x in range(10, 22):
                p = sh.px[y][x]
                self.assertTrue(p[0] in ("stone", "iron") or (p == ("mortar", 2) and y == 17 + 0), (x, y, p))
        # a horizontal iron band across the full width (horizontal = slab, vertical-only = door)
        self.assertTrue(all(sh.px[24][x][0] == "iron" for x in range(1, 31)))
        # the iron shoe is unchanged: the sweep edge (row 64) is the brightest line of the slab
        self.assertEqual(sh.px[64][5], ("iron", 6))

    def test_k4_the_bedroll_is_a_cylinder_mostly_outside_the_dark_interior(self):
        camp = all_pieces()["ring_camp_a"]
        cells = [(x, y) for y, row in enumerate(camp.px) for x, p in enumerate(row) if p and p[0] == "lichen"]
        xs = [x for x, _ in cells]
        ys = [y for _, y in cells]
        self.assertEqual((min(xs), max(xs), min(ys), max(ys)), (44, 68, 62, 71))
        # flat top and bottom rows, squared ends
        for y in (62, 71):
            self.assertTrue(all(camp.px[y][x] and camp.px[y][x][0] in ("lichen", "wood") for x in range(44, 69)), y)
            self.assertEqual(sum(1 for x in range(44, 69) if camp.px[y][x][0] == "lichen"), 21, y)
        self.assertTrue(all(camp.px[y][68] == ("lichen", 0) for y in range(62, 72)))
        # at least half of its length lies right of the dark interior (x 14..53)
        self.assertGreaterEqual((68 - 53) / (68 - 44 + 1), 0.5)
        # two dark wood 0..1 ties
        ties = {x for x in range(44, 69) if camp.px[66][x] and camp.px[66][x][0] == "wood"}
        self.assertEqual(len(ties), 4)
        for x in ties:
            self.assertLessEqual(camp.px[66][x][1], 1)
        # no lichen green within 16 px of a goblin's x at its body band, in any placed piece
        kit = load(CONTRACT)
        pieces = all_pieces()
        goblins = {"ring_court": (1424, 1824), "ring_split": (560, 848)}
        for room, d in kit["room_dressing"].items():
            for p in d["place"]:
                img = pieces[p["piece"]]
                for y in range(img.h):
                    for x in range(img.w):
                        px = img.px[y][x]
                        if px and px[0] == "lichen" and 544 <= p["y"] + y < 580:
                            for gx in goblins[room]:
                                self.assertGreater(abs(p["x"] + x - gx), 16, (room, p["piece"], x, y, gx))

    def test_c5_props_never_pose_as_platforms(self):
        pieces = all_pieces()
        self.assertEqual(SIZES["ring_crates_b"][1], 28)                      # top at 532, below the R3 ledge top 516 + 10
        for pid in ("ring_fallen_voussoir_a", "ring_fallen_voussoir_b"):
            img = pieces[pid]
            rows = [y for y, row in enumerate(img.px) if any(row)]
            self.assertLessEqual(max(rows) - min(rows) + 1, 14, pid)
            self.assertLessEqual(longest_run(img, lambda p: p[0] == "stone" and p[1] >= 5), 12, pid)
        # the camp's crate stack and the marker stay off every walkable top
        for pid in ("ring_camp_a", "ring_crates_b", "ring_marker_a"):
            self.assertLessEqual(longest_run(pieces[pid], lambda p: p[0] == "stone" and p[1] >= 7), 12, pid)

    def test_silhouettes_are_flat_faceless_and_small(self):
        for fn, h in ((rings.silhouette_standing, 24), (rings.silhouette_seated, 14)):
            c = rings.Pix(40, 40)
            fn(c, 20, 39, h)
            cells = [(x, y) for y, row in enumerate(c.px) for x, p in enumerate(row) if p]
            self.assertLessEqual(max(y for _, y in cells) - min(y for _, y in cells) + 1, 24)
            self.assertTrue(all(c.px[y][x][0] == "gal" and c.px[y][x][1] <= 1 for x, y in cells))
        # only the breach and the far camp carry silhouettes: a, b and c, d (at most two per piece)
        src = (ROOT / "scripts/author_rings01.py").read_text(encoding="utf-8")
        self.assertEqual(src.count("silhouette_standing(c,"), 2)
        self.assertEqual(src.count("silhouette_seated(c,"), 2)

    def test_marker_lantern_is_pixel_identical_to_the_camp_lantern(self):
        pieces = all_pieces()
        camp, marker = pieces["ring_camp_a"], pieces["ring_marker_a"]
        # camp lantern origin (84, 10); marker lantern origin (12, 29); body and spill rows 0..13, x -6..+12
        for dy in range(0, 14):
            for dx in range(-6, 11):
                self.assertEqual(camp.px[10 + dy][84 + dx], marker.px[29 + dy][12 + dx], (dx, dy))

    def test_the_shutter_travel_frames_are_row_crops_of_the_key(self):
        from PIL import ImageChops
        key = rings.to_image(rings.ring_shutter_closed())
        for e in (0, 17, 34, 51, 68):
            frame = rings.shutter_frame(e)
            want = key.crop((0, rings.SH_H - e, rings.SH_W, rings.SH_H)) if e else None
            if want is None:
                self.assertIsNone(frame.getbbox())
            else:
                self.assertIsNone(ImageChops.difference(frame.crop((0, 0, rings.SH_W, e)).convert("RGBA"),
                                                        want.convert("RGBA")).getbbox())

    def test_contract_validates_against_the_exported_atlases(self):
        if not (ROOT / "generated/art/floor01_ring_wall.png").is_file():
            self.skipTest("atlases not exported (bash scripts/export_assets.sh)")
        out = subprocess.run([sys.executable, str(ROOT / "scripts/env_kit.py"), "validate", "--contract",
                              str(CONTRACT.relative_to(ROOT))], cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stderr + out.stdout)


if __name__ == "__main__":
    unittest.main()
