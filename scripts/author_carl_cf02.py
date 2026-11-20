#!/usr/bin/env python3
"""Authoring record for ART-CARL-CF02: Carl 52 px combat-acting sheet (13 frames).

Writes ``assets/source/characters/carl/carl_h52_cf02.grid.json`` (pixel-grid-v1, 48 x 56, pivot (25, 55)),
the editable source of the tags ``stomp``, ``air_kick``, ``dive_tuck``, ``dive_plunge``, ``dive_land``,
``bailout``, ``flinch`` and ``frustration`` defined in ``docs/02-art/cf02-acting-spec.md`` (section 3).
``scripts/export_assets.sh`` turns the grid into the ``.ase`` and the atlas.

How the frames are made (no resampling, no rotation, no image model):

* **Frozen keys.** The nine ACT-DESIGN keys in ``assets/source/characters/carl/cf02_keys/`` are copied pixel for
  pixel (``stomp_knee_up``, ``stomp_stamp``, ``dive_heavy_landing``, ``flinch``, ``frustration_shake``,
  ``air_kick_boot``, ``dive_tuck``, ``dive_plunge``, ``bailout_scramble``). The only change allowed is the
  airborne lift of spec section 2: the production sheet lifts airborne frames by 2 px (lowest row 53, nothing
  on rows 54-55). ``bailout_scramble`` is 55 px tall and would lose its crown row, so it is lifted 1 px (see the
  design question in the PR and ``AIRBORNE_LIFT``).
* **Derived frames** (spec section 3, "D"): ``stomp_recover``, ``air_kick_chamber``, ``bailout_pushoff`` and
  ``frustration_flap`` are drawn from the named key or approved frame. Their parts are the approved master's
  own pixels (``author_carl_melee`` helpers: translated upper block, stamps, rasterised limbs) or key pixels
  moved, never new heads.

The committed grid is the source of truth. Run ``python3 scripts/author_carl_cf02.py`` to rewrite it
(deterministic) or ``--check`` to verify that the committed grid equals the build output.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_melee as cm  # noqa: E402  (helpers and approved-master parts; read-only)

KEYS_DIR = ROOT / "assets/source/characters/carl/cf02_keys"
GROUND_KEYS = KEYS_DIR / "carl_h52_cf02_keys_ground.grid.json"
AIR_KEYS = KEYS_DIR / "carl_h52_cf02_keys_air.grid.json"
LOCO_PATH = cm.LOCO_PATH
OUT_PATH = ROOT / "assets/source/characters/carl/carl_h52_cf02.grid.json"
TICK_HZ = 60
LAYERS = cm.LAYERS
W, H = cm.W, cm.H


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


GROUND = _load(GROUND_KEYS)
AIR = _load(AIR_KEYS)
LOCO = _load(LOCO_PATH)
KEY = {}
for _g in (GROUND, AIR):
    for _t in _g["tags"]:
        KEY[_t["name"]] = cm.layers_from_cels(_g["frames"][_t["from"]]["cels"])
JSTART1 = cm.layers_from_cels(LOCO["frames"][11]["cels"])   # jump_start, take-off frame (lowest row 55)

# Airborne lift in rows (spec section 2). The spec asks for 2 px; bailout_scramble spans rows 1-55 in the key
# file, so a 2 px lift would clip its crown row. It is lifted 1 px instead (lowest row 54, still clear of the
# contact row 55) and the question goes to the top-tier review.
AIRBORNE_LIFT = {"air_kick_chamber": 2, "air_kick_boot": 2, "dive_tuck": 2, "dive_plunge": 2,
                 "bailout_pushoff": 2, "bailout_scramble": 1}


# ------------------------------------------------------------------ helpers

def merge(f: cm.Frame, layers: dict, names=LAYERS, dx: int = 0, dy: int = 0, x0=0, y0=0, x1=W, y1=H) -> None:
    for l in names:
        f.add(l, cm.rect(layers[l], x0, y0, x1, y1), dx, dy)


def from_key(name: str) -> cm.Frame:
    f = cm.Frame()
    merge(f, KEY[name])
    return f


# ------------------------------------------------------------------ derived frames

def stomp_recover() -> cm.Frame:
    """Q02 recovery: from ``stomp_stamp`` the upper block comes back up 3 px (U(1, 3)), head v3 one row lower, arms
    hang. The stamping leg pulls back off the target: both Crocs stand flat, back x15-24 and front x28-37 (at most
    2 px from the idle Crocs, so the handover to idle snaps at most 2 px). Reviewer ruling Q3, PR #146."""
    f = cm.upper(cm.Frame(), (1, 3), "v3", (0, 1))
    cm.idle_arms(f, (1, 3))
    cm.standing_legs(f, 42, 16, 26, 15, 28)
    return f


