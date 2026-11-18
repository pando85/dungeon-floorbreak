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
TICK_HZ = 60

# ------------------------------------------------------------------ frame plans (spec section 3)
# (tag, [(frame name, ticks, key it copies or None if it is a key)], contact frame name)
CARL_PLAN = [
    ("interact_hand", [("u0", 5, "u1"), ("u1", 8, None), ("u2", 10, None), ("u3", 8, "u2")], "u1"),
    ("interact_floor", [("p0", 8, "p1"), ("p1", 10, None), ("p2", 10, "p1"), ("p3", 8, "p1")], "p1"),
    ("interact_push", [("s0", 6, "s1"), ("s1", 8, None), ("s2", 12, "s1"), ("s3", 8, "s1")], "s1"),
]
DONUT_PLAN = [
    ("interact_paw", [("d0", 6, "d1"), ("d1", 6, None), ("d2", 8, "d1"), ("d3", 6, "d1")], "d1"),
    ("interact_nudge", [("n0", 8, "n1"), ("n1", 8, None), ("n2", 10, "n1"), ("n3", 8, "n1")], "n1"),
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


CARL_KEYS = {"u1": carl_u1, "u2": carl_u2, "p1": carl_p1, "s1": carl_s1}

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


DONUT_KEYS = {"d1": donut_d1, "n1": donut_n1}

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


def evidence() -> None:
    from PIL import Image, ImageDraw
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import CARL_X, DONUT_X, grid_images
    from scripts.pixelart.scene import Floor

    carl = grid_images(json.loads(CARL_OUT.read_text(encoding="utf-8")))
    donut = grid_images(json.loads(DONUT_OUT.read_text(encoding="utf-8")))
    cpiv, dpiv = tuple(cm.LOCO["pivot"]), tuple(DONUT["pivot"])
    carl_idle = grid_images(cm.LOCO)[0]
    donut_idle = grid_images(DONUT)[0]

    def prop(w, h, draw_fn):
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        draw_fn(ImageDraw.Draw(img))
        return img

    iron, iron_d, wood, wood_d = (88, 86, 98, 255), (34, 30, 40, 255), (112, 84, 58, 255), (46, 32, 26, 255)
    # placeholder props (context only, not art): lever post with the handle at grip height, low switch, crate, item
    lever_up = prop(6, 32, lambda d: (d.rectangle([2, 4, 4, 31], fill=iron, outline=iron_d),
                                      d.rectangle([0, 0, 5, 5], fill=iron, outline=iron_d)))
    lever_down = prop(9, 32, lambda d: (d.rectangle([5, 4, 7, 31], fill=iron, outline=iron_d),
                                        d.rectangle([0, 7, 5, 12], fill=iron, outline=iron_d)))
    crate = prop(22, 32, lambda d: (d.rectangle([0, 0, 21, 31], fill=wood, outline=wood_d),
                                    d.line([0, 0, 21, 31], fill=wood_d), d.line([0, 31, 21, 0], fill=wood_d)))
    item = prop(6, 5, lambda d: d.rectangle([0, 0, 5, 4], fill=iron, outline=iron_d))
    low_switch = prop(5, 5, lambda d: d.rectangle([0, 0, 4, 4], fill=iron, outline=iron_d))

    def fixed(img, x, piv_x):  # prop whose bottom row sits on the walking line, left edge at pivot-relative x
        return (img, (piv_x, img.height - 1), x, 0, False)

    xs = {"u1": 500, "u2": 556, "p1": 612, "s1": 668, "d1": 744, "n1": 816}
    c_idx = {"u1": 1, "u2": 2, "p1": 5, "s1": 9}
    d_idx = {"d1": 1, "n1": 5}
    entries = [(donut_idle, dpiv, DONUT_X, 0), (carl_idle, cpiv, CARL_X, 0)]
    entries += [fixed(lever_up, xs["u1"], -17), fixed(lever_down, xs["u2"], -11),
                fixed(item, xs["p1"], -15), fixed(crate, xs["s1"], -20),
                fixed(low_switch, xs["d1"], -18), fixed(item, xs["n1"], -21)]
    entries += [(carl[c_idx[k]], cpiv, xs[k], 0) for k in c_idx]
    entries += [(donut[d_idx[k]], dpiv, xs[k], 0) for k in d_idx]
    floor = Floor()
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    full = floor.render(entries, 1)
    vx = floor.view[0] if isinstance(floor.view, list) else floor.view.get("x", 0)
    left = DONUT_X - 40 - (vx if isinstance(vx, int) else 0)
    gy = floor.ground(CARL_X) - (floor.view[1] if isinstance(floor.view, list) else floor.view.get("y", 0))
    box = (max(0, left), max(0, gy - 70), min(full.width, left + 480), min(full.height, gy + 14))
    one = full.crop(box)
    one.save(EVIDENCE / "keys-scale.png")
    one.resize((one.width * 4, one.height * 4), Image.NEAREST).save(EVIDENCE / "keys-scale-x4.png")
    print(f"wrote {EVIDENCE / 'keys-scale.png'} {one.size} and the x4 copy (crop {box} of {full.size})")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grids equal the build output")
    ap.add_argument("--evidence", action="store_true", help="render the key scale evidence")
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
