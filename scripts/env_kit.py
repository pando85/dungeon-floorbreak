#!/usr/bin/env python3
"""Validate and preview modular environment kits (env-kit-v2).

An environment kit is described by an ``env-kit-v2`` contract (tile/piece IDs,
atlas rects, layer roles and light groups, collidable flags, walking lines and
collider rects, anchors, parallax strips, arch windows, light emitters,
animation frames) and is exercised by one or more ``env-kit-layout-v2`` sample
layouts. This tool checks the contract against the exported atlases, manifests
and pixel grids, checks the layouts against the contract, and renders
deterministic, unlit art previews (contact sheets and corridor compositions)
**from the exported atlases only**. ``scripts/env_kit_light.py`` adds the
presentation-only lighting pass on top of the same layer images.

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

from PIL import Image, ImageChops, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
LEVELS = ROOT / "assets" / "levels"
OUT_ROOT = ROOT / "generated" / "art" / "env_kit"

CONTRACT_FORMAT = "env-kit-v2"
LAYOUT_FORMAT = "env-kit-layout-v2"
PARALLAX_ROLES = ("parallax_far", "parallax_mid")
ROLES = ("collision", "backwall", "structure", "prop", "foreground") + PARALLAX_ROLES
TILE_ROLES = ("collision", "backwall")
LIGHT_GROUPS = ("parallax_far", "parallax_mid", "scene", "actors", "foreground")
MAX_ATLAS_COLOURS = 63
ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")
HEX_RE = re.compile(r"^#[0-9a-fA-F]{6}$")
# Runtime IDs must stay IP-neutral (fan-concept names live in an adapter layer).
NON_NEUTRAL = re.compile(r"carl|donut|mongo|crawl|mordecai|goblin|dozer|princess", re.I)
GRAMMAR = ("A", "B", "C")
TILE = 16
MAX_STEP_TILES = 2  # largest allowed walking-line step between neighbouring floor columns
MAX_LIGHT_RADIUS = 1024
DECK_VIEW = (640, 400)  # largest supported virtual view (Steam Deck); parallax must cover it
MIGRATION_DOC = "docs/02-art/floor-01-catacomb-kit.md"
MIGRATION_HINT = (f"see the section 'Migration from v1' in {MIGRATION_DOC}")
LIGHT_KEYS = {"id", "offset", "colour", "radius", "intensity", "group", "glow", "flicker",
              "project", "project_dy", "squash_y"}
LIGHTING_PROFILE_KEYS = {"profile_id", "presentation_only", "groups", "actor_contact_shadow",
                         "bloom", "note"}
LIGHTING_GROUP_KEYS = {"ambient", "haze", "light_scale"}
# Fields that decide collision or gameplay; none may reference lighting data.
GAMEPLAY_SECTIONS = ("tiles", "markers", "segments", "requirements", "walk_line")


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


def check_format(data, expected: str, what: str) -> None:
    """Reject anything but the current format; v1 gets a pointer to the migration notes."""
    check(isinstance(data, dict), f"{what} must be a JSON object")
    fmt = data.get("format")
    if fmt == expected:
        return
    if isinstance(fmt, str) and re.fullmatch(r"env-kit(-layout)?-v1", fmt):
        raise KitError(f"{what} format {fmt!r} is no longer supported: no v1 id keeps its meaning, "
                       f"so v1 files are rejected, not reinterpreted. Expected {expected!r}; "
                       f"{MIGRATION_HINT}.")
    raise KitError(f"{what} format {fmt!r} is not supported (expected {expected!r}); "
                   f"{MIGRATION_HINT}.")


def load_json(raw: str | Path) -> dict:
    path = raw if isinstance(raw, Path) else repo_path(raw)
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise KitError(f"cannot read {path}: {exc}") from exc


def load_layout(raw: str | Path) -> dict:
    layout = load_json(raw)
    check_format(layout, LAYOUT_FORMAT, "layout")
    return layout


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
        self.layers = {layer["name"]: layer for layer in contract.get("layers", [])}
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
    check_format(contract, CONTRACT_FORMAT, "contract")
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


def _number(value, what: str, lo: float | None = None, hi: float | None = None) -> float:
    check(isinstance(value, (int, float)) and not isinstance(value, bool), f"{what}: must be a number")
    check((lo is None or value >= lo) and (hi is None or value <= hi),
          f"{what}: {value} outside [{lo}, {hi}]")
    return float(value)


def _check_lights(kit: Kit, piece: dict) -> None:
    """Emitter schema. Lights are presentation data owned by a piece, never free-floating."""
    pid = piece["id"]
    lights = piece.get("lights")
    if lights is None:
        return
    check(isinstance(lights, list) and lights, f"piece {pid}: lights must be a non-empty list")
    layer = kit.layers[piece["layer"]]
    parallax = piece["role"] in PARALLAX_ROLES
    n_frames = len(piece.get("animation", {}).get("frames", []))
    ids = set()
    for light in lights:
        check(isinstance(light, dict), f"piece {pid}: every light must be an object")
        lid = light.get("id")
        check(isinstance(lid, str) and lid and lid not in ids, f"piece {pid}: light ids must be unique strings")
        ids.add(lid)
        what = f"piece {pid} light {lid}"
        unknown = sorted(set(light) - LIGHT_KEYS)
        check(not unknown, f"{what}: unknown field(s) {unknown}")
        off = light.get("offset")
        check(isinstance(off, list) and len(off) == 2 and all(type(v) is int for v in off),
              f"{what}: offset must be [x, y] integers")
        check(isinstance(light.get("colour"), str) and HEX_RE.fullmatch(light["colour"]) is not None,
              f"{what}: colour must be #rrggbb")
        _number(light.get("radius"), f"{what} radius", 1, MAX_LIGHT_RADIUS)
        _number(light.get("intensity"), f"{what} intensity", 0, 4)
        group = light.get("group")
        check(group in LIGHT_GROUPS and group != "actors",
              f"{what}: group must be one of {tuple(g for g in LIGHT_GROUPS if g != 'actors')}")
        if parallax:
            check(group == layer["light_group"],
                  f"{what}: a parallax strip's lights light its own group {layer['light_group']!r}")
        else:
            check(group not in PARALLAX_ROLES, f"{what}: only parallax strips may light {group!r}")
        if "glow" in light:
            glow = light["glow"]
            check(isinstance(glow, dict) and set(glow) == {"radius", "additive"},
                  f"{what}: glow must be {{radius, additive}}")
            _number(glow["radius"], f"{what} glow radius", 1, MAX_LIGHT_RADIUS)
            _number(glow["additive"], f"{what} glow additive", 0, 1)
        if "flicker" in light:
            fl = light["flicker"]
            check(n_frames > 0, f"{what}: flicker needs an animated piece")
            check(isinstance(fl, list) and len(fl) == n_frames,
                  f"{what}: flicker needs one multiplier per animation frame ({n_frames})")
            for v in fl:
                _number(v, f"{what} flicker", 0, 2)
        if "project" in light:
            check(light["project"] == "walk_line", f"{what}: project must be 'walk_line'")
            check(not parallax, f"{what}: parallax lights cannot project onto the walking line")
        check("project_dy" not in light or "project" in light, f"{what}: project_dy needs project")
        check(type(light.get("project_dy", 0)) is int, f"{what}: project_dy must be an integer")
        if "squash_y" in light:
            _number(light["squash_y"], f"{what} squash_y", 0.05, 1)


def _check_window(piece: dict) -> None:
    win = piece["window"]
    pid = piece["id"]
    check(isinstance(win, dict), f"piece {pid}: window must be an object")
    c = win.get("arc_centre")
    check(isinstance(c, list) and len(c) == 2 and all(type(v) is int for v in c),
          f"piece {pid}: window arc_centre must be [x, y] integers")
    check(type(win.get("arc_radius")) is int and win["arc_radius"] > 0, f"piece {pid}: bad arc_radius")
    rect = win.get("rect")
    check(isinstance(rect, list) and len(rect) == 4 and all(type(v) is int for v in rect)
          and rect[2] > 0 and rect[3] > 0, f"piece {pid}: window rect must be [x, y, w, h] integers")
    check(c[1] - win["arc_radius"] >= 0 and rect[0] >= 0 and rect[1] >= 0,
          f"piece {pid}: window must not extend above or left of the piece origin")
    check(piece["role"] == "structure", f"piece {pid}: windows belong to structure pieces")


def _wrap_seam_scores(strip: Image.Image) -> tuple[int, int]:
    """(wrap, allowed): differing rows between the last and first column, and the 90th
    percentile of the same measure between interior neighbouring columns (1..n-2)."""
    w, h = strip.size
    raw = strip.convert("RGBA").tobytes()
    data = [raw[i:i + 4] if raw[i + 3] else b"\0\0\0\0" for i in range(0, len(raw), 4)]

    def diff(a: int, b: int) -> int:
        return sum(1 for y in range(h) if data[y * w + a] != data[y * w + b])
    interior = sorted(diff(x, x + 1) for x in range(1, w - 2))
    allowed = interior[int(len(interior) * 0.9)] if interior else 0
    return diff(w - 1, 0), allowed


def validate_contract(kit: Kit) -> None:
    c = kit.contract
    check_format(c, CONTRACT_FORMAT, "contract")
    _check_id(c.get("kit_id", ""), "kit")
    check(c.get("tile_size") == [TILE, TILE], "tile_size must be [16, 16]")
    coords = c.get("coordinate_system", {})
    check(coords.get("origin") == "top_left" and coords.get("y_axis") == "down"
          and coords.get("row_order") == "top_to_bottom",
          "coordinate_system must declare top_left origin, y down, rows top_to_bottom")
    palette = c.get("palette")
    check(isinstance(palette, list) and palette, "palette must be a non-empty list")
    by_name: dict[str, tuple] = {}
    colours = set()
    for entry in palette:
        name = entry.get("name")
        check(isinstance(name, str) and name not in by_name, f"duplicate or missing palette name {name!r}")
        rgba = parse_rgba(entry.get("rgba"))
        check(rgba[3] == 255, f"palette colour {name} must be opaque")
        check(rgba[:3] not in colours, f"duplicate palette colour {entry.get('rgba')}")
        colours.add(rgba[:3])
        by_name[name] = rgba

    layers = c.get("layers", [])
    check(layers, "contract needs layers")
    names, orders = set(), set()
    for layer in layers:
        lname = layer.get("name")
        check(layer.get("role") in ROLES, f"layer {lname}: unknown role")
        check(layer.get("kind") in ("tiles", "pieces", "parallax"), f"layer {lname}: bad kind")
        if layer["kind"] == "tiles":
            check(layer["role"] in TILE_ROLES, f"layer {lname}: tiles must be collision or backwall")
        check((layer["kind"] == "parallax") == (layer["role"] in PARALLAX_ROLES),
              f"layer {lname}: parallax roles need kind 'parallax' (and only they)")
        if layer["kind"] == "parallax":
            factor = _number(layer.get("parallax"), f"layer {lname} parallax", 0.0, 1.0)
            check(0.0 < factor < 1.0, f"layer {lname}: parallax factor {factor} must be in (0, 1)")
        check(layer.get("light_group") in LIGHT_GROUPS and layer["light_group"] != "actors",
              f"layer {lname}: light_group must be one of {LIGHT_GROUPS}")
        check(lname not in names, f"duplicate layer {lname}")
        check(layer.get("draw_order") not in orders, "duplicate draw_order")
        names.add(lname)
        orders.add(layer["draw_order"])
    check(c.get("actor_draw_order") not in orders, "actor_draw_order collides with a layer")
    parallax_orders = [l["draw_order"] for l in layers if l["kind"] == "parallax"]
    other_orders = [l["draw_order"] for l in layers if l["kind"] != "parallax"]
    check(not parallax_orders or max(parallax_orders) < min(other_orders),
          "parallax layers must draw behind every other layer")
    factors = [l["parallax"] for l in sorted((l for l in layers if l["kind"] == "parallax"),
                                             key=lambda l: l["draw_order"])]
    check(all(a < b for a, b in zip(factors, factors[1:])),
          "parallax factors must increase with draw order (farther layers move slower)")
    lighting = c.get("lighting")
    check(isinstance(lighting, dict) and lighting.get("presentation_only") is True,
          "contract lighting block must exist and be presentation_only")
    _check_presentation_only(c, "contract")
    walk = c.get("walk_line", {}).get("base_floor", {})
    check(all(type(walk.get(k)) is int for k in ("floor_back_y", "walk_line_y", "front_edge_y"))
          and walk["floor_back_y"] < walk["walk_line_y"] < walk["front_edge_y"],
          "contract walk_line.base_floor needs integer floor_back_y < walk_line_y < front_edge_y")

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
        allowed = colours
        if atlas.get("grid") is not None:
            grid_palette = atlas["grid"]["palette"]
            check(len(grid_palette) <= MAX_ATLAS_COLOURS,
                  f"{name}: grid palette has {len(grid_palette)} colours (max {MAX_ATLAS_COLOURS})")
            allowed = set()
            for e in grid_palette:
                check(e.get("name") in by_name and parse_rgba(e["rgba"]) == by_name[e["name"]],
                      f"{name}: grid colour {e.get('name')} {e.get('rgba')} is not in the kit palette")
                allowed.add(parse_rgba(e["rgba"])[:3])
        histogram = image.getchannel("A").histogram()
        check(sum(histogram[1:255]) == 0, f"{name}: partial alpha found (1-bit alpha only)")
        used = image.getcolors(maxcolors=1 << 16)
        check(used is not None, f"{name}: too many colours")
        for _count, rgba in used:
            if rgba[3] == 255:
                check(rgba[:3] in allowed,
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
        check(w == TILE and h == TILE and x % TILE == 0 and y % TILE == 0,
              f"tile {tid}: rect must be a 16x16 cell on the tile grid")
        check(_inside(tile["rect"], atlas["image"]), f"tile {tid}: rect outside atlas")
        cols = atlas["image"].width // TILE
        index = (y // TILE) * cols + x // TILE
        check(tile.get("grid_index") == index, f"tile {tid}: grid_index {tile.get('grid_index')} != {index}")
        manifest_tiles = atlas["manifest"].get("tiles", [])
        check(index < len(manifest_tiles) and manifest_tiles[index] == tid,
              f"tile {tid}: manifest tiles[{index}] disagrees with contract")
        per_atlas.setdefault(tile["atlas"], {})
        check(index not in per_atlas[tile["atlas"]], f"tile {tid}: cell already used")
        per_atlas[tile["atlas"]][index] = tid
        role = tile.get("role")
        check(role in TILE_ROLES, f"tile {tid}: role {role!r} invalid for tiles")
        deck_back = tile.get("deck_back", False)
        check(type(deck_back) is bool and (not deck_back or role == "collision"),
              f"tile {tid}: deck_back is only valid on collision-role tiles")
        check(tile.get("collidable") is (role == "collision" and not deck_back),
              f"tile {tid}: collision-role tiles must be collidable (deck_back tiles must not); "
              "backwall tiles are never collidable")
        check(type(tile.get("walkable_top", False)) is bool,
              f"tile {tid}: walkable_top must be a boolean")
        if tile.get("walkable_top"):
            wl = tile.get("walk_line_y")
            check(type(wl) is int and 0 <= wl < TILE, f"tile {tid}: walkable_top needs walk_line_y 0..15")
            check(tile["collidable"], f"tile {tid}: walkable tiles must be collidable")
        else:
            check("walk_line_y" not in tile, f"tile {tid}: walk_line_y without walkable_top")
        if tile["collidable"]:
            wl = tile.get("walk_line_y", 0)
            collider = tile.get("collider")
            check(isinstance(collider, list) and all(type(v) is int for v in collider)
                  and collider == [0, wl, TILE, TILE - wl],
                  f"tile {tid}: collider must be [0, {wl}, 16, {TILE - wl}] (walking line rule)")
        else:
            check("collider" not in tile, f"tile {tid}: non-collidable tiles have no collider")
        low, _high = _alpha_extrema(kit.crop(tile["atlas"], tile["rect"]))
        check(low == 255, f"tile {tid}: {role} tiles must be fully opaque")
        check(isinstance(tile.get("grammar"), list) and tile["grammar"]
              and set(tile["grammar"]) <= set(GRAMMAR), f"tile {tid}: grammar must be A/B/C")
        if "panel" in tile:
            p = tile["panel"]
            pw, ph = p.get("size", [0, 0])
            cx, cy = p.get("cell", [-1, -1])
            check(ph == 1 and 0 <= cx < pw and cy == 0, f"tile {tid}: panel cell outside a one-row panel")
            entry = panels.setdefault(p["id"], {"size": p["size"], "family": p.get("family"),
                                                "cells": set()})
            check(entry["size"] == p["size"] and entry["family"] == p.get("family"),
                  f"tile {tid}: inconsistent panel size/family")
            check((cx, cy) not in entry["cells"], f"tile {tid}: duplicate panel cell")
            entry["cells"].add((cx, cy))
        if "edge" in tile:
            edge = tile["edge"]
            check(edge.get("kind") in ("end", "inner", "wall") and edge.get("side") in ("l", "r"),
                  f"tile {tid}: edge needs kind end/inner/wall and side l/r")
    for pid, entry in panels.items():
        pw, ph = entry["size"]
        check(len(entry["cells"]) == pw * ph, f"panel {pid}: incomplete")
    for name, atlas in kit.atlases.items():
        if atlas["manifest"].get("type") == "tileset":
            count = (atlas["image"].width // TILE) * (atlas["image"].height // TILE)
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
        layer = kit.layers.get(piece.get("layer"))
        check(layer is not None and layer["role"] == role and layer["kind"] != "tiles",
              f"piece {pid}: layer {piece.get('layer')!r} must be a piece/parallax layer of role {role}")
        check(piece.get("collidable") is False, f"piece {pid}: decor pieces are never collidable")
        x, y, w, h = _rect(piece.get("rect"), f"piece {pid}")
        check(_inside(piece["rect"], atlas["image"]), f"piece {pid}: rect outside atlas")
        ax, ay = piece.get("anchor", [None, None])
        check(type(ax) is int and type(ay) is int and 0 <= ax <= w and 0 <= ay <= h,
              f"piece {pid}: anchor outside piece")
        check(piece.get("grid_snap") in (1, 8, 16), f"piece {pid}: grid_snap must be 1, 8 or 16")
        check(isinstance(piece.get("grammar"), list) and set(piece["grammar"]) <= set(GRAMMAR),
              f"piece {pid}: grammar must be A/B/C")
        check(piece.get("tile_x", False) is (role in PARALLAX_ROLES),
              f"piece {pid}: parallax pieces (and only they) must declare tile_x")
        if "fixed_y" in piece:
            check(type(piece["fixed_y"]) is int and piece["fixed_y"] % piece["grid_snap"] == 0,
                  f"piece {pid}: fixed_y must be an integer on its grid")
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
        if role in PARALLAX_ROLES:
            check(not anim, f"piece {pid}: parallax strips are static (no animation)")
            check(w >= 2 * TILE, f"piece {pid}: parallax strip must be at least 32 px wide")
            wrap, allowed = _wrap_seam_scores(kit.crop(piece["atlas"], piece["rect"]))
            check(wrap <= allowed,
                  f"piece {pid}: strip does not tile horizontally: the last and first columns "
                  f"differ in {wrap} rows (interior neighbours differ in at most {allowed}); "
                  "author the strip as a periodic image so the wrap seam is as smooth as any column")
        for rect in rects:
            low, high = _alpha_extrema(kit.crop(piece["atlas"], rect))
            if role == "parallax_far":
                check(low == 255, f"piece {pid}: the far parallax strip must be fully opaque")
            else:
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
        if "window" in piece:
            _check_window(piece)
        _check_lights(kit, piece)


# ---------------------------------------------------------------------------
# Layout validation
# ---------------------------------------------------------------------------

def window_regions(kit: Kit, layout: dict) -> list[tuple[tuple, tuple]]:
    """Arch windows in layout px: ((arc cx, arc cy, r), (rect x, y, w, h))."""
    out = []
    for name, items in layout.get("piece_layers", {}).items():
        for item in items:
            piece = kit.pieces.get(item.get("piece"))
            if piece and "window" in piece:
                win = piece["window"]
                ox, oy = item["x"], item["y"]
                out.append(((ox + win["arc_centre"][0], oy + win["arc_centre"][1], win["arc_radius"]),
                            (ox + win["rect"][0], oy + win["rect"][1], win["rect"][2], win["rect"][3])))
    return out


def window_mask(size: tuple[int, int], windows) -> Image.Image:
    mask = Image.new("L", size, 0)
    draw = ImageDraw.Draw(mask)
    for (cx, cy, r), (rx, ry, rw, rh) in windows:
        # pixel (x, y) is inside the arc when its centre is within r of the arc centre, above cy
        for y in range(max(0, cy - r - 1), min(size[1], cy)):
            dy = y + 0.5 - cy
            if abs(dy) >= r:
                continue
            half = math.sqrt(r * r - dy * dy)
            x0 = math.ceil(cx - half - 0.5)
            x1 = math.floor(cx + half - 0.5)
            if x1 >= x0:
                draw.line([(max(0, x0), y), (min(size[0] - 1, x1), y)], fill=255)
        draw.rectangle([rx, ry, rx + rw - 1, ry + rh - 1], fill=255)
    return mask


def _cell_hits_mask(mask: Image.Image, x: int, r: int) -> bool:
    return mask.crop((x * TILE, r * TILE, x * TILE + TILE, r * TILE + TILE)).getextrema()[1] > 0


def validate_layout(kit: Kit, layout: dict) -> None:
    c = kit.contract
    check_format(layout, LAYOUT_FORMAT, "layout")
    check(layout.get("kit") == c.get("kit_id"), "layout kit differs from contract kit_id")
    check(not {"lights", "emitters"} & set(layout),
          "layout must not declare free-floating lights: emitters belong to kit pieces")
    if kit.path is not None:
        check(repo_path(layout.get("kit_contract")) == kit.path,
              f"layout kit_contract {layout.get('kit_contract')!r} does not resolve to the "
              "loaded contract")
    check(layout.get("tile_size") == c.get("tile_size"), "layout tile_size differs")
    cols, rows = layout.get("grid_size", [0, 0])
    check(type(cols) is int and type(rows) is int and cols > 0 and rows > 0, "bad grid_size")
    width, height = cols * TILE, rows * TILE
    check(layout.get("pixel_size") == [width, height], "pixel_size must equal grid_size * 16")
    layers = kit.layers

    tile_layers = layout.get("tile_layers", {})
    piece_layers = layout.get("piece_layers", {})
    parallax_layers = layout.get("parallax_layers", {})
    check(set(tile_layers) == {n for n, l in layers.items() if l["kind"] == "tiles"},
          "layout tile_layers must match the contract tile layers")
    check(set(piece_layers) == {n for n, l in layers.items() if l["kind"] == "pieces"},
          "layout piece_layers must match the contract piece layers")
    check(set(parallax_layers) == {n for n, l in layers.items() if l["kind"] == "parallax"},
          "layout parallax_layers must match the contract parallax layers")
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
            _check_row_joins(kit, name, r, row)
    for name, items in piece_layers.items():
        role = layers[name]["role"]
        check(isinstance(items, list), f"{name}: must be a list")
        for i, item in enumerate(items):
            piece = kit.pieces.get(item.get("piece"))
            check(piece is not None, f"{name}[{i}]: unknown piece {item.get('piece')!r}")
            check(piece["role"] == role and piece["layer"] == name,
                  f"{name}[{i}]: {piece['id']} belongs to layer {piece['layer']}, not {name}")
            check(isinstance(item, dict) and set(item) <= {"piece", "x", "y", "frame_offset"},
                  f"{name}[{i}]: an instance may only carry piece, x, y and frame_offset "
                  "(lights come from the piece declaration, never from the instance)")
            px, py = item.get("x"), item.get("y")
            check(type(px) is int and type(py) is int, f"{name}[{i}]: x/y must be integers")
            snap = piece["grid_snap"]
            check(px % snap == 0 and py % snap == 0,
                  f"{name}[{i}]: {piece['id']} at ({px},{py}) not on its {snap} px grid")
            if "fixed_y" in piece:
                check(py == piece["fixed_y"], f"{name}[{i}]: {piece['id']} must be placed at "
                      f"y = {piece['fixed_y']}")
            pw, ph = piece["rect"][2:]
            check(0 <= px and 0 <= py and px + pw <= width and py + ph <= height,
                  f"{name}[{i}]: {piece['id']} outside layout bounds")
            if "frame_offset" in item:
                n = len(piece.get("animation", {}).get("frames", [])) or 1
                check(type(item["frame_offset"]) is int and 0 <= item["frame_offset"] < n,
                      f"{name}[{i}]: bad frame_offset")
    windows = window_regions(kit, layout)
    for (cx, cy, r), (rx, ry, rw, rh) in windows:
        check(cx - r >= 0 and cx + r <= width and cy - r >= 0 and rx >= 0 and rx + rw <= width
              and ry + rh <= height, f"arch window at ({cx},{cy}) r{r} extends outside the layout")
    if windows:
        check(any(l["role"] == "parallax_far" for l in layers.values()),
              "arch windows need a parallax_far layer to show through them")
    for name, spec in parallax_layers.items():
        piece = kit.pieces.get(spec.get("piece"))
        check(piece is not None and piece["layer"] == name,
              f"parallax layer {name}: piece {spec.get('piece')!r} does not belong to it")
        check(type(spec.get("y")) is int and type(spec.get("x_origin")) is int,
              f"parallax layer {name}: y and x_origin must be integers")
        sw, sh = piece["rect"][2:]
        check(0 <= spec["x_origin"] < sw,
              f"parallax layer {name}: x_origin {spec['x_origin']} must be a phase in [0, {sw}) "
              "(the strip repeats every strip width)")
        check(0 <= spec["y"] and spec["y"] + sh <= height,
              f"parallax layer {name}: strip y {spec['y']}..{spec['y'] + sh} outside layout height {height}")
        if layers[name]["role"] == "parallax_far" and windows:
            top, bottom = spec["y"], spec["y"] + piece["rect"][3]
            for (cx, cy, r), (rx, ry, rw, rh) in windows:
                check(top <= cy - r and bottom >= ry + rh,
                      f"parallax layer {name}: strip y {top}..{bottom} does not cover a window")

    # Segments, markers, views
    segments = layout.get("segments", [])
    check(segments, "layout needs A/B/C segments")
    cursor = 0
    for seg in segments:
        check(seg.get("grammar") in GRAMMAR, f"segment {seg.get('id')}: grammar must be A/B/C")
        x0, x1 = seg.get("x", [None, None])
        check(x0 == cursor and type(x1) is int and x1 > x0, f"segment {seg.get('id')}: x ranges "
              "must be contiguous and increasing")
        check(x0 % TILE == 0 and x1 % TILE == 0,
              f"segment {seg.get('id')}: boundaries must be multiples of 16 px")
        cursor = x1
    check(cursor == width, "segments must cover the full layout width")
    tops = collider_tops(kit, tile_layers["collision"], cols, rows)
    base = _base_walk_line(tops)
    check(base is not None and layout.get("base_walk_line_y") == base,
          f"base_walk_line_y {layout.get('base_walk_line_y')!r} must equal the most common "
          f"walking line {base}")
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
            top = tops[mx // TILE]
            check(top is not None and top != "wall" and my == top - 1,
                  f"marker {marker['id']}: y {my} must be the foot contact row "
                  f"(walking line {top} - 1, 0 px foot sink) of column {mx // TILE}")
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
    _check_lighting_profile(layout)
    _check_presentation_only(layout, "layout", (layout["lighting"].get("profile_id"),))
    _check_light_anchors(kit, piece_layers, tops, cols)
    _check_preview_actors(layout, tops, width)

    req = layout.get("requirements")
    check(isinstance(req, dict) and req.get("continuous_floor") is True,
          "layout requirements.continuous_floor must be true (every column needs a walking line)")
    max_step = req.get("max_step_tiles", MAX_STEP_TILES)
    check(type(max_step) is int and 1 <= max_step <= MAX_STEP_TILES,
          f"requirements.max_step_tiles must be an integer in 1..{MAX_STEP_TILES}")
    check(kit.contract["walk_line"]["base_floor"]["walk_line_y"] == layout["base_walk_line_y"],
          "layout base_walk_line_y differs from the contract walk_line.base_floor.walk_line_y")
    _check_floor(kit, tile_layers["collision"], cols, rows, max_step)
    _check_walking_surfaces(kit, tile_layers["collision"], cols, rows)
    _check_collision_topology(kit, tile_layers["collision"], cols, rows)
    _check_backwall_coverage(kit, tile_layers["backwall"], tile_layers["collision"], cols, rows,
                             window_mask((width, height), windows) if windows else None)
    if windows:
        _check_windows(kit, layout, windows, width, height)
        _check_parallax_coverage(kit, layout, windows, width, height)


def _check_row_joins(kit: Kit, name: str, r: int, row: list) -> None:
    """Panel continuity, plus: edge singles touch panel edge columns only."""
    cols = len(row)
    for x in range(cols - 1):
        left, right = row[x], row[x + 1]
        lt = kit.tiles[left] if left else None
        rt = kit.tiles[right] if right else None
        lp = lt.get("panel") if lt else None
        rp = rt.get("panel") if rt else None
        if lp and rp and lp.get("family") == rp.get("family"):
            if lp["cell"][0] < lp["size"][0] - 1:
                check(rp["id"] == lp["id"] and rp["cell"] == [lp["cell"][0] + 1, 0],
                      f"{name} ({x + 1},{r}): {right} breaks panel {lp['id']} continuity")
            else:
                check(rp["cell"][0] == 0, f"{name} ({x + 1},{r}): {right} must start a panel (column 0)")
        if lt and "edge" in lt and lt["edge"]["kind"] != "end" and rp:
            check(rp["cell"][0] == 0, f"{name} ({x + 1},{r}): {right} after edge tile {left} "
                  "must be a panel's column 0")
        if rt and "edge" in rt and rt["edge"]["kind"] != "end" and lp:
            check(lp["cell"][0] == lp["size"][0] - 1, f"{name} ({x},{r}): {left} before edge tile "
                  f"{right} must be a panel's last column")


def _check_windows(kit: Kit, layout: dict, windows, width: int, height: int) -> None:
    """Arch windows show the parallax gallery: no backwall/decal pixels inside them, and every
    transparent pixel of the non-parallax layers below the props lies inside a window."""
    mask = window_mask((width, height), windows)
    bw = layout["tile_layers"]["backwall"]
    for r, row in enumerate(bw):
        for x, tid in enumerate(row):
            if tid is not None and _cell_hits_mask(mask, x, r):
                raise KitError(f"backwall ({x},{r}): {tid} intrudes into an arch window")
    for r, row in enumerate(layout["tile_layers"]["collision"]):
        for x, tid in enumerate(row):
            if tid is not None and _cell_hits_mask(mask, x, r):
                raise KitError(f"collision ({x},{r}): {tid} intrudes into an arch window")
    imgs = render_layers(kit, layout, names=("backwall", "backwall_decal", "collision", "structure"))
    decal_alpha = imgs["backwall_decal"].getchannel("A")
    check(ImageChops.multiply(decal_alpha, mask).getextrema()[1] == 0,
          "backwall_decal pixels intrude into an arch window")
    solid = Image.new("L", (width, height), 0)
    for name in ("backwall", "backwall_decal", "collision", "structure"):
        solid = ImageChops.lighter(solid, imgs[name].getchannel("A"))
    holes = ImageChops.subtract(ImageChops.invert(solid), mask)
    bbox = holes.getbbox()
    check(bbox is None, f"transparent pixels outside every arch window around {bbox}")


def _check_lighting_profile(layout: dict) -> None:
    lighting = layout.get("lighting")
    check(isinstance(lighting, dict) and lighting.get("presentation_only") is True,
          "layout lighting profile must exist and be presentation_only")
    unknown = sorted(set(lighting) - LIGHTING_PROFILE_KEYS)
    check(not unknown, f"lighting profile has unknown field(s) {unknown}: it carries presentation "
          "data only (ambient, haze, shadow, bloom), never collision or gameplay fields")
    check(isinstance(lighting.get("profile_id"), str) and lighting["profile_id"],
          "lighting profile needs a profile_id")
    groups = lighting.get("groups", {})
    check(set(groups) == set(LIGHT_GROUPS), f"lighting groups must be exactly {LIGHT_GROUPS}")
    for g, cfg in groups.items():
        unknown = sorted(set(cfg) - LIGHTING_GROUP_KEYS)
        check(not unknown, f"lighting {g}: unknown field(s) {unknown}")
        amb = cfg.get("ambient")
        check(isinstance(amb, list) and len(amb) == 3, f"lighting {g}: ambient must be [r, g, b]")
        for v in amb:
            _number(v, f"lighting {g} ambient", 0, 2)
        if "haze" in cfg:
            check(HEX_RE.fullmatch(cfg["haze"].get("colour", "")) is not None,
                  f"lighting {g}: haze colour must be #rrggbb")
            _number(cfg["haze"].get("amount"), f"lighting {g} haze amount", 0, 1)
        if "light_scale" in cfg:
            _number(cfg["light_scale"], f"lighting {g} light_scale", 0, 1)


def _light_references(node, profile_ids=(), path: str = "") -> list[str]:
    """Paths inside ``node`` that name lighting data (keys starting with light/emitter/glow/
    flicker/haze/bloom/ambient, or string values equal to a lighting profile id)."""
    found: list[str] = []
    if isinstance(node, dict):
        for key, value in node.items():
            here = f"{path}/{key}"
            if isinstance(key, str) and key.lower().startswith(
                    ("light", "emitter", "glow", "flicker", "haze", "bloom", "ambient")):
                found.append(here)
            found += _light_references(value, profile_ids, here)
    elif isinstance(node, list):
        for i, value in enumerate(node):
            found += _light_references(value, profile_ids, f"{path}[{i}]")
    elif isinstance(node, str) and node in profile_ids:
        found.append(f"{path} = {node!r}")
    return found


def _check_presentation_only(data: dict, what: str, profile_ids=()) -> None:
    """Collision/gameplay sections never reference lighting: lights are presentation only."""
    ids = tuple(p for p in profile_ids if isinstance(p, str) and p)
    for section in GAMEPLAY_SECTIONS:
        refs = _light_references(data.get(section), ids, section)
        check(not refs, f"{what}: gameplay/collision data references lighting "
              f"({refs[0] if refs else ''}); lights are presentation only")


def _check_light_anchors(kit: Kit, piece_layers: dict, tops: list, cols: int) -> None:
    """A walking-line-projected light must sit over a column that has a walking line."""
    for name, items in piece_layers.items():
        for item in items:
            piece = kit.pieces[item["piece"]]
            for light in piece.get("lights", []):
                if light.get("project") == "walk_line":
                    col = (item["x"] + light["offset"][0]) // TILE
                    check(0 <= col < cols and isinstance(tops[col], int),
                          f"{name}: light {piece['id']}.{light['id']} at x {item['x']} projects onto "
                          f"column {col}, which has no walking line")


def _check_preview_actors(layout: dict, tops: list, width: int) -> None:
    """Actor-check entries stand on a walking line (geometry is validated even though the
    image inputs of the preview are best-effort)."""
    for entry in layout.get("preview", {}).get("actor_check", []):
        x = entry.get("x") if isinstance(entry, dict) else None
        check(type(x) is int and 0 <= x < width and isinstance(tops[x // TILE], int),
              f"preview actor_check entry {entry!r}: x must be an integer on a column with a walking line")


def _check_walking_surfaces(kit: Kit, grid: list, cols: int, rows: int) -> None:
    """Exactly one walking surface per floor column, none anywhere else.

    * a floor run (collidable down to the last row) has exactly one ``walkable_top`` tile and
      it is the run's top tile, with a ``deck_back`` tile directly above it;
    * ceiling runs and wall columns have no walkable tile;
    * a ``deck_back`` tile always sits directly above a walkable tile (no orphans)."""
    for x in range(cols):
        r = 0
        while r < rows:
            if not _is_solid(kit, grid[r][x]):
                r += 1
                continue
            start = r
            while r + 1 < rows and _is_solid(kit, grid[r + 1][x]):
                r += 1
            end = r
            r += 1
            walkable = [row for row in range(start, end + 1) if kit.tiles[grid[row][x]].get("walkable_top")]
            is_floor = end == rows - 1 and start > 0
            if is_floor:
                check(walkable == [start],
                      f"collision column {x}: a floor run must have exactly one walking surface, "
                      f"on its top tile (walkable rows {walkable}, run rows {start}-{end})")
            else:
                check(not walkable, f"collision column {x}: walkable tile in row {walkable[:1]} "
                      "outside a floor run (ceiling and wall columns have no walking surface)")
        for row in range(rows):
            tid = grid[row][x]
            if tid is not None and kit.tiles[tid].get("deck_back"):
                below = grid[row + 1][x] if row + 1 < rows else None
                check(below is not None and kit.tiles[below].get("walkable_top"),
                      f"collision ({x},{row}): deck_back tile {tid} has no walkable tile directly below")


def _check_parallax_coverage(kit: Kit, layout: dict, windows, width: int, height: int) -> None:
    """The far strip repeats forever horizontally, so it must be drawn behind every arch window
    for any camera x. Check it on rendered pixels with the camera clamped at the left end and
    at the right end of the corridor, for every declared view size and the 640x400 deck view."""
    far = tuple(n for n, l in kit.layers.items() if l["role"] == "parallax_far")
    mask = window_mask((width, height), windows)
    views = [tuple(v) for v in layout.get("views", {}).values()]
    if DECK_VIEW[0] <= width and DECK_VIEW[1] <= height:
        views.append((0, 0, *DECK_VIEW))
    for vx, vy, vw, vh in views:
        for cam in sorted({0, width - vw}):
            alpha = render_layers(kit, layout, camera_x=cam, names=far)[far[0]].getchannel("A")
            box = (cam, vy, cam + vw, vy + vh)
            gaps = ImageChops.subtract(mask.crop(box), alpha.crop(box)).getbbox()
            check(gaps is None, f"parallax_far leaves empty sky behind an arch window with the "
                  f"camera at x {cam} (view {vw}x{vh} at y {vy}): uncovered pixels near {gaps}")


def _is_solid(kit: Kit, tid) -> bool:
    return tid is not None and bool(kit.tiles[tid]["collidable"])


def collider_tops(kit: Kit, grid: list, cols: int, rows: int) -> list:
    """Per column: walking line y (collider top) of the floor run connected to the bottom row,
    'wall' for a column collidable from top to bottom, None for a pit."""
    out: list = []
    for x in range(cols):
        r = rows - 1
        if not _is_solid(kit, grid[r][x]):
            out.append(None)
            continue
        while r - 1 >= 0 and _is_solid(kit, grid[r - 1][x]):
            r -= 1
        if r == 0:
            out.append("wall")
            continue
        out.append(r * TILE + kit.tiles[grid[r][x]].get("walk_line_y", 0))
    return out


def _surface_row(kit: Kit, grid: list, x: int, rows: int) -> int:
    r = rows - 1
    while r - 1 >= 0 and _is_solid(kit, grid[r - 1][x]):
        r -= 1
    return r


def _base_walk_line(tops: list) -> int | None:
    vals = [t for t in tops if isinstance(t, int)]
    if not vals:
        return None
    counts = {v: vals.count(v) for v in set(vals)}
    return max(sorted(counts, reverse=True), key=lambda v: counts[v])  # ties: lowest on screen


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
                             rows: int, mask: Image.Image | None) -> None:
    """The backwall covers every cell not hidden behind a collision-layer tile (all opaque)
    and not part of an arch window."""
    for r in range(rows):
        for x in range(cols):
            if backwall[r][x] is None:
                check(collision[r][x] is not None or (mask is not None and _cell_hits_mask(mask, x, r)),
                      f"backwall ({x},{r}): hole (no tile, no collision-layer tile, no arch window)")


def _check_floor(kit: Kit, grid: list, cols: int, rows: int, max_step: int) -> None:
    tops = collider_tops(kit, grid, cols, rows)
    prev = None
    for x, top in enumerate(tops):
        check(top is not None, f"collision column {x}: pit (no collidable tile on the bottom row)")
        if top == "wall":
            prev = None
            continue
        r = _surface_row(kit, grid, x, rows)
        check(kit.tiles[grid[r][x]].get("walkable_top"),
              f"collision column {x}: surface tile {grid[r][x]} is not walkable_top")
        check(grid[r - 1][x] is not None and kit.tiles[grid[r - 1][x]].get("deck_back"),
              f"collision column {x}: walkable tile needs a deck_back tile above it (deep floor)")
        if prev is not None:
            check(abs(top - prev) <= max_step * TILE,
                  f"collision column {x}: step of {abs(top - prev)} px (max {max_step * TILE})")
        prev = top


# ---------------------------------------------------------------------------
# Rendering (deterministic previews from exported atlases)
# ---------------------------------------------------------------------------
BACKGROUND = (12, 10, 16, 255)


def anim_rect(piece: dict, item: dict, time_ms: int) -> tuple[list[int], int]:
    """Atlas rect and frame index of a (possibly animated) piece at time_ms."""
    anim = piece.get("animation")
    if not anim:
        return piece["rect"], 0
    frames = anim["frames"]
    total = sum(f["duration"] for f in frames)
    offset = sum(f["duration"] for f in frames[:item.get("frame_offset", 0)])
    t = (time_ms + offset) % total
    for i, fr in enumerate(frames):
        if t < fr["duration"]:
            return [fr["frame"][k] for k in ("x", "y", "w", "h")], i
        t -= fr["duration"]
    raise AssertionError("unreachable")


def parallax_offset(layout: dict, kit: Kit, name: str, camera_x: float) -> float:
    return layout["parallax_layers"][name]["x_origin"] - camera_x * kit.layers[name]["parallax"]


def render_layers(kit: Kit, layout: dict, time_ms: int = 0, camera_x: float = 0.0,
                  names=None) -> dict[str, Image.Image]:
    """One full-layout RGBA image per contract layer (transparent background).

    Parallax strips are placed for a camera whose left edge is ``camera_x`` (layout px)."""
    width, height = layout["pixel_size"]
    out = {}
    for layer in sorted(kit.contract["layers"], key=lambda l: l["draw_order"]):
        name = layer["name"]
        if names is not None and name not in names:
            continue
        image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        if layer["kind"] == "tiles":
            for r, row in enumerate(layout["tile_layers"][name]):
                for x, tid in enumerate(row):
                    if tid:
                        tile = kit.tiles[tid]
                        image.alpha_composite(kit.crop(tile["atlas"], tile["rect"]), (x * TILE, r * TILE))
        elif layer["kind"] == "parallax":
            spec = layout["parallax_layers"][name]
            piece = kit.pieces[spec["piece"]]
            strip = kit.crop(piece["atlas"], piece["rect"])
            # screen x = offset + k * w + u - camera_x * parallax; draw in layout px (+camera_x)
            # (snapped to whole virtual pixels)
            start = round(camera_x + parallax_offset(layout, kit, name, camera_x))
            sw = strip.width
            x = start % sw - sw if start % sw else 0
            while x < width:
                image.alpha_composite(strip, (x, spec["y"])) if x >= 0 else \
                    image.alpha_composite(strip.crop((-x, 0, sw, strip.height)), (0, spec["y"]))
                x += sw
        else:
            for item in layout["piece_layers"][name]:
                piece = kit.pieces[item["piece"]]
                rect, _i = anim_rect(piece, item, time_ms)
                image.alpha_composite(kit.crop(piece["atlas"], rect), (item["x"], item["y"]))
        out[name] = image
    return out


def render_layout(kit: Kit, layout: dict, time_ms: int = 0, camera_x: float = 0.0,
                  actors: Image.Image | None = None) -> Image.Image:
    width, height = layout["pixel_size"]
    image = Image.new("RGBA", (width, height), BACKGROUND)
    actor_order = kit.contract.get("actor_draw_order")
    drawn_actors = actors is None
    for layer in sorted(kit.contract["layers"], key=lambda l: l["draw_order"]):
        if not drawn_actors and layer["draw_order"] > actor_order:
            image.alpha_composite(actors)
            drawn_actors = True
        image.alpha_composite(render_layers(kit, layout, time_ms, camera_x, names=(layer["name"],))[layer["name"]])
    if not drawn_actors:
        image.alpha_composite(actors)
    return image


def render_view(kit: Kit, layout: dict, view: list[int], time_ms: int = 0,
                actors: Image.Image | None = None) -> Image.Image:
    vx, vy, vw, vh = view
    full = render_layout(kit, layout, time_ms, camera_x=vx, actors=actors)
    return full.crop((vx, vy, vx + vw, vy + vh))


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
            cols = atlas["image"].width // TILE
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
    max_w = max([460] + [img.width + 2 * pad for _n, items, _c in sections for _i, img in items])
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
                placed.append(("img", img, pad + cx * (TILE + gap), y + cy * (TILE + gap)))
            y += ((len(items) + cols - 1) // cols) * (TILE + gap) + pad
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


def side_by_side(left: Image.Image, target_path: Path, left_label: str, right_label: str) -> Image.Image:
    if not target_path.is_file():
        raise FileNotFoundError(target_path)
    with Image.open(target_path) as opened:
        target = opened.convert("RGBA")
    if target.size != left.size:
        factor = left.width // target.width
        target = scale(target, factor) if factor * target.width == left.width else \
            target.resize(left.size, Image.Resampling.NEAREST)
    band, gap = 28, 16
    out = Image.new("RGBA", (left.width * 2 + gap, left.height + band), (16, 14, 20, 255))
    out.alpha_composite(left.convert("RGBA"), (0, band))
    out.alpha_composite(target, (left.width + gap, band))
    draw = ImageDraw.Draw(out)
    _text(draw, (8, 8), left_label)
    _text(draw, (left.width + gap + 8, 8), right_label)
    return out


def actor_layer(kit: Kit, layout: dict, entries: list[dict] | None = None) -> tuple[Image.Image, list]:
    """Approved actor frame 0 composited on the walking line of its column (0 px sink).

    Returns the full-layout actor image and the actor boxes. Raises FileNotFoundError when an
    input is missing (callers downgrade it to a warning)."""
    from validate_assets import safe_path  # same path rules as the asset validator
    width, height = layout["pixel_size"]
    cols, rows = layout["grid_size"]
    tops = collider_tops(kit, layout["tile_layers"]["collision"], cols, rows)
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    boxes = []
    for entry in entries if entries is not None else layout.get("preview", {}).get("actor_check", []):
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
        top = tops[entry["x"] // TILE]
        check(isinstance(top, int), f"actor at x {entry['x']} has no walking line")
        pos = (entry["x"] - px, top - 1 - py)
        image.alpha_composite(sprite, pos)
        boxes.append((pos[0], pos[1], pos[0] + sprite.width, pos[1] + sprite.height))
    return image, boxes


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
    for vid, view in layout.get("views", {}).items():
        img = render_view(kit, layout, view)
        vw, vh = view[2:]
        p1 = out / f"corridor_{vid}.png"
        p2 = out / f"corridor_{vid.split('_')[0]}_{vw * 2}x{vh * 2}_2x.png"
        save_png(img, p1)
        save_png(scale(img, 2), p2)
        written += [p1, p2]
    sheet = contact_sheet(kit)
    for factor in (1, 2):
        p = out / f"kit_sheet_{factor}x.png"
        save_png(scale(sheet, factor), p)
        written.append(p)
    preview = layout.get("preview", {})
    primary = preview.get("primary_view")
    if primary:
        view = layout["views"][primary]
        vw, vh = view[2:]
        unlit = render_view(kit, layout, view)
        # Best-effort previews: unrelated asset renames must not break the kit export.
        if preview.get("target_frame"):
            try:
                img = side_by_side(scale(unlit, 2), repo_path(preview["target_frame"]),
                                   "ENV-01 kit, UNLIT pixel layers from exported atlases (nearest 2x) "
                                   "- art preview, NOT a Bevy capture",
                                   "A1 target frame (owner-approved look-dev, lit)")
            except FileNotFoundError as exc:
                print(f"WARNING: skipping target-frame side-by-side, missing input: {exc}",
                      file=sys.stderr)
            else:
                p = out / "corridor_unlit_vs_a1_target.png"
                save_png(img, p)
                written.append(p)
        if actors and preview.get("actor_check"):
            try:
                actor_img, _boxes = actor_layer(kit, layout)
            except FileNotFoundError as exc:
                print(f"WARNING: skipping actor check preview, missing input: {exc}",
                      file=sys.stderr)
            else:
                check_img = render_view(kit, layout, view, actors=actor_img)
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
            if fmt != LAYOUT_FORMAT:
                try:
                    check_format(data, LAYOUT_FORMAT, "layout")
                except KitError as exc:
                    raise KitError(f"{path}: {exc}") from exc
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
                validate_layout(kit, load_layout(raw))
            print(f"VALID kit {kit.contract['kit_id']}: {len(kit.tiles)} tiles, "
                  f"{len(kit.pieces)} pieces, {len(args.layout)} layout(s)")
            return 0
        jobs = []
        if args.cmd == "render":
            layout = load_layout(args.layout)
            jobs.append((layout, args.out or OUT_ROOT / layout["layout_id"]))
        else:
            for path in find_layouts():
                layout = load_layout(path)
                jobs.append((layout, args.out_root / layout["layout_id"]))
            check(bool(jobs), f"no {LAYOUT_FORMAT} layouts found under assets/levels/")
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
