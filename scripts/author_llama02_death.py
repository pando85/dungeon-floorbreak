#!/usr/bin/env python3
"""LLAMA-02-DEATH authoring record: Bad Llama death poses and the death-parts sheet (pixel-grid-v1 sources).

Usage:

    python3 scripts/author_llama02_death.py keys [--check]
    python3 scripts/author_llama02_death.py scale

``keys`` (re)writes the two sources from the rig poses, rules and literal stamps below (see
``docs/02-art/llama-02-death-spec.md``):

* ``assets/source/characters/llama/death/llama_death_h64.grid.json``: 8 frames of 96 x 72, pivot (44, 71), the LLAMA-01
  frame contract. Keys 0 ``crush_contact``, 2 ``crushed_remains``, 3 ``decap_contact``, 5 ``headless_collapse`` (hold),
  6 ``slam_contact`` and 7 ``broken_heap`` (hold). Frames 1 and 4 are **placeholders** (copies of keys 0 and 3) that the
  executor replaces with the in-betweens of spec section 9.
* ``assets/source/characters/llama/death/llama_death_parts.grid.json``: 32 x 32 cells, one part per frame. Keys 0
  ``head``, 1 ``neck`` and 2 ``leg_fore``. The executor appends frames 3 onward (spec section 7) and the manifests.

``scale`` writes the design-stage evidence into ``docs/evidence/llama-02-death/`` (key sheet at 4x with presets 4, 2
and 1, the 1x scale lineup next to Carl 52 / Donut 40 / goblin 36 on lit Floor 1 brick with a 4x band, and the in-scene
composition at 640 x 360 and 1920 x 1080).

Construction: every body pixel is rasterized by the LLAMA-01 parametric rig (``scripts/author_llama01.py``: capsules and
ellipses, the torch ramp, shag, mange, a 1 px outline) from new pose dicts, plus literal face stamps; cuts are half-plane
or rectangle clips before the outline pass, so the cut is closed by the rig's own outline. Gore and molten pixels are
shape rules (cut faces, jets, squeeze bursts, seeps, a molten stream) on their own layers. No concept pixels, no Death
Lab pixels, no image-model output.

Layer rule (the preset contract): ``body`` alone is always a closed, outlined, bloodless sprite. Every wound, blood and
bone pixel is on ``gore`` (hidden at preset 1, remapped dark at preset 2). ``molten`` holds the ruptured spit gland
(the creature's own lava, not blood): it is shown at every preset. Nothing is redrawn per preset.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_llama01 as L  # noqa: E402  (the LLAMA-01 rig: Part, Canvas, poly, mane, stamps, key poses)

DEATH_DIR = ROOT / "assets/source/characters/llama/death"
POSES = DEATH_DIR / "llama_death_h64.grid.json"
POSES_ASE = DEATH_DIR / "llama_death_h64.ase"
PARTS = DEATH_DIR / "llama_death_parts.grid.json"
PARTS_ASE = DEATH_DIR / "llama_death_parts.ase"
LLAMA = ROOT / "assets/source/characters/llama/llama-01.grid.json"
EVIDENCE = ROOT / "docs/evidence/llama-02-death"
CANVAS = (96, 72)
PIVOT = (44, 71)
CELL = (32, 32)
CELL_PIVOT = (16, 16)
LAYERS = ["body", "gore", "molten"]
TICK_HZ = 60
HOLD_MS = 100

# llama-death-floor01-v0 (20 of 20): the 17 llama-floor01-v0 keys with the same hex, plus the shared gore ramp of
# GOB-02-DEATH with the same hex (B blood_dark, b blood_mid = the goblin's red_mid, P blood_wet). Blood ramp, dark to
# light: o (the llama mouth, deepest blood and old rims), B, b, P. Bone ends: e. Molten ramp: a crust, r, R, O, Y.
PALETTE = L.PALETTE + [
    ("B", "blood_dark", "#6e1a22ff", False),
    ("b", "blood_mid", "#b8322eff", False),
    ("P", "blood_wet", "#e2434bff", True),
]
KEYS = [p[0] for p in PALETTE]
GORE_ONLY = set("BbP")
GORE_KEYS = set("oBbPe")          # what may appear on the gore layer
MOLTEN_KEYS = set("KarROY")       # what may appear on the molten layer
MUTED = {"b": "B", "P": "B"}      # preset 2 remap of the gore layer

Pix = dict  # {(x, y): key}


# --------------------------------------------------------------------------- pixel helpers
def to_pix(rows: list[str]) -> Pix:
    return {(x, y): ch for y, row in enumerate(rows) for x, ch in enumerate(row) if ch != "."}


def rows_of(pix: Pix, size=CANVAS) -> list[str]:
    w, h = size
    grid = [["."] * w for _ in range(h)]
    for (x, y), ch in pix.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel outside the {w}x{h} canvas: {(x, y)}")
        if ch not in KEYS:
            raise ValueError(f"unknown key {ch!r}")
        grid[y][x] = ch
    return ["".join(r) for r in grid]


def shift(pix: Pix, dx: int, dy: int) -> Pix:
    return {(x + dx, y + dy): ch for (x, y), ch in pix.items()}


def bbox(pix: Pix) -> tuple[int, int, int, int]:
    xs = [p[0] for p in pix]
    ys = [p[1] for p in pix]
    return min(xs), min(ys), max(xs), max(ys)


def tidy(layer: Pix) -> Pix:
    """Repaint one-pixel colour islands of a gore or molten shape with the commonest neighbouring key of the same
    layer (accents and outline stay), so splats read as shapes, not noise."""
    out = dict(layer)
    for (x, y) in sorted(layer, key=lambda q: (q[1], q[0])):
        ch = out[(x, y)]
        if ch in "KPeYa":
            continue
        around = [out.get((x + dx, y + dy)) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
        if ch in around:
            continue
        pool = sorted(n for n in around if n and n not in "KPeYa")
        if pool:
            out[(x, y)] = max(pool, key=pool.count)
    return out


def cels(body: Pix, gore: Pix | None = None, molten: Pix | None = None, size=CANVAS) -> dict[str, list[str]]:
    gore, molten = tidy(gore or {}), tidy(molten or {})
    if any(ch in GORE_ONLY for ch in body.values()):
        raise ValueError("gore-only key on the body layer")
    if set(gore.values()) - GORE_KEYS or set(molten.values()) - MOLTEN_KEYS:
        raise ValueError("wrong key on the gore or molten layer")
    return {"body": rows_of(body, size), "gore": rows_of(gore, size), "molten": rows_of(molten, size)}


def literal(x0: int, y0: int, rows: list[str]) -> Pix:
    return {(x0 + i, y0 + j): ch for j, row in enumerate(rows) for i, ch in enumerate(row) if ch != "."}


# --------------------------------------------------------------------------- the rig, with cuts
ALL = frozenset({"far_hind", "far_front", "ear_far", "tail", "barrel", "near_hind", "near_front", "neck", "head"})
EYES = dict(L.EYES) | {
    "bulge": [".KKK.", "KeeeK", "KeeKK", ".KKK."],     # popped eye, pinpoint pupil at the front
    "shock": [".KK.", "KeeK", "KeKK", ".KK."],         # severed head: wide white eye, small pupil
}
TEETH = dict(L.TEETH) | {
    "gape_up": ["KTTKTK", "KTtKtK", ".KK.K."],         # upper buck teeth of a jaw sprung wide (tilted muzzle)
    "gape_lo": [".K.K.", "KTKTK", "KtTtK"],
}


def _clip(c: L.Canvas, group: str, clips) -> None:
    for y in range(c.h):
        for x in range(c.w):
            if c.group[y][x] != group:
                continue
            px, py = x + 0.5, y + 0.5
            for kind, *a in clips:
                if kind == "plane":
                    (qx, qy), (nx, ny) = a
                    gone = (px - qx) * nx + (py - qy) * ny > 0
                else:
                    x0, y0, x1, y1 = a[0]
                    gone = x0 <= x <= x1 and y0 <= y <= y1
                if gone:
                    c.px[y][x], c.group[y][x], c.mat[y][x] = ".", None, None
                    break


def beast(p: dict, include=ALL, clips=(), size=CANVAS) -> Pix:
    """The LLAMA-01 rig (``author_llama01.llama``) with optional components and clips applied before the outline, so
    every cut is closed by the rig's own 1 px outline. Body layer only."""
    c = L.Canvas(*size)
    far = []
    for leg in ("far_hind", "far_front"):
        if leg in include and p.get(leg):
            pts = p[leg]
            for (x1, y1, r1), (x2, y2, r2) in zip(pts, pts[1:]):
                far.append(L.Part("cap", ((x1, y1), (x2, y2), r1, r2), "far", "far"))
            hx = p[leg + "_hoof"]
            far.append(L.Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof", "far"))
    er = p.get("ear_r", (1.8, 1.0))
    if "ear_far" in include and "head" in include and p.get("ear_far"):
        far += L.poly(p["ear_far"], (er[0] - 0.2, er[-1] - 0.1), "ear_in", "far")
    for part in far:
        c.draw(part)
    _clip(c, "far", clips)
    c.outline("far")

    main = []
    if "tail" in include and p.get("tail"):
        tail = p["tail"]
        main.append(L.Part("cap", (tail[0], tail[1], 1.4, 2.4), "fur", shag=((0, 0), 2.6, 0.5)))
    if "barrel" in include:
        rr = p.get("barrel_rr") or (p["barrel_r"], p["barrel_r"] - 0.5)
        main.append(L.Part("cap", (p["rump"], p["chest"], *rr), "fur", shag=((0, 0), 1.0)))
        main.append(L.Part("ell", p["belly"], "fur", shag=p.get("belly_shag", ((0, 1), 2.0))))
        main.append(L.Part("ell", p["haunch"], "fur", shag=((-1, 0.3), 1.6)))
        main.append(L.Part("ell", p["shoulder"], "fur", shag=((0.6, 1), 2.0)))
    for leg in ("near_hind", "near_front"):
        if leg in include and p.get(leg):
            pts = p[leg]
            for i, ((x1, y1, r1), (x2, y2, r2)) in enumerate(zip(pts, pts[1:])):
                mat = "fur" if i == 0 else "leg"
                main.append(L.Part("cap", ((x1, y1), (x2, y2), r1, r2), mat, shag=((0, 1), 1.0) if i == 0 else None))
            hx = p[leg + "_hoof"]
            main.append(L.Part("ell", (hx[0] + 0.5, hx[1] + 0.6, 2.0, 1.2), "hoof"))
    if "neck" in include and p.get("neck"):
        main += L.poly(p["neck"], p["neck_r"], "fur", shag=L.mane)
    ear_parts = []
    if "head" in include:
        main.append(L.Part("ell", p["skull"], "fur"))
        ja, jb = p["jaw"]
        main.append(L.Part("cap", (ja, jb, *p.get("jaw_r", (2.6, 2.0))), "muzzle"))
        ma, mb = p["muzzle"]
        main.append(L.Part("cap", (ma, mb, *p.get("muzzle_r", (3.6, 2.9))), "muzzle"))
        if p.get("mouth"):
            main.append(L.Part("tri", p["mouth"], "mouth"))
        ear_parts = L.poly(p["ear_near"], er, "fur")
        main += ear_parts
    for part in main:
        c.draw(part)
    c.strands()
    if "barrel" in include:
        rx0, ry0 = p["rump"]
        for kind, dx, dy, a, b in p.get("mange", L.MANGE):
            (c.streak if kind == "s" else c.bald)(rx0 + dx, ry0 + dy, a, b)
    _clip(c, "main", clips)
    c.outline("main")
    if "head" in include:
        for name, (x, y) in p.get("teeth", ()):
            c.stamp(x, y, TEETH[name])
        if p.get("eye"):
            c.stamp(*p["eye_at"], EYES[p["eye"]])
        if p.get("nostril"):
            c.stamp(*p["nostril"], ["KK"])
    for x, y, rows in p.get("extra", ()):
        c.stamp(x, y, rows)
    c.cleanup()
    return close_outline(to_pix(c.rows()), size)


def close_outline(pix: Pix, size=CANVAS) -> Pix:
    """Every empty 4-neighbour (inside the canvas) of a coloured pixel becomes ``K``: the face and tongue stamps sit on
    the rig's outline, so this keeps the bloodless body a closed sprite."""
    out = dict(pix)
    for (x, y), ch in pix.items():
        if ch == "K":
            continue
        for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if n not in pix and 0 <= n[0] < size[0] and 0 <= n[1] < size[1]:
                out[n] = "K"
    return out


# --------------------------------------------------------------------------- gore and molten rules
def cut_face(body: Pix, at, normal, radius: float, back) -> Pix:
    """The cut surface of a clipped stump on ``gore``. ``at``/``normal`` is the clip plane (normal pointing out of the
    stump), ``back`` a unit vector toward the mane side (the spine). The outline line on the cut and the fill row behind
    it become meat (``b``, clotted ``B`` at the two ends), with a spine end ``e`` toward the back and a wet glint ``P``
    toward the front. Hiding ``gore`` leaves the plain fur-and-outline cap of the clip."""
    nx, ny = normal
    tx, ty = -ny, nx
    sel = {}
    for (x, y), ch in body.items():
        px, py = x + 0.5 - at[0], y + 0.5 - at[1]
        s, u = px * nx + py * ny, px * tx + py * ty
        if -1.7 < s <= 1.6 and abs(u) <= radius + 1.2:
            sel[(x, y)] = (s, u)
    if not sel:
        return {}
    us = [u for _s, u in sel.values()]
    lo, hi = min(us), max(us)
    out: Pix = {}
    for p_, (s, u) in sel.items():
        out[p_] = "B" if (u - lo < 0.9 or hi - u < 0.9) else "b"
    side = 1 if back[0] * tx + back[1] * ty > 0 else -1
    inner = sorted((p_ for p_, (s, _u) in sel.items() if s <= 0 and out[p_] == "b"), key=lambda q: sel[q][1] * side)
    if inner:
        out[inner[-1]] = "e"                  # spine end at the back
        if len(inner) >= 2:
            out[inner[-2]] = "e"
        out[inner[min(1, len(inner) - 1)]] = "P"
    return out


def stroke(pts, widths, keys_for, out: Pix) -> None:
    """Rasterize a tapering stroke through ``pts``; ``keys_for(t, q)`` picks the key from the axis position ``t``
    (0..1) and the signed cross offset ``q`` (-1..1, positive on the right of the travel direction)."""
    segs = list(zip(pts, pts[1:]))
    lens = [math.dist(a, b) for a, b in segs]
    total = sum(lens)
    acc = 0.0
    x0 = int(min(p[0] for p in pts) - max(widths) - 2)
    x1 = int(max(p[0] for p in pts) + max(widths) + 2)
    y0 = int(min(p[1] for p in pts) - max(widths) - 2)
    y1 = int(max(p[1] for p in pts) + max(widths) + 2)
    best: dict = {}
    for (a, b), ln in zip(segs, lens):
        vx, vy = b[0] - a[0], b[1] - a[1]
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                px, py = x + 0.5, y + 0.5
                t = max(0.0, min(1.0, ((px - a[0]) * vx + (py - a[1]) * vy) / (ln * ln)))
                ox, oy = px - (a[0] + t * vx), py - (a[1] + t * vy)
                tt = (acc + t * ln) / total
                w = (widths[0] + (widths[-1] - widths[0]) * tt) / 2
                d = math.hypot(ox, oy)
                if d <= w and (d < best.get((x, y), (9, 0))[0]):
                    q = (ox * -vy + oy * vx) / ln / max(w, 0.5)
                    best[(x, y)] = (d, keys_for(tt, q))
        acc += ln
    for p_, (_d, k) in best.items():
        if k:
            out[p_] = k


def jet_keys(t: float, q: float) -> str:
    """Blood jet: ``B`` shadow edge on the right of travel, ``P`` core near the base, ``b`` body."""
    if q > 0.45:
        return "B"
    if t < 0.55 and -0.35 < q < 0.35:
        return "P"
    return "b"


def molten_keys(t: float, q: float) -> str:
    a = abs(q)
    if t > 0.9:
        return "R" if a < 0.5 else "r"
    return "Y" if a < 0.3 and t < 0.75 else "O" if a < 0.65 else "R" if a < 0.95 else "r"


def outline_molten(molten: Pix, body: Pix, size=CANVAS) -> Pix:
    """A 1 px ``K`` outline around the molten shapes where they sit over empty canvas (as on the LLAMA-01 spit glob)."""
    out = dict(molten)
    for (x, y) in molten:
        for n in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
            if n not in molten and n not in body and 0 <= n[0] < size[0] and 0 <= n[1] < size[1]:
                out[n] = "K"
    return out


def splat_row(out: Pix, x: int, y: int, run: str) -> None:
    for i, ch in enumerate(run):
        if ch != ".":
            out[(x + i, y)] = ch


# --------------------------------------------------------------------------- pose key 0: crush_contact
STONE = (13, 76)        # stone footprint columns: the falling block's anchor on frame x 42 (ENV-02 opaque x 7..70)
STONE_ROW = 52          # stone underside: the K line on row 52, nothing of the llama above it under the stone

CRUSH = L.shifted(
    L.IDLE,
    rump=(25, 62), chest=(59, 62), barrel_rr=(8.0, 8.5), belly=(42, 67, 16, 4.0),
    haunch=(24, 61, 9.0, 7.5), shoulder=(60, 61, 7.5, 7.5),
    near_hind=[(24, 65, 4.2), (15, 68, 2.8), (7, 69, 2.0)], near_hind_hoof=(3, 69),
    far_hind=[(29, 62, 3.2), (21, 65, 2.2), (16, 66, 1.6)], far_hind_hoof=(13, 66),
    near_front=[(60, 66, 3.4), (69, 68.5, 2.3), (75, 69.3, 1.6)], near_front_hoof=(77, 69),
    far_front=[(56, 64, 2.6), (63, 66.5, 2.0), (68, 67.5, 1.4)], far_front_hoof=(70, 67),
    neck=((62, 61), (75, 61), (81, 58)), neck_r=(4.6, 3.6, 3.2),
    skull=(84, 61.5, 4.6, 4.0), muzzle=((86, 59), (92, 55)), muzzle_r=(3.0, 2.4),
    jaw=((86, 66), (93, 67)), jaw_r=(2.4, 1.8), mouth=((86, 62.5), (94, 56.5), (94, 66)),
    ear_near=((81, 59), (77, 56.5), (73, 55.5)), ear_far=((82, 58), (78, 55), (74, 54)),
    tail=((17, 57), (5, 49), (-1, -0.6)),
    teeth=[("gape_up", (89, 56)), ("gape_lo", (89, 63))],
    eye="bulge", eye_at=(81, 58), nostril=(91, 53),
    mouth_line=None,
)
CRUSH_CLIPS = [("rect", (STONE[0], 0, STONE[1], STONE_ROW))]


def crush_contact() -> dict[str, list[str]]:
    body = beast(CRUSH, clips=CRUSH_CLIPS)
    body.update({(x, STONE_ROW): "K" for x in range(STONE[0], STONE[1] + 1)
                 if (x, STONE_ROW + 1) in body and body[(x, STONE_ROW + 1)] != "K"})
    gore: Pix = {}
    # squeeze-out bursts at both ends of the stone: low fans on the floor and one droplet pair each
    splat_row(gore, 0, 71, "BbbbbbPbbbbbbbBB")
    splat_row(gore, 0, 70, "bbPbbbbB")
    splat_row(gore, 0, 69, "Bbb")
    splat_row(gore, 1, 66, "bb")
    splat_row(gore, 0, 63, "bb")
    splat_row(gore, 4, 61, "bb")
    splat_row(gore, 9, 62, "bB")
    splat_row(gore, 72, 71, "BBbbbbbbbbbbbB")
    splat_row(gore, 78, 70, "bbPbbbB")
    splat_row(gore, 82, 69, "bbb")
    splat_row(gore, 86, 68, "Pb")
    splat_row(gore, 89, 69, "bB")
    # the hide bursts under the stone edge: a seam of meat on the front flank and a gush between the fore legs
    for x in range(52, 64):
        if (x, 54) in body:
            gore[(x, 54)] = "b" if x % 5 else "P"
    gore.update({(64, 54): "B", (65, 55): "b", (66, 55): "b", (67, 56): "B"})
    # blood out of the gaping mouth, running down the jaw onto the floor
    splat_row(gore, 85, 64, "bP")
    splat_row(gore, 85, 65, "Bb")
    splat_row(gore, 83, 70, "obbbo")
    molten: Pix = {}
    # the spit-gland burst: the squeezed throat blows a molten geyser out of the open mouth, breaking into globs
    stroke([(91.5, 60.5), (92, 52), (90.5, 44), (91.5, 39)], (4.6, 2.2), molten_keys, molten)
    stroke([(87.5, 34.5), (88.5, 33.5)], (2.2, 2.0), lambda t, q: "O" if t < 0.6 else "R", molten)
    stroke([(94, 35.5), (94.5, 34.5)], (2.0, 2.0), lambda t, q: "R", molten)
    molten = {p_: k for p_, k in molten.items()      # the geyser leaves the teeth and the eye visible
              if p_ not in body or (p_[1] < 58 and body[p_] not in "TteK")}
    molten = outline_molten(molten, body)
    return cels(body, gore, molten)


# --------------------------------------------------------------------------- pose key 2: crushed_remains
REMAINS = L.shifted(
    L.DEAD,
    rump=(24, 65), chest=(57, 65.5), barrel_rr=(5.5, 6.0), belly=(41, 67.5, 16, 3.5),
    haunch=(23, 64.5, 8.0, 5.5), shoulder=(57, 64.5, 6.5, 5.5),
    near_hind=[(23, 65, 3.6), (16, 59, 2.5), (21, 54, 1.9), (19, 51, 1.4)], near_hind_hoof=(18, 49),
    far_hind=[(29, 65, 2.6), (18, 67.5, 2.0), (9, 68.5, 1.4)], far_hind_hoof=(5, 68),
    near_front=[(59, 67.5, 3.0), (66, 69, 2.2), (72, 69.5, 1.4)], near_front_hoof=(73, 69),
    far_front=[(55, 65, 2.4), (62, 67.5, 1.9), (68, 68, 1.3)], far_front_hoof=(69, 67),
    neck=((60, 65), (77, 65.5)), neck_r=(4.0, 3.3),
    skull=(80, 65, 4.4, 3.6), muzzle=((82, 66), (90, 67.5)), jaw=((82, 68.7), (89, 69.5)), muzzle_r=(3.0, 2.4),
    mouth=((83, 67.5), (91, 68.5), (89, 70)),
    ear_near=((78, 62), (73, 61), (68.5, 62)), ear_far=((80, 62), (75, 61.5), (70.5, 62.5)),
    tail=((17, 64), (6, 63), (-1, 0)),
    teeth=[("upper", (88, 67))], eye="dead", eye_at=(79, 63), nostril=(89, 65),
    extra=[(91, 69, ["gK", "ggK"])],             # the tongue lolled onto the floor
)


def crushed_remains() -> dict[str, list[str]]:
    body = beast(REMAINS)
    top = {}
    for (x, y), ch in body.items():
        if 30 <= x <= 52 and ch != "K":
            top[x] = min(top.get(x, 99), y)
    gore: Pix = {}
    # the hide split open along the pressed back like a burst sausage: meat, three rib ends, two wet glints
    for x, y in top.items():
        gore[(x, y)] = "b"
        gore[(x, y + 1)] = "B" if x in (30, 52) else "b"
    for x in (35, 41, 47):
        gore[(x, top[x] - 1)] = "e"
        gore[(x, top[x])] = "e"
    gore[(38, top[38] + 1)] = "P"
    gore[(45, top[45] + 1)] = "P"
    # one continuous pressed-out seep under the whole heap, and blood out of the mouth
    splat_row(gore, 0, 71, "oBB" + "b" * 10 + "BbbbbPbbbb" + "b" * 30 + "BbbbbbB" + "bbbbbo")
    splat_row(gore, 18, 70, "bP")
    splat_row(gore, 62, 70, "Pb")
    molten: Pix = {}
    # the gland drains: a cooling dribble out of the mouth onto the floor, crusting at the tip
    stroke([(90.5, 71.5), (94.5, 71.5)], (2.0, 1.4), lambda t, q: "O" if t < 0.35 else "R" if t < 0.75 else "r", molten)
    molten = {p_: k for p_, k in molten.items() if p_[1] == 71}          # lies on the floor line, over the outline
    molten[(95, 71)] = "a"
    return cels(body, gore, molten)


# --------------------------------------------------------------------------- pose key 3: decap_contact
DECAP_AT, DECAP_N = (54.0, 22.5), (0.28, -0.96)        # blade plane through the upper neck, slightly rising forward
DECAP = L.shifted(L.HURT)


def decap_contact() -> dict[str, list[str]]:
    body = beast(DECAP, include=ALL - {"head"}, clips=[("plane", DECAP_AT, DECAP_N)])
    gore = cut_face(body, DECAP_AT, DECAP_N, 3.6, back=(-1, 0))
    # the jet: straight up out of the stump, leaning back the way the head flew, breaking into gobbets near the top
    stroke([(54, 21.5), (52.5, 12), (49.5, 4)], (7.0, 3.0), jet_keys, gore)
    gore.update({(47, 0): "b", (48, 0): "P", (46, 1): "b", (45, 1): "B", (51, 0): "b", (52, 0): "b", (56, 5): "b",
                 (56, 6): "B", (43, 4): "b", (43, 5): "B", (57, 12): "b", (58, 12): "b", (48, 10): "B", (47, 10): "b"})
    molten: Pix = {}
    # the severed spit duct: a hot dot in the front of the cut and a glowing dribble down the throat edge
    molten.update({(56, 22): "Y", (57, 22): "O", (56, 23): "O"})
    throat = sorted((p_ for p_ in body if 24 <= p_[1] <= 33 and body[p_] == "K"
                     and (p_[0] + 1, p_[1]) not in body and p_[0] >= 55), key=lambda q: q[1])
    for x, y in throat[:9]:
        molten[(x - 1, y)] = "O" if y < 28 else "R"
    molten.update({(57, 10): "O", (58, 10): "R", (57, 11): "r", (58, 11): "R"})   # a molten gobbet in the spray
    return cels(body, gore, outline_molten(molten, body))


# --------------------------------------------------------------------------- pose key 5: headless_collapse
HEADLESS_AT, HEADLESS_N = (74.0, 56.5), (0.72, -0.69)
HEADLESS = L.shifted(L.DEAD, neck=((57, 61), (67, 60), (77, 53.5)), neck_r=(4.6, 3.8, 3.4))


def headless_collapse() -> dict[str, list[str]]:
    body = beast(HEADLESS, include=ALL - {"head"}, clips=[("plane", HEADLESS_AT, HEADLESS_N)])
    gore = cut_face(body, HEADLESS_AT, HEADLESS_N, 3.6, back=(0, -1))
    for y in range(59, 70):                  # the stump drains down the throat side onto the floor
        xs = [x for (x, yy) in body if yy == y and x >= 70]
        if xs:
            gore[(max(xs) + 1, y)] = "b" if y % 4 else "B"
            gore[(max(xs) + 2, y)] = "b"
    splat_row(gore, 73, 70, "BbbbbbbPbbbbbbbB")
    splat_row(gore, 64, 71, "oBbbbbbbbbbbbPbbbbbbbbbbbbbbbBo")
    molten: Pix = {(76, 55): "Y", (77, 55): "O", (76, 56): "O", (82, 69): "O", (83, 69): "R", (84, 69): "r",
                   (85, 69): "a"}
    return cels(body, gore, molten)


# --------------------------------------------------------------------------- pose keys 6 and 7: slam (heavy impact)
SLAM_BODY = dict(
    rump=(27, 61), chest=(56, 62), barrel_rr=(6.0, 6.5), belly=(42, 56.5, 10, 3.5), belly_shag=((0, -1), 2.0),
    haunch=(26, 61.5, 6.5, 6.5), shoulder=(56, 62, 5.5, 6.5),
    mange=(("s", -5, 2, 6, 0.45), ("s", 7, 1, 7, 0.35), ("s", 19, 2, 6, 0.4), ("b", 12, 0.5, 3.4, 1.8)),
)
SLAM = L.shifted(
    L.DEAD, **SLAM_BODY,
    near_front=[(57, 57, 3.4), (64, 50, 2.4), (69, 44, 2.1), (75, 39, 1.3)], near_front_hoof=(76, 37),
    far_front=[(53, 56, 3.0), (56, 48, 2.2), (60, 42, 1.9), (63, 36, 1.2)], far_front_hoof=(63, 34),
    near_hind=[(28, 56, 4.2), (21, 50, 3.0), (17, 45, 2.1), (11, 41, 1.3)], near_hind_hoof=(9, 39),
    far_hind=[(33, 56, 3.2), (31, 48, 2.5), (28, 43, 1.9), (25, 37, 1.2)], far_hind_hoof=(24, 35),
    neck=((60, 63), (70, 58), (78, 62)), neck_r=(4.6, 3.6, 3.3),
    skull=(80, 64.5, 4.4, 3.6), muzzle=((82, 65), (89, 64)), jaw=((82, 68.5), (89, 70)), muzzle_r=(3.0, 2.4),
    mouth=((83, 66.5), (91, 64.5), (89, 69.5)),
    ear_near=((78, 61), (73, 63.5), (69, 66)), ear_far=((79, 60.5), (75, 64), (71, 67)),
    tail=((21, 62), (10, 57), (-1, -0.5)),
    teeth=[("upper", (85, 63))], eye="shut", eye_at=(78, 62), nostril=(88, 62),
)
BROKEN = L.shifted(
    SLAM,
    near_front=[(57, 57, 3.4), (60, 49, 2.4), (62, 42, 2.1), (65, 35, 1.3)], near_front_hoof=(65, 33),
    far_front=[(53, 56, 3.0), (54, 48, 2.2), (56, 42, 1.9), (56, 36, 1.2)], far_front_hoof=(56, 34),
    near_hind=[(28, 56, 4.2), (26, 49, 3.0), (29, 43, 2.1), (25, 37, 1.3)], near_hind_hoof=(24, 35),
    far_hind=[(33, 56, 3.2), (34, 48, 2.5), (36, 43, 1.9), (34, 37, 1.2)], far_hind_hoof=(33, 35),
    neck=((60, 63), (69, 57.5), (77, 64)), neck_r=(4.6, 3.5, 3.3),
    skull=(80, 65.5, 4.4, 3.6), muzzle=((82, 66.5), (90, 68)), jaw=((82, 69.2), (89, 70)),
    mouth=((83, 68), (91, 69), (89, 70.5)),
    ear_near=((78, 62.5), (73, 65), (68.5, 68)), ear_far=((79, 62), (75, 65.5), (70.5, 68.5)),
    tail=((21, 63), (9, 67), (-1, 0.4)),
    teeth=[("upper", (87, 67))], eye="dead", eye_at=(79, 64), nostril=(89, 65),
    extra=[(91, 69, ["gK", "ggK"])],
)


def slam_contact() -> dict[str, list[str]]:
    body = beast(SLAM)
    gore: Pix = {}
    # blood coughed up out of the mouth in a rising arc, breaking into droplet pairs; a first splat under the head
    stroke([(90, 66), (92.5, 60), (93, 54)], (3.0, 1.6), jet_keys, gore)
    gore.update({(91, 50): "b", (92, 50): "P", (94, 47): "b", (94, 46): "B", (89, 51): "b", (89, 52): "B"})
    splat_row(gore, 84, 71, "BbbPbbbB")
    # the snapped neck: a bone end through the hide at the kink
    gore.update({(69, 53): "e", (70, 53): "e", (70, 52): "e", (68, 54): "b", (69, 54): "b", (70, 54): "B",
                 (71, 54): "b"})
    molten: Pix = {}
    stroke([(86.5, 58.5), (88, 58)], (3.0, 2.4), lambda t, q: "Y" if abs(q) < 0.3 else "O", molten)   # coughed glob
    molten = {p_: k for p_, k in molten.items() if p_ not in body}
    return cels(body, gore, outline_molten(molten, body))


def broken_heap() -> dict[str, list[str]]:
    body = beast(BROKEN)
    gore: Pix = {}
    gore.update({(68, 52): "e", (69, 52): "e", (69, 51): "e", (67, 53): "b", (68, 53): "b", (69, 53): "B",
                 (70, 53): "b", (66, 54): "B"})
    splat_row(gore, 70, 71, "oBbbbbbPbbbbbbbbbbbbBbbo")
    splat_row(gore, 83, 70, "bb")
    splat_row(gore, 40, 71, "Bbbbb")
    molten: Pix = {(91, 71): "Y", (92, 71): "O", (93, 71): "O", (94, 71): "R", (95, 71): "r", (90, 71): "O",
                   (93, 70): "R", (94, 70): "r"}           # the gland drains out of the mouth onto the floor line
    return cels(body, gore, molten)


# --------------------------------------------------------------------------- part keys
HEAD_POSE = L.shifted(L.IDLE, neck=((59.6, 29), (60, 23)), neck_r=(3.4, 3.1), eye="shock", eye_at=(60, 16))
HEAD_AT, HEAD_N = (59.8, 27.5), (0.0, 1.0)
NECK_POSE = L.shifted(L.IDLE, neck=((55, 40), (59, 31), (60, 22)), neck_r=(4.6, 3.5, 3.1))
NECK_TOP = ((60.0, 24.5), (-0.05, -1.0))
NECK_BOTTOM = ((56.5, 37.5), (-0.3, 0.95))
LEG_AT, LEG_N = (52.0, 48.5), (0.0, -1.0)


def to_cell(body: Pix, gore: Pix, molten: Pix, centre=None):
    box = bbox({**body, **gore, **molten})
    cx, cy = centre or ((box[0] + box[2]) // 2, (box[1] + box[3]) // 2)
    dx, dy = CELL_PIVOT[0] - cx, CELL_PIVOT[1] - cy
    return cels(shift(body, dx, dy), shift(gore, dx, dy), shift(molten, dx, dy), CELL), (dx, dy)


CELL_SHIFT: dict[str, tuple[int, int]] = {}


def part_head():
    body = beast(HEAD_POSE, include={"head", "ear_far", "neck"}, clips=[("plane", HEAD_AT, HEAD_N)])
    gore = cut_face(body, HEAD_AT, HEAD_N, 3.4, back=(-1, 0))
    molten = {(62, 27): "O", (62, 26): "Y"}
    c, sh = to_cell(body, gore, molten)
    CELL_SHIFT["head"] = sh
    return c


def part_neck():
    clips = [("plane", *NECK_TOP), ("plane", *NECK_BOTTOM)]
    body = beast(NECK_POSE, include={"neck"}, clips=clips)
    gore = cut_face(body, *NECK_TOP, 3.4, back=(-1, 0))
    gore.update(cut_face(body, *NECK_BOTTOM, 4.4, back=(-1, 0)))
    molten = {(62, 26): "Y", (62, 27): "O", (59, 36): "O", (60, 36): "R"}
    c, sh = to_cell(body, gore, molten)
    CELL_SHIFT["neck"] = sh
    return c


def part_leg_fore():
    body = beast(L.IDLE, include={"near_front"}, clips=[("plane", LEG_AT, LEG_N)])
    gore = cut_face(body, LEG_AT, LEG_N, 3.6, back=(-1, 0))
    c, sh = to_cell(body, gore, {})
    CELL_SHIFT["leg_fore"] = sh
    return c


PART_BUILDERS = [("head", part_head), ("neck", part_neck), ("leg_fore", part_leg_fore)]
PART_IDS = [p[0] for p in PART_BUILDERS]
POSE_TAGS = [("crush_contact", 0, 1), ("crushed_remains", 2, 2), ("decap_contact", 3, 3), ("headless_collapse", 4, 5),
             ("slam_contact", 6, 6), ("broken_heap", 7, 7)]
POSE_TICKS = [3, 3, 0, 4, 6, 0, 6, 0]
POSE_KEYS = (0, 2, 3, 5, 6, 7)
PART_KEYS = (0, 1, 2)


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ) if ticks else HOLD_MS


def grid_of(frames, ticks, canvas, pivot, tags) -> dict:
    return {
        "format": "pixel-grid-v1",
        "canvas": list(canvas),
        "pivot": list(pivot),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": c} for t, c in zip(ticks, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in tags],
    }


def poses_grid() -> dict:
    k0, k2, k3, k5 = crush_contact(), crushed_remains(), decap_contact(), headless_collapse()
    frames = [k0, k0, k2, k3, k3, k5, slam_contact(), broken_heap()]      # 1 and 4: placeholders for the executor
    return grid_of(frames, POSE_TICKS, CANVAS, PIVOT, POSE_TAGS)


def parts_grid() -> dict:
    frames = [b() for _pid, b in PART_BUILDERS]
    return grid_of(frames, [0] * len(frames), CELL, CELL_PIVOT, [(pid, i, i) for i, pid in enumerate(PART_IDS)])


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# --------------------------------------------------------------------------- presets (data only) and evidence
def preset_view(grid: dict, preset: int) -> dict:
    """The same grid at a presentation preset: 4 and 3 as authored, 2 with ``gore`` remapped dark, 1 with ``gore``
    hidden. ``molten`` is shown at every preset. Nothing is redrawn."""
    out = json.loads(json.dumps(grid))
    for f in out["frames"]:
        if preset == 1:
            f["cels"].pop("gore", None)
        elif preset == 2:
            f["cels"]["gore"] = ["".join(MUTED.get(ch, ch) for ch in row) for row in f["cels"]["gore"]]
    if preset == 1:
        out["layers"] = [n for n in out["layers"] if n != "gore"]
    return out


def _images(grid):
    from scripts.pixelart.common import grid_images
    return grid_images(grid)


OPENING_KIT = ROOT / "assets/source/terrain/floor01/floor01_opening_kit.json"


def block_piece(piece_id: str):
    from scripts.pixelart.common import grid_images, load_grid_file
    kit = json.loads(OPENING_KIT.read_text(encoding="utf-8"))
    piece = next(p for p in kit["pieces"] if p["id"] == piece_id)
    atlas = grid_images(load_grid_file(ROOT / kit["atlases"][piece["atlas"]]["grid"]))[0]
    x, y, w, h = piece["rect"]
    return atlas.crop((x, y, x + w, y + h)), tuple(piece["anchor"])


BLOCK_X = 42            # frame x of the falling block's anchor in crush_contact (footprint STONE)


def write_key_sheet(out: Path = EVIDENCE / "keys_x4_presets.png") -> Path:
    """Every key at 4x on a flat brick tone, three rows per key: preset 4 (as authored), preset 2 (gore remapped dark)
    and preset 1 (gore hidden; molten kept). Bottom strip: every key at 1x, presets 4 and 1."""
    from PIL import Image, ImageDraw
    k = 4
    poses, parts = poses_grid(), parts_grid()
    views = {p: (_images(preset_view(poses, p)), _images(preset_view(parts, p))) for p in (4, 2, 1)}
    tiles = [("pose", n) for n in POSE_KEYS] + [("part", n) for n in PART_KEYS]
    cw, ch = CANVAS[0] * k, CANVAS[1] * k
    cols = 3
    rows_n = math.ceil(len(tiles) / cols)
    strip_h = CANVAS[1] + 20
    sheet = Image.new("RGB", (cols * (cw + 8) + 8, rows_n * (3 * ch + 30) + strip_h + 8), (40, 30, 34))
    d = ImageDraw.Draw(sheet)
    names = [t[0] for t in POSE_TAGS]
    for i, (kind, n) in enumerate(tiles):
        r, c = divmod(i, cols)
        x0, y0 = 8 + c * (cw + 8), 8 + r * (3 * ch + 30)
        label = (f"pose {n} {next(t for t, a, b in POSE_TAGS if a <= n <= b)}" if kind == "pose"
                 else f"part {n} {PART_IDS[n]}")
        d.text((x0, y0), label + "   rows: preset 4 / 2 / 1", fill=(240, 220, 180))
        for j, p in enumerate((4, 2, 1)):
            img = views[p][0 if kind == "pose" else 1][n]
            tile = Image.new("RGB", (cw, ch), (110, 64, 44) if j != 2 else (96, 58, 42))
            big = img.resize((img.width * k, img.height * k), Image.NEAREST)
            tile.paste(big, ((cw - big.width) // 2, ch - big.height), big)
            sheet.paste(tile, (x0, y0 + 12 + j * ch))
    x = 8
    y = sheet.height - strip_h
    for kind, n in tiles:
        for p in (4, 1):
            img = views[p][0 if kind == "pose" else 1][n]
            tile = Image.new("RGB", (img.width, CANVAS[1]), (110, 64, 44))
            tile.paste(img, (0, CANVAS[1] - img.height), img)
            sheet.paste(tile, (x, y))
            x += img.width + 2
        x += 4
    del names
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def _scene_parts(parts_imgs, x):
    out = []
    for img in parts_imgs:
        out.append((img, (CELL_PIVOT[0], img.getbbox()[3] - 1), x, 0, False))
        x += 26
    return out


def write_scale_sheet(out: Path = EVIDENCE / "silhouette_scale_1x.png") -> Path:
    """Two lit Floor 1 views at 1x (640 x 360), stacked, sprites as authored (facing right). Top: Donut (40 px), Carl
    (52 px), GOB-01 A idle (36 px), LLAMA-01 idle (64 px), then ``crush_contact`` with the ENV-02 falling block drawn
    over it (its anchor row on frame row 52), ``crushed_remains`` in front of the landed block, ``decap_contact``. Bottom: Donut and
    Carl, ``headless_collapse``, ``slam_contact``, ``broken_heap`` and the three part keys on the walking line. A second
    file holds the walking-line bands of both views at 4x."""
    from PIL import Image
    from scripts.pixelart.common import CARL_X, DONUT_X, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    gob, gob_piv = reference_idle("goblin")
    llama = grid_images(load_grid_file(LLAMA))[0]
    poses, parts = _images(poses_grid()), _images(parts_grid())
    falling, fa = block_piece("loose_block_falling")
    landed, la = block_piece("loose_block_landed")
    piv = tuple(PIVOT)
    cast = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
    top = cast + [
        (gob, gob_piv, CARL_X + 40, 0),
        (llama, piv, CARL_X + 110, 0),
        (poses[0], piv, CARL_X + 220, 0),
        (falling, fa, CARL_X + 220 + (BLOCK_X - PIVOT[0]), PIVOT[1] - STONE_ROW, False),   # stone drawn over the llama
        (landed, la, CARL_X + 330, 0, False),
        (poses[2], piv, CARL_X + 330, 0),
        (poses[3], piv, CARL_X + 440, 0),
    ]
    bottom = cast + [
        (poses[5], piv, CARL_X + 70, 0),
        (poses[6], piv, CARL_X + 170, 0),
        (poses[7], piv, CARL_X + 270, 0),
    ] + _scene_parts(parts, CARL_X + 350)
    floor = Floor()
    scenes = [floor.render(top, 1), floor.render(bottom, 1)]
    ground = floor.ground(CARL_X) - floor.view[1]
    w, h = scenes[0].size
    sheet = Image.new("RGB", (w, h * 2), (24, 23, 34))
    bands = []
    for i, sc in enumerate(scenes):
        sheet.paste(sc, (0, i * h))
        bands.append(sc.crop((0, max(0, ground - 100), w, min(h, ground + 14))))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    bw, bh = bands[0].size
    big = Image.new("RGB", (bw * 4, bh * 8 + 8), (24, 23, 34))
    for i, band in enumerate(bands):
        big.paste(band.resize((bw * 4, bh * 4), Image.NEAREST), (0, i * (bh * 4 + 8)))
    big.save(out.with_name("silhouette_scale_4x.png"))
    return out


def write_scene(out: Path = EVIDENCE / "scene_640x360.png") -> Path:
    """The in-scene composition as the runtime draws it: enemies mirrored to face Carl (the flip around the pivot
    column), Carl and Donut unobstructed on the left, then the crush under the falling block, the remains in front of
    the landed block, the decapitation tick and the broken heap, at 1x and at 3x (1920 x 1080)."""
    from PIL import Image
    from scripts.pixelart.common import CARL_X, DONUT_X, flip
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    poses = _images(poses_grid())
    falling, fa = block_piece("loose_block_falling")
    landed, la = block_piece("loose_block_landed")

    def m(n):
        return flip(poses[n], PIVOT)

    entries = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
    x = CARL_X + 120
    entries += [(*m(0), x, 0), (falling, fa, x - (BLOCK_X - PIVOT[0]), PIVOT[1] - STONE_ROW, False)]
    x += 120
    entries += [(landed, la, x, 0, False), (*m(2), x, 0)]
    x += 110
    entries += [(*m(3), x, 0)]
    x += 105
    entries += [(*m(7), x, 0)]
    floor = Floor()
    img1 = floor.render(entries, 1)
    out.parent.mkdir(parents=True, exist_ok=True)
    img1.save(out)
    floor.render(entries, 3).save(out.with_name("scene_1920x1080.png"))
    return out


# --------------------------------------------------------------------------- manifests (spec sections 2 and 7)
POSE_ANCHORS = {   # frame coordinates, facing right; frames 1 and 4 carry their key's anchors until the executor moves them
    0: {"hit_origin": [42, 52], "mouth": [90, 60], "gland": [91, 58], "squeeze_back": [4, 70], "squeeze_front": [76, 70]},
    2: {"pool_origin": [40, 71], "mouth": [90, 69], "split": [41, 58]},
    3: {"neck": [54, 22], "hit_origin": [54, 22], "jet_base": [54, 21], "gland": [56, 22]},
    5: {"neck": [76, 55], "gland": [76, 55], "pool_origin": [80, 71]},
    6: {"hit_origin": [42, 68], "mouth": [89, 66], "kink": [69, 53], "gland": [87, 58]},
    7: {"pool_origin": [82, 71], "mouth": [90, 69], "kink": [69, 52]},
}
POSE_ANCHORS[1] = POSE_ANCHORS[0]
POSE_ANCHORS[4] = POSE_ANCHORS[3]
POSE_AUTHOR = ["key", "executor", "key", "key", "executor", "key", "key", "key"]
PLAYBACK = {"crush_contact": "once", "crushed_remains": "hold", "decap_contact": "once", "headless_collapse": "hold",
            "slam_contact": "once", "broken_heap": "hold"}
# id -> (class, material, mass, spin, settle, bounce, swap_on_first_contact, stain, trail, ttl, presets)
PART_META = {
    "head": ("wound", "llama_flesh", "medium", "fast", "roll", 0.3, "head_dead", "decal_splat_s", "decal_smear", "keep",
             [1, 2, 3, 4]),
    "neck": ("wound", "llama_flesh", "medium", "slow", "flop", 0.2, None, "decal_splat_s", None, "keep", [3, 4]),
    "leg_fore": ("wound", "llama_flesh", "medium", "fast", "clatter", 0.3, None, "decal_splat_s", None, "long", [3, 4]),
}


def _cell(pt, pid):
    dx, dy = CELL_SHIFT[pid]
    return [int(math.floor(pt[0])) + dx, int(math.floor(pt[1])) + dy]


def part_entries(grid: dict) -> list[dict]:
    hoof = L.IDLE["near_front_hoof"]
    anchors = {
        "head": {"neck": _cell(HEAD_AT, "head")},
        "neck": {"top": _cell(NECK_TOP[0], "neck"), "bottom": _cell(NECK_BOTTOM[0], "neck")},
        "leg_fore": {"shoulder": _cell(LEG_AT, "leg_fore"), "hoof": _cell((hoof[0] + 0.5, hoof[1] + 1), "leg_fore")},
    }
    out = []
    for i, pid in enumerate(PART_IDS):
        frame = grid["frames"][i]["cels"]
        cls, material, mass, spin, settle, bounce, swap, stain, trail, ttl, presets = PART_META[pid]
        entry = {"id": pid, "frame": i, "cell": list(CELL), "pivot": list(CELL_PIVOT), "anchors": anchors[pid],
                 "frame0_origin": [CELL_PIVOT[0] - CELL_SHIFT[pid][0], CELL_PIVOT[1] - CELL_SHIFT[pid][1]],
                 "class": cls, "material": material, "presets": presets,
                 "gore_layer": any(ch != "." for row in frame["gore"] for ch in row),
                 "molten_layer": any(ch != "." for row in frame["molten"] for ch in row),
                 "mass": mass, "spin": spin, "settle": settle, "bounce": bounce}
        if swap:
            entry["swap_on_first_contact"] = swap
        if stain:
            entry["stain"] = stain
        if trail:
            entry["trail"] = trail
        entry["ttl"] = ttl
        out.append(entry)
    return out


PROVENANCE_ORIGIN = ("LLAMA-02-DEATH Bad Llama death art at intensity preset 4 (docs/02-art/llama-02-death-spec.md); "
                     "pixel-grid-v1 source written by scripts/author_llama02_death.py with the LLAMA-01 parametric rig, "
                     "new pose dicts, shape rules and literal stamps; no image-model output, no tracing")
PROVENANCE_GEN = "Design, spec and hero keys: Claude Opus. Execution and independent review pending"
LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
           "no game/asset-pack distribution rights")
STATUS = "design stage: keys frozen, in-betweens and parts 3 onward pending (executor); not wired into any Bevy binary"


def manifest_base(mid: str, stem: str, grid: Path, ase: Path, frame, pivot, tags: list[str]) -> dict:
    return {
        "schema_version": 1, "id": mid, "type": "character_animation",
        "source_path": str(ase.relative_to(ROOT)), "grid_source": str(grid.relative_to(ROOT)),
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "llama-death-floor01-v0", "frame_size": list(frame), "pivot": list(pivot),
        "animation_tags": tags, "required_alpha": True,
        "owner": "AI-authored enemy death art (fan-concept layer: Floor 1 Bad Llama; runtime role IP-neutral ranged spitter)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {"origin": PROVENANCE_ORIGIN, "generator": PROVENANCE_GEN, "generated_art": True,
                       "input_reference_ids": ["assets/concept/floor-01/enemies/bad-llama-c-plus-a-approved.png"]},
        "license": LICENSE, "review_status": "prototype_fixture_unapproved",
    }


def pose_manifest() -> dict:
    grid = poses_grid()
    m = manifest_base("llama_death_h64_candidate_v1", "llama_death_h64", POSES, POSES_ASE, CANVAS, PIVOT,
                      [t[0] for t in POSE_TAGS])
    frames = []
    for i, author in enumerate(POSE_AUTHOR):
        tag = next(n for n, a, b in POSE_TAGS if a <= i <= b)
        frames.append({"index": i, "tag": tag, "ticks": POSE_TICKS[i] or None,
                       "duration_ms": grid["frames"][i]["duration_ms"], "author": author, "anchors": POSE_ANCHORS[i]})
    m["death_poses"] = {
        "spec": "docs/02-art/llama-02-death-spec.md", "variant": "A", "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS,
        "status": STATUS, "facing": "right", "mirror": "x' = 88 - x around the pivot column, anchors included",
        "key_frames": list(POSE_KEYS), "key_fixture": "tests/fixtures/llama-02-death_keys.json",
        "layers": LAYERS, "preset_layer": "gore", "always_shown_layers": ["body", "molten"], "default_preset": 4,
        "playback": PLAYBACK, "frames": frames,
    }
    return m


def parts_manifest() -> dict:
    grid = parts_grid()
    m = manifest_base("llama_death_parts_v1", "llama_death_parts", PARTS, PARTS_ASE, CELL, CELL_PIVOT, PART_IDS)
    m["death_parts"] = {
        "spec": "docs/02-art/llama-02-death-spec.md", "variant": "A", "cell": list(CELL), "pivot": list(CELL_PIVOT),
        "layers": LAYERS, "mirror": "x' = 32 - x around the cell pivot", "status": STATUS,
        "key_frames": list(PART_KEYS), "key_fixture": "tests/fixtures/llama-02-death-parts_keys.json",
        "preset_filter": {"1": "clean and molten parts, and head",
                          "2": "wound (gore muted), clean, molten, blood drops and s decals",
                          "3": "wound, clean, molten, gib (no bone), blood (pools 1 and 2)", "4": "all"},
        "parts": part_entries(grid),
    }
    return m


MANIFESTS = (("llama_death_h64.json", pose_manifest), ("llama_death_parts.json", parts_manifest))


def manifest_text(build) -> str:
    return json.dumps(build(), indent=2) + "\n"


# --------------------------------------------------------------------------- main
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    sys.path.insert(0, str(ROOT))
    if args.command == "scale":
        for path in (write_key_sheet(), write_scale_sheet(), write_scene()):
            print(f"wrote {path.relative_to(ROOT)}")
        return 0
    outputs = [(POSES, POSES_ASE, poses_grid()), (PARTS, PARTS_ASE, parts_grid())]
    manifests = [(ROOT / "assets/manifests" / name, manifest_text(build)) for name, build in MANIFESTS]
    if args.check:
        same = all(p.is_file() and p.read_text(encoding="utf-8") == dump(g) for p, _a, g in outputs)
        same = same and all(p.is_file() and p.read_text(encoding="utf-8") == t for p, t in manifests)
        print("death sources up to date" if same else "death sources differ from the generator")
        return 0 if same else 1
    import pixel_grid_to_ase as pga  # noqa: E402
    for path, ase, grid in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(dump(grid), encoding="utf-8")
        pga.write_ase(grid, ase)
        print(f"wrote {path.relative_to(ROOT)}, {ase.relative_to(ROOT)}")
    for path, text in manifests:
        path.write_text(text, encoding="utf-8")
        print(f"wrote {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
