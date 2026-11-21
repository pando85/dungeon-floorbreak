#!/usr/bin/env python3
"""Authoring record for AREA-REFUGE-01 (design phase): the hearth hero key.

Writes the native pixel source of the guild-room hearth on the ``floor01-catacomb-v2``
palette (no new colours):

- ``assets/source/props/floor01/refuge_hearth.grid.json``  pixel-grid-v1, 80 x 80, pivot (40, 79),
  layers ``surround`` (static stone, firebox, logs, andirons, mantel dressing) and ``fire``
  (flames only). Frame 0 of the ``burn`` loop is the frozen hero key; frames 1-3 are execution work.
- ``assets/source/props/floor01/refuge_hearth.ase``        built from the grid by ``scripts/pixel_grid_to_ase.py``.

``scene`` composes the key at 1x in the lit ENV-02 guild view (``opening_route_proposal.json``,
``s6_guild_640x360``) with Carl and Donut at scale, adding the hearth emitters of the design
spec to the A1 lighting pass. Design spec: ``docs/02-art/area-refuge-01-spec.md``.

    python3 scripts/author_refuge01.py               # grid + .ase
    python3 scripts/author_refuge01.py scene OUT_DIR # lit previews (needs scripts/export_assets.sh)
"""
from __future__ import annotations

import hashlib
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402  (palette ramps incl. wood)

RAMPS = kit.RAMPS
GRID = ROOT / "assets/source/props/floor01/refuge_hearth.grid.json"
ASE = ROOT / "assets/source/props/floor01/refuge_hearth.ase"
W, H = 80, 80
PIVOT = (40, 79)
FIRE_C = (40, 62)            # firebox light centre, sprite-local (spec section 5)


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("refuge01",) + key)).encode()).digest()
    return h[0] / 255.0


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


class Layer:
    def __init__(self):
        self.w, self.h = W, H
        self.px = [[None] * W for _ in range(H)]

    def put(self, x, y, ramp, v):
        if 0 <= x < W and 0 <= y < H:
            self.px[y][x] = (ramp, int(clamp(round(v), 0, len(RAMPS[ramp]) - 1)))

    def get(self, x, y):
        return self.px[y][x] if 0 <= x < W and 0 <= y < H else None


def fire_light(x, y, r=30.0) -> float:
    d = math.hypot(x - FIRE_C[0], (y - FIRE_C[1]) * 1.2)
    return clamp(1.0 - d / r, 0.0, 1.0)


# --------------------------------------------------------------- surround
def ashlar(s, x0, x1, y0, y1, *, base, inner_side=0, seed="", joints=()):
    """A dressed stone block: bevel top/inner edge, dark joint bottom/outer edge, fire light."""
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            if y == y1 or x in joints:
                s.put(x, y, "mortar", 2 + 1.5 * fire_light(x, y))
                continue
            v = base + (rnd(seed, x, y) - 0.5) * 0.7
            if y == y0:
                v += 1.3
            if (inner_side < 0 and x == x0) or (inner_side > 0 and x == x1):
                v += 1.0 + 2.0 * fire_light(x, y, 40)
            if (inner_side < 0 and x == x1) or (inner_side > 0 and x == x0):
                v -= 1.2
            v += 2.2 * fire_light(x, y, 36)
            if rnd(seed, "pit", x, y) > 0.93:          # tooled pits
                v -= 1.5
            s.put(x, y, "stone", v)


