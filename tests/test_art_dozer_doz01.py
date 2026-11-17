"""Contract tests for DOZ-01 (Goblin Murder Dozer): the frozen Opus keys, frame sizes, pivots, palette,
colour counts, lint, the full Sonnet frame set (wheel, tread, riders, smoke, spark), the rig contract and the
readability rules of docs/02-art/doz-01-dozer-spec.md."""
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
MACHINE = ("dozer_hull_h120", "dozer_front_cab_h120", "dozer_front_tub_h120", "dozer_wheel_h120", "dozer_tread_h120",
           "dozer_yoke_h120")
ALL = MACHINE + ("dozer_riders_h36", "dozer_smoke", "dozer_size_study", "vfx_hit_spark_metal")
FIXTURE = ROOT / "tests/fixtures/dozer_doz01_keys.json"
VFX = ROOT / "assets/source/vfx"
RIDER_TAGS = [tag for tag, _part, _fn in ad.RIDER_KEYS]
WEAPON = set("xyFGHVv")
GOBLIN_SKIN = set("asml")
FIRE = set("jknqz")
PALETTE = {k: c for k, _n, c, _a in ad.ALL_PALETTE}


def load(stem: str) -> dict:
    return pg.load_grid((VFX if stem.startswith("vfx_") else DOZER) / f"{stem}.grid.json")


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


def frame_tags(grid: dict) -> list[tuple[str, int]]:
    """Per-frame tags (from == to) in frame order; range tags such as ``spin`` or ``roll`` are skipped."""
    return [(t["name"], t["from"]) for t in grid["tags"] if t["from"] == t["to"]]