def air_kick_chamber() -> cm.Frame:
    """Q50 startup: upper block and arms of ``air_kick_boot``; the near knee pulled up with the Croc hanging
    (``dive_tuck`` near leg, moved by the upper-block offset), the far leg hanging straight down (not tucked)."""
    boot, tuck = KEY["air_kick_boot"], KEY["dive_tuck"]
    f = cm.Frame()
    # upper block, arms, head: the boot key (everything above the legs, plus its seat and hem rows)
    merge(f, boot, ("far_arm", "torso", "jacket", "head", "hair", "near_arm", "watch", "pin"))
    # boxers: seat and far hem of the boot key (x <= 26), near flap from the tuck (shifted by the upper offset)
    f.add("boxers", cm.rect(boot["boxers"], 0, 0, 26, H))
    f.add("boxers", cm.rect(tuck["boxers"], 28, 0, W, H), -1, -4)
    # far leg: boot key (hangs down-back), Croc included
    f.add("legs", cm.rect(boot["legs"], 0, 0, 27, H))
    f.add("shoes", cm.rect(boot["shoes"], 0, 0, 30, H))
    # near leg: knee up, Croc toe-down, from the tuck
    f.add("legs", cm.rect(tuck["legs"], 33, 0, W, H), -1, -4)
    f.add("shoes", cm.rect(tuck["shoes"], 32, 0, W, H), -1, -4)
    return f


def bailout_pushoff() -> cm.Frame:
    """Q54 take-off: ``jump_start`` take-off body thrown back 1 px, front Croc pushing, the yelling head already on
    that frame, and the ``dive_plunge`` arms (out sideways at shoulder height, open hands, the far arm 3 px higher)
    in place of the scramble arms, which read as cheering at 1x. Reviewer ruling Q5, PR #146."""
    pl = KEY["dive_plunge"]                 # key-file coordinates (before the lift)
    f = cm.Frame()
    # body and legs: jump_start, take-off frame; upper block (above the hems) 1 px back
    for l in ("torso", "jacket", "pin", "head", "hair"):
        f.add(l, JSTART1[l], -1, 0)
    f.add("boxers", cm.rect(JSTART1["boxers"], 0, 0, W, 37), -1, 0)
    f.add("boxers", cm.rect(JSTART1["boxers"], 0, 38, W, H))
    f.add("legs", JSTART1["legs"])
    f.add("shoes", JSTART1["shoes"])
    # the plunge upper block (pin (22, 21)) sits 2 rows below this one (pin (22, 19)): arms move up 2
    f.add("near_arm", pl["near_arm"], 0, -2)
    f.add("watch", pl["watch"], 0, -2)
    f.add("far_arm", pl["far_arm"], 0, -2)
    return f


