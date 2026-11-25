#!/usr/bin/env python3
"""DONUT-MOVES-ART-01 execution: the production sheets of Donut's reflex cues and Puddle Jumper.

Spec: docs/02-art/donut-moves-art-01-spec.md (authoritative). The two hero keys (``evade_twist`` and ``blink_arrive``)
are drawn by ``scripts/author_donut_moves_keys.py`` and frozen; this script imports their builders, so frames e2 and
a1 of the moves sheet are byte-identical to the keys, and draws every other frame by rule from the keys, the approved
locomotion, melee and CF02 cels and the key's helpers (``chain``, ``shade``, ``paint``, ``fur_fringe``, ``cleanup``).
Nothing is traced or taken from an image model: literal pixel edits and float rules, no randomness.

Outputs (all deterministic, ``--check`` verifies them):

* ``assets/source/characters/donut/donut_h40_moves.grid.json`` (13 frames, 72 x 44, pivot (46, 43)): evade e0-e4,
  blink b0-b2 and a0-a2, fizzle f0-f1.
* ``assets/source/characters/donut/donut_h40_tail_ready.grid.json`` (32 frames): the locomotion atlas with only the
  ``tail`` cel changed, ``16 * phase + locomotion frame``, phase 0 ``flick_up`` and phase 1 ``flick_snap``
  (adopted default DMA-Q10: a separate sheet, so the index rule is the missile head swap's).
* ``assets/source/vfx/puddle_jumper/vfx_puddle_jumper.grid.json`` (16 frames, 48 x 16, pivot (24, 11)).
* The two ``.pixelart.json`` configs and the three manifests under ``assets/manifests``.

Usage:
  python3 scripts/author_donut_moves.py keys [--check]    write (or check) grids, configs and manifests
  python3 scripts/author_donut_moves.py --check           the same check (what CI runs)
  python3 scripts/author_donut_moves.py manifests [--check]
  python3 scripts/author_donut_moves.py evidence          docs/evidence/donut-moves-art-01 sheets and GIFs
                                                          (needs `bash scripts/export_assets.sh` first)
"""
from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from collections import Counter, deque
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))

import author_donut_moves_keys as K  # noqa: E402  (the frozen keys and their helpers)
from author_donut_moves_keys import capsule, chain, cleanup, fur_fringe, paint, pix, shade, shift  # noqa: E402

D = ROOT / "assets/source/characters/donut"
LOCO_GRID = D / "donut_h40_locomotion.grid.json"
MELEE_GRID = D / "donut_h40_melee.grid.json"
CF02_GRID = D / "donut_h40_cf02.grid.json"
CF02_KEYS_GRID = D / "cf02_keys/donut_h40_cf02_keys.grid.json"
MOVES_KEYS_GRID = K.OUT
MOVES_GRID = D / "donut_h40_moves.grid.json"
MOVES_CFG = D / "donut_h40_moves.pixelart.json"
MOVES_MANIFEST = ROOT / "assets/manifests/donut_h40_moves.json"
TAIL_GRID = D / "donut_h40_tail_ready.grid.json"
TAIL_CFG = D / "donut_h40_tail_ready.pixelart.json"
TAIL_MANIFEST = ROOT / "assets/manifests/donut_h40_tail_ready.json"
VFX_GRID = ROOT / "assets/source/vfx/puddle_jumper/vfx_puddle_jumper.grid.json"
VFX_CFG = ROOT / "assets/source/vfx/puddle_jumper/vfx_puddle_jumper.pixelart.json"
VFX_MANIFEST = ROOT / "assets/manifests/vfx_puddle_jumper.json"
MISSILE_VFX_GRID = ROOT / "assets/source/vfx/magic_missile/vfx_magic_missile.grid.json"
KEY_FIXTURE = "tests/fixtures/donut-moves-art-01_keys.json"
EVIDENCE = ROOT / "docs/evidence/donut-moves-art-01"

TICK_HZ = 60
W, H = K.W, K.H
PIVOT = K.PIVOT
LAYERS = K.LAYERS
LOCO_LAYERS = ["tail", "legs_far", "body", "legs_near", "head", "face", "gear"]
N_LOCO = 16

# ---------------------------------------------------------------------------- frame table (spec sections 4 and 5)
# (name, tag, ticks, author)
MOVES = [
    ("evade_coil", "evade", 2, "execution"),
    ("evade_arch", "evade", 3, "execution"),
    ("evade_twist", "evade", 3, "key"),
    ("evade_untwist", "evade", 2, "execution"),
    ("evade_land", "evade", 2, "execution"),
    ("blink_crouch", "blink_depart", 3, "execution"),
    ("blink_dip", "blink_depart", 2, "execution"),
    ("blink_vanish", "blink_depart", 1, "execution"),
    ("blink_pop", "blink_arrive", 2, "execution"),
    ("blink_arrive", "blink_arrive", 3, "key"),
    ("blink_settle", "blink_arrive", 2, "execution"),
    ("fizzle_bonk", "fizzle", 4, "execution"),
    ("fizzle_huff", "fizzle", 4, "execution"),
]
MOVE_KEYS = (2, 9)
# Pockets closed on purpose (review F4, F6, F8): the collar gaps of e0 and e4, the pinholes of e4, b1 (every small
# pocket) and a2. The pockets inherited from the approved cels (the legs gap of e0 and b0, the key's tail hook) stay.
FILL = {0: ("body", [(25, 28), (47, 33)]), 4: ("body", [(45, 30), (16, 24)]), 6: ("body", None),
        10: ("body", [(41, 41), (44, 41)])}
