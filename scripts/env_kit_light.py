#!/usr/bin/env python3
"""Presentation-only lighting pass for env-kit-v2 layouts (Pillow only, deterministic).

Renders the A1 lighting model on top of the unlit pixel layers produced by
``scripts/env_kit.py`` from the **exported atlases and the layout only**:

* every layer is cropped to a view and nearest-upscaled by an integer factor;
* each layer is multiplied by its light group's map, computed at the *output*
  resolution: the group's cold ambient plus warm emitter pools (torch flames, floor
  pools projected onto the walking line, gallery bounce/sky, distant gallery torches);
* parallax groups are mixed toward a haze colour (constant + ground fog);
* emitters with ``glow`` add a halo; bloom is fed only by the emissive layers.

Emitters come from the kit contract (``pieces[].lights``) and their placement in the
layout; ambient, haze and bloom come from the layout's ``lighting`` profile. The
falloff sprites, bloom and readability metric are shared with the A1 look-dev
compositor (``scripts/lookdev_compose.py``). Nothing here changes collision or
gameplay data. The output is an art-review aid, not a Bevy capture.

Usage::

    python3 scripts/env_kit_light.py --layout assets/levels/floor01/corridor_abc_sample.json \
        --out docs/evidence/floor01-catacomb-kit
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageMath

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import env_kit as ek  # noqa: E402
from lookdev_compose import (_add_scaled, bloom, colour_count, falloff_sprite, hex_rgb,  # noqa: E402
                             readability)


# ---------------------------------------------------------------------------
# Emitters
# ---------------------------------------------------------------------------

def gather_lights(kit: ek.Kit, layout: dict, view: list[int], time_ms: int = 0) -> list[dict]:
    """All emitters that can reach the view, in view pixels."""
    vx, vy, vw, vh = view
    cols, rows = layout["grid_size"]
    tops = ek.collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    found = []

    def add(light: dict, wx: float, wy: float, frame: int) -> None:
        if light.get("project") == "walk_line":
            col = int(wx) // ek.TILE
            top = tops[col] if 0 <= col < cols else None
            if not isinstance(top, int):
                return
            wy = top + light.get("project_dy", 0)
        k = light.get("flicker", [1.0])[frame % len(light.get("flicker", [1.0]))]
        r = light["radius"]
        x, y = wx - vx, wy - vy
        if x + r < 0 or x - r > vw or y + r < 0 or y - r > vh:
            return
        found.append({"x": x, "y": y, "radius": r, "colour": light["colour"],
                      "intensity": light["intensity"] * k, "group": light["group"],
                      "squash_y": light.get("squash_y", 1.0),
                      "glow_radius": light.get("glow", {}).get("radius", 0.0),
                      "additive": light.get("glow", {}).get("additive", 0.0) * k})

    for name, items in layout["piece_layers"].items():
        for item in items:
            piece = kit.pieces[item["piece"]]
            if "lights" not in piece:
                continue
            _rect, frame = ek.anim_rect(piece, item, time_ms)
            for light in piece["lights"]:
                add(light, item["x"] + light["offset"][0], item["y"] + light["offset"][1], frame)
    for name, spec in layout["parallax_layers"].items():
        piece = kit.pieces[spec["piece"]]
        sw = piece["rect"][2]
        start = round(vx + ek.parallax_offset(layout, kit, name, vx))
        x0 = start % sw - sw if start % sw else 0
        for light in piece.get("lights", []):
            x = x0
            while x < layout["pixel_size"][0] + sw:
                add(light, x + light["offset"][0], spec["y"] + light["offset"][1], 0)
                x += sw
    return found


# ---------------------------------------------------------------------------
# Light maps at output resolution
# ---------------------------------------------------------------------------

def light_maps(size, lights: list[dict], group: str, cfg: dict, k: int, kind: str = "pool",
               scale: float = 1.0, base=None, source_group: str | None = None) -> list[Image.Image]:
    base = tuple(cfg["ambient"]) if base is None else base
    bufs = [Image.new("F", size, float(base[i])) for i in range(3)]
    for light in lights:
        if light["group"] != (source_group or group):
            continue
        mult = light["intensity"] if kind == "pool" else light["additive"]
        if mult <= 0.0:
            continue
        radius = light["radius"] if kind == "pool" else light["glow_radius"]
        if radius <= 0:
            continue
        diameter = max(2, round(2 * radius * k))
        height = max(2, round(diameter * (light["squash_y"] if kind == "pool" else 1.0)))
        sprite = falloff_sprite(kind).resize((diameter, height), Image.Resampling.BICUBIC)
        left = round(light["x"] * k - diameter / 2)
        top = round(light["y"] * k - height / 2)
        rgb = hex_rgb(light["colour"])
        for i in range(3):
            _add_scaled(bufs[i], sprite, left, top, scale * mult * rgb[i] / 255.0)
    return bufs


def haze_map(size, haze: dict, view: list[int], k: int) -> Image.Image | None:
    amount = haze.get("amount", 0.0)
    fog = haze.get("floor_fog")
    if not fog:
        return Image.new("F", size, float(amount)) if amount else None
    col = []
    for y in range(size[1]):
        ly = (y + 0.5) / k + view[1]
        t = (ly - fog["from_y"]) / (fog["to_y"] - fog["from_y"])
        t = 0.0 if t < 0 else 1.0 if t > 1 else t * t * (3 - 2 * t)
        col.append(amount + fog["amount"] * t)
    strip = Image.new("F", (1, size[1]))
    strip.putdata(col)
    return strip.resize(size, Image.Resampling.NEAREST)


def shade(img: Image.Image, maps: list[Image.Image], haze: dict | None, hmap) -> Image.Image:
    r, g, b, a = img.split()
    chans = [ImageMath.lambda_eval(lambda v: v["c"] * v["l"], c=ch.convert("F"), l=m)
             for ch, m in zip((r, g, b), maps)]
    if haze and hmap is not None:
        hc = hex_rgb(haze["colour"])
        chans = [ImageMath.lambda_eval(lambda v: v["c"] * (1.0 - v["h"]) + v["h"] * hv, c=cf, h=hmap)
                 for cf, hv in zip(chans, hc)]
    out = Image.merge("RGB", [cf.convert("L") for cf in chans]).convert("RGBA")
    out.putalpha(a)
    return out


def contact_shadow(size, boxes, view, k: int, cfg: dict) -> Image.Image | None:
    """Soft dark ellipse under each actor's feet (presentation only; A1 baked these)."""
    if not boxes or not cfg:
        return None
    buf = Image.new("F", size, 0.0)
    for (x0, y0, x1, y1) in boxes:
        half_w = (x1 - x0) * cfg["width_scale"] / 2
        diameter = max(2, round(2 * half_w * k))
        height = max(2, round(2 * cfg["half_height"] * k))
        sprite = falloff_sprite("pool").resize((diameter, height), Image.Resampling.BICUBIC)
        cx, cy = ((x0 + x1) / 2 - view[0]) * k, (y1 + 0.5 - view[1]) * k
        _add_scaled(buf, sprite, round(cx - diameter / 2), round(cy - height / 2), cfg["strength"])
    return ImageMath.lambda_eval(lambda v: 1.0 - v["s"], s=buf)


