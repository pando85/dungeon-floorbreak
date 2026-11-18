#!/usr/bin/env python3
"""Review evidence for the Donut melee and damage-reaction set (ANIM-DONUT-02), all 15 frames.

Usage (after ``bash scripts/export_assets.sh`` has exported the Floor 1 kit atlases):

    python3 scripts/make_donut_melee_previews.py [--out DIR]

Writes, deterministically, into DIR (default ``docs/evidence/anim-donut-02``):

* ``contact_sheet_x1.png`` and ``contact_sheet_x6.png``: every frame, keys outlined, tick windows, floor row and pivot;
* ``frame_layout_timing_diagram.png``: frame layout with pivot, floor row and the manifest hit boxes, and the tick
  contract of every move drawn from ``melee.moves``;
* ``*_x4.gif``: swat, swat > swat (restart at t10), pounce (with the proposed launch values), hurt, knockback >
  knockdown > get-up, played at the 60 Hz tick durations and followed by 24 ticks of the approved idle;
* ``floor01_melee_*``: lit Floor 1 compositions at gameplay scale (640 x 360 and exact 3x 1920 x 1080) with Carl and a
  GOB-01 goblin: the swat contact, the pounce and pin, and a lineup of the seven in-betweens;
* ``measurements.json`` and ``SHA256SUMS`` (every file in DIR except itself).

Presentation-only: composed from the grid source, the manifest and the approved Carl, GOB-01 and locomotion grids.
Nothing here is a Bevy capture and nothing here changes a source.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]

from scripts.pixelart.common import (BG, GREY, WHITE, YELLOW, cell, flip, gif_delays, grid_images,  # noqa: E402
                                     label, load_grid_file, opaque_bbox, save_gif, text_width, up)
from scripts.pixelart.config import load_config  # noqa: E402
from scripts.pixelart.evidence import contact_sheet, sprite_on, write_sums  # noqa: E402
from scripts.pixelart.scene import Floor  # noqa: E402

CONFIG = ROOT / "assets/source/characters/donut/donut_h40_melee.pixelart.json"
LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
CARL = ROOT / "assets/source/characters/carl/carl_h52.grid.json"
GOBLIN = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
DEFAULT_OUT = ROOT / "docs/evidence/anim-donut-02"
GRID_NAMES = {0: "s0", 1: "s1", 2: "s2", 3: "s3", 4: "p0", 5: "p1", 6: "p2", 7: "p3", 8: "h0", 9: "h1", 10: "kb",
              11: "d0", 12: "d1", 13: "g0", 14: "g1"}
IDLE_TAIL_TICKS = 24          # approved idle shown after each GIF move so the loop boundary reads
KB_PREVIEW_TICKS = 12         # the knockback hold is state-driven; the preview shows 12 ticks of it
GIF_SCALE = 4
GRAVITY = 1050.0
PHASE_COLOURS = {"startup": (226, 168, 60), "active": (232, 64, 64), "recovery": (92, 104, 140),
                 "hitstun": (150, 96, 200), "down": (120, 92, 170), "getup": (80, 170, 120),
                 "airborne": (60, 170, 170)}
RED, MAGENTA = (255, 72, 72), (255, 96, 210)


# ----------------------------------------------------------------------------- shared inputs

class Inputs:
    def __init__(self) -> None:
        self.cfg = load_config(CONFIG)
        self.grid = self.cfg.load_grid()
        self.imgs = grid_images(self.grid)
        self.manifest = json.loads((ROOT / "assets/manifests/donut_h40_melee.json").read_text(encoding="utf-8"))
        self.pivot = tuple(self.grid["pivot"])
        loco = load_grid_file(LOCO)
        self.idle = grid_images(loco)[0]
        carl = load_grid_file(CARL)
        self.carl, self.carl_pivot = grid_images(carl)[0], tuple(carl["pivot"])
        gob = load_grid_file(GOBLIN)
        self.goblin, self.goblin_pivot = flip(grid_images(gob)[0], tuple(gob["pivot"]))   # faces left, towards Donut
        self.floor = Floor()

    def goblin_front(self, rows_above_floor: tuple[int, int]) -> int:
        """Pivot-relative x of the goblin's leading edge over the given heights above the floor (inclusive)."""
        lo, hi = rows_above_floor
        pivot_row = self.goblin_pivot[1]
        xs = [x for y in range(pivot_row - hi, pivot_row - lo + 1) for x in range(self.goblin.width)
              if self.goblin.getpixel((x, y))[3]]
        return min(xs) - self.goblin_pivot[0]


