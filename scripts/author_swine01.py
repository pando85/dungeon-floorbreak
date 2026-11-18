#!/usr/bin/env python3
"""Author SWINE-01 Ball of Swine (Floor 01 arena boss) keys as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/swine-01-spec.md``. The Ball is built from a feature model on a disc: Tuskling heads,
costume scraps, trotters and bristle tufts placed by polar position (rho, alpha) and a facing angle, so a roll
phase is the same model with every alpha and facing advanced by ``theta`` and re-rasterized (spec section 3.1).
Shading uses one fixed screen-space light (torch upper left) after the rotation, so light never turns with the
ball. No raster image-model output, no tracing or sampling of concept pixels, no resampling.

Phase 1 authors idle key 0 only (silhouette gate); the remaining keys follow after the gate passes.

    python3 scripts/author_swine01.py               # write grid + .ase
    python3 scripts/author_swine01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_swine01.py key0-scale    # write docs/evidence/swine-01/key0-scale*.png
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
CANVAS = (176, 144)
PIVOT = (88, 143)
TICK_HZ = 60
DISC_C = (88.0, 83.0)      # disc centre; the disc is cut at the contact row (flat weight patch)
DISC_R = (64.0, 62.0)

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
FLESH = "dsml"
DARKER = {"h": "l", "l": "m", "m": "s", "s": "d", "d": "c", "c": "c", "v": "u", "u": "t", "t": "a",
          "b": "a", "a": "c", "o": "n", "n": "n", "r": "d", "g": "r", "q": "p", "p": "n", "K": "K", "w": "w"}
_l = (-0.5, -0.62, 0.6)
LIGHT = tuple(c / math.sqrt(sum(v * v for v in _l)) for c in _l)   # torch key upper left, screen space

TAGS = [("idle", 0, 0)]          # phase 1: key 0 only; the full tag plan is spec section 4
TICKS = [16]
KEY_FRAMES = (0,)

# --------------------------------------------------------------------------- feature model (theta = 0)
# Heads: (rho, alpha deg, radius px, up deg, width scale, height scale, dress). Alpha and up are screen angles (0 = facing right, -90 = up).
# Drawn back to front: the list order is the depth order; the front-facing hero head is last.
HEADS = (
    (0.95, -172, 11, -172, 1.0, 0.8, None),        # rim heads: half outside the mass, they scallop the outline
    (0.95, -66, 11, -60, 1.0, 0.8, None),
    (0.95, 34, 11, 30, 1.0, 0.8, None),
    (0.95, 152, 11, 158, 1.0, 0.8, None),
    (0.12, 160, 21, -118, 0.95, 0.86, "gape"),     # centre face, half buried under its neighbours
    (0.90, 128, 11, 128, 1.0, 0.85, None),         # rim, lower left, inverted
    (0.90, -18, 11, -10, 0.9, 1.0, None),          # rim, front upper, sideways
    (0.88, 82, 12, 74, 1.0, 0.8, "gape"),          # rim, bottom, inverted, squashed on the floor
    (0.90, -122, 10, -118, 1.0, 0.9, None),        # rim, top left, half outside the mass
    (0.62, -96, 16, -84, 1.05, 0.82, "gape"),      # top, squeals upward
    (0.64, -152, 15, -140, 0.9, 1.0, "hat"),       # upper left, top hat breaks the silhouette
    (0.62, 176, 15, 196, 1.0, 0.85, None),         # back, sideways
    (0.60, 114, 16, 124, 1.0, 0.9, "bowtie"),      # lower left, upside down, bow tie under its chin
    (0.56, 50, 16, 40, 0.95, 0.85, "gape"),        # lower right, upside down
    (0.57, -46, 17, -64, 1.0, 0.9, "flower"),      # upper right, flower and pearls
    (0.60, 10, 18, -22, 1.05, 0.9, "gape"),        # front: the aggressor snarling at the player
)
TROTTERS = ((26, 17), (150, 16), (-108, 14), (96, 12))   # (alpha deg, length px): limbs kicking out of the mass
# Body regions behind the heads (alpha ranges at theta 0): what shows in the gaps between the heads.
REGIONS = ((-178, -112, "sequin"), (-112, -28, "flesh"), (-28, 62, "hair"), (62, 142, "tux"), (142, 182, "flesh"))
TUFTS = (-176, -164, -134, -112, -101, -88, -74, -62, -38, -28, -4, 6, 34, 44, 60, 70, 108, 138, 162, 194)


def rad(deg: float) -> float:
    return math.radians(deg)


def to_xy(rho: float, alpha: float) -> tuple[float, float]:
    return DISC_C[0] + rho * DISC_R[0] * math.cos(rad(alpha)), DISC_C[1] + rho * DISC_R[1] * math.sin(rad(alpha))


def disc_uv(x: float, y: float) -> tuple[float, float]:
    return (x - DISC_C[0]) / DISC_R[0], (y - DISC_C[1]) / DISC_R[1]


def lum3(n) -> float:
    d = n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2]
    return 0.3 + 0.7 * max(0.0, d)


def sphere_n(x: float, y: float):
    u, v = disc_uv(x + 0.5, y + 0.5)
    r2 = min(0.999, u * u + v * v)
    return (u, v, math.sqrt(1.0 - r2))


def blend(a, b, wa: float):
    n = tuple(wa * a[i] + (1 - wa) * b[i] for i in range(3))
    s = math.sqrt(sum(c * c for c in n)) or 1.0
    return tuple(c / s for c in n)


def flesh_key(v: float) -> str:
    return "d" if v < 0.42 else "s" if v < 0.6 else "m" if v < 0.8 else "l"


class Head:
    """A Tuskling head in local face coordinates: a along the face's right, b from forehead to chin (units of r)."""

    def __init__(self, rho, alpha, r, up, sa, sb, dress, theta):
        self.c = to_xy(rho, alpha + theta)
        self.r = r * 1.12        # heads press into each other: no flat gaps between them
        self.sa, self.sb = sa, sb
        g = rad(up + theta)
        self.U = (math.cos(g), math.sin(g))
        self.D = (-self.U[0], -self.U[1])
        self.R = (-self.U[1], self.U[0])
        self.dress = dress

    def local(self, x, y):
        dx, dy = x + 0.5 - self.c[0], y + 0.5 - self.c[1]
        return ((dx * self.R[0] + dy * self.R[1]) / (self.r * self.sa),
                (dx * self.D[0] + dy * self.D[1]) / (self.r * self.sb))

    def world(self, a, b):
        a, b = a * self.sa * self.r, b * self.sb * self.r
        return self.c[0] + a * self.R[0] + b * self.D[0] + 0.5, self.c[1] + a * self.R[1] + b * self.D[1] + 0.5

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


