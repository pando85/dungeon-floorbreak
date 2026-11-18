#!/usr/bin/env python3
"""JUICE-01 authoring record: the Juicer (Floor 1 Gym troglodyte boss) palette and key poses.

Usage:

    python3 scripts/author_juice01.py frames [--check]
    python3 scripts/author_juice01.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/juicer/juice_01.grid.json`` (pixel-grid-v1, layer ``body``)
and ``assets/manifests/juice-01.json``. Phase 1 holds the idle key 0 only; the other keys follow after the
silhouette gate (``docs/02-art/juice-01-spec.md`` sections 4 and 7).

The Juicer is redrawn at its own 100 px size from the troglodyte family construction (``docs/02-art/trog-01-spec.md``
section 2b, ``scripts/author_trog01.py``): the same 15 core colours, part order, 1 px outline, face order, mop and
rope-dread hair, pointed swept-back ear, ragged loincloth, grime and dirt-caked soles. It is never scaled up from
Pygmy or Basher pixels. The boss adds round muscle volumes: every limb and muscle blob is shaded from its own surface
normal (light from the upper left, the torch side), so the mass reads as lumps and creases instead of a flat fill.

Every pose is a *rule*: thick polylines and discs rasterized at pixel centres in design units (x right from the pivot
column, y up from the contact row) plus literal face stamps. No raster image-model output, no tracing of concept
pixels. ``--check`` compares instead of writing (exit 1 on a difference). Presentation-only art tooling; nothing here
is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_trog01 as trog  # noqa: E402  (the family palette core and construction constants)

OUT_DIR = ROOT / "assets/source/characters/troglodyte/juicer"
GRID = OUT_DIR / "juice_01.grid.json"
MANIFEST = ROOT / "assets/manifests/juice-01.json"
CANVAS = (160, 136)
PIVOT = (72, 135)
H = 100                     # idle opaque height (orchestrator default, pending owner confirmation)

# trog-family-v0 core (15 colours, frozen, imported, never recoloured) plus four Juicer keys: 19 of 20.
EXTRA = [
    ("b", "boil_red", "#b3283aff", False),       # swollen boils (body detail, not an effect)
    ("v", "vein_purple", "#5d3878ff", False),    # swollen veins, 1 px strands on lit and mid skin
    ("i", "iron_deep", "#353a48ff", False),      # chain cuffs and waist chain, shade
    ("j", "iron_light", "#8b93a6ff", False),     # chain links, cold lit edge
]
PALETTE = list(trog.PALETTE) + EXTRA
RAMPS = dict(trog.RAMPS)
RAMPS["iron"] = ("K", "i", "i", "j")
PROTECTED = {k for k, _n, _c, acc in PALETTE if acc} | {"b", "q", "v", "i", "j"}


def palette_json(keys: set[str]) -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        if key in keys:
            entry = {"key": key, "name": name, "rgba": rgba}
            if accent:
                entry["accent"] = True
            out.append(entry)
    return out


class Canvas:
    """Back-to-front part canvas. Each filled pixel keeps its part id and a surface normal (from the nearest axis
    point of a thick line, or the centre of a disc) used for volume shading; polygons (cloth) have no normal and use
    the family's edge-rim rule."""

    def __init__(self):
        self.w, self.h = CANVAS
        self.px, self.bottom = PIVOT[0], PIVOT[1] - 1     # fill ends one row above the contact (outline) row
        self.part: list[list[int | None]] = [[None] * self.w for _ in range(self.h)]
        self.nrm: list[list[tuple[float, float] | None]] = [[None] * self.w for _ in range(self.h)]
        self.ramps: list[tuple] = []

    def P(self, x: float, y: float) -> tuple[float, float]:
        return (self.px + 0.5 + x, self.bottom + 1.0 - y)

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1

    def _cells(self, x0, x1, y0, y1):
        for y in range(max(0, int(y0) - 1), min(self.bottom + 1, int(y1) + 2)):
            for x in range(max(0, int(x0) - 1), min(self.w, int(x1) + 2)):
                yield x, y

    def disc(self, cx, cy, rx, ry, pid):
        cx, cy = self.P(cx, cy)
        for x, y in self._cells(cx - rx, cx + rx, cy - ry, cy + ry):
            nx, ny = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            if nx * nx + ny * ny <= 1.0:
                self.part[y][x], self.nrm[y][x] = pid, (nx, ny)

    def thick(self, pts, radii, pid):
        best: dict[tuple[int, int], tuple[float, tuple[float, float]]] = {}
        for i in range(len(pts) - 1):
            (ax, ay), (bx, by) = self.P(*pts[i]), self.P(*pts[i + 1])
            ra, rb = radii[i], radii[i + 1]
            dx, dy = bx - ax, by - ay
            ll = dx * dx + dy * dy or 1e-9
            r = max(ra, rb)
            for x, y in self._cells(min(ax, bx) - r, max(ax, bx) + r, min(ay, by) - r, max(ay, by) + r):
                qx, qy = x + 0.5, y + 0.5
                t = max(0.0, min(1.0, ((qx - ax) * dx + (qy - ay) * dy) / ll))
                rr = ra + (rb - ra) * t
                ox, oy = qx - (ax + dx * t), qy - (ay + dy * t)
                d = math.hypot(ox, oy) / rr
                if d <= 1.0 and ((x, y) not in best or d < best[(x, y)][0]):
                    best[(x, y)] = (d, (ox / rr, oy / rr))
        for (x, y), (_d, n) in best.items():
            self.part[y][x], self.nrm[y][x] = pid, n

    def poly(self, pts, pid):
        pts = [self.P(*p) for p in pts]

        def inside(qx, qy):
            c = False
            for i in range(len(pts)):
                (xa, ya), (xb, yb) = pts[i], pts[(i + 1) % len(pts)]
                if (ya > qy) != (yb > qy) and qx < xa + (qy - ya) * (xb - xa) / (yb - ya):
                    c = not c
            return c
        xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
        for x, y in self._cells(min(xs), max(xs), min(ys), max(ys)):
            if inside(x + 0.5, y + 0.5):
                self.part[y][x], self.nrm[y][x] = pid, None

    def render(self) -> list[list[str]]:
        out = [["."] * self.w for _ in range(self.h)]

        def get(x, y):
            return self.part[y][x] if 0 <= x < self.w and 0 <= y < self.h else None
        for y in range(self.h):
            for x in range(self.w):
                pid = self.part[y][x]
                if pid is None:
                    if any(get(x + dx, y + dy) is not None for dx, dy in ((0, 1), (1, 0), (0, -1), (-1, 0))):
                        out[y][x] = "K"
                    continue
                n4 = [get(x, y - 1), get(x - 1, y), get(x + 1, y), get(x, y + 1)]
                if any(n is not None and n > pid for n in n4):
                    out[y][x] = "K"
                    continue
                deep, shade, mid, light = self.ramps[pid]
                rim = n4[0] != pid or n4[1] != pid
                n = self.nrm[y][x]
                if n is None:                                   # cloth: the family edge rule
                    if rim:
                        out[y][x] = light
                    elif n4[2] != pid or n4[3] != pid or get(x + 1, y + 1) != pid:
                        out[y][x] = shade
                    else:
                        out[y][x] = mid
                    continue
                nx, ny = n
                z = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
                lam = -0.5 * nx - 0.62 * ny + 0.6 * z        # torch light from the upper left and the front
                if lam > 0.87 or (rim and lam > 0.25):
                    out[y][x] = light
                elif lam > 0.36:
                    out[y][x] = mid
                elif lam > -0.05:
                    out[y][x] = shade
                else:
                    out[y][x] = deep
        return out


