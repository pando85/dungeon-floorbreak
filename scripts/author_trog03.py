#!/usr/bin/env python3
"""TROG-03 authoring record: the Troglodyte Virtuoso (variant of the TROG-01 Pygmy base), palette slots and hero key 0.

Usage:

    python3 scripts/author_trog03.py frames [--check]
    python3 scripts/author_trog03.py key0-scale

``frames`` rebuilds ``assets/source/characters/troglodyte/virtuoso/trog_03.grid.json`` (pixel-grid-v1, layer ``body``).
Design stage: only key 0 (``idle``) is drawn; the other keys and the in-betweens are added to ``POSES``, ``KEY_FRAMES``,
``INBETWEEN`` and ``TAGS`` by the execution stage from ``docs/02-art/trog-03-spec.md`` sections 4, 7, 8 and 14.

The family construction (part canvas, outline-by-part-order, rim light, despeckle, the 15-colour core palette, the
thick-line helper) is imported from ``scripts/author_trog01.py``; the palette helper and the necklace stamp follow
``scripts/author_trog02.py``. The Virtuoso is redrawn at 66 px from ``docs/02-art/trog-01-spec.md`` section 2b; nothing
is scaled up from Pygmy or Basher pixels. Every pose is a rule (thick polylines, discs and polygons in design units:
x right from the pivot column, y up from the contact row) plus literal face stamps. No raster image-model output, no
tracing of concept pixels. ``--check`` compares instead of writing. Presentation-only art tooling; nothing here is wired
into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import author_trog01 as t1  # noqa: E402  (shared family construction)

ROOT = t1.ROOT
OUT_DIR = ROOT / "assets/source/characters/troglodyte/virtuoso"
GRID = OUT_DIR / "trog_03.grid.json"
CANVAS = (128, 88)
PIVOT = (44, 87)
H = 66                                   # idle opaque height, hair included (family ladder, trog-01 spec 2b)

# trog-family-v0 core (15, frozen) + the Virtuoso's three slots: the ragged purple robe, deep, mid and torch-lit.
# Gold bells reuse t and T; the tongue reuses q with s and o for its shade. 18 of 20.
PALETTE = t1.PALETTE + [
    ("u", "robe_deep", "#2f1c3aff", False),
    ("v", "robe_mid", "#583768ff", False),
    ("V", "robe_light", "#8a5c8cff", False),
]
RAMPS = dict(t1.RAMPS)
RAMPS.update({
    "robe": ("u", "u", "v", "V"),        # shawl and skirt: grimy purple, dusty lit edge
    "bell": ("K", "T", "T", "t"),        # gold bell: dark-gold body, lit upper-left rim
    "cord": ("c", "c", "C", "C"),        # bell cord
    "tongue": ("o", "s", "q", "q"),      # attack frames only (execution stage)
})
ACCENTS = {k for k, _n, _c, acc in PALETTE if acc}
L = t1.L


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
    """The family part canvas on the Virtuoso frame and ramps."""

    def __init__(self):
        super().__init__(CANVAS[0], CANVAS[1], PIVOT[1])
        self.px = PIVOT[0]

    def add(self, name: str) -> int:
        self.ramps.append(RAMPS[name])
        return len(self.ramps) - 1


# ------------------------------------------------------------------ face stamps (Virtuoso, redrawn at 66 px)
# 13 columns; top-left at (hx - 5, hy + 6) where (hx, hy) is the jaw-circle centre. Brow 1 row over a bulging eye
# (3 white, 1 pupil, 3 red: bloodshot and staring); the mouth is narrower than the Basher's (hx - 2 .. hx + 6) and opens
# taller. The Virtuoso's own note: the tongue tip always lolls over the lower lip, with a drool strand (canon: a very
# long pink tongue). The tongue's length is only shown in the attack frames.
FACES = {
    "leer": [".............",
             "....KKKKKK...",
             "....ewwwwK...",
             "....ewwwKK...",
             "......ee.....",
             "..........s..",
             "...t.t.t.tt..",
             "..oooooooooo.",
             "..ooooooqqqo.",
             "...t.t..qqq..",
             "........qqq..",
             ".........qq..",
             ".........qq.w",
             "..........q.w",
             "............w"],
}
for _name, _rows in FACES.items():
    assert all(len(r) == 13 for r in _rows), _name

HEAD_R = (6.0, 5.6)        # skull 0.09 H and jaw (0.93 of it); constant in every pose
HEAD_OFF = 4.0             # skull centre this far behind the jaw centre: head width 4 + 6 + 5.6 = 15.6 px (0.236 H)


def head(hx, hy, tilt=-1.5):
    """Skull circle 4 px behind and ``-tilt`` px above the jaw circle centred on (hx, hy)."""
    return L([(hx - HEAD_OFF, hy - tilt), (hx, hy)], list(HEAD_R))


def jaw_b(hx, hy, drop=1.0):
    """Dropped lower jaw for the open-mouth faces (0..1): the Virtuoso's opens tall, not wide."""
    return L([(hx - 4, hy - 3.5), (hx + 3, hy - 3.5 - 4.0 * drop)], [3.0, 2.6 + 0.3 * drop])


