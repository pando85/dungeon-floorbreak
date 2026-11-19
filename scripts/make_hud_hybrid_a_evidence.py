#!/usr/bin/env python3
"""HUD-HYBRID-A-01 evidence: contact sheets, portrait close-ups and the silhouette-gate pack.

Reads the exported atlas (generated/art/hud_hybrid_a.png, from scripts/export_assets.sh)
and the committed manifest; writes into docs/evidence/hud-hybrid-a-01/ (default) and an
SHA256SUMS file. Output is deterministic (nearest-neighbour scaling, fixed layout).

Images written:
  contact_sheet_x1.png, contact_sheet_x4.png   every atlas region on a neutral ground, labelled
  portraits_x8.png                              Carl, Donut portraits and the 3 CRT frames at 8x
  gate_carl.png, gate_donut.png, gate_crt.png   silhouette-gate pack: source vs HUD asset
  gate_scene_640x360_x1.png / _x3.png           plates, portraits and CRT composed on the lit
                                                Floor 01 scene frame (atlas pieces only; bars,
                                                text and chips are code-drawn and not shown)
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import pixel_grid_to_ase as pg  # noqa: E402

MANIFEST = ROOT / "assets/manifests/hud_hybrid_a_v1.json"
ATLAS = ROOT / "generated/art/hud_hybrid_a.png"
SCENE = ROOT / "docs/evidence/floor01-scene/floor01-scene-advanced-640x360.png"
GROUND = (0x26, 0x26, 0x30, 255)
WELL = {"portrait_carl": (0x2b, 0x20, 0x38, 255), "portrait_donut": (0x30, 0x1a, 0x2e, 255)}


def up(im: Image.Image, n: int) -> Image.Image:
    return im.resize((im.width * n, im.height * n), Image.NEAREST)


def regions() -> dict[str, tuple[int, int, int, int]]:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {r["name"]: tuple(r["rect"]) for r in data["hud"]["regions"]}


def piece(atlas: Image.Image, rect) -> Image.Image:
    x, y, w, h = rect
    return atlas.crop((x, y, x + w, y + h))


def master_frame(grid_path: Path) -> Image.Image:
    g = pg.validate_grid(json.loads(grid_path.read_text(encoding="utf-8")))
    pal = {e["key"]: pg.parse_rgba(e["rgba"]) for e in g["palette"]}
    flat = pg.composite_keys(g, 0)
    im = Image.new("RGBA", (g["canvas"][0], g["canvas"][1]), (0, 0, 0, 0))
    for y, row in enumerate(flat):
        for x, ch in enumerate(row):
            if ch != ".":
                im.putpixel((x, y), pal[ch])
    return im


def on_ground(im: Image.Image, ground=GROUND) -> Image.Image:
    base = Image.new("RGBA", im.size, ground)
    base.alpha_composite(im)
    return base


def label_font() -> ImageFont.ImageFont:
    return ImageFont.load_default()


def contact_sheet(atlas: Image.Image, regs: dict, scale: int) -> Image.Image:
    pad, label_h = 6 * scale if scale == 1 else 8, 12
    cols = 6 if scale > 1 else 8
    items = list(regs.items())
    cell_w = max(r[2] for r in regs.values() if r[2] <= 40) * scale + pad
    big = [(n, r) for n, r in items if r[2] > 40]
    small = [(n, r) for n, r in items if r[2] <= 40]
    font = label_font()
    rows_small = (len(small) + cols - 1) // cols
    cell_h = max(r[3] for _, r in small) * scale + label_h + pad
    width = max(cols * cell_w, max(r[2] for _, r in big) * scale + 2 * pad)
    height = rows_small * cell_h + sum(r[3] * scale + label_h + pad for _, r in big) + pad
    sheet = Image.new("RGBA", (width, height), GROUND)
    d = ImageDraw.Draw(sheet)
    y = pad // 2
    for n, r in big:
        sheet.alpha_composite(up(piece(atlas, r), scale), (pad, y + label_h))
        d.text((pad, y), f"{n} {r[2]}x{r[3]}", fill=(220, 220, 220, 255), font=font)
        y += r[3] * scale + label_h + pad
    for i, (n, r) in enumerate(small):
        cx, cy = (i % cols) * cell_w + pad // 2, y + (i // cols) * cell_h
        d.text((cx, cy), n, fill=(220, 220, 220, 255), font=font)
        sheet.alpha_composite(up(piece(atlas, r), scale), (cx, cy + label_h))
    return sheet


def with_well(atlas: Image.Image, rect, name: str) -> Image.Image:
    im = piece(atlas, rect)
    return on_ground(im, WELL.get(name, GROUND))


def side_by_side(images: list[Image.Image], gap: int = 16, ground=GROUND) -> Image.Image:
    w = sum(i.width for i in images) + gap * (len(images) + 1)
    h = max(i.height for i in images) + 2 * gap
    out = Image.new("RGBA", (w, h), ground)
    x = gap
    for im in images:
        out.alpha_composite(im, (x, (h - im.height) // 2))
        x += im.width + gap
    return out


def plate_composite(atlas: Image.Image, regs, portrait: str, tag: str) -> Image.Image:
    plate = Image.new("RGBA", (164, 48), (0, 0, 0, 0))
    plate.alpha_composite(piece(atlas, regs["plate_frame"]))
    well = Image.new("RGBA", (28, 28), WELL[portrait])
    well.alpha_composite(piece(atlas, regs[portrait]))
    base = Image.new("RGBA", (164, 48), (0, 0, 0, 0))
    base.alpha_composite(plate)
    base.alpha_composite(well, (6, 6))
    base.alpha_composite(piece(atlas, regs[tag]), (4, 36))
    return base


def scene_composite(atlas: Image.Image, regs) -> Image.Image:
    scene = Image.open(SCENE).convert("RGBA")
    if scene.size == (1280, 720):  # the committed capture is the 2x render of the 640x360 view
        scene = scene.resize((640, 360), Image.NEAREST)
    assert scene.size == (640, 360), scene.size
    scene.alpha_composite(plate_composite(atlas, regs, "portrait_carl", "tag_p1"), (8, 8))
    scene.alpha_composite(plate_composite(atlas, regs, "portrait_donut", "tag_ai"), (468, 8))
    scene.alpha_composite(piece(atlas, regs["crt_idle"]), (612, 334))
    return scene


def write_sums(out: Path) -> None:
    lines = []
    for p in sorted(out.glob("*.png")):
        lines.append(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}")
    (out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, default=ROOT / "docs/evidence/hud-hybrid-a-01")
    args = ap.parse_args(argv)
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    atlas = Image.open(ATLAS).convert("RGBA")
    regs = regions()

    contact_sheet(atlas, regs, 1).save(out / "contact_sheet_x1.png")
    contact_sheet(atlas, regs, 4).save(out / "contact_sheet_x4.png")

    crt = [on_ground(piece(atlas, regs[n])) for n in ("crt_idle", "crt_blink", "crt_notice")]
    portraits = [with_well(atlas, regs["portrait_carl"], "portrait_carl"),
                 with_well(atlas, regs["portrait_donut"], "portrait_donut")]
    side_by_side([up(i, 8) for i in portraits + crt]).save(out / "portraits_x8.png")

    # silhouette gate pack: sprite master next to the HUD portrait, 1x and 4x / 8x
    carl_master = master_frame(ROOT / "assets/source/characters/carl/carl_h52.grid.json")
    donut_master = master_frame(ROOT / "assets/source/characters/donut/donut_h40.grid.json")
    for tag, master, name, key in (("carl", carl_master, "portrait_carl", "portrait_carl"),
                                    ("donut", donut_master, "portrait_donut", "portrait_donut")):
        p = with_well(atlas, regs[name], key)
        m = on_ground(master)
        side_by_side([m, p, up(m, 4), up(p, 8)]).save(out / f"gate_{tag}.png")
    key_crt = master_frame(ROOT / "assets/source/hud/keys/system_crt_face.grid.json")
    frames = [on_ground(piece(atlas, regs[n])) for n in ("crt_idle", "crt_blink", "crt_notice")]
    side_by_side(frames + [up(f, 8) for f in frames]
                 + [up(on_ground(key_crt), 8)]).save(out / "gate_crt.png")

    scene = scene_composite(atlas, regs)
    scene.save(out / "gate_scene_640x360_x1.png")
    up(scene, 3).save(out / "gate_scene_640x360_x3.png")
    write_sums(out)
    print(f"wrote evidence to {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
