#!/usr/bin/env python3
"""Authoring record for ANIM-CARL-03: Carl's dynamite throw (equipped special).

Writes two pixel-grid-v1 sources defined in ``docs/02-art/carl-dynamite-anim-03-spec.md``:

* ``assets/source/characters/carl/carl_h52_dynamite.grid.json``: 48 x 56, pivot (25, 55) (the locomotion
  contract). Tag ``dynamite_throw`` (9 standing frames) and six ``dynamite_move_*`` tags (the throw arms
  baked onto the six run frames, one tag per throw phase).
* ``assets/source/vfx/dynamite/vfx_dynamite.grid.json``: 24 x 24, pivot (12, 12). Tags ``dynamite_flight``
  (tumbling lit stick, loop) and ``dynamite_fizzle`` (the stick lying on the floor, fuse burning, loop).
  The explosion itself is the VFX-01 ``burst``, reused unchanged.

How the frames are made (no resampling, no rotation, no image model, no tracing):

* Carl's hair, head, torso, jacket, pin and boxer seat are the approved master's own pixels, translated only
  (``author_carl_melee.upper``); faces are the existing variants. Arms and legs are rasterised tubes with
  the master ramps (the ANIM-CARL-02 helpers); Crocs and fists are the existing stamps.
* The dynamite stick, its fuse and the sparks are literal pixel rows in this file (``STICK_*``, ``SPARK_*``).
  They sit on a new ``prop`` layer between ``hair`` and ``near_arm``, so the throwing hand covers its grip.
* Keys (L light, H hold, W wind-up, R release; VFX flight 0 and 3, fizzle 0) are frozen fixtures. The standing
  in-betweens (draw, hold flicker, follow-through, flinch, settle) and the stick in-betweens (flight down, flight
  left, fizzle flicker) are drawn here from the spec's in-between table. The moving tags are derived by rule
  (``move_frame``) from the standing frames and the run frames, so they follow the in-betweens automatically.

Run ``python3 scripts/author_carl_dynamite.py`` to rewrite both grids and both export manifests (deterministic),
``--check`` to verify them, ``--evidence`` to render the sheets, scenes, GIFs and the timing diagram in
``docs/evidence/anim-carl-03/`` (needs ``bash scripts/export_assets.sh`` first for the Floor 1 kit atlases),
``--ascii <frame id | atlas index | vfx>`` to dump frames as text.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_melee as cm  # noqa: E402  (helpers and approved-master parts; read-only)

CARL_OUT = ROOT / "assets/source/characters/carl/carl_h52_dynamite.grid.json"
VFX_OUT = ROOT / "assets/source/vfx/dynamite/vfx_dynamite.grid.json"
VFX01 = ROOT / "assets/source/vfx/explosion/vfx_01.grid.json"
EVIDENCE = ROOT / "docs/evidence/anim-carl-03"
TICK_HZ = 60
W, H = cm.W, cm.H

# ------------------------------------------------------------------ palette and layers
# Carl's 23 master colours unchanged, plus the dynamite ramp and the spark core. The spark halo reuses the
# pin yellow ``G`` (an accent) and the fuse cord reuses the hair-light brown ``h``.
DYN_COLOURS = [
    {"key": "n", "name": "dynamite_shade", "rgba": "#6e1622ff"},
    {"key": "m", "name": "dynamite_red", "rgba": "#c42a2eff"},
    {"key": "o", "name": "dynamite_light", "rgba": "#f06a4cff"},
    {"key": "y", "name": "spark_core", "rgba": "#fff4b8ff", "accent": True},
]
PALETTE = cm.PALETTE + DYN_COLOURS
LAYERS = cm.LAYERS[:8] + ["prop"] + cm.LAYERS[8:]   # ... hair, prop, near_arm, watch, pin
assert LAYERS[7] == "hair" and LAYERS[9] == "near_arm"


class Frame(cm.Frame):
    """ANIM-CARL-02 frame plus the ``prop`` layer (stick, fuse, spark)."""

    def __init__(self) -> None:
        super().__init__()
        self.L["prop"] = {}

    def cels(self) -> dict[str, list[str]]:
        return {l: cm.rows_of(self.L[l]) for l in LAYERS if self.L[l]}

    def flat(self) -> dict:
        out: dict = {}
        for l in LAYERS:
            out.update(self.L[l])
        return out


# ------------------------------------------------------------------ prop stamps (literal rows)
# Stick: K outline, o lit (upper-left), m body, n shade (lower-right). About 9 px long, 4 px thick at 1x:
# exaggerated against Carl's 52 px like every Floor 1 prop (the Bard bomb is 7 x 7).
STICK_V = cm.parse("""
.KK.
KomK
KomK
KmnK
KmnK
KmnK
KmnK
KmnK
.KK.
""")                       # fuse end up; grip at the bottom
STICK_D = cm.parse("""
....KK.
...KomK
..KomnK
.KomnK.
KomnK..
KmnK...
.KK....
""")                       # "/" fuse end up-right
STICK_B = cm.parse("""
.KK....
KomK...
KnmoK..
.KnmoK.
..KnmoK
...KnmK
....KK.
""")                       # "\" fuse end up-left
STICK_H = cm.parse("""
.KKKKKKK.
KoommmmmK
KmnnnnnnK
.KKKKKKK.
""")                       # fuse end right
# fuse cord (hair-light brown h) relative to each stamp's origin, last pixel = burning tip
FUSE = {"V": [(2, -1), (3, -2)], "D": [(5, -1), (6, -2)], "B": [(1, -1), (0, -2)], "H": [(9, 1), (10, 0)]}

# Sparks: y core (pale), G halo (Carl's pin yellow). Origin = top-left; the core touches the fuse tip.
SPARK_A = cm.parse("""
G.G.
.yyG
Gyy.
..G.
""")
SPARK_B = cm.parse("""
.G..
GyyG
.yy.
G..G
""")
SPARK_S = cm.parse("""
G.G
.y.
G.G
""")                       # compact spark for the release key (canvas edge)
FLARE = cm.parse("""
..G..
.GyG.
GyyyG
.GyG.
..G..
""")                       # the light beat: a short pop at the fuse

# Hands
HAND_OPEN_UP = cm.parse("""
.KKKKK.
KMMKMMK
KMMKMMK
KMMMMSK
.KMMSK.
..KKK..
""")                       # open, fingers up (the ANIM-CARL-02 kb hand)
HAND_SPREAD = cm.parse("""
.K.K.K.
KMKMKMK
KMMMMMK
KMMMMSK
.KMMSK.
..KKK..
""")                       # open, fingers spread (palm out: "whoa"), never a fist above the shoulder
HAND_PALM = cm.parse("""
..L.L.L.
.KLKLKLK
.KLMMMSK
MKLMMMSK
KMKMMSK.
.KKSSK..
...KK...
""")                       # gate fix: open palm, skin finger tips fanned with 1 px gaps, thumb out at lower left
HAND_CUP = cm.parse("""
.L.L.
KLKLK
KMMMK
KMMSK
.KSK.
""")                       # open hand cupped over the ear (t7_brace, execution phase); never a fist by the head
HAND_FLAT_R = cm.parse("""
.KKKKK.
KLMMMMK
KMMMMSK
.KSSKK.
..KK...
""")                       # open, fingers forward (a balance/aim hand, not a point)


def stick(f: Frame, kind: str, x: int, y: int, spark: dict | None, spark_at: tuple[int, int] | None) -> None:
    """Stick stamp at (x, y), its fuse and (optionally) a spark stamp with top-left ``spark_at``."""
    f.add("prop", {"V": STICK_V, "D": STICK_D, "B": STICK_B, "H": STICK_H}[kind], x, y)
    f.add("prop", {(x + dx, y + dy): "h" for dx, dy in FUSE[kind]})
    if spark is not None:
        f.add("prop", spark, *spark_at)


def finish(f: Frame) -> Frame:
    """ANIM-CARL-02 post-pass (pin occlusion, orphan merge of drawn limbs); master and prop pixels stay."""
    cm.occlude_pin(f)
    for _ in range(4):
        flat, top = {}, {}
        for l in LAYERS:
            for k, c in f.L[l].items():
                flat[k], top[k] = c, l
        changed = False
        for (x, y), c in sorted(flat.items()):
            if c in ACCENTS or top[(x, y)] == "prop" or (top[(x, y)], x, y) in f.locked:
                continue
            around = [flat[(x + dx, y + dy)] for dx, dy in cm.NEIGHBOURS if (x + dx, y + dy) in flat]
            if not around:
                del f.L[top[(x, y)]][(x, y)]
                changed = True
            elif c not in around:
                f.L[top[(x, y)]][(x, y)] = max(sorted(set(around)), key=around.count)
                changed = True
        if not changed:
            break
    return f


ACCENTS = {e["key"] for e in PALETTE if e.get("accent")}

# ------------------------------------------------------------------ Carl keys
# Coordinates: canvas pixels, x right, y down, floor row 55. U(dx, dy) = upper-block translation from idle 0.


def key_light() -> Frame:
    """L, the light beat: chin down, both hands at the chest; the near hand holds the stick upright, the far
    hand strikes at the fuse and it pops (FLARE). Plain standing feet: this is a chore, not a stance."""
    f = Frame()
    cm.upper(f, (1, 2), "v3", (1, 1))
    cm.standing_legs(f, 41, 17, 26, 15, 27)
    # near arm: from the back shoulder down past the ribs, forearm up to the chest
    cm.sleeve_arm(f, "near_arm", [(20, 22), (23, 31), (31, 29)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 31, 27)                 # grip x31-36, rows 27-30
    cm.watch_cuff(f, 29, 29, ((0, -1),))
    stick(f, "V", 32, 21, FLARE, (34, 14))                   # stick x32-35 rows 21-29, fuse to (35, 19)
    # far arm: elbow down in front, hand up beside the fuse (the lighter hand)
    cm.sleeve_arm(f, "far_arm", [(30, 23), (35, 28), (39, 22)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 37, 19)                  # x37-42, rows 19-22
    return finish(f)


def key_hold(spark: dict = None) -> Frame:
    """H, the tell: the lit stick held out at arm's length in front, upright, spark at face height; he leans
    back, head pulled away, the far hand up palm-out between his face and the spark. Brave enough to hold it,
    not happy about it."""
    f = Frame()
    cm.upper(f, (-2, 2), "v3", (-2, 0))                     # gate fix F3: lean back 2 px, head 2 px further
    cm.standing_legs(f, 41, 14, 23, 12, 30)
    # far arm (behind): upper arm forward to a low elbow out front, forearm angled up and back, wrist bent so the
    # open palm faces the spark, between his face and the spark at chin-to-brow height (gate fix F2)
    cm.sleeve_arm(f, "far_arm", [(27, 23), (35, 23), (33, 18)], 2.0)
    f.add("far_arm", HAND_PALM, 30, 11, lock=True)           # x30-37, rows 11-17 (thumb at x30); locked: the
    #                                                          finger separators survive the orphan merge
    # near arm: nearly straight, forward and a little down, as far from his face as it goes
    cm.sleeve_arm(f, "near_arm", [(17, 22), (28, 25), (37, 27)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 37, 25)                 # grip x37-42, rows 25-28
    cm.watch_cuff(f, 35, 27, ((0, -1),))
    stick(f, "V", 38, 17, spark or SPARK_A, (40, 11))        # stick x38-41 rows 17-25, spark x40-43 rows 11-14
    return finish(f)


def key_wind() -> Frame:
    """W, the wind-up: weight back, the near arm cocked high behind the head with the stick pointing up-back
    (spark above and behind the hair), the far hand flat forward for balance, front foot on its toe."""
    f = Frame()
    cm.upper(f, (-2, 1), "v3", (-1, 0))
    cm.hem(f, 15, 40, cm.HEM_BACK)
    cm.hem(f, 24, 40, cm.HEM_FRONT)
    f.add("legs", cm.leg((18, 41), (16, 49), 3.0, 2.0))      # back leg carries the weight
    f.add("legs", cm.leg_bent((28, 41), (32, 46), (33, 49), 3.0, 2.8, 2.0))
    cm.croc_flat(f, 12)
    cm.croc_toe_down(f, 31)
    # far arm: level forward, open hand
    cm.sleeve_arm(f, "far_arm", [(27, 21), (36, 21)], 2.0)
    f.add("far_arm", HAND_FLAT_R, 36, 19)                    # x36-42, rows 19-23
    # near arm: shoulder -> elbow back and up -> hand high behind the head
    cm.sleeve_arm(f, "near_arm", [(17, 22), (10, 19), (9, 14)], 2.0)
    stick(f, "B", 3, 6, SPARK_A, (1, 1))                     # stick x3-9 rows 6-12, fuse up-left
    f.add("near_arm", cm.FIST_SMALL, 6, 11)                  # grip x6-11, rows 11-14
    return finish(f)


def key_release() -> Frame:
    """R, the release: a lunge onto the front foot, the near arm whipped up-forward with the hand opening, the
    stick just off the fingers, fuse and spark leading; far fist yanked back to the hip; the yell (v2)."""
    f = Frame()
    cm.upper(f, (3, 2), "v2")
    cm.hem(f, 19, 41, cm.HEM_BACK)
    cm.hem(f, 28, 41, cm.HEM_FRONT)
    f.add("legs", cm.leg((22, 42), (18, 49), 3.0, 2.0))      # back leg trailing to its toe
    f.add("legs", cm.leg_bent((32, 42), (37, 46), (36, 49), 3.0, 2.8, 2.0))
    cm.croc_toe_down(f, 15)
    cm.croc_flat(f, 32)
    # far arm: swung back behind the hip
    cm.sleeve_arm(f, "far_arm", [(30, 23), (17, 31)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 11, 30)                  # x11-16, rows 30-33
    # near arm: up-forward
    cm.sleeve_arm(f, "near_arm", [(22, 22), (30, 19), (37, 16)], 2.0)
    f.add("near_arm", HAND_OPEN_UP, 36, 12)                  # x36-42, rows 12-17
    stick(f, "D", 38, 6, SPARK_S, (44, 2))                   # stick x38-44 rows 6-12, spark x44-46 rows 2-4
    return finish(f)


# ------------------------------------------------------------------ in-betweens (spec section 6, execution phase)
# Same construction as the keys: master upper block translated, rasterised limbs, stamps, literal prop rows.


def lifted_leg_set(f: Frame, u: tuple[int, int], back_foot: int, front_foot: int, back_lift: int = 0,
                   front_lift: int = 0) -> None:
    """Two flat-Croc standing legs under the boxer hems of upper block ``u``; a foot with ``lift`` > 0 is raised
    that many rows (its tube ends higher): a foot that moves between frames is visibly off the floor in the
    frame it moves, a planted foot keeps its exact pixels."""
    ux, uy = u
    hem_y, top = 39 + uy, 40 + uy
    cm.hem(f, 16 + ux, hem_y, cm.HEM_BACK)
    cm.hem(f, 25 + ux, hem_y, cm.HEM_FRONT)
    f.add("legs", cm.leg((19 + ux, top), (back_foot + 4, 49 - back_lift)))
    f.add("legs", cm.leg((29 + ux, top), (front_foot + 4, 49 - front_lift), 3.0, 2.0))
    f.add("shoes", cm.CROC_FLAT, back_foot, -back_lift)
    f.add("shoes", cm.CROC_FLAT, front_foot, -front_lift)


def between_draw() -> Frame:
    """t0, the draw: he fishes the unlit stick out of the back of his waistband. Looking down and back at the hip;
    the near hand grips the stick low behind him, its top 4 rows showing above the fist, the fuse not lit. Feet
    exactly as the light key (a chore, not a stance)."""
    f = Frame()
    cm.upper(f, (1, 1), "v3", (1, 1))
    lifted_leg_set(f, (1, 1), 15, 27)
    # near arm: shoulder -> hand low and back at the waistband
    cm.sleeve_arm(f, "near_arm", [(19, 23), (16, 29), (15, 34)], 2.0)
    stick_rows = {k: c for k, c in STICK_V.items() if k[1] <= 4}          # top 5 rows; the fist covers the rest
    f.add("prop", stick_rows, 12, 29)                                     # stick x12-15 rows 29-33 (+ grip under the fist)
    f.add("prop", {(14, 28): "h", (15, 27): "h"})                         # fuse, unlit (no spark)
    f.add("near_arm", cm.FIST_SMALL, 11, 33)                              # grip x11-16 rows 33-36
    cm.watch_cuff(f, 16, 31, ((0, -1),))
    # far arm: the idle far arm, moved with the upper block
    f.add("far_arm", cm.IDLE0["far_arm"], 1, 1)
    return finish(f)


def between_hold_b() -> Frame:
    """t3, the hold's second flicker: the hold key pixel for pixel, except the spark (SPARK_B, same top-left)."""
    return key_hold(SPARK_B)


