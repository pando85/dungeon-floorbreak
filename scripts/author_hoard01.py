#!/usr/bin/env python3
"""Author HOARD-01 The Hoarder (Floor 01 boss) keys as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/hoard-01-spec.md``. The keys are built from geometric rules (ellipses, tapered
capsules, polygons), shaded by one fixed light vector and finished with explicit detail pixels. No raster
image-model output, no tracing or sampling of concept pixels.

Phase 1 (silhouette gate) authors idle key 0 only. Later phases append the remaining keys to the same grid.

    python3 scripts/author_hoard01.py               # write grid + .ase
    python3 scripts/author_hoard01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_hoard01.py --scale DIR   # write key0 1x / 4x scale images next to Carl
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

HOARD_DIR = ROOT / "assets/source/characters/hoarder"
GRID = HOARD_DIR / "hoard-01.grid.json"
ASE = HOARD_DIR / "hoard-01.ase"
CARL_GRID = ROOT / "assets/source/characters/carl/carl_h52.grid.json"
CANVAS = (160, 128)
PIVOT = (70, 127)
TICK_HZ = 60

# key, name, rgba, accent (accents are exempt from the orphan rule)
PALETTE = [
    ("K", "outline", "#1c1218ff", False),
    ("d", "skin_deep", "#5c2e2aff", False),
    ("s", "skin_shade", "#94523fff", False),
    ("m", "skin_mid", "#c67c5aff", False),
    ("h", "skin_light", "#e6a982ff", False),
    ("x", "scar_dark", "#5e3442ff", False),
    ("y", "scar_light", "#8f5a5eff", False),
    ("a", "hair_deep", "#241e2cff", False),
    ("b", "hair_mid", "#3c3550ff", False),
    ("c", "hair_rim", "#5a6078ff", False),
    ("t", "shirt_shade", "#6e604eff", False),
    ("u", "shirt_mid", "#a39276ff", False),
    ("v", "shirt_light", "#d4c4a2ff", False),
    ("p", "pants_deep", "#262944ff", False),
    ("q", "pants_mid", "#3e4468ff", False),
    ("r", "pants_light", "#626a94ff", False),
    ("g", "grime", "#5a4a2cff", False),
    ("w", "eye_white", "#ece2caff", True),
    ("n", "tin_scrap", "#94a4a8ff", True),
]
ACCENTS = {p[0] for p in PALETTE if p[3]}
RAMPS = {"skin": "smh", "shirt": "tuv", "pants": "pqr", "hair": "abc"}
DARKER = {"h": "m", "m": "s", "s": "d", "d": "d", "v": "u", "u": "t", "t": "g", "r": "q", "q": "p", "p": "p",
          "c": "b", "b": "a", "a": "a", "y": "x", "x": "x", "g": "g"}
_l = (-0.5, -0.62, 0.6)
LIGHT = tuple(c / math.sqrt(sum(v * v for v in _l)) for c in _l)   # torch key upper left, same for every key

TAGS = [("idle", 0, 0)]
TICKS = [14]
KEY_FRAMES = (0,)


# --------------------------------------------------------------------------- primitives
def ellipse(cx, cy, rx, ry) -> dict:
    out = {}
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            nx, ny = (x - cx) / rx, (y - cy) / ry
            if nx * nx + ny * ny <= 1.0:
                out[(x, y)] = (nx, ny)
    return out


def capsule(x0, y0, r0, x1, y1, r1) -> dict:
    out = {}
    dx, dy = x1 - x0, y1 - y0
    l2 = dx * dx + dy * dy or 1.0
    rmax = max(r0, r1)
    for y in range(int(min(y0, y1) - rmax) - 1, int(max(y0, y1) + rmax) + 2):
        for x in range(int(min(x0, x1) - rmax) - 1, int(max(x0, x1) + rmax) + 2):
            t = max(0.0, min(1.0, ((x - x0) * dx + (y - y0) * dy) / l2))
            cx, cy, r = x0 + t * dx, y0 + t * dy, r0 + (r1 - r0) * t
            d = math.hypot(x - cx, y - cy)
            if d <= r:
                out[(x, y)] = ((x - cx) / r, (y - cy) / r)
    return out


def polygon(pts, centre, radii) -> dict:
    out = {}
    ys = [p[1] for p in pts]
    cx, cy = centre
    rx, ry = radii
    for y in range(int(min(ys)), int(max(ys)) + 1):
        xs = []
        for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
            if (ay <= y < by) or (by <= y < ay):
                xs.append(ax + (y - ay) * (bx - ax) / (by - ay))
        xs.sort()
        for a, b in zip(xs[::2], xs[1::2]):
            for x in range(math.ceil(a), math.floor(b) + 1):
                out[(x, y)] = (max(-1, min(1, (x - cx) / rx)), max(-1, min(1, (y - cy) / ry)))
    return out


def line(x0, y0, x1, y1) -> dict:
    n = max(abs(x1 - x0), abs(y1 - y0), 1)
    return {(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n)): (0.0, 0.0) for i in range(n + 1)}


def union(*shapes) -> dict:
    out = {}
    for s in shapes:
        out.update(s)
    return out


def lum(n) -> float:
    nx, ny = n
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    d = nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]
    return 0.3 + 0.7 * max(0.0, d)


# --------------------------------------------------------------------------- canvas
class Canvas:
    def __init__(self, w: int = CANVAS[0], h: int = CANVAS[1]):
        self.w, self.h = w, h
        self.owner: dict = {}    # pixel -> part order
        self.family: dict = {}   # pixel -> material family
        self.normal: dict = {}
        self.parts: list = []
        self.key: dict = {}

    def part(self, name: str, family: str, px: dict, clip=None) -> None:
        idx = len(self.parts)
        self.parts.append(name)
        for p, n in px.items():
            if not (0 <= p[0] < self.w and 0 <= p[1] < self.h):
                continue
            if clip and not clip(*p):
                continue
            self.owner[p], self.family[p], self.normal[p] = idx, family, n

    def erase(self, pred) -> None:
        for p in [p for p in self.owner if pred(*p)]:
            del self.owner[p], self.family[p], self.normal[p]

    def shade(self, hair_sheen=None) -> None:
        for p, fam in self.family.items():
            v = lum(self.normal[p])
            ramp = RAMPS[fam]
            if fam == "hair":
                k = "a"
                if v > 0.8:
                    k = "b"
                if hair_sheen and hair_sheen(*p) and v > 0.55:
                    k = "c" if v > 0.78 else "b"
            else:
                k = ramp[0] if v < 0.5 else ramp[1] if v < 0.8 else ramp[2]
            self.key[p] = k
        # cast shadow: a front part of another family up-left of a pixel darkens it one step
        snap = dict(self.key)
        for (x, y), k in snap.items():
            mine = self.owner[(x, y)]
            for q in ((x - 1, y - 1), (x, y - 1), (x, y - 2), (x - 1, y - 2)):
                o = self.owner.get(q)
                if o is not None and o > mine and self.family[q] != self.family[(x, y)]:
                    self.key[(x, y)] = DARKER[k]
                    break
        # internal separation: the front part's edge against a different family goes to the outline key
        for (x, y) in list(self.key):
            mine = self.owner[(x, y)]
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                o = self.owner.get(q)
                if o is not None and o < mine and self.family[q] != self.family[(x, y)]:
                    self.key[(x, y)] = "K"
                    break

    def put(self, x: int, y: int, k: str) -> None:
        if (x, y) in self.owner:
            self.key[(x, y)] = k

    def finish(self) -> list[str]:
        outer = {(x, y) for (x, y) in self.key
                 if any(q not in self.owner for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)))}
        for p in outer:
            self.key[p] = "K"
        # 1 px blue-grey rim on the top and back edges of the hair: reads against the dark navy arch
        for (x, y), fam in self.family.items():
            if fam == "hair" and (x, y) not in outer and self.key[(x, y)] != "K" and \
                    ((x, y - 1) in outer or (x - 1, y) in outer):
                self.key[(x, y)] = "c"
        rows = [["."] * self.w for _ in range(self.h)]
        for (x, y), k in self.key.items():
            rows[y][x] = k
        return ["".join(r) for r in despeckle(rows)]


def despeckle(rows: list[list[str]], passes: int = 4) -> list[list[str]]:
    """Replace non-accent pixels that have no same-key 8-neighbour with the commonest opaque neighbour."""
    h, w = len(rows), len(rows[0])
    for _ in range(passes):
        changed = False
        for y in range(h):
            for x in range(w):
                c = rows[y][x]
                if c == "." or c in ACCENTS:
                    continue
                around = [rows[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h]
                if c in around:
                    continue
                opaque = [n for n in around if n != "." and n not in ACCENTS]
                if opaque:
                    rows[y][x] = max(sorted(set(opaque)), key=opaque.count)
                    changed = True
        if not changed:
            break
    return rows


# --------------------------------------------------------------------------- the Hoarder, idle key 0
def hem_y(x: int) -> int:
    """Ragged shirt hem: a 6 px saw-tooth plus one deep rip at the belly front."""
    y = 95 + abs((x % 6) - 3)
    if 83 <= x <= 90:
        y = min(y, 88 + abs(x - 86))
    return y


NOTCH = (0, 2, 4, 1, 0, 3, 5, 2, 0, 1, 4, 2, 0, 3)   # ragged top of the hair: no helmet edge


def lock(*pts) -> dict:
    """A tapered hair lock through (x, y, r) points."""
    return union(*(capsule(*a, *b) for a, b in zip(pts, pts[1:])))


def idle0() -> list[str]:
    c = Canvas()
    # far side (her left arm and leg, screen right, behind the body); the far shoulder sits 2 px in
    c.part("far_arm", "skin", union(capsule(88, 46, 9, 101, 68, 8), capsule(101, 68, 8, 106, 86, 5.5),
                                    ellipse(108, 91, 5, 5),
                                    capsule(104, 94, 1.7, 102, 100, 1.4), capsule(107, 95, 1.7, 107, 102, 1.4),
                                    capsule(110, 95, 1.7, 112, 101, 1.4), capsule(113, 92, 1.7, 116, 97, 1.4),
                                    capsule(104, 88, 1.8, 101, 93, 1.5)))
    c.part("far_sleeve", "shirt", polygon([(82, 38), (96, 44), (99, 56), (95, 58), (92, 55), (88, 59), (82, 52)],
                                          (88, 46), (10, 12)))
    c.part("far_leg", "pants", union(capsule(82, 100, 13, 96, 111, 10), capsule(96, 111, 9.5, 100, 117, 8.5),
                                     ellipse(100, 116, 10.5, 3)))
    c.part("far_foot", "skin", polygon([(92, 128), (92, 121), (98, 118), (108, 119), (115, 123), (117, 128)],
                                       (102, 121), (13, 8)))
    c.part("hips", "pants", ellipse(68, 100, 30, 9))
    c.part("near_leg", "pants", union(capsule(54, 100, 14, 44, 112, 11), capsule(44, 112, 10, 40, 117, 9),
                                      ellipse(40, 116, 11.5, 3)))
    c.part("near_foot", "skin", polygon([(28, 128), (28, 122), (34, 119), (46, 118), (52, 121), (54, 128)],
                                        (40, 121), (14, 8)))
    # torso: upper-back hump, chest, belly; one plain shirt
    torso = union(ellipse(56, 46, 17, 13), ellipse(72, 52, 22, 16), ellipse(72, 77, 30, 20), ellipse(68, 91, 29, 7))
    c.part("torso", "shirt", torso, clip=lambda x, y: y <= hem_y(x))
    c.part("neckline", "skin", polygon([(80, 42), (89, 42), (88, 46), (86, 48), (85, 46), (83, 48), (81, 46)],
                                       (85, 43), (6, 4)))
    # long hair down the back, over the hump
    c.part("hair_back", "hair", union(
        polygon([(68, 18), (58, 18), (50, 23), (44, 30), (40, 40), (37, 52), (36, 62), (39, 70), (44, 66),
                 (48, 58), (54, 48), (62, 40), (70, 34)], (60, 30), (22, 26)),
        line(60, 19, 53, 16), line(55, 21, 46, 18), line(38, 64, 33, 74), line(42, 66, 40, 77),
        line(46, 60, 46, 71)))
    # near arm (her right, closest to the viewer)
    c.part("near_arm", "skin", union(capsule(48, 46, 11, 36, 70, 9), capsule(36, 70, 9, 31, 88, 6),
                                     ellipse(30, 94, 6, 5),
                                     capsule(25, 97, 1.8, 23, 104, 1.5), capsule(28, 98, 1.8, 27, 106, 1.5),
                                     capsule(32, 98, 1.8, 33, 106, 1.5), capsule(35, 96, 1.8, 37, 103, 1.5),
                                     capsule(35, 91, 1.9, 39, 96, 1.6)))
    c.part("near_sleeve", "shirt", polygon([(40, 34), (58, 36), (60, 50), (52, 60), (47, 57), (42, 61), (36, 55),
                                            (35, 44)], (50, 44), (14, 14)))
    # front locks over the far shoulder: they start behind the head and frame the far side of the face
    c.part("locks_far", "hair", union(lock((89, 34, 2.2), (91, 48, 2.0), (90, 62, 1.0)),
                                      lock((93, 24, 2.0), (96, 42, 2.2), (95, 60, 1.0)),
                                      lock((96, 32, 1.6), (98, 46, 1.6), (97, 56, 0.8))))
    # head: 24 x 23, turned about 25 degrees toward the camera, chin below the hump top
    c.part("head", "skin", union(ellipse(81, 29, 11.75, 11.5), ellipse(84, 39, 7, 3)))
    cap = ellipse(79, 27.5, 13.5, 11.5)
    c.part("hair_cap", "hair", union(
        {p: n for p, n in cap.items()
         if (p[0] < 74 - max(0, p[1] - 36) or p[1] <= 23) and p[1] >= 16 + (NOTCH[p[0] % 14] if p[1] <= 22 else 0)},
        line(70, 20, 66, 16), line(76, 18, 79, 16), line(86, 19, 88, 16),
        line(74, 24, 76, 38), line(77, 23, 78, 26)))
    # back locks: ragged, 3-5 px wide, over the near shoulder and down the back to y 78-82
    c.part("locks_back", "hair", polygon(
        [(50, 34), (64, 34), (72, 40), (73, 52), (71, 64), (69, 72), (67, 80), (65, 70), (63, 74), (61, 79),
         (59, 68), (56, 73), (54, 80), (52, 66), (49, 76), (48, 81), (47, 64), (47, 50)], (58, 44), (14, 22)))

    c.shade(hair_sheen=lambda x, y: (x * 2 + y // 3) % 9 == 0)

    # --- details (explicit pixels after shading)
    # burn: near half of the face only, dark plum scar tissue with raised rose-brown clusters, no open wound
    for (x, y) in ellipse(78, 31, 3.6, 5.6):
        if c.family.get((x, y)) == "skin" and c.key[(x, y)] != "K":
            c.key[(x, y)] = "x"
    for cx, cy in ((77, 26), (77, 30), (79, 32), (77, 34), (80, 35)):
        for x, y in ((cx, cy), (cx + 1, cy), (cx, cy + 1)):
            if c.key.get((x, y)) == "x":
                c.key[(x, y)] = "y"
    # eyes: two separate whites 3 px apart, pupils forward; no brow above the burned eye
    for x, y, k in ((79, 29, "w"), (80, 29, "w"), (81, 29, "K"), (79, 28, "K"), (80, 28, "K"), (81, 28, "K"),
                    (84, 29, "w"), (85, 29, "w"), (86, 29, "K"), (84, 28, "K"), (85, 28, "K"), (86, 28, "K"),
                    (84, 26, "K"), (85, 25, "K"), (86, 25, "K"), (87, 26, "K")):
        c.put(x, y, k)
    # nose (frontal: shadow only) and a 3 x 2 open mouth
    for x, y, k in ((87, 30, "h"), (87, 31, "h"), (87, 33, "s"), (88, 33, "s"), (86, 33, "s"),
                    (84, 36, "K"), (85, 36, "K"), (86, 36, "K"), (84, 37, "d"), (85, 37, "d"), (86, 37, "d"),
                    (84, 38, "s"), (85, 38, "s"), (86, 38, "s")):
        c.put(x, y, k)
    # two 1 px shade curves under the chest: a heavy woman's body, not a sexualised one
    for x0, x1, xc, yb in ((68, 79, 73.5, 56), (82, 91, 86.5, 55)):
        for x in range(x0, x1 + 1):
            y = yb - round(((x - xc) / ((x1 - x0) / 2)) ** 2 * 3)
            if c.key.get((x, y)) in ("u", "v"):
                c.key[(x, y)] = "t"
    # grime stains on the plain shirt (no print, no logo)
    for sx, sy, sw, sh in ((60, 70, 3, 2), (76, 63, 2, 3), (90, 74, 3, 2), (64, 84, 4, 2), (54, 60, 2, 2),
                           (95, 58, 2, 2), (72, 90, 2, 2)):
        for y in range(sy, sy + sh):
            for x in range(sx, sx + sw):
                if c.key.get((x, y)) in ("t", "u", "v"):
                    c.key[(x, y)] = "g"
    # side tear in the shirt showing skin
    tear = polygon([(88, 80), (95, 77), (97, 80), (93, 84), (89, 85)], (92, 81), (5, 4))
    for (x, y) in tear:
        if c.family.get((x, y)) == "shirt":
            c.key[(x, y)] = "s" if y > 81 else "m"
    for (x, y) in polygon([(86, 80), (95, 75), (98, 80), (94, 86), (88, 86)], (92, 81), (6, 5)):
        if c.family.get((x, y)) == "shirt" and (x, y) not in tear:
            c.key[(x, y)] = "K"
    # filth drips from the hem onto the pants
    for x, y0, n in ((50, 97, 4), (74, 98, 4), (93, 96, 3), (60, 99, 3)):
        for y in range(y0, y0 + n):
            if c.family.get((x, y)) == "pants" and c.key.get((x, y)) != "K":
                c.key[(x, y)] = "g"
    # near knee tear
    for (x, y) in ellipse(43, 111, 3.5, 2.6):
        if c.family.get((x, y)) == "pants":
            c.key[(x, y)] = "m" if y < 111 else "s"
    # faded sweatpants side stripe, hip to ankle (2 px on the thigh, 1 px below the knee)
    for y in range(93, 116):
        xs = [x for x in range(0, 66) if c.family.get((x, y)) == "pants" and c.key.get((x, y)) != "K"]
        if xs:
            for dx in ((2, 3) if y < 106 else (2,)):
                if c.key.get((min(xs) + dx, y)) in ("p", "q", "r"):
                    c.key[(min(xs) + dx, y)] = "u"
    # ankle cuffs gathered over the feet: a crease and the cuff edge
    for x0, x1 in ((31, 49), (92, 109)):
        for x in range(x0, x1):
            c.put(x, 118, "K")
        for x in range(x0 + 3, x1 - 3, 3):
            c.put(x, 115, "p")
            c.put(x + 1, 115, "p")
    # toes
    for x in (47, 50, 52):
        c.put(x, 125, "K")
        c.put(x, 126, "K")
    for x in (109, 112, 114):
        c.put(x, 125, "K")
        c.put(x, 126, "K")
    # a scrap of tin foil snagged in the back hair (the hoard clings to her)
    for x, y in ((57, 27), (58, 27), (58, 28), (59, 28), (59, 29)):
        c.put(x, y, "n")
    return c.finish()


RENDER = {0: idle0}


# --------------------------------------------------------------------------- grid io
def palette_json() -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


def build_grid() -> dict:
    frames = []
    for i, ticks in enumerate(TICKS):
        frames.append({"duration_ms": round((ticks or 6) * 1000 / TICK_HZ), "cels": {"body": RENDER[i]()}})
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def report(rows: list[str], label: str) -> None:
    flat = [list(r) for r in rows]
    orphans, isolated = pg._stray_pixels(flat, ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    used = sorted({ch for r in rows for ch in r} - {"."})
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} keys {''.join(used)}")


# --------------------------------------------------------------------------- scale evidence
def _image(rows: list[str], palette: list[dict]):
    from PIL import Image
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in palette}
    im = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch != ".":
                im.putpixel((x, y), pal[ch])
    return im


def scale_images(grid: dict, out_dir: Path) -> dict:
    """Key 0 next to Carl (h52 frame 0) on the floor line: a 1x image and the same scene at 4x.

    The scene shows the pair twice, on a warm brick-toned panel and on a cold arch-toned panel."""
    from PIL import Image
    carl = pg.load_grid(CARL_GRID)
    carl_im = _image(["".join(r) for r in pg.composite_keys(carl, 0)], carl["palette"])
    boss_im = _image(grid["frames"][0]["cels"]["body"], grid["palette"])
    panel_w, h = 220, 140
    scene = Image.new("RGBA", (panel_w * 2, h), (0, 0, 0, 255))
    floor = h - 8
    for i, bg in enumerate(((112, 58, 40, 255), (36, 42, 60, 255))):
        x0 = i * panel_w
        scene.paste(Image.new("RGBA", (panel_w, h), bg), (x0, 0))
        scene.paste(Image.new("RGBA", (panel_w, 8), (40, 30, 30, 255)), (x0, floor + 1))
        cx = x0 + 20 + carl["pivot"][0]
        scene.alpha_composite(carl_im, (cx - carl["pivot"][0], floor - carl["pivot"][1]))
        bx = x0 + 70 + PIVOT[0]
        scene.alpha_composite(boss_im, (bx - PIVOT[0], floor - PIVOT[1]))
    out_dir.mkdir(parents=True, exist_ok=True)
    one = out_dir / "key0-scale-1x.png"
    four = out_dir / "key0-scale-4x.png"
    scene.save(one)
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    combo = Image.new("RGBA", (scene.width * 4, scene.height * 5 + 8), (20, 16, 20, 255))
    combo.alpha_composite(scene, (0, 0))
    combo.alpha_composite(scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST), (0, scene.height + 8))
    combo_path = out_dir / "key0-scale.png"
    combo.save(combo_path)
    return {"1x": one, "4x": four, "combo": combo_path}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--scale", type=Path, help="directory for the key0 scale images")
    a = ap.parse_args(argv)
    grid = build_grid()
    for f in range(len(TICKS)):
        report(grid["frames"][f]["cels"]["body"], f"frame {f:2d}")
    text = dump(grid)
    if a.scale:
        for k, p in scale_images(grid, a.scale).items():
            print(f"{k}: {p}")
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
