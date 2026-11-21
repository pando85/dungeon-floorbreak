#!/usr/bin/env python3
"""MORDECAI-01 authoring record: native Rat Hooligan guide, idle key and talk-gesture key (frozen by the editor; see docs/02-art/mordecai-01-spec.md).

Usage:

    python3 scripts/author_mordecai01.py keys [--check] [--preview PNG]
    python3 scripts/author_mordecai01.py lineup OUT_DIR

``keys`` (re)writes ``assets/source/characters/mordecai/mordecai_h44.grid.json`` and the ``.ase`` master from
parametric shapes (polygons, ellipses, thick lines), an automatic plum outline, 1 px rim light and shade rules and a few
literal pixel touches (see ``docs/02-art/mordecai-01-spec.md``). No concept pixel is traced, cropped or sampled; colours are
chosen by eye. Frames: 0 = ``idle`` key, 1 = ``talk`` key. ``lineup`` writes the unlit cast lineup (1x and 4x). Presentation-only art
tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
OUT_DIR = ROOT / "assets/source/characters/mordecai"
GRID = OUT_DIR / "mordecai_h44.grid.json"
ASE = OUT_DIR / "mordecai_h44.ase"
W, H = 52, 48
PIVOT = (24, 47)
LAYERS = ["tail", "far_arm", "torso", "near_arm", "legs", "head"]

# mordecai-floor01-v0 (19 of 20). Outline and the eye, tooth and brass accents are shared with the goblin and Carl keys.
PALETTE = [
    ("K", "outline_plum", "#1b1124ff", False),
    ("a", "fur_deep", "#3b3340ff", False),
    ("c", "fur_shade", "#6c6068ff", False),
    ("f", "fur_mid", "#998d88ff", False),
    ("g", "fur_light", "#c8bcaaff", False),
    ("p", "skin_dark", "#8a4a5aff", False),
    ("q", "skin_mid", "#c77f8aff", False),
    ("r", "skin_light", "#eaa9a9ff", False),
    ("v", "vest_dark", "#2b2231ff", False),
    ("V", "vest_mid", "#43363eff", False),
    ("u", "vest_light", "#685452ff", False),
    ("i", "indigo_dark", "#2a2c58ff", False),
    ("I", "indigo_mid", "#41498aff", False),
    ("j", "indigo_light", "#6571b0ff", False),
    ("s", "sandal_leather", "#7c5a3eff", False),
    ("e", "eye_yellow", "#ffd23aff", True),
    ("w", "tooth_white", "#efe4c4ff", True),
    ("b", "brass_mid", "#b8862eff", True),
    ("o", "mouth_dark", "#5c1426ff", False),
]
KEYS = [p[0] for p in PALETTE]
TAGS = [("idle", 0, 0), ("talk", 1, 1)]

# ------------------------------------------------------------------ rasteriser
def poly(pts):
    def hit(x, y):
        px, py, inside = x + .5, y + .5, False
        for (x1, y1), (x2, y2) in zip(pts, pts[1:] + pts[:1]):
            if (y1 > py) != (y2 > py) and px < (x2 - x1) * (py - y1) / (y2 - y1) + x1:
                inside = not inside
        return inside
    return hit

def ell(cx, cy, rx, ry):
    return lambda x, y: ((x + .5 - cx) / rx) ** 2 + ((y + .5 - cy) / ry) ** 2 <= 1

def rect(x0, y0, x1, y1):
    return lambda x, y: x0 <= x <= x1 and y0 <= y <= y1

def path(pts, w):
    def hit(x, y):
        px, py = x + .5, y + .5
        for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
            dx, dy = x2 - x1, y2 - y1
            t = max(0, min(1, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy or 1)))
            if math.hypot(px - x1 - t * dx, py - y1 - t * dy) <= w / 2:
                return True
        return False
    return hit


class Canvas:
    def __init__(self):
        self.owner = {}     # (x, y) -> shape id
        self.shapes = []    # dicts: layer, pix(set), edge
        self.px = {}        # (x, y) -> key

    def add(self, layer, fn, fill, light=None, shade=None, edge=None, name=""):
        pix = {(x, y) for y in range(H) for x in range(W) if fn(x, y)}
        sid = len(self.shapes)
        self.shapes.append({"layer": layer, "pix": pix, "edge": edge, "name": name, "z": LAYERS.index(layer), "id": sid})
        for p in pix:
            self.owner[p] = sid
            top = (p[0], p[1] - 1) not in pix or (p[0] - 1, p[1]) not in pix
            low = (p[0], p[1] + 1) not in pix or (p[0] + 1, p[1]) not in pix
            self.px[p] = light if (light and top and not low) else shade if (shade and low) else fill
        return pix

    def put(self, layer, pts, key):
        """literal pixels on a layer (overwrite whatever is there)."""
        sid = len(self.shapes)
        self.shapes.append({"layer": layer, "pix": set(pts), "edge": None, "name": "lit", "z": LAYERS.index(layer), "id": sid})
        for p in pts:
            self.owner[p] = sid
            self.px[p] = key

    def erase(self, pts):
        for p in pts:
            self.owner.pop(p, None)
            self.px.pop(p, None)

    def finish(self):
        # internal edges where a shape sits over a different, lower-order shape
        for s in self.shapes:
            if not s["edge"]:
                continue
            for p in s["pix"]:
                if self.owner.get(p) != s["id"]:
                    continue
                for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    q = (p[0] + d[0], p[1] + d[1])
                    o = self.owner.get(q)
                    if o is not None and o != s["id"] and self.shapes[o]["z"] < s["z"]:
                        self.px[p] = s["edge"]
        # outer outline on the layer of the neighbouring pixel with the highest z
        out = {}
        for (x, y), sid in self.owner.items():
            for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                q = (x + d[0], y + d[1])
                if q not in self.owner and 0 <= q[0] < W and 0 <= q[1] < H:
                    z = self.shapes[sid]["z"]
                    if q not in out or z > out[q][0]:
                        out[q] = (z, sid)
        for q, (_, sid) in out.items():
            self.owner[q] = sid
            self.px[q] = "K"
        self.despeckle()
        return self

    def despeckle(self, protect=((35, 10),)):
        acc = {"b", "e", "w"}
        for _ in range(2):
            fix = {}
            for (x, y), k in self.px.items():
                if k in acc or (x, y) in protect:
                    continue
                nb = [self.px.get((x + dx, y + dy)) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx or dy)]
                if k in nb:
                    continue
                cand = [n for n in nb if n and n != "K"] or [n for n in nb if n]
                if cand:
                    fix[(x, y)] = max(set(cand), key=cand.count)
            self.px.update(fix)

    def cels(self):
        cel = {l: [["."] * W for _ in range(H)] for l in LAYERS}
        for (x, y), sid in self.owner.items():
            cel[self.shapes[sid]["layer"]][y][x] = self.px[(x, y)]
        return {l: ["".join(r) for r in rows] for l, rows in cel.items()}


# ------------------------------------------------------------------ the character
def draw(pose: str) -> Canvas:
    c = Canvas()
    talk = pose == "talk"
    # tail (pink, curling back along the floor), two faint segment rings
    c.add("tail", path([(15, 31), (11, 33), (8, 36), (6, 40), (5, 43), (7, 46), (12, 46.5)], 2.3), "q", "r", "p")
    c.put("tail", [(9, 35), (6, 41), (9, 46)], "p")
    # far arm: a small fur elbow behind the back (kept inside the vest line so the back does not hump)
    c.add("far_arm", path([(16, 22), (13.5, 26), (15.5, 30)], 2.6), "c", "f", "a")
    # torso: scrawny fur chest and a soft pot belly in the open front of the vest
    c.add("torso", poly([(14, 19), (18, 17), (25, 17), (28, 19), (30, 23), (30.5, 28), (28.5, 31), (15, 31), (13, 26)]), "f", "g", "c")
    c.add("torso", poly([(23.5, 12.5), (19, 15), (15.5, 18.5), (23, 18.5)]), "f", "g", "c", name="nape")   # stooped nape
    c.put("torso", [(26, 19), (27, 20), (26, 21), (28, 22), (27, 23), (29, 24), (28, 25)], "g")     # chest tufts
    c.put("torso", [(27, 26), (28, 27), (29, 27), (26, 28), (27, 28)], "c")                            # belly fold
    # belt (only the front shows; the vest hangs over it at the side)
    c.add("torso", rect(14, 29, 30, 30), "v")
    c.put("torso", [(28, 29), (29, 29), (28, 30), (29, 30)], "b")
    c.put("torso", [(29, 30)], "v")
    # open vest: one faded side panel from the spine to a worn front edge, collar on top, ragged hem past the belt
    vest = c.add("torso", poly([(22, 16.5), (17.5, 16.8), (14.6, 19), (13.2, 23), (13, 27.5), (13.5, 31), (25.5, 31), (24.5, 24), (23, 19)]),
                 "V", "u", "v", name="vest")
    for y in range(18, 31):
        row = [x for x, yy in vest if yy == y]
        if row:
            c.put("torso", [(max(row), y)], "u")                                                     # worn front edge
    c.put("torso", [(23, 23), (24, 27)], "b")                                                          # brass buttons
    c.put("torso", [(15, 24), (16, 25), (14, 28)], "v")                                                # creases
    # near arm (scrawny fur arm over the vest)
    if not talk:
        c.add("near_arm", path([(19.5, 19.5), (20.5, 24.5), (22.5, 29)], 3.6), "f", "g", "c")
        c.add("near_arm", path([(22.5, 29), (25.5, 32)], 3.0), "f", None, "c", edge="c")
    else:
        c.add("near_arm", path([(19.5, 19.5), (20.5, 24.5), (23, 30)], 3.6), "f", "g", "c")
        c.add("near_arm", path([(23, 30), (29, 29.5), (33, 28.5)], 2.8), "f", "g", "c", edge="c")
        # open hand, palm up: pink palm and three separate fingers (outline gaps between them), thumb up
        c.add("near_arm", poly([(33, 27), (37, 26.5), (38, 31), (34, 31)]), "q", "r", "p")
        c.put("near_arm", [(35, 28), (36, 28)], "r")
        c.put("near_arm", [(38, 27), (39, 27), (40, 26), (41, 26)], "q")
        c.put("near_arm", [(38, 29), (39, 29), (40, 29), (41, 28)], "q")
        c.put("near_arm", [(38, 31), (39, 31), (40, 31)], "p")
        c.put("near_arm", [(41, 26), (41, 28)], "r")
        c.put("near_arm", [(37, 25), (37, 24)], "q")
    c.add("near_arm", poly([(18, 17), (22, 17), (23.5, 20), (19, 21.5)]), "V", "u", "v", name="armhole")
    # legs: baggy faded trousers, fur shins, pink rat feet in worn sandals
    c.add("legs", poly([(15, 31), (23, 31), (22, 41), (14, 41)]), "i", "I", "i", name="farleg")
    c.add("legs", poly([(22, 31), (31, 31), (33, 41), (25, 41)]), "I", "j", "i", name="nearleg")
    c.put("legs", [(14, 31), (15, 31), (16, 31), (17, 31), (19, 31), (20, 31), (15, 32), (17, 32), (20, 32)], "V")
    c.put("legs", [(14, 32), (16, 32)], "v")
    c.add("legs", rect(16, 41, 19, 44), "c")
    c.add("legs", rect(27, 41, 30, 44), "f", None, "c")
    c.add("legs", poly([(15, 44), (20, 44), (22.5, 45), (22.5, 47), (14, 47)]), "p")
    c.add("legs", poly([(26, 44), (31, 44), (35.5, 45), (35.5, 47), (25, 47)]), "q", "r", "p")
    c.put("legs", [(16, 45), (17, 45), (18, 45), (19, 45), (28, 45), (29, 45), (30, 45), (31, 45)], "v")   # straps
    c.put("legs", [(34, 46), (21, 46)], "r")                                                               # toes
    c.add("legs", rect(13, 47, 22, 47), "s")
    c.add("legs", rect(25, 47, 35, 47), "s")
    c.erase([(16, 41), (19, 41), (27, 41), (31, 41), (28, 40), (15, 41), (22, 41)])                       # ragged hem
    c.put("legs", [(27, 35), (28, 35), (28, 36), (29, 36)], "f")                                          # knee tear
    c.put("legs", [(26, 35), (29, 35), (27, 36), (30, 36), (28, 37)], "i")
    c.put("legs", [(17, 38), (18, 39)], "c")                                                              # far shin tear
    c.put("legs", [(26, 32), (27, 33), (28, 34)], "i")                                                    # pocket lip
    # head
    c.add("head", ell(29, 12.5, 6.2, 5.6), "f", "g", "c", name="skull")
    c.add("head", poly([(31, 9), (37, 10), (41, 13), (42, 15), (41, 16), (35, 17), (30, 17)]), "f", "g", "c", name="muzzle")
    c.put("head", [(31, 15), (32, 16), (33, 16), (34, 16), (30, 14)], "c")                                # cheek shade
    c.add("head", ell(26, 8.5, 3.9, 3.5), "q", "r", "p", edge="p", name="ear")
    c.put("head", [(25, 9), (26, 9), (26, 8), (25, 10), (26, 10)], "r")                                   # inner ear
    c.put("head", [(41, 14), (42, 14), (41, 15), (42, 15)], "q")
    c.put("head", [(41, 13)], "r")
    # eye, heavy lid, brow
    c.put("head", [(32, 9), (33, 9), (34, 9), (35, 9), (36, 9)], "a")
    c.put("head", [(33, 10), (34, 10), (35, 10)], "e")
    c.put("head", [(35, 10)], "K")
    c.put("head", [(33, 11), (34, 11), (35, 11)], "c")
    c.put("head", [(31, 8), (32, 8), (33, 7), (34, 7)], "a" if not talk else "c")
    if talk:
        c.put("head", [(33, 6), (34, 6), (35, 7)], "a")
    c.put("head", [(32, 10), (32, 11), (36, 11), (36, 10), (33, 12), (34, 12), (35, 12)], "c")
    # mouth
    dy = 2 if talk else 0
    if not talk:
        c.put("head", [(36, 16), (37, 16), (38, 16), (39, 16), (40, 16)], "a")
        c.put("head", [(38, 17)], "w")
    else:
        c.add("head", poly([(34, 17), (40, 17), (39, 20), (35, 20)]), "f", None, "c", name="jaw")
        c.put("head", [(36, 16), (37, 16), (38, 16), (39, 16), (40, 16), (36, 17), (37, 17), (38, 17), (39, 17)], "o")
        c.put("head", [(37, 15), (39, 15), (38, 18)], "w")
    # shaggy chin beard: grey-white strands with shaded partings and a ragged, uneven tip line
    c.add("head", poly([(29, 17 + dy), (37, 17 + dy), (36.5, 19 + dy), (35, 21 + dy), (32, 22 + dy), (29.5, 20 + dy)]), "g", None, "f", name="beard")
    c.put("head", [(31, 19 + dy), (33, 19 + dy), (34, 20 + dy), (31, 20 + dy), (32, 18 + dy)], "f")
    c.put("head", [(30, 21 + dy), (32, 23 + dy), (33, 23 + dy), (35, 22 + dy), (36, 21 + dy)], "g")
    c.put("head", [(32, 24 + dy), (34, 22 + dy)], "f")
    # scruffy crown and cheek fringe
    c.put("head", [(22, 10), (22, 12), (23, 14), (22, 15), (24, 17), (30, 5), (32, 6)], "f")
    c.put("head", [(27, 18), (28, 19)], "f")
    c.finish()
    # the beard casts a 1 px shadow on the chest fur below it, so the two light areas do not merge
    beard = {s["id"] for s in c.shapes if s["name"] == "beard"}
    torso = {s["id"] for s in c.shapes if s["layer"] == "torso"}
    for (x, y), sid in list(c.owner.items()):
        if sid in torso and c.owner.get((x, y - 1)) in beard and c.px[(x, y)] in ("f", "g"):
            c.px[(x, y)] = "c"
    # whiskers drawn after the outline pass so they stay thin light bristles, not outlined blobs
    for pts in (((43, 12), (44, 11), (45, 11)), ((44, 14), (45, 14), (46, 15)), ((43, 17), (44, 17), (45, 18))):
        sid = len(c.shapes)
        c.shapes.append({"layer": "head", "pix": set(pts), "edge": None, "name": "whisker", "z": LAYERS.index("head"), "id": sid})
        for p in pts:
            if p not in c.owner:
                c.owner[p] = sid
                c.px[p] = "g"
    return c


def build() -> dict:
    frames = []
    for pose, ms in (("idle", 667), ("talk", 1000)):          # 40 and 60 ticks at 60 Hz
        frames.append({"duration_ms": ms, "cels": draw(pose).cels()})
    return {
        "format": "pixel-grid-v1", "canvas": [W, H], "pivot": list(PIVOT),
        "palette": [{"key": k, "name": n, "rgba": h, **({"accent": True} if a else {})} for k, n, h, a in PALETTE],
        "layers": LAYERS, "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def flat(frame_cels):
    rows = [["."] * W for _ in range(H)]
    for l in LAYERS:
        for y, r in enumerate(frame_cels[l]):
            for x, ch in enumerate(r):
                if ch != ".":
                    rows[y][x] = ch
    return rows


def preview(grid, path: Path, k=10):
    from PIL import Image
    pal = {p["key"]: tuple(int(p["rgba"][i:i + 2], 16) for i in (1, 3, 5)) for p in grid["palette"]}
    n = len(grid["frames"])
    img = Image.new("RGB", ((W * k + 10) * n + 10, H * k + 10), (74, 52, 70))
    for i, f in enumerate(grid["frames"]):
        for y, r in enumerate(flat(f["cels"])):
            for x, ch in enumerate(r):
                if ch != ".":
                    for dy in range(k):
                        for dx in range(k):
                            img.putpixel((10 + i * (W * k + 10) + x * k + dx, 5 + y * k + dy), pal[ch])
    img.save(path)


def lineup(out: Path):
    """Unlit cast lineup on the Floor 1 walking line (Carl, Donut, Mordecai idle) at 1x and 4x, plus the lit scene."""
    from PIL import Image
    from pixelart import scene
    from pixelart.common import CARL_X, DONUT_X, flip, grid_images, load_grid_file
    out.mkdir(parents=True, exist_ok=True)
    grid = load_grid_file(GRID)
    imgs = grid_images(grid)
    floor = scene.Floor()
    carl, cp = scene.reference_idle("carl")
    donut, dp = scene.reference_idle("donut")
    mord, mp = flip(imgs[0], tuple(grid["pivot"]))      # faces Carl (the enemy mirror rule)
    entries = [(donut, dp, DONUT_X, 0), (carl, cp, CARL_X, 0), (mord, mp, 520, 0)]
    vx, vy = floor.view[0], floor.view[1]
    g = floor.ground(CARL_X) - vy
    for k in (1, 4):
        im = floor.render(entries, k, lit=False)
        x0, y0 = DONUT_X - vx - 34, g - 62
        im.crop((x0 * k, y0 * k, (x0 + 200) * k, (y0 + 72) * k)).save(out / f"cast_lineup_{k}x.png")
    return floor, entries, imgs


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("keys"); s.add_argument("--check", action="store_true"); s.add_argument("--preview")
    s = sub.add_parser("lineup"); s.add_argument("out")
    a = ap.parse_args(argv)
    if a.cmd == "lineup":
        lineup(Path(a.out)); return 0
    grid = build()
    text = json.dumps(grid, indent=1) + "\n"
    if a.preview:
        preview(grid, Path(a.preview))
    if a.check:
        return 0 if GRID.exists() and GRID.read_text() == text else 1
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text)
    import pixel_grid_to_ase as pg
    pg.write_ase(grid, ASE)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
