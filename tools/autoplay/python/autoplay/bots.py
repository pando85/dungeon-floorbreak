"""Baseline policies. They read the observation and never ``info`` (AP-Q08).

The scripted Rust bots (``reactive_survivor`` and friends) belong to stage B
and are not duplicated here. Stage C ships a seeded random-legal baseline so
the evaluation and PPO comparison have something fair to run against.
"""

from __future__ import annotations

from typing import Any, Protocol

import numpy as np

from autoplay.env import ACTION_NVEC

NOOP_ACTION = np.array([1, 0, 0, 0, 0, 0], dtype=np.int64)
#: Components a policy may legally use on this build: the special and the solo
#: swap are reserved and unavailable (``available = [0, 0]``).
ACTIVE_COMPONENTS = 4


class Policy(Protocol):
    policy_id: str

    def reset(self, seed: int) -> None: ...

    def act(self, obs: dict[str, np.ndarray]) -> np.ndarray: ...


class NoopPolicy:
    """Neutral stick, nothing held."""

    policy_id = "noop"

    def reset(self, seed: int) -> None:
        pass

    def act(self, obs: dict[str, np.ndarray]) -> np.ndarray:
        return NOOP_ACTION.copy()


class RandomLegalPolicy:
    """Uniform over the actions Carl can actually use, redrawn every tick.

    Ignores the observation. The reserved special and swap components stay 0,
    so the baseline never counts as an attempted unavailable action.
    """

    policy_id = "random_legal"

    def __init__(self) -> None:
        self._rng = np.random.default_rng(0)

    def reset(self, seed: int) -> None:
        self._rng = np.random.default_rng(seed)

    def act(self, obs: dict[str, np.ndarray]) -> np.ndarray:
        action = np.zeros(len(ACTION_NVEC), dtype=np.int64)
        for index in range(ACTIVE_COMPONENTS):
            action[index] = self._rng.integers(0, ACTION_NVEC[index])
        return action


POLICIES: dict[str, Any] = {
    NoopPolicy.policy_id: NoopPolicy,
    RandomLegalPolicy.policy_id: RandomLegalPolicy,
}


def make_policy(policy_id: str) -> Policy:
    try:
        return POLICIES[policy_id]()
    except KeyError:
        raise ValueError(
            f"unknown policy {policy_id!r} (available in Python: {', '.join(sorted(POLICIES))}; "
            "the scripted Rust bots arrive with stage B)"
        ) from None
