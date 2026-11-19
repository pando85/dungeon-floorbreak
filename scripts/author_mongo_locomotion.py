#!/usr/bin/env python3
"""Authoring record for ANIM-MONGO-01: adult Mongo 76 px locomotion (design stage).

Writes ``assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json`` (pixel-grid-v1),
the editable source for the tags ``idle`` (the approved 76 px idle, unchanged), ``run``,
``jump_start``, ``jump_rise``, ``jump_fall`` and ``land``. The design spec is
``docs/02-art/mongo-locomotion-anim-01-spec.md``.

How the frames are made (no resampling, no rotation, no image model, no concept pixels):

* **Copied from the approved master** ``mongo_adult_h76.grid.json`` frame 0 (never modified
  here): head, crest and mane, torso and neck, tail plume, wing fan, near thigh and the foot
  (toes, claws, sickle claw). They are only translated or edited with the pixel-safe
  operations below, so the face, mask, eye and feather map stay exact.
* **Pixel-safe edits** (whole-pixel, per row or per column, never resampled):
  ``neck`` (head rigidly moved; neck rows dropped and sheared forward, the cut neck columns
  filled by repeating the column at the seam), ``row_shear`` (per-row x offset) and
  ``col_shear`` (per-column y offset) for the tail streaming back and the wing fan trailing.
* **Rule-drawn parts** (deterministic, from numbers in ``POSES``): the shin and metatarsus
  are drawn as tapered capsules from knee to hock to the ball of the foot, shaded with the
  master leg ramp (lit back edge ``S``, base ``s``, shade ``z``, outline ``Z``; the far leg
  uses the darker master far-leg ramp ``s``/``z``/``Z``). The hock is placed by two-bone
  inverse kinematics with the hock always behind the knee-ball line (bird leg).
* **Hand-typed parts** in this file: the curled swing foot and the reaching foot.
* **Cleanup** (``PATCHES``): per-pixel fixes after the lint report, each with its expected
  old value so a regenerated pose cannot silently move under a patch.

Frame layout: canvas 128x88, pivot (76, 87). The idle is the master 108x80 canvas placed at
offset (8, 8), so the master pivot (68, 79) lands exactly on the locomotion pivot and the
idle-to-run switch does not move the feet, the face or the body.

Usage::

    python3 scripts/author_mongo_locomotion.py --write         # write the grid (all frames)
    python3 scripts/author_mongo_locomotion.py --check-keys    # keys in the grid == recipe
    python3 scripts/author_mongo_locomotion.py --preview DIR   # scratch renders for review

Since the design stage the keys (frames 4, 5, 6, 10, 12, 13, 14) are frozen: ``--check-keys``
must keep passing. The executor finishes frames 7, 8, 9, 11 and 15 (see the spec, section 9).
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MASTER_PATH = ROOT / "assets/source/characters/mongo/mongo_adult_h76.grid.json"
OUT_PATH = ROOT / "assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json"

CANVAS = (128, 88)
PIVOT = (76, 87)
G = 87                      # contact row (pivot y)
OX, OY = 8, 8               # master idle placement in the locomotion canvas

MASTER = json.loads(MASTER_PATH.read_text(encoding="utf-8"))
LAYERS = list(MASTER["layers"])  # tail, far_leg, far_arm, body, near_leg, head, crest, near_arm

Part = dict  # {(x, y): key} in locomotion-canvas coordinates


# ---------------------------------------------------------------------------
# Master parts (frame 0 of the approved idle), in locomotion coordinates
# ---------------------------------------------------------------------------

def master_layer(layer: str, frame: int = 0) -> Part:
    rows = MASTER["frames"][frame]["cels"][layer]
    return {(x + OX, y + OY): c for y, r in enumerate(rows) for x, c in enumerate(r) if c != "."}


def select(part: Part, pred) -> Part:
    return {p: c for p, c in part.items() if pred(*p)}


def moved(part: Part, dx: int, dy: int) -> Part:
    return {(x + dx, y + dy): c for (x, y), c in part.items()}


def recolor(part: Part, mapping: dict) -> Part:
    return {p: mapping.get(c, c) for p, c in part.items()}


def row_shear(part: Part, k: float, anchor_y: int, x_floor: int | None = None) -> Part:
    """Shift each row left by round((anchor_y - y) * k) (rows above the anchor move back).

    Negative k moves rows above the anchor forward. Rows below the anchor move the other way.
    """
    out: Part = {}
    for (x, y), c in sorted(part.items(), key=lambda t: (t[0][1], t[0][0])):
        out[(x - round((anchor_y - y) * k), y)] = c
    return out


def col_shear(part: Part, k: float, anchor_x: int) -> Part:
    """Raise each column by round((anchor_x - x) * k) (columns behind the anchor move up)."""
    out: Part = {}
    for (x, y), c in sorted(part.items(), key=lambda t: (t[0][0], t[0][1])):
        out[(x, y - round((anchor_x - x) * k))] = c
    return out


TAIL = master_layer("tail")
BODY = master_layer("body")
HEAD = master_layer("head")
CREST = master_layer("crest")
WING = master_layer("near_arm")
FAR_WING = master_layer("far_arm")
_NEAR_LEG = master_layer("near_leg")
THIGH = select(_NEAR_LEG, lambda x, y: y <= 57 + OY)                   # feathered drumstick
# The far thigh is hidden in the idle; it is the near drumstick one step darker.
FAR_THIGH = recolor(THIGH, {"1": "3", "2": "3", "3": "4", "4": "5", "5": "5"})

# Foot stamp, anchored at the ball of the foot (ANCHOR = bottom-centre of the metatarsus).
# Copied from the idle near foot (rows 75-79) plus the raised sickle claw (rows 68-71).
_FOOT_ROWS = {y: r for y, r in enumerate(MASTER["frames"][0]["cels"]["near_leg"])}
FOOT_ANCHOR_IDLE = (57, 75)   # master coords: metatarsus bottom centre


def _idle_foot() -> Part:
    out: Part = {}
    ax, ay = FOOT_ANCHOR_IDLE
    for y in range(75, 80):
        for x, c in enumerate(_FOOT_ROWS[y]):
            if c != ".":
                out[(x - ax, y - ay)] = c
    # sickle claw (rows 68-71, x >= 61 only; its stalk at row 71 x 60-61)
    for y in range(68, 72):
        for x, c in enumerate(_FOOT_ROWS[y]):
            if c != "." and x >= 60:
                out[(x - ax, y - ay)] = c
    return out


FOOT_PLANT = _idle_foot()
FOOT_PLANT_FAR = recolor(FOOT_PLANT, {"S": "s", "s": "z", "z": "z", "Z": "Z"})


def stamp(text: str, ax: int, ay: int) -> Part:
    """Hand-typed stamp; (ax, ay) is the anchor cell inside the text block."""
    out: Part = {}
    for y, r in enumerate(text.strip("\n").split("\n")):
        for x, c in enumerate(r):
            if c not in ". ":
                out[(x - ax, y - ay)] = c
    return out


# Curled swing foot: toes folded down and back under the ball, claws hooked, sickle claw
# tucked. Anchor = ball (top centre of the knuckle). Hand-typed.
FOOT_CURL = stamp("""
.SsszZ.
SsssszZ
Ssssz.Z
.ZszZgG
..Zz.gg
..gG.g.
..gg...
""", 3, 0)
FOOT_CURL_FAR = recolor(FOOT_CURL, {"S": "s", "s": "z"})

# Reaching foot: toes spread forward and slightly down, claws leading, sickle claw raised.
# Anchor = ball. Hand-typed.
FOOT_REACH = stamp("""
.....GG.......
....GggG......
...gg.gg......
.SssszZ.g.....
SsssssszSSSZgG
SsssszZsssszZgG
.zzzzZ..zzzzZgg
..ZZZ....ZZZ..g
""", 3, 3)
FOOT_REACH_FAR = recolor(FOOT_REACH, {"S": "s", "s": "z"})

FEET = {
    ("plant", "near"): FOOT_PLANT, ("plant", "far"): FOOT_PLANT_FAR,
    ("curl", "near"): FOOT_CURL, ("curl", "far"): FOOT_CURL_FAR,
    ("reach", "near"): FOOT_REACH, ("reach", "far"): FOOT_REACH_FAR,
}


# ---------------------------------------------------------------------------
# Rule-drawn leg segments
# ---------------------------------------------------------------------------

def capsule(p0, p1, r0: float, r1: float, ramp: str) -> Part:
    """Tapered capsule from p0 to p1. ``ramp`` = lit, base, shade, outline keys.

    Shading by the signed distance across the segment: the side facing back/up (the light,
    upper left) gets the lit key, the opposite edge the outline key.
    """
    lit, base, shade, outline = ramp
    (x0, y0), (x1, y1) = p0, p1
    vx, vy = x1 - x0, y1 - y0
    L2 = vx * vx + vy * vy or 1.0
    # normal pointing to the lit side (towards -x / -y)
    nx, ny = -vy, vx
    if nx * -1 + ny * -0.6 < 0:
        nx, ny = -nx, -ny
    nl = math.hypot(nx, ny) or 1.0
    nx, ny = nx / nl, ny / nl
    out: Part = {}
    rmax = max(r0, r1) + 1
    for y in range(int(min(y0, y1) - rmax), int(max(y0, y1) + rmax) + 1):
        for x in range(int(min(x0, x1) - rmax), int(max(x0, x1) + rmax) + 1):
            cx, cy = x + 0.5, y + 0.5
            t = max(0.0, min(1.0, ((cx - x0) * vx + (cy - y0) * vy) / L2))
            px, py = x0 + t * vx, y0 + t * vy
            r = r0 + (r1 - r0) * t
            dx, dy = cx - px, cy - py
            if dx * dx + dy * dy > r * r:
                continue
            s = (dx * nx + dy * ny) / r        # +1 lit edge .. -1 dark edge
            if s > 0.55:
                c = lit
            elif s > -0.35:
                c = base
            elif s > -0.75:
                c = shade
            else:
                c = outline
            out[(x, y)] = c
    return out


NEAR_RAMP = "SszZ"
FAR_RAMP = "szZZ"
SHIN_LEN, META_LEN = 14.0, 11.0


def solve_hock(knee, ball, l1=SHIN_LEN, l2=META_LEN):
    """Two-bone IK; the hock bends backwards (towards -x), like a bird's ankle."""
    kx, ky = knee
    bx, by = ball
    dx, dy = bx - kx, by - ky
    d = math.hypot(dx, dy)
    d = min(d, l1 + l2 - 0.01)
    d = max(d, abs(l1 - l2) + 0.01)
    a = (l1 * l1 - l2 * l2 + d * d) / (2 * d)
    h = math.sqrt(max(0.0, l1 * l1 - a * a))
    ux, uy = dx / (math.hypot(dx, dy) or 1), dy / (math.hypot(dx, dy) or 1)
    mx, my = kx + a * ux, ky + a * uy
    # two candidates; pick the one further back (smaller x)
    c1 = (mx + h * -uy, my + h * ux)
    c2 = (mx - h * -uy, my - h * ux)
    return c1 if c1[0] < c2[0] else c2


