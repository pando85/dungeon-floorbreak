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
ALL = MACHINE + ("dozer_riders_h36", "dozer_smoke", "dozer_size_study")
RIDER_TAGS = [tag for tag, _part, _fn in ad.RIDER_KEYS]
WEAPON = set("xyFGHVv")
GOBLIN_SKIN = set("asml")
FIRE = set("jknqz")
PALETTE = {k: c for k, _n, c, _a in ad.ALL_PALETTE}


def load(stem: str) -> dict:
    return pg.load_grid(DOZER / f"{stem}.grid.json")


def flat(grid: dict, i: int) -> list[list[str]]:
    return pg.composite_keys(grid, i)


def opaque(rows) -> set[tuple[int, int]]:
    return {(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def tag_index(grid: dict, name: str) -> int:
    return next(t["from"] for t in grid["tags"] if t["name"] == name)


def lstar(rgba: str) -> float:
    """CIELAB lightness L* / 100 of an sRGB colour (D65)."""
    def lin(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (int(rgba[i:i + 2], 16) / 255 for i in (1, 3, 5))
    y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    f = y ** (1 / 3) if y > 0.008856 else 7.787 * y + 16 / 116
    return (116 * f - 16) / 100


def mean_l(keys) -> float:
    keys = [k for k in keys if k not in ".K"]          # material pixels; the plum outline is excluded
    return sum(lstar(PALETTE[k]) for k in keys) / len(keys)


def simulated_apex() -> float:
    """Carl's feet at the top of a jump, integrated like src/prototype.rs (speed before position)."""
    vy, y, best = 440.0, 0.0, 0.0
    while True:
        vy -= 1050.0 / 60
        y += vy / 60
        if vy <= 0:
            return best
        best = max(best, y)


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
        self.assertEqual([t["name"] for t in g["tags"]], [
            "engineer_point", "engineer_lever_pull", "engineer_lurch_back", "engineer_lurch_forward",
            "rider_a_brandish", "rider_a_lurch_back", "rider_a_lurch_forward",
            "rider_b_hold", "rider_b_lurch_back", "rider_b_lurch_forward"])

    def test_smoke_keys(self) -> None:
        g = load("dozer_smoke")
        self.assertEqual(g["canvas"], [24, 24])
        self.assertEqual(g["pivot"], [12, 12])
        self.assertEqual([t["name"] for t in g["tags"]], ["black_0", "black_2", "steam_0", "steam_2"])
        widths = {}
        for t in g["tags"]:
            rows = flat(g, t["from"])
            keys = {c for r in rows for c in r} - {"."}
            ramp = set("+=-:") if t["name"].startswith("black") else set(";/*")
            self.assertTrue(keys <= ramp, (t["name"], keys))          # no outline, own ramp only
            px = opaque(rows)
            widths[t["name"]] = max(x for x, _ in px) - min(x for x, _ in px) + 1
            low = max(y for _x, y in px)
            dark = ramp and ("+" if t["name"].startswith("black") else ";")
            self.assertIn(dark, rows[low], "1 px darker underside")
        self.assertTrue(5 <= widths["black_0"] <= 7, widths)          # the 6 px spawn
        self.assertTrue(15 <= widths["black_2"] <= 17, widths)        # the 16 px puff
        self.assertLess(widths["steam_2"], widths["black_2"])

    def test_tread_has_9_phases_on_the_contact_row(self) -> None:
        self.assertEqual(ad.TREAD_PHASES, 9)
        frames = []
        for phase in range(ad.TREAD_PHASES):
            rows, piv = ad.tread_rows(phase)
            rows = ad.despeckle(rows)
            self.assertEqual(max(y for _x, y in opaque(rows)), piv[1], f"phase {phase} contact row")
            frames.append(rows)
        self.assertEqual(len({str(f) for f in frames}), 9, "9 distinct belt phases")
        hubs = [sum(r.count("7") for r in f) for f in frames]
        self.assertTrue(all(n >= 4 for n in hubs), hubs)               # one lit bolt per road wheel

    def test_size_study(self) -> None:
        g = load("dozer_size_study")
        self.assertEqual([t["name"] for t in g["tags"]], ["h96", "h120", "h150"])
        heights = []
        for i in range(3):
            px = opaque(flat(g, i))
            self.assertEqual(max(y for _x, y in px), g["pivot"][1], "machine stands on the contact row")
            heights.append(g["pivot"][1] + 1 - min(y for _x, y in px))
        self.assertTrue(heights[0] < heights[1] < heights[2], heights)


class WheelReadsAsTheHazard(unittest.TestCase):
    """Phase 2 redesign: the lethal spike ring is the brightest part of the machine."""

    def ring_and_drum(self, phase: float):
        rows, c0 = ad.wheel_rows(phase)
        rows = ad.despeckle(rows)
        ring = [rows[y][x] for y in range(len(rows)) for x in range(len(rows)) if rows[y][x] != "."
                and math.hypot(x - c0, y - c0) > ad.WHEEL_RIM]
        drum = [rows[y][x] for y in range(len(rows)) for x in range(len(rows)) if rows[y][x] != "."
                and math.hypot(x - c0, y - c0) <= ad.WHEEL_RIM]
        return ring, drum, rows

    def test_spike_ring_mean_lightness(self) -> None:
        for i in range(4):
            ring, drum, _rows = self.ring_and_drum(7.5 * i)
            self.assertGreaterEqual(mean_l(ring), 0.5, f"phase {i}: spike ring L* {mean_l(ring):.3f}")
            self.assertLessEqual(mean_l(drum), 0.3, f"phase {i}: drum stays dark")

    def test_spike_ring_is_the_brightest_part(self) -> None:
        ring, drum, _rows = self.ring_and_drum(0.0)
        ring_l = mean_l(ring)
        others = {"wheel drum": drum}
        hull = load("dozer_hull_h120")
        others["hull body"] = [c for r in flat(hull, 0) for c in r]
        others["hull front"] = [c for r in flat(hull, 1) for c in r]
        others["tread"] = [c for r in flat(load("dozer_tread_h120"), 0) for c in r]
        others["yoke"] = [c for r in flat(load("dozer_yoke_h120"), 0) for c in r]
        for name, keys in others.items():
            self.assertGreater(ring_l, mean_l(keys) + 0.1, f"spike ring vs {name}")

    def test_spike_tips_and_outline(self) -> None:
        _ring, _drum, rows = self.ring_and_drum(0.0)
        c0 = len(rows) // 2
        tips = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r)
                if c != "." and math.hypot(x - c0, y - c0) > ad.WHEEL_TIP - 2]
        self.assertTrue(tips)
        self.assertTrue(all(rows[y][x] in "7K" for x, y in tips), "outer 2 px of every spike are 7, outlined in K")

    def test_twelve_fold_loop(self) -> None:
        a, _c0 = ad.wheel_rows(0.0)
        self.assertEqual(a, ad.wheel_rows(30.0)[0], "phase 30 degrees equals phase 0: 4 frames of 7.5 degrees loop")
        # the silhouette (geometry) is 12-fold: a quarter turn maps it onto itself
        sil = [[c != "." for c in r] for r in a]
        n = len(sil)
        rot = [[sil[n - 1 - x][y] for x in range(n)] for y in range(n)]
        self.assertEqual(sil, rot)

    def test_light_is_fixed_between_phases(self) -> None:
        # the drum crescent (E) sits upper-left in every phase
        for i in range(4):
            rows, c0 = ad.wheel_rows(7.5 * i)
            e = [(x - c0, c0 - y) for y, r in enumerate(rows) for x, c in enumerate(r) if c == "E"]
            mx = sum(p[0] for p in e) / len(e)
            my = sum(p[1] for p in e) / len(e)
            self.assertLess(mx, -5, f"phase {i}")
            self.assertGreater(my, 5, f"phase {i}")


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
        for stem in ("dozer_wheel_h120", "dozer_tread_h120", "dozer_riders_h36", "dozer_smoke"):
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
        for p in sorted([p for p in self.rig["parts"] if "tag" in p], key=lambda q: q["z"]):
            grid = load(p["atlas"])
            if p["atlas"] == "dozer_riders_h36":
                i = tag_index(grid, ad.RIDER_SEATED[p["part"]])
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

    def rider_frames(self, name: str):
        riders = load("dozer_riders_h36")
        return riders, [(t["name"], t["from"]) for t in riders["tags"] if t["name"].startswith(name + "_")
                        or (name == "engineer" and t["name"].startswith("engineer"))]

    def test_riders_below_the_rim_are_hidden(self) -> None:
        """Every rider body pixel below the occluder row, in every key, is covered by the hull front.
        Weapon pixels are exempt: a flung pineapple may hang over the tub's back edge."""
        front = load("dozer_hull_h120")
        hp = front["pivot"]
        front_px = opaque(flat(front, 1))
        for name in ("engineer", "rider_a", "rider_b"):
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim = self.rig["occluder_rows"][name]
            ax, ay = part["anchor"]
            riders, frames = self.rider_frames(name)
            self.assertTrue(frames, name)
            for tag, i in frames:
                rows = flat(riders, i)
                px = opaque(rows)
                def weapon(x, y):   # a weapon key, or the K outline touching one (8-neighbourhood)
                    if rows[y][x] in WEAPON:
                        return True
                    return rows[y][x] == "K" and any(
                        0 <= y + dy < 56 and 0 <= x + dx < 64 and rows[y + dy][x + dx] in WEAPON
                        for dx in (-1, 0, 1) for dy in (-1, 0, 1))
                hidden = [(x, y) for x, y in px if (55 - y) < rim - ay and not weapon(x, y)]
                visible = [(x, y) for x, y in px if (55 - y) >= rim - ay]
                self.assertGreater(len(visible), 120, f"{tag} must read above the rim")
                for x, y in hidden:
                    hx = hp[0] + ax + (x - 28)
                    hy = hp[1] - ay + (y - 55)
                    self.assertIn((hx, hy), front_px, f"{tag} pixel {(x, y)} pokes out under the rim")

    def test_heads_clear_the_rim(self) -> None:
        riders = load("dozer_riders_h36")
        for name, tag in ad.RIDER_SEATED.items():
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim_row = 55 - (self.rig["occluder_rows"][name] - part["anchor"][1])
            eyes = [y for y, r in enumerate(flat(riders, tag_index(riders, tag))) if "e" in r]
            self.assertTrue(eyes and max(eyes) < rim_row, f"{name}'s eyes sit above the rim")

    def test_rider_pixels_come_from_gob01(self) -> None:
        """Visible rows (above the always-hidden rows) equal the seated GOB-01 key pixels."""
        riders = load("dozer_riders_h36")
        cut = ad.HIDDEN_FROM_ROW
        for tag, variant, frame in (("rider_a_brandish", "a", 11), ("rider_b_hold", "b", 0),
                                    ("rider_a_lurch_back", "a", 16), ("rider_b_lurch_back", "b", 16)):
            got = flat(riders, tag_index(riders, tag))[:cut]
            self.assertEqual(got, ad._shift(ad._goblin(variant, frame), 0, ad.SEAT_SHIFT)[:cut], tag)

    def test_lurch_keys(self) -> None:
        riders = load("dozer_riders_h36")
        for v in ("a", "b"):
            fwd = flat(riders, tag_index(riders, f"rider_{v}_lurch_forward"))
            hold = flat(riders, tag_index(riders, "rider_a_brandish" if v == "a" else "rider_b_hold"))
            self.assertFalse(any("e" in r for r in fwd), "white eyes when thrown forward")
            self.assertTrue(any("w" in r for r in fwd))
            back = flat(riders, tag_index(riders, f"rider_{v}_lurch_back"))
            self.assertFalse(any("e" in r for r in back), "GOB-01 hurt eyes when thrown back")
            _ = hold
        eng_fwd = flat(riders, tag_index(riders, "engineer_lurch_forward"))
        self.assertFalse(any("e" in r for r in eng_fwd), "the pot slides over the engineer's eyes")
        point = flat(riders, tag_index(riders, "engineer_point"))
        hop = flat(riders, tag_index(riders, "engineer_lurch_back"))
        top = lambda rows: min(y for y, r in enumerate(rows) if set(r) & set("3456"))  # noqa: E731
        self.assertLessEqual(top(hop), top(point) - 3, "the pot hops off his head")

    def test_engineer_lever_pull(self) -> None:
        riders = load("dozer_riders_h36")
        pull = flat(riders, tag_index(riders, "engineer_lever_pull"))
        point = flat(riders, tag_index(riders, "engineer_point"))
        skin = lambda rows: [x for r in rows[:48] for x, c in enumerate(r) if c in GOBLIN_SKIN]  # noqa: E731
        self.assertLess(sum(skin(pull)) / len(skin(pull)), sum(skin(point)) / len(skin(point)),
                        "torso and head lean back")
        mouth = sum(r.count("o") for r in pull)
        self.assertGreaterEqual(mouth, sum(r.count("o") for r in point) + 3, "mouth open")
        knob = (40, 47)                                     # the lever knob under the hands
        self.assertNotEqual(pull[knob[1]][knob[0] - 2], ".", "hands reach the knob")

    def test_engineer_pot(self) -> None:
        """Phase 2 pot: flat top with a light row, a 2 px K notch on the top-front corner, a lip
        1 px wider than the body each side, a 2 px handle at the lip rows rising 2 px to the
        back with a K hanging hole 2 px from its end."""
        eng = flat(load("dozer_riders_h36"), 0)
        top = 26
        self.assertTrue(all(eng[top][x] == "K" for x in range(27, 41)), "flat top outline")
        self.assertTrue(all(eng[27][x] == "6" for x in range(27, 39)), "1 px light row")
        self.assertEqual((eng[27][39], eng[27][40]), ("K", "K"), "notch on the top-front corner")
        self.assertEqual((eng[31][41], eng[31][42], eng[30][41]), ("4", "K", "K"),
                         "lip 1 px wider than the body at the front")
        self.assertEqual((eng[32][26], eng[30][26]), ("3", "K"), "and at the back, where the handle joins it")
        self.assertEqual((eng[29][26], eng[29][41]), ("K", "K"), "body outline")
        self.assertEqual((eng[31][24], eng[32][24]), ("5", "3"), "handle leaves the lip rows")
        self.assertEqual((eng[29][14], eng[30][14]), ("5", "3"), "handle end rises 2 px")
        self.assertEqual(eng[29][16], "K", "hanging hole 2 px from the end")
        self.assertEqual(eng[29][13], "K")
        self.assertTrue(any(c == "e" for c in eng[35]), "eyes stay visible under the lip")

    def test_gameplay_proposal_geometry(self) -> None:
        apex = simulated_apex()                    # src/prototype.rs: speed updated before position
        self.assertAlmostEqual(apex, 88.54, places=1)
        self.assertLess(apex, 90, "documented CHASE-01 target: the lethal-margin test fails at >= 90 px")
        ext = self.rig["extents"]
        wheel = next(p for p in self.rig["parts"] if p["part"] == "wheel")
        lethal_top = wheel["anchor"][1] + 44       # proposed lethal circle r 44 around the axle
        self.assertGreater(lethal_top, apex, "the lethal box reaches above Carl's feet at the apex")
        self.assertGreater(ext["tub_rim_y"], apex, "Carl cannot land on the tub")
        self.assertLessEqual(ext["top_y_stack"] + 8, 164, "ENV-02: chase ceiling >= 164 px clears the stack")
        self.assertGreater(116, ext["tub_rim_y"], "ENV-02: a side passage <= 116 px is too low for the machine")
        self.assertEqual(ext["dome_y"], 120)


if __name__ == "__main__":
    unittest.main()
