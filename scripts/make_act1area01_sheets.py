#!/usr/bin/env python3
"""AREA-ACT1-01 evidence sheets and measurements from the real window stills (scripts/make_act1area01_evidence.sh).

    python3 scripts/make_act1area01_sheets.py OUT_DIR [LAYOUT_DIR]

Writes into OUT_DIR: `drawn_views.txt` (the measured drawn view of each still), `measurements.txt` (the KEY pins, the
visibility of every placed piece and the lit read of spec section 10), `contact_1x.png`, `contact_x4.png` (the new
pieces and spot crops at x4), `strip_barricade.png` and `strip_timbers.png`, `strip_guild.png`. Without LAYOUT_DIR (the
composed layouts written by the ignored test `act1_kit::tests::dump_act1_layouts`) only the sheets that need no model are
rebuilt and the committed `drawn_views.txt` is read.
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_act1area01 as act1  # noqa: E402

BG = (58, 54, 66, 255)
VIEW_W, VIEW_H = 640, 360
ROWS = (64, 190)                      # the still's wall rows (no HUD)
TOPS = range(196, 300)               # the camera follows a jumping hero up (S3 timbers)
# The presenter pieces' state per still: (timber a smashed, timber b smashed, barricade smashed, grant box open).
STATE = {
    "chase_spawn": (0, 0, 0, 0), "chase_s2_breach_heroes": (0, 0, 0, 0), "chase_s2_emergence": (0, 0, 0, 0), "chase_s3_timbers": (0, 0, 0, 0),
    "chase_s3_after_first_smash": (1, 0, 0, 0), "chase_s4_alcove": (1, 0, 0, 0), "chase_s5_barricade_intact": (1, 1, 0, 0),
    "chase_s5_barricade_broken": (1, 1, 1, 0), "chase_s5_after_defeat": (1, 1, 1, 0),
    "chase_s5_after_respawn": (0, 0, 0, 0), "chase_wedge": (1, 1, 1, 0), "shore_spawn": (0, 0, 0, 0),
    "guild_door_closed": (0, 0, 0, 0), "guild_door_open": (0, 0, 0, 1),
}
PRESENTER = ("timber_set_intact", "timber_set_smashed", "timber_barricade_full_intact", "timber_barricade_full_smashed",
             "act1_grant_box_closed", "act1_grant_box_open", "guild_portal_open")


def luma(a: np.ndarray) -> np.ndarray:
    return 0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2]


def lstar(rgb: np.ndarray) -> np.ndarray:
    """CIE L* of sRGB values (0..255), as ``lookdev_compose.srgb_to_lab_l``."""
    c = rgb.astype(np.float64) / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    y = 0.2126 * lin[..., 0] + 0.7152 * lin[..., 1] + 0.0722 * lin[..., 2]
    return 116 * np.where(y > 216 / 24389, np.cbrt(y), (24389 / 27 * y + 16) / 116) - 16


def stills_of(out_dir: Path) -> dict:
    out = {}
    for line in (out_dir / "capture_lines.txt").read_text(encoding="utf-8").splitlines():
        if line.startswith("ACT1_ART_STILL "):
            f = dict(kv.split("=", 1) for kv in line.split()[1:])
            out[f["name"]] = f
    return out


def contract() -> dict:
    return json.loads(act1.CONTRACT.read_text(encoding="utf-8"))


def load_room(layout_dir: Path, room: str):
    import env_kit as ek  # noqa: PLC0415
    kit = ek.load_kit(act1.CONTRACT)
    layout = json.loads((layout_dir / f"{room}.dressed.layout.json").read_text(encoding="utf-8"))
    return kit, layout


def with_presenters(layout: dict, name: str) -> dict:
    """The dressed layout plus the presenter pieces of a still's state (they are not placements in the game)."""
    lay = copy.deepcopy(layout)
    ta, tb, barr, box = STATE[name]
    add = []
    if name.startswith("chase_") and name != "chase_wedge_x":
        for (x, y), smashed in zip(((1104, 456), (1424, 456)), (ta, tb)):
            add.append(("timber_set_smashed" if smashed else "timber_set_intact", x, y))
        add.append(("timber_barricade_full_smashed" if barr else "timber_barricade_full_intact", 2304, 304))
    if name.startswith("guild_"):
        add.append(("act1_grant_box_open" if box else "act1_grant_box_closed", 340, 552))
        if box:
            add.append(("guild_portal_open", 536, 444))
    for piece, x, y in add:
        lay["piece_layers"]["prop"].append({"piece": piece, "x": x, "y": y})
    return lay


