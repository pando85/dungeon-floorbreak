#!/usr/bin/env python3
"""AREA-TROG-01 stage 1 offline evidence sheets and measurements (deterministic, from the committed grids, the dumped
layouts and the real window stills).

    python3 scripts/make_trogarea01_sheets.py OUT_DIR [LAYOUT_DIR]

Reads ``OUT_DIR/scene_*_640x360.png`` and ``OUT_DIR/capture_lines.txt`` (the runtime captures of
``make_trogarea01_evidence.sh``) and writes into OUT_DIR:
- ``contact_sheet_x1.png`` / ``contact_sheet_x4.png``: every stage-1 piece (the six keys of the run and the three
  executor step pieces) at native size on a neutral ground (the 4x sheet is labelled);
- ``layout_diagram_*_x2.png`` (needs LAYOUT_DIR): the room schema over a real still (placements, openings, bays, the
  walking line). The drawn view of each still is read from landmarks (the kit's lit model slid over the still's wall
  rows, cross-checked on the torch flames: flame centre = placement x + 7.5) and written to ``drawn_views.txt``, never
  taken from the logged ``cam``;
- ``lit_read.txt`` (TROG-ART-A12, gate note G6) and ``lit_read_masks_x2.png`` (the masks over the stills);
- ``measurements.txt``: sizes, colours, luma, the key pins, and (TROG-ART-A13) the visibility of each placed piece.
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import fftconvolve

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_trogarea01 as trog  # noqa: E402

BG = (58, 54, 66, 255)
ROOM = "trog_run"
CONTRACT = trog.CONTRACT
ROWS = (64, 190)                      # the still's wall rows (no HUD, no actors)
TOPS = range(196, 320)
VIEW_W, VIEW_H = 640, 360
# Per room: the openings, the bricked ones (the fill rect is a - 8 .. b + 48), the troglodytes at their roster x
# (sidecar), the stills that get a layout diagram, and the placed pieces the lit read counts.
CFG = {
    "trog_run": {
        "openings": [(184, 335), (520, 671), (856, 1007), (1192, 1343), (1528, 1679), (2024, 2175)],
        "bricked": [(184, 335), (1192, 1343)],
        "trogs": {"pygmy": ("trog_01", 640), "basher": ("trog_02", 1376), "virtuoso": ("trog_03", 2144)},
        "diagrams": ("t0_spawn", "pygmy_land", "basher_dormant", "virtuoso_dormant"),
        "counted": ("trog_",),
    },
    "trog_strength": {
        "openings": [(56, 207), (392, 543), (728, 879)],
        "bricked": [(728, 879)],
        "trogs": {"basher": ("trog_02", 800), "virtuoso": ("trog_03", 496)},
        "diagrams": ("strength_open", "strength_bench_beside", "strength_bench_climb"),
        "counted": ("trog_", "gym_"),
    },
}
OPENINGS, BRICKED, TROGS = CFG[ROOM]["openings"], CFG[ROOM]["bricked"], CFG[ROOM]["trogs"]
FRAME = {"trog_01": (80, 56, 30, 55), "trog_02": (112, 88, 48, 87), "trog_03": (128, 88, 44, 87)}
FLOOR = 580


def use_room(room: str) -> None:
    """Select the room the module-level helpers measure."""
    global ROOM, OPENINGS, BRICKED, TROGS
    ROOM = room
    OPENINGS, BRICKED, TROGS = CFG[room]["openings"], CFG[room]["bricked"], CFG[room]["trogs"]


# ------------------------------------------------------------------------------------------- inputs
def stills_of(out_dir: Path) -> dict[str, dict]:
    """name -> the TROG_ART_STILL fields of the 640x360 line."""
    out = {}
    for line in (out_dir / "capture_lines.txt").read_text(encoding="utf-8").splitlines():
        if not line.startswith("TROG_ART_STILL ") or " size=640x360 " not in line:
            continue
        f = dict(kv.split("=", 1) for kv in line.split()[1:])
        out[f["name"]] = f
    return out


def load_room(layout_dir: Path):
    import env_kit as ek  # noqa: PLC0415
    kit = ek.load_kit(CONTRACT)
    layout = json.loads((layout_dir / f"{ROOM}.dressed.layout.json").read_text(encoding="utf-8"))
    return kit, layout


def luma(a: np.ndarray) -> np.ndarray:
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def lstar(rgb: np.ndarray) -> np.ndarray:
    """CIE L* of sRGB values (0..255), as ``lookdev_compose.srgb_to_lab_l``."""
    c = rgb.astype(np.float64) / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    y = 0.2126 * lin[..., 0] + 0.7152 * lin[..., 1] + 0.0722 * lin[..., 2]
    return 116 * np.where(y > 216 / 24389, np.cbrt(y), (24389 / 27 * y + 16) / 116) - 16


# ------------------------------------------------------------------------------------------- the drawn view
def _err(win: np.ndarray, s: np.ndarray) -> float:
    diff = np.sort(np.abs(win - s), axis=None)
    return float(diff[: int(diff.size * 0.7)].mean())        # trimmed: actors, HUD text and effects drop out


def locate(strip: np.ndarray, still: Path) -> tuple[int, int, float]:
    """(left, top, error): a coarse 2D search on a 4x downsample, then a full-resolution refinement."""
    width = strip.shape[1]
    s = np.asarray(Image.open(still).convert("L"), dtype=np.float32)[ROWS[0]:ROWS[1]]
    s4 = s[::4, ::4]
    best = (1e30, 0, 0)
    for top in TOPS:
        for left in range(0, width - VIEW_W + 1, 4):
            win = strip[top + ROWS[0]:top + ROWS[1], left:left + VIEW_W][::4, ::4]
            err = _err(win, s4)
            if err < best[0]:
                best = (err, left, top)
    _, l0, t0 = best
    best = (1e30, 0, 0)
    for top in range(t0 - 4, t0 + 5):
        for left in range(max(0, l0 - 8), min(width - VIEW_W, l0 + 8) + 1):
            err = _err(strip[top + ROWS[0]:top + ROWS[1], left:left + VIEW_W], s)
            if err < best[0]:
                best = (err, left, top)
    return best[1], best[2], best[0]


def flame_landmarks(still: np.ndarray, torches: list, left: int, top: int) -> list[tuple[int, float, float]]:
    """(torch x, predicted flame centre on screen, measured centroid): the bright flame pixels (luma >= 235) in a
    +-12 px column window of each torch in view, the rows of its flame (torch y + 4 .. + 30)."""
    lum = luma(still.astype(np.float32))
    out = []
    for tx, ty in torches:
        pred = tx + 7.5 - left
        if not 16 <= pred <= VIEW_W - 16:
            continue
        y0, y1 = ty - top + 4, ty - top + 30
        x0, x1 = int(pred) - 12, int(pred) + 13
        win = lum[y0:y1, x0:x1]
        ys, xs = np.nonzero(win >= 235)
        if len(xs) >= 6:
            out.append((tx, pred, x0 + float(xs.mean()) + 0.5))
    return out


def drawn_views(out_dir: Path, layout_dir: Path | None, stills: dict, lines: list[str]):
    """{still name: (left, top)} and the lit model of the current room; the lines of `drawn_views.txt` are appended
    to `lines`. Without LAYOUT_DIR the committed cache is read."""
    views = {}
    if layout_dir is None:
        cache = out_dir / "drawn_views.txt"
        if cache.is_file():
            for line in cache.read_text(encoding="utf-8").splitlines():
                if line and not line.startswith("#"):
                    name, left, top = line.split()[:3]
                    views[name] = (int(left), int(top))
        return views, None
    import env_kit_light as lit  # noqa: PLC0415
    kit, layout = load_room(layout_dir)
    cols, rows = layout["grid_size"]
    img = lit.render_lit(kit, layout, [0, 0, cols * 16, rows * 16], 1)
    strip = np.asarray(img.convert("L"), dtype=np.float32)
    torches = [(p["x"], p["y"]) for p in layout["piece_layers"]["prop"] if p["piece"] == "torch_wall"]
    for name, f in sorted(stills.items()):
        still = out_dir / f"scene_{name}_640x360.png"
        left, top, err = locate(strip, still)
        views[name] = (left, top)
        marks = flame_landmarks(np.asarray(Image.open(still).convert("RGB")), torches, left, top)
        off = [m - p for _, p, m in marks]
        note = (f" torch_flames={len(marks)} mean_offset={sum(off) / len(off):+.2f}px max_abs={max(abs(o) for o in off):.2f}px"
                if off else " torch_flames=0")
        lines.append(f"{name} {left} {top} trimmed_abs_err={err:.2f}{note}")
    return views, (kit, layout)


# ------------------------------------------------------------------------------------------- sheets
def contact(pieces, scale: int, labels: bool, width: int = 560) -> Image.Image:
    pad, label_h = 6, (12 if labels else 0)
    x = y = row_h = 0
    spots = []
    for pid, img in pieces:
        w, h = img.width, img.height
        cell = max(w, -(-len(pid) * 6 // scale)) if labels else w       # a cell is at least as wide as its label
        if x + cell + pad > width and x > 0:
            x, y, row_h = 0, y + row_h + pad + label_h, 0
        spots.append((pid, img, x + pad, y + pad + label_h))
        x += cell + pad
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


def layout_diagram(still: Path, view: tuple, sizes: dict) -> Image.Image:
    """The room schema at 2x over a still. Every label sits in a staggered band under the HUD (never over an actor
    or a piece), on a dark plate with a tick down to what it names; the two line legends are at the bottom."""
    left, top = view
    img = Image.open(still).convert("RGBA").resize((1280, 720), Image.NEAREST)
    d = ImageDraw.Draw(img)

    def sx(x):
        return (x - left) * 2

    def sy(y):
        return (y - top) * 2

    labels: list[tuple[int, str, tuple]] = []           # (screen x, text, colour)
    for y, col in ((580, (120, 255, 120)), (560, (120, 200, 255))):
        d.line((0, sy(y), 1279, sy(y)), fill=col + (255,), width=1)
    for a, b in OPENINGS:
        bricked = (a, b) in BRICKED
        if not bricked:
            d.rectangle((sx(a), sy(304), sx(b + 1) - 1, sy(544)), outline=(90, 160, 255, 150))
            d.line((sx(a), sy(520), sx(b + 1) - 1, sy(520)), fill=(90, 160, 255, 220))
            labels.append((sx(a), f"open window {a}..{b}: props from y 520", (120, 180, 255, 255)))
        else:
            d.rectangle((sx(a - 8), sy(352), sx(b + 49) - 1, sy(560) - 1), outline=(255, 120, 90, 150))
            labels.append((sx(a - 8), f"bricked: fill {a - 8}..{b + 48}, y 352..559", (255, 150, 120, 255)))
    runs = {("trog_step_end_l", 352): "refuge face 352..415", ("trog_step_end_l", 1840): "bell step face 1840..1903",
            ("trog_shelf48_end_l", 288): "retreat shelf face 288..351", ("gym_mat_a", 760): "mats 760..903"}
    for p in trog.dressing()[ROOM]["place"]:
        w, h = sizes[p["piece"]]
        x0, y0 = sx(p["x"]), sy(p["y"])
        if x0 > 1280 or x0 + w * 2 < 0 or y0 > 720 or y0 + h * 2 < 0:
            continue
        col = {"backwall_decal": (255, 90, 60, 255), "prop": (255, 210, 90, 255)}.get(p["layer"], (200, 160, 70, 255))
        d.rectangle((x0, y0, x0 + w * 2 - 1, y0 + h * 2 - 1), outline=col)
        if p["piece"].startswith(("trog_step_", "trog_shelf48_", "gym_mat")):     # one label per run
            if (p["piece"], p["x"]) in runs:
                labels.append((x0, runs[(p["piece"], p["x"])], col))
        elif w * h >= 400:
            labels.append((x0, p["piece"], col))
    for key, (_, x) in TROGS.items():
        d.line((sx(x), sy(520), sx(x), sy(580)), fill=(255, 255, 255, 200))
        labels.append((sx(x), f"{key} spawn x {x}", (255, 255, 255, 255)))
    # the band: rows of 13 px from y 120, a label's row is the first whose last label ends 6 px before its x
    ends: list[int] = []
    for x, text, col in sorted(labels):
        if x > 1279:
            continue
        x = max(x, 2)
        w = int(d.textlength(text)) + 6
        row = next((i for i, e in enumerate(ends) if x >= e + 6), len(ends))
        if row == len(ends):
            ends.append(0)
        ends[row] = x + w
        y = 120 + row * 13
        d.rectangle((x, y, x + w, y + 11), fill=(10, 8, 14, 215))
        d.text((x + 3, y), text, fill=col)
        d.line((x, y + 12, x, y + 16), fill=col)
    for i, (text, col) in enumerate((("walking line W 580", (120, 255, 120)), ("floor back line 560", (120, 200, 255)))):
        d.text((8, 676 + i * 13), text, fill=col + (255,))
    return img


# ------------------------------------------------------------------------------------------- A13: visibility
def piece_image(kit, pid: str) -> Image.Image:
    p = kit.pieces[pid]
    return kit.crop(p["atlas"], p["rect"])


def visibility(kit, layout) -> list[str]:
    """TROG-ART-A13: for each placed `trog_*` piece the share of its opaque pixels that show behind the architecture
    (the prop and foreground layers emptied), with the props in front, and the backwall decal pixels over a null tile."""
    import env_kit_light as el  # noqa: PLC0415
    lines = []
    for name, items in layout["piece_layers"].items():
        for i, it in enumerate(items):
            if not it["piece"].startswith("trog_"):
                continue
            src = piece_image(kit, it["piece"])
            m = Image.new("RGBA", src.size, (255, 0, 255, 255))
            m.putalpha(src.getchannel("A"))
            mid = it["piece"] + "__mag"
            kit.atlases["scr_" + mid] = {"image": m}
            kit.pieces[mid] = {"id": mid, "atlas": "scr_" + mid, "rect": [0, 0, m.width, m.height]}
            lay = copy.deepcopy(layout)
            lay["piece_layers"][name][i] = dict(it, piece=mid)
            view = [it["x"], it["y"], m.width, m.height]
            a = m.getchannel("A").load()
            res = []
            for drop in (("prop", "foreground"), ()):
                l2 = copy.deepcopy(lay)
                for dl in drop:
                    if dl != name:
                        l2["piece_layers"][dl] = []
                px = el.render_lit(kit, l2, view, 1, lit=False).convert("RGB").load()
                tot = vis = 0
                for y in range(m.height):
                    for x in range(m.width):
                        if a[x, y]:
                            tot += 1
                            vis += px[x, y] == (255, 0, 255)
                res.append((vis, tot))
            (vis, tot), (vis2, _) = res
            holes = 0
            if name == "backwall_decal":
                bw = layout["tile_layers"]["backwall"]
                for y in range(m.height):
                    for x in range(m.width):
                        if a[x, y]:
                            holes += bw[(it["y"] + y) // 16][(it["x"] + x) // 16] is None
            ok = vis >= 0.9 * tot and holes == 0
            lines.append(f"visibility {ROOM} {name} {it['piece']} @({it['x']},{it['y']}): {vis}/{tot} = "
                         f"{100 * vis / tot:.1f}% behind architecture (target >= 90), {100 * vis2 / tot:.1f}% with the "
                         f"props in front; backwall_null_px={holes} (target 0) {'ok' if ok else 'FAIL'}")
    return lines


def visible_mask(kit, layout, it: dict, name: str) -> np.ndarray:
    """The pixels of a placed piece that show in the layout (props and foreground in front, no actors)."""
    import env_kit_light as el  # noqa: PLC0415
    src = piece_image(kit, it["piece"])
    m = Image.new("RGBA", src.size, (255, 0, 255, 255))
    m.putalpha(src.getchannel("A"))
    mid = it["piece"] + "__magv"
    kit.atlases["scr_" + mid] = {"image": m}
    kit.pieces[mid] = {"id": mid, "atlas": "scr_" + mid, "rect": [0, 0, m.width, m.height]}
    lay = copy.deepcopy(layout)
    idx = next(i for i, q in enumerate(lay["piece_layers"][name]) if q["x"] == it["x"] and q["y"] == it["y"]
               and q["piece"] == it["piece"])
    lay["piece_layers"][name][idx] = dict(it, piece=mid)
    rgb = np.asarray(el.render_lit(kit, lay, [it["x"], it["y"], m.width, m.height], 1, lit=False).convert("RGB"))
    return (rgb == (255, 0, 255)).all(axis=2)


# ------------------------------------------------------------------------------------------- A12: the lit read
FRAMES = 16
IDLE_FRAMES = (0, 1, 2)               # the TROG-A frame map: Dormant, Rest and a standing Approach loop frames 0..2
MIN_NCC = 0.55
SPECIES_OF = {"pygmy": "trog_01", "basher": "trog_02", "virtuoso": "trog_03"}


def sprite_frames(species: str) -> list[Image.Image]:
    w, h, _, _ = FRAME[SPECIES_OF[species]]
    sheet = Image.open(ROOT / f"generated/art/{SPECIES_OF[species]}.png").convert("RGBA")
    return [sheet.crop((i * w, 0, (i + 1) * w, h)) for i in range(FRAMES)]


def _corr(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    return fftconvolve(a, b[::-1, ::-1], mode="full")


def match_sprite(lum: np.ndarray, species: str):
    """Find a troglodyte in a window still: the (frame, mirrored, x0, y0, ncc) of the best masked normalised
    cross-correlation of the species' atlas frames (both facings, any position, partly outside the view allowed)
    against the still's luma. The pose and the position are not logged, so they are measured on the still."""
    img = lum.astype(np.float64)
    img[:ROWS[0]] = 0.0
    valid = np.ones_like(img)
    valid[:ROWS[0]] = 0.0
    best = (-1.0, 0, False, 0, 0)
    for i, frame in enumerate(sprite_frames(species)):
        for mirrored in (False, True):
            fr = frame.transpose(Image.FLIP_LEFT_RIGHT) if mirrored else frame
            m = (np.asarray(fr.getchannel("A")) > 0).astype(np.float64)
            t = luma(np.asarray(fr.convert("RGB")).astype(np.float64)) * m
            n = _corr(valid, m)
            si, si2 = _corr(valid * img, m), _corr(valid * img * img, m)
            st, st2, sit = _corr(valid, t), _corr(valid, t * t), _corr(valid * img, t)
            with np.errstate(divide="ignore", invalid="ignore"):
                cov = sit - si * st / n
                var = (si2 - si * si / n) * (st2 - st * st / n)
                ncc = cov / np.sqrt(var)
            ncc = np.where((n >= 0.5 * m.sum()) & (var > 1e-6), ncc, -1.0)
            k = int(np.argmax(ncc))
            y, x = divmod(k, ncc.shape[1])
            if ncc[y, x] > best[0]:
                best = (float(ncc[y, x]), i, mirrored, x - (fr.width - 1), y - (fr.height - 1))
    return best[1], best[2], best[3], best[4], best[0]


