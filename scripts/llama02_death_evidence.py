#!/usr/bin/env python3
"""LLAMA-02-DEATH evidence renders (still sheets and GIFs composed from the committed sources; not Bevy captures).

    python3 scripts/llama02_death_evidence.py            # writes docs/evidence/llama-02-death/*

Outputs (deterministic, seeded; the same stage and placement simulation as ``gob02_death_evidence.py``, on the LLAMA-01
frame and the llama presets):

* ``preset_poses_x4.png`` / ``preset_parts_x3.png``: every pose and part at presets 4, 3, 2 and 1 (preset 1 hides
  ``gore``, preset 2 remaps it dark, ``molten`` is always shown; a part outside the preset's list is crossed out);
* ``parts_anchors_x4.png``: the 32 parts at x4 with every anchor marked;
* ``frame_anchor_timing.png``: the eight pose frames with anchors, tags, ticks and authors, plus the crush, decapitation
  and slam tick timelines;
* ``storyboard_crush.png``, ``storyboard_decap.png``, ``storyboard_slam.png``: the spec section 8 timelines as stills on
  lit Floor 1 (the llama mirrored to face Carl; the stone drawn over the crushed llama on the contact ticks);
* ``preset_strip.png``: the same moments at presets 4, 3, 2 and 1;
* ``scenes_1x.png``: six 640 x 360 lit views at 1x with Carl and Donut (crush, remains with pool stage 3 and the cooled
  molten puddle, decapitation tick, headless collapse, slam heap, an irregular explosion scatter);
* ``scene_carl_in_debris.png``: Carl standing in the debris (the cast is drawn last: nothing over a player sprite);
* ``gif_crush_sequence``, ``gif_decap_sequence``, ``gif_slam_sequence`` at x1 and x3, rendered unlit with an exact
  palette (no quantisation), and exact-palette replacements of the toolkit's per-tag pose GIFs.

The simulation only places pieces for stills (gravity 1050 px/s^2, the section 8 speed ranges, seeded); rotation is shown
in 90 degree steps. It feeds nothing back into the art or the game.
"""
from __future__ import annotations

import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw, ImageOps  # noqa: E402

import author_llama02_death as ad  # noqa: E402
from scripts.pixelart.common import CARL_X, DONUT_X, gif_delays, grid_images, label  # noqa: E402
from scripts.pixelart.scene import Floor, reference_idle  # noqa: E402

OUT = ROOT / "docs/evidence/llama-02-death"
G = 1050.0
DT = 1 / 60
PIV = tuple(ad.PIVOT)
CP = ad.CELL_PIVOT
PRESETS = (4, 3, 2, 1)
BRICK = (110, 64, 44)
SCALE = {4: 1.0, 3: 0.5, 2: 0.25, 1: 0.0}
MOLTEN_SCALE = {4: 1.0, 3: 1.0, 2: 0.5, 1: 0.5}
MIRROR_PIV_X = ad.CANVAS[0] - 1 - PIV[0]            # pivot column of a flipped pose image (51)


# --------------------------------------------------------------------------- sprites and anchors
class Sprites:
    def __init__(self) -> None:
        self.parts_grid, self.poses_grid = ad.parts_grid(), ad.poses_grid()
        self.entries = {e["id"]: e for e in ad.part_entries(self.parts_grid)}
        self.cache: dict = {}

    def part(self, pid: str, preset: int) -> Image.Image:
        key = ("part", preset)
        if key not in self.cache:
            self.cache[key] = grid_images(ad.preset_view(self.parts_grid, preset))
        return self.cache[key][ad.PART_IDS.index(pid)]

    def pose(self, n: int, preset: int, mirrored: bool = False) -> Image.Image:
        key = ("pose", preset)
        if key not in self.cache:
            self.cache[key] = grid_images(ad.preset_view(self.poses_grid, preset))
        img = self.cache[key][n]
        return img.transpose(Image.Transpose.FLIP_LEFT_RIGHT) if mirrored else img

    def allowed(self, pid: str, preset: int) -> bool:
        return preset in self.entries[pid]["presets"]

    def bottom(self, pid: str) -> int:
        return bbox_of(self.part(pid, 4))[3]


def bbox_of(img: Image.Image):
    x0, y0, x1, y1 = img.getbbox()
    return x0, y0, x1 - 1, y1 - 1


def world_of(anchor, gx: int, ground: int):
    """World position of a pose anchor of the mirrored llama (it faces Carl, on the left): x' = 95 - x on the flipped
    image, whose pivot column is 51."""
    return gx - MIRROR_PIV_X + (ad.CANVAS[0] - 1 - anchor[0]), ground - PIV[1] + anchor[1]


