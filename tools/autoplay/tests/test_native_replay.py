"""Python-written episodes are native ``autoplay-episode/1`` replays.

The replay is written by the native recorder through the binding (no format
code in Python) and must pass ``autoplay replay --verify`` of the Rust CLI in a
fresh process. Set ``AUTOPLAY_CLI`` to the built ``autoplay`` binary; CI sets
``AUTOPLAY_REQUIRE_CLI=1`` so a missing binary fails instead of skipping.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

from autoplay import AutoplayEnv
from autoplay.bots import make_policy
from autoplay.evaluate import evaluate, run_episode
from autoplay.provenance import assets_root

CLI = os.environ.get("AUTOPLAY_CLI")
if not CLI and os.environ.get("AUTOPLAY_REQUIRE_CLI") == "1":
    raise RuntimeError("AUTOPLAY_REQUIRE_CLI=1 but AUTOPLAY_CLI is not set")

needs_cli = pytest.mark.skipif(not CLI, reason="AUTOPLAY_CLI (native autoplay binary) not set")


def cli(*args: str) -> subprocess.CompletedProcess[str]:
    """Run the native CLI in a fresh process from an unrelated directory."""
    return subprocess.run(
        [CLI, *args, "--assets-root", str(assets_root())],
        capture_output=True,
        text=True,
        cwd=Path(CLI).parent,
    )


def record(scenario: str, policy_id: str, seed: int = 3) -> str:
    env = AutoplayEnv(scenario)
    row = run_episode(env, make_policy(policy_id), seed, record_as=("0001", "test-commit"))
    return row["replay"]


@needs_cli
@pytest.mark.parametrize(
    ("scenario", "policy_id"),
    [
        ("cp01_single_goblin", "reactive_survivor"),
        ("cp02_two_goblins", "approach_and_attack"),
        ("ap_single_goblin_far", "random_legal"),
    ],
)
def test_python_episode_verifies_in_a_fresh_native_process(tmp_path, scenario, policy_id):
    path = tmp_path / "episode.jsonl"
    path.write_text(record(scenario, policy_id))
    result = cli("replay", "--episode", str(path), "--verify")
    assert result.returncode == 0, result.stderr
    assert "replay verified" in result.stdout


@needs_cli
def test_corrupted_python_episode_fails_verification(tmp_path):
    text = record("cp01_single_goblin", "reactive_survivor")
    lines = text.splitlines()
    # Flip the action of a tick after the first checkpoint: the hashes must diverge.
    index = next(i for i, line in enumerate(lines) if json.loads(line).get("t") == 100)
    tick = json.loads(lines[index])
    assert tick["type"] == "tick"
    tick["a"] = "000000" if tick["a"] != "000000" else "222222"
    lines[index] = json.dumps(tick, separators=(",", ":"))
    path = tmp_path / "corrupted.jsonl"
    path.write_text("\n".join(lines) + "\n")
    result = cli("replay", "--episode", str(path), "--verify")
    assert result.returncode != 0
    assert "replay verified" not in result.stdout


@needs_cli
def test_tampered_checkpoint_hash_fails_verification(tmp_path):
    lines = record("cp01_single_goblin", "approach_and_attack").splitlines()
    index = next(i for i, line in enumerate(lines) if json.loads(line)["type"] == "checkpoint")
    checkpoint = json.loads(lines[index])
    checkpoint["state_hash"] = "0" * 16
    lines[index] = json.dumps(checkpoint, separators=(",", ":"))
    path = tmp_path / "tampered.jsonl"
    path.write_text("\n".join(lines) + "\n")
    assert cli("replay", "--episode", str(path), "--verify").returncode != 0


@needs_cli
def test_python_replay_is_byte_identical_to_the_native_cli_run(tmp_path):
    """The Python path writes exactly what `autoplay run` writes for the same bot and seed."""
    native = tmp_path / "native.jsonl"
    result = cli(
        "run", "--scenario", "cp01_single_goblin", "--policy", "reactive_survivor",
        "--seed", "3", "--out", str(native),
    )
    assert result.returncode == 0, result.stderr
    python_lines = record("cp01_single_goblin", "reactive_survivor").splitlines()
    native_lines = native.read_text().splitlines()
    for lines in (python_lines, native_lines):  # only the commit label may differ
        header = json.loads(lines[0])
        header["commit"] = "x"
        lines[0] = json.dumps(header, sort_keys=True)
    assert python_lines == native_lines


@needs_cli
def test_evaluate_writes_verifiable_replays(tmp_path):
    result = evaluate(
        ["cp01_single_goblin"],
        make_policy("reactive_survivor"),
        2,
        7,
        replays="all",
        replay_dir=tmp_path / "replays",
        commit="test-commit",
    )
    assert len(result["replay_files"]) == 2
    for name in result["replay_files"]:
        assert cli("replay", "--episode", str(tmp_path / "replays" / name), "--verify").returncode == 0


def test_recording_contract_errors():
    env = AutoplayEnv("cp01_single_goblin")
    with pytest.raises(Exception, match="reset"):
        env.start_recording("x", "0001", "c")
    env.reset(seed=1)
    env.start_recording("x", "0001", "c")
    with pytest.raises(RuntimeError, match="terminated or truncated"):
        env.finish_recording()
    env.reset(seed=1)  # a new reset drops the recording
    with pytest.raises(RuntimeError, match="no recording|terminated"):
        env.finish_recording()
    env.step([1, 0, 0, 0, 0, 0])
    with pytest.raises(RuntimeError, match="right after reset"):
        env.start_recording("x", "0001", "c")


def test_recording_needs_action_repeat_one():
    env = AutoplayEnv("cp01_single_goblin", action_repeat=4)
    env.reset(seed=1)
    with pytest.raises(ValueError, match="action_repeat"):
        env.start_recording("x", "0001", "c")


def test_replay_mode_is_validated(tmp_path):
    with pytest.raises(ValueError, match="replays must be"):
        evaluate(["cp01_single_goblin"], make_policy("noop"), 1, 0, replays="some")
    with pytest.raises(ValueError, match="replay_dir"):
        evaluate(["cp01_single_goblin"], make_policy("noop"), 1, 0, replays="first")
