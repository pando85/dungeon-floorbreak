#!/usr/bin/env python3
"""Review renders for the ANIM-DONUT-01 locomotion candidate (Pillow only, deterministic).

Reads the exported candidate atlas (``assets/manifests/donut_h40_locomotion.json``),
the approved Carl 52 px idle atlas (``assets/manifests/carl_book1_h52.json``) and the
Floor 1 sample layout, and writes art-review aids:

* ``contact_sheet_1x.png`` / ``contact_sheet_8x_grid.png``: every frame by tag, native
  and enlarged with a pixel grid, the pivot pixel and the walking line;
* ``run_4x.gif``, ``jump_land_4x.gif`` and ``run_flip_2x.gif``: animated previews on
  a dark matte with the walking line (the jump sequence holds rise/fall for a beat);
* ``floor01_trail_run_640x360.png`` / ``_1920x1080.png``: Carl (idle frame 0) with
  Donut running 32 px behind him, as in the COMP-01 trail, lit with the A1 model of
  ``scripts/env_kit_light.py``; ``floor01_lineup_*``: every pose class on the floor
  next to Carl; ``floor01_trail_run_x3.gif``: the run cycle in that lit scene (3x crop);
* ``measurements.json``: per-frame bbox, opaque height, contact row and colours.

These are review aids for a human decision, not Bevy captures and not approvals.

Usage::

    bash scripts/export_assets.sh   # writes the atlases this script reads
    python3 scripts/make_donut_locomotion_review.py --out docs/evidence/anim-donut-01
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import env_kit as ek  # noqa: E402
from env_kit_light import render_lit  # noqa: E402
from validate_assets import safe_path  # noqa: E402

DONUT_MANIFEST = "assets/manifests/donut_h40_locomotion.json"
CARL_MANIFEST = "assets/manifests/carl_book1_h52.json"
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
TRAIL_PX = 32          # COMP-01: Donut trails Carl by 32 px of path length
MATTE = (25, 26, 44, 255)
GROUND = (110, 84, 70, 255)
LIFT = {"jump_rise": 26, "jump_fall": 14}   # presentation-only height above the floor


class Sheet:
    def __init__(self, manifest_path: str):
        self.manifest = json.loads(safe_path(manifest_path).read_text(encoding="utf-8"))
        self.meta = json.loads(safe_path(self.manifest["output_metadata"]).read_text(encoding="utf-8"))
        with Image.open(safe_path(self.manifest["output_atlas"])) as opened:
            self.atlas = opened.convert("RGBA")
        self.pivot = tuple(self.manifest["pivot"])
        self.tags = {t["name"]: (t["from"], t["to"]) for t in self.meta["meta"]["frameTags"]}

    def frame(self, index: int) -> Image.Image:
        r = self.meta["frames"][index]["frame"]
        return self.atlas.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))

    def duration(self, index: int) -> int:
        return self.meta["frames"][index]["duration"]

    def frames(self, tag: str) -> list[int]:
        a, b = self.tags[tag]
        return list(range(a, b + 1))


def tag_of(sheet: Sheet, index: int) -> str:
    for name, (a, b) in sheet.tags.items():
        if a <= index <= b:
            return name
    return "?"


# ---------------------------------------------------------------------------
# Matte previews
# ---------------------------------------------------------------------------

def matte_frame(sprite: Image.Image, pivot, scale: int, flip: bool = False, lift: int = 0,
                pad=(8, 30, 8, 3)) -> Image.Image:
    """Sprite on the matte with the walking line under the pivot row (row pivot+1)."""
    w, h = sprite.size
    left, top, right, bottom = pad
    img = Image.new("RGBA", (w + left + right, h + top + bottom), MATTE)
    d = ImageDraw.Draw(img)
    ground_y = top + pivot[1] + 1
    d.rectangle((0, ground_y, img.width, img.height), fill=(40, 36, 52, 255))
    d.line((0, ground_y, img.width, ground_y), fill=GROUND)
    spr = sprite.transpose(Image.Transpose.FLIP_LEFT_RIGHT) if flip else sprite
    # The pivot pixel lands on matte column left + pivot x in both facings (the
    # mirrored sprite's pivot column is w - 1 - pivot x).
    x = left + pivot[0] - (w - 1 - pivot[0] if flip else pivot[0])
    img.alpha_composite(spr, (x, top - lift))
    return img.resize((img.width * scale, img.height * scale), Image.Resampling.NEAREST)


def save_gif(frames: list[Image.Image], durations: list[int], path: Path) -> None:
    frames = [f.convert("RGB") for f in frames]
    frames[0].save(path, format="GIF", save_all=True, append_images=frames[1:],
                   duration=durations, loop=0, optimize=False, disposal=2)


def contact_sheets(sheet: Sheet, out: Path) -> list[Path]:
    rows = [["idle"], ["run"], ["jump_start", "jump_rise", "jump_fall", "land"]]
    w, h = sheet.frame(0).size
    px, py = sheet.pivot
    written = []
    for scale, grid, name in ((1, False, "contact_sheet_1x.png"), (8, True, "contact_sheet_8x_grid.png")):
        gap = 2 if scale == 1 else 3
        cols = max(sum(len(sheet.frames(t)) for t in r) for r in rows)
        cw, ch = w * scale + gap, (h + 2) * scale + gap
        img = Image.new("RGBA", (cols * cw + gap, len(rows) * ch + gap), (10, 10, 14, 255))
        for ri, tags in enumerate(rows):
            ci = 0
            for tag in tags:
                for index in sheet.frames(tag):
                    cell = Image.new("RGBA", (w, h + 2), MATTE)
                    d = ImageDraw.Draw(cell)
                    d.line((0, py + 1, w, py + 1), fill=GROUND)
                    cell.alpha_composite(sheet.frame(index), (0, 0))
                    cell = cell.resize((w * scale, (h + 2) * scale), Image.Resampling.NEAREST)
                    if grid:
                        d = ImageDraw.Draw(cell)
                        for x in range(0, cell.width, scale):
                            d.line((x, 0, x, cell.height), fill=(0, 0, 0, 255) if x % (8 * scale) else (70, 70, 96, 255))
                        for y in range(0, cell.height, scale):
                            d.line((0, y, cell.width, y), fill=(0, 0, 0, 255) if y % (8 * scale) else (70, 70, 96, 255))
                        d.rectangle((px * scale, py * scale, px * scale + scale - 1, py * scale + scale - 1),
                                    outline=(60, 255, 120, 255), width=2)
                        d.text((6, 4), f"{index} {tag} {sheet.duration(index)}ms", fill=(235, 235, 245, 255))
                    img.alpha_composite(cell, (gap + ci * cw, gap + ri * ch))
                    ci += 1
        path = out / name
        img.convert("RGB").save(path, optimize=False)
        written.append(path)
    return written


def gifs(sheet: Sheet, out: Path) -> list[Path]:
    written = []
    run = sheet.frames("run")
    path = out / "run_4x.gif"
    save_gif([matte_frame(sheet.frame(i), sheet.pivot, 4) for i in run * 2],
             [sheet.duration(i) for i in run * 2], path)
    written.append(path)
    # Jump sequence: idle beat, start, rise held, fall held, land, idle beat.
    seq = [(0, 400, 0)]
    seq += [(i, sheet.duration(i), 0) for i in sheet.frames("jump_start")]
    seq += [(sheet.frames("jump_rise")[0], 360, LIFT["jump_rise"])]
    seq += [(sheet.frames("jump_fall")[0], 300, LIFT["jump_fall"])]
    seq += [(i, sheet.duration(i), 0) for i in sheet.frames("land")]
    seq += [(0, 500, 0)]
    path = out / "jump_land_4x.gif"
    save_gif([matte_frame(sheet.frame(i), sheet.pivot, 4, lift=lift) for i, _, lift in seq],
             [d for _, d, _ in seq], path)
    written.append(path)
    # Facing check: right and mirrored-left side by side, pivots on one column.
    frames = []
    for i in run * 2:
        a = matte_frame(sheet.frame(i), sheet.pivot, 2, pad=(26, 30, 26, 3))
        b = matte_frame(sheet.frame(i), sheet.pivot, 2, flip=True, pad=(26, 30, 26, 3))
        both = Image.new("RGBA", (a.width + b.width + 4, a.height), (10, 10, 14, 255))
        both.alpha_composite(a, (0, 0))
        both.alpha_composite(b, (a.width + 4, 0))
        frames.append(both)
    path = out / "run_flip_2x.gif"
    save_gif(frames, [sheet.duration(i) for i in run * 2], path)
    written.append(path)
    return written


# ---------------------------------------------------------------------------
# Floor 1 compositions (A1 lighting model)
# ---------------------------------------------------------------------------

def place(layer: Image.Image, sprite: Image.Image, pivot, foot_x: int, walk_top: int,
          lift: int = 0, flip: bool = False) -> tuple[int, int, int, int]:
    """Composite with the pivot on the walking line (0 px sink); return the box."""
    spr = sprite.transpose(Image.Transpose.FLIP_LEFT_RIGHT) if flip else sprite
    px = sprite.width - 1 - pivot[0] if flip else pivot[0]
    pos = (foot_x - px, walk_top - 1 - pivot[1] - lift)
    layer.alpha_composite(spr, pos)
    bbox = spr.getbbox()
    return (pos[0] + bbox[0], pos[1] + bbox[1], pos[0] + bbox[2], pos[1] + bbox[3])


def floor_scenes(donut: Sheet, carl: Sheet, out: Path) -> tuple[list[Path], dict]:
    layout = ek.load_json(LAYOUT)
    kit = ek.load_kit(layout["kit_contract"])
    ek.validate_contract(kit)
    ek.validate_layout(kit, layout)
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    view = layout["views"][layout["preview"]["primary_view"]]
    size = tuple(layout["pixel_size"])

    def top_at(x: int) -> int:
        t = tops[x // ek.TILE]
        if not isinstance(t, int):
            raise ValueError(f"no walking line at x {x}")
        return t

    written: list[Path] = []
    info: dict = {"layout": LAYOUT, "view": view, "trail_px": TRAIL_PX}

    # Trail: Carl idle frame 0, Donut 32 px behind (drawn behind him, as in COMP-01).
    carl_x = view[0] + 200
    donut_x = carl_x - TRAIL_PX

    def trail_layer(run_index: int):
        layer = Image.new("RGBA", size, (0, 0, 0, 0))
        boxes = [place(layer, donut.frame(run_index), donut.pivot, donut_x, top_at(donut_x))]
        boxes.append(place(layer, carl.frame(0), carl.pivot, carl_x, top_at(carl_x)))
        return layer, boxes

    layer, boxes = trail_layer(donut.frames("run")[0])
    for k, name in ((1, "floor01_trail_run_640x360.png"), (3, "floor01_trail_run_1920x1080.png")):
        path = out / name
        render_lit(kit, layout, view, k, layer, boxes=boxes).convert("RGB").save(path, optimize=False)
        written.append(path)
    info["trail"] = {"carl_foot_x": carl_x, "donut_foot_x": donut_x, "walking_line_y": top_at(carl_x),
                     "donut_frame": donut.frames("run")[0], "carl_frame": 0}

    # Run cycle in the lit scene, 3x crop around both actors.
    crop = (carl_x - 110 - view[0], 150, carl_x + 50 - view[0], 290)
    frames, durations = [], []
    for i in donut.frames("run") * 2:
        layer, boxes = trail_layer(i)
        lit = render_lit(kit, layout, view, 3, layer, boxes=boxes)
        frames.append(lit.crop(tuple(v * 3 for v in crop)))
        durations.append(donut.duration(i))
    path = out / "floor01_trail_run_x3.gif"
    save_gif(frames, durations, path)
    written.append(path)
    info["trail_gif_crop_view_px"] = list(crop)

    # Lineup: Carl, then one frame per pose class, left to right.
    poses = [0, donut.frames("run")[0], donut.frames("run")[2], donut.frames("run")[4],
             donut.frames("jump_start")[0], donut.frames("jump_rise")[0], donut.frames("jump_fall")[0],
             donut.frames("land")[0]]
    lineup = [("carl idle 0", None)] + [(f"{tag_of(donut, i)} {i}", i) for i in poses]
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    boxes, placed = [], []
    x = view[0] + 30
    for label, index in lineup:
        lift = 0
        if index is None:
            box = place(layer, carl.frame(0), carl.pivot, x, top_at(x))
            x += 60
        else:
            lift = LIFT.get(tag_of(donut, index), 0)
            box = place(layer, donut.frame(index), donut.pivot, x, top_at(x), lift=lift)
            x += 68
        if not lift:  # contact shadows only under grounded actors
            boxes.append(box)
        placed.append({"label": label, "box": list(box)})
    for k, name in ((1, "floor01_lineup_640x360.png"), (3, "floor01_lineup_1920x1080.png")):
        path = out / name
        render_lit(kit, layout, view, k, layer, boxes=boxes).convert("RGB").save(path, optimize=False)
        written.append(path)
    info["lineup"] = placed
    return written, info


def measurements(sheet: Sheet) -> list[dict]:
    rows = []
    for i, _ in enumerate(sheet.meta["frames"]):
        img = sheet.frame(i)
        x0, y0, x1, y1 = img.getbbox()
        colours = {c for _, c in img.getcolors(4096) if c[3] == 255}
        rows.append({"frame": i, "tag": tag_of(sheet, i), "duration_ms": sheet.duration(i),
                     "bbox": [x0, y0, x1 - 1, y1 - 1], "opaque_height": y1 - y0, "width": x1 - x0,
                     "lowest_row": y1 - 1, "rows_above_pivot": sheet.pivot[1] - (y1 - 1),
                     "colours": len(colours)})
    return rows


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=ROOT / "generated/art/anim_donut_01")
    args = parser.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    donut, carl = Sheet(DONUT_MANIFEST), Sheet(CARL_MANIFEST)
    written = contact_sheets(donut, args.out) + gifs(donut, args.out)
    scenes, info = floor_scenes(donut, carl, args.out)
    written += scenes
    report = {"format": "anim-donut-01-review-v1", "manifest": DONUT_MANIFEST, "carl_manifest": CARL_MANIFEST,
              "frame_size": donut.manifest["frame_size"], "pivot": donut.manifest["pivot"],
              "tags": {k: list(v) for k, v in donut.tags.items()}, "frames": measurements(donut),
              "floor01": info,
              "note": "Review aids; the lit scenes use the Pillow A1 model, not a Bevy capture."}
    path = args.out / "measurements.json"
    path.write_text(json.dumps(report, indent=1) + "\n", encoding="utf-8")
    written.append(path)
    print("\n".join(p.resolve().relative_to(ROOT).as_posix() if p.resolve().is_relative_to(ROOT)
                    else str(p) for p in written))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
