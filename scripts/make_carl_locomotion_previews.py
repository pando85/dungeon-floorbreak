#!/usr/bin/env python3
"""Review previews for the Carl 52 px locomotion candidate (ANIM-CARL-01).

Usage (after ``bash scripts/export_assets.sh`` has exported the atlas):

    python3 scripts/make_carl_locomotion_previews.py [--out DIR] [--no-floor] [--run-speed PX_S]

Reads assets/manifests/carl_book1_h52_locomotion.json and its exported atlas +
metadata and writes, deterministically (no timestamps), into DIR (default
generated/art/carl_locomotion): contact sheets (x1, x6 with tag/duration headers and
pivot marks), a run stride diagram (frames 4-9 side by side with ground line, pivot and the
foot-contact x of every frame), idle/run/jump GIFs (jump follows the greybox physics read from
src/prototype.rs), foot-slip run-scroll GIFs, lit Floor 1 compositions made with the
existing env_kit_light compositor (skipped by --no-floor) and measurements.json
(per-frame bbox/pixel counts, per-tag durations, sha256 of every written file).
Presentation-only art preview; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
import time
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_assets import safe_path  # noqa: E402

MANIFEST = "assets/manifests/carl_book1_h52_locomotion.json"
LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
DEFAULT_OUT = ROOT / "generated" / "art" / "carl_locomotion"
SAMPLE_MS = 20  # GIF sampling step for physics / scroll previews
TICK_SPACING = 16  # px between ground tick marks in the run-scroll strips
# Floor 1 lineup: (x on the walking line, atlas frame index, lift in px). Donut is staged
# separately at her layout preview position. x values must stay >= MIN_SPACING apart.
LINEUP = [(372, 4, 0), (530, 6, 0), (616, 10, 0), (702, 12, 36), (788, 13, 18), (880, 14, 0)]
MIN_SPACING = 56
BG_SHEET1, BG_SHEET6, BG_GIF = (25, 26, 44), (42, 40, 58), (37, 35, 55)
GROUND_LIGHT, GROUND_DARK, TICK = (74, 68, 100), (52, 48, 74), (120, 112, 150)
CYAN, YELLOW = (0, 255, 255), (255, 224, 0)
STRIDE_PX = 8  # ground travel of the planted foot per run frame (ANIM-CARL-01 run spec)
RUN_MS_MIN, RUN_MS_MAX = 50, 90


def run_frame_ms(speed_px_s: float) -> int:
    """Speed-linked run playback rule: frame_ms = clamp(round(8000 / |vx|), 50, 90)."""
    if speed_px_s == 0:
        return RUN_MS_MAX
    return max(RUN_MS_MIN, min(RUN_MS_MAX, round(STRIDE_PX * 1000 / abs(speed_px_s))))


class PreviewError(Exception):
    pass


def greybox_constants() -> dict[str, float]:
    text = (ROOT / "src" / "prototype.rs").read_text(encoding="utf-8")
    out = {}
    for name in ("RUN_SPEED", "JUMP_SPEED", "GRAVITY"):
        found = re.search(rf"const {name}: f32 = ([0-9.]+);", text)
        if not found:
            raise PreviewError(f"src/prototype.rs: constant {name} not found")
        out[name] = float(found.group(1))
    return out


def load_atlas(manifest_path: str) -> tuple[dict, list[Image.Image], list[int], dict[str, tuple[int, int]]]:
    manifest = json.loads(safe_path(manifest_path).read_text(encoding="utf-8"))
    atlas_path, meta_path = safe_path(manifest["output_atlas"]), safe_path(manifest["output_metadata"])
    if not atlas_path.is_file() or not meta_path.is_file():
        raise PreviewError(f"exported atlas missing for {manifest['id']}: run scripts/export_assets.sh first")
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    with Image.open(atlas_path) as opened:
        atlas = opened.convert("RGBA")
    images, durations = [], []
    for item in meta["frames"]:
        r = item["frame"]
        images.append(atlas.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"])))
        durations.append(int(item["duration"]))
    tags = {t["name"]: (t["from"], t["to"]) for t in meta["meta"]["frameTags"]}
    return manifest, images, durations, tags


def save_gif(path: Path, images: list[Image.Image], durations: list[int]) -> None:
    """Looping GIF; consecutive identical frames are merged (durations summed)."""
    frames, times = [], []
    for img, dur in zip(images, durations):
        if frames and frames[-1].tobytes() == img.tobytes():
            times[-1] += dur
        else:
            frames.append(img)
            times.append(dur)
    frames[0].save(path, format="GIF", save_all=True, append_images=frames[1:], duration=times,
                   loop=0, optimize=False, disposal=2)


# ---------------------------------------------------------------- contact sheets

def contact_sheet_x1(frames: list[Image.Image]) -> Image.Image:
    w, h = frames[0].size
    sheet = Image.new("RGBA", (len(frames) * (w + 1) - 1, h), BG_SHEET1 + (255,))
    for i, f in enumerate(frames):
        sheet.alpha_composite(f, (i * (w + 1), 0))
    return sheet.convert("RGB")


def contact_sheet_x6(frames, durations, tags, pivot, s: int = 6) -> Image.Image:
    w, h = frames[0].size
    head, foot, gap = 14, 10, 8
    cell_w, cell_h = w * s, head + h * s + foot
    cols = max(t - f + 1 for f, t in tags.values())
    sheet = Image.new("RGB", (cols * (cell_w + gap) + gap, len(tags) * (cell_h + gap) + gap), BG_SHEET6)
    draw, font = ImageDraw.Draw(sheet), ImageFont.load_default()
    for row, (name, (first, last)) in enumerate(tags.items()):
        for col, i in enumerate(range(first, last + 1)):
            ox, oy = gap + col * (cell_w + gap), gap + row * (cell_h + gap)
            draw.text((ox, oy), f"{name} #{i} {durations[i]}ms", fill=(226, 214, 200), font=font)
            big = frames[i].resize((cell_w, h * s), Image.Resampling.NEAREST)
            cell = Image.new("RGBA", big.size, BG_SHEET6 + (255,))
            cell.alpha_composite(big)
            sheet.paste(cell.convert("RGB"), (ox, oy + head))
            line_y = oy + head + (pivot[1] + 1) * s  # directly under the pivot row
            draw.line([(ox, line_y), (ox + cell_w - 1, line_y)], fill=CYAN, width=1)
            cx = ox + pivot[0] * s + s // 2
            draw.rectangle([cx - 2, line_y + 3, cx + 2, line_y + 7], fill=YELLOW)
    return sheet


# ---------------------------------------------------------------- stride diagram

def foot_contact(frame: Image.Image, row: int) -> tuple[int, int] | None:
    """(first, last) opaque x on the contact row, or None when the foot is off the ground."""
    alpha = frame.getchannel("A")
    xs = [x for x in range(frame.width) if alpha.getpixel((x, row))]
    return (xs[0], xs[-1]) if xs else None


def stride_diagram(frames: list[Image.Image], tags, pivot, s: int = 8) -> Image.Image:
    """Run frames side by side with a ground line, the pivot and each frame's row-55 foot contact."""
    first, last = tags["run"]
    w, h = frames[0].size
    pad, top, bottom = 10, 26, 64
    cell_w = w * s + pad
    img = Image.new("RGB", (pad + (last - first + 1) * cell_w, top + h * s + bottom), BG_SHEET6)
    d, font = ImageDraw.Draw(img), ImageFont.load_default()
    ground = top + (pivot[1] + 1) * s
    for col, i in enumerate(range(first, last + 1)):
        ox = pad + col * cell_w
        big = frames[i].resize((w * s, h * s), Image.Resampling.NEAREST)
        cell = Image.new("RGBA", big.size, BG_SHEET6 + (255,))
        cell.alpha_composite(big)
        img.paste(cell.convert("RGB"), (ox, top))
        d.text((ox, 6), f"run frame {i}", fill=(226, 214, 200), font=font)
        px = ox + pivot[0] * s + s // 2  # pivot column centre
        d.line([(px, top), (px, ground + 12)], fill=YELLOW, width=1)
        d.rectangle([px - 3, ground + 4, px + 3, ground + 10], fill=YELLOW)
        contact = foot_contact(frames[i], pivot[1])
        if contact:
            c0, c1 = contact
            d.rectangle([ox + c0 * s, ground + 14, ox + (c1 + 1) * s - 1, ground + 18], fill=CYAN)
            d.text((ox, ground + 24), f"row {pivot[1]}: x{c0}-{c1}", fill=CYAN, font=font)
            d.text((ox, ground + 38), f"centre {(c0 + c1) / 2:.1f} ({(c0 + c1) / 2 - pivot[0]:+.1f} from pivot)",
                   fill=CYAN, font=font)
        else:
            d.text((ox, ground + 24), f"row {pivot[1]}: no foot", fill=CYAN, font=font)
    d.line([(0, ground), (img.width - 1, ground)], fill=CYAN, width=1)
    d.text((pad, ground + 50), f"yellow = pivot x{pivot[0]}, cyan line = ground (row {pivot[1]}), bar = foot pixels on the ground row",
           fill=(226, 214, 200), font=font)
    return img


