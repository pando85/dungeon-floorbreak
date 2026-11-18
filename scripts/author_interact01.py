#!/usr/bin/env python3
"""Authoring record for ANIM-INTERACT-01: Carl (52 px) and Princess Donut (40 px) interact keys (design phase).

Writes two pixel-grid-v1 sources defined in ``docs/02-art/interact-anim-01-spec.md``:

* ``assets/source/characters/carl/carl_h52_interact.grid.json``  (12 frames, 48 x 56, pivot (25, 55))
* ``assets/source/characters/donut/donut_h40_interact.grid.json`` (8 frames, 72 x 44, pivot (46, 43))

Same canvas, pivot, palette and layers as each character's approved locomotion grid. The design phase
draws only the KEY poses (Carl u1, u2, p1, s1; Donut d1, n1); every in-between frame is a copy of its
nearest key until the execution phase replaces it (spec section 6).

How the keys are made (no resampling, no rotation, no image model):

* Carl: hair, head, torso, jacket, pin and the boxer seat are the approved master's own pixels, translated
  only (``author_carl_melee.upper``). Arms, legs, boxer thighs and hands are rasterised from centre lines
  and shaded with the master ramps (the ANIM-CARL-02 helpers); Crocs and fists are stamps cut from the
  approved locomotion idle and the review-passed melee keys.
* Donut: tail, body, head, face and gear are her approved idle frame's own pixels, translated only. The
  tapping foreleg is rasterised and shaded with her cream ramp; covered collar pixels are dropped.

Run ``python3 scripts/author_interact01.py`` to rewrite both grids (deterministic), ``--check`` to verify
the committed grids, ``--evidence`` to render ``docs/evidence/anim-interact-01/keys-scale*.png``.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_melee as cm  # noqa: E402  (helpers and approved-master parts; read-only)

CARL_OUT = ROOT / "assets/source/characters/carl/carl_h52_interact.grid.json"
DONUT_LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
DONUT_OUT = ROOT / "assets/source/characters/donut/donut_h40_interact.grid.json"
EVIDENCE = ROOT / "docs/evidence/anim-interact-01"
CONFIGS_CARL = ROOT / "assets/source/characters/carl/carl_h52_interact.pixelart.json"
TICK_HZ = 60

# ------------------------------------------------------------------ frame plans (spec section 3)
# (tag, [(frame name, ticks, key it copies or None if it is a key)], contact frame name)
CARL_PLAN = [
    ("interact_hand", [("u0", 5, None), ("u1", 8, None), ("u2", 10, None), ("u3", 8, None)], "u1"),
    ("interact_floor", [("p0", 8, None), ("p1", 10, None), ("p2", 10, None), ("p3", 8, None)], "p1"),
    ("interact_push", [("s0", 6, None), ("s1", 8, None), ("s2", 12, None), ("s3", 8, None)], "s1"),
]
DONUT_PLAN = [
    ("interact_paw", [("d0", 6, None), ("d1", 6, None), ("d2", 8, None), ("d3", 6, None)], "d1"),
    ("interact_nudge", [("n0", 8, None), ("n1", 8, None), ("n2", 10, None), ("n3", 8, None)], "n1"),
]


def contact_ticks(plan) -> dict[str, int]:
    out = {}
    for tag, rows, contact in plan:
        t = 0
        for name, ticks, _ in rows:
            if name == contact:
                out[tag] = t
            t += ticks
    return out


# ------------------------------------------------------------------ Carl keys (frame 48 x 56, pivot (25, 55))
IDLE0 = cm.IDLE0


def shade_boxer(mask: set) -> dict:
    """Boxer thigh: K outline, V shade on the lower-right rim, W cloth."""
    def border(x, y):
        return any((x + dx, y + dy) not in mask for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))

    out = {}
    for (x, y) in mask:
        if border(x, y):
            out[(x, y)] = "K"
        elif (x, y + 1) in mask and border(x, y + 1):
            out[(x, y)] = "V"
        else:
            out[(x, y)] = "W"
    return out


def carl_u1() -> cm.Frame:
    """Hand-height use, contact: relaxed stance, the far arm reaches forward-down and grips the handle."""
    f = cm.upper(cm.Frame(), (2, 1), "v0")
    cm.standing_legs(f, 40, 18, 27, 15, 29)
    f.add("near_arm", IDLE0["near_arm"], 2, 1)
    f.add("watch", IDLE0["watch"], 2, 1)
    cm.sleeve_arm(f, "far_arm", [(31, 21), (37, 26)], 2.0)
    f.add("far_arm", cm.FIST_BIG, 37, 24)                    # grip x37-42, rows 24-29 (26-31 above the floor)
    return f


def carl_u2() -> cm.Frame:
    """Hand-height use, throw: the handle is hauled down to the belt, he sags back on his heels."""
    f = cm.upper(cm.Frame(), (-1, 3), "v3", (0, 1))
    cm.standing_legs(f, 42, 15, 24, 13, 26)
    f.add("near_arm", IDLE0["near_arm"], -1, 3)
    f.add("watch", IDLE0["watch"], -1, 3)
    cm.sleeve_arm(f, "far_arm", [(28, 23), (36, 32)], 2.0)  # taut diagonal from the shoulder to the handle
    f.add("far_arm", cm.FIST_BIG, 36, 31)                    # fist x36-41, rows 31-36 (19-24 above the floor)
    return f


def carl_p1() -> cm.Frame:
    """Floor pickup, contact: a deep, heels-up squat, near hand braced on the knee, far hand on the floor."""
    f = cm.upper(cm.Frame(), (1, 11), "v3", (1, 0))
    # legs: back shin (behind), front thigh (boxer then skin) and shin
    f.add("legs", cm.shade_skin(cm.capsule((33, 43), (25, 48), 2.6, 2.0)))
    thigh = cm.capsule((28, 45), (36, 42), 3.2, 3.0)
    f.add("boxers", shade_boxer({p for p in thigh if p[0] <= 31}))
    f.add("legs", cm.shade_skin(cm.union({p for p in thigh if p[0] > 31}, cm.capsule((37, 42), (35, 49), 2.8, 2.0))))
    cm.croc_toe_down(f, 21)                                  # back Croc on its toe under the seat, heel lifted
    cm.croc_flat(f, 30)                                      # front Croc x30-39
    # far arm: shoulder behind the jacket, straight down-forward past the knee, hand on the item
    cm.sleeve_arm(f, "far_arm", [(31, 31), (40, 47)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 39, 48)                  # hand x39-44, rows 48-51 (4-7 above the floor)
    # near arm: crosses the chest, hand braced on the front knee
    cm.sleeve_arm(f, "near_arm", [(21, 30), (31, 38)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 31, 38)
    cm.watch_cuff(f, 30, 38, ((-1, 0),))
    return f


def carl_s1() -> cm.Frame:
    """Shoulder push, contact: leaning hard into the prop, back leg driving on its toe, both fists on the face."""
    f = cm.upper(cm.Frame(), (8, 3), "v3", (1, 1))
    cm.hem(f, 22, 42, cm.HEM_BACK)
    cm.hem(f, 31, 42, cm.HEM_FRONT)
    f.add("legs", cm.leg((25, 44), (16, 49), 3.0, 2.0))      # back leg: long diagonal, the push comes from here
    f.add("legs", cm.leg((36, 44), (37, 49), 3.0, 2.0))      # front leg planted under the hips
    cm.croc_toe_down(f, 12)
    cm.croc_flat(f, 33)
    cm.sleeve_arm(f, "far_arm", [(37, 22), (41, 23)], 2.0)
    f.add("far_arm", cm.FIST_BIG, 40, 20)                    # far fist x40-45, rows 20-25 (chest)
    cm.sleeve_arm(f, "near_arm", [(27, 23), (38, 33)], 2.0)
    f.add("near_arm", cm.FIST_BIG, 39, 31)                   # near fist x39-44, rows 31-36 (belly)
    cm.watch_cuff(f, 37, 32, ((0, -1),))
    return f


# ------------------------------------------------------------------ Carl in-betweens (execution phase)
# Feet: idle back/front Croc x15/x27. Verbs keep both feet planted; the small shuffles below are the only motion.


def _finish(f: cm.Frame) -> cm.Frame:
    """Same post-pass as ANIM-CARL-02 in-betweens: pin occlusion and orphan merge (master pixels stay)."""
    return cm.cleanup(cm.occlude_pin(f))


def carl_u0() -> cm.Frame:
    """idle -> u1: a half-step of weight forward, far arm half raised with the elbow bent, fist low, head v0."""
    f = cm.upper(cm.Frame(), (1, 0), "v0")
    cm.standing_legs(f, 39, 17, 26, 15, 28)
    f.add("near_arm", IDLE0["near_arm"], 1, 0)
    f.add("watch", IDLE0["watch"], 1, 0)
    cm.sleeve_arm(f, "far_arm", [(30, 21), (34, 26), (36, 30)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 34, 30)                  # elbow out, fist x34-39, rows 30-33
    return _finish(f)


def carl_u3() -> cm.Frame:
    """u2 -> idle: lets go, the far arm drops to the idle position, a glance back to see if it worked (head v1)."""
    f = cm.upper(cm.Frame(), (0, 2), "v1")
    cm.standing_legs(f, 41, 16, 26, 15, 27)
    cm.idle_arms(f, (0, 2))
    return _finish(f)


def carl_p0() -> cm.Frame:
    """idle -> p1: half squat, back heel still down, far arm hanging forward, near arm going to the knee."""
    f = cm.upper(cm.Frame(), (1, 5), "v0")
    hem_y = 44
    cm.hem(f, 18, hem_y, cm.HEM_BACK)
    cm.hem(f, 27, hem_y, cm.HEM_FRONT)
    f.add("legs", cm.leg_bent((21, hem_y + 2), (20, 48), (22, 50), 3.0, 2.8, 2.0))
    f.add("legs", cm.leg_bent((31, hem_y + 2), (34, 47), (33, 50), 3.0, 2.8, 2.0))
    cm.croc_flat(f, 17)
    cm.croc_flat(f, 29)
    cm.sleeve_arm(f, "far_arm", [(31, 26), (35, 34), (38, 40)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 37, 40)                  # hangs forward, fist x37-42, rows 40-43
    cm.sleeve_arm(f, "near_arm", [(22, 27), (27, 35), (30, 38)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 29, 38)
    cm.watch_cuff(f, 28, 37, ((-1, 0),))
    return _finish(f)


def _stand_up_legs(f: cm.Frame, hem_y: int, back_foot: int, front_foot: int, toe: bool) -> None:
    cm.hem(f, 18, hem_y, cm.HEM_BACK)
    cm.hem(f, 27, hem_y, cm.HEM_FRONT)
    f.add("legs", cm.leg_bent((21, hem_y + 2), (20, hem_y + 5), (back_foot + 4, 49), 3.0, 2.8, 2.0))
    f.add("legs", cm.leg_bent((31, hem_y + 2), (33, hem_y + 4), (front_foot + 4, 49), 3.0, 2.8, 2.0))
    if toe:
        cm.croc_toe_down(f, back_foot)
    else:
        cm.croc_flat(f, back_foot)
    cm.croc_flat(f, front_foot)


def carl_p2() -> cm.Frame:
    """p1 -> p3: pushing up off the knee, the item rides in the far hand at the hip."""
    f = cm.upper(cm.Frame(), (1, 6), "v3", (1, 0))
    _stand_up_legs(f, 45, 20, 29, True)
    cm.sleeve_arm(f, "far_arm", [(31, 30), (35, 36), (37, 39)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 35, 38)                  # far hand at the hip, rows 38-41
    cm.sleeve_arm(f, "near_arm", [(21, 30), (29, 40)], 2.0)
    f.add("near_arm", cm.FIST_SMALL, 29, 40)
    cm.watch_cuff(f, 28, 40, ((-1, 0),))
    return _finish(f)


def carl_p3() -> cm.Frame:
    """p2 -> idle: standing, the item at the hip in the far hand, head v3."""
    f = cm.upper(cm.Frame(), (1, 1), "v3")
    cm.standing_legs(f, 40, 17, 27, 16, 28)
    f.add("near_arm", IDLE0["near_arm"], 1, 1)
    f.add("watch", IDLE0["watch"], 1, 1)
    cm.sleeve_arm(f, "far_arm", [(30, 22), (34, 30), (36, 35)], 2.0)
    f.add("far_arm", cm.FIST_SMALL, 34, 34)                  # far hand at the hip: rows 34-37
    return _finish(f)


def carl_s0() -> cm.Frame:
    """idle -> s1: set, lean in; back foot stepped back, both arms bent forward, fists at chest and belly."""
    f = cm.upper(cm.Frame(), (4, 2), "v3")
    cm.hem(f, 19, 41, cm.HEM_BACK)
    cm.hem(f, 29, 41, cm.HEM_FRONT)
    f.add("legs", cm.leg((22, 43), (15, 49), 3.0, 2.0))
    f.add("legs", cm.leg((33, 43), (35, 49), 3.0, 2.0))
    cm.croc_flat(f, 11)
    cm.croc_flat(f, 31)
    cm.sleeve_arm(f, "far_arm", [(33, 22), (36, 23)], 2.0)
    f.add("far_arm", cm.FIST_BIG, 35, 20)                    # far fist at chest height
    cm.sleeve_arm(f, "near_arm", [(23, 24), (33, 32)], 2.0)
    f.add("near_arm", cm.FIST_BIG, 34, 30)                   # near fist at belly height
    cm.watch_cuff(f, 33, 31, ((0, -1),))
    return _finish(f)


def carl_s2() -> cm.Frame:
    """s1 hold: the contact holds, he strains: head 1 px lower, back heel 1 px lower, fists unchanged."""
    f = cm.upper(cm.Frame(), (8, 3), "v3", (1, 2))
    cm.hem(f, 22, 42, cm.HEM_BACK)
    cm.hem(f, 31, 42, cm.HEM_FRONT)
    f.add("legs", cm.leg((25, 44), (16, 50), 3.0, 2.0))
    f.add("legs", cm.leg((36, 44), (37, 49), 3.0, 2.0))
    f.add("shoes", {k: c for k, c in cm.CROC_TOE.items() if k[1] >= 1}, 12, 55 - 5 - 1)   # heel row dropped
    cm.croc_flat(f, 33)
    cm.sleeve_arm(f, "far_arm", [(37, 22), (41, 23)], 2.0)
    f.add("far_arm", cm.FIST_BIG, 40, 20)
    cm.sleeve_arm(f, "near_arm", [(27, 23), (38, 33)], 2.0)
    f.add("near_arm", cm.FIST_BIG, 39, 31)
    cm.watch_cuff(f, 37, 32, ((0, -1),))
    return f                                                 # no cleanup: the arms stay s1's exact pixels


def carl_s3() -> cm.Frame:
    """s2 -> idle: steps the back foot in, arms drop, glances at the prop (head v1)."""
    f = cm.upper(cm.Frame(), (3, 1), "v1")
    cm.standing_legs(f, 40, 19, 29, 14, 30)
    cm.idle_arms(f, (3, 1))
    return _finish(f)


CARL_KEYS = {"u0": carl_u0, "u1": carl_u1, "u2": carl_u2, "u3": carl_u3,
             "p0": carl_p0, "p1": carl_p1, "p2": carl_p2, "p3": carl_p3,
             "s0": carl_s0, "s1": carl_s1, "s2": carl_s2, "s3": carl_s3}

# ------------------------------------------------------------------ Donut keys (frame 72 x 44, pivot (46, 43))
DONUT = json.loads(DONUT_LOCO.read_text(encoding="utf-8"))
D_LAYERS = DONUT["layers"]
D_W, D_H = DONUT["canvas"]
D_IDLE = {l: {(x, y): c for y, row in enumerate(DONUT["frames"][0]["cels"].get(l, [])) for x, c in enumerate(row)
              if c != "."} for l in D_LAYERS}
TIARA = {p: c for p, c in D_IDLE["gear"].items() if p[1] <= 8}
COLLAR = {p: c for p, c in D_IDLE["gear"].items() if p[1] > 8}


def shade_fur(mask: set) -> dict:
    """Cream foreleg: K outline, c shadow on the lower-right rim, W lit upper-left rim, C base."""
    def border(x, y):
        return any((x + dx, y + dy) not in mask for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))

    out = {}
    for (x, y) in mask:
        if border(x, y):
            out[(x, y)] = "K"
        elif ((x + 1, y) in mask and border(x + 1, y)) or ((x, y + 1) in mask and border(x, y + 1)):
            out[(x, y)] = "c"
        elif (x, y - 1) in mask and border(x, y - 1):
            out[(x, y)] = "W"
        else:
            out[(x, y)] = "C"
    return out


class DFrame:
    def __init__(self) -> None:
        self.L = {l: {} for l in D_LAYERS}

    def add(self, layer, pix, dx=0, dy=0):
        for (x, y), c in pix.items():
            self.L[layer][(x + dx, y + dy)] = c
        return self

    def cels(self):
        out = {}
        for l in D_LAYERS:
            if self.L[l]:
                g = [["."] * D_W for _ in range(D_H)]
                for (x, y), c in self.L[l].items():
                    if 0 <= x < D_W and 0 <= y < D_H:
                        g[y][x] = c
                out[l] = ["".join(r) for r in g]
        return out


def donut_d1() -> DFrame:
    """Paw tap, contact: the front foreleg reaches forward and pats the low target; head dips to watch it."""
    f = DFrame()
    f.add("tail", D_IDLE["tail"])
    body = {p: c for p, c in D_IDLE["body"].items() if not (53 <= p[0] <= 61 and p[1] >= 41)}
    for x in range(53, 62):                                  # close the chest where the front leg lifted
        body[(x, 41)] = "K"
    f.add("body", body)
    leg = cm.union(cm.capsule((57, 33), (62, 35), 2.4, 2.2), cm.capsule((65, 36), (65, 36), 2.5, 2.5))
    f.add("legs_near", shade_fur(leg))                       # round paw x63-67, rows 34-38, on the switch top
    f.add("head", D_IDLE["head"], 1, 1)
    f.add("face", D_IDLE["face"], 1, 1)
    f.add("gear", TIARA, 1, 1)
    f.add("gear", {p: c for p, c in COLLAR.items() if (p[0] - 1, p[1] - 1) not in D_IDLE["head"]})
    return f


def donut_n1() -> DFrame:
    """Nose nudge, contact: head dropped low and forward, muzzle on the item, rump and tail stay up."""
    dx, dy = 2, 10
    f = DFrame()
    f.add("tail", D_IDLE["tail"])
    f.add("body", D_IDLE["body"])
    f.add("head", D_IDLE["head"], dx, dy)
    f.add("face", D_IDLE["face"], dx, dy)
    f.add("gear", TIARA, dx, dy)
    head_now = {(x + dx, y + dy) for (x, y) in D_IDLE["head"]}
    f.add("gear", {p: c for p, c in COLLAR.items() if p not in head_now})
    return f


# ------------------------------------------------------------------ Donut in-betweens (execution phase)
def _paw_body() -> dict:
    """Body with the front foreleg lifted off and the chest closed (as in key d1)."""
    body = {p: c for p, c in D_IDLE["body"].items() if not (53 <= p[0] <= 61 and p[1] >= 41)}
    for x in range(53, 62):
        body[(x, 41)] = "K"
    return body


def _paw_frame(head_d: tuple[int, int], leg_mask: set) -> DFrame:
    f = DFrame()
    f.add("tail", D_IDLE["tail"])
    f.add("body", _paw_body())
    f.add("legs_near", shade_fur(leg_mask))
    f.add("head", D_IDLE["head"], *head_d)
    f.add("face", D_IDLE["face"], *head_d)
    f.add("gear", TIARA, *head_d)
    head_now = {(x + head_d[0], y + head_d[1]) for (x, y) in D_IDLE["head"]}
    f.add("gear", {p: c for p, c in COLLAR.items() if p not in head_now})
    return f


def donut_d0() -> DFrame:
    """idle -> d1: weight shifts back, the front paw is lifted and tucked (round), above the d1 paw."""
    leg = cm.union(cm.capsule((56, 31), (59, 32), 2.2, 2.2), cm.capsule((60, 32), (60, 32), 2.5, 2.5))
    return _paw_frame((0, 1), leg)                           # paw disc x58-62, rows 30-34


def donut_d2() -> DFrame:
    """d1 -> d3: the paw lifts 2 px straight up off the switch top, the head stays on the target."""
    leg = cm.union(cm.capsule((57, 32), (62, 33), 2.4, 2.2), cm.capsule((65, 34), (65, 34), 2.5, 2.5))
    return _paw_frame((1, 1), leg)                           # paw disc x63-67, rows 32-36


def donut_d3() -> DFrame:
    """d2 -> idle: the paw is back on the floor (the idle front leg), the head is at idle."""
    f = DFrame()
    for layer in D_LAYERS:
        f.add(layer, D_IDLE[layer])
    return f


def _nudge_frame(head_d: tuple[int, int], tail: dict | None = None) -> DFrame:
    f = DFrame()
    f.add("tail", tail if tail is not None else D_IDLE["tail"])
    f.add("body", D_IDLE["body"])
    f.add("head", D_IDLE["head"], *head_d)
    f.add("face", D_IDLE["face"], *head_d)
    f.add("gear", TIARA, *head_d)
    head_now = {(x + head_d[0], y + head_d[1]) for (x, y) in D_IDLE["head"]}
    f.add("gear", {p: c for p, c in COLLAR.items() if p not in head_now})
    return f


def tail_flick() -> dict:
    """The tail tip 1 px up: rows 14 and above move up one row and row 14 is repeated (a 1 px stretch)."""
    tail = D_IDLE["tail"]
    out = {(x, y - 1): c for (x, y), c in tail.items() if y <= 14}
    out.update({(x, y): c for (x, y), c in tail.items() if y >= 14})
    return out


def donut_n0() -> DFrame:
    """idle -> n1: the head starts to lower toward the item."""
    return _nudge_frame((1, 5))


def donut_n2() -> DFrame:
    """n1 hold: the nudge pushes through: head 1 px lower (the table's +1 px forward would put the head outline
    on column 71, the frame's right edge, which must stay empty), tail tip flicking up. Not a dive."""
    return _nudge_frame((2, 11), tail_flick())


def donut_n3() -> DFrame:
    """n2 -> idle: the head comes back up."""
    return _nudge_frame((1, 4))


DONUT_KEYS = {"d0": donut_d0, "d1": donut_d1, "d2": donut_d2, "d3": donut_d3,
              "n0": donut_n0, "n1": donut_n1, "n2": donut_n2, "n3": donut_n3}

# ------------------------------------------------------------------ grids


def build(plan, keys, base: dict) -> dict:
    frames, tags, i = [], [], 0
    for tag, rows, _ in plan:
        start = i
        for name, ticks, src in rows:
            frames.append({"duration_ms": round(ticks * 1000 / TICK_HZ), "cels": keys[src or name]().cels()})
            i += 1
        tags.append({"name": tag, "from": start, "to": i - 1, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": base["canvas"], "pivot": base["pivot"],
            "palette": base["palette"], "layers": base["layers"], "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def outputs() -> dict[Path, str]:
    return {CARL_OUT: dump(build(CARL_PLAN, CARL_KEYS, cm.LOCO)),
            DONUT_OUT: dump(build(DONUT_PLAN, DONUT_KEYS, DONUT))}


# ------------------------------------------------------------------ evidence


def _props():
    """Placeholder props (context only, not art): lever post (handle up / thrown), low switch, crate face, small item."""
    from PIL import Image, ImageDraw

    def prop(w, h, draw_fn):
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        draw_fn(ImageDraw.Draw(img))
        return img

    iron, iron_d, wood, wood_d = (88, 86, 98, 255), (34, 30, 40, 255), (112, 84, 58, 255), (46, 32, 26, 255)
    return {
        "lever_up": prop(6, 32, lambda d: (d.rectangle([2, 4, 4, 31], fill=iron, outline=iron_d),
                                           d.rectangle([0, 0, 5, 5], fill=iron, outline=iron_d))),
        "lever_down": prop(9, 32, lambda d: (d.rectangle([5, 4, 7, 31], fill=iron, outline=iron_d),
                                             d.rectangle([0, 7, 5, 12], fill=iron, outline=iron_d))),
        "crate": prop(22, 32, lambda d: (d.rectangle([0, 0, 21, 31], fill=wood, outline=wood_d),
                                         d.line([0, 0, 21, 31], fill=wood_d), d.line([0, 31, 21, 0], fill=wood_d))),
        "item": prop(6, 5, lambda d: d.rectangle([0, 0, 5, 4], fill=iron, outline=iron_d)),
        "low_switch": prop(5, 5, lambda d: d.rectangle([0, 0, 4, 4], fill=iron, outline=iron_d)),
    }


def _fixed(img, x, stand_off):
    """A prop whose bottom row sits on the walking line and whose left edge is ``stand_off`` px right of ``x``."""
    return (img, (-stand_off, img.height - 1), x, 0, False)


def _scene(floor, entries, name: str) -> None:
    """Lit Floor 1 at the 640 x 360 virtual view (x1) and the exact 3x 1080p copy."""
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    floor.render(entries, 1).save(EVIDENCE / f"{name}_640x360.png")
    floor.render(entries, 3).save(EVIDENCE / f"{name}_1920x1080.png")


def _timing_diagram() -> None:
    """Frame layout, pivot and tick timing of both sets, drawn from the configs and manifests."""
    from PIL import Image, ImageDraw
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import BG, CYAN, GREY, WHITE, YELLOW, cell, grid_images, label

    PHASE = {"startup": (90, 140, 255), "contact": (255, 80, 80), "active": (255, 150, 40), "recovery": (120, 200, 120)}
    sets = [("Carl", CARL_OUT, ROOT / "assets/manifests/carl_book1_h52_interact.json"),
            ("Donut", DONUT_OUT, ROOT / "assets/manifests/donut_h40_interact.json")]
    K, TICK_PX, PAD = 2, 5, 10
    rows = []
    for who, grid_path, man_path in sets:
        grid = json.loads(grid_path.read_text(encoding="utf-8"))
        man = json.loads(man_path.read_text(encoding="utf-8"))
        rows.append((who, grid, man, grid_images(grid)))
    width = max(sum(m["total_ticks"] * TICK_PX + 70 for m in r[2]["interact"]["moves"].values()) for r in rows)
    width = max(width, 1100) + 2 * PAD
    heights = [26 + len(r[2]["interact"]["moves"]) * (r[1]["canvas"][1] * K + 70) for r in rows]
    out = Image.new("RGBA", (width, sum(heights) + 2 * PAD + 40), BG + (255,))
    d = ImageDraw.Draw(out)
    label(d, (PAD, 4), "ANIM-INTERACT-01 frame layout, pivot and 60 Hz timing. Frames face right, grounded on the pivot row, no root motion.", WHITE, 12)
    label(d, (PAD, 20), "cyan line = floor row, yellow box = pivot, white tick on a red bar = contact tick (the effect applies on it, never on the press).", WHITE, 12)
    y = 42
    for who, grid, man, imgs in rows:
        w, h = grid["canvas"]
        label(d, (PAD, y), f"{who}: frame {w} x {h} px, pivot {tuple(grid['pivot'])}, {len(imgs)} frames "
              f"(stand-off px: {', '.join(f'{k} {v}' for k, v in man['interact']['stand_off_px'].items() if k != 'note')})", YELLOW, 12)
        y += 22
        for tag in grid["tags"]:
            move = man["interact"]["moves"][tag["name"]]
            x = PAD
            label(d, (x, y), f"{tag['name']}: {move['total_ticks']} ticks ({round(move['total_ticks'] * 1000 / 60)} ms), "
                  f"contact t{move['contact_tick']}", WHITE, 12)
            y += 16
            top = y
            for fr in move["frames"]:
                idx = tag["from"] + move["frames"].index(fr)
                ticks = fr["ticks"][1] - fr["ticks"][0] + 1
                cw = max(w * K, ticks * TICK_PX + 4)
                sheet = cell(imgs[idx], K, grid["pivot"], grid["pivot"][1])
                out.alpha_composite(sheet.convert("RGBA"), (x, top))
                label(d, (x + 2, top + h * K + 2), f"{fr['name'].split()[0]}{' KEY' if fr['key'] else ''}", YELLOW if fr["key"] else GREY, 11)
                label(d, (x + 2, top + h * K + 15), f"t{fr['ticks'][0]}-{fr['ticks'][1]}  {ticks}t", GREY, 11)
                bar_x = x
                col = PHASE[fr["phase"]]
                d.rectangle([bar_x, top + h * K + 32, bar_x + ticks * TICK_PX - 1, top + h * K + 42], fill=col)
                if fr["phase"] == "contact":
                    d.rectangle([bar_x, top + h * K + 28, bar_x + 1, top + h * K + 46], fill=(255, 255, 255))
                x += max(w * K, ticks * TICK_PX) + 14
            y = top + h * K + 54
        y += 6
    label(d, (PAD, out.height - 22), "phase colours: blue startup, red contact, orange active, green recovery. Bar length = ticks x 5 px.", GREY, 11)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    out.convert("RGB").save(EVIDENCE / "timing_diagram.png")


def evidence() -> None:
    from PIL import Image
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import CARL_X, DONUT_X, grid_images
    from scripts.pixelart.scene import Floor

    carl = grid_images(json.loads(CARL_OUT.read_text(encoding="utf-8")))
    donut = grid_images(json.loads(DONUT_OUT.read_text(encoding="utf-8")))
    cpiv, dpiv = tuple(cm.LOCO["pivot"]), tuple(DONUT["pivot"])
    carl_idle = grid_images(cm.LOCO)[0]
    donut_idle = grid_images(DONUT)[0]
    P = _props()
    floor = Floor()
    EVIDENCE.mkdir(parents=True, exist_ok=True)

    # ---- keys at 1x next to idle Carl and Donut (design-phase evidence, unchanged: keys are frozen)
    xs = {"u1": 500, "u2": 556, "p1": 612, "s1": 668, "d1": 744, "n1": 816}
    c_idx = {"u1": 1, "u2": 2, "p1": 5, "s1": 9}
    d_idx = {"d1": 1, "n1": 5}
    entries = [(donut_idle, dpiv, DONUT_X, 0), (carl_idle, cpiv, CARL_X, 0)]
    entries += [_fixed(P["lever_up"], xs["u1"], 17), _fixed(P["lever_down"], xs["u2"], 11),
                _fixed(P["item"], xs["p1"], 15), _fixed(P["crate"], xs["s1"], 20),
                _fixed(P["low_switch"], xs["d1"], 18), _fixed(P["item"], xs["n1"], 21)]
    entries += [(carl[c_idx[k]], cpiv, xs[k], 0) for k in c_idx]
    entries += [(donut[d_idx[k]], dpiv, xs[k], 0) for k in d_idx]
    full = floor.render(entries, 1)
    vx, vy = floor.view[0], floor.view[1]
    left, gy = DONUT_X - 40 - vx, floor.ground(CARL_X) - vy
    box = (max(0, left), max(0, gy - 70), min(full.width, left + 480), min(full.height, gy + 14))
    one = full.crop(box)
    one.save(EVIDENCE / "keys-scale.png")
    one.resize((one.width * 4, one.height * 4), Image.NEAREST).save(EVIDENCE / "keys-scale-x4.png")

    # ---- Carl: idle plus all 12 frames, each verb with its placeholder prop at the stand-off distance
    step, x0 = 48, 346
    entries = [(carl_idle, cpiv, x0, 0)]
    for i in range(12):
        x = x0 + step * (i + 1)
        entries.append((carl[i], cpiv, x, 0))
        if i < 4:        # hand: handle up until the grip, thrown from the haul on
            entries.append(_fixed(P["lever_up"] if i < 2 else P["lever_down"], x, 17 if i < 2 else 11))
        elif i < 8:      # floor: item on the floor until it is picked up, then it rides in the far hand
            if i < 6:
                entries.append(_fixed(P["item"], x, 15))
            else:
                ax, ay = json.loads(CONFIGS_CARL.read_text(encoding="utf-8"))["anchors"]["carried_item_hand"][str(i)]
                entries.append((P["item"], (3, 2), x + ax, ay, False))
        else:            # push: the crate gives 2 px after the contact tick (s2, s3)
            entries.append(_fixed(P["crate"], x, 20 + (2 if i >= 10 else 0)))
    _scene(floor, entries, "carl_set_floor1")

    # ---- Donut: idle plus all 8 frames, idle Carl on the right for scale
    step, x0 = 58, 346
    entries = [(donut_idle, dpiv, x0, 0)]
    for i in range(8):
        x = x0 + step * (i + 1)
        entries.append((donut[i], dpiv, x, 0))
        if i < 4:
            entries.append(_fixed(P["low_switch"], x, 18))
        else:            # nudge: the item gives 2 px after the contact tick (n2, n3)
            entries.append(_fixed(P["item"], x, 21 + (2 if i >= 6 else 0)))
    entries.append((carl_idle, cpiv, x0 + step * 9 + 8, 0))
    _scene(floor, entries, "donut_set_floor1")
    _timing_diagram()
    print(f"wrote evidence in {EVIDENCE.relative_to(ROOT)}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grids equal the build output")
    ap.add_argument("--evidence", action="store_true", help="render the scene and timing evidence")
    a = ap.parse_args(argv)
    outs = outputs()
    if a.check:
        bad = [p for p, text in outs.items() if not p.exists() or p.read_text(encoding="utf-8") != text]
        for p in bad:
            print(f"stale: {p.relative_to(ROOT)}")
        return 1 if bad else 0
    if a.evidence:
        evidence()
        return 0
    for p, text in outs.items():
        p.write_text(text, encoding="utf-8")
        print(f"wrote {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
