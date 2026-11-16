#!/usr/bin/env python3
"""Authoring record for ANIM-DONUT-01: Princess Donut 40 px locomotion candidate.

Writes ``assets/source/characters/donut/donut_h40_locomotion.grid.json`` (pixel-grid-v1),
the editable candidate source for the tags ``idle`` (the approved master idle, unchanged),
``run``, ``jump_start``, ``jump_rise``, ``jump_fall`` and ``land``. ``scripts/export_assets.sh``
turns it into the ``.ase`` and the atlas through ``assets/manifests/donut_h40_locomotion.json``.

How the frames are made (no resampling, no rotation, no image model):

* **Copied from the approved master** ``donut_h40.grid.json`` (never modified here): head,
  face, tiara, chest ruff, collar, butterfly charm (rows 4-30, unchanged pixels, only
  translated) and the tail plume (rows 12-32). Face consistency is therefore exact.
* **Hand-placed parts** in this file: the side-view torso, a rounded chest bottom, seven
  1 px shadow-side tufts on the exposed tail edge, and the leg poses.
* **Per-frame pose data** (``RUN``, ``JUMP``): whole-pixel offsets, plus four pixel-safe
  edits on the torso: ``stretch`` (repeat or drop one column), ``arch`` (lift a column run,
  1 px ramps), ``tilt`` (per-column vertical offset) and a tail ``shear`` (per-row offset).
* **Cleanup** (``PATCHES``): per-pixel art fixes after the lint orphan report, each with
  its expected old value so a regenerated pose cannot silently move under a patch.

Frame layout: canvas 72x44, pivot (46, 43). The pivot is on the contact row like the idle,
and it sits 6 px behind the nose as in the idle (pivot 22, nose 28), so the face does not
jump when the state changes between idle and locomotion.

The committed grid is the source of truth. Re-running overwrites hand edits made to the
grid, so it only writes with ``--force``; ``--check`` reports whether they still match.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MASTER_PATH = ROOT / "assets/source/characters/donut/donut_h40.grid.json"
OUT_PATH = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"

LAYERS = ["tail", "legs_far", "body", "legs_near", "head", "face", "gear"]
BUILD_W, BUILD_H = 80, 44      # working canvas
CROP_X = 7                     # working-canvas column that becomes column 0
CANVAS = [72, 44]
PIVOT = [46, 43]
G = 43                         # contact row (pivot y)
GX = 6                         # horizontal origin of the rig in the working canvas


def part(text: str) -> list[str]:
    rows = text.strip("\n").split("\n")
    width = max(len(r) for r in rows)
    return [r.ljust(width, ".") for r in rows]


MASTER = json.loads(MASTER_PATH.read_text(encoding="utf-8"))


def master_cel(layer: str, frame: int, x0: int, y0: int, x1: int, y1: int) -> list[str]:
    rows = MASTER["frames"][frame]["cels"].get(layer, ["." * 46] * 44)
    return [r[x0:x1] for r in rows[y0:y1]]


class Canvas:
    def __init__(self) -> None:
        self.layers = {name: [["."] * BUILD_W for _ in range(BUILD_H)] for name in LAYERS}

    def put(self, layer: str, rows: list[str], x: int, y: int) -> None:
        grid = self.layers[layer]
        for dy, row in enumerate(rows):
            for dx, ch in enumerate(row):
                if ch == ".":
                    continue
                if not (0 <= x + dx < BUILD_W and 0 <= y + dy < BUILD_H):
                    raise ValueError(f"pixel off canvas: {layer} {x + dx},{y + dy}")
                grid[y + dy][x + dx] = ch

    def cels(self) -> dict[str, list[str]]:
        out = {}
        for name in LAYERS:
            rows = ["".join(r) for r in self.layers[name]]
            if any(c != "." for r in rows for c in r):
                for r in rows:
                    if any(c != "." for c in r[:CROP_X] + r[CROP_X + CANVAS[0]:]):
                        raise ValueError(f"{name}: pixels outside the {CANVAS[0]} px crop")
                out[name] = [r[CROP_X:CROP_X + CANVAS[0]] for r in rows]
        return out


# ---------------------------------------------------------------------------
# Pixel-safe edits
# ---------------------------------------------------------------------------

def shear(rows: list[str], k: float) -> tuple[list[str], int]:
    """Shift row y left by round((last - y) * k); returns rows and the base row's shift."""
    anchor = len(rows) - 1
    offs = [round((anchor - y) * k) for y in range(len(rows))]
    lo, hi = min(offs), max(offs)
    width = len(rows[0]) + (hi - lo)
    return [("." * (hi - offs[y]) + r).ljust(width, ".") for y, r in enumerate(rows)], hi