def render_lit(kit: ek.Kit, layout: dict, view: list[int], k: int, actors: Image.Image | None = None,
               time_ms: int = 0, lit: bool = True, boxes=None) -> Image.Image:
    """Composite one view at k x its size (k = integer output scale)."""
    vx, vy, vw, vh = view
    size = (vw * k, vh * k)
    layers = ek.render_layers(kit, layout, time_ms, camera_x=vx)
    profile = layout["lighting"]
    groups = profile["groups"]
    lights = gather_lights(kit, layout, view, time_ms)
    emissive_names = set(profile["bloom"]["emissive_layers"])
    stack = []
    for layer in sorted(kit.contract["layers"], key=lambda l: l["draw_order"]):
        stack.append((layer["draw_order"], layer["name"], layer["light_group"], layers[layer["name"]]))
    if actors is not None:
        stack.append((kit.contract["actor_draw_order"], "actors", kit.contract["actor_light_group"], actors))
    stack.sort(key=lambda s: s[0])
    canvas = Image.new("RGBA", size, (0, 0, 0, 255))
    emissive = Image.new("RGBA", size, (0, 0, 0, 255))
    cache: dict[str, tuple] = {}
    cover = None
    for _order, name, group, full in stack:
        img = full.crop((vx, vy, vx + vw, vy + vh)).resize(size, Image.Resampling.NEAREST)
        if lit:
            if group not in cache:
                cfg = groups[group]
                src = group if group.startswith("parallax") else "scene"
                maps = light_maps(size, lights, group, cfg, k, scale=cfg.get("light_scale", 1.0),
                                  source_group=src)
                if group == "scene":
                    shadow = contact_shadow(size, boxes, view, k, profile.get("actor_contact_shadow"))
                    if shadow is not None:
                        maps = [ImageMath.lambda_eval(lambda v: v["m"] * v["s"], m=m, s=shadow)
                                for m in maps]
                haze = cfg.get("haze")
                cache[group] = (maps, haze, haze_map(size, haze, view, k) if haze else None)
            maps, haze, hmap = cache[group]
            img = shade(img, maps, haze, hmap)
        canvas.alpha_composite(img)
        if lit:
            if name in emissive_names:
                emissive.alpha_composite(img)
            else:  # nearer opaque layers hide emissive pixels behind them (no bloom through walls)
                occluder = Image.new("RGBA", size, (0, 0, 0, 0))
                occluder.putalpha(img.getchannel("A"))
                emissive.alpha_composite(occluder)
            if not group.startswith("parallax"):
                cover = ImageChops.lighter(cover, img.getchannel("A")) if cover else img.getchannel("A")
    if not lit:
        return canvas.convert("RGB")
    rgb = canvas.convert("RGB")
    visible = ImageChops.invert(cover) if cover else None
    for group in ("parallax_far", "parallax_mid", "scene"):
        glow = light_maps(size, lights, group, groups[group], k, kind="glow", base=(0.0, 0.0, 0.0))
        add = Image.merge("RGB", [ImageMath.lambda_eval(lambda v: v["g"] * 255.0, g=g).convert("L")
                                  for g in glow])
        if group.startswith("parallax") and visible is not None:  # halos only where the gallery shows
            add = ImageChops.multiply(add, Image.merge("RGB", [visible] * 3))
        rgb = ImageChops.add(rgb, add)
    return bloom(rgb.convert("RGBA"), emissive, profile["bloom"], k).convert("RGB")


