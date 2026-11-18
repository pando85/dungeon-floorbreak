#!/usr/bin/env python3
"""Deterministic authoring script for SLIME-01 (Green Slime readability set, first draft).

Writes ``assets/source/characters/slime/slime_01.grid.json`` (pixel-grid-v1), the matching ``.ase`` via
``scripts/pixel_grid_to_ase.py`` and ``assets/manifests/slime-01.json``. Design spec: ``docs/02-art/slime-01-spec.md``.

Method (senior design edit): a body is a gel mass made of a few floor-anchored ellipses blended by a summed
inverse-square field (domed crest, sagging skirt), minus optional carve ellipses. It is shaded with five value steps
from the 4-neighbour distance to the outline (glowing rim all round, brightest on the torch-lit upper left, lit upper
skin, dark thick core, caustic band above the floor), then dark lenses, a three-quarter skull, a rusted plate jutting
from the skirt, grit, drip runs and glints are placed by rule. Integer inputs, deterministic IEEE arithmetic and
literal pixel rows; no concept pixels are traced or resampled. Six keys are drawn (frames 0, 3, 8, 10, 12, 15); in-between slots are copies of the nearest key until a
later pass (spec section 8).
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "source" / "characters" / "slime"
GRID = OUT_DIR / "slime_01.grid.json"
ASE = OUT_DIR / "slime_01.ase"
MANIFEST = ROOT / "assets" / "manifests" / "slime-01.json"

W, H = 80, 40
PIVOT = [40, 39]
BASE = 39  # contact row

# Palette `slime-floor01-v1`: 17 keys (spec section 6). Accent keys are exempt from the orphan lint.
PALETTE = [
    # key, name, rgba, accent
    ("K", "outline_murk", "#121a0bff", False),
    ("a", "gel_0_deep", "#222f14ff", False),
    ("b", "gel_1_shade", "#3a5219ff", False),
    ("c", "gel_2_mid", "#5d861cff", False),
    ("d", "gel_3_light", "#8eba24ff", False),
    ("f", "gel_4_lit", "#bddb37ff", False),
    ("h", "sheen_torch", "#dcef7cff", False),
    ("H", "glint_pale", "#f6f8c8ff", True),
    ("v", "acid_hot", "#f0e61eff", True),
    ("o", "bone_dark", "#6e6a3cff", False),
    ("p", "bone_mid", "#b0a868ff", False),
    ("q", "bone_lit", "#e0d898ff", False),
    ("r", "rust_dark", "#4a2818ff", False),
    ("s", "rust_mid", "#7c4526ff", False),
    ("u", "rust_lit", "#a96a3cff", False),
    ("n", "debris_grit", "#2a3418ff", False),
    ("w", "hurt_flash", "#f4f2dcff", True),
]
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}

SKULL = [  # 9 x 8, three-quarter skull: far socket 1 px, near socket 2 x 2, jaw set 1 px back, teeth broken by a gap
    "..ooooo..",
    ".opqqqqo.",
    "opqqqqqqo",
    "oqKqqKKqo",
    "opKqqKKpo",
    ".oqpKqqpo",
    ".oqKq.qo.",
    "..oooo...",
]
PLATE = [  # 16 x 7, a riveted pauldron wedged under the gel's skirt: lit top edge, 2 rivets, a corroded notch (left)
    "..........KKKK..",
    "......KKKKuuuuK.",
    "...KKKuuuussusuK",
    ".KKKuuussssurssK",
    "KrssusrssssssrrK",
    "KrrsrrssrrsrrsK.",
    ".KKKKKKKKKKKKK..",
]
PLATE_FLAT = [  # 22 x 4, the plate fallen out flat beside the remains: lit top edge, 2 rivets
    "..KKKKKKKKKKKKKKKKKK..",
    ".KuuuuuuuuuuuuuuuuuuK.",
    "KrssrssssssrsssssrrrK.",
    ".KKrrrKKrrrKKrrrrKKK..",
]
N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))
RAMP = "abcdf"  # gel value steps, murk to lit


class Cv:
    def __init__(self) -> None:
        self.p: dict[tuple[int, int], str] = {}

    def put(self, x: int, y: int, c: str) -> None:
        assert 0 <= x < W and 0 <= y < H, (x, y, c)
        self.p[(x, y)] = c

    def get(self, x: int, y: int) -> str | None:
        return self.p.get((x, y))


def blob(balls, carve=(), film_l=2, film_r=2):
    """Gel mass: summed inverse-square ellipse field (sharp blend, sagging skirt where balls meet the floor),
    clipped at the floor row, minus ``carve`` ellipses, plus a 1 px wet film either side on the floor row."""
    m = set()
    for y in range(H):
        for x in range(W):
            f = 0.0
            for cx, cy, rx, ry in balls:
                q = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
                f += 1e9 if q == 0 else 1.0 / (q * q)
            if f >= 1.0 and not any(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 < 1.0 for cx, cy, rx, ry in carve):
                m.add((x, y))
    row = sorted(x for x, y in m if y == BASE)
    for i in range(1, film_l + 1):
        m.add((row[0] - i, BASE))
    for i in range(1, film_r + 1):
        m.add((row[-1] + i, BASE))
    return m


def depth(mask):
    """4-neighbour distance to the outside: 1 on the outline, 2 on the rim, growing inward."""
    d = {p: 1 for p in mask if any((p[0] + dx, p[1] + dy) not in mask for dx, dy in N4)}
    front, k = sorted(d), 1
    while front:
        nxt = []
        for x, y in front:
            for dx, dy in N4:
                q = (x + dx, y + dy)
                if q in mask and q not in d:
                    d[q] = k + 1
                    nxt.append(q)
        front, k = sorted(nxt), k + 1
    return d


def tops(mask):
    t: dict[int, int] = {}
    for x, y in mask:
        t[x] = min(t.get(x, H), y)
    return t


def lit_side(mask, x, y):
    return any((x + dx, y + dy) not in mask for dx, dy in ((0, -2), (-2, 0), (-1, -2), (-2, -1), (-2, -2)))


def shade(cv: Cv, mask, dead=False, lift=False):
    """Translucent gel from five value steps: a glowing rim all round (brightest on the torch-lit upper left), a lit
    upper skin, a dark thick core, and light caught through the gel in a caustic band just above the floor."""
    d, top = depth(mask), tops(mask)
    for (x, y) in sorted(mask):
        k = d[(x, y)]
        if y == BASE and (x, y - 1) not in mask:
            cv.put(x, y, "a" if dead else "c")  # wet film on the floor
            continue
        if k == 1:
            cv.put(x, y, "K")
            continue
        lit = lit_side(mask, x, y)
        shadow = any((x + dx, y) not in mask for dx in (2, 3))
        dep = y - top[x]
        if k == 2:
            v = 4 if lit else 3  # the rim glows all round: light travelling through the gel skin
            if shadow and not lit:
                v = 2
        elif k == 3:
            v = 3 if lit else 2
        else:
            v = 3 if dep <= 2 else (2 if dep <= 6 else 1)  # lit upper skin, mid gel, body shade
            if k >= 6 and dep >= 9:
                v = 0  # thick core
        if lift and k >= 3 and y < BASE - 2 and v <= 1:
            v += 1  # alive keys: mostly light gel; darkness only in the lenses and the skull's shadow
        if y == BASE - 1 and k >= 2:
            v = max(v, 3)  # caustic band: torchlight through the gel at the floor
        elif y == BASE - 2 and k >= 3:
            v = max(v, 2)
        if dead:
            v = 2 if (k == 2 and lit) else min(v, 1) - (1 if k >= 4 else 0)
            v = max(0, v)
        cv.put(x, y, RAMP[v])


def pocket(cv: Cv, mask, cx, cy, rx, ry):
    """A dark lens of thick gel with light refracted along its lower-right edge (the concept's dark hollows)."""
    d = depth(mask)
    inside = {(x, y) for x in range(cx - rx, cx + rx + 1) for y in range(cy - ry, cy + ry + 1)
              if ((x - cx) * ry) ** 2 + ((y - cy) * rx) ** 2 <= (rx * ry) ** 2 and d.get((x, y), 0) >= 3}
    for x, y in inside:
        cv.put(x, y, "a")
    tx, ty = max(inside, key=lambda p: (p[0] + p[1], p[0]))
    for y in (ty - 1, ty, ty + 1):  # a short vertical refraction tick on the lower-right edge
        if (tx + 1, y) not in inside and d.get((tx + 1, y), 0) >= 3:
            cv.put(tx + 1, y, "d")


def bubbles(cv: Cv, mask, pts):
    """Trapped air: a 2 px light dot over mid gel."""
    d = depth(mask)
    for x, y in pts:
        if min(d.get(q, 0) for q in ((x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1))) >= 3:
            cv.put(x, y, "d")
            cv.put(x + 1, y, "d")
            cv.put(x, y + 1, "c")
            cv.put(x + 1, y + 1, "c")


def draw_obj(cv: Cv, rows, ox, oy, cover=0, mask=None, stain_from=99):
    """Literal rows at (ox, oy). The bottom ``cover`` rows stay gel (submerged); rows from ``stain_from`` on are one
    bone step darker (seen through thicker gel)."""
    darker = {"q": "p", "p": "o"}
    for j, line in enumerate(rows):
        for i, c in enumerate(line):
            x, y = ox + i, oy + j
            if c == "." or (mask is not None and (x, y) not in mask) or j > len(rows) - 1 - cover:
                continue
            cv.put(x, y, darker.get(c, c) if j >= stain_from else c)


def sunk_skull(cv: Cv, mask, ox, oy):
    """The skull inside the gel: top row and rear column overpainted with gel, lower half stained, a gel highlight
    crossing the cranium and a small shadow under and behind it."""
    draw_obj(cv, SKULL, ox, oy, mask=mask, stain_from=4)
    for j, line in enumerate(SKULL):
        cols = [i for i, c in enumerate(line) if c != "."]
        rear = [(ox + i, oy + j) for i in (cols[:1] if j else cols)]
        for q in rear:
            if q in mask:
                cv.put(*q, "c")
    cv.put(ox + 3, oy + 2, "d")
    cv.put(ox + 4, oy + 1, "d")
    for x in range(ox + 2, ox + 6):
        if (x, oy + 8) in mask and cv.get(x, oy + 8) != "K":
            cv.put(x, oy + 8, "a")
    for y in range(oy + 4, oy + 7):
        if (ox - 1, y) in mask and cv.get(ox - 1, y) != "K":
            cv.put(ox - 1, y, "b")


def lcg(seed):
    s = seed
    while True:
        s = (s * 1103515245 + 12345) & 0x7FFFFFFF
        yield s >> 8


def debris(cv: Cv, mask, seed, count, skip_boxes):
    """Suspended grit: 2-pixel specks in the body (grime, not sparkle)."""
    d = depth(mask)
    g = lcg(seed)
    pts = sorted(mask)
    placed = tries = 0
    while placed < count and tries < 600:
        tries += 1
        x, y = pts[next(g) % len(pts)]
        if any(bx0 <= x <= bx1 and by0 <= y <= by1 for bx0, by0, bx1, by1 in skip_boxes):
            continue
        if min(d.get((x, y), 0), d.get((x + 1, y), 0)) < 4:
            continue
        for q in ((x, y), (x + 1, y)):
            cv.put(*q, "n" if placed % 2 == 0 else "r")
        placed += 1


def streaks(cv: Cv, mask, xs, length):
    """Lighter gel runs hanging from the crest toward the floor (attached drips, drawn inside the body)."""
    top = tops(mask)
    for x in xs:
        y0 = top[x] + 3
        for y in range(y0, y0 + length):
            if (x, y) in mask and cv.get(x, y) in tuple("abc"):
                cv.put(x, y, "d")
        if cv.get(x, y0 + length) in tuple("abcd"):
            cv.put(x, y0 + length, "f")


def sheen(cv: Cv, mask, x0, x1):
    """Torch sheen on the top-facing rim of the lit side."""
    top = tops(mask)
    for x in range(x0, x1 + 1):
        if x in top and cv.get(x, top[x] + 1) in tuple("cdf"):
            cv.put(x, top[x] + 1, "h")


def glint(cv: Cv, mask, x, dy=2):
    y = tops(mask)[x] + dy
    for q in ((x, y), (x + 1, y), (x, y + 1)):
        cv.put(*q, "H")


def acid(cv: Cv, pts):
    for x, y in pts:
        cv.put(x, y, "v")


def acid_crest(cv: Cv, mask, xs):
    """Hot acid beading two pixels deep under the crest outline: the tell."""
    top = tops(mask)
    for x in xs:
        cv.put(x, top[x] + 1, "v")
        cv.put(x, top[x] + 2, "v")


def lip_drips(cv: Cv, mask, count=5):
    """Acid hanging under the tell's curled lip, over the dark carve: a 1 x 2 drip with a dark pixel behind it, and a
    wet glint on the lip's leading edge."""
    under = []
    for x in range(W):
        ys = [y for y in range(H) if (x, y) in mask and (x, y + 1) not in mask and y < BASE - 4]
        if ys:
            under.append((x, min(ys)))
    picks = under[-2 * count + 1::2][:count] if len(under) >= 2 * count - 1 else under[-count:]
    for x, y in picks:
        cv.put(x, y + 1, "v")
        cv.put(x, y + 2, "v")
        cv.put(x - 1, y + 1, "a")
    fx = max(x for x, _y in under)
    cv.put(fx - 1, tops(mask)[fx - 1] + 1, "H")


def finish(cv: Cv):
    rows = [["."] * W for _ in range(H)]
    for (x, y), c in cv.p.items():
        rows[y][x] = c
    return ["".join(r) for r in rows]


def despeckle(rows):
    """A non-accent pixel with no same-key neighbour takes the commonest neighbour key."""
    g = [list(r) for r in rows]
    for _ in range(3):
        changed = False
        for y in range(H):
            for x in range(W):
                c = g[y][x]
                if c == "." or c in ACCENTS:
                    continue
                around = [g[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < W and 0 <= y + dy < H and g[y + dy][x + dx] != "."]
                if around and c not in around:
                    counts = {k: around.count(k) for k in dict.fromkeys(around)}
                    g[y][x] = max(counts, key=lambda k: counts[k])
                    changed = True
        if not changed:
            break
    return ["".join(r) for r in g]


# ---------------------------------------------------------------------------
# The six keys. Facing right; the pivot column is 40, the floor is row 39.
# ---------------------------------------------------------------------------
IDLE_BALLS = [(37, 40, 24, 22), (32, 18, 10, 8), (48, 23, 9, 7), (15, 40, 7, 5), (60, 40, 7, 6)]


def key_idle():
    """Frame 0: a sagging, lumpy mound (domed crest, flared skirt), skull riding mid-body, plate jutting from the
    front skirt with acid eating its edge, dark lenses of thick gel, grit, drip runs, a torch sheen."""
    mask = blob(IDLE_BALLS)
    cv = Cv()
    shade(cv, mask, lift=True)
    pocket(cv, mask, 40, 19, 3, 2)
    pocket(cv, mask, 17, 30, 2, 2)
    pocket(cv, mask, 47, 28, 3, 2)
    pocket(cv, mask, 35, 31, 2, 1)
    debris(cv, mask, 11, 6, [(22, 19, 35, 30), (50, 31, 70, 39)])
    streaks(cv, mask, (19, 56), 3)
    bubbles(cv, mask, [(33, 16), (44, 21), (14, 33)])
    sunk_skull(cv, mask, 24, 21)
    draw_obj(cv, PLATE, 51, 33)
    sheen(cv, mask, 22, 38)
    glint(cv, mask, 29)
    glint(cv, mask, 45)
    acid(cv, [(51, 34), (52, 34), (54, 32), (55, 32), (57, 31)])  # gel eating the plate where it enters the body
    return finish(cv)


CREEP_BALLS = [(36, 40, 17, 18), (55, 37, 10, 11), (64, 40, 7, 6), (21, 40, 8, 5), (30, 24, 8, 6)]


def key_creep():
    """Frame 3: creep stretch. Lower and longer, the front lobe flowing forward, a thinning film trailing behind,
    skull and plate sheared toward the front."""
    mask = blob(CREEP_BALLS, film_l=6, film_r=2)
    cv = Cv()
    shade(cv, mask, lift=True)
    pocket(cv, mask, 44, 27, 3, 2)
    pocket(cv, mask, 24, 33, 2, 1)
    debris(cv, mask, 23, 5, [(28, 22, 40, 33), (54, 31, 72, 39)])
    streaks(cv, mask, (20, 60), 3)
    bubbles(cv, mask, [(42, 22), (52, 30)])
    sunk_skull(cv, mask, 30, 25)
    draw_obj(cv, PLATE, 55, 33)
    sheen(cv, mask, 25, 44)
    glint(cv, mask, 32)
    glint(cv, mask, 58)
    acid(cv, [(55, 34), (56, 34), (58, 32), (59, 32), (61, 31)])
    return finish(cv)


TELL_BALLS = [(32, 40, 16, 9), (29, 23, 9, 15), (35, 8, 8, 5), (22, 33, 6, 8), (43, 38, 6, 4)]


def key_tell():
    """Frame 8: the tell. The mass gathers off the floor and rears up and back into a crest that curls toward the
    target, base narrowed, front face drawn in under the lip, skull hauled up into the crest, acid beading along it."""
    mask = blob(TELL_BALLS, carve=[(44, 22, 6, 9)])
    cv = Cv()
    shade(cv, mask, lift=True)
    pocket(cv, mask, 30, 30, 2, 2)
    pocket(cv, mask, 33, 12, 2, 2)
    debris(cv, mask, 31, 4, [(22, 16, 35, 27), (36, 31, 56, 39)])
    streaks(cv, mask, (23,), 3)
    bubbles(cv, mask, [(31, 7), (26, 30)])
    sunk_skull(cv, mask, 24, 18)
    draw_obj(cv, PLATE, 37, 33)
    sheen(cv, mask, 20, 30)
    glint(cv, mask, 24)
    acid_crest(cv, mask, (29, 32))  # hot acid beads on the crest: the readable tell
    cv.put(35, tops(mask)[35] + 1, "v")
    lip_drips(cv, mask)
    acid(cv, [(37, 34), (38, 34), (40, 32), (41, 32)])
    return finish(cv)


LUNGE_BALLS = [(20, 40, 10, 9), (38, 40, 15, 15), (55, 40, 11, 10), (66, 34, 8, 9), (74, 39, 4, 5)]


def key_lunge():
    """Frame 10: contact lunge. The crest has thrown itself forward into a flat reach along the floor with a heavy
    bulb at the leading edge; skull and plate dragged behind, acid at the leading edge."""
    mask = blob(LUNGE_BALLS, film_l=4, film_r=0)
    cv = Cv()
    shade(cv, mask, lift=True)
    pocket(cv, mask, 62, 33, 2, 1)
    pocket(cv, mask, 44, 30, 3, 1)
    debris(cv, mask, 41, 4, [(22, 26, 34, 39), (30, 31, 50, 39)])
    streaks(cv, mask, (48,), 3)
    bubbles(cv, mask, [(52, 31), (40, 29)])
    sunk_skull(cv, mask, 24, 28)
    draw_obj(cv, PLATE, 33, 33, cover=1, mask=mask)
    sheen(cv, mask, 26, 72)
    glint(cv, mask, 30)
    glint(cv, mask, 68)
    acid(cv, [(36, 34), (37, 34), (71, 33), (72, 33), (75, 36), (76, 37)])
    return finish(cv)


HURT_BALLS = [(30, 40, 20, 18), (48, 40, 11, 10), (16, 40, 6, 5)]


def key_hurt():
    """Frame 12: flinch. Knocked flat and back, a dent punched into the front of the crest, the whole lit skin
    flashing, skull jolted to the rear."""
    mask = blob(HURT_BALLS, carve=[(43, 23, 5, 4)])
    cv = Cv()
    shade(cv, mask)
    d, top = depth(mask), tops(mask)
    for (x, y) in sorted(mask):
        if 1 < d[(x, y)] <= 3 and y - top[x] <= 3 and y < BASE - 3:
            cv.put(x, y, "w")  # flash: the struck skin, rim and top two rows
        elif d[(x, y)] >= 4 and cv.get(x, y) in tuple("abc"):
            cv.put(x, y, RAMP[RAMP.index(cv.get(x, y)) + 1])  # the body pales one step
    debris(cv, mask, 53, 4, [(14, 22, 28, 39), (44, 31, 64, 39)])
    draw_obj(cv, SKULL, 18, 26, mask=mask, stain_from=6)
    draw_obj(cv, PLATE, 45, 33)
    return finish(cv)


DEFEAT_BALLS = [(24, 42, 10, 11), (38, 42, 10, 12), (50, 42, 7, 10), (36, 43, 22, 6)]


def key_defeat():
    """Frame 15: the remains. The mass has sagged into a dull, deflated heap with a skinned-over crust (still a
    body, not a floor stain); the skull has rolled out of the front and the plate has slid out flat."""
    mask = blob(DEFEAT_BALLS, carve=[(31, 29, 2, 2), (45, 30, 2, 2)], film_l=3, film_r=0)  # dips between 3 lumps
    cv = Cv()
    draw_obj(cv, PLATE_FLAT, 52, 36)  # under the body: the slumped front lobe overhangs its back edge
    shade(cv, mask, dead=True)
    top = tops(mask)
    for fx, fy in ((21, 2), (35, 2), (47, 2)):  # diagonal wrinkle folds: a dark crease with a dull lit lip above
        for i in range(3):
            q = (fx + i, top[fx + i] + fy + i)
            if q in mask and cv.get(*q) not in ("K", None):
                cv.put(*q, "K")
                cv.put(q[0], q[1] - 1, "c")
    debris(cv, mask, 61, 3, [(52, 30, 79, 39)])
    draw_obj(cv, SKULL, 62, 32, stain_from=6)
    return finish(cv)


KEYS = {0: key_idle, 3: key_creep, 8: key_tell, 10: key_lunge, 12: key_hurt, 15: key_defeat}
# In-between slots are copies of the nearest key for now (spec section 8).
NEAREST = {0: 0, 1: 0, 2: 0, 3: 3, 4: 3, 5: 3, 6: 3, 7: 8, 8: 8, 9: 10, 10: 10, 11: 10, 12: 12, 13: 15, 14: 15, 15: 15}


def all_frames() -> list[list[str]]:
    keys = {k: despeckle(fn()) for k, fn in KEYS.items()}
    return [list(keys[NEAREST[i]]) for i in range(16)]


# Frame table (spec section 4): index -> (tag, ticks). Ticks are 60 Hz.
TAGS = [
    ("idle", 0, 2),
    ("creep", 3, 6),
    ("attack_tell", 7, 8),
    ("attack_lunge", 9, 11),
    ("hurt", 12, 12),
    ("down", 13, 15),
]
TICKS = [12, 8, 12, 8, 8, 8, 8, 6, 24, 2, 6, 20, 12, 6, 10, 0]
KEY_FRAMES = sorted(KEYS)


def build() -> dict:
    frames = []
    for ticks, cel in zip(TICKS, all_frames()):
        frames.append({"duration_ms": max(1, round((ticks or 6) * 1000 / 60)), "cels": {"body": cel}})
    return {
        "format": "pixel-grid-v1",
        "canvas": [W, H],
        "pivot": PIVOT,
        "palette": [{"key": k, "name": n, "rgba": c, **({"accent": True} if acc else {})} for k, n, c, acc in PALETTE],
        "layers": ["body"],
        "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


def bbox(rows):
    xs = [x for r in rows for x, c in enumerate(r) if c != "."]
    ys = [y for y, r in enumerate(rows) if any(c != "." for c in r)]
    return min(xs), min(ys), max(xs), max(ys)


BOXES = {  # frame pixels (x, y, w, h); rationale in spec section 3
    "standing": [12, 16, 51, 24],
    "hurtbox": [8, 10, 60, 30],
    "hurtbox_tell_hold": [15, 3, 38, 37],
    "attack_hitbox": [58, 25, 21, 15],
}


def write_manifest() -> None:
    rows = []
    for name, a, b in TAGS:
        for i in range(a, b + 1):
            ticks = TICKS[i] or None
            rows.append({"index": i, "tag": name, "ticks": ticks,
                         "duration_ms": max(1, round((ticks or 6) * 1000 / 60)),
                         "author": "key" if i in KEYS else "copy_of_key_%d" % NEAREST[i]})
    manifest = {
        "schema_version": 1,
        "id": "slime_floor01_v1_candidate",
        "type": "character_animation",
        "source_path": "assets/source/characters/slime/slime_01.ase",
        "grid_source": "assets/source/characters/slime/slime_01.grid.json",
        "output_atlas": "generated/art/slime-01.png",
        "output_metadata": "generated/art/slime-01.json",
        "palette_id": "slime-floor01-v1",
        "frame_size": [W, H],
        "pivot": PIVOT,
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation candidate (fan-concept layer: Green Slime, a Floor 1 corrosive hazard "
                 "creature; runtime role IP-neutral slow contact-hazard blob)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "SLIME-01 Green Slime readability set, first draft: six keys (frames 0, 3, 8, 10, 12, 15) are "
                      "drawn by scripts/author_slime01.py from blended floor-anchored ellipses, rule-based shading and "
                      "literal skull and plate rows; in-between slots are copies of the nearest key",
            "generator": "Draft spec and keys: Claude Sonnet. Design edit and freeze: Claude Opus. Review: pending. "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "slime": {
            "status": "edited and frozen keys; in-betweens are copies; not wired into any Bevy binary",
            "spec": "docs/02-art/slime-01-spec.md",
            "opaque_height_idle": 30,
            "tick_hz": 60,
            "hold_preview_ms": 100,
            "grounded_tags": [n for n, _a, _b in TAGS],
            "airborne_tags": [],
            "playback": {"idle": "loop", "creep": "loop", "attack_tell": "once", "attack_lunge": "once",
                         "hurt": "once", "down": "hold"},
            "key_frames": KEY_FRAMES,
            "key_fixture": "tests/fixtures/slime-01_keys.json",
            "boxes": BOXES,
            "attack_ticks": {"tell": [0, 29], "launch": [30, 31], "impact": [32, 37], "recover": [38, 57]},
            "frames": rows,
        },
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    GRID.write_text(json.dumps(build(), indent=1) + "\n", encoding="utf-8")
    write_manifest()
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(GRID), str(ASE)],
                   check=True)
    print(f"wrote {GRID.relative_to(ROOT)} and {ASE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
