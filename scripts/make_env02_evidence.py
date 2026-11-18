#!/usr/bin/env python3
"""Render the ENV-02 evidence (key sheet, route plan, ceiling options, lit frames).

Inputs are the exported atlases (``bash scripts/export_assets.sh``) and the committed
ENV-02 contract and route layouts. Everything is a deterministic **art preview** made
with the ENV-01 Pillow lighting model (``scripts/env_kit_light.py``): not a Bevy capture,
not gameplay evidence. Actors are frame 0 of the approved Carl 52 / Donut 40 atlases and
the GOB-01 h36 goblin candidates, placed on the walking line with 0 px sink.

Usage::

    python3 scripts/make_env02_evidence.py --out docs/evidence/env-02
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import env_kit as ek  # noqa: E402
import env_kit_light as el  # noqa: E402
from validate_assets import safe_path  # noqa: E402

LAYOUT_R = "assets/levels/floor01/opening_route_proposal.json"
LAYOUT_T = "assets/levels/floor01/opening_route_proposal_ceiling_t.json"
CARL = "assets/manifests/carl_book1_h52.json"
DONUT = "assets/manifests/donut_scale_h40.json"
GOB_A = "assets/manifests/goblin_a_h36.json"
GOB_B = "assets/manifests/goblin_b_h36.json"
# frame durations of the presentation loops, in ms (60 Hz ticks: 7, 5 and 4)
TRICKLE_MS, IMPACT_MS, BURST_MS = 117, 83, 67
PAGE = (20, 18, 26)
INK = (236, 224, 200)
KIND_COLOURS = {
    "spawn": (120, 220, 120), "checkpoint": (120, 220, 120), "exit": (120, 220, 120),
    "protected": (90, 200, 255), "trigger": (255, 255, 255), "pursuer_spawn": (255, 80, 80),
    "pursuer_path": (255, 80, 80), "pursuer_end": (255, 80, 80), "pursuer_charge": (255, 120, 60),
    "pursuer_breakable": (255, 170, 60), "scripted_blocker": (255, 170, 60),
    "clearance_gauge": (200, 120, 255), "safe_pocket": (90, 200, 255), "jump_lane": (255, 230, 90),
    "breakable_prop": (255, 230, 90), "falling_debris": (255, 230, 90), "hazard": (255, 60, 160),
    "enemy_spawn": (160, 255, 90),
}


def text(draw: ImageDraw.ImageDraw, xy, s: str, fill=INK) -> None:
    x, y = xy
    draw.text((x + 1, y + 1), s, fill=(0, 0, 0))
    draw.text((x, y), s, fill=fill)


# ---------------------------------------------------------------------------
# Actors
# ---------------------------------------------------------------------------

def sprite(manifest_path: str, frame: int = 0, mirror: bool = False):
    manifest = json.loads(safe_path(manifest_path).read_text(encoding="utf-8"))
    meta = json.loads(safe_path(manifest["output_metadata"]).read_text(encoding="utf-8"))
    with Image.open(safe_path(manifest["output_atlas"])) as opened:
        atlas = opened.convert("RGBA")
    fr = meta["frames"][frame]["frame"]
    img = atlas.crop((fr["x"], fr["y"], fr["x"] + fr["w"], fr["y"] + fr["h"]))
    px, py = manifest["pivot"]
    if mirror:
        img = img.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        px = img.width - 1 - px
    return img, (px, py)


def actors(kit: ek.Kit, layout: dict, entries: list[tuple]) -> tuple[Image.Image, list]:
    """entries: (manifest, pivot x, mirror). Pivot row on walking line - 1 (0 px sink)."""
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    image = Image.new("RGBA", tuple(layout["pixel_size"]), (0, 0, 0, 0))
    boxes = []
    for manifest, x, mirror in entries:
        img, (px, py) = sprite(manifest, mirror=mirror)
        top = tops[x // ek.TILE]
        pos = (x - px, top - 1 - py)
        image.alpha_composite(img, pos)
        boxes.append((pos[0], pos[1], pos[0] + img.width, pos[1] + img.height))
    return image, boxes


# ---------------------------------------------------------------------------
# Layout variants (art preview only: swap a piece for another state)
# ---------------------------------------------------------------------------

def swap(layout: dict, old: str, new: str | None, dx: int = 0, dy: int = 0) -> dict:
    out = copy.deepcopy(layout)
    for name, items in out["piece_layers"].items():
        for i, item in enumerate(items):
            if item["piece"] == old:
                if new is None:
                    items.pop(i)
                else:
                    item["piece"] = new
                    item["x"] += dx
                    item["y"] += dy
                return out
    raise KeyError(old)


def add(layout: dict, layer: str, piece: str, x: int, y: int) -> dict:
    out = copy.deepcopy(layout)
    out["piece_layers"][layer].append({"piece": piece, "x": x, "y": y})
    return out


def marker(layout: dict, mid: str) -> dict:
    return next(m for m in layout["markers"] if m["id"] == mid)


# ---------------------------------------------------------------------------
# Images
# ---------------------------------------------------------------------------

def key_sheet(kit: ek.Kit, factor: int) -> Image.Image:
    groups = [
        ("CP-03 raking shore: intact / damaged_1 / damaged_2 / broken (3 hits)",
         ["shore_post_intact", "shore_post_damaged_1", "shore_post_damaged_2", "shore_post_broken"]),
        ("loose block: seated / settling / slipped / falling (debris) / landed; wall cavity; hazard grit",
         ["loose_block_seated", "loose_block_settling", "loose_block_slipped", "loose_block_falling",
          "loose_block_landed",
          "wall_cavity", "hazard_grit"]),
        ("pursuer-smashed timber (presentation + authored event): timber set (the old 96x176 low barricade "
         "is DEPRECATED, background only, and is not shown)",
         ["timber_set_intact", "timber_set_smashed"]),
        ("full-height barricade 96x280 (cornice to the deck front lip: crosses the lane, covers the blocker): intact / smashed",
         ["timber_barricade_full_intact", "timber_barricade_full_smashed"]),
        ("dodge pocket and maintenance hint: alcove niche (backwall decal), iron hatch",
         ["alcove_niche", "maint_hatch"]),
        ("guild exit: portal closed / ajar / open, hanging sign (lantern pictogram, no text)",
         ["guild_portal_closed", "guild_portal_ajar", "guild_portal_open", "guild_sign"]),
        ("presentation frames (frame 0 of each loop; the 3x sheet scales every frame equally)",
         ["shore_dust_trickle", "block_impact_dust", "timber_splinter_burst"]),
    ]
    anim_frames = {"shore_dust_trickle": 3, "block_impact_dust": 4, "timber_splinter_burst": 4}
    tile_ids = ["soffit_end_l", "soffit_end_l_b", "soffit_end_r", "soffit_end_r_b", "soffit_corner_l",
                "soffit_corner_r", "soffit_joint_l", "soffit_joint_r", "soffit_end_l_c", "soffit_end_r_c"]
    pad, gap, label_h = 8, 10, 14
    under = (52, 46, 58, 255)
    rows = []
    for title, ids in groups:
        imgs = []
        for pid in ids:
            piece = kit.pieces[pid]
            if pid in anim_frames:     # all frames of the loop, side by side
                for i, fr in enumerate(piece["animation"]["frames"]):
                    rect = [fr["frame"][k] for k in ("x", "y", "w", "h")]
                    imgs.append((f"{pid} f{i}", ek.scale(kit.crop(piece["atlas"], rect), factor)))
            else:
                imgs.append((pid, ek.scale(kit.crop(piece["atlas"], piece["rect"]), factor)))
        rows.append((title, imgs))
    tiles = [(tid, ek.scale(kit.crop(kit.tiles[tid]["atlas"], kit.tiles[tid]["rect"]), factor * 2))
             for tid in tile_ids]
    rows.append(("soffit tiles (collision tileset, 2x the sheet scale): l = left step (col 61), r = "
                 "right step (col 104), _b = 2nd quoin phase, _c = cut-end face under the blue course", tiles))
    def slot(pid: str, img: Image.Image) -> int:
        return max(img.width, 6 * len(pid) + 4) + gap

    width = max(sum(slot(p, i) for p, i in imgs) + pad * 2 for _t, imgs in rows)
    height = sum(label_h * 2 + max(i.height for _p, i in imgs) + pad * 2 for _t, imgs in rows)
    sheet = Image.new("RGBA", (max(width, 760), height), PAGE + (255,))
    draw = ImageDraw.Draw(sheet)
    y = pad
    for title, imgs in rows:
        text(draw, (pad, y), title)
        y += label_h
        x = pad
        for pid, img in imgs:
            text(draw, (x, y), pid, fill=(190, 180, 170))
            sheet.paste(Image.new("RGBA", img.size, under), (x, y + label_h))
            sheet.alpha_composite(img, (x, y + label_h))
            x += slot(pid, img)
        y += label_h + max(i.height for _p, i in imgs) + pad * 2
    return sheet


def route_plan(kit: ek.Kit, layout: dict) -> Image.Image:
    """Unlit full route at 1x, split into two stacked halves, with segment and marker overlays."""
    w, h = layout["pixel_size"]
    full = ek.render_layout(kit, layout).convert("RGBA")
    over = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(over)
    for seg in layout["segments"]:
        x0, x1 = seg["x"]
        draw.line([(x0, 0), (x0, h)], fill=(255, 255, 255, 160), width=1)
        draw.rectangle([x0 + 2, 2, x0 + 7 * len(seg["id"]) + 60, 14], fill=(0, 0, 0, 170))
        draw.text((x0 + 4, 3), f"{seg['id']} (beat {seg['beat']})", fill=INK)
    for m in layout["markers"]:
        colour = KIND_COLOURS.get(m["kind"], (255, 255, 255))
        if "rect" in m:
            rx, ry, rw, rh = m["rect"]
            draw.rectangle([rx, ry, rx + rw - 1, ry + rh - 1], outline=colour + (255,), width=1)
            if m["kind"] in ("hazard", "safe_pocket", "protected"):
                draw.rectangle([rx, ry, rx + rw - 1, ry + rh - 1], fill=colour + (46,))
        if m["kind"] == "pursuer_path":
            pts = [tuple(p) for p in m["points"]]
            draw.line([(x, y - 70) for x, y in pts], fill=colour + (255,), width=2)
            for x, y in pts:
                draw.ellipse([x - 3, y - 73, x + 3, y - 67], outline=colour + (255,))
        mx, my = m["x"], m["y"]
        draw.line([(mx - 4, my), (mx + 4, my)], fill=colour + (255,))
        draw.line([(mx, my - 4), (mx, my + 4)], fill=colour + (255,))
    full.alpha_composite(over)
    labels = Image.new("RGBA", (w, 120), PAGE + (255,))
    ld = ImageDraw.Draw(labels)
    halves: dict[bool, list[int]] = {True: [], False: []}   # per half: right end of each lane
    for m in sorted(layout["markers"], key=lambda m: m["x"]):
        colour = KIND_COLOURS.get(m["kind"], (255, 255, 255))
        x = m["x"]
        label_w = 6 * len(f"{m['id']} [{m['kind']}]") + 8
        lanes = halves[x < w // 2]
        lane = next((i for i, end in enumerate(lanes) if end < x), len(lanes))
        if lane == len(lanes):
            lanes.append(0)
        lanes[lane] = x + label_w
        ld.line([(x, 0), (x, 6 + lane * 14)], fill=colour)
        ld.text((x + 2, 2 + lane * 14), f"{m['id']} [{m['kind']}]", fill=colour)
    half = w // 2
    out = Image.new("RGBA", (half, (h + 120) * 2 + 8), PAGE + (255,))
    for i in range(2):
        out.alpha_composite(full.crop((i * half, 0, (i + 1) * half, h)), (0, i * (h + 128)))
        out.alpha_composite(labels.crop((i * half, 0, (i + 1) * half, 120)), (0, i * (h + 128) + h))
    return out


def ceiling_options(kit: ek.Kit, layouts: dict) -> Image.Image:
    panels = []
    for opt, layout in layouts.items():
        x0, x1 = 928, 1728
        img = ek.render_layout(kit, layout).crop((x0, 0, x1, 400)).convert("RGBA")
        gauge = marker(layout, "clearance_gauge_s3")
        over = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(over)
        under = gauge["rect"][1]
        walk = layout["base_walk_line_y"]
        doz = layout["design_parameters"]["pursuer_interface"]
        for hgt, length, colour, lx, name in (
                (doz["total_height_px"], doz["length_px"], (255, 80, 80), 1160, "DOZ-01 h120 pursuer"),
                (doz["ceiling_recommended_px"], 24, (120, 220, 255), 1480, "recommended")):
            bx = lx - x0
            draw.rectangle([bx, walk - hgt, bx + length, walk - 1], outline=colour + (255,), width=2)
            draw.text((bx + 4, walk - hgt + 4), f"{name} {hgt} px", fill=colour)
        ax = 1100 - x0
        draw.line([(ax, under), (ax, walk)], fill=(200, 120, 255, 255), width=2)
        draw.text((ax + 4, (under + walk) // 2), f"clear {walk - under} px", fill=(200, 120, 255))
        img.alpha_composite(over)
        p = layout["design_parameters"]["pursuer_clearance"]["options"][opt]
        panel = Image.new("RGBA", (img.width, img.height + 34), PAGE + (255,))
        panel.alpha_composite(img, (0, 34))
        d = ImageDraw.Draw(panel)
        text(d, (6, 4), f"Option {opt}: cornice underside y {p['underside_y']}, clearance "
             f"{p['clearance_px']} px, fits pursuer total height <= {p['fits_pursuer_total_height_max_px']} px")
        text(d, (6, 18), p["note"] + " (the pipe run is on the back wall, not over the lane)")
        panels.append(ek.scale(panel, 2))
    out = Image.new("RGBA", (panels[0].width, sum(p.height for p in panels) + 16), PAGE + (255,))
    y = 0
    for p in panels:
        out.alpha_composite(p, (0, y))
        y += p.height + 16
    return out


def label(img: Image.Image, s: str) -> Image.Image:
    return el.label(img.convert("RGB"), s)


def save(img: Image.Image, path: Path, written: list[Path]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.convert("RGB").save(path, format="PNG", optimize=False)
    written.append(path)


def save_gif(frames: list[tuple[Image.Image, int]], path: Path, written: list[Path]) -> None:
    """Deterministic GIF: one shared 255-colour palette from the first frame, no dithering."""
    path.parent.mkdir(parents=True, exist_ok=True)
    rgb = [f.convert("RGB") for f, _ms in frames]
    big = Image.new("RGB", (rgb[0].width, sum(f.height for f in rgb[:6])))
    y = 0
    for f in rgb[:6]:
        big.paste(f, (0, y))
        y += f.height
    base = big.quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    pal = [f.quantize(palette=base, dither=Image.Dither.NONE) for f in rgb]
    pal[0].save(path, format="GIF", save_all=True, append_images=pal[1:], duration=[ms for _f, ms in frames],
                loop=0, optimize=False, disposal=1)
    written.append(path)


def render(out: Path) -> dict:
    layout = ek.load_layout(LAYOUT_R)
    layout_t = ek.load_layout(LAYOUT_T)
    kit = ek.load_kit(layout["kit_contract"])
    ek.validate_contract(kit)
    ek.validate_layout(kit, layout)
    ek.validate_layout(kit, layout_t)
    written: list[Path] = []
    for f in (1, 3):
        save(key_sheet(kit, f), out / f"env02_key_sheet_{f}x.png", written)
    save(route_plan(kit, layout), out / "env02_route_plan_unlit.png", written)
    save(ceiling_options(kit, {"R": layout, "T": layout_t}), out / "env02_ceiling_options_unlit_2x.png",
         written)
    walk = layout["base_walk_line_y"]

    def both_scales(name: str, lay: dict, view: list[int], act, box) -> Image.Image:
        lit1 = el.render_lit(kit, lay, view, 1, act, boxes=box)
        save(lit1, out / f"env02_lit_{name}_640x360.png", written)
        save(el.render_lit(kit, lay, view, 3, act, boxes=box), out / f"env02_lit_{name}_1920x1080.png", written)
        return lit1

    # --- S1 intro: Carl at the spawn, Donut behind him, a goblin at scale (reference) near the door
    cast1 = [(CARL, 104, False), (DONUT, 62, False), (GOB_A, 330, True)]
    act1, box1 = actors(kit, layout, cast1)
    both_scales("s1_intro", layout, layout["views"]["s1_intro_640x360"], act1, box1)
    # --- S2 arch bay: the pursuer's entry window; Carl and Donut crossing, a goblin at scale
    cast2 = [(CARL, 700, False), (DONUT, 656, False), (GOB_B, 860, True)]
    act2, box2 = actors(kit, layout, cast2)
    both_scales("s2_arch", layout, layout["views"]["s2_arch_640x360"], act2, box2)
    # --- S3 chase passage, both ceiling options: Carl and Donut fleeing right, a goblin rider behind
    s3 = layout["views"]["s3_chase_640x360"]
    for opt, lay in (("R", layout), ("T", layout_t)):
        act3, box3 = actors(kit, lay, [(CARL, 1328, False), (DONUT, 1290, False), (GOB_A, 1180, False)])
        both_scales(f"s3_chase_option_{opt}", lay, s3, act3, box3)

    shore = marker(layout, "breakable_shore")
    strike_x = shore["x"]
    # --- S4 widened: Carl and Donut pressed into the alcove, waiting for the charge; the full-height
    # barricade closes the passage on the right; a goblin at scale in front of it
    s4 = layout["views"]["s4_widened_640x360"]
    pocket = marker(layout, "safe_pocket_alcove")
    act4, box4 = actors(kit, layout, [(CARL, pocket["x"] - 18, False), (DONUT, pocket["x"] + 22, True),
                                      (GOB_B, 2240, True)])
    both_scales("s4_widened", layout, s4, act4, box4)
    # --- S5 skirmish: Carl at the strike position, Donut behind, three goblins (two in the fall region)
    s5 = layout["views"]["s5_skirmish_640x360"]
    cast5 = [(CARL, strike_x - 19, False), (DONUT, strike_x - 64, False), (GOB_A, 3024, True),
             (GOB_B, 3060, True), (GOB_A, 3150, True)]
    act5, box5 = actors(kit, layout, cast5)
    lit5 = both_scales("s5_skirmish", layout, s5, act5, box5)
    # --- S6 guild: closed (no actors), open (Carl walking in, Donut following)
    s6 = layout["views"]["s6_guild_640x360"]
    open6 = swap(layout, "guild_portal_closed", "guild_portal_open")
    exit_m = marker(layout, "exit_guild")
    act6, box6 = actors(kit, open6, [(CARL, exit_m["x"] - 30, False), (DONUT, exit_m["x"] - 90, False)])
    save(el.render_lit(kit, layout, s6, 1), out / "env02_lit_s6_guild_closed_640x360.png", written)
    save(el.render_lit(kit, layout, s6, 3), out / "env02_lit_s6_guild_closed_1920x1080.png", written)
    save(el.render_lit(kit, open6, s6, 1, act6, boxes=box6), out / "env02_lit_s6_guild_open_640x360.png", written)
    save(el.render_lit(kit, open6, s6, 3, act6, boxes=box6), out / "env02_lit_s6_guild_open_1920x1080.png", written)

    # --- Guild door states closed / ajar / open (lit, 2x) and the opening as a GIF
    dview = [3216, 130, 320, 230]
    states6 = [("closed", "guild_portal_closed", 110, 90), ("ajar", "guild_portal_ajar", 78, 130),
               ("open", "guild_portal_open", 36, 90)]
    panels, gif6 = [], []
    for name, pid, carl_dx, donut_dx in states6:
        lay = swap(layout, "guild_portal_closed", pid) if pid != "guild_portal_closed" else layout
        act, box = actors(kit, lay, [(CARL, exit_m["x"] - carl_dx, False), (DONUT, exit_m["x"] - donut_dx - 40, False)])
        img = el.render_lit(kit, lay, dview, 2, act, boxes=box)
        panels.append(label(img, f"guild door: {name}"))
        gif6.append((panels[-1], {"closed": 900, "ajar": 500, "open": 1400}[name]))
    strip = Image.new("RGB", (sum(p.width for p in panels) + 8 * 2, panels[0].height), PAGE)
    x = 0
    for p in panels:
        strip.paste(p, (x, 0))
        x += p.width + 8
    save(strip, out / "env02_guild_door_states_lit_2x.png", written)
    save_gif(gif6, out / "env02_guild_door_opening_lit_2x.gif", written)

    # --- Soffit steps of the lowered ceiling, option R and T, left and right step (lit, 3x)
    soff = []
    for opt, lay in (("R", layout), ("T", layout_t)):
        for side, vx in (("left step (column 61)", 912), ("right step (column 104)", 1632)):
            soff.append(label(el.render_lit(kit, lay, [vx, 0, 128, 220], 3), f"option {opt}, {side}"))
    strip = Image.new("RGB", (sum(p.width for p in soff) + 8 * 3, soff[0].height), PAGE)
    x = 0
    for p in soff:
        strip.paste(p, (x, 0))
        x += p.width + 8
    save(strip, out / "env02_soffit_steps_lit_3x.png", written)

    # --- CP-03 state sequence in scene (lit, nearest 2x crops) and the same as a GIF
    crop_box = (2896, 100, 3216, 330)
    view = [crop_box[0], crop_box[1], crop_box[2] - crop_box[0], crop_box[3] - crop_box[1]]
    block = marker(layout, "falling_block")
    bx, by = block["rect"][0], block["rect"][1]
    hazard = marker(layout, "hazard_fall_region")["rect"]
    centre_x = hazard[0] + hazard[2] // 2
    strike = (strike_x, walk - 28)
    trickle = (bx + 41 - 4, by + 46)
    burst = (strike[0] - 24, strike[1] - 24)
    shore_states = [("shore_post_intact", "loose_block_seated"),
                    ("shore_post_damaged_1", "loose_block_settling"),
                    ("shore_post_damaged_2", "loose_block_slipped")]

    def state_layout(shore_piece: str, block_piece: str | None, extra=()) -> dict:
        lay = swap(layout, "shore_post_intact", shore_piece) if shore_piece != "shore_post_intact" else layout
        if block_piece is None:
            lay = swap(lay, "loose_block_seated", None)
        elif block_piece != "loose_block_seated":
            lay = swap(lay, "loose_block_seated", block_piece)
        for layer, piece, x, y in extra:
            lay = add(lay, layer, piece, x, y)
        return lay

    def frame(lay: dict, cast, time_ms: int = 0) -> Image.Image:
        act, box = actors(kit, lay, cast)
        return el.render_lit(kit, lay, view, 2, act, time_ms=time_ms, boxes=box)

    tiles = []
    for i, (name, spiece, bpiece, dust) in enumerate([
            ("1 intact", "shore_post_intact", "loose_block_seated", False),
            ("2 hit 1: damaged_1, block settling, trickle", "shore_post_damaged_1", "loose_block_settling", True),
            ("3 hit 2: damaged_2, block slipped, trickle", "shore_post_damaged_2", "loose_block_slipped", True)]):
        extra = [("prop", "shore_dust_trickle", *trickle)] if dust else []
        tiles.append(label(frame(state_layout(spiece, bpiece, extra), cast5, TRICKLE_MS), name))
    falling = state_layout("shore_post_broken", None, [("prop", "loose_block_falling", centre_x - 36, 120 + 70)])
    tiles.append(label(frame(falling, cast5), "4 hit 3: broken, block falling"))
    landed = state_layout("shore_post_broken", None, [("prop", "loose_block_landed", centre_x - 48, walk - 30),
                                                      ("prop", "block_impact_dust", centre_x - 56, walk - 36)])
    cast_landed = [c for c in cast5 if c[1] < 3000 or c[1] > 3100]
    tiles.append(label(frame(landed, cast_landed, IMPACT_MS), "5 landed: rubble and impact dust"))
    burst_lay = state_layout("shore_post_damaged_1", "loose_block_settling",
                             [("prop", "timber_splinter_burst", *burst)])
    tiles.append(label(frame(burst_lay, cast5, BURST_MS), "(hit flash: splinter burst frame 2)"))
    strip = Image.new("RGB", (sum(t.width for t in tiles) + 8 * (len(tiles) - 1), tiles[0].height), PAGE)
    x = 0
    for t in tiles:
        strip.paste(t, (x, 0))
        x += t.width + 8
    save(strip, out / "env02_cp03_state_sequence_lit_2x.png", written)

    # --- review fix E3: the four shore states and the block states with NO actor in front, so the
    # notch, the split and the block tilt can be judged at gameplay scale (1x) and at 3x
    nv = [2936, 112, 240, 200]
    nolive = [
        ("1 intact / block seated", state_layout("shore_post_intact", "loose_block_seated"), 0),
        ("2 damaged_1 / settling", state_layout("shore_post_damaged_1", "loose_block_settling",
                                                [("prop", "shore_dust_trickle", *trickle)]), TRICKLE_MS),
        ("3 damaged_2 / slipped", state_layout("shore_post_damaged_2", "loose_block_slipped",
                                               [("prop", "shore_dust_trickle", *trickle)]), TRICKLE_MS),
        ("4 broken / falling", falling, 0),
        ("5 broken / landed", state_layout("shore_post_broken", None,
                                           [("prop", "loose_block_landed", centre_x - 48, walk - 30),
                                            ("prop", "block_impact_dust", centre_x - 56, walk - 36)]), IMPACT_MS),
    ]
    for factor in (1, 3):
        panels = []
        for name, lay, t_ms in nolive:
            act0, box0 = actors(kit, lay, [])
            panels.append(label(el.render_lit(kit, lay, nv, factor, act0, time_ms=t_ms, boxes=box0), name))
        if factor == 1:
            cols = len(panels)
        else:
            cols = 3
        rows_n = (len(panels) + cols - 1) // cols
        pw, ph = panels[0].size
        sheet = Image.new("RGB", (cols * pw + 8 * (cols - 1), rows_n * ph + 8 * (rows_n - 1)), PAGE)
        for i, pn in enumerate(panels):
            sheet.paste(pn, ((i % cols) * (pw + 8), (i // cols) * (ph + 8)))
        save(sheet, out / f"env02_cp03_states_no_actor_lit_{factor}x.png", written)

    gif: list[tuple[Image.Image, int]] = []
    gif.append((label(frame(state_layout("shore_post_intact", "loose_block_seated"), cast5), "intact"), 700))
    for spiece, bpiece, nm in (("shore_post_damaged_1", "loose_block_settling", "hit 1"),
                               ("shore_post_damaged_2", "loose_block_slipped", "hit 2")):
        for k in range(4):              # splinter burst at the hit (4 ticks per frame), trickle loops
            lay = state_layout(spiece, bpiece, [("prop", "timber_splinter_burst", *burst),
                                                ("prop", "shore_dust_trickle", *trickle)])
            gif.append((label(frame(lay, cast5, BURST_MS * k + 1), nm), BURST_MS))
        for k in range(6):
            lay = state_layout(spiece, bpiece, [("prop", "shore_dust_trickle", *trickle)])
            gif.append((label(frame(lay, cast5, TRICKLE_MS * k), nm), TRICKLE_MS))
    # hit 3 on a real timeline (50 ms steps): the shore breaks at t = 0, the block holds for 6 ticks
    # (100 ms), then falls 140 px under 1050 px/s^2 (about 0.52 s) and lands with impact dust
    t_land = int(100 + 1000 * (2 * 140 / 1050) ** 0.5)
    t = 0
    while t < t_land:
        extra = [("prop", "timber_splinter_burst", *burst)] if t < 4 * BURST_MS else []
        if t < 100:
            lay = state_layout("shore_post_broken", "loose_block_slipped", extra)
            nm = "hit 3: broken, block holds (6 ticks)"
        else:
            tf = (t - 100) / 1000
            y = 168 + 525 * tf * tf
            lay = state_layout("shore_post_broken", None,
                               extra + [("prop", "loose_block_falling", centre_x - 36, int(y) - 52)])
            nm = "falling"
        gif.append((label(frame(lay, cast5, t + 1), nm), 50))
        t += 50
    for k in range(4):
        lay = state_layout("shore_post_broken", None, [("prop", "loose_block_landed", centre_x - 48, walk - 30),
                                                       ("prop", "block_impact_dust", centre_x - 56, walk - 36)])
        gif.append((label(frame(lay, cast_landed, IMPACT_MS * k + 1), "landed"), IMPACT_MS))
    lay = state_layout("shore_post_broken", None, [("prop", "loose_block_landed", centre_x - 48, walk - 30)])
    gif.append((label(frame(lay, cast_landed), "landed"), 1500))
    save_gif(gif, out / "env02_cp03_sequence_lit_2x.gif", written)

    # --- design fix round: the rotten shore foot at 1x and 4x, and the barricade in the lane
    feet = []
    for st in ("intact", "damaged_1", "damaged_2", "broken"):
        lay = layout if st == "intact" else swap(layout, "shore_post_intact", f"shore_post_{st}")
        act, box = actors(kit, lay, [(CARL, shore["x"] - 26, False)])
        feet.append(el.render_lit(kit, lay, [2920, 230, 120, 90], 1, act, boxes=box))
    row = Image.new("RGB", (sum(f.width for f in feet) + 6 * (len(feet) - 1), feet[0].height), PAGE)
    x = 0
    for f in feet:
        row.paste(f, (x, 0))
        x += f.width + 6
    sheet = Image.new("RGB", (row.width * 4, row.height * 5 + 24), PAGE)
    sheet.paste(row, (0, 24))
    sheet.paste(row.resize((row.width * 4, row.height * 4), Image.Resampling.NEAREST), (0, 24 + row.height))
    text(ImageDraw.Draw(sheet), (6, 6), "Fix round: rotten shore foot (intact / d1 / d2 / broken), lit, 1x (top) "
         "and nearest 4x: warm wood, eaten outline, neck at the strike band, pale fibres, crumbling base")
    save(sheet, out / "env02_fix_shore_foot_1x_4x.png", written)
    bview = [2160, 20, 320, 330]
    act_b, box_b = actors(kit, layout, [(CARL, 2296, False), (DONUT, 2240, False)])
    smashed = swap(layout, "timber_barricade_full_intact", "timber_barricade_full_smashed")
    act_s, box_s = actors(kit, smashed, [(CARL, 2350, False)])
    pair = [label(el.render_lit(kit, layout, bview, 2, act_b, boxes=box_b),
                  "intact: crosses the lane to the deck lip (y 312); Carl stopped at the blocker"),
            label(el.render_lit(kit, smashed, bview, 2, act_s, boxes=box_s),
                  "smashed: boards thrown across the lane; Carl walks over them")]
    both = Image.new("RGB", (pair[0].width * 2 + 8, pair[0].height), PAGE)
    both.paste(pair[0], (0, 0))
    both.paste(pair[1], (pair[0].width + 8, 0))
    save(both, out / "env02_fix_barricade_lane_lit_2x.png", written)

    mask = el.actor_mask(act5, s5)
    report = {
        "format": "env02-evidence-v2",
        "note": "Pillow art preview with the ENV-01 lighting model on exported atlases; not a Bevy "
                "capture and not gameplay evidence.",
        "layouts": [LAYOUT_R, LAYOUT_T],
        "readability_s5_640x360": dict(zip(["carl_h52", "donut_h40", "goblin_a", "goblin_b", "goblin_a_2"],
                                           el.measure(lit5, mask, box5, s5))),
        "atlas_colours": {name: len(a["grid"]["palette"]) for name, a in kit.atlases.items()
                          if name.startswith(("floor01_opening", "floor01_guild", "floor01_soffit",
                                              "floor01_shore", "floor01_block", "floor01_splinter"))},
        "lit_colours_s5_640x360": el.colour_count(lit5),
        "files": [p.relative_to(out).as_posix() for p in written],
    }
    (out / "measurements.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    names = sorted([p.relative_to(out).as_posix() for p in written] + ["measurements.json"])
    (out / "SHA256SUMS").write_text("".join(
        f"{hashlib.sha256((out / n).read_bytes()).hexdigest()}  {n}\n" for n in names), encoding="utf-8")
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=ROOT / "generated/art/env02_evidence")
    args = parser.parse_args(argv)
    report = render(args.out)
    for f in report["files"]:
        print(f"wrote {f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
