#!/usr/bin/env python3
"""Render the ENV-02 design evidence (key sheet, route plan, ceiling options, lit frames).

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
        ("pursuer-smashed timber (presentation + authored event): set and barricade, intact / smashed",
         ["timber_set_intact", "timber_set_smashed", "timber_barricade_intact", "timber_barricade_smashed"]),
        ("dodge pocket and maintenance hint: alcove niche (backwall decal), iron hatch",
         ["alcove_niche", "maint_hatch"]),
        ("guild exit: portal closed / open, hanging sign (lantern pictogram, no text)",
         ["guild_portal_closed", "guild_portal_open", "guild_sign"]),
    ]
    pad, gap, label_h = 8, 10, 14
    under = (52, 46, 58, 255)
    rows = []
    for title, ids in groups:
        imgs = [(pid, ek.scale(kit.crop(kit.pieces[pid]["atlas"], kit.pieces[pid]["rect"]), factor))
                for pid in ids]
        rows.append((title, imgs))
    width = max(sum(i.width + gap for _p, i in imgs) + pad * 2 for _t, imgs in rows)
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
            x += img.width + gap
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

    shore = marker(layout, "breakable_shore")
    strike_x = shore["x"]
    # --- S5 skirmish: Carl at the strike position, Donut behind, three goblins (two in the fall region)
    s5 = layout["views"]["s5_skirmish_640x360"]
    cast5 = [(CARL, strike_x - 19, False), (DONUT, strike_x - 64, False), (GOB_A, 3024, True),
             (GOB_B, 3060, True), (GOB_A, 3150, True)]
    act5, box5 = actors(kit, layout, cast5)
    lit5 = el.render_lit(kit, layout, s5, 1, act5, boxes=box5)
    save(lit5, out / "env02_lit_s5_skirmish_640x360.png", written)
    save(el.render_lit(kit, layout, s5, 3, act5, boxes=box5), out / "env02_lit_s5_skirmish_1920x1080.png",
         written)
    # --- S4 widened: Carl and Donut pressed into the alcove, waiting for the charge
    s4 = layout["views"]["s4_widened_640x360"]
    pocket = marker(layout, "safe_pocket_alcove")
    act4, box4 = actors(kit, layout, [(CARL, pocket["x"] - 18, False), (DONUT, pocket["x"] + 22, True)])
    save(el.render_lit(kit, layout, s4, 1, act4, boxes=box4), out / "env02_lit_s4_widened_640x360.png", written)
    save(el.render_lit(kit, layout, s4, 3, act4, boxes=box4), out / "env02_lit_s4_widened_1920x1080.png", written)
    # --- S3 chase passage (option R) and option T, Carl and Donut fleeing right
    s3 = layout["views"]["s3_chase_640x360"]
    for opt, lay in (("R", layout), ("T", layout_t)):
        act3, box3 = actors(kit, lay, [(CARL, 1328, False), (DONUT, 1290, False)])
        save(el.render_lit(kit, lay, s3, 1, act3, boxes=box3), out / f"env02_lit_s3_chase_option_{opt}_640x360.png",
             written)
    # --- S6 guild: portal open, Carl walking in, Donut following
    s6 = layout["views"]["s6_guild_640x360"]
    open6 = swap(layout, "guild_portal_closed", "guild_portal_open")
    exit_m = marker(layout, "exit_guild")
    act6, box6 = actors(kit, open6, [(CARL, exit_m["x"] - 30, False), (DONUT, exit_m["x"] - 90, False)])
    save(el.render_lit(kit, layout, s6, 1), out / "env02_lit_s6_guild_closed_640x360.png", written)
    save(el.render_lit(kit, open6, s6, 1, act6, boxes=box6), out / "env02_lit_s6_guild_open_640x360.png", written)
    save(el.render_lit(kit, open6, s6, 3, act6, boxes=box6), out / "env02_lit_s6_guild_open_1920x1080.png", written)

    # --- CP-03 state sequence in scene (lit, nearest 2x crops)
    crop_box = (2896, 100, 3216, 330)
    view = [crop_box[0], crop_box[1], crop_box[2] - crop_box[0], crop_box[3] - crop_box[1]]
    block = marker(layout, "falling_block")
    bx, by = block["rect"][0], block["rect"][1]
    states = [
        ("1 intact", layout, cast5),
        ("2 hit 1: damaged_1 + block settling", swap(swap(layout, "shore_post_intact", "shore_post_damaged_1"),
                                                   "loose_block_seated", "loose_block_settling"), cast5),
        ("3 hit 2: damaged_2 + block slipped", swap(swap(layout, "shore_post_intact", "shore_post_damaged_2"),
                                                   "loose_block_seated", "loose_block_slipped"), cast5),
        ("4 hit 3: broken, block falling", swap(swap(layout, "shore_post_intact", "shore_post_broken"),
                                                "loose_block_seated", "loose_block_falling", -4, 70), cast5),
        ("5 landed: rubble in the fall region", add(swap(swap(layout, "shore_post_intact", "shore_post_broken"),
                                                         "loose_block_seated", None),
                                                    "prop", "loose_block_landed", bx - 16, 280),
         [c for c in cast5 if c[1] < 3000 or c[1] > 3100]),
    ]
    tiles = []
    for name, lay, cast in states:
        act, box = actors(kit, lay, cast)
        img = el.render_lit(kit, lay, view, 2, act, boxes=box)
        tiles.append(label(img, name))
    strip = Image.new("RGB", (sum(t.width for t in tiles) + 8 * (len(tiles) - 1), tiles[0].height), PAGE)
    x = 0
    for t in tiles:
        strip.paste(t, (x, 0))
        x += t.width + 8
    save(strip, out / "env02_cp03_state_sequence_lit_2x.png", written)

    mask = el.actor_mask(act5, s5)
    report = {
        "format": "env02-design-evidence-v1",
        "note": "Pillow art preview with the ENV-01 lighting model on exported atlases; not a Bevy "
                "capture and not gameplay evidence.",
        "layouts": [LAYOUT_R, LAYOUT_T],
        "readability_s5_640x360": dict(zip(["carl_h52", "donut_h40", "goblin_a", "goblin_b", "goblin_a_2"],
                                           el.measure(lit5, mask, box5, s5))),
        "atlas_colours": {name: len(a["grid"]["palette"]) for name, a in kit.atlases.items()
                          if name.startswith(("floor01_opening", "floor01_guild"))},
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
