import numpy as np
import pytest
from gymnasium import spaces

from autoplay import ACTION_NVEC, AutoplayEnv, InvalidActionError, scenario_roster
from autoplay.env import observation_schema


def test_action_space_is_exactly_actionv1():
    env = AutoplayEnv()
    assert isinstance(env.action_space, spaces.MultiDiscrete)
    assert list(env.action_space.nvec) == [3, 2, 2, 2, 2, 2] == list(ACTION_NVEC)


def test_observation_space_matches_the_native_schema():
    env = AutoplayEnv()
    schema = observation_schema()
    space = env.observation_space
    assert isinstance(space, spaces.Dict)
    assert set(space.spaces) == {
        "self", "enemies", "enemy_mask", "hazards", "hazard_mask", "available",
    }
    assert space["self"].shape == (14,)
    assert space["enemies"].shape == (16, 14)
    assert space["hazards"].shape == (8, 6)
    assert space["enemy_mask"].shape == (16,)
    assert space["available"].shape == (2,)
    assert len(schema["self_names"]) == 14 and len(schema["enemy_names"]) == 14
    for key in space.spaces:
        assert space[key].dtype == np.float32
    assert list(space["self"].low) == [b[0] for b in schema["self_bounds"]]
    assert list(space["self"].high) == [b[1] for b in schema["self_bounds"]]


def test_observations_are_float32_in_space_and_available_is_zero():
    env = AutoplayEnv("ap_two_goblins_wide")
    obs, _ = env.reset(seed=1)
    assert env.observation_space.contains(obs)
    assert list(obs["available"]) == [0.0, 0.0]
    assert not obs["hazard_mask"].any()


@pytest.mark.parametrize("index,value", [(0, 3), (1, 2), (2, 2), (3, 2), (4, 2), (5, 2), (0, -1)])
def test_out_of_range_components_are_rejected_not_clamped(index, value):
    env = AutoplayEnv()
    env.reset(seed=0)
    action = [1, 0, 0, 0, 0, 0]
    action[index] = value
    with pytest.raises(InvalidActionError):
        env._native.step(action)  # bypass the adapter's own shape check


@pytest.mark.parametrize("action", [[1, 0, 0], [1, 0, 0, 0, 0, 0, 0], [[1, 0, 0, 0, 0, 0]], [1.0, 0, 0, 0, 0, 0]])
def test_wrong_shape_or_dtype_is_rejected(action):
    env = AutoplayEnv()
    env.reset(seed=0)
    with pytest.raises(InvalidActionError):
        env.step(action)


def test_roster_has_two_development_and_two_holdout():
    roster = dict(scenario_roster())
    assert sorted(v for v in roster.values()) == ["development"] * 2 + ["holdout"] * 2
    assert "cp01_single_goblin" in roster


def test_unsupported_scenario_is_rejected():
    with pytest.raises(ValueError, match="unsupported autoplay scenario"):
        AutoplayEnv("cp02_kick_contact")
