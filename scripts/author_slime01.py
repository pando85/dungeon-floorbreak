#!/usr/bin/env python3
"""Deterministic authoring script for SLIME-01 (Green Slime readability set, first draft).

Writes ``assets/source/characters/slime/slime_01.grid.json`` (pixel-grid-v1), the matching ``.ase`` via
``scripts/pixel_grid_to_ase.py`` and ``assets/manifests/slime-01.json``. Design spec: ``docs/02-art/slime-01-spec.md``.

Method: a body is a lumpy mound described by integers (peak column, two half-widths, height, lump bumps). Its
mask is shaded with a few value steps from ray distances to the outline (lit rim top-left, dark core, dark right
edge, bright caustic band along the floor), then an inner skull and rusted plate, suspended debris, gel streaks and
glints are placed by rule. Everything is integer maths and literal pixel rows; no concept pixels are traced or
resampled. Six keys are drawn (frames 0, 3, 8, 10, 12, 15); in-between slots are copies of the nearest key until a
later pass (spec section 8).
"""
from __future__ import annotations

import json
import subprocess
import sys
from math import isqrt
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
    ("K", "outline_murk", "#0e1a0eff", False),
    ("a", "gel_0_deep", "#1b3317ff", False),
    ("b", "gel_1_shade", "#2b4e1fff", False),
    ("c", "gel_2_mid", "#44801fff", False),
    ("d", "gel_3_light", "#74ae2aff", False),
    ("f", "gel_4_lit", "#a6d03aff", False),
    ("h", "sheen_torch", "#d3e868ff", False),
    ("H", "glint_pale", "#f3f7bcff", True),
    ("v", "acid_hot", "#c8dc22ff", True),
    ("o", "bone_dark", "#6b6842ff", False),
    ("p", "bone_mid", "#aaa56eff", False),
    ("q", "bone_lit", "#dcd69eff", False),
    ("r", "rust_dark", "#4a2818ff", False),
    ("s", "rust_mid", "#7c4526ff", False),
    ("u", "rust_lit", "#a96a3cff", False),
    ("n", "debris_grit", "#2a3a1eff", False),
    ("w", "hurt_flash", "#f4f2dcff", True),
]
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}

SKULL = [  # 10 x 9, facing the viewer, bone seen through gel
    "..oooooo..",
    ".oppqqppo.",
    "oppqqqqppo",
    "opKKqqKKpo",
    "opKKqqKKpo",
    "oppqKKqppo",
    ".oppqqppo.",
    "..opqqpo..",
    "..opoqpo..",
]
PLATE = [  # 20 x 7, a rusted armour plate half-sunk in the body, rivets lit, one acid-eaten edge
    "......KKKKKKKK......",
    "....KKrssuuussrKK...",
    "..KKrssusssusssurKK.",
    ".KrssussrssussssusrK",
    "KrsussrrsssurssussrK",
    "KrrsrrsssrrsssrrsrrK",
    ".KKrrrKKKrrKKKrrKKK.",
]
PLATE_FLAT = [  # 22 x 4, the plate lying in the defeat puddle
    "..KKKKKKKKKKKKKKKKKK..",
    ".KrssussusssussussurK.",
    "KrsussrrsssurssussrrK.",
    ".KKrrrKKrrrKKrrrrKKK..",
]


class Cv:
    def __init__(self) -> None:
        self.p: dict[tuple[int, int], str] = {}

    def put(self, x: int, y: int, c: str) -> None:
        assert 0 <= x < W and 0 <= y < H, (x, y, c)
        self.p[(x, y)] = c

    def get(self, x: int, y: int) -> str | None:
        return self.p.get((x, y))


