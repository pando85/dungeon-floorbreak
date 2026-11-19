"""BOSS-F01 (brute_boss_f01, THE JUICER) production art: the 16-frame encounter atlas, the VFX and finishes atlas, the plate
rack, the rotten strut, the ledge with its release plate and the 18 x 18 HUD emblem. Bounds, transparency, palette, frame
counts, pivots, frozen keys, manifest consistency and evidence checksums. The shared toolkit validators run through
``python3 -m scripts.pixelart validate assets/source/enemies/brute_boss_f01/brute_boss_f01.pixelart.json``."""
from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

import author_boss_f01_fx as fx  # noqa: E402
import author_boss_f01_keys as keys  # noqa: E402
import author_boss_f01_set as bs  # noqa: E402
import author_juice01 as jc  # noqa: E402
from scripts.pixelart import validators  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

SRC = ROOT / "assets/source/enemies/brute_boss_f01"
MAN = ROOT / "assets/manifests"
GENERATED = ROOT / "generated/art"
CONFIG = SRC / "brute_boss_f01.pixelart.json"
FIXTURE = ROOT / "tests/fixtures/boss-f01_keys.json"
EVIDENCE = ROOT / "docs/evidence/boss-f01"

# manifest id -> (grid file, frame count, canvas, default pivot)
ATLASES = {
    "boss-f01": (SRC / "brute_boss_f01.grid.json", 16, [160, 136], [72, 135]),
    "boss-f01-vfx": (SRC / "brute_boss_f01_vfx.grid.json", 38, [152, 72], [76, 71]),
    "boss-f01-rack": (SRC / "brute_boss_f01_rack.grid.json", 7, [136, 72], [48, 71]),
    "boss-f01-strut": (SRC / "brute_boss_f01_strut.grid.json", 3, [40, 196], [20, 195]),
    "boss-f01-ledge": (SRC / "brute_boss_f01_ledge.grid.json", 2, [64, 40], [32, 12]),
    "boss-f01-emblem": (SRC / "keys/brute_boss_f01_emblem.grid.json", 1, [18, 18], [0, 0]),
}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def cel(frame: dict) -> list[str]:
    return next(iter(frame["cels"].values()))


def content_box(rows: list[str]):
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, c in enumerate(r) if c != "."]
    return min(xs), min(ys), max(xs), max(ys)


class GeneratorTests(unittest.TestCase):
    def test_committed_files_match_the_generators(self):
        for path, text in bs.targets() + fx.targets():
            self.assertEqual(path.read_text(encoding="utf-8"), text, path.name)

    def test_the_six_keys_are_the_top_tier_keys_unchanged(self):
        bs.check_keys_unchanged()
        committed = load(keys.KEYS_GRID)["frames"]
        grid = load(bs.GRID)
        for name, index in load(MAN / "boss-f01.json")["boss"]["key_atlas_index"].items():
            tag = next(f[2] for f in bs.FRAMES if f[0] == name)
            self.assertEqual(cel(grid["frames"][index]), cel(committed[keys.ORDER.index(tag)]), name)

    def test_keys_are_frozen_by_digest(self):
        grid = load(bs.GRID)
        fixture = load(FIXTURE)
        self.assertEqual(sorted(fixture["keys"], key=int), [str(i) for i in bs.KEY_FRAMES])
        for index, key in fixture["keys"].items():
            self.assertEqual(key["sha256"], validators.frame_digest(grid["frames"][int(index)]["cels"], ["body"]))

    def test_toolkit_validators_pass(self):
        report = validators.run(validators.build_context(load_config(CONFIG)))
        failing = [r for r in report["rules"] if r["status"] == "fail"]
        self.assertEqual(failing, [])


