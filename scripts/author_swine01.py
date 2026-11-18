#!/usr/bin/env python3
"""Author SWINE-01 Ball of Swine (Floor 01 arena boss) keys as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/swine-01-spec.md``. The Ball is a feature model on a disc: Tuskling heads, costume
masses, trotters and bristle tufts placed by polar position (rho, alpha) and a facing angle, in depth order. A pose
moves the disc (centre, radii, shear, a flat front cut) and turns every alpha by ``theta``; each pose is
re-rasterized at native resolution and shaded with one fixed screen-space light (torch upper left), so the light
never turns with the ball (spec section 3.1). No raster image-model output, no tracing or sampling of concept
pixels, no resampling.

Keys 0, 3, 10, 12, 14 and 17 are authored here, and the roll frames 4-8 follow from the frame-3 rule (theta + 60
per frame). Frames 1, 2, 9, 11, 13, 15 and 16 are the in-betweens (INBETWEENS, spec section 8, drawn through the same model).

    python3 scripts/author_swine01.py               # write grid + .ase
    python3 scripts/author_swine01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_swine01.py key0-scale    # docs/evidence/swine-01/key0-scale.png (1x) + key0-scale-4x.png
    python3 scripts/author_swine01.py roll-preview  # docs/evidence/swine-01/roll-preview.gif (1x, six roll frames)
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import pixel_grid_to_ase as pg  # noqa: E402

SWINE_DIR = ROOT / "assets/source/characters/swine"
GRID = SWINE_DIR / "swine-01.grid.json"
ASE = SWINE_DIR / "swine-01.ase"
EVIDENCE = ROOT / "docs/evidence/swine-01"
CANVAS = (176, 144)
PIVOT = (88, 143)
TICK_HZ = 60

# key, name, rgba, accent (accents are exempt from the orphan rule). Shared Tuskling family palette
# `tuskling-floor01-v1`: flesh, tusk, hair, cloth, sequin and flower ramps are the same for the later Tusklings.
PALETTE = [
    ("K", "outline", "#1a0b12ff", False),
    ("c", "crevice", "#3d1420ff", False),
    ("d", "flesh_deep", "#6e2630ff", False),
    ("s", "flesh_shade", "#a8474aff", False),
    ("m", "flesh_mid", "#d36e66ff", False),
    ("l", "flesh_light", "#ef9c86ff", False),
    ("h", "flesh_high", "#f8c4a6ff", False),
    ("t", "tusk_shade", "#9a7a52ff", False),
    ("u", "tusk_mid", "#d8c094ff", False),
    ("v", "tusk_light", "#f6ead0ff", False),
    ("a", "hair_dark", "#5a2e1eff", False),
    ("b", "hair_tan", "#a86a38ff", False),
    ("n", "cloth_black", "#231c2aff", False),
    ("o", "cloth_sheen", "#4e4660ff", False),
    ("r", "sequin_red", "#a8142eff", False),
    ("g", "sequin_glint", "#f25a5eff", True),
    ("p", "flower_deep", "#4e1f6eff", False),
    ("q", "flower_mid", "#9b55c4ff", False),
    ("w", "eye_white", "#f0e2ccff", True),
]
ACCENTS = {p[0] for p in PALETTE if p[3]}
DARKER = {"h": "l", "l": "m", "m": "s", "s": "d", "d": "c", "c": "c", "v": "u", "u": "t", "t": "a",
          "b": "a", "a": "c", "o": "n", "n": "n", "r": "d", "g": "r", "q": "p", "p": "n", "K": "K", "w": "w"}
_l = (-0.5, -0.62, 0.6)
LIGHT = tuple(c / math.sqrt(sum(v * v for v in _l)) for c in _l)   # torch key upper left, screen space

TAGS = [("idle", 0, 2), ("roll", 3, 8), ("attack_tell", 9, 10), ("attack_charge", 11, 13), ("hurt", 14, 14),
        ("down", 15, 17)]
TICKS = [16, 12, 16, 4, 4, 4, 4, 4, 4, 8, 28, 3, 6, 24, 12, 8, 12, None]
KEY_FRAMES = (0, 3, 10, 12, 14, 17)
ROLL_FRAMES = (3, 4, 5, 6, 7, 8)

# --------------------------------------------------------------------------- poses
POSE0 = dict(theta=0.0, cx=88.0, cy=83.0, rx=64.0, ry=62.0, shear=0.0, front_cut=None, tuft_add=0, tuft_sweep=0.0,
             eyes="open", gape_add=0, head_k=None, head_sb=1.0, limp=False, cracks=0.0)
KEY_POSES = {
    0: {},
    3: dict(shear=2.0, tuft_sweep=-2.0),                                                   # roll, theta 0
    10: dict(theta=-20.0, cx=82.0, cy=89.0, rx=68.0, ry=57.0, tuft_add=3, eyes="wide", gape_add=1),   # coil
    12: dict(cx=113.0, cy=85.0, rx=66.0, ry=60.0, front_cut=0.94, eyes="wide", gape_add=2, tuft_sweep=-3.0),
    14: dict(theta=-6.0, cx=84.0, cy=84.0, rx=65.0, ry=61.0, eyes="shut", gape_add=2),      # recoil
    17: dict(theta=60.0, cx=88.0, cy=117.0, rx=78.0, ry=27.0, head_k=1.0, head_sb=0.62, eyes="shut", limp=True,
             tuft_add=-1, gape_add=1),                                                     # heap
}


# In-betweens: pose values from spec section 8, rasterized through the same model (no pixel edits).
INBETWEENS = {
    1: dict(cy=84.0, ry=61.0, rx=65.0),
    2: dict(theta=2.0),
    9: dict(theta=-10.0, cx=85.0, cy=86.0, rx=66.0, ry=60.0, tuft_add=1, eyes="wide"),
    11: dict(theta=20.0, cx=92.0, cy=87.0, rx=74.0, ry=56.0, shear=4.0, tuft_sweep=-5.0, eyes="wide", gape_add=2),
    13: dict(theta=10.0, cx=92.0, cy=83.0, rx=63.0, ry=62.0, eyes="open", gape_add=2),
    15: dict(theta=20.0, cx=88.0, cy=95.0, rx=70.0, ry=50.0, head_sb=0.85, eyes="shut", gape_add=1, cracks=2.0),
    16: dict(theta=40.0, cx=88.0, cy=105.0, rx=75.0, ry=39.0, head_k=1.0, head_sb=0.72, eyes="shut", limp=True,
             gape_add=1, cracks=3.0),
}


def pose_for(frame: int) -> dict:
    if frame in ROLL_FRAMES:
        return dict(POSE0, **KEY_POSES[3], theta=60.0 * (frame - 3))
    if frame in INBETWEENS:
        return dict(POSE0, **INBETWEENS[frame])
    return dict(POSE0, **KEY_POSES[frame])


# --------------------------------------------------------------------------- feature model (theta = 0)
# Parts in depth order (back to front). Heads: ("head", rho, alpha, r, up, sa, sb, dress, tusks, mouth).
#   up: screen angle of the forehead (-90 = upright); tusks: (left px, right px, chipped sides); mouth: gape width px.
# Rim heads sit at irregular spacing (27, 46, 31, 49, 35, 44, 26, 41, 29, 32 degrees): no period near 60/n, so a
# 60 degree roll step never aliases into a backward turn.
PARTS = (
    ("head", 0.90, -175, 13, 180, 1.0, 0.85, None, (5, 7, "R"), 4),     # sideways
    ("head", 0.79, -148, 14, -132, 1.0, 0.9, "hat", (4, 6, ""), 5),
    ("head", 0.93, -102, 11, -100, 1.0, 0.8, None, (4, 5, "L"), 4),
    ("head", 0.88, -71, 15, -52, 0.95, 0.9, None, (6, 8, ""), 5),
    ("head", 0.93, -22, 11, -6, 1.0, 0.85, None, (5, 4, ""), 4),          # sideways
    ("head", 0.88, 13, 15, 24, 1.0, 0.85, None, (7, 9, "L"), 6),
    ("head", 0.92, 57, 12, 64, 1.0, 0.8, None, (5, 6, ""), 4),           # upside down
    ("head", 0.90, 83, 14, 98, 1.05, 0.8, None, (6, 4, "R"), 5),         # upside down, on the floor
    ("head", 0.93, 124, 11, 132, 1.0, 0.85, None, (4, 5, ""), 4),        # upside down
    ("head", 0.88, 153, 16, 168, 0.95, 0.9, None, (7, 6, ""), 6),        # sideways
    ("cloth", 0.72, 126, 15, 11),                                         # tux jacket, collar and bow tie
    ("sequin", 0.76, 30, 14, 10),                                         # red sequin dress scrap
    ("head", 0.12, 172, 24, -122, 0.95, 0.86, None, (8, 7, "R"), 6),     # centre face, buried by its neighbours
    ("head", 0.52, 176, 18, 200, 1.0, 0.85, None, (6, 7, ""), 5),        # back, sideways
    ("head", 0.56, 92, 17, 104, 1.0, 0.85, None, (5, 7, "L"), 5),        # lower, upside down
    ("head", 0.54, -150, 16, -165, 0.95, 0.9, None, (6, 5, ""), 5),
    ("head", 0.55, -96, 18, -82, 1.05, 0.82, None, (7, 8, ""), 6),       # top, squealing upward
    ("head", 0.52, -48, 17, -66, 1.0, 0.9, "flower", (5, 6, "R"), 5),
    ("head", 0.50, 8, 20, -26, 1.05, 0.9, "pearls", (8, 9, ""), 6),      # front: the aggressor at the player
)
TROTTERS = (35, 140, -125, -40)       # alpha deg: legs kicking out past the rim between rim heads
TUFTS = (-176, -164, -134, -112, -88, -78, -62, -30, -12, 2, 34, 44, 70, 108, 138, 164, 194)
TROTTER_OWNER, HAT_OWNER = 90, 95
ENVELOPE = 2.0     # px a head dome may stand proud of the disc ellipse (ears, tufts, hat and trotters are free)


def rad(deg: float) -> float:
    return math.radians(deg)


def bezier(p0, p1, p2, n=24):
    return [((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
             (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]) for t in (i / (n - 1) for i in range(n))]


def flesh_key(v: float) -> str:
    return "d" if v < 0.42 else "s" if v < 0.6 else "m" if v < 0.8 else "l"


def lum3(n) -> float:
    d = n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2]
    return 0.3 + 0.7 * max(0.0, d)


def blend(a, b, wa: float):
    n = tuple(wa * a[i] + (1 - wa) * b[i] for i in range(3))
    s = math.sqrt(sum(c * c for c in n)) or 1.0
    return tuple(c / s for c in n)


class Head:
    """A Tuskling head. Local face coordinates: a along the face's right, b from forehead to chin (units of r);
    ``px`` converts pixel offsets along the face axes to world pixels."""

    def __init__(self, c, r, up, sa, sb, dress):
        self.c, self.r, self.sa, self.sb, self.dress = c, r, sa, sb, dress
        g = rad(up)
        self.U = (math.cos(g), math.sin(g))
        self.D = (-self.U[0], -self.U[1])
        self.R = (-self.U[1], self.U[0])

    def local(self, x, y):
        dx, dy = x + 0.5 - self.c[0], y + 0.5 - self.c[1]
        return ((dx * self.R[0] + dy * self.R[1]) / (self.r * self.sa),
                (dx * self.D[0] + dy * self.D[1]) / (self.r * self.sb))

    def world(self, a, b):
        a, b = a * self.sa * self.r, b * self.sb * self.r
        return self.c[0] + a * self.R[0] + b * self.D[0], self.c[1] + a * self.R[1] + b * self.D[1]

    def px(self, origin, da, db):
        return origin[0] + da * self.R[0] + db * self.D[0], origin[1] + da * self.R[1] + db * self.D[1]

    def pixels(self, test, pad=1.6):
        rr = int(self.r * pad * max(self.sa, self.sb)) + 2
        cx, cy = int(self.c[0]), int(self.c[1])
        for y in range(cy - rr, cy + rr + 1):
            for x in range(cx - rr, cx + rr + 1):
                a, b = self.local(x, y)
                if test(a, b):
                    yield (x, y), (a, b)


def ell(a0, b0, ra, rb):
    return lambda a, b: ((a - a0) / ra) ** 2 + ((b - b0) / rb) ** 2 <= 1.0


def tri(p0, p1, p2):
    def inside(a, b):
        def s(p, q):
            return (a - q[0]) * (p[1] - q[1]) - (p[0] - q[0]) * (b - q[1])
        d1, d2, d3 = s(p0, p1), s(p1, p2), s(p2, p0)
        return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))
    return inside


def ip(p):
    return int(math.floor(p[0])), int(math.floor(p[1]))


# --------------------------------------------------------------------------- renderer
class Ball:
    def __init__(self, pose: dict):
        self.P = pose
        self.theta = pose["theta"]
        self.key: dict = {}
        self.owner: dict = {}      # pixel -> depth index (-1 body, part index, 90 trotters, 95 hat)
        self.w, self.h = CANVAS
        k = pose["head_k"]
        self.hk = k if k is not None else math.sqrt(pose["rx"] * pose["ry"] / (64.0 * 62.0))

    # geometry of the posed disc
    def to_xy(self, rho, alpha):
        P = self.P
        u, v = rho * math.cos(rad(alpha)), rho * math.sin(rad(alpha))
        return P["cx"] + u * P["rx"] - P["shear"] * v, P["cy"] + v * P["ry"]

    def uv(self, x, y):
        P = self.P
        v = (y - P["cy"]) / P["ry"]
        return (x + P["shear"] * v - P["cx"]) / P["rx"], v

    def sphere_n(self, x, y):
        u, v = self.uv(x + 0.5, y + 0.5)
        r2 = min(0.999, u * u + v * v)
        return (u, v, math.sqrt(1.0 - r2))

    def inside(self, x, y) -> bool:
        cut = self.P["front_cut"]
        if cut is not None and x + 0.5 > self.P["cx"] + cut * self.P["rx"]:
            return False
        return 0 <= x < self.w and 0 <= y <= PIVOT[1]

    def outside_envelope(self, p, extra: float = 0.0) -> bool:
        P = self.P
        dx, dy = p[0] + 0.5 - P["cx"], p[1] + 0.5 - P["cy"]
        return (dx / (P["rx"] + ENVELOPE + extra)) ** 2 + (dy / (P["ry"] + ENVELOPE + extra)) ** 2 > 1.0

    def set(self, p, k, owner=None):
        if self.inside(*p):
            self.key[p] = k
            if owner is not None:
                self.owner[p] = owner

    # body: the recessed mass between the heads (bristle and flesh, one step darker than an exposed surface)
    def body(self):
        for y in range(self.h):
            for x in range(self.w):
                u, v = self.uv(x + 0.5, y + 0.5)
                if u * u + v * v > 1.0 and (self.P["limp"] or self.outside_envelope((x, y))):
                    continue      # the body fills the envelope ellipse: the crown of the disc is fixed in every pose
                lv = lum3(self.sphere_n(x, y)) * 0.66
                stripe = (x * 2 + y) % 7 < 2
                k = ("b" if lv > 0.5 and stripe else "a") if (x + y // 3) % 9 < 4 else flesh_key(lv * 0.9)
                self.set((x, y), k, -1)

    def tufts(self):
        P = self.P
        for i, a in enumerate(TUFTS):
            a2 = a + self.theta
            ln = max(2, 4 + (i * 5) % 4 + P["tuft_add"])
            base = self.to_xy(0.97, a2)
            tip = self.to_xy(1.0 + ln / 62.0, a2 + (4 if i % 2 else -4))
            tip = (tip[0] + P["tuft_sweep"], tip[1])
            side = (-math.sin(rad(a2)), math.cos(rad(a2)))
            for s in range(0, 7):
                t = s / 6
                x = base[0] + (tip[0] - base[0]) * t
                y = base[1] + (tip[1] - base[1]) * t
                half = 2.4 * (1 - t) + 0.3
                for q in (-1, 0, 1):
                    if abs(q) <= half:
                        p = (int(x + side[0] * q * 0.8), int(y + side[1] * q * 0.8))
                        if p not in self.key:
                            self.set(p, "b" if (a2 % 360) > 180 and t > 0.4 else "a", -1)

    def trotters(self):
        """Pig legs kicking out 6-8 px past the rim: d/s shank, n hoof with a K cleft."""
        for i, a in enumerate(TROTTERS):
            a2 = a + self.theta
            ux, uy = math.cos(rad(a2)), math.sin(rad(a2))
            base = self.to_xy(0.9, a2)
            ln = 13 + i % 2 * 2
            for s in range(ln * 2 + 1):
                d = s / 2
                cx, cy = base[0] + ux * d, base[1] + uy * d
                rr = 2.7 if d < ln - 3 else 2.9
                for yy in range(int(cy - 4), int(cy + 5)):
                    for xx in range(int(cx - 4), int(cx + 5)):
                        ox, oy = xx + 0.5 - cx, yy + 0.5 - cy
                        if ox * ox + oy * oy <= rr * rr:
                            side = ox * LIGHT[0] + oy * LIGHT[1]
                            k = "n" if d >= ln - 3 else ("m" if side > 0.8 else "s" if side > -0.6 else "d")
                            self.set((xx, yy), k, TROTTER_OWNER)
            for t in range(3):     # the cleft of the hoof
                q = (base[0] + ux * (ln - t + 1.2), base[1] + uy * (ln - t + 1.2))
                self.set(ip(q), "K", TROTTER_OWNER)

    def patch(self, idx, kind, rho, alpha, ra, rb):
        """A costume mass between the heads: an ellipse oriented along its polar direction."""
        c = self.to_xy(rho, alpha + self.theta)
        g = rad(alpha + self.theta)
        A, B = (math.cos(g), math.sin(g)), (-math.sin(g), math.cos(g))
        ra, rb = ra * self.hk, rb * self.hk
        mine = set()
        for y in range(int(c[1] - ra - 2), int(c[1] + ra + 3)):
            for x in range(int(c[0] - ra - 2), int(c[0] + ra + 3)):
                dx, dy = x + 0.5 - c[0], y + 0.5 - c[1]
                pa, pb = dx * A[0] + dy * A[1], dx * B[0] + dy * B[1]
                wob = 1.0 + 0.12 * math.sin(3 * math.atan2(pb, pa) + idx)
                if (pa / ra) ** 2 + (pb / rb) ** 2 <= wob:
                    mine.add((x, y))
        for (x, y) in mine:
            lit = (x - 1, y) not in mine or (x, y - 1) not in mine
            lv = lum3(self.sphere_n(x, y))
            if kind == "cloth":
                k = "o" if lit and lv > 0.45 else "n"
            else:
                k = "g" if (lv > 0.42 and (x * 3 + y * 5) % 7 == 0) else ("d" if lv < 0.31 and (x + 2 * y) % 5 == 0 else "r")
            self.set((x, y), k, idx)
        if kind == "cloth":
            self.tux(c, A, B, ra, rb, mine, idx)

    def tux(self, c, A, B, ra, rb, mine, idx):
        """Shirt front (a V of `u`/`v`, widest toward the rim), notched lapels (`o` sheen on the lit side, `K`
        notch) and a `n` bow tie with a `K` knot at the V apex."""
        pa0, pa1 = -0.45 * ra, 0.62 * ra
        inside, edge = {}, {}
        for (x, y) in mine:
            dx, dy = x + 0.5 - c[0], y + 0.5 - c[1]
            pa, pb = dx * A[0] + dy * A[1], dx * B[0] + dy * B[1]
            hw = 2.4 + 0.2 * (pa - pa0)
            if pa0 - 1.5 <= pa <= pa1 and abs(pb) <= hw:
                lv = lum3(self.sphere_n(x, y))
                inside[(x, y)] = "t" if lv < 0.34 else ("v" if lv > 0.5 else "u")
            elif pa0 - 1.5 <= pa <= pa1 + 1.0 and abs(pb) <= hw + 1.6:
                edge[(x, y)] = (pa, pb)
        for q, k in inside.items():
            self.set(q, k, idx)
        for q, (pa, pb) in edge.items():
            lv = lum3(self.sphere_n(*q))
            notch = abs(pa - (pa0 + 0.55 * (pa1 - pa0))) < 1.0
            self.set(q, "K" if notch and abs(pb) > 2.4 else ("o" if lv > 0.42 else "n"), idx)
        for (x, y) in mine:
            dx, dy = x + 0.5 - c[0], y + 0.5 - c[1]
            pa, pb = dx * A[0] + dy * A[1], dx * B[0] + dy * B[1]
            if abs(pa - pa0) <= 1.6 and abs(pb) <= 3.4:
                self.set((x, y), "K" if abs(pa - pa0) <= 0.7 and abs(pb) <= 0.9 else "n", idx)

    def head(self, idx, part):
        _, rho, alpha, r, up, sa, sb, dress, tusks, mouth = part
        P = self.P
        c = self.to_xy(rho, alpha + self.theta)
        if P["limp"]:
            up_eff = (0 if idx % 2 else 180) + (idx * 23) % 40 - 20
        else:
            up_eff = up + self.theta
        H = Head(c, r * 1.04 * self.hk, up_eff, sa, sb * P["head_sb"], dress)
        dome = ell(0, 0, 1.0, 1.0)
        ears = (tri((-0.8, -0.38), (-0.25, -0.86), (-1.05, -1.12)), tri((0.8, -0.38), (0.25, -0.86), (1.05, -1.12)))
        mine = {}
        lumps = set()
        for p, (a, b) in H.pixels(lambda a, b: dome(a, b) or ears[0](a, b) or ears[1](a, b)):
            rn = math.sqrt(a * a + b * b)
            dn = (a * H.R[0] + b * H.D[0], a * H.R[1] + b * H.D[1])
            local = (dn[0] * 0.85, dn[1] * 0.85, math.sqrt(max(0.05, 1 - min(1, rn) ** 2)))
            k = flesh_key(lum3(blend(self.sphere_n(*p), local, 0.58)))
            if rn > 0.72:
                k = DARKER[k]
            if rn > 0.9:
                k = DARKER[k]
            if (ears[0](a, b) or ears[1](a, b)) and not dome(a, b):
                k = "d" if abs(a) < 0.8 else "s"
            if b < -0.55 and dome(a, b) and int((a * 9 + b * 3) * 2) % 3 == 0:   # coarse bristle on the crown
                k = "a" if k in "ds" else "b"
            mine[p] = k
            if (ears[0](a, b) or ears[1](a, b)) or (abs(a) >= 0.5 and b <= 0.1):
                lumps.add(p)      # ears and their shoulders stay attached to the head
        for p, k in mine.items():
            if not P["limp"] and p not in lumps and self.outside_envelope(p):
                continue      # head domes never leave the disc envelope: the disc crown stays put (ears are lumps)
            self.set(p, k, idx)
        self.face(H, idx, tusks, mouth + P["gape_add"])
        if dress == "hat":
            self.hat(H)
        if dress == "flower":
            self.flower(H, idx)
        if dress == "pearls":
            self.pearl_head = H

    def face(self, H, idx, tusks, mw):
        R, Ds = H.r * H.sa, H.r * H.sb
        mw = int(round(mw * max(1.0, H.r / 15.0)))
        m0 = H.world(0, 0.47)
        # tusks: ivory hooks from the jaw corners sweeping out and up; length 4-9 px, some chipped
        lt, rt, chip = tusks
        for s, ln in ((-1, lt), (1, rt)):
            chipped = ("L" in chip and s < 0) or ("R" in chip and s > 0)
            p0 = H.px(m0, s * (mw / 2 + 0.5), 1.5)
            p0 = H.px(m0, s * (mw / 2 + 0.5), 0.5)
            p1 = H.px(p0, s * ln * 0.75, 0.1 * ln)
            p2 = H.px(p0, s * ln * 0.9, -ln * 0.6)
            pts = bezier(p0, p1, p2, n=3 * ln)
            if chipped:
                pts = pts[: int(len(pts) * 0.68)]
            tusk = {}
            for i, (wx, wy) in enumerate(pts):
                rr = 1.5 * (1 - i / len(pts)) + 0.45
                if chipped:
                    rr = max(rr, 0.9)
                for yy in range(int(wy - 3), int(wy + 3)):
                    for xx in range(int(wx - 3), int(wx + 3)):
                        if (xx + 0.5 - wx) ** 2 + (yy + 0.5 - wy) ** 2 <= max(0.3, rr * rr):
                            n = (xx + 0.5 - wx) * LIGHT[0] + (yy + 0.5 - wy) * LIGHT[1]
                            tusk[(xx, yy)] = "v" if n > 0.25 else "u" if n > -0.45 else "t"
            if chipped:
                tip = ip(pts[-1])
                for q in ((tip[0], tip[1]), (tip[0] + 1, tip[1]), (tip[0], tip[1] + 1)):
                    if q in tusk:
                        tusk[q] = "t"
            for p in tusk:
                for q in ((p[0] + 1, p[1]), (p[0] - 1, p[1]), (p[0], p[1] + 1), (p[0], p[1] - 1)):
                    if q not in tusk and q in self.key and self.owner.get(q, -9) <= idx:
                        self.set(q, "K", idx)
            for p, k in tusk.items():
                self.set(p, k, idx)

        # snout disc: the pig read. Lit top flesh_high, rim flesh_shade, nostrils 1 x 2 px
        sn = ell(0, 0.18, 0.36, 0.26)
        for p, (a, b) in H.pixels(sn):
            edge = not ell(0, 0.18, 0.36 - 1.1 / R, 0.26 - 1.1 / Ds)(a, b)
            lit = (a * H.R[0] + b * H.D[0]) * LIGHT[0] + (a * H.R[1] + b * H.D[1]) * LIGHT[1]
            self.set(p, "s" if edge else ("h" if lit > 0.0 else "l"), idx)
        for s in (-1, 1):
            o = H.world(0.13 * s, 0.17)
            for db in (0, 1):
                self.set(ip(H.px(o, 0, db)), "c", idx)
        # snarling maw under the snout: the upper lip arches up into the snout (gums and fangs bared), the corners
        # drag down, lower teeth stand up from the dropped jaw, and the tongue lolls out past the lower lip
        mh = max(3, int(round(mw * 0.7)))
        arch = max(1.5, mh * 0.55)
        half = mw / 2
        for db2 in range(int(-2 * arch) - 2, 2 * mh + 8):
            db = db2 / 2
            for da2 in range(-2 * mw - 4, 2 * mw + 5):
                da = da2 / 2
                if abs(da) > half + 0.6:
                    continue
                f = min(1.0, (da / half) ** 2)
                top = -arch * (1 - f)                 # arched upper lip, highest at the centre
                bot = mh - 0.8 * f * mh * 0.35        # lower jaw, corners pulled down and in
                q = ip(H.px(m0, da, db))
                if abs(da) <= half and top <= db <= bot:
                    self.set(q, "c", idx)
                elif abs(da) <= half + 0.6 and top - 1.0 <= db < top:
                    self.set(q, "s", idx)             # raised gum line
                elif abs(da) <= half and bot < db <= bot + 1.2:
                    self.set(q, "d", idx)             # lower lip
        for sd in (-1, 1):
            fang = H.px(m0, sd * (half - 1.2), -arch * (1 - ((half - 1.2) / half) ** 2))
            for db in (0, 1, 2):
                self.set(ip(H.px(fang, 0, db)), "v", idx)
            low = H.px(m0, sd * (half - 2.2), mh - 0.4)
            for db in (0, -1):
                self.set(ip(H.px(low, 0, db)), "u", idx)
        for db in range(int(mh - 1), int(mh + 3)):     # tongue lolling over the lower lip
            for da in (-1, 0, 1) if db < mh + 2 else (0,):
                self.set(ip(H.px(m0, da + 0.5, db)), "r", idx)
        # cheek folds from the snout down to the dragged mouth corners, bags under the eyes
        for sd in (-1, 1):
            a0 = H.world(0.42 * sd, 0.2)
            a1 = H.px(m0, sd * (half + 1.0), mh * 0.7)
            for t in range(7):
                self.set(ip((a0[0] + (a1[0] - a0[0]) * t / 6, a0[1] + (a1[1] - a0[1]) * t / 6)), "d", idx)
            for t in range(-2, 3):
                self.set(ip(H.px(H.world(0.36 * sd, -0.22), t, 2.4 + 0.15 * abs(t))), "d", idx)
        # eyes: a 2 px K pupil and 1-2 px eye white in a dark socket, under a slanted brow (l over d): a scowl
        eyes = self.P["eyes"]
        for s in (-1, 1):
            e = H.world(0.36 * s, -0.22)
            nw = 2 if (eyes == "wide" or H.r >= 16) else 1
            pts_w = [H.px(e, s * i, 0) for i in range(nw)]
            pup = [H.px(e, -s, 0), H.px(e, -s, 1)]
            body = [ip(p) for p in pts_w + pup]
            for p in body:
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        q = (p[0] + dx, p[1] + dy)
                        if q not in body and self.owner.get(q) == idx:
                            self.set(q, "c", idx)
            if eyes == "shut":
                for p in body:
                    self.set(p, "K", idx)
            else:
                for p in pts_w:
                    self.set(ip(p), "w", idx)
                for p in pup:
                    self.set(ip(p), "K", idx)
            top = H.px(e, 0, -2.2 if eyes != "wide" else -3.0)
            for i in range(-3, 4):
                drop = 0.45 * i * (-s)        # higher at the outer end, lower toward the snout
                b0 = H.px(top, s * i, drop)
                self.set(ip(b0), "l", idx)
                self.set(ip(H.px(b0, 0, 1)), "d", idx)
    def hat(self, H):
        """A crushed, tilted top hat set into the rim: n crown, 2 px r band, 1 px o sheen, full K outline."""
        g = math.atan2(H.U[1], H.U[0]) + rad(16)
        U = (math.cos(g), math.sin(g))
        Rv = (-U[1], U[0])
        anchor = H.world(0, -0.72)
        mine = {}
        for y in range(int(anchor[1] - 22), int(anchor[1] + 23)):
            for x in range(int(anchor[0] - 22), int(anchor[0] + 23)):
                dx, dy = x + 0.5 - anchor[0], y + 0.5 - anchor[1]
                ha, hb = dx * Rv[0] + dy * Rv[1], dx * U[0] + dy * U[1]
                ha -= 0.22 * max(0.0, hb)                           # the crown leans: crushed
                dent = 1.5 if -1.5 <= ha <= 2.5 else 0.0            # dented top
                if 0 <= hb < 2.6 and abs(ha + 0.22 * hb) <= 11.2:
                    mine[(x, y)] = "n"
                elif 2.6 <= hb < 14.5 - dent and abs(ha) <= 6.8:
                    k = "r" if hb < 4.8 else "n"
                    if ha < -5.6 and k == "n":
                        k = "o"
                    mine[(x, y)] = k
        for (x, y), k in mine.items():
            edge = any(q not in mine for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)))
            self.set((x, y), "K" if edge else k, HAT_OWNER)

    def flower(self, H, idx):
        cx, cy = 0.46, -0.66
        for k in range(5):
            ang = rad(k * 72 - 90)
            fa, fb = cx + 0.25 * math.cos(ang), cy + 0.25 * math.sin(ang)
            for p, (a, b) in H.pixels(ell(fa, fb, 0.17, 0.17), pad=1.9):
                lit = (a - cx) * -0.6 + (b - cy) * -0.8
                self.set(p, "q" if lit > -0.05 else "p", idx)
        for p, _ab in H.pixels(ell(cx, cy, 0.1, 0.1), pad=1.9):
            self.set(p, "b", idx)

    def pearls(self, H):
        """A pearl string draped below the front head: a sagging parabola between two anchors on the head's flanks,
        2 px thick, two ivory (`v`) beads then one `t` thread link, so it hangs across the jowl and neighbours."""
        r = H.r
        x0, y0 = H.c[0] - 1.25 * r, H.c[1] - 0.15 * r
        x1, y1 = H.c[0] + 1.25 * r, H.c[1] - 0.05 * r
        sag = 1.45 * r
        path = []
        for xi in range(int(round(x0)), int(round(x1)) + 1):
            t = (xi - x0) / (x1 - x0)
            y = int(round(y0 + (y1 - y0) * t + sag * 4 * t * (1 - t)))
            if path and abs(y - path[-1][1]) > 1:
                step = 1 if y > path[-1][1] else -1
                for yy in range(path[-1][1] + step, y, step):
                    path.append((xi, yy))
            path.append((xi, y))
        for i, (x, y) in enumerate(path):
            bead = i % 3 != 2
            for q in ((x, y), (x, y + 1)):
                if self.P["limp"] or not self.outside_envelope(q):
                    self.set(q, "v" if bead else "t", HAT_OWNER)

    def cracks(self):
        """Frame 15/16 fracture lines (K, ``cracks`` px wide) from the rim toward the centre; the flesh on either
        side of a line takes the deep `d` edge. Recolours existing pixels only."""
        w = self.P["cracks"]
        if not w:
            return
        P = self.P
        lines = (((1.08, -115), (0.8, -105), (0.62, -122), (0.4, -100), (0.15, -86)),
                 ((1.08, -22), (0.8, -16), (0.6, -30), (0.36, -10)),
                 ((1.08, 40), (0.8, 56), (0.62, 44), (0.36, 70)),
                 ((1.08, 162), (0.8, 150), (0.6, 166), (0.36, 146)))
        marks = set()
        for ln in lines:
            pts = [(P["cx"] + rho * P["rx"] * math.cos(rad(a)), P["cy"] + rho * P["ry"] * math.sin(rad(a)))
                   for rho, a in ln]
            for (ax, ay), (bx, by) in zip(pts, pts[1:]):
                dx, dy = bx - ax, by - ay
                ll = dx * dx + dy * dy
                for y in range(int(min(ay, by)) - 3, int(max(ay, by)) + 4):
                    for x in range(int(min(ax, bx)) - 3, int(max(ax, bx)) + 4):
                        t = max(0.0, min(1.0, ((x + 0.5 - ax) * dx + (y + 0.5 - ay) * dy) / ll))
                        if math.hypot(x + 0.5 - ax - t * dx, y + 0.5 - ay - t * dy) <= w / 2:
                            marks.add((x, y))
        for q in marks:
            if q in self.key and self.key[q] != "K":
                for n in ((q[0] + 1, q[1]), (q[0] - 1, q[1]), (q[0], q[1] + 1), (q[0], q[1] - 1)):
                    if n not in marks and self.key.get(n) in ("m", "l", "h", "s"):
                        self.key[n] = "d"
        for q in marks:
            if q in self.key:
                self.key[q] = "K"

    def separate(self):
        """Crevices: a pixel whose 4-neighbour belongs to a deeper owner gets the crevice key on the front side and
        darkens that neighbour one step, so every head reads as its own dome. Outer edge pixels become the outline."""
        snap = dict(self.key)
        for (x, y), k in snap.items():
            mine = self.owner.get((x, y), -1)
            if k in "wKgv" or mine < 0 or mine >= TROTTER_OWNER:
                continue
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                o = self.owner.get(q)
                if o is not None and o < mine and snap.get(q) not in ("K", None):
                    self.key[(x, y)] = "c"
                    if snap.get(q) not in ("w", "g", "v"):
                        self.key[q] = DARKER.get(snap[q], "c")
                    break
        for (x, y) in list(self.key):
            if any(q not in self.key for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1))):
                self.key[(x, y)] = "K"

    def render(self) -> list[str]:
        self.body()
        for i, part in enumerate(PARTS):
            if part[0] == "head":
                self.head(i, part)
            else:
                self.patch(i, *part)
        self.pearls(self.pearl_head)
        self.trotters()
        self.tufts()
        self.separate()
        self.cracks()
        rows = [["."] * self.w for _ in range(self.h)]
        for (x, y), k in self.key.items():
            rows[y][x] = k
        return ["".join(r) for r in despeckle(rows)]


def despeckle(rows: list[list[str]], passes: int = 4) -> list[list[str]]:
    """Replace non-accent pixels that have no same-key 8-neighbour with the commonest opaque neighbour;
    drop opaque pixels with no opaque 8-neighbour."""
    h, w = len(rows), len(rows[0])
    for _ in range(passes):
        changed = False
        for y in range(h):
            for x in range(w):
                c = rows[y][x]
                if c == ".":
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if all(n == "." for n in around):
                    rows[y][x] = "."
                    changed = True
                    continue
                if c in ACCENTS or c in around:
                    continue
                opaque = [n for n in around if n != "." and n not in ACCENTS]
                if opaque:
                    rows[y][x] = max(sorted(set(opaque)), key=opaque.count)
                    changed = True
        if not changed:
            break
    return rows


def render(frame: int, _cache: dict = {}) -> list[str]:   # noqa: B006 - deterministic memo
    if frame not in _cache:
        _cache[frame] = Ball(pose_for(frame)).render()
    return _cache[frame]


# --------------------------------------------------------------------------- grid output
def palette_json() -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def build_grid() -> dict:
    frames = [{"duration_ms": round((t or 6) * 1000 / TICK_HZ), "cels": {"body": render(i)}}
              for i, t in enumerate(TICKS)]
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def report(rows: list[str], label: str) -> None:
    orphans, isolated = pg._stray_pixels([list(r) for r in rows], ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    text = "".join(rows)
    counts = " ".join(f"{k}{text.count(k)}" for k in "whnrqev")
    print(f"{label}: x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} {counts}")


# --------------------------------------------------------------------------- evidence
def _images(grid: dict, frames) -> list:
    sys.path.insert(0, str(ROOT / "scripts"))
    from pixelart.common import grid_images  # noqa: E402
    imgs = grid_images(grid)
    return [imgs[f] for f in frames]


def write_key0_scale(out: Path = EVIDENCE / "key0-scale.png") -> list[Path]:
    """Silhouette-gate evidence: key 0 at 1x on lit Floor 1 next to Carl and Donut (640 x 360, mirrored to face
    Carl, no scaling), plus the same scene at 4x (``key0-scale-4x.png``)."""
    from PIL import Image
    from pixelart.common import CARL_X, flip, load_grid_file  # noqa: E402
    from pixelart.scene import Floor, reference_idle  # noqa: E402

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    ball, b_piv = flip(_images(load_grid_file(GRID), [0])[0], tuple(PIVOT))
    entries = [(donut, donut_piv, CARL_X - 40, 0), (carl, carl_piv, CARL_X, 0), (ball, b_piv, CARL_X + 150, 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    scene = Floor().render(entries, 1)
    scene.save(out)
    four = out.with_name("key0-scale-4x.png")
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    return [out, four]


def write_roll_preview(out: Path = EVIDENCE / "roll-preview.gif") -> list[Path]:
    """The six roll frames at 1x on a brick-toned panel, 4 ticks (67 ms) each, looping; plus a 2x strip PNG."""
    from PIL import Image
    from pixelart.common import load_grid_file  # noqa: E402

    imgs = _images(load_grid_file(GRID), ROLL_FRAMES)
    frames = []
    for im in imgs:
        bg = Image.new("RGBA", im.size, (112, 58, 40, 255))
        bg.alpha_composite(im)
        frames.append(bg.convert("RGB"))
    out.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(out, save_all=True, append_images=frames[1:], duration=67, loop=0, disposal=1)
    strip = Image.new("RGB", (CANVAS[0] * 6, CANVAS[1]), (0, 0, 0))
    for i, f in enumerate(frames):
        strip.paste(f, (i * CANVAS[0], 0))
    strip_path = out.with_name("roll-strip-2x.png")
    strip.resize((strip.width * 2, strip.height * 2), Image.NEAREST).save(strip_path)
    return [out, strip_path]


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", nargs="?", choices=["key0-scale", "roll-preview"])
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.command == "key0-scale":
        for p in write_key0_scale():
            print(f"wrote {p.relative_to(ROOT)}")
        return 0
    if a.command == "roll-preview":
        for p in write_roll_preview():
            print(f"wrote {p.relative_to(ROOT)}")
        return 0
    grid = build_grid()
    for f in range(len(TICKS)):
        if True:
            report(grid["frames"][f]["cels"]["body"], f"frame {f:2d}")
    text = dump(grid)
    if a.check:
        ok = GRID.is_file() and GRID.read_text() == text
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    pg.write_ase(grid, ASE)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