def L(pts, r):
    return {"pts": pts, "r": r}


# ------------------------------------------------------------------ head, face and hair (family construction, redrawn)
SKULL_R, JAW_R = 7.5, 7.0          # head width about 0.21 H: smaller than the Basher's 0.25 H (2b: small head, big body)


def head(hx, hy, tilt=-2.0):
    return L([(hx - 6, hy - tilt), (hx, hy)], [SKULL_R, JAW_R])


# Face stamp, 15 columns; top-left at (hx - 7, hy + 6). Family order: heavy K brow, bulging bloodshot eye (white,
# red rim behind and below, pupil at the front), upper teeth, dark mouth that breaks the front outline, lower fangs,
# drool. The idle mouth is a half-open 2-row growl (canon: a roaring open mouth); the tell opens it to 4 rows.
FACES = {
    "growl": ["...............",
              "....KKKKKKK....",
              "....eewwwwK....",
              ".....ewwwKK....",
              "......ee.......",
              "...........s...",
              "...tTttTttTtt..",
              "..ooooooooooooo",
              "..ooooooooooooo",
              "...tT.tTt..tT..",
              "...w.......w...",
              "...w.......w...",
              "...........w..."],
}

# Mop and rope dreads relative to the jaw centre: the Pygmy's rules at 1.3x, rope radius 1.5 (the Basher's), six
# ropes hanging down the back over the trapezius. No point above the crown; every rope ends lower than it starts.
MOP = [(-12.5, 4), (-13, 9), (-10.5, 11.5), (-9.5, 14.5), (-7, 15), (-4.5, 17), (-2.5, 15.5), (-0.5, 16.2),
       (1.6, 13), (0.5, 10.5), (-3, 8.6), (-6.5, 6.6), (-9, 5)]
