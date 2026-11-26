"""Contract tests for the DOZ-02 design keys (Dozer damage states and the stop frame): sources, manifests,
validator configs, canvases and pivots, the pitch rule, the damage overlays, the readability rules of
docs/02-art/doz-02-dozer-damage-spec.md and the damage-state data contract."""
from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "tests"))

import author_dozer_doz01 as ad  # noqa: E402
import author_dozer_doz02 as a2  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from test_art_dozer_doz01 import mean_l  # noqa: E402

DOZER = ROOT / "assets/source/enemies/dozer"
PROPS = ROOT / "assets/source/props/stop_frame"
ENV = ROOT / "assets/source/props/dozer_room"
GOBLIN = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
FIRE = set("jknqz")
BRIGHT_FIRE = set("qnz")
PALETTE = {k: c for k, _n, c, _a in ad.ALL_PALETTE}
EXPECTED = {   # stem: (canvas, pivot, per-frame tags)
    "dozer_wheel_jammed_h120": ([99, 99], [49, 49], ["jammed"]),
    "dozer_hull_damage_h120": ([175, 160], [87, 159], ["pitch_0", "pitch_1", "disabled"]),
    "dozer_front_cab_damage_h120": ([60, 23], [0, 22], ["pitch_0", "pitch_1", "disabled"]),
    "dozer_front_tub_damage_h120": ([65, 86], [0, 81], ["pitch_0", "pitch_1", "disabled"]),
    "dozer_yoke_damage_h120": ([64, 44], [54, 30], ["bent"]),
    "prop_stop_frame": ([96, 60], [8, 59], ["raised", "pulley", "warn_0", "warn_1", "falling"]),
    "prop_stop_frame_remnant": ([80, 16], [0, 15], ["landed", "landed_hit"]),
    "prop_stop_frame_brace": ([16, 64], [0, 63], ["intact", "weakened", "snapped", "snapped_swing"]),
    "prop_stop_frame_shadow": ([80, 4], [0, 3], ["s0", "s1", "s2", "s3"]),
    "prop_stop_frame_glance": ([88, 88], [44, 44], ["glance_0", "glance_1", "glance_2", "shadow"]),
    "env_dozer_gate": ([32, 32], [0, 31], ["closed", "shake_0", "shake_1", "burst"]),
    "env_dozer_buffer": ([16, 24], [0, 23], ["idle"]),
    "env_dozer_lintel": ([32, 32], [0, 31], ["mouth", "fill"]),
    "env_dozer_haul": ([12, 12], [0, 11], ["h", "v", "idler"]),
    "dozer_rider_tumble_h120": ([64, 56], [32, 55], [f"rider_{v}_tumble_{i}" for v in "ab" for i in range(3)]),
    "dozer_engineer_slump_h120": ([64, 56], [28, 55], ["engineer_slump_0", "engineer_slump_1"]),
}


def folder(stem: str) -> Path:
    if stem.startswith("env_"):
        return ENV
    return PROPS if stem.startswith("prop_") else DOZER


def load(stem: str) -> dict:
    return pg.load_grid(folder(stem) / f"{stem}.grid.json")


def frame(stem: str, tag: str) -> list[list[str]]:
    g = load(stem)
    return pg.composite_keys(g, next(t["from"] for t in g["tags"] if t["name"] == tag))


def doz01(stem: str, i: int = 0) -> list[list[str]]:
    return pg.composite_keys(pg.load_grid(DOZER / f"{stem}.grid.json"), i)


