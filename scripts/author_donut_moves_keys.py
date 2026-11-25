#!/usr/bin/env python3
"""Authoring record for the DONUT-MOVES-ART-01 hero keys (docs/02-art/donut-moves-art-01-spec.md, section 9).

Writes ``assets/source/characters/donut/moves_keys/donut_h40_moves_keys.grid.json`` (2 frames, 72 x 44, pivot
(46, 43), the approved Donut palette unchanged, binary alpha):

* ``evade_twist`` (frame 0), the apex of the reflex evade: a low cat hop with the back arched into a hump above the
  head line and the hindquarters twisted toward the camera (cream belly and the near hind leg kicked back), the
  front paws gathered stiff under the chest, the tail bottle-brushed into a hook over the hump. Head, face and
  tiara/collar are the approved ``jump_rise`` cels translated (ducked 6 px); tail, legs and body are new pixels
  built from the shapes below.
* ``blink_arrive`` (frame 1), the Puddle Jumper pop-out: the approved CF02 ``smug`` key (tail, body, head, face,
  gear) raised 2 px, clipped at the puddle surface, two front paws tucked to the chest (new pixels) and the puddle
  mouth (back rim under her, front rim over her) in her own palette's gem magenta, collar violet and tiara lilac.

Every new region is a union of capsules (a disc swept along a segment) so the shapes are parameters, not hand
edits; shading follows the approved frames' light (soft plum outline on top and left edges, plum outline on bottom
and right edges, one highlight row under a lit edge, two shadow rows over the belly line). A cleanup pass recolours
every orphan pixel (no same-colour 8-neighbour) to its most common opaque neighbour, so the keys pass the strict
lint (0 isolated, 0 orphan pixels, lowest row on the pivot row). The few hand pixels are listed in ``TOUCH``.

Run ``python3 scripts/author_donut_moves_keys.py`` to rewrite the grid (deterministic), ``--check`` to verify the
committed grid, ``--sheets <dir>`` to write the 1x / 3x key sheets and the silhouette-gate sheet.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
MELEE = ROOT / "assets/source/characters/donut/donut_h40_melee.grid.json"
CF02_KEYS = ROOT / "assets/source/characters/donut/cf02_keys/donut_h40_cf02_keys.grid.json"
OUT = ROOT / "assets/source/characters/donut/moves_keys/donut_h40_moves_keys.grid.json"
W, H = 72, 44
PIVOT = (46, 43)
LAYERS = ["puddle_back", "tail", "legs_far", "body", "legs_near", "head", "face", "gear", "puddle_front"]

JUMP_RISE = 12          # locomotion frame whose head, face and tiara the evade key reuses
EVADE_HEAD = (0, 7)     # head ducked 7 px (dx, dy)
ARRIVE_LIFT = -2        # the smug key raised 2 px
WATERLINE = 38          # arrive: first row hidden inside the puddle
PUDDLE = (45, 39.0, 24.8, 4.6)   # arrive: puddle mouth ellipse (cx, cy, rx, ry)
BELLY_SEAM = (17.0, 0.5, 34)   # evade: cream below y = 17 + 0.5 (x - 10), up to column 34 (the rolled belly)


# ------------------------------------------------------------------ helpers


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pix(rows: list[str]) -> dict:
    return {(x, y): c for y, row in enumerate(rows) for x, c in enumerate(row) if c != "."}


def shift(p: dict, dx: int, dy: int) -> dict:
    return {(x + dx, y + dy): c for (x, y), c in p.items()}


def cel(p: dict) -> list[str]:
    rows = [["."] * W for _ in range(H)]
    for (x, y), c in p.items():
        if not (0 <= x < W and 0 <= y < H):
            raise ValueError(f"pixel {(x, y)} outside the {W}x{H} canvas")
        rows[y][x] = c
    return ["".join(r) for r in rows]


def capsule(a, b, ra, rb) -> set:
    """Pixels whose centre lies within the disc swept from ``a`` (radius ra) to ``b`` (radius rb)."""
    (ax, ay), (bx, by) = a, b
    out = set()
    x0, x1 = int(min(ax - ra, bx - rb)) - 1, int(max(ax + ra, bx + rb)) + 2
    y0, y1 = int(min(ay - ra, by - rb)) - 1, int(max(ay + ra, by + rb)) + 2
    dx, dy = bx - ax, by - ay
    ll = dx * dx + dy * dy
    for y in range(y0, y1):
        for x in range(x0, x1):
            t = 0.0 if ll == 0 else max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / ll))
            r = ra + (rb - ra) * t
            if math.hypot(x - (ax + t * dx), y - (ay + t * dy)) <= r:
                out.add((x, y))
    return out


def chain(points) -> set:
    """Union of capsules along ``[(x, y, r), ...]``."""
    out = set()
    for (ax, ay, ra), (bx, by, rb) in zip(points, points[1:]):
        out |= capsule((ax, ay), (bx, by), ra, rb)
    return out


def shade(mask: set, fill: str, light: str, dark: str, top_soft: str = "j", hard: str = "K",
          dark_rows: int = 2) -> dict:
    """Outline and shade a mask in the approved light: soft outline where the outside is above or left, hard
    outline elsewhere, one ``light`` row under a lit edge, ``dark_rows`` of ``dark`` over the lower edge."""
    out = {}
    for (x, y) in mask:
        up, left = (x, y - 1) not in mask, (x - 1, y) not in mask
        down, right = (x, y + 1) not in mask, (x + 1, y) not in mask
        if up or left or down or right:
            out[(x, y)] = top_soft if (up or left) and not (down or right) else hard
        elif (x, y - 2) not in mask:
            out[(x, y)] = light
        elif any((x, y + k) not in mask for k in range(1, dark_rows + 1)):
            out[(x, y)] = dark
        else:
            out[(x, y)] = fill
    return out


def paint(layer: dict, region: set, colour: str, inside_only: bool = True) -> None:
    """Recolour interior pixels (not the outline) of ``layer`` that fall in ``region``."""
    for p in region:
        if p in layer and (not inside_only or layer[p] not in ("K", "j")):
            layer[p] = colour


def fur_fringe(layer: dict, mask: set, colour: str, side: str, every: int = 2) -> None:
    """Bottle-brush fringe: every ``every``-th boundary pixel on ``side`` grows one outward pixel."""
    step = {"left": (-1, 0), "up": (0, -1), "right": (1, 0)}[side]
    edge = sorted(p for p in mask if (p[0] + step[0], p[1] + step[1]) not in mask)
    for i, (x, y) in enumerate(edge):
        if i % every == 0:
            layer[(x + step[0], y + step[1])] = colour


def cleanup(layers: dict[str, dict], accents: set[str]) -> None:
    """Recolour every composite orphan (a non-accent pixel with no same-colour 8-neighbour) on its own layer to the
    most common opaque colour around it; repeat until stable."""
    for _ in range(8):
        comp = {}
        owner = {}
        for name in LAYERS:
            for p, c in layers.get(name, {}).items():
                comp[p] = c
                owner[p] = name
        changed = False
        for (x, y), c in sorted(comp.items()):
            if c in accents:
                continue
            around = [comp.get((x + dx, y + dy)) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
            if c in around:
                continue
            votes = Counter(n for n in around if n is not None)
            if votes:
                layers[owner[(x, y)]][(x, y)] = votes.most_common(1)[0][0]
                changed = True
        if not changed:
            return


# ------------------------------------------------------------------ evade_twist


def evade_twist(loco: dict) -> dict:
    src = loco["frames"][JUMP_RISE]["cels"]
    dx, dy = EVADE_HEAD
    L: dict[str, dict] = {}
    L["head"] = shift(pix(src["head"]), dx, dy)
    L["face"] = shift(pix(src["face"]), dx, dy)
    L["gear"] = shift(pix(src["gear"]), dx, dy)          # tiara and collar ride with the ducked head

    # Body: the arched back. Spine from behind the head over a hump peak, down to the twisted rump.
    back = chain([(44, 31, 7.0), (38, 22, 8.5), (31, 16, 9.0), (24, 18, 8.5), (20, 25, 7.0)])
    chest = chain([(47, 37, 3.5), (58, 37, 3.5)])         # cream bib under the ducked chin
    body = shade(back, "B", "H", "D")
    # Calico patches on the hump (the coat map keeps ginger on the shoulder and the haunch).
    paint(body, chain([(35, 12, 2.5), (40, 17, 2.0)]), "O")
    paint(body, chain([(34, 10, 1.0), (37, 12, 1.0)]), "G")
    paint(body, chain([(39, 19, 1.0), (41, 19, 1.0)]), "o")
    paint(body, chain([(22, 12, 2.0), (25, 11, 1.5)]), "O")
    # Twist: the rear half rolls toward the camera, so the belly shows cream under the haunch.
    belly = {(x, y) for (x, y) in back if x <= BELLY_SEAM[2] and y >= BELLY_SEAM[0] + BELLY_SEAM[1] * (x - 10)}
    paint(body, belly, "C")
    paint(body, {(x, y) for (x, y) in belly if (x, y + 2) not in back}, "c")
    paint(body, {(x, y) for (x, y) in belly if (x, y - 1) not in belly and (x, y - 1) in back}, "W")
    bib = shade(chest, "C", "W", "c", top_soft="c", hard="K", dark_rows=1)
    for p, c in bib.items():
        if p not in body or body[p] in ("K", "j", "D"):
            body[p] = c
    L["body"] = body

    # Legs: short and stubby (the bible: short legs). Front paws gathered under the bib; far hind leg straight
    # down; near hind leg kicked back with its sole toward the camera (the twist read).
    front_far = chain([(47, 37, 2.3), (47, 41, 2.0)])
    front_near = chain([(57, 37, 2.4), (57, 41, 2.1)])
    hind_far = chain([(24, 32, 2.6), (25, 41, 2.1)])
    thigh = chain([(18, 29, 4.6), (14, 34, 3.0)])
    shin = chain([(14, 34, 2.4), (12, 37, 2.2)])       # kicked back and tucked up: airborne
    far = shade(hind_far, "B", "H", "D", dark_rows=1)
    paint(far, {(x, y) for (x, y) in hind_far if y >= 40}, "c")
    far.update(shade(front_far, "c", "C", "c", top_soft="c", dark_rows=1))
    L["legs_far"] = far
    near = shade(thigh | shin, "C", "W", "c", dark_rows=1)   # inner thigh toward the camera: cream
    paint(near, chain([(19, 26, 2.2), (22, 28, 1.5)]), "O")    # the haunch patch rolls over the top
    paint(near, chain([(18, 25, 1.0), (20, 25, 1.0)]), "G")
    paint(near, {(11, 38), (12, 38)}, "E")                  # toe bean: the sole faces the camera
    near.update(shade(front_near, "C", "W", "c", top_soft="c", dark_rows=1))
    L["legs_near"] = near

    # Tail: bottle-brushed, out of the rump, up, and hooked forward over the hump.
    tail_mask = chain([(13, 22, 3.5), (8, 15, 4.5), (8, 9, 4.5), (13, 6, 3.5), (19, 6, 2.5), (22, 8, 1.5)])
    tail = shade(tail_mask, "O", "G", "o", dark_rows=1)
    paint(tail, chain([(7, 16, 2.0), (10, 15, 1.5)]), "B")      # dark band (the coat map's tail ring)
    paint(tail, chain([(9, 9, 1.5), (12, 8, 1.0)]), "B")
    fur_fringe(tail, tail_mask, "G", "left", every=2)
    fur_fringe(tail, tail_mask, "C", "up", every=3)
    L["tail"] = tail
    return L


# ------------------------------------------------------------------ blink_arrive


def blink_arrive(keys: dict) -> dict:
    src = keys["frames"][1]["cels"]                       # the frozen CF02 smug key
    L: dict[str, dict] = {}
    for name in ("tail", "body", "head", "face", "gear"):
        L[name] = {p: c for p, c in shift(pix(src[name]), 0, ARRIVE_LIFT).items() if p[1] < WATERLINE}
    # The smug body's front legs sat below the waterline: their outline marks inside the chest are smoothed.
    body = L["body"]
    for (x, y), c in list(body.items()):
        if y >= 35 and c == "K" and all(n in body for n in ((x - 1, y), (x + 1, y))):
            body[(x, y)] = "c"
    # The pop tucks two front paws to the chest (filled cream mitts, outline only below and outside).
    paws = {}
    for cx in (46, 57):
        m = chain([(cx, 29, 2.4), (cx, 32, 2.4)])
        for (x, y) in m:
            lower = (x, y + 1) not in m
            outer = (x - 1, y) not in m or (x + 1, y) not in m
            paws[(x, y)] = "K" if lower else ("c" if outer else ("W" if (x, y - 1) not in m else "C"))
    L["legs_near"] = paws
    # Puddle mouth: a flat ellipse centred under her. The back half (lilac rim, violet bank, plum water) sits
    # behind her; the front half (plum water, magenta lip, plum outline) covers her lower body, so she reads as
    # rising out of it rather than standing on it.
    cx, cy, rx, ry = PUDDLE
    disc = {(x, y) for y in range(30, H) for x in range(W) if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0}
    behind = set(L["body"]) | set(L["tail"])
    back, front = {}, {}
    for (x, y) in disc:
        if (x, y - 1) not in disc:
            back[(x, y)] = "L"
        elif (x, y - 2) not in disc:
            back[(x, y)] = "V"
        elif y < WATERLINE - 1:
            back[(x, y)] = "D"
        elif (x, y + 1) not in disc:
            front[(x, y)] = "K"
        elif (x, y + 2) not in disc:
            front[(x, y)] = "M"
        elif y == WATERLINE - 1 and (x, y - 1) in behind:
            front[(x, y)] = "L"                           # the meniscus where her body breaks the surface
        else:
            front[(x, y)] = "D"
    # Splash: where her body breaks the surface, a lilac crown spikes up at each flank and two drops fly off,
    # so the key reads as coming up out of the water, not sitting on a mat.
    edge = sorted(x for (x, y) in behind if y == WATERLINE - 1)
    if edge:
        lo, hi = edge[0], edge[-1]
        for (x, y) in ((lo - 1, WATERLINE - 2), (lo - 1, WATERLINE - 3), (lo - 2, WATERLINE - 2),
                       (hi + 1, WATERLINE - 2), (hi + 1, WATERLINE - 3), (hi + 2, WATERLINE - 2)):
            front[(x, y)] = "L"
        for (x0, y0) in ((lo - 6, WATERLINE - 7), (hi + 5, WATERLINE - 8)):
            front.update({(x0, y0): "L", (x0 + 1, y0): "L", (x0, y0 + 1): "M", (x0 + 1, y0 + 1): "M"})
    L["puddle_back"] = back
    L["puddle_front"] = front
    return L


# ------------------------------------------------------------------ build


TOUCH: dict[str, dict] = {"evade_twist": {}, "blink_arrive": {}}


def build() -> dict:
    loco, keys = load(LOCO), load(CF02_KEYS)
    palette = loco["palette"]
    accents = {e["key"] for e in palette if e.get("accent")}
    frames = []
    for name, layers in (("evade_twist", evade_twist(loco)), ("blink_arrive", blink_arrive(keys))):
        for (x, y), c in TOUCH[name].items():
            layers.setdefault("body", {})[(x, y)] = c
        cleanup(layers, accents)
        frames.append({"duration_ms": 100, "cels": {n: cel(layers[n]) for n in LAYERS if layers.get(n)}})
    return {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": list(PIVOT), "palette": palette,
            "layers": LAYERS, "frames": frames,
            "tags": [{"name": "evade_twist", "from": 0, "to": 0, "direction": "forward"},
                     {"name": "blink_arrive", "from": 1, "to": 1, "direction": "forward"}]}


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# ------------------------------------------------------------------ review sheets


EVADE_LIFT = 10     # the twist key holds the arc peak (ages 5-7, arc_lift_px 10), spec section 4


def sheets(out_dir: Path) -> list[Path]:
    """Key sheets for review: enlarged keys with references, the lit Floor 1 composition at 1x and 3x, and the
    silhouette-gate sheet (flat silhouettes at 1x and 3x next to the frames the evade must not read as)."""
    from PIL import Image, ImageDraw
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import (BG, CARL_X, DONUT_X, GREY, WHITE, YELLOW, cell, grid_images, label,
                                         load_grid_file, repo_path)
    from scripts.pixelart.scene import Floor

    out_dir.mkdir(parents=True, exist_ok=True)
    keys = grid_images(load(OUT))
    piv = PIVOT
    loco = grid_images(load_grid_file(repo_path("assets/source/characters/donut/donut_h40_locomotion.grid.json")))
    melee = grid_images(load_grid_file(repo_path("assets/source/characters/donut/donut_h40_melee.grid.json")))
    cf02 = grid_images(load_grid_file(repo_path("assets/source/characters/donut/donut_h40_cf02.grid.json")))
    carl_grid = load_grid_file(repo_path("assets/source/characters/carl/carl_h52.grid.json"))
    carl_idle, carl_piv = grid_images(carl_grid)[0], tuple(carl_grid["pivot"])
    written = []

    # 1. enlarged keys beside the approved frames they must stay distinct from
    k = 6
    row = [("idle (approved)", loco[0]), ("jump_rise (approved)", loco[12]), ("hurt (approved)", melee[8]),
           ("evade_twist (KEY)", keys[0]), ("smug (approved)", cf02[2]), ("blink_arrive (KEY)", keys[1])]
    sheet = Image.new("RGB", (len(row) * (W * k + 10) + 10, H * k + 56), BG)
    d = ImageDraw.Draw(sheet)
    for n, (name, im) in enumerate(row):
        x = 10 + n * (W * k + 10)
        sheet.paste(cell(im, k, piv, piv[1]).convert("RGB"), (x, 30))
        label(d, (x, 8), name, YELLOW if "KEY" in name else WHITE, 14)
    label(d, (10, H * k + 36), "72 x 44 canvas, pivot (46, 43) yellow, contact row 43 cyan; facing right", GREY, 12)
    p = out_dir / "keys_x6.png"
    sheet.save(p)
    written.append(p)

    # 2. lit Floor 1 composition at gameplay scale: Carl, idle Donut, the evade at its arc height, a plain jump at
    #    the same height (what it must not read as), the arrive key
    floor = Floor()
    entries = [(carl_idle, carl_piv, CARL_X, 0), (loco[0], piv, DONUT_X, 0),
               (keys[0], piv, 520, EVADE_LIFT), (loco[12], piv, 600, EVADE_LIFT), (keys[1], piv, 690, 0)]
    for scale in (1, 3):
        img = floor.render(entries, scale)
        p = out_dir / f"keys_floor1_{640 * scale}x{360 * scale}.png"
        img.save(p)
        written.append(p)

    # 3. silhouette gate: flat silhouettes and colour at 1x and 3x, evade beside jump_rise, pounce_air and hurt
    probe = [("evade_twist", keys[0]), ("jump_rise", loco[12]), ("pounce_air", melee[5]), ("hurt", melee[8])]

    def flat(im):
        a = im.getchannel("A")
        sil = Image.new("RGBA", im.size, (0, 0, 0, 0))
        sil.paste((20, 16, 28, 255), mask=a)
        return sil

    pad = 6
    cw = W * 3 + pad
    gate = Image.new("RGB", (len(probe) * cw + pad, 30 + H * 3 + pad + H + pad + H * 3 + pad + H + 16),
                     (196, 190, 178))
    d = ImageDraw.Draw(gate)
    for n, (name, im) in enumerate(probe):
        x = pad + n * cw
        label(d, (x, 6), name, (20, 16, 28), 13)
        y = 30
        gate.paste(flat(im).resize((W * 3, H * 3), Image.NEAREST), (x, y), flat(im).resize((W * 3, H * 3)))
        y += H * 3 + pad
        gate.paste(flat(im), (x, y), flat(im))
        y += H + pad
        big = im.resize((W * 3, H * 3), Image.NEAREST)
        gate.paste(big, (x, y), big)
        y += H * 3 + pad
        gate.paste(im, (x, y), im)
    p = out_dir / "gate_silhouettes_1x_3x.png"
    gate.save(p)
    written.append(p)
    return written


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grid instead of writing it")
    ap.add_argument("--sheets", type=Path, help="write the review sheets into this directory (needs generated/art)")
    args = ap.parse_args(argv)
    if args.sheets:
        for p in sheets(args.sheets):
            print(f"wrote {p}")
        return 0
    text = dump(build())
    if args.check:
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} is stale: run scripts/author_donut_moves_keys.py", file=sys.stderr)
            return 1
        print("moves keys up to date")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
