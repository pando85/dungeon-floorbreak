"""Seeded evaluation of a policy over scenarios, with JSON and CSV reports.

Policies read the observation only (AP-Q08). Episode ``i`` uses reset seed
``base_seed + i`` and policy seed ``base_seed + i`` too, so a run is
reproducible from ``(scenario, policy, episodes, base_seed)``.

The simulation ignores the reset seed (there is no gameplay RNG on this build),
so a deterministic policy plays the same trajectory every episode. Every row
therefore carries ``trajectory_sha256`` (a hash of the episode's action trace)
and every summary reports ``distinct_trajectories`` and ``independent_episodes``
like the native evaluator: ``episodes`` counts copies, ``independent_episodes``
counts what a success interval may honestly treat as separate samples.
"""

from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path
from typing import Any

import numpy as np

from autoplay.bots import Policy
from autoplay.env import AutoplayEnv

EPISODE_FIELDS = [
    "scenario",
    "episode",
    "seed",
    "outcome",
    "ticks",
    "steps",
    "return",
    "carl_hurt_events",
    "enemies_defeated",
    "trajectory_sha256",
]

SUMMARY_NOTES = [
    "The reset seed does not change the simulation; it seeds policy-side randomness only.",
    "A deterministic policy repeats the same trajectory every episode: read "
    "independent_episodes (= distinct_trajectories), not episodes, before treating "
    "success_rate as a sample estimate.",
    "Outcomes describe scripted baselines and short PPO runs, not human skill, difficulty or fun.",
]


REPLAY_MODES = ("none", "first", "all")


def run_episode(
    env: AutoplayEnv,
    policy: Policy,
    seed: int,
    *,
    record_as: tuple[str, str] | None = None,
) -> dict[str, Any]:
    """Play one episode. ``record_as = (episode_label, commit)`` also returns the
    native ``autoplay-episode/1`` JSONL text under the key ``replay``."""
    policy.reset(seed)
    obs, _ = env.reset(seed=seed)
    if record_as is not None:
        env.start_recording(policy.policy_id, *record_as)
    total = 0.0
    steps = 0
    hurt = 0
    defeated = 0
    info: dict[str, Any] = {}
    trace = hashlib.sha256()
    terminated = truncated = False
    while not (terminated or truncated):
        action = policy.act(obs, env)
        trace.update(np.asarray(action, dtype=np.int64).tobytes())
        obs, reward, terminated, truncated, info = env.step(action)
        total += reward
        steps += 1
        hurt += info["reward_breakdown"]["hurt_events"]
        defeated += info["reward_breakdown"]["defeats"]
    row = {
        "scenario": env.scenario_id,
        "seed": seed,
        "outcome": info["outcome"],
        "ticks": info["tick"],
        "steps": steps,
        "return": total,
        "carl_hurt_events": hurt,
        "enemies_defeated": defeated,
        "trajectory_sha256": trace.hexdigest(),
    }
    if record_as is not None:
        row["replay"] = env.finish_recording()
    return row


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    outcomes: dict[str, int] = {}
    for row in rows:
        outcomes[row["outcome"]] = outcomes.get(row["outcome"], 0) + 1
    returns = np.array([r["return"] for r in rows], dtype=np.float64)
    ticks = np.array([r["ticks"] for r in rows], dtype=np.float64)
    distinct = len({(r["scenario"], r["trajectory_sha256"]) for r in rows})
    return {
        "episodes": len(rows),
        "distinct_trajectories": distinct,
        "independent_episodes": distinct,
        "outcomes": dict(sorted(outcomes.items())),
        "success_rate": outcomes.get("success", 0) / len(rows),
        "return_mean": float(returns.mean()),
        "return_std": float(returns.std()),
        "ticks_mean": float(ticks.mean()),
        "carl_hurt_events_mean": float(np.mean([r["carl_hurt_events"] for r in rows])),
        "enemies_defeated_mean": float(np.mean([r["enemies_defeated"] for r in rows])),
    }


def evaluate(
    scenarios: list[str],
    policy: Policy,
    episodes: int,
    base_seed: int,
    *,
    env_kwargs: dict[str, Any] | None = None,
    replays: str = "none",
    replay_dir: Path | None = None,
    commit: str = "unknown",
) -> dict[str, Any]:
    """Run ``episodes`` per scenario; returns per-episode rows and summaries.

    ``replays`` is ``none``, ``first`` (episode 1 of each scenario) or ``all``.
    Recorded episodes are written by the native replay writer to
    ``replay_dir/<scenario>/<policy>/episode_NNNN.jsonl`` (the layout of the
    native ``autoplay evaluate``) and listed in ``replay_files``.
    """
    if replays not in REPLAY_MODES:
        raise ValueError(f"replays must be one of {REPLAY_MODES}, got {replays!r}")
    if replays != "none" and replay_dir is None:
        raise ValueError("replay_dir is required when replays are recorded")
    rows: list[dict[str, Any]] = []
    per_scenario: dict[str, Any] = {}
    replay_files: list[str] = []
    for scenario in scenarios:
        env = AutoplayEnv(scenario, **(env_kwargs or {}))
        scenario_rows = []
        for episode in range(episodes):
            number = episode + 1
            wanted = replays == "all" or (replays == "first" and number == 1)
            row = run_episode(
                env,
                policy,
                base_seed + episode,
                record_as=(f"{number:04d}", commit) if wanted else None,
            )
            row["episode"] = number
            text = row.pop("replay", None)
            if text is not None:
                target = Path(replay_dir) / scenario / policy.policy_id / f"episode_{number:04d}.jsonl"
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(text)
                replay_files.append(target.relative_to(replay_dir).as_posix())
            scenario_rows.append(row)
        env.close()
        per_scenario[scenario] = summarize(scenario_rows)
        rows.extend(scenario_rows)
    return {
        "policy_id": policy.policy_id,
        "base_seed": base_seed,
        "replays": replays,
        "replay_files": replay_files,
        "episodes_per_scenario": episodes,
        "notes": SUMMARY_NOTES,
        "scenarios": per_scenario,
        "overall": summarize(rows),
        "rows": rows,
    }


def write_report(result: dict[str, Any], out: Path, extra: dict[str, Any] | None = None) -> None:
    out.mkdir(parents=True, exist_ok=True)
    with (out / "episodes.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=EPISODE_FIELDS)
        writer.writeheader()
        for row in result["rows"]:
            writer.writerow({field: row[field] for field in EPISODE_FIELDS})
    summary = {k: v for k, v in result.items() if k != "rows"}
    summary.update(extra or {})
    (out / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
