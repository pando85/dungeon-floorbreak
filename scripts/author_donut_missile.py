#!/usr/bin/env python3
"""ANIM-DONUT-03 authoring record: Princess Donut's Magic Missile cast (40 px) and the missile VFX.

Spec: docs/02-art/donut-magic-missile-anim-03-spec.md.

The cast is a **head swap phase-synced to locomotion**: every missile-atlas frame is a locomotion frame whose
``tail``, ``legs_far``, ``body`` and ``legs_near`` cels are copied unchanged, and whose ``head``, ``face`` and tiara
part of ``gear`` are replaced by a cast head, translated by that locomotion frame's head offset (the locomotion
head is the master head translated in every frame, see ``HEAD_OFFSETS``). Atlas frame = 16 * phase + locomotion
frame, so the runtime keeps playing the locomotion clock and only swaps atlas row while a cast phase is active.

Keys drawn here (frozen by ``tests/fixtures/anim-donut-03_keys.json``): cast frames 0 (gather head on idle), 16
(release head on idle), 24 (release on run suspension) and 28 (release on jump rise); VFX frames 0 (bolt), 4 (flash),
5 (claw-scratch peak) and 7 (fade). Derived cast frames are composited by the same rule. In-betweens (cast_follow
32-47, VFX 1-3, 6, 8) are copies of the nearest key for the executor to redraw (spec section 9).

Deterministic: literal pixel edits and float rules with no randomness; no image-model output, no tracing.

Usage:
  python3 scripts/author_donut_missile.py keys [--check]     write (or check) grids and manifests
  python3 scripts/author_donut_missile.py key-scale          docs/evidence/anim-donut-03/keys-scale.png (+ -4x copy)
"""
from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCO_GRID = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
CAST_GRID = ROOT / "assets/source/characters/donut/donut_h40_missile.grid.json"
CAST_MANIFEST = ROOT / "assets/manifests/donut_h40_missile.json"
VFX_GRID = ROOT / "assets/source/vfx/magic_missile/vfx_magic_missile.grid.json"
VFX_MANIFEST = ROOT / "assets/manifests/vfx_magic_missile.json"
EVIDENCE = ROOT / "docs/evidence/anim-donut-03"
TICK_HZ = 60

# ----------------------------------------------------------------------------- cast (character atlas)
CANVAS = (72, 44)
PIVOT = (46, 43)
N_LOCO = 16
PHASES = ("cast_gather", "cast_release", "cast_follow")
PHASE_TICKS = {"cast_gather": (0, 3), "cast_release": (4, 9), "cast_follow": (10, 15)}   # cast-local, inclusive
RELEASE_TICK = 4
REFIRE_TICK = 16
# head offset (dx, dy) of each locomotion frame relative to idle frame 0 (measured; tests re-measure it)
HEAD_OFFSETS = [(0, 0), (0, -1), (0, -1), (0, 0), (0, 4), (0, 5), (-1, 5), (0, 4), (1, 4), (1, 4),
                (0, 7), (1, 3), (1, 2), (0, 3), (0, 7), (0, 3)]
THRUST = {"cast_gather": (0, 0), "cast_release": (1, 0), "cast_follow": (1, 0)}
SPAWN_FRAME0 = (68, 24)          # bolt head centre at the release tick, frame-0 coordinates (right cheek, mouth row)
CAST_KEYS = (0, 16, 24, 28)
CAST_INBETWEENS = tuple(range(32, 48))


def blank() -> list[list[str]]:
    return [["."] * CANVAS[0] for _ in range(CANVAS[1])]


def to_rows(g: list[list[str]]) -> list[str]:
    return ["".join(r) for r in g]


def shift(rows: list[str], dx: int, dy: int) -> list[str]:
    out = blank()
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch != "." and 0 <= x + dx < CANVAS[0] and 0 <= y + dy < CANVAS[1]:
                out[y + dy][x + dx] = ch
    return to_rows(out)


def put(g: list[list[str]], pts: dict[tuple[int, int], str]) -> None:
    for (x, y), ch in pts.items():
        g[y][x] = ch


def row_put(g: list[list[str]], y: int, x0: int, s: str) -> None:
    for i, ch in enumerate(s):
        if ch != " ":
            g[y][x0 + i] = ch


