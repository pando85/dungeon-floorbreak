"""Policies. They read the observation only (AP-Q08).

The scripted bots (``random_legal``, ``approach_and_attack``,
``reactive_survivor``) live in Rust (stage B, ``src/autoplay/bots.rs``).
Python only forwards the env's observation to the native bot through the
binding and never re-implements their rules. ``noop`` is a constant action,
kept as a trivial test baseline.
"""

from __future__ import annotations

from typing import Any, Protocol

import numpy as np

from autoplay import _native
from autoplay.env import AutoplayEnv

NOOP_ACTION = np.array([1, 0, 0, 0, 0, 0], dtype=np.int64)
NOOP_ID = "noop"


class Policy(Protocol):
    policy_id: str

    def reset(self, seed: int) -> None: ...

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray: ...


class NoopPolicy:
    """Neutral stick, nothing held."""

    policy_id = NOOP_ID

    def reset(self, seed: int) -> None:
        pass

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray:
        return NOOP_ACTION.copy()


class NativePolicy:
    """A Rust bot driven through the binding. Seeds the bot like the native CLI."""

    def __init__(self, policy_id: str) -> None:
        self._bot = _native.NativeBot(policy_id)
        self.policy_id = self._bot.policy_id

    def reset(self, seed: int) -> None:
        self._bot.reset(seed)

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray:
        return np.asarray(self._bot.act(env.native), dtype=np.int64)


def native_policy_ids() -> list[str]:
    return list(_native.bot_ids())


def make_policy(policy_id: str) -> Any:
    if policy_id == NOOP_ID:
        return NoopPolicy()
    if policy_id in native_policy_ids():
        return NativePolicy(policy_id)
    raise ValueError(
        f"unknown policy {policy_id!r} (available: "
        f"{', '.join(sorted([NOOP_ID, *native_policy_ids()]))})"
    )