def heights(px, rxl, rxr, hh, lumps, film_l=3, film_r=3, gain=9, div=8):
    """column -> height. Ellipse with a flattened shoulder (gain/div), integer lump bumps and a thin floor film."""
    cols: dict[int, int] = {}
    base_h = hh - 3
    for x in range(px - rxl, px + rxr + 1):
        dx = x - px
        r = rxl if dx < 0 else rxr
        h = isqrt(base_h * base_h * (r * r - dx * dx) // (r * r)) * gain // div
        for bx, amp, wid in lumps:
            h += max(0, amp - abs(x - bx) * amp // wid) if amp > 0 else min(0, amp + abs(x - bx) * -amp // wid)
        cols[x] = max(1, min(hh, h))
    for i in range(1, film_l + 1):
        cols[px - rxl - i] = 1
    for i in range(1, film_r + 1):
        cols[px + rxr + i] = 1
    if film_l > 1:
        del cols[px - rxl - film_l]  # ragged film end, not a straight cut
    return cols


def mask_of(cols):
    return {(x, y) for x, h in cols.items() for y in range(BASE - h + 1, BASE + 1)}


def ray(mask, x, y, dx, dy):
    n = 0
    while (x + dx * (n + 1), y + dy * (n + 1)) in mask:
        n += 1
    return n


def shade(cv: Cv, mask, dead=False):
    """Value steps by ray distance to the outline: lit rim top-left, dark core, dark right edge, caustic floor band."""
    ramp = "abcdfh"
    for (x, y) in sorted(mask):
        du, dl = ray(mask, x, y, 0, -1), ray(mask, x, y, -1, 0)
        dd, dr = ray(mask, x, y, 0, 1), ray(mask, x, y, 1, 0)
        if min(du, dd, dl, dr) == 0:
            cv.put(x, y, "K")
            continue
        v = 1 if (du <= 14 or dd <= 6) else 0  # thick middle of the gel is deepest
        if du <= 1:
            v = 4 if dl <= 6 else 3
        elif du <= 3:
            v = 3 if dl <= 8 else 2
        elif du <= 7:
            v = 2
        if dl <= 1 and du > 1:
            v = max(v, 3)
        if dd <= 1:
            v = 2 if dr > 1 else 1  # caustic band, light through the gel at the floor
        elif dd <= 3:
            v = max(v, 1)
        if dr <= 1 and du > 3:
            v = min(v, 0)  # dark right edge
        if dead:
            v = max(0, v - 1)
        cv.put(x, y, ramp[v])


def top_of(cols, x):
    return BASE - cols[x] + 1


def draw_obj(cv: Cv, rows, ox, oy, cover=0, mask=None, skip=""):
    """Literal rows at (ox, oy); the bottom ``cover`` rows stay gel (submerged)."""
    for j, line in enumerate(rows):
        for i, c in enumerate(line):
            if c == "." or c in skip:
                continue
            x, y = ox + i, oy + j
            if mask is not None and (x, y) not in mask:
                continue
            if y > oy + len(rows) - 1 - cover:
                continue
            cv.put(x, y, c)


def lcg(seed):
    s = seed
    while True:
        s = (s * 1103515245 + 12345) & 0x7FFFFFFF
        yield s >> 8


def debris(cv: Cv, mask, seed, count, skip_boxes):
    """Suspended grit: 2-pixel specks deep inside the body (grime, not sparkle)."""
    g = lcg(seed)
    pts = sorted(mask)
    placed = 0
    tries = 0
    while placed < count and tries < 400:
        tries += 1
        x, y = pts[next(g) % len(pts)]
        if any(bx0 <= x <= bx1 and by0 <= y <= by1 for bx0, by0, bx1, by1 in skip_boxes):
            continue
        if min(ray(mask, x, y, 0, -1), ray(mask, x, y, 0, 1), ray(mask, x, y, -1, 0), ray(mask, x + 1, y, 1, 0)) < 4:
            continue
        if ray(mask, x, y, 0, 1) > 9:
            continue
        for (qx, qy) in ((x, y), (x + 1, y)):
            cv.put(qx, qy, "n" if placed % 2 == 0 else "r")
        placed += 1


def streaks(cv: Cv, mask, cols, xs, length):
    """Lighter gel runs hanging from the crest toward the floor (attached drips, drawn inside the body)."""
    for x in xs:
        if x not in cols:
            continue
        y0 = top_of(cols, x) + 4
        for j in range(length):
            y = y0 + j
            if (x, y) in mask and cv.get(x, y) in "abc":
                cv.put(x, y, "d")
        if (x, y0 + length) in mask and cv.get(x, y0 + length) in "abcd":
            cv.put(x, y0 + length, "f")


def glints(cv: Cv, cols, xs, top_dy=2):
    for x in xs:
        y = top_of(cols, x) + top_dy
        for dx, dy in ((0, 0), (1, 0), (0, 1)):
            cv.put(x + dx, y + dy, "H")


def sheen_arc(cv: Cv, cols, x0, x1, dy=1):
    for x in range(x0, x1 + 1):
        y = top_of(cols, x) + dy
        if cv.get(x, y) in tuple("bcdf"):
            cv.put(x, y, "h")


def acid(cv: Cv, pts):
    for x, y in pts:
        cv.put(x, y, "v")


def acid_crest(cv: Cv, cols, xs):
    """Hot acid beads two pixels deep just under the crest outline."""
    for x in xs:
        y = top_of(cols, x) + 1
        while cv.get(x, y) == "K":
            y += 1
        cv.put(x, y, "v")
        cv.put(x, y + 1, "v")


def bubble(cv: Cv, x, y):
    for dx, dy in ((0, 0), (1, 0), (-1, 1), (2, 1), (0, 2), (1, 2)):
        cv.put(x + dx, y + dy, "d")
    cv.put(x, y, "f")
    cv.put(x + 1, y, "f")


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
def key_idle():
    """Frame 0: a heavy lumpy mound, skull and plate half-dissolved inside, crest dips and streaks."""
    cols = heights(38, 25, 30, 30, [(28, 4, 5), (50, 3, 6), (36, -2, 3), (62, -1, 4)])
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask)
    skull_xy = (22, 27)
    plate_xy = (45, 32)
    draw_obj(cv, SKULL, *skull_xy, cover=1, mask=mask)
    draw_obj(cv, PLATE, *plate_xy, cover=1, mask=mask)
    debris(cv, mask, 11, 7, [(20, 24, 34, 39), (43, 30, 66, 39)])
    streaks(cv, mask, cols, (17, 31, 57, 63), 5)
    bubble(cv, 33, 17)
    bubble(cv, 52, 23)
    sheen_arc(cv, cols, 24, 46)
    glints(cv, cols, (27, 47), 2)
    acid(cv, [(48, 31), (49, 31), (58, 32), (62, 33), (45, 35)])  # acid eating the plate edge
    return finish(cv)


def key_creep():
    """Frame 3: creep stretch. Long, low, leading lobe forward, tail thinning behind, parts sheared forward."""
    cols = heights(34, 22, 34, 24, [(22, 3, 4), (56, 3, 5), (44, -2, 3)], film_l=7, film_r=3, gain=10)
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask)
    draw_obj(cv, SKULL, 27, 30, cover=1, mask=mask)
    draw_obj(cv, PLATE, 49, 32, cover=1, mask=mask)
    debris(cv, mask, 23, 6, [(25, 28, 38, 39), (47, 30, 70, 39)])
    streaks(cv, mask, cols, (14, 44, 62), 4)
    bubble(cv, 40, 19)
    sheen_arc(cv, cols, 26, 52)
    glints(cv, cols, (30, 58), 2)
    acid(cv, [(52, 32), (53, 32), (63, 33), (66, 34)])
    return finish(cv)


