#!/usr/bin/env python3
"""Deterministic authoring script for ROT-01 (Rot Sticker readability set), first draft.

Writes ``assets/source/characters/rot_sticker/rot_01.grid.json`` (pixel-grid-v1), the matching ``.ase`` via
``scripts/pixel_grid_to_ase.py`` and ``assets/manifests/rot-01.json``. Design spec: ``docs/02-art/rot-01-spec.md``.

Status: the six keys (design-edited) (frames 0, 3, 8, 10, 11, 14) are drawn by rules (an oblique disc: two-band shell, crimped lip, pleated
face, glow core and spikes; direction-neutral shading), parametrised per key in ``KEYS``. The nine in-between slots are copies of the nearest
key until the execution tier replaces them (spec section 8). Nothing is traced or resampled from the concept.

Orientation: authored WALL-MOUNTED, attached to a vertical wall on the LEFT, dome bulging to the right (room side),
creeping toward -y (up the wall). The wall plane is column x = 6 (the pivot column); the pivot is (6, 16).
"""
from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "assets" / "source" / "characters" / "rot_sticker"
GRID = OUT_DIR / "rot_01.grid.json"
ASE = OUT_DIR / "rot_01.ase"
MANIFEST = ROOT / "assets" / "manifests" / "rot-01.json"

W, H = 32, 32
PIVOT = [6, 16]
WALL_X = 6

# Palette `rot-sticker-floor01-v1`: 13 keys used, 7 slots reserved (spec section 6).
PALETTE = [
    # key, name, rgba, accent
    ("K", "outline_soot", "#0d0a10ff", False),
    ("a", "shell_0_deep", "#1c1722ff", False),
    ("b", "shell_1_shade", "#2b2230ff", False),
    ("c", "shell_2_mid", "#3f3243ff", False),
    ("v", "rim_cold", "#6a6682ff", False),
    ("x", "rot_olive", "#4e4a28ff", False),
    ("m", "rib_dead", "#5a1218ff", False),
    ("r", "rib_red", "#a51418ff", True),
    ("p", "spike_tip", "#e2583aff", True),
    ("o", "glow_ember", "#ec3a1cff", True),
    ("g", "glow_hot", "#ff8a3cff", True),
    ("G", "glow_core", "#ffdc8aff", True),
    ("w", "flash_white", "#efe3d3ff", True),
]

IDLE_SPIKES = ((-64, 2), (-30, 1), (-12, 3), (14, 2), (36, 1), (64, 2))  # (angle from +x, length), top to bottom
WALL_ROUND = ((0.6, 1), (0.8, 2))  # (normalised distance along the wall, inset from the wall column)


