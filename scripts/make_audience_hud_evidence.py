#!/usr/bin/env python3
"""AUDIENCE-01 part C evidence: native-scale 1x captures, contact sheets and the mock comparison.

Usage: make_audience_hud_evidence.py [captures-dir] [evidence-dir]

Reads the real window captures `artifacts/audience/audience-<state>-<size>.png`
written by `scripts/ci_floor01_render_smoke.sh audience` and `audience-deck`
(2x), reduces them to 1x by integer nearest-neighbour (`check_hud_capture.py`
proves every 2x2 block is flat, so nothing is lost), and writes into the
evidence directory (default docs/evidence/audience-01):

  hud-<REAL|PREVIEW>-viewers-<state>-<size>-1x.png   the native 640x360 / 640x400 frames
  contact-sheet-<kind>-<size>-2x.png                  top HUD band (rows 0..59) of each state, 2x, labelled
  compare-mock-<size>-<mode>.txt                      per-pixel comparison of the plate readouts with the part A mocks

PREVIEW states are `--hud-preview viewers*` fixtures; REAL states run the QA
probe (`--audience-probe`) on real fixtures. It then rewrites `SHA256SUMS`
for every PNG of the directory (the part A script does the same).
"""
from __future__ import annotations

import hashlib
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent

# (capture state, evidence kind, evidence state name, label)
STATES = [
    ("locked", "REAL", "locked", "REAL locked (Floor 01 default): no viewer art"),
    ("normal", "PREVIEW", "normal", "PREVIEW viewers: 6.8K and 12.4K, steady"),
    ("rising", "PREVIEW", "rising", "PREVIEW viewers-rising: 38.4K and 87.1K, up arrows, amber"),
    ("falling", "PREVIEW", "falling", "PREVIEW viewers-falling: 6.8K and 12.4K, down arrows"),
    ("max", "PREVIEW", "max", "PREVIEW viewers-max: 999M+ up, 99.9K down (widest strings)"),
    ("normal-high-contrast", "PREVIEW", "normal-high-contrast", "PREVIEW viewers: --hud-high-contrast"),
    ("rising-high-contrast", "PREVIEW", "rising-high-contrast", "PREVIEW viewers-rising: --hud-high-contrast"),
    ("falling-high-contrast", "PREVIEW", "falling-high-contrast", "PREVIEW viewers-falling: --hud-high-contrast (amber arrow)"),
    ("max-high-contrast", "PREVIEW", "max-high-contrast", "PREVIEW viewers-max: --hud-high-contrast"),
    ("rising-reduced-motion", "PREVIEW", "rising-reduced-motion", "PREVIEW viewers-rising: --hud-reduced-motion"),
    ("real-donut-kill", "REAL", "donut-kill", "REAL probe: Donut pounce kill, her count 370, Carl 250"),
    ("real-spectacular", "REAL", "spectacular", "REAL probe: shore block kill, Carl 2.0K spike"),
    ("real-spectacular-high-contrast", "REAL", "spectacular-high-contrast", "REAL probe: shore block kill, --hud-high-contrast"),
    ("real-anticipation", "REAL", "anticipation", "REAL probe: authored cue, both heroes 4.2K, no action on screen"),
    ("real-anticipation-high-contrast", "REAL", "anticipation-high-contrast", "REAL probe: authored cue, --hud-high-contrast"),
    ("real-falling", "REAL", "falling", "REAL probe: the shore kill count decaying, down arrow"),
    ("real-falling-high-contrast", "REAL", "falling-high-contrast", "REAL probe: decay, --hud-high-contrast (amber arrow)"),
    ("real-boss", "REAL", "boss", "REAL probe: THE JUICER boss strip with both readouts"),
    ("real-post-mana", "REAL", "post-mana", "REAL probe: Donut after the biscuit (mana bar) with her readout"),
    ("real-post-mana-high-contrast", "REAL", "post-mana-high-contrast", "REAL probe: post-mana, --hud-high-contrast"),
]
PLATES = {"carl": (8, 8), "donut": (468, 8)}
VIEWER_FIELD = (80, 3, 79, 12)  # plate-relative x, y, w, h (the F16 clear rect)


def load(caps: Path, state: str, size: str, view_h: int) -> Image.Image:
    im = Image.open(caps / f"audience-{state}-{size}.png").convert("RGB")
    assert im.size == (1280, view_h * 2), (state, im.size)
    return im


def reduce(im: Image.Image, view_h: int) -> Image.Image:
    # Integer nearest-neighbour: take the top-left pixel of every 2x2 block.
    return im.resize((640, view_h), Image.NEAREST)


def compare_with_mock(one_x: Image.Image, mock_path: Path, label: str) -> str:
    """Per-pixel difference of the viewer field of both plates against a part A mock."""
    mock = Image.open(mock_path).convert("RGB")
    lines = [f"{label}", f"mock: {mock_path.name}"]
    total = 0
    for hero, (px, py) in PLATES.items():
        x0, y0, w, h = VIEWER_FIELD
        diffs = []
        for y in range(py + y0, py + y0 + h):
            for x in range(px + x0, px + x0 + w):
                a, b = one_x.getpixel((x, y)), mock.getpixel((x, y))
                if a != b:
                    diffs.append(((x - px, y - py), a, b))
        total += len(diffs)
        lines.append(f"{hero}: {len(diffs)} differing pixels in the viewer field (plate-relative x {x0}..{x0 + w - 1}, y {y0}..{y0 + h - 1})")
        for (x, y), a, b in diffs[:12]:
            lines.append(f"  ({x},{y}) runtime {a} mock {b}")
        if len(diffs) > 12:
            lines.append(f"  ... {len(diffs) - 12} more")
    lines.append(f"total differing pixels: {total}")
    return "\n".join(lines) + "\n"


def main() -> int:
    caps = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "artifacts/audience"
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "docs/evidence/audience-01"
    out.mkdir(parents=True, exist_ok=True)
    font = ImageFont.load_default()
    for size, view_h in (("640x360", 360), ("640x400", 400)):
        ones = {}
        for state, kind, name, _ in STATES:
            one = reduce(load(caps, state, size, view_h), view_h)
            ones[state] = one
            one.save(out / f"hud-{kind}-viewers-{name}-{size}-1x.png")
        for kind in ("PREVIEW", "REAL"):
            rows = [(s, label) for s, k, _, label in STATES if k == kind]
            band, label_h = 60, 14
            sheet = Image.new("RGB", (1280, len(rows) * (band * 2 + label_h)), (0x26, 0x26, 0x30))
            draw = ImageDraw.Draw(sheet)
            for i, (state, label) in enumerate(rows):
                y = i * (band * 2 + label_h)
                draw.text((4, y + 1), label, fill=(220, 220, 220), font=font)
                sheet.paste(load(caps, state, size, view_h).crop((0, 0, 1280, band * 2)), (0, y + label_h))
            sheet.save(out / f"contact-sheet-{kind.lower()}-{size}-2x.png")
        # The widest-string preview must equal the part A mock (Carl 999M+ up
        # amber, Donut 99.9K down) except where the runtime's GPU tint differs.
        for mode, state in (("normal", "max"), ("high-contrast", "max-high-contrast")):
            mock = out / f"mock-composite-{view_h}-{mode}-1x.png"
            if mock.is_file():
                text = compare_with_mock(
                    ones[state], mock,
                    f"runtime PREVIEW viewers-max {mode} at 640x{view_h} versus the part A mock composite",
                )
                (out / f"compare-mock-{size}-{mode}.txt").write_text(text, encoding="utf-8")
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}" for p in sorted(out.glob("*.png"))]
    (out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote evidence to {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
