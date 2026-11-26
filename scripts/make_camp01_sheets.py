#!/usr/bin/env python3
"""AREA-CAMP-01 offline evidence sheets and measurements (deterministic, from the committed grids and the real stills).

    python3 scripts/make_camp01_sheets.py OUT_DIR [LAYOUT_DIR]

Reads ``OUT_DIR/scene_*_640x360.png`` and ``OUT_DIR/capture_lines.txt`` (the runtime captures of
``make_camp01_evidence.sh``) and writes into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every new piece at native size on a neutral ground (the keys
  and the executor pieces; the 4x sheet is labelled);
- ``fire_loop_x4.png``: the four ``camp_fire`` frames; ``bag_frames_x4.png``: the five bag frames;
- ``bag_sway_window_x4.png``: the bag cropped from the six ``scene_aft_punch_*`` window stills (rest, then the hit sway);
- ``layout_diagram_*_x2.png``: the room schema over a real still (placements, the sentry / figure / light origins, the
  openings, the walking line). The drawn view left of each still is read from landmarks (needs LAYOUT_DIR: the kit's lit
  model slid over the still's wall rows) and written to ``drawn_views.txt``;
- ``figures_lit.txt``: gate note G2, the four figures measured on the lit window stills;
- ``measurements.txt``: sizes, colours, luma, the key pins and the sway table.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_camp01 as camp  # noqa: E402

BG = (58, 54, 66, 255)
ROOMS = {"rest_camp": ("camp", camp.CAMP_CONTRACT, 1536), "rest_aftermath": ("aft", camp.AFT_CONTRACT, 1280)}
ROOM_H, ROWS = 672, (64, 190)
TOPS = range(280, 306)
OPENINGS = {"rest_camp": [(184, 335), (520, 671), (856, 1007), (1192, 1343)],
            "rest_aftermath": [(216, 367), (888, 1039)]}
# the designer's landmark values (torch_wall flame centre = placement x + 7.5), both rooms: tick -> view left
DESIGNER_VIEW = {41: 0, 161: 237, 201: 360, 241: 483, 281: 607}
FIGURE_AT = {"camp_figure_sentry": (500, 512), "camp_figure_repair": (872, 524), "camp_figure_seated": (1024, 530),
             "camp_figure_map": (1152, 514)}


def luma(a: np.ndarray) -> np.ndarray:
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def stills_of(out_dir: Path) -> dict[str, dict]:
    """name -> the CAMP_STILL fields of the 640x360 line."""
    out = {}
    for line in (out_dir / "capture_lines.txt").read_text(encoding="utf-8").splitlines():
        if not line.startswith("CAMP_STILL ") or " size=640x360 " not in line:
            continue
        f = dict(kv.split("=", 1) for kv in line.split()[1:])
        out[f["name"]] = f
    return out


# ------------------------------------------------------------------------------------------- the drawn view
def room_image(room: str, layout_dir: Path):
    import env_kit as ek  # noqa: PLC0415
    import env_kit_light as lit  # noqa: PLC0415
    _, contract, width = ROOMS[room]
    kit = ek.load_kit(contract)
    layout = json.loads((layout_dir / f"{room}.dressed.layout.json").read_text(encoding="utf-8"))
    return lit.render_lit(kit, layout, [0, 0, width, ROOM_H], 1)


def _err(win: np.ndarray, s: np.ndarray) -> float:
    diff = np.sort(np.abs(win - s), axis=None)
    return float(diff[: int(diff.size * 0.7)].mean())        # trimmed: actors, HUD text and the bag drop out


def locate(strip: np.ndarray, still: Path) -> tuple[int, int, float]:
    """(left, top, error): a coarse 2D search on a 4x downsample, then a full-resolution refinement."""
    width = strip.shape[1]
    s = np.asarray(Image.open(still).convert("L"), dtype=np.float32)[ROWS[0]:ROWS[1]]
    s4 = s[::4, ::4]
    best = (1e30, 0, 0)
    for top in TOPS:
        for left in range(0, width - 640 + 1, 4):
            win = strip[top + ROWS[0]:top + ROWS[1], left:left + 640][::4, ::4]
            err = _err(win, s4)
            if err < best[0]:
                best = (err, left, top)
    _, l0, t0 = best
    best = (1e30, 0, 0)
    for top in range(t0 - 4, t0 + 5):
        for left in range(max(0, l0 - 6), min(width - 640, l0 + 6) + 1):
            err = _err(strip[top + ROWS[0]:top + ROWS[1], left:left + 640], s)
            if err < best[0]:
                best = (err, left, top)
    return best[1], best[2], best[0]


def drawn_views(out_dir: Path, layout_dir: Path | None, stills: dict) -> tuple[dict, dict]:
    """{still name: (left, top)} and {room: lit model luma array}. Without LAYOUT_DIR the cache is read."""
    views, models = {}, {}
    cache = out_dir / "drawn_views.txt"
    if layout_dir is None:
        if cache.is_file():
            for line in cache.read_text(encoding="utf-8").splitlines():
                if line and not line.startswith("#"):
                    name, left, top = line.split()[:3]
                    views[name] = (int(left), int(top))
        return views, models
    lines = ["# still_name view_left view_top (landmarks: the kit's lit model slid over the still's wall rows)"]
    strips = {}
    for room in ROOMS:
        img = room_image(room, layout_dir)
        models[room] = np.asarray(img.convert("RGB"), dtype=np.float32)
        strips[room] = np.asarray(img.convert("L"), dtype=np.float32)
    for name, f in sorted(stills.items()):
        still = out_dir / f"scene_{name}_640x360.png"
        left, top, err = locate(strips[f["room"]], still)
        views[name] = (left, top)
        note = ""
        tick = int(f["tick"])
        if f["pilot"] == "walk" and tick in DESIGNER_VIEW:
            note = f" designer_torch_landmark_left={DESIGNER_VIEW[tick]} diff={left - DESIGNER_VIEW[tick]:+d}"
        lines.append(f"{name} {left} {top} trimmed_abs_err={err:.2f}{note}")
    cache.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return views, models


# ------------------------------------------------------------------------------------------- sheets
def contact(pieces, scale: int, labels: bool, width: int = 560) -> Image.Image:
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
    d = ImageDraw.Draw(out)
    for pid, img, px, py in spots:
        out.alpha_composite(img.resize((img.width * scale, img.height * scale), Image.NEAREST),
                            (px * scale, py * scale))
        if labels:
            d.text((px * scale, (py - label_h) * scale + 2), pid, fill=(236, 226, 210, 255))
    return out


def strip(items, scale: int, pad: int = 6) -> Image.Image:
    w = sum(i.width * scale + pad for _, i in items) + pad
    h = max(i.height for _, i in items) * scale + 2 * pad + 12
    out = Image.new("RGBA", (w, h), BG)
    d = ImageDraw.Draw(out)
    x = pad
    for name, im in items:
        out.alpha_composite(im.resize((im.width * scale, im.height * scale), Image.NEAREST),
                            (x, h - pad - im.height * scale))
        d.text((x, 2), name, fill=(236, 226, 210, 255))
        x += im.width * scale + pad
    return out


def layout_diagram(still: Path, view: tuple, room: str) -> Image.Image:
    left, top = view
    contract = json.loads(ROOMS[room][1].read_text(encoding="utf-8"))
    sizes = {p["id"]: p["rect"][2:] for p in contract["pieces"]}
    kind = ROOMS[room][0]
    img = Image.open(still).convert("RGBA").resize((1280, 720), Image.NEAREST)
    d = ImageDraw.Draw(img)

    def sx(x):
        return (x - left) * 2

    def sy(y):
        return (y - top) * 2

    for y, label, col in ((580, "walking line W 580", (120, 255, 120)), (560, "floor back line 560", (120, 200, 255))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
        d.text((560, sy(y) - 12), label, fill=col + (255,))
    for a, b in OPENINGS[room]:
        d.rectangle((sx(a), sy(304), sx(b + 1) - 1, sy(544)), outline=(90, 160, 255, 150))
        d.line((sx(a), sy(520), sx(b + 1) - 1, sy(520)), fill=(90, 160, 255, 220))
        d.text((sx(a) + 3, sy(520) + 2), f"opening {a}..{b}: props start at y 520", fill=(120, 180, 255, 255))
    for p in contract["room_dressing"][f"rest_{'camp' if kind == 'camp' else 'aftermath'}"]["place"]:
        if p["piece"] not in sizes or p["piece"].startswith("torch_pool"):
            continue
        w, h = sizes[p["piece"]]
        x0, y0 = sx(p["x"]), sy(p["y"])
        if x0 > 1280 or x0 + w * 2 < 0 or y0 > 720 or y0 + h * 2 < 0:
            continue
        col = {"backwall_decal": (255, 90, 60, 255), "prop": (255, 210, 90, 255)}.get(p["layer"], (200, 160, 70, 255))
        d.rectangle((x0, y0, x0 + w * 2 - 1, y0 + h * 2 - 1), outline=col)
        if w * h >= 250:
            d.text((x0 + 3, y0 + 3), p["piece"], fill=col)
    if kind == "camp":
        px, py = 758 + 10, 526 + 22
        d.ellipse((sx(px) - 5, sy(py) - 5, sx(px) + 5, sy(py) + 5), outline=(255, 140, 60, 255), width=2)
        d.text((sx(px) + 8, sy(py) - 4), "fire light origin (768, 548)", fill=(255, 170, 90, 255))
    else:
        bx, by = camp.BAG_AT
        d.rectangle((sx(bx), sy(by), sx(bx + 40) - 1, sy(by + 122) - 1), outline=(255, 90, 200, 255))
        d.text((sx(bx), sy(by) - 11), "bag canvas (presenter) 620,450", fill=(255, 140, 220, 255))
        d.rectangle((sx(624), sy(516), sx(656) - 1, sy(580) - 1), outline=(120, 255, 200, 255))
        d.text((sx(624) + 36, sy(516)), "sim hit rect 624,516,32,64", fill=(120, 255, 200, 255))
    return img


def bag_strip(out_dir: Path, views: dict, stills: dict) -> None:
    names = [n for n in ("aft_punch_rest", "aft_punch_r1", "aft_punch_r2", "aft_punch_r1b", "aft_punch_l1",
                         "aft_punch_late") if n in views]
    if not names:
        return
    items = []
    for n in names:
        left, top = views[n]
        im = Image.open(out_dir / f"scene_{n}_640x360.png").convert("RGBA")
        x0, y0 = camp.BAG_AT[0] - 24 - left, camp.BAG_AT[1] - top
        items.append((f"{n.removeprefix('aft_punch_')} t{stills[n]['tick']}", im.crop((x0, y0, x0 + 88, y0 + 136))))
    strip(items, 4).save(out_dir / "bag_sway_window_x4.png")


# ------------------------------------------------------------------------------------------- G2
def dilate(mask: np.ndarray, n: int) -> np.ndarray:
    out = mask.copy()
    for dy in range(-n, n + 1):
        for dx in range(-n, n + 1):
            out |= np.roll(np.roll(mask, dy, 0), dx, 1)
    return out


def figures_lit(out_dir: Path, views: dict, stills: dict, models: dict) -> list[str]:
    """Gate note G2: for each of the four figures, on a 640x360 window still with the figure fully in view and no hero
    within the actor zone of it (Carl's x from the capture line minus 120 .. plus 60 on screen: Donut follows Carl on
    his left): (a) the figure's lit mean luma, (b) the mean luma of the 2 px ring outside its silhouette, (c) the
    |(b) - (a)| contrast over the top 40 % of its rows, (d) Carl's lit mean luma in the same still (the pixels the lit
    model of the room does not explain within Carl's box, plus-or-minus the difference threshold). Targets:
    (a) <= 45, (a) < (d), (c) >= 20."""
    lines = ["# gate note G2: the four survivor figures on the lit 640x360 window stills",
             "# (a) lit mean luma of the figure  (b) mean luma of the 2 px ring outside it  (c) |b - a| over its top 40 % rows",
             "# (d) Carl's lit mean luma in the same still (difference-mask estimate)  targets: a <= 45, a < d, c >= 20"]
    pieces = camp.all_pieces()
    for pid, (fx, fy) in FIGURE_AT.items():
        img = pieces[pid]
        best = None
        for name, f in sorted(stills.items()):
            if f["room"] != "rest_camp" or name not in views:
                continue
            left, top = views[name]
            carl = float(f.get("carl_x", "nan"))
            if not (left + 4 <= fx and fx + img.w <= left + 636):
                continue
            zone = (carl - 120, carl + 60)
            if not (fx + img.w < zone[0] or fx > zone[1]):
                continue
            dist = min(abs(fx - carl), abs(fx + img.w - carl))
            if best is None or dist > best[0]:
                best = (dist, name)
        if best is None:
            lines.append(f"{pid} no still shows it clear of the heroes")
            continue
        name = best[1]
        left, top = views[name]
        rgb = np.asarray(Image.open(out_dir / f"scene_{name}_640x360.png").convert("RGB"), dtype=np.float32)
        lum = luma(rgb)
        mask = np.zeros(lum.shape, dtype=bool)
        ox, oy = fx - left, fy - top
        for y in range(img.h):
            for x in range(img.w):
                if img.px[y][x] is not None:
                    mask[oy + y, ox + x] = True
        ring = dilate(mask, 2) & ~dilate(mask, 0) & ~mask
        ring &= ~(dilate(mask, 2) & ~mask & False)
        a = float(lum[mask].mean())
        rows = [oy + y for y in range(img.h) if any(img.px[y])]
        top_rows = set(range(min(rows), min(rows) + max(1, int(round(0.4 * (max(rows) - min(rows) + 1))))))
        sel = np.zeros(lum.shape, dtype=bool)
        for r in top_rows:
            sel[r, :] = True
        a_top = float(lum[mask & sel].mean())
        ring_top = ring & sel
        b = float(lum[ring].mean())
        b_top = float(lum[ring_top].mean())
        c = abs(b_top - a_top)
        d_lum = carl_luma(lum, rgb, models.get("rest_camp"), views[name], float(stills[name]["carl_x"]))
        ok = a <= 45 and (d_lum is None or a < d_lum) and c >= 20
        lines.append(f"{pid} still={name} (a)={a:.1f} (a_top40)={a_top:.1f} (b)={b:.1f} (b_top40)={b_top:.1f} "
                     f"(c)={c:.1f} (d)={'n/a' if d_lum is None else format(d_lum, '.1f')} "
                     f"targets a<=45:{'ok' if a <= 45 else 'FAIL'} a<d:{'ok' if d_lum is None or a < d_lum else 'FAIL'} "
                     f"c>=20:{'ok' if c >= 20 else 'FAIL'} -> {'PASS' if ok else 'FAIL'}")
    return lines


def carl_luma(lum, rgb, model, view, carl_x):
    """Carl's lit mean luma: the pixels of his box (x +- 14, the 52 rows above the walking line) that the lit model of
    the room does not explain, by more than the model's own error elsewhere in the actor band plus a margin."""
    if model is None:
        return None
    left, top = view
    cx, foot = int(round(carl_x)) - left, 580 - top
    box = (slice(foot - 52, foot), slice(max(0, cx - 14), min(640, cx + 14)))
    m = luma(model)[top:top + 360, left:left + 640]
    diff = np.abs(lum - m)
    band = diff[foot - 52:foot, :]
    far = np.ones(band.shape[1], dtype=bool)
    far[max(0, cx - 130):min(640, cx + 70)] = False
    base = float(np.percentile(band[:, far], 95)) if far.any() else 20.0
    sel = diff[box] > base + 10
    if sel.sum() < 40:
        return None
    return float(lum[box][sel].mean())


