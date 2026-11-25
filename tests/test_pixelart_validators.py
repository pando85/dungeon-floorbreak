"""PIPE-01: unit tests of every toolkit validator on tiny synthetic grids, with negative cases."""
from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from scripts.pixelart import validators as v  # noqa: E402
from scripts.pixelart.common import ToolkitError, pg  # noqa: E402
from scripts.pixelart.config import AssetConfig, default_config  # noqa: E402

W, H, PIVOT = 8, 8, [3, 7]
BLANK = "." * W


def rows(*lines: str) -> list[str]:
    """Bottom-aligned art: the given lines are the lowest rows of the 8 x 8 cel."""
    pad = [BLANK] * (H - len(lines))
    return pad + [ln.ljust(W, ".") for ln in lines]


# a 3 x 3 block of 'm' on the floor (rows 5..7), x 2..4
BLOCK = rows("..mmm...", "..mmm...", "..mmm...")
BLOCK_NOTCH = rows("..mmm...", "..mmmm..", "..mmm...")      # differs from BLOCK
BLOCK_UP = rows("..mmm...", "..mmm...", "..mmm...", BLANK)   # floating one row above the floor
BLOCK_UP2 = rows("..mmm...", "..mmmm..", "..mmm...", BLANK)


def make_grid(frames: list[dict], layers=("body",), durations=None, tags=None) -> dict:
    durations = durations or [100] * len(frames)
    return pg.validate_grid({
        "format": "pixel-grid-v1", "canvas": [W, H], "pivot": PIVOT, "layers": list(layers),
        "palette": [{"key": "m", "name": "mid", "rgba": "#7a9e36ff"}, {"key": "K", "name": "outline", "rgba": "#1b1124ff"},
                    {"key": "e", "name": "eye", "rgba": "#ffd23aff", "accent": True}],
        "frames": [{"duration_ms": d, "cels": f} for d, f in zip(durations, frames)],
        "tags": tags if tags is not None else [{"name": "a", "from": 0, "to": 1, "direction": "forward"},
                                              {"name": "b", "from": 2, "to": 2, "direction": "forward"}]})


def make_cfg(**rules) -> AssetConfig:
    return AssetConfig({
        "format": "pixelart-asset-v1", "asset_id": "synthetic", "grid": "unused.grid.json",
        "frame_size": [W, H], "pivot": PIVOT, "contact_row": 7, "tick_hz": 60, "hold_ms": 100,
        "tags": {"a": {"playback": "loop", "contact": "grounded", "frames": [[0, 6], [1, 6]]},
                 "b": {"playback": "hold", "contact": "airborne", "frames": [[2, None]]}},
        "rules": rules})


def good_frames() -> list[dict]:
    return [{"body": BLOCK}, {"body": BLOCK_NOTCH}, {"body": BLOCK_UP}]


def good_grid() -> dict:
    return make_grid(good_frames(), durations=[100, 100, 100])


def result(report: dict, rule_id: str) -> dict:
    return next(r for r in report["rules"] if r["id"] == rule_id)


def run(cfg: AssetConfig, grid: dict | None = None, **kw) -> dict:
    return v.run(v.Context(cfg, grid or good_grid(), **kw))


class BaselineTests(unittest.TestCase):
    def test_a_clean_asset_passes_every_always_on_rule(self) -> None:
        report = run(make_cfg())
        self.assertTrue(report["passed"], v.summary(report))
        self.assertEqual(report["counts"]["fail"], 0)
        self.assertEqual({r["id"] for r in report["rules"] if r["status"] == "skipped"},
                         {"V04", "V05", "V06", "V09", "V10", "V11", "V12", "V13", "V14", "V16", "V17", "V18"})

    def test_report_is_machine_readable_and_summary_is_short(self) -> None:
        report = run(make_cfg(max_colors=3, max_isolated_per_frame=0))
        json.dumps(report)
        self.assertEqual(report["format"], "pixelart-report-v1")
        text = v.summary(report)
        self.assertIn("verdict: PASS", text)
        self.assertLess(len(text.splitlines()), 25)

    def test_zero_is_a_real_limit_not_a_disabled_rule(self) -> None:
        self.assertEqual(result(run(make_cfg(max_isolated_per_frame=0)), "V05")["status"], "pass")