def key_tell():
    """Frame 8: the tell. Rears up and back, tallest pose (36 px), front face drawn in, acid beading on the crest."""
    cols = heights(31, 12, 24, 36, [(24, 5, 5), (38, 3, 5)], film_l=3, film_r=2, gain=9)
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask)
    draw_obj(cv, SKULL, 29, 20, cover=1, mask=mask)
    draw_obj(cv, PLATE, 35, 31, cover=1, mask=mask)
    debris(cv, mask, 31, 6, [(28, 18, 40, 31), (33, 30, 56, 39)])
    streaks(cv, mask, cols, (24, 41, 47), 7)
    bubble(cv, 30, 9)
    sheen_arc(cv, cols, 22, 40)
    glints(cv, cols, (28,), 2)
    acid_crest(cv, cols, (29, 32, 35, 38, 41))  # hot acid beads on the crest: the readable tell
    acid(cv, [(40, 28), (41, 28)])
    return finish(cv)


def key_lunge():
    """Frame 10: contact lunge. Flat reach at floor height, a bulb at the leading edge, parts dragged behind."""
    cols = heights(33, 20, 43, 17, [(26, 2, 4), (66, 3, 6), (74, -2, 3), (48, -1, 4)], film_l=4, film_r=2, gain=11)
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask)
    draw_obj(cv, SKULL, 20, 29, cover=1, mask=mask)
    draw_obj(cv, PLATE, 36, 33, cover=1, mask=mask)
    debris(cv, mask, 41, 5, [(18, 27, 32, 39), (34, 31, 58, 39)])
    streaks(cv, mask, cols, (46, 60), 3)
    sheen_arc(cv, cols, 24, 70)
    glints(cv, cols, (30, 64), 2)
    acid(cv, [(40, 34), (41, 34), (72, 34), (73, 34), (74, 35)])
    # the bulb: bright leading edge
    for x in range(70, 76):
        y = top_of(cols, x) + 2
        if cv.get(x, y) in tuple("bcd"):
            cv.put(x, y, "f")
    return finish(cv)