# ------------------------------------------------------------------------------------------- R1 and R2
def occupancy(room: str, layout_dir: Path, skip=()) -> np.ndarray:
    """Opaque pixels of every structure, prop and foreground piece of the dressed layout (the wall decals and the
    deck's collision tiles are not counted: a decal is what the arch and the pier cover, the deck is the floor)."""
    import env_kit as ek  # noqa: PLC0415
    kit = ek.load_kit(ROOMS[room][1])
    lay = json.loads((layout_dir / f"{room}.dressed.layout.json").read_text(encoding="utf-8"))
    occ = np.zeros((ROOM_H, ROOMS[room][2]), dtype=bool)
    for layer in ("structure", "prop", "foreground"):
        for p in lay["piece_layers"].get(layer, []):
            if p["piece"] in skip:
                continue
            d = kit.pieces[p["piece"]]
            a = np.asarray(kit.crop(d["atlas"], d["rect"]).convert("RGBA"))[..., 3] > 0
            h, w = a.shape
            x0, y0, x1, y1 = max(p["x"], 0), max(p["y"], 0), min(p["x"] + w, occ.shape[1]), min(p["y"] + h, ROOM_H)
            if x1 > x0 and y1 > y0:
                occ[y0:y1, x0:x1] |= a[y0 - p["y"]:y1 - p["y"], x0 - p["x"]:x1 - p["x"]]
    return occ


