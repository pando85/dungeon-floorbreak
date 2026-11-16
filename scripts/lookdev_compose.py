#!/usr/bin/env python3
"""Look-dev compositor for the Floor 1 resolution spike (Pillow only, deterministic).

LOOK-DEV ONLY: renders review frames, not runtime code. It reads the pixel-grid
layer sources and ``scene.json`` written by ``scripts/lookdev_floor01_author.py``,
the approved character grids, and ``lighting.json`` (light list, ambient, haze,
bloom, parallax demo), then writes:

* frame ``a0`` -- 640x360 pixel layers, lighting baked into the pixels only;
* frame ``a1`` -- the same 640x360 layers plus a lighting pass computed at the
  *output* resolution (cold ambient multiply, warm torch pools with smooth
  falloff, additive glow, bloom on flames, distance haze on gallery layers);
* frame ``b1`` -- the 960x540 redraw with the same lighting pass.

Pixel layers are always enlarged with nearest-neighbour by an integer factor;
only the lighting buffers are smooth. See
``docs/02-art/lookdev-floor01-resolution-spike.md`` for how each step maps to a
proposed Bevy 0.20 technique (proposal only, not verified in engine).

Usage::

    python3 scripts/lookdev_compose.py all --out generated/lookdev
    python3 scripts/lookdev_compose.py frame a1 --scale 6 --out /tmp/a1.png
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont, ImageMath

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "assets" / "source" / "lookdev" / "floor01"
TRANSPARENT = "."


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------

def hex_rgb(text: str) -> tuple[int, int, int]:
    text = text.lstrip("#")
    return tuple(int(text[i:i + 2], 16) for i in (0, 2, 4))


def grid_image(path: Path, upscale: float = 1.0) -> tuple[Image.Image, tuple[int, int]]:
    """Flatten frame 0 of a pixel-grid-v1 file (all layers, bottom to top)."""
    grid = json.loads(path.read_text(encoding="utf-8"))
    pal = {e["key"]: hex_rgb(e["rgba"][:7]) + (255,) for e in grid["palette"]}
    w, h = grid["canvas"]
    data = [(0, 0, 0, 0)] * (w * h)
    cels = grid["frames"][0]["cels"]
    for layer in grid["layers"]:
        rows = cels.get(layer)
        if not rows:
            continue
        for y, row in enumerate(rows):
            base = y * w
            for x, ch in enumerate(row):
                if ch != TRANSPARENT:
                    data[base + x] = pal[ch]
    img = Image.new("RGBA", (w, h))
    img.putdata(data)
    pivot = tuple(grid["pivot"])
    if upscale != 1.0:
        img = img.resize((round(w * upscale), round(h * upscale)), Image.Resampling.NEAREST)
        pivot = (math.floor(pivot[0] * upscale), round((pivot[1] + 1) * upscale) - 1)
    return img, pivot


def load_config() -> tuple[dict, dict]:
    scene = json.loads((SRC / "scene.json").read_text(encoding="utf-8"))
    lighting = json.loads((SRC / "lighting.json").read_text(encoding="utf-8"))
    return scene, lighting


# ---------------------------------------------------------------------------
# Frame assembly at native (virtual) resolution
# ---------------------------------------------------------------------------

class Frame:
    """Layer stack of one frame at native density, cropped to a view."""

    def __init__(self, scene: dict, lighting: dict, name: str, view: str | None = None,
                 actors: str = "auto"):
        spec = lighting["frames"][name]
        self.name = name
        self.lit = spec["lighting"]
        self.density_key = spec["density"]
        dens = scene["densities"][self.density_key]
        self.s = dens["density"]
        self.origin = dens["design_origin"]
        self.view_name = view or spec["view"]
        vx, vy, vw, vh = dens["views"][self.view_name]
        self.view = (vx, vy, vw, vh)
        self.size = (vw, vh)
        self.layers: list[dict] = []
        actor_set = spec["actors"] if actors == "auto" else actors
        for i, entry in enumerate(dens["layers"]):
            if i == dens["actor_insert_index"] and actor_set != "none":
                self.layers.append(self._actor_layer(lighting, actor_set))
            img, _ = grid_image(SRC / entry["grid"])
            full = Image.new("RGBA", tuple(dens["canvas"]), (0, 0, 0, 0))
            full.alpha_composite(img, tuple(entry["offset"]))
            self.layers.append({"name": entry["name"], "image": full.crop((vx, vy, vx + vw, vy + vh)),
                                "group": entry["light_group"], "parallax": entry["parallax"]})
        self.actor_set = actor_set

    def _actor_layer(self, lighting: dict, actor_set: str) -> dict:
        dens = lighting["actors"][actor_set]
        canvas = Image.new("RGBA", self.size, (0, 0, 0, 0))
        walk = lighting["walk_line_design_y"]
        contact_row = round((walk - self.origin[1]) * self.s) - 1 - self.view[1]
        self.actor_boxes = []
        for a in dens["sprites"]:
            path = ROOT / a["grid"]
            img, (px, py) = grid_image(path, a.get("upscale", 1.0))
            x = round(a["x_design"] * self.s) - self.view[0]
            pos = (x - px, contact_row - py)
            canvas.alpha_composite(img, pos)
            self.actor_boxes.append((pos[0], pos[1], pos[0] + img.width, pos[1] + img.height))
        return {"name": "actors", "image": canvas, "group": "actors", "parallax": 1.0,
                "label": dens.get("label")}

    def layer(self, name: str) -> Image.Image:
        return next(layer["image"] for layer in self.layers if layer["name"] == name)


# ---------------------------------------------------------------------------
# Lighting pass at output resolution
# ---------------------------------------------------------------------------

_SPRITE_CACHE: dict[str, Image.Image] = {}


def falloff_sprite(kind: str) -> Image.Image:
    """512x512 'F' image of a radial falloff, 1.0 at the centre, 0.0 at the rim."""
    if kind in _SPRITE_CACHE:
        return _SPRITE_CACHE[kind]
    n = 512
    data = []
    for y in range(n):
        dy = (y + 0.5) / n * 2 - 1
        for x in range(n):
            dx = (x + 0.5) / n * 2 - 1
            t2 = dx * dx + dy * dy
            if t2 >= 1.0:
                data.append(0.0)
            elif kind == "pool":
                data.append((1.0 - t2) ** 2 / (1.0 + 5.0 * t2))
            else:  # "glow": tighter, for additive halos
                data.append((1.0 - t2) ** 3 / (1.0 + 30.0 * t2))
    img = Image.new("F", (n, n))
    img.putdata(data)
    _SPRITE_CACHE[kind] = img
    return img


def _add_scaled(buf: Image.Image, sprite: Image.Image, left: int, top: int, k: float) -> None:
    bw, bh = buf.size
    x0, y0 = max(0, left), max(0, top)
    x1, y1 = min(bw, left + sprite.width), min(bh, top + sprite.height)
    if x0 >= x1 or y0 >= y1 or k == 0.0:
        return
    region = buf.crop((x0, y0, x1, y1))
    part = sprite.crop((x0 - left, y0 - top, x1 - left, y1 - top))
    out = ImageMath.lambda_eval(lambda a: a["r"] + a["p"] * k, r=region, p=part)
    buf.paste(out, (x0, y0))


def light_buffers(size, frame: Frame, lighting: dict, group: str, out_scale: int,
                  shift_x: float = 0.0, kind: str = "pool", key: str = "lights",
                  base: tuple[float, float, float] | None = None) -> list[Image.Image]:
    """Per-channel 'F' buffers: base value + sum of light falloffs (output pixels)."""
    gcfg = lighting["groups"][group]
    if base is None:
        base = tuple(gcfg["ambient"])
    bufs = [Image.new("F", size, float(base[i])) for i in range(3)]
    k = frame.s * out_scale  # design unit -> output pixels
    vx, vy = frame.view[0] / frame.s, frame.view[1] / frame.s  # view origin, design units
    ox, oy = frame.origin
    scale = gcfg.get("light_scale", 1.0)
    for name in gcfg.get(key, []):
        light = lighting["lights"][name]
        mult = light["intensity"] if kind == "pool" else light.get("additive", 0.0)
        if mult == 0.0:
            continue
        radius = light["radius"] if kind == "pool" else light.get("glow_radius", light["radius"] * 0.35)
        diameter = max(2, round(2 * radius * k))
        height = max(2, round(diameter * light.get("squash_y", 1.0)))  # floor pools are flat ellipses
        sprite = falloff_sprite(kind).resize((diameter, height), Image.Resampling.BICUBIC)
        lx = (light["x"] + shift_x - ox - vx) * k
        ly = (light["y"] - oy - vy) * k
        left, top = round(lx - diameter / 2), round(ly - height / 2)
        rgb = hex_rgb(light["colour"])
        for i in range(3):
            _add_scaled(bufs[i], sprite, left, top, scale * mult * rgb[i] / 255.0)
    return bufs


def haze_buffer(size, frame: Frame, haze: dict, out_scale: int) -> Image.Image | None:
    """'F' buffer of the haze mix factor (constant plus an optional ground-fog ramp)."""
    amount = haze.get("amount", 0.0)
    fog = haze.get("floor_fog")
    if not fog:
        return Image.new("F", size, float(amount)) if amount else None
    k = frame.s * out_scale
    vy = frame.view[1] / frame.s
    oy = frame.origin[1]
    col = []
    for y in range(size[1]):
        dy = (y + 0.5) / k + vy + oy  # design y
        t = (dy - fog["from_y"]) / (fog["to_y"] - fog["from_y"])
        t = 0.0 if t < 0 else 1.0 if t > 1 else t * t * (3 - 2 * t)
        col.append(amount + fog["amount"] * t)
    strip = Image.new("F", (1, size[1]))
    strip.putdata(col)
    return strip.resize(size, Image.Resampling.NEAREST)


def shade_layer(img: Image.Image, frame: Frame, lighting: dict, group: str, out_scale: int,
                shift_x: float = 0.0) -> Image.Image:
    """Multiply by the group light map, then mix toward the haze colour."""
    gcfg = lighting["groups"][group]
    r, g, b, a = img.split()
    light = light_buffers(img.size, frame, lighting, group, out_scale, shift_x)
    chans = []
    for ch, lbuf in zip((r, g, b), light):
        chans.append(ImageMath.lambda_eval(lambda v: v["c"] * v["l"], c=ch.convert("F"), l=lbuf))
    haze = gcfg.get("haze")
    if haze:
        hbuf = haze_buffer(img.size, frame, haze, out_scale)
        if hbuf is not None:
            hc = hex_rgb(haze["colour"])
            chans = [ImageMath.lambda_eval(lambda v: v["c"] * (1.0 - v["h"]) + v["h"] * hv,
                                           c=cf, h=hbuf) for cf, hv in zip(chans, hc)]
    out = Image.merge("RGB", [cf.convert("L") for cf in chans]).convert("RGBA")
    out.putalpha(a)
    return out


def bloom(img: Image.Image, emissive: Image.Image, cfg: dict, out_scale_px: float) -> Image.Image:
    """Bright-pass of the emissive layers only, blur at reduced size, add back (deterministic).

    Restricting the source to emissive layers (flames, distant torches) keeps bright
    actor colours such as Donut's white fur from blooming.
    """
    thr, gain = cfg["threshold"], cfg["intensity"]
    down = max(1, round(out_scale_px * cfg["downsample_per_virtual_px"]))
    rgb = img.convert("RGB")
    src = emissive.convert("RGB")
    small = src.resize((max(1, src.width // down), max(1, src.height // down)), Image.Resampling.BOX)
    bands = []
    for ch in small.split():
        bands.append(ch.point(lambda v: 0 if v < thr else min(255, int((v - thr) * gain * 255 / (255 - thr)))))
    bright = Image.merge("RGB", bands)
    radius = cfg["radius_virtual"] * out_scale_px / down
    blurred = bright.filter(ImageFilter.GaussianBlur(radius))
    glow = blurred.resize(rgb.size, Image.Resampling.BILINEAR)
    return ImageChops.add(rgb, glow).convert("RGBA")


def render(frame: Frame, lighting: dict, out_scale: int, cam_dx: float = 0.0) -> Image.Image:
    """Composite one frame at ``out_scale`` x its native size."""
    w, h = frame.size
    size = (w * out_scale, h * out_scale)
    canvas = Image.new("RGBA", size, (0, 0, 0, 255))
    emissive = Image.new("RGBA", size, (0, 0, 0, 255))
    emissive_layers = set(lighting["bloom"]["emissive_layers"])
    group_shift: dict[str, float] = {}
    for layer in frame.layers:
        shift = (1.0 - layer["parallax"]) * cam_dx if layer["parallax"] < 1.0 else 0.0
        group_shift.setdefault(layer["group"], shift)
        img = layer["image"]
        if shift:
            moved = Image.new("RGBA", img.size, (0, 0, 0, 0))
            moved.alpha_composite(img, (round(shift * frame.s), 0))
            img = moved
        img = img.resize(size, Image.Resampling.NEAREST)
        if frame.lit:
            img = shade_layer(img, frame, lighting, layer["group"], out_scale, shift)
        canvas.alpha_composite(img)
        if frame.lit and layer["name"] in emissive_layers:
            emissive.alpha_composite(img)
    if frame.lit:
        rgb = canvas.convert("RGB")
        for group, shift in group_shift.items():  # additive halos, per depth group
            if not lighting["groups"][group].get("glow_lights"):
                continue
            glow = light_buffers(size, frame, lighting, group, out_scale, shift_x=shift, kind="glow",
                                 key="glow_lights", base=(0.0, 0.0, 0.0))
            add = Image.merge("RGB", [ImageMath.lambda_eval(lambda v: v["g"] * 255.0, g=g).convert("L")
                                      for g in glow])
            rgb = ImageChops.add(rgb, add)
        canvas = bloom(rgb.convert("RGBA"), emissive, lighting["bloom"], out_scale * frame.s)
    return canvas.convert("RGB")


# ---------------------------------------------------------------------------
# Labels, sheets and measurements
# ---------------------------------------------------------------------------

def font(size: int) -> ImageFont.ImageFont:
    return ImageFont.load_default(size=size)


def banner(img: Image.Image, text: str, colour=(255, 214, 64)) -> Image.Image:
    out = img.copy()
    size = max(10, img.height // 36)
    draw = ImageDraw.Draw(out)
    pad = size // 2
    box = draw.textbbox((0, 0), text, font=font(size))
    draw.rectangle((0, 0, box[2] + 2 * pad, box[3] + 2 * pad), fill=(20, 10, 10))
    draw.text((pad, pad), text, fill=colour, font=font(size))
    return out


def letterbox(img: Image.Image, size: tuple[int, int]) -> Image.Image:
    out = Image.new("RGB", size, (0, 0, 0))
    out.paste(img, ((size[0] - img.width) // 2, (size[1] - img.height) // 2))
    return out


def sharp_bilinear(img: Image.Image, target: tuple[int, int]) -> Image.Image:
    """Nearest prescale to the next integer factor, then bilinear down to the target."""
    k = math.ceil(max(target[0] / img.width, target[1] / img.height))
    big = img.resize((img.width * k, img.height * k), Image.Resampling.NEAREST)
    return big.resize(target, Image.Resampling.BILINEAR)


def srgb_to_lab_l(rgb) -> float:
    def lin(c):
        c /= 255.0
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    y = 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])
    return 116 * (y ** (1 / 3) if y > 216 / 24389 else (24389 / 27 * y + 16) / 116) - 16, y


def readability(img: Image.Image, mask: Image.Image, boxes, ring: int) -> list[dict]:
    """Mean L* of actor pixels vs a surrounding ring, plus silhouette edge contrast."""
    px = img.convert("RGB").load()
    mk = mask.load()
    w, h = img.size
    results = []
    for (x0, y0, x1, y1) in boxes:
        a_l, a_y, b_l, b_y, edges = [], [], [], [], []
        for y in range(max(0, y0 - ring), min(h, y1 + ring)):
            for x in range(max(0, x0 - ring), min(w, x1 + ring)):
                L, Y = srgb_to_lab_l(px[x, y])
                inside = x0 <= x < x1 and y0 <= y < y1 and mk[x, y] > 0
                if inside:
                    a_l.append(L)
                    a_y.append(Y)
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < w and 0 <= ny < h and mk[nx, ny] == 0:
                            edges.append(abs(L - srgb_to_lab_l(px[nx, ny])[0]))
                elif mk[x, y] == 0:
                    b_l.append(L)
                    b_y.append(Y)
        ya, yb = sum(a_y) / len(a_y), sum(b_y) / len(b_y)
        hi, lo = max(ya, yb), min(ya, yb)
        results.append({
            "actor_mean_L": round(sum(a_l) / len(a_l), 1),
            "surround_mean_L": round(sum(b_l) / len(b_l), 1),
            "delta_L": round(sum(a_l) / len(a_l) - sum(b_l) / len(b_l), 1),
            "luminance_contrast_ratio": round((hi + 0.05) / (lo + 0.05), 2),
            "silhouette_edge_mean_abs_dL": round(sum(edges) / len(edges), 1),
            "edge_samples": len(edges),
        })
    return results


def colour_count(img: Image.Image) -> int:
    return len(img.convert("RGB").getcolors(maxcolors=1 << 24))


# ---------------------------------------------------------------------------
# Full evidence set
# ---------------------------------------------------------------------------

def save(img: Image.Image, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, format="PNG", optimize=False)
    return path


def render_all(out: Path) -> dict:
    scene, lighting = load_config()
    b_actor_set = lighting["frames"]["b1"]["actors"]
    real_b = (lighting["actors"][b_actor_set].get("status") == "real")
    report: dict = {"b1_actor_set": b_actor_set, "b1_actors_real": real_b,
                    "frames": {}, "files": []}
    written = report["files"]

    def frame(name, view=None, actors="auto"):
        return Frame(scene, lighting, name, view, actors)

    # ---- A0 / A1 (640x360) and their 16:10 Steam Deck variants (640x400)
    for name in ("a0", "a1"):
        f = frame(name)
        native = render(f, lighting, 1)
        written.append(save(native, out / "native" / f"{name}_640x360.png"))
        written.append(save(render(f, lighting, 6), out / "4k" / f"{name}_3840x2160_x6.png"))
        written.append(save(render(f, lighting, 3), out / "1080p" / f"{name}_1920x1080_x3.png"))
        deck = frame(name, view="640x400")
        written.append(save(render(deck, lighting, 1), out / "native" / f"{name}_640x400.png"))
        written.append(save(render(deck, lighting, 2), out / "deck" / f"{name}_1280x800_640x400_x2.png"))
        report["frames"][name] = measure(f, native)
    # ---- B1 (960x540)
    variants = [("b1", "auto", None)]
    if not real_b:
        variants = [("b1_no_actors", "none", None),
                    ("b1_placeholder", "auto",
                     "PLACEHOLDER - approved h52/h40 sprites nearest-upscaled 1.5x - not representative")]
    for stem, actors, label in variants:
        f = frame("b1", actors=actors)

        def lab(img, label=label):
            return banner(img, label) if label else img
        native = render(f, lighting, 1)
        written.append(save(lab(native), out / "native" / f"{stem}_960x540.png"))
        x4 = render(f, lighting, 4)
        written.append(save(lab(x4), out / "4k" / f"{stem}_3840x2160_x4.png"))
        x2 = render(f, lighting, 2)
        written.append(save(lab(x2), out / "1080p" / f"{stem}_1920x1080_x2.png"))
        deck_sb = letterbox(sharp_bilinear(native, (1280, 720)), (1280, 800))
        written.append(save(lab(banner(deck_sb, "960x540 at 1.33x, sharp-bilinear (non-integer), letterboxed",
                                       (200, 220, 255))),
                            out / "deck" / f"{stem}_1280x800_sharp_bilinear_1.33x.png"))
        deck_1x = letterbox(native, (1280, 800))
        written.append(save(lab(banner(deck_1x, "960x540 at 1x, letterboxed (integer)", (200, 220, 255))),
                            out / "deck" / f"{stem}_1280x800_1x_letterbox.png"))
        if actors != "none":
            report["frames"][stem] = measure(f, native)
        else:
            report["frames"][stem] = {"colours_native": colour_count(native)}
    # ---- separate native pixel layers (view-cropped, transparent), bottom to top
    for name, actors in (("a0", "auto"), ("b1", "none" if not real_b else "auto")):
        f = frame(name, actors=actors)
        for i, layer in enumerate(f.layers):
            path = out / "layers" / f"{f.density_key}_{i}_{layer['name']}.png"
            written.append(save(layer["image"], path))
    # ---- 4K 1:1 detail crop comparison around Carl
    written.append(save(crop_sheet(out, lighting, real_b), out / "crop_compare_4k_800px.png"))
    written.append(save(preview_sheet(out, real_b), out / "preview_triptych.png"))
    written.append(save_gif(parallax_demo(scene, lighting), out / "a1_parallax_demo.gif"))
    report["pixel_layers"] = layer_stats(scene, lighting)
    report["files"] = [str(p.relative_to(out)) for p in written]
    (out / "measurements.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def measure(f: Frame, native: Image.Image) -> dict:
    actors = next(layer for layer in f.layers if layer["name"] == "actors")
    mask = actors["image"].split()[3]
    ring = round(6 * f.s)
    return {
        "colours_native": colour_count(native),
        "readability": dict(zip(("carl", "donut"), readability(native, mask, f.actor_boxes, ring))),
    }


def crop_sheet(out: Path, lighting: dict, real_b: bool) -> Image.Image:
    box = tuple(lighting["crop_compare"]["box_4k"])
    b_stem = "b1" if real_b else "b1_placeholder"
    names = [("A0  640x360 baked  (x6)", out / "4k" / "a0_3840x2160_x6.png"),
             ("A1  640x360 + lighting pass  (x6)", out / "4k" / "a1_3840x2160_x6.png"),
             ("B1  960x540 redraw + lighting  (x4)" + ("" if real_b else "  PLACEHOLDER ACTORS"),
              out / "4k" / f"{b_stem}_3840x2160_x4.png")]
    cw, ch = box[2] - box[0], box[3] - box[1]
    gap, band = 24, 56
    sheet = Image.new("RGB", (3 * cw + 2 * gap, ch + band), (14, 12, 18))
    draw = ImageDraw.Draw(sheet)
    for i, (label, path) in enumerate(names):
        with Image.open(path) as im:
            crop = im.convert("RGB").crop(box)
        sheet.paste(crop, (i * (cw + gap), band))
        draw.text((i * (cw + gap) + 8, 14), label, fill=(240, 230, 210), font=font(26))
    return sheet


def preview_sheet(out: Path, real_b: bool) -> Image.Image:
    b_stem = "b1" if real_b else "b1_placeholder"
    paths = [out / "1080p" / "a0_1920x1080_x3.png", out / "1080p" / "a1_1920x1080_x3.png",
             out / "1080p" / f"{b_stem}_1920x1080_x2.png"]
    tiles = []
    for p in paths:
        with Image.open(p) as im:
            tiles.append(im.convert("RGB").resize((640, 360), Image.Resampling.BOX))
    sheet = Image.new("RGB", (640 * 3 + 16, 360 + 30), (14, 12, 18))
    draw = ImageDraw.Draw(sheet)
    for i, (t, label) in enumerate(zip(tiles, ("A0 baked", "A1 + lighting pass",
                                               "B1 960x540" + ("" if real_b else " (placeholder actors)")))):
        sheet.paste(t, (i * 648, 30))
        draw.text((i * 648 + 6, 6), label + "  (display-downscaled preview)", fill=(240, 230, 210),
                  font=font(15))
    return sheet


def parallax_demo(scene: dict, lighting: dict) -> list[Image.Image]:
    cfg = lighting["parallax_demo"]
    f = Frame(scene, lighting, "a1")
    frames = []
    steps = cfg["steps"]
    for i in range(steps):
        dx = cfg["amplitude_design_px"] * math.sin(2 * math.pi * i / steps)
        img = render(f, lighting, cfg["scale"], cam_dx=dx)
        frames.append(img.crop(tuple(v * cfg["scale"] for v in cfg["crop_view_px"])))
    return frames


def save_gif(frames: list[Image.Image], path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    pal = frames[0].quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    q = [fr.quantize(palette=pal, dither=Image.Dither.NONE) for fr in frames]
    q[0].save(path, save_all=True, append_images=q[1:], duration=90, loop=0, optimize=False)
    return path


def layer_stats(scene: dict, lighting: dict) -> dict:
    stats = {}
    for dkey, dens in scene["densities"].items():
        layers = {}
        for entry in dens["layers"]:
            img, _ = grid_image(SRC / entry["grid"])
            opaque = img.width * img.height - img.split()[3].histogram()[0]
            layers[entry["name"]] = {"opaque_px": opaque, "colours": entry["colours"],
                                     "bbox": [img.width, img.height]}
        stats[dkey] = {"layers": layers,
                       "total_opaque_px": sum(v["opaque_px"] for v in layers.values()),
                       "palette_union": len(palette_union(dens))}
    for key, aset in lighting["actors"].items():
        sprites = {}
        for a in aset["sprites"]:
            img, _ = grid_image(ROOT / a["grid"], a.get("upscale", 1.0))
            alpha = img.split()[3]
            bbox = alpha.getbbox()
            sprites[Path(a["grid"]).stem] = {"opaque_px": alpha.width * alpha.height - alpha.histogram()[0],
                                             "opaque_height": bbox[3] - bbox[1] if bbox else 0}
        stats[f"actors_{key}"] = sprites
    return stats


def palette_union(dens: dict) -> set:
    colours = set()
    for entry in dens["layers"]:
        grid = json.loads((SRC / entry["grid"]).read_text(encoding="utf-8"))
        colours |= {e["rgba"][:7].lower() for e in grid["palette"]}
    return colours


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("all", help="render every evidence file into --out")
    p.add_argument("--out", type=Path, default=ROOT / "generated" / "lookdev")
    p = sub.add_parser("frame", help="render one frame")
    p.add_argument("name")
    p.add_argument("--scale", type=int, default=1)
    p.add_argument("--view")
    p.add_argument("--actors", default="auto")
    p.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.cmd == "all":
        report = render_all(args.out)
        print(json.dumps({k: v for k, v in report.items() if k != "files"}, indent=1))
        print(f"wrote {len(report['files'])} files to {args.out}")
    else:
        scene, lighting = load_config()
        f = Frame(scene, lighting, args.name, args.view, args.actors)
        save(render(f, lighting, args.scale), args.out)
        print(args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
