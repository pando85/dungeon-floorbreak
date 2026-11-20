#!/usr/bin/env python3
"""ART-GOB-CF02 authoring record: the goblin CF02 acting and slammed sheets (pixel-grid-v1 sources), variants A and B.

Usage:

    python3 scripts/author_goblin_cf02.py all [--check]

Writes (or, with ``--check``, compares) four sheets, their ``.ase`` rebuilds, manifests, toolkit configs and the two
frozen-key fixtures:

* ``assets/source/characters/goblin/goblin_a_h36_cf02.*`` and ``goblin_b_h36_cf02.*``: 5 acting frames
  (``hesitate``, ``panic`` x2, ``taunt`` x2), 64 x 56, pivot (28, 55);
* ``assets/source/characters/goblin/death/goblin_a_slammed_h36.*`` and ``goblin_b_slammed_h36.*``: 6 slammed frames
  (stomped, wall, flung; impact and rest each).

Provenance (``docs/02-art/cf02-acting-spec.md`` sections 5, 6 and 9): the 9 frozen keys of
``assets/source/characters/goblin/cf02_keys`` are copied pixel for pixel; ``panic_flee_b`` and ``taunt_beckon`` are
derived here from those keys and GOB-01 walk frame 8; variant B is variant A plus the GOB-01 B accessory layer moved
with each head (hand-painted on ``wall_rest``). No concept pixels, Death Lab pixels or image-model output; nothing is
wired into the game.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import author_gob02_death as ad  # noqa: E402

KEYS_DIR = ROOT / "assets/source/characters/goblin/cf02_keys"
ACTING_KEYS = KEYS_DIR / "goblin_a_h36_cf02_keys_acting.grid.json"
SLAMMED_KEYS = KEYS_DIR / "goblin_a_h36_cf02_keys_slammed.grid.json"
GOBLIN_A = ad.GOBLIN_A
GOBLIN_B = ROOT / "assets/source/characters/goblin/goblin_b_h36.grid.json"
CANVAS = ad.CANVAS
Pix = dict


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def layer(grid: dict, frame: int, name: str) -> Pix:
    return ad.cel_pix(grid["frames"][frame]["cels"].get(name, ["." * 64] * 56))


def neighbours8(x, y):
    return [(x + a, y + b) for a in (-1, 0, 1) for b in (-1, 0, 1) if a or b]


def enclosed_pixels(flat: Pix) -> set:
    """Transparent pixels that no 4-connected path of transparent pixels joins to the canvas border (background that
    would show through as a hole)."""
    w, h = CANVAS
    seen: set = set()
    todo = [(x, y) for x in range(w) for y in range(h) if (x in (0, w - 1) or y in (0, h - 1)) and (x, y) not in flat]
    while todo:
        p = todo.pop()
        if p in seen or p in flat or not (0 <= p[0] < w and 0 <= p[1] < h):
            continue
        seen.add(p)
        todo += [(p[0] + 1, p[1]), (p[0] - 1, p[1]), (p[0], p[1] + 1), (p[0], p[1] - 1)]
    return {(x, y) for x in range(w) for y in range(h) if (x, y) not in flat and (x, y) not in seen}


def pinched_pixels(flat: Pix) -> set:
    """Transparent pixels with opaque pixels on both sides in a row or a column (one-pixel slits)."""
    w, h = CANVAS
    return {(x, y) for x in range(w) for y in range(h) if (x, y) not in flat
            and (((x - 1, y) in flat and (x + 1, y) in flat) or ((x, y - 1) in flat and (x, y + 1) in flat))}


def runs(rows: dict) -> Pix:
    """Literal rows ``{y: [(x0, "keys"), ...]}``."""
    return ad.segs(rows)


def put(pix: Pix, rows: dict) -> None:
    pix.update(runs(rows))


# --------------------------------------------------------------------------- derived frame: panic_flee_b
# The panic_flee key (acting key 1) with: the legs of GOB-01 walk frame 8 (the other stride; the first leg row is
# copied one row up to close the gap under the hem), the whole upper block and the trailing hand with the dragged club
# one pixel higher, and the raised empty arm two pixels lower. The arm is lowered by deleting two rows of its forearm
# and moving the hand and upper forearm down 2 and 1 px left, so the arm stays one connected limb; the shoulder end of
# the arm stays put while the torso rises.
ARM_X0 = {18: 44, 19: 44, 20: 44, 21: 44, 22: 44, 23: 44, 24: 44, 25: 46, 26: 45, 27: 44, 28: 44, 29: 43, 30: 42,
          31: 42, 32: 41, 33: 38, 34: 37, 35: 34, 36: 35}          # first column of the raised arm on each row
ARM_CUT = (29, 30)                                                  # forearm rows deleted
ARM_DROP = (-1, 2)                                                  # hand and upper forearm move by this


def is_arm(p) -> bool:
    return p[1] in ARM_X0 and p[0] >= ARM_X0[p[1]]


def panic_flee_b(keys: dict, walk: dict) -> dict:
    body, weapon = layer(keys, 1, "body"), layer(keys, 1, "weapon")
    legs = {p: c for p, c in layer(walk, 8, "body").items() if p[1] >= 48}
    out: Pix = {}
    for (x, y), c in body.items():
        if y > 47 or is_arm((x, y)) and y <= 36:
            continue
        out[(x, y - 1)] = c                                         # upper block, 1 px up
    for (x, y), c in body.items():
        if is_arm((x, y)):
            if y < ARM_CUT[0]:
                out[(x + ARM_DROP[0], y + ARM_DROP[1])] = c
            elif y > ARM_CUT[1]:
                out[(x, y)] = c
    out.update(legs)
    out.update({(x, 47): c for (x, y), c in legs.items() if y == 48})
    weapon_moved = {(x, y - 1): c for (x, y), c in weapon.items()}
    pockets = enclosed_pixels(out | weapon_moved)               # the lowered arm leaves pockets of background by the head
    out.update({p: "K" for p in pockets})
    return {"body": out, "weapon": {(x, y - 1): c for (x, y), c in weapon.items()}, "weapon_back": {}}


# --------------------------------------------------------------------------- derived frame: taunt_beckon
# The taunt_thump key with the head and neck one pixel forward and the near arm re-drawn: the upper arm hangs as before
# and the forearm goes forward at chest height with the palm up, a hand 6 px long and 4 px tall with all fingers
# together (a flat slab: no thumb or single-pixel spur above the knuckles).
BECKON_ARM = {
    34: [(42, "KKKK")],
    35: [(41, "K"), (42, "llll"), (46, "K")],
    36: [(33, "KKKKKKKK"), (41, "lllmmm"), (47, "K")],
    37: [(29, "lms"), (32, "l" * 9), (41, "mmmmms"), (47, "K")],
    38: [(29, "K"), (30, "lms"), (33, "m" * 8), (41, "ssssss"), (47, "K")],
    39: [(29, "K"), (30, "lmm"), (33, "s" * 8), (41, "KKKKKK")],
    40: [(29, "K" * 12)],
    41: [(30, "K" * 3)],
}


BECKON_SEAL = [(29, 41), (33, 41), (34, 41)]      # background holes under the re-drawn elbow and forearm


def taunt_beckon(keys: dict) -> dict:
    body = layer(keys, 2, "body")
    start = {}
    for y in range(30, 34):                                         # first neck column of each row (torso ends at 'tru' + K)
        start[y] = max(x for (x, yy), c in body.items() if yy == y and c in "tru" and x < 35) + 2
    out: Pix = {}
    for (x, y), c in body.items():
        if (37 <= y <= 41 and x >= 29) or (34 <= y <= 36 and x >= 34) or (y == 33 and x >= 34):
            continue                                                # the bent near arm
        head = y <= 29 or (y in start and x >= start[y])
        out[(x + 1 if head else x, y)] = c
    for y, x0 in start.items():                                     # the neck is one pixel wider at the back
        out[(x0, y)] = body[(x0, y)]
    out.update(runs(BECKON_ARM))
    out[(33, 35)] = "K"
    out.update({p: "K" for p in BECKON_SEAL})
    return {"body": out, "weapon": layer(keys, 2, "weapon"), "weapon_back": layer(keys, 2, "weapon_back")}


# --------------------------------------------------------------------------- goblin B accessory layer
def split_acc(acc: Pix) -> tuple[Pix, Pix]:
    """(head part, vest patch) of a GOB-01 B accessory cel; the patch is the 3 x 3 ``uuu / uKu / uKu`` stamp."""
    patch: Pix = {}
    for (x, y), ch in acc.items():
        if ch == "u" and acc.get((x + 1, y)) == "u" and acc.get((x + 2, y)) == "u":
            for dy, row in enumerate(("uuu", "uKu", "uKu")):
                for dx, c in enumerate(row):
                    patch[(x + dx, y + dy)] = c
    return {p: c for p, c in acc.items() if p not in patch}, patch


def moved(pix: Pix, mirror: bool, dx: int, dy: int) -> Pix:
    """``mirror`` is the A mirror around the pivot column (x' = 56 - x) before the shift."""
    return {((56 - x if mirror else x) + dx, y + dy): c for (x, y), c in pix.items()}


def b_acc(b: dict, frame: int) -> tuple[Pix, Pix]:
    return split_acc(layer(b, frame, "accessory"))


def chebyshev1(p, others: set) -> bool:
    return any(q in others for q in [p] + neighbours8(*p))


def tidy_acc(acc: Pix, body: Pix, gore: Pix | None = None) -> Pix:
    """Clip the accessory to the canvas, keep the band (R/B) one pixel away from any gore pixel (the headband shares
    its RGBA with blood), and outline every overhanging accessory pixel with ``K`` (as GOB-02B does)."""
    gore = gore or {}
    out: Pix = {}
    gore_px = set(gore)
    for (x, y), ch in acc.items():
        if not (0 <= x < CANVAS[0] and 0 <= y < CANVAS[1]):
            continue
        if ch in "RB" and chebyshev1((x, y), gore_px):
            continue
        out[(x, y)] = ch
    return out


# --------------------------------------------------------------------------- frames
ACTING_NAMES = ["hesitate", "panic_flee", "panic_flee_b", "taunt_thump", "taunt_beckon"]
SLAMMED_NAMES = ["stomped_impact", "stomped_rest", "wall_impact", "wall_rest", "flung_impact", "flung_rest"]
ACTING_LAYERS_A = ["weapon_back", "body", "weapon"]
ACTING_LAYERS_B = ["weapon_back", "body", "weapon", "accessory"]
SLAMMED_LAYERS_A = ["body", "weapon", "gore"]
SLAMMED_LAYERS_B = ["body", "weapon", "accessory", "gore"]


def rows_of(pix: Pix) -> list[str]:
    w, h = CANVAS
    grid = [["."] * w for _ in range(h)]
    for (x, y), ch in pix.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel outside the canvas: {(x, y)}")
        grid[y][x] = ch
    return ["".join(r) for r in grid]


def key_frame(keys: dict, i: int, layers: list[str]) -> dict:
    return {name: layer(keys, i, name) for name in layers}


def acting_frames_a() -> list[dict]:
    keys, walk = load(ACTING_KEYS), load(GOBLIN_A)
    return [key_frame(keys, 0, ACTING_LAYERS_A), key_frame(keys, 1, ACTING_LAYERS_A),
            panic_flee_b(keys, walk), key_frame(keys, 2, ACTING_LAYERS_A), taunt_beckon(keys)]


def slammed_frames_a() -> list[dict]:
    keys = load(SLAMMED_KEYS)
    return [key_frame(keys, i, SLAMMED_LAYERS_A) for i in range(6)]


# --------------------------------------------------------------------------- B accessory overlays (auto part)
def acting_acc_auto(b: dict) -> list[Pix]:
    """Auto-placed accessory of the five acting frames: the GOB-01 B cel moved with each head and with the vest."""
    h0, p0 = b_acc(b, 0)
    h18, p18 = b_acc(b, 18)
    hes = moved(h0, True, 4, -1) | moved(p0, False, -2, 0)             # the mirrored idle head, torso 2 px back
    panic = moved(h18, True, 2, 3) | moved(p18, True, 2, 3)            # the mirrored knockback frame 18
    panic_b = {(x, y - 1): c for (x, y), c in panic.items()}           # the whole upper block is 1 px higher
    taunt = moved(h0, False, -2, 0) | moved(p0, False, 0, 0)
    beckon = {(x + 1, y) if (y <= 33) else (x, y): c for (x, y), c in taunt.items()}   # head 1 px forward, vest not
    return [hes, panic, panic_b, taunt, beckon]


def slammed_acc_auto(b: dict) -> list[Pix]:
    h25, p25 = b_acc(b, 25)
    h18, p18 = b_acc(b, 18)
    h19, p19 = b_acc(b, 19)
    return [moved(h25, False, 2, -5) | moved(p25, False, 2, -5),       # stomped_impact
            moved(h25, False, 0, 0) | moved(p25, False, 0, 2),         # stomped_rest (skull = corpse 25 in place)
            moved(h18, False, 3, 6) | moved(p18, False, 3, 4),         # wall_impact
            {},                                                        # wall_rest: hand-painted below
            moved(h19, False, 0, 9) | moved(p19, False, 0, 9),         # flung_impact
            moved(h19, False, 0, 8)]                                   # flung_rest


# wall_rest: the dead head lolled on the near shoulder, so the GOB-01 B overlay does not apply: the band is painted
# across the forehead above the X eye (rows 35 and 36) as a plain 8 x 2 stripe (no knot tails: they would be orphans),
# the front tooth gap in the teeth row, the ear notch on the ear rim and the vest patch on the sitting vest.
WALL_REST_ACC = {
    35: [(23, "RRRRRRRR")],                                  # band, lit row
    36: [(23, "BBBBBBBB")],                                  # band, shadow row
    37: [(20, "K")],                                         # ear notch
    38: [(19, "K")],                                         # ear notch
    41: [(20, "uuu")],                                       # vest patch
    42: [(20, "uKu"), (29, "o")],                            # patch; the missing front tooth
    43: [(20, "uKu")],
}


def wall_rest_acc() -> Pix:
    return runs(WALL_REST_ACC)


def patch_ok(body: Pix, x: int, y: int) -> bool:
    return all(body.get((x + dx, y + dy)) in ("t", "r", "u") for dx in range(3) for dy in range(3))


def fit_patch(patch: Pix, body: Pix, radius: int = 6) -> Pix:
    """The 3 x 3 vest patch moved to the nearest spot (within ``radius``) that lies wholly on vest rag; dropped if none."""
    if not patch:
        return {}
    x0 = min(x for x, _ in patch)
    y0 = min(y for _, y in patch)
    best = None
    for dx in range(-radius, radius + 1):
        for dy in range(-radius, radius + 1):
            if patch_ok(body, x0 + dx, y0 + dy):
                d = max(abs(dx), abs(dy)) * 100 + abs(dx) + abs(dy)
                if best is None or d < best[0]:
                    best = (d, dx, dy)
    return {} if best is None else {(x + best[1], y + best[2]): c for (x, y), c in patch.items()}


def fit_tooth(head: Pix, body: Pix) -> Pix:
    """The missing front tooth ``o`` must lie on a tooth pixel ``w`` of a row that shows at least two teeth; it moves to
    an adjacent ``w`` of such a row, else it is dropped (a mouth with one tooth shows no gap)."""
    out = {p: c for p, c in head.items() if c != "o"}
    for (x, y), c in head.items():
        if c != "o":
            continue
        for q in [(x, y)] + neighbours8(x, y):
            if body.get(q) == "w" and sum(1 for (a, b), ch in body.items() if b == q[1] and ch == "w") >= 2:
                out[q] = "o"
                break
    return out


ACCENT_KEYS = set("HPew")           # accent palette keys (exempt from the orphan rule, as in the sources)
STACK = ["weapon_back", "body", "weapon", "accessory", "gore"]


def flat_of(frame: dict, acc: Pix) -> Pix:
    out: Pix = {}
    for name in STACK:
        out.update(acc if name == "accessory" else frame.get(name, {}))
    return out


def despeckle_acc(acc: Pix, frame: dict) -> Pix:
    """Drop accessory pixels that lint as orphans (no same-key pixel in the 8-neighbourhood) until none is left."""
    acc = dict(acc)
    while True:
        flat = flat_of(frame, acc)
        bad = [p for p in acc if flat.get(p) == acc[p] and acc[p] not in ACCENT_KEYS
               and not any(flat.get(q) == acc[p] for q in neighbours8(*p))
               and any(q in flat for q in neighbours8(*p))]
        if not bad:
            return acc
        for p in bad:
            del acc[p]


def heal_acc(acc: Pix, frame: dict) -> Pix:
    """A band or patch pixel can cover the only same-key neighbours of a body pixel and leave it an orphan; the
    accessory then repaints that pixel in the key most of its neighbours use (a body-coloured tidy pixel)."""
    acc = dict(acc)
    for _ in range(8):
        flat = flat_of(frame, acc)
        fixed = False
        for p, ch in sorted(flat.items()):
            around = [flat[q] for q in neighbours8(*p) if q in flat]
            if p in acc or ch in ACCENT_KEYS or not around or ch in around:
                continue
            counts = {k: around.count(k) for k in set(around)}
            acc[p] = max(sorted(counts), key=lambda k: counts[k])
            fixed = True
        if not fixed:
            break
    return acc


def seal_acc(acc: Pix, frame: dict) -> Pix:
    """The accessory must not open background holes or one-pixel slits that variant A does not have: every such new
    transparent pixel becomes an outline ``K`` on the accessory layer."""
    base, dressed = flat_of(frame, {}), flat_of(frame, acc)
    new = (enclosed_pixels(dressed) - enclosed_pixels(base)) | (pinched_pixels(dressed) - pinched_pixels(base))
    return acc | {p: "K" for p in new}


def finalize_acc(acc: Pix, frame: dict) -> Pix:
    head, patch = split_acc(acc)
    head = fit_tooth(head, frame["body"])
    patch = fit_patch(patch, frame["body"])
    return seal_acc(heal_acc(despeckle_acc(tidy_acc(head | patch, frame["body"], frame.get("gore")), frame), frame), frame)


def acting_acc_b() -> list[Pix]:
    b = load(GOBLIN_B)
    return [finalize_acc(a, f) for a, f in zip(acting_acc_auto(b), acting_frames_a())]


WALL_MIN_X = {2: 16, 3: 14}          # wall_impact contact column and wall_rest's leftmost (smear) column


def slammed_acc_b() -> list[Pix]:
    b = load(GOBLIN_B)
    auto = slammed_acc_auto(b)
    auto[3] = wall_rest_acc()
    for i, x0 in WALL_MIN_X.items():                 # nothing may stick out past the wall face
        auto[i] = {p: c for p, c in auto[i].items() if p[0] >= x0}
    out = []
    for i, (a, f) in enumerate(zip(auto, slammed_frames_a())):
        out.append(seal_acc(heal_acc(despeckle_acc(tidy_acc(a, f["body"], f["gore"]), f), f), f) if i == 3 else finalize_acc(a, f))
    return out


# --------------------------------------------------------------------------- sheets (grids, manifests, configs)
SPEC = "docs/02-art/cf02-acting-spec.md"
SOURCE_DIR = ROOT / "assets/source/characters/goblin"
DEATH_DIR = SOURCE_DIR / "death"
FIXTURES = ROOT / "tests/fixtures"
TICK_HZ = 60
HOLD_MS = 100


def ms(ticks) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


# (tag, playback, [(frame, ticks)]); ticks follow section 7 of the spec
ACTING_TAGS = [("hesitate", "once", [(0, 20)]),
               ("panic", "loop", [(1, 6), (2, 6)]),
               ("taunt", "loop", [(3, 9), (4, 9)])]
SLAMMED_TAGS = [("slammed_stomped", "hold", [(0, 8), (1, None)]),
                ("slammed_wall", "hold", [(2, 8), (3, None)]),
                ("slammed_flung", "hold", [(4, 8), (5, None)])]

SHEETS = {
    "acting_a": dict(stem="goblin_a_h36_cf02", dir=SOURCE_DIR, kind="acting", variant="A", layers=ACTING_LAYERS_A,
                     tags=ACTING_TAGS, names=ACTING_NAMES, manifest_id="goblin_a_h36_cf02_candidate_v1",
                     palette_id="goblin-floor01-v0", asset_id="gob-cf02-acting-a"),
    "acting_b": dict(stem="goblin_b_h36_cf02", dir=SOURCE_DIR, kind="acting", variant="B", layers=ACTING_LAYERS_B,
                     tags=ACTING_TAGS, names=ACTING_NAMES, manifest_id="goblin_b_h36_cf02_candidate_v1",
                     palette_id="goblin-floor01-v0", asset_id="gob-cf02-acting-b"),
    "slammed_a": dict(stem="goblin_a_slammed_h36", dir=DEATH_DIR, kind="slammed", variant="A", layers=SLAMMED_LAYERS_A,
                      tags=SLAMMED_TAGS, names=SLAMMED_NAMES, manifest_id="goblin_a_slammed_h36_candidate_v1",
                      palette_id="goblin-death-floor01-v0", asset_id="gob-cf02-slammed-a"),
    "slammed_b": dict(stem="goblin_b_slammed_h36", dir=DEATH_DIR, kind="slammed", variant="B", layers=SLAMMED_LAYERS_B,
                      tags=SLAMMED_TAGS, names=SLAMMED_NAMES, manifest_id="goblin_b_slammed_h36_candidate_v1",
                      palette_id="goblin-death-floor01-v0", asset_id="gob-cf02-slammed-b"),
}
for _s in SHEETS.values():
    _s["grid"] = _s["dir"] / (_s["stem"] + ".grid.json")
    _s["ase"] = _s["dir"] / (_s["stem"] + ".ase")
    _s["config"] = _s["dir"] / (_s["stem"] + ".pixelart.json")
    _s["manifest"] = ROOT / "assets/manifests" / (_s["stem"] + ".json")
KEY_FIXTURES = {"acting": FIXTURES / "goblin_cf02_acting_keys.json", "slammed": FIXTURES / "goblin_cf02_slammed_keys.json"}
KEY_FRAMES = {"acting": [0, 1, 3], "slammed": [0, 1, 2, 3, 4, 5]}          # frames copied pixel for pixel from cf02_keys
KEY_SOURCE_INDEX = {"acting": {0: 0, 1: 1, 3: 2}, "slammed": {i: i for i in range(6)}}


def palette_for(variant: str, kind: str, frames: list[dict]) -> list[dict]:
    """The key file's palette entries for the keys the sheet uses (B's headband ``B`` keeps its rag_red_dark name on the
    acting sheet and its blood_dark name on the slammed sheet, same RGBA)."""
    src = load(ACTING_KEYS if kind == "acting" else SLAMMED_KEYS)
    used = {c for f in frames for pix in f.values() for c in pix.values()}
    out = []
    for entry in src["palette"]:
        if entry["key"] in used:
            entry = dict(entry)
            if entry["key"] == "B" and kind == "acting":
                entry["name"] = "rag_red_dark"
            out.append(entry)
    return out


def sheet_frames(sheet: str) -> list[dict]:
    """Frames of a sheet as ``{layer: Pix}`` (accessory included for variant B)."""
    s = SHEETS[sheet]
    frames = acting_frames_a() if s["kind"] == "acting" else slammed_frames_a()
    if s["variant"] == "B":
        acc = acting_acc_b() if s["kind"] == "acting" else slammed_acc_b()
        frames = [dict(f, accessory=a) for f, a in zip(frames, acc)]
    return frames


def grid_for(sheet: str) -> dict:
    s = SHEETS[sheet]
    frames = sheet_frames(sheet)
    ticks = {f: t for _tag, _pb, rows in s["tags"] for f, t in rows}
    out_frames = []
    for i, f in enumerate(frames):
        out_frames.append({"duration_ms": ms(ticks[i]),
                           "cels": {name: rows_of(f.get(name, {})) for name in s["layers"]}})
    tags, start = [], 0
    for name, _pb, rows in s["tags"]:
        tags.append({"name": name, "from": rows[0][0], "to": rows[-1][0], "direction": "forward"})
    palette = palette_for(s["variant"], s["kind"], frames)
    if s["variant"] == "B" and s["kind"] == "acting":
        b_entry = next(p for p in load(GOBLIN_B)["palette"] if p["key"] == "B")
        palette = [b_entry if p["key"] == "B" else p for p in palette]
    return {"format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(ad.PIVOT), "palette": palette,
            "layers": list(s["layers"]), "frames": out_frames, "tags": tags}


# --------------------------------------------------------------------------- manifests
PROVENANCE_ORIGIN = {
    "acting": ("ART-GOB-CF02 goblin acting frames (docs/02-art/cf02-acting-spec.md section 5): hesitate, panic_flee and "
               "taunt_thump copied pixel for pixel from the frozen cf02_keys; panic_flee_b (the other stride) and "
               "taunt_beckon (the open-palm beckon) derived by scripts/author_goblin_cf02.py from those keys and GOB-01 "
               "walk frame 8"),
    "slammed": ("ART-GOB-CF02 goblin slammed finishes (docs/02-art/cf02-acting-spec.md section 5): the six frozen "
                "cf02_keys (stomped impact and rest, wall impact and rest, flung impact and rest) copied pixel for pixel; "
                "the gore layer is separate and a preset-4 reference only"),
}
B_NOTE = (" Variant B adds the GOB-01 B accessory layer (red headband, ear notch, missing front tooth, vest patch) moved "
          "with each head by scripts/author_goblin_cf02.py, hand-painted on wall_rest; the body, weapon and gore cels are "
          "variant A's, byte for byte.")
PROVENANCE_GEN = ("Design: Claude Opus (ACT-DESIGN, spec and keys). Execution: Claude Sonnet. Independent review: Claude Opus, "
                  "passed with fix-now items (fixes applied). No image-model output, no tracing")


def frame_rows(sheet: str) -> list[dict]:
    s = SHEETS[sheet]
    keys = set(KEY_FRAMES[s["kind"]])
    rows = []
    for tag, _pb, frames in s["tags"]:
        for f, t in frames:
            row = {"index": f, "name": s["names"][f], "tag": tag, "ticks": t, "duration_ms": ms(t),
                   "author": "key" if f in keys else "derived"}
            rows.append(row)
    return sorted(rows, key=lambda r: r["index"])


DERIVED_FROM = {"panic_flee_b": "panic_flee with the legs of GOB-01 walk frame 8, the upper block and club 1 px higher, "
                                "the raised arm 2 px lower",
                "taunt_beckon": "taunt_thump with the head and neck 1 px forward and the near arm re-drawn palm up"}

REACTIONS = {
    "hesitate": {"serves": "Q61 / Q72 hesitate", "tag": "hesitate", "frames": [0], "ticks": 20,
                 "note": "stand-still reaction; plays once for the Q61 stand-still length", "selector": "morale reaction"},
    "panic": {"serves": "Q61 / Q58 / Q72 panic", "tag": "panic", "frames": [1, 2], "ticks_per_frame": 6,
              "loop_ticks": "60-90 (Q58 panic length)", "facing": "the flee direction (the sheet's right is the flee direction; "
              "the presenter faces the sprite along the flee velocity, not toward the hero)", "selector": "morale reaction"},
    "taunt": {"serves": "Q65 / Q72 taunt", "tag": "taunt", "frames": [3, 4], "ticks_per_frame": 9, "loop_ticks": 36,
              "hit_mid_taunt": "a non-launching hit holds the shown frame 6 extra presentation ticks (Q72)",
              "selector": "acting overlay"},
}
FINISHES = {
    "stomped": {"serves": "Q21 stomped (Hit{Stomp} or Hit{DiveStomp}), Q43", "tag": "slammed_stomped", "impact_frame": 0,
                "rest_frame": 1, "impact_hold_ticks": 8, "after_taunt_hold": "the impact hold starts after any taunt hold",
                "anchor": "belly on row 55; the body stays where it died (lethal knock 0 / 0)",
                "scene_prop": "the stamping foot covers the belly in the scene (CF02-D)"},
    "wall": {"serves": "Q21 wall crumple (WallSlam)", "tag": "slammed_wall", "impact_frame": 2, "rest_frame": 3,
             "impact_hold_ticks": 8, "impact_anchor": "lowest-row anchored: the lowest opaque row is 55 although the body is "
             "airborne (the presenter lifts nothing)", "contact_column": 16,
             "rest_contact_column": 16,
             "rest_smear_columns": [14, 15],
             "presenter_contract": ("the presenter puts sprite column x16 of wall_impact, and column x16 of wall_rest "
                                    "(the slumped back), on the wall face; for facing left the column is mirrored "
                                    "(x' = 56 - x) and the wall is on the other side; no opaque pixel of wall_impact lies "
                                    "left of x16, and in wall_rest only gore-layer pixels (the smear at x14-15) do, drawn "
                                    "over the wall tile"),
             "club": "the club leaves the hand; the presenter keeps the flying club prop"},
    "flung": {"serves": "Q21 flung (lethal kick, Q06)", "tag": "slammed_flung", "impact_frame": 4, "rest_frame": 5,
              "impact_hold_ticks": 8, "anchor": "head on row 55 (impact), body on row 55 (rest)",
              "club": "the club lands separately"},
}
GORE_NOTE = ("the gore layer holds the blood pixels: a preset-4 reference only; preset 1 draws no blood sprites, so the "
             "presenter can hide it; gib quarters and separated parts (Q43) are particles from CF02-D, not frames here")


def manifest_for(sheet: str) -> dict:
    s = SHEETS[sheet]
    kind, variant = s["kind"], s["variant"]
    grid = load(s["grid"]) if s["grid"].is_file() else grid_for(sheet)
    block = {"spec": SPEC, "variant": variant, "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS, "facing": "right",
             "mirror": "x' = 56 - x around the pivot column",
             "status": "executor pass complete; independent review passed with fix-now items (fixes applied); not wired into any Bevy binary",
             "layers": list(s["layers"]),
             "key_frames": KEY_FRAMES[kind],
             "key_fixture": str(KEY_FIXTURES[kind].relative_to(ROOT)),
             "key_source": str((ACTING_KEYS if kind == "acting" else SLAMMED_KEYS).relative_to(ROOT)),
             "key_source_frames": {str(k): v for k, v in KEY_SOURCE_INDEX[kind].items()},
             "frames": frame_rows(sheet)}
    if kind == "acting":
        for row in block["frames"]:
            if row["name"] in DERIVED_FROM:
                row["derived_from"] = DERIVED_FROM[row["name"]]
        block["reactions"] = REACTIONS
        block["pose_budget"] = "5 acting frames (spec section 6: 11 per goblin sheet with the 6 slammed frames)"
    else:
        block["finishes"] = FINISHES
        block["gore_layer"] = GORE_NOTE
        block["pose_budget"] = "6 slammed frames (spec section 6: 11 per goblin sheet with the 5 acting frames)"
    if variant == "B":
        block["base_sheet"] = str(SHEETS[sheet.replace("_b", "_a")]["grid"].relative_to(ROOT))
        block["headband"] = ("accessory layer only; B shares its RGBA with the gore B (rag_red_dark = blood_dark); R "
                             "(red_mid #b8322e) is a different red from blood_wet; so no headband pixel touches or "
                             "overlaps a gore pixel (at least one pixel gap)")
    return {
        "schema_version": 1,
        "id": s["manifest_id"],
        "type": "character_animation",
        "source_path": str(s["ase"].relative_to(ROOT)),
        "grid_source": str(s["grid"].relative_to(ROOT)),
        "output_atlas": f"generated/art/{s['stem']}.png",
        "output_metadata": f"generated/art/{s['stem']}.json",
        "palette_id": s["palette_id"],
        "frame_size": list(CANVAS),
        "pivot": list(ad.PIVOT),
        "animation_tags": [t[0] for t in s["tags"]],
        "required_alpha": True,
        "owner": "AI-authored enemy animation (fan-concept layer: common Floor 1 goblin; runtime role IP-neutral small melee enemy)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {"origin": PROVENANCE_ORIGIN[kind] + (B_NOTE if variant == "B" else ""),
                       "generator": PROVENANCE_GEN, "generated_art": True,
                       "input_reference_ids": ["floor-01-opening-concept-references-v1"]},
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "cf02_goblin": block,
    }


