#!/usr/bin/env python3
"""ANIM-INT-01 evidence: GIF, stills and a timeline diagram from real Bevy frames.

Inputs are produced by the floor01_scene binary:

  floor01_scene --scale 3 --scenario run_jump --capture-sequence 142:3:81 \
      --capture-dir <frames> > <capture.log>
  floor01_scene --headless-floor01-smoke --scenario run_jump --capture-tick 382 \
      --anim-trace > <trace.txt>
  floor01_scene --scale 3 --scenario run_jump --capture-sequence 168:1:15 \
      --capture-dir <strip frames> > <strip.log>      (one per --strip)

The capture log carries one `FLOOR1_CAPTURE` line per saved 1920x1080 window
screenshot followed by its `FLOOR1_SCENE_STATE` and `FLOOR1_ANIM_STATE` lines.
The trace holds one `FLOOR1_ANIM_TRACE` row per simulated tick.

Outputs (all deterministic for the same inputs, Pillow only):

  run_jump_1080p.gif              cropped 3x frames, one per captured tick step, annotated
  still_mid_run.png               full 1920x1080 frame, Carl and Donut running
  still_jump_start.png            the takeoff frames (crouch on the impulse tick)
  still_apex.png                  the apex tick of the running jump
  still_landing.png               the touchdown tick (land impact frame)
  timeline_run_jump.png           ticks on x: animation state and frame index per actor,
                                  height curves, markers for jump input, apex and touchdown
  strip_<name>.png                per-tick contact strips (every tick, no sampling) around a
                                  takeoff and a landing, from --strip captures
  capture_log.txt, anim_trace.txt the raw state lines the images were made from
  capture_log_strip_<name>.txt    the state lines of each strip
  SHA256SUMS                      checksums of every file above
"""
from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# Crop of the 1920x1080 frame for the GIF: 3x of 352 x 198 virtual pixels around
# the actors, fixed so the camera motion stays visible.
CROP = (408, 330, 408 + 1056, 330 + 594)
GIF_COLOURS = 255
# Each captured step is 3 fixed ticks = 50 ms exactly (5 GIF centiseconds), so
# the GIF plays in real time.
GIF_DELAY_MS = 50

TAG_COLOURS = {
    "idle": (116, 124, 140),
    "run": (46, 120, 214),
    "jump_start": (226, 150, 30),
    "jump_rise": (38, 166, 120),
    "jump_fall": (208, 70, 70),
    "land": (140, 90, 200),
}
BACKGROUND = (22, 24, 30)
FOREGROUND = (232, 234, 240)
MUTED = (150, 156, 170)
CARL_LINE = (240, 240, 245)
DONUT_LINE = (255, 170, 80)
MARKER_COLOURS = {"jump input": (255, 220, 90), "apex": (120, 220, 255), "touchdown": (255, 120, 190)}


class EvidenceError(Exception):
    pass


@dataclass
class Capture:
    index: int
    tick: int
    file: str
    hero_x: float
    cam_left: int
    carl: str
    donut: str


@dataclass
class Row:
    tick: int
    hero_x: float
    hero_foot_y: float
    vx: float
    vy: float
    grounded: bool
    donut_foot_y: float
    carl: str
    donut: str

    @property
    def carl_tag(self) -> str:
        return self.carl.split(":")[0]

    @property
    def donut_tag(self) -> str:
        return self.donut.split(":")[0]


def kv(line: str) -> dict[str, str]:
    return dict(part.split("=", 1) for part in line.split()[1:] if "=" in part)


def parse_captures(text: str) -> list[Capture]:
    captures: list[Capture] = []
    lines = text.splitlines()
    for position, line in enumerate(lines):
        if not line.startswith("FLOOR1_CAPTURE "):
            continue
        head = kv(line)
        scene = anim = None
        for follow in lines[position + 1:position + 4]:
            if follow.startswith("FLOOR1_SCENE_STATE"):
                scene = kv(follow)
            elif follow.startswith("FLOOR1_ANIM_STATE"):
                anim = kv(follow)
        if scene is None or anim is None:
            raise EvidenceError(f"capture {head.get('index')} has no state lines")
        hero_x = float(scene["hero"].split(",")[0])
        cam_left = int(scene["cam"].split(",")[0])
        captures.append(Capture(int(head["index"]), int(head["tick"]), Path(head["file"]).name,
                                hero_x, cam_left, anim["carl"], anim["donut"]))
        if int(scene["tick"]) != captures[-1].tick or int(anim["tick"]) != captures[-1].tick:
            raise EvidenceError(f"capture {head['index']}: state lines belong to another tick")
    if not captures:
        raise EvidenceError("no FLOOR1_CAPTURE lines in the log")
    return captures


