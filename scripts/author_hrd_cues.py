#!/usr/bin/env python3
"""HRD-CUES-01 frozen keys and design gate (docs/02-art/hrd-cues-01-spec.md).

The cue frames themselves are authored by ``scripts/author_hoarder_room.py`` (``mound_stir``, ``post_shored``,
``post_give``) into the ``hoarder_mound`` and ``hoarder_post_b`` grids. This script freezes the two design keys
(``hoarder_mound`` frame 3 ``stir_heave`` and ``hoarder_post_b`` frame 3 ``shored``) as SHA-256 fixtures, prints the
stir mote start heights the presenter uses, and composes the design gate: the cue frames and their code-drawn motes
pasted onto real 640 x 360 window stills of the Hoarder room (``docs/evidence/f01-hoarder-presentation``), at 1x and
enlarged. The gate is a design composite, not a runtime capture: the runtime evidence comes from the presenter.

    python3 scripts/author_hrd_cues.py              # write the key fixtures
    python3 scripts/author_hrd_cues.py --check      # exit 1 when a fixture differs from the grids
    python3 scripts/author_hrd_cues.py --motes      # print the stir mote table (dx, start height over the floor)
    python3 scripts/author_hrd_cues.py --gate DIR   # write gate_1x.png and gate_x3.png
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_hoarder_room as room  # noqa: E402
from pixelart.validators import frame_digest  # noqa: E402

KEYS = {   # fixture -> (piece, frozen frame index)
    "hrd-cues-01-mound": ("hoarder_mound", 3),
    "hrd-cues-01-post": ("hoarder_post_b", 3),
}
FIXTURES = ROOT / "tests/fixtures"
STILLS = ROOT / "docs/evidence/f01-hoarder-presentation"

# Stir motes (spec section 4.3): dx from her home column; S1 the approach stir, S2 once the engage line is crossed.
STIR_MOTE_DX = {"S1": (-40, -20, 12), "S2": (-52, -34, -16, 6, 28)}
MOTE_LEAD = 2          # first mote leaves this many ticks into the breath
MOTE_STAGGER = 3       # each next mote 3 ticks later
MOTE_LIFE = 20         # ticks a mote is drawn
MOTE_FALL = 1.2        # px per tick down
MOTE_DRIFT = 0.3       # px per tick outward (away from the crown)
GIVE_MOTE_DX = (0, 2, 14, 16)   # from the post's left edge
GIVE_TICKS = 8
DUST = (242, 209, 158, 242)     # DUST_COLOUR of the Bevy half (0.95, 0.82, 0.62, 0.95)
DUST_PX = 3


def fixture(name: str) -> dict:
    piece, frame = KEYS[name]
    grid = room.piece_grid(piece)
    return {"format": "pixelart-keys-v1", "asset_id": name, "layers": ["main"],
            "keys": {str(frame): {"sha256": frame_digest(grid["frames"][frame]["cels"], ["main"])}}}


def mote_heights() -> dict[int, int]:
    """Start height over the floor (world px, y up) of each stir mote: 2 px over the sealed back layer's surface."""
    back = room.mound_frames()[0]
    out = {}
    for dx in sorted({d for v in STIR_MOTE_DX.values() for d in v}):
        top = room.top_row(back, room.PIVOT[0] + dx)
        out[dx] = (room.H - 1 - top) + 2
    return out


def stir_motes(level: str, phase: int) -> list[tuple[float, float]]:
    """(dx from home, height over the floor) of the drawn stir motes at ``phase`` ticks into a breath."""
    h = mote_heights()
    out = []
    for k, dx in enumerate(STIR_MOTE_DX[level]):
        a = phase - MOTE_LEAD - MOTE_STAGGER * k
        if 0 <= a < MOTE_LIFE:
            side = -1.0 if dx < 0 else 1.0
            out.append((dx + side * MOTE_DRIFT * a, h[dx] - MOTE_FALL * a))
    return out


