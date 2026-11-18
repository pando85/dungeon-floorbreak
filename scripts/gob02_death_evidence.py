#!/usr/bin/env python3
"""GOB-02-DEATH evidence renders (still sheets composed from the committed sources; nothing here is a Bevy capture).

    python3 scripts/gob02_death_evidence.py            # writes docs/evidence/gob-02-death/*.png

Outputs (all deterministic, seeded):

* ``preset_poses_x4.png``: every pose frame at presets 4, 3, 2 and 1 (the section 5 switches applied to the sources:
  preset 1 hides the ``gore`` layer, preset 2 remaps it ``R,P -> B``, presets 3 and 4 are as authored);
* ``preset_parts_x3.png``: every part at presets 4, 3, 2 and 1 (a part outside the preset's class filter is crossed out);
* ``parts_anchors_x4.png``: the parts sheet at x4 with every anchor marked;
* ``frame_anchor_timing.png``: pose frames with their anchors, and the tick timeline of both sequences;
* ``scenes_1x.png``: four 640 x 360 lit Floor 1 views at 1x with Carl and Donut: the crush under the ENV-02 stone,
  the decapitation tick, the headless collapse, and a scatter of parts;
* ``storyboard_crush.png``, ``storyboard_decap.png``: the section 8 spawner timelines as stills (L, L+3, L+6, L+30 and
  D, D+4, D+10, D+40), a still sheet from a small seeded simulation, not a Bevy capture;
* ``preset_strip.png``: the same crush and decapitation moments at presets 4, 3, 2 and 1.

The simulation only places pieces for stills (gravity 1050 px/s^2, the section 8 speed ranges, seeded); rotation is
shown in 90 degree steps. It feeds nothing back into the art.
"""
from __future__ import annotations

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw  # noqa: E402

import author_gob02_death as ad  # noqa: E402
from scripts.pixelart.common import CARL_X, DONUT_X, grid_images, label  # noqa: E402
from scripts.pixelart.scene import Floor, reference_idle  # noqa: E402

OUT = ROOT / "docs/evidence/gob-02-death"
G = 1050.0
DT = 1 / 60
PIV = tuple(ad.PIVOT)
CP = ad.CELL_PIVOT
PRESETS = (4, 3, 2, 1)
BRICK = (110, 64, 44)
SCALE = {4: 1.0, 3: 0.5, 2: 0.25, 1: 0.0}


# --------------------------------------------------------------------------- presets (data only)
def preset_grid(grid: dict, preset: int) -> dict:
    """Section 5 applied to a source: preset 1 hides ``gore``; preset 2 remaps its blood ``R,P -> B``; 3 and 4 as authored."""
    if preset == 1:
        return ad.strip_gore(grid)
    out = json.loads(json.dumps(grid)) if preset == 2 else grid
    if preset == 2:
        for f in out["frames"]:
            f["cels"]["gore"] = [row.translate(str.maketrans("RP", "BB")) for row in f["cels"]["gore"]]
    return out


import json  # noqa: E402


class Sprites:
    def __init__(self) -> None:
        self.parts_grid, self.poses_grid = ad.parts_grid(), ad.poses_grid()
        self.entries = {e["id"]: e for e in ad.part_entries(self.parts_grid)}
        self.cache: dict = {}

    def part(self, pid: str, preset: int) -> Image.Image:
        key = ("part", preset)
        if key not in self.cache:
            self.cache[key] = grid_images(preset_grid(self.parts_grid, preset))
        return self.cache[key][ad.PART_IDS.index(pid)]

    def pose(self, n: int, preset: int) -> Image.Image:
        key = ("pose", preset)
        if key not in self.cache:
            self.cache[key] = grid_images(preset_grid(self.poses_grid, preset))
        return self.cache[key][n]

    def allowed(self, pid: str, preset: int) -> bool:
        return preset in self.entries[pid]["presets"]

    def bottom(self, pid: str) -> int:
        return ad_bbox(self.part(pid, 4))[3]


def ad_bbox(img: Image.Image):
    x0, y0, x1, y1 = img.getbbox()
    return x0, y0, x1 - 1, y1 - 1


# --------------------------------------------------------------------------- tiny placement simulation
class Piece:
    def __init__(self, pid, x, y, vx, vy, spin, bounces, bounce, rot=0.0, swap=None):
        self.pid, self.x, self.y, self.vx, self.vy = pid, x, y, vx, vy      # world pos of the cell pivot, px/s (y up +)
        self.spin, self.rot, self.bounces, self.bounce, self.swap = spin, rot, bounces, bounce, swap
        self.rest = False
        self.hits: list[float] = []        # world x of every floor contact
        self.t = 0


