"""Gate tests for scripts/compare_floor01_lighting.py (needs numpy and Pillow, no atlases).

The Bevy-vs-Pillow comparison must fail when the frames diverge, not only report.
"""
import argparse
import sys
import unittest
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import compare_floor01_lighting as cmp  # noqa: E402


def gate_args(**over):
    base = dict(max_mean_de=cmp.GATE_MEAN_DE, max_p95_de=cmp.GATE_P95_DE,
                max_abs_dl=cmp.GATE_ABS_DL, max_region_mean_de=cmp.GATE_REGION_DE)
    base.update(over)
    return argparse.Namespace(**base)


def frame(value):
    rng = np.random.default_rng(1)
    base = rng.integers(20, 200, size=(24, 32, 3), dtype=np.int16)
    return np.clip(base + value, 0, 255).astype(np.uint8)


MASKS = {"wall": np.ones((24, 32), dtype=bool)}


class CompareGates(unittest.TestCase):
    def failures(self, bevy, ref, **over):
        result = cmp.compare(bevy, ref, MASKS)
        result.pop("_de")
        return cmp.check_gates(result, gate_args(**over))

    def test_identical_frames_pass(self):
        ref = frame(0)
        self.assertEqual(self.failures(ref.copy(), ref), [])

    def test_rounding_sized_difference_passes(self):
        ref = frame(0)
        self.assertEqual(self.failures(np.clip(ref.astype(int) + 1, 0, 255).astype(np.uint8), ref), [])

    def test_a_missing_pass_fails_every_gate(self):
        ref = frame(0)
        dark = (ref * 0.4).astype(np.uint8)  # e.g. ambient lost
        failures = self.failures(dark, ref)
        text = " ".join(failures)
        self.assertIn("mean dE76", text)
        self.assertIn("p95", text)
        self.assertIn("delta L*", text)

    def test_an_empty_region_mask_fails(self):
        ref = frame(0)
        result = cmp.compare(ref, ref, {"wall": np.zeros((24, 32), dtype=bool)})
        result.pop("_de")
        failures = cmp.check_gates(result, gate_args())
        self.assertTrue(any("no pixels" in f for f in failures))

    def test_gates_are_adjustable(self):
        ref = frame(0)
        shifted = np.clip(ref.astype(int) + 12, 0, 255).astype(np.uint8)
        self.assertTrue(self.failures(shifted, ref))
        self.assertEqual(
            self.failures(shifted, ref, max_mean_de=50, max_p95_de=50, max_abs_dl=50,
                          max_region_mean_de=50), [])


if __name__ == "__main__":
    unittest.main()
