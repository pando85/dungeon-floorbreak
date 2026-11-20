"""Seeded evaluation of a policy over scenarios, with JSON and CSV reports.

Policies read the observation only (AP-Q08). Episode ``i`` uses reset seed
``base_seed + i`` and policy seed ``base_seed + i`` too, so a run is
reproducible from ``(scenario, policy, episodes, base_seed)``.
"""

from __future__ import annotations

import csv
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
]


def run_episode(env: AutoplayEnv, policy: Policy, seed: int) -> dict[str, Any]:
    policy.reset(seed)
    obs, _ = env.reset(seed=seed)
    total = 0.0
    steps = 0
    hurt = 0
    defeated = 0
    info: dict[str, Any] = {}
    terminated = truncated = False
    while not (terminated or truncated):
        action = policy.act(obs)
        obs, reward, terminated, truncated, info = env.step(action)
        total += reward
        steps += 1
        hurt += info["reward_breakdown"]["hurt_events"]
        defeated += info["reward_breakdown"]["defeats"]
    return {
        "scenario": env.scenario_id,
        "seed": seed,
        "outcome": info["outcome"],
        "ticks": info["tick"],
        "steps": steps,
        "return": total,
        "carl_hurt_events": hurt,
        "enemies_defeated": defeated,
    }


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    outcomes: dict[str, int] = {}
    for row in rows:
        outcomes[row["outcome"]] = outcomes.get(row["outcome"], 0) + 1
    returns = np.array([r["return"] for r in rows], dtype=np.float64)
    ticks = np.array([r["ticks"] for r in rows], dtype=np.float64)
    return {
        "episodes": len(rows),
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
) -> dict[str, Any]:
    """Run ``episodes`` per scenario; returns per-episode rows and summaries."""
    rows: list[dict[str, Any]] = []
    per_scenario: dict[str, Any] = {}
    for scenario in scenarios:
        env = AutoplayEnv(scenario, **(env_kwargs or {}))
        scenario_rows = []
        for episode in range(episodes):
            row = run_episode(env, policy, base_seed + episode)
            row["episode"] = episode + 1
            scenario_rows.append(row)
        env.close()
        per_scenario[scenario] = summarize(scenario_rows)
        rows.extend(scenario_rows)
    return {
        "policy_id": policy.policy_id,
        "base_seed": base_seed,
        "episodes_per_scenario": episodes,
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
