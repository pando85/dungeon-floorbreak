#!/usr/bin/env python3
"""ART-GOB-CF02 evidence renders: stills and GIFs composed from the committed goblin CF02 sources (nothing here is a
Bevy capture or a feel approval).

    bash scripts/export_assets.sh                       # once: the Floor 1 kit atlases the lit scenes need
    python3 scripts/goblin_cf02_evidence.py             # writes docs/evidence/cf02-goblin/*.png and *.gif
    python3 -m scripts.pixelart evidence assets/source/characters/goblin/goblin_a_h36_cf02.pixelart.json \
        --out docs/evidence/cf02-goblin/toolkit/acting_a        # likewise acting_b, slammed_a, slammed_b

Outputs (deterministic, seeded):

* ``contact_sheet_acting_x1.png`` / ``_x3.png`` and ``contact_sheet_slammed_x1.png`` / ``_x3.png``: A above B for every
  frame, captioned with tag, name and ticks; the slammed sheets at presets 4, 3, 2 and 1 (preset 1 hides the gore
  layer, preset 2 remaps it ``R,P -> B``);
* ``hesitate_check_1x.png`` / ``_3x.png``: the unlabelled idle-strip check of spec section 5 (GOB-01 idle frames and
  ``hesitate`` in a seeded shuffle; the answer is in the README);
* ``beckon_palm_1x.png``: ``taunt_beckon`` A and B at 1x and the hand at 8x (the open-palm check);
* ``blind_check_rests_1x.png`` / ``_3x.png``: the three slammed rest frames, ``crushed_remains`` and the routine corpse in
  a seeded shuffle (the answer is in the README);
* ``scene_acting_a_1x.png``, ``scene_acting_b_1x.png``, ``scene_acting_a_3x.png``: lit Floor 1 at gameplay scale with Carl
  and Donut: GOB-01 idle, hesitate, panic x2, taunt x2; the panic goblin faces its flee direction, the others face Carl;
* ``scene_slammed_a_1x.png``, ``scene_slammed_b_1x.png``, ``scene_slammed_a_3x.png``: the six slammed frames with Carl and
  Donut; the wall frames against a placeholder wall stub on their contact column;
* ``ab_pairs_x4.png``: A above B at x4 for every frame (the headband, the notch, the tooth gap, the vest patch);
* ``timing_diagram.png``: the spec section 7 tick windows of the three reactions and the three slammed finishes;
* ``gif_reactions_scene_x1.gif``: hesitate, panic and taunt in the lit scene at tick timing, exact palette.
"""
from __future__ import annotations

import json
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw  # noqa: E402

import author_goblin_cf02 as cf  # noqa: E402
import gob02_death_evidence as dev  # noqa: E402
from scripts.pixelart.common import (BG, CARL_X, DONUT_X, GREY, WHITE, YELLOW, flip, gif_delays,  # noqa: E402
                                     grid_images, label, load_grid_file, repo_path)
from scripts.pixelart.scene import Floor, reference_idle  # noqa: E402

OUT = ROOT / "docs/evidence/cf02-goblin"
PIV = tuple(cf.ad.PIVOT)
FLAT = (42, 42, 58)
GOBLIN_A = "assets/source/characters/goblin/goblin_a_h36.grid.json"
DEATH_A = "assets/source/characters/goblin/death/goblin_a_death_h36.grid.json"


def grid(sheet: str) -> dict:
    return json.loads(cf.SHEETS[sheet]["grid"].read_text(encoding="utf-8"))


def images(sheet: str, preset: int = 4) -> list[Image.Image]:
    return grid_images(dev.preset_grid(grid(sheet), preset))


def gob01_images() -> list[Image.Image]:
    return grid_images(load_grid_file(repo_path(GOBLIN_A)))


def death_images() -> list[Image.Image]:
    return grid_images(load_grid_file(repo_path(DEATH_A)))


def tile(img: Image.Image, k: int, bg=FLAT) -> Image.Image:
    t = Image.new("RGB", img.size, bg)
    t.paste(img, (0, 0), img)
    return t.resize((img.width * k, img.height * k), Image.NEAREST)