class AtlasContractTests(unittest.TestCase):
    def test_frame_counts_canvas_and_pivots(self):
        for mid, (path, count, canvas, pivot) in ATLASES.items():
            with self.subTest(mid):
                grid, man = load(path), load(MAN / f"{mid}.json")
                self.assertEqual(len(grid["frames"]), count)
                self.assertEqual(grid["canvas"], canvas)
                self.assertEqual(man["frame_size"], canvas)
                self.assertEqual(grid["pivot"], pivot)
                self.assertEqual(man["pivot"], pivot)
                self.assertEqual(len(man["boss"]["frames"]), count)
                for f in man["boss"]["frames"]:
                    px, py = f["pivot"]
                    self.assertTrue(0 <= px < canvas[0] and 0 <= py < canvas[1], f["name"])

    def test_boss_pivot_and_contact_row(self):
        grid = load(bs.GRID)
        for i, f in enumerate(grid["frames"]):
            self.assertEqual(content_box(cel(f))[3], 135, f"frame {i} feet on contact row 135")

    def test_props_stand_on_their_pivot_rows(self):
        for mid in ("boss-f01-rack", "boss-f01-strut"):
            grid = load(ATLASES[mid][0])
            for i, f in enumerate(grid["frames"]):
                want = grid["pivot"][1]
                if mid == "boss-f01-rack" and i == 5:
                    want -= 26                                            # rack_landed_boss: 26 px up
                if mid == "boss-f01-rack" and i == 6:
                    want = 41                                             # rack_pulley: the disc ends on row 41
                self.assertEqual(content_box(cel(f))[3], want, f"{mid} frame {i}")
        ledge = load(ATLASES["boss-f01-ledge"][0])
        for f in ledge["frames"]:
            rows = cel(f)
            self.assertTrue(set(rows[11][20:44]) - {"."}, "the plate or the slab edge sits on the pivot row")

    def test_content_inside_the_frame_with_a_margin(self):
        for mid, (path, _n, canvas, _p) in ATLASES.items():
            grid = load(path)
            for i, f in enumerate(grid["frames"]):
                x0, y0, x1, y1 = content_box(cel(f))
                with self.subTest(f"{mid} frame {i}"):
                    self.assertGreaterEqual(x0, 0 if mid == "boss-f01-emblem" else 1)
                    # the rack's chains run off the top edge on purpose (they continue into the ceiling)
                    self.assertGreaterEqual(y0, 0 if mid in ("boss-f01-emblem", "boss-f01-rack") else 1)
                    self.assertLessEqual(x1, canvas[0] - (1 if mid == "boss-f01-emblem" else 2))
                    if mid in ("boss-f01-vfx", "boss-f01-rack", "boss-f01-strut", "boss-f01-ledge", "boss-f01"):
                        self.assertLessEqual(y1, canvas[1] - 1)

    def test_transparency_every_cel_is_one_bit_and_non_empty(self):
        for mid, (path, _n, canvas, _p) in ATLASES.items():
            grid = load(path)
            declared = {p["key"] for p in grid["palette"]}
            for i, f in enumerate(grid["frames"]):
                rows = cel(f)
                self.assertEqual((len(rows), {len(r) for r in rows}), (canvas[1], {canvas[0]}), f"{mid} frame {i}")
                used = set("".join(rows)) - {"."}
                self.assertTrue(used, f"{mid} frame {i} is empty")
                self.assertLessEqual(used, declared, f"{mid} frame {i}")
                for p in grid["palette"]:
                    self.assertTrue(p["rgba"].endswith("ff"), "palette entries are opaque; transparency is the '.' key only")

    def test_palette_membership_and_limits(self):
        boss_palette = {k: rgba for k, _n, rgba, _a in jc.PALETTE}
        for mid in ("boss-f01", "boss-f01-emblem"):
            grid = load(ATLASES[mid][0])
            self.assertLessEqual(len(grid["palette"]), 19)
            for p in grid["palette"]:
                self.assertEqual(p["rgba"], boss_palette[p["key"]], f"{mid} {p['key']}")
        known = {hexa + "ff" for _n, hexa in fx.COLOURS.values()}
        for mid in ("boss-f01-vfx", "boss-f01-rack", "boss-f01-strut", "boss-f01-ledge"):
            grid = load(ATLASES[mid][0])
            self.assertLessEqual(len(grid["palette"]), 20, mid)
            self.assertLessEqual({p["rgba"] for p in grid["palette"]}, known, mid)

    def test_boss_opaque_heights_follow_the_spec_table(self):
        grid = load(bs.GRID)
        index = {f[0]: i for i, f in enumerate(bs.FRAMES)}
        for name, want in bs.KEY_HEIGHT.items():
            x0, y0, x1, y1 = content_box(cel(grid["frames"][index[name]]))
            self.assertLessEqual(abs((y1 - y0 + 1) - want), 4, name)

    def test_tag_and_tick_table(self):
        man = load(MAN / "boss-f01.json")["boss"]
        got = {}
        for f in man["frames"]:
            got.setdefault(f["tag"], []).append(f["ticks"])
        self.assertEqual(got, {"charge_tell": [6, 6, None], "charge_run": [5, 4, 5, 4], "crash_stun": [3, 3, None],
                               "phase_roar": [6, 24, 30], "pinned": [6, None], "pinned_dead": [None]})
        self.assertEqual(sum(got["phase_roar"]), 60)                      # roar.ticks
        self.assertEqual([f["name"] for f in man["frames"]], [f[0] for f in bs.FRAMES])
        self.assertEqual(len({f["name"] for f in man["frames"]}), 16)

    def test_manifests_hash_the_grid_frames_and_describe_the_rects(self):
        for mid, (path, count, canvas, _p) in ATLASES.items():
            grid, man = load(path), load(MAN / f"{mid}.json")
            for i, f in enumerate(man["boss"]["frames"]):
                layers = list(grid["layers"])
                self.assertEqual(f["sha256"], validators.frame_digest(grid["frames"][i]["cels"], layers), f"{mid} {f['name']}")
                self.assertEqual(f["rect"], {"x": i * canvas[0], "y": 0, "w": canvas[0], "h": canvas[1]})
                self.assertEqual(f["duration_ms"], grid["frames"][i]["duration_ms"])
            self.assertEqual(man["review_status"], "prototype_fixture_unapproved")
            self.assertIn("designed by Claude Opus keys, executed by Claude Sonnet; no AI concept pixels copied", man["provenance"]["generator"])

    def test_vfx_and_prop_counts_per_tag(self):
        vfx = load(MAN / "boss-f01-vfx.json")["boss"]["frames"]
        counts = {}
        for f in vfx:
            counts[f["tag"]] = counts.get(f["tag"], 0) + 1
        self.assertEqual(counts, {"pound_debris": 5, "shockwave_ripple": 4, "crash_dust": 6, "roar_ring": 4, "mirror_crack": 3,
                                  "floor_dust": 5, "boil_spatter": 4, "blood_pool": 4, "rack_shadow": 3})
        rack = [f["name"] for f in load(MAN / "boss-f01-rack.json")["boss"]["frames"]]
        self.assertEqual(rack, ["rack_hang", "rack_shudder_a", "rack_shudder_b", "rack_fall", "rack_landed", "rack_landed_boss", "rack_pulley"])
        self.assertEqual([f["name"] for f in load(MAN / "boss-f01-strut.json")["boss"]["frames"]],
                         ["strut_sound", "strut_cracked", "strut_splintered"])

    def test_review_fixes_round_1(self):
        """Independent review 2026-11-19: foot rule, squash heights, ripple, rack, strut and effect size limits."""
        boss = load(bs.GRID)["frames"]
        idx = {f[0]: i for i, f in enumerate(bs.FRAMES)}
        h = lambda rows: content_box(rows)[3] - content_box(rows)[1] + 1
        self.assertTrue(82 <= h(cel(boss[idx["crash_hit"]])) <= 86)                 # B-04: an impact squash, not a jab
        self.assertLess(h(cel(boss[idx["crash_hit"]])), 90)
        self.assertLessEqual(h(cel(boss[idx["pinned_squash"]])), 48)                # B-05
        for name in ("charge_run_pass_a", "charge_run_pass_b", "crash_slump", "crash_hit"):
            row = cel(boss[idx[name]])[135]
            runs = [r for r in "".join(row).replace(".", " ").split() if r]
            self.assertGreaterEqual(len(runs), 1, name)
            self.assertGreaterEqual(max(len(r) for r in runs), 6, f"{name}: a foot block, not a spike, on row 135")
        vfx = load(SRC / "brute_boss_f01_vfx.grid.json")["frames"]
        for i in range(5, 9):                                                       # V-01: <= 11 rows with a lit lip
            self.assertLessEqual(h(cel(vfx[i])) - 5, 11 + 8, "ripple body plus chips")
            body = [r for r in cel(vfx[i])[58:71] if "h" in r]
            self.assertTrue(body, "chip_hi lip on the ripple crest")
        for i in range(15, 19):
            rows = cel(vfx[i])
            ys = [y for y, r in enumerate(rows) if r.strip(".")]
            self.assertLessEqual(max(ys) - min(ys) + 1, 14, "roar ring flattened to the floor plane")
        for i in range(19, 22):
            x0, y0, x1, y1 = content_box(cel(vfx[i]))
            self.assertLessEqual((x1 - x0 + 1, y1 - y0 + 1), (30, 40), "mirror crack within 30 x 40")
        rack = load(SRC / "brute_boss_f01_rack.grid.json")["frames"]
        for i in (0, 1, 2):                                                         # R-05: nothing above the ceiling row 36
            self.assertEqual(sum(1 for y in range(36) for c in cel(rack[i])[y] if c != "."), 0, f"rack frame {i}")
        fall_above = {x for y in range(36) for x, c in enumerate(cel(rack[3])[y]) if c != "."}
        self.assertTrue(fall_above and max(fall_above) - min(fall_above) <= 8, "only the loose cable end rises above row 36")
        for i in (0, 1, 2):                                                         # R-05: the cleat on the strut cap, one down-run
            rows = cel(rack[i])
            self.assertEqual(set(rows[61][122:128]), {"K"}, "cleat rim")
            self.assertTrue(all(rows[r][122] == "K" for r in range(61, 65)))
            self.assertEqual(rows[60][124], "K", "the down-run ends on the cleat")
            self.assertEqual(sum(1 for r in rows for c in r[86:121] if c in "uyzx"), 0, "no timber beam any more")
        hang = cel(rack[0])
        for n in range(4):                                                          # R-01b: selector hole 2 x 3, weight tags
            y0 = 47 + 6 * n
            self.assertEqual([hang[y][47:49] for y in (y0 + 2, y0 + 3, y0 + 4)], ["KK"] * 3, f"plate {n} selector hole")
            self.assertNotEqual(hang[y0 + 1][47], "K", "the bevel runs across the hole")
            self.assertEqual(set(hang[y0 + 2][26:31]), {"d"}, f"plate {n} weight tag")
            self.assertEqual((hang[y0 + 3][27], hang[y0 + 3][29]), ("K", "K"), "digit pixels")
        self.assertEqual(hang[62][49:54], "kkkkk", "selector pin")
        self.assertEqual([hang[60][53:58], hang[62][54:57], hang[64][53:58]], ["KKKKK", "ddd", "KKKKK"], "selector knob")
        pulley = cel(rack[6])
        self.assertEqual((pulley[34][47:50], pulley[35][47:50], pulley[38][48]), ("KKK", "KKK", "K"), "bracket and axle")
        self.assertEqual(sum(1 for y in range(34) for c in pulley[y] if c != "."), 0)
        shadow = load(SRC / "brute_boss_f01_vfx.grid.json")["frames"]
        for i, (w, hh) in zip(range(35, 38), ((48, 4), (64, 5), (80, 6))):          # V-07
            x0, y0, x1, y1 = content_box(cel(shadow[i]))
            self.assertEqual((x1 - x0 + 1, y1 - y0 + 1, y1), (w, hh, 71), f"rack_shadow_{i - 35}")
            self.assertEqual((x0 + x1 + 1) // 2, 76)
        strut = load(SRC / "brute_boss_f01_strut.grid.json")["frames"]
        for i, f in enumerate(strut[1:], 1):                                        # S-01: damage in the 10..35 px band
            rows = cel(f)
            high = [y for y in range(20, 137) for ch in rows[y] if ch == "h"]
            self.assertEqual(high, [], f"strut state {i}: no torn fibres above the strike band")
            self.assertTrue(any("h" in rows[y] or "K" in rows[y][16:23] for y in range(160, 186)))

    def test_every_frame_is_distinct(self):
        for mid, (path, _n, _c, _p) in ATLASES.items():
            grid = load(path)
            digests = {"".join(cel(f)) for f in grid["frames"]}
            self.assertEqual(len(digests), len(grid["frames"]), mid)


@unittest.skipUnless((GENERATED / "brute_boss_f01.json").is_file(), "run scripts/export_assets.sh first")
class ExportTests(unittest.TestCase):
    def test_exported_atlas_rects_alpha_and_palette(self):
        from PIL import Image
        for mid, (_path, count, canvas, _p) in ATLASES.items():
            man = load(MAN / f"{mid}.json")
            meta = load(ROOT / man["output_metadata"])
            atlas = Image.open(ROOT / man["output_atlas"]).convert("RGBA")
            self.assertEqual(len(meta["frames"]), count)
            self.assertEqual(atlas.size, (canvas[0] * count, canvas[1]))
            self.assertEqual([t["name"] for t in meta["meta"]["frameTags"]], man["animation_tags"])
            for f, want in zip(meta["frames"], man["boss"]["frames"]):
                r = f["frame"]
                self.assertEqual({"x": r["x"], "y": r["y"], "w": r["w"], "h": r["h"]}, want["rect"])
            alphas = {a for *_rgb, a in atlas.getdata()}
            self.assertLessEqual(alphas, {0, 255}, f"{mid}: 1-bit alpha")
            for f in meta["frames"]:
                r = f["frame"]
                crop = atlas.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))
                self.assertIsNotNone(crop.getchannel("A").getbbox(), f"{mid} {f['filename']} is empty")


