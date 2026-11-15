#!/usr/bin/env python3
"""Create review GIFs per Aseprite animation tag from already validated atlases.

This is a preview/export tool, not a generator of new franchise character art.
The existing editable .ase sources and manifest metadata remain authoritative.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from PIL import Image

from validate_assets import AssetError, safe_path

ROOT = Path(__file__).resolve().parent.parent
MANIFESTS = ROOT / "assets" / "manifests"
PREVIEWS = ROOT / "generated" / "art" / "previews"
SAFE_ID = re.compile(r"^[a-zA-Z0-9_-]+$")


def render_tag(image: Image.Image, metadata: dict, tag: dict, dest: Path,
               scale: int = 6) -> Path:
    """Render an individual source tag as nearest-neighbor looping GIF."""
    frames = metadata.get("frames")
    name = tag.get("name")
    start, end = tag.get("from"), tag.get("to")
    if not name or not SAFE_ID.fullmatch(name):
        raise AssetError(f"Unsafe animation tag name: {name!r}")
    if (not isinstance(frames, list) or not frames
            or type(start) is not int or type(end) is not int
            or not 0 <= start <= end < len(frames)
            or not 1 <= scale <= 16):
        raise AssetError(f"Invalid frames/tag/scale for {name}")
    background = (37, 35, 55, 255)
    rendered = []
    durations = []
    for index in range(start, end + 1):
        item = frames[index]
        box = item.get("frame", {})
        rect = [box.get(k) for k in ("x", "y", "w", "h")]
        duration = item.get("duration")
        if (any(type(v) is not int for v in rect)
                or min(rect) < 0 or rect[2] <= 0 or rect[3] <= 0
                or rect[0] + rect[2] > image.width
                or rect[1] + rect[3] > image.height
                or type(duration) is not int or not 10 <= duration <= 60_000):
            raise AssetError(f"Invalid export rectangle/duration at frame {index}")
        x, y, w, h = rect
        frame = image.convert("RGBA").crop((x, y, x + w, y + h))
        frame = frame.resize((w * scale, h * scale), Image.Resampling.NEAREST)
        matte = Image.new("RGBA", frame.size, background)
        matte.alpha_composite(frame)
        rendered.append(matte.convert("RGB"))
        durations.append(duration)
    dest.parent.mkdir(parents=True, exist_ok=True)
    rendered[0].save(dest, format="GIF", save_all=True,
                     append_images=rendered[1:], duration=durations, loop=0,
                     optimize=False, disposal=2)
    return dest


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Create per-tag review GIFs")
    parser.add_argument("--manifest-dir", type=Path, default=MANIFESTS,
                        help="directory of manifest *.json (default: assets/manifests)")
    parser.add_argument("--out", type=Path, default=PREVIEWS,
                        help="GIF output directory (default: generated/art/previews)")
    parser.add_argument("--allow-empty", action="store_true",
                        help="do not fail when no tagged animation exists")
    args = parser.parse_args(argv)
    previews = args.out
    try:
        outputs = []
        for path in sorted(args.manifest_dir.glob("*.json")):
            manifest = json.loads(path.read_text(encoding="utf-8"))
            if manifest.get("type") == "tileset":
                continue
            asset_id = manifest["id"]
            if not SAFE_ID.fullmatch(asset_id):
                raise AssetError(f"Unsafe manifest id: {asset_id!r}")
            atlas = safe_path(manifest["output_atlas"])
            metadata = json.loads(safe_path(manifest["output_metadata"]).read_text(encoding="utf-8"))
            with Image.open(atlas) as opened:
                image = opened.convert("RGBA")
            for tag in metadata["meta"]["frameTags"]:
                if tag["name"] not in manifest["animation_tags"]:
                    continue
                out = previews / f"{asset_id}-{tag['name']}.gif"
                render_tag(image, metadata, tag, out)
                outputs.append(out.resolve().relative_to(ROOT).as_posix()
                               if out.resolve().is_relative_to(ROOT) else out.as_posix())
        if not outputs and not args.allow_empty:
            raise AssetError("No tagged animation previews were generated")
        print("Animation previews:\n" + "\n".join(outputs))
        return 0
    except (AssetError, OSError, ValueError, KeyError, TypeError) as exc:
        print(f"ART_PREVIEW_ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
