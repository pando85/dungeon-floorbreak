"""LLAMA-03: the Bad Llama rush design keys (brace, scrape, run_contact, recover_skid).

Spec: docs/02-art/llama-03-rush-spec.md. These tests check the committed design-key grid against its authoring
script, the frozen-key fixture (keyed by the final rush-atlas indices 1, 2, 4 and 8), the frame contract shared
with LLAMA-01, the no-spit rule (zero glow keys), the silhouette targets of spec section 9 and the gate evidence
checksums.
"""
from __future__ import annotations

import contextlib
import hashlib
import io
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

import author_llama03_rush as author  # noqa: E402
import pixel_grid_to_ase as pga  # noqa: E402
from scripts.pixelart import validators as val  # noqa: E402

GRID = author.KEYS_GRID
FIXTURE = author.FIXTURE
LLAMA01 = ROOT / "assets/source/characters/llama/llama-01.grid.json"
EVIDENCE = ROOT / "docs/evidence/llama-03-rush"
NAMES = ["brace", "scrape", "run_contact", "recover_skid"]
ATLAS = {"brace": 1, "scrape": 2, "run_contact": 4, "recover_skid": 8}
GLOW = set("rRYO")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def frames() -> dict[str, list[str]]:
    grid = load(GRID)
    return {t["name"]: grid["frames"][t["from"]]["cels"]["body"] for t in grid["tags"]}