def render(rx=12.0, ry_up=10.0, ry_dn=10.0, cy=16.0, n=2.3, face_x=0.66, face_shift=1.0, face_ax=4.4, face_ay=7.0,
           face_dy=0.0, pleat=("r", "m"), core=("g", "o"), rc=0.34, lip="c", lip_hi="v", shell=("a", "b"),
           spikes=IDLE_SPIKES, spike_add=0, spike_root="K", spike_tip="r", wall_round=WALL_ROUND, halo=None,
           crack=None, grime=((0, 2), (0, 3), (0, 4), (0, 5), (1, 3), (1, 4)), drip=0, patch=()) -> list[str]:
    """Oblique disc on a wall at the left: dark ringed shell on the wall side, pleated red face toward the room.

    The outline is an oval: only the middle of the wall side touches the wall column (``wall_round``). Shading is
    direction-neutral: it depends only on the distance from the outline and the wall, and is mirror symmetric along the
    wall, so a runtime rotation or mirror never puts a baked light on the wrong edge. Spike lengths, spike spacing and
    the rot stain are deliberately uneven (shape and texture, not light).
    """
    def norm(x, y):
        dy = y + 0.5 - cy
        return (x + 0.5 - WALL_X) / rx, dy / (ry_up if dy < 0 else ry_dn)

    def inset(y):
        v = abs(norm(WALL_X, y)[1])
        return max([0] + [i for t, i in wall_round if v >= t])

    mask = set()
    for y in range(H):
        for x in range(WALL_X + inset(y), W):
            u, v = norm(x, y)
            if abs(u) ** n + abs(v) ** n <= 1.0:
                mask.add((x, y))

    def boundary(p):
        x, y = p
        return any(q not in mask for q in ((x + 1, y), (x, y + 1), (x, y - 1))) or (x > WALL_X and (x - 1, y) not in mask)

    g: dict[tuple[int, int], str] = {}
    fcx, fcy = WALL_X + face_x * rx + face_shift, cy + face_dy

    def fe(p):  # normalised face-ellipse coordinates
        return (p[0] + 0.5 - fcx) / face_ax, (p[1] + 0.5 - fcy) / face_ay

    edge = {p for p in mask if boundary(p)}
    face = {p for p in mask - edge if sum(c * c for c in fe(p)) <= 1.0}
    lips = sorted(p for p in mask - face - edge if any(q in face for q in ((p[0] + 1, p[1]), (p[0], p[1] - 1),
                                                                             (p[0], p[1] + 1))))
    # shell: dark body with one lighter ring 1 px inside the outline (the ringed shell)
    ring = {p for p in mask - face - edge - set(lips)
            if any((p[0] + dx, p[1] + dy) in edge for dx in (-1, 0, 1) for dy in (-1, 0, 1))}
    for p in mask - face:
        g[p] = shell[1] if p in ring else shell[0]
    for p in lips:
        g[p] = lip
    if lips and lip_hi:
        ys = [p[1] for p in lips]
        ends = [p for p in lips if p[1] in (min(ys), min(ys) + 1, max(ys), max(ys) - 1) and p[0] >= fcx - 2]
        for p in ends:
            g[p] = lip_hi
        # cold accent runs on from each lip end along the shell ring: the 3 ring pixels nearest each end
        for end_y in (min(ys), max(ys)):
            near = sorted((p for p in ring if (p[1] - cy) * (end_y - cy) > 0),
                          key=lambda p: (abs(p[0] - fcx) + abs(p[1] - end_y), p[0]))
            for p in near[:3]:
                g[p] = lip_hi
    for p in face:
        a, b = fe(p)
        r_ = math.hypot(a, b)
        if r_ <= rc:
            g[p] = core[0] if r_ <= rc * 0.5 else core[1]
        else:
            sector = int((math.atan2(b, a) + math.pi) / (2 * math.pi) * 10 + 0.5) % 10
            g[p] = pleat[sector % 2]
    for p in edge:
        g[p] = "K"
    for ox, oy in grime or ():  # olive rot stain where the shell meets the wall, off-centre on the low side
        p = (WALL_X + ox, int(cy) + oy)
        if p in mask and p not in edge and p not in face:
            g[p] = "x"
    # spikes: rooted on the outline, uneven lengths and spacing
    for ang, length in spikes:
        length = max(1, length + spike_add)
        a = math.radians(ang)
        dx, dy = math.cos(a) * rx, math.sin(a) * (ry_up if ang < 0 else ry_dn)
        ln = math.hypot(dx, dy)
        dx, dy = dx / ln, dy / ln
        t = 0.0
        while (int(WALL_X + 0.5 + dx * t), int(cy + dy * t)) in mask:
            t += 0.25
        for i in range(length):
            px = int(math.floor(WALL_X + 0.5 + dx * (t + i)))
            py = int(math.floor(cy + dy * (t + i)))
            if (px, py) not in mask and 0 <= px < W and 1 <= py < H - 1:
                g[(px, py)] = spike_tip if i == length - 1 else spike_root
    if halo:  # a 1 px glow ring around the outline, hotter on the face side
        rim = set()
        for (x, y) in mask:
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1), (x + 1, y + 1), (x + 1, y - 1)):
                if q not in mask and q not in g and q[0] > WALL_X and 1 <= q[1] < H - 1:
                    rim.add(q)
        for q in rim:
            g[q] = halo[1] if q[0] + 0.5 >= fcx else halo[0]
    if crack:  # burst: bright fissures from the core through the shell
        for ang in (150, 180, 210, 120, 240):
            a = math.radians(ang)
            for i in range(30):
                p = (int(math.floor(fcx + math.cos(a) * i * 0.5)), int(math.floor(fcy + math.sin(a) * i * 0.5)))
                if p in mask and p not in face:
                    g[p] = crack
    ys = [p[1] for p in mask if p[0] == WALL_X]
    for i in range(drip):  # dud: olive rot running down the wall below the skin
        y = max(ys) + 1 + i
        if y < H - 1:
            g[(WALL_X, y)] = "x"
            if 0 < i < drip - 1:
                g[(WALL_X + 1, y)] = "x"
    for (x, y), c in patch:
        g[(x, y)] = c
    rows = [["."] * W for _ in range(H)]
    for (x, y), c in g.items():
        if c != ".":
            rows[y][x] = c
    return ["".join(r) for r in rows]