def opaque(rows) -> set[tuple[int, int]]:
    return {(x, y) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def keys(rows) -> list[str]:
    return [c for r in rows for c in r if c != "."]


def ring(rows) -> list[str]:
    """Wheel pixels outside the rim (the spike zone)."""
    c0 = len(rows) // 2
    return [rows[y][x] for y in range(len(rows)) for x in range(len(rows))
            if rows[y][x] != "." and math.hypot(x - c0, y - c0) > ad.WHEEL_RIM + 0.5]


class Sources(unittest.TestCase):
    def test_generator_reproduces_the_committed_sources(self) -> None:
        for path, text in a2.build_all().items():
            self.assertTrue(path.is_file(), path)
            self.assertEqual(path.read_text(encoding="utf-8"), text, f"{path.name} differs from the generator")

    def test_check_mode_passes(self) -> None:
        self.assertEqual(a2.main(["--check"]), 0)

    def test_doz02_never_writes_a_doz01_path(self) -> None:
        self.assertFalse(set(a2.build_all()) & set(ad.build_all()))

    def test_every_grid_has_its_ase(self) -> None:
        for stem in EXPECTED:
            self.assertTrue((folder(stem) / f"{stem}.ase").is_file(), stem)

    def test_manifests_match_their_grids(self) -> None:
        for stem, (canvas, pivot, tags) in EXPECTED.items():
            grid = load(stem)
            m = json.loads((ROOT / f"assets/manifests/{stem}.json").read_text(encoding="utf-8"))
            self.assertEqual(grid["canvas"], canvas, stem)
            self.assertEqual(grid["pivot"], pivot, stem)
            self.assertEqual([t["name"] for t in grid["tags"]], tags, stem)
            self.assertEqual(m["frame_size"], canvas, stem)
            self.assertEqual(m["pivot"], pivot, stem)
            self.assertEqual(m["animation_tags"], tags, stem)
            self.assertEqual(m["palette_id"], "dozer-floor01-v0")
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            self.assertIn("ART-PUBLICATION-ALL-V1", m["license"])
            self.assertEqual(m["grid_source"], f"{folder(stem).relative_to(ROOT).as_posix()}/{stem}.grid.json")
            self.assertEqual(m["dozer"]["spec"], "docs/02-art/doz-02-dozer-damage-spec.md")

    def test_palette_is_the_dozer_palette(self) -> None:
        for stem in EXPECTED:
            grid = load(stem)
            for entry in grid["palette"]:
                self.assertEqual(PALETTE.get(entry["key"]), entry["rgba"], f"{stem}: {entry['key']}")

    def test_lint(self) -> None:
        for stem in EXPECTED:
            report = pg.lint_grid(load(stem))
            self.assertFalse(any(report["isolated_count_per_frame"]), stem)
            self.assertTrue(all(n <= a2.MAX_ORPHANS.get(stem, 9) for n in report["orphan_count_per_frame"]),
                            (stem, report["orphan_count_per_frame"]))

    def test_validator_configs(self) -> None:
        for stem in EXPECTED:
            cfg = json.loads((folder(stem) / f"{stem}.pixelart.json").read_text(encoding="utf-8"))
            self.assertEqual(cfg["format"], "pixelart-asset-v1")
            self.assertEqual(cfg["frame_size"], EXPECTED[stem][0])
            self.assertEqual(cfg["rules"]["max_colors"], 30 if stem == "dozer_hull_damage_h120" else 24)
            self.assertEqual(cfg["rules"]["max_orphans_per_frame"], a2.MAX_ORPHANS.get(stem, 9))
            used = {c for i in range(len(load(stem)["frames"])) for c in keys(pg.composite_keys(load(stem), i))}
            self.assertLessEqual(len(used), cfg["rules"]["max_colors"], stem)


class Pitch(unittest.TestCase):
    """Item 2: the impact pitch is a whole-pixel column translation of the level damaged frame."""

    def check_columns(self, stem: str, anchor_x: int, col0_x: int) -> None:
        g = load(stem)
        level = pg.composite_keys(g, 2)
        h = len(level)
        for fi, amp in ((0, 1.0), (1, 0.5)):
            rows = pg.composite_keys(g, fi)
            for c in range(len(level[0])):
                s = a2.pitch_dy(col0_x + c - anchor_x, amp)
                col = [level[r][c] for r in range(h)]
                moved = [rows[r - s][c] if 0 <= r - s < h else "." for r in range(h)]
                self.assertEqual(moved, col, f"{stem} frame {fi} column {c} (shift {s})")

    def test_pitch_formula_end_points(self) -> None:
        self.assertEqual(a2.pitch_dy(a2.PITCH_REAR_X, 1.0), 2)
        self.assertEqual(a2.pitch_dy(a2.PITCH_FRONT_X, 1.0), -4)
        self.assertEqual(a2.pitch_dy(a2.PITCH_FRONT_X, 0.5), -2)
        self.assertEqual(a2.pitch_dy(0, 0.0), 0)

    def test_hull_body_frames_are_column_translations(self) -> None:
        self.check_columns("dozer_hull_damage_h120", 87, 0)

    def test_front_boxes_are_column_translations(self) -> None:
        for name in ("front_cab", "front_tub"):
            anchor = ad.front_boxes()[name]["anchor"][0]
            self.check_columns(f"dozer_{name}_damage_h120", 0, anchor)

    def test_pitch_keeps_every_pixel(self) -> None:
        for stem in ("dozer_hull_damage_h120", "dozer_front_cab_damage_h120", "dozer_front_tub_damage_h120"):
            g = load(stem)
            counts = [len(opaque(pg.composite_keys(g, i))) for i in range(3)]
            self.assertEqual(len(set(counts)), 1, f"{stem}: {counts}")


class Damage(unittest.TestCase):
    """Item 3: the disabled overlays on the hull, the front, the yoke."""

    def test_hull_silhouette_below_the_stack_collar_is_unchanged(self) -> None:
        new, old = frame("dozer_hull_damage_h120", "disabled"), doz01("dozer_hull_h120")
        w, h, _ox, fy = ad.hull_canvas(1.0)
        collar_row = math.floor(fy + 1 - 129)
        for y in range(collar_row + 1, h):
            self.assertEqual([c != "." for c in new[y]], [c != "." for c in old[y]], f"row {y}")

    def test_hull_differs_where_the_damage_is(self) -> None:
        new, old = frame("dozer_hull_damage_h120", "disabled"), doz01("dozer_hull_h120")
        diff = {(x, y) for y in range(len(new)) for x in range(len(new[0])) if new[y][x] != old[y][x]}
        self.assertGreater(len(diff), 80)
        w, h, ox, fy = ad.hull_canvas(1.0)
        rs = ad.Raster(w, h, ox, fy, 1.0)
        for (x, y) in ((80, 88), (17, 68), (86, 140)):     # crack, cab dent, crooked stack
            c, r = rs.to_px(x, y)
            near = {(c + dx, r + dy) for dx in range(-3, 4) for dy in range(-3, 4)}
            self.assertTrue(near & diff, f"no damage near design ({x}, {y})")

    def test_the_firebox_is_down_to_embers(self) -> None:
        for tag in ("pitch_0", "pitch_1", "disabled"):
            self.assertFalse(set(keys(frame("dozer_hull_damage_h120", tag))) & BRIGHT_FIRE, tag)
        self.assertTrue(set(keys(doz01("dozer_hull_h120"))) & BRIGHT_FIRE, "DOZ-01 hull had live fire")

    def test_the_bent_yoke_has_no_lamp_light(self) -> None:
        rows = frame("dozer_yoke_damage_h120", "bent")
        self.assertFalse(set(keys(rows)) & FIRE)
        self.assertTrue(set(keys(doz01("dozer_yoke_h120"))) & FIRE, "DOZ-01 yoke lamp was lit")

    def test_the_bent_yoke_keeps_its_axle_cap(self) -> None:
        new, old = frame("dozer_yoke_damage_h120", "bent"), doz01("dozer_yoke_h120")
        for y in range(len(new)):
            self.assertEqual(new[y][a2.YOKE_KINK[2] + 2:], old[y][a2.YOKE_KINK[2] + 2:], f"row {y}")

    def test_the_bent_yoke_sags(self) -> None:
        new, old = opaque(frame("dozer_yoke_damage_h120", "bent")), opaque(doz01("dozer_yoke_h120"))
        col = a2.YOKE_KINK[1] - 1
        self.assertEqual(min(y for x, y in new if x == col) - min(y for x, y in old if x == col), a2.YOKE_SAG)

    def test_the_tub_lip_is_crumpled(self) -> None:
        new = frame("dozer_front_tub_damage_h120", "disabled")
        old = ad.front_boxes()["front_tub"]["rows"]
        top_new = [min((y for y in range(len(new)) if new[y][x] != "."), default=None) for x in range(len(new[0]))]
        top_old = [min((y for y in range(len(old)) if old[y][x] != "."), default=None) for x in range(len(old[0]))]
        drops = [a - b for a, b in zip(top_new, top_old) if a is not None and b is not None]
        self.assertEqual(max(drops), 3)


class JammedWheel(unittest.TestCase):
    """Item 1: the jammed wheel reads as stopped and not lethal."""

    def test_no_glint_survives(self) -> None:
        self.assertFalse(set(keys(frame("dozer_wheel_jammed_h120", "jammed"))) & {"6", "7"})

    def test_spike_ring_is_dark(self) -> None:
        jam, live = ring(frame("dozer_wheel_jammed_h120", "jammed")), ring(doz01("dozer_wheel_h120"))
        self.assertLessEqual(mean_l(jam), 0.35)
        self.assertGreaterEqual(mean_l(live) - mean_l(jam), 0.2)

    def test_spike_ring_is_no_longer_the_brightest_part(self) -> None:
        jam = mean_l(ring(frame("dozer_wheel_jammed_h120", "jammed")))
        yoke = mean_l(keys(frame("dozer_yoke_damage_h120", "bent")))
        self.assertLess(jam, yoke + 0.1, "the DOZ-01 rule (ring > every part + 0.1) must not hold any more")

    def test_the_tie_bar_is_brighter_than_the_spikes(self) -> None:
        rows = frame("dozer_wheel_jammed_h120", "jammed")
        self.assertGreater(mean_l([k for k in keys(rows) if k in "YT"]), mean_l(ring(rows)))

    def test_the_wheel_still_stands_on_the_floor(self) -> None:
        jam, live = opaque(frame("dozer_wheel_jammed_h120", "jammed")), opaque(doz01("dozer_wheel_h120"))
        self.assertEqual(max(y for _x, y in jam), max(y for _x, y in live))

    def test_three_spikes_are_broken(self) -> None:
        jam, live = frame("dozer_wheel_jammed_h120", "jammed"), doz01("dozer_wheel_h120")
        c0 = len(jam) // 2
        for axis in (90, 30, 150):
            a = math.radians(axis)
            x, y = c0 + round((ad.WHEEL_TIP - 1) * math.cos(a)), c0 - round((ad.WHEEL_TIP - 1) * math.sin(a))
            self.assertNotEqual(live[y][x], ".", axis)
            self.assertEqual(jam[y][x], ".", f"spike {axis}: tip still there")


class StopFrame(unittest.TestCase):
    """Item 4: the hanging frame, the remnant and the brace."""

    def test_the_mass_fills_its_rectangle(self) -> None:
        rows = frame("prop_stop_frame", "raised")
        px, py = a2.FRAME_PIVOT
        mass = {(x, y) for x, y in opaque(rows) if y > py - 16}
        self.assertEqual(min(x for x, _y in mass), px)
        self.assertEqual(max(x for x, _y in mass), px + a2.FRAME_W - 1)
        self.assertEqual(max(y for _x, y in mass), py)

    def test_the_sling_reaches_the_block(self) -> None:
        raised, pulley = opaque(frame("prop_stop_frame", "raised")), opaque(frame("prop_stop_frame", "pulley"))
        top_of_chain = min(y for _x, y in raised)
        bottom_of_block = max(y for _x, y in pulley if y > 0 and _x < 60)
        self.assertLessEqual(top_of_chain, bottom_of_block + 1)
        self.assertEqual(min(y for _x, y in pulley), 0, "the strap reaches the ceiling row")

    def test_the_haul_chain_leaves_the_canvas(self) -> None:
        rows = frame("prop_stop_frame", "pulley")
        self.assertTrue(any(r[-1] != "." for r in rows))

    def test_remnant(self) -> None:
        for tag in ("landed", "landed_hit"):
            px = opaque(frame("prop_stop_frame_remnant", tag))
            self.assertEqual(min(x for x, _y in px), 0, tag)
            self.assertEqual(max(x for x, _y in px), 79, tag)
            self.assertEqual(max(y for _x, y in px), 15, tag)
        landed, hit = (opaque(frame("prop_stop_frame_remnant", t)) for t in ("landed", "landed_hit"))
        self.assertGreater(len(landed), len(hit), "the miss keeps the tie bar")

    def test_brace_chain_runs_up(self) -> None:
        rows = frame("prop_stop_frame_brace", "intact")
        self.assertTrue(any(c != "." for c in rows[0]))
        self.assertTrue(any(c != "." for c in rows[-1]))


class ExecutionFrames(unittest.TestCase):
    """The execution frames of spec section 10: warn, falling, the shadow, the glance, the brace states, the room
    pieces, the tumbles and the slump."""

    def test_warn_frames_are_the_raised_key_shifted_one_pixel(self) -> None:
        raised = frame("prop_stop_frame", "raised")
        for tag, dx in (("warn_0", -1), ("warn_1", 1)):
            warn = frame("prop_stop_frame", tag)
            shifted = {(x + dx, y): c for y, r in enumerate(raised) for x, c in enumerate(r) if c != "."}
            have = {(x, y): c for y, r in enumerate(warn) for x, c in enumerate(r) if c != "."}
            self.assertTrue(all(have.get(p) == c for p, c in shifted.items()), tag)
            motes = {p: c for p, c in have.items() if p not in shifted}
            self.assertTrue(2 <= len(motes) <= 6, (tag, len(motes)))
            self.assertLessEqual(set(motes.values()), {"4", "5"}, tag)

    def test_falling_keeps_the_mass_rectangle_and_bows_the_sling(self) -> None:
        raised, fall = frame("prop_stop_frame", "raised"), frame("prop_stop_frame", "falling")
        py = a2.FRAME_PIVOT[1]
        self.assertEqual(fall[py - 15:], raised[py - 15:], "the mass rows are the raised key's")
        top = (24, 38)                                  # the sling legs' rows
        left = lambda rows: min(x for y in range(*top) for x, c in enumerate(rows[y]) if c != ".")  # noqa: E731
        right = lambda rows: max(x for y in range(*top) for x, c in enumerate(rows[y]) if c != ".")  # noqa: E731
        self.assertLessEqual(left(fall), left(raised) - 1)
        self.assertGreaterEqual(right(fall), right(raised) + 1)
        streaks = [(x, y) for y in range(len(fall)) for x in range(len(fall[0]))
                   if fall[y][x] == "3" and raised[y][x] != "3"]
        self.assertGreaterEqual(len(streaks), 12)

    def test_shadow_grows_from_40x1_to_80x4(self) -> None:
        sizes = []
        for tag in ("s0", "s1", "s2", "s3"):
            rows = frame("prop_stop_frame_shadow", tag)
            px = opaque(rows)
            self.assertEqual(set(keys(rows)), {"K"})
            sizes.append((max(x for x, _y in px) - min(x for x, _y in px) + 1,
                          max(y for _x, y in px) - min(y for _x, y in px) + 1))
        self.assertEqual(sizes[0], (40, 1))
        self.assertEqual(sizes[3], (80, 4))
        self.assertEqual([s[1] for s in sizes], [1, 2, 3, 4])
        self.assertEqual([s[0] for s in sizes], sorted(s[0] for s in sizes))

    def test_glance_never_overlaps_the_wheels_lethal_circle(self) -> None:
        axle = (124, 49)
        ax, ay = a2.GLANCE_ANCHOR_RIG
        for i, tag in enumerate(("glance_0", "glance_1", "glance_2")):
            ox, oy = a2.GLANCE_OFFSETS[i]
            for (x, y) in opaque(frame("prop_stop_frame_glance", tag)):
                rx, ry = ax + ox + x - 44, ay + oy - (y - 44)
                self.assertGreater(math.hypot(rx - axle[0], ry - axle[1]), 44, (tag, x, y))

    def test_glance_beam_turns_a_whole_beam(self) -> None:
        g0 = opaque(frame("prop_stop_frame_glance", "glance_0"))
        self.assertEqual(max(x for x, _y in g0) - min(x for x, _y in g0) + 1, 80)
        for tag, deg in (("glance_1", 15), ("glance_2", 35)):
            px = opaque(frame("prop_stop_frame_glance", tag))
            tilt = max(y for _x, y in px) - min(y for _x, y in px) + 1
            self.assertGreater(tilt, 16 + int(80 * math.sin(math.radians(deg)) * 0.8), tag)
            # the left end is the low one (it tips toward the rear)
            left = [y for x, y in px if x < 44]
            right = [y for x, y in px if x >= 44]
            self.assertGreater(sum(left) / len(left), sum(right) / len(right), tag)
        rivets = [sum(1 for k in keys(frame("prop_stop_frame_glance", t)) if k == "7") for t in
                  ("glance_0", "glance_1", "glance_2")]
        self.assertEqual(len(set(rivets)), 1, f"every rivet survives the turn: {rivets}")
        self.assertEqual(set(keys(frame("prop_stop_frame_glance", "shadow"))), {"K"})

    def test_brace_states(self) -> None:
        intact, weak, snap, swing = (frame("prop_stop_frame_brace", t)
                                     for t in ("intact", "weakened", "snapped", "snapped_swing"))
        diff = lambda a, b: sum(1 for y in range(len(a)) for x in range(len(a[0])) if a[y][x] != b[y][x])  # noqa: E731
        self.assertTrue(10 <= diff(intact, weak) <= 120, diff(intact, weak))
        # the post below the horn is the intact post but for the 3 px crack
        crack = [(x, y) for y in range(12, 22) for x in range(len(weak[0])) if weak[y][x] != intact[y][x]]
        self.assertEqual(len(crack), 3)
        self.assertTrue(all(weak[y][x] == "K" for x, y in crack))
        self.assertTrue(any(c != "." for c in snap[0]) and any(c != "." for c in snap[-1]))
        self.assertEqual(snap[:9], swing[:9], "the chain swings below the stub only")
        self.assertNotEqual(snap, swing)
        self.assertEqual(len({tuple(map(tuple, f)) for f in (intact, weak, snap, swing)}), 4)
        horn = lambda rows: sum(1 for y in range(5, 9) for c in rows[y] if c in "36")  # noqa: E731
        self.assertLess(horn(snap), horn(intact), "the horn is broken off")
        self.assertTrue(any(snap[y][x] != "." for y in range(61, 64) for x in range(12, 16)), "horn on the floor")

    def test_gate_tiles(self) -> None:
        closed, burst = frame("env_dozer_gate", "closed"), frame("env_dozer_gate", "burst")
        for tag in ("shake_0", "shake_1"):
            shake = frame("env_dozer_gate", tag)
            self.assertEqual([r[0] for r in shake], [r[0] for r in closed], "the jamb stays")
            self.assertEqual([r[-1] for r in shake], [r[-1] for r in closed], "the jamb stays")
            self.assertNotEqual(shake, closed)
        self.assertLess(len(opaque(burst)), len(opaque(closed)))
        for y in range(32):
            self.assertTrue(all(burst[y][x] == "." for x in range(12, 20)), f"row {y}: the opening is clear")
        for tag in ("closed", "shake_0", "shake_1", "burst"):
            rows = frame("env_dozer_gate", tag)
            self.assertTrue(set(keys(rows)) <= set(PALETTE), tag)

    def test_buffer_stop(self) -> None:
        rows = frame("env_dozer_buffer", "idle")
        px = opaque(rows)
        self.assertEqual(max(y for _x, y in px), 23)
        self.assertEqual(min(y for _x, y in px), 0)
        self.assertTrue({"S", "0"} <= set(keys(rows)), "the hazard band")
        self.assertTrue(set(keys(rows)) <= set(PALETTE))

    def test_lintel_tiles(self) -> None:
        mouth, fill = frame("env_dozer_lintel", "mouth"), frame("env_dozer_lintel", "fill")
        self.assertTrue(set(keys(mouth)) <= set(PALETTE) and set(keys(fill)) <= set(PALETTE))
        self.assertTrue(all(c in ".K0" for y in range(24, 32) for c in mouth[y]), "8 px of inner shadow")
        self.assertTrue(all(c != "." for y in range(0, 24) for c in mouth[y]), "stone and header are solid")

    def test_haul_tiles(self) -> None:
        h, v, idler = (frame("env_dozer_haul", t) for t in ("h", "v", "idler"))
        self.assertTrue(all(sum(h[y][x] != "." for y in (4, 5, 6)) >= 1 for x in range(8)), "an unbroken run")
        self.assertTrue(all(sum(v[y][x] != "." for x in (4, 5, 6)) >= 1 for y in range(8)), "an unbroken run")
        self.assertTrue(all(c == "." for y in range(8, 12) for c in h[y]))
        self.assertGreater(len(opaque(idler)), 80)
        chain = set(keys(h)) | set(keys(v))
        self.assertTrue(chain <= {k for row in a2.CHAIN_V for k in row if k != "."} | {"K"})
        self.assertTrue(set(keys(idler)) <= set(PALETTE))

    def test_tumbles_use_only_the_rider_palette(self) -> None:
        gob = {e["key"] for g in (GOBLIN, GOBLIN.with_name("goblin_b_h36.grid.json"))
               for e in json.loads(g.read_text(encoding="utf-8"))["palette"]}
        for stem in ("dozer_rider_tumble_h120", "dozer_engineer_slump_h120"):
            for t in load(stem)["tags"]:
                rows = frame(stem, t["name"])
                extra = set(keys(rows)) - gob - set(ad.RIDER_ENGINEER_KEYS if hasattr(ad, "RIDER_ENGINEER_KEYS") else ())
                if stem == "dozer_rider_tumble_h120":
                    self.assertFalse(extra, (t["name"], extra))
        for i in range(3):
            a, b = frame("dozer_rider_tumble_h120", f"rider_a_tumble_{i}"), frame("dozer_rider_tumble_h120",
                                                                                  f"rider_b_tumble_{i}")
            self.assertNotEqual(a, b, "rider B keeps the GOB-01 B overlay")
        down = frame("dozer_rider_tumble_h120", "rider_a_tumble_2")
        self.assertEqual(max(y for _x, y in opaque(down)), 55, "landed on the floor")

    def test_slump_shows_no_eye(self) -> None:
        eyes = {"e"}                                  # the goblin eye key (w is the teeth)
        hold = pg.composite_keys(pg.load_grid(DOZER / "dozer_riders_h36.grid.json"),
                                 next(t["from"] for t in pg.load_grid(DOZER / "dozer_riders_h36.grid.json")["tags"]
                                      if t["name"] == "engineer_hold"))
        self.assertTrue(set(keys(hold)) & eyes, "the alert driver shows an eye")
        for tag in ("engineer_slump_0", "engineer_slump_1"):
            self.assertFalse(set(keys(frame("dozer_engineer_slump_h120", tag))) & eyes, tag)
        a, b = (frame("dozer_engineer_slump_h120", t) for t in ("engineer_slump_0", "engineer_slump_1"))
        self.assertNotEqual(a, b)
        self.assertLess(abs(len(opaque(a)) - len(opaque(b))), 40, "breathing is a 1 px lift, not a new pose")


class Contract(unittest.TestCase):
    def contract(self) -> dict:
        return json.loads((DOZER / "dozer_damage_h120.json").read_text(encoding="utf-8"))

    def test_impact_schedule_covers_48_ticks(self) -> None:
        hull = self.contract()["timing"]["impact_stunned"]["hull"]
        self.assertEqual(hull[0][1], 0)
        self.assertEqual(hull[-1][2], 47)
        for (_a, _s, e), (_b, s, _e) in zip(hull, hull[1:]):
            self.assertEqual(s, e + 1)

    def test_rider_seat_offsets_follow_the_pitch(self) -> None:
        rig = {p["part"]: p["anchor"] for p in json.loads((DOZER / "dozer_rig_h120.json").read_text())["parts"]
               if isinstance(p.get("anchor"), list)}
        seats = self.contract()["pitch"]["rider_seat_dy"]
        for state, amp in a2.PITCH_AMPS.items():
            for name in ("engineer", "rider_a", "rider_b"):
                self.assertEqual(seats[state][name], a2.pitch_dy(rig[name][0], amp), (state, name))

    def test_execution_timings(self) -> None:
        c = self.contract()
        sf = c["stop_frame"]
        self.assertEqual((sf["warning"]["ticks"], sf["warning"]["alternate_ticks"]), (30, 3))
        self.assertEqual(sum(sf["warning"]["shadow"]["ticks_each"]), 30)
        self.assertEqual(sf["warning"]["shadow"]["size_px"], [[40, 1], [53, 2], [67, 3], [80, 4]])
        self.assertEqual(sf["falling"]["ticks"], 8)
        self.assertEqual([f["ticks"] for f in sf["glance"]["frames"]], [4, 4, 6])
        self.assertEqual([f["offset"] for f in sf["glance"]["frames"]], [[0, 0], [-6, 0], [-16, -8]])
        self.assertEqual(sf["brace"]["swing"], {"ticks_each": 8, "for_ticks": 48, "then": "snapped (still)"})
        tumble = c["crew"]["tumble"]
        self.assertEqual(tumble["start_tick"], {"rider_a": 12, "rider_b": 16})
        self.assertEqual((tumble["airborne_ticks"], tumble["arc_up_px"]), (24, 12))
        self.assertEqual(tumble["design_forward_px"], {"rider_a": 28, "rider_b": 36})
        self.assertEqual(c["crew"]["slump"]["ticks_each"], 30)
        self.assertFalse(c["room_pieces"]["gate"]["solid"] or c["room_pieces"]["buffer"]["solid"])

    def test_front_box_anchors_are_the_doz01_anchors(self) -> None:
        rig = {p["part"]: p for p in json.loads((DOZER / "dozer_rig_h120.json").read_text())["parts"]}
        rep = self.contract()["replaces"]
        for part in ("hull_front_cab", "hull_front_tub"):
            self.assertEqual(rep[part]["anchor"], rig[part]["anchor"], part)


if __name__ == "__main__":
    unittest.main()
