"""Tests for the capture-directory guard of the CP-B2 evidence scripts (CP-B2-FU-EVID-RM).

`scripts/evidence_captures_guard.sh` stands in front of the one `rm -rf` of
scripts/make_cp_b2_evidence.sh, and `check_captures` in scripts/make_cp_b2_evidence.py
validates the same directory before anything is read or written. Each test builds a
throwaway "repository" in a temporary directory; nothing here touches the real tree.
"""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GUARD = ROOT / "scripts" / "evidence_captures_guard.sh"
sys.path.insert(0, str(ROOT / "scripts"))
import make_cp_b2_evidence as evidence  # noqa: E402


def run_guard(repo: Path, target: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["bash", "-c", 'source "$1"; prepare_captures_dir "$2"', "guard", str(GUARD), target],
        cwd=repo, capture_output=True, text=True)


class ShellGuard(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name) / "repo"
        (self.repo / "artifacts").mkdir(parents=True)
        self.outside = Path(self._tmp.name) / "outside"
        self.outside.mkdir()
        (self.outside / "keep.txt").write_text("keep")
        (self.repo / "keep.txt").write_text("keep")
        (self.repo / "artifacts" / "other.txt").write_text("keep")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def assert_refused(self, target: str) -> None:
        result = run_guard(self.repo, target)
        self.assertEqual(result.returncode, 2, f"{target!r}: {result.stderr}")
        self.assertIn("refusing", result.stderr)
        # Nothing was removed.
        self.assertTrue((self.outside / "keep.txt").exists())
        self.assertTrue((self.repo / "keep.txt").exists())
        self.assertTrue((self.repo / "artifacts" / "other.txt").exists())

    def test_refuses_the_dangerous_values(self) -> None:
        for target in ["", ".", "..", "/", "artifacts", "artifacts/", "artifacts/.",
                       "artifacts/..", "artifacts/../keep.txt", "../outside",
                       str(self.outside), str(self.repo), str(self.repo.parent)]:
            with self.subTest(target=target):
                self.assert_refused(target)

    def test_refuses_a_symlink_even_one_that_points_below_artifacts(self) -> None:
        (self.repo / "artifacts" / "real").mkdir()
        (self.repo / "artifacts" / "link").symlink_to(self.repo / "artifacts" / "real")
        self.assert_refused("artifacts/link")
        (self.repo / "artifacts" / "out").symlink_to(self.outside)
        self.assert_refused("artifacts/out")

    def test_refuses_an_existing_directory_without_the_marker(self) -> None:
        stranger = self.repo / "artifacts" / "somebody-elses"
        stranger.mkdir()
        (stranger / "precious.txt").write_text("keep")
        result = run_guard(self.repo, "artifacts/somebody-elses")
        self.assertEqual(result.returncode, 2)
        self.assertIn("not a dedicated capture directory", result.stderr)
        self.assertTrue((stranger / "precious.txt").exists())

    def test_refuses_an_existing_file(self) -> None:
        self.assert_refused("artifacts/other.txt")

    def test_creates_a_fresh_marked_directory_and_empties_a_marked_one(self) -> None:
        result = run_guard(self.repo, "artifacts/caps")
        self.assertEqual(result.returncode, 0, result.stderr)
        caps = self.repo / "artifacts" / "caps"
        self.assertTrue((caps / ".capture-dir").is_file())
        (caps / "old").mkdir()
        (caps / "old" / "frame_0000.png").write_text("stale")
        again = run_guard(self.repo, "artifacts/caps")
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual(sorted(p.name for p in caps.iterdir()), [".capture-dir"])
        # A nested path below artifacts works too and leaves its neighbours alone.
        nested = run_guard(self.repo, "artifacts/a/b/caps")
        self.assertEqual(nested.returncode, 0, nested.stderr)
        self.assertTrue((self.repo / "artifacts" / "other.txt").exists())


class PythonGuard(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name).resolve()
        (self.repo / "artifacts").mkdir()
        self._root = evidence.ROOT
        evidence.ROOT = self.repo

    def tearDown(self) -> None:
        evidence.ROOT = self._root
        self._tmp.cleanup()

    def capture(self, name: str, marker: bool = True) -> Path:
        caps = self.repo / "artifacts" / name
        caps.mkdir()
        if marker:
            (caps / evidence.MARKER).write_text("")
        return caps

    def add_folder(self, caps: Path, name: str, frames: bool = True) -> None:
        folder = caps / name
        folder.mkdir()
        if frames:
            (folder / "frame_0000.png").write_bytes(b"x")

    def test_refuses_outside_artifacts_and_unmarked_directories(self) -> None:
        for bad in [self.repo, self.repo / "artifacts", self.repo.parent, Path("/")]:
            with self.subTest(bad=bad), self.assertRaises(SystemExit):
                evidence.check_captures(bad)
        unmarked = self.capture("unmarked", marker=False)
        self.add_folder(unmarked, "00_carl_prog_dyn_arc_standing_4")
        with self.assertRaises(SystemExit):
            evidence.check_captures(unmarked)
        with self.assertRaises(SystemExit):
            evidence.check_captures(self.repo / "artifacts" / "missing")

    def test_refuses_unknown_empty_and_frameless_capture_folders(self) -> None:
        caps = self.capture("caps")
        with self.assertRaises(SystemExit):  # no folder at all
            evidence.check_captures(caps)
        self.add_folder(caps, "00_carl_prog_dyn_arc_standing_4")
        self.assertEqual(len(evidence.check_captures(caps)), 1)
        self.add_folder(caps, "01_not_a_known_sequence_7")
        with self.assertRaises(SystemExit):
            evidence.check_captures(caps)
        other = self.capture("other")
        self.add_folder(other, "00_carl_prog_dyn_arc_standing_4", frames=False)
        with self.assertRaises(SystemExit):
            evidence.check_captures(other)

    def test_evidence_directory_is_untouched_by_a_refusal(self) -> None:
        out = self.repo / "docs" / "evidence" / "carl-prog-01" / "cp-b2-c"
        evidence.OUT = out
        with self.assertRaises(SystemExit):
            evidence.main(self.repo / "artifacts")
        self.assertFalse(out.exists())


if __name__ == "__main__":
    unittest.main()
