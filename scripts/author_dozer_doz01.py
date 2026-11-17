#!/usr/bin/env python3
"""DOZ-01 authoring record: Goblin Murder Dozer palette, part construction, size study and keys.

Usage:

    python3 scripts/author_dozer_doz01.py [--check] [--out-dir DIR]

Writes (deterministic, pixel-grid-v1) under ``assets/source/enemies/dozer/``:

- ``dozer_size_study.grid.json``: the whole machine with its three riders, flattened, at three
  body heights (96, 120 and 150 px). One parametric construction is rasterized natively at each
  scale (no resampling between sizes); the 36 px riders keep their size. Block-ins for the size
  decision, not production pixels.
- ``dozer_hull_h120.grid.json``: hull key, frame 0 ``body`` (behind the riders) and frame 1 ``front``
  (the rider tub and the cab rail, drawn over the riders' lower bodies).
- ``dozer_wheel_h120.grid.json``: spiked wheel key (phase 2 redesign), phase 0 of the 4-frame plan.
- ``dozer_tread_h120.grid.json``: tread key, phase 0 of the 9-frame (1 px) belt plan.
- ``dozer_yoke_h120.grid.json``: near-side push arm, axle cap and lamp (static).
- ``dozer_riders_h36.grid.json``: rider keys: engineer point, lever pull and both lurches; rider A
  brandish and both lurches; rider B hold and both lurches (A's construction on GOB-01 variant B).
- ``dozer_smoke.grid.json``: smoke keys: black puff frames 0 and 2, steam puff frames 0 and 2.

The size h120 was proposed here and decided by a separate Opus agent (DOZ-01 phase 1 / 2).
- ``dozer_rig_h120.json``: part pivots, anchors and draw order (the data contract of the spec).

and one manifest per grid under ``assets/manifests/``. ``--check`` compares instead of writing.

How the pixels are made: the machine parts are polygons, discs and capsules in design units
(x forward from the rear end of the tread, y up from the floor), filled per pixel centre with a
material ramp, bevelled (light from the upper left), outlined with the shared plum outline, and
detailed with rivets, seams and streaks placed on a grid. A deterministic despeckle pass removes
orphan pixels. The wheel and tread are rendered from their geometry at a phase, so the remaining
rotation and belt frames are the same function at other phases plus hand cleanup (spec section 7).
The riders reuse GOB-01 goblin pixels (heads, vests, pineapple) translated whole, with a new pot
helmet, pointing arm and rim grip drawn here. No image-model output, tracing or resampling.
The concept crop under ``assets/concept/floor-01/opening/`` was used for mood only.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import author_goblin_gob01 as ag  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
DOZER_DIR = ROOT / "assets/source/enemies/dozer"
MANIFEST_DIR = ROOT / "assets/manifests"
GOBLIN_DIR = ROOT / "assets/source/characters/goblin"

# ------------------------------------------------------------------ palette `dozer-floor01-v0`
# Machine: cool boiler iron (separates from the warm orange brick), dark rust plates, an oxblood
# painted plate, ENV-01 copper and brass ramps, bone paint and ENV-01 fire colours (the firebox
# slots and lamp glass are the only emissive pixels). Smoke and steam ramps are reserved for the
# puff sprites (Sonnet). The riders use the GOB-01 palette; K (outline plum) is shared.
DOZER_PALETTE = [
    ("0", "iron_0", "#121220ff", False),
    ("1", "iron_1", "#1e2032ff", False),
    ("2", "iron_2", "#2c3046ff", False),
    ("3", "iron_3", "#3f445eff", False),
    ("4", "iron_4", "#575e7aff", False),
    ("5", "iron_5", "#78809aff", False),
    ("6", "iron_6", "#a3abbfff", False),
    ("7", "iron_glint", "#dde2ecff", True),     # accent: rivet heads, spike-tip glints
    ("Q", "rust_0", "#2a1612ff", False),
    ("W", "rust_1", "#4a2318ff", False),
    ("E", "rust_2", "#6c321eff", False),
    ("T", "rust_3", "#914524ff", False),
    ("Y", "rust_4", "#b55f2cff", False),
    ("p", "paint_0", "#3c1219ff", False),
    ("b", "paint_1", "#611b22ff", False),
    ("d", "paint_2", "#8a2b2bff", False),
    ("c", "copper_1", "#3a1a14ff", False),     # ENV-01 copper_01
    ("f", "copper_2", "#5c2a18ff", False),     # ENV-01 copper_02
    ("g", "copper_3", "#823e1eff", False),     # ENV-01 copper_03
    ("h", "copper_4", "#a85626ff", False),     # ENV-01 copper_04
    ("i", "copper_5", "#cc7332ff", False),     # ENV-01 copper_05
    ("L", "brass_1", "#43331aff", False),      # ENV-01 brass_01
    ("M", "brass_2", "#6a5224ff", False),      # ENV-01 brass_02
    ("N", "brass_3", "#93752eff", False),      # ENV-01 brass_03
    ("S", "brass_4", "#bc9a3aff", False),      # ENV-01 brass_04
    ("Z", "brass_5", "#e0c25aff", True),       # ENV-01 brass_05; accent: brass glints
    ("X", "bone_paint", "#d8cdb0ff", False),
    ("j", "fire_1", "#8a2a12ff", False),       # ENV-01 flame/emissive colours (bloom)
    ("k", "fire_2", "#d4501aff", False),
    ("n", "fire_3", "#f58a26ff", False),
    ("q", "fire_4", "#ffc048ff", False),
    ("z", "fire_5", "#ffe98aff", False),
    ("+", "smoke_0", "#1c1a22ff", False),      # reserved: black exhaust puffs (Sonnet)
    ("=", "smoke_1", "#2e2a34ff", False),
    ("-", "smoke_2", "#4a4450ff", False),
    (":", "smoke_3", "#6c6474ff", False),
    (";", "steam_0", "#9c9caeff", False),      # reserved: steam puffs (Sonnet)
    ("/", "steam_1", "#c6c8d4ff", False),
    ("*", "steam_2", "#eceef4ff", False),
]
OUTLINE = ("K", "outline_plum", "#1b1124ff", False)
ALL_PALETTE = [OUTLINE] + DOZER_PALETTE + [p for p in ag.PALETTE if p[0] != "K"]
ACCENTS = {k for k, _n, _c, a in ALL_PALETTE if a}
ORDER = {k: i for i, (k, *_r) in enumerate(ALL_PALETTE)}

IRON = ("0", "1", "2", "3", "4", "5", "6")
RUST = ("Q", "W", "E", "T", "Y")
PAINT = ("p", "p", "b", "d", "d")
COPPER = ("c", "f", "g", "h", "i")
BRASS = ("L", "M", "N", "S", "S")
DARK_IRON = ("0", "0", "1", "2", "3", "4")
CAB_RUST = ("Q", "Q", "W", "W", "E", "T")   # dark, desaturated rust: must not melt into the orange brick

# ------------------------------------------------------------------ geometry (design units, s = 1)
H_KEY = 120                    # body height of the recommended option: floor to boiler dome top
SIZE_OPTIONS = (96, 120, 150)  # body heights of the three size options
TREAD_X = (0.0, 148.0)         # tread footprint, rig x from the rear end
O_X = 74.0                     # rig origin O: tread footprint midpoint on the floor
AXLE = (198.0, 49.0)           # spiked wheel axle pixel: the lowest spike outline lands on the contact row
WHEEL_RIM = 36.0
WHEEL_TIP = 48.0
SPIKES = 12
SEATS = {"engineer": (26.0, 98.0), "rider_a": (112.0, 96.0), "rider_b": (140.0, 96.0)}
RIM_Y = {"engineer": 105.0, "rider_a": 104.0, "rider_b": 104.0}   # occluder top over each seat
EMITTERS = {"stack": (89.0, 156.0), "rear_exhaust": (-8.0, 124.0), "kettle": (79.0, 129.0)}
LAMP = (172.0, 66.0)
TREAD_PHASES = 9                # belt frames: 1 px steps of the 9 px link pitch
RIDER_CANVAS = (64, 56)
RIDER_PIVOT = (28, 55)          # the seat point (hip contact), not the floor


# ------------------------------------------------------------------ raster core

class Raster:
    """Per-pixel part map over a canvas; design point (x, y_up) -> canvas (ox + x*s, fy + 1 - y_up*s)."""

    def __init__(self, w: int, h: int, ox: float, fy: int, s: float = 1.0):
        self.w, self.h, self.ox, self.fy, self.s = w, h, ox, fy, s
        self.pid: list[list[int | None]] = [[None] * w for _ in range(h)]
        self.parts: list[dict] = []
        self.key: list[list[str]] = [["."] * w for _ in range(h)]

    # design <-> pixel
    def to_design(self, c: int, r: int) -> tuple[float, float]:
        return ((c + 0.5 - self.ox) / self.s, (self.fy + 0.5 - r) / self.s)

    def to_px(self, x: float, y: float) -> tuple[int, int]:
        return (math.floor(self.ox + x * self.s), math.floor(self.fy + 1 - y * self.s))

    def anchor_px(self, x: float, y: float) -> tuple[int, int]:
        """Anchor convention: design point (x, y) names the pixel whose lower-left corner it is,
        so (x, 0) is a pixel on the contact row ``fy``."""
        return (math.floor(self.ox + x * self.s), self.fy - math.floor(y * self.s))

    def add(self, test, bbox, ramp, mode="flat", outline=False, **kw) -> int:
        part = {"ramp": ramp, "mode": mode, "outline": outline, "bbox": bbox, **kw}
        self.parts.append(part)
        pid = len(self.parts) - 1
        x0, y0, x1, y1 = bbox
        c0, r1 = self.to_px(x0, y0)
        c1, r0 = self.to_px(x1, y1)
        for r in range(max(0, r0 - 2), min(self.h, r1 + 3)):
            for c in range(max(0, c0 - 2), min(self.w, c1 + 3)):
                if test(*self.to_design(c, r)):
                    self.pid[r][c] = pid
        return pid

    def poly(self, pts, ramp, **kw) -> int:
        def inside(px, py):
            hit = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > py) != (yb > py) and px < xa + (py - ya) * (xb - xa) / (yb - ya):
                    hit = not hit
            return hit
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        return self.add(inside, (min(xs), min(ys), max(xs), max(ys)), ramp, **kw)

    def rect(self, x0, y0, x1, y1, ramp, **kw) -> int:
        kw.setdefault("x0", x0); kw.setdefault("x1", x1); kw.setdefault("y0", y0); kw.setdefault("y1", y1)
        return self.add(lambda px, py: x0 <= px < x1 and y0 <= py < y1, (x0, y0, x1, y1), ramp, **kw)

    def disc(self, cx, cy, r, ramp, **kw) -> int:
        kw.setdefault("cx", cx); kw.setdefault("cy", cy); kw.setdefault("r", r)
        return self.add(lambda px, py: (px - cx) ** 2 + (py - cy) ** 2 <= r * r,
                        (cx - r, cy - r, cx + r, cy + r), ramp, mode=kw.pop("mode", "radial"), **kw)

    def capsule(self, p0, p1, rad, ramp, **kw) -> int:
        (xa, ya), (xb, yb) = p0, p1

        def near(px, py):
            dx, dy = xb - xa, yb - ya
            ln = dx * dx + dy * dy
            t = 0.0 if ln == 0 else max(0.0, min(1.0, ((px - xa) * dx + (py - ya) * dy) / ln))
            return math.hypot(px - (xa + t * dx), py - (ya + t * dy)) <= rad
        return self.add(near, (min(xa, xb) - rad, min(ya, yb) - rad, max(xa, xb) + rad, max(ya, yb) + rad),
                        ramp, **kw)

    # shading
    def _level(self, part: dict, c: int, r: int) -> int:
        x, y = self.to_design(c, r)
        n = len(part["ramp"])
        mode = part["mode"]
        if mode == "cyl_v":                       # vertical cylinder, lit from the left
            t = (x - part["x0"]) / max(1e-6, part["x1"] - part["x0"])
            bands = (0.14, 0.42, 0.72, 0.9)
            lv = [2, 3, 2, 1, 0][sum(t > b for b in bands)]
            return min(n - 2, lv + 1)
        if mode == "cyl_h":                       # horizontal tube, lit from above
            t = (part["y1"] - y) / max(1e-6, part["y1"] - part["y0"])
            bands = (0.18, 0.5, 0.8)
            return min(n - 2, [3, 2, 1, 0][sum(t > b for b in bands)] + 1)
        if mode == "radial":                      # disc / dome: light from the upper left
            dx, dy = x - part["cx"], y - part["cy"]
            d = math.hypot(dx, dy) / max(1e-6, part["r"])
            lit = (-dx * 0.6 + dy * 0.8) / max(1e-6, part["r"])
            v = 2 + (1 if lit > 0.25 else 0) + (1 if lit > 0.6 else 0) - (1 if lit < -0.35 else 0) - (1 if d > 0.92 and lit < 0 else 0)
            return max(0, min(n - 2, v))
        return part.get("level", 2)               # flat plate

    def render(self) -> list[list[str]]:
        out = self.key
        get = lambda c, r: self.pid[r][c] if 0 <= c < self.w and 0 <= r < self.h else None  # noqa: E731
        for r in range(self.h):
            for c in range(self.w):
                pid = self.pid[r][c]
                if pid is None:
                    if any(get(c + dc, r + dr) is not None for dc, dr in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                        out[r][c] = "K"
                    continue
                part = self.parts[pid]
                ramp = part["ramp"]
                up, left, right, down = get(c, r - 1), get(c - 1, r), get(c + 1, r), get(c, r + 1)
                nb = (up, left, right, down)
                if part["outline"] and any(n is not None and n < pid for n in nb):
                    out[r][c] = "K"
                    continue
                lv = self._level(part, c, r)
                if part.get("bevel", True):
                    if right != pid or down != pid:
                        lv = 0 if (right is not None and right != pid) or (down is not None and down != pid) else max(0, lv - 1)
                    elif up != pid or left != pid:
                        lv = min(len(ramp) - 1, lv + 2)
                out[r][c] = ramp[max(0, min(len(ramp) - 1, lv))]
        return out

    # details (drawn into self.key after render)
    def put(self, x: float, y: float, k: str, only_on=None) -> None:
        c, r = self.to_px(x, y)
        if 0 <= c < self.w and 0 <= r < self.h and self.key[r][c] != "." and self.key[r][c] != "K":
            if only_on is None or self.pid[r][c] in only_on:
                self.key[r][c] = k

    def put_px(self, c: int, r: int, k: str) -> None:
        if 0 <= c < self.w and 0 <= r < self.h:
            self.key[r][c] = k

    def hline(self, x0, x1, y, k, only_on=None) -> None:
        c0, r = self.to_px(x0, y)
        c1, _ = self.to_px(x1, y)
        for c in range(c0, c1 + 1):
            if 0 <= c < self.w and 0 <= r < self.h and self.key[r][c] not in ".K":
                if only_on is None or self.pid[r][c] in only_on:
                    self.key[r][c] = k

    def vline(self, x, y0, y1, k, only_on=None) -> None:
        c, r1 = self.to_px(x, y0)
        _, r0 = self.to_px(x, y1)
        for r in range(r0, r1 + 1):
            if 0 <= c < self.w and 0 <= r < self.h and self.key[r][c] not in ".K":
                if only_on is None or self.pid[r][c] in only_on:
                    self.key[r][c] = k

    def rivets(self, x0, x1, y, step, k="7", only_on=None) -> None:
        n = max(1, int((x1 - x0) * self.s // max(3, step * self.s)))
        for i in range(n + 1):
            self.put(x0 + (x1 - x0) * i / n, y, k, only_on)

    def seam(self, x0, x1, y, part_ramp, only_on=None, rivet_step=None) -> None:
        """Horizontal plate seam: a dark line with a lit line under it (and rivets above)."""
        self.hline(x0, x1, y, part_ramp[0], only_on)
        self.hline(x0, x1, y - 1 / self.s, part_ramp[min(len(part_ramp) - 1, 4)], only_on)
        if rivet_step:
            self.rivets(x0 + 2, x1 - 2, y + 2 / self.s, rivet_step, only_on=only_on)


def despeckle(rows: list[list[str]], passes: int = 3) -> list[list[str]]:
    """Recolour orphan pixels (non-accent, no same-key 8-neighbour) to their most common
    non-transparent 8-neighbour key (ties: palette order). Isolated pixels are removed."""
    h, w = len(rows), len(rows[0])
    for _ in range(passes):
        changed = False
        for y in range(h):
            for x in range(w):
                k = rows[y][x]
                if k == ".":
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                solid = [a for a in around if a != "."]
                if not solid:
                    rows[y][x] = "."
                    changed = True
                    continue
                if k in ACCENTS or k in around:
                    continue
                counts: dict[str, int] = {}
                for a in solid:
                    counts[a] = counts.get(a, 0) + 1
                best = sorted(counts.items(), key=lambda kv: (-kv[1], ORDER[kv[0]]))[0][0]
                rows[y][x] = best
                changed = True
        if not changed:
            break
    return rows


# ------------------------------------------------------------------ machine parts

def build_hull(rs: Raster, layer: str) -> dict[str, list[int]]:
    """Draw the hull body (``body``) or the hull front occluder (``front``) into ``rs``."""
    ids: dict[str, list[int]] = {}
    if layer == "body":
        # rear exhaust: copper pipe up the back, elbow back, dented tin-can cap
        ids["pipe"] = [rs.rect(2, 40, 7, 118, COPPER, mode="cyl_v", outline=True),
                       rs.capsule((4.5, 116), (-2, 122), 2.6, COPPER, outline=True)]
        ids["can"] = [rs.poly([(-10, 119), (-3, 118), (-2, 127), (-9, 129)], RUST, outline=True)]
        # cab lower box: mismatched rust plates on the chassis
        ids["cab"] = [rs.poly([(4, 26), (58, 26), (58, 86), (6, 86)], CAB_RUST, level=2)]
        # deck plate the engineer's seat stands on
        ids["deck"] = [rs.rect(1, 84, 60, 89, IRON, mode="cyl_h", outline=True)]
        # throttle lever: rod from the deck pivot to a brass knob under the engineer's hand
        ids["lever"] = [rs.capsule((45, 89), (39, 104), 1.1, IRON, outline=True),
                        rs.disc(38.5, 106, 2.4, BRASS, outline=True)]
        # vertical boiler with a dome; the body height is the dome top (120)
        ids["boiler"] = [rs.rect(56, 26, 102, 97, IRON, mode="cyl_v"),
                         rs.disc(79, 97, 23, IRON, mode="cyl_v", x0=56, x1=102)]
        # kettle soldered on as the steam whistle; main stack with a collar and a bucket cap
        ids["kettle"] = [rs.poly([(64, 117), (75, 117), (74, 125), (66, 125)], BRASS, outline=True),
                         rs.poly([(74, 121), (79, 127), (78, 129), (73, 124)], BRASS, outline=True),
                         rs.rect(68, 125, 72, 128, BRASS, outline=True)]
        ids["stack"] = [rs.rect(82, 112, 90, 131, RUST, mode="cyl_v", outline=True),
                        rs.rect(84, 130, 92, 147, RUST, mode="cyl_v", outline=True),
                        rs.rect(81, 129, 93, 133, BRASS, mode="cyl_h", outline=True)]
        ids["bucket"] = [rs.poly([(79, 146), (99, 146), (96, 154), (82, 154)], IRON, outline=True)]
        # front-tub back wall, seen only between the riders above the boiler line
        ids["tubwall"] = [rs.rect(100, 26, 150, 101, DARK_IRON, level=1, bevel=False)]
        # far-side push arm stub (the near arm is the yoke part)
        ids["chassis"] = [rs.rect(2, 24, 146, 30, DARK_IRON, level=2)]
    else:
        # rider tub: slanted front, rolled lip; painted oxblood plate with bone teeth
        ids["tub"] = [rs.poly([(96, 24), (141, 24), (147, 50), (153, 101), (96, 101)], IRON, level=2)]
        ids["paint"] = [rs.poly([(103, 28), (140, 28), (145, 50), (147, 64), (103, 64)], PAINT, level=2)]
        ids["lip"] = [rs.rect(94, 100, 157, 104, IRON, mode="cyl_h", outline=True)]
        # cab rail plate in front of the engineer's legs, with a tube rail on top
        ids["rail"] = [rs.rect(2, 86, 55, 102, RUST, level=2),
                       rs.rect(0, 101, 57, 105, IRON, mode="cyl_h", outline=True)]
    return ids


def detail_hull(rs: Raster, layer: str, ids: dict[str, list[int]]) -> None:
    s = rs.s
    if layer == "body":
        cab = set(ids["cab"])
        rs.seam(6, 57, 60, CAB_RUST, cab, rivet_step=7)
        rs.seam(6, 57, 74, CAB_RUST, cab, rivet_step=7)
        rs.vline(31, 27, 59, "Q", cab)
        rs.vline(31.9, 27, 59, "T", cab)
        # firebox door with three glowing slots (the only emissive pixels with the lamp)
        door = rs.poly([(36, 40), (52, 40), (52, 54), (44, 58), (36, 54)], BRASS, outline=True)
        rs.render_part(door)
        for i, y in enumerate((44, 48, 52)):
            rs.hline(38.5, 49.5, y, "n" if i < 2 else "k", {door})
            rs.hline(38.5, 49.5, y + 1 / s, "q" if i == 0 else "n", {door})
        rs.put(37.5, 55, "Z", {door})
        # rust streaks running down from the seams
        for x, y0, y1 in ((12, 44, 58), (24, 62, 72), (46, 66, 73), (20, 30, 40)):
            rs.vline(x, y0, y1, "T", cab)
            rs.vline(x + 1 / s, y0 + 3, y1, "E", cab)
        boiler = set(ids["boiler"])
        for y in (46, 70, 92):
            rs.hline(57, 101, y, IRON[0], boiler)
            rs.hline(57, 101, y + 1 / s, IRON[5], boiler)
            rs.rivets(60, 98, y - 2 / s, 6, only_on=boiler)
        # copper patch, riveted at the corners
        patch = rs.rect(61, 52, 76, 64, COPPER, level=2, outline=True)
        rs.render_part(patch)
        for x, y in ((63, 54), (73.5, 54), (63, 61.5), (73.5, 61.5)):
            rs.put(x, y, "Z", {patch})
        # pressure gauge, needle past the red
        gauge = rs.disc(90, 78, 5.2, BRASS, outline=True)
        face = rs.disc(90, 78, 3.6, ("X", "X", "X", "X", "X"), mode="flat", outline=False, bevel=False)
        rs.render_part(gauge)
        rs.render_part(face)
        rs.put(91.2, 79.2, "d", {face}); rs.put(92.3, 80.3, "d", {face})
        rs.put(90, 78, "K", {face}); rs.put(91, 78.8, "K", {face})
        # soot on the stack top and the bucket's dent
        stack = set(ids["stack"])
        rs.hline(84, 92, 145, "Q", stack)
        rs.hline(84, 92, 144, "W", stack)
        rs.put(88, 153, "2", set(ids["bucket"])); rs.put(89, 153, "2", set(ids["bucket"]))
        rs.rivets(4, 59, 86.5, 6, only_on=set(ids["deck"]))
        # soot and oil above the treads, a dent in the boiler (dark crease with a lit lip)
        for y, k in ((27, "0"), (28, "1"), (30, "1")):
            rs.hline(57, 101, y, k, boiler)
        for x, y in ((66, 38), (67, 37), (68, 37)):
            rs.put(x, y, "1", boiler); rs.put(x, y - 1 / s, "5", boiler)
    else:
        tub = set(ids["tub"])
        # a mismatched rust plate bolted over a hole in the tub side
        patch = rs.poly([(127, 68), (149, 69), (150, 80), (128, 80)], CAB_RUST, level=3, outline=True)
        rs.render_part(patch)
        for x, y in ((130, 77.5), (146.5, 77.5), (130, 71), (146.5, 71.5)):
            rs.put(x, y, "7", {patch})
        rs.seam(101, 151, 82, IRON, tub, rivet_step=7)
        for y, k in ((25, "0"), (26, "1")):
            rs.hline(101, 141, y, k, tub)
        rs.rivets(104, 148, 67, 7, only_on=tub)
        rs.vline(122, 66, 81, IRON[1], tub)
        rs.vline(123, 66, 81, IRON[4], tub)
        for x, y0, y1 in ((110, 70, 80), (138, 84, 98), (128, 72, 80)):
            rs.vline(x, y0, y1, "E", tub)
            rs.vline(x + 1 / s, y0 + 2, y1, "W", tub)
        paint = set(ids["paint"])
        # the painted grin: dark mouth band, bone teeth top and bottom, chipped paint
        for y in range(43, 51):
            rs.hline(104, 145, y + 0.5, "p", paint)
        for x0 in range(105, 144, 6):
            for i in range(5):
                w = 2.5 - i * 0.5
                rs.hline(x0 + 3 - w, x0 + 3 + w, 54.5 - i, "X", paint)       # upper teeth point down
                rs.hline(x0 + 6 - w, x0 + 6 + w, 39.5 + i, "X", paint)       # lower teeth, staggered, point up
        for x, y in ((108, 60), (130, 33), (136, 58), (117, 31)):
            rs.put(x, y, "E", paint); rs.put(x + 1 / s, y, "E", paint)
        rs.rivets(4, 54, 89, 7, only_on=set(ids["rail"][:1]))
        rail = set(ids["rail"][:1])
        rs.vline(28, 87, 100, "Q", rail)
        rs.vline(29, 87, 100, "T", rail)
        for x, y0, y1 in ((10, 88, 97), (44, 90, 99)):
            rs.vline(x, y0, y1, "T", rail)


def _render_part(self: Raster, pid: int) -> None:
    """Re-render one part's pixels (used for detail parts added after the main render)."""
    get = lambda c, r: self.pid[r][c] if 0 <= c < self.w and 0 <= r < self.h else None  # noqa: E731
    part = self.parts[pid]
    for r in range(self.h):
        for c in range(self.w):
            if self.pid[r][c] != pid:
                continue
            nb = (get(c, r - 1), get(c - 1, r), get(c + 1, r), get(c, r + 1))
            if part["outline"] and any(n is not None and n != pid for n in nb):
                self.key[r][c] = "K"
                continue
            lv = self._level(part, c, r)
            up, left, right, down = nb
            if part.get("bevel", True):
                if right != pid or down != pid:
                    lv = max(0, lv - 1)
                elif up != pid or left != pid:
                    lv = min(len(part["ramp"]) - 1, lv + 2)
            self.key[r][c] = part["ramp"][max(0, min(len(part["ramp"]) - 1, lv))]


