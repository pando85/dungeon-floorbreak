import pytest

from autoplay import AutoplayEnv, EpisodeOverError

from conftest import DEV, chaser_action


def test_step_before_reset_raises():
    env = AutoplayEnv(DEV)
    with pytest.raises(EpisodeOverError):
        env.step([1, 0, 0, 0, 0, 0])


def test_timeout_is_truncated_not_terminated_and_step_after_raises():
    env = AutoplayEnv(DEV, max_ticks=25)
    env.reset(seed=0)
    for _ in range(24):
        _, _, terminated, truncated, info = env.step([1, 0, 0, 0, 0, 0])
        assert not terminated and not truncated and info["outcome"] is None
    _, _, terminated, truncated, info = env.step([1, 0, 0, 0, 0, 0])
    assert truncated and not terminated
    assert info["outcome"] == "timeout" and info["tick"] == 25
    with pytest.raises(EpisodeOverError):
        env.step([1, 0, 0, 0, 0, 0])
    env.reset(seed=0)
    env.step([1, 0, 0, 0, 0, 0])  # a reset makes it usable again


def play(env, chooser, seed=0, limit=1800):
    obs, _ = env.reset(seed=seed)
    for _ in range(limit):
        obs, reward, terminated, truncated, info = env.step(chooser(obs))
        if terminated or truncated:
            return info
    raise AssertionError("episode did not end")


def test_standing_still_ends_in_a_terminal_defeat_before_any_restart():
    env = AutoplayEnv(DEV)
    info = play(env, lambda obs: [1, 0, 0, 0, 0, 0])
    assert info["outcome"] in {"defeat", "timeout"}
    if info["outcome"] == "defeat":
        with pytest.raises(EpisodeOverError):
            env.step([1, 0, 0, 0, 0, 0])


def test_an_observation_only_script_can_reach_a_terminal_outcome():
    env = AutoplayEnv(DEV)
    info = play(env, chaser_action)
    assert info["outcome"] in {"success", "defeat", "abandoned", "timeout"}
    assert info["terminal_reason"] == info["outcome"]
    with pytest.raises(EpisodeOverError):
        env.step([1, 0, 0, 0, 0, 0])


def test_reserved_actions_are_counted_not_granted():
    env = AutoplayEnv(DEV)
    env.reset(seed=0)
    _, _, _, _, info = env.step([1, 0, 0, 0, 1, 1])
    assert info["unavailable"]["special_presses"] == 1
    assert info["unavailable"]["swap_presses"] == 1
    obs = env._native.observe()
    assert list(obs["available"]) == [0.0, 0.0]
