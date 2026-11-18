#!/usr/bin/env python3
"""Authoring record for ANIM-CARL-03: Carl's dynamite throw (equipped special), design phase.

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
* Design phase: the keys (L light, H hold, W wind-up, R release; VFX flight 0 and 3, fizzle 0) are drawn
  here and frozen; every in-between is a copy of its nearest key until the execution phase draws it from
  the spec's in-between table. The moving tags are derived by rule (``move_frame``) from the standing
  frames and the run frames, so they follow the in-betweens automatically.

Run ``python3 scripts/author_carl_dynamite.py`` to rewrite both grids (deterministic), ``--check`` to verify
the committed grids, ``--evidence`` to render the silhouette and scale sheets in ``docs/evidence/anim-carl-03/``.
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


def key_hold() -> Frame:
    """H, the tell: the lit stick held out at arm's length in front, upright, spark at face height; he leans
    back, head pulled away, the far hand up palm-out between his face and the spark. Brave enough to hold it,
    not happy about it."""
    f = Frame()
    cm.upper(f, (-1, 2), "v3", (-2, 0))                     # head pulled back from the spark (2 px, the limit)
    cm.standing_legs(f, 41, 15, 24, 12, 30)
    # far arm (behind): elbow forward, open hand up beside the face, palm toward the spark
    cm.sleeve_arm(f, "far_arm", [(28, 23), (33, 24), (34, 17)], 2.0)
    f.add("far_arm", HAND_SPREAD, 31, 10)                    # x31-37, rows 10-15
    # near arm: nearly straight, forward and a little down, as far from his face as it goes
    cm.sleeve_arm(f, "near_arm", [(18, 22), (28, 25), (37, 27)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 37, 25)                 # grip x37-42, rows 25-28
    cm.watch_cuff(f, 35, 27, ((0, -1),))
    stick(f, "V", 38, 17, SPARK_A, (40, 11))                 # stick x38-41 rows 17-25, spark x40-43 rows 11-14
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


# ------------------------------------------------------------------ standing plan (spec section 4)
# (frame id, ticks, key it copies in the design phase or None for a key)
STANDING = [
    ("t0_draw", 6, "t1_light"),
    ("t1_light", 10, None),
    ("t2_hold", 7, None),
    ("t3_hold_b", 7, "t2_hold"),
    ("t4_wind", 8, None),
    ("t5_release", 4, None),
    ("t6_follow", 8, "t5_release"),
    ("t7_brace", 12, "t5_release"),
    ("t8_settle", 8, "t5_release"),
]
KEYS = {"t1_light": key_light, "t2_hold": key_hold, "t4_wind": key_wind, "t5_release": key_release}
KEY_INDEX = {name: i for i, (name, _, src) in enumerate(STANDING) if src is None}

# Moving variant: throw arms baked onto the six run frames (locomotion 4-9). Phases that keep the plain run
# arms (draw, brace, settle) have no moving frames: the runtime shows the run frame itself.
MOVE_PHASES = ["t1_light", "t2_hold", "t3_hold_b", "t4_wind", "t5_release", "t6_follow"]
RUN_FRAMES = [4, 5, 6, 7, 8, 9]
ARM_LAYERS = ("far_arm", "near_arm", "watch", "prop")
# Upper-block translation of each standing frame (U), used to move its arms onto a run frame's upper block.
# Keys: as drawn. In-betweens: the spec's planned U (section 6); while an in-between is still a copy of its key
# (design phase), ``phase_u`` uses the key's U instead, so the copied arms stay on the shoulder.
STANDING_U = {"t0_draw": (1, 1), "t1_light": (1, 2), "t2_hold": (-1, 2), "t3_hold_b": (-1, 2),
              "t4_wind": (-2, 1), "t5_release": (3, 2), "t6_follow": (5, 3), "t7_brace": (1, 3),
              "t8_settle": (1, 1)}
# The hold and wind-up phases lean back when standing; on the run body (U(+3, +2..3)) their forward hand or spark
# would reach column 47 (the clipping rule keeps it empty). Their arms sit 1 px further back on the shoulder
# instead; the arm root is drawn over the jacket, so the 1 px slide does not open a gap.
MOVE_TRIM = {"t2_hold": (-1, 0), "t3_hold_b": (-1, 0), "t4_wind": (-1, 0)}


def phase_u(phase: str) -> tuple[int, int]:
    src = {name: src for name, _, src in STANDING}[phase]
    return STANDING_U[src or phase]


def run_u(n: int) -> tuple[int, int]:
    """Upper-block translation of locomotion frame ``n`` (measured on the torso layer, which is translated only)."""
    torso = cm.layers_from_cels(cm.LOCO["frames"][n]["cels"])["torso"]
    for dy in range(-6, 8):
        for dx in range(-6, 7):
            if cm.shift(cm.IDLE0["torso"], dx, dy) == torso:
                return dx, dy
    raise ValueError(f"locomotion frame {n}: torso is not a translation of idle 0")


def standing_frames() -> dict[str, dict]:
    built = {name: fn().L for name, fn in KEYS.items()}
    return {name: built[src or name] for name, _, src in STANDING}


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


VFX_PLAN = [  # (tag, [(frame id, ticks, key it copies or None)])
    # A forward (clockwise) tumble in 90 degree steps; the release key's stick points up-right, so the loop
    # starts at f0 (fuse forward): right, down, left, up. The stick body is symmetric: the fuse and spark carry
    # the spin; the lit row stays on top in every frame (the light does not turn with the stick).
    ("dynamite_flight", [("f0_right", 3, None), ("f1_down", 3, "f0_right"), ("f2_left", 3, "f0_right"),
                         ("f3_up", 3, None)]),
    ("dynamite_fizzle", [("z0", 4, None), ("z1", 4, "z0")]),
]
VFX_KEYS = {"f0_right": lambda: vfx_flight("H", SPARK_A), "f3_up": lambda: vfx_flight("V", SPARK_B),
            "z0": lambda: vfx_fizzle(SPARK_A)}


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
    built = {k: fn() for k, fn in VFX_KEYS.items()}
    frames, tags = [], []
    for tag, plan in VFX_PLAN:
        start = len(frames)
        for name, ticks, src in plan:
            frames.append({"duration_ms": round(ticks * 1000 / TICK_HZ),
                           "cels": {"body": rows(built[src or name]["body"], VFX_W, VFX_H)}})
        tags.append({"name": tag, "from": start, "to": len(frames) - 1, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [VFX_W, VFX_H], "pivot": [12, 12], "palette": VFX_PALETTE,
            "layers": ["body"], "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    return cm.dump(grid)


def outputs() -> dict[Path, str]:
    return {CARL_OUT: dump(carl_grid()), VFX_OUT: dump(vfx_grid())}


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

    # 4. Moving variant: the release phase baked onto the six run frames, at 1x on lit Floor 1 and 4x.
    mv = [i for i, t in enumerate(carl_g["tags"]) if t["name"] == "dynamite_move_release"][0]
    t = carl_g["tags"][mv]
    entries = [(donut, dpiv, 404, 0), (loco[0], cpiv, 452, 0)]
    entries += [(carl[t["from"] + j], cpiv, 500 + 48 * j, 0) for j in range(6)]
    sheet(entries, "move-release-scale", 370, 440, k3=True)
    print(f"wrote evidence in {EVIDENCE.relative_to(ROOT)}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grids equal the build output")
    ap.add_argument("--evidence", action="store_true", help="render the silhouette and scale evidence")
    ap.add_argument("--ascii", help="print one standing frame id (or 'vfx') as ASCII and exit")
    a = ap.parse_args(argv)
    if a.ascii:
        if a.ascii == "vfx":
            g = vfx_grid()
            for i, fr in enumerate(g["frames"]):
                print(i)
                print("\n".join(fr["cels"]["body"]))
            return 0
        f = KEYS[a.ascii]()
        flat = f.flat()
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
