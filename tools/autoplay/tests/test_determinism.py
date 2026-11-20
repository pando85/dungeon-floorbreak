import numpy as np

from autoplay import AutoplayEnv

from conftest import DEV, pattern_actions


def run(seed, scenario=DEV, **kwargs):
    env = AutoplayEnv(scenario, **kwargs)
    env.reset(seed=seed)
    hashes, obs_bytes, rewards = [], [], []
    for action in pattern_actions():
        obs, reward, terminated, truncated, _ = env.step(action)
        hashes.append(env.state_hash())
        obs_bytes.append(b"".join(obs[k].tobytes() for k in sorted(obs)))
        rewards.append(reward)
        if terminated or truncated:
            break
    return hashes, obs_bytes, rewards


def test_same_seed_and_actions_give_identical_runs():
    assert run(5) == run(5)


def test_reset_seed_does_not_reach_the_simulation():
    assert run(5) == run(987654321)


def test_reset_twice_from_an_ended_episode_is_identical():
    env = AutoplayEnv(DEV, max_ticks=30)
    obs_a, _ = env.reset(seed=3)
    hash_a = env.state_hash()
    terminated = truncated = False
    while not (terminated or truncated):
        _, _, terminated, truncated, _ = env.step([1, 0, 0, 0, 0, 0])
    obs_b, _ = env.reset(seed=3)
    assert env.state_hash() == hash_a
    assert all(np.array_equal(obs_a[k], obs_b[k]) for k in obs_a)


def test_seed_is_disclosed_and_the_sim_does_not_use_it():
    env = AutoplayEnv(DEV)
    _, info = env.reset(seed=77)
    assert info["requested_seed"] == 77
    assert info["seed_affects_sim"] is False
    assert info["cf02_rules"] is False
    assert isinstance(info["sim_seed"], int)


def test_unseeded_reset_draws_from_the_adapter_rng_and_records_it():
    env = AutoplayEnv(DEV)
    env.reset(seed=11)
    _, info_a = env.reset()
    env2 = AutoplayEnv(DEV)
    env2.reset(seed=11)
    _, info_b = env2.reset()
    assert info_a["requested_seed"] == info_b["requested_seed"]


def test_observation_does_not_advance_or_mutate_the_sim():
    env = AutoplayEnv(DEV)
    env.reset(seed=0)
    for _ in range(40):
        env.step([2, 0, 0, 0, 0, 0])
    before = env.state_hash()
    first = env._native.observe()
    second = env._native.observe()
    assert env.state_hash() == before
    assert all(np.array_equal(first[k], second[k]) for k in first)


def test_held_button_across_an_action_repeat_is_one_hold_not_new_presses():
    # One step of repeat=4 must equal four steps of repeat=1 with the same action.
    action = [2, 0, 1, 0, 0, 0]
    bundled = AutoplayEnv(DEV, action_repeat=4)
    bundled.reset(seed=0)
    single = AutoplayEnv(DEV, action_repeat=1)
    single.reset(seed=0)
    for _ in range(10):
        _, _, _, _, info = bundled.step(action)
        for _ in range(4):
            single.step(action)
        assert bundled.state_hash() == single.state_hash()
        assert info["ticks_executed"] == 4