def bezier(p0, p1, p2, n=24):
    return [((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
             (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]) for t in (i / (n - 1) for i in range(n))]


# --------------------------------------------------------------------------- renderer
class Ball:
    def __init__(self, theta: float = 0.0):
        self.theta = theta
        self.key: dict = {}
        self.owner: dict = {}      # pixel -> depth index (-1 body, 0.. heads, 100 props)
        self.w, self.h = CANVAS

    def inside(self, x, y) -> bool:
        return 0 <= x < self.w and 0 <= y <= PIVOT[1]

    def set(self, p, k, owner=None):
        if self.inside(*p):
            self.key[p] = k
            if owner is not None:
                self.owner[p] = owner

    # body: the disc and its regions (recessed: one step darker than an exposed surface)
    def body(self):
        for y in range(self.h):
            for x in range(self.w):
                u, v = disc_uv(x + 0.5, y + 0.5)
                if u * u + v * v > 1.0 or y > PIVOT[1]:
                    continue
                n = sphere_n(x, y)
                lv = lum3(n) * 0.66
                ang = (math.degrees(math.atan2(v, u)) - self.theta + 540) % 360 - 180
                reg = next((r for a0, a1, r in REGIONS if a0 <= ang < a1 or a0 <= ang + 360 < a1), "flesh")
                k = self.region_key(reg, x, y, lv, ang)
                self.set((x, y), k, -1)

    def region_key(self, reg, x, y, lv, ang):
        if reg == "sequin":
            if lv > 0.62 and (x + 2 * y) % 5 == 0 and (x * 7 + y * 3) % 3 != 0:
                return "g"
            return "r" if lv > 0.4 else "d"
        if reg == "tux":
            # black jacket with a soiled shirt wedge
            if -0.5 < (ang - 100) / 12 < 0.5 and lv > 0.35:
                return "u" if lv < 0.62 else "v"
            return "o" if lv > 0.7 else "n"
        if reg == "hair":
            stripe = (x * 2 + y) % 6 < 2
            return "b" if (lv > 0.62 and stripe) else "a" if lv > 0.3 else "c"
        return flesh_key(lv * 0.92)

    def tufts(self):
        for i, a in enumerate(TUFTS):
            a2 = a + self.theta
            ln = 4 + (i * 5) % 4
            base = to_xy(0.97, a2)
            tip = to_xy(1.0 + ln / 62.0, a2 + (4 if i % 2 else -4))
            side = (-math.sin(rad(a2)), math.cos(rad(a2)))
            for s in range(0, 7):
                t = s / 6
                px = base[0] + (tip[0] - base[0]) * t
                py = base[1] + (tip[1] - base[1]) * t
                half = 2.4 * (1 - t) + 0.3
                for q in (-1, 0, 1):
                    if abs(q) <= half:
                        p = (int(px + side[0] * q * 0.8), int(py + side[1] * q * 0.8))
                        if p not in self.key:
                            self.set(p, "b" if (a2 % 360) > 180 and t > 0.4 else "a", -1)

    def trotters(self):
        """Short pig legs kicking out of the mass, past the rim: flesh shank, black split hoof."""
        for a, ln in TROTTERS:
            a2 = a + self.theta
            ux, uy = math.cos(rad(a2)), math.sin(rad(a2))
            base = to_xy(0.74, a2)
            for s in range(ln * 2 + 1):
                d = s / 2
                cx, cy = base[0] + ux * d, base[1] + uy * d
                rr = 3.2 if d < ln - 4 else 3.6
                for yy in range(int(cy - 5), int(cy + 6)):
                    for xx in range(int(cx - 5), int(cx + 6)):
                        ox, oy = xx + 0.5 - cx, yy + 0.5 - cy
                        if ox * ox + oy * oy <= rr * rr:
                            side = ox * LIGHT[0] + oy * LIGHT[1]
                            if d >= ln - 4:
                                k = "o" if side > 1.2 else "n"
                            else:
                                k = "l" if side > 1.4 else "m" if side > -0.4 else "s" if side > -1.8 else "d"
                            self.set((xx, yy), k, 40)
            for t in range(4):     # the cleft of the hoof
                q = (base[0] + ux * (ln - t + 0.5), base[1] + uy * (ln - t + 0.5))
                self.set((int(q[0]), int(q[1])), "K", 40)

    def head(self, idx, spec):
        rho, alpha, r, up, sa, sb, dress = spec
        H = Head(rho, alpha, r, up, sa, sb, dress, self.theta)
        dome = ell(0, 0, 1.0, 1.0)
        ears = (tri((-0.8, -0.38), (-0.25, -0.86), (-1.05, -1.12)), tri((0.8, -0.38), (0.25, -0.86), (1.05, -1.12)))
        mine = {}
        for p, (a, b) in H.pixels(lambda a, b: dome(a, b) or ears[0](a, b) or ears[1](a, b)):
            rn = math.sqrt(a * a + b * b)
            dn = (a * H.R[0] + b * H.D[0], a * H.R[1] + b * H.D[1])
            local = (dn[0] * 0.85, dn[1] * 0.85, math.sqrt(max(0.05, 1 - min(1, rn) ** 2)))
            k = flesh_key(lum3(blend(sphere_n(*p), local, 0.58)))
            if rn > 0.84:
                k = DARKER[k]
            if (ears[0](a, b) or ears[1](a, b)) and not dome(a, b):
                k = "d" if abs(a) < 0.8 else "s"
            # bristle on the crown: coarse dark hair strokes
            if b < -0.55 and dome(a, b) and int((a * 9 + b * 3) * 2) % 3 == 0:
                k = "a" if k in "ds" else "b"
            mine[p] = k
        for p, k in mine.items():
            self.set(p, k, idx)
        self.face(H, idx)
        return H

    def face(self, H, idx):
        gape = H.dress == "gape"
        # jowls and lower lip: sagging flesh_deep fold under the mouth
        for p, (a, b) in H.pixels(ell(0, 0.66, 0.56 if gape else 0.44, 0.34 if gape else 0.24)):
            self.set(p, "d", idx)
        # mouth: a dark gape, a red tongue and a row of ivory teeth along the upper lip in the big ones
        mo = ell(0, 0.64, 0.46 if gape else 0.34, 0.24 if gape else 0.11)
        for p, (a, b) in H.pixels(mo):
            self.set(p, "r" if gape and b > 0.72 and abs(a) < 0.22 else "c", idx)
        if gape:
            for i, ta in enumerate((-0.3, -0.12, 0.08, 0.26)):
                w = H.world(ta, 0.46)
                for dx, dy in ((0, 0), (0, 1)):
                    self.set((int(w[0]) + dx, int(w[1]) + dy), "v" if i % 2 == 0 else "u", idx)
        # snout disc: the pig read. Lit top in flesh_high, rim in flesh_shade, two dark nostrils
        sn = ell(0, 0.2, 0.36, 0.25)
        for p, (a, b) in H.pixels(sn):
            edge = not ell(0, 0.2, 0.36 - 1.1 / (H.r * H.sa), 0.25 - 1.1 / (H.r * H.sb))(a, b)
            lit = (a * H.R[0] + b * H.D[0]) * LIGHT[0] + (a * H.R[1] + b * H.D[1]) * LIGHT[1]
            self.set(p, "s" if edge else ("h" if lit > 0.0 else "l"), idx)
        for s in (-1, 1):
            for p, _ab in H.pixels(ell(0.13 * s, 0.23, max(0.08, 0.9 / H.r), max(0.1, 1.4 / H.r))):
                self.set(p, "c", idx)
        # snout-bridge wrinkles above the snout
        for row in (-0.07, -0.16):
            for t in range(-2, 3):
                q = H.world(0.08 * t, row + 0.03 * abs(t))
                self.set((int(q[0]), int(q[1])), "d", idx)
        # eyes: sunk in a dark socket under a heavy brow; tiny bloodshot-cream eye with a black pupil
        for s in (-1, 1):
            for p, _ab in H.pixels(ell(0.36 * s, -0.24, 0.17, 0.11)):
                self.set(p, "c", idx)
            e = H.world(0.34 * s, -0.22)
            ex, ey = int(e[0]), int(e[1])
            self.set((ex, ey), "w", idx)
            o = H.world(0.34 * s + 0.08 * s, -0.22)
            if (int(o[0]), int(o[1])) != (ex, ey) and H.r >= 15:
                self.set((int(o[0]), int(o[1])), "w", idx)
            pp = H.world(0.34 * s - 0.09 * s, -0.22)
            if (int(pp[0]), int(pp[1])) != (ex, ey):
                self.set((int(pp[0]), int(pp[1])), "K", idx)
            for t in range(7):    # brow ridge: a lit fold that slopes down to the snout (scowl)
                q = H.world((0.56 - 0.06 * t) * s, -0.42 + 0.03 * t)
                self.set((int(q[0]), int(q[1])), "l" if (s < 0) == (H.R[0] > 0) else "m", idx)
        # tusks: ivory hooks from the lower jaw, sweeping out and up past the snout; dark edge on the flesh
        big = 1.3 if gape else 1.0
        for s in (-1, 1):
            pts = bezier((0.3 * s, 0.62), (0.72 * s * big, 0.7), (0.6 * s * big, -0.22 - 0.25 * (big - 1)))
            tusk = {}
            for i, (a, b) in enumerate(pts):
                rr = (0.15 * (1 - i / len(pts)) + 0.03) * H.r
                wx, wy = H.world(a, b)
                for yy in range(int(wy - rr - 1), int(wy + rr + 2)):
                    for xx in range(int(wx - rr - 1), int(wx + rr + 2)):
                        if (xx + 0.5 - wx) ** 2 + (yy + 0.5 - wy) ** 2 <= max(0.5, rr * rr):
                            n = (xx + 0.5 - wx) * LIGHT[0] + (yy + 0.5 - wy) * LIGHT[1]
                            tusk[(xx, yy)] = "v" if n > 0.3 else "u" if n > -0.5 else "t"
            for p in tusk:
                for q in ((p[0] + 1, p[1]), (p[0] - 1, p[1]), (p[0], p[1] + 1), (p[0], p[1] - 1)):
                    if q not in tusk and q in self.key and self.owner.get(q, -9) <= idx:
                        self.set(q, "K", idx)
            for p, k in tusk.items():
                self.set(p, k, idx)
        if H.dress == "hat":
            self.hat(H, idx)
        if H.dress == "flower":
            self.flower(H, idx)
            self.pearls(H, idx)
        if H.dress == "bowtie":
            self.bow_tie(H, idx)

    def hat(self, H, idx):
        """A crushed top hat with a red band: rim and crown in cloth, the lit side in cloth_sheen."""
        for p, (a, b) in H.pixels(lambda a, b: (-1.0 <= a <= 0.95 and -1.08 <= b <= -0.86) or
                                  (-0.62 <= a <= 0.55 and -2.15 <= b <= -0.86), pad=2.6):
            band = -1.3 <= b <= -1.1 and -0.62 <= a <= 0.55
            lit = (a * H.R[0] + b * H.D[0]) * LIGHT[0] + (a * H.R[1] + b * H.D[1]) * LIGHT[1] > 0.55
            self.set(p, "r" if band else ("o" if lit else "n"), 70)

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

    def pearls(self, H, idx):
        pts = bezier((-0.85, 0.86), (0.0, 1.32), (0.98, 0.72), n=9)
        for a, b in pts:
            wx, wy = H.world(a, b)
            x, y = int(wx), int(wy)
            for p, k in (((x, y), "v"), ((x + 1, y), "u"), ((x, y + 1), "u"), ((x + 1, y + 1), "t")):
                self.set(p, k, idx)

    def bow_tie(self, H, idx):
        """A soiled shirt collar and a black bow tie under the chin of an upside-down Tuskling."""
        for p, (a, b) in H.pixels(lambda a, b: 0.86 <= b <= 1.3 and abs(a) <= 0.55 - (b - 0.86) * 0.9, pad=1.8):
            self.set(p, "v" if b < 1.0 else "u", idx)
        for p, (a, b) in H.pixels(lambda a, b: abs(b - 1.0) <= 0.05 + abs(a) * 0.45 and abs(a) <= 0.42, pad=1.8):
            self.set(p, "o" if a * H.R[0] < 0 else "n", idx)
        for p, (a, b) in H.pixels(ell(0, 1.0, 0.08, 0.08), pad=1.8):
            self.set(p, "K", idx)

    def separate(self):
        """Crevices: a pixel whose 4-neighbour belongs to a deeper owner gets the crevice key on the front side,
        so every head reads as its own dome. Outer edge pixels become the outline key."""
        snap = dict(self.key)
        for (x, y), k in snap.items():
            mine = self.owner.get((x, y), -1)
            if k in "wKgv" or mine < 0:
                continue
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                o = self.owner.get(q)
                if o is not None and o < mine and snap.get(q) not in ("K", None) and mine < 70:
                    self.key[(x, y)] = "c"
                    if snap.get(q) not in ("w", "g", "v") and o < 40:
                        self.key[q] = DARKER.get(snap[q], "c")
                    break
        for (x, y) in list(self.key):
            if any(q not in self.key for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1))):
                self.key[(x, y)] = "K"

    def render(self) -> list[str]:
        self.body()
        self.trotters()
        heads = list(HEADS)
        for i, spec in enumerate(heads):
            self.head(i, spec)
        self.tufts()
        self.separate()
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


