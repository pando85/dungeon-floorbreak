"""The design packet: everything a designer needs to start without exploring the repository.

``build_packet`` writes, into one directory:

- ``concept_crop_<n>.png``       the requested crops of the approved concept (the whole image, long side capped
                                 at 1024 px, when no crop is given); crops are for reference, never for tracing;
- ``palette_candidates.json`` and ``palette_swatches.png``
                                 median-cut quantization of each crop and of all crops together, at most 24
                                 colours; candidates only, the designer picks, ramps and names the real palette;
- ``scale_lineup.png``           the approved Carl (52 px), Donut (40 px) and goblin (36 px) idle frames at x1 and
                                 x4 with three empty slots for candidate heights;
- ``frame_pivot_template.json``  a draft ``pixelart-asset-v1`` config for the asset (readability-set tags);
- ``spec_template.md``           a pre-filled design spec modelled on ``docs/02-art/gob-01-goblin-spec.md``;
- ``excerpts.md``                optional doc excerpts (``--excerpt path#Heading``).
"""
from __future__ import annotations

import json
import os
import re
from pathlib import Path
from urllib.parse import unquote

from PIL import Image, ImageDraw

from .common import (BG, CYAN, GREY, GROUND_DARK, GROUND_LIGHT, REFERENCES, WHITE, YELLOW, ToolkitError, grid_images,
                     label, load_grid_file, opaque_bbox, rel, repo_path, up)
from .config import FORMAT

MAX_COLOURS = 24
ASSET_ID = re.compile(r"^[a-z0-9][a-z0-9-]*$")
CONCEPT_LONG_SIDE = 1024
SAMPLE_LONG_SIDE = 256

# The readability set: role -> (tag, frame-count range, playback, tick-window proposal, GOB-01 reference)
READABILITY = [
    ("idle", "idle", (2, 4), "loop", "8-12 ticks per frame, a 32-40 tick breathing cycle", "GOB-01: 12 + 8 + 12 + 8"),
    ("move", "walk", (4, 6), "loop", "5 ticks per frame, speed from the gameplay spec", "GOB-01: 6 x 5 = 30 ticks"),
    ("tell", "attack_tell", (1, 2), "once", "at least 18 ticks of readable hold, about 24 total", "GOB-01: 6 + 18"),
    ("attack", "attack_swing", (2, 3), "once", "smear 2, impact 3-4 (shown on exactly the hitbox-active ticks)",
     "GOB-01: 2 + 4, then recover 20"),
    ("hurt", "hurt", (1, 1), "once", "8-12 ticks", "GOB-01: 4 + 8"),
    ("defeat", "down", (2, 3), "hold", "floor impact 4, settle 8, then a state-driven hold", "GOB-01: 4 + 8 + hold"),
]
PROPOSED_TICKS = {"idle": [12, 8, 12, 8], "walk": [5, 5, 5, 5, 5, 5], "attack_tell": [6, 18],
                  "attack_swing": [2, 4], "hurt": [4, 8], "down": [4, 8, None]}
PROPOSED_FRAMES = {"idle": 3, "walk": 4, "attack_tell": 2, "attack_swing": 2, "hurt": 2, "down": 3}


def parse_crop(text: str) -> tuple[int, int, int, int]:
    try:
        x, y, w, h = (int(p) for p in text.split(","))
    except ValueError as exc:
        raise ToolkitError(f"crop must be x,y,w,h in pixels, got {text!r}") from exc
    if w <= 0 or h <= 0 or x < 0 or y < 0:
        raise ToolkitError(f"crop {text!r} must have positive size and a non-negative origin")
    return x, y, w, h


# ------------------------------------------------------------------ concept crops and palette candidates

def concept_crops(concept: Path, crops: list[tuple[int, int, int, int]]) -> list[Image.Image]:
    try:
        img = Image.open(concept).convert("RGBA")
    except OSError as exc:
        raise ToolkitError(f"cannot open the concept image: {exc}") from exc
    if not crops:
        scale = max(img.width, img.height) / CONCEPT_LONG_SIDE
        if scale > 1:
            img = img.resize((round(img.width / scale), round(img.height / scale)), Image.Resampling.LANCZOS)
        return [img]
    out = []
    for x, y, w, h in crops:
        if x + w > img.width or y + h > img.height:
            raise ToolkitError(f"crop {x},{y},{w},{h} is outside the {img.width}x{img.height} concept")
        out.append(img.crop((x, y, x + w, y + h)))
    return out