class StructureTagsTicksTests(unittest.TestCase):
    def test_v01_wrong_pivot_canvas_and_frame_count(self) -> None:
        cfg = make_cfg()
        cfg.data["pivot"] = [2, 7]
        self.assertEqual(result(run(cfg), "V01")["status"], "fail")
        cfg = make_cfg()
        cfg.data["frame_size"] = [9, 8]
        self.assertEqual(result(run(cfg), "V01")["status"], "fail")
        grid = make_grid(good_frames()[:2], durations=[100, 100], tags=[{"name": "a", "from": 0, "to": 1}])
        self.assertTrue(any("tag table covers 3" in f for f in result(run(make_cfg(), grid), "V01")["failures"]))

    def test_v01_manifest_pivot_must_match(self) -> None:
        manifest = {"frame_size": [W, H], "pivot": [0, 0]}
        self.assertEqual(result(run(make_cfg(), manifest=manifest), "V01")["status"], "fail")

    def test_v02_tag_range_and_name_mismatch(self) -> None:
        grid = make_grid(good_frames(), tags=[{"name": "a", "from": 0, "to": 0}, {"name": "b", "from": 2, "to": 2}])
        self.assertEqual(result(run(make_cfg(), grid), "V02")["status"], "fail")
        grid = make_grid(good_frames(), tags=[{"name": "x", "from": 0, "to": 1}, {"name": "b", "from": 2, "to": 2}])
        self.assertEqual(result(run(make_cfg(), grid), "V02")["status"], "fail")

    def test_v03_durations_follow_the_tick_table(self) -> None:
        grid = make_grid(good_frames(), durations=[100, 100, 100])      # 6 ticks at 60 Hz = 100 ms
        self.assertEqual(result(run(make_cfg(), grid), "V03")["status"], "pass")
        bad = make_grid(good_frames(), durations=[100, 117, 100])
        self.assertTrue(any("frame 1" in f for f in result(run(make_cfg(), bad), "V03")["failures"]))
        hold = make_grid(good_frames(), durations=[100, 100, 250])      # hold frames use hold_ms
        self.assertEqual(result(run(make_cfg(), hold), "V03")["status"], "fail")

    def test_v03_manifest_tick_rows(self) -> None:
        rows_ok = [{"index": 0, "tag": "a", "ticks": 6, "duration_ms": 100}, {"index": 1, "tag": "a", "ticks": 6, "duration_ms": 100},
                   {"index": 2, "tag": "b", "ticks": None, "duration_ms": 100}]
        cfg = make_cfg(manifest_tick_rows="x.frames", manifest_tick_hz="x.tick_hz")
        manifest = {"x": {"frames": rows_ok, "tick_hz": 60}}
        self.assertEqual(result(run(cfg, manifest=manifest), "V03")["status"], "pass")
        broken = copy.deepcopy(manifest)
        broken["x"]["frames"][1]["ticks"] = 7
        self.assertEqual(result(run(cfg, manifest=broken), "V03")["status"], "fail")
        broken = copy.deepcopy(manifest)
        del broken["x"]["frames"][2]
        self.assertTrue(any("missing" in f for f in result(run(cfg, manifest=broken), "V03")["failures"]))
        self.assertEqual(result(run(cfg), "V03")["status"], "fail")     # rule set but no manifest


