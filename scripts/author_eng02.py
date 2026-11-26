#!/usr/bin/env python3
"""ENG-02 authoring record: the Goblin Engineer's get-up keys and the pot rules of his death set.

Usage:

    python3 scripts/author_eng02.py keys [--check]
    python3 scripts/author_eng02.py gate OUT.png

``keys`` (re)writes ``assets/source/characters/goblin/engineer/eng_02_keys.grid.json`` (and its ``.ase``) from the
rule-composed get-up sit (key grid frame 0, ENG-01 atlas frame 18) and the literal get-up shove rows below (key grid
frame 1, ENG-01 atlas frame 19). Both are frozen in ``tests/fixtures/eng-02_keys.json``. See
``docs/02-art/eng-02-spec.md`` sections 3 to 6.

Design stage: the key grid holds only these two frames. The executor appends them byte-identical to the ENG-01 atlas
as frames 18 and 19 (tag ``getup``), draws the two ram-run frames 20 and 21, and builds the death poses, the slammed
sheet and the parts sheet from the GOB-02 A and CF02 slammed A cels with the rules of this module (``pot_part``,
``flat_pot``, ``belt_recolour``) and of the spec.

Construction (no concept pixels traced or sampled):

* get-up sit (frame 18): the GOB-01 variant A get-up frame 23 body (``goblin_a_h36.grid.json``, layer ``body``), the
  rope belt recoloured to the ENG-01 rag belt with its iron buckle, and the ENG-01 ``POT`` block moved (-4, +11) so the
  pan sits jammed down over the eyes, rim on the snout, the teeth showing under it; no wrench (behind the hip);
* get-up shove (frame 19, hero key): the GOB-01 A get-up frame 24 crouch (legs rows 49 to 55, the bald crown rows 25 to
  28 that the lifted pot uncovers), the ENG-01 ``TORSO``, ``ARM``, ``WRENCH`` and ``HEAD`` blocks moved (0, +4), the
  ``POT`` block moved (-3, 0) and the far arm, new, raised behind the head with the fist on the pot handle: he levers
  the pot up off his eyes by its handle while he rises.

Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_eng01 as eng01  # noqa: E402
from author_eng01 import PARTS, Canvas, gob_body  # noqa: E402

ENG_DIR = ROOT / "assets/source/characters/goblin/engineer"
GRID = ENG_DIR / "eng_02_keys.grid.json"
ASE = ENG_DIR / "eng_02_keys.ase"
CANVAS = eng01.CANVAS
PIVOT = eng01.PIVOT
LAYERS = eng01.LAYERS
TICK_HZ = 60
ATLAS_INDEX = (18, 19)                    # where the executor appends the two keys in the ENG-01 atlas
TICKS = (10, 12)                          # the get-up window: GETUP_TICKS / q04_getup_ticks = 22
TAGS = [("getup", 0, 1)]

# eng-death-floor01-v0 (20 of 20): the 17 eng-floor01-v0 keys, unchanged, plus the three shared GOB-02 gore keys with
# the same hex. The death poses, the slammed sheet and the parts sheet use it; the get-up keys use only the first 17.
GORE_KEYS = [
    ("R", "red_mid", "#b8322eff", False),
    ("B", "blood_dark", "#6e1a22ff", False),
    ("P", "blood_wet", "#e2434bff", False),
]
DEATH_PALETTE = eng01.PALETTE + GORE_KEYS

# --------------------------------------------------------------------------- key grid frame 1: the shove (literal rows)
KEY_SHOVE = {
    16: "........................KKKKKKKKKKKKKK..........................",
    17: ".......................K666666666666KKK.........................",
    18: ".......................K56665555544432K.........................",
    19: "...........KKKK........K56765555544432K.........................",
    20: "..........K55K5KKKK....K56665555544432K.........................",
    21: "..........K3333mmm5KKKKK56665555544432K.........................",
    22: "...........KKKKsss35555K56665555544432K.........................",
    23: "..............KaaaK33336666666666555544K........................",
    24: "...............KssKKKKKKKKKKKKKKKKKKKKK.........................",
    25: "...............KssK..........KKlllllK...........................",
    26: "...............KssK.......KKKllmmmmmmKK.........................",
    27: "..............KsssK......KllmmmmmmmmmmmK........................",
    28: "..............KssKKKKK...KlmmmmmmmmmKKmmK.......................",
    29: "..............KsKKllaaKKKKsmmmmmmmmaaKKmmK......................",
    30: "..............KasKKssaaKKKsmmmmmmseeKaeKlmK.....................",
    31: "...............KamKKKKKKKKlmmmmmmmsaammmlmmK....................",
    32: "................KammK...KKlmmmmmssKKKKmmsmmsK...................",
    33: ".................KaamK..KKKlmmmmsKwwowKKmsssK...................",
    34: "..................KKamKKuuuKlmmsKooooowKKKKK....................",
    35: "....................KKuuuutrKssssKKKKKKsK.......................",
    36: "....................KuutttttrKsssssssssK........................",
    37: "....................KuttttttrrKKKKKKKKK.........................",
    38: "....................KuttttttrrKlmKmmmsK.........................",
    39: "...............K..KKutttrtttrKlmsKmmmmsK........................",
    40: "..............K5..4KuttttrttrKlmsKmmmsK.........................",
    41: "..............K6KK4KutttttrtrKlmsKmmmKKKK.......................",
    42: "..............K6554KuttttttrrKlmsKmmKllmmK......................",
    43: "...............K65KKutttttrrrKlmmKKmmmmmK.......................",
    44: "................K5KKrtrKrtrrrKslmmmsssKK........................",
    45: ".................K4KrrKKtrrKrrKssssKKK..........................",
    46: ".................KKKuutuutuutuK56KuK............................",
    47: "...................KrrrrrrrtrrrrrrK.............................",
    48: "...................KrrrrrrtrrrrtrrK.............................",
    49: "....................KrrrrrtKKrrtrrrKKK..........................",
    50: ".....................KKKKKK..KKKKKKKaaK.........................",
    51: "...................................KaaK.........................",
    52: ".................KK.............KKKKmaKK........................",
    53: ".................KlK......KKKKKKKllKKmaaKKK.....................",
    54: ".................KllKKKKKKllllllllmmKKmmaaaK....................",
    55: ".................KsssssssssssssssssK..KKKKKKK...................",
}
WRENCH_SHOVE = (14, 39, 18, 46)           # ENG-01 WRENCH box moved (0, +4): the same body/prop split rule as key 0


def frame_shove() -> dict[str, list[str]]:
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    for y, row in KEY_SHOVE.items():
        if len(row) != w:
            raise ValueError(f"row {y} is {len(row)} px wide")
        for x, ch in enumerate(row):
            if ch == ".":
                continue
            if ch not in eng01.KEYS:
                raise ValueError(f"unknown key {ch!r} at ({x}, {y})")
            cels["prop" if eng01.in_box(x, y, WRENCH_SHOVE) else "body"][y][x] = ch
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}


# --------------------------------------------------------------------------- rules shared with the executor
BELT_BUCKLE = ("5", "6")


def belt_recolour(pix: dict, buckle_at: tuple[int, int] | None = None) -> dict:
    """GOB rope belt to the ENG-01 rag belt: ``x``/``y`` become ``u``, every third pixel along the row ``t`` (the
    ENG-01 key 0 row 42 rhythm ``uut``); ``buckle_at`` (the left pixel) gets the 2 px iron buckle ``56``."""
    out = dict(pix)
    for (x, y), ch in pix.items():
        if ch in ("x", "y"):
            out[(x, y)] = "u" if x % 3 else "t"
    if buckle_at:
        bx, by = buckle_at
        out[(bx, by)], out[(bx + 1, by)] = BELT_BUCKLE
    return out


def frame_sit() -> dict[str, list[str]]:
    """Get-up sit, key grid frame 0 (atlas 18): GOB-01 A frame 23 body, the ENG belt, the pot jammed over the eyes."""
    c = Canvas()
    c.paint(belt_recolour(gob_body(23), buckle_at=(29, 50)))
    c.block("POT", -4, 11)
    return c.cels()


def pot_part() -> dict:
    """The ``pot`` part cel (32 x 32, pivot (16, 16)): the ENG-01 ``POT`` block (x 13 to 42, rows 16 to 24), whole and
    unrotated, moved (-12, -4) so the block sits in cell columns 1 to 30, rows 12 to 20. The presenter spins it."""
    return {(x - 12, y - 4): ch for (x, y), ch in PARTS["POT"].items()}


FLAT_ROWS = (16, 17, 23, 24)              # the pan's rim top, light rim band, lower band and lip


def flat_pot() -> dict:
    """The flattened pot of the crush and the stomp: a 4-row pancake of the ``POT`` block at the block's own width (iron
    does not stretch). Rows 16, 17, 23 and 24 of the pan are stacked; the handle keeps only its row 21, laid on the
    third pancake row left of the pan. Origin: the block's top-left (13, 16) maps to (0, 0)."""
    out: dict = {}
    pot = PARTS["POT"]
    pan_left = 26                                        # the pan's left outline column in key 0
    for i, src in enumerate(FLAT_ROWS):
        for (x, y), ch in pot.items():
            if y == src and x >= pan_left - (2 if src >= 23 else 0):
                out[(x - 13, i)] = ch
    for (x, y), ch in pot.items():
        if y == 21 and x < pan_left:
            out.setdefault((x - 13, 2), ch)
    for (x, y), ch in list(out.items()):                 # the handle line gets its outline above and below
        if y == 2 and ch != "K":
            out.setdefault((x, 1), "K")
            out.setdefault((x, 3), "K")
    return out