def key_hurt():
    """Frame 12: flinch. Squashed flat and slumped back (recoil), flash on the crest, skull knocked sideways."""
    cols = heights(34, 28, 27, 20, [(40, 2, 5), (52, -2, 3)], film_l=3, film_r=3, gain=10)
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask)
    draw_obj(cv, SKULL, 24, 30, cover=1, mask=mask)
    draw_obj(cv, PLATE, 42, 33, cover=1, mask=mask)
    debris(cv, mask, 53, 6, [(22, 28, 36, 39), (40, 31, 64, 39)])
    streaks(cv, mask, cols, (19, 55), 4)
    for x in range(26, 33):
        cv.put(x, top_of(cols, x) + 1, "w")
    for x in range(38, 43):
        cv.put(x, top_of(cols, x) + 1, "w")
    cv.put(44, top_of(cols, 44) + 2, "w")
    cv.put(30, top_of(cols, 30) + 3, "w")
    return finish(cv)


def key_defeat():
    """Frame 15: collapsed to a flat dull puddle, skull and plate left lying in it; no acid, no flash."""
    cols = heights(37, 28, 29, 9, [(26, 1, 4), (50, 1, 4), (36, -1, 3)], film_l=4, film_r=3, gain=10, div=8)
    mask = mask_of(cols)
    cv = Cv()
    shade(cv, mask, dead=True)
    draw_obj(cv, SKULL, 26, 28, cover=3)
    draw_obj(cv, PLATE_FLAT, 40, 35, cover=1)
    for x in range(22, 56):
        y = top_of(cols, x) + 1
        if (x, y) in mask and cv.get(x, y) in tuple("abc") and x % 5 == 0:
            cv.put(x, y, "d")
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
    "standing": [14, 16, 53, 24],
    "hurtbox": [12, 12, 57, 28],
    "hurtbox_tell_hold": [17, 4, 41, 36],
    "attack_hitbox": [56, 24, 22, 15],
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
                      "drawn by scripts/author_slime01.py from integer mound profiles, rule-based shading and literal "
                      "skull and plate rows; in-between slots are copies of the nearest key",
            "generator": "Draft spec and keys: Claude Sonnet. Edit, freeze and review: pending. "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "slime": {
            "status": "first draft keys; in-betweens are copies; not wired into any Bevy binary",
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