def ghost(img: Image.Image, alpha: int) -> Image.Image:
    out = img.copy()
    out.putalpha(img.getchannel("A").point(lambda a: a * alpha // 255))
    return out


# ----------------------------------------------------------------------------- GIFs

def idle_tail(inp: Inputs, k: int) -> list[tuple[Image.Image, int]]:
    return [(sprite_on(inp.idle, k, inp.cfg.contact_row), IDLE_TAIL_TICKS)]


def tag_ticks(inp: Inputs, tag: str, hold_ticks: int = KB_PREVIEW_TICKS) -> list[tuple[int, int]]:
    return [(f, t if t is not None else hold_ticks) for f, t in inp.cfg.tags[tag]["frames"]]


def write_gif(path: Path, frames: list[tuple[Image.Image, int]]) -> Path:
    save_gif([im for im, _t in frames], gif_delays([t for _im, t in frames]), path)
    return path


def simple_gif(inp: Inputs, path: Path, sequence: list[tuple[int, int]]) -> Path:
    frames = [(sprite_on(inp.imgs[f], GIF_SCALE, inp.cfg.contact_row), t) for f, t in sequence]
    return write_gif(path, frames + idle_tail(inp, GIF_SCALE))


def pounce_gif(inp: Inputs, path: Path) -> Path:
    """The pounce with the proposed launch (vx 180 px/s, vy 130 px/s, GRAVITY 1050): one image per tick on a wider
    ground strip, goblin placeholder at the pin. Frame choice is the manifest window; motion is the proposal."""
    launch = inp.manifest["melee"]["moves"]["pounce"]["launch_proposal"]
    vx, vy = launch["vx_px_s"], launch["vy_px_s"]
    width, h = 150, inp.imgs[0].height
    x0 = 52                                           # pivot column at launch
    land_tick = 24                                    # first pin tick (manifest nominal window)
    travel = round(vx * (land_tick - 10) / 60)
    front = inp.goblin_front((0, 5))
    goblin_x = x0 + travel + 23 - front               # pin paws at the goblin's feet
    out: list[tuple[Image.Image, int]] = []
    for t in range(44):                               # one image per tick; frame windows from the manifest
        frame = 4 if t < 10 else 5 if t < 24 else 6 if t < 32 else 7
        tau = max(t - 10, 0) / 60
        lift = max(round(vy * tau - 0.5 * GRAVITY * tau * tau), 0) if 10 <= t < 24 else 0
        x = x0 + (round(vx * tau) if 10 <= t < 24 else travel if t >= 24 else 0)
        canvas = Image.new("RGBA", (width, h + 4), BG + (255,))
        d = ImageDraw.Draw(canvas)
        d.rectangle([0, h, width - 1, h], fill=(74, 68, 100))
        d.rectangle([0, h + 1, width - 1, h + 3], fill=(52, 48, 74))
        canvas.alpha_composite(inp.goblin, (goblin_x - inp.goblin_pivot[0], h - 1 - inp.goblin_pivot[1] + 0))
        canvas.alpha_composite(inp.imgs[frame], (x - inp.pivot[0], -lift))
        out.append((up(canvas, GIF_SCALE), 1))
    tail = Image.new("RGBA", (width, h + 4), BG + (255,))
    d = ImageDraw.Draw(tail)
    d.rectangle([0, h, width - 1, h], fill=(74, 68, 100))
    d.rectangle([0, h + 1, width - 1, h + 3], fill=(52, 48, 74))
    tail.alpha_composite(inp.goblin, (goblin_x - inp.goblin_pivot[0], h - 1 - inp.goblin_pivot[1]))
    tail.alpha_composite(inp.idle, (x0 + travel - inp.pivot[0], 0))
    out.append((up(tail, GIF_SCALE), IDLE_TAIL_TICKS))
    return write_gif(path, out)


def make_gifs(inp: Inputs, out: Path) -> list[Path]:
    swat = tag_ticks(inp, "swat")
    first = [(0, 4), (1, 3), (2, 3)]                  # the buffered press restarts the swat at t10 (spec section 4)
    return [
        simple_gif(inp, out / "swat_x4.gif", swat),
        simple_gif(inp, out / "swat_swat_x4.gif", first + swat),
        pounce_gif(inp, out / "pounce_x4.gif"),
        simple_gif(inp, out / "hurt_x4.gif", tag_ticks(inp, "hurt")),
        simple_gif(inp, out / "knockdown_getup_x4.gif",
                   tag_ticks(inp, "knockback") + tag_ticks(inp, "knockdown") + tag_ticks(inp, "getup")),
    ]


# ----------------------------------------------------------------------------- Floor 1 compositions

CARL_X, DONUT_X = 590, 650


def scene_files(inp: Inputs, out: Path, name: str, entries: list[tuple]) -> list[Path]:
    paths = []
    for suffix, k in (("640x360", 1), ("1920x1080", 3)):
        path = out / f"floor01_melee_{name}_{suffix}.png"
        inp.floor.render(entries, k).save(path)
        paths.append(path)
    return paths


def goblin_entry(inp: Inputs, x: int) -> tuple:
    return inp.goblin, inp.goblin_pivot, x, 0


def swat_scene(inp: Inputs, out: Path) -> list[Path]:
    """s1 at the goblin's shins (art reach +23 px), Carl idle beside her."""
    box = inp.manifest["melee"]["hit_boxes_px"]["swat_contact"]["paw"]
    goblin_x = DONUT_X + box[2] - inp.goblin_front((box[1], box[3]))
    entries = [goblin_entry(inp, goblin_x), (inp.carl, inp.carl_pivot, CARL_X, 0),
               (ghost(inp.imgs[0], 110), inp.pivot, DONUT_X - 8, 0, False),   # the cock, faint, behind the contact
               (inp.imgs[1], inp.pivot, DONUT_X, 0)]
    return scene_files(inp, out, "swat", entries)


def pounce_scene(inp: Inputs, out: Path) -> list[Path]:
    """The pounce: crouch and flight as faint ghosts, then the pin at the goblin's feet, Carl idle beside her."""
    box = inp.manifest["melee"]["hit_boxes_px"]["pounce_pin"]["far_paw"]
    goblin_x = DONUT_X + box[2] - inp.goblin_front((0, 5))
    entries = [goblin_entry(inp, goblin_x), (inp.carl, inp.carl_pivot, CARL_X - 70, 0),
               (ghost(inp.imgs[4], 90), inp.pivot, DONUT_X - 45, 0, False),
               (ghost(inp.imgs[5], 140), inp.pivot, DONUT_X - 22, 8, False),
               (inp.imgs[6], inp.pivot, DONUT_X, 0)]
    return scene_files(inp, out, "pounce_pin", entries)


def lineup_scene(inp: Inputs, out: Path) -> list[Path]:
    """The seven executed in-betweens at gameplay scale, 60 px apart, with Carl and a goblin for scale."""
    frames = [2, 3, 7, 9, 12, 13, 14]
    entries = [(inp.carl, inp.carl_pivot, 396, 0)]
    entries += [(inp.imgs[f], inp.pivot, 470 + 60 * i, 0) for i, f in enumerate(frames)]
    entries.append(goblin_entry(inp, 930))
    return scene_files(inp, out, "inbetweens_lineup", entries)


# ----------------------------------------------------------------------------- diagram

def box_rect(pivot, box, k):
    x0, y0, x1, y1 = box                              # forward x, up y, inclusive, relative to the pivot
    return [(pivot[0] + x0) * k, (pivot[1] - y1) * k, (pivot[0] + x1 + 1) * k - 1, (pivot[1] - y0 + 1) * k - 1]


def diagram(inp: Inputs) -> Image.Image:
    melee = inp.manifest["melee"]
    boxes = {  # frame -> list of (box, colour, name) from melee.hit_boxes_px
        1: [(melee["hit_boxes_px"]["swat_contact"]["paw"], RED, "swat paw")],
        5: [(melee["hit_boxes_px"]["pounce_flight"]["near_paw"], RED, "near paw"),
            (melee["hit_boxes_px"]["pounce_flight"]["far_paw"], MAGENTA, "far paw")],
        6: [(melee["hit_boxes_px"]["pounce_pin"]["near_paw"], RED, "near paw"),
            (melee["hit_boxes_px"]["pounce_pin"]["far_paw"], MAGENTA, "far paw")],
    }
    k, cols, gap, cap = 3, 8, 8, 34
    w, h = inp.imgs[0].size
    pad, head = 10, 24
    cell_w, cell_h = w * k + gap, h * k + cap + gap
    layout_h = head + 2 * cell_h
    # timing bars
    tick_px, row_h = 28, 44
    moves = list(melee["moves"].items())
    timing_h = head + len(moves) * row_h + 64
    total_w = max(cols * cell_w + 2 * pad, 150 + 44 * tick_px + 2 * pad)
    sheet = Image.new("RGB", (total_w, layout_h + timing_h + 2 * pad), BG)
    d = ImageDraw.Draw(sheet)
    keys = set(inp.cfg.rules["frozen_keys"]["frames"])
    label(d, (pad, 6), "ANIM-DONUT-02 frame layout: 72 x 44 px, pivot (46, 43), cyan = floor row 43, yellow = pivot, "
                       "red / magenta = manifest hit boxes (keys only), yellow caption = frozen key", WHITE, 11)
    for i, img in enumerate(inp.imgs):
        r, c = divmod(i, cols)
        x, y = pad + c * cell_w, head + r * cell_h
        tile = cell(img, k, inp.pivot, inp.cfg.contact_row).convert("RGB")
        td = ImageDraw.Draw(tile)
        for box, colour, _n in boxes.get(i, []):
            td.rectangle(box_rect(inp.pivot, box, k), outline=colour)
        sheet.paste(tile, (x, y))
        ticks = inp.cfg.ticks_of(i)
        label(d, (x + 1, y + h * k + 2), f"{i:02d} {GRID_NAMES[i]} {inp.cfg.tag_of(i)}", YELLOW if i in keys else WHITE, 11)
        label(d, (x + 1, y + h * k + 17), "hold" if ticks is None else f"{ticks} ticks", GREY, 11)
    y0 = layout_h + pad
    label(d, (pad, y0), "Tick contract at 60 Hz (melee.moves; tick 0 = the press is accepted). Bar colour = phase, "
                        "red tick = first active (hit) tick, hatched = state-driven hold", WHITE, 11)
    y = y0 + head
    for name, move in moves:
        label(d, (pad, y + 14), name, YELLOW, 11)
        x = pad + 150
        for fr in move["frames"]:
            ticks = fr["ticks"]
            n = (ticks[1] - ticks[0] + 1) if ticks else KB_PREVIEW_TICKS
            phase = fr["phase"].split()[0].rstrip(",")
            colour = PHASE_COLOURS.get(phase, (110, 110, 130))
            rect = [x, y + 4, x + n * tick_px - 2, y + 34]
            d.rectangle(rect, fill=colour)
            if not ticks:
                for hx in range(rect[0], rect[2], 8):
                    d.line([(hx, rect[3]), (min(hx + 12, rect[2]), rect[1])], fill=BG)
            cap_text = f"{fr['name'].split()[0]} {fr['phase'].split(' (')[0]}"
            if text_width(cap_text, 11) < n * tick_px - 6:
                label(d, (x + 4, y + 12), cap_text, (10, 10, 20), 11)
            if move.get("hit_tick") is not None and ticks and ticks[0] <= move["hit_tick"] <= ticks[1]:
                hx = x + (move["hit_tick"] - ticks[0]) * tick_px
                d.rectangle([hx, y, hx + 3, y + 38], fill=(255, 255, 255))
                label(d, (hx + 6, y + 36), f"hit t{move['hit_tick']}", WHITE, 11)
            if move.get("hit_tick_earliest") is not None and ticks and ticks[0] <= move["hit_tick_earliest"] <= ticks[1]:
                hx = x + (move["hit_tick_earliest"] - ticks[0]) * tick_px
                d.rectangle([hx, y, hx + 3, y + 38], fill=(255, 255, 255))
                label(d, (hx + 6, y + 36), f"earliest hit t{move['hit_tick_earliest']}", WHITE, 11)
            x += n * tick_px
        y += row_h
    # ruler
    d.line([(pad + 150, y + 4), (pad + 150 + 44 * tick_px, y + 4)], fill=GREY)
    for t in range(0, 45, 5):
        d.line([(pad + 150 + t * tick_px, y + 1), (pad + 150 + t * tick_px, y + 8)], fill=GREY)
        label(d, (pad + 150 + t * tick_px - 4, y + 10), f"t{t}", GREY, 11)
    return sheet


# ----------------------------------------------------------------------------- measurements

def measurements(inp: Inputs, written: list[Path]) -> dict:
    frames = []
    for i, img in enumerate(inp.imgs):
        bbox = opaque_bbox(img)
        frames.append({"index": i, "name": GRID_NAMES[i], "tag": inp.cfg.tag_of(i), "key": i in
                       set(inp.cfg.rules["frozen_keys"]["frames"]), "ticks": inp.cfg.ticks_of(i),
                       "duration_ms": inp.grid["frames"][i]["duration_ms"],
                       "bbox": list(bbox), "opaque_size": [bbox[2] - bbox[0] + 1, bbox[3] - bbox[1] + 1],
                       "lowest_row": bbox[3]})
    return {"asset": inp.cfg.asset_id, "frame_size": list(inp.cfg["frame_size"]), "pivot": list(inp.pivot),
            "frames": frames,
            "scene": {"carl_x": CARL_X, "donut_x": DONUT_X, "view": "abc_640x360 (level x 328..968)"}}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args(argv)
    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)
    inp = Inputs()
    written: list[Path] = []
    for k in (1, 6):
        path = out / f"contact_sheet_x{k}.png"
        contact_sheet(inp.cfg, inp.imgs, k).save(path)
        written.append(path)
    path = out / "frame_layout_timing_diagram.png"
    diagram(inp).save(path)
    written.append(path)
    written += make_gifs(inp, out)
    written += swat_scene(inp, out) + pounce_scene(inp, out) + lineup_scene(inp, out)
    path = out / "measurements.json"
    path.write_text(json.dumps(measurements(inp, written), indent=2) + "\n", encoding="utf-8")
    written.append(path)
    # the five key-scale files are written by scripts/author_donut_melee.py --evidence; sum everything present
    files = sorted(p for p in out.iterdir() if p.is_file() and p.name != "SHA256SUMS")
    write_sums(out, files)
    for p in written:
        print(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