def stretch(rows: list[str], at: int, n: int) -> list[str]:
    """Repeat column ``at`` n times (n > 0) or drop -n columns from ``at`` (n < 0)."""
    if n >= 0:
        return [r[:at] + r[at] * n + r[at:] for r in rows]
    return [r[:at] + r[at - n:] for r in rows]


def arch(rows: list[str], x0: int, x1: int, dy: int) -> list[str]:
    """Lift columns x0..x1 by dy, with 1-column ramps of dy-1 on both sides."""
    h = len(rows)
    out = [list(r) for r in rows]
    lifts = {x: dy for x in range(x0, x1 + 1)}
    if dy > 1:
        lifts[x0 - 1] = lifts[x1 + 1] = dy - 1
    for x, d in lifts.items():
        col = [rows[y][x] for y in range(h)][d:] + ["."] * d
        for y in range(h):
            out[y][x] = col[y]
    return ["".join(r) for r in out]


def tilt(rows: list[str], t: float) -> list[str]:
    """Raise column x by round((x - centre) * t): t > 0 lifts the front (right)."""
    h, w = len(rows), len(rows[0])
    cx = (w - 1) / 2
    out = [["."] * w for _ in range(h)]
    for x in range(w):
        d = round((x - cx) * t)
        for y in range(h):
            if rows[y][x] != ".":
                out[y - d][x] = rows[y][x]
    return ["".join(r) for r in out]


# ---------------------------------------------------------------------------
# Hand-placed parts
# ---------------------------------------------------------------------------

# Side-view torso, light from the upper left: H rim on the back, j lit outline, K on the
# shadow side, D belly and a tufted fur skirt. Ginger saddle behind the shoulder and the
# ginger haunch blotch with its dark brindle bite (the master's near-side coat map).
TORSO = part("""
......jjjjjjjjjj........
....jjHHHHHHHHHHjjj.....
...jHHBBBBBBBBBHHHHj....
..jHBBBBBGGOOBBBBBBHj...
.jHBBBBGGOOOOooBBBBBBj..
.jHBBBGOOOOooBBBBBBBBBj.
jHBBBBOOOooBBBBBBBBBBBBj
jHBBBBBBBBBBBBBBBBBBBBBK
jBBGGOBBBBBBBBBBBBBBBBBK
jBGOOOOBBBBBBBBBBBBBBBBK
jGOOOOOOBBBBBBBBBBBBBBDK
joOOOOOOoBBBBBBBBBBBBDDK
KoOOOOOOoBBBBBBBBBBBDDDK
KBoOOBBOoBBBBBBBBBBDDDDK
.KBooBBBoBBBBBBBBBDDDDK.
.KDBBBBBBBBBDDDDDDDDDDK.
..KDDDDDDDDDDDDDDDDDDK..
..KKDDKKDDDKDDDKDDDKK...
....KK..KDK.KDK.KK......
""")

# Rounded, tufted bottom for the master chest ruff (the seated ruff runs to the paws).
CHEST_BOTTOM = part("""
ccCCCCCCCCCCCCcccK
.KcCCCCCCCCCCcccK.
..KccKcccKcccKK...
...KK.KKK.KKK.....
""")