# Hair: a matted crown mop about 0.06 H above the skull top (the long head, not a tall mop, carries the height) and
# seven rope dreads (radius 1.5) hanging down and back over the neck, the hump and the shawl; the longest reaches
# 0.6 H behind the jaw centre, to the middle of the back. A short front lock ends above the brow.
MOP = [(-10.5, 3), (-11, 6.5), (-9.5, 8.5), (-8.5, 10.5), (-7, 8.4), (-6, 11), (-4, 11.8), (-3, 9.0), (-1.5, 11.4),
       (0.5, 11.2), (2.5, 9.8),
       (1.5, 8), (-1.5, 7.2), (-5, 5.6), (-8, 3.5)]
ROPES = [[(-9, 8), (-14, 5), (-18, -2), (-21, -10), (-22, -18), (-22, -25)],   # longest: 0.6 H, to mid back
         [(-6, 10), (-12, 9), (-17, 5), (-21, -1), (-23, -5)],                  # upper rope streaming back
         [(-10, 4), (-12, -1), (-12, -6), (-11, -11)],                          # over the nape
         [(-3, 10.5), (-8, 11.2), (-12, 10.5)],
         [(-7, 4), (-8, 0), (-7.5, -4)],                                        # short, behind the ear
         [(1.5, 10), (4, 9), (5, 7.5)]]                                         # the short front lock
ROPE_R = 1.5
BEADS = {0: "T", 1: "C", 2: "T"}   # dull knots (the Basher gate: a light bead in a dark rope reads as a second eye)
EAR = [(-6, 4), (-16, 7.5), (-9, 1)]          # long, pointed, straight back below the crown, skin over the mop


def hair(hx, hy, sweep=0.0, sway=0.0):
    mop = [(hx + x, hy + y) for x, y in MOP]
    ropes = []
    for rope in ROPES:
        n = len(rope) - 1
        ropes.append([(hx + x - 4.0 * sweep * i / n - sway * i / n, hy + y + 3.0 * sweep * i / n + 0.4 * sway * i / n)
                      for i, (x, y) in enumerate(rope)])
    return {"mop": mop, "ropes": ropes, "ears": [[(hx + x, hy + y) for x, y in EAR]]}


def bell(bx, by, s=1.0):
    """A hand bell hanging from (bx, by) (top of the crown): crown, flared skirt, about 7 x 7 px at s = 1."""
    return [(bx - 1.2 * s, by), (bx + 1.2 * s, by), (bx + 2.2 * s, by - 2.0 * s), (bx + 2.6 * s, by - 4.6 * s),
            (bx + 3.6 * s, by - 6.4 * s), (bx - 3.6 * s, by - 6.4 * s), (bx - 2.6 * s, by - 4.6 * s),
            (bx - 2.2 * s, by - 2.0 * s)]


def skirt(x0, y0, drop=16.0, half=6.5, lean=0.0):
    """Ragged robe wrap hanging down from the hips (gravity): short at the front, long at the back, with three deep
    torn cuts so the hem reads as strips, never as a box. ``lean`` trails the hem (motion)."""
    return [(x0 + half, y0 + 1), (x0 + half + 0.5 + lean, y0 - drop * 0.45), (x0 + half * 0.55 + lean, y0 - drop * 0.25),
            (x0 + half * 0.3 + lean, y0 - drop * 0.7), (x0 + lean, y0 - drop * 0.4),
            (x0 - half * 0.3 + lean, y0 - drop * 0.95), (x0 - half * 0.55 + lean, y0 - drop * 0.55),
            (x0 - half * 0.85 + lean, y0 - drop * 1.05), (x0 - half - 0.8 + lean, y0 - drop * 0.7), (x0 - half, y0 + 1)]


def drooping_hand(x, y, droop=1.0):
    """Hanging hand: a small palm at the wrist and three long 1 px claws (0.12 H, 8 px) curling down and forward."""
    return [L([(x, y), (x + 1.2, y - 0.8)], [1.6, 1.4]),
            L([(x + 1.0, y - 0.6), (x + 3.6, y - 2.4 * droop), (x + 5.2, y - 6.4 * droop)], [0.55, 0.55, 0.55]),
            L([(x + 1.6, y - 0.2), (x + 5.0, y - 1.6 * droop), (x + 7.4, y - 4.8 * droop)], [0.55, 0.55, 0.55]),
            L([(x + 0.2, y - 1.0), (x + 1.4, y - 3.6 * droop), (x + 2.2, y - 7.0 * droop)], [0.55, 0.55, 0.55])]


