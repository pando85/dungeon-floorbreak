"""Schema check of checkpoints; needs neither SB3 nor torch."""

import json

import pytest

from autoplay.checkpoint import (
    METADATA_FILE,
    MODEL_FILE,
    SchemaMismatchError,
    verify_checkpoint,
    write_metadata,
)
from autoplay.provenance import schema_stamp


def make_checkpoint(tmp_path, **schema_overrides):
    (tmp_path / MODEL_FILE).write_bytes(b"not a real model")
    schema = {**schema_stamp("combat-v1"), **schema_overrides}
    write_metadata(tmp_path, {"environment": {"schema": schema}})
    return tmp_path


def test_matching_schema_loads(tmp_path):
    record = verify_checkpoint(make_checkpoint(tmp_path), reward_id="combat-v1")
    assert record["schema"] == "autoplay-checkpoint/1"


@pytest.mark.parametrize(
    "override",
    [
        {"observation_version": 2},
        {"action_version": 2},
        {"action_nvec": [3, 2, 2, 2, 2, 2, 2]},
        {"observation_schema_sha256": "0" * 64},
        {"reward_id": "sparse-v1"},
        {"reward_profiles_sha256": "0" * 64},
    ],
)
def test_mismatched_schema_is_rejected(tmp_path, override):
    with pytest.raises(SchemaMismatchError, match="schema mismatch"):
        verify_checkpoint(make_checkpoint(tmp_path, **override), reward_id="combat-v1")


def test_checkpoint_is_refused_for_another_reward_profile_than_it_was_trained_on(tmp_path):
    make_checkpoint(tmp_path)  # trained on combat-v1
    verify_checkpoint(tmp_path, reward_id="combat-v1")
    with pytest.raises(SchemaMismatchError, match="reward_id"):
        verify_checkpoint(tmp_path, reward_id="sparse-v1")


def test_tampered_model_is_rejected(tmp_path):
    make_checkpoint(tmp_path)
    (tmp_path / MODEL_FILE).write_bytes(b"changed")
    with pytest.raises(SchemaMismatchError, match="checksum"):
        verify_checkpoint(tmp_path, reward_id="combat-v1")


def test_missing_or_foreign_metadata_is_rejected(tmp_path):
    with pytest.raises(SchemaMismatchError):
        verify_checkpoint(tmp_path, reward_id="combat-v1")
    (tmp_path / METADATA_FILE).write_text(json.dumps({"schema": "other/9"}))
    with pytest.raises(SchemaMismatchError, match="unknown checkpoint metadata schema"):
        verify_checkpoint(tmp_path, reward_id="combat-v1")
