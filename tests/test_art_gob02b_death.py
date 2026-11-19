"""GOB-02B-DEATH (goblin variant B death poses and parts): B is the A death set plus the GOB-01 B accessory layer.
The toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/goblin/death/goblin_b_death_h36.pixelart.json`` and ``goblin_b_death_parts.pixelart.json``.
See ``docs/02-art/gob-02b-death-spec.md`` section 4."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_gob02_death as ad  # noqa: E402
import author_gob02b_death as bd  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GOBLIN_B = ROOT / "assets/source/characters/goblin/goblin_b_h36.grid.json"
HEAD_PARTS = ("head", "head_dead")
ACC_PARTS = ("head", "head_dead", "torso_upper", "ear")
PATCH = ("uuu", "uKu", "uKu")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pix(rows: list[str]) -> dict:
    return {(x, y): ch for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."}


def has_patch(acc: dict) -> bool:
    return any(all(acc.get((x + dx, y + dy)) == PATCH[dy][dx] for dx in range(3) for dy in range(3)) for x, y in acc)


class Gob02bDeathSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.a_poses, cls.a_parts = load(ad.POSES), load(ad.PARTS)
        cls.poses, cls.parts = load(bd.POSES), load(bd.PARTS)

    def test_committed_sources_match_the_generator(self):
        self.assertEqual(bd.POSES.read_text(encoding="utf-8"), ad.dump(bd.poses_grid()))
        self.assertEqual(bd.PARTS.read_text(encoding="utf-8"), ad.dump(bd.parts_grid()))
        for grid_path, ase in ((bd.POSES, bd.POSES_ASE), (bd.PARTS, bd.PARTS_ASE)):
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(load(grid_path)))

    def test_keys_are_frozen_with_the_accessory(self):
        for fixture, keys in (("gob-02b-death_keys.json", ["0", "2", "3", "5"]),
                              ("gob-02b-death-parts_keys.json", ["0", "1", "2"])):
            data = load(ROOT / "tests/fixtures" / fixture)
            self.assertEqual(sorted(data["keys"], key=int), keys)
            self.assertEqual(data["layers"], ["body", "weapon", "accessory", "gore"])

    def test_same_contract_as_a(self):
        """Frame sizes, pivots, palette, tags, durations and frame counts are A's."""
        for a, b in ((self.a_poses, self.poses), (self.a_parts, self.parts)):
            for key in ("format", "canvas", "pivot", "palette", "tags"):
                self.assertEqual(a[key], b[key], key)
            self.assertEqual([f["duration_ms"] for f in a["frames"]], [f["duration_ms"] for f in b["frames"]])
            self.assertEqual(b["layers"], ["body", "weapon", "accessory", "gore"])

    def test_body_weapon_and_gore_cels_are_a_byte_for_byte(self):
        for a, b in ((self.a_poses, self.poses), (self.a_parts, self.parts)):
            for fa, fb in zip(a["frames"], b["frames"]):
                for layer in ("body", "weapon", "gore"):
                    self.assertEqual(fa["cels"][layer], fb["cels"][layer])

    def test_accessory_keys(self):
        """The bandana is R/B cloth on ``accessory``; the wet highlight P stays gore-only; no new colour."""
        for grid in (self.poses, self.parts):
            for f in grid["frames"]:
                keys = {ch for r in f["cels"]["accessory"] for ch in r} - {"."}
                self.assertLessEqual(keys, bd.ACCESSORY_KEYS)
                self.assertNotIn("P", keys)
                for layer in ("body", "weapon"):
                    self.assertFalse({ch for r in f["cels"][layer] for ch in r} & ad.GORE_ONLY)

    def test_head_frames_carry_the_bandana_headless_frames_do_not(self):
        for n in (0, 1, 2):
            acc = pix(self.poses["frames"][n]["cels"]["accessory"])
            self.assertGreaterEqual(sum(ch == "R" for ch in acc.values()), 6, n)
            self.assertGreaterEqual(sum(ch == "B" for ch in acc.values()), 2, n)
            self.assertIn("o", acc.values(), n)                       # the missing front tooth
        for n in (3, 4, 5):                                           # the bandana left with the head
            acc = pix(self.poses["frames"][n]["cels"]["accessory"])
            self.assertFalse({"R", "B", "o"} & set(acc.values()), n)
        for pid in HEAD_PARTS:
            acc = pix(self.parts["frames"][ad.PART_IDS.index(pid)]["cels"]["accessory"])
            self.assertGreaterEqual(sum(ch == "R" for ch in acc.values()), 10, pid)
            self.assertIn("o", acc.values(), pid)

    def test_vest_patch_on_every_pose_and_on_torso_upper(self):
        for n in range(6):
            self.assertTrue(has_patch(pix(self.poses["frames"][n]["cels"]["accessory"])), n)
        self.assertTrue(has_patch(pix(self.parts["frames"][ad.PART_IDS.index("torso_upper")]["cels"]["accessory"])))

    def test_ear_notch(self):
        crush = pix(self.poses["frames"][0]["cels"]["accessory"])
        self.assertEqual((crush.get((7, 34)), crush.get((7, 35))), ("K", "K"))
        ear = pix(self.parts["frames"][ad.PART_IDS.index("ear")]["cels"]["accessory"])
        self.assertEqual(sorted(ear.values()), ["K", "K"])

    def test_only_the_listed_parts_carry_an_accessory(self):
        for i, pid in enumerate(ad.PART_IDS):
            acc = pix(self.parts["frames"][i]["cels"]["accessory"])
            self.assertEqual(bool(acc), pid in ACC_PARTS, pid)

    def test_overlays_follow_the_gob01_b_sources(self):
        """Derived overlays are the GOB-01 B overlays moved by the A rules (and nothing else)."""
        b = load(GOBLIN_B)
        f0 = pix(b["frames"][0]["cels"]["accessory"])
        head = pix(self.parts["frames"][0]["cels"]["accessory"])
        hx, hy = ad.HEAD_SHIFT
        moved = {(x + hx, y + hy): ch for (x, y), ch in f0.items()
                 if y in ad.HEAD_ROWS and ad.HEAD_ROWS[y][0] <= x <= ad.HEAD_ROWS[y][1]}
        self.assertEqual({p: ch for p, ch in head.items() if p in moved}, moved)
        self.assertEqual({ch for p, ch in head.items() if p not in moved}, {"K"})       # closing outline only
        self.assertEqual(head, pix(self.parts["frames"][3]["cels"]["accessory"]))      # head_dead = head
        f25 = pix(b["frames"][25]["cels"]["accessory"])
        remains = pix(self.poses["frames"][2]["cels"]["accessory"])
        for (x, y), ch in f25.items():
            if x <= (22 if y <= 50 else 18) and y <= bd.REMAINS_BAND_END:       # the tails below it were cut (E1)
                self.assertEqual(remains.get((ad.REMAINS_MIRROR_X - x, y)), ch, (x, y))
        p0, p1 = (pix(self.poses["frames"][n]["cels"]["accessory"]) for n in (0, 1))
        self.assertEqual(p1, {(x, y + 3): ch for (x, y), ch in p0.items()})
        p3, p4 = (pix(self.poses["frames"][n]["cels"]["accessory"]) for n in (3, 4))
        self.assertEqual(p4, {(x - 2, y + 4): ch for (x, y), ch in p3.items()})

    def test_headless_patch_sits_on_the_vest_inside_a(self):
        """Review fix E2: pose 5's patch lies on A's rag mass, off the floor row, every pixel inside A's body."""
        a = self.a_poses["frames"][5]["cels"]
        body = pix(a["body"])
        patch = pix(self.poses["frames"][5]["cels"]["accessory"])
        self.assertTrue(has_patch(patch))
        self.assertEqual(len(patch), 9)                                     # uuu / uKu / uKu: no extra pixel
        for (x, y) in patch:
            self.assertIn((x, y), body, (x, y))
            self.assertLess(y, 55, (x, y))                                  # not on the floor row
        self.assertGreaterEqual(sum(body[p] == "r" for p in patch), 6)      # on the dark rag mass, not on skin

    def test_headless_accessory_adds_no_silhouette_pixel(self):
        """The headless frames' accessory (the patch) stays inside A's body and weapon silhouette."""
        for n in (3, 4, 5):
            a = self.a_poses["frames"][n]["cels"]
            sil = set(pix(a["body"])) | set(pix(a["weapon"]))
            for p in pix(self.poses["frames"][n]["cels"]["accessory"]):
                self.assertIn(p, sil, (n, p))

    def test_bandana_never_touches_blood(self):
        """Review fix E1: composited at preset 4, no visible bandana R/B pixel is 4-adjacent to a gore R/B/P pixel, so
        cloth and blood stay separate shapes (they share the R/B keys)."""
        for grid in (self.poses, self.parts):
            w, h = grid["canvas"]
            for n, f in enumerate(grid["frames"]):
                acc, gore = pix(f["cels"]["accessory"]), pix(f["cels"]["gore"])
                comp = pg.composite_keys(grid, n)
                red = {"R", "B", "P"}
                cloth = [p for p, ch in acc.items() if ch in ("R", "B") and p not in gore and comp[p[1]][p[0]] == ch]
                for (x, y) in cloth:
                    for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                        self.assertNotIn(gore.get((nx, ny)), red, (n, x, y, nx, ny))

    def test_remains_band_ends_on_a_dark_cap(self):
        """The remains' band stops at row 52 and a dark K cap closes it (x 48 and 49, row 53)."""
        acc = pix(self.poses["frames"][2]["cels"]["accessory"])
        self.assertEqual(max(y for (x, y), ch in acc.items() if ch in ("R", "B")), bd.REMAINS_BAND_END)
        self.assertEqual((acc.get((48, 53)), acc.get((49, 53))), ("K", "K"))

    def test_accessory_closed_with_gore_hidden(self):
        """Preset 1: body + weapon + accessory is a closed, outlined sprite (no coloured pixel next to transparency)."""
        for grid in (self.poses, self.parts):
            stripped = ad.strip_gore(grid)
            w, h = grid["canvas"]
            for n in range(len(grid["frames"])):
                acc = pix(grid["frames"][n]["cels"]["accessory"])
                comp = pg.composite_keys(stripped, n)
                for (x, y), ch in acc.items():
                    if ch == "K":
                        continue
                    for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                        if 0 <= nx < w and 0 <= ny < h:
                            self.assertNotEqual(comp[ny][nx], ".", (n, x, y))

    def test_palette_budget(self):
        for grid in (self.poses, self.parts):
            used = set()
            for n in range(len(grid["frames"])):
                used |= {ch for r in pg.composite_keys(grid, n) for ch in r} - {"."}
            self.assertLessEqual(len(used), 20)


