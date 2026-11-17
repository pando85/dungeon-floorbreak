#!/usr/bin/env python3
"""Review previews for the Carl 52 px unarmed melee candidate (ANIM-CARL-02).

Usage (after ``bash scripts/export_assets.sh`` has exported the atlas):

    python3 scripts/make_carl_melee_previews.py [--out DIR] [--no-floor]

Reads assets/manifests/carl_book1_h52_melee.json, its exported atlas and the approved
locomotion atlas (idle reference) and writes, deterministically, into DIR (default
generated/art/carl_melee): a key-pose sheet at native size and x6 (move, tick window,
hit tick, pivot, floor line and the art reach boxes from the manifest), a tick timing
diagram drawn from the manifest ``melee.moves`` table, lit Floor 1 compositions of the
kick contact next to two grey goblin-sized placeholder silhouettes (640 x 360 and exact
3x 1920 x 1080, skipped by --no-floor) and measurements.json. Presentation-only art
preview; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))

import make_carl_locomotion_previews as loco  # noqa: E402

ROOT = loco.ROOT
MANIFEST = "assets/manifests/carl_book1_h52_melee.json"
LOCO_MANIFEST = "assets/manifests/carl_book1_h52_locomotion.json"
DEFAULT_OUT = ROOT / "generated" / "art" / "carl_melee"
MAGENTA, RED, WHITE = (255, 70, 200), (255, 72, 72), (226, 214, 200)
PHASE_COLOURS = {"startup": (226, 168, 60), "active": (232, 64, 64), "recovery": (92, 104, 140),
                 "reaction": (150, 96, 200), "airborne": (60, 170, 170)}

# Goblin-sized grey placeholder (26 x 40 px, facing left, pivot on its feet). Not goblin art:
# a neutral scale/contact silhouette for the review mock only. o = outline, g = fill.
GOBLIN = [
    "........oooooooo..........",
    "......oogggggggggo........",
    ".....ogggggggggggggo......",
    "o...ogggggggggggggggo...o.",
    "oo..oggggggggggggggggo.oo.",
    ".oooggggggggggggggggggoo..",
    "..oggggggggggggggggggggo..",
    "..oggggggggggggggggggggo..",
    ".ooggggggggggggggggggggo..",
    "ogggggggggggggggggggggo...",
    "ogggggggggggggggggggggo...",
    ".oooggggggggggggggggggo...",
    "....ogggggggggggggggggo...",
    ".....oggggggggggggggggo...",
    "....ogggggggggggggggggo...",
    "...ogggggggggggggggggggo..",
    "..ogggggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    ".ogggoggggggggggggggggggo.",
    "..ooo.oggggggggggggggggo..",
    "......oggggggggggggggggo..",
    "......oggggggggggggggggo..",
    ".......oggggggggggggggo...",
    "........oggggggggggggo....",
    "........ogggggoogggggo....",
    "........oggggo..oggggo....",
    "........oggggo..oggggo....",
    "........oggggo..oggggo....",
    "........oggggo..oggggo....",
    "........oggggo..oggggo....",
    "........oggggo..oggggo....",
    "......ooggggo.ooggggo.....",
    ".....ogggggggoggggggo.....",
    ".....ogggggggoggggggo.....",
    "......ooooooo.oooooo......",
]
GOBLIN_PIVOT = (13, 39)
GOBLIN_FRONT_X = 1  # leftmost body column at the kick's contact height (rows 12-21)
GOBLIN_GAP = 30  # px between the two goblin pivots (CP-02: "two goblins close together")
CARL_X = 520  # level x of Carl's pivot in the Floor 1 mock


def goblin_sprite() -> Image.Image:
    img = Image.new("RGBA", (len(GOBLIN[0]), len(GOBLIN)), (0, 0, 0, 0))
    colours = {"o": (58, 58, 68, 255), "g": (128, 128, 138, 255)}
    for y, row in enumerate(GOBLIN):
        for x, ch in enumerate(row):
            if ch in colours:
                img.putpixel((x, y), colours[ch])
    return img


def frame_labels(manifest: dict) -> list[tuple[str, str]]:
    """(title, timing line) per atlas frame of the phase-1 key atlas."""
    moves = manifest["melee"]["moves"]
    keys = manifest["melee"]["phase1_keys"]
    lookup = {"punch_a_contact": ("punch_a", "a1 contact"), "kick_windup": ("kick", "k1 wind-up"),
              "kick_contact": ("kick", "k3 contact"), "hurt_impact": ("hurt", "h0 impact")}
    out = [("", "")] * len(keys)
    for key, index in keys.items():
        move, name = lookup[key]
        frame = next(f for f in moves[move]["frames"] if f["name"] == name)
        a, b = frame["ticks"]
        hit = moves[move]["hit_tick"]
        timing = f"t{a}-{b} ({b - a + 1} ticks)" + (f" hit t{hit}" if hit is not None and a <= hit <= b else "")
        out[index] = (f"{move}: {name}", timing)
    return out


def key_sheet_x1(idle: Image.Image, frames: list[Image.Image]) -> Image.Image:
    images = [idle] + frames
    w, h = images[0].size
    sheet = Image.new("RGBA", (len(images) * (w + 1) - 1, h), loco.BG_SHEET1 + (255,))
    for i, f in enumerate(images):
        sheet.alpha_composite(f, (i * (w + 1), 0))
    return sheet.convert("RGB")


def key_sheet_x6(idle, frames, manifest, s: int = 6) -> Image.Image:
    images = [idle] + frames
    labels = [("idle (approved ref)", "locomotion frame 0")] + frame_labels(manifest)
    boxes = manifest["melee"]["hit_boxes_px"]
    keys = {v: k for k, v in manifest["melee"]["phase1_keys"].items()}
    pivot = manifest["pivot"]
    w, h = images[0].size
    head, foot, gap = 30, 12, 8
    cell_w, cell_h = w * s, head + h * s + foot
    sheet = Image.new("RGB", (len(images) * (cell_w + gap) + gap, cell_h + 2 * gap), loco.BG_SHEET6)
    draw, font = ImageDraw.Draw(sheet), ImageFont.load_default()
    for col, img in enumerate(images):
        ox, oy = gap + col * (cell_w + gap), gap
        draw.text((ox, oy), labels[col][0], fill=WHITE, font=font)
        draw.text((ox, oy + 13), labels[col][1], fill=WHITE, font=font)
        big = img.resize((cell_w, h * s), Image.Resampling.NEAREST)
        cell = Image.new("RGBA", big.size, loco.BG_SHEET6 + (255,))
        cell.alpha_composite(big)
        sheet.paste(cell.convert("RGB"), (ox, oy + head))
        line_y = oy + head + (pivot[1] + 1) * s
        draw.line([(ox, line_y), (ox + cell_w - 1, line_y)], fill=loco.CYAN, width=1)
        cx = ox + pivot[0] * s + s // 2
        draw.rectangle([cx - 2, line_y + 3, cx + 2, line_y + 7], fill=loco.YELLOW)
        for name, (x0, y0, x1, y1) in boxes.get(keys.get(col - 1, ""), {}).items():
            # reach boxes are pivot-relative (x forward, y up, inclusive) -> sprite pixels
            left, right = (pivot[0] + x0) * s, (pivot[0] + x1 + 1) * s - 1
            top, bottom = (pivot[1] - y1) * s, (pivot[1] - y0 + 1) * s - 1
            colour = RED if name in ("fist", "sole") else MAGENTA
            draw.rectangle([ox + left, oy + head + top, ox + right, oy + head + bottom], outline=colour)
    return sheet


def timing_diagram(manifest: dict, px: int = 14) -> Image.Image:
    moves = manifest["melee"]["moves"]
    keyed = [(n, m) for n, m in moves.items() if any(f["ticks"] for f in m["frames"])]
    label_w, row_h, top = 150, 46, 40
    span = max(m["total_ticks"] for _, m in keyed)
    width = label_w + span * px + 40
    img = Image.new("RGB", (width, top + len(keyed) * row_h + 60), loco.BG_SHEET6)
    d, font = ImageDraw.Draw(img), ImageFont.load_default()
    d.text((8, 8), "ANIM-CARL-02 tick timeline (60 Hz fixed step, tick 0 = input accepted)", fill=WHITE, font=font)
    for t in range(0, span + 1, 6):
        x = label_w + t * px
        d.line([(x, top - 6), (x, top + len(keyed) * row_h)], fill=(70, 66, 92))
        d.text((x - 4, top - 18), str(t), fill=WHITE, font=font)
    for r, (name, move) in enumerate(keyed):
        y = top + r * row_h + 8
        d.text((8, y + 4), f"{name} ({move['total_ticks']} t)", fill=WHITE, font=font)
        for f in move["frames"]:
            if not f["ticks"]:
                continue
            a, b = f["ticks"]
            x0, x1 = label_w + a * px, label_w + (b + 1) * px - 1
            d.rectangle([x0, y, x1, y + 16], fill=PHASE_COLOURS[f["phase"]], outline=(20, 18, 30))
            if f["key"]:
                d.rectangle([x0 + 1, y + 1, x1 - 1, y + 15], outline=loco.YELLOW)
            d.text((x0 + 2, y + 19), f["name"].split()[0], fill=WHITE, font=font)
        if move["active_ticks"]:
            a, b = move["active_ticks"]
            d.line([(label_w + a * px, y - 3), (label_w + (b + 1) * px - 1, y - 3)], fill=RED, width=2)
        if move["hit_tick"] is not None:
            hx = label_w + move["hit_tick"] * px
            d.polygon([(hx, y - 1), (hx - 4, y - 8), (hx + 4, y - 8)], fill=RED)
        cancel = move["cancel"]
        if cancel["buffer_ticks"]:
            a, b = cancel["buffer_ticks"]
            yy = y + 33
            d.line([(label_w + a * px, yy), (label_w + (b + 1) * px - 1, yy)], fill=loco.CYAN, width=1)
            sx = label_w + cancel["earliest_start_tick"] * px
            d.line([(sx, yy - 3), (sx, yy + 3)], fill=loco.CYAN, width=2)
    ly = top + len(keyed) * row_h + 12
    x = 8
    for phase, colour in PHASE_COLOURS.items():
        if phase == "airborne":
            continue
        d.rectangle([x, ly, x + 12, ly + 10], fill=colour)
        d.text((x + 16, ly), phase, fill=WHITE, font=font)
        x += 90
    d.text((8, ly + 18), "yellow box = Opus key | red bar = active ticks, red marker = hit tick | "
           "cyan = cancel buffer, tick = earliest chained start", fill=WHITE, font=font)
    d.text((8, ly + 32), "Hit-stop pauses this clock (contact frame held); knockback hold frame is "
           "state-driven, not timed.", fill=WHITE, font=font)
    return img


def render_floor(out: Path, manifest: dict, kick: Image.Image, pivot, written: list[Path]) -> None:
    import env_kit as ek
    import env_kit_light as ekl
    layout = ek.load_json(loco.LAYOUT)
    kit = ek.load_kit(layout["kit_contract"])
    ek.validate_contract(kit)
    ek.validate_layout(kit, layout)
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    view = layout["views"][layout["preview"]["primary_view"]]
    goblin = goblin_sprite()
    reach = manifest["melee"]["hit_boxes_px"]["kick_contact"]["sole"][2]  # sole tip, px ahead of pivot
    gx_a = CARL_X + reach + 1 + (GOBLIN_PIVOT[0] - GOBLIN_FRONT_X)
    entries = [(kick, pivot, CARL_X), (goblin, GOBLIN_PIVOT, gx_a), (goblin, GOBLIN_PIVOT, gx_a + GOBLIN_GAP)]
    layer = Image.new("RGBA", tuple(layout["pixel_size"]), (0, 0, 0, 0))
    boxes = []
    for sprite, piv, x in entries:
        ground = tops[x // ek.TILE]
        if not isinstance(ground, int):
            raise loco.PreviewError(f"no walking line at x={x}")
        pos = (x - piv[0], ground - 1 - piv[1])
        layer.alpha_composite(sprite, pos)
        boxes.append((pos[0], pos[1], pos[0] + sprite.width, pos[1] + sprite.height))
    for k, tag in ((1, "640x360"), (3, "1920x1080")):
        img = ekl.render_lit(kit, layout, view, k, layer, boxes=boxes)
        path = out / f"floor01_kick_{tag}.png"
        img.save(path, format="PNG", optimize=False)
        written.append(path)
        if k == 3:  # 160 x 90 virtual-px window around the action, from the 3x render, shown at x6
            left = (CARL_X - 48 - view[0]) * k
            top = (tops[CARL_X // ek.TILE] - 70 - view[1]) * k
            crop = img.crop((left, top, left + 160 * k, top + 90 * k))
            crop = crop.resize((crop.width * 2, crop.height * 2), Image.Resampling.NEAREST)
            path = out / "floor01_kick_crop_x6.png"
            crop.save(path, format="PNG", optimize=False)
            written.append(path)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--no-floor", action="store_true", help="skip the lit Floor 1 renders")
    args = parser.parse_args(argv)
    try:
        manifest, frames, durations, tags = loco.load_atlas(MANIFEST)
        _, loco_frames, _, _ = loco.load_atlas(LOCO_MANIFEST)
        pivot = tuple(manifest["pivot"])
        out = args.out
        out.mkdir(parents=True, exist_ok=True)
        written: list[Path] = []
        for name, img in (("key_poses_x1.png", key_sheet_x1(loco_frames[0], frames)),
                          ("key_poses_x6.png", key_sheet_x6(loco_frames[0], frames, manifest)),
                          ("timing_diagram.png", timing_diagram(manifest))):
            img.save(out / name, format="PNG", optimize=False)
            written.append(out / name)
        if not args.no_floor:
            render_floor(out, manifest, frames[manifest["melee"]["phase1_keys"]["kick_contact"]], pivot, written)
        report = loco.measure(frames, durations, tags)
        report.update({"manifest": MANIFEST,
                       "sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(written)}})
        (out / "measurements.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print("Carl melee previews:\n" + "\n".join(f"  {p}" for p in sorted(written)))
        return 0
    except (loco.PreviewError, OSError, KeyError, ValueError, TypeError) as exc:
        print(f"CARL_MELEE_PREVIEW_ERROR: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
