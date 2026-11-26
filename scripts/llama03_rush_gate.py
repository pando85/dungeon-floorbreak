"""LLAMA-03 silhouette gate images (``python3 scripts/author_llama03_rush.py gate``).

Writes into ``docs/evidence/llama-03-rush/``:

- ``gate_silhouettes_1x.png`` / ``gate_silhouettes_4x.png``: Carl, Donut and GOB-01 A idle, the LLAMA-01 idle, spit
  tell and spit release, then the four rush keys; colour on a dark brick tone above, black silhouettes on a pale tone
  below, all on one contact row, facing right as authored.
- ``gate_corral_strips_1x.png`` / ``gate_corral_strips_3x.png``: the same llama frames on the merged Infernal Corral
  (the CORRAL-WIRE tick-360 window still), mirrored to face Carl as the runtime draws them, one strip per frame, plus
  the lane mock.
- ``gate_corral_640x360_brace.png`` / ``gate_corral_640x360_run.png``: two full 640 x 360 frames at 1x.
- ``SHA256SUMS`` over the PNG files.

The backdrop still already holds Carl and Donut (their runtime sprites); the llama, a goblin, the dust-trail mock and
the lane band are composited over it here. The dust puffs and the lane band are review annotations in the CORRAL-E
stand-in's dust tone and a neutral white, not authored art. Presentation-only tooling.
"""
from __future__ import annotations

import hashlib
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "docs/evidence/llama-03-rush"
LLAMA = ROOT / "assets/source/characters/llama/llama-01.grid.json"
STILL = ROOT / "docs/evidence/corral-wire/still-corral-640x360-tick360.png"
PIVOT = (44, 71)

# Measured on the tick-360 still: the walking line (sprite contact row) and Carl's pivot column (his shoes span x 396
# to 417). The llama stands at the rush trigger's approach distance (112 px) in front of him, facing left.
CONTACT_Y = 287
CARL_PIVOT_X = 406
APPROACH = 112
TRAVEL = 96                        # 4 px/tick x 24 active ticks (enemy_llama_rush_v0.json)
BOX = (0, 28, 2, 30)               # rush box x [0, 28), heights [2, 30) above the contact row, from the pivot
STRIP = (300, 196, 640, 300)       # crop of the still for one strip (x0, y0, x1, y1)
DUST = (242, 209, 158, 210)        # the stand-in's dust tone (src/combat_anim/rush_stand_in.rs PALETTE)
LANE_FILL = (255, 255, 255, 56)
LANE_EDGE = (255, 255, 255, 170)
LABEL_BG = (24, 23, 34)
LABEL_FG = (236, 230, 214)


def _images(grid: dict) -> list[Image.Image]:
    from scripts.pixelart.common import grid_images
    return grid_images(grid)


def _llama01() -> list[Image.Image]:
    from scripts.pixelart.common import load_grid_file
    return _images(load_grid_file(LLAMA))


def _cast():
    from scripts.pixelart.scene import reference_idle
    return [(n, *reference_idle(n)) for n in ("carl", "donut", "goblin")]


def _black(img: Image.Image) -> Image.Image:
    a = img.getchannel("A")
    out = Image.new("RGBA", img.size, (16, 14, 20, 255))
    out.putalpha(a.point(lambda v: 255 if v else 0))
    return out


def _up(img: Image.Image, k: int) -> Image.Image:
    return img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)


def _frames(keys_grid: dict) -> list[tuple[str, Image.Image]]:
    l1 = _llama01()
    keys = _images(keys_grid)
    names = [t["name"] for t in keys_grid["tags"]]
    return [("LLAMA-01 idle", l1[0]), ("spit tell", l1[8]), ("spit release", l1[9])] + list(zip(names, keys))


def write_silhouettes(keys_grid: dict) -> list[Path]:
    cast = _cast()
    frames = _frames(keys_grid)
    cells = [(n, img, piv) for n, img, piv in cast] + [(n, img, PIVOT) for n, img in frames]
    pad, base = 6, 74                                   # base: the shared contact row inside a cell row
    widths = [img.width + pad for _n, img, _p in cells]
    w, h = sum(widths) + pad, base + 4
    colour = Image.new("RGBA", (w, h), (58, 36, 38, 255))
    black = Image.new("RGBA", (w, h), (206, 196, 176, 255))
    x = pad
    lefts = []
    for (name, img, piv), cw in zip(cells, widths):
        pos = (x, base - piv[1])
        colour.alpha_composite(img, pos)
        black.alpha_composite(_black(img), pos)
        lefts.append((x, cw, name))
        x += cw
    sheet = Image.new("RGBA", (w, h * 2), LABEL_BG + (255,))
    sheet.paste(colour, (0, 0))
    sheet.paste(black, (0, h))
    out1 = EVIDENCE / "gate_silhouettes_1x.png"
    sheet.convert("RGB").save(out1)
    k = 4
    big = Image.new("RGB", (w * k, h * 2 * k + 16), LABEL_BG)
    big.paste(_up(sheet, k).convert("RGB"), (0, 16))
    d = ImageDraw.Draw(big)
    for x0, _cw, name in lefts:
        d.text((x0 * k, 3), name, fill=LABEL_FG)
    out4 = EVIDENCE / "gate_silhouettes_4x.png"
    big.save(out4)
    return [out1, out4]