def give_motes(age: int) -> list[tuple[float, float]]:
    """(x from the post's left edge, height over the floor) of the give grit at ``age`` ticks after she is out."""
    if age >= GIVE_TICKS:
        return []
    return [(dx + 0.5, 54.0 - 3 * k - 2.0 * age) for k, dx in enumerate(GIVE_MOTE_DX)]


# ------------------------------------------------------------------------------------------------------ gate --
def _np_frame(piece: str, index: int):
    import numpy as np
    return np.asarray(room.frame_image(room.piece_grid(piece), index)).astype(int)


def _colour_map(still, sprite, ox, oy):
    """The still's colour for each sprite colour where the sprite is drawn (the window's sRGB rounding)."""
    from collections import Counter, defaultdict
    h, w = sprite.shape[:2]
    votes = defaultdict(Counter)
    for y in range(h):
        for x in range(w):
            if sprite[y, x, 3] and 0 <= oy + y < still.shape[0] and 0 <= ox + x < still.shape[1]:
                votes[tuple(sprite[y, x, :3])][tuple(still[oy + y, ox + x, :3])] += 1
    return {k: v.most_common(1)[0][0] for k, v in votes.items()}


def _paste(still, old, new, ox, oy, cmap):
    """Replace ``old`` by ``new`` at (ox, oy): pixels ``old`` covered and ``new`` does not get the wall from above."""
    out = still.copy()
    h, w = new.shape[:2]
    for y in range(h):
        for x in range(w):
            sy, sx = oy + y, ox + x
            if not (0 <= sy < out.shape[0] and 0 <= sx < out.shape[1]):
                continue
            if new[y, x, 3]:
                c = tuple(new[y, x, :3])
                out[sy, sx, :3] = cmap.get(c, c)
            elif old[y, x, 3]:
                yy = y
                while yy >= 0 and old[yy, x, 3]:
                    yy -= 1
                out[sy, sx, :3] = still[oy + yy, sx, :3] if yy >= 0 else out[sy, sx, :3]
    return out


