"""Load an exported dataset and check it against the source sessions.

``load_dataset`` refuses an unknown dataset schema or observation schema (no
silent migration) and re-hashes every file against ``dataset.json``.
``check_against_sources`` re-reads the sessions and proves, row by row, that each
row maps to its original tick, hashes, action bits and observations.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from . import schema
from .export import DATASET_SCHEMA, _sha256
from .journal import read_session
from .split import SPLITS, LeakageError, check_leakage


class DatasetError(Exception):
    """The dataset cannot be loaded or fails a check."""


@dataclass
class Stream:
    rows: list[dict[str, Any]]
    obs: np.ndarray
    action: np.ndarray
    mask: np.ndarray
    split: np.ndarray
    next_obs: np.ndarray | None = None
    next_valid: np.ndarray | None = None
    terminal: np.ndarray | None = None
    context: np.ndarray | None = None


@dataclass
class Dataset:
    path: Path
    meta: dict[str, Any]
    streams: dict[str, Stream]


def load_dataset(path: Path) -> Dataset:
    try:
        meta = json.loads((path / "dataset.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise DatasetError(f"dataset.json unreadable: {error}") from error
    if meta.get("schema") != DATASET_SCHEMA:
        raise DatasetError(
            f"unsupported dataset schema {meta.get('schema')!r} (this loader reads {DATASET_SCHEMA!r}; "
            "re-export from the sessions instead of migrating silently)"
        )
    if meta.get("observation", {}).get("schema") != schema.OBS_SCHEMA_ID or meta["observation"].get(
        "length"
    ) != schema.OBS_LEN:
        raise DatasetError("unsupported observation schema: re-export or retrain, never pad or drop")
    for name, expected in meta["files"].items():
        file = path / name
        if not file.is_file() or _sha256(file) != expected:
            raise DatasetError(f"{name} is missing or does not match dataset.json")
    core = {k: v for k, v in meta.items() if k not in ("dataset_id", "path")}
    if hashlib.sha256(json.dumps(core, sort_keys=True).encode()).hexdigest()[:16] != meta["dataset_id"]:
        raise DatasetError("dataset_id does not match the dataset content")

    streams: dict[str, Stream] = {}
    for name in meta["streams"]:
        directory = path / name
        rows = [
            json.loads(line)
            for line in (directory / "rows.jsonl").read_text(encoding="utf-8").splitlines()
        ]
        stream = Stream(
            rows=rows,
            obs=np.load(directory / "obs.npy"),
            action=np.load(directory / "action.npy"),
            mask=np.load(directory / "mask.npy"),
            split=np.load(directory / "split.npy"),
        )
        for optional, attribute in (
            ("next_obs.npy", "next_obs"),
            ("next_valid.npy", "next_valid"),
            ("terminal.npy", "terminal"),
            ("context.npy", "context"),
        ):
            if (directory / optional).is_file():
                setattr(stream, attribute, np.load(directory / optional))
        if not (len(rows) == len(stream.obs) == len(stream.action) == len(stream.mask) == len(stream.split)):
            raise DatasetError(f"{name}: array lengths do not match the rows")
        streams[name] = stream
    return Dataset(path, meta, streams)


def check_against_sources(dataset: Dataset, base: Path) -> dict[str, Any]:
    """Raise ``DatasetError`` unless every row maps to its source tick.

    Checks, per row: the source tick exists exactly once and carries the same
    ``pre``/``post``/``eh`` hashes, chunk and line; the action bits equal the
    recorded routed (human) or effective (AI) input; the stored observation is
    the tick's own ``o_t`` (the observation that precedes the action); and
    ``next_obs`` is the next tick's ``o_t`` or the episode-end final observation.
    Also re-checks split leakage and ``(session, tick, hero)`` uniqueness.
    """
    sessions: dict[str, Any] = {}
    checked = 0
    all_rows: list[dict[str, Any]] = []
    for name, stream in dataset.streams.items():
        for index, row in enumerate(stream.rows):
            sid = row["session_id"]
            if sid not in sessions:
                path = base / sid
                if not path.is_dir():
                    raise DatasetError(f"source session {sid} is not under the base directory")
                sessions[sid] = read_session(path)
            session = sessions[sid]
            hero = schema.HEROES.index(row["hero"])
            where = f"{name} row {index} ({sid[:8]} t={row['t']} {row['hero']})"
            if not 0 <= row["t"] < len(session.ticks):
                raise DatasetError(f"{where}: no such source tick")
            tick = session.ticks[row["t"]]
            raw = tick.raw
            if (tick.chunk, tick.line) != (row["chunk"], row["line"]):
                raise DatasetError(f"{where}: source chunk/line differ")
            for key in ("pre", "post"):
                if raw[key] != row[key]:
                    raise DatasetError(f"{where}: {key} hash differs from the source")
            expected_source = raw["src"][hero]
            if expected_source != row["controlled_by"]:
                raise DatasetError(f"{where}: control source relabelled")
            if (expected_source.startswith("human")) != (name == "human"):
                raise DatasetError(f"{where}: human/AI stream mix-up")
            bits = raw["hero"][hero] if name == "human" else raw["eff"][hero]
            if bits != row["raw_action"] or schema.policy_action(bits) != row["action"]:
                raise DatasetError(f"{where}: action differs from the source")
            if list(stream.action[index]) != row["action"]:
                raise DatasetError(f"{where}: action tensor differs from the row")
            own = schema.decode_obs(raw["obs"][hero])
            if not np.array_equal(own, stream.obs[index]):
                raise DatasetError(f"{where}: observation is not the tick's o_t")
            if schema.action_mask(own) != row["mask"]:
                raise DatasetError(f"{where}: mask is not derived from the observation")
            if stream.next_obs is not None:
                kind = row["next_kind"]
                if kind == "step":
                    expected = schema.decode_obs(session.ticks[row["t"] + 1].raw["obs"][hero])
                elif kind == "final_obs":
                    end = session.episode_ends[row["ep"]]
                    expected = schema.decode_obs(end["final_obs"][hero])
                else:
                    expected = None
                if expected is None:
                    if stream.next_valid[index]:
                        raise DatasetError(f"{where}: next_obs marked valid without a source")
                elif not np.array_equal(expected, stream.next_obs[index]):
                    raise DatasetError(f"{where}: next_obs is not the following o_t")
            checked += 1
            all_rows.append(row)
    groups = dataset.meta["splits"]["groups"]
    try:
        leakage = check_leakage(all_rows, groups)
    except LeakageError as error:
        raise DatasetError(str(error)) from error
    return {"rows_checked": checked, "sessions": len(sessions), "leakage": leakage}


def split_rows(stream: Stream, split: str) -> np.ndarray:
    """Indices of ``stream`` rows in ``split``."""
    return np.flatnonzero(stream.split == SPLITS.index(split))
