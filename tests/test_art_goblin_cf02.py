"""ART-GOB-CF02 (goblin acting and slammed sheets, variants A and B). The toolkit validators run with
``python3 -m scripts.pixelart validate`` on the four ``*.pixelart.json`` configs next to the grids. See
``docs/02-art/cf02-acting-spec.md`` sections 5, 6 and 9 and ``docs/evidence/cf02-goblin/README.md``."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))

import author_goblin_cf02 as cf  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

PIVOT_ROW = 55
SKIN = set("aslm")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pix(grid: dict, frame: int, name: str) -> dict:
    rows = grid["frames"][frame]["cels"].get(name)
    return {} if rows is None else cf.ad.cel_pix(rows)


def flat(grid: dict, frame: int) -> dict:
    return {(x, y): ch for y, row in enumerate(pg.composite_keys(grid, frame)) for x, ch in enumerate(row) if ch != "."}


class Cf02GoblinTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grids = {k: load(s["grid"]) for k, s in cf.SHEETS.items()}
        cls.acting_keys, cls.slammed_keys = load(cf.ACTING_KEYS), load(cf.SLAMMED_KEYS)

    # ------------------------------------------------------------------ sources
    def test_committed_files_match_the_generator(self):
        for path, text in cf.outputs():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)
        for key, s in cf.SHEETS.items():
            self.assertEqual(s["ase"].read_bytes(), pg.grid_to_ase(self.grids[key]), s["ase"].name)
            self.assertEqual(load(s["manifest"]), cf.manifest_for(key), s["manifest"].name)

    def test_lint_is_strict_clean(self):
        for key, grid in self.grids.items():
            report = pg.lint_grid(grid)
            self.assertEqual(pg.lint_problems(report), [], key)

    def test_frame_counts_fit_the_q73_budget(self):
        """5 acting + 6 slammed = 11 frames per goblin (Q73); the two goblins are A and B."""
        for variant in "ab":
            self.assertEqual(len(self.grids[f"acting_{variant}"]["frames"]), 5)
            self.assertEqual(len(self.grids[f"slammed_{variant}"]["frames"]), 6)
        self.assertEqual([t["name"] for t in self.grids["acting_a"]["tags"]], ["hesitate", "panic", "taunt"])
        self.assertEqual([t["name"] for t in self.grids["slammed_a"]["tags"]],
                         ["slammed_stomped", "slammed_wall", "slammed_flung"])

    def test_canvas_pivot_and_clipping_columns(self):
        for key, grid in self.grids.items():
            self.assertEqual((grid["canvas"], grid["pivot"]), ([64, 56], [28, 55]), key)
            for n in range(len(grid["frames"])):
                f = flat(grid, n)
                xs = [x for x, _y in f]
                self.assertGreaterEqual(min(xs), 1, (key, n))
                self.assertLessEqual(max(xs), 62, (key, n))
                self.assertEqual(max(y for _x, y in f), PIVOT_ROW, (key, n))      # lowest row on the pivot row

    # ------------------------------------------------------------------ keys
    def test_keys_are_copied_pixel_for_pixel(self):
        for kind, keys, sheets in (("acting", self.acting_keys, ("acting_a", "acting_b")),
                                   ("slammed", self.slammed_keys, ("slammed_a", "slammed_b"))):
            for frame, src in cf.KEY_SOURCE_INDEX[kind].items():
                for sheet in sheets:
                    for name in ("weapon_back", "body", "weapon", "gore"):
                        if name in self.grids[sheet]["layers"]:
                            self.assertEqual(pix(self.grids[sheet], frame, name), pix(keys, src, name),
                                             (sheet, frame, name))

    def test_derived_frames_come_from_the_named_sources(self):
        walk = load(cf.GOBLIN_A)
        a = self.grids["acting_a"]
        legs8 = {(x, y): c for (x, y), c in pix(walk, 8, "body").items() if y >= 48}
        body = pix(a, 2, "body")
        self.assertEqual({p: c for p, c in body.items() if p[1] >= 48}, legs8)             # walk frame 8 legs
        self.assertEqual({(x, 47): c for (x, y), c in legs8.items() if y == 48},
                         {p: c for p, c in body.items() if p[1] == 47 and p in {(x, 47) for (x, _y) in legs8}})
        thump_club = pix(a, 1, "weapon")                                                    # club 1 px higher
        self.assertEqual(pix(a, 2, "weapon"), {(x, y - 1): c for (x, y), c in thump_club.items()})
        self.assertEqual(pix(a, 4, "weapon_back"), pix(a, 3, "weapon_back"))               # beckon keeps the club behind
        self.assertEqual(pix(a, 4, "weapon"), pix(a, 3, "weapon"))

    # ------------------------------------------------------------------ rules of the spec
    def test_derived_frames_and_b_open_no_background_holes(self):
        """Review rule (Donut #142): no enclosed transparent pixel or one-pixel slit that the frozen key or variant A does
        not already have: derived frames have none of their own, variant B adds none to A."""
        def comp(sheet, n):
            return flat(self.grids[sheet], n)
        self.assertEqual(cf.enclosed_pixels(comp("acting_a", 2)), set())                 # panic_flee_b: none at all
        self.assertEqual(cf.enclosed_pixels(comp("acting_a", 4)), cf.enclosed_pixels(comp("acting_a", 3)))
        for kind, n_frames in (("acting", 5), ("slammed", 6)):
            for n in range(n_frames):
                a, b = comp(f"{kind}_a", n), comp(f"{kind}_b", n)
                self.assertEqual(cf.enclosed_pixels(b) - cf.enclosed_pixels(a), set(), (kind, n))
                self.assertEqual(cf.pinched_pixels(b) - cf.pinched_pixels(a), set(), (kind, n))

    def test_clubs_stay_below_the_shoulder_line(self):
        """Hesitate, panic and taunt clubs never rise above the shoulders (GOB-01 R1: never the attack-tell sign)."""
        tops = {"hesitate": 28, "panic_flee": 40, "panic_flee_b": 40, "taunt_thump": 34, "taunt_beckon": 34}
        for n, name in enumerate(cf.ACTING_NAMES):
            for sheet in ("acting_a", "acting_b"):
                club = [y for layer in ("weapon", "weapon_back") for (_x, y) in pix(self.grids[sheet], n, layer)]
                self.assertGreaterEqual(min(club), tops[name], (sheet, name))
        for n in range(len(cf.ACTING_NAMES)):
            pine = [y for layer in ("weapon", "weapon_back") for (_x, y), c in pix(self.grids["acting_a"], n, layer).items()
                    if c in "FGH"]
            self.assertGreaterEqual(min(pine), 34, n)                        # the pineapple itself, shoulder line is row 28

    def test_panic_frames_bounce_and_face_the_flee_direction(self):
        a = self.grids["acting_a"]
        top = [min(y for (_x, y) in flat(a, n)) for n in (1, 2)]
        self.assertEqual(top, [18, 20])            # the raised arm drops 2 px while the body rises 1
        for n in (1, 2):
            f = flat(a, n)
            feet = [x for (x, y) in f if y == PIVOT_ROW]
            self.assertGreater(max(feet), 28)       # the leading foot is on the right (the flee direction)

    def test_taunt_beckon_is_an_open_palm_with_no_spur(self):
        """The palm-up hand is at least 3 px wide and 3 px tall, fingers together, with no single-pixel column above the
        knuckles (a lone spur at 1x reads as a raised middle finger)."""
        body = pix(self.grids["acting_a"], 4, "body")
        hand = {p: c for p, c in body.items() if p[0] >= 41 and p[1] >= 33 and c in SKIN}
        cols: dict[int, list[int]] = {}
        for x, y in sorted(hand):
            cols.setdefault(x, []).append(y)
        self.assertGreaterEqual(len(cols), 3)
        for x, ys in cols.items():
            self.assertGreaterEqual(len(ys), 3, x)                         # every hand column is at least 3 px tall
            self.assertEqual(ys, list(range(ys[0], ys[-1] + 1)), x)        # and unbroken: the fingers are together
        top = min(ys[0] for ys in cols.values())
        run = [x for x, ys in cols.items() if ys[0] == top]
        self.assertGreaterEqual(len(run), 3)                               # the top edge is a flat run, not a spur
        self.assertEqual(run, list(range(run[0], run[-1] + 1)))
        self.assertFalse([p for p, c in body.items() if p[0] >= 41 and 33 <= p[1] < top and c in SKIN])

    def test_wall_frames_keep_the_contact_column(self):
        for variant in "ab":
            grid = self.grids[f"slammed_{variant}"]
            for frame, column in ((2, 16), (3, 14)):
                xs = [x for (x, _y) in flat(grid, frame)]
                self.assertEqual(min(xs), column, (variant, frame))
        man = load(cf.SHEETS["slammed_a"]["manifest"])["cf02_goblin"]["finishes"]["wall"]
        self.assertEqual((man["contact_column"], man["rest_contact_column"]), (16, 14))
        self.assertIn("presenter", man["presenter_contract"])

    def test_gore_is_a_separate_layer(self):
        """Body, weapon and accessory never hold blood-only keys; gore holds blood keys only."""
        for variant in "ab":
            grid = self.grids[f"slammed_{variant}"]
            for n in range(6):
                for layer in ("body", "weapon"):
                    self.assertFalse(set(pix(grid, n, layer).values()) & set("BP"), (variant, n, layer))
                self.assertFalse(set(pix(grid, n, "gore").values()) - set("BPRowKw"), (variant, n))

    def test_slammed_rests_are_told_apart(self):
        """Blind-check geometry: stomped rest has its legs up (top row above the corpse's), the flung rest is a low heap,
        the wall rest sits upright on the wall."""
        g = self.grids["slammed_a"]
        tops = [min(y for (_x, y) in flat(g, n)) for n in (1, 3, 5)]
        self.assertEqual(tops, [37, 33, 40])
        death = load(cf.ad.POSES)
        remains = min(y for (_x, y) in flat(death, 2))
        self.assertLess(tops[0], remains)                    # legs up: higher than crushed_remains
        self.assertLess(tops[1], tops[0])                    # a seated torso is the tallest rest
        self.assertGreaterEqual(tops[2], remains)            # the face-down heap is as low as the remains

    # ------------------------------------------------------------------ goblin B
    def test_b_is_a_plus_the_accessory_layer(self):
        for kind in ("acting", "slammed"):
            a, b = self.grids[f"{kind}_a"], self.grids[f"{kind}_b"]
            self.assertEqual([t for t in a["tags"]], [t for t in b["tags"]])
            self.assertEqual([f["duration_ms"] for f in a["frames"]], [f["duration_ms"] for f in b["frames"]])
            self.assertIn("accessory", b["layers"])
            for n in range(len(a["frames"])):
                for layer in a["layers"]:
                    self.assertEqual(pix(a, n, layer), pix(b, n, layer), (kind, n, layer))

    def test_every_b_frame_wears_the_headband(self):
        for kind, n_frames in (("acting", 5), ("slammed", 6)):
            grid = self.grids[f"{kind}_b"]
            for n in range(n_frames):
                acc = pix(grid, n, "accessory")
                self.assertGreaterEqual(sum(ch in "RB" for ch in acc.values()), 6, (kind, n))

    def test_headband_never_touches_blood(self):
        """No headband R/B pixel overlaps or touches (8-neighbourhood) a gore pixel, so cloth and blood stay separate
        shapes although they share keys (and B has the same RGBA as blood_dark)."""
        grid = self.grids["slammed_b"]
        for n in range(6):
            acc, gore = pix(grid, n, "accessory"), pix(grid, n, "gore")
            for (x, y), ch in acc.items():
                if ch in "RB":
                    near = [(x + a, y + b) for a in (-1, 0, 1) for b in (-1, 0, 1)]
                    self.assertFalse([p for p in near if p in gore], (n, x, y))

    def test_accessory_keys(self):
        for variant in "ab":
            for sheet in (f"acting_{variant}", f"slammed_{variant}"):
                grid = self.grids[sheet]
                for n in range(len(grid["frames"])):
                    keys = set(pix(grid, n, "accessory").values())
                    self.assertFalse(keys & set("P"), (sheet, n))
        a = self.grids["acting_a"]
        self.assertNotIn("accessory", a["layers"])

    def test_missing_tooth_sits_on_a_tooth(self):
        for sheet in ("acting_b", "slammed_b"):
            grid, base = self.grids[sheet], self.grids[sheet.replace("_b", "_a")]
            for n in range(len(grid["frames"])):
                for p, ch in pix(grid, n, "accessory").items():
                    if ch == "o":
                        self.assertEqual(pix(base, n, "body").get(p), "w", (sheet, n, p))

    # ------------------------------------------------------------------ manifests and configs
    def test_manifest_tick_ranges_and_blocks(self):
        for key, s in cf.SHEETS.items():
            m = load(s["manifest"])
            self.assertEqual(m["animation_tags"], [t[0] for t in s["tags"]])
            self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
            block = m["cf02_goblin"]
            self.assertEqual(block["tick_hz"], 60)
            self.assertEqual(block["key_fixture"], str(cf.KEY_FIXTURES[s["kind"]].relative_to(ROOT)))
            for row in block["frames"]:
                self.assertEqual(row["duration_ms"], cf.ms(row["ticks"]), (key, row["index"]))
        acting = load(cf.SHEETS["acting_a"]["manifest"])["cf02_goblin"]
        self.assertEqual(acting["reactions"]["hesitate"]["ticks"], 20)
        self.assertEqual(acting["reactions"]["panic"]["ticks_per_frame"], 6)
        self.assertEqual(acting["reactions"]["taunt"]["loop_ticks"], 36)
        slammed = load(cf.SHEETS["slammed_a"]["manifest"])["cf02_goblin"]
        for finish in slammed["finishes"].values():
            self.assertEqual(finish["impact_hold_ticks"], 8)

    def test_toolkit_validators_pass(self):
        for key, s in cf.SHEETS.items():
            cfg = load_config(s["config"])
            report = validators.run(validators.build_context(cfg))
            self.assertTrue(report["passed"], (key, validators.summary(report)))

    def test_b_sheets_share_the_a_key_digests(self):
        """The B configs freeze the same body, weapon (and gore) digests as A: the keys are one set of pixels."""
        for kind in ("acting", "slammed"):
            fixture = load(cf.KEY_FIXTURES[kind])
            self.assertEqual(sorted(int(k) for k in fixture["keys"]), cf.KEY_FRAMES[kind])

    def test_goblin_a_and_b_gob01_sheets_untouched(self):
        """The new sheets are separate files: the approved GOB-01 sheet names are not reused."""
        stems = {s["stem"] for s in cf.SHEETS.values()}
        self.assertFalse(stems & {"goblin_a_h36", "goblin_b_h36", "goblin_a_death_h36", "goblin_b_death_h36"})


if __name__ == "__main__":
    unittest.main()