class Gob02bDeathManifestTests(unittest.TestCase):
    def test_manifests_are_the_generator_output(self):
        for name, m in (("goblin_b_death_h36.json", bd.pose_manifest()), ("goblin_b_death_parts.json", bd.parts_manifest())):
            self.assertEqual(load(ROOT / "assets/manifests" / name), m)

    def test_spawner_contract_is_a(self):
        """Anchors, ticks, part ids, classes, presets and spawner hints equal A's: the spawner needs no code change."""
        a, b = load(ROOT / "assets/manifests/goblin_a_death_h36.json"), load(ROOT / "assets/manifests/goblin_b_death_h36.json")
        self.assertEqual(a["death_poses"]["frames"], b["death_poses"]["frames"])
        self.assertEqual(a["death_poses"]["spawner_hints"], b["death_poses"]["spawner_hints"])
        for key in ("frame_size", "pivot", "animation_tags", "palette_id"):
            self.assertEqual(a[key], b[key])
        a, b = load(ROOT / "assets/manifests/goblin_a_death_parts.json"), load(ROOT / "assets/manifests/goblin_b_death_parts.json")
        self.assertEqual(a["death_parts"]["parts"], b["death_parts"]["parts"])
        self.assertEqual(a["animation_tags"], b["animation_tags"])
        self.assertEqual(b["death_parts"]["variant"], "B")
        self.assertEqual(b["death_parts"]["layers"], ["body", "weapon", "accessory", "gore"])


class Gob02bDeathEvidenceGifTests(unittest.TestCase):
    """The evidence GIFs keep the exact sprite palette: no frame loses the goblin skin or blood red to quantisation."""
    SKIN_MID, RED_MID = (0x7A, 0x9E, 0x36), (0xB8, 0x32, 0x2E)

    def test_every_gif_frame_keeps_the_sprite_colours(self):
        from PIL import Image, ImageSequence
        gifs = sorted((ROOT / "docs/evidence/gob-02b-death").rglob("*.gif"))
        self.assertGreaterEqual(len(gifs), 12)
        for path in gifs:
            with Image.open(path) as im:
                for n, frame in enumerate(ImageSequence.Iterator(im)):
                    colours = {c for _n, c in frame.convert("RGB").getcolors(1 << 16)}
                    self.assertIn(self.SKIN_MID, colours, (path.name, n))
                    self.assertIn(self.RED_MID, colours, (path.name, n))


if __name__ == "__main__":
    unittest.main()