# ------------------------------------------------------------------------------------------- the drawn view
def _err(win: np.ndarray, s: np.ndarray) -> float:
    diff = np.sort(np.abs(win - s), axis=None)
    return float(diff[: int(diff.size * 0.7)].mean())        # trimmed: actors, HUD text and effects drop out


def locate(strip: np.ndarray, still: Path, hint_x: int | None) -> tuple[int, int, float]:
    width = strip.shape[1]
    s = np.asarray(Image.open(still).convert("L"), dtype=np.float32)[ROWS[0]:ROWS[1]]
    s4 = s[::4, ::4]
    lo, hi = (0, width - VIEW_W) if hint_x is None else (max(0, hint_x - 96), min(width - VIEW_W, hint_x + 96))
    best = (1e30, 0, 0)
    for top in TOPS:
        for left in range(lo, hi + 1, 4):
            err = _err(strip[top + ROWS[0]:top + ROWS[1], left:left + VIEW_W][::4, ::4], s4)
            if err < best[0]:
                best = (err, left, top)
    _, l0, t0 = best
    best = (1e30, 0, 0)
    for top in range(t0 - 2, t0 + 3):
        for left in range(max(0, l0 - 6), min(width - VIEW_W, l0 + 6) + 1):
            if not 0 <= top <= strip.shape[0] - VIEW_H:
                continue
            err = _err(strip[top + ROWS[0]:top + ROWS[1], left:left + VIEW_W], s)
            if err < best[0]:
                best = (err, left, top)
    return best[1], best[2], best[0]


def drawn_views(out_dir: Path, layout_dir, stills: dict, lines: list):
    views = {}
    if layout_dir is None:
        cache = out_dir / "drawn_views.txt"
        if cache.is_file():
            for line in cache.read_text(encoding="utf-8").splitlines():
                if line and not line.startswith("#"):
                    name, left, top = line.split()[:3]
                    views[name] = (int(left), int(top))
        return views, {}
    import env_kit_light as lit  # noqa: PLC0415
    rooms, strips = {}, {}
    for name, f in sorted(stills.items()):
        room = f["room"]
        if room not in rooms:
            rooms[room] = load_room(layout_dir, room)
            kit, layout = rooms[room]
            cols, rows = layout["grid_size"]
            img = lit.render_lit(kit, layout, [0, 0, cols * 16, rows * 16], 1)
            strips[room] = np.asarray(img.convert("L"), dtype=np.float32)
        left, top, err = locate(strips[room], out_dir / f"scene_{name}_640x360.png", int(f["cam_x"]))
        views[name] = (left, top)
        lines.append(f"{name} {left} {top} trimmed_abs_err={err:.2f} logged_cam_x={f['cam_x']}")
    return views, rooms


# ------------------------------------------------------------------------------------------- visibility (A9)
def piece_image(kit, pid: str) -> Image.Image:
    p = kit.pieces[pid]
    return kit.crop(p["atlas"], p["rect"])


def _magenta(kit, it, name):
    src = piece_image(kit, it["piece"])
    m = Image.new("RGBA", src.size, (255, 0, 255, 255))
    m.putalpha(src.getchannel("A"))
    mid = it["piece"] + "__mag" + name
    kit.atlases["scr_" + mid] = {"image": m}
    kit.pieces[mid] = {"id": mid, "atlas": "scr_" + mid, "rect": [0, 0, m.width, m.height]}
    return m, mid