TAG_CONTACT = {"evade": "free", "blink_depart": "grounded", "blink_arrive": "grounded", "fizzle": "grounded"}
TAG_PLAYBACK = {"evade": "once", "blink_depart": "once", "blink_arrive": "once", "fizzle": "once"}
PHASES = ("flick_up", "flick_snap")
ACCENTS = {"M", "N", "P", "w"}     # the locomotion palette's accent keys (exempt from the orphan rule)
N8 = [(dx, dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
N4 = [(1, 0), (-1, 0), (0, 1), (0, -1)]


# ---------------------------------------------------------------------------- generic helpers


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def lerp(a, b, t):
    return a + (b - a) * t


def lerp_pose(a, b, t):
    """Interpolate two poses (same keys, same lengths): every number, nested lists and tuples included."""
    if isinstance(a, (int, float)):
        return lerp(a, b, t)
    return [lerp_pose(x, y, t) for x, y in zip(a, b)]


def anchor_down(layers: dict[str, dict], lowest: int = 43) -> None:
    """Shift every layer so the lowest opaque row is ``lowest`` (the airborne rule: lowest-row anchored)."""
    bottom = max(y for layer in layers.values() for (_x, y) in layer)
    if bottom != lowest:
        for name in list(layers):
            layers[name] = shift(layers[name], 0, lowest - bottom)


def composite(layers: dict[str, dict]) -> dict:
    out: dict = {}
    for name in LAYERS:
        out.update(layers.get(name, {}))
    return out


def pockets(comp: dict) -> list[set]:
    """Enclosed transparent pockets (4-connected, not touching the canvas border) of a composite."""
    seen, out = set(), []
    for y in range(H):
        for x in range(W):
            if (x, y) in comp or (x, y) in seen:
                continue
            cells, stack, edge = set(), [(x, y)], False
            seen.add((x, y))
            while stack:
                cx, cy = stack.pop()
                cells.add((cx, cy))
                for dx, dy in N4:
                    q = (cx + dx, cy + dy)
                    if not (0 <= q[0] < W and 0 <= q[1] < H):
                        edge = True
                    elif q not in comp and q not in seen:
                        seen.add(q)
                        stack.append(q)
            if not edge:
                out.append(cells)
    return out


def fill_pockets(layers: dict[str, dict], layer: str, seeds=None, max_size: int = 24) -> None:
    """Close enclosed transparent pockets (the ones holding a seed cell, or every pocket of at most ``max_size``
    cells when ``seeds`` is None) in ``layer``, each cell taking the most common opaque non-accent colour around it
    (repeated from the pocket's rim inward)."""
    comp = composite(layers)
    for cells in pockets(comp):
        if len(cells) > max_size or (seeds is not None and not cells & set(seeds)):
            continue
        todo = set(cells)
        while todo:
            progress = False
            for (x, y) in sorted(todo):
                votes = Counter(comp[(x + dx, y + dy)] for dx, dy in N8
                                if (x + dx, y + dy) in comp and comp[(x + dx, y + dy)] not in ACCENTS)
                if votes:
                    c = votes.most_common(1)[0][0]
                    layers.setdefault(layer, {})[(x, y)] = c
                    comp[(x, y)] = c
                    todo.discard((x, y))
                    progress = True
            if not progress:
                break


def finish(layers: dict[str, dict], accents: set[str], anchor: bool = True, fill=None) -> dict:
    """Clean orphans, anchor, and turn the layers into a grid frame (every layer name of the sheet). ``fill`` is
    ``(layer, seeds)`` for ``fill_pockets`` (``seeds`` None: every small pocket)."""
    layers = {n: dict(p) for n, p in layers.items() if p}
    if fill is not None:
        fill_pockets(layers, fill[0], fill[1], 3 if fill[1] is None else 24)
    cleanup(layers, accents)
    if anchor:
        anchor_down(layers)
    return {n: K.cel(layers[n]) for n in LAYERS if layers.get(n)}


def cel_pixels(frame_cels: dict, name: str) -> dict:
    return pix(frame_cels[name]) if frame_cels.get(name) else {}


def compress_rows(p: dict, at: int, rows: int) -> dict:
    """Delete ``rows`` rows starting at ``at`` and move everything above them down by ``rows`` (compression)."""
    out = {}
    for (x, y), c in p.items():
        if y < at:
            out[(x, y + rows)] = c
        elif y >= at + rows:
            out[(x, y)] = c
    return out


def merge_over(*layers: dict) -> dict:
    out: dict = {}
    for layer in layers:
        out.update(layer)
    return out


# ---------------------------------------------------------------------------- evade (spec section 4)

# Control points of the key's capsule chains (author_donut_moves_keys.evade_twist) and of the two poses they are
# interpolated toward (the coil and the landing). Every pose lists the same points in the same order.
KEY_POSE = {
    "back": [(44, 31, 7.0), (38, 22, 8.5), (31, 16, 9.0), (24, 18, 8.5), (20, 25, 7.0)],
    "tail": [(13, 22, 3.5), (8, 15, 4.5), (8, 9, 4.5), (13, 6, 3.5), (19, 6, 2.5), (22, 8, 1.5)],
}
COIL_POSE = {
    "back": [(44, 34, 7.0), (38, 31, 8.0), (31, 30, 8.5), (24, 31, 8.0), (20, 34, 7.0)],
    "tail": [(13, 30, 3.5), (9, 27, 4.5), (8, 23, 4.5), (8, 20, 4.0), (9, 18, 3.2), (10, 17, 2.4)],
}
LAND_POSE = {
    "back": [(44, 33, 7.0), (38, 30, 8.0), (31, 29, 8.5), (24, 30, 8.0), (20, 33, 7.0)],
    "tail": [(14, 28, 3.5), (10, 22, 4.5), (9, 16, 4.5), (10, 12, 3.8), (13, 10, 2.8), (15, 9, 2.0)],
}


def to_points(pose: list) -> list:
    return [tuple(p) for p in pose]


def evade_body(loco: dict, *, back, tail, head_dy: int, seam: int, thigh_cream: bool, hind, front_lift: int) -> dict:
    """An arch frame built the way the key is (shared helpers and shading), from control-point poses.

    ``seam`` is the width of the cream belly seam (0 none), ``thigh_cream`` whether the near hind leg shows its cream
    inner side, ``hind`` the two hind legs as capsule chains, ``front_lift`` how many px the stubby front paws are
    raised off the floor."""
    src = loco["frames"][K.JUMP_RISE]["cels"]
    L: dict[str, dict] = {}
    L["head"] = shift(pix(src["head"]), 0, head_dy)
    L["face"] = shift(pix(src["face"]), 0, head_dy)
    L["gear"] = shift(pix(src["gear"]), 0, head_dy)
    back_mask = chain(to_points(back))
    body = shade(back_mask, "B", "H", "D")
    p = back[2][1]          # the spine peak: the coat patches follow it so they stay on the hump as it moves
    paint(body, chain([(35, p - 4, 2.5), (40, p + 1, 2.0)]), "O")
    paint(body, chain([(34, p - 6, 1.0), (37, p - 4, 1.0)]), "G")
    paint(body, chain([(39, p + 3, 1.0), (41, p + 3, 1.0)]), "o")
    paint(body, chain([(22, p - 4, 2.0), (25, p - 5, 1.5)]), "O")
    if seam:
        # The rolled belly: a cream seam `seam` px thick along the underside of the rear half (never wider).
        belly = {(x, y) for (x, y) in back_mask
                 if x <= 34 and any((x, y + k) not in back_mask for k in range(1, seam + 1))}
        paint(body, belly, "C")
        paint(body, {(x, y) for (x, y) in belly if (x, y + 1) not in back_mask}, "c")
    chest = chain([(47, 37, 3.5), (58, 37, 3.5)])
    bib = shade(chest, "C", "W", "c", top_soft="c", hard="K", dark_rows=1)
    for p, c in bib.items():
        if p not in body or body[p] in ("K", "j", "D"):
            body[p] = c
    L["body"] = body

    far_pts, near_pts = hind
    hind_far = chain(to_points(far_pts))
    far = shade(hind_far, "B", "H", "D", dark_rows=1)
    paint(far, {(x, y) for (x, y) in hind_far if y >= max(q[1] for q in far_pts) - 2}, "c")
    ff = chain([(47, 37 - front_lift, 2.3), (47, 41 - front_lift, 2.0)])
    far.update(shade(ff, "c", "C", "c", top_soft="c", dark_rows=1))
    L["legs_far"] = far
    hind_near = chain(to_points(near_pts))
    if thigh_cream:
        near = shade(hind_near, "C", "W", "c", dark_rows=1)
    else:
        near = shade(hind_near, "B", "H", "D", dark_rows=1)
        paint(near, {(x, y) for (x, y) in hind_near if y >= max(q[1] for q in near_pts) - 2}, "c")
    fn = chain([(57, 37 - front_lift, 2.4), (57, 41 - front_lift, 2.1)])
    near.update(shade(fn, "C", "W", "c", top_soft="c", dark_rows=1))
    L["legs_near"] = near

    tail_mask = chain(to_points(tail))
    t = shade(tail_mask, "O", "G", "o", dark_rows=1)
    paint(t, chain([(tail[1][0] - 1, tail[1][1], 2.0), (tail[1][0] + 2, tail[1][1] - 1, 1.5)]), "B")
    paint(t, chain([(tail[2][0] + 1, tail[2][1], 1.5), (tail[2][0] + 4, tail[2][1] - 1, 1.0)]), "B")
    fur_fringe(t, tail_mask, "G", "left", every=2)
    fur_fringe(t, tail_mask, "C", "up", every=3)
    L["tail"] = t
    return L


def evade_coil(loco: dict, melee: dict) -> dict:
    """e0: the melee ``p0`` body compressed 2 px, with the ``jump_rise`` head ducked 3 px."""
    p0 = melee["frames"][4]["cels"]
    jr = loco["frames"][K.JUMP_RISE]["cels"]
    L: dict[str, dict] = {}
    for name in ("body", "legs_far", "legs_near"):
        p = cel_pixels(p0, name)
        L[name] = compress_rows(p, 30, 2) if name == "body" else p
    # The compression drops the body 2 px; the legs stay on the floor row; the tail keeps its root on the body.
    tail = shift(cel_pixels(p0, "tail"), 0, 2)
    # A fat, low, bottle-brushed tail: the p0 tail with a fringe of fur along its top and tip.
    fur_fringe(tail, set(tail), "G", "up", every=3)
    fur_fringe(tail, set(tail), "G", "left", every=2)
    L["tail"] = tail
    # Head: jump_rise head ducked 3 px, one column left so it sits on the p0 body.
    for name in ("head", "face", "gear"):
        L[name] = shift(pix(jr[name]), -1, 3)
    return L


def evade_arch(loco: dict) -> dict:
    """e1: the hump rises, head ducked as in the key, hind legs gathered under the rump, tail straight up."""
    t = 0.7
    back = lerp_pose(COIL_POSE["back"], KEY_POSE["back"], t)
    tail = [(13, 23, 3.5), (9, 17, 4.5), (8, 11, 4.5), (8, 6, 3.6), (9, 4, 2.6), (10, 3, 1.6)]
    hind = ([(24, 32, 2.6), (25, 40, 2.1)], [(20, 30, 4.4), (22, 35, 3.4), (24, 40, 2.4)])
    return evade_body(loco, back=back, tail=tail, head_dy=7, seam=0, thigh_cream=False, hind=hind,
                      front_lift=2)


def evade_untwist(loco: dict) -> dict:
    """e3: the rear rolls back (1-2 px seam, dark thigh), both hind legs reach down, the hump drops 3 px, the tail
    hook opens to a question mark."""
    back = lerp_pose(KEY_POSE["back"], LAND_POSE["back"], 0.25)
    tail = [(13, 24, 3.5), (8, 18, 4.5), (7, 12, 4.5), (9, 7, 3.6), (14, 5, 2.6), (17, 6, 1.6)]
    hind = ([(24, 32, 2.6), (25, 41, 2.1)], [(19, 31, 4.2), (18, 36, 3.2), (18, 41, 2.3)])
    return evade_body(loco, back=back, tail=tail, head_dy=5, seam=2, thigh_cream=False, hind=hind,
                      front_lift=1)


def evade_land(loco: dict) -> dict:
    """e4: the locomotion ``land`` frame (14), its tail the key's puffed bottle-brush lowered, the head back up at
    the ``jump_fall`` height."""
    land = loco["frames"][14]["cels"]
    fall = loco["frames"][13]["cels"]
    L: dict[str, dict] = {}
    for name in ("legs_far", "body", "legs_near"):
        L[name] = cel_pixels(land, name)
    tail = cel_pixels(land, "tail")
    fur_fringe(tail, set(tail), "G", "left", every=2)
    fur_fringe(tail, set(tail), "C", "up", every=3)
    L["tail"] = tail
    for name in ("head", "face", "gear"):
        L[name] = cel_pixels(fall, name)
    return L


# ---------------------------------------------------------------------------- Puddle Jumper (spec section 5)

PUDDLE = K.PUDDLE                  # the key's puddle mouth: (cx, cy, rx, ry), centred under her
WATERLINE = K.WATERLINE


def puddle_mouth(cx: float, cy: float, rx: float, ry: float, waterline: int, behind: set | None = None,
                 back_only: bool = False, splash: bool = False) -> tuple[dict, dict]:
    """The key's puddle mouth rule on any ellipse: ``(puddle_back, puddle_front)`` layers.

    The back half (lilac rim, violet bank, plum water) sits behind her; the front half (plum water, magenta lip, plum
    bottom edge) covers her lower body; ``waterline`` is the first row hidden in the puddle, a lilac meniscus marks
    where her body (``behind``) breaks the surface. ``back_only`` draws just the rows above the centre (the opening
    of the crouch). Rows below the canvas are clipped."""
    disc = {(x, y) for y in range(H) for x in range(W)
            if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0}
    behind = behind or set()
    back, front = {}, {}
    full = {(x, y) for y in range(-4, H + 6) for x in range(W)
            if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0}
    for (x, y) in disc:
        if (x, y - 1) not in full:
            back[(x, y)] = "L"
        elif (x, y - 2) not in full:
            back[(x, y)] = "V"
        elif y < waterline - 1 or (back_only and y <= cy):
            back[(x, y)] = "D"
        elif back_only:
            continue
        elif (x, y + 1) not in full:
            front[(x, y)] = "K"
        elif (x, y + 2) not in full:
            front[(x, y)] = "M"
        elif y == waterline - 1 and (x, y - 1) in behind:
            front[(x, y)] = "L"
        else:
            front[(x, y)] = "D"
    if splash and behind:
        edge = sorted(x for (x, y) in behind if y == waterline - 1)
        if edge:
            lo, hi = edge[0], edge[-1]
            for (x, y) in ((lo - 1, waterline - 2), (lo - 1, waterline - 3), (lo - 2, waterline - 2),
                           (hi + 1, waterline - 2), (hi + 1, waterline - 3), (hi + 2, waterline - 2)):
                front[(x, y)] = "L"
    return back, front


def blink_crouch(melee: dict) -> dict:
    """b0: the melee ``p0`` crouch, head tilted down 2 px, tail tip up and curled, the puddle mouth opening under
    her front paws (``puddle_back`` only, a thin rim)."""
    p0 = melee["frames"][4]["cels"]
    L: dict[str, dict] = {}
    for name in ("legs_far", "body", "legs_near"):
        L[name] = cel_pixels(p0, name)
    for name in ("head", "face", "gear"):
        L[name] = shift(cel_pixels(p0, name), 0, 2)
    # Smug, not tense: the tail trails low and the tip curls up (a J).
    mask = chain([(28, 24, 3.2), (22, 25, 3.6), (16, 24, 3.4), (11, 20, 3.0), (11, 16, 2.4), (14, 14, 1.6)])
    tail = shade(mask, "O", "G", "o", dark_rows=1)
    paint(tail, chain([(17, 24, 1.6), (19, 24, 1.2)]), "B")
    paint(tail, chain([(11, 19, 1.6)]), "B")
    L["tail"] = tail
    back, _front = puddle_mouth(46, 41.5, 21.0, 3.4, 99, back_only=True)
    L["puddle_back"] = back
    return L


def dive_body(spine, tail, thigh, patches: bool = True) -> dict:
    """A nose-first dive: the spine slopes from a raised rump into the puddle; hind legs trail, the tail stands."""
    L: dict[str, dict] = {}
    mask = chain(spine)
    body = shade(mask, "B", "H", "D")
    if patches:
        # Calico as on the e1 and key bodies: a big ginger haunch patch with a light edge and a shadow, two more
        # patches down the back, a fur fringe on the rump's outline.
        (x0, y0, _r0), (x1, y1, _r1), (x2, y2, _r2) = spine[0], spine[1], spine[2]
        paint(body, chain([(x0 + 1, y0 - 1, 4.4), (x0 + 5, y0 + 3, 3.4)]), "O")
        paint(body, chain([(x0 - 2, y0 - 3, 1.6), (x0 + 2, y0 - 4, 1.2)]), "G")
        paint(body, chain([(x0 + 2, y0 + 4, 1.4), (x0 + 6, y0 + 6, 1.2)]), "o")
        paint(body, chain([(x1 + 4, y1 + 2, 2.6), (x2 + 1, y2 - 1, 2.0)]), "O")
        paint(body, chain([(x1 + 3, y1 + 1, 1.0), (x2, y2 - 2, 1.0)]), "G")
        paint(body, chain([(x2 + 3, y2 + 4, 2.2), (spine[3][0], spine[3][1] - 1, 1.6)]), "O")
        fur_fringe(body, {q for q in mask if q[0] < x1 + 2}, "G", "left", every=3)
    L["body"] = body
    # Hind legs trail behind the rump, dark like the coat with a cream paw (the rear is not rolled toward the camera).
    near_mask = chain(thigh)
    near = shade(near_mask, "B", "H", "D", dark_rows=1)
    tip = max(near_mask, key=lambda p: (p[1], -p[0]))
    paint(near, {(x, y) for (x, y) in near_mask if (x - tip[0]) ** 2 + (y - tip[1]) ** 2 <= 5}, "c")
    L["legs_near"] = near
    far_mask = chain([(thigh[0][0] + 5, thigh[0][1] - 2, 3.6), (thigh[1][0] + 5, thigh[1][1] + 1, 2.6),
                      (thigh[2][0] + 5, thigh[2][1] + 2, 2.0)])
    far = shade(far_mask, "B", "H", "D", dark_rows=1)
    ftip = max(far_mask, key=lambda p: (p[1], -p[0]))
    paint(far, {(x, y) for (x, y) in far_mask if (x - ftip[0]) ** 2 + (y - ftip[1]) ** 2 <= 5}, "c")
    L["legs_far"] = far
    tmask = chain(tail)
    t = shade(tmask, "O", "G", "o", dark_rows=1)
    paint(t, chain([tail[1][:2] + (1.8,), (tail[2][0], tail[2][1], 1.5)]), "B")
    fur_fringe(t, tmask, "G", "left", every=2)
    L["tail"] = t
    return L


DIVE_SPINE = [(31, 21, 10.0), (35, 25, 9.0), (39, 30, 8.2), (43, 35, 7.6), (47, 40, 7.0)]
DIVE_TAIL = [(30, 14, 3.5), (27, 10, 3.8), (26, 6, 3.4), (29, 3, 2.6), (32, 3, 1.6)]
DIVE_THIGH = [(29, 27, 4.4), (26, 31, 3.2), (23, 34, 2.4)]


def blink_dip(_loco: dict) -> dict:
    """b1: head and shoulders under the front rim, rump and tail up; the puddle at full width."""
    cx, cy, rx, ry = PUDDLE
    L = dive_body(DIVE_SPINE, DIVE_TAIL, DIVE_THIGH)
    for name in ("body", "legs_near", "legs_far", "tail"):
        L[name] = {p: c for p, c in L[name].items() if p[1] < WATERLINE}
    behind = set(L["body"]) | set(L["tail"])
    L["puddle_back"], L["puddle_front"] = puddle_mouth(cx, cy, rx, ry, WATERLINE, behind)
    return L


def blink_vanish() -> dict:
    """b2: only the tail tip (about 8 px) above the rim, a lilac meniscus where it enters, the puddle."""
    cx, cy, rx, ry = PUDDLE
    mask = {p for p in chain([(44, 37.5, 2.6), (44, 33, 2.4), (44, 29.5, 2.2), (46, 27.5, 1.4)]) if p[1] < WATERLINE}
    tip = shade(mask, "O", "G", "o", dark_rows=1)
    paint(tip, {(x, y) for (x, y) in mask if y in (31, 32)}, "B")
    back, front = puddle_mouth(cx, cy, rx, ry, WATERLINE, set(tip))
    for dx in (-3, -2, 2, 3):
        front[(44 + dx, WATERLINE - 1)] = "L"
    return {"tail": tip, "puddle_back": back, "puddle_front": front}


def blink_pop(cf_keys: dict) -> dict:
    """a0: the key's smug head raised only far enough for ears, tiara and eyes to clear the rim (the waterline moved
    up the face): the puddle at full width, her body entirely under."""
    cx, cy, rx, ry = PUDDLE
    src = cf_keys["frames"][1]["cels"]
    drop = WATERLINE - 22           # smug rows 0..21 (ears, tiara, eyes) end on the waterline
    L: dict[str, dict] = {}
    for name in ("head", "face", "gear"):
        L[name] = {p: c for p, c in shift(pix(src[name]), 0, drop).items() if p[1] < WATERLINE}
    L["gear"] = {p: c for p, c in L["gear"].items() if p[1] < WATERLINE - 14}   # tiara only: the collar is under
    behind = set(L["head"])
    L["puddle_back"], L["puddle_front"] = puddle_mouth(cx, cy, rx, ry, WATERLINE, behind, splash=True)
    return L


def blink_settle(cf_keys: dict) -> dict:
    """a2: the CF02 ``smug`` key unshifted, the puddle mouth shrunk to a 2-row lilac and magenta ring, half its width."""
    src = cf_keys["frames"][1]["cels"]
    L = {name: pix(src[name]) for name in LOCO_LAYERS if src.get(name)}
    cx, _cy, rx, _ry = PUDDLE
    half = round(rx / 2)
    ring = {}
    for x in range(round(cx) - half + 1, round(cx) + half):
        ring[(x, 42)] = "L"
    for x in range(round(cx) - half, round(cx) + half + 1):
        ring[(x, 43)] = "M"
    L["puddle_front"] = ring
    return L


def fizzle_bonk(loco: dict, cf: dict) -> dict:
    """f0: a half dip bounced back: head out (the CF02 ``startle_puff`` face), ears flat, one paw on the rim, the
    puddle at half width."""
    src = cf["frames"][0]["cels"]
    cx, cy, rx, ry = PUDDLE
    drop = 14
    L: dict[str, dict] = {}
    for name in ("head", "face", "gear"):
        L[name] = shift(pix(src[name]), 0, drop)
    # The b1 body, shallower: the rump up behind the head, the front under the rim.
    spine = [(22, 26, 7.0), (28, 28, 7.5), (34, 31, 7.5), (40, 34, 7.0), (45, 37, 6.5)]
    body = dive_body(spine, [(17, 22, 3.5), (12, 18, 4.0), (10, 13, 3.6), (13, 10, 2.6), (16, 10, 1.8)],
                     [(18, 29, 4.0), (12, 33, 2.8), (9, 36, 2.1)])
    for name, layer in body.items():
        L[name] = {p: c for p, c in layer.items() if p[1] < WATERLINE}
    behind = set(L["head"]) | set(L["body"])
    half = (cx + 4, cy, rx / 2, ry * 0.8)
    L["puddle_back"], L["puddle_front"] = puddle_mouth(*half, WATERLINE, behind)
    # One paw on the rim: a cream mitt drawn over the front rim (in puddle_front, so the rim does not cover it).
    paw = chain([(59, 34, 3.0), (59, 37, 3.0)])
    mitt = {}
    for (x, y) in paw:
        lower = (x, y + 1) not in paw
        outer = (x - 1, y) not in paw or (x + 1, y) not in paw
        mitt[(x, y)] = "K" if lower else ("c" if outer else ("W" if (x, y - 1) not in paw else "C"))
    L["puddle_front"] = merge_over(L["puddle_front"], mitt)
    return L


def fizzle_huff(cf: dict) -> dict:
    """f1: sitting (the CF02 ``startle_settle`` frame), ears flat, the tail lashed to one side. No puddle."""
    src = cf["frames"][1]["cels"]
    L = {name: pix(src[name]) for name in LOCO_LAYERS if src.get(name)}
    mask = chain([(28, 37, 3.4), (21, 39, 3.8), (14, 38, 3.8), (9, 35, 3.2), (8, 31, 2.4), (10, 28, 1.6)])
    tail = shade(mask, "O", "G", "o", dark_rows=1)
    paint(tail, chain([(18, 39, 1.8), (22, 39, 1.4)]), "B")
    paint(tail, chain([(9, 33, 1.5)]), "B")
    fur_fringe(tail, mask, "G", "up", every=3)
    fur_fringe(tail, mask, "G", "left", every=2)
    L["tail"] = tail
    return L


# ---------------------------------------------------------------------------- tail swap (spec section 3)

TIP_PIXELS = 8              # the tip is the 8 tail pixels farthest from the root by path distance
MAX_TAIL_CHANGES = 24       # at most this many pixels change per frame


def tail_distances(tail: dict, body: dict) -> tuple[tuple, dict]:
    """The tail root and every tail pixel's path distance from it along the tail mask (8-connected steps).

    The root is the tail pixel nearest the body's centroid (where the tail meets her)."""
    cx = sum(x for x, _y in body) / len(body)
    cy = sum(y for _x, y in body) / len(body)
    root = min(sorted(tail), key=lambda p: ((p[0] - cx) ** 2 + (p[1] - cy) ** 2, p[1], p[0]))
    dist = {root: 0}
    queue = deque([root])
    while queue:
        x, y = queue.popleft()
        for dx, dy in N8:
            q = (x + dx, y + dy)
            if q in tail and q not in dist:
                dist[q] = dist[(x, y)] + 1
                queue.append(q)
    return root, dist


def tail_tip(tail: dict, body: dict) -> set:
    """The 8 tail pixels farthest from the root by path distance (ties broken top-left first)."""
    _root, dist = tail_distances(tail, body)
    far = sorted(dist, key=lambda p: (-dist[p], p[1], p[0]))
    return set(far[:TIP_PIXELS])


def reclose_outline(tail: dict, before: dict) -> None:
    """Re-close the outline with ``K``: a fill pixel the swap newly exposed to empty space becomes plum."""
    for (x, y), c in sorted(tail.items()):
        if c in ("K", "j"):
            continue
        exposed = any((x + dx, y + dy) not in tail for dx, dy in N4)
        was_edge = (x, y) in before and any((x + dx, y + dy) not in before for dx, dy in N4)
        if exposed and not was_edge:
            tail[(x, y)] = "K"


def stem_colour(base: dict, x: int, y: int) -> str | None:
    """The colour of the first base pixel at or below ``(x, y)`` in its column (the stem a lifted tip leaves)."""
    for yy in range(y, y + 6):
        if (x, yy) in base:
            return base[(x, yy)]
    return None


def fur_cell(out: dict, moved: dict, original: dict) -> tuple | None:
    """Where the one extra fur pixel of the snap curl goes: an empty cell, not part of the base tail (a genuinely extra
    pixel), touching a ``G`` fur pixel so it is never an orphan. Preferred: on the curl's outer edge (above or to the
    back of the moved tip); else any such cell near the tip. The top-left-most cell of the first tier that has one;
    ``None`` if the curl has none."""
    top = min(y for (_x, y) in moved)
    left = min(x for (x, _y) in moved)
    outer, near = set(), set()
    for (x, y), c in out.items():
        if c != "G" or abs(x - left) > 10 or abs(y - top) > 10:
            continue
        for dx, dy in N8:
            q = (x + dx, y + dy)
            if q in out or q in original:
                continue
            near.add(q)
            if q[1] <= top or q[0] <= left:
                outer.add(q)
    pool = outer or near
    return min(pool, key=lambda q: ((q[0] - left) ** 2 + (q[1] - top) ** 2, q[1], q[0])) if pool else None


def tail_swap(tail: dict, body: dict, phase: str, reclose: bool = True) -> dict:
    """One flick phase of a locomotion tail (spec section 3).

    ``flick_up`` raises the tip 2 px and leans it 1 px back (away from the facing); the cells the lift opens under it
    take the stem's own colour so the tail stays one piece. ``flick_snap`` curls it 1 px forward and 1 px down, with
    one extra fur pixel (``G``) showing on the curl's outer edge. Vacated pixels are erased and the outline is
    re-closed with ``K``. Only the tip moves: the root half is untouched and at most 24 pixels change."""
    tip = tail_tip(tail, body)
    base = {p: c for p, c in tail.items() if p not in tip}
    out = dict(base)
    if phase == "flick_up":
        moved = {(x - 1, y - 2): tail[(x, y)] for (x, y) in tip}
        out.update(moved)
        for (x, y) in sorted(moved):
            for yy in (y + 1, y + 2):
                if (x, yy) not in out:
                    c = stem_colour(base, x, yy)
                    if c is not None:
                        out[(x, yy)] = c
    elif phase == "flick_snap":
        moved = {(x + 1, y + 1): tail[(x, y)] for (x, y) in tip}
        out.update(moved)
    else:
        raise ValueError(phase)
    if reclose:
        reclose_outline(out, tail)
    if phase == "flick_snap":
        cell = fur_cell(out, moved, tail)
        if cell is not None:
            out[cell] = "G"
    return out


def loco_ticks(frame: dict) -> int:
    """A locomotion frame's hold in 60 Hz ticks (the same quantisation as the missile cast)."""
    return max(1, round(frame["duration_ms"] * TICK_HZ / 1000))


def changed_pixels(a: dict, b: dict) -> int:
    return len({p for p in set(a) | set(b) if a.get(p) != b.get(p)})


def fix_tail_orphans(tail: dict, others: dict, accents: set) -> None:
    """The hand-fix of the outline: recolour every orphan of the ``tail`` layer (a visible pixel with no same-colour
    8-neighbour in the composite, the tail drawn under every other layer) to its most common neighbour. Only tail
    pixels are touched."""
    for _ in range(6):
        comp = dict(tail)
        comp.update(others)
        changed = False
        for (x, y) in sorted(tail):
            if (x, y) in others:
                continue                                  # hidden under the body
            c = tail[(x, y)]
            if c in accents:
                continue
            around = [comp.get((x + dx, y + dy)) for dx, dy in N8]
            if c in around:
                continue
            votes = Counter(n for n in around if n is not None and n != c)
            if votes:
                tail[(x, y)] = votes.most_common(1)[0][0]
                comp[(x, y)] = tail[(x, y)]
                changed = True
        if not changed:
            return


def close_new_tail_pockets(base_tail: dict, swapped: dict, others: dict) -> None:
    """The swap must not add enclosed transparent pockets (review F8: a pinhole on locomotion frame 8): any pocket of
    at most 2 cells that the base tail did not have is filled from its tail neighbours. Tail pixels only."""
    def comp(tail):
        out = dict(tail)
        out.update(others)
        return out
    before = {c for cells in pockets(comp(base_tail)) for c in cells}
    for cells in pockets(comp(swapped)):
        if len(cells) <= 2 and not cells & before:
            for (x, y) in sorted(cells):
                votes = Counter(swapped[(x + dx, y + dy)] for dx, dy in N8 if (x + dx, y + dy) in swapped)
                if votes:
                    swapped[(x, y)] = votes.most_common(1)[0][0]


def tail_ready_frames(loco: dict) -> list[dict]:
    """The 32 frames: ``16 * phase + locomotion frame`` with only the ``tail`` cel changed."""
    frames = []
    for phase in PHASES:
        for n in range(N_LOCO):
            base = loco["frames"][n]
            cels = copy.deepcopy(base["cels"])
            tail = pix(base["cels"]["tail"])
            body = pix(base["cels"]["body"])
            others = {}
            for name in LOCO_LAYERS[1:]:
                others.update(pix(base["cels"][name]) if base["cels"].get(name) else {})
            for reclose in (True, False):       # the K re-close is dropped only where it would break the 24 px limit
                swapped = tail_swap(tail, body, phase, reclose)
                fix_tail_orphans(swapped, others, ACCENTS)
                close_new_tail_pockets(tail, swapped, others)
                if changed_pixels(tail, swapped) <= MAX_TAIL_CHANGES:
                    break
            cels["tail"] = K.cel(swapped)
            frames.append({"duration_ms": ms(loco_ticks(base)), "cels": cels})
    return frames


# ---------------------------------------------------------------------------- VFX strip (spec section 5)

VCANVAS = (48, 16)
VPIVOT = (24, 11)
VFX_NAMES = (
    "ring_open_0", "ring_open_1", "ring_open_2", "ring_open_3",
    "ring_close_0", "ring_close_1", "ring_close_2",
    "streak_dash", "streak_head",
    "fizzle_0", "fizzle_1", "fizzle_2",
    "glint", "twinkle_small",
    "swish_a", "swish_b",
)
VFX_TAGS = (("ring_open", 0, 3, "once"), ("ring_close", 4, 6, "once"), ("streak", 7, 8, "free"),
            ("fizzle", 9, 11, "once"), ("glint", 12, 13, "free"), ("swish", 14, 15, "once"))
VFX_TICKS = [3, 3, 3, 3, 3, 3, 3, 1, 1, 3, 3, 2, 4, 1, 1, 1]      # spec section 5 table; glint 4 on, twinkle 1
DASH_RECT = (16, 10, 16, 3)              # the tileable dash inside frame 7: x, y, w, h
HEAD_RECT = (22, 10, 4, 3)               # the streak head inside frame 8
RING_ROW = VPIVOT[1]                     # row 11 is the puddle surface line


def vblank() -> list[list[str]]:
    return [["."] * VCANVAS[0] for _ in range(VCANVAS[1])]


def ring_cells(hw: int, cx: int = VPIVOT[0], cy: int = RING_ROW) -> set:
    """A flat ellipse outline of half width ``hw`` (ratio about 1:4) as rows that join: a connected 1 px ring whose
    top and bottom rows are flat."""
    ry = max(1, hw // 4)

    def extent(dy: int) -> int:
        t = abs(dy) / (ry + 0.5)
        return round(hw * math.sqrt(max(0.0, 1.0 - t * t)))

    out = set()
    for dy in range(-ry, ry + 1):
        outer = extent(dy)
        inner = 0 if abs(dy) == ry else min(extent(abs(dy) + 1), outer)
        for dx in range(inner, outer + 1):
            out.add((cx + dx, cy + dy))
            out.add((cx - dx, cy + dy))
    return out


def draw_ring(g, hw: int, colour: str, shadow: str | None = None, keep=None) -> None:
    cells = ring_cells(hw)
    if keep is not None:
        cells = {c for c in cells if keep(c)}
    if shadow:
        for (x, y) in sorted(cells):
            if y + 1 < VCANVAS[1] and (x, y + 1) not in cells and g[y + 1][x] == ".":
                g[y + 1][x] = shadow
    for (x, y) in cells:
        if 0 <= y < VCANVAS[1]:
            g[y][x] = colour


def drops(g, pts, colour: str = "l") -> None:
    """Two vertical 2 px drops (never a lone pixel)."""
    for (x, y) in pts:
        g[y][x] = colour
        g[y + 1][x] = colour


def vfx_rows() -> list[list[str]]:
    frames = []
    # ring_open 0-3: the newest ring c, the one before p, older rings m over a k shadow; frame 3 the outer ring only,
    # broken into dashes.
    for n in range(3):
        g = vblank()
        for i in range(n + 1):
            age = n - i
            hw = (6, 10, 14)[i]
            if age == 0:
                draw_ring(g, hw, "c")
            elif age == 1:
                draw_ring(g, hw, "p")
            else:
                draw_ring(g, hw, "m", "k")
        frames.append(g)
    g = vblank()
    draw_ring(g, 18, "m", "k", keep=lambda c: (abs(c[0] - VPIVOT[0]) // 4) % 2 == 0 or c[1] in (RING_ROW - 4, RING_ROW + 4))
    frames.append(g)
    # ring_close 4-6: the ring pulls in (half widths 14, 8, 3), two lilac drops above it
    g = vblank(); draw_ring(g, 14, "m", "k"); drops(g, [(19, 4), (29, 3)]); frames.append(g)
    g = vblank(); draw_ring(g, 8, "p", "k"); drops(g, [(21, 6), (28, 5)]); frames.append(g)
    g = vblank(); draw_ring(g, 3, "c"); drops(g, [(23, 7), (26, 6)]); frames.append(g)
    # streak_dash 7: 16 x 3, tileable. Top edge m, p core (the head c, 2 px), d under.
    g = vblank()
    x0, y0, w, _h = DASH_RECT
    for x in range(x0, x0 + w):
        head = x >= x0 + w - 2
        g[y0][x] = "c" if head else "m"
        g[y0 + 1][x] = "c" if head else "p"
        g[y0 + 2][x] = "d"
    frames.append(g)
    # streak_head 8: the arrival end, a 4 x 3 c/w drop
    g = vblank()
    x0, y0, _w, _h = HEAD_RECT
    for dx, ch in enumerate(".cc."):
        if ch != ".":
            g[y0][x0 + dx] = ch
            g[y0 + 2][x0 + dx] = ch
    for dx, ch in enumerate("cwwc"):
        g[y0 + 1][x0 + dx] = ch
    frames.append(g)
    # fizzle 9-11: a drying ring (half widths 18, 10, 4), then a single violet drip
    g = vblank(); draw_ring(g, 18, "m", "k"); frames.append(g)
    g = vblank(); draw_ring(g, 10, "d"); frames.append(g)
    g = vblank(); draw_ring(g, 4, "d"); drops(g, [(24, 6)], "v"); frames.append(g)
    # glint 12: a 3 x 3 plus, w centre and c arms; twinkle_small 13: the one-pixel core with a single pale neighbour
    # (a lone pixel is an isolated-pixel lint error); both are accent white, so no orphan either
    g = vblank()
    cx, cy = VPIVOT
    g[cy][cx] = "w"
    for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        g[cy + dy][cx + dx] = "c"
    frames.append(g)
    g = vblank()
    g[cy][cx] = "w"
    g[cy + 1][cx] = "w"
    frames.append(g)
    # swish 14-15: a thin curved arc in c with a w head and a k underside; the visible path of the attack she slipped
    for a, b, lift in ((4, 42, 0), (14, 44, 1)):
        g = vblank()
        pts = []
        for x in range(a, b + 1):
            t = (x - a) / (b - a)
            y = round(11 - lift + 2 - 4.0 * math.sin(math.pi * t))
            pts.append((x, y))
        for i, (x, y) in enumerate(pts):
            g[y][x] = "c"
            if i:
                py = pts[i - 1][1]
                for yy in range(min(py, y), max(py, y) + 1):
                    g[yy][x] = "c"
        for (x, y) in pts:
            if y + 1 < VCANVAS[1] and g[y + 1][x] == ".":
                g[y + 1][x] = "k"
        hx, hy = pts[-1]
        for dx in (0, -1, -2):
            g[hy][hx + dx] = "w"
        frames.append(g)
    for g in frames:
        merge_orphans(g)
    return frames


def merge_orphans(g) -> None:
    """Recolour every non-accent pixel with no same-colour 8-neighbour to its most common opaque neighbour (the spec's
    orphan rule: orphans only on accents and motes), so the strip lints with 0 orphans."""
    h, w = len(g), len(g[0])
    for _ in range(4):
        changed = False
        for y in range(h):
            for x in range(w):
                c = g[y][x]
                if c in (".", "w"):
                    continue
                around = [g[y + dy][x + dx] for dx, dy in N8 if 0 <= x + dx < w and 0 <= y + dy < h]
                opaque = [n for n in around if n != "."]
                if c in around or not opaque:
                    continue
                g[y][x] = Counter(opaque).most_common(1)[0][0]
                changed = True
        if not changed:
            return


def vpalette() -> list[dict]:
    import author_donut_missile as M
    pal = []
    for key, name, rgba, accent in M.VPALETTE:
        e = {"key": key, "name": name, "rgba": rgba}
        if accent:
            e["accent"] = True
        pal.append(e)
    return pal


def build_vfx_grid() -> dict:
    frames = [{"duration_ms": round(t * 1000 / TICK_HZ), "cels": {"body": ["".join(r) for r in g]}}
              for t, g in zip(VFX_TICKS, vfx_rows())]
    return {"format": "pixel-grid-v1", "canvas": list(VCANVAS), "pivot": list(VPIVOT), "palette": vpalette(),
            "layers": ["body"], "frames": frames,
            "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b, _ in VFX_TAGS]}


# ---------------------------------------------------------------------------- the sheets


def ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ)


def move_layers() -> list[dict]:
    """The 13 moves frames' layers (pixel dicts by layer name), in sheet order."""
    loco, melee, cf, cfk = load(LOCO_GRID), load(MELEE_GRID), load(CF02_GRID), load(CF02_KEYS_GRID)
    return [
        evade_coil(loco, melee), evade_arch(loco), None, evade_untwist(loco), evade_land(loco),
        blink_crouch(melee), blink_dip(loco), blink_vanish(), blink_pop(cfk), None, blink_settle(cfk),
        fizzle_bonk(loco, cf), fizzle_huff(cf),
    ]


def build_moves_grid() -> dict:
    loco = load(LOCO_GRID)
    palette = loco["palette"]
    accents = {e["key"] for e in palette if e.get("accent")}
    keys = load(MOVES_KEYS_GRID)
    frames = []
    for n, (layers, (_name, _tag, ticks, _author)) in enumerate(zip(move_layers(), MOVES)):
        if layers is None:                       # e2 and a1 are the frozen keys, byte for byte
            cels = copy.deepcopy(keys["frames"][0 if n == 2 else 1]["cels"])
        else:
            cels = finish(layers, accents, fill=FILL.get(n))
        frames.append({"duration_ms": ms(ticks), "cels": cels})
    tags = []
    for name in dict.fromkeys(t for _n, t, _k, _a in MOVES):
        idx = [i for i, (_n, t, _k, _a) in enumerate(MOVES) if t == name]
        tags.append({"name": name, "from": idx[0], "to": idx[-1], "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": list(PIVOT), "palette": palette,
            "layers": LAYERS, "frames": frames, "tags": tags}


def build_tail_grid() -> dict:
    loco = load(LOCO_GRID)
    tags = [{"name": p, "from": i * N_LOCO, "to": i * N_LOCO + N_LOCO - 1, "direction": "forward"}
            for i, p in enumerate(PHASES)]
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": list(PIVOT), "palette": loco["palette"],
            "layers": loco["layers"], "frames": tail_ready_frames(loco), "tags": tags}


def moves_config() -> dict:
    tags = {}
    for name in dict.fromkeys(t for _n, t, _k, _a in MOVES):
        rows = [[i, ticks] for i, (_n, t, ticks, _a) in enumerate(MOVES) if t == name]
        tags[name] = {"playback": TAG_PLAYBACK[name], "contact": TAG_CONTACT[name], "frames": rows}
    return {
        "format": "pixelart-asset-v1", "asset_id": "donut-moves-art-01",
        "title": "DONUT-MOVES-ART-01 Donut reflex evade, Puddle Jumper blink and fizzle frames "
                 "(docs/02-art/donut-moves-art-01-spec.md)",
        "grid": "assets/source/characters/donut/donut_h40_moves.grid.json",
        "manifest": "assets/manifests/donut_h40_moves.json",
        "frame_size": [W, H], "pivot": list(PIVOT), "contact_row": PIVOT[1], "tick_hz": TICK_HZ, "hold_ms": 100,
        "facing": "right", "tags": tags,
        "roles": {"move": "evade", "tell": "blink_depart", "attack": "blink_arrive", "hurt": "fizzle"},
        "rules": {
            "max_colors": 22, "max_isolated_per_frame": 0, "max_orphans_per_frame": 0,
            "canvas_margin": [1, 1, 1, 0], "unique_frames": True,
            "frozen_keys": {"fixture": KEY_FIXTURE, "frames": list(MOVE_KEYS), "layers": LAYERS},
            "bbox": [
                {"id": "evade-size", "frames": [0, 1, 2, 3, 4], "max_height": 42},
                {"id": "blink-size", "frames": [5, 6, 7, 8, 9, 10, 11, 12], "max_width": 70},
            ],
        },
        "scene": {"shadow": True, "actors": [{"frame": 0, "x": 520, "flip": False},
                                             {"frame": 2, "x": 600, "lift": 10, "flip": False},
                                             {"frame": 4, "x": 680, "flip": False}]},
        "review": {"strip_tags": ["evade", "blink_depart", "blink_arrive"]},
    }


def tail_config() -> dict:
    tags = {}
    for i, name in enumerate(PHASES):
        loco = load(LOCO_GRID)
        rows = [[i * N_LOCO + n, loco_ticks(loco["frames"][n])] for n in range(N_LOCO)]
        tags[name] = {"playback": "loop", "contact": "free", "frames": rows}
    return {
        "format": "pixelart-asset-v1", "asset_id": "donut-moves-art-01-tail-ready",
        "title": "DONUT-MOVES-ART-01 tail swap: the locomotion atlas with only the tail cel flicked "
                 "(docs/02-art/donut-moves-art-01-spec.md section 3)",
        "grid": "assets/source/characters/donut/donut_h40_tail_ready.grid.json",
        "manifest": "assets/manifests/donut_h40_tail_ready.json",
        "frame_size": [W, H], "pivot": list(PIVOT), "contact_row": PIVOT[1], "tick_hz": TICK_HZ, "hold_ms": 100,
        "facing": "right", "tags": tags,
        "roles": {"idle": "flick_up", "tell": "flick_snap"},
        "rules": {"max_colors": 22, "max_isolated_per_frame": 0, "max_orphans_per_frame": 0,
                  "canvas_margin": [0, 0, 0, 0], "unique_frames": True},
        "scene": {"shadow": True, "actors": [{"frame": 0, "x": 520, "flip": False},
                                             {"frame": 16, "x": 586, "flip": False},
                                             {"frame": 24, "x": 652, "flip": False}]},
        "review": {"strip_tags": ["flick_up", "flick_snap"]},
    }


def vfx_config() -> dict:
    tags = {}
    for name, a, b, pb in VFX_TAGS:
        tags[name] = {"playback": pb if pb != "free" else "once", "contact": "free",
                      "frames": [[i, VFX_TICKS[i]] for i in range(a, b + 1)]}
    return {
        "format": "pixelart-asset-v1", "asset_id": "donut-moves-art-01-vfx",
        "title": "DONUT-MOVES-ART-01 Puddle Jumper rings, streak, fizzle, glint and evade swish "
                 "(docs/02-art/donut-moves-art-01-spec.md section 5)",
        "grid": "assets/source/vfx/puddle_jumper/vfx_puddle_jumper.grid.json",
        "manifest": "assets/manifests/vfx_puddle_jumper.json",
        "frame_size": list(VCANVAS), "pivot": list(VPIVOT), "contact_row": None, "tick_hz": TICK_HZ, "hold_ms": 100,
        "facing": "right", "tags": tags,
        "roles": {"idle": "glint", "move": "streak", "tell": "ring_open", "attack": "swish", "hurt": "fizzle",
                  "defeat": "ring_close"},
        "rules": {"max_colors": 20, "max_isolated_per_frame": 0, "max_orphans_per_frame": 9,
                  "canvas_margin": [0, 0, 0, 0], "unique_frames": True,
                  "manifest_tick_rows": "vfx.frames", "manifest_tick_hz": "vfx.tick_hz"},
        "scene": {"shadow": False, "actors": [{"frame": 3, "x": 520, "flip": False},
                                              {"frame": 7, "x": 600, "lift": 8, "flip": False},
                                              {"frame": 14, "x": 680, "lift": 8, "flip": False}]},
        "review": {"strip_tags": ["ring_open", "fizzle"]},
    }


def _provenance(origin: str, inputs: list[str]) -> dict:
    return {"origin": origin,
            "generator": "design keys and spec: Claude Opus; remaining frames, export, wiring and evidence: Claude "
                         "Sonnet; independent review pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True, "input_reference_ids": inputs}


LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
           "license; no game/asset-pack distribution rights")


def moves_manifest() -> dict:
    frames = [{"index": i, "name": n, "tag": t, "ticks": k, "duration_ms": ms(k), "author": a}
              for i, (n, t, k, a) in enumerate(MOVES)]
    return {
        "schema_version": 1, "id": "donut_h40_moves_candidate_v0", "type": "character_animation",
        "source_path": "assets/source/characters/donut/donut_h40_moves.ase",
        "grid_source": "assets/source/characters/donut/donut_h40_moves.grid.json",
        "output_atlas": "generated/art/donut_h40_moves.png", "output_metadata": "generated/art/donut_h40_moves.json",
        "palette_id": "donut-master-h40", "frame_size": [W, H], "pivot": list(PIVOT),
        "animation_tags": list(dict.fromkeys(t for _n, t, _k, _a in MOVES)), "required_alpha": True,
        "owner": "AI-authored animation candidate (fan-concept layer: Princess Donut; runtime role IP-neutral small "
                 "quadruped reflex evade and short-blink cues)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": _provenance(
            "DONUT-MOVES-ART-01 hero keys (evade_twist, blink_arrive; scripts/author_donut_moves_keys.py) and the "
            "remaining frames drawn by rule in scripts/author_donut_moves.py from the keys and the approved "
            "locomotion, melee and CF02 cels",
            ["assets/source/characters/donut/moves_keys/donut_h40_moves_keys.grid.json",
             "assets/source/characters/donut/donut_h40_locomotion.grid.json",
             "assets/source/characters/donut/donut_h40_melee.grid.json",
             "assets/source/characters/donut/donut_h40_cf02.grid.json"]),
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
        "moves": {
            "status": "Keys frozen; remaining frames drawn; wired into the Floor 1 scene presenter; owner art review "
                      "pending",
            "spec": "docs/02-art/donut-moves-art-01-spec.md", "tick_hz": TICK_HZ,
            "composition": "full 72 x 44 frames, lowest-row anchored; the evade arc lift is applied by the presenter "
                           "(donut_cues::arc_lift_px), never baked",
            "key_fixture": KEY_FIXTURE, "key_frames": list(MOVE_KEYS),
            "evade_ticks": 12, "evade_frame_for_age": [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 4],
            "blink_depart_frame_for_age": [5, 5, 5, 6, 6, 7], "blink_arrive_frame_for_age": {"6": 8, "7": 8, "8": 9,
                                                                                         "9": 9, "10": 9, "11": 10,
                                                                                         "12": 10},
            "fizzle_frame_for_age": [11, 11, 11, 11, 12, 12, 12, 12],
            "frames": frames,
        },
    }


def tail_manifest() -> dict:
    return {
        "schema_version": 1, "id": "donut_h40_tail_ready_candidate_v0", "type": "character_animation",
        "source_path": "assets/source/characters/donut/donut_h40_tail_ready.ase",
        "grid_source": "assets/source/characters/donut/donut_h40_tail_ready.grid.json",
        "output_atlas": "generated/art/donut_h40_tail_ready.png",
        "output_metadata": "generated/art/donut_h40_tail_ready.json",
        "palette_id": "donut-master-h40", "frame_size": [W, H], "pivot": list(PIVOT),
        "animation_tags": list(PHASES), "required_alpha": True,
        "owner": "AI-authored animation candidate (fan-concept layer: Princess Donut; runtime role IP-neutral small "
                 "quadruped ready tell)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": _provenance(
            "DONUT-MOVES-ART-01 tail swap: the ANIM-DONUT-01 locomotion frames with only the tail cel changed, by the "
            "rule of spec section 3 in scripts/author_donut_moves.py",
            ["assets/source/characters/donut/donut_h40_locomotion.grid.json"]),
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
        "tail_ready": {
            "status": "Drawn by rule; wired into the Floor 1 scene presenter; owner art review pending",
            "spec": "docs/02-art/donut-moves-art-01-spec.md", "tick_hz": TICK_HZ,
            "composition": "tail_swap_phase_synced: atlas frame = 16 * phase_index + locomotion_frame",
            "locomotion_atlas": "assets/source/characters/donut/donut_h40_locomotion.grid.json",
            "phases": list(PHASES), "phase_ticks": {"flick_up": [0, 1], "flick_snap": [2, 3]},
            "tip_pixels": TIP_PIXELS, "max_changed_pixels": MAX_TAIL_CHANGES,
        },
    }


def vfx_manifest() -> dict:
    tag_of = {n: name for name, a, b, _ in VFX_TAGS for n in range(a, b + 1)}
    frames = [{"index": n, "name": VFX_NAMES[n], "tag": tag_of[n], "ticks": t, "duration_ms": ms(t),
               "author": "execution"} for n, t in enumerate(VFX_TICKS)]
    return {
        "schema_version": 1, "id": "vfx_puddle_jumper_48_candidate_v0", "type": "vfx_animation",
        "source_path": "assets/source/vfx/puddle_jumper/vfx_puddle_jumper.ase",
        "grid_source": "assets/source/vfx/puddle_jumper/vfx_puddle_jumper.grid.json",
        "output_atlas": "generated/art/vfx_puddle_jumper.png", "output_metadata": "generated/art/vfx_puddle_jumper.json",
        "palette_id": "vfx-missile-v0", "frame_size": list(VCANVAS), "pivot": list(VPIVOT),
        "animation_tags": [n for n, *_ in VFX_TAGS], "required_alpha": True,
        "owner": "AI-authored VFX candidate (fan-concept layer: Donut's Puddle Jumper and reflex cues; runtime role "
                 "IP-neutral small magic ripple, streak and glint)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": _provenance(
            "DONUT-MOVES-ART-01 VFX strip rasterized natively from rules in scripts/author_donut_moves.py, in the "
            "Magic Missile palette unchanged", []),
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
        "vfx": {
            "status": "Drawn by rule; wired into the Floor 1 scene presenter; owner art review pending",
            "spec": "docs/02-art/donut-moves-art-01-spec.md", "tick_hz": TICK_HZ,
            "free_tags": [n for n, *_ in VFX_TAGS], "playback": {n: pb for n, _a, _b, pb in VFX_TAGS},
            "facing": "right", "surface_row": VPIVOT[1],
            "dash_rect": list(DASH_RECT), "head_rect": list(HEAD_RECT),
            "frames": frames,
        },
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def mdump(data: dict) -> str:
    return json.dumps(data, indent=2) + "\n"


OUTPUTS = (
    (MOVES_GRID, build_moves_grid, dump), (TAIL_GRID, build_tail_grid, dump), (VFX_GRID, build_vfx_grid, dump),
    (MOVES_CFG, moves_config, mdump), (TAIL_CFG, tail_config, mdump), (VFX_CFG, vfx_config, mdump),
    (MOVES_MANIFEST, moves_manifest, mdump), (TAIL_MANIFEST, tail_manifest, mdump), (VFX_MANIFEST, vfx_manifest, mdump),
)
MANIFESTS = {MOVES_MANIFEST, TAIL_MANIFEST, VFX_MANIFEST}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", nargs="?", choices=("keys", "manifests", "evidence"), default="keys")
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.mode == "evidence":
        import donut_moves_evidence
        return donut_moves_evidence.main()
    stale = []
    for path, build, fmt in OUTPUTS:
        if a.mode == "manifests" and path not in MANIFESTS:
            continue
        text = fmt(build())
        if a.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != text:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    if stale:
        print("differs: " + ", ".join(stale), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
