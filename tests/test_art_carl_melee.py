"""Grid, manifest and export checks for the Carl 52 px unarmed melee candidate (ANIM-CARL-02).

The check_* functions take plain dicts and return a list of error strings (empty = pass)
so the negative tests can run them against mutated in-memory copies.
"""
from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "tests"))

import make_carl_melee_previews as previews  # noqa: E402
import pixel_grid_to_ase as pga  # noqa: E402
from test_art_carl_locomotion import (  # noqa: E402
    HEAD_SEARCH, HEAD_TOLERANCE, PLAYBACK, bottom_row, cel_pixels, clear_layer, normalise, put_pixel,
    shift_frame, translate)

MANIFEST = ROOT / "assets/manifests/carl_book1_h52_melee.json"
LOCO_MANIFEST = ROOT / "assets/manifests/carl_book1_h52_locomotion.json"
GRID = ROOT / "assets/source/characters/carl/carl_h52_melee.grid.json"
LOCO_GRID = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
KEYS = ROOT / "tests/fixtures/carl_melee_phase1_keys.json"
EVIDENCE = ROOT / "docs/evidence/anim-carl-02/measurements.json"
ATLAS_JSON = ROOT / "generated/art/carl_book1_h52_melee.json"
ATLAS_PNG = ROOT / "generated/art/carl_book1_h52_melee.png"

TAG_COUNTS = {"punch_a": 4, "punch_b": 4, "kick": 7, "hurt": 3, "knockback": 1, "knockdown": 2, "getup": 3}
FRAME_IDS = ["a0", "a1", "a2", "a3", "b0", "b1", "b2", "b3", "k0", "k1", "k2", "k3", "k4", "k5", "k6",
             "h0", "h1", "h2", "kb", "d0", "d1", "g0", "g1", "g2"]
KEY_IDS = {"a1": "punch_a_contact", "k1": "kick_windup", "k3": "kick_contact", "h0": "hurt_impact"}
# tick contract of the spec (section 4): (total ticks, active ticks, hit tick)
MOVES = {"punch_a": (14, [3, 4], 3), "punch_b": (22, [5, 7], 5), "kick": (36, [13, 16], 13),
         "hurt": (18, None, None), "knockdown": (36, None, None), "getup": (22, None, None)}
KB_HOLD_MS = 100
HEAD_SEARCH_Y = 20  # the sitting frames sit 16 rows lower than the idle
MAX_ORPHANS = 5  # per frame; master jacket/neck pixels next to an offset head are the documented ones
BOX_FRAMES = {"punch_a_contact": "a1", "punch_b_contact": "b1", "kick_contact": "k3"}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def frame_tags(grid: dict) -> dict[str, range]:
    return {t["name"]: range(t["from"], t["to"] + 1) for t in grid["tags"]}


def tag_of(grid: dict, index: int) -> str:
    return next(name for name, r in frame_tags(grid).items() if index in r)


def ticks_ms(ticks: list[int] | None) -> int:
    return KB_HOLD_MS if ticks is None else round((ticks[1] - ticks[0] + 1) * 1000 / 60)


# ------------------------------------------------------------------ checks

def check_manifest(m: dict, grid: dict) -> list[str]:
    errs = []
    mel = m.get("melee", {})
    if m.get("id") != "carl_book1_h52_melee_candidate_v1":
        errs.append(f"manifest id {m.get('id')!r}")
    if m.get("frame_size") != [48, 56]:
        errs.append(f"frame_size {m.get('frame_size')}")
    if m.get("pivot") != [25, 55]:
        errs.append(f"pivot {m.get('pivot')}")
    names = [t["name"] for t in grid["tags"]]
    if m.get("animation_tags") != names:
        errs.append(f"animation_tags {m.get('animation_tags')} != grid tags {names}")
    grounded, airborne = set(mel.get("grounded_tags", [])), set(mel.get("airborne_tags", []))
    if grounded & airborne:
        errs.append(f"grounded/airborne overlap: {sorted(grounded & airborne)}")
    if grounded | airborne != set(names):
        errs.append("grounded_tags + airborne_tags must cover animation_tags exactly")
    if airborne != {"knockback"}:
        errs.append(f"airborne_tags {sorted(airborne)} != ['knockback']")
    for name in names:
        if mel.get("playback", {}).get(name) not in PLAYBACK:
            errs.append(f"playback[{name}] = {mel.get('playback', {}).get(name)!r} not in {sorted(PLAYBACK)}")
    if mel.get("tick_hz") != 60:
        errs.append(f"tick_hz {mel.get('tick_hz')}")
    index = {fid: i for i, fid in enumerate(FRAME_IDS)}
    if mel.get("keys") != {name: index[fid] for fid, name in KEY_IDS.items()}:
        errs.append(f"keys {mel.get('keys')} do not point at a1/k1/k3/h0")
    if "phase1_keys" in mel:
        errs.append("phase1_keys must be renamed to keys")
    if m.get("review_status") != "prototype_fixture_unapproved":
        errs.append(f"review_status {m.get('review_status')!r}")
    return errs