def leg(side: str, knee, ball, foot: str, thigh_xy) -> Part:
    """Thigh stamp at thigh_xy offset, shin + metatarsus capsules, foot stamp at the ball."""
    ramp = NEAR_RAMP if side == "near" else FAR_RAMP
    hock = solve_hock(knee, ball)
    out: Part = {}
    thigh = THIGH if side == "near" else FAR_THIGH
    out.update(capsule(knee, hock, 3.1, 2.6, ramp))
    out.update(capsule(hock, ball, 2.6, 2.1, ramp))
    out.update({(x + ball[0], y + ball[1]): c for (x, y), c in FEET[(foot, side)].items()})
    out.update(moved(thigh, *thigh_xy))
    return out


# ---------------------------------------------------------------------------
# Upper body (torso, neck, head, crest, wing, tail)
# ---------------------------------------------------------------------------

NECK_TOP = 26 + OY        # master row 26: below the head
NECK_HINGE = 44 + OY      # master row 44: neck base (no lean below)
NECK_SEAM = 74 + OX       # body columns >= seam belong to the neck/chest in rows >= 36
NECK_DROP_AT = 30 + OY    # rows dropped from the neck here when the head comes down


def lean_of(y: int, lean: int) -> int:
    if y <= NECK_TOP:
        return lean
    if y >= NECK_HINGE:
        return 0
    return round(lean * (NECK_HINGE - y) / (NECK_HINGE - NECK_TOP))


