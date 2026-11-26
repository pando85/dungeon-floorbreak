#!/usr/bin/env python3
"""LLAMA-03 authoring record: the Bad Llama rush keys (pixel-grid-v1 source) and the silhouette gate.

Usage:

    python3 scripts/author_llama03_rush.py keys [--check]
    python3 scripts/author_llama03_rush.py atlas [--check]
    python3 scripts/author_llama03_rush.py gate
    python3 scripts/author_llama03_rush.py evidence

``keys`` (re)writes ``assets/source/characters/llama/rush/llama_rush_keys.grid.json``: the four design keys of
``docs/02-art/llama-03-rush-spec.md`` on the LLAMA-01 frame contract (96 x 72, pivot (44, 71), contact row 71, the
17-key ``llama-floor01-v0`` palette): 0 ``brace``, 1 ``scrape``, 2 ``run_contact``, 3 ``recover_skid``. They become
frames 1, 2, 4 and 8 of the ten-frame rush atlas (spec section 4); ``tests/fixtures/llama-03-rush_keys.json`` freezes
them under those atlas indices. ``--check`` compares instead of writing.

``evidence`` writes ``contact-sheet.png`` and ``rush.gif`` of the ten-frame atlas into the same folder and refreshes
``SHA256SUMS``. ``gate`` writes the silhouette gate into ``docs/evidence/llama-03-rush/``: the keys next to the LLAMA-01 idle and spit
silhouettes, Carl, Donut and a goblin (black and in colour, 1x and 4x), the keys on the Infernal Corral at 1x and 3x,
and the lane mock.

``atlas`` (re)writes ``llama_rush_h64.grid.json``: the ten frames of the rush atlas (spec section 4), the four keys above
plus six in-betweens (``BRACE_IN``, ``COIL``, ``RUN_GATHER``, ``RUN_PUSH``, ``RUN_REACH``, ``RECOVER_BLOWN``). The keys come
byte-identical from the key grid; ``keys`` and ``freeze`` are never rerun after the silhouette gate (K1 and K2).

Construction: every body pixel is rasterized by the LLAMA-01 parametric rig (``scripts/author_llama01.py``) from the
pose dicts below plus the rig's literal face stamps and two new ones (``EYES['squint']``, ``TONGUE``). No concept
pixels, no image-model output. A rush frame never carries a glow key (r, R, O, Y): the rush must never read as a spit.
Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_llama01 as L  # noqa: E402  (the LLAMA-01 rig: Part, Canvas, poly, mane, stamps, key poses)

RUSH_DIR = ROOT / "assets/source/characters/llama/rush"
KEYS_GRID = RUSH_DIR / "llama_rush_keys.grid.json"
KEYS_ASE = RUSH_DIR / "llama_rush_keys.ase"
FIXTURE = ROOT / "tests/fixtures/llama-03-rush_keys.json"
EVIDENCE = ROOT / "docs/evidence/llama-03-rush"
CANVAS = L.CANVAS
PIVOT = L.PIVOT
TICK_HZ = 60

# The four keys: (name, pose, atlas index in the ten-frame rush atlas of spec section 4, design ticks for the preview)
EYES = dict(L.EYES) | {
    "squint": ["KKK.", ".KKK", "..K."],      # winded squint: lid down over the eye, no white
}
GLOW = set(L.GLOW)
# Gate K1: the winded tongue hangs 2 px below the jaw line (a panting tongue, not a red snarl); gum `g` only.
TONGUE = ["gK", "ggK", "ggK", "KgK", ".K"]


def hooves(**kw):
    return {k + "_hoof": v for k, v in kw.items()}


# --------------------------------------------------------------------------- key 1: brace (atlas frame 1)
# The committed lane is told by the whole body: the fore legs are planted stiff and splayed forward, the chest drops,
# the back slopes down to the front, the hind legs coil under the rump, and the long neck runs forward and level so
# the head sits about 2 px under the withers, aimed down the lane like a lowered battering ram (never raised like the
# spit tell). The llama aggression cue is ears pinned flat along the neck (ring) plus the tail raised high. Clenched
# teeth, the angry eye. Throat plain fur: no glow, no sac.
BRACE = L.shifted(
    L.IDLE,
    rump=(27, 40), chest=(48, 43), barrel_rr=(5.5, 6.5),
    belly=(40, 46.5, 8, 4.0), haunch=(26, 41, 6.5, 7.0), shoulder=(49, 45, 5.5, 7.0),
    near_hind=[(27, 44, 4.5), (32, 51, 3.0), (25, 59, 2.1), (27, 67, 1.3)],
    far_hind=[(32, 44, 3.5), (36, 51, 2.5), (30, 59, 1.9), (32, 67, 1.2)],
    near_front=[(50, 48, 3.6), (54, 54, 2.4), (58, 60, 2.1), (61, 67, 1.3)],
    far_front=[(54, 48, 3.0), (58, 54, 2.2), (62, 60, 1.9), (66, 67, 1.2)],
    **hooves(near_hind=(27, 69), far_hind=(32, 69), near_front=(61, 69), far_front=(66, 69)),
    neck=((51, 40), (60, 38.5), (69, 38.5)), neck_r=(4.6, 3.5, 3.1),
    skull=(72, 37.5, 4.8, 4.0),
    muzzle=((74, 38.5), (82, 41)), muzzle_r=(3.4, 2.7), jaw=((73, 42.5), (80, 44.5)),
    mouth=None, mouth_line=None, teeth=[("clench", (76, 40))],
    ear_near=((69, 34.5), (65.5, 33.5), (62, 34.5)), ear_far=((70, 33.5), (66.5, 32.3), (63, 33.3)), ear_ring=True,
    tail=((21, 37), (13, 29), (-0.5, -1)),
    eye="angry", eye_at=(68, 34), nostril=(82, 38),
)

# --------------------------------------------------------------------------- key 2: scrape (atlas frame 2)
# The brace with the near fore hoof pawing back like a bull: the near fore leg drawn back under the chest with the knee
# bent and the hoof dragged on the floor 9 px behind its brace position (the dirt kick is the presentation layer's
# gouge, flecks and puff, spec section 7). The head dips 1 px and the body rocks 1 px forward onto the far fore leg.
# One literal stamp closes a mane pinhole on the neck (V17).
SCRAPE = L.shifted(
    BRACE,
    rump=(28, 40), chest=(49, 43.5), belly=(41, 47, 8, 4.0), haunch=(27, 41, 6.5, 7.0), shoulder=(50, 45.5, 5.5, 7.0),
    # Gate K2: the hind hooves stay planted on the brace footprint (rule 5); only the hip end follows the rock.
    near_hind=[(28, 44, 4.5), (32.5, 51, 3.0), (25, 59, 2.1), (27, 67, 1.3)],
    far_hind=[(33, 44, 3.5), (36.5, 51, 2.5), (30, 59, 1.9), (32, 67, 1.2)],
    near_front=[(51, 48.5, 3.6), (56, 54, 2.4), (54, 61, 2.1), (51, 67, 1.3)],
    far_front=[(55, 48.5, 3.0), (59, 54, 2.2), (63, 60, 1.9), (66, 67, 1.2)],
    **hooves(near_hind=(27, 69), far_hind=(32, 69), near_front=(50, 69), far_front=(66, 69)),
    **L.move_head(BRACE, dx=1, dy=1),
)
SCRAPE["neck"] = ((52, 40.5),) + tuple(L.mv(q, 1, 1) for q in BRACE["neck"][1:])
SCRAPE["extra"] = [(54, 34, ["K"])]                    # closes the mane pinhole on the neck

# --------------------------------------------------------------------------- key 4: run_contact (atlas frame 4)
# The charge: a low, long battering ram. The body is level and low (withers 2 px under the brace), the near fore leg
# reaches far forward to strike the floor, the far fore folds back under the chest, the near hind is flung out behind
# after the push and the far hind is planted under the barrel. The neck runs forward and slightly down so the skull
# sits at chest height in front of the shoulder, inside the rush box band (heights 2..30 above the contact row); the
# chest and shoulder lead. Ears flat on the neck, tail streaming straight back, clenched teeth, angry eye.
RUN_CONTACT = L.shifted(
    L.IDLE,
    rump=(30, 41), chest=(52, 43), barrel_rr=(5.5, 6.5),
    belly=(43, 47, 8, 3.8), haunch=(29, 42, 6.5, 6.5), shoulder=(53, 45, 5.5, 7.0),
    near_front=[(54, 48, 3.6), (60, 53, 2.4), (66, 58, 2.1), (71, 66, 1.3)],
    far_front=[(56, 48, 3.0), (54, 54, 2.2), (49, 58, 1.9), (45, 62, 1.2)],
    near_hind=[(30, 45, 4.5), (25, 51, 3.0), (19, 56, 2.1), (13, 62, 1.3)],
    far_hind=[(35, 45, 3.5), (38, 52, 2.5), (35, 59, 1.9), (37, 67, 1.2)],
    **hooves(near_front=(71, 69), far_front=(43, 63), near_hind=(11, 64), far_hind=(37, 69)),
    neck=((55, 41), (65, 40.5), (74, 42)), neck_r=(4.8, 3.6, 3.2),
    skull=(77, 42.5, 4.6, 3.8),
    muzzle=((79, 43.5), (86, 46.5)), muzzle_r=(3.2, 2.6), jaw=((78, 47.5), (84, 49)),
    mouth=None, mouth_line=None, teeth=[("clench", (81, 44))],
    ear_near=((74, 39), (70, 38), (66, 38)), ear_far=((75, 38), (71, 36.8), (67, 36.8)), ear_ring=True,
    tail=((22, 38), (10, 32), (-1, -0.5)),
    eye="angry", eye_at=(73, 39), nostril=(86, 43),
)

# --------------------------------------------------------------------------- key 8: recover_skid (atlas frame 8)
# The first recover frame: the stop. The fore legs are jammed stiff far forward (a skid), the rump sits down onto the
# coiled hind legs so the back slopes up to the front, and the head pitches forward and DOWN by inertia (never up:
# an upward head is the spit tell's language). Mouth hanging open (dark mouth, the tongue lolled out, no glow), the
# winded squint, ears hanging limp down the cheeks (no longer pinned), the tail drooped.
RECOVER_SKID = L.shifted(
    L.IDLE,
    rump=(27, 47), chest=(48, 40), barrel_rr=(5.5, 6.5),
    belly=(38, 49, 8, 4.0), haunch=(26, 49, 6.5, 6.5), shoulder=(49, 42, 5.5, 7.0),
    near_hind=[(27, 51, 4.5), (35, 56, 3.0), (29, 62, 2.1), (31, 67, 1.3)],
    far_hind=[(32, 51, 3.5), (39, 56, 2.5), (34, 62, 1.9), (36, 67, 1.2)],
    near_front=[(51, 45, 3.6), (58, 52, 2.4), (65, 59, 2.1), (71, 67, 1.3)],
    far_front=[(54, 45, 3.0), (61, 52, 2.2), (68, 59, 1.9), (75, 67, 1.2)],
    **hooves(near_hind=(31, 69), far_hind=(36, 69), near_front=(71, 69), far_front=(75, 69)),
    neck=((51, 38), (60, 38.5), (68, 42)), neck_r=(4.8, 3.5, 3.1),
    skull=(71, 43, 4.8, 4.0),
    muzzle=((73, 45), (79, 49)), muzzle_r=(3.2, 2.5), jaw=((71, 50), (76, 53)),
    mouth=((72, 47), (80, 50), (76, 54)), mouth_glow=0.0,
    mouth_line=None, teeth=[("upper", (74, 47))],
    ear_near=((68, 40), (65, 42.5), (62.5, 45.5)), ear_far=((70, 39.5), (67.5, 42), (65.5, 45)), ear_ring=True,
    tail=((20, 44), (15, 52), (-1, 0.5)),
    eye="squint", eye_at=(67, 41), nostril=(79, 47),
    extra=[(77, 53, TONGUE)],                           # the tongue lolled out of the open mouth corner (gate K1)
)

KEYS = [("brace", BRACE, 1), ("scrape", SCRAPE, 2), ("run_contact", RUN_CONTACT, 4), ("recover_skid", RECOVER_SKID, 8)]

# --------------------------------------------------------------------------- the six in-betweens (executor step)
# Recipes of the execution brief with the silhouette-gate corrections B1 (far legs of the run in-betweens) and B6 (the
# K1 tongue on the blown frame). All of them are rig poses over the keys; no new colour, no literal pixel copy.
HEAD = ("neck", "neck_r", "skull", "muzzle", "jaw", "ear_near", "ear_far")
HIND = ("near_hind", "far_hind", "near_hind_hoof", "far_hind_hoof")
BODY_NO_HIND = tuple(k for k in L.INBETWEEN_BODY if k not in HIND)

# 0: body and head halfway between the idle and the brace (hind legs already braced): the head on its way down.
BRACE_IN = L.shifted(BRACE, **L.mean_pose(L.IDLE, BRACE, BODY_NO_HIND + HEAD),
                     teeth=[("clench", (72, 31))], eye_at=(64, 26), nostril=(77, 28), mouth=None, mouth_line=None,
                     eye="angry", ear_ring=True, extra=[(58, 28, ["K"])])
# 3: the brace 1 px lower, head drawn back 2 px: the gather before the launch.
COIL = L.shifted(BRACE, **L.move_body(BRACE, 1), **L.move_head(BRACE, dx=-2, dy=1),
                 tail=((21, 38), (12, 31), (-0.5, -1)), extra=[(56, 44, ["s", "ss", ".s"])])
# 5: near fore vertical under the shoulder, near hind swinging under; the far hind toes off (gate B1).
RUN_GATHER = L.shifted(
    RUN_CONTACT,
    near_front=[(54, 48, 3.6), (56, 54, 2.4), (57, 60, 2.1), (58, 67, 1.3)],
    far_front=[(56, 48, 3.0), (61, 53, 2.2), (65, 57, 1.9), (67, 63, 1.2)],
    near_hind=[(30, 45, 4.5), (36, 51, 3.0), (33, 57, 2.1), (38, 62, 1.3)],
    far_hind=[(35, 45, 3.5), (36, 52, 2.5), (30, 59, 1.9), (26, 67, 1.2)],
    **hooves(near_front=(58, 69), far_front=(67, 65), near_hind=(39, 64), far_hind=(25, 69)),
)
# 6: near hind planted under the barrel, far hind swinging forward, both fores folded up (gate B1).
RUN_PUSH = L.shifted(
    RUN_CONTACT, rump=(30, 42), chest=(52, 42), belly=(43, 47, 8, 3.8), haunch=(29, 43, 6.5, 6.5),
    shoulder=(53, 44, 5.5, 7.0),
    near_hind=[(30, 46, 4.5), (36, 52, 3.0), (34, 59, 2.1), (38, 67, 1.3)],
    far_hind=[(35, 46, 3.5), (38, 52, 2.5), (34, 58, 1.9), (31, 63, 1.2)],
    near_front=[(54, 47, 3.6), (59, 52, 2.4), (61, 58, 2.1), (58, 62, 1.3)],
    far_front=[(56, 47, 3.0), (60, 52, 2.2), (64, 56, 1.9), (64, 61, 1.2)],
    **hooves(near_hind=(38, 69), far_hind=(31, 65), near_front=(57, 64), far_front=(65, 63)),
    **L.move_head(RUN_CONTACT, dy=-1),
)
# 7: far fore striking under the chest, near fore reaching, near hind kicked back (gate B1).
RUN_REACH = L.shifted(
    RUN_CONTACT, rump=(30, 41), chest=(52, 42), shoulder=(53, 44, 5.5, 7.0),
    near_hind=[(30, 45, 4.5), (26, 51, 3.0), (20, 56, 2.1), (15, 61, 1.3)],
    far_hind=[(35, 45, 3.5), (40, 51, 2.5), (38, 57, 1.9), (41, 62, 1.2)],
    near_front=[(54, 47, 3.6), (61, 51, 2.4), (67, 55, 2.1), (71, 61, 1.3)],
    far_front=[(56, 47, 3.0), (60, 53, 2.2), (62, 59, 1.9), (63, 67, 1.2)],
    **hooves(near_hind=(13, 63), far_hind=(42, 64), near_front=(72, 63), far_front=(63, 69)),
    **L.move_head(RUN_CONTACT, dx=1, dy=-1),
    extra=[(31, 51, ["s"]), (30, 52, ["s"]), (29, 53, ["s"])],
)
# 9: the stop is over: legs back under, the head hanging 2 px lower, the K1 tongue out, the squint (gate B6).
RECOVER_BLOWN = L.shifted(
    RECOVER_SKID, rump=(28, 45), chest=(49, 41), belly=(39, 48, 8, 4.0), haunch=(27, 47, 6.5, 6.5),
    shoulder=(50, 43, 5.5, 7.0),
    near_front=[(51, 46, 3.6), (55, 53, 2.4), (58, 60, 2.1), (60, 67, 1.3)],
    far_front=[(54, 46, 3.0), (59, 53, 2.2), (63, 60, 1.9), (65, 67, 1.2)],
    near_hind=[(28, 49, 4.5), (35, 55, 3.0), (29, 61, 2.1), (30, 67, 1.3)],
    far_hind=[(33, 49, 3.5), (39, 55, 2.5), (34, 61, 1.9), (35, 67, 1.2)],
    **hooves(near_front=(60, 69), far_front=(65, 69), near_hind=(30, 69), far_hind=(35, 69)),
    **L.move_head(RECOVER_SKID, dx=-1, dy=2),
    extra=[(76, 55, TONGUE)],
)

# The ten frames of the rush atlas, in atlas order: (name, pose, tag, duration ms for the preview).
ATLAS = [("brace_in", BRACE_IN), ("brace", BRACE), ("scrape", SCRAPE), ("coil", COIL),
         ("run_contact", RUN_CONTACT), ("run_gather", RUN_GATHER), ("run_push", RUN_PUSH),
         ("run_reach", RUN_REACH), ("recover_skid", RECOVER_SKID), ("recover_blown", RECOVER_BLOWN)]
ATLAS_TAGS = [("rush_brace", 0, 3), ("rush_run", 4, 7), ("rush_recover", 8, 9)]
ATLAS_MS = [67, 67, 67, 67, 50, 50, 50, 50, 167, 500]    # 4 ticks, 3 ticks, 10 ticks, a review hold
ATLAS_GRID = RUSH_DIR / "llama_rush_h64.grid.json"
ATLAS_ASE = RUSH_DIR / "llama_rush_h64.ase"


def render(pose: dict) -> list[str]:
    c = L.Canvas(*CANVAS)
    saved = L.EYES
    L.EYES = EYES                    # the rig looks the eye stamp up by name
    try:
        L.llama(c, pose)
    finally:
        L.EYES = saved
    return c.rows()


# --------------------------------------------------------------------------- grid io
def palette_json():
    return L.palette_json()


def keys_grid() -> dict:
    frames = [{"duration_ms": 100, "cels": {"body": render(pose)}} for _n, pose, _i in KEYS]
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": i, "to": i, "direction": "forward"} for i, (n, _p, _a) in enumerate(KEYS)],
    }


def atlas_grid() -> dict:
    frames = [{"duration_ms": ATLAS_MS[i], "cels": {"body": render(pose)}} for i, (_n, pose) in enumerate(ATLAS)]
    return {
        "format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT),
        "palette": palette_json(), "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in ATLAS_TAGS],
    }


def digest(cels: dict) -> str:
    """The toolkit's V09 frame digest (``scripts/pixelart/validators.frame_digest``) of the ``body`` layer."""
    payload = {"body": cels.get("body")}
    return hashlib.sha256(json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def fixture(grid: dict) -> dict:
    return {"asset_id": "llama-03-rush", "format": "pixelart-keys-v1",
            "keys": {str(a): {"sha256": digest(grid["frames"][i]["cels"])} for i, (_n, _p, a) in enumerate(KEYS)}}


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def report(rows: list[str], label: str) -> dict:
    import pixel_grid_to_ase as pg
    flat = [list(r) for r in rows]
    orphans, isolated = pg._stray_pixels(flat, L.ACCENTS)
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    used = sorted({ch for r in rows for ch in r} - {"."})
    glow = sum(ch in GLOW for r in rows for ch in r)
    stats = {"top": min(ys), "bottom": max(ys), "left": min(xs), "right": max(xs), "orphans": len(orphans),
             "isolated": len(isolated), "glow": glow, "keys": "".join(used)}
    print(f"{label}: bbox x {min(xs)}-{max(xs)} y {min(ys)}-{max(ys)} (h {max(ys) - min(ys) + 1}, "
          f"w {max(xs) - min(xs) + 1}) orphans {len(orphans)} isolated {len(isolated)} glow {glow} keys {''.join(used)}")
    return stats


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "atlas", "gate", "evidence"))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.mode == "gate":
        import llama03_rush_gate as gate
        for path in gate.write_all(keys_grid()):
            print(f"wrote {path.relative_to(ROOT)}")
        return 0
    if a.mode == "evidence":
        import llama03_rush_gate as gate
        for path in gate.write_atlas_evidence(atlas_grid()) + [gate.write_sums()]:
            print(f"wrote {path.relative_to(ROOT)}")
        return 0
    if a.mode == "atlas":
        grid = atlas_grid()
        for i, (name, _p) in enumerate(ATLAS):
            report(grid["frames"][i]["cels"]["body"], f"frame {i} {name:13s}")
        text = dump(grid)
        if a.check:
            ok = ATLAS_GRID.is_file() and ATLAS_GRID.read_text() == text
            print("up to date" if ok else f"differs: {ATLAS_GRID.name}")
            return 0 if ok else 1
        ATLAS_GRID.write_text(text, encoding="utf-8")
        print(f"wrote {ATLAS_GRID.relative_to(ROOT)}")
        return 0
    grid = keys_grid()
    for i, (name, _p, atlas) in enumerate(KEYS):
        report(grid["frames"][i]["cels"]["body"], f"key {name:13s} (atlas {atlas})")
    text, fix = dump(grid), json.dumps(fixture(grid), indent=2, sort_keys=True) + "\n"
    if a.check:
        bad = [p.name for p, t in ((KEYS_GRID, text), (FIXTURE, fix)) if not p.is_file() or p.read_text() != t]
        print("up to date" if not bad else f"differs: {bad}")
        return 1 if bad else 0
    RUSH_DIR.mkdir(parents=True, exist_ok=True)
    KEYS_GRID.write_text(text, encoding="utf-8")
    FIXTURE.write_text(fix, encoding="utf-8")
    print(f"wrote {KEYS_GRID.relative_to(ROOT)}\nwrote {FIXTURE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
