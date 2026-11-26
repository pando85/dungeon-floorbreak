"""ENG-02 Goblin Engineer completion (docs/02-art/eng-02-spec.md): the two frozen get-up keys, ENG-01 atlas frames 18 to 21,
the slammed sheet, the death poses and the parts sheet (E2-1 to E2-5). The toolkit validators run with
``python3 -m scripts.pixelart validate assets/source/characters/goblin/engineer/<name>.pixelart.json`` (tested below)."""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as e1  # noqa: E402
import author_eng02 as e2  # noqa: E402

FIXTURE = ROOT / "tests/fixtures/eng-02_keys.json"
CONFIG = ROOT / "assets/source/characters/goblin/engineer/eng_02_keys.pixelart.json"


def merged(cels: dict[str, list[str]]) -> list[str]:
    rows = [list(r) for r in next(iter(cels.values()))]
    for layer in cels.values():
        for y, r in enumerate(layer):
            for x, ch in enumerate(r):
                if ch != ".":
                    rows[y][x] = ch
    return ["".join(r) for r in rows]


class Eng02KeyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grid = json.loads(e2.GRID.read_text(encoding="utf-8"))
        cls.sit = merged(cls.grid["frames"][0]["cels"])
        cls.shove = merged(cls.grid["frames"][1]["cels"])

    def test_committed_grid_matches_the_generator(self):
        self.assertEqual(e2.GRID.read_text(encoding="utf-8"), e2.dump(e2.frames_grid()))
        self.assertTrue(e2.ASE.is_file())

    def test_both_keys_are_frozen(self):
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(fixture["asset_id"], "eng-02-keys")
        self.assertEqual(sorted(fixture["keys"], key=int), ["0", "1"])
        cfg = json.loads(CONFIG.read_text(encoding="utf-8"))
        self.assertEqual(cfg["rules"]["frozen_keys"]["frames"], [0, 1])

    def test_canvas_pivot_palette_match_eng01(self):
        self.assertEqual(self.grid["canvas"], [64, 56])
        self.assertEqual(self.grid["pivot"], [28, 55])
        self.assertEqual([p["key"] for p in self.grid["palette"]], [k for k, *_ in e1.PALETTE])

    def test_shove_body_is_the_literal_hero_key(self):
        for y, row in e2.KEY_SHOVE.items():
            self.assertEqual(self.shove[y], row, y)
        self.assertFalse(any(r.strip(".") for r in self.shove[:16]))

    def test_getup_timing_fills_the_22_tick_window(self):
        self.assertEqual(sum(e2.TICKS), 22)
        self.assertEqual(e2.ATLAS_INDEX, (18, 19))
        self.assertEqual([f["duration_ms"] for f in self.grid["frames"]], [e2.duration_ms(t) for t in e2.TICKS])

    def test_sit_hides_the_eyes_and_drops_the_wrench(self):
        self.assertEqual(sum(r.count("e") for r in self.sit), 0)
        self.assertGreaterEqual(sum(r.count("w") for r in self.sit), 2)
        prop = self.grid["frames"][0]["cels"]["prop"]
        self.assertFalse(any(r.strip(".") for r in prop))

    def test_shove_carries_the_wrench_on_the_prop_layer(self):
        prop = self.grid["frames"][1]["cels"]["prop"]
        self.assertTrue(any(r.strip(".") for r in prop))
        self.assertGreaterEqual(sum(r.count("e") for r in self.shove), 3)

    def test_pot_part_and_flat_pot_rules(self):
        pot = e2.pot_part()
        xs, ys = [x for x, _ in pot], [y for _, y in pot]
        self.assertEqual((min(xs), max(xs), min(ys), max(ys)), (1, 30, 12, 20))   # whole pan inside the 32 x 32 cell
        self.assertEqual(sum(ch == "7" for ch in pot.values()), 1)
        flat = e2.flat_pot()
        fys = {y for _, y in flat}
        self.assertEqual(fys, {0, 1, 2, 3})                                       # 4-row pancake
        fxs = [x for x, _ in flat]
        self.assertLessEqual(max(fxs) - min(fxs) + 1, 30)                          # iron does not stretch
        self.assertEqual(sum(ch == "7" for ch in flat.values()), 0)

    def test_death_palette_adds_three_gore_keys(self):
        keys = [k for k, *_ in e2.DEATH_PALETTE]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual(keys[-3:], ["R", "B", "P"])
        self.assertLessEqual(len(keys), 20)


