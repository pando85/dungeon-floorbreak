#!/usr/bin/env python3
"""Check native Aseprite/LibreSprite JSON-array atlas exports and manifests.

These checks validate structure and safety, NOT artistic quality or franchise rights.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
MANIFEST_DIR = ROOT / "assets/manifests"


class AssetError(ValueError):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise AssetError(message)


def safe_path(raw: str) -> Path:
    check(bool(raw) and not Path(raw).is_absolute(), f"Path must be relative: {raw}")
    path = (ROOT / raw).resolve()
    check(path.is_relative_to(ROOT), f"Path escapes repository: {raw}")
    return path


def validate(manifest: dict, metadata: dict, image: Image.Image) -> None:
    required = {
        "schema_version", "id", "type", "source_path", "output_atlas",
        "output_metadata", "palette_id", "frame_size", "pivot",
        "animation_tags", "required_alpha", "owner", "tool_version",
        "provenance", "license", "review_status"
    }
    check(required <= manifest.keys(), f"Manifest missing: {sorted(required - manifest.keys())}")
    check(manifest["schema_version"] == 1, "Unsupported schema")
    check(manifest["review_status"] == "prototype_fixture_unapproved",
          "Fixture must not be labeled final/approved")
    check(bool(manifest["license"]) and bool(manifest["provenance"].get("origin")),
          "License/provenance required")
    source = safe_path(manifest["source_path"])
    check(source.is_file() and source.suffix in (".ase", ".aseprite"),
          "Editable ASE source missing")
    if manifest.get("grid_source"):
        grid = safe_path(manifest["grid_source"])
        check(grid.is_file() and grid.suffix == ".json", "Pixel-grid source missing")
    check(safe_path(manifest["output_atlas"]).is_file(), "Atlas file missing")
    check(safe_path(manifest["output_metadata"]).is_file(), "Metadata file missing")
    width, height = manifest["frame_size"]
    check(all(isinstance(x, int) and x > 0 for x in (width, height)), "Invalid frame size")
    px, py = manifest["pivot"]
    check(0 <= px < width and 0 <= py < height, "Pivot outside sprite")
    check(image.mode in ("RGB", "RGBA"), "Atlas must use RGB(A)")
    check(image.width > 0 and image.height > 0, "Empty atlas")
    frames = metadata.get("frames")
    check(isinstance(frames, list) and bool(frames), "Missing frame list")
    expected_tags = manifest["animation_tags"]
    check(isinstance(expected_tags, list) and len(expected_tags) == len(set(expected_tags)),
          "Invalid tags")
    source_tags = metadata.get("meta", {}).get("frameTags", [])
    names = {tag["name"] for tag in source_tags}
    check(set(expected_tags) <= names, f"Missing animation tags: {set(expected_tags) - names}")
    for tag in source_tags:
        check(0 <= tag["from"] <= tag["to"] < len(frames), "Tag range out of bounds")
    regions = []
    for i, frame in enumerate(frames):
        r = frame.get("frame", {})
        x, y, w, h = (r.get(k) for k in ("x", "y", "w", "h"))
        check(all(isinstance(z, int) for z in (x, y, w, h)),
              f"Frame {i} has invalid rectangle")
        check(x >= 0 and y >= 0 and w == width and h == height,
              f"Frame {i} has wrong source size")
        check(x + w <= image.width and y + h <= image.height,
              f"Frame {i} outside atlas")
        check(frame.get("duration", 0) > 0, f"Frame {i} has invalid duration")
        check(not frame.get("rotated", False), "Rotated packing unsupported")
        check(not frame.get("trimmed", False), "Trimmed frame changes pivot")
        crop = image.crop((x, y, x + w, y + h)).convert("RGBA")
        check(crop.getbbox() is not None, f"Frame {i} is completely empty")
        if manifest["required_alpha"]:
            low, high = crop.getchannel("A").getextrema()
            check(low == 0 and high == 255,
                  f"Frame {i} must include transparent and opaque pixels")
        regions.append((x, y, x + w, y + h))
    for i, a in enumerate(regions):
        for b in regions[i + 1:]:
            check(a[2] <= b[0] or b[2] <= a[0] or a[3] <= b[1] or b[3] <= a[1],
                  "Atlas frame rectangles overlap")
    palette = image.convert("RGBA").getcolors(maxcolors=512)
    check(palette is not None and len(palette) <= 64, "Unexpected color count; check palette drift")
    if manifest["type"] == "tileset":
        check(manifest.get("tile_size") == [16, 16], "Expected 16x16 tile grid")
        check(width % 16 == 0 and height % 16 == 0,
              "Tile dimensions not divisible by 16")
        check(len(manifest.get("tiles", [])) == (width // 16) * (height // 16),
              "Missing tile IDs")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate exported art atlases against manifests")
    parser.add_argument("--manifest-dir", type=Path, default=MANIFEST_DIR,
                        help="directory of manifest *.json (default: assets/manifests)")
    args = parser.parse_args(argv)
    manifests = sorted(args.manifest_dir.glob("*.json"))
    if not manifests:
        print("Missing asset manifests", file=sys.stderr)
        return 1
    for path in manifests:
        try:
            manifest = json.loads(path.read_text(encoding="utf-8"))
            metadata = json.loads(safe_path(manifest["output_metadata"]).read_text(encoding="utf-8"))
            with Image.open(safe_path(manifest["output_atlas"])) as image:
                validate(manifest, metadata, image)
            print(f"VALID: {manifest['id']}: {len(metadata['frames'])} frames")
        except (AssetError, ValueError, OSError, KeyError, TypeError) as exc:
            print(f"INVALID: {path}: {exc}", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
