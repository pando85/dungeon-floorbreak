"""AUTOPLAY-01 Python tooling over the native Rust environment.

Tooling only: the shipped game never imports or builds any of this. The
simulation is the game's own Rust code; Python converts types and runs
baselines and the optional CPU PPO smoke (see ``tools/autoplay/README.md``).
"""

from autoplay.env import (
    ACTION_NVEC,
    ACTION_VERSION,
    OBSERVATION_VERSION,
    AutoplayEnv,
    EpisodeOverError,
    InvalidActionError,
    scenario_roster,
)

__all__ = [
    "ACTION_NVEC",
    "ACTION_VERSION",
    "OBSERVATION_VERSION",
    "AutoplayEnv",
    "EpisodeOverError",
    "InvalidActionError",
    "scenario_roster",
]