# =========================================================================== execution: atlas, sheets, parts
ENG = ROOT / "assets/source/characters/goblin/engineer"
DEATH_A = ROOT / "assets/source/characters/goblin/death"
ENG01_FRAMES_0_17_SHA256 = "07a4dff7fd5801967c42b543d93686111a19411a9b78761f71d95d2ff8140ba7"
CONFIGS = ["eng_01", "eng_02_keys", "eng_slammed_h36", "eng_death_h36", "eng_death_parts"]


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def layer_pix(cels: dict, layer: str) -> dict:
    return {(x, y): ch for y, row in enumerate(cels.get(layer, [])) for x, ch in enumerate(row) if ch != "."}


class Eng02AtlasTests(unittest.TestCase):
    """E2-1 and E2-2: ENG-01 frames 18 to 21."""

    @classmethod
    def setUpClass(cls):
        cls.grid = load(e1.GRID)
        cls.keys = load(e2.GRID)
        cls.frames = [merged(f["cels"]) for f in cls.grid["frames"]]

    def test_atlas_matches_the_generator(self):
        self.assertEqual(e1.GRID.read_text(encoding="utf-8"), e1.dump(e1.frames_grid()))
        self.assertEqual(len(self.grid["frames"]), 22)

    def test_e2_1_frames_0_to_17_are_unchanged_and_18_19_are_the_frozen_keys(self):
        sha = hashlib.sha256(json.dumps(self.grid["frames"][:18], sort_keys=True).encode()).hexdigest()
        self.assertEqual(sha, ENG01_FRAMES_0_17_SHA256)
        for atlas, key in ((18, 0), (19, 1)):
            self.assertEqual(self.grid["frames"][atlas]["cels"], self.keys["frames"][key]["cels"], atlas)   # both layers
        self.assertEqual([self.grid["frames"][n]["duration_ms"] for n in (18, 19)], [e2.duration_ms(10), e2.duration_ms(12)])

    def test_tags_and_ticks_of_the_new_frames(self):
        tags = {t["name"]: (t["from"], t["to"]) for t in self.grid["tags"]}
        self.assertEqual(tags["getup"], (18, 19))
        self.assertEqual(tags["attack_ram_run"], (20, 21))
        manifest = load(ROOT / "assets/manifests/eng_01.json")
        ticks = [f["ticks"] for f in manifest["eng"]["frames"]]
        self.assertEqual(ticks[18:], [10, 12, 4, 4])
        self.assertEqual(manifest["eng"]["eng02"]["ram_run_cycle"], [12, 20, 21, 20])

    def test_e2_2_ram_run_frames(self):
        for n, tops in ((20, (20, 22)), (21, (21, 23))):
            rows = self.frames[n]
            ys = [y for y, r in enumerate(rows) if r.strip(".")]
            xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
            self.assertEqual(max(ys), 55, n)
            self.assertGreaterEqual(max(xs), 51, n)
            self.assertTrue(tops[0] <= min(ys) <= tops[1], (n, min(ys)))
            flat = "".join(rows)
            self.assertGreaterEqual(flat.count("e"), 3, n)
            self.assertEqual(flat.count("7"), 1, n)
            self.assertGreaterEqual(flat.count("5"), 40, n)

    def test_e2_2_only_the_legs_differ_from_frame_12(self):
        f12, f20, f21 = self.frames[12], self.frames[20], self.frames[21]
        self.assertEqual(f21[:47], f12[:47])                               # torso, head, pot, wrench: frame 12's rows 0 to 46
        self.assertEqual(f20[:46], f12[1:47])                              # the whole 1-row lift
        for n in (20, 21):
            self.assertNotEqual(self.frames[n], f12)
        self.assertNotEqual(f20, f21)

    def test_ram_run_legs_are_the_walk_legs(self):
        # the leg rows (below the hem) are the ENG-01 walk 7 (frame 20) and walk 6 (frame 21) pixels, thighs extended up
        for n, walk, hem in ((20, 7, 46), (21, 6, 48)):
            legs = [row for row in self.frames[walk][hem + 2:]]
            self.assertEqual(self.frames[n][hem + 2 + (0 if n == 21 else 0):], legs, n)


