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


def test_standing_still_ends_in_a_defeat_at_a_pinned_tick_with_no_restart():
    # Deterministic: idle Carl on cp01_single_goblin is defeated at tick 438. The
    # fixture restarts 49 ticks later; the tick counter staying at 438 after the
    # terminal step shows no restart happened inside the episode.
    env = AutoplayEnv(DEV)
    info = play(env, lambda obs: [1, 0, 0, 0, 0, 0])
    assert info["outcome"] == "defeat" and info["terminal_reason"] == "defeat"
    assert info["tick"] == 438
    with pytest.raises(EpisodeOverError):
        env.step([1, 0, 0, 0, 0, 0])
    assert env.diagnostics()["tick"] == 438


def test_an_observation_only_script_ends_in_its_pinned_deterministic_outcome():
    # chaser_action closes in and holds primary; on cp01_single_goblin it loses at tick 354.
    env = AutoplayEnv(DEV)
    info = play(env, chaser_action)
    assert info["outcome"] == "defeat" and info["terminal_reason"] == "defeat"
    assert info["tick"] == 354
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
