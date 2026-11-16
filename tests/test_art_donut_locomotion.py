"""ANIM-DONUT-01 candidate: structural and identity checks on the editable grid.

These pin what the art must keep (approved idle, master face and head pixels, palette,
pivot and contact convention, clean clusters). They do not judge artistic quality.
"""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import pixel_grid_to_ase as pg  # noqa: E402

MASTER = ROOT / "assets/source/characters/donut/donut_h40.grid.json"
CANDIDATE = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
MANIFEST = ROOT / "assets/manifests/donut_h40_locomotion.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pixels(rows: list[str]) -> set[tuple[int, int, str]]:
    return {(x, y, c) for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def normalised(px: set[tuple[int, int, str]]) -> set[tuple[int, int, str]]:
    x0 = min(x for x, _, _ in px)
    y0 = min(y for _, y, _ in px)
    return {(x - x0, y - y0, c) for x, y, c in px}


class DonutLocomotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.master = load(MASTER)
        cls.grid = load(CANDIDATE)
        cls.manifest = load(MANIFEST)
        pg.validate_grid(cls.grid)
        cls.tags = {t["name"]: (t["from"], t["to"]) for t in cls.grid["tags"]}

    def frames(self, tag: str) -> list[int]:
        a, b = self.tags[tag]
        return list(range(a, b + 1))

    def test_manifest_matches_grid(self) -> None:
        self.assertEqual(self.manifest["frame_size"], self.grid["canvas"])
        self.assertEqual(self.manifest["pivot"], self.grid["pivot"])
        self.assertEqual(self.manifest["pivot"], [46, 43])
        self.assertEqual(set(self.manifest["animation_tags"]), set(self.tags))
        self.assertEqual(self.manifest["review_status"], "prototype_fixture_unapproved")
        self.assertEqual([len(self.frames(t)) for t in
                          ("idle", "run", "jump_start", "jump_rise", "jump_fall", "land")],
                         [4, 6, 2, 1, 1, 2])

    def test_palette_is_the_master_palette(self) -> None:
        self.assertEqual(self.grid["palette"], self.master["palette"])

    def test_idle_is_the_approved_master_idle(self) -> None:
        dx = self.grid["pivot"][0] - self.master["pivot"][0]
        self.assertEqual(self.grid["pivot"][1], self.master["pivot"][1])
        for i in range(4):
            mine = pg.composite_keys(self.grid, i)
            ref = pg.composite_keys(self.master, i)
            for y, row in enumerate(ref):
                self.assertEqual("".join(mine[y][dx:dx + len(row)]), "".join(row), f"idle {i} row {y}")
                self.assertTrue(all(c == "." for c in mine[y][:dx] + mine[y][dx + len(row):]))
            self.assertEqual(self.grid["frames"][i]["duration_ms"], self.master["frames"][i]["duration_ms"])

    def test_clean_clusters(self) -> None:
        report = pg.lint_grid(self.grid)
        self.assertEqual(report["orphan_count_per_frame"], [0] * len(self.grid["frames"]))
        self.assertEqual(report["isolated_count_per_frame"], [0] * len(self.grid["frames"]))

    def test_contact_rows(self) -> None:
        report = pg.lint_grid(self.grid)
        bottom = {f["frame"]: f["bottom_row"] for f in report["frames"]}
        py = self.grid["pivot"][1]
        run = self.frames("run")
        grounded = (self.frames("idle") + run[:4] + self.frames("jump_start") + self.frames("land")
                    + self.frames("jump_rise") + self.frames("jump_fall"))
        for i in grounded:
            self.assertEqual(bottom[i], py, f"frame {i} must touch the contact row")
        for i in run[4:]:  # gallop suspension: daylight under every paw
            self.assertLess(bottom[i], py, f"run frame {i} is the airborne phase")

    def test_face_and_head_are_master_pixels(self) -> None:
        def master_layer(layer: str, frame: int, rows: slice) -> set:
            return normalised(pixels(self.master["frames"][frame]["cels"][layer][rows]))

        head_ref = master_layer("head", 0, slice(4, 28))
        face_open = master_layer("face", 0, slice(0, 44))
        face_blink = master_layer("face", 2, slice(0, 44))
        blink_frame = self.frames("land")[0]
        for i in range(self.tags["run"][0], len(self.grid["frames"])):
            cels = self.grid["frames"][i]["cels"]
            face = normalised(pixels(cels["face"]))
            self.assertEqual(face, face_blink if i == blink_frame else face_open, f"face frame {i}")
            if i != blink_frame:
                self.assertEqual(normalised(pixels(cels["head"])), head_ref, f"head frame {i}")

    def test_face_stays_put_between_idle_and_run(self) -> None:
        def face_x(i: int) -> int:
            return min(x for x, _, _ in pixels(self.grid["frames"][i]["cels"]["face"]))
        self.assertEqual(face_x(0), face_x(self.frames("run")[0]))

    # ------------------------------------------------------------------
    # Round 2: gait geometry from docs/02-art/donut-locomotion-anim-01-run-spec.md
    # ------------------------------------------------------------------

    def cel(self, frame: int, layer: str) -> list[str]:
        return self.grid["frames"][frame]["cels"].get(layer, ["." * 72] * 44)

    def contacts(self, frame: int) -> dict[str, list[tuple[int, int]]]:
        """Row-43 paw runs per leg layer as inclusive (x0, x1) ranges."""
        out: dict[str, list[tuple[int, int]]] = {}
        for layer in ("legs_near", "legs_far"):
            row = self.cel(frame, layer)[43]
            runs, x = [], 0
            while x < len(row):
                if row[x] != ".":
                    x0 = x
                    while x + 1 < len(row) and row[x + 1] != ".":
                        x += 1
                    runs.append((x0, x))
                x += 1
            if runs:
                out[layer] = runs
        return out

    def assert_range(self, got: tuple[int, int], want: tuple[int, int], msg: str) -> None:
        self.assertLessEqual(abs(got[0] - want[0]), 1, msg)
        self.assertLessEqual(abs(got[1] - want[1]), 1, msg)

    def test_run_footfalls_one_paw_per_frame(self) -> None:
        run = self.frames("run")
        want = {0: ("legs_near", (51, 56)), 1: ("legs_far", (53, 58)),
                2: ("legs_near", (36, 41)), 3: ("legs_far", (32, 37))}
        seen = []
        for k, (layer, rng) in want.items():
            c = self.contacts(run[k])
            self.assertEqual(list(c), [layer], f"run frame {k}: exactly one paw on row 43")
            self.assertEqual(len(c[layer]), 1, f"run frame {k}: one contiguous paw")
            self.assert_range(c[layer][0], rng, f"run frame {k} paw x range")
            seen.append((layer, c[layer][0]))
        for k in (4, 5):  # suspension and descent: daylight under every paw
            self.assertEqual(self.contacts(run[k]), {}, f"run frame {k} has no paw on row 43")
        # A paw is on row 43 in one frame only, so a planted paw can never slide.
        self.assertEqual(len(set(seen)), len(seen))
        for (la, ra), (lb, rb) in zip(seen, seen[1:]):
            self.assertNotEqual(la, lb, "consecutive contacts alternate near/far")

    def test_jump_start_contacts(self) -> None:
        crouch, push = self.frames("jump_start")
        c = self.contacts(crouch)
        self.assertEqual(sorted(c["legs_near"]), [(29, 34), (50, 55)])
        self.assertEqual(sorted(c["legs_far"]), [(33, 38), (54, 59)])
        c = self.contacts(push)  # push-off: hind toes only (fores already lifted)
        self.assertEqual(set(c), {"legs_near", "legs_far"})
        xs = [x for runs in c.values() for r in runs for x in r]
        self.assertGreaterEqual(min(xs), 17)
        self.assertLessEqual(max(xs), 27)
        c = self.contacts(self.frames("jump_rise")[0])
        self.assertEqual(list(c), ["legs_near"])
        self.assert_range(c["legs_near"][0], (12, 16), "rise trailing toe")

    def back_top(self, frame: int, x: int):
        body = self.cel(frame, "body")
        return next((y for y in range(44) if body[y][x] != "."), None)

    def test_back_top_profiles(self) -> None:
        xs = (16, 20, 24, 28, 32, 36)
        spec = {4: (None, None, 24, 21, 21, 21), 5: (None, None, 25, 22, 20, 20),
                6: (None, None, 27, 21, 18, 18), 7: (None, 23, 21, 20, 19, 19),
                8: (21, 20, 20, 20, 20, 20), 9: (23, 21, 21, 21, 21, 21),
                10: (None, None, None, 24, 21, 20), 11: (None, 24, 22, 21, 20, 19),
                12: (26, 23, 22, 21, 20, 19)}
        for frame, row in spec.items():
            for x, want in zip(xs, row):
                got = self.back_top(frame, x)
                if want is None:
                    self.assertIsNone(got, f"frame {frame} x {x}: no body expected")
                else:
                    self.assertIsNotNone(got, f"frame {frame} x {x}")
                    self.assertLessEqual(abs(got - want), 1, f"frame {frame} x {x}: {got} vs {want}")
        run = self.frames("run")
        # The gathered spine is at least 3 px higher than the contact spine at x 32.
        self.assertGreaterEqual(self.back_top(run[0], 32) - self.back_top(run[2], 32), 3)
        # The crouch is not the impact frame: back top y 21 versus 24 at x 32.
        crouch, impact = self.frames("jump_start")[0], self.frames("land")[0]
        self.assertEqual(self.back_top(crouch, 32), 21)
        self.assertEqual(self.back_top(impact, 32), 24)
        self.assertNotEqual(self.grid["frames"][crouch]["cels"], self.grid["frames"][impact]["cels"])

    def test_back_line_steps_are_even(self) -> None:
        """Past the rump curve no step of the back line exceeds 2 px (x 27-36)."""
        for frame in self.frames("run") + [self.frames("jump_start")[1], self.frames("jump_rise")[0]]:
            tops = [self.back_top(frame, x) for x in range(27, 37)]
            if any(t is None for t in tops):
                continue
            for a, b in zip(tops, tops[1:]):
                self.assertLessEqual(abs(a - b), 2, f"frame {frame} back line {tops}")

    def face_xy(self, frame: int) -> tuple[int, int]:
        px = pixels(self.cel(frame, "face"))
        return min(x for x, _, _ in px), min(y for _, y, _ in px)

    def test_run_head_bob_is_one_pixel(self) -> None:
        want_x = [43, 43, 43, 43, 44, 43]
        faces = [self.face_xy(i) for i in self.frames("run")]
        self.assertEqual([f[0] for f in faces], want_x)
        ys = [f[1] for f in faces]
        self.assertEqual(sorted(set(ys)), [20, 21], f"run face y {ys}")
        self.assertEqual(ys, [20, 21, 21, 20, 20, 20])
        self.assertEqual(max(ys) - min(ys), 1)

    def tail_lean(self, frame: int) -> int:
        rows = [r for r in self.cel(frame, "tail") if any(c != "." for c in r)]
        first = lambda r: next(i for i, c in enumerate(r) if c != ".")  # noqa: E731
        return first(rows[0]) - first(rows[-1])

    def test_tail_sway_is_halved(self) -> None:
        run = self.frames("run")
        leans = [self.tail_lean(i) for i in run]
        for k in range(len(run)):  # includes the loop closure 9 -> 4
            a, b = leans[k], leans[(k + 1) % len(run)]
            self.assertLessEqual(abs(a - b), 2, f"tail tip lean jumps {a} -> {b} (leans {leans})")

    def test_durations_follow_the_185_px_s_rule(self) -> None:
        run = self.frames("run")
        self.assertEqual([self.grid["frames"][i]["duration_ms"] for i in run], [80, 80, 80, 80, 90, 80])
        self.assertEqual(self.grid["frames"][self.frames("jump_start")[1]]["duration_ms"], 70)

    def test_keys_and_inbetweens_match_their_recipes(self) -> None:
        import author_donut_locomotion as author

        def used(fi: int) -> dict:
            return {k: v for k, v in self.grid["frames"][fi]["cels"].items()
                    if any(c != "." for r in v for c in r)}

        for recipe in (author.round2_keys(), author.inbetweens()):
            for fi, cels in recipe.items():
                self.assertEqual(used(fi), cels, f"frame {fi} differs from its recipe")


if __name__ == "__main__":
    unittest.main()
