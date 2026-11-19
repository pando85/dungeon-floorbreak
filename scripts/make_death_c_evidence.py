#!/usr/bin/env python3
"""DEATH-C evidence: real Bevy captures of the cosmetic death presenter.

Captures (xvfb, the real render loop, lighting on, one frame per tick = full
speed) the goblin crush (`cp03_shore_kill`) and the kick kill (`cp01_kick`) at
gore preset 4 (the default) and preset 1, at 640x360 and 1920x1080, then builds:

  <scenario>_p<preset>_<WxH>_60fps.mp4       every tick, 60 fps = real time
  <scenario>_p1_vs_p4_640x360_30fps.gif      left preset 1, right preset 4, every
                                             second tick at 33 ms = real time
  sheet_<scenario>_p4_vs_p1.png              1920x1080 crops at key ticks (left 4,
                                             right 1)
  still_*.png                                full 1920x1080 captures of key ticks
  perf_frame_times.txt                       frame times with the pools at max load
  SHA256SUMS                                 checksums of every file (README excluded)

Needs the debug binary (`cargo build --locked --bin floor01_scene`), xvfb-run,
ffmpeg and Pillow. The per-tick PNG frames stay in artifacts/death-c (git
ignored; the MP4s hold every one of them). Run from the repository root:
`python3 scripts/make_death_c_evidence.py [capture] [build] [perf]` (no argument
runs all three).
"""

from __future__ import annotations

import hashlib
import re
import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "debug" / "floor01_scene"
FRAMES = ROOT / "artifacts" / "death-c"
OUT = ROOT / "docs" / "evidence" / "death-c"

XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]

# name -> (fixture, first tick, frame count, tick of the defeat event)
SCENARIOS = {
    "crush": ("cp03_shore_kill", 138, 100, 139),
    "kick": ("cp01_kick", 256, 85, 260),
}
PRESETS = (4, 1)
RESOLUTIONS = {"640x360": 1, "1920x1080": 3}
# Key ticks (relative to the defeat tick) of the contact sheets.
SHEET_OFFSETS = {
    "crush": (-1, 1, 4, 9, 21, 61, 90),
    "kick": (0, 6, 14, 26, 44, 70),
}
SHEET_CROP = (640, 540, 1760, 960)
STILLS = {
    "still_crush_p4_L+1_1920x1080.png": ("crush", 4, 1),
    "still_crush_p1_L+1_1920x1080.png": ("crush", 1, 1),
    "still_crush_p4_L+90_1920x1080.png": ("crush", 4, 90),
    "still_kick_p4_K+14_1920x1080.png": ("kick", 4, 14),
}


def frames_dir(name: str, preset: int, res: str) -> Path:
    return FRAMES / f"{name}_p{preset}_{res}"


def combat_state(text: str) -> str:
    lines = [l for l in text.splitlines() if l.startswith("COMBAT_STATE")]
    return lines[-1] if lines else ""


def capture() -> None:
    for name, (fixture, first, count, _) in SCENARIOS.items():
        last = first + count - 1
        headless = subprocess.run(
            [str(BIN), "--headless-floor01-smoke", "--fixture", fixture, "--capture-tick", str(last)],
            check=True, capture_output=True, text=True, cwd=ROOT,
        ).stdout
        expected = combat_state(headless)
        for res, scale in RESOLUTIONS.items():
            for preset in PRESETS:
                target = frames_dir(name, preset, res)
                shutil.rmtree(target, ignore_errors=True)
                log = subprocess.run(
                    XVFB + [str(BIN), "--scale", str(scale), "--fixture", fixture,
                            "--gore-preset", str(preset),
                            "--capture-sequence", f"{first}:1:{count}", "--capture-dir", str(target)],
                    check=True, capture_output=True, text=True, cwd=ROOT,
                ).stdout
                frames = sorted(target.glob("frame_*.png"))
                if len(frames) != count:
                    sys.exit(f"{target}: {len(frames)} frames, expected {count}")
                w, h = res.split("x")
                if Image.open(frames[0]).size != (int(w), int(h)):
                    sys.exit(f"{frames[0]} is not {res}")
                # The presentation never changes the replay: the capture's final
                # state is the headless one, at any preset and resolution.
                if combat_state(log) != expected:
                    sys.exit(f"{target}: replay differs from headless\n{combat_state(log)}\n{expected}")
                print(f"captured {target.name}: {count} frames, replay identical ({expected.split('hash=')[1][:16]})")


def encode(name: str, preset: int, res: str) -> None:
    out = OUT / f"{name}_p{preset}_{res}_60fps.mp4"
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-framerate", "60", "-pattern_type", "glob",
         "-i", str(frames_dir(name, preset, res) / "frame_*.png"),
         "-c:v", "libx264", "-preset", "slow", "-crf", "12", "-pix_fmt", "yuv420p",
         "-movflags", "+faststart", str(out)],
        check=True,
    )


def label(img: Image.Image, text: str) -> Image.Image:
    bar = 18
    out = Image.new("RGB", (img.width, img.height + bar), (12, 10, 18))
    out.paste(img, (0, bar))
    font = ImageFont.load_default(size=13)
    ImageDraw.Draw(out).text((6, 2), text, fill=(235, 225, 200), font=font)
    return out


