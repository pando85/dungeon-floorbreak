"""BOS-ART: the Ball of Swine's room 18 art (the chain-weight prop key, the kit contract with its `bos` dressing, the SWINE-01
runtime manifest and the evidence pins): the authoring script reproduces the committed grid, the .ase matches it, the palette
stays on floor01-catacomb-v2, the pieces have the sizes the presenter and the spec rely on, and the dressing stands on the
gameplay rects with the torches clear of the alcoves, the roll lane and the weight."""
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_bosart as bos  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

GRID = ROOT / "assets/source/props/floor01/bos_props.grid.json"
ASE = ROOT / "assets/source/props/floor01/bos_props.ase"
CONTRACT = bos.CONTRACT
MANIFEST = ROOT / "assets/manifests/floor01_bos_props.json"
SWINE_MANIFEST = ROOT / "assets/manifests/swine-01.json"
EVIDENCE = ROOT / "docs/evidence/bos-art"
SIZES = {"bos_weight_hung": (80, 36), "bos_weight_landed": (80, 36), "bos_chain_v": (8, 16), "bos_chain_h": (16, 8)}
# The key pins: the route-B draft as gated. A key change is a design change (the top tier re-gates it).
KEY_SHA = {
    "bos_weight_hung": "f0253362bef9cc8421ae99daad9c4291858482b9aa6457ca8c8226cce652da5d",
    "bos_weight_landed": "b9fa7a78259e26e9ce9972f329bb50be9490a794c4ee9ff4e0c5b9dd0ed1ae20",
    "bos_chain_v": "d5f2796b9920cd0a668edb7839d507dd8c678f8ead1c7dac8922fe8f687873c4",
    "bos_chain_h": "43ba16d5d829d7e7afed7a6816087ea2e43a63a10019b95170c4d26411a97438",
}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


