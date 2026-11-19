"""LLAMA-02-DEATH (Bad Llama crush, decapitation and heavy-impact death poses, death-parts sheet): spec rules the
shared pixel-art toolkit does not cover. The toolkit validators run with ``python3 -m scripts.pixelart validate`` on
``assets/source/characters/llama/death/llama_death_h64.pixelart.json`` and ``llama_death_parts.pixelart.json``.
See ``docs/02-art/llama-02-death-spec.md`` section 10."""
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_llama02_death as ad  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

LLAMA = ROOT / "assets/source/characters/llama/llama-01.grid.json"
GOB_DEATH = ROOT / "assets/source/characters/goblin/death/goblin_a_death_h36.grid.json"
GOB_PARTS_MANIFEST = ROOT / "assets/manifests/goblin_a_death_parts.json"
POSE_FIXTURE = ROOT / "tests/fixtures/llama-02-death_keys.json"
PART_FIXTURE = ROOT / "tests/fixtures/llama-02-death-parts_keys.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def opaque(rows: list[str]) -> dict[tuple[int, int], str]:
    return {(x, y): ch for y, r in enumerate(rows) for x, ch in enumerate(r) if ch != "."}


class Llama02DeathSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.poses = load(ad.POSES)
        cls.parts = load(ad.PARTS)
        cls.llama = load(LLAMA)

    def test_committed_sources_and_manifests_match_the_generator(self):
        self.assertEqual(ad.POSES.read_text(encoding="utf-8"), ad.dump(ad.poses_grid()))
        self.assertEqual(ad.PARTS.read_text(encoding="utf-8"), ad.dump(ad.parts_grid()))
        for grid_path, ase in ((ad.POSES, ad.POSES_ASE), (ad.PARTS, ad.PARTS_ASE)):
            self.assertEqual(ase.read_bytes(), pg.grid_to_ase(load(grid_path)))
        for name, build in ad.MANIFESTS:
            self.assertEqual((ROOT / "assets/manifests" / name).read_text(encoding="utf-8"), ad.manifest_text(build))

    def test_keys_are_frozen(self):
        self.assertEqual(sorted(load(POSE_FIXTURE)["keys"], key=int), ["0", "2", "3", "5", "6", "7"])
        self.assertEqual(sorted(load(PART_FIXTURE)["keys"], key=int), ["0", "1", "2"])

    def test_llama01_frame_contract_and_grounding(self):
        self.assertEqual(self.poses["canvas"], self.llama["canvas"])
        self.assertEqual(self.poses["pivot"], self.llama["pivot"])
        self.assertEqual([t["name"] for t in self.poses["tags"]],
                         ["crush_contact", "crushed_remains", "decap_contact", "headless_collapse", "slam_contact",
                          "broken_heap"])
        for n in range(len(self.poses["frames"])):
            self.assertEqual(pg._bbox(pg.composite_keys(self.poses, n))[3], 71, n)

    def test_palette_is_llama_plus_the_shared_gore_ramp(self):
        for grid in (self.poses, self.parts):
            pal = {e["key"]: e["rgba"] for e in grid["palette"]}
            self.assertLessEqual(len(pal), 20)
            for e in self.llama["palette"]:
                self.assertEqual(pal[e["key"]], e["rgba"], e["key"])
            gob = {e["key"]: e["rgba"] for e in load(GOB_DEATH)["palette"]}
            self.assertEqual(pal["B"], gob["B"])
            self.assertEqual(pal["b"], gob["R"])        # the goblin's red_mid, shared blood body
            self.assertEqual(pal["P"], gob["P"])

    def test_layer_rule_and_closed_body(self):
        for grid in (self.poses, self.parts):
            self.assertEqual(grid["layers"], ["body", "gore", "molten"])
            w, h = grid["canvas"]
            for n, frame in enumerate(grid["frames"]):
                body = opaque(frame["cels"]["body"])
                gore = set(opaque(frame["cels"]["gore"]).values())
                molten = set(opaque(frame["cels"]["molten"]).values())
                self.assertFalse(set(body.values()) & ad.GORE_ONLY, n)
                self.assertLessEqual(gore, ad.GORE_KEYS, n)
                self.assertLessEqual(molten, ad.MOLTEN_KEYS, n)
                for (x, y), ch in body.items():          # bloodless body is closed: colour only touches opaque pixels
                    if ch == "K":
                        continue
                    for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                        if 0 <= nx < w and 0 <= ny < h:
                            self.assertTrue((nx, ny) in body, (n, x, y))

    def test_crush_stone_footprint_and_gland_burst(self):
        body = opaque(self.poses["frames"][0]["cels"]["body"])
        x0, x1 = ad.STONE
        self.assertFalse([p for p in body if x0 < p[0] < x1 and p[1] < ad.STONE_ROW])   # edge columns: outline only
        line = [x for x in range(x0, x1 + 1) if body.get((x, ad.STONE_ROW)) == "K"]
        self.assertGreaterEqual(len(line), 30)
        molten = opaque(self.poses["frames"][0]["cels"]["molten"])
        self.assertLess(min(y for _x, y in molten), 40)                  # the geyser rises above the stone line
        self.assertGreaterEqual(sum(1 for ch in molten.values() if ch == "Y"), 8)

    def test_decap_body_is_the_llama01_hurt_body_below_the_cut(self):
        hurt = opaque(self.llama["frames"][11]["cels"]["body"])
        body = opaque(self.poses["frames"][3]["cels"]["body"])
        self.assertEqual({p: c for p, c in body.items() if p[1] >= 30}, {p: c for p, c in hurt.items() if p[1] >= 30})
        self.assertFalse({"e", "T", "t"} & set(body.values()))        # no head: no eye, no teeth

    def test_headless_body_is_the_llama01_down_body_behind_the_neck(self):
        dead = opaque(self.llama["frames"][14]["cels"]["body"])
        body = opaque(self.poses["frames"][5]["cels"]["body"])
        self.assertEqual({p: c for p, c in body.items() if p[0] <= 48}, {p: c for p, c in dead.items() if p[0] <= 48})

    def test_preset_one_hides_blood_and_keeps_molten(self):
        for grid in (self.poses, self.parts):
            p1 = ad.preset_view(grid, 1)
            p2 = ad.preset_view(grid, 2)
            for n in range(len(grid["frames"])):
                self.assertNotIn("gore", p1["frames"][n]["cels"])
                self.assertEqual(p1["frames"][n]["cels"]["molten"], grid["frames"][n]["cels"]["molten"])
                muted = set("".join(p2["frames"][n]["cels"]["gore"]))
                self.assertFalse(muted & {"b", "P"}, n)

    def test_parts_manifest_uses_the_goblin_parts_schema(self):
        gob = load(GOB_PARTS_MANIFEST)["death_parts"]
        mine = load(ROOT / "assets/manifests/llama_death_parts.json")["death_parts"]
        self.assertLessEqual(set(gob) - {"parts"}, set(mine))
        required = {"id", "frame", "cell", "pivot", "anchors", "frame0_origin", "class", "material", "presets",
                    "gore_layer", "mass", "spin", "settle", "bounce", "ttl"}
        self.assertEqual([p["id"] for p in mine["parts"]], ["head", "neck", "leg_fore"])
        for entry in mine["parts"]:
            self.assertLessEqual(required, set(entry), entry["id"])
            for name, (x, y) in entry["anchors"].items():
                self.assertTrue(0 <= x < 32 and 0 <= y < 32, (entry["id"], name))

    def test_parts_fit_the_launched_part_cap(self):
        for n in range(len(self.parts["frames"])):
            x0, y0, x1, y1 = pg._bbox(pg.composite_keys(self.parts, n))
            self.assertLessEqual(x1 - x0 + 1, 28, n)
            self.assertLessEqual(y1 - y0 + 1, 28, n)


if __name__ == "__main__":
    unittest.main()
