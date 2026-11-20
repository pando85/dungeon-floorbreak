"""Bounded CPU PPO smoke (AP-Q20: runs locally, never in CI).

Trains Stable-Baselines3 PPO with ``MultiInputPolicy`` on the Gymnasium env,
saves a checkpoint with full metadata, reloads it through the schema check and
evaluates against the random-legal baseline on the development and hold-out
roster. It demonstrates that the pipeline works. It makes no claim that the
policy is strong.
"""

from __future__ import annotations

import json
import random
import time
from pathlib import Path
from typing import Any

import numpy as np

from autoplay.bots import NativePolicy
from autoplay.checkpoint import MODEL_FILE, verify_checkpoint, write_metadata
from autoplay.env import AutoplayEnv, scenario_roster
from autoplay.evaluate import evaluate, write_report
from autoplay.provenance import environment_record

DEFAULT_PPO = {
    "n_steps": 512,
    "batch_size": 64,
    "n_epochs": 4,
    "learning_rate": 3e-4,
    "gamma": 0.99,
    "gae_lambda": 0.95,
    "clip_range": 0.2,
    "ent_coef": 0.0,
}


def _require_sb3() -> Any:
    try:
        import stable_baselines3  # noqa: F401
        import torch  # noqa: F401
    except ImportError as exc:  # pragma: no cover - message path
        raise SystemExit(
            "PPO needs the optional 'train' extra (stable-baselines3 and CPU torch); "
            "see tools/autoplay/README.md"
        ) from exc
    import stable_baselines3

    return stable_baselines3


def seed_everything(seed: int) -> None:
    import torch
    from stable_baselines3.common.utils import set_random_seed

    random.seed(seed)
    np.random.seed(seed)
    set_random_seed(seed, using_cuda=False)
    torch.manual_seed(seed)
    # One thread: reproducible and, for this tiny net, faster than many.
    torch.set_num_threads(1)


class PpoPolicy:
    """Deterministic PPO policy over the observation only."""

    policy_id = "ppo"

    def __init__(self, model: Any) -> None:
        self._model = model

    def reset(self, seed: int) -> None:
        pass

    def act(self, obs: dict[str, np.ndarray], env: Any = None) -> np.ndarray:
        action, _ = self._model.predict(obs, deterministic=True)
        return np.asarray(action, dtype=np.int64)


def load_policy(
    checkpoint: Path, reward_id: str = "combat-v1"
) -> tuple[PpoPolicy, dict[str, Any]]:
    """Reload a checkpoint, refusing a schema or reward mismatch first.

    ``checkpoint`` is the checkpoint directory or its ``model.zip``;
    ``reward_id`` is the reward profile of the environment it will run in.
    """
    sb3 = _require_sb3()
    if checkpoint.is_file():
        checkpoint = checkpoint.parent
    record = verify_checkpoint(checkpoint, reward_id=reward_id)
    model = sb3.PPO.load(checkpoint / MODEL_FILE, device="cpu")
    return PpoPolicy(model), record


def train(
    scenarios: list[str],
    steps: int,
    seed: int,
    out: Path,
    *,
    eval_episodes: int = 20,
    ppo: dict[str, Any] | None = None,
    reward_id: str = "combat-v1",
) -> dict[str, Any]:
    sb3 = _require_sb3()
    from stable_baselines3.common.vec_env import DummyVecEnv

    seed_everything(seed)
    hyper = {**DEFAULT_PPO, **(ppo or {})}
    vec = DummyVecEnv([(lambda s=s: AutoplayEnv(s, reward_id=reward_id)) for s in scenarios])
    vec.seed(seed)
    model = sb3.PPO("MultiInputPolicy", vec, seed=seed, device="cpu", verbose=0, **hyper)
    started = time.perf_counter()
    model.learn(total_timesteps=steps)
    train_seconds = time.perf_counter() - started
    out.mkdir(parents=True, exist_ok=True)
    model.save(out / MODEL_FILE)
    vec.close()

    roster = scenario_roster()
    development = [sid for sid, split in roster if split == "development"]
    holdout = [sid for sid, split in roster if split == "holdout"]
    record = {
        "algorithm": "PPO",
        "policy": "MultiInputPolicy",
        "device": "cpu",
        "seed": seed,
        "timesteps_requested": steps,
        "timesteps_trained": int(model.num_timesteps),
        "train_wall_seconds": round(train_seconds, 3),
        "hyperparameters": hyper,
        "train_scenarios": scenarios,
        "development_scenarios": development,
        "holdout_scenarios": holdout,
        "environment": environment_record(reward_id),
    }
    write_metadata(out, record)

    # Reload through the schema check, then evaluate the reloaded model.
    policy, reloaded = load_policy(out, reward_id=reward_id)
    eval_seed = seed + 10_000
    ppo_result = evaluate(
        development + holdout,
        policy,
        eval_episodes,
        eval_seed,
        replays="first",
        replay_dir=out / "eval_ppo" / "replays",
        commit=record["environment"]["commit"],
    )
    baseline = evaluate(development + holdout, NativePolicy("random_legal"), eval_episodes, eval_seed)
    write_report(ppo_result, out / "eval_ppo", {"checkpoint_sha256": reloaded["model_sha256"]})
    write_report(baseline, out / "eval_random_legal")
    smoke = {
        "steps": record["timesteps_trained"],
        "train_wall_seconds": record["train_wall_seconds"],
        "eval_episodes_per_scenario": eval_episodes,
        "ppo": {sid: s for sid, s in ppo_result["scenarios"].items()},
        "random_legal": {sid: s for sid, s in baseline["scenarios"].items()},
        "note": (
            "A pipeline demonstration. Short PPO runs are not expected to beat the "
            "baseline or to approximate human skill; do not read strength into these numbers. "
            "The reset seed does not change the simulation: a deterministic policy (the PPO "
            "evaluation) repeats one trajectory, so compare independent_episodes, not episodes, "
            "and do not read the PPO and random_legal success counts as like-for-like samples."
        ),
    }
    (out / "smoke.json").write_text(json.dumps(smoke, indent=2, sort_keys=True) + "\n")
    return smoke