def frustration_flap() -> cm.Frame:
    """Q70 frustration, second frame: ``frustration_shake`` with the hand 3 px lower and turned down at the wrist
    (fingers toward the floor), head 1 px lower."""
    s = KEY["frustration_shake"]
    f = cm.Frame()
    merge(f, s, ("far_arm", "legs", "shoes", "torso", "boxers", "jacket", "pin"))
    f.add("head", s["head"], 0, 1)
    f.add("hair", s["hair"], 0, 1)
    f.add("watch", s["watch"])
    arm = s["near_arm"]
    sleeve = cm.rect(arm, 0, 0, W, H)
    hand = {k: c for k, c in arm.items() if k[1] <= 23 and k[0] >= 29}   # fan hand, rows 19-23
    for k in hand:
        del sleeve[k]
    f.add("near_arm", sleeve)
    # turn the hand down: flip it vertically about its centre row and drop it 3 px
    ys = [y for _, y in hand]
    y0, y1 = min(ys), max(ys)
    flipped = {(x, y0 + y1 - y): c for (x, y), c in hand.items()}
    f.add("near_arm", flipped, 0, 3)
    # the lowered head and hand leave three orphans; repaint them by hand (no automatic cleanup on this frame, it
    # would turn the chest highlight into jacket sheen): chest skin back to M, two far-sleeve pixels to outline K
    f.L["torso"][(25, 21)] = "M"
    f.L["far_arm"][(30, 21)] = "K"
    f.L["far_arm"][(30, 23)] = "K"
    return f


DERIVED = {"stomp_recover": stomp_recover, "air_kick_chamber": air_kick_chamber,
           "bailout_pushoff": bailout_pushoff, "frustration_flap": frustration_flap}

# (frame name, key or derived, ticks of the preview duration, tag)
FRAMES = [
    ("stomp_knee_up", "K", 9, "stomp"),
    ("stomp_stamp", "K", 7, "stomp"),
    ("stomp_recover", "D", 10, "stomp"),
    ("air_kick_chamber", "D", 5, "air_kick"),
    ("air_kick_boot", "K", 10, "air_kick"),
    ("dive_tuck", "K", 6, "dive_tuck"),
    ("dive_plunge", "K", 30, "dive_plunge"),
    ("dive_heavy_landing", "K", 10, "dive_land"),
    ("bailout_pushoff", "D", 3, "bailout"),
    ("bailout_scramble", "K", 30, "bailout"),
    ("flinch", "K", 8, "flinch"),
    ("frustration_shake", "K", 6, "frustration"),
    ("frustration_flap", "D", 6, "frustration"),
]


def lift(cels: dict, n: int) -> dict:
    """Shift every layer up by ``n`` rows; the rows that leave the canvas must be empty."""
    out = {}
    for l, rows in cels.items():
        assert all(set(r) == {"."} for r in rows[:n]), f"lift would clip layer {l}"
        out[l] = rows[n:] + ["." * W] * n
    return out


def frame_cels(name: str, kind: str) -> dict:
    if kind == "K":
        gf = GROUND if name in {t["name"] for t in GROUND["tags"]} else AIR
        tag = next(t for t in gf["tags"] if t["name"] == name)
        cels = {l: list(rows) for l, rows in gf["frames"][tag["from"]]["cels"].items()}
    else:
        f = DERIVED[name]()
        if name != "frustration_flap":
            f = cm.cleanup(cm.occlude_pin(f))
        cels = f.cels()
    if name in AIRBORNE_LIFT:
        cels = lift(cels, AIRBORNE_LIFT[name])
    return cels


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ)


def build() -> dict:
    frames = [{"duration_ms": duration_ms(t), "cels": frame_cels(n, k)} for n, k, t, _ in FRAMES]
    tags = []
    for i, (_, _, _, tag) in enumerate(FRAMES):
        if tags and tags[-1]["name"] == tag:
            tags[-1]["to"] = i
        else:
            tags.append({"name": tag, "from": i, "to": i, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": [25, 55], "palette": cm.PALETTE,
            "layers": LAYERS, "frames": frames, "tags": tags}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=OUT_PATH, help="grid to write (default: the committed source)")
    parser.add_argument("--check", action="store_true", help="exit 1 if the committed grid differs from the build")
    args = parser.parse_args(argv)
    text = cm.dump(build())
    if args.check:
        same = args.out.is_file() and args.out.read_text(encoding="utf-8") == text
        print("carl cf02 grid matches the build" if same else "carl cf02 grid DIFFERS from the build")
        return 0 if same else 1
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({len(FRAMES)} frames)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