# Master tail rows 12-32 with 1 px tufts on the shadow (right) edge, which the seated
# idle hides behind the body; exposed in profile it would read as one straight line.
TAIL_TUFTS = {14: "....CGGOODDK..", 15: "...CGOOOBDDK..", 18: ".GGOOBBBBDDK..",
              19: ".OOBBOOBBDDK..", 23: "OOOBBOOBBDDDK.", 24: ".GOOBBBBBDDDK.",
              28: "CCBBBOOBBBDDK."}

# Legs (near-side colours; the far side is derived by far()). FN = fore near, HN = hind
# near. Fore legs are cream socks; hind legs continue the ginger haunch blotch.
LEGS = {
    "FN_plant": part("""
.CCcK
KCCcK
KCCcK
KCCCcK
KccccK
"""),
    "FN_reach_low": part("""
CCcK...
KCCcK..
.KCCcK.
..KCCcK
..KccccK
"""),
    "FN_reach_high": part("""
CCcK.....
KCCCCcKK.
.KKCCCCcK
...KKcccK
"""),
    "FN_push": part("""
..CCcK
.CCcK.
KCCcK.
KCcK..
KccK..
"""),
    "FN_tuck": part("""
CCCcK
KCCCcK
.KcccK
"""),
    "FN_swing": part("""
CCcK.
KCCcK
.KCcK
.KccK
"""),
    "FN_crouch": part("""
CCCcK
KCCCcK
KccccK
"""),
    "FN_extend_down": part("""
CCcK...
KCCcK..
KCCcK..
.KCCcK.
.KCCcK.
..KCCcK
..KccccK
"""),
    "FN_fwd": part("""
CCCCCcK.
KCCCCccK
.KKcccK.
"""),
    "HN_plant": part("""
.GOOOoK
.GOOOoK
..GOOoK
..KOOoK
.KCCCcK
.KccccK
"""),
    "HN_push": part("""
....GOOoK
...GOOoK.
..GOOoK..
.GOOoK...
KCCcK....
KcccK....
"""),
    "HN_trail": part("""
....GOOoK
..GGOOoK.
.GOOooK..
KCCcKK...
KcccK....
"""),
    "HN_tuck": part("""
GOOOooK..
.GOOOOCCK
..KKcccCK
.....KKK.
"""),
    "HN_crouch": part("""
OOOOooK
KCCCCcK
KccccK.
"""),
}

FAR = str.maketrans({"C": "c", "B": "D", "j": "D", "H": "B", "G": "D", "O": "D", "o": "D"})


# ---------------------------------------------------------------------------
# Rig
# ---------------------------------------------------------------------------

def head_block(c: Canvas, dx: int, dy: int, frame: int = 0) -> None:
    """Master head, face, tiara, chest ruff, collar and charm, translated by (dx, dy).

    Master idle frames 1-2 are the inhale (rows above 38 lifted 1 px), so their rows are
    read 1 px higher to land on the frame-0 position."""
    lift = 1 if frame in (1, 2) else 0
    for layer in ("head", "face"):
        c.put(layer, master_cel(layer, frame, 11, 4 - lift, 45, 28 - lift), 11 + dx, 4 + dy)
    c.put("gear", master_cel("gear", frame, 11, 4 - lift, 45, 9 - lift), 11 + dx, 4 + dy)
    c.put("body", master_cel("body", frame, 21, 27 - lift, 38, 31 - lift), 21 + dx, 27 + dy)
    c.put("body", ["K"] * 4, 38 + dx, 27 + dy)
    c.put("body", CHEST_BOTTOM, 21 + dx, 31 + dy)
    c.put("gear", master_cel("gear", frame, 21, 27 - lift, 37, 31 - lift), 21 + dx, 27 + dy)