def cast_head(phase: str, master: dict) -> dict[str, list[str]]:
    """head, face and tiara cels of one cast phase at the idle frame-0 position (before the thrust)."""
    head = [list(r) for r in master["head"]]
    face = [list(r) for r in master["face"]]
    tiara = [list(r) if y <= 8 else ["."] * CANVAS[0] for y, r in enumerate(master["gear"])]

    # Ears pinned flat and out (airplane ears): drop the tips, show the inner ear sideways one row lower.
    for x in (39, 40, 41, 42, 61, 62, 63, 64):
        head[8][x] = head[9][x] = "."
    row_put(head, 10, 37, "jjEEBB")
    row_put(head, 11, 38, "jBEB")
    row_put(head, 10, 61, "BBEEjK")
    row_put(head, 11, 62, "BEDK")

    if phase == "cast_gather":
        # Anticipation: upper lids drop one row (focused stare, pupils stay big); the tiara gem charges.
        for x in range(43, 61):
            face[16][x] = "."
        row_put(face, 17, 44, "KKKKK")
        row_put(face, 17, 56, "KKKKK")
        put(tiara, {(51, 1): "P", (50, 2): "P", (51, 2): "w", (52, 2): "P", (51, 3): "P", (51, 4): "w", (51, 5): "P"})
    else:
        # Payoff: angry half-lidded glare (lids two rows down, inner corners lower), short hiss with fangs.
        for y in (16, 17):
            for x in range(43, 61):
                face[y][x] = "."
        row_put(face, 18, 43, "KKKKKK")
        row_put(face, 18, 54, "KKKKKK")
        row_put(face, 19, 48, "KK")
        row_put(face, 19, 54, "KK")
        row_put(face, 19, 45, "w")
        row_put(face, 19, 56, "w")
        row_put(face, 22, 50, "    ")
        row_put(face, 23, 50, "KKKK")
        row_put(face, 24, 50, "WEEW")
        row_put(face, 25, 51, "cc")
        put(tiara, {(51, 4): "P"})
    return {"head": to_rows(head), "face": to_rows(face), "tiara": to_rows(tiara)}


def collar_rows(master: dict) -> list[str]:
    return [r if y >= 9 else "." * CANVAS[0] for y, r in enumerate(master["gear"])]


def merge(a: list[str], b: list[str]) -> list[str]:
    return ["".join(cb if cb != "." else ca for ca, cb in zip(ra, rb)) for ra, rb in zip(a, b)]


def cast_frames() -> list[dict]:
    loco = json.loads(LOCO_GRID.read_text())
    master = loco["frames"][0]["cels"]
    collar = collar_rows(master)
    heads = {p: cast_head("cast_gather" if p == "cast_gather" else "cast_release", master) for p in PHASES}
    frames = []
    for phase in PHASES:
        tx, ty = THRUST[phase]
        h = heads[phase]
        for n in range(N_LOCO):
            base = loco["frames"][n]
            dx, dy = HEAD_OFFSETS[n]
            cels = copy.deepcopy(base["cels"])
            cels["head"] = shift(h["head"], dx + tx, dy + ty)
            cels["face"] = shift(h["face"], dx + tx, dy + ty)
            cels["gear"] = merge(shift(collar, dx, dy), shift(h["tiara"], dx + tx, dy + ty))
            frames.append({"duration_ms": round(round(base["duration_ms"] * TICK_HZ / 1000) * 1000 / TICK_HZ), "cels": cels})
    return frames


def build_cast_grid() -> dict:
    loco = json.loads(LOCO_GRID.read_text())
    return {"format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT), "palette": loco["palette"],
            "layers": loco["layers"], "frames": cast_frames(),
            "tags": [{"name": p, "from": i * N_LOCO, "to": i * N_LOCO + N_LOCO - 1, "direction": "forward"}
                     for i, p in enumerate(PHASES)]}


def spawn_offset(n: int) -> list[int]:
    dx, dy = HEAD_OFFSETS[n]
    tx, ty = THRUST["cast_release"]
    return [SPAWN_FRAME0[0] + dx + tx - PIVOT[0], SPAWN_FRAME0[1] + dy + ty - PIVOT[1]]


