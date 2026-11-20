"""Tiny PPO round trip. Skipped when the optional `train` extra is not installed."""

import pytest

pytest.importorskip("stable_baselines3")
pytest.importorskip("torch")

from autoplay.checkpoint import MODEL_FILE, SchemaMismatchError, read_metadata
from autoplay.train import load_policy, train


def test_train_save_reload_evaluate_and_schema_refusal(tmp_path):
    smoke = train(["cp01_single_goblin"], 256, 3, tmp_path, eval_episodes=1, ppo={"n_steps": 128})
    assert smoke["steps"] >= 256
    assert (tmp_path / MODEL_FILE).is_file()
    record = read_metadata(tmp_path)
    assert record["device"] == "cpu" and record["seed"] == 3
    assert record["environment"]["versions"]["stable-baselines3"]
    policy, _ = load_policy(tmp_path)
    assert policy.policy_id == "ppo"
    record["environment"]["schema"]["observation_version"] = 99
    (tmp_path / "metadata.json").write_text(__import__("json").dumps(record))
    with pytest.raises(SchemaMismatchError):
        load_policy(tmp_path)
