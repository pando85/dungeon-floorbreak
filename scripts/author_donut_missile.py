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
5 (claw-scratch peak) and 7 (fade). Derived cast frames are composited by the same rule. In-betweens (spec section
9): the follow head of cast_follow 32-47 (``cast_head("cast_follow")``) and VFX 1-3 (``BOLT_CYCLE``), 6
(``scratch_cooling``) and 8 (``scratch_embers``), all drawn by rules in this file.

Deterministic: literal pixel edits and float rules with no randomness; no image-model output, no tracing.

Usage:
  python3 scripts/author_donut_missile.py keys [--check]     write (or check) grids and manifests
  python3 scripts/author_donut_missile.py key-scale          docs/evidence/anim-donut-03/keys-scale.png (+ -4x copy)
  python3 scripts/author_donut_missile.py evidence           bases, 1x scenes, cast GIF, layout diagram and key-scale
                                                             (needs `bash scripts/export_assets.sh` first)
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
THRUST = {"cast_gather": (0, 0), "cast_release": (1, 0), "cast_follow": (0, 0)}      # the head returns on follow
SPAWN_FRAME0 = (72, 24)          # bolt pivot at the release tick, frame-0 coordinates: 4 px ahead of the right
                                 # cheek at mouth row (gate fix: the streaks must not cover the hiss on ticks 4-5)
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

    if phase == "cast_follow":
        # Recovery (spec section 9, review fix): the lids come back to the master lids, the head returns (no
        # thrust), the closed master mouth, no fangs, brow pixels or twinkle, the master gem, and the ears halfway
        # up: tips one row lower than the master (row 9), the ear wedge 1 px out past the master outline (release:
        # 2 px, tips gone). The face cel is the master's; the ears are the only relax cue left.
        for x in (39, 40, 41, 42, 62, 63, 64):
            head[8][x] = "."
        for x in (39, 40, 41, 42, 61, 62, 63, 64):
            head[9][x] = "."
        row_put(head, 9, 40, "jK")
        row_put(head, 9, 63, "jK")
        row_put(head, 10, 38, "Kj")
        row_put(head, 11, 37, "K")
        row_put(head, 10, 62, "EBjK")
        row_put(head, 11, 64, "jDK")
    elif phase == "cast_gather":
        # Anticipation: master ears; upper lids drop one row (focused stare, pupils stay big); the gem charges.
        for x in range(43, 61):
            face[16][x] = "."
        row_put(face, 17, 44, "KKKKK")
        row_put(face, 17, 56, "KKKKK")
        put(tiara, {(51, 1): "P", (50, 2): "P", (51, 2): "w", (52, 2): "P", (51, 3): "P", (51, 4): "w", (51, 5): "P"})
    else:
        # Payoff (gate fix): airplane ears, tips removed, ear wedge 2 rows lower and out past the head outline
        # with a dark outer edge, so the head outline drops against the master and gather.
        for x in (39, 40, 41, 42, 61, 62, 63, 64):
            head[8][x] = head[9][x] = "."
        row_put(head, 10, 37, "KjEEBB")
        row_put(head, 11, 37, "KjEB")
        row_put(head, 10, 61, "BBEEjK")
        row_put(head, 11, 63, "BEjK")
        # Angry half-lidded glare (lids two rows down, inner corners lower, outer brow pixel raised: inward slant)
        # and an open 3 x 2 hiss with two white fang points.
        for y in (16, 17):
            for x in range(43, 61):
                face[y][x] = "."
        row_put(face, 18, 43, "KKKKKK")
        row_put(face, 18, 54, "KKKKKK")
        row_put(face, 19, 48, "KK")
        row_put(face, 19, 54, "KK")
        row_put(face, 19, 45, "w")
        row_put(face, 19, 56, "w")
        row_put(face, 17, 43, "K")
        row_put(face, 17, 59, "K")
        for x in range(50, 55):
            face[23][x] = "."
        row_put(face, 23, 51, "wKw")
        row_put(face, 24, 51, "KKK")
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
    heads = {p: cast_head(p, master) for p in PHASES}
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
            "generator": "spec + keys: Claude Opus; in-betweens + export: Claude Sonnet; review pending. No image-model "
                         "raster output, no tracing",
            "generated_art": True,
            "input_reference_ids": ["assets/source/characters/donut/donut_h40_locomotion.grid.json"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "cast": {
            "status": "Keys frozen; cast_follow in-betweens drawn; independent review pending; not wired into any Bevy binary",
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


def bolt(lengths=(8, 12, 8), core=0.0) -> list[str]:
    """Flight key: hot oval head at the pivot, three parallel claw-scratch streaks trailing behind.

    ``lengths`` are the three streak lengths (top, middle, bottom) and ``core`` shifts the hot core fore (+) or
    aft (-) inside the unchanged oval; the defaults are the frozen key (frame 0)."""
    g = vblank()
    cx, cy, rx, ry = 19.0, 16.0, 3.6, 2.6
    for y in range(VCANVAS[1]):
        for x in range(VCANVAS[0]):
            e = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
            ef = ((x - (cx + 0.8 + core)) / rx) ** 2 + ((y - cy) / ry) ** 2
            if ef <= 0.10 and x >= cx + min(core, 0.0):
                g[y][x] = "w"
            elif ef <= 0.42:
                g[y][x] = "c"
            elif e <= 0.78:
                g[y][x] = "p"
            elif e <= 1.0:
                g[y][x] = "m"
            elif e <= 1.75:
                g[y][x] = "d" if x <= cx + 1 else "m"
    for y, length in zip((14, 16, 18), lengths):
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


def slashes(g, width: float, gap: bool, hot: bool, offsets=(-5.0, 0.0, 5.0), span=(0.0, 1.0), ramp=(0.4, 0.75)) -> None:
    """Three parallel claw scratches raking down and back across the hit point (a right-paw swipe).

    ``offsets`` picks the scratches and ``span`` the part of each that is drawn (remnants); ``ramp`` the core and pink limits of the hot ramp."""
    ux, uy = -0.55, 0.835
    nx, ny = 0.835, 0.55
    length = 15.0 if not gap else 13.0
    for off in offsets:
        sx, sy = 15.5 - ux * length / 2 + nx * off, 15.5 - uy * length / 2 + ny * off
        for y in range(VCANVAS[1]):
            for x in range(VCANVAS[0]):
                px_, py_ = x - sx, y - sy
                s = (px_ * ux + py_ * uy) / length
                if not span[0] <= s <= span[1] or (gap and 0.42 < s < 0.58):
                    continue
                d = abs(px_ * nx + py_ * ny)
                w = width * math.sin(math.pi * s) + 0.62
                if d > w:
                    continue
                q = d / w
                if s < 0.12 or s > 0.88:
                    ch = "m" if hot else "v"
                elif hot:
                    ch = "c" if q < ramp[0] and w > 1.4 else "p" if q < ramp[1] else "m"
                else:
                    ch = "m" if q < 0.5 and w > 1.0 else "d"
                g[y][x] = ch


def chunks(g, pts) -> None:
    for x, y, ch in pts:
        g[y][x] = ch
        g[y][x + 1] = ch


def merge_orphans(g) -> None:
    """Recolour every non-accent pixel with no same-key 8-neighbour to its most common opaque neighbour key, so
    slash anti-aliasing touches its stroke (toolkit V06 orphan definition)."""
    order = [k for k, *_ in VPALETTE]
    for _ in range(4):
        changed = False
        for y in range(VCANVAS[1]):
            for x in range(VCANVAS[0]):
                c = g[y][x]
                if c in (".", "w"):
                    continue
                around = [g[y + dy][x + dx] for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                          if (dx or dy) and 0 <= x + dx < VCANVAS[0] and 0 <= y + dy < VCANVAS[1]]
                opaque = [n for n in around if n not in (".", "w")]
                if c in around or not opaque:
                    continue
                g[y][x] = max(sorted(set(opaque), key=order.index), key=opaque.count)
                changed = True
        if not changed:
            return


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
    merge_orphans(g)
    chunks(g, [(4, 9, "m"), (3, 21, "v"), (25, 7, "v"), (24, 24, "m"), (11, 4, "l"), (19, 27, "v")])
    return to_rows(g)


SLASH_U = (-0.55, 0.835)            # rake direction of the claw scratches (down and back)
SLASH_N = (0.835, 0.55)             # normal to it: the three scratches sit at -5, 0, +5 px


def stroke(g, off: float, s0: float, s1: float, length: float, left: str, right: str, tip: str) -> None:
    """One clean 2 px wide diagonal piece of scratch ``off`` between ``s0`` and ``s1`` (fractions of ``length``): a
    ``left``/``right`` column pair with a 2 px ``tip`` row at each end (no isolated or fragmented pixels)."""
    cx, cy = 15.5 + SLASH_N[0] * off, 15.5 + SLASH_N[1] * off

    def point(s):
        t = (s - 0.5) * length
        return cx + SLASH_U[0] * t, cy + SLASH_U[1] * t

    (x0, y0), (_x1, y1) = point(s0), point(s1)
    ya, yb = round(y0), round(y1)
    for y in range(ya, yb + 1):
        xl = int(math.floor(x0 + (y - y0) * SLASH_U[0] / SLASH_U[1]))
        if y in (ya, yb):
            g[y][xl] = g[y][xl + 1] = tip
        else:
            g[y][xl], g[y][xl + 1] = left, right


def scratch_fade() -> list[str]:
    """Key 7: three scratches cooled to magenta/deep/violet, each broken once in the middle into two clean pieces."""
    g = vblank()
    for off in (-5.0, 0.0, 5.0):
        stroke(g, off, 0.0, 0.42, 13.0, "d", "m", "v")
        stroke(g, off, 0.58, 1.0, 13.0, "d", "m", "v")
    chunks(g, [(25, 4, "l"), (2, 22, "d"), (25, 22, "v")])
    return to_rows(g)


# --- in-betweens (spec section 9)
# frame 1 is the launch frame (short streaks: the release-tick bolt must not touch her cheek, spec section 4)
BOLT_CYCLE = (((8, 12, 8), 0.0), ((3, 4, 3), 1.0), ((7, 12, 10), -1.0), ((9, 11, 9), 0.0))   # frames 0-3
FLIGHT_PLAYBACK = {"launch": [1], "loop": [2, 3, 0]}
PEAK_MOTES = ((4, 9, "m"), (3, 21, "v"), (25, 7, "v"), (24, 24, "m"), (11, 4, "l"), (19, 27, "v"))


def push_out(pts, dist: int):
    """Move mote chunks ``dist`` px further from the hit point (15.5, 15.5), staying inside the cell."""
    out = []
    for x, y, ch in pts:
        vx, vy = x + 0.5 - 15.5, y - 15.5
        n = math.hypot(vx, vy) or 1.0
        nx_, ny_ = round(x + dist * vx / n), round(y + dist * vy / n)
        out.append((min(max(nx_, 0), VCANVAS[0] - 2), min(max(ny_, 0), VCANVAS[1] - 1), ch))
    return out


def scratch_cooling() -> list[str]:
    """Frame 6, between peak and fade: slashes thinner, no white and no core, motes 2 px further out."""
    g = vblank()
    disc(g, 15.5, 15.5, [(1.0, "p"), (2.0, "m")])
    slashes(g, 1.1, gap=False, hot=True, ramp=(0.0, 0.5))      # no core, pink body, magenta rim
    merge_orphans(g)
    chunks(g, push_out(PEAK_MOTES, 2))
    return to_rows(g)


def scratch_embers() -> list[str]:
    """Frame 8, last frame: the lower piece of each of the three scratches, cold and thin, and two last motes."""
    g = vblank()
    for off in (-5.0, 0.0, 5.0):
        stroke(g, off, 0.60, 1.0, 13.0, "d", "d", "v")
    chunks(g, [(2, 22, "d"), (26, 5, "v")])
    return to_rows(g)


def vfx_rows() -> list[list[str]]:
    flight = [bolt(*c) for c in BOLT_CYCLE]
    return [*flight, flash(), scratch_peak(), scratch_cooling(), scratch_fade(), scratch_embers()]


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
            "origin": "ANIM-DONUT-03 keys 0, 4, 5, 7 and in-betweens 1-3, 6, 8 rasterized natively from "
                      "rules in scripts/author_donut_missile.py",
            "generator": "spec + keys: Claude Opus; in-betweens + export: Claude Sonnet; review pending. No image-model "
                         "raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": [],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse "
                   "license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "vfx": {
            "status": "Keys frozen; in-betweens drawn; independent review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/donut-magic-missile-anim-03-spec.md",
            "tick_hz": TICK_HZ,
            "free_tags": [n for n, *_ in VTAGS],
            "playback": {n: pb for n, _a, _b, pb in VTAGS},
            "facing": "right",
            "speed_px_per_tick_placeholder": SPEED_PX_PER_TICK,
            "range_px_placeholder": RANGE_PX,
            "hitbox_placeholder": HITBOX,
            "flight_playback": FLIGHT_PLAYBACK,
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
    loco = grid_images(json.loads(LOCO_GRID.read_text()))
    vfx = grid_images(build_vfx_grid())
    gob_grid = load_grid_file(repo_path("assets/source/characters/goblin/goblin_a_h36.grid.json"))
    gob, gob_piv = flip(grid_images(gob_grid)[0], tuple(gob_grid["pivot"]))
    x0 = 404
    entries = [(cast[0], PIVOT, x0, 0), (cast[16], PIVOT, x0 + 64, 0), (cast[24], PIVOT, x0 + 128, 0),
               (cast[28], PIVOT, x0 + 192, 14, False),
               (vfx[0], VPIVOT, x0 + 268, 19, False), (gob, gob_piv, x0 + 344, 0),
               (vfx[5], VPIVOT, x0 + 326, 19, False), (vfx[0], VPIVOT, x0 + 100, 50, False)]
    # Comparison strip: master head vs release head on the run (frame 8) and air (frame 12) bases.
    compare = [(loco[8], PIVOT, x0, 0), (cast[24], PIVOT, x0 + 70, 0), (loco[12], PIVOT, x0 + 150, 14, False),
               (cast[28], PIVOT, x0 + 220, 14, False), (vfx[0], VPIVOT, x0 + 300, 40, False)]
    fl = Floor()
    vx, vy = fl.view[0], fl.view[1]
    gy, ox = fl.ground(x0) - vy, x0 - vx
    box = (ox - 52, gy - 62, ox + 380, gy + 10)
    lit = fl.render(entries, 1).crop(box)
    unlit = fl.render(entries, 1, lit=False).crop(box)
    comp = fl.render(compare, 1).crop(box)
    sheet = Image.new("RGB", (lit.width, lit.height * 3 + 8), (20, 16, 24))
    for n, im in enumerate((lit, unlit, comp)):
        sheet.paste(im, (0, n * (lit.height + 4)))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    big = out.with_name(out.stem + "-4x.png")
    both = Image.new("RGB", (lit.width, lit.height * 2 + 4), (20, 16, 24))
    both.paste(lit, (0, 0))
    both.paste(comp, (0, lit.height + 4))
    both.resize((both.width * 4, both.height * 4), Image.NEAREST).save(big)
    print(f"wrote {out.relative_to(ROOT)} {sheet.size} and {big.relative_to(ROOT)}")


# ----------------------------------------------------------------------------- evidence (Pillow models, not Bevy captures)
LOCO_NAMES = ("idle 0", "idle 1", "idle 2", "idle 3", "run 4", "run 5", "run 6", "run 7", "run 8", "run 9",
              "jstart 10", "jstart 11", "jrise 12", "jfall 13", "land 14", "land 15")
BASES = ((0, 0, "idle (loco 0)"), (8, 0, "run, suspension (loco 8)"), (12, 14, "jump rise (loco 12, lifted 14 px)"))
GOB_X = 610                      # goblin walking-line x in the Floor 1 layout
FIRST_BOLT_X = 479               # Donut at 452: bolt pivot at the release tick = 452 + 27


def _kit():
    sys.path.insert(0, str(ROOT / "scripts"))
    sys.path.insert(0, str(ROOT))
    from PIL import Image, ImageDraw
    from scripts.pixelart import common as c
    from scripts.pixelart.scene import Floor, reference_idle
    return Image, ImageDraw, c, Floor, reference_idle


def _sprites():
    _, _, c, _, reference_idle = _kit()
    cast = c.grid_images(build_cast_grid())
    loco = c.grid_images(json.loads(LOCO_GRID.read_text()))
    vfx = c.grid_images(build_vfx_grid())
    gob_grid = c.load_grid_file(c.repo_path("assets/source/characters/goblin/goblin_a_h36.grid.json"))
    gob, gob_piv = c.flip(c.grid_images(gob_grid)[0], tuple(gob_grid["pivot"]))
    carl, carl_piv = reference_idle("carl")
    return cast, loco, vfx, (gob, gob_piv), (carl, carl_piv)


def bases_sheet(out: Path) -> None:
    """Enlarged frames: the locomotion base and the three cast phases on the idle, run and air bases (4x)."""
    Image, ImageDraw, c, _, _ = _kit()
    cast, loco, _vfx, _g, _c = _sprites()
    k, pad, head, lab = 4, 6, 26, 15
    cols = [("locomotion (base)", loco, 0), ("cast_gather", cast, 0), ("cast_release", cast, 16), ("cast_follow", cast, 32)]
    w, h = CANVAS[0] * k, CANVAS[1] * k
    sheet = Image.new("RGB", (len(cols) * (w + pad) + pad, head + len(BASES) * (h + lab + pad) + pad), c.BG)
    d = ImageDraw.Draw(sheet)
    c.label(d, (pad, 6), "ANIM-DONUT-03 head swap on three locomotion bases at 4x; yellow = pivot (46, 43), cyan = walking line",
            c.WHITE, 12)
    for r, (n, _lift, name) in enumerate(BASES):
        for ci, (title, imgs, base) in enumerate(cols):
            x, y = pad + ci * (w + pad), head + r * (h + lab + pad)
            sheet.paste(c.cell(imgs[base + n], k, PIVOT, 43).convert("RGB"), (x, y))
            c.label(d, (x + 2, y + h + 1), f"{title}  frame {base + n}", c.GREY, 11)
        c.label(d, (pad + 4, head + r * (h + lab + pad) + 2), name, c.YELLOW, 11)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {out.relative_to(ROOT)}")


def cast_scenes(out_dir: Path) -> None:
    """1x gameplay-scale compositions on Floor 1: Donut casting next to Carl (lit and unlit), and the three phases
    on all three bases (lit)."""
    Image, ImageDraw, c, Floor, _ = _kit()
    cast, loco, vfx, (gob, gob_piv), (carl, carl_piv) = _sprites()
    fl = Floor()

    def bolt(frame, x, lift=19):
        return (vfx[frame], VPIVOT, x, lift, False)

    # Donut releasing (frame 16) at x 452 beside Carl; a fresh bolt at the spawn point, one 8 ticks later in flight
    # and the scratch peak on a GOB-01 goblin (a bolt from the previous cast).
    entries = [(carl, carl_piv, 404, 0), (cast[16], PIVOT, 452, 0), (gob, gob_piv, GOB_X, 0),
               bolt(1, FIRST_BOLT_X), bolt(3, FIRST_BOLT_X + 8 * SPEED_PX_PER_TICK), bolt(5, GOB_X - 18)]
    for k, tag in ((1, "640x360"), (3, "1920x1080")):
        fl.render(entries, k, True).save(out_dir / f"scene_cast_lit_{tag}.png")
        fl.render(entries, k, False).save(out_dir / f"scene_cast_unlit_{tag}.png")
    ph = []
    i = 0
    for n, lift, _name in BASES:
        for p in range(3):
            ph.append((cast[16 * p + n], PIVOT, 390 + 66 * i, lift, True))
            i += 1
    for k, tag in ((1, "640x360"), (3, "1920x1080")):
        fl.render(ph, k, True).save(out_dir / f"scene_phases_lit_{tag}.png")
    print("wrote scene_cast_{lit,unlit}_* and scene_phases_lit_*")


def flight_frame(k: int) -> int:
    """VFX frame of the k-th four-tick step of a bolt's flight: the launch frame once, then the loop."""
    loop = FLIGHT_PLAYBACK["loop"]
    return FLIGHT_PLAYBACK["launch"][k] if k < len(FLIGHT_PLAYBACK["launch"]) else loop[(k - len(FLIGHT_PLAYBACK["launch"])) % len(loop)]


def cast_sequence_gif(out: Path) -> None:
    """Cast timeline on lit Floor 1 at 2x, one frame per two 60 Hz ticks: gather 0-3, release 4-9 (bolt spawns at
    tick 4), follow 10-15, bolt flight at 6 px/tick, scratch impact on a goblin, then idle."""
    Image, _D, c, Floor, _ = _kit()
    cast, loco, vfx, (gob, gob_piv), (carl, carl_piv) = _sprites()
    fl = Floor()
    gob_left = GOB_X - gob_piv[0] + c.opaque_bbox(gob)[0]
    reach = HITBOX["offset"][0] + HITBOX["size"][0]
    contact_tick = next(t for t in range(RELEASE_TICK, 200)
                        if FIRST_BOLT_X + SPEED_PX_PER_TICK * (t - RELEASE_TICK) + reach >= gob_left)
    contact_x = FIRST_BOLT_X + SPEED_PX_PER_TICK * (contact_tick - RELEASE_TICK)
    impact_start = [sum(VTICKS[4:4 + i]) for i in range(5)]
    last = contact_tick + sum(VTICKS[4:]) + 14
    vx, vy = fl.view[0], fl.view[1]
    gy = fl.ground(452) - vy
    box = ((404 - vx - 44) * 2, (gy - 62) * 2, (GOB_X - vx + 34) * 2, (gy + 10) * 2)
    frames, ticks = [], []
    for t in range(0, last + 1, 2):
        donut = cast[0] if t < 4 else cast[16] if t < 10 else cast[32] if t < 16 else loco[0]
        ents = [(carl, carl_piv, 404, 0), (donut, PIVOT, 452, 0), (gob, gob_piv, GOB_X, 0)]
        if RELEASE_TICK <= t < contact_tick:
            ents.append((vfx[flight_frame((t - RELEASE_TICK) // 4)], VPIVOT,
                         FIRST_BOLT_X + SPEED_PX_PER_TICK * (t - RELEASE_TICK), 19, False))
        elif contact_tick <= t < contact_tick + sum(VTICKS[4:]):
            e = max(i for i, s0 in enumerate(impact_start) if s0 <= t - contact_tick)
            ents.append((vfx[4 + e], VPIVOT, contact_x, 19, False))
        frames.append(fl.render(ents, 2, True).crop(box))
        ticks.append(2)
    ticks[-1] += 40                   # rest on the idle pose before the loop
    c.save_gif(frames, c.gif_delays(ticks, TICK_HZ), out)
    print(f"wrote {out.relative_to(ROOT)} ({len(frames)} frames; contact at cast tick {contact_tick})")


def layout_diagram(out: Path) -> None:
    """Frame layout, pivot and timing of both atlases (for the owner)."""
    Image, ImageDraw, c, _, _ = _kit()
    cast, _loco, vfx, _g, _c = _sprites()
    W, H = 1500, 880
    im = Image.new("RGB", (W, H), c.BG)
    d = ImageDraw.Draw(im)
    tone = {"cast_gather": (90, 140, 255), "cast_release": (255, 80, 110), "cast_follow": (120, 200, 120)}
    c.label(d, (14, 10), "ANIM-DONUT-03 layout, pivot and timing. Pillow diagram from the committed grids, not a Bevy capture.", c.WHITE, 14)
    c.label(d, (14, 40), "A. donut_h40_missile atlas: 48 frames, 72 x 44, pivot (46, 43). Atlas frame = 16 * phase + locomotion frame",
            c.YELLOW, 12)
    cw, ch, x0, y0 = 84, 40, 130, 66
    for n, name in enumerate(LOCO_NAMES):
        c.label(d, (x0 + n * cw + 4, y0), name, c.GREY, 10)
    for r, phase in enumerate(PHASES):
        y = y0 + 16 + r * (ch + 8)
        c.label(d, (14, y + 12), phase, tone[phase], 12)
        for n in range(16):
            idx = 16 * r + n
            x = x0 + n * cw
            key = idx in CAST_KEYS
            d.rectangle([x, y, x + cw - 6, y + ch], outline=tone[phase], width=1, fill=tone[phase] if key else (36, 34, 50))
            c.label(d, (x + 4, y + 4), str(idx), c.BG if key else c.WHITE, 12)
            if key:
                c.label(d, (x + 4, y + 22), "KEY", c.BG, 10)
    c.label(d, (14, y0 + 16 + 3 * (ch + 8)),
            "Body, legs and tail cels are the locomotion frame's; only head, face and tiara change. Keys (filled): 0, 16, 24, 28. "
            "Frames 1-15 and 17-31: the key heads translated by the head offset; 32-47: the follow head.", c.GREY, 11)
    by = 290
    c.label(d, (14, by), "B. pivot and bolt spawn (cast frame 16 with the launch bolt, idle base, 6x). Yellow = pivot (46, 43) on the walking line; "
            "cyan cross = bolt pivot at (+27, -19)", c.YELLOW, 12)
    k = 6
    base = c.cell(cast[16], k, PIVOT, 43)
    panel = Image.new("RGBA", (base.width + 30 * k, base.height), c.CELL_BG + (255,))
    panel.paste(base, (0, 0))
    sx = (PIVOT[0] + 27) * k - VPIVOT[0] * k
    sy = (PIVOT[1] - 19) * k - VPIVOT[1] * k
    panel.alpha_composite(c.up(vfx[FLIGHT_PLAYBACK["launch"][0]], k), (sx, sy))
    pd = ImageDraw.Draw(panel)
    cx, cy = (PIVOT[0] + 27) * k + k // 2, (PIVOT[1] - 19) * k + k // 2
    pd.line([(cx - 10, cy), (cx + 10, cy)], fill=c.CYAN)
    pd.line([(cx, cy - 10), (cx, cy + 10)], fill=c.CYAN)
    im.paste(panel.convert("RGB"), (14, by + 22))
    px = 14 + panel.width + 24
    sp = [spawn_offset(n) for n in (0, 8, 12)]
    for i, line in enumerate((
            "Canvas 72 x 44, facing right; mirror around the pivot column for left.",
            "Spawn offset by locomotion frame = (27 + head dx, -19 + head dy), thrust included:",
            f"  idle {tuple(sp[0])}   run suspension 8 {tuple(sp[1])}   jump rise 12 {tuple(sp[2])}",
            "The cast never moves the feet: atlas frame 16 * p + n replaces locomotion frame n.",
            "VFX cell 32 x 32, pivot (16, 16); bolt head about 3 px ahead of the pivot.",
            "Hitbox placeholder 8 x 6 at (+4, 0) from the VFX pivot.")):
        c.label(d, (px, by + 30 + i * 22), line, c.WHITE, 12)
    ty = 650
    c.label(d, (14, ty), "C. timing (60 Hz). Cast-local ticks; the locomotion clock keeps running, only the atlas row swaps", c.YELLOW, 12)
    tx0, tw = 130, 56
    for t in range(0, 19):
        d.line([(tx0 + t * tw, ty + 36), (tx0 + t * tw, ty + 98)], fill=(60, 58, 80))
        c.label(d, (tx0 + t * tw - 4, ty + 22), str(t), c.GREY, 10)
    for phase in PHASES:
        a, b = PHASE_TICKS[phase]
        d.rectangle([tx0 + a * tw, ty + 40, tx0 + (b + 1) * tw - 2, ty + 66], fill=tone[phase])
        c.label(d, (tx0 + a * tw + 6, ty + 46), f"{phase} ({b - a + 1} t)", c.BG, 12)
    for tick, col, text, dx in ((RELEASE_TICK, c.YELLOW, "tick 4: bolt spawns", 10), (REFIRE_TICK, c.CYAN, "tick 16: loco again; re-fire (placeholder)", 10)):
        xx = tx0 + tick * tw
        d.polygon([(xx, ty + 70), (xx - 7, ty + 84), (xx + 7, ty + 84)], fill=col)
        c.label(d, (xx + dx, ty + 72), text, col, 11)
    c.label(d, (14, ty + 46), "cast_*", c.WHITE, 12)
    vy0 = ty + 112
    names = ["F0", "F1 launch", "F2", "F3", "I4 flash", "I5 peak", "I6", "I7 fade", "I8"]
    for row, (lo, hi, title) in enumerate(((0, 4, "missile_flight (loop)"), (4, 9, "missile_impact (once)"))):
        y = vy0 + row * 34
        c.label(d, (14, y + 6), title, c.WHITE, 11)
        x = tx0
        for n in range(lo, hi):
            wdt = VTICKS[n] * tw
            d.rectangle([x, y, x + wdt - 2, y + 26], fill=(255, 127, 208) if n < 4 else (226, 48, 159))
            c.label(d, (x + 3, y + 6), f"{names[n]} {VTICKS[n]}t", c.BG, 10)
            x += wdt
    c.label(d, (tx0, vy0 + 72), "missile_flight: launch frame 1 once, then the loop 2, 3, 0 (4 t each, 6 px/t); missile_impact plays once, 2 + 3 + 4 + 4 + 5 = 18 t "
            "(both rows drawn at the same tick scale as the cast bars)", c.GREY, 11)
    out.parent.mkdir(parents=True, exist_ok=True)
    im.save(out)
    print(f"wrote {out.relative_to(ROOT)} {im.size}")


def evidence() -> None:
    bases_sheet(EVIDENCE / "bases_x4.png")
    cast_scenes(EVIDENCE)
    cast_sequence_gif(EVIDENCE / "cast_sequence_x2.gif")
    layout_diagram(EVIDENCE / "layout_pivot_timing.png")
    key_scale(EVIDENCE / "keys-scale.png")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "key-scale", "evidence"))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    if a.mode == "key-scale":
        key_scale(EVIDENCE / "keys-scale.png")
        return 0
    if a.mode == "evidence":
        evidence()
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