def surround(s: Layer) -> None:
    # chimney hood: corbelled steps, narrowing upward (y 0..21)
    steps = [(0, 6, 18, 61), (7, 13, 14, 65), (14, 21, 10, 69)]
    for i, (y0, y1, x0, x1) in enumerate(steps):
        course = 7
        for y in range(y0, y1 + 1):
            row = (y - y0)
            off = 8 if i % 2 else 0
            for x in range(x0, x1 + 1):
                if row == course - 1 or (x - x0 + off) % 16 == 15:
                    s.put(x, y, "mortar", 2)
                    continue
                v = 4.6 + (rnd("hood", x, y) - 0.5) * 0.7 + (1.2 if row == 0 else 0)
                v -= 1.4 * clamp(1 - abs(x - 40) / 14, 0, 1) * clamp((y - 6) / 15, 0, 1)   # soot plume
                if x == x0:
                    v += 0.8
                if x == x1:
                    v -= 0.8
                s.put(x, y, "stone", v)
    # mantel shelf y 22..27, full width
    for x in range(0, 80):
        s.put(x, 22, "stone", 8 + (rnd("m", x) - 0.5) * 0.6)
        for y in (23, 24, 25):
            s.put(x, y, "stone", 6.2 + (rnd("m", x, y) - 0.5) * 0.7 - (0.8 if x in (0, 79) else 0))
        s.put(x, 26, "stone", 4 + 2.5 * fire_light(x, 40, 40))
        s.put(x, 27, "mortar", 1)
    # lintel y 28..35: two jamb caps and a centre block with a raised key
    ashlar(s, 4, 21, 28, 35, base=5.6, inner_side=1, seed="capL")
    ashlar(s, 58, 75, 28, 35, base=5.6, inner_side=-1, seed="capR")
    ashlar(s, 22, 57, 28, 35, base=5.4, seed="lintel", joints=(36, 43))
    for y in range(28, 35):
        for x in range(37, 43):
            s.put(x, y, "stone", 6.6 + (rnd("key", x, y) - 0.5) * 0.6 + (1 if y == 28 else 0))
    for x in range(22, 58):
        s.put(x, 35, "mortar", 0)                          # opening head: deep shadow line
    # jambs y 36..73, four courses
    for ci, (y0, y1) in enumerate(((36, 45), (46, 55), (56, 64), (65, 73))):
        ashlar(s, 6, 21, y0, y1, base=5.2 + 0.2 * ci, inner_side=1, seed=f"jL{ci}",
               joints=(13,) if ci % 2 else ())
        ashlar(s, 58, 73, y0, y1, base=5.2 + 0.2 * ci, inner_side=-1, seed=f"jR{ci}",
               joints=(66,) if ci % 2 else ())
    # firebox: angled reveals and a sooted small-brick back
    for y in range(36, 74):
        for x in range(22, 58):
            reveal = x < 26 or x > 53
            light = fire_light(x, y, 26)
            if reveal:
                v = 1.5 + 5.0 * light - (1 if y < 42 else 0)
                s.put(x, y, "stone", v)
                continue
            bx = (x - 26 + (4 if ((y - 36) // 4) % 2 else 0)) % 8
            if (y - 36) % 4 == 3 or bx == 7:
                s.put(x, y, "mortar", 0.4 + 2.5 * light)
            else:
                soot = clamp((46 - y) / 10, 0, 1)
                s.put(x, y, "brick", 1.6 + 7.2 * light - 1.5 * soot + (rnd("fb", x, y) - 0.5) * 0.6)
    # ember bed and ash
    for y in range(69, 74):
        for x in range(25, 55):
            r = rnd("ember", x, y)
            if r > 0.72:
                s.put(x, y, "iron", 2)
            else:
                s.put(x, y, "flame", 0.4 + 1.6 * fire_light(x, y, 18) + (r - 0.4))
    # back log (y 61..64) and front log (y 66..69)
    for y in range(61, 65):
        for x in range(28, 53):
            s.put(x, y, "wood", 3.5 - (y - 61) * 0.7 + (rnd("lb", x, y) - 0.5) * 0.8)
    for y in range(66, 70):
        for x in range(25, 55):
            v = 4.0 - (y - 66) * 0.9 + (rnd("lf", x, y) - 0.5) * 0.8
            if y == 66:
                s.put(x, y, "copper", 5 + rnd("lfe", x) * 1.5)   # top edge glowing where the flames lick
            else:
                s.put(x, y, "wood", v)
    for (ex, ey) in ((25, 67), (54, 67), (28, 62), (52, 62)):  # end grain
        s.put(ex, ey, "copper", 6)
        s.put(ex, ey + 1, "copper", 4)
    # andirons with brass knobs
    for ax in (24, 55):
        for y in (60, 61):
            s.put(ax, y, "brass", 5 if y == 60 else 4)
            s.put(ax + 1, y, "brass", 4 if y == 60 else 3)
        for y in range(62, 74):
            s.put(ax, y, "iron", 4)
            s.put(ax + 1, y, "iron", 3)
        for dx in (-1, 2):
            s.put(ax + dx, 73, "iron", 3)
    # hearthstone slab y 74..79
    for y in range(74, 80):
        for x in range(0, 80):
            if y == 74:
                v = 6.5 + 2.5 * fire_light(x, 70, 44)
            elif y == 75:
                v = 5.8 + 2.0 * fire_light(x, 70, 44)
            elif y == 79:
                v = 3
            else:
                v = 4.4 + (rnd("slab", x, y) - 0.5) * 0.6
            if y >= 76 and x in (19, 46, 70):
                s.put(x, y, "mortar", 2)
                continue
            s.put(x, y, "stone", v - (0.8 if x in (0, 79) else 0))
    # mantel dressing: clay jug (left), tin mug, drape with fringe (right)
    jug = {13: (12, 15), 14: (11, 16), 15: (10, 17), 16: (10, 17), 17: (10, 17), 18: (10, 17),
           19: (11, 16), 20: (11, 16), 21: (12, 15), 12: (12, 15), 11: (13, 14)}
    for y, (a, b) in jug.items():
        for x in range(a, b + 1):
            v = 3.2 + (x - a) / max(1, b - a) * 2.6 if y > 11 else 2.5
            s.put(x, y, "copper", v)
    s.put(18, 15, "copper", 3)
    s.put(18, 16, "copper", 3)
    s.put(18, 17, "copper", 3)
    for y in range(17, 22):
        for x in range(52, 57):
            s.put(x, y, "iron", 5 if x == 52 else (6 if x == 53 else 4))
    s.put(57, 18, "iron", 4)
    s.put(57, 19, "iron", 4)
    for x in range(63, 78):                                # drape over the mantel end
        drop = 30 + (2 if x % 3 == 0 else 0) + (1 if x > 74 else 0)
        for y in range(22, drop + 1):
            if y == drop and x % 2:
                s.put(x, y, "brick", 6)                    # fringe
                continue
            v = 5.0 + (1.2 if y == 22 else 0) - (1.0 if (x - 63) % 4 == 3 else 0) + 0.6 * fire_light(x, y, 40)
            s.put(x, y, "brick", v)


# --------------------------------------------------------------- fire (frame 0)
TONGUES = [  # base x, base y, height, half width at base, lean (px at tip)
    (40, 66, 26, 7.5, -1.0),
    (33, 66, 17, 5.0, -2.0),
    (47, 66, 19, 5.0, 2.0),
    (29, 65, 9, 3.0, -1.5),
    (51, 65, 8, 3.0, 1.0),
]


def fire(f: Layer) -> None:
    best = {}
    for bx, by, h, hw, lean in TONGUES:
        for y in range(by - h, by + 1):
            t = (by - y) / h
            half = hw * (1 - t) ** 0.75 * (1 + 0.18 * math.sin(t * 7.0 + bx))
            cx = bx + lean * t * t
            for x in range(int(cx - half - 1), int(cx + half + 2)):
                d = abs(x + 0.5 - cx) / max(half, 0.5)
                if d > 1.0:
                    continue
                if t < 0.55 and d < 0.42:
                    v = 5 if t < 0.3 and d < 0.25 else 4
                elif d < 0.72 and t < 0.75:
                    v = 3
                elif t < 0.9:
                    v = 2
                else:
                    v = 1
                if d > 0.85 and t > 0.4:
                    v = min(v, 1)
                best[(x, y)] = max(best.get((x, y), -1), v)
    for (x, y), v in best.items():
        f.put(x, y, "flame", v)
    for x, y in ((38, 36), (44, 41), (31, 44)):           # loose sparks
        f.put(x, y, "flame", 3)


def outline(s: Layer, f: Layer) -> None:
    """1 px mortar contour on the outer silhouette so the stone separates from the brick wall."""
    solid = [[s.px[y][x] is not None or f.px[y][x] is not None for x in range(W)] for y in range(H)]
    for y in range(H):
        for x in range(W):
            if not solid[y][x] or y >= 74:
                continue
            edge = any(not (0 <= x + dx < W and 0 <= y + dy < H) or not solid[y + dy][x + dx]
                       for dx, dy in ((1, 0), (-1, 0), (0, -1)))
            if edge and s.px[y][x] and s.px[y][x][0] == "stone":
                s.put(x, y, "mortar", 1)


def build() -> dict:
    s, f = Layer(), Layer()
    surround(s)
    fire(f)
    outline(s, f)
    grid = kit.grid_from([], pivot=PIVOT, layer_names=["surround", "fire"],
                        frames_layers=[{"surround": s, "fire": f}], durations=[120],
                        tags=[{"name": "burn", "from": 0, "to": 0, "direction": "forward"}])
    return grid


def write() -> None:
    import json
    grid = build()
    GRID.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} ({len(grid['palette'])} colours) and {ASE.relative_to(ROOT)}")


# --------------------------------------------------------------- lit check scene
HEARTH_LIGHTS = [  # spec section 5; offsets are sprite-local from the top-left
    {"offset": (40, 58), "colour": "#ffb060", "radius": 230, "intensity": 0.5, "squash_y": 1.0,
     "glow_radius": 26, "additive": 0.12},
    {"offset": (40, 79), "colour": "#ffb060", "radius": 150, "intensity": 0.32, "squash_y": 0.3,
     "glow_radius": 0, "additive": 0.0},
]


def scene(out: Path) -> None:
    from PIL import Image
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import grid_images, load_grid_file
    from scripts.pixelart.scene import reference_idle
    import env_kit as ek
    import env_kit_light as ekl

    layout = ek.load_json("assets/levels/floor01/opening_route_proposal.json")
    kt = ek.load_kit(layout["kit_contract"])
    view = layout["views"]["s6_guild_640x360"]
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kt, layout["tile_layers"]["collision"], cols, rows)
    hearth = grid_images(load_grid_file(GRID))[0]
    hx = view[0] + 300                                     # hearth centre, layout x
    walk = tops[hx // ek.TILE] - 1
    base = walk - 20                                       # floor back line row (deep floor)
    hpos = (hx - PIVOT[0], base - PIVOT[1])
    orig = ekl.gather_lights

    def lights(*a, **k):
        found = orig(*a, **k)
        for li in HEARTH_LIGHTS:
            found.append({"x": hpos[0] + li["offset"][0] - view[0], "y": hpos[1] + li["offset"][1] - view[1],
                          "radius": li["radius"], "colour": li["colour"], "intensity": li["intensity"],
                          "group": "scene", "squash_y": li["squash_y"], "glow_radius": li["glow_radius"],
                          "additive": li["additive"]})
        return found
    ekl.gather_lights = lights
    out.mkdir(parents=True, exist_ok=True)
    for with_cast in (False, True):
        layer = Image.new("RGBA", tuple(layout["pixel_size"]), (0, 0, 0, 0))
        layer.alpha_composite(hearth, hpos)
        boxes = []
        if with_cast:
            for name, x in (("donut", view[0] + 226), ("carl", view[0] + 258)):
                img, piv = reference_idle(name)
                pos = (x - piv[0], walk - piv[1])
                layer.alpha_composite(img, pos)
                boxes.append((pos[0], pos[1], pos[0] + img.width, pos[1] + img.height))
        tag = "cast" if with_cast else "empty"
        for k in (1, 3):
            img = ekl.render_lit(kt, layout, view, k, layer, boxes=boxes, lit=True).convert("RGB")
            img.save(out / f"refuge01_hearth_lit_{tag}_{img.width}x{img.height}.png")
        if with_cast:
            img1 = ekl.render_lit(kt, layout, view, 1, layer, boxes=boxes, lit=True).convert("RGB")
            crop = img1.crop((200, 150, 400, 300)).resize((800, 600), Image.NEAREST)
            crop.save(out / "refuge01_hearth_lit_cast_crop_4x.png")
    print(f"previews in {out}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "scene":
        scene(Path(sys.argv[2]))
    else:
        write()
