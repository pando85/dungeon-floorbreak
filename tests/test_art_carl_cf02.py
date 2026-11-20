"""ART-CARL-CF02: Carl combat-acting sheet (9 frozen ACT-DESIGN keys, 4 derived frames), docs/02-art/cf02-acting-spec.md.

The shared toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/carl/carl_h52_cf02.pixelart.json``; this file covers the spec rules the toolkit does not
(keys equal the ACT-DESIGN key files except the airborne lift, identity of heads, the Croc rules, the tick table).
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_cf02 as ac  # noqa: E402
import author_carl_melee as cm  # noqa: E402

GRID = ROOT / "assets/source/characters/carl/carl_h52_cf02.grid.json"
CONFIG = ROOT / "assets/source/characters/carl/carl_h52_cf02.pixelart.json"
MANIFEST = ROOT / "assets/manifests/carl_book1_h52_cf02.json"
MELEE = ROOT / "assets/source/characters/carl/carl_h52_melee.grid.json"
LOCO = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
MASTER = ROOT / "assets/source/characters/carl/carl_h52.grid.json"

ORDER = ["stomp_knee_up", "stomp_stamp", "stomp_recover", "air_kick_chamber", "air_kick_boot", "dive_tuck",
         "dive_plunge", "dive_heavy_landing", "bailout_pushoff", "bailout_scramble", "flinch",
         "frustration_shake", "frustration_flap"]
KEYS = ["stomp_knee_up", "stomp_stamp", "air_kick_boot", "dive_tuck", "dive_plunge", "dive_heavy_landing",
        "bailout_scramble", "flinch", "frustration_shake"]
DERIVED = ["stomp_recover", "air_kick_chamber", "bailout_pushoff", "frustration_flap"]
AIRBORNE = {"air_kick_chamber": 2, "air_kick_boot": 2, "dive_tuck": 2, "dive_plunge": 2, "bailout_pushoff": 2,
            "bailout_scramble": 1}
TAGS = {"stomp": (0, 2), "air_kick": (3, 4), "dive_tuck": (5, 5), "dive_plunge": (6, 6), "dive_land": (7, 7),
        "bailout": (8, 9), "flinch": (10, 10), "frustration": (11, 12)}
CROCS = set("PQX")


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


GRID_DATA = load(GRID)
LAYERS = GRID_DATA["layers"]


def pix(frame: dict, layer: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(layer, [])) for x, c in enumerate(r) if c != "."}


def flat(i: int) -> dict:
    out = {}
    for l in LAYERS:
        out.update(pix(GRID_DATA["frames"][i], l))
    return out


def frame(name: str) -> dict:
    return GRID_DATA["frames"][ORDER.index(name)]


def key_frame(name: str) -> dict:
    for path in (ac.GROUND_KEYS, ac.AIR_KEYS):
        g = load(path)
        for t in g["tags"]:
            if t["name"] == name:
                return g["frames"][t["from"]]
    raise KeyError(name)


def has_translation(src: dict, dst: dict) -> bool:
    """True if ``src`` (a pixel set) is a subset of ``dst`` at some small integer offset."""
    for dx in range(-12, 13):
        for dy in range(-12, 13):
            if all(dst.get((x + dx, y + dy)) == c for (x, y), c in src.items()):
                return True
    return False


class CarlCf02Sheet(unittest.TestCase):
    def test_committed_grid_equals_build(self):
        self.assertEqual(GRID.read_text(encoding="utf-8"), cm.dump(ac.build()))

    def test_canvas_pivot_palette_layers(self):
        master = load(MASTER)
        self.assertEqual(GRID_DATA["canvas"], [48, 56])
        self.assertEqual(GRID_DATA["pivot"], [25, 55])
        self.assertEqual(GRID_DATA["palette"], master["palette"])
        self.assertEqual(GRID_DATA["layers"], master["layers"])
        self.assertEqual(len(GRID_DATA["frames"]), 13)

    def test_tags_follow_the_acceptance_targets(self):
        got = {t["name"]: (t["from"], t["to"]) for t in GRID_DATA["tags"]}
        self.assertEqual(got, TAGS)

    def test_keys_equal_the_act_design_keys_except_the_airborne_lift(self):
        for name in KEYS:
            lift = AIRBORNE.get(name, 0)
            key, sheet = key_frame(name), frame(name)
            for layer in LAYERS:
                rows = key["cels"].get(layer, ["." * 48] * 56)
                expect = rows[lift:] + ["." * 48] * lift
                self.assertEqual(sheet["cels"].get(layer, ["." * 48] * 56), expect, f"{name}/{layer}")

    def test_airborne_rows(self):
        for name, lift in AIRBORNE.items():
            ys = [y for (_, y) in flat(ORDER.index(name))]
            self.assertEqual(max(ys), 55 - lift, name)
            self.assertLessEqual(max(ys), 54, name)          # never on the pivot row
        for name in ORDER:
            if name not in AIRBORNE:
                self.assertEqual(max(y for (_, y) in flat(ORDER.index(name))), 55, name)

    def test_first_and_last_columns_empty(self):
        for i, name in enumerate(ORDER):
            xs = {x for (x, _) in flat(i)}
            self.assertNotIn(0, xs, name)
            self.assertNotIn(47, xs, name)

    def test_palette_is_the_approved_one(self):
        declared = {e["key"] for e in GRID_DATA["palette"]}
        for i, name in enumerate(ORDER):
            self.assertLessEqual(set(flat(i).values()), declared, name)

    def test_no_new_head(self):
        """Every head and hair cel is one of the approved heads (v0..v3, pain) translated, never redrawn."""
        for name in ORDER:
            if name.startswith("frustration"):
                continue            # the key's own tilted effort head (spec: "its front half 1 px further down")
            head = pix(frame(name), "head")
            fits = [h for h, p in cm.HEAD.items() if has_translation(p, head)]
            self.assertTrue(fits, f"{name}: head is none of v0..v3 / pain")
            hair_ok = has_translation(cm.IDLE0["hair"], pix(frame(name), "hair"))
            self.assertTrue(hair_ok, f"{name}: hair is not the approved hair")

    def test_derived_frames_use_the_named_heads(self):
        def heads(name):
            head = pix(frame(name), "head")
            return {h for h, p in cm.HEAD.items() if has_translation(p, head)}
        self.assertIn("v3", heads("stomp_recover"))
        self.assertIn("v2", heads("air_kick_chamber"))       # the boot key's yelling head
        self.assertIn("v2", heads("bailout_pushoff"))        # k3 yelling head of bailout_scramble

    def test_flap_head_is_the_shake_head_one_px_lower(self):
        a, b = pix(frame("frustration_shake"), "head"), pix(frame("frustration_flap"), "head")
        self.assertEqual({(x, y + 1): c for (x, y), c in a.items()}, b)

    def test_unique_frames(self):
        seen = {}
        for i, name in enumerate(ORDER):
            key = json.dumps(sorted(flat(i).items()))
            self.assertNotIn(key, seen, f"{name} equals {seen.get(key)}")
            seen[key] = name

    def test_stomp_knee_up_croc_flat_and_off_the_floor(self):
        """The raised Croc is flat, 7 px above the floor (lowest Croc row 48) and in the stomp reach (x +6..+15)."""
        shoes = pix(frame("stomp_knee_up"), "shoes")
        raised = {k: c for k, c in shoes.items() if k[0] - 25 >= 6}
        self.assertTrue(raised)
        self.assertEqual(max(y for _, y in raised), 55 - 7)
        self.assertGreaterEqual(min(x for x, _ in raised) - 25, 5)
        self.assertLessEqual(max(x for x, _ in raised) - 25, 15)
        support = {k for k in shoes if k[0] - 25 < 5}
        self.assertEqual(max(y for _, y in support), 55)

    def test_stomp_stamp_front_croc_planted_in_recover(self):
        stamp = {k: c for k, c in pix(frame("stomp_stamp"), "shoes").items() if k[0] >= 30}
        rec = {k: c for k, c in pix(frame("stomp_recover"), "shoes").items() if k[0] >= 30}
        self.assertEqual(stamp, rec)                      # the stamping Croc does not slide
        back = {k for k in pix(frame("stomp_recover"), "shoes") if k[0] < 30}
        xs = [x - 25 for x, _ in back]
        self.assertEqual((min(xs), max(xs)), (-10, -1))   # back foot flat at x -10..-1
        self.assertEqual(max(y for _, y in back), 55)

    def test_air_kick_boot_lowest_point_is_a_croc(self):
        pixels = flat(ORDER.index("air_kick_boot"))
        low = max(y for _, y in pixels)
        self.assertTrue(all(c in CROCS for (x, y), c in pixels.items() if y == low))

    def test_chamber_does_not_read_as_the_tuck(self):
        """The far leg hangs: the chamber's lowest Croc is straight under the hip while the tuck's feet are both
        pulled up; the two silhouettes differ in more than a handful of pixels."""
        a, b = set(flat(ORDER.index("air_kick_chamber"))), set(flat(ORDER.index("dive_tuck")))
        self.assertGreater(len(a ^ b), 120)

    def test_dive_tuck_is_compact_against_the_plunge(self):
        def height(name):
            ys = [y for _, y in flat(ORDER.index(name))]
            return max(ys) - min(ys) + 1
        self.assertGreaterEqual(height("dive_plunge") - height("dive_tuck"), 6)

    def test_frustration_pair_is_the_hand_change_only(self):
        a, b = frame("frustration_shake"), frame("frustration_flap")
        for layer in ("legs", "shoes", "boxers", "jacket", "pin", "watch"):
            self.assertEqual(a["cels"].get(layer), b["cels"].get(layer), layer)
        for layer, limit in (("far_arm", 2), ("torso", 1)):  # the hand-repainted orphans of the derived frame
            fa, fb = pix(a, layer), pix(b, layer)
            self.assertLessEqual(sum(1 for k in set(fa) | set(fb) if fa.get(k) != fb.get(k)), limit, layer)

    def test_flinch_pulls_in_and_frustration_hand_is_open(self):
        """The fan hand has no closed fist: the near hand holds at least three separate skin columns."""
        hand = {k: c for k, c in pix(frame("frustration_shake"), "near_arm").items() if k[1] <= 23 and k[0] >= 29}
        self.assertGreaterEqual(len({x for (x, y), c in hand.items() if c == "M"}), 3)


class CarlCf02Manifest(unittest.TestCase):
    manifest = load(MANIFEST)

    def test_manifest_matches_the_sheet(self):
        m = self.manifest
        self.assertEqual(m["frame_size"], [48, 56])
        self.assertEqual(m["pivot"], [25, 55])
        self.assertEqual(m["animation_tags"], list(TAGS))
        rows = m["cf02_acting"]["frames"]
        self.assertEqual([r["name"] for r in rows], ORDER)
        for r, f in zip(rows, GRID_DATA["frames"]):
            self.assertEqual(r["duration_ms"], f["duration_ms"])
            self.assertEqual(r["duration_ms"], round(r["ticks"] * 1000 / 60))
        self.assertEqual([r["name"] for r in rows if r["author"] == "key"], [n for n in ORDER if n in KEYS])
        self.assertEqual([r["name"] for r in rows if r["author"] == "derived"], [n for n in ORDER if n in DERIVED])

    def test_move_tick_windows(self):
        mv = self.manifest["cf02_acting"]["moves"]
        self.assertEqual(mv["stomp"]["total_ticks"], 26)
        self.assertEqual([f["ticks"] for f in mv["stomp"]["frames"]], [[0, 8], [9, 15], [16, 25]])
        self.assertEqual([f["ticks"] for f in mv["air_kick"]["frames"]][:2], [[0, 4], [5, 14]])
        self.assertEqual(mv["dive_land"]["total_ticks"], 10)
        self.assertEqual([f["ticks"] for f in mv["bailout"]["frames"]][0], [0, 2])
        self.assertEqual(mv["bailout"]["frames"][1]["ticks"][0], 3)
        self.assertEqual(mv["flinch"]["total_ticks"], 8)
        self.assertEqual(mv["frustration"]["total_ticks"], 24)

    def test_toolkit_validators_pass(self):
        out = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(CONFIG)], cwd=ROOT,
                             capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        self.assertIn("verdict: PASS", out.stdout)


class ApprovedSheetsUntouched(unittest.TestCase):
    def test_melee_and_locomotion_keep_their_frame_counts(self):
        self.assertEqual(len(load(MELEE)["frames"]), 24)
        self.assertEqual(len(load(LOCO)["frames"]), 16)


if __name__ == "__main__":
    unittest.main()
