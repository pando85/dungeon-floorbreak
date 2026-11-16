#!/usr/bin/env python3
"""Deterministic text "pixel grid" -> Aseprite-format .ase converter.

A ``pixel-grid-v1`` JSON file is a human/agent-editable, diff-friendly pixel
source. This tool writes a byte-identical (no timestamps) native ``.ase`` file
following the official Aseprite file format specification
(https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md) that
Aseprite and LibreSprite can open and export. It also contains a small reader
for round-trip tests, a ``lint`` report and a ``render`` preview command.

Grid format (JSON)::

    {"format": "pixel-grid-v1", "canvas": [W, H], "pivot": [x, y],
     "palette": [{"key": "K", "name": "outline", "rgba": "#1b1222ff"}, ...],
     "layers": ["tail", "body", ...],            # bottom -> top, unique
     "frames": [{"duration_ms": 180,
                 "cels": {"body": ["..K..", ...]}}],   # H rows of W chars
     "tags": [{"name": "idle", "from": 0, "to": 3, "direction": "forward"}]}

``.`` is reserved for fully transparent. Every other character must be a
palette key (single character, not whitespace). Only fully opaque or fully
transparent pixels exist: palette alpha must be ``ff``. A layer missing from a
frame's ``cels`` is an empty cel. A palette entry may add ``"accent": true`` for
deliberate single-pixel details (catchlight, gem, nose, charm); accent keys are
exempt from the lint orphan rule (isolated pixels are still always errors).

Subcommands: ``build``, ``lint``, ``render``, ``to-grid``.
"""
from __future__ import annotations

import argparse
import json
import re
import struct
import sys
import zlib
from pathlib import Path

FORMAT = "pixel-grid-v1"
TRANSPARENT = "."
DIRECTIONS = {"forward": 0, "reverse": 1, "pingpong": 2, "pingpong_reverse": 3}
_DIRECTION_NAMES = {v: k for k, v in DIRECTIONS.items()}
_RGBA = re.compile(r"^#([0-9a-fA-F]{8})$")
_TAG_NAME = re.compile(r"^[A-Za-z0-9_-]+$")
PIVOT_PREFIX = "pivot="
ACCENT_SUFFIX = " [accent]"  # palette-entry name marker for accent keys
MAX_CANVAS = 4096


class GridError(ValueError):
    """Invalid pixel grid source or unreadable ASE data."""


# --------------------------------------------------------------------------
# Validation
# --------------------------------------------------------------------------

def _req(condition: bool, message: str) -> None:
    if not condition:
        raise GridError(message)


