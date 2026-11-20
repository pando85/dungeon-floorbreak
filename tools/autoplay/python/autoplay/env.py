"""Gymnasium adapter over the native ``AutoplayEnv`` (AUTOPLAY-01C, AP-Q08).

The adapter owns no gameplay rules. It converts types, builds the exact
spaces from the native ObservationV1 schema and keeps ``info`` free of
privileged data: raw events, hidden enemy state and state hashes stay behind
the explicit ``state_hash()`` / ``diagnostics()`` accessors.
"""

from __future__ import annotations

from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium import spaces

from autoplay import _native
from autoplay._native import EpisodeOverError, InvalidActionError

#: ActionV1: horizontal (left/neutral/right), jump, primary, physical,
#: special (reserved), solo swap (reserved). Matches ``ActionV1`` in Rust.
ACTION_NVEC = (3, 2, 2, 2, 2, 2)
ACTION_VERSION = 1
OBSERVATION_VERSION = 1

#: Keys of ``info`` (reset and step). Anything else is a leak (AP-Q08).
RESET_INFO_KEYS = frozenset(
    {
        "scenario_id",
        "tick",
        "requested_seed",
        "sim_seed",
        "seed_affects_sim",
        "cf02_rules",
        "action_version",
        "observation_version",
        "reward_id",
        "max_ticks",
        "action_repeat",
        "unavailable",
    }
)
STEP_INFO_KEYS = RESET_INFO_KEYS | {
    "ticks_executed",
    "outcome",
    "terminal_reason",
    "simultaneous_clear",
    "reward_breakdown",
}


def scenario_roster() -> list[tuple[str, str]]:
    """The v1 roster as ``(scenario_id, "development" | "holdout")``."""
    return [tuple(entry) for entry in _native.scenario_roster()]


def observation_schema() -> dict[str, Any]:
    """The native ObservationV1 schema (names, order, bounds)."""
    return _native.observation_schema()


def build_observation_space(schema: dict[str, Any] | None = None) -> spaces.Dict:
    """The exact ObservationV1 ``spaces.Dict``, built from the native schema."""
    schema = schema or observation_schema()
    if schema["version"] != OBSERVATION_VERSION:
        raise RuntimeError(
            f"native observation schema v{schema['version']} != adapter v{OBSERVATION_VERSION}"
        )

    def box(bounds: list[tuple[float, float]], rows: int | None = None) -> spaces.Box:
        low = np.array([b[0] for b in bounds], dtype=np.float32)
        high = np.array([b[1] for b in bounds], dtype=np.float32)
        if rows is not None:
            low = np.tile(low, (rows, 1))
            high = np.tile(high, (rows, 1))
        return spaces.Box(low=low, high=high, dtype=np.float32)

    def unit(n: int) -> spaces.Box:
        return spaces.Box(low=0.0, high=1.0, shape=(n,), dtype=np.float32)

    return spaces.Dict(
        {
            "self": box(schema["self_bounds"]),
            "enemies": box(schema["enemy_bounds"], schema["max_enemies"]),
            "enemy_mask": unit(schema["max_enemies"]),
            "hazards": box(schema["hazard_bounds"], schema["max_hazards"]),
            "hazard_mask": unit(schema["max_hazards"]),
            "available": unit(schema["available_len"]),
        }
    )


class AutoplayEnv(gym.Env):
    """One Carl-only fight on the real Floor 01 fixture simulation.

    ``reset(seed)`` seeds policy-side randomness only: the simulation has no
    gameplay RNG on this build and ``info['seed_affects_sim']`` is ``False``.
    ``step`` after ``terminated`` or ``truncated`` raises
    :class:`EpisodeOverError`; nothing resets silently.
    """

    metadata = {"render_modes": []}

    def __init__(
        self,
        scenario_id: str = "cp01_single_goblin",
        *,
        max_ticks: int = 1800,
        action_repeat: int = 1,
        reward_id: str = "combat-v1",
        assets_root: str | None = None,
    ) -> None:
        super().__init__()
        self.scenario_id = scenario_id
        self._native = _native.NativeEnv(
            scenario_id,
            assets_root=assets_root,
            max_ticks=max_ticks,
            action_repeat=action_repeat,
            reward_id=reward_id,
        )
        schema = observation_schema()
        if schema["action_len"] != len(ACTION_NVEC) or schema["action_version"] != ACTION_VERSION:
            raise RuntimeError("native ActionV1 does not match the adapter")
        self.action_space = spaces.MultiDiscrete(np.array(ACTION_NVEC, dtype=np.int64))
        self.observation_space = build_observation_space(schema)

    @property
    def native(self) -> _native.NativeEnv:
        """The underlying native env (policies and replay recording only)."""
        return self._native

    # -- Gymnasium API -----------------------------------------------------

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[dict[str, np.ndarray], dict[str, Any]]:
        super().reset(seed=seed)
        if seed is None:
            # The adapter's own RNG supplies the seed (AP-Q08); it is recorded
            # as `requested_seed` in info.
            seed = int(self.np_random.integers(0, 2**63 - 1))
        elif not 0 <= int(seed) < 2**64:
            raise ValueError("seed must fit an unsigned 64-bit integer")
        return self._native.reset(int(seed))

    def step(
        self, action: Any
    ) -> tuple[dict[str, np.ndarray], float, bool, bool, dict[str, Any]]:
        values = np.asarray(action)
        if values.shape != (len(ACTION_NVEC),) or not np.issubdtype(values.dtype, np.integer):
            raise InvalidActionError(
                f"action must be {len(ACTION_NVEC)} integers, got shape {values.shape} "
                f"dtype {values.dtype}"
            )
        return self._native.step([int(v) for v in values])

    def render(self) -> None:
        return None

    def close(self) -> None:
        return None

    # -- Native episode replay (stage B format, written by the native code) --

    def start_recording(self, policy_id: str, episode_label: str, commit: str) -> None:
        """Record the current episode as ``autoplay-episode/1``; call right after reset."""
        self._native.start_recording(policy_id, episode_label, commit)

    def finish_recording(self) -> str:
        """JSONL text of the finished recorded episode."""
        return self._native.finish_recording()

    # -- Privileged accessors: tests, parity and QA only -------------------

    def state_hash(self) -> int:
        """Authoritative state hash. Never a policy input, never in ``info``."""
        return self._native.state_hash()

    def diagnostics(self) -> dict[str, Any]:
        """Privileged QA record. Never a policy input."""
        return self._native.diagnostics()
