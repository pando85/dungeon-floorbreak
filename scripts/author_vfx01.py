#!/usr/bin/env python3
"""VFX-01 authoring record: bomb in flight and explosion burst, shared by the Goblin Bomb Bard and the Rot Sticker.

Usage:

    python3 scripts/author_vfx01.py keys [--check]
    python3 scripts/author_vfx01.py key-scale [--out PNG]
    python3 scripts/author_vfx01.py keys-x3 [--out PNG]

``keys`` (re)writes ``assets/source/vfx/explosion/vfx_01.grid.json`` and ``assets/manifests/vfx-01.json``
(see ``docs/02-art/vfx-01-spec.md`` sections 3 to 8). Frames 0 (bomb flight), 3 (burst flash), 5 (burst peak)
and 10 (smoke fade) are drawn natively from rules plus a fixed-seed integer hash (no floating-point
randomness, no external RNG); every other frame is a literal copy of the key it follows
(``COPIES``) until the executor draws real in-betweens. ``--check`` compares instead of writing
(exit 1 on a difference). ``key-scale`` writes ``docs/evidence/vfx-01/key-scale.png``.

No concept pixels are traced or sampled. Presentation-only art tooling; nothing here is wired into the game.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = ROOT / "assets/source/vfx/explosion"
GRID = SRC_DIR / "vfx_01.grid.json"
MANIFEST = ROOT / "assets/manifests/vfx-01.json"
CANVAS = (48, 48)
PIVOT = (24, 24)
CX = CY = 24.0
TICK_HZ = 60

# vfx-floor01-v0 (16 of 20). Iron, brass and the fuse spark are the Bard's keys (same hex); the ember
# ramp m r o g G w is the Rot Sticker's flame ramp (same hex). The three smoke keys are the Rot shell
# ramp's cool violets, so smoke and iron stay one cold family against the warm torch light.
PALETTE = [
    ("K", "outline_soot", "#1b1124ff", False),
    ("q", "iron_dark", "#3a3245ff", False),
    ("Q", "iron_light", "#766c86ff", False),
    ("n", "brass_dark", "#6a4a1eff", False),
    ("b", "brass_mid", "#b8862eff", False),
    ("h", "brass_light", "#f0cf6aff", True),
    ("e", "eye_yellow_spark_halo", "#ffd23aff", True),
    ("Y", "spark_core", "#fff4b8ff", True),
    ("s", "smoke_deep", "#2b2230ff", False),
    ("c", "smoke_mid", "#3f3243ff", False),
    ("m", "ember_dead", "#5a1218ff", False),
    ("r", "ember_red", "#a51418ff", True),
    ("o", "glow_ember", "#ec3a1cff", True),
    ("g", "glow_hot", "#ff8a3cff", True),
    ("G", "glow_core", "#ffdc8aff", True),
    ("w", "flash_white", "#efe3d3ff", True),
]
KEYS = [p[0] for p in PALETTE]
ACCENT_KEYS = {p[0] for p in PALETTE if p[3]}

# Frame table: tag, 60 Hz ticks. Atlas frames 0-2 bomb_flight, 3-10 burst.
TAGS = [("bomb_flight", 0, 2), ("burst", 3, 10)]
TICKS = [4, 4, 4, 2, 3, 4, 4, 5, 6, 8, 10]
KEY_FRAMES = (0, 3, 5, 10)
COPIES = {1: 0, 2: 0, 4: 3, 6: 5, 7: 5, 8: 5, 9: 5}   # in-between slot -> key it copies for now
DAMAGE_ACTIVE_TICKS = (0, 8)                          # placeholder, burst-local ticks 0..8 inclusive (frames 3-5)


# --------------------------------------------------------------------------- deterministic noise
def h2(x: int, y: int, seed: int) -> int:
    n = (x * 374761393 + y * 668265263 + seed * 2147483647) & 0xFFFFFFFF
    n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
    return (n ^ (n >> 16)) & 0xFFFF


def lump(theta: float, seed: int, amp: float, lobes: int = 7) -> float:
    """Radius multiplier around the circle: a few irregular lobes, same value every run."""
    v = 0.0
    for k in range(3):
        f = lobes + 3 * k
        ph = (h2(k, seed, 7) % 628) / 100.0
        v += math.sin(theta * f + ph) / (1 + k)
    return 1.0 + amp * v / 1.8


class Canvas:
    def __init__(self) -> None:
        self.w, self.h = CANVAS
        self.px = [["." for _ in range(self.w)] for _ in range(self.h)]

    def put(self, x: int, y: int, k: str) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = k

    def get(self, x: int, y: int) -> str:
        return self.px[y][x] if 0 <= x < self.w and 0 <= y < self.h else "."

    def stamp(self, x0: int, y0: int, rows: list[str]) -> None:
        for dy, r in enumerate(rows):
            for dx, ch in enumerate(r):
                if ch != ".":
                    self.put(x0 + dx, y0 + dy, ch)

    def blob(self, cx: float, cy: float, rad: float, seed: int, amp: float, pick, lobes: int = 7) -> None:
        """Lumpy disc; ``pick(t, dx, dy, x, y)`` maps normalised radius (0..1) to a key or None."""
        for y in range(self.h):
            for x in range(self.w):
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                d = math.hypot(dx, dy)
                lim = rad * lump(math.atan2(dy, dx), seed, amp, lobes)
                if d <= lim:
                    k = pick(d / lim, dx, dy, x, y)
                    if k:
                        self.put(x, y, k)

    def cleanup(self) -> None:
        """Drop isolated pixels (no 8-neighbour), so debris and embers are drawn as 2 px chunks only."""
        kill = []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] == ".":
                    continue
                if all(self.get(x + i, y + j) == "." for i in (-1, 0, 1) for j in (-1, 0, 1) if i or j):
                    kill.append((x, y))
        for x, y in kill:
            self.px[y][x] = "."

    def despeckle(self, passes: int = 3) -> None:
        """Recolour lint orphans (a non-accent key with no same-key 8-neighbour) to the commonest neighbour key,
        so smoke and debris read as clumps, not single-pixel noise. Accents (fire, sparks) are left alone."""
        for _ in range(passes):
            changes = []
            for y in range(self.h):
                for x in range(self.w):
                    k = self.px[y][x]
                    if k == "." or k in ACCENT_KEYS:
                        continue
                    nb = [self.get(x + i, y + j) for i in (-1, 0, 1) for j in (-1, 0, 1) if i or j]
                    nb = [n for n in nb if n != "."]
                    if nb and k not in nb:
                        changes.append((x, y, max(sorted(set(nb)), key=nb.count)))
            for x, y, k in changes:
                self.px[y][x] = k
            if not changes:
                break

    def rows(self) -> list[str]:
        return ["".join(r) for r in self.px]


# --------------------------------------------------------------------------- bomb flight key (frame 0)
BOMB7 = [   # the Bard's iron bomb (docs/02-art/bard-01-spec.md section 6), brass fuse cap top centre
    "..KbK..",
    ".KQqqK.",
    "KQqqqqK",
    "KqqqqqK",
    "KqqqqqK",
    ".KqqqK.",
    "..KKK..",
]
SPARK = [   # lit fuse spark, 5 x 5: Y core, e halo, R-class ember points
    "..e..",
    ".eYe.",
    "eYYYe",
    ".eYe.",
    "..e.o",
]


def bomb_flight() -> list[str]:
    c = Canvas()
    for y, x0, n in ((22, 8, 8), (26, 11, 6), (19, 13, 4)):   # three soot-violet motion streaks behind (left)
        for x in range(x0, x0 + n):
            c.put(x, y, "c" if (x - x0) % 4 != 3 else "s")
    c.stamp(21, 21, BOMB7)                                    # body centred on the pivot (24, 24)
    for x, y in ((24, 20), (25, 19), (25, 18), (26, 17)):     # fuse cord curling up and forward
        c.put(x, y, "n")
    c.stamp(25, 12, SPARK)                                    # spark crown on the fuse tip (27, 14)
    for x, y, k in ((24, 14, "o"), (23, 14, "o"), (22, 15, "m"), (21, 15, "m")):   # fuse sparks streaming
        c.put(x, y, k)                                                              # behind, attached
    c.cleanup()
    return c.rows()


# --------------------------------------------------------------------------- burst keys
def fire_pick(core: float, hot: float, mid: float, red: float, white: float = 0.0):
    def pick(t, dx, dy, x, y):
        t = t + ((h2(x, y, 3) % 100) / 100.0 - 0.5) * 0.10         # grit on the band edges
        if t < white:
            return "w"
        if t < core:
            return "G"
        if t < hot:
            return "g"
        if t < mid:
            return "o"
        if t < red:
            return "r"
        return "m"
    return pick


def chunk(c: Canvas, x: int, y: int, key: str, horizontal: bool = True) -> None:
    c.put(x, y, key)
    c.put(x + 1, y, key) if horizontal else c.put(x, y + 1, key)


def burst_flash() -> list[str]:
    c = Canvas()
    # Design edit: no radial spikes (they read as a cartoon star). Three uneven fire tongues bulge out of a
    # compressed core, and the outward read comes from dark iron and stone debris thrown at different distances.
    for cx, cy, r, sd in ((17.5, 19.5, 4.0, 12), (30.5, 17.5, 3.5, 13), (29.5, 30.0, 3.0, 14)):
        c.blob(cx, cy, r, sd, 0.20, fire_pick(0.0, 0.0, 0.45, 0.80, white=-1.0))
    c.blob(CX, CY, 8.5, 11, 0.20, fire_pick(0.34, 0.60, 0.84, 0.95, white=0.12), lobes=5)
    for x, y, k, hz in ((9, 13, "K", True), (37, 12, "q", True), (11, 33, "q", False), (36, 33, "K", False),
                        (22, 7, "K", False), (27, 39, "q", True), (6, 22, "q", True)):
        chunk(c, x, y, k, hz)
    for x, y in ((13, 9), (38, 26), (17, 37)):                          # hot grit thrown with the debris
        chunk(c, x, y, "o", True)
    c.cleanup()
    return c.rows()


def burst_peak() -> list[str]:
    c = Canvas()

    def smoke(t, dx, dy, x, y):
        if t > 0.80 and (x + y) % 2:                                   # ragged, see-through smoke edge
            return None
        up = -dy / 14.0                                                # lit from the upper left
        side = -dx / 14.0
        v = up * 0.6 + side * 0.4 + ((h2(x, y, 4) % 100) / 100.0 - 0.5) * 0.5
        return "Q" if v > 0.48 else "c" if v > 0.0 else "s"

    # Design edit: smoke rises (lobes sit above and beside the fire, none under it), the fireball is smaller
    # and lumpier so it reads as a burning cloud rather than a round ball, and the total mass is lighter.
    for cx, cy, r, sd in ((15, 15, 6.0, 21), (30, 11, 6.5, 22), (23, 8, 6.0, 25), (37, 21, 5.0, 23),
                          (11, 26, 4.5, 24)):
        c.blob(cx, cy, r, sd, 0.16, smoke)
    c.blob(CX, CY + 2, 11.5, 31, 0.18, fire_pick(0.24, 0.50, 0.76, 0.92), lobes=5)
    for cx, cy, r, sd in ((18, 14, 4.5, 27), (29, 13, 4.0, 28)):         # smoke rolls over the fire's crown
        c.blob(cx, cy, r, sd, 0.18, smoke)
    for x, y, k, hz in ((6, 9, "K", True), (41, 10, "q", False), (4, 24, "q", False), (40, 35, "K", True),
                        (9, 38, "K", True)):
        chunk(c, x, y, k, hz)
    for x, y in ((17, 39), (31, 40), (40, 28)):                         # embers: 2 px chunks, ember-red family
        chunk(c, x, y, "o", True)
    c.despeckle()
    c.cleanup()
    return c.rows()


def smoke_fade() -> list[str]:
    c = Canvas()

    def puff(t, dx, dy, x, y):
        v = (-dy - dx * 0.6) / 8.0 + ((h2(x, y, 6) % 100) / 100.0 - 0.5) * 0.45
        k = "Q" if v > 0.7 else "c" if v > -0.1 else "s"
        if t > 0.78 and (x + y) % 2:                                      # ragged edge: dither, never a solid rim
            return None
        return k

    for cx, cy, r, sd in ((20, 14, 7.0, 41), (29, 10, 6.0, 42), (16, 23, 6.0, 43), (31, 21, 5.5, 44),
                          (24, 29, 5.0, 45), (22, 6, 4.0, 46)):
        c.blob(cx, cy, r, sd, 0.16, puff)
    for x, y, k in ((14, 34, "o"), (30, 36, "m"), (35, 30, "o"), (12, 8, "m")):   # a few dying embers, 2 px
        chunk(c, x, y, k, True)
    for x, y in ((18, 38), (27, 40)):                                    # falling debris
        chunk(c, x, y, "q", True)
    c.despeckle()
    c.cleanup()
    return c.rows()


# --------------------------------------------------------------------------- grid, manifest
def key_rows() -> dict[int, list[str]]:
    return {0: bomb_flight(), 3: burst_flash(), 5: burst_peak(), 10: smoke_fade()}


def palette_json() -> list[dict]:
    out = []
    for key, name, rgba, accent in PALETTE:
        e = {"key": key, "name": name, "rgba": rgba}
        if accent:
            e["accent"] = True
        out.append(e)
    return out


def build_grid() -> dict:
    keys = key_rows()
    frames = []
    for i, t in enumerate(TICKS):
        rows = keys[i] if i in keys else keys[COPIES[i]]
        frames.append({"duration_ms": round(t * 1000 / TICK_HZ), "cels": {"body": rows}})
    return {"format": "pixel-grid-v1", "canvas": list(CANVAS), "pivot": list(PIVOT), "palette": palette_json(),
            "layers": ["body"], "frames": frames,
            "tags": [{"name": n, "from": a, "to": b, "direction": "forward"} for n, a, b in TAGS]}


def build_manifest() -> dict:
    tag_of = {n: name for name, a, b in TAGS for n in range(a, b + 1)}
    frames = [{"index": n, "tag": tag_of[n], "ticks": t, "duration_ms": round(t * 1000 / TICK_HZ),
               "author": "key" if n in KEY_FRAMES else "copy_placeholder"} for n, t in enumerate(TICKS)]
    return {
        "schema_version": 1,
        "id": "vfx_bomb_burst_48_candidate_v1",
        "type": "vfx_animation",
        "source_path": "assets/source/vfx/explosion/vfx_01.ase",
        "grid_source": "assets/source/vfx/explosion/vfx_01.grid.json",
        "output_atlas": "generated/art/vfx-01.png",
        "output_metadata": "generated/art/vfx-01.json",
        "palette_id": "vfx-floor01-v0",
        "frame_size": list(CANVAS),
        "pivot": list(PIVOT),
        "animation_tags": [n for n, _a, _b in TAGS],
        "required_alpha": True,
        "owner": "AI-authored VFX candidate (fan-concept layer: Goblin Bomb Bard bomb and Rot Sticker burst; runtime role IP-neutral thrown explosive and blast burst)",
        "tool_version": "LibreSprite=1.3 / Aseprite-compatible",
        "provenance": {
            "origin": ("VFX-01 draft: 4 key frames (atlas frames 0, 3, 5, 10, fixture tests/fixtures/vfx-01_keys.json) rasterized "
                       "natively by scripts/author_vfx01.py from rules and a fixed-seed integer hash; the other 7 frames are "
                       "literal copies of a key (placeholders); layer body"),
            "generator": "Draft spec and keys: Claude Sonnet; design edit and key freeze: Claude Opus; review pending. No image-model raster output, no tracing, no resampling",
            "generated_art": True,
            "input_reference_ids": ["assets/concept/floor-01/enemies/floor01-goblin-bomb-bard-concept-approved.png",
                                    "assets/concept/floor-01/enemies/scatterers-rot-sticker-slime-bestiary.png"],
        },
        "license": "Fan-concept art published under owner-reported scope ART-PUBLICATION-ALL-V1; no public reuse license; no game/asset-pack distribution rights",
        "review_status": "prototype_fixture_unapproved",
        "vfx": {
            "status": "Draft keys, copies for in-betweens; top-tier edit, freeze and review pending; not wired into any Bevy binary",
            "spec": "docs/02-art/vfx-01-spec.md",
            "tick_hz": TICK_HZ,
            "hold_preview_ms": 100,
            "free_tags": [n for n, _a, _b in TAGS],
            "playback": {"bomb_flight": "loop", "burst": "once"},
            "key_frames": list(KEY_FRAMES),
            "copy_frames": sorted(COPIES),
            "key_fixture": "tests/fixtures/vfx-01_keys.json",
            "damage_active_ticks_placeholder": list(DAMAGE_ACTIVE_TICKS),
            "frames": frames,
        },
    }


def dump(data: dict) -> str:
    return json.dumps(data, indent=1) + "\n"


def report(rows: list[str], label: str) -> None:
    n = sum(ch != "." for r in rows for ch in r)
    white = sum(r.count("w") for r in rows)
    core = sum(r.count("G") for r in rows)
    xs = [x for r in rows for x, ch in enumerate(r) if ch != "."]
    ys = [y for y, r in enumerate(rows) if r.strip(".")]
    print(f"{label}: {n} px, bbox x {min(xs)}..{max(xs)} y {min(ys)}..{max(ys)}, w {white}, G {core}")


# --------------------------------------------------------------------------- key-scale evidence
def key_scale(out: Path) -> None:
    sys.path.insert(0, str(ROOT / "scripts"))
    sys.path.insert(0, str(ROOT))
    from PIL import Image
    from scripts.pixelart.scene import Floor, reference_idle
    from scripts.pixelart.common import load_grid_file, grid_images, repo_path, flip

    grid = build_grid()
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}

    def img(rows):
        im = Image.new("RGBA", (len(rows[0]), len(rows)), (0, 0, 0, 0))
        for y, r in enumerate(rows):
            for x, ch in enumerate(r):
                if ch != ".":
                    im.putpixel((x, y), pal[ch])
        return im

    carl, carl_piv = reference_idle("carl")
    bard_grid = load_grid_file(repo_path("assets/source/characters/goblin/bard/bard_01.grid.json"))
    bard, bard_piv = grid_images(bard_grid)[10], tuple(bard_grid["pivot"])
    bard, bard_piv = flip(bard, bard_piv)
    fl = Floor()
    cels = [img(grid["frames"][i]["cels"]["body"]) for i in KEY_FRAMES]
    x0 = 452
    entries = [(carl, carl_piv, x0, 0), (bard, bard_piv, x0 + 56, 0)]
    for n, im in enumerate(cels):
        entries.append((im, PIVOT, x0 + 120 + n * 56, 34 if n == 0 else 20))
    # Player-visibility panels: the peak with Carl overlapping it. Decided order: burst drawn first (behind the
    # actors). Second panel, for comparison only: the burst drawn over Carl.
    xo = x0 + 120 + 4 * 56 + 8
    entries += [(cels[2], PIVOT, xo, 20), (carl, carl_piv, xo + 6, 0),
                (carl, carl_piv, xo + 64 + 6, 0), (cels[2], PIVOT, xo + 64, 20)]
    lit = fl.render(entries, 1)
    vx, vy = fl.view[0], fl.view[1]
    gy, ox = fl.ground(x0) - vy, x0 - vx
    crop = lit.crop((ox - 40, gy - 76, ox + 120 + 4 * 56 + 8 + 64 + 40, gy + 12))
    big = lit.crop((ox + 120 - 28, gy - 56, ox + 120 - 28 + 4 * 56, gy)).resize((4 * 56 * 4, 56 * 4), Image.NEAREST)
    sheet = Image.new("RGB", (max(crop.width, big.width), crop.height + big.height + 8), (20, 16, 24))
    sheet.paste(crop, (0, 0))
    sheet.paste(big, (0, crop.height + 8))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {out.relative_to(ROOT)} {sheet.size}")


def keys_x3(out: Path) -> None:
    """The four frozen keys at 3x on the dark Floor 1 brick value, 3x3 px gutters, no lighting."""
    from PIL import Image
    grid = build_grid()
    pal = {e["key"]: tuple(int(e["rgba"][i:i + 2], 16) for i in (1, 3, 5, 7)) for e in grid["palette"]}
    s, gap, bg = 3, 6, (38, 30, 40)
    sheet = Image.new("RGB", (len(KEY_FRAMES) * (48 * s + gap) + gap, 48 * s + 2 * gap), (20, 16, 24))
    for n, f in enumerate(KEY_FRAMES):
        cell = Image.new("RGB", (48, 48), bg)
        for y, r in enumerate(grid["frames"][f]["cels"]["body"]):
            for x, ch in enumerate(r):
                if ch != ".":
                    cell.putpixel((x, y), pal[ch][:3])
        sheet.paste(cell.resize((48 * s, 48 * s), Image.NEAREST), (gap + n * (48 * s + gap), gap))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(f"wrote {out.relative_to(ROOT)} {sheet.size}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("keys", "key-scale", "keys-x3"))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--out", type=Path, default=ROOT / "docs/evidence/vfx-01/key-scale.png")
    a = ap.parse_args(argv)
    if a.mode == "key-scale":
        key_scale(a.out)
        return 0
    if a.mode == "keys-x3":
        keys_x3(ROOT / "docs/evidence/vfx-01/keys-x3.png" if a.out.name == "key-scale.png" else a.out)
        return 0
    data = build_grid()
    for f in KEY_FRAMES:
        report(data["frames"][f]["cels"]["body"], f"frame {f:2d}")
    text, mtext = dump(data), json.dumps(build_manifest(), indent=2) + "\n"
    if a.check:
        if not GRID.is_file() or GRID.read_text() != text or not MANIFEST.is_file() or MANIFEST.read_text() != mtext:
            print("differs")
            return 1
        print("up to date")
        return 0
    GRID.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    GRID.write_text(text, encoding="utf-8")
    MANIFEST.write_text(mtext, encoding="utf-8")
    print(f"wrote {GRID.relative_to(ROOT)} and {MANIFEST.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