def check_header(grid: dict, m: dict, master: dict) -> list[str]:
    errs = []
    if grid["canvas"] != m["frame_size"]:
        errs.append(f"canvas {grid['canvas']} != manifest frame_size {m['frame_size']}")
    if grid["pivot"] != m["pivot"]:
        errs.append(f"pivot {grid['pivot']} != manifest pivot {m['pivot']}")
    if grid["palette"] != master["palette"]:
        errs.append("palette differs from the master palette")
    if grid["layers"] != master["layers"]:
        errs.append("layers differ from the master layers")
    return errs


def check_tags(grid: dict) -> list[str]:
    errs, expected_next = [], 0
    if len(grid["frames"]) != 24:
        errs.append(f"{len(grid['frames'])} frames, expected 24")
    for t in grid["tags"]:
        if t["from"] != expected_next:
            errs.append(f"tag {t['name']} starts at {t['from']}, expected {expected_next}")
        expected_next = t["to"] + 1
        n = t["to"] - t["from"] + 1
        if n != TAG_COUNTS.get(t["name"]):
            errs.append(f"tag {t['name']} has {n} frames, expected {TAG_COUNTS.get(t['name'])}")
    if expected_next != len(grid["frames"]):
        errs.append(f"tags cover {expected_next} frames but grid has {len(grid['frames'])}")
    if [t["name"] for t in grid["tags"]] != list(TAG_COUNTS):
        errs.append(f"tag order {[t['name'] for t in grid['tags']]}")
    return errs


def check_durations(grid: dict, m: dict) -> list[str]:
    """Grid durations = the tick table at 60 Hz; the tick ranges are contiguous and total correctly."""
    errs, by_id = [], {}
    for move, data in m["melee"]["moves"].items():
        want_total, want_active, want_hit = MOVES[move]
        if data["total_ticks"] != want_total:
            errs.append(f"{move}: total_ticks {data['total_ticks']} != {want_total}")
        if data["active_ticks"] != want_active or data["hit_tick"] != want_hit:
            errs.append(f"{move}: active/hit {data['active_ticks']}/{data['hit_tick']} != {want_active}/{want_hit}")
        nxt = 0
        for fr in data["frames"]:
            by_id[fr["name"].split()[0]] = fr["ticks"]
            if fr["ticks"] is None:
                continue
            if fr["ticks"][0] != nxt:
                errs.append(f"{move} {fr['name']}: starts at tick {fr['ticks'][0]}, expected {nxt}")
            nxt = fr["ticks"][1] + 1
        if nxt != want_total:
            errs.append(f"{move}: tick ranges end at {nxt}, expected {want_total}")
    if list(by_id) != FRAME_IDS:
        errs.append(f"manifest frame order {list(by_id)}")
        return errs
    for i, fid in enumerate(FRAME_IDS):
        got = grid["frames"][i]["duration_ms"]
        if got != ticks_ms(by_id[fid]):
            errs.append(f"frame {i} ({fid}): duration {got} ms != {ticks_ms(by_id[fid])} ms from ticks {by_id[fid]}")
    return errs


def check_keys(grid: dict, keys: dict, m: dict) -> list[str]:
    errs = []
    for fid, name in KEY_IDS.items():
        i = m["melee"]["keys"][name]
        if grid["frames"][i]["cels"] != keys["keys"][fid]["cels"]:
            errs.append(f"key {fid} (frame {i}) differs from the frozen Opus key")
    return errs


