"""A pure-Python SYNTHETIC ``playtrace-session/1`` writer for the exporter's tests.

Everything it writes is made up: seeded random observations and actions, fake
hashes. It carries the quality flag ``synthetic_fixture``, the build commit
``synthetic-python-fixture`` and mode ``synthetic_fixture``. It is **never** a
human demonstration, and it cannot pass the real ``playtrace verify`` (the hashes
are not a replay): tests pair it with a stub verifier. The Rust binary's
``synth-fixture`` command writes sessions that really verify (see the e2e tests).
"""

from __future__ import annotations

import json
import struct
from pathlib import Path
from typing import Any

import numpy as np

from playtrace_tools import schema
from playtrace_tools.journal import fnv1a64


def encode_obs_bits(bits: list[int]) -> str:
    """The recorder's ``f32bits-rle-hex/1`` encoding (mirrors the Rust encoder)."""
    tokens: list[str] = []
    zeros = 0
    for value in bits:
        if value == 0:
            zeros += 1
            continue
        if zeros:
            tokens.append(f"z{zeros}")
            zeros = 0
        tokens.append(f"{value:x}")
    if zeros:
        tokens.append(f"z{zeros}")
    return ",".join(tokens)


def f32_bits(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


def make_obs(hero: int, rng: np.random.Generator, enemies: int = 2) -> str:
    obs = np.zeros(schema.OBS_LEN, dtype=np.float32)
    obs[0:14] = rng.uniform(-1, 1, 14).astype(np.float32)
    obs[3:12] = rng.integers(0, 2, 9).astype(np.float32)
    obs[schema.SELF_IS_CARL] = 1.0 if hero == 0 else 0.0
    obs[schema.SELF_IS_DONUT] = 0.0 if hero == 0 else 1.0
    start = schema.SELF_LEN + schema.ALLY_LEN
    obs[start] = 1.0  # ally visible
    enemy_start = start + 1
    for i in range(enemies):
        obs[enemy_start + i * schema.ENEMY_FEATURES : enemy_start + (i + 1) * schema.ENEMY_FEATURES] = (
            rng.uniform(-1, 1, schema.ENEMY_FEATURES).astype(np.float32)
        )
    mask_start = enemy_start + schema.MAX_ENEMIES * schema.ENEMY_FEATURES
    obs[mask_start : mask_start + enemies] = 1.0
    return encode_obs_bits([int(b) for b in obs.view(np.uint32)])


def random_action(rng: np.random.Generator) -> list[int]:
    move = rng.choice([-1.0, 0.0, 0.0, 1.0, 0.4])
    buttons = 0
    for bit in (1, 2, 4, 64):
        if rng.random() < 0.2:
            buttons |= bit
    return [f32_bits(float(move)), buttons]


def _hex(value: int) -> str:
    return f"{value & 0xFFFFFFFFFFFFFFFF:016x}"


def write_session(
    base: Path,
    session_id: str,
    ticks: int = 200,
    seed: int = 1,
    src: tuple[str, str] = ("human_p1", "companion_ai"),
    run_kind: str = "human",
    participants: str = "owner_only",
    status: str = "completed",
    restart_at: int | None = 120,
    defeat_at: int | None = None,
    end_open: bool = False,
    partial_observation: bool = False,
    synthetic: bool = True,
    variant: str = "fixture_a",
) -> Path:
    """Write a valid synthetic session directory ``base/session_id``."""
    rng = np.random.default_rng(seed)
    directory = base / session_id
    directory.mkdir(parents=True)
    records: list[dict[str, Any]] = []
    control: list[dict[str, Any]] = [
        {"seq": 0, "at_t": 0, "ev": {"k": "episode_begin", "episode": 0, "reason": "start", "stage": 0}}
    ]
    episode = 0
    et = 0
    pre = 0x1234
    current_obs = [make_obs(0, rng), make_obs(1, rng)]
    for t in range(ticks):
        boundary = None
        if restart_at is not None and t == restart_at:
            boundary = "restart"
        if defeat_at is not None and t == defeat_at:
            boundary = "party_defeat"
        if boundary == "restart":
            # A manual restart is a control event before this tick; the episode
            # ends with the observation of the state after the previous tick.
            control.append({"seq": len(control), "at_t": t, "ev": {"k": "restart"}})
            control.append(
                {
                    "seq": len(control),
                    "at_t": t,
                    "ev": {"k": "episode_end", "episode": episode, "outcome": "restart", "final_obs": list(current_obs)},
                }
            )
            episode += 1
            et = 0
            control.append(
                {"seq": len(control), "at_t": t, "ev": {"k": "episode_begin", "episode": episode, "reason": "restart", "stage": 0}}
            )
            current_obs = [make_obs(0, rng), make_obs(1, rng)]
        if boundary == "party_defeat":
            # Reset inside the step: final_obs is already the next episode's first observation.
            episode_end_at = t
            next_first = [make_obs(0, rng), make_obs(1, rng)]
            control.append(
                {
                    "seq": len(control),
                    "at_t": episode_end_at,
                    "ev": {"k": "episode_end", "episode": episode, "outcome": "party_defeat", "final_obs": next_first},
                }
            )
            episode += 1
            et = 0
            control.append(
                {
                    "seq": len(control),
                    "at_t": episode_end_at,
                    "ev": {"k": "episode_begin", "episode": episode, "reason": "party_defeat_restart", "stage": 0},
                }
            )
            current_obs = next_first
        et += 1
        hero_actions = []
        eff = []
        for hero in range(2):
            action = random_action(rng)
            if src[hero].startswith("human"):
                hero_actions.append(action)
                eff.append(action)
            else:
                hero_actions.append([0, 0])
                eff.append(action)
        post = (pre * 6364136223846793005 + 1442695040888963407 + t) & 0xFFFFFFFFFFFFFFFF
        record: dict[str, Any] = {
            "t": t,
            "ep": episode,
            "et": et,
            "stage": 0,
            "pre": _hex(pre),
            "slots": hero_actions,
            "hero": hero_actions,
            "src": list(src),
            "ev": [{"i": t, "k": "Fake", "d": f"Fake {{ t: {t} }}"}] if t % 40 == 0 else [],
            "post": _hex(post),
            "eh": _hex(t),
            "ph": _hex(0),
            "obs": list(current_obs),
            "eff": eff,
        }
        if partial_observation:
            del record["obs"]
            del record["eff"]
        records.append(record)
        pre = post
        current_obs = [make_obs(0, rng), make_obs(1, rng)]  # the next tick's o_t
    if not end_open:
        control.append(
            {
                "seq": len(control),
                "at_t": ticks,
                "ev": {"k": "episode_end", "episode": episode, "outcome": "exit", "final_obs": list(current_obs)},
            }
        )

    checksums = []
    for index in range(0, max(1, (len(records) + 599) // 600)):
        chunk = records[index * 600 : (index + 1) * 600]
        lines = [json.dumps(r, separators=(",", ":")) for r in chunk]
        body = "".join(line + "\n" for line in lines).encode()
        digest = fnv1a64(body)
        trailer = {
            "end": True,
            "chunk": index,
            "ticks": len(chunk),
            "first_t": chunk[0]["t"] if chunk else None,
            "last_t": chunk[-1]["t"] if chunk else None,
            "fnv": _hex(digest),
        }
        (directory / f"ticks-{index:06d}.jsonl").write_bytes(
            body + json.dumps(trailer, separators=(",", ":")).encode() + b"\n"
        )
        checksums.append({"chunk": index, "ticks": len(chunk), "fnv": _hex(digest)})
    control_lines = [json.dumps(c, separators=(",", ":")) for c in control]
    control_blob = "".join(line + "\n" for line in control_lines).encode()
    (directory / "control.jsonl").write_bytes(control_blob)
    manifest = {
        "schema": "playtrace-session/1",
        "session_id": session_id,
        "run_kind": run_kind,
        "participants": participants,
        "started_on_utc_day": "2026-01-01",
        "build_commit": "synthetic-python-fixture",
        "build_dirty": False,
        "source_fingerprint": "0000000000000000",
        "game_ruleset_id": "synthetic",
        "feature_flags": {"synthetic": "true"} if synthetic else {},
        "tick_rate_hz": 60,
        "reset": {
            "source": "party_fixture",
            "name": variant,
            "demo": False,
            "demo_beat": 0,
            "cf02_rules": False,
            "cf02_seed": None,
            "view_height": 360,
            "donut_biscuit": False,
            "kit_hash": "0000000000000000",
            "start_hash": "0000000000000000",
        },
        "action_schema_id": "rawinput-bits/2",
        "observation_schema_id": "none" if partial_observation else schema.OBS_SCHEMA_ID,
        "mode": "synthetic_fixture" if synthetic else "solo_human",
        "status": status,
        "capture_complete": status in ("completed", "abandoned"),
        "replay_verified": False,
        "training_eligible": False,
        "observation_is_partial": partial_observation,
        "quality_flags": ["synthetic_fixture"] if synthetic else [],
        "tick_count": len(records),
        "last_tick": len(records) - 1,
        "control_integrity": {"lines": len(control_lines), "fnv": _hex(fnv1a64(control_blob))},
        "chunk_checksums": checksums,
    }
    (directory / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return directory
