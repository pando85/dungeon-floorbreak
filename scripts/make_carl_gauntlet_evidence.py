#!/usr/bin/env python3
"""CARL-GAUNTLET-01 art evidence: contact sheets, timing GIFs and the two owner diagrams.

    python3 scripts/make_carl_gauntlet_evidence.py [OUT_DIR]      (default docs/evidence/carl-gauntlet-01)

Composed from the committed grids only (the four ``*_gauntlet.grid.json`` overlays on their body sheets); nothing
here is a Bevy capture (those come from ``scripts/make_carl_gauntlet_evidence.sh``) and nothing changes a source.
Output is deterministic (no timestamps). Writes, per sheet, ``contact_<sheet>_gauntlet_x3.png`` (overlay composed
on the body) and ``contact_<sheet>_overlay_only_x3.png``; ``gif_{idle,run,punch_a,punch_b}_gauntlet_x3.gif``;
``diagram_frame_layout.png`` and ``diagram_state_flow.png``.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

import author_carl_gauntlet as author  # noqa: E402
from scripts.pixelart.common import (BG, CYAN, GREY, GROUND_DARK, GROUND_LIGHT, WHITE, YELLOW, gif_delays,  # noqa: E402
                                     grid_images, label, save_gif, text_width, up)

SHEETS = author.SHEETS
# the four GIFs: (name, sheet, first frame, last frame, rests on the last frame)
GIFS = (("idle", "locomotion", 0, 3, False), ("run", "locomotion", 4, 9, False),
        ("punch_a", "melee", 0, 3, True), ("punch_b", "melee", 4, 7, True))
REST_MS = 400


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def composed(sheet: str) -> tuple[dict, dict, dict]:
    """``(body grid, overlay grid, combined grid)``: the body layers with the gauntlet layer on top."""
    body, over = load(author.body_path(sheet)), load(author.overlay_path(sheet))
    palette = {e["key"]: e for e in body["palette"]}
    both = dict(body)
    both["layers"] = list(body["layers"]) + ["gauntlet"]
    both["palette"] = list(palette.values())
    both["frames"] = [{"duration_ms": f["duration_ms"], "cels": {**f["cels"], "gauntlet": o["cels"]["gauntlet"]}}
                      for f, o in zip(body["frames"], over["frames"])]
    return body, over, both


def tag_of(grid: dict, i: int) -> str:
    return next(t["name"] for t in grid["tags"] if t["from"] <= i <= t["to"])


def sheet_image(grid: dict, imgs: list[Image.Image], title: str, k: int = 3, max_width: int = 1800) -> Image.Image:
    w, h = imgs[0].size
    gap, cap_h, pad, head = 6, 16, 4, 22
    cols = max(1, min(len(imgs), (max_width - 2 * pad) // (w * k + gap)))
    rows = -(-len(imgs) // cols)
    sheet = Image.new("RGB", (cols * (w * k + gap) + 2 * pad, head + rows * (h * k + cap_h + gap) + pad), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (pad, 5), title, WHITE, 11)
    keys = set(f for _, f in author.KEY_FRAMES.values())
    for i, img in enumerate(imgs):
        r, c = divmod(i, cols)
        x, y = pad + c * (w * k + gap), head + r * (h * k + cap_h + gap)
        base = Image.new("RGBA", img.size, (36, 34, 50, 255))
        base.alpha_composite(img)
        sheet.paste(up(base, k).convert("RGB"), (x, y))
        label(d, (x + 1, y + h * k + 1), f"{i:02d} {tag_of(grid, i)}", GREY, 11)
    return sheet


def contact_sheets(out: Path) -> list[Path]:
    written = []
    for sheet in SHEETS:
        body, over, both = composed(sheet)
        n = len(body["frames"])
        path = out / f"contact_{sheet}_gauntlet_x3.png"
        sheet_image(body, grid_images(both),
                    f"carl_h52_{sheet} + gauntlet overlay: {n} frames at x3 (overlay composed on the body)").save(path)
        written.append(path)
        path = out / f"contact_{sheet}_overlay_only_x3.png"
        sheet_image(body, grid_images(over), f"carl_h52_{sheet}_gauntlet: {n} overlay frames alone at x3").save(path)
        written.append(path)
    return written


def sprite_on(img: Image.Image, k: int) -> Image.Image:
    w, h = img.size
    base = Image.new("RGBA", (w, h + 4), BG + (255,))
    d = ImageDraw.Draw(base)
    d.rectangle([0, h, w - 1, h], fill=GROUND_LIGHT)
    d.rectangle([0, h + 1, w - 1, h + 3], fill=GROUND_DARK)
    base.alpha_composite(img)
    return up(base, k)


def gifs(out: Path) -> list[Path]:
    written = []
    for name, sheet, first, last, rest in GIFS:
        _, _, both = composed(sheet)
        imgs = grid_images(both)
        ms = [both["frames"][i]["duration_ms"] for i in range(first, last + 1)]
        frames = [sprite_on(imgs[i], 3) for i in range(first, last + 1)]
        if rest:
            ms[-1] += REST_MS
        path = out / f"gif_{name}_gauntlet_x3.gif"
        save_gif(frames, gif_delays(ms, 1000), path)
        written.append(path)
    return written


# ------------------------------------------------------------------ diagrams

def arrow(d: ImageDraw.ImageDraw, a: tuple[int, int], b: tuple[int, int], fill=WHITE) -> None:
    d.line([a, b], fill=fill, width=2)
    (x0, y0), (x1, y1) = a, b
    if abs(x1 - x0) >= abs(y1 - y0):
        s = 1 if x1 > x0 else -1
        d.polygon([(x1, y1), (x1 - 8 * s, y1 - 5), (x1 - 8 * s, y1 + 5)], fill=fill)
    else:
        s = 1 if y1 > y0 else -1
        d.polygon([(x1, y1), (x1 - 5, y1 - 8 * s), (x1 + 5, y1 - 8 * s)], fill=fill)


def box(d: ImageDraw.ImageDraw, xy, text_lines: list[str], fill=(36, 34, 50), outline=GREY, size: int = 12) -> None:
    x0, y0, x1, y1 = xy
    d.rectangle(xy, fill=fill, outline=outline, width=2)
    for i, line in enumerate(text_lines):
        label(d, (x0 + 10, y0 + 8 + i * (size + 5)), line, WHITE if i == 0 else GREY, size)


def frame_layout_diagram(out: Path) -> Path:
    """Overlay cell = body cell: the same 48 x 56 grid, index, flip and pivot; the two atlases side by side."""
    body, over, both = composed("melee")
    body_imgs = grid_images({**body})
    over_imgs = grid_images(over)
    both_imgs = grid_images(both)
    k = 5
    w, h = 48, 56
    frames = (4, 5, 0)
    pad = 16
    width = pad + len(frames) * (w * k + pad) + 20
    img = Image.new("RGB", (max(width, 1000), 92 + 3 * (h * k + 40) + 60), BG)
    d = ImageDraw.Draw(img)
    label(d, (pad, 8), "CARL-GAUNTLET-01 frame layout: the overlay cell is the body cell (same 48 x 56 grid, pivot (25, 55), index and flip)",
          WHITE, 13)
    label(d, (pad, 30), "melee atlas, frames 4 (punch_b windup), 5 (punch_b contact) and 0 (punch_a chamber), drawn at x5; cyan = contact row 55, yellow = pivot",
          GREY, 11)
    rows = (("body atlas  carl_book1_h52_melee.png", body_imgs), ("overlay atlas  carl_book1_h52_melee_gauntlet.png", over_imgs),
            ("runtime: body sprite, overlay sprite on top (z + 0.01)", both_imgs))
    for r, (title, imgs) in enumerate(rows):
        y = 56 + r * (h * k + 40)
        label(d, (pad, y), title, YELLOW, 12)
        for c, f in enumerate(frames):
            x = pad + c * (w * k + pad)
            cell = Image.new("RGBA", (w, h), (36, 34, 50, 255))
            cell.alpha_composite(imgs[f])
            big = up(cell, k)
            bd = ImageDraw.Draw(big)
            bd.rectangle([0, 0, w * k - 1, h * k - 1], outline=(90, 86, 120))
            bd.line([(0, 55 * k + k - 1), (w * k, 55 * k + k - 1)], fill=CYAN)
            bd.rectangle([25 * k, 55 * k, 25 * k + k - 1, 55 * k + k - 1], outline=YELLOW, width=2)
            img.paste(big.convert("RGB"), (x, y + 20))
            label(d, (x + 2, y + 20 + h * k + 2), f"cell {f:02d}", GREY, 11)
    return finish(img, out / "diagram_frame_layout.png")


def finish(img: Image.Image, path: Path) -> Path:
    img.save(path)
    return path


def state_flow_diagram(out: Path) -> Path:
    """The capability gate and the per-frame copy, as boxes and arrows."""
    img = Image.new("RGB", (1200, 560), BG)
    d = ImageDraw.Draw(img)
    label(d, (16, 8), "CARL-GAUNTLET-01 state flow: one gate, read-only, nothing hashed", WHITE, 14)
    box(d, (16, 50, 380, 170), ["Carl's combat state (simulation)",
                                "CombatState::story_holds(fist_reinforced)",
                                "granted on the Juicer's defeat (room 5), by --carl-gauntlet,",
                                "--f01-loadout all and every story pin that holds it"], outline=YELLOW)
    box(d, (470, 50, 790, 170), ["Presenter: sync_carl_gauntlet",
                                 "Update, after the body sprites are synced",
                                 "takes Res<Combat> (read-only) and",
                                 "writes only the overlay sprites"], outline=CYAN)
    box(d, (880, 50, 1184, 170), ["held == false",
                                  "every overlay Hidden",
                                  "(Carl is drawn bare, as before)",
                                  "no fade, flash or glow on the grant"], outline=GREY)
    arrow(d, (380, 110), (470, 110))
    arrow(d, (790, 110), (880, 110))
    label(d, (800, 92), "no", GREY, 12)
    label(d, (396, 92), "held?", GREY, 12)
    label(d, (16, 190), "held == true: one overlay entity per Carl body sprite copies that sprite every frame (a hidden body hides its overlay)",
          YELLOW, 13)
    rows = (("ActorVisual (is_hero)", "locomotion atlas (idle, run, jump, land)", "carl_book1_h52_locomotion_gauntlet.png"),
            ("CarlMelee", "melee atlas (punches, kick, hurt, knockdown)", "carl_book1_h52_melee_gauntlet.png"),
            ("CarlThrowSprite", "dynamite throw atlas", "carl_book1_h52_dynamite_gauntlet.png"),
            ("CarlCf02Sprite", "CF02 acting atlas (stomp, dives, flinch)", "carl_book1_h52_cf02_gauntlet.png"))
    for i, (comp, atlas, overlay) in enumerate(rows):
        y = 222 + i * 70
        box(d, (16, y, 380, y + 56), [comp, atlas], size=12)
        box(d, (470, y, 790, y + 56), ["copy: Visibility, atlas index, flip_x,", "colour, Transform (z + 0.01)"], size=12, outline=CYAN)
        box(d, (880, y, 1184, y + 56), [overlay.replace("carl_book1_h52_", "..."), "overlay sprite (same cell, same pivot)"], size=12)
        arrow(d, (380, y + 28), (470, y + 28))
        arrow(d, (790, y + 28), (880, y + 28))
    label(d, (16, 512), "No CombatEvent, no simulation write, not part of any hash, replay, playtrace pin or golden. Donut and every other actor never get an overlay.",
          GREY, 12)
    return finish(img, out / "diagram_state_flow.png")


def main(argv: list[str]) -> int:
    out = Path(argv[1]) if len(argv) > 1 else ROOT / "docs/evidence/carl-gauntlet-01"
    out.mkdir(parents=True, exist_ok=True)
    files = contact_sheets(out) + gifs(out) + [frame_layout_diagram(out), state_flow_diagram(out)]
    for f in files:
        print(f.relative_to(ROOT) if f.is_relative_to(ROOT) else f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