class Eng02SheetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.slam = load(e2.SLAMMED)
        cls.death = load(e2.DEATH)
        cls.parts = load(e2.PARTS_GRID)
        cls.slam_a = load(DEATH_A / "goblin_a_slammed_h36.grid.json")
        cls.death_a = load(DEATH_A / "goblin_a_death_h36.grid.json")
        cls.parts_a = load(DEATH_A / "goblin_a_death_parts.grid.json")
        cls.dm = load(ROOT / "assets/manifests/eng_death_h36.json")

    # ---- regenerate and compare
    def test_sheets_and_manifests_match_the_generator(self):
        for grid_path, ase, grid, manifest_path, manifest in e2.outputs():
            self.assertEqual(grid_path.read_text(encoding="utf-8"), e2.dump(grid), grid_path.name)
            self.assertEqual(manifest_path.read_text(encoding="utf-8"), json.dumps(manifest, indent=2) + "\n")
            self.assertTrue(ase.is_file(), ase.name)

    def test_validators_pass_on_every_config(self):
        for name in CONFIGS:
            out = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate",
                                  str(ENG / f"{name}.pixelart.json")], cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(out.returncode, 0, name + "\n" + out.stdout + out.stderr)
            self.assertIn("verdict: PASS", out.stdout, name)

    def test_every_config_runs_v17_except_the_atlas(self):
        for name in CONFIGS:
            rules = load(ENG / f"{name}.pixelart.json")["rules"]
            self.assertEqual(bool(rules.get("holes")), name != "eng_01", name)
        self.assertEqual(load(ENG / "eng_01.pixelart.json")["rules"]["max_colors"], 17)
        self.assertEqual(load(ENG / "eng_02_keys.pixelart.json")["rules"]["max_colors"], 17)
        for name in CONFIGS[2:]:
            self.assertEqual(load(ENG / f"{name}.pixelart.json")["rules"]["max_colors"], 20, name)

    # ---- E2-3 slammed
    def test_e2_3_slammed_body_is_the_goblin_body_with_the_belt_recoloured(self):
        self.assertEqual(self.slam["layers"], ["body", "gore"])
        self.assertEqual(self.slam["tags"], self.slam_a["tags"])
        for i, (f, a) in enumerate(zip(self.slam["frames"], self.slam_a["frames"])):
            body_a = layer_pix(a["cels"], "body")
            self.assertEqual(layer_pix(f["cels"], "body"), e2.belt_recolour(body_a, e2.buckle_for(body_a)), i)
            self.assertEqual(f["cels"]["gore"], a["cels"]["gore"], i)
            self.assertEqual(f["duration_ms"], a["duration_ms"], i)
            flat = "".join("".join(r) for r in f["cels"]["body"])
            self.assertEqual(flat.count("e") + flat.count("7"), 0, i)                 # no eyes, no pot glint
            self.assertFalse(set(flat) & set("234"), i)                                 # no pot iron (the buckle is 5 and 6)

    def buckles(self, body: dict) -> list[tuple[int, int]]:
        return sorted(p for p, ch in body.items() if ch == "5" and body.get((p[0] + 1, p[1])) == "6")

    def test_buckle_coordinates_are_pinned(self):
        slam = {i: self.buckles(layer_pix(f["cels"], "body")) for i, f in enumerate(self.slam["frames"])}
        self.assertEqual(slam, {0: [], 1: [], 2: [(35, 45)], 3: [], 4: [], 5: []})
        death = {i: self.buckles(layer_pix(f["cels"], "body")) for i, f in enumerate(self.death["frames"])}
        self.assertEqual(death, {0: [(20, 49)], 1: [(20, 49)], 2: [(32, 52)], 3: [], 4: [], 5: []})

    # ---- E2-4 death poses
    def test_e2_4_death_layers_and_body(self):
        self.assertEqual(self.death["layers"], ["body", "prop", "pot", "gore"])
        self.assertEqual(self.death["tags"], self.death_a["tags"])
        for i, f in enumerate(self.death["frames"]):
            c = f["cels"]
            self.assertEqual(f["duration_ms"], self.death_a["frames"][i]["duration_ms"], i)
            self.assertEqual(c["gore"], self.death_a["frames"][i]["cels"]["gore"], i)
            layers = [set(layer_pix(c, name)) for name in ("body", "prop", "pot")]
            self.assertFalse(layers[0] & layers[1] or layers[0] & layers[2] or layers[1] & layers[2], i)   # disjoint
            self.assertFalse(set("".join("".join(r) for r in c["body"])) & set("xyFGHVv"), i)
            self.assertFalse("e" in "".join("".join(r) for r in c["body"]), i)

    def test_e2_4_body_equals_the_goblin_body_minus_the_pot(self):
        for i, f in enumerate(self.death["frames"]):
            a = self.death_a["frames"][i]["cels"]
            _cels, info = e2.death_frame(i)
            expected = layer_pix(a, "body")
            buckle = e2.buckle_for(expected)
            expected = e2.belt_recolour({**expected, **e2.carried_strap(layer_pix(a, "weapon"))}, buckle)
            for p in layer_pix(f["cels"], "pot"):
                expected.pop(p, None)
            got = layer_pix(f["cels"], "body")
            cleaned = {(x, y) for x, y, _o, _n in info["cleaned_orphans"]}
            diff = {p for p in set(expected) | set(got) if expected.get(p) != got.get(p)}
            self.assertEqual(diff, cleaned, i)                    # only the recorded orphan clean-ups differ

    def test_e2_4_pot_layer_only_in_the_crush_frames(self):
        for i, f in enumerate(self.death["frames"]):
            pot = layer_pix(f["cels"], "pot")
            if i <= 2:
                self.assertGreaterEqual(len(pot), 90, i)
                self.assertNotIn("7", pot.values(), i)
            else:
                self.assertFalse(pot, i)

    def test_e2_4_crush_pot_is_under_the_stone_and_covers_no_tooth_or_eye(self):
        underside = {0: 33, 1: 36}
        for i, row in underside.items():
            a_body = layer_pix(self.death_a["frames"][i]["cels"], "body")
            pot = layer_pix(self.death["frames"][i]["cels"], "pot")
            self.assertEqual(min(y for _x, y in pot), row, i)                 # the top row is the stone underside row
            self.assertFalse([p for p in pot if p[1] < row], i)                # nothing above it
            self.assertFalse([p for p in pot if a_body.get(p) == "w"], i)      # no w pixel covered
            xs = [x for x, _y in pot]
            self.assertLessEqual(max(xs), 35 + 2, i)                           # right end at the eye outline, 2 px overlap at most
            self.assertGreaterEqual(min(xs), 0, i)
        origins = {i: e2.POT_ORIGINS[i] for i in range(3)}
        self.assertEqual(origins, {0: (6, 33), 1: (6, 36), 2: (28, 41)})      # no origin was moved

    def test_e2_4_prop_layer_only_in_the_decap_frames(self):
        for i, f in enumerate(self.death["frames"]):
            prop = layer_pix(f["cels"], "prop")
            self.assertEqual(bool(prop), i in (3, 4), i)                       # frame 5 omitted: gore overlaps the box
            if prop:
                block = e1.PARTS["WRENCH"]
                dx = min(x for x, _ in prop) - min(x for x, _ in block)
                dy = min(y for _, y in prop) - min(y for _, y in block)
                self.assertEqual(prop, {(x + dx, y + dy): ch for (x, y), ch in block.items()}, i)   # the whole ENG-01 block

    def test_death_manifest_anchors_and_slam_hints(self):
        d = self.dm["death_poses"]
        self.assertEqual(d["always_shown_layers"], ["body", "prop", "pot"])
        self.assertEqual(d["layers"], ["body", "prop", "pot", "gore"])
        for f in d["frames"]:
            self.assertIn("hip_back", f["anchors"])
        for key in ("hit_origin", "hand_near", "mouth", "squeeze_back", "squeeze_front", "pool_origin", "neck", "jet_base"):
            a = load(ROOT / "assets/manifests/goblin_a_death_h36.json")["death_poses"]["frames"]
            for fa, fe in zip(a, d["frames"]):
                if key in fa["anchors"]:
                    self.assertEqual(fe["anchors"][key], fa["anchors"][key])         # every GOB anchor is copied
        self.assertEqual(set(d["spawner_hints"]["slam"]["poses"]), {"stomped", "wall", "flung"})
        for pose, hint in d["spawner_hints"]["slam"]["poses"].items():
            frame = layer_pix(self.slam["frames"][hint["frame"]]["cels"], "body")
            for point in (hint["pot_launch"], hint["wrench_launch"]):
                self.assertIn(tuple(point), frame, pose)                           # the launch points lie on the impact cel
        self.assertEqual({p: h["frame"] for p, h in d["spawner_hints"]["slam"]["poses"].items()},
                         {"stomped": 0, "wall": 2, "flung": 4})

    # ---- E2-5 parts
    def test_e2_5_parts_ids_and_counts(self):
        ids = [t["name"] for t in self.parts["tags"]]
        gob = [t["name"] for t in self.parts_a["tags"] if t["name"] not in ("club", "club_stick", "pineapple")]
        self.assertEqual(len(ids), 27)
        self.assertEqual(ids, gob + ["pot", "head_bare", "head_bare_dead", "wrench"])
        self.assertEqual(ids[:3], ["head", "torso_upper", "head_dead"])
        self.assertEqual(ids[23:], ["pot", "head_bare", "head_bare_dead", "wrench"])
        for i, f in enumerate(self.parts["frames"]):
            self.assertTrue(any(layer_pix(f["cels"], n) for n in ("body", "prop", "gore")), ids[i])    # no empty cell
        for pid in ("pot", "head", "head_dead"):
            flat = "".join("".join(r) for l in ("body", "prop", "gore") for r in self.parts["frames"][ids.index(pid)]["cels"][l])
            self.assertEqual(flat.count("7"), 1, pid)

    def test_e2_5_unchanged_parts_are_pixel_identical_to_gob_02_a(self):
        ids = [t["name"] for t in self.parts["tags"]]
        gob_ids = [t["name"] for t in self.parts_a["tags"]]
        changed = {"head", "head_dead", "torso_lower", "pot", "wrench"}
        for i, pid in enumerate(ids):
            src = {"head_bare": "head", "head_bare_dead": "head_dead"}.get(pid, pid)
            if pid in changed or pid in ("pot", "wrench"):
                continue
            a = self.parts_a["frames"][gob_ids.index(src)]["cels"]
            c = self.parts["frames"][i]["cels"]
            self.assertEqual(c["body"], a["body"], pid)
            self.assertEqual(c["gore"], a["gore"], pid)
            self.assertFalse(any(r.strip(".") for r in c["prop"]), pid)
            self.assertFalse(any(r.strip(".") for r in a["weapon"]), pid)
        a = self.parts_a["frames"][gob_ids.index("torso_lower")]["cels"]
        self.assertEqual(layer_pix(self.parts["frames"][ids.index("torso_lower")]["cels"], "body"),
                         e2.belt_recolour(layer_pix(a, "body")))

    def test_head_parts_carry_the_pot_on_prop_only(self):
        ids = [t["name"] for t in self.parts["tags"]]
        for pid in ("head", "head_dead"):
            c = self.parts["frames"][ids.index(pid)]["cels"]
            prop = layer_pix(c, "prop")
            self.assertTrue(prop, pid)
            self.assertFalse(set(layer_pix(c, "body")) & set(prop), pid)                      # body cleared under the pot
            self.assertFalse(set(layer_pix(c, "body").values()) & set("234567"), pid)         # iron only on prop
            xs = [x for x, _ in prop] + [x for x, _ in layer_pix(c, "body")]
            self.assertTrue(0 <= min(xs) and max(xs) <= 31, pid)                              # neither cel is clipped
        for pid in ("head_bare", "head_bare_dead"):
            c = self.parts["frames"][ids.index(pid)]["cels"]
            self.assertFalse(any(r.strip(".") for r in c["prop"]), pid)

    def test_parts_manifest_fields(self):
        m = load(ROOT / "assets/manifests/eng_death_parts.json")["death_parts"]
        by = {p["id"]: p for p in m["parts"]}
        self.assertEqual([p["id"] for p in m["parts"]], [t["name"] for t in self.parts["tags"]])
        self.assertEqual([p["frame"] for p in m["parts"]], list(range(27)))
        self.assertEqual(m["always_shown_layers"], ["body", "prop", "pot"])
        self.assertEqual((by["head"]["frame0_origin"], by["head"]["anchors"]), ([29, 25], {"neck": [15, 22]}))
        self.assertEqual(by["head"]["swap_on_first_contact"], "head_dead")
        self.assertEqual(by["head_bare"]["frame0_origin"], [33, 27])
        self.assertEqual(by["head_bare"]["swap_on_first_contact"], "head_bare_dead")
        self.assertEqual(by["head_bare"]["presets"], [2, 3, 4])
        self.assertEqual(by["pot"]["anchors"], {"grip": [2, 17]})
        self.assertEqual((by["pot"]["frame0_origin"], by["pot"]["class"], by["pot"]["material"], by["pot"]["ttl"]),
                         ([28, 20], "clean", "iron", "keep"))
        self.assertEqual((by["wrench"]["frame0_origin"], by["wrench"]["anchors"], by["wrench"]["mass"]),
                         ([16, 38], {"grip": [16, 20]}, "light"))
        for dropped in ("club", "club_stick", "pineapple"):
            self.assertNotIn(dropped, by)

    def test_the_allowlist_lists_the_three_key_pockets(self):
        allow = load(ROOT / "assets/source/pixel_hole_allowlist.json")["grids"]
        entries = allow["assets/source/characters/goblin/engineer/eng_02_keys.grid.json"]
        self.assertEqual([(e["frames"], len(e["pixels"])) for e in entries], [([0], 11), ([1], 26), ([1], 5)])
        for e in entries:
            self.assertIn("design gate", e["reason"])


if __name__ == "__main__":
    unittest.main()
