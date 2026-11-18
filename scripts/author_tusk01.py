#!/usr/bin/env python3
"""Author TUSK-01 Tuskling (Floor 01 arena minion) hero key as pixel-grid-v1 sources (deterministic).

Design spec: ``docs/02-art/tusk-01-spec.md``. One base Tuskling (layer ``body``) wearing the tux costume (layer
``costume``); the gown variant is a rule, not a second drawing: ``variant(rows, "gown")`` remaps the costume keys and
swaps the bow tie for a pearl string and an ear flower (spec section 6.1). Shapes are shaded ellipses lit from the
upper left with the SWINE-01 light, then fixed facial features, an outer outline and an orphan clean-up. No raster
image-model output, no tracing or sampling of concept pixels, no resampling.

Only key 0 (``idle``) is authored at the design stage; the execution tier adds frames 1-15 from spec sections 4, 7
and 8 and must not change key 0 (frozen in ``tests/fixtures/tusk_01_keys.json``).

    python3 scripts/author_tusk01.py               # write grid + .ase
    python3 scripts/author_tusk01.py --check       # exit 1 when the committed grid differs
    python3 scripts/author_tusk01.py key0-scale    # docs/evidence/tusk-01/key0-scale.png (1x) + key0-scale-4x.png
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_swine01 as swine  # noqa: E402
import pixel_grid_to_ase as pg  # noqa: E402

OUT_DIR = ROOT / "assets/source/characters/swine/tusklings"
GRID = OUT_DIR / "tusk_01.grid.json"
ASE = OUT_DIR / "tusk_01.ase"
EVIDENCE = ROOT / "docs/evidence/tusk-01"
CANVAS = (56, 40)
PIVOT = (22, 39)
TICK_HZ = 60
LAYERS = ["body", "costume"]

# The shared family palette `tuskling-floor01-v1`, imported unchanged from SWINE-01 (19 keys; slot 20 stays free).
PALETTE = list(swine.PALETTE)
ACCENTS = {p[0] for p in PALETTE if p[3]}
DARKER = swine.DARKER
LIGHT = swine.LIGHT

TAGS = [("idle", 0, 0)]          # design stage: key 0 only (spec section 4 lists frames 0-15)
TICKS = [18]
KEY_FRAMES = (0,)

OX, OY = 2, 4                    # design space -> canvas offset (design rows 6..34 + outline land on rows 10..39)

# Gown variant (spec section 6.1): costume-layer key remap plus accessory swap. Body layer unchanged.
GOWN_MAP = {"n": "r", "o": "g", "u": "m", "v": "l", "t": "s"}
BOW = {(24, 21), (28, 21), (24, 22), (25, 22), (26, 22), (27, 22), (28, 22), (24, 23), (28, 23)}
PEARLS = {(23, 21): "v", (24, 22): "v", (25, 23): "t", (26, 23): "v", (27, 23): "t", (28, 22): "v", (29, 21): "v"}
FLOWER = {(19, 8): "q", (20, 8): "q", (19, 9): "p", (20, 9): "q", (21, 9): "q", (20, 10): "p"}


class Painter:
    def __init__(self):
        self.px: dict[tuple[int, int], tuple[str, str]] = {}

    def put(self, x, y, key, layer="body"):
        self.px[(x, y)] = (key, layer)

    def get(self, x, y):
        return self.px.get((x, y), (None, None))[0]


def lit(nx: float, ny: float) -> float:
    nz = math.sqrt(max(0.0, 1.0 - nx * nx - ny * ny))
    return nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]


def flesh(d: float, dark: int = 0) -> str:
    k = "l" if d > 0.9 else "m" if d > 0.6 else "s" if d > 0.28 else "d"
    for _ in range(dark):
        k = DARKER[k]
    return k


def ell(p: Painter, cx, cy, rx, ry, dark=0, keyfn=None, layer="body", where=None) -> set:
    got = set()
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            nx, ny = (x - cx) / rx, (y - cy) / ry
            if nx * nx + ny * ny <= 1.0 and (where is None or where(x, y)):
                d = lit(nx, ny)
                p.put(x, y, keyfn(d) if keyfn else flesh(d, dark), layer)
                got.add((x, y))
    return got


def stamp(p: Painter, pixels: dict, layer="body"):
    for (x, y), k in pixels.items():
        p.put(x, y, k, layer)


def collar(x: int) -> float:
    return 15 + 0.35 * (x - 10)


def shirt_edge(y: int) -> float:
    return 26 - (y - 20) * 0.55


def key0_design() -> Painter:
    p = Painter()
    # 3 coat-tails hanging straight behind the rump (costume), split at the vent (gate fix 3: no flare)
    for y in range(24, 32):
        for x in range(8, 12):
            if not (y >= 29 and x == 10):
                p.put(x, y, "n", "costume")
    # 4 trunk: torso, shoulder hump, pot belly
    trunk = set()
    trunk |= ell(p, 18, 23, 10, 9)
    trunk |= ell(p, 16, 17, 6.5, 5.5)
    trunk |= ell(p, 23, 26, 7.5, 6.5)
    # 5 open tux jacket: back and side below the collar line, ragged hem; shirt V in front; bare belly below
    for (x, y) in sorted(trunk):
        hem = 30 if (x * 7 + 3) % 5 == 0 else 31
        in_shirt = 20 <= y <= (27 if x % 3 else 28) and x >= shirt_edge(y)
        if in_shirt:
            nx, ny = (x - 23) / 7.5, (y - 26) / 6.5
            p.put(x, y, "v" if lit(nx, ny) > 0.8 else "u", "costume")
        elif collar(x) <= y <= hem and x < shirt_edge(min(y, 27)):
            rim = y - 1 < collar(x)
            p.put(x, y, "o" if rim and x < 20 else "n", "costume")   # 1 px collar and back sheen
    for (x, y) in sorted(trunk):     # lapel sheen: jacket pixels touching the shirt
        if p.px[(x, y)] == ("n", "costume") and p.get(x + 1, y) in ("u", "v") and p.px[(x + 1, y)][1] == "costume":
            p.put(x, y, "o", "costume")
    stamp(p, {(26, 25): "t", (27, 25): "t", (27, 26): "t"}, "costume")   # soiled shirt front
    # coat hem hangs behind the legs down to row 31/32 (gate fix 2); floor clearance below it
    for x in range(9, 18):
        for y in range(24, 31 if (x * 5) % 3 == 0 else 32):
            if (x, y) not in p.px or p.px[(x, y)][1] == "body":
                p.put(x, y, "n", "costume")
    # 1-2 legs in front of the coat, together under the belly (gate fix 1): two short pillars, a K gap at x 21
    for y in (31, 32):
        for x in range(17, 21):
            p.put(x, y, "s" if x == 18 else "d")
        for x in range(22, 27):
            p.put(x, y, "m" if x == 23 else "s" if x < 25 else "d")
    for y in (33, 34):
        for x in range(16, 21):
            p.put(x, y, "n")
        for x in range(22, 28):
            p.put(x, y, "n")
    p.put(21, 31, "c"), p.put(21, 32, "c")   # crease between the two shins
    p.put(18, 33, "K"), p.put(18, 34, "K")
    p.put(22, 33, "o"), p.put(23, 33, "o")
    p.put(24, 33, "K"), p.put(24, 34, "K")
    # 6 head: domed skull, jowls, snout bridge, snout disc; a crevice where the skull meets the hump
    head = ell(p, 26, 14.5, 6.5, 6)
    ell(p, 27, 19, 5.5, 2.6, dark=1)
    ell(p, 31, 14.5, 3.2, 2.6)
    ell(p, 35.5, 14, 2.0, 2.7, keyfn=lambda d: "h" if d > 0.55 else "l" if d > 0.3 else "s")
    for (x, y) in sorted(head):
        if (x - 1, y) not in head and (x - 1, y) in trunk and 11 <= y <= 18:
            p.put(x, y, "c")
    # ears: far ear dark and low, near ear pricked and folded forward
    stamp(p, {(20, 10): "d", (21, 10): "d", (20, 9): "d", (21, 9): "d", (21, 8): "d", (22, 10): "d"})
    stamp(p, {(23, 10): "s", (24, 10): "s", (25, 10): "s", (24, 9): "l", (25, 9): "s", (26, 9): "s",
              (25, 8): "l", (26, 8): "l", (27, 8): "s", (27, 7): "s", (26, 10): "c", (25, 11): "d"})
    # 7 bow tie at the throat (costume)
    for (x, y) in BOW:
        p.put(x, y, "r", "costume")
    p.put(26, 22, "g", "costume")
    # 8 near arm (gate fix 4-5): sleeve with a K seam on its back edge, n middle, o sheen on its front edge;
    #   a 3 px ivory cuff; the trotter-hand hangs past the belly outline, ringed in K where it overlaps the body
    seam = [(19, 21), (19, 22), (19, 23), (20, 24), (21, 25), (22, 26), (23, 27)]
    front = [(20, 21), (21, 22), (21, 23), (22, 24), (23, 25), (24, 26), (24, 27)]
    mid = [(20, 22), (20, 23), (21, 24), (22, 25), (23, 26)]
    stamp(p, {q: "K" for q in seam})
    stamp(p, {q: "n" for q in mid}, "costume")
    stamp(p, {q: "o" for q in front}, "costume")
    stamp(p, {(25, 26): "v", (25, 27): "v", (26, 27): "v"}, "costume")
    hand = {(27, 27): "m", (28, 27): "l", (29, 27): "m", (30, 27): "s",
            (27, 28): "m", (28, 28): "m", (29, 28): "m", (30, 28): "s", (31, 28): "s",
            (27, 29): "s", (28, 29): "s", (29, 29): "s", (30, 29): "d", (31, 29): "d",
            (28, 30): "n", (29, 30): "n", (30, 30): "n"}
    stamp(p, hand)
    cuff = {(25, 26), (25, 27), (26, 27)}
    for (x, y) in sorted(hand):
        for q in ((x - 1, y), (x, y + 1), (x + 1, y), (x, y - 1)):
            if q in p.px and q not in hand and q not in cuff:
                p.put(*q, "K")
    # 9 face: scowl brow over a small sunk eye with a bag, far eye, cheek fold
    stamp(p, {(23, 11): "l", (24, 11): "l", (25, 12): "l", (26, 12): "l", (27, 12): "l",
              (23, 12): "c", (24, 12): "c", (25, 13): "c", (26, 13): "w", (27, 13): "K", (28, 13): "K",
              (25, 14): "d", (26, 14): "d", (27, 14): "d", (28, 14): "s",
              (30, 11): "l", (31, 11): "l", (30, 12): "K", (31, 12): "K",
              (22, 15): "s", (23, 16): "s", (23, 17): "s", (22, 16): "d"})
    # snarl: arched upper lip bares red gums and fangs, lower teeth, a lolling tongue; nostril notches
    stamp(p, {(29, 16): "s", (30, 16): "s", (31, 16): "s", (32, 16): "s", (33, 16): "s", (34, 16): "s",
              (30, 17): "r", (31, 17): "r", (32, 17): "r", (33, 17): "r",
              (27, 17): "c", (28, 17): "c", (29, 17): "c", (34, 17): "c",
              (27, 18): "c", (28, 18): "c", (31, 18): "c", (34, 18): "c",
              (29, 18): "v", (30, 18): "v", (32, 18): "v", (33, 18): "v",
              (29, 19): "u", (30, 19): "u", (31, 19): "c", (32, 19): "c",
              (33, 19): "r", (33, 20): "r", (34, 20): "r", (34, 21): "d",
              (37, 13): "K", (37, 15): "K"})
    # tusks: near tusk hooks up in front of the cheek, far tusk hooks up past the snout
    stamp(p, {(26, 19): "t", (27, 19): "u", (28, 19): "u", (26, 18): "u", (26, 17): "v", (26, 16): "v",
              (27, 15): "v", (27, 16): "u", (28, 15): "v"})
    stamp(p, {(35, 19): "t", (35, 18): "u", (36, 18): "u", (36, 17): "v", (37, 17): "u", (37, 16): "v",
              (38, 16): "u", (38, 15): "v", (39, 14): "v"})
    # 10 coarse bristle along the hump and crown
    stamp(p, {(11, 13): "a", (12, 12): "a", (12, 13): "b", (13, 11): "b", (14, 11): "a", (15, 11): "a",
              (15, 12): "b", (16, 11): "b", (17, 11): "a", (18, 11): "a", (18, 10): "a", (19, 10): "a",
              (22, 9): "a", (22, 8): "a", (23, 9): "a"})
    return p


def outline(p: Painter) -> None:
    filled = set(p.px)
    for (x, y) in sorted(filled):
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            if q not in filled and q not in p.px:
                p.put(*q, "K")


def to_layers(p: Painter) -> dict[str, list[str]]:
    w, h = CANVAS
    cels = {name: [["."] * w for _ in range(h)] for name in LAYERS}
    flat = [["."] * w for _ in range(h)]
    for (x, y), (k, layer) in p.px.items():
        X, Y = x + OX, y + OY
        if not (0 <= X < w and 0 <= Y < h):
            raise ValueError(f"pixel off canvas: {(x, y)} -> {(X, Y)}")
        flat[Y][X] = k
    flat = swine.despeckle(flat)       # same orphan rule as the Ball (accents g, w exempt)
    for (x, y), (_k, layer) in p.px.items():
        X, Y = x + OX, y + OY
        cels[layer][Y][X] = flat[Y][X]
    return {name: ["".join(r) for r in rows] for name, rows in cels.items()}


def key0() -> dict[str, list[str]]:
    p = key0_design()
    outline(p)
    return to_layers(p)


def render(frame: int, _cache: dict = {}) -> dict[str, list[str]]:   # noqa: B006 - deterministic memo
    if frame not in _cache:
        _cache[frame] = {0: key0}[frame]()
    return _cache[frame]


def variant(cels: dict[str, list[str]], name: str) -> dict[str, list[str]]:
    """Costume variant by rule: ``tux`` is the drawn costume; ``gown`` remaps the costume keys (black cloth to red
    sequin, sheen to glint, shirt to bare flesh) and swaps the bow tie for a pearl string and an ear flower."""
    if name == "tux":
        return {k: list(v) for k, v in cels.items()}
    assert name == "gown", name
    body = [list(r) for r in cels["body"]]
    cos = [list(r) for r in cels["costume"]]
    for y, row in enumerate(cos):
        for x, k in enumerate(row):
            if k != ".":
                cos[y][x] = GOWN_MAP.get(k, k)
                if k == "n" and (x + 2 * y) % 11 == 0 and x < 20:
                    cos[y][x] = "g"           # scattered sequin glints
    for (x, y) in BOW:
        cos[y + OY][x + OX] = "l"
    for (x, y), k in PEARLS.items():
        if body[y + OY][x + OX] != ".":
            body[y + OY][x + OX] = "."
        cos[y + OY][x + OX] = k
    for (x, y), k in FLOWER.items():
        body[y + OY][x + OX] = "."
        cos[y + OY][x + OX] = k
    flat = swine.despeckle([[c if c != "." else b for b, c in zip(br, cr)] for br, cr in zip(body, cos)])
    for y, row in enumerate(flat):      # same orphan clean-up as key 0, written back to the owning layer
        for x, k in enumerate(row):
            if cos[y][x] != ".":
                cos[y][x] = k
            elif body[y][x] != ".":
                body[y][x] = k
    return {"body": ["".join(r) for r in body], "costume": ["".join(r) for r in cos]}


def flatten(cels: dict[str, list[str]]) -> list[str]:
    out = [list(r) for r in cels["body"]]
    for y, row in enumerate(cels["costume"]):
        for x, k in enumerate(row):
            if k != ".":
                out[y][x] = k
    return ["".join(r) for r in out]


# --------------------------------------------------------------------------- grid output
def build_grid() -> dict:
    frames = [{"duration_ms": round(t * 1000 / TICK_HZ), "cels": render(i)} for i, t in enumerate(TICKS)]
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": swine.palette_json(), "layers": list(LAYERS), "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def report(rows: list[str], label: str) -> None:
    orphans, isolated = pg._stray_pixels([list(r) for r in rows], ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    text = "".join(rows)
    counts = " ".join(f"{k}{text.count(k)}" for k in "Kwhvutnrgq")
    print(f"{label}: x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} {counts}")


# --------------------------------------------------------------------------- evidence
def _image(rows: list[str]):
    from PIL import Image
    pal = {k: tuple(int(rgba[i:i + 2], 16) for i in (1, 3, 5, 7)) for k, _n, rgba, _a in PALETTE}
    im = Image.new("RGBA", CANVAS, (0, 0, 0, 0))
    for y, r in enumerate(rows):
        for x, k in enumerate(r):
            if k != ".":
                im.putpixel((x, y), pal[k])
    return im


def write_key0_scale(out: Path = EVIDENCE / "key0-scale.png") -> list[Path]:
    """Silhouette-gate evidence: key 0 (tux and gown) at 1x on lit Floor 1 next to Carl and the Ball of Swine
    idle key (640 x 360, Tusklings mirrored to face Carl, no scaling), plus the same scene at 4x."""
    from PIL import Image
    from pixelart.common import CARL_X, flip, grid_images, load_grid_file  # noqa: E402
    from pixelart.scene import Floor, reference_idle  # noqa: E402

    carl, carl_piv = reference_idle("carl")
    ball, b_piv = flip(grid_images(load_grid_file(swine.GRID))[0], tuple(swine.PIVOT))
    k0 = load_grid_file(GRID)["frames"][0]["cels"]
    tux, t_piv = flip(_image(flatten(variant(k0, "tux"))), PIVOT)
    gown, g_piv = flip(_image(flatten(variant(k0, "gown"))), PIVOT)
    entries = [(carl, carl_piv, CARL_X, 0), (tux, t_piv, CARL_X + 44, 0), (gown, g_piv, CARL_X + 76, 0),
               (ball, b_piv, CARL_X + 190, 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    scene = Floor().render(entries, 1)
    scene.save(out)
    four = out.with_name("key0-scale-4x.png")
    scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST).save(four)
    return [out, four]


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", nargs="?", choices=["key0-scale"])
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.command == "key0-scale":
        for p in write_key0_scale():
            print(f"wrote {p.relative_to(ROOT)}")
        return 0
    grid = build_grid()
    for f in range(len(TICKS)):
        cels = grid["frames"][f]["cels"]
        report(flatten(cels), f"frame {f:2d} tux ")
        report(flatten(variant(cels, "gown")), f"frame {f:2d} gown")
    text = swine.dump(grid)
    if a.check:
        ok = GRID.is_file() and GRID.read_text() == text
        print("up to date" if ok else "differs")
        return 0 if ok else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    pg.write_ase(grid, ASE)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
