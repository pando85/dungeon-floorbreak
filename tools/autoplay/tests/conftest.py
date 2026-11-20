"""Shared helpers. Tests import the installed ``autoplay`` package only."""

from __future__ import annotations

import numpy as np
import pytest

from autoplay import AutoplayEnv

DEV = "cp01_single_goblin"


def pattern_actions() -> list[list[int]]:
    """Fixed parity sequence (shared with the native crate): walk right for 90
    ticks, then hold the primary button for 90 ticks."""
    return [[2, 0, 0, 0, 0, 0]] * 90 + [[1, 0, 1, 0, 0, 0]] * 90


def chaser_action(obs: dict[str, np.ndarray]) -> list[int]:
    """Tiny observation-only test script: close in on the nearest visible enemy
    and hold primary. Not a shipped bot."""
    if obs["enemy_mask"][0] > 0:
        dx = obs["enemies"][0][0]
        horizontal = 2 if dx > 0.02 else 0 if dx < -0.02 else 1
        return [horizontal, 0, 1, 0, 0, 0]
    return [2, 0, 0, 0, 0, 0]


@pytest.fixture
def env():
    e = AutoplayEnv(DEV)
    yield e
    e.close()