ROPES = [[(-10, 12), (-15, 10.5), (-20, 6), (-22.5, 0), (-23, -5)],
         [(-11, 8), (-15, 4.5), (-16.5, -1), (-16, -6)],
         [(-4, 14.5), (-9, 14.5), (-15, 12.5), (-19, 9)],
         [(0, 13), (2.6, 11), (3.6, 8.2)],
         [(-12.5, 6), (-17, 2), (-19, -5), (-18.5, -11)],
         [(-7, 15), (-13, 15), (-20, 12), (-25, 7), (-27, 2)],
         [(-9, 12), (-12, 5), (-12, -3), (-13, -14), (-12, -19)]]
ROPE_R = 1.5
BEADS = {0: "t", 1: "w", 4: "t"}
EAR = [(-6.5, 4.5), (-15.5, 6.5), (-9, 2)]


def hair(hx, hy):
    return {"mop": [(hx + x, hy + y) for x, y in MOP], "ropes": [[(hx + x, hy + y) for x, y in r] for r in ROPES],
            "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def ragged_loin(x0, x1, y0, drop):
    """Ragged loincloth hanging down from the hip line between x0 and x1, four torn points on the hem."""
    w = x1 - x0
    return [(x0, y0), (x1, y0 - 1.5), (x1 - 0.05 * w, y0 - drop * 0.6), (x1 - 0.15 * w, y0 - drop * 0.95),
            (x1 - 0.3 * w, y0 - drop * 0.62), (x1 - 0.45 * w, y0 - drop), (x1 - 0.58 * w, y0 - drop * 0.55),
            (x1 - 0.72 * w, y0 - drop * 0.88), (x1 - 0.86 * w, y0 - drop * 0.5), (x0 + 0.02 * w, y0 - drop * 0.7)]


# ------------------------------------------------------------------ KEY 0 idle
# A hunched, wide block: short bowed legs planted wide, a huge trunk tilted forward, the trapezius hump as the top of
# the silhouette with the small head sunk in front of it, both arms hanging forward so two very large fists rest just
# off the floor ahead of the feet (the near fist in front, the far fist peeking out ahead of it). Boils crowd the
# shoulders and back; veins run down the near arm, chest and thigh; iron cuffs on both wrists with a broken chain
# hanging from the near one; a chain sags across the hips over the rag.
POSES: dict[str, dict] = {}
POSES["idle0"] = {
    "far_limbs": [
        L([(-14, 34), (-23, 18), (-21, 5), (-11, 1.5)], [10.0, 7.5, 5.0, 3.2]),             # far leg
        L([(14, 68), (28, 52), (42, 42)], [8.0, 7.0, 6.5]),                                  # far arm, behind the near one
    ],
    "far_blobs": [(47, 33, 9.0, 8.5)],                                                       # far fist, ahead and higher
    "far_cuff": [(37, 49), (43, 38)],
    "torso": L([(-8, 36), (-6, 62), (-1, 82)], [14.0, 15.0, 13.0]),
    "blobs": [(-15, 38, 12, 11),       # glute
              (-10, 68, 13, 13),       # lat / upper back
              (8, 45, 14, 13),         # gut
              (13, 63, 12, 10),        # pec
              (2, 87, 14, 11)],        # trapezius hump: the top of the silhouette
    "near_legs": [L([(0, 34), (13, 19), (10, 5), (21, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (12, 12, 4.5, 6.0),
    "loin": ragged_loin(-15, 13, 35, 21),
    "waist_chain": [(-21, 38), (-8, 33), (4, 32), (17, 35)],
    "head_at": (28, 80),
    "deltoid": (7, 73, 12, 11.5),
    "near_arm": L([(9, 70), (23, 53), (31, 38)], [10.5, 9.0, 8.0]),
    "bicep": (17, 60, 8, 9),
    "forearm": (27, 47, 8.5, 9),
    "fist": (36, 26, 11.5, 11.0),
    "thumb": [(27, 32), (34, 33), (39, 31)],
    "knuckles": [(41, 23, 46, 23), (41, 28, 46, 28), (40, 19, 44, 19)],
    "cuff": [(23.5, 36.5), (39.5, 41.5)],
    "chain": [(26, 35), (24, 27), (25, 20), (23, 13)],
    "boils": [(-3, 92, 2.6), (8, 91, 2.0), (2, 80, 3.0), (11, 69, 2.2), (-14, 75, 2.6), (-18, 62, 2.0),
              (19, 57, 2.0), (-6, 88, 1.6), (12, 24, 1.8), (13, 44, 1.6), (-20, 44, 1.8)],
    "veins": [[(27, 53), (29, 48), (27, 44), (30, 40)], [(16, 66), (19, 60), (18, 55)],
              [(8, 61), (12, 58), (13, 54)], [(6, 27), (9, 23), (11, 18)], [(3, 52), (6, 47)]],
    "grime": [(-18, 50), (-4, 70), (6, 40), (22, 44), (-12, 58), (6, 58), (14, 10), (-2, 96), (30, 20), (-22, 30)],
    "face": "growl",
}

KEY_FRAMES = {0: "idle0"}
TAGS = [("idle", [(0, 12)])]
PLAYBACK = {"idle": "loop"}
TICK_HZ = 60


def stroke(rows, c, pts, key, on):
    """1 px polyline in design units; only recolours pixels whose current key is in ``on``."""
    for i in range(len(pts) - 1):
        (ax, ay), (bx, by) = c.P(*pts[i]), c.P(*pts[i + 1])
        n = max(1, int(max(abs(bx - ax), abs(by - ay)) * 2))
        for k in range(n + 1):
            x, y = int(ax + (bx - ax) * k / n), int(ay + (by - ay) * k / n)
            if rows[y][x] in on:
                rows[y][x] = key


def build(pose: dict) -> list[str]:
    c = Canvas()
    far = c.add("far"); far_iron = c.add("iron"); skin = c.add("skin"); leg = c.add("skin"); cloth = c.add("cloth")
    waist = c.add("iron"); hd = c.add("skin"); hair_id = c.add("hair"); arm = c.add("skin"); thumb = c.add("skin")
    iron = c.add("iron"); chain = c.add("iron"); ear = c.add("skin")
    hx, hy = pose["head_at"]
    hr = hair(hx, hy)
    for limb in pose["far_limbs"]:
        c.thick(limb["pts"], limb["r"], far)
    for b in pose["far_blobs"]:
        c.disc(*b, far)
    c.thick(pose["far_cuff"], [2.2, 2.2], far_iron)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for b in pose["blobs"]:
        c.disc(*b, skin)
    for limb in pose["near_legs"]:
        c.thick(limb["pts"], limb["r"], leg)
    c.disc(*pose["calf"], leg)
    c.poly(pose["loin"], cloth)
    c.thick(pose["waist_chain"], [1.3] * len(pose["waist_chain"]), waist)
    c.thick(head(hx, hy)["pts"], head(hx, hy)["r"], hd)
    c.poly(hr["mop"], hair_id)
    for rope in hr["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    c.thick(pose["near_arm"]["pts"], pose["near_arm"]["r"], arm)
    for key in ("deltoid", "bicep", "forearm"):
        c.disc(*pose[key], arm)
    fist = c.add("skin")                       # its own part: the outline separates the fist from the forearm
    c.disc(*pose["fist"], fist)
    c.thick(pose["thumb"], [2.2, 2.0, 1.7], thumb)
    c.thick(pose["cuff"], [2.3, 2.3], iron)
    c.thick(pose["chain"], [1.1] * len(pose["chain"]), chain)
    for e in hr["ears"]:
        c.poly(e, ear)
    rows = c.render()
    # dread texture and beads (family rule)
    for y in range(c.h):
        for x in range(c.w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
    for i, key in BEADS.items():
        bx, by = c.P(*hr["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    # chain links: alternate lit and dark pairs of rows so the iron reads as links, not a rod
    for y in range(c.h):
        for x in range(c.w):
            if c.part[y][x] in (chain, waist) and rows[y][x] in "ij":
                rows[y][x] = "j" if ((x if c.part[y][x] == waist else y) // 2) % 2 == 0 else "i"
    # mottled rim (family rule): no run of more than 5 clean lit skin pixels
    for y in range(c.h):
        run = 0
        for x in range(c.w + 1):
            if x < c.w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    # knuckle grooves on the near fist
    for x0, y0, x1, y1 in pose["knuckles"]:
        stroke(rows, c, [(x0, y0), (x1, y1)], "d", "sml")
    # veins: 1 px purple strands on lit and mid skin
    for v in pose["veins"]:
        stroke(rows, c, v, "v", "mls")
    # boils: red discs on skin with a pink highlight on the torch side
    skin_parts = {skin, leg, hd, arm, fist}
    for bx, by, r in pose["boils"]:
        cx, cy = c.P(bx, by)
        for y in range(int(cy - r) - 1, int(cy + r) + 2):
            for x in range(int(cx - r) - 1, int(cx + r) + 2):
                ox, oy = x + 0.5 - cx, y + 0.5 - cy
                if ox * ox + oy * oy <= r * r and c.part[y][x] in skin_parts and rows[y][x] in "dsml":
                    rows[y][x] = "q" if (-ox - oy) / r > 0.75 else "b"
    # grime smudges and dirt-caked soles and knuckles (family rule)
    for gx, gy in pose["grime"]:
        px, py = c.P(gx, gy)
        for dx in (0, 1):
            if rows[int(py)][int(px) + dx] in "ml":
                rows[int(py)][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    # face
    fx, fy = c.P(hx - 7, hy + 6)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if rows[y][x] != "." or ch in "ewtTq":
                    rows[y][x] = ch
    despeckle(rows)
    return ["".join(r) for r in rows]


def despeckle(rows: list[list[str]]) -> None:
    """Recolour orphan fill pixels to the most common neighbour key; outline, accents and body details never move."""
    h, w = len(rows), len(rows[0])
    for _ in range(2):
        for y in range(h):
            for x in range(w):
                ch = rows[y][x]
                if ch in ".K" or ch in PROTECTED:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if ch in around:
                    continue
                cand = [a for a in around if a not in ".K"]
                if cand:
                    rows[y][x] = max(sorted(set(cand)), key=cand.count)


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def frames_grid() -> dict:
    ticks = frame_ticks()
    frames = [{"rows": build(POSES[KEY_FRAMES[n]]), "ms": round(ticks[n] * 1000 / TICK_HZ)} for n in sorted(ticks)]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS],
    }


def manifest() -> dict:
    ticks = frame_ticks()
    tag_of = {n: name for name, fr in TAGS for n, _t in fr}
    return {
        "schema_version": 1,
        "id": "troglodyte_juicer_h100_candidate_v0",
        "type": "character_animation",
        "source_path": "assets/source/characters/troglodyte/juicer/juice_01.ase",
        "grid_source": "assets/source/characters/troglodyte/juicer/juice_01.grid.json",
        "output_atlas": "generated/art/juice_01.png",
        "output_metadata": "generated/art/juice_01.json",
        "palette_id": "trog-family-v0+juicer",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [name for name, _f in TAGS],
        "required_alpha": True,
        "owner": "AI-authored boss animation candidate (fan-concept layer: Floor 1 Gym troglodyte boss; runtime role IP-neutral heavy boss)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "JUICE-01 phase 1: idle key 0 at a 100 px idle height, rasterized from pose rules by scripts/author_juice01.py; layer body only",
            "generator": "Spec and keys: Claude Opus. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "juicer": {
            "status": "phase 1: idle key 0 only, awaiting the silhouette gate; not wired into any Bevy binary",
            "spec": "docs/02-art/juice-01-spec.md",
            "opaque_height_idle": H,
            "tick_hz": TICK_HZ,
            "playback": PLAYBACK,
            "key_frames": sorted(KEY_FRAMES),
            "key_fixture": "tests/fixtures/juice-01_keys.json",
            "frames": [{"index": n, "tag": tag_of[n], "ticks": ticks[n], "duration_ms": round(ticks[n] * 1000 / TICK_HZ),
                        "author": "key"} for n in sorted(ticks)],
        },
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/juice-01/key0-scale.png") -> Path:
    """Scale evidence: key 0 at 1x on lit Floor 1 next to Carl and the TROG-01 Pygmy (640 x 360, no scaling), plus a
    4x copy of key 0 on a side panel. Needs the Floor 1 kit atlases (``bash scripts/export_assets.sh``)."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    pygmy = grid_images(load_grid_file(trog.GRID))[0]
    juicer = grid_images(load_grid_file(GRID))[0]
    p_img, p_piv = flip(pygmy, tuple(trog.PIVOT))
    j_img, j_piv = flip(juicer, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (p_img, p_piv, CARL_X + 70, 0), (j_img, j_piv, CARL_X + 190, 0)]
    scene = Floor().render(entries, 1)
    bbox = juicer.getbbox()
    crop = juicer.crop(bbox)
    big = crop.resize((crop.width * 4, crop.height * 4), Image.NEAREST)
    sheet = Image.new("RGB", (scene.width + big.width + 16, max(scene.height, big.height + 16)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    sheet.paste(big, (scene.width + 8, 8), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    targets = [(GRID, dump(frames_grid())), (MANIFEST, json.dumps(manifest(), indent=2) + "\n")]
    status = 0
    for path, text in targets:
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
