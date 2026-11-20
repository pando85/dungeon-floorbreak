import json

from autoplay.bots import NoopPolicy, RandomLegalPolicy, make_policy
from autoplay.evaluate import evaluate, write_report

import pytest


def test_seeded_baseline_evaluation_is_reproducible():
    a = evaluate(["cp01_single_goblin"], RandomLegalPolicy(), 3, 42)
    b = evaluate(["cp01_single_goblin"], RandomLegalPolicy(), 3, 42)
    assert a == b
    assert a["overall"]["episodes"] == 3


def test_different_seed_changes_the_random_policy():
    a = evaluate(["cp01_single_goblin"], RandomLegalPolicy(), 3, 42)
    b = evaluate(["cp01_single_goblin"], RandomLegalPolicy(), 3, 43)
    assert a["rows"] != b["rows"]


def test_random_legal_never_touches_reserved_components():
    policy = RandomLegalPolicy()
    policy.reset(1)
    for _ in range(200):
        action = policy.act({})
        assert action[4] == 0 and action[5] == 0


def test_report_files_are_written(tmp_path):
    result = evaluate(["cp01_single_goblin"], NoopPolicy(), 2, 1)
    write_report(result, tmp_path, {"note": "x"})
    summary = json.loads((tmp_path / "summary.json").read_text())
    assert summary["policy_id"] == "noop" and summary["note"] == "x"
    assert (tmp_path / "episodes.csv").read_text().startswith("scenario,episode,seed")


def test_unknown_policy_is_rejected():
    with pytest.raises(ValueError, match="unknown policy"):
        make_policy("reactive_survivor")