def parse_trace(text: str) -> list[Row]:
    rows = []
    for line in text.splitlines():
        if not line.startswith("FLOOR1_ANIM_TRACE "):
            continue
        d = kv(line)
        rows.append(Row(int(d["tick"]), float(d["hero_x"]), float(d["hero_foot_y"]),
                        float(d["vx"]), float(d["vy"]), d["grounded"] == "1",
                        float(d["donut_foot_y"]), d["carl"], d["donut"]))
    if not rows:
        raise EvidenceError("no FLOOR1_ANIM_TRACE lines in the trace")
    return rows


def check_log_against_trace(captures: list[Capture], rows: list[Row]) -> None:
    """The rendered run and the headless simulation must agree on every capture."""
    by_tick = {row.tick: row for row in rows}
    for capture in captures:
        row = by_tick.get(capture.tick)
        if row is None:
            raise EvidenceError(f"tick {capture.tick} is not in the trace")
        if (row.carl, row.donut) != (capture.carl, capture.donut):
            raise EvidenceError(
                f"tick {capture.tick}: rendered {capture.carl}/{capture.donut} "
                f"but the headless trace says {row.carl}/{row.donut}")
        if abs(row.hero_x - capture.hero_x) > 0.0006:
            raise EvidenceError(f"tick {capture.tick}: hero x differs between render and trace")


# ------------------------------------------------------------------- markers --

def takeoffs(rows: list[Row]) -> list[int]:
    """Ticks on which the hero leaves the ground rising (the jump impulse)."""
    return [cur.tick for prev, cur in zip(rows, rows[1:])
            if prev.grounded and not cur.grounded and cur.vy > 0]


def jump_markers(rows: list[Row], start: int) -> dict[str, int]:
    """Takeoff, apex and touchdown ticks of the first rising jump at or after `start`."""
    by_tick = {row.tick: row for row in rows}
    takeoff = next(t for t in takeoffs(rows) if t >= start)
    apex = next(row.tick for row in rows if row.tick > takeoff and not row.grounded and row.vy <= 0)
    touchdown = next(row.tick for row in rows if row.tick > apex and row.grounded)
    assert by_tick[takeoff].carl_tag == "jump_start"
    return {"takeoff": takeoff, "apex": apex, "touchdown": touchdown}


def donut_events(rows: list[Row], start: int, end: int) -> list[tuple[str, int]]:
    """Donut's own takeoff, apex and touchdown ticks, from her animation state."""
    events = []
    previous = None
    for row in rows:
        if not start <= row.tick <= end:
            previous = row.donut_tag
            continue
        tag = row.donut_tag
        if tag != previous:
            if tag == "jump_start":
                events.append(("jump input", row.tick))
            elif tag == "jump_fall" and previous in ("jump_rise", "jump_start"):
                events.append(("apex", row.tick))
            elif tag == "land":
                events.append(("touchdown", row.tick))
        previous = tag
    return events


def nearest_capture(captures: list[Capture], tick: int) -> Capture:
    return min(captures, key=lambda c: (abs(c.tick - tick), c.tick))


# --------------------------------------------------------------------- fonts --

def font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.load_default(size=size)
    except TypeError:  # Pillow without sized default fonts
        return ImageFont.load_default()


# ----------------------------------------------------------------------- GIF --