def tail_block(c: Canvas, x: int, y: int, k: float) -> None:
    rows = master_cel("tail", 0, 0, 12, 14, 33)
    for row, text in TAIL_TUFTS.items():
        rows[row - 12] = text
    sheared, moved = shear(rows, k)
    c.put("tail", sheared, x - moved, y)


def pose(spec: dict) -> dict[str, list[str]]:
    c = Canvas()
    by, hy, hx = spec.get("body_dy", 0), spec.get("head_dy", 0), spec.get("head_dx", 0)
    pad = 6
    blank = "." * len(TORSO[0])
    torso = [blank] * pad + TORSO + [blank] * pad
    if spec.get("stretch"):
        torso = stretch(torso, 10, spec["stretch"])
    if spec.get("arch"):
        torso = arch(torso, *spec["arch"])
    if spec.get("tilt"):
        torso = tilt(torso, spec["tilt"])
    tx = spec.get("torso_dx", 0)
    tail_block(c, GX + 15 + tx + spec.get("tail_dx", 0), 7 + by + spec.get("tail_dy", 0),
               spec.get("tail_k", 0.0))
    c.put("body", torso, GX + 23 + tx, 21 - pad + by)
    head_block(c, GX + 25 + hx, 4 + hy, frame=spec.get("head_frame", 0))
    for name, side, x, bottom in spec["legs"]:
        rows = LEGS[name] if side == "near" else [r.translate(FAR) for r in LEGS[name]]
        # "near" legs draw over the body; "far" legs draw behind it.
        layer = "legs_near" if side == "near" else "legs_far"
        c.put(layer, rows, GX + x, bottom - len(rows) + 1)
    return c.cels()


# Leg tuples: (part, side, x, bottom row) in rig coordinates.
RUN = [  # gallop: alternating contacts near fore, far fore, near hind, far hind; 2 airborne
    ("F0 near fore contact, hinds trailing", 80,
     {"tail_k": 0.2, "tail_dy": -1,
      "legs": [("FN_reach_low", "far", 56, G - 1), ("HN_push", "far", 25, G - 1),
               ("FN_plant", "near", 52, G), ("HN_trail", "near", 21, G - 3)]}),
    ("F1 far fore contact, near fore toe-off, hinds swing forward", 70,
     {"body_dy": 1, "head_dy": 1, "stretch": -2, "torso_dx": 1, "arch": (6, 15, 1), "tail_k": 0.1,
      "tail_dy": -1,
      "legs": [("FN_plant", "far", 54, G), ("HN_trail", "far", 23, G - 2),
               ("FN_push", "near", 48, G), ("HN_tuck", "near", 29, G - 2)]}),
    ("F2 gathered: back arched, near hind contact under the belly", 80,
     {"head_dy": 1, "stretch": -4, "torso_dx": 3, "arch": (4, 14, 2), "tail_k": 0.0, "tail_dy": 0,
      "legs": [("FN_push", "far", 49, G), ("HN_tuck", "far", 38, G - 2),
               ("FN_tuck", "near", 45, G - 2), ("HN_plant", "near", 35, G)]}),
    ("F3 far hind contact, near hind push-off, fores swing forward", 70,
     {"body_dy": -1, "stretch": -2, "torso_dx": 1, "arch": (6, 15, 1), "tail_k": 0.15, "tail_dy": 1,
      "legs": [("FN_tuck", "far", 50, G - 3), ("HN_plant", "far", 32, G),
               ("FN_swing", "near", 53, G - 2), ("HN_push", "near", 25, G)]}),
    ("F4 extended suspension: body long, fores reach, hinds trail", 90,
     {"body_dy": -2, "head_dy": -1, "head_dx": 1, "stretch": 3, "torso_dx": -3, "tail_k": 0.35,
      "tail_dy": 1,
      "legs": [("FN_reach_high", "far", 58, G - 4), ("HN_trail", "far", 21, G - 6),
               ("FN_reach_high", "near", 55, G - 3), ("HN_trail", "near", 18, G - 5)]}),
    ("F5 descending: fores reach down for the next contact", 70,
     {"body_dy": -1, "head_dy": -1, "stretch": 1, "torso_dx": -1, "tail_k": 0.3, "tail_dy": -1,
      "legs": [("FN_reach_low", "far", 58, G - 2), ("HN_trail", "far", 23, G - 5),
               ("FN_reach_low", "near", 54, G - 1), ("HN_trail", "near", 20, G - 4)]}),
]

