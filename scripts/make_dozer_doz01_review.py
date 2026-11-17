#!/usr/bin/env python3
"""Review renders for DOZ-01 phase 1 (Goblin Murder Dozer size study, keys and construction).

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases):

    python3 scripts/make_dozer_doz01_review.py [--out DIR]

Reads the DOZ-01 pixel-grid sources, the rig contract ``dozer_rig_h120.json`` and the approved
Carl 52 px / Donut 40 px locomotion masters, and writes deterministically into DIR (default
generated/art/doz01_review):

- ``floor01_chase_hNN_640x360.png`` / ``_1920x1080.png``: one size option chasing Carl and Donut on
  the lit Floor 1 corridor (A1 compositor ``scripts/env_kit_light.py``), Carl one third of the
  view (213 px) ahead of the wheel;
- ``size_study_sheet.png``: the three options side by side at x2 with the measured gameplay lines,
  a metrics table and the three lit chase crops;
- ``key_sheet.png``: the hull, wheel, tread, yoke and rider keys, enlarged, with pivots, plus the
  rotation and belt plans rendered from the same geometry (plan previews, not keys);
- ``part_layout_diagram.png``: exploded parts with frame boxes, pivots, anchors and draw order,
  and the assembled rig with the proposed hazard box, pushing wall, occluder rows, emitters and
  light points;
- ``measurements.json``: per-option measurements, atlas cost, colours and file hashes.

Composed PNGs, not Bevy captures. Presentation-only art preview; nothing here is wired into
the game.
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
import make_goblin_gob01_review as gr  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

DOZER = "assets/source/enemies/dozer"
CARL = "assets/source/characters/carl/carl_h52_locomotion.grid.json"
DONUT = "assets/source/characters/donut/donut_h40_locomotion.grid.json"
DEFAULT_OUT = ROOT / "generated" / "art" / "doz01_review"
VIEW_W, VIEW_H = 640, 360
GAP = 213                    # Carl one third of the view ahead of the wheel's front tip
FRONT_VIEW_X = 262           # wheel front tip, view x: the whole h120 machine is in view
CEILING_CLEARANCE = 276      # walking line (world y 308) to the ceiling collider (world y 32)
JUMP_APEX = 92               # Carl's feet at the jump apex: 440^2 / (2 * 1050)
CARL_H = 52
RECOMMENDED = 120
BG = (24, 23, 34)
CYAN, YELLOW, WHITE, GREY = (0, 220, 255), (255, 214, 0), (236, 232, 240), (150, 146, 170)
RED, BLUE, GREEN, ORANGE = (255, 70, 70), (90, 160, 255), (120, 230, 120), (255, 150, 60)

# planned frame counts per part (spec section 5) for the atlas-cost estimate
PLAN_FRAMES = {"hull": 2, "wheel": 4, "tread": 3, "yoke": 1}
RIDER_FRAMES = 3 * 6          # 3 riders x (idle 2, yell 2, lurch back, lurch forward)
SMOKE_PX = 24 * 24 * 6 + 16 * 16 * 5


def rows_to_img(rows) -> Image.Image:
    pal = {k: pg.parse_rgba(c) for k, _n, c, _a in ad.ALL_PALETTE}
    h, w = len(rows), len(rows[0])
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    px = img.load()
    for y in range(h):
        for x in range(w):
            if rows[y][x] != ".":
                px[x, y] = pal[rows[y][x]]
    return img


def grid_imgs(path: str):
    frames, piv, grid = gr.grid_frames(path)
    return frames, piv, grid


def tag_frame(path: str, tag: str, offset: int = 0) -> tuple[Image.Image, tuple[int, int]]:
    frames, piv, grid = grid_imgs(path)
    t = next(t for t in grid["tags"] if t["name"] == tag)
    return frames[t["from"] + offset], piv


def bbox(img: Image.Image):
    return img.getchannel("A").getbbox()


def machine(h: int) -> tuple[Image.Image, tuple[int, int]]:
    """Size-study frame for body height h (flattened, riders seated), with its pivot (rig origin O)."""
    frames, piv, grid = grid_imgs(f"{DOZER}/dozer_size_study.grid.json")
    names = [t["name"] for t in grid["tags"]]
    return frames[names.index(f"h{h}")], piv


def compose_rig(rig: dict, rider_frames: dict[str, int] | None = None) -> tuple[Image.Image, tuple[int, int]]:
    """Assemble the h120 parts from their grids with the rig contract (pivot onto anchor, by z)."""
    rider_frames = rider_frames or {"engineer": 0, "rider_a": 1, "rider_b": 2}
    parts = []
    for p in rig["parts"]:
        if "tag" not in p:
            continue
        atlas = p["atlas"]
        frames, piv, grid = grid_imgs(f"{DOZER}/{atlas}.grid.json")
        if atlas == "dozer_riders_h36":
            img = frames[rider_frames[p["part"]]]
        else:
            t = next(t for t in grid["tags"] if t["name"] == p["tag"])
            img = frames[t["from"]]
        parts.append((p["z"], p["part"], img, tuple(piv), tuple(p["anchor"])))
    W, H = 300, 180
    ox, fy = 96, H - 1
    canvas = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    for _z, _name, img, piv, (ax, ay) in sorted(parts, key=lambda q: q[0]):
        canvas.alpha_composite(img, (ox + ax - piv[0], fy - ay - piv[1]))
    return canvas, (ox, fy)


# ------------------------------------------------------------------ measurements

def option_metrics(h: int) -> dict:
    s = h / ad.H_KEY
    img, piv = machine(h)
    x0, y0, x1, y1 = bbox(img)
    hull_w, hull_h, _ox, _fy = ad.hull_canvas(s)
    wheel = 2 * math.ceil(ad.WHEEL_TIP * s) + 3
    tread_rows, _ = ad.tread_rows(0, s)
    tread = (len(tread_rows[0]), len(tread_rows))
    yoke = (math.ceil(64 * s), math.ceil(44 * s))
    parts_px = (hull_w * hull_h * PLAN_FRAMES["hull"] + wheel * wheel * PLAN_FRAMES["wheel"]
                + tread[0] * tread[1] * PLAN_FRAMES["tread"] + yoke[0] * yoke[1] * PLAN_FRAMES["yoke"])
    riders_px = 64 * 56 * RIDER_FRAMES
    mono = (x1 - x0) * (y1 - y0) * 12 * 2   # lcm(4 wheel, 3 tread) x 2 rider states, flattened
    total_h = piv[1] + 1 - y0
    front = x1 - 1 - piv[0]
    rear = piv[0] - x0
    return {
        "body_height_px": h,
        "scale": round(s, 4),
        "opaque_height_px": total_h,
        "opaque_length_px": x1 - x0,
        "front_from_O_px": front,
        "rear_from_O_px": rear,
        "wheel_tip_to_tip_px": round(2 * ad.WHEEL_TIP * s),
        "tub_rim_px": round(104 * s),
        "rider_heads_px": round((ad.SEATS["rider_a"][1] * s) + 25),
        "ratio_body_to_carl": round(h / CARL_H, 2),
        "implied_feet_if_carl_is_6ft": round(6 * h / CARL_H, 1),
        "ceiling_clearance_px": CEILING_CLEARANCE - total_h,
        "view_height_fraction": round(total_h / VIEW_H, 3),
        "view_length_fraction": round((x1 - x0) / VIEW_W, 3),
        "jump_over_wheel_possible": JUMP_APEX >= round(2 * ad.WHEEL_TIP * s),
        "jump_onto_tub_possible": JUMP_APEX >= round(104 * s),
        "rider_to_body_ratio": round(36 / h, 3),
        "atlas_px_parts": parts_px,
        "atlas_px_riders": riders_px,
        "atlas_px_smoke": SMOKE_PX,
        "atlas_px_total_split": parts_px + riders_px + SMOKE_PX,
        "atlas_px_monolithic_estimate": mono,
        "part_frames": {"hull": [hull_w, hull_h], "wheel": [wheel, wheel], "tread": list(tread), "yoke": list(yoke)},
    }


# ------------------------------------------------------------------ scenes

class Chase:
    def __init__(self) -> None:
        self.floor = gr.Floor()
        self.carl = tag_frame(CARL, "run", 2)
        self.donut = tag_frame(DONUT, "run", 4)
        self.vx = self.floor.view[0]

    def entries(self, h: int, sprite=None, piv=None):
        img, p = (sprite, piv) if sprite is not None else machine(h)
        front = self.vx + FRONT_VIEW_X
        o = front - (bbox(img)[2] - 1 - p[0])
        carl_x = front + GAP
        return [(img, p, o, 0), (*self.donut, carl_x + 54, 0), (*self.carl, carl_x, 0)], o, carl_x

    def render(self, h: int, k: int, sprite=None, piv=None) -> Image.Image:
        ents, _o, _c = self.entries(h, sprite, piv)
        return self.floor.render(ents, k)


def label(d, xy, text, fill=WHITE):
    d.text(xy, text, fill=fill, font=gr.font())


def dashed(d, x0, x1, y, colour, step=6):
    for gx in range(x0, x1, step):
        d.line([(gx, y), (gx + 2, y)], fill=colour)


def size_sheet(chase: Chase, scenes: dict[int, Image.Image], metrics: dict[int, dict]) -> Image.Image:
    k = 2
    carl, cp = chase.carl
    donut, dp = chase.donut
    strip_items = [(machine(h), f"h{h}") for h in ad.SIZE_OPTIONS]
    pad, gap = 20, 30
    tops = [p[1] - bbox(img)[1] for (img, p), _ in strip_items]
    top = max(tops + [JUMP_APEX + CARL_H]) + 16
    widths = [bbox(img)[2] - bbox(img)[0] for (img, _p), _ in strip_items]
    total = pad + sum(widths) + gap * 3 + 40 + 30 + pad
    floor_y = top + 2
    strip = Image.new("RGBA", (total, floor_y + 8), BG + (255,))
    d = ImageDraw.Draw(strip)
    d.rectangle([0, floor_y + 1, total, floor_y + 1], fill=gr.GROUND_LIGHT)
    d.rectangle([0, floor_y + 2, total, floor_y + 8], fill=gr.GROUND_DARK)
    x = pad
    anchors = []
    for ((img, p), cap), w in zip(strip_items, widths):
        b = bbox(img)
        strip.alpha_composite(img, (x - b[0], floor_y - p[1]))
        anchors.append((x, cap, w))
        x += w + gap
    strip.alpha_composite(carl, (x + 10 - cp[0], floor_y - cp[1]))
    strip.alpha_composite(donut, (x + 50 - dp[0], floor_y - dp[1]))
    big = gr.up(strip, k)
    d = ImageDraw.Draw(big)
    guides = {CARL_H: (YELLOW, "Carl 52"), JUMP_APEX: (GREEN, "Carl jump apex (feet) 92"),
              JUMP_APEX + CARL_H: (ORANGE, "Carl head at apex 144")}
    for hh, (col, txt) in guides.items():
        y = (floor_y + 1 - hh) * k
        dashed(d, 0, big.width, y, col)
        label(d, (big.width - 210, y - 12), txt, col)
    head = 40
    table_h = 16 * 12
    crop_w = 640
    sheet_w = max(big.width, 3 * (crop_w + 16) + 16)
    crops = []
    for h in ad.SIZE_OPTIONS:
        img = scenes[h]
        crops.append(img.crop((0, 60, VIEW_W * 3, VIEW_H * 3)).resize((crop_w, (VIEW_H * 3 - 60) * crop_w // (VIEW_W * 3)),
                                                                      Image.Resampling.BOX))
    crop_h = crops[0].height
    sheet_h = head + big.height + 30 + table_h + 30 + crop_h + 40
    sheet = Image.new("RGB", (sheet_w, sheet_h), BG)
    ds = ImageDraw.Draw(sheet)
    label(ds, (8, 8), "DOZ-01 size study (proposal; a separate Opus agent decides). Native block-ins x2 on one floor, "
                      "riders unscaled at 36 px. Body height = floor to boiler dome top. Approved Carl 52 / Donut 40 at right.")
    label(ds, (8, 22), "Guides: Carl height, his jump apex (feet 92 px, from JUMP_SPEED 440 and GRAVITY 1050) and his "
                       "head at the apex. Corridor clearance floor to ceiling collider: 276 px.", GREY)
    sheet.paste(big, (0, head))
    for px, cap, w in anchors:
        h = int(cap[1:])
        label(ds, (px * k, head + 6), f"{cap}" + ("  (recommended)" if h == RECOMMENDED else ""),
              YELLOW if h == RECOMMENDED else WHITE)
    y = head + big.height + 14
    cols = [("", 230)] + [(f"h{h}" + (" (rec.)" if h == RECOMMENDED else ""), 170) for h in ad.SIZE_OPTIONS]
    rows = [
        ("opaque height / length (px)", lambda m: f"{m['opaque_height_px']} / {m['opaque_length_px']}"),
        ("view fraction (h / w)", lambda m: f"{m['view_height_fraction']:.2f} / {m['view_length_fraction']:.2f}"),
        ("ceiling clearance (px)", lambda m: f"{m['ceiling_clearance_px']}"),
        ("body / Carl, implied height", lambda m: f"{m['ratio_body_to_carl']}x, {m['implied_feet_if_carl_is_6ft']} ft"),
        ("wheel tip-to-tip (px)", lambda m: f"{m['wheel_tip_to_tip_px']}"),
        ("tub rim (px)", lambda m: f"{m['tub_rim_px']}"),
        ("jump over wheel / onto tub", lambda m: f"{'yes' if m['jump_over_wheel_possible'] else 'no'} / "
                                                 f"{'yes' if m['jump_onto_tub_possible'] else 'no'}"),
        ("rider 36 px / body", lambda m: f"{m['rider_to_body_ratio']:.2f}"),
        ("atlas px, split parts", lambda m: f"{m['atlas_px_total_split']:,}"),
        ("atlas px, monolithic est.", lambda m: f"{m['atlas_px_monolithic_estimate']:,}"),
    ]
    x = 8
    for name, w in cols:
        label(ds, (x, y), name, YELLOW if "rec" in name else WHITE)
        x += w
    for i, (name, fn) in enumerate(rows):
        x = 8
        yy = y + 16 * (i + 1)
        label(ds, (x, yy), name, GREY)
        x += cols[0][1]
        for (cname, w), h in zip(cols[1:], ad.SIZE_OPTIONS):
            label(ds, (x, yy), fn(metrics[h]))
            x += w
    y += table_h + 20
    label(ds, (8, y), "Lit Floor 1 (1920x1080 render, scaled to 1/3 for the sheet): the Dozer chasing; Carl 213 px "
                      "(one third of the view) ahead of the wheel, Donut beside him.")
    x = 16
    for h, c in zip(ad.SIZE_OPTIONS, crops):
        sheet.paste(c, (x, y + 18))
        label(ds, (x, y + 22 + c.height), f"h{h}" + ("  (recommended)" if h == RECOMMENDED else ""),
              YELLOW if h == RECOMMENDED else WHITE)
        x += c.width + 16
    return sheet


def boxed(img: Image.Image, k: int, piv, bg=(36, 34, 50)) -> Image.Image:
    cell = Image.new("RGBA", img.size, bg + (255,))
    cell.alpha_composite(img)
    big = gr.up(cell, k)
    d = ImageDraw.Draw(big)
    d.rectangle([0, 0, big.width - 1, big.height - 1], outline=GREY)
    px, py = piv
    d.rectangle([px * k, py * k, px * k + k - 1, py * k + k - 1], outline=YELLOW)
    return big


def key_sheet(rig: dict) -> Image.Image:
    hull_f, hull_p, _ = grid_imgs(f"{DOZER}/dozer_hull_h120.grid.json")
    wheel_f, wheel_p, _ = grid_imgs(f"{DOZER}/dozer_wheel_h120.grid.json")
    tread_f, tread_p, _ = grid_imgs(f"{DOZER}/dozer_tread_h120.grid.json")
    yoke_f, yoke_p, _ = grid_imgs(f"{DOZER}/dozer_yoke_h120.grid.json")
    rid_f, rid_p, _ = grid_imgs(f"{DOZER}/dozer_riders_h36.grid.json")
    comp, _cp = compose_rig(rig)
    blocks = []
    # row 1: assembled machine x3 and x1
    b = bbox(comp)
    c3 = gr.up(comp.crop(b), 3)
    blocks.append(("Assembled from the part keys with the rig contract (x3; x1 inset). Wheel phase 0, tread phase 0.",
                   [(c3, ""), (comp.crop(b), "x1")]))
    blocks.append(("Hull key x3: frame 0 'body' (behind riders) and frame 1 'front' (tub and cab rail, over riders). "
                   "Yellow = pivot (rig origin O).",
                   [(boxed(hull_f[0], 3, hull_p), "body 175x160"), (boxed(hull_f[1], 3, hull_p), "front 175x160")]))
    plan = []
    for i in range(4):
        rows, c0 = ad.wheel_rows(7.5 * i)
        plan.append((boxed(rows_to_img(ad.despeckle(rows)), 2, (c0, c0)), f"plan phase {i} ({7.5 * i:.1f} deg)"))
    blocks.append(("Wheel key x4 (phase 0, pivot = axle) and the rotation plan x2: 12 spikes, 4 frames of 7.5 deg "
                   "cover one 30 deg spike period (plan previews from the same geometry, not keys).",
                   [(boxed(wheel_f[0], 4, wheel_p), "key 99x99")] + plan))
    tplan = []
    for i in range(3):
        rows, tp = ad.tread_rows(i)
        tplan.append((boxed(rows_to_img(ad.despeckle(rows)), 2, tp), f"plan phase {i} (+{3 * i} px)"))
    blocks.append(("Tread key x4 (phase 0, pivot = footprint midpoint on the contact row) and the belt plan x2: "
                   "9 px link pitch, 3 frames of 3 px, advanced by distance travelled.",
                   [(boxed(tread_f[0], 4, tread_p), "key 151x34")] + tplan))
    blocks.append(("Rider keys x6 (pivot = seat point): engineer (key: pot helmet, pointing, hand on the lever), "
                   "rider A (key: GOB-01 tell seated), rider B (derived: GOB-01 B idle seated). Yoke x4 (pivot = axle).",
                   [(boxed(rid_f[0], 6, rid_p), "engineer_point"), (boxed(rid_f[1], 6, rid_p), "rider_a_brandish"),
                    (boxed(rid_f[2], 6, rid_p), "rider_b_hold (derived)"), (boxed(yoke_f[0], 4, yoke_p), "yoke 64x44")]))
    width = 16 + max(sum(im.width + 16 for im, _ in items) for _t, items in blocks)
    height = 30 + sum(30 + max(im.height for im, _ in items) + 22 for _t, items in blocks)
    sheet = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (8, 8), "DOZ-01 keys at the proposed 120 px body height (design: Claude Opus). Palette dozer-floor01-v0 + "
                     "GOB-01 goblin palette. Authored facing right; the runtime mirrors around each pivot.")
    y = 30
    for title, items in blocks:
        label(d, (8, y + 8), title)
        y += 26
        x = 16
        for im, cap in items:
            sheet.paste(im, (x, y), im if im.mode == "RGBA" else None)
            if cap:
                label(d, (x, y + im.height + 4), cap, GREY)
            x += im.width + 16
        y += max(im.height for im, _ in items) + 22
    return sheet


def layout_diagram(rig: dict) -> Image.Image:
    k = 3
    comp, (ox, fy) = compose_rig(rig)
    big = gr.up(Image.alpha_composite(Image.new("RGBA", comp.size, (36, 34, 50, 255)), comp), k)
    d = ImageDraw.Draw(big)

    def P(x, y):  # rig coords (relative to O, y up) -> big px (pixel centre)
        return ((ox + x) * k + k // 2, (fy - y) * k + k // 2)
    # floor
    d.line([P(-110, 0)[0], (fy + 1) * k, P(200, 0)[0], (fy + 1) * k], fill=CYAN)
    # hazard: wheel lethal circle r 44 (proposal); visual tips r 48
    cx, cy = P(*next(p["anchor"] for p in rig["parts"] if p["part"] == "wheel"))
    for r, col in ((44, RED), (48, (255, 160, 160))):
        d.ellipse([cx - r * k, cy - r * k, cx + r * k, cy + r * k], outline=col, width=2 if r == 44 else 1)
    # pushing wall: tread rear to tub front, floor to tub rim
    x0, y0 = P(-74, 104)
    x1, y1 = P(79, 0)
    d.rectangle([x0, y0, x1, y1], outline=BLUE, width=2)
    # occluder rows
    for name, row in rig["occluder_rows"].items():
        ax = next(p["anchor"] for p in rig["parts"] if p["part"] == name)
        a, yy = P(ax[0] - 14, row)
        b, _ = P(ax[0] + 18, row)
        d.line([a, yy, b, yy], fill=ORANGE, width=1)
    # anchors and emitters
    seen: dict[tuple, int] = {}
    for p in rig["parts"]:
        if isinstance(p.get("anchor"), list):
            x, y = P(*p["anchor"])
            n = seen.get((x, y), 0)
            seen[(x, y)] = n + 1
            d.line([x - 6, y, x + 6, y], fill=YELLOW); d.line([x, y - 6, x, y + 6], fill=YELLOW)
            label(d, (x + 6, y + 2 + 13 * n), f"z{p['z']} {p['part']}", YELLOW)
    for name, (ex, ey) in rig["emitters"].items():
        x, y = P(ex, ey)
        d.ellipse([x - 5, y - 5, x + 5, y + 5], outline=WHITE)
        label(d, (x - 30, y - 16), f"smoke: {name}", WHITE)
    for name, (lx, ly) in rig["lights"].items():
        x, y = P(lx, ly)
        d.ellipse([x - 4, y - 4, x + 4, y + 4], outline=ORANGE, width=2)
        label(d, (x + 6, y - 14), f"light: {name}", ORANGE)
    ox0, oy0 = P(0, 0)
    d.rectangle([ox0 - 4, oy0 - 4, ox0 + 4, oy0 + 4], outline=GREEN, width=2)
    label(d, (ox0 - 20, oy0 + 8), "O (rig origin)", GREEN)
    # exploded parts row
    parts = []
    for p in sorted([p for p in rig["parts"] if "tag" in p], key=lambda q: q["z"]):
        frames, piv, grid = grid_imgs(f"{DOZER}/{p['atlas']}.grid.json")
        if p["atlas"] == "dozer_riders_h36":
            img = frames[{"engineer": 0, "rider_a": 1, "rider_b": 2}[p["part"]]]
        else:
            t = next(t for t in grid["tags"] if t["name"] == p["tag"])
            img = frames[t["from"]]
        parts.append((p, boxed(img, 2, piv)))
    row_w = 16 + sum(im.width + 16 for _p, im in parts)
    row_h = max(im.height for _p, im in parts) + 60
    timeline_h = 150
    W = max(big.width + 360, row_w)
    H = 40 + big.height + 30 + row_h + timeline_h
    sheet = Image.new("RGB", (W, H), BG)
    ds = ImageDraw.Draw(sheet)
    label(ds, (8, 8), "DOZ-01 construction (proposal). Assembled rig x3: yellow crosses = part anchors (pivot goes on "
                      "anchor), green = rig origin O, cyan = walking line.")
    label(ds, (8, 22), "Red circle = proposed lethal wheel box (r 44; visual tips r 48). Blue = proposed pushing wall "
                       "(no damage). Orange lines = occluder rows over each seat. White = smoke emitters.", GREY)
    sheet.paste(big, (8, 40))
    lx = big.width + 24
    legend = ["Draw order (back to front):"] + [
        f"z{p['z']} {p['part']}: {p.get('atlas')}" + (f" [{p['tag']}]" if 'tag' in p else "") for p in rig["parts"]] + [
        "", "Anchors (x fwd, y up from O):"] + [
        f"{p['part']}: {p['anchor']}" for p in rig["parts"] if isinstance(p.get("anchor"), list)] + [
        "", "Facing left: mirror every anchor x", "and flip each frame around its pivot."]
    for i, line in enumerate(legend):
        label(ds, (lx, 46 + 14 * i), line, WHITE if line.endswith(":") else GREY)
    y = 40 + big.height + 20
    label(ds, (8, y), "Exploded parts x2 (frame box grey, pivot yellow):")
    x = 16
    for p, im in parts:
        sheet.paste(im, (x, y + 18))
        label(ds, (x, y + 22 + im.height), f"z{p['z']} {p['part']} {p['frame_size'][0]}x{p['frame_size'][1]}", GREY)
        label(ds, (x, y + 36 + im.height), f"pivot {tuple(p['pivot'])}", GREY)
        x += im.width + 16
    # charge timeline (proposal, 60 Hz ticks)
    y = 40 + big.height + 30 + row_h
    label(ds, (8, y), "Segment D charge (proposal, ticks at 60 Hz): tell 36 -> charge until overshoot -> brake 24 -> "
                      "stuck/turn 90 -> resume chase.")
    segs = [("tell 36", 36, ORANGE), ("charge (<= 90)", 90, RED), ("brake 24", 24, YELLOW),
            ("stuck / turn 90", 90, BLUE), ("resume", 40, GREEN)]
    x, scale = 16, 4
    for name, ticks, col in segs:
        ds.rectangle([x, y + 22, x + ticks * scale - 2, y + 46], outline=col, width=2)
        label(ds, (x + 4, y + 28), name, col)
        x += ticks * scale
    notes = ["tell: big black puff + kettle shriek, riders lurch back, hull squats 1 px at the rear, wheel 1 tick/frame",
             "charge: straight line, cannot steer; brake: treads stop, rig slides 4 px, riders lurch forward, steam burst",
             "stuck/turn: rattle x2 (1 px every 3 ticks), puffs from every emitter; the escape window for Carl"]
    for i, n in enumerate(notes):
        label(ds, (16, y + 56 + 14 * i), n, GREY)
    return sheet


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="DOZ-01 phase 1 review renders")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    out = args.out if args.out.is_absolute() else ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)
    rig = json.loads((ROOT / DOZER / "dozer_rig_h120.json").read_text(encoding="utf-8"))
    chase = Chase()
    written: list[Path] = []

    def save(img: Image.Image, name: str) -> None:
        path = out / name
        img.save(path, format="PNG", optimize=False)
        written.append(path)

    metrics = {h: option_metrics(h) for h in ad.SIZE_OPTIONS}
    scenes3 = {}
    for h in ad.SIZE_OPTIONS:
        save(chase.render(h, 1), f"floor01_chase_h{h}_640x360.png")
        scenes3[h] = chase.render(h, 3)
        save(scenes3[h], f"floor01_chase_h{h}_1920x1080.png")
    # the recommended option assembled from its part keys (not the block-in)
    comp, cpiv = compose_rig(rig)
    save(chase.render(RECOMMENDED, 1, comp, cpiv), "floor01_chase_keys_h120_640x360.png")
    save(chase.render(RECOMMENDED, 3, comp, cpiv), "floor01_chase_keys_h120_1920x1080.png")
    save(size_sheet(chase, scenes3, metrics), "size_study_sheet.png")
    save(key_sheet(rig), "key_sheet.png")
    save(layout_diagram(rig), "part_layout_diagram.png")

    measurements = {
        "note": "Composed review renders (A1 lighting via scripts/env_kit_light.py), not Bevy captures.",
        "reference": {"carl_h52": CARL_H, "carl_jump_apex_feet_px": JUMP_APEX,
                      "corridor_clearance_px": CEILING_CLEARANCE, "chase_gap_px": GAP,
                      "carl_run_speed_px_s": 185},
        "size_options": {f"h{h}": m for h, m in metrics.items()},
        "recommended": f"h{RECOMMENDED}",
        "keys": {},
        "files": {},
    }
    for stem in ("dozer_hull_h120", "dozer_wheel_h120", "dozer_tread_h120", "dozer_yoke_h120", "dozer_riders_h36"):
        grid = pg.load_grid(ROOT / DOZER / f"{stem}.grid.json")
        lint = pg.lint_grid(grid)
        measurements["keys"][stem] = {"canvas": grid["canvas"], "pivot": grid["pivot"],
                                      "colours": len(grid["palette"]),
                                      "orphans": lint["orphan_count_per_frame"],
                                      "isolated": lint["isolated_count_per_frame"]}
    for path in written:
        measurements["files"][path.name] = sha(path)
    mpath = out / "measurements.json"
    mpath.write_text(json.dumps(measurements, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for path in written + [mpath]:
        print(f"  {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