def build(pose: dict) -> list[str]:
    w, h = CANVAS
    c = Canvas()
    far = c.add("far"); cord = c.add("cord"); bell_id = c.add("bell"); skin = c.add("skin"); shawl = c.add("robe")
    neck = c.add("skin"); robe = c.add("robe"); head_id = c.add("skin"); hair_id = c.add("hair"); arm = c.add("skin")
    ear = c.add("skin")
    hd = pose["hair"]
    # back to front: far limbs and far hand, bell cord, torso and hump, shawl, near leg, robe skirt, back bell,
    # neck, head and jaw, mop and ropes, near arm, near hand claws (each its own part), ear; then texture, creases,
    # grime, face and stamps
    for limb in pose.get("far_limbs", []):
        c.thick(limb["pts"], limb["r"], far)
    for fin in pose.get("far_fingers", []):
        c.thick(fin["pts"], fin["r"], far)
    for line in pose.get("cords", []):
        c.thick(line, [0.6] * len(line), cord)
    c.thick(pose["torso"]["pts"], pose["torso"]["r"], skin)
    for cx, cy, r in pose.get("masses", []):
        c.disc(cx, cy, r, r, skin)
    if "shawl" in pose:
        c.poly(pose["shawl"], shawl)
    for leg in pose.get("near_legs", []):
        c.thick(leg["pts"], leg["r"], skin)
    c.poly(pose["skirt"], robe)
    for poly in pose.get("bells", []):                    # the back bell hangs at the near side of the back hip
        c.poly(poly, bell_id)
    c.thick(pose["neck"]["pts"], pose["neck"]["r"], neck)
    c.thick(pose["head"]["pts"], pose["head"]["r"], head_id)
    if "jaw" in pose:
        c.thick(pose["jaw"]["pts"], pose["jaw"]["r"], head_id)
    c.poly(hd["mop"], hair_id)
    for rope in hd["ropes"]:
        c.thick(rope, [ROPE_R] * len(rope), hair_id)
    for limb in pose.get("near_arms", []):
        c.thick(limb["pts"], limb["r"], arm)
    for fin in pose.get("fingers", []):                   # palm and claws: each its own part, so K gaps separate them
        c.thick(fin["pts"], fin["r"], c.add("skin"))
    if "tongue" in pose:                                  # execution stage: the strike and retract frames
        c.thick(pose["tongue"]["pts"], pose["tongue"]["r"], c.add("tongue"))
    for e in hd["ears"]:
        c.poly(e, ear)
    rows = c.render()
    for y in range(h):                                   # rope texture (family rule)
        for x in range(w):
            if c.part[y][x] == hair_id and rows[y][x] == "I" and (x % 3 == 0 or (x % 3 == 1 and y % 4 == 0)):
                rows[y][x] = "H"
            if c.part[y][x] in (shawl, robe) and rows[y][x] == "v" and (x + 2 * y) % 7 == 0:
                rows[y][x] = "u"                         # robe weave: sparse deep flecks keep the cloth grimy
    for i, key in BEADS.items():
        bx, by = c.P(*hd["ropes"][i][-2])
        if c.part[int(by)][int(bx)] == hair_id:
            rows[int(by)][int(bx)] = key
    for y in range(h):                                   # mottled rim: no clean lit run longer than 5
        run = 0
        for x in range(w + 1):
            if x < w and rows[y][x] == "l":
                run += 1
                continue
            if run >= 6:
                rows[y][x - run + 3] = rows[y][x - run + 4] = "m"
            run = 0
    for (x0, y0), (x1, y1) in pose.get("creases", []):   # rib strokes and joint creases: 1 px d on skin
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for k in range(n + 1):
            px, py = c.P(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n)
            if rows[int(py)][int(px)] in "mls":
                rows[int(py)][int(px)] = "d"
    for gx, gy in pose.get("grime", []):                  # 2 x 2 dirt smudges (deep skin) on lit/mid/shade skin
        px, py = c.P(gx, gy)
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            if rows[int(py) + dy][int(px) + dx] in "mls":
                rows[int(py) + dy][int(px) + dx] = "d"
    for y in (PIVOT[1] - 2, PIVOT[1] - 1):               # dirt-caked soles
        rows[y] = ["d" if ch in "lms" else ch for ch in rows[y]]
    hx, hy = pose["face_at"]
    fx, fy = c.P(hx - 5, hy + 6)
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