# --------------------------------------------------------------------------- tiny placement simulation
class Piece:
    def __init__(self, pid, x, y, vx, vy, spin, bounces, bounce, rot=0.0, swap=None):
        self.pid, self.x, self.y, self.vx, self.vy = pid, x, y, vx, vy      # world pos of the cell pivot, px/s (up +)
        self.spin, self.rot, self.bounces, self.bounce, self.swap = spin, rot, bounces, bounce, swap
        self.rest = False
        self.dead = False
        self.hits: list[float] = []
        self.t = 0


def current_id(p: Piece) -> str:
    return p.swap if (p.swap and p.hits) else p.pid


def step(sp: Sprites, p: Piece, ground: int) -> None:
    if p.rest:
        return
    p.t += 1
    p.vy -= G * DT
    p.x += p.vx * DT
    p.y -= p.vy * DT
    p.rot += p.spin * DT
    floor_y = ground - (sp.bottom(current_id(p)) - CP[1])
    if p.y >= floor_y and p.vy < 0:
        p.y = floor_y
        p.hits.append(p.x)
        if p.bounces <= 0 or abs(p.vy) < 40:
            p.rest, p.vx, p.vy, p.spin, p.rot = True, 0, 0, 0, 0
        else:
            p.vy = -p.vy * p.bounce
            p.vx *= 0.7
            p.bounces -= 1


def sprite_of(sp: Sprites, p: Piece, preset: int) -> Image.Image:
    img = sp.part(current_id(p), preset)
    quarter = round(p.rot / 90) % 4
    return img.rotate(90 * quarter) if (quarter and not p.rest) else img


class Scene:
    def __init__(self, sp: Sprites, floor: Floor, preset: int, seed: int) -> None:
        self.sp, self.floor, self.preset = sp, floor, preset
        self.rng = random.Random(seed)
        self.ground = floor.ground(CARL_X)
        self.carl, self.carl_piv = reference_idle("carl")
        self.donut, self.donut_piv = reference_idle("donut")
        self.pieces: list[Piece] = []
        self.decals: list[tuple[str, float, float]] = []
        self.walls: list[tuple] = []
        self.extra_pieces: list = []

    def rnd(self, lo, hi):
        return self.rng.uniform(lo, hi)

    def emit(self, pid, x, y, vx, vy, spin, bounces, bounce, rot=0.0, swap=None):
        if self.sp.allowed(pid, self.preset) or swap:
            self.pieces.append(Piece(pid, x, y, vx, vy, spin, bounces, bounce, rot, swap))

    def count(self, n: float, molten: bool = False) -> int:
        return int(round(n * (MOLTEN_SCALE if molten else SCALE)[self.preset]))

    def decal(self, pid, x, y=None):
        if self.sp.allowed(pid, self.preset):
            self.decals.append((pid, x, self.ground if y is None else y))
            if pid == "decal_pool_3":                  # stage 3 is two 19 px halves (a cell is 32 px wide)
                self.decals.append(("decal_pool_3_r", x + 1, self.ground if y is None else y))

    def wall(self, pid, wx, wy, flip=False):
        if self.sp.allowed(pid, self.preset):
            self.walls.append((pid, wx, wy, flip))

    def n_decals(self, pid: str) -> int:
        return sum(d[0] == pid for d in self.decals)

    def run(self, ticks: int, events: dict[int, list]) -> None:
        for t in range(ticks + 1):
            for fn in events.get(t, []):
                fn()
            for p in self.pieces:
                was = len(p.hits)
                step(self.sp, p, self.ground)
                if len(p.hits) > was and p.pid == "drop_blood":
                    if self.n_decals("decal_splat_s") < 8:
                        self.decal("decal_splat_s", p.hits[-1])
                    p.dead = True
                if len(p.hits) > was and p.pid == "drop_molten":
                    if self.n_decals("decal_molten") + self.n_decals("decal_molten_crust") < 6:
                        self.decal("decal_molten", p.hits[-1])
                    p.dead = True
                if len(p.hits) > was and p.pid in ("head", "head_dead"):
                    self.decal("decal_splat_s", p.hits[-1])
            self.pieces = [p for p in self.pieces if not p.dead]

    def anchor_of(self, pid: str, name: str | None = None):
        a = self.sp.entries[pid]["anchors"]
        return a[name] if name else list(a.values())[0]

    def entries(self, actor: list | None, cast_on_top: bool = True) -> list:
        """Draw order: wall decals, floor decals, debris, the actor entries (stone over the llama when the caller says
        so), then the cast last: nothing is ever drawn over a player sprite."""
        cast = [(self.donut, self.donut_piv, DONUT_X, 0), (self.carl, self.carl_piv, CARL_X, 0)]
        out = [] if cast_on_top else list(cast)
        for pid, wx, wy, flip in self.walls:
            img = self.sp.part(pid, self.preset)
            ax, ay = self.anchor_of(pid)
            if flip:
                img, ax = ImageOps.mirror(img), img.width - 1 - ax
            out.append((img, (ax, ay), int(wx), int(self.ground - wy), False))
        for pid, wx, wy in self.decals:
            img = self.sp.part(pid, self.preset)
            out.append((img, tuple(self.anchor_of(pid)), int(wx), 0, False))
        for p in self.pieces:
            out.append((sprite_of(self.sp, p, self.preset), CP, int(round(p.x)), int(round(self.ground - p.y)), False))
        out += self.extra_pieces
        out += actor or []
        if cast_on_top:
            out += cast
        return out


