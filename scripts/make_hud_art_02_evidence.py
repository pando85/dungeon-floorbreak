#!/usr/bin/env python3
"""HUD-ART-02 evidence helpers (deterministic; nearest-neighbour scaling only).

  sheet OUTDIR            contact sheet of the ten new atlas regions (key caps and item chips) at 1x and 8x,
                          each beside the pad disks (glyph_a..glyph_triangle), from the exported atlas
  crop FRAME OUT X Y W H S   crop a window capture to a box and scale it by S (3x crops of the HUD)
  at2 OUT FRAME:ROOM...   AT-2: contrast of the notice plate rim (metal_hi) against the median world colour
                          in a 2 px band around the notice rect, for each room capture frame (no notice drawn)
  sums DIR                write SHA256SUMS for the files of DIR (not the README, not sub-folders)
"""
from __future__ import annotations

import hashlib
import json
import statistics
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "assets/manifests/hud_hybrid_a_v1.json"
ATLAS = ROOT / "generated/art/hud_hybrid_a.png"
GROUND = (0x26, 0x26, 0x30, 255)
NEW = ["key_j", "key_k", "key_l", "key_q", "key_space", "key_left", "key_right", "key_down",
       "chip_dynamite", "chip_puddle"]
PADS = ["glyph_a", "glyph_b", "glyph_x", "glyph_y", "glyph_cross", "glyph_circle", "glyph_square",
        "glyph_triangle"]
METAL_HI = (0x8a, 0x7e, 0x8c)


def up(im: Image.Image, n: int) -> Image.Image:
    return im.resize((im.width * n, im.height * n), Image.NEAREST)


def regions() -> dict[str, tuple[int, int, int, int]]:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {r["name"]: tuple(r["rect"]) for r in data["hud"]["regions"]}


def sheet(out: Path, scale: int) -> None:
    atlas = Image.open(ATLAS).convert("RGBA")
    rects = regions()
    font = ImageFont.load_default()
    pad = 3 * scale
    rows = [("pad disks (existing)", PADS), ("keyboard caps (HUD-ART-02)", NEW[:8]), ("item chips (HUD-ART-02)", NEW[8:])]
    cell = 20 * scale
    width = pad + max(len(r[1]) for r in rows) * (cell + pad) + 2 * pad
    height = len(rows) * (14 + 11 * scale + 2 * pad) + pad
    im = Image.new("RGBA", (max(width, 240 if scale == 1 else width), height), GROUND)
    d = ImageDraw.Draw(im)
    y = pad
    for title, names in rows:
        d.text((pad, y), title, fill=(0xf4, 0xe6, 0xd2, 255), font=font)
        y += 12
        x = pad
        for n in names:
            rx, ry, w, h = rects[n]
            piece = up(atlas.crop((rx, ry, rx + w, ry + h)), scale)
            im.alpha_composite(piece, (x, y))
            x += (19 if n == "key_space" else 11) * scale + pad
        y += 11 * scale + 2 * pad
    im.convert("RGB").save(out)


def lum(c) -> float:
    def f(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])


def contrast(a, b) -> float:
    hi, lo = max(lum(a), lum(b)), min(lum(a), lum(b))
    return (hi + 0.05) / (lo + 0.05)


def at2(out: Path, items: list[str]) -> None:
    """The notice sits at x = view_w - 236 (margin 8 + 204 + slide 0 -> 404 at 640 wide)... the rect is
    read from the layout used by the HUD: 640x360 -> (404, 320), 640x400 -> (404, 360)."""
    lines = []
    for item in items:
        path, room = item.rsplit(":", 1)
        im = Image.open(path).convert("RGB")
        w, h = im.size
        ny = 320 if h == 360 else 360
        nx = 404
        band = []
        for x in range(nx - 2, nx + 204 + 2):
            for y in (ny - 2, ny - 1, ny + 32, ny + 33):
                if 0 <= y < h:
                    band.append(im.getpixel((x, y)))
        for y in range(ny, ny + 32):
            for x in (nx - 2, nx - 1, nx + 204, nx + 205):
                if 0 <= x < w:
                    band.append(im.getpixel((x, y)))
        med = tuple(int(statistics.median(p[i] for p in band)) for i in range(3))
        lines.append(f"{room:10s} {w}x{h} median_bg={med} rim_contrast={contrast(METAL_HI, med):.2f}")
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


def sums(d: Path) -> None:
    lines = []
    for p in sorted(d.iterdir()):  # top level only: the design-gate sub-folder has its own SHA256SUMS
        if p.is_file() and p.name not in ("SHA256SUMS", "README.md"):
            lines.append(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}")
    (d / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    cmd = argv[0]
    if cmd == "sheet":
        out = Path(argv[1])
        out.mkdir(parents=True, exist_ok=True)
        sheet(out / "new-regions-sheet-1x.png", 1)
        sheet(out / "new-regions-sheet-8x.png", 8)
    elif cmd == "crop":
        frame, out, x, y, w, h, s = argv[1], argv[2], *map(int, argv[3:8])
        im = Image.open(frame).convert("RGB").crop((x, y, x + w, y + h))
        up(im, s).save(out)
    elif cmd == "at2":
        at2(Path(argv[1]), argv[2:])
    elif cmd == "sums":
        sums(Path(argv[1]))
    else:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
