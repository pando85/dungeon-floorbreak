"""Key-level contract tests for DOZ-01 phase 1 (Goblin Murder Dozer): frame sizes, pivots, palette,
colour counts, lint, the rig contract and the readability rules of docs/02-art/doz-01-dozer-spec.md."""
from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_dozer_doz01 as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

DOZER = ROOT / "assets/source/enemies/dozer"
MACHINE = ("dozer_hull_h120", "dozer_wheel_h120", "dozer_tread_h120", "dozer_yoke_h120")
ALL = MACHINE + ("dozer_riders_h36", "dozer_size_study")
GOBLIN_SKIN = set("asml")
FIRE = set("jknqz")
PALETTE = {k: c for k, _n, c, _a in ad.ALL_PALETTE}


def load(stem: str) -> dict:
    return pg.load_grid(DOZER / f"{stem}.grid.json")


def flat(grid: dict, i: int) -> list[list[str]]:
    return pg.composite_keys(grid, i)


def opaque(rows) -> set[tuple[int, int]]:
    return {(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


class Sources(unittest.TestCase):
    def test_generator_reproduces_the_committed_sources(self) -> None:
        for path, text in ad.build_all().items():
            self.assertTrue(path.is_file(), path)
            self.assertEqual(path.read_text(encoding="utf-8"), text, f"{path.name} differs from the generator")

    def test_manifests_match_their_grids(self) -> None:
        for stem in ALL:
            grid = load(stem)
            m = json.loads((ROOT / f"assets/manifests/{stem}.json").read_text(encoding="utf-8"))
            self.assertEqual(m["frame_size"], grid["canvas"], stem)
            self.assertEqual(m["pivot"], grid["pivot"], stem)
            self.assertEqual(m["animation_tags"], [t["name"] for t in grid["tags"]], stem)
            self.assertEqual(m["palette_id"], "dozer-floor01-v0")
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            self.assertIn("ART-PUBLICATION-ALL-V1", m["license"])
            self.assertIn("Claude Opus", m["provenance"]["generator"])


class FramesAndPivots(unittest.TestCase):
    def test_hull(self) -> None:
        g = load("dozer_hull_h120")
        self.assertEqual(g["canvas"], [175, 160])
        self.assertEqual(g["pivot"], [87, 159])
        self.assertEqual([t["name"] for t in g["tags"]], ["body", "front"])
        body, front = opaque(flat(g, 0)), opaque(flat(g, 1))
        self.assertTrue(body and front)
        # body height: the boiler dome top is 120 px above the contact row (plus its outline)
        dome_col = g["pivot"][0] + 79 - 74          # dome centre column (design x 79, O at x 74)
        y = g["pivot"][1] - 100                      # inside the boiler; scan up to the dome's outline
        while (dome_col, y - 1) in body:
            y -= 1
        self.assertEqual(g["pivot"][1] - y, 120, "boiler dome top (outline row) is the 120 px body height")

    def test_wheel_bottom_spike_on_the_floor(self) -> None:
        g = load("dozer_wheel_h120")
        lowest = max(y for _x, y in opaque(flat(g, 0)))
        self.assertEqual(g["pivot"][1] + ad.AXLE[1], lowest, "lowest spike outline on the contact row")

    def test_wheel(self) -> None:
        g = load("dozer_wheel_h120")
        w, h = g["canvas"]
        self.assertEqual((w, h), (99, 99))
        self.assertEqual(g["pivot"], [49, 49])
        px = opaque(flat(g, 0))
        cx, cy = g["pivot"]
        r_max = max(math.hypot(x - cx, y - cy) for x, y in px)
        self.assertTrue(48 <= r_max <= 50, r_max)
        # 12 separate spikes in the ring outside the rim
        ring = {(x, y) for x, y in px if math.hypot(x - cx, y - cy) > ad.WHEEL_RIM + 5}
        clusters, seen = 0, set()
        for p in ring:
            if p in seen:
                continue
            clusters += 1
            stack = [p]
            while stack:
                q = stack.pop()
                if q in seen:
                    continue
                seen.add(q)
                stack += [(q[0] + dx, q[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (q[0] + dx, q[1] + dy) in ring]
        self.assertEqual(clusters, ad.SPIKES)

    def test_tread_sits_on_the_contact_row(self) -> None:
        g = load("dozer_tread_h120")
        self.assertEqual(g["canvas"], [151, 34])
        self.assertEqual(g["pivot"], [75, 33])
        px = opaque(flat(g, 0))
        self.assertEqual(max(y for _x, y in px), g["pivot"][1])
        bottom = [x for x, y in px if y == g["pivot"][1]]
        self.assertGreater(len(bottom), 80, "the belt's bottom run (cleats) lies on the contact row")

    def test_yoke_pivot_is_the_axle(self) -> None:
        g = load("dozer_yoke_h120")
        self.assertEqual(g["canvas"], [64, 44])
        x, y = g["pivot"]
        self.assertEqual(flat(g, 0)[y][x], "7", "axle bolt glint on the pivot pixel")

    def test_riders(self) -> None:
        g = load("dozer_riders_h36")
        self.assertEqual(g["canvas"], [64, 56])
        self.assertEqual(g["pivot"], [28, 55])
        self.assertEqual([t["name"] for t in g["tags"]], ["engineer_point", "rider_a_brandish", "rider_b_hold"])

    def test_size_study(self) -> None:
        g = load("dozer_size_study")
        self.assertEqual([t["name"] for t in g["tags"]], ["h96", "h120", "h150"])
        heights = []
        for i in range(3):
            px = opaque(flat(g, i))
            self.assertEqual(max(y for _x, y in px), g["pivot"][1], "machine stands on the contact row")
            heights.append(g["pivot"][1] + 1 - min(y for _x, y in px))
        self.assertTrue(heights[0] < heights[1] < heights[2], heights)


class PaletteAndLint(unittest.TestCase):
    def test_palette_budget(self) -> None:
        self.assertLessEqual(len(ad.ALL_PALETTE), 64, "dozer + goblin palette union")
        self.assertEqual(len({k for k, *_r in ad.ALL_PALETTE}), len(ad.ALL_PALETTE), "unique keys")
        for stem in ALL:
            grid = load(stem)
            self.assertLessEqual(len(grid["palette"]), 64, stem)
            for entry in grid["palette"]:
                self.assertEqual(PALETTE[entry["key"]], entry["rgba"], (stem, entry["key"]))
            used = {c for i in range(len(grid["frames"])) for r in flat(grid, i) for c in r} - {"."}
            self.assertEqual(used, {e["key"] for e in grid["palette"]}, stem)

    def test_env01_tones_reused(self) -> None:
        props = json.loads((ROOT / "assets/source/props/floor01/props.grid.json").read_text(encoding="utf-8"))
        env = {p["rgba"] for p in props["palette"]}
        for key in "cfghi" "LMNSZ":
            self.assertIn(PALETTE[key], env, f"{key} should be an ENV-01 copper/brass tone")

    def test_goblin_green_only_on_goblins(self) -> None:
        for stem in MACHINE:
            grid = load(stem)
            used = {e["key"] for e in grid["palette"]}
            self.assertFalse(used & GOBLIN_SKIN, stem)

    def test_emissive_only_firebox_and_lamp(self) -> None:
        for stem in ("dozer_wheel_h120", "dozer_tread_h120", "dozer_riders_h36"):
            self.assertFalse({e["key"] for e in load(stem)["palette"]} & FIRE, stem)
        hull = load("dozer_hull_h120")
        fire = [(x, y) for x, y in opaque(flat(hull, 0)) if flat(hull, 0)[y][x] in FIRE]
        self.assertTrue(fire)
        xs = [x for x, _ in fire]
        self.assertLessEqual(max(xs) - min(xs), 14, "fire colours stay inside the firebox door")
        self.assertFalse(any(c in FIRE for r in flat(hull, 1) for c in r), "no glow on the front layer")

    def test_iron_dominates_the_hull(self) -> None:
        hull = load("dozer_hull_h120")
        counts: dict[str, int] = {}
        for i in (0, 1):
            for r in flat(hull, i):
                for c in r:
                    if c not in ".K":
                        counts[c] = counts.get(c, 0) + 1
        iron = sum(n for k, n in counts.items() if k in "01234567")
        warm = sum(n for k, n in counts.items() if k in "QWETYcfghi")
        self.assertGreater(iron, warm, "cool iron must dominate so the hull separates from the orange brick")

    def test_lint(self) -> None:
        for stem in ALL:
            report = pg.lint_grid(load(stem))
            self.assertFalse(any(report["isolated_count_per_frame"]), stem)
            limit = 10 if stem in ("dozer_size_study", "dozer_riders_h36") else 0   # GOB-01 rule: under 10
            self.assertTrue(all(n <= limit for n in report["orphan_count_per_frame"]),
                            (stem, report["orphan_count_per_frame"]))


class RigContract(unittest.TestCase):
    def setUp(self) -> None:
        self.rig = json.loads((DOZER / "dozer_rig_h120.json").read_text(encoding="utf-8"))

    def compose(self) -> tuple[list[list[str]], tuple[int, int]]:
        W, H, ox, fy = 300, 180, 96, 179
        out = [["."] * W for _ in range(H)]
        frame_of = {"engineer": 0, "rider_a": 1, "rider_b": 2}
        for p in sorted([p for p in self.rig["parts"] if "tag" in p], key=lambda q: q["z"]):
            grid = load(p["atlas"])
            if p["atlas"] == "dozer_riders_h36":
                i = frame_of[p["part"]]
            else:
                i = next(t["from"] for t in grid["tags"] if t["name"] == p["tag"])
            self.assertEqual(grid["canvas"], p["frame_size"], p["part"])
            self.assertEqual(grid["pivot"], p["pivot"], p["part"])
            dx = ox + p["anchor"][0] - p["pivot"][0]
            dy = fy - p["anchor"][1] - p["pivot"][1]
            for y, row in enumerate(flat(grid, i)):
                for x, c in enumerate(row):
                    if c != ".":
                        out[y + dy][x + dx] = c
        return out, (ox, fy)

    def test_rig_reproduces_the_assembled_machine(self) -> None:
        comp, (ox, fy) = self.compose()
        ref, (rx, ry) = ad.assemble(1.0, ad.riders_all())
        a = {(x - ox, y - fy): c for x, y in opaque(comp) for c in [comp[y][x]]}
        b = {(x - rx, y - ry): c for x, y in opaque(ref) for c in [ref[y][x]]}
        self.assertEqual(a, b)

    def test_riders_below_the_rim_are_hidden(self) -> None:
        comp, (ox, fy) = self.compose()
        riders = load("dozer_riders_h36")
        front = load("dozer_hull_h120")
        hp = front["pivot"]
        front_px = opaque(flat(front, 1))
        frame_of = {"engineer": 0, "rider_a": 1, "rider_b": 2}
        for name, i in frame_of.items():
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim = self.rig["occluder_rows"][name]
            ax, ay = part["anchor"]
            px = opaque(flat(riders, i))
            hidden = [(x, y) for x, y in px if (55 - y) < rim - ay]       # rows below the rim line
            visible = [(x, y) for x, y in px if (55 - y) >= rim - ay]
            self.assertGreater(len(visible), 120, f"{name} must read above the rim")
            for x, y in hidden:
                hx = hp[0] + ax + (x - 28)
                hy = hp[1] - ay + (y - 55)
                self.assertIn((hx, hy), front_px, f"{name} pixel {(x, y)} pokes out under the rim")

    def test_heads_clear_the_rim(self) -> None:
        riders = load("dozer_riders_h36")
        for name, i in {"engineer": 0, "rider_a": 1, "rider_b": 2}.items():
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim_row = 55 - (self.rig["occluder_rows"][name] - part["anchor"][1])
            eyes = [y for y, r in enumerate(flat(riders, i)) if "e" in r]
            self.assertTrue(eyes and max(eyes) < rim_row, f"{name}'s eyes sit above the rim")

    def test_rider_pixels_come_from_gob01(self) -> None:
        riders = load("dozer_riders_h36")
        self.assertEqual(flat(riders, 1), ad._shift(ad._goblin("a", 11), 0, ad.SEAT_SHIFT))
        self.assertEqual(flat(riders, 2), ad._shift(ad._goblin("b", 0), 0, ad.SEAT_SHIFT))

    def test_engineer_has_the_pot_helmet(self) -> None:
        eng = flat(load("dozer_riders_h36"), 0)
        head_top = min(y for y, r in enumerate(eng) if set(r) & GOBLIN_SKIN)
        pot = [(x, y) for y, r in enumerate(eng) for x, c in enumerate(r) if c in "23456" and y < head_top + 8]
        self.assertGreater(len(pot), 80)
        self.assertLess(min(x for x, _ in pot), 18, "the saucepan handle sticks out to the back")

    def test_gameplay_proposal_geometry(self) -> None:
        apex = 440 ** 2 / (2 * 1050)               # Carl's feet at the jump apex (src/prototype.rs)
        ext = self.rig["extents"]
        self.assertGreater(2 * ad.WHEEL_TIP, apex, "the wheel cannot be jumped")
        self.assertGreater(ext["tub_rim_y"], apex, "Carl cannot land on the tub")
        self.assertLess(ext["top_y_stack"], 276 - 60, "stack and smoke stay under the corridor ceiling")
        self.assertEqual(ext["dome_y"], 120)


if __name__ == "__main__":
    unittest.main()