def _motes(img, pts):
    for x, y in pts:
        cx, cy = round(x), round(y)
        for yy in range(cy - DUST_PX // 2, cy + DUST_PX // 2 + 1):
            for xx in range(cx - DUST_PX // 2, cx + DUST_PX // 2 + 1):
                if 0 <= yy < img.shape[0] and 0 <= xx < img.shape[1]:
                    a = DUST[3] / 255
                    img[yy, xx, :3] = [round(a * DUST[i] + (1 - a) * img[yy, xx, i]) for i in range(3)]


def gate(out_dir: Path) -> None:
    import numpy as np
    from PIL import Image, ImageDraw
    load = lambda n: np.asarray(Image.open(STILLS / n).convert("RGBA")).astype(int)  # noqa: E731
    # Where the sprites sit in the stills (found by matching the committed frames; the stills are 1:1 world px).
    MOUND_AT, POST_AT = (466, 272), (601, 224)          # top-left of the 176 x 72 mound and the 16 x 64 post
    floor_m = MOUND_AT[1] + room.H - 1                   # screen row of the mound's floor contact
    home_x = MOUND_AT[0] + room.PIVOT[0]
    floor_p = POST_AT[1] + 63

    back, front = _np_frame("hoarder_mound", 0), _np_frame("hoarder_mound", 1)
    dormant = load("dormant_640x360.png")
    sealed = back.copy()
    sealed[front[..., 3] > 0] = front[front[..., 3] > 0]
    cmap_m = _colour_map(dormant, sealed, *MOUND_AT)

    def mound(frame: int, level: str | None, phase: int):
        img = _paste(dormant, back, _np_frame("hoarder_mound", frame), *MOUND_AT, cmap_m)
        if level:
            _motes(img, [(home_x + dx, floor_m - h) for dx, h in stir_motes(level, phase)])
        lip = _paste(img, front, front, *MOUND_AT, cmap_m)     # the unchanged front lip over the motes
        return lip

    post0 = _np_frame("hoarder_post_b", 0)
    reveal, out = load("reveal2_640x360.png"), load("out_640x360.png")
    cmap_p = _colour_map(reveal, post0, *POST_AT)

    def post(base, frame: int, age: int | None):
        img = _paste(base, post0, _np_frame("hoarder_post_b", frame), *POST_AT, cmap_p)
        if age is not None:
            _motes(img, [(POST_AT[0] + x, floor_p - h) for x, h in give_motes(age)])
        return img

    stir_s1 = mound(3, "S1", 5)
    shored = post(reveal, 3, None)
    give = post(out, 4, 2)
    full = [("today: sealed_back, no stir", dormant), ("S1 stir: stir_heave, phase 5", stir_s1),
            ("not yet: post shored (Reveal)", shored), ("she is out: post give, age 2", give)]

    def label(im: Image.Image, text: str) -> Image.Image:
        d = ImageDraw.Draw(im)
        d.rectangle((0, im.height - 13, im.width, im.height), fill=(16, 12, 14, 255))
        d.text((3, im.height - 12), text, fill=(235, 225, 205, 255))
        return im

    pad = 4
    one = Image.new("RGBA", (640 * 2 + pad * 3, 360 * 2 + 13 * 2 + pad * 3), (24, 20, 22, 255))
    for i, (t, a) in enumerate(full):
        im = Image.new("RGBA", (640, 373), (16, 12, 14, 255))
        im.paste(Image.fromarray(a.astype("uint8"), "RGBA"), (0, 0))
        one.paste(label(im, t), (pad + (i % 2) * (640 + pad), pad + (i // 2) * (373 + pad)))
    out_dir.mkdir(parents=True, exist_ok=True)
    one.save(out_dir / "gate_1x.png")

    # Enlarged: the mound's visible part (rest, S1 heave, S1 slump, S2 heave_hi, S2 slump) and the post (now, shored,
    # give at ages 0 and 4, intact after), x3.
    mbox = (440, 236, 640, 350)
    rows = [[("rest", dormant), ("S1 heave ph 5", stir_s1), ("S1 slump ph 9", mound(5, "S1", 9)),
             ("S2 heave_hi ph 6", mound(4, "S2", 6)), ("S2 slump ph 12", mound(5, "S2", 12))]]
    pbox = (566, 196, 640, 296)
    rows.append([("today: intact in Reveal", reveal), ("shored (Reveal)", shored), ("give age 0", post(out, 4, 0)),
                 ("give age 4", post(out, 4, 4)), ("intact (age 8+)", out)])
    scale = 3
    tiles = []
    for r, row in enumerate(rows):
        box = mbox if r == 0 else pbox
        tiles.append([label(Image.fromarray(a.astype("uint8"), "RGBA").crop(box).resize(
            ((box[2] - box[0]) * scale, (box[3] - box[1]) * scale), Image.NEAREST), t) for t, a in row])
    width = max(sum(t.width + pad for t in row) for row in tiles) + pad
    height = sum(max(t.height for t in row) + pad for row in tiles) + pad
    big = Image.new("RGBA", (width, height), (24, 20, 22, 255))
    y = pad
    for row in tiles:
        x = pad
        for t in row:
            big.paste(t, (x, y))
            x += t.width + pad
        y += max(t.height for t in row) + pad
    big.save(out_dir / "gate_x3.png")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--motes", action="store_true")
    ap.add_argument("--gate", type=Path)
    a = ap.parse_args(argv)
    if a.motes:
        for dx, h in mote_heights().items():
            print(f"dx {dx:+d}: start {h} px over the floor")
        return 0
    if a.gate:
        gate(a.gate)
        return 0
    bad = 0
    for name in KEYS:
        path = FIXTURES / f"{name}_keys.json"
        text = json.dumps(fixture(name), indent=2) + "\n"
        if a.check:
            ok = path.is_file() and path.read_text() == text
            print(f"{name}: {'up to date' if ok else 'differs'}")
            bad += 0 if ok else 1
        else:
            path.write_text(text, encoding="utf-8")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
