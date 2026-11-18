"""ANIM-CARL-03: Carl's dynamite throw (design phase), docs/02-art/carl-dynamite-anim-03-spec.md.

The shared toolkit validators run on ``assets/source/characters/carl/carl_h52_dynamite.pixelart.json`` and
``assets/source/vfx/dynamite/vfx_dynamite.pixelart.json`` (frozen keys, palette, margins, contact rows). This file
covers the spec rules the toolkit does not: locomotion parity, the tick contract, identity, the prop and spark
rules of the keys and the moving-variant composition rule.

Design phase: the in-betweens are copies of their nearest key. The execution phase replaces
``test_design_phase_in_betweens_are_key_copies`` with per-frame checks of the spec's in-between table.
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


def load(p: Path) -> dict:
    return json.loads(p.read_text(encoding="utf-8"))


def layer(frame: dict, name: str) -> dict:
    return {(x, y): c for y, r in enumerate(frame["cels"].get(name, [])) for x, c in enumerate(r) if c != "."}


def flat(frame: dict, layers: list[str]) -> dict:
    out = {}
    for name in layers:
        out.update(layer(frame, name))
    return out


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

    def test_design_phase_in_betweens_are_key_copies(self):
        frames = self.carl["frames"]
        for i, (name, _, src) in enumerate(ad.STANDING):
            if src is not None:
                self.assertEqual(frames[i]["cels"], frames[[n for n, _, _ in ad.STANDING].index(src)]["cels"], name)

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

    def test_toolkit_validators_pass(self):
        for cfg in CONFIGS:
            r = subprocess.run([sys.executable, "-m", "scripts.pixelart", "validate", str(cfg.relative_to(ROOT))],
                               cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