Raster.render_part = _render_part


def build_yoke(rs: Raster) -> None:
    arm = rs.poly([(146, 64), (152, 76), (200, 53), (196, 43)], IRON, level=3, outline=True)
    ax, ay = AXLE[0] + 0.5, AXLE[1] + 0.5       # centre of the axle pixel (the wheel's centre pixel)
    rs.disc(ax, ay, 6.5, BRASS, outline=True)
    hub = rs.disc(ax, ay, 2.6, IRON, outline=True)
    lamp = rs.poly([(166, 66), (176, 66), (177, 73), (165, 73)], IRON, outline=True)
    glass = rs.rect(176, 67, 179, 72, ("q", "q", "z", "z", "z"), level=3, outline=False, bevel=False)
    rs.render()
    rs.rivets(155, 191, 61.5, 7, only_on={arm})
    rs.put(ax, ay, "7", {hub})
    rs.put(177, 70, "z", {glass})
    _ = lamp


def wheel_rows(phase_deg: float, s: float = 1.0, canvas: int | None = None) -> tuple[list[list[str]], int]:
    """Spiked wheel at a rotation phase (degrees, clockwise). Returns rows and the centre index.

    Phase 2 redesign (the lethal part is the brightest thing on the machine):

    * 12 spikes in iron 4 / 5 with a 6 face on the side towards the upper-left light, the
      outer 2 px of every spike in 7, outlined in K;
    * a 3 px rim band (5 outside, 3 inside) carrying the 12 bolt glints between the spikes;
    * a dark drum (rust Q / W) with E only as an upper-left crescent inside the rim;
    * 12 spokes, 2 px of iron 2 with a 1 px iron 4 edge on the lit side, from the brass hub
      to a 2 px iron 3 ring at r 30.

    Every feature is 12-fold symmetric, so phases 0, 7.5, 15 and 22.5 degrees loop over one
    30 degree spike period. All shading is in screen space (light from the upper left), so
    the light does not turn with the wheel.
    """
    tip, rim = WHEEL_TIP * s, WHEEL_RIM * s
    size = canvas or (2 * math.ceil(tip) + 3)
    c0 = size // 2
    rows = [["."] * size for _ in range(size)]
    inside = [[False] * size for _ in range(size)]
    lx, ly = -0.6, 0.8
    step = 360.0 / SPIKES
    phase_deg = phase_deg % step        # every feature is 12-fold: the 30 degree loop is exact
    base_half = 5.0 * s
    band_in = rim - 3.0 * s             # rim band: (rim - 3, rim]
    ring_out, ring_in = 31.0 * s, 29.0 * s
    hub = 10.0 * s
    for r in range(size):
        for c in range(size):
            x, y = c - c0, c0 - r      # pixel centres on integers: the centre pixel is the axle
            d = math.hypot(x, y)
            if d > tip + 0.2:
                continue
            ang = math.degrees(math.atan2(y, x))
            # clockwise rotation: spike k sits at 90 - phase - k*step
            rel = (ang - (90.0 - phase_deg)) % step
            delta = rel if rel <= step / 2 else rel - step
            lat = d * math.sin(math.radians(delta))
            phi = math.radians(ang - delta)                     # axis of the nearest spike / spoke
            nx, ny = -math.sin(phi), math.cos(phi)              # normal on the +delta side
            side = 1 if lat >= 0 else -1
            lit_side = side * (nx * lx + ny * ly)               # > 0: this half faces the light
            radial_lit = (x * lx + y * ly) / max(1.0, d)        # > 0: upper-left of the wheel
            key = None
            if d > rim:
                half = base_half * max(0.0, (tip - d) / (tip - rim + 1.0))
                if abs(lat) <= half + 0.35:
                    if d > tip - 2.0 * s:
                        key = "7"                               # bright tip, outer 2 px
                    elif lit_side > 0.3:
                        key = "6"                               # lit face
                    elif lit_side > -0.3 or abs(lat) < 0.8:
                        key = "5"
                    else:
                        key = "4"
            elif d > band_in:                                   # 3 px rim band
                key = "5" if d > rim - 2.0 * s else "3"
            elif d > ring_out:                                  # drum plate between ring and rim
                key = "E" if radial_lit > 0.55 else ("W" if radial_lit > -0.5 else "Q")
            elif d > ring_in:                                   # 2 px inner ring
                key = "3"
            elif d > hub:                                       # spokes over the dark drum
                if abs(lat) <= 1.0:
                    key = "2"
                elif abs(lat) <= 2.0 and lit_side > 0:
                    key = "4"                                   # 1 px lit edge
                else:
                    key = "E" if (radial_lit > 0.7 and d > 22.0 * s) else ("W" if radial_lit > -0.3 else "Q")
            elif d > 7.0 * s:                                   # brass hub
                key = "S" if radial_lit > 0.3 else ("N" if radial_lit > -0.3 else "M")
            else:
                key = "M" if radial_lit > 0 else "L"
            if key is not None:
                rows[r][c] = key
                inside[r][c] = True
    # 12 bolt glints on the rim band, between the spikes, turning with the phase
    for k in range(SPIKES):
        b = math.radians(90.0 - phase_deg - (k + 0.5) * step)
        t = rim - 1.5 * s
        c, r = c0 + round(t * math.cos(b)), c0 - round(t * math.sin(b))
        rows[r][c] = "7"
    for r in range(size):
        for c in range(size):
            if not inside[r][c] and any(0 <= c + dc < size and 0 <= r + dr < size and inside[r + dr][c + dc]
                                        for dc, dr in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                rows[r][c] = "K"
    return rows, c0


def tread_rows(phase: int, s: float = 1.0) -> tuple[list[list[str]], tuple[int, int]]:
    """Tread unit at belt phase ``phase`` (0 to 8: 1 px steps of the 9 px link pitch; the
    runtime shows frame ``floor(distance) mod 9``). Increasing phase is forward travel: the
    bottom run moves back and the top run forward relative to the hull.
    Returns rows and the pivot (footprint midpoint on the contact row)."""
    ro, ri = 16.0 * s, 10.5 * s
    length = (TREAD_X[1] - TREAD_X[0]) * s
    w = math.ceil(length) + 3
    h = math.ceil(2 * ro) + 2
    cy = h - 1 - ro + 0.5          # the bottom belt row lands on the last row
    xa, xb = 1.5 + ro, w - 1.5 - ro
    rm = (ro + ri) / 2
    straight = xb - xa
    total = 2 * straight + 2 * math.pi * rm
    n_links = max(1, round(total / (9.0 * s)))
    pitch = total / n_links
    shift = phase * pitch / TREAD_PHASES
    phase_px = shift                     # belt travel of this phase, px
    rows = [["."] * w for _ in range(h)]
    inside = [[False] * w for _ in range(h)]
    road = [xa + (xb - xa) * f for f in (0.2, 0.4, 0.6, 0.8)]
    for r in range(h):
        for c in range(w):
            x, y = c + 0.5, r + 0.5
            if xa <= x <= xb:
                d = abs(y - cy)
                if y >= cy:
                    u = x - xa                                  # bottom run, moving back (rearward)
                else:
                    u = straight + math.pi * rm + (xb - x)      # top run
            else:
                cx = xa if x < xa else xb
                d = math.hypot(x - cx, y - cy)
                ang = math.atan2(y - cy, x - cx)                # screen coords: +y down
                if x > xb:   # front arc from the bottom (pi/2) up to the top (-pi/2)
                    u = straight + rm * (math.pi / 2 - ang)
                else:        # rear arc from the top (-pi/2) round to the bottom (pi/2)
                    a2 = (ang + math.pi / 2) % (2 * math.pi)    # 0 at top, increasing anticlockwise on screen
                    u = 2 * straight + math.pi * rm + rm * (math.pi - a2 if a2 <= math.pi else 0)
            if d > ro + 0.3:
                continue
            key = None
            if d >= ri:
                wpos = (u + shift) % pitch
                top_run = y < cy and xa <= x <= xb
                if d > ro - 2.0 * s:                            # grouser cleats on the outer face
                    if 2.0 * s <= wpos <= 6.5 * s:
                        key = "4" if top_run else ("2" if y >= cy else "3")
                else:
                    if wpos < 1.2 * s or wpos > pitch - 0.8 * s:
                        key = "0"                               # pin gap
                    else:
                        key = "3" if top_run else ("1" if y >= cy else "2")
                        if top_run and d > ro - 3.0 * s:
                            key = "5"
                        if not top_run and y >= cy and wpos < 2.5 * s:
                            key = "W"                           # rusted link edge
            else:
                key = "0"                                       # dark interior
                for rx in road:
                    rr = 7.0 * s
                    dd = math.hypot(x - rx, y - (cy + 2.5 * s))
                    if dd <= rr:
                        lit = (-(x - rx) * 0.6 - (y - cy - 2.5 * s) * 0.8) / max(1.0, dd)
                        key = "4" if lit > 0.4 else ("3" if lit > -0.3 else "2")
                        if dd <= 2.4 * s:
                            key = "S" if lit > 0 else "N"
                for ex in (xa, xb):
                    dd = math.hypot(x - ex, y - cy)
                    if dd <= 9.5 * s:
                        lit = (-(x - ex) * 0.6 - (y - cy) * 0.8) / max(1.0, dd)
                        key = "E" if lit > 0.3 else ("W" if lit > -0.3 else "Q")
                        if dd <= 3.0 * s:
                            key = "S" if lit > 0 else "M"
                        elif dd > 7.6 * s:
                            ang = math.degrees(math.atan2(y - cy, x - ex)) - math.degrees(phase_px / (9.5 * s))
                            key = "T" if (ang % (360.0 / 7)) < 25.0 and lit > -0.3 else key
                # track frame bar across the upper half of the road wheels, riveted
                if xa + 6 * s <= x <= xb - 6 * s and cy - 6.0 * s <= y <= cy - 1.5 * s:
                    key = "T" if y < cy - 4.8 * s else ("E" if y < cy - 2.8 * s else "W")
            if key is not None:
                rows[r][c] = key
                inside[r][c] = True
    for r in range(h):
        for c in range(w):
            if not inside[r][c] and any(0 <= c + dc < w and 0 <= r + dr < h and inside[r + dr][c + dc]
                                        for dc, dr in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                rows[r][c] = "K"
    for i in range(int(xa + 9 * s), int(xb - 8 * s), max(4, round(8 * s))):
        rr = int(cy - 4.5 * s)
        if rows[rr][i] in "TEW":
            rows[rr][i] = "7"
    # road-wheel hubs: 5 dark bolts at r 4.5 turning clockwise with the belt (1 px of travel
    # turns a 7 px wheel by 8.2 degrees, so 9 phases turn it 73.7 degrees, one 5-fold period
    # within 1.7 degrees); the bolt nearest the upper-left light carries the one 7 glint
    for rx in road:
        hub_y = cy + 2.5 * s
        theta = phase_px / (7.0 * s)
        best = None
        for k in range(5):
            a = math.radians(-90.0 + 72.0 * k) + theta          # screen coords: clockwise
            bx, by = rx + 4.5 * s * math.cos(a), hub_y + 4.5 * s * math.sin(a)
            c, r = math.floor(bx), math.floor(by)
            if 0 <= c < w and 0 <= r < h and inside[r][c]:
                rows[r][c] = "1"
                score = (bx - rx) * 0.7071 + (by - hub_y) * 0.7071   # most negative = upper-left
                if best is None or score < best[0]:
                    best = (score, c, r)
        if best is not None:
            rows[best[2]][best[1]] = "7"
    pivot = (round(w / 2) - 1, h - 1)
    return rows, pivot


# ------------------------------------------------------------------ riders (GOB-01 pixels)

def _goblin(variant: str, frame: int, layers=None) -> list[list[str]]:
    grid = pg.load_grid(GOBLIN_DIR / f"goblin_{variant}_h36.grid.json")
    if layers is None:
        return pg.composite_keys(grid, frame)
    w, h = grid["canvas"]
    out = [["."] * w for _ in range(h)]
    for layer in grid["layers"]:
        if layer not in layers:
            continue
        for y, row in enumerate(grid["frames"][frame]["cels"].get(layer, [])):
            for x, ch in enumerate(row):
                if ch != ".":
                    out[y][x] = ch
    return out


def _shift(rows: list[list[str]], dx: int, dy: int, w: int = 64, h: int = 56) -> list[list[str]]:
    out = [["."] * w for _ in range(h)]
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != "." and 0 <= x + dx < w and 0 <= y + dy < h:
                out[y + dy][x + dx] = ch
    return out


def _stamp(rows: list[list[str]], art: list[str], x0: int, y0: int, under: bool = False) -> None:
    """ASCII stamp; space keeps the pixel, ``_`` clears it. ``under`` paints empty pixels only."""
    for j, line in enumerate(art):
        for i, ch in enumerate(line):
            if ch == " ":
                continue
            if under and rows[y0 + j][x0 + i] != ".":
                continue
            rows[y0 + j][x0 + i] = "." if ch == "_" else ch


SEAT_SHIFT = 9   # goblin rows move down 9: the shorts hem (row 46) lands on the seat row 55

# ---- rider construction (all coordinates are rider-canvas pixels AFTER the seat shift) ----
#
# The goblin parts come from GOB-01 key pixels, translated whole: the frame-0 head (grin), the
# frame-11 head (yell: frame 0's head moved (+2, +2) with the jaw open), the frame-16 head
# (hurt: thrown back, white eyes) and the frame-0 torso, near arm and vest. New pixels drawn here:
# the pot helmet, the engineer's arms and a few seams. No resampling, no rotation.

_HEAD0_TOP = 20


def _head0_mask() -> set[tuple[int, int]]:
    """Frame-0 head pixels (unshifted): rows 20-29 right of the ear root, rows 30-33 right of the
    vest boundary (x >= 27, 28, 29, 30)."""
    mask = set()
    for y in range(20, 34):
        x0 = 18 if y <= 29 else (27, 28, 29, 30)[y - 30]
        for x in range(x0, 64):
            mask.add((x, y))
    return mask


def _pixels(rows, keep=None) -> dict[tuple[int, int], str]:
    return {(x, y): c for y, r in enumerate(rows) for x, c in enumerate(r)
            if c != "." and (keep is None or (x, y) in keep)}


def head_px(variant: str, kind: str) -> dict[tuple[int, int], str]:
    """A GOB-01 head in frame-0 position, seat-shifted. kind: grin (frame 0) or yell (frame 11)."""
    mask = _head0_mask()
    layers = {"body", "accessory"}                   # never the pineapple (weapon layer)
    if kind == "grin":
        px = _pixels(_goblin(variant, 0, layers), mask)
    else:
        f = _goblin(variant, 11, layers)
        px = {}
        for (x, y) in mask:
            if x + 2 < 64 and f[y + 2][x + 2] != ".":
                px[(x, y)] = f[y + 2][x + 2]
        for y in range(31, 37):                      # the open jaw below the frame-0 head line
            for x in range(30, 62):
                if f[y][x] != ".":
                    px[(x - 2, y - 2)] = f[y][x]
    return {(x, y + SEAT_SHIFT): c for (x, y), c in px.items()}


def torso_px(variant: str, weapon: bool) -> dict[tuple[int, int], str]:
    """Frame-0 body without the head (vest, near arm, shorts), seat-shifted."""
    rows = _goblin(variant, 0, None if weapon else {"body", "accessory"})
    mask = _head0_mask()
    return {(x, y + SEAT_SHIFT): c for (x, y), c in _pixels(rows).items() if (x, y) not in mask}


def _canvas() -> list[list[str]]:
    return [["."] * RIDER_CANVAS[0] for _ in range(RIDER_CANVAS[1])]


def _paint(rows, px: dict, dx: int = 0, dy: int = 0, under: bool = False) -> None:
    for (x, y), c in px.items():
        X, Y = x + dx, y + dy
        if 0 <= X < RIDER_CANVAS[0] and 0 <= Y < RIDER_CANVAS[1]:
            if under and rows[Y][X] != ".":
                continue
            rows[Y][X] = c


def _outlined(fill: dict[tuple[int, int], str]) -> dict[tuple[int, int], str]:
    """Fill pixels plus a K outline on every empty 4-neighbour."""
    out = dict(fill)
    for (x, y) in fill:
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            if q not in fill:
                out.setdefault(q, "K")
    return out


def pot_px(lift: int = 0, tilt: int = 0, dx: int = 0) -> dict[tuple[int, int], str]:
    """The engineer's saucepan, upside down, jammed to the brow (phase 2 redesign).

    Flat top with a 1 px light row and a 2 px K notch on the top-front corner; a 4-row body
    shaded as a vertical cylinder (light left); a rolled lip only 1 px wider than the body on
    each side, on the rows just above the brow; a 2 px handle about 11 px long leaving the lip
    and rising 2 px toward the back, with a 1 px K hanging hole 2 px from its end.
    ``lift`` raises it, ``tilt`` moves the top rows back (tilted back), ``dx`` moves it all."""
    fill: dict[tuple[int, int], str] = {}
    shade = {27: "5", 28: "6", 29: "6", 30: "6", 36: "4", 37: "4", 38: "4", 39: "3", 40: "2"}
    for y in range(27, 31):
        for x in range(27, 41):
            fill[(x, y)] = "6" if y == 27 else shade.get(x, "5")
    for x in range(26, 42):
        fill[(x, 31)] = "6" if x <= 35 else ("5" if x <= 39 else "4")
        fill[(x, 32)] = "3" if x <= 39 else "2"
    for x in range(22, 26):                       # handle: rows 31-32 at the lip ...
        fill[(x, 31)], fill[(x, 32)] = "5", "3"
    for x in range(18, 22):                       # ... rising 1 px ...
        fill[(x, 30)], fill[(x, 31)] = "5", "3"
    for x in range(14, 18):                       # ... and 2 px at the back end
        fill[(x, 29)], fill[(x, 30)] = "5", "3"
    px = _outlined(fill)
    px[(16, 29)] = "K"                            # hanging hole, 2 px from the end
    px[(39, 27)] = px[(40, 27)] = "K"             # notch on the top-front corner
    px[(29, 28)] = "7"                            # glint
    out = {}
    for (x, y), c in px.items():
        shift = tilt if y <= 28 else 0
        out[(x + dx - shift, y - lift)] = c
    return out


def _white_eyes(px: dict) -> dict:
    return {p: ("w" if c == "e" else c) for p, c in px.items()}


# engineer arms (new pixels): the far arm pointing forward under the chin, and the arms
# reaching to the lever knob at about (40, 47)
POINT = [
    "             KKKK      ",
    "KKKKKKKKKKKKKsmmmKKKKK ",
    "sssssssssssssmmmmmmmmmK",
    "sssssssssssssmmmmKKKKK ",
    "KKKKKKKKKKKKKKsssK     ",
    "              KKK      ",
]
# lever pull: both forearms straight to the knob (near arm lit, far arm in shade above it)
PULL_ARMS = [
    "  KKKKKKKK   ",
    " KssssssssKK ",
    "KKKKKKKKsssK ",
    "KllmmmmmmmmmK",
    "KmmmmmmmmsmmK",
    " KKKKKKKKKKK ",
]


def _stamp_px(art: list[str], x0: int, y0: int) -> dict[tuple[int, int], str]:
    return {(x0 + i, y0 + j): ch for j, line in enumerate(art) for i, ch in enumerate(line) if ch != " "}


def rider_engineer(pose: str = "point") -> list[list[str]]:
    """Engineer keys: ``point`` (phase 1 key), ``lever_pull``, ``lurch_back``, ``lurch_forward``."""
    rows = _canvas()
    lean = {"point": 0, "lever_pull": -2, "lurch_back": -3, "lurch_forward": 2}[pose]
    drop = {"lurch_forward": 1}.get(pose, 0)
    torso = torso_px("a", weapon=False)
    yell = pose in ("lever_pull", "lurch_back")
    head = head_px("a", "yell" if yell else "grin")
    if pose == "lurch_back":
        head = _white_eyes(head)
    _paint(rows, torso, lean, drop)
    _paint(rows, head, lean, drop)
    if pose == "point":
        _paint(rows, _stamp_px(POINT, 38, 39), under=True)
        _paint(rows, pot_px())
    elif pose == "lever_pull":
        _paint(rows, _stamp_px(PULL_ARMS, 30, 42))
        _paint(rows, pot_px(tilt=1, dx=lean))
    elif pose == "lurch_back":
        _paint(rows, _stamp_px(PULL_ARMS, 29, 42))
        _paint(rows, pot_px(lift=3, tilt=1, dx=lean))     # the pot hops off his head
    else:  # lurch_forward: the pot slides down over his eyes
        _paint(rows, pot_px(lift=-3, dx=lean + 1), 0, drop)
    return rows


HIDDEN_FROM_ROW = 48   # rider-canvas rows from here down are always behind the tub (rim y 104, seat y 96)


def _clean_hidden(rows: list[list[str]]) -> list[list[str]]:
    """Despeckle only the rows the tub always hides (cropping the goblin legs leaves orphans
    there); the visible GOB-01 pixels stay byte-identical."""
    clean = despeckle([r[:] for r in rows])
    return rows[:HIDDEN_FROM_ROW] + clean[HIDDEN_FROM_ROW:]


def rider_brandish(variant: str) -> list[list[str]]:
    """GOB-01 attack-tell key (frame 11) seated: pineapple raised back, mouth open."""
    return _clean_hidden(_shift(_goblin(variant, 11), 0, SEAT_SHIFT))


def rider_hold(variant: str) -> list[list[str]]:
    """GOB-01 idle key (frame 0) seated: pineapple held upright at the rim."""
    return _clean_hidden(_shift(_goblin(variant, 0), 0, SEAT_SHIFT))


def rider_lurch(variant: str, direction: str) -> list[list[str]]:
    """Common rider lurch keys (A; B is the same with the GOB-01 B overlay).

    ``back`` (charge start): the GOB-01 hurt key (frame 16) seated: head thrown back, white eyes,
    O mouth, the pineapple arm flung up and back.
    ``forward`` (brake): the seated hold pose pitched 2 px forward and 1 px down, white eyes."""
    if direction == "back":
        return _clean_hidden(_shift(_goblin(variant, 16), 0, SEAT_SHIFT))
    rows = _canvas()
    _paint(rows, _white_eyes(_pixels(rider_hold(variant))), 2, 1)
    return _clean_hidden(rows)


RIDER_KEYS = [   # (tag, rider part, builder) in atlas order
    ("engineer_point", "engineer", lambda: despeckle(rider_engineer("point"))),
    ("engineer_lever_pull", "engineer", lambda: despeckle(rider_engineer("lever_pull"))),
    ("engineer_lurch_back", "engineer", lambda: despeckle(rider_engineer("lurch_back"))),
    ("engineer_lurch_forward", "engineer", lambda: despeckle(rider_engineer("lurch_forward"))),
    ("rider_a_brandish", "rider_a", lambda: rider_brandish("a")),
    ("rider_a_lurch_back", "rider_a", lambda: rider_lurch("a", "back")),
    ("rider_a_lurch_forward", "rider_a", lambda: rider_lurch("a", "forward")),
    ("rider_b_hold", "rider_b", lambda: rider_hold("b")),
    ("rider_b_lurch_back", "rider_b", lambda: rider_lurch("b", "back")),
    ("rider_b_lurch_forward", "rider_b", lambda: rider_lurch("b", "forward")),
]
RIDER_SEATED = {"engineer": "engineer_point", "rider_a": "rider_a_brandish", "rider_b": "rider_b_hold"}


# ------------------------------------------------------------------ assembly

def palette_json(used: set[str]) -> list[dict]:
    out = []
    for key, name, rgba, accent in ALL_PALETTE:
        if key in used:
            entry = {"key": key, "name": name, "rgba": rgba}
            if accent:
                entry["accent"] = True
            out.append(entry)
    return out


def hull_canvas(s: float) -> tuple[int, int, float, int]:
    ox = math.ceil(12 * s) + 1
    w = ox + math.ceil(160 * s) + 2
    h = math.ceil(157 * s) + 3
    return w, h, ox, h - 1


def hull_layer(layer: str, s: float = 1.0, size=None) -> Raster:
    w, h, ox, fy = size or hull_canvas(s)
    rs = Raster(w, h, ox, fy, s)
    ids = build_hull(rs, layer)
    rs.render()
    detail_hull(rs, layer, ids)
    return rs


def yoke_raster(s: float, ox: float, fy: int, w: int, h: int) -> Raster:
    rs = Raster(w, h, ox, fy, s)
    build_yoke(rs)
    return rs


def overlay(dst: list[list[str]], src: list[list[str]], dx: int, dy: int) -> None:
    for y, row in enumerate(src):
        for x, ch in enumerate(row):
            if ch != "." and 0 <= y + dy < len(dst) and 0 <= x + dx < len(dst[0]):
                dst[y + dy][x + dx] = ch


def assemble(s: float, riders: dict[str, list[list[str]]] | None = None,
             wheel_phase: float = 0.0, tread_phase: int = 0) -> tuple[list[list[str]], tuple[int, int]]:
    """The whole machine at scale ``s`` in draw order; returns rows and the pivot (rig origin O)."""
    W = math.ceil(270 * s) + 16
    H = math.ceil(158 * s) + 4
    ox, fy = math.ceil(12 * s) + 2, H - 1
    out = [["."] * W for _ in range(H)]

    def place(rows, anchor_x, anchor_y_up, piv):
        cx, cy = math.floor(ox + anchor_x * s), fy - math.floor(anchor_y_up * s)
        overlay(out, rows, cx - piv[0], cy - piv[1])

    body = hull_layer("body", s, (W, H, ox, fy))
    overlay(out, despeckle(body.key), 0, 0)
    tread, tpiv = tread_rows(tread_phase, s)
    place(despeckle(tread), O_X, 0, tpiv)
    if riders:
        for name in ("engineer", "rider_a", "rider_b"):
            sx, sy = SEATS[name]
            place(riders[name], sx, sy, RIDER_PIVOT)
    front = hull_layer("front", s, (W, H, ox, fy))
    overlay(out, despeckle(front.key), 0, 0)
    wheel, c0 = wheel_rows(wheel_phase, s)
    place(despeckle(wheel), AXLE[0], AXLE[1], (c0, c0))
    yoke = yoke_raster(s, ox=ox, fy=fy, w=W, h=H)
    overlay(out, despeckle(yoke.key), 0, 0)
    pivot = (math.floor(ox + O_X * s), fy)
    return out, pivot


def grid_from(frames: list[tuple[dict[str, list[list[str]]], int]], canvas, pivot, layers, tags) -> dict:
    used: set[str] = set()
    out_frames = []
    for cels, ms in frames:
        out_cels = {}
        for layer, rows in cels.items():
            used |= {c for r in rows for c in r if c != "."}
            out_cels[layer] = ["".join(r) for r in rows]
        out_frames.append({"duration_ms": ms, "cels": out_cels})
    return {"format": "pixel-grid-v1", "canvas": list(canvas), "pivot": list(pivot),
            "palette": palette_json(used), "layers": layers, "frames": out_frames, "tags": tags}


def riders_all() -> dict[str, list[list[str]]]:
    """The seated pose of each rider used for the assembled machine. Rider A and B frames are
    GOB-01 key pixels translated whole and never despeckled (GOB-01 frame 11 keeps its approved
    lone stick pixel); the engineer's new pixels are cleaned."""
    build = {tag: fn for tag, _part, fn in RIDER_KEYS}
    return {part: build[tag]() for part, tag in RIDER_SEATED.items()}


def size_study_grid() -> dict:
    riders = riders_all()
    big = assemble(SIZE_OPTIONS[-1] / H_KEY, riders)[0]
    W, H = len(big[0]), len(big)
    frames, pivot = [], None
    for hgt in SIZE_OPTIONS:
        rows, piv = assemble(hgt / H_KEY, riders)
        canvas = [["."] * W for _ in range(H)]
        # align every option on the same pivot: the rig origin O on the floor row
        dx = (math.floor(math.ceil(12 * SIZE_OPTIONS[-1] / H_KEY) + 2 + O_X * SIZE_OPTIONS[-1] / H_KEY)) - piv[0]
        overlay(canvas, rows, dx, (H - 1) - piv[1])
        pivot = (piv[0] + dx, H - 1)
        frames.append(({"machine": despeckle(canvas)}, 500))
    tags = [{"name": f"h{h}", "from": i, "to": i, "direction": "forward"} for i, h in enumerate(SIZE_OPTIONS)]
    return grid_from(frames, (W, H), pivot, ["machine"], tags)


def hull_grid() -> dict:
    w, h, ox, fy = hull_canvas(1.0)
    body = despeckle(hull_layer("body").key)
    front = despeckle(hull_layer("front").key)
    tags = [{"name": "body", "from": 0, "to": 0, "direction": "forward"},
            {"name": "front", "from": 1, "to": 1, "direction": "forward"}]
    return grid_from([({"body": body}, 100), ({"front": front}, 100)], (w, h),
                     (math.floor(ox + O_X), fy), ["body", "front"], tags)


def wheel_grid() -> dict:
    rows, c0 = wheel_rows(0.0)
    rows = despeckle(rows)
    return grid_from([({"wheel": rows}, 33)], (len(rows[0]), len(rows)), (c0, c0), ["wheel"],
                     [{"name": "spin", "from": 0, "to": 0, "direction": "forward"}])


def tread_grid() -> dict:
    rows, piv = tread_rows(0)
    rows = despeckle(rows)
    return grid_from([({"tread": rows}, 17)], (len(rows[0]), len(rows)), piv, ["tread"],
                     [{"name": "roll", "from": 0, "to": 0, "direction": "forward"}])


YOKE_BOX = (64, 44, -144.0, 79)   # w, h, ox, fy: rig x 144..207, y 36..79


def yoke_grid() -> dict:
    w, h, ox, fy = YOKE_BOX
    rs = yoke_raster(1.0, ox=ox, fy=fy, w=w, h=h)
    rows = despeckle(rs.key)
    ax, ay = rs.anchor_px(*AXLE)
    return grid_from([({"yoke": rows}, 100)], (w, h), (ax, ay), ["yoke"],
                     [{"name": "static", "from": 0, "to": 0, "direction": "forward"}])


def riders_grid() -> dict:
    tags = [{"name": tag, "from": i, "to": i, "direction": "forward"} for i, (tag, _p, _f) in enumerate(RIDER_KEYS)]
    return grid_from([({"rider": fn()}, 200) for _t, _p, fn in RIDER_KEYS], RIDER_CANVAS, RIDER_PIVOT,
                     ["rider"], tags)


# ------------------------------------------------------------------ smoke keys (phase 2)

SMOKE_CANVAS = (24, 24)
SMOKE_PIVOT = (12, 12)
# (centre x, centre y, radius) lumps per key; frame 0 is the 6 px spawn, frame 2 the 16 px puff
SMOKE_LUMPS = {
    "black_0": [(12.0, 12.5, 2.6), (13.5, 11.0, 1.8)],
    "black_2": [(12.0, 13.5, 5.6), (8.0, 10.5, 3.6), (15.0, 9.5, 4.2), (17.5, 14.0, 3.4), (6.5, 14.5, 2.6)],
    "steam_0": [(12.0, 12.5, 2.2), (13.2, 11.4, 1.5)],
    "steam_2": [(12.0, 13.0, 4.4), (9.0, 10.5, 3.0), (14.5, 10.0, 3.2), (16.0, 13.5, 2.6)],
}
SMOKE_RAMPS = {"black": ("+", "=", "-", ":"), "steam": (";", "/", "*", "*")}


def smoke_rows(name: str) -> list[list[str]]:
    """Round, lumpy blob with no outline: body in ramp[1], a 1 px darker underside (ramp[0]),
    an upper-left lit band (ramp[2]) and a few brightest pixels on the upper-left edge (ramp[3])."""
    w, h = SMOKE_CANVAS
    ramp = SMOKE_RAMPS[name.split("_")[0]]
    lumps = SMOKE_LUMPS[name]

    def inside(x, y):
        return any((x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r for cx, cy, r in lumps)
    rows = [["."] * w for _ in range(h)]
    for y in range(h):
        for x in range(w):
            if not inside(x, y):
                continue
            if not inside(x, y + 1):
                key = ramp[0]
            elif not inside(x - 1, y - 1) and not inside(x, y - 1):
                key = ramp[3]
            elif not inside(x - 1, y - 1) or not inside(x - 2, y - 2):
                key = ramp[2]
            else:
                key = ramp[1]
            rows[y][x] = key
    return despeckle(rows)


def smoke_grid() -> dict:
    names = ("black_0", "black_2", "steam_0", "steam_2")
    ms = {"black": round(4 * 1000 / 60), "steam": round(3 * 1000 / 60)}
    frames = [({"smoke": smoke_rows(n)}, ms[n.split("_")[0]]) for n in names]
    tags = [{"name": n, "from": i, "to": i, "direction": "forward"} for i, n in enumerate(names)]
    return grid_from(frames, SMOKE_CANVAS, SMOKE_PIVOT, ["smoke"], tags)


def rig_contract() -> dict:
    hull = hull_grid(); wheel = wheel_grid(); tread = tread_grid(); yoke = yoke_grid()

    def anchor(x, y):
        return [round(x - O_X), round(y)]
    return {
        "format": "dozer-rig-v0",
        "status": "DOZ-01 phase 2 design (Claude Opus); size h120 decided by a separate Opus agent",
        "body_height_px": H_KEY,
        "coordinates": "anchors are px relative to the rig origin O (tread footprint midpoint on the walking line); "
                       "x forward (mirror when facing left), y up. A part's pivot is placed on its anchor.",
        "parts": [
            {"part": "smoke_back", "z": 0, "atlas": "dozer_smoke", "frame_size": list(SMOKE_CANVAS),
             "pivot": list(SMOKE_PIVOT), "anchor": "emitters (world space after spawn)"},
            {"part": "hull_body", "z": 1, "atlas": "dozer_hull_h120", "tag": "body",
             "frame_size": hull["canvas"], "pivot": hull["pivot"], "anchor": [0, 0]},
            {"part": "tread", "z": 2, "atlas": "dozer_tread_h120", "tag": "roll",
             "frame_size": tread["canvas"], "pivot": tread["pivot"], "anchor": [0, 0]},
            {"part": "engineer", "z": 3, "atlas": "dozer_riders_h36", "tag": "engineer_*",
             "frame_size": list(RIDER_CANVAS), "pivot": list(RIDER_PIVOT), "anchor": anchor(*SEATS["engineer"])},
            {"part": "rider_a", "z": 3, "atlas": "dozer_riders_h36", "tag": "rider_a_*",
             "frame_size": list(RIDER_CANVAS), "pivot": list(RIDER_PIVOT), "anchor": anchor(*SEATS["rider_a"])},
            {"part": "rider_b", "z": 4, "atlas": "dozer_riders_h36", "tag": "rider_b_*",
             "frame_size": list(RIDER_CANVAS), "pivot": list(RIDER_PIVOT), "anchor": anchor(*SEATS["rider_b"])},
            {"part": "hull_front", "z": 5, "atlas": "dozer_hull_h120", "tag": "front",
             "frame_size": hull["canvas"], "pivot": hull["pivot"], "anchor": [0, 0]},
            {"part": "wheel", "z": 6, "atlas": "dozer_wheel_h120", "tag": "spin",
             "frame_size": wheel["canvas"], "pivot": wheel["pivot"], "anchor": anchor(*AXLE)},
            {"part": "yoke", "z": 7, "atlas": "dozer_yoke_h120", "tag": "static",
             "frame_size": yoke["canvas"], "pivot": yoke["pivot"], "anchor": anchor(*AXLE)},
        ],
        "occluder_rows": {name: round(RIM_Y[name]) for name in RIM_Y},
        "emitters": {k: anchor(*v) for k, v in EMITTERS.items()},
        "lights": {"lamp": anchor(*LAMP), "firebox": anchor(44, 49)},
        "extents": {"rear_x": round(-10 - O_X), "front_x": round(AXLE[0] + WHEEL_TIP - O_X),
                    "top_y_stack": 156, "dome_y": H_KEY, "tub_rim_y": 104},
    }


# ------------------------------------------------------------------ manifests

LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
           "license; no game/asset-pack distribution rights")
GENERATOR = ("Design, size study and keys: Claude Opus (DOZ-01 phases 1 and 2). Size h120 decided by a "
             "separate Opus agent. Execution pending (Claude Sonnet). Deterministic script output of "
             "scripts/author_dozer_doz01.py; riders reuse GOB-01 goblin pixels. No image-model raster output, "
             "no tracing, no resampling")
ASSETS = {
    "dozer_size_study": ("dozer_size_study_blockin_v0", "vehicle_scale_study",
                         "DOZ-01 size study: one parametric construction rasterized natively at 96, 120 and 150 px "
                         "body height, with the 36 px riders unscaled (block-in, not production pixels)"),
    "dozer_hull_h120": ("dozer_hull_h120_key_v0", "vehicle_part",
                        "DOZ-01 hull key at the proposed 120 px body height: frame body (behind the riders) and frame "
                        "front (rider tub and cab rail, over the riders)"),
    "dozer_wheel_h120": ("dozer_wheel_h120_key_v0", "vehicle_part",
                         "DOZ-01 spiked wheel key, phase 0 of the 4-frame (7.5 degree) rotation plan"),
    "dozer_tread_h120": ("dozer_tread_h120_key_v0", "vehicle_part",
                         "DOZ-01 tread key, phase 0 of the 3-frame (3 px) belt plan"),
    "dozer_yoke_h120": ("dozer_yoke_h120_key_v0", "vehicle_part",
                        "DOZ-01 near-side push arm with axle cap and lamp (static)"),
    "dozer_riders_h36": ("dozer_riders_h36_keys_v0", "character_key_poses",
                         "DOZ-01 rider keys: goblin engineer with the saucepan helmet (point, lever pull, lurch back "
                         "with the pot hopping off, lurch forward with the pot over his eyes); rider A brandishing "
                         "(GOB-01 frame 11 seated) and lurching (GOB-01 frame 16 seated; frame 0 seated pitched forward "
                         "with white eyes); rider B the same from GOB-01 variant B; goblin pixels translated whole"),
    "dozer_smoke": ("dozer_smoke_keys_v0", "vfx_keys",
                    "DOZ-01 smoke keys: black exhaust puff frames 0 and 2 and steam puff frames 0 and 2 "
                    "(lumpy blobs, no outline, darker underside)"),
}


def manifest(stem: str, grid: dict) -> dict:
    ident, kind, origin = ASSETS[stem]
    return {
        "schema_version": 1,
        "id": ident,
        "type": kind,
        "source_path": f"assets/source/enemies/dozer/{stem}.ase",
        "grid_source": f"assets/source/enemies/dozer/{stem}.grid.json",
        "output_atlas": f"generated/art/{stem}.png",
        "output_metadata": f"generated/art/{stem}.json",
        "palette_id": "dozer-floor01-v0",
        "frame_size": grid["canvas"],
        "pivot": grid["pivot"],
        "animation_tags": [t["name"] for t in grid["tags"]],
        "required_alpha": True,
        "owner": "AI-authored vehicle enemy design keys (fan-concept layer: Goblin Murder Dozer; runtime role "
                 "IP-neutral large pursuit machine)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": origin,
            "generator": GENERATOR,
            "generated_art": True,
            "input_reference_ids": ["floor-01-opening-concept-references-v1"],
        },
        "license": LICENSE,
        "review_status": "prototype_fixture_unapproved",
        "dozer": {"status": "DOZ-01 phase 2 design keys; size h120 decided by a separate Opus agent; "
                            "execution (Sonnet) and review (separate Opus) pending; not wired into any Bevy binary",
                  "spec": "docs/02-art/doz-01-dozer-spec.md"},
    }


def build_all() -> dict[Path, str]:
    grids = {"dozer_size_study": size_study_grid(), "dozer_hull_h120": hull_grid(),
             "dozer_wheel_h120": wheel_grid(), "dozer_tread_h120": tread_grid(),
             "dozer_yoke_h120": yoke_grid(), "dozer_riders_h36": riders_grid(),
             "dozer_smoke": smoke_grid()}
    out: dict[Path, str] = {}
    for stem, grid in grids.items():
        pg.validate_grid(grid)
        out[DOZER_DIR / f"{stem}.grid.json"] = json.dumps(grid, indent=1) + "\n"
        out[MANIFEST_DIR / f"{stem}.json"] = json.dumps(manifest(stem, grid), indent=2) + "\n"
    out[DOZER_DIR / "dozer_rig_h120.json"] = json.dumps(rig_contract(), indent=2) + "\n"
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--check", action="store_true", help="compare instead of writing (exit 1 on a difference)")
    parser.add_argument("--out-dir", type=Path, default=None, help="write here instead of the source tree")
    args = parser.parse_args(argv)
    status = 0
    for path, text in build_all().items():
        if args.out_dir is not None:
            path = args.out_dir / path.name
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name}: " + ("matches the build" if same else "DIFFERS from the build"))
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return status


if __name__ == "__main__":
    sys.exit(main())
