#!/usr/bin/env python3
"""Authoring record for BOS-ART: the chain-weight prop of the Ball of Swine's arena (room 18, ``bos``).

Route B (an easy prop, owner pipeline): the key is drafted here by the medium tier on ``floor01-catacomb-v2``
(no new colour), then gated, and frozen if need be, by the top tier. Four pieces share one atlas:

- ``bos_weight_hung``    80 x 36, the weight while it hangs: an iron slab 80 x 24 (the sim's rack footprint),
                         a brass collar and a shackle on top. Floor contact row = the slab's bottom row.
- ``bos_weight_landed``  80 x 36, the weight once it dropped: the same slab, the chain lying in a heap on it.
- ``bos_chain_v``        8 x 16, a vertical chain tile (two links); the presenter stacks it from the shackle
                         up to the ceiling girder (the chain lengthens while the weight falls).
- ``bos_chain_h``        16 x 8, the same chain on its side: the tie to the rotten post.

Plain and small: iron, chain, brass. No text, no faces, no symbols. Every pixel comes from rules (shapes, ramps,
a hash noise); nothing is traced or resampled. Light from the upper left.

    python3 scripts/author_bosart.py            # grid + .ase + manifest, key pins
    python3 scripts/author_bosart.py keys       # the key pins only
    python3 scripts/author_bosart.py measure    # the key measurements
    python3 scripts/author_bosart.py gate OUT   # the unlit x4 + 1x preview PNG
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_floor01_kit as kit  # noqa: E402  (kit palette ramps and the grid writer)

RAMPS = kit.RAMPS
SRC = ROOT / "assets/source/props/floor01"
STEM = "bos_props"
W, H = 80, 36            # the weight frames: the slab is the sim's 80 x 24 rack footprint
SLAB_Y0 = 12             # slab rows 12..35
COLLAR = (30, 49)        # collar columns


def rnd(*key) -> float:
    h = hashlib.sha256("|".join(map(str, ("bosart",) + key)).encode()).digest()
    return h[0] / 255.0


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


class Pix:
    """A piece canvas: (ramp, index) per pixel, ``None`` transparent."""

    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.px = [[None] * w for _ in range(h)]

    def put(self, x, y, ramp, v):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = (ramp, int(clamp(round(v), 0, len(RAMPS[ramp]) - 1)))

    def get(self, x, y):
        return self.px[y][x] if 0 <= x < self.w and 0 <= y < self.h else None

    def box(self, x0, y0, x1, y1, ramp, v):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.put(x, y, ramp, v)


# --------------------------------------------------------------------------------------------- the slab
def _slab(c: Pix) -> None:
    """The cool iron slab, rows 12..35 over the full width: chamfered top corners, a lit top arris, a dark underside row,
    two brass straps (BOS-ART-AD12: ``stone_cold`` greys, so it reads as iron, not as a wooden chest)."""
    for y in range(SLAB_Y0, H):
        inset = {SLAB_Y0: 3, SLAB_Y0 + 1: 1}.get(y, 0)
        for x in range(inset, W - inset):
            v = 2
            if y == SLAB_Y0 + 1:
                v = 4                                              # the lit top arris
            elif y == SLAB_Y0 + 2:
                v = 3
            elif y >= H - 8:
                v = 1                                              # the underside falls into shade
            if x >= W - 4:
                v -= 1                                             # the right edge turns away from the light
            if x < 3:
                v += 1                                             # the left edge catches it
            if v == 2 and rnd("grain", x // 2, y) > 0.9:
                v = 1                                              # cast-iron pitting
            elif v == 2 and rnd("glint", x, y) > 0.97:
                v = 3
            c.put(x, y, "stone_cold", v)
        c.put(inset, y, "stone_cold", 1)                           # the outline, left and right
        c.put(W - 1 - inset, y, "stone_cold", 0)
    for x in range(3, W - 3):
        c.put(x, SLAB_Y0, "stone_cold", 4)                         # the cut top face
    for x in range(W):
        c.put(x, H - 1, "stone_cold", 0)                           # the floor contact row, the darkest
    for x in range(20, 60):
        c.put(x, SLAB_Y0 + 12, "stone_cold", 1)                    # a cast seam
    for (x0, x1) in ((14, 19), (60, 65)):                          # brass straps
        for y in range(SLAB_Y0 + 2, H - 1):
            for x in range(x0, x1 + 1):
                v = 4 if x == x0 + 1 else (5 if x == x0 else (2 if x == x1 else 3))
                if y >= H - 6:
                    v -= 1
                c.put(x, y, "brass", v)
        c.put(x0 + 2, SLAB_Y0 + 6, "brass", 6)                     # strap rivets
        c.put(x0 + 2, SLAB_Y0 + 18, "brass", 5)
    for (x, y) in ((5, SLAB_Y0 + 5), (W - 6, SLAB_Y0 + 5), (5, H - 6), (W - 6, H - 6)):
        c.put(x, y, "stone_cold", 4)                               # corner rivets
        c.put(x, y + 1, "stone_cold", 0)


def _collar(c: Pix, x0: int = COLLAR[0], y0: int = 4) -> None:
    """The brass collar (20 x 8) the shackle rises from, sitting on the slab's top face."""
    x1 = x0 + 19
    for y in range(y0, y0 + 8):
        inset = 2 if y == y0 else 0
        for x in range(x0 + inset, x1 + 1 - inset):
            v = 4
            if y == y0:
                v = 6
            elif y == y0 + 1:
                v = 5
            elif y >= y0 + 6:
                v = 2
            if x >= x1 - 2:
                v -= 1
            c.put(x, y, "brass", v)
    c.box(x0 + 3, y0 + 3, x0 + 4, y0 + 4, "iron", 2)               # two bolt heads
    c.box(x1 - 4, y0 + 3, x1 - 3, y0 + 4, "iron", 2)
    c.put(x0 + 3, y0 + 3, "iron", 5)
    c.put(x1 - 4, y0 + 3, "iron", 5)


