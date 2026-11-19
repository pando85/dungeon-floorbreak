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
    """A billowing dust cloud (a cluster of lumpy lobes), four-step ramp lit from the upper left, dust_deep on the rim.
    ``fade`` 0..1 dissolves the outer part of the cloud in clustered 2 x 2 holes and leaves a solid core."""
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
            if not best:
                continue
            rim = max(0.0, (best[0] - 0.5) / 0.5)                          # 0 in the core, 1 on the rim
            if h2(xx // 2, yy // 2, seed + 31) % 100 < fade * 100 * rim:
                continue
            img.put(xx, yy, "a" if best[0] > 0.88 else pick(best[0], best[1], best[2]))


# ------------------------------------------------------------------ VFX atlas (152 x 72, pivots are per frame, see ANCHOR)
VW, VH = 152, 72
VPIV = (76, 71)
RING_Y = 60                                   # roar ring centre row
CRACK_C = (76, 24)                           # mirror crack and wall-crash contact point (hump height)


def pound_debris(i: int) -> Img:
    """Fists hit the floor: one low dust cloud that rolls out and up, brick chunks thrown up and out (every arc stays in the
    frame and comes back down), chips left on the floor."""
    t = [1, 4, 8, 13, 20][i]
    img = Img(VW, VH)
    for s in range(5):                       # a billowing cloud on both sides of the strike point
        side = -1 if s % 2 else 1
        r = 7 + 0.45 * t + (s % 3) * 1.5
        dust(img, VPIV[0] + side * (3 + 1.5 * min(t, 14) + s * 4), 68 - 5 - 0.3 * t - (s % 3) * 2, r, 10 + s, fade=min(0.85, t / 24), flat=0.9)
    for s in range(10):                      # chunks: ballistic arcs from the impact, apex 25 to 40 px up
        vx = (h2(s, 1, 5) % 17 - 8) * 0.35
        vy = 2.2 + (h2(s, 2, 5) % 10) * 0.14
        x, y = VPIV[0] + vx * t + (s % 3 - 1) * 4, 64 - (vy * t - 0.07 * t * t)
        if y >= 67:
            y = 67 - (s % 2) * 2
        chunk(img, x, y, 1.6 + (s % 3) * 0.8, 40 + s)
    return img


def shockwave_ripple(i: int) -> Img:
    """A floor ripple travelling right, at most 11 rows tall (the 10 px hitbox): a soot-outlined crest with a chip_hi lip on
    the leading 12 px, dust_hi in the crest, dust_deep only in the trailing third. Frames 0 and 2 crest at y 60, frames 1 and 3
    at y 62 with the lip curling 2 px forward; the stepped tail slides 3 px toward the crest each frame."""
    img = Img(VW, VH)
    front = 94 + (2 if i % 2 else 0)
    crest = 11 if i % 2 == 0 else 9
    tail = front - 54
    for x in range(tail, front + 1):
        u = (x - tail) / float(front - tail)
        step = (x + 3 * i) // 3 % 2                                # the stepped tail, sliding toward the crest
        height = max(2, min(crest, int(round(crest * (u ** 1.1))) + (step if 0.1 < u < 0.7 else 0)))
        top = 70 - height + 1
        img.put(x, top - 1, "K")
        for y in range(top, 71):
            lip = x >= front - 12 and y <= top + 1
            third = u < 0.34
            k = "h" if lip else ("d" if y <= top + 2 else ("c" if y <= top + 5 else "b"))
            if third:
                k = "a" if y > top + 2 else "b"
            img.put(x, y, k)
    for y in range(70 - crest + 1, 71):                              # the vertical front edge
        img.put(front + 1, y, "K")
    if i % 2:                                                       # the lip curls 2 px forward on the odd frames
        for y in (61, 62, 63):
            img.put(front + 2, y, "h"); img.put(front + 3, y, "K")
        img.put(front + 2, 60, "K")
    for s in range(4):
        chunk(img, front - 10 - s * 11 + (i * 3 + s) % 4, 56 - (s % 2) * 3 + (i + s) % 2 * 2, 1.2 + (s % 2) * 0.6, 70 + s, "efg")
    return img


def crash_dust(i: int) -> Img:
    """The boss meets the wall at hump height (the contact point is x 75, y 22): frames 0 and 1 are chips and a small bright
    dust pop at the contact point, frames 2 to 5 the dust slides down the wall face and settles on the floor."""
    t = [1, 3, 6, 10, 15, 22][i]
    img = Img(VW, VH)
    cx, cy = 75, 22
    if i <= 1:
        r = 3.5 + 2.2 * i
        img.disc(cx - 2, cy, r + 1.5, r + 1.5, lambda tt, dx, dy: "d" if tt < 0.55 else "c", 90 + i, 0.3, 5)
        img.disc(cx - 8 - 4 * i, cy + 2, r, r * 0.8, lambda tt, dx, dy: "c" if tt < 0.6 else "b", 93 + i, 0.3, 5)
    else:
        slide = min(1.0, (t - 3) / 12.0)                            # 0 at the contact point, 1 settled on the floor
        for s in range(6):
            r = 6 + 0.35 * t + (s % 3) * 1.4
            x = 72 - 3 - (s % 3) * 4 - 1.1 * min(t, 11) * (0.3 + (s % 4) * 0.22)
            y = cy + (62 - cy) * min(1.0, slide * (0.6 + 0.12 * (s % 4))) + (s % 2) * 2
            dust(img, x, y, r, 100 + s, fade=min(0.9, t / 28), flat=0.9)
    for s in range(7):
        vx = -(1.0 + (h2(s, 3, 9) % 9) * 0.3)
        vy = 1.0 + (h2(s, 4, 9) % 8) * 0.25
        x, y = cx - 3 + vx * t, cy + 2 - (vy * t - 0.1 * t * t)
        if y > 67:
            y = 67 - (s % 2) * 2
        chunk(img, x, y, 1.5 + (s % 3) * 0.7, 120 + s, "fghh" if s % 2 else "efgh")
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
    """The push wave, radius 18, 36, 54, 72 px: a floor-plane ellipse at most 12 rows tall, a lit outer rim, a dust inner band,
    fading and breaking up as it spreads. Pivot: the ring centre (the boss's feet)."""
    r = [18, 36, 54, 72][i]
    img = Img(VW, VH)
    cy, ry = RING_Y, min(4.5, r * 0.2)
    rim = ("w", "h", "g", "d")[i]
    band = ("d", "g", "d", "c")[i]
    low = ("c", "c", "b", "a")[i]
    for ang in range(0, 1440):
        a = math.radians(ang / 4.0)
        for off, k in ((0.0, rim), (0.8, rim), (1.6, band), (2.4, low)):
            x = VPIV[0] + (r - off * 1.1) * math.cos(a)
            y = cy + (ry - off * 0.5) * math.sin(a)
            if i >= 2 and h2(int(x), int(y), 4 + i) % (4 if i == 2 else 2) == 0:
                continue                                            # the wave breaks up as it spreads
            if math.sin(a) > -0.2 or h2(int(x), int(y), 5) % 3 == 0:    # the far half is fainter
                img.put(int(round(x)), int(round(y)), k)
    return img


def mirror_crack(i: int) -> Img:
    """A star crack on a cosmetic wall mirror, three states (small, spread, shattered), each within 30 x 40 px. Cracks are
    iron_light on a soot edge; flash_white is only a 1 px glint on about one crack pixel in eight. Pivot: the impact point."""
    img = Img(VW, VH)
    cx, cy = CRACK_C
    rays, reach = [(5, 9), (7, 13), (9, 16)][i]
    n = 0
    for s in range(rays):
        a = (s / rays) * math.tau + (h2(s, i, 11) % 100) / 100.0 * 0.5
        x, y = cx, cy
        length = reach * (0.6 + 0.4 * (h2(s, 5, 11) % 100) / 100.0)
        steps = max(2, int(length // 3))
        for st in range(steps):
            a += ((h2(s, st, 13) % 100) / 100.0 - 0.5) * 0.7
            nx, ny = x + 3 * math.cos(a), y + 3 * math.sin(a) * 1.25
            n += 1
            img.line(x + 1, y + 1, nx + 1, ny + 1, "K", 1)
            img.line(x, y, nx, ny, "w" if n % 8 == 0 else "k", 1)
            x, y = nx, ny
    for ring in range(i):                                           # concentric chords between the rays
        r = 5 + ring * 5
        for ang in range(0, 360, 6):
            if h2(ang, ring, 17) % 3:
                a = math.radians(ang)
                img.put(int(cx + r * math.cos(a)), int(cy + r * 1.25 * math.sin(a)), "j")
    if i == 2:                                                      # missing shards: dark holes
        for (sx, sy, sr) in ((cx - 4, cy - 3, 2), (cx + 4, cy + 3, 2), (cx + 1, cy - 8, 1.5)):
            img.disc(sx, sy, sr, sr, lambda t, dx, dy: "K", 30 + int(sx), 0.4, 4)
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


def rack_shadow(i: int) -> Img:
    """The rack-crush floor-shadow tell over the 36-tick fall: a flat ellipse 48 x 4, 64 x 5 and 80 x 6 px (the last is the
    80 px footprint), a solid soot core inset 2 px and a 2 px rim of 2-on 2-off soot dashes. Pivot: bottom centre."""
    w, h = [(48, 4), (64, 5), (80, 6)][i]
    img = Img(VW, VH)

    def half(width: int, height: int, r: int) -> int:
        """Pixel width of row ``r`` of a ``width`` x ``height`` ellipse (a rounded outline that reaches the full width)."""
        f = math.sqrt(max(0.0, 1.0 - ((r + 0.5 - height / 2.0) / (height / 2.0 + 0.5)) ** 2))
        return min(width, int(round(width * f + 1)))

    for r in range(h):
        y = 72 - h + r
        rw = half(w, h, r)
        x0 = VPIV[0] - rw // 2
        cr = r - 1
        core_w = half(w - 4, h - 2, cr) if 0 <= cr < h - 2 else 0
        cx0 = VPIV[0] - core_w // 2
        for x in range(x0, x0 + rw):
            if (cx0 <= x < cx0 + core_w) or ((x - x0) // 2) % 2 == 0 or x >= x0 + rw - 2:
                img.put(x, y, "K")
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
    *[("rack_shadow", f"rack_shadow_{n}", 12, rack_shadow, n, "once") for n in range(3)],
]


# ------------------------------------------------------------------ props
PLATE_W = 64                                                       # one weight plate: 64 x 7 with shared outline rows
PLATE_PITCH = 6                                                    # four plates, 25 rows tall in all


def slab(img: Img, x0: int, y0: int, seed: int, tilt: int = 0) -> None:
    """One iron weight plate 64 x 7: soot outline, a 1 px iron_light top bevel, an iron_dark body, a 1 px iron_mid bottom edge
    that run across a 2 x 3 selector hole at x +31..+32 (middle three rows), a 5 x 3 dust_hi weight tag with two soot digit
    pixels at x +10..+14, and at most six rust pixels."""
    def at(dx: int) -> int:
        return (dx * tilt) // (PLATE_W - 1) if tilt else 0
    for dx in range(PLATE_W):
        for dy, k in enumerate(["K", "k", "i", "i", "i", "j", "K"]):
            img.put(x0 + dx, y0 + dy + at(dx), k)
    for dy in range(7):
        img.put(x0, y0 + dy, "K"); img.put(x0 + PLATE_W - 1, y0 + dy + tilt, "K")
    for dx in (31, 32):                                              # the selector hole, middle three rows only
        for dy in (2, 3, 4):
            img.put(x0 + dx, y0 + dy + at(dx), "K")
    for n in range(6):                                              # rust: six pixels, no more
        x, y = 4 + (h2(n, seed, 3) % 56), 2 + (h2(n, seed, 4) % 3)
        if not (9 <= x <= 15) and x not in (31, 32):
            img.put(x0 + x, y0 + y + at(x), "f")
    for dx in range(10, 15):                                        # the weight tag: bright, two dark digit pixels
        for dy in (2, 3, 4):
            img.put(x0 + dx, y0 + dy + at(dx), "d")
    img.put(x0 + 11, y0 + 3 + at(11), "K"); img.put(x0 + 13, y0 + 3 + at(13), "K")


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
STACK_X = 16                                                       # stack left edge (64 wide, centred on the pivot column)
BEAM_UNDER = 65                                                    # the outrigger's underside row = the strut cap (195 px up)


def stack(img: Img, base: int, dx: int = 0, dy: int = 0, offs=(0, 0, 0, 0), tilts=(0, 0, 0, 0), with_pin: bool = True) -> int:
    """Four plates, flush unless ``offs``/``tilts`` stagger them (landed), two iron_mid guide rods 2 px wide at plate x +6 and
    +58 through the whole stack and a 10 x 4 headplate with the cable eye on top. ``base`` is the underside row. Returns the top."""
    top = base - (PLATE_PITCH * 4)
    for n in range(4):
        slab(img, STACK_X + dx + offs[n], top + dy + n * PLATE_PITCH, 90 + n, tilts[n])
    if not any(offs):
        for rx in (STACK_X + dx + 6, STACK_X + dx + 58):
            for y in range(top + dy - 2, base + dy):
                if img.get(rx, y) in "ijk":
                    continue
            for y in range(top + dy + 1, base + dy):
                if y % PLATE_PITCH != (top + dy) % PLATE_PITCH:
                    img.put(rx, y, "j"); img.put(rx + 1, y, "j")
    if with_pin:                                                    # selector pin in plate 3 with a bright knob
        py = top + dy + 2 * PLATE_PITCH + 3
        kx = STACK_X + dx + 37
        img.rect(kx, py - 2, kx + 4, py + 2, "K"); img.rect(kx + 1, py - 1, kx + 3, py + 1, "d")
        for x in range(STACK_X + dx + 33, STACK_X + dx + 38):          # the pin runs into the knob's rim
            img.put(x, py, "k")
    hx = STACK_X + dx + 27
    img.rect(hx, top + dy - 4, hx + 9, top + dy - 1, "i"); img.rect(hx, top + dy - 4, hx + 9, top + dy - 4, "k")
    for x in range(hx - 1, hx + 11):
        img.put(x, top + dy - 5, "K")
    img.put(hx + 4, top + dy - 3, "K"); img.put(hx + 5, top + dy - 3, "K")                  # the cable eye
    return top


PULLEY = (48, 38)                                                  # pulley centre in the rack frame (row 36 = the 224 px ceiling)
TIE = (124, 60)                                                    # down-run end: the cleat on the strut head (strut x 200 = frame x 125)


def cable_down(img: Img) -> None:
    """The cable's down-run: a 1 px soot line from the pulley's right tangent to the tie-off, iron_light on the pixel above
    each step, ending in a 6 x 4 iron_mid cleat (soot rim, two diagonal wrap pixels) on the strut cap (row 65)."""
    x0, y0 = 51, 38
    x1, y1 = TIE
    for x in range(x0, x1 + 1):
        y = y0 + int(round((y1 - y0) * (x - x0) / (x1 - x0)))
        img.put(x, y, "K")
        if img.get(x, y - 1) == ".":
            img.put(x, y - 1, "k")
    img.rect(122, 61, 127, 64, "K"); img.rect(123, 62, 126, 63, "j")
    img.put(123, 62, "i"); img.put(124, 63, "i")


def cable_up(img: Img, dx: int = 0, dy: int = 0) -> None:
    """The 2 px cable from the headplate eye up into the pulley (iron_light / soot, the chain pattern)."""
    for y in range(39, 42):
        img.put(47 + dx, y + dy, "k" if (y // 2) % 2 == 0 else "i"); img.put(48 + dx, y + dy, "K")


def rack(state: str) -> Img:
    """The left plate rack: a selectorised stack of four iron plates (64 x 25) with guide rods, a selector pin, weight tags and
    a headplate, hung from ONE cable that runs up over a ceiling pulley (the ``pulley`` frame) and down to a cleat on the strut
    head, so breaking the strut lets the cable run free. Pivot: the underside centre of the stack. ``landed`` is the floor
    version (plates staggered), ``landed_boss`` the stack lying on the pinned boss's back, its underside 26 px up."""
    img = Img(RW, RH)
    base = 71
    if state == "pulley":
        cx, cy = PULLEY
        img.rect(cx - 1, cy - 4, cx + 1, cy - 3, "K")               # the 3 x 2 ceiling bracket, rows 34..35
        for y in range(cy - 3, cy + 4):
            for x in range(cx - 3, cx + 4):
                d = math.hypot(x - cx, y - cy)
                if d <= 3.6:
                    img.put(x, y, "K" if d > 2.6 else "j")
        img.put(cx - 1, cy - 1, "k"); img.put(cx, cy, "K")          # the highlight and the axle
        return img
    if state in ("landed", "landed_boss"):
        up = 26 if state == "landed_boss" else 0
        offs, tilts = (0, 4, -3, 5), (0, 2, 0, -3)
        top = base - up - PLATE_PITCH * 4
        for n in range(4):
            slab(img, STACK_X + 2 + offs[n], top + n * PLATE_PITCH, 90 + n, tilts[n])
        chain(img, [(30, top - 2), (36, top), (42, top - 3), (48, top - 1)])                # one slack cable coil on the top plate
        for s in range(5 if up == 0 else 0):                         # dust settling around the floor version only
            dust(img, 20 + s * 22, 66, 4 + s % 2, 300 + s, fade=0.45, flat=0.55)
        return img
    dx = {"hang": 0, "shudder_a": -1, "shudder_b": 1, "fall": 0}[state]
    dy = {"hang": 0, "shudder_a": 1, "shudder_b": 0, "fall": 0}[state]
    top = stack(img, base, dx, dy)
    if state == "fall":                                             # the cable end runs free: a loose S rising from the headplate
        for (x0, y0), (x1, y1) in zip([(48, 41), (47, 36), (50, 32)], [(47, 36), (50, 32), (48, 28)]):
            n = max(abs(x1 - x0), abs(y1 - y0)) + 1
            for t in range(n + 1):
                x, y = x0 + (x1 - x0) * t / n, y0 + (y1 - y0) * t / n
                img.put(int(round(x)), int(round(y)), "k" if (t // 2) % 2 == 0 else "i")
                img.put(int(round(x)) + 1, int(round(y)), "K")
    else:
        cable_up(img, dx, dy)
        cable_down(img)
    if state in ("shudder_a", "shudder_b"):                         # dust sifting off the cleat and the stack
        for s in range(6):
            x, y = 100 + s * 4 + (h2(s, 1, 2) % 3), 66 + (s * 5) % 6
            img.put(x, y, "c" if s % 2 else "d"); img.put(x + 1, y, "c")
    if state == "fall":                                             # four broken speed streaks above the plate ends
        for s, x in enumerate((STACK_X + 2, STACK_X + 20, STACK_X + 43, STACK_X + 61)):
            y0 = 37 + (s % 2)                                        # between the ceiling row and the plate tops
            img.line(x, y0, x, y0 + 3, "d", 1); img.line(x, y0 + 6, x, y0 + 8, "d", 1)
    return img


SW, SH = 40, 196
SPIV = (20, 195)
NECK = (160, 185)                                                  # the strike band: 10 to 35 px up (fist and kick height)


def shaft(img: Img, y0: int, y1: int, shift: int, seed: int, state: int) -> None:
    """The post, row by row: 14 px wide, a rotten neck 2 to 3 px narrower with bites through the strike band, a warm splayed
    base; timber_deep on the shadow edge, chip_hi torn fibres on the lit edge, no iron bands."""
    for y in range(y0, y1 + 1):
        l, r = 13, 26
        if NECK[0] <= y <= NECK[1]:
            l, r = 14, 25
            if h2(y // 3, seed, 5) % 3 == 0:
                l += 1
            if h2(y // 3, seed, 6) % 3 == 0:
                r -= 1
        if y > 185:                                                    # soft splayed base
            l -= (y - 185) // 3
            r += (y - 185) // 3
        for x in range(l + shift, r + shift + 1):
            u = (x - l - shift) / max(1, r - l)
            k = "u" if u < 0.25 else ("z" if u < 0.6 else "y")
            q = h2(x, y, seed) % 100
            if q < 6 or (x == r + shift and q < 40):
                k = "x"                                                 # rot and a deep rim on the shadow edge
            elif x == l + shift and q < 25 and y >= 137:
                k = "h"                                                 # torn lit fibres on the lit edge, lower 30 %
            img.put(x, y, k)
        img.put(l + shift - 1, y, "K"); img.put(r + shift + 1, y, "K")


def strut(state: int) -> Img:
    """The rotten timber strut: a post under a notched cradle that holds the outrigger, one iron band under the cap, a rotten
    neck and damage in the 10 to 35 px strike band. States: sound, cracked (one hit), splintered (two hits: the upper shaft
    bowed 2 px, the head slipped 1 px, jagged fibres). A third hit breaks it (the splinter burst is a kit asset)."""
    img = Img(SW, SH)
    lean = 1 if state == 2 else 0
    if state < 2:
        shaft(img, 15, 194, 0, 11 + state, state)
    else:
        shaft(img, 15, 166, 2, 11, state)                              # upper shaft, bowed 2 px
        shaft(img, 176, 194, 0, 12, state)
        # jagged diagonal fibres, long to short, chip_hi on the lit edge and timber_deep below
        for n, (x, ln) in enumerate(((14, 11), (18, 8), (22, 10), (25, 6))):
            img.line(x + 2, 166, x - 1 - n % 2, 166 + ln, "h", 1)
            img.line(x + 3, 166, x - n % 2, 167 + ln, "x", 1)
        for n, (x, ln) in enumerate(((15, 7), (20, 9), (24, 5))):
            img.line(x, 176, x + 2, 176 - ln, "h", 1)
            img.line(x + 1, 176, x + 3, 177 - ln, "x", 1)
        img.rect(15, 168, 24, 174, "K")                                # the dark break between the halves
    for x in range(8, 34):                                              # the soot underside of the splayed base, on the pivot row
        if img.get(x, 194) != ".":
            img.put(x, 195, "K")
    timber(img, 6 + lean, 2, 33 + lean, 8, 21)                          # the cradle top
    img.rect(6 + lean, 2, 14 + lean, 5, "K"); img.rect(7 + lean, 3, 13 + lean, 4, ".")    # the notch the beam rests in
    img.rect(7 + lean, 5, 13 + lean, 5, "x")
    img.rect(12 + lean, 10, 27 + lean, 14, "i"); img.rect(12 + lean, 10, 27 + lean, 10, "k"); img.rect(12 + lean, 15, 27 + lean, 15, "K")
    img.put(14 + lean, 12, "w"); img.put(25 + lean, 12, "w")
    if state >= 1:                                                     # the crack in the strike band: dark core, lit lip
        pts = [(18, 158), (16, 164), (21, 171), (17, 178), (21, 184)] if state == 1 else [(18, 155), (16, 160)]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            img.line(x0, y0, x1, y1, "K", 1)
            img.line(x0 + 1, y0, x1 + 1, y1, "h", 1)
        if state == 1:
            img.rect(14, 166, 15, 168, "K"); img.rect(24, 175, 25, 177, "K")
    return img


LW, LH = 64, 40
LPIV = (32, 12)


def ledge(pressed: bool) -> Img:
    """The release-plate ledge for the Donut rack: a masonry slab on two triangular iron knee braces that end in wall plates,
    and a pressure plate (raised or pressed flush). Pivot: the walking surface at the slab centre."""
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
    for side in (0, 1):                                              # knee braces: a wall plate and a diagonal strut to the slab
        wx = 8 if side == 0 else 52
        step = 1 if side == 0 else -1
        img.rect(wx, 23, wx + 3, 38, "i"); img.rect(wx, 23, wx, 38, "k"); img.rect(wx + 4, 23, wx + 4, 38, "K"); img.rect(wx - 1, 23, wx - 1, 38, "K")
        img.rect(wx - 1, 39, wx + 4, 39, "K")
        for n in range(15):                                          # the diagonal: from the plate foot up to the slab underside
            x = wx + (2 + n) * step if side == 0 else wx + 1 - n
            y = 36 - n
            for t in range(3):
                img.put(x, y - t, "j" if t == 0 else "i")
            img.put(x, y + 1, "K"); img.put(x, y - 3, "K")
        img.put(wx + 1, 28, "w"); img.put(wx + 1, 34, "w")
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


ROUTE_B = "Route B draft: drawn from the written spec; the top-tier narrow re-gate is pending."
ROUTE_C = "Route C, extends the CP-03 shore kit; the top-tier editor pass is pending."


# ------------------------------------------------------------------ atlases
ANCHOR = {"pound_debris": (76, 71), "shockwave_ripple": (76, 71), "crash_dust": (76, 71), "mirror_crack": CRACK_C,
          "floor_dust": (76, 71), "roar_ring": (76, RING_Y), "boil_spatter": (76, 52), "blood_pool": (76, 62), "rack_shadow": (76, 71)}
FX_ROLE = {
    "pound_debris": "brute pound impact: debris and dust at the strike point, 50 px ahead of the pivot (the pound box centre)",
    "shockwave_ripple": "P2+ shockwave ripple: 10 px tall, travels 120 px each way at 150 px/s (mirror for the left)",
    "crash_dust": "wall crash: dust and chips at the ram contact point, billowing into the room (mirror for the left wall)",
    "mirror_crack": "cosmetic crack on the wall mirror at the crash height (pivot = impact point, within 30 x 40 px): three states, held",
    "floor_dust": "brute_ko_timber floor impact: 120 px dust burst along the floor both ways",
    "roar_ring": "phase roar push wave: a flat floor ring at radius 18, 36, 54, 72 px (push_radius 72)",
    "boil_spatter": "preset 4 finish: boils burst in red spatters from the body; the origin (pivot) is 19 px above the floor contact row (floor_offset_px 19)",
    "blood_pool": "preset 4 finish: spreading pool, the last frame stays",
    "rack_shadow": "rack-crush tell (V-07): floor shadow 48, 64 and 80 px wide, ticks 0..11, 12..23, 24..35 after RackDropped, removed on RackLanded; place at the footprint centre; draw over the floor, under heroes, boss and rack; cosmetic, not hashed",
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
    return dict(route=ROUTE_B, id="boss-f01-vfx", base="brute_boss_f01_vfx", size=(VW, VH), pivot=VPIV, type="vfx_animation",
                names=[n for _t, n, *_r in VFX], tag_seq=[t for t, *_r in VFX], ticks=[tk for _t, _n, tk, *_r in VFX],
                frames=rows, pivots=[ANCHOR[t] for t, *_r in VFX],
                playback={t: p for t, _n, _tk, _f, _i, p in VFX}, role=FX_ROLE,
                origin="boss encounter VFX and finishes: pound debris, shockwave ripple, crash dust, mirror crack, floor-impact dust, "
                       "roar push ring, boil spatter and blood pool (38 frames), rasterized from rules by scripts/author_boss_f01_fx.py")


def atlas_rack() -> dict:
    states = [("rack_hang", "hang", None), ("rack_shudder", "shudder_a", 3), ("rack_shudder", "shudder_b", 3),
              ("rack_fall", "fall", None), ("rack_landed", "landed", None), ("rack_landed_boss", "landed_boss", None),
              ("rack_pulley", "pulley", None)]
    return dict(route=ROUTE_B, id="boss-f01-rack", base="brute_boss_f01_rack", size=(RW, RH), pivot=RPIV, type="prop_animation",
                names=["rack_hang", "rack_shudder_a", "rack_shudder_b", "rack_fall", "rack_landed", "rack_landed_boss", "rack_pulley"],
                tag_seq=[t for t, _s, _k in states], ticks=[k for _t, _s, k in states],
                frames=[(rack(s).rows(), k) for _t, s, k in states], pivots=[RPIV] * 7,
                playback={"rack_hang": "hold", "rack_shudder": "loop", "rack_fall": "hold", "rack_landed": "hold", "rack_landed_boss": "hold", "rack_pulley": "hold"},
                role={"rack_hang": "left plate rack on one cable over the ceiling pulley, tied off on the strut head at floor x 200, underside 189 px up",
                      "rack_shudder": "pre-fall shudder loop with dust, 36 tick fall window",
                      "rack_fall": "falling stack, the cable end running free, speed streaks (the game translates it)",
                      "rack_landed": "landed stack on the floor, plates staggered (the rack missed: nobody in the footprint)",
                      "rack_landed_boss": "landed stack lying on the pinned boss's back, underside 26 px above the pivot row (landed_on_boss_offset_px 26); draw it over pinned_hold or pinned_dead",
                      "rack_pulley": "static ceiling pulley and bracket, drawn at the hang position for the whole fight and never translated (rack.cable_mechanism single_cable_pulley_strut_tieoff)"},
                origin="plate rack prop in four states, the on-boss landing and the ceiling pulley (seven frames), left-rack facing; mirror for the disabled right rack")


def atlas_strut() -> dict:
    names = ["strut_sound", "strut_cracked", "strut_splintered"]
    return dict(route=ROUTE_C, id="boss-f01-strut", base="brute_boss_f01_strut", size=(SW, SH), pivot=SPIV, type="prop_animation", names=names,
                tag_seq=names, ticks=[None] * 3, frames=[(strut(i).rows(), None) for i in range(3)], pivots=[SPIV] * 3,
                playback={n: "hold" for n in names},
                role={"strut_sound": "no hits", "strut_cracked": "after the first strut hit", "strut_splintered": "after the second hit; the third breaks it"},
                origin="rotten timber strut in three damage states, floor contact at the pivot, extends the CP-03 shore kit")


def atlas_ledge() -> dict:
    names = ["plate_raised", "plate_pressed"]
    return dict(route=ROUTE_C, id="boss-f01-ledge", base="brute_boss_f01_ledge", size=(LW, LH), pivot=LPIV, type="prop_animation", names=names,
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
            "generator": "designed by Claude Opus keys, executed by Claude Sonnet; no AI concept pixels copied. " + a["route"] +
                         " Review fix round 1 applied by Claude Sonnet. No image-model raster output, no tracing, no resampling",
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
