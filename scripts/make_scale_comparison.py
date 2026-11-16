#!/usr/bin/env python3
"""Deterministic gameplay-scale sprite comparison scene (native 640x360).

Places 1-6 exported actors (candidates and optional companions, plus a height
reference actor) on the SAME floor line, under the same camera and backdrop,
spaced evenly, and writes clean, annotated, 2x, Steam Deck (640x400) and 6x
close-up PNGs plus ``comparison.json``.

Explicit usage::

    python3 scripts/make_scale_comparison.py \\
      --sprite "A 32px=out/a.png:out/a.json" --sprite "B 40px=out/b.png:out/b.json:0" \\
      --companion "Other=out/c.png:out/c.json" --out out/scale_study

Manifest-declared groups (used by ``scripts/export_assets.sh``)::

    python3 scripts/make_scale_comparison.py --list-groups
    python3 scripts/make_scale_comparison.py --group GROUP [--out DIR]

A manifest in ``assets/manifests/`` joins a group with an optional field (an
object, or a list of objects to join several groups)::

    "scale_study": {"group": "my_study", "order": 1, "role": "candidate"}

``role`` is ``candidate`` (a sprite under study, gets close-ups), ``companion``
(extra actor for context, e.g. another character's chosen sprite) or
``reference`` (the group's height reference, at most one; the default is the
neutral 52 px mannequin). Candidates and companions are placed left to right
sorted by ``order`` (ties by manifest id), after the reference.

Sprite spec: ``LABEL=ATLAS_PNG:METADATA_JSON[:FRAME]`` (FRAME defaults to 0).
The metadata is the LibreSprite/Aseprite ``--format json-array`` file. A sprite's
contact row is its bottom-most opaque pixel row and its pivot x is the centre of
its opaque bounding box; both are measured and recorded in comparison.json.
The default reference is the neutral 52 px height mannequin; it is NOT Carl.

This is a native-scale pixel review tool, not a gameplay capture. All outputs
are deterministic for the same inputs.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

from make_contact_sheet import draw_backdrop, draw_floor

ROOT = Path(__file__).resolve().parent.parent
WIDTH, HEIGHT, DECK_HEIGHT = 640, 360, 400
FLOOR_TOP = 296  # first floor row in the 640x360 scene; feet rest on FLOOR_TOP - 1
ZOOM = 6
BACKGROUND = "#191a2c"
DEFAULT_REFERENCE_LABEL = "Carl-height proxy (neutral 52 px mannequin, NOT Carl)"
MAX_ACTORS = 6  # candidates + companions; the reference is extra
ROLES = ("candidate", "companion", "reference")
GROUP_ID = re.compile(r"[A-Za-z0-9_-]+")
MANIFEST_DIR = ROOT / "assets/manifests"
LEGEND_TAGS = {"reference": "REFERENCE", "candidate": "SPRITE", "companion": "COMPANION"}
GUIDE_HEIGHTS = ((48, (79, 216, 255, 255)), (56, (255, 225, 79, 255)))


class SceneError(ValueError):
    """Invalid sprite specification or image data."""


@dataclass
class Actor:
    label: str
    atlas: str
    metadata: str
    frame_index: int
    image: Image.Image
    reference: bool = False
    role: str = "candidate"
    bbox: tuple[int, int, int, int] = (0, 0, 0, 0)  # inclusive x0,y0,x1,y1
    colors: list[str] = field(default_factory=list)
    x: int = 0  # top-left of frame in the 640-wide scene
    y: int = 0

    @property
    def pivot(self) -> tuple[int, int]:
        return (self.bbox[0] + self.bbox[2]) // 2, self.bbox[3]


def slug(label: str) -> str:
    text = re.sub(r"[^A-Za-z0-9]+", "_", label).strip("_")
    return text[:48] or "sprite"


def parse_spec(spec: str) -> tuple[str, str, str, int]:
    label, sep, rest = spec.partition("=")
    parts = rest.split(":")
    if not sep or not label.strip() or len(parts) not in (2, 3) or not all(parts):
        raise SceneError(f"Bad sprite spec {spec!r}; expected LABEL=ATLAS_PNG:METADATA_JSON[:FRAME]")
    frame = 0
    if len(parts) == 3:
        if not parts[2].isdigit():
            raise SceneError(f"Frame index must be a non-negative integer in {spec!r}")
        frame = int(parts[2])
    return label.strip(), parts[0], parts[1], frame


def display_path(raw: str) -> str:
    path = Path(raw)
    if path.is_absolute():
        try:
            return path.resolve().relative_to(ROOT).as_posix()
        except ValueError:
            return path.name
    return path.as_posix()


def load_actor(label: str, atlas: str, metadata: str, frame_index: int,
               reference: bool = False, role: str = "candidate") -> Actor:
    try:
        data = json.loads(Path(metadata).read_text(encoding="utf-8"))
        with Image.open(atlas) as opened:
            sheet = opened.convert("RGBA")
    except (OSError, json.JSONDecodeError) as exc:
        raise SceneError(f"Cannot load {label!r}: {exc}") from exc
    frames = data.get("frames")
    if not isinstance(frames, list) or not 0 <= frame_index < len(frames):
        raise SceneError(f"{label!r}: frame {frame_index} not in metadata")
    rect = frames[frame_index]["frame"]
    x, y, w, h = (rect[k] for k in ("x", "y", "w", "h"))
    if x < 0 or y < 0 or x + w > sheet.width or y + h > sheet.height:
        raise SceneError(f"{label!r}: frame rectangle outside atlas")
    image = sheet.crop((x, y, x + w, y + h))
    alpha = image.getchannel("A")
    low_high = [v for v in range(1, 255) if alpha.histogram()[v]]
    if low_high:
        raise SceneError(f"{label!r}: partially transparent pixels (antialiasing) are not allowed")
    bbox = alpha.getbbox()
    if bbox is None:
        raise SceneError(f"{label!r}: frame {frame_index} is empty")
    colors = sorted("#%02x%02x%02x" % rgba[:3]
                    for _count, rgba in image.getcolors(maxcolors=image.width * image.height)
                    if rgba[3])
    return Actor(label, display_path(atlas), display_path(metadata), frame_index, image,
                 reference, "reference" if reference else role,
                 (bbox[0], bbox[1], bbox[2] - 1, bbox[3] - 1), colors)


def place(actors: list[Actor]) -> None:
    """Evenly space actors; every bottom opaque row lands on the shared floor row."""
    n = len(actors)
    floor_row = FLOOR_TOP - 1
    for i, actor in enumerate(actors):
        centre = WIDTH * (i + 1) // (n + 1)
        actor.x = centre - actor.pivot[0]
        actor.y = floor_row - actor.pivot[1]
        top = actor.y + actor.bbox[1]
        if top < 0 or actor.x + actor.bbox[0] < 0 or actor.x + actor.bbox[2] >= WIDTH:
            raise SceneError(f"{actor.label!r} does not fit the 640x360 scene")
    for left, right in zip(actors, actors[1:]):
        if left.x + left.bbox[2] >= right.x + right.bbox[0]:
            raise SceneError(f"{left.label!r} and {right.label!r} overlap: {n} actors do not fit "
                             f"evenly in the {WIDTH} px wide scene")


def backdrop(height: int) -> Image.Image:
    dy = height - HEIGHT
    image = Image.new("RGBA", (WIDTH, height), BACKGROUND)
    draw_backdrop(image, dy)
    with Image.open(ROOT / "generated/art/dungeon_materials.png") as atlas:
        materials = atlas.convert("RGBA")
    frame0 = json.loads((ROOT / "generated/art/dungeon_materials.json").read_text(encoding="utf-8"))
    r = frame0["frames"][0]["frame"]
    materials = materials.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))
    draw_floor(image, materials, FLOOR_TOP + dy)
    return image


def scene(height: int, actors: list[Actor], annotated: bool = False) -> Image.Image:
    dy = height - HEIGHT
    image = backdrop(height)
    for actor in actors:
        image.alpha_composite(actor.image, (actor.x, actor.y + dy))
    if annotated:
        # 1 px dotted guides at the top row of a 48 px / 56 px tall sprite.
        floor_row = FLOOR_TOP - 1 + dy
        for guide, colour in GUIDE_HEIGHTS:
            y = floor_row - (guide - 1)
            for x in range(0, WIDTH, 8):
                for dx in (0, 1):
                    image.putpixel((x + dx, y), colour)
    return image


def nearest(image: Image.Image, factor: int) -> Image.Image:
    return image.resize((image.width * factor, image.height * factor), Image.Resampling.NEAREST)


def closeup(actor: Actor, margin: int = 12) -> Image.Image:
    """Actor alone on the real scene background, cropped, 6x nearest (no actors)."""
    image = backdrop(HEIGHT)
    image.alpha_composite(actor.image, (actor.x, actor.y))
    x0 = max(0, actor.x + actor.bbox[0] - margin)
    x1 = min(WIDTH, actor.x + actor.bbox[2] + 1 + margin)
    y0 = max(0, actor.y + actor.bbox[1] - margin)
    y1 = min(HEIGHT, FLOOR_TOP + margin)
    return nearest(image.crop((x0, y0, x1, y1)), ZOOM)


def with_pixel_grid(image: Image.Image) -> Image.Image:
    """Overlay a faint 1 px grid on the boundaries between source pixels."""
    overlay = Image.new("RGBA", image.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    line = (255, 255, 255, 30)
    for x in range(0, image.width, ZOOM):
        draw.line((x, 0, x, image.height - 1), fill=line)
    for y in range(0, image.height, ZOOM):
        draw.line((0, y, image.width - 1, y), fill=line)
    out = image.copy()
    out.alpha_composite(overlay)
    return out


def side_by_side(crops: list[Image.Image], gap: int = 24) -> Image.Image:
    """Bottom-aligned (shared floor) horizontal strip of 6x close-ups."""
    width = sum(c.width for c in crops) + gap * (len(crops) - 1)
    height = max(c.height for c in crops)
    out = Image.new("RGBA", (width, height), "#0d0f1a")
    x = 0
    for crop in crops:
        out.paste(crop, (x, height - crop.height))
        x += crop.width + gap
    return out


def comparison_sheet(actors: list[Actor], clean: Image.Image) -> Image.Image:
    """1x and 2x scenes with a text legend band outside the scene areas."""
    try:
        font = ImageFont.load_default(size=13)
        small = ImageFont.load_default(size=11)
    except TypeError:  # very old Pillow: fixed bitmap font
        font = small = ImageFont.load_default()
    margin = 16
    width = WIDTH * 2 + 2 * margin
    lines = [("GAMEPLAY-SCALE SPRITE COMPARISON - native 640x360 (top, 1x) and nearest-neighbour 2x (middle)", font),
             ("Same floor line, camera, lighting and backdrop. Review for scale only; the artistic decision is human.", small)]
    legend = []
    for actor in actors:
        w = actor.bbox[2] - actor.bbox[0] + 1
        h = actor.bbox[3] - actor.bbox[1] + 1
        tag = LEGEND_TAGS[actor.role]
        legend.append((f"{tag}: {actor.label}  -  opaque height {h} px, width {w} px, "
                       f"pivot {actor.pivot[0]},{actor.pivot[1]}, {len(actor.colors)} colours", small))
    legend.append(("Annotated variant guides (separate file only): cyan = 48 px, yellow = 56 px sprite top.", small))
    head_h = 12 + 20 * len(lines)
    foot_h = 12 + 16 * len(legend)
    sheet = Image.new("RGBA", (width, head_h + clean.height + margin + clean.height * 2
                               + margin + foot_h), "#0d0f1a")
    draw = ImageDraw.Draw(sheet)
    y = 8
    for text, fnt in lines:
        draw.text((margin, y), text, fill="#ebdcce", font=fnt)
        y += 20
    y = head_h
    sheet.paste(clean, (margin, y))
    y += clean.height + margin
    sheet.paste(nearest(clean, 2), (margin, y))
    y += clean.height * 2 + margin
    for text, fnt in legend:
        draw.text((margin, y), text, fill="#c7b7ad", font=fnt)
        y += 16
    return sheet


def build(sprite_specs: list[str], reference_spec: str | None, out_dir: Path,
          companion_specs: list[str] | tuple[str, ...] = ()) -> dict:
    """Explicit mode: candidates first, then companions, left to right."""
    entries = [(spec, "candidate") for spec in sprite_specs]
    entries += [(spec, "companion") for spec in companion_specs]
    return build_actors(entries, reference_spec, out_dir)


def build_actors(entries: list[tuple[str, str]], reference_spec: str | None,
                 out_dir: Path) -> dict:
    """Render ``(spec, role)`` entries (role candidate|companion) in the given order."""
    if not any(role == "candidate" for _spec, role in entries):
        raise SceneError("At least one candidate sprite is required")
    if not 1 <= len(entries) <= MAX_ACTORS:
        raise SceneError(f"Provide 1 to {MAX_ACTORS} candidate/companion actors "
                         f"(got {len(entries)}); the reference actor is extra")
    actors = []
    if reference_spec is not None:
        label, atlas, meta, frame = parse_spec(reference_spec)
        actors.append(load_actor(label, atlas, meta, frame, reference=True))
    for spec, role in entries:
        label, atlas, meta, frame = parse_spec(spec)
        actors.append(load_actor(label, atlas, meta, frame, role=role))
    slugs = [slug(a.label) for a in actors if a.role == "candidate"]
    if len(set(slugs)) != len(slugs):
        raise SceneError("Sprite labels must be distinct after sanitizing for filenames")
    place(actors)

    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = {}

    def save(name: str, image: Image.Image) -> None:
        image.convert("RGB").save(out_dir / name, optimize=False)
        outputs[name] = list(image.size)

    clean = scene(HEIGHT, actors)
    save("scene_640x360.png", clean)
    save("scene_640x360_annotated.png", scene(HEIGHT, actors, annotated=True))
    save("scene_1280x720_2x.png", nearest(clean, 2))
    deck = scene(DECK_HEIGHT, actors)
    save("scene_640x400.png", deck)
    save("scene_1280x800_2x.png", nearest(deck, 2))
    crops = []
    for actor, name in zip((a for a in actors if a.role == "candidate"), slugs):
        crop = closeup(actor)
        crops.append(crop)
        save(f"closeup_{name}_6x.png", crop)
        save(f"closeup_{name}_6x_grid.png", with_pixel_grid(crop))
    save("closeups_side_by_side_6x.png", side_by_side(crops))
    save("comparison_sheet.png", comparison_sheet(actors, clean))

    report = {
        "scene": {"native": [WIDTH, HEIGHT], "steam_deck": [WIDTH, DECK_HEIGHT],
                  "floor_contact_row": FLOOR_TOP - 1, "first_floor_row": FLOOR_TOP,
                  "steam_deck_extra_top_rows": DECK_HEIGHT - HEIGHT,
                  "guides_px_above_floor": [g for g, _ in GUIDE_HEIGHTS],
                  "pivot_rule": "contact row = bottom-most opaque row; pivot x = opaque bbox centre"},
        "actors": [{
            "label": a.label, "reference": a.reference, "role": a.role, "atlas": a.atlas,
            "metadata": a.metadata, "frame": a.frame_index,
            "frame_size": list(a.image.size),
            "opaque_height_px": a.bbox[3] - a.bbox[1] + 1,
            "opaque_width_px": a.bbox[2] - a.bbox[0] + 1,
            "opaque_bbox": list(a.bbox),
            "pivot": list(a.pivot),
            "scene_position": [a.x, a.y],
            "scene_bottom_opaque_row": a.y + a.bbox[3],
            "colors_used": a.colors, "color_count": len(a.colors),
        } for a in actors],
        "outputs": outputs,
    }
    (out_dir / "comparison.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                                             encoding="utf-8")
    return report


def _group_error(path: Path, message: str) -> SceneError:
    return SceneError(f"{path}: scale_study {message}")


def parse_memberships(path: Path, manifest: dict) -> list[dict]:
    """Validate a manifest's optional ``scale_study`` field into membership dicts."""
    raw = manifest.get("scale_study")
    if raw is None:
        return []
    items = raw if isinstance(raw, list) else [raw]
    if not items:
        raise _group_error(path, "must be an object or a non-empty list of objects")
    members = []
    for item in items:
        if not isinstance(item, dict):
            raise _group_error(path, "entries must be objects")
        unknown = sorted(set(item) - {"group", "order", "role"})
        if unknown:
            raise _group_error(path, f"has unknown keys {unknown}")
        group, role = item.get("group"), item.get("role")
        if not isinstance(group, str) or not GROUP_ID.fullmatch(group):
            raise _group_error(path, f"group {group!r} must match [A-Za-z0-9_-]+")
        if role not in ROLES:
            raise _group_error(path, f"role {role!r} must be one of {', '.join(ROLES)}")
        order = item.get("order", 0)
        if isinstance(order, bool) or not isinstance(order, int):
            raise _group_error(path, f"order {order!r} must be an integer")
        for key in ("id", "output_atlas", "output_metadata"):
            if not isinstance(manifest.get(key), str) or not manifest[key]:
                raise _group_error(path, f"needs manifest field {key!r}")
        members.append({"group": group, "role": role, "order": order, "id": manifest["id"],
                        "atlas": manifest["output_atlas"], "metadata": manifest["output_metadata"],
                        "manifest": path.name})
    groups = [m["group"] for m in members]
    if len(set(groups)) != len(groups):
        raise _group_error(path, "lists the same group twice")
    return members


