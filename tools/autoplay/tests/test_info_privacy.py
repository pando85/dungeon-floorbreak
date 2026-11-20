from autoplay import AutoplayEnv
from autoplay.env import RESET_INFO_KEYS, STEP_INFO_KEYS

from conftest import DEV, pattern_actions


PRIVILEGED_KEYS = {
    "events", "state_hash", "initial_hash", "event_cursor", "required_enemies",
    "health", "enemies", "cooldown", "carl_health", "diagnostics",
}


def all_keys(value):
    if isinstance(value, dict):
        for key, inner in value.items():
            yield key
            yield from all_keys(inner)


def test_info_keys_are_whitelisted_and_carry_no_privileged_data():
    env = AutoplayEnv(DEV)
    _, info = env.reset(seed=0)
    assert set(info) == RESET_INFO_KEYS
    for action in pattern_actions()[:120]:
        _, _, terminated, truncated, info = env.step(action)
        assert set(info) == STEP_INFO_KEYS
        for key in all_keys(info):
            assert key not in PRIVILEGED_KEYS, key
        if terminated or truncated:
            break


def test_privileged_accessors_are_separate_from_info():
    env = AutoplayEnv(DEV)
    env.reset(seed=0)
    assert isinstance(env.state_hash(), int)
    assert "carl_health" in env.diagnostics()
