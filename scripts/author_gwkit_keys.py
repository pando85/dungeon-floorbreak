#!/usr/bin/env python3
"""AREA-GOBLINWORKSHOP-01: author the key frames of the Goblin Workshop prop kit.

Writes one ``pixel-grid-v1`` source per prop under ``assets/source/props/floor01/``
(``gw_*.grid.json``). Every key is drawn from explicit primitives (rectangles,
ellipses, lines, column shading) in the named Floor 01 kit palette
(``assets/source/terrain/floor01/floor01_catacomb_kit.json``); no concept pixel is
cropped, traced or resampled. The design and the frozen-key list are in
``docs/02-art/goblin-workshop-prop-kit.md``.

The script is deterministic (no randomness, no timestamps). The grids it writes
are the source of truth from here on: the execution step adds in-between and
state frames to the grids (by hand or by extending this script) and must keep the
key frames byte-identical once they are frozen.

Usage: ``python3 scripts/author_gwkit_keys.py [--out DIR]``
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json"
OUT = ROOT / "assets/source/props/floor01"

KEYS = "#$%&'()*+,-/0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~"


def kit_palette() -> dict[str, str]:
    data = json.loads(KIT.read_text())
    return {p["name"]: p["rgba"] for p in data["palette"]}


class Canvas:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.p: list[list[str | None]] = [[None] * w for _ in range(h)]

    def copy(self) -> "Canvas":
        c = Canvas(self.w, self.h)
        c.p = [row[:] for row in self.p]
        return c

    def set(self, x: int, y: int, c: str | None) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            self.p[y][x] = c

    def get(self, x: int, y: int) -> str | None:
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.p[y][x]
        return None

    def rect(self, x0: int, y0: int, x1: int, y1: int, c: str | None) -> None:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def hline(self, x0: int, x1: int, y: int, c: str) -> None:
        self.rect(x0, y, x1, y, c)

    def vline(self, x: int, y0: int, y1: int, c: str) -> None:
        self.rect(x, y0, x, y1, c)

    def ellipse(self, cx: float, cy: float, rx: float, ry: float, c: str, only=None) -> None:
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                dx = (x + 0.5 - cx) / rx
                dy = (y + 0.5 - cy) / ry
                if dx * dx + dy * dy <= 1.0 and (only is None or only(x, y)):
                    self.set(x, y, c)

    def line(self, x0: int, y0: int, x1: int, y1: int, c: str, w: int = 1) -> None:
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            x = round(x0 + (x1 - x0) * i / n)
            y = round(y0 + (y1 - y0) * i / n)
            for k in range(w):
                if abs(x1 - x0) >= abs(y1 - y0):
                    self.set(x, y + k, c)
                else:
                    self.set(x + k, y, c)

    def outline(self, c: str, keep: tuple[str, ...] = ()) -> None:
        """Turn every opaque pixel on the silhouette edge (4-neighbour) into ``c``."""
        edge = []
        for y in range(self.h):
            for x in range(self.w):
                v = self.p[y][x]
                if v is None or v in keep:
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if self.get(x + dx, y + dy) is None:
                        edge.append((x, y))
                        break
        for x, y in edge:
            self.p[y][x] = c

    def recolor(self, x0, y0, x1, y1, mapping: dict[str, str]) -> None:
        for y in range(max(0, y0), min(self.h - 1, y1) + 1):
            for x in range(max(0, x0), min(self.w - 1, x1) + 1):
                v = self.p[y][x]
                if v in mapping:
                    self.p[y][x] = mapping[v]


def band(u: float, stops: list[tuple[float, str]]) -> str:
    """Column shading: the first stop whose upper bound exceeds ``u``."""
    for edge, c in stops:
        if u < edge:
            return c
    return stops[-1][1]


def cylinder(cv: Canvas, x0: int, x1: int, y0: int, y1: int, stops, only=None) -> None:
    for x in range(x0, x1 + 1):
        u = (x - x0 + 0.5) / (x1 - x0 + 1)
        c = band(u, stops)
        for y in range(y0, y1 + 1):
            if only is None or only(x, y):
                cv.set(x, y, c)


def pipe_h(cv: Canvas, x0: int, x1: int, y0: int, y1: int, ramp: list[str]) -> None:
    """A horizontal pipe, lit from above: rows from highlight to shadow."""
    h = y1 - y0 + 1
    for y in range(y0, y1 + 1):
        v = (y - y0 + 0.5) / h
        if v < 0.2:
            c = ramp[3]
        elif v < 0.45:
            c = ramp[2]
        elif v < 0.8:
            c = ramp[1]
        else:
            c = ramp[0]
        cv.hline(x0, x1, y, c)


def hazard_stripes(cv: Canvas, x0, y0, x1, y1, period=4, light="brass_05", dark="iron_00", slope=1):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            k = (x + slope * y) // (period // 2)
            cv.set(x, y, light if k % 2 == 0 else dark)


# ----------------------------------------------------------------- ramps ----
# The boiler is a background landmark: its copper sits one step darker than the
# interactive props, with copper_05 kept for the single specular streak.
COPPER = [(0.06, "copper_00"), (0.14, "copper_01"), (0.24, "copper_02"), (0.33, "copper_03"),
          (0.40, "copper_04"), (0.52, "copper_03"), (0.72, "copper_02"), (0.90, "copper_01"),
          (1.01, "copper_00")]
IRON = [(0.08, "iron_01"), (0.22, "iron_02"), (0.34, "iron_04"), (0.40, "iron_05"),
        (0.55, "iron_03"), (0.85, "iron_02"), (1.01, "iron_01")]
STEEL = [(0.10, "stone_cold_00"), (0.25, "stone_cold_01"), (0.36, "stone_cold_03"),
         (0.44, "stone_cold_04"), (0.60, "stone_cold_02"), (0.85, "stone_cold_01"),
         (1.01, "stone_cold_00")]
RED = [(0.10, "brick_04"), (0.26, "flame_00"), (0.36, "flame_01"), (0.42, "flame_02"),
       (0.62, "flame_01"), (0.86, "flame_00"), (1.01, "brick_04")]
WOOD = [(0.15, "wood_02"), (0.35, "wood_04"), (0.50, "wood_05"), (0.80, "wood_03"), (1.01, "wood_02")]
STEEL_ROWS = ["stone_cold_00", "stone_cold_01", "stone_cold_02", "stone_cold_03"]
COPPER_ROWS = ["copper_01", "copper_02", "copper_03", "copper_05"]

OUT_C = "iron_00"


# ================================================================ BOILER ====
BW, BH = 192, 208


def boiler_intact() -> Canvas:
    cv = Canvas(BW, BH)
    # Chimney stack (behind the dome top): iron pipe with a flared cap.
    cylinder(cv, 118, 131, 3, 30, IRON)
    cv.rect(115, 0, 134, 3, "iron_03")
    cv.hline(115, 134, 0, "iron_05")
    cv.hline(115, 134, 3, "iron_01")
    cv.rect(118, 10, 131, 11, "iron_01")  # stack joint
    cv.recolor(118, 4, 131, 9, {"iron_02": "mortar_02", "iron_03": "mortar_03"})  # soot under the cap
    # Steam dome with the safety-valve lever.
    cylinder(cv, 52, 75, 14, 34, COPPER)
    cv.ellipse(63.5, 15, 12, 5, "copper_04", only=lambda x, y: y < 15)
    cv.ellipse(60, 13, 5, 2, "copper_05", only=lambda x, y: y < 15)
    cv.set(58, 12, "copper_06")
    cv.hline(52, 75, 19, "copper_01")
    for x in range(54, 75, 4):
        cv.set(x, 21, "brass_03")
    cv.rect(61, 6, 66, 12, "iron_03")  # valve tower
    cv.vline(61, 6, 12, "iron_04")
    cv.rect(40, 7, 90, 8, "iron_02")  # lever arm
    cv.hline(40, 90, 7, "iron_04")
    cv.ellipse(88, 9, 4, 4, "iron_03")  # ball weight
    cv.set(87, 7, "iron_05")
    cv.set(86, 8, "iron_05")
    # Goblin bodge: a brick wired under the lever to keep the valve shut.
    cv.vline(80, 9, 12, "iron_04")
    cv.rect(77, 13, 83, 16, "brick_06")
    cv.hline(77, 83, 13, "brick_07")
    cv.hline(77, 83, 16, "brick_04")
    cv.set(80, 14, "brick_05")
    cv.vline(40, 9, 13, "iron_03")  # lever pivot post
    cv.vline(41, 9, 13, "iron_02")
    # Dome cap (upper half ellipse) and top rim band.
    cylinder(cv, 20, 171, 24, 45, COPPER,
             only=lambda x, y: ((x + 0.5 - 96) / 76) ** 2 + ((y + 0.5 - 46) / 22) ** 2 <= 1.0)
    cv.ellipse(70, 33, 26, 6, "copper_04",
               only=lambda x, y: ((x + 0.5 - 96) / 74) ** 2 + ((y + 0.5 - 46) / 20) ** 2 <= 1.0 and y < 34)
    cv.ellipse(66, 31, 12, 3, "copper_05",
               only=lambda x, y: ((x + 0.5 - 96) / 74) ** 2 + ((y + 0.5 - 46) / 20) ** 2 <= 1.0)
    # Shell.
    cylinder(cv, 20, 171, 44, 167, COPPER)
    cylinder(cv, 20, 171, 44, 47, IRON)  # top rim band
    # Specular streak on the lit side.
    cv.vline(68, 50, 160, "copper_05")
    cv.vline(69, 56, 140, "copper_04")
    cv.rect(68, 60, 68, 64, "copper_06")
    # Plate seams with rivet rows.
    for sy in (78, 110, 142):
        cv.hline(21, 170, sy, "copper_00")
        for x in range(23, 170, 6):
            lit = x < 110
            cv.set(x, sy + 2, "brass_04" if lit else "brass_02")
            cv.set(x, sy - 2, "brass_03" if lit else "brass_02")
    for x in range(23, 170, 6):
        cv.set(x, 50, "brass_04" if x < 110 else "brass_02")
        cv.set(x, 161, "brass_03" if x < 110 else "brass_02")
    cv.vline(140, 48, 163, "copper_00")  # vertical plate seam
    for y in range(52, 162, 6):
        cv.set(142, y, "brass_02")
    # Iron hoop bands.
    for by in (90, 126):
        cylinder(cv, 20, 171, by, by + 5, IRON)
        cv.hline(20, 171, by, "iron_05")
        cv.hline(20, 171, by + 5, "iron_01")
        for x in (34, 98, 160):  # band bolts
            cv.rect(x, by + 2, x + 1, by + 3, "iron_06")
    # Bodged patch plate (lighter copper, crooked rivets, a lifted corner).
    cv.rect(112, 100, 135, 121, "copper_03")
    cv.hline(112, 135, 100, "copper_04")
    cv.vline(112, 100, 121, "copper_04")
    cv.hline(112, 135, 121, "copper_01")
    cv.vline(135, 100, 121, "copper_01")
    for x, y in ((115, 103), (121, 102), (128, 104), (133, 103), (114, 111), (133, 112),
                 (116, 119), (124, 118), (131, 119)):
        cv.set(x, y, "brass_04")
    cv.rect(133, 100, 135, 102, "copper_05")
    cv.set(135, 100, "copper_01")
    # Sight glass (cool accent: water level).
    cv.rect(150, 56, 154, 88, "iron_02")
    cv.vline(150, 56, 88, "iron_04")
    cv.rect(151, 59, 153, 85, "gal_08")
    cv.rect(151, 59, 153, 70, "gal_04")
    cv.hline(151, 153, 71, "gal_10")
    cv.vline(151, 72, 85, "gal_09")
    cv.rect(149, 55, 155, 57, "iron_04")
    cv.rect(149, 87, 155, 89, "iron_03")
    # Pressure gauge (brass rim, cream face, red zone, needle).
    gx, gy = 48, 66
    cv.rect(gx - 1, gy + 8, gx + 2, gy + 14, "iron_03")  # neck into the shell
    cv.vline(gx - 1, gy + 8, gy + 14, "iron_04")
    cv.ellipse(gx + 0.5, gy + 0.5, 10, 10, "iron_00")
    cv.ellipse(gx + 0.5, gy + 0.5, 9, 9, "brass_03")
    cv.ellipse(gx - 0.5, gy - 0.5, 8, 8, "brass_04", only=lambda x, y: x + y < 2 * gx - 2)
    cv.ellipse(gx + 0.5, gy + 0.5, 7, 7, "stone_10")
    cv.ellipse(gx + 1.5, gy + 1.5, 6, 6, "stone_09", only=lambda x, y: x + y > 2 * gx + 4)
    for ang in range(-30, 91, 15):  # red zone arc, upper right
        a = math.radians(ang)
        cv.set(round(gx + 0.5 + 6 * math.cos(a) - 0.5), round(gy + 0.5 - 6 * math.sin(a) - 0.5), "flame_01")
    for ang in (150, 210, 270):  # tick marks
        a = math.radians(ang)
        cv.set(round(gx + 6 * math.cos(a)), round(gy - 6 * math.sin(a)), "iron_02")
    cv.line(gx, gy, gx + 4, gy - 3, "iron_00")  # needle, high but not yet red
    cv.rect(gx, gy, gx + 1, gy + 1, "iron_00")
    # Base / firebox plinth.
    cylinder(cv, 12, 179, 168, 203, IRON)
    cv.hline(12, 179, 168, "iron_05")
    cv.hline(12, 179, 169, "iron_04")
    cv.hline(12, 179, 171, "iron_01")
    cv.rect(10, 204, 181, 207, "iron_01")
    cv.hline(10, 181, 204, "iron_03")
    for x in range(16, 178, 8):
        cv.set(x, 173, "iron_06" if x < 100 else "iron_04")
        cv.set(x, 201, "iron_04" if x < 100 else "iron_03")
    for x in (40, 152):  # plinth corner straps
        cv.rect(x, 172, x + 3, 202, "iron_03")
        cv.vline(x, 172, 202, "iron_04")
    # Firebox door: arched barred opening with the glow behind it.
    fx0, fx1, fy0, fy1 = 70, 121, 175, 200
    cv.rect(fx0, fy0 + 4, fx1, fy1, "iron_04")
    cv.ellipse((fx0 + fx1 + 1) / 2, fy0 + 6, (fx1 - fx0 + 1) / 2, 6, "iron_04")
    cv.rect(fx0 + 3, fy0 + 6, fx1 - 3, fy1 - 2, "flame_00")
    cv.ellipse((fx0 + fx1 + 1) / 2, fy0 + 7, (fx1 - fx0 - 5) / 2, 4, "flame_00")
    cv.rect(fx0 + 6, fy0 + 8, fx1 - 6, fy1 - 4, "flame_01")
    cv.rect(fx0 + 12, fy0 + 11, fx1 - 12, fy1 - 5, "flame_02")
    cv.rect(fx0 + 18, fy0 + 14, fx1 - 18, fy1 - 6, "flame_03")
    cv.rect(fx0 + 22, fy0 + 17, fx1 - 22, fy1 - 7, "flame_04")
    for x in range(fx0 + 6, fx1 - 3, 5):  # grate bars
        cv.vline(x, fy0 + 5, fy1 - 2, "iron_00")
        cv.vline(x + 1, fy0 + 6, fy1 - 2, "iron_02")
    cv.hline(fx0 + 3, fx1 - 3, fy1 - 2, "iron_01")
    cv.hline(fx0, fx1, fy0 + 4, "iron_05")
    cv.rect(fx1 - 1, fy0 + 9, fx1 + 2, fy0 + 12, "iron_05")  # latch
    cv.rect(fx0 - 2, fy0 + 8, fx0, fy0 + 10, "iron_03")  # hinges
    cv.rect(fx0 - 2, fy1 - 6, fx0, fy1 - 4, "iron_03")
    # Glow spill on the plinth below the door.
    cv.hline(fx0 + 4, fx1 - 4, fy1 + 1, "flame_00")
    cv.hline(fx0 + 10, fx1 - 10, fy1 + 2, "copper_04")
    # Inlet pipe from the valve (left edge, layout y 550..557).
    pipe_h(cv, 0, 13, 178, 185, COPPER_ROWS)
    cv.rect(6, 176, 8, 187, "iron_03")
    cv.vline(6, 176, 187, "iron_05")
    # Outlet pipe and nozzle (right edge, steam lane origin at x 688).
    pipe_h(cv, 168, 181, 144, 155, COPPER_ROWS)
    cv.rect(172, 142, 175, 157, "iron_03")  # flange
    cv.vline(172, 142, 157, "iron_05")
    hazard_stripes(cv, 177, 143, 180, 156, period=4)  # hazard collar
    for i, x in enumerate(range(181, 191)):  # flared cone
        top = 144 - (i * 4) // 9
        bot = 155 + (i * 4) // 9
        for y in range(top, bot + 1):
            v = (y - top + 0.5) / (bot - top + 1)
            cv.set(x, y, "copper_05" if v < 0.2 else "copper_04" if v < 0.45 else "copper_03" if v < 0.8 else "copper_01")
    cv.rect(190, 140, 191, 159, "copper_05")  # lip seen edge-on
    cv.rect(191, 143, 191, 156, "iron_00")  # mouth
    # Soot over the nozzle and the stack top.
    for y, x0, x1 in ((138, 176, 186), (136, 178, 186), (134, 180, 185), (132, 181, 184)):
        cv.recolor(x0, y, x1, y, {"copper_00": "mortar_01", "copper_01": "mortar_02", "copper_02": "mortar_02",
                                  "copper_03": "mortar_03"})
    cv.outline(OUT_C)
    return cv


def boiler_damaged(intact: Canvas) -> Canvas:
    cv = intact.copy()
    # Firebox dims to deep red embers.
    cv.recolor(68, 175, 124, 203, {"flame_04": "flame_02", "flame_03": "flame_01", "flame_02": "flame_01",
                                   "copper_04": "flame_00"})
    # Stack kinked: the top 12 rows shear 3 px right, cap askew.
    for y in range(0, 12):
        row = cv.p[y][:]
        shift = 3 if y < 8 else 2 if y < 10 else 1
        for x in range(112, 140):
            cv.p[y][x] = None
        for x in range(112, 137):
            v = row[x]
            if v is not None:
                cv.set(x + shift, y, v)
    cv.rect(118, 10, 133, 12, "iron_01")
    # Safety lever snapped: arm hangs down the left side, ball and brick gone.
    cv.rect(40, 4, 92, 17, None)
    cv.rect(61, 6, 66, 12, "iron_03")
    cv.vline(61, 6, 12, "iron_04")
    cv.rect(60, 4, 67, 6, "iron_02")  # sheared stub
    cv.line(60, 9, 46, 27, "iron_03", w=2)
    cv.line(59, 9, 45, 27, "iron_05")
    # Dome split: a torn plate flap lifting off the dome, steam-stained.
    for i, x in enumerate(range(84, 100)):
        top = 22 - (i // 3)
        cv.rect(x, top, x, 25, "copper_04" if i < 6 else "copper_03")
        cv.set(x, top, "copper_05")
    cv.line(80, 26, 106, 30, "iron_00")
    cv.line(81, 27, 105, 31, "mortar_02")
    # Gauge smashed: dark wedge in the face, needle bent past the red zone.
    gx, gy = 48, 66
    cv.ellipse(gx + 0.5, gy + 0.5, 7, 7, "stone_08")
    cv.rect(gx - 5, gy - 4, gx - 1, gy, "mortar_03")
    cv.line(gx - 5, gy + 4, gx + 3, gy - 5, "iron_02")
    cv.line(gx, gy, gx + 6, gy + 2, "iron_00")
    cv.rect(gx, gy, gx + 1, gy + 1, "iron_00")
    # Upper hoop sprung: its end snaps off the shell and juts out to the left.
    by = 90
    cv.rect(20, by, 33, by + 5, None)
    cylinder(cv, 21, 33, by, by + 5, COPPER)
    cv.vline(32, by, by + 5, "iron_01")
    for i in range(14):
        x = 19 - i
        y = by + (i * i) // 22
        cv.rect(x, y, x, y + 4, "iron_03")
        cv.set(x, y, "iron_05")
        cv.set(x, y + 4, "iron_01")
    # Soot streaks rising from the nozzle and the split.
    soot = {"copper_00": "mortar_01", "copper_01": "mortar_02", "copper_02": "mortar_02",
            "copper_03": "mortar_03", "copper_04": "copper_02", "copper_05": "copper_03"}
    for x, y0, y1, w in ((158, 100, 141, 4), (165, 92, 141, 3), (90, 32, 60, 4), (99, 34, 70, 3)):
        cv.recolor(x, y0 + 6, x + w - 1, y1, soot)
        for y in range(y0, y0 + 6):  # dithered fade at the top of each streak
            for xx in range(x, x + w):
                if (xx + y) % 2 == 0:
                    cv.recolor(xx, y, xx, y, soot)
    # Patch plate corner torn loose.
    cv.rect(133, 100, 138, 102, "copper_03")
    cv.rect(136, 97, 139, 99, "copper_04")
    cv.set(139, 97, "copper_05")
    # Nozzle collar scorched.
    cv.recolor(177, 143, 180, 156, {"brass_05": "brass_02"})
    cv.outline(OUT_C, keep=("mortar_02", "mortar_03"))
    return cv


# ================================================================= VALVE ====
VW, VH = 32, 48


def valve_body(cv: Canvas) -> None:
    # Floor base plate with bolts.
    cv.rect(3, 44, 28, 47, "stone_cold_02")
    cv.hline(3, 28, 44, "stone_cold_04")
    cv.hline(3, 28, 47, "stone_cold_00")
    for x in (5, 26):
        cv.rect(x, 45, x + 1, 46, "stone_cold_04")
    # Riser.
    cylinder(cv, 11, 20, 32, 43, STEEL)
    cv.rect(9, 32, 22, 34, "stone_cold_03")
    cv.hline(9, 22, 32, "stone_cold_04")
    # Side outlet pipe into the boiler inlet (layout y 550..557).
    pipe_h(cv, 24, 31, 18, 25, COPPER_ROWS)
    cv.rect(28, 17, 29, 26, "stone_cold_03")
    cv.vline(28, 17, 26, "stone_cold_04")
    # Cast body.
    cv.ellipse(15.5, 24.5, 11, 9, "stone_cold_01")
    cv.ellipse(14.5, 23.5, 9, 7, "stone_cold_02")
    cv.ellipse(12.5, 21.5, 5, 4, "stone_cold_03")
    cv.ellipse(11.5, 20.5, 2, 2, "stone_cold_04")
    cv.hline(6, 25, 25, "stone_cold_00")  # casting seam
    for x in (8, 15, 22):
        cv.set(x, 26, "stone_cold_04")
    # Bonnet and stem.
    cylinder(cv, 11, 20, 11, 16, STEEL)
    cv.hline(10, 21, 11, "stone_cold_04")
    cv.rect(15, 6, 16, 10, "brass_04")
    cv.vline(16, 6, 10, "brass_02")


def valve_intact() -> Canvas:
    cv = Canvas(VW, VH)
    valve_body(cv)
    # Handwheel: horizontal wheel seen from a little above, hazard-yellow rim.
    cv.ellipse(16, 5, 9, 4, "brass_05")
    cv.ellipse(16, 5.5, 9, 4, "brass_04", only=lambda x, y: y >= 6)
    cv.ellipse(16, 5, 7, 2.2, "iron_00")
    cv.hline(9, 22, 5, "iron_02")  # spoke seen edge-on
    cv.rect(15, 4, 16, 6, "brass_03")  # hub
    for x in (8, 13, 18, 23):  # black rim marks: read as hazard paint
        cv.set(x, 8 if 10 < x < 21 else 7, "iron_00")
    cv.outline(OUT_C)
    return cv


def valve_spent() -> Canvas:
    cv = Canvas(VW, VH)
    valve_body(cv)
    # Bonnet blown off: a bent stem stub and a torn neck.
    cv.rect(10, 6, 21, 13, None)
    cv.rect(11, 13, 20, 15, "stone_cold_01")
    for x, y in ((11, 12), (13, 11), (15, 13), (17, 12), (19, 11), (20, 13)):
        cv.rect(x, y, x, 13, "stone_cold_03")
    cv.line(15, 12, 17, 8, "brass_02")
    # Side pipe ruptured: torn petals curling out, dark interior.
    cv.rect(24, 16, 31, 27, None)
    pipe_h(cv, 24, 26, 18, 25, COPPER_ROWS)
    cv.rect(27, 19, 27, 24, "iron_00")
    for x, y0, y1 in ((27, 15, 18), (28, 14, 16), (29, 13, 14), (27, 25, 28), (28, 27, 29), (29, 29, 30)):
        cv.vline(x, y0, y1, "copper_04")
    cv.set(30, 13, "copper_05")
    cv.set(30, 30, "copper_03")
    # Crack through the body and a condensation drip.
    cv.line(9, 19, 14, 24, "iron_00")
    cv.line(14, 24, 12, 30, "iron_00")
    cv.vline(23, 27, 30, "stone_cold_04")
    cv.outline(OUT_C)
    return cv


# ================================================================= BRACE ====
RW, RH = 32, 40


def brace_parts(cv: Canvas, lean: int = 0, cracked: bool = False) -> None:
    # Kicker strut (behind the post), floor foot on the right.
    for i in range(26):
        y = 13 + i
        x = 20 + (i * 9) // 25
        cv.rect(x, y, x + 3, y, "wood_03" if i % 7 else "wood_02")
        cv.set(x, y, "wood_04")
    cv.rect(26, 36, 31, 39, "stone_cold_02")
    cv.hline(26, 31, 36, "stone_cold_04")
    # Post.
    cylinder(cv, 10, 21, 4, 34, WOOD)
    for y in (9, 16, 27):
        cv.hline(12, 14, y, "wood_02")
    cv.vline(18, 6, 12, "wood_03")
    cv.vline(13, 20, 26, "wood_03")
    # Iron shoe bolted to the floor.
    cv.rect(6, 33, 23, 39, "stone_cold_02")
    cv.hline(6, 23, 33, "stone_cold_04")
    cv.hline(6, 23, 39, "stone_cold_00")
    for x in (8, 20):
        cv.rect(x, 36, x + 1, 37, "stone_cold_04")
    # Strap band.
    cv.rect(10, 24, 21, 26, "stone_cold_02")
    cv.hline(10, 21, 24, "stone_cold_03")
    # Top cleat with the shackle eye under the chain (chain column x 15..16).
    cv.rect(9, 2, 22, 5, "stone_cold_02")
    cv.hline(9, 22, 2, "stone_cold_04")
    cv.rect(14, 0, 17, 2, "stone_cold_03")
    cv.rect(15, 0, 16, 1, "iron_00")
    # Hazard wedge pin through the post: the hit target.
    hazard_stripes(cv, 3, 17, 10, 20, period=4)
    cv.rect(21, 18, 23, 19, "stone_cold_03")
    if cracked:
        pass


def brace_supported() -> Canvas:
    cv = Canvas(RW, RH)
    brace_parts(cv)
    cv.outline(OUT_C)
    return cv


def brace_weakened() -> Canvas:
    cv = Canvas(RW, RH)
    brace_parts(cv)
    # The upper post leans 1 px left above the crack.
    for y in range(0, 18):
        row = cv.p[y][:]
        for x in range(0, RW):
            cv.p[y][x] = row[x + 1] if x + 1 < RW else None
    # Split through the post with splinters jutting out.
    cv.line(10, 19, 15, 22, "iron_00")
    cv.line(15, 22, 21, 20, "iron_00")
    cv.line(11, 20, 15, 23, "wood_01")
    for x, y in ((8, 19), (9, 21), (22, 19), (23, 21)):
        cv.set(x, y, "wood_05")
    # Wedge pushed half out and tilted.
    cv.rect(0, 15, 10, 21, None)
    cv.rect(8, 17, 9, 20, "wood_03")
    hazard_stripes(cv, 1, 16, 7, 19, period=4)
    cv.set(0, 17, "brass_05")
    # Strut cracked.
    cv.line(23, 25, 26, 27, "iron_00")
    cv.outline(OUT_C)
    return cv


def brace_stump() -> Canvas:
    cv = Canvas(RW, RH)
    brace_parts(cv)
    # Everything above the break is gone; the strut has dropped flat.
    cv.rect(0, 0, RW - 1, 21, None)
    cv.rect(20, 12, 31, 35, None)
    for x, top in zip(range(10, 22), (22, 20, 23, 21, 24, 22, 20, 23, 25, 22, 24, 23)):
        cv.rect(x, 22, x, top, None)
        cv.set(x, top, "wood_05")
    for x, top in zip(range(10, 22), (22, 20, 23, 21, 24, 22, 20, 23, 25, 22, 24, 23)):
        for y in range(top, 34):
            if cv.get(x, y) is None:
                cv.set(x, y, "wood_04")
    cv.rect(20, 35, 31, 37, "wood_03")  # fallen strut
    cv.hline(20, 31, 35, "wood_04")
    cv.rect(24, 38, 29, 38, "wood_02")
    cv.rect(0, 15, 10, 21, None)
    hazard_stripes(cv, 1, 36, 5, 38, period=4)  # wedge on the floor
    cv.outline(OUT_C)
    return cv


# ================================================================ WEIGHT ====
WW, WH = 64, 48  # body 64 x 32 in the bottom rows, 16 rows above for the fall streaks


def weight_body(cv: Canvas, oy: int) -> None:
    # Trapezoid cast-iron block: bottom 64 wide, top 48 wide.
    for y in range(8, 32):
        inset = 8 - ((y - 8) * 8) // 23
        cylinder(cv, inset, 63 - inset, oy + y, oy + y,
                 [(0.06, "stone_cold_00"), (0.20, "stone_cold_01"), (0.34, "stone_cold_02"),
                  (0.42, "stone_cold_03"), (0.70, "stone_cold_02"), (0.92, "stone_cold_01"),
                  (1.01, "stone_cold_00")])
    cv.hline(8, 55, oy + 8, "stone_cold_04")  # lit top edge
    cv.hline(9, 54, oy + 9, "stone_cold_03")
    cv.hline(1, 62, oy + 31, "stone_cold_00")
    # Hazard chevron band.
    for y in range(18, 24):
        inset = 8 - ((y - 8) * 8) // 23 + 1
        hazard_stripes(cv, inset, 62 - inset, oy + y, oy + y, period=8)
    cv.hline(5, 58, oy + 17, "stone_cold_00")
    cv.hline(4, 59, oy + 24, "stone_cold_00")
    # Bolts along the top and a welded goblin patch.
    for x in range(12, 53, 8):
        cv.set(x, oy + 12, "stone_cold_04")
        cv.set(x, oy + 13, "stone_cold_00")
    cv.rect(40, oy + 26, 49, oy + 29, "iron_03")
    cv.hline(40, 49, oy + 26, "iron_05")
    for x in range(40, 50, 2):
        cv.set(x, oy + 30, "iron_05")
    # Lifting eye and shackle.
    cv.ellipse(32, oy + 4, 6, 5, "stone_cold_02")
    cv.ellipse(32, oy + 4, 3, 2.5, None)
    cv.rect(26, oy + 6, 37, oy + 7, "stone_cold_01")
    cv.set(28, oy + 1, "stone_cold_04")
    cv.set(29, oy + 0, "stone_cold_04")


def weight_hang() -> Canvas:
    cv = Canvas(WW, WH)
    weight_body(cv, 16)
    cv.rect(31, 12, 32, 16, "stone_cold_03")  # shackle pin up to the chain
    cv.outline(OUT_C)
    return cv


# =============================================================== REMNANT ====
MW, MH = 64, 32


def remnant() -> Canvas:
    cv = Canvas(MW, MH)
    # Heaved floor slabs at both ends.
    for x0, x1, top_l, top_r in ((0, 13, 9, 4), (50, 63, 3, 10)):
        for x in range(x0, x1 + 1):
            t = top_l + (top_r - top_l) * (x - x0) // (x1 - x0)
            cv.rect(x, t, x, 31, "floor_05")
            cv.set(x, t, "floor_08")
            cv.set(x, t + 1, "floor_07")
        cv.vline(x0 + 6, 12, 31, "floor_03")
    # The weight, sunk and tilted: visible top 20 rows, left side higher.
    for x in range(6, 58):
        top = 1 + (x - 6) * 2 // 51
        u = (x - 6 + 0.5) / 52
        c = band(u, [(0.06, "stone_cold_00"), (0.22, "stone_cold_01"), (0.36, "stone_cold_02"),
                     (0.44, "stone_cold_03"), (0.72, "stone_cold_02"), (0.93, "stone_cold_01"),
                     (1.01, "stone_cold_00")])
        cv.rect(x, top, x, 22, c)
        cv.set(x, top, "stone_cold_04")
        cv.set(x, top + 1, "stone_cold_03")
        band_y = top + 9
        k = (x + band_y) // 4
        cv.rect(x, band_y, x, band_y + 4, "brass_05" if k % 2 == 0 else "iron_00")
        cv.set(x, band_y - 1, "stone_cold_00")
        cv.set(x, band_y + 5, "stone_cold_00")
    # Broken floor and brick rubble along the base.
    cv.rect(2, 23, 61, 31, "floor_04")
    cv.hline(2, 61, 23, "floor_06")
    for x0, y0, w, h, c, hi in ((4, 21, 7, 5, "brick_06", "brick_08"), (14, 22, 9, 4, "stone_05", "stone_07"),
                                 (27, 24, 6, 3, "brick_05", "brick_07"), (36, 22, 8, 5, "stone_04", "stone_06"),
                                 (47, 21, 7, 6, "brick_06", "brick_08"), (56, 24, 6, 4, "stone_05", "stone_07")):
        cv.rect(x0, y0, x0 + w - 1, y0 + h - 1, c)
        cv.hline(x0, x0 + w - 2, y0, hi)
    for x in range(3, 61, 5):
        cv.set(x, 28, "floor_02")
        cv.set(x + 2, 30, "floor_03")
    # Slack chain coiled on top.
    for i, x in enumerate(range(20, 40, 3)):
        y = 1 + (i % 2)
        cv.rect(x, y - 1, x + 1, y, "stone_cold_03")
        cv.set(x + 2, y, "stone_cold_01")
    cv.outline(OUT_C)
    return cv


# ================================================================== DRUM ====
DW, DH = 36, 36  # body 32 x 32 at (2, 4); 2 px margins for the telegraph bulge and hiss


def drum_armed() -> Canvas:
    cv = Canvas(DW, DH)
    ox, oy = 2, 4
    cylinder(cv, ox, ox + 31, oy + 2, oy + 31, RED)
    # Lid seen from a little above.
    cv.rect(ox + 1, oy, ox + 30, oy + 2, "brick_04")
    cv.hline(ox + 2, ox + 29, oy, "stone_cold_04")
    cv.hline(ox + 1, ox + 30, oy + 1, "stone_cold_02")
    cv.hline(ox + 1, ox + 30, oy + 2, "stone_cold_03")
    cv.rect(ox + 21, oy, ox + 24, oy + 1, "brass_04")  # bung cap
    cv.hline(ox + 21, ox + 24, oy, "brass_05")
    # Rolling hoops.
    for hy in (oy + 9, oy + 22):
        cylinder(cv, ox, ox + 31, hy, hy + 1, STEEL)
        cv.hline(ox, ox + 31, hy - 1, "brick_04")
        cv.hline(ox + 4, ox + 13, hy, "stone_cold_04")
    # Hazard stripe band.
    hazard_stripes(cv, ox + 1, oy + 13, ox + 30, oy + 18, period=6)
    cv.hline(ox, ox + 31, oy + 12, "iron_00")
    cv.hline(ox, ox + 31, oy + 19, "iron_00")
    # Bottom chime, a dent and a fuel stain.
    cv.rect(ox, oy + 29, ox + 31, oy + 31, "stone_cold_01")
    cv.hline(ox, ox + 31, oy + 29, "stone_cold_02")
    cv.rect(ox + 22, oy + 4, ox + 24, oy + 6, "brick_04")
    cv.set(ox + 22, oy + 4, "flame_00")
    for y in range(oy + 3, oy + 9):
        cv.set(ox + 25, y, "mortar_02")
    cv.set(ox + 26, oy + 8, "mortar_02")
    cv.outline(OUT_C)
    return cv


SW, SH = 48, 8


def drum_scorch() -> Canvas:
    cv = Canvas(SW, SH)
    cv.ellipse(23.5, 6, 23, 3, "mortar_01")
    cv.ellipse(22.5, 6, 18, 2.2, "iron_00")
    cv.ellipse(20, 6, 10, 1.5, "mortar_00")
    # Blackened hoop lying flat on the right, twisted shards.
    cv.ellipse(36.5, 4.5, 7, 2.5, "stone_cold_01")
    cv.ellipse(36.5, 4.5, 5, 1.2, "mortar_01")
    cv.hline(31, 37, 2, "stone_cold_02")
    for x, y, c in ((8, 4, "stone_cold_02"), (9, 3, "stone_cold_03"), (14, 5, "flame_00"), (15, 5, "brick_04"),
                    (26, 3, "stone_cold_02"), (27, 4, "stone_cold_01")):
        cv.set(x, y, c)
    cv.set(19, 5, "flame_01")  # last ember (animated by execution)
    cv.set(20, 5, "flame_00")
    return cv


# =================================================================== RIG ====
GW, GH = 112, 24


def rig() -> Canvas:
    """Ceiling-hung I-beam with two sheave blocks (chain columns x 15..16 and 71..72)."""
    cv = Canvas(GW, GH)
    # Hanger straps up to the ceiling, with bolt plates.
    for hx in (40, 96):
        cv.rect(hx, 0, hx + 3, 6, "iron_02")
        cv.vline(hx, 0, 6, "iron_04")
        cv.rect(hx - 2, 4, hx + 5, 6, "iron_03")
        cv.hline(hx - 2, hx + 5, 4, "iron_05")
    # I-beam: top flange, web, bottom flange.
    cv.rect(0, 7, 111, 8, "stone_cold_02")
    cv.hline(0, 111, 7, "stone_cold_04")
    cv.rect(2, 9, 109, 11, "stone_cold_01")
    cv.rect(0, 12, 111, 13, "stone_cold_02")
    cv.hline(0, 111, 13, "stone_cold_00")
    for x in range(6, 108, 10):
        cv.rect(x, 10, x + 1, 10, "stone_cold_03")
    # Sheave blocks: side cheek plates with the pulley rim showing between them.
    for cx in (16, 72):
        cv.rect(cx - 1, 14, cx, 15, "stone_cold_03")  # clevis pin
        cv.rect(cx - 5, 15, cx + 4, 21, "stone_cold_01")
        cv.hline(cx - 5, cx + 4, 15, "stone_cold_03")
        cv.vline(cx - 5, 15, 21, "stone_cold_02")
        cv.ellipse(cx, 18.5, 3, 3, "stone_cold_03")
        cv.ellipse(cx, 18.5, 1.5, 1.5, "stone_cold_00")
        cv.rect(cx - 6, 17, cx - 6, 19, "stone_cold_04")  # axle bolts
        cv.rect(cx + 5, 17, cx + 5, 19, "stone_cold_02")
        cv.rect(cx - 1, 22, cx, 23, "stone_cold_02")  # chain lead-out
    # Return chain sagging between the sheaves, hazard tag on the slack.
    for x in range(22, 67):
        sag = 0 if x < 28 or x > 60 else 1 if x < 34 or x > 54 else 2
        cv.set(x, 20 + sag, "stone_cold_03" if (x // 2) % 2 == 0 else "stone_cold_01")
        cv.set(x, 21 + sag, "stone_cold_00")
    hazard_stripes(cv, 42, 23, 47, 23, period=2)
    cv.outline(OUT_C)
    return cv


CHW, CHH = 4, 8


def chain_tile() -> Canvas:
    cv = Canvas(CHW, CHH)
    # A face-on oval link (rows 0..4), then an edge-on link (rows 5..7).
    cv.rect(0, 0, 3, 4, "stone_cold_02")
    cv.rect(1, 1, 2, 3, None)
    cv.set(0, 0, None)
    cv.set(3, 0, None)
    cv.set(0, 4, None)
    cv.set(3, 4, None)
    cv.set(0, 1, "stone_cold_04")
    cv.set(0, 2, "stone_cold_03")
    cv.set(3, 3, "stone_cold_00")
    cv.rect(1, 4, 2, 7, "stone_cold_03")
    cv.vline(2, 5, 7, "stone_cold_01")
    cv.set(1, 5, "stone_cold_04")
    return cv


# ================================================================ OUTPUT ====
def to_grid(frames: list[tuple[str, Canvas, int]], pivot: tuple[int, int], layer: str,
            pal: dict[str, str]) -> dict:
    names: list[str] = []
    for _, cv, _ in frames:
        for row in cv.p:
            for v in row:
                if v is not None and v not in names:
                    names.append(v)
    names.sort(key=lambda n: list(pal).index(n))
    if len(names) > len(KEYS):
        raise SystemExit("too many colours")
    key_of = {n: KEYS[i] for i, n in enumerate(names)}
    w, h = frames[0][1].w, frames[0][1].h
    out_frames, tags = [], []
    for i, (tag, cv, ms) in enumerate(frames):
        rows = ["".join("." if v is None else key_of[v] for v in row) for row in cv.p]
        out_frames.append({"duration_ms": ms, "cels": {layer: rows}})
        tags.append({"name": tag, "from": i, "to": i, "direction": "forward"})
    return {
        "format": "pixel-grid-v1",
        "canvas": [w, h],
        "pivot": list(pivot),
        "palette": [{"key": key_of[n], "name": n, "rgba": pal[n]} for n in names],
        "layers": [layer],
        "frames": out_frames,
        "tags": tags,
    }


def build(out: Path) -> list[Path]:
    pal = kit_palette()
    b0 = boiler_intact()
    assets = {
        "gw_boiler": ([("boiler_idle", b0, 133), ("boiler_damaged", boiler_damaged(b0), 133)], (96, BH - 1)),
        "gw_valve": ([("valve_intact", valve_intact(), 100), ("valve_spent", valve_spent(), 100)], (16, VH - 1)),
        "gw_brace": ([("brace_supported", brace_supported(), 100), ("brace_weakened", brace_weakened(), 100),
                      ("brace_stump", brace_stump(), 100)], (16, RH - 1)),
        "gw_weight": ([("weight_hang", weight_hang(), 100)], (32, WH - 1)),
        "gw_remnant": ([("remnant", remnant(), 100)], (32, MH - 1)),
        "gw_drum": ([("drum_armed", drum_armed(), 100)], (18, DH - 1)),
        "gw_scorch": ([("scorch", drum_scorch(), 100)], (24, SH - 1)),
        "gw_rig": ([("rig", rig(), 100)], (0, 0)),
        "gw_chain": ([("chain", chain_tile(), 100)], (0, 0)),
    }
    written = []
    out.mkdir(parents=True, exist_ok=True)
    for name, (frames, pivot) in assets.items():
        grid = to_grid(frames, pivot, "body", pal)
        path = out / f"{name}.grid.json"
        path.write_text(json.dumps(grid, indent=1) + "\n")
        written.append(path)
    return written


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args()
    for p in build(args.out):
        print(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p)


if __name__ == "__main__":
    main()