def components(px: set[tuple[int, int]]) -> int:
    """8-connected components of a pixel set."""
    seen, n = set(), 0
    for p in px:
        if p in seen:
            continue
        n += 1
        stack = [p]
        while stack:
            q = stack.pop()
            if q in seen:
                continue
            seen.add(q)
            stack += [(q[0] + dx, q[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (q[0] + dx, q[1] + dy) in px]
    return n


def fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def part_px(grid: dict, i: int, anchor, pivot=None) -> dict[tuple[int, int], str]:
    """Pixels of one frame in rig coordinates (x forward from O, y up), pivot placed on the anchor."""
    vx, vy = pivot or grid["pivot"]
    ax, ay = anchor
    rows = flat(grid, i)
    return {(ax + x - vx, ay - (y - vy)): c for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def rig() -> dict:
    return json.loads((DOZER / "dozer_rig_h120.json").read_text(encoding="utf-8"))


def front_rig_px() -> dict[tuple[int, int], str]:
    """The hull front as the rig draws it: the cab rail box and the rider tub box."""
    out: dict[tuple[int, int], str] = {}
    for part in ("hull_front_cab", "hull_front_tub"):
        p = next(q for q in rig()["parts"] if q["part"] == part)
        g = load(p["atlas"])
        out.update(part_px(g, tag_index(g, p["tag"]), p["anchor"]))
    return out


def fixture_px(key: str, stem: str) -> dict[tuple[int, int], str]:
    meta = fixture()["meta"][stem]
    vx, vy = meta["pivot"]
    rows = fixture()["keys"][key]
    return {(x - vx, vy - y): c for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


class FrozenKeys(unittest.TestCase):
    """The Opus keys are frozen in tests/fixtures/dozer_doz01_keys.json. The execution may add only the
    documented deltas (spec section 15)."""

    def test_unchanged_keys_are_pixel_identical(self) -> None:
        fix = fixture()["keys"]
        same = [("dozer_hull_h120", "body"), ("dozer_wheel_h120", "phase_0"), ("dozer_tread_h120", "belt_0"),
                ("dozer_smoke", "black_0"), ("dozer_smoke", "black_2"), ("dozer_smoke", "steam_0"),
                ("dozer_smoke", "steam_2")] + [("dozer_riders_h36", tag) for tag in (
                    "engineer_point", "engineer_lever_pull", "engineer_lurch_back", "engineer_lurch_forward",
                    "rider_a_lurch_back", "rider_a_lurch_forward", "rider_b_hold", "rider_b_lurch_back",
                    "rider_b_lurch_forward")]
        old_tag = {"phase_0": "spin", "belt_0": "roll"}
        for stem, tag in same:
            g = load(stem)
            got = ["".join(r) for r in flat(g, tag_index(g, tag))]
            self.assertEqual(got, fix[f"{stem}:{old_tag.get(tag, tag)}"], f"{stem}:{tag} is a frozen Opus key")

    def test_fixture_covers_every_key(self) -> None:
        self.assertEqual(len(fixture()["keys"]), 19)

    def test_hull_front_differs_only_by_the_cab_posts(self) -> None:
        old = fixture_px("dozer_hull_h120:front", "dozer_hull_h120")
        new = front_rig_px()
        diff = {p for p in set(old) | set(new) if old.get(p) != new.get(p)}
        self.assertTrue(diff)
        self.assertTrue(all((-75 <= x <= -73 or -18 <= x <= -16) and 85 <= y <= 101 for x, y in diff), sorted(diff))
        for x in (-74, -73, -18, -17):          # the 2 px iron-2 posts at the rail ends
            self.assertTrue(all(new[(x, y)] == "2" for y in range(86, 101)), x)

    def test_trim_keeps_every_front_pixel_in_place(self) -> None:
        full = despeckle_front()
        self.assertEqual({p: c for p, c in front_rig_px().items()}, full)
        sizes = [load(s)["canvas"] for s in ("dozer_front_cab_h120", "dozer_front_tub_h120")]
        self.assertLess(sum(w * h for w, h in sizes), 175 * 160 // 2, "trimmed boxes are well under the untrimmed frame")

    def test_yoke_polish_recolours_inside_the_silhouette(self) -> None:
        old = fixture_px("dozer_yoke_h120:static", "dozer_yoke_h120")
        g = load("dozer_yoke_h120")
        new = part_px(g, 0, (0, 0))
        self.assertEqual(set(old), set(new), "silhouette unchanged")
        changed = {p: new[p] for p in new if new[p] != old[p]}
        self.assertTrue(set(changed.values()) <= set("17EW"), set(changed.values()))
        counts = {k: list(changed.values()).count(k) for k in "17EW"}
        self.assertGreaterEqual(counts["1"], 40, "2 px iron-1 underside")
        self.assertGreaterEqual(counts["7"], 3, "second rivet row")
        self.assertTrue(counts["E"] >= 3 and counts["W"] >= 3, "rust streak E over W")
        x, y = g["pivot"]
        self.assertEqual(flat(g, 0)[y][x], "7")

    def test_yoke_streak_and_rivet_rows(self) -> None:
        rows = flat(load("dozer_yoke_h120"), 0)
        sevens = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c == "7"]
        row1 = [p for p in sevens if p[1] == 18]
        self.assertGreaterEqual(len(row1), 4, "the key's rivet row")
        row2 = [p for p in sevens if p[1] != 18 and p != (54, 30)]
        self.assertGreaterEqual(len(row2), 3, "the new diagonal rivet row")
        col = [rows[y][29] for y in range(14, 34)]
        run = "".join(col)
        self.assertIn("EEEEWWWW", run.replace("3", "").replace("5", "") or run)

    def test_rider_a_stick_fix_only_changes_three_pixels(self) -> None:
        old = fixture()["keys"]["dozer_riders_h36:rider_a_brandish"]
        g = load("dozer_riders_h36")
        new = ["".join(r) for r in flat(g, tag_index(g, "rider_a_brandish"))]
        diff = [(x, y, old[y][x], new[y][x]) for y in range(56) for x in range(64) if old[y][x] != new[y][x]]
        self.assertEqual(sorted(diff), [(10, 23, "K", "y"), (13, 26, "K", "y"), (14, 26, "x", "y")])


def despeckle_front() -> dict[tuple[int, int], str]:
    w, h, ox, fy = ad.hull_canvas(1.0)
    full = ad.despeckle(ad.hull_layer("front").key)
    return {(x - math.floor(ox + ad.O_X), fy - y): c for y, r in enumerate(full) for x, c in enumerate(r) if c != "."}


class FramesAndPivots(unittest.TestCase):
    def test_hull(self) -> None:
        g = load("dozer_hull_h120")
        self.assertEqual(g["canvas"], [175, 160])
        self.assertEqual(g["pivot"], [87, 159])
        self.assertEqual([t["name"] for t in g["tags"]], ["body"])
        body = opaque(flat(g, 0))
        dome_col = g["pivot"][0] + 79 - 74          # dome centre column (design x 79, O at x 74)
        y = g["pivot"][1] - 100                      # inside the boiler; scan up to the dome's outline
        while (dome_col, y - 1) in body:
            y -= 1
        self.assertEqual(g["pivot"][1] - y, 120, "boiler dome top (outline row) is the 120 px body height")

    def test_front_boxes(self) -> None:
        cab, tub = load("dozer_front_cab_h120"), load("dozer_front_tub_h120")
        self.assertEqual((cab["canvas"], cab["pivot"]), ([60, 21], [0, 20]))
        self.assertEqual((tub["canvas"], tub["pivot"]), ([65, 82], [0, 81]))
        r = rig()
        anchors = {p["part"]: p["anchor"] for p in r["parts"]}
        self.assertEqual(anchors["hull_front_cab"], [-75, 85])
        self.assertEqual(anchors["hull_front_tub"], [19, 23])
        for g in (cab, tub):                         # trimmed: every edge of the box touches an opaque pixel
            px = opaque(flat(g, 0))
            w, h = g["canvas"]
            self.assertTrue(any(x == 0 for x, _ in px) and any(x == w - 1 for x, _ in px))
            self.assertTrue(any(y == 0 for _, y in px) and any(y == h - 1 for _, y in px))
        front = front_rig_px()
        self.assertEqual(max(y for _x, y in front), 105, "rail top outline")
        self.assertEqual(max(y for (x, y) in front if x > 0), 104, "tub rim top outline")

    def test_wheel_frames(self) -> None:
        g = load("dozer_wheel_h120")
        self.assertEqual((g["canvas"], g["pivot"]), ([99, 99], [49, 49]))
        self.assertEqual([n for n, _i in frame_tags(g)], ["phase_0", "phase_1", "phase_2", "phase_3", "smear"])
        self.assertEqual(next((t["from"], t["to"]) for t in g["tags"] if t["name"] == "spin"), (0, 3))
        self.assertEqual(len({str(flat(g, i)) for i in range(5)}), 5, "all wheel frames are distinct")

    def test_wheel_bottom_spike_on_the_floor(self) -> None:
        g = load("dozer_wheel_h120")
        lowest = max(y for _x, y in opaque(flat(g, 0)))
        self.assertEqual(g["pivot"][1] + ad.AXLE[1], lowest, "phase 0: lowest spike outline on the contact row")
        for i in range(1, 4):
            self.assertLessEqual(max(y for _x, y in opaque(flat(g, i))), g["pivot"][1] + ad.AXLE[1],
                                 f"phase {i}: no spike below the floor")

    def test_tread_frames(self) -> None:
        g = load("dozer_tread_h120")
        self.assertEqual((g["canvas"], g["pivot"]), ([151, 34], [75, 33]))
        self.assertEqual([n for n, _i in frame_tags(g)], [f"belt_{i}" for i in range(9)])
        self.assertEqual(next((t["from"], t["to"]) for t in g["tags"] if t["name"] == "roll"), (0, 8))

    def test_yoke_pivot_is_the_axle(self) -> None:
        g = load("dozer_yoke_h120")
        self.assertEqual(g["canvas"], [64, 44])
        x, y = g["pivot"]
        self.assertEqual(flat(g, 0)[y][x], "7", "axle bolt glint on the pivot pixel")

    def test_riders(self) -> None:
        g = load("dozer_riders_h36")
        self.assertEqual(g["canvas"], [64, 56])
        self.assertEqual(g["pivot"], [28, 55])
        names = [n for n, _i in frame_tags(g)]
        self.assertEqual(names, [
            "engineer_point", "engineer_jab", "engineer_hold", "engineer_idle_1", "engineer_lever_pull",
            "engineer_lurch_back", "engineer_lurch_forward",
            "rider_a_hold", "rider_a_idle_1", "rider_a_brandish", "rider_a_yell_1", "rider_a_lurch_back",
            "rider_a_lurch_forward",
            "rider_b_hold", "rider_b_idle_1", "rider_b_brandish", "rider_b_yell_1", "rider_b_lurch_back",
            "rider_b_lurch_forward"])
        self.assertEqual((len([n for n in names if n.startswith("engineer")]),
                          len([n for n in names if n.startswith("rider_a")]),
                          len([n for n in names if n.startswith("rider_b")])), (7, 6, 6))
        self.assertEqual(len({str(flat(g, i)) for i in range(19)}), 19, "every rider frame is distinct")

    def test_smoke_frames(self) -> None:
        g = load("dozer_smoke")
        self.assertEqual(g["canvas"], [24, 24])
        self.assertEqual(g["pivot"], [12, 12])
        self.assertEqual([n for n, _i in frame_tags(g)], ad.SMOKE_NAMES)
        widths, comps, count = {}, {}, {}
        for name, i in frame_tags(g):
            rows = flat(g, i)
            keys = {c for r in rows for c in r} - {"."}
            kind = name.split("_")[0]
            ramp = set("+=-:") if kind == "black" else set(";/*")
            self.assertTrue(keys <= ramp, (name, keys))                   # no outline, own ramp only
            px = opaque(rows)
            widths[name] = max(x for x, _ in px) - min(x for x, _ in px) + 1
            comps[name], count[name] = components(px), len(px)
            dark = "+" if kind == "black" else ";"
            self.assertIn(dark, rows[max(y for _x, y in px)], f"{name}: 1 px darker underside")
            self.assertTrue(all(any(c == dark for c in r) for r in [rows[max(y for _x, y in px)]]))
        self.assertTrue(5 <= widths["black_0"] <= 7, widths)                 # the 6 px spawn
        self.assertTrue(9 <= widths["black_1"] <= 11, widths)                # 10 px
        self.assertTrue(15 <= widths["black_2"] <= 17, widths)               # the 16 px puff
        self.assertEqual((comps["black_3"], comps["black_4"]), (2, 3))       # breaking up, thin
        self.assertTrue(2 <= widths["black_5"] <= 3, widths)                  # last wisp: 2 to 3 px wide
        self.assertLess(widths["steam_2"], widths["black_2"])
        self.assertTrue(widths["steam_0"] < widths["steam_1"] < widths["steam_2"], widths)
        self.assertEqual((comps["steam_3"], comps["steam_4"]), (2, 3))
        self.assertTrue(all(count[f"black_{i}"] > count[f"black_{i + 1}"] for i in (2, 3, 4)), "a puff thins out")

    def test_hit_spark(self) -> None:
        g = load("vfx_hit_spark_metal")
        self.assertEqual((g["canvas"], g["pivot"]), ([16, 16], [8, 8]))
        self.assertEqual([n for n, _i in frame_tags(g)], ["star", "burst", "fade"])
        star, burst, fade = (flat(g, i) for i in range(3))
        self.assertEqual({c for r in star for c in r} - {"."}, {"6", "7"})
        self.assertTrue(all(star[y][x] == "7" for y in (7, 8, 9) for x in (7, 8, 9)), "7 core on the pivot")
        arms = {(x, y) for y, r in enumerate(star) for x, c in enumerate(r) if c == "6"}
        self.assertEqual(len(arms), 20)
        self.assertEqual({c for r in burst for c in r} - {"."}, {"5"})
        self.assertEqual(components(opaque(burst)), 6, "6 short streaks (spec: 4 to 6)")
        self.assertEqual({c for r in fade for c in r} - {"."}, {"4"})
        self.assertTrue(2 <= len(opaque(fade)) <= 3)

    def test_size_study(self) -> None:
        g = load("dozer_size_study")
        self.assertEqual([t["name"] for t in g["tags"]], ["h96", "h120", "h150"])
        heights = []
        for i in range(3):
            px = opaque(flat(g, i))
            self.assertEqual(max(y for _x, y in px), g["pivot"][1], "machine stands on the contact row")
            heights.append(g["pivot"][1] + 1 - min(y for _x, y in px))
        self.assertTrue(heights[0] < heights[1] < heights[2], heights)


class Timing(unittest.TestCase):
    def test_frame_ticks_match_the_grids_and_the_manifests(self) -> None:
        for stem, ticks in ad.FRAME_TICKS.items():
            g = load(stem)
            self.assertEqual(len(g["frames"]), len(ticks), stem)
            self.assertEqual([f["duration_ms"] for f in g["frames"]],
                             [100 if t is None else round(t * 1000 / 60) for t in ticks], stem)
            m = json.loads((ROOT / f"assets/manifests/{stem}.json").read_text(encoding="utf-8"))
            self.assertEqual([f["ticks"] for f in m["dozer"]["frames"]], ticks, stem)

    def test_spec_tick_table(self) -> None:
        ticks = dict(zip([n for n, _i in frame_tags(load("dozer_riders_h36"))], ad.FRAME_TICKS["dozer_riders_h36"]))
        for n in ("engineer_point", "engineer_jab"):
            self.assertEqual(ticks[n], 8, n)
        for who in ("engineer", "rider_a", "rider_b"):
            self.assertEqual((ticks[f"{who}_hold"], ticks[f"{who}_idle_1"]), (16, 16), who)
            self.assertEqual(ticks[f"{who}_lurch_back"], 8, who)
            self.assertEqual(ticks[f"{who}_lurch_forward"], 10, who)
        for who in ("rider_a", "rider_b"):
            self.assertEqual((ticks[f"{who}_brandish"], ticks[f"{who}_yell_1"]), (6, 6), who)
        self.assertEqual(ad.FRAME_TICKS["dozer_tread_h120"], [1] * 9)
        self.assertEqual(ad.FRAME_TICKS["dozer_wheel_h120"][:4], [2] * 4)
        self.assertEqual(ad.FRAME_TICKS["dozer_smoke"], [4] * 6 + [3] * 5)
        self.assertEqual(ad.FRAME_TICKS["vfx_hit_spark_metal"], [2, 2, 2])

    def test_rig_state_table(self) -> None:
        states = rig()["timing"]["states"]
        self.assertEqual([states[s]["ticks"] for s in ("tell", "charge", "brake", "strain")], [36, 90, 24, 90])
        self.assertAlmostEqual(states["cruise"]["px_per_tick"] * 60, 160, delta=0.1)
        self.assertEqual(states["charge"]["px_per_tick"] * 60, 240)
        self.assertEqual((states["tell"]["wheel"], states["cruise"]["wheel"], states["strain"]["wheel"]), (1, 2, 3))
        self.assertEqual(states["strain"]["rattle"]["period"], 3, "strain rattles at double rate")
        self.assertEqual(states["tell"]["stack_puff_every"], 4)
        self.assertEqual(states["cruise"]["stack_puff_every"], 12)
        self.assertEqual(states["charge"]["stack_puff_every"], 8)
        sm = rig()["timing"]["smoke"]
        self.assertEqual((sm["life_ticks"], sm["max_alive"]), (24, 8))
        self.assertEqual(sm["life_ticks"], 6 * ad.SMOKE_TICKS["black"], "6 frames of 4 ticks")


class WheelReadsAsTheHazard(unittest.TestCase):
    """Phase 2 redesign: the lethal spike ring is the brightest part of the machine."""

    def frames(self) -> list[list[list[str]]]:
        g = load("dozer_wheel_h120")
        return [flat(g, i) for i in range(5)]

    def ring_and_drum(self, rows):
        c0 = len(rows) // 2
        ring = [rows[y][x] for y in range(len(rows)) for x in range(len(rows)) if rows[y][x] != "."
                and math.hypot(x - c0, y - c0) > ad.WHEEL_RIM]
        drum = [rows[y][x] for y in range(len(rows)) for x in range(len(rows)) if rows[y][x] != "."
                and math.hypot(x - c0, y - c0) <= ad.WHEEL_RIM]
        return ring, drum

    def test_spike_ring_mean_lightness_in_every_frame(self) -> None:
        for i, rows in enumerate(self.frames()):
            ring, drum = self.ring_and_drum(rows)
            self.assertGreaterEqual(mean_l(ring), 0.5, f"frame {i}: spike ring L* {mean_l(ring):.3f}")
            self.assertLessEqual(mean_l(drum), 0.3, f"frame {i}: drum stays dark")

    def test_spike_ring_is_the_brightest_part_in_every_frame(self) -> None:
        others = {}
        for stem in ("dozer_hull_h120", "dozer_front_cab_h120", "dozer_front_tub_h120", "dozer_tread_h120",
                     "dozer_yoke_h120"):
            g = load(stem)
            others[stem] = [c for i in range(len(g["frames"])) for r in flat(g, i) for c in r]
        for i, rows in enumerate(self.frames()[:4]):
            ring, drum = self.ring_and_drum(rows)
            self.assertGreater(mean_l(ring), mean_l(drum) + 0.1, f"frame {i}: ring vs drum")
            for name, keys in others.items():
                self.assertGreater(mean_l(ring), mean_l(keys) + 0.1, f"frame {i}: spike ring vs {name}")

    def test_spike_clusters_tips_and_outline(self) -> None:
        for i, rows in enumerate(self.frames()[:4]):
            c0 = len(rows) // 2
            px = opaque(rows)
            r_max = max(math.hypot(x - c0, y - c0) for x, y in px)
            self.assertTrue(48 <= r_max <= 50, (i, r_max))
            ring = {(x, y) for x, y in px if math.hypot(x - c0, y - c0) > ad.WHEEL_RIM + 5}
            self.assertEqual(components(ring), ad.SPIKES, f"frame {i}: 12 separate spikes")
            tips = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r)
                    if c != "." and math.hypot(x - c0, y - c0) > ad.WHEEL_TIP - 2]
            self.assertTrue(tips)
            self.assertTrue(all(rows[y][x] in "7K" for x, y in tips), f"frame {i}: outer 2 px are 7, outlined in K")

    def test_twelve_fold_symmetry_and_the_loop(self) -> None:
        for i, rows in enumerate(self.frames()[:4]):
            sil = [[c != "." for c in r] for r in rows]
            n = len(sil)
            rot = [[sil[n - 1 - x][y] for x in range(n)] for y in range(n)]
            self.assertEqual(sil, rot, f"frame {i}: 12-fold silhouette maps onto itself under a quarter turn")
        a = ad.wheel_rows(0.0)[0]
        self.assertEqual(a, ad.wheel_rows(30.0)[0], "4 frames of 7.5 degrees loop over the 30 degree spike period")
        for i in range(4):
            self.assertEqual(self.frames()[i], ad.wheel_frame(i))
            self.assertEqual(ad.wheel_rows(7.5 * i)[0], ad.wheel_rows(7.5 * i + 30.0)[0], f"phase {i} loops")
        self.assertEqual(ad.WHEEL_STEP * 4, 30.0)

    def test_hub_rings_and_spokes_stay_concentric(self) -> None:
        for i, rows in enumerate(self.frames()[:4]):
            c0 = len(rows) // 2
            for keys, rmin, rmax, tol in (("SNML", 0, 11, 0.6), ("3", 28.5, 31.5, 0.6)):
                pts = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r)
                       if c in keys and rmin <= math.hypot(x - c0, y - c0) <= rmax]
                self.assertGreater(len(pts), 60, (i, keys))
                mx = sum(p[0] for p in pts) / len(pts)
                my = sum(p[1] for p in pts) / len(pts)
                self.assertLess(math.hypot(mx - c0, my - c0), tol, f"frame {i} {keys}: concentric (no wobble)")
            for k in range(ad.SPIKES):          # every spoke is unbroken from the hub to the inner ring
                a = math.radians(90 - 7.5 * i - 30 * k)
                hits = [rows[c0 - round(r * math.sin(a))][c0 + round(r * math.cos(a))] in "24" for r in range(12, 28)]
                self.assertGreaterEqual(sum(hits), 14, f"frame {i} spoke {k}: {hits}")

    def test_light_is_fixed_between_phases(self) -> None:
        for i, rows in enumerate(self.frames()[:4]):
            c0 = len(rows) // 2
            e = [(x - c0, c0 - y) for y, r in enumerate(rows) for x, c in enumerate(r) if c == "E"]
            self.assertLess(sum(p[0] for p in e) / len(e), -5, f"phase {i}")
            self.assertGreater(sum(p[1] for p in e) / len(e), 5, f"phase {i}")

    def test_charge_smear(self) -> None:
        base, smear = self.frames()[0], self.frames()[4]
        c0 = len(base) // 2
        tips = [(x, y) for y, r in enumerate(base) for x, c in enumerate(r)
                if c == "7" and math.hypot(x - c0, y - c0) > ad.WHEEL_TIP - 2]
        self.assertTrue(tips)
        self.assertTrue(all(smear[y][x] == "6" for x, y in tips), "tips in 6 instead of 7")
        arc = [(x, y) for y in range(len(base)) for x in range(len(base)) if base[y][x] == "." and smear[y][x] == "5"]
        self.assertGreater(len(arc), 100, "1 px iron-5 arcs joining the spike tips")
        self.assertTrue(all(44 < math.hypot(x - c0, y - c0) < 48 for x, y in arc))
        same = [(x, y) for y in range(len(base)) for x in range(len(base))
                if base[y][x] != "." and smear[y][x] != base[y][x] and (x, y) not in tips]
        self.assertFalse(same, "everything else of phase 0 is unchanged")
        ring, _drum = self.ring_and_drum(smear)
        self.assertGreaterEqual(mean_l(ring), 0.5)


