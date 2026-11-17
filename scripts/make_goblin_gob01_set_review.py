#!/usr/bin/env python3
"""Review renders for the full GOB-01 goblin set (phase 3: 27 frames, variants A and B).

Usage (after ``bash scripts/export_assets.sh``, which builds the Floor 1 kit atlases the compositor needs):

    python3 scripts/make_goblin_gob01_set_review.py [--out DIR]

Reads the pixel-grid sources directly and writes, deterministically, into DIR (default
``generated/art/gob01_set``):

- ``<tag>_x3.gif`` / ``<tag>_x6.gif``: one GIF per tag, variant A left and variant B right, played at the
  tick table at 60 Hz (GIF delays are whole centiseconds, so the delay of each frame is rounded on the
  cumulative tick count and the total never drifts); once and hold tags rest on their last frame;
- ``contact_sheet_x1.png`` and ``contact_sheet_A_x6.png`` / ``contact_sheet_B_x6.png``: all 27 frames per
  variant with the contact row 55 (cyan) and the pivot (yellow) marked;
- ``floor01_set_640x360.png`` / ``floor01_set_1920x1080.png``: Carl with 3 A and 2 B goblins on lit Floor 1;
- ``tell_hurt_1x.png``: attack tell and hurt, A and B, at native size;
- ``cp02_strip.png``: Carl kick, goblin launch, flight, contact (frame 26), second goblin hurt, first goblin down;
- ``layout_diagram.png``: frame grid, pivot, boxes, the frame 13 hitbox, the atlas strip and the tick bars;
- ``measurements_set.json``: per frame opaque bboxes, colours and file hashes.

Composed PNGs and GIFs, not Bevy captures. Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))

import make_goblin_gob01_review as rv  # noqa: E402
import author_goblin_frames as af  # noqa: E402

ROOT = rv.ROOT
DEFAULT_OUT = ROOT / "generated" / "art" / "gob01_set"
GRIDS = rv.GOBLIN_KEYS
CARL_KICK = ROOT / "tests/fixtures/carl_kick_cp02_frames.json"
BG, CYAN, YELLOW, WHITE, GREY = rv.BG, rv.CYAN, rv.YELLOW, rv.WHITE, rv.GREY
GROUND_LIGHT, GROUND_DARK = rv.GROUND_LIGHT, rv.GROUND_DARK
TAG_ROWS = af.TAGS
REST_TICKS = {"hold": 30, "once": 20, "loop": 0}


def label(d: ImageDraw.ImageDraw, xy, text: str, fill=WHITE) -> None:
    d.text(xy, text, fill=fill, font=rv.font())


def sprite_on(img: Image.Image, k: int, size=(64, 56), pivot=(28, 55), draw_floor: bool = True) -> Image.Image:
    """One frame at scale k on the flat review background, the walking line under row 55."""
    cell = Image.new("RGBA", (size[0], size[1] + 4), BG + (255,))
    d = ImageDraw.Draw(cell)
    if draw_floor:
        d.rectangle([0, size[1], size[0] - 1, size[1]], fill=GROUND_LIGHT)
        d.rectangle([0, size[1] + 1, size[0] - 1, size[1] + 3], fill=GROUND_DARK)
    cell.alpha_composite(img)
    return rv.up(cell, k)


# ------------------------------------------------------------------ GIFs

def tick_sequence(tag: str) -> list[tuple[int, int]]:
    """[(atlas frame, ticks)] for one tag, with the resting tail for once and hold tags."""
    frames = dict(TAG_ROWS)[tag]
    seq = [(n, t if t is not None else REST_TICKS['hold']) for n, t in frames]
    mode = af.PLAYBACK[tag]
    if mode != "loop":
        n, t = seq[-1]
        seq[-1] = (n, t + REST_TICKS[mode] if frames[-1][1] is not None else t)
    return seq


def gif_delays(ticks: list[int]) -> list[int]:
    """Whole-centisecond GIF delays (in ms) from tick counts at 60 Hz, rounded on the cumulative sum."""
    out, cum, prev = [], 0, 0
    for t in ticks:
        cum += t
        cs = round(cum * 100 / 60)
        out.append((cs - prev) * 10)
        prev = cs
    return out


def make_gif(frames: dict[str, list[Image.Image]], tag: str, k: int, path: Path) -> list[int]:
    seq = tick_sequence(tag)
    images = []
    for n, _t in seq:
        a = sprite_on(frames["A"][n], k)
        b = sprite_on(frames["B"][n], k)
        sheet = Image.new("RGB", (a.width * 2 + 4 * k, a.height), BG)
        sheet.paste(a.convert("RGB"), (0, 0))
        sheet.paste(b.convert("RGB"), (a.width + 4 * k, 0))
        images.append(sheet.convert("P", palette=Image.Palette.ADAPTIVE, colors=64))
    delays = gif_delays([t for _n, t in seq])
    images[0].save(path, save_all=True, append_images=images[1:], duration=delays, loop=0, disposal=1,
                   optimize=False)
    return delays


# ------------------------------------------------------------------ contact sheets

def cell_with_marks(img: Image.Image, k: int, pivot) -> Image.Image:
    cell = Image.new("RGBA", img.size, (36, 34, 50, 255))
    cell.alpha_composite(img)
    big = rv.up(cell, k)
    d = ImageDraw.Draw(big)
    d.line([(0, pivot[1] * k + k - 1), (big.width, pivot[1] * k + k - 1)], fill=CYAN)
    d.rectangle([pivot[0] * k, pivot[1] * k, pivot[0] * k + k - 1, pivot[1] * k + k - 1], outline=YELLOW)
    return big


def frame_caption(n: int) -> str:
    tag = next(t for t, fr in TAG_ROWS if any(f == n for f, _ in fr))
    ticks = af.frame_ticks()[n]
    key = "KEY" if n in af.KEY_IDS else "inb"
    return f"{n:02d} {tag} {('hold' if ticks is None else str(ticks) + 't')} {key}"


def contact_x1(frames, pivot) -> Image.Image:
    w, h = frames["A"][0].size
    sheet = Image.new("RGB", (27 * w + 8, 2 * (h + 26) + 28), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (4, 4), "GOB-01 all 27 frames at 1x, variant A (top) and B (bottom); cyan = contact row 55, yellow = pivot (28,55)")
    for row, v in enumerate("AB"):
        for n in range(27):
            cell = cell_with_marks(frames[v][n], 1, pivot)
            sheet.paste(cell, (4 + n * w, 26 + row * (h + 26)))
            label(d, (6 + n * w, 26 + row * (h + 26) + h + 2), f"{n}", GREY if n not in af.KEY_IDS else YELLOW)
    return sheet


def contact_x6(frames, pivot, variant: str) -> Image.Image:
    k, cols = 6, 9
    w, h = frames[variant][0].size
    cw, ch = w * k, h * k + 34
    rows = 3
    sheet = Image.new("RGB", (cols * (cw + 8) + 8, rows * (ch + 8) + 34), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (8, 8), f"GOB-01 variant {variant}: all 27 frames at x6 (frame, tag, ticks at 60 Hz, KEY = Opus key, inb = in-between); "
                     f"cyan = contact row 55, yellow = pivot (28,55)")
    for n in range(27):
        r, c = divmod(n, cols)
        x, y = 8 + c * (cw + 8), 34 + r * (ch + 8)
        sheet.paste(cell_with_marks(frames[variant][n], k, pivot), (x, y))
        label(d, (x + 2, y + h * k + 6), frame_caption(n), YELLOW if n in af.KEY_IDS else WHITE)
        info = rv.bbox_info(frames[variant][n])
        label(d, (x + 2, y + h * k + 20), f"w={info['opaque_width']} h={info['opaque_height']} low={info['lowest_row']}", GREY)
    return sheet


# ------------------------------------------------------------------ scenes

def set_scene(floor: rv.Floor, frames, pivot, carl, k: int) -> Image.Image:
    entries = [(carl[0], carl[1], rv.CARL_X, 0)]
    # (variant, atlas frame, walking-line x, lift, facing right?)
    cast = [("A", 4, 392, 0, False), ("A", 11, 514, 0, True), ("B", 6, 574, 0, True),
            ("A", 2, 640, 0, True), ("B", 17, 704, 0, True)]
    for v, n, x, lift, flipped in cast:
        img = frames[v][n]
        piv = pivot
        if flipped:
            img, piv = rv.flip(img, pivot)
        entries.append((img, piv, x, lift))
    return floor.render(entries, k)


def tell_hurt(frames, pivot) -> Image.Image:
    order = [("A", 11, "A tell (11)"), ("A", 16, "A hurt (16)"), ("A", 17, "A stagger (17)"),
             ("B", 11, "B tell (11)"), ("B", 16, "B hurt (16)"), ("B", 17, "B stagger (17)")]
    w, h = frames["A"][0].size
    sheet = Image.new("RGB", (len(order) * (w + 6) + 6, h + 24), BG)
    d = ImageDraw.Draw(sheet)
    d.rectangle([0, h + 1, sheet.width, h + 1], fill=GROUND_LIGHT)
    d.rectangle([0, h + 2, sheet.width, h + 5], fill=GROUND_DARK)
    for i, (v, n, cap) in enumerate(order):
        x = 6 + i * (w + 6)
        sheet.paste(frames[v][n], (x, 0), frames[v][n])
        label(d, (x, h + 8), cap, GREY)
    return sheet


def load_carl_kick() -> dict[int, Image.Image]:
    data = json.loads(CARL_KICK.read_text(encoding="utf-8"))
    pal = {p["key"]: tuple(int(p["rgba"][1 + i:3 + i], 16) for i in (0, 2, 4, 6)) for p in data["palette"]}
    w, h = data["canvas"]
    out = {}
    for n, fr in data["frames"].items():
        img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        px = img.load()
        for y, row in enumerate(fr["rows"]):
            for x, c in enumerate(row):
                if c != ".":
                    px[x, y] = pal[c]
        out[int(n)] = img
    return out, tuple(data["pivot"])


def cp02_strip(floor: rv.Floor, frames, pivot, carl_kick, carl_piv) -> Image.Image:
    """Illustrative staging of the proposed contract (not a simulation): ticks counted from the kick contact T."""
    gx = floor.ground(rv.CARL_X) - floor.view[1]
    panels = [
        ("kick snap (k2)", "T-2", 10, [("A", 6, 504, 0, True), ("B", 4, 590, 0, True)]),
        ("contact (k3): goblin 1 hit, launch (18)", "T+0 .. T+3", 11, [("A", 18, 506, 0, True), ("B", 4, 590, 0, True)]),
        ("flight (19), leaves the floor", "T+4 .. T+15", 11, [("A", 19, 540, 16, True), ("B", 4, 590, 0, True)]),
        ("contact (26): knockback boxes overlap", "T+16 .. T+18", 12, [("A", 26, 566, 12, True), ("B", 4, 592, 0, True)]),
        ("goblin 2 hurt (16), goblin 1 rebounds (19)", "T+19 .. T+22", 12, [("A", 19, 552, 8, True), ("B", 16, 596, 0, True)]),
        ("goblin 2 stagger (17), goblin 1 down (20)", "T+23 .. T+34", 12, [("A", 20, 530, 0, True), ("B", 17, 600, 0, True)]),
    ]
    scale = 2
    x0 = rv.CARL_X - 36 - floor.view[0]
    x1 = x0 + 200
    tiles = []
    for cap, ticks, carl_frame, goblins in panels:
        entries = [(carl_kick[carl_frame] if carl_frame in carl_kick else carl_kick[10], carl_piv, rv.CARL_X, 0)]
        for v, n, x, lift, flipped in goblins:
            img, piv = (rv.flip(frames[v][n], pivot) if flipped else (frames[v][n], pivot))
            entries.append((img, piv, x, lift))
        scene = floor.render(entries, 1)
        crop = scene.crop((x0, gx - 76, x1, gx + 10))
        tiles.append((cap, ticks, rv.up(crop, scale)))
    tw, th = tiles[0][2].size
    cols = 3
    sheet = Image.new("RGB", (cols * (tw + 10) + 10, 2 * (th + 40) + 34), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (10, 8), "CP-02 staging (proposal, not a simulation): Carl kick -> goblin 1 launched -> flight -> contact with goblin 2 -> goblin 2 hurt -> goblin 1 down. "
                      "T = the kick's active tick 13; lit Floor 1 crops at x2.")
    for i, (cap, ticks, img) in enumerate(tiles):
        r, c = divmod(i, cols)
        x, y = 10 + c * (tw + 10), 34 + r * (th + 40)
        sheet.paste(img, (x, y))
        label(d, (x + 2, y + th + 4), f"{i + 1}. {cap}", WHITE)
        label(d, (x + 2, y + th + 18), ticks, YELLOW)
    return sheet


# ------------------------------------------------------------------ diagram

def layout_diagram(frames, pivot) -> Image.Image:
    k = 8
    w, h = 64, 56
    gw, gh = w * k, h * k
    atlas_k = 3
    sheet = Image.new("RGB", (max(3 * (gw + 12) + 12, 27 * w * atlas_k // 3 + 40), 40 + gh + 24 + 150 + 60 + 90), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (10, 8), "GOB-01 layout: frame 64 x 56, pivot (28,55) (yellow), contact row 55 (cyan); boxes in frame space, facing right "
                      "(mirrored at runtime around the pivot column)")
    panels = [(0, "idle: standing box 24 x 38 (white), hurtbox 22 x 34 (green)"),
              (13, "frame 13 impact: hitbox +8..+30 px forward, 0..30 px up (red)"),
              (19, "knockback: 28 x 18 box centred on the torso at (28,40) (orange)")]
    for i, (n, cap) in enumerate(panels):
        ox, oy = 12 + i * (gw + 12), 36
        cell = Image.new("RGBA", (w, h), (36, 34, 50, 255))
        cell.alpha_composite(frames["A"][n])
        big = rv.up(cell, k)
        sheet.paste(big.convert("RGB"), (ox, oy))
        dd = ImageDraw.Draw(sheet)
        px, py = pivot

        def rect(x0, y0, x1, y1, colour):  # inclusive frame-space pixel box
            dd.rectangle([ox + x0 * k, oy + y0 * k, ox + (x1 + 1) * k - 1, oy + (y1 + 1) * k - 1], outline=colour)
        dd.line([(ox, oy + (py + 1) * k - 1), (ox + gw, oy + (py + 1) * k - 1)], fill=CYAN)
        rect(px, py, px, py, YELLOW)
        if n == 0:
            rect(px - 12, py - 37, px + 11, py, WHITE)
            rect(px - 11, py - 33, px + 10, py - 0, (90, 220, 120))
        if n == 13:
            rect(px + 8, py - 30, px + 30, py, (255, 80, 80))
            rect(px - 12, py - 37, px + 11, py, WHITE)
        if n == 19:
            rect(14, 31, 41, 48, (255, 150, 40))
        label(dd, (ox + 2, oy + gh + 4), cap, WHITE)
    # atlas strip
    y = 36 + gh + 28
    label(d, (10, y), "Atlas: one row of 27 frames per variant (row-major), tag brackets and ticks at 60 Hz; cyan tick = Opus key frame", WHITE)
    y += 16
    cw = 52
    x = 10
    for tag, fr in TAG_ROWS:
        first = x
        for n, t in fr:
            cell = Image.new("RGBA", (w, h), (36, 34, 50, 255))
            cell.alpha_composite(frames["A"][n])
            small = cell.resize((cw, int(h * cw / w)), Image.Resampling.NEAREST)
            sheet.paste(small.convert("RGB"), (x, y))
            label(d, (x + 2, y + small.height + 2), f"{n}", YELLOW if n in af.KEY_IDS else GREY)
            label(d, (x + 2, y + small.height + 14), "hold" if t is None else f"{t}t", WHITE)
            x += cw + 2
        d.line([(first, y - 3), (x - 4, y - 3)], fill=CYAN)
        label(d, (first, y - 14), tag, CYAN)
        x += 6
    # tick bars
    y += int(h * cw / w) + 40
    label(d, (10, y), "Attack ticks (tell start = 0): tell 24 + smear 2 + impact 4 (hitbox active) + recover 20 = 50; cooldown 40 after it; "
                      "hurt 12 then 20-tick hurt cooldown", WHITE)
    y += 18
    scale = 12
    segs = [("tell 24", 24, (90, 140, 255)), ("smear 2", 2, (255, 214, 0)), ("impact 4", 4, (255, 80, 80)),
            ("recover 20", 20, (120, 200, 120)), ("cooldown 40", 40, (130, 130, 150))]
    x = 10
    for name, ticks, colour in segs:
        d.rectangle([x, y, x + ticks * scale - 1, y + 22], fill=colour)
        label(d, (x + 3, y + 6), name, (10, 10, 20))
        x += ticks * scale
    y += 34
    x = 10
    for name, ticks, colour in [("hurt 12", 12, (255, 150, 40)), ("hurt cooldown 20", 20, (130, 130, 150)),
                                ("down 4 + settle 8 + hold 24", 36, (180, 120, 200)), ("getup 18", 18, (120, 200, 120))]:
        d.rectangle([x, y, x + ticks * scale - 1, y + 22], fill=colour)
        label(d, (x + 3, y + 6), name, (10, 10, 20))
        x += ticks * scale
    return sheet.crop((0, 0, sheet.width, y + 34))


# ------------------------------------------------------------------ main

def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="GOB-01 full-set review renders")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    out = args.out if args.out.is_absolute() else ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)

    frames, pivot = {}, None
    for v, path in GRIDS.items():
        frames[v], pivot, _grid = rv.grid_frames(path)
    carl_frames, carl_piv, _ = rv.grid_frames(rv.CARL)
    carl = (carl_frames[0], carl_piv)
    kick, kick_piv = load_carl_kick()
    floor = rv.Floor()
    written: list[Path] = []

    def save(img: Image.Image, name: str) -> None:
        path = out / name
        img.save(path, format="PNG", optimize=False)
        written.append(path)

    delays = {}
    for tag, _fr in TAG_ROWS:
        for k in (3, 6):
            path = out / f"{tag}_x{k}.gif"
            delays[f"{tag}_x{k}"] = make_gif(frames, tag, k, path)
            written.append(path)
    save(contact_x1(frames, pivot), "contact_sheet_x1.png")
    save(contact_x6(frames, pivot, "A"), "contact_sheet_A_x6.png")
    save(contact_x6(frames, pivot, "B"), "contact_sheet_B_x6.png")
    save(set_scene(floor, frames, pivot, carl, 1), "floor01_set_640x360.png")
    save(set_scene(floor, frames, pivot, carl, 3), "floor01_set_1920x1080.png")
    save(tell_hurt(frames, pivot), "tell_hurt_1x.png")
    save(cp02_strip(floor, frames, pivot, kick, kick_piv), "cp02_strip.png")
    save(layout_diagram(frames, pivot), "layout_diagram.png")

    meas = {"note": "Composed review renders (A1 lighting via scripts/env_kit_light.py), not Bevy captures.",
            "tick_hz": 60, "gif_delays_ms": delays, "frames": {"A": {}, "B": {}}, "files": {}}
    for v in "AB":
        for n, img in enumerate(frames[v]):
            meas["frames"][v][f"{n:02d}"] = rv.bbox_info(img)
    for path in written:
        meas["files"][path.name] = sha(path)
    mpath = out / "measurements_set.json"
    mpath.write_text(json.dumps(meas, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for path in written + [mpath]:
        print(f"  {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except rv.ReviewError as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)