def pose_tag(species: str, frame_index: int) -> str:
    """The atlas tag (`idle`, `walk`, `attack_*`, `hurt`, `down`) of a frame, from the exported atlas JSON."""
    meta = json.loads((ROOT / f"generated/art/{SPECIES_OF[species]}.json").read_text(encoding="utf-8"))["meta"]
    for t in meta["frameTags"]:
        if t["from"] <= frame_index <= t["to"]:
            return t["name"]
    return "?"


def trog_mask(species: str, frame_index: int, mirrored: bool) -> Image.Image:
    fr = sprite_frames(species)[frame_index]
    return fr.transpose(Image.FLIP_LEFT_RIGHT) if mirrored else fr


def lit_read(out_dir: Path, stills: dict, views: dict, room, header: bool = True):
    """TROG-ART-A12 on the real stills (spec 10.1, gate note G6): the sleeping troglodytes' actor L* and edge
    contrast (`lookdev_compose.readability`, ring 6) and every in-view dressing piece's lit mean L*, measured on the
    window still over the pixels the lit model explains (no actor, HUD or effect pixels)."""
    from lookdev_compose import readability  # noqa: PLC0415
    import env_kit_light as el  # noqa: PLC0415
    kit, layout = room
    head = ["# TROG-ART-A12 (spec 10.1, gate note G6) measured on the real 640x360 window stills",
             "# actor L* and edge_dL: lookdev_compose.readability(still, mask, box, ring=6); the troglodyte's pose and",
             "# position are not logged, so the mask is the species' atlas frame (both facings) that best matches the",
             "# still (masked normalised cross-correlation, ncc >= 0.55); a living troglodyte counts when >= 50 % of its",
             "# mask is inside the view",
             "# piece L*: mean CIE L* of the placed piece's visible pixels (visible in the layout, not explained away",
             "# by an actor, the HUD or an effect: |L*(still) - L*(lit model)| <= 12)",
             "# gated stills (G6): a troglodyte in an idle pose (atlas frames 0..2: dormant, rest, standing) is in view;",
             "# attack and walk poses are measured and reported, not gated",
             "# verdict per piece: PASS (<= the lowest in-view troglodyte's actor L*), NOTE (key piece within 2.0 L*,",
             "# a non-blocking owner note tied to TROG-ART-Q6), FAIL"]
    lines = head if header else []
    lines.append(f"## room {ROOM}")
    counted = CFG[ROOM]["counted"]
    # every new or reused dressing piece in view; the emissive cold tube is excluded (spec 10.1)
    placed = [(n, it) for n, items in layout["piece_layers"].items() for it in items
              if it["piece"].startswith(counted) and it["piece"] != "gym_fluoro_cold"]
    vis_cache = {}
    overview = []
    worst = {"pieces": [], "edge": []}
    lowest_gated = None
    for name in sorted(stills, key=lambda n: int(stills[n]["tick"])):
        if name not in views:
            continue
        left, top = views[name]
        still_img = Image.open(out_dir / f"scene_{name}_640x360.png").convert("RGB")
        still = np.asarray(still_img)
        sl = lstar(still)
        model = np.asarray(el.render_lit(kit, layout, [left, top, VIEW_W, VIEW_H], 1).convert("RGB"))
        ml = lstar(model)
        lum = luma(still.astype(np.float32))
        actors = []
        occluders = []
        found = []
        for species in sorted(TROGS):
            fi, mirrored, x0m, y0m, ncc = match_sprite(lum, species)
            if ncc < MIN_NCC:
                continue
            fr = trog_mask(species, fi, mirrored)
            alpha = np.asarray(fr.getchannel("A")) > 0
            mask = np.zeros((VIEW_H, VIEW_W), dtype=np.uint8)
            x0, x1 = max(0, x0m), min(VIEW_W, x0m + fr.width)
            y0, y1 = max(0, y0m), min(VIEW_H, y0m + fr.height)
            if x1 > x0 and y1 > y0:
                mask[y0:y1, x0:x1] = alpha[y0 - y0m:y1 - y0m, x0 - x0m:x1 - x0m] * 255
            share = int((mask > 0).sum()) / int(alpha.sum())
            idle = fi in IDLE_FRAMES
            tag = pose_tag(species, fi)
            if share < 0.5:
                found.append(f"  {species}: located (frame {fi}, {tag}, ncc {ncc:.2f}) but only {100 * share:.0f} % of the "
                             f"mask is in view (< 50 %): not counted")
                continue
            ys, xs = np.nonzero(mask)
            box = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
            r = readability(still_img, Image.fromarray(mask, "L"), [box], ring=6)[0]
            living = tag != "down"
            occluders.append(mask)
            if living:
                actors.append((species, r, mask, idle))
            if idle:
                verdict = "PASS" if r["silhouette_edge_mean_abs_dL"] >= 15 else "FAIL edge_dL < 15"
            elif not living:
                verdict = "down: not counted (G6 counts living troglodytes only)"
            else:
                verdict = ("reported" if r["silhouette_edge_mean_abs_dL"] >= 15
                           else "reported (edge_dL < 15 in a pose that is not gated)")
            found.append(f"  {species}: frame {fi} ({tag}){' mirrored' if mirrored else ''} at screen ({x0m},{y0m}) "
                         f"ncc={ncc:.2f}, {100 * share:.0f} % of the mask in view, "
                         f"{'idle' if idle else tag + ' pose'}: actor_L={r['actor_mean_L']} "
                         f"surround_L={r['surround_mean_L']} delta_L={r['delta_L']} "
                         f"edge_dL={r['silhouette_edge_mean_abs_dL']} edge_samples={r['edge_samples']} {verdict}")
            if idle:
                worst["edge"].append((r["silhouette_edge_mean_abs_dL"], name, species))
        gated = any(a[3] for a in actors)
        lines.append(f"still {name} tick={stills[name]['tick']} view_left={left} view_top={top} "
                     f"gated={'yes' if gated else 'no'}")
        lines += found
        actors = [(sp, r, mk) for sp, r, mk, _ in actors]
        ref = min((r["actor_mean_L"] for _, r, _ in actors), default=None)
        if gated and ref is not None:
            lowest_gated = ref if lowest_gated is None else min(lowest_gated, ref)
        base = ref
        for lname, it in placed:
            w, h = piece_image(kit, it["piece"]).size
            if it["x"] + w <= left or it["x"] >= left + VIEW_W or it["y"] + h <= top or it["y"] >= top + VIEW_H:
                continue
            key = (it["piece"], it["x"], it["y"])
            if key not in vis_cache:
                vis_cache[key] = visible_mask(kit, layout, it, lname)
            vis = vis_cache[key]
            sel = np.zeros((VIEW_H, VIEW_W), dtype=bool)
            x0, y0 = it["x"] - left, it["y"] - top
            ax0, ay0, ax1, ay1 = max(0, x0), max(0, y0), min(VIEW_W, x0 + w), min(VIEW_H, y0 + h)
            sel[ay0:ay1, ax0:ax1] = vis[ay0 - y0:ay1 - y0, ax0 - x0:ax1 - x0]
            shown = int(sel.sum())
            for mk in occluders:
                sel &= mk == 0
            kept_vis = int(sel.sum())
            sel &= np.abs(sl - ml) <= 12
            n = int(sel.sum())
            dropped = f"dropped {shown - kept_vis} under a troglodyte, {kept_vis - n} not explained by the lit model"
            if n < 12:
                continue
            mean = float(sl[sel].mean())
            is_key = it["piece"] in trog.KEY_PIECE_IDS
            if base is None:
                verdict = "n/a (no troglodyte measured)"
            elif mean <= base:
                verdict = "PASS"
            elif is_key and mean - base <= 2.0:
                verdict = f"NOTE (key piece over by {mean - base:.1f} L*, non-blocking, Q6)"
            else:
                verdict = f"FAIL (over by {mean - base:.1f} L*, {'key' if is_key else 'executor'} piece)"
            if gated and base is not None:
                worst["pieces"].append((mean - base, name, it["piece"], verdict))
            lines.append(f"  piece {it['piece']}@({it['x']},{it['y']}): mean_L={mean:.1f} px={n} of {shown} in view ({dropped}) vs lowest actor L* "
                         f"{'-' if base is None else format(base, '.1f')} {verdict}")
        overview.append((name, still_img, occluders))
    if worst["pieces"]:
        margin, name, pid, verdict = max(worst["pieces"])
        lines.append(f"summary {ROOM}: thinnest piece margin on the gated stills: {pid} in {name}: {margin:+.1f} L* "
                     f"(<= 0 passes) {verdict}")
    if worst["edge"]:
        e, name, sp = min(worst["edge"])
        lines.append(f"summary {ROOM}: lowest troglodyte edge_dL on the gated stills: {e} ({sp} in {name}; >= 15 passes)")
    # the masks over the stills (x2), for review (the caller joins the rooms' cells)
    cells = []
    for name, im, masks in overview:
        if not masks:
            continue
        big = im.resize((VIEW_W * 2, VIEW_H * 2), Image.NEAREST).convert("RGBA")
        d = ImageDraw.Draw(big)
        for mk in masks:
            edge = np.asarray(Image.fromarray(mk, "L").resize((VIEW_W * 2, VIEW_H * 2), Image.NEAREST)) > 0
            ys, xs = np.nonzero(edge & ~(np.roll(edge, 1, 0) & np.roll(edge, -1, 0) & np.roll(edge, 1, 1) & np.roll(edge, -1, 1)))
            for x, y in zip(xs, ys):
                d.point((int(x), int(y)), fill=(0, 255, 120, 255))
        d.text((8, 8), name, fill=(255, 255, 0, 255))
        cells.append(big)
    return lines, cells


