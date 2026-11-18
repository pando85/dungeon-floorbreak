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
            self.assertEqual(release[23 + dy][51 + dx + 1:55 + dx], "KKK")
            self.assertEqual(release[24 + dy][51 + dx:55 + dx], "WEEW")

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
        self.assertEqual(m["projectile_spawn_offset_by_locomotion_frame"][0], [23, -19])

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


if __name__ == "__main__":
    unittest.main()
