#!/usr/bin/env python3
"""AREA-RINGS-01 offline evidence sheets (deterministic, from the committed grids and the real stills).

    python3 scripts/make_rings01_sheets.py OUT_DIR

Reads ``OUT_DIR/scene_*_640x360.png`` and ``OUT_DIR/capture_lines.txt`` (the view left of every still) and writes
into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every kit piece at native size on a neutral ground (the 4x
  sheet labelled; the three parallax strips are in ``parallax_x2.png``);
- ``shutter_states_x4.png``: the stone shutter through Open, Warning, Closing, Closed, the floor-contact puff and
  Opening (the section 6 table of the spec, drawn from the same piece crops the presenter uses);
- ``layout_diagram_court_x2.png`` / ``layout_diagram_split_x2.png``: the room schema (walking line, floor back
  line, soffit band and ceiling slab, every dressing rect, the platforms, the shutter rect) over a real still;
- ``crop_shutter_court_x4.png``: a 4x crop of the court's shutter, housing and jambs from a real still.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_rings01 as rings  # noqa: E402

BG = (58, 54, 66, 255)
W_LINE, FB = 580, 560
VIEW_TOP = 292          # layout y of the 640x360 view's top row (walk line 580 - 288)


def piece_images() -> dict[str, Image.Image]:
    out = {}
    for name in rings.ATLASES:
        for pid, img in rings.build_pieces(name):
            out[pid] = rings.to_image(img)
    return out


def contact(pieces, scale: int, labels: bool, width: int = 460) -> Image.Image:
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
            draw.text((px * scale, (py - label_h) * scale + 2), pid, fill=(236, 226, 210, 255))
    return out


def shutter_states(imgs: dict[str, Image.Image]) -> Image.Image:
    """One stage per state: brick, jambs, the slab crop, the extras, the housing on top (spec section 6)."""
    sw, sh = 96, 96
    rect_x, rect_y, rect_w, rect_h = 32, 8, 32, 68          # the shutter rect in the stage; its bottom is the floor
    floor_y = rect_y + rect_h
    key = imgs["ring_shutter_closed"]
    states = [
        ("Open", 0, []),
        ("Warning t0", 0, [("ring_shutter_shadow_a", rect_x - 4, floor_y - 4), ("ring_shutter_grit_a", rect_x, rect_y + 6)]),
        ("Warning t20", 0, [("ring_shutter_shadow_b", rect_x - 4, floor_y - 4), ("ring_shutter_grit_b", rect_x, rect_y + 6)]),
        ("Warning t35", 0, [("ring_shutter_shadow_c", rect_x - 4, floor_y - 4), ("ring_shutter_grit_a", rect_x, rect_y + 6)]),
        ("Closing 17", 17, [("ring_shutter_shadow_c", rect_x - 4, floor_y - 4), ("ring_shutter_dust_a", rect_x - 8, rect_y + 17 - 6)]),
        ("Closing 34", 34, [("ring_shutter_shadow_c", rect_x - 4, floor_y - 4), ("ring_shutter_dust_b", rect_x - 8, rect_y + 34 - 6)]),
        ("Closing 51", 51, [("ring_shutter_shadow_c", rect_x - 4, floor_y - 4), ("ring_shutter_dust_a", rect_x - 8, rect_y + 51 - 6)]),
        ("Closed t0", 68, [("ring_shutter_dust_c", rect_x - 8, floor_y - 6)]),
        ("Closed t8+", 68, []),
        ("Opening 34", 34, [("ring_shutter_dust_b", rect_x - 8, rect_y + 34 - 6)]),
    ]
    cols = 5
    sheet = Image.new("RGBA", (cols * sw, 2 * (sh + 14)), (16, 14, 20, 255))
    draw = ImageDraw.Draw(sheet)
    for i, (label, rows, extras) in enumerate(states):
        stage = Image.new("RGBA", (sw, sh), (96, 52, 44, 255))              # a patch of lit brick
        for y in range(0, sh, 8):
            stage.paste((84, 44, 40, 255), (0, y, sw, y + 1))
        stage.paste((70, 48, 40, 255), (0, floor_y, sw, sh))                 # the floor
        stage.alpha_composite(imgs["ring_shutter_jamb_l"], (rect_x - 8, rect_y))
        stage.alpha_composite(imgs["ring_shutter_jamb_r"], (rect_x + rect_w, rect_y))
        if rows:
            stage.alpha_composite(key.crop((0, key.height - rows, key.width, key.height)), (rect_x, rect_y))
        for pid, ex, ey in extras:
            if not pid.startswith("ring_shutter_dust"):
                stage.alpha_composite(imgs[pid], (ex, ey))
        stage.alpha_composite(imgs["ring_shutter_housing"], (rect_x - 8, rect_y))
        for pid, ex, ey in extras:
            if pid.startswith("ring_shutter_dust"):
                stage.alpha_composite(imgs[pid], (ex, ey))
        x, y = (i % cols) * sw, (i // cols) * (sh + 14)
        sheet.alpha_composite(stage, (x, y + 14))
        draw.text((x + 2, y + 1), label, fill=(236, 226, 210, 255))
    return sheet.resize((sheet.width * 4, sheet.height * 4), Image.NEAREST)


def view_lefts(out_dir: Path) -> dict[str, int]:
    """The sim camera x of each 640x360 still. It equals the window's view left only where the presentation camera
    has settled (the hero standing, or the camera clamped at the room's end): the diagrams use such stills."""
    lefts = {}
    for line in (out_dir / "capture_lines.txt").read_text(encoding="utf-8").splitlines():
        m = re.search(r"name=(\S+) room=\S+ tick=\d+ size=640x360 cam=(\d+)", line)
        if m:
            lefts[m.group(1)] = int(m.group(2))
    return lefts


def layout_diagram(room: str, still: Path, left: int, contract: dict, sizes: dict) -> Image.Image:
    img = Image.open(still).convert("RGBA").resize((1280, 720), Image.NEAREST)
    d = ImageDraw.Draw(img)

    def sx(x):
        return (x - left) * 2

    def sy(y):
        return (y - VIEW_TOP) * 2

    for y, label, col in ((W_LINE, "walking line W 580", (120, 255, 120)), (FB, "floor back line FB 560", (120, 200, 255))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
        d.text((560, sy(y) - 12), label, fill=col + (255,))
    d.rectangle((0, sy(336), 1279, sy(352) - 1), outline=(255, 120, 120, 255), width=2)
    d.text((560, sy(336) - 12), "soffit band 336..352 = ceiling slab solid", fill=(255, 120, 120, 255))
    shown = {"ring_soffit", "ring_ledge_mid", "ring_ledge_end_l", "ring_ledge_end_r", "ring_arc_upper_a", "ring_arc_upper_b"}
    for p in contract["room_dressing"][room]["place"]:
        w, h = sizes[p["piece"]]
        col = (255, 210, 90, 255) if p["piece"] not in shown else (200, 160, 70, 255)
        if p["piece"] in ("ring_soffit",):
            continue
        d.rectangle((sx(p["x"]), sy(p["y"]), sx(p["x"] + w) - 1, sy(p["y"] + h) - 1), outline=col)
        if p["piece"] not in shown:
            d.text((sx(p["x"]) + 3, sy(p["y"]) + 3), p["piece"].replace("ring_", ""), fill=col)
    for x, y, w in rings.PLATFORMS[room]:
        d.rectangle((sx(x), sy(y), sx(x + w) - 1, sy(y + 16) - 1), outline=(255, 255, 255, 255), width=2)
    if room == "ring_court":
        d.rectangle((sx(1200), sy(512), sx(1232) - 1, sy(580) - 1), outline=(255, 80, 200, 255), width=3)
        d.text((sx(1200) - 20, sy(580) + 4), "shutter rect 1200,512 32x68", fill=(255, 80, 200, 255))
    return img


def main(argv: list[str]) -> int:
    out_dir = Path(argv[0])
    out_dir.mkdir(parents=True, exist_ok=True)
    imgs = piece_images()
    big = [(pid, img) for pid, img in imgs.items() if not pid.startswith(("ring_tiers", "ring_camp_far"))]
    contact(list(imgs.items()), 1, False, width=470).save(out_dir / "contact_sheet_x1.png")
    contact(big, 4, True, width=440).save(out_dir / "contact_sheet_x4.png")
    par = [(pid, imgs[pid]) for pid in ("ring_tiers_far", "ring_tiers_mid", "ring_camp_far")]
    contact(par, 2, True, width=450).save(out_dir / "parallax_x2.png")
    shutter_states(imgs).save(out_dir / "shutter_states_x4.png")
    contract = json.loads(rings.CONTRACT.read_text(encoding="utf-8"))
    sizes = {p["id"]: tuple(p["rect"][2:]) for p in contract["pieces"]}
    lefts = view_lefts(out_dir)
    layout_diagram("ring_court", out_dir / "scene_after_walkway_640x360.png", lefts["after_walkway"], contract,
                   sizes).save(out_dir / "layout_diagram_court_x2.png")
    layout_diagram("ring_split", out_dir / "scene_marker_exit_640x360.png", lefts["marker_exit"], contract,
                   sizes).save(out_dir / "layout_diagram_split_x2.png")
    still = Image.open(out_dir / "scene_after_walkway_640x360.png").convert("RGBA")
    left = lefts["after_walkway"]
    x0, y0 = 1160 - left, 470 - VIEW_TOP
    still.crop((x0, y0, x0 + 120, y0 + 110)).resize((480, 440), Image.NEAREST).save(out_dir / "crop_shutter_court_x4.png")
    print(f"sheets in {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
