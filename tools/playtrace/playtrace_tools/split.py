"""Session-level train/dev/holdout partition and the leakage check.

The partition unit is the player pseudonym when a manifest supplies one,
otherwise the whole session (PT-Q09). Ticks, episodes and retries of one group
never appear in two splits. The assignment is a deterministic ranking by a
hash of ``(seed, group)``, so the same sessions and seed give the same split.
"""

from __future__ import annotations

import hashlib
from collections import defaultdict
from typing import Any, Iterable

SPLITS = ("train", "dev", "holdout")


class LeakageError(Exception):
    """A dataset breaks the split or uniqueness rules."""


def group_key(manifest: dict[str, Any]) -> str:
    pseudonym = manifest.get("participant_pseudonym")
    return f"player:{pseudonym}" if pseudonym else f"session:{manifest['session_id']}"


def assign_splits(
    groups: Iterable[str], seed: int = 0, dev_frac: float = 0.2, holdout_frac: float = 0.2
) -> dict[str, str]:
    """Deterministically map each group to ``train``, ``dev`` or ``holdout``.

    At least one group stays in ``train``. With fewer than two groups there is
    no leak-free validation split: everything is ``train``.
    """
    unique = sorted(set(groups))
    ranked = sorted(unique, key=lambda g: hashlib.sha256(f"{seed}:{g}".encode()).hexdigest())
    n = len(ranked)
    n_hold = int(n * holdout_frac)
    n_dev = int(n * dev_frac)
    if n >= 2 and n_dev == 0:
        n_dev = 1
    while n_dev + n_hold >= n and (n_hold or n_dev):
        if n_hold:
            n_hold -= 1
        else:
            n_dev -= 1
    out: dict[str, str] = {}
    for position, group in enumerate(ranked):
        if position < n_hold:
            out[group] = "holdout"
        elif position < n_hold + n_dev:
            out[group] = "dev"
        else:
            out[group] = "train"
    return out


def check_leakage(rows: list[dict[str, Any]], groups: dict[str, str]) -> dict[str, Any]:
    """Raise ``LeakageError`` on any leak; otherwise return the report.

    Checks: every row's split equals its group's split; no group, session,
    episode or trajectory appears in two splits; ``(session, t, hero)`` is unique
    (no duplicate tick).
    """
    seen_rows: set[tuple[str, int, str]] = set()
    where: dict[str, dict[Any, set[str]]] = {
        "group": defaultdict(set),
        "session": defaultdict(set),
        "episode": defaultdict(set),
        "trajectory": defaultdict(set),
    }
    per_split: dict[str, int] = defaultdict(int)
    for row in rows:
        key = (row["session_id"], row["t"], row["hero"])
        if key in seen_rows:
            raise LeakageError(f"duplicate row for session/tick/hero {key}")
        seen_rows.add(key)
        group = row["group"]
        if groups.get(group) != row["split"]:
            raise LeakageError(f"row {key} split {row['split']!r} != group split")
        where["group"][group].add(row["split"])
        where["session"][row["session_id"]].add(row["split"])
        where["episode"][(row["session_id"], row["ep"])].add(row["split"])
        where["trajectory"][row["traj_id"]].add(row["split"])
        per_split[row["split"]] += 1
    for unit, table in where.items():
        for name, splits in table.items():
            if len(splits) > 1:
                raise LeakageError(f"{unit} {name!r} appears in several splits: {sorted(splits)}")
    return {
        "rows_by_split": dict(per_split),
        "groups_by_split": {s: sorted(g for g, v in groups.items() if v == s) for s in SPLITS},
        "duplicate_rows": 0,
        "units_in_several_splits": 0,
    }