def visibility(kit, layout, room: str) -> list:
    """ACT1-ART-A9: for each placed piece of the room's dressing the share of its opaque pixels that show behind the
    architecture (the prop and foreground layers emptied), with the props in front, and the backwall decal pixels over a
    null tile; plus the bricked bay's null backwall tiles for the chase."""
    import env_kit_light as el  # noqa: PLC0415
    lines = []
    placed = {(p["layer"], p["piece"], p["x"], p["y"]) for p in contract()["room_dressing"][room]["place"]}
    for name, items in layout["piece_layers"].items():
        for i, it in enumerate(items):
            if (name, it["piece"], it["x"], it["y"]) not in placed:
                continue
            m, mid = _magenta(kit, it, "")
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
            lines.append(f"visibility {room} {name} {it['piece']} @({it['x']},{it['y']}): {vis}/{tot} = "
                         f"{100 * vis / tot:.1f}% behind architecture (target >= 90), {100 * vis2 / tot:.1f}% with the "
                         f"props in front; backwall_null_px={holes} (target 0) {'ok' if ok else 'FAIL'}")
    if room == "act1_chase":
        bw = layout["tile_layers"]["backwall"]
        nulls = sum(bw[r][c] is None for r in range(22, 35) for c in range(584 // 16, 736 // 16))
        lines.append(f"visibility {room} bricked bay x 584..735 y 352..559: backwall_null_px={nulls * 256} "
                     f"(target 0) {'ok' if nulls == 0 else 'FAIL'}")
    return lines


def visible_mask(kit, layout, it: dict, name: str) -> np.ndarray:
    import env_kit_light as el  # noqa: PLC0415
    m, mid = _magenta(kit, it, "v")
    lay = copy.deepcopy(layout)
    idx = next(i for i, q in enumerate(lay["piece_layers"][name]) if q["x"] == it["x"] and q["y"] == it["y"]
               and q["piece"] == it["piece"])
    lay["piece_layers"][name][idx] = dict(it, piece=mid)
    rgb = np.asarray(el.render_lit(kit, lay, [it["x"], it["y"], m.width, m.height], 1, lit=False).convert("RGB"))
    return (rgb == (255, 0, 255)).all(axis=2)


# ------------------------------------------------------------------------------------------- the lit read (A9)
def components(diff: np.ndarray, min_px: int = 120) -> list:
    """The actor, HUD and effect pixels of a still (the pixels the lit model does not explain) as connected components."""
    d = ndimage.binary_closing(diff, structure=np.ones((3, 3)), iterations=2)
    d[:62] = False                                          # the HUD rows
    lab, n = ndimage.label(d)
    out = []
    for i in range(1, n + 1):
        mask = lab == i
        if int(mask.sum()) >= min_px:
            ys, xs = np.nonzero(mask)
            out.append((int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1, mask))
    return out


def lit_read(out_dir: Path, stills: dict, views: dict, rooms: dict) -> list:
    from lookdev_compose import readability  # noqa: PLC0415
    import env_kit_light as el  # noqa: PLC0415
    lines = ["# ACT1-ART-A9 (spec section 10) measured on the real 640x360 window stills",
             "# model: the lit render of the dressed layout plus the presenter pieces of the still's state; a pixel is",
             "# 'explained' when |L*(still) - L*(model)| <= 12; the others (actors, HUD, effects) are components",
             "# piece L*: mean CIE L* of a placed piece's visible, explained pixels (n >= 12); the torch flames and pools",
             "# are excluded; edge_dL: lookdev_compose.readability(still, mask, box, ring=6)"]
    for name in sorted(stills, key=lambda n: (stills[n]["room"], int(stills[n]["tick"]))):
        if name not in views:
            continue
        f = stills[name]
        kit, layout = rooms[f["room"]]
        lay = with_presenters(layout, name)
        left, top = views[name]
        still_img = Image.open(out_dir / f"scene_{name}_640x360.png").convert("RGB")
        still = np.asarray(still_img)
        sl = lstar(still)
        model = np.asarray(el.render_lit(kit, lay, [left, top, VIEW_W, VIEW_H], 1).convert("RGB"))
        ml = lstar(model)
        explained = np.abs(sl - ml) <= 12
        comps = components(~explained)
        lines.append(f"still {name} room={f['room']} tick={f['tick']} view_left={left} view_top={top} "
                     f"explained={100 * explained[62:].mean():.1f}% components={len(comps)}")
        for x0, y0, x1, y1, mask in comps:
            r = readability(still_img, Image.fromarray(mask.astype(np.uint8) * 255, "L"), [(x0, y0, x1, y1)], ring=6)[0]
            lines.append(f"  component bbox=({x0},{y0})-({x1},{y1}) px={int(mask.sum())} room_x={left + x0}..{left + x1} "
                         f"actor_L={r['actor_mean_L']} edge_dL={r['silhouette_edge_mean_abs_dL']}")
        occluded = np.zeros((VIEW_H, VIEW_W), dtype=bool)
        for *_, mask in comps:
            occluded |= mask
        pieces = []
        for lname, items in lay["piece_layers"].items():
            for it in items:
                if it["piece"].startswith("torch_"):
                    continue
                w, h = piece_image(kit, it["piece"]).size
                if it["x"] + w <= left or it["x"] >= left + VIEW_W or it["y"] + h <= top or it["y"] >= top + VIEW_H:
                    continue
                vis = visible_mask(kit, lay, it, lname)
                sel = np.zeros((VIEW_H, VIEW_W), dtype=bool)
                x0, y0 = it["x"] - left, it["y"] - top
                ax0, ay0, ax1, ay1 = max(0, x0), max(0, y0), min(VIEW_W, x0 + w), min(VIEW_H, y0 + h)
                sel[ay0:ay1, ax0:ax1] = vis[ay0 - y0:ay1 - y0, ax0 - x0:ax1 - x0]
                sel &= explained & ~occluded
                sel[:62] = False
                n = int(sel.sum())
                if n >= 12:
                    pieces.append((float(sl[sel].mean()), it["piece"], it["x"], it["y"], n, sel))
        pieces.sort(reverse=True)
        for mean, pid, x, y, n, _ in pieces[:8]:
            lines.append(f"  piece {pid}@({x},{y}): mean_L={mean:.1f} px={n}")
        by = {(pid, x, y): (mean, n, sel) for mean, pid, x, y, n, sel in pieces}
        lines += verdicts(name, f, by, comps, still, sl, left, top, kit, lay, readability, still_img)
    return lines


def verdicts(name, f, by, comps, still, sl, left, top, kit, lay, readability, still_img) -> list:
    out = []
    mean_of = lambda pid: next((v[0] for k, v in by.items() if k[0] == pid), None)      # noqa: E731
    if name == "chase_s4_alcove":
        niche = mean_of("act1_alcove_niche")
        others = [(m, k[0]) for k, (m, n, _) in by.items() if k[0] != "act1_alcove_niche" and not k[0].startswith("act1_step")]
        if niche is None:
            out.append("  S4 verdict: act1_alcove_niche not measurable in this still FAIL")
        else:
            top_other = max(others) if others else (0.0, "-")
            out.append(f"  S4 verdict (target 1): niche mean_L={niche:.1f}; the brightest other shape is {top_other[1]} "
                       f"at {top_other[0]:.1f}: {'PASS' if niche > top_other[0] else 'FAIL'} (margin {niche - top_other[0]:+.1f})")
            p90 = {k: float(np.percentile(sl[v[2]], 90)) for k, v in by.items()}
            nk = next(k for k in by if k[0] == "act1_alcove_niche")
            rest = sorted(((v, k[0]) for k, v in p90.items() if k[0] != "act1_alcove_niche" and not k[0].startswith("act1_step")),
                          reverse=True)
            out.append(f"  S4 info (not the spec's metric): 90th-percentile L*: niche {p90[nk]:.1f}; next {rest[0][1]} {rest[0][0]:.1f}")
    if name in ("chase_s2_emergence", "chase_spawn"):
        # the void: the breach key's void pixels (mortar_00) visible and explained
        for k, (m, n, sel) in by.items():
            if k[0] == "act1_gallery_breach":
                rows = act1.act1_gallery_breach().px
                void = np.zeros((VIEW_H, VIEW_W), dtype=bool)
                for y in range(act1.BR_H):
                    for x in range(act1.BR_W):
                        if rows[y][x] == ("mortar", 0):
                            vx, vy = act1.BR_AT[0] + x - left, act1.BR_AT[1] + y - top
                            if 0 <= vx < VIEW_W and 0 <= vy < VIEW_H:
                                void[vy, vx] = True
                v = void & sel
                if int(v.sum()) >= 12:
                    mm = float(sl[v].mean())
                    out.append(f"  S2 verdict (target 2, void): the breach void's lit mean L*={mm:.1f} over {int(v.sum())} px "
                               f"(<= 12): {'PASS' if mm <= 12 else 'FAIL'}")
    if name == "chase_s2_breach_heroes":
        hits = [(x0, y0, x1, y1, mask) for x0, y0, x1, y1, mask in comps
                if y1 - y0 >= 30 and left + x1 > act1.BR_AT[0] and left + x0 < act1.BR_AT[0] + act1.BR_W]
        for x0, y0, x1, y1, mask in hits:
            r = readability(still_img, Image.fromarray(mask.astype(np.uint8) * 255, "L"), [(x0, y0, x1, y1)], ring=6)[0]
            e = r["silhouette_edge_mean_abs_dL"]
            out.append(f"  S2 verdict (target 2, heroes): component room_x={left + x0}..{left + x1} in front of the breach "
                       f"edge_dL={e} (>= 15): {'PASS' if e >= 15 else 'FAIL'}")
        if not hits:
            out.append("  S2 verdict (target 2, heroes): no hero component in front of the breach FAIL")
    if name == "chase_s2_emergence":
        for x0, y0, x1, y1, mask in comps:
            if (x1 - x0) >= 120 and left + x0 < act1.BR_AT[0] + act1.BR_W:
                r = readability(still_img, Image.fromarray(mask.astype(np.uint8) * 255, "L"), [(x0, y0, x1, y1)], ring=6)[0]
                e = r["silhouette_edge_mean_abs_dL"]
                out.append(f"  S2 verdict (target 2, machine): component room_x={left + x0}..{left + x1} (the rig) edge_dL={e} "
                           f"(>= 15): {'PASS' if e >= 15 else 'FAIL'}")
    if name == "chase_s3_timbers":
        for k, (m, n, sel) in by.items():
            if k[0].startswith("timber_set"):
                ys, xs = np.nonzero(sel)
                r = readability(still_img, Image.fromarray(sel.astype(np.uint8) * 255, "L"),
                                [(int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)], ring=6)[0]
                e = r["silhouette_edge_mean_abs_dL"]
                out.append(f"  S3 verdict (target 3): {k[0]}@({k[1]},{k[2]}) edge_dL={e} px={n} (>= 10): "
                           f"{'PASS' if e >= 10 else 'FAIL'}")
    return out


def guild_read(out_dir: Path, stills: dict, views: dict, rooms: dict, lines: list) -> None:
    """Spec 10.4 on the guild stills: the grant box between the desk and Donut, the hearth against Mordecai's face."""
    from lookdev_compose import readability  # noqa: PLC0415
    import env_kit_light as el  # noqa: PLC0415
    for name in ("guild_door_closed", "guild_door_open"):
        if name not in views:
            continue
        kit, layout = rooms["act1_guild"]
        lay = with_presenters(layout, name)
        left, top = views[name]
        still_img = Image.open(out_dir / f"scene_{name}_640x360.png").convert("RGB")
        sl = lstar(np.asarray(still_img))
        ml = lstar(np.asarray(el.render_lit(kit, lay, [left, top, VIEW_W, VIEW_H], 1).convert("RGB")))
        explained = np.abs(sl - ml) <= 12
        comps = components(~explained)
        occluded = np.zeros((VIEW_H, VIEW_W), dtype=bool)
        for *_, mask in comps:
            occluded |= mask
        means = {}
        for lname, items in lay["piece_layers"].items():
            for it in items:
                if it["piece"] in ("act1_grant_box_closed", "act1_grant_box_open", "refuge_desk", "refuge_hearth"):
                    w, h = piece_image(kit, it["piece"]).size
                    vis = visible_mask(kit, lay, it, lname)
                    sel = np.zeros((VIEW_H, VIEW_W), dtype=bool)
                    x0, y0 = it["x"] - left, it["y"] - top
                    sel[max(0, y0):min(VIEW_H, y0 + h), max(0, x0):min(VIEW_W, x0 + w)] = vis[
                        max(0, y0) - y0:min(VIEW_H, y0 + h) - y0, max(0, x0) - x0:min(VIEW_W, x0 + w) - x0]
                    sel &= explained & ~occluded
                    if int(sel.sum()) >= 12:
                        means["box" if "grant" in it["piece"] else it["piece"].replace("refuge_", "")] = float(sl[sel].mean())
        # the two actors next to the box and the guide: by bbox height (Carl 52 px > Donut 40 px > ...), Mordecai at x 380
        boxes = [(c[0], c[1], c[2], c[3], c[4]) for c in comps if c[3] - c[1] >= 20 and c[1] > 140]
        guide = next((c for c in boxes if abs(left + (c[0] + c[2]) / 2 - 380) <= 24 and 40 <= c[3] - c[1] <= 60), None)
        donut = next((c for c in boxes if c is not guide and c[2] - c[0] >= 46 and 34 <= c[3] - c[1] <= 44), None)
        txt = f"guild {name}: " + " ".join(f"{k}_L={v:.1f}" for k, v in sorted(means.items()))
        if donut is not None:
            dl = float(sl[donut[4]].mean())
            txt += f" donut_L={dl:.1f} (component {donut[0]},{donut[1]}-{donut[2]},{donut[3]})"
            if "box" in means:
                ok = means["box"] <= dl and ("desk" not in means or means["box"] >= means["desk"])
                txt += f" | box <= donut and >= desk: {'PASS' if ok else 'FAIL'}"
        if guide is not None:
            x0, y0, x1, y1, mask = guide
            face = np.zeros_like(mask)
            face[y0:y0 + 12] = mask[y0:y0 + 12]
            fl = float(sl[face].mean()) if face.any() else float("nan")
            txt += f" | mordecai_face_L={fl:.1f}"
            if "hearth" in means:
                txt += f" hearth_L={means['hearth']:.1f} hearth <= face: {'PASS' if means['hearth'] <= fl else 'FAIL'}"
        lines.append(txt)


# ------------------------------------------------------------------------------------------- sheets
def contact(stills_dir: Path, names: list, cols: int, scale: int, path: Path) -> None:
    ims = [Image.open(stills_dir / f"scene_{n}_640x360.png").convert("RGB") for n in names]
    rows = -(-len(ims) // cols)
    sheet = Image.new("RGB", (cols * 640 * scale, rows * 360 * scale), BG[:3])
    d = ImageDraw.Draw(sheet)
    for i, im in enumerate(ims):
        x, y = (i % cols) * 640 * scale, (i // cols) * 360 * scale
        sheet.paste(im.resize((640 * scale, 360 * scale), Image.NEAREST) if scale > 1 else im, (x, y))
        d.text((x + 8 * scale, y + 62 * scale), names[i], fill=(255, 255, 0))
    sheet.save(path)


def crop_row(stills_dir: Path, spec: list, scale: int, path: Path) -> None:
    """spec: [(still name, (x0, y0, x1, y1)), ...] joined left to right at `scale`."""
    ims = []
    for name, box in spec:
        im = Image.open(stills_dir / f"scene_{name}_640x360.png").convert("RGB").crop(box)
        ims.append(im.resize((im.width * scale, im.height * scale), Image.NEAREST))
    sheet = Image.new("RGB", (sum(i.width for i in ims) + 6 * (len(ims) - 1), max(i.height for i in ims)), BG[:3])
    x = 0
    for im in ims:
        sheet.paste(im, (x, 0))
        x += im.width + 6
    sheet.save(path)


def pieces_sheet(path: Path) -> None:
    """The new and derived pieces at x4 on a flat ground (the gate's composite for the executed pieces)."""
    pieces = act1.all_pieces()
    order = ["act1_gallery_breach", "act1_grant_box_closed", "act1_grant_box_open", "act1_alcove_niche",
             "act1_step_mid", "act1_step_end_l", "act1_step_end_r", "act1_soffit_a"]
    scale, pad = 4, 8
    imgs = [(pid, act1.to_image(pieces[pid])) for pid in order]
    cols = [[imgs[0]], [imgs[1], imgs[2], imgs[4], imgs[5], imgs[6]], [imgs[3]], [imgs[7]]]
    w = sum(max(i.width for _, i in c) * scale + pad for c in cols) + pad
    h = max(sum(i.height * scale + pad for _, i in c) for c in cols) + pad
    sheet = Image.new("RGBA", (w, h), BG)
    x = pad
    for c in cols:
        y = pad
        for pid, im in c:
            sheet.alpha_composite(im.resize((im.width * scale, im.height * scale), Image.NEAREST), (x, y))
            y += im.height * scale + pad
        x += max(i.width for _, i in c) * scale + pad
    sheet.convert("RGB").save(path)


def main(argv: list) -> int:
    out_dir = Path(argv[1])
    layout_dir = Path(argv[2]) if len(argv) > 2 and argv[2] else None
    stills = stills_of(out_dir)
    lines_views = ["# the drawn view (left, top) of each still, measured against the lit render of the dressed room"]
    views, rooms = drawn_views(out_dir, layout_dir, stills, lines_views)
    if layout_dir is not None:
        (out_dir / "drawn_views.txt").write_text("\n".join(lines_views) + "\n", encoding="utf-8")
        text = ["# AREA-ACT1-01 measurements (scripts/make_act1area01_sheets.py)",
                "# the frozen keys' pins (equal gate/measurements.txt)"]
        text += [f"KEY {pid} sha256={sha}" for pid, sha in act1.key_sha().items()]
        text.append("# A9 visibility: the share of each placed piece's opaque pixels that show behind the architecture")
        for room, (kit, layout) in sorted(rooms.items()):
            text += visibility(kit, layout, room)
        lr = lit_read(out_dir, stills, views, rooms)
        guild_read(out_dir, stills, views, rooms, lr)
        text += lr
        (out_dir / "measurements.txt").write_text("\n".join(text) + "\n", encoding="utf-8")
    names = [n for n in STATE if (out_dir / f"scene_{n}_640x360.png").is_file()]
    contact(out_dir, names, 2, 1, out_dir / "contact_1x.png")
    pieces_sheet(out_dir / "contact_x4_pieces.png")
    crop_row(out_dir, [("chase_s5_barricade_intact", (320, 0, 640, 330)), ("chase_s5_barricade_broken", (320, 0, 640, 330)),
                       ("chase_s5_after_defeat", (320, 0, 640, 330)), ("chase_s5_after_respawn", (320, 0, 640, 330))],
             2, out_dir / "strip_barricade_x2.png")
    crop_row(out_dir, [("chase_s2_breach_heroes", (0, 60, 320, 330)), ("chase_s2_emergence", (0, 60, 320, 330))],
             2, out_dir / "strip_breach_x2.png")
    crop_row(out_dir, [("chase_s3_timbers", (160, 60, 480, 330)), ("chase_s3_after_first_smash", (160, 60, 480, 330))],
             2, out_dir / "strip_timbers_x2.png")
    crop_row(out_dir, [("guild_door_closed", (180, 150, 580, 300)), ("guild_door_open", (180, 150, 580, 300))],
             3, out_dir / "strip_guild_x3.png")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
