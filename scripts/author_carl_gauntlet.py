#!/usr/bin/env python3
"""Deterministic authoring record of the CARL-GAUNTLET-01 overlay sheets (Carl's reinforced fist).

The gauntlet is a separate full-cell overlay atlas per Carl body sheet (locomotion, melee, cf02, dynamite): same
48 x 56 cells, same pivot, frame order, tags and durations, one ``gauntlet`` layer. Three frames are the frozen
Opus keys (``gauntlet_keys/carl_h52_gauntlet_keys.grid.json``): locomotion 0, melee 4 and melee 5 are copied
byte for byte. The other 95 frames are stamped from the same construction by this script (Sonnet): per hand of the
body frame (a connected patch of near-arm skin) a glove is built from the key's materials (brown strap ``H`` at the
wrist, steel plate ``U``/``V``/``T`` with a lit knuckle bar, glint ``w``, one dent, a 1 px heavier outline at the
far end). Only the near hand (``near_arm`` layer, CG-AD2) is gloved, and only keys ``K J j T U V w H`` are used.
No image-model output, no resampling, no rotation of pixels.

Layout of this file: the generator (section A), the hand-fix tables (B), the writer (C) and a review command (D).

    python3 scripts/author_carl_gauntlet.py write      # write the four overlay grids (gauntlet layer only)
    python3 scripts/author_carl_gauntlet.py check      # CG-L1..CG-L9 on every frame, print findings
    python3 scripts/author_carl_gauntlet.py review DIR # zoomed bare-vs-gauntlet strips of every frame
    python3 scripts/author_carl_gauntlet.py freeze     # write tests/fixtures/art-carl-gauntlet_keys.json (once; keys are frozen)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from scripts.pixelart import validators as val  # noqa: E402

CARL = ROOT / "assets/source/characters/carl"
KEYS_GRID = CARL / "gauntlet_keys/carl_h52_gauntlet_keys.grid.json"
SHEETS = ("locomotion", "melee", "cf02", "dynamite")
LAYER = "gauntlet"
SKIN = "LMSD"
PARAMS = val.overlay_params({"body_grid": "x"})
# the frozen keys: key tag -> (sheet, frame); their gauntlet cels come from KEYS_GRID byte for byte
KEY_FRAMES = {"idle_hand": ("locomotion", 0), "punch_b_windup": ("melee", 4), "punch_b_contact": ("melee", 5)}


def body_path(sheet: str) -> Path:
    return CARL / f"carl_h52_{sheet}.grid.json"


def overlay_path(sheet: str) -> Path:
    return CARL / f"carl_h52_{sheet}_gauntlet.grid.json"


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


# ===================================================================== A. generator

DIRS = {"U": (0, -1), "D": (0, 1), "L": (-1, 0), "R": (1, 0)}


def components(points: set) -> list[set]:
    """8-connected components, ordered by their top-left pixel."""
    seen, out = set(), []
    for start in sorted(points, key=lambda p: (p[1], p[0])):
        if start in seen:
            continue
        stack, comp = [start], {start}
        seen.add(start)
        while stack:
            x, y = stack.pop()
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    q = (x + dx, y + dy)
                    if q in points and q not in seen:
                        seen.add(q)
                        comp.add(q)
                        stack.append(q)
        out.append(comp)
    return out


class Body:
    """One body frame: the layers the overlay rules look at."""

    def __init__(self, grid: dict, index: int) -> None:
        cels = grid["frames"][index]["cels"]
        self.w, self.h = grid["canvas"]
        self.near = val.cel_points(cels.get("near_arm"))
        self.watch = set(val.cel_points(cels.get("watch")))
        self.pin = set(val.cel_points(cels.get("pin")))
        self.prop = set(val.cel_points(cels.get("prop"))) - set(self.near)   # visible stick pixels (CG-EX-AD2)
        self.head_hair = set(val.cel_points(cels.get("head"))) | set(val.cel_points(cels.get("hair")))
        self.exposed = {p for p, k in self.near.items() if k in SKIN} - self.watch - self.pin
        self.grown = val.dilate1(set(self.near), self.w, self.h)
        self.flat = val.pg.composite_keys(grid, index)

    def free(self, p) -> bool:
        """May an overlay pixel sit here (CG-L1..CG-L3 for a pixel outside the arm)?"""
        if p[0] in (0, self.w - 1) or p not in self.grown or p in self.watch or p in self.pin or p in self.prop:
            return False
        return p in self.near or p not in self.head_hair

    def wrist_side(self, comp: set) -> str:
        """Where the cuff is: towards the watch pixel when one sits beside the patch, else towards the sleeve."""
        x0, x1 = min(p[0] for p in comp), max(p[0] for p in comp)
        y0, y1 = min(p[1] for p in comp), max(p[1] for p in comp)
        near_watch = [p for p in self.watch if x0 - 2 <= p[0] <= x1 + 2 and y0 - 2 <= p[1] <= y1 + 2]
        if near_watch:
            wx = sum(p[0] for p in near_watch) / len(near_watch)
            wy = sum(p[1] for p in near_watch) / len(near_watch)
            dx, dy = wx - (x0 + x1) / 2, wy - (y0 + y1) / 2
            return ("R" if dx > 0 else "L") if abs(dx) > abs(dy) else ("D" if dy > 0 else "U")
        count = dict.fromkeys("UDLR", 0)
        for (x, y), k in self.near.items():
            if k in "LMSDK" or not (x0 - 3 <= x <= x1 + 3 and y0 - 3 <= y <= y1 + 3):
                continue
            count["U"] += y < y0
            count["D"] += y > y1
            count["L"] += x < x0
            count["R"] += x > x1
        return max("UDLR", key=lambda s: count[s])


def glove(body: Body, comp: set, side: str) -> dict:
    """The glove of one hand patch: ``{(x, y): key}``. ``side`` is where the cuff is (U D L R)."""
    x0, x1 = min(p[0] for p in comp), max(p[0] for p in comp)
    y0, y1 = min(p[1] for p in comp), max(p[1] for p in comp)
    horiz = side in "LR"
    depth_n = (x1 - x0 + 1) if horiz else (y1 - y0 + 1)

    def depth(p):
        return {"U": p[1] - y0, "D": y1 - p[1], "L": p[0] - x0, "R": x1 - p[0]}[side]

    def cross(p):
        return p[1] - y0 if horiz else p[0] - x0

    out: dict = {}
    by_depth: dict = {}
    for p in comp:
        by_depth.setdefault(depth(p), []).append(p)
    for d, pts in by_depth.items():
        pts.sort(key=cross)
        n = len(pts)
        for i, p in enumerate(pts):
            if depth_n >= 2 and d == 0:
                out[p] = "H"
            elif d == depth_n - 1:
                # lit knuckle bar: a glint at the lit end, steel face, a dent, shadow at the far end (V needs a V neighbour)
                out[p] = ("w" if i == 0 and n >= 3 else "T" if i == n - 1 and n >= 4 else
                          "U" if i == 2 and n >= 5 else "U" if n == 1 else "V")
            else:
                out[p] = "T" if i == n - 1 else "U"
    # one heavier row at the far end: the outline pixel there becomes steel shadow, a new outline pixel goes beyond
    # (only along runs of two or more pixels, so no lone spur is made)
    fx, fy = DIRS[{"U": "D", "D": "U", "L": "R", "R": "L"}[side]]
    cands = []
    for p in sorted((q for q in out if depth(q) == depth_n - 1), key=cross):
        q = (p[0] + fx, p[1] + fy)
        q2 = (q[0] + fx, q[1] + fy)
        if body.near.get(q) == "K" and q not in out and body.free(q) and body.free(q2):
            cands.append((cross(p), q, q2))
    for c, q, q2 in cands:
        if any(abs(c - o) == 1 for o, _, _ in cands):
            out[q] = "T"
            out[q2] = "K"
    return out


def repair_orphans(body_flat: list, over: dict, protected: set) -> dict:
    """Recolour overlay pixels that have no same-key 8-neighbour (CG-L8) to the commonest neighbouring overlay key."""
    over = dict(over)
    for _ in range(4):
        changed = False
        for p in sorted(over, key=lambda q: (q[1], q[0])):
            k = over[p]
            if k == "w" or p in protected:
                continue
            around = [(p[0] + dx, p[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx, dy) != (0, 0)]
            seen = [over.get(q) or body_flat[q[1]][q[0]] for q in around
                    if 0 <= q[0] < len(body_flat[0]) and 0 <= q[1] < len(body_flat)]
            if k in seen:
                continue
            votes: dict = {}
            for q in around:
                if q in over and over[q] not in ("w", k):
                    votes[over[q]] = votes.get(over[q], 0) + 1
            if votes:
                over[p] = max(sorted(votes), key=lambda c: votes[c])
                changed = True
        if not changed:
            break
    return over


def repair_strays(body: "Body", over: dict, protected: set) -> dict:
    """Make the overlay add no orphan or isolated pixel to the body (CG-L8) by recolouring overlay pixels.

    A new orphan is either an overlay pixel without a same-key neighbour (it takes the commonest neighbouring overlay
    key) or a body pixel whose last same-key neighbour the overlay covered (one overlay neighbour takes the body
    pixel's key, when that key is allowed on the gauntlet).
    """
    accents = {"w", "Y", "G", "E", "p"}
    over = dict(repair_orphans(body.flat, over, protected))
    for _ in range(6):
        flat = [list(r) for r in body.flat]
        for (x, y), k in over.items():
            flat[y][x] = k
        base = val.pg._stray_pixels(body.flat, accents)
        both = val.pg._stray_pixels(flat, accents)
        old = {tuple(q) for part in base for q in part}
        new = [tuple(q) for part in both for q in part if tuple(q) not in old]
        body_new = [q for q in new if q not in over]
        if not body_new:
            break
        for q in sorted(body_new, key=lambda t: (t[1], t[0])):
            kq = flat[q[1]][q[0]]
            cand = [(q[0] + dx, q[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                    if (dx, dy) != (0, 0) and (q[0] + dx, q[1] + dy) in over and (q[0] + dx, q[1] + dy) not in protected]
            if kq in PARAMS["allowed_keys"] and cand:
                # the neighbour with the fewest overlay neighbours of its own (an edge pixel of the glove)
                r = min(cand, key=lambda t: (sum(1 for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                                                 if (t[0] + dx, t[1] + dy) in over), t[1], t[0]))
                over[r] = kq
        over = repair_orphans(body.flat, over, protected)
    return over


# ===================================================================== B. hand fixes

# which way the cuff lies in each frozen key (per patch), so a key is reused only on the same hand with the same cuff
KEY_SIDES = {("locomotion", 0): ["D", "U"], ("melee", 4): ["D"], ("melee", 5): ["U"]}  # melee 4 via SIDES
# side overrides: (sheet, frame) -> {component index (top-left order): side}
SIDES: dict = {("melee", 4): {0: "D"}}
# hand-fix tables: (sheet, frame) -> {(x, y): key or "." to clear}. The four 11 px fists below got no dent from the
# generator: the plate row ends in two T pixels (a shadow notch) instead of the stray H the repair left
EDITS: dict = {
    ("melee", 6): {(40, 36): "T", (41, 36): "T"},
    ("melee", 21): {(31, 45): "T", (32, 45): "T"},
    ("melee", 22): {(33, 45): "T", (34, 45): "T"},
    ("dynamite", 0): {(14, 35): "T", (15, 35): "T"},
}


def _rows(*lines: str) -> dict:
    """A hand template from text rows (``.`` = no pixel), relative to the patch's top-left pixel."""
    return {(x, y): ch for y, line in enumerate(lines) for x, ch in enumerate(line) if ch != "."}


# Open hands (spec section 3: dark plum leather fingers and palm, a small steel back plate with a lit bar and a dent,
# the brown strap at the cuff, never a flat steel fill or a claw). Review fix F1 of PR #284. Each template covers
# exactly the skin pixels of one body hand shape; the key is (sheet, frame) of a frame that shows the shape.
HAND_TEMPLATES = {
    # two knuckle columns with a finger gap, wrist below: leather finger ends, lit steel bar, strap
    ("melee", 18): _rows("jj.jj",
                         "JJ.JJ",
                         "wVVTT",
                         ".HHH."),
    # open hand, three finger tips up, wrist below
    ("cf02", 9): _rows("j.j.j",
                       "jjjjj",
                       "wVVTT",
                       ".HHH."),
    # frustration, fingers up (cf02 11), the cuff below on the left
    ("cf02", 11): _rows("...j...",
                        "j..j..j",
                        ".j.j.j.",
                        "..wVVTT",
                        "..HHHH"),
    # frustration, fingers down (cf02 12), the cuff on the left: strap and lit plate on the back of the hand
    ("cf02", 12): _rows("..HwVT",
                        "..HVVTT",
                        ".j.j.j.",
                        "j..j..j",
                        "...j..."),
    # knockdown: the hand lies on the floor, wrist above (melee 20)
    ("melee", 20): _rows("....J.J",
                         "HHHHHH",
                         "wVVUTT",
                         "VUUTT",
                         "jjjJJ"),
    # a fist by the cheek with two finger tips up, wrist below (dynamite 7)
    ("dynamite", 7): _rows("j.j",
                           "j.j",
                           "wVV",
                           "HHH",
                           ".H."),
}
TEMPLATE_BY_SHAPE: dict = {}


def register_templates() -> None:
    """Index the templates by the skin shape of the frame they name."""
    for (sheet, index), rows in HAND_TEMPLATES.items():
        body = Body(load(body_path(sheet)), index)
        patch = hand_patches(sheet, index, body)[0]
        sig = signature(patch)
        covered = {p for p in rows if p in set(sig)}
        if covered != set(sig):
            raise SystemExit(f"hand template {sheet}[{index}] does not cover its skin shape exactly")
        TEMPLATE_BY_SHAPE[sig] = rows
# skin pixels of the near arm that are not part of the hand (CG-L6 exceptions): (sheet, frame) -> [(x, y)]
# melee 5: the jaw-line pixel of the head drawn on the arm layer; melee 22 and dynamite 6, 39-44: a skin shading
# pixel of the chest edge that the arm layer carries inside the sleeve. None is on the hand.
EXCEPTIONS: dict = {
    ("melee", 5): [(35, 19)],
    ("melee", 22): [(27, 30), (29, 35)],
    ("dynamite", 6): [(33, 30)],
    **{("dynamite", i): [(31, 30 if i in (40, 43) else 29)] for i in range(39, 45)},
}


def signature(comp: set) -> tuple:
    """The skin patch shape, independent of where it sits."""
    x0, y0 = min(p[0] for p in comp), min(p[1] for p in comp)
    return tuple(sorted((x - x0, y - y0) for x, y in comp))


def hand_patches(sheet: str, index: int, body: Body) -> list[set]:
    """The exposed near-arm skin patches that are hands (the CG-L6 exceptions are left out)."""
    skip = set(EXCEPTIONS.get((sheet, index), ()))
    return [c for c in components(body.exposed) if not c <= skip]


def key_stamps(keys: dict) -> list[tuple]:
    """``(patch shapes, key overlay relative to the first patch, sides)`` of the frozen keys, to reuse on equal hands."""
    stamps = []
    for (sheet, index), over in keys.items():
        grid = load(body_path(sheet))
        body = Body(grid, index)
        patches = hand_patches(sheet, index, body)
        ox, oy = min(patches[0], key=lambda p: (p[1], p[0]))
        shapes = [signature(c) for c in patches]
        stamps.append((shapes, {(x - ox, y - oy): k for (x, y), k in over.items()}, KEY_SIDES[(sheet, index)]))
    return stamps


def draw_frame(sheet: str, index: int, body: Body, keys: dict, stamps: list) -> dict:
    """The overlay pixels of one body frame."""
    if (sheet, index) in keys:
        return keys[(sheet, index)]
    patches = hand_patches(sheet, index, body)
    sides = [SIDES.get((sheet, index), {}).get(ci) or body.wrist_side(c) for ci, c in enumerate(patches)]
    over: dict = {}
    for shapes, rel, key_sides in stamps:
        if [signature(c) for c in patches] == shapes and sides == key_sides:
            ox, oy = min(patches[0], key=lambda p: (p[1], p[0]))
            over = {(x + ox, y + oy): k for (x, y), k in rel.items() if body.free((x + ox, y + oy))}
            break
    else:
        for comp, side in zip(patches, sides):
            template = TEMPLATE_BY_SHAPE.get(signature(comp))
            if template is not None:
                x0, y0 = min(p[0] for p in comp), min(p[1] for p in comp)
                built = {(x + x0, y + y0): k for (x, y), k in template.items()}
            else:
                built = glove(body, comp, side)
            over.update({p: k for p, k in built.items() if body.free(p)})
    for p, k in EDITS.get((sheet, index), {}).items():
        if k == ".":
            over.pop(p, None)
        else:
            over[p] = k
    return repair_strays(body, over, set(EDITS.get((sheet, index), {})))


# ===================================================================== C. writer

def key_cels() -> dict:
    grid = load(KEYS_GRID)
    names = [t["name"] for t in grid["tags"]]
    return {KEY_FRAMES[n]: val.cel_points(grid["frames"][i]["cels"][LAYER]) for i, n in enumerate(names)}


def to_rows(points: dict, w: int, h: int) -> list[str]:
    rows = [["."] * w for _ in range(h)]
    for (x, y), k in points.items():
        rows[y][x] = k
    return ["".join(r) for r in rows]


def build_overlay(sheet: str, keys: dict, stamps: list | None = None) -> tuple[dict, dict]:
    """``(overlay grid, {frame: points})`` of one sheet."""
    if not TEMPLATE_BY_SHAPE:
        register_templates()
    body = load(body_path(sheet))
    w, h = body["canvas"]
    frames, drawn = [], {}
    stamps = key_stamps(keys) if stamps is None else stamps
    for i, fr in enumerate(body["frames"]):
        pts = draw_frame(sheet, i, Body(body, i), keys, stamps)
        drawn[i] = pts
        frames.append({"duration_ms": fr["duration_ms"], "cels": {LAYER: to_rows(pts, w, h)}})
    grid = {"format": "pixel-grid-v1", "canvas": body["canvas"], "pivot": body["pivot"],
            "palette": [e for e in body["palette"] if e["key"] in PARAMS["allowed_keys"]],   # the 8 gauntlet keys (CG-L5)
            "layers": [LAYER], "frames": frames, "tags": body["tags"]}
    return grid, drawn


def write_grid(path: Path, grid: dict) -> None:
    path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")


# ===================================================================== E. manifests and configs

BODY_IDS = {s: load(ROOT / f"assets/manifests/carl_book1_h52_{s}.json")["id"] for s in SHEETS}
LICENSE = ("Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
           "no game/asset-pack distribution rights")


def manifest_for(sheet: str, grid: dict) -> dict:
    body = load(ROOT / f"assets/manifests/carl_book1_h52_{sheet}.json")
    frames = len(grid["frames"])
    key_frames = {n: f for n, (sh, f) in KEY_FRAMES.items() if sh == sheet}
    return {
        "schema_version": 1,
        "id": f"carl_book1_h52_{sheet}_gauntlet_candidate_v1",
        "type": "character_animation",
        "source_path": f"assets/source/characters/carl/carl_h52_{sheet}_gauntlet.ase",
        "grid_source": f"assets/source/characters/carl/carl_h52_{sheet}_gauntlet.grid.json",
        "output_atlas": f"generated/art/carl_book1_h52_{sheet}_gauntlet.png",
        "output_metadata": f"generated/art/carl_book1_h52_{sheet}_gauntlet.json",
        "palette_id": body["palette_id"],
        "frame_size": body["frame_size"],
        "pivot": body["pivot"],
        "animation_tags": body["animation_tags"],
        "required_alpha": True,
        "owner": "AI-authored pixel overlay candidate (fan-concept layer: Carl -> demolitionist, reinforced fist)",
        "tool_version": body["tool_version"],
        "provenance": {
            "origin": ("CARL-GAUNTLET-01 overlay of the ANIM-CARL-02 body sheet " + body["id"] + ": a single gauntlet "
                       "layer, same cell grid, pivot, frame order, tags and durations, on the camera-near hand (near_arm "
                       "layer): 3 frames are hand-placed keys, the others are stamped by script from those keys; the "
                       "body sheet is unchanged and is not copied into this file"),
            "generator": ("Design and the 3 frozen key frames (locomotion 0, melee 4, melee 5): Claude Opus (hand-placed pixel "
                          "grids, assets/source/characters/carl/gauntlet_keys/carl_h52_gauntlet_keys.grid.json). Remaining "
                          "frames: Claude Sonnet, stamped by scripts/author_carl_gauntlet.py; no image-model raster output, "
                          "no resampling, no rotation"),
            "generated_art": True,
            "input_reference_ids": ["CARL-CONCEPT-APPROVED-V1"],
        },
        "license": LICENSE,
        "review_status": "prototype_fixture_unapproved",
        "overlay_of": body["id"],
        "overlay": {
            "status": f"{frames} frames drawn (Sonnet, {len(key_frames)} of them Opus keys), awaiting independent Opus "
                      "review and owner art review; presented by src/bin/floor01_scene/carl_gauntlet.rs",
            "spec": "docs/02-art/carl-gauntlet-01-spec.md",
            "body_manifest": f"assets/manifests/carl_book1_h52_{sheet}.json",
            "body_grid": f"assets/source/characters/carl/carl_h52_{sheet}.grid.json",
            "layer": LAYER,
            "key_frames": key_frames,
            "empty_frames": [i for i, f in enumerate(grid["frames"]) if not any(set(r) != {"."} for r in f["cels"][LAYER])],
            "skin_exceptions": {str(i): [list(p) for p in pts] for (sh, i), pts in sorted(EXCEPTIONS.items()) if sh == sheet},
            "skin_exception_note": ("near-arm skin pixels that are not on the hand: the jaw-line pixel of the head on the arm "
                                    "layer (melee 5) or a chest-edge shading pixel carried inside the sleeve; CG-L6 skips them"),
        },
    }


def config_for(sheet: str, grid: dict) -> dict:
    tags = {}
    for t in grid["tags"]:
        tags[t["name"]] = {"playback": "once" if t["from"] == t["to"] else "loop", "contact": "free",
                           "frames": [[i, grid["frames"][i]["duration_ms"]] for i in range(t["from"], t["to"] + 1)]}
    return {
        "format": "pixelart-asset-v1",
        "asset_id": f"art-carl-gauntlet-{sheet}",
        "title": f"CARL-GAUNTLET-01 reinforced-fist overlay for the Carl 52 px {sheet} sheet ({len(grid['frames'])} frames)",
        "grid": f"assets/source/characters/carl/carl_h52_{sheet}_gauntlet.grid.json",
        "manifest": f"assets/manifests/carl_book1_h52_{sheet}_gauntlet.json",
        "frame_size": grid["canvas"], "pivot": grid["pivot"], "contact_row": None,
        "tick_hz": 1000, "hold_ms": 100, "facing": "right",
        "tags": tags,
        "overlay": {"body_grid": f"assets/source/characters/carl/carl_h52_{sheet}.grid.json", "layer": LAYER},
        "rules": {"max_colors": len(PARAMS["allowed_keys"])},
        "_note": "tick_hz 1000: the tick column is the frame duration in ms, copied from the body sheet",
    }


def write_all(keys: dict) -> None:
    for sheet in SHEETS:
        grid, _ = build_overlay(sheet, keys)
        write_grid(overlay_path(sheet), grid)
        (ROOT / f"assets/manifests/carl_book1_h52_{sheet}_gauntlet.json").write_text(
            json.dumps(manifest_for(sheet, grid), indent=2) + "\n", encoding="utf-8")
        (CARL / f"carl_h52_{sheet}_gauntlet.pixelart.json").write_text(
            json.dumps(config_for(sheet, grid), indent=2) + "\n", encoding="utf-8")


FIXTURE = ROOT / "tests/fixtures/art-carl-gauntlet_keys.json"


def freeze_keys() -> None:
    """The frozen-key fixture: one SHA-256 of the ``gauntlet`` cel per key, taken from the key grid itself."""
    grid = load(KEYS_GRID)
    names = [t["name"] for t in grid["tags"]]
    keys = {}
    for i, name in enumerate(names):
        sheet, frame = KEY_FRAMES[name]
        keys[name] = {"sheet": sheet, "frame": frame,
                      "sha256": val.frame_digest(grid["frames"][i]["cels"], [LAYER])}
    fixture = {"format": "pixelart-keys-v1", "asset_id": "art-carl-gauntlet", "layers": [LAYER], "keys": keys,
               "source": "assets/source/characters/carl/gauntlet_keys/carl_h52_gauntlet_keys.grid.json"}
    FIXTURE.write_text(json.dumps(fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")


# ===================================================================== D. review

def review(out_dir: Path, sheets=SHEETS, zoom: int = 10) -> None:
    """Zoomed bare-vs-gauntlet crops of every frame, one PNG per sheet (for the author, not an evidence asset)."""
    from PIL import Image
    keys = key_cels()
    out_dir.mkdir(parents=True, exist_ok=True)
    for sheet in sheets:
        body = load(body_path(sheet))
        _, drawn = build_overlay(sheet, keys)
        pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in body["palette"]}
        cw, ch = 22, 16
        cells = []
        for i in range(len(body["frames"])):
            flat = val.pg.composite_keys(body, i)
            pts = drawn[i]
            if pts:
                cx = (min(p[0] for p in pts) + max(p[0] for p in pts)) // 2
                cy = (min(p[1] for p in pts) + max(p[1] for p in pts)) // 2
            else:
                cx, cy = 24, 28
            x0 = max(0, min(48 - cw, cx - cw // 2))
            y0 = max(0, min(56 - ch, cy - ch // 2))
            pair = []
            for ov in (None, pts):
                im = Image.new("RGBA", (cw, ch), (52, 44, 70, 255))
                for y in range(ch):
                    for x in range(cw):
                        k = (ov or {}).get((x0 + x, y0 + y)) or flat[y0 + y][x0 + x]
                        if k != ".":
                            im.putpixel((x, y), pal[k])
                pair.append(im.resize((cw * zoom, ch * zoom), Image.NEAREST))
            cells.append(pair)
        per_row = 4
        rows = (len(cells) + per_row - 1) // per_row
        sheet_im = Image.new("RGBA", (per_row * (2 * cw * zoom + 12), rows * (ch * zoom + 8)), (20, 20, 20, 255))
        for n, (a, b) in enumerate(cells):
            gx, gy = (n % per_row) * (2 * cw * zoom + 12), (n // per_row) * (ch * zoom + 8)
            sheet_im.paste(a, (gx, gy))
            sheet_im.paste(b, (gx + cw * zoom + 2, gy))
        sheet_im.save(out_dir / f"{sheet}.png")


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else "check"
    if cmd == "freeze":
        freeze_keys()
        return 0
    if cmd == "review":
        review(Path(argv[2]), tuple(argv[3:]) or SHEETS)
        return 0
    keys = key_cels()
    if cmd == "write":
        write_all(keys)
    bad = 0
    for sheet in SHEETS:
        grid, drawn = build_overlay(sheet, keys)
        body = load(body_path(sheet))
        for i in range(len(grid["frames"])):
            rows = grid["frames"][i]["cels"][LAYER]
            found = val.overlay_frame_findings(body["frames"][i]["cels"], rows, body["canvas"], PARAMS,
                                               EXCEPTIONS.get((sheet, i), ()))
            found["CG-L8"] = val.overlay_l8(body, rows, i, LAYER)[0]
            for rule, msgs in found.items():
                for m in msgs:
                    bad += 1
                    print(f"{sheet}[{i}] {rule} {m}")
    print(f"findings: {bad}")
    return 1 if bad and cmd == "check" else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
