#!/usr/bin/env python3
"""AUDIENCE-01 part A evidence: viewer icon contact sheets and static mock composites.

Reads the exported atlas (generated/art/hud_hybrid_a.png, from scripts/export_assets.sh) and the
committed manifest; writes into docs/evidence/audience-01/ plus SHA256SUMS. Deterministic
(nearest-neighbour scaling, fixed layout).

Images written:
  contact_sheet_x1.png, contact_sheet_x4.png        the 16 viewer atlas regions, labelled, plus the
                                                    code-drawn decimal point (not an atlas region)
  mock-composite-{360,400}-{normal,high-contrast}-1x.png
                                                    both plates over the real HUD captures with the
                                                    eye, the worst-case 999M+ (Carl) and 99.9K with a
                                                    decimal point (Donut), and the trend arrows at the
                                                    design-review coordinates. Mock composite, not
                                                    runtime: the real captures carry no viewer art.
  mock-composite-{360,400}-{normal,high-contrast}-plates-4x.png
                                                    4x crop of the two plates of the same composite

The mock tints the masks directly (amber number, amber_dim or amber down arrow); the runtime
tints by multiplication, which part C must check.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_hud_hybrid_a as author  # noqa: E402

MANIFEST = ROOT / "assets/manifests/hud_hybrid_a_v1.json"
ATLAS = ROOT / "generated/art/hud_hybrid_a.png"
CAPTURES = ROOT / "docs/evidence/hud-hybrid-a"
OUT = ROOT / "docs/evidence/audience-01"
GROUND = (0x26, 0x26, 0x30, 255)
LABEL = "MOCK COMPOSITE, NOT RUNTIME"

# plate origins and plate-relative element positions (design review section 3.1)
PLATES = {"carl": (8, 8), "donut": (468, 8)}
EYE_AT, NUM_AT, TREND_AT = (92, 5), (104, 5), (153, 5)
ADVANCE = {"dot": 2}  # every other glyph advances 6
NUM_ADVANCE = 6


def rgb(token: str) -> tuple[int, int, int, int]:
    h = author.HUD[token]
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) + (255,)


def up(im: Image.Image, n: int) -> Image.Image:
    return im.resize((im.width * n, im.height * n), Image.NEAREST)


def regions() -> dict[str, tuple[int, int, int, int]]:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {r["name"]: tuple(r["rect"]) for r in data["hud"]["regions"]}


def piece(atlas: Image.Image, rect) -> Image.Image:
    x, y, w, h = rect
    return atlas.crop((x, y, x + w, y + h))


def tinted(im: Image.Image, colour: tuple[int, int, int, int]) -> Image.Image:
    """Recolour a one-colour mask."""
    out = Image.new("RGBA", im.size, (0, 0, 0, 0))
    for y in range(im.height):
        for x in range(im.width):
            if im.getpixel((x, y))[3]:
                out.putpixel((x, y), colour)
    return out


def glyph_name(ch: str) -> str | None:
    if ch == ".":
        return None
    return {"K": "viewer_digit_k", "M": "viewer_digit_m", "+": "viewer_digit_plus"}.get(ch, f"viewer_digit_{ch}")


def draw_number(img: Image.Image, atlas: Image.Image, regs: dict, text: str, x: int, y: int,
                colour: tuple[int, int, int, int]) -> int:
    """Left-aligned 5x7 digits, advance 6; '.' is a code-placed 1x1 rect on row 6, advance 2."""
    for ch in text:
        name = glyph_name(ch)
        if name is None:
            img.putpixel((x, y + 6), colour)
            x += ADVANCE["dot"]
        else:
            img.alpha_composite(tinted(piece(atlas, regs[name]), colour), (x, y))
            x += NUM_ADVANCE
    return x - 1  # last ink column is the glyph's own last column (K, M, + end 5 px wide)


def number_width(text: str) -> int:
    w = 0
    for ch in text:
        w += ADVANCE["dot"] if ch == "." else NUM_ADVANCE
    return w - 1


def build_composite(atlas: Image.Image, regs: dict, capture: Path, high_contrast: bool) -> Image.Image:
    base = Image.open(capture).convert("RGBA")
    # fit check: the viewer field must be pure plate_fill in the real capture
    fill = rgb("plate_fill")
    for _, (px, py) in PLATES.items():
        for yy in range(py + 3, py + 15):
            for xx in range(px + 80, px + 159):
                assert base.getpixel((xx, yy)) == fill, (capture.name, xx, yy)
    states = {
        "carl": ("999M+", rgb("amber"), "viewer_up", rgb("magenta_hi")),
        "donut": ("99.9K", rgb("text_white"), "viewer_down",
                  rgb("amber") if high_contrast else rgb("amber_dim")),
    }
    for hero, (px, py) in PLATES.items():
        text, num_colour, arrow, arrow_colour = states[hero]
        base.alpha_composite(piece(atlas, regs["viewer_eye"]), (px + EYE_AT[0], py + EYE_AT[1]))
        end = draw_number(base, atlas, regs, text, px + NUM_AT[0], py + NUM_AT[1], num_colour)
        assert end - px <= 150, (hero, end - px)  # reserved field ends at +150
        base.alpha_composite(tinted(piece(atlas, regs[arrow]), arrow_colour),
                             (px + TREND_AT[0], py + TREND_AT[1]))
    return base


def banner(img: Image.Image, text: str) -> Image.Image:
    """Append a label band below the image so the HUD pixel coordinates stay untouched."""
    out = Image.new("RGBA", (img.width, img.height + 14), (0, 0, 0, 255))
    out.alpha_composite(img)
    ImageDraw.Draw(out).text((4, img.height + 1), text, fill=(255, 194, 74, 255), font=ImageFont.load_default())
    return out


def contact_sheet(atlas: Image.Image, regs: dict, scale: int) -> Image.Image:
    names = [n for n in regs if n.startswith("viewer_")]
    pad, label_h = 8, 12
    cell_w = 9 * scale + pad + (24 if scale == 1 else 0)
    cols = 8
    rows = (len(names) + 1 + cols - 1) // cols
    cell_h = 7 * scale + label_h + pad
    sheet = Image.new("RGBA", (cols * cell_w + pad, rows * cell_h + pad + label_h), GROUND)
    d = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    d.text((pad // 2, 1), "viewer_* atlas regions (AUDIENCE-01 part A); last cell: code-drawn dot, not art",
           fill=(220, 220, 220, 255), font=font)
    items: list[tuple[str, Image.Image]] = []
    for n in names:
        im = piece(atlas, regs[n])
        # show state-coloured masks on the plate colour so the ink reads
        items.append((n.replace("viewer_", "").replace("digit_", "d_"), im))
    dot = Image.new("RGBA", (1, 7), (0, 0, 0, 0))
    dot.putpixel((0, 6), rgb("text_white"))
    items.append(("dot (code)", dot))
    plate = rgb("plate_fill")
    for i, (label, im) in enumerate(items):
        cx, cy = (i % cols) * cell_w + pad // 2, (i // cols) * cell_h + label_h + pad // 2
        d.rectangle([cx - 1, cy + label_h - 1, cx + 9 * scale, cy + label_h + 7 * scale], fill=plate)
        d.text((cx, cy), label, fill=(220, 220, 220, 255), font=font)
        sheet.alpha_composite(up(im, scale), (cx, cy + label_h))
    return sheet


def write_sums(out: Path) -> None:
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}" for p in sorted(out.glob("*.png"))]
    (out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    atlas = Image.open(ATLAS).convert("RGBA")
    regs = regions()
    contact_sheet(atlas, regs, 1).save(OUT / "contact_sheet_x1.png")
    contact_sheet(atlas, regs, 4).save(OUT / "contact_sheet_x4.png")
    for height in (360, 400):
        for mode, stem, high in (("normal", "normal", False), ("high-contrast", "low-hp-high-contrast", True)):
            capture = CAPTURES / f"hud-REAL-{stem}-640x{height}-1x.png"
            comp = build_composite(atlas, regs, capture, high)
            name = f"mock-composite-{height}-{mode}"
            banner(comp, f"{LABEL} | 640x{height} {mode} | real capture + viewer art at design-review coordinates").save(
                OUT / f"{name}-1x.png")
            crops = []
            for hero, (px, py) in PLATES.items():
                crops.append(up(comp.crop((px, py, px + 164, py + 48)), 4))
            strip = Image.new("RGBA", (crops[0].width * 2 + 16, crops[0].height), GROUND)
            strip.alpha_composite(crops[0], (0, 0))
            strip.alpha_composite(crops[1], (crops[0].width + 16, 0))
            banner(strip, f"{LABEL} | plates 4x (Carl 999M+, Donut 99.9K) | {mode}").save(OUT / f"{name}-plates-4x.png")
    write_sums(OUT)
    print(f"wrote evidence to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