def _is_int(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def parse_rgba(text: str) -> tuple[int, int, int, int]:
    _req(isinstance(text, str) and _RGBA.match(text) is not None,
         f"palette rgba must look like '#rrggbbaa', got {text!r}")
    v = int(text[1:], 16)
    return (v >> 24) & 255, (v >> 16) & 255, (v >> 8) & 255, v & 255


def validate_grid(grid: dict) -> dict:
    """Validate a pixel-grid-v1 dict, return a normalized copy."""
    _req(isinstance(grid, dict), "Grid root must be an object")
    _req(grid.get("format") == FORMAT, f"format must be {FORMAT!r}")
    canvas = grid.get("canvas")
    _req(isinstance(canvas, list) and len(canvas) == 2 and all(_is_int(v) for v in canvas)
         and 1 <= canvas[0] <= MAX_CANVAS and 1 <= canvas[1] <= MAX_CANVAS,
         f"canvas must be [W, H] integers in 1..{MAX_CANVAS}")
    w, h = canvas
    pivot = grid.get("pivot")
    _req(isinstance(pivot, list) and len(pivot) == 2 and all(_is_int(v) for v in pivot),
         "pivot must be [x, y] integers")
    _req(0 <= pivot[0] < w and 0 <= pivot[1] < h,
         f"pivot {pivot} is outside canvas {canvas}")

    palette = grid.get("palette")
    _req(isinstance(palette, list) and palette, "palette must be a non-empty list")
    keys: dict[str, dict] = {}
    for entry in palette:
        _req(isinstance(entry, dict) and {"key", "name", "rgba"} <= entry.keys(),
             "palette entries need key, name, rgba")
        key, name = entry["key"], entry["name"]
        _req(isinstance(key, str) and len(key) == 1 and key.isprintable() and not key.isspace(),
             f"palette key must be one printable non-space character: {key!r}")
        _req(key != TRANSPARENT, f"palette key {TRANSPARENT!r} is reserved for transparent")
        _req(key not in keys, f"duplicate palette key {key!r}")
        _req(isinstance(name, str) and name.strip() != "" and name.isprintable(),
             f"palette key {key!r} needs a printable name")
        r, g, b, a = parse_rgba(entry["rgba"])
        _req(a == 255, f"palette key {key!r}: only fully opaque colours allowed (alpha ff)")
        accent = entry.get("accent", False)
        _req(isinstance(accent, bool), f"palette key {key!r}: accent must be true or false")
        _req(not name.endswith(ACCENT_SUFFIX), f"palette key {key!r}: name must not end with {ACCENT_SUFFIX!r}")
        keys[key] = {"key": key, "name": name, "rgba": f"#{r:02x}{g:02x}{b:02x}ff"}
        if accent:
            keys[key]["accent"] = True
    _req(len(palette) <= 255, "palette supports at most 255 keys")

    layers = grid.get("layers")
    _req(isinstance(layers, list) and layers and all(isinstance(n, str) and n for n in layers),
         "layers must be a non-empty list of names")
    _req(len(set(layers)) == len(layers), "duplicate layer names")

    frames = grid.get("frames")
    _req(isinstance(frames, list) and frames, "frames must be a non-empty list")
    _req(len(frames) <= 65535, "too many frames")
    norm_frames = []
    for fi, frame in enumerate(frames):
        _req(isinstance(frame, dict), f"frame {fi} must be an object")
        duration = frame.get("duration_ms")
        _req(_is_int(duration) and 1 <= duration <= 65535,
             f"frame {fi} duration_ms must be an integer in 1..65535")
        cels = frame.get("cels", {})
        _req(isinstance(cels, dict), f"frame {fi} cels must be an object")
        norm_cels = {}
        for layer, rows in cels.items():
            _req(layer in layers, f"frame {fi}: unknown layer {layer!r}")
            _req(isinstance(rows, list) and len(rows) == h,
                 f"frame {fi} layer {layer!r}: expected {h} rows, got "
                 f"{len(rows) if isinstance(rows, list) else type(rows).__name__}")
            for y, row in enumerate(rows):
                _req(isinstance(row, str) and len(row) == w,
                     f"frame {fi} layer {layer!r} row {y}: expected {w} chars, got "
                     f"{len(row) if isinstance(row, str) else type(row).__name__}")
                for x, ch in enumerate(row):
                    _req(ch == TRANSPARENT or ch in keys,
                         f"frame {fi} layer {layer!r} row {y} col {x}: unknown key {ch!r}")
            norm_cels[layer] = list(rows)
        norm_frames.append({"duration_ms": duration, "cels": norm_cels})

    tags = grid.get("tags", [])
    _req(isinstance(tags, list), "tags must be a list")
    norm_tags = []
    names = set()
    for tag in tags:
        _req(isinstance(tag, dict) and {"name", "from", "to"} <= tag.keys(),
             "tags need name, from, to")
        name = tag["name"]
        _req(isinstance(name, str) and _TAG_NAME.match(name) is not None,
             f"tag name must match [A-Za-z0-9_-]+: {name!r}")
        _req(name not in names, f"duplicate tag {name!r}")
        names.add(name)
        a, b = tag["from"], tag["to"]
        _req(_is_int(a) and _is_int(b) and 0 <= a <= b < len(norm_frames),
             f"tag {name!r} range {a}..{b} outside frames 0..{len(norm_frames) - 1}")
        direction = tag.get("direction", "forward")
        _req(direction in DIRECTIONS, f"tag {name!r} direction {direction!r} unknown")
        norm_tags.append({"name": name, "from": a, "to": b, "direction": direction})

    return {"format": FORMAT, "canvas": [w, h], "pivot": list(pivot),
            "palette": list(keys.values()), "layers": list(layers),
            "frames": norm_frames, "tags": norm_tags}


def load_grid(path: Path) -> dict:
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise GridError(f"cannot read grid {path}: {exc}") from exc
    return validate_grid(data)


# --------------------------------------------------------------------------
# ASE writer
# --------------------------------------------------------------------------

def _string(text: str) -> bytes:
    raw = text.encode("utf-8")
    return struct.pack("<H", len(raw)) + raw


def _chunk(kind: int, payload: bytes) -> bytes:
    return struct.pack("<IH", len(payload) + 6, kind) + payload


_LEN_BASE = (3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
             67, 83, 99, 115, 131, 163, 195, 227, 258)
_LEN_EXTRA = (0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4,
              5, 5, 5, 5, 0)
_DIST_BASE = (1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
              769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577)
_DIST_EXTRA = (0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
               11, 11, 12, 12, 13, 13)


def _zlib_deterministic(data: bytes, stride: int) -> bytes:
    """zlib stream (fixed-Huffman deflate, greedy LZ77) independent of libz.

    ``zlib.compress`` output may differ between zlib builds, which would make
    committed .ase files differ from CI rebuilds. Matches are only searched at
    distance 4 (previous pixel) and ``stride`` (pixel above); the result is
    a valid zlib stream that any inflater reads.
    """
    acc = 0
    nbits = 0
    out = bytearray()

    def put(value: int, count: int) -> None:  # LSB-first bit packing
        nonlocal acc, nbits
        acc |= value << nbits
        nbits += count
        while nbits >= 8:
            out.append(acc & 255)
            acc >>= 8
            nbits -= 8

    def huffman(code: int, count: int) -> None:  # codes are packed MSB-first
        put(int(format(code, f"0{count}b")[::-1], 2), count)

    def literal_or_length(sym: int) -> None:
        if sym < 144:
            huffman(0x30 + sym, 8)
        elif sym < 256:
            huffman(0x190 + sym - 144, 9)
        elif sym < 280:
            huffman(sym - 256, 7)
        else:
            huffman(0xC0 + sym - 280, 8)

    put(1, 1)  # final block
    put(1, 2)  # fixed Huffman
    n = len(data)
    i = 0
    distances = sorted({4, stride})
    while i < n:
        best_len, best_dist = 0, 0
        for dist in distances:
            if dist > i or dist > 32768:
                continue
            length = 0
            while length < 258 and i + length < n and data[i + length] == data[i + length - dist]:
                length += 1
            if length > best_len:
                best_len, best_dist = length, dist
        if best_len >= 3:
            li = max(k for k in range(29) if _LEN_BASE[k] <= best_len)
            literal_or_length(257 + li)
            put(best_len - _LEN_BASE[li], _LEN_EXTRA[li])
            di = max(k for k in range(30) if _DIST_BASE[k] <= best_dist)
            huffman(di, 5)
            put(best_dist - _DIST_BASE[di], _DIST_EXTRA[di])
            i += best_len
        else:
            literal_or_length(data[i])
            i += 1
    literal_or_length(256)
    put(0, 7)  # flush partial byte
    return b"\x78\x01" + bytes(out) + struct.pack(">I", zlib.adler32(data))


def _rgba_bytes(rows: list[str], keys: dict[str, tuple]) -> bytes:
    clear = bytes(4)
    out = bytearray()
    for row in rows:
        for ch in row:
            out += clear if ch == TRANSPARENT else bytes(keys[ch])
    return bytes(out)


def grid_to_ase(grid: dict) -> bytes:
    """Serialize a (validated) grid to deterministic .ase bytes."""
    grid = validate_grid(grid)
    w, h = grid["canvas"]
    keys = {e["key"]: parse_rgba(e["rgba"]) for e in grid["palette"]}
    layers = grid["layers"]
    frames_out = []
    for fi, frame in enumerate(grid["frames"]):
        chunks = []
        if fi == 0:
            for li, name in enumerate(layers):
                chunks.append(_chunk(0x2004, struct.pack(
                    "<HHHHHHB3x", 3, 0, 0, 0, 0, 0, 255) + _string(name)))
                if li == 0:
                    # Pivot as layer user data text: LibreSprite 1.3 rejects the
                    # slice chunk (0x2022) with a load error but accepts this.
                    text = f"{PIVOT_PREFIX}{grid['pivot'][0]},{grid['pivot'][1]}"
                    chunks.append(_chunk(0x2020, struct.pack("<I", 1) + _string(text)))
            # Tags chunk
            if grid["tags"]:
                body = struct.pack("<H8x", len(grid["tags"]))
                for tag in grid["tags"]:
                    body += struct.pack("<HHB8x3sx", tag["from"], tag["to"],
                                        DIRECTIONS[tag["direction"]], b"\x00\x00\x00")
                    body += _string(tag["name"])
                chunks.append(_chunk(0x2018, body))
            # Named palette: entry 0 transparent, then keys in declared order.
            entries = [(".", "transparent", (0, 0, 0, 0))] + [
                (e["key"], e["name"] + (ACCENT_SUFFIX if e.get("accent") else ""),
                 keys[e["key"]]) for e in grid["palette"]]
            body = struct.pack("<III8x", len(entries), 0, len(entries) - 1)
            for key, name, (r, g, b, a) in entries:
                body += struct.pack("<HBBBB", 1, r, g, b, a) + _string(f"{key}:{name}")
            chunks.append(_chunk(0x2019, body))
        for li, name in enumerate(layers):
            rows = frame["cels"].get(name)
            if rows is None or all(set(r) <= {TRANSPARENT} for r in rows):
                continue
            pixels = _rgba_bytes(rows, keys)
            body = struct.pack("<HhhBHh5x", li, 0, 0, 255, 2, 0)
            body += struct.pack("<HH", w, h) + _zlib_deterministic(pixels, w * 4)
            chunks.append(_chunk(0x2005, body))
        data = b"".join(chunks)
        header = struct.pack("<IHHH2xI", len(data) + 16, 0xF1FA,
                             min(len(chunks), 0xFFFF), frame["duration_ms"], len(chunks))
        frames_out.append(header + data)
    body = b"".join(frames_out)
    header = struct.pack(
        "<IHHHHHIH8xB3xHBBhhHH84x",
        128 + len(body), 0xA5E0, len(grid["frames"]), w, h, 32, 1,
        grid["frames"][0]["duration_ms"], 0, 0, 1, 1, 0, 0, 0, 0)
    assert len(header) == 128
    return header + body


def write_ase(grid: dict, path: Path) -> None:
    data = grid_to_ase(grid)
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


# --------------------------------------------------------------------------
# ASE reader (subset: RGBA, layers, compressed/raw cels, tags, palette, user data)
# --------------------------------------------------------------------------

def _read_string(buf: bytes, pos: int) -> tuple[str, int]:
    (n,) = struct.unpack_from("<H", buf, pos)
    return buf[pos + 2:pos + 2 + n].decode("utf-8"), pos + 2 + n


def read_ase(data: bytes) -> dict:
    """Parse ASE bytes written by this tool (RGBA) into a plain dict.

    Returns ``{"canvas", "layers", "frames": [{"duration_ms", "cels":
    {layer: [rows of (r,g,b,a)]}}], "tags", "palette": [(key, name, rgba)],
    "pivot"}``.
    """
    if len(data) < 128:
        raise GridError("ASE data shorter than header")
    size, magic, nframes, w, h, depth, _flags = struct.unpack_from("<IHHHHHI", data, 0)
    if magic != 0xA5E0 or size != len(data):
        raise GridError("Bad ASE magic or file size")
    if depth != 32:
        raise GridError("Only RGBA (32 bpp) ASE files are supported")
    pos = 128
    layers: list[str] = []
    tags = []
    palette = []
    pivot = None
    frames = []
    for _ in range(nframes):
        fsize, fmagic, _old, duration, _res, new_chunks = struct.unpack_from("<IHHH2sI", data, pos)
        if fmagic != 0xF1FA:
            raise GridError("Bad frame magic")
        end = pos + fsize
        cpos = pos + 16
        cels = {}
        for _c in range(new_chunks):
            csize, ctype = struct.unpack_from("<IH", data, cpos)
            body = data[cpos + 6:cpos + csize]
            if ctype == 0x2004:
                (_f, ltype, _lvl, _dw, _dh, _bm, _op) = struct.unpack_from("<HHHHHHB", body, 0)
                if ltype != 0:
                    raise GridError("Only normal layers are supported")
                layers.append(_read_string(body, 16)[0])
            elif ctype == 0x2005:
                li, cx, cy, _op, ctp, _z = struct.unpack_from("<HhhBHh", body, 0)
                cw, ch = struct.unpack_from("<HH", body, 16)
                raw = body[20:]
                if ctp == 2:
                    raw = zlib.decompress(raw)
                elif ctp != 0:
                    raise GridError(f"Unsupported cel type {ctp}")
                if (cx, cy, cw, ch) != (0, 0, w, h) or len(raw) != w * h * 4:
                    raise GridError("Only full-canvas cels are supported")
                rows = [[tuple(raw[(y * w + x) * 4:(y * w + x) * 4 + 4]) for x in range(w)]
                        for y in range(h)]
                cels[layers[li]] = rows
            elif ctype == 0x2018:
                (count,) = struct.unpack_from("<H", body, 0)
                p = 10
                for _t in range(count):
                    a, b, direction = struct.unpack_from("<HHB", body, p)
                    name, p = _read_string(body, p + 17)
                    tags.append({"name": name, "from": a, "to": b,
                                 "direction": _DIRECTION_NAMES[direction]})
            elif ctype == 0x2019:
                count, first, _last = struct.unpack_from("<III", body, 0)
                p = 20
                for _e in range(count):
                    eflags, r, g, b, a = struct.unpack_from("<HBBBB", body, p)
                    p += 6
                    name = ""
                    if eflags & 1:
                        name, p = _read_string(body, p)
                    palette.append((name, (r, g, b, a)))
            elif ctype == 0x2020:
                (uflags,) = struct.unpack_from("<I", body, 0)
                if uflags & 1:
                    text = _read_string(body, 4)[0]
                    if text.startswith(PIVOT_PREFIX):
                        pivot = [int(v) for v in text[len(PIVOT_PREFIX):].split(",")]
            cpos += csize
        if cpos != end:
            raise GridError("Frame chunk sizes do not add up")
        frames.append({"duration_ms": duration, "cels": cels})
        pos = end
    if pos != len(data):
        raise GridError("Trailing bytes after last frame")
    return {"canvas": [w, h], "layers": layers, "frames": frames, "tags": tags,
            "palette": palette, "pivot": pivot}


def normalize(grid: dict) -> dict:
    """Validated grid with empty (all-transparent) cels dropped."""
    grid = validate_grid(grid)
    for frame in grid["frames"]:
        frame["cels"] = {k: v for k, v in frame["cels"].items()
                         if any(set(r) - {TRANSPARENT} for r in v)}
    return grid


def ase_to_grid(data: bytes) -> dict:
    """Reverse ``grid_to_ase``: recover a normalized pixel-grid-v1 dict."""
    parsed = read_ase(data)
    palette, by_rgba = [], {}
    for entry_name, rgba in parsed["palette"][1:]:
        key, _, name = entry_name.partition(":")
        accent = name.endswith(ACCENT_SUFFIX)
        if accent:
            name = name[:-len(ACCENT_SUFFIX)]
        palette.append({"key": key, "name": name,
                        "rgba": "#%02x%02x%02x%02x" % rgba})
        if accent:
            palette[-1]["accent"] = True
        by_rgba.setdefault(rgba, key)
    frames = []
    for frame in parsed["frames"]:
        cels = {}
        for layer, rows in frame["cels"].items():
            cels[layer] = ["".join(TRANSPARENT if px[3] == 0 else by_rgba[px] for px in row)
                           for row in rows]
        frames.append({"duration_ms": frame["duration_ms"], "cels": cels})
    return {"format": FORMAT, "canvas": parsed["canvas"], "pivot": parsed["pivot"],
            "palette": palette, "layers": parsed["layers"], "frames": frames,
            "tags": parsed["tags"]}


# --------------------------------------------------------------------------
# Compositing, lint, render
# --------------------------------------------------------------------------

def composite_keys(grid: dict, frame_index: int) -> list[list[str]]:
    """Flatten one frame to rows of palette keys ('.' transparent), top layer wins."""
    w, h = grid["canvas"]
    out = [[TRANSPARENT] * w for _ in range(h)]
    cels = grid["frames"][frame_index]["cels"]
    for layer in grid["layers"]:
        rows = cels.get(layer)
        if rows is None:
            continue
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch != TRANSPARENT:
                    out[y][x] = ch
    return out


def _bbox(flat: list[list[str]]):
    ys = [y for y, row in enumerate(flat) if any(c != TRANSPARENT for c in row)]
    if not ys:
        return None
    xs = [x for row in flat for x, c in enumerate(row) if c != TRANSPARENT]
    return [min(xs), min(ys), max(xs), max(ys)]


_NEIGHBOURS = ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1))