FLAT_HAND_DOWN = {(x, 4 - y): c for (x, y), c in HAND_FLAT_R.items()}   # HAND_FLAT_R turned fingers-down


def between_follow() -> Frame:
    """t6, the follow-through: he over-commits, chin down; the near arm swung down and across with the hand open,
    fingers down, in front of the boxers; the far fist still back at the hip. The front Croc holds the release
    key's x32; the back foot comes off its toe and steps forward, off the floor (no foot slides). No prop pixels."""
    f = Frame()
    cm.upper(f, (5, 3), "v3", (1, 1))
    cm.hem(f, 21, 42, cm.HEM_BACK)
    cm.hem(f, 30, 42, cm.HEM_FRONT)
    f.add("legs", cm.leg((24, 43), (20, 47), 3.0, 2.0))
    f.add("legs", cm.leg_bent((34, 43), (38, 46), (36, 49), 3.0, 2.8, 2.0))
    cm.croc_toe_down(f, 17, 53)                              # back foot in the air, toe down, x17 (the key's x15 + 2)
    cm.croc_flat(f, 32)                                      # front foot planted exactly as the release key
    cm.sleeve_arm(f, "far_arm", [(32, 24), (18, 33)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 12, 32)                  # x12-17, rows 32-35
    cm.sleeve_arm(f, "near_arm", [(24, 23), (31, 27), (38, 30)], 2.0)
    f.add("near_arm", FLAT_HAND_DOWN, 36, 30, lock=True)     # open hand, fingers down, x36-42 rows 30-34
    cm.watch_cuff(f, 33, 28, ((0, -1),))
    return finish(f)


def between_brace() -> Frame:
    """t7, the flinch: shoulders up, chin tucked, the near hand OPEN and cupped over the near ear (never a fist),
    elbow out and down behind him; the far arm tense and straight down. The front Croc steps back (off the floor
    for the step), the back foot is down flat."""
    f = Frame()
    cm.upper(f, (1, 3), "v3", (0, 1))
    lifted_leg_set(f, (1, 3), 15, 29, 0, 2)
    cm.sleeve_arm(f, "near_arm", [(20, 23), (13, 25), (18, 20)], 2.0)
    f.add("near_arm", HAND_CUP, 18, 14, lock=True)           # open hand cupped over the ear, x18-22 rows 14-18
    cm.watch_cuff(f, 16, 21, ((0, -1),))
    f.add("far_arm", cm.IDLE0["far_arm"], 1, 3)
    return finish(f)


def between_settle() -> Frame:
    """t8, the settle: hands over to idle. Head v0 (a "huh"), idle arms moved with the upper block, feet exactly the
    light key's x15 / x27 (the idle stance)."""
    f = Frame()
    cm.upper(f, (1, 1), "v0")
    lifted_leg_set(f, (1, 1), 15, 27)
    cm.idle_arms(f, (1, 1))
    return finish(f)


# ------------------------------------------------------------------ standing plan (spec section 4)
# (frame id, ticks, "key" (frozen, fixture-checked) or "between" (drawn in the execution phase))
STANDING = [
    ("t0_draw", 6, "between"),
    ("t1_light", 10, "key"),
    ("t2_hold", 7, "key"),
    ("t3_hold_b", 7, "between"),
    ("t4_wind", 8, "key"),
    ("t5_release", 4, "key"),
    ("t6_follow", 8, "between"),
    ("t7_brace", 12, "between"),
    ("t8_settle", 8, "between"),
]
FRAMES = {"t0_draw": between_draw, "t1_light": key_light, "t2_hold": key_hold, "t3_hold_b": between_hold_b,
          "t4_wind": key_wind, "t5_release": key_release, "t6_follow": between_follow,
          "t7_brace": between_brace, "t8_settle": between_settle}
KEYS = {n: FRAMES[n] for n, _, kind in STANDING if kind == "key"}
KEY_INDEX = {name: i for i, (name, _, kind) in enumerate(STANDING) if kind == "key"}

# Moving variant: throw arms baked onto the six run frames (locomotion 4-9). Phases that keep the plain run
# arms (draw, brace, settle) have no moving frames: the runtime shows the run frame itself.
MOVE_PHASES = ["t1_light", "t2_hold", "t3_hold_b", "t4_wind", "t5_release", "t6_follow"]
RUN_FRAMES = [4, 5, 6, 7, 8, 9]
ARM_LAYERS = ("far_arm", "near_arm", "watch", "prop")
# Upper-block translation of each standing frame (U), used to move its arms onto a run frame's upper block.
# Keys: as drawn. In-betweens: the upper-block translation they are drawn with (spec section 6).
STANDING_U = {"t0_draw": (1, 1), "t1_light": (1, 2), "t2_hold": (-2, 2), "t3_hold_b": (-2, 2),
              "t4_wind": (-2, 1), "t5_release": (3, 2), "t6_follow": (5, 3), "t7_brace": (1, 3),
              "t8_settle": (1, 1)}
# The hold and wind-up phases lean back when standing; on the run body (U(+3, +2..3)) their forward hand or spark
# would reach column 47 (the clipping rule keeps it empty). Their arms sit further back on the shoulder instead
# (hold 2 px, wind-up 1 px); the arm root is drawn over the jacket, so the slide does not open a gap.
MOVE_TRIM = {"t2_hold": (-2, 0), "t3_hold_b": (-2, 0), "t4_wind": (-1, 0)}


def phase_u(phase: str) -> tuple[int, int]:
    return STANDING_U[phase]


def run_u(n: int) -> tuple[int, int]:
    """Upper-block translation of locomotion frame ``n`` (measured on the torso layer, which is translated only)."""
    torso = cm.layers_from_cels(cm.LOCO["frames"][n]["cels"])["torso"]
    for dy in range(-6, 8):
        for dx in range(-6, 7):
            if cm.shift(cm.IDLE0["torso"], dx, dy) == torso:
                return dx, dy
    raise ValueError(f"locomotion frame {n}: torso is not a translation of idle 0")


def standing_frames() -> dict[str, dict]:
    return {name: FRAMES[name]().L for name, _, _ in STANDING}


def move_frame(stand: dict, phase: str, n: int) -> dict:
    """Run frame ``n`` with its arms replaced by the standing phase's arms and prop, moved by U(run) - U(phase).
    Body, legs, head and hair stay the run frame's; the pin is hidden when the near sleeve covers it."""
    run = cm.layers_from_cels(cm.LOCO["frames"][n]["cels"])
    (rx, ry), (sx, sy) = run_u(n), phase_u(phase)
    tx, ty = MOVE_TRIM.get(phase, (0, 0))
    out = {l: dict(run.get(l, {})) for l in LAYERS}
    for l in ARM_LAYERS:
        out[l] = cm.shift(stand[l], rx - sx + tx, ry - sy + ty)
    if len([k for k in out["pin"] if k in out["near_arm"]]) >= 3:
        out["pin"] = {}
    return out


# ------------------------------------------------------------------ VFX (24 x 24, pivot (12, 12))
VFX_W = VFX_H = 24
VFX_KEYS_FROM_CARL = ("K", "h", "G", "n", "m", "o", "y")
VFX_PALETTE = [e for e in PALETTE if e["key"] in VFX_KEYS_FROM_CARL]


def vfx_flight(kind: str, spark: dict) -> dict:
    """Stick centred on the pivot (12, 12), lit fuse and spark. No motion streaks: the arc and the spin carry
    the motion, and streaks would hide a 9 px stick."""
    f = Frame()
    stamp = {"V": STICK_V, "D": STICK_D, "B": STICK_B, "H": STICK_H}[kind]
    xs = [x for x, _ in stamp]
    ys = [y for _, y in stamp]
    ox = 12 - (min(xs) + max(xs) + 1) // 2
    oy = 12 - (min(ys) + max(ys) + 1) // 2
    tip = FUSE[kind][-1]
    sx, sy = {"V": (1, -3), "D": (0, -3), "B": (-3, -3), "H": (0, -3)}[kind]
    stick(f, kind, ox, oy, spark, (ox + tip[0] + sx, oy + tip[1] + sy))
    return {"body": f.L["prop"]}


def vfx_fizzle(spark: dict) -> dict:
    """The stick lying on the floor: lowest row = pivot row 12 (the walking line), fuse burning up-right."""
    f = Frame()
    stick(f, "H", 7, 9, spark, (17, 6))
    return {"body": f.L["prop"]}


def vfx_flip(kind: str, spark: dict, spark_origin_from_tip: tuple[int, int], flip: str) -> dict:
    """In-between of the tumble: the stick stamp mirrored (``v`` rows, ``h`` columns) and centred on the pivot like
    the keys; fuse and spark mirrored with it. ``spark_origin_from_tip``: top-left of the spark stamp relative to the
    burning fuse tip (the core touches the tip, the halo points away from the stick)."""
    stamp = {"V": STICK_V, "H": STICK_H}[kind]
    w, h = max(x for x, _ in stamp) + 1, max(y for _, y in stamp) + 1
    mirror = (lambda x, y: (x, h - 1 - y)) if flip == "v" else (lambda x, y: (w - 1 - x, y))
    body = {mirror(*k): c for k, c in stamp.items()}
    fuse = [mirror(*p) for p in FUSE[kind]]
    ox, oy = 12 - (0 + w) // 2, 12 - (0 + h) // 2
    f = Frame()
    f.add("prop", body, ox, oy)
    f.add("prop", {(ox + x, oy + y): "h" for x, y in fuse})
    tx, ty = ox + fuse[-1][0], oy + fuse[-1][1]
    f.add("prop", spark, tx + spark_origin_from_tip[0], ty + spark_origin_from_tip[1])
    return {"body": f.L["prop"]}


SPARK_B_DOWN = {(x, 3 - y): c for (x, y), c in SPARK_B.items()}      # SPARK_B turned over: halo below the core


VFX_PLAN = [  # (tag, [(frame id, ticks, "key" or "between")])
    # A forward (clockwise) tumble in 90 degree steps; the release key's stick points up-right, so the loop
    # starts at f0 (fuse forward): right, down, left, up. The stick body is symmetric: the fuse and spark carry
    # the spin; the lit row stays on top in every frame (the light does not turn with the stick).
    ("dynamite_flight", [("f0_right", 3, "key"), ("f1_down", 3, "between"), ("f2_left", 3, "between"),
                         ("f3_up", 3, "key")]),
    ("dynamite_fizzle", [("z0", 4, "key"), ("z1", 4, "between")]),
]
VFX_FRAMES = {"f0_right": lambda: vfx_flight("H", SPARK_A),
              "f1_down": lambda: vfx_flip("V", SPARK_B_DOWN, (-1, 0), "v"),
              "f2_left": lambda: vfx_flip("H", SPARK_A, (-3, -3), "h"),
              "f3_up": lambda: vfx_flight("V", SPARK_B),
              "z0": lambda: vfx_fizzle(SPARK_A), "z1": lambda: vfx_fizzle(SPARK_B)}
VFX_KEYS = {n: VFX_FRAMES[n] for n in ("f0_right", "f3_up", "z0")}


# ------------------------------------------------------------------ grids


def rows(pix: dict, w: int, h: int) -> list[str]:
    g = [["."] * w for _ in range(h)]
    for (x, y), c in pix.items():
        if 0 <= x < w and 0 <= y < h:
            g[y][x] = c
    return ["".join(r) for r in g]


def carl_grid() -> dict:
    stand = standing_frames()
    frames, tags = [], []
    for name, ticks, _ in STANDING:
        frames.append({"duration_ms": round(ticks * 1000 / TICK_HZ),
                       "cels": {l: rows(stand[name][l], W, H) for l in LAYERS if stand[name][l]}})
    tags.append({"name": "dynamite_throw", "from": 0, "to": len(STANDING) - 1, "direction": "forward"})
    loco_ms = {n: cm.LOCO["frames"][n]["duration_ms"] for n in RUN_FRAMES}
    for phase in MOVE_PHASES:
        start = len(frames)
        for n in RUN_FRAMES:
            mf = move_frame(stand[phase], phase, n)
            frames.append({"duration_ms": round(round(loco_ms[n] * TICK_HZ / 1000) * 1000 / TICK_HZ),
                           "cels": {l: rows(mf[l], W, H) for l in LAYERS if mf[l]}})
        tags.append({"name": "dynamite_move_" + phase.split("_", 1)[1], "from": start, "to": len(frames) - 1,
                     "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": [25, 55], "palette": PALETTE,
            "layers": LAYERS, "frames": frames, "tags": tags}


def vfx_grid() -> dict:
    built = {k: fn() for k, fn in VFX_FRAMES.items()}
    frames, tags = [], []
    for tag, plan in VFX_PLAN:
        start = len(frames)
        for name, ticks, src in plan:
            frames.append({"duration_ms": round(ticks * 1000 / TICK_HZ),
                           "cels": {"body": rows(built[name]["body"], VFX_W, VFX_H)}})
        tags.append({"name": tag, "from": start, "to": len(frames) - 1, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [VFX_W, VFX_H], "pivot": [12, 12], "palette": VFX_PALETTE,
            "layers": ["body"], "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    return cm.dump(grid)


def outputs() -> dict[Path, str]:
    return {CARL_OUT: dump(carl_grid()), VFX_OUT: dump(vfx_grid()),
            CARL_MANIFEST: mdump(carl_manifest()), VFX_MANIFEST: mdump(vfx_manifest())}


# ------------------------------------------------------------------ anchors (pivot-relative, x forward, y up)


def prop_anchors(grid: dict) -> dict[str, dict]:
    """Per standing frame: the stick's bounding-box centre and the burning fuse tip (spark centre), from pixels."""
    piv_x, piv_y = grid["pivot"]
    out = {}
    for i, (name, _, _) in enumerate(STANDING):
        cel = grid["frames"][i]["cels"].get("prop")
        if not cel:
            continue
        pix = {(x, y): c for y, r in enumerate(cel) for x, c in enumerate(r) if c != "."}
        body = [p for p, c in pix.items() if c in "Knmo"]
        spark = [p for p, c in pix.items() if c in "yG"]
        cx = (min(x for x, _ in body) + max(x for x, _ in body)) // 2
        cy = (min(y for _, y in body) + max(y for _, y in body)) // 2
        out[str(i)] = {"stick_centre": [cx - piv_x, piv_y - cy]}
        if spark:
            core = [p for p, c in pix.items() if c == "y"]
            sx = round(sum(x for x, _ in core) / len(core))
            sy = round(sum(y for _, y in core) / len(core))
            out[str(i)]["fuse_spark"] = [sx - piv_x, piv_y - sy]
    return out


# ------------------------------------------------------------------ evidence


def frame_anchors(grid: dict, i: int) -> dict | None:
    """Stick centre (bounding box of the stick body) and burning fuse tip (spark core centre) of atlas frame ``i``,
    pivot-relative (x forward, y up); None when the frame has no prop pixels."""
    cel = grid["frames"][i]["cels"].get("prop")
    if not cel:
        return None
    piv_x, piv_y = grid["pivot"]
    pix = {(x, y): c for y, r in enumerate(cel) for x, c in enumerate(r) if c != "."}
    body = [p for p, c in pix.items() if c in "Knmo"]
    out = {"stick_centre": [(min(x for x, _ in body) + max(x for x, _ in body)) // 2 - piv_x,
                            piv_y - (min(y for _, y in body) + max(y for _, y in body)) // 2]}
    core = [p for p, c in pix.items() if c == "y"]
    if core:
        out["fuse_spark"] = [round(sum(x for x, _ in core) / len(core)) - piv_x,
                             piv_y - round(sum(y for _, y in core) / len(core))]
    return out


# ------------------------------------------------------------------ manifests
CARL_MANIFEST = ROOT / "assets/manifests/carl_book1_h52_dynamite.json"
VFX_MANIFEST = ROOT / "assets/manifests/vfx_dynamite.json"
LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
           "no game/asset-pack distribution rights")
PHASE = {"t0_draw": "startup", "t1_light": "startup", "t2_hold": "startup", "t3_hold_b": "startup",
         "t4_wind": "startup", "t5_release": "release", "t6_follow": "recovery", "t7_brace": "recovery",
         "t8_settle": "recovery"}
SPAWN_TICK = 41
FUSE_LIT_TICK = 6
SPAWN_OFFSET = (16, -46)          # standing release: stick centre from the pivot (x forward, y down)


def standing_ticks() -> list[tuple[str, int, int, int]]:
    """(frame id, first tick, last tick, atlas frame index) of the standing throw."""
    out, t = [], 0
    for i, (name, ticks, _) in enumerate(STANDING):
        out.append((name, t, t + ticks - 1, i))
        t += ticks
    return out


def carl_manifest() -> dict:
    grid = carl_grid()
    names = [n for n, _, _ in STANDING]
    kinds = {n: k for n, _, k in STANDING}
    move_tags = ["dynamite_move_" + ph.split("_", 1)[1] for ph in MOVE_PHASES]
    rel = {t["name"]: t for t in grid["tags"]}["dynamite_move_release"]
    stand_rel = frame_anchors(grid, names.index("t5_release"))["stick_centre"]
    moving_spawn = []
    for j, n in enumerate(RUN_FRAMES):
        a = frame_anchors(grid, rel["from"] + j)["stick_centre"]
        moving_spawn.append({"run_frame": n, "spawn_offset_px": [a[0], -a[1]],
                             "delta_from_standing_px": [a[0] - stand_rel[0], -(a[1] - stand_rel[1])]})
    anchors = {}
    for name, _, _, i in standing_ticks():
        a = frame_anchors(grid, i)
        if a:
            anchors[name] = a
    return {
        "schema_version": 1,
        "id": "carl_book1_h52_dynamite_candidate_v1",
        "type": "character_animation",
        "source_path": "assets/source/characters/carl/carl_h52_dynamite.ase",
        "grid_source": "assets/source/characters/carl/carl_h52_dynamite.grid.json",
        "output_atlas": "generated/art/carl_book1_h52_dynamite.png",
        "output_metadata": "generated/art/carl_book1_h52_dynamite.json",
        "palette_id": "carl-book1-dynamite-v0",
        "frame_size": [W, H],
        "pivot": [25, 55],
        "animation_tags": ["dynamite_throw"] + move_tags,
        "required_alpha": True,
        "owner": "AI-authored pixel animation candidate (fan-concept layer: Carl -> demolitionist)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("ANIM-CARL-03 Carl dynamite throw derived from the owner-approved 52 px master "
                       "assets/source/characters/carl/carl_h52.grid.json (via the approved ANIM-CARL-01 48 x 56 canvas): "
                       "head, hair, torso, jacket, boxer seat and pin are the master's own pixels, translated only; arms, "
                       "legs and hands are re-pixelled per pose in pixel-grid-v1; the stick, fuse and sparks are literal "
                       "pixel rows on a new prop layer (scripts/author_carl_dynamite.py is the deterministic authoring "
                       "record); the 36 moving frames are baked by rule from the run frames and the standing arms"),
            "generator": ("Spec and 4 key poses: Claude Opus (hand-placed pixel rules). 5 standing in-betweens, moving "
                          "variant, export, tests and evidence: Claude Sonnet, composed by scripts/author_carl_dynamite.py "
                          "from master pixels, key stamps and rasterised limbs; no image-model raster output, no "
                          "resampling, no rotation of master pixels"),
            "generated_art": True,
            "input_reference_ids": ["CARL-CONCEPT-APPROVED-V1"],
        },
        "license": LICENSE,
        "review_status": "prototype_fixture_unapproved",
        "dynamite": {
            "status": "keys frozen, 5 standing in-betweens drawn, moving variant baked, independent review pending; 45 frames, 4 of them frozen keys; not wired into any Bevy binary",
            "spec": "docs/02-art/carl-dynamite-anim-03-spec.md",
            "master": "assets/manifests/carl_book1_h52.json",
            "locomotion": "assets/manifests/carl_book1_h52_locomotion.json",
            "projectile": "assets/manifests/vfx_dynamite.json",
            "blast": "assets/manifests/vfx-01.json tag burst (reused unchanged)",
            "tick_hz": TICK_HZ,
            "grounded_tags": ["dynamite_throw"] + move_tags,
            "airborne_tags": [],
            "playback": {"dynamite_throw": "once", **{t: "loop" for t in move_tags}},
            "root_motion": "none (the throw is rooted standing; the moving variant follows the locomotion clock)",
            "key_fixture": "tests/fixtures/anim-carl-03_keys.json",
            "key_frames": [i for i, n in enumerate(names) if kinds[n] == "key"],
            "inbetween_frames": [i for i, n in enumerate(names) if kinds[n] == "between"],
            "total_ticks": sum(t for _, t, _ in STANDING),
            "fuse_lit_tick": FUSE_LIT_TICK,
            "spark_visible_in_hand_ticks": [FUSE_LIT_TICK, SPAWN_TICK],
            "projectile_spawn": {
                "tick": SPAWN_TICK,
                "note": "last tick of t5_release; the runtime spawns the dynamite_flight entity at the stick centre of the release frame",
                "standing_offset_px": [stand_rel[0], -stand_rel[1]],
                "offset_convention": "from Carl's pivot, x forward (mirror when facing left), y down (screen)",
                "moving_offsets": moving_spawn,
            },
            "prop_anchors_px": {
                "note": "pivot-relative, x forward, y up; stick_centre is the bounding-box centre of the stick body (on t0_draw only the part above the fist is visible); fuse_spark is the spark core centre",
                **anchors,
            },
            "moves": {
                "dynamite_throw": {
                    "role": "Circle/B equipped special: draw, light, hold (the tell), wind-up, lob, flinch, settle",
                    "total_ticks": sum(t for _, t, _ in STANDING),
                    "spawn_tick": SPAWN_TICK,
                    "frames": [{"index": i, "name": n, "ticks": [a, b], "duration_ticks": b - a + 1,
                                "phase": PHASE[n], "key": kinds[n] == "key"} for n, a, b, i in standing_ticks()],
                },
            },
            "moving_variant": {
                "note": "the run frame's body with the standing phase's arms and prop baked on; the runtime draws dynamite_move_<phase> frame (run frame - 4) while a throw phase is active and the locomotion state is run; draw, brace and settle show the plain run frame",
                "run_frames": RUN_FRAMES,
                "phases": MOVE_PHASES,
                "trim_px": {k: list(v) for k, v in MOVE_TRIM.items()},
            },
        },
    }


def vfx_manifest() -> dict:
    kinds = {n: k for _, plan in VFX_PLAN for n, _, k in plan}
    frames, i = [], 0
    for tag, plan in VFX_PLAN:
        for name, ticks, kind in plan:
            frames.append({"index": i, "name": name, "tag": tag, "ticks": ticks,
                           "duration_ms": round(ticks * 1000 / TICK_HZ),
                           "author": "key" if kind == "key" else "inbetween"})
            i += 1
    return {
        "schema_version": 1,
        "id": "vfx_dynamite_24_candidate_v1",
        "type": "vfx_animation",
        "source_path": "assets/source/vfx/dynamite/vfx_dynamite.ase",
        "grid_source": "assets/source/vfx/dynamite/vfx_dynamite.grid.json",
        "output_atlas": "generated/art/vfx_dynamite.png",
        "output_metadata": "generated/art/vfx_dynamite.json",
        "palette_id": "vfx-dynamite-v0",
        "frame_size": [VFX_W, VFX_H],
        "pivot": [12, 12],
        "animation_tags": ["dynamite_flight", "dynamite_fizzle"],
        "required_alpha": True,
        "owner": "AI-authored VFX candidate (fan-concept layer: Carl's dynamite; runtime role IP-neutral thrown explosive)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "ANIM-CARL-03 stick frames rasterized natively from literal pixel rows in scripts/author_carl_dynamite.py (the same stamps as the Carl prop layer)",
            "generator": "spec + keys: Claude Opus; in-betweens + export: Claude Sonnet; review pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": [],
        },
        "license": LICENSE,
        "review_status": "prototype_fixture_unapproved",
        "vfx": {
            "status": "Keys frozen; in-betweens drawn; independent review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/carl-dynamite-anim-03-spec.md",
            "tick_hz": TICK_HZ,
            "free_tags": ["dynamite_flight", "dynamite_fizzle"],
            "playback": {"dynamite_flight": "loop", "dynamite_fizzle": "loop"},
            "facing": "right",
            "spawn": {"tick": SPAWN_TICK, "source": "assets/manifests/carl_book1_h52_dynamite.json dynamite.projectile_spawn",
                      "pivot_on_stick_centre": True},
            "flight": {"order": ["f0_right", "f1_down", "f2_left", "f3_up"], "ticks_per_frame": 3, "ticks_per_turn": 12,
                       "spin": "clockwise (forward tumble)", "streaks": False},
            "fizzle": {"note": "the stick lying on the floor, lowest row on the pivot row (the walking line), fuse burning", "pivot_row_is_walking_line": True},
            "blast": {"reuse": "assets/manifests/vfx-01.json", "tag": "burst", "centre_above_walking_line_px": 8,
                      "note": "reused unchanged; a bigger blast is a separate natively drawn burst_large (VFX-01 section 8)"},
            "placeholders_for_evidence_only": {"vx_px_s": 150, "vy_px_s": -260, "gravity_px_s2": 1050,
                                               "blast_ticks_after_light": 90},
            "key_fixture": "tests/fixtures/anim-carl-03-vfx_keys.json",
            "key_frames": [i for i, f in enumerate(frames) if f["author"] == "key"],
            "inbetween_frames": [i for i, f in enumerate(frames) if f["author"] == "inbetween"],
            "frames": frames,
        },
    }


def mdump(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def evidence() -> None:
    from PIL import Image, ImageDraw
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import BG, CYAN, GREY, WHITE, YELLOW, cell, grid_images, label
    from scripts.pixelart.scene import Floor

    carl_g = json.loads(CARL_OUT.read_text(encoding="utf-8"))
    vfx_g = json.loads(VFX_OUT.read_text(encoding="utf-8"))
    carl, vfx = grid_images(carl_g), grid_images(vfx_g)
    boom = grid_images(json.loads(VFX01.read_text(encoding="utf-8")))
    loco = grid_images(cm.LOCO)
    donut_g = json.loads((ROOT / "assets/source/characters/donut/donut_h40.grid.json").read_text(encoding="utf-8"))
    gob_g = json.loads((ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json").read_text(encoding="utf-8"))
    donut, gob = grid_images(donut_g)[0], grid_images(gob_g)[0]
    gob_flip = gob.transpose(Image.FLIP_LEFT_RIGHT)
    gpiv = tuple(gob_g["pivot"])
    gpiv_flip = (gob.width - 1 - gpiv[0], gpiv[1])
    cpiv, dpiv, vpiv = (25, 55), tuple(donut_g["pivot"]), (12, 12)
    floor = Floor()
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    k = {name: i for i, (name, _, _) in enumerate(STANDING)}

    def sheet(entries, name, x0, width, lit=True, k3=False):
        full = floor.render(entries, 1, lit)
        vx, vy = floor.view[0], floor.view[1]
        gy = floor.ground(452) - vy
        box = (max(0, x0 - vx), max(0, gy - 74), min(full.width, x0 - vx + width), min(full.height, gy + 14))
        one = full.crop(box)
        one.save(EVIDENCE / f"{name}.png")
        if k3:
            one.resize((one.width * 4, one.height * 4), Image.NEAREST).save(EVIDENCE / f"{name}-x4.png")
        return one

    # 1. Key silhouettes at 1x on lit Floor 1: Donut and idle Carl for scale, the four keys, a goblin target,
    #    the stick in flight, fizzling on the floor, and the VFX-01 burst peak behind the goblin.
    xs = [404, 452, 500, 548, 596, 650]
    entries = [(boom[5], (24, 24), 812, 8, False)]         # burst peak, centre 8 px above the walking line
    entries += [(donut, dpiv, xs[0], 0), (loco[0], cpiv, xs[1], 0)]
    entries += [(carl[k[n]], cpiv, x, 0) for n, x in zip(("t1_light", "t2_hold", "t4_wind", "t5_release"), xs[2:])]
    entries += [(vfx[0], vpiv, 700, 34, False), (vfx[3], vpiv, 716, 46, False), (vfx[4], vpiv, 742, 0, False)]
    entries += [(vfx[1], vpiv, 470, 66, False), (vfx[2], vpiv, 560, 64, False)]   # flight over the lit brick
    entries += [(gob_flip, gpiv_flip, 786, 0), (gob_flip, gpiv_flip, 842, 0)]
    sheet(entries, "keys-scale", 370, 500, k3=True)
    sheet(entries, "keys-scale-unlit", 370, 500, lit=False)

    # 2. Silhouette sheet: the keys as flat black shapes at 1x and 4x next to Donut, idle Carl and a goblin,
    #    plus the haymaker wind-up (ANIM-CARL-02 b0) for the "not a punch" check.
    melee = grid_images(json.loads((ROOT / "assets/source/characters/carl/carl_h52_melee.grid.json").read_text()))
    sil = [("Donut idle", donut, dpiv), ("Carl idle", loco[0], cpiv), ("L light", carl[k["t1_light"]], cpiv),
           ("H hold (tell)", carl[k["t2_hold"]], cpiv), ("W wind-up", carl[k["t4_wind"]], cpiv),
           ("R release", carl[k["t5_release"]], cpiv), ("melee b0 (compare)", melee[4], cpiv),
           ("goblin A", gob_flip, gpiv_flip)]
    S, PAD, CW = 4, 12, 50
    out = Image.new("RGB", (PAD * 2 + len(sil) * (CW * S + PAD), 60 * S + 120), BG)
    d = ImageDraw.Draw(out)
    label(d, (PAD, 6), "ANIM-CARL-03 silhouette check: keys as solid shapes (top 1x, below 4x); prop pixels in red. "
          "Feet on one line; frames face right.", WHITE, 12)
    base1, base4 = 80, 60 * S + 80
    for i, (name, img, piv) in enumerate(sil):
        x = PAD + i * (CW * S + PAD)
        solid = Image.new("RGBA", img.size, (0, 0, 0, 0))
        a = img.split()[3]
        solid.paste((214, 208, 224, 255), mask=a)
        if img.size == (48, 56) and name[0] in "LHWR":
            g = carl_g["frames"][k[{"L": "t1_light", "H": "t2_hold", "W": "t4_wind", "R": "t5_release"}[name[0]]]]
            for yy, r in enumerate(g["cels"].get("prop", [])):
                for xx, c in enumerate(r):
                    if c != ".":
                        solid.putpixel((xx, yy), (200, 40, 40, 255))
        out.paste(solid, (x + 20 - piv[0] + 25, base1 - piv[1]), solid)
        big = solid.resize((img.width * S, img.height * S), Image.NEAREST)
        out.paste(big, (x + (CW * S - img.width * S) // 2, base4 - (piv[1] + 1) * S), big)
        label(d, (x, base4 + 8), name, YELLOW if name[0] in "LHWR" and img.size == (48, 56) else GREY, 12)
    d.line([PAD, base4, out.width - PAD, base4], fill=CYAN)
    out.save(EVIDENCE / "silhouettes.png")

    # 3. Keys at 4x in colour on the cell background, with pivot and floor marks.
    cells = [cell(carl[k[n]], 4, cpiv, 55) for n in ("t1_light", "t2_hold", "t4_wind", "t5_release")]
    cells += [cell(vfx[i], 4, vpiv, None) for i in (0, 3, 4)]
    wsum = sum(c.width for c in cells) + PAD * (len(cells) + 1)
    out = Image.new("RGB", (wsum, max(c.height for c in cells) + 50), BG)
    d = ImageDraw.Draw(out)
    x = PAD
    for c, name in zip(cells, ("L light (t6)", "H hold / tell", "W wind-up", "R release (spawn)",
                               "VFX f0 flight", "VFX f3 flight", "VFX z0 fizzle")):
        out.paste(c, (x, 30))
        label(d, (x, 10), name, YELLOW, 12)
        x += c.width + PAD
    out.save(EVIDENCE / "keys-x4.png")

    # 4. Moving variant: each overlay phase baked onto the six run frames, at 1x on lit Floor 1 and 4x.
    for phase in ("light", "hold", "wind", "release"):
        t = [t for t in carl_g["tags"] if t["name"] == "dynamite_move_" + phase][0]
        entries = [(donut, dpiv, 404, 0), (loco[0], cpiv, 452, 0)]
        entries += [(carl[t["from"] + j], cpiv, 500 + 48 * j, 0) for j in range(6)]
        sheet(entries, f"move-{phase}-scale", 370, 440, k3=True)
    scenes(carl_g, vfx_g, carl, vfx, boom, loco, donut, gob_flip, gpiv_flip, floor)
    timing_diagram(carl_g, carl)
    import hashlib
    sums = sorted(f"{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}" for f in EVIDENCE.iterdir()
                  if f.is_file() and f.suffix in (".png", ".gif"))
    (EVIDENCE / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")
    print(f"wrote evidence in {EVIDENCE.relative_to(ROOT)}")


# ------------------------------------------------------------------ in-scene evidence (Pillow models, not Bevy captures)
# Placeholder numbers from spec section 9 (evidence only, never gameplay): lob vx 150 px/s, vy 260 px/s up,
# gravity 1050 px/s^2, blast 90 ticks after the fuse is lit.
LOB_VX, LOB_VY, GRAVITY, BLAST_AFTER_LIGHT = 150.0, 260.0, 1050.0, 90
RUN_PX_PER_TICK = 185.0 / TICK_HZ                 # the greybox RUN_SPEED (src/anim.rs), so the run frames do not skate


def throw_timeline(carl_x0: float, moving: bool, last_tick: int) -> list[dict]:
    """Per tick: Carl's atlas frame and x, the stick's state (``hand``/``flight``/``fizzle``/``gone``) and position
    (x, lift above the walking line), the burst frame, whether the goblins still stand."""
    names = [n for n, _, _ in STANDING]
    bounds = standing_ticks()
    tag = {t["name"]: t for t in carl_grid()["tags"]}
    out, x = [], carl_x0
    stick = None
    blast_tick = FUSE_LIT_TICK + BLAST_AFTER_LIGHT
    burst_ticks = [2, 3, 4, 4, 5, 6, 8, 6, 4]
    burst_at = {}
    t0 = blast_tick
    for i, n in enumerate(burst_ticks):
        for k in range(n):
            burst_at[t0 + k] = i
        t0 += n
    land = None
    for t in range(last_tick + 1):
        in_throw = t < 70
        phase = next((nm for nm, a, b, _ in bounds if a <= t <= b), None) if in_throw else None
        run_j = (t // 4) % 6
        if in_throw and moving:
            x += RUN_PX_PER_TICK
        if not in_throw:
            frame = ("loco", 0)                          # after the settle: the locomotion idle frame
        elif moving and phase in MOVE_PHASES:
            frame = tag["dynamite_move_" + phase.split("_", 1)[1]]["from"] + run_j
        elif moving:
            frame = ("loco", 4 + run_j)                  # draw, brace, settle: the plain run frame
        else:
            frame = names.index(phase)
        if t == SPAWN_TICK:
            if moving:
                sj = frame - tag["dynamite_move_release"]["from"]
                off = ad_offsets()[sj]
            else:
                off = SPAWN_OFFSET
            stick = {"x": x + off[0], "h": -off[1], "vy": LOB_VY, "born": t}
        if stick and t > SPAWN_TICK and stick["h"] > 0 and land is None:
            stick["vy"] -= GRAVITY / TICK_HZ
            stick["x"] += (LOB_VX + (RUN_PX_PER_TICK * TICK_HZ if moving else 0.0)) / TICK_HZ   # placeholder: inherits the run speed
            stick["h"] += stick["vy"] / TICK_HZ
            if stick["h"] <= 0:
                stick["h"], land = 0.0, t
        if t == SPAWN_TICK + 1 or (t > SPAWN_TICK and stick and "flight_x0" not in stick):
            stick["flight_x0"] = stick["x"]
        state = "hand" if (t < SPAWN_TICK and FUSE_LIT_TICK <= t) else "none"
        if stick and t >= SPAWN_TICK:
            state = "flight" if land is None else "fizzle"
        if t >= blast_tick:
            state = "gone"
        out.append({"t": t, "frame": frame, "x": round(x), "stick": dict(stick) if stick else None, "state": state,
                    "phase": phase or "idle", "burst": burst_at.get(t), "goblins": not (burst_at.get(t, -1) >= 3),
                    "lands": land})
    return out


def ad_offsets() -> list[tuple[int, int]]:
    g = carl_grid()
    rel = {t["name"]: t for t in g["tags"]}["dynamite_move_release"]
    return [(a[0], -a[1]) for a in (frame_anchors(g, rel["from"] + j)["stick_centre"] for j in range(6))]


def scenes(carl_g, vfx_g, carl, vfx, boom, loco, donut, gob_flip, gpiv_flip, floor) -> None:
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import BG, CYAN, GREY, WHITE, YELLOW, gif_delays, label, save_gif
    cpiv, dpiv, vpiv, bpiv = (25, 55), (22, 43), (12, 12), (24, 24)
    donut_piv = tuple(json.loads((ROOT / "assets/source/characters/donut/donut_h40.grid.json").read_text())["pivot"])
    gob_piv = (gob_flip.width - 1 - json.loads((ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json").read_text())["pivot"][0],
               json.loads((ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json").read_text())["pivot"][1])
    tags = {t["name"]: t for t in carl_g["tags"]}
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    names = [n for n, _, _ in STANDING]

    def crop_scene(entries, x0, width, k=1, up=1):
        full = floor.render(entries, k, True)
        vx, vy = floor.view[0], floor.view[1]
        gy = floor.ground(452) - vy
        box = (max(0, (x0 - vx) * k), (gy - 80) * k, min(full.width, (x0 - vx + width) * k), (gy + 14) * k)
        im = full.crop(box)
        return im.resize((im.width * up, im.height * up), Image.NEAREST) if up > 1 else im

    # 1. Stills at 1x (640x360 full view) and 3x: Carl mid-throw next to Donut with goblins in range; and all nine
    #    standing frames in a row at 1x.
    goblins = [(gob_flip, gob_piv, 566, 0), (gob_flip, gob_piv, 590, 0)]
    stick_lift = 46
    # Carl at the release key (the stick just off his fingers), the stick in flight toward two goblins, Donut behind.
    entries = [(donut, donut_piv, 404, 0), (carl[names.index("t5_release")], cpiv, 452, 0),
               (vfx[1], vpiv, 500, 66, False), (vfx[2], vpiv, 524, 72, False)] + goblins
    for k, nm in ((1, "scene-throw-lit-640x360"), (3, "scene-throw-lit-1920x1080")):
        floor.render(entries, k, True).save(EVIDENCE / f"{nm}.png")
    row = [(donut, donut_piv, 404, 0)]
    xs = [452 + 46 * i for i in range(9)]
    row += [(carl[i], cpiv, x, 0) for i, x in enumerate(xs)]
    row += [(gob_flip, gob_piv, 880, 0), (gob_flip, gob_piv, 904, 0)]
    floor.render(row, 1, True).save(EVIDENCE / "scene-phases-lit-640x360.png")
    floor.render(row, 3, True).save(EVIDENCE / "scene-phases-lit-1920x1080.png")

    # 2. In-scene GIFs: standing throw, moving throw (stick flight, fizzle on the floor, VFX-01 burst), 20 fps.
    font_cap = lambda im, text: (ImageDraw.Draw(im).rectangle((0, 0, im.width, 20), fill=(0, 0, 0)),
                                 label(ImageDraw.Draw(im), (6, 4), text, WHITE, 12))
    for moving, name, x0, cropx, cropw, upk in ((False, "throw-standing-scene", 452, 380, 270, 3),
                                                (True, "throw-moving-scene", 372, 330, 470, 2)):
        tl = throw_timeline(x0, moving, 150)
        lx = round(tl[[r["lands"] for r in tl if r["lands"] is not None][0]]["stick"]["x"])
        goblin_xs = (lx + 3, lx + 26)             # two goblins inside the burst radius at the landing point
        frames = []
        for rec in tl[::3]:
            f = rec["frame"]
            ents = []
            if rec["burst"] is not None:
                ents.append((boom[3 + rec["burst"]], bpiv, round(tl[rec["lands"]]["stick"]["x"]), 8, False))
            ents.append((donut, donut_piv, 404 if not moving else 335, 0))
            img, piv = (loco[f[1]], cpiv) if isinstance(f, tuple) else (carl[f], cpiv)
            ents.append((img, piv, rec["x"], 0))
            if rec["goblins"]:
                ents += [(gob_flip, gob_piv, gx, 0) for gx in goblin_xs]
            st = rec["stick"]
            if rec["state"] == "flight":
                ents.append((vfx[((rec["t"] - SPAWN_TICK) // 3) % 4], vpiv, round(st["x"]), max(0, round(st["h"])), False))
            elif rec["state"] == "fizzle":
                ents.append((vfx[4 + ((rec["t"] - rec["lands"]) // 4) % 2], vpiv, round(st["x"]), 0, False))
            im = crop_scene(ents, cropx, cropw, 1, upk)
            tag = (("plain run frame" if f[1] >= 4 else "idle") if isinstance(f, tuple) else rec["phase"])
            lit = "fuse lit" if FUSE_LIT_TICK <= rec["t"] < BLAST_AFTER_LIGHT + FUSE_LIT_TICK else "unlit"
            font_cap(im, f"tick {rec['t']:3d}   {tag}   {'blast' if rec['burst'] is not None else lit}   "
                         f"(placeholders: lob 150 px/s{' + run speed' if moving else ''}, blast t{FUSE_LIT_TICK + BLAST_AFTER_LIGHT}; "
                         f"goblins vanish: death art is separate)")
            frames.append(im)
        save_gif(frames, gif_delays([3] * len(frames)), EVIDENCE / f"{name}.gif")


def timing_diagram(carl_g, carl) -> None:
    """Frame, pivot and tick diagram of the standing throw: each frame at 3x with the pivot, the stick centre and
    the spark core, planted/lifted feet, and a to-scale tick bar with the fuse-lit and spawn ticks."""
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import BG, CYAN, GREY, WHITE, YELLOW, label, up
    K, PAD, CW = 3, 10, 48 * 3
    bounds = standing_ticks()
    kinds = {n: k for n, _, k in STANDING}
    W_img = PAD + len(bounds) * (CW + PAD)
    H_img = 360
    out = Image.new("RGB", (W_img, H_img + 150), BG)
    d = ImageDraw.Draw(out)
    label(d, (PAD, 6), "ANIM-CARL-03 standing throw: 9 frames, 70 ticks at 60 Hz. Yellow = pivot (25, 55), "
          "red = stick centre, white = spark core, cyan = floor row 55. Key frames are frozen.", WHITE, 12)
    grid = carl_g
    for n, a, b, i in bounds:
        x = PAD + i * (CW + PAD)
        im = Image.new("RGBA", (48, 56), (36, 34, 50, 255))
        im.alpha_composite(carl[i])
        big = up(im, K).convert("RGB")
        dd = ImageDraw.Draw(big)
        dd.line([0, 55 * K + K - 1, big.width, 55 * K + K - 1], fill=CYAN)
        px, py = 25 * K + 1, 55 * K + 1
        dd.rectangle([px - 2, py - 2, px + 2, py + 2], outline=YELLOW)
        an = frame_anchors(grid, i)
        if an:
            sx, sy = an["stick_centre"]
            dd.ellipse([(25 + sx) * K - 3, (55 - sy) * K - 3, (25 + sx) * K + 5, (55 - sy) * K + 5], outline=(255, 70, 70), width=2)
            if "fuse_spark" in an:
                fx, fy = an["fuse_spark"]
                dd.ellipse([(25 + fx) * K - 3, (55 - fy) * K - 3, (25 + fx) * K + 5, (55 - fy) * K + 5], outline=(255, 255, 255), width=2)
        out.paste(big, (x, 28))
        feet = []
        for c in sorted(_feet(grid["frames"][i]), key=lambda c: c[0]):
            feet.append(f"x{c[0]} {'planted' if c[1] == 55 else 'lifted ' + str(55 - c[1])}")
        label(d, (x, 28 + 56 * K + 4), f"{i} {n}", YELLOW if kinds[n] == "key" else WHITE, 12)
        label(d, (x, 28 + 56 * K + 20), f"ticks {a}-{b} ({b - a + 1})  {'KEY' if kinds[n] == 'key' else 'in-between'}", GREY, 11)
        label(d, (x, 28 + 56 * K + 35), "feet: " + ", ".join(feet), GREY, 11)
        if an:
            label(d, (x, 28 + 56 * K + 50), f"stick ({an['stick_centre'][0]:+d},{an['stick_centre'][1]:+d})"
                  + (f" spark ({an['fuse_spark'][0]:+d},{an['fuse_spark'][1]:+d})" if "fuse_spark" in an else " unlit"), GREY, 11)
    # tick bar, to scale
    y0 = 28 + 56 * K + 86
    scale = (W_img - 2 * PAD) / 70
    palette = [(90, 140, 255), (255, 214, 0), (255, 80, 80), (120, 200, 120), (255, 150, 40), (180, 120, 200),
               (0, 200, 200), (200, 120, 120), (150, 150, 220)]
    for n, a, b, i in bounds:
        x0, x1 = PAD + a * scale, PAD + (b + 1) * scale
        d.rectangle([x0, y0, x1 - 1, y0 + 30], fill=palette[i % len(palette)], outline=BG)
        label(d, (x0 + 3, y0 + 8), f"{i}", (10, 10, 20), 12)
        label(d, (x0 + 3, y0 + 34), f"t{a}", GREY, 11)
    for tick, text, colour in ((FUSE_LIT_TICK, f"fuse lit t{FUSE_LIT_TICK}", (255, 244, 184)),
                               (SPAWN_TICK, f"projectile spawns t{SPAWN_TICK} (last tick of frame 5)", (255, 120, 90))):
        x = PAD + (tick + (1 if tick == SPAWN_TICK else 0)) * scale if tick == SPAWN_TICK else PAD + tick * scale
        d.line([x, y0 - 12, x, y0 + 34], fill=colour, width=2)
        label(d, (x + 4, y0 - 16), text, colour, 12)
    xs, xe = PAD + FUSE_LIT_TICK * scale, PAD + (SPAWN_TICK + 1) * scale
    d.line([xs, y0 + 62, xe, y0 + 62], fill=(255, 244, 184), width=3)
    label(d, (xs, y0 + 66), f"spark visible in hand t{FUSE_LIT_TICK}-t{SPAWN_TICK} (36 ticks, 600 ms), then on the stick until the blast", GREY, 11)
    label(d, (PAD, y0 + 86), "Moving variant: 6 tags (light, hold, hold_b, wind, release, follow) x 6 run frames = 36 frames, 4 ticks each, "
          "following the locomotion clock; draw, brace and settle show the plain run frame.", GREY, 11)
    label(d, (PAD, y0 + 102), "Spawn offset (16, -46) from the pivot (screen y down); on the moving variant +1 px lower on run frames 5 and 8.", GREY, 11)
    out.crop((0, 0, W_img, y0 + 122)).save(EVIDENCE / "timing_diagram.png")


def _feet(frame: dict) -> list[tuple[int, int]]:
    """(leftmost x, lowest row) of each foot (connected component of the shoes layer) of a grid frame."""
    pix = {(x, y) for y, r in enumerate(frame["cels"].get("shoes", [])) for x, c in enumerate(r) if c != "."}
    seen, out = set(), []
    for start in sorted(pix):
        if start in seen:
            continue
        todo, comp = [start], []
        seen.add(start)
        while todo:
            x, y = todo.pop()
            comp.append((x, y))
            for dx, dy in cm.NEIGHBOURS:
                q = (x + dx, y + dy)
                if q in pix and q not in seen:
                    seen.add(q)
                    todo.append(q)
        out.append((min(x for x, _ in comp), max(y for _, y in comp)))
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grids equal the build output")
    ap.add_argument("--evidence", action="store_true", help="render the silhouette and scale evidence")
    ap.add_argument("--ascii", help="print a Carl atlas frame index or standing frame id (t2_hold), or 'vfx', as ASCII")
    a = ap.parse_args(argv)
    if a.ascii:
        if a.ascii == "vfx":
            g = vfx_grid()
            for i, fr in enumerate(g["frames"]):
                print(i)
                print("\n".join(fr["cels"]["body"]))
            return 0
        if a.ascii.isdigit():               # an atlas frame index of the Carl grid
            fr = carl_grid()["frames"][int(a.ascii)]
            flat = {}
            for l in LAYERS:
                flat.update({(x, y): c for y, r in enumerate(fr["cels"].get(l, [])) for x, c in enumerate(r) if c != "."})
        else:                               # a standing frame id, e.g. t2_hold
            flat = FRAMES[a.ascii]().flat()
        print("    " + "".join(str(i % 10) for i in range(W)))
        for y, r in enumerate(rows(flat, W, H)):
            print(f"{y:3d} {r}")
        return 0
    outs = outputs()
    if a.check:
        bad = [p for p, text in outs.items() if not p.is_file() or p.read_text(encoding="utf-8") != text]
        for p in bad:
            print(f"{p.relative_to(ROOT)} DIFFERS from the build")
        print("dynamite grids match the build" if not bad else "")
        return 1 if bad else 0
    if a.evidence:
        evidence()
        return 0
    for p, text in outs.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8")
        print(f"wrote {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
