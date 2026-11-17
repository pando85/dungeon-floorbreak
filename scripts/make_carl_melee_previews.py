#!/usr/bin/env python3
"""Review previews for the Carl 52 px unarmed melee candidate (ANIM-CARL-02).

Usage (after ``bash scripts/export_assets.sh`` has exported the atlas):

    python3 scripts/make_carl_melee_previews.py [--out DIR] [--no-floor]

Reads assets/manifests/carl_book1_h52_melee.json, its exported atlas and the approved
locomotion atlas (idle reference) and writes, deterministically, into DIR (default
generated/art/carl_melee):

* ``contact_sheet_x1.png`` (idle reference + all 24 frames, native size) and
  ``contact_sheet_x6.png`` (one row per move at x6: frame, tick window, hit tick, phase colour,
  pivot, floor line, Opus keys outlined, art reach boxes from the manifest);
* ``timing_diagram.png``: the tick contract of every move, drawn from ``melee.moves``;
* ``*_x4.gif``: one GIF per move and two strings (jab > haymaker, jab > kick), x4, played at the
  60 Hz tick durations (each frame shows its tick count, quantised to the 10 ms GIF clock at the
  cumulative tick boundaries, so no drift builds up) followed by 24 ticks of the approved idle;
* ``floor01_*``: lit Floor 1 compositions at gameplay scale (640 x 360 and exact 3x 1920 x 1080,
  skipped by --no-floor): the kick contact next to two grey goblin-sized placeholder silhouettes,
  a lineup of the whole move set, and a kick sequence mock with the launch values proposed in
  spec section 4;
* ``measurements.json``.

Presentation-only art preview; nothing here is wired into the game.
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
TICK_HZ = 60
IDLE_TAIL_TICKS = 24     # approved idle shown after each GIF move so the loop boundary reads
KB_PREVIEW_TICKS = 12    # the knockback hold is state-driven; the preview shows 12 ticks of it
GIF_SCALE = 4
GIF_SIZE = (84, 66)      # 1x scene: sprite pivot at x32, ground row 60
GIF_PIVOT_X, GIF_GROUND = 32, 60
# art reach boxes in the manifest -> the atlas frame they were measured on
BOX_FRAME = {"punch_a_contact": "a1", "punch_b_contact": "b1", "kick_contact": "k3"}

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
CARL_X = 520  # level x of Carl's pivot in the Floor 1 kick mocks
# proposed kick launch (spec section 4): vx 300 px/s, vy 220 px/s, GRAVITY 1050 px/s^2
LAUNCH_VX, LAUNCH_VY, GRAVITY = 300.0, 220.0, 1050.0
LINEUP_X = [372, 450, 528, 606, 684, 762, 840]
LINEUP_FRAMES = ["a1", "b1", "k1", "k3", "h1", "d1", "g0"]


def goblin_sprite() -> Image.Image:
    img = Image.new("RGBA", (len(GOBLIN[0]), len(GOBLIN)), (0, 0, 0, 0))
    colours = {"o": (58, 58, 68, 255), "g": (128, 128, 138, 255)}
    for y, row in enumerate(GOBLIN):
        for x, ch in enumerate(row):
            if ch in colours:
                img.putpixel((x, y), colours[ch])
    return img


# ---------------------------------------------------------------- frame table

def frame_table(manifest: dict) -> list[dict]:
    """Atlas frames in order with move, id, name, tick window and phase (from ``melee.moves``)."""
    rows = []
    for move, data in manifest["melee"]["moves"].items():
        for fr in data["frames"]:
            ticks = tuple(fr["ticks"]) if fr["ticks"] else None
            hit = data["hit_tick"]
            rows.append({"move": move, "id": fr["name"].split()[0], "name": fr["name"], "ticks": ticks,
                         "phase": fr["phase"], "key": fr["key"],
                         "hit": ticks is not None and hit is not None and ticks[0] <= hit <= ticks[1]})
    return rows


def frame_timing(row: dict) -> str:
    if row["ticks"] is None:
        return "hold while airborne"
    a, b = row["ticks"]
    return f"t{a}-{b} ({b - a + 1} ticks)" + (" HIT t%d" % a if row["hit"] else "")


def index_of(table: list[dict], fid: str) -> int:
    return next(i for i, r in enumerate(table) if r["id"] == fid)


# ---------------------------------------------------------------- contact sheets

def contact_sheet_x1(idle: Image.Image, frames: list[Image.Image]) -> Image.Image:
    images = [idle] + frames
    w, h = images[0].size
    sheet = Image.new("RGBA", (len(images) * (w + 1) - 1, h), loco.BG_SHEET1 + (255,))
    for i, f in enumerate(images):
        sheet.alpha_composite(f, (i * (w + 1), 0))
    return sheet.convert("RGB")


def contact_sheet_x6(frames: list[Image.Image], table: list[dict], manifest: dict, s: int = 6) -> Image.Image:
    boxes = manifest["melee"]["hit_boxes_px"]
    box_of = {index_of(table, fid): name for name, fid in BOX_FRAME.items()}
    pivot = manifest["pivot"]
    w, h = frames[0].size
    head, foot, gap, band = 30, 12, 8, 14
    cell_w, cell_h = w * s, head + h * s + foot
    moves: dict[str, list[int]] = {}
    for i, r in enumerate(table):
        moves.setdefault(r["move"], []).append(i)
    cols = max(len(v) for v in moves.values())
    sheet = Image.new("RGB", (cols * (cell_w + gap) + gap, len(moves) * (cell_h + band + gap) + gap), loco.BG_SHEET6)
    draw, font = ImageDraw.Draw(sheet), ImageFont.load_default()
    for row, (move, idxs) in enumerate(moves.items()):
        data = manifest["melee"]["moves"][move]
        oy = gap + row * (cell_h + band + gap)
        title = f"{move}: {data['role']} | {data['total_ticks']} ticks"
        if data["hit_tick"] is not None:
            title += f" | active t{data['active_ticks'][0]}-{data['active_ticks'][1]}, hit t{data['hit_tick']}"
        draw.text((gap, oy), title, fill=WHITE, font=font)
        for col, i in enumerate(idxs):
            r = table[i]
            ox = gap + col * (cell_w + gap)
            y0 = oy + band
            draw.rectangle([ox, y0, ox + cell_w - 1, y0 + 4], fill=PHASE_COLOURS[r["phase"]])
            draw.text((ox, y0 + 7), f"{r['name']}" + ("  [OPUS KEY]" if r["key"] else ""), fill=WHITE, font=font)
            draw.text((ox, y0 + 18), f"#{i}  {frame_timing(r)}", fill=RED if r["hit"] else WHITE, font=font)
            big = frames[i].resize((cell_w, h * s), Image.Resampling.NEAREST)
            cell = Image.new("RGBA", big.size, loco.BG_SHEET6 + (255,))
            cell.alpha_composite(big)
            sheet.paste(cell.convert("RGB"), (ox, y0 + head))
            if r["key"]:
                draw.rectangle([ox - 2, y0 - 1, ox + cell_w + 1, y0 + head + h * s + 2], outline=loco.YELLOW)
            line_y = y0 + head + (pivot[1] + 1) * s
            draw.line([(ox, line_y), (ox + cell_w - 1, line_y)], fill=loco.CYAN, width=1)
            cx = ox + pivot[0] * s + s // 2
            draw.rectangle([cx - 2, line_y + 3, cx + 2, line_y + 7], fill=loco.YELLOW)
            for name, (x0, by0, x1, by1) in boxes.get(box_of.get(i, ""), {}).items():
                # reach boxes are pivot-relative (x forward, y up, inclusive) -> sprite pixels
                left, right = (pivot[0] + x0) * s, (pivot[0] + x1 + 1) * s - 1
                top, bottom = (pivot[1] - by1) * s, (pivot[1] - by0 + 1) * s - 1
                colour = RED if name in ("fist", "sole") else MAGENTA
                draw.rectangle([ox + left, y0 + head + top, ox + right, y0 + head + bottom], outline=colour)
    return sheet


def opus_keys_x6(idle: Image.Image, frames: list[Image.Image], table: list[dict], manifest: dict,
                 s: int = 6) -> Image.Image:
    """The approved idle and every frozen Opus key frame side by side at x6 (floor line, pivot)."""
    keys = [i for i, r in enumerate(table) if r["key"]]
    images = [("idle (approved ref)", "locomotion frame 0", idle)]
    images += [(f"{table[i]['move']}: {table[i]['name']}", f"#{i}  {frame_timing(table[i])}", frames[i]) for i in keys]
    pivot = manifest["pivot"]
    w, h = idle.size
    head, foot, gap = 30, 12, 8
    cell_w = w * s
    sheet = Image.new("RGB", (len(images) * (cell_w + gap) + gap, head + h * s + foot + 2 * gap), loco.BG_SHEET6)
    draw, font = ImageDraw.Draw(sheet), ImageFont.load_default()
    for col, (title, timing, img) in enumerate(images):
        ox, oy = gap + col * (cell_w + gap), gap
        draw.text((ox, oy), title, fill=WHITE, font=font)
        draw.text((ox, oy + 13), timing, fill=WHITE, font=font)
        big = img.resize((cell_w, h * s), Image.Resampling.NEAREST)
        cell = Image.new("RGBA", big.size, loco.BG_SHEET6 + (255,))
        cell.alpha_composite(big)
        sheet.paste(cell.convert("RGB"), (ox, oy + head))
        line_y = oy + head + (pivot[1] + 1) * s
        draw.line([(ox, line_y), (ox + cell_w - 1, line_y)], fill=loco.CYAN, width=1)
        cx = ox + pivot[0] * s + s // 2
        draw.rectangle([cx - 2, line_y + 3, cx + 2, line_y + 7], fill=loco.YELLOW)
    return sheet


# ---------------------------------------------------------------- timing diagram

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


# ---------------------------------------------------------------- tick-timed GIFs

def tick_durations(ticks: list[int]) -> list[int]:
    """Milliseconds per frame for a list of tick counts, quantised to the 10 ms GIF clock.

    Boundaries are rounded at the cumulative tick position (n ticks = n * 1000 / 60 ms), so the
    error never exceeds 5 ms and does not accumulate over a move.
    """
    out, prev, total = [], 0, 0
    for t in ticks:
        total += t
        boundary = int(round(total * 1000 / TICK_HZ / 10)) * 10
        out.append(boundary - prev)
        prev = boundary
    return out


def gif_sequences(table: list[dict]) -> dict[str, list[tuple[int, int]]]:
    """GIF name -> [(atlas frame index, ticks shown)], from the tick table (before the idle tail)."""
    def move_frames(move: str, until_tick: int | None = None) -> list[tuple[int, int]]:
        out = []
        for i, r in enumerate(table):
            if r["move"] != move:
                continue
            if r["ticks"] is None:
                out.append((i, KB_PREVIEW_TICKS))
                continue
            a, b = r["ticks"]
            if until_tick is not None:
                if a >= until_tick:
                    continue
                b = min(b, until_tick - 1)
            out.append((i, b - a + 1))
        return out

    chain_start = 7  # earliest start of the next move after the jab (manifest cancel.earliest_start_tick)
    jab_cut = move_frames("punch_a", chain_start)
    return {
        "punch_a": move_frames("punch_a"),
        "punch_b": move_frames("punch_b"),
        "jab_haymaker": jab_cut + move_frames("punch_b"),
        "jab_kick": jab_cut + move_frames("kick"),
        "kick": move_frames("kick"),
        "hurt": move_frames("hurt"),
        "knockdown_getup": move_frames("knockdown") + move_frames("getup"),
    }


def gif_scene(sprite: Image.Image, pivot, label: str, hit: bool) -> Image.Image:
    img = loco.up(loco.scene(sprite, pivot, GIF_SIZE, GIF_GROUND, x_center=GIF_PIVOT_X), GIF_SCALE)
    d = ImageDraw.Draw(img)
    d.text((6, 4), label, fill=RED if hit else WHITE, font=ImageFont.load_default())
    return img


def write_tick_gifs(out: Path, frames, idle, pivot, table, written: list[Path]) -> dict[str, list[int]]:
    expected: dict[str, list[int]] = {}
    for name, seq in gif_sequences(table).items():
        images, ticks = [], []
        for i, n in seq:
            r = table[i]
            images.append(gif_scene(frames[i], pivot, f"{r['id']} {frame_timing(r)}", r["hit"]))
            ticks.append(n)
        images.append(gif_scene(idle, pivot, "idle", False))
        ticks.append(IDLE_TAIL_TICKS)
        durations = tick_durations(ticks)
        path = out / f"{name}_x4.gif"
        loco.save_gif(path, images, durations)
        written.append(path)
        expected[name] = durations
    return expected


# ---------------------------------------------------------------- Floor 1 compositions

def fade(sprite: Image.Image, alpha: int) -> Image.Image:
    out = sprite.copy()
    out.putalpha(sprite.getchannel("A").point(lambda v: v * alpha // 255))
    return out


def launch_offset(ticks: int) -> tuple[int, int]:
    """(dx, rise) in px of a launched goblin ``ticks`` ticks after the kick hit (proposed values)."""
    t = ticks / TICK_HZ
    return round(LAUNCH_VX * t), round(LAUNCH_VY * t - 0.5 * GRAVITY * t * t)


def render_floor(out: Path, manifest: dict, frames, table, pivot, written: list[Path]) -> None:
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
    kick = frames[index_of(table, "k3")]
    windup = frames[index_of(table, "k1")]

    def stage(entries):  # (sprite, pivot, x, lift) -> actor layer + ground-level boxes for the shadows
        layer = Image.new("RGBA", tuple(layout["pixel_size"]), (0, 0, 0, 0))
        boxes = []
        for sprite, piv, x, lift in entries:
            ground = tops[x // ek.TILE]
            if not isinstance(ground, int):
                raise loco.PreviewError(f"no walking line at x={x}")
            pos = (x - piv[0], ground - 1 - piv[1] - lift)
            layer.alpha_composite(sprite, pos)
            if lift == 0 and sprite.getchannel("A").point(lambda v: 255 if v == 255 else 0).getbbox():
                boxes.append((pos[0], pos[1], pos[0] + sprite.width, pos[1] + sprite.height))
        return layer, boxes

    sets = {"kick": [(kick, pivot, CARL_X, 0), (goblin, GOBLIN_PIVOT, gx_a, 0),
                     (goblin, GOBLIN_PIVOT, gx_a + GOBLIN_GAP, 0)]}
    # kick sequence: wind-up ghost, contact, goblin A launched (ghosts at 4, 8, 14, 20, 25 ticks)
    # into goblin B, which stands 30 px behind it (CP-02). Positions from the proposed launch values.
    seq = [(fade(windup, 90), pivot, CARL_X, 0), (goblin, GOBLIN_PIVOT, gx_a + GOBLIN_GAP, 0)]
    for n in (25, 20, 14, 8, 4):
        dx, rise = launch_offset(n)
        seq.append((fade(goblin, 110), GOBLIN_PIVOT, gx_a + dx, rise))
    seq += [(kick, pivot, CARL_X, 0), (goblin, GOBLIN_PIVOT, gx_a, 0)]
    sets["kick_sequence"] = seq
    lineup = []
    for x, fid in zip(LINEUP_X, LINEUP_FRAMES):
        lineup.append((frames[index_of(table, fid)], pivot, x, 0))
    sets["lineup"] = lineup
    for name, entries in sets.items():
        actors, boxes = stage(entries)
        for k, tag in ((1, "640x360"), (3, "1920x1080")):
            if name != "kick" and k == 1:
                continue  # the new compositions are reviewed at 1080p; the 640 x 360 original stays for the kick
            img = ekl.render_lit(kit, layout, view, k, actors, boxes=boxes)
            path = out / f"floor01_{name}_{tag}.png"
            img.save(path, format="PNG", optimize=False)
            written.append(path)
            if name == "kick" and k == 3:  # 160 x 90 virtual-px window around the action, shown at x6
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
        table = frame_table(manifest)
        if len(table) != len(frames):
            raise loco.PreviewError(f"manifest lists {len(table)} frames, atlas has {len(frames)}")
        out = args.out
        out.mkdir(parents=True, exist_ok=True)
        written: list[Path] = []
        for name, img in (("contact_sheet_x1.png", contact_sheet_x1(loco_frames[0], frames)),
                          ("contact_sheet_x6.png", contact_sheet_x6(frames, table, manifest)),
                          ("opus_keys_x6.png", opus_keys_x6(loco_frames[0], frames, table, manifest)),
                          ("timing_diagram.png", timing_diagram(manifest))):
            img.save(out / name, format="PNG", optimize=False)
            written.append(out / name)
        write_tick_gifs(out, frames, loco_frames[0], pivot, table, written)
        if not args.no_floor:
            render_floor(out, manifest, frames, table, pivot, written)
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
