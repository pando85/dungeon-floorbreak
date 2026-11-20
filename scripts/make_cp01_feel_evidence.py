#!/usr/bin/env python3
"""CP-01 evidence: GIFs, stills, a per-tick strip and timing diagrams.

Inputs (produced by `xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render-sequences`, real Bevy 1920x1080
captures, one per tick, lighting on):

  artifacts/cp01-feel/frames_<fixture>/frame_NNNN_tick_TTTTT.png

and the `floor01_scene` debug binary (`cargo build --locked --bin floor01_scene`)
for the headless logs and the per-tick traces.

Outputs in docs/evidence/cp01-feel (Pillow and matplotlib only):

  headless_<fixture>.txt              COMBAT_EVENT lines and the final COMBAT_STATE
                                      line of every fixture (CI diffs against them)
  trace_<fixture>.txt                 one COMBAT_TRACE row per tick (see --combat-trace)
  <fixture>_ticks<a>-<b>_960x540.gif  native-pixel 960x540 crop of the 1920x1080
                                      capture, one frame per tick, shown at 50 ms per
                                      tick (3x slower than real time), annotated
  strip_haymaker_kill_ticks150-161.png every tick around the killing hit
  still_<name>_tick<N>_1920x1080.png  full captures of the key ticks
  timing_haymaker_chain.png           input, startup, hit-stop, stagger, knockback,
                                      defeat, camera kick on one time axis
  timeline_goblin_attack.png          the goblin tell and swing, Carl's flinch and
                                      invulnerability
  timing_interrupt.png                the interrupted tell
  SHA256SUMS                          checksums of every file above (README excluded)

Run from the repository root: `python3 scripts/make_cp01_feel_evidence.py`.
"""
from __future__ import annotations

import hashlib
import re
import shutil
import subprocess
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib import font_manager  # noqa: E402
from matplotlib.patches import Patch  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "debug" / "floor01_scene"
FRAMES = ROOT / "artifacts" / "cp01-feel"
OUT = ROOT / "docs" / "evidence" / "cp01-feel"

FIXTURES = [
    "cp01_single_goblin",
    "cp01_two_hits",
    "cp02_two_goblins",
    "cp01_goblin_attacks",
    "cp01_interrupt",
    "cp01_haymaker_chain",
    "cp01_retry",
    "cp01_kick",
]
TRACED = ["cp01_single_goblin", "cp01_interrupt", "cp01_haymaker_chain", "cp01_goblin_attacks", "cp01_retry"]
# fixture -> (first tick, frames) of the real capture sequence (scripts/ci_floor01_combat.sh)
SEQUENCES = {
    "cp01_single_goblin": (118, 61),
    "cp01_haymaker_chain": (128, 61),
    "cp01_interrupt": (122, 61),
}
# Attack presses of the scripted fixtures (the `script` of src/combat_scene.rs).
PRESSES = {
    "cp01_haymaker_chain": [137, 141],
    "cp01_interrupt": [137],
    "cp01_single_goblin": [122, 136],
}
# Native-pixel crop of the 1920x1080 capture: Carl and the goblin stand at about x 890..1000.
CROP = (450, 440, 1410, 980)
STRIP_CROP = (740, 620, 1200, 900)
GIF_MS_PER_TICK = 50
# Move timings (startup, active, recovery) from src/combat.rs.
MOVES = {"jab": (3, 2, 9), "haymaker": (5, 3, 14)}

PHASE_COLOURS = {"startup": "#e8c547", "active": "#d1495b", "recovery": "#8aa1b1"}
MODE_COLOURS = {
    "Neutral": "#9bc53d",
    "Attack/Tell": "#f08a24",
    "Attack/Smear": "#f2c14e",
    "Attack/Impact": "#d1495b",
    "Attack/Recover": "#8aa1b1",
    "Hurt": "#f4e285",
    "Airborne": "#8d6cab",
    "Down": "#7d7d7d",
    "Corpse": "#3b3b3b",
}


