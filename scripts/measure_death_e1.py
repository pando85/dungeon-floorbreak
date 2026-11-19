#!/usr/bin/env python3
"""DEATH-E1: frame-time, GPU-pass, CPU and memory probe of the death presenter.

Runs the real Bevy render loop (xvfb) of `floor01_scene` with the tick-locked
probe (`--perf-from-tick`: exactly one fixture tick per measured frame, so every
run covers the same deaths) over this matrix, at 640x360 and 1920x1080:

  baseline      cp03_shore_miss, nobody dies (the scene without cosmetics)
  pileup p1     cp03_goblin_pileup, gore preset 1 (reduced, no blood sprite)
  pileup p4     cp03_goblin_pileup, gore preset 4 (the default)
  stress p1/p4  the pileup plus `--death-stress`, which keeps every pool full

and prints a Markdown table plus the raw probe lines. Needs a release build
(`cargo build --locked --release --bin floor01_scene`), `xvfb-run`, and
`bash scripts/export_assets.sh` run once.

  python3 scripts/measure_death_e1.py [--bin PATH] [--reps 3] [--frames 600]
                                      [--out docs/evidence/death-e/perf_raw.txt]

The wall-clock frame interval under Xvfb is bound by the X11 present path (a
copy of the frame, about 60 ms at 1920x1080) and by a 60 Hz pacing at 640x360,
so it only shows whether frames were missed. The cost of the presentation is
read from the in-process timers: GPU pass time (Vulkan timestamp queries of the
render passes Bevy times: the 2D passes and the upscaling) and CPU time of the
fixture tick (simulation, animation and presenter) and of the death sprite sync.
The custom lighting pass has no timer.
"""

from __future__ import annotations

import argparse
import os
import re
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
XVFB = ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]
FROM_TICK = 70  # the first pileup defeat is on tick 83
RESOLUTIONS = {"640x360": 1, "1920x1080": 3}
CONFIGS = [
    ("baseline (nobody dies)", "cp03_shore_miss", 4, False),
    ("pileup, preset 1", "cp03_goblin_pileup", 1, False),
    ("pileup, preset 4", "cp03_goblin_pileup", 4, False),
    ("pileup + stress, preset 1", "cp03_goblin_pileup", 1, True),
    ("pileup + stress, preset 4", "cp03_goblin_pileup", 4, True),
]
# Phases of the pileup (ticks): impact 83..160 (the defeats are on 83, 139 and
# 148), peak 160..260, settle after.
PHASES = {"impact 71-160": (71, 160), "peak 161-260": (161, 260), "settle 261+": (261, 10**9)}


def kv(line: str) -> dict[str, str]:
    return dict(re.findall(r"(\w+)=(\S+)", line))


def run(binary: Path, fixture: str, scale: int, preset: int, stress: bool, frames: int) -> dict:
    cmd = XVFB + [
        str(binary), "--scale", str(scale), "--fixture", fixture,
        "--gore-preset", str(preset),
        "--perf-from-tick", str(FROM_TICK), "--perf-frames", str(frames),
    ]
    if stress:
        cmd.append("--death-stress")
    out = subprocess.run(cmd, check=True, capture_output=True, text=True, cwd=ROOT).stdout
    lines = out.splitlines()

    def find(prefix: str) -> str:
        return next(l for l in lines if l.startswith(prefix))

    perf = kv(find("FLOOR1_PERF "))
    mem = kv(find("FLOOR1_PERF_MEM"))
    cg = kv(find("FLOOR1_PERF_CPU_GPU"))
    death_line = find("DEATH_PRESENT")
    death = kv(death_line)
    death["recycled"] = re.search(r"recycled=\(([^)]*)\)", death_line).group(1)
    series = [kv(l) for l in lines if l.startswith("FLOOR1_PERF_TICK")]
    phases = {}
    for name, (lo, hi) in PHASES.items():
        ms = [float(s["ms"]) for s in series if lo <= int(s["tick"]) <= hi]
        if ms:
            phases[name] = (statistics.mean(ms), max(ms))
    return {
        "perf": perf, "mem": mem, "cg": cg, "death": death, "phases": phases,
        "raw": [find("FLOOR1_PERF "), find("FLOOR1_PERF_MEM"), find("FLOOR1_PERF_CPU_GPU"),
                find("DEATH_PRESENT"), find("FLOOR1_PERF_GPU_PASSES")],
        "peak_series": (
            max(int(s["parts"]) for s in series),
            max(int(s["floor_decals"]) for s in series),
            max(int(s["wall_decals"]) for s in series),
            max(int(s["visible_death_sprites"]) for s in series),
        ),
    }


