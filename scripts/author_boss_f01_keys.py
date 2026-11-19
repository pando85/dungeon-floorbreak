#!/usr/bin/env python3
"""BOSS-F01 design keys: the Floor 01 boss encounter states that the JUICE-01 readability set does not cover.

Usage:

    python3 scripts/author_boss_f01_keys.py keys [--check]
    python3 scripts/author_boss_f01_keys.py previews

``keys`` writes ``assets/source/enemies/brute_boss_f01/keys/brute_boss_f01_keys.grid.json`` (pixel-grid-v1, layer
``body``, one frame per key) and ``brute_boss_f01_emblem.grid.json`` (the 18 x 18 boss-strip emblem). ``previews``
writes the review PNGs to ``docs/evidence/floor01-boss/`` (needs ``bash scripts/export_assets.sh`` for the lit floor).

The five body keys reuse the JUICE-01 construction unchanged (``scripts/author_juice01.py``: canvas 160 x 136, pivot
(72, 135), the 19-colour ``trog-family-v0+juicer`` palette, part order, boils, veins, grime, hair and faces); only the
pose rules below are new. Design spec: ``docs/02-art/floor-01-boss-design.md`` section 8. Presentation-only art
tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import author_juice01 as jc  # noqa: E402  (the Juicer rig, palette and rasterizer)
from author_juice01 import L, POSES, open_hand, ragged_loin  # noqa: E402

OUT_DIR = ROOT / "assets/source/enemies/brute_boss_f01/keys"
KEYS_GRID = OUT_DIR / "brute_boss_f01_keys.grid.json"
EMBLEM_GRID = OUT_DIR / "brute_boss_f01_emblem.grid.json"
EVIDENCE = ROOT / "docs/evidence/floor01-boss"

# A dazed face (crash stun): a heavy lid half over the eye (no red pupil), a slack open mouth, tongue out of the
# front corner, one drool strand. Same 15-column stamp rules as the family faces.
jc.FACES["dazed"] = ["...............",
                     "....KKKKKKK....",
                     "....KKKKKKK....",
                     ".....ewwwKK....",
                     "...............",
                     "...............",
                     "...tTttTttT....",
                     "..ooooooooooo..",
                     "..ooooooooqqq..",
                     "...tT..tT.qqq..",
                     "..........qq...",
                     ".....w....q....",
                     ".....w........."]

# ------------------------------------------------------------------ the five new keys (design units: x forward
# from the pivot column, y up from the contact row; authored facing right like JUICE-01)
NEW: dict[str, dict] = {}

# K-A charge_tell: a lineman crouch. Weight forward on the bent near leg, the far foot scraping back, the far fist's
# knuckles on the floor ahead, the near fist pulled back past the hip, the head dropped below the hump and pushed
# forward. Lower and longer than every JUICE-01 key: "he is about to run", never "the hammer is up".
NEW["charge_tell"] = {
    "far_legs": [L([(-12, 30), (-26, 18), (-40, 7), (-48, 2.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(22, 58), (40, 38), (52, 18)], [8.0, 7.0, 6.5])],
    "far_fist": (56, 9, 9.0, -80),
    "torso": L([(-8, 32), (4, 50), (16, 62)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-16, 34, 12, 11), "lat": (0, 56, 13, 12), "gut": (10, 36, 13, 12), "pec": (22, 48, 12, 10),
              "traps": (12, 68, 13, 10)},
    "near_legs": [L([(2, 30), (20, 22), (18, 5), (28, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (19, 13, 4.5, 6.0),
    "loin": ragged_loin(-16, 10, 31, 18),
    "waist_chain": [(-22, 34), (-10, 29), (2, 28), (14, 31)],
    "head_at": (38, 54), "tilt": -3.0, "face": "growl", "hair": {"sweep": -0.2},
    "near_arm": L([(10, 58), (-6, 56), (-22, 58)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (9, 60, 12, 11), "bicep": (1, 56, 8, 9), "forearm": (-15, 57, 8.5, 8.5)},
    "fist": (-31, 59, 11.0, 175),
}

# K-B charge_run: the ram. Trunk pitched about 40 degrees forward, the hump and the near shoulder leading, the head
# tucked under it, the far fist punched ahead like a battering ram, the near fist pumping back; the far knee driven up
# (airborne), the near leg straight behind pushing off the contact row. Hair streams back.
NEW["charge_run"] = {
    "far_legs": [L([(-2, 38), (16, 32), (12, 16), (22, 11)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(30, 66), (48, 56), (62, 50)], [8.0, 7.0, 6.5])],
    "far_fist": (70, 49, 9.5, -5),
    "torso": L([(-8, 40), (10, 58), (26, 70)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-16, 42, 12, 11), "lat": (4, 64, 13, 12), "gut": (10, 46, 13, 12), "pec": (30, 60, 12, 10),
              "traps": (22, 78, 13, 10)},
    "near_legs": [L([(-6, 38), (-18, 24), (-32, 9), (-40, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (-25, 16, 4.5, 6.0),
    "loin": ragged_loin(-18, 8, 41, 17),
    "waist_chain": [(-24, 44), (-12, 38), (0, 37), (10, 41)],
    "head_at": (46, 64), "tilt": -3.0, "face": "growl", "hair": {"sweep": 1.0},
    "near_arm": L([(22, 70), (8, 56), (-2, 44)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (22, 72, 12, 11), "bicep": (14, 62, 8, 9), "forearm": (3, 50, 8.5, 8.5)},
    "fist": (-8, 38, 11.0, 220),
}

# K-C crash_stun: rebounded off the wall and sat down hard. Rump on the floor, the near leg straight out, the far knee
# up, the trunk slumped back, the head lolled forward and down with the dazed face, both fists dropped on the floor.
# About 62 px tall: the lowest standing-up-able pose, the punish window reads at a glance.
NEW["crash_stun"] = {
    "far_legs": [L([(-2, 12), (16, 26), (26, 8), (34, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(-12, 46), (-24, 30), (-30, 14)], [8.0, 7.0, 6.5])],
    "far_fist": (-32, 8, 8.5, 250),
    "torso": L([(-8, 12), (-14, 30), (-16, 46)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-14, 11, 12, 10), "lat": (-20, 32, 12, 12), "gut": (2, 22, 13, 12), "pec": (-2, 38, 12, 10),
              "traps": (-14, 52, 13, 10)},
    "near_legs": [L([(0, 9), (22, 9), (40, 5), (50, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (31, 8, 4.5, 5.0),
    "loin": ragged_loin(-14, 10, 19, 10),
    "waist_chain": [(-20, 20), (-8, 16), (4, 15), (14, 18)],
    "head_at": (14, 47), "tilt": 2.0, "face": "dazed", "hair": {"turn": -18, "sweep": -0.3},
    "near_arm": L([(-8, 42), (-8, 26), (-6, 13)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (-9, 44, 12, 11), "bicep": (-8, 34, 8, 9), "forearm": (-7, 19, 8.5, 8.5)},
    "fist": (-4, 8, 10.5, -70),
}

# K-D phase_roar: the gym brute's "most muscular" crab flex. Feet wide, chest up, both elbows driven out (the near
# elbow forward, the far elbow behind the back line), both fists clenched together in front of the gut, the head
# pushed forward in the full roar. As tall as idle but the widest key: fists LOW, never the overhead pound tell.
NEW["phase_roar"] = {
    "far_legs": [L([(-14, 34), (-28, 18), (-29, 5), (-20, 1.5)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(-8, 78), (-30, 66), (-12, 52)], [8.0, 7.0, 6.5])],
    "far_fist": (-4, 50, 9.0, 0),
    "torso": L([(-6, 38), (-4, 64), (0, 86)], [14.0, 15.0, 13.0]),
    "blobs": {"glute": (-14, 38, 12, 11), "lat": (-14, 70, 14, 14), "gut": (6, 48, 13, 12), "pec": (12, 72, 13, 11),
              "traps": (0, 92, 14, 10)},
    "near_legs": [L([(2, 34), (18, 20), (20, 5), (30, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (19, 12, 4.5, 6.0),
    "loin": ragged_loin(-14, 12, 36, 21),
    "waist_chain": [(-20, 40), (-8, 35), (4, 34), (16, 37)],
    "head_at": (30, 86), "tilt": -1.0, "face": "roar", "jaw_drop": True, "hair": {"turn": 4, "sweep": 0.3},
    "near_arm": L([(12, 78), (40, 66), (24, 50)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (12, 80, 12, 11.5), "bicep": (27, 72, 8, 9), "forearm": (33, 57, 8.5, 9)},
    "fist": (16, 46, 11.0, 200),
}

# K-E pinned: flattened face-down under the dropped plate stack (the stack is a prop drawn on top, not in this
# sprite). Trunk flat on the floor with a level back for the plates (x -24 to +12), the head turned toward the camera
# at the front with the pain face (the lethal crush swaps in the "dead" face), the near arm splayed forward along the
# floor with an open hand, the far hand clawing up behind, the far leg kicking up behind the rump.
NEW["pinned"] = {
    "far_legs": [L([(-28, 14), (-42, 20), (-52, 32), (-50, 40)], [10.0, 7.5, 5.0, 3.2])],
    "far_arms": [L([(8, 18), (0, 30), (-6, 40)], [8.0, 7.0, 6.5])],
    "far_hand": open_hand(-8, 44, 4.0, 100),
    "torso": L([(-30, 12), (-8, 13), (14, 12)], [12.0, 13.0, 12.0]),
    "blobs": {"glute": (-30, 15, 11, 10), "lat": (-10, 16, 13, 8), "gut": (-6, 7, 13, 7), "pec": (12, 9, 11, 8),
              "traps": (16, 18, 10, 7)},
    "near_legs": [L([(-28, 10), (-44, 9), (-56, 4), (-63, 1.5)], [11.0, 8.5, 5.5, 3.4])],
    "calf": (-50, 6, 4.0, 5.0),
    "loin": ragged_loin(-40, -20, 24, 8),
    "waist_chain": [(-38, 22), (-30, 24), (-22, 22)],
    "head_at": (36, 9), "tilt": 1.0, "face": "pain", "jaw_drop": True, "hair": {"turn": 40, "sweep": -0.4},
    "near_arm": L([(20, 12), (36, 6), (50, 4)], [10.5, 9.0, 8.0]),
    "arm_blobs": {"deltoid": (20, 13, 11, 9), "bicep": (28, 9, 8, 7), "forearm": (43, 5, 8, 6)},
    "hand": open_hand(58, 5, 5.0, -5),
}

ORDER = ["charge_tell", "charge_run", "crash_stun", "phase_roar", "pinned"]

# ------------------------------------------------------------------ the boss-strip emblem (18 x 18 inside the
# 20 x 20 well of docs/02-art/hybrid-hud-a-native-spec.md 2.3): the Juicer's head in profile facing right, mop and
# ropes, the swept-back pointed ear, the red-rimmed eye, the roaring mouth with fangs, one boil on the brow.
# Hand-placed pixels in the same 19-colour palette; no scaling of sprite pixels.
EMBLEM = [".....KKKKKKK......",
          "...KKHHIHHIHKK....",
          "..KHIHHIHHIHHHK...",
          ".KHIHlHIHHIHHIHHK.",
          ".KHHIlmHmmllllmHK.",
          "KHIHsmsmmlbqlmmmK.",
          "KHHKssKKKKKKKKmmK.",
          "KIHKsmmewwwKmmmmK.",
          "KHIKdsmmeeKmmmllmK",
          "KHHKdsmmmmmmmmmmmK",
          "KIH.KdtTttTttTttTK",
          "KHH.Kdoooooooooooo",
          "KIH.Kdooqqqqqooooo",
          "KHH.KdtTotTotTotK.",
          ".KH..KdsmmmmmmmK..",
          ".KH...KdsssssdK...",
          "..K....KKddddK....",
          "........KKKK......"]


def grid_for(names: list[str]) -> dict:
    frames = [jc.build(NEW[n]) for n in names]
    used = {ch for f in frames for ch in "".join(f) if ch != "."}
    return {
        "format": "pixel-grid-v1",
        "canvas": list(jc.CANVAS),
        "pivot": list(jc.PIVOT),
        "palette": jc.palette_json(used),
        "layers": ["body"],
        "frames": [{"duration_ms": 100, "cels": {"body": f}} for f in frames],
        "tags": [{"name": n, "from": i, "to": i, "direction": "forward"} for i, n in enumerate(names)],
    }


def emblem_grid() -> dict:
    assert len(EMBLEM) == 18 and all(len(r) == 18 for r in EMBLEM), "emblem must be 18 x 18"
    used = {ch for r in EMBLEM for ch in r if ch != "."}
    return {"format": "pixel-grid-v1", "canvas": [18, 18], "pivot": [0, 0], "palette": jc.palette_json(used),
            "layers": ["key"], "frames": [{"duration_ms": 100, "cels": {"key": list(EMBLEM)}}],
            "tags": [{"name": "emblem", "from": 0, "to": 0, "direction": "forward"}]}


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def write_previews() -> list[Path]:
    """keys-x3.png: the five new keys at 1x on lit Floor 1 brick, enlarged 3x. silhouettes.png: all eleven keys
    (six frozen JUICE-01 keys, then the five new ones) as flat silhouettes, 2x, to check that every state reads apart.
    scale-1x.png: Carl, Donut, crash_stun and charge_run at 1x on the lit floor (640 x 360). emblem-x6.png: the
    emblem in its 20 x 20 well, 6x."""
    sys.path.insert(0, str(ROOT))
    from PIL import Image, ImageDraw
    from scripts.pixelart.common import CARL_X, flip, grid_images, load_grid_file
    from scripts.pixelart.scene import Floor, reference_idle

    EVIDENCE.mkdir(parents=True, exist_ok=True)
    new = grid_images(load_grid_file(KEYS_GRID))
    old_all = grid_images(load_grid_file(jc.GRID))
    old = [old_all[a] for a, *_r in jc.KEYS]
    piv = tuple(jc.PIVOT)
    out = []

    # 1. new keys on the lit floor, 3x (two rows)
    strips = []
    for row in (new[0:3], new[3:5]):
        entries = [(img, piv, CARL_X - 40 + i * 175, 0) for i, img in enumerate(row)]
        floor = Floor()
        lit = floor.render(entries, 1)
        ground = max(floor.ground(x) for _i, _p, x, _l in entries)
        strips.append(lit.crop((0, max(0, ground - 150), lit.width, ground + 6)))
    sheet = Image.new("RGB", (strips[0].width, sum(s.height for s in strips)))
    y = 0
    for s in strips:
        sheet.paste(s, (0, y)); y += s.height
    p = EVIDENCE / "keys-x3.png"
    sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(p); out.append(p)

    # 2. silhouettes of all eleven keys, 2x
    w, h = jc.CANVAS
    names = ["idle", "walk", "tell", "pound", "hurt", "down"] + ORDER
    sil = Image.new("RGB", (w * 6, (h + 14) * 2), (232, 228, 216))
    d = ImageDraw.Draw(sil)
    for i, img in enumerate(old + new):
        x, yy = (i % 6) * w, (i // 6) * (h + 14)
        mask = img.split()[3].point(lambda a: 255 if a else 0)
        sil.paste((30, 22, 36), (x, yy, x + w, yy + h), mask)
        d.text((x + 4, yy + h), names[i], fill=(30, 22, 36))
    p = EVIDENCE / "silhouettes.png"
    sil.resize((sil.width * 2, sil.height * 2), Image.NEAREST).save(p); out.append(p)

    # 3. scale at 1x: Carl, Donut, the boss stunned and charging
    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    run_img, run_piv = flip(new[1], piv)
    entries = [(carl, carl_piv, CARL_X, 0), (donut, donut_piv, CARL_X - 34, 0),
               (new[2], piv, CARL_X + 120, 0), (run_img, run_piv, CARL_X + 300, 0)]
    p = EVIDENCE / "scale-1x.png"
    Floor().render(entries, 1).save(p); out.append(p)

    # 4. emblem in the well (hud_ink border, magenta_dk fill), 6x
    eg = emblem_grid()
    em = grid_images(eg)[0]
    well = Image.new("RGBA", (20, 20), (0x14, 0x0e, 0x1a, 255))
    ImageDraw.Draw(well).rectangle((1, 1, 18, 18), fill=(0x6e, 0x1a, 0x4c, 255))
    well.paste(em, (1, 1), em)
    p = EVIDENCE / "emblem-x6.png"
    well.resize((120, 120), Image.NEAREST).save(p); out.append(p)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("command", choices=["keys", "previews"])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args(argv)
    if args.command == "previews":
        for p in write_previews():
            print(f"wrote {p.relative_to(ROOT)}")
        return 0
    status = 0
    for path, text in ((KEYS_GRID, dump(grid_for(ORDER))), (EMBLEM_GRID, dump(emblem_grid()))):
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