def check_contact(grid: dict, m: dict) -> list[str]:
    errs, w = [], grid["canvas"][0]
    grounded = set(m["melee"]["grounded_tags"])
    ground_row = m["pivot"][1]
    for i in range(len(grid["frames"])):
        flat = pga.composite_keys(grid, i)
        bottom, tag = bottom_row(grid, i), tag_of(grid, i)
        if bottom is None:
            errs.append(f"frame {i} ({tag}): empty")
            continue
        if tag in grounded and bottom != ground_row:
            errs.append(f"frame {i} ({tag}): grounded but lowest opaque row is {bottom}, expected {ground_row}")
        if tag not in grounded and bottom > ground_row - 2:
            errs.append(f"frame {i} ({tag}): airborne but lowest opaque row is {bottom} (> {ground_row - 2})")
        if any(row[0] != "." or row[w - 1] != "." for row in flat) or any(c != "." for c in flat[0]):
            errs.append(f"frame {i} ({tag}): opaque pixel in column 0, column {w - 1} or row 0 (clipping)")
    return errs


def check_lint(grid: dict) -> list[str]:
    report = pga.lint_grid(grid)
    errs = pga.lint_errors(report)
    for i, n in enumerate(report["orphan_count_per_frame"]):
        if n > MAX_ORPHANS:
            errs.append(f"frame {i}: {n} orphan pixels (max {MAX_ORPHANS})")
    return errs


def check_palette(grid: dict, master: dict) -> list[str]:
    declared = {e["key"] for e in master["palette"]}
    errs = []
    for i, f in enumerate(grid["frames"]):
        for layer, rows in f["cels"].items():
            if layer not in grid["layers"]:
                errs.append(f"frame {i}: unknown layer {layer}")
            used = {c for row in rows for c in row} - {pga.TRANSPARENT}
            if used - declared:
                errs.append(f"frame {i} {layer}: keys outside the palette {sorted(used - declared)}")
            if len(rows) != grid["canvas"][1] or any(len(r) != grid["canvas"][0] for r in rows):
                errs.append(f"frame {i} {layer}: cel is not {grid['canvas']}")
    if len(declared) > 64:
        errs.append("palette exceeds 64 colours")
    return errs


def check_identity(grid: dict, loco: dict) -> list[str]:
    """Hair, head, torso, jacket and pin: the approved idle's pixels, translated only (head edits <= 8 px)."""
    errs = []
    idle = range(4)
    hairs = {normalise(cel_pixels(loco, i, "hair")) for i in idle}
    heads = [cel_pixels(loco, i, "head") for i in idle]
    locked = {layer: {normalise(cel_pixels(loco, i, layer)) for i in idle} for layer in ("torso", "jacket", "pin")}
    for i in range(len(grid["frames"])):
        tag = tag_of(grid, i)
        if normalise(cel_pixels(grid, i, "hair")) not in hairs:
            errs.append(f"frame {i} ({tag}): hair is not a pure translation of a master idle hair")
        head = cel_pixels(grid, i, "head")
        best = min((len(head ^ translate(h, dx, dy)) for h in heads
                    for dx in range(-HEAD_SEARCH, HEAD_SEARCH + 1) for dy in range(-HEAD_SEARCH, HEAD_SEARCH_Y + 1)),
                   default=len(head))
        if best > HEAD_TOLERANCE:
            errs.append(f"frame {i} ({tag}): head differs from the closest master head by {best} px "
                        f"(max {HEAD_TOLERANCE})")
        for layer, allowed in locked.items():
            pix = cel_pixels(grid, i, layer)
            # the pin may be hidden behind an arm: a subset of the master pin is fine, a changed pixel is not
            if layer == "pin":
                if pix and not any(normalise(pix) <= a for a in allowed):
                    errs.append(f"frame {i} ({tag}): pin is not the master pin")
            elif normalise(pix) not in allowed:
                errs.append(f"frame {i} ({tag}): {layer} is not a pure translation of the master {layer}")
    return errs


def reach(grid: dict, index: int, pivot: list[int]) -> dict:
    """Pivot-relative extents (x forward, y up) of the opaque pixels of one frame."""
    flat = pga.composite_keys(grid, index)
    pts = [(x - pivot[0], pivot[1] - y) for y, row in enumerate(flat) for x, c in enumerate(row) if c != "."]
    return {"max_x": max(p[0] for p in pts), "pts": pts}