def rest_y(sp: Sprites, pid: str, ground: int) -> float:
    return ground - (sp.bottom(pid) - CP[1])


def step(sp: Sprites, p: Piece, ground: int) -> None:
    if p.rest:
        return
    p.t += 1
    p.vy -= G * DT
    p.x += p.vx * DT
    p.y -= p.vy * DT
    p.rot += p.spin * DT
    floor_y = rest_y(sp, p.swap if (p.swap and p.hits) else p.pid, ground)
    if p.y >= floor_y and p.vy < 0:
        p.y = floor_y
        p.hits.append(p.x)
        if p.bounces <= 0 or abs(p.vy) < 40:
            p.rest, p.vx, p.vy, p.spin, p.rot = True, 0, 0, 0, 0
        else:
            p.vy = -p.vy * p.bounce
            p.vx *= 0.7
            p.bounces -= 1


def current_id(p: Piece) -> str:
    return p.swap if (p.swap and p.hits) else p.pid


def sprite_of(sp: Sprites, p: Piece, preset: int) -> Image.Image:
    img = sp.part(current_id(p), preset)
    quarter = round(p.rot / 90) % 4
    return img.rotate(90 * quarter) if (quarter and not p.rest) else img


# --------------------------------------------------------------------------- scenario builders
class Scene:
    def __init__(self, sp: Sprites, floor: Floor, preset: int, seed: int) -> None:
        self.sp, self.floor, self.preset = sp, floor, preset
        self.rng = random.Random(seed)
        self.ground = floor.ground(CARL_X)
        self.carl, self.carl_piv = reference_idle("carl")
        self.donut, self.donut_piv = reference_idle("donut")
        self.pieces: list[Piece] = []
        self.decals: list[tuple[str, float, float]] = []     # (part id, world x of anchor, world y of the anchor row)
        self.walls: list[tuple[str, float, float]] = []
        self.pending: list[tuple[int, callable]] = []

    def rnd(self, lo, hi):
        return self.rng.uniform(lo, hi)

    def emit(self, pid, x, y, vx, vy, spin, bounces, bounce, rot=0.0, swap=None):
        if self.sp.allowed(pid, self.preset) or swap:
            self.pieces.append(Piece(pid, x, y, vx, vy, spin, bounces, bounce, rot, swap))

    def count(self, n: float) -> int:
        return int(round(n * SCALE[self.preset]))

    def decal(self, pid, x, y=None):
        if self.sp.allowed(pid, self.preset):
            self.decals.append((pid, x, self.ground if y is None else y))

    def run(self, ticks: int, events: dict[int, list]) -> None:
        """Advance ``ticks`` ticks; ``events[t]`` are callables run before tick t's physics."""
        for t in range(ticks + 1):
            for fn in events.get(t, []):
                fn()
            for p in self.pieces:
                was = len(p.hits)
                step(self.sp, p, self.ground)
                if len(p.hits) > was and p.pid in ("drop_blood",):
                    self.decal("decal_splat_s", p.hits[-1])
                    p.dead = True
                if len(p.hits) > was and p.pid in ("head", "head_dead"):
                    self.decal("decal_splat_s", p.hits[-1])
            self.pieces = [p for p in self.pieces if not getattr(p, "dead", False)]

    # ---- drawing
    def entries(self, goblin: tuple | None, extras_back: list | None = None) -> list:
        """Draw order: wall decals, floor decals, debris, then the goblin pose and the cast. All unshadowed but the cast."""
        out = [(self.donut, self.donut_piv, DONUT_X, 0), (self.carl, self.carl_piv, CARL_X, 0)]
        for pid, wx, wy in self.walls:
            img = self.sp.part(pid, self.preset)
            anchor = self.sp.entries[pid]["anchors"]
            ax, ay = list(anchor.values())[0]
            out.append((img, (ax, ay), int(wx), int(self.ground - wy), False))
        for pid, wx, wy in self.decals:
            img = self.sp.part(pid, self.preset)
            ax, ay = list(self.sp.entries[pid]["anchors"].values())[0]
            out.append((img, (ax, ay), int(wx), 0, False))
        out += extras_back or []
        for p in self.pieces:
            img = sprite_of(self.sp, p, self.preset)
            out.append((img, CP, int(round(p.x)), int(round(self.ground - p.y)), False))
        if goblin:
            out.append(goblin)
        return out