class Stage:
    def __init__(self) -> None:
        self.sp = Sprites()
        self.floor = Floor()
        self.falling, self.falling_anchor = ad.block_piece("loose_block_falling")
        self.landed, self.landed_anchor = ad.block_piece("loose_block_landed")
        self.pa = {f["index"]: f["anchors"] for f in ad.pose_manifest()["death_poses"]["frames"]}

    def pose_entry(self, n: int, preset: int, gx: int):
        return (self.sp.pose(n, preset, True), (MIRROR_PIV_X, PIV[1]), gx, 0)

    def _pool(self, sc: Scene, stage: int, gx: int, origin):
        if sc.sp.allowed(f"decal_pool_{stage}", sc.preset):
            sc.decals = [d for d in sc.decals if not d[0].startswith("decal_pool")]
            sc.decal(f"decal_pool_{stage}", world_of(origin, gx, sc.ground)[0])

    def _away(self, key_x: float, hit_x: float) -> int:
        return -1 if key_x < hit_x else 1

    # -- crush at tick L + t
    def crush(self, preset: int, t: int, gx: int = CARL_X + 120, seed: int = 11, lit: bool = True) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        a0 = self.pa[0]
        at = lambda anchor: world_of(anchor, gx, g)
        hit_x = at(a0["hit_origin"])[0]

        def molten():
            for _ in range(sc.count(sc.rng.randint(10, 14), True)):
                x, y = at(a0["gland"])
                ang = math.radians(90 + sc.rnd(-15, 15))
                sp_ = sc.rnd(220, 320)
                sc.emit("drop_molten", x, y, sp_ * math.cos(ang), sp_ * math.sin(ang), 0, 0, 0)
            x, y = at(a0["mouth"])
            ang = math.radians(60)
            sc.emit("tooth", x, y, -sc.rnd(60, 120), sc.rnd(180, 240), 900, 2, 0.4)

        def blood():
            plan = [("mouth", sc.rng.randint(2, 4)), ("squeeze_back", sc.rng.randint(8, 10)),
                    ("squeeze_front", sc.rng.randint(8, 10))]
            for key, n in plan:
                for _ in range(sc.count(n)):
                    x, y = at(a0[key])
                    ang = math.radians(sc.rnd(10, 35))
                    sp_ = sc.rnd(80, 160)
                    sign = self._away(x, hit_x)
                    sc.emit("drop_blood", x, y, sign * sp_ * math.cos(ang), sp_ * math.sin(ang), 0, 0, 0)

        def squeezed():
            plan = [("wool_tuft_a", "squeeze_back"), ("wool_tuft_b", "squeeze_front"), ("gib_flesh_a", "squeeze_back"),
                    ("gib_flesh_b", "squeeze_front"), ("gib_bone", "squeeze_front")]
            for pid, key in plan:
                x, y = at(a0[key])
                sign = self._away(x, hit_x)
                sc.emit(pid, x, y, sign * sc.rnd(60, 120), sc.rnd(30, 70), sign * sc.rnd(300, 600), 1, 0.45)

        def stains():
            for key in ("squeeze_back", "squeeze_front"):
                sc.decal("decal_splat_m" if sc.preset >= 3 else "decal_splat_s", at(a0[key])[0])
            self._pool(sc, 1, gx, self.pa[2]["pool_origin"])

        def wall_spatter():
            variant = sc.rng.choice(["decal_wall_spatter", "decal_wall_spatter_b"])
            flip = sc.rng.random() < 0.5
            sc.wall(variant, at(a0["squeeze_front"])[0] + (-6 if flip else 6), g - 8, flip)

        def crust():
            sc.decals = [(("decal_molten_crust" if d[0] == "decal_molten" else d[0]), d[1], d[2]) for d in sc.decals]

        events = {0: [molten, blood, squeezed], 4: [stains], 8: [wall_spatter],
                  20: [lambda: self._pool(sc, 2, gx, self.pa[2]["pool_origin"])],
                  60: [lambda: self._pool(sc, 3, gx, self.pa[2]["pool_origin"])], 240: [crust]}
        sc.run(t, events)
        # the stone draws over the crushed llama on the contact ticks (data: hazard_draw "over"), anchored on the
        # frame's mirrored hit_origin; the 3 px sink of frame 1 is a render-only offset. From the remains on: goblin order.
        if t < 3:
            actor = [self.pose_entry(0, sc.preset, gx),
                     (self.falling, self.falling_anchor, int(hit_x), PIV[1] - a0["hit_origin"][1], False)]
        elif t < 6:
            actor = [self.pose_entry(1, sc.preset, gx),
                     (self.falling, self.falling_anchor, int(hit_x), PIV[1] - self.pa[1]["hit_origin"][1], False)]
        else:
            actor = [(self.landed, self.landed_anchor, int(hit_x), 0, False), self.pose_entry(2, sc.preset, gx)]
        return self.floor.render(sc.entries(actor), 1, lit)

    # -- decapitation at tick D + t
    def decap(self, preset: int, t: int, gx: int = CARL_X + 170, seed: int = 23, lit: bool = True) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        a3, a4, a5 = self.pa[3], self.pa[4], self.pa[5]

        def head():
            nx, ny = world_of(a3["neck"], gx, g)
            ha = sc.anchor_of("head", "neck")
            sc.emit("head", nx - (ha[0] - CP[0]), ny - (ha[1] - CP[1]), sc.rnd(50, 90), sc.rnd(240, 300), -600, 2, 0.3,
                    -25.0, swap="head_dead")

        def jet(k):
            def fn():
                jx, jy = world_of(a4["jet_base"], gx, g)
                for _ in range(sc.count(3)):
                    ang = math.radians(90 + sc.rnd(-12, 12))
                    sp_ = sc.rnd(170, 240)
                    sc.emit("drop_blood", jx, jy, sp_ * math.cos(ang), sp_ * math.sin(ang), 0, 0, 0)
                if k % 2 == 0:
                    gxw, gyw = world_of(a4["gland"], gx, g)
                    sc.emit("drop_molten", gxw, gyw, sc.rnd(-20, 20), sc.rnd(120, 180), 0, 0, 0)
            return fn

        def trickle():
            nx, ny = world_of(a5["neck"], gx, g)
            if sc.count(1):
                sc.emit("drop_blood", nx, ny, sc.rnd(-8, 8), 10, 0, 0, 0)

        def wall():
            jx, jy = world_of(a3["jet_base"], gx, g)
            variant = sc.rng.choice(["decal_wall_spatter", "decal_wall_spatter_b"])
            flip = sc.rng.random() < 0.5
            sc.wall(variant, jx + (3 if flip else -3), g - 26, flip)

        events: dict[int, list] = {0: [head], 4: [wall]}
        for k, tk in enumerate(range(4, 41, 3)):
            events.setdefault(tk, []).append(jet(k))
        for tk in range(10, 51, 8):
            events.setdefault(tk, []).append(trickle)
        for tk, stage in ((14, 1), (40, 2), (100, 3)):
            events.setdefault(tk, []).append(lambda s=stage: self._pool(sc, s, gx, a5["pool_origin"]))
        events.setdefault(4, []).append(lambda: sc.decal("decal_molten", world_of(a5["gland"], gx, g)[0] + 6))
        sc.run(t, events)
        if t >= 12 and preset == 4:                    # the roll leaves a smear from the second contact
            hits = [p for p in sc.pieces if p.pid == "head"]
            if hits and len(hits[0].hits) >= 2:
                sc.decal("decal_smear", hits[0].hits[1])
        n = 3 if t < 4 else (4 if t < 10 else 5)
        return self.floor.render(sc.entries([self.pose_entry(n, preset, gx)]), 1, lit)

    # -- slam at tick S + t
    def slam(self, preset: int, t: int, gx: int = CARL_X + 130, seed: int = 31, lit: bool = True) -> Image.Image:
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        a6, a7 = self.pa[6], self.pa[7]
        hit_x = world_of(a6["hit_origin"], gx, g)[0]

        def impact():
            x, y = world_of(a6["mouth"], gx, g)
            sc.emit("tooth", x, y, sc.rnd(60, 100), sc.rnd(200, 260), 900, 2, 0.4)
            for _ in range(sc.count(sc.rng.randint(8, 12))):
                ang = math.radians(90 + sc.rnd(-25, 35))
                sp_ = sc.rnd(120, 200)
                sc.emit("drop_blood", x, y, sp_ * math.cos(ang), sp_ * math.sin(ang), 0, 0, 0)
            gx_, gy_ = world_of(a6["gland"], gx, g)
            if sc.count(1, True):
                sc.emit("drop_molten", gx_, gy_, sc.rnd(-30, 30), sc.rnd(180, 240), 0, 0, 0)
            for _ in range(3):
                sc.emit(sc.rng.choice(["wool_tuft_a", "wool_tuft_b"]), hit_x, g - 8, sc.rnd(-90, 90), sc.rnd(120, 200),
                        sc.rnd(-400, 400), 1, 0.2)

        events = {0: [impact], 8: [lambda: self._pool(sc, 1, gx, a7["pool_origin"])],
                  30: [lambda: self._pool(sc, 2, gx, a7["pool_origin"])]}
        sc.run(t, events)
        n = 6 if t < 6 else 7
        return self.floor.render(sc.entries([self.pose_entry(n, preset, gx)]), 1, lit)

    # -- explosive breakup: an irregular settle (spec section 8)
    def explosion(self, preset: int = 4, centre: int = CARL_X + 150, seed: int = 5, carl_inside: bool = False):
        """Every part of the preset at its settled place: a seeded scatter around ``centre``, rotations in 90 degree
        steps, light pieces piled on heavier ones (never higher than 12 px), then 10 gibs, 2 ribs, 28 drops (at most 8
        splats), 8 molten drops (at most 6 puddles), both wall-spatter variants and pools under the two heaviest."""
        sc = Scene(self.sp, self.floor, preset, seed)
        g = sc.ground
        heavy = ["torso_front", "torso_rump", "head_dead", "neck", "leg_fore", "leg_hind", "leg_fore_far",
                 "leg_hind_far", "jaw", "ear"]
        light = (["gib_flesh_a"] * 3 + ["gib_flesh_b"] * 3 + ["gib_bone"] * 2 + ["gib_rib"] * 2 + ["tooth"] * 2
                 + ["wool_tuft_a", "wool_tuft_b", "gland"])
        placed: list[tuple[int, int, str]] = []
        spots: dict[str, int] = {}
        for pid in heavy + light:
            if not (self.sp.allowed(pid, preset) or pid == "head_dead"):
                continue
            x = int(centre + max(-80, min(80, sc.rng.gauss(0, 34))))
            quarter = sc.rng.randint(0, 3)
            img = self.sp.part(pid, preset).rotate(90 * quarter)
            x0, y0, x1, y1 = bbox_of(img)
            lift = 0
            for ox, otop, _oid in placed:
                if abs(ox - x) < 9:
                    lift = max(lift, min(12, otop))
            placed.append((x, lift + (y1 - y0 + 1), pid))
            spots.setdefault(pid, x)
            sc.extra_pieces.append((img, CP, x, y1 - CP[1] + lift, False))
        for _ in range(sc.count(28)):
            if sc.n_decals("decal_splat_s") < 8:
                sc.decal("decal_splat_s", int(centre + sc.rng.gauss(0, 44)))
        for _ in range(2):
            sc.decal("decal_splat_m", int(centre + sc.rng.uniform(-60, 60)))
        for _ in range(sc.count(8, True)):
            if sc.n_decals("decal_molten") < 6:
                sc.decal("decal_molten", int(centre + sc.rng.gauss(0, 40)))
        if "gland" in spots:
            sc.decal("decal_molten", spots["gland"] + 2)
        if "torso_front" in spots:
            sc.decals.append(("decal_pool_2", spots["torso_front"], g))
        if "torso_rump" in spots:
            sc.decals.append(("decal_pool_1", spots["torso_rump"] + 4, g))
        sc.wall("decal_wall_spatter", centre - 48, g - 22, False)
        sc.wall("decal_wall_spatter_b", centre + 40, g - 12, True)
        return self.floor.render(sc.entries(None, cast_on_top=True), 1)