def check_reach(grid: dict, m: dict, evidence: dict) -> list[str]:
    """The manifest reach boxes match the art and the committed measurements.json."""
    errs, pivot = [], m["pivot"]
    for name, fid in BOX_FRAMES.items():
        i = FRAME_IDS.index(fid)
        info = reach(grid, i, pivot)
        for part, (x0, y0, x1, y1) in m["melee"]["hit_boxes_px"][name].items():
            if part in ("fist", "sole"):
                layers = ("far_arm", "near_arm") if part == "fist" else ("shoes",)
                inside = [(x - pivot[0], pivot[1] - y) for layer in layers
                          for (x, y, _c) in cel_pixels(grid, i, layer) if x - pivot[0] >= x0]
                if not inside:
                    errs.append(f"{name}.{part}: no pixels at x >= {x0}")
                bad = [p for p in inside if not (x0 <= p[0] <= x1 and y0 <= p[1] <= y1)]
                if bad:
                    errs.append(f"{name}.{part}: pixels outside the box {[x0, y0, x1, y1]}: {bad[:3]}")
                if info["max_x"] != x1:
                    errs.append(f"{name}.{part}: art reaches +{info['max_x']}, manifest says +{x1}")
                if min(p[0] for p in inside) != x0:
                    errs.append(f"{name}.{part}: reach starts at +{min(p[0] for p in inside)}, manifest says +{x0}")
            else:  # whole kicking leg: leg-layer pixels above the hem
                pts = [(x - pivot[0], pivot[1] - y) for (x, y, _c) in cel_pixels(grid, i, "legs")
                       if y <= 42 and x - pivot[0] >= x0 - 1]
                if min(p[0] for p in pts) != x0:
                    errs.append(f"{name}.{part}: leg starts at +{min(p[0] for p in pts)}, manifest says +{x0}")
                if not all(y0 <= p[1] <= y1 for p in pts):
                    errs.append(f"{name}.{part}: leg pixels outside y {y0}..{y1}")
        frame = evidence["frames"][i]
        got = frame["bbox"][2] - pivot[0]
        want = max(b[2] for b in m["melee"]["hit_boxes_px"][name].values())
        if got != want:
            errs.append(f"{name}: evidence measurements.json bbox reaches +{got}, manifest +{want}")
    return errs


# ------------------------------------------------------------------ tests

class CarlMeleeGridTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load_json(MANIFEST)
        cls.grid = pga.load_grid(GRID)
        cls.loco = pga.load_grid(LOCO_GRID)
        cls.keys = load_json(KEYS)
        cls.evidence = load_json(EVIDENCE)

    def mutated(self) -> dict:
        return copy.deepcopy(self.grid)

    def assertClean(self, errors: list[str]) -> None:
        self.assertEqual(errors, [], "\n".join(errors))

    def assertRejects(self, errors: list[str], needle: str = "") -> None:
        self.assertTrue(errors, "mutated data was accepted")
        if needle:
            self.assertTrue(any(needle in e for e in errors), errors)

    # -- real data
    def test_manifest(self) -> None:
        self.assertClean(check_manifest(self.manifest, self.grid))

    def test_header_matches_manifest_and_locomotion(self) -> None:
        self.assertClean(check_header(self.grid, self.manifest, self.loco))

    def test_frame_count_and_tags(self) -> None:
        self.assertClean(check_tags(self.grid))

    def test_durations_equal_the_tick_table(self) -> None:
        self.assertClean(check_durations(self.grid, self.manifest))

    def test_opus_keys_are_pixel_identical(self) -> None:
        self.assertClean(check_keys(self.grid, self.keys, self.manifest))

    def test_pivot_baseline_and_no_clipping(self) -> None:
        self.assertClean(check_contact(self.grid, self.manifest))

    def test_lint(self) -> None:
        self.assertClean(check_lint(self.grid))

    def test_palette_is_the_master_subset(self) -> None:
        self.assertClean(check_palette(self.grid, self.loco))

    def test_head_jacket_hair_identity(self) -> None:
        self.assertClean(check_identity(self.grid, self.loco))

    def test_reach_matches_manifest_and_evidence(self) -> None:
        self.assertClean(check_reach(self.grid, self.manifest, self.evidence))

    def test_grid_equals_authoring_script_output(self) -> None:
        import author_carl_melee as author
        self.assertEqual(GRID.read_text(encoding="utf-8"), author.dump(author.build()))

    def test_every_grounded_move_ends_on_the_idle_footprint(self) -> None:
        """Hand-over to idle: the last frame of each returning move has both Crocs on row 55."""
        for fid in ("a3", "b3", "k6", "h2", "g2"):
            i = FRAME_IDS.index(fid)
            self.assertEqual(bottom_row(self.grid, i), 55, fid)

    # -- negative tests
    def test_rejects_changed_key(self) -> None:
        g = self.mutated()
        put_pixel(g, 1, "far_arm", 44, 21, "K")
        self.assertRejects(check_keys(g, self.keys, self.manifest), "key a1")

    def test_rejects_wrong_duration(self) -> None:
        g = self.mutated()
        g["frames"][11]["duration_ms"] = 117
        self.assertRejects(check_durations(g, self.manifest), "frame 11")

    def test_rejects_bad_tick_table(self) -> None:
        m = copy.deepcopy(self.manifest)
        m["melee"]["moves"]["kick"]["frames"][3]["ticks"] = [14, 18]
        self.assertRejects(check_durations(self.grid, m), "kick")

    def test_rejects_grounded_frame_lifted_one_row(self) -> None:
        g = self.mutated()
        shift_frame(g, 3, -1)
        self.assertRejects(check_contact(g, self.manifest), "frame 3")

    def test_rejects_airborne_frame_on_ground(self) -> None:
        g = self.mutated()
        shift_frame(g, 18, 55 - bottom_row(g, 18))
        self.assertRejects(check_contact(g, self.manifest), "frame 18")

    def test_rejects_clipping(self) -> None:
        g = self.mutated()
        put_pixel(g, 6, "torso", 47, 30, "K")
        self.assertRejects(check_contact(g, self.manifest), "clipping")

    def test_rejects_palette_and_layer_breakage(self) -> None:
        g = self.mutated()
        g["palette"][0]["rgba"] = "#010203ff"
        g["layers"] = g["layers"][::-1]
        errs = check_header(g, self.manifest, self.loco)
        self.assertRejects(errs, "palette")
        self.assertTrue(any("layers" in e for e in errs))
        g2 = self.mutated()
        put_pixel(g2, 2, "legs", 20, 50, "Z")
        self.assertRejects(check_palette(g2, self.loco), "outside the palette")

    def test_rejects_bad_manifest(self) -> None:
        m = copy.deepcopy(self.manifest)
        m["pivot"] = [24, 55]
        m["melee"]["playback"]["kick"] = "bounce"
        m["melee"]["airborne_tags"].append("kick")
        m["melee"]["phase1_keys"] = m["melee"]["keys"]
        errs = check_manifest(m, self.grid)
        self.assertRejects(errs, "pivot")
        for needle in ("playback[kick]", "overlap", "phase1_keys"):
            self.assertTrue(any(needle in e for e in errs), (needle, errs))

    def test_rejects_broken_tags(self) -> None:
        g = self.mutated()
        g["tags"][2]["from"] += 1
        self.assertRejects(check_tags(g), "tag kick")
        g = self.mutated()
        g["frames"].pop()
        self.assertRejects(check_tags(g), "23 frames")

    def test_rejects_isolated_pixel(self) -> None:
        g = self.mutated()
        put_pixel(g, 5, "torso", 3, 3, g["palette"][0]["key"])
        self.assertRejects(check_lint(g))

    def test_rejects_hair_and_head_and_jacket_changes(self) -> None:
        g = self.mutated()
        clear_layer(g, 4, "hair", 1)
        clear_layer(g, 5, "head", HEAD_TOLERANCE + 4)
        clear_layer(g, 6, "jacket", 1)
        errs = check_identity(g, self.loco)
        self.assertRejects(errs, "frame 4")
        self.assertTrue(any("frame 5" in e and "head" in e for e in errs))
        self.assertTrue(any("frame 6" in e and "jacket" in e for e in errs))

    def test_rejects_reach_mismatch(self) -> None:
        g = self.mutated()
        put_pixel(g, 11, "shoes", 47, 31, "Q")
        self.assertRejects(check_reach(g, self.manifest, self.evidence))
        m = copy.deepcopy(self.manifest)
        m["melee"]["hit_boxes_px"]["punch_a_contact"]["fist"][2] = 22
        self.assertRejects(check_reach(self.grid, m, self.evidence), "punch_a_contact")