# ---------------------------------------------------------------- GIF scenes

def scene(sprite: Image.Image, pivot, size, ground_y: int, lift: int = 0, x_center: int | None = None,
          scroll_px: int | None = None) -> Image.Image:
    """Sprite on a flat 2-tone ground strip under pivot row ``ground_y`` (1x content)."""
    w, h = size
    img = Image.new("RGBA", size, BG_GIF + (255,))
    d = ImageDraw.Draw(img)
    d.rectangle([0, ground_y + 1, w - 1, min(h - 1, ground_y + 2)], fill=GROUND_LIGHT)
    d.rectangle([0, ground_y + 3, w - 1, h - 1], fill=GROUND_DARK)
    if scroll_px is not None:
        for x in range(-scroll_px % TICK_SPACING, w, TICK_SPACING):
            d.line([(x, ground_y + 1), (x, h - 1)], fill=TICK)
    cx = w // 2 if x_center is None else x_center
    img.alpha_composite(sprite, (cx - pivot[0], ground_y - pivot[1] - lift))
    return img.convert("RGB")


def up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def jump_timeline(durations, tags, consts) -> list[tuple[int, float, int]]:
    """(frame index, lift px, duration ms): start -> rise -> fall -> land -> idle hold."""
    v, g = consts["JUMP_SPEED"], consts["GRAVITY"]
    s0, s1 = tags["jump_start"]
    rise, fall = tags["jump_rise"][0], tags["jump_fall"][0]
    out = [(s0, 0.0, durations[s0])]  # anticipation on the ground
    n = 0
    while True:  # impulse starts when jump_start frame 1 begins
        t = n * SAMPLE_MS / 1000.0
        y, vy = v * t - g * t * t / 2, v - g * t
        if n * SAMPLE_MS >= durations[s1] and y <= 0:
            break
        frame = s1 if n * SAMPLE_MS < durations[s1] else (rise if vy > 0 else fall)
        out.append((frame, max(y, 0.0), SAMPLE_MS))
        n += 1
    land0, land1 = tags["land"]
    out += [(i, 0.0, durations[i]) for i in range(land0, land1 + 1)]
    return out + [(tags["idle"][0], 0.0, 400)]