# --------------------------------------------------------------------------- toolkit configs and key fixtures
BBOX = {"acting": [("hesitate", [0], {"top": 19, "bottom": 55, "left": 16, "right": 46}),
                   ("panic-flee", [1], {"top": 18, "bottom": 55, "left": 4, "right": 51}),
                   ("panic-flee-b-bounce", [2], {"top": 20, "bottom": 55, "min_width": 40}),
                   ("taunt-thump", [3], {"top": 20, "bottom": 55, "left": 1, "right": 42}),
                   ("taunt-beckon-reach", [4], {"top": 20, "bottom": 55, "right": 47})],
        "slammed": [("stomped-impact-low", [0], {"top": 36, "bottom": 55}),
                    ("stomped-rest-low", [1], {"top": 37, "bottom": 55}),
                    ("wall-impact-lowest-row-and-contact-column", [2], {"top": 21, "bottom": 55, "left": 16}),
                    ("wall-rest-smear-column", [3], {"top": 33, "bottom": 55, "left": 14}),
                    ("flung-impact-head-on-floor", [4], {"top": 35, "bottom": 55}),
                    ("flung-rest-low-heap", [5], {"top": 40, "bottom": 55})]}


def config_for(sheet: str) -> dict:
    s = SHEETS[sheet]
    kind = s["kind"]
    palette_n = len(grid_for(sheet)["palette"])
    tags = {}
    for name, pb, rows in s["tags"]:
        tags[name] = {"playback": pb, "contact": "grounded", "frames": [[f, t] for f, t in rows]}
    layers = ["weapon_back", "body", "weapon"] if kind == "acting" else ["body", "weapon", "gore"]
    cfg = {
        "format": "pixelart-asset-v1",
        "asset_id": s["asset_id"],
        "title": (f"ART-GOB-CF02 goblin {s['variant']} {'acting' if kind == 'acting' else 'slammed'} sheet "
                  f"({SPEC}): {len(s['names'])} frames, {len(KEY_FRAMES[kind])} frozen keys"),
        "grid": str(s["grid"].relative_to(ROOT)),
        "manifest": str(s["manifest"].relative_to(ROOT)),
        "frame_size": list(CANVAS),
        "pivot": list(ad.PIVOT),
        "contact_row": 55,
        "tick_hz": TICK_HZ,
        "hold_ms": HOLD_MS,
        "facing": "right",
        "tags": tags,
        "roles": {},
        "rules": {
            "max_colors": max(palette_n, 20 if kind == "slammed" else palette_n),
            "max_isolated_per_frame": 0,
            "max_orphans_per_frame": 0,
            "canvas_margin": [1, 0, 1, 0],
            "unique_frames": True,
            "manifest_tick_rows": "cf02_goblin.frames",
            "manifest_tick_hz": "cf02_goblin.tick_hz",
            "frozen_keys": {"fixture": str(KEY_FIXTURES[kind].relative_to(ROOT)), "frames": KEY_FRAMES[kind],
                            "layers": layers},
            "bbox": [{"id": i, "frames": f, **r} for i, f, r in BBOX[kind]],
        },
        "review": {"strip_tags": [t[0] for t in s["tags"]]},
        "scene": {"actors": [{"frame": f, "x": x} for f, x in zip(range(len(s["names"])), (520, 580, 640, 700, 760, 820))]},
    }
    return cfg