class TreadBelt(unittest.TestCase):
    def frames(self) -> list[list[list[str]]]:
        g = load("dozer_tread_h120")
        return [flat(g, i) for i in range(9)]

    def test_9_distinct_phases_with_a_contact_row(self) -> None:
        fr = self.frames()
        self.assertEqual(len({str(f) for f in fr}), 9)
        for i, rows in enumerate(fr):
            self.assertEqual(max(y for _x, y in opaque(rows)), 33, f"phase {i}: bottom run on contact row 33")
            self.assertGreater(sum(c != "." for c in rows[33]), 60, f"phase {i}: contact row is populated")
            self.assertEqual(rows, ad.tread_frame(i))

    def test_cleats_on_the_bottom_run_differ_by_at_most_one(self) -> None:
        def cleats(rows) -> int:
            n, run = 0, 0
            for c in rows[33][20:131]:
                run = run + 1 if c == "2" else 0
                n += run == 3
            return n
        counts = [cleats(f) for f in self.frames()]
        self.assertLessEqual(max(counts) - min(counts), 1, counts)
        self.assertGreaterEqual(min(counts), 9, counts)

    def test_each_road_wheel_hub_has_one_off_centre_glint_bolt(self) -> None:
        xa, ro = 1.5 + 16.0, 16.0
        xb = 151 - 1.5 - ro
        centres = [(xa + (xb - xa) * f - 0.5, 17.5 + 2.5 - 0.5) for f in (0.2, 0.4, 0.6, 0.8)]
        for i, rows in enumerate(self.frames()):
            sevens = [(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c == "7"]
            for cx, cy in centres:
                near = [p for p in sevens if math.hypot(p[0] - cx, p[1] - cy) <= 5.5]
                self.assertEqual(len(near), 1, f"phase {i}: hub at {cx:.0f}")
                self.assertGreaterEqual(math.hypot(near[0][0] - cx, near[0][1] - cy), 3.0, "off-centre")

    def test_end_wheels_do_not_pop_at_the_wrap(self) -> None:
        fr = self.frames()

        def end_diff(a, b) -> int:
            return sum(a[y][x] != b[y][x] for y in range(34) for x in list(range(0, 28)) + list(range(123, 151)))
        steps = [end_diff(fr[i], fr[i + 1]) for i in range(8)]
        wrap = end_diff(fr[8], fr[0])
        self.assertLessEqual(wrap, max(steps) + 6, (steps, wrap))


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
        for stem in MACHINE + ("dozer_smoke", "vfx_hit_spark_metal"):
            grid = load(stem)
            used = {e["key"] for e in grid["palette"]}
            self.assertFalse(used & GOBLIN_SKIN, stem)

    def test_emissive_only_firebox_and_lamp(self) -> None:
        for stem in ("dozer_wheel_h120", "dozer_tread_h120", "dozer_riders_h36", "dozer_smoke",
                     "dozer_front_cab_h120", "dozer_front_tub_h120", "vfx_hit_spark_metal"):
            self.assertFalse({e["key"] for e in load(stem)["palette"]} & FIRE, stem)
        hull = load("dozer_hull_h120")
        fire = [(x, y) for x, y in opaque(flat(hull, 0)) if flat(hull, 0)[y][x] in FIRE]
        self.assertTrue(fire)
        xs = [x for x, _ in fire]
        self.assertLessEqual(max(xs) - min(xs), 14, "fire colours stay inside the firebox door")

    def test_iron_dominates_the_hull(self) -> None:
        counts: dict[str, int] = {}
        for stem in ("dozer_hull_h120", "dozer_front_cab_h120", "dozer_front_tub_h120"):
            for r in flat(load(stem), 0):
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
        self.rig = rig()

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

    def test_rig_parts_match_the_manifests(self) -> None:
        for p in self.rig["parts"]:
            m = json.loads((ROOT / f"assets/manifests/{p['atlas']}.json").read_text(encoding="utf-8"))
            self.assertEqual(m["frame_size"], p["frame_size"], p["part"])
            self.assertEqual(m["pivot"], p["pivot"], p["part"])

    def rider_frames(self, name: str):
        riders = load("dozer_riders_h36")
        return riders, [(n, i) for n, i in frame_tags(riders) if n.startswith(name + "_")]

    def test_riders_below_the_rim_are_hidden(self) -> None:
        """Every rider body pixel below the occluder row, in every frame, is covered by the hull front (the two
        trimmed boxes). Weapon pixels are exempt: a flung pineapple may hang over the tub's back edge."""
        front_px = set(front_rig_px())
        for name in ("engineer", "rider_a", "rider_b"):
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim = self.rig["occluder_rows"][name]
            ax, ay = part["anchor"]
            riders, frames = self.rider_frames(name)
            self.assertEqual(len(frames), 7 if name == "engineer" else 6, name)
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
                    self.assertIn((ax + (x - 28), ay + (55 - y)), front_px, f"{tag} pixel {(x, y)} pokes out under the rim")

    def test_heads_clear_the_rim(self) -> None:
        riders = load("dozer_riders_h36")
        for name in ("engineer", "rider_a", "rider_b"):
            part = next(p for p in self.rig["parts"] if p["part"] == name)
            rim_row = 55 - (self.rig["occluder_rows"][name] - part["anchor"][1])
            _g, frames = self.rider_frames(name)
            for tag, i in frames:
                if "lurch" in tag:
                    continue
                eyes = [y for y, r in enumerate(flat(riders, i)) if "e" in r]
                if tag == "engineer_lever_pull":
                    continue
                self.assertTrue(eyes and max(eyes) < rim_row, f"{tag}: eyes sit above the rim")

    def test_rider_pixels_come_from_gob01(self) -> None:
        """Visible rows (above the always-hidden rows) equal the seated GOB-01 pixels (merged frames, stick fixed)."""
        riders = load("dozer_riders_h36")
        cut = ad.HIDDEN_FROM_ROW
        for tag, variant, frame in (("rider_a_hold", "a", 0), ("rider_a_brandish", "a", 11), ("rider_b_hold", "b", 0),
                                    ("rider_b_brandish", "b", 11), ("rider_a_lurch_back", "a", 16),
                                    ("rider_b_lurch_back", "b", 16)):
            got = flat(riders, tag_index(riders, tag))[:cut]
            self.assertEqual(got, ad._shift(ad._goblin(variant, frame), 0, ad.SEAT_SHIFT)[:cut], tag)

    def test_idle_1_is_rows_0_to_46_one_px_lower(self) -> None:
        riders = load("dozer_riders_h36")
        for who in ("engineer", "rider_a", "rider_b"):
            a = flat(riders, tag_index(riders, f"{who}_hold"))
            b = flat(riders, tag_index(riders, f"{who}_idle_1"))
            self.assertEqual(b[1:47], a[0:46], who)
            self.assertEqual(b[0], ["."] * 64)
            self.assertEqual(b[48:], a[48:], f"{who}: the always-hidden rows stay")

    def test_yell_1_pineapple_higher_and_mouth_wider(self) -> None:
        riders = load("dozer_riders_h36")
        for v in ("a", "b"):
            y0 = flat(riders, tag_index(riders, f"rider_{v}_brandish"))
            y1 = flat(riders, tag_index(riders, f"rider_{v}_yell_1"))
            top = lambda rows: min(y for y, r in enumerate(rows) if set(r) & set("FGHVv"))  # noqa: E731
            self.assertEqual(top(y1), top(y0) - 2, "pineapple 2 px higher")
            mouth = lambda rows: sum(r.count("o") + r.count("w") for r in rows[38:46])  # noqa: E731
            self.assertGreaterEqual(mouth(y1), mouth(y0) + 3, "mouth 1 px wider on its three rows")
            # the stick stays continuous: y pixels in column 10 from the fruit down to the fist, K on both sides
            col = [y for y in range(len(y1)) if y1[y][10] == "y"]
            self.assertEqual(col, list(range(col[0], col[-1] + 1)), "continuous 1 px stick")
            self.assertGreaterEqual(len(col), 3)
            self.assertTrue(all(y1[y][9] == "K" and y1[y][11] == "K" for y in col[1:]), "K edges")

    def test_rider_stick_has_no_gap_to_the_fist(self) -> None:
        riders = load("dozer_riders_h36")
        for v in ("a", "b"):
            rows = flat(riders, tag_index(riders, f"rider_{v}_brandish"))
            self.assertEqual((rows[23][10], rows[26][13], rows[26][14]), ("y", "y", "y"))
            self.assertIn(rows[22][10], "FG", "the stick enters the fruit")
            self.assertEqual((rows[24][11], rows[25][12]), ("m", "m"), "fist pixels")
            self.assertEqual(rows[24][10], "K", "K edge between the stick and the fist")
            # stick -> fist -> butt: each y pixel touches the fist (8-neighbourhood, through its outline)
            near = lambda x, y: any(rows[y + dy][x + dx] == "m" for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2))  # noqa: E731
            self.assertTrue(near(10, 23) and near(13, 26))

    def test_lurch_keys(self) -> None:
        riders = load("dozer_riders_h36")
        for v in ("a", "b"):
            fwd = flat(riders, tag_index(riders, f"rider_{v}_lurch_forward"))
            self.assertFalse(any("e" in r for r in fwd), "white eyes when thrown forward")
            self.assertTrue(any("w" in r for r in fwd))
            back = flat(riders, tag_index(riders, f"rider_{v}_lurch_back"))
            self.assertFalse(any("e" in r for r in back), "GOB-01 hurt eyes when thrown back")
        eng_fwd = flat(riders, tag_index(riders, "engineer_lurch_forward"))
        self.assertFalse(any("e" in r for r in eng_fwd), "the pot slides over the engineer's eyes")
        point = flat(riders, tag_index(riders, "engineer_point"))
        hop = flat(riders, tag_index(riders, "engineer_lurch_back"))
        top = lambda rows: min(y for y, r in enumerate(rows) if set(r) & set("3456"))  # noqa: E731
        self.assertLessEqual(top(hop), top(point) - 3, "the pot hops off his head")

    def test_engineer_point_jab_and_hold(self) -> None:
        riders = load("dozer_riders_h36")
        point = flat(riders, tag_index(riders, "engineer_point"))
        jab = flat(riders, tag_index(riders, "engineer_jab"))
        hold = flat(riders, tag_index(riders, "engineer_hold"))
        right = lambda rows: max(x for r in rows for x, c in enumerate(r) if c != ".")  # noqa: E731
        self.assertEqual(right(jab), right(point) + 1, "the finger moves 1 px forward")
        self.assertLess(right(hold), right(point) - 10, "idle 0 has no pointing arm")
        self.assertEqual(hold[:38], point[:38], "same head, pot and body above the arm")

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