class CarlMeleeTickTests(unittest.TestCase):
    def test_tick_durations_do_not_drift(self) -> None:
        for ticks in ([3, 4, 4, 3], [4, 7, 2, 6, 5, 7, 5], [1] * 36):
            ms = previews.tick_durations(ticks)
            self.assertEqual(sum(ms), round(sum(ticks) * 1000 / 60 / 10) * 10)
            for d in ms:
                self.assertEqual(d % 10, 0)
            running = 0
            for i, d in enumerate(ms):
                running += d
                self.assertLessEqual(abs(running - sum(ticks[:i + 1]) * 1000 / 60), 5.01)

    def test_gif_string_lengths_match_the_spec_cadence(self) -> None:
        table = previews.frame_table(load_json(MANIFEST))
        seqs = previews.gif_sequences(table)
        total = lambda name: sum(n for _, n in seqs[name])  # noqa: E731
        self.assertEqual(total("punch_a"), 14)
        self.assertEqual(total("punch_b"), 22)
        self.assertEqual(total("jab_haymaker"), 7 + 22)       # hits on t3 and t7 + 5 = t12 (29 ticks)
        self.assertEqual(total("jab_kick"), 7 + 36)
        self.assertEqual(total("kick"), 36)
        self.assertEqual(total("hurt"), 18)
        self.assertEqual(total("knockdown_getup"), previews.KB_PREVIEW_TICKS + 36 + 22)


# ------------------------------------------------------------------ export level

@unittest.skipUnless(ATLAS_JSON.is_file() and ATLAS_PNG.is_file(),
                     "run scripts/export_assets.sh to generate the melee atlas")
class CarlMeleeExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.grid = pga.load_grid(GRID)
        cls.meta = load_json(ATLAS_JSON)

    def test_atlas_rectangles_are_unpadded_row_major(self) -> None:
        frames = self.meta["frames"]
        self.assertEqual(len(frames), 24)
        for i, f in enumerate(frames):
            r = f["frame"]
            self.assertEqual((r["x"], r["y"], r["w"], r["h"]), (i * 48, 0, 48, 56), f"frame {i}")
        with Image.open(ATLAS_PNG) as atlas:
            self.assertEqual(atlas.size, (24 * 48, 56))

    def test_tags_and_durations_match_grid(self) -> None:
        tags = [(t["name"], t["from"], t["to"]) for t in self.meta["meta"]["frameTags"]]
        self.assertEqual(tags, [(t["name"], t["from"], t["to"]) for t in self.grid["tags"]])
        self.assertEqual([f["duration"] for f in self.meta["frames"]],
                         [f["duration_ms"] for f in self.grid["frames"]])

    def test_preview_script_smoke_and_gif_timing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertEqual(previews.main(["--no-floor", "--out", str(out)]), 0)
            names = ["contact_sheet_x1.png", "contact_sheet_x6.png", "timing_diagram.png", "punch_a_x4.gif",
                     "punch_b_x4.gif", "jab_haymaker_x4.gif", "jab_kick_x4.gif", "kick_x4.gif", "hurt_x4.gif",
                     "knockdown_getup_x4.gif", "measurements.json"]
            for name in names:
                self.assertGreater((out / name).stat().st_size, 0, name)
            self.assertEqual(len(list(out.glob("floor01_*"))), 0)
            manifest = load_json(MANIFEST)
            table = previews.frame_table(manifest)
            with Image.open(ATLAS_PNG) as atlas:
                atlas = atlas.convert("RGBA")
                raw = [atlas.crop((i * 48, 0, i * 48 + 48, 56)).tobytes() for i in range(24)]
            for name, seq in previews.gif_sequences(table).items():
                ticks = [n for _, n in seq] + [previews.IDLE_TAIL_TICKS]
                durations = previews.tick_durations(ticks)
                with Image.open(out / f"{name}_x4.gif") as gif:
                    actual = []
                    for n in range(gif.n_frames):
                        gif.seek(n)
                        actual.append(gif.info["duration"])
                self.assertEqual(sum(actual), sum(durations), name)
                self.assertLessEqual(len(actual), len(durations), name)
                self.assertEqual(actual[-1], durations[-1], name)
                # the first frame (and thus the start of the move) keeps its tick-table duration
                if len(seq) > 1 and raw[seq[0][0]] != raw[seq[1][0]]:
                    self.assertEqual(actual[0], durations[0], name)
            report = load_json(out / "measurements.json")
            self.assertEqual(len(report["frames"]), 24)
            self.assertEqual(set(report["sha256"]), set(names) - {"measurements.json"})


if __name__ == "__main__":
    unittest.main()