def gif(name: str) -> None:
    first = SCENARIOS[name][1]
    left = sorted(frames_dir(name, 1, "640x360").glob("frame_*.png"))
    right = sorted(frames_dir(name, 4, "640x360").glob("frame_*.png"))
    images = []
    for i in range(0, len(left), 2):
        a = label(Image.open(left[i]).convert("RGB"), f"preset 1 (no gore)  tick {first + i}")
        b = label(Image.open(right[i]).convert("RGB"), f"preset 4 (full, default)  tick {first + i}")
        sheet = Image.new("RGB", (a.width + b.width + 4, a.height), (12, 10, 18))
        sheet.paste(a, (0, 0))
        sheet.paste(b, (a.width + 4, 0))
        images.append(sheet.quantize(colors=128, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    # Two ticks per GIF frame: 33.3 ms, as 30, 30, 40 ms (GIF counts in 10 ms).
    durations = [(30, 30, 40)[i % 3] for i in range(len(images))]
    images[0].save(OUT / f"{name}_p1_vs_p4_640x360_30fps.gif", save_all=True,
                   append_images=images[1:], duration=durations, loop=0, optimize=True)


def sheet(name: str) -> None:
    fixture, first, count, event = SCENARIOS[name]
    rows = []
    for offset in SHEET_OFFSETS[name]:
        tick = event + offset
        cells = []
        for preset in PRESETS:
            frame = sorted(frames_dir(name, preset, "1920x1080").glob("frame_*.png"))[tick - first]
            cells.append(label(Image.open(frame).convert("RGB").crop(SHEET_CROP),
                               f"preset {preset}   tick {tick} ({'event' if offset == 0 else f'{offset:+d}'})"))
        row = Image.new("RGB", (cells[0].width * 2 + 4, cells[0].height), (12, 10, 18))
        row.paste(cells[0], (0, 0))
        row.paste(cells[1], (cells[0].width + 4, 0))
        rows.append(row)
    out = Image.new("RGB", (rows[0].width, sum(r.height for r in rows) + 4 * (len(rows) - 1)), (12, 10, 18))
    y = 0
    for row in rows:
        out.paste(row, (0, y))
        y += row.height + 4
    out.save(OUT / f"sheet_{name}_p4_vs_p1.png", optimize=True)


def stills() -> None:
    for filename, (name, preset, offset) in STILLS.items():
        _, first, _, event = SCENARIOS[name]
        frame = sorted(frames_dir(name, preset, "1920x1080").glob("frame_*.png"))[event + offset - first]
        shutil.copy(frame, OUT / filename)


def build() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name in SCENARIOS:
        for res in RESOLUTIONS:
            for preset in PRESETS:
                encode(name, preset, res)
        gif(name)
        sheet(name)
    stills()
    write_sums()


def perf() -> None:
    """Frame times with the pools at max load (`--death-stress`), software GL."""
    OUT.mkdir(parents=True, exist_ok=True)
    lines = [
        "Frame-time probe of the death presenter, DEATH-C (debug binary, opt-level 1).",
        "Software rendering under xvfb (llvmpipe): the absolute numbers are CPU raster time,",
        "not a GPU or Steam Deck figure. The comparison is between rows of the same machine.",
        "`stress` keeps every pool loaded with synthetic crushes and decapitations",
        "(a presentation-only lab flag): parts 160/160, floor decals 96/96, wall decals 12/12.",
        "",
    ]
    runs = [
        ("1920x1080 baseline (no death on screen)", "3", []),
        ("1920x1080 stress, preset 4", "3", ["--death-stress", "--gore-preset", "4"]),
        ("1920x1080 stress, preset 1", "3", ["--death-stress", "--gore-preset", "1"]),
        ("640x360 baseline (no death on screen)", "1", []),
        ("640x360 stress, preset 4", "1", ["--death-stress", "--gore-preset", "4"]),
        ("640x360 stress, preset 1", "1", ["--death-stress", "--gore-preset", "1"]),
    ]
    for title, scale, extra in runs:
        text = subprocess.run(
            XVFB + [str(BIN), "--scale", scale, "--fixture", "cp03_shore_miss", "--perf-frames", "600"] + extra,
            check=True, capture_output=True, text=True, cwd=ROOT,
        ).stdout
        perf_line = next(l for l in text.splitlines() if l.startswith("FLOOR1_PERF"))
        death_line = next(l for l in text.splitlines() if l.startswith("DEATH_PRESENT"))
        lines += [f"== {title}", perf_line, death_line, ""]
        print(title, "->", re.search(r"median_ms=\S+ p95_ms=\S+", perf_line).group(0))
    probe = subprocess.run(
        ["cargo", "test", "--locked", "--lib", "--", "--ignored", "--nocapture", "presenter_step_cost"],
        check=True, capture_output=True, text=True, cwd=ROOT,
    )
    lines += ["== presenter step on the CPU, every pool full (cargo test profile)"]
    lines += [l for l in (probe.stdout + probe.stderr).splitlines() if l.startswith("PRESENTER_STEP")]
    (OUT / "perf_frame_times.txt").write_text("\n".join(lines) + "\n")
    write_sums()


def write_sums() -> None:
    lines = []
    for path in sorted(OUT.iterdir()):
        if path.name in ("README.md", "SHA256SUMS") or not path.is_file():
            continue
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n")


def main() -> None:
    steps = sys.argv[1:] or ["capture", "build", "perf"]
    for step in steps:
        {"capture": capture, "build": build, "perf": perf}[step]()


if __name__ == "__main__":
    main()