JUMP = [
    ("jump_start", "J0 crouch: chest low, rear up, hinds bunched, tail lifts", 90,
     {"body_dy": 2, "head_dy": 3, "tilt": -0.1, "stretch": -3, "torso_dx": 2, "tail_k": -0.1,
      "tail_dy": -2,
      "legs": [("FN_crouch", "far", 54, G), ("HN_crouch", "far", 34, G),
               ("FN_crouch", "near", 50, G), ("HN_crouch", "near", 31, G)]}),
    ("jump_start", "J1 push-off: hinds extend, front rises, fores lift forward", 70,
     {"body_dy": -1, "head_dy": -3, "tilt": 0.17, "tail_k": 0.35, "tail_dy": 2,
      "legs": [("FN_fwd", "far", 58, G - 7), ("HN_push", "far", 27, G),
               ("FN_fwd", "near", 56, G - 6), ("HN_push", "near", 23, G)]}),
    ("jump_rise", "J2 rising: long tilted body, fores forward, hinds trailing, tail streaming", 120,
     {"body_dy": -3, "head_dy": -6, "head_dx": 1, "tilt": 0.25, "stretch": 2, "torso_dx": -2,
      "tail_k": 0.5, "tail_dy": 3,
      "legs": [("FN_fwd", "far", 59, G - 10), ("HN_trail", "far", 21, G - 1),
               ("FN_fwd", "near", 57, G - 9), ("HN_trail", "near", 18, G)]}),
    ("jump_fall", "J3 falling: front slightly down, fores reach for the floor, tail up", 120,
     {"body_dy": -2, "head_dy": -1, "tilt": -0.08, "tail_k": 0.0, "tail_dy": -1,
      "legs": [("FN_extend_down", "far", 54, G - 1), ("HN_tuck", "far", 35, G - 4),
               ("FN_extend_down", "near", 50, G), ("HN_tuck", "near", 31, G - 3)]}),
    ("land", "L0 impact: squashed, eyes squeezed shut (master blink), tail flicks", 80,
     {"body_dy": 3, "head_dy": 3, "stretch": -3, "torso_dx": 2, "tail_k": -0.1, "tail_dy": -1,
      "head_frame": 2,
      "legs": [("FN_crouch", "far", 55, G), ("HN_crouch", "far", 34, G),
               ("FN_crouch", "near", 51, G), ("HN_crouch", "near", 31, G)]}),
    ("land", "L1 recovery: chin up first while the body is still low", 110,
     {"body_dy": 1, "head_dy": -1, "stretch": -1, "torso_dx": 1, "tail_k": 0.1,
      "legs": [("FN_plant", "far", 54, G), ("HN_plant", "far", 33, G),
               ("FN_plant", "near", 51, G), ("HN_plant", "near", 30, G)]}),
]

# Cleanup pass after the lint orphan report: (frame, layer, x, y, expected old, new).
PATCHES = [
    (5, "legs_near", 34, 38, "K", "D"), (5, "body", 50, 38, "K", "c"),
    (6, "body", 28, 22, "j", "H"),
    (6, "body", 35, 23, "o", "O"), (6, "body", 35, 24, "B", "O"), (6, "body", 34, 25, "o", "O"),
    (8, "tail", 14, 7, "K", "."), (8, "body", 55, 36, "K", "c"),
    (9, "tail", 17, 6, "K", "."),
    (10, "body", 34, 27, "o", "O"), (10, "body", 33, 29, "o", "O"), (10, "body", 50, 40, "K", "c"),
    (11, "tail", 17, 9, "K", "."), (11, "body", 36, 38, "K", "."),
    (12, "tail", 12, 8, "K", "."), (12, "tail", 18, 17, "K", "."), (12, "tail", 22, 21, "K", "D"),
    (12, "tail", 22, 23, "K", "D"), (12, "body", 26, 25, "G", "O"), (12, "legs_far", 23, 42, "c", "K"),
    (13, "body", 33, 36, "K", "D"), (13, "body", 50, 36, "K", "c"),
    (14, "body", 33, 30, "o", "O"),
]


