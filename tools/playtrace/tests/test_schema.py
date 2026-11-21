from __future__ import annotations

import math

import numpy as np
import pytest

from playtrace_tools import schema
from synthetic_journal import encode_obs_bits, f32_bits, make_obs


def test_obs_layout_is_345_values():
    assert schema.OBS_LEN == 345


def test_obs_roundtrip_is_exact_and_validates():
    rng = np.random.default_rng(3)
    for hero in (0, 1):
        text = make_obs(hero, rng)
        obs = schema.decode_obs(text)
        assert obs.dtype == np.float32 and obs.shape == (schema.OBS_LEN,)
        schema.validate_obs(obs, hero)
        again = encode_obs_bits([int(b) for b in obs.view(np.uint32)])
        assert again == text


def test_obs_decoder_is_strict():
    for bad in ("z0", "zx", "0f", "g1", "1ffffffff", ",", "z2,"):
        with pytest.raises(schema.SchemaError):
            schema.decode_obs_bits(bad)
    with pytest.raises(schema.SchemaError):
        schema.decode_obs("z344")  # wrong length


def test_validate_obs_rejects_wrong_seat_nonfinite_and_leaky_rows():
    rng = np.random.default_rng(4)
    carl = schema.decode_obs(make_obs(0, rng))
    with pytest.raises(schema.SchemaError):
        schema.validate_obs(carl, 1)  # a Carl observation is not Donut's
    bad = carl.copy()
    bad[0] = np.nan
    with pytest.raises(schema.SchemaError):
        schema.validate_obs(bad, 0)
    bad = carl.copy()
    bad[5] = 2.0
    with pytest.raises(schema.SchemaError):
        schema.validate_obs(bad, 0)
    bad = carl.copy()
    # An enemy row populated without its mask bit would leak an unseen entity.
    first_enemy = schema.SELF_LEN + schema.ALLY_LEN + 1
    mask_start = first_enemy + schema.MAX_ENEMIES * schema.ENEMY_FEATURES
    bad[first_enemy + 5 * schema.ENEMY_FEATURES] = 0.5
    assert bad[mask_start + 5] == 0.0
    with pytest.raises(schema.SchemaError):
        schema.validate_obs(bad, 0)


def test_policy_action_discretizes_and_strips_nonpolicy_bits():
    right_jump = [f32_bits(1.0), 1 | 32 | 16]  # jump + swap + pause
    assert schema.policy_action(right_jump) == [2, 1, 0, 0, 0, 0]
    assert schema.policy_action([f32_bits(-0.7), 2 | 4 | 8 | 64]) == [0, 0, 1, 1, 1, 1]
    assert schema.policy_action([f32_bits(0.49), 0]) == [1, 0, 0, 0, 0, 0]
    move, buttons = schema.unpack_action(right_jump)
    assert move == 1.0 and buttons["swap"] and buttons["pause"] and buttons["jump"]


def test_legal_actions_only():
    schema.check_action_legal([f32_bits(-1.0), 127])
    for bad in ([f32_bits(math.nan), 0], [f32_bits(1.5), 0], [f32_bits(math.inf), 0], [0, 128], [0], [-1, 0]):
        with pytest.raises(schema.SchemaError):
            schema.check_action_legal(bad)


def test_mask_rule_is_observation_only():
    rng = np.random.default_rng(5)
    obs = schema.decode_obs(make_obs(1, rng))
    assert schema.action_mask(obs) == [1, 1, 1, 1, 0, 1]
    obs[schema.SELF_CAN_CAST] = 1.0
    assert schema.action_mask(obs) == [1, 1, 1, 1, 1, 1]