def make_gifs(out: Path, frames, durations, tags, pivot, consts, run_speed: float, written: list[Path]) -> None:
    w, h = frames[0].size
    apex = math.ceil(consts["JUMP_SPEED"] ** 2 / (2 * consts["GRAVITY"]))
    size_flat = (w + 24, h + 11)
    ground_jump = apex + h + 4
    size_jump = (w + 24, ground_jump + 8)

    def put(name: str, imgs, durs) -> None:
        path = out / name
        save_gif(path, imgs, durs)
        written.append(path)

    for tag in ("idle", "run"):
        a, b = tags[tag]
        imgs = [up(scene(frames[i], pivot, size_flat, pivot[1] + 3), 4) for i in range(a, b + 1)]
        put(f"{tag}_x4.gif", imgs, [durations[i] for i in range(a, b + 1)])
    timeline = jump_timeline(durations, tags, consts)
    for k in (4, 1):
        imgs = [up(scene(frames[f], pivot, size_jump, ground_jump - 1, round(lift)), k) for f, lift, _ in timeline]
        put(f"jump_sequence_x{k}.gif", imgs, [d for _, _, d in timeline])
    a, b = tags["run"]
    n_run = b - a + 1
    for speed, name in ((run_speed, "run_scroll_x2.gif"), (145.0, "run_scroll_145_x2.gif")):
        step = run_frame_ms(speed)  # playback rule, not the authored 60 ms
        cycle = step * n_run
        imgs, durs = [], []
        for t in range(0, 2 * cycle, SAMPLE_MS):
            idx = a + (t % cycle) // step
            imgs.append(up(scene(frames[idx], pivot, (320, 60), pivot[1], 0, scroll_px=round(speed * t / 1000.0)), 2))
            durs.append(SAMPLE_MS)
        put(name, imgs, durs)


# ---------------------------------------------------------------- Floor 1

