#!/usr/bin/env python3
"""Validate and preview modular environment kits.

An environment kit is described by an ``env-kit-v1`` contract (tile/piece IDs,
atlas rects, layer roles, collidable flags, anchors, animation frames) and is
exercised by one or more ``env-kit-layout-v1`` sample layouts. This tool checks
the contract against the exported atlases, manifests and pixel grids, checks the
layouts against the contract, and renders deterministic art previews (contact
sheets and corridor compositions) **from the exported atlases only**.

The previews are art-review aids, not Bevy screenshots or gameplay evidence.

Usage::

    python3 scripts/env_kit.py validate --contract C.json [--layout L.json ...]
    python3 scripts/env_kit.py render --layout L.json --out DIR
    python3 scripts/env_kit.py all            # every layout under assets/levels/
"""
from __future__ import annotations

import argparse
import json
import math
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
LEVELS = ROOT / "assets" / "levels"
OUT_ROOT = ROOT / "generated" / "art" / "env_kit"

ROLES = ("collision", "backwall", "structure", "prop", "foreground")
TILE_ROLES = ("collision", "backwall")
ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")
# Runtime IDs must stay IP-neutral (fan-concept names live in an adapter layer).
NON_NEUTRAL = re.compile(r"carl|donut|mongo|crawl|mordecai|goblin|dozer|princess", re.I)
GRAMMAR = ("A", "B", "C")


class KitError(ValueError):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise KitError(message)


def repo_path(raw: str) -> Path:
    check(isinstance(raw, str) and raw != "" and not Path(raw).is_absolute(),
          f"path must be relative to the repository: {raw!r}")
    path = (ROOT / raw).resolve()
    check(path.is_relative_to(ROOT), f"path escapes repository: {raw}")
    return path


def load_json(raw: str | Path) -> dict:
    path = raw if isinstance(raw, Path) else repo_path(raw)
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise KitError(f"cannot read {path}: {exc}") from exc


def parse_rgba(text: str) -> tuple[int, int, int, int]:
    check(isinstance(text, str) and re.fullmatch(r"#[0-9a-fA-F]{8}", text) is not None,
          f"bad rgba {text!r}")
    return tuple(int(text[i:i + 2], 16) for i in (1, 3, 5, 7))  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------
class Kit:
    """A contract plus its loaded atlases (image, metadata, manifest, grid)."""

    def __init__(self, contract: dict, atlases: dict[str, dict],
                 path: Path | None = None) -> None:
        self.contract = contract
        self.path = path  # resolved contract path; None for in-memory kits
        self.atlases = atlases
        self.tiles = {t["id"]: t for t in contract.get("tiles", [])}
        self.pieces = {p["id"]: p for p in contract.get("pieces", [])}
        self._crops: dict[tuple, Image.Image] = {}

    def crop(self, atlas: str, rect: list[int]) -> Image.Image:
        key = (atlas, tuple(rect))
        if key not in self._crops:
            x, y, w, h = rect
            self._crops[key] = self.atlases[atlas]["image"].crop((x, y, x + w, y + h))
        return self._crops[key]


def load_kit(contract_path: str | Path, load_grids: bool = True) -> Kit:
    resolved = (contract_path if isinstance(contract_path, Path) else repo_path(contract_path)).resolve()
    contract = load_json(resolved)
    atlases = {}
    for name, ref in contract.get("atlases", {}).items():
        manifest = load_json(ref["manifest"])
        with Image.open(repo_path(ref["image"])) as opened:
            image = opened.convert("RGBA")
        atlases[name] = {
            "ref": ref, "manifest": manifest, "image": image,
            "metadata": load_json(ref["metadata"]),
            "grid": load_json(ref["grid"]) if load_grids and ref.get("grid") else None,
        }
    return Kit(contract, atlases, resolved)


# ---------------------------------------------------------------------------
# Contract validation
# ---------------------------------------------------------------------------

def _check_id(identifier: str, what: str) -> None:
    check(isinstance(identifier, str) and ID_RE.fullmatch(identifier) is not None,
          f"{what} id {identifier!r} must match {ID_RE.pattern}")
    check(NON_NEUTRAL.search(identifier) is None,
          f"{what} id {identifier!r} is not IP-neutral")


def _rect(rect, what: str) -> tuple[int, int, int, int]:
    check(isinstance(rect, list) and len(rect) == 4
          and all(type(v) is int for v in rect) and rect[0] >= 0 and rect[1] >= 0
          and rect[2] > 0 and rect[3] > 0, f"{what}: invalid rect {rect!r}")
    return tuple(rect)  # type: ignore[return-value]


def _inside(rect, image: Image.Image) -> bool:
    x, y, w, h = rect
    return x + w <= image.width and y + h <= image.height


def _alpha_extrema(img: Image.Image) -> tuple[int, int]:
    return img.getchannel("A").getextrema()