def _shackle(c: Pix, x0: int = 36, y0: int = 0) -> None:
    """The iron ring above the collar: 8 wide, 6 tall, hollow."""
    for x in range(x0 + 1, x0 + 7):
        c.put(x, y0, "iron", 5)
        c.put(x, y0 + 5, "iron", 2)
    for y in range(y0 + 1, y0 + 5):
        c.put(x0, y, "iron", 4)
        c.put(x0 + 1, y, "iron", 3)
        c.put(x0 + 6, y, "iron", 2)
        c.put(x0 + 7, y, "iron", 1)


def weight_hung() -> Pix:
    c = Pix(W, H)
    _slab(c)
    _collar(c)
    _shackle(c)
    return c


def _link(c: Pix, cx: float, cy: float, rx: float, ry: float, lit: int = 6, dark: int = 3) -> None:
    """A hollow oval link, one to two pixels thick, lit from the upper left."""
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            d = ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2
            if 0.55 <= d <= 1.15:
                up_left = (x + 0.5 - cx) + (y + 0.5 - cy) < 0
                c.put(x, y, "iron", lit if up_left else dark)


def weight_landed() -> Pix:
    """The same slab, dropped: the chain lies in a heap of links on the top face and the collar rests tipped."""
    c = Pix(W, H)
    _slab(c)
    spots = [(30, 9, 5.0, 3.0), (38, 6, 4.5, 3.0), (46, 9, 5.0, 3.0), (24, 6, 4.0, 2.5), (53, 6, 4.0, 2.5),
             (34, 10, 4.0, 2.0), (42, 10, 4.0, 2.0), (28, 3, 3.5, 2.2), (49, 3, 3.5, 2.2), (39, 3, 3.5, 2.0)]
    for (cx, cy, rx, ry) in spots:
        _link(c, cx, cy, rx, ry)
    _collar(c, 56, 4)                                              # the collar thrown to one side, on its face
    for x in range(56, 76):
        c.put(x, 11, "stone_cold", 0)                               # the heap's shadow on the slab's top face
    return c