def med(runs: list[dict], section: str, key: str) -> float:
    return statistics.median(float(r[section][key]) for r in runs)


def mx(runs: list[dict], section: str, key: str) -> float:
    return max(float(r[section][key]) for r in runs)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", default=str(ROOT / "target" / "release" / "floor01_scene"))
    parser.add_argument("--reps", type=int, default=3)
    parser.add_argument("--frames", type=int, default=600)
    parser.add_argument("--out", default=str(ROOT / "docs" / "evidence" / "death-e" / "perf_raw.txt"))
    args = parser.parse_args()
    binary = Path(args.bin)
    raw = [
        f"DEATH-E1 probe: {args.reps} runs of {args.frames} measured frames per row, "
        f"tick-locked from tick {FROM_TICK} (one fixture tick per frame).",
        f"load average before: {os.getloadavg()}",
        "",
    ]
    table = [
        "| Resolution | Scenario | frame ms mean / p95 / max | GPU ms mean / p95 / max | "
        "tick us mean / max | sprite sync us mean / max | RSS MiB (peak) | entities | "
        "peak parts / floor / wall / visible sprites | final recycled (parts, floor, wall) |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    phase_rows = ["| Resolution | Scenario | " + " | ".join(PHASES) + " |", "| --- | --- | " + " | ".join("---" for _ in PHASES) + " |"]
    for res, scale in RESOLUTIONS.items():
        for title, fixture, preset, stress in CONFIGS:
            runs = []
            for rep in range(args.reps):
                result = run(binary, fixture, scale, preset, stress, args.frames)
                runs.append(result)
                raw += [f"== {res} {title} run {rep + 1}"] + result["raw"] + [""]
                print(res, title, rep + 1, result["perf"]["mean_ms"], result["cg"]["gpu_ms_mean"], flush=True)
            peaks = [max(r["peak_series"][i] for r in runs) for i in range(4)]
            table.append(
                f"| {res} | {title} | {med(runs, 'perf', 'mean_ms'):.2f} / {med(runs, 'perf', 'p95_ms'):.2f} / "
                f"{mx(runs, 'perf', 'max_ms'):.2f} | {med(runs, 'cg', 'gpu_ms_mean'):.3f} / "
                f"{med(runs, 'cg', 'gpu_ms_p95'):.3f} / {mx(runs, 'cg', 'gpu_ms_max'):.3f} | "
                f"{med(runs, 'cg', 'tick_us_mean'):.1f} / {mx(runs, 'cg', 'tick_us_max'):.0f} | "
                f"{med(runs, 'cg', 'death_sync_us_mean'):.1f} / {mx(runs, 'cg', 'death_sync_us_max'):.0f} | "
                f"{med(runs, 'mem', 'rss_mib'):.0f} ({mx(runs, 'mem', 'peak_rss_mib'):.0f}) | "
                f"{med(runs, 'mem', 'entities'):.0f} | {peaks[0]} / {peaks[1]} / {peaks[2]} / {peaks[3]} | "
                f"{runs[-1]['death']['recycled']} |"
            )
            phase_rows.append(
                f"| {res} | {title} | "
                + " | ".join(
                    f"{statistics.median(r['phases'][p][0] for r in runs):.2f} / {max(r['phases'][p][1] for r in runs):.2f}"
                    for p in PHASES
                )
                + " |"
            )
    raw += [f"load average after: {os.getloadavg()}", ""]
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(raw) + "\n")
    summary = out.with_name(out.stem.replace("raw", "table") + ".md")
    summary.write_text(
        "# DEATH-E1 probe tables\n\n"
        "Frame interval ms by phase of the pileup (mean / max, median of runs):\n\n"
        + "\n".join(phase_rows) + "\n\nAll probes:\n\n" + "\n".join(table) + "\n"
    )
    print("\n".join(table))
    print("\n".join(phase_rows))


if __name__ == "__main__":
    sys.exit(main())
