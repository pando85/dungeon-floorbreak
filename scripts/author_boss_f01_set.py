#!/usr/bin/env python3
"""BOSS-F01 production set: the 16-frame encounter atlas of the Floor 01 boss (``brute_boss_f01``, fan layer THE JUICER).

Usage:

    python3 scripts/author_boss_f01_set.py frames [--check]

``frames`` rebuilds ``assets/source/enemies/brute_boss_f01/brute_boss_f01.grid.json`` (pixel-grid-v1, layer ``body``).
The six top-tier keys (``scripts/author_boss_f01_keys.py``: K-A charge_tell, K-B charge_run, K-B2 charge_run_b, K-C
crash_stun, K-D phase_roar, K-E pinned) are rebuilt here from the very same pose rules and are asserted pixel-identical
to the committed key grid. The nine in-betweens and the face swap are pose rules on the JUICE-01 rig
(``scripts/author_juice01.py``): tweens of the keys with the family's feet-on-contact-row, thigh/shin and fist rules.
The JUICE-01 base set (idle, walk, pound, hurt, down) is imported, not redrawn: it stays in ``juice-01``.

No raster image-model output, no tracing, no resampling. Presentation-only art tooling; nothing here is wired into the
game. Design spec: ``docs/02-art/floor-01-boss-design.md`` sections 8 and 9.
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_boss_f01_keys as bk  # noqa: E402  (the six keys, the dazed face)
import author_juice01 as jc  # noqa: E402
from author_juice01 import L, POSES, lerp, open_hand, ragged_loin  # noqa: E402

OUT_DIR = ROOT / "assets/source/enemies/brute_boss_f01"
GRID = OUT_DIR / "brute_boss_f01.grid.json"
KEY_GRID = bk.KEYS_GRID
MANIFEST = ROOT / "assets/manifests/boss-f01.json"
TICK_HZ = 60
HOLD_MS = 100

for _name, _pose in bk.NEW.items():
    POSES[_name] = _pose
IDLE, HURT = POSES["idle0"], POSES["hurt12"]


def tween(a: dict, b: dict, t: float, **over) -> dict:
    """``b`` with every numeric part blended toward ``a`` (t 0 = a, 1 = b); face, hair and flags come from ``b``."""
    p = copy.deepcopy(b)
    for k in a:
        if k in b and k not in ("face", "hair", "jaw_drop", "tilt", "no_beads"):
            p[k] = lerp(a[k], b[k], t)
    p["tilt"] = lerp(a.get("tilt", -2.0), b.get("tilt", -2.0), t)
    p.update(over)
    return p


def feet(p: dict, near, far, calf=True) -> dict:
    p["near_legs"] = [L(near, p["near_legs"][0]["r"])]
    p["far_legs"] = [L(far, p["far_legs"][0]["r"])]
    if calf:
        jc.set_calf(p)
    return p


# ------------------------------------------------------------------ the nine in-betweens and the face swap
# charge tell (K-A is the key): the weight drops from idle into the lineman crouch in two steps of 6 ticks
POSES["tell_a"] = tween(IDLE, POSES["charge_tell"], 0.35, face="growl", hair={"sweep": -0.1})
POSES["tell_b"] = tween(IDLE, POSES["charge_tell"], 0.7, face="growl", hair={"sweep": -0.15})

# charge run passing frames: the pushing foot has left the floor and swings under the hip while the other foot lands
POSES["run_pass_a"] = tween(POSES["charge_run"], POSES["charge_run_b"], 0.45)
POSES["run_pass_b"] = tween(POSES["charge_run_b"], POSES["charge_run"], 0.45)
# pass_a: the far leg lands under the hip with a full foot block (heel on the contact row, toe forward); the near leg
# swings through with the knee forward and a hanging shin and foot (the K-B2 near-leg shape). pass_b is the mirror contact.
feet(POSES["run_pass_a"], [(-4, 39), (18, 31), (10, 12), (20, 7)], [(-3, 39), (7, 23), (3, 6), (14, 1.5)])
feet(POSES["run_pass_b"], [(-3, 40), (7, 24), (3, 6), (14, 1.5)], [(-4, 40), (17, 32), (9, 14), (19, 9)])
POSES["run_pass_a"]["hair"] = {"sweep": 0.9}
POSES["run_pass_b"]["hair"] = {"sweep": 0.7}

# crash stun (K-C is the key): the ram front meets the wall (3 ticks), then the rebound sits him down (3 ticks).
# crash_hit: an impact squash. The hump and shoulder are flattened against the wall line (pivot + 82), the head is jammed
# down into the shoulders with drool flung forward, the far fist is splayed flat on the wall at hump height, the near fist is
# flung back low, the knees buckle and both feet stay on the contact row. About 84 px tall, lower than the run key.
p = copy.deepcopy(POSES["charge_run"])
p["torso"] = L([(-8, 34), (8, 52), (22, 64)], [14.0, 15.0, 13.0])
p["blobs"] = {"glute": (-16, 36, 12, 11), "lat": (4, 56, 13, 12), "gut": (10, 42, 13, 12), "pec": (26, 56, 12, 10),
              "traps": (20, 71, 13, 10)}
p["far_arms"] = [L([(28, 62), (50, 62), (66, 60)], [8.0, 7.0, 6.5])]
p["far_fist"] = (74, 60, 9.5, 90)
p["near_arm"] = L([(12, 62), (-2, 48), (-12, 36)], [10.5, 9.0, 8.0])
p["fist"] = (-17, 31, 11.0, 200)
jc.fit_arm_blobs(p)
p["head_at"] = (40, 62); p["tilt"] = 2.0; p["face"] = "pain"; p["jaw_drop"] = True; p["hair"] = {"sweep": -0.9}
feet(p, [(-6, 34), (8, 19), (-4, 6), (7, 1.5)], [(-9, 34), (16, 23), (10, 6), (21, 1.5)])
p["loin"] = ragged_loin(-18, 8, 36, 16); p["waist_chain"] = [(-24, 38), (-12, 32), (0, 31), (10, 35)]
POSES["crash_hit"] = p

# crash_slump: rebounding backward, rump just off the floor, both arms flung back, dazed face. The near leg is thrust
# forward nearly straight with its heel on the contact row and the toe up; the far knee is up with a flat foot.
p = copy.deepcopy(POSES["crash_stun"])
p["torso"] = L([(-8, 18), (-12, 38), (-12, 58)], [14.0, 15.0, 13.0])
p["blobs"] = {"glute": (-14, 17, 12, 10), "lat": (-18, 42, 12, 12), "gut": (2, 30, 13, 12), "pec": (-2, 50, 12, 10),
              "traps": (-14, 64, 13, 10)}
p["near_arm"] = L([(-6, 56), (-20, 62), (-34, 66)], [10.5, 9.0, 8.0])
p["fist"] = (-41, 68, 11.0, 150)
p["far_arms"] = [L([(-14, 60), (-28, 70), (-38, 77)], [8.0, 7.0, 6.5])]
p["far_fist"] = (-43, 82, 9.0, 120)
jc.fit_arm_blobs(p)
p["head_at"] = (10, 66); p["tilt"] = 2.0; p["face"] = "dazed"; p["jaw_drop"] = False
p["hair"] = {"turn": -10, "sweep": -0.6}
feet(p, [(0, 17), (21, 15), (37, 6), (47, 9)], [(-2, 18), (12, 32), (17, 7), (28, 1.5)])
p["loin"] = ragged_loin(-14, 10, 22, 12); p["waist_chain"] = [(-20, 26), (-8, 22), (4, 21), (14, 24)]
POSES["crash_slump"] = p

# phase roar (K-D is the key): roar_in gathers the crab flex from idle (6 ticks); roar_flex holds the squeezed peak.
POSES["roar_in"] = tween(IDLE, POSES["phase_roar"], 0.55, face="roar", jaw_drop=True, hair={"turn": 2, "sweep": 0.2})
p = copy.deepcopy(POSES["phase_roar"])
jc.shift(p, ["torso", "head_at", "arm_blobs", "near_arm", "far_arms"], 0, 1)
p["far_arms"] = [L([(-8, 79), (-34, 68), (-14, 53)], [8.0, 7.0, 6.5])]
p["far_fist"] = (-6, 51, 9.0, 0)
p["near_arm"] = L([(12, 79), (44, 67), (26, 51)], [10.5, 9.0, 8.0])
p["arm_blobs"] = {"deltoid": (12, 81, 12, 11.5), "bicep": (29, 73, 8, 9), "forearm": (35, 58, 8.5, 9)}
p["fist"] = (18, 47, 11.0, 200)
p["head_at"] = (31, 88); p["hair"] = {"turn": 6, "sweep": 0.5}
POSES["roar_flex"] = p

# pinned (K-E is the key): pin_squash is the smash beat as the stack lands (6 ticks): flatter than K-E, the trunk about 22 px
# thick with the level back at the K-E line, limbs splayed wider (K-E settles from this overshoot), head pressed to the floor,
# the far hand flat, both legs ending in feet.
p = copy.deepcopy(POSES["pinned"])
p["torso"] = L([(-30, 10), (-8, 11), (14, 10)], [11.0, 11.5, 11.0])
p["blobs"] = {"glute": (-31, 12, 11, 9), "lat": (-10, 13, 13, 7), "gut": (-6, 6, 13, 6), "pec": (13, 8, 11, 7), "traps": (17, 14, 10, 6)}
p["far_legs"] = [L([(-28, 12), (-43, 15), (-52, 19), (-47, 22)], [10.0, 7.5, 5.0, 3.2])]
p["near_legs"] = [L([(-28, 9), (-44, 8), (-58, 5), (-65, 1.5)], [11.0, 8.5, 5.5, 3.4])]
p["calf"] = (-49, 6, 4.0, 5.0)
p["far_arms"] = [L([(10, 11), (2, 13), (-6, 15)], [8.0, 7.0, 6.5])]
p["far_hand"] = open_hand(-11, 16, 4.0, 95)
p["near_arm"] = L([(22, 10), (40, 5), (56, 4)], [10.5, 9.0, 8.0])
p["arm_blobs"] = {"deltoid": (22, 11, 11, 8), "bicep": (32, 8, 8, 6), "forearm": (49, 5, 8, 5)}
p["hand"] = open_hand(64, 5, 5.0, -5)
p["head_at"] = (40, 6); p["tilt"] = 1.0; p["face"] = "pain"; p["jaw_drop"] = True; p["hair"] = {"turn": 40, "sweep": -0.4}
p["loin"] = ragged_loin(-40, -20, 20, 7); p["waist_chain"] = [(-38, 20), (-30, 22), (-22, 20)]
POSES["pin_squash"] = p
# face swap for the lethal rack crush: K-E unchanged but for the dead face (crossed eye, slack mouth, tongue out)
p = copy.deepcopy(POSES["pinned"])
p["face"] = "dead"; p["jaw_drop"] = False
POSES["pinned_dead"] = p

# ------------------------------------------------------------------ the atlas: (frame name, tag, pose, ticks, author)
# ``None`` ticks is a state-driven hold (the game holds the frame until the state ends; the preview shows HOLD_MS).
FRAMES = [
    ("charge_tell_a", "charge_tell", "tell_a", 6, "in-between"),
    ("charge_tell_b", "charge_tell", "tell_b", 6, "in-between"),
    ("charge_tell_hold", "charge_tell", "charge_tell", None, "key K-A"),
    ("charge_run_contact_a", "charge_run", "charge_run", 5, "key K-B"),
    ("charge_run_pass_a", "charge_run", "run_pass_a", 4, "in-between"),
    ("charge_run_contact_b", "charge_run", "charge_run_b", 5, "key K-B2"),
    ("charge_run_pass_b", "charge_run", "run_pass_b", 4, "in-between"),
    ("crash_hit", "crash_stun", "crash_hit", 3, "in-between"),
    ("crash_slump", "crash_stun", "crash_slump", 3, "in-between"),
    ("crash_stun_hold", "crash_stun", "crash_stun", None, "key K-C"),
    ("phase_roar_in", "phase_roar", "roar_in", 6, "in-between"),
    ("phase_roar_hold", "phase_roar", "phase_roar", 24, "key K-D"),
    ("phase_roar_flex", "phase_roar", "roar_flex", 30, "in-between"),
    ("pinned_squash", "pinned", "pin_squash", 6, "in-between"),
    ("pinned_hold", "pinned", "pinned", None, "key K-E"),
    ("pinned_dead", "pinned_dead", "pinned_dead", None, "face swap of K-E"),
]
KEY_FRAMES = [i for i, f in enumerate(FRAMES) if f[4].startswith("key")]
TAGS: list[tuple[str, list[tuple[int, int | None]]]] = []
for _i, (_n, _tag, _p, _t, _au) in enumerate(FRAMES):
    if TAGS and TAGS[-1][0] == _tag:
        TAGS[-1][1].append((_i, _t))
    else:
        TAGS.append((_tag, [(_i, _t)]))
PLAYBACK = {"charge_tell": "hold", "charge_run": "loop", "crash_stun": "hold", "phase_roar": "once",
            "pinned": "hold", "pinned_dead": "hold"}
# Spec section 8 opaque heights of the six keys (acceptance target 3: within 4 px)
KEY_HEIGHT = {"charge_tell_hold": 80, "charge_run_contact_a": 90, "charge_run_contact_b": 92,
              "crash_stun_hold": 72, "phase_roar_hold": 105, "pinned_hold": 56}


def duration_ms(ticks: int | None) -> int:
    return HOLD_MS if ticks is None else round(ticks * 1000 / TICK_HZ)


def frames_grid() -> dict:
    rows = [jc.build(POSES[p]) for _n, _t, p, _tk, _a in FRAMES]
    used = {ch for r in rows for ch in "".join(r) if ch != "."}
    return {
        "format": "pixel-grid-v1", "canvas": list(jc.CANVAS), "pivot": list(jc.PIVOT),
        "palette": jc.palette_json(used), "layers": ["body"],
        "frames": [{"duration_ms": duration_ms(f[3]), "cels": {"body": r}} for f, r in zip(FRAMES, rows)],
        "tags": [{"name": name, "from": fr[0][0], "to": fr[-1][0], "direction": "forward"} for name, fr in TAGS],
    }


def config() -> dict:
    """The toolkit config (``pixelart-asset-v1``): the spec's frame table, size and silhouette rules."""
    names = {f[0]: i for i, f in enumerate(FRAMES)}
    bbox = [{"id": f"size-{n}-{h}px", "frames": [names[n]], "min_top": 136 - h - 4, "max_top": 136 - h + 4, "bottom": 135}
            for n, h in KEY_HEIGHT.items()]
    run = [names[n] for n in ("charge_run_contact_a", "charge_run_pass_a", "charge_run_contact_b", "charge_run_pass_b")]
    return {
        "format": "pixelart-asset-v1", "asset_id": "boss-f01",
        "title": "BOSS-F01 brute_boss_f01 (THE JUICER) Floor 01 boss encounter set: six frozen keys, nine in-betweens, one face swap",
        "grid": "assets/source/enemies/brute_boss_f01/brute_boss_f01.grid.json", "manifest": "assets/manifests/boss-f01.json",
        "frame_size": list(jc.CANVAS), "pivot": list(jc.PIVOT), "contact_row": 135, "tick_hz": TICK_HZ, "hold_ms": HOLD_MS,
        "facing": "right",
        "tags": {name: {"playback": PLAYBACK[name], "contact": "grounded", "frames": [[i, t] for i, t in fr]}
                 for name, fr in TAGS},
        "rules": {
            "max_colors": 20, "max_isolated_per_frame": 0, "max_orphans_per_frame": 40, "canvas_margin": [1, 1, 1, 0],
            "unique_frames": True, "manifest_tick_rows": "boss.frames", "manifest_tick_hz": "boss.tick_hz",
            "foot_drift": {"max_px": 30, "tags": ["charge_run"]},
            "frozen_keys": {"fixture": "tests/fixtures/boss-f01_keys.json", "frames": KEY_FRAMES, "layers": ["body"]},
            "bbox": bbox + [
                {"id": "R-roar-widest-key-fists-low", "frames": [names["phase_roar_hold"]], "min_width": 70},
                {"id": "R-run-lower-than-roar", "frames": run, "min_top": 136 - 105 + 1},
                {"id": "R-pinned-stays-flat", "frames": [names["pinned_hold"], names["pinned_dead"]], "min_top": 136 - 56 - 4},
                {"id": "B-04-crash-hit-squash-82-86px", "frames": [names["crash_hit"]], "min_top": 136 - 86, "max_top": 136 - 82},
                {"id": "B-05-pin-squash-flatter-than-key", "frames": [names["pinned_squash"]], "min_top": 136 - 48},
                {"id": "R-crash-stun-lower-than-idle", "frames": [names["crash_slump"], names["crash_stun_hold"]], "min_top": 36}],
            "key_counts": [
                {"id": "R1-boils-visible", "frames": list(range(len(FRAMES))), "key": "b", "min": 15},
                {"id": "R2-iron-visible", "frames": list(range(len(FRAMES))), "key": "j", "min": 10},
                {"id": "R3-roar-mouth-open", "frames": [names["phase_roar_hold"], names["phase_roar_flex"]], "key": "o", "min": 60},
                {"id": "R4-dead-face-tongue", "frames": [names["pinned_dead"]], "key": "q", "min": 3},
                {"id": "R5-dazed-and-dead-no-red-pupil-ring", "frames": [names["pinned_dead"]], "key": "e", "max": 0}],
        },
        "review": {"strip_tags": ["charge_tell", "charge_run", "crash_stun"]},
        "scene": {"actors": [{"frame": names["charge_tell_hold"], "x": 560}, {"frame": names["charge_run_contact_a"], "x": 685},
                             {"frame": names["crash_stun_hold"], "x": 790}, {"frame": names["phase_roar_hold"], "x": 905}]},
    }