def world_of(anchor, gx, ground, pivot=PIV):
    return gx - pivot[0] + anchor[0], ground - pivot[1] + anchor[1]


class Stage:
    """Shared scene setup: sprites, the lit floor and the ENV-02 stone."""
    def __init__(self) -> None:
        self.sp = Sprites()
        self.floor = Floor()
        self.falling, self.falling_anchor = ad.block_piece("loose_block_falling")
        self.landed, self.landed_anchor = ad.block_piece("loose_block_landed")
        self.pa = {f["index"]: f["anchors"] for f in ad.pose_manifest()["death_poses"]["frames"]}

    # -- crush at tick L + t
    def crush(self, preset: int, t: int, gx: int = CARL_X + 120, seed: int = 11) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        a0 = self.pa[0]

        def at(anchor):
            return world_of(anchor, gx, g)

        def club():
            x, y = at(a0["hand_near"])
            sc.emit("club", x, y, sc.rnd(90, 150), sc.rnd(120, 180), -sc.rnd(540, 900), 2, 0.3)

        def blood():
            for _ in range(sc.count(12)):
                for key in ("mouth", "squeeze_back", "squeeze_front"):
                    pass
            for i in range(sc.count(12)):
                key = ("mouth", "squeeze_back", "squeeze_front")[i % 3]
                x, y = at(a0[key])
                ang = __import__("math").radians(sc.rnd(10, 35))
                sp_ = sc.rnd(80, 160)
                sign = -1 if key == "squeeze_back" else 1
                sc.emit("drop_blood", x, y, sign * sp_ * __import__("math").cos(ang), sp_ * __import__("math").sin(ang), 0, 0, 0)

        def gibs():
            plan = [("gib_flesh_a", "squeeze_back"), ("gib_flesh_b", "squeeze_front"), ("gib_bone", "squeeze_front")]
            for pid, key in plan[:max(0, sc.count(3)) if sc.preset == 3 else 3]:
                x, y = at(a0[key])
                sign = -1 if key == "squeeze_back" else 1
                sc.emit(pid, x, y, sign * sc.rnd(60, 120), sc.rnd(30, 70), sign * sc.rnd(300, 600), 1, 0.45)

        def stains():
            for key in ("squeeze_back", "squeeze_front"):
                sc.decal("decal_splat_m" if sc.preset >= 3 else "decal_splat_s", at(a0[key])[0])
            sc.decal("decal_pool_1", at(self.pa[2]["pool_origin"])[0] - 28 + 28)

        events = {0: [club, blood, gibs], 4: [stains]}
        if t >= 30:
            events[30] = [lambda: self._pool(sc, 2, gx)]
        sc.run(t, events)
        if t >= 30 and preset >= 3:
            sc.decals = [d for d in sc.decals if d[0] != "decal_pool_1"]
        goblin, back = self._crush_actor(sc, t, gx)
        return self.floor.render(sc.entries(goblin, back), 1)

    def _pool(self, sc, stage, gx):
        sc.decals = [d for d in sc.decals if not d[0].startswith("decal_pool")]
        sc.decal(f"decal_pool_{stage}", world_of(self.pa[2]["pool_origin"], gx, sc.ground)[0])

    def _crush_actor(self, sc, t, gx):
        g = sc.ground
        pose = lambda n: (self.sp.pose(n, sc.preset), PIV, gx, 0)
        stone = lambda img, anc, lift: (img, anc, gx, lift, False)
        if t < 3:
            return pose(0), [stone(self.falling, self.falling_anchor, PIV[1] - 33)]
        if t < 6:
            return pose(1), [stone(self.falling, self.falling_anchor, PIV[1] - 36)]
        return pose(2), [stone(self.landed, self.landed_anchor, 0)]

    # -- decapitation at tick D + t
    def decap(self, preset: int, t: int, gx: int = CARL_X + 170, seed: int = 23) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        a3 = self.pa[3]

        def head():
            nx, ny = world_of(a3["neck"], gx, g)
            sc.emit("head", nx + (CP[0] - 11), ny + (CP[1] - 20), -sc.rnd(60, 100), sc.rnd(220, 280), 720, 2, 0.35, 20.0,
                    swap="head_dead")
            sc.pieces[-1].pid = "head"

        def jet(k):
            def fn():
                jx, jy = world_of(self.pa[4]["jet_base"], gx, g)
                n = sc.count(2) if preset != 2 else (1 if k % 4 == 0 else 0)
                for _ in range(n):
                    import math
                    ang = math.radians(90 + sc.rnd(-12, 12))
                    sp_ = sc.rnd(150, 220)
                    sc.emit("drop_blood", jx, jy, sp_ * math.cos(ang), sp_ * math.sin(ang), 0, 0, 0)
            return fn

        def trickle():
            nx, ny = world_of(self.pa[5]["neck"], gx, g)
            if sc.count(1):
                sc.emit("drop_blood", nx, ny, sc.rnd(-8, 8), 10, 0, 0, 0)

        def wall():
            jx, jy = world_of(self.pa[3]["jet_base"], gx, g)
            sc.walls.append(("decal_wall_spatter", jx - 3, g - 26)) if sc.sp.allowed("decal_wall_spatter", preset) else None

        def pool(stage):
            return lambda: self._pool(sc, stage, gx + 0)

        events: dict[int, list] = {0: [head], 4: [wall]}
        for k, tk in enumerate(range(4, 35, 3)):
            events.setdefault(tk, []).append(jet(k))
        for tk in range(10, 51, 8):
            events.setdefault(tk, []).append(trickle)
        events.setdefault(14, []).append(pool(1))
        events.setdefault(40, []).append(pool(2))
        events.setdefault(100, []).append(pool(3))
        sc.run(t, events)
        if t >= 12 and preset == 4:                 # the roll leaves a smear from the second contact
            hits = [p for p in sc.pieces if p.pid == "head"]
            if hits and len(hits[0].hits) >= 2:
                sc.decal("decal_smear", hits[0].hits[1])
        n = 3 if t < 4 else (4 if t < 10 else 5)
        # pose 4's stump moves; the pool origin for pose 5 sits on the pose's own pool_origin
        goblin = (self.sp.pose(n, preset), PIV, gx, 0)
        return self.floor.render(sc.entries(goblin), 1)

    # -- headless collapse hold and the scatter
    def scatter(self, preset: int = 4, gx: int = CARL_X + 150, seed: int = 5) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        ids = ["head_dead", "torso_upper", "torso_lower", "arm_near", "arm_far", "leg_near", "leg_far", "club_stick",
               "pineapple", "ear", "gib_flesh_a", "gib_flesh_b", "gib_bone", "gib_rag", "gib_flesh_a", "gib_rag"]
        x = gx - 40
        for pid in ids:
            if self.sp.allowed(pid, preset) or pid == "head_dead":
                p = Piece(pid, x, 0, 0, 0, 0, 0, 0)
                p.y, p.rest = rest_y(self.sp, pid, g), True
                sc.pieces.append(p)
            x += sc.rng.randint(8, 15)
        for pid, off in (("decal_pool_2", 48), ("decal_pool_1", 100), ("decal_splat_m", 80), ("decal_splat_s", 130)):
            sc.decal(pid, gx - 40 + off)
        sc.walls.append(("decal_wall_spatter", gx + 10, g - 24))
        return self.floor.render(sc.entries(None), 1)