# ------------------------------------------------------------------------------------------- main
def main(argv: list[str]) -> int:
    out_dir = Path(argv[0])
    layout_dir = Path(argv[1]) if len(argv) > 1 and argv[1] else None
    pieces = trog.all_pieces()
    imgs = {pid: trog.to_image(img) for pid, img in pieces.items()}
    listed = list(imgs)
    contact([(p, imgs[p]) for p in listed], 1, False).save(out_dir / "contact_sheet_x1.png")
    contact([(p, imgs[p]) for p in listed], 4, True, 360).save(out_dir / "contact_sheet_x4.png")
    all_stills = stills_of(out_dir)
    meas = ["# AREA-TROG-01 stages 1 and 2 execution measurements (scripts/make_trogarea01_sheets.py)"]
    for pid in trog.EXEC_PIECES:
        img = pieces[pid]
        cols = {p for row in img.px for p in row if p}
        n = sum(1 for row in img.px for p in row if p)
        meas.append(f"{pid} {img.w}x{img.h} opaque={n} colours={len(cols)} mean_luma={trog.mean_luma(img):.1f}")
    dv = ["# still_name view_left view_top (landmarks: the kit's lit model slid over the still's wall rows; "
          "flame check: |measured - predicted| of the torch flame centres, flame centre = placement x + 7.5)"]
    lit_lines, cells, first = [], [], True
    for room_id in CFG:
        use_room(room_id)
        stills = {n: f for n, f in all_stills.items() if f["room"] == room_id}
        views, room = drawn_views(out_dir, layout_dir, stills, dv)
        if room is None:
            continue
        kit, layout = room
        sizes = {pid: d["rect"][2:] for pid, d in kit.pieces.items()}
        for name in CFG[room_id]["diagrams"]:
            if name in views:
                layout_diagram(out_dir / f"scene_{name}_640x360.png", views[name], sizes).save(
                    out_dir / f"layout_diagram_{name}_x2.png")
        meas += visibility(kit, layout)
        if views:
            lines, c = lit_read(out_dir, stills, views, room, header=first)
            lit_lines += lines
            cells += c
            first = False
    if layout_dir is not None:
        (out_dir / "drawn_views.txt").write_text("\n".join(dv) + "\n", encoding="utf-8")
        (out_dir / "lit_read.txt").write_text("\n".join(lit_lines) + "\n", encoding="utf-8")
        if cells:
            sheet = Image.new("RGBA", (VIEW_W * 2 * 2 + 8, (VIEW_H * 2 + 8) * ((len(cells) + 1) // 2)), BG)
            for i, c in enumerate(cells):
                sheet.alpha_composite(c, ((i % 2) * (VIEW_W * 2 + 8), (i // 2) * (VIEW_H * 2 + 8)))
            sheet.convert("RGB").save(out_dir / "lit_read_masks_x2.png")
    meas += [f"KEY {pid} sha256={sha}" for pid, sha in trog.key_sha().items()]
    (out_dir / "measurements.txt").write_text("\n".join(meas) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
