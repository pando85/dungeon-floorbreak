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
    record = verify_checkpoint(make_checkpoint(tmp_path))
    assert record["schema"] == "autoplay-checkpoint/1"


@pytest.mark.parametrize(
    "override",
    [
        {"observation_version": 2},
        {"action_version": 2},
        {"action_nvec": [3, 2, 2, 2, 2, 2, 2]},
        {"observation_schema_sha256": "0" * 64},
    ],
)
def test_mismatched_schema_is_rejected(tmp_path, override):
    with pytest.raises(SchemaMismatchError, match="schema mismatch"):
        verify_checkpoint(make_checkpoint(tmp_path, **override))


def test_tampered_model_is_rejected(tmp_path):
    make_checkpoint(tmp_path)
    (tmp_path / MODEL_FILE).write_bytes(b"changed")
    with pytest.raises(SchemaMismatchError, match="checksum"):
        verify_checkpoint(tmp_path)


def test_missing_or_foreign_metadata_is_rejected(tmp_path):
    with pytest.raises(SchemaMismatchError):
        verify_checkpoint(tmp_path)
    (tmp_path / METADATA_FILE).write_text(json.dumps({"schema": "other/9"}))
    with pytest.raises(SchemaMismatchError, match="unknown checkpoint metadata schema"):
        verify_checkpoint(tmp_path)
