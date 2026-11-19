#!/usr/bin/env python3
"""GOB-02B-DEATH evidence renders: the goblin A death evidence pipeline (``scripts/gob02_death_evidence.py``) run on
the variant B sources, plus the A-versus-B sheets. Stills composed from the committed sources, not Bevy captures.

    python3 scripts/gob02b_death_evidence.py           # writes docs/evidence/gob-02b-death/*.png and *.gif

Outputs (deterministic, seeded; the same seeds and placements as the A evidence, so A and B sheets line up):

* ``a_vs_b_x4.png``: every frame whose pixels differ between A and B (the six poses; parts ``head``, ``head_dead``,
  ``torso_upper``, ``ear``) at x4, A above B, at preset 4 and preset 1, with a 1x strip at the bottom;
* ``frames_x8.png``: the four B pose keys and the two B head parts enlarged x8 (preset 4);
* ``preset_poses_x4.png``, ``preset_parts_x3.png``: every B frame at presets 4, 3, 2, 1;
* ``scenes_1x.png``: lit Floor 1 at 1x with Carl and Donut: crush under the ENV-02 stone (L), the remains hold (L+70),
  the decapitation tick (D), the headless hold (D+60) and an explosion settle;
* ``storyboard_crush.png``, ``storyboard_decap.png``: the A section 8 timelines on the B sprites;
* ``gif_crush_sequence_x1.gif`` / ``_x3.gif`` and ``gif_decap_sequence_x1.gif`` / ``_x3.gif``: preset 4 sequences,
  unlit, exact GIF palette (no quantisation, no dither).
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "scripts"))

from PIL import Image, ImageDraw  # noqa: E402

import author_gob02_death as ad  # noqa: E402
import author_gob02b_death as bd  # noqa: E402
import gob02_death_evidence as ev  # noqa: E402
from scripts.pixelart.common import CARL_X, gif_delays, grid_images, label  # noqa: E402

OUT = ROOT / "docs/evidence/gob-02b-death"
DIFF_PARTS = ["head", "head_dead", "torso_upper", "ear"]
BG = (30, 24, 30)
ASprites = ev.Sprites                                   # the A sprites (main() repoints ev.Sprites to B)


class BSprites(ev.Sprites):
    """The A evidence sprite cache, loading the variant B grids (same part ids, anchors and metadata)."""

    def __init__(self) -> None:
        self.parts_grid, self.poses_grid = bd.parts_grid(), bd.poses_grid()
        self.entries = {e["id"]: e for e in bd.parts_manifest()["death_parts"]["parts"]}
        self.cache: dict = {}


def a_vs_b() -> Path:
    k = 4
    a_sp, b_sp = ASprites(), BSprites()
    w, h = ad.CANVAS
    pose_w = 6 * (w * k + 6)
    part_w = len(DIFF_PARTS) * (32 * k + 6)
    band_h = 2 * (h * k + 6) + 22
    strip_h = 2 * (h + 6) + 22
    sheet = Image.new("RGB", (70 + pose_w + 20 + part_w, 26 + 2 * band_h + strip_h), BG)
    d = ImageDraw.Draw(sheet)
    label(d, (6, 4), "A (top) vs B (bottom), x4. Poses 0 to 5, then parts head, head_dead, torso_upper, ear. "
                     "Band 1: preset 4. Band 2: preset 1 (gore hidden). Bottom: 1x, preset 4")
    for band, preset in enumerate((4, 1)):
        y0 = 26 + band * band_h
        label(d, (6, y0 + 4), f"preset {preset}")
        for r, (name, sp) in enumerate((("A", a_sp), ("B", b_sp))):
            yy = y0 + 20 + r * (h * k + 6)
            label(d, (6, yy + h * k // 2), name, size=14)
            for n in range(6):
                sheet.paste(ev.tile(sp.pose(n, preset), k), (70 + n * (w * k + 6), yy))
            for c, pid in enumerate(DIFF_PARTS):
                sheet.paste(ev.tile(sp.part(pid, preset), k), (70 + pose_w + 20 + c * (32 * k + 6), yy + (h - 32) * k))
    y0 = 26 + 2 * band_h
    label(d, (6, y0 + 4), "1x")
    for r, sp in enumerate((a_sp, b_sp)):
        for n in range(6):
            sheet.paste(ev.tile(sp.pose(n, 4), 1), (70 + n * (w + 6), y0 + 20 + r * (h + 6)))
        for c, pid in enumerate(DIFF_PARTS):
            sheet.paste(ev.tile(sp.part(pid, 4), 1), (70 + 6 * (w + 6) + 20 + c * 38, y0 + 20 + r * (h + 6) + h - 32))
    path = OUT / "a_vs_b_x4.png"
    sheet.save(path)
    return path


def frames_x8() -> Path:
    k = 8
    sp = BSprites()
    imgs = [sp.pose(n, 4) for n in ad.POSE_KEYS] + [sp.part("head", 4), sp.part("head_dead", 4)]
    names = [f"pose {n} {t}" for n, t in zip(ad.POSE_KEYS, ("crush_contact", "crushed_remains", "decap_contact",
                                                            "headless_collapse"))] + ["part head", "part head_dead"]
    cols = 3
    cw, ch = 64 * k + 8, 56 * k + 26
    sheet = Image.new("RGB", (cols * cw + 8, 2 * ch + 8), BG)
    d = ImageDraw.Draw(sheet)
    for i, (img, name) in enumerate(zip(imgs, names)):
        x, y = 8 + (i % cols) * cw, 8 + (i // cols) * ch
        label(d, (x, y), name, size=13)
        sheet.paste(ev.tile(img, k), (x, y + 20))
    path = OUT / "frames_x8.png"
    sheet.save(path)
    return path


def decap_sequence_gif(stage: ev.Stage, k: int) -> Path:
    """Decapitation at preset 4: D, D+4, D+10, D+14, D+40, D+100 (pose 3, 4, 5 and pool stages), unlit, exact
    palette."""
    gx = CARL_X + 170
    ticks = [0, 2, 4, 7, 10, 14, 40, 100]
    frames = []
    for t in ticks:
        frames.append(ev.crop_of(_decap_unlit(stage, t, gx), stage.floor, gx, k, left=110, right=150))
    holds = [t2 - t1 for t1, t2 in zip(ticks, ticks[1:])] + [90]
    path = OUT / f"gif_decap_sequence_x{k}.gif"
    ev.save_exact_gif(frames, gif_delays(holds), path)
    return path


def _decap_unlit(stage: ev.Stage, t: int, gx: int) -> Image.Image:
    """``Stage.decap`` renders lit; the GIF needs the untinted palette, so render the same scene with ``lit=False``."""
    render = stage.floor.render
    try:
        stage.floor.render = lambda entries, scale, lit=True: render(entries, scale, False)
        return stage.decap(4, t, gx)
    finally:
        stage.floor.render = render


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    ev.OUT = OUT
    ev.Sprites = BSprites                                  # Stage() builds its sprites from this name
    sp, stage = BSprites(), ev.Stage()
    for fn in (ev.preset_poses, ev.preset_parts):
        print("wrote", fn(sp).relative_to(ROOT))
    print("wrote", a_vs_b().relative_to(ROOT))
    print("wrote", frames_x8().relative_to(ROOT))
    print("wrote", ev.storyboard(stage, "storyboard_crush.png", "crush", [0, 3, 6, 30], CARL_X + 120).relative_to(ROOT))
    print("wrote", ev.storyboard(stage, "storyboard_decap.png", "decap", [0, 4, 10, 40], CARL_X + 170).relative_to(ROOT))
    print("wrote", ev.scenes_1x(stage).relative_to(ROOT))
    for k in (1, 3):
        print("wrote", ev.crush_sequence_gif(stage, k).relative_to(ROOT))
        print("wrote", decap_sequence_gif(stage, k).relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