def chain_v() -> Pix:
    """A vertical chain tile, 8 x 16: a face-on link (rows 0..9) and an edge-on link (rows 8..17, wrapped)."""
    c = Pix(8, 16)
    _link(c, 4.0, 5.0, 4.0, 5.0, lit=6, dark=3)
    for y in range(8, 18):
        yy = y % 16
        c.put(3, yy, "iron", 6)
        c.put(4, yy, "iron", 4)
    return c


def chain_h() -> Pix:
    """The chain on its side, 16 x 8 (the tie to the post): the vertical tile transposed, re-lit."""
    v = chain_v()
    c = Pix(16, 8)
    for y in range(v.h):
        for x in range(v.w):
            p = v.px[y][x]
            if p:
                c.px[x][y] = p
    return c


PIECES = [("bos_weight_hung", weight_hung), ("bos_weight_landed", weight_landed),
          ("bos_chain_v", chain_v), ("bos_chain_h", chain_h)]


def _framed(img: Pix) -> Pix:
    """Every frame of the grid shares the 80 x 36 canvas; the small tiles sit at its top left."""
    out = Pix(W, H)
    for y in range(img.h):
        for x in range(img.w):
            out.px[y][x] = img.px[y][x]
    return out


def key_sha() -> dict:
    """The key pins: SHA-256 of each piece's pixel list (the execution's byte-identity check)."""
    return {pid: hashlib.sha256(json.dumps(fn().px).encode()).hexdigest() for pid, fn in PIECES}


def write() -> None:
    grid = kit.grid_from([_framed(fn()) for _, fn in PIECES], layer_names=["main"],
                         durations=[100] * len(PIECES), pivot=(0, 0))
    path = SRC / f"{STEM}.grid.json"
    path.write_text(json.dumps(grid, indent=1) + "\n", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "pixel_grid_to_ase.py"), "build", str(path),
                    str(SRC / f"{STEM}.ase")], check=True)
    manifest = {
        "schema_version": 1,
        "id": "floor01_bos_props_v1",
        "type": "decor_atlas",
        "source_path": f"assets/source/props/floor01/{STEM}.ase",
        "grid_source": f"assets/source/props/floor01/{STEM}.grid.json",
        "output_atlas": "generated/art/floor01_bos_props.png",
        "output_metadata": "generated/art/floor01_bos_props.json",
        "palette_id": "floor01-catacomb-v2",
        "frame_size": [W, H],
        "pivot": [0, 0],
        "animation_tags": [],
        "required_alpha": True,
        "kit_contract": "assets/source/terrain/floor01/floor01_bos_kit.json",
        "owner": "AI-authored environment pixel art (BOS-ART: chain-weight prop key drafted by Claude Sonnet, route B; "
                 "gate and independent review Claude Opus)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "license": "Original repo-specific environment art; no standalone asset reuse license; "
                   "no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "provenance": {
            "origin": "The chain weight (hung, landed) and the chain tiles of the Ball of Swine arena (BOS-ART): "
                      "iron slab, brass collar and straps, iron chain; nothing cropped, traced or resampled from "
                      "concept art; no text, logos, brands, faces or likenesses",
            "generator": "Deterministic authoring script scripts/author_bosart.py written by an AI agent "
                         "(route B: key drafted by Claude Sonnet; gate and review by Claude Opus); "
                         "no image-model raster output",
            "generated_art": True,
            "input_reference_ids": [
                "floor01-catacomb-kit/env-kit-v2 (owner-approved ENV-01 kit, 2026-11-16)",
                "floor-01-ball-of-swine-arena-concept (mood reference only; no pixel cropped, traced or resampled)",
            ],
        },
    }
    (ROOT / "assets/manifests/floor01_bos_props.json").write_text(json.dumps(manifest, indent=1) + "\n",
                                                                 encoding="utf-8")
    print(f"{STEM}: {[pid for pid, _ in PIECES]} {grid['canvas']} {len(grid['palette'])} colours")
    for pid, sha in key_sha().items():
        print(f"KEY {pid} {sha}")


def _luma(ramp: str, v: int) -> float:
    h = RAMPS[ramp][v]
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def _bbox(img: Pix):
    opaque = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x]]
    xs, ys = [p[0] for p in opaque], [p[1] for p in opaque]
    return (min(xs), min(ys), max(xs), max(ys)), len(opaque)


