#!/usr/bin/env python3
"""BOSS-F01 effects and props: VFX (pound debris, shockwave ripple, crash dust, mirror crack, roar push ring, floor-impact
dust, boil spatter, blood pool), the plate rack, the rotten strut and the ledge with its release plate.

Usage:

    python3 scripts/author_boss_f01_fx.py build [--check]

Every sprite is a rule (a lumpy disc, a thick line, a lit rectangle) rasterized at native size with a fixed-seed integer
hash: no raster image-model output, no tracing, no resampling, no floating-point randomness. Colours are existing repo
hexes (the floor01-catacomb ramps, the trog-family and VFX iron and smoke keys, the goblin-death gore ramp): no new
colour. Frames are drawn in pixel space (x right, y down); the pivot of each atlas is its floor-contact point.
Design spec: ``docs/02-art/floor-01-boss-design.md`` sections 2, 5 and 8. Presentation-only art tooling.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
SRC = ROOT / "assets/source/enemies/brute_boss_f01"
MAN = ROOT / "assets/manifests"
TICK_HZ, HOLD_MS = 60, 100

# every colour is an existing repo hex (palette_id "boss-f01-fx-v0")
COLOURS = {
    "K": ("outline_soot", "#1b1124"), "a": ("dust_deep", "#5d4045"), "b": ("dust_mid", "#7a5249"),
    "c": ("dust_light", "#9a684f"), "d": ("dust_hi", "#b98256"), "e": ("chip_dark", "#57352c"),
    "f": ("chip_mid", "#74472f"), "g": ("chip_light", "#d0965c"), "h": ("chip_hi", "#f2c486"),
    "i": ("iron_dark", "#3a3245"), "j": ("iron_mid", "#766c86"), "k": ("iron_light", "#8b93a6"),
    "w": ("flash_white", "#efe3d3"), "x": ("timber_deep", "#3a2219"), "y": ("timber_dark", "#52301f"),
    "z": ("timber_mid", "#6d4127"), "u": ("timber_light", "#8d5631"), "s": ("smoke_deep", "#2b2230"),
    "B": ("blood_deep", "#6e1a22"), "R": ("blood_red", "#b8322e"), "P": ("blood_pink", "#e2434b"),
    "m": ("mouth_dark", "#4a1020"),
}


def h2(x: int, y: int, seed: int) -> int:
    n = (x * 374761393 + y * 668265263 + seed * 2147483647) & 0xFFFFFFFF
    n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
    return (n ^ (n >> 16)) & 0xFFFF


def lump(theta: float, seed: int, amp: float, lobes: int = 5) -> float:
    v = 0.0
    for k in range(3):
        v += math.sin(theta * (lobes + 2 * k) + (h2(k, seed, 7) % 628) / 100.0) / (1 + k)
    return 1.0 + amp * v / 1.8


class Img:
    def __init__(self, w: int, h: int) -> None:
        self.w, self.h = w, h
        self.p = [["."] * w for _ in range(h)]

    def put(self, x: int, y: int, k: str) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            self.p[y][x] = k

    def get(self, x: int, y: int) -> str:
        return self.p[y][x] if 0 <= x < self.w and 0 <= y < self.h else "."

    def rect(self, x0: int, y0: int, x1: int, y1: int, k: str) -> None:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, k)

    def disc(self, cx: float, cy: float, rx: float, ry: float, pick, seed: int = 0, amp: float = 0.0, lobes: int = 5) -> None:
        for y in range(int(cy - ry * (1 + amp)) - 1, int(cy + ry * (1 + amp)) + 2):
            for x in range(int(cx - rx * (1 + amp)) - 1, int(cx + rx * (1 + amp)) + 2):
                dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
                d = math.hypot(dx, dy)
                lim = lump(math.atan2(dy, dx), seed, amp, lobes) if amp else 1.0
                if d <= lim:
                    k = pick(d / lim, dx, dy)
                    if k:
                        self.put(x, y, k)

    def line(self, x0: float, y0: float, x1: float, y1: float, k: str, width: float = 1.0) -> None:
        n = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
        for i in range(n + 1):
            t = i / n
            x, y = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            for yy in range(int(y - width / 2), int(y + width / 2) + 1):
                for xx in range(int(x - width / 2), int(x + width / 2) + 1):
                    self.put(xx, yy, k)

    def drop_isolated(self) -> "Img":
        """Remove pixels with no opaque 8-neighbour (the grid lint treats a lone pixel as an error)."""
        lone = [(x, y) for y in range(self.h) for x in range(self.w) if self.p[y][x] != "." and not any(
            self.get(x + dx, y + dy) != "." for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy)]
        for x, y in lone:
            self.p[y][x] = "."
        return self

    def rows(self) -> list[str]:
        return ["".join(r) for r in self.drop_isolated().p]


def lit(ramp: str):
    """A disc picker: light from the upper left over the ramp (dark to light)."""
    n = len(ramp)
    return lambda t, dx, dy: ramp[max(0, min(n - 1, int((0.55 - 0.45 * (dx + dy) - 0.25 * t) * n)))]


def chunk(img: Img, x: float, y: float, r: float, seed: int, ramp: str = "efgh") -> None:
    """A brick chip: lit from the upper left; chips of radius 2 and up get a soot outline. Chips that leave the frame are dropped."""
    if not (r + 3 <= x <= img.w - r - 4 and r + 3 <= y):
        return
    if r >= 2.0:
        img.disc(x, y, r + 0.9, r * 0.8 + 0.9, lambda t, dx, dy: "K", seed, 0.3, 4)
    img.disc(x, y, r, r * 0.8, lit(ramp), seed, 0.3, 4)


def dust(img: Img, x: float, y: float, r: float, seed: int, fade: float = 0.0, flat: float = 0.8) -> None:
    """A billowing dust cloud (a cluster of lumpy lobes), four-step ramp lit from the upper left; ``fade`` 0..1 dissolves it
    with a fixed-seed dither from the edges inwards."""
    pick = lit("abcd")
    lobes = [(0.0, 0.0, 1.0), (-0.55, -0.1, 0.7), (0.6, 0.05, 0.75), (0.1, -0.55, 0.6)]
    for yy in range(int(y - r * 1.4) - 2, int(y + r * flat * 1.4) + 3):
        for xx in range(int(x - r * 1.9) - 2, int(x + r * 1.9) + 3):
            best = None
            for li, (ox, oy, sc) in enumerate(lobes):
                dx = (xx + 0.5 - (x + ox * r)) / (r * 1.1 * sc)
                dy = (yy + 0.5 - (y + oy * r)) / (r * flat * sc)
                d = math.hypot(dx, dy)
                if d <= lump(math.atan2(dy, dx), seed + li, 0.22, 6) and (best is None or d < best[0]):
                    best = (d, dx, dy)
            if best and h2(xx, yy, seed + 31) % 100 >= fade * 100 * (0.35 + 0.65 * min(1.0, best[0])):
                img.put(xx, yy, pick(best[0], best[1], best[2]))


# ------------------------------------------------------------------ VFX atlas (152 x 72, pivots are per frame, see ANCHOR)
VW, VH = 152, 72
VPIV = (76, 71)


def pound_debris(i: int) -> Img:
    """Fists hit the floor: one low dust cloud that rolls out and up, brick chunks thrown up and out, chips left on the floor."""
    t = [1, 4, 8, 13, 20][i]
    img = Img(VW, VH)
    for s in range(5):                       # a billowing cloud on both sides of the strike point
        side = -1 if s % 2 else 1
        r = 7 + 0.45 * t + (s % 3) * 1.5
        dust(img, VPIV[0] + side * (3 + 1.5 * min(t, 14) + s * 4), 68 - 5 - 0.3 * t - (s % 3) * 2, r, 10 + s, fade=min(0.85, t / 24), flat=0.9)
    for s in range(10):                      # chunks: ballistic arcs from the impact
        vx = (h2(s, 1, 5) % 17 - 8) * 0.6
        vy = 2.8 + (h2(s, 2, 5) % 10) * 0.3
        x, y = VPIV[0] + vx * t + (s % 3 - 1) * 4, 64 - (vy * t - 0.11 * t * t)
        if y >= 67 and t > 6:
            y = 67 - (s % 2) * 2
        chunk(img, x, y, 1.6 + (s % 3) * 0.8, 40 + s)
    return img


def shockwave_ripple(i: int) -> Img:
    """A 10 px floor ripple travelling right: a rounded lit crest with a vertical front, a dust skirt behind it, loose chips."""
    img = Img(VW, VH)
    front = 76 + 18
    for x in range(front - 52, front + 1):
        u = (x - (front - 52)) / 52.0                             # 0 at the tail, 1 at the front
        wob = (h2(x // 2, i, 3) % 3) - 1
        height = max(1, min(10, int(round(10 * (u ** 1.15) + (wob if 0.15 < u < 0.92 else 0)))))
        if x >= front - 1:
            height = 10
        for dy in range(height):
            y = 70 - dy
            k = "d" if dy == height - 1 else ("c" if dy >= height - 3 else ("b" if dy >= 3 else "a"))
            img.put(x, y, k)
        if height >= 4:                                         # dust above the crest, a few pixels drifting
            for s in range(2):
                if h2(x, i * 7 + s, 9) % 5 == 0:
                    img.put(x, 70 - height - 1 - s - (h2(x, i, 1) % 2), "c" if s else "d")
    for y in range(60, 71):                                     # the vertical front edge, lit
        img.put(front + 1, y, "K")
    for s in range(5):
        chunk(img, front - 8 - s * 9 + (i * 3 + s) % 4, 58 - (s % 2) * 3 + (i + s) % 2 * 2, 1.2 + (s % 2) * 0.6, 70 + s, "efg")
    return img


def crash_dust(i: int) -> Img:
    """The boss meets the wall at the pivot: dust billows back into the room and up the wall, chips ricochet."""
    t = [1, 3, 6, 10, 15, 22][i]
    img = Img(VW, VH)
    for s in range(7):
        r = 8 + 0.4 * t + (s % 4) * 1.5
        x = VPIV[0] - 8 - 1.5 * min(t, 11) * (0.4 + (s % 5) * 0.22) - (s % 3) * 3
        y = 62 - 0.9 * t * ((s * 7) % 10) / 10.0 - (s % 3) * 6
        dust(img, x, y, r, 100 + s, fade=min(0.9, t / 28), flat=0.95)
    for s in range(9):
        vx = -(1.0 + (h2(s, 3, 9) % 9) * 0.35)
        vy = 1.8 + (h2(s, 4, 9) % 8) * 0.3
        x, y = VPIV[0] - 2 + vx * t, 58 - (vy * t - 0.1 * t * t)
        if y > 67:
            y = 67 - (s % 2) * 2
        chunk(img, x, y, 1.5 + (s % 3) * 0.8, 120 + s)
    return img


def floor_dust(i: int) -> Img:
    """The felled body lands (brute_ko_timber): a 120 px dust burst both ways along the floor."""
    t = [1, 4, 8, 14, 22][i]
    img = Img(VW, VH)
    for s in range(8):
        side = -1 if s % 2 else 1
        r = 8 + 0.4 * t + (s % 4) * 1.5
        dust(img, VPIV[0] + side * (4 + 1.4 * min(t, 18) * (0.5 + (s % 4) * 0.25)), 62 - 0.3 * t - (s % 3) * 4, r, 200 + s,
             fade=min(0.9, t / 30), flat=0.9)
    return img


def roar_ring(i: int) -> Img:
    """The push wave, radius 18, 36, 54, 72 px: a flat floor ellipse three pixels thick, a lit outer rim, a dust inner
    band, fading and breaking up as it spreads."""
    r = [18, 36, 54, 72][i]
    img = Img(VW, VH)
    cy, ry = 57, r * 0.2
    rim = ("w", "h", "g", "d")[i]
    band = ("d", "g", "d", "c")[i]
    low = ("c", "c", "b", "a")[i]
    for ang in range(0, 1440):
        a = math.radians(ang / 4.0)
        for off, k in ((0.0, rim), (0.8, rim), (1.6, band), (2.4, band), (3.2, low)):
            x = VPIV[0] + (r - off * 1.1) * math.cos(a)
            y = cy + (ry - off * 0.8) * math.sin(a)
            if i >= 2 and h2(int(x), int(y), 4 + i) % (4 if i == 2 else 2) == 0:
                continue                                            # the wave breaks up as it spreads
            if math.sin(a) > -0.2 or h2(int(x), int(y), 5) % 3 == 0:    # the far half is fainter
                img.put(int(round(x)), int(round(y)), k)
    return img


def mirror_crack(i: int) -> Img:
    """A star crack on a cosmetic wall mirror: three states (small, spread, shattered), light cracks with a dark edge."""
    img = Img(VW, VH)
    cx, cy = 76, 36
    rays, reach = [(5, 16), (7, 26), (9, 34)][i]
    for s in range(rays):
        a = (s / rays) * math.tau + (h2(s, i, 11) % 100) / 100.0 * 0.5
        x, y = cx, cy
        length = reach * (0.55 + 0.45 * (h2(s, 5, 11) % 100) / 100.0)
        steps = int(length // 3)
        for st in range(steps):
            a += ((h2(s, st, 13) % 100) / 100.0 - 0.5) * 0.7
            nx, ny = x + 3 * math.cos(a), y + 3 * math.sin(a) * 1.15
            img.line(x + 1, y + 1, nx + 1, ny + 1, "K", 1)
            img.line(x, y, nx, ny, "w" if st < steps - 1 else "k", 1)
            x, y = nx, ny
    for ring in range(i):                                           # concentric chords between the rays
        r = 7 + ring * 9
        for ang in range(0, 360, 6):
            if h2(ang, ring, 17) % 3:
                a = math.radians(ang)
                img.put(int(cx + r * math.cos(a)), int(cy + r * 1.1 * math.sin(a)), "k")
    if i == 2:                                                      # missing shards: dark holes
        for (sx, sy, sr) in ((cx - 8, cy - 5, 3), (cx + 6, cy + 4, 3), (cx + 2, cy - 12, 2)):
            img.disc(sx, sy, sr, sr, lambda t, dx, dy: "K", 30 + sx, 0.4, 4)
    return img


def boil_spatter(i: int) -> Img:
    """A boil bursts: red spatter radiating from the pivot (hip height), drops falling to the floor and splashing."""
    t = [2, 5, 9, 16][i]
    img = Img(VW, VH)
    cx, cy = VPIV[0], 52
    for s in range(18):
        a = math.tau * s / 18 + (h2(s, 2, 21) % 100) / 100.0 * 0.4
        sp = 1.4 + (h2(s, 3, 21) % 100) / 100.0 * 2.6
        x, y = cx + math.cos(a) * sp * min(t, 11) * 1.3, cy + math.sin(a) * sp * min(t, 11) * 1.0 + 0.1 * t * t
        r = max(1.1, 3.2 - t * 0.09 - (s % 3) * 0.6)
        if y > 67:
            y = 68 - (s % 2)
            img.disc(x, y, r + 2.2, 1.2, lambda tt, dx, dy: "R" if tt < 0.55 else "B", 50 + s, 0.2, 4)
            continue
        if t > 3 and s % 2 == 0:                                    # streak behind a flying drop
            img.line(x - math.cos(a) * 4, y - math.sin(a) * 3, x, y, "B", 1.5)
        img.disc(x, y, r, r, lambda tt, dx, dy: "P" if (dx + dy) < -0.55 else ("R" if tt < 0.75 else "B"), 60 + s, 0.25, 4)
    return img


def blood_pool(i: int) -> Img:
    """A pool spreading from under the body: 28, 62, 100, 124 px wide, a wet lit rim, a dark core."""
    w = [28, 62, 100, 124][i]
    img = Img(VW, VH)
    ry = max(3.5, w * 0.07)
    cy = 62.0
    def pick(t, dx, dy):
        if t > 0.86:
            return "B"
        if dy < -0.3 and t > 0.45:
            return "P" if t > 0.72 else "R"
        return "B" if t > 0.3 else "m"
    img.disc(VPIV[0], cy, w / 2, ry, pick, 70 + i, 0.14, 7)
    for s in range(i * 2):                                          # satellite drops
        img.disc(VPIV[0] + (s - i + 0.5) * (w / 8), cy + 1 - (s % 2) * 5, 1.5, 1.1, lambda t, dx, dy: "R", 80 + s, 0.2, 4)
    return img


VFX = [  # (tag, name, ticks, fn, i, playback)
    *[("pound_debris", f"pound_debris_{n}", tk, pound_debris, n, "once") for n, tk in enumerate([3, 4, 5, 6, 12])],
    *[("shockwave_ripple", f"shockwave_ripple_{n}", 4, shockwave_ripple, n, "loop") for n in range(4)],
    *[("crash_dust", f"crash_dust_{n}", tk, crash_dust, n, "once") for n, tk in enumerate([3, 3, 4, 5, 7, 12])],
    *[("roar_ring", f"roar_ring_{n}", 3, roar_ring, n, "once") for n in range(4)],
    *[("mirror_crack", f"mirror_crack_{n}", None, mirror_crack, n, "hold") for n in range(3)],
    *[("floor_dust", f"floor_dust_{n}", tk, floor_dust, n, "once") for n, tk in enumerate([3, 4, 6, 8, 14])],
    *[("boil_spatter", f"boil_spatter_{n}", tk, boil_spatter, n, "once") for n, tk in enumerate([3, 3, 6, 10])],
    *[("blood_pool", f"blood_pool_{n}", None if n == 3 else tk, blood_pool, n, "hold") for n, tk in enumerate([8, 10, 14, 0])],
]


# ------------------------------------------------------------------ props
def slab(img: Img, x0: int, y0: int, w: int, seed: int, tilt: int = 0) -> None:
    """One iron weight plate, 7 rows with a shared outline: lit top edge, rust speckle, dark underside."""
    for dx in range(w):
        x = x0 + dx
        off = (dx * tilt) // max(1, w - 1) if tilt else 0
        img.put(x, y0 + off, "K")
        img.put(x, y0 + 1 + off, "k" if dx % 7 != 3 else "j")
        for dy in (2, 3, 4):
            r = h2(x, y0 + dy, seed) % 100
            img.put(x, y0 + dy + off, "f" if r < 7 else ("e" if r < 13 else ("i" if dy == 4 else "j")))
        img.put(x, y0 + 5 + off, "i")
        img.put(x, y0 + 6 + off, "K")
    for dy in range(7):
        img.put(x0, y0 + dy, "K"); img.put(x0 + w - 1, y0 + dy, "K")


def chain(img: Img, pts: list[tuple[float, float]]) -> None:
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for s in range(n + 1):
            x, y = x0 + (x1 - x0) * s / n, y0 + (y1 - y0) * s / n
            img.put(int(round(x)), int(round(y)), "k" if (s // 2) % 2 == 0 else "i")
            img.put(int(round(x)) + 1, int(round(y)), "K")


def timber(img: Img, x0: int, y0: int, x1: int, y1: int, seed: int) -> None:
    """A lit timber box: light left, dark right, grain strands and rot patches (deep brown), K outline."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            u = (x - x0) / max(1, x1 - x0)
            k = "u" if u < 0.25 else ("z" if u < 0.6 else "y")
            r = h2(x, y, seed) % 100
            if r < 6:
                k = "x"
            elif r < 9 and k != "x":
                k = "u" if k == "z" else "z"
            if (x + y // 5 * 3) % 5 == 0 and r < 55:
                k = {"u": "z", "z": "y", "y": "x", "x": "x"}[k]
            img.put(x, y, k)
    for x in range(x0 - 1, x1 + 2):
        img.put(x, y0 - 1, "K"); img.put(x, y1 + 1, "K")
    for y in range(y0 - 1, y1 + 2):
        img.put(x0 - 1, y, "K"); img.put(x1 + 1, y, "K")


RW, RH = 136, 72
RPIV = (48, 71)


def rack(state: str) -> Img:
    """The left plate rack: a stack of five iron plates (80 px wide, 31 px tall), two chains from the ceiling and a rotten
    timber outrigger resting on the strut. Pivot: the underside centre of the stack (the floor-contact row when landed)."""
    img = Img(RW, RH)
    sx, base = 8, 71                                        # stack left edge, underside row (pivot row)
    widths = [80, 76, 80, 74, 80]
    if state == "landed":
        offs = [0, 5, -3, 6, -2]
        tilts = [0, 2, 0, -3, 3]
        y = base - 6
        for n in range(5):
            slab(img, sx + (80 - widths[n]) // 2 + offs[n], y - n * 5, widths[n], 90 + n, tilts[n])
        chain(img, [(30, 38), (36, 41), (44, 38), (52, 42)])           # slack chain heap on top
        chain(img, [(60, 40), (66, 37), (74, 41)])
        timber(img, 82, 47, 120, 53, 7)                               # the broken outrigger lying across the stack
        img.line(120, 54, 128, 62, "K", 2)
        for s in range(5):
            dust(img, 20 + s * 22, 68, 4 + s % 2, 300 + s, fade=0.45, flat=0.55)
        return img
    dx = {"hang": 0, "shudder_a": -1, "shudder_b": 1, "fall": 0}[state]
    dy = {"hang": 0, "shudder_a": 1, "shudder_b": 0, "fall": 0}[state]
    top = base - 30
    for n in range(5):
        slab(img, sx + dx + (80 - widths[n]) // 2, top + dy + n * 6 - 0, widths[n], 90 + n)
    # the iron strap that clamps the outrigger to the stack
    img.rect(sx + dx + 78, top + dy + 1, sx + dx + 85, top + dy + 14, "i")
    img.rect(sx + dx + 78, top + dy + 1, sx + dx + 85, top + dy + 1, "k")
    for yy in (top + dy + 3, top + dy + 12):
        img.put(sx + dx + 81, yy, "w")
    # the outrigger, level when held, dropping at its free end when the strut gives
    if state == "fall":
        for x in range(sx + 86, 133):
            drop = int((x - (sx + 86)) * 0.22)
            for y in range(top + 5 + drop, top + 12 + drop):
                u = (y - top - 5 - drop) / 6.0
                img.put(x, y, "u" if u < 0.3 else ("z" if u < 0.7 else "y"))
            img.put(x, top + 4 + drop, "K"); img.put(x, top + 12 + drop, "K")
    else:
        timber(img, sx + dx + 86, top + dy + 5, 132, top + dy + 11, 5)
    # chains from the ceiling to the stack top (taut; whipping loose when falling)
    for cx0 in (28, 68):
        if state == "fall":
            chain(img, [(cx0 + dx, top + dy), (cx0 + 3, top - 9), (cx0 - 3, top - 17), (cx0 + 4, top - 25), (cx0, top - 33)])
        else:
            chain(img, [(cx0 + dx, top + dy), (cx0 + dx, 0)])
    if state in ("shudder_a", "shudder_b"):                  # dust sifting off the outrigger and the chain mounts
        for s in range(6):
            x, y = 96 + s * 6 + (h2(s, 1, 2) % 3), top + 14 + (s * 5) % 17
            img.put(x, y, "c" if s % 2 else "d"); img.put(x + 1, y, "c")
        for s in range(4):
            img.put(30 + s * 14, 3 + (s * 7) % 11, "c"); img.put(31 + s * 14, 3 + (s * 7) % 11, "c")
    if state == "fall":                                       # speed streaks over the stack
        for s in range(9):
            x = sx + 4 + s * 9
            img.line(x, top - 18 - (s % 3) * 4, x, top - 4 - (s % 2) * 3, "b" if s % 2 else "c", 1)
    return img


SW, SH = 40, 196
SPIV = (20, 195)


def strut(state: int) -> Img:
    """The rotten timber strut: a post with a notched cradle that holds the outrigger, iron straps and rot; three damage
    states: sound, cracked (one hit), splintered (two hits). A third hit breaks it (the splinter burst is a separate kit asset)."""
    img = Img(SW, SH)
    lean = [0, 0, 1][state]
    def post(y0: int, y1: int, shift: int) -> None:
        timber(img, 13 + shift, y0, 26 + shift, y1, 11 + state)
    if state < 2:
        post(10, 192, 0)
    else:
        post(10, 96, 3); post(106, 192, 0)                     # split at mid height, the upper half knocked sideways
        img.rect(12, 98, 29, 104, "K")                           # the dark gap, bridged by long splinters
        for s in range(7):
            x0 = 13 + s * 2
            img.line(x0, 106, x0 + 2 - (s % 2) * 4, 97 - (s % 3) * 2, "u" if s % 2 else "z", 1)
            img.line(x0 + 1, 106, x0 + 4 - (s % 2) * 3, 99, "y", 1)
        for s in range(5):                                       # shards flaring out of the broken faces
            img.line(27, 92 + s * 3, 34 + s % 3, 88 + s * 4, "z", 1)
            img.line(13, 109 + s * 2, 7 - s % 2, 112 + s * 3, "y", 1)
    cap = 10
    timber(img, 6 + lean, 2, 33 + lean, 8, 21)                   # the cradle top
    img.rect(6 + lean, 2, 14 + lean, 5, "K"); img.rect(7 + lean, 3, 13 + lean, 4, ".")    # the notch the beam rests in
    img.rect(7 + lean, 5, 13 + lean, 5, "x")
    for y in (40, 100, 160):                                       # iron straps
        sh = 2 if (state == 2 and y < 100) else 0
        img.rect(12 + sh, y, 27 + sh, y + 4, "i"); img.rect(12 + sh, y, 27 + sh, y, "k"); img.rect(12 + sh, y + 5, 27 + sh, y + 5, "K")
        img.put(14 + sh, y + 2, "w"); img.put(25 + sh, y + 2, "w")
    for x in range(8, 33):                                         # a wedged foot
        for y in range(184 + max(0, 9 - (x - 8) if x < 14 else 0), 195):
            if x < 12 or x > 28:
                img.put(x, y, "y" if x < 12 else "x")
    img.rect(10, 192, 30, 195, "K"); img.rect(11, 192, 29, 194, "y")
    if state >= 1:                                                 # the crack, with a dark core and a lit lip
        pts = [(18, 82), (16, 92), (21, 100), (17, 111), (22, 121)]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            img.line(x0, y0, x1, y1, "K", 1)
            img.line(x0 + 1, y0, x1 + 1, y1, "u", 1)
        img.rect(11, 94, 12, 96, "K"); img.rect(27, 108, 28, 110, "K")
    return img


LW, LH = 64, 40
LPIV = (32, 12)


def ledge(pressed: bool) -> Img:
    """The release-plate ledge for the Donut rack: a masonry slab with iron brackets and a pressure plate (raised or
    pressed flush). Pivot: the walking surface at the slab centre."""
    img = Img(LW, LH)
    for y in range(12, 22):
        for x in range(8, 56):
            r = h2(x, y, 41) % 100
            k = "g" if y == 12 else ("f" if y < 18 else "e")
            if y == 12 and r < 25:
                k = "h"
            if r > 92:
                k = "c" if y < 18 else "a"
            img.put(x, y, k)
    for x in range(7, 57):
        img.put(x, 11, "K"); img.put(x, 22, "K")
    for y in range(11, 23):
        img.put(7, y, "K"); img.put(56, y, "K")
    for x0 in (11, 44):                                            # iron brackets down the wall side
        img.rect(x0, 23, x0 + 7, 37, "i"); img.rect(x0, 23, x0 + 7, 23, "k"); img.rect(x0, 38, x0 + 7, 38, "K")
        img.rect(x0 - 1, 23, x0 - 1, 38, "K"); img.rect(x0 + 8, 23, x0 + 8, 38, "K")
        img.put(x0 + 3, 28, "w"); img.put(x0 + 3, 33, "w")
    h = 2 if pressed else 4
    top = 12 - h
    img.rect(18, top, 45, 11, "j"); img.rect(18, top, 45, top, "k")
    for x in range(19, 45, 4):                                     # hazard hatch (dark stripes)
        for y in range(top + 1, 12):
            img.put(x + (y - top) % 3, y, "i")
    for x in range(17, 47):
        img.put(x, top - 1, "K")
    img.put(17, top, "K"); img.put(46, top, "K")
    for y in range(top, 12):
        img.put(17, y, "K"); img.put(46, y, "K")
    if pressed:
        for x in range(19, 45):
            img.put(x, 12, "i")                                    # the sunk gap under the plate
    return img


# ------------------------------------------------------------------ atlases
ANCHOR = {"pound_debris": (76, 71), "shockwave_ripple": (76, 71), "crash_dust": (76, 71), "mirror_crack": (76, 36),
          "floor_dust": (76, 71), "roar_ring": (76, 57), "boil_spatter": (76, 52), "blood_pool": (76, 62)}
FX_ROLE = {
    "pound_debris": "brute pound impact: debris and dust at the strike point, 50 px ahead of the pivot (the pound box centre)",
    "shockwave_ripple": "P2+ shockwave ripple: 10 px tall, travels 120 px each way at 150 px/s (mirror for the left)",
    "crash_dust": "wall crash: dust and chips at the ram contact point, billowing into the room (mirror for the left wall)",
    "mirror_crack": "cosmetic crack on the wall mirror at the crash height: three states, held",
    "floor_dust": "brute_ko_timber floor impact: 120 px dust burst along the floor both ways",
    "roar_ring": "phase roar push wave: a flat floor ring at radius 18, 36, 54, 72 px (push_radius 72)",
    "boil_spatter": "preset 4 finish: boils burst in red spatters from the body",
    "blood_pool": "preset 4 finish: spreading pool, the last frame stays",
}


def to_grid(w: int, h: int, pivot: tuple[int, int], frames: list[tuple[list[str], int | None]], tags: list[tuple[str, int, int]]) -> dict:
    used = sorted({c for rows, _t in frames for r in rows for c in r if c != "."})
    return {
        "format": "pixel-grid-v1", "canvas": [w, h], "pivot": list(pivot),
        "palette": [{"key": k, "name": COLOURS[k][0], "rgba": COLOURS[k][1] + "ff"} for k in used],
        "layers": ["body"],
        "frames": [{"duration_ms": HOLD_MS if t is None else round(t * 1000 / TICK_HZ), "cels": {"body": rows}} for rows, t in frames],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in tags],
    }


def tags_from(seq: list[str]) -> list[tuple[str, int, int]]:
    out: list[tuple[str, int, int]] = []
    for i, t in enumerate(seq):
        if out and out[-1][0] == t:
            out[-1] = (t, out[-1][1], i)
        else:
            out.append((t, i, i))
    return out


def atlas_vfx() -> dict:
    rows = [(fn(i).rows(), tk) for _t, _n, tk, fn, i, _p in VFX]
    return dict(id="boss-f01-vfx", base="brute_boss_f01_vfx", size=(VW, VH), pivot=VPIV, type="vfx_animation",
                names=[n for _t, n, *_r in VFX], tag_seq=[t for t, *_r in VFX], ticks=[tk for _t, _n, tk, *_r in VFX],
                frames=rows, pivots=[ANCHOR[t] for t, *_r in VFX],
                playback={t: p for t, _n, _tk, _f, _i, p in VFX}, role=FX_ROLE,
                origin="boss encounter VFX and finishes: pound debris, shockwave ripple, crash dust, mirror crack, floor-impact dust, "
                       "roar push ring, boil spatter and blood pool (35 frames), rasterized from rules by scripts/author_boss_f01_fx.py")


def atlas_rack() -> dict:
    states = [("rack_hang", "hang", None), ("rack_shudder", "shudder_a", 3), ("rack_shudder", "shudder_b", 3),
              ("rack_fall", "fall", None), ("rack_landed", "landed", None)]
    return dict(id="boss-f01-rack", base="brute_boss_f01_rack", size=(RW, RH), pivot=RPIV, type="prop_animation",
                names=["rack_hang", "rack_shudder_a", "rack_shudder_b", "rack_fall", "rack_landed"],
                tag_seq=[t for t, _s, _k in states], ticks=[k for _t, _s, k in states],
                frames=[(rack(s).rows(), k) for _t, s, k in states], pivots=[RPIV] * 5,
                playback={"rack_hang": "hold", "rack_shudder": "loop", "rack_fall": "hold", "rack_landed": "hold"},
                role={"rack_hang": "left plate rack held by the strut at floor x 200, underside 189 px up",
                      "rack_shudder": "pre-fall shudder loop with dust, 36 tick fall window",
                      "rack_fall": "falling stack with snapped chains and speed streaks (the game translates it)",
                      "rack_landed": "landed stack, plates staggered (drawn over the pinned boss)"},
                origin="plate rack prop in four states (five frames), left-rack facing; mirror for the disabled right rack")


def atlas_strut() -> dict:
    names = ["strut_sound", "strut_cracked", "strut_splintered"]
    return dict(id="boss-f01-strut", base="brute_boss_f01_strut", size=(SW, SH), pivot=SPIV, type="prop_animation", names=names,
                tag_seq=names, ticks=[None] * 3, frames=[(strut(i).rows(), None) for i in range(3)], pivots=[SPIV] * 3,
                playback={n: "hold" for n in names},
                role={"strut_sound": "no hits", "strut_cracked": "after the first strut hit", "strut_splintered": "after the second hit; the third breaks it"},
                origin="rotten timber strut in three damage states, floor contact at the pivot, extends the CP-03 shore kit")


def atlas_ledge() -> dict:
    names = ["plate_raised", "plate_pressed"]
    return dict(id="boss-f01-ledge", base="brute_boss_f01_ledge", size=(LW, LH), pivot=LPIV, type="prop_animation", names=names,
                tag_seq=names, ticks=[None, None], frames=[(ledge(False).rows(), None), (ledge(True).rows(), None)], pivots=[LPIV] * 2,
                playback={n: "hold" for n in names},
                role={"plate_raised": "ledge and release plate waiting (104 px up at x 432..480)", "plate_pressed": "plate pressed flush"},
                origin="ledge and release plate for the Donut rack (shipped disabled until the higher cat jump lands)")


def atlas_grid(a: dict) -> dict:
    w, h = a["size"]
    return to_grid(w, h, a["pivot"], list(a["frames"]), tags_from(a["tag_seq"]))


def atlas_manifest(a: dict, grid: dict) -> dict:
    from scripts.pixelart.validators import frame_digest
    w, h = a["size"]
    base = a["base"]
    tags = tags_from(a["tag_seq"])
    frames = []
    for i, name in enumerate(a["names"]):
        rows = grid["frames"][i]["cels"]["body"]
        xs = [x for r in rows for x, c in enumerate(r) if c != "."]
        ys = [y for y, r in enumerate(rows) if r.strip(".")]
        frames.append({"index": i, "name": name, "tag": a["tag_seq"][i], "ticks": a["ticks"][i],
                       "duration_ms": grid["frames"][i]["duration_ms"], "pivot": list(a["pivots"][i]),
                       "rect": rect_of(i, w, h, len(a["names"])), "content": [min(xs), min(ys), max(xs), max(ys)],
                       "sha256": frame_digest(grid["frames"][i]["cels"], ["body"])})
    return {
        "schema_version": 1, "id": base + "_candidate_v0", "type": a["type"],
        "source_path": f"assets/source/enemies/brute_boss_f01/{base}.ase",
        "grid_source": f"assets/source/enemies/brute_boss_f01/{base}.grid.json",
        "output_atlas": f"generated/art/{base}.png", "output_metadata": f"generated/art/{base}.json",
        "palette_id": "boss-f01-fx-v0", "frame_size": [w, h], "pivot": list(a["pivot"]),
        "animation_tags": [t for t, _a, _b in tags], "required_alpha": True,
        "owner": "AI-authored Floor 1 boss encounter art (fan-concept layer: THE JUICER encounter; runtime ids IP-neutral brute_boss_f01)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": a["origin"],
            "generator": "designed by Claude Opus keys, executed by Claude Sonnet; no AI concept pixels copied. Drawn by Claude Sonnet "
                         "from the written spec (route B: the top-tier editor pass and silhouette gate are still pending). "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["docs/02-art/floor-01-boss-design.md", "tools/art-references/death-lab/README.md"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "boss": {"status": f"{len(a['names'])} frames drawn; top-tier editor pass and independent review pending; not wired into any Bevy binary",
                 "spec": "docs/02-art/floor-01-boss-design.md", "data_id": "brute_boss_f01", "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS,
                 "playback": a["playback"], "roles": a["role"], "frames": frames},
    }


def rect_of(i: int, w: int, h: int, n: int) -> dict:
    """The atlas rectangle of frame ``i`` in the LibreSprite ``--sheet-type rows`` export (see ``tests/test_art_boss_f01.py``)."""
    cols = sheet_columns(n)
    return {"x": (i % cols) * w, "y": (i // cols) * h, "w": w, "h": h}


def sheet_columns(n: int) -> int:
    return n


def emblem_manifest() -> dict:
    from scripts.pixelart.validators import frame_digest
    grid = json.loads((SRC / "keys/brute_boss_f01_emblem.grid.json").read_text(encoding="utf-8"))
    return {
        "schema_version": 1, "id": "brute_boss_f01_emblem_candidate_v0", "type": "hud_icon",
        "source_path": "assets/source/enemies/brute_boss_f01/brute_boss_f01_emblem.ase",
        "grid_source": "assets/source/enemies/brute_boss_f01/keys/brute_boss_f01_emblem.grid.json",
        "output_atlas": "generated/art/brute_boss_f01_emblem.png", "output_metadata": "generated/art/brute_boss_f01_emblem.json",
        "palette_id": "trog-family-v0+juicer", "frame_size": [18, 18], "pivot": [0, 0], "animation_tags": ["emblem"],
        "required_alpha": True,
        "owner": "AI-authored HUD emblem (fan-concept layer: THE JUICER boss strip, HUD-DEBT-03; runtime id brute_boss_f01_emblem)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "the 18 x 18 keyed emblem from scripts/author_boss_f01_keys.py (head in profile: mop, ear, red-rimmed eye, roaring mouth, brow boil), hand-placed pixels in the JUICE-01 palette",
            "generator": "designed by Claude Opus keys, executed by Claude Sonnet; no AI concept pixels copied. Route C: keyed, copied unchanged. "
                         "Not yet merged into the HUD atlas (palette budget of hud-hybrid-a-v1)",
            "generated_art": True,
            "input_reference_ids": ["docs/02-art/floor-01-boss-design.md"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "boss": {"status": "1 frame; sits in the 20 x 20 boss-strip well at offset (1, 1) over magenta_dk", "spec": "docs/02-art/floor-01-boss-design.md",
                 "data_id": "brute_boss_f01", "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS, "playback": {"emblem": "hold"},
                 "roles": {"emblem": "boss strip emblem, HUD native spec 2.3 and 6, well offset (1, 1)"},
                 "frames": [{"index": 0, "name": "brute_boss_f01_emblem", "tag": "emblem", "ticks": None, "duration_ms": HOLD_MS,
                             "pivot": [0, 0], "rect": {"x": 0, "y": 0, "w": 18, "h": 18}, "content": [0, 0, 17, 17],
                             "sha256": frame_digest(grid["frames"][0]["cels"], ["key"])}]},
    }


ATLASES = [atlas_vfx, atlas_rack, atlas_strut, atlas_ledge]


def targets() -> list[tuple[Path, str]]:
    out = []
    for make in ATLASES:
        a = make()
        grid = atlas_grid(a)
        out.append((SRC / f"{a['base']}.grid.json", json.dumps(grid, indent=1) + "\n"))
        out.append((MAN / f"{a['id']}.json", json.dumps(atlas_manifest(a, grid), indent=2) + "\n"))
    out.append((MAN / "boss-f01-emblem.json", json.dumps(emblem_manifest(), indent=2) + "\n"))
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["build"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    status = 0
    for path, text in targets():
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name} up to date" if same else f"{path.name} differs from the generator")
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    return status


if __name__ == "__main__":
    sys.exit(main())