# Key poses (spec section 7). Index -> render parameters.
KEYS = {
    # idle: oval disc, dark ringed shell, dim maroon pleated face, ember core, uneven short spikes
    0: dict(pleat=("m", "b")),
    # creep: the leading (upper) half stretches up the wall, the trailing half drags short, face rides up, core dims
    3: dict(rx=11.0, ry_up=11.0, ry_dn=8.0, face_ax=4.0, face_dy=-1.5, pleat=("m", "b"), core=("o", "r")),
    # tell: the face lights up pleat by pleat, hot core, red halo, longer lit spikes; the shell stays dark
    8: dict(rx=13.0, ry_up=11.0, ry_dn=11.0, face_ax=5.0, face_ay=8.0, pleat=("o", "r"), core=("G", "g"), rc=0.4,
            lip="r", spike_add=1, spike_tip="p", spike_root="r", halo=("m", "r")),
    # first burst frame: blown-out face, fissures through the shell, hot halo
    10: dict(rx=14.5, ry_up=12.0, ry_dn=12.0, face_ax=6.0, face_ay=9.0, pleat=("G", "g"), core=("G", "G"), rc=0.5,
             lip="g", lip_hi="G", spike_add=1, spike_tip="G", spike_root="g", halo=("o", "g"), crack="G"),
    # hurt: squashed flat, spikes tucked, white flash on the lip and core
    11: dict(rx=10.0, ry_up=9.5, ry_dn=9.5, face_x=0.62, face_ax=3.6, face_ay=6.0, pleat=("m", "a"), core=("w", "w"),
             lip="w", lip_hi="w", spike_add=-3, spike_tip="K"),
    # defeat (dud): a deflated, dead skin, spike nubs, no glow, olive rot running down the wall
    14: dict(rx=5.5, ry_up=9.0, ry_dn=9.0, cy=16.5, face_x=0.55, face_shift=0.0, face_ax=2.2, face_ay=5.5,
             pleat=("m", "a"), core=("a", "m"), lip="b", lip_hi=None, shell=("a", "a"), spike_add=-3, spike_tip="K",
             wall_round=((0.85, 1),), grime=None, drip=4),
}
# In-between slots are copies of the nearest key until the execution tier draws them.
SLOT_SOURCE = {0: 0, 1: 0, 2: 0, 3: 3, 4: 3, 5: 3, 6: 3, 7: 8, 8: 8, 9: 10, 10: 10, 11: 11, 12: 14, 13: 14, 14: 14}
TAGS = [("idle", 0, 2), ("walk", 3, 6), ("attack_tell", 7, 8), ("attack_swing", 9, 10), ("hurt", 11, 11),
        ("down", 12, 14)]
TICKS = [14, 12, 14, 8, 8, 8, 8, 6, 24, 2, 4, 10, 4, 8, 0]
KEY_FRAMES = sorted(KEYS)


def all_frames() -> list[list[str]]:
    drawn = {k: render(**v) for k, v in KEYS.items()}
    return [drawn[SLOT_SOURCE[i]] for i in range(len(TICKS))]


def build() -> dict:
    frames = [{"duration_ms": max(1, round((t or 6) * 1000 / 60)), "cels": {"body": cel}}
              for t, cel in zip(TICKS, all_frames())]
    return {
        "format": "pixel-grid-v1", "canvas": [W, H], "pivot": PIVOT,
        "palette": [{"key": k, "name": n, "rgba": c, **({"accent": True} if acc else {})} for k, n, c, acc in PALETTE],
        "layers": ["body"], "frames": frames,
        "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS],
    }


BOXES = {  # frame pixels (x, y, w, h); placeholders, spec section 3
    "hurtbox": [6, 6, 14, 20],
    "contact_trigger": [6, 4, 18, 24],
    "blast_radius_px": 40,
    "blast_centre": [12, 16],
}


def write_manifest() -> None:
    rows = []
    for name, a, b in TAGS:
        for i in range(a, b + 1):
            ticks = TICKS[i] or None
            rows.append({"index": i, "tag": name, "ticks": ticks, "duration_ms": max(1, round((ticks or 6) * 1000 / 60)),
                         "author": "key" if i in KEYS else "copy-of-key-%d" % SLOT_SOURCE[i]})
    manifest = {
        "schema_version": 1,
        "id": "rot_sticker_floor01_v1_candidate",
        "type": "character_animation",
        "source_path": "assets/source/characters/rot_sticker/rot_01.ase",
        "grid_source": "assets/source/characters/rot_sticker/rot_01.grid.json",
        "output_atlas": "generated/art/rot-01.png",
        "output_metadata": "generated/art/rot-01.json",
        "palette_id": "rot-sticker-floor01-v1",
        "frame_size": [W, H],
        "pivot": PIVOT,
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored enemy animation draft (fan-concept layer: Floor 1 Rot Sticker; runtime role IP-neutral "
                 "surface-crawling contact bomb)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "ROT-01 design-edited keys: six keys drawn by rules in scripts/author_rot01.py; nine in-between slots are "
                      "copies of the nearest key",
            "generator": "Draft spec and keys: Claude Sonnet. Design edit: Claude Opus. Independent review: pending. "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "rot_sticker": {
            "status": "keys design-edited; review and in-betweens pending; not wired into any Bevy binary",
            "spec": "docs/02-art/rot-01-spec.md",
            "opaque_height_idle": 20,
            "tick_hz": 60,
            "hold_preview_ms": 100,
            "orientation": {"authored": "wall_left_attach", "creep_direction": "-y",
                            "ceiling": "rotate 90 degrees clockwise about the pivot at runtime",
                            "floor": "rotate 90 degrees counter-clockwise about the pivot at runtime",
                            "right_wall": "mirror horizontally about the pivot column"},
            "contact_column": WALL_X,
            "grounded_tags": [],
            "airborne_tags": [],
            "playback": {"idle": "loop", "walk": "loop", "attack_tell": "once", "attack_swing": "once",
                         "hurt": "once", "down": "hold"},
            "key_frames": KEY_FRAMES,
            "key_fixture": "tests/fixtures/rot-01_keys.json",
            "boxes": BOXES,
            "attack_ticks": {"tell": [0, 29], "burst_onset": [30, 31], "burst_first": [32, 35]},
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