def _stray_pixels(flat: list[list[str]], accents: set[str]) -> tuple[list[list[int]], list[list[int]]]:
    """Return ``(orphans, isolated)`` pixel coordinates.

    isolated: opaque pixel with no opaque pixel in its 8-neighbourhood (always an
    error, even for accent keys). orphan: a non-isolated, non-accent pixel with no
    same-key pixel in its 8-neighbourhood. Diagonal steps therefore never count.
    """
    h, w = len(flat), len(flat[0])
    orphans, isolated = [], []
    for y in range(h):
        for x in range(w):
            c = flat[y][x]
            if c == TRANSPARENT:
                continue
            around = [flat[y + dy][x + dx] for dx, dy in _NEIGHBOURS
                      if 0 <= x + dx < w and 0 <= y + dy < h]
            if all(n == TRANSPARENT for n in around):
                isolated.append([x, y])
            elif c not in accents and c not in around:
                orphans.append([x, y])
    return orphans, isolated


def lint_grid(grid: dict) -> dict:
    """Measure silhouette, contact row, palette use, orphan pixels and drift."""
    grid = validate_grid(grid)
    px, py = grid["pivot"]
    frames = []
    used: dict[str, int] = {}
    accents = {e["key"] for e in grid["palette"] if e.get("accent")}
    for fi in range(len(grid["frames"])):
        flat = composite_keys(grid, fi)
        box = _bbox(flat)
        orphans, isolated = _stray_pixels(flat, accents)
        for row in flat:
            for c in row:
                if c != TRANSPARENT:
                    used[c] = used.get(c, 0) + 1
        frames.append({
            "frame": fi,
            "empty": box is None,
            "bbox": box,
            "height": None if box is None else box[3] - box[1] + 1,
            "width": None if box is None else box[2] - box[0] + 1,
            "bottom_row": None if box is None else box[3],
            "orphan_pixels": orphans,
            "isolated_pixels": isolated,
        })
    first = frames[0]
    bottoms = [f["bottom_row"] for f in frames if not f["empty"]]
    names = {e["key"]: e["name"] for e in grid["palette"]}
    return {
        "canvas": grid["canvas"], "pivot": [px, py], "frame_count": len(frames),
        "frame0_silhouette_height": first["height"],
        "frame0_bbox": first["bbox"],
        "frame0_width": first["width"],
        "contact_row_matches_pivot": first["bottom_row"] == py,
        "frame0_contact_row": first["bottom_row"],
        "colors_used": {k: {"name": names[k], "pixels": n} for k, n in sorted(used.items())},
        "colors_declared_unused": sorted(set(names) - set(used)),
        "frame0_orphan_count": len(first["orphan_pixels"]),
        "orphan_count_per_frame": [len(f["orphan_pixels"]) for f in frames],
        "frame0_isolated_count": len(first["isolated_pixels"]),
        "isolated_count_per_frame": [len(f["isolated_pixels"]) for f in frames],
        "accent_keys": sorted(accents),
        "bottom_row_drift": (max(bottoms) - min(bottoms)) if bottoms else 0,
        "every_frame_bottom_equals_pivot": all(b == py for b in bottoms),
        "frames": frames,
    }