def run(args: list[str]) -> str:
    return subprocess.run([str(BIN), *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout


def headless(fixture: str, trace: bool = False) -> str:
    args = ["--headless-floor01-smoke", "--fixture", fixture]
    if trace:
        args.append("--combat-trace")
    return run(args)


def parse_trace(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        if not line.startswith("COMBAT_TRACE"):
            continue
        row = dict(token.split("=", 1) for token in line.split()[1:])
        rows.append(row)
    return rows


def font(size: int) -> ImageFont.FreeTypeFont:
    path = font_manager.findfont("DejaVu Sans")
    return ImageFont.truetype(path, size)


def frame_files(fixture: str) -> list[tuple[int, Path]]:
    folder = FRAMES / f"frames_{fixture}"
    found = []
    for path in sorted(folder.glob("frame_*_tick_*.png")):
        found.append((int(re.search(r"tick_(\d+)", path.name).group(1)), path))
    if not found:
        sys.exit(f"no frames in {folder}: run scripts/ci_floor01_combat.sh render-sequences first")
    return found


def caption(row: dict) -> str:
    stop = "  FROZEN" if row["carl_move"].endswith("!") else ""
    goblin = row["g0"]
    hs = int(row["g0_hs"])
    return f"tick {row['tick']:>3}   Carl {row['carl_move']}{stop}   goblin {goblin}" + (f" (frozen {hs})" if hs else "")


def make_gif(fixture: str, rows_by_tick: dict[int, dict]) -> Path:
    frames = frame_files(fixture)
    small = font(20)
    images = []
    for tick, path in frames:
        image = Image.open(path).convert("RGB").crop(CROP)
        draw = ImageDraw.Draw(image, "RGBA")
        draw.rectangle((0, 0, image.width, 30), fill=(0, 0, 0, 170))
        draw.text((10, 3), caption(rows_by_tick[tick]), font=small, fill=(255, 255, 255, 255))
        images.append(image.quantize(colors=128, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    first, last = frames[0][0], frames[-1][0]
    target = OUT / f"{fixture}_ticks{first}-{last}_960x540.gif"
    images[0].save(target, save_all=True, append_images=images[1:], duration=GIF_MS_PER_TICK, loop=0, optimize=True)
    return target


def make_strip(rows_by_tick: dict[int, dict]) -> Path:
    frames = dict(frame_files("cp01_haymaker_chain"))
    ticks = list(range(150, 162))
    cell_w, cell_h = STRIP_CROP[2] - STRIP_CROP[0], STRIP_CROP[3] - STRIP_CROP[1]
    cols, label_h = 4, 62
    rows_n = (len(ticks) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * cell_w, rows_n * (cell_h + label_h)), (16, 16, 20))
    draw = ImageDraw.Draw(sheet)
    big, small = font(24), font(18)
    for index, tick in enumerate(ticks):
        x, y = (index % cols) * cell_w, (index // cols) * (cell_h + label_h)
        sheet.paste(Image.open(frames[tick]).convert("RGB").crop(STRIP_CROP), (x, y + label_h))
        row = rows_by_tick[tick]
        frozen = row["carl_move"].endswith("!")
        draw.text((x + 8, y + 4), f"tick {tick}", font=big, fill=(255, 255, 255))
        draw.text((x + 8, y + 34), f"Carl {row['carl_move']}", font=small, fill=(255, 120, 120) if frozen else (200, 200, 200))
        draw.text((x + cell_w // 2, y + 34), f"goblin hs={row['g0_hs']}", font=small, fill=(255, 220, 120))
    target = OUT / "strip_haymaker_kill_ticks150-161.png"
    sheet.save(target)
    return target


def make_stills() -> list[Path]:
    wanted = [
        ("cp01_haymaker_chain", 140, "jab_hit_interrupts_tell"),
        ("cp01_haymaker_chain", 152, "haymaker_kill_hit_stop"),
        ("cp01_haymaker_chain", 175, "kill_knockback_flight"),
        ("cp01_single_goblin", 141, "single_goblin_kill"),
        ("cp01_interrupt", 130, "goblin_tell"),
    ]
    made = []
    for fixture, tick, name in wanted:
        frames = dict(frame_files(fixture))
        if tick not in frames:
            continue
        target = OUT / f"still_{name}_tick{tick}_1920x1080.png"
        shutil.copyfile(frames[tick], target)
        made.append(target)
    return made


def runs(values: list[str]) -> list[tuple[int, int, str]]:
    """Run-length segments (start index, length, value)."""
    segments = []
    for index, value in enumerate(values):
        if segments and segments[-1][2] == value:
            start, length, _ = segments[-1]
            segments[-1] = (start, length + 1, value)
        else:
            segments.append((index, 1, value))
    return segments


def goblin_state(row: dict, goblin: str = "g0") -> str:
    return row[goblin]


def goblin_frozen(window: list[dict], goblin: str = "g0") -> list[bool]:
    """Ticks whose goblin step was skipped by hit-stop: the hit tick itself and
    the `hs` ticks after it (the trace shows the counter after the step)."""
    flags = []
    previous = 0
    for row in window:
        current = int(row[f"{goblin}_hs"])
        flags.append(current > 0 or previous > 0)
        previous = current
    return flags


SHORT = {
    "Attack/Tell": "tell",
    "Attack/Smear": "smear",
    "Attack/Impact": "HIT",
    "Attack/Recover": "recover",
}


def goblin_lane(ax, window: list[dict]) -> list[str]:
    ticks = [int(r["tick"]) for r in window]
    states = [goblin_state(r) for r in window]
    frozen = goblin_frozen(window)
    slot = 0
    for start, length, value in runs(states):
        ax.broken_barh([(ticks[start] - 0.5, length)], (0.1, 0.8), facecolors=MODE_COLOURS.get(value, "#cccccc"), edgecolor="black", lw=0.5)
        seg = window[start:start + length]
        label = SHORT.get(value, value)
        if value.startswith("Attack/"):
            label += f" t{seg[0]['g0_t']}-{seg[-1]['g0_t']}"
        else:
            label += f" ({length})"
        if length >= 12:
            ax.text(ticks[start] - 0.3, 0.5, label, fontsize=8, va="center")
        else:
            # Short segments: the label sits above the lane, alternating height.
            ax.text(ticks[start] - 0.3, 1.04 + 0.16 * (slot % 2), label, fontsize=7.5, va="bottom")
            slot += 1
    for index, is_frozen in enumerate(frozen):
        if is_frozen:
            ax.broken_barh([(ticks[index] - 0.5, 1)], (0.1, 0.8), facecolors="none", edgecolor="#222", hatch="////", lw=0)
    return states


def timing_diagram(fixture: str, rows: list[dict], lo: int, hi: int, hits: dict[int, str], title: str, name: str) -> Path:
    window = [r for r in rows if lo <= int(r["tick"]) <= hi]
    ticks = [int(r["tick"]) for r in window]
    fig, axes = plt.subplots(
        5, 1, figsize=(15, 9.5), sharex=True, gridspec_kw={"height_ratios": [0.8, 1.2, 1.2, 1.6, 1.0]}
    )
    fig.suptitle(title, fontsize=14, y=0.995)
    # 1. input
    ax = axes[0]
    for press in PRESSES.get(fixture, []):
        ax.axvline(press, color="#1f77b4", lw=2)
        ax.text(press - 0.3, 0.05, "attack press", rotation=90, va="bottom", ha="right", fontsize=9, color="#1f77b4")
    ax.set_yticks([])
    ax.set_ylabel("input")
    # 2. Carl's move
    ax = axes[1]
    carl_states = []
    for row in window:
        move = row["carl_move"].rstrip("!")
        if move == "-":
            carl_states.append("idle")
            continue
        name_, tick_ = move.split(":")
        startup, active, _ = MOVES[name_]
        tick_i = int(tick_)
        phase = "startup" if tick_i < startup else "active" if tick_i < startup + active else "recovery"
        carl_states.append(f"{name_} {phase}")
    for start, length, value in runs(carl_states):
        if value == "idle":
            continue
        phase = value.split()[1]
        ax.broken_barh([(ticks[start] - 0.5, length)], (0.1, 0.8), facecolors=PHASE_COLOURS[phase], edgecolor="black", lw=0.5)
        ax.text(ticks[start] - 0.3, 0.5, value, fontsize=7.5, va="center")
    for row in window:
        if row["carl_move"].endswith("!"):
            ax.broken_barh([(int(row["tick"]) - 0.5, 1)], (0.1, 0.8), facecolors="none", edgecolor="#222", hatch="////", lw=0)
    ax.set_yticks([])
    ax.set_ylabel("Carl move")
    # 3. hit-stop
    ax = axes[2]
    carl_frozen = [int(r["tick"]) for r in window if r["carl_move"].endswith("!")]
    gob_frozen = [t for t, f in zip(ticks, goblin_frozen(window)) if f]
    for tick in carl_frozen:
        ax.broken_barh([(tick - 0.5, 1)], (0.55, 0.35), facecolors="#d1495b")
    for tick in gob_frozen:
        ax.broken_barh([(tick - 0.5, 1)], (0.1, 0.35), facecolors="#f2a541")
    ax.set_yticks([0.275, 0.725])
    ax.set_yticklabels(["goblin", "Carl"])
    ax.set_ylabel("hit-stop (frozen)")
    # 4. goblin state
    ax = axes[3]
    states = goblin_lane(ax, window)
    ax.set_yticks([])
    ax.set_ylabel("goblin state\n(hatched = frozen)")
    # 5. knockback and camera kick
    ax = axes[4]
    x0 = float(window[0]["g0_x"])
    ax.plot(ticks, [float(r["g0_x"]) - x0 for r in window], color="#444", label="goblin x (px from start)")
    ax.plot(ticks, [float(r["g0_y"]) - float(window[0]["g0_y"]) for r in window], color="#8d6cab", label="goblin height (px)")
    ax.plot(ticks, [int(r["kick"]) * 5 for r in window], color="#2a9d8f", drawstyle="steps-post", label="camera kick (x5, virtual px)")
    ax.axhline(0, color="#aaa", lw=0.5)
    ax.legend(loc="upper left", fontsize=8)
    ax.set_ylabel("px")
    ax.set_xlabel("scene tick (60 per second)")
    for ax in axes:
        for tick, label in hits.items():
            ax.axvline(tick, color="#d1495b", ls="--", lw=1)
        ax.set_xlim(lo - 0.5, hi + 0.5)
        ax.grid(axis="x", color="#ddd", lw=0.4)
    for tick, label in hits.items():
        axes[0].text(tick + 0.3, 0.95, label, color="#d1495b", fontsize=9, va="top")
    axes[3].legend(
        handles=[Patch(facecolor=c, label=m) for m, c in MODE_COLOURS.items() if m in set(states)],
        loc="upper right",
        fontsize=7,
        ncol=3,
    )
    fig.tight_layout()
    target = OUT / name
    fig.savefig(target, dpi=100)
    plt.close(fig)
    return target


def goblin_timeline(rows: list[dict], lo: int, hi: int) -> Path:
    window = [r for r in rows if lo <= int(r["tick"]) <= hi]
    ticks = [int(r["tick"]) for r in window]
    fig, axes = plt.subplots(4, 1, figsize=(15, 7.5), sharex=True, gridspec_kw={"height_ratios": [1.6, 0.8, 0.8, 0.8]})
    fig.suptitle("cp01_goblin_attacks: the goblin's tell and swing, Carl's flinch and invulnerability (real tick data)", fontsize=13, y=0.995)
    ax = axes[0]
    goblin_lane(ax, window)
    ax.set_yticks([])
    ax.set_ylabel("goblin (attack ticks t..)\nhatched = hit-stop")
    ax = axes[1]
    ax.fill_between(ticks, [int(r["hurt"]) for r in window], step="post", color="#d1495b")
    ax.set_ylabel("Carl flinch\n(ticks left, 18)")
    ax = axes[2]
    ax.fill_between(ticks, [int(r["invuln"]) for r in window], step="post", color="#4c78a8")
    ax.set_ylabel("Carl i-frames\n(ticks left, 55)")
    ax = axes[3]
    ax.step(ticks, [int(r["carl_hp"]) for r in window], where="post", color="#222")
    ax.set_ylim(0, 4.5)
    ax.set_ylabel("Carl hit points")
    ax.set_xlabel("scene tick (60 per second)")
    for ax in axes:
        ax.set_xlim(lo - 0.5, hi + 0.5)
        ax.grid(axis="x", color="#ddd", lw=0.4)
    fig.tight_layout()
    target = OUT / "timeline_goblin_attack.png"
    fig.savefig(target, dpi=100)
    plt.close(fig)
    return target


def main() -> int:
    if not BIN.is_file():
        sys.exit("build first: cargo build --locked --bin floor01_scene")
    OUT.mkdir(parents=True, exist_ok=True)
    for fixture in FIXTURES:
        lines = [
            line
            for line in headless(fixture).splitlines()
            if line.startswith(("COMBAT_EVENT", "COMBAT_STATE"))
        ]
        (OUT / f"headless_{fixture}.txt").write_text("\n".join(lines) + "\n")
    traces = {}
    for fixture in TRACED:
        text = headless(fixture, trace=True)
        rows = parse_trace(text)
        traces[fixture] = rows
        (OUT / f"trace_{fixture}.txt").write_text("\n".join(l for l in text.splitlines() if l.startswith("COMBAT_TRACE")) + "\n")
    made = []
    if FRAMES.is_dir():
        for fixture in SEQUENCES:
            by_tick = {int(r["tick"]): r for r in traces[fixture]}
            made.append(make_gif(fixture, by_tick))
        made.append(make_strip({int(r["tick"]): r for r in traces["cp01_haymaker_chain"]}))
        made += make_stills()
    made.append(
        timing_diagram(
            "cp01_haymaker_chain",
            traces["cp01_haymaker_chain"],
            132,
            192,
            {140: "jab hit (hit-stop 3)", 152: "haymaker hit = kill (hit-stop 8)"},
            "cp01_haymaker_chain: input -> startup -> hit-stop -> stagger -> knockback -> defeat (real tick data)",
            "timing_haymaker_chain.png",
        )
    )
    made.append(
        timing_diagram(
            "cp01_interrupt",
            traces["cp01_interrupt"],
            122,
            196,
            {140: "jab hit during the tell: swing cancelled"},
            "cp01_interrupt: a hit in the tell cancels the swing; the next tell waits for the stagger and the 20-tick cooldown",
            "timing_interrupt.png",
        )
    )
    made.append(goblin_timeline(traces["cp01_goblin_attacks"], 120, 262))
    for path in made:
        print("wrote", path.relative_to(ROOT))
    sums = []
    for path in sorted(OUT.iterdir()):
        if path.name in ("README.md", "SHA256SUMS") or not path.is_file():
            continue
        sums.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (OUT / "SHA256SUMS").write_text("\n".join(sums) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