class PaletteAndLintTests(unittest.TestCase):
    def test_v04_colour_limit(self) -> None:
        self.assertEqual(result(run(make_cfg(max_colors=2)), "V04")["status"], "fail")   # 3 declared
        self.assertEqual(result(run(make_cfg(max_colors=3)), "V04")["status"], "pass")

    def test_v05_isolated_pixel_fails_at_limit_zero(self) -> None:
        frames = good_frames()
        frames[0] = {"body": rows("..mmm...", "..mmm...", "..mmm..m")}      # x=7 touches nothing? (6,7) empty -> isolated
        grid = make_grid(frames)
        res = result(run(make_cfg(max_isolated_per_frame=0), grid), "V05")
        self.assertEqual(res["status"], "fail")
        self.assertIn("frame 0", res["failures"][0])
        self.assertEqual(result(run(make_cfg(max_isolated_per_frame=1), grid), "V05")["status"], "pass")

    def test_v06_orphans_are_pixels_with_no_same_key_neighbour(self) -> None:
        frames = good_frames()
        frames[0] = {"body": rows("..mmmK..", "..mmm...", "..mmm...")}     # K touches m only: an orphan
        grid = make_grid(frames)
        self.assertEqual(result(run(make_cfg(max_orphans_per_frame=0), grid), "V06")["status"], "fail")
        self.assertEqual(result(run(make_cfg(max_orphans_per_frame=1), grid), "V06")["status"], "pass")
        accent = good_frames()
        accent[0] = {"body": rows("..mmme..", "..mmm...", "..mmm...")}     # an accent key is exempt
        self.assertEqual(result(run(make_cfg(max_orphans_per_frame=0), make_grid(accent)), "V06")["status"], "pass")


class ContactTests(unittest.TestCase):
    def test_v07_grounded_frame_must_end_on_the_contact_row(self) -> None:
        frames = good_frames()
        frames[1] = {"body": BLOCK_UP2}                                      # floating, but in grounded tag a
        res = result(run(make_cfg(), make_grid(frames)), "V07")
        self.assertEqual(res["status"], "fail")
        self.assertIn("frame 1 (a)", res["failures"][0])

    def test_v08_airborne_frame_must_leave_the_floor(self) -> None:
        self.assertEqual(result(run(make_cfg()), "V08")["status"], "pass")
        frames = good_frames()
        frames[2] = {"body": BLOCK}                                          # airborne tag b touching row 7
        res = result(run(make_cfg(), make_grid(frames)), "V08")
        self.assertEqual(res["status"], "fail")
        self.assertIn("contact row 7", res["failures"][0])

    def test_v07_v08_are_skipped_without_such_tags(self) -> None:
        cfg = make_cfg()
        for tag in cfg.data["tags"].values():
            tag["contact"] = "free"
        report = run(cfg)
        self.assertEqual((result(report, "V07")["status"], result(report, "V08")["status"]), ("skipped", "skipped"))


