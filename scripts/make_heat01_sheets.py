#!/usr/bin/env python3
"""AREA-HEAT-01 offline evidence sheets (deterministic, from the committed grids and the real stills).

    python3 scripts/make_heat01_sheets.py OUT_DIR [LAYOUT_DIR]

Reads ``OUT_DIR/scene_*_640x360.png`` (the runtime captures of ``make_heat01_evidence.sh``) and writes into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every kit piece at native size on a neutral ground (the 4x
  sheet labelled);
- ``partition_states_x4.png``: the three partition frames at 4x (key pixels, unlit);
- ``partition_states_with_actors_1x.png``: **gate composite (CORRAL-D pending), not runtime**: the three states
  at 1x in the real pen still beside Carl, Donut, a staged goblin and llama B (the partition is not placed in the
  room before CORRAL-D, ``HEAT-AD1``), from ``gate/scene_pen_*_1x.png``;
- ``layout_diagram_pen_x2.png`` / ``layout_diagram_dock_x2.png``: the room schema over a real still (the walking
  line W, the floor back line FB, the glob bands, the head bands, every dressing rect, the partition's clear zone
  x 1372..1428 and the step rects). The drawn view left of each still is read from landmarks
  (``scripts/heat01_view.py``, needs LAYOUT_DIR) and written to ``drawn_views.txt``;
- ``before_after_pen_entry_x1.png`` / ``_x3.png`` and ``before_after_dock_x1.png`` / ``_x3.png`` (when the
  ``scene_before_*`` stills exist): the undressed room (origin/main) beside the dressed room;
- ``scorch_wall_runtime_before_after_x1.png`` / ``_x3.png``: the runtime pen entry cropped to x 10..149, y 150..289,
  before beside after, and ``crop_n1_plume_right_x12.png``: the plume's top-right bricks beside the right torch
  pool's offset bricks (gate note N1).
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_heat01 as heat  # noqa: E402

BG = (58, 54, 66, 255)
W_LINE, FB = 580, 560
VIEW_TOP = 289          # the drawn view top of the 640 x 360 view (spec section 2)
STEPS = ((1120, 548, 96, 32), (1856, 500, 32, 80), (1888, 548, 416, 32))


def piece_images() -> dict[str, Image.Image]:
    out = {}
    for name in heat.ATLASES:
        for pid, img in heat.build_pieces(name):
            out[pid] = heat.to_image(img)
    return out


def contact(pieces, scale: int, labels: bool, width: int = 480) -> Image.Image:
    pad, label_h = 6, (12 if labels else 0)
    x = y = row_h = 0
    spots = []
    for pid, img in pieces:
        w, h = img.width, img.height
        if x + w + pad > width and x > 0:
            x, y, row_h = 0, y + row_h + pad + label_h, 0
        spots.append((pid, img, x + pad, y + pad + label_h))
        x += w + pad
        row_h = max(row_h, h)
    total_h = y + row_h + 2 * pad + label_h
    out = Image.new("RGBA", (max(width, x + pad) * scale, total_h * scale), BG)
    draw = ImageDraw.Draw(out)
    for pid, img, px, py in spots:
        out.alpha_composite(img.resize((img.width * scale, img.height * scale), Image.NEAREST),
                            (px * scale, py * scale))
        if labels:
            draw.text((px * scale, (py - label_h) * scale + 2),
                      pid.removeprefix("heat_").replace("shelf_end_", "end_"), fill=(236, 226, 210, 255))
    return out


def labelled(img: Image.Image, text: str, scale: int = 1) -> Image.Image:
    band = 14 * scale
    out = Image.new("RGBA", (img.width, img.height + band), (16, 14, 20, 255))
    out.alpha_composite(img.convert("RGBA"), (0, band))
    ImageDraw.Draw(out).text((4 * scale, 2 * scale), text, fill=(236, 226, 210, 255))
    return out


def drawn_views(out_dir: Path, layout_dir: Path | None) -> dict[str, int]:
    """The drawn view left of each 640 x 360 still, from landmarks (pen_entry: the entry shelf's edges at screen
    x 82..177 give 1038 exactly; the others by matching the lit model, +-1 px)."""
    views = {"pen_entry": 1038}
    cache = out_dir / "drawn_views.txt"
    if layout_dir is None:
        if cache.is_file():
            for line in cache.read_text(encoding="utf-8").splitlines():
                name, _, left = line.partition(" ")
                views[name] = int(left)
        return views
    import heat01_view as hv  # noqa: PLC0415
    strip = hv.room_strip(layout_dir)
    lines = []
    for still in sorted(out_dir.glob("scene_*_640x360.png")):
        name = still.name.removeprefix("scene_").removesuffix("_640x360.png")
        if name.startswith("before_"):
            continue
        left, err = hv.locate(strip, still)
        views[name] = 1038 if name == "pen_entry" else left
        lines.append(f"{name} {views[name]}")
    cache.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return views


def layout_diagram(still: Path, left: int, contract: dict, sizes: dict) -> Image.Image:
    img = Image.open(still).convert("RGBA").resize((1280, 720), Image.NEAREST)
    bands = Image.new("RGBA", img.size, (0, 0, 0, 0))
    for (y0, y1) in ((532, 539), (500, 507)):                      # the glob bands, tinted (blended, not painted)
        ImageDraw.Draw(bands).rectangle((0, (y0 - VIEW_TOP) * 2, 1279, (y1 - VIEW_TOP) * 2 - 1), fill=(255, 160, 40, 70))
    img.alpha_composite(bands)
    d = ImageDraw.Draw(img)

    def sx(x):
        return (x - left) * 2

    def sy(y):
        return (y - VIEW_TOP) * 2

    for y, label, col in ((W_LINE, "walking line W 580", (120, 255, 120)), (FB, "floor back line FB 560", (120, 200, 255))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
        d.text((560, sy(y) - 12), label, fill=col + (255,))
    for (y0, y1), label in (((532, 539), "glob band 532..538 (floor)"), ((500, 507), "glob band 500..506 (dock)")):
        d.text((8, sy(y0) - 11), label, fill=(255, 190, 90, 255))
    for (y0, y1), label in (((524, 560), "head band 524..560 (floor)"), ((492, 528), "head band 492..528 (dock)")):
        d.rectangle((0, sy(y0), 1279, sy(y1) - 1), outline=(200, 120, 255, 160))
        d.text((8, sy(y1) - 11), label, fill=(210, 150, 255, 255))
    for p in contract["room_dressing"]["corral"]["place"]:
        w, h = sizes[p["piece"]]
        x0, y0 = sx(p["x"]), sy(p["y"])
        if x0 > 1280 or x0 + w * 2 < 0:
            continue
        wall = p["layer"] == "backwall_decal"
        col = (255, 90, 60, 255) if wall else ((255, 210, 90, 255) if p["layer"] == "prop" else (200, 160, 70, 255))
        d.rectangle((x0, y0, x0 + w * 2 - 1, y0 + h * 2 - 1), outline=col)
        if wall or p["layer"] == "prop" or "scorch_floor" in p["piece"] or "bumper" in p["piece"]:
            d.text((x0 + 3, y0 + 3), p["piece"].removeprefix("heat_"), fill=col)
    for x, y, w, h in STEPS:
        d.rectangle((sx(x), sy(y), sx(x + w) - 1, sy(y + h) - 1), outline=(255, 255, 255, 255), width=2)
    d.rectangle((sx(1372), sy(300), sx(1428) - 1, sy(580) - 1), outline=(255, 60, 200, 255), width=2)
    d.rectangle((sx(1380), sy(532), sx(1420) - 1, sy(580) - 1), outline=(255, 60, 200, 160))
    d.text((sx(1372) + 4, sy(470)), "partition clear zone 1372..1428\n(art rect 1380,532 40x48: CORRAL-D)",
           fill=(255, 120, 220, 255))
    return img


def pair(before: Image.Image, after: Image.Image, scale: int, text: tuple) -> Image.Image:
    b = before.convert("RGBA")
    a = after.convert("RGBA")
    w, h = b.width, b.height
    out = Image.new("RGBA", (w * 2 + 8, h + 14), (16, 14, 20, 255))
    out.alpha_composite(b, (0, 14))
    out.alpha_composite(a, (w + 8, 14))
    dr = ImageDraw.Draw(out)
    dr.text((4, 2), text[0], fill=(236, 226, 210, 255))
    dr.text((w + 12, 2), text[1], fill=(236, 226, 210, 255))
    if scale > 1:
        out = out.resize((out.width * scale, out.height * scale), Image.NEAREST)
    return out


def main(argv: list[str]) -> int:
    out_dir = Path(argv[0])
    layout_dir = Path(argv[1]) if len(argv) > 1 and argv[1] else None
    out_dir.mkdir(parents=True, exist_ok=True)
    imgs = piece_images()
    contact(list(imgs.items()), 1, False, width=470).save(out_dir / "contact_sheet_x1.png")
    contact(list(imgs.items()), 4, True, width=330).save(out_dir / "contact_sheet_x4.png")
    strip = [imgs[p] for p in heat.PARTITION_IDS]
    s = Image.new("RGBA", (3 * 40 + 4 * 8, 48 + 16), BG)
    for k, im in enumerate(strip):
        s.alpha_composite(im, (8 + k * 48, 8))
    s.resize((s.width * 4, s.height * 4), Image.NEAREST).save(out_dir / "partition_states_x4.png")
    # the gate composites: the pen still with each state, cropped around Carl, Donut, the goblin and llama B
    gate = out_dir / "gate"
    rows = []
    for name in ("intact", "weakened", "broken"):
        im = Image.open(gate / f"scene_pen_{name}_1x.png").convert("RGBA").crop((20, 196, 500, 300))
        rows.append(labelled(im, f"{name}: gate composite (CORRAL-D pending), not runtime"))
    sheet = Image.new("RGBA", (480, sum(r.height for r in rows) + 8), (16, 14, 20, 255))
    y = 0
    for r in rows:
        sheet.alpha_composite(r, (0, y))
        y += r.height + 4
    sheet.save(out_dir / "partition_states_with_actors_1x.png")
    sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(out_dir / "partition_states_with_actors_3x.png")

    contract = json.loads(heat.CONTRACT.read_text(encoding="utf-8"))
    sizes = {p["id"]: tuple(p["rect"][2:]) for p in contract["pieces"]}
    views = drawn_views(out_dir, layout_dir)
    for name, label in (("pen_entry", "pen"), ("dock", "dock")):
        still = out_dir / f"scene_{name}_640x360.png"
        if still.is_file() and name in views:
            layout_diagram(still, views[name], contract, sizes).save(out_dir / f"layout_diagram_{label}_x2.png")
    for name in ("pen_entry", "dock"):
        before = out_dir / f"scene_before_{name}_640x360.png"
        after = out_dir / f"scene_{name}_640x360.png"
        if before.is_file() and after.is_file():
            text = ("before: origin/main (undressed room)", "after: AREA-HEAT-01 dressing")
            p1 = pair(Image.open(before), Image.open(after), 1, text)
            p1.save(out_dir / f"before_after_{name}_x1.png")
            p3 = pair(Image.open(out_dir / f"scene_before_{name}_1920x1080.png"),
                      Image.open(out_dir / f"scene_{name}_1920x1080.png"), 1, text) \
                if (out_dir / f"scene_before_{name}_1920x1080.png").is_file() else None
            if p3 is not None:
                p3.save(out_dir / f"before_after_{name}_x3.png")
    pen_before = out_dir / "scene_before_pen_entry_640x360.png"
    pen_after = out_dir / "scene_pen_entry_640x360.png"
    if pen_before.is_file() and pen_after.is_file():
        box = (10, 150, 150, 290)
        b, a = Image.open(pen_before).convert("RGBA").crop(box), Image.open(pen_after).convert("RGBA").crop(box)
        text = ("before (origin/main)", "after (AREA-HEAT-01)")
        pair(b, a, 1, text).save(out_dir / "scorch_wall_runtime_before_after_x1.png")
        pair(b, a, 3, text).save(out_dir / "scorch_wall_runtime_before_after_x3.png")
        # gate note N1: world x 1112..1176, y 470..520 -> screen x 74..138, y 181..231 at view (1038, 289)
        box = (74, 181, 138, 231)
        b, a = Image.open(pen_before).convert("RGBA").crop(box), Image.open(pen_after).convert("RGBA").crop(box)
        pair(b, a, 12, ("before", "after")).save(out_dir / "crop_n1_plume_right_x12.png")
    print(f"sheets in {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