def lint_errors(report: dict) -> list[str]:
    """Findings that fail ``lint`` even without ``--strict``."""
    if any(report["isolated_count_per_frame"]):
        where = [(f["frame"], p) for f in report["frames"] for p in f["isolated_pixels"]][:5]
        return [f"isolated pixels (no opaque 8-neighbour) per frame: "
                f"{report['isolated_count_per_frame']}, first (frame, [x, y]): {where}"]
    return []


def lint_problems(report: dict) -> list[str]:
    problems = []
    if not report["contact_row_matches_pivot"]:
        problems.append(f"frame 0 contact row {report['frame0_contact_row']} != pivot y {report['pivot'][1]}")
    if report["bottom_row_drift"]:
        problems.append(f"bottom row drifts by {report['bottom_row_drift']} px across frames")
    elif not report["every_frame_bottom_equals_pivot"]:
        problems.append("a frame's bottom row differs from pivot y")
    problems += lint_errors(report)
    if any(report["orphan_count_per_frame"]):
        problems.append(f"orphan pixels per frame: {report['orphan_count_per_frame']}")
    if any(f["empty"] for f in report["frames"]):
        problems.append("an empty frame exists")
    return problems


def render_frames(grid: dict, out_dir: Path, scale: int = 8, bg: str = "#2a2a3a",
                  stem: str = "grid") -> list[Path]:
    """Write one nearest-neighbour PNG per frame plus a horizontal strip."""
    from PIL import Image  # local import keeps the converter dependency-free
    grid = validate_grid(grid)
    _req(1 <= scale <= 64, "scale must be 1..64")
    m = re.fullmatch(r"#?([0-9a-fA-F]{6})", bg)
    _req(m is not None, "bg must be '#rrggbb'")
    bgc = tuple(int(m.group(1)[i:i + 2], 16) for i in (0, 2, 4)) + (255,)
    keys = {e["key"]: parse_rgba(e["rgba"]) for e in grid["palette"]}
    w, h = grid["canvas"]
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    images, paths = [], []
    for fi in range(len(grid["frames"])):
        img = Image.new("RGBA", (w, h), bgc)
        flat = composite_keys(grid, fi)
        for y, row in enumerate(flat):
            for x, c in enumerate(row):
                if c != TRANSPARENT:
                    img.putpixel((x, y), keys[c])
        img = img.resize((w * scale, h * scale), Image.Resampling.NEAREST)
        images.append(img)
        path = out_dir / f"{stem}_f{fi:02d}.png"
        img.save(path)
        paths.append(path)
    strip = Image.new("RGBA", (w * scale * len(images), h * scale), bgc)
    for i, img in enumerate(images):
        strip.paste(img, (i * w * scale, 0))
    strip_path = out_dir / f"{stem}_strip.png"
    strip.save(strip_path)
    paths.append(strip_path)
    return paths


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("build", help="pixel-grid JSON -> deterministic .ase")
    p.add_argument("grid", type=Path)
    p.add_argument("out", type=Path)
    p = sub.add_parser("lint", help="measure silhouette/contact/orphans/drift")
    p.add_argument("grid", type=Path)
    p.add_argument("--json", action="store_true", help="print full JSON report")
    p.add_argument("--strict", action="store_true",
                   help="exit 1 on any problem (isolated pixels always fail)")
    p.add_argument("--expect-height", type=int, help="required frame-0 silhouette height")
    p = sub.add_parser("render", help="nearest-neighbour PNG per frame + strip")
    p.add_argument("grid", type=Path)
    p.add_argument("--out", type=Path, required=True, help="output directory")
    p.add_argument("--scale", type=int, default=8)
    p.add_argument("--bg", default="#2a2a3a")
    p = sub.add_parser("to-grid", help="read a .ase written by this tool back to grid JSON")
    p.add_argument("ase", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.cmd == "build":
            write_ase(load_grid(args.grid), args.out)
            print(f"wrote {args.out}")
        elif args.cmd == "lint":
            report = lint_grid(load_grid(args.grid))
            problems = lint_problems(report)
            if args.expect_height is not None and \
                    report["frame0_silhouette_height"] != args.expect_height:
                problems.append(f"silhouette height {report['frame0_silhouette_height']} "
                                f"!= expected {args.expect_height}")
            if args.json:
                print(json.dumps(report, indent=2))
            else:
                print(f"canvas {report['canvas']} pivot {report['pivot']} frames {report['frame_count']}")
                print(f"frame0 silhouette height {report['frame0_silhouette_height']} "
                      f"width {report['frame0_width']} bbox {report['frame0_bbox']}")
                print(f"contact row {report['frame0_contact_row']} "
                      f"{'==' if report['contact_row_matches_pivot'] else '!='} pivot y {report['pivot'][1]}")
                print("colors used: " + ", ".join(
                    f"{k}={v['name']}({v['pixels']})" for k, v in report["colors_used"].items()))
                print(f"orphan pixels per frame: {report['orphan_count_per_frame']} "
                      f"(8-neighbourhood, accent keys exempt: {report['accent_keys']})")
                print(f"isolated pixels per frame: {report['isolated_count_per_frame']}")
                print(f"bottom row drift across frames: {report['bottom_row_drift']} px")
            for problem in problems:
                print(f"LINT: {problem}", file=sys.stderr)
            if lint_errors(report) or (args.strict and problems):
                return 1
        elif args.cmd == "render":
            for path in render_frames(load_grid(args.grid), args.out, args.scale, args.bg,
                                      stem=args.grid.stem):
                print(path)
        elif args.cmd == "to-grid":
            print(json.dumps(ase_to_grid(args.ase.read_bytes()), indent=1))
        return 0
    except (GridError, OSError) as exc:
        print(f"PIXEL_GRID_ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
