"""AREA-REFUGE-01: the guild-room kit (sources, contract, door geometry, hearth frames, determinism)."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_refuge01 as hearth  # noqa: E402
import author_refuge01_props as props  # noqa: E402
from scripts.pixelart.common import grid_images, load_grid_file  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
SIZES = {  # spec section 4: native size per piece
    "refuge_bed": (64, 28), "refuge_bookshelf": (36, 56), "refuge_desk": (56, 26), "refuge_stool": (14, 18),
    "refuge_barrel": (22, 28), "refuge_wall_map": (32, 24), "refuge_frame_portrait": (24, 28),
    "refuge_frame_landscape": (28, 20), "refuge_satchel": (12, 18), "refuge_window": (96, 100),
    "refuge_hearth_flue": (48, 16), "refuge_door_inside_closed": (112, 136), "refuge_door_inside_open": (112, 136),
}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


class RefugeKitTest(unittest.TestCase):
    def test_sources_are_deterministic(self):
        for name, spec in props.ATLASES.items():
            grid, _ = props.build(name)
            self.assertEqual(grid, load(PROPS / f"{spec['stem']}.grid.json"), name)
        self.assertEqual(hearth.build(), load(PROPS / "refuge_hearth.grid.json"))
        self.assertEqual(props.candle_grid(), load(PROPS / "refuge_candle.grid.json"))

    def test_native_sizes_and_colour_budget(self):
        kit = load(ROOT / "assets/source/terrain/floor01/floor01_refuge_kit.json")
        sizes = {p["id"]: tuple(p["rect"][2:]) for p in kit["pieces"]}
        for pid, size in SIZES.items():
            self.assertEqual(sizes[pid], size, pid)
        self.assertEqual(sizes["refuge_hearth"], (80, 80))
        for stem in ("hearth", "candle", "props", "wall", "door"):
            grid = load(PROPS / f"refuge_{stem}.grid.json")
            self.assertLessEqual(len(grid["palette"]), 63, stem)
            ase = PROPS / f"refuge_{stem}.ase"
            self.assertTrue(ase.is_file(), stem)

    def test_palette_is_the_kit_palette(self):
        kit_colours = {c for ramp in props.RAMPS.values() for c in ramp}
        for stem in ("hearth", "candle", "props", "wall", "door"):
            for p in load(PROPS / f"refuge_{stem}.grid.json")["palette"]:
                self.assertIn(p["rgba"][:7], kit_colours, f"{stem} {p['name']}")

    def test_hearth_surround_identical_and_frame0_fire_frozen(self):
        grid = load(PROPS / "refuge_hearth.grid.json")
        self.assertEqual(len(grid["frames"]), 4)
        self.assertEqual([f["duration_ms"] for f in grid["frames"]], [120] * 4)
        names = {p["key"]: p["name"] for p in grid["palette"]}

        def cel(i, layer):
            return [[names.get(c, ".") for c in row] for row in grid["frames"][i]["cels"][layer]]

        for i in (1, 2, 3):
            self.assertEqual(cel(i, "surround"), cel(0, "surround"))
            self.assertNotEqual(cel(i, "fire"), cel(0, "fire"))
        # the frozen frame-0 key is enforced by validator V09 (python3 -m scripts.pixelart validate)

    def test_fire_loop_rules(self):
        # centre tongue never below 20 px; sparks 1-3 per frame; no spark pixel repeats in the next frame (loop wrap)
        for f in range(4):
            self.assertTrue(1 <= len(hearth.SPARKS[f]) <= 3)
            self.assertFalse(set(hearth.SPARKS[f]) & set(hearth.SPARKS[(f + 1) % 4]))
            dh, dl = zip(*hearth.FLICKER[f])
            self.assertTrue(all(-4 <= d <= 3 for d in dh) and all(abs(v) <= 2 for v in dl))
            self.assertGreaterEqual(hearth.TONGUES[0][2] + hearth.FLICKER[f][0][0], 20)

    def test_door_inside_matches_the_guild_portal_geometry(self):
        guild = grid_images(load_grid_file(PROPS / "guild_exit.grid.json"))[0]
        wall = grid_images(load_grid_file(PROPS / "refuge_door.grid.json"))[0]
        kit = load(ROOT / "assets/source/terrain/floor01/floor01_refuge_kit.json")
        opening = load(ROOT / "assets/source/terrain/floor01/floor01_opening_kit.json")
        rect = {p["id"]: p["rect"] for p in kit["pieces"] + opening["pieces"]}
        for ours, theirs in (("refuge_door_inside_closed", "guild_portal_closed"),
                             ("refuge_door_inside_open", "guild_portal_open")):
            a, b = rect[ours], rect[theirs]
            self.assertEqual(a[2:], b[2:])
            self.assertEqual(tuple(a[2:]), (112, 136))
            ca = wall.crop((a[0], a[1], a[0] + 112, a[1] + 136)).getchannel("A")
            cb = guild.crop((b[0], b[1], b[0] + 112, b[1] + 136)).getchannel("A")
            self.assertEqual(ca.tobytes(), cb.tobytes(), f"{ours} silhouette (arch centre 56, spring 44)")

    def test_window_gallery_stays_at_or_below_gal_5(self):
        grid = load(PROPS / "refuge_wall.grid.json")
        for p in grid["palette"]:
            if p["name"].startswith("gal_"):
                self.assertLessEqual(int(p["name"][4:]), 5, p["name"])

    def test_room_is_not_in_the_playtest_manifest(self):
        rooms = load(ROOT / "assets/levels/f01_late/manifest.json")["rooms"]
        self.assertNotIn("refuge_guild", [r["id"] for r in rooms])


if __name__ == "__main__":
    unittest.main()