def validate_contract(kit: Kit) -> None:
    c = kit.contract
    check(c.get("format") == "env-kit-v1", "contract format must be env-kit-v1")
    _check_id(c.get("kit_id", ""), "kit")
    check(c.get("tile_size") == [16, 16], "tile_size must be [16, 16]")
    coords = c.get("coordinate_system", {})
    check(coords.get("origin") == "top_left" and coords.get("y_axis") == "down"
          and coords.get("row_order") == "top_to_bottom",
          "coordinate_system must declare top_left origin, y down, rows top_to_bottom")
    palette = c.get("palette")
    check(isinstance(palette, list) and 0 < len(palette) <= 64,
          "palette must list 1..64 colours (validate_assets.py colour budget)")
    colours = set()
    keys = set()
    for entry in palette:
        check(entry.get("key") not in keys, f"duplicate palette key {entry.get('key')!r}")
        keys.add(entry.get("key"))
        rgba = parse_rgba(entry.get("rgba"))
        check(rgba[3] == 255, f"palette colour {entry.get('name')} must be opaque")
        check(rgba[:3] not in colours, f"duplicate palette colour {entry.get('rgba')}")
        colours.add(rgba[:3])

    layers = c.get("layers", [])
    check(layers, "contract needs layers")
    names, orders = set(), set()
    for layer in layers:
        check(layer.get("role") in ROLES, f"layer {layer.get('name')}: unknown role")
        check(layer.get("kind") in ("tiles", "pieces"), f"layer {layer.get('name')}: bad kind")
        if layer["kind"] == "tiles":
            check(layer["role"] in TILE_ROLES, f"layer {layer['name']}: tiles must be "
                  "collision or backwall")
        check(layer.get("name") not in names, f"duplicate layer {layer.get('name')}")
        check(layer.get("draw_order") not in orders, "duplicate draw_order")
        names.add(layer["name"])
        orders.add(layer["draw_order"])
    check(c.get("actor_draw_order") not in orders, "actor_draw_order collides with a layer")

    # Atlases: manifest <-> contract <-> grid consistency and pixel rules.
    for name, atlas in kit.atlases.items():
        ref, manifest, image, meta = atlas["ref"], atlas["manifest"], atlas["image"], atlas["metadata"]
        check(manifest.get("palette_id") == c.get("palette_id"),
              f"{name}: manifest palette_id differs from contract")
        check(manifest.get("output_atlas") == ref.get("image"), f"{name}: atlas path mismatch")
        check(manifest.get("output_metadata") == ref.get("metadata"),
              f"{name}: metadata path mismatch")
        check(manifest.get("grid_source") == ref.get("grid"), f"{name}: grid path mismatch")
        check(manifest.get("review_status") == "prototype_fixture_unapproved",
              f"{name}: must not claim approval")
        if atlas.get("grid") is not None:
            grid_palette = [{k: e[k] for k in ("key", "name", "rgba")} for e in atlas["grid"]["palette"]]
            want = [{k: e[k] for k in ("key", "name", "rgba")} for e in palette]
            check(grid_palette == want, f"{name}: grid palette differs from the kit palette")
        histogram = image.getchannel("A").histogram()
        check(sum(histogram[1:255]) == 0, f"{name}: partial alpha found (1-bit alpha only)")
        used = image.getcolors(maxcolors=1 << 16)
        check(used is not None, f"{name}: too many colours")
        for _count, rgba in used:
            if rgba[3] == 255:
                check(rgba[:3] in colours,
                      f"{name}: colour #{rgba[0]:02x}{rgba[1]:02x}{rgba[2]:02x} not in kit palette")
        frames = meta.get("frames")
        check(isinstance(frames, list) and frames, f"{name}: missing metadata frames")
        for frame in frames:
            check(not frame.get("rotated") and not frame.get("trimmed"),
                  f"{name}: rotated/trimmed frames are not supported")
        if manifest.get("type") in ("tileset", "decor_atlas"):
            check(len(frames) == 1, f"{name}: tileset/decor atlas must be a single frame")
            fr = frames[0]["frame"]
            check([fr["x"], fr["y"], fr["w"], fr["h"]] == [0, 0, image.width, image.height]
                  and [image.width, image.height] == manifest.get("frame_size"),
                  f"{name}: atlas frame must cover the whole canvas")

    # Tiles
    seen_ids = set()
    per_atlas: dict[str, dict[int, str]] = {}
    panels: dict[str, dict] = {}
    for tile in c.get("tiles", []):
        tid = tile.get("id")
        _check_id(tid, "tile")
        check(tid not in seen_ids, f"duplicate id {tid}")
        seen_ids.add(tid)
        atlas = kit.atlases.get(tile.get("atlas"))
        check(atlas is not None, f"tile {tid}: unknown atlas {tile.get('atlas')!r}")
        check(atlas["manifest"].get("type") == "tileset", f"tile {tid}: atlas is not a tileset")
        x, y, w, h = _rect(tile.get("rect"), f"tile {tid}")
        check(w == 16 and h == 16 and x % 16 == 0 and y % 16 == 0,
              f"tile {tid}: rect must be a 16x16 cell on the tile grid")
        check(_inside(tile["rect"], atlas["image"]), f"tile {tid}: rect outside atlas")
        cols = atlas["image"].width // 16
        index = (y // 16) * cols + x // 16
        check(tile.get("grid_index") == index, f"tile {tid}: grid_index {tile.get('grid_index')} != {index}")
        manifest_tiles = atlas["manifest"].get("tiles", [])
        check(index < len(manifest_tiles) and manifest_tiles[index] == tid,
              f"tile {tid}: manifest tiles[{index}] disagrees with contract")
        per_atlas.setdefault(tile["atlas"], {})
        check(index not in per_atlas[tile["atlas"]], f"tile {tid}: cell already used")
        per_atlas[tile["atlas"]][index] = tid
        role = tile.get("role")
        check(role in TILE_ROLES, f"tile {tid}: role {role!r} invalid for tiles")
        check(tile.get("collidable") is (role == "collision"),
              f"tile {tid}: only collision-role tiles may (and must) be collidable")
        low, _high = _alpha_extrema(kit.crop(tile["atlas"], tile["rect"]))
        check(low == 255, f"tile {tid}: {role} tiles must be fully opaque")
        check(isinstance(tile.get("grammar"), list) and tile["grammar"]
              and set(tile["grammar"]) <= set(GRAMMAR), f"tile {tid}: grammar must be A/B/C")
        if "panel" in tile:
            p = tile["panel"]
            pw, ph = p.get("size", [0, 0])
            cx, cy = p.get("cell", [-1, -1])
            check(0 <= cx < pw and 0 <= cy < ph, f"tile {tid}: panel cell outside panel")
            entry = panels.setdefault(p["id"], {"size": p["size"], "family": p.get("family"),
                                                "cells": set()})
            check(entry["size"] == p["size"] and entry["family"] == p.get("family"),
                  f"tile {tid}: inconsistent panel size/family")
            check((cx, cy) not in entry["cells"], f"tile {tid}: duplicate panel cell")
            entry["cells"].add((cx, cy))
    for pid, entry in panels.items():
        pw, ph = entry["size"]
        check(len(entry["cells"]) == pw * ph, f"panel {pid}: incomplete")
    for name, atlas in kit.atlases.items():
        if atlas["manifest"].get("type") == "tileset":
            count = (atlas["image"].width // 16) * (atlas["image"].height // 16)
            check(len(per_atlas.get(name, {})) == count,
                  f"{name}: every tileset cell needs exactly one contract tile")

    # Pieces
    static_rects: dict[str, list[tuple[str, tuple]]] = {}
    for piece in c.get("pieces", []):
        pid = piece.get("id")
        _check_id(pid, "piece")
        check(pid not in seen_ids, f"duplicate id {pid}")
        seen_ids.add(pid)
        atlas = kit.atlases.get(piece.get("atlas"))
        check(atlas is not None, f"piece {pid}: unknown atlas")
        role = piece.get("role")
        check(role in ROLES and role != "collision", f"piece {pid}: invalid role {role!r}")
        check(piece.get("collidable") is False, f"piece {pid}: decor pieces are never collidable")
        x, y, w, h = _rect(piece.get("rect"), f"piece {pid}")
        check(_inside(piece["rect"], atlas["image"]), f"piece {pid}: rect outside atlas")
        ax, ay = piece.get("anchor", [None, None])
        check(type(ax) is int and type(ay) is int and 0 <= ax <= w and 0 <= ay <= h,
              f"piece {pid}: anchor outside piece")
        check(piece.get("grid_snap") in (1, 8, 16), f"piece {pid}: grid_snap must be 1, 8 or 16")
        check(isinstance(piece.get("grammar"), list) and set(piece["grammar"]) <= set(GRAMMAR),
              f"piece {pid}: grammar must be A/B/C")
        anim = piece.get("animation")
        rects = [piece["rect"]]
        if anim:
            meta = atlas["metadata"]
            tag = anim.get("tag", {})
            check(tag.get("name") in atlas["manifest"].get("animation_tags", []),
                  f"piece {pid}: tag not declared in manifest animation_tags")
            src_tags = {t["name"]: t for t in meta.get("meta", {}).get("frameTags", [])}
            check(tag.get("name") in src_tags, f"piece {pid}: tag missing from exported metadata")
            src = src_tags[tag["name"]]
            check((tag.get("from"), tag.get("to"), tag.get("direction", "forward")) ==
                  (src["from"], src["to"], src.get("direction", "forward")),
                  f"piece {pid}: tag range differs from exported metadata")
            frames = anim.get("frames", [])
            check(len(frames) == tag["to"] - tag["from"] + 1, f"piece {pid}: frame count mismatch")
            rects = []
            for i, fr in enumerate(frames):
                exported = meta["frames"][tag["from"] + i]
                check(fr.get("frame") == {k: exported["frame"][k] for k in ("x", "y", "w", "h")},
                      f"piece {pid}: frame {i} rect differs from exported metadata")
                check(fr.get("duration") == exported.get("duration") and fr["duration"] > 0,
                      f"piece {pid}: frame {i} duration differs from exported metadata")
                rect = [fr["frame"][k] for k in ("x", "y", "w", "h")]
                check(rect[2:] == [w, h], f"piece {pid}: frame size differs from rect")
                rects.append(rect)
        for rect in rects:
            low, high = _alpha_extrema(kit.crop(piece["atlas"], rect))
            check(low == 0 and high == 255,
                  f"piece {pid}: decor pieces must use (1-bit) alpha and be non-empty")
        if not anim:
            for other_id, other in static_rects.get(piece["atlas"], []):
                ox, oy, ow, oh = other
                overlap = x < ox + ow and ox < x + w and y < oy + oh and oy < y + h
                check(not overlap, f"piece {pid}: rect overlaps {other_id}")
            static_rects.setdefault(piece["atlas"], []).append((pid, (x, y, w, h)))
        if "arch" in piece:
            arch = piece["arch"]
            check(0 < arch.get("radius_inner", 0) < arch.get("radius_outer", 0),
                  f"piece {pid}: invalid arch radii")


# ---------------------------------------------------------------------------
# Layout validation
# ---------------------------------------------------------------------------

def validate_layout(kit: Kit, layout: dict) -> None:
    c = kit.contract
    check(layout.get("format") == "env-kit-layout-v1", "layout format must be env-kit-layout-v1")
    check(layout.get("kit") == c.get("kit_id"), "layout kit differs from contract kit_id")
    if kit.path is not None:
        check(repo_path(layout.get("kit_contract")) == kit.path,
              f"layout kit_contract {layout.get('kit_contract')!r} does not resolve to the "
              "loaded contract")
    check(layout.get("tile_size") == c.get("tile_size"), "layout tile_size differs")
    cols, rows = layout.get("grid_size", [0, 0])
    check(type(cols) is int and type(rows) is int and cols > 0 and rows > 0, "bad grid_size")
    width, height = cols * 16, rows * 16
    check(layout.get("pixel_size") == [width, height], "pixel_size must equal grid_size * 16")
    layers = {layer["name"]: layer for layer in c["layers"]}

    tile_layers = layout.get("tile_layers", {})
    piece_layers = layout.get("piece_layers", {})
    check(set(tile_layers) == {n for n, l in layers.items() if l["kind"] == "tiles"},
          "layout tile_layers must match the contract tile layers")
    check(set(piece_layers) == {n for n, l in layers.items() if l["kind"] == "pieces"},
          "layout piece_layers must match the contract piece layers")
    for name, grid in tile_layers.items():
        role = layers[name]["role"]
        check(isinstance(grid, list) and len(grid) == rows, f"{name}: expected {rows} rows")
        for r, row in enumerate(grid):
            check(isinstance(row, list) and len(row) == cols, f"{name} row {r}: expected {cols} cells")
            for x, tid in enumerate(row):
                if tid is None:
                    continue
                tile = kit.tiles.get(tid)
                check(tile is not None, f"{name} ({x},{r}): unknown tile {tid!r}")
                check(tile["role"] == role, f"{name} ({x},{r}): {tid} has role {tile['role']}, "
                      f"layer needs {role}")
            # Panel continuity: a panel cell is followed by its right neighbour.
            for x in range(cols - 1):
                left, right = row[x], row[x + 1]
                lp = kit.tiles[left].get("panel") if left else None
                rp = kit.tiles[right].get("panel") if right else None
                if not lp or not rp or lp.get("family") != rp.get("family"):
                    continue
                if lp["cell"][0] < lp["size"][0] - 1:
                    check(rp["id"] == lp["id"] and rp["cell"] == [lp["cell"][0] + 1, lp["cell"][1]],
                          f"{name} ({x + 1},{r}): {right} breaks panel {lp['id']} continuity")
                else:
                    check(rp["cell"][0] == 0,
                          f"{name} ({x + 1},{r}): {right} must start a panel (column 0)")
    arches: list[tuple[dict, int, int]] = []
    for name, items in piece_layers.items():
        role = layers[name]["role"]
        check(isinstance(items, list), f"{name}: must be a list")
        for i, item in enumerate(items):
            piece = kit.pieces.get(item.get("piece"))
            check(piece is not None, f"{name}[{i}]: unknown piece {item.get('piece')!r}")
            check(piece["role"] == role, f"{name}[{i}]: {piece['id']} has role {piece['role']}, "
                  f"layer needs {role}")
            px, py = item.get("x"), item.get("y")
            check(type(px) is int and type(py) is int, f"{name}[{i}]: x/y must be integers")
            snap = piece["grid_snap"]
            check(px % snap == 0 and py % snap == 0,
                  f"{name}[{i}]: {piece['id']} at ({px},{py}) not on its {snap} px grid")
            pw, ph = piece["rect"][2:]
            check(0 <= px and 0 <= py and px + pw <= width and py + ph <= height,
                  f"{name}[{i}]: {piece['id']} outside layout bounds")
            if "frame_offset" in item:
                n = len(piece.get("animation", {}).get("frames", [])) or 1
                check(type(item["frame_offset"]) is int and 0 <= item["frame_offset"] < n,
                      f"{name}[{i}]: bad frame_offset")
            if "arch" in piece:
                arches.append((piece, px, py))
    if arches:
        _check_arch_backwall(kit, layout, arches)

    # Segments, markers, views
    segments = layout.get("segments", [])
    check(segments, "layout needs A/B/C segments")
    cursor = 0
    for seg in segments:
        check(seg.get("grammar") in GRAMMAR, f"segment {seg.get('id')}: grammar must be A/B/C")
        x0, x1 = seg.get("x", [None, None])
        check(x0 == cursor and type(x1) is int and x1 > x0, f"segment {seg.get('id')}: x ranges "
              "must be contiguous and increasing")
        check(x0 % 16 == 0 and x1 % 16 == 0,
              f"segment {seg.get('id')}: boundaries must be multiples of 16 px")
        cursor = x1
    check(cursor == width, "segments must cover the full layout width")
    surfaces = _surface_rows(kit, tile_layers["collision"], cols, rows)
    base_row = _base_surface_row(surfaces)
    check(layout.get("base_floor_top_y") == base_row * 16 if base_row is not None else False,
          f"base_floor_top_y {layout.get('base_floor_top_y')!r} must equal 16 x the base "
          f"(most common) floor surface row {base_row}")
    marker_ids = set()
    kinds = set()
    for marker in layout.get("markers", []):
        check(marker.get("id") not in marker_ids, f"duplicate marker {marker.get('id')}")
        marker_ids.add(marker["id"])
        kinds.add(marker.get("kind"))
        mx, my = marker.get("x"), marker.get("y")
        check(type(mx) is int and type(my) is int and 0 <= mx < width and 0 <= my < height,
              f"marker {marker['id']} outside layout")
        if marker.get("kind") in ("spawn", "exit"):
            surface = surfaces[mx // 16]
            check(surface is not None and my == surface * 16 - 1,
                  f"marker {marker['id']}: y {my} must be the foot contact row "
                  f"(surface row {surface} * 16 - 1, 0 px foot sink) of column {mx // 16}")
        if "rect" in marker:
            rx, ry, rw, rh = marker["rect"] if isinstance(marker["rect"], list) \
                and len(marker["rect"]) == 4 else (None,) * 4
            check(all(type(v) is int for v in (rx, ry, rw, rh)) and rx >= 0 and ry >= 0
                  and rw > 0 and rh > 0 and rx + rw <= width and ry + rh <= height,
                  f"marker {marker['id']}: rect {marker['rect']!r} outside layout")
    check({"spawn", "exit"} <= kinds, "layout needs spawn and exit markers")
    for vid, (vx, vy, vw, vh) in layout.get("views", {}).items():
        check(vx >= 0 and vy >= 0 and vx + vw <= width and vy + vh <= height,
              f"view {vid} outside layout")

    req = layout.get("requirements", {})
    if req.get("continuous_floor"):
        _check_floor(kit, tile_layers["collision"], cols, rows, req.get("max_step_tiles", 1))
    _check_collision_topology(kit, tile_layers["collision"], cols, rows)
    _check_backwall_coverage(kit, tile_layers["backwall"], tile_layers["collision"], cols, rows)


def _check_arch_backwall(kit: Kit, layout: dict, arches: list[tuple[dict, int, int]]) -> None:
    """Above the springing line, cells inside an arch ring's mid radius must show
    recess (blue depth), and recess must never show outside every arch ring."""
    grid = layout["tile_layers"]["backwall"]
    for r, row in enumerate(grid):
        for x, tid in enumerate(row):
            if tid is None:
                continue
            tx, ty = x * 16 + 8, r * 16 + 8
            is_recess = bool(kit.tiles[tid].get("recess"))
            above_any = near_any = False
            for piece, px, py in arches:
                arch = piece["arch"]
                cx, cy = px + piece["anchor"][0], py + piece["anchor"][1]
                if ty >= cy:
                    continue
                above_any = True
                d = math.hypot(tx - cx, ty - cy)
                if d < (arch["radius_inner"] + arch["radius_outer"]) / 2:
                    check(is_recess,
                          f"backwall ({x},{r}) inside {piece['id']} must be a recess tile")
                if d <= arch["radius_outer"] + 12:
                    near_any = True
            if above_any and is_recess:
                check(near_any, f"backwall ({x},{r}) recess tile shows outside every arch ring")


def _is_solid(kit: Kit, tid) -> bool:
    return tid is not None and bool(kit.tiles[tid]["collidable"])


def _surface_rows(kit: Kit, grid: list, cols: int, rows: int) -> list[int | None]:
    """Top row of each column's floor run (connected to the bottom row); None for a pit."""
    surfaces: list[int | None] = []
    for x in range(cols):
        r = rows - 1
        if not _is_solid(kit, grid[r][x]):
            surfaces.append(None)
            continue
        while r - 1 >= 0 and _is_solid(kit, grid[r - 1][x]):
            r -= 1
        surfaces.append(r)
    return surfaces


def _base_surface_row(surfaces: list[int | None]) -> int | None:
    rows = [s for s in surfaces if s is not None]
    if not rows:
        return None
    counts = {r: rows.count(r) for r in set(rows)}
    return max(sorted(counts), key=lambda r: counts[r])  # ties: lowest row index


def _check_collision_topology(kit: Kit, grid: list, cols: int, rows: int) -> None:
    """Each column may hold a ceiling run (from row 0) and a floor run (to the last row)."""
    for x in range(cols):
        runs: list[list[int]] = []
        for r in range(rows):
            if _is_solid(kit, grid[r][x]):
                if runs and runs[-1][1] == r - 1:
                    runs[-1][1] = r
                else:
                    runs.append([r, r])
        check(len(runs) <= 2, f"collision column {x}: {len(runs)} collidable runs (max 2)")
        for start, end in runs:
            check(start == 0 or end == rows - 1,
                  f"collision column {x}: floating collision run rows {start}-{end} "
                  "(must connect to the ceiling row or the floor row)")


def _check_backwall_coverage(kit: Kit, backwall: list, collision: list, cols: int,
                             rows: int) -> None:
    """The backwall must cover every cell not hidden behind an opaque collidable tile."""
    for r in range(rows):
        for x in range(cols):
            if backwall[r][x] is None:
                check(_is_solid(kit, collision[r][x]),
                      f"backwall ({x},{r}): hole (no tile and no opaque collision tile in front)")


def _check_floor(kit: Kit, grid: list, cols: int, rows: int, max_step: int) -> None:
    surfaces = _surface_rows(kit, grid, cols, rows)
    for x, r in enumerate(surfaces):
        check(r is not None, f"collision column {x}: pit (no collidable tile on the bottom row)")
        check(kit.tiles[grid[r][x]].get("walkable_top"),
              f"collision column {x}: surface tile {grid[r][x]} is not walkable_top")
    for x in range(1, cols):
        check(abs(surfaces[x] - surfaces[x - 1]) <= max_step,
              f"collision column {x}: step of {abs(surfaces[x] - surfaces[x - 1])} tiles")


# ---------------------------------------------------------------------------
# Rendering (deterministic previews from exported atlases)
# ---------------------------------------------------------------------------
BACKGROUND = (12, 10, 16, 255)


def render_layout(kit: Kit, layout: dict, time_ms: int = 0) -> Image.Image:
    width, height = layout["pixel_size"]
    image = Image.new("RGBA", (width, height), BACKGROUND)
    for layer in sorted(kit.contract["layers"], key=lambda l: l["draw_order"]):
        name = layer["name"]
        if layer["kind"] == "tiles":
            for r, row in enumerate(layout["tile_layers"][name]):
                for x, tid in enumerate(row):
                    if tid:
                        tile = kit.tiles[tid]
                        image.alpha_composite(kit.crop(tile["atlas"], tile["rect"]), (x * 16, r * 16))
        else:
            for item in layout["piece_layers"][name]:
                piece = kit.pieces[item["piece"]]
                rect = piece["rect"]
                anim = piece.get("animation")
                if anim:
                    frames = anim["frames"]
                    total = sum(f["duration"] for f in frames)
                    offset = sum(f["duration"] for f in frames[:item.get("frame_offset", 0)])
                    t = (time_ms + offset) % total
                    for fr in frames:
                        if t < fr["duration"]:
                            rect = [fr["frame"][k] for k in ("x", "y", "w", "h")]
                            break
                        t -= fr["duration"]
                image.alpha_composite(kit.crop(piece["atlas"], rect), (item["x"], item["y"]))
    return image


def _text(draw: ImageDraw.ImageDraw, xy, text: str, fill=(226, 214, 200)) -> None:
    draw.text(xy, text, fill=fill)


def contact_sheet(kit: Kit) -> Image.Image:
    """Exploded native 1x view of every tile, piece and animation frame."""
    gap, pad, label_h = 2, 8, 12
    page, under = (24, 22, 30, 255), (52, 48, 60, 255)
    sections = []
    for name, atlas in kit.atlases.items():
        items: list[tuple[str, Image.Image]] = []
        if atlas["manifest"]["type"] == "tileset":
            for tile in sorted((t for t in kit.contract["tiles"] if t["atlas"] == name),
                               key=lambda t: t["grid_index"]):
                items.append((tile["id"], kit.crop(name, tile["rect"])))
            cols = atlas["image"].width // 16
        else:
            for piece in (p for p in kit.contract["pieces"] if p["atlas"] == name):
                frames = piece.get("animation", {}).get("frames")
                if frames:
                    for fr in frames:
                        items.append((piece["id"], kit.crop(name, [fr["frame"][k] for k in "xywh"])))
                else:
                    items.append((piece["id"], kit.crop(name, piece["rect"])))
            cols = None
        sections.append((name, items, cols))
    # layout: tilesets as exploded grids, pieces in a wrapping row
    max_w = 420
    placed = []
    y = pad
    for name, items, cols in sections:
        placed.append(("label", name, pad, y))
        y += label_h
        x = pad
        row_h = 0
        if cols:
            for i, (_id, img) in enumerate(items):
                cx, cy = i % cols, i // cols
                placed.append(("img", img, pad + cx * (16 + gap), y + cy * (16 + gap)))
            y += ((len(items) + cols - 1) // cols) * (16 + gap) + pad
        else:
            for _id, img in items:
                if x + img.width > max_w and x > pad:
                    x, y = pad, y + row_h + gap * 2
                    row_h = 0
                placed.append(("img", img, x, y))
                x += img.width + gap * 2
                row_h = max(row_h, img.height)
            y += row_h + pad
    sheet = Image.new("RGBA", (max_w + pad, y), page)
    draw = ImageDraw.Draw(sheet)
    for kind, obj, x, y0 in placed:
        if kind == "label":
            _text(draw, (x, y0), obj)
        else:
            sheet.paste(Image.new("RGBA", obj.size, under), (x, y0))
            sheet.alpha_composite(obj, (x, y0))
    return sheet


def scale(image: Image.Image, factor: int) -> Image.Image:
    return image.resize((image.width * factor, image.height * factor), Image.Resampling.NEAREST)


def side_by_side(composition_2x: Image.Image, concept_path: Path) -> Image.Image:
    if not concept_path.is_file():
        raise FileNotFoundError(concept_path)
    with Image.open(concept_path) as opened:
        concept = opened.convert("RGBA")
    # Display-only downscale of the mood reference; never used as a pixel source.
    concept = concept.resize(composition_2x.size, Image.Resampling.LANCZOS)
    band, gap = 28, 16
    out = Image.new("RGBA", (composition_2x.width * 2 + gap, composition_2x.height + band),
                    (16, 14, 20, 255))
    out.alpha_composite(composition_2x, (0, band))
    out.alpha_composite(concept, (composition_2x.width + gap, band))
    draw = ImageDraw.Draw(out)
    _text(draw, (8, 8), "ENV-01 kit composition from exported atlases (native 640x360, "
          "nearest 2x) - art preview, NOT a Bevy capture")
    _text(draw, (composition_2x.width + gap + 8, 8),
          "Concept 01 clean catacomb - generated MOOD reference only (display-scaled, not a source)")
    return out


def actor_check(kit: Kit, layout: dict, base: Image.Image, view: list[int]) -> Image.Image:
    from validate_assets import safe_path  # same path rules as the asset validator
    # Raises FileNotFoundError when an input is missing (the caller downgrades it to a warning).
    sink = kit.contract.get("floor_contact", {}).get("recommended_visual_foot_sink_px", 0)
    floor_y = layout["base_floor_top_y"]
    image = base.copy()
    for entry in layout.get("preview", {}).get("actor_check", []):
        manifest = json.loads(safe_path(entry["manifest"]).read_text(encoding="utf-8"))
        meta = json.loads(safe_path(manifest["output_metadata"]).read_text(encoding="utf-8"))
        atlas_path = safe_path(manifest["output_atlas"])
        if not atlas_path.is_file():
            raise FileNotFoundError(atlas_path)
        with Image.open(atlas_path) as opened:
            atlas = opened.convert("RGBA")
        fr = meta["frames"][0]["frame"]
        sprite = atlas.crop((fr["x"], fr["y"], fr["x"] + fr["w"], fr["y"] + fr["h"]))
        px, py = manifest["pivot"]
        image.alpha_composite(sprite, (entry["x"] - px, floor_y - 1 + sink - py))
    vx, vy, vw, vh = view
    return image.crop((vx, vy, vx + vw, vy + vh))


def save_png(image: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    image.convert("RGB").save(path, format="PNG", optimize=False)


def render_all(kit: Kit, layout: dict, out: Path, actors: bool = True) -> list[Path]:
    written = []
    full = render_layout(kit, layout)
    w, h = layout["pixel_size"]
    p = out / f"corridor_overview_{w}x{h}.png"
    save_png(full, p)
    written.append(p)
    for vid, (vx, vy, vw, vh) in layout.get("views", {}).items():
        view = full.crop((vx, vy, vx + vw, vy + vh))
        p1 = out / f"corridor_{vid}.png"
        p2 = out / f"corridor_{vid.split('_')[0]}_{vw * 2}x{vh * 2}_2x.png"
        save_png(view, p1)
        save_png(scale(view, 2), p2)
        written += [p1, p2]
    sheet = contact_sheet(kit)
    for factor in (1, 2):
        p = out / f"kit_sheet_{factor}x.png"
        save_png(scale(sheet, factor), p)
        written.append(p)
    preview = layout.get("preview", {})
    primary = preview.get("primary_view")
    if primary:
        vx, vy, vw, vh = layout["views"][primary]
        view2x = scale(full.crop((vx, vy, vx + vw, vy + vh)), 2)
        # Best-effort previews: unrelated asset renames must not break the kit export.
        if preview.get("concept_reference"):
            try:
                img = side_by_side(view2x, repo_path(preview["concept_reference"]))
            except FileNotFoundError as exc:
                print(f"WARNING: skipping concept side-by-side preview, missing input: {exc}",
                      file=sys.stderr)
            else:
                p = out / "corridor_vs_concept_side_by_side.png"
                save_png(img, p)
                written.append(p)
        if actors and preview.get("actor_check"):
            try:
                check_img = actor_check(kit, layout, full, layout["views"][primary])
            except FileNotFoundError as exc:
                print(f"WARNING: skipping actor check preview, missing input: {exc}",
                      file=sys.stderr)
            else:
                p1 = out / f"actor_check_{vw}x{vh}.png"
                p2 = out / f"actor_check_{vw * 2}x{vh * 2}_2x.png"
                save_png(check_img, p1)
                save_png(scale(check_img, 2), p2)
                written += [p1, p2]
    return written


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def find_layouts(levels: Path | None = None) -> list[Path]:
    """Every env-kit layout under assets/levels/. Unreadable files and unknown
    ``env-kit*`` format versions are errors, never silently skipped."""
    root = levels or LEVELS
    found = []
    for path in sorted(root.rglob("*.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise KitError(f"cannot parse {path}: {exc}") from exc
        fmt = data.get("format") if isinstance(data, dict) else None
        if isinstance(fmt, str) and fmt.startswith("env-kit"):
            check(fmt == "env-kit-layout-v1",
                  f"{path}: unsupported layout format {fmt!r} (expected env-kit-layout-v1)")
            found.append(path)
    return found


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate and preview environment kits")
    sub = parser.add_subparsers(dest="cmd", required=True)
    v = sub.add_parser("validate", help="validate a contract and optional layouts")
    v.add_argument("--contract", required=True)
    v.add_argument("--layout", action="append", default=[])
    r = sub.add_parser("render", help="validate and render previews for one layout")
    r.add_argument("--layout", required=True)
    r.add_argument("--out", type=Path)
    r.add_argument("--no-actors", action="store_true")
    a = sub.add_parser("all", help="validate and render every layout under assets/levels/")
    a.add_argument("--out-root", type=Path, default=OUT_ROOT)
    a.add_argument("--no-actors", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.cmd == "validate":
            kit = load_kit(args.contract)
            validate_contract(kit)
            for raw in args.layout:
                validate_layout(kit, load_json(raw))
            print(f"VALID kit {kit.contract['kit_id']}: {len(kit.tiles)} tiles, "
                  f"{len(kit.pieces)} pieces, {len(args.layout)} layout(s)")
            return 0
        jobs = []
        if args.cmd == "render":
            layout = load_json(args.layout)
            jobs.append((layout, args.out or OUT_ROOT / layout["layout_id"]))
        else:
            for path in find_layouts():
                layout = load_json(path)
                jobs.append((layout, args.out_root / layout["layout_id"]))
            check(bool(jobs), "no env-kit-layout-v1 layouts found under assets/levels/")
        for layout, out in jobs:
            kit = load_kit(layout["kit_contract"])
            validate_contract(kit)
            validate_layout(kit, layout)
            written = render_all(kit, layout, out, actors=not args.no_actors)
            print(f"VALID kit {kit.contract['kit_id']} + layout {layout['layout_id']}; previews:")
            for p in written:
                print("  " + (p.relative_to(ROOT).as_posix() if p.is_relative_to(ROOT) else str(p)))
        return 0
    except (KitError, KeyError, TypeError, OSError) as exc:
        print(f"ENV_KIT_ERROR: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
