"""GOB-02-DEATH (goblin A crush and decapitation poses, death-parts sheet): spec rules the shared pixel-art toolkit does
not cover. The toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/goblin/death/goblin_a_death_h36.pixelart.json`` and ``goblin_a_death_parts.pixelart.json``.
See ``docs/02-art/gob-02-death-spec.md`` section 10."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_gob02_death as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN_A = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
GOBLIN_B = ROOT / "assets/source/characters/goblin/goblin_b_h36.grid.json"
POSE_FIXTURE = ROOT / "tests/fixtures/gob-02-death_keys.json"
PART_FIXTURE = ROOT / "tests/fixtures/gob-02-death-parts_keys.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def opaque(rows: list[str]) -> set[tuple[int, int]]:
    return {(x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."}


def without_gore(grid: dict) -> dict:
    return ad.strip_gore(grid)


class Gob02DeathSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.goblin = load(GOBLIN_A)

    def test_committed_sources_match_the_generator(self):
        self.assertEqual(ad.POSES.read_text(encoding="utf-8"), ad.dump(ad.poses_grid()))
        self.assertEqual(ad.PARTS.read_text(encoding="utf-8"), ad.dump(ad.parts_grid()))
        for grid_path, ase in ((ad.POSES, ad.POSES_ASE), (ad.PARTS, ad.PARTS_ASE)):
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(load(grid_path)))

    def test_keys_are_frozen(self):
        self.assertEqual(sorted(load(POSE_FIXTURE)["keys"], key=int), ["0", "2", "3", "5"])
        self.assertEqual(sorted(load(PART_FIXTURE)["keys"], key=int), ["0", "1", "2"])

    def test_gob01_frame_contract(self):
        self.assertEqual(self.poses["canvas"], self.goblin["canvas"])
        self.assertEqual(self.poses["pivot"], self.goblin["pivot"])
        self.assertEqual([t["name"] for t in self.poses["tags"]],
                         ["crush_contact", "crushed_remains", "decap_contact", "headless_collapse"])
        for n in range(len(self.poses["frames"])):       # every pose is grounded on row 55
            box = pg._bbox(pg.composite_keys(self.poses, n))
            self.assertEqual(box[3], 55, n)

    def test_palette_is_goblin_floor01_v0_plus_one_blood_highlight(self):
        source = {p["key"]: p["rgba"] for p in self.goblin["palette"] + load(GOBLIN_B)["palette"]}
        for grid in (self.poses, self.parts):
            self.assertLessEqual(len(grid["palette"]), 20)
            new = [p["key"] for p in grid["palette"] if source.get(p["key"]) != p["rgba"]]
            self.assertEqual(new, ["P"])
            self.assertNotIn("e", [p["key"] for p in grid["palette"]])   # no living glare in a death atlas

    def test_blood_keys_live_on_the_gore_layer_only(self):
        for grid in (self.poses, self.parts):
            for n, f in enumerate(grid["frames"]):
                for layer in ("body", "weapon"):
                    used = set("".join(f["cels"][layer]))
                    self.assertFalse(used & ad.GORE_ONLY, (n, layer))

    def test_presets_1_and_2_are_the_same_pixels_with_gore_hidden(self):
        """Hiding the gore layer leaves a clean, outlined sprite: no isolated pixels, under 10 orphans, and every
        coloured body pixel is closed by outline or another opaque pixel (no open wound to transparency)."""
        for grid in (self.poses, self.parts):
            bare = without_gore(grid)
            accents = {e["key"] for e in bare["palette"] if e.get("accent")}
            for n in range(len(bare["frames"])):
                flat = pg.composite_keys(bare, n)
                orphans, isolated = pg._stray_pixels(flat, accents)
                self.assertEqual(len(isolated), 0, n)
                self.assertLess(len(orphans), 10, n)
                h, w = len(flat), len(flat[0])
                for y in range(h):
                    for x in range(w):
                        if flat[y][x] in ".K":
                            continue
                        for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                            inside = 0 <= nx < w and 0 <= ny < h          # the canvas edge counts as closed
                            self.assertFalse(inside and flat[ny][nx] == ".", (n, x, y))

    def test_decap_and_headless_reuse_gob01_pixels(self):
        f16, f25 = self.goblin["frames"][16]["cels"], self.goblin["frames"][25]["cels"]
        decap, headless = self.poses["frames"][3]["cels"], self.poses["frames"][5]["cels"]
        self.assertEqual(decap["weapon"], f16["weapon"])
        self.assertEqual(headless["weapon"], f25["weapon"])
        self.assertEqual(decap["body"][38:], f16["body"][38:])           # recoiling body from the chest down
        for y in range(56):                                               # corpse body right of the neck
            self.assertEqual(headless["body"][y][28:], f25["body"][y][28:], y)

    def test_no_head_on_the_beheaded_poses(self):
        for n in (3, 5):
            body = "".join(self.poses["frames"][n]["cels"]["body"])
            self.assertNotIn("w", body)                                   # no eyes, no teeth: the head is a part
            self.assertNotIn("o", body)

    def test_crush_contact_is_pressed_flat_under_the_stone(self):
        flat = pg.composite_keys(self.poses, 0)
        self.assertTrue(all(ch == "." for row in flat[:33] for ch in row))
        top = flat[33]
        self.assertGreaterEqual(top.count("K"), 36)                       # a long, straight stone-contact line
        self.assertEqual(self.poses["frames"][0]["cels"]["weapon"], ["." * 64] * 56)   # the club has left the hand

    def test_crushed_remains_keep_the_corpse_head(self):
        """Not a paper cut-out: the GOB-01 corpse head (X eyes, ear up, tongue) is reused pixel for pixel."""
        f25 = opaque(self.goblin["frames"][25]["cels"]["body"])
        rem = self.poses["frames"][2]["cels"]["body"]
        src = self.goblin["frames"][25]["cels"]["body"]
        head = [(x, y) for x, y in f25 if x <= 20 and y <= 50]
        self.assertGreater(len(head), 50)
        for x, y in head:
            self.assertEqual(rem[y][x], src[y][x], (x, y))

    def test_parts_fit_their_cell_and_stay_small(self):
        for n in range(len(self.parts["frames"])):
            box = pg._bbox(pg.composite_keys(self.parts, n))
            self.assertGreaterEqual(min(box[0], box[1]), 1)
            self.assertLessEqual(max(box[2], box[3]), 30)
            self.assertLessEqual(box[2] - box[0] + 1, 28)
            self.assertLessEqual(box[3] - box[1] + 1, 28)

    def test_club_keeps_the_gob01_pineapple(self):
        f0 = self.goblin["frames"][0]["cels"]["weapon"]
        club = self.parts["frames"][2]["cels"]["weapon"]
        dx, dy = ad.CLUB_SHIFT
        for y in range(23, 36):
            for x, ch in enumerate(f0[y]):
                if ch in "FGHVv":
                    self.assertEqual(club[y + dy][x + dx], ch, (x, y))
        self.assertEqual("".join(self.parts["frames"][2]["cels"]["gore"]).strip("."), "")


if __name__ == "__main__":
    unittest.main()