def build() -> dict:
    dx = PIVOT[0] - MASTER["pivot"][0]
    frames = [{"duration_ms": f["duration_ms"],
               "cels": {layer: ["." * dx + r + "." * (CANVAS[0] - dx - len(r)) for r in rows]
                        for layer, rows in f["cels"].items()}}
              for f in MASTER["frames"]]
    tags = [{"name": "idle", "from": 0, "to": len(frames) - 1, "direction": "forward"}]
    groups = [("run", [(d, s) for _, d, s in RUN])]
    for name in ("jump_start", "jump_rise", "jump_fall", "land"):
        groups.append((name, [(d, s) for tag, _, d, s in JUMP if tag == name]))
    for name, items in groups:
        start = len(frames)
        for duration, spec in items:
            frames.append({"duration_ms": duration, "cels": pose(spec)})
        tags.append({"name": name, "from": start, "to": len(frames) - 1, "direction": "forward"})
    for fi, layer, x, y, old, new in PATCHES:
        rows = frames[fi]["cels"][layer]
        if rows[y][x] != old:
            raise ValueError(f"patch {fi} {layer} ({x},{y}): expected {old!r}, found {rows[y][x]!r}")
        rows[y] = rows[y][:x] + new + rows[y][x + 1:]
    return {"format": "pixel-grid-v1", "canvas": CANVAS, "pivot": PIVOT, "palette": MASTER["palette"],
            "layers": LAYERS, "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    """Diff-friendly layout: one pixel row per line."""
    lines = ["{", f' "format": "{grid["format"]}",', f' "canvas": {json.dumps(grid["canvas"])},',
             f' "pivot": {json.dumps(grid["pivot"])},', ' "palette": [']
    pal = grid["palette"]
    lines += [f"  {json.dumps(e)}" + ("," if i < len(pal) - 1 else "") for i, e in enumerate(pal)]
    lines += [" ],", f' "layers": {json.dumps(grid["layers"])},', ' "frames": [']
    for fi, fr in enumerate(grid["frames"]):
        lines.append(f'  {{"duration_ms": {fr["duration_ms"]}, "cels": {{')
        names = [n for n in grid["layers"] if n in fr["cels"]]
        for li, name in enumerate(names):
            rows = fr["cels"][name]
            lines.append(f'   "{name}": [')
            lines += [f'    "{r}"' + ("," if ri < len(rows) - 1 else "") for ri, r in enumerate(rows)]
            lines.append("   ]" + ("," if li < len(names) - 1 else ""))
        lines.append("  }}" + ("," if fi < len(grid["frames"]) - 1 else ""))
    tags = grid["tags"]
    lines += [" ],", ' "tags": [']
    lines += [f"  {json.dumps(t)}" + ("," if i < len(tags) - 1 else "") for i, t in enumerate(tags)]
    lines += [" ]", "}"]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="exit 1 if the committed grid differs")
    mode.add_argument("--force", action="store_true", help="overwrite the committed grid")
    parser.add_argument("--out", type=Path, default=OUT_PATH)
    args = parser.parse_args(argv)
    text = dump(build())
    if args.check:
        same = args.out.is_file() and args.out.read_text(encoding="utf-8") == text
        print("grid matches the authoring record" if same else "grid differs from the authoring record")
        return 0 if same else 1
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
