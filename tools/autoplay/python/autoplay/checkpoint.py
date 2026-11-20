"""Checkpoint metadata and the schema check on reload.

A checkpoint is ``model.zip`` plus ``metadata.json``. Reload refuses a schema
mismatch (observation, action or reward version) and a model file whose
checksum differs from the one recorded at save time.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from autoplay.provenance import schema_stamp

METADATA_FILE = "metadata.json"
MODEL_FILE = "model.zip"
METADATA_SCHEMA = "autoplay-checkpoint/1"


class SchemaMismatchError(RuntimeError):
    """The checkpoint was trained against a different env schema."""


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_metadata(directory: Path, record: dict[str, Any]) -> Path:
    record = {"schema": METADATA_SCHEMA, **record}
    record["model_sha256"] = sha256_file(directory / MODEL_FILE)
    path = directory / METADATA_FILE
    path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    return path


def read_metadata(directory: Path) -> dict[str, Any]:
    path = directory / METADATA_FILE
    if not path.is_file():
        raise SchemaMismatchError(f"{path} is missing; not an autoplay checkpoint")
    record = json.loads(path.read_text())
    if record.get("schema") != METADATA_SCHEMA:
        raise SchemaMismatchError(
            f"unknown checkpoint metadata schema {record.get('schema')!r}, "
            f"expected {METADATA_SCHEMA!r}"
        )
    return record


def verify_checkpoint(directory: Path) -> dict[str, Any]:
    """Return the metadata, or raise :class:`SchemaMismatchError`."""
    record = read_metadata(directory)
    recorded = record.get("environment", {}).get("schema", {})
    current = schema_stamp(recorded.get("reward_id", ""))
    mismatches = [
        f"{key}: checkpoint {recorded.get(key)!r} != current {current[key]!r}"
        for key in current
        if recorded.get(key) != current[key]
    ]
    if mismatches:
        raise SchemaMismatchError("schema mismatch, refusing to load: " + "; ".join(mismatches))
    model = directory / MODEL_FILE
    if not model.is_file():
        raise SchemaMismatchError(f"{model} is missing")
    if sha256_file(model) != record.get("model_sha256"):
        raise SchemaMismatchError(f"{model} does not match the checksum in {METADATA_FILE}")
    return record
