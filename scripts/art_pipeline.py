#!/usr/bin/env python3
"""Export, validate and preview Aseprite/LibreSprite sprite sheets (WORK-008).

Original editable .ase/.aseprite files are required; a generated concept
illustration cannot be used as a production sprite or timeline.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SUPPORTED_TYPES = {"character_animation", "effect_animation", "prop_animation"}


class AssetError(ValueError):
    """An asset does not satisfy the pipeline contract."""


def safe_path(root: Path, value: str) -> Path:
    if not isinstance(value, str) or not value or Path(value).is_absolute():
        raise AssetError(f"Expected a repository-relative path: {value!r}")
    root = root.resolve()
    path = (root / value).resolve()
    if not path.is_relative_to(root):
        raise AssetError(f"Path escapes repository: {value}")
    return path


def positive_pair(value: object, label: str, *, allow_zero: bool = False) -> tuple[int, int]:
    if not isinstance(value, list) or len(value) != 2 or any(type(v) is not int for v in value):
        raise AssetError(f"{label}: expected two integers")
    if any(v < (0 if allow_zero else 1) for v in value):
        raise AssetError(f"{label}: values out of range")
    return value[0], value[1]


@dataclass(frozen=True)
class Manifest:
    path: Path
    id: str
    source: Path
    output: Path
    frame_size: tuple[int, int]
    pivot: tuple[int, int]
    tags: tuple[str, ...]
    alpha: bool
    max_colors: int | None

    @staticmethod
    def load(path: Path, *, root: Path = ROOT) -> "Manifest":
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as e:
            raise AssetError(f"Cannot load manifest {path}: {e}") from e
        if not isinstance(data, dict) or data.get("schema_version") != 1:
            raise AssetError("Manifest must have schema_version=1")
        asset_id = data.get("id")
        if (not isinstance(asset_id, str) or not asset_id
                or not all(c.isascii() and (c.islower() or c.isdigit() or c in "_-") for c in asset_id)):
            raise AssetError("Invalid asset id; use lowercase ASCII letters, numbers, hyphen and underscore")
        if data.get("type") not in SUPPORTED_TYPES:
            raise AssetError(f"Unsupported animation type: {data.get('type')}")
        source = safe_path(root, data.get("source_path"))
        if source.suffix not in (".ase", ".aseprite"):
            raise AssetError("Source must be an editable .aseprite or .ase file")
        output = safe_path(root, data.get("output_atlas"))
        if output.suffix.lower() != ".png":
            raise AssetError("Output atlas must be PNG")
        if not output.relative_to(root.resolve()).as_posix().startswith("generated/"):
            raise AssetError("Output must be under generated/ (not committed to Git)")
        size = positive_pair(data.get("frame_size"), "frame_size")
        pivot = positive_pair(data.get("pivot"), "pivot", allow_zero=True)
        if pivot[0] >= size[0] or pivot[1] >= size[1]:
            raise AssetError("Pivot must be inside the source frame")
        tags = data.get("animation_tags")
        if (not isinstance(tags, list) or not tags
                or any(not isinstance(t, str) or not t.strip() for t in tags)
                or len(set(tags)) != len(tags)):
            raise AssetError("animation_tags must contain distinct, nonempty names")
        if type(data.get("required_alpha")) is not bool:
            raise AssetError("required_alpha must be boolean")
        colors = data.get("max_colors")
        if colors is not None and (type(colors) is not int or not 1 <= colors <= 256):
            raise AssetError("max_colors must be in [1,256]")
        for required in ("palette_id", "owner", "tool_version", "provenance", "license", "review_status"):
            if not data.get(required):
                raise AssetError(f"Missing required metadata: {required}")
        if not isinstance(data["provenance"], dict):
            raise AssetError("provenance must be an object")
        if not source.is_file():
            raise AssetError(f"Missing source: {source}")
        return Manifest(path, asset_id, source, output, size, pivot, tuple(tags),
                        data["required_alpha"], colors)


def frame_rect(item: dict) -> tuple[int, int, int, int]:
    rect = item.get("frame")
    if not isinstance(rect, dict):
        raise AssetError("Frame missing rectangle")
    try:
        x, y, w, h = (rect[k] for k in ("x", "y", "w", "h"))
    except KeyError as e:
        raise AssetError(f"Missing frame property: {e}") from e
    if any(type(v) is not int for v in (x, y, w, h)) or min(x, y) < 0 or min(w, h) <= 0:
        raise AssetError("Frame rectangles must be nonnegative with nonzero size")
    return x, y, w, h


def validate(manifest: Manifest, *, atlas: Path | None = None,
             data: Path | None = None) -> dict:
    atlas = atlas or manifest.output
    data = data or manifest.output.with_suffix(".json")
    try:
        exported = json.loads(data.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        raise AssetError(f"Cannot read exported JSON: {e}") from e
    frames = exported.get("frames")
    info = exported.get("meta")
    if not isinstance(frames, list) or not frames or not isinstance(info, dict):
        raise AssetError("Expected nonempty Aseprite json-array frames and meta")
    tags = info.get("frameTags")
    if not isinstance(tags, list) or not tags:
        raise AssetError("Metadata lacks frameTags (use --list-tags)")
    names = [tag.get("name") for tag in tags]
    if len(set(names)) != len(names) or set(names) != set(manifest.tags):
        raise AssetError(f"Tags differ from manifest: expected {manifest.tags}, got {names}")
    covered: set[int] = set()
    for tag in tags:
        start, end = tag.get("from"), tag.get("to")
        if any(type(v) is not int for v in (start, end)) or not 0 <= start <= end < len(frames):
            raise AssetError(f"Invalid tag bounds: {tag}")
        for i in range(start, end + 1):
            if i in covered:
                raise AssetError(f"Overlapping tags at frame {i}")
            covered.add(i)
    if covered != set(range(len(frames))):
        raise AssetError("Some frames have no animation tag")
    with Image.open(atlas) as opened:
        if opened.format != "PNG":
            raise AssetError("Atlas must be PNG")
        image = opened.convert("RGBA")
    if (not isinstance(info.get("size"), dict)
            or [info["size"].get("w"), info["size"].get("h")] != list(image.size)):
        raise AssetError("PNG dimensions differ from JSON metadata")
    boxes: list[tuple[int, int, int, int]] = []
    colors: set[tuple[int, int, int]] = set()
    details = []
    for i, frame in enumerate(frames):
        if not isinstance(frame, dict):
            raise AssetError("Every frame must be an object")
        x, y, w, h = frame_rect(frame)
        if x + w > image.width or y + h > image.height:
            raise AssetError(f"Frame {i} outside atlas bounds")
        source = frame.get("sourceSize", {})
        if [source.get("w"), source.get("h")] != list(manifest.frame_size):
            raise AssetError(f"Frame {i} sourceSize differs from manifest")
        if (w, h) != manifest.frame_size or frame.get("trimmed") is True or frame.get("rotated") is True:
            raise AssetError(f"Frame {i} must be untrimmed and unrotated at source size")
        duration = frame.get("duration")
        if type(duration) is not int or not 0 < duration <= 60_000:
            raise AssetError(f"Frame {i} has invalid duration")
        box = (x, y, x + w, y + h)
        if any(x < b[2] and x + w > b[0] and y < b[3] and y + h > b[1] for b in boxes):
            raise AssetError(f"Frame {i} overlaps another frame")
        boxes.append(box)
        region = image.crop(box)
        lo, hi = region.getchannel("A").getextrema()
        if hi == 0:
            raise AssetError(f"Frame {i} is completely transparent")
        if manifest.alpha and lo == 255:
            raise AssetError(f"Frame {i} lacks transparent pixels")
        colors.update(rgb[:3] for rgb in region.get_flattened_data() if rgb[3] > 0)
        details.append({"index": i, "duration_ms": duration, "rect": [x, y, w, h]})
    if manifest.max_colors is not None and len(colors) > manifest.max_colors:
        raise AssetError(f"Palette limit exceeded: {len(colors)} > {manifest.max_colors}")
    return {"id": manifest.id, "frames": len(frames), "tags": names,
            "palette_colors": len(colors), "frame_size": list(manifest.frame_size),
            "pivot": list(manifest.pivot), "details": details}


def make_preview(manifest: Manifest, *, atlas: Path | None = None,
                 data: Path | None = None, output_dir: Path | None = None,
                 scale: int = 6) -> tuple[Path, Path]:
    if not 1 <= scale <= 16:
        raise AssetError("Preview scale must be in [1,16]")
    atlas = atlas or manifest.output
    data = data or manifest.output.with_suffix(".json")
    output_dir = output_dir or ROOT / "artifacts" / "art"
    validate(manifest, atlas=atlas, data=data)
    exported = json.loads(data.read_text(encoding="utf-8"))
    with Image.open(atlas) as image:
        rgba = image.convert("RGBA")
    frames = []
    for item in exported["frames"]:
        x, y, w, h = frame_rect(item)
        frames.append(rgba.crop((x, y, x + w, y + h)))
    scaled = [frame.resize((frame.width * scale, frame.height * scale),
                           Image.Resampling.NEAREST) for frame in frames]
    margin, labels = 12, 24
    width = len(scaled) * (manifest.frame_size[0] * scale + margin) + margin
    height = manifest.frame_size[1] * scale + 2 * margin + labels
    sheet = Image.new("RGBA", (width, height), (39, 37, 58, 255))
    draw = ImageDraw.Draw(sheet)
    for i, frame in enumerate(scaled):
        x = margin + i * (frame.width + margin)
        sheet.alpha_composite(frame, (x, margin))
        draw.text((x, margin + frame.height + 3), f"frame {i}", fill=(255, 255, 255, 255))
    output_dir.mkdir(parents=True, exist_ok=True)
    contact = output_dir / f"{manifest.id}-contact.png"
    anim = output_dir / f"{manifest.id}-loop.gif"
    sheet.convert("RGB").save(contact, optimize=False)
    gif_frames = []
    for frame in scaled:
        background = Image.new("RGBA", frame.size, (39, 37, 58, 255))
        background.alpha_composite(frame)
        gif_frames.append(background.convert("RGB"))
    durations = [f["duration"] for f in exported["frames"]]
    gif_frames[0].save(anim, save_all=True, append_images=gif_frames[1:],
                       duration=durations, loop=0, optimize=False)
    return contact, anim


def export(manifest: Manifest, editor: str, *, preview_dir: Path | None = None) -> dict:
    executable = shutil.which(editor) if os.sep not in editor else editor
    if not executable or not Path(executable).is_file():
        raise AssetError(f"Editor not found: {editor}; set --editor or ASEPRITE_BIN")
    manifest.output.parent.mkdir(parents=True, exist_ok=True)
    metadata = manifest.output.with_suffix(".json")
    cmd = [str(executable), "-b", "--list-tags", "--sheet-type", "rows",
           "--shape-padding", "2", str(manifest.source), "--sheet",
           str(manifest.output), "--data", str(metadata), "--format", "json-array"]
    process = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True,
                             check=False, timeout=120)
    if process.returncode != 0:
        raise AssetError(f"Editor failed ({process.returncode}): {process.stdout}\n{process.stderr}")
    report = validate(manifest)
    contact, gif = make_preview(manifest, output_dir=preview_dir)
    report.update({"atlas": str(manifest.output.relative_to(ROOT)),
                   "metadata": str(metadata.relative_to(ROOT)),
                   "contact_sheet": str(contact), "animation_preview": str(gif)})
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("export", "validate", "preview"))
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--editor", default=os.environ.get("ASEPRITE_BIN", "aseprite"))
    parser.add_argument("--preview-dir", type=Path, default=ROOT / "artifacts" / "art")
    args = parser.parse_args(argv)
    try:
        manifest = Manifest.load(args.manifest.resolve())
        if args.command == "export":
            result = export(manifest, args.editor, preview_dir=args.preview_dir)
        else:
            result = validate(manifest)
            if args.command == "preview":
                contact, gif = make_preview(manifest, output_dir=args.preview_dir)
                result.update({"contact_sheet": str(contact), "animation_preview": str(gif)})
        print(json.dumps(result, indent=2))
        return 0
    except (AssetError, OSError, subprocess.TimeoutExpired) as e:
        print(f"ART_PIPELINE_ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