def ticks_label(sheet: str, n: int) -> str:
    for tag, _pb, rows in cf.SHEETS[sheet]["tags"]:
        for f, t in rows:
            if f == n:
                return f"{cf.SHEETS[sheet]['names'][n]} {'hold' if t is None else str(t) + 't'}"
    return ""


# --------------------------------------------------------------------------- contact sheets
def contact_sheets() -> list[Path]:
    paths = []
    for kind, n_frames in (("acting", 5), ("slammed", 6)):
        for k in (1, 3):
            presets = (4,) if kind == "acting" else (4, 3, 2, 1)
            w, h = 64 * k + 4, 56 * k + 4
            cap = 14 if k > 1 else 0
            rows = [(v, p) for p in presets for v in "ab"]
            sheet = Image.new("RGB", (n_frames * w + 70, 22 + len(rows) * (h + cap) + 6), BG)
            d = ImageDraw.Draw(sheet)
            label(d, (4, 4), f"goblin {kind} sheets, A above B, x{k}"
                  + ("; slammed at presets 4, 3, 2, 1 (1 hides the gore layer, 2 remaps it to dark red)"
                     if kind == "slammed" else ""), WHITE, 11)
            for r, (variant, preset) in enumerate(rows):
                sheet_key = f"{kind}_{variant}"
                imgs = images(sheet_key, preset)
                y = 22 + r * (h + cap)
                label(d, (4, y + h // 2), f"{variant.upper()}" + (f" p{preset}" if kind == "slammed" else ""), YELLOW, 11)
                for n, im in enumerate(imgs):
                    x = 70 + n * w
                    sheet.paste(tile(im, k), (x, y))
                    px = cf.ad.PIVOT
                    ImageDraw.Draw(sheet).rectangle([x + px[0] * k, y + px[1] * k, x + px[0] * k + k - 1, y + px[1] * k + k - 1],
                                                    outline=YELLOW)
                    if k > 1 and r == 0:
                        label(d, (x + 1, y + h - 1), ticks_label(sheet_key, n), GREY, 10)
            path = OUT / f"contact_sheet_{kind}_x{k}.png"
            sheet.save(path)
            paths.append(path)
    return paths


def ab_pairs() -> Path:
    k = 4
    cols = 11
    w, h = 64 * k + 4, 56 * k + 4
    sheet = Image.new("RGB", (6 * w + 60, 22 + 2 * (2 * (h + 4)) + 6), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (4, 4), "A above B, x4: acting frames (top block), slammed frames (bottom block); preset 4", WHITE, 11)
    for block, (kind, n_frames) in enumerate((("acting", 5), ("slammed", 6))):
        for r, v in enumerate("ab"):
            y = 22 + block * 2 * (h + 4) + r * (h + 4)
            label(d, (4, y + h // 2), v.upper(), YELLOW, 11)
            for n, im in enumerate(images(f"{kind}_{v}")):
                sheet.paste(tile(im, k), (60 + n * w, y))
    path = OUT / "ab_pairs_x4.png"
    sheet.save(path)
    return path


# --------------------------------------------------------------------------- blind and readability checks
def hesitate_check() -> list[Path]:
    paths = []
    gob = gob01_images()
    strip = [("idle 0", gob[0]), ("idle 1", gob[1]), ("idle 2", gob[2]), ("idle 3", gob[3]),
             ("hesitate", images("acting_a")[0])]
    order = list(range(5))
    random.Random(2026).shuffle(order)
    for k in (1, 3):
        w = 64 * k
        sheet = Image.new("RGB", (5 * w + 4, 56 * k + 4), FLAT)
        for i, j in enumerate(order):
            sheet.paste(tile(strip[j][1], k), (2 + i * w, 2))
        path = OUT / f"hesitate_check_{k}x.png"
        sheet.save(path)
        paths.append(path)
    (OUT / "hesitate_check_answer.txt").write_text(
        "left to right: " + ", ".join(strip[j][0] for j in order) + "\n", encoding="utf-8")
    return paths


def beckon_palm() -> Path:
    """taunt_thump and taunt_beckon of A and B at 1x, then the beckon hand of A and B at 8x."""
    tiles = []
    for v in "ab":
        im = images(f"acting_{v}")
        tiles += [im[3], im[4]]
    zoom = 8
    palm_w, palm_h = 20 * zoom, 16 * zoom
    width = max(4 * 66 + 4, 2 * (palm_w + 6) + 2)
    sheet = Image.new("RGB", (width, 62 + palm_h + 6), FLAT)
    for i, im in enumerate(tiles):
        sheet.paste(tile(im, 1), (2 + i * 66, 2))
    for i, im in enumerate(tiles[1::2]):
        sheet.paste(tile(im.crop((36, 28, 56, 44)), zoom), (2 + i * (palm_w + 6), 62))
    path = OUT / "beckon_palm_1x.png"
    sheet.save(path)
    return path


def blind_check() -> list[Path]:
    paths = []
    death = death_images()
    gob = gob01_images()
    for variant in "ab":
        sl = images(f"slammed_{variant}")
        items = [("stomped_rest", sl[1]), ("wall_rest", sl[3]), ("flung_rest", sl[5]),
                 ("crushed_remains", death[2]), ("corpse", gob[25])]
        order = list(range(5))
        random.Random(77 if variant == "a" else 78).shuffle(order)
        for k in (1, 3):
            w = 64 * k
            sheet = Image.new("RGB", (5 * w + 4, 56 * k + 4), FLAT)
            for i, j in enumerate(order):
                sheet.paste(tile(items[j][1], k), (2 + i * w, 2))
            path = OUT / f"blind_check_rests_{variant}_{k}x.png"
            sheet.save(path)
            paths.append(path)
        (OUT / f"blind_check_rests_{variant}_answer.txt").write_text(
            "left to right: " + ", ".join(items[j][0] for j in order) + "\n", encoding="utf-8")
    return paths


# --------------------------------------------------------------------------- lit scenes
def wall_stub(height: int = 56, width: int = 14) -> Image.Image:
    im = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, width - 1, height - 1], fill=(88, 80, 104, 255), outline=(27, 17, 36, 255))
    for y in range(6, height, 12):
        d.line([(1, y), (width - 2, y)], fill=(70, 64, 86, 255))
    return im


def scene_entries(floor: Floor, specs: list[dict]) -> list[tuple]:
    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    entries = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
    for s in specs:
        img, piv = s["img"], PIV
        if s.get("flip", True):                     # goblins face Carl: mirrored around the pivot column (runtime rule)
            img, piv = flip(img, PIV)
        entries.append((img, piv, s["x"], s.get("lift", 0)))
        if "wall" in s:                             # wall face on sprite column ``wall`` (authored, facing right)
            col = s["wall"]
            stub = wall_stub()
            sx = s["x"] - (PIV[0] - col)            # world x of that column (unflipped sprite)
            entries.insert(0, (stub, (stub.width, stub.height - 1), sx, 0, False))
    return entries


def acting_scene(floor: Floor, variant: str) -> list[dict]:
    a = images(f"acting_{variant}")
    gob = gob01_images()
    xs = [520, 572, 624, 676, 728, 780]
    return [{"img": gob[0], "x": xs[0]},
            {"img": a[0], "x": xs[1]},
            {"img": a[1], "x": xs[2], "flip": False},          # the fleeing goblin faces its flee direction
            {"img": a[2], "x": xs[3], "flip": False},
            {"img": a[3], "x": xs[4]},
            {"img": a[4], "x": xs[5]}]


def slammed_scene(floor: Floor, variant: str) -> list[dict]:
    s = images(f"slammed_{variant}")
    return [{"img": s[0], "x": 510},
            {"img": s[1], "x": 562},
            {"img": s[2], "x": 640, "flip": False, "wall": 16},
            {"img": s[3], "x": 712, "flip": False, "wall": 16},
            {"img": s[4], "x": 776},
            {"img": s[5], "x": 836}]


def scenes() -> list[Path]:
    floor = Floor()
    paths = []
    for name, fn, variants, ks in (("acting", acting_scene, "ab", (1, 3)), ("slammed", slammed_scene, "ab", (1, 3))):
        for v in variants:
            for k in ks:
                if k == 3 and v == "b":
                    continue
                path = OUT / f"scene_{name}_{v}_{k}x.png"
                floor.render(scene_entries(floor, fn(floor, v)), k).save(path)
                paths.append(path)
    return paths


def reactions_gif(floor: Floor) -> Path:
    """Hesitate (20 ticks), panic (60 ticks, 6 + 6 alternating) and taunt (36 ticks, 9 + 9 alternating) in the lit scene
    at tick timing, unlit so the sprite palette survives exactly."""
    a = images("acting_a")
    seq = [(0, 20, True)] + [(1 if i % 2 == 0 else 2, 6, False) for i in range(10)] + \
          [(3 if i % 2 == 0 else 4, 9, True) for i in range(4)]
    frames, delays = [], []
    gx = 560
    left, right = 120, 120
    for f, ticks, faces_carl in seq:
        specs = [{"img": a[f], "x": gx, "flip": faces_carl}]
        im = floor.render(scene_entries(floor, specs), 1, lit=False)
        vx = CARL_X - floor.view[0]
        gv = floor.ground(CARL_X) - floor.view[1]
        frames.append(im.crop((vx - 70, gv - 90, vx + 190, gv + 14)))
        delays.append(ticks)
    path = OUT / "gif_reactions_scene_x1.gif"
    dev.save_exact_gif(frames, gif_delays(delays), path)
    return path


# --------------------------------------------------------------------------- timing diagram
def timing_diagram() -> Path:
    rows = [("hesitate (Q61)", [("hesitate", 20)]),
            ("panic (Q58, 60 to 90 ticks)", [("flee", 6), ("flee_b", 6)] * 5),
            ("taunt (Q65, 36 ticks)", [("thump", 9), ("beckon", 9)] * 2),
            ("stomped finish (Q21, Q72)", [("hold", 6), ("impact", 8), ("rest", 0)]),
            ("wall finish (Q21, Q72)", [("hold", 6), ("impact", 8), ("rest", 0)]),
            ("flung finish (Q21, Q72)", [("hold", 6), ("impact", 8), ("rest", 0)])]
    px_per_tick = 10
    width = 150 + 60 * px_per_tick + 60
    sheet = Image.new("RGB", (width, 30 + len(rows) * 30), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (4, 4), "goblin CF02 tick windows, 60 Hz (spec section 7); hold 6t = the Q72 +6 mid-taunt hold (only if it died mid-taunt); rest = state-driven hold", WHITE, 11)
    colours = [(90, 140, 255), (255, 214, 0), (255, 80, 80), (120, 200, 120)]
    for r, (name, segs) in enumerate(rows):
        y = 26 + r * 30
        label(d, (4, y + 6), name, WHITE, 11)
        x = 150
        for i, (seg, t) in enumerate(segs):
            w = max(t, 12 if t == 0 else t) * px_per_tick
            d.rectangle([x, y, x + w - 2, y + 22], fill=colours[i % 2 if t else 3])
            label(d, (x + 3, y + 5), f"{seg} {t}t" if t else f"{seg} hold", (10, 10, 20), 10)
            x += w
    path = OUT / "timing_diagram.png"
    sheet.save(path)
    return path


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for fn in (contact_sheets, hesitate_check, blind_check):
        for p in fn():
            print("wrote", p.relative_to(ROOT))
    for fn in (ab_pairs, beckon_palm, timing_diagram):
        print("wrote", fn().relative_to(ROOT))
    for p in scenes():
        print("wrote", p.relative_to(ROOT))
    print("wrote", reactions_gif(Floor()).relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