# --------------------------------------------------------------------------- exact-palette GIFs
def save_exact_gif(images: list[Image.Image], delays: list[int], path: Path) -> None:
    """GIF with the exact colours of the frames (no quantisation, no dither): one shared palette of every colour used."""
    rgb = [im.convert("RGB") for im in images]
    colours = sorted({c for im in rgb for c in im.getdata()})
    if len(colours) > 256:
        raise ValueError(f"{len(colours)} colours do not fit a GIF palette exactly")
    index = {c: i for i, c in enumerate(colours)}
    frames = []
    for im in rgb:
        pal = Image.new("P", im.size)
        pal.putpalette([v for c in colours for v in c])
        pal.putdata([index[c] for c in im.getdata()])
        frames.append(pal)
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=delays, loop=0, disposal=1, optimize=False)


def crop_of(img: Image.Image, floor: Floor, gx: int, k: int = 3, left: int = 110, right: int = 150) -> Image.Image:
    vx = gx - floor.view[0]
    gv = floor.ground(CARL_X) - floor.view[1]
    return img.crop((vx - left, gv - 96, vx + right, gv + 12)).resize(((left + right) * k, 108 * k), Image.NEAREST)


def sequence_gif(name: str, frames_for, ticks: list[int], holds: int, gx: int, k: int, **crop) -> Path:
    """``frames_for(t)`` renders the unlit view at tick ``t``; the last frame holds ``holds`` extra ticks."""
    imgs = [crop_of(frames_for(t), STAGE.floor, gx, k, **crop) for t in ticks]
    spans = [t2 - t1 for t1, t2 in zip(ticks, ticks[1:])] + [holds]
    path = OUT / f"gif_{name}_x{k}.gif"
    save_exact_gif(imgs, gif_delays(spans), path)
    return path