class EvidenceTests(unittest.TestCase):
    @unittest.skipUnless((EVIDENCE / "SHA256SUMS").is_file(), "evidence not generated")
    def test_evidence_checksums_and_required_files(self):
        for line in (EVIDENCE / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
            digest, name = line.split("  ", 1)
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)
        for name in ("contact_sheet_x1.png", "contact_sheet_x4.png", "scene_640x360.png", "arena_640x360.png",
                     "arena_pinned_640x360.png", "compare/arena_before_after_1x.png", "compare/rack_shadow_1x.png", "silhouette-gate/silhouettes_2x.png", "silhouette-gate/keys_lit_1x.png", "silhouette-gate/gate_pack.md"):
            self.assertTrue((EVIDENCE / name).is_file(), name)
        for tag in ("charge_tell", "charge_run", "crash_stun", "phase_roar", "pinned", "pinned_dead"):
            self.assertTrue((EVIDENCE / f"gif_{tag}_x1.gif").is_file(), tag)

    def test_gate_pack_records_no_verdict(self):
        pack = EVIDENCE / "silhouette-gate/gate_pack.md"
        if pack.is_file():
            text = pack.read_text(encoding="utf-8").lower()
            self.assertIn("no verdict", text)
            self.assertNotIn("verdict: pass", text)


if __name__ == "__main__":
    unittest.main()
