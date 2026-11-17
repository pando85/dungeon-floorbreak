#!/usr/bin/env python3
"""ART-INT-02 evidence: GIFs, per-tick strips and stills of the real art in the Bevy scene.

Inputs (produced by `xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render`,
real Bevy 1920x1080 captures, one per tick, lighting on):

  artifacts/cp01-feel/frames_<fixture>/frame_NNNN_tick_TTTTT.png

and the `floor01_scene` debug binary (`cargo build --locked --bin floor01_scene`) for the
per-tick animation traces (`--anim-trace`: the frame drawn next to the sim state it follows).

Outputs in docs/evidence/art-int-02 (Pillow and matplotlib's font finder only):

  <fixture>_ticks<a>-<b>_960x540.gif   native-pixel 960x540 crop of the 1920x1080 captures,
                                       one frame per tick, 50 ms per tick (3x slower than real
                                       time), captioned with the drawn frames
  strip_haymaker_kill_ticks150-161.png every tick around the killing hit (hit-stop, recoil,
                                       launch frame)
  strip_goblin_swing_hits_carl_ticks149-160.png  the goblin's smear, impact and Carl's flinch
  still_*.png                          full 1920x1080 captures of key ticks
  still_goblins_A_B_with_carl_donut_x2.png  variant A and B next to Carl and Donut (crop, 2x)
  SHA256SUMS                           checksums of every file in the folder (README excluded)

The committed `anim_trace_<fixture>.txt` files are written by CI's headless step, not here.
Run from the repository root: `python3 scripts/make_art_int02_evidence.py`.
"""
from __future__ import annotations

import hashlib
import re
import shutil
import subprocess
import sys
from pathlib import Path

from matplotlib import font_manager
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "debug" / "floor01_scene"
FRAMES = ROOT / "artifacts" / "cp01-feel"
OUT = ROOT / "docs" / "evidence" / "art-int-02"

# fixture -> (first tick, frame count) of the real capture sequences
# (scripts/ci_floor01_combat.sh)
GIFS = {
    "cp01_haymaker_chain": (128, 61),
    "cp01_goblin_attacks": (128, 61),
    "cp01_interrupt": (122, 61),
    "cp02_two_goblins": (60, 61),
}
CROP = (450, 440, 1410, 980)
STRIP_CROP = (700, 600, 1260, 900)
GIF_MS_PER_TICK = 50