def bbox(rows: list[str]) -> tuple[int, int, int, int]:
    pts = [(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def count(rows: list[str], key: str) -> int:
    return sum(r.count(key) for r in rows)


class ContractTests(unittest.TestCase):
    def test_the_grid_is_the_authoring_script_output(self) -> None:
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(author.main(["keys", "--check"]), 0, "run: python3 scripts/author_llama03_rush.py keys")

    def test_four_keys_on_the_llama01_frame_pivot_and_palette(self) -> None:
        grid, l1 = load(GRID), load(LLAMA01)
        self.assertEqual(grid["canvas"], [96, 72])
        self.assertEqual(grid["pivot"], [44, 71])
        self.assertEqual(grid["palette"], l1["palette"], "the rush reuses llama-floor01-v0 unchanged")
        self.assertEqual([t["name"] for t in grid["tags"]], NAMES)
        self.assertEqual(grid["layers"], ["body"])

    def test_the_fixture_freezes_the_keys_under_their_atlas_indices(self) -> None:
        fix, grid = load(FIXTURE), load(GRID)
        self.assertEqual(fix["asset_id"], "llama-03-rush")
        self.assertEqual(fix["format"], "pixelart-keys-v1")
        self.assertEqual(sorted(fix["keys"], key=int), ["1", "2", "4", "8"])
        for i, name in enumerate(NAMES):
            cels = grid["frames"][i]["cels"]
            self.assertEqual(fix["keys"][str(ATLAS[name])]["sha256"], author.digest(cels), name)

    def test_the_ase_source_round_trips_to_the_grid(self) -> None:
        ase = pga.normalize(pga.ase_to_grid(author.KEYS_ASE.read_bytes()))
        grid = pga.load_grid(GRID)
        self.assertEqual([f["cels"] for f in ase["frames"]], [f["cels"] for f in grid["frames"]])


class ReadabilityTests(unittest.TestCase):
    def test_a_rush_frame_never_carries_a_glow_key(self) -> None:
        for name, rows in frames().items():
            self.assertEqual(sum(count(rows, k) for k in GLOW), 0, f"{name}: the rush must never read as a spit")

    def test_every_key_stands_on_contact_row_71(self) -> None:
        for name, rows in frames().items():
            self.assertEqual(bbox(rows)[3], 71, name)
            self.assertGreaterEqual(rows[69].count("h"), 3, f"{name}: a hoof stamp sits on the floor (rows 68 to 71)")

    def test_the_head_is_low_unlike_the_idle_and_the_spit_tell(self) -> None:
        tops = {name: bbox(rows)[1] for name, rows in frames().items()}
        self.assertGreaterEqual(tops["brace"], 20)       # LLAMA-01 idle top 8, spit tell top <= 4
        self.assertGreaterEqual(tops["scrape"], 20)
        self.assertGreaterEqual(tops["run_contact"], 24)
        self.assertGreaterEqual(tops["recover_skid"], 28)

    def test_the_head_points_down_the_lane_at_chest_height(self) -> None:
        for name, rows in frames().items():
            ahead = [y for y, r in enumerate(rows) for x in range(76, 96) if r[x] != "."]
            if name == "recover_skid":
                self.assertGreaterEqual(min(ahead), 40, "the recover head pitches down, never up")
                continue
            self.assertGreaterEqual(min(ahead), 33, name)
            self.assertLessEqual(max(ahead), 56, name)
        run = frames()["run_contact"]
        self.assertGreaterEqual(bbox(run)[2], 86, "the run reaches far ahead of the pivot")

    def test_identity_cues_teeth_and_eye(self) -> None:
        for name, rows in frames().items():
            self.assertGreaterEqual(count(rows, "T"), 6, f"{name}: yellow buck teeth")
            if name == "recover_skid":
                self.assertEqual(count(rows, "e"), 0, "the winded squint has no eye white")
            else:
                self.assertGreaterEqual(count(rows, "e"), 2, name)

    def test_brace_and_scrape_differ_only_where_the_paw_and_rock_are(self) -> None:
        f = frames()
        self.assertNotEqual(f["brace"], f["scrape"])
        paw = [x for y in range(60, 72) for x in range(44, 60) if f["scrape"][y][x] == "h"]
        self.assertTrue(paw and min(paw) <= 52, "the near fore hoof is drawn back under the chest")

    def test_the_scrape_hind_hooves_stay_on_the_brace_footprint(self) -> None:
        """Gate K2 / spec rule 5: rows 66 to 71, columns 0 to 39 of the scrape equal the brace's."""
        f = frames()
        for y in range(66, 72):
            self.assertEqual(f["scrape"][y][:40], f["brace"][y][:40], f"row {y}")

    def test_the_winded_tongue_hangs_below_the_jaw(self) -> None:
        """Gate K1: the gum-coloured tongue (key g) of the recover key reaches 2 px below the jaw line."""
        rows = frames()["recover_skid"]
        g = [y for y, r in enumerate(rows) for ch in r if ch == "g"]
        self.assertGreaterEqual(len(g), 7)
        self.assertGreaterEqual(max(g), 56)

    def test_lint_orphans_isolated_and_pockets(self) -> None:
        import author_llama01 as rig
        for name, rows in frames().items():
            orphans, isolated = pga._stray_pixels([list(r) for r in rows], rig.ACCENTS)
            self.assertLessEqual(len(orphans), 9, name)
            self.assertEqual(len(isolated), 0, name)
            for pocket in val.find_holes([list(r) for r in rows], 4):
                xs, ys = [p[0] for p in pocket], [p[1] for p in pocket]
                tail = max(xs) <= 24 and max(ys) <= 46          # the rig's tail-mane pinholes, as on LLAMA-01
                legs = len(pocket) <= 4 and min(ys) >= 64       # the gap between two touching hooves
                self.assertTrue(tail or legs, f"{name}: unexpected enclosed pocket {pocket}")


class EvidenceTests(unittest.TestCase):
    def test_gate_images_match_their_checksums(self) -> None:
        sums = (EVIDENCE / "SHA256SUMS").read_text(encoding="utf-8").split("\n")
        listed = {}
        for line in filter(None, sums):
            digest, name = line.split("  ", 1)
            listed[name] = digest
        self.assertEqual(set(listed), {p.name for p in EVIDENCE.glob("*.png")})
        for name, digest in listed.items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)


if __name__ == "__main__":
    unittest.main()
