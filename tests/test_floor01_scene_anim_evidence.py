"""Tests for the ANIM-INT-01 evidence builder (log parsing, marker detection, checks).

Synthetic traces only: no renders and no exported atlases are needed.
"""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import make_anim_int_evidence as ev  # noqa: E402


def trace_line(tick, grounded, vy, carl="idle:0", donut="idle:0", foot=92.0, vx=0.0):
    return (f"FLOOR1_ANIM_TRACE tick={tick} hero_x={100 + tick:.3f} hero_foot_y={foot:.3f} "
            f"vx={vx:.3f} vy={vy:.3f} grounded={int(grounded)} donut_x=50.000 "
            f"donut_foot_y=92.000 carl={carl} donut={donut}")


def jump_trace():
    """Stand 3 ticks, jump (takeoff tick 4), apex at tick 8, touchdown at tick 12."""
    lines = []
    for tick in range(1, 4):
        lines.append(trace_line(tick, True, 0.0))
    for tick, vy in zip(range(4, 8), (400, 300, 200, 100)):
        lines.append(trace_line(tick, False, vy, carl="jump_start:10" if tick == 4 else "jump_rise:12",
                                foot=100 + tick))
    for tick, vy in zip(range(8, 12), (-10, -110, -210, -310)):
        lines.append(trace_line(tick, False, vy, carl="jump_fall:13", foot=120 - tick))
    for tick in range(12, 16):
        lines.append(trace_line(tick, True, 0.0, carl="land:14" if tick == 12 else "idle:0"))
    return "\n".join(lines)


class ParseTests(unittest.TestCase):
    def test_trace_rows_round_trip(self):
        rows = ev.parse_trace(jump_trace() + "\nnot a trace line\n")
        self.assertEqual(len(rows), 15)
        self.assertEqual((rows[3].tick, rows[3].carl_tag, rows[3].grounded), (4, "jump_start", False))

    def test_empty_trace_is_rejected(self):
        with self.assertRaises(ev.EvidenceError):
            ev.parse_trace("nothing here")

    def test_capture_log_needs_state_lines_for_the_same_tick(self):
        good = ("FLOOR1_CAPTURE index=0 tick=4 file=frames/frame_0000_tick_00004.png\n"
                "FLOOR1_SCENE_STATE tick=4 hero=104.000,118.000 cam=0,20 hash=00\n"
                "FLOOR1_ANIM_STATE tick=4 carl=jump_start:10 donut=idle:0\n")
        captures = ev.parse_captures(good)
        self.assertEqual((captures[0].tick, captures[0].carl, captures[0].file),
                         (4, "jump_start:10", "frame_0000_tick_00004.png"))
        with self.assertRaisesRegex(ev.EvidenceError, "no state lines"):
            ev.parse_captures(good.split("FLOOR1_SCENE_STATE")[0])
        with self.assertRaisesRegex(ev.EvidenceError, "another tick"):
            ev.parse_captures(good.replace("FLOOR1_ANIM_STATE tick=4", "FLOOR1_ANIM_STATE tick=5"))
        with self.assertRaisesRegex(ev.EvidenceError, "no FLOOR1_CAPTURE"):
            ev.parse_captures("FLOOR1_SCENE_STATE tick=1")


class MarkerTests(unittest.TestCase):
    def test_takeoff_apex_touchdown(self):
        rows = ev.parse_trace(jump_trace())
        self.assertEqual(ev.takeoffs(rows), [4])
        self.assertEqual(ev.jump_markers(rows, 1), {"takeoff": 4, "apex": 8, "touchdown": 12})

    def test_a_walk_off_is_not_a_takeoff(self):
        lines = [trace_line(1, True, 0.0), trace_line(2, False, -17.5, carl="jump_fall:13"),
                 trace_line(3, True, 0.0, carl="land:14")]
        self.assertEqual(ev.takeoffs(ev.parse_trace("\n".join(lines))), [])

    def test_donut_events_follow_her_own_tags(self):
        lines = []
        tags = ["run:4", "jump_start:10", "jump_rise:12", "jump_fall:13", "land:14", "run:4"]
        for tick, tag in enumerate(tags, start=1):
            lines.append(trace_line(tick, True, 0.0, donut=tag))
        rows = ev.parse_trace("\n".join(lines))
        self.assertEqual(ev.donut_events(rows, 1, 6),
                         [("jump input", 2), ("apex", 4), ("touchdown", 5)])

    def test_nearest_capture_prefers_the_earlier_tick_on_a_tie(self):
        captures = [ev.Capture(i, t, f"f{i}.png", 0.0, 0, "idle:0", "idle:0")
                    for i, t in enumerate((142, 148))]
        self.assertEqual(ev.nearest_capture(captures, 145).tick, 142)
        self.assertEqual(ev.nearest_capture(captures, 146).tick, 148)
        self.assertEqual(ev.nearest_capture(captures, 100).tick, 142)


class CrossCheckTests(unittest.TestCase):
    def test_render_and_headless_must_agree(self):
        rows = ev.parse_trace(jump_trace())
        agree = ev.Capture(0, 4, "f.png", 104.0, 0, "jump_start:10", "idle:0")
        ev.check_log_against_trace([agree], rows)
        wrong_frame = ev.Capture(0, 4, "f.png", 104.0, 0, "jump_rise:12", "idle:0")
        with self.assertRaisesRegex(ev.EvidenceError, "headless trace says"):
            ev.check_log_against_trace([wrong_frame], rows)
        wrong_x = ev.Capture(0, 4, "f.png", 105.0, 0, "jump_start:10", "idle:0")
        with self.assertRaisesRegex(ev.EvidenceError, "hero x differs"):
            ev.check_log_against_trace([wrong_x], rows)
        unknown = ev.Capture(0, 99, "f.png", 0.0, 0, "idle:0", "idle:0")
        with self.assertRaisesRegex(ev.EvidenceError, "not in the trace"):
            ev.check_log_against_trace([unknown], rows)


class OutputTests(unittest.TestCase):
    def test_timeline_png_and_checksums(self):
        rows = ev.parse_trace(jump_trace())
        markers = [ev.jump_markers(rows, 1)]
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            ev.draw_timeline(rows, 1, 15, markers, [("apex", 9)], out / "timeline.png")
            self.assertGreater((out / "timeline.png").stat().st_size, 1000)
            (out / "README.md").write_text("ignored", encoding="utf-8")
            ev.write_sums(out)
            sums = (out / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(sums), 1)
            self.assertTrue(sums[0].endswith("  timeline.png"))
            self.assertRegex(sums[0], r"^[0-9a-f]{64}  ")


if __name__ == "__main__":
    unittest.main()
