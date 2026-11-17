#!/usr/bin/env python3
"""Evidence for the DOZ-01 full frame set (Goblin Murder Dozer, execution by Claude Sonnet).

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases the lit compositor needs):

    python3 scripts/make_dozer_doz01_set_review.py [--out DIR] [--no-gifs]

Reads the DOZ-01 pixel-grid sources and the rig contract ``dozer_rig_h120.json`` (part pivots, anchors, draw order,
emitters and the 60 Hz tick table) and writes deterministically into DIR (default generated/art/doz01_set_review):

- ``<state>_x1.gif`` / ``<state>_x3.gif`` for cruise, tell, charge, brake and strain: the machine composed from its
  parts with the rig contract and driven tick by tick (60 Hz) by the state table of the rig contract: tread frame =
  floor(distance) mod 9, wheel rate, rattle with the riders 2 ticks late, squat, slide, rider and engineer frames,
  and the smoke and steam puffs in world space. The five states run as one chain (cruise, tell, charge, brake,
  strain), so every GIF starts with the smoke of the state before it. GIF delays are whole centiseconds,
  rounded on the cumulative tick count; one GIF frame is drawn every 2 ticks. Unlit, on a plain floor.
- ``floor01_chase_lit_{640x360,1920x1080}.png`` and ``floor01_chase_unlit_*.png``: the machine chasing Carl and Donut
  on the lit Floor 1 corridor (A1 compositor ``scripts/env_kit_light.py``) with the wheel in front of a torch-lit
  stretch of brick and in front of an unlit stretch. The contact shadow uses the tread footprint.
- ``part_contact_sheet.png``: every frame of every part atlas, with pivots and tick counts.
- ``rig_timing_diagram.png``: frame layout and pivots, the 60 Hz tick bars of the five states and the state flow.
- ``set_measurements.json``: file hashes, frame counts, colours and lint per atlas.

Composed PNGs and GIFs, not Bevy captures. Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

import author_dozer_doz01 as ad  # noqa: E402
import make_dozer_doz01_review as rv  # noqa: E402
import make_goblin_gob01_review as gr  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

DOZER = "assets/source/enemies/dozer"
VFX = "assets/source/vfx"
DEFAULT_OUT = ROOT / "generated" / "art" / "doz01_set_review"
STATES = ("cruise", "tell", "charge", "brake", "strain")
PREROLL = 60                         # ticks of cruise before the recorded chain, so the smoke is in steady state
CANVAS = (480, 200)                  # x1 GIF canvas; O sits at (250, 185)
ORIGIN = (250, 185)
BG = (36, 34, 50)
FLOOR = (58, 52, 70)
WHITE, GREY, YELLOW, CYAN = (236, 232, 240), (150, 146, 170), (255, 214, 0), (0, 220, 255)
ORANGE, RED, GREEN, BLUE = (255, 150, 60), (255, 70, 70), (120, 230, 120), (90, 160, 255)

STEM = {"smoke": "dozer_smoke", "spark": "vfx_hit_spark_metal"}


class Atlases:
    """Every part atlas as RGBA frames with its tag lookup."""

    def __init__(self) -> None:
        self.data: dict[str, tuple[list[Image.Image], tuple[int, int], dict[str, int]]] = {}
        for stem in ("dozer_hull_h120", "dozer_front_cab_h120", "dozer_front_tub_h120", "dozer_wheel_h120",
                     "dozer_tread_h120", "dozer_yoke_h120", "dozer_riders_h36", "dozer_smoke", "vfx_hit_spark_metal"):
            folder = VFX if stem.startswith("vfx_") else DOZER
            frames, piv, grid = gr.grid_frames(f"{folder}/{stem}.grid.json")
            tags = {t["name"]: t["from"] for t in grid["tags"] if t["from"] == t["to"]}
            self.data[stem] = (frames, piv, tags)

    def get(self, stem: str, tag: str) -> tuple[Image.Image, tuple[int, int]]:
        frames, piv, tags = self.data[stem]
        return frames[tags[tag]], piv


# ------------------------------------------------------------------ the state chain (60 Hz)

def simulate(rig: dict) -> list[dict]:
    """One dict per tick for PREROLL ticks of cruise, then the five states in order; each carries the pose of every
    part. Distance, wheel phase and the smoke emitters run continuously across the states."""
    states = rig["timing"]["states"]
    emit = rig["emitters"]
    smoke_cfg = rig["timing"]["smoke"]
    seq = [("cruise", PREROLL, False)] + [(s, states[s]["ticks"], True) for s in STATES]
    dist = tread_dist = wheel_phase = 0.0
    slide = 0.0
    puffs: list[dict] = []
    out: list[dict] = []
    tick = 0
    for name, n, record in seq:
        st = states[name]
        for t in range(n):
            speed = st["px_per_tick"]
            dist += speed
            tread_dist += speed
            if name == "brake":
                slide = min(float(st["slide_px"]), slide + st["slide_px"] / 8.0)
            wheel_phase += 1.0 / st["wheel"]
            wheel_i = int(wheel_phase) % 4
            wheel = f"phase_{wheel_i}"
            if st.get("smear_every") and t % st["smear_every"] == st["smear_every"] - 1:
                wheel = "smear"                     # fix round D3: the smear shows on 1 charge tick in 4
            # rattle (moving states) with the riders `rider_lag` ticks late; squat overrides it
            body_dy = rider_dy = 0
            r = st["rattle"]
            if r:
                body_dy = 1 if (t % r["period"]) < r["up"] else 0
                lt = t - r["rider_lag"]
                rider_dy = 1 if lt >= 0 and (lt % r["period"]) < r["up"] else 0
            if st["squat"] is True or (st["squat"] == "first 4 ticks" and t < 4):
                body_dy = rider_dy = -1
            # rider and engineer frames
            a = b = eng = None
            if name == "cruise":
                a = "rider_a_hold" if ((t // 16) % 2 == 0) else "rider_a_idle_1"
                b = "rider_b_hold" if (((t + 8) // 16) % 2 == 0) else "rider_b_idle_1"
                eng = "engineer_hold" if ((t // 16) % 2 == 0) else "engineer_idle_1"
            elif name == "tell":
                a = "rider_a_brandish" if ((t // 6) % 2 == 0) else "rider_a_yell_1"
                b = "rider_b_brandish" if ((t // 6) % 2 == 0) else "rider_b_yell_1"
                eng = "engineer_lever_pull"
            elif name == "charge":
                if t < 8:
                    a, b, eng = "rider_a_lurch_back", "rider_b_lurch_back", "engineer_lurch_back"
                else:
                    a = "rider_a_brandish" if (((t - 8) // 6) % 2 == 0) else "rider_a_yell_1"
                    b = "rider_b_brandish" if (((t - 8) // 6) % 2 == 0) else "rider_b_yell_1"
                    eng = "engineer_point" if (((t - 8) // 8) % 2 == 0) else "engineer_jab"
            elif name == "brake":
                if t < 10:
                    a, b, eng = "rider_a_lurch_forward", "rider_b_lurch_forward", "engineer_lurch_forward"
                else:
                    a = "rider_a_hold" if (((t - 10) // 16) % 2 == 0) else "rider_a_idle_1"
                    b = "rider_b_hold" if (((t - 10 + 8) // 16) % 2 == 0) else "rider_b_idle_1"
                    eng = "engineer_hold" if (((t - 10) // 16) % 2 == 0) else "engineer_idle_1"
            else:  # strain
                a = "rider_a_hold" if ((t // 16) % 2 == 0) else "rider_a_idle_1"
                b = "rider_b_hold" if (((t + 8) // 16) % 2 == 0) else "rider_b_idle_1"
                eng = "engineer_lever_pull"
            # smoke: spawn schedule of the state (world space after spawn, at most 8 black puffs alive)
            ox_world = dist + slide

            def spawn(kind: str, emitter: str) -> None:
                alive = [p for p in puffs if p["kind"] == "black" and tick - p["t0"] < smoke_cfg["life_ticks"]]
                if kind == "black" and len(alive) >= smoke_cfg["max_alive"]:
                    return
                ex, ey = emit[emitter]
                puffs.append({"kind": kind, "wx": ox_world + ex, "ey": ey, "t0": tick})
            if st["stack_puff_every"] and t % st["stack_puff_every"] == 0:
                spawn("black", "stack")
            if st["rear_puff_every"] and speed > 0 and t % st["rear_puff_every"] == 0:
                spawn("black", "rear_exhaust")
            if name == "strain" and t % st["rear_puff_every"] == 0:
                spawn("black", "rear_exhaust")
            stm = st["steam"]
            if stm:
                rep = stm.get("repeat", n + 1)
                tt = t % rep
                if tt % stm["every"] == 0 and tt // stm["every"] < stm["burst"]:
                    spawn("steam", "kettle")
            out.append({"state": name, "t": t, "tick": tick, "record": record, "dist": dist, "slide": slide,
                        "tread": f"belt_{int(tread_dist) % 9}", "wheel": wheel, "body_dy": body_dy,
                        "rider_dy": rider_dy, "a": a, "b": b, "eng": eng, "puffs": [dict(p) for p in puffs],
                        "speed": speed})
            puffs = [p for p in puffs if tick - p["t0"] < (smoke_cfg["life_ticks"] if p["kind"] == "black" else 15)]
            tick += 1
    return out


# ------------------------------------------------------------------ composition

def compose(rig: dict, at: Atlases, pose: dict, size=CANVAS, origin=ORIGIN, smoke: bool = True,
            parts_only: bool = False) -> Image.Image:
    """The machine of one pose on a transparent canvas; the rig origin O is the pixel ``origin`` (contact row)."""
    img = Image.new("RGBA", size, (0, 0, 0, 0))
    ox, oy = origin
    life = {"black": 24, "steam": 15}
    if smoke:
        for p in pose["puffs"]:
            age = pose["tick"] - p["t0"]
            if age >= life[p["kind"]]:
                continue
            per = 4 if p["kind"] == "black" else 3
            frame = f"{p['kind']}_{min(age // per, 5 if p['kind'] == 'black' else 4)}"
            spr, piv = at.get("dozer_smoke", frame)
            sx = ox + round(p["wx"] - pose["dist"])
            sy = oy - round(p["ey"] + 0.5 * age)
            img.alpha_composite(spr, (sx - piv[0], sy - piv[1]))
    sl = round(pose["slide"])
    choose = {"hull_body": ("dozer_hull_h120", "body", "body"), "tread": ("dozer_tread_h120", pose["tread"], None),
              "engineer": ("dozer_riders_h36", pose["eng"], "rider"), "rider_a": ("dozer_riders_h36", pose["a"], "rider"),
              "rider_b": ("dozer_riders_h36", pose["b"], "rider"),
              "hull_front_cab": ("dozer_front_cab_h120", "front_cab", "body"),
              "hull_front_tub": ("dozer_front_tub_h120", "front_tub", "body"),
              "wheel": ("dozer_wheel_h120", pose["wheel"], None), "yoke": ("dozer_yoke_h120", "static", None)}
    for p in sorted([q for q in rig["parts"] if q["part"] in choose], key=lambda q: q["z"]):
        stem, tag, group = choose[p["part"]]
        spr, piv = at.get(stem, tag)
        dy = {"body": pose["body_dy"], "rider": pose["rider_dy"]}.get(group, 0)
        ax, ay = p["anchor"]
        img.alpha_composite(spr, (ox + sl + ax - piv[0], oy - ay - piv[1] - dy))
    return img


def delays_ms(ticks: list[int]) -> list[int]:
    out, cum, prev = [], 0, 0
    for t in ticks:
        cum += t
        cs = round(cum * 100 / 60)
        out.append((cs - prev) * 10)
        prev = cs
    return out


def gif_frame(rig, at, pose, k: int) -> Image.Image:
    w, h = CANVAS
    base = Image.new("RGBA", CANVAS, BG + (255,))
    d = ImageDraw.Draw(base)
    ox, oy = ORIGIN
    d.rectangle([0, oy + 1, w, h], fill=FLOOR)
    d.line([0, oy + 1, w, oy + 1], fill=GREY)
    for gx in range(-20, w + 40, 20):                        # floor marks scroll with the distance travelled
        sx = gx - int(pose["dist"]) % 20
        d.line([sx, oy + 4, sx, oy + 9], fill=(86, 78, 100))
    base.alpha_composite(compose(rig, at, pose))
    big = gr.up(base, k)
    d = ImageDraw.Draw(big)
    d.text((6, 4), f"{pose['state']}  tick {pose['t']:3d}  tread {pose['tread']}  wheel {pose['wheel']}", fill=WHITE, font=gr.font())
    d.text((6, 16), f"hull dy {pose['body_dy']:+d}  riders dy {pose['rider_dy']:+d}  v {pose['speed'] * 60:.0f} px/s", fill=GREY, font=gr.font())
    return big.convert("RGB")


def make_gifs(rig, at, chain, out: Path) -> dict[str, list[int]]:
    info: dict[str, list[int]] = {}
    for name in STATES:
        poses = [p for p in chain if p["record"] and p["state"] == name]
        sample = poses[::2]
        ticks = [min(2, len(poses) - 2 * i) for i in range(len(sample))]
        delays = delays_ms(ticks)
        for k in (1, 3):
            imgs = [gif_frame(rig, at, p, k).convert("P", palette=Image.Palette.ADAPTIVE, colors=64) for p in sample]
            path = out / f"{name}_x{k}.gif"
            imgs[0].save(path, save_all=True, append_images=imgs[1:], duration=delays, loop=0, disposal=1, optimize=False)
        info[name] = delays
    return info


# ------------------------------------------------------------------ lit chase

def lit_scene(floor, rig, at, centre_x: int, k: int, carl, donut) -> Image.Image:
    """The machine (cruise pose, two puffs of the chain) chasing Carl and Donut; wheel centre at world x ``centre_x``."""
    chain = POSES["chain"]
    pose = next(p for p in chain if p["state"] == "cruise" and p["record"] and p["t"] == 100)
    size, origin = (520, 220), (330, 199)
    spr = compose(rig, at, pose, size, origin)
    axle_x = rig["parts"][[p["part"] for p in rig["parts"]].index("wheel")]["anchor"][0]
    o_x = centre_x - axle_x
    front = o_x + rig["extents"]["front_x"]
    vx = front - rv.FRONT_VIEW_X
    floor.view = [vx, 20, 640, 360]
    layer = Image.new("RGBA", tuple(floor.layout["pixel_size"]), (0, 0, 0, 0))
    ground = floor.ground(o_x)
    layer.alpha_composite(spr, (o_x - origin[0], ground - origin[1]))
    boxes = [(o_x - 74, ground - 33, o_x + 75, ground + 1)]          # the tread footprint (spec section 6)
    for (sprite, piv), x in ((carl, front + rv.GAP), (donut, front + rv.GAP - 54)):
        gy = floor.ground(x)
        layer.alpha_composite(sprite, (x - piv[0], gy - piv[1]))
        boxes.append((x - piv[0], gy - piv[1], x - piv[0] + sprite.width, gy - piv[1] + sprite.height))
    img = floor.ekl.render_lit(floor.kit, floor.layout, floor.view, k, layer, boxes=boxes)
    return img.convert("RGB")


POSES: dict = {}


# ------------------------------------------------------------------ sheets

def boxed(img: Image.Image, k: int, piv) -> Image.Image:
    return rv.boxed(img, k, piv)


def contact_sheet(rig: dict, at: Atlases) -> Image.Image:
    blocks = []
    stems = [("dozer_hull_h120", 2, 1), ("dozer_front_cab_h120", 4, 1), ("dozer_front_tub_h120", 3, 1),
             ("dozer_yoke_h120", 5, 1), ("dozer_wheel_h120", 2, 5), ("dozer_tread_h120", 3, 3),
             ("dozer_riders_h36", 4, 7), ("dozer_smoke", 5, 6), ("vfx_hit_spark_metal", 8, 3)]
    ticks_all = rig["timing"]["frame_ticks"]
    for stem, k, cols in stems:
        frames, piv, tags = at.data[stem]
        names = sorted(tags, key=lambda n: tags[n])
        items = []
        for n in names:
            tk = ticks_all[stem][tags[n]]
            items.append((boxed(frames[tags[n]], k, piv), f"{n} {'hold' if tk is None else str(tk) + 't'}"))
        blocks.append((stem, f"{frames[0].width}x{frames[0].height} pivot {piv}, x{k}", cols, items))
    sheet_w = 1800
    y = 30
    placed = []
    for stem, title, cols, items in blocks:
        y += 22
        x, row_h = 8, 0
        placed.append(("t", 8, y - 18, f"{stem}: {title}"))
        for im, cap in items:
            if x + im.width > sheet_w:
                x, y, row_h = 8, y + row_h + 22, 0
            placed.append(("i", x, y, im, cap))
            x += im.width + 12
            row_h = max(row_h, im.height)
        y += row_h + 26
    sheet = Image.new("RGB", (sheet_w, y + 10), BG)
    d = ImageDraw.Draw(sheet)
    d.text((8, 8), "DOZ-01 every part frame (execution: Claude Sonnet; design Claude Opus). Grey frame box, yellow pivot, "
                   "caption = tag and ticks at 60 Hz ('hold' = static or state-driven).", fill=WHITE, font=gr.font())
    for item in placed:
        if item[0] == "t":
            d.text((item[1], item[2]), item[3], fill=YELLOW, font=gr.font())
        else:
            _t, x, yy, im, cap = item
            sheet.paste(im, (x, yy))
            d.text((x, yy + im.height + 3), cap, fill=GREY, font=gr.font())
    return sheet


def rig_timing_diagram(rig: dict, at: Atlases, chain: list[dict]) -> Image.Image:
    comp = compose(rig, at, next(p for p in chain if p["state"] == "cruise" and p["record"] and p["t"] == 0),
                   smoke=False)
    comp = comp.crop((ORIGIN[0] - 90, ORIGIN[1] - 160, ORIGIN[0] + 180, ORIGIN[1] + 6))
    k = 3
    big = gr.up(Image.alpha_composite(Image.new("RGBA", comp.size, BG + (255,)), comp), k)
    d = ImageDraw.Draw(big)
    ox, oy = 90, 160

    def P(x, y):
        return ((ox + x) * k + k // 2, (oy - y) * k + k // 2)
    for p in rig["parts"]:
        if isinstance(p.get("anchor"), list):
            x, y = P(*p["anchor"])
            d.line([x - 6, y, x + 6, y], fill=YELLOW)
            d.line([x, y - 6, x, y + 6], fill=YELLOW)
    ox0, oy0 = P(0, 0)
    d.rectangle([ox0 - 4, oy0 - 4, ox0 + 4, oy0 + 4], outline=GREEN, width=2)
    d.line([0, oy0 + k // 2 + 1, big.width, oy0 + k // 2 + 1], fill=CYAN)
    states = rig["timing"]["states"]
    tick_w = 4
    total = sum(states[s]["ticks"] for s in STATES)
    W = max(big.width + 520, 60 + total * tick_w + 40, 1500)
    layout_rows = ["Frame layout (frame size, pivot placed on anchor from O):"] + [
        f"z{p['z']} {p['part']}: {p.get('frame_size')} pivot {p.get('pivot')} anchor {p['anchor'] if isinstance(p['anchor'], list) else 'emitters / contact point'}"
        for p in rig["parts"]]
    H = 60 + big.height + 40 + 12 * 16 + 24 * 8 + 280
    sheet = Image.new("RGB", (W, H), BG)
    ds = ImageDraw.Draw(sheet)
    ds.text((8, 8), "DOZ-01 rig and timing. Assembled rig x3 (cruise tick 0): yellow cross = part anchor (pivot goes on it), green = rig "
                    "origin O, cyan = walking line. Facing left: mirror every anchor x and flip each frame around its pivot.",
            fill=WHITE, font=gr.font())
    sheet.paste(big, (8, 30))
    for i, line in enumerate(layout_rows):
        ds.text((big.width + 24, 36 + 13 * i), line, fill=WHITE if line.endswith(":") else GREY, font=gr.font())
    y = 30 + big.height + 24
    ds.text((8, y), "Tick bars at 60 Hz (4 px per tick, one chain: cruise 120, tell 36, charge 90, brake 24, strain 90). Per tick: tread frame "
                    "(= floor(distance) mod 9), wheel frame, engineer, rider A, rider B, smoke spawns, hull and rider offsets (rattle, squat).",
            fill=WHITE, font=gr.font())
    y += 20
    rows = ["tread", "wheel", "engineer", "rider A", "rider B", "puffs", "hull dy", "riders dy"]
    x0 = 70
    cols = {"cruise": (80, 120, 80), "tell": (255, 150, 60), "charge": (255, 70, 70), "brake": (255, 214, 0), "strain": (90, 160, 255)}
    rec = [p for p in chain if p["record"]]
    for s_i, name in enumerate(STATES):
        poses = [p for p in rec if p["state"] == name]
        x_state = x0 + sum(states[s]["ticks"] for s in STATES[:s_i]) * tick_w
        ds.rectangle([x_state, y, x_state + len(poses) * tick_w - 1, y + 14], outline=cols[name])
        ds.text((x_state + 3, y + 2), f"{name} {len(poses)} t", fill=cols[name], font=gr.font())
        for j, pz in enumerate(poses):
            tx = x_state + j * tick_w
            vals = {"tread": pz["tread"].split("_")[1], "wheel": pz["wheel"].replace("phase_", "p"),
                    "engineer": pz["eng"], "rider A": pz["a"], "rider B": pz["b"],
                    "puffs": str(sum(1 for q in pz["puffs"] if q["t0"] == pz["tick"])),
                    "hull dy": str(pz["body_dy"]), "riders dy": str(pz["rider_dy"])}
            for r_i, r in enumerate(rows):
                yy = y + 20 + r_i * 24
                v = vals[r]
                if r in ("tread", "wheel"):
                    shade = (60 + (int(v[-1]) if v[-1].isdigit() else 5) * 20, 90, 130 if r == "tread" else 160)
                    ds.rectangle([tx, yy, tx + tick_w - 1, yy + 18], fill=shade)
                elif r in ("engineer", "rider A", "rider B"):
                    idx = sum(ord(c) for c in v)
                    ds.rectangle([tx, yy, tx + tick_w - 1, yy + 18], fill=(60 + idx % 150, 100 + (idx * 7) % 120, 90 + (idx * 13) % 130))
                elif r == "puffs":
                    if v != "0":
                        ds.rectangle([tx, yy, tx + tick_w - 1, yy + 18], fill=WHITE)
                else:
                    if v not in ("0",):
                        ds.rectangle([tx, yy, tx + tick_w - 1, yy + 18], fill=ORANGE if v == "1" else BLUE)
    for r_i, r in enumerate(rows):
        ds.text((8, y + 24 + r_i * 24), r, fill=GREY, font=gr.font())
    ds.text((x0, y + 20 + len(rows) * 24 + 2), "Colours encode the frame (tread and wheel: hue by index; riders: by frame name); "
                                              "orange = up 1 px, blue = down 1 px (squat), white = a puff spawns that tick.",
            fill=GREY, font=gr.font())
    y += 20 + len(rows) * 24 + 26
    ds.text((8, y), "State flow (spec section 9): enter -> cruise -> tell (gap >= 140 px) -> charge -> brake -> strain -> cruise.", fill=WHITE, font=gr.font())
    bx = 12
    flow = [("cruise", "160 px/s, wheel 2 t/frame\nstack puff 12 t"), ("tell", "36 t, wheel 1 t\nstack puff 4 t, steam\nlever pull, yell, squat"),
            ("charge", "<= 90 t, 240 px/s\nlurch back 8 t, smear\nstack puff 8 t"), ("brake", "24 t, slide 4 px\nlurch forward 10 t\nsteam burst"),
            ("strain", "90 t, rattle x2\npuffs from 3 emitters\nwheel 3 t/frame")]
    for i, (name, txt) in enumerate(flow):
        x = bx + i * 290
        ds.rectangle([x, y + 22, x + 250, y + 90], outline=cols[name], width=2)
        ds.text((x + 6, y + 26), f"{name.upper()} {states[name]['ticks']} ticks", fill=cols[name], font=gr.font())
        for j, line in enumerate(txt.split("\n")):
            ds.text((x + 6, y + 42 + 13 * j), line, fill=GREY, font=gr.font())
        if i < len(flow) - 1:
            ds.line([x + 252, y + 56, x + 286, y + 56], fill=WHITE, width=2)
            ds.polygon([(x + 286, y + 50), (x + 286, y + 62), (x + 292, y + 56)], fill=WHITE)
    ds.text((bx, y + 100), "strain -> cruise after 90 ticks (turned); cruise -> stopped when Carl is pinned by the hull, stopped -> cruise when free.", fill=GREY, font=gr.font())
    return sheet.crop((0, 0, W, y + 120))


# ------------------------------------------------------------------ main

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="DOZ-01 full-set evidence")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--no-gifs", action="store_true")
    args = parser.parse_args(argv)
    out = args.out if args.out.is_absolute() else ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)
    rig = json.loads((ROOT / DOZER / "dozer_rig_h120.json").read_text(encoding="utf-8"))
    at = Atlases()
    chain = simulate(rig)
    POSES["chain"] = chain
    written: list[Path] = []

    def save(img: Image.Image, name: str) -> None:
        path = out / name
        img.save(path, format="PNG", optimize=False)
        written.append(path)
    delays = {}
    if not args.no_gifs:
        delays = make_gifs(rig, at, chain, out)
        written += sorted(out.glob("*_x[13].gif"))
    save(contact_sheet(rig, at), "part_contact_sheet.png")
    save(rig_timing_diagram(rig, at, chain), "rig_timing_diagram.png")
    floor = gr.Floor()
    carl = rv.tag_frame(rv.CARL, "run", 2)
    donut = rv.tag_frame(rv.DONUT, "run", 4)
    for label, centre in (("lit", 440), ("unlit", 1100)):
        for k, size in ((1, "640x360"), (3, "1920x1080")):
            save(lit_scene(floor, rig, at, centre, k, carl, donut), f"floor01_chase_{label}_{size}.png")
    meas = {"note": "Composed review renders (A1 lighting via scripts/env_kit_light.py), not Bevy captures.",
            "gif_delays_ms": delays, "chain_ticks": {s: rig["timing"]["states"][s]["ticks"] for s in STATES},
            "atlases": {}, "files": {}}
    for stem, (frames, piv, _tags) in at.data.items():
        folder = VFX if stem.startswith("vfx_") else DOZER
        grid = pg.load_grid(ROOT / folder / f"{stem}.grid.json")
        lint = pg.lint_grid(grid)
        meas["atlases"][stem] = {"frames": len(frames), "frame_size": grid["canvas"], "pivot": grid["pivot"],
                                 "colours": len(grid["palette"]), "orphans": lint["orphan_count_per_frame"],
                                 "isolated": lint["isolated_count_per_frame"]}
    for path in written:
        meas["files"][path.name] = sha(path)
    mpath = out / "set_measurements.json"
    mpath.write_text(json.dumps(meas, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for path in written + [mpath]:
        print(f"  {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