def check_keys_unchanged() -> None:
    """The six top-tier keys must be pixel-identical to the committed key grid."""
    committed = json.loads(KEY_GRID.read_text(encoding="utf-8"))["frames"]
    key_rows = [(f, jc.build(POSES[f[2]])) for f in FRAMES if f[4].startswith("key")]
    order = {p: i for i, p in enumerate(bk.ORDER)}
    for f, rows in key_rows:
        want = committed[order[f[2]]]["cels"]["body"]
        if rows != want:
            raise SystemExit(f"key {f[0]} differs from {KEY_GRID.relative_to(ROOT)}")


def manifest() -> dict:
    from scripts.pixelart.validators import frame_digest
    grid = frames_grid()
    w, h = jc.CANVAS
    return {
        "schema_version": 1, "id": "brute_boss_f01_encounter_h100_candidate_v0", "type": "character_animation",
        "source_path": "assets/source/enemies/brute_boss_f01/brute_boss_f01.ase",
        "grid_source": "assets/source/enemies/brute_boss_f01/brute_boss_f01.grid.json",
        "output_atlas": "generated/art/brute_boss_f01.png", "output_metadata": "generated/art/brute_boss_f01.json",
        "palette_id": "trog-family-v0+juicer", "frame_size": [w, h], "pivot": list(jc.PIVOT),
        "animation_tags": [name for name, _f in TAGS], "required_alpha": True,
        "owner": "AI-authored boss animation candidate (fan-concept layer: Floor 1 boss THE JUICER; runtime role IP-neutral heavy boss brute_boss_f01)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("BOSS-F01 encounter set: the six top-tier keys K-A..K-E (scripts/author_boss_f01_keys.py, frozen in "
                       "tests/fixtures/boss-f01_keys.json), nine in-betweens and the K-E dead-face swap, rasterized from pose "
                       "rules on the JUICE-01 rig by scripts/author_boss_f01_set.py; 16 frames, layer body only. The JUICE-01 base "
                       "set (idle, walk, pound, hurt, down) stays in juice-01"),
            "generator": "designed by Claude Opus keys, executed by Claude Sonnet; no AI concept pixels copied. Silhouette gate pack: "
                         "written, no verdict. Independent review: pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "boss": {
            "status": "16 frames drawn; art-director review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/floor-01-boss-design.md", "base_set": "assets/manifests/juice-01.json",
            "data_id": "brute_boss_f01", "tick_hz": TICK_HZ, "hold_preview_ms": HOLD_MS, "playback": PLAYBACK,
            "key_atlas_index": {f[0]: i for i, f in enumerate(FRAMES) if f[4].startswith("key")},
            "key_fixture": "tests/fixtures/boss-f01_keys.json",
            "frames": [{"index": i, "name": f[0], "tag": f[1], "ticks": f[3], "duration_ms": duration_ms(f[3]), "author": f[4],
                        "pivot": list(jc.PIVOT), "rect": {"x": i * w, "y": 0, "w": w, "h": h},
                        "sha256": frame_digest(grid["frames"][i]["cels"], ["body"])} for i, f in enumerate(FRAMES)],
        },
    }


def dump(data: dict, indent: int = 2) -> str:
    return json.dumps(data, indent=indent) + "\n"


def targets() -> list[tuple[Path, str]]:
    return [(GRID, dump(frames_grid(), 1)), (MANIFEST, dump(manifest())),
            (OUT_DIR / "brute_boss_f01.pixelart.json", dump(config()))]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["frames"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    sys.path.insert(0, str(ROOT))
    check_keys_unchanged()
    status = 0
    for path, text in targets():
        if args.check:
            same = path.is_file() and path.read_text(encoding="utf-8") == text
            print(f"{path.name} up to date" if same else f"{path.name} differs from the generator")
            status |= 0 if same else 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    return status


if __name__ == "__main__":
    sys.exit(main())