def fixture_for(kind: str) -> dict:
    """The frozen-key fixture: one SHA-256 per key frame over the A layers (the same digests freeze the B sheets)."""
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart import validators
    from scripts.pixelart.config import AssetConfig
    sheet = "acting_a" if kind == "acting" else "slammed_a"
    cfg = AssetConfig(config_for(sheet))
    layers = ["weapon_back", "body", "weapon"] if kind == "acting" else ["body", "weapon", "gore"]
    return validators.freeze(cfg, grid_for(sheet), KEY_FRAMES[kind], layers)


def dump_json(data: dict) -> str:
    return json.dumps(data, indent=2) + "\n"


def outputs() -> list[tuple[Path, str]]:
    """Every generated file as ``(path, text)``; the .ase files are written separately."""
    out = []
    for key, s in SHEETS.items():
        out.append((s["grid"], ad.dump(grid_for(key))))
    for kind, path in KEY_FIXTURES.items():
        out.append((path, dump_json(fixture_for(kind))))
    for key, s in SHEETS.items():
        out.append((s["config"], dump_json(config_for(key))))
    return out


def write_all() -> None:
    import pixel_grid_to_ase as pga
    for path, text in outputs():
        path.write_text(text, encoding="utf-8")
        print(f"wrote {path.relative_to(ROOT)}")
    for key, s in SHEETS.items():
        pga.write_ase(load(s["grid"]), s["ase"])
        print(f"wrote {s['ase'].relative_to(ROOT)}")
    for key, s in SHEETS.items():                      # the manifests read the committed grids
        s["manifest"].write_text(dump_json(manifest_for(key)), encoding="utf-8")
        print(f"wrote {s['manifest'].relative_to(ROOT)}")


def check_all() -> int:
    import pixel_grid_to_ase as pga
    bad = [str(p.relative_to(ROOT)) for p, t in outputs() if not p.is_file() or p.read_text(encoding="utf-8") != t]
    bad += [str(s["ase"].relative_to(ROOT)) for s in SHEETS.values()
            if not s["ase"].is_file() or s["ase"].read_bytes() != pga.grid_to_ase(load(s["grid"]))]
    bad += [str(s["manifest"].relative_to(ROOT)) for k, s in SHEETS.items()
            if not s["manifest"].is_file() or s["manifest"].read_text(encoding="utf-8") != dump_json(manifest_for(k))]
    print("goblin CF02 sources up to date" if not bad else "differ from the generator: " + ", ".join(bad))
    return 1 if bad else 0


def main(argv: list[str] | None = None) -> int:
    import argparse
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["all"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.check:
        return check_all()
    write_all()
    return 0


if __name__ == "__main__":
    sys.exit(main())