class FrozenKeyTests(unittest.TestCase):
    def fixture(self, grid: dict, frames=(0, 2)) -> dict:
        return v.freeze(make_cfg(), grid, list(frames))

    def test_freeze_then_validate_passes(self) -> None:
        grid = good_grid()
        cfg = make_cfg(frozen_keys={"frames": [0, 2], "layers": ["body"]})
        self.assertEqual(result(run(cfg, grid, fixture=self.fixture(grid)), "V09")["status"], "pass")

    def test_a_changed_pixel_breaks_the_freeze(self) -> None:
        grid = good_grid()
        fixture = self.fixture(grid)
        tampered = copy.deepcopy(good_frames())
        tampered[0] = {"body": rows("..mmm...", "..mmmK..", "..mmm...")}
        cfg = make_cfg(frozen_keys={"frames": [0, 2], "layers": ["body"]})
        res = result(run(cfg, make_grid(tampered), fixture=fixture), "V09")
        self.assertEqual(res["status"], "fail")
        self.assertIn("key frame 0", res["failures"][0])

    def test_fixture_may_carry_the_cels_themselves(self) -> None:
        cfg = make_cfg(frozen_keys={"frames": [0], "layers": ["body"]})
        fixture = {"keys": {"0": {"body": BLOCK}}}
        self.assertEqual(result(run(cfg, fixture=fixture), "V09")["status"], "pass")
        fixture = {"keys": {"0": {"body": BLOCK_NOTCH}}}
        self.assertEqual(result(run(cfg, fixture=fixture), "V09")["status"], "fail")

    def test_missing_fixture_or_key_fails(self) -> None:
        cfg = make_cfg(frozen_keys={"fixture": "x.json", "frames": [0, 2], "layers": ["body"]})
        self.assertEqual(result(run(cfg), "V09")["status"], "fail")
        grid = good_grid()
        fixture = self.fixture(grid, frames=(0,))
        self.assertEqual(result(run(cfg, grid, fixture=fixture), "V09")["status"], "fail")

    def test_digest_is_stable_and_layer_scoped(self) -> None:
        a = v.frame_digest({"body": BLOCK, "weapon": BLOCK_NOTCH}, ["body"])
        self.assertEqual(a, v.frame_digest({"body": BLOCK, "weapon": BLOCK}, ["body"]))
        self.assertNotEqual(a, v.frame_digest({"body": BLOCK_NOTCH}, ["body"]))


class LayerMarginUniqueEmptyTests(unittest.TestCase):
    def test_v10_overlapping_layers_fail(self) -> None:
        grid = make_grid([{"body": BLOCK, "extra": rows("..m.....")}, {"body": BLOCK_NOTCH}, {"body": BLOCK_UP}], layers=("body", "extra"))
        res = result(run(make_cfg(layers_disjoint=[["body", "extra"]]), grid), "V10")
        self.assertEqual(res["status"], "fail")
        disjoint = make_grid([{"body": BLOCK, "extra": rows("m.......")}, {"body": BLOCK_NOTCH}, {"body": BLOCK_UP}], layers=("body", "extra"))
        self.assertEqual(result(run(make_cfg(layers_disjoint=[["body", "extra"]]), disjoint), "V10")["status"], "pass")

    def test_v11_margin(self) -> None:
        self.assertEqual(result(run(make_cfg(canvas_margin=[2, 0, 2, 0])), "V11")["status"], "pass")
        res = result(run(make_cfg(canvas_margin=[3, 0, 0, 0])), "V11")     # content starts at x = 2
        self.assertEqual(res["status"], "fail")
        self.assertEqual(result(run(make_cfg(canvas_margin=[0, 0, 0, 1])), "V11")["status"], "fail")   # touches the bottom row

    def test_v12_duplicate_frames(self) -> None:
        frames = [{"body": BLOCK}, {"body": BLOCK}, {"body": BLOCK_UP}]
        res = result(run(make_cfg(unique_frames=True), make_grid(frames)), "V12")
        self.assertEqual(res["status"], "fail")
        self.assertIn("frame 1 is pixel-identical to frame 0", res["failures"][0])
        self.assertEqual(result(run(make_cfg(unique_frames=True)), "V12")["status"], "pass")

    def test_v15_empty_frame(self) -> None:
        frames = [{"body": BLOCK}, {"body": [BLANK] * H}, {"body": BLOCK_UP}]
        res = result(run(make_cfg(), make_grid(frames)), "V15")
        self.assertEqual(res["status"], "fail")
        self.assertIn("frame 1 is empty", res["failures"])