def build_gif(frames_dir: Path, captures: list[Capture], out: Path) -> None:
    small = font(15)
    crops = []
    for capture in captures:
        image = Image.open(frames_dir / capture.file).convert("RGB")
        if image.size != (1920, 1080):
            raise EvidenceError(f"{capture.file} is {image.size}, expected 1920x1080")
        crop = image.crop(CROP)
        draw = ImageDraw.Draw(crop)
        draw.rectangle((0, 0, crop.width, 26), fill=(0, 0, 0))
        draw.text((8, 4), f"tick {capture.tick:3d}   Carl {capture.carl:<13} Donut {capture.donut}",
                  fill=(240, 240, 240), font=small)
        crops.append(crop)
    # One shared palette so the lighting gradients do not flicker between frames.
    sample = Image.new("RGB", (CROP[2] - CROP[0], (CROP[3] - CROP[1]) * min(len(crops), 8)))
    for k, crop in enumerate(crops[:: max(1, len(crops) // 8)][:8]):
        sample.paste(crop, (0, k * (CROP[3] - CROP[1])))
    palette = sample.quantize(colors=GIF_COLOURS, method=Image.Quantize.MEDIANCUT,
                              dither=Image.Dither.NONE)
    frames = [crop.quantize(palette=palette, dither=Image.Dither.NONE) for crop in crops]
    frames[0].save(out, save_all=True, append_images=frames[1:], duration=GIF_DELAY_MS,
                   loop=0, optimize=False, disposal=1)


# --------------------------------------------------------------------- strips --

STRIP_COLUMNS = 4
STRIP_CELL = (480, 460)          # crop of the 1920x1080 frame, 3x pixels
STRIP_TOP = 440                  # crop rows 440..900 hold the floor and the apex region
STRIP_LABEL_H = 38


def build_strip(frames_dir: Path, captures: list[Capture], out: Path) -> None:
    """Every captured tick as one cell: crop around Carl, tick and states above it."""
    small = font(15)
    cell_w, cell_h = STRIP_CELL
    rows_n = (len(captures) + STRIP_COLUMNS - 1) // STRIP_COLUMNS
    sheet = Image.new("RGB", (cell_w * STRIP_COLUMNS, (cell_h + STRIP_LABEL_H) * rows_n), BACKGROUND)
    draw = ImageDraw.Draw(sheet)
    for position, capture in enumerate(captures):
        image = Image.open(frames_dir / capture.file).convert("RGB")
        if image.size != (1920, 1080):
            raise EvidenceError(f"{capture.file} is {image.size}, expected 1920x1080")
        carl_x = round((capture.hero_x - capture.cam_left) * 3)
        left = min(max(carl_x - 330, 0), 1920 - cell_w)
        cell = image.crop((left, STRIP_TOP, left + cell_w, STRIP_TOP + cell_h))
        x = (position % STRIP_COLUMNS) * cell_w
        y = (position // STRIP_COLUMNS) * (cell_h + STRIP_LABEL_H)
        sheet.paste(cell, (x, y + STRIP_LABEL_H))
        draw.text((x + 6, y + 3), f"tick {capture.tick}", fill=FOREGROUND, font=small)
        draw.text((x + 6, y + 20), f"Carl {capture.carl}   Donut {capture.donut}", fill=MUTED, font=small)
        draw.rectangle((x, y, x + cell_w - 1, y + cell_h + STRIP_LABEL_H - 1), outline=(60, 64, 76))
    sheet.save(out, optimize=True)


def state_lines(text: str, directory: Path, name: str) -> str:
    keep = ("FLOOR1_CAPTURE", "FLOOR1_SCENE_STATE", "FLOOR1_ANIM_STATE")
    return "\n".join(line for line in text.splitlines() if line.startswith(keep)) \
        .replace(str(directory), name) + "\n"


# ------------------------------------------------------------------ timeline --

def draw_timeline(rows: list[Row], first: int, last: int, markers: list[dict[str, int]],
                  donut_marks: list[tuple[str, int]], out: Path) -> None:
    px = 7                       # pixels per tick
    left, right, top = 96, 28, 20
    ticks = [row for row in rows if first <= row.tick <= last]
    width = left + len(ticks) * px + right
    bar_h, gap = 34, 18
    plot_h = 150
    carl_y = top + 102
    donut_y = carl_y + bar_h + gap
    plot_y = donut_y + bar_h + 40
    axis_y = plot_y + plot_h + 10
    height = axis_y + 64
    image = Image.new("RGB", (width, height), BACKGROUND)
    draw = ImageDraw.Draw(image)
    f_title, f_text, f_small = font(20), font(14), font(12)

    def x_of(tick: int) -> int:
        return left + (tick - first) * px

    draw.text((left, 8), "ANIM-INT-01 timeline: run_jump scenario, 60 Hz fixed ticks "
              "(Bevy floor01_scene state trace)", fill=FOREGROUND, font=f_title)
    # Legend.
    lx = left
    for tag, colour in TAG_COLOURS.items():
        draw.rectangle((lx, top + 22, lx + 16, top + 36), fill=colour)
        draw.text((lx + 22, top + 22), tag, fill=FOREGROUND, font=f_text)
        lx += 22 + int(draw.textlength(tag, font=f_text)) + 22
    draw.text((lx + 10, top + 22), "number in a bar = atlas frame index", fill=MUTED, font=f_text)
    mx = left
    for name, colour in MARKER_COLOURS.items():
        draw.line((mx, top + 52, mx + 22, top + 52), fill=colour, width=3)
        draw.text((mx + 28, top + 44), name, fill=FOREGROUND, font=f_text)
        mx += 28 + int(draw.textlength(name, font=f_text)) + 26

    def draw_row(y: int, label: str, pick) -> None:
        draw.text((10, y + 9), label, fill=FOREGROUND, font=f_text)
        start = 0
        while start < len(ticks):
            key = pick(ticks[start])
            end = start
            while end + 1 < len(ticks) and pick(ticks[end + 1]) == key:
                end += 1
            tag, index = key.split(":")
            x0, x1 = x_of(ticks[start].tick), x_of(ticks[end].tick) + px
            draw.rectangle((x0, y, x1 - 1, y + bar_h), fill=TAG_COLOURS[tag], outline=BACKGROUND)
            if x1 - x0 >= 10:
                draw.text((x0 + 3, y + 10), index, fill=(255, 255, 255), font=f_small)
            start = end + 1
        # Tag names above runs of the same tag when wide enough.
        start = 0
        while start < len(ticks):
            tag = pick(ticks[start]).split(":")[0]
            end = start
            while end + 1 < len(ticks) and pick(ticks[end + 1]).split(":")[0] == tag:
                end += 1
            x0, x1 = x_of(ticks[start].tick), x_of(ticks[end].tick) + px
            if x1 - x0 >= 8 * len(tag):
                draw.text((x0 + 2, y - 15), tag, fill=TAG_COLOURS[tag], font=f_small)
            start = end + 1

    draw_row(carl_y, "Carl", lambda r: r.carl)
    draw_row(donut_y, "Donut", lambda r: r.donut)

    # Height curves (feet above the base walking line, px).
    base = 92.0
    top_value = max(max(r.hero_foot_y, r.donut_foot_y) for r in ticks) - base
    scale = (plot_h - 12) / max(top_value, 1.0)
    draw.rectangle((left, plot_y, left + len(ticks) * px, plot_y + plot_h), outline=(70, 74, 86))
    draw.text((10, plot_y + 4), "foot height", fill=FOREGROUND, font=f_text)
    draw.text((10, plot_y + 22), "above floor", fill=MUTED, font=f_small)
    for value in range(0, int(top_value) + 1, 30):
        y = plot_y + plot_h - 4 - int(value * scale)
        draw.line((left, y, left + len(ticks) * px, y), fill=(44, 48, 58))
        draw.text((left - 30, y - 7), f"{value}", fill=MUTED, font=f_small)

    def curve(values: list[float], colour) -> None:
        points = [(x_of(r.tick) + px // 2, plot_y + plot_h - 4 - int((v - base) * scale))
                  for r, v in zip(ticks, values)]
        draw.line(points, fill=colour, width=2)

    curve([r.donut_foot_y for r in ticks], DONUT_LINE)
    curve([r.hero_foot_y for r in ticks], CARL_LINE)
    draw.text((left + 8, plot_y + 6), "Carl", fill=CARL_LINE, font=f_small)
    draw.text((left + 48, plot_y + 6), "Donut (32 px behind on Carl's path)", fill=DONUT_LINE, font=f_small)

    # Markers: Carl above, Donut below.
    for group in markers:
        for name, key in (("jump input", "takeoff"), ("apex", "apex"), ("touchdown", "touchdown")):
            tick = group[key]
            if not first <= tick <= last:
                continue
            x = x_of(tick) + px // 2
            colour = MARKER_COLOURS[name]
            draw.line((x, top + 68, x, plot_y + plot_h), fill=colour, width=2)
            draw.text((x + 3, top + 70), f"{name} t={tick}", fill=colour, font=f_small)
    for name, tick in donut_marks:
        if not first <= tick <= last:
            continue
        x = x_of(tick) + px // 2
        colour = MARKER_COLOURS[name]
        draw.line((x, donut_y + bar_h + 2, x, donut_y + bar_h + 14), fill=colour, width=3)
        draw.text((x - 30, donut_y + bar_h + 14), f"Donut {name} t={tick}", fill=colour, font=f_small)

    # Tick axis: minor every 6 ticks (0.1 s), major every 30 (0.5 s).
    draw.line((left, axis_y, left + len(ticks) * px, axis_y), fill=MUTED)
    for tick in range(first - first % 6, last + 1):
        if tick < first:
            continue
        x = x_of(tick)
        if tick % 6 == 0:
            major = tick % 30 == 0
            draw.line((x, axis_y, x, axis_y + (8 if major else 4)), fill=MUTED)
            if major:
                draw.text((x - 12, axis_y + 10), f"{tick}", fill=FOREGROUND, font=f_small)
                draw.text((x - 12, axis_y + 24), f"{tick / 60:.1f}s", fill=MUTED, font=f_small)
    draw.text((10, axis_y + 10), "fixed tick", fill=FOREGROUND, font=f_text)
    image.save(out, optimize=True)


# ----------------------------------------------------------------------- main --

def write_sums(out: Path) -> None:
    lines = []
    for path in sorted(out.iterdir()):
        if path.name in ("SHA256SUMS", "README.md") or not path.is_file():
            continue
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
    (out / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def run(args: argparse.Namespace) -> None:
    log_text = args.log.read_text(encoding="utf-8")
    trace_text = args.trace.read_text(encoding="utf-8")
    captures = parse_captures(log_text)
    rows = parse_trace(trace_text)
    check_log_against_trace(captures, rows)
    for capture in captures:
        if not (args.frames / capture.file).is_file():
            raise EvidenceError(f"missing frame {capture.file}")
    args.out.mkdir(parents=True, exist_ok=True)

    first, last = captures[0].tick, captures[-1].tick
    running = jump_markers(rows, first)
    standing_start = next(t for t in takeoffs(rows) if t > running["touchdown"] + 30)
    standing = jump_markers(rows, standing_start)
    stills = {
        "still_mid_run.png": running["takeoff"] - 12,
        "still_jump_start.png": running["takeoff"] + 1,
        "still_apex.png": running["apex"],
        "still_landing.png": running["touchdown"],
    }
    chosen = {}
    for name, tick in stills.items():
        capture = nearest_capture(captures, tick)
        shutil.copyfile(args.frames / capture.file, args.out / name)
        chosen[name] = capture
    build_gif(args.frames, captures, args.out / "run_jump_1080p.gif")
    draw_timeline(rows, first, last, [running, standing],
                  donut_events(rows, first, last), args.out / "timeline_run_jump.png")
    (args.out / "capture_log.txt").write_text(
        state_lines(log_text, args.frames, "frames"), encoding="utf-8")
    for spec in args.strip or []:
        name, _, rest = spec.partition("=")
        directory, _, log_path = rest.partition(":")
        if not (name and directory and log_path):
            raise EvidenceError(f"--strip expects NAME=FRAMES_DIR:LOG, got {spec!r}")
        strip_text = Path(log_path).read_text(encoding="utf-8")
        strip_captures = parse_captures(strip_text)
        check_log_against_trace(strip_captures, rows)
        for capture in strip_captures:
            if not (Path(directory) / capture.file).is_file():
                raise EvidenceError(f"missing strip frame {capture.file}")
        build_strip(Path(directory), strip_captures, args.out / f"strip_{name}.png")
        (args.out / f"capture_log_strip_{name}.txt").write_text(
            state_lines(strip_text, Path(directory), f"frames_{name}"), encoding="utf-8")
    (args.out / "anim_trace.txt").write_text(
        "\n".join(line for line in trace_text.splitlines()
                  if line.startswith(("FLOOR1_ANIM_TRACE", "FLOOR1_SCENE_STATE", "FLOOR1_ANIM_STATE")))
        + "\n", encoding="utf-8")
    write_sums(args.out)
    print(f"ANIM_INT_EVIDENCE_OK: {len(captures)} frames, ticks {first}..{last}, "
          f"running jump takeoff {running['takeoff']} apex {running['apex']} "
          f"touchdown {running['touchdown']}; stills "
          + ", ".join(f"{n}=tick {c.tick} ({c.carl}/{c.donut})" for n, c in chosen.items()))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    parser.add_argument("--frames", type=Path, required=True, help="directory of captured PNG frames")
    parser.add_argument("--log", type=Path, required=True, help="stdout of the capture run")
    parser.add_argument("--trace", type=Path, required=True, help="stdout of the --anim-trace run")
    parser.add_argument("--out", type=Path, required=True, help="evidence output directory")
    parser.add_argument("--strip", action="append", metavar="NAME=FRAMES_DIR:LOG",
                        help="per-tick strip capture (repeatable), e.g. takeoff=dir:strip.log")
    try:
        run(parser.parse_args())
    except (EvidenceError, OSError, StopIteration) as error:
        print(f"ANIM_INT_EVIDENCE_FAILURE: {error!r}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
