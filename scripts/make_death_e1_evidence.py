#!/usr/bin/env python3
"""DEATH-E1 evidence: real Bevy captures of the ten-goblin pileup at preset 4.

Captures (xvfb, the real render loop, lighting on, one frame per tick) the
`cp03_goblin_pileup` fixture at gore preset 4 (the default), 640x360 and
1920x1080, from tick 70 to tick 329, then builds:

  pileup_p4_<WxH>_60fps.mp4            every tick, 60 fps = real time
  sheet_pileup_p4_640x360.png          the six moments at native 640x360
  still_pileup_<moment>_1920x1080.png  the six moments, full 1920x1080
  sheet_pileup_p4_actors_1920x1080.png crops of the moments around Carl and Donut
  SHA256SUMS                           checksums of every file (README excluded)

Moments: impact (the three jab defeats), crush (the block lands on four
goblins), peak (the kick defeats on top of the crush: the most gore), settle
(ten settled corpses), retiring (the two oldest fade out: at most eight
settled corpses stay) and retired (they are gone, their stains stay).

Needs the release binary (`cargo build --locked --release --bin floor01_scene`),
`bash scripts/export_assets.sh` run once, xvfb-run, ffmpeg and Pillow. The
per-tick PNG frames stay in artifacts/death-e (git ignored). From the repository
root: `python3 scripts/make_death_e1_evidence.py [capture] [build]` (no argument
runs both). The capture run prints its final COMBAT_STATE; the script fails
unless it equals the headless replay of the same tick.
"""

from __future__ import annotations

import hashlib
import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "floor01_scene"
FRAMES = ROOT / "artifacts" / "death-e"
OUT = ROOT / "docs" / "evidence" / "death-e"
XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]

FIXTURE = "cp03_goblin_pileup"
FIRST, COUNT = 70, 260
RESOLUTIONS = {"640x360": 1, "1920x1080": 3}
# moment -> tick (jab defeats on 83, block lands and crushes on 139, kick
# defeats on 148; the pose and the first pieces appear the tick after).
MOMENTS = {
    "impact": 85,
    "crush": 141,
    "peak": 158,
    "settle": 265,
    "retiring": 283,
    "retired": 325,
}
# Crop of the 1920x1080 frame around the actors and the gore at their feet.
ACTOR_CROP = (480, 380, 1760, 1000)


def frames_dir(res: str) -> Path:
    return FRAMES / f"pileup_p4_{res}"


def combat_state(text: str) -> str:
    lines = [l for l in text.splitlines() if l.startswith("COMBAT_STATE")]
    return lines[-1] if lines else ""


def frame_of(res: str, tick: int) -> Path:
    return sorted(frames_dir(res).glob("frame_*.png"))[tick - FIRST]


def capture() -> None:
    last = FIRST + COUNT - 1
    headless = subprocess.run(
        [str(BIN), "--headless-floor01-smoke", "--fixture", FIXTURE, "--capture-tick", str(last)],
        check=True, capture_output=True, text=True, cwd=ROOT,
    ).stdout
    expected = combat_state(headless)
    for res, scale in RESOLUTIONS.items():
        target = frames_dir(res)
        shutil.rmtree(target, ignore_errors=True)
        log = subprocess.run(
            XVFB + [str(BIN), "--scale", str(scale), "--fixture", FIXTURE, "--gore-preset", "4",
                    "--capture-sequence", f"{FIRST}:1:{COUNT}", "--capture-dir", str(target)],
            check=True, capture_output=True, text=True, cwd=ROOT,
        ).stdout
        frames = sorted(target.glob("frame_*.png"))
        if len(frames) != COUNT:
            sys.exit(f"{target}: {len(frames)} frames, expected {COUNT}")
        w, h = res.split("x")
        if Image.open(frames[0]).size != (int(w), int(h)):
            sys.exit(f"{frames[0]} is not {res}")
        if combat_state(log) != expected:
            sys.exit(f"{target}: replay differs from headless\n{combat_state(log)}\n{expected}")
        print(f"captured {target.name}: {COUNT} frames, replay identical ({expected.split('hash=')[1][:16]})")


def label(img: Image.Image, text: str) -> Image.Image:
    bar = 18
    out = Image.new("RGB", (img.width, img.height + bar), (12, 10, 18))
    out.paste(img, (0, bar))
    font = ImageFont.load_default(size=13)
    ImageDraw.Draw(out).text((6, 2), text, fill=(235, 225, 200), font=font)
    return out


def grid(cells: list[Image.Image], columns: int) -> Image.Image:
    rows = [cells[i:i + columns] for i in range(0, len(cells), columns)]
    cw, ch = cells[0].size
    out = Image.new("RGB", (columns * cw + 4 * (columns - 1), len(rows) * ch + 4 * (len(rows) - 1)), (12, 10, 18))
    for r, row in enumerate(rows):
        for c, cell in enumerate(row):
            out.paste(cell, (c * (cw + 4), r * (ch + 4)))
    return out


def build() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for res in RESOLUTIONS:
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-framerate", "60", "-pattern_type", "glob",
             "-i", str(frames_dir(res) / "frame_*.png"),
             "-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p",
             "-movflags", "+faststart", str(OUT / f"pileup_p4_{res}_60fps.mp4")],
            check=True,
        )
    small = [label(Image.open(frame_of("640x360", t)).convert("RGB"), f"{name}  tick {t}  preset 4  640x360")
             for name, t in MOMENTS.items()]
    grid(small, 2).save(OUT / "sheet_pileup_p4_640x360.png", optimize=True)
    crops = []
    for name, t in MOMENTS.items():
        full = Image.open(frame_of("1920x1080", t)).convert("RGB")
        full.save(OUT / f"still_pileup_{name}_1920x1080.png", optimize=True)
        crops.append(label(full.crop(ACTOR_CROP), f"{name}  tick {t}  preset 4  1920x1080 crop"))
    grid(crops, 2).save(OUT / "sheet_pileup_p4_actors_1920x1080.png", optimize=True)
    write_sums()


def write_sums() -> None:
    lines = []
    for path in sorted(OUT.iterdir()):
        if path.name in ("README.md", "SHA256SUMS") or not path.is_file():
            continue
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n")


def main() -> None:
    steps = sys.argv[1:] or ["capture", "build"]
    for step in steps:
        {"capture": capture, "build": build}[step]()


if __name__ == "__main__":
    main()
