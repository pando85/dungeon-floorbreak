#!/usr/bin/env python3
"""Author the AREA-HOARDER-01 room kit (HOARD-02 props) as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/hoarder-hoard-02.md`` section 7. The design stage authors one hero key, the lair mound
in its ``sealed`` state (two layers: ``back`` drawn behind the boss, ``front`` drawn in front of her while she is
Dormant or emerging). The execution tier adds the other mound states, bank A, the J-B stack and post, the door
and the entry gate to this script on the same material rules. Colours come only from the floor01-catacomb-v2
ramps (``author_floor01_kit.RAMPS``); no new colours, no raster image-model output, no tracing.

    python3 scripts/author_hoarder_room.py            # write grid + .ase
    python3 scripts/author_hoarder_room.py --check    # exit 1 when the committed grid differs
    python3 scripts/author_hoarder_room.py --preview PNG
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402

RAMPS = kit.RAMPS
OUT = ROOT / "assets/source/props/floor01"
GRID = OUT / "hoarder_mound.grid.json"
ASE = OUT / "hoarder_mound.ase"
W, H = 176, 72
PIVOT = (88, 71)          # centre of the floor contact; placed at the boss home x 608
LIGHT = (-0.55, -0.65, 0.52)   # torch key upper left, as on the boss and the ENV-01 kit
LIP_Y = 40                # front layer: everything at or below this row in the mound's near half


def rnd(*key) -> float:
    return int(hashlib.sha256(repr(key).encode()).hexdigest()[:8], 16) / 0xFFFFFFFF


def lum(nx, ny) -> float:
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    ln = math.sqrt(sum(v * v for v in LIGHT))
    return max(0.0, (nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]) / ln)


# Material bands: (ramp, darkest index, brightest index). The hoard is kept, not rotting: wood, iron, copper,
# brass, cold glass and grey rag; a few lichen flecks only on the floor-side underlayer.
MAT = {"rag": ("floor", 1, 5), "wood": ("wood", 0, 5), "iron": ("iron", 0, 5), "copper": ("copper", 1, 6),
       "brass": ("brass", 0, 3), "glass": ("gal", 3, 9), "stone": ("stone", 1, 6), "lichen": ("lichen", 0, 2)}
OUTLINE = ("mortar", 0)


def shade(mat: str, v: float, x: int, y: int) -> tuple[str, int]:
    ramp, lo, hi = MAT[mat]
    v = max(0.0, min(1.0, v + (rnd(mat, x, y) - 0.5) * 0.12))
    return ramp, lo + round(v * (hi - lo))


class Layer:
    def __init__(self) -> None:
        self.px = [[None] * W for _ in range(H)]
        self.mat = [[None] * W for _ in range(H)]
        self.w, self.h = W, H

    def fill(self, pts, mat, normal, gain=1.0, bias=0.0) -> None:
        for (x, y) in pts:
            if 0 <= x < W and 0 <= y < H:
                n = normal(x, y)
                self.px[y][x] = shade(mat, bias + gain * lum(*n), x, y)
                self.mat[y][x] = mat


def ellipse(cx, cy, rx, ry):
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1:
                yield x, y


def box(x0, y0, w, h, ang=0.0):
    ca, sa = math.cos(math.radians(ang)), math.sin(math.radians(ang))
    cx, cy = x0 + w / 2, y0 + h / 2
    r = int(max(w, h)) + 2
    for y in range(int(cy) - r, int(cy) + r):
        for x in range(int(cx) - r, int(cx) + r):
            lx = (x - cx) * ca + (y - cy) * sa
            ly = -(x - cx) * sa + (y - cy) * ca
            if abs(lx) <= w / 2 and abs(ly) <= h / 2:
                yield x, y


def capsule(x0, y0, x1, y1, r):
    dx, dy = x1 - x0, y1 - y0
    l2 = dx * dx + dy * dy or 1.0
    for y in range(int(min(y0, y1) - r) - 1, int(max(y0, y1) + r) + 2):
        for x in range(int(min(x0, x1) - r) - 1, int(max(x0, x1) + r) + 2):
            t = max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
            if math.hypot(x - x0 - t * dx, y - y0 - t * dy) <= r:
                yield x, y


def dome_n(cx, cy, rx, ry):
    return lambda x, y: (max(-1, min(1, (x - cx) / rx)) * 0.9, max(-1, min(1, (y - cy) / ry)) * 0.9)


FLAT_UP = lambda x, y: (-0.2, -0.75)          # noqa: E731  lit top faces
FLAT_SIDE = lambda x, y: (0.35, 0.2)          # noqa: E731  shadowed front faces

# Back heap items, far to near: (kind, geometry, material). Hand-placed, read left to right at 1x.
BACK_ITEMS = [
    ("dome", (92, 76, 84, 64), "rag"),                   # the rag-and-cloth body that binds the heap
    ("plank", (22, 40, 44, 6, -28), "wood"),             # a door plank leaning out of the left flank
    ("plank", (96, 20, 40, 5, 16), "wood"),              # a broken bed rail across the crown
    ("crate", (50, 28, 26, 20, -8), "wood"),             # a crate, lid gone
    ("pot", (126, 36, 13, 10), "iron"),                  # an iron cooking pot on its side
    ("pipe", (66, 10, 90, 30, 2.5), "copper"),           # a bent copper pipe, the one thing that sticks up
    ("frame", (84, 24, 16, 20, 6), "brass"),             # an empty brass mirror frame
    ("pot", (40, 54, 9, 7), "copper"),                   # a dented copper kettle
    ("bottle", (140, 50, 4, 9), "glass"),                # cold glass bottles at the right foot
    ("bottle", (149, 56, 4, 8), "glass"),
    ("plank", (144, 34, 28, 5, 52), "wood"),             # a chair leg
    ("drape", (60, 44, 22, 8), "rag"),                   # rag folds that bury the lower half of the items
    ("drape", (112, 48, 26, 9), "rag"),
    ("drape", (92, 40, 14, 6), "rag"),
]
POKES = {"pipe"}          # only these may break the heap silhouette

# Front lip: the near ridge that hides her below row LIP_Y while Dormant and through the emerge
FRONT_ITEMS = [
    ("ridge", (88, 74, 76, 30), "rag"),
    ("crate", (30, 48, 24, 16, 10), "wood"),
    ("pot", (112, 56, 12, 8), "iron"),
    ("plank", (66, 50, 44, 5, -6), "wood"),
    ("pipe", (126, 46, 160, 60, 2.0), "copper"),
    ("bottle", (96, 58, 4, 8), "glass"),
    ("stone", (150, 62, 10, 7), "stone"),
    ("stone", (16, 64, 9, 6), "stone"),
]


def draw(layer: Layer, items) -> None:
    mask = None
    for kind, g, mat in items:
        if mask is not None and kind not in POKES:
            before = [row[:] for row in layer.px], [row[:] for row in layer.mat]
        else:
            before = None
        if kind in ("dome", "ridge"):
            cx, cy, rx, ry = g
            pts = [(x, y) for x, y in ellipse(cx, cy, rx, ry) if y < H]
            # a lumpy top edge: rag folds every few pixels
            pts = [(x, y) for x, y in pts if y >= cy - ry * math.sqrt(max(0, 1 - ((x - cx) / rx) ** 2))
                   + 2 + 2 * math.sin(x * 0.45) + 3 * rnd(kind, x // 5)]
            layer.fill(pts, mat, dome_n(cx, cy, rx, ry), gain=0.85, bias=0.05)
            # fold shadows: short dark diagonals
            for i in range(0, 2 * rx, 9):
                x = cx - rx + i + int(rnd(kind, i) * 5)
                for k in range(5):
                    y = int(cy - ry * 0.55 + rnd(kind, i, 1) * ry * 0.7) + k
                    if 0 <= y < H and 0 <= x + k < W and layer.mat[y][x + k] == mat:
                        layer.px[y][x + k] = (MAT[mat][0], MAT[mat][1])
        elif kind in ("plank", "crate"):
            x0, y0, w, h, ang = g
            pts = list(box(x0, y0, w, h, ang))
            top = min(y for _, y in pts)
            layer.fill(pts, mat, lambda x, y, t=top: FLAT_UP(x, y) if y - t < 3 else FLAT_SIDE(x, y), bias=0.1)
            if kind == "crate":   # board seams and the open top
                for x, y in pts:
                    if (x - x0) % 7 == 0 or y - top in (1,):
                        layer.px[y][x] = (MAT[mat][0], MAT[mat][1])
        elif kind == "pot":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)), mat, dome_n(cx, cy, rx, ry), gain=1.0)
            for x, y in ellipse(cx + rx * 0.35, cy, rx * 0.45, ry * 0.6):   # the dark open mouth
                if 0 <= y < H and 0 <= x < W:
                    layer.px[y][x] = (MAT[mat][0], MAT[mat][1])
        elif kind == "pipe":
            x0, y0, x1, y1, r = g
            pts = list(capsule(x0, y0, x1, y1, r))
            layer.fill(pts, mat, lambda x, y: (0.0, -0.8 if (y - y0 - (x - x0) * (y1 - y0) / max(1, x1 - x0)) < 0
                                               else 0.5), bias=0.1)
        elif kind == "frame":
            x0, y0, w, h, ang = g
            outer = set(box(x0, y0, w, h, ang))
            inner = set(box(x0 + 3, y0 + 3, w - 6, h - 6, ang))
            layer.fill(sorted(outer - inner), mat, lambda x, y: (-0.3, -0.5), bias=0.15)
            for x, y in inner:   # black void inside the frame: no reflection, no face
                if 0 <= y < H and 0 <= x < W:
                    layer.px[y][x] = ("gal", 1)
        elif kind == "bottle":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)) + list(box(cx - 1, cy - ry - 4, 2, 4)), mat,
                       dome_n(cx, cy, rx, ry), bias=0.1)
        elif kind == "stone":
            cx, cy, rx, ry = g
            layer.fill(list(ellipse(cx, cy, rx, ry)), mat, dome_n(cx, cy, rx, ry))
        elif kind == "drape":
            cx, cy, rx, ry = g
            pts = [(x, y) for x, y in ellipse(cx, cy, rx, ry) if y >= cy - ry * 0.2 + 2 * math.sin(x * 0.7)]
            layer.fill(pts, mat, dome_n(cx, cy - ry, rx, ry * 2), gain=0.7, bias=0.25)
        if kind in ("dome", "ridge") and mask is None:
            mask = {(x, y) for y in range(H) for x in range(W) if layer.px[y][x] is not None}
        elif before is not None:   # clip the item to the heap body grown by 2 px
            for y in range(H):
                for x in range(W):
                    if layer.px[y][x] != before[0][y][x] and not any(
                            (x + dx, y + dy) in mask for dx in (-2, 0, 2) for dy in (-2, 0, 2)):
                        layer.px[y][x], layer.mat[y][x] = before[0][y][x], before[1][y][x]


def outline(layer: Layer) -> None:
    edge = []
    for y in range(H):
        for x in range(W):
            if layer.px[y][x] is None:
                continue
            if any(not (0 <= x + dx < W and 0 <= y + dy < H) or layer.px[y + dy][x + dx] is None
                   for dx, dy in ((1, 0), (-1, 0), (0, -1))):
                edge.append((x, y))
    for x, y in edge:
        layer.px[y][x] = OUTLINE


def build() -> dict:
    back, front = Layer(), Layer()
    draw(back, BACK_ITEMS)
    draw(front, FRONT_ITEMS)
    for y in range(H):              # the front layer keeps only the near lip
        for x in range(W):
            if y < LIP_Y + (abs(x - PIVOT[0]) // 6):
                front.px[y][x] = None
    outline(back)
    outline(front)
    return kit.grid_from([], pivot=PIVOT, layer_names=["back", "front"], frames_layers=[{"back": back, "front": front}],
                         durations=[100], tags=[{"name": "sealed", "from": 0, "to": 0, "direction": "forward"}])


def preview(grid: dict, out: Path) -> None:
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}
    im = Image.new("RGBA", (W, H), (40, 30, 36, 255))
    for name in grid["layers"]:
        for y, row in enumerate(grid["frames"][0]["cels"][name]):
            for x, ch in enumerate(row):
                if ch != ".":
                    im.putpixel((x, y), pal[ch])
    im.resize((W * 4, H * 4), Image.NEAREST).save(out)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--preview", type=Path)
    a = ap.parse_args(argv)
    grid = build()
    text = json.dumps(grid, indent=1) + "\n"
    print(f"{len(grid['palette'])} colours")
    if a.preview:
        preview(grid, a.preview)
        return 0
    if a.check:
        ok = GRID.is_file() and GRID.read_text() == text
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    GRID.write_text(text, encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
