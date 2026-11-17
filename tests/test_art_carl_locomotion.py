"""Grid, manifest and export checks for the Carl 52 px locomotion candidate (ANIM-CARL-01).

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

import make_carl_locomotion_previews as previews  # noqa: E402
import pixel_grid_to_ase as pga  # noqa: E402

MANIFEST = ROOT / "assets/manifests/carl_book1_h52_locomotion.json"
MASTER_MANIFEST = ROOT / "assets/manifests/carl_book1_h52.json"
GRID = ROOT / "assets/source/characters/carl/carl_h52_locomotion.grid.json"
MASTER_GRID = ROOT / "assets/source/characters/carl/carl_h52.grid.json"
ATLAS_JSON = ROOT / "generated/art/carl_book1_h52_locomotion.json"
ATLAS_PNG = ROOT / "generated/art/carl_book1_h52_locomotion.png"

TAG_COUNTS = {"idle": 4, "run": 6, "jump_start": 2, "jump_rise": 1, "jump_fall": 1, "land": 2}
HEAD_TOLERANCE = 8  # px: allowed head expression edits vs the translated master head
HEAD_SEARCH = 12  # px: translation search radius for the head comparison
PLAYBACK = {"loop", "once", "hold"}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def frame_tags(grid: dict) -> dict[str, range]:
    return {t["name"]: range(t["from"], t["to"] + 1) for t in grid["tags"]}


def tag_of(grid: dict, index: int) -> str:
    return next(name for name, r in frame_tags(grid).items() if index in r)


def cel_pixels(grid: dict, index: int, layer: str) -> set[tuple[int, int, str]]:
    rows = grid["frames"][index]["cels"].get(layer, [])
    return {(x, y, ch) for y, row in enumerate(rows) for x, ch in enumerate(row) if ch != pga.TRANSPARENT}


def translate(pixels, dx: int, dy: int) -> set:
    return {(x + dx, y + dy, ch) for x, y, ch in pixels}


def normalise(pixels) -> frozenset:
    if not pixels:
        return frozenset()
    x0, y0 = min(p[0] for p in pixels), min(p[1] for p in pixels)
    return frozenset(translate(pixels, -x0, -y0))


def bottom_row(grid: dict, index: int) -> int | None:
    flat = pga.composite_keys(grid, index)
    rows = [y for y, row in enumerate(flat) if any(c != pga.TRANSPARENT for c in row)]
    return rows[-1] if rows else None


# ------------------------------------------------------------------ checks

def check_manifest(m: dict, master_m: dict, grid: dict) -> list[str]:
    errs = []
    loco = m.get("locomotion", {})
    pad = loco.get("padding_from_master", [0, 0])
    if m.get("id") != "carl_book1_h52_locomotion_candidate_v1":
        errs.append(f"manifest id {m.get('id')!r}")
    if m.get("frame_size") != [48, 56]:
        errs.append(f"frame_size {m.get('frame_size')}")
    if m.get("pivot") != [25, 55]:
        errs.append(f"pivot {m.get('pivot')}")
    if m.get("pivot") != [master_m["pivot"][0] + pad[0], master_m["pivot"][1]]:
        errs.append("pivot != master pivot + left padding")
    if m.get("frame_size", [0, 0])[1] != master_m["frame_size"][1]:
        errs.append("frame height != master frame height")
    names = [t["name"] for t in grid["tags"]]
    if m.get("animation_tags") != names:
        errs.append(f"animation_tags {m.get('animation_tags')} != grid tags {names}")
    grounded, airborne = set(loco.get("grounded_tags", [])), set(loco.get("airborne_tags", []))
    if grounded & airborne:
        errs.append(f"grounded/airborne overlap: {sorted(grounded & airborne)}")
    if grounded | airborne != set(names):
        errs.append("grounded_tags + airborne_tags must cover animation_tags exactly")
    playback = loco.get("playback", {})
    for name in names:
        if playback.get(name) not in PLAYBACK:
            errs.append(f"playback[{name}] = {playback.get(name)!r} not in {sorted(PLAYBACK)}")
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
    for t in grid["tags"]:
        if t["from"] != expected_next:
            errs.append(f"tag {t['name']} starts at {t['from']}, expected {expected_next}")
        expected_next = t["to"] + 1
        n = t["to"] - t["from"] + 1
        if n != TAG_COUNTS.get(t["name"]):
            errs.append(f"tag {t['name']} has {n} frames, expected {TAG_COUNTS.get(t['name'])}")
    if expected_next != len(grid["frames"]):
        errs.append(f"tags cover {expected_next} frames but grid has {len(grid['frames'])}")
    for i, f in enumerate(grid["frames"]):
        if not 40 <= f["duration_ms"] <= 1000:
            errs.append(f"frame {i}: duration {f['duration_ms']} ms outside 40..1000")
    return errs


def check_idle_preservation(grid: dict, master: dict, pad: int = 6) -> list[str]:
    errs = []
    for i in range(len(master["frames"])):
        cand, base = grid["frames"][i], master["frames"][i]
        if cand["duration_ms"] != base["duration_ms"]:
            errs.append(f"idle frame {i}: duration {cand['duration_ms']} != master {base['duration_ms']}")
        if set(cand["cels"]) != set(base["cels"]):
            errs.append(f"idle frame {i}: cel layers {sorted(cand['cels'])} != master {sorted(base['cels'])}")
        for layer, rows in base["cels"].items():
            got = cand["cels"].get(layer, [])
            for y, row in enumerate(rows):
                crow = got[y] if y < len(got) else ""
                if crow[:pad].strip(".") or crow[len(crow) - pad:].strip("."):
                    errs.append(f"idle frame {i} {layer} row {y}: padding columns not empty")
                if crow[pad:len(crow) - pad] != row:
                    errs.append(f"idle frame {i} {layer} row {y}: differs from master")
    return errs


def check_contact(grid: dict, m: dict) -> list[str]:
    errs, w = [], grid["canvas"][0]
    grounded = set(m["locomotion"]["grounded_tags"])
    ground_row = m["pivot"][1]
    for i in range(len(grid["frames"])):
        flat = pga.composite_keys(grid, i)
        bottom, tag = bottom_row(grid, i), tag_of(grid, i)
        if bottom is None:
            errs.append(f"frame {i} ({tag}): empty")
            continue
        if tag in grounded and bottom != ground_row:
            errs.append(f"frame {i} ({tag}): grounded but lowest opaque row is {bottom}, expected {ground_row}")
        if tag not in grounded and bottom >= ground_row:
            errs.append(f"frame {i} ({tag}): airborne but lowest opaque row is {bottom} (>= {ground_row})")
        if any(row[0] != "." or row[w - 1] != "." for row in flat) or any(c != "." for c in flat[0]):
            errs.append(f"frame {i} ({tag}): opaque pixel in column 0, column {w - 1} or row 0 (clipping)")
    return errs


def check_lint(grid: dict) -> list[str]:
    return pga.lint_errors(pga.lint_grid(grid))


def check_identity(grid: dict, master: dict) -> list[str]:
    errs = []
    hairs = {normalise(cel_pixels(master, i, "hair")) for i in range(len(master["frames"]))}
    heads = [cel_pixels(master, i, "head") for i in range(len(master["frames"]))]
    for i in range(len(grid["frames"])):
        if normalise(cel_pixels(grid, i, "hair")) not in hairs:
            errs.append(f"frame {i} ({tag_of(grid, i)}): hair is not a pure translation of a master idle hair")
        head = cel_pixels(grid, i, "head")
        best = min(
            (len(head ^ translate(h, dx, dy))
             for h in heads
             for dx in range(-HEAD_SEARCH, HEAD_SEARCH + 1)
             for dy in range(-HEAD_SEARCH, HEAD_SEARCH + 1)),
            default=len(head))
        if best > HEAD_TOLERANCE:
            errs.append(f"frame {i} ({tag_of(grid, i)}): head differs from the closest master head by {best} px "
                        f"(max {HEAD_TOLERANCE})")
    return errs


# ------------------------------------------------------------------ mutation helpers

def shift_frame(grid: dict, index: int, dy: int) -> None:
    """Move every cel of one frame by dy rows (positive = down), padding with empty rows."""
    w = grid["canvas"][0]
    for layer, rows in grid["frames"][index]["cels"].items():
        blank = "." * w
        rows = rows[:]
        rows = ([blank] * dy + rows[:len(rows) - dy]) if dy > 0 else (rows[-dy:] + [blank] * -dy)
        grid["frames"][index]["cels"][layer] = rows


def put_pixel(grid: dict, index: int, layer: str, x: int, y: int, ch: str) -> None:
    rows = grid["frames"][index]["cels"].setdefault(layer, ["." * grid["canvas"][0]] * grid["canvas"][1])
    rows[y] = rows[y][:x] + ch + rows[y][x + 1:]


def clear_layer(grid: dict, index: int, layer: str, count: int | None = None) -> None:
    removed = 0
    rows = grid["frames"][index]["cels"][layer]
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != "." and (count is None or removed < count):
                put_pixel(grid, index, layer, x, y, ".")
                removed += 1


class CarlLocomotionGridTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load_json(MANIFEST)
        cls.master_manifest = load_json(MASTER_MANIFEST)
        cls.grid = pga.load_grid(GRID)
        cls.master = pga.load_grid(MASTER_GRID)

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
        self.assertClean(check_manifest(self.manifest, self.master_manifest, self.grid))

    def test_header_matches_manifest_and_master(self) -> None:
        self.assertClean(check_header(self.grid, self.manifest, self.master))

    def test_tags_and_durations(self) -> None:
        self.assertClean(check_tags(self.grid))

    def test_idle_frames_preserve_master_pixels(self) -> None:
        self.assertClean(check_idle_preservation(self.grid, self.master))

    def test_ground_contact_and_no_clipping(self) -> None:
        self.assertClean(check_contact(self.grid, self.manifest))

    def test_lint_has_no_errors(self) -> None:
        self.assertClean(check_lint(self.grid))

    def test_head_and_hair_identity(self) -> None:
        self.assertClean(check_identity(self.grid, self.master))

    # -- negative tests
    def test_rejects_changed_idle_pixel(self) -> None:
        g = self.mutated()
        clear_layer(g, 0, "head", 1)
        self.assertRejects(check_idle_preservation(g, self.master), "idle frame 0")

    def test_rejects_idle_pixel_in_padding(self) -> None:
        g = self.mutated()
        put_pixel(g, 1, "torso", 2, 30, "K")
        self.assertRejects(check_idle_preservation(g, self.master), "padding")

    def test_rejects_grounded_frame_lifted_one_row(self) -> None:
        g = self.mutated()
        shift_frame(g, 4, -1)
        self.assertRejects(check_contact(g, self.manifest), "frame 4")

    def test_rejects_airborne_frame_on_ground(self) -> None:
        g = self.mutated()
        shift_frame(g, 12, 55 - bottom_row(g, 12))
        self.assertRejects(check_contact(g, self.manifest), "frame 12")

    def test_rejects_clipping(self) -> None:
        g = self.mutated()
        put_pixel(g, 6, "torso", 0, 30, "K")
        self.assertRejects(check_contact(g, self.manifest), "clipping")

    def test_rejects_broken_palette_and_layers(self) -> None:
        g = self.mutated()
        g["palette"][0]["rgba"] = "#010203ff"
        g["layers"] = g["layers"][::-1]
        errs = check_header(g, self.manifest, self.master)
        self.assertRejects(errs, "palette")
        self.assertTrue(any("layers" in e for e in errs))

    def test_rejects_bad_manifest(self) -> None:
        m = copy.deepcopy(self.manifest)
        m["pivot"] = [24, 55]
        m["locomotion"]["playback"]["run"] = "bounce"
        m["locomotion"]["airborne_tags"].append("idle")
        errs = check_manifest(m, self.master_manifest, self.grid)
        self.assertRejects(errs, "pivot")
        self.assertTrue(any("playback[run]" in e for e in errs))
        self.assertTrue(any("overlap" in e for e in errs))

    def test_rejects_broken_tags_and_durations(self) -> None:
        g = self.mutated()
        g["tags"][1]["from"] += 1
        g["frames"][3]["duration_ms"] = 5
        errs = check_tags(g)
        self.assertRejects(errs, "tag run")
        self.assertTrue(any("duration" in e for e in errs))

    def test_rejects_isolated_pixel(self) -> None:
        g = self.mutated()
        put_pixel(g, 5, "torso", 3, 3, g["palette"][0]["key"])
        self.assertRejects(check_lint(g))

    def test_rejects_hair_change_and_large_head_edit(self) -> None:
        g = self.mutated()
        clear_layer(g, 7, "hair", 1)
        clear_layer(g, 8, "head", HEAD_TOLERANCE + 4)
        errs = check_identity(g, self.master)
        self.assertRejects(errs, "frame 7")
        self.assertTrue(any("frame 8" in e and "head" in e for e in errs))


# ------------------------------------------------------------------ export level

@unittest.skipUnless(ATLAS_JSON.is_file() and ATLAS_PNG.is_file(),
                     "run scripts/export_assets.sh to generate the locomotion atlas")
class CarlLocomotionExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.grid = pga.load_grid(GRID)
        cls.meta = load_json(ATLAS_JSON)

    def test_atlas_rectangles_are_unpadded_row_major(self) -> None:
        frames = self.meta["frames"]
        self.assertEqual(len(frames), 16)
        for i, f in enumerate(frames):
            r = f["frame"]
            self.assertEqual((r["x"], r["y"], r["w"], r["h"]), (i * 48, 0, 48, 56), f"frame {i}")
        with Image.open(ATLAS_PNG) as atlas:
            self.assertEqual(atlas.size, (16 * 48, 56))

    def test_tags_and_durations_match_grid(self) -> None:
        tags = [(t["name"], t["from"], t["to"]) for t in self.meta["meta"]["frameTags"]]
        self.assertEqual(tags, [(t["name"], t["from"], t["to"]) for t in self.grid["tags"]])
        self.assertEqual([f["duration"] for f in self.meta["frames"]],
                         [f["duration_ms"] for f in self.grid["frames"]])

    def test_preview_script_smoke(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertEqual(previews.main(["--no-floor", "--out", str(out)]), 0)
            names = ["contact_sheet_x1.png", "contact_sheet_x6.png", "run_stride_diagram.png", "idle_x4.gif", "run_x4.gif",
                     "jump_sequence_x4.gif", "jump_sequence_x1.gif", "run_scroll_x2.gif",
                     "run_scroll_145_x2.gif", "measurements.json"]
            for name in names:
                self.assertGreater((out / name).stat().st_size, 0, name)
            self.assertEqual(len(list(out.glob("floor01_*"))), 0)
            durations = [f["duration"] for f in self.meta["frames"]]
            for tag in ("idle", "run"):
                a, b = next((x["from"], x["to"]) for x in self.meta["meta"]["frameTags"]
                            if x["name"] == tag)
                expected: list[int] = []
                prev = None
                with Image.open(ATLAS_PNG) as atlas:
                    atlas = atlas.convert("RGBA")
                    for i in range(a, b + 1):
                        img = atlas.crop((i * 48, 0, i * 48 + 48, 56)).tobytes()
                        if img == prev:
                            expected[-1] += durations[i]  # identical neighbours merge in a GIF
                        else:
                            expected.append(durations[i])
                        prev = img
                with Image.open(out / f"{tag}_x4.gif") as gif:
                    actual = []
                    for n in range(gif.n_frames):
                        gif.seek(n)
                        actual.append(gif.info["duration"])
                self.assertEqual(actual, expected, tag)
            report = json.loads((out / "measurements.json").read_text(encoding="utf-8"))
            self.assertEqual(len(report["frames"]), 16)
            self.assertEqual(set(report["sha256"]), set(names) - {"measurements.json"})


if __name__ == "__main__":
    unittest.main()