CONTRACT = ROOT / "assets/source/terrain/floor01/floor01_bos_kit.json"
FLOOR = 420                 # the walking line (layout y)
ALCOVES = ((28, 110), (530, 608))
LANE = (111, 529)


def placements() -> list[dict]:
    """The room's `bos` dressing placements with each piece's rect, from the contract (and the kits it extends)."""
    out = []
    rects = {}
    kits = [ROOT / "assets/source/terrain/floor01" / n for n in (
        "floor01_catacomb_kit.json", "floor01_opening_kit.json", "floor01_gym_kit.json", "floor01_bos_kit.json")]
    for k in kits:
        for p in json.loads(k.read_text(encoding="utf-8"))["pieces"]:
            rects[p["id"]] = p["rect"]
    block = json.loads(CONTRACT.read_text(encoding="utf-8"))["room_dressing"]["bos"]
    for pl in block["place"]:
        r = rects[pl["piece"]]
        out.append({**pl, "w": r[2], "h": r[3]})
    return out


def dressing_measurements() -> list[str]:
    """The placement boxes the spec asks for: the posts and lintels on the gameplay rects, the torches clear of the
    alcoves, the lane and the weight, the girder over the weight's chain."""
    pl = placements()
    out = []

    def of(name):
        return [p for p in pl if p["piece"] == name]

    for cx in (104, 152, 488):
        parts = {n: [p for p in of(n) if abs(p["x"] + p["w"] // 2 - cx) <= 2 or abs(p["x"] + 2 - cx) <= 16] for n in
                 ("bos_post_cap", "bos_post_shaft", "bos_post_base")}
        cap = min(parts["bos_post_cap"], key=lambda p: abs(p["x"] + p["w"] / 2 - cx))
        shaft = min(parts["bos_post_shaft"], key=lambda p: abs(p["x"] + p["w"] / 2 - cx))
        base = min(parts["bos_post_base"], key=lambda p: abs(p["x"] + p["w"] / 2 - cx))
        out.append(f"post c={cx}: cap y {cap['y']}..{cap['y'] + cap['h']} shaft x {shaft['x']}..{shaft['x'] + shaft['w']} "
                   f"y {shaft['y']}..{shaft['y'] + shaft['h']} base y {base['y']}..{base['y'] + base['h']} "
                   f"(floor {FLOOR}; gameplay post {cx - 8}..{cx + 8}, top {FLOOR - 112})")
    lint = sorted(of("bos_lintel"), key=lambda p: p["x"])
    out.append("lintels " + " ".join(f"x {p['x']}..{p['x'] + p['w']} y {p['y']}..{p['y'] + p['h']}" for p in lint)
               + f" (gameplay lintels 96..160 and 480..544 at y {FLOOR - 112})")
    for t in of("torch_wall"):
        flame_bottom = FLOOR - (t["y"] + 16)
        out.append(f"torch x {t['x']}..{t['x'] + t['w']} y {t['y']}..{t['y'] + t['h']}: flame bottom {flame_bottom} px over "
                   f"the floor (ball 134, Donut's jump 138); in an alcove: "
                   f"{any(a <= t['x'] + 8 <= b for a, b in ALCOVES)}")
    g = of("gym_girder")
    out.append(f"girder x {min(p['x'] for p in g)}..{max(p['x'] + p['w'] for p in g)} y {g[0]['y']}..{g[0]['y'] + g[0]['h']} "
               f"(underside {FLOOR - (g[0]['y'] + g[0]['h'])} over the floor = the sim ceiling 224; weight chain x 205)")
    pipes = [p for p in pl if p["piece"].startswith("pipe_v")]
    out.append(f"copper run x {pipes[0]['x']}..{pipes[0]['x'] + 16} y {pipes[0]['y']}..{pipes[-1]['y'] + 16} "
               f"(the gate is 0..28, the post 52..92)")
    out.append(f"tubes {[(p['x'], p['y']) for p in of('gym_fluoro_accent')]} (limited UV: {2 * len(of('gym_fluoro_accent'))} lights)")
    return out


def measurements() -> list[str]:
    out = []
    pal = set()
    for pid, fn in PIECES:
        img = fn()
        cols = {p for row in img.px for p in row if p}
        pal |= cols
        (x0, y0, x1, y1), n = _bbox(img)
        out.append(f"{pid} {img.w}x{img.h} opaque={n} bbox=({x0},{y0})-({x1},{y1}) colours={len(cols)} "
                   f"max_luma={max(_luma(*c) for c in cols):.0f} transparent_px={img.w * img.h - n}")
    for pid, fn in PIECES[:2]:
        img = fn()
        slab = [(x, y) for y in range(SLAB_Y0, H) for x in range(W) if img.px[y][x]]
        out.append(f"{pid} slab rows {SLAB_Y0}..{H - 1} = {H - SLAB_Y0} px tall x {W} wide "
                   f"(sim rack footprint 80 x 24): floor contact row {_bbox(img)[0][3]} == {H - 1}: "
                   f"{'ok' if _bbox(img)[0][3] == H - 1 else 'FAIL'}; slab opaque {len(slab)}")
    out.append(f"all pieces: {len(pal)} distinct palette entries (per-atlas budget 63); ramps used "
               f"{sorted({p[0] for p in pal})}")
    out += [f"KEY {pid} sha256={sha}" for pid, sha in key_sha().items()]
    out += ["DRESSING " + line for line in dressing_measurements()]
    return out


def to_image(img: Pix):
    from PIL import Image
    out = Image.new("RGBA", (img.w, img.h), (0, 0, 0, 0))
    for y in range(img.h):
        for x in range(img.w):
            p = img.px[y][x]
            if p is None:
                continue
            h = RAMPS[p[0]][p[1]]
            out.putpixel((x, y), (int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16), 255))
    return out


def gate_preview(out_path: Path) -> None:
    """Unlit x4 and 1x of every piece on a flat catacomb-brick-dark ground (no lighting pass)."""
    from PIL import Image
    ground = tuple(int(RAMPS["brick"][3][i:i + 2], 16) for i in (1, 3, 5)) + (255,)
    ims = {pid: to_image(fn()) for pid, fn in PIECES}
    pad = 8
    sheet = Image.new("RGBA", (80 * 4 * 2 + pad * 3, 36 * 4 + 16 * 4 + pad * 3 + 80 + 36 + pad * 2), ground)
    # row 1, x4: hung and landed
    for k, pid in enumerate(("bos_weight_hung", "bos_weight_landed")):
        im = ims[pid]
        sheet.alpha_composite(im.resize((im.width * 4, im.height * 4), Image.NEAREST), (pad + k * (320 + pad), pad))
    # row 2, x4: the two chain tiles
    y2 = 36 * 4 + 2 * pad
    for k, pid in enumerate(("bos_chain_v", "bos_chain_h")):
        im = ims[pid]
        sheet.alpha_composite(im.resize((im.width * 4, im.height * 4), Image.NEAREST), (pad + k * 60, y2))
    # row 3, 1x: the hung weight under four chain tiles, and the landed weight
    y3 = y2 + 16 * 4 + pad
    for k in range(5):
        sheet.alpha_composite(ims["bos_chain_v"], (pad + 36, y3 + k * 16))
    sheet.alpha_composite(ims["bos_weight_hung"], (pad, y3 + 80))
    sheet.alpha_composite(ims["bos_weight_landed"], (pad + 80 + pad, y3 + 80))
    sheet.save(out_path)
    print(f"gate preview {sheet.size} -> {out_path}")


def main(argv: list[str]) -> None:
    cmd = argv[1] if len(argv) > 1 else "all"
    if cmd == "keys":
        for pid, sha in key_sha().items():
            print(f"KEY {pid} {sha}")
    elif cmd == "measure":
        print("\n".join(measurements()))
    elif cmd == "gate":
        gate_preview(Path(argv[2]))
    else:
        write()


if __name__ == "__main__":
    main(sys.argv)