class FootDriftTests(unittest.TestCase):
    """V16: a planted foot must not slide more than the limit across any step of a loop tag, the seam included."""

    FOOT_A = rows("..mmm...")             # a foot at x 2..4 on the contact row
    FOOT_B = rows("....mmm.")             # the same foot 2 px further right
    FOOT_FAR = rows("mmm.....")           # 2 px to the left of FOOT_A

    def _drift(self, frames: list[list[str]], limit: int) -> dict:
        cels = [{"body": f} for f in frames]
        n = len(cels)
        grid = make_grid(cels, durations=[100] * n, tags=[{"name": "a", "from": 0, "to": n - 1}])
        cfg = make_cfg(foot_drift={"max_px": limit, "tags": ["a"]})
        cfg.data["tags"] = {"a": {"playback": "loop", "contact": "grounded", "frames": [[i, 6] for i in range(n)]}}
        return result(run(cfg, grid), "V16")

    def test_small_steps_and_a_matching_seam_pass(self) -> None:
        self.assertEqual(self._drift([self.FOOT_A, self.FOOT_B], 2)["status"], "pass")

    def test_a_jump_at_the_loop_seam_fails(self) -> None:
        # steps 0 -> 1 -> 2 are 2 px each, but 2 -> 0 (the seam) is 4 px
        res = self._drift([self.FOOT_FAR, self.FOOT_A, self.FOOT_B], 2)
        self.assertEqual(res["status"], "fail")
        self.assertTrue(any("loop seam" in f and "frame 2 -> 0" in f for f in res["failures"]), res["failures"])

    def test_a_planted_foot_that_jumps_inside_the_loop_fails(self) -> None:
        wide = rows("......mm")
        res = self._drift([self.FOOT_A, wide, self.FOOT_A], 2)
        self.assertEqual(res["status"], "fail")

    def test_lifted_and_landing_feet_are_not_counted(self) -> None:
        self.assertEqual(self._drift([self.FOOT_A, [BLANK] * H], 1)["status"], "pass")

    def test_a_lifted_foot_is_not_matched_to_the_other_planted_foot(self) -> None:
        # two feet, then one foot lifts off: the remaining foot stays put (0 px); the lifted one is not matched to it
        two, one = "mm....mm", "mm......"
        self.assertEqual(v.foot_drift(two, one), 0)
        self.assertEqual(v.foot_drift(one, two), 0)
        self.assertEqual(self._drift([rows(two), rows(one)], 0)["status"], "pass")

    def test_equal_foot_counts_are_paired_one_to_one_and_still_detect_sliding(self) -> None:
        # both feet slide 2 px between the frames: the pairing must still report 2 (not weakened)
        a, b = "m...m...", "..m...m."
        self.assertEqual(v.foot_drift(a, b), 2)
        self.assertEqual(self._drift([rows(a), rows(b)], 1)["status"], "fail")
        self.assertEqual(self._drift([rows(a), rows(b)], 2)["status"], "pass")

    def test_heel_lift_that_keeps_one_edge_planted_passes(self) -> None:
        self.assertEqual(v.foot_drift("..mmmm..", "....mm.."), 0)

    def test_not_configured_is_skipped_and_unknown_tag_is_an_error(self) -> None:
        self.assertEqual(result(run(make_cfg()), "V16")["status"], "skipped")
        with self.assertRaises(ToolkitError):
            run(make_cfg(foot_drift={"max_px": 3, "tags": ["nope"]}))


class BoxAndKeyCountTests(unittest.TestCase):
    def test_v13_limits_pass_and_fail(self) -> None:
        ok = [{"id": "r", "frames": [0], "top": 5, "bottom": 7, "min_width": 3, "max_width": 3, "min_height": 3}]
        self.assertEqual(result(run(make_cfg(bbox=ok)), "V13")["status"], "pass")
        for bad in ({"top": 4}, {"max_width": 2}, {"min_height": 4}, {"max_bottom": 6}, {"min_left": 3}):
            res = result(run(make_cfg(bbox=[{"id": "r", "frames": [0], **bad}])), "V13")
            self.assertEqual(res["status"], "fail", bad)

    def test_v13_selects_frames_by_tag_and_rejects_unknown_tags(self) -> None:
        res = result(run(make_cfg(bbox=[{"id": "t", "tags": ["a"], "max_height": 3}])), "V13")
        self.assertEqual(res["status"], "pass")
        with self.assertRaises(ToolkitError):
            run(make_cfg(bbox=[{"id": "t", "tags": ["nope"], "max_height": 3}]))

    def test_v14_key_counts(self) -> None:
        rules = [{"id": "mid", "frames": [0], "key": "m", "min": 9, "max": 9}]
        self.assertEqual(result(run(make_cfg(key_counts=rules)), "V14")["status"], "pass")
        self.assertEqual(result(run(make_cfg(key_counts=[{"id": "k", "frames": [0], "key": "K", "min": 1}])), "V14")["status"], "fail")
        self.assertEqual(result(run(make_cfg(key_counts=[{"id": "k", "frames": [0], "key": "m", "max": 8}])), "V14")["status"], "fail")
        layered = [{"id": "l", "frames": [0], "key": "m", "layer": "body", "min": 9}]
        self.assertEqual(result(run(make_cfg(key_counts=layered)), "V14")["status"], "pass")