# --------------------------------------------------------------------------- sheets
def tile(img: Image.Image, k: int, bg=BRICK) -> Image.Image:
    t = Image.new("RGB", img.size, bg)
    t.paste(img, (0, 0), img)
    return t.resize((img.width * k, img.height * k), Image.NEAREST)


def crossed(img: Image.Image) -> Image.Image:
    d = ImageDraw.Draw(img)
    d.line([(0, 0), img.size], fill=(30, 24, 30), width=2)
    d.line([(0, img.height - 1), (img.width - 1, 0)], fill=(30, 24, 30), width=2)
    return img


def preset_poses(sp: Sprites) -> Path:
    k = 4
    w, h = ad.CANVAS
    sheet = Image.new("RGB", (6 * (w * k + 6) + 70, 4 * (h * k + 6) + 24), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    label(d, (6, 4), "poses atlas: frames 0 to 5 at presets 4, 3, 2, 1 (same pixels; 1 hides gore, 2 remaps it dark; 3 = 4 for poses)")
    for r, preset in enumerate(PRESETS):
        label(d, (6, 24 + r * (h * k + 6) + h * k // 2), f"preset {preset}")
        for n in range(6):
            sheet.paste(tile(sp.pose(n, preset), k), (70 + n * (w * k + 6), 24 + r * (h * k + 6)))
    path = OUT / "preset_poses_x4.png"
    sheet.save(path)
    return path


def preset_parts(sp: Sprites) -> Path:
    k = 3
    cols = 12
    cw = 32 * k + 4
    sheet = Image.new("RGB", (cols * cw + 70, 2 * 4 * (32 * k + 4) + 40), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    label(d, (6, 4), "parts atlas at presets 4, 3, 2, 1 (crossed out: not spawned at that preset); 24 parts in two bands of 12")
    for band in range(2):
        for r, preset in enumerate(PRESETS):
            y0 = 24 + (band * 4 + r) * (32 * k + 4)
            label(d, (6, y0 + 40), f"preset {preset}")
            for c in range(cols):
                pid = ad.PART_IDS[band * cols + c]
                img = tile(sp.part(pid, preset), k)
                if not sp.allowed(pid, preset):
                    img = crossed(Image.eval(img, lambda v: int(v * 0.45)))
                sheet.paste(img, (70 + c * cw, y0))
                if r == 0:
                    label(d, (70 + c * cw + 2, y0 + 2), pid[:13], size=9)
    path = OUT / "preset_parts_x3.png"
    sheet.save(path)
    return path


ANCHOR_COLOURS = [(0, 220, 255), (255, 214, 0), (255, 90, 200), (120, 255, 120), (255, 150, 40), (180, 140, 255)]


def parts_anchors(sp: Sprites) -> Path:
    k = 4
    cols, cw = 6, 32 * 4 + 6
    rows = 4
    sheet = Image.new("RGB", (cols * cw + 6, rows * (32 * k + 22) + 6), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for i, pid in enumerate(ad.PART_IDS):
        x0, y0 = 6 + (i % cols) * cw, 6 + (i // cols) * (32 * k + 22)
        sheet.paste(tile(sp.part(pid, 4), k), (x0, y0))
        e = sp.entries[pid]
        label(d, (x0 + 1, y0 + 32 * k + 3), f"{i:02d} {pid}", size=10)
        d.rectangle([x0 + 16 * k, y0 + 16 * k, x0 + 16 * k + k - 1, y0 + 16 * k + k - 1], outline=(255, 255, 255))   # cell pivot
        for j, (name, (ax, ay)) in enumerate(e["anchors"].items()):
            c = ANCHOR_COLOURS[j % len(ANCHOR_COLOURS)]
            cx, cy = x0 + ax * k + k // 2, y0 + ay * k + k // 2
            d.line([(cx - 5, cy), (cx + 5, cy)], fill=c)
            d.line([(cx, cy - 5), (cx, cy + 5)], fill=c)
            label(d, (x0 + 2, y0 + 2 + j * 11), name, c, 9)
    path = OUT / "parts_anchors_x4.png"
    sheet.save(path)
    return path


def frame_anchor_timing(sp: Sprites) -> Path:
    k = 3
    w, h = ad.CANVAS
    man = ad.pose_manifest()["death_poses"]["frames"]
    sheet = Image.new("RGB", (6 * (w * k + 8) + 8, h * k + 190), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for n, f in enumerate(man):
        x0 = 8 + n * (w * k + 8)
        sheet.paste(tile(sp.pose(n, 4), k), (x0, 8))
        d.line([(x0, 8 + 56 * k), (x0 + w * k, 8 + 56 * k)], fill=(0, 220, 255))
        d.rectangle([x0 + 28 * k, 8 + 55 * k, x0 + 28 * k + k - 1, 8 + 55 * k + k - 1], outline=(255, 214, 0))
        label(d, (x0 + 3, 11), f"{n} {f['tag']} {'hold' if f['ticks'] is None else str(f['ticks']) + 't'} {f['author']}", size=10)
        for j, (name, (ax, ay)) in enumerate(f["anchors"].items()):
            c = ANCHOR_COLOURS[j % len(ANCHOR_COLOURS)]
            cx, cy = x0 + ax * k + k // 2, 8 + ay * k + k // 2
            d.line([(cx - 5, cy), (cx + 5, cy)], fill=c)
            d.line([(cx, cy - 5), (cx, cy + 5)], fill=c)
            label(d, (x0 + 3, 24 + j * 11), name, c, 9)
    y = h * k + 24
    label(d, (8, y), "timeline at 60 Hz (1 tick = 4 px): crush L, decapitation D; holds are state-driven", size=11)
    for row, (name, spans) in enumerate((("crush", [(0, 3, "f0 contact"), (3, 6, "f1 flatten"), (6, 60, "f2 remains hold")]),
                                         ("decap", [(0, 4, "f3 blade"), (4, 10, "f4 buckle"), (10, 60, "f5 hold")]))):
        yy = y + 22 + row * 44
        label(d, (8, yy + 8), name, size=10)
        for a, b, text in spans:
            col = (110, 64, 44) if "hold" not in text else (70, 52, 60)
            d.rectangle([60 + a * 4, yy, 60 + b * 4 - 1, yy + 24], fill=col, outline=(236, 232, 240))
            label(d, (63 + a * 4, yy + 6), text, size=9)
    for t in (0, 3, 6, 30, 60):
        d.line([(60 + t * 4, y + 18), (60 + t * 4, y + 22 + 88)], fill=(150, 146, 170))
        label(d, (56 + t * 4, y + 108 + 4), f"+{t}", size=9)
    path = OUT / "frame_anchor_timing.png"
    sheet.save(path)
    return path


def crop_of(img: Image.Image, floor: Floor, gx: int, k: int = 3, left: int = 110, right: int = 150) -> Image.Image:
    vx = gx - floor.view[0]
    gv = floor.ground(CARL_X) - floor.view[1]
    return img.crop((vx - left, gv - 96, vx + right, gv + 12)).resize(((left + right) * k, 108 * k), Image.NEAREST)


def storyboard(stage: Stage, name: str, kind: str, ticks: list[int], gx: int, preset: int = 4) -> Path:
    k = 3
    crops = []
    for t in ticks:
        img = stage.crush(preset, t, gx) if kind == "crush" else stage.decap(preset, t, gx)
        crop = crop_of(img, stage.floor, gx, k)
        ImageDraw.Draw(crop)
        label(ImageDraw.Draw(crop), (6, 4), f"{'L' if kind == 'crush' else 'D'}+{t}", (255, 255, 255), 14)
        crops.append(crop)
    cw, ch = crops[0].size
    sheet = Image.new("RGB", (2 * cw + 18, 2 * ch + 18), (30, 24, 30))
    for i, c in enumerate(crops):
        sheet.paste(c, (6 + (i % 2) * (cw + 6), 6 + (i // 2) * (ch + 6)))
    path = OUT / name
    sheet.save(path)
    return path


def preset_strip(stage: Stage) -> Path:
    k = 2
    cells = []
    for preset in PRESETS:
        row = [crop_of(stage.crush(preset, 30, CARL_X + 120), stage.floor, CARL_X + 120, k),
               crop_of(stage.decap(preset, 4, CARL_X + 170), stage.floor, CARL_X + 170, k),
               crop_of(stage.decap(preset, 40, CARL_X + 170), stage.floor, CARL_X + 170, k)]
        cells.append(row)
    cw, ch = cells[0][0].size
    sheet = Image.new("RGB", (3 * cw + 90, 4 * ch + 40), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for c, text in enumerate(("crush L+30", "decapitation D+4 (jet)", "decapitation D+40")):
        label(d, (80 + c * (cw + 4), 6), text, size=12)
    for r, preset in enumerate(PRESETS):
        label(d, (6, 40 + r * (ch + 4) + ch // 2), f"preset {preset}", size=12)
        for c in range(3):
            sheet.paste(cells[r][c], (80 + c * (cw + 4), 28 + r * (ch + 4)))
    path = OUT / "preset_strip.png"
    sheet.save(path)
    return path


def scenes_1x(stage: Stage) -> Path:
    views = [stage.crush(4, 0, CARL_X + 90), stage.decap(4, 0, CARL_X + 170), stage.decap(4, 60, CARL_X + 170),
             stage.scatter()]
    w, h = views[0].size
    sheet = Image.new("RGB", (w, h * len(views) + 4 * (len(views) - 1)), (24, 23, 34))
    for i, v in enumerate(views):
        sheet.paste(v, (0, i * (h + 4)))
    path = OUT / "scenes_1x.png"
    sheet.save(path)
    return path


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    sp, stage = Sprites(), Stage()
    for fn in (preset_poses, preset_parts, parts_anchors, frame_anchor_timing):
        print("wrote", fn(sp).relative_to(ROOT))
    print("wrote", storyboard(stage, "storyboard_crush.png", "crush", [0, 3, 6, 30], CARL_X + 120).relative_to(ROOT))
    print("wrote", storyboard(stage, "storyboard_decap.png", "decap", [0, 4, 10, 40], CARL_X + 170).relative_to(ROOT))
    print("wrote", preset_strip(stage).relative_to(ROOT))
    print("wrote", scenes_1x(stage).relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