def font(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(font_manager.findfont("DejaVu Sans"), size)


def anim_trace(fixture: str) -> dict[int, dict[str, str]]:
    out = subprocess.run(
        [str(BIN), "--headless-floor01-smoke", "--fixture", fixture, "--anim-trace"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    rows = {}
    for line in out.splitlines():
        if line.startswith("COMBAT_ANIM_TRACE"):
            row = dict(token.split("=", 1) for token in line.split()[1:])
            rows[int(row["tick"])] = row
    return rows


def frame_files(fixture: str) -> list[tuple[int, Path]]:
    folder = FRAMES / f"frames_{fixture}"
    found = [(int(re.search(r"tick_(\d+)", p.name).group(1)), p)
             for p in sorted(folder.glob("frame_*_tick_*.png"))]
    if not found:
        sys.exit(f"no frames in {folder}: run scripts/ci_floor01_combat.sh render first")
    return found


def goblin_labels(row: dict[str, str]) -> str:
    labels = []
    for index in range(4):
        draw = row.get(f"g{index}_draw")
        if draw is None:
            break
        labels.append(f"g{index} {draw}")
    return "   ".join(labels)


def caption(row: dict[str, str]) -> str:
    move = row["carl_move"]
    frozen = "  FROZEN" if move.endswith("!") else ""
    return f"tick {row['tick']:>3}   Carl {row['carl_draw']} ({move}){frozen}   {goblin_labels(row)}"


def make_gif(fixture: str, rows: dict[int, dict[str, str]]) -> Path:
    frames = frame_files(fixture)
    small = font(19)
    images = []
    for tick, path in frames:
        image = Image.open(path).convert("RGB").crop(CROP)
        draw = ImageDraw.Draw(image, "RGBA")
        draw.rectangle((0, 0, image.width, 30), fill=(0, 0, 0, 175))
        draw.text((10, 4), caption(rows[tick]), font=small, fill=(255, 255, 255, 255))
        images.append(image.quantize(colors=128, method=Image.Quantize.MEDIANCUT,
                                     dither=Image.Dither.NONE))
    first, last = frames[0][0], frames[-1][0]
    target = OUT / f"{fixture}_ticks{first}-{last}_960x540.gif"
    images[0].save(target, save_all=True, append_images=images[1:], duration=GIF_MS_PER_TICK,
                   loop=0, optimize=True)
    return target


def make_strip(fixture: str, ticks: range, name: str, rows: dict[int, dict[str, str]],
               cols: int = 4) -> Path:
    frames = dict(frame_files(fixture))
    cell_w, cell_h = STRIP_CROP[2] - STRIP_CROP[0], STRIP_CROP[3] - STRIP_CROP[1]
    label_h = 92
    tick_list = list(ticks)
    rows_n = (len(tick_list) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * cell_w, rows_n * (cell_h + label_h)), (16, 16, 20))
    draw = ImageDraw.Draw(sheet)
    big, small = font(24), font(17)
    for index, tick in enumerate(tick_list):
        x, y = (index % cols) * cell_w, (index // cols) * (cell_h + label_h)
        sheet.paste(Image.open(frames[tick]).convert("RGB").crop(STRIP_CROP), (x, y + label_h))
        row = rows[tick]
        frozen = row["carl_move"].endswith("!") or row["carl_stop"] != "0"
        draw.text((x + 8, y + 4), f"tick {tick}", font=big, fill=(255, 255, 255))
        draw.text((x + 8, y + 36), f"Carl {row['carl_draw']}  move {row['carl_move']}", font=small,
                  fill=(255, 120, 120) if frozen else (205, 205, 205))
        draw.text((x + 8, y + 62), f"goblin {row['g0_draw']}  hit-stop {row['g0_hs']}", font=small,
                  fill=(255, 220, 120))
    target = OUT / name
    sheet.save(target)
    return target


def copy_still(fixture: str, tick: int, name: str) -> Path:
    frames = dict(frame_files(fixture))
    target = OUT / f"still_{name}_tick{tick}_1920x1080.png"
    shutil.copyfile(frames[tick], target)
    return target


def make_ab_still(rows: dict[int, dict[str, str]], tick: int) -> Path:
    """Variant A (goblin 0) and B (goblin 1) next to Carl and Donut, 2x crop."""
    frames = dict(frame_files("cp02_two_goblins"))
    crop = (640, 640, 1280, 940)  # 640x300 native pixels
    image = Image.open(frames[tick]).convert("RGB").crop(crop)
    image = image.resize((crop[2] - crop[0], crop[3] - crop[1]), Image.Resampling.NEAREST)
    image = image.resize((image.width * 2, image.height * 2), Image.Resampling.NEAREST)
    draw = ImageDraw.Draw(image, "RGBA")
    draw.rectangle((0, 0, image.width, 34), fill=(0, 0, 0, 175))
    row = rows[tick]
    draw.text((10, 5), f"tick {tick}: Donut and Carl (left), goblin A (g0) and goblin B (g1) "
              f"| {goblin_labels(row)}", font=font(19), fill=(255, 255, 255, 255))
    target = OUT / "still_goblins_A_B_with_carl_donut_x2.png"
    image.save(target)
    return target


def write_sums() -> None:
    lines = []
    for path in sorted(OUT.iterdir()):
        if path.name in ("README.md", "SHA256SUMS") or not path.is_file():
            continue
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    traces = {f: anim_trace(f) for f in GIFS}
    for fixture in GIFS:
        print("gif", make_gif(fixture, traces[fixture]).name)
    print("strip", make_strip("cp01_haymaker_chain", range(150, 162),
                              "strip_haymaker_kill_ticks150-161.png",
                              traces["cp01_haymaker_chain"]).name)
    print("strip", make_strip("cp01_goblin_attacks", range(149, 161),
                              "strip_goblin_swing_hits_carl_ticks149-160.png",
                              traces["cp01_goblin_attacks"]).name)
    for fixture, tick, name in [
        ("cp01_haymaker_chain", 140, "jab_contact_a1"),
        ("cp01_haymaker_chain", 152, "haymaker_kill_contact_b1_recoil"),
        ("cp01_haymaker_chain", 168, "kill_flight_airborne_frame"),
        ("cp01_goblin_attacks", 140, "goblin_tell"),
        ("cp01_goblin_attacks", 153, "goblin_impact_on_carl"),
        ("cp01_interrupt", 143, "jab_interrupts_the_tell_recoil_held"),
    ]:
        print("still", copy_still(fixture, tick, name).name)
    print("still", make_ab_still(traces["cp02_two_goblins"], 76).name)
    write_sums()
    return 0


if __name__ == "__main__":
    sys.exit(main())