def _mirror(img: Image.Image) -> tuple[Image.Image, tuple[int, int]]:
    from scripts.pixelart.common import flip
    return flip(img, PIVOT)


def _place(canvas: Image.Image, img: Image.Image, piv: tuple[int, int], x: int) -> None:
    canvas.alpha_composite(img, (x - piv[0], CONTACT_Y - piv[1]))


def _dust_trail(canvas: Image.Image, x: int, puffs: int) -> None:
    """The stand-in's committed trail, facing left: 3 x 3 puffs every 16 px behind the hind hoof (x + 14)."""
    over = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(over)
    for i in range(puffs):
        px = x + 14 + 16 * i
        d.rectangle((px - 1, CONTACT_Y - 2, px + 1, CONTACT_Y), fill=DUST[:3] + (DUST[3] - 30 * i,))
    canvas.alpha_composite(over)


def _lane(canvas: Image.Image, x: int) -> None:
    """Review annotation: the box's sweep from the braced pivot toward Carl (travel + box length), heights 2..30."""
    over = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(over)
    x0, x1 = x - TRAVEL - BOX[1], x - 1
    y0, y1 = CONTACT_Y - BOX[3] + 1, CONTACT_Y - BOX[2]
    d.rectangle((x0, y0, x1, y1), fill=LANE_FILL, outline=LANE_EDGE)
    canvas.alpha_composite(over)                      # blended, drawn over the sprites as an annotation


def _scene(llama: tuple[Image.Image, tuple[int, int]] | None, x: int, goblin=None, trail: int = 0,
           lane: bool = False) -> Image.Image:
    still = Image.open(STILL).convert("RGBA")
    if goblin is not None:
        _place(still, *goblin)
    if llama is not None:
        _place(still, llama[0], llama[1], x)
    if trail:
        _dust_trail(still, x, trail)
    if lane:
        _lane(still, x)
    return still


def write_corral(keys_grid: dict) -> list[Path]:
    from scripts.pixelart.common import flip
    frames = dict(_frames(keys_grid))
    cast = {n: (img, piv) for n, img, piv in _cast()}
    gob_img, gob_piv = flip(*cast["goblin"])
    goblin = (gob_img, gob_piv, CARL_PIVOT_X + 52)            # a GOB-01 between Carl and the llama, facing Carl
    at = CARL_PIVOT_X + APPROACH
    run_at = CARL_PIVOT_X + 64
    rows = [
        ("LLAMA-01 idle, d 112", _scene(_mirror(frames["LLAMA-01 idle"]), at)),
        ("spit tell, d 112", _scene(_mirror(frames["spit tell"]), at)),
        ("brace, d 112, with a goblin", _scene(_mirror(frames["brace"]), at, goblin=goblin)),
        ("scrape, d 112", _scene(_mirror(frames["scrape"]), at)),
        ("run_contact, d 64, trail mock", _scene(_mirror(frames["run_contact"]), run_at, trail=3)),
        ("recover_skid, d 64", _scene(_mirror(frames["recover_skid"]), run_at)),
        ("lane mock (review annotation)", _scene(_mirror(frames["brace"]), at, lane=True)),
    ]
    x0, y0, x1, y1 = STRIP
    sw, sh = x1 - x0, y1 - y0
    one = Image.new("RGB", (sw, (sh + 1) * len(rows) - 1), LABEL_BG)
    for i, (_n, sc) in enumerate(rows):
        one.paste(sc.crop(STRIP).convert("RGB"), (0, i * (sh + 1)))
    out1 = EVIDENCE / "gate_corral_strips_1x.png"
    one.save(out1)
    k = 3
    big = Image.new("RGB", (sw * k, (sh * k + 14) * len(rows)), LABEL_BG)
    d = ImageDraw.Draw(big)
    for i, (name, sc) in enumerate(rows):
        top = i * (sh * k + 14)
        d.text((4, top + 2), name, fill=LABEL_FG)
        big.paste(_up(sc.crop(STRIP), k).convert("RGB"), (0, top + 14))
    out3 = EVIDENCE / "gate_corral_strips_3x.png"
    big.save(out3)
    brace = EVIDENCE / "gate_corral_640x360_brace.png"
    rows[2][1].convert("RGB").save(brace)
    run = EVIDENCE / "gate_corral_640x360_run.png"
    rows[4][1].convert("RGB").save(run)
    return [out1, out3, brace, run]


def write_sums() -> Path:
    out = EVIDENCE / "SHA256SUMS"
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}" for p in sorted(EVIDENCE.glob("*.png"))]
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return out


def write_all(keys_grid: dict) -> list[Path]:
    import sys
    if str(ROOT) not in sys.path:
        sys.path.insert(0, str(ROOT))
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    paths = write_silhouettes(keys_grid) + write_corral(keys_grid)
    return paths + [write_sums()]
