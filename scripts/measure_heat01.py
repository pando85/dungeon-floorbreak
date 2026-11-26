#!/usr/bin/env python3
"""AREA-HEAT-01 measurements (the gate's C2, C4, C5 and R6, spec A8) on the lit render of room 0.

Uses the kit's own lit method (``scripts/env_kit_light.py``: the A1 lighting model on the exported atlases, actor
frame 0 on the walking line of its column, ``lookdev_compose.readability``: mean CIE L* of the actor pixels vs a
6 px ring around the actor box), as ``scripts/measure_rings01.py`` does. The composed layouts of room 0, plain and
dressed, come from the ignored test ``heat_kit::tests::dump_heat_layouts``:

    HEAT_LAYOUT_DIR=DIR cargo test --lib -- --ignored dump_heat_layouts
    python3 scripts/measure_heat01.py DIR [OUT.txt] [STILLS_DIR]

``STILLS_DIR`` (optional) holds the runtime captures ``scene_*_640x360.png`` of ``make_heat01_evidence.sh``: the
walkable-top check is then repeated on the real window pixels (the runtime's own lighting).

Prints (and writes) one line per measurement. The plain layout is the greybox room without the dressing: the
baseline of "no worse than on plain brick in the same light".
"""
from __future__ import annotations

import colorsys
import json
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import author_heat01 as heat  # noqa: E402
import env_kit as ek  # noqa: E402
import env_kit_light as lit  # noqa: E402
from lookdev_compose import srgb_to_lab_l  # noqa: E402

CONTRACT = "assets/source/terrain/floor01/floor01_heat_kit.json"
CARL = "assets/manifests/carl_book1_h52.json"
GOBLIN = "assets/manifests/goblin_a_h36.json"
W = 580
ROOM_W = 2320
VIEW_TOP = 289          # the pen still's drawn view top (spec section 2): the 640 x 360 view
# Walkable tops of the kit (name, x0, x1, top y): the entry shelf, the dock and the bumper (spec section 2).
TOPS = (("shelf", 1120, 1216, 548), ("dock", 1888, 2304, 548), ("bumper", 1856, 1888, 500))
# The actor spots of R6 (x of the feet): beside the plume, over the plume, llama B's spot, the dock.
SPOTS = (1078, 1110, 1488, 1968)
BAYS = ((112, 320), (448, 656), (784, 992), (1248, 1456), (1584, 1792))


