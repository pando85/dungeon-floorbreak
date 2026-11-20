"""PIPE-01 parity proof: the toolkit on goblin variant A (GOB-01) gives the same verdicts as
``tests/test_art_goblin_gob01.py``, plus evidence and review-pack smoke tests.

The evidence and review-pack tests need the Floor 1 kit atlases (``bash scripts/export_assets.sh``) and are
skipped without them.
"""
from __future__ import annotations

import copy
import hashlib
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))

from scripts.pixelart import cli, evidence, review, validators as v  # noqa: E402
from scripts.pixelart.common import gif_delays, grid_images, pg  # noqa: E402
from scripts.pixelart.config import load_config  # noqa: E402

# The spec table and key list of the existing contract test: the config must mirror them exactly.
from test_art_goblin_gob01 import AIRBORNE, GROUNDED, KEYS, SPEC_TABLE  # noqa: E402

CONFIG = ROOT / "assets/source/characters/goblin/goblin_a_h36.pixelart.json"
FLOOR_ATLAS = ROOT / "generated/art/floor01_terrain.png"
needs_floor = unittest.skipUnless(FLOOR_ATLAS.is_file(), "run scripts/export_assets.sh for the Floor 1 kit atlases")


def status(report: dict, rule_id: str) -> str:
    return next(r["status"] for r in report["rules"] if r["id"] == rule_id)


class ParityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cfg = load_config(CONFIG)
        self.grid = self.cfg.load_grid()

    def run_with(self, grid: dict | None = None, cfg=None) -> dict:
        return v.run(v.build_context(cfg or self.cfg, grid=grid))

    def test_config_mirrors_the_spec_table_of_the_contract_test(self) -> None:
        self.assertEqual({t: [tuple(r) for r in spec["frames"]] for t, spec in self.cfg.tags.items()}, SPEC_TABLE)
        self.assertEqual([t for t, s in self.cfg.tags.items() if s["contact"] == "grounded"], GROUNDED)
        self.assertEqual([t for t, s in self.cfg.tags.items() if s["contact"] == "airborne"], AIRBORNE)
        self.assertEqual(tuple(self.cfg.rules["frozen_keys"]["frames"]), KEYS)
        self.assertEqual((self.cfg.frame_size, self.cfg.pivot, self.cfg.contact_row), ([64, 56], [28, 55], 55))

    def test_every_rule_runs_and_passes_like_the_contract_test(self) -> None:
        report = self.run_with()
        self.assertTrue(report["passed"], v.summary(report))
        self.assertEqual(report["counts"], {"pass": 15, "fail": 0, "skipped": 2})
        self.assertEqual(report["frames"], 27)

    def test_same_measurements_as_the_contract_test(self) -> None:
        lint = pg.lint_grid(self.grid)
        self.assertEqual(lint["isolated_count_per_frame"], [0] * 27)                  # test_lint_isolated_and_orphans
        self.assertTrue(all(n < 10 for n in lint["orphan_count_per_frame"]))
        used = {c for i in range(27) for r in pg.composite_keys(self.grid, i) for c in r} - {"."}
        self.assertEqual(len(used), 19)                                                # variant A uses 19 colours
        self.assertEqual(len(self.grid["palette"]), 19)
        for status_id in ("V04", "V05", "V06", "V07", "V08", "V09", "V10", "V12"):
            self.assertEqual(status(self.run_with(), status_id), "pass", status_id)

    def test_key_fixture_digests_equal_the_committed_fixture(self) -> None:
        fixture = json.loads((ROOT / "tests/fixtures/goblin_gob01_keys.json").read_text(encoding="utf-8"))
        for n in KEYS:
            want = v.frame_digest(fixture["keys"][str(n)], ["body", "weapon"])
            self.assertEqual(v.frame_digest(self.grid["frames"][n]["cels"], ["body", "weapon"]), want, n)

    # ---- negative cases on the real asset: each rule family fails when its contract is broken
    def mutated(self, edit) -> dict:
        grid = copy.deepcopy(self.grid)
        edit(grid)
        return grid

    def test_floating_grounded_frame_fails_v07(self) -> None:
        def edit(g):
            for layer in g["frames"][4]["cels"]:
                g["frames"][4]["cels"][layer][55] = "." * 64
        self.assertEqual(status(self.run_with(self.mutated(edit)), "V07"), "fail")

    def test_grounded_airborne_frame_fails_v08(self) -> None:
        def edit(g):
            g["frames"][18]["cels"]["body"][55] = "." * 20 + "K" * 4 + "." * 40
        self.assertEqual(status(self.run_with(self.mutated(edit)), "V08"), "fail")

    def test_stray_pixel_fails_v05_and_a_touched_key_fails_v09(self) -> None:
        def stray(g):
            g["frames"][3]["cels"]["body"][2] = "..K" + "." * 61
        report = self.run_with(self.mutated(stray))
        self.assertEqual((status(report, "V05"), status(report, "V09")), ("fail", "pass"))

        def touch_key(g):
            row = g["frames"][0]["cels"]["body"][30]
            g["frames"][0]["cels"]["body"][30] = row[:10] + ("K" if row[10] != "K" else "m") + row[11:]
        self.assertEqual(status(self.run_with(self.mutated(touch_key)), "V09"), "fail")

    def test_tick_table_palette_and_readability_rules_fail_when_broken(self) -> None:
        def slow(g):
            g["frames"][5]["duration_ms"] += 10
        self.assertEqual(status(self.run_with(self.mutated(slow)), "V03"), "fail")
        tight = copy.deepcopy(self.cfg.data)
        tight["rules"]["max_colors"] = 18
        self.assertEqual(status(self.run_with(cfg=type(self.cfg)(tight)), "V04"), "fail")

        def yellow_eyes(g):                                # R5: a hurt frame must never show yellow eyes
            row = g["frames"][16]["cels"]["body"][25]
            g["frames"][16]["cels"]["body"][25] = "e" + row[1:]
        self.assertEqual(status(self.run_with(self.mutated(yellow_eyes)), "V14"), "fail")

    def test_validate_cli_accepts_config_manifest_and_grid(self) -> None:
        for target in (CONFIG, ROOT / "assets/manifests/goblin_a_h36.json",
                       ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"):
            self.assertEqual(cli.main(["validate", str(target)]), 0, target.name)

    def test_gif_delays_keep_tick_time(self) -> None:
        delays = gif_delays([12, 8, 12, 8])
        self.assertEqual(delays, [200, 130, 200, 140])
        self.assertAlmostEqual(sum(delays), 40 * 1000 / 60, delta=10)


class EvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.tmp = Path(tempfile.mkdtemp())
        cls.cfg = load_config(CONFIG)

    @classmethod
    def tearDownClass(cls) -> None:
        shutil.rmtree(cls.tmp, ignore_errors=True)

    @needs_floor
    def test_evidence_set_is_complete_checksummed_and_deterministic(self) -> None:
        a, b = self.tmp / "ev_a", self.tmp / "ev_b"
        files = evidence.build_evidence(self.cfg, a)
        evidence.build_evidence(self.cfg, b)
        names = {f.name for f in files}
        for tag in self.cfg.tags:
            self.assertTrue({f"gif_{tag}_x1.gif", f"gif_{tag}_x3.gif"} <= names, tag)
        self.assertTrue({"contact_sheet_x1.png", "contact_sheet_x4.png", "tell_attack_strip.png",
                         "scene_640x360.png", "scene_1920x1080.png", "SHA256SUMS"} <= names)
        self.assertEqual(Image.open(a / "scene_640x360.png").size, (640, 360))
        self.assertEqual(Image.open(a / "scene_1920x1080.png").size, (1920, 1080))
        sums = (a / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(sums), len(files) - 1)
        for line in sums:
            digest, name = line.split("  ", 1)
            self.assertEqual(hashlib.sha256((a / name).read_bytes()).hexdigest(), digest, name)
        self.assertEqual((a / "SHA256SUMS").read_text(encoding="utf-8"), (b / "SHA256SUMS").read_text(encoding="utf-8"))
        gif = Image.open(a / "gif_attack_swing_x1.gif")
        self.assertEqual(gif.n_frames, 2)

    @needs_floor
    def test_review_pack_is_exactly_three_images_and_the_report(self) -> None:
        out = self.tmp / "pack"
        report = review.build_review_pack(self.cfg, out)
        self.assertTrue(report["passed"])
        self.assertEqual(sorted(p.name for p in out.iterdir() if p.suffix in (".png", ".gif", ".jpg")), sorted(review.IMAGE_NAMES))
        self.assertEqual(sorted(p.name for p in out.iterdir()), sorted(review.IMAGE_NAMES + review.TEXT_NAMES))
        self.assertEqual(Image.open(out / review.IMAGE_NAMES[1]).size, (1920, 1080))
        self.assertLessEqual(Image.open(out / review.IMAGE_NAMES[0]).width, 1800)
        on_disk = json.loads((out / "validator_report.json").read_text(encoding="utf-8"))
        self.assertEqual(on_disk["counts"], {"pass": 15, "fail": 0, "skipped": 2})
        self.assertIn("verdict: PASS", (out / "validator_summary.txt").read_text(encoding="utf-8"))

    @needs_floor
    def test_review_pack_command_refuses_failing_validators(self) -> None:
        data = copy.deepcopy(self.cfg.data)
        data["rules"]["max_colors"] = 5
        bad = self.tmp / "bad.pixelart.json"
        bad.write_text(json.dumps(data), encoding="utf-8")
        out = self.tmp / "refused"
        self.assertEqual(cli.main(["review-pack", str(bad), "--out", str(out)]), 1)
        self.assertFalse(out.exists())
        self.assertEqual(cli.main(["review-pack", str(bad), "--out", str(out), "--allow-failing"]), 0)
        self.assertEqual(len(list(out.glob("*.png"))), 3)

    def test_contact_sheet_and_strip_need_no_scene(self) -> None:
        imgs = grid_images(self.cfg.load_grid())
        sheet = evidence.contact_sheet(self.cfg, imgs, 4)
        self.assertLessEqual(sheet.width, 1800)
        self.assertEqual(evidence.strip_tags(self.cfg), ["attack_tell", "attack_swing", "attack_recover"])
        self.assertGreater(evidence.tell_attack_strip(self.cfg, imgs).width, 1000)
        seq = evidence.tick_sequence(self.cfg, "knockback")               # hold tag: last frame rests
        self.assertEqual(seq, [(18, 4), (19, 30)])
        self.assertEqual(evidence.tick_sequence(self.cfg, "idle"), [(0, 12), (1, 8), (2, 12), (3, 8)])


if __name__ == "__main__":
    unittest.main()