def render(frame: int) -> list[str]:
    if frame == 0:
        return Ball(0.0).render()
    raise ValueError(f"frame {frame} is not authored in phase 1")


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
    frames = [{"duration_ms": round(t * 1000 / TICK_HZ), "cels": {"body": render(i)}} for i, t in enumerate(TICKS)]
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
    used = sorted({ch for r in rows for ch in r} - {"."})
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} keys {''.join(used)}")


# --------------------------------------------------------------------------- scale evidence
def write_key0_scale(out: Path = ROOT / "docs/evidence/swine-01/key0-scale.png") -> list[Path]:
    """Silhouette-gate evidence: key 0 at 1x on lit Floor 1 next to Carl and Donut (640 x 360, mirrored to face
    Carl, no scaling), plus the same scene at 4x (``key0-scale-4x.png``)."""
    from PIL import Image
    from pixelart.common import CARL_X, flip, grid_images, load_grid_file  # noqa: E402
    from pixelart.scene import Floor, reference_idle  # noqa: E402

    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    ball, b_piv = flip(grid_images(load_grid_file(GRID))[0], tuple(PIVOT))
    entries = [(donut, donut_piv, CARL_X - 40, 0), (carl, carl_piv, CARL_X, 0), (ball, b_piv, CARL_X + 150, 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    scene = Floor().render(entries, 1)
    scene.save(out)
    four = out.with_name("key0-scale-4x.png")
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    return [out, four]


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
    for f in range(len(TICKS)):
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
