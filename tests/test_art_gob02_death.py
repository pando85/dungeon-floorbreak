"""GOB-02-DEATH (goblin A crush and decapitation poses, death-parts sheet): spec rules the shared pixel-art toolkit does
not cover. The toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/goblin/death/goblin_a_death_h36.pixelart.json`` and ``goblin_a_death_parts.pixelart.json``.
See ``docs/02-art/gob-02-death-spec.md`` section 10."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_gob02_death as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN_A = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
GOBLIN_B = ROOT / "assets/source/characters/goblin/goblin_b_h36.grid.json"
POSE_FIXTURE = ROOT / "tests/fixtures/gob-02-death_keys.json"
PART_FIXTURE = ROOT / "tests/fixtures/gob-02-death-parts_keys.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def opaque(rows: list[str]) -> set[tuple[int, int]]:
    return {(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."}


def without_gore(grid: dict) -> dict:
    return ad.strip_gore(grid)


class Gob02DeathSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.goblin = load(GOBLIN_A)

    def test_committed_sources_match_the_generator(self):
        self.assertEqual(ad.POSES.read_text(encoding="utf-8"), ad.dump(ad.poses_grid()))
        self.assertEqual(ad.PARTS.read_text(encoding="utf-8"), ad.dump(ad.parts_grid()))
        for grid_path, ase in ((ad.POSES, ad.POSES_ASE), (ad.PARTS, ad.PARTS_ASE)):
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(load(grid_path)))

    def test_keys_are_frozen(self):
        self.assertEqual(sorted(load(POSE_FIXTURE)["keys"], key=int), ["0", "2", "3", "5"])
        self.assertEqual(sorted(load(PART_FIXTURE)["keys"], key=int), ["0", "1", "2"])

    def test_gob01_frame_contract(self):
        self.assertEqual(self.poses["canvas"], self.goblin["canvas"])
        self.assertEqual(self.poses["pivot"], self.goblin["pivot"])
        self.assertEqual([t["name"] for t in self.poses["tags"]],
                         ["crush_contact", "crushed_remains", "decap_contact", "headless_collapse"])
        for n in range(len(self.poses["frames"])):       # every pose is grounded on row 55
            box = pg._bbox(pg.composite_keys(self.poses, n))
            self.assertEqual(box[3], 55, n)

    def test_palette_is_goblin_floor01_v0_plus_one_blood_highlight(self):
        source = {p["key"]: p["rgba"] for p in self.goblin["palette"] + load(GOBLIN_B)["palette"]}
        for grid in (self.poses, self.parts):
            self.assertLessEqual(len(grid["palette"]), 20)
            new = [p["key"] for p in grid["palette"] if source.get(p["key"]) != p["rgba"]]
            self.assertEqual(new, ["P"])
            self.assertNotIn("e", [p["key"] for p in grid["palette"]])   # no living glare in a death atlas

    def test_blood_keys_live_on_the_gore_layer_only(self):
        for grid in (self.poses, self.parts):
            for n, f in enumerate(grid["frames"]):
                for layer in ("body", "weapon"):
                    used = set("".join(f["cels"][layer]))
                    self.assertFalse(used & ad.GORE_ONLY, (n, layer))

    def test_presets_1_and_2_are_the_same_pixels_with_gore_hidden(self):
        """Hiding the gore layer leaves a clean, outlined sprite: no isolated pixels, under 10 orphans, and every
        coloured body pixel is closed by outline or another opaque pixel (no open wound to transparency)."""
        for grid in (self.poses, self.parts):
            bare = without_gore(grid)
            accents = {e["key"] for e in bare["palette"] if e.get("accent")}
            for n in range(len(bare["frames"])):
                flat = pg.composite_keys(bare, n)
                orphans, isolated = pg._stray_pixels(flat, accents)
                self.assertEqual(len(isolated), 0, n)
                self.assertLess(len(orphans), 10, n)
                h, w = len(flat), len(flat[0])
                for y in range(h):
                    for x in range(w):
                        if flat[y][x] in ".K":
                            continue
                        for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                            inside = 0 <= nx < w and 0 <= ny < h          # the canvas edge counts as closed
                            self.assertFalse(inside and flat[ny][nx] == ".", (n, x, y))

    def test_decap_and_headless_reuse_gob01_pixels(self):
        f16, f25 = self.goblin["frames"][16]["cels"], self.goblin["frames"][25]["cels"]
        decap, headless = self.poses["frames"][3]["cels"], self.poses["frames"][5]["cels"]
        self.assertEqual(decap["weapon"], f16["weapon"])
        self.assertEqual(headless["weapon"], f25["weapon"])
        self.assertEqual(decap["body"][38:], f16["body"][38:])           # recoiling body from the chest down
        for y in range(56):                                               # corpse body right of the neck
            self.assertEqual(headless["body"][y][28:], f25["body"][y][28:], y)

    def test_no_head_on_the_beheaded_poses(self):
        for n in (3, 5):
            body = "".join(self.poses["frames"][n]["cels"]["body"])
            self.assertNotIn("w", body)                                   # no eyes, no teeth: the head is a part
            self.assertNotIn("o", body)

    def test_crush_contact_is_pressed_flat_under_the_stone(self):
        flat = pg.composite_keys(self.poses, 0)
        self.assertTrue(all(ch == "." for row in flat[:33] for ch in row))
        top = flat[33]
        self.assertGreaterEqual(top.count("K"), 36)                       # a long, straight stone-contact line
        self.assertEqual(self.poses["frames"][0]["cels"]["weapon"], ["." * 64] * 56)   # the club has left the hand

    def test_preset4_crush_carries_its_own_blood(self):
        """Gate fix: the crush impact and the remains show the squeeze-out on the pose itself, not only via the spawner
        (at least 40 gore pixels with at least 3 wet highlights on frame 0; at least 40 on frame 2)."""
        for n, lo, wet in ((0, 40, 3), (2, 40, 1)):
            gore = "".join(self.poses["frames"][n]["cels"]["gore"])
            self.assertGreaterEqual(sum(ch != "." for ch in gore), lo, n)
            self.assertGreaterEqual(gore.count("P"), wet, n)
        floor = self.poses["frames"][2]["cels"]["gore"][55]
        run = max(len(r) for r in floor.split("."))
        self.assertGreaterEqual(run, 16)                                  # one continuous seep, not specks

    def test_crushed_remains_keep_the_corpse_head(self):
        """Not a paper cut-out: the GOB-01 corpse head (X eyes, ear up, tongue) is reused pixel for pixel."""
        f25 = opaque(self.goblin["frames"][25]["cels"]["body"])
        rem = self.poses["frames"][2]["cels"]["body"]
        src = self.goblin["frames"][25]["cels"]["body"]
        head = [(x, y) for x, y in f25 if x <= 20 and y <= 50]
        self.assertGreater(len(head), 50)
        for x, y in head:
            self.assertEqual(rem[y][x], src[y][x], (x, y))

    def test_parts_fit_their_cell_and_stay_small(self):
        for n in range(len(self.parts["frames"])):
            box = pg._bbox(pg.composite_keys(self.parts, n))
            self.assertGreaterEqual(min(box[0], box[1]), 1)
            self.assertLessEqual(max(box[2], box[3]), 30)
            self.assertLessEqual(box[2] - box[0] + 1, 28)
            self.assertLessEqual(box[3] - box[1] + 1, 28)

    def test_club_keeps_the_gob01_pineapple(self):
        f0 = self.goblin["frames"][0]["cels"]["weapon"]
        club = self.parts["frames"][2]["cels"]["weapon"]
        dx, dy = ad.CLUB_SHIFT
        for y in range(23, 36):
            for x, ch in enumerate(f0[y]):
                if ch in "FGHVv":
                    self.assertEqual(club[y + dy][x + dx], ch, (x, y))
        self.assertEqual("".join(self.parts["frames"][2]["cels"]["gore"]).strip("."), "")


MANIFEST_POSES = ROOT / "assets/manifests/goblin_a_death_h36.json"
MANIFEST_PARTS = ROOT / "assets/manifests/goblin_a_death_parts.json"
WOUND_CUTS = {"head", "torso_upper", "head_dead", "torso_lower", "arm_near", "arm_far", "leg_near", "leg_far", "ear"}


def gore_px(grid: dict, n: int) -> set[tuple[int, int]]:
    return opaque(grid["frames"][n]["cels"]["gore"])


def body_px(grid: dict, n: int) -> set[tuple[int, int]]:
    return opaque(grid["frames"][n]["cels"]["body"]) | opaque(grid["frames"][n]["cels"]["weapon"])


class Gob02DeathExecutorTests(unittest.TestCase):
    """The executor pass: in-betweens, parts 3 to 23, anchors and spawn metadata (spec sections 7 and 9)."""

    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.goblin = load(GOBLIN_A)
        cls.pm = load(MANIFEST_PARTS)
        cls.qm = load(MANIFEST_POSES)
        cls.ids = ad.PART_IDS
        cls.entries = {e["id"]: e for e in cls.pm["death_parts"]["parts"]}

    def idx(self, pid: str) -> int:
        return self.ids.index(pid)

    def test_part_list_and_tags_follow_the_spec_order(self):
        self.assertEqual(len(self.parts["frames"]), 24)
        self.assertEqual(self.ids[:12], ["head", "torso_upper", "club", "head_dead", "torso_lower", "arm_near", "arm_far",
                                         "leg_near", "leg_far", "club_stick", "pineapple", "ear"])
        self.assertEqual(self.ids[16:], ["drop_blood", "decal_splat_s", "decal_splat_m", "decal_pool_1", "decal_pool_2",
                                         "decal_pool_3", "decal_wall_spatter", "decal_smear"])
        self.assertEqual([t["name"] for t in self.parts["tags"]], self.ids)
        self.assertEqual(self.pm["animation_tags"], self.ids)

    def test_in_betweens_are_new_pixels_and_keep_the_pose_contract(self):
        frames = [f["cels"] for f in self.poses["frames"]]
        for n in (1, 4):
            self.assertNotEqual(frames[n], frames[n - 1] if n == 1 else frames[3])
        # frame 1 keeps frame 0's floor rows and legs; the popped eye is white
        self.assertEqual(frames[1]["body"][50][:30], frames[0]["body"][50][:30])
        self.assertEqual(frames[1]["body"][39][40], "w")
        # frame 4 is a kneeling, lower buckle: its jet is shorter than the key's, the stump face moved with the neck
        self.assertGreater(min(y for y, r in enumerate(frames[3]["gore"]) if r.strip(".")),
                           -1)
        top3 = min(y for y, r in enumerate(frames[3]["gore"]) if r.strip("."))
        top4 = min(y for y, r in enumerate(frames[4]["gore"]) if r.strip("."))
        self.assertGreater(top4, top3)

    def test_pose_anchors_are_in_the_frame_and_follow_the_offsets(self):
        poses = {f["index"]: f["anchors"] for f in self.qm["death_poses"]["frames"]}
        self.assertEqual(sorted(poses), [0, 1, 2, 3, 4, 5])
        for n, anchors in poses.items():
            for name, (x, y) in anchors.items():
                self.assertTrue(0 <= x < 64 and 0 <= y < 56, (n, name))
        self.assertEqual(poses[1]["hit_origin"], [28, 36])
        self.assertEqual(poses[1]["mouth"], [49, 46])
        self.assertEqual(poses[4]["neck"], [21, 35])
        self.assertEqual(poses[4]["jet_base"], [21, 34])
        self.assertEqual(poses[3]["neck"], [23, 31])
        self.assertEqual(poses[5]["pool_origin"], [12, 55])

    def test_committed_manifests_match_the_generator(self):
        ad.parts_grid()
        self.assertEqual(MANIFEST_POSES.read_text(encoding="utf-8"), json.dumps(ad.pose_manifest(), indent=2) + "\n")
        self.assertEqual(MANIFEST_PARTS.read_text(encoding="utf-8"), json.dumps(ad.parts_manifest(), indent=2) + "\n")

    def test_every_part_has_schema_complete_metadata(self):
        required = {"id", "frame", "cell", "pivot", "anchors", "frame0_origin", "class", "material", "presets",
                    "gore_layer", "mass", "spin", "settle", "bounce", "ttl"}
        self.assertEqual(len(self.entries), 24)
        for i, pid in enumerate(self.ids):
            e = self.entries[pid]
            self.assertLessEqual(required, e.keys(), pid)
            self.assertEqual((e["frame"], e["cell"], e["pivot"]), (i, [32, 32], [16, 16]), pid)
            self.assertIn(e["class"], ("wound", "clean", "gib", "blood"), pid)
            self.assertIn(e["mass"], ("light", "medium", "heavy"), pid)
            self.assertIn(e["spin"], ("none", "slow", "fast"), pid)
            self.assertIn(e["settle"], ("roll", "flop", "clatter", "flutter", "bounce", "static"), pid)
            self.assertIn(e["ttl"], ("keep", "long", "short"), pid)
            self.assertTrue(set(e["presets"]) <= {1, 2, 3, 4} and e["presets"], pid)
            self.assertTrue(e["anchors"], pid)
            for name, (x, y) in e["anchors"].items():
                self.assertTrue(0 <= x < 32 and 0 <= y < 32, (pid, name))
        for pid in self.ids[3:12]:
            self.assertIsNotNone(self.entries[pid]["frame0_origin"], pid)     # spec: parts 3 to 11 are measured
        self.assertEqual(self.entries["head"]["swap_on_first_contact"], "head_dead")
        self.assertEqual(self.entries["drop_blood"]["on_contact"], "decal_splat_s")

    def test_anchors_sit_on_the_pixels_they_name(self):
        for pid, name in (("arm_near", "shoulder"), ("arm_far", "shoulder"), ("leg_near", "hip"), ("leg_far", "hip"),
                          ("ear", "base"), ("torso_lower", "hip_far"), ("torso_lower", "hip_near")):
            x, y = self.entries[pid]["anchors"][name]
            self.assertIn((x, y), gore_px(self.parts, self.idx(pid)), (pid, name))      # on the cut face
        for pid in ("leg_near", "leg_far"):
            x, y = self.entries[pid]["anchors"]["foot"]
            self.assertIn((x, y), body_px(self.parts, self.idx(pid)), pid)
        x, y = self.entries["club_stick"]["anchors"]["grip"]
        self.assertIn((x, y), opaque(self.parts["frames"][self.idx("club_stick")]["cels"]["weapon"]))

    def test_gore_layer_separation_on_every_new_part(self):
        """Wound parts: the cut is on gore, the body stays a closed bloodless sprite. Clean parts and gib_bone have no
        gore. Blood parts are gore only. ``gore_layer`` in the manifest says which."""
        for i, pid in enumerate(self.ids):
            cels = self.parts["frames"][i]["cels"]
            gore, body, weapon = opaque(cels["gore"]), opaque(cels["body"]), opaque(cels["weapon"])
            cls = self.entries[pid]["class"]
            self.assertEqual(self.entries[pid]["gore_layer"], bool(gore), pid)
            self.assertFalse(body & weapon, pid)
            if cls == "blood":
                self.assertFalse(body or weapon, pid)
                self.assertTrue(gore, pid)
            elif cls == "clean" or pid == "gib_bone":
                self.assertFalse(gore, pid)
            else:
                self.assertTrue(gore and body, pid)
            if pid in WOUND_CUTS or pid.startswith("gib_flesh"):
                self.assertTrue(set("R") & set("".join(cels["gore"])), pid)         # a meat colour on every cut
            for key in "BP":
                self.assertEqual(sum(row.count(key) for layer in ("body", "weapon") for row in cels[layer]), 0, pid)
            if cls in ("wound", "gib") and body:
                bare = ad.strip_gore(self.parts)
                self.assertTrue(pg._bbox(pg.composite_keys(bare, i)), pid)          # hidden gore leaves a sprite

    def test_gore_stays_where_the_cut_is(self):
        """A cut face touches the body (it is a wound on that part), never floats beside it."""
        for pid in WOUND_CUTS | {"gib_flesh_a", "gib_flesh_b"}:
            i = self.idx(pid)
            gore, body = gore_px(self.parts, i), body_px(self.parts, i)
            self.assertTrue(any((x + dx, y + dy) in body for x, y in gore for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
                                if (x, y) not in body) or gore & body, pid)
            self.assertGreaterEqual(len(gore & body), 1 if pid.startswith("gib") else 3, pid)

    def test_part_sizes_stay_within_the_spec_table(self):
        sizes = {"head_dead": (27, 15), "torso_lower": (18, 7), "arm_near": (9, 9), "leg_near": (10, 9), "leg_far": (7, 9),
                 "club_stick": (15, 11), "pineapple": (13, 9), "ear": (7, 5), "decal_splat_s": (6, 2),
                 "decal_splat_m": (12, 3), "decal_pool_1": (12, 2), "decal_pool_2": (20, 3), "decal_wall_spatter": (22, 18),
                 "decal_smear": (18, 2)}
        for pid, (w, h) in sizes.items():
            box = pg._bbox(pg.composite_keys(self.parts, self.idx(pid)))
            got = (box[2] - box[0] + 1, box[3] - box[1] + 1)
            self.assertLessEqual(abs(got[0] - w), 5, (pid, got))
            self.assertLessEqual(abs(got[1] - h), 5, (pid, got))
        for pid in ("gib_flesh_a", "gib_flesh_b", "gib_bone", "gib_rag", "drop_blood"):
            box = pg._bbox(pg.composite_keys(self.parts, self.idx(pid)))
            self.assertLessEqual(max(box[2] - box[0], box[3] - box[1]) + 1, 6, pid)      # nothing large lands at 1x
        pool3 = pg._bbox(pg.composite_keys(self.parts, self.idx("decal_pool_3")))
        self.assertLessEqual(pool3[2] - pool3[0] + 1, 28)                                # the 28 px rule (spec lists 30)

    def test_head_dead_is_the_head_outside_the_eyes_and_the_tongue(self):
        head, dead = self.parts["frames"][0]["cels"], self.parts["frames"][3]["cels"]
        edited = {(x, y) for x in range(16, 23) for y in range(14, 17)} | {(x, y) for x in range(19, 23) for y in range(19, 25)}
        for y in range(32):
            for x in range(32):
                if (x, y) not in edited and (head["body"][y][x] != dead["body"][y][x]):
                    self.assertEqual(dead["body"][y][x], "K", (x, y))                    # only outline closure differs
        self.assertEqual(head["gore"], dead["gore"])
        self.assertEqual(dead["body"][14][16:19], "KwK")
        self.assertEqual(dead["body"][15][20:23], "wKw")

    def test_arm_far_is_the_arm_near_ramp_shifted(self):
        near, far = self.parts["frames"][self.idx("arm_near")]["cels"], self.parts["frames"][self.idx("arm_far")]["cels"]
        ramp = {"l": "m", "m": "s", "s": "a"}
        for y in range(32):
            for x in range(32):
                n = near["body"][y][x]
                self.assertEqual(far["body"][y][x], ramp.get(n, n), (x, y))
        self.assertEqual(near["gore"], far["gore"])

    def test_frame0_pieces_are_unchanged_gob01_pixels(self):
        f0 = self.goblin["frames"][0]["cels"]["body"]
        for pid, rect in (("arm_near", ad.ARM_RECT), ("leg_near", ad.LEG_NEAR_RECT), ("leg_far", ad.LEG_FAR_RECT)):
            ox, oy = self.entries[pid]["frame0_origin"]
            x0, y0, x1, y1 = rect
            body = self.parts["frames"][self.idx(pid)]["cels"]["body"]
            for y in range(y0, y1 + 1):
                for x in range(x0, x1 + 1):
                    if f0[y][x] != ".":
                        self.assertEqual(body[y - oy + 16][x - ox + 16], f0[y][x], (pid, x, y))
        f25w = self.goblin["frames"][25]["cels"]["weapon"]
        fruit = self.parts["frames"][self.idx("pineapple")]["cels"]["weapon"]
        for y in range(47, 56):
            for x in range(50, 63):
                self.assertEqual(fruit[y - 35][x - 40], f25w[y][x], (x, y))

    def test_club_stick_is_the_club_shaft_alone(self):
        club = opaque(self.parts["frames"][2]["cels"]["weapon"])
        stick = self.parts["frames"][self.idx("club_stick")]["cels"]["weapon"]
        self.assertFalse(set("".join(stick)) & set("FGHVv"))                          # no fruit, no crown
        self.assertEqual("".join(self.parts["frames"][self.idx("club_stick")]["cels"]["gore"]).strip("."), "")
        self.assertGreaterEqual("".join(stick).count("y"), 8)
        self.assertGreater(len(club), len(opaque(stick)))

    def test_pools_grow_by_frame_swap(self):
        pools = [pg.composite_keys(self.parts, self.idx(f"decal_pool_{k}")) for k in (1, 2, 3)]
        sets = [{(x, y) for y, r in enumerate(p) for x, ch in enumerate(r) if ch != "."} for p in pools]
        self.assertTrue(sets[0] < sets[1] < sets[2])
        for pid in ("decal_pool_1", "decal_pool_2", "decal_pool_3"):
            self.assertGreaterEqual("".join(self.parts["frames"][self.idx(pid)]["cels"]["gore"]).count("P"), 1)

    def test_preset_classes_follow_the_section_5_table(self):
        presets = {pid: set(e["presets"]) for pid, e in self.entries.items()}
        self.assertEqual(presets["head"], {1, 2, 3, 4})
        for pid in ("gib_flesh_a", "gib_flesh_b"):
            self.assertEqual(presets[pid], {3, 4})                                    # no gibs at 1 or 2
        self.assertEqual(presets["gib_bone"], {4})                                    # section 5: bone gibs only at 4
        for pid, e in self.entries.items():
            if e["class"] == "clean" and pid in ("club", "gib_rag"):
                self.assertEqual(presets[pid], {1, 2, 3, 4})
        self.assertEqual(presets["decal_wall_spatter"], {4})
        self.assertEqual(presets["decal_smear"], {4})
        self.assertEqual(presets["decal_pool_3"], {4})
        self.assertEqual(presets["drop_blood"], {2, 3, 4})


if __name__ == "__main__":
    unittest.main()