class BosArtTests(unittest.TestCase):
    def test_authoring_script_reproduces_the_grid(self):
        grid = bos.kit.grid_from([bos._framed(fn()) for _, fn in bos.PIECES], layer_names=["main"],
                                 durations=[100] * len(bos.PIECES), pivot=(0, 0))
        self.assertEqual(json.dumps(grid, indent=1) + "\n", GRID.read_text(encoding="utf-8"))

    def test_ase_matches_the_grid(self):
        ase = pg.read_ase(ASE.read_bytes())
        self.assertEqual(ase["canvas"], [80, 36])
        self.assertEqual(len(ase["frames"]), 4)

    def test_palette_is_the_catacomb_kit_and_within_budget(self):
        grid = load(GRID)
        self.assertLessEqual(len(grid["palette"]), 63)
        for entry in grid["palette"]:
            ramp, _, idx = entry["name"].rpartition("_")
            self.assertIn(ramp, bos.RAMPS)
            self.assertEqual(bos.RAMPS[ramp][int(idx)] + "ff", entry["rgba"], entry["name"])
            self.assertIn(ramp, ("iron", "brass"), "the weight is iron and brass only")

    def test_piece_sizes_and_the_weights_contact_row(self):
        for pid, fn in bos.PIECES:
            img = fn()
            self.assertEqual((img.w, img.h), SIZES[pid], pid)
        for pid in ("bos_weight_hung", "bos_weight_landed"):
            img = dict(bos.PIECES)[pid]()
            (x0, y0, x1, y1), _ = bos._bbox(img)
            self.assertEqual((x0, x1, y1), (0, 79, 35), "the slab is the sim's 80 px footprint on the contact row")
            slab = [(x, y) for y in range(bos.SLAB_Y0, 36) for x in range(80) if img.px[y][x]]
            self.assertEqual(max(y for _, y in slab) - min(y for _, y in slab) + 1, 24, "the sim's 24 px footprint height")

    def test_key_pins(self):
        self.assertEqual(bos.key_sha(), KEY_SHA)

    def test_the_pieces_hold_no_text_or_symbol_by_construction(self):
        # Only iron and brass ramps, no transparent holes inside the slab (a solid prop, not a sign).
        img = bos.weight_hung()
        for y in range(bos.SLAB_Y0 + 1, 36):
            for x in range(3, 77):
                self.assertIsNotNone(img.px[y][x], (x, y))

    def test_contract_pieces_and_atlas(self):
        c = load(CONTRACT)
        self.assertEqual(c["format"], "env-kit-v2")
        pieces = {p["id"]: p for p in c["pieces"]}
        for pid, (w, h) in SIZES.items():
            self.assertEqual(pieces[pid]["rect"][2:], [w, h], pid)
            self.assertFalse(pieces[pid]["collidable"])
        # The four props sit in the atlas at the frame origins the presenter relies on.
        self.assertEqual([pieces[p]["rect"][:2] for p in SIZES], [[0, 0], [80, 0], [160, 0], [240, 0]])
        # The crops stay inside the structure grid they copy.
        canvas = load(ROOT / "assets/source/terrain/floor01/structure.grid.json")["canvas"]
        for pid in ("bos_post_cap", "bos_post_shaft", "bos_post_base", "bos_lintel"):
            x, y, w, h = pieces[pid]["rect"]
            self.assertTrue(x >= 0 and y >= 0 and x + w <= canvas[0] and y + h <= canvas[1], pid)
        # The manifest names the contract and the same frame size.
        m = load(MANIFEST)
        self.assertEqual(m["frame_size"], [80, 36])
        self.assertEqual(m["kit_contract"], "assets/source/terrain/floor01/floor01_bos_kit.json")
        self.assertEqual(m["review_status"], "prototype_fixture_unapproved")
        self.assertNotIn("Sonnet", json.dumps(m["provenance"]).replace("Claude Sonnet", ""))

    def test_swine_manifest_is_the_existing_source(self):
        m = load(SWINE_MANIFEST)
        self.assertEqual((m["frame_size"], m["pivot"]), ([176, 144], [88, 143]))
        self.assertEqual(m["source_path"], "assets/source/characters/swine/swine-01.ase")
        self.assertEqual(m["grid_source"], "assets/source/characters/swine/swine-01.grid.json")
        self.assertEqual(m["animation_tags"], ["idle", "roll", "attack_tell", "attack_charge", "hurt", "down"])
        self.assertEqual(len(m["swine"]["frames"]), 18)
        self.assertEqual(m["swine"]["playback"]["roll"], "loop")

    def test_the_dressing_stands_on_the_gameplay_rects(self):
        pl = bos.placements()
        floor = bos.FLOOR
        for cx in (104, 152, 488):
            posts = {n: min((p for p in pl if p["piece"] == n), key=lambda p: abs(p["x"] + p["w"] / 2 - cx))
                     for n in ("bos_post_cap", "bos_post_shaft", "bos_post_base")}
            self.assertEqual(posts["bos_post_cap"]["y"], floor - 112, "the cap is at the gameplay lintel's top")
            self.assertEqual(posts["bos_post_base"]["y"] + posts["bos_post_base"]["h"], floor, "the post stands on the deck")
            shaft = posts["bos_post_shaft"]
            self.assertEqual(posts["bos_post_cap"]["y"] + posts["bos_post_cap"]["h"], shaft["y"])
            self.assertEqual(shaft["y"] + shaft["h"], posts["bos_post_base"]["y"], "cap, shaft and base are one post")
            self.assertLessEqual(shaft["x"], cx - 8)
            self.assertGreaterEqual(shaft["x"] + shaft["w"], cx + 8, "the shaft covers the 16 px gameplay post")
        lintels = sorted(p["x"] for p in pl if p["piece"] == "bos_lintel")
        self.assertTrue(lintels[0] <= 96 and lintels[1] + 40 >= 160, "the left lintel spans the 64 px arch")
        self.assertTrue(lintels[2] <= 480 and lintels[3] + 40 >= 544, "the right lintel spans the 64 px arch")
        for p in pl:
            if p["piece"] == "bos_lintel":
                self.assertEqual((p["y"], p["y"] + p["h"]), (floor - 124, floor - 112))

    def test_torches_are_clear_of_the_alcoves_the_lane_and_the_weight(self):
        pl = bos.placements()
        torches = [p for p in pl if p["piece"] == "torch_wall"]
        self.assertEqual(len(torches), 2)
        for t in torches:
            flame_bottom = bos.FLOOR - (t["y"] + 16)
            self.assertGreaterEqual(flame_bottom, 150, "above the ball (134) and Donut's jump apex (138)")
            for a, b in bos.ALCOVES:
                self.assertFalse(a <= t["x"] + 8 <= b, "no flame over an alcove")
            self.assertFalse(169 - 16 <= t["x"] + 8 <= 249 + 16 and flame_bottom < 231, "no flame beside the weight")
        # The composer's four torches, their pools and the timber door are removed.
        block = load(CONTRACT)["room_dressing"]["bos"]
        removed = {(r["piece"], r["x"], r["y"]) for r in block["remove_at"]}
        for t in (("torch_wall", 48, 272), ("torch_wall", 176, 288), ("torch_wall", 416, 288), ("torch_wall", 576, 272),
                  ("door_timber", 216, 328)):
            self.assertIn(t, removed)

    def test_the_girder_carries_the_weights_chain_at_the_sim_ceiling(self):
        g = [p for p in bos.placements() if p["piece"] == "gym_girder"]
        self.assertEqual(bos.FLOOR - (g[0]["y"] + g[0]["h"]), 224)
        self.assertTrue(min(p["x"] for p in g) <= 205 <= max(p["x"] + p["w"] for p in g))

    def test_every_dressed_piece_is_a_known_kit_piece(self):
        rects = {}
        for n in ("floor01_catacomb_kit", "floor01_opening_kit", "floor01_gym_kit", "floor01_bos_kit"):
            for p in load(ROOT / "assets/source/terrain/floor01" / f"{n}.json")["pieces"]:
                rects[p["id"]] = p
        for p in load(CONTRACT)["room_dressing"]["bos"]["place"]:
            self.assertIn(p["piece"], rects, p)

    def test_evidence_pins(self):
        if not (EVIDENCE / "SHA256SUMS").is_file():
            self.skipTest("evidence not generated yet")
        for line in (EVIDENCE / "SHA256SUMS").read_text().splitlines():
            digest, name = line.split(None, 1)
            self.assertEqual(hashlib.sha256((EVIDENCE / name.strip()).read_bytes()).hexdigest(), digest, name)
        text = (EVIDENCE / "measurements.txt").read_text()
        for pid, sha in KEY_SHA.items():
            self.assertIn(f"KEY {pid} sha256={sha}", text)
        lines = (EVIDENCE / "capture_lines.txt").read_text().splitlines()
        self.assertGreaterEqual(len(lines), 6)
        for line in lines:
            self.assertIn("hash_match=yes", line)
            self.assertIn("passes_byte_identical=yes", line)


if __name__ == "__main__":
    unittest.main()
