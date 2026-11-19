#!/usr/bin/env python3
"""HUD-DEBT-06 evidence: 1x captures and a 2x contact sheet of the accessibility settings.

Usage: make_hud_a11y_evidence.py [captures-dir] [evidence-dir]

Reads the real window captures `artifacts/hud/hud-<state>-<size>.png` written by
`scripts/ci_floor01_render_smoke.sh hud` and `hud-deck` (2x), reduces them to
1x by integer nearest-neighbour (`check_hud_capture.py` proves every 2x2 block
is flat, so nothing is lost), and tiles the top HUD band (virtual rows 0 to 59,
full width) of each default/setting pair at 2x with a label. Appends the new
files to the evidence `SHA256SUMS`.
"""
from __future__ import annotations

import hashlib
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
# (capture state, evidence name, label)
ONE_X = [
    ("depleted-high-contrast", "hud-PREVIEW-depleted-high-contrast"),
    ("depleted-high-contrast-reduced-motion", "hud-PREVIEW-depleted-high-contrast-reduced-motion"),
    ("real-hurt-reduced-motion", "hud-REAL-hurt-reduced-motion"),
    ("real-low-hp-high-contrast", "hud-REAL-low-hp-high-contrast"),
]
SHEET = [
    ("real-hurt", "REAL hurt flash: default"),
    ("real-hurt-reduced-motion", "REAL hurt flash: --hud-reduced-motion (hatched, no flash)"),
    ("real-low-hp", "REAL Carl at 1 HP: default"),
    ("real-low-hp-high-contrast", "REAL Carl at 1 HP: --hud-high-contrast"),
    ("depleted", "PREVIEW depleted: default"),
    ("depleted-high-contrast", "PREVIEW depleted: --hud-high-contrast"),
    ("depleted-high-contrast-reduced-motion", "PREVIEW depleted: --hud-high-contrast --hud-reduced-motion"),
]


def main() -> int:
    caps = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "artifacts/hud"
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "docs/evidence/hud-hybrid-a"
    for size, view_h in (("640x360", 360), ("640x400", 400)):
        def load(state: str) -> Image.Image:
            im = Image.open(caps / f"hud-{state}-{size}.png").convert("RGB")
            assert im.size == (1280, view_h * 2), (state, im.size)
            return im

        for state, name in ONE_X:
            load(state).resize((640, view_h), Image.NEAREST).save(out / f"{name}-{size}-1x.png")
        font, label_h, band = ImageFont.load_default(), 14, 120
        sheet = Image.new("RGB", (1280, len(SHEET) * (band + label_h)), (0x26, 0x26, 0x30))
        draw = ImageDraw.Draw(sheet)
        for i, (state, label) in enumerate(SHEET):
            y = i * (band + label_h)
            draw.text((4, y + 1), label, fill=(220, 220, 220), font=font)
            sheet.paste(load(state).crop((0, 0, 1280, band)), (0, y + label_h))
        sheet.save(out / f"hud-contact-sheet-accessibility-{size}-2x.png")
    sums = {}
    for line in (out / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
        digest, name = line.split("  ", 1)
        sums[name] = digest
    for path in out.glob("*.png"):
        sums[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    (out / "SHA256SUMS").write_text(
        "".join(f"{d}  {n}\n" for n, d in sorted(sums.items(), key=lambda kv: kv[0].lower())), encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
