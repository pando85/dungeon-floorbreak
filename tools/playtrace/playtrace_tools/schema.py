"""Observation, action and mask contracts shared by the exporter and the loader.

Nothing here defines a competing observation type: the policy observation is
the game's ObservationV2 (``autoplay-obs/2``) of one hero, stored by the
recorder as ``f32bits-rle-hex/1``. The layout below mirrors
``src/autoplay/party/observation.rs`` and is checked by length and structure on
every row; a change of the schema fails the export instead of being padded.
"""

from __future__ import annotations

import struct

import numpy as np

OBS_SCHEMA_ID = "autoplay-obs/2+f32bits-rle-hex/1"
ACTION_SCHEMAS = ("rawinput-bits/1", "rawinput-bits/2")

# ObservationV2 layout (flattened, in schema order).
SELF_LEN = 22
ALLY_LEN = 24
MAX_ENEMIES = 16
ENEMY_FEATURES = 14
MAX_HAZARDS = 8
HAZARD_FEATURES = 6
AVAILABLE_LEN = 2

_OWN = slice(0, SELF_LEN)
_ALLY = slice(SELF_LEN, SELF_LEN + ALLY_LEN)
_ALLY_MASK = SELF_LEN + ALLY_LEN
_ENEMIES = slice(_ALLY_MASK + 1, _ALLY_MASK + 1 + MAX_ENEMIES * ENEMY_FEATURES)
_ENEMY_MASK = slice(_ENEMIES.stop, _ENEMIES.stop + MAX_ENEMIES)
_HAZARDS = slice(_ENEMY_MASK.stop, _ENEMY_MASK.stop + MAX_HAZARDS * HAZARD_FEATURES)
_HAZARD_MASK = slice(_HAZARDS.stop, _HAZARDS.stop + MAX_HAZARDS)
_AVAILABLE = slice(_HAZARD_MASK.stop, _HAZARD_MASK.stop + AVAILABLE_LEN)
OBS_LEN = _AVAILABLE.stop  # 345

# Indices inside the ``own`` block.
SELF_IS_CARL = 14
SELF_IS_DONUT = 15
SELF_CAN_CAST = 21

HEROES = ("carl", "donut")

# Policy action: horizontal in {0 left, 1 neutral, 2 right}, then five buttons.
# This is ActionV1's order (horizontal, jump, primary, physical, special) with
# ``down`` appended (CF02) and the reserved ``swap`` left out: ``swap`` is a
# party command and ``pause`` a system button, neither is a hero action.
ACTION_COMPONENTS = ("horizontal", "jump", "primary", "physical", "special", "down")
ACTION_SIZES = (3, 2, 2, 2, 2, 2)
ACTION_LEN = len(ACTION_COMPONENTS)

# Same threshold as ``playtrace stats`` (``STICK_THRESHOLD`` in the Rust bin).
STICK_THRESHOLD = 0.5
MASK_RULE = "mask-rule/1"

_BUTTON_BITS = {
    "jump": 1,
    "attack": 2,
    "physical": 4,
    "ability": 8,
    "pause": 16,
    "swap": 32,
    "down": 64,
}


class SchemaError(ValueError):
    """A value violates the observation or action contract."""


def decode_obs_bits(text: str) -> list[int]:
    """Strict inverse of the recorder's ``encode_obs_bits``."""
    out: list[int] = []
    if not text:
        return out
    for token in text.split(","):
        if token.startswith("z"):
            run = token[1:]
            if not run.isdigit() or run.startswith("0"):
                raise SchemaError(f"bad zero run {token!r}")
            out.extend([0] * int(run))
        else:
            if not token or token.startswith("0"):
                raise SchemaError(f"bad value token {token!r}")
            try:
                value = int(token, 16)
            except ValueError as error:
                raise SchemaError(f"bad value token {token!r}") from error
            if value > 0xFFFFFFFF:
                raise SchemaError(f"value token {token!r} exceeds 32 bits")
            out.append(value)
    return out