# --------------------------------------------------------------------------- grid
def build_frames() -> list[dict]:
    return [frame_sit(), frame_shove()]


def duration_ms(ticks: int) -> int:
    return round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    frames = build_frames()
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": [dict({"key": k, "name": n, "rgba": c}, **({"accent": True} if acc else {}))
                    for k, n, c, acc in eng01.PALETTE],
        "layers": LAYERS,
        "frames": [{"duration_ms": duration_ms(t), "cels": cels} for t, cels in zip(TICKS, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


# --------------------------------------------------------------------------- gate image (review only, not committed)
def write_gate(out: Path) -> Path:
    """ENG-01 idle (0) and down (17) next to the two get-up keys, plus the pot part and the flattened pot rule previews:
    4x colour, 4x black silhouette, and a 1x strip on the Floor 1 ground tone."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import grid_images, label, load_grid_file

    eng = grid_images(load_grid_file(eng01.GRID))
    keys = grid_images(frames_grid())
    rgba = {k: tuple(int(c[i:i + 2], 16) for i in (1, 3, 5, 7)) for k, _n, c, _a in DEATH_PALETTE}

    def from_pix(pix: dict, size=(32, 32), at=(0, 0)) -> Image.Image:
        im = Image.new("RGBA", size, (0, 0, 0, 0))
        for (x, y), ch in pix.items():
            im.putpixel((x + at[0], y + at[1]), rgba[ch])
        return im

    cells = [("ENG-01 idle 0", eng[0]), ("ENG-01 down 17", eng[17]), ("18 getup sit", keys[0]),
             ("19 getup shove (hero)", keys[1])]
    extras = [("pot part (rule)", from_pix(pot_part())), ("flat pot (rule)", from_pix(flat_pot(), (32, 8)))]
    k = 4
    bg, sil_bg, floor = (58, 34, 30, 255), (236, 232, 240, 255), (74, 68, 100, 255)
    cw, ch = 64 * k, 56 * k
    pad, top = 12, 22
    width = len(cells) * (cw + pad) + 2 * (32 * k + pad) + pad
    height = top + ch + top + ch + top + 56 + 16
    sheet = Image.new("RGBA", (width, height), (24, 23, 34, 255))
    d = ImageDraw.Draw(sheet)
    for i, (name, im) in enumerate(cells):
        x = pad + i * (cw + pad)
        label(d, (x, 4), name)
        sheet.paste(Image.new("RGBA", (cw, ch), bg), (x, top))
        big = im.resize((cw, ch), Image.NEAREST)
        sheet.paste(big, (x, top), big)
        sil = Image.new("RGBA", (cw, ch), sil_bg)
        mask = big.split()[3]
        sil.paste(Image.new("RGBA", (cw, ch), (0, 0, 0, 255)), (0, 0), mask)
        sheet.paste(sil, (x, top + ch + top))
    x = pad + len(cells) * (cw + pad)
    for name, im in extras:
        label(d, (x, 4), name)
        sheet.paste(Image.new("RGBA", (32 * k, 32 * k), bg), (x, top))
        big = im.resize((im.width * k, im.height * k), Image.NEAREST)
        sheet.paste(big, (x, top), big)
        x += 32 * k + pad
    y1 = top + ch + top + ch + top - 16
    label(d, (pad, y1), "1x on the Floor 1 ground tone: idle, down, sit, shove")
    strip = Image.new("RGBA", (64 * 4 + 24, 56), floor)
    for i, (_n, im) in enumerate(cells):
        strip.paste(im, (i * 64 + 6 * i, 0), im)
    sheet.paste(strip, (pad, y1 + 16))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(out)
    return out



# =========================================================================== execution (docs/02-art/eng-02-spec.md)
# Everything below is built from committed cels by the rules of the spec: ENG-01 atlas frames 18 to 21 (section 6), the
# slammed sheet (section 7), the death poses (section 8) and the parts sheet (section 9). Nothing is hand-edited.
import copy  # noqa: E402

import author_gob02_death as ad  # noqa: E402

Pix = dict
DEATH_DIR = ROOT / "assets/source/characters/goblin/death"
SLAMMED_A = DEATH_DIR / "goblin_a_slammed_h36.grid.json"
SLAMMED = ENG_DIR / "eng_slammed_h36.grid.json"
SLAMMED_ASE = ENG_DIR / "eng_slammed_h36.ase"
DEATH = ENG_DIR / "eng_death_h36.grid.json"
DEATH_ASE = ENG_DIR / "eng_death_h36.ase"
PARTS_GRID = ENG_DIR / "eng_death_parts.grid.json"
PARTS_ASE = ENG_DIR / "eng_death_parts.ase"
MANIFESTS = ROOT / "assets/manifests"
SPEC = "docs/02-art/eng-02-spec.md"
DEATH_PALETTE_ID = "eng-death-floor01-v0"
ALWAYS_SHOWN_LAYERS = ["body", "prop", "pot"]       # documentation only: Rust draws every non-gore layer


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def cel_pix(rows: list[str]) -> Pix:
    return ad.cel_pix(rows)


DEATH_KEYS = {k for k, *_ in DEATH_PALETTE}


def rows_of(pix: Pix, size: tuple[int, int] = CANVAS) -> list[str]:
    w, h = size
    grid = [["."] * w for _ in range(h)]
    for (x, y), ch in pix.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel outside the {w}x{h} canvas: {(x, y)}")
        if ch not in DEATH_KEYS:
            raise ValueError(f"unknown key {ch!r}")
        grid[y][x] = ch
    return ["".join(r) for r in grid]


# --------------------------------------------------------------------------- ENG-01 atlas frames 18 to 21
RAM_RUN_TICKS = (4, 4)                    # the state clock (ram age) plays [12, 20, 21, 20] at 4 ticks a frame
RAM_CYCLE = (12, 20, 21, 20)


def _split_legs(cels: dict[str, list[str]], hem_row: int) -> tuple[Pix, Pix, Pix]:
    """``(torso side, legs, prop)`` of an ENG-01 frame: body pixels on or above ``hem_row`` and below it."""
    body = cel_pix(cels["body"])
    return ({p: c for p, c in body.items() if p[1] <= hem_row}, {p: c for p, c in body.items() if p[1] > hem_row},
            cel_pix(cels["prop"]))


def _components(pix: Pix) -> list[Pix]:
    left, out = dict(pix), []
    while left:
        start = next(iter(left))
        comp, todo = {}, [start]
        while todo:
            p = todo.pop()
            if p not in left:
                continue
            comp[p] = left.pop(p)
            todo += [(p[0] + a, p[1] + b) for a in (-1, 0, 1) for b in (-1, 0, 1) if a or b]
        out.append(comp)
    return out


def extend_thighs(legs: Pix, top_row: int) -> Pix:
    """Extend each leg upward to ``top_row`` by repeating its own shaft row (the row under the knee bump when the top
    row is wider): the thigh keeps its own outline and fill columns."""
    out = dict(legs)
    for comp in _components(legs):
        y0 = min(y for _, y in comp)
        row0 = {x: c for (x, y), c in comp.items() if y == y0}
        row1 = {x: c for (x, y), c in comp.items() if y == y0 + 1}
        template = row1 if row1 and len(row1) < len(row0) else row0
        for y in range(top_row, y0):
            for x, c in template.items():
                out[(x, y)] = c
    return out


def ram_run_frame(n: int) -> dict[str, list[str]]:
    """Frame 20 (passing): frame 12's blocks moved (0, -1) and the ENG-01 walk 7 legs. Frame 21 (opposite contact): the
    unmoved frame 12 blocks and the walk 6 legs. Thighs extended up to the hip outline (spec section 6)."""
    f12 = eng01.f_ram12()
    torso, _legs12, wrench = _split_legs(f12, 46)
    walk_n, dy = (7, -1) if n == 20 else (6, 0)
    walk = eng01.f_walk(walk_n)
    _t, walk_legs, _p = _split_legs(walk, 46 + eng01.WALK[walk_n][1])
    legs = extend_thighs(walk_legs, 47 + dy)
    c = Canvas()
    c.paint(torso, 0, dy)
    c.paint(wrench, 0, dy, "prop")
    c.paint(legs)
    return c.cels()


def atlas_frames() -> list[dict[str, list[str]]]:
    """ENG-01 atlas frames 18 to 21: the two frozen keys byte for byte, then the ram-run pair."""
    return build_frames() + [ram_run_frame(20), ram_run_frame(21)]


# --------------------------------------------------------------------------- shared rules: strap, buckle, belt
STRAP = "xy"


def strap_runs(pix: Pix) -> list[tuple[int, int, int]]:
    """Horizontal runs ``(y, x0, n)`` of strap pixels (keys ``x`` and ``y``)."""
    runs = []
    for y in sorted({y for (_x, y), ch in pix.items() if ch in STRAP}):
        xs = sorted(x for (x, yy), ch in pix.items() if yy == y and ch in STRAP)
        start = prev = None
        for x in xs + [None]:
            if start is None:
                start = prev = x
            elif x is not None and x == prev + 1:
                prev = x
            else:
                runs.append((y, start, prev - start + 1))
                start = prev = x
    return runs


def buckle_for(pix: Pix) -> tuple[int, int] | None:
    """Spec section 7: the left pixel of the 2 px buckle, at index ``floor((n - 2) / 2)`` of the longest horizontal
    strap run of length n >= 4, on that run's row; ``None`` when no run is that long."""
    runs = [r for r in strap_runs(pix) if r[2] >= 4]
    if not runs:
        return None
    y, x0, n = max(runs, key=lambda r: (r[2], -r[0]))
    return (x0 + (n - 2) // 2, y)


def carried_strap(weapon: Pix) -> Pix:
    """EXECUTOR NOTE ENG2-X1. In GOB-02 A death frames 3 and 4 the belt strap (an 18 px horizontal run and the outline pixel
    at its right end) is drawn on the ``weapon`` layer, not on ``body``. Dropping the layer would cut a transparent slit
    through the torso, so the strap run is carried to ``body`` (and belt-recoloured, no buckle: spec section 7 pins none
    in frames 3 to 5). Every other weapon pixel (the club) is dropped."""
    out: Pix = {}
    for y, x0, n in strap_runs(weapon):
        if n >= 4:
            for x in range(x0, x0 + n):
                out[(x, y)] = weapon[(x, y)]
            if weapon.get((x0 + n, y)) == "K":
                out[(x0 + n, y)] = "K"
    return out


def belt_pixels(pix: Pix) -> list[tuple[int, int]]:
    return [p for p, ch in pix.items() if ch in STRAP]


def rearmost(points: list[tuple[int, int]]) -> tuple[int, int]:
    return min(points, key=lambda p: (p[0], -p[1]))


def _palette(accent_from: dict | None = None) -> list[dict]:
    gob = {p["key"]: p for p in load(ad.POSES)["palette"]}
    out = []
    for k, n, c, acc in DEATH_PALETTE:
        entry = {"key": k, "name": n, "rgba": c}
        if acc or gob.get(k, {}).get("accent"):
            entry["accent"] = True
        out.append(entry)
    return out


def grid_of(frames: list[dict], durations: list[int], layers: list[str], tags: list[tuple[str, int, int]],
            canvas=CANVAS, pivot=PIVOT) -> dict:
    return {
        "format": "pixel-grid-v1",
        "canvas": list(canvas),
        "pivot": list(pivot),
        "palette": _palette(),
        "layers": layers,
        "frames": [{"duration_ms": d, "cels": c} for d, c in zip(durations, frames)],
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in tags],
    }


# --------------------------------------------------------------------------- slammed sheet (spec section 7)
def slammed_frames() -> list[dict[str, list[str]]]:
    out = []
    for frame in load(SLAMMED_A)["frames"]:
        c = frame["cels"]
        body = cel_pix(c["body"])
        out.append({"body": rows_of(belt_recolour(body, buckle_for(body))), "gore": c["gore"]})
    return out


def slammed_grid() -> dict:
    a = load(SLAMMED_A)
    return grid_of(slammed_frames(), [f["duration_ms"] for f in a["frames"]], ["body", "gore"],
                   [(t["name"], t["from"], t["to"]) for t in a["tags"]])


# --------------------------------------------------------------------------- orphan clean-up (spec section 13)
MAX_ORPHANS = 9                           # V06 limit of every ENG-02 sheet
ACCENT_KEYS = {k for k, *_rest in DEATH_PALETTE if k in ("e", "w", "7", "P")}


def _composite(layers: dict[str, Pix]) -> Pix:
    out: Pix = {}
    for pix in layers.values():                       # back to front, the dict order
        out.update(pix)
    return out


def orphans_of(flat: Pix) -> list[tuple[int, int]]:
    """Orphan pixels as the V06 validator counts them: not isolated, not an accent key, no same-key 8-neighbour."""
    out = []
    for (x, y), ch in flat.items():
        around = [flat[(x + a, y + b)] for a in (-1, 0, 1) for b in (-1, 0, 1) if (a or b) and (x + a, y + b) in flat]
        if around and ch not in ACCENT_KEYS and ch not in around:
            out.append((x, y))
    return sorted(out, key=lambda p: (p[1], p[0]))


def source_orphans(a_cels: dict, _unused=None) -> set:
    """``{(x, y, key)}`` of the GOB source composite's orphans: these are inherited and never touched."""
    flat = _composite({layer: cel_pix(rows) for layer, rows in a_cels.items()})
    return {(x, y, flat[(x, y)]) for x, y in orphans_of(flat)}


def clean_orphans(layers: dict[str, Pix], inherited: set, limit: int = MAX_ORPHANS) -> list[tuple]:
    """Spec section 13 (recorded): while a frame has more orphans than the V06 limit, recolour the first body-layer
    orphan that is not inherited from the GOB source to the commonest key among its 8 neighbours (a belt or rag pixel only among the
    rag keys ``u``, ``t`` and ``r``). Returns ``[(x, y, old, new), ...]`` in the order applied."""
    log = []
    while True:
        flat = _composite(layers)
        orphans = orphans_of(flat)
        if len(orphans) <= limit:
            return log
        todo = [p for p in orphans if p in layers["body"] and (p[0], p[1], flat[p]) not in inherited]
        if not todo:
            return log
        x, y = todo[0]
        counts: dict = {}
        for a in (-1, 0, 1):
            for b in (-1, 0, 1):
                n = flat.get((x + a, y + b))
                if (a or b) and n and (flat[(x, y)] not in "utr" or n in "utr"):    # a belt pixel stays in the rag family
                    counts[n] = counts.get(n, 0) + 1
        new = max(counts, key=lambda k: (counts[k], k != "K", k)) if counts else "u"
        log.append((x, y, flat[(x, y)], new))
        layers["body"][(x, y)] = new


# --------------------------------------------------------------------------- death poses (spec section 8)
POT_ORIGINS = {0: (6, 33), 1: (6, 36), 2: (28, 41)}      # flat_pot() origins, crush frames 0 to 2
WRENCH_BEHIND = 2                                        # right edge of the wrench this many px behind the rearmost belt pixel
WRENCH_SLIDE = 2                                         # it may slide back this many more px to clear the body outline


def stowed_wrench(belt_row: int, rear_x: int, behind: int = WRENCH_BEHIND) -> Pix:
    """The ENG-01 ``WRENCH`` block (box x 14 to 18, rows 35 to 42, belt row 42 in key 0) stowed jaw-up at the back hip:
    bottom row on the belt row and right edge ``behind`` px behind the rearmost belt pixel, as in key 0 (20 vs 18)."""
    x0, y0, x1, y1 = eng01.WRENCH
    return {(x + (rear_x - behind - x1), y + (belt_row - y1)): ch for (x, y), ch in PARTS["WRENCH"].items()}


def pot_at(frame: int) -> Pix:
    ox, oy = POT_ORIGINS[frame]
    return {(x + ox, y + oy): ch for (x, y), ch in flat_pot().items()}


def death_frame(i: int) -> tuple[dict[str, list[str]], dict]:
    """``(cels, info)`` of death frame ``i``. ``info`` carries the belt, the buckle, the rearmost belt pixel and the
    stowed wrench state (used by the manifest and the tests)."""
    a = load(ad.POSES)["frames"][i]["cels"]
    body, weapon, gore = cel_pix(a["body"]), cel_pix(a["weapon"]), a["gore"]
    buckle = buckle_for(body)                                          # from the body layer only (spec section 7)
    body.update(carried_strap(weapon))
    belt = belt_pixels(body)
    body = belt_recolour(body, buckle)
    prop: Pix = {}
    pot: Pix = {}
    if i in POT_ORIGINS:
        pot = pot_at(i)
        for p in pot:
            body.pop(p, None)
    hip = rearmost(belt) if belt else None
    if hip is None:                                                    # frame 5: no belt; the rearmost rag pixel
        rag = [p for p, ch in body.items() if ch in "rtu"]
        hip = rearmost(rag)
    wrench_behind = None
    if i >= 3:
        gore_px = cel_pix(gore)
        for behind in range(WRENCH_BEHIND, WRENCH_BEHIND + WRENCH_SLIDE + 1):       # slide back until it clears the body
            w = stowed_wrench(hip[1], hip[0], behind)
            if all(p not in body and p not in gore_px for p in w):
                prop, wrench_behind = w, behind
                break
    layers = {"body": body, "prop": prop, "pot": pot, "gore": cel_pix(gore)}
    cleaned = clean_orphans(layers, source_orphans(a, load(ad.POSES)["frames"][i]["cels"]))
    cels = {"body": rows_of(body), "prop": rows_of(prop), "pot": rows_of(pot), "gore": gore}
    return cels, {"buckle": buckle, "hip_back": hip, "wrench_behind": wrench_behind, "pot_origin": POT_ORIGINS.get(i),
                  "cleaned_orphans": cleaned}


def death_grid() -> dict:
    a = load(ad.POSES)
    frames = [death_frame(i)[0] for i in range(len(a["frames"]))]
    return grid_of(frames, [f["duration_ms"] for f in a["frames"]], ["body", "prop", "pot", "gore"],
                   [(t["name"], t["from"], t["to"]) for t in a["tags"]])


# --------------------------------------------------------------------------- parts sheet (spec section 9)
DROPPED_PARTS = ("club", "club_stick", "pineapple")
NEW_PARTS = ("pot", "head_bare", "head_bare_dead", "wrench")
HEAD_MOVE = (4, 2)                        # the GOB head cel moves (+4, +2); the pot sits at cell = frame - (13, 9)
POT_IN_HEAD = (-13, -9)
WRENCH_CELL = (0, -22)


def _shift_rows(rows: list[str], dx: int, dy: int, size=ad.CELL) -> list[str]:
    return rows_of({(x + dx, y + dy): ch for (x, y), ch in cel_pix(rows).items()}, size)


def part_cels(pid: str) -> dict[str, list[str]]:
    a = load(ad.PARTS)
    ids = [t["name"] for t in a["tags"]]
    size = ad.CELL
    if pid in ("pot", "wrench"):
        pix = pot_part() if pid == "pot" else {(x + WRENCH_CELL[0], y + WRENCH_CELL[1]): ch
                                               for (x, y), ch in PARTS["WRENCH"].items()}
        empty = rows_of({}, size)
        return {"body": empty, "prop": rows_of(pix, size), "gore": empty}
    src = {"head_bare": "head", "head_bare_dead": "head_dead"}.get(pid, pid)
    c = a["frames"][ids.index(src)]["cels"]
    assert not any(r.strip(".") for r in c["weapon"]), pid
    body, gore = cel_pix(c["body"]), c["gore"]
    prop: Pix = {}
    if pid in ("head", "head_dead"):
        body = {(x + HEAD_MOVE[0], y + HEAD_MOVE[1]): ch for (x, y), ch in body.items()}
        gore = _shift_rows(gore, *HEAD_MOVE)
        prop = {(x + POT_IN_HEAD[0], y + POT_IN_HEAD[1]): ch for (x, y), ch in PARTS["POT"].items()}
        for p in prop:
            body.pop(p, None)
    elif pid == "torso_lower":
        body = belt_recolour(body)
    return {"body": rows_of(body, size), "prop": rows_of(prop, size), "gore": gore}


def parts_ids() -> list[str]:
    ids = [t["name"] for t in load(ad.PARTS)["tags"] if t["name"] not in DROPPED_PARTS]
    return ids + list(NEW_PARTS)


def parts_grid() -> dict:
    ids = parts_ids()
    frames = [part_cels(pid) for pid in ids]
    return grid_of(frames, [100] * len(ids), ["body", "prop", "gore"], [(pid, i, i) for i, pid in enumerate(ids)],
                   canvas=ad.CELL, pivot=ad.CELL_PIVOT)


# --------------------------------------------------------------------------- manifests
PROVENANCE = {
    "origin": ("ENG-02 Goblin Engineer completion art (" + SPEC + "): cels derived by scripts/author_eng02.py from "
               "committed GOB-02 A, GOB-01 A CF02 and ENG-01 cels with the spec's belt, buckle, pot and wrench rules; "
               "no image-model output, no tracing"),
    "generator": ("Design, spec and the two get-up keys: Claude Opus. Design gate: Claude Opus (independent). "
                  "Execution, atlases, manifests, validators and evidence: Claude Sonnet. Independent review pending"),
    "generated_art": True,
    "input_reference_ids": ["floor-01-opening-concept-references-v1"],
}
OWNER = ("AI-authored enemy art (fan-concept layer: Floor 1 Goblin Engineer; runtime role IP-neutral foreman, "
         "pot-ram placeholder)")


def _base_manifest(mid: str, stem: str, grid_path: Path, ase: Path, frame, pivot, tags: list[str]) -> dict:
    return {
        "schema_version": 1, "id": mid, "type": "character_animation",
        "source_path": str(ase.relative_to(ROOT)), "grid_source": str(grid_path.relative_to(ROOT)),
        "output_atlas": f"generated/art/{stem}.png", "output_metadata": f"generated/art/{stem}.json",
        "palette_id": DEATH_PALETTE_ID, "frame_size": list(frame), "pivot": list(pivot),
        "animation_tags": tags, "required_alpha": True, "owner": OWNER,
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": copy.deepcopy(PROVENANCE),
        "license": ad.LICENSE, "review_status": "prototype_fixture_unapproved",
    }


def slammed_manifest() -> dict:
    a = load(MANIFESTS / "goblin_a_slammed_h36.json")
    g = slammed_grid()
    m = _base_manifest("eng_slammed_h36_candidate_v1", "eng_slammed_h36", SLAMMED, SLAMMED_ASE, CANVAS, PIVOT,
                       [t["name"] for t in g["tags"]])
    c = copy.deepcopy(a["cf02_goblin"])
    c.update({
        "spec": SPEC, "variant": "ENG", "layers": ["body", "gore"], "key_frames": [],
        "status": "executor pass complete; independent review pending; wired into the Bevy scene through "
                  "combat_anim::engineer_slammed_pose",
        "base_sheet": "assets/manifests/goblin_a_slammed_h36.json",
        "derivation": "GOB A slammed cels (CF02): weapon layer dropped, belt recoloured, buckle by the spec section 7 rule, "
                      "gore kept; the pot and the wrench are launched parts (ENG2-AD6), launch points in "
                      "eng_death_h36.json death_poses.spawner_hints.slam",
    })
    for k in ("key_fixture", "key_source", "key_source_frames"):
        c.pop(k, None)
    for f in c["frames"]:
        f["author"] = "executor"
    c["finishes"]["wall"]["club"] = "the wrench leaves the hip and the pot pops off; the presenter launches both parts"
    c["finishes"]["flung"]["club"] = "the wrench and the pot launch on landing (ENG2-AD20); the airborne base sprite wears the pot"
    c["finishes"]["stomped"]["scene_prop"] = "the stamping foot covers the belly in the scene (CF02-D); the pot pops off"
    m["cf02_goblin"] = c
    return m


def _frame_points() -> dict:
    """Launch points measured from the impact cels (spec section 7)."""
    out = {}
    frames = slammed_frames()
    for pose, frame in (("stomped", 0), ("wall", 2), ("flung", 4)):
        body = cel_pix(frames[frame]["body"])
        out[pose] = {"frame": frame, "pot_launch": list(head_top(body)), "wrench_launch": list(hip_point(body))}
    return out


def head_top(body: Pix) -> tuple[int, int]:
    """The head's top outline pixel nearest the head centre. Head centre: the centroid of the mouth and teeth pixels
    (``w`` and ``o``); candidates: ``K`` pixels with empty space above that are no lower than the centre."""
    mouth = [p for p, ch in body.items() if ch in "wo"]
    cx = sum(x for x, _ in mouth) / len(mouth)
    cy = sum(y for _, y in mouth) / len(mouth)
    cand = [(x, y) for (x, y), ch in body.items()
            if ch == "K" and (x, y - 1) not in body and y <= cy and abs(x - cx) <= 14 and abs(y - cy) <= 14]
    return min(cand, key=lambda p: ((p[0] - cx) ** 2 + (p[1] - cy) ** 2, p[1], p[0]))


def hip_point(body: Pix) -> tuple[int, int]:
    """The rearmost belt pixel; a cel without a belt (no strap in the GOB cel) uses its rearmost rag pixel."""
    belt = [p for p, ch in body.items() if ch in "ut" and any(body.get((p[0] + dx, p[1])) in ("u", "t")
                                                              for dx in (-1, 1))]
    return rearmost(belt) if belt else rearmost([p for p, ch in body.items() if ch in "rtu"])


SLAM_VELOCITIES = {
    "stomped": {"pot": {"vx_away": [20, 50], "vy_up": [150, 210], "spin_dps": 720},
                "wrench": {"vx_away": [-30, 60], "vy_up": [110, 170], "spin_dps": 900}, "timing": "age 0"},
    "wall": {"pot": {"vx_away": [-80, -40], "vy_up": [160, 220], "spin_dps": 720},
             "wrench": {"vx_away": [-30, 60], "vy_up": [110, 170], "spin_dps": 900}, "timing": "age 0"},
    "flung": {"pot": {"vx_away": [120, 170], "vy_up": [90, 140], "spin_dps": 720},
              "wrench": {"vx_away": [-30, 60], "vy_up": [110, 170], "spin_dps": 900},
              "timing": "first tick the slammed pose is shown (the landing)"},
}


def death_manifest() -> dict:
    a = load(MANIFESTS / "goblin_a_death_h36.json")
    g = death_grid()
    m = _base_manifest("eng_death_h36_candidate_v1", "eng_death_h36", DEATH, DEATH_ASE, CANVAS, PIVOT,
                       [t["name"] for t in g["tags"]])
    d = copy.deepcopy(a["death_poses"])
    frames = []
    for i, f in enumerate(d["frames"]):
        _cels, info = death_frame(i)
        f = dict(f)
        f["author"] = "executor"
        f["anchors"] = dict(f["anchors"])
        f["anchors"]["hip_back"] = list(info["hip_back"])
        frames.append(f)
    hints = d["spawner_hints"]
    hints["crush"].pop("club", None)
    hints["crush"]["wrench"] = 1
    hints["slam"] = {"note": "Engineer only (ENG2-AD6, ENG2-AD20): the pot and the wrench are launched parts; points are "
                             "frame pixels of the 64 x 56 pivot (28, 55) frame, measured on the slammed impact cel",
                     "poses": {pose: dict(v, **SLAM_VELOCITIES[pose]) for pose, v in _frame_points().items()}}
    d.update({"spec": SPEC, "base_spec": "docs/02-art/gob-02-death-spec.md", "variant": "ENG", "layers": g["layers"],
              "always_shown_layers": ALWAYS_SHOWN_LAYERS, "frames": frames, "spawner_hints": hints,
              "status": "executor pass complete; independent review pending; wired into the Bevy presenter",
              "key_frames": []})
    d.pop("key_fixture", None)
    m["death_poses"] = d
    return m


PART_FIELDS = {
    "pot": dict(anchors={"grip": [2, 17]}, frame0_origin=[28, 20], cls="clean", material="iron", mass="medium",
                spin="fast", settle="clatter", bounce=0.45, presets=[1, 2, 3, 4], ttl="keep"),
    "wrench": dict(anchors={"grip": [16, 20]}, frame0_origin=[16, 38], cls="clean", material="iron", mass="light",
                   spin="fast", settle="clatter", bounce=0.3, presets=[1, 2, 3, 4], ttl="keep"),
}


def parts_manifest() -> dict:
    a = load(MANIFESTS / "goblin_a_death_parts.json")
    g = parts_grid()
    ids = [t["name"] for t in g["tags"]]
    m = _base_manifest("eng_death_parts_v1", "eng_death_parts", PARTS_GRID, PARTS_ASE, ad.CELL, ad.CELL_PIVOT, ids)
    d = copy.deepcopy(a["death_parts"])
    by_id = {p["id"]: p for p in d["parts"]}
    parts = []
    for i, pid in enumerate(ids):
        if pid in by_id:
            p = copy.deepcopy(by_id[pid])
            if pid in ("head", "head_dead"):
                p["frame0_origin"] = [29, 25]
                p["anchors"] = {"neck": [15, 22]}
        elif pid in ("head_bare", "head_bare_dead"):
            p = copy.deepcopy(by_id[pid.replace("_bare", "")])
            p["id"] = pid
            p["presets"] = [2, 3, 4]
            if pid == "head_bare":
                p["swap_on_first_contact"] = "head_bare_dead"
        else:
            f = PART_FIELDS[pid]
            p = {"id": pid, "frame": 0, "cell": [32, 32], "pivot": [16, 16], "anchors": f["anchors"],
                 "frame0_origin": f["frame0_origin"], "class": f["cls"], "material": f["material"],
                 "presets": f["presets"], "gore_layer": False, "mass": f["mass"], "spin": f["spin"],
                 "settle": f["settle"], "bounce": f["bounce"], "ttl": f["ttl"]}
        p["frame"] = i
        parts.append(p)
    d.update({"spec": SPEC, "base_spec": "docs/02-art/gob-02-death-spec.md", "variant": "ENG", "layers": g["layers"],
              "always_shown_layers": ALWAYS_SHOWN_LAYERS, "parts": parts, "key_frames": [],
              "status": "executor pass complete; independent review pending; wired into the Bevy presenter"})
    d.pop("key_fixture", None)
    m["death_parts"] = d
    return m


def outputs() -> list[tuple[Path, Path, dict, Path, dict]]:
    return [(SLAMMED, SLAMMED_ASE, slammed_grid(), MANIFESTS / "eng_slammed_h36.json", slammed_manifest()),
            (DEATH, DEATH_ASE, death_grid(), MANIFESTS / "eng_death_h36.json", death_manifest()),
            (PARTS_GRID, PARTS_ASE, parts_grid(), MANIFESTS / "eng_death_parts.json", parts_manifest())]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "gate", "sheets"])
    parser.add_argument("out", nargs="?", help="gate: output PNG path")
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "gate":
        if not args.out:
            parser.error("gate needs an output path")
        print(f"wrote {write_gate(Path(args.out))}")
        return 0
    if args.command == "sheets":
        outs = outputs()
        if args.check:
            same = all(g.is_file() and g.read_text(encoding="utf-8") == dump(grid) and m.is_file()
                       and m.read_text(encoding="utf-8") == json.dumps(man, indent=2) + "\n"
                       for g, _a, grid, m, man in outs)
            print("ENG-02 sheets up to date" if same else "ENG-02 sheets differ from the generator")
            return 0 if same else 1
        import pixel_grid_to_ase as pg  # noqa: E402
        for g, ase, grid, m, man in outs:
            g.write_text(dump(grid), encoding="utf-8")
            pg.write_ase(grid, ase)
            m.write_text(json.dumps(man, indent=2) + "\n", encoding="utf-8")
            print(f"wrote {g.relative_to(ROOT)}, {ase.relative_to(ROOT)}, {m.relative_to(ROOT)}")
        return 0
    text = dump(frames_grid())
    if args.check:
        same = GRID.is_file() and GRID.read_text(encoding="utf-8") == text
        print(f"{GRID.name} up to date" if same else f"{GRID.name} differs from the generator")
        return 0 if same else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    import pixel_grid_to_ase as pg  # noqa: E402
    pg.write_ase(frames_grid(), ASE)
    print(f"wrote {GRID.relative_to(ROOT)}, {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
