#!/usr/bin/env python3
"""AREA-FASTFOOD-01 offline evidence sheets and measurements (deterministic, from the committed grids and the real stills).

    python3 scripts/make_ff01_sheets.py OUT_DIR [LAYOUT_DIR]

Reads ``OUT_DIR/scene_*_640x360.png`` (the runtime captures of ``make_ff01_evidence.sh``) and writes into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every kit piece at native size on a neutral ground (the 4x sheet
  labelled; the 12 rising door crops are in the door strip);
- ``door_states_x4.png``: the 14 door frames at 4x (closed, rising 1..12, open); ``box_states_x4.png``: the box's three
  frames, the sparkle over empty, and the bell's three frames at 4x (key pixels, unlit);
- ``layout_diagram_play_x2.png`` / ``layout_diagram_shelf_x2.png``: the room schema over a real still (the walking line W,
  the floor back line FB, the three stand lines, the lock rect, the grant column, the presenter rects and every dressing
  rect). The drawn view left of each still is read from landmarks (``scripts/ff01_view.py``, needs LAYOUT_DIR) and
  written to ``drawn_views.txt``;
- ``before_after_door_closed_x1.png`` / ``_x3.png`` and ``before_after_shelf_376_x1.png`` / ``_x3.png``: the REST-WIRE
  greybox still of the same pilot and tick beside the dressed room (docs/evidence/rest-wire);
- ``measurements.txt``: gate note G7 (door against wall brightness), rule 10.1 on the runtime stills, the light count,
  the colour budgets and the key pins.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_ff01 as ff  # noqa: E402

BG = (58, 54, 66, 255)
W_LINE, FB = 580, 560
STAND = (("deck", 464, 559, 513), ("slide top", 560, 687, 497), ("shelf", 1296, 1391, 449))
LOCK = (1440, 352, 16, 228)
GRANT = (1200, 1240)
RW = ROOT / "docs/evidence/rest-wire"
UNLIT = {"ff_staff_door_open", "ff_reward_sparkle_a", "ff_reward_sparkle_b", "ff_reward_sparkle_c"}


def piece_images() -> dict[str, Image.Image]:
    out = {}
    for name in ff.ATLASES:
        for pid, img in ff.build_pieces(name):
            out[pid] = ff.to_image(img)
    return out


def contact(pieces, scale: int, labels: bool, width: int = 480) -> Image.Image:
    pad, label_h = 6, (12 if labels else 0)
    x = y = row_h = 0
    spots = []
    for pid, img in pieces:
        w, h = img.width, img.height
        if x + w + pad > width and x > 0:
            x, y, row_h = 0, y + row_h + pad + label_h, 0
        spots.append((pid, img, x + pad, y + pad + label_h))
        x += w + pad
        row_h = max(row_h, h)
    total_h = y + row_h + 2 * pad + label_h
    out = Image.new("RGBA", (max(width, x + pad) * scale, total_h * scale), BG)
    draw = ImageDraw.Draw(out)
    for pid, img, px, py in spots:
        out.alpha_composite(img.resize((img.width * scale, img.height * scale), Image.NEAREST),
                            (px * scale, py * scale))
        if labels:
            draw.text((px * scale, (py - label_h) * scale + 2), pid.removeprefix("ff_"), fill=(236, 226, 210, 255))
    return out


def strip(items, scale: int, pad: int = 6, labels: bool = True) -> Image.Image:
    """Frames side by side, bottom aligned, at ``scale``x with a label above each."""
    label_h = 12 if labels else 0
    w = sum(i.width * scale + pad for _, i in items) + pad
    h = max(i.height for _, i in items) * scale + 2 * pad + label_h
    out = Image.new("RGBA", (w, h), BG)
    d = ImageDraw.Draw(out)
    x = pad
    for name, im in items:
        out.alpha_composite(im.resize((im.width * scale, im.height * scale), Image.NEAREST),
                            (x, h - pad - im.height * scale))
        if labels:
            d.text((x, 2), name, fill=(236, 226, 210, 255))
        x += im.width * scale + pad
    return out


def luma(rgb) -> float:
    return 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]


def drawn_views(out_dir: Path, layout_dir: Path | None) -> dict[str, tuple[int, int]]:
    """The drawn view (left, top) of each 640 x 360 still, from landmarks (``scripts/ff01_view.py``: a 2D match
    against the lit model). Without LAYOUT_DIR the cached ``drawn_views.txt`` is read."""
    views: dict[str, tuple[int, int]] = {}
    cache = out_dir / "drawn_views.txt"
    if layout_dir is None:
        if cache.is_file():
            for line in cache.read_text(encoding="utf-8").splitlines():
                name, left, top = line.split()
                views[name] = (int(left), int(top))
        return views
    import ff01_view as fv  # noqa: PLC0415
    strip_img = fv.room_strip(layout_dir)
    lines = []
    for still in sorted(out_dir.glob("scene_*_640x360.png")):
        name = still.name.removeprefix("scene_").removesuffix("_640x360.png")
        left, top, _err = fv.locate(strip_img, still)
        views[name] = (left, top)
        lines.append(f"{name} {left} {top}")
    cache.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return views


def layout_diagram(still: Path, view: tuple, contract: dict, sizes: dict) -> Image.Image:
    left, top = view
    img = Image.open(still).convert("RGBA").resize((1280, 720), Image.NEAREST)
    d = ImageDraw.Draw(img)

    def sx(x):
        return (x - left) * 2

    def sy(y):
        return (y - top) * 2

    for y, label, col in ((W_LINE, "walking line W 580", (120, 255, 120)),
                          (FB, "floor back line FB 560", (120, 200, 255))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
        d.text((560, sy(y) - 12), label, fill=col + (255,))
    d.rectangle((0, sy(524), 1279, sy(560) - 1), outline=(200, 120, 255, 160))
    d.text((8, sy(560) - 11), "head and torso band 524..560 (wall art mid-value)", fill=(210, 150, 255, 255))
    for p in contract["room_dressing"]["rest_refuge"]["place"]:
        w, h = sizes[p["piece"]]
        x0, y0 = sx(p["x"]), sy(p["y"])
        if x0 > 1280 or x0 + w * 2 < 0 or y0 > 720 or y0 + h * 2 < 0:
            continue
        col = {"backwall_decal": (255, 90, 60, 255), "prop": (255, 210, 90, 255)}.get(p["layer"], (200, 160, 70, 255))
        d.rectangle((x0, y0, x0 + w * 2 - 1, y0 + h * 2 - 1), outline=col)
        if w * h >= 400 and p["piece"] not in ("ff_wall_tile_a", "ff_ceiling_tile", "ff_soffit_stone",
                                                "ff_wainscot_a", "ff_wainscot_b", "ff_floor_checker_a"):
            d.text((x0 + 3, y0 + 3), p["piece"].removeprefix("ff_"), fill=col)
    for name, x0, x1, y in STAND:                       # the three stand lines: flame 5 on tile row 1
        d.line((sx(x0), sy(y), sx(x1 + 1) - 1, sy(y)), fill=(255, 255, 255, 255), width=2)
        d.rectangle((sx(x0 - 64), sy(y - 64), sx(x1 + 65) - 1, sy(y + 65) - 1), outline=(255, 255, 255, 70))
        d.text((sx(x0) + 2, sy(y) - 12), f"stand line {name} y {y} (+/- 64 px envelope)", fill=(255, 255, 255, 255))
    x, y, w, h = LOCK
    d.rectangle((sx(x), sy(y), sx(x + w) - 1, sy(y + h) - 1), outline=(255, 60, 200, 255), width=2)
    d.text((sx(x) - 120, sy(y) + 4), "lock rect 1440..1455 (door: presenter)", fill=(255, 120, 220, 255))
    d.rectangle((sx(GRANT[0]), sy(300), sx(GRANT[1]) - 1, sy(580) - 1), outline=(120, 255, 200, 200))
    d.text((sx(GRANT[0]) + 3, sy(300) + 3), "grant column 1200..1240", fill=(120, 255, 200, 255))
    for nm, (px, py), (w, h) in (("door canvas", ff.DOOR_AT, (24, 228)), ("box", ff.BOX_AT, (32, 24)),
                                 ("sparkle", (1212, 554), (16, 16)), ("bell", (1380, 442), (8, 6))):
        d.rectangle((sx(px), sy(py), sx(px + w) - 1, sy(py + h) - 1), outline=(255, 160, 40, 255))
        if nm != "door canvas":
            d.text((sx(px), sy(py) - 11), nm, fill=(255, 190, 90, 255))
    return img


def pair(before: Image.Image, after: Image.Image, scale: int, text: tuple) -> Image.Image:
    b, a = before.convert("RGBA"), after.convert("RGBA")
    w, h = b.width, b.height
    out = Image.new("RGBA", (w * 2 + 8, h + 14), (16, 14, 20, 255))
    out.alpha_composite(b, (0, 14))
    out.alpha_composite(a, (w + 8, 14))
    dr = ImageDraw.Draw(out)
    dr.text((4, 2), text[0], fill=(236, 226, 210, 255))
    dr.text((w + 12, 2), text[1], fill=(236, 226, 210, 255))
    return out.resize((out.width * scale, out.height * scale), Image.NEAREST) if scale > 1 else out


# ---------------------------------------------------------------------------------------- measurements
def model_mask(still: np.ndarray, model: np.ndarray, thr: float = 24.0) -> np.ndarray:
    """Pixels the lit model does not explain (actors, HUD plates, notices, the presenter sprites): dilated by 2 px."""
    bad = np.abs(still - model) > thr
    out = bad.copy()
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            out |= np.roll(np.roll(bad, dy, 0), dx, 1)
    return out


def best_window(lum: np.ndarray, ok: np.ndarray, y: int, x0: int, x1: int, n: int = 8):
    """The brightest ``n`` px run on row ``y`` between x0..x1 (screen px): (mean luma, x) over unmasked pixels."""
    best = (-1.0, -1)
    if not (0 <= y < lum.shape[0]):
        return best
    for x in range(max(0, x0), min(lum.shape[1], x1) - n + 1):
        if ok[y, x:x + n].all():
            m = float(lum[y, x:x + n].mean())
            if m > best[0]:
                best = (m, x)
    return best


def measure(out_dir: Path, views: dict, layout_dir: Path | None) -> list[str]:
    lines = []
    contract = json.loads(ff.CONTRACT.read_text(encoding="utf-8"))
    # ---- gate note G7: the door's panel against the wall field beside it, in the runtime `door_closed` still
    still = out_dir / "scene_door_closed_640x360.png"
    if still.is_file() and "door_closed" in views:
        left, top = views["door_closed"]
        img = np.asarray(Image.open(still).convert("RGB"), dtype=np.float32)
        lum = 0.2126 * img[..., 0] + 0.7152 * img[..., 1] + 0.0722 * img[..., 2]

        def mean_of(x0, x1, y0, y1):
            return float(lum[y0 - top:y1 - top + 1, x0 - left:x1 - left + 1].mean())

        panel = mean_of(1441, 1454, 368, 527)
        wall = mean_of(1408, 1435, 368, 527)
        diff = wall - panel
        # the rail's outline column (x 1436, canvas column 0, stone_cold 0) against its neighbours on the same rows
        col = lum[368 - top:528 - top, 1436 - left]
        nb = np.minimum(lum[368 - top:528 - top, 1435 - left], lum[368 - top:528 - top, 1437 - left])
        dark_rows = int((col < nb).sum())
        lines.append(f"G7 door_closed (view left {left}, top {top}): panel interior x 1441..1454 y 368..527 mean luma "
                     f"{panel:.1f}; wall field x 1408..1435 same rows mean luma {wall:.1f}; difference {diff:.1f} "
                     f"(target >= 25: {'MET' if diff >= 25 else 'MISSED: a design note for the top tier, not tweaked'})")
        lines.append(f"G7 rail outline (stone_cold 0, x 1436) darker than both neighbours on {dark_rows} of "
                     f"{col.size} rows ({'unbroken' if dark_rows == col.size else 'broken on ' + str(col.size - dark_rows) + ' rows'})")
    # ---- rule 10.1 on the runtime stills (the lit model explains the room; actors, HUD and presenter art are masked)
    if layout_dir is not None:
        import ff01_view as fv  # noqa: PLC0415
        model_img = np.asarray(fv.room_image(layout_dir).convert("RGB"), dtype=np.float32)
        model = 0.2126 * model_img[..., 0] + 0.7152 * model_img[..., 1] + 0.0722 * model_img[..., 2]
        for name in ("play_area", "window", "box_full", "door_closed", "door_open", "door_rising_376"):
            f = out_dir / f"scene_{name}_640x360.png"
            if not f.is_file() or name not in views:
                continue
            left, top = views[name]
            img = np.asarray(Image.open(f).convert("RGB"), dtype=np.float32)
            lum = 0.2126 * img[..., 0] + 0.7152 * img[..., 1] + 0.0722 * img[..., 2]
            mod = model[top:top + 360, left:left + 640]
            ok = ~model_mask(lum[:, :], mod)
            excl = np.zeros(ok.shape, dtype=bool)        # the three stand-line pieces themselves are not rivals
            for _n, ex0, ex1, ey in STAND:
                excl[max(0, ey - top - 1):max(0, ey - top + 15), max(0, ex0 - left):max(0, ex1 + 1 - left)] = True
            ok &= ~excl
            for sname, x0, x1, y in STAND:
                sx0, sx1, sy_ = x0 - left, x1 - left, y - top
                if sx1 < 8 or sx0 > 640 - 8 or not (0 <= sy_ < 360):
                    continue
                stand, sxx = best_window(lum, ok | excl, sy_, sx0, sx1 + 1)
                if stand < 0:
                    lines.append(f"R10.1 {name}: the {sname} stand line is hidden (actors) in this still")
                    continue
                rival, where = (-1.0, None)
                # the envelope: the segment's rect grown by 64 px on every side (gate note G2)
                for yy in range(max(0, sy_ - 64), min(360, sy_ + 65)):
                    m, xw = best_window(lum, ok, yy, sx0 - 64, sx1 + 65)
                    if m > rival:
                        rival, where = m, (xw + left, yy + top)
                lines.append(f"R10.1 {name}: {sname} stand line (y {y}) best 8 px run luma {stand:.0f} at x {sxx + left}; "
                             f"brightest unmasked rival run within 64 px luma {rival:.0f} at {where}; "
                             f"stand line is brighter by {stand - rival:.0f} ({'OK' if stand > rival else 'RIVAL BRIGHTER: design note'})")
        # ---- light count: the model's emitters in the worst 640 px view
        import env_kit as ek  # noqa: PLC0415
        import env_kit_light as lit  # noqa: PLC0415
        kit = ek.load_kit("assets/source/terrain/floor01/floor01_fastfood_kit.json")
        layout = json.loads((layout_dir / "rest_refuge.dressed.layout.json").read_text(encoding="utf-8"))
        plain = json.loads((layout_dir / "rest_refuge.plain.layout.json").read_text(encoding="utf-8"))
        counts = [len(lit.gather_lights(kit, layout, [left, 292, 640, 360])) for left in range(0, 1600 - 640 + 1, 8)]
        counts_p = [len(lit.gather_lights(kit, plain, [left, 292, 640, 360])) for left in range(0, 1600 - 640 + 1, 8)]
        lines.append(f"LIGHTS worst 640 px view (model, step 8 px): dressed {max(counts)} (at left {8 * counts.index(max(counts))}), "
                     f"plain greybox {max(counts_p)}; MAX_LIGHTS 64; the Rust test fastfood_kit::tests::"
                     f"the_refuge_lights_stay_inside_the_light_budget proves the kit's own count and the net minus 8 plus 6")
    # ---- colour budgets and the pins
    for name, spec in ff.ATLASES.items():
        grid = json.loads((ff.SRC / f"{spec['stem']}.grid.json").read_text(encoding="utf-8"))
        lines.append(f"ATLAS {name} canvas {grid['canvas']} colours {len(grid['palette'])} (max 63)")
    for pid, sha in ff.key_sha().items():
        lines.append(f"KEY {pid} {sha}")
    return lines


def main(argv: list[str]) -> int:
    out_dir = Path(argv[0])
    layout_dir = Path(argv[1]) if len(argv) > 1 and argv[1] else None
    out_dir.mkdir(parents=True, exist_ok=True)
    imgs = piece_images()
    flat = [(p, i) for p, i in imgs.items() if "_rise_" not in p]
    contact(flat, 1, False, width=560).save(out_dir / "contact_sheet_x1.png")
    contact(flat, 4, True, width=560).save(out_dir / "contact_sheet_x4.png")
    door = [("closed", imgs["ff_staff_door_closed"])] + [(f"rise {k}", imgs[f"ff_staff_door_rise_{k:02d}"])
                                                          for k in range(1, 13)] + [("open", imgs["ff_staff_door_open"])]
    strip(door, 4).save(out_dir / "door_states_x4.png")
    box = [("full_a", imgs["ff_reward_box_full_a"]), ("full_b", imgs["ff_reward_box_full_b"]),
           ("empty", imgs["ff_reward_box_empty"])]
    for s in "abc":
        canvas = Image.new("RGBA", (32, 26), (0, 0, 0, 0))
        canvas.alpha_composite(imgs["ff_reward_box_empty"], (0, 2))
        canvas.alpha_composite(imgs[f"ff_reward_sparkle_{s}"], (8, 0))
        box.append((f"empty+spark {s}", canvas))
    box += [("bell idle", imgs["ff_bell_idle"]), ("ding_a", imgs["ff_bell_ding_a"]), ("ding_b", imgs["ff_bell_ding_b"])]
    strip(box, 4).save(out_dir / "box_states_x4.png")

    contract = json.loads(ff.CONTRACT.read_text(encoding="utf-8"))
    sizes = {p["id"]: tuple(p["rect"][2:]) for p in contract["pieces"]}
    views = drawn_views(out_dir, layout_dir)
    for name, label in (("play_area", "play"), ("door_open", "shelf")):
        still = out_dir / f"scene_{name}_640x360.png"
        if still.is_file() and name in views:
            layout_diagram(still, views[name], contract, sizes).save(out_dir / f"layout_diagram_{label}_x2.png")
    for ours, theirs, label in (("door_closed", "still_swap_hint", "door_closed"),
                                ("door_rising_376", "still_shelf_door_open", "shelf_376")):
        text = ("before: REST-WIRE greybox (same pilot, tick)", "after: AREA-FASTFOOD-01")
        for res, tag, scale in (("640x360", "x1", 1), ("1920x1080", "x3", 1)):
            b, a = RW / f"{theirs}_{res}.png", out_dir / f"scene_{ours}_{res}.png"
            if b.is_file() and a.is_file():
                pair(Image.open(b), Image.open(a), scale, text).save(out_dir / f"before_after_{label}_{tag}.png")
    lines = measure(out_dir, views, layout_dir)
    (out_dir / "measurements.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"sheets in {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
