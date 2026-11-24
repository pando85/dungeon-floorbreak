#!/usr/bin/env python3
"""HOARD-02 execution evidence: prop contact sheets, the frame-layout / pivot / timing diagram, and offline composites
of the states the room does not reach yet (bank A's slide, the J-B fall, pinned under the landed remnant, the get-up).

Everything here is built from the committed grids on the lit Floor 1 kit (``scripts/pixelart/scene.py``): offline
composites, **not runtime captures**, and each image says so. Needs the kit atlases (``bash scripts/export_assets.sh``).

    python3 scripts/make_hoard02_evidence.py docs/evidence/hoard-02
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]

from PIL import Image, ImageDraw  # noqa: E402

import author_hoard02 as boss_author  # noqa: E402
import author_hoarder_room as room  # noqa: E402
from scripts.pixelart import scene  # noqa: E402
from scripts.pixelart.common import CARL_X, DONUT_X, flip, grid_images  # noqa: E402

BOSS = ROOT / "assets/source/characters/hoarder/hoard-02.grid.json"
LABEL = "composite, not runtime"
BG, INK, RED, GREEN, GREY = (36, 28, 32), (230, 220, 200), (230, 70, 60), (90, 200, 120), (120, 110, 120)


def prop_grid(name: str) -> dict:
    return json.loads((room.OUT / f"{name}.grid.json").read_text())


def frames_of(name: str) -> list[Image.Image]:
    return grid_images(prop_grid(name))


def text(draw: ImageDraw.ImageDraw, xy, s, fill=INK) -> None:
    draw.text(xy, s, fill=fill)


def prop_contact(out: Path) -> None:
    """Every state of every prop at 1x and x4, labelled; the sealed mound frames are shown composed as in the room."""
    for k, name in ((1, "props_contact_x1.png"), (4, "props_contact_x4.png")):
        rows = []
        for piece, (_fn, names, _tags, _pivot, role) in room.PIECES.items():
            rows.append((piece, role, names, frames_of(piece)))
        pad, label_h = 8, 12
        width = max(sum(im.width * k + pad for im in r[3]) for r in rows) + pad
        height = sum(max(im.height for im in r[3]) * k + label_h + pad for r in rows) + pad
        if k == 4:
            # the tall door and gate would dwarf the sheet at x4: draw them at x1 on their own row
            pass
        sheet = Image.new("RGB", (width, height), (58, 40, 40))
        d = ImageDraw.Draw(sheet)
        y = pad
        for piece, role, names, imgs in rows:
            kk = 1 if piece in ("hoarder_door", "hoarder_gate") and k == 4 else k
            text(d, (pad, y), f"{piece} ({role}): " + ", ".join(names))
            y += label_h
            x = pad
            for im in imgs:
                sheet.paste(im.resize((im.width * kk, im.height * kk), Image.NEAREST), (x, y), im.resize((im.width * kk, im.height * kk), Image.NEAREST))
                x += im.width * kk + pad
            y += max(im.height for im in imgs) * kk + pad
        sheet.save(out / name)


def cross(d, x, y, colour) -> None:
    d.line((x - 3, y, x + 3, y), fill=colour)
    d.line((x, y - 3, x, y + 3), fill=colour)


def frame_layout(out: Path) -> None:
    """One diagram: the boss atlas (29 frames, frame size, pivot, contact row, opaque box, tag, authored ticks) and the
    sim-mode to frame table; then each prop's frames with canvas, pivot and collision rectangle."""
    boss = json.loads(BOSS.read_text())
    imgs = grid_images(boss)
    tag_of = {n: t["name"] for t in boss["tags"] for n in range(t["from"], t["to"] + 1)}
    ticks = list(boss_author.h1.TICKS) + [boss_author.TICKS[f] for f in range(16, 29)]
    cols, cell_w, cell_h = 6, 164, 150
    rows_n = (len(imgs) + cols - 1) // cols
    map_lines = [f"{k}: {v}" for k, v in boss_author.FRAME_MAP.items()]
    prop_w = 900
    width = max(cols * cell_w + 8, prop_w) + 8
    height = rows_n * cell_h + 30 + len(map_lines) * 12 + 40 + 560
    sheet = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(sheet)
    text(d, (6, 4), f"HOARD-02 atlas: 29 frames of 160x128 in one row, pivot (70,127) red, contact row 127 green, opaque box grey. {LABEL}")
    for n, im in enumerate(imgs):
        cx, cy = 4 + (n % cols) * cell_w, 20 + (n // cols) * cell_h
        d.rectangle((cx, cy, cx + 161, cy + 129), outline=GREY)
        sheet.paste(im, (cx + 1, cy + 1), im)
        d.line((cx + 1, cy + 128, cx + 160, cy + 128), fill=GREEN)
        cross(d, cx + 1 + 70, cy + 1 + 127, RED)
        bb = im.getbbox()
        if bb:
            d.rectangle((cx + 1 + bb[0], cy + 1 + bb[1], cx + 1 + bb[2] - 1, cy + 1 + bb[3] - 1), outline=GREY)
        text(d, (cx + 2, cy + 131), f"{n} {tag_of[n]} {ticks[n] if ticks[n] else 'hold'}t")
    y = 20 + rows_n * cell_h + 6
    text(d, (6, y), "Sim mode -> frame (ticks), spec section 4 (mode_tick is 1 on the entry tick):")
    for line in map_lines:
        y += 12
        text(d, (14, y), line)
    y += 28
    text(d, (6, y), "Room kit: canvas outline grey, pivot red, collision rectangle green (sprite px, bottom-left at the pivot row).")
    y += 16
    coll = {"hoarder_bank_a": (0, 0, 64, 48), "hoarder_post_b": (0, 0, 16, 64), "hoarder_stack_b": (0, 8, 128, 64),
            "hoarder_door": (0, 0, 16, 228), "hoarder_gate": (0, 0, 16, 228)}
    x = 6
    row_h = 0
    for piece, (_fn, names, _tags, pivot, _role) in room.PIECES.items():
        if piece in ("hoarder_door", "hoarder_gate", "hoarder_mound"):
            continue
        for im, nm in zip(frames_of(piece), names):
            if x + im.width + 8 > width:
                x, y, row_h = 6, y + row_h + 26, 0
            sheet.paste(im, (x, y + 14), im)
            d.rectangle((x - 1, y + 13, x + im.width, y + 14 + im.height), outline=GREY)
            rect = {("hoarder_bank_a", 5): (0, 32, 80, 16), ("hoarder_stack_b", 3): (0, 88, 128, 16),
                    ("hoarder_post_b", 2): (0, 0, 16, 64)}.get((piece, names.index(nm)), coll.get(piece))
            if rect:
                cx0, cy0, cw, ch = rect
                d.rectangle((x + cx0, y + 14 + cy0, x + cx0 + cw - 1, y + 14 + cy0 + ch - 1), outline=GREEN)
            cross(d, x + pivot[0], y + 14 + pivot[1], RED)
            text(d, (x, y), nm[:14])
            x += max(im.width, 70) + 8
            row_h = max(row_h, im.height + 14)
    sheet.crop((0, 0, width, min(height, y + row_h + 30))).save(out / "frame_layout_diagram.png")


# ------------------------------------------------------------------ composites
ROOM_TO_FLOOR_BANK = 204      # room x 296..416 -> floor x 500..620
ROOM_TO_FLOOR_STACK = -168    # room x 752..880 -> floor x 584..712


def clip(im: Image.Image, label: str) -> Image.Image:
    d = ImageDraw.Draw(im)
    d.rectangle((0, 0, 168, 10), fill=(0, 0, 0))
    d.text((2, 0), label, fill=(255, 255, 255))
    return im


def render(floor, entries, label, k=1):
    return clip(floor.render(entries, k), label)


def composites(out: Path) -> None:
    floor = scene.Floor()
    boss = json.loads(BOSS.read_text())
    bimgs, bpiv = grid_images(boss), tuple(boss["pivot"])
    carl, cpiv = scene.reference_idle("carl")
    donut, dpiv = scene.reference_idle("donut")
    kit = {n: frames_of(n) for n in room.PIECES}
    pivots = {n: tuple(prop_grid(n)["pivot"]) for n in room.PIECES}

    def prop(name, frame, x, lift=0, shadow=False, pivot=None):
        return (kit[name][frame], pivot or pivots[name], x, lift, shadow)

    def hero(x, lift=0):
        return [(donut, dpiv, DONUT_X - 80, 0), (carl, cpiv, x, lift)]

    # 1. Bank A: carl on its lit top, the shove jams, the bank shudders, slides 3 px/tick for 16, settles
    seq = []
    bx = 352 + ROOM_TO_FLOOR_BANK
    mound_x = 700
    for frame, dx, note in [(0, 0, "stable"), (1, 1, "disturbed"), (2, -1, "disturbed"), (1, 1, "disturbed"), (2, -1, "disturbed")]:
        seq.append((prop("hoarder_bank_a", frame, bx + dx), note))
    for t in range(16):
        seq.append((prop("hoarder_bank_a", 3 + t % 2, bx - 3 * (t + 1)), f"slid +{t + 1}"))
    seq += [(prop("hoarder_bank_a", 5, 296 + ROOM_TO_FLOOR_BANK), "settled")] * 6
    gif = []
    for (e, note) in seq:
        h = (bx + 58) if note.startswith(("stable", "disturbed")) else (bx - 48 - 10)
        lift = 48 if note.startswith(("stable", "disturbed")) else (0 if note == "settled" else 0)
        gif.append(render(floor, hero(h - 20, lift) + [e], f"bank A: {note}. {LABEL}"))
    gif[0].save(out / "composite_bank_slide_640x360.png")
    gif[0].save(out / "composite_bank_slide.gif", save_all=True, append_images=gif[1:], duration=[100] * 5 + [50] * 16 + [150] * 6, loop=0)
    # 2. J-B: post intact, stack supported, released (sag), falling 12 px/tick x 8, landed remnant in front of her
    px = 808 + ROOM_TO_FLOOR_STACK
    sx = 752 + ROOM_TO_FLOOR_STACK
    post_note = lambda f: prop("hoarder_post_b", f, px)  # noqa: E731
    stack_h = 64 + room.STACK_LEG
    seq = [("supported", [post_note(0), prop("hoarder_stack_b", 0, sx, 64)])] * 8
    seq += [("cracked", [post_note(1), prop("hoarder_stack_b", 0, sx, 64)])] * 4
    for t in range(24):
        seq.append(("released", [post_note(2), prop("hoarder_stack_b", 1, sx, 64 - t // 6)]))
    for t in range(8):
        seq.append(("falling", [post_note(2), prop("hoarder_stack_b", 2, sx, max(0, 64 - 12 * t) - (0 if t < 7 else 64))]))
    seq += [("landed", [post_note(2), prop("hoarder_stack_b", 3, sx, 0)])] * 8
    frames = [render(floor, hero(CARL_X) + e + [(bimgs[22], bpiv, px - 60, 0)] if n == "landed" else hero(CARL_X) + e,
                     f"J-B stack: {n}. {LABEL}") for n, e in seq]
    frames[0].save(out / "composite_jb_supported_640x360.png")
    frames[-1].save(out / "composite_jb_landed_640x360.png")
    frames[0].save(out / "composite_jb_fall.gif", save_all=True, append_images=frames[1:], duration=[100] * 12 + [50] * 32 + [120] * 8, loop=0)
    # 3. Pinned under the landed remnant (remnant drawn in front), then the get-up
    gx = px - 70
    seq = []
    for f in [22] * 20 + [23] * 20 + [22] * 10:
        b, bp = flip(bimgs[f], bpiv)
        seq.append(("pinned", f, b, bp))
    for f in [24] * 12 + [25] * 12:
        b, bp = flip(bimgs[f], bpiv)
        seq.append(("get-up", f, b, bp))
    out_frames = []
    for n, f, b, bp in seq:
        entries = hero(CARL_X - 120) + [(b, bp, gx, 0), prop("hoarder_stack_b", 3, sx, 0)]
        out_frames.append(render(floor, entries, f"{n} (frame {f}), remnant in front. {LABEL}"))
    out_frames[0].save(out / "composite_pinned_640x360.png")
    out_frames[50].save(out / "composite_getup_640x360.png")
    out_frames[0].save(out / "composite_pinned_getup.gif", save_all=True, append_images=out_frames[1:],
                       duration=[40] * 50 + [40] * 24, loop=0)
    # 4. The emerge in the sealed mound: tinted, lip in front, then out with the opened spill
    mound = {n: im for n, im in zip(("back", "front", "opened"), kit["hoarder_mound"])}
    mp = pivots["hoarder_mound"]
    mx = 640
    out_frames = []
    for f, note in [(26, "Reveal 1-30"), (27, "Reveal 31-60"), (28, "Reveal 61-90")]:
        b, bp = flip(bimgs[f], bpiv)
        tint = Image.new("RGBA", b.size, (158, 153, 173, 255))
        shaded = Image.composite(Image.blend(b, b, 0), b, b.getchannel("A"))
        shaded = Image.merge("RGBA", [*(Image.eval(shaded.getchannel(i), lambda v, m=m: int(v * m / 255))
                                        for i, m in zip(range(3), tint.getpixel((0, 0))[:3])), shaded.getchannel("A")])
        entries = hero(CARL_X - 40) + [(mound["back"], mp, mx, 0, False), (shaded, bp, mx, 0, False),
                                       (mound["front"], mp, mx, 0, False)]
        out_frames.append(render(floor, entries, f"{note}: shadow tint, lip in front. {LABEL}"))
    b, bp = flip(bimgs[28], bpiv)
    entries = hero(CARL_X - 40) + [(mound["opened"], mp, mx, 0, False), (b, bp, mx, 0)]
    out_frames.append(render(floor, entries, f"first hittable tick: full value, opened. {LABEL}"))
    for i, fr in enumerate(out_frames):
        fr.save(out / f"composite_emerge_{i}_640x360.png")
    out_frames[0].save(out / "composite_emerge.gif", save_all=True, append_images=out_frames[1:], duration=[500, 500, 500, 900], loop=0)


def attack_composites(out: Path) -> None:
    """The shove tell (settle, brace), the lunge (heave, drag) and the jam (impact, stagger) facing Carl on lit Floor 1.
    The room pilot (`hoarder:carl_solo`) kills her before she shoves, so the runtime stills cannot show these modes."""
    floor = scene.Floor()
    boss = json.loads(BOSS.read_text())
    bimgs, bpiv = grid_images(boss), tuple(boss["pivot"])
    carl, cpiv = scene.reference_idle("carl")
    donut, dpiv = scene.reference_idle("donut")
    for name, frames, bx in (("shove_tell", (16, 17), 700), ("lunge", (18, 19), 640), ("jam", (20, 21), 700)):
        for k, size in ((1, "640x360"), (3, "1920x1080")):
            row = []
            for f in frames:
                b, bp = flip(bimgs[f], bpiv)
                entries = [(donut, dpiv, DONUT_X - 20, 0), (carl, cpiv, CARL_X, 0), (b, bp, bx, 0)]
                row.append(floor.render(entries, k))
            for f, im in zip(frames, row):
                clip(im, f"{name} frame {f}. {LABEL}").save(out / f"composite_{name}_f{f}_{size}.png")


def main(argv=None) -> int:
    out = Path((argv or sys.argv[1:])[0])
    out.mkdir(parents=True, exist_ok=True)
    prop_contact(out)
    frame_layout(out)
    composites(out)
    attack_composites(out)
    print("wrote", out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
