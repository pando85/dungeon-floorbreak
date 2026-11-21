from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest

from conftest import pass_verifier
from playtrace_tools import schema
from playtrace_tools.bc import CAVEAT, TrainConfig, features, train_bc
from playtrace_tools.dataset import DatasetError, load_dataset
from playtrace_tools.export import Options, export_dataset
from synthetic_journal import write_session


def make_dataset(base, out, context=0):
    options = Options(kind="bc", actor="human", allow_synthetic=True, context=context)
    return load_dataset(Path(export_dataset(base, out, options, pass_verifier)["path"]))


def test_bc_smoke_trains_validates_and_reports_real_numbers(five_sessions, out):
    dataset = make_dataset(five_sessions, out)
    report = train_bc(dataset, TrainConfig(epochs=8, seed=0))
    assert report["caveat"].startswith(CAVEAT) and "SYNTHETIC" in report["caveat"]
    assert report["synthetic"] is True
    assert report["rows"]["train"] > 0 and report["rows"]["dev"] > 0
    assert report["sessions_by_split"]["train"] >= 1 and report["sessions_by_split"]["dev"] >= 1
    dev = report["dev"]
    for key in ("horizontal_accuracy", "horizontal_majority_baseline_accuracy", "exact_action_accuracy"):
        assert 0.0 <= dev[key] <= 1.0
    assert set(dev["buttons"]) == set(schema.ACTION_COMPONENTS[1:])
    assert np.isfinite(dev["horizontal_cross_entropy"]) and np.isfinite(dev["button_bce"])
    # Training loss goes down: the optimizer works.
    assert report["curve"][-1]["train_ce"] < report["curve"][0]["train_ce"]


def test_bc_smoke_is_deterministic(five_sessions, out):
    dataset = make_dataset(five_sessions, out)
    config = TrainConfig(epochs=4, seed=3)
    assert train_bc(dataset, config) == train_bc(dataset, config)


def test_policy_features_are_the_observation_and_context_only(five_sessions, out):
    dataset = make_dataset(five_sessions, out, context=3)
    stream = dataset.streams["human"]
    index = np.arange(10)
    assert features(stream, index, 0).shape == (10, schema.OBS_LEN)
    assert features(stream, index, 2).shape == (10, schema.OBS_LEN + 2 * schema.ACTION_LEN + 2)
    assert np.array_equal(features(stream, index, 0), stream.obs[index])
    with pytest.raises(DatasetError, match="context"):
        features(stream, index, 5)
    report = train_bc(dataset, TrainConfig(epochs=2, history=2))
    assert "repeat_previous_action_baseline" in report["dev"]
    assert report["inputs"].startswith("ObservationV2")


def test_bc_needs_a_leak_free_validation_split(base, out):
    write_session(base, "a" * 32, seed=1)
    dataset = make_dataset(base, out)
    with pytest.raises(DatasetError, match="at least two sessions"):
        train_bc(dataset, TrainConfig(epochs=1))


def test_bc_refuses_the_ai_stream(five_sessions, out):
    options = Options(kind="transitions", actor="both", allow_synthetic=True)
    dataset = load_dataset(Path(export_dataset(five_sessions, out, options, pass_verifier)["path"]))
    with pytest.raises(DatasetError, match="human stream only"):
        train_bc(dataset, TrainConfig(stream="ai", epochs=1))


def test_context_padding_is_marked_invalid_not_read_as_left(five_sessions, out):
    """F-3: the first rows of a trajectory have no previous action; a validity bit says so."""
    stream = make_dataset(five_sessions, out, context=3).streams["human"]
    first = next(i for i, r in enumerate(stream.rows) if r["ctx_len"] == 0)
    second = next(i for i, r in enumerate(stream.rows) if r["ctx_len"] == 1)
    third = next(i for i, r in enumerate(stream.rows) if r["ctx_len"] == 3)
    width = schema.OBS_LEN + 3 * schema.ACTION_LEN
    x = features(stream, np.array([first, second, third]), 3)
    assert x[0, width:].tolist() == [0, 0, 0]
    assert x[1, width:].tolist() == [0, 0, 1]
    assert x[2, width:].tolist() == [1, 1, 1]
    # a real "left" first action differs from padding only by its validity bit
    assert np.array_equal(x[0, schema.OBS_LEN:width], np.zeros(3 * schema.ACTION_LEN))
