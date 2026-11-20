#!/usr/bin/env python3
"""Authoring record for ART-DONUT-CF02: Princess Donut's CF02 acting sheet (docs/02-art/cf02-acting-spec.md, section 4).

Writes ``assets/source/characters/donut/donut_h40_cf02.grid.json`` (3 frames, 72 x 44, pivot (46, 43)):

* ``startle_puff`` and ``smug`` are the frozen ACT-DESIGN keys, copied pixel for pixel from
  ``assets/source/characters/donut/cf02_keys/donut_h40_cf02_keys.grid.json``.
* ``startle_settle`` (the one derived frame) is built from ``startle_puff`` and her approved idle frame with
  translations only (no resampling, no rotation, no new head, no image model):
  body 2 px back down (the upper block moves down 2 rows and 2 of the 4 stiff shaft rows of the legs are removed,
  so the body rides 1 px above idle), tail 2 px lower (still bottle-brushed, fur rim kept), pupils back to the idle
  size (the idle face cel, translated with the head), head, face and tiara 1 px toward the event side.
  The collar and charm travel with the body; the tiara travels with the head.

The 4th budget slot of the spec (Q73) is a reserve and is not drawn. Run ``python3 scripts/author_donut_cf02.py`` to
rewrite the grid (deterministic), ``--check`` to verify the committed grid, ``--evidence`` to render the scene and
timing evidence under ``docs/evidence/cf02-donut/``.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

KEYS = ROOT / "assets/source/characters/donut/cf02_keys/donut_h40_cf02_keys.grid.json"
LOCO = ROOT / "assets/source/characters/donut/donut_h40_locomotion.grid.json"
OUT = ROOT / "assets/source/characters/donut/donut_h40_cf02.grid.json"
EVIDENCE = ROOT / "docs/evidence/cf02-donut"
TICK_HZ = 60

# Frame plan (spec section 7): (tag, [(frame name, ticks)]). startle t0-5 puff, t6-11 settle; smug holds 20 ticks.
PLAN = [
    ("startle", [("startle_puff", 6), ("startle_settle", 6)]),
    ("smug", [("smug", 20)]),
]
# Derivation parameters (spec section 4, row 2). Head offset toward the event side: facing right is +x, the
# presenter mirrors the whole sprite with the facing (design question DQ-1 in the PR).
SETTLE_BODY_DOWN = 2          # puff -> settle, whole body block (rows above the legs' shaft)
SETTLE_TAIL_DOWN = 2          # puff -> settle, tail (still puffed)
SETTLE_HEAD_DX = 1            # head, face, tiara toward the event side
SHAFT_ROWS = (38, 39, 40, 41)  # the four identical stiff shaft rows of the puff legs (idle has one)
OUTLINE_CLOSURE = ((63, 26),)  # body layer K pixels that close the cheek/chest outline after the head shift
TIARA_MAX_ROW = 8              # gear pixels on or above this row are the tiara; below it the collar and charm


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def pix(cel: list[str]) -> dict:
    return {(x, y): c for y, row in enumerate(cel) for x, c in enumerate(row) if c != "."}


def shift(p: dict, dx: int, dy: int) -> dict:
    return {(x + dx, y + dy): c for (x, y), c in p.items()}


def cel(p: dict, w: int, h: int) -> list[str]:
    rows = [["."] * w for _ in range(h)]
    for (x, y), c in p.items():
        if not (0 <= x < w and 0 <= y < h):
            raise ValueError(f"pixel {(x, y)} outside the {w}x{h} canvas")
        rows[y][x] = c
    return ["".join(r) for r in rows]


def settle_frame(keys: dict, idle: dict) -> dict:
    """The derived ``startle_settle`` cels (layer -> rows)."""
    w, h = keys["canvas"]
    puff = keys["frames"][0]["cels"]
    out: dict[str, dict] = {}
    out["tail"] = shift(pix(puff["tail"]), 0, SETTLE_TAIL_DOWN)
    # Body: the upper block (rows above the shaft) drops 2 rows; the shaft keeps 2 of its 4 identical rows (they
    # follow the block down), the feet stay on rows 42-43 (pivot row 43).
    body = pix(puff["body"])
    first_shaft, last_shaft = SHAFT_ROWS[0], SHAFT_ROWS[-1]
    new_body = {}
    for (x, y), c in body.items():
        if y < first_shaft:
            new_body[(x, y + SETTLE_BODY_DOWN)] = c
        elif y < first_shaft + (len(SHAFT_ROWS) - SETTLE_BODY_DOWN):   # the two kept shaft rows, moved down 2
            new_body[(x, y + SETTLE_BODY_DOWN)] = c
        elif y > last_shaft:
            new_body[(x, y)] = c
        # the two remaining shaft rows (puff rows 40-41) are dropped
    # Hand touch: the head moved 1 px toward the event side, which opens the diagonal link between the cheek outline
    # (head layer) and the chest outline (body layer) on the far side: one K outline pixel closes it (0 orphans).
    new_body.update({p: "K" for p in OUTLINE_CLOSURE})
    out["body"] = new_body
    # Head, tiara: the puff head moves down 2 and 1 px toward the event side (no new head).
    out["head"] = shift(pix(puff["head"]), SETTLE_HEAD_DX, SETTLE_BODY_DOWN)
    # Face: the pupils return to the idle size, so the face is the idle face cel at the head's place:
    # idle -> puff was -3 rows, so idle -> settle is -3 + 2 = -1 row, plus the event-side column.
    out["face"] = shift(pix(idle["cels"]["face"]), SETTLE_HEAD_DX, -(3 - SETTLE_BODY_DOWN))
    gear = pix(puff["gear"])
    tiara = {p: c for p, c in gear.items() if p[1] <= TIARA_MAX_ROW}
    collar = {p: c for p, c in gear.items() if p[1] > TIARA_MAX_ROW}
    out["gear"] = {**shift(tiara, SETTLE_HEAD_DX, SETTLE_BODY_DOWN), **shift(collar, 0, SETTLE_BODY_DOWN)}
    return {layer: cel(out.get(layer, {}), w, h) for layer in keys["layers"]}


def build() -> dict:
    keys, idle = load(KEYS), load(LOCO)["frames"][0]
    cels = {"startle_puff": keys["frames"][0]["cels"],
            "startle_settle": settle_frame(keys, idle),
            "smug": keys["frames"][1]["cels"]}
    frames, tags, i = [], [], 0
    for tag, rows in PLAN:
        start = i
        for name, ticks in rows:
            frames.append({"duration_ms": round(ticks * 1000 / TICK_HZ), "cels": cels[name]})
            i += 1
        tags.append({"name": tag, "from": start, "to": i - 1, "direction": "forward"})
    return {"format": "pixel-grid-v1", "canvas": keys["canvas"], "pivot": keys["pivot"], "palette": keys["palette"],
            "layers": keys["layers"], "frames": frames, "tags": tags}


def dump(grid: dict) -> str:
    return json.dumps(grid, indent=1) + "\n"


def outputs() -> dict[Path, str]:
    return {OUT: dump(build())}


# ------------------------------------------------------------------ evidence


def _scene(floor, entries, name: str) -> None:
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    floor.render(entries, 1).save(EVIDENCE / f"{name}_640x360.png")
    floor.render(entries, 3).save(EVIDENCE / f"{name}_1920x1080.png")


def evidence() -> None:
    from PIL import Image, ImageDraw
    sys.path.insert(0, str(ROOT))
    from scripts.pixelart.common import (BG, CARL_X, CYAN, DONUT_X, GREY, WHITE, YELLOW, cell, flip, grid_images,
                                         label, load_grid_file, repo_path)
    from scripts.pixelart.scene import Floor

    grid = load(OUT)
    imgs = grid_images(grid)
    piv = tuple(grid["pivot"])
    donut_idle = grid_images(load_grid_file(repo_path("assets/source/characters/donut/donut_h40.grid.json")))[0]
    carl_grid = load_grid_file(repo_path("assets/source/characters/carl/carl_h52.grid.json"))
    carl_idle, carl_piv = grid_images(carl_grid)[0], tuple(carl_grid["pivot"])
    gob_grid = load_grid_file(repo_path("assets/source/characters/goblin/goblin_a_h36.grid.json"))
    gob_idle, gob_piv = grid_images(gob_grid)[0], tuple(gob_grid["pivot"])
    gob_idle, gob_piv = flip(gob_idle, gob_piv)
    floor = Floor()
    EVIDENCE.mkdir(parents=True, exist_ok=True)

    # ---- gameplay-scale composition: idle Carl and idle Donut, then idle Donut and her CF02 frames in tick order beside a goblin
    # (the startle trigger context). Donut stands idle, puffs, settles (tick order), then holds the smug frame.
    x0, step = 520, 64
    entries = [(donut_idle, tuple(grid["pivot"]), DONUT_X, 0), (carl_idle, carl_piv, CARL_X, 0)]
    entries.append((donut_idle, piv, x0, 0))                         # idle Donut for direct comparison, clear of Carl
    for i, name in enumerate(("startle_puff", "startle_settle", "smug")):
        entries.append((imgs[i], piv, x0 + step * (i + 1), 0))
    entries.append((gob_idle, gob_piv, x0 + step * 4 + 8, 0))
    _scene(floor, entries, "donut_cf02_floor1")

    # ---- startle trigger in context: puff next to a goblin hit by Carl (Carl kicks, goblin) - kept simple: Donut at
    # her real stand position (right behind Carl) as idle, puffed and smug, one scene per frame, 1x only.
    scenes = []
    for i in (None, 0, 1, 2):
        e = [(carl_idle, carl_piv, CARL_X, 0),
             (donut_idle if i is None else imgs[i], piv, DONUT_X, 0)]
        scenes.append(floor.render(e, 1))
    vx, vy = floor.view[0], floor.view[1]
    gy = floor.ground(CARL_X) - vy
    left = DONUT_X - 64 - vx
    crops = [s.crop((left, gy - 70, left + 136, gy + 14)) for s in scenes]
    strip = Image.new("RGB", (136 * 4 + 6, crops[0].height), BG)
    for n, c in enumerate(crops):
        strip.paste(c, (n * 138, 0))
    strip.save(EVIDENCE / "donut_cf02_beside_carl_1x.png")
    strip.resize((strip.width * 3, strip.height * 3), Image.NEAREST).save(EVIDENCE / "donut_cf02_beside_carl_3x.png")

    # ---- enlarged frames: idle, puff, settle, smug at 8x with the pivot, the floor row and the rows lowered
    k = 8
    names = ["idle (approved)", "startle_puff (key)", "startle_settle (derived)", "smug (key)"]
    frames = [donut_idle] + imgs
    w, h = grid["canvas"]
    sheet = Image.new("RGB", (len(frames) * (w * k + 12) + 12, h * k + 64), BG)
    d = ImageDraw.Draw(sheet)
    for n, im in enumerate(frames):
        cx = 12 + n * (w * k + 12)
        panel = cell(im, k, piv, piv[1])
        sheet.paste(panel.convert("RGB"), (cx, 36))
        label(d, (cx, 8), names[n], YELLOW if "key" in names[n] else WHITE, 14)
        label(d, (cx, 36 + h * k + 6), "paws on row 43, pivot (46, 43)", GREY, 12)
    sheet.save(EVIDENCE / "frames_x8.png")

    # ---- timing diagram
    TICK_PX, K, PAD = 12, 3, 12
    plan = [("startle (Q71, idle only, cooldown 180)", [("startle_puff", 0, 5, 0), ("startle_settle", 6, 11, 1)]),
            ("smug (Q71, idle only, cooldown 300)", [("smug", 0, 19, 2)])]
    width = PAD * 2 + 20 * TICK_PX + 2 * (w * K) + 260
    out = Image.new("RGB", (max(width, 900), 2 * 40 + 2 * (h * K + 70) + 40), BG)
    d = ImageDraw.Draw(out)
    label(d, (PAD, 6), "ART-DONUT-CF02 frame layout, pivot and 60 Hz timing. Frames face right, paws on the pivot row.", WHITE, 12)
    label(d, (PAD, 22), "yellow box = pivot, cyan line = floor row (43). Bar length = ticks x 12 px. Overlays show only in idle; "
          "duration_ms is preview-only (round(ticks x 1000 / 60)).", GREY, 12)
    y = 46
    for title, rows in plan:
        label(d, (PAD, y), title, YELLOW, 13)
        y += 20
        x = PAD
        for name, t0, t1, idx in rows:
            ticks = t1 - t0 + 1
            im = cell(imgs[idx], K, piv, piv[1])
            out.paste(im.convert("RGB"), (x, y))
            label(d, (x + 2, y + h * K + 2), f"{name}  t{t0}-{t1}  {ticks} ticks ({round(ticks * 1000 / 60)} ms)", WHITE, 11)
            bar = (90, 140, 255) if idx != 1 else (120, 200, 120)
            d.rectangle([x, y + h * K + 20, x + ticks * TICK_PX - 1, y + h * K + 30], fill=bar)
            x += max(w * K, ticks * TICK_PX) + 20
        y += h * K + 46
    out.crop((0, 0, out.width, y + 6)).save(EVIDENCE / "timing_diagram.png")
    print(f"wrote evidence in {EVIDENCE.relative_to(ROOT)}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="verify the committed grid equals the build output")
    ap.add_argument("--evidence", action="store_true", help="render the scene, enlarged-frame and timing evidence")
    a = ap.parse_args(argv)
    outs = outputs()
    if a.check:
        bad = [p for p, text in outs.items() if not p.exists() or p.read_text(encoding="utf-8") != text]
        for p in bad:
            print(f"stale: {p.relative_to(ROOT)}")
        return 1 if bad else 0
    if a.evidence:
        evidence()
        return 0
    for p, text in outs.items():
        p.write_text(text, encoding="utf-8")
        print(f"wrote {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
