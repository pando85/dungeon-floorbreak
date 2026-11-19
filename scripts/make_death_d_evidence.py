#!/usr/bin/env python3
"""DEATH-D evidence: real Bevy captures of the decapitation and the explosion.

Captures (xvfb, the real render loop, lighting on, one frame per tick) the two
DEATH-D fixtures, at gore preset 4 (the default) and preset 1, at 640x360 and
1920x1080:

  cp03_block_edge_decap   the block's edge across the neck, L = 139
  qa_blast_breakup        the QA charge, X = 60 (a point blank burst, a thrown
                          ring kill and an untouched goblin)

and builds, in docs/evidence/death-d/:

  still_<name>_p<preset>_<tick>_<WxH>.png   decap on L-2, L+0, L+4, L+10, L+40,
                                            L+120; blast on X-10, X+0, X+3, X+12,
                                            X+40, X+180 (640x360 and 1920x1080,
                                            preset 4 and preset 1)
  sheet_<name>_p4_vs_p1.png                 1920x1080 crops at the same ticks,
                                            preset 4 left, preset 1 right
  gif_<name>_p4_960x540.gif                 every tick, downscaled from the
                                            1920x1080 frames (decap L-10..L+120,
                                            blast X-10..X+180)
  perf_frame_times.txt                      frame-interval probe of --death-stress
                                            (crush, decapitation and explosion in
                                            thirds) at 1920x1080 and 640x360
  SHA256SUMS                                checksums of every file (README
                                            excluded)

Needs the debug binary (`cargo build --locked --bin floor01_scene`), the exported
atlases (`bash scripts/export_assets.sh`), xvfb-run and Pillow. The per-tick PNG
frames stay in artifacts/death-d (git ignored). Run from the repository root:
`python3 scripts/make_death_d_evidence.py [capture] [build] [perf]` (no argument
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
FRAMES = ROOT / "artifacts" / "death-d"
OUT = ROOT / "docs" / "evidence" / "death-d"

XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]

# name -> fixture, first captured tick, frame count, event tick, tick label, offsets
SCENARIOS = {
    "decap": {
        "fixture": "cp03_block_edge_decap",
        "first": 129,
        "count": 131,
        "event": 139,
        "tag": "L",
        "offsets": (-2, 0, 4, 10, 40, 120),
        "crop": (640, 520, 1840, 960),
    },
    "blast": {
        "fixture": "qa_blast_breakup",
        # Starts at tick 40: the first frame of a run is taken before the
        # lighting and the assets are up (a black frame), so the X-10 still is
        # the eleventh frame of the run.
        "first": 40,
        "count": 201,
        "event": 60,
        "tag": "X",
        "offsets": (-10, 0, 3, 12, 40, 180),
        "crop": (640, 520, 1840, 960),
    },
}
PRESETS = (4, 1)
RESOLUTIONS = {"640x360": 1, "1920x1080": 3}


def frames_dir(name: str, preset: int, res: str) -> Path:
    return FRAMES / f"{name}_p{preset}_{res}"


def frame_at(name: str, preset: int, res: str, tick: int) -> Path:
    spec = SCENARIOS[name]
    frames = sorted(frames_dir(name, preset, res).glob("frame_*.png"))
    return frames[tick - spec["first"]]


def combat_state(text: str) -> str:
    lines = [l for l in text.splitlines() if l.startswith("COMBAT_STATE")]
    return lines[-1] if lines else ""


def tick_label(name: str, offset: int) -> str:
    return f"{SCENARIOS[name]['tag']}{offset:+d}"


MIN_LUMA = 100  # a real lit frame of this scene peaks far above this


def check_lit(path: Path) -> None:
    """Exit when a still is a black or unlit frame (max luma below MIN_LUMA)."""
    peak = Image.open(path).convert("L").getextrema()[1]
    if peak < MIN_LUMA:
        sys.exit(f"{path} is not a lit frame (max luma {peak} < {MIN_LUMA})")


def capture(only: tuple[str, ...] = ()) -> None:
    for name, spec in SCENARIOS.items():
        if only and name not in only:
            continue
        fixture, first, count = spec["fixture"], spec["first"], spec["count"]
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
                for offset in spec["offsets"]:
                    check_lit(frame_at(name, preset, res, spec["event"] + offset))
                print(f"captured {target.name}: {count} frames, replay identical ({expected.split('hash=')[1][:16]})")


def label(img: Image.Image, text: str) -> Image.Image:
    bar = 18
    out = Image.new("RGB", (img.width, img.height + bar), (12, 10, 18))
    out.paste(img, (0, bar))
    font = ImageFont.load_default(size=13)
    ImageDraw.Draw(out).text((6, 2), text, fill=(235, 225, 200), font=font)
    return out


def stills() -> None:
    for name, spec in SCENARIOS.items():
        for res in RESOLUTIONS:
            for preset in PRESETS:
                for offset in spec["offsets"]:
                    tick = spec["event"] + offset
                    check_lit(frame_at(name, preset, res, tick))
                    out = OUT / f"still_{name}_p{preset}_{tick_label(name, offset)}_{res}.png"
                    Image.open(frame_at(name, preset, res, tick)).convert("RGB").save(out, optimize=True)


def sheet(name: str) -> None:
    spec = SCENARIOS[name]
    rows = []
    for offset in spec["offsets"]:
        tick = spec["event"] + offset
        cells = []
        for preset in PRESETS:
            frame = Image.open(frame_at(name, preset, "1920x1080", tick)).convert("RGB").crop(spec["crop"])
            cells.append(label(frame, f"preset {preset}   tick {tick} ({tick_label(name, offset)})"))
        row = Image.new("RGB", (cells[0].width * 2 + 4, cells[0].height), (12, 10, 18))
        row.paste(cells[0], (0, 0))
        row.paste(cells[1], (cells[0].width + 4, 0))
        rows.append(row)
    out = Image.new("RGB", (rows[0].width, sum(r.height for r in rows) + 4 * (len(rows) - 1)), (12, 10, 18))
    y = 0
    for row in rows:
        out.paste(row, (0, y))
        y += row.height + 4
    # Half size keeps the sheet readable and small; the stills are full size.
    out = out.resize((out.width // 2, out.height // 2), Image.Resampling.LANCZOS)
    out.save(OUT / f"sheet_{name}_p4_vs_p1.png", optimize=True)


def gif(name: str) -> None:
    spec = SCENARIOS[name]
    first_tick = spec["event"] - 10
    last_tick = spec["event"] + (120 if name == "decap" else 180)
    images = []
    for tick in range(first_tick, last_tick + 1):
        frame = Image.open(frame_at(name, 4, "1920x1080", tick)).convert("RGB")
        frame = frame.resize((960, 540), Image.Resampling.LANCZOS)
        images.append(frame.quantize(colors=96, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    # One GIF frame per tick: 16.7 ms as 20, 20, 10 ms (GIF counts in 10 ms).
    durations = [(20, 20, 10)[i % 3] for i in range(len(images))]
    images[0].save(OUT / f"gif_{name}_p4_960x540.gif", save_all=True,
                   append_images=images[1:], duration=durations, loop=0, optimize=True)


def build() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    stills()
    for name in SCENARIOS:
        sheet(name)
        gif(name)
    write_sums()


def perf() -> None:
    """Frame intervals with the pools at max load (`--death-stress`), software GL."""
    OUT.mkdir(parents=True, exist_ok=True)
    lines = [
        "Frame-interval probe of the death presenter with explosions in the stress mix, DEATH-D",
        "(debug binary, opt-level 1). Software rendering under xvfb (llvmpipe): the absolute",
        "numbers are CPU raster time, not a GPU or Steam Deck figure; the machine was shared.",
        "The comparison is between rows of the same run. `stress` keeps every pool loaded with",
        "synthetic crushes, decapitations and explosions in thirds (a presentation-only lab flag).",
        "The DEATH-C probe of the same kind is in docs/evidence/death-c/perf_frame_times.txt",
        "(1920x1080: baseline 76.74 ms median, stress p4 78.04, stress p1 83.15).",
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
        perf_line = next(l for l in text.splitlines() if l.startswith("FLOOR1_PERF "))
        death_line = next(l for l in text.splitlines() if l.startswith("DEATH_PRESENT"))
        lines += [f"== {title}", perf_line, death_line, ""]
        print(title, "->", re.search(r"median_ms=\S+ p95_ms=\S+", perf_line).group(0),
              re.search(r"max_ms=\S+", perf_line).group(0))
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
    # `capture [decap|blast ...]` captures only the named scenarios.
    args = sys.argv[1:]
    if args and args[0] == "capture":
        capture(tuple(args[1:]))
        return
    steps = args or ["capture", "build", "perf"]
    for step in steps:
        {"capture": capture, "build": build, "perf": perf}[step]()


if __name__ == "__main__":
    main()
