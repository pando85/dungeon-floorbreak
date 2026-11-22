#!/usr/bin/env python3
"""EVID-ROOM: assemble a GIF (and optional stills) from the frames of
scripts/room_capture.sh.

  room_gif.py FRAMES_DIR OUT.gif --step-ticks N [--crop L,T,R,B --zoom K]
              [--still INDEX OUT.png ...]

The frame delay is STEP ticks at 60 Hz (rounded to 10 ms), so the GIF plays at
game speed. `--crop` with `--zoom` writes a nearest-neighbour enlargement of a
region (the 3x reading of a small action). Needs Pillow."""
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


def load(folder: Path) -> list[Image.Image]:
    return [Image.open(p).convert("RGB") for p in sorted(folder.glob("frame_*.png"))]


def check_first_frame(images: list[Image.Image], folder: Path) -> None:
    """Refuse a capture whose first frame is an unrendered (near-black) frame: the
    palette is sampled from it, so such a GIF is neither right nor byte-stable (the
    capture waits for the first real render, `CAPTURE_FIRST_SETTLE_FRAMES`)."""
    def lit(im: Image.Image) -> float:
        hist = im.convert("L").histogram()
        return sum(hist[25:]) / sum(hist)
    first, rest = lit(images[0]), [lit(im) for im in images[1:]] or [lit(images[0])]
    median = sorted(rest)[len(rest) // 2]
    if first < 0.25 * median:
        raise SystemExit(f"{folder}: frame 0 is mostly black ({first:.3f} lit against a median "
                         f"{median:.3f}): the first screenshot was taken before the scene rendered")


def save_gif(images: list[Image.Image], path: Path, ms: int) -> None:
    """One global 96-colour palette (median cut over every sixth frame) and no
    dither: far smaller than a palette per frame, and the same on every run."""
    sample = images[::6] or images[:1]
    w, h = images[0].size
    sheet = Image.new("RGB", (w, h * len(sample)))
    for i, im in enumerate(sample):
        sheet.paste(im, (0, i * h))
    palette = sheet.quantize(colors=96, method=Image.Quantize.MEDIANCUT,
                             dither=Image.Dither.NONE)
    frames = [im.quantize(palette=palette, dither=Image.Dither.NONE) for im in images]
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=ms, loop=0,
                   optimize=True, disposal=1)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("frames", type=Path)
    ap.add_argument("out", type=Path)
    ap.add_argument("--step-ticks", type=int, required=True)
    ap.add_argument("--crop")
    ap.add_argument("--zoom", type=int, default=1)
    ap.add_argument("--still", nargs=2, action="append", default=[],
                    metavar=("INDEX", "PNG"))
    args = ap.parse_args()
    images = load(args.frames)
    if not images:
        raise SystemExit(f"no frames in {args.frames}")
    check_first_frame(images, args.frames)
    if args.crop:
        box = tuple(int(v) for v in args.crop.split(","))
        images = [im.crop(box).resize(((box[2] - box[0]) * args.zoom,
                                       (box[3] - box[1]) * args.zoom), Image.NEAREST)
                  for im in images]
    ms = round(args.step_ticks * 1000 / 60 / 10) * 10
    args.out.parent.mkdir(parents=True, exist_ok=True)
    save_gif(images, args.out, ms)
    for index, png in args.still:
        images[int(index)].save(png, optimize=True)


if __name__ == "__main__":
    main()
