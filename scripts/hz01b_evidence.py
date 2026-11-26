#!/usr/bin/env python3
"""AREA-HAZARDS-01B: the pixel-art evidence of the splash, the corrosion decals and the Rot marks.

Writes into ``docs/evidence/area-hazards-01b`` (``--out``):

- ``contact_sheet_x8.png``: every frame of every new grid at 8x on the dim Floor 1 lane tone, one row
  per tag, labelled;
- ``splash_x1.gif`` and ``splash_x4.gif``: the five splash frames at tick timing (2, 3, 3, 4, 4 ticks of
  60 Hz), unmirrored then mirrored, on the lane tone;
- ``cvd_room4_slime_pool.png``: the room 4 slime on its teal pool (a crop of the window capture
  ``scene/r4_slime_pool_640x360.png``) at 6x, rows from the top: normal, protan, deutan, tritan
  (Machado 2009, severity 1.0);
- ``SHA256SUMS``: the checksums of those four images;

The per-sheet toolkit evidence (``python3 -m scripts.pixelart evidence``) and the window captures
(``scripts/make_hz01b_evidence.sh``) are produced by their own commands, see the evidence README. This script
draws only from the committed grids and the committed window capture; it is deterministic (no timestamps).

Usage: ``python3 scripts/hz01b_evidence.py [--out DIR]``
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import hz01b_key_gate as gate  # noqa: E402

PROPS = ROOT / "assets/source/props/floor01"
LANE = (0x4F, 0x31, 0x2E)  # the dim Floor 1 floor tone of the design-step gate
SHEETS = [  # (grid, label)
    ("dfx_acid_splash", "splash"),
    ("dfx_corr_face", "corr_face"),
    ("dfx_corr_top", "corr_top"),
    ("dfx_corr_wall", "corr_wall"),
    ("dfx_rot_nest", "nest"),
    ("dfx_rot_trail", "trail"),
    ("dfx_rot_drip", "drip"),
]
SPLASH_TICKS = [2, 3, 3, 4, 4]


def grid_of(name: str) -> dict:
    return json.loads((PROPS / f"{name}.grid.json").read_text())


def frame_images(name: str) -> list[tuple[str, Image.Image]]:
    """``(tag, image)`` per frame of a grid, in order."""
    g = grid_of(name)
    pal = gate.pal_of(g)
    tags = {}
    for t in g["tags"]:
        for i in range(t["from"], t["to"] + 1):
            tags[i] = t["name"]
    return [(tags[i], gate.grid_img(f["cels"]["body"], pal)) for i, f in enumerate(g["frames"])]


def on_lane(im: Image.Image, scale: int, w: int | None = None, h: int | None = None, pad: int = 1) -> Image.Image:
    w, h = w or im.width, h or im.height
    out = Image.new("RGB", ((w + 2 * pad) * scale, (h + 2 * pad) * scale), LANE)
    big = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    out.paste(big, (pad * scale, pad * scale), big)
    return out


def contact_sheet(scale: int = 8) -> Image.Image:
    rows: list[tuple[str, list[Image.Image]]] = []
    for name, label in SHEETS:
        g = grid_of(name)
        by_tag: dict[str, list[Image.Image]] = {}
        for tag, im in frame_images(name):
            by_tag.setdefault(tag, []).append(on_lane(im, scale, g["canvas"][0], g["canvas"][1]))
        for tag, ims in by_tag.items():
            rows.append((f"{name}  {tag}  {g['canvas'][0]}x{g['canvas'][1]}", ims))
    label_h = 14
    gap = 6
    width = max(sum(i.width + gap for i in ims) for _l, ims in rows) + gap
    height = sum(max(i.height for i in ims) + label_h + gap for _l, ims in rows) + gap
    sheet = Image.new("RGB", (width, height), (0x16, 0x10, 0x14))
    draw = ImageDraw.Draw(sheet)
    y = gap
    for label, ims in rows:
        draw.text((gap, y), label, fill=(0xD8, 0xC8, 0xB0))
        y += label_h
        x = gap
        for im in ims:
            sheet.paste(im, (x, y))
            x += im.width + gap
        y += max(i.height for i in ims) + gap
    return sheet


def splash_gif(out: Path, scale: int) -> None:
    """The splash at tick timing, then mirrored; the lane tone behind; looped with a short hold."""
    g = grid_of("dfx_acid_splash")
    frames = [im for _t, im in frame_images("dfx_acid_splash")]
    seq: list[Image.Image] = []
    durations: list[int] = []
    for mirrored in (False, True):
        for im, ticks in zip(frames, SPLASH_TICKS):
            im2 = im.transpose(Image.FLIP_LEFT_RIGHT) if mirrored else im
            seq.append(on_lane(im2, scale, g["canvas"][0], g["canvas"][1], pad=2))
            durations.append(round(ticks * 1000 / 60))
        # a beat of the settled water before the next play
        seq.append(on_lane(Image.new("RGBA", frames[0].size), scale, g["canvas"][0], g["canvas"][1], pad=2))
        durations.append(300)
    pal = seq[0].convert("P", palette=Image.ADAPTIVE, colors=64)
    imgs = [im.quantize(palette=pal, dither=Image.NONE) for im in seq]
    imgs[0].save(out, save_all=True, append_images=imgs[1:], duration=durations, loop=0, disposal=1)


def cvd_strip(still: Path, out: Path, box=(205, 256, 125, 42), scale: int = 6) -> None:
    im = Image.open(still).convert("RGB").crop((box[0], box[1], box[0] + box[2], box[1] + box[3]))
    big = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    rows = [("normal", big)]
    for kind in ("protan", "deutan", "tritan"):
        px = [gate.sim(p, kind) for p in im.get_flattened_data()]
        sim = Image.new("RGB", im.size)
        sim.putdata(px)
        rows.append((kind, sim.resize(big.size, Image.NEAREST)))
    label_h = 14
    sheet = Image.new("RGB", (big.width, (big.height + label_h) * 4), (0x16, 0x10, 0x14))
    draw = ImageDraw.Draw(sheet)
    for i, (label, r) in enumerate(rows):
        y = i * (big.height + label_h)
        draw.text((4, y + 1), label, fill=(0xD8, 0xC8, 0xB0))
        sheet.paste(r, (0, y + label_h))
    sheet.save(out)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=ROOT / "docs/evidence/area-hazards-01b")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    contact_sheet(8).save(args.out / "contact_sheet_x8.png")
    splash_gif(args.out / "splash_x1.gif", 1)
    splash_gif(args.out / "splash_x4.gif", 4)
    cvd_strip(args.out / "scene/r4_slime_pool_640x360.png", args.out / "cvd_room4_slime_pool.png")
    names = ("contact_sheet_x8.png", "splash_x1.gif", "splash_x4.gif", "cvd_room4_slime_pool.png")
    (args.out / "SHA256SUMS").write_text(
        "".join(f"{hashlib.sha256((args.out / n).read_bytes()).hexdigest()}  {n}\n" for n in names))
    for n in names:
        print(args.out / n)


if __name__ == "__main__":
    main()