def upper(lean: int, drop: int) -> dict[str, Part]:
    """Head, crest and neck leaning forward by ``lean`` px at the head and lowered by ``drop``."""
    def vdrop(y):  # vertical move of a source row
        return drop if y < NECK_DROP_AT else 0

    body: Part = {}
    for (x, y), c in BODY.items():
        if NECK_DROP_AT <= y < NECK_DROP_AT + drop:
            continue                                    # dropped neck rows
        neck_px = y < 36 + OY or x >= NECK_SEAM
        s = lean_of(y, lean) if neck_px else 0
        body[(x + s, y + vdrop(y) * (1 if y < NECK_DROP_AT else 0))] = c
    # fill the seam gap: repeat the seam column into the gap in the leaning chest rows
    for y in range(36 + OY, NECK_HINGE):
        s = lean_of(y, lean)
        if s and (NECK_SEAM, y) in BODY:
            for i in range(s):
                body.setdefault((NECK_SEAM + i, y), BODY[(NECK_SEAM, y)])
    head = {(x + lean, y + drop): c for (x, y), c in HEAD.items()}
    crest: Part = {}
    for (x, y), c in sorted(CREST.items(), key=lambda t: -t[0][1]):
        if y < NECK_DROP_AT:
            crest[(x + lean_of(y, lean), y + drop)] = c
        else:
            crest.setdefault((x + lean_of(y, lean), y), c)
    return {"body": body, "head": head, "crest": crest}