def opaque_pixels(img: Image.Image) -> list[tuple[int, int, int]]:
    """Opaque pixels of a nearest-sampled copy (no blended colours) capped near SAMPLE_LONG_SIDE."""
    scale = max(img.width, img.height) / SAMPLE_LONG_SIDE
    if scale > 1:
        img = img.resize((max(1, round(img.width / scale)), max(1, round(img.height / scale))), Image.Resampling.NEAREST)
    raw = img.convert("RGBA").tobytes()
    return [(raw[i], raw[i + 1], raw[i + 2]) for i in range(0, len(raw), 4) if raw[i + 3] >= 128]


def quantize(pixels: list[tuple[int, int, int]], colours: int = MAX_COLOURS) -> list[dict]:
    """Median-cut palette of the pixels: ``[{hex, share}]`` by descending share (ties by hex), at most 24."""
    if not pixels:
        return []
    colours = max(1, min(colours, MAX_COLOURS))
    strip = Image.new("RGB", (len(pixels), 1))
    strip.putdata(pixels)
    q = strip.quantize(colors=colours, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    pal = q.getpalette() or []
    counts = q.getcolors(maxcolors=256) or []
    total = sum(c for c, _ in counts)
    rows = [{"hex": "#%02x%02x%02x" % tuple(pal[i * 3:i * 3 + 3]), "share": round(c / total, 4)} for c, i in counts]
    return sorted(rows, key=lambda r: (-r["share"], r["hex"]))[:colours]


def palette_candidates(crops: list[Image.Image], colours: int = MAX_COLOURS) -> dict:
    per_crop = [opaque_pixels(c) for c in crops]
    return {"format": "pixelart-palette-candidates-v1", "max_colours": min(colours, MAX_COLOURS),
            "note": "Candidates from median-cut quantization of the concept crops. The designer chooses, ramps and names the real palette; do not trace.",
            "crops": [{"crop": i + 1, "colours": quantize(p, colours)} for i, p in enumerate(per_crop)],
            "merged": quantize([px for p in per_crop for px in p], colours)}


def swatch_sheet(cands: dict) -> Image.Image:
    sets = [(f"crop {c['crop']}", c["colours"]) for c in cands["crops"]]
    if len(cands["crops"]) > 1:
        sets.append(("all crops", cands["merged"]))
    sw, gap, cols = 40, 6, 12
    rows_per = [max(1, -(-len(cs) // cols)) for _n, cs in sets]
    height = sum(24 + r * (sw + 18 + gap) for r in rows_per) + 8
    sheet = Image.new("RGB", (cols * (sw + gap) + 12, height), BG)
    d = ImageDraw.Draw(sheet)
    y = 6
    for (name, colours), rows in zip(sets, rows_per):
        label(d, (6, y), f"{name}: {len(colours)} candidate colours (by share)", WHITE, 11)
        y += 22
        for i, c in enumerate(colours):
            r, col = divmod(i, cols)
            x, yy = 6 + col * (sw + gap), y + r * (sw + 18 + gap)
            d.rectangle([x, yy, x + sw - 1, yy + sw - 1], fill=c["hex"], outline=GREY)
            label(d, (x, yy + sw + 1), c["hex"], GREY, 9)
        y += rows * (sw + 18 + gap) + 2
    return sheet


# ------------------------------------------------------------------ scale lineup

def reference_sprites() -> list[tuple[str, Image.Image, tuple[int, int], int]]:
    """``(name, idle frame, pivot, opaque height)`` of the approved Carl, Donut and goblin."""
    out = []
    for name, (path, _h) in REFERENCES.items():
        grid = load_grid_file(repo_path(path))
        img = grid_images(grid)[0]
        x0, y0, x1, y1 = opaque_bbox(img)
        out.append((name, img, tuple(grid["pivot"]), y1 - y0 + 1))
    return out


def scale_lineup(heights: list[int] | None = None, slot_width: int = 48) -> Image.Image:
    """Approved idle frames at x1 (top) and x4 (bottom) on one floor, with three empty candidate slots.

    Slots are outlined boxes ``slot_width`` wide; with ``heights`` each is as tall as the candidate, so the
    designer sees it against the references. Dotted guides mark the three reference heights.
    """
    refs = reference_sprites()
    slots = (list(heights) + [None] * 3)[:3] if heights else [None] * 3
    max_h = max([r[3] for r in refs] + [h or 0 for h in slots] + [52])
    pad, gap, floor_h = 8, 14, 5
    items = []
    for name, img, piv, h in refs:
        x0, y0, x1, y1 = opaque_bbox(img)
        items.append((f"{name} {h} px", img.crop((x0, y0, x1 + 1, y1 + 1)), x1 - x0 + 1))
    width = pad * 2 + sum(i[2] for i in items) + 3 * slot_width + gap * (len(items) + 3 - 1)
    rows = []
    for k in (1, 4):
        canvas = Image.new("RGBA", (width, max_h + floor_h + 14 + (16 if k == 1 else 0)), BG + (255,))
        d = ImageDraw.Draw(canvas)
        floor_y = max_h + 14
        d.rectangle([0, floor_y, width - 1, floor_y], fill=GROUND_LIGHT)
        d.rectangle([0, floor_y + 1, width - 1, floor_y + floor_h - 1], fill=GROUND_DARK)
        x = pad
        for name, img, w in items:
            canvas.alpha_composite(img, (x, floor_y - img.height))
            x += w + gap
        slot_x = []
        for i in range(3):
            h = slots[i] or 40
            d.rectangle([x, floor_y - h, x + slot_width - 1, floor_y - 1], outline=CYAN)
            slot_x.append(x)
            x += slot_width + gap
        big = up(canvas, k)
        d = ImageDraw.Draw(big)
        for h, colour in ((36, GREY), (40, GREY), (52, GREY)):
            y = (floor_y - h) * k
            for gx in range(0, big.width, 8):
                d.line([(gx, y), (gx + 3, y)], fill=colour)
            label(d, (2, y - 12), f"{h} px", colour, 11)
        xx = pad * k
        for name, img, w in items:
            label(d, (xx, (floor_y + 1) * k + 2), name, WHITE, 11)
            xx += (w + gap) * k
        for i, sx in enumerate(slot_x):
            label(d, (sx * k + 2, (floor_y + 1) * k + 2), f"candidate {'ABC'[i]}: {slots[i] or '??'} px", CYAN, 11)
        rows.append((k, big))
    sheet = Image.new("RGB", (max(r[1].width for r in rows), sum(r[1].height + 26 for r in rows)), BG)
    d = ImageDraw.Draw(sheet)
    y = 0
    for k, big in rows:
        label(d, (6, y + 5), f"approved idle frames and three empty candidate slots, x{k} (grey guides: 36, 40, 52 px)", YELLOW, 11)
        sheet.paste(big.convert("RGB"), (0, y + 24))
        y += big.height + 26
    return sheet


# ------------------------------------------------------------------ templates

def frame_template(asset_id: str, frame_size=(64, 56), pivot=(28, 55)) -> dict:
    """A draft ``pixelart-asset-v1`` config with the readability set; the designer adjusts every number."""
    tags, frame = {}, 0
    for role, tag, _rng, playback, _prop, _ref in READABILITY:
        n = PROPOSED_FRAMES[tag]
        ticks = (PROPOSED_TICKS[tag] + [6] * n)[:n]
        tags[tag] = {"playback": playback, "contact": "grounded",
                     "frames": [[frame + i, ticks[i]] for i in range(n)]}
        frame += n
    return {
        "format": FORMAT, "asset_id": asset_id,
        "grid": f"assets/source/<kind>/{asset_id}/{asset_id}.grid.json",
        "manifest": f"assets/manifests/{asset_id}.json",
        "frame_size": list(frame_size), "pivot": list(pivot), "contact_row": pivot[1], "tick_hz": 60, "hold_ms": 100,
        "facing": "right", "tags": tags,
        "roles": {role: tag for role, tag, *_ in READABILITY},
        "rules": {"max_colors": 20, "max_isolated_per_frame": 0, "max_orphans_per_frame": 9,
                  "canvas_margin": [0, 0, 0, 0], "unique_frames": True,
                  "frozen_keys": {"fixture": f"tests/fixtures/{asset_id}_keys.json", "frames": [], "layers": ["body"]},
                  "bbox": [], "key_counts": []},
        "review": {"strip_tags": ["attack_tell", "attack_swing"]},
        "_template_note": "Draft. Set sizes, pivot, frame counts, tick windows and key frames from the design spec; "
                          "remove _template_note and every unused rule before use.",
    }


def spec_template(asset_id: str, cands: dict | None, tmpl: dict, heights: list[int] | None,
                  title: str | None = None) -> str:
    name = title or asset_id.upper()
    pal_rows = "\n".join(f"| {i + 1} | `{c['hex']}` | {c['share'] * 100:.1f} % | <fill: key, name, role> |"
                         for i, c in enumerate((cands or {}).get("merged", [])[:MAX_COLOURS])) or "| 1 | <fill> | | |"
    h = heights or [None, None, None]
    size_rows = "\n".join(f"| {'ABC'[i]} | {h[i] if i < len(h) and h[i] else '<fill>'} px | <fill> | <fill> | <fill> |" for i in range(3))
    anim_rows = "\n".join(
        f"| {role} | `{tag}` | {lo if lo == hi else f'{lo}-{hi}'} | {pb} | {prop} | {ref} |"
        for role, tag, (lo, hi), pb, prop, ref in READABILITY)
    frame_rows = []
    for role, tag, *_ in READABILITY:
        for f, ticks in tmpl["tags"][tag]["frames"]:
            frame_rows.append(f"| {f} | `{tag}` | <fill: pose> | {'hold' if ticks is None else ticks} | <fill: key or Sonnet> |")
    return f"""# {name}: design spec (template)

**Status:** draft. Design: <fill: model>; execution: <fill: model>; review: <fill: a separate top-tier model>.

**Decision record:** height <fill> px, frame <fill> x <fill>, pivot (<fill>, <fill>). Proposed by <fill>; decided by <fill>.

The sections follow `docs/02-art/gob-01-goblin-spec.md`. Replace every `<fill: ...>`; write numbers, not adjectives. The design step stops at the keys and this spec (see the top-tier budget in `docs/07-agents/agent-orchestration.md`).

## 1. Canon and tone constraints

- Approved concept: <fill: concept file and the approval record>.
- Tone (`docs/01-design/antihero-tone.md`): <fill: how this asset reads as a believable, slightly ridiculous or nasty everyman-world threat, never a heroic design>.
- Canon facts that must survive in pixels: <fill: 3 to 5 bullets>.

## 2. Size decision

Carl is 52 px (about 6 ft), Donut 40 px, the goblin 36 px (see `scale_lineup.png`).

| Option | Idle opaque height | Ratio to Carl | Implied real height | Why / why not |
| --- | --- | --- | --- | --- |
{size_rows}

Decision: <fill: height and the reason, one paragraph; the proposer and the decider are different agents>.

## 3. Frame, pivot, boxes and atlas

- Frame <fill> x <fill>, pivot (<fill>, <fill>), contact row <fill> (the lowest opaque row of every grounded frame).
- Grounded tags: <fill>. Airborne tags (no pixel on the contact row): <fill or none>.
- Standing box <fill> x <fill>; hurtbox <fill>; attack hitbox <fill> on the impact frame.
- Authored facing right; the runtime mirrors around the pivot column.
- One atlas, frames 0 to <fill>, layers: <fill: for example body, weapon>.

## 4. Readability-set animation list

Readability set: idle, move, tell, attack, hurt, defeat (about 12 to 20 frames until the asset has a gameplay work order). Tick windows are at 60 Hz.

| Role | Tag | Frames | Playback | Tick-window proposal | Reference |
| --- | --- | --- | --- | --- | --- |
{anim_rows}

Frame table (`frame_pivot_template.json` carries the same rows):

| # | Tag | Pose | Ticks | Author |
| --- | --- | --- | --- | --- |
{chr(10).join(frame_rows)}

## 5. State diagram

```mermaid
stateDiagram-v2
    [*] --> idle
    idle --> move: <fill: trigger>
    move --> idle: <fill: trigger>
    idle --> tell: <fill: in range>
    move --> tell: <fill: in range>
    tell --> attack: tell ticks elapsed
    attack --> idle: recover elapsed
    idle --> hurt: hit
    hurt --> idle: hurt ticks elapsed
    hurt --> defeat: health 0
    defeat --> [*]
```

## 6. Palette

At most <fill: 20> colours. Candidates from the concept crop (median cut, by pixel share; see `palette_candidates.json`). These are candidates, not decisions.

| # | Candidate | Share | Decision (key, name, role) |
| --- | --- | --- | --- |
{pal_rows}

The chosen palette must separate this asset from the warm brick and the cold arch of Floor 1 at 1x, and fit the A1 lighting.

## 7. Key poses

One key per readability tag (about 6 to 8 keys): idle 0, move contact, tell hold, attack impact, hurt, defeat. Each key lists silhouette, joint positions, palette use and the readability reason. Keys are frozen once the review passes (`pixelart freeze`).

| Frame | Tag | Silhouette and pose notes | Readability reason |
| --- | --- | --- | --- |
| <fill> | <fill> | <fill> | <fill> |

## 8. In-between list

For the execution tier; no design decisions needed. One row per non-key frame, as in section 9 of the GOB-01 spec.

| # | Pose notes |
| --- | --- |
| <fill> | <fill> |

## 9. Validators and readability rules

Run by `python3 -m scripts.pixelart validate <asset-config>` (rules V01 to V15 in `scripts/pixelart/validators.py`). Parametrize in the asset config:

- V04 palette: at most <fill> colours; V05 isolated pixels: 0; V06 orphan pixels: at most <fill> per frame.
- V07/V08 contact rows: grounded tags end on the contact row, airborne tags never touch it.
- V09 frozen keys: SHA-256 per key frame, fixture `tests/fixtures/{asset_id}_keys.json`.
- V13/V14 readability rules (write each as a bbox or key-count rule): <fill: for example the tell is the highest point, the hurt eyes are white>.

## 10. Evidence

```text
python3 -m scripts.pixelart validate <asset-config> --out <dir>
python3 -m scripts.pixelart evidence <asset-config> --out <dir>
python3 -m scripts.pixelart review-pack <asset-config> --out <dir>
```

The review pack holds exactly three images (contact sheet x4, lit scene 640 x 360 shown x3, tell to attack strip) and the validator report.

## 11. Files

| File | Role |
| --- | --- |
| `assets/source/<fill>/{asset_id}.grid.json` / `.ase` | pixel-grid source and Aseprite master |
| `assets/manifests/{asset_id}.json` | manifest |
| `<fill>/{asset_id}.pixelart.json` | the asset config (from `frame_pivot_template.json`) |
| `tests/fixtures/{asset_id}_keys.json` | frozen keys |

## 12. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Record in the PR's `Rights/Provenance` section which model designed, executed and reviewed the art. No tracing of the concept, no image-model raster output.
"""


# ------------------------------------------------------------------ excerpts

# Same shape as ``scripts/check_docs.py``'s link pattern, but captures the parts so the destination can be replaced.
_MD_LINK = re.compile(r"((?<!!)\[[^]]+\]\()([^)]+)(\))")


def rebase_links(text: str, source: Path, dest_dir: Path) -> str:
    """Rewrite relative Markdown link targets so they resolve from ``dest_dir`` instead of ``source``'s folder.

    URLs, ``mailto:`` and pure ``#anchor`` links stay as they are. A link keeps its fragment and optional title.
    """
    def fix(m: re.Match) -> str:
        inner = m.group(2)
        parts = inner.strip().split(maxsplit=1)
        if not parts:
            return m.group(0)
        raw = parts[0]
        dest = raw.strip("<>")
        if dest.startswith(("https://", "http://", "mailto:", "#")) or not dest:
            return m.group(0)
        location, hash_, frag = dest.partition("#")
        location, q, query = location.partition("?")
        target = (source.parent / unquote(location)).resolve()
        new = Path(os.path.relpath(target, dest_dir.resolve())).as_posix().replace(" ", "%20")
        new += (q + query if q else "") + (hash_ + frag if hash_ else "")
        title = f" {parts[1]}" if len(parts) > 1 else ""
        return f"{m.group(1)}{new}{title}{m.group(3)}"
    return _MD_LINK.sub(fix, text)


def excerpt(spec: str, limit: int = 60, dest_dir: Path | None = None) -> str:
    """``path`` or ``path#Heading``: the file's first ``limit`` lines, or the section under a matching heading.

    With ``dest_dir`` (the folder the excerpt file is written to) relative links are rebased to resolve from there.
    """
    path_text, _, heading = spec.partition("#")
    source = repo_path(path_text)
    lines = source.read_text(encoding="utf-8").splitlines()
    if heading:
        start = next((i for i, ln in enumerate(lines) if ln.startswith("#") and heading.lower() in ln.lower()), None)
        if start is None:
            raise ToolkitError(f"no heading containing {heading!r} in {path_text}")
        level = len(lines[start]) - len(lines[start].lstrip("#"))
        end = next((i for i in range(start + 1, len(lines))
                    if lines[i].startswith("#") and len(lines[i]) - len(lines[i].lstrip("#")) <= level), len(lines))
        lines = lines[start:end]
    shown = lines[:limit]
    if dest_dir is not None:
        shown = rebase_links("\n".join(shown), source, dest_dir).split("\n")
    more = f"\n(truncated: {len(lines) - limit} more lines in {path_text})" if len(lines) > limit else ""
    return f"## {spec}\n\n" + "\n".join(shown) + more + "\n"


# ------------------------------------------------------------------ entry point

def build_packet(asset_id: str, concept: Path, crops: list[tuple[int, int, int, int]], out: Path,
                 colours: int = MAX_COLOURS, heights: list[int] | None = None,
                 excerpts: list[str] | None = None, frame_size=(64, 56), pivot=(28, 55)) -> list[Path]:
    title = asset_id.upper()
    asset_id = asset_id.lower()  # work-order ids such as LLAMA-01 are accepted; files and folders use lowercase
    if not ASSET_ID.match(asset_id):
        raise ToolkitError(f"asset id must be letters, digits and dashes (case is normalised to lowercase): {title!r}")
    out.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    images = concept_crops(concept, crops)
    for i, img in enumerate(images, start=1):
        path = out / f"concept_crop_{i}.png"
        img.save(path)
        written.append(path)
    cands = palette_candidates(images, colours)
    path = out / "palette_candidates.json"
    path.write_text(json.dumps(cands, indent=2) + "\n", encoding="utf-8")
    written.append(path)
    path = out / "palette_swatches.png"
    swatch_sheet(cands).save(path)
    written.append(path)
    path = out / "scale_lineup.png"
    scale_lineup(heights).save(path)
    written.append(path)
    tmpl = frame_template(asset_id, frame_size, pivot)
    path = out / "frame_pivot_template.json"
    path.write_text(json.dumps(tmpl, indent=2) + "\n", encoding="utf-8")
    written.append(path)
    path = out / "spec_template.md"
    path.write_text(spec_template(asset_id, cands, tmpl, heights, title=title), encoding="utf-8")
    written.append(path)
    if excerpts:
        path = out / "excerpts.md"
        path.write_text("# Doc excerpts\n\n" + "\n".join(excerpt(e, dest_dir=out) for e in excerpts), encoding="utf-8")
        written.append(path)
    return written


def describe(files: list[Path]) -> str:
    return "\n".join(f"  {rel(f)}" for f in files)
