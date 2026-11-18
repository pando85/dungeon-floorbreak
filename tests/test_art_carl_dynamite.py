"""ANIM-CARL-03: Carl's dynamite throw, docs/02-art/carl-dynamite-anim-03-spec.md.

The shared toolkit validators run on ``assets/source/characters/carl/carl_h52_dynamite.pixelart.json`` and
``assets/source/vfx/dynamite/vfx_dynamite.pixelart.json`` (frozen keys, palette, margins, contact rows). This file
covers the spec rules the toolkit does not: locomotion parity, the tick contract, identity, the prop and spark
rules of the keys and the moving-variant composition rule.

Keys are frozen (toolkit V09); the in-betweens are checked frame by frame against the spec's in-between table
(section 6): upper-block translation and head variants, prop rules, the open-hand flinch, and feet that hold their
exact pixels or visibly lift in the frame they move.
"""
from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_dynamite as ad  # noqa: E402
import author_carl_melee as cm  # noqa: E402

CONFIGS = [ROOT / "assets/source/characters/carl/carl_h52_dynamite.pixelart.json",
           ROOT / "assets/source/vfx/dynamite/vfx_dynamite.pixelart.json"]
KEY_FRAMES = {"t1_light": 1, "t2_hold": 2, "t4_wind": 4, "t5_release": 5}
# Spec section 3: (frame id, ticks) of the standing throw and the head variant/offset of each key.
TICKS = [6, 10, 7, 7, 8, 4, 8, 12, 8]
KEY_HEAD = {"t1_light": ("v3", (1, 1)), "t2_hold": ("v3", (-2, 0)), "t4_wind": ("v3", (-1, 0)),
            "t5_release": ("v2", (0, 0))}
SPAWN_TICK = 41
SPAWN_OFFSET = (16, -46)          # stick centre on the release key, from Carl's pivot (x forward, y down)
# Spec section 6: upper-block translation and head (variant, offset) of every standing frame.
STAND_U = {"t0_draw": (1, 1), "t1_light": (1, 2), "t2_hold": (-2, 2), "t3_hold_b": (-2, 2), "t4_wind": (-2, 1),
           "t5_release": (3, 2), "t6_follow": (5, 3), "t7_brace": (1, 3), "t8_settle": (1, 1)}
STAND_HEAD = {**KEY_HEAD, "t0_draw": ("v3", (1, 1)), "t3_hold_b": ("v3", (-2, 0)), "t6_follow": ("v3", (1, 1)),
              "t7_brace": ("v3", (0, 1)), "t8_settle": ("v0", (0, 0))}
NAMES = ["t0_draw", "t1_light", "t2_hold", "t3_hold_b", "t4_wind", "t5_release", "t6_follow", "t7_brace", "t8_settle"]


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def layer(frame: dict, name: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(name, [])) for x, c in enumerate(r) if c != "."}


def flat(frame: dict, layers: list[str]) -> dict:
    out = {}
    for name in layers:
        out.update(layer(frame, name))
    return out


def _components(pix: dict) -> list[dict]:
    seen, comps = set(), []
    for start in sorted(pix):
        if start in seen:
            continue
        comp, todo = {}, [start]
        seen.add(start)
        while todo:
            x, y = todo.pop()
            comp[(x, y)] = pix[(x, y)]
            for dx, dy in cm.NEIGHBOURS:
                q = (x + dx, y + dy)
                if q in pix and q not in seen:
                    seen.add(q)
                    todo.append(q)
        comps.append(comp)
    return comps


class DynamiteTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.carl, cls.vfx = load(ad.CARL_OUT), load(ad.VFX_OUT)

    def test_committed_grids_match_the_generator(self):
        for path, text in ad.outputs().items():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)

    def test_locomotion_frame_pivot_and_master_palette(self):
        self.assertEqual(self.carl["canvas"], cm.LOCO["canvas"])
        self.assertEqual(self.carl["pivot"], cm.LOCO["pivot"])
        self.assertEqual(self.carl["palette"][:23], cm.LOCO["palette"])     # master colours unchanged
        self.assertEqual([e["key"] for e in self.carl["palette"][23:]], ["n", "m", "o", "y"])
        self.assertEqual(self.carl["layers"], cm.LOCO["layers"][:8] + ["prop"] + cm.LOCO["layers"][8:])
        self.assertTrue({e["key"] for e in self.vfx["palette"]} <= {e["key"] for e in self.carl["palette"]})

    def test_tick_contract(self):
        tags = {t["name"]: t for t in self.carl["tags"]}
        throw = tags["dynamite_throw"]
        ticks = [round(self.carl["frames"][i]["duration_ms"] * 60 / 1000) for i in range(throw["from"], throw["to"] + 1)]
        self.assertEqual(ticks, TICKS)
        self.assertEqual(sum(TICKS), 70)
        self.assertEqual(sum(TICKS[:6]) - 1, SPAWN_TICK)          # last tick of the release frame
        self.assertEqual(sum(TICKS[:1]), 6)                       # the fuse is lit on t6 (light key's first tick)
        for phase in ad.MOVE_PHASES:
            t = tags["dynamite_move_" + phase.split("_", 1)[1]]
            self.assertEqual(t["to"] - t["from"] + 1, len(ad.RUN_FRAMES))

    def test_keys_upper_block_is_the_translated_master(self):
        for name, i in KEY_FRAMES.items():
            ux, uy = ad.STANDING_U[name]
            fr = self.carl["frames"][i]
            for part in ("torso", "jacket"):
                self.assertEqual(layer(fr, part), cm.shift(cm.IDLE0[part], ux, uy), f"{name} {part}")
            head, (hx, hy) = KEY_HEAD[name]
            self.assertLessEqual(max(abs(hx), abs(hy)), 2)
            self.assertEqual(layer(fr, "head"), cm.shift(cm.HEAD[head], ux + hx, uy + hy), name)
            self.assertEqual(layer(fr, "hair"), cm.shift(cm.IDLE0["hair"], ux + hx, uy + hy), name)

    def test_keys_carry_a_lit_stick_and_the_spark_stays_small(self):
        for name, i in KEY_FRAMES.items():
            prop = layer(self.carl["frames"][i], "prop")
            keys = set(prop.values())
            self.assertTrue({"K", "m", "n", "o", "h", "y", "G"} <= keys, name)    # stick, fuse, spark
            self.assertLessEqual(sum(c == "y" for c in prop.values()), 5, name)  # no blow-out: core <= 5 px
            self.assertLessEqual(len([c for c in prop.values() if c in "yG"]), 13, name)

    def test_release_spawn_offset(self):
        a = ad.prop_anchors(self.carl)[str(KEY_FRAMES["t5_release"])]["stick_centre"]
        self.assertEqual((a[0], -a[1]), SPAWN_OFFSET)

    def test_wind_up_reads_behind_and_above_the_head(self):
        fr = self.carl["frames"][KEY_FRAMES["t4_wind"]]
        prop, hair = layer(fr, "prop"), layer(fr, "hair")
        self.assertLess(max(x for x, _ in prop), min(x for x, _ in hair))   # the stick is entirely behind the hair
        self.assertLess(min(y for _, y in prop), min(y for _, y in hair))   # and the spark is above it

    def test_release_is_not_the_haymaker(self):
        """The release hand is open and above the shoulder line; the stick is ahead of the hand."""
        fr = self.carl["frames"][KEY_FRAMES["t5_release"]]
        near, prop = layer(fr, "near_arm"), layer(fr, "prop")
        self.assertLessEqual(min(y for _, y in near), 13)                   # hand top at head height
        self.assertGreater(max(x for x, _ in prop), max(x for x, _ in near))

    # ---------------------------------------------------------------- in-betweens (spec section 6)
    def stand(self, name: str) -> dict:
        return self.carl["frames"][NAMES.index(name)]

    def test_every_standing_frame_keeps_the_master_upper_block(self):
        for name in NAMES:
            ux, uy = STAND_U[name]
            self.assertEqual(ad.STANDING_U[name], (ux, uy), name)
            fr = self.stand(name)
            for part in ("torso", "jacket"):
                self.assertEqual(layer(fr, part), cm.shift(cm.IDLE0[part], ux, uy), f"{name} {part}")
            head, (hx, hy) = STAND_HEAD[name]
            self.assertLessEqual(max(abs(hx), abs(hy)), 2, name)
            self.assertEqual(layer(fr, "head"), cm.shift(cm.HEAD[head], ux + hx, uy + hy), name)
            self.assertEqual(layer(fr, "hair"), cm.shift(cm.IDLE0["hair"], ux + hx, uy + hy), name)

    def test_hold_b_differs_from_hold_only_in_the_spark(self):
        h, hb = self.stand("t2_hold"), self.stand("t3_hold_b")
        for l in self.carl["layers"]:
            if l != "prop":
                self.assertEqual(h["cels"].get(l), hb["cels"].get(l), l)
        a, b = layer(h, "prop"), layer(hb, "prop")
        spark = {(x, y) for x in range(40, 44) for y in range(11, 15)}
        self.assertEqual({k: c for k, c in a.items() if k not in spark}, {k: c for k, c in b.items() if k not in spark})
        self.assertEqual({k: c for k, c in b.items() if k in spark}, cm.shift(ad.SPARK_B, 40, 11))
        self.assertNotEqual(a, b)

    def test_draw_stick_is_unlit_and_low_behind_him(self):
        fr = self.stand("t0_draw")
        prop = layer(fr, "prop")
        self.assertTrue(prop)
        self.assertFalse({"y", "G"} & set(prop.values()))                      # no spark yet (it lights on t6)
        self.assertLessEqual(min(x for x, _ in prop), 16)                      # behind the hip, not in front
        self.assertGreaterEqual(min(y for _, y in prop), 26)                   # below the chest
        self.assertEqual(layer(fr, "far_arm"), cm.shift(cm.IDLE0["far_arm"], 1, 1))   # idle far arm moved (+1, +1)

    def test_recovery_frames_carry_no_prop(self):
        for name in ("t6_follow", "t7_brace", "t8_settle"):
            self.assertEqual(layer(self.stand(name), "prop"), {}, name)

    def test_settle_hands_over_to_idle(self):
        fr = self.stand("t8_settle")
        for l in ("far_arm", "near_arm", "watch"):
            self.assertEqual(layer(fr, l), cm.shift(cm.IDLE0[l], 1, 1), l)

    def test_follow_hand_is_open_and_below_the_chest(self):
        near = layer(self.stand("t6_follow"), "near_arm")
        self.assertTrue(all(near.get((36 + x, 30 + y)) == c for (x, y), c in ad.FLAT_HAND_DOWN.items()))
        self.assertGreaterEqual(min(y for (x, y) in near if x >= 38), 26)      # the arm swings down, hand below the ribs

    def test_flinch_is_an_open_cupped_hand_never_a_fist_or_two_hands(self):
        fr = self.stand("t7_brace")
        near, far = layer(fr, "near_arm"), layer(fr, "far_arm")
        cups = [(x, y) for x in range(48) for y in range(56)
                if all(near.get((x + dx, y + dy)) == c for (dx, dy), c in ad.HAND_CUP.items())]
        self.assertEqual(len(cups), 1)                                         # the open-hand stamp, once
        cx, cy = cups[0]
        self.assertLessEqual(cy + 4, 19)                                       # beside the head, above the shoulder line
        tips = [ad.HAND_CUP.get((dx, 0), ".") for dx in range(5)]
        self.assertEqual(tips, [".", "L", ".", "L", "."])                      # two skin finger tips with a gap, no outline tips
        fist = cm.FIST_SMALL
        for x in range(48):
            for y in range(0, 20):                                             # no fist stamp above the shoulders
                self.assertFalse(all(near.get((x + dx, y + dy)) == c for (dx, dy), c in fist.items()), (x, y))
        self.assertGreaterEqual(min(y for _, y in far), 21)                    # the far arm is down: one hand up only

    def foot_components(self, name: str) -> list[dict]:
        return sorted(_components(layer(self.stand(name), "shoes")), key=lambda c: min(x for x, _ in c))

    def test_feet_hold_their_exact_pixels_or_visibly_lift(self):
        """Between consecutive frames drawn in the execution phase (and the release key they follow), each foot is
        either planted in both (identical pixels: no slide) or raised at least 2 rows in one of them."""
        pairs = [("t0_draw", "t1_light"), ("t2_hold", "t3_hold_b"), ("t5_release", "t6_follow"),
                 ("t6_follow", "t7_brace"), ("t7_brace", "t8_settle")]
        for a, b in pairs:
            fa, fb = self.foot_components(a), self.foot_components(b)
            self.assertEqual((len(fa), len(fb)), (2, 2), (a, b))
            for i, (ca, cb) in enumerate(zip(fa, fb)):
                ba, bb = max(y for _, y in ca), max(y for _, y in cb)
                if ba == 55 and bb == 55:
                    self.assertEqual(ca, cb, f"{a} -> {b} foot {i} slides")
                else:
                    self.assertLessEqual(min(ba, bb), 53, f"{a} -> {b} foot {i} moves without lifting")

    def test_every_frame_has_a_planted_foot_and_settle_matches_idle_feet(self):
        for name in NAMES:
            self.assertTrue(any(max(y for _, y in c) == 55 for c in self.foot_components(name)), name)
        idle = layer(cm.LOCO["frames"][0], "shoes")
        idle_x = sorted({min(x for x, y in comp) for comp in _components(idle)})
        settle_x = [min(x for x, _ in c) for c in self.foot_components("t8_settle")]
        for ix, sx in zip(idle_x, settle_x):
            self.assertLessEqual(abs(ix - sx), 2)                              # at most a 2 px foot snap

    def test_in_between_anchors_and_spark_caps(self):
        for name in ("t3_hold_b",):
            prop = layer(self.stand(name), "prop")
            self.assertLessEqual(sum(c == "y" for c in prop.values()), 5)
        a = ad.frame_anchors(self.carl, 3)
        self.assertEqual(a, ad.frame_anchors(self.carl, 2))                    # hold_b: same stick and spark centre

    def test_moving_frames_keep_the_run_body_and_move_the_throw_arms(self):
        tags = {t["name"]: t for t in self.carl["tags"]}
        body = [l for l in self.carl["layers"] if l not in ad.ARM_LAYERS and l != "pin"]
        names = [n for n, _, _ in ad.STANDING]
        for phase in ad.MOVE_PHASES:
            t = tags["dynamite_move_" + phase.split("_", 1)[1]]
            stand = self.carl["frames"][names.index(phase)]
            for j, n in enumerate(ad.RUN_FRAMES):
                fr, run = self.carl["frames"][t["from"] + j], cm.LOCO["frames"][n]
                for l in body:
                    self.assertEqual(layer(fr, l), layer(run, l), f"{phase} run {n} {l}")
                (rx, ry), (sx, sy) = ad.run_u(n), ad.phase_u(phase)
                tx, ty = ad.MOVE_TRIM.get(phase, (0, 0))
                for l in ad.ARM_LAYERS:
                    self.assertEqual(layer(fr, l), cm.shift(layer(stand, l), rx - sx + tx, ry - sy + ty),
                                     f"{phase} run {n} {l}")

    def test_vfx_stick_frames(self):
        piv = tuple(self.vfx["pivot"])
        for i in (0, 1, 2, 3):          # flight: the stick body is centred on the pivot (within 1 px)
            body = [p for p, c in layer(self.vfx["frames"][i], "body").items() if c in "Kmno"]
            cx = (min(x for x, _ in body) + max(x for x, _ in body)) / 2
            cy = (min(y for _, y in body) + max(y for _, y in body)) / 2
            self.assertLessEqual(abs(cx - piv[0]), 1.0, i)
            self.assertLessEqual(abs(cy - piv[1]), 1.0, i)
        for i in (4, 5):                # fizzle: the stick lies on the walking line (pivot row)
            px = layer(self.vfx["frames"][i], "body")
            self.assertEqual(max(y for _, y in px), piv[1])

    def test_vfx_in_betweens_follow_the_spec_table(self):
        frames = self.vfx["frames"]
        body = [layer(f, "body") for f in frames]
        def split(pix):
            stick = {k: c for k, c in pix.items() if c in "Kmno"}
            fuse = [k for k, c in pix.items() if c == "h"]
            core = [k for k, c in pix.items() if c == "y"]
            return stick, fuse, core
        stick, fuse, core = split(body[1])                         # f1_down: fuse at the bottom, spark below it
        self.assertGreater(min(y for _, y in fuse), max(y for _, y in stick))
        self.assertGreater(min(y for _, y in core), max(y for _, y in fuse))
        stick, fuse, core = split(body[2])                         # f2_left: fuse on the left, spark above-left
        self.assertLess(max(x for x, _ in fuse), min(x for x, _ in stick))
        self.assertLess(max(x for x, _ in core), min(x for x, _ in stick))
        for i in range(6):                 # the core sits at the burning fuse tip (within 2 px: a halo pixel can be the tip)
            stick, fuse, core = split(body[i])
            self.assertTrue(any(abs(c[0] - f[0]) <= 2 and abs(c[1] - f[1]) <= 2 for c in core for f in fuse), i)
        for i in (0, 1, 2, 4):             # and touches it outright on every frame except the frozen f3_up key and z1
            stick, fuse, core = split(body[i])
            self.assertTrue(any(abs(c[0] - f[0]) <= 1 and abs(c[1] - f[1]) <= 1 for c in core for f in fuse), i)
        outside = lambda pix: {k: c for k, c in pix.items() if not (17 <= k[0] <= 20 and 6 <= k[1] <= 9)}
        self.assertEqual(outside(body[5]), outside(body[4]))                  # z1 flickers inside the spark's 4 x 4 box
        self.assertNotEqual(body[4], body[5])

    def test_manifests_carry_the_tick_contract_anchors_and_spawn(self):
        cm_ = load(ad.CARL_MANIFEST)
        d = cm_["dynamite"]
        self.assertEqual(d["total_ticks"], 70)
        self.assertEqual(d["fuse_lit_tick"], 6)
        self.assertEqual(d["projectile_spawn"]["tick"], SPAWN_TICK)
        self.assertEqual(tuple(d["projectile_spawn"]["standing_offset_px"]), SPAWN_OFFSET)
        move = d["projectile_spawn"]["moving_offsets"]
        self.assertEqual([m["run_frame"] for m in move], ad.RUN_FRAMES)
        # spec section 4: add (0, 0) on run frames 4, 6, 7, 9 and (0, +1) on 5, 8 (screen y down)
        self.assertEqual([tuple(m["delta_from_standing_px"]) for m in move],
                         [(0, 0), (0, 1), (0, 0), (0, 0), (0, 1), (0, 0)])
        frames = d["moves"]["dynamite_throw"]["frames"]
        self.assertEqual([f["duration_ticks"] for f in frames], TICKS)
        self.assertEqual(frames[5]["ticks"][1], SPAWN_TICK)
        self.assertEqual(frames[1]["ticks"][0], 6)
        self.assertEqual({f["name"] for f in frames if f["key"]}, set(KEY_FRAMES))
        self.assertEqual(d["prop_anchors_px"]["t5_release"]["stick_centre"], [16, 46])
        for name, a in d["prop_anchors_px"].items():
            if name != "note":
                self.assertEqual(a, ad.frame_anchors(self.carl, NAMES.index(name)), name)
        self.assertEqual(cm_["animation_tags"], [t["name"] for t in self.carl["tags"]])
        v = load(ad.VFX_MANIFEST)
        self.assertEqual(v["vfx"]["spawn"]["tick"], SPAWN_TICK)
        self.assertEqual(v["animation_tags"], ["dynamite_flight", "dynamite_fizzle"])
        self.assertEqual(v["vfx"]["blast"]["reuse"], "assets/manifests/vfx-01.json")
        self.assertEqual([f["author"] for f in v["vfx"]["frames"]],
                         ["key", "inbetween", "inbetween", "key", "key", "inbetween"])

    def test_ascii_option_runs_for_every_frame_id_and_index(self):
        for arg in NAMES + ["0", "2", "8", "44", "vfx"]:
            r = subprocess.run([sys.executable, "scripts/author_carl_dynamite.py", "--ascii", arg], cwd=ROOT,
                               capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, f"--ascii {arg}: {r.stderr}")

    def test_toolkit_validators_pass(self):
        for cfg in CONFIGS:
            r = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(cfg.relative_to(ROOT))],
                               cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