KNEE = (60.5 + OX, 57.5 + OY)   # master knee: top centre of the shin under the drumstick
TAIL_ROOT = (50 + OX, 50 + OY)   # tail base under the rump
WING_ROOT_Y = 46 + OY


def pose(spec: dict) -> dict[str, list[str]]:
    bdx, bdy = spec["body"]
    up = upper(spec["lean"], spec["drop"])
    layers: dict[str, Part] = {name: {} for name in LAYERS}
    # tail: stream back (row shear) and lift the far end (column shear), then sway
    tail = row_shear(TAIL, spec["tail_kx"], TAIL_ROOT[1])
    tail = col_shear(tail, spec["tail_ky"], TAIL_ROOT[0])
    layers["tail"] = moved(tail, bdx + spec.get("tail_dx", 0), bdy + spec.get("tail_dy", 0))
    layers["body"] = moved(up["body"], bdx, bdy)
    hdx, hdy = spec.get("head", (0, 0))
    layers["head"] = moved(up["head"], bdx + hdx, bdy + hdy)
    layers["crest"] = moved(up["crest"], bdx + hdx, bdy + hdy)
    wing = row_shear(WING, -spec["wing_k"], WING_ROOT_Y)
    layers["near_arm"] = moved(wing, bdx + spec.get("wing_dx", 0), bdy + spec.get("wing_dy", 0))
    layers["far_arm"] = moved(row_shear(FAR_WING, -spec["wing_k"], WING_ROOT_Y), bdx, bdy)
    # legs: hip-relative thigh, knee, ball
    for side, layer in (("near", "near_leg"), ("far", "far_leg")):
        L = spec[side]
        tx, ty = L["thigh"]
        knee = (KNEE[0] + bdx + tx, KNEE[1] + bdy + ty)
        layers[layer] = leg(side, knee, L["ball"], L["foot"], (bdx + tx, bdy + ty))
    for name, part in layers.items():
        for (x, y) in part:
            if not (0 <= x < CANVAS[0] and 0 <= y < CANVAS[1]):
                raise ValueError(f"{spec.get('name')}: {name} pixel off canvas at {x},{y}")
    return {name: to_rows(part) for name, part in layers.items()}


def to_rows(part: Part) -> list[str]:
    grid = [["."] * CANVAS[0] for _ in range(CANVAS[1])]
    for (x, y), c in part.items():
        grid[y][x] = c
    return ["".join(r) for r in grid]