def view_around(x: int, h: int = 360, width: int = 640) -> list[int]:
    left = max(0, min(ROOM_W - width, x - width // 2))
    return [left, VIEW_TOP, width, h]


def load_layout(dir_: Path, kind: str) -> dict:
    return json.loads((dir_ / f"corral.{kind}.layout.json").read_text(encoding="utf-8"))


def render(kit, layout, view, entries):
    actors, boxes = ek.actor_layer(kit, layout, entries)
    img = lit.render_lit(kit, layout, view, 1, actors, boxes=boxes)
    return img, lit.actor_mask(actors, view), boxes


def actor_lines(dir_: Path, kit, who: str, manifest: str, x: int) -> list[str]:
    out = []
    for kind in ("plain", "dressed"):
        layout = load_layout(dir_, kind)
        view = view_around(x)
        img, mask, boxes = render(kit, layout, view, [{"manifest": manifest, "x": x}])
        m = lit.measure(img, mask, boxes, view)[0]
        out.append(f"ACTOR_SEPARATION actor={who} x={x} layout={kind} actor_L={m['actor_mean_L']} "
                   f"ring_L={m['surround_mean_L']} dL={m['delta_L']} contrast={m['luminance_contrast_ratio']} "
                   f"edge_dL={m['silhouette_edge_mean_abs_dL']}")
    return out


def run_means(img, y: int, x0: int, x1: int, run: int = 13) -> float:
    """The brightest mean L* of a ``run`` px horizontal run of row ``y`` between x0 and x1 (view px)."""
    px = img.convert("RGB").load()
    best = 0.0
    xs = list(range(max(0, x0), min(img.width, x1)))
    ls = [srgb_to_lab_l(px[x, y])[0] for x in xs]
    for i in range(0, max(1, len(ls) - run + 1)):
        window = ls[i:i + run]
        best = max(best, sum(window) / len(window))
    return best


def walkable_lines(dir_: Path, kit) -> list[str]:
    """C2 on the lit model: a walkable top's first row (``stone`` 8) against the brightest 13 px horizontal run
    within +-48 px of it (rows top-1..top+3, the platform's own face, and the deck below the walking line excepted;
    the layers of the render without actors hold no torch flame)."""
    out = []
    for kind in ("plain", "dressed"):
        layout = load_layout(dir_, kind)
        for name, x0, x1, py in TOPS:
            view = view_around((x0 + x1) // 2)
            img = lit.render_lit(kit, layout, view, 1)
            vx, vy = view[0], view[1]
            a, b = max(x0, vx) - vx, min(x1, vx + view[2]) - vx
            top_run = max(run_means(img, py + dy - vy, a, b) for dy in (0, 1))
            rival, rival_y = 0.0, 0
            for y in range(py - 48, min(py + 48, W - 24)):
                if py - 1 <= y <= py + 3 or not (0 <= y - vy < view[3]):
                    continue
                v = run_means(img, y - vy, a, b)
                if v > rival:
                    rival, rival_y = v, y
            out.append(f"WALKABLE_TOP source=lit_model platform={name} layout={kind} top_arris_run_L={top_run:.1f} "
                       f"brightest_rival_run_L={rival:.1f} at_y={rival_y} brightest={'top' if top_run >= rival else 'rival'}")
    return out


def runtime_walkable(stills: Path, layout_dir: Path) -> list[str]:
    """C2 on the runtime pixels: the same check on the window captures (the dressed stills at 640 x 360). The
    drawn view (left, top) of each still comes from ``drawn_views.txt`` (landmarks and a 2D match,
    ``scripts/heat01_view.py``). The arris is the platform's own columns (6 px inset) of its first rows; the rival is the brightest
    13 px run in the same columns within +-48 px. Pixels that differ from the lit model of the static room by more
    than 40 grey levels (the actors, the torch flames and their bloom, the HUD) are not candidates."""
    import numpy as np  # noqa: PLC0415
    import heat01_view as hv  # noqa: PLC0415
    views = {}
    for line in (stills / "drawn_views.txt").read_text(encoding="utf-8").splitlines():
        name, left, top = line.split()
        views[name] = (int(left), int(top))
    strip = hv.room_strip(layout_dir)
    pools = pool_mask(load_layout(layout_dir, "plain"))
    out = []
    for name in ("m0_goblins", "m1_throat", "pen_entry", "pen_rails", "dock", "m3_exit", "bare_run"):
        path = stills / f"scene_{name}_640x360.png"
        if name not in views or not path.is_file():
            continue
        left, vtop = views[name]
        img = Image.open(path).convert("RGB")
        px = img.load()
        still_g = np.asarray(img.convert("L"), dtype=np.int16)
        model_g = strip[vtop:vtop + 360, left:left + 640].astype(np.int16)
        static = np.abs(still_g - model_g) <= 40
        for plat, x0, x1, py in TOPS:
            a, b = max(x0, left) - left + 6, min(x1, left + 640) - left - 6
            if b - a < min(x1 - x0 - 12, 40):          # a platform cut by the view edge is not measured
                continue
            sy = py - vtop
            top_l = max(run_means(img, sy + dy, a, b) for dy in (0, 1))
            rival, ry, outside, oy = 0.0, 0, 0.0, 0
            for y in range(max(0, sy - 48), min(img.height, sy + 48, W - 24 - vtop)):
                if sy - 1 <= y <= sy + 3:
                    continue
                ok = [static[y, x] and not _flame(px[x, y]) for x in range(a, b)]
                ls = [srgb_to_lab_l(px[x, y])[0] if o else 0.0 for x, o in zip(range(a, b), ok)]
                lo = [v if not pools[vtop + y][left + x] else 0.0 for x, v in zip(range(a, b), ls)]
                for i in range(max(1, len(ls) - 12)):
                    v = sum(ls[i:i + 13]) / len(ls[i:i + 13])
                    if v > rival:
                        rival, ry = v, y
                    w = sum(lo[i:i + 13]) / len(lo[i:i + 13])
                    if w > outside:
                        outside, oy = w, y
            out.append(f"WALKABLE_TOP source=runtime still={name} platform={plat} view_left={left} view_top={vtop} "
                       f"arris_screen_x={a}..{b} top_arris_run_L={top_l:.1f} brightest_rival_run_L={rival:.1f} "
                       f"at_world_y={ry + vtop} brightest={'top' if top_l >= rival else 'rival'} "
                       f"rival_outside_torch_pool_L={outside:.1f} at_world_y={oy + vtop} "
                       f"C2_with_pool_exemption={'holds' if top_l >= outside else 'FAILS'}")
    return out


def _flame(rgb) -> bool:
    r, g, b = rgb
    return r > 215 and g > 150 and b < 140


def pool_mask(layout: dict) -> list[list[int]]:
    """The torch pools' re-shaded pixels (the opaque pixels of the composer's pool decals) of the plain layout."""
    base = json.loads((ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json").read_text(encoding="utf-8"))
    rect = {p["id"]: p["rect"] for p in base["pieces"]}
    atlas = Image.open(ROOT / "generated/art/floor01_walldecal.png").convert("RGBA")
    h = layout["pixel_size"][1]
    mask = [[0] * ROOM_W for _ in range(h)]
    for p in layout["piece_layers"]["backwall_decal"]:
        if not p["piece"].startswith("torch_pool"):
            continue
        rx, ry, rw, rh = rect[p["piece"]]
        for yy in range(rh):
            for xx in range(rw):
                if atlas.getpixel((rx + xx, ry + yy))[3] and 0 <= p["x"] + xx < ROOM_W and 0 <= p["y"] + yy < h:
                    mask[p["y"] + yy][p["x"] + xx] = 1
    return mask


def free_spans(mask, y0: int, y1: int, x_from: int, x_to: int, min_w: int = 16) -> list[tuple]:
    """The runs of columns in x_from..x_to with no pooled pixel in rows y0..y1-1 that are not inside a bay opening."""
    def free(x):
        return not any(mask[y][x] for y in range(y0, y1)) and not any(a <= x < b for a, b in BAYS)
    spans, start = [], None
    for x in range(x_from, x_to + 1):
        ok = x < x_to and free(x)
        if ok and start is None:
            start = x
        elif not ok and start is not None:
            if x - start >= min_w:
                spans.append((start, x))
            start = None
    return spans


def overlap_lines(layout: dict) -> list[str]:
    mask = pool_mask(layout)
    out = []
    for rows, label in (((464, 544), "rows 464..543 (wall_a, wall_b)"), ((512, 560), "rows 512..559 (wall_c)"),
                        ((544, 560), "rows 544..559 (trace)")):
        spans = free_spans(mask, rows[0], rows[1], 0, ROOM_W)
        out.append(f"FREE_SPANS {label} unpooled brick outside the bay openings, 16 px or wider: {spans}")
    placed = {(p["piece"], p["x"], p["y"]) for p in load_dressing()["place"] if p["layer"] == "backwall_decal"}
    cands = [("heat_scorch_wall_a", 1088, 464), ("heat_scorch_wall_b", 1920, 480), ("heat_trace_wall_a", 392, 544),
             ("heat_trace_wall_a", 384, 544)]
    cands += [("heat_scorch_wall_c", x, 512) for x in range(448, 1856, 32)
              if not any(a < x + 32 and b > x for a, b in BAYS) and not _on_pier(x, x + 32)]
    fns = {"heat_scorch_wall_a": heat.heat_scorch_wall_a, "heat_scorch_wall_b": heat.heat_scorch_wall_b,
           "heat_scorch_wall_c": heat.heat_scorch_wall_c, "heat_trace_wall_a": heat.heat_trace_wall_a}
    for pid, x, y in cands:
        img = fns[pid]()
        opaque = pooled = 0
        for yy in range(img.h):
            for xx in range(img.w):
                if img.px[yy][xx] is not None:
                    opaque += 1
                    pooled += mask[y + yy][x + xx]
        out.append(f"POOL_OVERLAP piece={pid} at=({x}, {y}) on_bond={x % 32 == 0 and y % 16 == 0} opaque_px={opaque} "
                   f"pooled_px={pooled} placed={'yes' if (pid, x, y) in placed else 'no'}")
    return out


def _on_pier(x0: int, x1: int) -> bool:
    bays = (80, 416, 752, 1216, 1552)
    return any(x0 < b + 72 and x1 > b for b in bays) or any(x0 < b + 304 and x1 > b + 224 for b in bays)


def load_dressing() -> dict:
    return json.loads((ROOT / CONTRACT).read_text(encoding="utf-8"))["room_dressing"]["corral"]


def coverage_lines(dressed: dict) -> list[str]:
    """A8: wall decals over the visible plain brick between y 352 and 560, and floor decals over the deck's back
    half (y 561..572), for every 640 px window (x step 32). The brick denominator drops the piers, the arch rings
    and the bay openings (the cells of the backwall that are empty), so the ratio is an upper bound."""
    contract = json.loads((ROOT / CONTRACT).read_text(encoding="utf-8"))
    base = json.loads((ROOT / "assets/source/terrain/floor01/floor01_catacomb_kit.json").read_text(encoding="utf-8"))
    size = {p["id"]: p["rect"][2:] for p in base["pieces"] + contract["pieces"]}
    fns = dict(heat.ATLASES["floor01_heat_wall"]["pieces"] + heat.ATLASES["floor01_heat_structure"]["pieces"])
    brick = [[0] * ROOM_W for _ in range(360, 560)]
    rows = dressed["tile_layers"]["backwall"]
    for r in range(352 // 16, 560 // 16):
        for c, tid in enumerate(rows[r]):
            if tid is not None and c * 16 < ROOM_W:
                for yy in range(16):
                    for xx in range(16):
                        if 352 <= r * 16 + yy < 560:
                            brick[r * 16 + yy - 360][c * 16 + xx] = 1
    for p in dressed["piece_layers"]["structure"]:
        if p["piece"].startswith(("pier_", "arch_")):
            w, h = size[p["piece"]]
            for y in range(max(352, p["y"]), min(560, p["y"] + h)):
                for x in range(max(0, p["x"]), min(ROOM_W, p["x"] + w)):
                    brick[y - 360][x] = 0
    wall = [[0] * ROOM_W for _ in range(200)]
    floor = [[0] * ROOM_W for _ in range(12)]
    for p in dressed["piece_layers"]["backwall_decal"]:
        if p["piece"].startswith("heat_scorch_wall") or p["piece"].startswith("heat_trace_wall"):
            img = fns[p["piece"]]()
            for yy in range(img.h):
                for xx in range(img.w):
                    y = p["y"] + yy
                    if img.px[yy][xx] is not None and 352 <= y < 560:
                        wall[y - 360][p["x"] + xx] = 1
    for p in dressed["piece_layers"]["structure"]:
        if p["piece"].startswith(("heat_scorch_floor", "heat_ash_floor", "heat_trace_spit")):
            img = fns[p["piece"]]()
            for yy in range(img.h):
                for xx in range(img.w):
                    y = p["y"] + yy
                    if img.px[yy][xx] is not None and 561 <= y <= 572:
                        floor[y - 561][p["x"] + xx] = 1
    wmax = fmax = 0.0
    wat = fat = 0
    for x0 in range(0, ROOM_W - 640 + 1, 32):
        b = sum(sum(row[x0:x0 + 640]) for row in brick)
        w_ = sum(sum(row[x0:x0 + 640]) for row in wall)
        f_ = sum(sum(row[x0:x0 + 640]) for row in floor)
        wr = w_ / b if b else 0.0
        fr = f_ / (12 * 640)
        if wr > wmax:
            wmax, wat = wr, x0
        if fr > fmax:
            fmax, fat = fr, x0
    return [f"A8_WALL_COVERAGE max={wmax * 100:.2f}% at_window_x={wat} cap=18% {'ok' if wmax <= 0.18 else 'FAIL'}",
            f"A8_FLOOR_COVERAGE max={fmax * 100:.2f}% at_window_x={fat} cap=25% {'ok' if fmax <= 0.25 else 'FAIL'}"]


def hue_deg(rgb) -> float:
    r, g, b = (c / 255 for c in rgb)
    return colorsys.rgb_to_hsv(r, g, b)[0] * 360


def partition_lines() -> list[str]:
    """The partition against the opening behind it on the gate composite (the real pen still with the unlit key
    pixels pasted at (1380, 532); a runtime still cannot show it before CORRAL-D, HEAT-AD1): mean L* of the key's
    pixels, of the still's pixels behind them, the difference and the hue angle difference of the mean colours."""
    still = Image.open(ROOT / "docs/evidence/area-heat-01/gate/scene_pen_before_1x.png").convert("RGB")
    vx, vy = heat.VIEW
    out = []
    for name, fn in (("intact", heat.heat_partition_intact), ("weakened", heat.heat_partition_weakened),
                     ("broken", heat.heat_partition_broken)):
        img = fn()
        keys, behind = [], []
        for y in range(img.h):
            for x in range(img.w):
                if img.px[y][x] is None:
                    continue
                hexc = heat.RAMPS[img.px[y][x][0]][img.px[y][x][1]]
                keys.append(tuple(int(hexc[i:i + 2], 16) for i in (1, 3, 5)))
                behind.append(still.getpixel((heat.PARTITION_AT[0] - vx + x, heat.PARTITION_AT[1] - vy + y)))
        mean = lambda ps: tuple(sum(p[i] for p in ps) / len(ps) for i in range(3))  # noqa: E731
        ka, ba = mean(keys), mean(behind)
        kl = sum(srgb_to_lab_l(p)[0] for p in keys) / len(keys)
        bl = sum(srgb_to_lab_l(p)[0] for p in behind) / len(behind)
        dh = abs(hue_deg(ka) - hue_deg(ba))
        dh = min(dh, 360 - dh)
        out.append(f"PARTITION_VS_OPENING source=gate_composite(CORRAL-D pending) state={name} partition_L={kl:.1f} "
                   f"behind_L={bl:.1f} dL={kl - bl:.1f} mean_hue_diff_deg={dh:.0f}")
    return out


def main(argv: list[str]) -> int:
    dir_ = Path(argv[0])
    kit = ek.load_kit(CONTRACT)
    lines = ["# AREA-HEAT-01 measurements: the kit's lit model (scripts/env_kit_light.py) on the composed layouts of room 0, "
             "plain and dressed; runtime lines read the window captures (make_heat01_evidence.sh)"]
    lines += overlap_lines(load_layout(dir_, "plain"))
    lines += coverage_lines(load_layout(dir_, "dressed"))
    for who, manifest in (("goblin", GOBLIN), ("carl", CARL)):
        for x in SPOTS:
            lines += actor_lines(dir_, kit, who, manifest, x)
    lines += walkable_lines(dir_, kit)
    if len(argv) > 2:
        lines += runtime_walkable(Path(argv[2]), dir_)
    lines += partition_lines()
    text = "\n".join(lines) + "\n"
    print(text, end="")
    if len(argv) > 1 and argv[1]:
        Path(argv[1]).write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
