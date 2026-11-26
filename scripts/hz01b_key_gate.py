#!/usr/bin/env python3
"""AREA-HAZARDS-01B design gate: palette metrics and the 1x readability composites.

This is the design-step record, not the production pipeline. It reads the two
hand-drawn key grids in ``assets/source/props/floor01/hz01b_keys/`` and the
committed acid, slime, ring and Carl grids, applies the adopted palette
decisions (HZB-AD1 teal acid ramp, HZB-AD2 ring tape backing) as pure
recolours, and composites them into two 640x360 game captures:

* room 4: ``docs/evidence/f01-danger-fx/scene/acid_full_640x360.png``
* room 6: a capture of the room-6 open scenario (pass it with ``--room6``)

Usage::

    python3 scripts/hz01b_key_gate.py --room6 <room6_640x360.png> --out <dir>

It prints the CIEDE2000 table (normal vision and Machado 2009 protan, deutan,
tritan at severity 1.0) and writes the composites, 3x crops, a CVD strip and
the key sheet. The scene light is approximated by a flat multiply (0.9 in the
dim floor areas, 1.0 near the torch).
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
KEYS = ROOT / "assets/source/props/floor01/hz01b_keys"
FLOOR_Y = 287  # the room-4 and room-6 walking line in both captures

# HZB-AD1: the teal acid ramp, a 1:1 index remap of the dfx_acid palette order.
ACID_RAMP = ["#061418", "#0d3034", "#124c50", "#197a7e", "#2eb2a8",
             "#78e2d4", "#c6f8ee", "#eafffa", "#a4c4c0", "#587a78"]
FLAME_00 = "#8a2a12"


def hx(h: str) -> tuple[int, int, int]:
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def s2l(c: float) -> float:
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def l2s(c: float) -> int:
    c = max(0.0, min(1.0, c))
    return round(255 * (12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055))


MACHADO = {
    "protan": [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
    "deutan": [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
    "tritan": [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
}


def sim(rgb, kind):
    lin = [s2l(c) for c in rgb[:3]]
    m = MACHADO[kind]
    return tuple(l2s(sum(m[i][j] * lin[j] for j in range(3))) for i in range(3))


def lab(rgb):
    r, g, b = (s2l(c) for c in rgb[:3])
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    f = lambda t: t ** (1 / 3) if t > 0.008856 else 7.787 * t + 16 / 116  # noqa: E731
    return 116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))


def de2000(c1, c2) -> float:
    l1, a1, b1 = lab(c1)
    l2, a2, b2 = lab(c2)
    cb = (math.hypot(a1, b1) + math.hypot(a2, b2)) / 2
    g = 0.5 * (1 - math.sqrt(cb ** 7 / (cb ** 7 + 25 ** 7)))
    a1p, a2p = a1 * (1 + g), a2 * (1 + g)
    c1p, c2p = math.hypot(a1p, b1), math.hypot(a2p, b2)
    h1 = math.degrees(math.atan2(b1, a1p)) % 360
    h2 = math.degrees(math.atan2(b2, a2p)) % 360
    dl, dc = l2 - l1, c2p - c1p
    dh = 0.0 if c1p * c2p == 0 else (h2 - h1 if abs(h2 - h1) <= 180 else h2 - h1 - 360 * math.copysign(1, h2 - h1))
    dhh = 2 * math.sqrt(c1p * c2p) * math.sin(math.radians(dh / 2))
    lb, cbp = (l1 + l2) / 2, (c1p + c2p) / 2
    if c1p * c2p == 0:
        hb = h1 + h2
    elif abs(h1 - h2) <= 180:
        hb = (h1 + h2) / 2
    else:
        hb = (h1 + h2 + 360) / 2 if h1 + h2 < 360 else (h1 + h2 - 360) / 2
    t = (1 - 0.17 * math.cos(math.radians(hb - 30)) + 0.24 * math.cos(math.radians(2 * hb))
         + 0.32 * math.cos(math.radians(3 * hb + 6)) - 0.2 * math.cos(math.radians(4 * hb - 63)))
    dth = 30 * math.exp(-((hb - 275) / 25) ** 2)
    rc = 2 * math.sqrt(cbp ** 7 / (cbp ** 7 + 25 ** 7))
    sl = 1 + 0.015 * (lb - 50) ** 2 / math.sqrt(20 + (lb - 50) ** 2)
    sc, sh = 1 + 0.045 * cbp, 1 + 0.015 * cbp * t
    rt = -math.sin(math.radians(2 * dth)) * rc
    return math.sqrt((dl / sl) ** 2 + (dc / sc) ** 2 + (dhh / sh) ** 2 + rt * (dc / sc) * (dhh / sh))


def load(rel: str) -> dict:
    return json.loads((ROOT / rel).read_text())


def pal_of(grid: dict) -> dict[str, str]:
    return {p["key"]: p["rgba"][:7] for p in grid["palette"]}


def grid_img(rows: list[str], pal: dict[str, str], light: float = 1.0) -> Image.Image:
    im = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch in pal:
                im.putpixel((x, y), tuple(min(255, round(v * light)) for v in hx(pal[ch])) + (255,))
    return im


def ring_tape_rows(ring: dict, frame: int, radius: int) -> list[str]:
    """HZB-AD2 preview: every band pixel of the outer row (distance >= R + 0.25 from the pivot) becomes
    flame_00 ('@'): both dash colours of ``ring_march`` and the outer row of ``ring_hot``. The production
    rule is the author script's ``r <= d < r + 1`` row; this pixel-centre approximation is for the gate only."""
    px, py = ring["pivot"]
    rows = ring["frames"][frame]["cels"]["body"]
    names = ("brass_05", "flame_02", "flame_03", "flame_04")
    keys = {p["key"] for p in ring["palette"] if p["name"] in names}
    return ["".join("@" if ch in keys and math.hypot(x - px, y - py) >= radius + 0.25 else ch
                    for x, ch in enumerate(row)) for y, row in enumerate(rows)]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--room6", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)

    acid = load("assets/source/props/floor01/dfx_acid.grid.json")
    slime = load("assets/source/characters/slime/slime_01.grid.json")
    ring = load("assets/source/props/floor01/dfx_ring_r48.grid.json")
    carl = load("assets/source/characters/carl/carl_h52_melee.grid.json")
    splash = json.loads((KEYS / "acid_splash_key.grid.json").read_text())
    corr = json.loads((KEYS / "corrosion_key.grid.json").read_text())

    old = pal_of(acid)
    new = {p["key"]: ACID_RAMP[i] for i, p in enumerate(acid["palette"])}
    slime_pal = pal_of(slime)
    ring_old = pal_of(ring)
    ring_new = dict(ring_old, **{"@": FLAME_00})

    # Palette metrics: slime ramp vs acid ramp, entry by entry (shade..sheen).
    slime_ramp = [slime_pal[k] for k in "bcdfh"]
    acid_ramp = ACID_RAMP[2:7]
    for kind in ("normal", "protan", "deutan", "tritan"):
        f = (lambda c: c) if kind == "normal" else (lambda c, k=kind: sim(c, k))
        vals = " ".join(f"{de2000(f(hx(a)), f(hx(b))):5.1f}" for a, b in zip(slime_ramp, acid_ramp))
        print(f"dE00 slime vs acid  {kind:7s} shade/mid/light/lit/sheen: {vals}")

    hurt = next(t for t in carl["tags"] if t["name"] == "hurt")["from"]

    def carl_img(light: float) -> Image.Image:
        im = Image.new("RGBA", tuple(carl["canvas"]), (0, 0, 0, 0))
        for layer in carl["layers"]:
            rows = carl["frames"][hurt]["cels"].get(layer)
            if rows:
                im.alpha_composite(grid_img(rows, pal_of(carl), light))
        return im

    def pool(pal, light):  # a 64 px patch: both caps of the 80 px strip, as the presenter crops
        rows = [r[:32] + r[48:] for r in acid["frames"][0]["cels"]["body"]]
        return grid_img(rows, pal, light)

    sp_rows = splash["frames"][0]["cels"]["body"]
    sp_pivot = splash["pivot"]
    co_rows = corr["frames"][0]["cels"]["body"]
    co_pivot = corr["pivot"]
    sp_pal, co_pal = pal_of(splash), pal_of(corr)

    def compose(bg: Path, name: str, spots) -> Image.Image:
        im = Image.open(bg).convert("RGBA")
        for kind, x, y, light in spots:
            if kind == "pool_new":
                im.alpha_composite(pool(new, light), (x, y - 9))
            elif kind == "pool_old":
                im.alpha_composite(pool(old, light), (x, y - 9))
            elif kind == "slime":
                im.alpha_composite(grid_img(slime["frames"][0]["cels"]["body"], slime_pal, light),
                                   (x - slime["pivot"][0], y - slime["pivot"][1] - 1))
            elif kind == "carl":
                im.alpha_composite(carl_img(light), (x - carl["pivot"][0], y - carl["pivot"][1]))
            elif kind == "splash":
                im.alpha_composite(grid_img(sp_rows, sp_pal, light), (x - sp_pivot[0], y - sp_pivot[1]))
            elif kind == "corr":
                im.alpha_composite(grid_img(co_rows, co_pal, light), (x - co_pivot[0], y - co_pivot[1]))
        im.save(out / f"gate_{name}_640x360.png")
        return im

    room4_bg = ROOT / "docs/evidence/f01-danger-fx/scene/acid_full_640x360.png"
    r4 = compose(room4_bg, "room4", [
        ("pool_new", 40, FLOOR_Y, 1.0), ("corr", 72, FLOOR_Y + 1, 1.0), ("splash", 60, FLOOR_Y, 1.0),
        ("pool_new", 400, FLOOR_Y, 0.9), ("slime", 420, FLOOR_Y, 0.9), ("corr", 432, FLOOR_Y + 1, 0.9),
        ("pool_new", 500, FLOOR_Y, 0.9), ("carl", 516, FLOOR_Y, 0.9), ("splash", 516, FLOOR_Y, 0.9),
    ])
    r6 = compose(args.room6, "room6", [
        ("pool_old", 100, FLOOR_Y, 0.9), ("slime", 120, FLOOR_Y, 0.9),
        ("pool_new", 470, FLOOR_Y, 0.95), ("slime", 490, FLOOR_Y, 0.95), ("corr", 502, FLOOR_Y + 1, 0.95),
        ("pool_new", 560, FLOOR_Y, 0.95), ("carl", 610, FLOOR_Y, 0.95), ("splash", 610, FLOOR_Y, 0.95),
    ])
    for name, im, box in [("room4_right", r4, (380, 220, 580, 310)), ("room6_right", r6, (440, 220, 640, 310))]:
        c = im.crop(box)
        c.resize((c.width * 3, c.height * 3), Image.NEAREST).save(out / f"gate_{name}_x3.png")

    # CVD strip: room 6, old green pool + slime (left) vs teal pool + slime (right).
    strip = r6.crop((60, 230, 640, 300)).convert("RGB")
    sheets = [strip]
    for kind in ("protan", "deutan", "tritan"):
        s = strip.copy()
        px = s.load()
        for yy in range(s.height):
            for xx in range(s.width):
                px[xx, yy] = sim(px[xx, yy], kind)
        sheets.append(s)
    cvd = Image.new("RGB", (strip.width, strip.height * 4))
    for i, s in enumerate(sheets):
        cvd.paste(s, (0, i * strip.height))
    cvd.resize((cvd.width * 2, cvd.height * 2), Image.NEAREST).save(out / "gate_room6_cvd_x2.png")

    # HZB-AD2 ring sheet: the same background patch twice, current ring left, tape-backed ring right.
    # Columns: march now, march HZB-AD2, hot now, hot HZB-AD2 (frame 2; the inner row keeps flame_04).
    hot = next(t for t in ring["tags"] if t["name"] == "ring_hot")["from"]
    sheet = Image.new("RGBA", (440, 220))
    r4bg, r6bg = Image.open(room4_bg).convert("RGBA"), Image.open(args.room6).convert("RGBA")
    for row, (bg, box) in enumerate([(r4bg, (2, 115, 112, 225)), (r6bg, (300, 90, 410, 200))]):
        for col, (rows, pal) in enumerate([(ring["frames"][0]["cels"]["body"], ring_old),
                                           (ring_tape_rows(ring, 0, 48), ring_new),
                                           (ring["frames"][hot]["cels"]["body"], ring_old),
                                           (ring_tape_rows(ring, hot, 48), ring_new)]):
            patch = bg.crop(box)
            patch.alpha_composite(grid_img(rows, pal), (55 - ring["pivot"][0], 55 - ring["pivot"][1]))
            sheet.paste(patch, (col * 110, row * 110))
    sheet.resize((1320, 660), Image.NEAREST).save(out / "ring_tape_old_vs_new_x3.png")

    # Key sheet at 8x on the floor-face colour.
    ks = Image.new("RGBA", (32 * 8 + 32 * 8 + 48, 24 * 8 + 16), (58, 42, 42, 255))
    ks.alpha_composite(grid_img(sp_rows, sp_pal).resize((256, 192), Image.NEAREST), (8, 8))
    ks.alpha_composite(grid_img(co_rows, co_pal).resize((256, 80), Image.NEAREST), (280, 8))
    ks.save(out / "keys_x8.png")


if __name__ == "__main__":
    main()