def build_cast_manifest() -> dict:
    return {
        "schema_version": 1,
        "id": "donut_h40_missile_candidate_v0",
        "type": "character_animation",
        "source_path": "assets/source/characters/donut/donut_h40_missile.ase",
        "grid_source": "assets/source/characters/donut/donut_h40_missile.grid.json",
        "output_atlas": "generated/art/donut_h40_missile.png",
        "output_metadata": "generated/art/donut_h40_missile.json",
        "palette_id": "donut-master-h40",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": list(PHASES),
        "required_alpha": True,
        "owner": "AI-authored animation candidate (fan-concept layer: Princess Donut; runtime role IP-neutral small "
                 "quadruped ranged primary attack)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "ANIM-DONUT-03 design keys: cast heads drawn as literal pixel edits of the approved master head "
                      "in scripts/author_donut_missile.py, composited onto the ANIM-DONUT-01 locomotion frames",
            "generator": "spec + keys: Claude Opus; in-betweens and execution pending; review pending. No image-model "
                         "raster output, no tracing",
            "generated_art": True,
            "input_reference_ids": ["assets/source/characters/donut/donut_h40_locomotion.grid.json"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "cast": {
            "status": "Design keys frozen; cast_follow in-betweens are copies of cast_release; not wired into any Bevy binary",
            "spec": "docs/02-art/donut-magic-missile-anim-03-spec.md",
            "tick_hz": TICK_HZ,
            "composition": "head_swap_phase_synced: atlas frame = 16 * phase_index + locomotion_frame",
            "locomotion_atlas": "assets/source/characters/donut/donut_h40_locomotion.grid.json",
            "phase_ticks": {p: list(v) for p, v in PHASE_TICKS.items()},
            "release_tick": RELEASE_TICK,
            "refire_tick_placeholder": REFIRE_TICK,
            "head_offsets": [list(o) for o in HEAD_OFFSETS],
            "projectile_spawn_offset_by_locomotion_frame": [spawn_offset(n) for n in range(N_LOCO)],
            "key_frames": list(CAST_KEYS),
            "inbetween_frames": list(CAST_INBETWEENS),
            "key_fixture": "tests/fixtures/anim-donut-03_keys.json",
        },
    }


# ----------------------------------------------------------------------------- VFX (missile and impact)
VCANVAS = (32, 32)
VPIVOT = (16, 16)
VPALETTE = [  # key, name, rgba, accent
    ("k", "plum_rim", "#2a0f2eff", False),
    ("d", "magenta_deep", "#7a1a63ff", False),
    ("m", "magenta", "#e2309fff", False),      # same value as Donut's tiara gem M
    ("p", "pink_hot", "#ff7fd0ff", False),
    ("c", "core_pale", "#ffd8f4ff", False),
    ("w", "white_hot", "#ffffffff", True),
    ("v", "violet_mote", "#9a6ee0ff", False),  # Donut's collar violet
    ("l", "lilac_mote", "#dbc6fbff", False),   # Donut's tiara lilac
]
VTAGS = [("missile_flight", 0, 3, "loop"), ("missile_impact", 4, 8, "once")]
VTICKS = [4, 4, 4, 4, 2, 3, 4, 4, 5]
VFX_KEYS = (0, 4, 5, 7)
VFX_INBETWEENS = (1, 2, 3, 6, 8)
SPEED_PX_PER_TICK = 6
RANGE_PX = 240
HITBOX = {"offset": [4, 0], "size": [8, 6]}


def vblank() -> list[list[str]]:
    return [["."] * VCANVAS[0] for _ in range(VCANVAS[1])]


def bolt() -> list[str]:
    """Flight key: hot oval head at the pivot, three parallel claw-scratch streaks trailing behind."""
    g = vblank()
    cx, cy, rx, ry = 19.0, 16.0, 3.6, 2.6
    for y in range(VCANVAS[1]):
        for x in range(VCANVAS[0]):
            e = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
            ef = ((x - (cx + 0.8)) / rx) ** 2 + ((y - cy) / ry) ** 2
            if ef <= 0.10 and x >= cx:
                g[y][x] = "w"
            elif ef <= 0.42:
                g[y][x] = "c"
            elif e <= 0.78:
                g[y][x] = "p"
            elif e <= 1.0:
                g[y][x] = "m"
            elif e <= 1.75:
                g[y][x] = "d" if x <= cx + 1 else "m"
    for y, length in ((14, 8), (16, 12), (18, 8)):
        x0 = 15
        while g[y][x0] != ".":
            x0 -= 1
        for i in range(length):
            x = x0 - i
            g[y][x] = "p" if i < 3 else "m" if i < 6 else "d" if i < length - 1 else "v"
    return to_rows(g)


def disc(g, cx, cy, ramp) -> None:
    for y in range(VCANVAS[1]):
        for x in range(VCANVAS[0]):
            r = math.hypot(x - cx, y - cy)
            for limit, ch in ramp:
                if r <= limit:
                    g[y][x] = ch
                    break


def slashes(g, width: float, gap: bool, hot: bool) -> None:
    """Three parallel claw scratches raking down and back across the hit point (a right-paw swipe)."""
    ux, uy = -0.55, 0.835
    nx, ny = 0.835, 0.55
    length = 15.0 if not gap else 13.0
    for off in (-5.0, 0.0, 5.0):
        sx, sy = 15.5 - ux * length / 2 + nx * off, 15.5 - uy * length / 2 + ny * off
        for y in range(VCANVAS[1]):
            for x in range(VCANVAS[0]):
                px_, py_ = x - sx, y - sy
                s = (px_ * ux + py_ * uy) / length
                if not 0.0 <= s <= 1.0 or (gap and 0.42 < s < 0.58):
                    continue
                d = abs(px_ * nx + py_ * ny)
                w = width * math.sin(math.pi * s) + 0.62
                if d > w:
                    continue
                q = d / w
                if s < 0.12 or s > 0.88:
                    ch = "m" if hot else "v"
                elif hot:
                    ch = "c" if q < 0.4 and w > 1.4 else "p" if q < 0.75 else "m"
                else:
                    ch = "m" if q < 0.5 and w > 1.0 else "d"
                g[y][x] = ch


def chunks(g, pts) -> None:
    for x, y, ch in pts:
        g[y][x] = ch
        g[y][x + 1] = ch


def flash() -> list[str]:
    g = vblank()
    disc(g, 15.5, 15.5, [(0.8, "w"), (2.0, "c"), (3.2, "p"), (4.3, "m"), (5.0, "d")])
    chunks(g, [(8, 12, "p"), (9, 20, "m"), (6, 16, "p")])
    return to_rows(g)


def scratch_peak() -> list[str]:
    g = vblank()
    disc(g, 15.5, 15.5, [(1.6, "p"), (2.6, "m")])
    slashes(g, 1.2, gap=False, hot=True)
    g[15][15] = g[16][15] = "w"
    chunks(g, [(4, 9, "m"), (3, 21, "v"), (25, 7, "v"), (24, 24, "m"), (11, 4, "l"), (19, 27, "v")])
    return to_rows(g)


def scratch_fade() -> list[str]:
    g = vblank()
    slashes(g, 1.0, gap=True, hot=False)
    chunks(g, [(3, 7, "v"), (2, 18, "d"), (25, 4, "l"), (25, 21, "v"), (10, 2, "v")])
    return to_rows(g)


def vfx_rows() -> list[list[str]]:
    b, f, p, z = bolt(), flash(), scratch_peak(), scratch_fade()
    return [b, b, b, b, f, p, p, z, z]          # 1-3, 6, 8: copies of the nearest key for the executor


def build_vfx_grid() -> dict:
    frames = [{"duration_ms": round(t * 1000 / TICK_HZ), "cels": {"body": rows}} for t, rows in zip(VTICKS, vfx_rows())]
    pal = []
    for key, name, rgba, accent in VPALETTE:
        e = {"key": key, "name": name, "rgba": rgba}
        if accent:
            e["accent"] = True
        pal.append(e)
    return {"format": "pixel-grid-v1", "canvas": list(VCANVAS), "pivot": list(VPIVOT), "palette": pal,
            "layers": ["body"], "frames": frames,
            "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b, _ in VTAGS]}


def build_vfx_manifest() -> dict:
    tag_of = {n: name for name, a, b, _ in VTAGS for n in range(a, b + 1)}
    frames = [{"index": n, "tag": tag_of[n], "ticks": t, "duration_ms": round(t * 1000 / TICK_HZ),
               "author": "key" if n in VFX_KEYS else "inbetween"} for n, t in enumerate(VTICKS)]
    return {
        "schema_version": 1,
        "id": "vfx_magic_missile_32_candidate_v0",
        "type": "vfx_animation",
        "source_path": "assets/source/vfx/magic_missile/vfx_magic_missile.ase",
        "grid_source": "assets/source/vfx/magic_missile/vfx_magic_missile.grid.json",
        "output_atlas": "generated/art/vfx_magic_missile.png",
        "output_metadata": "generated/art/vfx_magic_missile.json",
        "palette_id": "vfx-missile-v0",
        "frame_size": list(VCANVAS),
        "pivot": list(VPIVOT),
        "animation_tags": [n for n, *_ in VTAGS],
        "required_alpha": True,
        "owner": "AI-authored VFX candidate (fan-concept layer: Donut's Magic Missile; runtime role IP-neutral small "
                 "magic projectile and impact)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": "ANIM-DONUT-03 design keys 0, 4, 5, 7 rasterized natively from rules in "
                      "scripts/author_donut_missile.py; in-betweens 1-3, 6, 8 are key copies pending execution",
            "generator": "spec + keys: Claude Opus; in-betweens and execution pending; review pending. No image-model "
                         "raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": [],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "vfx": {
            "status": "Design keys frozen; in-betweens are key copies; not wired into any Bevy binary",
            "spec": "docs/02-art/donut-magic-missile-anim-03-spec.md",
            "tick_hz": TICK_HZ,
            "free_tags": [n for n, *_ in VTAGS],
            "playback": {n: pb for n, _a, _b, pb in VTAGS},
            "facing": "right",
            "speed_px_per_tick_placeholder": SPEED_PX_PER_TICK,
            "range_px_placeholder": RANGE_PX,
            "hitbox_placeholder": HITBOX,
            "fizzle_at_range": "play missile_impact frames 7-8 only",
            "key_frames": list(VFX_KEYS),
            "inbetween_frames": list(VFX_INBETWEENS),
            "key_fixture": "tests/fixtures/anim-donut-03-vfx_keys.json",
            "frames": frames,
        },
    }


# ----------------------------------------------------------------------------- output
def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def mdump(data: dict) -> str:
    return json.dumps(data, indent=2) + "\n"


OUTPUTS = ((CAST_GRID, build_cast_grid, dump), (CAST_MANIFEST, build_cast_manifest, mdump),
           (VFX_GRID, build_vfx_grid, dump), (VFX_MANIFEST, build_vfx_manifest, mdump))


def key_scale(out: Path) -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.common import flip, grid_images, load_grid_file, repo_path
    from scripts.pixelart.scene import Floor

    cast = grid_images(build_cast_grid())
    vfx = grid_images(build_vfx_grid())
    gob_grid = load_grid_file(repo_path("assets/source/characters/goblin/goblin_a_h36.grid.json"))
    gob, gob_piv = flip(grid_images(gob_grid)[0], tuple(gob_grid["pivot"]))
    x0 = 404
    entries = [(cast[0], PIVOT, x0, 0), (cast[16], PIVOT, x0 + 64, 0), (cast[24], PIVOT, x0 + 128, 0),
               (cast[28], PIVOT, x0 + 192, 14, False),
               (vfx[0], VPIVOT, x0 + 268, 19, False), (gob, gob_piv, x0 + 344, 0),
               (vfx[5], VPIVOT, x0 + 326, 19, False)]
    fl = Floor()
    vx, vy = fl.view[0], fl.view[1]
    gy, ox = fl.ground(x0) - vy, x0 - vx
    box = (ox - 52, gy - 62, ox + 380, gy + 10)
    lit = fl.render(entries, 1).crop(box)
    unlit = fl.render(entries, 1, lit=False).crop(box)
    sheet = Image.new("RGB", (lit.width, lit.height * 2 + 4), (20, 16, 24))
    sheet.paste(lit, (0, 0))
    sheet.paste(unlit, (0, lit.height + 4))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    big = out.with_name(out.stem + "-4x.png")
    lit.resize((lit.width * 4, lit.height * 4), Image.NEAREST).save(big)
    print(f"wrote {out.relative_to(ROOT)} {sheet.size} and {big.relative_to(ROOT)}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "key-scale"))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.mode == "key-scale":
        key_scale(EVIDENCE / "keys-scale.png")
        return 0
    stale = []
    for path, build, fmt in OUTPUTS:
        text = fmt(build())
        if a.check:
            if not path.is_file() or path.read_text() != text:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    if stale:
        print("differs: " + ", ".join(stale))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
