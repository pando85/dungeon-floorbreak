#!/usr/bin/env python3
"""Author TUSK-01 Tuskling (Floor 01 arena minion) hero key as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/tusk-01-spec.md``. One base Tuskling (layer ``body``) wearing the tux costume (layer
``costume``); the gown variant is a rule, not a second drawing: ``variant(rows, "gown")`` remaps the costume keys and
swaps the bow tie for a pearl string and an ear flower (spec section 6.1). Shapes are shaded ellipses lit from the
upper left with the SWINE-01 light, then fixed facial features, an outer outline and an orphan clean-up. No raster
image-model output, no tracing or sampling of concept pixels, no resampling.

Only key 0 (``idle``) is authored at the design stage; the execution tier adds frames 1-15 from spec sections 4, 7
and 8 and must not change key 0 (frozen in ``tests/fixtures/tusk_01_keys.json``).

    python3 scripts/author_tusk01.py               # write grid + .ase
    python3 scripts/author_tusk01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_tusk01.py key0-scale    # docs/evidence/tusk-01/key0-scale.png (1x) + key0-scale-4x.png
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_swine01 as swine  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

OUT_DIR = ROOT / "assets/source/characters/swine/tusklings"
GRID = OUT_DIR / "tusk_01.grid.json"
ASE = OUT_DIR / "tusk_01.ase"
EVIDENCE = ROOT / "docs/evidence/tusk-01"
CANVAS = (56, 40)
PIVOT = (22, 39)
TICK_HZ = 60
LAYERS = ["body", "costume"]

# The shared family palette `tuskling-floor01-v1`, imported unchanged from SWINE-01 (19 keys; slot 20 stays free).
PALETTE = list(swine.PALETTE)
ACCENTS = {p[0] for p in PALETTE if p[3]}
DARKER = swine.DARKER
LIGHT = swine.LIGHT

TAGS = [("idle", 0, 2), ("scramble", 3, 6), ("tell", 7, 8), ("gore", 9, 11), ("hurt", 12, 12), ("down", 13, 15)]
TICKS = [18, 12, 14, 5, 5, 5, 5, 8, 20, 3, 6, 14, 12, 8, 10, None]   # None: state-driven hold (preview 100 ms)
KEY_FRAMES = (0, 3, 8, 10, 12, 15)
HOLD_MS = 100
GRID_GOWN = OUT_DIR / "tusk_01_gown.grid.json"
ASE_GOWN = OUT_DIR / "tusk_01_gown.ase"
MANIFESTS = ROOT / "assets/manifests"

OX, OY = 2, 4                    # design space -> canvas offset (design rows 6..34 + outline land on rows 10..39)

# Gown variant (spec section 6.1): costume-layer key remap plus accessory swap. Body layer unchanged.
GOWN_MAP = {"n": "r", "o": "g", "u": "m", "v": "l", "t": "s"}
BOW = {(24, 21), (28, 21), (24, 22), (25, 22), (26, 22), (27, 22), (28, 22), (24, 23), (28, 23)}
PEARLS = {(23, 21): "v", (24, 22): "v", (25, 23): "t", (26, 23): "v", (27, 23): "t", (28, 22): "v", (29, 21): "v"}
FLOWER = {(19, 8): "q", (20, 8): "q", (19, 9): "p", (20, 9): "q", (21, 9): "q", (20, 10): "p"}


class Painter:
    def __init__(self):
        self.px: dict[tuple[int, int], tuple[str, str]] = {}
        self.group = "trunk"
        self.log: list[tuple[str, int, int, str, str]] = []   # every put, in order, with its part group (replayed by compose)

    def put(self, x, y, key, layer="body"):
        self.px[(x, y)] = (key, layer)
        self.log.append((self.group, x, y, key, layer))

    def get(self, x, y):
        return self.px.get((x, y), (None, None))[0]


def lit(nx: float, ny: float) -> float:
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    return nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]


def flesh(d: float, dark: int = 0) -> str:
    k = "l" if d > 0.9 else "m" if d > 0.6 else "s" if d > 0.28 else "d"
    for _ in range(dark):
        k = DARKER[k]
    return k


def ell(p: Painter, cx, cy, rx, ry, dark=0, keyfn=None, layer="body", where=None) -> set:
    got = set()
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            nx, ny = (x - cx) / rx, (y - cy) / ry
            if nx * nx + ny * ny <= 1.0 and (where is None or where(x, y)):
                d = lit(nx, ny)
                p.put(x, y, keyfn(d) if keyfn else flesh(d, dark), layer)
                got.add((x, y))
    return got


def stamp(p: Painter, pixels: dict, layer="body"):
    for (x, y), k in pixels.items():
        p.put(x, y, k, layer)


def collar(x: int) -> float:
    return 15 + 0.35 * (x - 10)


def shirt_edge(y: int) -> float:
    return 26 - (y - 20) * 0.55


def key0_design() -> Painter:
    p = Painter()
    p.group = "coat"
    # 3 coat-tails hanging straight behind the rump (costume), split at the vent (gate fix 3: no flare)
    for y in range(24, 32):
        for x in range(8, 12):
            if not (y >= 29 and x == 10):
                p.put(x, y, "n", "costume")
    # 4 trunk: torso, shoulder hump, pot belly
    p.group = "trunk"
    trunk = set()
    trunk |= ell(p, 18, 23, 10, 9)
    trunk |= ell(p, 16, 17, 6.5, 5.5)
    trunk |= ell(p, 23, 26, 7.5, 6.5)
    # 5 open tux jacket: back and side below the collar line, ragged hem; shirt V in front; bare belly below
    for (x, y) in sorted(trunk):
        hem = 30 if (x * 7 + 3) % 5 == 0 else 31
        in_shirt = 20 <= y <= (27 if x % 3 else 28) and x >= shirt_edge(y)
        if in_shirt:
            nx, ny = (x - 23) / 7.5, (y - 26) / 6.5
            p.put(x, y, "v" if lit(nx, ny) > 0.8 else "u", "costume")
        elif collar(x) <= y <= hem and x < shirt_edge(min(y, 27)):
            rim = y - 1 < collar(x)
            p.put(x, y, "o" if rim and x < 20 else "n", "costume")   # 1 px collar and back sheen
    for (x, y) in sorted(trunk):     # lapel sheen: jacket pixels touching the shirt
        if p.px[(x, y)] == ("n", "costume") and p.get(x + 1, y) in ("u", "v") and p.px[(x + 1, y)][1] == "costume":
            p.put(x, y, "o", "costume")
    stamp(p, {(26, 25): "t", (27, 25): "t", (27, 26): "t"}, "costume")   # soiled shirt front
    p.group = "coat"
    # coat hem hangs behind the legs down to row 31/32 (gate fix 2); floor clearance below it
    for x in range(9, 18):
        for y in range(24, 31 if (x * 5) % 3 == 0 else 32):
            if (x, y) not in p.px or p.px[(x, y)][1] == "body":
                p.put(x, y, "n", "costume")
    p.group = "legs"
    # 1-2 legs in front of the coat, together under the belly (gate fix 1): two short pillars, a K gap at x 21
    for y in (31, 32):
        for x in range(17, 21):
            p.put(x, y, "s" if x == 18 else "d")
        for x in range(22, 27):
            p.put(x, y, "m" if x == 23 else "s" if x < 25 else "d")
    for y in (33, 34):
        for x in range(16, 21):
            p.put(x, y, "n")
        for x in range(22, 28):
            p.put(x, y, "n")
    p.put(21, 31, "c"), p.put(21, 32, "c")   # crease between the two shins
    p.put(18, 33, "K"), p.put(18, 34, "K")
    p.put(22, 33, "o"), p.put(23, 33, "o")
    p.put(24, 33, "K"), p.put(24, 34, "K")
    p.group = "head"
    # 6 head: domed skull, jowls, snout bridge, snout disc; a crevice where the skull meets the hump
    head = ell(p, 26, 14.5, 6.5, 6)
    ell(p, 27, 19, 5.5, 2.6, dark=1)
    ell(p, 31, 14.5, 3.2, 2.6)
    ell(p, 35.5, 14, 2.0, 2.7, keyfn=lambda d: "h" if d > 0.55 else "l" if d > 0.3 else "s")
    for (x, y) in sorted(head):
        if (x - 1, y) not in head and (x - 1, y) in trunk and 11 <= y <= 18:
            p.put(x, y, "c")
    # ears: far ear dark and low, near ear pricked and folded forward
    p.group = "ear_far"
    stamp(p, {(20, 10): "d", (21, 10): "d", (20, 9): "d", (21, 9): "d", (21, 8): "d", (22, 10): "d"})
    p.group = "ear_near"
    stamp(p, {(23, 10): "s", (24, 10): "s", (25, 10): "s", (24, 9): "l", (25, 9): "s", (26, 9): "s",
              (25, 8): "l", (26, 8): "l", (27, 8): "s", (27, 7): "s", (26, 10): "c", (25, 11): "d"})
    # 7 bow tie at the throat (costume)
    p.group = "trunk"
    for (x, y) in BOW:
        p.put(x, y, "r", "costume")
    p.put(26, 22, "g", "costume")
    p.group = "arm"
    # 8 near arm (gate fix 4-5): sleeve with a K seam on its back edge, n middle, o sheen on its front edge;
    #   a 3 px ivory cuff; the trotter-hand hangs past the belly outline, ringed in K where it overlaps the body
    seam = [(19, 21), (19, 22), (19, 23), (20, 24), (21, 25), (22, 26), (23, 27)]
    front = [(20, 21), (21, 22), (21, 23), (22, 24), (23, 25), (24, 26), (24, 27)]
    mid = [(20, 22), (20, 23), (21, 24), (22, 25), (23, 26)]
    stamp(p, {q: "K" for q in seam})
    stamp(p, {q: "n" for q in mid}, "costume")
    stamp(p, {q: "o" for q in front}, "costume")
    stamp(p, {(25, 26): "v", (25, 27): "v", (26, 27): "v"}, "costume")
    hand = {(27, 27): "m", (28, 27): "l", (29, 27): "m", (30, 27): "s",
            (27, 28): "m", (28, 28): "m", (29, 28): "m", (30, 28): "s", (31, 28): "s",
            (27, 29): "s", (28, 29): "s", (29, 29): "s", (30, 29): "d", (31, 29): "d",
            (28, 30): "n", (29, 30): "n", (30, 30): "n"}
    stamp(p, hand)
    cuff = {(25, 26), (25, 27), (26, 27)}
    for (x, y) in sorted(hand):
        for q in ((x - 1, y), (x, y + 1), (x + 1, y), (x, y - 1)):
            if q in p.px and q not in hand and q not in cuff:
                p.put(*q, "K")
    p.group = "head"
    # 9 face: scowl brow over a small sunk eye with a bag, far eye, cheek fold
    stamp(p, {(23, 11): "l", (24, 11): "l", (25, 12): "l", (26, 12): "l", (27, 12): "l",
              (23, 12): "c", (24, 12): "c", (25, 13): "c", (26, 13): "w", (27, 13): "K", (28, 13): "K",
              (25, 14): "d", (26, 14): "d", (27, 14): "d", (28, 14): "s",
              (30, 11): "l", (31, 11): "l", (30, 12): "K", (31, 12): "K",
              (22, 15): "s", (23, 16): "s", (23, 17): "s", (22, 16): "d"})
    # snarl: arched upper lip bares red gums and fangs, lower teeth, a lolling tongue; nostril notches
    stamp(p, {(29, 16): "s", (30, 16): "s", (31, 16): "s", (32, 16): "s", (33, 16): "s", (34, 16): "s",
              (30, 17): "r", (31, 17): "r", (32, 17): "r", (33, 17): "r",
              (27, 17): "c", (28, 17): "c", (29, 17): "c", (34, 17): "c",
              (27, 18): "c", (28, 18): "c", (31, 18): "c", (34, 18): "c",
              (29, 18): "v", (30, 18): "v", (32, 18): "v", (33, 18): "v"})
    p.group = "jaw"
    stamp(p, {(29, 19): "u", (30, 19): "u", (31, 19): "c", (32, 19): "c",
              (33, 19): "r", (33, 20): "r", (34, 20): "r", (34, 21): "d"})
    p.group = "head"
    stamp(p, {(37, 13): "K", (37, 15): "K"})
    # tusks: near tusk hooks up in front of the cheek, far tusk hooks up past the snout
    p.group = "tusk_near"
    stamp(p, {(26, 19): "t", (27, 19): "u", (28, 19): "u", (26, 18): "u", (26, 17): "v", (26, 16): "v",
              (27, 15): "v", (27, 16): "u", (28, 15): "v"})
    p.group = "tusk_far"
    stamp(p, {(35, 19): "t", (35, 18): "u", (36, 18): "u", (36, 17): "v", (37, 17): "u", (37, 16): "v",
              (38, 16): "u", (38, 15): "v", (39, 14): "v"})
    # 10 coarse bristle along the hump and crown
    p.group = "trunk"
    stamp(p, {(11, 13): "a", (12, 12): "a", (12, 13): "b", (13, 11): "b", (14, 11): "a", (15, 11): "a",
              (15, 12): "b", (16, 11): "b", (17, 11): "a", (18, 11): "a", (18, 10): "a", (19, 10): "a"})
    p.group = "head"
    stamp(p, {(22, 9): "a", (22, 8): "a", (23, 9): "a"})
    return p


def outline(p: Painter) -> None:
    filled = set(p.px)
    for (x, y) in sorted(filled):
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            if q not in filled and q not in p.px:
                p.put(*q, "K")


def to_layers(p: Painter) -> dict[str, list[str]]:
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    flat = [["."] * w for _ in range(h)]
    for (x, y), (k, layer) in p.px.items():
        X, Y = x + OX, y + OY
        if not (0 <= X < w and 0 <= Y < h):
            raise ValueError(f"pixel off canvas: {(x, y)} -> {(X, Y)}")
        flat[Y][X] = k
    flat = swine.despeckle(flat)       # same orphan rule as the Ball (accents g, w exempt)
    for (x, y), (_k, layer) in p.px.items():
        X, Y = x + OX, y + OY
        cels[layer][Y][X] = flat[Y][X]
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}


def key0() -> dict[str, list[str]]:
    p = key0_design()
    outline(p)
    return to_layers(p)


# --------------------------------------------------------------------------- frames 1-15 (spec sections 4, 7, 8)
# Frames are composed from key 0's part groups: ``compose`` replays every logged put of ``key0_design`` with a per-group
# offset (a tuple, or a function of canvas x, y returning offsets), replaces the groups a note redraws, runs the
# named hooks, then re-runs the outline and the orphan clean-up. All coordinates in hooks are canvas coordinates.
HEAD_FOLLOW = ("ear_far", "ear_near", "jaw", "tusk_near", "tusk_far")   # move with the head; their own entry is an extra offset


def cp(q: Painter, x: int, y: int, key: str, layer: str = "body") -> None:
    q.put(x - OX, y - OY, key, layer)


def cg(q: Painter, x: int, y: int):
    return q.get(x - OX, y - OY)


def _offs(spec, x, y) -> list:
    r = spec(x + OX, y + OY) if callable(spec) else spec
    return r if isinstance(r, list) else [r]


def compose(pose: dict) -> Painter:
    base = key0_design()
    q = Painter()
    hd = pose.get("head", (0, 0))
    last = {g: i for i, (g, *_r) in enumerate(base.log)}
    started: set = set()
    if "replace_all" in pose:
        pose["replace_all"](q)
        return q
    for i, (g, x, y, k, layer) in enumerate(base.log):
        q.group = g
        if g in pose.get("replace", {}):
            if g not in started:
                started.add(g)
                pose["replace"][g](q)
            continue
        if g not in started:
            started.add(g)
            if g in pose.get("pre", {}):
                pose["pre"][g](q)
        q.group = g
        drop = pose.get("drop", {}).get(g)
        if not (drop and drop(x + OX, y + OY, k)):
            if g in HEAD_FOLLOW:
                extra = pose.get(g, (0, 0))
                spec = (hd[0] + extra[0], hd[1] + extra[1])
            else:
                spec = pose.get(g, (0, 0))
            for dx, dy in _offs(spec, x, y):
                q.put(x + dx, y + dy, k, layer)
        if i == last[g] and g in pose.get("post", {}):
            q.group = g
            pose["post"][g](q)
    return q


def shear(y0, y1, a, b):
    """Offsets blending linearly from ``a`` at canvas row y0 to ``b`` at row y1 (floor and ceil, so no holes)."""
    def f(_x, y):
        t = min(1.0, max(0.0, (y - y0) / (y1 - y0)))
        fx, fy = a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t
        return sorted({(math.floor(fx), math.floor(fy)), (math.ceil(fx), math.ceil(fy))})
    return f


# ---- legs: parametric trotters and shins (the key-0 look: n hooves with a K cleft, shaded shins)
FAR_SHIN, NEAR_SHIN = "dsdd", "smsdd"


def foot(q, x0, y, w=6):
    for r in (0, 1):
        for i in range(w):
            cp(q, x0 + i, y + r, "n")
        cp(q, x0 + 2, y + r, "K")
    if w == 6:
        cp(q, x0, y, "o"), cp(q, x0 + 1, y, "o")


def shin(q, x_top, x_bot, y_top, ramp):
    for r, x0 in ((0, x_top), (1, x_bot)):
        for i, k in enumerate(ramp):
            cp(q, x0 + i, y_top + r, k)


def _chain(q, a, b, far):
    n = max(abs(b[0] - a[0]), abs(b[1] - a[1]), 1)
    for i in range(n + 1):
        x, y = round(a[0] + (b[0] - a[0]) * i / n), round(a[1] + (b[1] - a[1]) * i / n)
        cp(q, x, y, "s" if far else "m"), cp(q, x + 1, y, "d" if far else "s")


def limb(q, x0, y0, anchor, far=False):
    """Trotter in the air (frames 14, 15): a 4 x 2 hoof with a K cleft pointing up, a 2 px shin, a line to the trunk."""
    for i in range(4):
        cp(q, x0 + i, y0 + 1, "K" if i == 1 else "n")
        cp(q, x0 + i, y0 + 2, "n")
    _chain(q, (x0 + 1, y0 + 3), anchor, far)


def legs_fn(far=None, near=None, line=None):
    """far / near: (foot_x, foot_y, shin_top_x, shin_bottom_x, shin_top_y). Far is drawn first (behind the near shin)."""
    def draw(q):
        if far:
            fx, fy, tx, bx, ty = far
            shin(q, tx, bx, ty, FAR_SHIN)
            foot(q, fx, fy, 5)
        if line:
            _chain(q, *line, far=True)
        if near:
            fx, fy, tx, bx, ty = near
            shin(q, tx, bx, ty, NEAR_SHIN)
            foot(q, fx, fy, 6)
    return draw


# ---- head hooks (canvas coordinates; hd is the head offset)
def eye_open(hd):
    def f(q):
        cp(q, 27 + hd[0], 17 + hd[1], "w")        # the socket turns white: w w K K
    return f


def eye_shut(hd):
    def f(q):
        cp(q, 27 + hd[0], 17 + hd[1], "d"), cp(q, 28 + hd[0], 17 + hd[1], "d")
        cp(q, 29 + hd[0], 17 + hd[1], "c"), cp(q, 30 + hd[0], 17 + hd[1], "c")     # 2 px slit, w and K removed
        cp(q, 32 + hd[0], 16 + hd[1], "c"), cp(q, 33 + hd[0], 16 + hd[1], "c")      # far eye pair
    return f


def jaw_pre(hd, gape):
    def f(q):
        for r in range(gape):
            for x in range(31, 36):
                cp(q, x + hd[0], 23 + r + hd[1], "c")      # dark mouth interior behind the lowered jaw
    return f


def tongue_post(hd, gape, n):
    """The tongue ``n`` px longer: red down to row 24 + n (relative to the lowered jaw), then the dark tip."""
    def f(q):
        for j in range(n):
            cp(q, 36 + hd[0], 25 + gape + j + hd[1], "r")
        cp(q, 36 + hd[0], 25 + gape + n + hd[1], "d")
    return f


def far_tusk_level(hd):
    """Frames 8, 9: the far tusk levelled forward (root t (39,27), u row 26, v row 25, tip (44,24) at head (+1,+4))."""
    def f(q):
        dx, dy = hd[0] - 1, hd[1] - 4
        cp(q, 39 + dx, 27 + dy, "t")
        for x in (40, 41, 42):
            cp(q, x + dx, 26 + dy, "u")
        for x in (41, 42, 43):
            cp(q, x + dx, 25 + dy, "v")
        cp(q, 44 + dx, 24 + dy, "v")
    return f


def ear_flat(hd):
    """Frames 12, 13: the near ear folded back along the skull, its tip at the back of the head."""
    def f(q):
        data = {(23, 11): "l", (24, 11): "s", (25, 11): "s", (22, 12): "d", (23, 12): "s", (24, 12): "s",
                (21, 13): "d", (22, 13): "d", (23, 13): "s", (20, 14): "d", (21, 14): "d"}
        for (x, y), k in data.items():
            cp(q, x + hd[0], y + hd[1], k)
    return f


def hand_closed(x, y, k):
    return x >= 33 and 31 <= y <= 35          # the hand closes to 4 x 3: its front column and outline go


def post_all(*fs):
    def f(q):
        for g in fs:
            g(q)
    return f


# ---- lying poses (frames 14, 15): redrawn with the same palette and light, never rotated at runtime
def fill_ell(q, cx, cy, rx, ry, fn, ang=0.0):
    ca, sa = math.cos(ang), math.sin(ang)
    big = int(max(rx, ry)) + 2
    r = max(rx, ry)
    for y in range(int(cy) - big, int(cy) + big + 1):
        for x in range(int(cx) - big, int(cx) + big + 1):
            u, v = (x - cx) * ca + (y - cy) * sa, -(x - cx) * sa + (y - cy) * ca
            if (u / rx) ** 2 + (v / ry) ** 2 <= 1.0:
                d = lit(max(-1, min(1, (x - cx) / r)) * 0.95, max(-1, min(1, (y - cy) / r)) * 0.95)
                got = fn(x, y, d, u, v)
                if got:
                    cp(q, x, y, got[0], got[1])


def bow_at(q, ax, ay):
    for (x, y) in BOW:
        cp(q, x + OX + ax, y + OY + ay, "r", "costume")
    cp(q, 26 + OX + ax, 22 + OY + ay, "g", "costume")


def coat_floor(x0, x1, y0):
    def draw(q):
        for y in range(y0, 39):
            for x in range(x0 + (y - y0) // 2, x1 + 1):
                cp(q, x, y, "o" if y == y0 and x % 3 == 0 else "n", "costume")
    return draw


def head_lying(q):
    fill_ell(q, 39, 33, 6.5, 5.5, lambda x, y, d, u, v: (flesh(d), "body"))
    fill_ell(q, 40, 36, 5.0, 2.2, lambda x, y, d, u, v: (flesh(d, 1), "body"))        # jowl
    fill_ell(q, 44, 30, 2.0, 2.7, lambda x, y, d, u, v: ("h" if d > 0.55 else "l" if d > 0.3 else "s", "body"))
    for (x, y), k in {(45, 29): "K", (45, 31): "K", (37, 31): "l", (38, 31): "l", (39, 31): "l", (40, 31): "l",
                      (37, 32): "c", (38, 32): "c", (39, 32): "c", (40, 32): "c"}.items():
        cp(q, x, y, k)                                                                 # nostrils; brow over a shut eye
    for x, k in zip(range(41, 46), "svvvs"):
        cp(q, x, 34, "r" if k != "s" else "s")                                         # lip and gums
    for (x, y), k in {(42, 35): "v", (43, 35): "u", (45, 35): "v", (44, 35): "c", (46, 35): "c"}.items():
        cp(q, x, y, k)
    for j in range(35, 39):
        cp(q, 45, j, "r"), cp(q, 46, j, "r" if j < 38 else "d")                         # tongue lolls down to the floor
    for (x, y), k in {(41, 29): "t", (41, 28): "u", (41, 27): "u", (42, 26): "v", (42, 25): "v",
                      (47, 31): "t", (47, 30): "u", (48, 29): "u", (48, 28): "v", (48, 27): "v", (48, 26): "v"}.items():
        cp(q, x, y, k)                                                                 # both tusks point up
    for y in range(35, 39):                                                            # ears hang to the floor behind the skull
        cp(q, 32, y, "s"), cp(q, 33, y, "s"), cp(q, 31, y, "d")
    for (x, y), k in {(34, 30): "a", (35, 29): "a", (36, 29): "b"}.items():            # crown bristle
        cp(q, x, y, k)


def lying_all(q):
    q.group = "coat"
    coat_floor(12, 34, 37)(q)
    q.group = "trunk"

    def fn(x, y, d, u, v):
        if y >= 39:
            return None                                  # the coat and the outline own the contact row
        if y >= 35 or x <= 15:
            return ("o" if (y == 35 or x == 15) and d > 0.5 else "n", "costume")
        if v < 0.5 and u > -2:
            return ("v" if d > 0.8 else "u", "costume")
        return (flesh(d), "body")
    fill_ell(q, 24, 33, 12, 6, fn)
    bow_at(q, 5, 4)
    for yy in (37, 38):
        for x in range(12, 35):
            if cg(q, x, yy) is None:
                cp(q, x, yy, "n", "costume")
    q.group = "legs"
    limb(q, 20, 24, (22, 29))                           # near hand
    limb(q, 16, 26, (18, 31), far=True)                 # far hand
    limb(q, 10, 25, (13, 31))                           # near foot
    limb(q, 8, 28, (12, 33), far=True)                  # far foot
    q.group = "head"
    head_lying(q)


def _breath(x, y):
    if 32 <= y <= 34 and x == 31:
        return [(0, -1), (1, -1)]
    if 32 <= y <= 34 and x > 31:
        return [(1, -1)]
    return [(0, -1)]


def _extend_shins(q):
    for x in list(range(19, 23)) + list(range(24, 29)):
        k = cg(q, x, 35)
        if k:
            cp(q, x, 34, k)


def _hump_shear(x, y):          # frame 8: hump and bristle up 1 px, shoulders under the head down 1 px
    return (0, -1) if x <= 19 else (0, 1)


def _trunk_push(dx):            # frame 9: frame 8's shear moved right
    return lambda x, y: (dx, -1) if x <= 19 else (dx, 1)


def _coat_gore(x, y):           # frame 10: the coat lags the sheared body (+6 x, 1 px up; +4 left a detached block)
    return (6, -1)


def _trunk_gore(x, y):          # frame 10: body +8 x, sheared forward (rows 14-24 +2, rows 25-30 +1)
    return (10, 0) if 14 <= y <= 24 else (9, 0) if 25 <= y <= 30 else (8, 0)


def _trunk_hurt(x, y):          # frame 12: body -2 x, upper trunk leans back 1 px more
    return (-3, 0) if 14 <= y <= 24 else (-2, 0)


def _trunk_sag(x, y):           # frame 13: key 12 lowered 2 px (the body's lowest pixel is row 36; 3 or 4 px push the outline off the canvas)
    return (-3, 2) if 14 <= y <= 24 else (-2, 2)


def _tilt_trunk(q):
    def fn(x, y, d, u, v):
        if v > 0.5:
            return ("o" if v < 2.0 and d > 0.5 else "n", "costume")           # jacket on the back and underside
        if v < -0.5 and u > -5:
            return ("v" if d > 0.8 else "u", "costume")                       # shirt on the belly side
        return (flesh(d), "body")
    fill_ell(q, 20, 31, 9, 5.5, fn, -0.8)
    bow_at(q, -1, 2)


HD8, HD9, HD10, HD12, HD13 = (1, 4), (6, 4), (11, -3), (-4, -2), (-5, 4)
POSES: dict[int, dict] = {
    0: {},
    1: dict(head=(0, -1), trunk=_breath, arm=(0, -1), post={"legs": _extend_shins}),
    2: dict(head=(1, 1), post={"jaw": tongue_post((1, 1), 0, 1)}),
    3: dict(head=(2, 1), trunk=(2, 1), arm=(0, 1), coat=(2, -1),
            replace={"legs": legs_fn(far=(15, 37, 19, 17, 35), near=(28, 37, 25, 27, 35))}),
    4: dict(head=(2, 0), trunk=(2, 0), arm=(2, 0), coat=(2, 0),
            replace={"legs": legs_fn(far=(19, 36, 19, 19, 34), near=(24, 37, 24, 24, 35))}),
    5: dict(head=(2, 1), trunk=(2, 1), arm=(4, 1), coat=(2, -1),
            replace={"legs": legs_fn(far=(23, 37, 22, 23, 35), near=(20, 37, 23, 21, 35))}),
    6: dict(head=(2, 0), trunk=(2, 0), arm=(2, 0), coat=(2, 0),
            replace={"legs": legs_fn(far=(18, 37, 19, 19, 35), near=(23, 36, 23, 23, 34))}),
    7: dict(head=(1, 2), trunk=(0, 1), arm=(-2, 0)),
    8: dict(head=HD8, trunk=_hump_shear, arm=(-2, 0), coat=(0, -1), jaw=(0, 1), drop={"arm": hand_closed},
            replace={"tusk_far": far_tusk_level(HD8),
                     "legs": legs_fn(far=(13, 36, 18, 16, 34), near=(24, 37, 24, 24, 35))},
            pre={"jaw": jaw_pre(HD8, 1)}, post={"head": eye_open(HD8)}),
    9: dict(head=HD9, trunk=_trunk_push(5), arm=(3, 0), coat=(5, -1), jaw=(0, 1), drop={"arm": hand_closed},
            replace={"tusk_far": far_tusk_level(HD9),
                     "legs": legs_fn(far=(16, 37, 19, 17, 35), near=(29, 37, 29, 29, 35))},
            pre={"jaw": jaw_pre(HD9, 1)}, post={"head": eye_open(HD9)}),
    10: dict(head=HD10, trunk=_trunk_gore, arm=(8, -2), coat=_coat_gore, jaw=(0, 1),
             replace={"legs": legs_fn(near=(32, 37, 32, 32, 35), line=((28, 33), (21, 36)))},
             pre={"jaw": jaw_pre(HD10, 1)}, post={"head": eye_open(HD10), "jaw": tongue_post(HD10, 1, 1)}),
    11: dict(head=(3, 2), trunk=(3, 0), arm=(3, 0), coat=(3, 0), legs=(3, 0),
             post={"jaw": tongue_post((3, 2), 0, 2)}),
    12: dict(head=HD12, trunk=_trunk_hurt, arm=shear(25, 34, (-2, 0), (2, -4)), coat=(-2, 0),
             jaw=(0, 2), drop={"arm": hand_closed},
             replace={"ear_near": ear_flat(HD12),
                      "legs": legs_fn(far=(16, 37, 17, 17, 35), near=(22, 37, 22, 22, 35))},
             pre={"jaw": jaw_pre(HD12, 2)}, post={"head": eye_shut(HD12), "jaw": tongue_post(HD12, 2, 3)}),
    13: dict(head=HD13, trunk=_trunk_sag, arm=(-2, 3), coat=(-2, 2), jaw=(0, 1),
             replace={"ear_near": ear_flat(HD13),
                      "legs": legs_fn(far=(15, 37, 15, 15, 37), near=(23, 37, 23, 23, 37))},
             pre={"jaw": jaw_pre(HD13, 1)}, post={"head": eye_shut(HD13), "jaw": tongue_post(HD13, 1, 2)}),
    14: dict(head=(5, 6),
             replace={"coat": coat_floor(8, 18, 36), "trunk": _tilt_trunk, "arm": lambda q: None,
                      "legs": lambda q: (limb(q, 29, 33, (24, 31)), limb(q, 26, 35, (22, 32), far=True))},
             post={"head": eye_shut((5, 6))}),
    15: dict(replace_all=lying_all),
}
# ANCHORS[frame] = (throat_dx, throat_dy, ear_dx, ear_dy): offsets of the bow tie / pearl string and of the far ear
# from key 0, applied by ``variant`` to the gown accessories (spec section 6.1). Other frames use their pose offsets.
ANCHORS = {14: (-1, 2, 5, 6), 15: (5, 4, 9, 23)}


def anchors(frame: int) -> tuple:
    if frame in ANCHORS:
        return ANCHORS[frame]
    pose = POSES.get(frame, {})
    hd, tr = pose.get("head", (0, 0)), pose.get("trunk", (0, 0))
    t = _offs(tr, 28 - OX, 26 - OY)[0] if callable(tr) else tr
    return (t[0], t[1], hd[0], hd[1])


def render(frame: int, _cache: dict = {}) -> dict[str, list[str]]:   # noqa: B006 - deterministic memo
    if frame not in _cache:
        if frame == 0:
            _cache[frame] = key0()
        else:
            q = compose(POSES[frame])
            outline(q)
            _cache[frame] = to_layers(q)
    return _cache[frame]


def variant(cels: dict[str, list[str]], name: str, frame: int = 0) -> dict[str, list[str]]:
    """Costume variant by rule: ``tux`` is the drawn costume; ``gown`` remaps the costume keys (black cloth to red
    sequin, sheen to glint, shirt to bare flesh) and swaps the bow tie for a pearl string and an ear flower."""
    if name == "tux":
        return {k: list(v) for k, v in cels.items()}
    assert name == "gown", name
    body = [list(r) for r in cels["body"]]
    cos = [list(r) for r in cels["costume"]]
    for y, row in enumerate(cos):
        for x, k in enumerate(row):
            if k != ".":
                cos[y][x] = GOWN_MAP.get(k, k)
                if k == "n" and (x + 2 * y) % 11 == 0 and x < 20:
                    cos[y][x] = "g"           # scattered sequin glints
    tx, ty, ex, ey = anchors(frame)       # the accessories follow the throat and the far ear of this frame

    def inside(X, Y):
        return 0 <= Y < len(cos) and 0 <= X < len(cos[0])
    for (x, y) in BOW:
        if inside(x + OX + tx, y + OY + ty) and cos[y + OY + ty][x + OX + tx] != ".":
            cos[y + OY + ty][x + OX + tx] = "l"
    for (x, y), k in PEARLS.items():
        X, Y = x + OX + tx, y + OY + ty
        if inside(X, Y):
            body[Y][X] = "."
            cos[Y][X] = k
    for (x, y), k in FLOWER.items():
        X, Y = x + OX + ex, y + OY + ey
        if inside(X, Y):
            body[Y][X] = "."
            cos[Y][X] = k
    flat = swine.despeckle([[c if c != "." else b for b, c in zip(br, cr)] for br, cr in zip(body, cos)])
    for y, row in enumerate(flat):      # same orphan clean-up as key 0, written back to the owning layer
        for x, k in enumerate(row):
            if cos[y][x] != ".":
                cos[y][x] = k
            elif body[y][x] != ".":
                body[y][x] = k
    return {"body": ["".join(r) for r in body], "costume": ["".join(r) for r in cos]}


def flatten(cels: dict[str, list[str]]) -> list[str]:
    out = [list(r) for r in cels["body"]]
    for y, row in enumerate(cels["costume"]):
        for x, k in enumerate(row):
            if k != ".":
                out[y][x] = k
    return ["".join(r) for r in out]


# --------------------------------------------------------------------------- grid output
def build_grid(name: str = "tux") -> dict:
    frames = [{"duration_ms": round((t if t is not None else HOLD_MS * TICK_HZ / 1000) * 1000 / TICK_HZ),
               "cels": variant(render(i), name, i)} for i, t in enumerate(TICKS)]
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": swine.palette_json(), "layers": list(LAYERS), "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def report(rows: list[str], label: str) -> None:
    orphans, isolated = pg._stray_pixels([list(r) for r in rows], ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    text = "".join(rows)
    counts = " ".join(f"{k}{text.count(k)}" for k in "Kwhvutnrgq")
    print(f"{label}: x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} {counts}")


# --------------------------------------------------------------------------- evidence
def _image(rows: list[str]):
    from PIL import Image
    pal = {k: tuple(int(rgba[i:i + 2], 16) for i in (1, 3, 5, 7)) for k, _n, rgba, _a in PALETTE}
    im = Image.new("RGBA", CANVAS, (0, 0, 0, 0))
    for y, r in enumerate(rows):
        for x, k in enumerate(r):
            if k != ".":
                im.putpixel((x, y), pal[k])
    return im


def write_key0_scale(out: Path = EVIDENCE / "key0-scale.png") -> list[Path]:
    """Silhouette-gate evidence: key 0 (tux and gown) at 1x on lit Floor 1 next to Carl and the Ball of Swine
    idle key (640 x 360, Tusklings mirrored to face Carl, no scaling), plus the same scene at 4x."""
    from PIL import Image
    from pixelart.common import CARL_X, flip, grid_images, load_grid_file  # noqa: E402
    from pixelart.scene import Floor, reference_idle  # noqa: E402

    carl, carl_piv = reference_idle("carl")
    ball, b_piv = flip(grid_images(load_grid_file(swine.GRID))[0], tuple(swine.PIVOT))
    k0 = load_grid_file(GRID)["frames"][0]["cels"]
    tux, t_piv = flip(_image(flatten(variant(k0, "tux"))), PIVOT)
    gown, g_piv = flip(_image(flatten(variant(k0, "gown"))), PIVOT)
    entries = [(carl, carl_piv, CARL_X, 0), (tux, t_piv, CARL_X + 44, 0), (gown, g_piv, CARL_X + 76, 0),
               (ball, b_piv, CARL_X + 190, 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    scene = Floor().render(entries, 1)
    scene.save(out)
    four = out.with_name("key0-scale-4x.png")
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    return [out, four]


def manifest(variant_name: str) -> dict:
    gown = variant_name == "gown"
    stem = "tusk_01_gown" if gown else "tusk_01"
    tags = {}
    for n, a, b in TAGS:
        tags[n] = (a, b)
    tag_of = {f: n for n, (a, b) in tags.items() for f in range(a, b + 1)}
    frames = []
    for i, t in enumerate(TICKS):
        ms = round((t if t is not None else HOLD_MS * TICK_HZ / 1000) * 1000 / TICK_HZ)
        frames.append({"index": i, "tag": tag_of[i], "ticks": t, "duration_ms": ms,
                       "author": "key" if i in KEY_FRAMES else "inbetween"})
    costume = "gown (costume layer remapped by variant(): red sequin gown, pearl string, ear flower)" if gown else \
        "tux (black jacket, ivory shirt, red bow tie)"
    return {
        "schema_version": 1,
        "id": f"{stem}_floor01_v1_candidate",
        "type": "character_animation",
        "source_path": f"assets/source/characters/swine/tusklings/{stem}.ase",
        "grid_source": f"assets/source/characters/swine/tusklings/{stem}.grid.json",
        "output_atlas": f"generated/art/{stem}.png",
        "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "tuskling-floor01-v1",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Floor 1 Tuskling, minion of the Ball of "
                 "Swine; runtime role IP-neutral small group melee rusher), costume " + costume,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "TUSK-01 Tuskling readability set: frame 0 (idle) is the frozen hero key built by key0_design() in "
                      "scripts/author_tusk01.py; frames 1 to 15 (5 further keys and 10 in-betweens) are composed from "
                      "key 0's part groups (offset, replaced or redrawn parts, outline, orphan clean-up); the gown "
                      "atlas is the same grid remapped by variant()",
            "generator": "Design, spec and hero key: Claude Opus. Other keys, in-betweens, atlases, validators and "
                         "evidence: Claude Sonnet, composed by scripts/author_tusk01.py. Review: pending, a separate "
                         "top-tier agent. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-ball-of-swine-arena-concept"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "tuskling": {
            "status": "execution done, independent top-tier review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/tusk-01-spec.md",
            "variant": variant_name,
            "opaque_height_idle": 30,
            "tick_hz": TICK_HZ,
            "hold_preview_ms": HOLD_MS,
            "grounded_tags": [n for n, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": {"idle": "loop", "scramble": "loop", "tell": "once", "gore": "once", "hurt": "once",
                         "down": "hold"},
            "key_frames": list(KEY_FRAMES),
            "inbetween_frames": [i for i in range(len(TICKS)) if i not in KEY_FRAMES],
            "key_fixture": "tests/fixtures/tusk_01_keys.json",
            "boxes": {"collision": [-11, 0, 23, 24], "hurtbox": [-13, 0, 29, 27], "gore_hitbox": [22, 12, 9, 12],
                      "note": "pivot-relative, x right, y up, [x, y, w, h]; placeholders, not approved"},
            "attack_ticks": {"tell": [0, 27], "gore_launch": [28, 30], "hitbox_active": [31, 36], "recover": [37, 50]},
            "frames": frames,
        },
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", nargs="?", choices=["key0-scale"])
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.command == "key0-scale":
        for p in write_key0_scale():
            print(f"wrote {p.relative_to(ROOT)}")
        return 0
    grid = build_grid()
    gown = build_grid("gown")
    for f in range(len(TICKS)):
        report(flatten(grid["frames"][f]["cels"]), f"frame {f:2d} tux ")
        report(flatten(gown["frames"][f]["cels"]), f"frame {f:2d} gown")
    jobs = [(GRID, ASE, grid), (GRID_GOWN, ASE_GOWN, gown)]
    if a.check:
        ok = all(g.is_file() and g.read_text() == swine.dump(d) for g, _a, d in jobs) and all(
            (MANIFESTS / f"{n}.json").read_text() == json.dumps(manifest(v), indent=2) + "\n"
            for n, v in (("tusk-01", "tux"), ("tusk-01-gown", "gown")))
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    for g, ase, d in jobs:
        g.parent.mkdir(parents=True, exist_ok=True)
        g.write_text(swine.dump(d), encoding="utf-8")
        pg.write_ase(d, ase)
        print(f"wrote {g.relative_to(ROOT)} and {ase.relative_to(ROOT)}")
    for n, v in (("tusk-01", "tux"), ("tusk-01-gown", "gown")):
        (MANIFESTS / f"{n}.json").write_text(json.dumps(manifest(v), indent=2) + "\n", encoding="utf-8")
        print(f"wrote assets/manifests/{n}.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
