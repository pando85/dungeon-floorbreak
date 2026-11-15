#!/usr/bin/env python3
"""Deterministic 640x360 original-IP-neutral art preview, not a gameplay capture."""
from __future__ import annotations

import json
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / "generated/art"


def frame(name: str, number: int = 0) -> Image.Image:
    with Image.open(DEST / f"{name}.png") as atlas:
        rgba = atlas.convert("RGBA")
    data = json.loads((DEST / f"{name}.json").read_text(encoding="utf-8"))
    rect = data["frames"][number]["frame"]
    return rgba.crop((rect["x"], rect["y"],
                      rect["x"] + rect["w"], rect["y"] + rect["h"]))


def render() -> None:
    DEST.mkdir(parents=True, exist_ok=True)
    image = Image.new("RGBA", (640, 360), "#191a2c")
    d = ImageDraw.Draw(image)
    # Distance and midground are simple staging geometry. Gameplay tiles,
    # prop and actor below come from actual editable native ASE source.
    for i in range(7):
        x = 20 + i * 105
        d.rectangle((x, 55, x + 78, 304), fill="#25243c")
        d.rectangle((x + 10, 65, x + 67, 301), fill="#171a2b")
        d.arc((x + 8, 31, x + 69, 99), 180, 360, fill="#39334f", width=5)
        for y in (118, 162, 206, 250):
            d.line((x + 15, y, x + 64, y), fill="#2c2841", width=2)
    for x in (94, 288, 516):
        d.line((x, 0, x, 120), fill="#52445b", width=5)
        for y in range(0, 110, 12):
            d.rectangle((x - 1, y, x + 2, y + 5), fill="#81718b")
    d.ellipse((416, 59, 531, 174), outline="#8f437c", width=8)
    d.ellipse((428, 71, 519, 162), outline="#c457a2", width=3)
    d.rectangle((456, 104, 489, 125), fill="#b75eab")
    d.rectangle((461, 110, 469, 116), fill="#352139")
    d.rectangle((479, 110, 487, 116), fill="#352139")
    d.line((469, 126, 474, 133, 481, 126), fill="#b75eab", width=3)

    materials = frame("dungeon_materials")
    def tile(tx: int, ty: int, index: int) -> None:
        image.alpha_composite(
            materials.crop((index * 16, 0, (index + 1) * 16, 16)),
            (tx, ty),
        )

    for y in range(296, 360, 16):
        for x in range(0, 640, 16):
            tile(x, y, 0 if y == 296 else (0 if (x+y)//16 % 7 else 2))
    for x in range(144, 289, 16):
        tile(x, 248, 2)
    for x in range(305, 418, 16):
        tile(x, 232, 0)
    for x in range(513, 625, 16):
        tile(x, 256, 2)
    for y in range(248, 296, 16):
        for x in range(352, 400, 16):
            tile(x, y, 1)
    for y in range(248, 296, 16):
        tile(414, y, 2)
    for x in range(480, 513, 16):
        tile(x, 280, 3)
    image.alpha_composite(frame("neutral_worker", 1), (90, 244))
    image.alpha_composite(frame("fuse_barrel", 2), (323, 272))

    d = ImageDraw.Draw(image)
    d.rectangle((0, 0, 639, 24), fill="#101421")
    d.rectangle((0, 0, 5, 24), fill="#d6a66b")
    d.text((14, 7), "INSPECTION CORRIDOR / NATIVE ART PIPELINE TEST", fill="#ebdcce")
    d.text((14, 337), "TILESET 16px / ACTOR 52px / REAL ASE MASTER", fill="#c7b7ad")
    d.text((435, 337), "MOOD ONLY - NOT FINAL", fill="#d8a079")
    image.convert("RGB").save(DEST / "review_640x360.png", optimize=False)
    image.convert("RGB").resize(
        (1280, 720), Image.Resampling.NEAREST
    ).save(DEST / "review_2x.png", optimize=False)
    print("Contact sheets: generated/art/review_640x360.png and review_2x.png")


if __name__ == "__main__":
    render()
