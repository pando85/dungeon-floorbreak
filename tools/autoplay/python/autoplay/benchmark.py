"""Single-env throughput: a pure-Rust loop, the binding, and the Gymnasium env.

All three step the same scenario with the same cycled random-legal actions and
read the observation after every step, so the numbers differ only by the layer
under test. Reported honestly; no tuning.
"""

from __future__ import annotations

import time
from typing import Any

import numpy as np

from autoplay import _native
from autoplay.env import AutoplayEnv


def _actions(count: int = 4096) -> list[list[int]]:
    """Cycled actions drawn from the native ``random_legal`` bot (never stepped)."""
    scratch = _native.NativeEnv("cp01_single_goblin")
    scratch.reset(0)
    bot = _native.NativeBot("random_legal")
    bot.reset(0)
    return [bot.act(scratch) for _ in range(count)]


def run_benchmark(scenario: str, steps: int) -> dict[str, Any]:
    actions = _actions()
    native_steps, native_seconds = _native.bench_native(scenario, actions, steps)

    raw = _native.NativeEnv(scenario)
    raw.reset(0)
    started = time.perf_counter()
    for i in range(steps):
        _, _, terminated, truncated, _ = raw.step(actions[i % len(actions)])
        if terminated or truncated:
            raw.reset(0)
    binding_seconds = time.perf_counter() - started

    env = AutoplayEnv(scenario)
    env.reset(seed=0)
    arrays = [np.asarray(a, dtype=np.int64) for a in actions]
    started = time.perf_counter()
    for i in range(steps):
        _, _, terminated, truncated, _ = env.step(arrays[i % len(arrays)])
        if terminated or truncated:
            env.reset(seed=0)
    gym_seconds = time.perf_counter() - started

    return {
        "scenario": scenario,
        "steps": steps,
        "native_rust_steps_per_s": round(native_steps / native_seconds),
        "binding_steps_per_s": round(steps / binding_seconds),
        "gymnasium_steps_per_s": round(steps / gym_seconds),
        "note": (
            "single env, one tick per step, observation read after every step; "
            "episodes restart at the end"
        ),
    }