def visibility(layout_dir: Path) -> list[str]:
    """Reviews R1 and R2: the share of the motifs' pixels that show, and the chalk pixels on a plinth face."""
    pieces = camp.all_pieces()
    out = []
    occ = occupancy("rest_camp", layout_dir, skip=("camp_chalk_floor",))
    m = pieces["camp_motifs"]
    at = next(p for p in camp.CAMP_PLACE if p[1] == "camp_motifs")
    total = [(x, y) for y in range(m.h) for x in range(m.w) if m.px[y][x]]
    hidden = sum(1 for x, y in total if occ[at[3] + y, at[2] + x])
    out.append(f"R1 camp_motifs at ({at[2]}, {at[3]}): {len(total) - hidden} of {len(total)} opaque pixels are not under any "
               f"structure or prop piece ({100 * (len(total) - hidden) / len(total):.1f} % visible; target >= 90 %)")
    for room, pid in (("rest_camp", "camp_chalk_floor"), ("rest_aftermath", "aft_lane_chalk")):
        occ = occupancy(room, layout_dir, skip=("camp_chalk_floor", "aft_lane_chalk", "aft_sand_spill"))
        at = next(p for p in (camp.CAMP_PLACE + camp.AFT_PLACE) if p[1] == pid)
        img = pieces[pid]
        px = [(x, y) for y in range(img.h) for x in range(img.w) if img.px[y][x]]
        bad = sum(1 for x, y in px if occ[at[3] + y, at[2] + x])
        low = sum(1 for x, y in px if y >= camp.CHALK_LOW)
        out.append(f"R2 {pid}: {len(px)} pixels, {bad} on a plinth face (target 0), {low} on rows 573..576 in front of the plinths")
    return out


