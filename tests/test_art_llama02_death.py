"""LLAMA-02-DEATH (Bad Llama crush, decapitation and heavy-impact death poses, death-parts sheet): spec rules the
shared pixel-art toolkit does not cover. The toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/llama/death/llama_death_h64.pixelart.json`` and ``llama_death_parts.pixelart.json``.
See ``docs/02-art/llama-02-death-spec.md`` section 10."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_llama02_death as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

LLAMA = ROOT / "assets/source/characters/llama/llama-01.grid.json"
GOB_DEATH = ROOT / "assets/source/characters/goblin/death/goblin_a_death_h36.grid.json"
GOB_PARTS_MANIFEST = ROOT / "assets/manifests/goblin_a_death_parts.json"
POSE_FIXTURE = ROOT / "tests/fixtures/llama-02-death_keys.json"
PART_FIXTURE = ROOT / "tests/fixtures/llama-02-death-parts_keys.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def opaque(rows: list[str]) -> dict[tuple[int, int], str]:
    return {(x, y): ch for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."}


class Llama02DeathSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.llama = load(LLAMA)

    def test_committed_sources_and_manifests_match_the_generator(self):
        self.assertEqual(ad.POSES.read_text(encoding="utf-8"), ad.dump(ad.poses_grid()))
        self.assertEqual(ad.PARTS.read_text(encoding="utf-8"), ad.dump(ad.parts_grid()))
        for grid_path, ase in ((ad.POSES, ad.POSES_ASE), (ad.PARTS, ad.PARTS_ASE)):
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(load(grid_path)))
        for name, build in ad.MANIFESTS:
            self.assertEqual((ROOT / "assets/manifests" / name).read_text(encoding="utf-8"), ad.manifest_text(build))

    def test_keys_are_frozen(self):
        self.assertEqual(sorted(load(POSE_FIXTURE)["keys"], key=int), ["0", "2", "3", "5", "6", "7"])
        self.assertEqual(sorted(load(PART_FIXTURE)["keys"], key=int), ["0", "1", "2"])

    def test_llama01_frame_contract_and_grounding(self):
        self.assertEqual(self.poses["canvas"], self.llama["canvas"])
        self.assertEqual(self.poses["pivot"], self.llama["pivot"])
        self.assertEqual([t["name"] for t in self.poses["tags"]],
                         ["crush_contact", "crushed_remains", "decap_contact", "headless_collapse", "slam_contact",
                          "broken_heap"])
        for n in range(len(self.poses["frames"])):
            self.assertEqual(pg._bbox(pg.composite_keys(self.poses, n))[3], 71, n)

    def test_palette_is_llama_plus_the_shared_gore_ramp(self):
        for grid in (self.poses, self.parts):
            pal = {e["key"]: e["rgba"] for e in grid["palette"]}
            self.assertLessEqual(len(pal), 20)
            for e in self.llama["palette"]:
                self.assertEqual(pal[e["key"]], e["rgba"], e["key"])
            gob = {e["key"]: e["rgba"] for e in load(GOB_DEATH)["palette"]}
            self.assertEqual(pal["B"], gob["B"])
            self.assertEqual(pal["b"], gob["R"])        # the goblin's red_mid, shared blood body
            self.assertEqual(pal["P"], gob["P"])

    def test_layer_rule_and_closed_body(self):
        for grid in (self.poses, self.parts):
            self.assertEqual(grid["layers"], ["body", "gore", "molten"])
            w, h = grid["canvas"]
            for n, frame in enumerate(grid["frames"]):
                body = opaque(frame["cels"]["body"])
                gore = set(opaque(frame["cels"]["gore"]).values())
                molten = set(opaque(frame["cels"]["molten"]).values())
                self.assertFalse(set(body.values()) & ad.GORE_ONLY, n)
                self.assertLessEqual(gore, ad.GORE_KEYS, n)
                self.assertLessEqual(molten, ad.MOLTEN_KEYS, n)
                for (x, y), ch in body.items():          # bloodless body is closed: colour only touches opaque pixels
                    if ch == "K":
                        continue
                    for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                        if 0 <= nx < w and 0 <= ny < h:
                            self.assertTrue((nx, ny) in body, (n, x, y))

    def test_crush_stone_footprint_and_gland_burst(self):
        body = opaque(self.poses["frames"][0]["cels"]["body"])
        x0, x1 = ad.STONE
        self.assertFalse([p for p in body if x0 < p[0] < x1 and p[1] < ad.STONE_ROW])   # edge columns: outline only
        line = [x for x in range(x0, x1 + 1) if body.get((x, ad.STONE_ROW)) == "K"]
        self.assertGreaterEqual(len(line), 30)
        molten = opaque(self.poses["frames"][0]["cels"]["molten"])
        self.assertLess(min(y for _x, y in molten), 40)                  # the geyser rises above the stone line
        self.assertGreaterEqual(sum(1 for ch in molten.values() if ch == "Y"), 8)

    def test_decap_body_is_the_llama01_hurt_body_below_the_cut(self):
        hurt = opaque(self.llama["frames"][11]["cels"]["body"])
        body = opaque(self.poses["frames"][3]["cels"]["body"])
        self.assertEqual({p: c for p, c in body.items() if p[1] >= 30}, {p: c for p, c in hurt.items() if p[1] >= 30})
        self.assertFalse({"e", "T", "t"} & set(body.values()))        # no head: no eye, no teeth

    def test_headless_body_is_the_llama01_down_body_behind_the_neck(self):
        dead = opaque(self.llama["frames"][14]["cels"]["body"])
        body = opaque(self.poses["frames"][5]["cels"]["body"])
        self.assertEqual({p: c for p, c in body.items() if p[0] <= 48}, {p: c for p, c in dead.items() if p[0] <= 48})

    def test_preset_one_hides_blood_and_keeps_molten(self):
        for grid in (self.poses, self.parts):
            p1 = ad.preset_view(grid, 1)
            p2 = ad.preset_view(grid, 2)
            for n in range(len(grid["frames"])):
                self.assertNotIn("gore", p1["frames"][n]["cels"])
                self.assertEqual(p1["frames"][n]["cels"]["molten"], grid["frames"][n]["cels"]["molten"])
                muted = set("".join(p2["frames"][n]["cels"]["gore"]))
                self.assertFalse(muted & {"b", "P"}, n)

    def test_parts_manifest_uses_the_goblin_parts_schema(self):
        gob = load(GOB_PARTS_MANIFEST)["death_parts"]
        mine = load(ROOT / "assets/manifests/llama_death_parts.json")["death_parts"]
        self.assertLessEqual(set(gob) - {"parts"}, set(mine))
        required = {"id", "frame", "cell", "pivot", "anchors", "frame0_origin", "class", "material", "presets",
                    "gore_layer", "mass", "spin", "settle", "bounce", "ttl"}
        self.assertEqual([p["id"] for p in mine["parts"]], ad.PART_IDS)
        optional = {"swap_on_first_contact", "on_contact", "floor_decal", "span", "pair", "pair_offset", "stain", "trail"}
        for entry in mine["parts"]:
            self.assertLessEqual(required, set(entry), entry["id"])
            self.assertLessEqual(set(entry) - required - optional, {"molten_layer", "swap_on_ttl"}, entry["id"])
            for name, (x, y) in entry["anchors"].items():
                self.assertTrue(0 <= x < 32 and 0 <= y < 32, (entry["id"], name))

    def test_parts_fit_the_launched_part_cap(self):
        for n in range(len(self.parts["frames"])):
            x0, y0, x1, y1 = pg._bbox(pg.composite_keys(self.parts, n))
            self.assertLessEqual(x1 - x0 + 1, 28, n)
            self.assertLessEqual(y1 - y0 + 1, 28, n)


PARTS_MANIFEST = ROOT / "assets/manifests/llama_death_parts.json"
POSES_MANIFEST = ROOT / "assets/manifests/llama_death_h64.json"
EVIDENCE = ROOT / "docs/evidence/llama-02-death"
SPEC_PART_IDS = [
    "head", "neck", "leg_fore", "head_dead", "torso_front", "torso_rump", "leg_hind", "leg_fore_far", "leg_hind_far",
    "ear", "jaw", "tooth", "gland", "wool_tuft_a", "wool_tuft_b", "gib_flesh_a", "gib_flesh_b", "gib_bone", "gib_rib",
    "drop_blood", "drop_molten", "decal_splat_s", "decal_splat_m", "decal_pool_1", "decal_pool_2", "decal_pool_3",
    "decal_pool_3_r", "decal_wall_spatter", "decal_wall_spatter_b", "decal_smear", "decal_molten", "decal_molten_crust",
]
FUR_BLOOD = "#b8322e"
FUR_MID = "#a37c58"


def flat(grid: dict, n: int) -> dict[tuple[int, int], str]:
    return opaque(pg.composite_keys(grid, n))


class Llama02DeathExecutorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.pm = load(POSES_MANIFEST)["death_poses"]
        cls.qm = load(PARTS_MANIFEST)["death_parts"]
        cls.entries = {e["id"]: e for e in cls.qm["parts"]}

    def idx(self, pid: str) -> int:
        return SPEC_PART_IDS.index(pid)

    def test_frame_and_part_counts_and_order(self):
        self.assertEqual(len(self.poses["frames"]), 8)
        self.assertEqual(len(self.parts["frames"]), 32)
        self.assertEqual(ad.PART_IDS, SPEC_PART_IDS)
        self.assertEqual([e["id"] for e in self.qm["parts"]], SPEC_PART_IDS)
        self.assertEqual([f["author"] for f in self.pm["frames"]],
                         ["key", "executor", "key", "key", "executor", "key", "key", "key"])
        self.assertEqual([f["ticks"] for f in self.pm["frames"]], [3, 3, None, 4, 6, None, 6, None])
        frames = [json.dumps(f["cels"], sort_keys=True) for f in self.poses["frames"]]
        self.assertEqual(len(set(frames)), 8)             # no placeholder copies remain
        frames = [json.dumps(f["cels"], sort_keys=True) for f in self.parts["frames"]]
        self.assertEqual(len(set(frames)), 32)

    def test_pose_and_part_anchors_land_on_drawn_pixels(self):
        for f in self.pm["frames"]:
            drawn = flat(self.poses, f["index"])
            for name, (x, y) in f["anchors"].items():
                self.assertIn((x, y), drawn, (f["index"], name))
        for e in self.qm["parts"]:
            drawn = flat(self.parts, e["frame"])
            for name, (x, y) in e["anchors"].items():
                if name == "centre":                       # the rotation centre is the cell pivot: inside the bitmap
                    xs, ys = [p[0] for p in drawn], [p[1] for p in drawn]
                    self.assertTrue(min(xs) <= x <= max(xs) and min(ys) <= y <= max(ys), (e["id"], name))
                elif e["id"] == "decal_pool_3_r" and name == "centre_bottom":
                    self.assertIn((x + 1, y), drawn)       # the shared centre-bottom pixel sits one column left
                else:
                    self.assertIn((x, y), drawn, (e["id"], name))

    def test_moved_anchors_of_the_in_between_frames(self):
        anchors = {f["index"]: f["anchors"] for f in self.pm["frames"]}
        self.assertEqual(anchors[1]["hit_origin"], [42, 55])
        self.assertEqual(anchors[1]["mouth"], anchors[0]["mouth"])
        self.assertEqual(anchors[4], {"neck": [64, 36], "jet_base": [64, 35], "gland": [66, 36]})
        body = opaque(self.poses["frames"][1]["cels"]["body"])
        self.assertGreaterEqual(len([x for x in range(13, 77) if body.get((x, 55)) == "K"]), 30)
        self.assertFalse([p for p in body if 13 < p[0] < 76 and p[1] < 55])
        molten = opaque(self.poses["frames"][1]["cels"]["molten"])
        self.assertLess(min(y for _x, y in molten), min(y for _x, y in opaque(self.poses["frames"][0]["cels"]["molten"])))
        # the popped eye: bone white where frame 0 had a pupil
        self.assertGreater(sum(1 for c in body.values() if c == "e"), sum(
            1 for c in opaque(self.poses["frames"][0]["cels"]["body"]).values() if c == "e"))

    def test_buckle_is_a_headless_body_with_a_bent_jet(self):
        body = opaque(self.poses["frames"][4]["cels"]["body"])
        gore = opaque(self.poses["frames"][4]["cels"]["gore"])
        self.assertFalse({"e", "T", "t"} & set(body.values()))
        self.assertGreaterEqual(sum(1 for c in gore.values() if c == "b"), 20)
        top = min(y for _x, y in gore)
        self.assertLess(top, 30)
        self.assertGreater(min(x for x, y in gore if y == top) + 0, 40)
        self.assertEqual(pg._bbox(pg.composite_keys(self.poses, 4))[3], 71)

    def test_preset_filtering_by_the_presets_list(self):
        def at(preset: int, cls=None) -> set[str]:
            return {e["id"] for e in self.qm["parts"] if preset in e["presets"] and (cls is None or e["class"] == cls)}
        self.assertEqual(self.entries["neck"]["presets"], [4])           # gate ruling N2
        self.assertEqual(at(3, "wound"), {"head", "head_dead", "torso_front", "torso_rump", "leg_fore", "leg_hind"})
        self.assertEqual(at(2, "wound"), {"head", "head_dead", "torso_front"})
        self.assertEqual(at(1, "wound"), {"head", "head_dead"})
        for pid in ("gib_bone", "gib_rib", "decal_pool_3", "decal_pool_3_r", "decal_wall_spatter",
                    "decal_wall_spatter_b", "decal_smear"):
            self.assertEqual(self.entries[pid]["presets"], [4], pid)
        for e in self.qm["parts"]:                                       # clean and molten parts: every preset
            if e["class"] in ("clean", "molten"):
                self.assertEqual(e["presets"], [1, 2, 3, 4], e["id"])
            if e["class"] == "blood":
                self.assertNotIn(1, e["presets"], e["id"])
        self.assertEqual(at(1, "blood"), set())
        self.assertEqual(self.qm["filter_by"], "the per-part presets list, never the class name")

    def test_layer_metadata_in_both_manifests(self):
        for m in (self.pm, self.qm):
            self.assertEqual(m["preset_layer"], "gore")
            self.assertEqual(m["always_shown_layers"], ["body", "molten"])

    def test_hazard_draw_flag_on_the_crush_contact_frames_only(self):
        flags = {f["index"]: f.get("hazard_draw") for f in self.pm["frames"]}
        self.assertEqual({n: v for n, v in flags.items() if v}, {0: "over", 1: "over"})
        self.assertEqual(self.pm["hazard_draw"]["frames"], [0, 1])
        self.assertEqual(self.pm["hazard_draw"]["default_when_absent"], "behind")
        self.assertIn("render-only", self.pm["hazard_draw"]["sink"])

    def test_gore_only_on_the_gore_layer_and_molten_has_no_blood(self):
        for grid in (self.poses, self.parts):
            for n, frame in enumerate(grid["frames"]):
                for layer in ("body", "molten"):
                    self.assertFalse(set("".join(frame["cels"][layer])) & set("BbP"), (n, layer))
                self.assertFalse(set("".join(frame["cels"]["gore"])) - set(".oBbPe"), n)
        for e in self.qm["parts"]:
            if e["class"] == "molten" and e["id"] != "gland":
                self.assertEqual("".join(self.parts["frames"][e["frame"]]["cels"]["gore"]).strip("."), "", e["id"])
            if e["class"] == "blood":
                self.assertEqual("".join(self.parts["frames"][e["frame"]]["cels"]["body"]).strip("."), "", e["id"])
                self.assertEqual("".join(self.parts["frames"][e["frame"]]["cels"]["molten"]).strip("."), "", e["id"])

    def test_composited_palette_is_at_most_20(self):
        for grid in (self.poses, self.parts):
            used = set()
            for n in range(len(grid["frames"])):
                used |= set(flat(grid, n).values())
            self.assertLessEqual(len(used), 20)
            self.assertLessEqual(len(grid["palette"]), 20)

    def test_head_dead_is_head_outside_the_eye_and_tongue(self):
        head, dead = flat(self.parts, 0), flat(self.parts, 3)
        eye = {(x, y) for x, y in set(head) | set(dead) if 8 <= x <= 16 and 10 <= y <= 18}
        tongue = {(x, y) for x, y in set(head) | set(dead) if x >= 21 and y >= 22}
        diff = {p for p in set(head) | set(dead) if head.get(p) != dead.get(p)}
        self.assertTrue(diff)
        self.assertLessEqual(diff, eye | tongue)
        self.assertGreaterEqual("".join(self.parts["frames"][3]["cels"]["body"]).count("g"), 2)   # the tongue

    def test_far_legs_are_ramp_shifted_near_legs(self):
        ramp = {"L": "l", "l": "m", "m": "s", "s": "a"}
        for near, far in (("leg_fore", "leg_fore_far"), ("leg_hind", "leg_hind_far")):
            n, f = self.parts["frames"][self.idx(near)]["cels"], self.parts["frames"][self.idx(far)]["cels"]
            self.assertEqual(f["body"], ["".join(ramp.get(c, c) for c in row) for row in n["body"]])
            self.assertEqual(f["gore"], n["gore"])
            self.assertEqual(self.entries[far]["anchors"], self.entries[near]["anchors"])

    def test_rig_parts_have_cut_faces_on_gore_and_closed_outlines(self):
        for pid in ("torso_front", "torso_rump", "leg_hind", "ear", "jaw", "gland"):
            gore = self.parts["frames"][self.idx(pid)]["cels"]["gore"]
            self.assertTrue(any("b" in row for row in gore), pid)
        for pid in ("torso_front", "torso_rump"):
            self.assertEqual(set(self.entries[pid]["anchors"]) & {"waist"}, {"waist"})
        self.assertEqual(set(self.entries["torso_front"]["anchors"]), {"waist", "neck"})
        self.assertEqual(set(self.entries["torso_rump"]["anchors"]), {"waist", "tail"})
        self.assertEqual(set(self.entries["leg_hind"]["anchors"]), {"hip", "hoof"})

    def test_molten_parts(self):
        gland = self.parts["frames"][self.idx("gland")]["cels"]["molten"]
        self.assertGreaterEqual("".join(gland).count("Y"), 2)
        self.assertTrue(set("".join(gland)) >= set("YOR."))
        self.assertTrue(self.entries["gland"]["molten_layer"] and self.entries["gland"]["gore_layer"])
        self.assertEqual(self.entries["decal_molten"]["swap_on_ttl"], "decal_molten_crust")
        crust = "".join(self.parts["frames"][self.idx("decal_molten_crust")]["cels"]["molten"])
        self.assertEqual(crust.count("r"), 2)
        for pid in ("decal_molten", "decal_molten_crust"):
            xs = [x for x, _y in flat(self.parts, self.idx(pid))]
            self.assertEqual(max(xs) - min(xs) + 1, 10)
            self.assertTrue(self.entries[pid]["floor_decal"])

    def pool_cols(self, pid: str) -> set[tuple[int, int]]:
        import author_gob02_death as g
        origin = {"decal_pool_3": g.POOL3_LEFT_X, "decal_pool_3_r": g.POOL3_RIGHT_X}.get(pid)
        out = set()
        for (x, y) in flat(self.parts, self.idx(pid)):
            u = x - 16 if origin is None else (x - origin) - g.POOL3_WIDTH // 2 + (0 if pid == "decal_pool_3" else g.POOL3_WIDTH // 2)
            out.add((u, y))
        return out

    def test_decals_are_the_goblin_bitmaps_and_nest(self):
        import author_gob02_death as g
        gob = load(ROOT / "assets/source/characters/goblin/death/goblin_a_death_parts.grid.json")
        gids = [e["id"] for e in load(GOB_PARTS_MANIFEST)["death_parts"]["parts"]]
        for pid in ("decal_splat_s", "decal_splat_m", "decal_pool_1", "decal_pool_2", "decal_pool_3", "decal_pool_3_r",
                    "decal_wall_spatter", "decal_wall_spatter_b", "decal_smear"):
            theirs = {p: ("b" if c == "R" else c) for p, c in opaque(gob["frames"][gids.index(pid)]["cels"]["gore"]).items()}
            mine = opaque(self.parts["frames"][self.idx(pid)]["cels"]["gore"])
            self.assertEqual(mine, theirs, pid)
        p1, p2 = self.pool_cols("decal_pool_1"), self.pool_cols("decal_pool_2")
        p3 = self.pool_cols("decal_pool_3") | self.pool_cols("decal_pool_3_r")
        self.assertTrue(p1 < p2 < p3)
        us = [u for u, _y in p3]
        self.assertEqual(max(us) - min(us) + 1, g.POOL3_WIDTH)
        self.assertEqual(self.entries["decal_pool_3"]["pair"], "decal_pool_3_r")
        self.assertEqual(self.entries["decal_pool_3"]["span"], [38, 4])
        flagged = {pid for pid, e in self.entries.items() if e.get("floor_decal")}
        self.assertEqual(flagged, {"decal_splat_s", "decal_splat_m", "decal_pool_1", "decal_pool_2", "decal_pool_3",
                                   "decal_pool_3_r", "decal_smear", "decal_molten", "decal_molten_crust"})

    def test_sizes_of_the_new_parts(self):
        limits = {"tooth": (4, 5), "wool_tuft_a": (6, 5), "wool_tuft_b": (5, 5), "gib_flesh_a": (5, 4),
                  "gib_flesh_b": (4, 3), "gib_bone": (5, 3), "gib_rib": (7, 4), "drop_blood": (2, 2),
                  "drop_molten": (2, 2), "decal_smear": (18, 2)}
        for pid, (w, h) in limits.items():
            x0, y0, x1, y1 = pg._bbox(pg.composite_keys(self.parts, self.idx(pid)))
            self.assertEqual((x1 - x0 + 1, y1 - y0 + 1), (w, h), pid)
        for pid in ("torso_front", "torso_rump", "leg_hind", "jaw", "ear", "gland"):
            x0, y0, x1, y1 = pg._bbox(pg.composite_keys(self.parts, self.idx(pid)))
            self.assertLessEqual(max(x1 - x0 + 1, y1 - y0 + 1), 28, pid)

    def test_gif_frames_keep_the_body_colour_and_the_blood_red(self):
        from PIL import Image
        gifs = sorted(EVIDENCE.glob("**/gif_*.gif"))
        self.assertTrue(gifs, "no evidence GIFs")
        want = {tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in (FUR_MID, FUR_BLOOD)}
        for path in gifs:
            with Image.open(path) as im:
                for n in range(getattr(im, "n_frames", 1)):
                    im.seek(n)
                    colours = {c for _count, c in im.convert("RGB").getcolors(1 << 16)}
                    self.assertLessEqual(want, colours, (path.name, n))

    def test_spawner_hints_are_present(self):
        hints = self.pm["spawner_hints"]
        self.assertEqual(set(hints), {"crush", "decapitation", "slam", "explosive"})
        self.assertEqual(hints["crush"]["ticks"]["pool_3"], 60)
        self.assertEqual(hints["crush"]["ticks"]["decal_molten_to_crust"], 240)
        self.assertTrue(hints["slam"]["no_pool_3"])
        self.assertEqual(hints["decapitation"]["ticks"]["pool_3"], 100)


if __name__ == "__main__":
    unittest.main()