class ConfigTests(unittest.TestCase):
    def test_config_rejects_bad_tag_tables_and_roles(self) -> None:
        base = make_cfg().data
        for mutate in (lambda d: d["tags"]["a"].update(frames=[[0, 6], [2, 6]]),           # not contiguous
                       lambda d: d["tags"]["b"].update(frames=[[1, 6]]),                    # frame in two tags
                       lambda d: d["tags"]["a"].update(contact="flying"),
                       lambda d: d.update(roles={"tell": "nope"}),
                       lambda d: d.update(roles={"jump": "a"}),
                       lambda d: d.update(format="other")):
            data = copy.deepcopy(base)
            mutate(data)
            with self.assertRaises(ToolkitError):
                AssetConfig(data)

    def test_default_config_for_a_bare_grid_runs_structural_rules(self) -> None:
        grid = good_grid()
        cfg = default_config(grid, "x.grid.json", "bare")
        report = v.run(v.Context(cfg, grid))
        self.assertTrue(report["passed"], v.summary(report))
        self.assertEqual(result(report, "V04")["status"], "pass")


class OverlayRuleTests(unittest.TestCase):
    """CG-L1..CG-L9 (V18, the CARL-GAUNTLET-01 overlay layering) on a 12 x 10 body: one passing and one failing case each."""

    OW, OH = 12, 10
    PARAMS = v.overlay_params({"body_grid": "synthetic"})

    @classmethod
    def blank(cls) -> list[list[str]]:
        return [["."] * cls.OW for _ in range(cls.OH)]

    @classmethod
    def cel(cls, points: dict) -> list[str]:
        rows = cls.blank()
        for (x, y), k in points.items():
            rows[y][x] = k
        return ["".join(r) for r in rows]

    def body(self, **layers) -> dict:
        """Body cels: a 2 x 2 hand of skin ``M`` at (5..6, 4..5) ringed by outline ``K`` on the near arm."""
        near = {(x, y): "K" for x in range(4, 8) for y in range(3, 7)}
        near.update({(5, 4): "M", (6, 4): "M", (5, 5): "M", (6, 5): "M"})
        cels = {"near_arm": self.cel(near)}
        for name, pts in layers.items():
            cels[name] = self.cel(pts)
        return cels

    def findings(self, body: dict, over: dict, exceptions=()) -> dict:
        return v.overlay_frame_findings(body, self.cel(over), (self.OW, self.OH), self.PARAMS, exceptions)

    GLOVE = {(5, 4): "U", (6, 4): "U", (5, 5): "T", (6, 5): "T"}

    def test_a_clean_glove_passes_every_rule(self) -> None:
        found = self.findings(self.body(), self.GLOVE)
        self.assertEqual({k: x for k, x in found.items() if x}, {})

    def test_cg_l1_a_pixel_beyond_the_near_arm_grown_by_one_fails(self) -> None:
        self.assertEqual(self.findings(self.body(), {**self.GLOVE, (8, 4): "K"})["CG-L1"], [])          # inside the 1 px growth
        far = self.findings(self.body(), {**self.GLOVE, (9, 4): "K"})
        self.assertEqual(len(far["CG-L1"]), 1)

    def test_cg_l2_the_watch_pin_and_prop_pixels_stay_uncovered(self) -> None:
        body = self.body(watch={(4, 3): "w"}, pin={(7, 6): "G"}, prop={(8, 5): "m"})
        self.assertEqual(self.findings(body, self.GLOVE)["CG-L2"], [])
        for covered in ((4, 3), (7, 6), (8, 5)):
            self.assertEqual(len(self.findings(body, {**self.GLOVE, covered: "K"})["CG-L2"]), 1, covered)

    def test_cg_l2_only_the_visible_prop_counts_the_near_arm_is_drawn_over_it(self) -> None:
        body = self.body(prop={(5, 4): "m", (8, 5): "m"})       # (5, 4) is under the hand's skin pixel
        self.assertEqual(self.findings(body, self.GLOVE)["CG-L2"], [])

    def test_cg_l3_head_and_hair_outside_the_near_arm_stay_uncovered(self) -> None:
        body = self.body(head={(8, 4): "M"}, hair={(8, 5): "H"})
        self.assertEqual(self.findings(body, self.GLOVE)["CG-L3"], [])
        self.assertEqual(len(self.findings(body, {**self.GLOVE, (8, 4): "K"})["CG-L3"]), 1)
        # a head pixel that lies under the near arm is the arm's own pixel (the jaw-line case): allowed
        under = self.body(head={(4, 3): "M"})
        self.assertEqual(self.findings(under, {**self.GLOVE, (4, 3): "K"})["CG-L3"], [])

    def test_cg_l4_at_most_twelve_pixels_outside_the_near_arm(self) -> None:
        ring = {(x, 2): "K" for x in range(3, 9)}                # 6 px above the arm
        self.assertEqual(self.findings(self.body(), {**self.GLOVE, **ring})["CG-L4"], [])
        more = {**ring, **{(x, 7): "K" for x in range(3, 9)}, (8, 3): "K"}       # 13 outside
        self.assertEqual(len(self.findings(self.body(), {**self.GLOVE, **more})["CG-L4"]), 1)

    def test_cg_l5_only_the_eight_gauntlet_keys_and_three_glints(self) -> None:
        self.assertEqual(self.findings(self.body(), {**self.GLOVE, (5, 4): "w"})["CG-L5"], [])
        for bad in ("M", "Y", "m"):
            self.assertEqual(len(self.findings(self.body(), {**self.GLOVE, (5, 4): bad})["CG-L5"]), 1, bad)
        glints = {(4, 3): "w", (5, 3): "w", (6, 3): "w", (7, 3): "w"}
        self.assertEqual(len(self.findings(self.body(), {**self.GLOVE, **glints})["CG-L5"]), 1)
        self.assertEqual(self.findings(self.body(), {**self.GLOVE, (4, 3): "w", (5, 3): "w", (6, 3): "w"})["CG-L5"], [])

    def test_cg_l6_every_skin_pixel_is_covered_except_the_listed_ones(self) -> None:
        partial = {(5, 4): "U", (6, 4): "U", (5, 5): "T"}
        self.assertEqual(len(self.findings(self.body(), partial)["CG-L6"]), 1)
        self.assertEqual(self.findings(self.body(), partial, [(6, 5)])["CG-L6"], [])
        # skin under the watch is not visible, so it needs no cover
        body = self.body(watch={(6, 5): "w"})
        self.assertEqual(self.findings(body, partial)["CG-L6"], [])

    def test_cg_l7_the_edge_columns_stay_empty(self) -> None:
        self.assertEqual(self.findings(self.body(), self.GLOVE)["CG-L7"], [])
        near = {(0, 4): "K", (1, 4): "M", (1, 5): "M"}
        body = {"near_arm": self.cel(near)}
        edge = v.overlay_frame_findings(body, self.cel({(0, 4): "K", (1, 4): "U", (1, 5): "U"}), (self.OW, self.OH),
                                        self.PARAMS)
        self.assertEqual(len(edge["CG-L7"]), 1)

    def body_grid(self, near: dict) -> dict:
        cel = self.cel(near)
        return pg.validate_grid({
            "format": "pixel-grid-v1", "canvas": [self.OW, self.OH], "pivot": [5, 9], "layers": ["near_arm", "gauntlet"],
            "palette": [{"key": k, "name": n, "rgba": c} for k, n, c in (
                ("K", "outline", "#1b1124ff"), ("M", "skin", "#eb9a6cff"), ("U", "steel", "#82689aff"),
                ("T", "shade", "#5a4068ff"), ("w", "glint", "#b8c0d8ff"))],
            "frames": [{"duration_ms": 100, "cels": {"near_arm": cel}}], "tags": []})

    def test_cg_l8_the_overlay_adds_no_orphan_or_isolated_pixel(self) -> None:
        near = {(x, y): "K" for x in range(3, 9) for y in range(3, 7)}
        near.update({(5, 4): "M", (6, 4): "M", (5, 5): "M", (6, 5): "M"})
        grid = self.body_grid(near)
        good, _ = v.overlay_l8(grid, self.cel(self.GLOVE), 0, "gauntlet")
        self.assertEqual(good, [])
        # a lone steel pixel in a field of outline has no same-key neighbour: a new orphan
        failures, _ = v.overlay_l8(grid, self.cel({(4, 3): "T", (5, 4): "U", (6, 4): "U", (5, 5): "U", (6, 5): "U"}), 0, "gauntlet")
        self.assertTrue(failures, "the lone T is an orphan")

    def test_cg_l8_orphans_the_body_already_has_are_reported_not_failed(self) -> None:
        near = {(x, y): "K" for x in range(3, 9) for y in range(3, 7)}
        near.update({(5, 4): "M", (6, 4): "M", (5, 5): "M", (6, 5): "M", (10, 8): "T", (10, 7): "K"})
        grid = self.body_grid(near)
        failures, inherited = v.overlay_l8(grid, self.cel(self.GLOVE), 0, "gauntlet")
        self.assertEqual(failures, [])
        self.assertIn((10, 8), inherited)

    def test_cg_l9_an_empty_overlay_is_allowed_only_without_near_arm_skin(self) -> None:
        with_skin = self.findings(self.body(), {})
        self.assertEqual(len(with_skin["CG-L9"]), 1)
        no_skin = {"near_arm": self.cel({(5, 4): "K", (6, 4): "K"})}
        self.assertEqual(v.overlay_frame_findings(no_skin, self.cel({}), (self.OW, self.OH), self.PARAMS)["CG-L9"], [])
        self.assertEqual(self.findings(self.body(), {}, [(5, 4), (6, 4), (5, 5), (6, 5)])["CG-L9"], [])

    def test_the_overlay_section_needs_a_body_grid_and_v18_is_skipped_without_it(self) -> None:
        with self.assertRaises(ToolkitError):
            v.overlay_params({})
        report = v.run(v.Context(make_cfg(), good_grid()))
        self.assertEqual(result(report, "V18")["status"], "skipped")

    def test_v15_leaves_empty_overlay_frames_to_cg_l9(self) -> None:
        grid = make_grid([{"body": BLOCK}, {"body": [BLANK] * H}, {"body": BLOCK_UP}])
        plain = v.Context(make_cfg(), grid)
        self.assertEqual(len(v.v15_nonempty(plain)), 1)
        cfg = make_cfg()
        cfg.data["overlay"] = {"body_grid": "synthetic"}
        self.assertEqual(v.v15_nonempty(v.Context(cfg, grid)), [])


if __name__ == "__main__":
    unittest.main()
