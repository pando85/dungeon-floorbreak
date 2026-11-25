#!/usr/bin/env python3
"""LLAMA-DEATH-WIRE evidence: real Bevy captures of the llama's decapitation and
explosive breakup (LLAMA-02-DEATH art, now wired in `src/death_present.rs`).

Captures (xvfb, the real render loop, lighting on, one frame per tick), at gore
preset 4 (the default) and preset 1, at 640x360 and 1920x1080:

  blast   `--party-fixture carl_prog_dyn_llama_core`: Carl's thrown dynamite
          lands under a passive llama and its core kills it point blank on
          X = 195 (`llama_exploded`). A real simulation kill.
  decap   `--fixture cp05_llama_crush --death-lab-edge`: the shore block lands
          on the llama on L = 139; the presentation-only evidence lab reads that
          block kill as a block-edge cut, so the presenter plays
          `llama_decapitated`. No simulation path shears a llama today (only
          the goblin has neck probes), so this finish is captured through the
          lab; the simulation still crushes it and its hash is the committed one.

Every capture's final `COMBAT_STATE` line must equal the headless replay's
(the window hash equals the headless hash at both presets and resolutions).

Builds, in docs/evidence/llama-death-wire/:

  still_<name>_p<preset>_<tick>_640x360.png    every offset, presets 4 and 1
  still_<name>_p4_<tick>_1920x1080.png         three key offsets at preset 4
  sheet_<name>_p4_vs_p1.png                    1920x1080 crops, preset 4 left, 1 right
  gif_<name>_p4_crop_960x540.gif               every tick, a 960x540 crop of the
                                               1920x1080 frames (3x, gameplay scale)
  capture_lines.txt                            the window and headless COMBAT_STATE
                                               lines of every capture
  SHA256SUMS                                   checksums (README excluded)

Needs the debug binary (`cargo build --locked --bin floor01_scene`; BIN=<path>
overrides it), the exported atlases (`bash scripts/export_assets.sh`), xvfb-run
and Pillow. The per-tick frames stay in artifacts/llama-death-wire (git ignored).
Run from the repository root:
`python3 scripts/make_llama_death_wire_evidence.py [capture] [build]`.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BIN = Path(os.environ.get("BIN", ROOT / "target" / "debug" / "floor01_scene"))
FRAMES = ROOT / "artifacts" / "llama-death-wire"
OUT = ROOT / "docs" / "evidence" / "llama-death-wire"

XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]

SCENARIOS = {
    "decap": {
        "run": ["--fixture", "cp05_llama_crush", "--death-lab-edge"],
        "headless": ["--fixture", "cp05_llama_crush"],
        # The first frame of a run is taken before the lighting is up, so the
        # first still is ten frames in.
        "first": 125,
        "count": 135,
        "event": 139,
        "tag": "L",
        "offsets": (-2, 0, 4, 10, 40, 120),
        "big": (0, 10, 120),
        "crop": (600, 420, 1560, 960),
    },
    "blast": {
        "run": ["--party-fixture", "carl_prog_dyn_llama_core"],
        "headless": ["--party-fixture", "carl_prog_dyn_llama_core"],
        "first": 180,
        "count": 151,
        "event": 195,
        "tag": "X",
        "offsets": (-5, 0, 3, 12, 40, 120),
        "big": (0, 12, 120),
        "crop": (600, 420, 1560, 960),
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


def capture() -> None:
    lines = []
    for name, spec in SCENARIOS.items():
        first, count = spec["first"], spec["count"]
        last = first + count - 1
        headless = subprocess.run(
            [str(BIN), "--headless-floor01-smoke", *spec["headless"], "--capture-tick", str(last)],
            check=True, capture_output=True, text=True, cwd=ROOT,
        ).stdout
        expected = combat_state(headless)
        lines.append(f"{name} headless {expected}")
        for res, scale in RESOLUTIONS.items():
            for preset in PRESETS:
                target = frames_dir(name, preset, res)
                shutil.rmtree(target, ignore_errors=True)
                log = subprocess.run(
                    XVFB + [str(BIN), "--scale", str(scale), *spec["run"],
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
                # The presentation never changes the replay: the window's final
                # state is the headless one, at any preset and resolution.
                window = combat_state(log)
                if window != expected:
                    sys.exit(f"{target}: replay differs from headless\n{window}\n{expected}")
                lines.append(f"{name} window p{preset} {res} {window}")
                for offset in spec["offsets"]:
                    check_lit(frame_at(name, preset, res, spec["event"] + offset))
                print(f"captured {target.name}: {count} frames, window == headless")
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "capture_lines.txt").write_text("\n".join(lines) + "\n")


def label(img: Image.Image, text: str) -> Image.Image:
    bar = 18
    out = Image.new("RGB", (img.width, img.height + bar), (12, 10, 18))
    out.paste(img, (0, bar))
    font = ImageFont.load_default(size=13)
    ImageDraw.Draw(out).text((6, 2), text, fill=(235, 225, 200), font=font)
    return out


def stills() -> None:
    for name, spec in SCENARIOS.items():
        for preset in PRESETS:
            for offset in spec["offsets"]:
                tick = spec["event"] + offset
                src = frame_at(name, preset, "640x360", tick)
                check_lit(src)
                out = OUT / f"still_{name}_p{preset}_{tick_label(name, offset)}_640x360.png"
                Image.open(src).convert("RGB").save(out, optimize=True)
        for offset in spec["big"]:
            tick = spec["event"] + offset
            src = frame_at(name, 4, "1920x1080", tick)
            check_lit(src)
            out = OUT / f"still_{name}_p4_{tick_label(name, offset)}_1920x1080.png"
            Image.open(src).convert("RGB").save(out, optimize=True)


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
    out = out.resize((out.width // 2, out.height // 2), Image.Resampling.LANCZOS)
    out.save(OUT / f"sheet_{name}_p4_vs_p1.png", optimize=True)


def gif(name: str) -> None:
    spec = SCENARIOS[name]
    images = []
    for tick in range(spec["event"] - 10, spec["first"] + spec["count"]):
        frame = Image.open(frame_at(name, 4, "1920x1080", tick)).convert("RGB").crop(spec["crop"])
        images.append(frame.quantize(colors=128, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    # One GIF frame per tick: 16.7 ms as 20, 20, 10 ms (GIF counts in 10 ms).
    durations = [(20, 20, 10)[i % 3] for i in range(len(images))]
    images[0].save(OUT / f"gif_{name}_p4_crop_960x540.gif", save_all=True,
                   append_images=images[1:], duration=durations, loop=0, optimize=True)


def build() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    stills()
    for name in SCENARIOS:
        sheet(name)
        gif(name)
    write_sums()


def write_sums() -> None:
    lines = []
    for path in sorted(OUT.iterdir()):
        if path.name in ("README.md", "SHA256SUMS") or not path.is_file():
            continue
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n")


def main() -> None:
    for step in sys.argv[1:] or ["capture", "build"]:
        {"capture": capture, "build": build}[step]()


if __name__ == "__main__":
    main()
