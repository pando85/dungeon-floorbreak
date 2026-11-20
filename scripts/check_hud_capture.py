#!/usr/bin/env python3
"""Pixel-integrity check of a HUD-HYBRID-A-01 window capture.

Usage: check_hud_capture.py <png> <scale> <deck 0|1> [preview-name]

The capture is a real window screenshot of `floor01_scene` at `scale` times the
640 x 360 (or 640 x 400 with deck=1) virtual canvas. The HUD is drawn on whole
virtual pixels by a dedicated unlit camera, so inside its zones:

* every `scale` x `scale` block is one flat colour (integer nearest-neighbour
  magnification, no fractional offsets or smoothing),
* the plate, strip and notice text areas contain only HUD palette colours (no
  anti-aliased text, no lighting tint), the palette being read from
  `src/hud.rs`,
* the Jersey 10 names have exactly 10 px caps and the Silkscreen labels 5 px.

Portraits and the System face use their own sprite palettes and are excluded.
Exit status 0 on success, 1 on a failed check, 2 on bad usage.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
AMBER = (0xFF, 0xC2, 0x4A)
TEXT_WHITE = (0xF4, 0xE6, 0xD2)
# AUDIENCE-01: a spike turns the `text_white` digit masks amber by a GPU
# multiply (src/hud_draw.rs TEXT_WHITE_TO_AMBER), which renders as this colour
# (the mask red 244 cannot be raised). The code-drawn decimal dot uses it too.
VIEWER_AMBER = (0xF4, 0xC2, 0x4A)
# The two viewer colours made by a GPU multiply (the amber digits and the dim
# down arrow, `amber_dim`). Rasterisers round the linear multiply differently
# (a software Vulkan runner lands one level off a hardware GPU), so these two
# match within one level per channel; every other colour must be exact.
TINTED = (VIEWER_AMBER, (0xA8, 0x74, 0x2A))


def is_tinted(c: tuple[int, int, int]) -> bool:
    return any(all(abs(a - b) <= 1 for a, b in zip(c, t)) for t in TINTED)


def palette() -> set[tuple[int, int, int]]:
    source = (ROOT / "src/hud.rs").read_text(encoding="utf-8")
    block = source[source.index("pub mod palette"): source.index("// ----", source.index("pub mod palette"))]
    colours = set()
    for value in re.findall(r"Rgb::hex\(0x([0-9a-fA-F]{6})\)", block):
        number = int(value, 16)
        colours.add((number >> 16, number >> 8 & 0xFF, number & 0xFF))
    if len(colours) < 34:
        raise SystemExit(f"expected at least 34 HUD palette colours, found {len(colours)}")
    colours.add(VIEWER_AMBER)
    return colours


def main() -> int:
    if len(sys.argv) not in (4, 5):
        print(__doc__, file=sys.stderr)
        return 2
    path, scale, deck = Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3] == "1"
    preview = sys.argv[4] if len(sys.argv) == 5 else None
    view_h = 400 if deck else 360
    image = Image.open(path).convert("RGB")
    failures: list[str] = []
    if image.size != (640 * scale, view_h * scale):
        failures.append(f"{path}: size {image.size}, expected {(640 * scale, view_h * scale)}")
        return report(failures)
    px = image.load()
    colours = palette()

    def at(x: int, y: int) -> tuple[int, int, int]:
        return px[x * scale, y * scale]

    def zone(name: str, x0: int, y0: int, w: int, h: int, check_palette: bool) -> None:
        bad_block = bad_colour = 0
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w):
                first = at(x, y)
                if any(
                    px[x * scale + dx, y * scale + dy] != first
                    for dy in range(scale)
                    for dx in range(scale)
                ):
                    bad_block += 1
                if check_palette and first not in colours and not is_tinted(first):
                    bad_colour += 1
        if bad_block:
            failures.append(f"{name}: {bad_block} virtual pixels are not a flat {scale}x{scale} block")
        if bad_colour:
            failures.append(f"{name}: {bad_colour} pixels outside the HUD palette")

    # Plate rows right of the portrait well: name, HP and mana only palette
    # colours. The ability row also holds glyphs; a muted chip is tinted (not a
    # palette colour by design), so it only gets the block check below.
    zone("carl plate rows", 8 + 40, 8 + 3, 164 - 40 - 4, 31, True)
    zone("donut plate rows", 468 + 40, 8 + 3, 164 - 40 - 4, 31, True)
    # Whole plates (frame art included) must at least be integer blocks.
    zone("carl plate", 8, 8, 164, 48, False)
    zone("donut plate", 468, 8, 164, 48, False)
    # Notice panel and PREVIEW tag, when present (identified by their frames).
    notice_y = view_h - 40
    if at(404, notice_y) == (0x12, 0x0C, 0x18):
        zone("notice panel", 404, notice_y, 204, 32, True)
    if preview:
        zone("preview tag", 8, view_h - 19, 70, 11, True)
        if at(8, view_h - 19 + 5) != (0x12, 0x0C, 0x18):
            failures.append("preview tag missing in a preview capture")

    # Cap heights: amber rows of the first name (Jersey 10) and chip labels (Silkscreen 5).
    def rows_of(color, x0, x1, y0, y1):
        rows = [y for y in range(y0, y1) if any(at(x, y) == color for x in range(x0, x1))]
        return (min(rows), max(rows)) if rows else None

    name = rows_of(AMBER, 8 + 40, 8 + 40 + 30, 8 + 2, 8 + 15)
    if name is None or name[1] - name[0] + 1 != 10:
        failures.append(f"name cap height {name}, expected 10 px rows")
    label = rows_of(AMBER, 8 + 54, 8 + 54 + 28, 8 + 36, 8 + 44)
    if label is not None and label[1] - label[0] + 1 != 5:
        failures.append(f"label cap height {label}, expected 5 px rows")
    return report(failures, path)


def report(failures: list[str], path: Path | None = None) -> int:
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print(f"HUD_CAPTURE_OK: {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
