#!/usr/bin/env python3
"""TROG-02 authoring record: the Troglodyte Basher (variant of the TROG-01 Pygmy base), palette slots and hero key 0.

Usage:

    python3 scripts/author_trog02.py frames [--check]
    python3 scripts/author_trog02.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/basher/trog_02.grid.json`` (pixel-grid-v1, layer ``body``).
Design stage: only key 0 (``idle``) is drawn; the other keys and the in-betweens are added to ``POSES``, ``KEY_FRAMES``,
``INBETWEEN`` and ``TAGS`` by the execution stage from ``docs/02-art/trog-02-spec.md`` sections 4 and 7.

The family construction (part canvas, outline-by-part-order, rim light, despeckle, the 15-colour core palette) is
imported from ``scripts/author_trog01.py`` and redrawn at the Basher's 66 px from the rules in
``docs/02-art/trog-01-spec.md`` section 2b; nothing is scaled up from Pygmy pixels. Every pose is a rule (thick polylines,
discs and polygons in design units: x right from the pivot column, y up from the contact row) plus literal face
stamps. No raster image-model output, no tracing of concept pixels. ``--check`` compares instead of writing.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import author_trog01 as t1  # noqa: E402  (shared family construction)

ROOT = t1.ROOT
OUT_DIR = ROOT / "assets/source/characters/troglodyte/basher"
GRID = OUT_DIR / "trog_02.grid.json"
CANVAS = (112, 88)
PIVOT = (48, 87)
H = 66                                   # idle opaque height (family ladder, trog-01 spec 2b)

# trog-family-v0 core (15, frozen) + the Basher's two slots: forearm wrap mid and light. Necklace and shoulder trash
# reuse t, w, c, C. 17 of 20.
PALETTE = t1.PALETTE + [
    ("g", "wrap_mid", "#77706bff", False),
    ("G", "wrap_light", "#b3a993ff", False),
]
RAMPS = dict(t1.RAMPS)
RAMPS.update({
    "wrap": ("c", "c", "g", "G"),        # near forearm wraps: dirty linen over a rag-brown shade
    "farwrap": ("K", "c", "c", "g"),     # far forearm wraps in the far-limb light
    "trash": ("c", "c", "C", "C"),       # back junk heap (rubble, rags); can and bone are stamps
})
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}


def palette_json(keys: set[str] | None = None) -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        if keys is not None and key not in keys:
            continue
        entry = {"key": key, "name": name, "rgba": rgba}
        if accent:
            entry["accent"] = True
        out.append(entry)
    return out


class Canvas(t1.Canvas):
    """The family part canvas on the Basher frame and ramps."""

    def __init__(self):
        super().__init__(CANVAS[0], CANVAS[1], PIVOT[1])
        self.px = PIVOT[0]

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1


L = t1.L

# ------------------------------------------------------------------ face stamps (Basher, redrawn at 66 px)
# 14 columns; top-left at (hx - 5, hy + 5) where (hx, hy) is the jaw-circle centre. Brow 2 rows over the eye at
# hx - 1 .. hx + 4; eye 2 x 2 white + 1 pupil column + 3 red (bloodshot); mouth from the back of the cheek (hx - 3) to
# the front edge (hx + 6). The Basher's own note: an underbite with two lower tusks (cols 3 and 10) rising through the
# gash past the upper teeth, so the teeth band is 3 rows (upper row, gash, lower row) plus the tusk tips.
FACES = {
    "snarl": ["....KKKKKK....",
              ".....KKKKK....",
              "....eewwK.....",
              ".....ewwK.....",
              ".....e....s...",
              ".......t..t...",
              ".....tTtTtTt..",
              "..oooootooto..",
              "...ooootooto..",
              "....tTttTtt...",
              ".....w....w...",
              ".....w........"],
    # tell and smash: the jaw drops (jaw_b drop 1), 4-row gaping mouth that opens the front outline, tusks low
    "roar": ["....KKKKKK....",
             ".....KKKKK....",
             "....eewwK.....",
             ".....ewwK.....",
             "....TtTtTTtt..",
             "..oooooooooooo",
             "..oooooooooooo",
             "..oooooooooooo",
             "..ooooqqqqooo.",
             "..tTt.tT.tTt..",
             "...t......t...",
             "...w......w...",
             "...w.........."],
    # recover (after the smash): lid dropped over the eye, slack 1-row mouth short of the front, loose tusks, drool
    "pant": ["..............",
             "....KKKKKK....",
             "....eKKwK.....",
             ".....ewwK.....",
             "..........s...",
             ".......t..t...",
             ".....tT.tT....",
             "..ooooooooo...",
             "....t..t..t...",
             "....w..w......",
             "....w..w......",
             ".......w......"],
    # hurt: eye squeezed to a K slit, no red; open yelp, no tongue
    "pain": ["...KKKKKK.....",
             "....sKKKKK....",
             "..............",
             "..............",
             "..TtTtTtTt....",
             "..oooooooooo..",
             "..oooooooooo..",
             "..ooooooooo...",
             "..tTt.tT.tT...",
             "..............",
             "......w......."],
    # defeat: crossed eye, slack jaw, tongue out of the mouth corner (q >= 3)
    "dead": ["..............",
             ".....K.K......",
             "......K.......",
             ".....K.K......",
             "..............",
             "..............",
             "..ooooooooo...",
             "..tTt.tT.qq...",
             ".........qq...",
             "..........q..."],
}
for _name, _rows in FACES.items():
    assert all(len(r) == 14 for r in _rows), _name

HEAD_R = (6.5, 6.0)        # skull 0.10 H and jaw (0.93 of it); constant in every pose


def head(hx, hy, tilt=-1.6):
    """Skull circle 4.5 px behind and ``-tilt`` px above the jaw circle centred on (hx, hy)."""
    return L([(hx - 4.5, hy - tilt), (hx, hy)], list(HEAD_R))


def jaw_b(hx, hy, drop=1.0):
    """Dropped lower jaw for the roar and pain faces (0..1)."""
    return L([(hx - 6, hy - 3.5), (hx + 4, hy - 3.5 - 3.0 * drop)], [3.4, 2.8 + 0.3 * drop])


# Hair: a flatter mop than the Pygmy's (about 0.09 H above the skull top, so the small head stays small) and five rope
# dreads (radius 1.5) hanging down and back over the nape and the trash heap; longest rope 0.45 H behind the jaw.
MOP = [(-11, 4), (-11.5, 7.5), (-9.5, 9), (-8, 10.6), (-6, 10.4), (-4, 11.4), (-2, 10.6), (0, 11), (2, 9.6),
       (1, 8), (-2, 7), (-5.5, 5.5), (-8, 4)]
ROPES = [[(-9, 9), (-14, 8), (-19, 5), (-24, 0), (-26, -5)],
         [(-10, 6), (-14, 2), (-16, -3), (-16, -7)],
         [(-5, 10.5), (-10, 10.5), (-15, 9.5), (-20, 6)],
         [(-7, 7), (-10, 3), (-11, -2)],
         [(1, 9.6), (3.5, 8.6), (4.5, 7)]]          # the short front lock, ends above the brow
ROPE_R = 1.5
BEADS = {0: "t", 1: "w", 2: "t"}
EAR = [(-6, 4), (-14.5, 6), (-8.5, 1.5)]          # pointed, straight back, below the crown, skin over the mop


def hair(hx, hy, sweep=0.0, sway=0.0):
    mop = [(hx + x, hy + y) for x, y in MOP]
    ropes = []
    for rope in ROPES:
        n = len(rope) - 1
        ropes.append([(hx + x - 4.0 * sweep * i / n - sway * i / n, hy + y + 3.0 * sweep * i / n + 0.4 * sway * i / n)
                      for i, (x, y) in enumerate(rope)])
    return {"mop": mop, "ropes": ropes, "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def ragged_loin(x0, y0, drop=12.0, half=6.0, lean=0.0):
    """Wide ragged rag hanging down from just below the hip centre, four torn points."""
    return [(x0 - half, y0 - 0.5), (x0 + half, y0 - 0.5), (x0 + half - 0.4 + lean, y0 - drop * 0.7),
            (x0 + half * 0.6 + lean, y0 - drop * 0.95), (x0 + half * 0.25 + lean, y0 - drop * 0.6),
            (x0 - half * 0.1 + lean, y0 - drop), (x0 - half * 0.45 + lean, y0 - drop * 0.62),
            (x0 - half * 0.75 + lean, y0 - drop * 0.9), (x0 - half + lean, y0 - drop * 0.5)]


def planted_fist(x, y):
    """Knuckle-planted hand: three fingers (r 0.8) curling down from the wrist and splaying 7 to 8 px forward along the
    contact row (claws 0.11 to 0.13 H)."""
    return [[(x, y), (x + 3.0, y - 2.5), (x + 7.0, 1.3), (x + 9.5, 0.8)],
            [(x, y), (x + 3.8, y - 1.0), (x + 7.6, y - 3.4), (x + 10.4, y - 4.6)],
            [(x, y), (x + 2.6, y + 0.8), (x + 6.6, y - 0.6), (x + 9.0, y - 1.6)]]


def wrap_texture(rows, part, wrap_ids):
    """Slanted 1 px shade bands every third row across the wraps, so they read as wound strips, not a sleeve."""
    for y in range(len(rows)):
        for x in range(len(rows[0])):
            if part[y][x] in wrap_ids and rows[y][x] in "gG" and (y + x // 4) % 3 == 0:
                rows[y][x] = "c"


def build(pose: dict) -> list[str]:
    w, h = CANVAS
    c = Canvas()
    far = c.add("far"); far_wrap_ids = [c.add("farwrap") for _ in pose.get("far_wraps", [])]; skin = c.add("skin"); trash = c.add("trash")
    cloth = c.add("cloth"); head_id = c.add("skin"); hair_id = c.add("hair"); arm = c.add("skin")
    ear = c.add("skin")
    hd = pose["hair"]
    # back to front: far limbs (+ far wraps), torso and trapezius, trash heap, near leg, rag, head and jaw, mop and
    # ropes, near arm, near wrap, fingers, ear; then texture, grime, face and stamps
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for limb in pose.get("far_limbs_fore", []):
        c.thick(limb["pts"], limb["r"], far)
    for limb, wid in zip(pose.get("far_wraps", []), far_wrap_ids):
        c.thick(limb["pts"], limb["r"], wid)
    for fin in pose.get("far_fingers", []):
        c.thick(fin, [0.8] * len(fin), far)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for cx, cy, r in pose.get("masses", []):
        c.disc(cx, cy, r, r, skin)
    for poly in pose.get("trash", []):
        c.poly(poly, trash)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["loin"], cloth)
    c.thick(pose["head"]["pts"], pose["head"]["r"], head_id)
    if "jaw" in pose:
        c.thick(pose["jaw"]["pts"], pose["jaw"]["r"], head_id)
    c.poly(hd["mop"], hair_id)
    for rope in hd["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], arm)
    for limb in pose.get("near_arms_fore", []):
        c.thick(limb["pts"], limb["r"], c.add("skin"))
    for limb in pose.get("near_wraps", []):
        c.thick(limb["pts"], limb["r"], c.add("wrap"))
    for i, fin in enumerate(pose.get("fingers", [])):
        c.thick(fin, [0.8] * len(fin), c.add("skin") if i % 3 == 1 else arm)
    for e in hd["ears"]:
        c.poly(e, ear)
    rows = c.render()
    for y in range(h):                                   # rope texture (family rule)
        for x in range(w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
    for i, key in BEADS.items():
        bx, by = c.P(*hd["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    wrap_texture(rows, c.part, {i for i, r in enumerate(c.ramps) if r in (RAMPS["wrap"], RAMPS["farwrap"])})
    for y in range(h):                                   # mottled rim: no clean lit run longer than 5
        run = 0
        for x in range(w + 1):
            if x < w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    for (x0, y0), (x1, y1) in pose.get("creases", []):    # muscle separations: 1 px shade strokes on lit/mid skin
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for k in range(n + 1):
            px, py = c.P(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n)
            if rows[int(py)][int(px)] in "ml":
                rows[int(py)][int(px)] = "d" if rows[int(py)][int(px)] == "l" else "s"
    for gx, gy in pose.get("grime", []):
        px, py = c.P(gx, gy)
        for dx in (0, 1):
            if rows[int(py)][int(px) + dx] in "ml":
                rows[int(py)][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):               # dirt-caked soles and knuckles
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 5, hy + 5)
    for ry, line in enumerate(FACES[pose["face"]]):
        for rx, ch in enumerate(line):
            if ch != ".":
                x, y = int(fx) + rx, int(fy) + ry
                if rows[y][x] != "." or ch in "ewtTq":
                    rows[y][x] = ch
    for sx, sy, key in pose.get("stamps", []):
        px, py = c.P(sx, sy)
        rows[int(py)][int(px)] = key
    t1.despeckle(rows)
    return ["".join(r) for r in rows]


def necklace(pts):
    """Stamped necklace: a 1 px string of alternating t (tooth) and w (bone) beads; the pendant (w) is stamped below."""
    out = []
    for i, (x, y) in enumerate(pts):
        out.append((x, y, "t" if i % 2 == 0 else "w"))
    return out


POSES: dict[str, dict] = {}

# KEY 0 idle: a hunched brute. Spine about 35 degrees from the hips (-16, 31) to the trapezius peak (2, 55); the small
# head sunk 4.5 px below that peak and 9 px ahead of it; both fists knuckle-planted; wrapped forearms thicker than the
# upper arms; a junk heap on the upper back under the hanging ropes; a tooth-and-bone necklace across the chest.
POSES["idle0"] = {
    "far_limbs": [L([(-20, 29), (-23, 17), (-27, 6), (-24, 1.6), (-19, 1.4)], [5.4, 4.0, 2.8, 1.8, 1.4]),     # far leg
                  L([(2, 47), (5, 35), (8, 26)], [4.6, 4.0, 4.0])],                                          # far upper arm
    "far_limbs_fore": [L([(8, 26), (10, 16), (11.5, 9)], [4.4, 4.0, 2.8])],
    "far_fingers": [[(11.5, 8), (14, 5), (17, 1.3), (19, 0.9)], [(11.5, 8), (15, 6), (18.5, 3.8), (20.5, 3)]],
    "torso": L([(-18, 31), (-7, 40), (5, 47)], [8.0, 10.0, 9.5]),
    "masses": [(-6, 47, 8.5), (-1, 51.5, 9.0), (5, 50, 8.0)],                                                     # trapezius masses
    "trash": [[(-20, 47), (-18, 51.5), (-15, 52.5), (-13, 50), (-15, 46.5)],
              [(-14, 53), (-12, 57.5), (-9, 58.5), (-7.5, 55.5), (-10, 52)],
              [(-24, 42), (-23, 45.5), (-20, 46.5), (-19, 43)]],
    "near_legs": [L([(-13, 30), (-4, 20), (-7, 8), (-7, 2.5), (0, 1.5), (3, 1.4)], [6.4, 5.0, 3.4, 2.2, 1.5, 1.2])],
    "loin": ragged_loin(-13, 28, 13.0, 7.0),
    "head": head(14, 52), "hair": hair(14, 52),
    "near_arms": [L([(10, 47), (15, 36), (18, 28)], [6.4, 5.0, 4.6])],
    "near_arms_fore": [L([(18, 28), (20.5, 19), (22, 11)], [5.4, 4.8, 3.6])],
    "near_wraps": [L([(19.3, 24), (20.1, 21)], [5.4, 5.2]), L([(21, 16.5), (21.5, 14)], [4.6, 4.4]),
                   L([(21.9, 11.4), (22, 10.4)], [3.9, 3.8])],
    "fingers": planted_fist(22, 9),
    "creases": [((-9, 34), (-5, 31)), ((-2, 40), (2, 37)), ((7, 40), (10, 37)), ((14, 41), (16, 37))],
    "grime": [(-22, 36), (-11, 45), (-4, 31), (-7, 15), (2, 4), (-16, 38), (12, 38), (-24, 26), (3, 48), (-19, 12)],
    "stamps": necklace([(8, 42.5), (9, 41.5), (10, 40.5), (11, 39.5), (12, 39), (13, 39), (14, 39.5), (15, 40.5),
                        (16, 41.5)]) + [(12.5, 38, "w"), (12.5, 37, "w"), (-19, 47, "t"), (-18, 47, "t"), (-19, 46, "T"),
                                         (-18, 46, "T"), (-12, 55, "w"), (-11, 55.5, "w"), (-10, 56, "w")],
    "face": "snarl", "face_at": (14, 52),
}

KEY_FRAMES = {0: "idle0"}
INBETWEEN: dict[int, str] = {}
TAGS = [("idle", [(0, 12)])]           # design stage; the full table is spec section 4
TICK_HZ = 60
HOLD_MS = 100


def pose_for(n: int) -> dict:
    return POSES[KEY_FRAMES[n]] if n in KEY_FRAMES else POSES[INBETWEEN[n]]


def frame_ticks() -> dict[int, int | None]:
    return {n: t for _tag, fr in TAGS for n, t in fr}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    ticks = frame_ticks()
    frames = [{"rows": build(pose_for(n)), "ms": duration_ms(ticks[n])} for n in range(len(ticks))]
    used = {ch for f in frames for ch in "".join(f["rows"]) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(CANVAS),
        "pivot": list(PIVOT),
        "palette": palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": f["ms"], "cels": {"body": f["rows"]}} for f in frames],
        "tags": [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS],
    }


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-02/key0-scale.png") -> Path:
    """Key 0 at 1x on lit Floor 1 next to Carl and the Pygmy (TROG-01 key 0), 640 x 360 unscaled, plus a 4x copy."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    pygmy = grid_images(load_grid_file(t1.GRID))[0]
    basher = grid_images(load_grid_file(GRID))[0]
    p_img, p_piv = flip(pygmy, tuple(t1.PIVOT))
    b_img, b_piv = flip(basher, tuple(PIVOT))
    entries = [(carl, carl_piv, CARL_X, 0), (p_img, p_piv, CARL_X + 70, 0), (b_img, b_piv, CARL_X + 160, 0)]
    scene = Floor().render(entries, 1)
    panel_w = basher.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, basher.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = basher.resize((panel_w, basher.height * 4), Image.NEAREST)
    sheet.paste(big, (scene.width, 0), big)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames", "key0-scale"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "key0-scale":
        print(f"wrote {write_key0_scale().relative_to(ROOT)}")
        return 0
    text = dump(frames_grid())
    if args.check:
        same = GRID.is_file() and GRID.read_text(encoding="utf-8") == text
        print(f"{GRID.name} up to date" if same else f"{GRID.name} differs from the generator")
        return 0 if same else 1
    GRID.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    print(f"wrote {GRID.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