def decode_obs(text: str) -> np.ndarray:
    """One observation as ``float32[OBS_LEN]`` (exact bits)."""
    bits = decode_obs_bits(text)
    if len(bits) != OBS_LEN:
        raise SchemaError(f"observation has {len(bits)} values, expected {OBS_LEN}")
    return np.array(bits, dtype=np.uint32).view(np.float32)


def validate_obs(obs: np.ndarray, hero: int) -> None:
    """Structural checks of one observation of ``hero`` (0 carl, 1 donut)."""
    if not np.all(np.isfinite(obs)):
        raise SchemaError("observation holds a non-finite value")
    if obs.min() < -1.0 or obs.max() > 1.0:
        raise SchemaError("observation value outside [-1, 1]")
    own = obs[_OWN]
    want = (1.0, 0.0) if hero == 0 else (0.0, 1.0)
    if (own[SELF_IS_CARL], own[SELF_IS_DONUT]) != want:
        raise SchemaError("the seat identity flags do not match the hero")
    masks = np.concatenate(
        [obs[_ALLY_MASK : _ALLY_MASK + 1], obs[_ENEMY_MASK], obs[_HAZARD_MASK]]
    )
    if not np.all((masks == 0.0) | (masks == 1.0)):
        raise SchemaError("a presence mask is not 0 or 1")
    enemies = obs[_ENEMIES].reshape(MAX_ENEMIES, ENEMY_FEATURES)
    if np.any(enemies[obs[_ENEMY_MASK] == 0.0] != 0.0):
        raise SchemaError("an unmasked enemy row is not zero")
    hazards = obs[_HAZARDS].reshape(MAX_HAZARDS, HAZARD_FEATURES)
    if np.any(hazards[obs[_HAZARD_MASK] == 0.0] != 0.0):
        raise SchemaError("an unmasked hazard row is not zero")


def action_mask(obs: np.ndarray) -> list[int]:
    """Which action components the observation offers (``mask-rule/1``).

    Derived from observation fields only: ``special`` is selectable when the
    ``available`` block offers it or the seat's own ``can_cast`` flag is set;
    every other component is always selectable. A pressed button the mask does
    not offer is kept in the data and counted as a mask violation.
    """
    special = bool(obs[_AVAILABLE][0] > 0.0 or obs[_OWN][SELF_CAN_CAST] > 0.0)
    return [1, 1, 1, 1, int(special), 1]


def unpack_action(bits: list[int]) -> tuple[float, dict[str, bool]]:
    """``(move_x, buttons)`` of a ``[move_x f32 bits, button bitmask]`` pair."""
    move_x = struct.unpack("<f", struct.pack("<I", bits[0] & 0xFFFFFFFF))[0]
    buttons = {name: bool(bits[1] & bit) for name, bit in _BUTTON_BITS.items()}
    return move_x, buttons


def check_action_legal(bits: list[int]) -> None:
    """The routed action must be a finite stick in [-1, 1] with defined bits."""
    if len(bits) != 2 or not (0 <= bits[0] <= 0xFFFFFFFF) or not (0 <= bits[1] <= 127):
        raise SchemaError(f"malformed action {bits!r}")
    move_x, _ = unpack_action(bits)
    if not np.isfinite(move_x) or abs(move_x) > 1.0:
        raise SchemaError(f"illegal stick value {move_x!r}")


def policy_action(bits: list[int]) -> list[int]:
    """Discretize one recorded action to the six policy components.

    The exact bits stay in the dataset row; ``swap`` and ``pause`` are not
    policy components and are reported separately by the exporter.
    """
    move_x, buttons = unpack_action(bits)
    if move_x <= -STICK_THRESHOLD:
        horizontal = 0
    elif move_x >= STICK_THRESHOLD:
        horizontal = 2
    else:
        horizontal = 1
    return [
        horizontal,
        int(buttons["jump"]),
        int(buttons["attack"]),
        int(buttons["physical"]),
        int(buttons["ability"]),
        int(buttons["down"]),
    ]
