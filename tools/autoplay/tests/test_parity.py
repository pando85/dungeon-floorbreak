"""Parity: the Python env drives the native sim exactly like the native env."""

from autoplay import AutoplayEnv, _native

from conftest import DEV, pattern_actions

# Literal goldens of the fixed sequence on cp01_single_goblin as (steps, state_hash).
# Pinned in tools/autoplay/native/src/bridge.rs too: both sides must move together.
GOLDEN = {
    1: 7_889_728_490_312_218_380,
    90: 2_903_576_537_444_934_922,
    180: 5_063_528_022_019_914_666,
}


def python_hashes(scenario, actions, **kwargs):
    env = AutoplayEnv(scenario, **kwargs)
    env.reset(seed=0)
    hashes = []
    for action in actions:
        _, _, terminated, truncated, _ = env.step(action)
        hashes.append(env.state_hash())
        if terminated or truncated:
            break
    return hashes


def test_per_tick_hashes_equal_the_native_env():
    actions = pattern_actions()
    assert python_hashes(DEV, actions) == _native.reference_hashes(DEV, actions)


def test_every_roster_scenario_matches_the_native_env():
    actions = pattern_actions()
    for scenario in ("ap_two_goblins_wide", "cp02_two_goblins", "ap_single_goblin_far"):
        assert python_hashes(scenario, actions) == _native.reference_hashes(scenario, actions)


def test_action_repeat_parity():
    actions = pattern_actions()[::4]
    assert python_hashes(DEV, actions, action_repeat=4) == _native.reference_hashes(
        DEV, actions, action_repeat=4
    )


def test_golden_hashes_are_pinned():
    hashes = python_hashes(DEV, pattern_actions())
    for count, expected in GOLDEN.items():
        assert hashes[count - 1] == expected, f"step {count}"