def exact_pose_gifs() -> list[Path]:
    """Replace the toolkit's per-tag pose GIFs (lossy 64-colour quantisation) with exact-palette ones."""
    from scripts.pixelart import config as pcfg
    from scripts.pixelart.common import up
    from scripts.pixelart.evidence import sprite_on, tick_sequence
    cfg = pcfg.load_config(ROOT / "assets/source/characters/llama/death/llama_death_h64.pixelart.json")
    imgs = grid_images(cfg.load_grid())
    paths = []
    for tag in cfg.tags:
        seq = tick_sequence(cfg, tag)
        delays = gif_delays([t for _f, t in seq], cfg.tick_hz)
        for k in (1, 3):
            path = OUT / "poses" / f"gif_{tag}_x{k}.gif"
            save_exact_gif([sprite_on(imgs[f], k, cfg.contact_row) for f, _t in seq], delays, path)
            paths.append(path)
    del up
    return paths


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
    k = 3
    w, h = ad.CANVAS
    cols = 4
    band_h = 4 * (h * k + 6)
    sheet = Image.new("RGB", (cols * (w * k + 6) + 70, 2 * band_h + 30), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    label(d, (6, 4), "poses atlas: frames 0 to 7 at presets 4, 3, 2, 1 (same pixels; 1 hides gore, 2 remaps it dark, molten "
          "always shown; 3 = 4 for poses)")
    for band in range(2):
        for r, preset in enumerate(PRESETS):
            y0 = 26 + band * band_h + r * (h * k + 6)
            label(d, (6, y0 + h * k // 2), f"preset {preset}")
            for c in range(cols):
                n = band * cols + c
                sheet.paste(tile(sp.pose(n, preset), k), (70 + c * (w * k + 6), y0))
                if r == 0:
                    label(d, (74 + c * (w * k + 6), y0 + 2), f"{n}", size=11)
    path = OUT / "preset_poses_x4.png"
    sheet.save(path)
    return path


def preset_parts(sp: Sprites) -> Path:
    k = 3
    cols = 16
    cw = 32 * k + 4
    sheet = Image.new("RGB", (cols * cw + 70, 2 * 4 * (32 * k + 4) + 40), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    label(d, (6, 4), "parts atlas at presets 4, 3, 2, 1 (crossed out: not spawned at that preset); 32 parts in two bands of 16")
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
                    label(d, (70 + c * cw + 2, y0 + 2), pid[:14], size=9)
    path = OUT / "preset_parts_x3.png"
    sheet.save(path)
    return path


ANCHOR_COLOURS = [(0, 220, 255), (255, 214, 0), (255, 90, 200), (120, 255, 120), (255, 150, 40), (180, 140, 255)]


def parts_anchors(sp: Sprites) -> Path:
    k = 4
    cols, cw = 8, 32 * 4 + 6
    rows = 4
    sheet = Image.new("RGB", (cols * cw + 6, rows * (32 * k + 22) + 6), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for i, pid in enumerate(ad.PART_IDS):
        x0, y0 = 6 + (i % cols) * cw, 6 + (i // cols) * (32 * k + 22)
        sheet.paste(tile(sp.part(pid, 4), k), (x0, y0))
        e = sp.entries[pid]
        label(d, (x0 + 1, y0 + 32 * k + 3), f"{i:02d} {pid}", size=10)
        d.rectangle([x0 + 16 * k, y0 + 16 * k, x0 + 16 * k + k - 1, y0 + 16 * k + k - 1], outline=(255, 255, 255))
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
    cols = 4
    fh = h * k + 8
    sheet = Image.new("RGB", (cols * (w * k + 8) + 8, 2 * fh + 250), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for n, f in enumerate(man):
        x0, y0 = 8 + (n % cols) * (w * k + 8), 8 + (n // cols) * fh
        sheet.paste(tile(sp.pose(n, 4), k), (x0, y0))
        d.line([(x0, y0 + 72 * k), (x0 + w * k, y0 + 72 * k)], fill=(0, 220, 255))
        d.rectangle([x0 + PIV[0] * k, y0 + PIV[1] * k, x0 + PIV[0] * k + k - 1, y0 + PIV[1] * k + k - 1],
                    outline=(255, 214, 0))
        hz = f"  hazard {f['hazard_draw']}" if f.get("hazard_draw") else ""
        label(d, (x0 + 3, y0 + 3), f"{n} {f['tag']} {'hold' if f['ticks'] is None else str(f['ticks']) + 't'} "
              f"{f['author']}{hz}", size=10)
        for j, (name, (ax, ay)) in enumerate(f["anchors"].items()):
            c = ANCHOR_COLOURS[j % len(ANCHOR_COLOURS)]
            cx, cy = x0 + ax * k + k // 2, y0 + ay * k + k // 2
            d.line([(cx - 5, cy), (cx + 5, cy)], fill=c)
            d.line([(cx, cy - 5), (cx, cy + 5)], fill=c)
            label(d, (x0 + 3, y0 + 16 + j * 11), name, c, 9)
    y = 2 * fh + 10
    label(d, (8, y), "timeline at 60 Hz (1 tick = 4 px): crush L, decapitation D, slam S; holds are state-driven", size=11)
    rows = (("crush", [(0, 3, "f0 contact"), (3, 6, "f1 flatten"), (6, 60, "f2 remains hold")]),
            ("decap", [(0, 4, "f3 blade"), (4, 10, "f4 buckle"), (10, 60, "f5 hold")]),
            ("slam", [(0, 6, "f6 impact"), (6, 60, "f7 broken heap hold")]))
    for row, (name, spans) in enumerate(rows):
        yy = y + 24 + row * 44
        label(d, (8, yy + 8), name, size=10)
        for a, b, text in spans:
            col = (110, 64, 44) if "hold" not in text else (70, 52, 60)
            d.rectangle([60 + a * 4, yy, 60 + b * 4 - 1, yy + 24], fill=col, outline=(236, 232, 240))
            label(d, (63 + a * 4, yy + 6), text, size=9)
    for t in (0, 3, 6, 30, 60):
        d.line([(60 + t * 4, y + 20), (60 + t * 4, y + 24 + 3 * 44)], fill=(150, 146, 170))
        label(d, (56 + t * 4, y + 24 + 3 * 44 + 2), f"+{t}", size=9)
    path = OUT / "frame_anchor_timing.png"
    sheet.save(path)
    return path


def storyboard(name: str, kind: str, ticks: list[int], gx: int, preset: int = 4) -> Path:
    k = 3
    crops = []
    for t in ticks:
        img = getattr(STAGE, kind)(preset, t, gx)
        crop = crop_of(img, STAGE.floor, gx, k, left=130, right=150)
        label(ImageDraw.Draw(crop), (6, 4), f"{ {'crush': 'L', 'decap': 'D', 'slam': 'S'}[kind] }+{t}", (255, 255, 255), 14)
        crops.append(crop)
    cw, ch = crops[0].size
    sheet = Image.new("RGB", (2 * cw + 18, 2 * ch + 18), (30, 24, 30))
    for i, c in enumerate(crops):
        sheet.paste(c, (6 + (i % 2) * (cw + 6), 6 + (i // 2) * (ch + 6)))
    path = OUT / name
    sheet.save(path)
    return path


def preset_strip() -> Path:
    k = 2
    specs = [("crush L+30", "crush", 30, CARL_X + 120), ("decapitation D+4 (jet)", "decap", 4, CARL_X + 170),
             ("decapitation D+40", "decap", 40, CARL_X + 170), ("slam S+14", "slam", 14, CARL_X + 130)]
    cells = []
    for preset in PRESETS:
        cells.append([crop_of(getattr(STAGE, kind)(preset, t, gx), STAGE.floor, gx, k, left=130, right=150)
                      for _l, kind, t, gx in specs])
    cw, ch = cells[0][0].size
    sheet = Image.new("RGB", (4 * cw + 90, 4 * ch + 40), (30, 24, 30))
    d = ImageDraw.Draw(sheet)
    for c, (text, *_rest) in enumerate(specs):
        label(d, (80 + c * (cw + 4), 6), text, size=12)
    for r, preset in enumerate(PRESETS):
        label(d, (6, 40 + r * (ch + 4) + ch // 2), f"preset {preset}", size=12)
        for c in range(4):
            sheet.paste(cells[r][c], (80 + c * (cw + 4), 28 + r * (ch + 4)))
    path = OUT / "preset_strip.png"
    sheet.save(path)
    return path


def scenes_1x() -> Path:
    views = [STAGE.crush(4, 0, CARL_X + 120), STAGE.crush(4, 250, CARL_X + 120), STAGE.decap(4, 0, CARL_X + 170),
             STAGE.decap(4, 110, CARL_X + 170), STAGE.slam(4, 40, CARL_X + 130), STAGE.explosion()]
    w, h = views[0].size
    sheet = Image.new("RGB", (w, h * len(views) + 4 * (len(views) - 1)), (24, 23, 34))
    for i, v in enumerate(views):
        sheet.paste(v, (0, i * (h + 4)))
    path = OUT / "scenes_1x.png"
    sheet.save(path)
    return path


def carl_in_debris() -> Path:
    view = STAGE.explosion(4, CARL_X + 6, seed=9, carl_inside=True)
    k = 3
    vx, gv = CARL_X - STAGE.floor.view[0], STAGE.floor.ground(CARL_X) - STAGE.floor.view[1]
    crop = view.crop((vx - 110, gv - 80, vx + 110, gv + 12)).resize((220 * k, 92 * k), Image.NEAREST)
    sheet = Image.new("RGB", (view.width, view.height + crop.height + 4), (24, 23, 34))
    sheet.paste(view, (0, 0))
    sheet.paste(crop, ((view.width - crop.width) // 2, view.height + 4))
    path = OUT / "scene_carl_in_debris.png"
    sheet.save(path)
    return path


def headless_lit_x3() -> Path:
    """The headless_collapse hold on lit brick next to Carl, 3x (gate note N1: judge the stump at 1x on lit brick)."""
    view = STAGE.decap(4, 60, CARL_X + 90)
    k = 3
    vx, gv = CARL_X - STAGE.floor.view[0], STAGE.floor.ground(CARL_X) - STAGE.floor.view[1]
    crop = view.crop((vx - 20, gv - 66, vx + 170, gv + 8)).resize((190 * k, 74 * k), Image.NEAREST)
    path = OUT / "scene_headless_x3.png"
    crop.save(path)
    return path


STAGE: Stage


def write_sums() -> None:
    """SHA256SUMS of ``poses/`` and ``parts/`` (their own lists) and of the whole folder (last: it lists them)."""
    import hashlib

    def digest(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def write(folder: Path) -> None:
        files = [f for f in folder.rglob("*") if f.is_file() and f.name != "SHA256SUMS" or
                 (f.is_file() and f.parent != folder)]
        files = [f for f in files if not (f.parent == folder and f.name == "SHA256SUMS")]
        lines = sorted((f"{digest(f)}  {f.relative_to(folder).as_posix()}" for f in files),
                       key=lambda line: line.split("  ", 1)[1].lower())
        (folder / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")

    for sub in ("poses", "parts"):
        write(OUT / sub)
    write(OUT)


def main() -> int:
    global STAGE
    OUT.mkdir(parents=True, exist_ok=True)
    STAGE = Stage()
    sp = STAGE.sp
    for fn in (preset_poses, preset_parts, parts_anchors, frame_anchor_timing):
        print("wrote", fn(sp).relative_to(ROOT))
    print("wrote", storyboard("storyboard_crush.png", "crush", [0, 3, 6, 30], CARL_X + 120).relative_to(ROOT))
    print("wrote", storyboard("storyboard_decap.png", "decap", [0, 4, 10, 40], CARL_X + 170).relative_to(ROOT))
    print("wrote", storyboard("storyboard_slam.png", "slam", [0, 6, 14, 40], CARL_X + 130).relative_to(ROOT))
    print("wrote", preset_strip().relative_to(ROOT))
    print("wrote", scenes_1x().relative_to(ROOT))
    print("wrote", carl_in_debris().relative_to(ROOT))
    print("wrote", headless_lit_x3().relative_to(ROOT))
    crush_ticks, decap_ticks, slam_ticks = [0, 3, 4, 6, 8, 20, 60, 240], [0, 4, 10, 14, 40, 100], [0, 6, 8, 30]
    for k in (1, 3):
        for name, fn, ticks, hold, gx in (
                ("crush_sequence", lambda t: STAGE.crush(4, t, CARL_X + 120, lit=False), crush_ticks, 120, CARL_X + 120),
                ("decap_sequence", lambda t: STAGE.decap(4, t, CARL_X + 170, lit=False), decap_ticks, 120, CARL_X + 170),
                ("slam_sequence", lambda t: STAGE.slam(4, t, CARL_X + 130, lit=False), slam_ticks, 120, CARL_X + 130)):
            print("wrote", sequence_gif(name, fn, ticks, hold, gx, k, left=130, right=150).relative_to(ROOT))
    for path in exact_pose_gifs():
        print("wrote", path.relative_to(ROOT))
    write_sums()
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["sums"]:
        write_sums()
        sys.exit(0)
    sys.exit(main())
