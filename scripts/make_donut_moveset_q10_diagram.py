#!/usr/bin/env python3
"""Design diagram for Q10, Donut's swat and pounce (docs/02-art/donut-moveset-q10.md).

Presentation only: composed from the frozen ANIM-DONUT-02 keys (s1, p0, p1, p2), the GOB-01 goblin
tell frame and the gameplay numbers of the spec (hit boxes, the 20 x 24 hurtbox, the goblin body and
swing box, the pounce launch). Not a Bevy capture and not authoritative collision; the Rust tests are.

    python3 scripts/make_donut_moveset_q10_diagram.py [--out DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]

from scripts.pixelart.common import BG, GREY, WHITE, YELLOW, flip, grid_images, label, load_grid_file  # noqa: E402
from scripts.pixelart.evidence import write_sums  # noqa: E402

MELEE = ROOT / "assets/source/characters/donut/donut_h40_melee.grid.json"
GOBLIN = ROOT / "assets/source/characters/goblin/goblin_a_h36.grid.json"
DEFAULT_OUT = ROOT / "docs/evidence/donut-moveset-q10"

GREEN, RED, MAGENTA, ORANGE, CYAN = (90, 220, 120), (240, 70, 70), (230, 90, 230), (255, 160, 50), (80, 210, 230)

# Spec numbers (pivot-relative, x forward, y up from the feet, half-open [min, max)).
SWAT_BOX = (13, 24, 5, 11)          # Q10-03
POUNCE_BOX = (11, 25, 0, 17)        # Q10-11
HURTBOX = (-10, 10, 0, 24)          # Q4 Option C, 20 x 24
GOB_BODY = (-12, 12, 0, 38)         # ENEMY_HALF_WIDTH 12, ENEMY_HALF_HEIGHT 19
GOB_SWING = (8, 30, 0, 30)          # SWING_X_MIN..MAX, SWING_Y_MIN..MAX
TRIGGER = 30                        # GOBLIN_TRIGGER
LAUNCH_VX, LAUNCH_VY, GRAVITY = 180.0, 130.0, 1050.0   # Q10-10
POUNCE_GAP = 72                     # demo launch distance, centre to centre
FLIGHT_GHOST_TICK = 5               # the p1 ghost is drawn at launch + 5
K = 4                               # diagram scale
FLOOR_Y = 200                       # floor line inside a panel, diagram pixels


def rect(d, origin_x, facing, box, colour, width=2):
    """Draw a pivot-relative half-open box at diagram scale; origin_x is the actor pivot (diagram px)."""
    x0, x1, y0, y1 = box
    if facing < 0:
        x0, x1 = -x1, -x0
    d.rectangle([origin_x + x0 * K, FLOOR_Y - y1 * K, origin_x + x1 * K - 1, FLOOR_Y - y0 * K - 1],
                outline=colour, width=width)


def paste(panel, img, pivot, origin_x, lift_px=0, alpha=255):
    big = img.resize((img.width * K, img.height * K), Image.Resampling.NEAREST)
    if alpha < 255:
        a = big.getchannel("A").point(lambda v: v * alpha // 255)
        big.putalpha(a)
    panel.alpha_composite(big, (origin_x - pivot[0] * K, FLOOR_Y - lift_px * K - (pivot[1] + 1) * K))


def pounce_path(gap: int):
    """Semi-implicit 60 Hz flight from launch; stops x on the first tick the box overlaps the goblin body."""
    x = y = 0.0
    vx, vy = LAUNCH_VX, LAUNCH_VY
    pts, hit_tick = [(0.0, 0.0)], None
    for tick in range(1, 40):
        vy -= GRAVITY / 60
        x += vx / 60
        y = max(0.0, y + vy / 60)
        if hit_tick is None and x + POUNCE_BOX[1] > gap + GOB_BODY[0]:
            hit_tick, vx = tick, 0.0
        pts.append((x, y))
        if y == 0.0:
            break
    return pts, hit_tick


def swat_panel(donut, dpiv, gob, gpiv) -> Image.Image:
    w, h = 72 * K + TRIGGER * K + 120, FLOOR_Y + 70
    p = Image.new("RGBA", (w, h), BG + (255,))
    d = ImageDraw.Draw(p)
    ox = 50 * K
    gx = ox + TRIGGER * K
    paste(p, gob[0], gob[1], gx)
    paste(p, donut, dpiv, ox)
    d.line([(0, FLOOR_Y), (w, FLOOR_Y)], fill=CYAN)
    rect(d, gx, -1, GOB_BODY, WHITE, 1)
    rect(d, gx, -1, GOB_SWING, ORANGE)
    rect(d, ox, 1, HURTBOX, GREEN)
    rect(d, ox, 1, SWAT_BOX, RED)
    for x, c in ((ox, YELLOW), (gx, YELLOW)):
        d.rectangle([x - 2, FLOOR_Y - 2, x + 2, FLOOR_Y + 2], fill=c)
    label(d, (8, 6), "A. Swat (s1, hit tick 4) at the goblin trigger distance, 30 px centre to centre", WHITE, 12)
    label(d, (8, FLOOR_Y + 8), "red swat box +13..24, 5..11 up: reaches a goblin centre < 36 px", RED)
    label(d, (8, FLOOR_Y + 24), "orange goblin swing 8..30 ahead: reaches Donut's green 20x24 hurtbox < 40 px", ORANGE)
    label(d, (8, FLOOR_Y + 40), "white = goblin body 24x38; yellow = pivots; the swing out-ranges the swat by 4 px", GREY)
    return p


def pounce_panel(melee, mpiv, gob, gpiv) -> Image.Image:
    pts, hit_tick = pounce_path(POUNCE_GAP)
    w, h = (POUNCE_GAP + 60) * K + 140, FLOOR_Y + 70
    p = Image.new("RGBA", (w, h), BG + (255,))
    d = ImageDraw.Draw(p)
    ox = 40 * K
    gx = ox + POUNCE_GAP * K
    paste(p, gob[0], gob[1], gx)
    paste(p, melee[4], mpiv, ox, alpha=110)                              # p0 crouch at the start
    hx, _hy = pts[hit_tick]
    mx, my = pts[FLIGHT_GHOST_TICK]
    paste(p, melee[5], mpiv, ox + round(mx * K), lift_px=round(my), alpha=150)   # p1 mid-flight
    lx, _ = pts[-1]
    paste(p, melee[6], mpiv, ox + round(lx * K))                         # p2 pin at touchdown
    d.line([(0, FLOOR_Y), (w, FLOOR_Y)], fill=CYAN)
    for i, (x, y) in enumerate(pts):
        c = RED if i == hit_tick else YELLOW
        d.rectangle([ox + x * K - 2, FLOOR_Y - y * K - 2, ox + x * K + 2, FLOOR_Y - y * K + 2], fill=c)
    rect(d, gx, -1, GOB_BODY, WHITE, 1)
    rect(d, ox + round(hx * K), 1, POUNCE_BOX, MAGENTA)
    rect(d, ox + round(lx * K), 1, HURTBOX, GREEN)
    free_pts, _ = pounce_path(10_000)
    label(d, (8, 6), f"B. Pounce launched {POUNCE_GAP} px from a goblin: crouch t0-9, launch t10 "
                     f"(vx {LAUNCH_VX:.0f}, vy {LAUNCH_VY:.0f} px/s)", WHITE, 12)
    label(d, (8, FLOOR_Y + 8), f"dots = pivot per tick; red dot = hit tick (launch + {hit_tick}), x stops, she drops into "
                               f"the pin", YELLOW)
    label(d, (8, FLOOR_Y + 24), f"free flight: {free_pts[-1][0]:.0f} px, {len(free_pts) - 1} ticks, apex "
                                f"{max(y for _, y in free_pts):.0f} px; magenta = pounce box +11..25, 0..17 up", MAGENTA)
    label(d, (8, FLOOR_Y + 40), f"ghosts: p0 crouch (start), p1 flight (launch + {FLIGHT_GHOST_TICK}); solid: p2 pin at touchdown (launch + {len(pts) - 1})", GREY)
    return p


TIMELINES = [  # (name, [(ticks, colour, text)])
    ("Donut swat 14", [(4, GREY, "4"), (3, RED, "3"), (7, (90, 90, 120), "7")]),
    ("swat chain", [(10, (60, 60, 80), "re-swat t10"), (4, GREY, ""), (3, RED, ""), (7, (90, 90, 120), "")]),
    ("Donut pounce 44", [(10, GREY, "crouch 10"), (14, MAGENTA, "flight 14"), (2, RED, "2"),
                          (6, (90, 90, 120), "6"), (12, (60, 60, 80), "12 move-cancel")]),
    ("Carl jab 14", [(3, GREY, "3"), (2, RED, "2"), (9, (90, 90, 120), "9")]),
    ("Carl kick 36", [(13, GREY, "13"), (4, RED, "4"), (19, (90, 90, 120), "19")]),
    ("goblin attack 50", [(24, ORANGE, "tell 24 (interruptible)"), (2, (200, 120, 40), ""),
                          (4, RED, "4"), (20, (90, 90, 120), "recover 20 (interruptible)")]),
    ("light guard", [(60, (70, 120, 90), ">= 60 ticks and until the next swing Impact: swats glance, others interrupt")]),
]


def timing_panel() -> Image.Image:
    tick_px, row_h, left = 9, 26, 150
    w, h = left + 62 * tick_px + 20, 34 + len(TIMELINES) * row_h + 26
    p = Image.new("RGBA", (w, h), BG + (255,))
    d = ImageDraw.Draw(p)
    label(d, (8, 6), "C. Frame data at 60 Hz (startup grey, active red, recovery blue; ticks)", WHITE, 12)
    for r, (name, segs) in enumerate(TIMELINES):
        y = 30 + r * row_h
        label(d, (8, y + 4), name, WHITE)
        x = left
        for ticks, colour, text in segs:
            d.rectangle([x, y, x + ticks * tick_px - 1, y + row_h - 6], fill=colour)
            if text:
                label(d, (x + 3, y + 4), text, BG if colour in (ORANGE, GREEN) else WHITE, 10)
            x += ticks * tick_px
    for t in range(0, 61, 10):
        x = left + t * tick_px
        d.line([(x, h - 22), (x, h - 16)], fill=GREY)
        label(d, (x - 4, h - 14), str(t), GREY, 10)
    return p


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args(argv)
    melee_grid, gob_grid = load_grid_file(MELEE), load_grid_file(GOBLIN)
    melee, mpiv = grid_images(melee_grid), tuple(melee_grid["pivot"])
    gob_img, gpiv = flip(grid_images(gob_grid)[10], tuple(gob_grid["pivot"]))   # attack_tell, facing left
    panels = [swat_panel(melee[1], mpiv, (gob_img, gpiv), gpiv), pounce_panel(melee, mpiv, (gob_img, gpiv), gpiv),
              timing_panel()]
    width = max(pn.width for pn in panels)
    sheet = Image.new("RGB", (width, sum(pn.height for pn in panels) + 8 * len(panels)), BG)
    y = 0
    for pn in panels:
        sheet.paste(pn.convert("RGB"), (0, y))
        y += pn.height + 8
    args.out.mkdir(parents=True, exist_ok=True)
    out = args.out / "q10_hitboxes_timing_x4.png"
    sheet.save(out, optimize=True)
    write_sums(args.out, [out])
    print(out.relative_to(ROOT) if out.is_relative_to(ROOT) else out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