# ---------------------------------------------------------------------------
# Evidence
# ---------------------------------------------------------------------------

def label(img: Image.Image, text: str) -> Image.Image:
    band = 24
    out = Image.new("RGB", (img.width, img.height + band), (16, 14, 20))
    out.paste(img, (0, band))
    ImageDraw.Draw(out).text((8, 6), text, fill=(236, 224, 200))
    return out


def save(img: Image.Image, path: Path, written: list[Path]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, format="PNG", optimize=False)
    written.append(path)


def actor_mask(actors: Image.Image, view: list[int]) -> Image.Image:
    vx, vy, vw, vh = view
    return actors.crop((vx, vy, vx + vw, vy + vh)).getchannel("A")


def measure(img: Image.Image, mask: Image.Image, boxes, view) -> list[dict]:
    vx, vy = view[0], view[1]
    local = [(x0 - vx, y0 - vy, x1 - vx, y1 - vy) for (x0, y0, x1, y1) in boxes]
    return readability(img, mask, local, ring=6)


def render_evidence(layout_path: str, out: Path) -> dict:
    layout = ek.load_json(layout_path)
    kit = ek.load_kit(layout["kit_contract"])
    ek.validate_contract(kit)
    ek.validate_layout(kit, layout)
    written: list[Path] = []
    preview = layout["preview"]
    primary = layout["views"][preview["primary_view"]]
    deck_view = layout["views"].get("abc_640x400")
    pair = preview["actor_check"][:2]  # Carl + Donut in the primary view (A1 positions)
    actors, boxes = ek.actor_layer(kit, layout, pair)
    all_actors, all_boxes = ek.actor_layer(kit, layout)
    report: dict = {"format": "env-kit-lit-evidence-v1", "layout": layout_path,
                    "note": "Presentation-only lighting pass on exported atlases; art preview, not a Bevy capture.",
                    "files": []}

    native = render_lit(kit, layout, primary, 1, actors, boxes=boxes)
    save(native, out / "lit_abc_640x360.png", written)
    save(render_lit(kit, layout, primary, 3, actors, boxes=boxes), out / "lit_abc_1920x1080_x3.png", written)
    if deck_view:
        save(render_lit(kit, layout, deck_view, 2, actors, boxes=boxes), out / "lit_abc_1280x800_640x400_x2.png", written)
    env_only = render_lit(kit, layout, primary, 1)
    save(env_only, out / "lit_abc_env_only_640x360.png", written)
    w, h = layout["pixel_size"]
    save(render_lit(kit, layout, [0, 0, w, h], 1, all_actors, boxes=all_boxes), out / "lit_overview_2048x400.png", written)
    unlit = render_lit(kit, layout, primary, 1, actors, lit=False)

    # Side-by-side with the owner-approved A1 target frame (both nearest 2x).
    target = ek.repo_path(preview["target_frame"])
    with Image.open(target) as opened:
        a1 = opened.convert("RGB")
    left = label(ek.scale(native.convert("RGBA"), 2).convert("RGB"),
                 "ENV-01 rework 3: modular kit + sample layout, lit (A1 lighting model), 640x360 nearest 2x")
    right = label(ek.scale(a1.convert("RGBA"), 2).convert("RGB"),
                  "A1 TARGET FRAME (owner-approved look-dev, 640x360 nearest 2x)")
    sbs = Image.new("RGB", (left.width * 2 + 16, left.height), (16, 14, 20))
    sbs.paste(left, (0, 0))
    sbs.paste(right, (left.width + 16, 0))
    save(sbs, out / "lit_vs_a1_target_side_by_side.png", written)

    # Actor readability close-up (lit, nearest 4x crop around Carl and Donut).
    x0 = min(b[0] for b in boxes) - primary[0] - 24
    y0 = min(b[1] for b in boxes) - primary[1] - 24
    x1 = max(b[2] for b in boxes) - primary[0] + 24
    y1 = max(b[3] for b in boxes) - primary[1] + 16
    crop = native.crop((max(0, x0), max(0, y0), x1, y1))
    save(label(ek.scale(crop.convert("RGBA"), 4).convert("RGB"), "Actor check (lit, nearest 4x crop): "
               "approved Carl 52 / Donut 40 on the walking line, 0 px sink"), out / "actor_check_lit_x4.png",
         written)

    mask = actor_mask(actors, primary)
    names = ["carl_h52", "donut_h40"]
    a1_meas = json.loads((ROOT / "docs/evidence/lookdev-floor01-resolution-spike/measurements.json")
                         .read_text(encoding="utf-8"))
    report["readability"] = {
        "method": "lookdev_compose.readability: mean CIE L* of actor pixels vs a 6 px ring around the "
                  "actor box (actor pixels excluded); edge = mean |dL*| between silhouette pixels and "
                  "their non-actor 4-neighbours; native 640x360.",
        "lit": dict(zip(names, measure(native, mask, boxes, primary))),
        "unlit": dict(zip(names, measure(unlit, mask, boxes, primary))),
        "a1_target_reference": a1_meas.get("frames", {}).get("a1", {}).get("readability"),
    }
    report["colours"] = {
        "lit_native_640x360": colour_count(native),
        "atlases": {name: len(a["grid"]["palette"]) for name, a in kit.atlases.items()},
        "kit_palette_total": len(kit.contract["palette"]),
    }
    report["files"] = [p.relative_to(out).as_posix() for p in written]
    report["sha256"] = {p.relative_to(out).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in written}
    (out / "measurements.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--layout", default="assets/levels/floor01/corridor_abc_sample.json")
    parser.add_argument("--out", type=Path, default=ROOT / "generated/art/env_kit_lit")
    args = parser.parse_args(argv)
    try:
        report = render_evidence(args.layout, args.out)
    except (ek.KitError, OSError, KeyError) as exc:
        print(f"ENV_KIT_LIGHT_ERROR: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report["readability"]["lit"], indent=1))
    for f in report["files"]:
        print("  " + f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
