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

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray:
        """Choose an action from ``obs``. ``env`` is passed only so a native bot can
        read the same observation inside Rust; a Python policy must not read
        ``env.state_hash()``, ``env.diagnostics()`` or any other privileged state."""
        ...


class NoopPolicy:
    """Neutral stick, nothing held."""

    policy_id = NOOP_ID

    def reset(self, seed: int) -> None:
        pass

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray:
        return NOOP_ACTION.copy()


class NativePolicy:
    """A Rust bot driven through the binding. Seeds the bot like the native CLI.

    The Rust bot sees only the ``ObservationV1`` it builds from the env. The env
    is passed to the binding for that and for nothing else; ``act`` checks that
    the ``obs`` it was given is that same observation.
    """

    def __init__(self, policy_id: str) -> None:
        self._bot = _native.NativeBot(policy_id)
        self.policy_id = self._bot.policy_id

    def reset(self, seed: int) -> None:
        self._bot.reset(seed)

    def act(self, obs: dict[str, np.ndarray], env: AutoplayEnv) -> np.ndarray:
        current = env.native.observe()
        if any(not np.array_equal(obs[key], current[key]) for key in current):
            raise ValueError("obs is not the env's latest observation")
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
