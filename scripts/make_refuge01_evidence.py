#!/usr/bin/env python3
"""AREA-REFUGE-01 evidence assembly (execution tier).

Inputs: real Bevy captures of ``floor01_scene --scene refuge_guild`` (``scripts/refuge01_capture.sh``) in CAPTURES,
and the exported atlases (``bash scripts/export_assets.sh``). Writes under ``docs/evidence/area-refuge-01/``:
the empty-room contact sheet (1x and 2x, unlit and lit), the cast captures, the per-prop 1x/3x sheets, the hearth
burn GIF at 2x ``SHA256SUMS`` (evidence) and ``SOURCE_SHA256.txt`` (sources, manifests, kit, layout, room).

    python3 scripts/make_refuge01_evidence.py CAPTURES_DIR
"""
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/evidence/area-refuge-01"
BG = (28, 24, 34)
INK = (226, 214, 200)


def label(img: Image.Image, text: str, h: int = 14) -> Image.Image:
    out = Image.new("RGB", (img.width, img.height + h), BG)
    out.paste(img.convert("RGB"), (0, h))
    ImageDraw.Draw(out).text((3, 2), text, fill=INK)
    return out


def contact_sheet(caps: Path) -> Image.Image:
    tiles = {k: Image.open(caps / f"empty_{k}.png") for k in ("unlit_1x", "lit_1x", "unlit_2x", "lit_2x")}
    row1 = [label(tiles["unlit_1x"], "empty room, unlit, 1x (640x360)"), label(tiles["lit_1x"], "empty room, lit, 1x")]
    row2 = [label(tiles["unlit_2x"], "empty room, unlit, 2x (1280x720)"), label(tiles["lit_2x"], "empty room, lit, 2x")]
    gap = 8
    w = max(sum(t.width for t in row) + gap for row in (row1, row2))
    sheet = Image.new("RGB", (w, row1[0].height + row2[0].height + gap), BG)
    for row, y in ((row1, 0), (row2, row1[0].height + gap)):
        x = 0
        for t in row:
            sheet.paste(t, (x, y))
            x += t.width + gap
    return sheet


def prop_sheets() -> None:
    kit = json.loads((ROOT / "assets/source/terrain/floor01/floor01_refuge_kit.json").read_text())
    pieces = [p for p in kit["pieces"] if p["id"] not in ("refuge_hearth_flue_s1", "refuge_hearth_flue_s2",
                                                          "refuge_hearth_flue_s3")]
    atlases = {n: Image.open(ROOT / a["image"]).convert("RGBA") for n, a in kit["atlases"].items()}
    for k in (1, 3):
        cells = []
        for p in pieces:
            x, y, w, h = p["rect"]
            crop = atlases[p["atlas"]].crop((x, y, x + w, y + h)).resize((w * k, h * k), Image.NEAREST)
            cell = Image.new("RGBA", (crop.width + 8, crop.height + 22), BG + (255,))
            cell.alpha_composite(crop, (4, 18))
            ImageDraw.Draw(cell).text((4, 3), p["id"].replace("refuge_", ""), fill=INK)
            cells.append(cell)
        width = 1800 if k == 1 else 2600
        x = y = shelf = 0
        placed = []
        for c in cells:
            if x + c.width > width:
                x, y, shelf = 0, y + shelf, 0
            placed.append((c, x, y))
            x += c.width
            shelf = max(shelf, c.height)
        sheet = Image.new("RGB", (width, y + shelf), BG)
        for c, px, py in placed:
            sheet.paste(c.convert("RGB"), (px, py))
        sheet.save(OUT / f"refuge01_props_sheet_{k}x.png")


def hearth_gif() -> None:
    from scripts.pixelart.common import grid_images, load_grid_file
    imgs = grid_images(load_grid_file(ROOT / "assets/source/props/floor01/refuge_hearth.grid.json"))
    frames = []
    for im in imgs:
        f = Image.new("RGBA", (80, 80), BG + (255,))
        f.alpha_composite(im)
        frames.append(f.convert("RGB").resize((160, 160), Image.NEAREST))
    frames[0].save(OUT / "refuge01_hearth_burn_2x.gif", save_all=True, append_images=frames[1:], duration=120, loop=0)


def sums() -> None:
    names = {"SHA256SUMS", "SOURCE_SHA256.txt"}
    ev = sorted(p for p in OUT.rglob("*") if p.is_file() and p.name not in names)
    (OUT / "SHA256SUMS").write_text(
        "\n".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(OUT)}" for p in ev) + "\n",
        encoding="utf-8")
    files = sorted(ROOT.glob("assets/source/props/floor01/refuge_*.grid.json"))
    files += sorted(ROOT.glob("assets/source/props/floor01/refuge_*.ase"))
    files += sorted(ROOT.glob("assets/manifests/floor01_refuge_*.json"))
    files += [ROOT / "assets/source/terrain/floor01/floor01_refuge_kit.json",
              ROOT / "assets/levels/floor01/refuge_guild.json", ROOT / "assets/levels/floor01/refuge_guild.scene.json"]
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(ROOT)}" for p in files]
    (OUT / "SOURCE_SHA256.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")   # repo-relative paths


def main(caps: Path) -> None:
    sys.path.insert(0, str(ROOT))
    OUT.mkdir(parents=True, exist_ok=True)
    contact_sheet(caps).save(OUT / "refuge01_empty_room_contact_sheet.png")
    for src, dst in (("r360", "refuge01_room_cast_lit_640x360"), ("r400", "refuge01_room_cast_lit_640x400"),
                     ("r1080", "refuge01_room_cast_lit_1920x1080")):
        shutil.copy(caps / f"{src}.png", OUT / f"{dst}.png")
    prop_sheets()
    hearth_gif()
    sums()
    print("evidence written to", OUT.relative_to(ROOT))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