# ---------------------------------------------------------------------------
# Poses (keys). Coordinates: locomotion canvas, ground row 87, pivot x 76.
# body = translation of the master torso; lean/drop = head forward/down via the neck.
# ---------------------------------------------------------------------------

RUN_BODY_DX = 3

POSES = {
    # F0 contact: near foot strikes ahead, far leg trails high behind
    4: dict(name="run F0 contact", body=(RUN_BODY_DX, 1), lean=7, drop=3, head=(0, 0),
            tail_kx=0.45, tail_ky=-0.04, wing_k=0.30,
            near=dict(thigh=(3, 0), ball=(90, 83), foot="plant"),
            far=dict(thigh=(-3, -1), ball=(60, 72), foot="curl")),
    # F1 passing: near foot under the hip carrying the weight, far leg swinging through
    5: dict(name="run F1 passing", body=(RUN_BODY_DX, 3), lean=7, drop=1, head=(0, 0),
            tail_kx=0.40, tail_ky=-0.02, wing_k=0.25,
            near=dict(thigh=(0, 0), ball=(74, 83), foot="plant"),
            far=dict(thigh=(1, -2), ball=(78, 70), foot="curl")),
    # F2 airborne: near foot just pushed off behind, far leg reaching forward
    6: dict(name="run F2 airborne", body=(RUN_BODY_DX, 0), lean=7, drop=3, head=(0, 0),
            tail_kx=0.50, tail_ky=-0.06, wing_k=0.35,
            near=dict(thigh=(-3, 0), ball=(52, 82), foot="plant"),
            far=dict(thigh=(3, -1), ball=(89, 79), foot="reach")),
    # J0 crouch: feet together under the hips, deep knee flex, head pulled in, tail held up
    10: dict(name="jump_start J0 crouch", body=(1, 4), lean=2, drop=4, head=(0, 0),
             tail_kx=0.15, tail_ky=0.10, wing_k=0.10,
             near=dict(thigh=(1, 0), ball=(67, 83), foot="plant"),
             far=dict(thigh=(2, -1), ball=(81, 83), foot="plant")),
    # J2 rise: legs trail extended down and back, body stretched forward, tail streams low
    12: dict(name="jump_rise", body=(RUN_BODY_DX, -6), lean=9, drop=0, head=(0, 0),
             tail_kx=0.55, tail_ky=-0.10, wing_k=0.45,
             near=dict(thigh=(-3, 0), ball=(50, 80), foot="plant"),
             far=dict(thigh=(2, -2), ball=(72, 68), foot="curl")),
    # J3 fall: legs reach forward and down for the landing, tail and wing float up
    13: dict(name="jump_fall", body=(RUN_BODY_DX, -2), lean=5, drop=1, head=(0, 0),
             tail_kx=0.20, tail_ky=0.14, wing_k=-0.05,
             near=dict(thigh=(3, 0), ball=(87, 81), foot="reach"),
             far=dict(thigh=(2, -1), ball=(80, 78), foot="reach")),
    # L0 impact: lowest pose, wide planted stance, head dropped, tail whips up
    14: dict(name="land L0 impact", body=(2, 7), lean=4, drop=6, head=(0, 0),
             tail_kx=0.05, tail_ky=0.18, wing_k=-0.15,
             near=dict(thigh=(2, 0), ball=(88, 83), foot="plant"),
             far=dict(thigh=(-1, -1), ball=(60, 83), foot="plant")),
}


KEY_FRAMES = sorted(POSES)


def swap_legs(spec: dict, name: str) -> dict:
    out = dict(spec, name=name)
    out["near"], out["far"] = dict(spec["far"]), dict(spec["near"])
    return out