def collect_groups(manifest_dir: Path = MANIFEST_DIR) -> dict[str, list[dict]]:
    """Group manifest memberships by id and validate each group's composition."""
    groups: dict[str, list[dict]] = {}
    for path in sorted(manifest_dir.glob("*.json")):
        try:
            manifest = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise SceneError(f"Cannot read manifest {path}: {exc}") from exc
        for member in parse_memberships(path, manifest):
            groups.setdefault(member["group"], []).append(member)
    for group, members in groups.items():
        if not any(m["role"] == "candidate" for m in members):
            raise SceneError(f"scale_study group {group!r} has no candidate")
        if sum(m["role"] == "reference" for m in members) > 1:
            raise SceneError(f"scale_study group {group!r} has more than one reference")
    return groups


def build_group(group: str, members: list[dict], out_dir: Path) -> dict:
    reference = next((m for m in members if m["role"] == "reference"), None)
    if reference is None:
        reference_spec = (f"{DEFAULT_REFERENCE_LABEL}={ROOT / 'generated/art/height_mannequin.png'}:"
                          f"{ROOT / 'generated/art/height_mannequin.json'}:0")
    else:
        reference_spec = f"{reference['id']}={ROOT / reference['atlas']}:{ROOT / reference['metadata']}:0"
    shown = sorted((m for m in members if m["role"] != "reference"),
                   key=lambda m: (m["order"], m["id"]))
    entries = [(f"{m['id']}={ROOT / m['atlas']}:{ROOT / m['metadata']}:0", m["role"]) for m in shown]
    try:
        return build_actors(entries, reference_spec, out_dir)
    except SceneError as exc:
        raise SceneError(f"scale_study group {group!r}: {exc}") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--sprite", action="append", default=[], metavar="LABEL=ATLAS:META[:FRAME]",
                        help="repeatable candidate sprite (1-6 with companions)")
    parser.add_argument("--companion", action="append", default=[], metavar="LABEL=ATLAS:META[:FRAME]",
                        help="repeatable companion actor shown for context (no close-ups)")
    parser.add_argument("--reference", metavar="LABEL=ATLAS:META[:FRAME]",
                        help="height reference actor (default: neutral 52 px mannequin, NOT Carl)")
    parser.add_argument("--no-reference", action="store_true", help="omit the reference actor")
    parser.add_argument("--list-groups", action="store_true",
                        help="validate manifests and print every scale_study group id")
    parser.add_argument("--group", metavar="ID", help="render a manifest-declared group")
    parser.add_argument("--manifest-dir", type=Path, default=MANIFEST_DIR,
                        help="manifest directory for --list-groups/--group")
    parser.add_argument("--out", type=Path,
                        help="output directory (default for --group: generated/art/scale_study/ID)")
    args = parser.parse_args(argv)
    try:
        if args.list_groups or args.group:
            if args.sprite or args.companion or args.reference or args.no_reference:
                raise SceneError("--list-groups/--group cannot be combined with explicit sprites")
            groups = collect_groups(args.manifest_dir)
            if args.list_groups:
                print("\n".join(sorted(groups)))
                return 0
            if args.group not in groups:
                raise SceneError(f"Unknown scale_study group {args.group!r}")
            out = args.out or ROOT / "generated/art/scale_study" / args.group
            report = build_group(args.group, groups[args.group], out)
        else:
            if args.out is None:
                parser.error("--out is required")
            out = args.out
            reference = None
            if not args.no_reference:
                reference = args.reference or (
                    f"{DEFAULT_REFERENCE_LABEL}={ROOT / 'generated/art/height_mannequin.png'}:"
                    f"{ROOT / 'generated/art/height_mannequin.json'}:0")
            report = build(args.sprite, reference, out, args.companion)
    except (SceneError, OSError, KeyError) as exc:
        print(f"SCALE_COMPARISON_ERROR: {exc}", file=sys.stderr)
        return 1
    for actor in report["actors"]:
        print(f"{actor['label']} [{actor['role']}]: height {actor['opaque_height_px']} px, "
              f"width {actor['opaque_width_px']} px, pivot {actor['pivot']}")
    print(f"wrote {len(report['outputs'])} images + comparison.json to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