# ------------------------------------------------------------------------------------------- main
def main(argv: list[str]) -> int:
    out_dir = Path(argv[0])
    layout_dir = Path(argv[1]) if len(argv) > 1 and argv[1] else None
    pieces = camp.all_pieces()
    imgs = {pid: camp.to_image(img) for pid, img in pieces.items()}
    listed = [pid for pid in imgs if not pid.startswith("camp_fire_") and not pid.startswith("aft_heavy_bag_")]
    contact([(p, imgs[p]) for p in listed], 1, False).save(out_dir / "contact_sheet_x1.png")
    contact([(p, imgs[p]) for p in listed], 4, True, 360).save(out_dir / "contact_sheet_x4.png")
    strip([(f"fire {k}", imgs[f"camp_fire_{k}"]) for k in range(4)], 4).save(out_dir / "fire_loop_x4.png")
    strip([(n, imgs[f"aft_heavy_bag_{n}"]) for n in camp.BAG_FRAMES], 3).save(out_dir / "bag_frames_x3.png")
    stills = stills_of(out_dir)
    views, models = drawn_views(out_dir, layout_dir, stills)
    if views:
        bag_strip(out_dir, views, stills)
        for name, room in (("camp_figures", "rest_camp"), ("camp_hearth", "rest_camp"), ("aft_wreck", "rest_aftermath"),
                           ("aft_lane", "rest_aftermath")):
            if name in views:
                layout_diagram(out_dir / f"scene_{name}_640x360.png", views[name], room).save(
                    out_dir / f"layout_diagram_{name}_x2.png")
    lines = []
    if views and models:
        lines += figures_lit(out_dir, views, stills, models)
        (out_dir / "figures_lit.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    # ---- measurements
    meas = ["# AREA-CAMP-01 execution measurements (scripts/make_camp01_sheets.py)"]
    for pid in camp.EXEC_PIECES:
        img = pieces[pid]
        cols = {p for row in img.px for p in row if p}
        n = sum(1 for row in img.px for p in row if p)
        meas.append(f"{pid} {img.w}x{img.h} opaque={n} colours={len(cols)} mean_luma={camp.mean_luma(img):.1f}")
    for pid in camp.FIGURES:
        meas.append(f"{pid} mean_luma={camp.mean_luma(pieces[pid]):.1f} (target <= 45)")
    if layout_dir is not None:
        meas += visibility(layout_dir)
    meas.append(f"sway_table ticks={len(camp.SWAY_TABLE)}")
    meas += [f"KEY {pid} sha256={sha}" for pid, sha in camp.key_sha().items()]
    (out_dir / "measurements.txt").write_text("\n".join(meas) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