# Executor frames: rig drafts only (block-in). The executor finishes them by hand to the
# targets in the spec, section 9. They are NOT keys and are not checked by --check-keys.
DRAFTS = {
    7: swap_legs(POSES[4], "run F3 contact (far) draft"),
    8: swap_legs(POSES[5], "run F4 passing (far) draft"),
    9: swap_legs(POSES[6], "run F5 airborne draft"),
    11: dict(name="jump_start J1 push-off draft", body=(2, 0), lean=6, drop=2, head=(0, 0),
             tail_kx=0.35, tail_ky=0.0, wing_k=0.30,
             near=dict(thigh=(-2, 0), ball=(60, 83), foot="plant"),
             far=dict(thigh=(-1, -1), ball=(68, 83), foot="plant")),
    15: dict(name="land L1 recover draft", body=(2, 3), lean=3, drop=2, head=(0, 0),
             tail_kx=0.12, tail_ky=0.06, wing_k=0.05,
             near=dict(thigh=(1, 0), ball=(80, 83), foot="plant"),
             far=dict(thigh=(0, -1), ball=(66, 83), foot="plant")),
}

# Tag table: (name, frames, ticks at 60 Hz; None = hold until the state changes)
TAGS = [
    ("idle", [0, 1, 2, 3], [27, 18, 33, 10]),
    ("run", [4, 5, 6, 7, 8, 9], [5, 5, 5, 5, 5, 5]),
    ("jump_start", [10, 11], [6, 4]),
    ("jump_rise", [12], [None]),
    ("jump_fall", [13], [None]),
    ("land", [14, 15], [5, 7]),
]
HOLD_MS = 100


def ticks_ms(t) -> int:
    return HOLD_MS if t is None else round(t * 1000 / 60)


def idle_frame(i: int) -> dict[str, list[str]]:
    return {name: to_rows(master_layer(name, i)) for name in LAYERS}


def apply_patches(frame: int, cels: dict[str, list[str]]) -> None:
    for layer, x, y, old, new in PATCHES.get(frame, []):
        row = cels[layer][y]
        if row[x] != old:
            raise ValueError(f"patch {frame} {layer} ({x},{y}): expected {old!r}, found {row[x]!r}")
        cels[layer][y] = row[:x] + new + row[x + 1:]


# Per-pixel cleanup after the lint report: (layer, x, y, expected old key, new key).
PATCHES: dict[int, list[tuple]] = {}


TIDY_PROTECTED = set("xYOywG")   # eye, catchlight, teeth and claw rims are never recoloured
_N8 = ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1))
_PAL_ORDER = [p["key"] for p in MASTER["palette"]]


def tidy(cels: dict[str, list[str]], passes: int = 3) -> int:
    """Rule-based orphan pass (deterministic): a non-accent pixel with no same-key pixel among
    its 8 composite neighbours takes the most common opaque neighbour key (ties: palette
    order), on the topmost layer that owns it. Shears and joins leave these 1 px seams.
    Protected keys (eye, teeth, claw rims) are never changed. Returns pixels changed."""
    W, H = CANVAS
    changed = 0
    for _ in range(passes):
        flat = [["."] * W for _ in range(H)]
        owner = [[None] * W for _ in range(H)]
        for layer in LAYERS:
            for y, r in enumerate(cels[layer]):
                for x, c in enumerate(r):
                    if c != ".":
                        flat[y][x] = c
                        owner[y][x] = layer
        edits = []
        for y in range(H):
            for x in range(W):
                c = flat[y][x]
                if c == "." or c in TIDY_PROTECTED:
                    continue
                around = [flat[y + dy][x + dx] for dx, dy in _N8 if 0 <= x + dx < W and 0 <= y + dy < H]
                opaque = [n for n in around if n != "."]
                if not opaque or c in around:
                    continue
                best = max(sorted(set(opaque), key=_PAL_ORDER.index), key=opaque.count)
                edits.append((owner[y][x], x, y, best))
        if not edits:
            break
        for layer, x, y, key in edits:
            row = cels[layer][y]
            cels[layer][y] = row[:x] + key + row[x + 1:]
        changed += len(edits)
    return changed


def build_frames(include_drafts: bool = True) -> dict[int, dict[str, list[str]]]:
    specs = dict(POSES)
    if include_drafts:
        specs.update(DRAFTS)
    out = {}
    for i, spec in sorted(specs.items()):
        cels = pose(spec)
        tidy(cels)
        apply_patches(i, cels)
        out[i] = cels
    return out