def small_bell(x, y):
    """A 2 x 3 px gold bell stamp hanging from (x, y): lit top, dark-gold skirt, K mouth."""
    return [(x, y, "t"), (x - 1, y - 1, "t"), (x, y - 1, "T"), (x - 1, y - 2, "T"), (x, y - 2, "T")]


POSES: dict[str, dict] = {}

# KEY 0 idle: a lean, tall, less hunched stalker. Spine about 55 degrees from the hips (-9, 26) to the shoulder (2, 41);
# the head held 7.5 px above the hump on a visible neck, so the mouth has a clear line ahead; long arms hanging forward
# with the claws 0.14 H above the floor; a ragged purple shawl over the hump and a torn purple skirt; a big gold bell
# hanging on a cord off the back; two small bells on the skirt and one on the chest cord; the tongue tip lolling.
POSES["idle0"] = {
    "far_limbs": [L([(-10, 23), (-7, 13), (-14, 4.5), (-14, 2.2), (-10, 1.4), (-6, 1.3)], [2.6, 2.1, 1.5, 1.3, 1.3, 1.2]),
                  L([(0, 38), (1, 20), (18, 16)], [1.9, 1.6, 1.3])],               # far leg; far arm
    "far_fingers": drooping_hand(18, 16, 0.8),
    "cords": [[(-10, 33), (-14, 26), (-17, 20)]],
    "bells": [bell(-17, 20, 1.0)],
    "torso": L([(-8, 24), (-3, 31.5), (2, 38)], [4.4, 4.8, 4.6]),
    "masses": [(0, 39, 5.0)],                                                          # shoulder hump
    "shawl": [(-6, 44), (-1, 44.5), (4, 42), (6, 37), (4, 33.5), (1, 34.5), (-2, 32), (-5, 30.5), (-7, 27),
              (-9.5, 29), (-11, 26), (-12.5, 30), (-11.5, 35), (-9, 40)],
    "near_legs": [L([(-7, 23), (2, 13.5), (-3, 4.5), (-3, 2.2), (1, 1.4), (6, 1.3)], [2.8, 2.2, 1.6, 1.3, 1.3, 1.2])],
    "skirt": skirt(-7, 23, 16.0, 6.5),
    "neck": L([(3, 40), (6.5, 47)], [2.6, 2.3]),
    "head": head(8, 51.5), "hair": hair(8, 51.5),
    "near_arms": [L([(3, 37), (6, 19), (24, 15)], [2.2, 1.8, 1.4])],
    "fingers": drooping_hand(24, 15, 0.8),
    "creases": [((-3, 28), (-1, 30)), ((-2, 25), (0, 27)), ((4, 20), (7, 19)), ((1, 13), (3, 12))],
    "grime": [(10, 18), (18, 16), (0, 15), (-5, 7), (-2, 33), (1, 30), (-1, 4), (3, 35), (6, 44), (-9, 20)],
    "stamps": small_bell(-5, 11.5) + small_bell(-9, 7.5) + small_bell(5.5, 42),
    "face": "leer", "face_at": (8, 51.5),
}

KEY_FRAMES = {0: "idle0"}
INBETWEEN: dict[int, str] = {}
TAGS = [("idle", [(0, 12)])]
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


def write_key0_scale(out: Path = ROOT / "docs/evidence/trog-03/key0-scale.png") -> Path:
    """Key 0 at 1x on lit Floor 1 next to Carl, the Pygmy (TROG-01) and the Basher (TROG-02), 640 x 360 unscaled,
    plus a 4x copy of key 0 on the right."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    carl, carl_piv = reference_idle("carl")
    entries = [(carl, carl_piv, CARL_X, 0)]
    others = [(t1.GRID, t1.PIVOT, 64)]
    basher = ROOT / "assets/source/characters/troglodyte/basher/trog_02.grid.json"
    if basher.is_file():
        others.append((basher, (48, 87), 148))
    for path, piv, dx in others:
        img, p = flip(grid_images(load_grid_file(path))[0], tuple(piv))
        entries.append((img, p, CARL_X + dx, 0))
    virt = grid_images(load_grid_file(GRID))[0]
    v_img, v_piv = flip(virt, tuple(PIVOT))
    entries.append((v_img, v_piv, CARL_X + 236, 0))
    scene = Floor().render(entries, 1)
    panel_w = virt.width * 4
    sheet = Image.new("RGB", (scene.width + panel_w, max(scene.height, virt.height * 4)), (24, 23, 34))
    sheet.paste(scene, (0, 0))
    big = virt.resize((panel_w, virt.height * 4), Image.NEAREST)
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
