"""DONUT-MOVES-ART-01 evidence: sheets and tick-timed GIFs composed from the committed grids.

``python3 scripts/author_donut_moves.py evidence`` (needs ``bash scripts/export_assets.sh`` for the Floor 1 kit
atlases) writes into ``docs/evidence/donut-moves-art-01``:

* ``moves_sheet_1x.png`` and ``moves_sheet_4x.png``: every moves frame and the VFX strip;
* ``tail_swap_sheet.png``: the 16 locomotion frames above their ``flick_up`` and ``flick_snap`` tails;
* ``gif_evade_{1x,3x}.gif``, ``gif_blink_{1x,3x}.gif``, ``gif_blink_arrive_{1x,3x}.gif``, ``gif_blink_origin_3x.gif``,
  ``gif_fizzle_3x.gif`` and ``gif_tail_flick_{idle,run}_3x.gif``: one frame per distinct picture, held for its 60 Hz
  ticks (the mappings are the Rust ones in ``src/donut_art.rs``, mirrored below and pinned by the unit tests), on lit
  Floor 1 (the toolkit's Pillow lighting pass, not a Bevy capture).

The in-engine captures (real Bevy renders of the render fixtures) are made by ``scripts/make_donut_moves_evidence.sh``.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(HERE))

import author_donut_moves as A  # noqa: E402
from scripts.pixelart import common as c  # noqa: E402
from scripts.pixelart.scene import Floor, reference_idle  # noqa: E402

ORIGIN_X, DEST_X, CARL_X = 500, 612, 452
EVADE_X, GOB_X = 540, 612
PIV = A.PIVOT
VPIV = A.VPIVOT


# ---- the Rust mappings, mirrored (src/donut_art.rs; the unit tests there pin the Rust side)
def evade_frame(age):
    return 0 if age < 2 else 1 if age < 5 else 2 if age < 8 else 3 if age < 10 else 4 if age < 12 else None


def arc_lift(age):
    return 0 if age >= 12 else round(4 * 10 * age * (12 - age) / 144)


def blink_frame(age):
    return 5 if age < 3 else 6 if age < 5 else 7 if age < 6 else None


def arrive_frame(age):
    return 8 if 6 <= age < 8 else 9 if 8 <= age < 11 else 10 if 11 <= age < 13 else None


def fizzle_frame(age):
    return 11 if age < 4 else 12 if age < 8 else None


def ring_open(age):
    return age // 3 if age < 12 else None


def ring_close(age):
    return 4 + age // 3 if age < 9 else None


def fizzle_vfx(age):
    return 9 if age < 3 else 10 if age < 6 else 11 if age < 8 else None


def glint_vfx(age):
    return 12 if age < 3 else 13 if age == 3 else None


class Kit:
    def __init__(self) -> None:
        self.floor = Floor()
        self.moves = c.grid_images(A.build_moves_grid())
        self.tail = c.grid_images(A.build_tail_grid())
        self.vfx = c.grid_images(A.build_vfx_grid())
        self.loco = c.grid_images(A.load(A.LOCO_GRID))
        self.loco_ticks = [A.loco_ticks(f) for f in A.load(A.LOCO_GRID)["frames"]]
        self.cf02 = c.grid_images(A.load(A.CF02_GRID))
        self.carl, self.carl_piv = reference_idle("carl")
        gob_grid = c.load_grid_file(c.repo_path("assets/source/characters/goblin/goblin_a_h36.grid.json"))
        gobs = c.grid_images(gob_grid)
        self.gob_idle = c.flip(gobs[0], tuple(gob_grid["pivot"]))
        self.gob_swing = c.flip(gobs[12], tuple(gob_grid["pivot"]))
        self.cache: dict = {}

    def donut(self, img, x, lift=0, flip=False):
        im, piv = (c.flip(img, PIV) if flip else (img, PIV))
        return (im, piv, x, lift)

    def v(self, frame, x, lift=0, flip=False, shadow=False):
        im, piv = (c.flip(self.vfx[frame], VPIV) if flip else (self.vfx[frame], VPIV))
        return (im, piv, x, lift, shadow)

    def render(self, key, entries, k, crop):
        """Render (cached by ``key``) and crop ``(cx, half_w, top, bottom)`` around world column ``cx``."""
        full = self.cache.get((key, k))
        if full is None:
            full = self.floor.render(entries, k, True)
            self.cache[(key, k)] = full
        cx, half_w, top, bottom = crop
        vx, vy = self.floor.view[0], self.floor.view[1]
        gy = self.floor.ground(cx) - vy
        box = ((cx - vx - half_w) * k, (gy - top) * k, (cx - vx + half_w) * k, (gy + bottom) * k)
        return full.crop(box)


def caption(img, text):
    from PIL import ImageDraw
    d = ImageDraw.Draw(img)
    w = c.text_width(text, 12) + 8
    d.rectangle([0, img.height - 16, w, img.height], fill=(0, 0, 0))
    c.label(d, (4, img.height - 15), text, c.WHITE, 12)
    return img


def make_gif(frames, path):
    """``frames``: list of ``(image, ticks)``; consecutive identical images are merged and their ticks added."""
    merged = []
    for im, t in frames:
        if merged and merged[-1][0].tobytes() == im.tobytes():
            merged[-1][1] += t
        else:
            merged.append([im, t])
    c.save_gif([m[0] for m in merged], c.gif_delays([m[1] for m in merged], A.TICK_HZ), path)
    print(f"wrote {path.relative_to(ROOT)} ({len(merged)} pictures, {sum(m[1] for m in merged)} ticks)")


# ---------------------------------------------------------------------------- sequences


def evade_seq(kit, k, crop):
    out = []
    for tick in range(0, 12 + 22):
        ents = [(kit.carl, kit.carl_piv, CARL_X, 0)]
        label = f"tick {tick}"
        if tick < 12:
            f = evade_frame(tick)
            lift = arc_lift(tick)
            ents.append(kit.donut(kit.moves[f], EVADE_X, lift))
            label += f"  {A.MOVES[f][0]}  lift {lift}"
            if tick < 2:
                ents.append(kit.v(14 + tick, EVADE_X + 6, 0, flip=True, shadow=False))
        elif tick < 12 + 20:
            ents.append(kit.donut(kit.cf02[2], EVADE_X))          # the existing smug overlay
            label += "  smug (CF02-I)"
        else:
            ents.append(kit.donut(kit.loco[0], EVADE_X))
            label += "  idle"
        gob, gpiv = kit.gob_swing if tick < 2 else kit.gob_idle
        ents.append((gob, gpiv, GOB_X, 0))
        key = (tick if tick < 2 else min(tick, 31), "evade")
        img = kit.render(key, ents, k, crop)
        out.append((caption(img.copy(), label) if k > 1 else img, 1))
    return out


def blink_seq(kit, k, crop, with_origin=True, start_age=0, end_age=24, fizzle=False):
    out = []
    for age in range(start_age, end_age):
        ents = [(kit.carl, kit.carl_piv, CARL_X, 0)]
        label = f"age {age}"
        # origin puddle
        o = ring_open(age) if age < 6 else None
        cl = ring_close(age - 6) if (age >= 6 and not fizzle) else None
        fz = fizzle_vfx(age - 6) if (fizzle and age >= 6) else None
        for f in (o, cl, fz):
            if f is not None and with_origin:
                ents.append(kit.v(f, ORIGIN_X))
        if age < 6:
            f = blink_frame(age)
            ents.append(kit.donut(kit.moves[f], ORIGIN_X))
            label += f"  {A.MOVES[f][0]}"
        elif fizzle:
            fi = fizzle_frame(age - 6)
            if fi is not None:
                ents.append(kit.donut(kit.moves[fi], ORIGIN_X))
                label += f"  {A.MOVES[fi][0]}"
            else:
                ents.append(kit.donut(kit.loco[0], ORIGIN_X))
        else:
            d = DEST_X
            if age - 6 < 12:
                ents.append(kit.v(ring_open(age - 6), d))
            f = arrive_frame(age)
            if f is not None:
                ents.append(kit.donut(kit.moves[f], d))
                label += f"  {A.MOVES[f][0]}"
            else:
                ents.append(kit.donut(kit.loco[0], d))
                label += "  idle"
            if age == 6:                                   # the one-frame tiled streak, origin to destination
                y = 8
                x = ORIGIN_X
                while x + 16 <= DEST_X - 4:
                    ents.append((kit.vfx[7], VPIV, x + 8, y - 0, False))
                    x += 16
                ents.append((kit.vfx[8], VPIV, DEST_X - 2, y - 0, False))
        key = (tuple(id(e[0]) for e in ents), tuple(e[2] for e in ents), tuple(e[3] for e in ents))
        img = kit.render(key, ents, k, crop)
        out.append((caption(img.copy(), label) if k > 1 else img, 1))
    return out


def tail_seq(kit, k, crop, loco_frames):
    out = []
    for age in range(0, 14):
        loco = loco_frames[age % len(loco_frames)]
        phase = 0 if age < 2 else 1 if age < 4 else None
        img = kit.loco[loco] if phase is None else kit.tail[16 * phase + loco]
        ents = [(kit.carl, kit.carl_piv, CARL_X, 0), kit.donut(img, EVADE_X)]
        g = glint_vfx(age) if age < 4 else None
        label = f"age {age}  " + ("base" if phase is None else A.PHASES[phase])
        ents_key = (age, loco, phase)
        out.append((caption(kit.render(ents_key, ents, k, crop).copy(), label), 1))
    return out


# ---------------------------------------------------------------------------- sheets


def moves_sheet(kit, out_dir):
    from PIL import Image, ImageDraw
    names = [m[0] for m in A.MOVES]
    for k in (1, 4):
        pad, head = 6, 20
        cols = 5 if k == 4 else 7
        w, h = 72 * k, 44 * k
        rows = (len(names) + cols - 1) // cols
        vw, vh = 48 * k, 16 * k
        vcols = 4 if k == 4 else 8
        vrows = (16 + vcols - 1) // vcols
        width = max(cols * (w + pad) + pad, vcols * (vw + pad) + pad)
        height = head + rows * (h + 16 + pad) + head + vrows * (vh + 16 + pad) + pad
        sheet = Image.new("RGB", (width, height), c.BG)
        d = ImageDraw.Draw(sheet)
        c.label(d, (pad, 4), "DONUT-MOVES-ART-01 moves frames (72 x 44, pivot (46, 43) yellow, contact row 43 cyan)",
                c.WHITE, 12)
        for i, im in enumerate(kit.moves):
            x, y = pad + (i % cols) * (w + pad), head + (i // cols) * (h + 16 + pad)
            sheet.paste(c.cell(im, k, PIV, PIV[1]).convert("RGB"), (x, y))
            key = " KEY" if i in A.MOVE_KEYS else ""
            c.label(d, (x + 2, y + h + 1), f"{i} {names[i]}{key}", c.YELLOW if key else c.GREY, 11)
        y0 = head + rows * (h + 16 + pad)
        c.label(d, (pad, y0 + 2), "vfx_puddle_jumper (48 x 16, pivot (24, 11); surface row 11)", c.WHITE, 12)
        for i, im in enumerate(kit.vfx):
            x, y = pad + (i % vcols) * (vw + pad), y0 + head + (i // vcols) * (vh + 16 + pad)
            sheet.paste(c.cell(im, k, VPIV, VPIV[1]).convert("RGB"), (x, y))
            c.label(d, (x + 2, y + vh + 1), f"{i} {A.VFX_NAMES[i]}", c.GREY, 11)
        p = out_dir / f"moves_sheet_{k}x.png"
        sheet.save(p)
        print(f"wrote {p.relative_to(ROOT)}")


def tail_sheet(kit, out_dir):
    """The 16 locomotion frames (row 1), their flick_up (row 2) and flick_snap (row 3) tails, cropped to the tail."""
    from PIL import Image, ImageDraw
    k, pad = 5, 4
    box = (4, 0, 42, 44)
    w, h = (box[2] - box[0]) * k, (box[3] - box[1]) * k
    sheet = Image.new("RGB", (16 * (w + pad) + pad + 70, 3 * (h + pad) + 26), c.BG)
    d = ImageDraw.Draw(sheet)
    c.label(d, (pad, 4), "tail swap: base (top), flick_up, flick_snap; only the tail cel changes (locomotion frames 0-15)",
            c.WHITE, 12)
    for r, (name, imgs, off) in enumerate((("base", kit.loco, 0), ("flick_up", kit.tail, 0), ("flick_snap", kit.tail, 16))):
        c.label(d, (pad, 26 + r * (h + pad) + 4), name, c.YELLOW, 11)
        for n in range(16):
            im = imgs[off + n].crop(box)
            base = Image.new("RGBA", im.size, c.CELL_BG + (255,))
            base.alpha_composite(im)
            sheet.paste(c.up(base, k).convert("RGB"), (70 + pad + n * (w + pad), 26 + r * (h + pad)))
    p = out_dir / "tail_swap_sheet.png"
    sheet.save(p)
    print(f"wrote {p.relative_to(ROOT)}")


# ---------------------------------------------------------------------------- entry


def main() -> int:
    out = A.EVIDENCE
    out.mkdir(parents=True, exist_ok=True)
    kit = Kit()
    moves_sheet(kit, out)
    tail_sheet(kit, out)
    wide = (EVADE_X + 20, 110, 96, 24)
    for k in (1, 3):
        make_gif(evade_seq(kit, k, wide), out / f"gif_evade_{k}x.gif")
    both = ((ORIGIN_X + DEST_X) // 2, 110, 96, 24)
    for k in (1, 3):
        make_gif(blink_seq(kit, k, both), out / f"gif_blink_{k}x.gif")
    arrive_crop = (DEST_X, 70, 80, 24)
    for k in (1, 3):
        make_gif(blink_seq(kit, k, arrive_crop, with_origin=False, start_age=5, end_age=26),
                 out / f"gif_blink_arrive_{k}x.gif")
    make_gif(blink_seq(kit, 3, (ORIGIN_X, 70, 80, 24), end_age=16), out / "gif_blink_origin_3x.gif")
    make_gif(blink_seq(kit, 3, (ORIGIN_X, 70, 80, 24), end_age=22, fizzle=True), out / "gif_fizzle_3x.gif")
    flick = (EVADE_X, 60, 70, 24)
    make_gif(tail_seq(kit, 3, flick, [0]), out / "gif_tail_flick_idle_3x.gif")
    make_gif(tail_seq(kit, 3, flick, [4, 5, 6, 7, 8, 9]), out / "gif_tail_flick_run_3x.gif")
    return 0


if __name__ == "__main__":
    sys.exit(main())