def build_grid() -> dict:
    frames_cels = {i: idle_frame(i) for i in range(4)}
    frames_cels.update(build_frames())
    frames = []
    ticks = {f: t for _, fs, ts in TAGS for f, t in zip(fs, ts)}
    for i in range(16):
        cels = {name: rows for name, rows in frames_cels[i].items() if any(c != "." for r in rows for c in r)}
        frames.append({"duration_ms": ticks_ms(ticks[i]), "cels": cels})
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": MASTER["palette"],
        "layers": LAYERS,
        "frames": frames,
        "tags": [{"name": n, "from": fs[0], "to": fs[-1], "direction": "forward"} for n, fs, _ in TAGS],
    }


def check_keys(grid: dict) -> list[str]:
    errors = []
    want = build_frames(include_drafts=False)
    for i in KEY_FRAMES:
        got = grid["frames"][i]["cels"]
        for layer in LAYERS:
            exp = want[i][layer]
            have = got.get(layer, ["." * CANVAS[0]] * CANVAS[1])
            if exp != have:
                errors.append(f"key frame {i} layer {layer} differs from its recipe")
    for i in range(4):
        exp = idle_frame(i)
        got = grid["frames"][i]["cels"]
        for layer in LAYERS:
            if exp[layer] != got.get(layer, ["." * CANVAS[0]] * CANVAS[1]):
                errors.append(f"idle frame {i} layer {layer} is not the master idle")
    return errors


# ---------------------------------------------------------------------------
# Preview (scratch review only)
# ---------------------------------------------------------------------------

def preview(out_dir: Path, scale: int = 4) -> None:
    from PIL import Image, ImageDraw
    pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for p in MASTER["palette"]}
    frames = build_frames()
    idle = {name: to_rows(master_layer(name)) for name in LAYERS}
    seq = [("idle", idle)] + [(f"{i} " + {**POSES, **DRAFTS}[i]["name"], f) for i, f in sorted(frames.items())]
    W, H = CANVAS
    cols = 4
    nrows = (len(seq) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * (W * scale + 8), nrows * (H * scale + 20)), (25, 26, 44, 255))
    small = Image.new("RGBA", (len(seq) * (W + 4), H), (44, 40, 65, 255))
    for i, (name, cels) in enumerate(seq):
        img = Image.new("RGBA", (W, H), (44, 40, 65, 255))
        for layer in LAYERS:
            for y, r in enumerate(cels.get(layer, [])):
                for x, c in enumerate(r):
                    if c != ".":
                        img.putpixel((x, y), pal[c])
        small.paste(img, (i * (W + 4), 0))
        big = img.resize((W * scale, H * scale), Image.NEAREST)
        d = ImageDraw.Draw(big)
        d.line([(0, G * scale + scale - 1), (W * scale, G * scale + scale - 1)], fill=(120, 90, 60, 255))
        d.line([(PIVOT[0] * scale, 0), (PIVOT[0] * scale, H * scale)], fill=(70, 70, 110, 255))
        sx, sy = (i % cols) * (W * scale + 8), (i // cols) * (H * scale + 20)
        sheet.paste(big, (sx, sy + 20))
        ImageDraw.Draw(sheet).text((sx + 4, sy + 4), name, fill=(230, 230, 230, 255))
    out_dir.mkdir(parents=True, exist_ok=True)
    sheet.save(out_dir / "keys.png")
    small.resize((small.width * 2, small.height * 2), Image.NEAREST).save(out_dir / "keys_2x.png")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--write", action="store_true", help="write the grid (keys and executor drafts)")
    ap.add_argument("--out", type=Path, default=OUT_PATH)
    ap.add_argument("--check-keys", action="store_true", help="verify key and idle frames in the grid")
    ap.add_argument("--preview", type=Path)
    args = ap.parse_args(argv)
    if args.write:
        args.out.write_text(json.dumps(build_grid(), indent=1) + "\n", encoding="utf-8")
        print(f"wrote {args.out.relative_to(ROOT) if args.out.is_relative_to(ROOT) else args.out}")
    if args.check_keys:
        errors = check_keys(json.loads(OUT_PATH.read_text(encoding="utf-8")))
        if errors:
            print("\n".join(errors))
            return 1
        print(f"keys {KEY_FRAMES} and idle 0-3 match their recipe")
    if args.preview:
        preview(args.preview)
    return 0


if __name__ == "__main__":
    sys.exit(main())
