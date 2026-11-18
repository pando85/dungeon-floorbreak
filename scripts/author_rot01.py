#!/usr/bin/env python3
"""Deterministic authoring script for ROT-01 (Rot Sticker readability set), first draft.

Writes ``assets/source/characters/rot_sticker/rot_01.grid.json`` (pixel-grid-v1), the matching ``.ase`` via
``scripts/pixel_grid_to_ase.py`` and ``assets/manifests/rot-01.json``. Design spec: ``docs/02-art/rot-01-spec.md``.

Draft status: the six keys (frames 0, 3, 8, 10, 11, 14) are drawn by rules (a shaded superellipse dome, radial ribs,
spikes, a glow core and a seam), parametrised per key in ``KEYS``. The nine in-between slots are copies of the nearest
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

SPIKE_ANGLES = (-66, -40, -14, 14, 40, 66)  # degrees from +x, y down; the wall side has none


def _sd(x, y, cx, cy, rx, ry, n=2.7):
    return (abs((x - cx) / rx) ** n + abs((y - cy) / ry) ** n)


def render(rx=12.5, ry=9.8, dy=0.0, shear=0.0, spike=2.0, droop=0.0, rib="m", seam="r", core=("g", "o", "r"),
           rc=2.0, halo=None, crack=None, flash=0, grime=True, drip=0, spike_key="p") -> list[str]:
    cx, cy = float(WALL_X), 16.0 + dy
    hx = cx + rx * 0.5  # rib hub (the glow core sits here)
    g: dict[tuple[int, int], str] = {}
    mask = set()
    for y in range(H):
        for x in range(WALL_X, W):
            xs = x + 0.5 - shear * (y + 0.5 - cy) / ry
            if _sd(xs, y + 0.5, cx, cy, rx, ry) <= 1.0:
                mask.add((x, y))

    def inside(p):
        return p in mask

    def boundary(p):
        x, y = p
        return any((x + dx, y + dy_) not in mask for dx, dy_ in ((1, 0), (0, 1), (0, -1))) or \
            (x - 1 >= WALL_X and (x - 1, y) not in mask)

    # fill ramp from a highlight above and ahead of the hub
    lx, ly = hx + 2.0, cy - 4.5
    for p in mask:
        x, y = p
        t = math.hypot(x + 0.5 - lx, y + 0.5 - ly) / (rx * 1.15)
        g[p] = "c" if t < 0.42 else ("b" if t < 0.78 else "a")
    # radial ribs: 8 rays from the hub, 1 px wide
    for p in mask:
        x, y = p
        vx, vy = x + 0.5 - hx, y + 0.5 - cy
        r_ = math.hypot(vx, vy)
        if r_ < rc + 0.8:
            continue
        for k in range(8):
            a = math.radians(k * 45 + 22.5)
            if abs(-vx * math.sin(a) + vy * math.cos(a)) < 0.52 and vx * math.cos(a) + vy * math.sin(a) > 0:
                g[p] = rib
    # rot grime: olive stains on the lower shell
    if grime:
        for ox, oy in ((2, 6), (3, 6), (2, 7), (8, 5), (9, 5), (9, 6), (4, -7), (5, -7)):
            p = (int(cx + ox), int(cy + oy))
            if p in mask and g[p] in "abc":
                g[p] = "x"
    # outline (not on the wall side) and a cold rim on the upper-right edge
    for p in sorted(mask):
        if boundary(p):
            g[p] = "K"
    for p in sorted(mask):
        x, y = p
        if g[p] in "abc" and x + 0.5 > hx and y + 0.5 < cy and ((x, y - 1) in mask and g[(x, y - 1)] == "K"
                                                                or (x + 1, y) in mask and g[(x + 1, y)] == "K"):
            g[p] = "v"
    # glow core
    for p in mask:
        x, y = p
        d = math.hypot(x + 0.5 - hx, y + 0.5 - cy)
        if d <= rc:
            g[p] = core[0] if d <= rc * 0.45 else (core[1] if d <= rc * 0.8 else core[2])
    # seam on the wall plane
    for y in range(H):
        if (WALL_X, y) in mask:
            near_centre = abs(y + 0.5 - cy) <= ry * 0.42
            g[(WALL_X, y)] = seam if near_centre else "m"
    # hurt flash: white pixels on the shell edge
    if flash:
        for p in sorted(mask):
            x, y = p
            if g[p] in "vb" and x + 0.5 > hx and y + 0.5 < cy + 2:
                g[p] = "w"
    # spikes
    if spike:
        for ang in SPIKE_ANGLES:
            a = math.radians(ang)
            ux, uy = math.cos(a), math.sin(a)
            # root on the dome edge along the ray from the hub
            rx0 = hx
            t = 0.0
            while t < 40 and (int(rx0 + ux * t), int(cy + uy * t)) in mask:
                t += 0.25
            for i in range(int(round(spike))):
                f = (i + 1) / max(1, int(round(spike)))
                ux2 = ux * (1 - droop * f)
                uy2 = uy * (1 - droop * f) + droop * f
                px = int(round(rx0 + ux * t + ux2 * (i + 0.2)))
                py = int(round(cy + uy * t + uy2 * (i + 0.2)))
                if 0 <= px < W and 0 <= py < H and (px, py) not in mask:
                    g[(px, py)] = spike_key if i == int(round(spike)) - 1 else "r"
    # halo: a contiguous dim ring just outside the outline
    if halo:
        ring = set()
        for (x, y) in mask:
            for dx, dy_ in ((1, 0), (0, 1), (0, -1), (1, 1), (1, -1)):
                q = (x + dx, y + dy_)
                if q not in mask and q not in g and 0 <= q[0] < W and 0 <= q[1] < H and q[0] > WALL_X:
                    ring.add(q)
        for q in sorted(ring):
            g[q] = halo[0] if (q[0] + q[1]) % 3 else halo[1]
    # burst cracks: bright seams from the core to the rim
    if crack:
        for k in range(5):
            a = math.radians(k * 72 - 144 + 20)
            for i in range(0, 20):
                px = int(round(hx + math.cos(a) * i * 0.8))
                py = int(round(cy + math.sin(a) * i * 0.8))
                if (px, py) in mask and g[(px, py)] != "K":
                    g[(px, py)] = crack
    # dud drip down the wall
    for i in range(drip):
        y = int(cy + ry) + i
        if y < H:
            g[(WALL_X + 1, y)] = "x"
            g[(WALL_X, y)] = "x" if i % 2 == 0 else "m"
    rows = [["."] * W for _ in range(H)]
    for (x, y), c in g.items():
        rows[y][x] = c
    return ["".join(r) for r in rows]


# Key poses (spec section 7). Index -> render parameters.
KEYS = {
    0: dict(),  # idle: relaxed dome, ember core, dark ribs, 2 px spikes
    3: dict(rx=11.5, ry=10.5, dy=-1.0, shear=-2.5, spike=2.0, core=("o", "r", "m"), rc=1.8),  # creep: top leans up the wall
    8: dict(rx=14.0, ry=10.5, spike=3.0, rib="o", seam="o", core=("G", "g", "o"), rc=3.4, halo=("m", "r")),  # tell: glow
    10: dict(rx=15.0, ry=11.5, spike=3.0, rib="g", seam="G", core=("G", "G", "g"), rc=5.0, halo=("g", "o"),
             crack="G", spike_key="g"),  # attack: first burst frame
    11: dict(rx=10.5, ry=10.0, dy=0.0, shear=1.5, spike=1.0, rib="m", seam="m", core=("w", "w", "r"), rc=1.9,
             flash=1),  # hurt
    14: dict(rx=5.5, ry=9.0, dy=0.5, spike=1.0, droop=0.8, rib="m", seam="m", core=("m", "m", "a"), rc=1.3,
             drip=3),  # defeat: dud, deflated flat
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
            "origin": "ROT-01 first draft: six keys drawn by rules in scripts/author_rot01.py; nine in-between slots are "
                      "copies of the nearest key",
            "generator": "Draft spec and keys: Claude Sonnet. Senior edit and independent review: pending. "
                         "No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["floor-01-enemy-concept-scatterers-rot-sticker-slime-v1"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "rot_sticker": {
            "status": "first draft; senior edit and review pending; not wired into any Bevy binary",
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