def render_floor(out: Path, frames, pivot, written: list[Path]) -> None:
    import env_kit as ek
    import env_kit_light as ekl
    layout = ek.load_json(LAYOUT)
    kit = ek.load_kit(layout["kit_contract"])
    ek.validate_contract(kit)
    ek.validate_layout(kit, layout)
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    view = layout["views"][layout["preview"]["primary_view"]]
    carl_check, donut_check = layout["preview"]["actor_check"][:2]
    donut_manifest, donut_frames, _, _ = load_atlas(donut_check["manifest"])
    donut_pivot = tuple(donut_manifest["pivot"])

    def snap(x: int) -> int:  # nearest column with a walking line (flat floor: normally a no-op)
        for d in sorted(range(-48, 49), key=abs):
            if view[0] + 24 <= x + d < view[0] + view[2] - 24 and isinstance(tops[(x + d) // ek.TILE], int):
                return x + d
        raise PreviewError(f"no walking line near x={x}")

    def stage(entries):  # (sprite, pivot, x, lift) -> actor layer + ground-level boxes for the shadows
        layer = Image.new("RGBA", tuple(layout["pixel_size"]), (0, 0, 0, 0))
        boxes = []
        for sprite, piv, x, lift in entries:
            pos = (x - piv[0], tops[x // ek.TILE] - 1 - piv[1] - lift)
            layer.alpha_composite(sprite, pos)
            boxes.append((pos[0], pos[1] + lift, pos[0] + sprite.width, pos[1] + lift + sprite.height))
        return layer, boxes

    donut = (donut_frames[0], donut_pivot, donut_check["x"], 0)
    lineup = [(frames[i], pivot, snap(x), lift) for x, i, lift in LINEUP]
    xs = sorted([e[2] for e in lineup] + [donut[2]])
    if any(b - a < MIN_SPACING for a, b in zip(xs, xs[1:])):
        raise PreviewError(f"lineup actors closer than {MIN_SPACING} px: {xs}")
    sets = {"lineup": [donut] + lineup, "run": [(frames[4], pivot, carl_check["x"], 0), donut]}
    for name, entries in sets.items():
        actors, boxes = stage(entries)
        for k, tag in ((1, "640x360"), (3, "1920x1080")):
            start = time.perf_counter()
            img = ekl.render_lit(kit, layout, view, k, actors, boxes=boxes)
            assert img.size == (view[2] * k, view[3] * k)
            path = out / f"floor01_{name}_{tag}.png"
            img.save(path, format="PNG", optimize=False)
            written.append(path)
            print(f"  {path.name}: {time.perf_counter() - start:.1f} s")


# ---------------------------------------------------------------- measurements

def measure(frames, durations, tags) -> dict:
    out = {"frames": [], "tag_duration_ms": {}}
    names = {i: n for n, (a, b) in tags.items() for i in range(a, b + 1)}
    for i, f in enumerate(frames):
        alpha = f.getchannel("A")
        x0, y0, x1, y1 = alpha.getbbox()
        out["frames"].append({
            "index": i, "tag": names[i], "duration_ms": durations[i], "bbox": [x0, y0, x1 - 1, y1 - 1],
            "opaque_width": x1 - x0, "opaque_height": y1 - y0,
            "opaque_pixels": sum(1 for p in alpha.tobytes() if p), "lowest_opaque_row": y1 - 1})
    for n, (a, b) in tags.items():
        out["tag_duration_ms"][n] = sum(durations[a:b + 1])
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--no-floor", action="store_true", help="skip the lit Floor 1 renders")
    parser.add_argument("--run-speed", type=float, default=None, help="run-scroll speed px/s (default: RUN_SPEED)")
    args = parser.parse_args(argv)
    try:
        consts = greybox_constants()
        manifest, frames, durations, tags = load_atlas(MANIFEST)
        pivot = tuple(manifest["pivot"])
        out = args.out
        out.mkdir(parents=True, exist_ok=True)
        written: list[Path] = []
        contact_sheet_x1(frames).save(out / "contact_sheet_x1.png")
        contact_sheet_x6(frames, durations, tags, pivot).save(out / "contact_sheet_x6.png")
        stride_diagram(frames, tags, pivot).save(out / "run_stride_diagram.png")
        written += [out / "contact_sheet_x1.png", out / "contact_sheet_x6.png", out / "run_stride_diagram.png"]
        make_gifs(out, frames, durations, tags, pivot, consts, args.run_speed or consts["RUN_SPEED"], written)
        if not args.no_floor:
            render_floor(out, frames, pivot, written)
        report = measure(frames, durations, tags)
        report.update({"manifest": MANIFEST, "greybox": consts,
                       "sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(written)}})
        (out / "measurements.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print("Carl locomotion previews:\n" + "\n".join(f"  {p}" for p in sorted(written)))
        return 0
    except (PreviewError, OSError, KeyError, ValueError, TypeError) as exc:
        print(f"CARL_LOCOMOTION_PREVIEW_ERROR: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
