"""ANIM-DONUT-03 spec rules not covered by ``python3 -m scripts.pixelart validate``: the authoring record is
reproducible, the cast composes with locomotion (body, legs and tail untouched, head translated by the measured
offsets), the master palette is unchanged, keys are frozen, and the missile VFX glow stays baked and capped."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_donut_missile as author  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

CAST_CFG = ROOT / "assets/source/characters/donut/donut_h40_missile.pixelart.json"
VFX_CFG = ROOT / "assets/source/vfx/magic_missile/vfx_magic_missile.pixelart.json"
BODY_LAYERS = ("tail", "legs_far", "body", "legs_near")


def pixels(rows: list[str]) -> set:
    return {(x, y, c) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


class DonutMissileTests(unittest.TestCase):
    def setUp(self) -> None:
        self.loco = json.loads(author.LOCO_GRID.read_text())
        self.cast = json.loads(author.CAST_GRID.read_text())
        self.vfx = json.loads(author.VFX_GRID.read_text())

    def test_authoring_script_reproduces_sources_and_manifests(self) -> None:
        self.assertEqual(author.main(["keys", "--check"]), 0)

    def test_ase_files_match_the_grids(self) -> None:
        for grid, ase in ((self.cast, author.CAST_GRID.with_suffix("").with_suffix(".ase")),
                          (self.vfx, author.VFX_GRID.with_suffix("").with_suffix(".ase"))):
            data = pg.read_ase(ase.read_bytes())
            self.assertEqual(data["canvas"], grid["canvas"])
            self.assertEqual([f["duration_ms"] for f in data["frames"]], [f["duration_ms"] for f in grid["frames"]])

    def test_same_frame_pivot_and_master_palette_as_locomotion(self) -> None:
        self.assertEqual(self.cast["canvas"], self.loco["canvas"])
        self.assertEqual(self.cast["pivot"], self.loco["pivot"])
        self.assertEqual(self.cast["palette"], self.loco["palette"])
        self.assertEqual(self.cast["layers"], self.loco["layers"])
        self.assertEqual(len(self.cast["frames"]), 48)

    def test_head_offsets_match_the_locomotion_atlas(self) -> None:
        h0 = pixels(self.loco["frames"][0]["cels"]["head"])
        for n, (dx, dy) in enumerate(author.HEAD_OFFSETS):
            moved = {(x + dx, y + dy, c) for x, y, c in h0}
            self.assertEqual(moved, pixels(self.loco["frames"][n]["cels"]["head"]), n)

    def test_cast_never_touches_body_legs_or_tail(self) -> None:
        for i, frame in enumerate(self.cast["frames"]):
            base = self.loco["frames"][i % 16]["cels"]
            for layer in BODY_LAYERS:
                self.assertEqual(frame["cels"].get(layer), base.get(layer), (i, layer))

    def test_gather_charges_the_gem_and_release_hisses(self) -> None:
        for i in range(16):
            dx, dy = author.HEAD_OFFSETS[i]
            gather = self.cast["frames"][i]["cels"]
            self.assertEqual(gather["gear"][2 + dy][51 + dx], "w")
            release = self.cast["frames"][16 + i]["cels"]["face"]
            self.assertEqual(release[23 + dy][52 + dx:55 + dx], "wKw")      # 3 x 2 hiss, two fang points
            self.assertEqual(release[24 + dy][52 + dx:55 + dx], "KKK")
            head = self.cast["frames"][16 + i]["cels"]["head"]
            self.assertEqual(head[8 + dy][40 + dx], ".")                  # ear tips gone on release
            self.assertNotEqual(gather["head"][8 + dy][40 + dx], ".")     # gather keeps the master ears

    def test_keys_frozen_and_inbetweens_listed(self) -> None:
        for cfg_path, keys, inb, total in ((CAST_CFG, author.CAST_KEYS, author.CAST_INBETWEENS, 48),
                                           (VFX_CFG, author.VFX_KEYS, author.VFX_INBETWEENS, 9)):
            cfg = json.loads(cfg_path.read_text())
            frozen = cfg["rules"]["frozen_keys"]
            self.assertEqual(tuple(frozen["frames"]), keys)
            fixture = json.loads((ROOT / frozen["fixture"]).read_text())
            self.assertEqual(set(fixture["keys"]), {str(k) for k in keys})
            self.assertFalse(set(keys) & set(inb))
            self.assertLessEqual(set(keys) | set(inb), set(range(total)))

    def test_cast_tick_contract(self) -> None:
        m = json.loads(author.CAST_MANIFEST.read_text())["cast"]
        self.assertEqual(m["phase_ticks"], {"cast_gather": [0, 3], "cast_release": [4, 9], "cast_follow": [10, 15]})
        self.assertEqual(m["release_tick"], 4)
        self.assertEqual(len(m["projectile_spawn_offset_by_locomotion_frame"]), 16)
        self.assertEqual(m["projectile_spawn_offset_by_locomotion_frame"][0], [27, -19])

    def test_vfx_palette_ticks_and_glow_caps(self) -> None:
        self.assertLessEqual(len(self.vfx["palette"]), 20)
        gem = {e["key"]: e["rgba"] for e in self.loco["palette"]}["M"]
        self.assertIn(gem, {e["rgba"] for e in self.vfx["palette"]})
        man = json.loads(author.VFX_MANIFEST.read_text())["vfx"]
        self.assertEqual(sum(f["ticks"] for f in man["frames"][:4]), 16)
        self.assertEqual(sum(f["ticks"] for f in man["frames"][4:]), 18)
        for n, frame in enumerate(self.vfx["frames"]):
            rows = frame["cels"]["body"]
            filled = sum(c != "." for r in rows for c in r)
            self.assertLess(filled, 0.35 * 32 * 32, n)
            self.assertLessEqual(sum(r.count("w") for r in rows), 4, n)

    # --- in-betweens (spec section 9)
    def test_follow_head_recovers_to_the_master_face_with_lowered_lids(self) -> None:
        master = self.loco["frames"][0]["cels"]
        for i in range(16):
            dx, dy = author.HEAD_OFFSETS[i]
            tx = author.THRUST["cast_follow"][0]
            follow = self.cast["frames"][32 + i]["cels"]
            shifted = author.shift(master["face"], dx + tx, dy)
            for y in range(19, 28):                    # mouth, muzzle and chin rows: the master face (mouth closed)
                self.assertEqual(follow["face"][y + dy], shifted[y + dy], (i, y))
            gather = self.cast["frames"][i]["cels"]["face"]
            for y in (16, 17):                         # gather lids, one row lowered; no brow pixels at x 43 / 59
                self.assertEqual(follow["face"][y + dy][43 + dx + tx:61 + dx + tx], gather[y + dy][43 + dx:61 + dx], (i, y))
            self.assertEqual(sum(r.count("w") for r in follow["face"]), 4, i)   # catchlights only, no fangs
            tiara_master = author.shift([r if y <= 8 else "." * 72 for y, r in enumerate(master["gear"])], dx + tx, dy)
            self.assertEqual(follow["gear"][:9 + dy], tiara_master[:9 + dy], i)       # master tiara and gem
            self.assertTrue(all("w" not in r and "P" not in r for r in follow["gear"][:9 + dy]), i)  # no twinkle, gem normal

    def test_follow_ears_sit_halfway_between_master_and_release(self) -> None:
        for i in range(16):
            dx, dy = author.HEAD_OFFSETS[i]
            gather = self.cast["frames"][i]["cels"]["head"]
            release = self.cast["frames"][16 + i]["cels"]["head"]
            follow = self.cast["frames"][32 + i]["cels"]["head"]
            self.assertEqual(follow[8 + dy][39 + dx:43 + dx], "....")
            self.assertEqual(follow[9 + dy][41 + dx:43 + dx], "jK")             # tip one row lower than the master (row 8)
            self.assertNotEqual(gather[8 + dy][40 + dx], ".")
            self.assertEqual(release[9 + dy][41 + dx], ".")                    # release: tips gone
            self.assertNotEqual(follow, gather)
            self.assertNotEqual(follow, release)

    def test_cast_frames_are_unique_and_the_orphan_limit_stays_at_nine(self) -> None:
        keys = [json.dumps(f["cels"], sort_keys=True) for f in self.cast["frames"]]
        self.assertEqual(len(set(keys)), 48)
        for cfg_path in (CAST_CFG, VFX_CFG):
            rules = json.loads(cfg_path.read_text())["rules"]
            self.assertEqual(rules["max_orphans_per_frame"], 9)
            self.assertEqual(rules["max_isolated_per_frame"], 0)
            self.assertTrue(rules["unique_frames"])
        rel = {c["id"]: c["frames"] for c in json.loads(CAST_CFG.read_text())["rules"]["key_counts"]}
        self.assertEqual(rel["C2-release-fangs-and-catchlights"], list(range(16, 32)))

    def test_bolt_flight_loop_cycles_streaks_and_keeps_white_tiny(self) -> None:
        frames = [self.vfx["frames"][n]["cels"]["body"] for n in range(4)]
        self.assertEqual(len({tuple(f) for f in frames}), 4)
        left = lambda rows, y: min(i for i, c in enumerate(rows[y]) if c != ".")      # noqa: E731
        for n, rows in enumerate(frames):
            white = sum(r.count("w") for r in rows)
            self.assertIn(white, (1, 2), n)
            for y, want0, want in zip((14, 16, 18), author.BOLT_CYCLE[0][0], author.BOLT_CYCLE[n][0]):
                self.assertEqual(left(frames[0], y) - left(rows, y), want - want0, (n, y))   # streak lengths cycle
        self.assertEqual(frames[0], author.bolt())

    def test_impact_in_betweens_cool_between_the_keys(self) -> None:
        def count(rows, ch):
            return sum(r.count(ch) for r in rows)
        peak, mid, fade, last = (self.vfx["frames"][n]["cels"]["body"] for n in (5, 6, 7, 8))
        self.assertEqual((count(mid, "w"), count(mid, "c")), (0, 0))
        self.assertLess(count(mid, "p") + count(mid, "m"), count(peak, "p") + count(peak, "m") + count(peak, "c"))
        self.assertGreater(count(mid, "p") + count(mid, "m"), count(fade, "m") + count(fade, "d"))
        for ch in "wcpm":
            self.assertEqual(count(last, ch), 0, ch)                          # nothing hot in the last frame
        self.assertGreater(count(last, "d") + count(last, "v"), 0)
        self.assertLess(sum(c != "." for r in last for c in r), sum(c != "." for r in fade for c in r))
        # motes two px further from the hit point than on the peak
        def far(rows):
            return max(max(abs(x + 0.5 - 15.5), abs(y - 15.5)) for y, r in enumerate(rows) for x, c in enumerate(r) if c in "lv")
        self.assertGreater(far(mid), far(peak))


if __name__ == "__main__":
    unittest.main()
